use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use ta_app_core::double_ma::VdpsConfig;
use ta_app_core::strategy_presets::{
    breakout_strategy_for_kernel, trend_quality_strategy_for_kernel,
};
use ta_app_core::{
    parse_timeframe_label, read_candles_from_csv_compat, resample_candles,
    run_double_ma_optimization_blocking_with_candles,
    run_double_ma_validation_blocking_with_candles, Backend, DoubleMaParamsResolved,
    DoubleMaRequest, DoubleMaValidationParams, DoubleMaValidationRequest, ValidationEngine,
};
use ta_optimizer::ObjectiveKind;
use ta_strategies::double_ma::{Metrics, StrategyConfig};
use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

#[derive(Clone, Debug)]
struct CliConfig {
    csv_path: PathBuf,
    timeframes: Vec<String>,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    top_k: usize,
    holdout_frac: f64,
    device_id: u32,
    out_path: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct MaCase {
    label: &'static str,
    fast_ma: &'static str,
    slow_ma: &'static str,
}

#[derive(Clone, Debug)]
struct StrategyCase {
    label: &'static str,
    strategy: StrategyConfig,
}

#[derive(serde::Serialize)]
struct EvalArtifact {
    csv_path: String,
    timeframes: Vec<TimeframeEval>,
}

#[derive(serde::Serialize)]
struct TimeframeEval {
    timeframe: String,
    total_bars: usize,
    train_bars: usize,
    holdout_bars: usize,
    train_start_ts: Option<i64>,
    train_end_ts: Option<i64>,
    holdout_start_ts: Option<i64>,
    holdout_end_ts: Option<i64>,
    cases: Vec<CaseEval>,
}

#[derive(serde::Serialize)]
struct CaseEval {
    strategy_case: String,
    ma_case: String,
    objective: String,
    pair_count: usize,
    grid_runtime_ms: u64,
    progressive_runtime_ms: u64,
    progressive_selected_exact_best: bool,
    progressive_best_exact_rank: Option<usize>,
    progressive_best_score_regret: f64,
    topk_overlap_ratio: f64,
    holdout_positive_exact_topk_count: usize,
    holdout_positive_progressive_topk_count: usize,
    progressive_recovered_holdout_positive_count: usize,
    progressive_recovered_holdout_positive_ratio: f64,
    grid_best: CandidateSummary,
    progressive_best: CandidateSummary,
    holdout_validation: HoldoutEval,
}

#[derive(serde::Serialize)]
struct CandidateSummary {
    fast_ma: String,
    slow_ma: String,
    fast_len: u16,
    slow_len: u16,
    score: f64,
    metrics: MetricsSummary,
}

#[derive(serde::Serialize)]
struct MetricsSummary {
    pnl: f64,
    sharpe: f64,
    sortino: f64,
    calmar: f64,
    omega: f64,
    volatility: f64,
    max_dd: f64,
    trades: u32,
}

#[derive(serde::Serialize)]
struct HoldoutEval {
    bars: usize,
    grid_best: MetricsSummary,
    progressive_best: MetricsSummary,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct CandidateKey {
    fast_len: u16,
    slow_len: u16,
    fast_ma: String,
    slow_ma: String,
}

fn main() -> Result<(), String> {
    let cfg = parse_args()?;
    let base = read_candles_from_csv_compat(cfg.csv_path.to_string_lossy().as_ref())?;
    let cancel = AtomicBool::new(false);
    let strategy_cases = [
        StrategyCase {
            label: "plain",
            strategy: StrategyConfig::default(),
        },
        StrategyCase {
            label: "trend_quality",
            strategy: trend_quality_strategy_for_kernel(),
        },
        StrategyCase {
            label: "breakout_momentum",
            strategy: breakout_strategy_for_kernel(),
        },
    ];
    let ma_cases = [
        MaCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];

    let mut timeframe_results: Vec<TimeframeEval> = Vec::new();
    for tf in cfg.timeframes.iter() {
        let tf_spec = parse_timeframe_label(tf)?;
        let candles = if tf_spec.label.eq_ignore_ascii_case("1m") {
            base.clone()
        } else {
            resample_candles(&base, tf_spec.interval_ms)?
        };
        let (train, holdout) = split_candles_chronological(&candles, cfg.holdout_frac)?;

        let mut case_results: Vec<CaseEval> = Vec::new();
        for strategy_case in strategy_cases.iter() {
            for ma in ma_cases.iter().copied() {
                let pair_count = count_pairs_fast_lt_slow(
                    &expand_range_u16(cfg.fast_range),
                    &expand_range_u16(cfg.slow_range),
                );

                let grid_req = build_request(
                    ma,
                    strategy_case,
                    cfg.fast_range,
                    cfg.slow_range,
                    ObjectiveKind::Sharpe,
                    cfg.top_k,
                    true,
                    Backend::GpuKernel {
                        device_id: cfg.device_id,
                    },
                    None,
                );
                let grid_t0 = Instant::now();
                let grid_res = run_double_ma_optimization_blocking_with_candles(
                    grid_req, &train, &cancel, None,
                )?;
                let grid_runtime_ms = grid_t0.elapsed().as_millis() as u64;

                let vdps_req = build_request(
                    ma,
                    strategy_case,
                    cfg.fast_range,
                    cfg.slow_range,
                    ObjectiveKind::Sharpe,
                    cfg.top_k,
                    false,
                    Backend::GpuKernel {
                        device_id: cfg.device_id,
                    },
                    Some(VdpsConfig {
                        eta: Some(4),
                        rungs: Some(vec![0.0625, 0.125, 0.25, 0.5, 1.0]),
                        min_initial_pairs: Some(4096),
                        max_survivor_seeds: Some(256),
                        max_period_candidates: Some(384),
                        random_injection_ratio: Some(0.10),
                        elite_mutation_ratio: Some(0.50),
                        differential_ratio: Some(0.40),
                        deterministic: Some(true),
                        seed: Some(1),
                        quality_check_max_pairs: Some(100_000),
                        quality_check_top_k: Some(cfg.top_k as u32),
                    }),
                );
                let vdps_t0 = Instant::now();
                let vdps_res = run_double_ma_optimization_blocking_with_candles(
                    vdps_req, &train, &cancel, None,
                )?;
                let progressive_runtime_ms = vdps_t0.elapsed().as_millis() as u64;

                let (progressive_rank, topk_overlap_ratio, progressive_best_score_regret) =
                    compare_against_exact(&grid_res, &vdps_res, ObjectiveKind::Sharpe)?;
                let progressive_selected_exact_best = progressive_rank == Some(1);

                let holdout_validation =
                    evaluate_holdout(ma, strategy_case, &holdout, &grid_res, &vdps_res, &cancel)?;
                let high_quality_stats = evaluate_topk_holdout_recovery(
                    ma,
                    strategy_case,
                    &holdout,
                    &grid_res,
                    &vdps_res,
                    &cancel,
                )?;

                case_results.push(CaseEval {
                    strategy_case: strategy_case.label.to_string(),
                    ma_case: ma.label.to_string(),
                    objective: "Sharpe".to_string(),
                    pair_count,
                    grid_runtime_ms,
                    progressive_runtime_ms,
                    progressive_selected_exact_best,
                    progressive_best_exact_rank: progressive_rank,
                    progressive_best_score_regret,
                    topk_overlap_ratio,
                    holdout_positive_exact_topk_count: high_quality_stats
                        .holdout_positive_exact_topk_count,
                    holdout_positive_progressive_topk_count: high_quality_stats
                        .holdout_positive_progressive_topk_count,
                    progressive_recovered_holdout_positive_count: high_quality_stats
                        .progressive_recovered_holdout_positive_count,
                    progressive_recovered_holdout_positive_ratio: high_quality_stats
                        .progressive_recovered_holdout_positive_ratio,
                    grid_best: candidate_summary(
                        ma,
                        &grid_res.best_params,
                        &grid_res.best_metrics,
                        ObjectiveKind::Sharpe,
                    ),
                    progressive_best: candidate_summary(
                        ma,
                        &vdps_res.best_params,
                        &vdps_res.best_metrics,
                        ObjectiveKind::Sharpe,
                    ),
                    holdout_validation,
                });
            }
        }

        timeframe_results.push(TimeframeEval {
            timeframe: tf_spec.label,
            total_bars: candles.close.len(),
            train_bars: train.close.len(),
            holdout_bars: holdout.close.len(),
            train_start_ts: train.timestamp.first().copied(),
            train_end_ts: train.timestamp.last().copied(),
            holdout_start_ts: holdout.timestamp.first().copied(),
            holdout_end_ts: holdout.timestamp.last().copied(),
            cases: case_results,
        });
    }

    let artifact = EvalArtifact {
        csv_path: cfg.csv_path.to_string_lossy().to_string(),
        timeframes: timeframe_results,
    };

    if let Some(parent) = cfg.out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&cfg.out_path, json).map_err(|e| e.to_string())?;
    println!(
        "Wrote evaluation artifact to {}",
        cfg.out_path.to_string_lossy()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn parse_args() -> Result<CliConfig, String> {
    let mut cfg = CliConfig {
        csv_path: PathBuf::from("data/benchmarks/AAPL.csv"),
        timeframes: vec!["15m".to_string(), "1h".to_string()],
        fast_range: (5, 200, 1),
        slow_range: (10, 400, 1),
        top_k: 20,
        holdout_frac: 0.20,
        device_id: 0,
        out_path: PathBuf::from("target/real_data_optimizer_eval_aapl.json"),
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--csv" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --csv".to_string());
                };
                cfg.csv_path = PathBuf::from(v);
            }
            "--timeframes" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --timeframes".to_string());
                };
                cfg.timeframes = v
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "--fast-range" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --fast-range".to_string());
                };
                cfg.fast_range = parse_range_arg(&v, "fast-range")?;
            }
            "--slow-range" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --slow-range".to_string());
                };
                cfg.slow_range = parse_range_arg(&v, "slow-range")?;
            }
            "--top-k" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --top-k".to_string());
                };
                cfg.top_k = v
                    .parse::<usize>()
                    .map_err(|_| "invalid --top-k".to_string())?;
            }
            "--holdout-frac" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --holdout-frac".to_string());
                };
                cfg.holdout_frac = v
                    .parse::<f64>()
                    .map_err(|_| "invalid --holdout-frac".to_string())?;
            }
            "--device" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --device".to_string());
                };
                cfg.device_id = v
                    .parse::<u32>()
                    .map_err(|_| "invalid --device".to_string())?;
            }
            "--out" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --out".to_string());
                };
                cfg.out_path = PathBuf::from(v);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if cfg.timeframes.is_empty() {
        return Err("at least one timeframe is required".to_string());
    }
    if !(0.05..0.50).contains(&cfg.holdout_frac) {
        return Err("--holdout-frac should usually be in [0.05, 0.50)".to_string());
    }
    Ok(cfg)
}

