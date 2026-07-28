use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use ta_desktop_demo_app::commands::{
    list_run_artifacts, load_artifact, run_research_campaign_optimize_only_blocking_headless,
    Backend, DoubleMaRequest, PortfolioFeasibilityRequestV1, ResearchCampaignDatasetInput,
    ResearchCampaignExecutionMode, ResearchCampaignPreloadPolicy, ResearchCampaignRequest,
    ResearchCampaignValidationConfig,
};
use ta_desktop_demo_app::data_cache;
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use tauri::{Listener, Manager};

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;

const EVIDENCE_SCHEMA_VERSION: u32 = 1;
const DATASET_COUNT: usize = 3;
const PARTIAL_FAILURE_DATASET_COUNT: usize = 3;
const PARTIAL_FAILURE_INDEX: usize = 1;
const LARGE_CATALOG_DATASET_COUNT: usize = 12;

#[derive(Parser, Debug)]
#[command(name = "ta_campaign_restart_qa")]
#[command(about = "Separate-process campaign interruption/resume and portfolio QA harness")]
struct Cli {
    #[arg(long)]
    run_root: PathBuf,

    #[arg(long)]
    run_id: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Prepare {
        #[arg(long)]
        source_dataset: PathBuf,
        #[arg(long)]
        output_json: PathBuf,
    },
    Run {
        #[arg(long, value_enum)]
        phase: CampaignPhase,
        #[arg(long = "dataset", required = true)]
        datasets: Vec<PathBuf>,
        #[arg(long)]
        output_json: PathBuf,
    },
    Mutate {
        #[arg(long)]
        dataset: PathBuf,
        #[arg(long)]
        output_json: PathBuf,
    },
    Repair {
        #[arg(long)]
        dataset: PathBuf,
        #[arg(long)]
        source_dataset: PathBuf,
        #[arg(long)]
        output_json: PathBuf,
    },
    ProbeCorrupt {
        #[arg(long)]
        phase_json: PathBuf,
        #[arg(long)]
        output_json: PathBuf,
    },
    Verify {
        #[arg(long = "phase-json", required = true)]
        phase_jsons: Vec<PathBuf>,
        #[arg(long)]
        probe_json: PathBuf,
        #[arg(long)]
        mutation_json: PathBuf,
        #[arg(long)]
        repair_json: PathBuf,
        #[arg(long)]
        output_json: PathBuf,
    },
    Cleanup {
        #[arg(long)]
        output_json: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
enum CampaignPhase {
    Interrupted,
    Resume,
    Noop,
    StaleResume,
    StaleNoop,
    PartialFailure,
    PartialRetry,
    LargeCatalog,
    LargeNoop,
}

impl CampaignPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Interrupted => "interrupted",
            Self::Resume => "resume",
            Self::Noop => "noop",
            Self::StaleResume => "stale_resume",
            Self::StaleNoop => "stale_noop",
            Self::PartialFailure => "partial_failure",
            Self::PartialRetry => "partial_retry",
            Self::LargeCatalog => "large_catalog",
            Self::LargeNoop => "large_noop",
        }
    }

    fn skip_existing(self) -> bool {
        !matches!(
            self,
            Self::Interrupted | Self::PartialFailure | Self::LargeCatalog
        )
    }

    fn cancel_after_items(self) -> Option<usize> {
        matches!(self, Self::Interrupted).then_some(1)
    }

    fn expected_counts(self, dataset_count: usize) -> (usize, usize, usize, bool) {
        match self {
            Self::Interrupted => (1, 0, 0, true),
            Self::Resume => (dataset_count.saturating_sub(1), 1, 0, false),
            Self::Noop => (0, dataset_count, 0, false),
            Self::StaleResume => (1, dataset_count.saturating_sub(1), 0, false),
            Self::StaleNoop => (0, dataset_count, 0, false),
            Self::PartialFailure => (dataset_count.saturating_sub(1), 0, 1, false),
            Self::PartialRetry => (1, dataset_count.saturating_sub(1), 0, false),
            Self::LargeCatalog => (dataset_count, 0, 0, false),
            Self::LargeNoop => (0, dataset_count, 0, false),
        }
    }

    fn expected_dataset_count(self) -> usize {
        match self {
            Self::PartialFailure | Self::PartialRetry => PARTIAL_FAILURE_DATASET_COUNT,
            Self::LargeCatalog | Self::LargeNoop => LARGE_CATALOG_DATASET_COUNT,
            _ => DATASET_COUNT,
        }
    }

    fn scope(self) -> &'static str {
        match self {
            Self::PartialFailure | Self::PartialRetry => "partial_failure",
            Self::LargeCatalog | Self::LargeNoop => "large_catalog",
            _ => "restart",
        }
    }
}

