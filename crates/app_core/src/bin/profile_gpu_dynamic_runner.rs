use std::sync::atomic::AtomicBool;
use std::time::Instant;

use serde_json::json;
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

#[derive(Clone, Copy)]
enum CandleShape {
    Auto,
    Synth,
    Trending,
    Choppy,
    Breakout,
}

#[derive(Clone, Copy)]
enum ToggleMode {
    Inherit,
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

fn parse_ma_type(flag: &str, default: &str) -> String {
    parse_arg_value(flag).unwrap_or_else(|| default.to_string())
}

fn parse_iterations() -> Result<usize, String> {
    let raw = parse_arg_value("--measure-iterations")
        .or_else(|| parse_arg_value("--iterations"))
        .unwrap_or_else(|| "1".to_string());
    let v = raw
        .trim()
        .parse::<usize>()
        .map_err(|e| format!("invalid --iterations '{}': {e}", raw.trim()))?;
    if v == 0 {
        return Err("--iterations must be >= 1".to_string());
    }
    Ok(v)
}

fn parse_warmup_iterations() -> Result<usize, String> {
    let raw = parse_arg_value("--warmup-iterations").unwrap_or_else(|| "0".to_string());
    raw.trim()
        .parse::<usize>()
        .map_err(|e| format!("invalid --warmup-iterations '{}': {e}", raw.trim()))
}

fn parse_heatmap_bins() -> Result<u16, String> {
    let raw = parse_arg_value("--heatmap-bins").unwrap_or_else(|| "64".to_string());
    raw.trim()
        .parse::<u16>()
        .map_err(|e| format!("invalid --heatmap-bins '{}': {e}", raw.trim()))
}

fn parse_objective_kind() -> Result<ObjectiveKind, String> {
    let raw = parse_arg_value("--objective").unwrap_or_else(|| "sharpe".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "sharpe" => Ok(ObjectiveKind::Sharpe),
        "pnl" => Ok(ObjectiveKind::Pnl),
        "sortino" => Ok(ObjectiveKind::Sortino),
        "omega" => Ok(ObjectiveKind::Omega),
        "calmar" => Ok(ObjectiveKind::Calmar),
        "max_drawdown" | "maxdd" | "drawdown" => Ok(ObjectiveKind::MaxDrawdown),
        "volatility" | "vol" => Ok(ObjectiveKind::Volatility),
        other => Err(format!(
            "unsupported --objective '{other}' (expected: sharpe|pnl|sortino|omega|calmar|max_drawdown|volatility)"
        )),
    }
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

fn parse_toggle_mode(flag: &str, default: &str) -> Result<ToggleMode, String> {
    let raw = parse_arg_value(flag).unwrap_or_else(|| default.to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "inherit" => Ok(ToggleMode::Inherit),
        "off" => Ok(ToggleMode::Off),
        "on" => Ok(ToggleMode::On),
        other => Err(format!(
            "unsupported {flag} '{other}' (expected: inherit|off|on)"
        )),
    }
}

fn parse_toggle_mode_alias(
    primary: &str,
    secondary: &str,
    default: &str,
) -> Result<ToggleMode, String> {
    let raw = parse_arg_value(primary)
        .or_else(|| parse_arg_value(secondary))
        .unwrap_or_else(|| default.to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "inherit" => Ok(ToggleMode::Inherit),
        "off" => Ok(ToggleMode::Off),
        "on" => Ok(ToggleMode::On),
        other => Err(format!(
            "unsupported {primary}/{secondary} '{other}' (expected: inherit|off|on)"
        )),
    }
}

fn parse_emit_json() -> bool {
    std::env::args().skip(1).any(|arg| arg == "--emit-json")
}

fn parse_candle_shape() -> Result<CandleShape, String> {
    let raw = parse_arg_value("--candle-shape").unwrap_or_else(|| "auto".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(CandleShape::Auto),
        "synth" => Ok(CandleShape::Synth),
        "trend" | "trending" => Ok(CandleShape::Trending),
        "choppy" => Ok(CandleShape::Choppy),
        "breakout" => Ok(CandleShape::Breakout),
        other => Err(format!(
            "unsupported --candle-shape '{other}' (expected: auto|synth|trending|choppy|breakout)"
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

fn apply_env_toggle(var_name: &str, mode: ToggleMode) {
    match mode {
        ToggleMode::Inherit => {}
        ToggleMode::Off => std::env::set_var(var_name, "0"),
        ToggleMode::On => std::env::set_var(var_name, "1"),
    }
}

fn apply_nvtx_phase(mode: ToggleMode, is_warmup: bool) {
    match mode {
        ToggleMode::Inherit => {}
        ToggleMode::Off => std::env::set_var("VECTORBT_ENABLE_NVTX", "0"),
        ToggleMode::On => {
            std::env::set_var("VECTORBT_ENABLE_NVTX", if is_warmup { "0" } else { "1" })
        }
    }
}

fn format_optional_ms(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string())
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

fn candles_for_run(kind: StrategyKind, shape: CandleShape, len: usize) -> Candles {
    match shape {
        CandleShape::Auto => candles_for_strategy(kind, len),
        CandleShape::Synth => make_synth_candles(len),
        CandleShape::Trending => make_trending_candles(len),
        CandleShape::Choppy => make_choppy_candles(len),
        CandleShape::Breakout => make_breakout_candles(len),
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
    let warmup_iterations = parse_warmup_iterations().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let heatmap_bins = parse_heatmap_bins().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let objective = parse_objective_kind().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let dynamic_enabled = parse_dynamic_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let cuda_graphs_mode = parse_toggle_mode_alias("--cuda-graphs", "--graphs", "inherit")
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
    let nvtx_mode = parse_toggle_mode("--nvtx", "inherit").unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let device_id = parse_device_id().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let fast_ma_type = parse_ma_type("--fast-ma", "sma");
    let slow_ma_type = parse_ma_type("--slow-ma", "sma");
    let adx_mode = parse_adx_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let candle_shape = parse_candle_shape().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let emit_json = parse_emit_json();

    if !cuda_available() {
        eprintln!("CUDA device not available");
        std::process::exit(1);
    }

    if dynamic_enabled {
        std::env::set_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "1");
    } else {
        std::env::remove_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL");
    }
    apply_env_toggle("VECTORBT_KERNEL_USE_CUDA_GRAPHS", cuda_graphs_mode);
    apply_nvtx_phase(nvtx_mode, warmup_iterations > 0);

    let strategy = strategy_config(strategy_kind, adx_mode);
    let candles = candles_for_run(strategy_kind, candle_shape, 200_000);
    let (fast_range, slow_range) = workload_ranges(workload_kind);
    let cancel = AtomicBool::new(false);

    let req = DoubleMaRequest {
        backend: backend_for_strategy(strategy_kind, adx_mode, device_id),
        data_id: "profile-gpu-dynamic-runner".to_string(),
        fast_range,
        slow_range,
        fast_ma_types: vec![fast_ma_type.clone()],
        slow_ma_types: vec![slow_ma_type.clone()],
        ma_source: "close".to_string(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter: None,
        objective,
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(heatmap_bins),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,

        hurdle_tau: None,
    };

    for i in 0..(warmup_iterations + iterations) {
        let is_warmup = i < warmup_iterations;
        let iter_index = if is_warmup {
            i + 1
        } else {
            i + 1 - warmup_iterations
        };
        apply_nvtx_phase(nvtx_mode, is_warmup);
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
        if emit_json {
            let payload = json!({
                "phase": if is_warmup { "warmup" } else { "measure" },
                "iter": iter_index,
                "backend_used": format!("{:?}", res.backend_used),
                "backend_route": &res.backend_route,
                "runtime_ms": res.runtime_ms,
                "wall_ms": wall_ms,
                "objective": format!("{:?}", objective),
                "fast_ma": &fast_ma_type,
                "slow_ma": &slow_ma_type,
                "heatmap_bins": heatmap_bins,
                "cuda_graphs": match cuda_graphs_mode {
                    ToggleMode::Inherit => "inherit",
                    ToggleMode::Off => "off",
                    ToggleMode::On => "on",
                },
                "nvtx": match nvtx_mode {
                    ToggleMode::Inherit => "inherit",
                    ToggleMode::Off => "off",
                    ToggleMode::On => if is_warmup { "warmup_off" } else { "measure_on" },
                },
                "best_params": {
                    "fast_len": res.best_params.fast_len,
                    "slow_len": res.best_params.slow_len,
                },
                "best_metrics": {
                    "sharpe": res.best_metrics.sharpe,
                    "pnl": res.best_metrics.pnl,
                },
                "gpu_kernel_timing_ms": &res.gpu_kernel_timing_ms,
                "hybrid_timing_ms": &res.hybrid_timing_ms,
                "indicator_route_plan": &res.indicator_route_plan,
                "backend_fallback_reason": res.backend_fallback_reason.as_deref(),
            });
            println!(
                "{}",
                serde_json::to_string(&payload).expect("serialize runner payload")
            );
            continue;
        }
        println!(
            "phase={} iter={} fast_ma={} slow_ma={} backend={:?} route={} objective={:?} runtime_ms={} wall_ms={} heatmap_bins={} cuda_graphs={} best=({}, {}) sharpe={:.6} pnl={:.6}",
            if is_warmup { "warmup" } else { "measure" },
            iter_index,
            fast_ma_type,
            slow_ma_type,
            res.backend_used,
            res.backend_route.as_deref().unwrap_or("<none>"),
            objective,
            res.runtime_ms,
            wall_ms,
            heatmap_bins,
            match cuda_graphs_mode {
                ToggleMode::Inherit => "inherit",
                ToggleMode::Off => "off",
                ToggleMode::On => "on",
            },
            res.best_params.fast_len,
            res.best_params.slow_len,
            res.best_metrics.sharpe,
            res.best_metrics.pnl
        );
        if let Some(timing) = res.gpu_kernel_timing_ms.as_ref() {
            println!(
                "gpu_kernel_timing_ms=route_wall:{} runtime_init:{} set_current:{} gpu_indicator_prep:{} h2d_upload:{} launch_submit_us:{} d2h_copy:{} host_decode:{} explicit_sync:{} host_finalize:{} graph_capture:{} graph_replay_count:{}",
                timing.route_wall_ms,
                format_optional_ms(timing.runtime_init_ms),
                format_optional_ms(timing.set_current_ms),
                format_optional_ms(timing.gpu_indicator_prep_ms),
                format_optional_ms(timing.h2d_upload_ms),
                timing.launch_submit_us
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                format_optional_ms(timing.d2h_copy_ms),
                format_optional_ms(timing.host_decode_ms),
                format_optional_ms(timing.explicit_sync_ms),
                format_optional_ms(timing.host_finalize_ms),
                format_optional_ms(timing.graph_capture_ms),
                timing.graph_replay_count.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())
            );
        }
        if let Some(timing) = res.hybrid_timing_ms.as_ref() {
            println!(
                "hybrid_timing_ms=planner:{} cpu_indicator_prep:{} gpu_indicator_prep:{} h2d_upload:{} kernel_eval:{}",
                timing.planner_ms,
                timing.cpu_indicator_prep_ms,
                timing.gpu_indicator_prep_ms,
                timing.h2d_upload_ms,
                timing.kernel_eval_ms
            );
        }
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