fn print_help() {
    println!("real_data_optimizer_eval");
    println!("  --csv <path>           Default: data/benchmarks/AAPL.csv");
    println!("  --timeframes <list>    Default: 15m,1h");
    println!("  --fast-range a:b:c     Default: 5:200:1");
    println!("  --slow-range a:b:c     Default: 10:400:1");
    println!("  --top-k <n>            Default: 20");
    println!("  --holdout-frac <f>     Default: 0.20");
    println!("  --device <id>          Default: 0");
    println!("  --out <path>           Default: target/real_data_optimizer_eval_aapl.json");
}

fn parse_range_arg(raw: &str, name: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() != 3 {
        return Err(format!("{name} must be start:end:step"));
    }
    let start = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("invalid {name} start"))?;
    let end = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("invalid {name} end"))?;
    let step = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("invalid {name} step"))?;
    Ok((start, end, step))
}

fn split_candles_chronological(
    candles: &Candles,
    holdout_frac: f64,
) -> Result<(Candles, Candles), String> {
    let n = candles.close.len();
    if n < 1000 {
        return Err("dataset too small for real-data evaluation".to_string());
    }
    let holdout = ((n as f64) * holdout_frac).round() as usize;
    let holdout = holdout.clamp(256, n.saturating_div(2).max(256));
    let train_end = n.saturating_sub(holdout);
    if train_end < 512 {
        return Err("not enough train bars after holdout split".to_string());
    }
    Ok((
        slice_candles(candles, 0, train_end),
        slice_candles(candles, train_end, n),
    ))
}