#[derive(Debug, Serialize)]
struct FileEvidence {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct RunArtifactEvidence {
    item_index: usize,
    item_id: String,
    csv_path: String,
    artifact: FileEvidence,
    source_content_fingerprint: String,
}

#[derive(Debug, Serialize)]
struct PrepareEvidence {
    schema_version: u32,
    kind: &'static str,
    process_id: u32,
    run_id: String,
    source_dataset: FileEvidence,
    datasets: Vec<FileEvidence>,
    partial_failure_datasets: Vec<FileEvidence>,
    injected_failure_dataset: FileEvidence,
    large_catalog_datasets: Vec<FileEvidence>,
}

#[derive(Clone, Debug, Serialize)]
struct ProgressCheckpointEvidence {
    dataset_index: usize,
    dataset_total: usize,
    optimized: usize,
    skipped: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct PhaseEvidence {
    schema_version: u32,
    kind: &'static str,
    phase: &'static str,
    process_id: u32,
    run_id: String,
    app_identifier: String,
    app_local_data_dir: String,
    summary: FileEvidence,
    durable_summary: Value,
    response_summary_matches_durable: bool,
    response: Value,
    progress_checkpoints: Vec<ProgressCheckpointEvidence>,
    item_artifacts: Vec<RunArtifactEvidence>,
    catalog_artifacts: Vec<FileEvidence>,
    portfolio_result: ta_app_core::PortfolioFeasibilityResultV1,
    portfolio_sha256: String,
}

#[derive(Debug, Serialize)]
struct MutationEvidence {
    schema_version: u32,
    kind: &'static str,
    process_id: u32,
    run_id: String,
    dataset_path: String,
    old_content_fingerprint: String,
    new_content_fingerprint: String,
    old_sha256: String,
    new_sha256: String,
}

#[derive(Debug, Serialize)]
struct RepairEvidence {
    schema_version: u32,
    kind: &'static str,
    process_id: u32,
    run_id: String,
    dataset_path: String,
    source_dataset_path: String,
    old_content_fingerprint: String,
    new_content_fingerprint: String,
    old_sha256: String,
    new_sha256: String,
    source_sha256: String,
}

#[derive(Debug, Serialize)]
struct CorruptionProbeEvidence {
    schema_version: u32,
    kind: &'static str,
    process_id: u32,
    run_id: String,
    valid_summary_loaded: bool,
    malformed_json_rejected: bool,
    malformed_json_error: String,
    manifest_tamper_rejected: bool,
    manifest_tamper_error: String,
    probe_files_removed: bool,
}

#[derive(Debug, Serialize)]
struct VerificationEvidence {
    schema_version: u32,
    kind: &'static str,
    passed: bool,
    process_id: u32,
    run_id: String,
    phase_process_ids: Vec<u32>,
    dataset_count: usize,
    partial_failure_dataset_count: usize,
    large_catalog_dataset_count: usize,
    process_restart_count: usize,
    initial_manifest_hash: String,
    stale_manifest_hash: String,
    stable_item_ids_before_mutation: Vec<String>,
    stable_artifact_paths_before_mutation: Vec<String>,
    stable_artifact_paths_after_mutation: Vec<String>,
    portfolio_hash_before_mutation: String,
    portfolio_hash_after_mutation: String,
    corruption_probes_passed: bool,
    stale_artifact_reuse_blocked: bool,
    partial_failure_retry_passed: bool,
    large_catalog_identity_collision_progress_passed: bool,
    no_op_resume_artifact_bytes_stable: bool,
    all_referenced_files_hash_verified: bool,
    notes: Vec<String>,
}

fn normalize_run_id(raw: &str) -> Result<String, String> {
    let value = raw.trim().to_ascii_lowercase();
    if !(8..=40).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(
            "--run-id must be 8-40 lowercase ASCII letters/digits for isolated app identity"
                .to_string(),
        );
    }
    Ok(value)
}

fn app_identifier(run_id: &str) -> String {
    format!("dev.vectorgrid.campaignrestartqa.{run_id}")
}

fn build_isolated_app(run_id: &str) -> Result<tauri::App, String> {
    let identifier = app_identifier(run_id);
    let mut context = tauri::generate_context!();
    context.config_mut().product_name = Some(format!("VectorGridCampaignRestartQa{run_id}"));
    context.config_mut().identifier = identifier;
    tauri::Builder::default()
        .build(context)
        .map_err(|error| error.to_string())
}

fn canonical_root(path: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("create run root {}: {error}", path.display()))?;
    path.canonicalize()
        .map_err(|error| format!("canonicalize run root {}: {error}", path.display()))
}

fn require_output_path(run_root: &Path, path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .filter(|entry| !entry.as_os_str().is_empty())
        .unwrap_or(run_root);
    fs::create_dir_all(parent)
        .map_err(|error| format!("create output parent {}: {error}", parent.display()))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("canonicalize output parent {}: {error}", parent.display()))?;
    if !parent.starts_with(run_root) {
        return Err(format!(
            "refusing to write QA evidence outside run root: {}",
            path.display()
        ));
    }
    let name = path
        .file_name()
        .ok_or_else(|| format!("output path has no file name: {}", path.display()))?;
    Ok(parent.join(name))
}

fn require_contained_file(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("canonicalize file {}: {error}", path.display()))?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(format!(
            "refusing to use file outside the expected QA root: {}",
            path.display()
        ));
    }
    Ok(canonical)
}

fn verify_isolated_app_dir(path: &Path, run_id: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("create isolated app-data dir {}: {error}", path.display()))?;
    let canonical = path.canonicalize().map_err(|error| {
        format!(
            "canonicalize isolated app-data dir {}: {error}",
            path.display()
        )
    })?;
    let normalized = canonical
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let expected = app_identifier(run_id);
    if !normalized.contains("campaignrestartqa") || !normalized.contains(&expected) {
        return Err(format!(
            "resolved app-data directory is not owned by this campaign QA run: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read file for hash {}: {error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn file_evidence(path: &Path) -> Result<FileEvidence, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("stat evidence file {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("evidence path is not a file: {}", path.display()));
    }
    Ok(FileEvidence {
        path: path.to_string_lossy().to_string(),
        bytes: metadata.len(),
        sha256: sha256_file(path)?,
    })
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    atomic_output::write_bytes(path, bytes)
}

fn json_semantically_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => {
            if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
                return left == right;
            }
            if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
                return left == right;
            }
            match (left.as_f64(), right.as_f64()) {
                (Some(left), Some(right)) => {
                    let scale = left.abs().max(right.abs()).max(1.0);
                    (left - right).abs() <= scale * 1.0e-12
                }
                _ => false,
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_semantically_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| json_semantically_equal(left, right))
                })
        }
        _ => false,
    }
}

fn prepare_datasets(
    run_root: &Path,
    run_id: &str,
    source_dataset: &Path,
    output_json: &Path,
) -> Result<(), String> {
    let source_dataset = source_dataset
        .canonicalize()
        .map_err(|error| format!("canonicalize source dataset: {error}"))?;
    if !source_dataset.is_file() {
        return Err(format!(
            "source dataset is not a regular file: {}",
            source_dataset.display()
        ));
    }
    let source_bytes = fs::read(&source_dataset)
        .map_err(|error| format!("read source dataset {}: {error}", source_dataset.display()))?;
    let dataset_dir = run_root.join("datasets");
    fs::create_dir_all(&dataset_dir)
        .map_err(|error| format!("create dataset dir {}: {error}", dataset_dir.display()))?;

    let mut datasets = Vec::with_capacity(DATASET_COUNT);
    for index in 0..DATASET_COUNT {
        let path = dataset_dir.join(format!("campaign_dataset_{:02}.csv", index + 1));
        atomic_output::write_bytes(&path, &source_bytes)?;
        datasets.push(file_evidence(&path)?);
    }
    let partial_failure_dir = dataset_dir.join("partial_failure");
    fs::create_dir_all(&partial_failure_dir).map_err(|error| {
        format!(
            "create partial-failure dataset dir {}: {error}",
            partial_failure_dir.display()
        )
    })?;
    let mut partial_failure_datasets = Vec::with_capacity(PARTIAL_FAILURE_DATASET_COUNT);
    for index in 0..PARTIAL_FAILURE_DATASET_COUNT {
        let path = partial_failure_dir.join(format!("partial_dataset_{:02}.csv", index + 1));
        if index == PARTIAL_FAILURE_INDEX {
            atomic_output::write_bytes(
                &path,
                b"timestamp,open,high,low,close,volume\n\xff,1,2,0,1,10\n",
            )?;
        } else {
            atomic_output::write_bytes(&path, &source_bytes)?;
        }
        partial_failure_datasets.push(file_evidence(&path)?);
    }
    let injected_failure_dataset = file_evidence(&partial_failure_dir.join(format!(
        "partial_dataset_{:02}.csv",
        PARTIAL_FAILURE_INDEX + 1
    )))?;

    let large_catalog_dir = dataset_dir.join("large_catalog");
    fs::create_dir_all(&large_catalog_dir).map_err(|error| {
        format!(
            "create large-catalog dataset dir {}: {error}",
            large_catalog_dir.display()
        )
    })?;
    let mut large_catalog_datasets = Vec::with_capacity(LARGE_CATALOG_DATASET_COUNT);
    for index in 0..LARGE_CATALOG_DATASET_COUNT {
        let path = large_catalog_dir.join(format!("large_dataset_{:02}.csv", index + 1));
        atomic_output::write_bytes(&path, &source_bytes)?;
        large_catalog_datasets.push(file_evidence(&path)?);
    }
    let evidence = PrepareEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: "vectorgrid_campaign_restart_prepare",
        process_id: std::process::id(),
        run_id: run_id.to_string(),
        source_dataset: file_evidence(&source_dataset)?,
        datasets,
        partial_failure_datasets,
        injected_failure_dataset,
        large_catalog_datasets,
    };
    write_json(output_json, &evidence)
}

