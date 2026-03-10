use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::double_ma::VdpsConfig;
use ta_app_core::{Backend, DoubleMaRequest};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;

use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy)]
struct MaBenchCase {
    label: &'static str,
    fast_ma: &'static str,
    slow_ma: &'static str,
}

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

fn make_trending_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let base = 100.0 + i as f64;
        open.push(base);
        high.push(base + 1.0);
        low.push(base - 1.0);
        close.push(base + 0.5);
        vol.push(1_000.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_breakout_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let up_start = len / 3;
    let down_start = (len * 2) / 3;
    for i in 0..len {
        ts.push(i as i64);
        let base = if i < up_start {
            100.0 + i as f64 * 0.02
        } else if i < down_start {
            102.0 + (i - up_start) as f64 * 0.35
        } else {
            102.0 + (down_start - up_start) as f64 * 0.35 - (i - down_start) as f64 * 0.38
        };
        let n = ((i as f64) / 7.0).sin() * 0.08;
        let o = base + n;
        let c = if i < down_start {
            base + 0.30 + n
        } else {
            base - 0.30 + n
        };
        open.push(o);
        close.push(c);
        high.push(o.max(c) + 0.22);
        low.push(o.min(c) - 0.22);
        let burst = if i < up_start {
            950.0
        } else if i < down_start {
            2_200.0
        } else {
            2_450.0
        };
        vol.push(burst + ((i % 11) as f64) * 35.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn alma_params_default() -> HashMap<String, f64> {
    let mut p: HashMap<String, f64> = HashMap::new();
    p.insert("offset".to_string(), 0.85);
    p.insert("sigma".to_string(), 6.0);
    p
}

fn epma_params_default() -> HashMap<String, f64> {
    let mut p: HashMap<String, f64> = HashMap::new();
    p.insert("offset".to_string(), 3.0);
    p
}

fn trend_quality_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.trend_quality_enable = true;
    s.trend_quality_use_adx = false;
    s
}

fn regime_hybrid_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.regime_hybrid_enable = true;
    s
}

fn breakout_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.long_only = false;
    s.breakout_momentum_enable = true;
    s.breakout_use_adx = false;
    s.breakout_bbands_mult = 0.8;
    s.breakout_cmo_min_abs = 0.0;
    s.breakout_macd_fast_period = 5;
    s.breakout_macd_slow_period = 13;
    s.breakout_macd_signal_period = 4;
    s.breakout_adosc_short_period = 2;
    s.breakout_adosc_long_period = 4;
    s
}

fn marrt_strategy_for_kernel() -> StrategyConfig {
    StrategyConfig::default()
}

fn vfct_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.volume_flow_confirm_enable = true;
    s
}

fn pit_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.pressure_impulse_trend_enable = true;
    s
}

fn set_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.slope_efficiency_trend_enable = true;
    s
}

fn mts_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.momentum_triple_stack_enable = true;
    s
}

fn orc_strategy_for_kernel() -> StrategyConfig {
    let mut s = StrategyConfig::default();
    s.oscillator_reversion_confluence_enable = true;
    s
}

fn expand_range_u16((start, end, step): (u32, u32, u32)) -> Vec<u16> {
    let push_checked = |out: &mut Vec<u16>, v: u32| {
        if v <= u16::MAX as u32 {
            out.push(v as u16);
        }
    };

    if step == 0 || start == end {
        let mut out = Vec::with_capacity(1);
        push_checked(&mut out, start);
        return out;
    }

    let step = step.max(1);
    let (lo, hi) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let mut out = Vec::new();
    let mut v = lo;
    loop {
        push_checked(&mut out, v);
        if v == hi {
            break;
        }
        match v.checked_add(step) {
            Some(next) if next > v && next <= hi => v = next,
            _ => break,
        }
    }
    out
}

fn count_pairs_fast_lt_slow(fast_periods: &[u16], slow_periods: &[u16]) -> usize {
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return 0;
    }
    let mut slow_sorted: Vec<u16> = slow_periods.to_vec();
    slow_sorted.sort_unstable();

    let s_len = slow_sorted.len();
    let mut total: usize = 0;
    for &f in fast_periods {
        let idx = slow_sorted.partition_point(|&s| s <= f);
        total = total.saturating_add(s_len.saturating_sub(idx));
    }
    total
}

