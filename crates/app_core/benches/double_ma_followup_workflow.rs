use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::double_ma::GpuExecutionPolicy;
use ta_app_core::{
    compute_double_ma_drilldown_blocking_with_candles,
    compute_double_ma_local_stability_blocking_with_candles,
    run_double_ma_optimization_blocking_with_candles, Backend, DoubleMaDrilldownRequest,
    DoubleMaLocalStabilityRequest, DoubleMaParamsResolved, DoubleMaRequest,
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
    let mut vol = Vec::with_capacity(len);

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

fn base_request(
    backend: Backend,
    gpu_execution: GpuExecutionPolicy,
    persist_followup_cache: bool,
) -> DoubleMaRequest {
    DoubleMaRequest {
        backend,
        data_id: "bench-followup".to_string(),
        fast_range: (5, 50, 1),
        slow_range: (20, 200, 5),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: "close".to_string(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy: StrategyConfig::default(),
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(16),
        include_all: Some(false),
        heatmap_bins: Some(0),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution,
        persist_followup_cache,
        hurdle_tau: None,
        include_trade_metrics: false,
        metric_contract: None,
    }
}

fn followup_requests(
    backend: Backend,
    best: &DoubleMaParamsResolved,
    followup_cache_key: Option<String>,
) -> (DoubleMaDrilldownRequest, DoubleMaLocalStabilityRequest) {
    let drilldown = DoubleMaDrilldownRequest {
        data_id: "bench-followup".to_string(),
        backend_hint: Some(backend.clone()),
        followup_cache_key: followup_cache_key.clone(),
        params: best.clone(),
        ma_source: "close".to_string(),
        fast_ma_params: best.fast_ma_params.clone(),
        slow_ma_params: best.slow_ma_params.clone(),
        strategy: StrategyConfig::default(),
        halftrend_filter: None,
        bins: Some(512),
    };
    let local_stability = DoubleMaLocalStabilityRequest {
        data_id: "bench-followup".to_string(),
        backend_hint: Some(backend),
        followup_cache_key,
        params: best.clone(),
        ma_source: "close".to_string(),
        fast_ma_params: best.fast_ma_params.clone(),
        slow_ma_params: best.slow_ma_params.clone(),
        strategy: StrategyConfig::default(),
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        score_mode: None,
        radius: Some(1),
        step: Some(1),
    };
    (drilldown, local_stability)
}

fn bench_followup_case(
    c: &mut Criterion,
    label: &str,
    backend: Backend,
    gpu_execution: GpuExecutionPolicy,
    candles: &Candles,
    cancel: &AtomicBool,
) {
    let opt_req = base_request(backend.clone(), gpu_execution, true);
    let opt_res = run_double_ma_optimization_blocking_with_candles(opt_req, candles, cancel, None)
        .expect("follow-up route probe");
    let followup_cache_key = opt_res
        .followup_cache_key
        .clone()
        .expect("follow-up cache key");
    let best = opt_res.best_params.clone();
    let (cached_drilldown, cached_local_stability) =
        followup_requests(backend.clone(), &best, Some(followup_cache_key));
    let (cold_drilldown, cold_local_stability) = followup_requests(backend, &best, None);

    let mut group = c.benchmark_group(format!("double_ma_followup/{label}/50k"));
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);

    group.bench_function("drilldown_then_local_stability_cold", |b| {
        b.iter(|| {
            let _ =
                compute_double_ma_drilldown_blocking_with_candles(cold_drilldown.clone(), candles)
                    .expect("cold drilldown");
            let _ = compute_double_ma_local_stability_blocking_with_candles(
                cold_local_stability.clone(),
                candles,
            )
            .expect("cold local stability");
        })
    });

    group.bench_function("drilldown_then_local_stability_cached", |b| {
        b.iter(|| {
            let _ = compute_double_ma_drilldown_blocking_with_candles(
                cached_drilldown.clone(),
                candles,
            )
            .expect("cached drilldown");
            let _ = compute_double_ma_local_stability_blocking_with_candles(
                cached_local_stability.clone(),
                candles,
            )
            .expect("cached local stability");
        })
    });

    group.finish();
}

fn bench_double_ma_followup_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);

    bench_followup_case(
        c,
        "cpu",
        Backend::CpuOnly,
        GpuExecutionPolicy::BestEffort,
        &candles,
        &cancel,
    );

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    if vector_ta::cuda::cuda_available() {
        bench_followup_case(
            c,
            "gpu_kernel",
            Backend::GpuKernel { device_id: 0 },
            GpuExecutionPolicy::PreferGpuResident,
            &candles,
            &cancel,
        );
    }
}

criterion_group!(benches, bench_double_ma_followup_workflow);
criterion_main!(benches);
