use criterion::{criterion_group, criterion_main, Criterion};
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

fn bench_composite_cpu_grid(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);

    let mut group = c.benchmark_group("composite_cpu/grid/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let req = CompositeCpuRequest {
        backend: Backend::CpuOnly,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
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
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(64),
    };

    group.bench_function("no_prescan/fast5-80_slow90-250", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("composite cpu run");
        })
    });

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_composite_cpu_prescan_gpu_kernel(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);

    let mut group = c.benchmark_group("composite_cpu/prescan_gpu_kernel/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let req = CompositeCpuRequest {
        backend: Backend::CpuOnly,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        data_id: "bench".to_string(),
        fast_range: (5, 400, 1),
        slow_range: (10, 800, 1),
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
        gpu_prescan: Some(ta_app_core::CompositeGpuPrescanConfig {
            top_k: 128,
            window: 8,
            predictive_top_k: 12,
            predictive_stride: 8,
            device_id: Some(0),
            require_gpu: true,
            objectives: ta_app_core::CompositeGpuPrescanObjectives::Both,
            ma_pairs: None,
            mode: OptimizationMode::Grid,
        }),
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(0),
    };

    group.bench_function("prescan/fast5-400_slow10-800", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("composite cpu prescan run");
        })
    });

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_composite_cpu_prescan_gpu_kernel(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_composite_cpu_grid,
    bench_composite_cpu_prescan_gpu_kernel
);
criterion_main!(benches);