fn slice_candles(candles: &Candles, start: usize, end: usize) -> Candles {
    let end = end.min(candles.close.len());
    let start = start.min(end);
    Candles::new_with_fields(
        candles.timestamp[start..end].to_vec(),
        candles.open[start..end].to_vec(),
        candles.high[start..end].to_vec(),
        candles.low[start..end].to_vec(),
        candles.close[start..end].to_vec(),
        candles.volume[start..end].to_vec(),
        CandleFieldFlags {
            open: candles.fields.open,
            high: candles.fields.high,
            low: candles.fields.low,
            close: candles.fields.close,
            volume: candles.fields.volume,
        },
    )
}

fn build_request(
    ma: MaCase,
    strategy_case: &StrategyCase,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    objective: ObjectiveKind,
    top_k: usize,
    include_all: bool,
    backend: Backend,
    vdps: Option<VdpsConfig>,
) -> DoubleMaRequest {
    let fast_ma_params = if ma.fast_ma == "alma" {
        Some(alma_params_default())
    } else {
        None
    };
    let slow_ma_params = if ma.slow_ma == "alma" {
        Some(alma_params_default())
    } else {
        None
    };
    DoubleMaRequest {
        backend,
        data_id: "real_eval".to_string(),
        fast_range,
        slow_range,
        fast_ma_types: vec![ma.fast_ma.to_string()],
        slow_ma_types: vec![ma.slow_ma.to_string()],
        ma_source: "close".to_string(),
        export_all_csv_path: None,
        fast_ma_params,
        slow_ma_params,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy: strategy_case.strategy.clone(),
        halftrend_filter: None,
        objective,
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: if vdps.is_some() {
            ta_optimizer::OptimizationMode::GpuVdps
        } else {
            ta_optimizer::OptimizationMode::Grid
        },
        top_k: Some(top_k),
        include_all: Some(include_all),
        heatmap_bins: Some(0),
        constraints: None,
        vdps,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        hurdle_tau: None,
        include_trade_metrics: false,
    }
}

