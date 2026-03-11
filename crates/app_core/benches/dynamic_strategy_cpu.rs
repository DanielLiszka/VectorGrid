use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::Duration;
use ta_app_core::{
    compile_dynamic_plan_for_composite_cpu_config, compile_dynamic_plan_for_double_ma_config,
    compile_strategy_plan, eval_dynamic_plan_metrics_cpu,
    list_dynamic_indicator_descriptors_cached, prepare_dynamic_cpu_context,
    prepare_dynamic_indicator_slot_series_cpu, run_composite_mo_optimization_blocking_with_candles,
    Backend, CompositeCpuRequest, CompositeMoObjectives, CompositeMoRequest, CompositeMoSampler,
    CompositeSignalMode, ConditionGroup, ConditionGroupMode, ConditionNode, ConditionOp,
    DoubleMaRequest, ExecutionPolicy, IndicatorNode, IndicatorParamKV, MovingAverageSpec,
    SignalBindings, SignalProgramGraph, StrategyIr, StrategyPlanBackend, StrategyProfileKind,
    ValueRef, STRATEGY_IR_SCHEMA_VERSION,
};
use ta_optimizer::mo::{ParamDef, ParamDomain, RandomMoConfig, SearchSpace};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::indicators::moving_averages::ma::{ma, MaData};
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
        let drift = 0.00004;
        let noise = (i as f64 * 0.021).sin() * 0.0008;
        px *= 1.0 + drift + noise;
        open.push(px);
        high.push(px * 1.001);
        low.push(px * 0.999);
        close.push(px);
        vol.push(2_000.0 + ((i % 13) as f64) * 15.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn compute_log_returns(prices: &[f64]) -> Vec<f64> {
    if prices.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(prices.len());
    out.push(0.0);
    for w in prices.windows(2) {
        let prev = w[0];
        let cur = w[1];
        if prev.is_finite() && cur.is_finite() && prev > 0.0 && cur > 0.0 {
            out.push((cur / prev).ln());
        } else {
            out.push(0.0);
        }
    }
    out
}

fn dynamic_params_from_defaults(
    descriptor: &ta_app_core::IndicatorDescriptor,
) -> Vec<IndicatorParamKV> {
    descriptor
        .params
        .iter()
        .filter_map(|param| {
            param.default.clone().map(|value| IndicatorParamKV {
                key: param.key.clone(),
                value,
            })
        })
        .collect()
}

fn descriptor_supports_mixed_route_bench(descriptor: &ta_app_core::IndicatorDescriptor) -> bool {
    !descriptor.outputs.is_empty()
        && descriptor
            .params
            .iter()
            .all(|param| !param.required || param.default.is_some())
}

fn build_mixed_route_plan() -> Option<ta_app_core::CompiledStrategyPlan> {
    let descriptors = list_dynamic_indicator_descriptors_cached();
    let gpu_preferred = descriptors.iter().find(|d| {
        d.prefers_gpu_by_speedup
            && d.supports_cpu_single
            && descriptor_supports_mixed_route_bench(d)
    })?;
    let cpu_preferred = descriptors.iter().find(|d| {
        !d.prefers_gpu_by_speedup
            && d.supports_cpu_single
            && d.id != gpu_preferred.id
            && descriptor_supports_mixed_route_bench(d)
    })?;

    let ir = StrategyIr {
        schema_version: STRATEGY_IR_SCHEMA_VERSION,
        profile: StrategyProfileKind::DoubleMa,
        ma_source: "close".to_string(),
        fast_ma: MovingAverageSpec {
            ma_type: "sma".to_string(),
            params: HashMap::new(),
        },
        slow_ma: MovingAverageSpec {
            ma_type: "sma".to_string(),
            params: HashMap::new(),
        },
        program: SignalProgramGraph {
            indicators: vec![
                IndicatorNode {
                    id: "gpu_slot".to_string(),
                    kind: None,
                    indicator_id: Some(gpu_preferred.id.clone()),
                    params: dynamic_params_from_defaults(gpu_preferred),
                    output_id: Some(gpu_preferred.outputs[0].id.clone()),
                },
                IndicatorNode {
                    id: "cpu_slot".to_string(),
                    kind: None,
                    indicator_id: Some(cpu_preferred.id.clone()),
                    params: dynamic_params_from_defaults(cpu_preferred),
                    output_id: Some(cpu_preferred.outputs[0].id.clone()),
                },
            ],
            conditions: vec![ConditionNode {
                id: "entry".to_string(),
                op: ConditionOp::Gt,
                lhs: ValueRef::Indicator {
                    id: "gpu_slot".to_string(),
                },
                rhs: ValueRef::constant(0.0),
            }],
            groups: vec![ConditionGroup {
                id: "entry_group".to_string(),
                mode: ConditionGroupMode::All,
                condition_ids: vec!["entry".to_string()],
            }],
            signals: SignalBindings {
                long_entry_group: Some("entry_group".to_string()),
                long_exit_group: None,
                short_entry_group: None,
                short_exit_group: None,
            },
        },
        execution: ExecutionPolicy {
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            halftrend: None,
            macz: None,
            ott: None,
            signal_mode: None,
        },
    };
    compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None).ok()
}

