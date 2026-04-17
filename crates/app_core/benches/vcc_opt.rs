use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    Backend, BackendUsed, CompositeCpuRequest, CompositeSignalMode, VccStrategyParams,
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

        open.push(px);
        high.push(px * 1.0015);
        low.push(px * 0.9985);
        close.push(px);
        vol.push(1_000.0 + (i % 17) as f64 * 40.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_vcc_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend,
        asr: None,
        vcc: Some(VccStrategyParams::default()),
        vemr: None,
        rocb: None,
        vcet: None,
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
        data_id: "bench".to_string(),
        fast_range,
        slow_range,
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["alma".to_string()],
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
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        top_k: Some(20),
        include_all: Some(false),
        heatmap_bins: Some(64),
        include_trade_metrics: false,
        metric_contract: None,
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_vcc_gpu_kernel(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);
    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small/975pairs", (5, 20, 1), (20, 80, 1)),
        ("medium/2365pairs", (5, 30, 1), (30, 120, 1)),
        ("large/13385pairs", (5, 80, 1), (20, 220, 1)),
    ];

    let mut group = c.benchmark_group("vcc/gpu_kernel/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_vcc_req(Backend::GpuKernel { device_id: 0 }, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let req2 = req.clone();
                let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("vcc gpu-kernel run");
                assert!(
                    matches!(res.backend_used, BackendUsed::Gpu),
                    "expected GPU backend for VCC bench, got backend={:?}, route={:?}, fallback={:?}",
                    res.backend_used,
                    res.backend_route,
                    res.backend_fallback_reason
                );
                assert!(
                    res.backend_fallback_reason.is_none(),
                    "expected no fallback reason for VCC bench, got {:?}",
                    res.backend_fallback_reason
                );
                assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
            })
        });
    }

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_vcc_gpu_kernel(_c: &mut Criterion) {}

criterion_group!(vcc_benches, bench_vcc_gpu_kernel);
criterion_main!(vcc_benches);
