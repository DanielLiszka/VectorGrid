use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use ta_app_core::{Backend, DoubleMaRequest};
use ta_desktop_demo_app::state::AppState;
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;

#[derive(Debug, Serialize)]
struct EnvironmentFingerprint {
    hostname: String,
    os: String,
    arch: String,
    family: String,
}

#[derive(Clone, Copy, Debug)]
struct U32Range {
    start: u32,
    end: u32,
    step: u32,
}

impl FromStr for U32Range {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                let v = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start: v,
                    end: v,
                    step: 0,
                })
            }
            2 => {
                let start = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                let end = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start,
                    end,
                    step: 1,
                })
            }
            3 => {
                let start = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                let end = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
                let step = parts[2].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self { start, end, step })
            }
            _ => Err("range must be N or start:end or start:end:step".to_string()),
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
enum BackendArg {
    Cpu,
    GpuKernel,
    GpuHybridKernel,
    Auto,
}

#[derive(ValueEnum, Clone, Debug)]
enum ObjectiveArg {
    Pnl,
    Sharpe,
    MaxDrawdown,
}

#[derive(Debug, Serialize)]
struct RangeArtifact {
    start: u32,
    end: u32,
    step: u32,
    spec: String,
    count: u64,
}

#[derive(Debug, Serialize)]
struct MetricContractArtifact {
    request: &'static str,
    include_all: bool,
    include_trade_metrics: bool,
    expected_contract: &'static str,
    expected_metrics_count: usize,
}

#[derive(Debug, Serialize)]
struct RequestArtifact {
    objective: &'static str,
    backend_request: String,
    fast_ma: String,
    slow_ma: String,
    ma_source: String,
    fast_range: RangeArtifact,
    slow_range: RangeArtifact,
    pair_count: u64,
    top_k: usize,
    heatmap_bins: u16,
    long_only: bool,
    allow_flip: bool,
    metric_contract: MetricContractArtifact,
}

#[derive(Parser, Debug)]
#[command(name = "ta_app_path_faceoff")]
struct Cli {
    #[arg(long)]
    csv: PathBuf,

    #[arg(long)]
    output_json: PathBuf,

    #[arg(long)]
    cache_dir: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = BackendArg::GpuKernel)]
    backend: BackendArg,

    #[arg(long, default_value = "alma")]
    fast_ma: String,

    #[arg(long, default_value = "alma")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long)]
    timeframe: Option<String>,

    #[arg(long, default_value = "5:200:1")]
    fast_period: U32Range,

    #[arg(long, default_value = "10:400:1")]
    slow_period: U32Range,

    #[arg(long, value_enum, default_value_t = ObjectiveArg::Sharpe)]
    objective: ObjectiveArg,

    #[arg(long, default_value_t = 10)]
    top_k: usize,

    #[arg(long, default_value_t = 0)]
    heatmap_bins: u16,

    #[arg(long, default_value_t = 0.85)]
    alma_offset: f64,

    #[arg(long, default_value_t = 6.0)]
    alma_sigma: f64,

    #[arg(long, default_value_t = 1)]
    warmup_trials: usize,

    #[arg(long, default_value_t = 5)]
    measured_trials: usize,

    #[arg(long, default_value_t = 0)]
    cold_state_trials: usize,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    long_only: bool,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    allow_flip: bool,
}

