use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    extract_ml_confidence_group_features, meta_group_key_for_request, meta_select_from_artifacts,
    predict_ml_confidence_for_features, predict_ml_confidence_for_group,
    prepare_ml_confidence_rows_from_meta_artifacts,
    run_composite_cpu_optimization_blocking_with_candles,
    run_composite_validation_blocking_with_candles, train_ml_confidence_model_from_meta_artifacts,
    train_ml_confidence_model_from_prepared_rows, Backend, CompositeCpuRequest, CompositeParams,
    CompositeValidationRequest, HoldoutAggregate, HoldoutSummary, MetaCandidateAggregate,
    MetaClusterConfig, MetaGroupingMode, MetaHoldoutMode, MetaScoreWeights, MetaSelectionRequest,
    MlConfidenceGroupFeatures, MlConfidencePreparedRows, MlConfidenceScope,
    MlConfidenceTrainConfig,
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

fn bench_env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
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
            backend: Backend::CpuOnly,
            ma_source: ma_source.clone(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
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
            max_candidates: groups.len(),
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
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

            let sharpe = cand.in_sample.sharpe;
            let max_dd = cand.in_sample.max_dd;
            let pnl = cand.in_sample.pnl;
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
    let stage_diagnostics = bench_env_flag("VTA_E2E_BENCH_STAGE_DIAGNOSTICS");
    let total_started = Instant::now();
    let mut optimize_total = Duration::ZERO;
    let mut validate_total = Duration::ZERO;

    let strategy = StrategyConfig {
        long_only: false,
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
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
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
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            top_k: Some(64),
            include_all: Some(false),
            heatmap_bins: Some(0),
            include_trade_metrics: false,
            metric_contract: None,
        };

        let optimize_started = Instant::now();
        let opt_res =
            run_composite_cpu_optimization_blocking_with_candles(opt_req, candles, cancel, None)?;
        optimize_total += optimize_started.elapsed();

        let mut candidates: Vec<CompositeParams> = Vec::new();
        candidates.reserve(opt_res.top.len().min(64).max(1));
        for (p, _) in opt_res.top.iter().take(64) {
            candidates.push(CompositeParams {
                fast_ma_type: p.fast_ma_type.clone(),
                slow_ma_type: p.slow_ma_type.clone(),
                fast_len: p.fast_len,
                slow_len: p.slow_len,
                halftrend: halftrend.clone(),
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                vcet: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
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
            backend: Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
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
            max_candidates: 64,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let validate_started = Instant::now();
        let val_res =
            run_composite_validation_blocking_with_candles(val_req, candles, cancel, None)?;
        validate_total += validate_started.elapsed();

        artifacts.push(json!({
            "kind": "vectorgrid_run_artifact",
            "ts": format!("bench_train_{i:04}"),
            "tags": ["bench"],
            "run": { "req": { "strategy_kind": "CompositeCpu", "ma_source": "close" }, "res": {} },
            "dataset_meta": { "label": label, "timeframe": "1m", "csv_path": format!("{label}.csv") },
            "validation": { "req": { "ma_source": "close" }, "res": val_res }
        }));
    }

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

    let meta_select_started = Instant::now();
    let mut meta_res = meta_select_from_artifacts(&meta_req, &artifacts, None);
    let meta_select_total = meta_select_started.elapsed();

    let holdout_pairs: Vec<(&str, &Candles)> =
        holdout.iter().map(|(l, c)| (l.as_str(), c)).collect();
    let holdout_started = Instant::now();
    let holdout_summary =
        evaluate_holdout_light(&meta_req, &mut meta_res.groups, &holdout_pairs, cancel)?;
    let holdout_total = holdout_started.elapsed();
    meta_res.holdout = Some(holdout_summary);

    let meta_artifact = json!({
        "kind": "vectorgrid_meta_selection",
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

    let attempts: &[(f64, f64)] = &[(0.75, 0.00), (0.60, -0.10), (0.50, -0.25), (0.50, -1.00)];

    let mut last_err: Option<String> = None;
    let mut model_opt = None;
    let ml_train_started = Instant::now();
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
    let ml_train_total = ml_train_started.elapsed();

    let model = model_opt.ok_or_else(|| {
        format!(
            "ml training failed for all label thresholds: {}",
            last_err.unwrap_or_else(|| "unknown".to_string())
        )
    })?;

    let groups = meta_artifact
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing meta groups".to_string())?;
    let mut _sum = 0.0f64;
    let ml_apply_started = Instant::now();
    for g in groups {
        if let Some(p) = predict_ml_confidence_for_group(&model, g) {
            _sum += p;
        }
    }
    let ml_apply_total = ml_apply_started.elapsed();

    if stage_diagnostics {
        println!(
            concat!(
                "benchmark stage diagnostics e2e/composite_cpu_validate_meta_holdout_ml: ",
                "candles_len={} train_assets={} holdout_assets={} ",
                "optimize_ms={:.3} validate_ms={:.3} meta_select_ms={:.3} holdout_ms={:.3} ",
                "ml_train_ms={:.3} ml_apply_ms={:.3} total_ms={:.3}"
            ),
            candles_len,
            train_assets,
            holdout_assets,
            optimize_total.as_secs_f64() * 1_000.0,
            validate_total.as_secs_f64() * 1_000.0,
            meta_select_total.as_secs_f64() * 1_000.0,
            holdout_total.as_secs_f64() * 1_000.0,
            ml_train_total.as_secs_f64() * 1_000.0,
            ml_apply_total.as_secs_f64() * 1_000.0,
            total_started.elapsed().as_secs_f64() * 1_000.0,
        );
    }

    Ok(())
}

struct MlHistoryBenchInput {
    scope: MlConfidenceScope,
    cfg: MlConfidenceTrainConfig,
    history: Vec<Value>,
    prepared_rows: MlConfidencePreparedRows,
    apply_features: Vec<MlConfidenceGroupFeatures>,
    prefix_len: usize,
}

fn build_ml_history_bench_input(
    history_metas: usize,
    groups_per_meta: usize,
) -> Result<MlHistoryBenchInput, String> {
    let history_metas = history_metas.max(2);
    let groups_per_meta = groups_per_meta.max(1);

    let scope = MlConfidenceScope {
        strategy_kind: "CompositeCpu".to_string(),
        timeframe: "1m".to_string(),
    };
    let mut cfg = MlConfidenceTrainConfig::default();
    cfg.label.min_holdout_datasets = 1;
    cfg.label.min_holdout_pass_rate = 0.55;
    cfg.label.min_holdout_score_p10 = 0.0;

    let mut history: Vec<Value> = Vec::with_capacity(history_metas);
    for meta_idx in 0..history_metas {
        let mut groups: Vec<Value> = Vec::with_capacity(groups_per_meta);
        for group_idx in 0..groups_per_meta {
            let strong = (meta_idx + group_idx) % 2 == 0;
            let mut rows: Vec<Value> = Vec::with_capacity(8);
            for ds in 0..8usize {
                let pass = if strong { ds != 0 } else { ds == 0 };
                rows.push(json!({
                    "dataset": format!("H{meta_idx:03}_G{group_idx:03}_D{ds:02}"),
                    "score_median": if pass { 0.20 } else { -0.12 },
                    "score_p10": if pass { 0.08 } else { -0.16 },
                    "sharpe_median": if pass { 0.95 } else { -0.45 },
                    "max_dd_worst": if pass { 0.28 } else { 0.62 },
                    "pnl_median": if pass { 1.0 } else { -0.9 },
                    "pass": pass
                }));
            }
            groups.push(json!({
                "samples": 40 + ((group_idx % 5) as u64) * 4,
                "pass_rate": if strong { 0.82 } else { 0.24 },
                "robust_score_p10": if strong { 0.16 } else { -0.10 },
                "robust_score_median": if strong { 0.24 } else { -0.04 },
                "max_dd_p90": if strong { 0.24 } else { 0.58 },
                "cross_score_median": if strong { 0.07 } else { -0.05 },
                "meta_score": if strong { 0.28 } else { -0.18 },
                "holdout": {
                    "datasets": 8,
                    "pass_rate": if strong { 0.875 } else { 0.125 },
                    "score_p10": if strong { 0.08 } else { -0.16 },
                    "score_median": if strong { 0.16 } else { -0.05 },
                    "sharpe_median": if strong { 1.0 } else { -0.3 },
                    "max_dd_p90": if strong { 0.26 } else { 0.57 },
                    "pnl_median": if strong { 1.1 } else { -0.8 }
                },
                "holdout_rows": rows
            }));
        }
        history.push(json!({
            "kind": "vectorgrid_meta_selection",
            "ts": format!("2026-03-{day:02}T00:00:00Z", day = meta_idx + 1),
            "req": { "strategy_kind": "CompositeCpu", "timeframe": "1m" },
            "res": { "strategy_kind": "CompositeCpu", "timeframe": "1m", "groups": groups }
        }));
    }

    let prefix_len = history.len().saturating_sub(1);
    let apply_features = history[prefix_len]
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing benchmark meta groups".to_string())?
        .iter()
        .filter_map(extract_ml_confidence_group_features)
        .collect::<Vec<_>>();
    let prepared_rows =
        prepare_ml_confidence_rows_from_meta_artifacts(&scope, &history, &cfg.label);

    Ok(MlHistoryBenchInput {
        scope,
        cfg,
        history,
        prepared_rows,
        apply_features,
        prefix_len,
    })
}

fn run_ml_history_prefix_once(
    input: &MlHistoryBenchInput,
    use_prepared_rows: bool,
) -> Result<(), String> {
    let (model, _metrics) = if use_prepared_rows {
        train_ml_confidence_model_from_prepared_rows(
            input.scope.clone(),
            &input.prepared_rows,
            input.cfg.clone(),
            Some(input.prefix_len),
        )?
    } else {
        train_ml_confidence_model_from_meta_artifacts(
            input.scope.clone(),
            &input.history[..input.prefix_len],
            input.cfg.clone(),
        )?
    };

    let mut _sum = 0.0f64;
    for features in &input.apply_features {
        if let Some(p) = predict_ml_confidence_for_features(&model, features) {
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

fn parse_bench_asset_count(var_name: &str, default_value: usize) -> usize {
    std::env::var(var_name)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

fn bench_e2e_workflow_with_ml(c: &mut Criterion) {
    let lens = parse_bench_lens();

    let train_assets = parse_bench_asset_count("VTA_E2E_BENCH_TRAIN_ASSETS", 12);
    let holdout_assets = parse_bench_asset_count("VTA_E2E_BENCH_HOLDOUT_ASSETS", 8);
    let history_metas = parse_bench_asset_count("VTA_ML_HISTORY_BENCH_METAS", 32).max(2);
    let groups_per_meta = parse_bench_asset_count("VTA_ML_HISTORY_BENCH_GROUPS_PER_META", 24);
    let stage_diagnostics = bench_env_flag("VTA_E2E_BENCH_STAGE_DIAGNOSTICS");

    let cancel = AtomicBool::new(false);

    for len in lens {
        let group_name = format!(
            "e2e/composite_cpu_validate_meta_holdout_ml/len{len}/train{train_assets}/holdout{holdout_assets}"
        );
        let mut group = c.benchmark_group(format!(
            "e2e/composite_cpu_validate_meta_holdout_ml/len{len}/train{train_assets}/holdout{holdout_assets}"
        ));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(10);
        print_bench_metadata(
            &group_name,
            &[
                ("candles_len", len.to_string()),
                ("train_assets", train_assets.to_string()),
                ("holdout_assets", holdout_assets.to_string()),
                ("surface", "optimize_validate_meta_holdout_ml".to_string()),
                ("stage_diagnostics", stage_diagnostics.to_string()),
            ],
        );

        group.bench_function("workflow", |b| {
            b.iter(|| {
                run_full_workflow_once(len, train_assets, holdout_assets, &cancel)
                    .expect("e2e workflow");
            })
        });

        group.finish();
    }

    let history_input =
        build_ml_history_bench_input(history_metas, groups_per_meta).expect("ml history inputs");
    let history_group_name = format!(
        "e2e/ml_stage5_history_prefix/metas{history_metas}/groups{groups_per_meta}/prefix{}",
        history_input.prefix_len
    );
    let mut history_group = c.benchmark_group(history_group_name.clone());
    history_group.measurement_time(Duration::from_secs(6));
    history_group.sample_size(10);
    print_bench_metadata(
        &history_group_name,
        &[
            ("history_metas", history_metas.to_string()),
            ("groups_per_meta", groups_per_meta.to_string()),
            ("prefix_len", history_input.prefix_len.to_string()),
            ("prefix_mode", "train_prefix_apply_current".to_string()),
            ("surface", "stage5_history_prefix".to_string()),
        ],
    );
    print_bench_metadata(
        &format!("{history_group_name}/raw_history_prefix"),
        &[
            ("history_metas", history_metas.to_string()),
            ("groups_per_meta", groups_per_meta.to_string()),
            ("prefix_len", history_input.prefix_len.to_string()),
            ("prepared_rows", "false".to_string()),
        ],
    );
    print_bench_metadata(
        &format!("{history_group_name}/prepared_history_prefix"),
        &[
            ("history_metas", history_metas.to_string()),
            ("groups_per_meta", groups_per_meta.to_string()),
            ("prefix_len", history_input.prefix_len.to_string()),
            ("prepared_rows", "true".to_string()),
        ],
    );

    history_group.bench_function("raw_history_prefix", |b| {
        b.iter(|| run_ml_history_prefix_once(&history_input, false).expect("raw history prefix"))
    });
    history_group.bench_function("prepared_history_prefix", |b| {
        b.iter(|| {
            run_ml_history_prefix_once(&history_input, true).expect("prepared history prefix")
        })
    });
    history_group.finish();
}

criterion_group!(benches, bench_e2e_workflow_with_ml);
criterion_main!(benches);
