use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_validate::{
    ReoptWalkForwardConfigOptional, SpaBenchmarkKind, SpaConfigOptional,
};
use ta_app_core::{
    run_double_ma_validation_blocking_with_candles, Backend, CscvPboConfig,
    DoubleMaValidationParams, DoubleMaValidationRequest, IterativeSimConfig, PerturbConfig,
    TradeBootstrapConfig, ValidationEngine, WalkForwardConfigOptional,
};
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
        let drift = 0.00004;
        let noise = (i as f64 * 0.019).sin() * 0.0012;
        px *= 1.0 + drift + noise;
        open.push(px);
        high.push(px * 1.001);
        low.push(px * 0.999);
        close.push(px);
        vol.push(1_000.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_candidates(n: usize) -> Vec<DoubleMaValidationParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let fast_len = 9u16.saturating_add((i as u16).saturating_mul(2));
        let slow_len = fast_len.saturating_add(24);
        out.push(DoubleMaValidationParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len,
            slow_len,
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        });
    }
    out
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn make_sparse_gpu_candidates() -> Vec<DoubleMaValidationParams> {
    [
        (7u16, 41u16),
        (11, 67),
        (19, 73),
        (23, 101),
        (29, 109),
        (31, 137),
        (37, 149),
        (43, 173),
    ]
    .into_iter()
    .map(|(fast_len, slow_len)| DoubleMaValidationParams {
        fast_ma_type: "sma".to_string(),
        slow_ma_type: "sma".to_string(),
        fast_len,
        slow_len,
        fast_ma_params: None,
        slow_ma_params: None,
        strategy: StrategyConfig::default(),
        halftrend_filter: None,
    })
    .collect()
}

fn base_request(candidates: Vec<DoubleMaValidationParams>) -> DoubleMaValidationRequest {
    DoubleMaValidationRequest {
        data_id: "bench-double-ma".to_string(),
        backend: Backend::CpuOnly,
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates,
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed: 123,
        max_candidates: 32,
        validation_engine: ValidationEngine::Vectorized,
        iterative: IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    }
}

fn print_double_ma_route_probe(label: &str, result: &ta_app_core::DoubleMaValidationResult) {
    println!(
        "benchmark route {label}: backend_used={:?} backend_route={} fallback={}",
        result.backend_used,
        result.backend_route.as_deref().unwrap_or("<none>"),
        result
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );
    if let Some(profile) = result.execution_profile.as_ref() {
        let grid_upper_bound = profile
            .unique_fast_periods
            .saturating_mul(profile.unique_slow_periods);
        println!(
            "benchmark profile {label}: requested_candidates={} unique_candidates={} unique_fast_periods={} unique_slow_periods={} evaluated_pairs_on_device={} grid_upper_bound={} expansion_ratio={} gpu_range_eval_calls={} gpu_range_count={} aux_finalist_analytics_wall_ms={} cpu_tail_wall_ms={} gpu_result_materialization_wall_ms={}",
            profile.requested_candidates,
            profile.unique_candidates,
            profile.unique_fast_periods,
            profile.unique_slow_periods,
            profile
                .evaluated_pairs_on_device
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            grid_upper_bound,
            profile
                .expansion_ratio
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "<none>".to_string()),
            profile
                .gpu_range_eval_calls
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            profile
                .gpu_range_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            profile
                .aux_finalist_analytics_wall_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            profile
                .cpu_tail_wall_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            profile
                .gpu_result_materialization_wall_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
        );
    }
}

