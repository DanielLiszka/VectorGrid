use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::{json, Value};
use std::time::Duration;

use ta_app_core::{
    meta_select_from_artifacts, MetaClusterConfig, MetaGroupingMode, MetaHoldoutMode,
    MetaScoreWeights, MetaSelectionRequest,
};

fn make_artifacts(n_artifacts: usize, n_groups: usize, cands_per_artifact: usize) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(n_artifacts);

    for i in 0..n_artifacts {
        let label = format!("SYM{i:04}");
        let timeframe = "1m";
        let csv_path = format!("{label}.csv");

        let mut cands: Vec<Value> = Vec::with_capacity(cands_per_artifact);
        for j in 0..cands_per_artifact {
            let gid = (i + j) % n_groups;
            let fast_len = 10 + (gid as i64 % 90) as i64;
            let slow_len = fast_len + 60;
            let robust_score = 1.0 + (gid as f64) * 0.01;
            let max_dd = 0.10 + (gid as f64) * 0.001;

            cands.push(json!({
                "params": {
                    "fast_ma_type": "sma",
                    "slow_ma_type": "sma",
                    "fast_len": fast_len,
                    "slow_len": slow_len,
                },
                "robust_score": robust_score,
                "robust_pareto": true,
                "in_sample": { "max_dd": max_dd },
            }));
        }

        out.push(json!({
            "kind": "vectorgrid_run_artifact",
            "tags": ["bench"],
            "dataset_meta": { "label": label, "timeframe": timeframe, "csv_path": csv_path },
            "run": { "req": { "strategy_kind": "composite", "ma_source": "close" } },
            "validation": {
                "req": { "ma_source": "close" },
                "res": { "candidates": cands }
            }
        }));
    }

    out
}

fn make_context_grouped_artifacts(
    n_artifacts: usize,
    n_groups: usize,
    cands_per_artifact: usize,
) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(n_artifacts);

    for i in 0..n_artifacts {
        let label = format!("CTX{i:04}");
        let timeframe = "5m";
        let csv_path = format!("{label}.csv");
        let variant = format!("cluster_{}", i % 4);

        let mut cands: Vec<Value> = Vec::with_capacity(cands_per_artifact);
        for j in 0..cands_per_artifact {
            let gid = (i + j) % n_groups;
            let fast_len = 12 + ((gid % 6) as i64) * 5;
            let slow_len = fast_len + 45;
            let robust_score = 1.20 + (gid as f64) * 0.015 + ((i % 5) as f64) * 0.002;
            let max_dd = 0.08 + (gid as f64) * 0.004;
            let amplitude = 2 + (gid % 3) as i64;
            let atr_period = 40 + ((gid % 5) as i64) * 4;
            let channel_deviation = 1.55 + ((gid % 4) as f64) * 0.08;

            let params = if (i + j) % 2 == 0 {
                json!({
                    "fast_ma_type": "ema",
                    "slow_ma_type": "ema",
                    "fast_len": fast_len,
                    "slow_len": slow_len,
                    "halftrend": {
                        "amplitude": amplitude,
                        "atr_period": atr_period,
                        "channel_deviation": channel_deviation
                    },
                    "macz": { "fast": 12, "slow": 26, "signal": 9 },
                    "strategy": { "confirm_trend": gid % 2 == 0, "weight": 1.0 + (gid % 3) as f64 * 0.1 }
                })
            } else {
                json!({
                    "strategy": { "weight": 1.0 + (gid % 3) as f64 * 0.1, "confirm_trend": gid % 2 == 0 },
                    "macz": { "signal": 9, "slow": 26, "fast": 12 },
                    "halftrend": {
                        "channel_deviation": channel_deviation,
                        "atr_period": atr_period,
                        "amplitude": amplitude
                    },
                    "slow_len": slow_len,
                    "fast_len": fast_len,
                    "slow_ma_type": "ema",
                    "fast_ma_type": "ema"
                })
            };

            cands.push(json!({
                "params": params,
                "robust_score": robust_score,
                "robust_pareto": true,
                "in_sample": {
                    "max_dd": max_dd,
                    "pnl": robust_score * 10.0,
                    "sharpe": 1.0 + robust_score * 0.2,
                    "trades": 24,
                    "exposure": 0.55
                }
            }));
        }

        out.push(json!({
            "kind": "vectorgrid_run_artifact",
            "tags": ["bench", "context"],
            "dataset_meta": { "label": label, "timeframe": timeframe, "csv_path": csv_path },
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo",
                    "feature_context": {
                        "family": "generic",
                        "preset": format!("preset_{}", i % 3),
                        "resolution": timeframe
                    },
                    "run_context": {
                        "variant": variant,
                        "engine": "meta_bench"
                    }
                }
            },
            "validation": {
                "req": {
                    "feature_context": {
                        "family": "generic",
                        "preset": format!("preset_{}", i % 3),
                        "resolution": timeframe
                    },
                    "run_context": {
                        "variant": variant,
                        "engine": "meta_bench"
                    }
                },
                "res": { "candidates": cands }
            }
        }));
    }

    out
}

