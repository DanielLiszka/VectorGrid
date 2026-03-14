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
}

criterion_group!(benches, bench_meta_select);
criterion_main!(benches);