fn bench_double_ma_validation_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_candidates(32);

    let mut group = c.benchmark_group("double_ma_validate/workflow/50k/32candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = base_request(candidates.clone());
    let route_probe =
        run_double_ma_validation_blocking_with_candles(base.clone(), &candles, &cancel, None)
            .expect("cpu validation route probe");
    print_double_ma_route_probe("double_ma_validate/workflow/50k/32candidates", &route_probe);

    group.bench_function("base", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("base validate");
        })
    });

    let mut with_wf = base.clone();
    with_wf.walk_forward = Some(WalkForwardConfigOptional {
        window_bars: 10_000,
        step_bars: 10_000,
    });
    group.bench_function("walk_forward", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                with_wf.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("wf validate");
        })
    });

    let mut with_cscv = base.clone();
    with_cscv.cscv_pbo = Some(CscvPboConfig {
        slices: 8,
        max_combinations: 2_000,
        threshold: 0.0,
    });
    group.bench_function("cscv_pbo", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                with_cscv.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("cscv validate");
        })
    });

    let mut with_spa = base.clone();
    with_spa.spa = Some(SpaConfigOptional {
        slices: 8,
        bootstrap_samples: 128,
        block_length: 3,
        benchmark: SpaBenchmarkKind::BuyAndHold,
    });
    group.bench_function("spa", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                with_spa.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("spa validate");
        })
    });

    let mut with_reopt = base.clone();
    with_reopt.reopt_walk_forward = Some(ReoptWalkForwardConfigOptional {
        window_bars: 8_000,
        step_bars: 2_000,
        top_n_candidates: 8,
    });
    group.bench_function("reopt_walk_forward", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                with_reopt.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("reopt validate");
        })
    });

    let mut with_perturb = base.clone();
    with_perturb.perturb = Some(PerturbConfig {
        iterations: 32,
        pct_min: -0.10,
        pct_max: 0.10,
        flip_bool_prob: 0.05,
    });
    group.bench_function("perturb_32", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                with_perturb.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("perturb validate");
        })
    });

    let mut with_bootstrap = base.clone();
    with_bootstrap.trade_bootstrap = Some(TradeBootstrapConfig {
        simulations: 250,
        stationary_mean_block_len: None,
    });
    group.bench_function("trade_bootstrap_250", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                with_bootstrap.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("trade bootstrap validate");
        })
    });

    let mut full_stack = base.clone();
    full_stack.walk_forward = with_wf.walk_forward.clone();
    full_stack.cscv_pbo = with_cscv.cscv_pbo.clone();
    full_stack.spa = with_spa.spa.clone();
    full_stack.reopt_walk_forward = with_reopt.reopt_walk_forward.clone();
    full_stack.perturb = with_perturb.perturb.clone();
    full_stack.trade_bootstrap = with_bootstrap.trade_bootstrap.clone();
    group.bench_function("full_stack", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                full_stack.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("full stack validate");
        })
    });

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_validation_gpu_sparse_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_sparse_gpu_candidates();
    let candidate_count = candidates.len();

    let mut group =
        c.benchmark_group("double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates");
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);

    let mut req = base_request(candidates.clone());
    req.data_id = "bench-double-ma-gpu-sparse".to_string();
    req.backend = Backend::GpuKernel { device_id: 0 };
    req.max_candidates = candidate_count;
    req.walk_forward = Some(WalkForwardConfigOptional {
        window_bars: 20_000,
        step_bars: 10_000,
    });
    req.reopt_walk_forward = Some(ReoptWalkForwardConfigOptional {
        window_bars: 20_000,
        step_bars: 10_000,
        top_n_candidates: candidate_count,
    });
    req.cscv_pbo = Some(CscvPboConfig {
        slices: 8,
        max_combinations: 512,
        threshold: 0.0,
    });
    req.spa = Some(SpaConfigOptional {
        slices: 8,
        bootstrap_samples: 128,
        block_length: 3,
        benchmark: SpaBenchmarkKind::BuyAndHold,
    });

    let route_probe =
        run_double_ma_validation_blocking_with_candles(req.clone(), &candles, &cancel, None)
            .expect("gpu sparse validation route probe");
    print_double_ma_route_probe(
        "double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates",
        &route_probe,
    );

    group.bench_function("wf_reopt_cscv_spa", |b| {
        b.iter(|| {
            let _ = run_double_ma_validation_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("gpu sparse validate");
        })
    });

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_validation_gpu_sparse_workflow(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_double_ma_validation_workflow,
    bench_double_ma_validation_gpu_sparse_workflow
);
criterion_main!(benches);
