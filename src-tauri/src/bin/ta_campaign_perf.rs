use clap::{Parser, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use ta_desktop_demo_app::commands::{
    preload_research_campaign_dataset_pack_headless, run_research_campaign_blocking_headless,
    run_research_campaign_optimize_only_blocking_headless,
    run_research_campaign_optimize_only_preloaded_pack_blocking_headless, Backend, DoubleMaRequest,
    ResearchCampaignDatasetInput, ResearchCampaignExecutionMode, ResearchCampaignPreloadPolicy,
    ResearchCampaignRequest, ResearchCampaignValidationConfig,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use tauri::{Listener, Manager};

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
        if parts.len() != 3 {
            return Err("range must be start:end:step".to_string());
        }
        let start = parts[0]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid range start '{}': {e}", parts[0].trim()))?;
        let end = parts[1]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid range end '{}': {e}", parts[1].trim()))?;
        let step = parts[2]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid range step '{}': {e}", parts[2].trim()))?;
        if start == 0 || end == 0 || step == 0 || start > end {
            return Err("range must be positive start:end:step with start <= end".to_string());
        }
        Ok(Self { start, end, step })
    }
}

#[derive(Clone, Debug, ValueEnum)]
enum ModeArg {
    Disabled,
    HostOnly,
    HostPlusDoublemaVram,
    FullPackHost,
}

#[derive(Clone, Debug, ValueEnum)]
enum BackendArg {
    CpuOnly,
    Auto,
    GpuKernel,
    GpuHybridKernel,
}

#[derive(Clone, Debug, ValueEnum)]
enum ScopeArg {
    Full,
    OptimizeOnly,
}

#[derive(Parser, Debug)]
#[command(name = "ta_campaign_perf")]
#[command(about = "Headless campaign-runner perf harness for preload and DoubleMa VRAM staging")]
struct Cli {
    #[arg(long)]
    output_json: PathBuf,

    #[arg(long)]
    dataset: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = ModeArg::HostOnly)]
    mode: ModeArg,

    #[arg(long, value_enum, default_value_t = ScopeArg::Full)]
    scope: ScopeArg,

    #[arg(long, value_enum, default_value_t = BackendArg::GpuKernel)]
    backend: BackendArg,

    #[arg(long, default_value_t = 1)]
    warmup_trials: usize,

    #[arg(long, default_value_t = 3)]
    measured_trials: usize,

    #[arg(long)]
    timeframe: Option<String>,

    #[arg(long, default_value_t = 0)]
    device_id: u32,

    #[arg(long, default_value = "sma")]
    fast_ma: String,

    #[arg(long, default_value = "ema")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long, default_value = "5:40:5")]
    fast_range: U32Range,

    #[arg(long, default_value = "20:100:10")]
    slow_range: U32Range,

    #[arg(long, default_value_t = 8)]
    top_k: usize,

    #[arg(long, default_value_t = 1)]
    validation_max_candidates: usize,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    cleanup_artifacts: bool,
}