fn build_optimization_template() -> Result<Value, String> {
    let request = DoubleMaRequest {
        backend: Backend::CpuOnly,
        data_id: "campaign-restart-qa".to_string(),
        fast_range: (3, 12, 3),
        slow_range: (15, 30, 5),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["ema".to_string()],
        ma_source: "close".to_string(),
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
        top_k: Some(4),
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
    serde_json::to_value(request).map_err(|error| error.to_string())
}

fn campaign_dataset_input(
    path: &Path,
    index: usize,
    scope: &str,
) -> Result<ResearchCampaignDatasetInput, String> {
    let identity = data_cache::identity_for_source(path)?;
    Ok(ResearchCampaignDatasetInput {
        csv_path: path.to_string_lossy().to_string(),
        label: Some(format!("{}{:02}", scope.to_ascii_uppercase(), index + 1)),
        timeframe: Some("1m".to_string()),
        source_size_bytes: Some(identity.size),
        source_mtime_utc_ms: Some(identity.mtime_utc_ms),
        source_content_fingerprint: Some(data_cache::content_fingerprint_for_source(path)?),
    })
}

fn build_campaign_request(
    phase: CampaignPhase,
    run_id: &str,
    datasets: &[PathBuf],
) -> Result<ResearchCampaignRequest, String> {
    Ok(ResearchCampaignRequest {
        strategy_kind: "DoubleMa".to_string(),
        datasets: datasets
            .iter()
            .enumerate()
            .map(|(index, path)| campaign_dataset_input(path, index, phase.scope()))
            .collect::<Result<Vec<_>, _>>()?,
        timeframe_override: Some("1m".to_string()),
        tags: vec![
            format!("campaign_restart_qa_{}", phase.scope()),
            format!("campaign_restart_qa_{run_id}"),
        ],
        holdout_pct: 0,
        holdout_tag: None,
        skip_existing: phase.skip_existing(),
        auto_meta: false,
        auto_stage5: false,
        execution_mode: ResearchCampaignExecutionMode::OptimizeOnly,
        optimization_template: build_optimization_template()?,
        validation_config: ResearchCampaignValidationConfig::default(),
        meta_request: None,
        stage5_request: None,
        preload_policy: ResearchCampaignPreloadPolicy::Disabled,
    })
}

fn run_phase(
    run_root: &Path,
    run_id: &str,
    phase: CampaignPhase,
    datasets: Vec<PathBuf>,
    output_json: &Path,
) -> Result<(), String> {
    let expected_dataset_count = phase.expected_dataset_count();
    if datasets.len() != expected_dataset_count {
        return Err(format!(
            "{} campaign QA phase requires exactly {} datasets",
            phase.as_str(),
            expected_dataset_count
        ));
    }
    let dataset_root = run_root.join("datasets").canonicalize().map_err(|error| {
        format!(
            "canonicalize campaign dataset root {}: {error}",
            run_root.join("datasets").display()
        )
    })?;
    let datasets = datasets
        .iter()
        .map(|path| require_contained_file(&dataset_root, path))
        .collect::<Result<Vec<_>, _>>()?;

    let app = build_isolated_app(run_id)?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = verify_isolated_app_dir(
        &app_handle
            .path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?,
        run_id,
    )?;

    let request = build_campaign_request(phase, run_id, &datasets)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let progress_checkpoints = Arc::new(Mutex::new(Vec::new()));
    let cancel_for_listener = Arc::clone(&cancel);
    let progress_for_listener = Arc::clone(&progress_checkpoints);
    let cancel_after_items = phase.cancel_after_items();
    let listener_id = app_handle.listen_any("research_campaign_progress", move |event| {
        let Ok(payload) = serde_json::from_str::<Value>(event.payload()) else {
            return;
        };
        let is_checkpoint = payload.get("stage").and_then(Value::as_str) == Some("checkpoint")
            && payload.get("message").and_then(Value::as_str) == Some("dataset checkpoint saved");
        if !is_checkpoint {
            return;
        }
        let checkpoint = ProgressCheckpointEvidence {
            dataset_index: payload
                .get("datasetIndex")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            dataset_total: payload
                .get("datasetTotal")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            optimized: payload
                .get("optimized")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            skipped: payload.get("skipped").and_then(Value::as_u64).unwrap_or(0) as usize,
            failed: payload.get("failed").and_then(Value::as_u64).unwrap_or(0) as usize,
        };
        if let Ok(mut checkpoints) = progress_for_listener.lock() {
            checkpoints.push(checkpoint.clone());
        }
        if cancel_after_items.is_some_and(|limit| checkpoint.dataset_index >= limit) {
            cancel_for_listener.store(true, Ordering::Release);
        }
    });

    let response = run_research_campaign_optimize_only_blocking_headless(
        request,
        app_handle.clone(),
        Arc::clone(&cancel),
    )?;
    app_handle.unlisten(listener_id);
    let progress_checkpoints = progress_checkpoints
        .lock()
        .map_err(|_| "campaign progress checkpoint evidence lock was poisoned".to_string())?
        .clone();

    let (expected_optimized, expected_skipped, expected_failed, expected_cancelled) =
        phase.expected_counts(datasets.len());
    if response.summary.counts.optimized != expected_optimized
        || response.summary.counts.skipped != expected_skipped
        || response.summary.counts.cancelled != expected_cancelled
        || response.summary.counts.failed != expected_failed
        || response.summary.counts.validated != 0
    {
        return Err(format!(
            "{} phase counts were unexpected: optimized={}, skipped={}, failed={}, cancelled={}",
            phase.as_str(),
            response.summary.counts.optimized,
            response.summary.counts.skipped,
            response.summary.counts.failed,
            response.summary.counts.cancelled
        ));
    }
    let expected_checkpoint_indices = response
        .summary
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.status == "optimized").then_some(index + 1))
        .collect::<Vec<_>>();
    let observed_checkpoint_indices = progress_checkpoints
        .iter()
        .map(|checkpoint| checkpoint.dataset_index)
        .collect::<Vec<_>>();
    if observed_checkpoint_indices != expected_checkpoint_indices
        || progress_checkpoints.iter().any(|checkpoint| {
            checkpoint.dataset_total != datasets.len()
                || checkpoint.dataset_index == 0
                || checkpoint.dataset_index > checkpoint.dataset_total
                || checkpoint
                    .optimized
                    .saturating_add(checkpoint.skipped)
                    .saturating_add(checkpoint.failed)
                    > checkpoint.dataset_total
        })
    {
        return Err(format!(
            "{} phase emitted invalid durable-checkpoint progress: observed={observed_checkpoint_indices:?}, expected={expected_checkpoint_indices:?}",
            phase.as_str()
        ));
    }
    if matches!(phase, CampaignPhase::StaleResume)
        && !response
            .summary
            .warnings
            .iter()
            .any(|warning| warning.contains("classified as stale"))
    {
        return Err("stale resume did not report rejected stale artifact reuse".to_string());
    }

    let summary_path =
        require_contained_file(&app_local_data_dir, Path::new(&response.summary_json_path))?;
    let loaded_summary = tauri::async_runtime::block_on(load_artifact(
        response.summary_json_path.clone(),
        app_handle.clone(),
    ))?;
    let response_summary =
        serde_json::to_value(&response.summary).map_err(|error| error.to_string())?;
    let response_summary_matches_durable =
        json_semantically_equal(&loaded_summary, &response_summary);
    if !response_summary_matches_durable {
        return Err(
            "returned campaign summary does not semantically match its durable checkpoint"
                .to_string(),
        );
    }

    let mut item_artifacts = Vec::new();
    let mut portfolio_sources = Vec::new();
    for (item_index, item) in response.summary.items.iter().enumerate() {
        let Some(path) = item.artifact_path.as_deref() else {
            continue;
        };
        let canonical = require_contained_file(&app_local_data_dir, Path::new(path))?;
        let raw = tauri::async_runtime::block_on(load_artifact(
            canonical.to_string_lossy().to_string(),
            app_handle.clone(),
        ))?;
        let artifact_hash = sha256_file(&canonical)?;
        portfolio_sources.push(json!({
            "artifact_kind": "validation_artifact_bundle",
            "artifact_hash": artifact_hash,
            "config_fingerprint": raw.get("config_fingerprint"),
            "dataset_meta": raw.get("dataset_meta"),
            "run_res": raw.pointer("/run/res"),
        }));
        let source_content_fingerprint = item
            .dataset_identity
            .as_ref()
            .and_then(|identity| identity.source_content_fingerprint.clone())
            .ok_or_else(|| {
                format!(
                    "campaign item {} did not preserve strict source identity",
                    item_index + 1
                )
            })?;
        item_artifacts.push(RunArtifactEvidence {
            item_index,
            item_id: item
                .item_id
                .clone()
                .ok_or_else(|| format!("campaign item {} is missing item_id", item_index + 1))?,
            csv_path: item.csv_path.clone(),
            artifact: file_evidence(&canonical)?,
            source_content_fingerprint,
        });
    }
    let expected_artifact_count = expected_optimized.saturating_add(expected_skipped);
    if item_artifacts.len() != expected_artifact_count {
        return Err(format!(
            "{} phase referenced {} artifacts; expected {}",
            phase.as_str(),
            item_artifacts.len(),
            expected_artifact_count
        ));
    }

    let portfolio_result =
        ta_app_core::portfolio_feasibility_from_artifacts(PortfolioFeasibilityRequestV1 {
            source_artifacts: portfolio_sources,
            max_candidates: 100,
            require_approved_validation: false,
            require_replay_matrix: false,
            require_source_provenance: true,
        });
    if portfolio_result.source_artifacts_seen != item_artifacts.len()
        || portfolio_result
            .eligible_candidates
            .len()
            .saturating_add(portfolio_result.rejected_candidates.len())
            == 0
    {
        return Err(format!(
            "{} phase portfolio feasibility did not preserve candidate evidence",
            phase.as_str()
        ));
    }
    let portfolio_value =
        serde_json::to_value(&portfolio_result).map_err(|error| error.to_string())?;
    if portfolio_value.get("target_weights").is_some()
        || portfolio_value.get("allocation_frontier").is_some()
    {
        return Err("portfolio feasibility unexpectedly emitted allocation output".to_string());
    }
    let portfolio_sha256 =
        sha256_bytes(&serde_json::to_vec(&portfolio_result).map_err(|error| error.to_string())?);

    let catalog = tauri::async_runtime::block_on(list_run_artifacts(app_handle.clone()))?;
    let mut catalog_artifacts = Vec::with_capacity(catalog.len());
    let mut unique_catalog_paths = BTreeSet::new();
    for artifact in catalog {
        let canonical = require_contained_file(&app_local_data_dir, Path::new(&artifact.path))?;
        if !unique_catalog_paths.insert(canonical.clone()) {
            return Err("run artifact catalog contains duplicate paths".to_string());
        }
        catalog_artifacts.push(file_evidence(&canonical)?);
    }

    let evidence = PhaseEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: "vectorgrid_campaign_restart_phase",
        phase: phase.as_str(),
        process_id: std::process::id(),
        run_id: run_id.to_string(),
        app_identifier: app_identifier(run_id),
        app_local_data_dir: app_local_data_dir.to_string_lossy().to_string(),
        summary: file_evidence(&summary_path)?,
        durable_summary: loaded_summary,
        response_summary_matches_durable,
        response: serde_json::to_value(response).map_err(|error| error.to_string())?,
        progress_checkpoints,
        item_artifacts,
        catalog_artifacts,
        portfolio_result,
        portfolio_sha256,
    };
    write_json(output_json, &evidence)
}