fn bench_dynamic_cpu_eval(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let strategy = StrategyConfig::default();
    let ma_source = "close";

    let fast = ma(
        "sma",
        MaData::Candles {
            candles: &candles,
            source: ma_source,
        },
        10,
    )
    .unwrap_or_else(|_| vec![f64::NAN; candles.close.len()]);
    let slow = ma(
        "sma",
        MaData::Candles {
            candles: &candles,
            source: ma_source,
        },
        30,
    )
    .unwrap_or_else(|_| vec![f64::NAN; candles.close.len()]);
    let close = vector_ta::utilities::data_loader::source_type(&candles, "close");
    let lr = compute_log_returns(close);
    let start_t = 128usize;

    let req_double = DoubleMaRequest {
        backend: Backend::CpuOnly,
        data_id: "bench".to_string(),
        fast_range: (10, 10, 1),
        slow_range: (30, 30, 1),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: ma_source.to_string(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy: strategy.clone(),
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        top_k: Some(10),
        include_all: Some(false),
        heatmap_bins: Some(0),
        constraints: None,
        vdps: None,
    };
    let plan_double = compile_dynamic_plan_for_double_ma_config(
        &req_double.ma_source,
        "sma",
        "sma",
        req_double.fast_ma_params.as_ref(),
        req_double.slow_ma_params.as_ref(),
        &req_double.strategy,
        req_double.halftrend_filter.as_ref(),
        StrategyPlanBackend::Cpu,
    )
    .expect("compile dynamic double plan");
    let ctx_double = prepare_dynamic_cpu_context(&plan_double, &candles, ma_source)
        .expect("prepare dynamic double context");

    let req_composite = CompositeCpuRequest {
        backend: Backend::CpuOnly,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        mkcs: None,
        data_id: "bench".to_string(),
        fast_range: (10, 10, 1),
        slow_range: (30, 30, 1),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: ma_source.to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        strategy: strategy.clone(),
        halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        },
        macz: ta_app_core::composite_cpu::MaczGateParams::default(),
        ott: ta_app_core::composite_cpu::OttGateParams::default(),
        signal_mode: CompositeSignalMode::HardGate,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        gpu_prescan: None,
        top_k: Some(10),
        include_all: Some(false),
        heatmap_bins: Some(0),
    };
    let plan_composite = compile_dynamic_plan_for_composite_cpu_config(
        &req_composite.ma_source,
        "sma",
        "sma",
        req_composite.fast_ma_params.as_ref(),
        req_composite.slow_ma_params.as_ref(),
        &req_composite.strategy,
        &req_composite.halftrend,
        &req_composite.macz,
        &req_composite.ott,
        req_composite.signal_mode,
        StrategyPlanBackend::Cpu,
    )
    .expect("compile dynamic composite plan");
    let ctx_composite = prepare_dynamic_cpu_context(&plan_composite, &candles, ma_source)
        .expect("prepare dynamic composite context");

    let mut group = c.benchmark_group("dynamic_strategy_cpu/eval/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(12);

    group.bench_function("double/static_eval", |b| {
        b.iter(|| {
            let m = ta_strategies::double_ma::eval_double_ma_from_log_returns_and_mas_f64(
                &lr, &fast, &slow, start_t, &strategy,
            );
            black_box(m);
        })
    });

    group.bench_function("double/dynamic_adapter_eval", |b| {
        b.iter(|| {
            let m = eval_dynamic_plan_metrics_cpu(
                &plan_double,
                &ctx_double,
                &fast,
                &slow,
                start_t,
                true,
            )
            .expect("dynamic eval");
            black_box(m);
        })
    });

    group.bench_function("composite/static_eval", |b| {
        b.iter(|| {
            let m =
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                    &ctx_composite.lr,
                    &ctx_composite.ma_source_prices,
                    &fast,
                    &slow,
                    ctx_composite
                        .composite_halftrend
                        .as_deref()
                        .expect("halftrend"),
                    ctx_composite.macz_hist.as_deref().expect("macz"),
                    ctx_composite.ott_line.as_deref().expect("ott"),
                    start_t,
                    &strategy,
                    CompositeSignalMode::HardGate,
                );
            black_box(m);
        })
    });

    group.bench_function("composite/dynamic_adapter_eval", |b| {
        b.iter(|| {
            let m = eval_dynamic_plan_metrics_cpu(
                &plan_composite,
                &ctx_composite,
                &fast,
                &slow,
                start_t,
                true,
            )
            .expect("dynamic eval");
            black_box(m);
        })
    });

    group.finish();

    if let Some(mixed_route_plan) = build_mixed_route_plan() {
        let mut staging_group = c.benchmark_group("dynamic_strategy_cpu/staging/50k");
        staging_group.measurement_time(Duration::from_secs(8));
        staging_group.sample_size(10);
        staging_group.bench_function("mixed_route_slot_prep_cpu", |b| {
            b.iter(|| {
                let slot_outputs = prepare_dynamic_indicator_slot_series_cpu(
                    &mixed_route_plan,
                    &candles,
                    ma_source,
                )
                .expect("mixed-route slot prep should succeed");
                black_box(slot_outputs.len());
            })
        });
        staging_group.finish();
    }

    let candles_mo = make_synth_candles(2_000);
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let req_mo = CompositeMoRequest {
        data_id: "bench".to_string(),
        fast_ma_type: "sma".to_string(),
        slow_ma_type: "sma".to_string(),
        ma_source: ma_source.to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        base_strategy: strategy,
        signal_mode: CompositeSignalMode::HardGate,
        search_space: SearchSpace {
            params: vec![
                ParamDef {
                    name: "fast_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 5,
                        high: 8,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "slow_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 10,
                        high: 14,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "ott_ma_type".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["sma".to_string()],
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
                seed: 999,
                trials: 40,
            },
        },
        constraints: None,
        gpu_prescan: None,
        mode: None,
    };

    let mut full_group = c.benchmark_group("dynamic_strategy_cpu/full_run/2k");
    full_group.measurement_time(Duration::from_secs(8));
    full_group.sample_size(10);

    full_group.bench_function("composite_mo/static_env_off", |b| {
        let key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo static run");
            black_box(out.pareto.len());
        });
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    });

    full_group.bench_function("composite_mo/dynamic_env_on", |b| {
        let key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev = std::env::var(key).ok();
        std::env::set_var(key, "1");
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo dynamic run");
            black_box(out.pareto.len());
        });
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    });

    full_group.finish();
}

criterion_group!(benches, bench_dynamic_cpu_eval);
criterion_main!(benches);
