use std::sync::atomic::AtomicBool;
use std::time::Instant;

use ta_app_core::strategy_presets::{
    breakout_strategy_for_kernel, regime_hybrid_strategy_for_kernel,
    trend_quality_strategy_for_kernel,
};
use ta_app_core::{
    run_double_ma_optimization_blocking_with_candles, Backend, BackendUsed, DoubleMaRequest,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy)]
enum StrategyKind {
    DoubleMa,
    TrendQuality,
    RegimeHybrid,
    BreakoutMomentum,
}

#[derive(Clone, Copy)]
enum WorkloadKind {
    Small,
    Medium,
    Large,
    Xlarge,
}

#[derive(Clone, Copy)]
enum AdxMode {
    Off,
    On,
}

fn parse_arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

fn parse_strategy_kind() -> Result<StrategyKind, String> {
    let raw = parse_arg_value("--strategy").unwrap_or_else(|| "breakout_momentum".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "double_ma" => Ok(StrategyKind::DoubleMa),
        "trend_quality" => Ok(StrategyKind::TrendQuality),
        "regime_hybrid" => Ok(StrategyKind::RegimeHybrid),
        "breakout_momentum" => Ok(StrategyKind::BreakoutMomentum),
        other => Err(format!(
            "unsupported --strategy '{other}' (expected: double_ma|trend_quality|regime_hybrid|breakout_momentum)"
        )),
    }
}

fn parse_workload_kind() -> Result<WorkloadKind, String> {
    let raw = parse_arg_value("--workload").unwrap_or_else(|| "xlarge".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "small" => Ok(WorkloadKind::Small),
        "medium" => Ok(WorkloadKind::Medium),
        "large" => Ok(WorkloadKind::Large),
        "xlarge" => Ok(WorkloadKind::Xlarge),
        other => Err(format!(
            "unsupported --workload '{other}' (expected: small|medium|large|xlarge)"
        )),
    }
}

fn parse_iterations() -> Result<usize, String> {
    let raw = parse_arg_value("--iterations").unwrap_or_else(|| "1".to_string());
    let v = raw
        .trim()
        .parse::<usize>()
        .map_err(|e| format!("invalid --iterations '{}': {e}", raw.trim()))?;
    if v == 0 {
        return Err("--iterations must be >= 1".to_string());
    }
    Ok(v)
}

fn parse_dynamic_mode() -> Result<bool, String> {
    let raw = parse_arg_value("--dynamic-mode").unwrap_or_else(|| "off".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!(
            "unsupported --dynamic-mode '{other}' (expected: on|off)"
        )),
    }
}

fn parse_device_id() -> Result<u32, String> {
    let raw = parse_arg_value("--device-id").unwrap_or_else(|| "0".to_string());
    raw.trim()
        .parse::<u32>()
        .map_err(|e| format!("invalid --device-id '{}': {e}", raw.trim()))
}