#[derive(Clone, Debug, Serialize)]
struct ProgressEventArtifact {
    elapsed_ms: f64,
    stage: String,
    dataset_index: usize,
    dataset_total: usize,
    label: String,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
struct DatasetRouteArtifact {
    label: String,
    prepare_source: Option<String>,
    prepare_wait_ms: Option<f64>,
    vram_prefetch_outcome: Option<String>,
    vram_prefetch_note: Option<String>,
    vram_prefetch_wall_ms: Option<f64>,
    vram_prefetch_policy: Option<String>,
    vram_prefetch_attempted: Option<bool>,
    vram_prefetch_success: Option<bool>,
    vram_prefetch_failure_risk_event: Option<bool>,
    vram_prefetch_stage_cost_vs_optimize_frac: Option<f64>,
    vram_prefetch_stage_cost_vs_prepare_wait_frac: Option<f64>,
    optimize_wall_ms: Option<f64>,
    optimize_artifact_save_ms: Option<f64>,
    checkpoint_save_ms: Option<f64>,
    optimize_backend_used: Option<String>,
    optimize_backend_route: Option<String>,
    optimize_fallback_reason: Option<String>,
    validate_backend_used: Option<String>,
    validate_backend_route: Option<String>,
    validate_fallback_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct TrialArtifact {
    trial_index: usize,
    warmup: bool,
    total_wall_ms: f64,
    initial_dataset_pack_preload_ms: Option<f64>,
    campaign_runtime_ms: u64,
    optimized: usize,
    validated: usize,
    skipped: usize,
    failed: usize,
    cancelled: bool,
    warning_count: usize,
    warnings: Vec<String>,
    dataset_routes: Vec<DatasetRouteArtifact>,
    progress_events: Vec<ProgressEventArtifact>,
}

#[derive(Debug, Serialize)]
struct RunSummary {
    mode: &'static str,
    scope: &'static str,
    backend: &'static str,
    app_local_data_dir: String,
    datasets: Vec<String>,
    timeframe: Option<String>,
    fast_ma: String,
    slow_ma: String,
    ma_source: String,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
    top_k: usize,
    validation_max_candidates: usize,
    warmup_trials: usize,
    measured_trials: usize,
    mean_total_wall_ms: f64,
    mean_initial_dataset_pack_preload_ms: Option<f64>,
    mean_campaign_runtime_ms: f64,
    prefetch_economics: PrefetchEconomicsSummary,
    measured_trials_detail: Vec<TrialArtifact>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
struct PrefetchEconomicsSummary {
    mode_note: String,
    attempted: usize,
    staged: usize,
    already_resident: usize,
    warnings: usize,
    fatals: usize,
    successful: usize,
    mean_wall_ms: Option<f64>,
    mean_stage_cost_vs_optimize_frac: Option<f64>,
    mean_stage_cost_vs_prepare_wait_frac: Option<f64>,
}

fn default_datasets() -> Vec<PathBuf> {
    [
        "data/AAPL.csv",
        "data/NVDA.csv",
        "data/QQQ.csv",
        "data/SPY.csv",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

fn normalize_dataset_path(path: &Path) -> Result<PathBuf, String> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    joined
        .canonicalize()
        .map_err(|e| format!("failed to resolve dataset '{}': {e}", joined.display()))
}

fn mode_label(mode: &ModeArg) -> &'static str {
    match mode {
        ModeArg::Disabled => "disabled",
        ModeArg::HostOnly => "host_only",
        ModeArg::HostPlusDoublemaVram => "host_plus_doublema_vram",
        ModeArg::FullPackHost => "full_pack_host",
    }
}

fn backend_label(backend: &BackendArg) -> &'static str {
    match backend {
        BackendArg::CpuOnly => "CpuOnly",
        BackendArg::Auto => "Auto",
        BackendArg::GpuKernel => "GpuKernel",
        BackendArg::GpuHybridKernel => "GpuHybridKernel",
    }
}

fn scope_label(scope: &ScopeArg) -> &'static str {
    match scope {
        ScopeArg::Full => "full",
        ScopeArg::OptimizeOnly => "optimize_only",
    }
}

fn to_backend(backend: &BackendArg, device_id: u32) -> Backend {
    match backend {
        BackendArg::CpuOnly => Backend::CpuOnly,
        BackendArg::Auto => Backend::Auto { device_id },
        BackendArg::GpuKernel => Backend::GpuKernel { device_id },
        BackendArg::GpuHybridKernel => Backend::GpuHybridKernel { device_id },
    }
}

fn to_preload_policy(mode: &ModeArg) -> ResearchCampaignPreloadPolicy {
    match mode {
        ModeArg::Disabled => ResearchCampaignPreloadPolicy::Disabled,
        ModeArg::HostOnly => ResearchCampaignPreloadPolicy::HostOnly,
        ModeArg::HostPlusDoublemaVram => ResearchCampaignPreloadPolicy::HostPlusDoublemaVram,
        ModeArg::FullPackHost => ResearchCampaignPreloadPolicy::Disabled,
    }
}

fn build_optimization_template(cli: &Cli) -> Result<Value, String> {
    let req = DoubleMaRequest {
        backend: to_backend(&cli.backend, cli.device_id),
        data_id: "campaign-perf".to_string(),
        fast_range: (
            cli.fast_range.start,
            cli.fast_range.end,
            cli.fast_range.step,
        ),
        slow_range: (
            cli.slow_range.start,
            cli.slow_range.end,
            cli.slow_range.step,
        ),
        fast_ma_types: vec![cli.fast_ma.clone()],
        slow_ma_types: vec![cli.slow_ma.clone()],
        ma_source: cli.ma_source.clone(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy: StrategyConfig::default(),
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        score_mode: None,
        hurdle_tau: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(cli.top_k.max(1)),
        include_all: Some(false),
        heatmap_bins: None,
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: ta_app_core::double_ma::GpuExecutionPolicy::BestEffort,
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,
    };
    serde_json::to_value(req).map_err(|e| e.to_string())
}

fn build_campaign_request(
    cli: &Cli,
    datasets: &[PathBuf],
    trial_index: usize,
) -> Result<ResearchCampaignRequest, String> {
    let optimization_template = build_optimization_template(cli)?;
    let validation_config = ResearchCampaignValidationConfig {
        max_candidates: cli.validation_max_candidates.max(1),
        ..ResearchCampaignValidationConfig::default()
    };
    Ok(ResearchCampaignRequest {
        strategy_kind: "DoubleMa".to_string(),
        datasets: datasets
            .iter()
            .map(|path| ResearchCampaignDatasetInput {
                csv_path: path.to_string_lossy().to_string(),
                label: Some(
                    path.file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("dataset")
                        .to_string(),
                ),
                timeframe: None,
                source_size_bytes: None,
                source_mtime_utc_ms: None,
                source_content_fingerprint: None,
            })
            .collect(),
        timeframe_override: cli.timeframe.clone(),
        tags: vec![
            "campaign_perf".to_string(),
            mode_label(&cli.mode).to_string(),
            format!("trial_{}", trial_index + 1),
        ],
        holdout_pct: 0,
        holdout_tag: None,
        skip_existing: false,
        auto_meta: false,
        auto_stage5: false,
        execution_mode: ResearchCampaignExecutionMode::Full,
        optimization_template,
        validation_config,
        meta_request: None,
        stage5_request: None,
        preload_policy: to_preload_policy(&cli.mode),
    })
}

fn parse_progress_event(payload: &str, trial_started: Instant) -> Option<ProgressEventArtifact> {
    let value: Value = serde_json::from_str(payload).ok()?;
    Some(ProgressEventArtifact {
        elapsed_ms: trial_started.elapsed().as_secs_f64() * 1000.0,
        stage: value.get("stage")?.as_str()?.to_string(),
        dataset_index: value.get("datasetIndex")?.as_u64()? as usize,
        dataset_total: value.get("datasetTotal")?.as_u64()? as usize,
        label: value.get("label")?.as_str()?.to_string(),
        message: value.get("message")?.as_str()?.to_string(),
    })
}

fn cleanup_campaign_outputs(
    summary_path: &Path,
    response: &ta_desktop_demo_app::commands::RunResearchCampaignResponse,
) {
    let mut paths = vec![summary_path.to_path_buf()];
    for item in &response.summary.items {
        if let Some(path) = item.artifact_path.as_ref() {
            paths.push(PathBuf::from(path));
        }
        if let Some(path) = item.report_xlsx_path.as_ref() {
            paths.push(PathBuf::from(path));
        }
    }
    if let Some(path) = response.summary.meta_json_path.as_ref() {
        paths.push(PathBuf::from(path));
    }
    if let Some(path) = response.summary.stage5_json_path.as_ref() {
        paths.push(PathBuf::from(path));
    }
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn artifact_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(|text| text.to_string())
}

fn load_dataset_routes(
    response: &ta_desktop_demo_app::commands::RunResearchCampaignResponse,
) -> Vec<DatasetRouteArtifact> {
    response
        .summary
        .items
        .iter()
        .map(|item| {
            let mut route = DatasetRouteArtifact {
                label: item.label.clone(),
                prepare_source: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.prepare_source.clone()),
                prepare_wait_ms: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.prepare_wait_ms),
                vram_prefetch_outcome: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_outcome.clone()),
                vram_prefetch_note: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_note.clone()),
                vram_prefetch_wall_ms: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_wall_ms),
                vram_prefetch_policy: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_policy.clone()),
                vram_prefetch_attempted: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_attempted),
                vram_prefetch_success: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_success),
                vram_prefetch_failure_risk_event: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_failure_risk_event),
                vram_prefetch_stage_cost_vs_optimize_frac: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_stage_cost_vs_optimize_frac),
                vram_prefetch_stage_cost_vs_prepare_wait_frac: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.vram_prefetch_stage_cost_vs_prepare_wait_frac),
                optimize_wall_ms: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.optimize_wall_ms),
                optimize_artifact_save_ms: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.optimize_artifact_save_ms),
                checkpoint_save_ms: item
                    .diagnostics
                    .as_ref()
                    .and_then(|diag| diag.checkpoint_save_ms),
                optimize_backend_used: None,
                optimize_backend_route: None,
                optimize_fallback_reason: None,
                validate_backend_used: None,
                validate_backend_route: None,
                validate_fallback_reason: None,
            };
            let Some(path) = item.artifact_path.as_ref() else {
                return route;
            };
            let Ok(bytes) = fs::read_to_string(path) else {
                return route;
            };
            let Ok(value) = serde_json::from_str::<Value>(&bytes) else {
                return route;
            };
            let optimize = value.get("run").and_then(|run| run.get("res"));
            route.optimize_backend_used =
                artifact_string(optimize.and_then(|row| row.get("backend_used")));
            route.optimize_backend_route =
                artifact_string(optimize.and_then(|row| row.get("backend_route")));
            route.optimize_fallback_reason =
                artifact_string(optimize.and_then(|row| row.get("backend_fallback_reason")));

            let validation = value
                .get("validation")
                .and_then(|row| row.as_object())
                .and_then(|row| row.get("res"));
            route.validate_backend_used =
                artifact_string(validation.and_then(|row| row.get("backend_used")));
            route.validate_backend_route =
                artifact_string(validation.and_then(|row| row.get("backend_route")));
            route.validate_fallback_reason =
                artifact_string(validation.and_then(|row| row.get("backend_fallback_reason")));
            route
        })
        .collect()
}

