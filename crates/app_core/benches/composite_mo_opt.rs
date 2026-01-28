use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::{CompositeMoObjectives, CompositeMoRequest, CompositeMoSampler};
use ta_optimizer::mo::{ParamDef, ParamDomain, RandomMoConfig, SearchSpace};
use ta_optimizer::ObjectiveKind;
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

fn bench_composite_mo_random(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);

    let mut group = c.benchmark_group("composite_mo/random/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeMoRequest {
        data_id: "bench".to_string(),
        fast_ma_type: "sma".to_string(),
        slow_ma_type: "sma".to_string(),
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        base_strategy: StrategyConfig::default(),
        search_space: SearchSpace {
            params: vec![
                ParamDef {
                    name: "fast_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 5,
                        high: 80,
                        step: 5,
                        log: false,
                    },
                },
                ParamDef {
                    name: "slow_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 100,
                        high: 250,
                        step: 10,
                        log: false,
                    },
                },
            ],
        },
        objectives: CompositeMoObjectives {
            maximize: ObjectiveKind::Sharpe,
            minimize: ObjectiveKind::MaxDrawdown,
        },
        sampler: CompositeMoSampler::Random {
            config: RandomMoConfig {
                seed: 123,
                trials: 256,
            },
        },
        constraints: None,
        gpu_prescan: None,
        mode: None,
    };

    group.bench_function("fixed_ma_types/256_trials", |b| {
        b.iter(|| {
            let req2 = base.clone();
            let _ = ta_app_core::run_composite_mo_optimization_blocking_with_candles(
                req2, &candles, &cancel, None,
            )
            .expect("composite mo run");
        })
    });

    let mut with_ma_types = base.clone();
    with_ma_types.search_space.params.insert(
        0,
        ParamDef {
            name: "fast_ma_type".to_string(),
            domain: ParamDomain::Categorical {
                choices: vec!["sma".to_string(), "ema".to_string()],
            },
        },
    );
    with_ma_types.search_space.params.insert(
        1,
        ParamDef {
            name: "slow_ma_type".to_string(),
            domain: ParamDomain::Categorical {
                choices: vec!["sma".to_string(), "ema".to_string()],
            },
        },
    );

    group.bench_function("search_ma_types/256_trials", |b| {
        b.iter(|| {
            let req2 = with_ma_types.clone();
            let _ = ta_app_core::run_composite_mo_optimization_blocking_with_candles(
                req2, &candles, &cancel, None,
            )
            .expect("composite mo run");
        })
    });

    group.finish();
}

criterion_group!(benches, bench_composite_mo_random);
criterion_main!(benches);
