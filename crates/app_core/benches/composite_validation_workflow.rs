use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::composite_validate::{
    ReoptWalkForwardConfigOptional, SpaBenchmarkKind, SpaConfigOptional,
};
use ta_app_core::{
    CarmStrategyParams, CdmcStrategyParams, CompositeParams, CompositeValidationRequest,
    CscvPboConfig, PerturbConfig, TradeBootstrapConfig, WalkForwardConfigOptional,
};
use ta_app_core::{KqsrStrategyParams, KvdfStrategyParams};
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
            macz,
            ott,
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
}

fn make_asr_candidates(n: usize) -> Vec<CompositeParams> {
    make_asr_candidates_with_config(n, "sma", 0, StrategyConfig::default())
}

fn make_asr_candidates_with_zscore_mode(
    n: usize,
    z_ma_type: &str,
    z_devtype: u8,
) -> Vec<CompositeParams> {
    make_asr_candidates_with_config(n, z_ma_type, z_devtype, StrategyConfig::default())
}

fn make_asr_candidates_with_config(
    n: usize,
    z_ma_type: &str,
    z_devtype: u8,
    strategy: StrategyConfig,
) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);
        let mut asr = ta_app_core::AsrStrategyParams::default();
        asr.z_ma_type = z_ma_type.to_string();
        asr.z_devtype = z_devtype;
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
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
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: strategy.clone(),
        });
    }
    out
}

fn make_vcc_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: Some(ta_app_core::VccStrategyParams::default()),
            vemr: None,
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_vemr_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: Some(ta_app_core::VemrStrategyParams::default()),
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_rocb_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: Some(ta_app_core::RocbStrategyParams::default()),
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_vcet_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            vcet: Some(ta_app_core::VcetStrategyParams::default()),
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_mkcs_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 8u16.saturating_add(i_u16 % 12);
        let slow_len = fast_len.saturating_add(24);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            vcet: None,
            mkcs: Some(ta_app_core::MkcsStrategyParams::default()),
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_carm_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 10u16.saturating_add(i_u16.saturating_mul(2));
        let slow_len = fast_len.saturating_add(20);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: Some(CarmStrategyParams::default()),
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_cdmc_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 10u16.saturating_add(i_u16.saturating_mul(2));
        let slow_len = fast_len.saturating_add(20);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: Some(CdmcStrategyParams::default()),
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_kqsr_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 10u16.saturating_add(i_u16.saturating_mul(2));
        let slow_len = fast_len.saturating_add(20);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: Some(KqsrStrategyParams::default()),
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: StrategyConfig::default(),
        });
    }
    out
}