#[derive(Debug, Serialize)]
struct TrialTimingArtifact {
    total_wall_ms: f64,
    harness_wall_ms: f64,
    state_lookup_ms: f64,
    engine_wall_ms: f64,
    engine_runtime_ms: u64,
    dataset_load_ms: Option<f64>,
    dataset_resample_ms: Option<f64>,
    gpu_kernel_route_wall_ms: Option<u64>,
    gpu_kernel_gpu_indicator_prep_ms: Option<u64>,
    gpu_kernel_h2d_upload_ms: Option<u64>,
    gpu_kernel_launch_submit_us: Option<u64>,
    gpu_kernel_d2h_copy_ms: Option<u64>,
    gpu_kernel_host_decode_ms: Option<u64>,
    hybrid_planner_ms: Option<u64>,
    hybrid_cpu_indicator_prep_ms: Option<u64>,
    hybrid_gpu_indicator_prep_ms: Option<u64>,
    hybrid_h2d_upload_ms: Option<u64>,
    hybrid_kernel_eval_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct TrialArtifact {
    trial_index: usize,
    runtime_ms_wall: f64,
    timing: TrialTimingArtifact,
    result: ta_app_core::BackendOptimizationResult,
}

#[derive(Debug, Serialize)]
struct TrialTimingSummaryArtifact {
    count: usize,
    total_wall_ms_min: f64,
    total_wall_ms_median: f64,
    total_wall_ms_max: f64,
    state_lookup_ms_min: f64,
    state_lookup_ms_median: f64,
    state_lookup_ms_max: f64,
    harness_wall_ms_min: f64,
    harness_wall_ms_median: f64,
    harness_wall_ms_max: f64,
    engine_wall_ms_min: f64,
    engine_wall_ms_median: f64,
    engine_wall_ms_max: f64,
    engine_runtime_ms_min: u64,
    engine_runtime_ms_median: f64,
    engine_runtime_ms_max: u64,
    dataset_load_ms_min: Option<f64>,
    dataset_load_ms_median: Option<f64>,
    dataset_load_ms_max: Option<f64>,
    dataset_resample_ms_min: Option<f64>,
    dataset_resample_ms_median: Option<f64>,
    dataset_resample_ms_max: Option<f64>,
    gpu_kernel_route_wall_ms_min: Option<u64>,
    gpu_kernel_route_wall_ms_median: Option<f64>,
    gpu_kernel_route_wall_ms_max: Option<u64>,
    gpu_kernel_gpu_indicator_prep_ms_min: Option<u64>,
    gpu_kernel_gpu_indicator_prep_ms_median: Option<f64>,
    gpu_kernel_gpu_indicator_prep_ms_max: Option<u64>,
    gpu_kernel_h2d_upload_ms_min: Option<u64>,
    gpu_kernel_h2d_upload_ms_median: Option<f64>,
    gpu_kernel_h2d_upload_ms_max: Option<u64>,
    gpu_kernel_launch_submit_us_min: Option<u64>,
    gpu_kernel_launch_submit_us_median: Option<f64>,
    gpu_kernel_launch_submit_us_max: Option<u64>,
    gpu_kernel_d2h_copy_ms_min: Option<u64>,
    gpu_kernel_d2h_copy_ms_median: Option<f64>,
    gpu_kernel_d2h_copy_ms_max: Option<u64>,
    gpu_kernel_host_decode_ms_min: Option<u64>,
    gpu_kernel_host_decode_ms_median: Option<f64>,
    gpu_kernel_host_decode_ms_max: Option<u64>,
    hybrid_planner_ms_min: Option<u64>,
    hybrid_planner_ms_median: Option<f64>,
    hybrid_planner_ms_max: Option<u64>,
    hybrid_cpu_indicator_prep_ms_min: Option<u64>,
    hybrid_cpu_indicator_prep_ms_median: Option<f64>,
    hybrid_cpu_indicator_prep_ms_max: Option<u64>,
    hybrid_gpu_indicator_prep_ms_min: Option<u64>,
    hybrid_gpu_indicator_prep_ms_median: Option<f64>,
    hybrid_gpu_indicator_prep_ms_max: Option<u64>,
    hybrid_h2d_upload_ms_min: Option<u64>,
    hybrid_h2d_upload_ms_median: Option<f64>,
    hybrid_h2d_upload_ms_max: Option<u64>,
    hybrid_kernel_eval_ms_min: Option<u64>,
    hybrid_kernel_eval_ms_median: Option<f64>,
    hybrid_kernel_eval_ms_max: Option<u64>,
}

#[derive(Debug, Serialize)]
struct FaceoffTimingSummaryArtifact {
    warmup: Option<TrialTimingSummaryArtifact>,
    measured: Option<TrialTimingSummaryArtifact>,
    cold_state: Option<TrialTimingSummaryArtifact>,
}

#[derive(Debug, Serialize)]
struct RouteSummaryArtifact {
    backend_used_counts: BTreeMap<String, usize>,
    backend_route_counts: BTreeMap<String, usize>,
    fallback_count: usize,
}

#[derive(Debug, Serialize)]
struct FaceoffDatasetArtifact {
    csv_path: String,
    cache_dir: String,
    load_ms: f64,
    resample_ms: Option<f64>,
    data_id: String,
    version: u64,
    bars: usize,
}

#[derive(Debug, Serialize)]
struct AppPathFaceoffArtifact {
    schema_version: u32,
    tool: &'static str,
    mode: &'static str,
    timing_boundary: &'static str,
    timing_boundary_note: &'static str,
    cold_state_boundary: Option<&'static str>,
    cold_state_boundary_note: Option<&'static str>,
    build_profile: &'static str,
    environment: EnvironmentFingerprint,
    requested_backend: String,
    backend: String,
    request: RequestArtifact,
    dataset: FaceoffDatasetArtifact,
    route_summary: RouteSummaryArtifact,
    timing_summary: FaceoffTimingSummaryArtifact,
    warmup_trials: Vec<TrialArtifact>,
    measured_trials: Vec<TrialArtifact>,
    cold_state_trials: Vec<TrialArtifact>,
    generated_at_utc: String,
}

fn median_f64(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) * 0.5
    } else {
        values[mid]
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

fn summarize_optional_u64(
    values: impl IntoIterator<Item = Option<u64>>,
) -> (Option<u64>, Option<f64>, Option<u64>) {
    let mut present = values.into_iter().flatten().collect::<Vec<_>>();
    if present.is_empty() {
        return (None, None, None);
    }
    let min = present.iter().copied().min();
    let max = present.iter().copied().max();
    let median = Some(median_u64_as_f64(present.as_mut_slice()));
    (min, median, max)
}

fn summarize_optional_f64(
    values: impl IntoIterator<Item = Option<f64>>,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let mut present = values.into_iter().flatten().collect::<Vec<_>>();
    if present.is_empty() {
        return (None, None, None);
    }
    let min = present.iter().copied().reduce(f64::min);
    let max = present.iter().copied().reduce(f64::max);
    let median = Some(median_f64(present.as_mut_slice()));
    (min, median, max)
}

fn summarize_trials(trials: &[TrialArtifact]) -> Option<TrialTimingSummaryArtifact> {
    if trials.is_empty() {
        return None;
    }

    let mut total_wall_ms: Vec<f64> = trials
        .iter()
        .map(|trial| trial.timing.total_wall_ms)
        .collect();
    let mut state_lookup_ms: Vec<f64> = trials
        .iter()
        .map(|trial| trial.timing.state_lookup_ms)
        .collect();
    let mut harness_wall_ms: Vec<f64> = trials
        .iter()
        .map(|trial| trial.timing.harness_wall_ms)
        .collect();
    let mut engine_wall_ms: Vec<f64> = trials
        .iter()
        .map(|trial| trial.timing.engine_wall_ms)
        .collect();
    let mut engine_runtime_ms: Vec<u64> = trials
        .iter()
        .map(|trial| trial.timing.engine_runtime_ms)
        .collect();

    let total_wall_ms_min = total_wall_ms.iter().copied().fold(f64::INFINITY, f64::min);
    let total_wall_ms_max = total_wall_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let state_lookup_ms_min = state_lookup_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let state_lookup_ms_max = state_lookup_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let harness_wall_ms_min = harness_wall_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let harness_wall_ms_max = harness_wall_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let engine_wall_ms_min = engine_wall_ms.iter().copied().fold(f64::INFINITY, f64::min);
    let engine_wall_ms_max = engine_wall_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let engine_runtime_ms_min = engine_runtime_ms.iter().copied().min().unwrap_or(0);
    let engine_runtime_ms_max = engine_runtime_ms.iter().copied().max().unwrap_or(0);
    let (dataset_load_ms_min, dataset_load_ms_median, dataset_load_ms_max) =
        summarize_optional_f64(trials.iter().map(|trial| trial.timing.dataset_load_ms));
    let (dataset_resample_ms_min, dataset_resample_ms_median, dataset_resample_ms_max) =
        summarize_optional_f64(trials.iter().map(|trial| trial.timing.dataset_resample_ms));
    let (
        gpu_kernel_route_wall_ms_min,
        gpu_kernel_route_wall_ms_median,
        gpu_kernel_route_wall_ms_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.gpu_kernel_route_wall_ms),
    );
    let (
        gpu_kernel_gpu_indicator_prep_ms_min,
        gpu_kernel_gpu_indicator_prep_ms_median,
        gpu_kernel_gpu_indicator_prep_ms_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.gpu_kernel_gpu_indicator_prep_ms),
    );
    let (
        gpu_kernel_h2d_upload_ms_min,
        gpu_kernel_h2d_upload_ms_median,
        gpu_kernel_h2d_upload_ms_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.gpu_kernel_h2d_upload_ms),
    );
    let (
        gpu_kernel_launch_submit_us_min,
        gpu_kernel_launch_submit_us_median,
        gpu_kernel_launch_submit_us_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.gpu_kernel_launch_submit_us),
    );
    let (gpu_kernel_d2h_copy_ms_min, gpu_kernel_d2h_copy_ms_median, gpu_kernel_d2h_copy_ms_max) =
        summarize_optional_u64(
            trials
                .iter()
                .map(|trial| trial.timing.gpu_kernel_d2h_copy_ms),
        );
    let (
        gpu_kernel_host_decode_ms_min,
        gpu_kernel_host_decode_ms_median,
        gpu_kernel_host_decode_ms_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.gpu_kernel_host_decode_ms),
    );
    let (hybrid_planner_ms_min, hybrid_planner_ms_median, hybrid_planner_ms_max) =
        summarize_optional_u64(trials.iter().map(|trial| trial.timing.hybrid_planner_ms));
    let (
        hybrid_cpu_indicator_prep_ms_min,
        hybrid_cpu_indicator_prep_ms_median,
        hybrid_cpu_indicator_prep_ms_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.hybrid_cpu_indicator_prep_ms),
    );
    let (
        hybrid_gpu_indicator_prep_ms_min,
        hybrid_gpu_indicator_prep_ms_median,
        hybrid_gpu_indicator_prep_ms_max,
    ) = summarize_optional_u64(
        trials
            .iter()
            .map(|trial| trial.timing.hybrid_gpu_indicator_prep_ms),
    );
    let (hybrid_h2d_upload_ms_min, hybrid_h2d_upload_ms_median, hybrid_h2d_upload_ms_max) =
        summarize_optional_u64(trials.iter().map(|trial| trial.timing.hybrid_h2d_upload_ms));
    let (hybrid_kernel_eval_ms_min, hybrid_kernel_eval_ms_median, hybrid_kernel_eval_ms_max) =
        summarize_optional_u64(
            trials
                .iter()
                .map(|trial| trial.timing.hybrid_kernel_eval_ms),
        );

    Some(TrialTimingSummaryArtifact {
        count: trials.len(),
        total_wall_ms_min,
        total_wall_ms_median: median_f64(total_wall_ms.as_mut_slice()),
        total_wall_ms_max,
        state_lookup_ms_min,
        state_lookup_ms_median: median_f64(state_lookup_ms.as_mut_slice()),
        state_lookup_ms_max,
        harness_wall_ms_min,
        harness_wall_ms_median: median_f64(harness_wall_ms.as_mut_slice()),
        harness_wall_ms_max,
        engine_wall_ms_min,
        engine_wall_ms_median: median_f64(engine_wall_ms.as_mut_slice()),
        engine_wall_ms_max,
        engine_runtime_ms_min,
        engine_runtime_ms_median: median_u64_as_f64(engine_runtime_ms.as_mut_slice()),
        engine_runtime_ms_max,
        dataset_load_ms_min,
        dataset_load_ms_median,
        dataset_load_ms_max,
        dataset_resample_ms_min,
        dataset_resample_ms_median,
        dataset_resample_ms_max,
        gpu_kernel_route_wall_ms_min,
        gpu_kernel_route_wall_ms_median,
        gpu_kernel_route_wall_ms_max,
        gpu_kernel_gpu_indicator_prep_ms_min,
        gpu_kernel_gpu_indicator_prep_ms_median,
        gpu_kernel_gpu_indicator_prep_ms_max,
        gpu_kernel_h2d_upload_ms_min,
        gpu_kernel_h2d_upload_ms_median,
        gpu_kernel_h2d_upload_ms_max,
        gpu_kernel_launch_submit_us_min,
        gpu_kernel_launch_submit_us_median,
        gpu_kernel_launch_submit_us_max,
        gpu_kernel_d2h_copy_ms_min,
        gpu_kernel_d2h_copy_ms_median,
        gpu_kernel_d2h_copy_ms_max,
        gpu_kernel_host_decode_ms_min,
        gpu_kernel_host_decode_ms_median,
        gpu_kernel_host_decode_ms_max,
        hybrid_planner_ms_min,
        hybrid_planner_ms_median,
        hybrid_planner_ms_max,
        hybrid_cpu_indicator_prep_ms_min,
        hybrid_cpu_indicator_prep_ms_median,
        hybrid_cpu_indicator_prep_ms_max,
        hybrid_gpu_indicator_prep_ms_min,
        hybrid_gpu_indicator_prep_ms_median,
        hybrid_gpu_indicator_prep_ms_max,
        hybrid_h2d_upload_ms_min,
        hybrid_h2d_upload_ms_median,
        hybrid_h2d_upload_ms_max,
        hybrid_kernel_eval_ms_min,
        hybrid_kernel_eval_ms_median,
        hybrid_kernel_eval_ms_max,
    })
}

