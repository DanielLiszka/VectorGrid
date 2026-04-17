use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::vcet_gpu_kernel::PreparedVcetGpuKernelBatch;
use ta_app_core::{
    Backend, BackendUsed, CompositeCpuRequest, CompositeSignalMode, VcetStrategyParams,
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

        let open_px = px * (1.0 - 0.0006);
        let close_px = px * (1.0 + 0.0006);
        open.push(open_px);
        high.push(open_px.max(close_px) * 1.0018);
        low.push(open_px.min(close_px) * 0.9982);
        close.push(close_px);
        vol.push(1_000.0 + (i % 19) as f64 * 34.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_vcet_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        vcet: Some(VcetStrategyParams::default()),
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
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

fn expand_period_range(range: (u32, u32, u32)) -> Vec<u16> {
    let (start, end, step) = range;
    assert!(step > 0, "benchmark range step must be > 0");
    assert!(start <= end, "benchmark range start must be <= end");

    let mut out = Vec::new();
    let mut current = start;
    loop {
        out.push(u16::try_from(current).expect("benchmark period exceeds u16"));
        match current.checked_add(step) {
            Some(next) if next <= end => current = next,
            _ => break,
        }
    }
    out
}

fn bench_vcet_cpu(c: &mut Criterion) {
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 20, 1), (20, 80, 1)),
        ("medium", (5, 30, 1), (30, 120, 1)),
    ];

    let mut group = c.benchmark_group("vcet/cpu/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_vcet_req(Backend::CpuOnly, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let req2 = req.clone();
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("vcet cpu run");
            })
        });
    }

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_vcet_gpu_kernel(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("vcet/gpu_kernel/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_vcet_req(Backend::GpuKernel { device_id: 0 }, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let req2 = req.clone();
                let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("vcet gpu-kernel run");
                assert!(
                    matches!(res.backend_used, BackendUsed::Gpu),
                    "expected GPU backend for VCET bench, got backend={:?}, route={:?}, fallback={:?}",
                    res.backend_used,
                    res.backend_route,
                    res.backend_fallback_reason
                );
                assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
                assert!(
                    res.backend_fallback_reason.is_none(),
                    "expected no fallback reason for VCET bench, got {:?}",
                    res.backend_fallback_reason
                );
            })
        });
    }

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_vcet_gpu_kernel(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_vcet_gpu_kernel_contract_split(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);
    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small/975pairs", (5, 20, 1), (20, 80, 1)),
        ("large/13385pairs", (5, 80, 1), (20, 220, 1)),
    ];

    let mut group = c.benchmark_group("vcet/gpu_kernel_contract_split/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_vcet_req(Backend::GpuKernel { device_id: 0 }, fast_range, slow_range);
        let fast_periods = expand_period_range(fast_range);
        let slow_periods = expand_period_range(slow_range);
        let vcet_cfg = req.vcet.clone().unwrap_or_default();
        let prepared = PreparedVcetGpuKernelBatch::prepare(
            &candles,
            &req.ma_source,
            fast_periods.as_slice(),
            slow_periods.as_slice(),
            &req.strategy,
            &vcet_cfg,
            0,
        )
        .expect("vcet prepare batch");
        assert!(
            !prepared.combo_params().is_empty(),
            "expected VCET prepared batch to contain combos"
        );
        let prewarmed_metrics = prepared
            .eval_metrics(0, &req.strategy, &vcet_cfg)
            .expect("vcet warm-eval prewarm");
        assert_eq!(prewarmed_metrics.len(), prepared.combo_count());

        let req_end_to_end = req.clone();
        group.bench_function(format!("{label}/end_to_end"), |b| {
            b.iter(|| {
                let req2 = req_end_to_end.clone();
                let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("vcet gpu-kernel end-to-end run");
                assert!(
                    matches!(res.backend_used, BackendUsed::Gpu),
                    "expected GPU backend for VCET contract split bench, got backend={:?}, route={:?}, fallback={:?}",
                    res.backend_used,
                    res.backend_route,
                    res.backend_fallback_reason
                );
                assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
                assert!(
                    res.backend_fallback_reason.is_none(),
                    "expected no fallback reason for VCET contract split bench, got {:?}",
                    res.backend_fallback_reason
                );
            })
        });

        let ma_source = req.ma_source.clone();
        let strategy = req.strategy.clone();
        let vcet_prepare = vcet_cfg.clone();
        let fast_periods_prepare = fast_periods.clone();
        let slow_periods_prepare = slow_periods.clone();
        group.bench_function(format!("{label}/prepare_only"), |b| {
            b.iter(|| {
                let batch = PreparedVcetGpuKernelBatch::prepare(
                    &candles,
                    &ma_source,
                    fast_periods_prepare.as_slice(),
                    slow_periods_prepare.as_slice(),
                    &strategy,
                    &vcet_prepare,
                    0,
                )
                .expect("vcet prepare-only run");
                assert!(batch.combo_count() > 0);
            })
        });

        group.bench_function(format!("{label}/warm_eval_only"), |b| {
            b.iter(|| {
                let metrics = prepared
                    .eval_metrics(0, &req.strategy, &vcet_cfg)
                    .expect("vcet warm-eval run");
                assert_eq!(metrics.len(), prepared.combo_count());
            })
        });
    }

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_vcet_gpu_kernel_contract_split(_c: &mut Criterion) {}

criterion_group!(
    vcet_benches,
    bench_vcet_cpu,
    bench_vcet_gpu_kernel,
    bench_vcet_gpu_kernel_contract_split
);
criterion_main!(vcet_benches);
