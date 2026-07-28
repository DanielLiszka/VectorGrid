use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use serde::Serialize;
use ta_app_core::double_ma::VdpsConfig;
use ta_app_core::strategy_presets::{
    breakout_strategy_for_kernel, trend_quality_strategy_for_kernel,
};
use ta_app_core::{
    parse_timeframe_label, read_candles_from_csv_compat, resample_candles,
    run_double_ma_optimization_blocking_with_candles,
    run_double_ma_validation_blocking_with_candles, Backend, BackendOptimizationResult,
    BackendUsed, CscvPboConfig, DoubleMaParamsResolved, DoubleMaRequest, DoubleMaValidationParams,
    DoubleMaValidationRequest, ReoptWalkForwardConfigOptional, SpaBenchmarkKind, SpaConfigOptional,
    ValidationArtifactContext, ValidationEngine, WalkForwardConfigOptional,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::{Metrics, StrategyConfig};
use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;
#[path = "../bin_support/benchmark_lock.rs"]
mod benchmark_lock;

#[derive(Clone, Debug)]
struct CliConfig {
    csv_path: PathBuf,
    timeframes: Vec<String>,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    top_k: usize,
    candidate_pool: usize,
    comparison_top_k: usize,
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

#[derive(Clone, Debug)]
struct ValidationPreset {
    label: &'static str,
    walk_forward: WalkForwardConfigOptional,
    reopt_walk_forward: ReoptWalkForwardConfigOptional,
    spa: SpaConfigOptional,
    cscv_pbo: CscvPboConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
struct CandidateKey {
    fast_len: u16,
    slow_len: u16,
    fast_ma: String,
    slow_ma: String,
}

#[derive(Clone, Debug)]
struct ValidationRow {
    key: CandidateKey,
    robust_score: f64,
    passed_hard_gates: bool,
    approved: bool,
}

#[derive(Serialize)]
struct FidelityAuditArtifact {
    csv_path: String,
    timeframes: Vec<TimeframeAudit>,
}

#[derive(Serialize)]
struct TimeframeAudit {
    timeframe: String,
    total_bars: usize,
    train_bars: usize,
    holdout_bars: usize,
    train_start_ts: Option<i64>,
    train_end_ts: Option<i64>,
    holdout_start_ts: Option<i64>,
    holdout_end_ts: Option<i64>,
    cases: Vec<CaseAudit>,
}

#[derive(Serialize)]
struct CaseAudit {
    strategy_case: String,
    ma_case: String,
    objective: String,
    pair_count: usize,
    optimization: OptimizationFidelityAudit,
    optimization_audits: Vec<OptimizationFidelityAudit>,
    validation: ValidationFidelityAudit,
}

#[derive(Clone, Serialize)]
struct OptimizationFidelityAudit {
    approx_mode: String,
    workload_family: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    contract_summary: String,
    exact_runtime_ms: u64,
    approx_runtime_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    approx_speedup_vs_exact: Option<f64>,
    approx_selected_exact_best: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    approx_best_exact_rank: Option<usize>,
    approx_best_score_regret: f64,
    topk_overlap_ratio: f64,
    exact_best: CandidateSummary,
    approx_best: CandidateSummary,
    holdout_winner_eval: OptimizationHoldoutEval,
    #[serde(skip_serializing_if = "Option::is_none")]
    vdps: Option<VdpsAuditSummary>,
}

#[derive(Clone, Serialize)]
struct VdpsAuditSummary {
    eta: u32,
    rungs: Vec<f64>,
    full_grid_pairs: usize,
    full_fidelity_pairs_evaluated: usize,
    full_fidelity_pairs_saved: usize,
    evaluated_pairs_total: usize,
    runtime_ms_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_top_k_overlap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_best_score_regret: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_exact_winner_rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_holdout_regret: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_speedup_vs_exact: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_baseline_runtime_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_baseline_backend: Option<BackendUsed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_check_baseline_pairs: Option<usize>,
}

#[derive(Clone, Serialize)]
struct OptimizationHoldoutEval {
    bars: usize,
    exact_best: MetricsSummary,
    approx_best: MetricsSummary,
}

#[derive(Serialize)]
struct ValidationFidelityAudit {
    candidate_pool_size: usize,
    comparison_top_k: usize,
    candidate_pool: Vec<CandidateKey>,
    baseline: ValidationRunSummary,
    cheaper: ValidationRunSummary,
    top_candidate_match: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    cheaper_top_candidate_baseline_rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cheaper_top_candidate_baseline_regret: Option<f64>,
    topk_overlap_ratio: f64,
    hard_gate_overlap_ratio: f64,
    approved_overlap_ratio: f64,
    candidate_hard_gate_disagreements: usize,
    candidate_approval_disagreements: usize,
    run_passed_match: bool,
    run_approved_match: bool,
    final_recommendation_match: bool,
    holdout_top_candidate_eval: ValidationTopCandidateHoldoutEval,
}

#[derive(Serialize)]
struct ValidationRunSummary {
    label: String,
    runtime_ms: u64,
    backend_used: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_fallback_reason: Option<String>,
    preset: ValidationPresetSummary,
    run_verdict: ValidationRunVerdictSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_candidate: Option<ValidationCandidateAuditSummary>,
    ranked_candidates: Vec<ValidationCandidateAuditSummary>,
}

#[derive(Serialize)]
struct ValidationPresetSummary {
    walk_forward_window_bars: usize,
    walk_forward_step_bars: usize,
    reopt_window_bars: usize,
    reopt_step_bars: usize,
    reopt_top_n_candidates: usize,
    cscv_slices: usize,
    cscv_max_combinations: usize,
    cscv_threshold: f64,
    spa_slices: usize,
    spa_bootstrap_samples: usize,
    spa_block_length: usize,
    spa_benchmark: String,
}

#[derive(Serialize)]
struct ValidationRunVerdictSummary {
    passed: bool,
    approved: bool,
    candidates_tested: usize,
    candidates_survived_hard_gates: usize,
    approved_candidate_count: usize,
    final_recommendation: String,
    warning_flags: Vec<String>,
    fail_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_approved_rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pbo: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_pbo: Option<f64>,
}

#[derive(Clone, Serialize)]
struct ValidationCandidateAuditSummary {
    rank: usize,
    candidate: CandidateKey,
    robust_score: f64,
    passed_hard_gates: bool,
    approved: bool,
    track_record_bars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_track_record_length_bars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    psr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dsr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    walk_forward_sharpe_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    walk_forward_sharpe_worst: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cscv_oos_metric_p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cscv_oos_rank_frac_p50: Option<f64>,
    warning_flags: Vec<String>,
    fail_reasons: Vec<String>,
    in_sample: MetricsSummary,
}

#[derive(Serialize)]
struct ValidationTopCandidateHoldoutEval {
    bars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_top_candidate: Option<HoldoutCandidateEval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cheaper_top_candidate: Option<HoldoutCandidateEval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cheaper_top_candidate_sharpe_regret_vs_baseline: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cheaper_top_candidate_pnl_regret_vs_baseline: Option<f64>,
}

#[derive(Serialize)]
struct HoldoutCandidateEval {
    candidate: CandidateKey,
    metrics: MetricsSummary,
}

#[derive(Clone, Serialize)]
struct CandidateSummary {
    fast_ma: String,
    slow_ma: String,
    fast_len: u16,
    slow_len: u16,
    score: f64,
    metrics: MetricsSummary,
}

#[derive(Clone, Serialize)]
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

fn main() -> Result<(), String> {
    let cfg = parse_args()?;
    let _benchmark_guard = benchmark_lock::acquire("real-data fidelity audit")?;
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

    let mut timeframe_results: Vec<TimeframeAudit> = Vec::new();
    for tf in cfg.timeframes.iter() {
        let tf_spec = parse_timeframe_label(tf)?;
        let candles = if tf_spec.label.eq_ignore_ascii_case("1m") {
            base.clone()
        } else {
            resample_candles(&base, tf_spec.interval_ms)?
        };
        let (train, holdout) = split_candles_chronological(&candles, cfg.holdout_frac)?;

        let mut case_results: Vec<CaseAudit> = Vec::new();
        for strategy_case in strategy_cases.iter() {
            for ma in ma_cases.iter().copied() {
                let pair_count = count_pairs_fast_lt_slow(
                    &expand_range_u16(cfg.fast_range),
                    &expand_range_u16(cfg.slow_range),
                );

                let exact_req = build_request(
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
                    OptimizationMode::Grid,
                    None,
                );
                let exact_res = run_double_ma_optimization_blocking_with_candles(
                    exact_req, &train, &cancel, None,
                )?;
                let mut optimization_audits: Vec<OptimizationFidelityAudit> = Vec::new();
                let mut primary_approx_res: Option<BackendOptimizationResult> = None;
                for approx_mode in [OptimizationMode::GpuVdps, OptimizationMode::CoarseToFine] {
                    let vdps_cfg = matches!(approx_mode, OptimizationMode::GpuVdps)
                        .then(|| default_vdps_config(cfg.top_k));
                    let approx_req = build_request(
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
                        approx_mode,
                        vdps_cfg,
                    );
                    let approx_res = run_double_ma_optimization_blocking_with_candles(
                        approx_req, &train, &cancel, None,
                    )?;
                    optimization_audits.push(build_optimization_audit(
                        ma,
                        strategy_case,
                        &holdout,
                        ObjectiveKind::Sharpe,
                        &exact_res,
                        &approx_res,
                        &cancel,
                    )?);
                    if primary_approx_res.is_none() {
                        primary_approx_res = Some(approx_res);
                    }
                }
                let optimization = optimization_audits
                    .first()
                    .cloned()
                    .ok_or_else(|| "optimization audits unexpectedly empty".to_string())?;
                let primary_approx_res = primary_approx_res
                    .as_ref()
                    .ok_or_else(|| "primary approximate optimization result missing".to_string())?;

                let candidate_pool = build_validation_candidate_pool(
                    ma,
                    strategy_case,
                    &exact_res,
                    primary_approx_res,
                    cfg.candidate_pool,
                );
                if candidate_pool.is_empty() {
                    return Err("validation candidate pool unexpectedly empty".to_string());
                }
                let comparison_top_k = cfg.comparison_top_k.min(candidate_pool.len()).max(1);
                let (baseline_preset, cheaper_preset) =
                    derive_validation_presets(train.close.len(), candidate_pool.len());

                let baseline_validation = run_validation_with_preset(
                    "baseline",
                    tf,
                    ma,
                    strategy_case,
                    &train,
                    &candidate_pool,
                    &baseline_preset,
                    &cancel,
                )?;
                let cheaper_validation = run_validation_with_preset(
                    "cheaper",
                    tf,
                    ma,
                    strategy_case,
                    &train,
                    &candidate_pool,
                    &cheaper_preset,
                    &cancel,
                )?;
                let holdout_top_candidate_eval = evaluate_validation_top_candidate_holdout(
                    ma,
                    &holdout,
                    &baseline_validation,
                    &cheaper_validation,
                    &cancel,
                )?;
                let validation = build_validation_audit(
                    candidate_pool,
                    comparison_top_k,
                    &baseline_preset,
                    &baseline_validation,
                    &cheaper_preset,
                    &cheaper_validation,
                    holdout_top_candidate_eval,
                );

                case_results.push(CaseAudit {
                    strategy_case: strategy_case.label.to_string(),
                    ma_case: ma.label.to_string(),
                    objective: "Sharpe".to_string(),
                    pair_count,
                    optimization,
                    optimization_audits,
                    validation,
                });
            }
        }

        timeframe_results.push(TimeframeAudit {
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

    let artifact = FidelityAuditArtifact {
        csv_path: artifact_path_label(&cfg.csv_path),
        timeframes: timeframe_results,
    };

    let json = serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?;
    atomic_output::write_str(&cfg.out_path, &json)?;
    println!(
        "Wrote fidelity audit artifact to {}",
        cfg.out_path.to_string_lossy()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn parse_args_from_iter<I, S>(args: I) -> Result<CliConfig, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut cfg = CliConfig {
        csv_path: PathBuf::from("data/benchmarks/AAPL.csv"),
        timeframes: vec!["15m".to_string(), "1h".to_string()],
        fast_range: (5, 200, 1),
        slow_range: (10, 400, 1),
        top_k: 16,
        candidate_pool: 24,
        comparison_top_k: 5,
        holdout_frac: 0.20,
        device_id: 0,
        out_path: PathBuf::from("output/reports/real_data_fidelity_audit_aapl.json"),
    };

    let mut args = args.into_iter().map(Into::into);
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
            "--candidate-pool" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --candidate-pool".to_string());
                };
                cfg.candidate_pool = v
                    .parse::<usize>()
                    .map_err(|_| "invalid --candidate-pool".to_string())?;
            }
            "--comparison-top-k" => {
                let Some(v) = args.next() else {
                    return Err("missing value for --comparison-top-k".to_string());
                };
                cfg.comparison_top_k = v
                    .parse::<usize>()
                    .map_err(|_| "invalid --comparison-top-k".to_string())?;
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
    if cfg.top_k == 0 {
        return Err("--top-k must be at least 1".to_string());
    }
    if cfg.candidate_pool == 0 {
        return Err("--candidate-pool must be at least 1".to_string());
    }
    if cfg.comparison_top_k == 0 {
        return Err("--comparison-top-k must be at least 1".to_string());
    }
    if !(0.05..0.50).contains(&cfg.holdout_frac) {
        return Err("--holdout-frac should usually be in [0.05, 0.50)".to_string());
    }
    Ok(cfg)
}

fn parse_args() -> Result<CliConfig, String> {
    parse_args_from_iter(std::env::args().skip(1))
}

fn print_help() {
    println!("real_data_fidelity_audit");
    println!("  --csv <path>              Default: data/benchmarks/AAPL.csv");
    println!("  --timeframes <list>       Default: 15m,1h");
    println!("  --fast-range a:b:c        Default: 5:200:1");
    println!("  --slow-range a:b:c        Default: 10:400:1");
    println!("  --top-k <n>               Default: 16");
    println!("  --candidate-pool <n>      Default: 24");
    println!("  --comparison-top-k <n>    Default: 5");
    println!("  --holdout-frac <f>        Default: 0.20");
    println!("  --device <id>             Default: 0");
    println!(
        "  --out <path>              Default: output/reports/real_data_fidelity_audit_aapl.json"
    );
}

fn build_optimization_audit(
    ma: MaCase,
    strategy_case: &StrategyCase,
    holdout: &Candles,
    objective: ObjectiveKind,
    exact: &BackendOptimizationResult,
    approx: &BackendOptimizationResult,
    cancel: &AtomicBool,
) -> Result<OptimizationFidelityAudit, String> {
    let (approx_rank, topk_overlap_ratio, approx_best_score_regret) =
        compare_against_exact(exact, approx, objective)?;
    let holdout_winner_eval =
        evaluate_optimization_winner_holdout(ma, strategy_case, holdout, exact, approx, cancel)?;
    let holdout_regret_fallback = {
        let exact_holdout_score =
            metrics_summary_objective_score(&holdout_winner_eval.exact_best, objective);
        let approx_holdout_score =
            metrics_summary_objective_score(&holdout_winner_eval.approx_best, objective);
        (exact_holdout_score.is_finite() && approx_holdout_score.is_finite())
            .then_some((exact_holdout_score - approx_holdout_score).max(0.0))
    };
    let (approx_mode, workload_family, contract_summary) =
        optimization_search_semantics_fields(approx);

    Ok(OptimizationFidelityAudit {
        approx_mode,
        workload_family,
        contract_summary,
        exact_runtime_ms: exact.runtime_ms,
        approx_runtime_ms: approx.runtime_ms,
        approx_speedup_vs_exact: ratio(exact.runtime_ms, approx.runtime_ms),
        approx_selected_exact_best: approx_rank == Some(1),
        approx_best_exact_rank: approx_rank,
        approx_best_score_regret,
        topk_overlap_ratio,
        exact_best: candidate_summary(ma, &exact.best_params, &exact.best_metrics, objective),
        approx_best: candidate_summary(ma, &approx.best_params, &approx.best_metrics, objective),
        holdout_winner_eval,
        vdps: approx.vdps.as_ref().map(|vdps| VdpsAuditSummary {
            eta: vdps.eta,
            rungs: vdps.rungs.clone(),
            full_grid_pairs: vdps.full_grid_pairs,
            full_fidelity_pairs_evaluated: vdps.full_fidelity_pairs_evaluated,
            full_fidelity_pairs_saved: vdps.full_fidelity_pairs_saved,
            evaluated_pairs_total: vdps.evaluated_pairs_total,
            runtime_ms_total: vdps.runtime_ms_total,
            quality_check_top_k_overlap: vdps.quality_check_top_k_overlap,
            quality_check_best_score_regret: vdps.quality_check_best_score_regret,
            quality_check_exact_winner_rank: vdps.quality_check_exact_winner_rank.or(approx_rank),
            quality_check_holdout_regret: vdps
                .quality_check_holdout_regret
                .or(holdout_regret_fallback),
            quality_check_speedup_vs_exact: vdps
                .quality_check_speedup_vs_exact
                .or(ratio(exact.runtime_ms, approx.runtime_ms)),
            quality_check_baseline_runtime_ms: vdps.quality_check_baseline_runtime_ms,
            quality_check_baseline_backend: vdps.quality_check_baseline_backend.clone(),
            quality_check_baseline_pairs: vdps.quality_check_baseline_pairs,
        }),
    })
}

fn optimization_search_semantics_fields(
    approx: &BackendOptimizationResult,
) -> (String, String, String) {
    if let Some(search_semantics) = approx.search_semantics.as_ref() {
        return (
            search_semantics.approx_mode.clone(),
            search_semantics.workload_family.clone(),
            search_semantics.contract_summary.clone(),
        );
    }

    let workload_family = match approx.mode_used {
        ta_app_core::OptimizationModeResolved::Grid => "double_ma_exact_grid_reduced_output",
        ta_app_core::OptimizationModeResolved::CoarseToFine => "double_ma_coarse_to_fine",
        ta_app_core::OptimizationModeResolved::GpuVdps => "double_ma_gpu_vdps",
    };
    (
        "legacy_unlabeled".to_string(),
        workload_family.to_string(),
        String::new(),
    )
}

fn build_validation_audit(
    candidate_pool: Vec<DoubleMaValidationParams>,
    comparison_top_k: usize,
    baseline_preset: &ValidationPreset,
    baseline_result: &ta_app_core::DoubleMaValidationResult,
    cheaper_preset: &ValidationPreset,
    cheaper_result: &ta_app_core::DoubleMaValidationResult,
    holdout_top_candidate_eval: ValidationTopCandidateHoldoutEval,
) -> ValidationFidelityAudit {
    let candidate_pool_keys: Vec<CandidateKey> = candidate_pool
        .iter()
        .map(candidate_key_from_validation)
        .collect();
    let baseline_rows = validation_rows(baseline_result);
    let cheaper_rows = validation_rows(cheaper_result);
    let agreement = compare_validation_rows(&baseline_rows, &cheaper_rows, comparison_top_k);

    ValidationFidelityAudit {
        candidate_pool_size: candidate_pool_keys.len(),
        comparison_top_k,
        candidate_pool: candidate_pool_keys,
        baseline: summarize_validation_run(baseline_preset, baseline_result),
        cheaper: summarize_validation_run(cheaper_preset, cheaper_result),
        top_candidate_match: agreement.top_candidate_match,
        cheaper_top_candidate_baseline_rank: agreement.cheaper_top_candidate_baseline_rank,
        cheaper_top_candidate_baseline_regret: agreement.cheaper_top_candidate_baseline_regret,
        topk_overlap_ratio: agreement.topk_overlap_ratio,
        hard_gate_overlap_ratio: agreement.hard_gate_overlap_ratio,
        approved_overlap_ratio: agreement.approved_overlap_ratio,
        candidate_hard_gate_disagreements: agreement.candidate_hard_gate_disagreements,
        candidate_approval_disagreements: agreement.candidate_approval_disagreements,
        run_passed_match: baseline_result.run_verdict.passed == cheaper_result.run_verdict.passed,
        run_approved_match: baseline_result.run_verdict.approved
            == cheaper_result.run_verdict.approved,
        final_recommendation_match: format!(
            "{:?}",
            baseline_result.run_verdict.final_recommendation
        ) == format!(
            "{:?}",
            cheaper_result.run_verdict.final_recommendation
        ),
        holdout_top_candidate_eval,
    }
}

fn run_validation_with_preset(
    label: &str,
    timeframe: &str,
    ma: MaCase,
    strategy_case: &StrategyCase,
    candles: &Candles,
    candidates: &[DoubleMaValidationParams],
    preset: &ValidationPreset,
    cancel: &AtomicBool,
) -> Result<ta_app_core::DoubleMaValidationResult, String> {
    let req = DoubleMaValidationRequest {
        data_id: format!("fidelity_audit_{label}"),
        backend: Backend::CpuOnly,
        ma_source: "close".to_string(),
        dynamic_strategy_ir: None,
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
        candidates: candidates.to_vec(),
        walk_forward: Some(preset.walk_forward.clone()),
        reopt_walk_forward: Some(preset.reopt_walk_forward.clone()),
        spa: Some(preset.spa.clone()),
        perturb: None,
        trade_bootstrap: None,
        monte_carlo: None,
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: preset.cscv_pbo.slices,
            max_combinations: preset.cscv_pbo.max_combinations,
            threshold: preset.cscv_pbo.threshold,
        }),
        seed: 1,
        max_candidates: candidates.len().max(1),
        validation_engine: ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: Some(ValidationArtifactContext {
            strategy_kind: strategy_case.label.to_string(),
            csv_path: None,
            timeframe: Some(timeframe.to_string()),
            label: Some(label.to_string()),
            optimization_trials: None,
            search_distribution: None,
            data_realism: None,
        }),
    };
    run_double_ma_validation_blocking_with_candles(req, candles, cancel, None)
}

fn derive_validation_presets(
    bars: usize,
    candidate_pool_size: usize,
) -> (ValidationPreset, ValidationPreset) {
    let window = derive_validation_window_bars(bars);
    let baseline_step = (window / 2).max(64);
    let cheaper_step = window.max(baseline_step);
    let baseline_reopt_top_n = candidate_pool_size.clamp(1, 8);
    let cheaper_reopt_top_n = candidate_pool_size.clamp(1, 4);
    let baseline_cscv_slices = derive_even_cscv_slices(bars, 16);
    let cheaper_cscv_slices = derive_even_cscv_slices(bars, baseline_cscv_slices.min(8));
    let baseline_spa_slices = derive_spa_slices(bars, 16);
    let cheaper_spa_slices = derive_spa_slices(bars, baseline_spa_slices.min(8));

    let baseline = ValidationPreset {
        label: "baseline",
        walk_forward: WalkForwardConfigOptional {
            window_bars: window,
            step_bars: baseline_step,
        },
        reopt_walk_forward: ReoptWalkForwardConfigOptional {
            window_bars: window,
            step_bars: baseline_step,
            top_n_candidates: baseline_reopt_top_n,
        },
        spa: SpaConfigOptional {
            slices: baseline_spa_slices,
            bootstrap_samples: 300,
            block_length: 4,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        },
        cscv_pbo: CscvPboConfig {
            slices: baseline_cscv_slices,
            max_combinations: 50_000,
            threshold: 0.0,
        },
    };
    let cheaper = ValidationPreset {
        label: "cheaper",
        walk_forward: WalkForwardConfigOptional {
            window_bars: window,
            step_bars: cheaper_step,
        },
        reopt_walk_forward: ReoptWalkForwardConfigOptional {
            window_bars: window,
            step_bars: cheaper_step,
            top_n_candidates: cheaper_reopt_top_n,
        },
        spa: SpaConfigOptional {
            slices: cheaper_spa_slices,
            bootstrap_samples: 100,
            block_length: 4,
            benchmark: SpaBenchmarkKind::BuyAndHold,
        },
        cscv_pbo: CscvPboConfig {
            slices: cheaper_cscv_slices,
            max_combinations: 10_000,
            threshold: 0.0,
        },
    };
    (baseline, cheaper)
}

fn derive_validation_window_bars(bars: usize) -> usize {
    let max_window = bars.saturating_sub(2).max(64);
    let target = ((bars as f64) * 0.35).round() as usize;
    target.clamp(max_window.min(256), max_window.min(20_000))
}

fn derive_even_cscv_slices(bars: usize, cap: usize) -> usize {
    let raw = (bars / 256).clamp(4, cap.max(4));
    if raw % 2 == 0 {
        raw
    } else {
        raw.saturating_sub(1).max(4)
    }
}

fn derive_spa_slices(bars: usize, cap: usize) -> usize {
    (bars / 256).clamp(4, cap.max(4))
}

fn summarize_validation_run(
    preset: &ValidationPreset,
    result: &ta_app_core::DoubleMaValidationResult,
) -> ValidationRunSummary {
    let ranked_candidates: Vec<ValidationCandidateAuditSummary> = result
        .candidates
        .iter()
        .enumerate()
        .map(|(idx, candidate)| ValidationCandidateAuditSummary {
            rank: idx + 1,
            candidate: candidate_key_from_validation(&candidate.params),
            robust_score: candidate.robust_score,
            passed_hard_gates: candidate.validation_verdict.passed,
            approved: candidate_is_approved(candidate),
            track_record_bars: candidate.track_record_bars,
            min_track_record_length_bars: candidate.min_track_record_length_bars,
            psr: candidate.psr,
            dsr: candidate.dsr,
            walk_forward_sharpe_median: candidate
                .walk_forward
                .as_ref()
                .map(|stats| stats.sharpe_median),
            walk_forward_sharpe_worst: candidate
                .walk_forward
                .as_ref()
                .map(|stats| stats.sharpe_worst),
            cscv_oos_metric_p50: candidate
                .cscv_pbo
                .as_ref()
                .and_then(|stats| stats.oos_metric_p50),
            cscv_oos_rank_frac_p50: candidate
                .cscv_pbo
                .as_ref()
                .and_then(|stats| stats.oos_rank_frac_p50),
            warning_flags: candidate.validation_verdict.warning_flags.clone(),
            fail_reasons: candidate.validation_verdict.fail_reasons.clone(),
            in_sample: metrics_summary(&candidate.in_sample),
        })
        .collect();

    ValidationRunSummary {
        label: preset.label.to_string(),
        runtime_ms: result.runtime_ms,
        backend_used: format!("{:?}", result.backend_used),
        backend_route: result.backend_route.clone(),
        backend_fallback_reason: result.backend_fallback_reason.clone(),
        preset: ValidationPresetSummary {
            walk_forward_window_bars: preset.walk_forward.window_bars,
            walk_forward_step_bars: preset.walk_forward.step_bars,
            reopt_window_bars: preset.reopt_walk_forward.window_bars,
            reopt_step_bars: preset.reopt_walk_forward.step_bars,
            reopt_top_n_candidates: preset.reopt_walk_forward.top_n_candidates,
            cscv_slices: preset.cscv_pbo.slices,
            cscv_max_combinations: preset.cscv_pbo.max_combinations,
            cscv_threshold: preset.cscv_pbo.threshold,
            spa_slices: preset.spa.slices,
            spa_bootstrap_samples: preset.spa.bootstrap_samples,
            spa_block_length: preset.spa.block_length,
            spa_benchmark: match preset.spa.benchmark {
                SpaBenchmarkKind::Flat => "flat".to_string(),
                SpaBenchmarkKind::BuyAndHold => "buy_and_hold".to_string(),
            },
        },
        run_verdict: ValidationRunVerdictSummary {
            passed: result.run_verdict.passed,
            approved: result.run_verdict.approved,
            candidates_tested: result.run_verdict.candidates_tested,
            candidates_survived_hard_gates: result.run_verdict.candidates_survived_hard_gates,
            approved_candidate_count: result.run_verdict.approved_candidate_count,
            final_recommendation: format!("{:?}", result.run_verdict.final_recommendation),
            warning_flags: result.run_verdict.warning_flags.clone(),
            fail_reasons: result.run_verdict.fail_reasons.clone(),
            first_approved_rank: result.run_verdict.first_approved_rank,
            pbo: result.run_verdict.pbo,
            max_pbo: result.run_verdict.max_pbo,
        },
        top_candidate: ranked_candidates.first().cloned(),
        ranked_candidates,
    }
}

fn candidate_is_approved(
    candidate: &ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult,
) -> bool {
    candidate.validation_verdict.passed
        && candidate
            .min_track_record_length_bars
            .map(|mintrl| candidate.track_record_bars.max(2) >= mintrl)
            .unwrap_or(false)
}

fn validation_rows(result: &ta_app_core::DoubleMaValidationResult) -> Vec<ValidationRow> {
    result
        .candidates
        .iter()
        .map(|candidate| ValidationRow {
            key: candidate_key_from_validation(&candidate.params),
            robust_score: candidate.robust_score,
            passed_hard_gates: candidate.validation_verdict.passed,
            approved: candidate_is_approved(candidate),
        })
        .collect()
}

#[derive(Default)]
struct ValidationAgreement {
    top_candidate_match: bool,
    cheaper_top_candidate_baseline_rank: Option<usize>,
    cheaper_top_candidate_baseline_regret: Option<f64>,
    topk_overlap_ratio: f64,
    hard_gate_overlap_ratio: f64,
    approved_overlap_ratio: f64,
    candidate_hard_gate_disagreements: usize,
    candidate_approval_disagreements: usize,
}

fn compare_validation_rows(
    baseline: &[ValidationRow],
    cheaper: &[ValidationRow],
    comparison_top_k: usize,
) -> ValidationAgreement {
    let baseline_top = baseline.first().map(|row| &row.key);
    let cheaper_top = cheaper.first().map(|row| &row.key);
    let baseline_best_score = baseline.first().map(|row| row.robust_score).unwrap_or(0.0);

    let baseline_rank_by_key: HashMap<CandidateKey, usize> = baseline
        .iter()
        .enumerate()
        .map(|(idx, row)| (row.key.clone(), idx + 1))
        .collect();
    let baseline_score_by_key: HashMap<CandidateKey, f64> = baseline
        .iter()
        .map(|row| (row.key.clone(), row.robust_score))
        .collect();
    let cheaper_pass_by_key: HashMap<CandidateKey, bool> = cheaper
        .iter()
        .map(|row| (row.key.clone(), row.passed_hard_gates))
        .collect();
    let cheaper_approved_by_key: HashMap<CandidateKey, bool> = cheaper
        .iter()
        .map(|row| (row.key.clone(), row.approved))
        .collect();

    let cheaper_top_candidate_baseline_rank =
        cheaper_top.and_then(|key| baseline_rank_by_key.get(key).copied());
    let cheaper_top_candidate_baseline_regret = cheaper_top.and_then(|key| {
        baseline_score_by_key
            .get(key)
            .map(|score| (baseline_best_score - *score).max(0.0))
    });

    let topk_overlap_ratio = ranked_overlap_ratio(
        &baseline
            .iter()
            .map(|row| row.key.clone())
            .collect::<Vec<_>>(),
        &cheaper
            .iter()
            .map(|row| row.key.clone())
            .collect::<Vec<_>>(),
        comparison_top_k,
    );

    let baseline_hard_gate: HashSet<CandidateKey> = baseline
        .iter()
        .filter(|row| row.passed_hard_gates)
        .map(|row| row.key.clone())
        .collect();
    let cheaper_hard_gate: HashSet<CandidateKey> = cheaper
        .iter()
        .filter(|row| row.passed_hard_gates)
        .map(|row| row.key.clone())
        .collect();
    let baseline_approved: HashSet<CandidateKey> = baseline
        .iter()
        .filter(|row| row.approved)
        .map(|row| row.key.clone())
        .collect();
    let cheaper_approved: HashSet<CandidateKey> = cheaper
        .iter()
        .filter(|row| row.approved)
        .map(|row| row.key.clone())
        .collect();

    let candidate_hard_gate_disagreements = baseline
        .iter()
        .filter(|row| {
            cheaper_pass_by_key
                .get(&row.key)
                .copied()
                .map(|other| other != row.passed_hard_gates)
                .unwrap_or(false)
        })
        .count();
    let candidate_approval_disagreements = baseline
        .iter()
        .filter(|row| {
            cheaper_approved_by_key
                .get(&row.key)
                .copied()
                .map(|other| other != row.approved)
                .unwrap_or(false)
        })
        .count();

    ValidationAgreement {
        top_candidate_match: baseline_top == cheaper_top,
        cheaper_top_candidate_baseline_rank,
        cheaper_top_candidate_baseline_regret,
        topk_overlap_ratio,
        hard_gate_overlap_ratio: jaccard_ratio(&baseline_hard_gate, &cheaper_hard_gate),
        approved_overlap_ratio: jaccard_ratio(&baseline_approved, &cheaper_approved),
        candidate_hard_gate_disagreements,
        candidate_approval_disagreements,
    }
}

fn ranked_overlap_ratio(left: &[CandidateKey], right: &[CandidateKey], limit: usize) -> f64 {
    let denom = limit.min(left.len().max(right.len())).max(1);
    let left_set: HashSet<&CandidateKey> = left.iter().take(denom).collect();
    let right_set: HashSet<&CandidateKey> = right.iter().take(denom).collect();
    left_set.intersection(&right_set).count() as f64 / denom as f64
}

fn jaccard_ratio(set_a: &HashSet<CandidateKey>, set_b: &HashSet<CandidateKey>) -> f64 {
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    let union = set_a.union(set_b).count().max(1);
    let intersection = set_a.intersection(set_b).count();
    intersection as f64 / union as f64
}

fn evaluate_validation_top_candidate_holdout(
    ma: MaCase,
    holdout: &Candles,
    baseline: &ta_app_core::DoubleMaValidationResult,
    cheaper: &ta_app_core::DoubleMaValidationResult,
    cancel: &AtomicBool,
) -> Result<ValidationTopCandidateHoldoutEval, String> {
    let mut candidates: Vec<DoubleMaValidationParams> = Vec::new();
    let mut seen: HashSet<CandidateKey> = HashSet::new();
    for source in [
        baseline.candidates.first().map(|cand| &cand.params),
        cheaper.candidates.first().map(|cand| &cand.params),
    ] {
        if let Some(params) = source {
            let key = candidate_key_from_validation(params);
            if seen.insert(key) {
                candidates.push(params.clone());
            }
        }
    }

    if candidates.is_empty() {
        return Ok(ValidationTopCandidateHoldoutEval {
            bars: holdout.close.len(),
            baseline_top_candidate: None,
            cheaper_top_candidate: None,
            cheaper_top_candidate_sharpe_regret_vs_baseline: None,
            cheaper_top_candidate_pnl_regret_vs_baseline: None,
        });
    }

    let req = DoubleMaValidationRequest {
        data_id: "validation_holdout_top_candidates".to_string(),
        backend: Backend::CpuOnly,
        ma_source: "close".to_string(),
        dynamic_strategy_ir: None,
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
        candidates,
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        monte_carlo: None,
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
    let holdout_by_key: HashMap<CandidateKey, MetricsSummary> = res
        .candidates
        .into_iter()
        .map(|candidate| {
            (
                candidate_key_from_validation(&candidate.params),
                metrics_summary(&candidate.in_sample),
            )
        })
        .collect();

    let baseline_top = baseline
        .candidates
        .first()
        .map(|candidate| candidate_key_from_validation(&candidate.params))
        .and_then(|key| {
            holdout_by_key
                .get(&key)
                .cloned()
                .map(|metrics| HoldoutCandidateEval {
                    candidate: key,
                    metrics,
                })
        });
    let cheaper_top = cheaper
        .candidates
        .first()
        .map(|candidate| candidate_key_from_validation(&candidate.params))
        .and_then(|key| {
            holdout_by_key
                .get(&key)
                .cloned()
                .map(|metrics| HoldoutCandidateEval {
                    candidate: key,
                    metrics,
                })
        });

    let sharpe_regret = match (&baseline_top, &cheaper_top) {
        (Some(base), Some(cheap)) => Some((base.metrics.sharpe - cheap.metrics.sharpe).max(0.0)),
        _ => None,
    };
    let pnl_regret = match (&baseline_top, &cheaper_top) {
        (Some(base), Some(cheap)) => Some((base.metrics.pnl - cheap.metrics.pnl).max(0.0)),
        _ => None,
    };

    Ok(ValidationTopCandidateHoldoutEval {
        bars: holdout.close.len(),
        baseline_top_candidate: baseline_top,
        cheaper_top_candidate: cheaper_top,
        cheaper_top_candidate_sharpe_regret_vs_baseline: sharpe_regret,
        cheaper_top_candidate_pnl_regret_vs_baseline: pnl_regret,
    })
}

fn build_validation_candidate_pool(
    ma: MaCase,
    strategy_case: &StrategyCase,
    exact: &BackendOptimizationResult,
    approx: &BackendOptimizationResult,
    limit: usize,
) -> Vec<DoubleMaValidationParams> {
    let mut out: Vec<DoubleMaValidationParams> = Vec::new();
    let mut seen: HashSet<CandidateKey> = HashSet::new();

    for (params, _) in exact.top.iter().chain(approx.top.iter()) {
        if out.len() >= limit {
            break;
        }
        let key = candidate_key(params);
        if seen.insert(key) {
            out.push(validation_params_from_result(ma, strategy_case, params));
        }
    }

    if out.is_empty() {
        for params in [&exact.best_params, &approx.best_params] {
            if out.len() >= limit {
                break;
            }
            let key = candidate_key(params);
            if seen.insert(key) {
                out.push(validation_params_from_result(ma, strategy_case, params));
            }
        }
    }

    out
}

fn default_vdps_config(top_k: usize) -> VdpsConfig {
    VdpsConfig {
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
        quality_check_top_k: Some(top_k as u32),
    }
}

fn compare_against_exact(
    exact: &BackendOptimizationResult,
    approx: &BackendOptimizationResult,
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

    let approx_rank = ranked
        .iter()
        .position(|(p, _)| {
            p.fast_len == approx.best_params.fast_len
                && p.slow_len == approx.best_params.slow_len
                && p.fast_ma_type == approx.best_params.fast_ma_type
                && p.slow_ma_type == approx.best_params.slow_ma_type
        })
        .map(|idx| idx + 1);

    let top_limit = exact.top.len().max(approx.top.len()).max(1);
    let exact_top: Vec<CandidateKey> = ranked
        .iter()
        .take(top_limit)
        .map(|(p, _)| candidate_key(p))
        .collect();
    let approx_top: Vec<CandidateKey> = approx.top.iter().map(|(p, _)| candidate_key(p)).collect();
    let top_overlap = ranked_overlap_ratio(&exact_top, &approx_top, top_limit);
    let regret = (objective_score(&exact.best_metrics, objective)
        - objective_score(&approx.best_metrics, objective))
    .max(0.0);

    Ok((approx_rank, top_overlap, regret))
}

fn evaluate_optimization_winner_holdout(
    ma: MaCase,
    strategy_case: &StrategyCase,
    holdout: &Candles,
    exact_res: &BackendOptimizationResult,
    approx_res: &BackendOptimizationResult,
    cancel: &AtomicBool,
) -> Result<OptimizationHoldoutEval, String> {
    let req = DoubleMaValidationRequest {
        data_id: "holdout_optimization_winners".to_string(),
        backend: Backend::CpuOnly,
        ma_source: "close".to_string(),
        dynamic_strategy_ir: None,
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
            validation_params_from_result(ma, strategy_case, &exact_res.best_params),
            validation_params_from_result(ma, strategy_case, &approx_res.best_params),
        ],
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        monte_carlo: None,
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
        return Err("unexpected optimization holdout candidate count".to_string());
    }
    Ok(OptimizationHoldoutEval {
        bars: holdout.close.len(),
        exact_best: metrics_summary(&res.candidates[0].in_sample),
        approx_best: metrics_summary(&res.candidates[1].in_sample),
    })
}

fn split_candles_chronological(
    candles: &Candles,
    holdout_frac: f64,
) -> Result<(Candles, Candles), String> {
    let n = candles.close.len();
    if n < 1000 {
        return Err("dataset too small for fidelity audit".to_string());
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
    mode: OptimizationMode,
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
        data_id: "real_fidelity_audit".to_string(),
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
        mode,
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
        metric_contract: None,
    }
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

fn metrics_summary_objective_score(metrics: &MetricsSummary, objective: ObjectiveKind) -> f64 {
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
    if start == 0 || end == 0 || step == 0 || start > end {
        return Err(format!(
            "invalid {name} '{raw}': expected positive start:end:step with start <= end"
        ));
    }
    Ok((start, end, step))
}

fn artifact_path_label(path: &std::path::Path) -> String {
    let cwd = std::env::current_dir().ok();
    let absolute = path.canonicalize().unwrap_or_else(|_| {
        cwd.as_ref()
            .map_or_else(|| path.to_path_buf(), |base| base.join(path))
    });
    if let Some(cwd) = cwd.and_then(|value| value.canonicalize().ok()) {
        if let Ok(relative) = absolute.strip_prefix(&cwd) {
            return relative.to_string_lossy().replace('\\', "/");
        }
    }
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|name| format!("<external>/{name}"))
        .unwrap_or_else(|| "<external>".to_string())
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

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(fast_len: u16, slow_len: u16) -> CandidateKey {
        CandidateKey {
            fast_len,
            slow_len,
            fast_ma: "sma".to_string(),
            slow_ma: "sma".to_string(),
        }
    }

    fn backend_result_with_search_semantics(
        mode_used: ta_app_core::OptimizationModeResolved,
        approx_mode: &str,
        workload_family: &str,
    ) -> BackendOptimizationResult {
        BackendOptimizationResult {
            best_params: DoubleMaParamsResolved {
                fast_len: 5,
                slow_len: 20,
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
            },
            best_metrics: Metrics::default(),
            best_score: Some(1.0),
            mode_used,
            backend_used: BackendUsed::Gpu,
            matrix_index: None,
            best_col: None,
            top_cols: None,
            num_combos: 1,
            num_candles: 100,
            runtime_ms: 10,
            top: Vec::new(),
            top_scores: None,
            all: None,
            all_scores: None,
            heatmap: None,
            vdps: None,
            hybrid_timing_ms: None,
            gpu_kernel_timing_ms: None,
            backend_route: None,
            backend_fallback_reason: None,
            route_diagnostics: None,
            indicator_route_plan: None,
            residency: None,
            followup_cache_key: None,
            shortlist_slice_diagnostics: None,
            score_observation_context: None,
            metric_surface_kind: None,
            shortlist_union_telemetry: None,
            search_semantics: Some(ta_app_core::OptimizationSearchSemanticsSummary {
                approx_mode: approx_mode.to_string(),
                workload_family: workload_family.to_string(),
                contract_summary: "contract".to_string(),
                declared_domain: None,
                base_ranker: None,
                rerank_operator: None,
                heatmap_semantics: None,
                search_distribution: None,
                search_distribution_unavailable_reason: None,
            }),
        }
    }

    #[test]
    fn optimization_search_semantics_fields_prefers_persisted_contract_labels() {
        let result = backend_result_with_search_semantics(
            ta_app_core::OptimizationModeResolved::GpuVdps,
            "heuristic_reduced_output",
            "double_ma_gpu_vdps",
        );

        let (approx_mode, workload_family, contract_summary) =
            optimization_search_semantics_fields(&result);

        assert_eq!(approx_mode, "heuristic_reduced_output");
        assert_eq!(workload_family, "double_ma_gpu_vdps");
        assert_eq!(contract_summary, "contract");
    }

    #[test]
    fn compare_validation_rows_reports_rank_overlap_and_disagreements() {
        let baseline = vec![
            ValidationRow {
                key: key(5, 20),
                robust_score: 1.0,
                passed_hard_gates: true,
                approved: true,
            },
            ValidationRow {
                key: key(6, 24),
                robust_score: 0.9,
                passed_hard_gates: true,
                approved: false,
            },
            ValidationRow {
                key: key(8, 30),
                robust_score: 0.8,
                passed_hard_gates: false,
                approved: false,
            },
        ];
        let cheaper = vec![
            ValidationRow {
                key: key(6, 24),
                robust_score: 0.95,
                passed_hard_gates: true,
                approved: true,
            },
            ValidationRow {
                key: key(5, 20),
                robust_score: 0.85,
                passed_hard_gates: true,
                approved: true,
            },
            ValidationRow {
                key: key(8, 30),
                robust_score: 0.7,
                passed_hard_gates: true,
                approved: false,
            },
        ];

        let agreement = compare_validation_rows(&baseline, &cheaper, 2);
        assert!(!agreement.top_candidate_match);
        assert_eq!(agreement.cheaper_top_candidate_baseline_rank, Some(2));
        assert!((agreement.topk_overlap_ratio - 1.0).abs() < 1.0e-9);
        assert!((agreement.hard_gate_overlap_ratio - (2.0 / 3.0)).abs() < 1.0e-9);
        assert!((agreement.approved_overlap_ratio - 0.5).abs() < 1.0e-9);
        assert_eq!(agreement.candidate_hard_gate_disagreements, 1);
        assert_eq!(agreement.candidate_approval_disagreements, 1);
    }

    #[test]
    fn derived_validation_presets_stay_ordered_and_bounded() {
        let (baseline, cheaper) = derive_validation_presets(1_200, 12);
        assert!(baseline.walk_forward.window_bars >= 256);
        assert!(baseline.walk_forward.window_bars <= 1_198);
        assert!(baseline.walk_forward.step_bars <= cheaper.walk_forward.step_bars);
        assert!(cheaper.reopt_walk_forward.top_n_candidates <= 4);
        assert!(baseline.cscv_pbo.slices % 2 == 0);
        assert!(cheaper.cscv_pbo.slices % 2 == 0);
        assert!(cheaper.cscv_pbo.slices <= baseline.cscv_pbo.slices);
        assert!(cheaper.spa.bootstrap_samples < baseline.spa.bootstrap_samples);
    }

    #[test]
    fn parse_args_from_iter_uses_the_documented_defaults() {
        let cfg = parse_args_from_iter(Vec::<String>::new()).expect("default args");
        assert_eq!(cfg.csv_path, PathBuf::from("data/benchmarks/AAPL.csv"));
        assert_eq!(cfg.timeframes, vec!["15m".to_string(), "1h".to_string()]);
        assert_eq!(cfg.fast_range, (5, 200, 1));
        assert_eq!(cfg.slow_range, (10, 400, 1));
        assert_eq!(cfg.top_k, 16);
        assert_eq!(cfg.candidate_pool, 24);
        assert_eq!(cfg.comparison_top_k, 5);
        assert!((cfg.holdout_frac - 0.20).abs() < f64::EPSILON);
        assert_eq!(cfg.device_id, 0);
        assert_eq!(
            cfg.out_path,
            PathBuf::from("output/reports/real_data_fidelity_audit_aapl.json")
        );
    }

    #[test]
    fn parse_args_from_iter_rejects_out_of_band_holdout_fraction() {
        let err = parse_args_from_iter(vec!["--holdout-frac", "0.80"]).unwrap_err();
        assert!(err.contains("--holdout-frac should usually be in [0.05, 0.50)"));
    }

    #[test]
    fn parse_args_from_iter_rejects_invalid_ranges() {
        for args in [
            vec!["--fast-range", "0:10:1"],
            vec!["--fast-range", "10:0:1"],
            vec!["--fast-range", "10:5:1"],
            vec!["--fast-range", "5:10:0"],
        ] {
            let err = parse_args_from_iter(args).unwrap_err();
            assert!(err.contains("expected positive start:end:step with start <= end"));
        }
    }
}