fn summarize_routes(trials: &[TrialArtifact]) -> RouteSummaryArtifact {
    let mut backend_used_counts = BTreeMap::new();
    let mut backend_route_counts = BTreeMap::new();
    let mut fallback_count = 0usize;

    for trial in trials {
        let backend_used = match trial.result.backend_used {
            ta_app_core::BackendUsed::Cpu => "Cpu".to_string(),
            ta_app_core::BackendUsed::Gpu => "Gpu".to_string(),
        };
        *backend_used_counts.entry(backend_used).or_insert(0) += 1;

        let backend_route = trial
            .result
            .backend_route
            .clone()
            .unwrap_or_else(|| "<none>".to_string());
        *backend_route_counts.entry(backend_route).or_insert(0) += 1;

        if trial.result.backend_fallback_reason.is_some() {
            fallback_count = fallback_count.saturating_add(1);
        }
    }

    RouteSummaryArtifact {
        backend_used_counts,
        backend_route_counts,
        fallback_count,
    }
}

const fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
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

fn backend_from_arg(arg: BackendArg) -> Backend {
    match arg {
        BackendArg::Cpu => Backend::CpuOnly,
        BackendArg::GpuKernel => Backend::GpuKernel { device_id: 0 },
        BackendArg::GpuHybridKernel => Backend::GpuHybridKernel { device_id: 0 },
        BackendArg::Auto => Backend::Auto { device_id: 0 },
    }
}