fn run_with_dynamic_strategy_opt_env<T>(enabled: bool, f: impl FnOnce() -> T) -> T {
    let key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
    let prev = std::env::var(key).ok();
    if enabled {
        std::env::set_var(key, "1");
    } else {
        std::env::remove_var(key);
    }
    let out = f();
    match prev {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
    out
}

fn bench_double_ma_cpu(c: &mut Criterion) {
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaBenchCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 50, 1), (20, 200, 5)),
        ("medium", (5, 100, 1), (10, 200, 1)),
        ("large", (5, 200, 1), (10, 400, 1)),
    ];

    for ma in ma_cases {
        let mut group = c.benchmark_group(format!("double_ma/cpu/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(if ma.fast_ma == "alma" || ma.slow_ma == "alma" {
            10
        } else {
            20
        });

        for (label, fast_range, slow_range) in cases.iter().copied() {
            let fast_periods = expand_range_u16(fast_range);
            let slow_periods = expand_range_u16(slow_range);
            let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
            let name = format!("{label}/{pairs}pairs");

            let fast_ma_params = if ma.fast_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };
            let slow_ma_params = if ma.slow_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };

            let req = DoubleMaRequest {
                backend: Backend::CpuOnly,
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params,
                slow_ma_params,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(64),
                constraints: None,
                vdps: None,
            };

            group.bench_function(&name, |b| {
                b.iter(|| {
                    let req2 = req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("cpu run");
                })
            });
        }

        group.finish();
    }
}

#[cfg(feature = "cuda")]
fn bench_double_ma_gpu_sweep(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaBenchCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 50, 1), (20, 200, 5)),
        ("medium", (5, 100, 1), (10, 200, 1)),
        ("large", (5, 200, 1), (10, 400, 1)),
    ];

    for ma in ma_cases {
        let mut group = c.benchmark_group(format!("double_ma/gpu_sweep/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(if ma.fast_ma == "alma" || ma.slow_ma == "alma" {
            10
        } else {
            20
        });

        for (label, fast_range, slow_range) in cases.iter().copied() {
            let fast_periods = expand_range_u16(fast_range);
            let slow_periods = expand_range_u16(slow_range);
            let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
            let name = format!("{label}/{pairs}pairs");

            let fast_ma_params = if ma.fast_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };
            let slow_ma_params = if ma.slow_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };

            let req = DoubleMaRequest {
                backend: Backend::GpuOnly { device_id: 0 },
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params,
                slow_ma_params,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(64),
                constraints: None,
                vdps: None,
            };

            group.bench_function(&name, |b| {
                b.iter(|| {
                    let req2 = req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("gpu sweep run");
                })
            });
        }

        group.finish();
    }
}

