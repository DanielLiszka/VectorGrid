use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

use ta_app_core::strategy_presets::{
    breakout_strategy_for_kernel, regime_hybrid_strategy_for_kernel,
    trend_quality_strategy_for_kernel,
};
use ta_app_core::{
    parse_timeframe_label, read_candles_from_csv_compat, resample_candles,
    run_double_ma_optimization_blocking_with_candles, Backend, BackendUsed, DoubleMaRequest,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

#[derive(Serialize)]
struct RouteMatrixArtifact {
    csv_path: String,
    timeframe: String,
    total_bars: usize,
    cases: Vec<RouteMatrixCase>,
}

#[derive(Serialize)]
struct RouteMatrixCase {
    case_label: String,
    requested_backend: String,
    backend_used: String,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    runtime_ms: u64,
    wall_ms: u64,
    best_fast_len: u16,
    best_slow_len: u16,
    best_sharpe: f64,
    best_pnl: f64,
}

struct CaseSpec {
    case_label: &'static str,
    strategy: StrategyConfig,
    backend: Backend,
}

fn parse_flag(name: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next();
        }
    }
    None
}

fn parse_csv() -> PathBuf {
    parse_flag("--csv")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/benchmarks/AAPL.csv"))
}

fn parse_timeframe() -> String {
    parse_flag("--timeframe").unwrap_or_else(|| "15m".to_string())
}

fn parse_out_path(timeframe: &str) -> PathBuf {
    parse_flag("--out").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(format!(
            "target/profile_route_matrix_aapl_{}.json",
            timeframe.replace(['/', '\\', ':'], "_")
        ))
    })
}

fn parse_device_id() -> u32 {
    parse_flag("--device-id")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
}

fn load_candles(csv_path: &PathBuf, timeframe: &str) -> Result<Candles, String> {
    let base = read_candles_from_csv_compat(csv_path.to_string_lossy().as_ref())?;
    let tf = parse_timeframe_label(timeframe)?;
    if tf.label.eq_ignore_ascii_case("1m") {
        Ok(base)
    } else {
        resample_candles(&base, tf.interval_ms)
    }
}

fn cpu_only() -> Backend {
    Backend::CpuOnly
}

fn gpu_kernel(device_id: u32) -> Backend {
    Backend::GpuKernel { device_id }
}

fn gpu_hybrid(device_id: u32) -> Backend {
    Backend::GpuHybridKernel { device_id }
}

fn requested_backend_label(backend: &Backend) -> &'static str {
    match backend {
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuKernel { .. } => "GpuKernel",
        Backend::GpuHybridKernel { .. } => "GpuHybridKernel",
        Backend::GpuOnly { .. } => "GpuOnly",
        Backend::Auto { .. } => "Auto",
    }
}

fn backend_used_label(backend: BackendUsed) -> &'static str {
    match backend {
        BackendUsed::Cpu => "Cpu",
        BackendUsed::Gpu => "Gpu",
    }
}

fn case_specs(device_id: u32) -> Vec<CaseSpec> {
    let mut trend_off = trend_quality_strategy_for_kernel();
    trend_off.trend_quality_use_adx = false;

    let mut trend_on = trend_quality_strategy_for_kernel();
    trend_on.trend_quality_use_adx = true;

    let mut breakout_off = breakout_strategy_for_kernel();
    breakout_off.breakout_use_adx = false;

    let mut breakout_on = breakout_strategy_for_kernel();
    breakout_on.breakout_use_adx = true;

    vec![
        CaseSpec {
            case_label: "CompositeTrend/ADX off/CpuOnly",
            strategy: trend_off.clone(),
            backend: cpu_only(),
        },
        CaseSpec {
            case_label: "CompositeTrend/ADX off/GpuKernel",
            strategy: trend_off,
            backend: gpu_kernel(device_id),
        },
        CaseSpec {
            case_label: "CompositeTrend/ADX on/CpuOnly",
            strategy: trend_on.clone(),
            backend: cpu_only(),
        },
        CaseSpec {
            case_label: "CompositeTrend/ADX on/GpuHybridKernel",
            strategy: trend_on,
            backend: gpu_hybrid(device_id),
        },
        CaseSpec {
            case_label: "BreakoutMomentum/ADX off/CpuOnly",
            strategy: breakout_off.clone(),
            backend: cpu_only(),
        },
        CaseSpec {
            case_label: "BreakoutMomentum/ADX off/GpuKernel",
            strategy: breakout_off,
            backend: gpu_kernel(device_id),
        },
        CaseSpec {
            case_label: "BreakoutMomentum/ADX on/CpuOnly",
            strategy: breakout_on.clone(),
            backend: cpu_only(),
        },
        CaseSpec {
            case_label: "BreakoutMomentum/ADX on/GpuHybridKernel",
            strategy: breakout_on,
            backend: gpu_hybrid(device_id),
        },
        CaseSpec {
            case_label: "RegimeHybrid/CpuOnly",
            strategy: regime_hybrid_strategy_for_kernel(),
            backend: cpu_only(),
        },
        CaseSpec {
            case_label: "RegimeHybrid/GpuHybridKernel",
            strategy: regime_hybrid_strategy_for_kernel(),
            backend: gpu_hybrid(device_id),
        },
    ]
}

fn run_case(spec: &CaseSpec, candles: &Candles) -> Result<RouteMatrixCase, String> {
    let req = DoubleMaRequest {
        backend: spec.backend.clone(),
        data_id: "profile-route-matrix".to_string(),
        fast_range: (5, 100, 1),
        slow_range: (10, 200, 2),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: "close".to_string(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy: spec.strategy.clone(),
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        top_k: Some(20),
        include_all: Some(false),
        heatmap_bins: Some(64),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
    };

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let t0 = Instant::now();
    let res = run_double_ma_optimization_blocking_with_candles(req, candles, &cancel, None)?;
    let wall_ms = t0.elapsed().as_millis() as u64;

    Ok(RouteMatrixCase {
        case_label: spec.case_label.to_string(),
        requested_backend: requested_backend_label(&spec.backend).to_string(),
        backend_used: backend_used_label(res.backend_used).to_string(),
        backend_route: res.backend_route,
        backend_fallback_reason: res.backend_fallback_reason,
        runtime_ms: res.runtime_ms,
        wall_ms,
        best_fast_len: res.best_params.fast_len,
        best_slow_len: res.best_params.slow_len,
        best_sharpe: res.best_metrics.sharpe,
        best_pnl: res.best_metrics.pnl,
    })
}

fn main() -> Result<(), String> {
    let csv_path = parse_csv();
    let timeframe = parse_timeframe();
    let out_path = parse_out_path(&timeframe);
    let device_id = parse_device_id();
    let candles = load_candles(&csv_path, &timeframe)?;

    let mut cases = Vec::new();
    for spec in case_specs(device_id) {
        let case = run_case(&spec, &candles)?;
        println!(
            "{} requested={} backend_used={} route={} fallback={} runtime_ms={} wall_ms={}",
            case.case_label,
            case.requested_backend,
            case.backend_used,
            case.backend_route.as_deref().unwrap_or("<none>"),
            case.backend_fallback_reason.as_deref().unwrap_or("<none>"),
            case.runtime_ms,
            case.wall_ms
        );
        cases.push(case);
    }

    let artifact = RouteMatrixArtifact {
        csv_path: csv_path.to_string_lossy().to_string(),
        timeframe,
        total_bars: candles.close.len(),
        cases,
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&out_path, json).map_err(|e| e.to_string())?;
    println!(
        "Wrote route/runtime artifact to {}",
        out_path.to_string_lossy()
    );
    Ok(())
}
