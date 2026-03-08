use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    meta_group_key_for_request, meta_select_from_artifacts,
    run_composite_cpu_optimization_blocking_with_candles,
    run_composite_validation_blocking_with_candles, Backend, CompositeCpuRequest, CompositeParams,
    CompositeValidationRequest, HoldoutAggregate, HoldoutSummary, MetaClusterConfig,
    MetaGroupingMode, MetaHoldoutMode, MetaScoreWeights, MetaSelectionRequest, ValidationEngine,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

#[derive(Parser, Debug)]
#[command(name = "ta_workflow_regression")]
#[command(about = "Deterministic optimize->validate->meta->holdout workflow regression gate")]
struct Cli {
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    baseline: Option<PathBuf>,
    #[arg(long, default_value_t = 10_000)]
    candles_len: usize,
    #[arg(long, default_value_t = 10)]
    train_assets: usize,
    #[arg(long, default_value_t = 6)]
    holdout_assets: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkflowSummary {
    candles_len: usize,
    train_assets: usize,
    holdout_assets: usize,
    artifacts_used: usize,
    groups: usize,
    top_meta_score: f64,
    robust_score_p10: f64,
    max_dd_p90: f64,
    holdout_score_p10: f64,
    holdout_max_dd_p90: f64,
    holdout_pass_rate: f64,
    runtime_ms: u64,
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn make_synth_candles(len: usize, drift: f64, noise_amp: f64, phase: f64) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);
        let t = i as f64;
        let noise = (t * 0.017 + phase).sin() * noise_amp;
        px *= 1.0 + drift + noise;
        open.push(px);
        high.push(px * (1.0 + noise_amp.abs().min(0.01)));
        low.push(px * (1.0 - noise_amp.abs().min(0.01)));
        close.push(px);
        vol.push(1_000.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn evaluate_holdout_light(
    req: &MetaSelectionRequest,
    groups: &mut [ta_app_core::MetaCandidateAggregate],
    holdout: &[(&str, &Candles)],
    cancel: &AtomicBool,
) -> Result<HoldoutSummary, String> {
    if groups.is_empty() {
        return Ok(HoldoutSummary {
            datasets: 0,
            skipped: holdout.len(),
        });
    }

    let mut idx_by_key: std::collections::HashMap<String, usize> =
        std::collections::HashMap::with_capacity(groups.len());
    for (i, g) in groups.iter().enumerate() {
        idx_by_key.insert(g.key.clone(), i);
    }

    #[derive(Default, Clone)]
    struct Acc {
        scores: Vec<f64>,
        sharpes: Vec<f64>,
        max_dds: Vec<f64>,
        pnls: Vec<f64>,
        passes: usize,
    }

    let mut accs: Vec<Acc> = vec![Acc::default(); groups.len()];
    let mut skipped = 0usize;

    for (label, candles) in holdout.iter().copied() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let g0 = &groups[0];
        let ma_source = g0.ma_source.clone();
        let fast_ma_params = g0.fast_ma_params.clone();
        let slow_ma_params = g0.slow_ma_params.clone();

        let mut candidates: Vec<CompositeParams> = Vec::new();
        candidates.reserve(groups.len());
        for g in groups.iter() {
            let rep = g.representatives.first().unwrap_or(&g.params).clone();
            let p: CompositeParams = serde_json::from_value(rep).map_err(|e| e.to_string())?;
            candidates.push(p);
        }

        let val_req = CompositeValidationRequest {
            data_id: label.to_string(),
            ma_source: ma_source.clone(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
            candidates,
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: groups.len(),
            validation_engine: ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
        };

        let val_res =
            match run_composite_validation_blocking_with_candles(val_req, candles, cancel, None) {
                Ok(v) => v,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };

        let mut local: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); groups.len()];
        for cand in val_res.candidates {
            let params_v = serde_json::to_value(&cand.params).map_err(|e| e.to_string())?;
            let key = meta_group_key_for_request(
                req,
                &params_v,
                &ma_source,
                &fast_ma_params,
                &slow_ma_params,
            );
            let Some(&gi) = idx_by_key.get(&key) else {
                continue;
            };

            let m = cand.iterative_in_sample.as_ref().unwrap_or(&cand.in_sample);
            let sharpe = m.sharpe;
            let max_dd = m.max_dd;
            let pnl = m.pnl;
            let score = sharpe - 2.0 * max_dd;
            local[gi].push((score, sharpe, max_dd, pnl));
        }

        for (gi, rows) in local.into_iter().enumerate() {
            if rows.is_empty() {
                continue;
            }
            let mut scores: Vec<f64> = Vec::with_capacity(rows.len());
            let mut sharpes: Vec<f64> = Vec::with_capacity(rows.len());
            let mut max_dds: Vec<f64> = Vec::with_capacity(rows.len());
            let mut pnls: Vec<f64> = Vec::with_capacity(rows.len());
            for (score, sharpe, max_dd, pnl) in rows {
                scores.push(score);
                sharpes.push(sharpe);
                max_dds.push(max_dd);
                pnls.push(pnl);
            }
            scores.sort_by(|a, b| a.total_cmp(b));
            sharpes.sort_by(|a, b| a.total_cmp(b));
            max_dds.sort_by(|a, b| a.total_cmp(b));
            pnls.sort_by(|a, b| a.total_cmp(b));

            let score = quantile(&scores, 0.5);
            let sharpe = quantile(&sharpes, 0.5);
            let max_dd = max_dds.last().copied().unwrap_or(0.0);
            let pnl = quantile(&pnls, 0.5);
            let acc = &mut accs[gi];
            acc.scores.push(score);
            acc.sharpes.push(sharpe);
            acc.max_dds.push(max_dd);
            acc.pnls.push(pnl);
            if score > 0.0 {
                acc.passes = acc.passes.saturating_add(1);
            }
        }
    }