fn objective_from_arg(arg: ObjectiveArg) -> ObjectiveKind {
    match arg {
        ObjectiveArg::Pnl => ObjectiveKind::Pnl,
        ObjectiveArg::Sharpe => ObjectiveKind::Sharpe,
        ObjectiveArg::MaxDrawdown => ObjectiveKind::MaxDrawdown,
    }
}

fn objective_name(arg: ObjectiveArg) -> &'static str {
    match arg {
        ObjectiveArg::Pnl => "pnl",
        ObjectiveArg::Sharpe => "sharpe",
        ObjectiveArg::MaxDrawdown => "max_drawdown",
    }
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

fn expected_metric_contract(arg: ObjectiveArg) -> (&'static str, usize) {
    match arg {
        ObjectiveArg::Pnl => ("auto_scalar_1", 1),
        ObjectiveArg::Sharpe => ("auto_product_reduced_10", 10),
        ObjectiveArg::MaxDrawdown => ("auto_drawdown_3", 3),
    }
}

fn range_count(range: U32Range) -> u64 {
    if range.step == 0 || range.end <= range.start {
        return 1;
    }
    (((range.end - range.start) / range.step) + 1) as u64
}

fn range_spec(range: U32Range) -> String {
    format!("{}:{}:{}", range.start, range.end, range.step)
}