fn parse_adx_mode() -> Result<AdxMode, String> {
    let raw = parse_arg_value("--adx-mode").unwrap_or_else(|| "off".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(AdxMode::Off),
        "on" => Ok(AdxMode::On),
        other => Err(format!(
            "unsupported --adx-mode '{other}' (expected: on|off)"
        )),
    }
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

fn make_choppy_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let base = 100.0 + ((i as f64) / 9.0).sin() * 2.0 + ((i as f64) / 37.0).sin() * 0.7;
        let c = base + if (i & 1) == 0 { 0.15 } else { -0.15 };
        let o = base;
        open.push(o);
        close.push(c);
        high.push(o.max(c) + 0.45);
        low.push(o.min(c) - 0.45);
        vol.push(1_000.0 + (i % 5) as f64 * 25.0);
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

fn workload_ranges(kind: WorkloadKind) -> ((u32, u32, u32), (u32, u32, u32)) {
    match kind {
        WorkloadKind::Small => ((5, 50, 1), (20, 200, 5)),
        WorkloadKind::Medium => ((5, 100, 1), (10, 200, 1)),
        WorkloadKind::Large => ((5, 200, 1), (10, 400, 1)),
        WorkloadKind::Xlarge => ((5, 500, 1), (10, 1000, 1)),
    }
}

fn strategy_config(kind: StrategyKind, adx_mode: AdxMode) -> StrategyConfig {
    match kind {
        StrategyKind::DoubleMa => StrategyConfig::default(),
        StrategyKind::TrendQuality => {
            let mut s = trend_quality_strategy_for_kernel();
            s.trend_quality_use_adx = matches!(adx_mode, AdxMode::On);
            s
        }
        StrategyKind::RegimeHybrid => regime_hybrid_strategy_for_kernel(),
        StrategyKind::BreakoutMomentum => {
            let mut s = breakout_strategy_for_kernel();
            s.breakout_use_adx = matches!(adx_mode, AdxMode::On);
            s
        }
    }
}

fn candles_for_strategy(kind: StrategyKind, len: usize) -> Candles {
    match kind {
        StrategyKind::DoubleMa | StrategyKind::TrendQuality => make_trending_candles(len),
        StrategyKind::RegimeHybrid => make_choppy_candles(len),
        StrategyKind::BreakoutMomentum => make_breakout_candles(len),
    }
}

fn backend_for_strategy(kind: StrategyKind, adx_mode: AdxMode, device_id: u32) -> Backend {
    match kind {
        StrategyKind::TrendQuality if matches!(adx_mode, AdxMode::On) => {
            Backend::GpuHybridKernel { device_id }
        }
        StrategyKind::BreakoutMomentum if matches!(adx_mode, AdxMode::On) => {
            Backend::GpuHybridKernel { device_id }
        }
        StrategyKind::RegimeHybrid => Backend::GpuHybridKernel { device_id },
        _ => Backend::GpuKernel { device_id },
    }
}

#[cfg(feature = "cuda")]
fn cuda_available() -> bool {
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn cuda_available() -> bool {
    false
}

fn main() {
    let strategy_kind = parse_strategy_kind().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let workload_kind = parse_workload_kind().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let iterations = parse_iterations().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let dynamic_enabled = parse_dynamic_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let device_id = parse_device_id().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let adx_mode = parse_adx_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    if !cuda_available() {
        eprintln!("CUDA device not available");
        std::process::exit(1);
    }

    if dynamic_enabled {
        std::env::set_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "1");
    } else {
        std::env::remove_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL");
    }

    let strategy = strategy_config(strategy_kind, adx_mode);
    let candles = candles_for_strategy(strategy_kind, 200_000);
    let (fast_range, slow_range) = workload_ranges(workload_kind);
    let cancel = AtomicBool::new(false);

    let req = DoubleMaRequest {
        backend: backend_for_strategy(strategy_kind, adx_mode, device_id),
        data_id: "profile-gpu-dynamic-runner".to_string(),
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
        strategy,
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(64),
        constraints: None,
        vdps: None,
    };

    for i in 0..iterations {
        let t0 = Instant::now();
        let res =
            run_double_ma_optimization_blocking_with_candles(req.clone(), &candles, &cancel, None)
                .unwrap_or_else(|e| {
                    eprintln!("run failed (iter {}): {}", i + 1, e);
                    std::process::exit(1);
                });
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            eprintln!(
                "unexpected backend_used={:?}; expected GPU",
                res.backend_used
            );
            std::process::exit(1);
        }
        let wall_ms = t0.elapsed().as_millis() as u64;
        println!(
            "iter={} backend={:?} route={} runtime_ms={} wall_ms={} best=({}, {}) sharpe={:.6} pnl={:.6}",
            i + 1,
            res.backend_used,
            res.backend_route.as_deref().unwrap_or("<none>"),
            res.runtime_ms,
            wall_ms,
            res.best_params.fast_len,
            res.best_params.slow_len,
            res.best_metrics.sharpe,
            res.best_metrics.pnl
        );
        if let Some(plan) = res.indicator_route_plan.as_ref() {
            println!(
                "indicator_route_plan=fast:{}->{} slow:{}->{}",
                plan.fast.indicator_id, plan.fast.route, plan.slow.indicator_id, plan.slow.route
            );
        }
        if let Some(reason) = res.backend_fallback_reason.as_deref() {
            println!("fallback_reason={reason}");
        }
    }
}
