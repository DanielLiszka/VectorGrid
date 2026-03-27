use criterion::{criterion_group, criterion_main, Criterion};
use std::env;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    Backend, BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeCpuRequest,
    CompositeSignalMode, KqsrStrategyParams, KvdfStrategyParams,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

fn base_req(backend: Backend) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend,
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
        fast_range: (5, 20, 1),
        slow_range: (20, 80, 1),
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
    }
}

fn bench_target_enabled(strategy_key: &str) -> bool {
    match env::var("VECTORBT_GPU_NATIVE_BENCH_ONLY") {
        Ok(raw) => raw.eq_ignore_ascii_case(strategy_key),
        Err(_) => true,
    }
}

fn make_kqsr_oscillating_candles(len: usize) -> Candles {
    let len = len.max(240);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let angle = (i as f64) * 0.18;
        let base = 100.0 + angle.sin() * 4.0 + (angle * 0.37).cos() * 1.2;
        let next = 100.0 + ((i + 1) as f64 * 0.18).sin() * 4.0;
        open.push(base - 0.25);
        high.push(base.max(next) + 0.9);
        low.push(base.min(next) - 0.9);
        close.push(base);
        vol.push(1_000.0 + (i % 13) as f64 * 35.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_carm_momentum_candles(len: usize) -> Candles {
    let len = len.max(260);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let px = if i < 70 {
            100.0 + (i as f64 * 0.12).sin() * 0.8 + i as f64 * 0.05
        } else if i < 140 {
            103.5 + (i - 70) as f64 * 0.21 + ((i - 70) as f64 * 0.16).sin() * 0.7
        } else if i < 200 {
            118.0 - (i - 140) as f64 * 0.18 + ((i - 140) as f64 * 0.22).cos() * 0.9
        } else {
            107.0 + (i - 200) as f64 * 0.14 + ((i - 200) as f64 * 0.14).sin() * 0.6
        };
        open.push(px * 0.998);
        high.push(px * 1.004);
        low.push(px * 0.996);
        close.push(px);
        vol.push(1_100.0 + (i % 17) as f64 * 28.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_kvdf_flow_candles(len: usize) -> Candles {
    let len = len.max(280);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64 * 60_000);
        let px = if i < 90 {
            100.0 + (i as f64 * 0.08).sin() * 0.9 + i as f64 * 0.03
        } else if i < 170 {
            102.8 + (i - 90) as f64 * 0.17 + ((i - 90) as f64 * 0.11).cos() * 0.8
        } else if i < 230 {
            116.0 - (i - 170) as f64 * 0.22 + ((i - 170) as f64 * 0.17).sin() * 1.1
        } else {
            102.4 + (i - 230) as f64 * 0.12 + ((i - 230) as f64 * 0.18).cos() * 0.7
        };
        open.push(px * 0.9985);
        high.push(px * 1.0045);
        low.push(px * 0.9955);
        close.push(px);
        vol.push(1_500.0 + (i % 19) as f64 * 55.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_cdmc_cycle_candles(len: usize) -> Candles {
    let len = len.max(260);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let angle = i as f64 * 0.11;
        let trend = if i < len / 2 {
            i as f64 * 0.035
        } else {
            (len - i) as f64 * 0.028
        };
        let px = 100.0 + angle.sin() * 3.6 + (angle * 0.43).cos() * 1.1 + trend;
        open.push(px * 0.9988);
        high.push(px * 1.0038);
        low.push(px * 0.9962);
        close.push(px);
        vol.push(1_250.0 + (i % 11) as f64 * 42.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_kqsr_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.kqsr = Some(KqsrStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}

fn make_carm_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.carm = Some(CarmStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}

fn make_kvdf_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.kvdf = Some(KvdfStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}

fn make_cdmc_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.cdmc = Some(CdmcStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}

fn bench_cpu_cases<F>(c: &mut Criterion, group_name: &str, candles: &Candles, build_req: F)
where
    F: Fn(Backend, (u32, u32, u32), (u32, u32, u32)) -> CompositeCpuRequest,
{
    let cancel = AtomicBool::new(false);
    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 20, 1), (20, 80, 1)),
        ("medium", (5, 30, 1), (30, 120, 1)),
    ];

    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = build_req(Backend::CpuOnly, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    candles,
                    &cancel,
                    None,
                )
                .expect("cpu bench run");
            })
        });
    }

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_gpu_cases<F>(
    c: &mut Criterion,
    route_label: &str,
    group_name: &str,
    candles: &Candles,
    build_req: F,
) where
    F: Fn(Backend, (u32, u32, u32), (u32, u32, u32)) -> CompositeCpuRequest,
{
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let cancel = AtomicBool::new(false);
    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small/975pairs", (5, 20, 1), (20, 80, 1)),
        ("medium/2365pairs", (5, 30, 1), (30, 120, 1)),
        ("large/13385pairs", (5, 80, 1), (20, 220, 1)),
    ];

    let probe = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
        build_req(Backend::GpuKernel { device_id: 0 }, cases[0].1, cases[0].2),
        candles,
        &cancel,
        None,
    )
    .expect("gpu route probe");
    println!(
        "benchmark route {}: backend_used={:?} backend_route={} fallback={}",
        route_label,
        probe.backend_used,
        probe.backend_route.as_deref().unwrap_or("<none>"),
        probe.backend_fallback_reason.as_deref().unwrap_or("<none>")
    );
    assert!(
        matches!(probe.backend_used, BackendUsed::Gpu),
        "expected GPU backend for {} probe, got backend={:?}, route={:?}, fallback={:?}",
        route_label,
        probe.backend_used,
        probe.backend_route,
        probe.backend_fallback_reason
    );
    assert!(
        probe.backend_fallback_reason.is_none(),
        "expected no fallback reason for {} probe, got {:?}",
        route_label,
        probe.backend_fallback_reason
    );

    let mut group = c.benchmark_group(group_name);
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = build_req(Backend::GpuKernel { device_id: 0 }, fast_range, slow_range);
        group.bench_function(label, |b| {
            b.iter(|| {
                let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    candles,
                    &cancel,
                    None,
                )
                .expect("gpu bench run");
                assert!(
                    matches!(res.backend_used, BackendUsed::Gpu),
                    "expected GPU backend for {}, got backend={:?}, route={:?}, fallback={:?}",
                    route_label,
                    res.backend_used,
                    res.backend_route,
                    res.backend_fallback_reason
                );
                assert!(
                    res.backend_fallback_reason.is_none(),
                    "expected no fallback reason for {}, got {:?}",
                    route_label,
                    res.backend_fallback_reason
                );
            })
        });
    }

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_gpu_cases<F>(
    _c: &mut Criterion,
    _route_label: &str,
    _group_name: &str,
    _candles: &Candles,
    _build_req: F,
) where
    F: Fn(Backend, (u32, u32, u32), (u32, u32, u32)) -> CompositeCpuRequest,
{
}