fn valid_pair_count(fast: U32Range, slow: U32Range) -> u64 {
    let mut total = 0_u64;
    let mut fast_len = fast.start;
    while fast_len <= fast.end {
        let mut slow_len = slow.start;
        while slow_len <= slow.end {
            if fast_len < slow_len {
                total += 1;
            }
            if slow.step == 0 {
                break;
            }
            slow_len = slow_len.saturating_add(slow.step);
            if slow_len == 0 {
                break;
            }
        }
        if fast.step == 0 {
            break;
        }
        fast_len = fast_len.saturating_add(fast.step);
        if fast_len == 0 {
            break;
        }
    }
    total
}

fn alma_params(offset: f64, sigma: f64, ma_name: &str) -> Option<HashMap<String, f64>> {
    if !ma_name.trim().eq_ignore_ascii_case("alma") {
        return None;
    }
    let mut params = HashMap::new();
    params.insert("offset".to_string(), offset);
    params.insert("sigma".to_string(), sigma);
    Some(params)
}

fn make_request(cli: &Cli, data_id: &str) -> DoubleMaRequest {
    let mut strategy = StrategyConfig::default();
    strategy.long_only = cli.long_only;
    strategy.allow_flip = cli.allow_flip;

    DoubleMaRequest {
        backend: backend_from_arg(cli.backend.clone()),
        data_id: data_id.to_string(),
        fast_range: (
            cli.fast_period.start,
            cli.fast_period.end,
            cli.fast_period.step,
        ),
        slow_range: (
            cli.slow_period.start,
            cli.slow_period.end,
            cli.slow_period.step,
        ),
        fast_ma_types: vec![cli.fast_ma.clone()],
        slow_ma_types: vec![cli.slow_ma.clone()],
        ma_source: cli.ma_source.clone(),
        export_all_csv_path: None,
        fast_ma_params: alma_params(cli.alma_offset, cli.alma_sigma, &cli.fast_ma),
        slow_ma_params: alma_params(cli.alma_offset, cli.alma_sigma, &cli.slow_ma),
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter: None,
        objective: objective_from_arg(cli.objective.clone()),
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(cli.top_k),
        include_all: Some(false),
        heatmap_bins: Some(cli.heatmap_bins),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,

        hurdle_tau: None,
    }
}

