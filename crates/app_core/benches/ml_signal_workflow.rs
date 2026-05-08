#[macro_use]
mod bench_support;

use criterion::{criterion_group, BatchSize, Criterion};
use csv::WriterBuilder;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};
use ta_app_core::ml_signal::{
    MlSignalInferenceBackendPreference, MlSignalNativeLibraryDiscoveryPolicy,
};
use ta_app_core::{
    build_ml_signal_score_cache_from_csv, run_ml_signal_policy_sweep_blocking_with_candles,
    run_ml_signal_validation_blocking_with_candles, IterativeSimConfig, MlSignalPolicyParams,
    MlSignalPolicySweepRequest, MlSignalScoreCacheBuildRequest, MlSignalScoreCacheContractMode,
    MlSignalSweepAxisF64, MlSignalSweepAxisUsize, MlSignalValidationRequest,
};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
}

fn sweep_axis_len_f64(axis: &MlSignalSweepAxisF64) -> usize {
    if !axis.start.is_finite()
        || !axis.end.is_finite()
        || !axis.step.is_finite()
        || axis.step <= 0.0
        || axis.end < axis.start
    {
        return 0;
    }
    let mut count = 0usize;
    let mut value = axis.start;
    while value <= axis.end + (axis.step * 1.0e-9) {
        count = count.saturating_add(1);
        value += axis.step;
        if count > 4096 {
            break;
        }
    }
    count
}

fn sweep_axis_len_usize(axis: &MlSignalSweepAxisUsize) -> usize {
    if axis.step == 0 || axis.end < axis.start {
        return 0;
    }
    ((axis.end - axis.start) / axis.step).saturating_add(1)
}

fn temp_file_path(name: &str, ext: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("ta_ml_signal_bench_{name}_{nonce}.{ext}"))
        .to_string_lossy()
        .to_string()
}

fn sample_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let base = 100.0 + (i as f64 * 0.25) + ((i % 9) as f64 * 0.08);
        let o = base;
        let c = base + if i % 11 < 6 { 0.40 } else { -0.18 };
        ts.push(1_700_000_000_000_i64 + (i as i64 * 86_400_000));
        open.push(o);
        high.push(o.max(c) + 0.30);
        low.push(o.min(c) - 0.25);
        close.push(c);
        volume.push(1_000.0 + (i % 13) as f64 * 20.0);
    }
    Candles::new_with_fields(
        ts,
        open,
        high,
        low,
        close,
        volume,
        CandleFieldFlags {
            open: true,
            high: true,
            low: true,
            close: true,
            volume: true,
        },
    )
}

fn write_probability_scores_csv(path: &str, candles: &Candles) -> usize {
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_path(path)
        .expect("create score csv");
    writer
        .write_record([
            "timestamp",
            "p_long_cal",
            "p_short_cal",
            "raw_long",
            "raw_short",
        ])
        .expect("header");
    let mut rows = 0usize;
    for (idx, ts) in candles.timestamp.iter().enumerate().skip(5).take(180) {
        let (p_long, p_short) = if idx % 7 < 4 {
            (0.74, 0.21)
        } else {
            (0.24, 0.73)
        };
        writer
            .write_record([
                ts.to_string(),
                p_long.to_string(),
                p_short.to_string(),
                ((p_long - 0.5) * 4.0).to_string(),
                ((p_short - 0.5) * 4.0).to_string(),
            ])
            .expect("row");
        rows = rows.saturating_add(1);
    }
    writer.flush().expect("flush score csv");
    rows
}

