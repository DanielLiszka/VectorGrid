use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use ta_app_core::{Backend, DoubleMaRequest};
use ta_desktop_demo_app::state::AppState;
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;

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
    harness_wall_ms_min: f64,
    harness_wall_ms_median: f64,
    harness_wall_ms_max: f64,
    engine_runtime_ms_min: u64,
    engine_runtime_ms_median: f64,
    engine_runtime_ms_max: u64,
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
    tool: &'static str,
    mode: &'static str,
    timing_boundary_note: &'static str,
    backend: String,
    dataset: FaceoffDatasetArtifact,
    timing_summary: FaceoffTimingSummaryArtifact,
    warmup_trials: Vec<TrialArtifact>,
    measured_trials: Vec<TrialArtifact>,
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

fn summarize_trials(trials: &[TrialArtifact]) -> Option<TrialTimingSummaryArtifact> {
    if trials.is_empty() {
        return None;
    }

    let mut harness_wall_ms: Vec<f64> = trials
        .iter()
        .map(|trial| trial.timing.harness_wall_ms)
        .collect();
    let mut engine_runtime_ms: Vec<u64> = trials
        .iter()
        .map(|trial| trial.timing.engine_runtime_ms)
        .collect();

    let harness_wall_ms_min = harness_wall_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let harness_wall_ms_max = harness_wall_ms
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let engine_runtime_ms_min = engine_runtime_ms.iter().copied().min().unwrap_or(0);
    let engine_runtime_ms_max = engine_runtime_ms.iter().copied().max().unwrap_or(0);
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
        harness_wall_ms_min,
        harness_wall_ms_median: median_f64(harness_wall_ms.as_mut_slice()),
        harness_wall_ms_max,
        engine_runtime_ms_min,
        engine_runtime_ms_median: median_u64_as_f64(engine_runtime_ms.as_mut_slice()),
        engine_runtime_ms_max,
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

    let mut warmup_trials = Vec::with_capacity(cli.warmup_trials);
    for i in 0..cli.warmup_trials {
        warmup_trials.push(run_trial(&req, &state, &data_id, i + 1)?);
    }

    let mut measured_trials = Vec::with_capacity(cli.measured_trials);
    for i in 0..cli.measured_trials {
        measured_trials.push(run_trial(&req, &state, &data_id, i + 1)?);
    }

    let artifact = AppPathFaceoffArtifact {
        tool: "vectorgrid",
        mode: "app-path",
        timing_boundary_note:
            "same-thread harness using AppState plus blocking optimizer call; not the full Tauri command path",
        backend: format!("{:?}", backend_from_arg(cli.backend)),
        dataset: FaceoffDatasetArtifact {
            csv_path: cli.csv.to_string_lossy().to_string(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            load_ms,
            resample_ms,
            data_id,
            version,
            bars,
        },
        timing_summary: FaceoffTimingSummaryArtifact {
            warmup: summarize_trials(warmup_trials.as_slice()),
            measured: summarize_trials(measured_trials.as_slice()),
        },
        warmup_trials,
        measured_trials,
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
    use super::{median_f64, median_u64_as_f64};

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
}