fn run_trial(
    req: &DoubleMaRequest,
    state: &AppState,
    data_id: &str,
    trial_index: usize,
) -> Result<TrialArtifact, String> {
    let total_started = Instant::now();
    let lookup_started = Instant::now();
    let candles = state.get_candles(data_id)?;
    let state_lookup_ms = lookup_started.elapsed().as_secs_f64() * 1000.0;
    let cancel = AtomicBool::new(false);
    let engine_started = Instant::now();
    let result = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req.clone(),
        candles.as_ref(),
        &cancel,
        None,
    )?;
    let engine_wall_ms = engine_started.elapsed().as_secs_f64() * 1000.0;
    let total_wall_ms = total_started.elapsed().as_secs_f64() * 1000.0;
    let timing = TrialTimingArtifact {
        total_wall_ms,
        harness_wall_ms: total_wall_ms,
        state_lookup_ms,
        engine_wall_ms,
        engine_runtime_ms: result.runtime_ms,
        dataset_load_ms: None,
        dataset_resample_ms: None,
        gpu_kernel_route_wall_ms: result
            .gpu_kernel_timing_ms
            .as_ref()
            .map(|t| t.route_wall_ms),
        gpu_kernel_gpu_indicator_prep_ms: result
            .gpu_kernel_timing_ms
            .as_ref()
            .and_then(|t| t.gpu_indicator_prep_ms),
        gpu_kernel_h2d_upload_ms: result
            .gpu_kernel_timing_ms
            .as_ref()
            .and_then(|t| t.h2d_upload_ms),
        gpu_kernel_launch_submit_us: result
            .gpu_kernel_timing_ms
            .as_ref()
            .and_then(|t| t.launch_submit_us),
        gpu_kernel_d2h_copy_ms: result
            .gpu_kernel_timing_ms
            .as_ref()
            .and_then(|t| t.d2h_copy_ms),
        gpu_kernel_host_decode_ms: result
            .gpu_kernel_timing_ms
            .as_ref()
            .and_then(|t| t.host_decode_ms),
        hybrid_planner_ms: result.hybrid_timing_ms.as_ref().map(|t| t.planner_ms),
        hybrid_cpu_indicator_prep_ms: result
            .hybrid_timing_ms
            .as_ref()
            .map(|t| t.cpu_indicator_prep_ms),
        hybrid_gpu_indicator_prep_ms: result
            .hybrid_timing_ms
            .as_ref()
            .map(|t| t.gpu_indicator_prep_ms),
        hybrid_h2d_upload_ms: result.hybrid_timing_ms.as_ref().map(|t| t.h2d_upload_ms),
        hybrid_kernel_eval_ms: result.hybrid_timing_ms.as_ref().map(|t| t.kernel_eval_ms),
    };
    Ok(TrialArtifact {
        trial_index,
        runtime_ms_wall: total_wall_ms,
        timing,
        result,
    })
}