fn bench_ml_signal_workflow(c: &mut Criterion) {
    let cancel = AtomicBool::new(false);
    let candles = sample_candles(256);
    let input_path = temp_file_path("scores_input", "csv");
    let score_rows_written = write_probability_scores_csv(&input_path, &candles);
    let score_input_bytes_estimate = score_rows_written * (8 + 4 * 8);
    let score_output_bytes_estimate = score_rows_written * (8 + 2 * 8);
    let policy_sweep_size = sweep_axis_len_f64(&MlSignalSweepAxisF64 {
        start: 0.55,
        end: 0.65,
        step: 0.05,
    }) * sweep_axis_len_f64(&MlSignalSweepAxisF64 {
        start: 0.55,
        end: 0.65,
        step: 0.05,
    }) * sweep_axis_len_f64(&MlSignalSweepAxisF64 {
        start: 0.05,
        end: 0.10,
        step: 0.05,
    }) * sweep_axis_len_usize(&MlSignalSweepAxisUsize {
        start: 8,
        end: 10,
        step: 2,
    }) * 2
        * 1;

    print_bench_metadata(
        "ml_signal_workflow",
        &[
            ("strategy", "MlSignal".to_string()),
            ("workload_shape", "score_cache_policy_scan".to_string()),
            ("candles", candles.close.len().to_string()),
            ("score_rows_written", score_rows_written.to_string()),
            ("score_source", "csv_score_cache".to_string()),
            ("native_backend_preference", "score_cache_only".to_string()),
            (
                "input_bytes_estimate",
                score_input_bytes_estimate.to_string(),
            ),
            (
                "output_bytes_estimate",
                score_output_bytes_estimate.to_string(),
            ),
            (
                "arithmetic_intensity_estimate",
                "low_scalar_scan".to_string(),
            ),
            ("gemm_like", "false".to_string()),
            ("dense_matrix_candidate", "false".to_string()),
            ("tensor_core_candidate", "false".to_string()),
            ("tensor_core_disposition", "kill".to_string()),
            (
                "reason",
                "score_cache_validation_and_policy_sweep_not_gemm".to_string(),
            ),
        ],
    );

    print_bench_metadata(
        "ml_signal_score_cache_build",
        &[
            ("strategy", "MlSignal".to_string()),
            ("workload_shape", "score_cache_scan".to_string()),
            ("candles", candles.close.len().to_string()),
            ("score_rows_written", score_rows_written.to_string()),
            ("score_backend", "score_cache_only".to_string()),
            ("native_inference_matrix", "absent".to_string()),
            ("cpu_only_safe", "true".to_string()),
            ("gemm_like", "false".to_string()),
            ("tensor_core_candidate", "false".to_string()),
            (
                "reason",
                "csv_probability_rows_to_score_cache_not_gemm".to_string(),
            ),
        ],
    );

    c.bench_function("ml_signal_score_cache_build", |b| {
        b.iter_batched(
            || temp_file_path("score_cache", "json"),
            |cache_path| {
                let build = build_ml_signal_score_cache_from_csv(
                    MlSignalScoreCacheBuildRequest {
                        data_id: "bench-data".to_string(),
                        input_path: input_path.clone(),
                        output_path: cache_path.clone(),
                        model_bundle_id: Some("bench-bundle".to_string()),
                        feature_schema_path: None,
                        feature_schema_hash: None,
                        export_manifest_path: None,
                        source_bundle_manifest_path: None,
                        calibration_state: None,
                        threshold_policy: None,
                        producer_kind: None,
                        score_contract_version: None,
                        score_scope: None,
                        allow_non_walkforward_score_scope: false,
                        native_backend_preference:
                            MlSignalInferenceBackendPreference::ScoreCacheOnly,
                        native_library_path: None,
                        native_gpu_device_id: None,
                        native_prediction_batch_size: None,
                        native_library_discovery_policy:
                            MlSignalNativeLibraryDiscoveryPolicy::Production,
                    },
                    &candles,
                    &cancel,
                )
                .expect("build score cache");
                let _ = fs::remove_file(&cache_path);
                let _ = fs::remove_file(build.manifest_path);
            },
            BatchSize::SmallInput,
        );
    });

    let cache_path = temp_file_path("validation_cache", "json");
    let build = build_ml_signal_score_cache_from_csv(
        MlSignalScoreCacheBuildRequest {
            data_id: "bench-data".to_string(),
            input_path: input_path.clone(),
            output_path: cache_path.clone(),
            model_bundle_id: Some("bench-bundle".to_string()),
            feature_schema_path: None,
            feature_schema_hash: None,
            export_manifest_path: None,
            source_bundle_manifest_path: None,
            calibration_state: None,
            threshold_policy: None,
            producer_kind: None,
            score_contract_version: None,
            score_scope: None,
            allow_non_walkforward_score_scope: false,
            native_backend_preference: MlSignalInferenceBackendPreference::ScoreCacheOnly,
            native_library_path: None,
            native_gpu_device_id: None,
            native_prediction_batch_size: None,
            native_library_discovery_policy: MlSignalNativeLibraryDiscoveryPolicy::Production,
        },
        &candles,
        &cancel,
    )
    .expect("prebuild score cache");

    print_bench_metadata(
        "ml_signal_validation",
        &[
            ("strategy", "MlSignal".to_string()),
            ("workload_shape", "score_cache_policy_scan".to_string()),
            ("candles", candles.close.len().to_string()),
            ("score_rows_written", score_rows_written.to_string()),
            ("validation_candidates", "1".to_string()),
            ("score_backend", "score_cache_only".to_string()),
            ("cpu_only_safe", "true".to_string()),
            ("gemm_like", "false".to_string()),
            ("tensor_core_candidate", "false".to_string()),
            (
                "reason",
                "probability_thresholding_and_trade_overlay_not_gemm".to_string(),
            ),
        ],
    );

    c.bench_function("ml_signal_validation", |b| {
        b.iter(|| {
            let mut strategy = StrategyConfig::default();
            strategy.long_only = false;
            let _ = run_ml_signal_validation_blocking_with_candles(
                MlSignalValidationRequest {
                    data_id: "bench-data".to_string(),
                    score_cache_path: cache_path.clone(),
                    score_cache_contract_mode: MlSignalScoreCacheContractMode::Research,
                    candidates: vec![MlSignalPolicyParams {
                        t_long: 0.60,
                        t_short: 0.60,
                        delta: 0.08,
                        max_hold_bars: 10,
                        exit_on_flat: true,
                        exit_on_opposite: true,
                        label: Some("bench_validation".to_string()),
                        strategy,
                    }],
                    iterative: IterativeSimConfig::default(),
                    min_psr: None,
                    min_dsr: None,
                    validation_context: None,
                },
                &candles,
                &cancel,
            )
            .expect("validation");
        });
    });

    print_bench_metadata(
        "ml_signal_policy_sweep",
        &[
            ("strategy", "MlSignal".to_string()),
            ("workload_shape", "score_cache_policy_scan".to_string()),
            ("candles", candles.close.len().to_string()),
            ("score_rows_written", score_rows_written.to_string()),
            ("sweep_size", policy_sweep_size.to_string()),
            ("top_k", "5".to_string()),
            ("score_backend", "score_cache_only".to_string()),
            ("cpu_only_safe", "true".to_string()),
            ("gemm_like", "false".to_string()),
            ("tensor_core_candidate", "false".to_string()),
            ("reason", "threshold_grid_scan_not_gemm".to_string()),
        ],
    );

    c.bench_function("ml_signal_policy_sweep", |b| {
        b.iter(|| {
            let mut strategy = StrategyConfig::default();
            strategy.long_only = false;
            let _ = run_ml_signal_policy_sweep_blocking_with_candles(
                MlSignalPolicySweepRequest {
                    data_id: "bench-data".to_string(),
                    score_cache_path: cache_path.clone(),
                    score_cache_contract_mode: MlSignalScoreCacheContractMode::Research,
                    t_long: Some(MlSignalSweepAxisF64 {
                        start: 0.55,
                        end: 0.65,
                        step: 0.05,
                    }),
                    t_short: Some(MlSignalSweepAxisF64 {
                        start: 0.55,
                        end: 0.65,
                        step: 0.05,
                    }),
                    delta: Some(MlSignalSweepAxisF64 {
                        start: 0.05,
                        end: 0.10,
                        step: 0.05,
                    }),
                    max_hold_bars: Some(MlSignalSweepAxisUsize {
                        start: 8,
                        end: 10,
                        step: 2,
                    }),
                    exit_on_flat_values: Some(vec![false, true]),
                    exit_on_opposite_values: Some(vec![true]),
                    base_strategy: strategy,
                    iterative: IterativeSimConfig::default(),
                    min_psr: None,
                    min_dsr: None,
                    validation_context: None,
                    top_k: 5,
                },
                &candles,
                &cancel,
            )
            .expect("sweep");
        });
    });

    let _ = fs::remove_file(&input_path);
    let _ = fs::remove_file(&cache_path);
    let _ = fs::remove_file(build.manifest_path);
}

criterion_group!(benches, bench_ml_signal_workflow);
serialized_criterion_main!(benches);
