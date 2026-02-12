use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::{
    CompositeParams, CompositeValidationRequest, CscvPboConfig, PerturbConfig,
    TradeBootstrapConfig, WalkForwardConfigOptional,
};
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

fn make_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 6u16.saturating_add(i_u16.saturating_mul(2));
        let slow_len = fast_len.saturating_add(40);

        let mut macz = ta_app_core::composite_cpu::MaczGateParams::default();
        macz.fast_length = macz.fast_length.saturating_add(i_u16 % 5);
        macz.slow_length = macz.slow_length.saturating_add(i_u16 % 7);

        let mut ott = ta_app_core::composite_cpu::OttGateParams::default();
        ott.period = ott.period.saturating_add(i_u16 % 5);

        let halftrend = ta_app_core::double_ma::HalfTrendRegimeFilter {
            amplitude: 2u16.saturating_add(i_u16 % 3),
            channel_deviation: 2.0,
            atr_period: 100u16.saturating_add(i_u16 % 10),
        };

        out.push(CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len,
            slow_len,
            halftrend,
            macz,
            ott,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn bench_composite_validation_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_candidates(32);

    let mut group = c.benchmark_group("composite_validate/workflow/50k/32candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeValidationRequest {
        data_id: "bench".to_string(),
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: candidates.clone(),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 10_000,
            step_bars: 10_000,
        }),
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed: 123,
        max_candidates: candidates.len(),
        validation_engine: ta_app_core::ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
    };

    group.bench_function("wf_only", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("wf validate");
        })
    });

    let mut with_perturb = base.clone();
    with_perturb.perturb = Some(PerturbConfig {
        iterations: 64,
        pct_min: 0.05,
        pct_max: 0.15,
        flip_bool_prob: 0.05,
    });

    group.bench_function("wf_perturb64", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                with_perturb.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("wf+perturb validate");
        })
    });

    let mut with_mc = with_perturb.clone();
    with_mc.trade_bootstrap = Some(TradeBootstrapConfig { simulations: 250 });

    group.bench_function("wf_perturb64_mc250", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                with_mc.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("wf+perturb+mc validate");
        })
    });

    let mut with_cscv = with_mc.clone();
    with_cscv.cscv_pbo = Some(CscvPboConfig {
        slices: 8,
        max_combinations: 2_000,
        threshold: 0.0,
    });

    group.bench_function("wf_perturb64_mc250_cscv8_2k", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                with_cscv.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("wf+perturb+mc+cscv validate");
        })
    });

    group.finish();
}

criterion_group!(benches, bench_composite_validation_workflow);
criterion_main!(benches);