fn run_cold_state_trial(
    cli: &Cli,
    cache_dir: &std::path::Path,
    trial_index: usize,
) -> Result<TrialArtifact, String> {
    let total_started = Instant::now();
    let state = AppState::default();
    let load_started = Instant::now();
    let load = state.load_price_data(
        cli.csv
            .to_str()
            .ok_or_else(|| "invalid csv path".to_string())?,
        cache_dir,
    )?;
    let dataset_load_ms = load_started.elapsed().as_secs_f64() * 1000.0;
    let (data_id, dataset_resample_ms) = if let Some(raw_tf) = cli.timeframe.as_deref() {
        let started = Instant::now();
        let derived = state.resample_loaded_data(&load.id, raw_tf)?;
        (derived.id, Some(started.elapsed().as_secs_f64() * 1000.0))
    } else {
        (load.id.clone(), None)
    };
    let req = make_request(cli, &data_id);
    let lookup_started = Instant::now();
    let candles = state.get_candles(&data_id)?;
    let state_lookup_ms = lookup_started.elapsed().as_secs_f64() * 1000.0;
    let cancel = AtomicBool::new(false);
    let engine_started = Instant::now();
    let result = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req,
        candles.as_ref(),
        &cancel,
        None,
    )?;
    let engine_wall_ms = engine_started.elapsed().as_secs_f64() * 1000.0;
    let total_wall_ms = total_started.elapsed().as_secs_f64() * 1000.0;
    Ok(TrialArtifact {
        trial_index,
        runtime_ms_wall: total_wall_ms,
        timing: TrialTimingArtifact {
            total_wall_ms,
            harness_wall_ms: total_wall_ms,
            state_lookup_ms,
            engine_wall_ms,
            engine_runtime_ms: result.runtime_ms,
            dataset_load_ms: Some(dataset_load_ms),
            dataset_resample_ms,
            gpu_kernel_route_wall_ms: result
                .gpu_kernel_timing_ms
                .as_ref()
                .map(|t| t.route_wall_ms),
            gpu_kernel_gpu_indicator_prep_ms: result
                .gpu_kernel_timing_ms
                .as_ref()
                .and_then(|t| t.gpu_indicator_prep_ms),
            gpu_kernel_h2d_upload_ms: result
                .gpu_kernel_timing_ms
                .as_ref()
                .and_then(|t| t.h2d_upload_ms),
            gpu_kernel_launch_submit_us: result
                .gpu_kernel_timing_ms
                .as_ref()
                .and_then(|t| t.launch_submit_us),
            gpu_kernel_d2h_copy_ms: result
                .gpu_kernel_timing_ms
                .as_ref()
                .and_then(|t| t.d2h_copy_ms),
            gpu_kernel_host_decode_ms: result
                .gpu_kernel_timing_ms
                .as_ref()
                .and_then(|t| t.host_decode_ms),
            hybrid_planner_ms: result.hybrid_timing_ms.as_ref().map(|t| t.planner_ms),
            hybrid_cpu_indicator_prep_ms: result
                .hybrid_timing_ms
                .as_ref()
                .map(|t| t.cpu_indicator_prep_ms),
            hybrid_gpu_indicator_prep_ms: result
                .hybrid_timing_ms
                .as_ref()
                .map(|t| t.gpu_indicator_prep_ms),
            hybrid_h2d_upload_ms: result.hybrid_timing_ms.as_ref().map(|t| t.h2d_upload_ms),
            hybrid_kernel_eval_ms: result.hybrid_timing_ms.as_ref().map(|t| t.kernel_eval_ms),
        },
        result,
    })
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let cache_dir = cli
        .cache_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("ta_desktop_demo_faceoff_cache"));
    std::fs::create_dir_all(&cache_dir).map_err(|e| {
        format!(
            "failed to create cache dir {}: {e}",
            cache_dir.to_string_lossy()
        )
    })?;

    let state = AppState::default();

    let load_started = Instant::now();
    let load = state.load_price_data(
        cli.csv
            .to_str()
            .ok_or_else(|| "invalid csv path".to_string())?,
        &cache_dir,
    )?;
    let load_ms = load_started.elapsed().as_secs_f64() * 1000.0;

    let (data_id, resample_ms) = if let Some(raw_tf) = cli.timeframe.as_deref() {
        let started = Instant::now();
        let derived = state.resample_loaded_data(&load.id, raw_tf)?;
        (derived.id, Some(started.elapsed().as_secs_f64() * 1000.0))
    } else {
        (load.id.clone(), None)
    };

    let version = state.dataset_version(&data_id)?;
    let bars = state.get_candles(&data_id)?.close.len();
    let req = make_request(&cli, &data_id);
    let objective_name = objective_name(cli.objective.clone());
    let (expected_contract, expected_metrics_count) =
        expected_metric_contract(cli.objective.clone());
    let fast_spec = range_spec(cli.fast_period);
    let slow_spec = range_spec(cli.slow_period);
    let backend_request = format!("{:?}", cli.backend);
    let requested_backend = backend_from_arg(cli.backend.clone());
    let backend = format!("{:?}", requested_backend.clone());

    let mut warmup_trials = Vec::with_capacity(cli.warmup_trials);
    for i in 0..cli.warmup_trials {
        warmup_trials.push(run_trial(&req, &state, &data_id, i + 1)?);
    }

    let mut measured_trials = Vec::with_capacity(cli.measured_trials);
    for i in 0..cli.measured_trials {
        measured_trials.push(run_trial(&req, &state, &data_id, i + 1)?);
    }
    let mut cold_state_trials = Vec::with_capacity(cli.cold_state_trials);
    for i in 0..cli.cold_state_trials {
        cold_state_trials.push(run_cold_state_trial(&cli, cache_dir.as_path(), i + 1)?);
    }

    let artifact = AppPathFaceoffArtifact {
        schema_version: 1,
        tool: "vectorgrid",
        mode: "app-path",
        timing_boundary: "app-path",
        timing_boundary_note:
            "same-thread harness using AppState plus blocking optimizer call; not the full Tauri command path",
        cold_state_boundary: if cli.cold_state_trials > 0 {
            Some("app-path-fresh-state")
        } else {
            None
        },
        cold_state_boundary_note: if cli.cold_state_trials > 0 {
            Some(
                "fresh AppState plus dataset load/resample and blocking optimizer call per trial; cache-dir policy unchanged",
            )
        } else {
            None
        },
        build_profile: build_profile(),
        environment: environment_fingerprint(),
        requested_backend: requested_backend_label(&requested_backend).to_string(),
        backend,
        request: RequestArtifact {
            objective: objective_name,
            backend_request,
            fast_ma: cli.fast_ma.clone(),
            slow_ma: cli.slow_ma.clone(),
            ma_source: cli.ma_source.clone(),
            fast_range: RangeArtifact {
                start: cli.fast_period.start,
                end: cli.fast_period.end,
                step: cli.fast_period.step,
                spec: fast_spec,
                count: range_count(cli.fast_period),
            },
            slow_range: RangeArtifact {
                start: cli.slow_period.start,
                end: cli.slow_period.end,
                step: cli.slow_period.step,
                spec: slow_spec,
                count: range_count(cli.slow_period),
            },
            pair_count: valid_pair_count(cli.fast_period, cli.slow_period),
            top_k: cli.top_k,
            heatmap_bins: cli.heatmap_bins,
            long_only: cli.long_only,
            allow_flip: cli.allow_flip,
            metric_contract: MetricContractArtifact {
                request: "auto",
                include_all: false,
                include_trade_metrics: false,
                expected_contract,
                expected_metrics_count,
            },
        },
        dataset: FaceoffDatasetArtifact {
            csv_path: cli.csv.to_string_lossy().to_string(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            load_ms,
            resample_ms,
            data_id,
            version,
            bars,
        },
        route_summary: summarize_routes(measured_trials.as_slice()),
        timing_summary: FaceoffTimingSummaryArtifact {
            warmup: summarize_trials(warmup_trials.as_slice()),
            measured: summarize_trials(measured_trials.as_slice()),
            cold_state: summarize_trials(cold_state_trials.as_slice()),
        },
        warmup_trials,
        measured_trials,
        cold_state_trials,
        generated_at_utc: chrono::Utc::now().to_rfc3339(),
    };

    if let Some(parent) = cli.output_json.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output dir {}: {e}",
                parent.to_string_lossy()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&cli.output_json, json).map_err(|e| {
        format!(
            "failed to write output json {}: {e}",
            cli.output_json.to_string_lossy()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        median_f64, median_u64_as_f64, summarize_routes, summarize_trials, TrialArtifact,
        TrialTimingArtifact,
    };
    use ta_app_core::{
        BackendOptimizationResult, BackendUsed, DoubleMaParamsResolved, OptimizationModeResolved,
    };
    use ta_strategies::double_ma::Metrics;

    fn sample_trial(
        total_wall_ms: f64,
        state_lookup_ms: f64,
        harness_wall_ms: f64,
        engine_wall_ms: f64,
        engine_runtime_ms: u64,
        backend_used: BackendUsed,
        backend_route: Option<&str>,
        fallback_reason: Option<&str>,
    ) -> TrialArtifact {
        TrialArtifact {
            trial_index: 1,
            runtime_ms_wall: total_wall_ms,
            timing: TrialTimingArtifact {
                total_wall_ms,
                harness_wall_ms,
                state_lookup_ms,
                engine_wall_ms,
                engine_runtime_ms,
                dataset_load_ms: None,
                dataset_resample_ms: None,
                gpu_kernel_route_wall_ms: None,
                gpu_kernel_gpu_indicator_prep_ms: None,
                gpu_kernel_h2d_upload_ms: None,
                gpu_kernel_launch_submit_us: None,
                gpu_kernel_d2h_copy_ms: None,
                gpu_kernel_host_decode_ms: None,
                hybrid_planner_ms: None,
                hybrid_cpu_indicator_prep_ms: None,
                hybrid_gpu_indicator_prep_ms: None,
                hybrid_h2d_upload_ms: None,
                hybrid_kernel_eval_ms: None,
            },
            result: BackendOptimizationResult {
                best_params: DoubleMaParamsResolved {
                    fast_len: 5,
                    slow_len: 10,
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                    fast_ma_params: None,
                    slow_ma_params: None,
                },
                best_metrics: Metrics::default(),
                best_score: None,
                mode_used: OptimizationModeResolved::Grid,
                backend_used,
                matrix_index: None,
                best_col: None,
                top_cols: None,
                num_combos: 0,
                num_candles: 0,
                runtime_ms: engine_runtime_ms,
                top: Vec::new(),
                top_scores: None,
                all: None,
                all_scores: None,
                heatmap: None,
                vdps: None,
                hybrid_timing_ms: None,
                gpu_kernel_timing_ms: None,
                backend_route: backend_route.map(ToOwned::to_owned),
                backend_fallback_reason: fallback_reason.map(ToOwned::to_owned),
                route_diagnostics: None,
                indicator_route_plan: None,
                residency: None,
                followup_cache_key: None,
                shortlist_slice_diagnostics: None,
                score_observation_context: None,
            },
        }
    }

    #[test]
    fn median_helpers_handle_even_and_odd_lengths() {
        let mut odd_f64 = vec![9.0, 1.0, 5.0];
        let mut even_f64 = vec![8.0, 2.0, 6.0, 4.0];
        let mut odd_u64 = vec![9_u64, 1, 5];
        let mut even_u64 = vec![8_u64, 2, 6, 4];

        assert_eq!(median_f64(odd_f64.as_mut_slice()), 5.0);
        assert_eq!(median_f64(even_f64.as_mut_slice()), 5.0);
        assert_eq!(median_u64_as_f64(odd_u64.as_mut_slice()), 5.0);
        assert_eq!(median_u64_as_f64(even_u64.as_mut_slice()), 5.0);
    }

    #[test]
    fn trial_summary_and_route_summary_include_additive_fields() {
        let trials = vec![
            sample_trial(
                12.0,
                1.0,
                12.0,
                10.0,
                9,
                BackendUsed::Gpu,
                Some("GpuKernel"),
                None,
            ),
            sample_trial(
                16.0,
                2.0,
                16.0,
                13.0,
                11,
                BackendUsed::Gpu,
                Some("GpuKernel"),
                Some("fallback"),
            ),
        ];

        let timing = summarize_trials(&trials).expect("timing summary");
        assert_eq!(timing.total_wall_ms_median, 14.0);
        assert_eq!(timing.state_lookup_ms_median, 1.5);
        assert_eq!(timing.engine_wall_ms_median, 11.5);
        assert_eq!(timing.engine_runtime_ms_median, 10.0);

        let routes = summarize_routes(&trials);
        assert_eq!(routes.backend_used_counts.get("Gpu"), Some(&2));
        assert_eq!(routes.backend_route_counts.get("GpuKernel"), Some(&2));
        assert_eq!(routes.fallback_count, 1);
    }
}