fn compare_against_exact(
    exact: &ta_app_core::BackendOptimizationResult,
    approx: &ta_app_core::BackendOptimizationResult,
    objective: ObjectiveKind,
) -> Result<(Option<usize>, f64, f64), String> {
    let Some(all_rows) = exact.all.as_ref() else {
        return Err(
            "exact-grid result is missing all rows; rerun with include_all=true".to_string(),
        );
    };
    let mut ranked: Vec<(&DoubleMaParamsResolved, &Metrics)> =
        all_rows.iter().map(|(p, m)| (p, m)).collect();
    ranked.sort_by(|(pa, ma), (pb, mb)| {
        let sa = objective_score(ma, objective);
        let sb = objective_score(mb, objective);
        sb.total_cmp(&sa)
            .then_with(|| ma.max_dd.total_cmp(&mb.max_dd))
            .then_with(|| mb.trades.cmp(&ma.trades))
            .then_with(|| pa.fast_len.cmp(&pb.fast_len))
            .then_with(|| pa.slow_len.cmp(&pb.slow_len))
    });

    let progressive_rank = ranked
        .iter()
        .position(|(p, _)| {
            p.fast_len == approx.best_params.fast_len
                && p.slow_len == approx.best_params.slow_len
                && p.fast_ma_type == approx.best_params.fast_ma_type
                && p.slow_ma_type == approx.best_params.slow_ma_type
        })
        .map(|idx| idx + 1);

    let top_limit = exact.top.len().max(approx.top.len()).max(1);
    let exact_top: Vec<(u16, u16, String, String)> = ranked
        .iter()
        .take(top_limit)
        .map(|(p, _)| {
            (
                p.fast_len,
                p.slow_len,
                p.fast_ma_type.clone(),
                p.slow_ma_type.clone(),
            )
        })
        .collect();
    let approx_top: Vec<(u16, u16, String, String)> = approx
        .top
        .iter()
        .map(|(p, _)| {
            (
                p.fast_len,
                p.slow_len,
                p.fast_ma_type.clone(),
                p.slow_ma_type.clone(),
            )
        })
        .collect();
    let overlap = approx_top
        .iter()
        .filter(|key| exact_top.contains(key))
        .count() as f64
        / (top_limit as f64);
    let regret = (objective_score(&exact.best_metrics, objective)
        - objective_score(&approx.best_metrics, objective))
    .max(0.0);

    Ok((progressive_rank, overlap, regret))
}

