use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    meta_group_key_for_request, meta_select_from_artifacts, predict_ml_confidence_for_group,
    run_composite_cpu_optimization_blocking_with_candles,
    run_composite_validation_blocking_with_candles, train_ml_confidence_model_from_meta_artifacts,
    CompositeCpuRequest, CompositeParams, CompositeValidationRequest, HoldoutAggregate,
    HoldoutSummary, MetaCandidateAggregate, MetaClusterConfig, MetaGroupingMode, MetaHoldoutMode,
    MetaScoreWeights, MetaSelectionRequest, MlConfidenceScope, MlConfidenceTrainConfig,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

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

fn evaluate_holdout_light(
    req: &MetaSelectionRequest,
    groups: &mut [MetaCandidateAggregate],
    holdout: &[(&str, &Candles)],
    cancel: &AtomicBool,
) -> Result<HoldoutSummary, String> {
    if groups.is_empty() {
        return Ok(HoldoutSummary {
            datasets: 0,
            skipped: holdout.len(),
        });
    }

    // Index groups by their meta key so we can map validation results back (validation output is sorted).
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

        // For this benchmark, we assume a single MA config across all groups, so we can evaluate
        // all candidate representatives in one validation call per dataset.
        let g0 = &groups[0];
        let ma_source = g0.ma_source.clone();
        let fast_ma_params = g0.fast_ma_params.clone();
        let slow_ma_params = g0.slow_ma_params.clone();

        let mut candidates: Vec<CompositeParams> = Vec::new();
        candidates.reserve(groups.len());
        for g in groups.iter() {
            // Exact grouping ensures at least one representative; fall back to params for safety.
            let rep = g
                .representatives
                .get(0)
                .unwrap_or_else(|| &g.params)
                .clone();
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
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
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

        // Accumulate per-group results for this dataset.
        let mut local: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); groups.len()]; // (score, sharpe, max_dd, pnl)
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

            let sharpe = cand.in_sample.sharpe;
            let max_dd = cand.in_sample.max_dd;
            let pnl = cand.in_sample.pnl;
            let score = sharpe - 2.0 * max_dd;
            local[gi].push((score, sharpe, max_dd, pnl));
        }

        // Reduce multiple representatives down to one dataset-level row per group.
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

        let pass_rate = (acc.passes as f64) / (datasets as f64);
        let score_median = quantile(&acc.scores, 0.5);
        let score_p10 = quantile(&acc.scores, 0.10);
        let sharpe_median = quantile(&acc.sharpes, 0.5);
        let max_dd_p90 = quantile(&acc.max_dds, 0.90);
        let pnl_median = quantile(&acc.pnls, 0.5);

        g.holdout = Some(HoldoutAggregate {
            datasets,
            pass_rate,
            score_median,
            score_p10,
            sharpe_median,
            max_dd_p90,
            pnl_median,
        });
    }

    Ok(HoldoutSummary { datasets, skipped })
}

