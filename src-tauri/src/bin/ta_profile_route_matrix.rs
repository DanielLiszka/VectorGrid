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

#[derive(Clone, Debug, Serialize)]
struct EnvironmentFingerprint {
    hostname: String,
    os: String,
    arch: String,
    family: String,
}

#[derive(Serialize)]
struct RouteMatrixArtifact {
    schema_version: u32,
    timing_boundary: &'static str,
    timing_boundary_note: &'static str,
    build_profile: &'static str,
    environment: EnvironmentFingerprint,
    csv_path: String,
    timeframe: String,
    total_bars: usize,
    cases: Vec<RouteMatrixCase>,
}

#[derive(Clone, Debug, Serialize)]
struct RouteMatrixRun {
    phase: &'static str,
    run_index: usize,
    backend_used: String,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    route_ok: bool,
    runtime_ms: u64,
    wall_ms: u64,
    best_fast_len: u16,
    best_slow_len: u16,
    best_sharpe: f64,
    best_pnl: f64,
}

#[derive(Serialize)]
struct RouteMatrixCase {
    case_label: String,
    requested_backend: String,
    expected_backend_route: Option<String>,
    backend_used: String,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    route_ok: bool,
    runtime_ms: u64,
    wall_ms: u64,
    best_fast_len: u16,
    best_slow_len: u16,
    best_sharpe: f64,
    best_pnl: f64,
    warmup_runs: usize,
    measured_runs: usize,
    canonical_measured_run_index: usize,
    all_measured_routes_ok: bool,
    runtime_ms_min: u64,
    runtime_ms_median: f64,
    runtime_ms_max: u64,
    wall_ms_min: u64,
    wall_ms_median: f64,
    wall_ms_max: u64,
    runs: Vec<RouteMatrixRun>,
}

struct CaseSpec {
    case_label: &'static str,
    strategy: StrategyConfig,
    backend: Backend,
}

const fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
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

fn parse_usize_flag(name: &str, default: usize) -> usize {
    parse_flag(name)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_strict() -> bool {
    std::env::args().any(|arg| arg == "--strict")
}

fn parse_warmup_runs() -> usize {
    parse_usize_flag("--warmup-runs", 0)
}

fn parse_measured_runs() -> usize {
    parse_usize_flag("--measured-runs", 1).max(1)
}

fn environment_fingerprint() -> EnvironmentFingerprint {
    EnvironmentFingerprint {
        hostname: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_default(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        family: std::env::consts::FAMILY.to_string(),
    }
}

fn median_u64_as_f64(values: &mut [u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] as f64 + values[mid] as f64) * 0.5
    } else {
        values[mid] as f64
    }
}