fn evaluate_holdout(
    ma: MaCase,
    strategy_case: &StrategyCase,
    holdout: &Candles,
    grid_res: &ta_app_core::BackendOptimizationResult,
    vdps_res: &ta_app_core::BackendOptimizationResult,
    cancel: &AtomicBool,
) -> Result<HoldoutEval, String> {
    let req = DoubleMaValidationRequest {
        data_id: "holdout".to_string(),
        backend: ta_app_core::Backend::CpuOnly,
        ma_source: "close".to_string(),
        fast_ma_params: if ma.fast_ma == "alma" {
            Some(alma_params_default())
        } else {
            None
        },
        slow_ma_params: if ma.slow_ma == "alma" {
            Some(alma_params_default())
        } else {
            None
        },
        candidates: vec![
            validation_params_from_result(ma, strategy_case, &grid_res.best_params),
            validation_params_from_result(ma, strategy_case, &vdps_res.best_params),
        ],
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed: 1,
        max_candidates: 8,
        validation_engine: ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };
    let res = run_double_ma_validation_blocking_with_candles(req, holdout, cancel, None)?;
    if res.candidates.len() != 2 {
        return Err("unexpected holdout candidate count".to_string());
    }
    Ok(HoldoutEval {
        bars: holdout.close.len(),
        grid_best: metrics_summary(&res.candidates[0].in_sample),
        progressive_best: metrics_summary(&res.candidates[1].in_sample),
    })
}

struct HighQualityRecoveryStats {
    holdout_positive_exact_topk_count: usize,
    holdout_positive_progressive_topk_count: usize,
    progressive_recovered_holdout_positive_count: usize,
    progressive_recovered_holdout_positive_ratio: f64,
}

fn evaluate_topk_holdout_recovery(
    ma: MaCase,
    strategy_case: &StrategyCase,
    holdout: &Candles,
    grid_res: &ta_app_core::BackendOptimizationResult,
    vdps_res: &ta_app_core::BackendOptimizationResult,
    cancel: &AtomicBool,
) -> Result<HighQualityRecoveryStats, String> {
    let mut dedup: Vec<DoubleMaValidationParams> = Vec::new();
    let mut seen: HashSet<CandidateKey> = HashSet::new();

    for (params, _) in grid_res.top.iter().chain(vdps_res.top.iter()) {
        let key = candidate_key(params);
        if seen.insert(key) {
            dedup.push(validation_params_from_result(ma, strategy_case, params));
        }
    }

    if dedup.is_empty() {
        return Ok(HighQualityRecoveryStats {
            holdout_positive_exact_topk_count: 0,
            holdout_positive_progressive_topk_count: 0,
            progressive_recovered_holdout_positive_count: 0,
            progressive_recovered_holdout_positive_ratio: 0.0,
        });
    }

    let req = DoubleMaValidationRequest {
        data_id: "holdout_topk".to_string(),
        backend: ta_app_core::Backend::CpuOnly,
        ma_source: "close".to_string(),
        fast_ma_params: if ma.fast_ma == "alma" {
            Some(alma_params_default())
        } else {
            None
        },
        slow_ma_params: if ma.slow_ma == "alma" {
            Some(alma_params_default())
        } else {
            None
        },
        candidates: dedup,
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed: 1,
        max_candidates: 128,
        validation_engine: ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };
    let res = run_double_ma_validation_blocking_with_candles(req, holdout, cancel, None)?;

    let mut holdout_by_key: HashMap<CandidateKey, MetricsSummary> = HashMap::new();
    for candidate in res.candidates {
        holdout_by_key.insert(
            candidate_key_from_validation(&candidate.params),
            metrics_summary(&candidate.in_sample),
        );
    }

    let exact_positive: HashSet<CandidateKey> = grid_res
        .top
        .iter()
        .filter_map(|(p, _)| {
            let key = candidate_key(p);
            holdout_by_key
                .get(&key)
                .filter(|m| is_holdout_positive(m))
                .map(|_| key)
        })
        .collect();

    let progressive_positive: HashSet<CandidateKey> = vdps_res
        .top
        .iter()
        .filter_map(|(p, _)| {
            let key = candidate_key(p);
            holdout_by_key
                .get(&key)
                .filter(|m| is_holdout_positive(m))
                .map(|_| key)
        })
        .collect();

    let recovered = exact_positive
        .iter()
        .filter(|key| progressive_positive.contains(*key))
        .count();
    let ratio = if exact_positive.is_empty() {
        1.0
    } else {
        recovered as f64 / exact_positive.len() as f64
    };

    Ok(HighQualityRecoveryStats {
        holdout_positive_exact_topk_count: exact_positive.len(),
        holdout_positive_progressive_topk_count: progressive_positive.len(),
        progressive_recovered_holdout_positive_count: recovered,
        progressive_recovered_holdout_positive_ratio: ratio,
    })
}