fn run_full_workflow_once(
    candles_len: usize,
    train_assets: usize,
    holdout_assets: usize,
    cancel: &AtomicBool,
) -> Result<(), String> {
    // This benchmark intentionally uses small inputs by default. The composite CPU path and
    // validation are roughly linear in bar count on CPU, so you can re-run with larger candle
    // counts to empirically validate scaling.

    let strategy = StrategyConfig {
        long_only: false, // allow shorts to avoid degenerate all-negative holdout labels on downtrends
        allow_flip: true,
        trade_on_next_bar: true,
        cost_preset: None,
        fee_bps: 0.0,
        spread_bps: 0.0,
        slippage_bps: 0.0,
        commission: 0.0,
        eps_rel: 0.0,
        ..StrategyConfig::default()
    };

    let halftrend = HalfTrendRegimeFilter {
        amplitude: 2,
        channel_deviation: 2.0,
        atr_period: 100,
    };
    let macz = MaczGateParams::default();
    let ott = OttGateParams::default();

    // Build a small universe with varied drift/noise so holdout labels aren’t single-class.
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

    // 1) Per-asset: composite CPU optimization -> validation -> artifact.
    let mut artifacts: Vec<Value> = Vec::with_capacity(train.len());
    for (i, (label, candles)) in train.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let opt_req = CompositeCpuRequest {
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
        candidates.reserve(opt_res.top.len().min(64).max(1));
        for (p, _) in opt_res.top.iter().take(64) {
            candidates.push(CompositeParams {
                fast_ma_type: p.fast_ma_type.clone(),
                slow_ma_type: p.slow_ma_type.clone(),
                fast_len: p.fast_len,
                slow_len: p.slow_len,
                halftrend: halftrend.clone(),
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
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
        };
        let val_res =
            run_composite_validation_blocking_with_candles(val_req, candles, cancel, None)?;

        artifacts.push(json!({
            "kind": "vectorbt_run_artifact",
            "ts": format!("bench_train_{i:04}"),
            "tags": ["bench"],
            "run": { "req": { "strategy_kind": "CompositeCpu", "ma_source": "close" }, "res": {} },
            "dataset_meta": { "label": label, "timeframe": "1m", "csv_path": format!("{label}.csv") },
            "validation": { "req": { "ma_source": "close" }, "res": val_res }
        }));
    }

    // 2) Post-validation: meta selection.
    let meta_req = MetaSelectionRequest {
        strategy_kind: "CompositeCpu".to_string(),
        timeframe: Some("1m".to_string()),
        required_tag: Some("bench".to_string()),
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

    // 3) Optional: holdout evaluation (light) to create labels.
    let holdout_pairs: Vec<(&str, &Candles)> =
        holdout.iter().map(|(l, c)| (l.as_str(), c)).collect();
    let holdout_summary =
        evaluate_holdout_light(&meta_req, &mut meta_res.groups, &holdout_pairs, cancel)?;
    meta_res.holdout = Some(holdout_summary);

    // 4) Optional: ML training + apply (confidence ranking).
    let meta_artifact = json!({
        "kind": "vectorbt_meta_selection",
        "ts": "bench_meta",
        "req": meta_req,
        "res": meta_res,
    });

    let scope = MlConfidenceScope {
        strategy_kind: "CompositeCpu".to_string(),
        timeframe: "1m".to_string(),
    };

    let base_cfg = {
        let mut cfg = MlConfidenceTrainConfig::default();
        cfg.label.min_holdout_datasets = holdout.len().max(1);
        cfg
    };

    // The benchmark uses synthetic candles. To avoid single-class label failures, we try a small
    // set of increasingly permissive label thresholds until training succeeds.
    let attempts: &[(f64, f64)] = &[(0.75, 0.00), (0.60, -0.10), (0.50, -0.25), (0.50, -1.00)];

    let mut last_err: Option<String> = None;
    let mut model_opt = None;
    for &(min_pass, min_p10) in attempts {
        let mut cfg = base_cfg.clone();
        cfg.label.min_holdout_pass_rate = min_pass;
        cfg.label.min_holdout_score_p10 = min_p10;
        match train_ml_confidence_model_from_meta_artifacts(
            scope.clone(),
            &[meta_artifact.clone()],
            cfg,
        ) {
            Ok((m, _metrics)) => {
                model_opt = Some(m);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }

    let model = model_opt.ok_or_else(|| {
        format!(
            "ml training failed for all label thresholds: {}",
            last_err.unwrap_or_else(|| "unknown".to_string())
        )
    })?;

    // Apply ML confidence (no sorting here; just compute probabilities).
    let groups = meta_artifact
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing meta groups".to_string())?;
    let mut _sum = 0.0f64;
    for g in groups {
        if let Some(p) = predict_ml_confidence_for_group(&model, g) {
            _sum += p;
        }
    }

    Ok(())
}

fn parse_bench_lens() -> Vec<usize> {
    let raw = std::env::var("VTA_E2E_BENCH_LENS").ok();
    let Some(s) = raw.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return vec![10_000];
    };
    let mut out: Vec<usize> = Vec::new();
    for part in s.split(',') {
        if let Ok(v) = part.trim().parse::<usize>() {
            if v >= 512 {
                out.push(v);
            }
        }
    }
    if out.is_empty() {
        vec![10_000]
    } else {
        out
    }
}

fn bench_e2e_workflow_with_ml(c: &mut Criterion) {
    let lens = parse_bench_lens();

    // Keep defaults small so `cargo bench` stays reasonable. Increase via VTA_E2E_BENCH_LENS.
    let train_assets = 12usize;
    let holdout_assets = 8usize;

    let cancel = AtomicBool::new(false);

    for len in lens {
        let mut group = c.benchmark_group(format!(
            "e2e/composite_cpu_validate_meta_holdout_ml/len{len}/train{train_assets}/holdout{holdout_assets}"
        ));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(10);

        group.bench_function("workflow", |b| {
            b.iter(|| {
                run_full_workflow_once(len, train_assets, holdout_assets, &cancel)
                    .expect("e2e workflow");
            })
        });

        group.finish();
    }
}

criterion_group!(benches, bench_e2e_workflow_with_ml);
criterion_main!(benches);