fn measured_mean(values: &[TrialArtifact], f: impl Fn(&TrialArtifact) -> f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(f).sum::<f64>() / values.len() as f64
}

fn measured_mean_option(
    values: &[TrialArtifact],
    f: impl Fn(&TrialArtifact) -> Option<f64>,
) -> Option<f64> {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        if let Some(sample) = f(value) {
            total += sample;
            count += 1;
        }
    }
    (count > 0).then_some(total / count as f64)
}

fn mean_option_values(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values.flatten() {
        total += value;
        count += 1;
    }
    (count > 0).then_some(total / count as f64)
}

fn summarize_prefetch_economics(trials: &[TrialArtifact]) -> PrefetchEconomicsSummary {
    let routes: Vec<&DatasetRouteArtifact> = trials
        .iter()
        .flat_map(|trial| trial.dataset_routes.iter())
        .collect();
    let attempted = routes
        .iter()
        .filter(|route| route.vram_prefetch_attempted == Some(true))
        .count();
    let staged = routes
        .iter()
        .filter(|route| route.vram_prefetch_outcome.as_deref() == Some("staged"))
        .count();
    let already_resident = routes
        .iter()
        .filter(|route| route.vram_prefetch_outcome.as_deref() == Some("already_resident"))
        .count();
    let warnings = routes
        .iter()
        .filter(|route| route.vram_prefetch_outcome.as_deref() == Some("warning"))
        .count();
    let fatals = routes
        .iter()
        .filter(|route| route.vram_prefetch_outcome.as_deref() == Some("fatal"))
        .count();
    let successful = routes
        .iter()
        .filter(|route| route.vram_prefetch_success == Some(true))
        .count();

    PrefetchEconomicsSummary {
        mode_note:
            "observational heuristic telemetry only; compare modes before promoting policy changes"
                .to_string(),
        attempted,
        staged,
        already_resident,
        warnings,
        fatals,
        successful,
        mean_wall_ms: mean_option_values(routes.iter().map(|route| route.vram_prefetch_wall_ms)),
        mean_stage_cost_vs_optimize_frac: mean_option_values(
            routes
                .iter()
                .map(|route| route.vram_prefetch_stage_cost_vs_optimize_frac),
        ),
        mean_stage_cost_vs_prepare_wait_frac: mean_option_values(
            routes
                .iter()
                .map(|route| route.vram_prefetch_stage_cost_vs_prepare_wait_frac),
        ),
    }
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    if cli.measured_trials == 0 {
        return Err("--measured-trials must be >= 1".to_string());
    }
    if matches!(cli.mode, ModeArg::FullPackHost) && !matches!(cli.scope, ScopeArg::OptimizeOnly) {
        return Err(
            "--mode full-pack-host currently supports only --scope optimize-only".to_string(),
        );
    }

    let datasets = if cli.dataset.is_empty() {
        default_datasets()
    } else {
        cli.dataset.clone()
    };
    let datasets: Vec<PathBuf> = datasets
        .iter()
        .map(|path| normalize_dataset_path(path))
        .collect::<Result<_, _>>()?;
    if datasets.len() < 2 {
        return Err("campaign perf requires at least two datasets".to_string());
    }

    if let Some(parent) = cli.output_json.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;

    let total_trials = cli.warmup_trials + cli.measured_trials;
    let mut measured = Vec::with_capacity(cli.measured_trials);

    for trial_index in 0..total_trials {
        let trial_started = Instant::now();
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_for_listener = Arc::clone(&events);
        let progress_listener = app_handle.listen_any("research_campaign_progress", move |event| {
            if let Some(parsed) = parse_progress_event(event.payload(), trial_started) {
                events_for_listener
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(parsed);
            }
        });

        let trial_result: Result<_, String> = (|| {
            let request = build_campaign_request(&cli, &datasets, trial_index)?;
            let cancel = Arc::new(AtomicBool::new(false));
            match (&cli.mode, &cli.scope) {
                (ModeArg::FullPackHost, ScopeArg::OptimizeOnly) => {
                    let preload_started = Instant::now();
                    let preloaded_pack =
                        preload_research_campaign_dataset_pack_headless(&request, &app_handle)?;
                    let preload_ms = preload_started.elapsed().as_secs_f64() * 1000.0;
                    let response =
                        run_research_campaign_optimize_only_preloaded_pack_blocking_headless(
                            request,
                            preloaded_pack,
                            app_handle.clone(),
                            cancel,
                        )?;
                    Ok((response, Some(preload_ms)))
                }
                (_, ScopeArg::Full) => {
                    let response = run_research_campaign_blocking_headless(
                        request,
                        app_handle.clone(),
                        cancel,
                    )?;
                    Ok((response, None))
                }
                (_, ScopeArg::OptimizeOnly) => {
                    let response = run_research_campaign_optimize_only_blocking_headless(
                        request,
                        app_handle.clone(),
                        cancel,
                    )?;
                    Ok((response, None))
                }
            }
        })();
        app_handle.unlisten(progress_listener);
        let (response, initial_dataset_pack_preload_ms) = trial_result?;

        let total_wall_ms = trial_started.elapsed().as_secs_f64() * 1000.0;
        let progress_events = events.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let dataset_routes = load_dataset_routes(&response);
        let summary_path = PathBuf::from(&response.summary_json_path);
        if cli.cleanup_artifacts {
            cleanup_campaign_outputs(&summary_path, &response);
        }

        if trial_index >= cli.warmup_trials {
            measured.push(TrialArtifact {
                trial_index: trial_index + 1,
                warmup: false,
                total_wall_ms,
                initial_dataset_pack_preload_ms,
                campaign_runtime_ms: response.summary.runtime_ms,
                optimized: response.summary.counts.optimized,
                validated: response.summary.counts.validated,
                skipped: response.summary.counts.skipped,
                failed: response.summary.counts.failed,
                cancelled: response.summary.counts.cancelled,
                warning_count: response.summary.warnings.len(),
                warnings: response.summary.warnings.clone(),
                dataset_routes,
                progress_events,
            });
        }
    }

    let summary = RunSummary {
        mode: mode_label(&cli.mode),
        scope: scope_label(&cli.scope),
        backend: backend_label(&cli.backend),
        app_local_data_dir: app_local_data_dir.to_string_lossy().to_string(),
        datasets: datasets
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        timeframe: cli.timeframe.clone(),
        fast_ma: cli.fast_ma.clone(),
        slow_ma: cli.slow_ma.clone(),
        ma_source: cli.ma_source.clone(),
        fast_range: (
            cli.fast_range.start,
            cli.fast_range.end,
            cli.fast_range.step,
        ),
        slow_range: (
            cli.slow_range.start,
            cli.slow_range.end,
            cli.slow_range.step,
        ),
        top_k: cli.top_k,
        validation_max_candidates: cli.validation_max_candidates,
        warmup_trials: cli.warmup_trials,
        measured_trials: cli.measured_trials,
        mean_total_wall_ms: measured_mean(&measured, |trial| trial.total_wall_ms),
        mean_initial_dataset_pack_preload_ms: measured_mean_option(&measured, |trial| {
            trial.initial_dataset_pack_preload_ms
        }),
        mean_campaign_runtime_ms: measured_mean(&measured, |trial| {
            trial.campaign_runtime_ms as f64
        }),
        prefetch_economics: summarize_prefetch_economics(&measured),
        measured_trials_detail: measured,
    };

    fs::write(
        &cli.output_json,
        serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    println!(
        "wrote {} summary to {}",
        summary.mode,
        cli.output_json.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        summarize_prefetch_economics, DatasetRouteArtifact, PrefetchEconomicsSummary, TrialArtifact,
    };

    fn route(label: &str, outcome: Option<&str>, wall_ms: Option<f64>) -> DatasetRouteArtifact {
        DatasetRouteArtifact {
            label: label.to_string(),
            prepare_source: None,
            prepare_wait_ms: None,
            vram_prefetch_outcome: outcome.map(str::to_string),
            vram_prefetch_note: None,
            vram_prefetch_wall_ms: wall_ms,
            vram_prefetch_policy: Some("heuristic_observational".to_string()),
            vram_prefetch_attempted: outcome.map(|_| true),
            vram_prefetch_success: matches!(outcome, Some("staged" | "already_resident"))
                .then_some(true)
                .or_else(|| outcome.map(|_| false)),
            vram_prefetch_failure_risk_event: matches!(outcome, Some("warning" | "fatal"))
                .then_some(true)
                .or_else(|| outcome.map(|_| false)),
            vram_prefetch_stage_cost_vs_optimize_frac: wall_ms.map(|value| value / 10.0),
            vram_prefetch_stage_cost_vs_prepare_wait_frac: wall_ms.map(|value| value / 20.0),
            optimize_wall_ms: None,
            optimize_artifact_save_ms: None,
            checkpoint_save_ms: None,
            optimize_backend_used: None,
            optimize_backend_route: None,
            optimize_fallback_reason: None,
            validate_backend_used: None,
            validate_backend_route: None,
            validate_fallback_reason: None,
        }
    }

    #[test]
    fn summarize_prefetch_economics_aggregates_observational_route_metrics() {
        let trials = vec![
            TrialArtifact {
                trial_index: 1,
                warmup: false,
                total_wall_ms: 10.0,
                initial_dataset_pack_preload_ms: None,
                campaign_runtime_ms: 10,
                optimized: 1,
                validated: 1,
                skipped: 0,
                failed: 0,
                cancelled: false,
                warning_count: 0,
                warnings: Vec::new(),
                dataset_routes: vec![
                    route("AAPL", Some("staged"), Some(4.0)),
                    route("MSFT", Some("warning"), Some(6.0)),
                ],
                progress_events: Vec::new(),
            },
            TrialArtifact {
                trial_index: 2,
                warmup: false,
                total_wall_ms: 12.0,
                initial_dataset_pack_preload_ms: None,
                campaign_runtime_ms: 12,
                optimized: 1,
                validated: 1,
                skipped: 0,
                failed: 0,
                cancelled: false,
                warning_count: 0,
                warnings: Vec::new(),
                dataset_routes: vec![
                    route("NVDA", Some("already_resident"), Some(2.0)),
                    route("QQQ", Some("fatal"), Some(8.0)),
                ],
                progress_events: Vec::new(),
            },
        ];

        let summary = summarize_prefetch_economics(&trials);

        assert_eq!(
            summary,
            PrefetchEconomicsSummary {
                mode_note:
                    "observational heuristic telemetry only; compare modes before promoting policy changes"
                        .to_string(),
                attempted: 4,
                staged: 1,
                already_resident: 1,
                warnings: 1,
                fatals: 1,
                successful: 2,
                mean_wall_ms: Some(5.0),
                mean_stage_cost_vs_optimize_frac: Some(0.5),
                mean_stage_cost_vs_prepare_wait_frac: Some(0.25),
            }
        );
    }
}