fn validation_params_from_result(
    ma: MaCase,
    strategy_case: &StrategyCase,
    params: &DoubleMaParamsResolved,
) -> DoubleMaValidationParams {
    DoubleMaValidationParams {
        fast_ma_type: ma.fast_ma.to_string(),
        slow_ma_type: ma.slow_ma.to_string(),
        fast_len: params.fast_len,
        slow_len: params.slow_len,
        fast_ma_params: if ma.fast_ma == "alma" {
            Some(alma_params_default())
        } else {
            None
        },
        slow_ma_params: if ma.slow_ma == "alma" {
            Some(alma_params_default())
        } else {
            None
        },
        strategy: strategy_case.strategy.clone(),
        halftrend_filter: None,
    }
}

fn candidate_key(params: &DoubleMaParamsResolved) -> CandidateKey {
    CandidateKey {
        fast_len: params.fast_len,
        slow_len: params.slow_len,
        fast_ma: params.fast_ma_type.clone(),
        slow_ma: params.slow_ma_type.clone(),
    }
}

fn candidate_key_from_validation(params: &DoubleMaValidationParams) -> CandidateKey {
    CandidateKey {
        fast_len: params.fast_len,
        slow_len: params.slow_len,
        fast_ma: params.fast_ma_type.clone(),
        slow_ma: params.slow_ma_type.clone(),
    }
}

fn candidate_summary(
    ma: MaCase,
    params: &DoubleMaParamsResolved,
    metrics: &Metrics,
    objective: ObjectiveKind,
) -> CandidateSummary {
    CandidateSummary {
        fast_ma: ma.fast_ma.to_string(),
        slow_ma: ma.slow_ma.to_string(),
        fast_len: params.fast_len,
        slow_len: params.slow_len,
        score: objective_score(metrics, objective),
        metrics: metrics_summary(metrics),
    }
}

fn metrics_summary(metrics: &Metrics) -> MetricsSummary {
    MetricsSummary {
        pnl: metrics.pnl,
        sharpe: metrics.sharpe,
        sortino: metrics.sortino,
        calmar: metrics.calmar,
        omega: metrics.omega,
        volatility: metrics.volatility,
        max_dd: metrics.max_dd,
        trades: metrics.trades,
    }
}

fn is_holdout_positive(metrics: &MetricsSummary) -> bool {
    metrics.sharpe > 0.0 && metrics.pnl > 0.0
}

fn objective_score(metrics: &Metrics, objective: ObjectiveKind) -> f64 {
    match objective {
        ObjectiveKind::Sharpe => metrics.sharpe,
        ObjectiveKind::Sortino => metrics.sortino,
        ObjectiveKind::Calmar => metrics.calmar,
        ObjectiveKind::Omega => metrics.omega,
        ObjectiveKind::Volatility => -metrics.volatility,
        ObjectiveKind::Pnl => metrics.pnl,
        ObjectiveKind::MaxDrawdown => -metrics.max_dd,
    }
}

fn alma_params_default() -> HashMap<String, f64> {
    let mut p = HashMap::new();
    p.insert("offset".to_string(), 0.85);
    p.insert("sigma".to_string(), 6.0);
    p
}

fn expand_range_u16((start, end, step): (u32, u32, u32)) -> Vec<u16> {
    let push_checked = |out: &mut Vec<u16>, v: u32| {
        if v <= u16::MAX as u32 {
            out.push(v as u16);
        }
    };

    if step == 0 || start == end {
        let mut out = Vec::with_capacity(1);
        push_checked(&mut out, start);
        return out;
    }

    let step = step.max(1);
    let (lo, hi) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let mut out = Vec::new();
    let mut v = lo;
    loop {
        push_checked(&mut out, v);
        if v == hi {
            break;
        }
        match v.checked_add(step) {
            Some(next) if next > v && next <= hi => v = next,
            _ => break,
        }
    }
    out
}

fn count_pairs_fast_lt_slow(fast_periods: &[u16], slow_periods: &[u16]) -> usize {
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return 0;
    }
    let mut slow_sorted: Vec<u16> = slow_periods.to_vec();
    slow_sorted.sort_unstable();
    let s_len = slow_sorted.len();
    let mut total: usize = 0;
    for &f in fast_periods {
        let idx = slow_sorted.partition_point(|&s| s <= f);
        total = total.saturating_add(s_len.saturating_sub(idx));
    }
    total
}