fn canonical_run_index_for_wall_ms(values: &[u64]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    let median = median_u64_as_f64(sorted.as_mut_slice());
    values
        .iter()
        .enumerate()
        .min_by(|(idx_a, wall_a), (idx_b, wall_b)| {
            let delta_a = ((**wall_a as f64) - median).abs();
            let delta_b = ((**wall_b as f64) - median).abs();
            delta_a
                .total_cmp(&delta_b)
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
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

fn expected_backend_route(backend: &Backend) -> Option<String> {
    match backend {
        Backend::CpuOnly => Some("CpuOnly".to_string()),
        Backend::GpuKernel { .. } => Some("GpuKernel".to_string()),
        Backend::GpuHybridKernel { .. } => Some("GpuHybridKernel".to_string()),
        Backend::GpuOnly { .. } | Backend::Auto { .. } => None,
    }
}

fn route_ok(
    expected_backend_route: Option<&str>,
    backend_used: BackendUsed,
    backend_route: Option<&str>,
    backend_fallback_reason: Option<&str>,
) -> bool {
    match expected_backend_route {
        Some("CpuOnly") => {
            backend_fallback_reason.is_none() && matches!(backend_used, BackendUsed::Cpu)
        }
        Some(expected) => {
            backend_fallback_reason.is_none()
                && matches!(backend_used, BackendUsed::Gpu)
                && backend_route == Some(expected)
        }
        None => true,
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

fn execute_case_run(
    spec: &CaseSpec,
    candles: &Candles,
    phase: &'static str,
    run_index: usize,
) -> Result<RouteMatrixRun, String> {
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
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(20),
        include_all: Some(false),
        heatmap_bins: Some(64),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,

        hurdle_tau: None,
    };

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let t0 = Instant::now();
    let res = run_double_ma_optimization_blocking_with_candles(req, candles, &cancel, None)?;
    let wall_ms = t0.elapsed().as_millis() as u64;
    let backend_used = res.backend_used.clone();
    let expected_backend_route = expected_backend_route(&spec.backend);
    let route_ok = route_ok(
        expected_backend_route.as_deref(),
        backend_used.clone(),
        res.backend_route.as_deref(),
        res.backend_fallback_reason.as_deref(),
    );

    Ok(RouteMatrixRun {
        phase,
        run_index,
        backend_used: backend_used_label(backend_used).to_string(),
        backend_route: res.backend_route,
        backend_fallback_reason: res.backend_fallback_reason,
        route_ok,
        runtime_ms: res.runtime_ms,
        wall_ms,
        best_fast_len: res.best_params.fast_len,
        best_slow_len: res.best_params.slow_len,
        best_sharpe: res.best_metrics.sharpe,
        best_pnl: res.best_metrics.pnl,
    })
}

fn run_case(
    spec: &CaseSpec,
    candles: &Candles,
    warmup_runs: usize,
    measured_runs: usize,
) -> Result<RouteMatrixCase, String> {
    let expected_backend_route = expected_backend_route(&spec.backend);
    let mut runs = Vec::with_capacity(warmup_runs + measured_runs);
    for run_index in 0..warmup_runs {
        runs.push(execute_case_run(spec, candles, "warmup", run_index + 1)?);
    }
    let mut measured = Vec::with_capacity(measured_runs);
    for run_index in 0..measured_runs {
        let run = execute_case_run(spec, candles, "measured", run_index + 1)?;
        runs.push(run.clone());
        measured.push(run);
    }
    let canonical_index = canonical_run_index_for_wall_ms(
        &measured.iter().map(|run| run.wall_ms).collect::<Vec<_>>(),
    );
    let canonical = &measured[canonical_index];
    let mut runtime_values = measured.iter().map(|run| run.runtime_ms).collect::<Vec<_>>();
    let mut wall_values = measured.iter().map(|run| run.wall_ms).collect::<Vec<_>>();

    Ok(RouteMatrixCase {
        case_label: spec.case_label.to_string(),
        requested_backend: requested_backend_label(&spec.backend).to_string(),
        expected_backend_route,
        backend_used: canonical.backend_used.clone(),
        backend_route: canonical.backend_route.clone(),
        backend_fallback_reason: canonical.backend_fallback_reason.clone(),
        route_ok: canonical.route_ok,
        runtime_ms: canonical.runtime_ms,
        wall_ms: canonical.wall_ms,
        best_fast_len: canonical.best_fast_len,
        best_slow_len: canonical.best_slow_len,
        best_sharpe: canonical.best_sharpe,
        best_pnl: canonical.best_pnl,
        warmup_runs,
        measured_runs,
        canonical_measured_run_index: canonical.run_index,
        all_measured_routes_ok: measured.iter().all(|run| run.route_ok),
        runtime_ms_min: runtime_values.iter().copied().min().unwrap_or(0),
        runtime_ms_median: median_u64_as_f64(runtime_values.as_mut_slice()),
        runtime_ms_max: runtime_values.iter().copied().max().unwrap_or(0),
        wall_ms_min: wall_values.iter().copied().min().unwrap_or(0),
        wall_ms_median: median_u64_as_f64(wall_values.as_mut_slice()),
        wall_ms_max: wall_values.iter().copied().max().unwrap_or(0),
        runs,
    })
}

fn main() -> Result<(), String> {
    let csv_path = parse_csv();
    let timeframe = parse_timeframe();
    let out_path = parse_out_path(&timeframe);
    let device_id = parse_device_id();
    let warmup_runs = parse_warmup_runs();
    let measured_runs = parse_measured_runs();
    let strict = parse_strict();
    let candles = load_candles(&csv_path, &timeframe)?;

    let mut cases = Vec::new();
    for spec in case_specs(device_id) {
        let case = run_case(&spec, &candles, warmup_runs, measured_runs)?;
        println!(
            "{} requested={} expected={} backend_used={} route={} fallback={} route_ok={} runtime_ms={} wall_ms={} median_wall_ms={} measured_runs={}",
            case.case_label,
            case.requested_backend,
            case.expected_backend_route.as_deref().unwrap_or("<none>"),
            case.backend_used,
            case.backend_route.as_deref().unwrap_or("<none>"),
            case.backend_fallback_reason.as_deref().unwrap_or("<none>"),
            if case.route_ok { "ok" } else { "mismatch" },
            case.runtime_ms,
            case.wall_ms,
            case.wall_ms_median,
            case.measured_runs,
        );
        cases.push(case);
    }

    let artifact = RouteMatrixArtifact {
        schema_version: 1,
        timing_boundary: "route-matrix",
        timing_boundary_note:
            "blocking optimizer route audit per case; top-level case fields reflect the canonical measured run and repeated-run summaries are additive",
        build_profile: build_profile(),
        environment: environment_fingerprint(),
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
    if strict {
        let mismatches: Vec<&RouteMatrixCase> = artifact
            .cases
            .iter()
            .filter(|case| !case.all_measured_routes_ok)
            .collect();
        if !mismatches.is_empty() {
            let labels = mismatches
                .iter()
                .map(|case| {
                    format!(
                        "{} expected={} actual={} fallback={} measured_runs={} canonical_run={}",
                        case.case_label,
                        case.expected_backend_route.as_deref().unwrap_or("<none>"),
                        case.backend_route.as_deref().unwrap_or("<none>"),
                        case.backend_fallback_reason.as_deref().unwrap_or("<none>"),
                        case.measured_runs,
                        case.canonical_measured_run_index
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!("strict route audit failed: {labels}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{canonical_run_index_for_wall_ms, route_ok};
    use ta_app_core::BackendUsed;

    #[test]
    fn route_ok_allows_cpu_without_route_label() {
        assert!(route_ok(Some("CpuOnly"), BackendUsed::Cpu, None, None));
        assert!(!route_ok(
            Some("GpuKernel"),
            BackendUsed::Gpu,
            Some("GpuKernel->CpuOnly"),
            Some("fallback")
        ));
    }

    #[test]
    fn canonical_run_index_prefers_median_wall_run() {
        assert_eq!(canonical_run_index_for_wall_ms(&[90, 100, 110]), 1);
        assert_eq!(canonical_run_index_for_wall_ms(&[90, 110]), 0);
    }
}
