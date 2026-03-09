use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    Backend, BackendUsed, CompositeCpuRequest, CompositeSignalMode, RocbStrategyParams,
};
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

        let open_px = px * (1.0 - 0.0007);
        let close_px = px * (1.0 + 0.0007);
        open.push(open_px);
        high.push(open_px.max(close_px) * 1.0020);
        low.push(open_px.min(close_px) * 0.9980);
        close.push(close_px);
        vol.push(1_000.0 + (i % 29) as f64 * 37.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_rocb_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: Some(RocbStrategyParams::default()),
        data_id: "bench".to_string(),
        fast_range,
        slow_range,
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["ema".to_string()],
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        strategy: StrategyConfig::default(),
        halftrend: HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        },
        macz: MaczGateParams::default(),
        ott: OttGateParams::default(),
        signal_mode: CompositeSignalMode::HardGate,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        gpu_prescan: None,
        top_k: Some(20),
        include_all: Some(false),
        heatmap_bins: Some(64),
    }
}

fn bench_rocb_cpu(c: &mut Criterion) {
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 20, 1), (20, 80, 1)),
        ("medium", (5, 30, 1), (30, 120, 1)),
    ];

    let mut group = c.benchmark_group("rocb/cpu/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_rocb_req(Backend::CpuOnly, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let req2 = req.clone();
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("rocb cpu run");
            })
        });
    }

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_rocb_gpu_kernel(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);
    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small/975pairs", (5, 20, 1), (20, 80, 1)),
        ("medium/2365pairs", (5, 30, 1), (30, 120, 1)),
        ("large/13385pairs", (5, 80, 1), (20, 220, 1)),
        ("xlarge/29765pairs", (5, 120, 1), (20, 320, 1)),
    ];

    let mut group = c.benchmark_group("rocb/gpu_kernel/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_rocb_req(Backend::GpuKernel { device_id: 0 }, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let req2 = req.clone();
                let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("rocb gpu-kernel run");
                assert!(
                    matches!(res.backend_used, BackendUsed::Gpu),
                    "expected GPU backend for ROCB bench, got backend={:?}, route={:?}, fallback={:?}",
                    res.backend_used,
                    res.backend_route,
                    res.backend_fallback_reason
                );
            })
        });
    }

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_rocb_gpu_kernel(_c: &mut Criterion) {}

criterion_group!(rocb_benches, bench_rocb_cpu, bench_rocb_gpu_kernel);
criterion_main!(rocb_benches);
