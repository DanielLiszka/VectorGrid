use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::time::Duration;
#[cfg(feature = "cuda")]
use ta_app_core::prepare_dynamic_indicator_slot_series;
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
use ta_optimizer::mo::{
    Nsga2Config, ParamDef, ParamDomain, RandomMoConfig, SearchSpace, TpeMoConfig,
};
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

#[cfg(feature = "cuda")]
fn build_cpu_only_route_plan() -> Option<ta_app_core::CompiledStrategyPlan> {
    let descriptors = list_dynamic_indicator_descriptors_cached();
    let cpu_only = descriptors.iter().find(|d| {
        d.supports_cpu_single && !d.supports_cuda_single && descriptor_supports_mixed_route_bench(d)
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
            indicators: vec![IndicatorNode {
                id: "cpu_only_slot".to_string(),
                kind: None,
                indicator_id: Some(cpu_only.id.clone()),
                params: dynamic_params_from_defaults(cpu_only),
                output_id: Some(cpu_only.outputs[0].id.clone()),
            }],
            conditions: vec![ConditionNode {
                id: "entry".to_string(),
                op: ConditionOp::Gt,
                lhs: ValueRef::Indicator {
                    id: "cpu_only_slot".to_string(),
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

fn bench_env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn quick_local_bench_settings(default_secs: u64, default_samples: usize) -> (u64, usize) {
    if bench_env_flag("VTA_DYNAMIC_CPU_BENCH_QUICK_LOCAL") {
        (default_secs.min(3), default_samples.min(6))
    } else {
        (default_secs, default_samples)
    }
}

fn print_bench_metadata(
    group_name: &str,
    default_secs: u64,
    default_samples: usize,
    details: &[(&str, String)],
) {
    let (measurement_secs, sample_size) = quick_local_bench_settings(default_secs, default_samples);
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
        format!("measurement_secs={measurement_secs}"),
        format!("sample_size={sample_size}"),
        format!(
            "quick_local={}",
            bench_env_flag("VTA_DYNAMIC_CPU_BENCH_QUICK_LOCAL")
        ),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
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
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(10),
        include_all: Some(false),
        heatmap_bins: Some(0),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,

        hurdle_tau: None,
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
        vcet: None,
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
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
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        top_k: Some(10),
        include_all: Some(false),
        heatmap_bins: Some(0),
        include_trade_metrics: false,
        metric_contract: None,
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
    let (measurement_secs, sample_size) = quick_local_bench_settings(8, 12);
    group.measurement_time(Duration::from_secs(measurement_secs));
    group.sample_size(sample_size);
    print_bench_metadata(
        "dynamic_strategy_cpu/eval/50k",
        8,
        12,
        &[
            ("candles_len", candles.close.len().to_string()),
            (
                "surfaces",
                "double/static_eval,double/dynamic_adapter_eval,composite/static_eval,composite/dynamic_adapter_eval"
                    .to_string(),
            ),
        ],
    );

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
        let (measurement_secs, sample_size) = quick_local_bench_settings(8, 10);
        staging_group.measurement_time(Duration::from_secs(measurement_secs));
        staging_group.sample_size(sample_size);
        print_bench_metadata(
            "dynamic_strategy_cpu/staging/50k",
            8,
            10,
            &[
                ("candles_len", candles.close.len().to_string()),
                ("surface", "dynamic_indicator_slot_staging".to_string()),
            ],
        );
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

    #[cfg(feature = "cuda")]
    if let Some(cpu_only_route_plan) = build_cpu_only_route_plan() {
        let mut staging_group = c.benchmark_group("dynamic_strategy_cpu/staging_cuda_build/50k");
        let (measurement_secs, sample_size) = quick_local_bench_settings(8, 10);
        staging_group.measurement_time(Duration::from_secs(measurement_secs));
        staging_group.sample_size(sample_size);
        print_bench_metadata(
            "dynamic_strategy_cpu/staging_cuda_build/50k",
            8,
            10,
            &[
                ("candles_len", candles.close.len().to_string()),
                ("surface", "dynamic_indicator_cuda_build".to_string()),
            ],
        );
        staging_group.bench_function("cpu_route_slot_prep", |b| {
            b.iter(|| {
                let slot_outputs = prepare_dynamic_indicator_slot_series(
                    &cpu_only_route_plan,
                    &candles,
                    ma_source,
                )
                .expect("cpu-route slot prep should succeed under cuda build");
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
    let (measurement_secs, sample_size) = quick_local_bench_settings(8, 10);
    full_group.measurement_time(Duration::from_secs(measurement_secs));
    full_group.sample_size(sample_size);
    print_bench_metadata(
        "dynamic_strategy_cpu/full_run/2k",
        8,
        10,
        &[
            ("candles_len", candles_mo.close.len().to_string()),
            ("surface", "composite_mo_full_run".to_string()),
            ("dynamic_env_modes", "off,on".to_string()),
        ],
    );

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

    let req_mo_prescan_off = CompositeMoRequest {
        search_space: SearchSpace {
            params: vec![
                ParamDef {
                    name: "fast_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 5,
                        high: 270,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "slow_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 10,
                        high: 300,
                        step: 1,
                        log: false,
                    },
                },
            ],
        },
        sampler: CompositeMoSampler::Random {
            config: RandomMoConfig {
                seed: 1,
                trials: 20,
            },
        },
        ..req_mo.clone()
    };
    let req_mo_prescan_on = CompositeMoRequest {
        gpu_prescan: Some(ta_app_core::CompositeGpuPrescanConfig {
            top_k: 8,
            window: 2,
            predictive_top_k: 0,
            predictive_stride: 8,
            device_id: None,
            require_gpu: false,
            objectives: ta_app_core::CompositeGpuPrescanObjectives::Both,
            ma_pairs: None,
            mode: OptimizationMode::Grid,
        }),
        ..req_mo_prescan_off.clone()
    };
    let mut prescan_group = c.benchmark_group("dynamic_strategy_cpu/composite_mo_prescan/2k");
    let (measurement_secs, sample_size) = quick_local_bench_settings(8, 10);
    prescan_group.measurement_time(Duration::from_secs(measurement_secs));
    prescan_group.sample_size(sample_size);
    print_bench_metadata(
        "dynamic_strategy_cpu/composite_mo_prescan/2k",
        8,
        10,
        &[
            ("candles_len", candles_mo.close.len().to_string()),
            ("surface", "composite_mo_prescan".to_string()),
            (
                "variants",
                "random_large_ranges_no_prescan,random_large_ranges_with_prescan".to_string(),
            ),
        ],
    );
    prescan_group.bench_function("random_large_ranges_no_prescan", |b| {
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo_prescan_off.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo no-prescan run");
            black_box(out.total_evaluations);
        });
    });
    prescan_group.bench_function("random_large_ranges_with_prescan", |b| {
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo_prescan_on.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo prescan run");
            black_box(out.total_evaluations);
        });
    });
    prescan_group.finish();

    let req_mo_sparse_bank = CompositeMoRequest {
        search_space: SearchSpace {
            params: vec![
                ParamDef {
                    name: "fast_len".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["5".into(), "8".into(), "13".into(), "21".into()],
                    },
                },
                ParamDef {
                    name: "slow_len".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["34".into(), "55".into(), "89".into(), "144".into()],
                    },
                },
                ParamDef {
                    name: "fast_ma_type".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["sma".into(), "ema".into()],
                    },
                },
                ParamDef {
                    name: "slow_ma_type".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["ema".into(), "wma".into()],
                    },
                },
            ],
        },
        sampler: CompositeMoSampler::Random {
            config: RandomMoConfig {
                seed: 5,
                trials: 64,
            },
        },
        ..req_mo.clone()
    };
    let mut sparse_group = c.benchmark_group("dynamic_strategy_cpu/composite_mo_sparse_bank/2k");
    let (measurement_secs, sample_size) = quick_local_bench_settings(8, 10);
    sparse_group.measurement_time(Duration::from_secs(measurement_secs));
    sparse_group.sample_size(sample_size);
    print_bench_metadata(
        "dynamic_strategy_cpu/composite_mo_sparse_bank/2k",
        8,
        10,
        &[
            ("candles_len", candles_mo.close.len().to_string()),
            ("surface", "composite_mo_sparse_bank".to_string()),
            ("variants", "random_nonuniform_periods".to_string()),
        ],
    );
    sparse_group.bench_function("random_nonuniform_periods", |b| {
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo_sparse_bank.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo sparse-bank run");
            black_box(out.total_evaluations);
        });
    });
    sparse_group.finish();

    let req_mo_sampler_random = CompositeMoRequest {
        search_space: SearchSpace {
            params: vec![
                ParamDef {
                    name: "fast_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 5,
                        high: 25,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "slow_len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 30,
                        high: 80,
                        step: 1,
                        log: false,
                    },
                },
            ],
        },
        sampler: CompositeMoSampler::Random {
            config: RandomMoConfig {
                seed: 11,
                trials: 512,
            },
        },
        ..req_mo.clone()
    };
    let req_mo_sampler_nsga2 = CompositeMoRequest {
        sampler: CompositeMoSampler::Nsga2 {
            config: Nsga2Config {
                seed: 11,
                population_size: 32,
                generations: 15,
                crossover_prob: 0.9,
                swapping_prob: 0.5,
                mutation_prob: None,
            },
        },
        ..req_mo_sampler_random.clone()
    };
    let req_mo_sampler_tpe = CompositeMoRequest {
        sampler: CompositeMoSampler::Tpe {
            config: TpeMoConfig {
                seed: 11,
                trials: 512,
                startup_trials: 32,
                n_ei_candidates: 24,
                gamma: 0.25,
                batch_size: 64,
                max_model_points: 512,
            },
        },
        ..req_mo_sampler_random.clone()
    };
    let mut sampler_group = c.benchmark_group("dynamic_strategy_cpu/composite_mo_sampler/2k");
    let (measurement_secs, sample_size) = quick_local_bench_settings(8, 10);
    sampler_group.measurement_time(Duration::from_secs(measurement_secs));
    sampler_group.sample_size(sample_size);
    print_bench_metadata(
        "dynamic_strategy_cpu/composite_mo_sampler/2k",
        8,
        10,
        &[
            ("candles_len", candles_mo.close.len().to_string()),
            ("surface", "composite_mo_sampler".to_string()),
            ("samplers", "random,nsga2,tpe".to_string()),
        ],
    );
    sampler_group.bench_function("random_medium_space", |b| {
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo_sampler_random.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo random run");
            black_box(out.total_evaluations);
        });
    });
    sampler_group.bench_function("nsga2_medium_space", |b| {
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo_sampler_nsga2.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo nsga2 run");
            black_box(out.total_evaluations);
        });
    });
    sampler_group.bench_function("tpe_medium_space", |b| {
        b.iter(|| {
            let out = run_composite_mo_optimization_blocking_with_candles(
                req_mo_sampler_tpe.clone(),
                &candles_mo,
                &cancel,
                None,
            )
            .expect("composite_mo tpe run");
            black_box(out.total_evaluations);
        });
    });
    sampler_group.finish();
}

criterion_group!(benches, bench_dynamic_cpu_eval);
criterion_main!(benches);