fn bench_meta_select(c: &mut Criterion) {
    let artifacts = make_artifacts(1_000, 50, 4);

    let mut group = c.benchmark_group("meta_select/workflow/1000_artifacts/4cands");
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);

    let exact = MetaSelectionRequest {
        strategy_kind: "composite".to_string(),
        timeframe: Some("1m".to_string()),
        required_tag: Some("bench".to_string()),
        holdout_tag: None,
        eval_holdout: true,
        save_holdout_rows: false,
        holdout_eval_top_n: None,
        holdout_eval_explore_n: 0,
        holdout_eval_rank_by: None,
        holdout_mode: MetaHoldoutMode::Light,
        holdout_validation: None,
        top_k: 25,
        min_samples: 30,
        strict_shortlist: true,
        allow_shortlist_fallback: true,
        weights: MetaScoreWeights::default(),
        grouping_mode: MetaGroupingMode::Exact,
        cluster_config: MetaClusterConfig::default(),
        cluster_representatives: 2,
    };

    group.bench_function("exact", |b| {
        b.iter(|| {
            let _ = meta_select_from_artifacts(&exact, &artifacts, None);
        })
    });

    let clustered = MetaSelectionRequest {
        grouping_mode: MetaGroupingMode::Clustered,
        ..exact
    };

    group.bench_function("clustered", |b| {
        b.iter(|| {
            let _ = meta_select_from_artifacts(&clustered, &artifacts, None);
        })
    });

    group.finish();

    let context_artifacts = make_context_grouped_artifacts(1_200, 24, 8);
    let mut grouped =
        c.benchmark_group("meta_select/workflow/context_grouped/1200_artifacts/8cands");
    grouped.measurement_time(Duration::from_secs(6));
    grouped.sample_size(10);

    let context_exact = MetaSelectionRequest {
        strategy_kind: "CompositeMo".to_string(),
        timeframe: Some("5m".to_string()),
        required_tag: Some("context".to_string()),
        holdout_tag: None,
        eval_holdout: true,
        save_holdout_rows: false,
        holdout_eval_top_n: None,
        holdout_eval_explore_n: 0,
        holdout_eval_rank_by: None,
        holdout_mode: MetaHoldoutMode::Light,
        holdout_validation: None,
        top_k: 25,
        min_samples: 40,
        strict_shortlist: true,
        allow_shortlist_fallback: true,
        weights: MetaScoreWeights::default(),
        grouping_mode: MetaGroupingMode::Exact,
        cluster_config: MetaClusterConfig {
            int_bucket: 5,
            float_bucket: 0.25,
        },
        cluster_representatives: 3,
    };

    grouped.bench_function("context_exact", |b| {
        b.iter(|| {
            let _ = meta_select_from_artifacts(&context_exact, &context_artifacts, None);
        })
    });

    let context_clustered = MetaSelectionRequest {
        grouping_mode: MetaGroupingMode::Clustered,
        ..context_exact
    };

    grouped.bench_function("context_clustered", |b| {
        b.iter(|| {
            let _ = meta_select_from_artifacts(&context_clustered, &context_artifacts, None);
        })
    });

    grouped.finish();
}

criterion_group!(benches, bench_meta_select);
criterion_main!(benches);