#[cfg(not(feature = "cuda"))]
fn bench_double_ma_gpu_sweep(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_gpu_kernel(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaBenchCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32), Option<&'static str>)] = &[
        ("small", (5, 50, 1), (20, 200, 5), None),
        ("medium", (5, 100, 1), (10, 200, 1), None),
        ("large", (5, 200, 1), (10, 400, 1), None),
        ("xlarge", (5, 500, 1), (10, 1000, 1), None),
        (
            "xlarge_batched_1gb",
            (5, 500, 1),
            (10, 1000, 1),
            Some("1024"),
        ),
        ("xxlarge", (5, 1000, 1), (10, 2000, 1), None),
        (
            "xxlarge_batched_1gb",
            (5, 1000, 1),
            (10, 2000, 1),
            Some("1024"),
        ),
        ("large_batched_1gb", (5, 200, 1), (10, 400, 1), Some("1024")),
    ];

    let prev_budget = std::env::var("VECTORBT_KERNEL_VRAM_BUDGET_MB").ok();

    for ma in ma_cases {
        let mut group = c.benchmark_group(format!("double_ma/gpu_kernel/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(if ma.fast_ma == "alma" || ma.slow_ma == "alma" {
            10
        } else {
            20
        });

        for (label, fast_range, slow_range, budget_mb) in cases.iter().copied() {
            match budget_mb {
                Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
                None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
            }

            let fast_periods = expand_range_u16(fast_range);
            let slow_periods = expand_range_u16(slow_range);
            let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
            let name = format!("{label}/{pairs}pairs");

            let fast_ma_params = if ma.fast_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };
            let slow_ma_params = if ma.slow_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };

            let req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params,
                slow_ma_params,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(64),
                constraints: None,
                vdps: None,
            };

            group.bench_function(&name, |b| {
                b.iter(|| {
                    let req2 = req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("gpu kernel run");
                })
            });
        }

        group.finish();
    }

    match prev_budget {
        Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
        None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_gpu_kernel(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_gpu_hybrid_kernel(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaBenchCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32), Option<&'static str>)] = &[
        ("small", (5, 50, 1), (20, 200, 5), None),
        ("large", (5, 200, 1), (10, 400, 1), None),
        ("xlarge", (5, 500, 1), (10, 1000, 1), None),
        (
            "xlarge_batched_1gb",
            (5, 500, 1),
            (10, 1000, 1),
            Some("1024"),
        ),
        ("xxlarge", (5, 1000, 1), (10, 2000, 1), None),
        (
            "xxlarge_batched_1gb",
            (5, 1000, 1),
            (10, 2000, 1),
            Some("1024"),
        ),
    ];

    let prev_budget = std::env::var("VECTORBT_KERNEL_VRAM_BUDGET_MB").ok();
    let prev_hybrid = std::env::var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL").ok();
    std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", "1");

    for ma in ma_cases {
        let mut group = c.benchmark_group(format!("double_ma/gpu_hybrid_kernel/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(if ma.fast_ma == "alma" || ma.slow_ma == "alma" {
            10
        } else {
            20
        });

        for (label, fast_range, slow_range, budget_mb) in cases.iter().copied() {
            match budget_mb {
                Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
                None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
            }

            let fast_periods = expand_range_u16(fast_range);
            let slow_periods = expand_range_u16(slow_range);
            let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
            let name = format!("{label}/{pairs}pairs");

            let fast_ma_params = if ma.fast_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };
            let slow_ma_params = if ma.slow_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };

            let req = DoubleMaRequest {
                backend: Backend::GpuHybridKernel { device_id: 0 },
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params,
                slow_ma_params,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(64),
                constraints: None,
                vdps: None,
            };

            group.bench_function(&name, |b| {
                b.iter(|| {
                    let req2 = req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("gpu hybrid kernel run");
                })
            });
        }

        group.finish();
    }

    match prev_budget {
        Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
        None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
    }
    match prev_hybrid {
        Some(v) => std::env::set_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL", v),
        None => std::env::remove_var("VECTORBT_ENABLE_GPU_HYBRID_KERNEL"),
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_gpu_hybrid_kernel(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_gpu_kernel_extra_ma(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "jsa_jsa",
            fast_ma: "jsa",
            slow_ma: "jsa",
        },
        MaBenchCase {
            label: "vwma_vwma",
            fast_ma: "vwma",
            slow_ma: "vwma",
        },
        MaBenchCase {
            label: "vpwma_vpwma",
            fast_ma: "vpwma",
            slow_ma: "vpwma",
        },
        MaBenchCase {
            label: "frama_frama",
            fast_ma: "frama",
            slow_ma: "frama",
        },
        MaBenchCase {
            label: "nma_nma",
            fast_ma: "nma",
            slow_ma: "nma",
        },
        MaBenchCase {
            label: "cwma_cwma",
            fast_ma: "cwma",
            slow_ma: "cwma",
        },
        MaBenchCase {
            label: "cora_wave_cora_wave",
            fast_ma: "cora_wave",
            slow_ma: "cora_wave",
        },
        MaBenchCase {
            label: "hma_hma",
            fast_ma: "hma",
            slow_ma: "hma",
        },
        MaBenchCase {
            label: "epma_epma",
            fast_ma: "epma",
            slow_ma: "epma",
        },
        MaBenchCase {
            label: "jma_jma",
            fast_ma: "jma",
            slow_ma: "jma",
        },
        MaBenchCase {
            label: "edcf_edcf",
            fast_ma: "edcf",
            slow_ma: "edcf",
        },
        MaBenchCase {
            label: "supersmoother_supersmoother",
            fast_ma: "supersmoother",
            slow_ma: "supersmoother",
        },
        MaBenchCase {
            label: "zlema_zlema",
            fast_ma: "zlema",
            slow_ma: "zlema",
        },
        MaBenchCase {
            label: "ehlers_itrend_ehlers_itrend",
            fast_ma: "ehlers_itrend",
            slow_ma: "ehlers_itrend",
        },
    ];

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32), Option<&'static str>)] = &[
        ("small", (5, 50, 1), (20, 200, 5), None),
        ("medium", (5, 100, 1), (10, 200, 1), None),
        ("large", (5, 200, 1), (10, 400, 1), None),
        ("large_batched_1gb", (5, 200, 1), (10, 400, 1), Some("1024")),
    ];

    let prev_budget = std::env::var("VECTORBT_KERNEL_VRAM_BUDGET_MB").ok();

    for ma in ma_cases {
        let mut group = c.benchmark_group(format!("double_ma/gpu_kernel_extra/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(10);

        for (label, fast_range, slow_range, budget_mb) in cases.iter().copied() {
            match budget_mb {
                Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
                None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
            }

            let fast_periods = expand_range_u16(fast_range);
            let slow_periods = expand_range_u16(slow_range);
            let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
            let name = format!("{label}/{pairs}pairs");

            let fast_ma_params = if ma.fast_ma == "epma" {
                Some(epma_params_default())
            } else {
                None
            };
            let slow_ma_params = if ma.slow_ma == "epma" {
                Some(epma_params_default())
            } else {
                None
            };

            let req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params,
                slow_ma_params,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(64),
                constraints: None,
                vdps: None,
            };

            group.bench_function(&name, |b| {
                b.iter(|| {
                    let req2 = req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("gpu kernel run");
                })
            });
        }

        group.finish();
    }

    match prev_budget {
        Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
        None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_gpu_kernel_extra_ma(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_gpu_kernel_strategy_profiles(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let trend_candles = make_trending_candles(200_000);
    let breakout_candles = make_breakout_candles(200_000);
    let cancel = AtomicBool::new(false);

    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaBenchCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];

    let strategy_cases: &[(&str, fn() -> StrategyConfig)] = &[
        ("marrt", marrt_strategy_for_kernel),
        ("pit", pit_strategy_for_kernel),
        ("set", set_strategy_for_kernel),
        ("mts", mts_strategy_for_kernel),
        ("orc", orc_strategy_for_kernel),
        ("vfct", vfct_strategy_for_kernel),
        ("trend_quality", trend_quality_strategy_for_kernel),
        ("regime_hybrid", regime_hybrid_strategy_for_kernel),
        ("breakout_momentum", breakout_strategy_for_kernel),
    ];

    let workloads: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("large", (5, 200, 1), (10, 400, 1)),
        ("xlarge", (5, 500, 1), (10, 1000, 1)),
    ];

    for ma in ma_cases {
        let mut group =
            c.benchmark_group(format!("double_ma/gpu_kernel_strategy/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(if ma.fast_ma == "alma" || ma.slow_ma == "alma" {
            10
        } else {
            20
        });

        for (strategy_label, make_strategy) in strategy_cases.iter().copied() {
            for (workload_label, fast_range, slow_range) in workloads.iter().copied() {
                let fast_periods = expand_range_u16(fast_range);
                let slow_periods = expand_range_u16(slow_range);
                let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
                let name = format!("{strategy_label}/{workload_label}/{pairs}pairs");

                let fast_ma_params = if ma.fast_ma == "alma" {
                    Some(alma_params_default())
                } else {
                    None
                };
                let slow_ma_params = if ma.slow_ma == "alma" {
                    Some(alma_params_default())
                } else {
                    None
                };

                let req = DoubleMaRequest {
                    backend: Backend::GpuKernel { device_id: 0 },
                    data_id: "bench".to_string(),
                    fast_range,
                    slow_range,
                    fast_ma_types: vec![ma.fast_ma.to_string()],
                    slow_ma_types: vec![ma.slow_ma.to_string()],
                    ma_source: "close".to_string(),
                    export_all_csv_path: None,
                    fast_ma_params,
                    slow_ma_params,
                    fast_ma_param_sweeps: None,
                    slow_ma_param_sweeps: None,
                    strategy: make_strategy(),
                    halftrend_filter: None,
                    objective: ObjectiveKind::Sharpe,
                    mode: OptimizationMode::Grid,
                    top_k: Some(50),
                    include_all: Some(false),
                    heatmap_bins: Some(64),
                    constraints: None,
                    vdps: None,
                };

                group.bench_function(&name, |b| {
                    b.iter(|| {
                        let req2 = req.clone();
                        let candles =
                            if strategy_label == "breakout_momentum" || strategy_label == "vfct" {
                                &breakout_candles
                            } else {
                                &trend_candles
                            };
                        let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                            req2, candles, &cancel, None,
                        )
                        .expect("gpu kernel strategy profile run");
                    })
                });
            }
        }

        group.finish();
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_gpu_kernel_strategy_profiles(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_gpu_kernel_dynamic_preflight_strategy_profiles(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let trend_candles = make_trending_candles(200_000);
    let breakout_candles = make_breakout_candles(200_000);
    let cancel = AtomicBool::new(false);

    let strategy_cases: &[(&str, fn() -> StrategyConfig)] = &[
        ("double_ma", StrategyConfig::default),
        ("marrt", marrt_strategy_for_kernel),
        ("pit", pit_strategy_for_kernel),
        ("set", set_strategy_for_kernel),
        ("mts", mts_strategy_for_kernel),
        ("orc", orc_strategy_for_kernel),
        ("vfct", vfct_strategy_for_kernel),
        ("trend_quality", trend_quality_strategy_for_kernel),
        ("regime_hybrid", regime_hybrid_strategy_for_kernel),
        ("breakout_momentum", breakout_strategy_for_kernel),
    ];
    let mode_cases: &[(&str, bool)] = &[("legacy_off", false), ("dynamic_on", true)];
    let workloads: &[(&str, (u32, u32, u32), (u32, u32, u32))] = &[
        ("small", (5, 50, 1), (20, 200, 5)),
        ("medium", (5, 100, 1), (10, 200, 1)),
        ("large", (5, 200, 1), (10, 400, 1)),
        ("xlarge", (5, 500, 1), (10, 1000, 1)),
    ];

    let mut group = c.benchmark_group("double_ma/gpu_kernel_dynamic_preflight/sma_sma/200k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (strategy_label, make_strategy) in strategy_cases.iter().copied() {
        for (mode_label, dynamic_enabled) in mode_cases.iter().copied() {
            for (workload_label, fast_range, slow_range) in workloads.iter().copied() {
                let fast_periods = expand_range_u16(fast_range);
                let slow_periods = expand_range_u16(slow_range);
                let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
                let name = format!("{strategy_label}/{mode_label}/{workload_label}/{pairs}pairs");

                let req = DoubleMaRequest {
                    backend: Backend::GpuKernel { device_id: 0 },
                    data_id: "bench".to_string(),
                    fast_range,
                    slow_range,
                    fast_ma_types: vec!["sma".to_string()],
                    slow_ma_types: vec!["sma".to_string()],
                    ma_source: "close".to_string(),
                    export_all_csv_path: None,
                    fast_ma_params: None,
                    slow_ma_params: None,
                    fast_ma_param_sweeps: None,
                    slow_ma_param_sweeps: None,
                    strategy: make_strategy(),
                    halftrend_filter: None,
                    objective: ObjectiveKind::Sharpe,
                    mode: OptimizationMode::Grid,
                    top_k: Some(50),
                    include_all: Some(false),
                    heatmap_bins: Some(64),
                    constraints: None,
                    vdps: None,
                };

                group.bench_function(&name, |b| {
                    b.iter(|| {
                        let req2 = req.clone();
                        let candles =
                            if strategy_label == "breakout_momentum" || strategy_label == "vfct" {
                                &breakout_candles
                            } else {
                                &trend_candles
                            };
                        run_with_dynamic_strategy_opt_env(dynamic_enabled, || {
                            let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                                req2, candles, &cancel, None,
                            )
                            .expect("gpu kernel dynamic-preflight strategy profile run");
                        });
                    })
                });
            }
        }
    }

    group.finish();
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_gpu_kernel_dynamic_preflight_strategy_profiles(_c: &mut Criterion) {}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_double_ma_gpu_kernel_vdps(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }
    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);
    let ma_cases: &[MaBenchCase] = &[
        MaBenchCase {
            label: "sma_sma",
            fast_ma: "sma",
            slow_ma: "sma",
        },
        MaBenchCase {
            label: "alma_alma",
            fast_ma: "alma",
            slow_ma: "alma",
        },
    ];
    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32), Option<&'static str>)] = &[
        ("large", (5, 200, 1), (10, 400, 1), None),
        (
            "xlarge_batched_1gb",
            (5, 500, 1),
            (10, 1000, 1),
            Some("1024"),
        ),
        (
            "xxlarge_batched_1gb",
            (5, 1000, 1),
            (10, 2000, 1),
            Some("1024"),
        ),
    ];
    let prev_budget = std::env::var("VECTORBT_KERNEL_VRAM_BUDGET_MB").ok();

    for ma in ma_cases {
        let mut group = c.benchmark_group(format!("double_ma/gpu_kernel_vdps/{}/200k", ma.label));
        group.measurement_time(Duration::from_secs(8));
        group.sample_size(if ma.fast_ma == "alma" || ma.slow_ma == "alma" {
            10
        } else {
            20
        });

        for (label, fast_range, slow_range, budget_mb) in cases.iter().copied() {
            match budget_mb {
                Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
                None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
            }

            let fast_periods = expand_range_u16(fast_range);
            let slow_periods = expand_range_u16(slow_range);
            let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
            let grid_name = format!("{label}/grid/{pairs}pairs");
            let vdps_name = format!("{label}/vdps/{pairs}pairs");
            let fast_ma_params = if ma.fast_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };
            let slow_ma_params = if ma.slow_ma == "alma" {
                Some(alma_params_default())
            } else {
                None
            };

            let grid_req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params: fast_ma_params.clone(),
                slow_ma_params: slow_ma_params.clone(),
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::Grid,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: None,
            };
            group.bench_function(&grid_name, |b| {
                b.iter(|| {
                    let req2 = grid_req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("gpu kernel grid run");
                })
            });

            let vdps_req = DoubleMaRequest {
                backend: Backend::GpuKernel { device_id: 0 },
                data_id: "bench".to_string(),
                fast_range,
                slow_range,
                fast_ma_types: vec![ma.fast_ma.to_string()],
                slow_ma_types: vec![ma.slow_ma.to_string()],
                ma_source: "close".to_string(),
                export_all_csv_path: None,
                fast_ma_params,
                slow_ma_params,
                fast_ma_param_sweeps: None,
                slow_ma_param_sweeps: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                objective: ObjectiveKind::Sharpe,
                mode: OptimizationMode::GpuVdps,
                top_k: Some(50),
                include_all: Some(false),
                heatmap_bins: Some(0),
                constraints: None,
                vdps: Some(VdpsConfig {
                    eta: Some(4),
                    rungs: Some(vec![0.0625, 0.125, 0.25, 0.5, 1.0]),
                    min_initial_pairs: Some(4096),
                    max_survivor_seeds: Some(256),
                    max_period_candidates: Some(384),
                    random_injection_ratio: Some(0.10),
                    elite_mutation_ratio: Some(0.50),
                    differential_ratio: Some(0.40),
                    deterministic: Some(true),
                    seed: Some(1),
                    quality_check_max_pairs: Some(0),
                    quality_check_top_k: Some(20),
                }),
            };
            group.bench_function(&vdps_name, |b| {
                b.iter(|| {
                    let req2 = vdps_req.clone();
                    let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                        req2, &candles, &cancel, None,
                    )
                    .expect("gpu kernel vdps run");
                })
            });
        }

        group.finish();
    }
    match prev_budget {
        Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
        None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_double_ma_gpu_kernel_vdps(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_double_ma_cpu,
    bench_double_ma_gpu_sweep,
    bench_double_ma_gpu_kernel,
    bench_double_ma_gpu_hybrid_kernel,
    bench_double_ma_gpu_kernel_extra_ma,
    bench_double_ma_gpu_kernel_strategy_profiles,
    bench_double_ma_gpu_kernel_dynamic_preflight_strategy_profiles,
    bench_double_ma_gpu_kernel_vdps
);
criterion_main!(benches);
