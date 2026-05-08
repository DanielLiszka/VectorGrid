#[macro_use]
mod bench_support;

use criterion::{criterion_group, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::{Backend, CompositeCpuRequest};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

fn make_synth_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);
        let drift = 0.00005;
        let noise = (i as f64 * 0.017).sin() * 0.001;
        px *= 1.0 + drift + noise;

        open.push(px);
        high.push(px * 1.001);
        low.push(px * 0.999);
        close.push(px);
        vol.push(1_000.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
}

fn print_route_probe(
    group_name: &str,
    req: &CompositeCpuRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) {
    let route_probe = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
        req.clone(),
        candles,
        cancel,
        None,
    )
    .expect("composite cpu route probe");

    println!(
        "benchmark route {group_name}: backend_used={:?} backend_route={} fallback={}",
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );
}

fn make_plain_req() -> CompositeCpuRequest {
    CompositeCpuRequest {
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
        data_id: "bench".to_string(),
        fast_range: (5, 80, 1),
        slow_range: (90, 250, 1),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        strategy: StrategyConfig::default(),
        halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        },
        macz: ta_app_core::composite_cpu::MaczGateParams::default(),
        ott: ta_app_core::composite_cpu::OttGateParams::default(),
        signal_mode: ta_app_core::CompositeSignalMode::HardGate,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        gpu_prescan: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(0),
        include_trade_metrics: false,
        metric_contract: None,
    }
}

fn bench_composite_cpu_plain_product_proxy(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let req = make_plain_req();

    let group_name = "composite_cpu/plain_product_proxy/50k";
    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    print_bench_metadata(
        group_name,
        &[
            ("backend", "CpuOnly".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("objective", "Sharpe".to_string()),
            ("heatmap_bins", "0".to_string()),
            ("include_all", "false".to_string()),
            ("gpu_prescan", "none".to_string()),
        ],
    );
    print_route_probe(group_name, &req, &candles, &cancel);

    group.bench_function(
        "topk50_include_all_false_no_heatmap/fast5-80_slow90-250",
        |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("plain CompositeCpu product proxy run");
            })
        },
    );

    group.finish();
}

fn bench_composite_cpu_plain_include_all(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let mut req = make_plain_req();
    req.include_all = Some(true);

    let group_name = "composite_cpu/plain_include_all/50k";
    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    print_bench_metadata(
        group_name,
        &[
            ("backend", "CpuOnly".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("objective", "Sharpe".to_string()),
            ("heatmap_bins", "0".to_string()),
            ("include_all", "true".to_string()),
            ("gpu_prescan", "none".to_string()),
        ],
    );
    print_route_probe(group_name, &req, &candles, &cancel);

    group.bench_function(
        "topk50_include_all_true_no_heatmap/fast5-80_slow90-250",
        |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("plain CompositeCpu include_all proxy run");
            })
        },
    );

    group.finish();
}

fn bench_composite_cpu_engine_heatmap(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let mut req = make_plain_req();
    req.heatmap_bins = Some(64);

    let group_name = "composite_cpu/engine_heatmap/50k";
    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    print_bench_metadata(
        group_name,
        &[
            ("backend", "CpuOnly".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("objective", "Sharpe".to_string()),
            ("heatmap_bins", "64".to_string()),
            ("include_all", "false".to_string()),
            ("gpu_prescan", "none".to_string()),
        ],
    );
    print_route_probe(group_name, &req, &candles, &cancel);

    group.bench_function("topk50_heatmap64/fast5-80_slow90-250", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("plain CompositeCpu engine heatmap run");
        })
    });

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_composite_cpu_engine_stage0_prescan_double_ma(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let mut req = make_plain_req();
    req.fast_range = (5, 400, 1);
    req.slow_range = (10, 800, 1);
    req.gpu_prescan = Some(ta_app_core::CompositeGpuPrescanConfig {
        top_k: 128,
        window: 8,
        predictive_top_k: 12,
        predictive_stride: 8,
        device_id: Some(0),
        require_gpu: true,
        objectives: ta_app_core::CompositeGpuPrescanObjectives::Both,
        ma_pairs: None,
        mode: OptimizationMode::Grid,
    });

    let group_name = "composite_cpu/engine_stage0_prescan_double_ma/50k";
    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    print_bench_metadata(
        group_name,
        &[
            ("backend", "CpuOnly".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("objective", "Sharpe".to_string()),
            ("heatmap_bins", "0".to_string()),
            ("include_all", "false".to_string()),
            ("gpu_prescan", "double_ma_stage0".to_string()),
        ],
    );
    print_route_probe(group_name, &req, &candles, &cancel);

    group.bench_function("topk50_prescan128_window8/fast5-400_slow10-800", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("plain CompositeCpu stage-0 prescan run");
        })
    });

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_composite_cpu_engine_stage0_prescan_double_ma(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_composite_cpu_plain_product_proxy,
    bench_composite_cpu_plain_include_all,
    bench_composite_cpu_engine_heatmap,
    bench_composite_cpu_engine_stage0_prescan_double_ma
);
serialized_criterion_main!(benches);
