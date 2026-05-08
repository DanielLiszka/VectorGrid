#[macro_use]
mod bench_support;

use criterion::{criterion_group, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_cpu::{MaczGateParams, OttGateParams};
use ta_app_core::double_ma::HalfTrendRegimeFilter;
use ta_app_core::{
    AsrStrategyParams, Backend, BackendUsed, CompositeCpuRequest, CompositeSignalMode,
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

fn make_asr_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    strategy: StrategyConfig,
    asr: AsrStrategyParams,
    data_id: &str,
) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend,
        asr: Some(asr),
        vcc: None,
        vemr: None,
        rocb: None,
        vcet: None,
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
        data_id: data_id.to_string(),
        fast_range,
        slow_range,
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["alma".to_string()],
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        strategy,
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

fn bench_asr_cpu(c: &mut Criterion) {
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 20, 1), (20, 80, 1)),
        ("medium", (5, 30, 1), (30, 120, 1)),
    ];

    let mut group = c.benchmark_group("asr/cpu/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (label, fast_range, slow_range) in cases.iter().copied() {
        let req = make_asr_req(
            Backend::CpuOnly,
            fast_range,
            slow_range,
            StrategyConfig::default(),
            AsrStrategyParams::default(),
            "cpu/default",
        );
        group.bench_function(label, |b| {
            b.iter(|| {
                let req2 = req.clone();
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req2, &candles, &cancel, None,
                )
                .expect("asr cpu run");
            })
        });
    }

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_asr_gpu_kernel(c: &mut Criterion) {
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
        ("xxlarge/57535pairs", (5, 180, 1), (20, 420, 1)),
        ("ultra/124695pairs", (5, 260, 1), (20, 620, 1)),
    ];

    let mut trade_next_bar = StrategyConfig::default();
    trade_next_bar.trade_on_next_bar = true;
    let mut long_only = StrategyConfig::default();
    long_only.long_only = true;
    let mut no_flip = StrategyConfig::default();
    no_flip.allow_flip = false;
    let mut ema_zscore = AsrStrategyParams::default();
    ema_zscore.z_ma_type = "ema".to_string();

    let surfaces: &[(&str, &str, StrategyConfig, AsrStrategyParams)] = &[
        (
            "asr/gpu_kernel/200k",
            "default",
            StrategyConfig::default(),
            AsrStrategyParams::default(),
        ),
        (
            "asr/gpu_kernel_long_only/200k",
            "long_only",
            long_only,
            AsrStrategyParams::default(),
        ),
        (
            "asr/gpu_kernel_no_flip/200k",
            "no_flip",
            no_flip,
            AsrStrategyParams::default(),
        ),
        (
            "asr/gpu_kernel_trade_next_bar/200k",
            "trade_next_bar",
            trade_next_bar,
            AsrStrategyParams::default(),
        ),
        (
            "asr/gpu_kernel_ema_zscore/200k",
            "ema_zscore",
            StrategyConfig::default(),
            ema_zscore,
        ),
    ];

    for (group_name, surface_label, strategy, asr) in surfaces.iter().cloned() {
        let mut group = c.benchmark_group(group_name);
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(10);

        for (label, fast_range, slow_range) in cases.iter().copied() {
            let req = make_asr_req(
                Backend::GpuKernel { device_id: 0 },
                fast_range,
                slow_range,
                strategy.clone(),
                asr.clone(),
                surface_label,
            );
            group.bench_function(label, |b| {
                b.iter(|| {
                    let req2 = req.clone();
                    let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("asr gpu-kernel run");
                    assert!(
                        matches!(res.backend_used, BackendUsed::Gpu),
                        "expected GPU backend for ASR bench, got backend={:?}, route={:?}, fallback={:?}",
                        res.backend_used,
                        res.backend_route,
                        res.backend_fallback_reason
                    );
                    assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
                    assert!(
                        res.backend_fallback_reason.is_none(),
                        "expected no fallback reason for ASR bench, got {:?}",
                        res.backend_fallback_reason
                    );
                })
            });
        }

        group.finish();
    }
}

criterion_group!(asr_benches, bench_asr_cpu);
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
criterion_group!(asr_benches_cuda, bench_asr_gpu_kernel);

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
serialized_criterion_main!(asr_benches, asr_benches_cuda);
#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
serialized_criterion_main!(asr_benches);