fn make_kvdf_candidates(n: usize) -> Vec<CompositeParams> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let i_u16 = i as u16;
        let fast_len = 10u16.saturating_add(i_u16.saturating_mul(2));
        let slow_len = fast_len.saturating_add(20);
        out.push(CompositeParams {
            fast_ma_type: "ema".to_string(),
            slow_ma_type: "ema".to_string(),
            fast_len,
            slow_len,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            vcet: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: Some(KvdfStrategyParams::default()),
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
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
    print_bench_metadata(
        "composite_validate/workflow/50k/32candidates",
        &[
            ("backend", "CpuOnly".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("candidate_count", candidates.len().to_string()),
            (
                "surfaces",
                "wf_only,wf_perturb64,wf_perturb64_mc250,wf_perturb64_mc250_cscv8_2k".to_string(),
            ),
        ],
    );

    let base = CompositeValidationRequest {
        data_id: "bench".to_string(),
        backend: ta_app_core::Backend::CpuOnly,
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: candidates.clone(),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 10_000,
            step_bars: 10_000,
        }),
        reopt_walk_forward: None,
        spa: None,
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
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };

    let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("cpu validation route probe");
    println!(
        "benchmark route composite_validate/workflow/50k/32candidates: backend_used={:?} backend_route={} fallback={}",
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );

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
    with_mc.trade_bootstrap = Some(TradeBootstrapConfig {
        simulations: 250,
        stationary_mean_block_len: None,
    });

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

    let mut bench_cpu_profile = |name: &str,
                                 candles_len: usize,
                                 candidates: Vec<CompositeParams>| {
        let candidate_count = candidates.len();
        let candles = make_synth_candles(candles_len);
        let cancel = AtomicBool::new(false);
        let mut group = c.benchmark_group(format!(
            "composite_validate/cpu/{name}/{candles_len}/{}candidates",
            candidate_count
        ));
        group.measurement_time(Duration::from_secs(6));
        group.sample_size(10);
        print_bench_metadata(
            &format!(
                "composite_validate/cpu/{name}/{candles_len}/{}candidates",
                candidate_count
            ),
            &[
                ("backend", "CpuOnly".to_string()),
                ("candles_len", candles_len.to_string()),
                ("candidate_count", candidate_count.to_string()),
                ("surface", "wf_reopt_cscv_spa".to_string()),
            ],
        );

        let base = CompositeValidationRequest {
            data_id: format!("bench-cpu-{name}"),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates,
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 10_000,
                step_bars: 10_000,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 15_000,
                step_bars: 10_000,
                top_n_candidates: 4,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 64,
                threshold: 0.0,
            }),
            seed: 321,
            max_candidates: candidate_count,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
            base.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("cpu profile validation route probe");
        println!(
            "benchmark route composite_validate/cpu/{}/{}/{}candidates: backend_used={:?} backend_route={} fallback={}",
            name,
            candles_len,
            candidate_count,
            route_probe.backend_used,
            route_probe.backend_route.as_deref().unwrap_or("<none>"),
            route_probe
                .backend_fallback_reason
                .as_deref()
                .unwrap_or("<none>")
        );

        group.bench_function("wf_reopt_cscv_spa", |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                    base.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("cpu profile wf+reopt+cscv+spa validate");
            })
        });

        group.finish();
    };

    bench_cpu_profile("base_composite", 50_000, make_candidates(8));
    bench_cpu_profile("asr", 50_000, make_asr_candidates(8));
    let mut asr_long_only = StrategyConfig::default();
    asr_long_only.long_only = true;
    bench_cpu_profile(
        "asr_long_only",
        50_000,
        make_asr_candidates_with_config(8, "sma", 0, asr_long_only),
    );
    let mut asr_no_flip = StrategyConfig::default();
    asr_no_flip.allow_flip = false;
    bench_cpu_profile(
        "asr_no_flip",
        50_000,
        make_asr_candidates_with_config(8, "sma", 0, asr_no_flip),
    );
    let mut asr_trade_next_bar = StrategyConfig::default();
    asr_trade_next_bar.trade_on_next_bar = true;
    bench_cpu_profile(
        "asr_trade_next_bar",
        50_000,
        make_asr_candidates_with_config(8, "sma", 0, asr_trade_next_bar),
    );
    bench_cpu_profile(
        "asr_ema",
        50_000,
        make_asr_candidates_with_zscore_mode(8, "ema", 0),
    );
    bench_cpu_profile("vcc", 50_000, make_vcc_candidates(8));
    bench_cpu_profile("vemr", 50_000, make_vemr_candidates(8));
    bench_cpu_profile("rocb", 50_000, make_rocb_candidates(8));
    bench_cpu_profile("vcet", 50_000, make_vcet_candidates(8));
    bench_cpu_profile("mkcs", 50_000, make_mkcs_candidates(8));
    bench_cpu_profile("kqsr", 50_000, make_kqsr_candidates(8));
    bench_cpu_profile("carm", 50_000, make_carm_candidates(8));
    bench_cpu_profile("kvdf", 50_000, make_kvdf_candidates(8));
    bench_cpu_profile("cdmc", 50_000, make_cdmc_candidates(8));
    bench_cpu_profile("kqsr", 100_000, make_kqsr_candidates(32));
    bench_cpu_profile("carm", 100_000, make_carm_candidates(32));
    bench_cpu_profile("cdmc", 100_000, make_cdmc_candidates(32));
    bench_cpu_profile("mkcs", 100_000, make_mkcs_candidates(32));
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_composite_validation_gpu_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_asr_candidates(64);

    let mut group = c.benchmark_group("composite_validate/gpu_kernel/asr/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);
    print_bench_metadata(
        "composite_validate/gpu_kernel/asr/100k/64candidates",
        &[
            ("backend", "GpuKernel".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("candidate_count", candidates.len().to_string()),
            ("surfaces", "wf_cscv,in_sample_192candidates".to_string()),
        ],
    );

    let base = CompositeValidationRequest {
        data_id: "bench-gpu".to_string(),
        backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: candidates.clone(),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 20_000,
            step_bars: 20_000,
        }),
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: 8,
            max_combinations: 2_000,
            threshold: 0.0,
        }),
        seed: 123,
        max_candidates: candidates.len(),
        validation_engine: ta_app_core::ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };

    let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("gpu validation route probe");
    println!(
        "benchmark route composite_validate/gpu_kernel/asr/100k/64candidates: backend_used={:?} backend_route={} fallback={}",
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );
    assert!(
        matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu),
        "expected GPU backend for ASR validation bench, got backend={:?}, route={:?}, fallback={:?}",
        route_probe.backend_used,
        route_probe.backend_route,
        route_probe.backend_fallback_reason
    );
    assert!(
        route_probe.backend_fallback_reason.is_none(),
        "expected no fallback reason for ASR validation bench, got {:?}",
        route_probe.backend_fallback_reason
    );
    assert_eq!(
        route_probe.backend_route.as_deref(),
        Some("GpuKernel"),
        "expected exact GpuKernel route for ASR validation bench, got route={:?}",
        route_probe.backend_route
    );

    group.bench_function("wf_cscv", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("gpu wf+cscv validate");
        })
    });

    let mut vram_heavy = base.clone();
    vram_heavy.candidates = make_asr_candidates(192);
    vram_heavy.max_candidates = vram_heavy.candidates.len();
    vram_heavy.walk_forward = None;
    vram_heavy.cscv_pbo = None;

    group.bench_function("in_sample_192candidates", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                vram_heavy.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("gpu in-sample validate");
        })
    });

    group.finish();

    let mut bench_profile = |name: &str, candles_len: usize, candidates: Vec<CompositeParams>| {
        let candidate_count = candidates.len();
        let candles = make_synth_candles(candles_len);
        let cancel = AtomicBool::new(false);
        let mut group = c.benchmark_group(format!(
            "composite_validate/gpu_kernel/{name}/{candles_len}/{}candidates",
            candidate_count
        ));
        group.measurement_time(Duration::from_secs(6));
        group.sample_size(10);
        print_bench_metadata(
            &format!(
                "composite_validate/gpu_kernel/{name}/{candles_len}/{}candidates",
                candidate_count
            ),
            &[
                ("backend", "GpuKernel".to_string()),
                ("candles_len", candles_len.to_string()),
                ("candidate_count", candidate_count.to_string()),
                ("surface", "wf_reopt_cscv_spa".to_string()),
            ],
        );

        let base = CompositeValidationRequest {
            data_id: format!("bench-gpu-{name}"),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates,
            walk_forward: Some(WalkForwardConfigOptional {
                window_bars: 10_000,
                step_bars: 10_000,
            }),
            reopt_walk_forward: Some(ReoptWalkForwardConfigOptional {
                window_bars: 15_000,
                step_bars: 10_000,
                top_n_candidates: 4,
            }),
            spa: Some(SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 64,
                block_length: 2,
                benchmark: SpaBenchmarkKind::BuyAndHold,
            }),
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: Some(CscvPboConfig {
                slices: 6,
                max_combinations: 64,
                threshold: 0.0,
            }),
            seed: 321,
            max_candidates: candidate_count,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
            base.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("gpu profile validation route probe");
        println!(
            "benchmark route composite_validate/gpu_kernel/{}/{}/{}candidates: backend_used={:?} backend_route={} fallback={}",
            name,
            candles_len,
            candidate_count,
            route_probe.backend_used,
            route_probe.backend_route.as_deref().unwrap_or("<none>"),
            route_probe
                .backend_fallback_reason
                .as_deref()
                .unwrap_or("<none>")
        );
        assert!(
            matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu),
            "expected GPU backend for {} validation bench, got backend={:?}, route={:?}, fallback={:?}",
            name,
            route_probe.backend_used,
            route_probe.backend_route,
            route_probe.backend_fallback_reason
        );
        assert!(
            route_probe.backend_fallback_reason.is_none(),
            "expected no fallback reason for {} validation bench, got {:?}",
            name,
            route_probe.backend_fallback_reason
        );
        assert_eq!(
            route_probe.backend_route.as_deref(),
            Some("GpuKernel"),
            "expected exact GpuKernel route for {} validation bench, got route={:?}",
            name,
            route_probe.backend_route
        );

        group.bench_function("wf_reopt_cscv_spa", |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                    base.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("gpu profile wf+reopt+cscv+spa validate");
            })
        });

        group.finish();
    };

    bench_profile("base_composite", 50_000, make_candidates(8));
    bench_profile("asr", 50_000, make_asr_candidates(8));
    let mut asr_long_only = StrategyConfig::default();
    asr_long_only.long_only = true;
    bench_profile(
        "asr_long_only",
        50_000,
        make_asr_candidates_with_config(8, "sma", 0, asr_long_only),
    );
    let mut asr_no_flip = StrategyConfig::default();
    asr_no_flip.allow_flip = false;
    bench_profile(
        "asr_no_flip",
        50_000,
        make_asr_candidates_with_config(8, "sma", 0, asr_no_flip),
    );
    let mut asr_trade_next_bar = StrategyConfig::default();
    asr_trade_next_bar.trade_on_next_bar = true;
    bench_profile(
        "asr_trade_next_bar",
        50_000,
        make_asr_candidates_with_config(8, "sma", 0, asr_trade_next_bar),
    );
    bench_profile(
        "asr_ema",
        50_000,
        make_asr_candidates_with_zscore_mode(8, "ema", 0),
    );
    bench_profile("vcc", 50_000, make_vcc_candidates(8));
    bench_profile("vemr", 50_000, make_vemr_candidates(8));
    bench_profile("rocb", 50_000, make_rocb_candidates(8));
    bench_profile("vcet", 50_000, make_vcet_candidates(8));
    bench_profile("mkcs", 50_000, make_mkcs_candidates(8));
    bench_profile("kqsr", 50_000, make_kqsr_candidates(8));
    bench_profile("carm", 50_000, make_carm_candidates(8));
    bench_profile("kvdf", 50_000, make_kvdf_candidates(8));
    bench_profile("cdmc", 50_000, make_cdmc_candidates(8));
    bench_profile("kqsr", 100_000, make_kqsr_candidates(32));
    bench_profile("vemr", 100_000, make_vemr_candidates(64));
    bench_profile("rocb", 100_000, make_rocb_candidates(64));
    bench_profile("vcet", 100_000, make_vcet_candidates(64));
    bench_profile("kvdf", 100_000, make_kvdf_candidates(64));
    bench_profile("carm", 100_000, make_carm_candidates(32));
    bench_profile("cdmc", 100_000, make_cdmc_candidates(32));
    bench_profile("mkcs", 100_000, make_mkcs_candidates(32));
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_composite_validation_gpu_workflow(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_composite_validation_workflow,
    bench_composite_validation_gpu_workflow
);
criterion_main!(benches);