fn bench_kqsr_cpu(c: &mut Criterion) {
    if !bench_target_enabled("kqsr") {
        return;
    }
    let candles = make_kqsr_oscillating_candles(200_000);
    bench_cpu_cases(c, "kqsr/cpu/200k", &candles, make_kqsr_req);
}

fn bench_carm_cpu(c: &mut Criterion) {
    if !bench_target_enabled("carm") {
        return;
    }
    let candles = make_carm_momentum_candles(200_000);
    bench_cpu_cases(c, "carm/cpu/200k", &candles, make_carm_req);
}

fn bench_kvdf_cpu(c: &mut Criterion) {
    if !bench_target_enabled("kvdf") {
        return;
    }
    let candles = make_kvdf_flow_candles(200_000);
    bench_cpu_cases(c, "kvdf/cpu/200k", &candles, make_kvdf_req);
}

fn bench_cdmc_cpu(c: &mut Criterion) {
    if !bench_target_enabled("cdmc") {
        return;
    }
    let candles = make_cdmc_cycle_candles(200_000);
    bench_cpu_cases(c, "cdmc/cpu/200k", &candles, make_cdmc_req);
}

fn bench_kqsr_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("kqsr") {
        return;
    }
    let candles = make_kqsr_oscillating_candles(200_000);
    bench_gpu_cases(
        c,
        "kqsr/gpu_kernel/200k",
        "kqsr/gpu_kernel/200k",
        &candles,
        make_kqsr_req,
    );
}

fn bench_carm_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("carm") {
        return;
    }
    let candles = make_carm_momentum_candles(200_000);
    bench_gpu_cases(
        c,
        "carm/gpu_kernel/200k",
        "carm/gpu_kernel/200k",
        &candles,
        make_carm_req,
    );
}

fn bench_kvdf_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("kvdf") {
        return;
    }
    let candles = make_kvdf_flow_candles(200_000);
    bench_gpu_cases(
        c,
        "kvdf/gpu_kernel/200k",
        "kvdf/gpu_kernel/200k",
        &candles,
        make_kvdf_req,
    );
}

fn bench_cdmc_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("cdmc") {
        return;
    }
    let candles = make_cdmc_cycle_candles(200_000);
    bench_gpu_cases(
        c,
        "cdmc/gpu_kernel/200k",
        "cdmc/gpu_kernel/200k",
        &candles,
        make_cdmc_req,
    );
}

criterion_group!(
    gpu_native_expansion_benches,
    bench_kqsr_cpu,
    bench_kqsr_gpu_kernel,
    bench_carm_cpu,
    bench_carm_gpu_kernel,
    bench_kvdf_cpu,
    bench_kvdf_gpu_kernel,
    bench_cdmc_cpu,
    bench_cdmc_gpu_kernel
);
criterion_main!(gpu_native_expansion_benches);