fn mutate_dataset(
    run_root: &Path,
    run_id: &str,
    dataset: &Path,
    output_json: &Path,
) -> Result<(), String> {
    let dataset_root = run_root.join("datasets").canonicalize().map_err(|error| {
        format!(
            "canonicalize campaign dataset root {}: {error}",
            run_root.join("datasets").display()
        )
    })?;
    let dataset = require_contained_file(&dataset_root, dataset)?;
    let old_content_fingerprint = data_cache::content_fingerprint_for_source(&dataset)?;
    let old_sha256 = sha256_file(&dataset)?;
    let raw = fs::read_to_string(&dataset)
        .map_err(|error| format!("read mutation dataset {}: {error}", dataset.display()))?;
    let mut lines = raw.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.len() < 3 {
        return Err("mutation dataset must contain a header and at least two rows".to_string());
    }
    let header = lines[0].split(',').collect::<Vec<_>>();
    let close_index = header
        .iter()
        .position(|entry| entry.trim().eq_ignore_ascii_case("close"))
        .ok_or_else(|| "mutation dataset is missing a close column".to_string())?;
    let mut row = lines[1].split(',').map(str::to_string).collect::<Vec<_>>();
    let current_close = row
        .get(close_index)
        .ok_or_else(|| "mutation row is missing its close value".to_string())?
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("parse mutation close value: {error}"))?;
    row[close_index] = format!("{:.8}", current_close + 0.125);
    lines[1] = row.join(",");
    let mut replacement = lines.join("\n");
    replacement.push('\n');
    atomic_output::write_str(&dataset, &replacement)?;

    let new_content_fingerprint = data_cache::content_fingerprint_for_source(&dataset)?;
    let new_sha256 = sha256_file(&dataset)?;
    if old_content_fingerprint == new_content_fingerprint || old_sha256 == new_sha256 {
        return Err("dataset mutation did not change its durable identity".to_string());
    }
    let evidence = MutationEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: "vectorgrid_campaign_restart_mutation",
        process_id: std::process::id(),
        run_id: run_id.to_string(),
        dataset_path: dataset.to_string_lossy().to_string(),
        old_content_fingerprint,
        new_content_fingerprint,
        old_sha256,
        new_sha256,
    };
    write_json(output_json, &evidence)
}