    let datasets = holdout.len().saturating_sub(skipped);
    if datasets == 0 {
        return Ok(HoldoutSummary {
            datasets: 0,
            skipped,
        });
    }

    for (i, g) in groups.iter_mut().enumerate() {
        let acc = &mut accs[i];
        if acc.scores.is_empty() {
            continue;
        }
        acc.scores.sort_by(|a, b| a.total_cmp(b));
        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.max_dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));

        g.holdout = Some(HoldoutAggregate {
            datasets,
            pass_rate: (acc.passes as f64) / (datasets as f64),
            score_median: quantile(&acc.scores, 0.5),
            score_p10: quantile(&acc.scores, 0.10),
            sharpe_median: quantile(&acc.sharpes, 0.5),
            max_dd_p90: quantile(&acc.max_dds, 0.90),
            pnl_median: quantile(&acc.pnls, 0.5),
        });
    }

    Ok(HoldoutSummary { datasets, skipped })
}

fn run_workflow_once(
    candles_len: usize,
    train_assets: usize,
    holdout_assets: usize,
    cancel: &AtomicBool,
) -> Result<WorkflowSummary, String> {
    let t0 = Instant::now();

    let strategy = StrategyConfig {
        long_only: false,
        allow_flip: true,
        trade_on_next_bar: true,
        ..StrategyConfig::default()
    };
    let halftrend = HalfTrendRegimeFilter {
        amplitude: 2,
        channel_deviation: 2.0,
        atr_period: 100,
    };
    let macz = MaczGateParams::default();
    let ott = OttGateParams::default();

    let total = train_assets.saturating_add(holdout_assets).max(2);
    let mut universe: Vec<(String, Candles)> = Vec::with_capacity(total);
    for i in 0..total {
        let label = format!("SYM{i:04}");
        let drift = match i % 3 {
            0 => 0.00008,
            1 => -0.00008,
            _ => 0.0,
        };
        let noise_amp = match i % 4 {
            0 => 0.001,
            1 => 0.002,
            2 => 0.003,
            _ => 0.0015,
        };
        let phase = i as f64 * 0.31;
        universe.push((
            label,
            make_synth_candles(candles_len, drift, noise_amp, phase),
        ));
    }

    let train = &universe[..train_assets.min(universe.len())];
    let holdout = &universe[train.len()..(train.len() + holdout_assets).min(universe.len())];

    let mut artifacts: Vec<Value> = Vec::with_capacity(train.len());
    for (i, (label, candles)) in train.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let opt_req = CompositeCpuRequest {
            backend: Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            data_id: label.to_string(),
            fast_range: (5, 60, 5),
            slow_range: (20, 220, 10),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: strategy.clone(),
            halftrend: halftrend.clone(),
            macz: macz.clone(),
            ott: ott.clone(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(64),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };
        let opt_res =
            run_composite_cpu_optimization_blocking_with_candles(opt_req, candles, cancel, None)?;

        let mut candidates: Vec<CompositeParams> = Vec::new();
        for (p, _) in opt_res.top.iter().take(64) {
            candidates.push(CompositeParams {
                fast_ma_type: p.fast_ma_type.clone(),
                slow_ma_type: p.slow_ma_type.clone(),
                fast_len: p.fast_len,
                slow_len: p.slow_len,
                halftrend: halftrend.clone(),
                asr: None,
                vemr: None,
                macz: macz.clone(),
                ott: ott.clone(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: strategy.clone(),
            });
        }
        if candidates.is_empty() {
            continue;
        }

        let val_req = CompositeValidationRequest {
            data_id: label.to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates,
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 64,
            validation_engine: ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
        };
        let val_res =
            run_composite_validation_blocking_with_candles(val_req, candles, cancel, None)?;

        artifacts.push(json!({
            "kind": "vectorbt_run_artifact",
            "ts": format!("reg_train_{i:04}"),
            "tags": ["regpack-v1"],
            "run": { "req": { "strategy_kind": "CompositeCpu", "ma_source": "close" }, "res": {} },
            "dataset_meta": { "label": label, "timeframe": "1m", "csv_path": format!("{label}.csv") },
            "validation": { "req": { "ma_source": "close" }, "res": val_res }
        }));
    }

    let meta_req = MetaSelectionRequest {
        strategy_kind: "CompositeCpu".to_string(),
        timeframe: Some("1m".to_string()),
        required_tag: Some("regpack-v1".to_string()),
        holdout_tag: None,
        eval_holdout: false,
        save_holdout_rows: false,
        holdout_eval_top_n: None,
        holdout_eval_explore_n: 0,
        holdout_eval_rank_by: None,
        holdout_mode: MetaHoldoutMode::Light,
        holdout_validation: None,
        top_k: 25,
        min_samples: 1,
        strict_shortlist: false,
        allow_shortlist_fallback: true,
        weights: MetaScoreWeights::default(),
        grouping_mode: MetaGroupingMode::Exact,
        cluster_config: MetaClusterConfig::default(),
        cluster_representatives: 1,
    };
    let mut meta_res = meta_select_from_artifacts(&meta_req, &artifacts, None);

    let holdout_pairs: Vec<(&str, &Candles)> =
        holdout.iter().map(|(l, c)| (l.as_str(), c)).collect();
    let holdout_summary =
        evaluate_holdout_light(&meta_req, &mut meta_res.groups, &holdout_pairs, cancel)?;
    meta_res.holdout = Some(holdout_summary);

    let top = meta_res.groups.first();
    let hold = top.and_then(|g| g.holdout.as_ref());

    Ok(WorkflowSummary {
        candles_len,
        train_assets: train.len(),
        holdout_assets: holdout.len(),
        artifacts_used: meta_res.artifacts_used,
        groups: meta_res.groups.len(),
        top_meta_score: top.map(|g| g.meta_score).unwrap_or(0.0),
        robust_score_p10: top.map(|g| g.robust_score_p10).unwrap_or(0.0),
        max_dd_p90: top.map(|g| g.max_dd_p90).unwrap_or(0.0),
        holdout_score_p10: hold.map(|h| h.score_p10).unwrap_or(0.0),
        holdout_max_dd_p90: hold.map(|h| h.max_dd_p90).unwrap_or(0.0),
        holdout_pass_rate: hold.map(|h| h.pass_rate).unwrap_or(0.0),
        runtime_ms: t0.elapsed().as_millis() as u64,
    })
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let cancel = AtomicBool::new(false);
    let summary = run_workflow_once(
        cli.candles_len,
        cli.train_assets,
        cli.holdout_assets,
        &cancel,
    )?;

    if let Some(parent) = cli.out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let txt = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    std::fs::write(&cli.out, txt).map_err(|e| e.to_string())?;
    println!("Saved summary: {}", cli.out.to_string_lossy());

    if let Some(base_path) = cli.baseline.as_ref() {
        let raw = std::fs::read_to_string(base_path).map_err(|e| e.to_string())?;
        let base: WorkflowSummary = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let tol = 1e-9_f64;

        let mut ok = true;
        let mut gate = |name: &str, pass: bool, msg: String| {
            println!("[{}] {}: {}", if pass { "PASS" } else { "FAIL" }, name, msg);
            if !pass {
                ok = false;
            }
        };

        gate(
            "robust_score_p10_non_decreasing",
            summary.robust_score_p10 + tol >= base.robust_score_p10,
            format!(
                "{:.6} vs baseline {:.6}",
                summary.robust_score_p10, base.robust_score_p10
            ),
        );
        gate(
            "max_dd_p90_non_increasing",
            summary.max_dd_p90 <= base.max_dd_p90 + tol,
            format!(
                "{:.6} vs baseline {:.6}",
                summary.max_dd_p90, base.max_dd_p90
            ),
        );
        gate(
            "holdout_score_p10_non_decreasing",
            summary.holdout_score_p10 + tol >= base.holdout_score_p10,
            format!(
                "{:.6} vs baseline {:.6}",
                summary.holdout_score_p10, base.holdout_score_p10
            ),
        );
        gate(
            "holdout_max_dd_p90_non_increasing",
            summary.holdout_max_dd_p90 <= base.holdout_max_dd_p90 + tol,
            format!(
                "{:.6} vs baseline {:.6}",
                summary.holdout_max_dd_p90, base.holdout_max_dd_p90
            ),
        );
        gate(
            "runtime_not_regressed_50pct",
            (summary.runtime_ms as f64) <= (base.runtime_ms as f64) * 1.5 + 1.0,
            format!("{}ms vs baseline {}ms", summary.runtime_ms, base.runtime_ms),
        );

        if !ok {
            return Err("workflow regression gate failed".to_string());
        }
    }

    Ok(())
}
