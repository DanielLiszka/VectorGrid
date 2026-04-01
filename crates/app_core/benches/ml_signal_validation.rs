use criterion::{black_box, criterion_group, criterion_main, Criterion};
use csv::WriterBuilder;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};
use ta_app_core::{
    build_ml_signal_score_cache_from_csv, run_ml_signal_policy_sweep_blocking_with_candles,
    run_ml_signal_validation_blocking_with_candles, IterativeSimConfig, MlSignalPolicyParams,
    MlSignalPolicySweepRequest, MlSignalScoreCacheBuildRequest, MlSignalSweepAxisF64,
    MlSignalSweepAxisUsize, MlSignalValidationRequest,
};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
        "strategy=MlSignal".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
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
        let base = 100.0 + (i as f64 * 0.18) + ((i % 9) as f64 * 0.09);
        let o = base;
        let c = base + if i % 11 < 6 { 0.42 } else { -0.24 };
        ts.push(1_700_000_000_000_i64 + (i as i64 * 3_600_000));
        open.push(o);
        high.push(o.max(c) + 0.30);
        low.push(o.min(c) - 0.28);
        close.push(c);
        volume.push(1_000.0 + (i % 13) as f64 * 17.0);
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

struct MlSignalBenchFixture {
    candles: Candles,
    score_cache_path: String,
    cancel: AtomicBool,
}

impl MlSignalBenchFixture {
    fn new() -> Self {
        let candles = sample_candles(512);
        let input_path = temp_file_path("scores_in", "csv");
        let cache_path = temp_file_path("scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long_cal", "p_short_cal", "score_scope"])
            .expect("header");
        for (idx, ts) in candles.timestamp.iter().enumerate().skip(8).take(420) {
            let (p_long, p_short) = if idx % 12 < 6 {
                (0.76, 0.18)
            } else {
                (0.22, 0.79)
            };
            writer
                .write_record([
                    ts.to_string(),
                    p_long.to_string(),
                    p_short.to_string(),
                    "walkforward_test".to_string(),
                ])
                .expect("row");
        }
        writer.flush().expect("flush score csv");

        build_ml_signal_score_cache_from_csv(
            MlSignalScoreCacheBuildRequest {
                data_id: "bench-data".to_string(),
                input_path: input_path.clone(),
                output_path: cache_path.clone(),
                model_bundle_id: Some("bench-bundle".to_string()),
            },
            &candles,
            &cancel,
        )
        .expect("build score cache");

        let _ = std::fs::remove_file(&input_path);

        Self {
            candles,
            score_cache_path: cache_path,
            cancel,
        }
    }
}

fn bench_ml_signal_validation(c: &mut Criterion) {
    let fixture = MlSignalBenchFixture::new();
    let mut strategy = StrategyConfig::default();
    strategy.long_only = false;
    strategy.allow_flip = false;
    strategy.cooldown_bars = 2;
    strategy.signal_persistence_bars = 2;

    print_bench_metadata(
        "ml_signal_validation",
        &[("candles", fixture.candles.close.len().to_string())],
    );

    let validation_req = MlSignalValidationRequest {
        data_id: "bench-data".to_string(),
        score_cache_path: fixture.score_cache_path.clone(),
        candidates: vec![MlSignalPolicyParams {
            t_long: 0.60,
            t_short: 0.60,
            delta: 0.08,
            max_hold_bars: 12,
            exit_on_flat: true,
            exit_on_opposite: true,
            label: Some("bench".to_string()),
            strategy: strategy.clone(),
        }],
        iterative: IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        validation_context: None,
    };

    c.bench_function("ml_signal_validation_single_candidate", |b| {
        b.iter(|| {
            run_ml_signal_validation_blocking_with_candles(
                black_box(validation_req.clone()),
                &fixture.candles,
                &fixture.cancel,
            )
            .expect("ml signal validation bench")
        })
    });

    let sweep_req = MlSignalPolicySweepRequest {
        data_id: "bench-data".to_string(),
        score_cache_path: fixture.score_cache_path.clone(),
        t_long: Some(MlSignalSweepAxisF64 {
            start: 0.58,
            end: 0.62,
            step: 0.02,
        }),
        t_short: Some(MlSignalSweepAxisF64 {
            start: 0.58,
            end: 0.62,
            step: 0.02,
        }),
        delta: Some(MlSignalSweepAxisF64 {
            start: 0.06,
            end: 0.10,
            step: 0.02,
        }),
        max_hold_bars: Some(MlSignalSweepAxisUsize {
            start: 8,
            end: 12,
            step: 2,
        }),
        exit_on_flat_values: Some(vec![false, true]),
        exit_on_opposite_values: Some(vec![true]),
        base_strategy: strategy,
        iterative: IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        validation_context: None,
        top_k: 10,
    };

    c.bench_function("ml_signal_policy_sweep_small", |b| {
        b.iter(|| {
            run_ml_signal_policy_sweep_blocking_with_candles(
                black_box(sweep_req.clone()),
                &fixture.candles,
                &fixture.cancel,
            )
            .expect("ml signal policy sweep bench")
        })
    });
}

criterion_group!(ml_signal_validation_benches, bench_ml_signal_validation);
criterion_main!(ml_signal_validation_benches);