fn repair_dataset(
    run_root: &Path,
    run_id: &str,
    dataset: &Path,
    source_dataset: &Path,
    output_json: &Path,
) -> Result<(), String> {
    let dataset_root = run_root.join("datasets").canonicalize().map_err(|error| {
        format!(
            "canonicalize campaign dataset root {}: {error}",
            run_root.join("datasets").display()
        )
    })?;
    let dataset = require_contained_file(&dataset_root, dataset)?;
    let source_dataset = require_contained_file(&dataset_root, source_dataset)?;
    if dataset == source_dataset {
        return Err("repair target and source dataset must be different files".to_string());
    }
    let old_content_fingerprint = data_cache::content_fingerprint_for_source(&dataset)?;
    let old_sha256 = sha256_file(&dataset)?;
    let source_bytes = fs::read(&source_dataset).map_err(|error| {
        format!(
            "read repair source dataset {}: {error}",
            source_dataset.display()
        )
    })?;
    let source_sha256 = sha256_bytes(&source_bytes);
    atomic_output::write_bytes(&dataset, &source_bytes)?;
    let new_content_fingerprint = data_cache::content_fingerprint_for_source(&dataset)?;
    let new_sha256 = sha256_file(&dataset)?;
    if old_content_fingerprint == new_content_fingerprint
        || old_sha256 == new_sha256
        || new_sha256 != source_sha256
    {
        return Err(
            "dataset repair did not replace the injected failure with source bytes".to_string(),
        );
    }
    let evidence = RepairEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: "vectorgrid_campaign_restart_repair",
        process_id: std::process::id(),
        run_id: run_id.to_string(),
        dataset_path: dataset.to_string_lossy().to_string(),
        source_dataset_path: source_dataset.to_string_lossy().to_string(),
        old_content_fingerprint,
        new_content_fingerprint,
        old_sha256,
        new_sha256,
        source_sha256,
    };
    write_json(output_json, &evidence)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read(path).map_err(|error| format!("read JSON {}: {error}", path.display()))?;
    serde_json::from_slice(&raw).map_err(|error| format!("parse JSON {}: {error}", path.display()))
}

fn required_text<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|entry| !entry.trim().is_empty())
        .ok_or_else(|| format!("missing required text at {pointer}"))
}

fn probe_corrupt(
    run_root: &Path,
    run_id: &str,
    phase_json: &Path,
    output_json: &Path,
) -> Result<(), String> {
    let phase_json = require_contained_file(run_root, phase_json)?;
    let phase = read_json(&phase_json)?;
    if required_text(&phase, "/run_id")? != run_id {
        return Err("corruption probe phase belongs to another run".to_string());
    }
    let app = build_isolated_app(run_id)?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = verify_isolated_app_dir(
        &app_handle
            .path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?,
        run_id,
    )?;
    let valid_summary = require_contained_file(
        &app_local_data_dir,
        Path::new(required_text(&phase, "/summary/path")?),
    )?;
    let valid = tauri::async_runtime::block_on(load_artifact(
        valid_summary.to_string_lossy().to_string(),
        app_handle.clone(),
    ))?;
    let runs_dir = valid_summary
        .parent()
        .ok_or_else(|| "valid campaign summary has no parent directory".to_string())?;
    let malformed_path = runs_dir.join("vectorgrid_campaign_summary_malformed_probe.json");
    let tampered_path = runs_dir.join("vectorgrid_campaign_summary_tampered_probe.json");
    atomic_output::write_str(&malformed_path, "{\"version\":2,")?;
    let malformed_error = tauri::async_runtime::block_on(load_artifact(
        malformed_path.to_string_lossy().to_string(),
        app_handle.clone(),
    ))
    .expect_err("malformed campaign summary must fail closed");

    let mut tampered = valid;
    tampered["request"]["optimization_template"]["qa_tamper"] = json!(true);
    write_json(&tampered_path, &tampered)?;
    let tampered_error = tauri::async_runtime::block_on(load_artifact(
        tampered_path.to_string_lossy().to_string(),
        app_handle,
    ))
    .expect_err("manifest-tampered campaign summary must fail closed");
    if !tampered_error.contains("integrity manifest") {
        return Err(format!(
            "manifest tamper failed for an unexpected reason: {tampered_error}"
        ));
    }

    fs::remove_file(&malformed_path).map_err(|error| format!("remove malformed probe: {error}"))?;
    fs::remove_file(&tampered_path).map_err(|error| format!("remove tampered probe: {error}"))?;
    let evidence = CorruptionProbeEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: "vectorgrid_campaign_restart_corruption_probe",
        process_id: std::process::id(),
        run_id: run_id.to_string(),
        valid_summary_loaded: true,
        malformed_json_rejected: true,
        malformed_json_error: malformed_error,
        manifest_tamper_rejected: true,
        manifest_tamper_error: tampered_error,
        probe_files_removed: !malformed_path.exists() && !tampered_path.exists(),
    };
    write_json(output_json, &evidence)
}

fn phase_map(
    run_root: &Path,
    run_id: &str,
    phase_jsons: &[PathBuf],
) -> Result<BTreeMap<String, Value>, String> {
    let mut phases = BTreeMap::new();
    for path in phase_jsons {
        let path = require_contained_file(run_root, path)?;
        let phase = read_json(&path)?;
        if required_text(&phase, "/kind")? != "vectorgrid_campaign_restart_phase"
            || required_text(&phase, "/run_id")? != run_id
        {
            return Err(format!(
                "phase evidence does not belong to this campaign QA run: {}",
                path.display()
            ));
        }
        let name = required_text(&phase, "/phase")?.to_string();
        if phases.insert(name.clone(), phase).is_some() {
            return Err(format!("duplicate phase evidence for {name}"));
        }
    }
    for expected in [
        "interrupted",
        "resume",
        "noop",
        "stale_resume",
        "stale_noop",
        "partial_failure",
        "partial_retry",
        "large_catalog",
        "large_noop",
    ] {
        if !phases.contains_key(expected) {
            return Err(format!("missing phase evidence for {expected}"));
        }
    }
    Ok(phases)
}

fn value_strings(value: &Value, pointer: &str, field: &str) -> Result<Vec<String>, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array at {pointer}"))?
        .iter()
        .map(|entry| {
            entry
                .get(field)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("missing {field} in {pointer}"))
        })
        .collect()
}

fn artifact_paths(value: &Value) -> Result<Vec<String>, String> {
    value
        .pointer("/item_artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| "phase evidence is missing item_artifacts".to_string())?
        .iter()
        .map(|entry| {
            entry
                .pointer("/artifact/path")
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| "phase item artifact is missing path".to_string())
        })
        .collect()
}

fn source_fingerprints(value: &Value) -> Result<Vec<String>, String> {
    value_strings(value, "/item_artifacts", "source_content_fingerprint")
}

fn catalog_count(value: &Value) -> Result<usize, String> {
    value
        .get("catalog_artifacts")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| "phase evidence is missing catalog_artifacts".to_string())
}

fn item_artifact_indices(value: &Value) -> Result<Vec<usize>, String> {
    value
        .get("item_artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| "phase evidence is missing item_artifacts".to_string())?
        .iter()
        .map(|entry| {
            entry
                .get("item_index")
                .and_then(Value::as_u64)
                .map(|index| index as usize)
                .ok_or_else(|| "phase item artifact is missing item_index".to_string())
        })
        .collect()
}

fn checkpoint_indices(value: &Value) -> Result<Vec<usize>, String> {
    value
        .get("progress_checkpoints")
        .and_then(Value::as_array)
        .ok_or_else(|| "phase evidence is missing progress_checkpoints".to_string())?
        .iter()
        .map(|entry| {
            entry
                .get("dataset_index")
                .and_then(Value::as_u64)
                .map(|index| index as usize)
                .ok_or_else(|| "progress checkpoint is missing dataset_index".to_string())
        })
        .collect()
}

fn verify_referenced_file_hashes(phases: &BTreeMap<String, Value>) -> Result<bool, String> {
    for phase in phases.values() {
        let summary_path = Path::new(required_text(phase, "/summary/path")?);
        let summary_hash = required_text(phase, "/summary/sha256")?;
        if sha256_file(summary_path)? != summary_hash {
            return Ok(false);
        }
        let items = phase
            .get("item_artifacts")
            .and_then(Value::as_array)
            .ok_or_else(|| "phase evidence is missing item_artifacts".to_string())?;
        for item in items {
            let path = Path::new(required_text(item, "/artifact/path")?);
            let expected = required_text(item, "/artifact/sha256")?;
            if sha256_file(path)? != expected {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn verify_phases(
    run_root: &Path,
    run_id: &str,
    phase_jsons: Vec<PathBuf>,
    probe_json: &Path,
    mutation_json: &Path,
    repair_json: &Path,
    output_json: &Path,
) -> Result<(), String> {
    let phases = phase_map(run_root, run_id, &phase_jsons)?;
    let probe = read_json(&require_contained_file(run_root, probe_json)?)?;
    let mutation = read_json(&require_contained_file(run_root, mutation_json)?)?;
    let repair = read_json(&require_contained_file(run_root, repair_json)?)?;
    if required_text(&probe, "/run_id")? != run_id
        || required_text(&mutation, "/run_id")? != run_id
        || required_text(&repair, "/run_id")? != run_id
    {
        return Err("probe, mutation, or repair evidence belongs to another run".to_string());
    }

    let interrupted = phases.get("interrupted").expect("phase checked");
    let resume = phases.get("resume").expect("phase checked");
    let noop = phases.get("noop").expect("phase checked");
    let stale_resume = phases.get("stale_resume").expect("phase checked");
    let stale_noop = phases.get("stale_noop").expect("phase checked");
    let partial_failure = phases.get("partial_failure").expect("phase checked");
    let partial_retry = phases.get("partial_retry").expect("phase checked");
    let large_catalog = phases.get("large_catalog").expect("phase checked");
    let large_noop = phases.get("large_noop").expect("phase checked");

    let process_ids = phases
        .values()
        .map(|phase| {
            phase
                .get("process_id")
                .and_then(Value::as_u64)
                .map(|value| value as u32)
                .ok_or_else(|| "phase evidence is missing process_id".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if process_ids.iter().copied().collect::<BTreeSet<_>>().len() != process_ids.len() {
        return Err("campaign phases did not execute in distinct OS processes".to_string());
    }
    let repair_process_id = repair
        .get("process_id")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| "repair evidence is missing process_id".to_string())?;
    if process_ids.contains(&repair_process_id) {
        return Err(
            "partial-failure repair did not run outside the campaign phase processes".to_string(),
        );
    }
    if phases.values().any(|phase| {
        !phase
            .get("response_summary_matches_durable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }) {
        return Err(
            "a returned campaign response differed semantically from its durable summary"
                .to_string(),
        );
    }

    let initial_manifest =
        required_text(resume, "/response/summary/manifest/manifest_hash")?.to_string();
    if required_text(noop, "/response/summary/manifest/manifest_hash")? != initial_manifest
        || required_text(interrupted, "/response/summary/manifest/manifest_hash")?
            != initial_manifest
    {
        return Err("pre-mutation process restarts changed the campaign manifest".to_string());
    }
    let stale_manifest =
        required_text(stale_resume, "/response/summary/manifest/manifest_hash")?.to_string();
    if stale_manifest == initial_manifest
        || required_text(stale_noop, "/response/summary/manifest/manifest_hash")? != stale_manifest
    {
        return Err("dataset mutation did not produce one stable replacement manifest".to_string());
    }

    let resume_item_ids = value_strings(resume, "/item_artifacts", "item_id")?;
    let noop_item_ids = value_strings(noop, "/item_artifacts", "item_id")?;
    if resume_item_ids != noop_item_ids
        || resume_item_ids.iter().collect::<BTreeSet<_>>().len() != DATASET_COUNT
    {
        return Err(
            "no-op resume changed or duplicated stable campaign item identities".to_string(),
        );
    }
    let resume_paths = artifact_paths(resume)?;
    let noop_paths = artifact_paths(noop)?;
    if resume_paths != noop_paths
        || required_text(resume, "/portfolio_sha256")? != required_text(noop, "/portfolio_sha256")?
    {
        return Err("no-op resume mutated run-artifact or portfolio evidence".to_string());
    }

    let stale_paths = artifact_paths(stale_resume)?;
    let stale_noop_paths = artifact_paths(stale_noop)?;
    if stale_paths != stale_noop_paths
        || required_text(stale_resume, "/portfolio_sha256")?
            != required_text(stale_noop, "/portfolio_sha256")?
    {
        return Err("post-mutation no-op resume changed durable evidence".to_string());
    }
    if stale_paths.first() == resume_paths.first()
        || stale_paths.iter().skip(1).ne(resume_paths.iter().skip(1))
    {
        return Err(
            "stale resume did not replace exactly the mutated dataset artifact".to_string(),
        );
    }
    let resume_fingerprints = source_fingerprints(resume)?;
    let stale_fingerprints = source_fingerprints(stale_resume)?;
    if stale_fingerprints.first() == resume_fingerprints.first()
        || stale_fingerprints
            .iter()
            .skip(1)
            .ne(resume_fingerprints.iter().skip(1))
    {
        return Err(
            "stale resume did not replace exactly the mutated dataset source identity".to_string(),
        );
    }

    let partial_failure_statuses =
        value_strings(partial_failure, "/response/summary/items", "status")?;
    let partial_retry_statuses = value_strings(partial_retry, "/response/summary/items", "status")?;
    if partial_failure_statuses != ["optimized", "failed", "optimized"]
        || partial_retry_statuses != ["skipped", "optimized", "skipped"]
        || required_text(partial_failure, "/response/summary/state")? != "completed_with_failures"
        || required_text(partial_retry, "/response/summary/state")? != "completed"
    {
        return Err(
            "injected partial failure did not produce the expected failure and retry states"
                .to_string(),
        );
    }
    if item_artifact_indices(partial_failure)? != [0, 2]
        || item_artifact_indices(partial_retry)? != [0, 1, 2]
        || checkpoint_indices(partial_failure)? != [1, 3]
        || checkpoint_indices(partial_retry)? != [2]
    {
        return Err(
            "partial failure or retry did not preserve truthful artifact/progress indices"
                .to_string(),
        );
    }
    let partial_failure_paths = artifact_paths(partial_failure)?;
    let partial_retry_paths = artifact_paths(partial_retry)?;
    if partial_failure_paths.len() != 2
        || partial_retry_paths.len() != PARTIAL_FAILURE_DATASET_COUNT
        || partial_failure_paths.first() != partial_retry_paths.first()
        || partial_failure_paths.get(1) != partial_retry_paths.get(2)
        || partial_retry_paths.iter().collect::<BTreeSet<_>>().len()
            != PARTIAL_FAILURE_DATASET_COUNT
    {
        return Err(
            "partial retry did not reuse both successes and publish exactly one repaired artifact"
                .to_string(),
        );
    }
    let partial_failure_item_ids =
        value_strings(partial_failure, "/response/summary/items", "item_id")?;
    let partial_retry_item_ids =
        value_strings(partial_retry, "/response/summary/items", "item_id")?;
    if partial_failure_item_ids.len() != PARTIAL_FAILURE_DATASET_COUNT
        || partial_retry_item_ids.len() != PARTIAL_FAILURE_DATASET_COUNT
        || partial_failure_item_ids
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != PARTIAL_FAILURE_DATASET_COUNT
        || partial_retry_item_ids.iter().collect::<BTreeSet<_>>().len()
            != PARTIAL_FAILURE_DATASET_COUNT
        || partial_failure_item_ids
            .iter()
            .zip(&partial_retry_item_ids)
            .any(|(before, after)| before == after)
        || required_text(partial_failure, "/response/summary/manifest/manifest_hash")?
            == required_text(partial_retry, "/response/summary/manifest/manifest_hash")?
    {
        return Err(
            "partial failure/retry item identities were duplicated or not re-scoped after the manifest changed"
                .to_string(),
        );
    }
    let failed_item = partial_failure
        .pointer(&format!("/response/summary/items/{PARTIAL_FAILURE_INDEX}"))
        .ok_or_else(|| "partial failure evidence is missing the injected item".to_string())?;
    let retry_item = partial_retry
        .pointer(&format!("/response/summary/items/{PARTIAL_FAILURE_INDEX}"))
        .ok_or_else(|| "partial retry evidence is missing the repaired item".to_string())?;
    if failed_item
        .get("artifact_path")
        .is_some_and(|value| !value.is_null())
        || failed_item
            .get("error")
            .and_then(Value::as_str)
            .is_none_or(|error| !error.contains("invalid utf-8"))
        || failed_item.get("resume_state").and_then(Value::as_str) != Some("resumable")
        || failed_item.get("evidence_state").and_then(Value::as_str) != Some("partial")
        || failed_item.get("csv_path").and_then(Value::as_str)
            != Some(required_text(&repair, "/dataset_path")?)
        || retry_item
            .pointer("/dataset_identity/source_content_fingerprint")
            .and_then(Value::as_str)
            != Some(required_text(&repair, "/new_content_fingerprint")?)
        || required_text(&repair, "/old_content_fingerprint")?
            == required_text(&repair, "/new_content_fingerprint")?
        || required_text(&repair, "/old_sha256")? == required_text(&repair, "/new_sha256")?
        || required_text(&repair, "/new_sha256")? != required_text(&repair, "/source_sha256")?
    {
        return Err(
            "partial failure/repair evidence did not preserve the injected and repaired identities"
                .to_string(),
        );
    }
    let partial_failure_retry_passed = true;

    let large_item_ids = value_strings(large_catalog, "/item_artifacts", "item_id")?;
    let large_noop_item_ids = value_strings(large_noop, "/item_artifacts", "item_id")?;
    let large_paths = artifact_paths(large_catalog)?;
    let large_noop_paths = artifact_paths(large_noop)?;
    let large_csv_paths = value_strings(large_catalog, "/item_artifacts", "csv_path")?;
    let expected_large_checkpoints = (1..=LARGE_CATALOG_DATASET_COUNT).collect::<Vec<_>>();
    if large_item_ids.len() != LARGE_CATALOG_DATASET_COUNT
        || large_item_ids.iter().collect::<BTreeSet<_>>().len() != LARGE_CATALOG_DATASET_COUNT
        || large_paths.iter().collect::<BTreeSet<_>>().len() != LARGE_CATALOG_DATASET_COUNT
        || large_csv_paths.iter().collect::<BTreeSet<_>>().len() != LARGE_CATALOG_DATASET_COUNT
        || large_item_ids != large_noop_item_ids
        || large_paths != large_noop_paths
        || checkpoint_indices(large_catalog)? != expected_large_checkpoints
        || !checkpoint_indices(large_noop)?.is_empty()
        || required_text(large_catalog, "/portfolio_sha256")?
            != required_text(large_noop, "/portfolio_sha256")?
    {
        return Err(
            "bounded large-catalog identity, collision, progress, or no-op invariants failed"
                .to_string(),
        );
    }
    let large_catalog_identity_collision_progress_passed = true;

    if catalog_count(interrupted)? != 1
        || catalog_count(resume)? != DATASET_COUNT
        || catalog_count(noop)? != DATASET_COUNT
        || catalog_count(stale_resume)? != DATASET_COUNT + 1
        || catalog_count(stale_noop)? != DATASET_COUNT + 1
        || catalog_count(partial_failure)? != DATASET_COUNT + 1 + 2
        || catalog_count(partial_retry)? != DATASET_COUNT + 1 + PARTIAL_FAILURE_DATASET_COUNT
        || catalog_count(large_catalog)?
            != DATASET_COUNT + 1 + PARTIAL_FAILURE_DATASET_COUNT + LARGE_CATALOG_DATASET_COUNT
        || catalog_count(large_noop)?
            != DATASET_COUNT + 1 + PARTIAL_FAILURE_DATASET_COUNT + LARGE_CATALOG_DATASET_COUNT
    {
        return Err(
            "campaign artifact catalog growth did not match one publish per strict source identity"
                .to_string(),
        );
    }

    let stale_warnings = stale_resume
        .pointer("/response/summary/warnings")
        .and_then(Value::as_array)
        .ok_or_else(|| "stale resume is missing warnings".to_string())?;
    let stale_artifact_reuse_blocked = stale_warnings.iter().any(|warning| {
        warning
            .as_str()
            .is_some_and(|entry| entry.contains("classified as stale"))
    });
    if !stale_artifact_reuse_blocked {
        return Err("stale resume did not record its fail-closed reuse decision".to_string());
    }

    let corruption_probes_passed = probe
        .get("valid_summary_loaded")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && probe
            .get("malformed_json_rejected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && probe
            .get("manifest_tamper_rejected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && probe
            .get("probe_files_removed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    if !corruption_probes_passed {
        return Err("campaign corruption probes did not all pass".to_string());
    }

    let all_referenced_files_hash_verified = verify_referenced_file_hashes(&phases)?;
    if !all_referenced_files_hash_verified {
        return Err("a referenced campaign file changed after its phase completed".to_string());
    }
    let interrupted_paths = artifact_paths(interrupted)?;
    if interrupted_paths.len() != 1 || resume_paths.first() != interrupted_paths.first() {
        return Err(
            "resume did not preserve the one artifact published before cancellation".to_string(),
        );
    }

    let verification = VerificationEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        kind: "vectorgrid_campaign_restart_verification",
        passed: true,
        process_id: std::process::id(),
        run_id: run_id.to_string(),
        phase_process_ids: process_ids,
        dataset_count: DATASET_COUNT,
        partial_failure_dataset_count: PARTIAL_FAILURE_DATASET_COUNT,
        large_catalog_dataset_count: LARGE_CATALOG_DATASET_COUNT,
        process_restart_count: phases.len().saturating_sub(1),
        initial_manifest_hash: initial_manifest,
        stale_manifest_hash: stale_manifest,
        stable_item_ids_before_mutation: resume_item_ids,
        stable_artifact_paths_before_mutation: resume_paths,
        stable_artifact_paths_after_mutation: stale_paths,
        portfolio_hash_before_mutation: required_text(resume, "/portfolio_sha256")?.to_string(),
        portfolio_hash_after_mutation: required_text(stale_resume, "/portfolio_sha256")?
            .to_string(),
        corruption_probes_passed,
        stale_artifact_reuse_blocked,
        partial_failure_retry_passed,
        large_catalog_identity_collision_progress_passed,
        no_op_resume_artifact_bytes_stable: true,
        all_referenced_files_hash_verified,
        notes: vec![
            "Every campaign phase ran in a distinct OS process against one isolated app identifier."
                .to_string(),
            "The first process cancelled only after the production runner atomically published its first item checkpoint."
                .to_string(),
            "Portfolio output is feasibility-only; this harness does not create target weights or place orders."
                .to_string(),
            "An invalid-UTF-8 dataset produced one durable item failure; a separate repair process and retry reused both successful artifacts and executed only the repaired identity."
                .to_string(),
            format!(
                "A bounded {LARGE_CATALOG_DATASET_COUNT}-dataset campaign proved unique item IDs, artifact paths, CSV paths, ordered durable-checkpoint progress, and a separate-process no-op resume; no performance conclusion is drawn."
            ),
        ],
    };
    write_json(output_json, &verification)
}

fn cleanup(run_root: &Path, run_id: &str, output_json: &Path) -> Result<(), String> {
    let app = build_isolated_app(run_id)?;
    let app_handle = app.handle().clone();
    let app_local_data_dir = verify_isolated_app_dir(
        &app_handle
            .path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?,
        run_id,
    )?;
    drop(app_handle);
    drop(app);
    if app_local_data_dir.exists() {
        fs::remove_dir_all(&app_local_data_dir).map_err(|error| {
            format!(
                "remove isolated campaign QA app-data {}: {error}",
                app_local_data_dir.display()
            )
        })?;
    }
    write_json(
        output_json,
        &json!({
            "schema_version": EVIDENCE_SCHEMA_VERSION,
            "kind": "vectorgrid_campaign_restart_cleanup",
            "passed": !app_local_data_dir.exists(),
            "process_id": std::process::id(),
            "run_id": run_id,
            "removed_app_local_data_dir": app_local_data_dir,
            "run_root_preserved": run_root,
        }),
    )
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let run_id = normalize_run_id(&cli.run_id)?;
    let run_root = canonical_root(&cli.run_root)?;
    match cli.command {
        Command::Prepare {
            source_dataset,
            output_json,
        } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            prepare_datasets(&run_root, &run_id, &source_dataset, &output_json)
        }
        Command::Run {
            phase,
            datasets,
            output_json,
        } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            run_phase(&run_root, &run_id, phase, datasets, &output_json)
        }
        Command::Mutate {
            dataset,
            output_json,
        } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            mutate_dataset(&run_root, &run_id, &dataset, &output_json)
        }
        Command::Repair {
            dataset,
            source_dataset,
            output_json,
        } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            repair_dataset(&run_root, &run_id, &dataset, &source_dataset, &output_json)
        }
        Command::ProbeCorrupt {
            phase_json,
            output_json,
        } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            probe_corrupt(&run_root, &run_id, &phase_json, &output_json)
        }
        Command::Verify {
            phase_jsons,
            probe_json,
            mutation_json,
            repair_json,
            output_json,
        } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            verify_phases(
                &run_root,
                &run_id,
                phase_jsons,
                &probe_json,
                &mutation_json,
                &repair_json,
                &output_json,
            )
        }
        Command::Cleanup { output_json } => {
            let output_json = require_output_path(&run_root, &output_json)?;
            cleanup(&run_root, &run_id, &output_json)
        }
    }
}
