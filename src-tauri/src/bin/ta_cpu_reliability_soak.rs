use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use ta_app_core::progress::{DoubleMaProgressPayload, ProgressSink};
use ta_app_core::{
    Backend, DoubleMaRequest, DoubleMaValidationParams, DoubleMaValidationRequest,
    IterativeSimConfig, ValidationArtifactContext, ValidationEngine,
};
use ta_desktop_demo_app::commands::{
    load_run_artifact_hydration, rebuild_artifact_catalog_blocking, save_run_artifact_blocking,
};
use ta_desktop_demo_app::state::{AppState, BackgroundTaskState, LoadSource};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use tauri::Manager;

#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;
#[path = "../bin_support/benchmark_lock.rs"]
mod benchmark_lock;

const ISOLATION_PREFIX: &str = "vectorgrid-soak-";
const ISOLATION_MARKER: &str = ".vectorgrid-cpu-reliability-soak-owner.json";
const APP_DATA_MARKER: &str = ".vectorgrid-cpu-reliability-soak-app-data-owner.json";
const MIB: f64 = 1024.0 * 1024.0;
const MIN_OPERATOR_DURATION_SECONDS: u64 = 6 * 60 * 60;
const MIN_OPERATOR_MEASURED_CYCLES: usize = 30;
const MAX_OPERATOR_SAMPLE_GAP_MS: u64 = 15 * 60 * 1_000;
const MIN_RELIABILITY_ROWS: usize = 2_048;
const MIN_OPERATOR_ROWS: usize = 20_000;
const MIN_OPERATOR_VALIDATION_CANDIDATES: usize = 4;
const OPERATOR_CANCEL_EVERY: usize = 2;
const EXPECTED_OPTIMIZATION_COMBINATIONS: usize = 240;
const MAX_OWNED_STORAGE_GROWTH_BYTES: f64 = 64.0 * MIB;
const MAX_OWNED_STORAGE_SLOPE_BYTES_PER_CYCLE: f64 = 64.0 * 1024.0;
const MAX_OWNED_STORAGE_FILE_GROWTH: f64 = 4.0;
const MAX_OWNED_STORAGE_FILE_SLOPE_PER_CYCLE: f64 = 0.01;
const MAX_OWNED_STORAGE_DIRECTORY_GROWTH: f64 = 4.0;
const MAX_OWNED_STORAGE_DIRECTORY_SLOPE_PER_CYCLE: f64 = 0.01;
const COMPILED_TAURI_CONFIG: Option<&str> = option_env!("TAURI_CONFIG");
const COMPILED_TAURI_TARGET_TRIPLE: Option<&str> = option_env!("TAURI_ENV_TARGET_TRIPLE");
const COMPILED_RESOURCE_COMPILER_ATTESTATION: Option<&str> =
    option_env!("VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION");

#[derive(Parser, Debug)]
#[command(name = "ta_cpu_reliability_soak")]
#[command(
    about = "Offline CPU reliability soak over load/optimize/validate/artifact/reload/cancel paths"
)]
struct Cli {
    #[arg(long)]
    out: PathBuf,

    #[arg(long)]
    isolation_root: PathBuf,

    #[arg(long)]
    run_id: String,

    #[arg(long)]
    cargo_target_dir: PathBuf,

    #[arg(long)]
    build_profile: String,

    #[arg(long)]
    cargo_build_target_env: String,

    #[arg(long)]
    cargo_target_triple: String,

    #[arg(long)]
    cargo_artifact_path: PathBuf,

    #[arg(long, action = clap::ArgAction::Set)]
    cargo_artifact_emitted: bool,

    #[arg(long, action = clap::ArgAction::Set)]
    cargo_artifact_fresh: bool,

    #[arg(long)]
    binary_sha256: String,

    #[arg(long)]
    source_manifest: PathBuf,

    #[arg(long)]
    source_manifest_sha256: String,

    #[arg(long)]
    launcher_manifest: PathBuf,

    #[arg(long)]
    launcher_manifest_sha256: String,

    #[arg(long, default_value_t = false)]
    fresh_build: bool,

    #[arg(long, default_value_t = 0)]
    recovered_stale_isolations: usize,

    #[arg(long, default_value = "short")]
    mode_label: String,

    #[arg(long, default_value_t = 4)]
    cycles: usize,

    #[arg(long, default_value_t = 0)]
    duration_seconds: u64,

    #[arg(long, default_value_t = 2)]
    cancel_every: usize,

    #[arg(long, default_value_t = 4_000)]
    rows: usize,

    #[arg(long, default_value_t = 4)]
    validation_candidates: usize,

    #[arg(long, default_value_t = 1)]
    warmup_cycles: usize,

    #[arg(long, default_value_t = 128)]
    retained_samples: usize,

    #[arg(long, default_value_t = 32)]
    retained_cycles: usize,

    #[arg(long, default_value_t = 0)]
    allowed_failures: usize,

    #[arg(long, default_value_t = 8.0)]
    max_private_slope_mib_per_cycle: f64,

    #[arg(long, default_value_t = 128.0)]
    max_private_growth_mib: f64,

    #[arg(long, default_value_t = 8.0)]
    max_working_set_slope_mib_per_cycle: f64,

    #[arg(long, default_value_t = 128.0)]
    max_working_set_growth_mib: f64,

    #[arg(long, default_value_t = 1.0)]
    max_handle_slope_per_cycle: f64,

    #[arg(long, default_value_t = 8)]
    max_handle_growth: u32,

    #[arg(long, default_value_t = 0)]
    max_catalog_growth: usize,
}

#[derive(Debug, Clone)]
struct ValidatedConfig {
    out: PathBuf,
    isolation_root: PathBuf,
    run_id: String,
    cargo_target_dir: PathBuf,
    build_profile: String,
    cargo_build_target_env: String,
    cargo_target_triple: String,
    cargo_artifact_path: PathBuf,
    cargo_artifact_emitted: bool,
    cargo_artifact_fresh: bool,
    binary_sha256: String,
    source_manifest: PathBuf,
    source_manifest_sha256: String,
    launcher_manifest: PathBuf,
    launcher_manifest_sha256: String,
    fresh_build: bool,
    recovered_stale_isolations: usize,
    mode_label: String,
    cycles: usize,
    duration_seconds: u64,
    cancel_every: usize,
    rows: usize,
    validation_candidates: usize,
    warmup_cycles: usize,
    retained_samples: usize,
    retained_cycles: usize,
    allowed_failures: usize,
    thresholds: LeakThresholds,
}

impl ValidatedConfig {
    fn from_cli(cli: Cli) -> Result<Self, String> {
        let out = absolute_lexical_path(&cli.out)?;
        let isolation_root = absolute_lexical_path(&cli.isolation_root)?;
        let cargo_target_dir = absolute_lexical_path(&cli.cargo_target_dir)?;
        let cargo_artifact_path = absolute_lexical_path(&cli.cargo_artifact_path)?;
        let source_manifest = absolute_lexical_path(&cli.source_manifest)?;
        let launcher_manifest = absolute_lexical_path(&cli.launcher_manifest)?;
        validate_isolation_root_shape(&isolation_root)?;

        if out.starts_with(&isolation_root) {
            return Err(
                "out must be outside isolation_root so evidence survives safe cleanup".to_string(),
            );
        }
        if source_manifest.starts_with(&isolation_root) {
            return Err("source_manifest must be outside isolation_root".to_string());
        }
        if launcher_manifest.starts_with(&isolation_root) {
            return Err("launcher_manifest must be outside isolation_root".to_string());
        }
        let run_id = cli.run_id.trim().to_string();
        if run_id.len() < 8
            || run_id.len() > 96
            || !run_id
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_'))
        {
            return Err(
                "run_id must be 8..=96 ASCII alphanumeric, '-' or '_' characters".to_string(),
            );
        }
        let build_profile = cli.build_profile.trim().to_ascii_lowercase();
        if !matches!(build_profile.as_str(), "release" | "dev") {
            return Err("build_profile must be 'release' or 'dev'".to_string());
        }
        let cargo_build_target_env = cli.cargo_build_target_env.trim().to_string();
        if cargo_build_target_env != "host-default" {
            return Err(
                "cargo_build_target_env must be host-default; the explicit target triple is CLI-pinned"
                    .to_string(),
            );
        }
        let cargo_target_triple = cli.cargo_target_triple.trim().to_string();
        if cargo_target_triple.len() < 3
            || cargo_target_triple.len() > 128
            || !cargo_target_triple
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_' | b'.'))
        {
            return Err("cargo_target_triple is invalid".to_string());
        }
        if COMPILED_TAURI_TARGET_TRIPLE != Some(cargo_target_triple.as_str()) {
            return Err(format!(
                "compiled Tauri target triple mismatch: expected {cargo_target_triple}, actual {}",
                COMPILED_TAURI_TARGET_TRIPLE.unwrap_or("<missing>")
            ));
        }
        if COMPILED_TAURI_CONFIG.is_some() {
            return Err("compiled Tauri context used a TAURI_CONFIG override".to_string());
        }
        let binary_sha256 = validated_sha256(&cli.binary_sha256, "binary_sha256")?;
        let source_manifest_sha256 =
            validated_sha256(&cli.source_manifest_sha256, "source_manifest_sha256")?;
        let launcher_manifest_sha256 =
            validated_sha256(&cli.launcher_manifest_sha256, "launcher_manifest_sha256")?;
        let mode_label = cli.mode_label.trim().to_ascii_lowercase();
        if !matches!(mode_label.as_str(), "short" | "operator") {
            return Err("mode_label must be 'short' or 'operator'".to_string());
        }
        if mode_label == "short" && cli.duration_seconds != 0 {
            return Err("short mode must use duration_seconds=0".to_string());
        }
        if mode_label == "operator"
            && (cli.duration_seconds < MIN_OPERATOR_DURATION_SECONDS
                || !cli.fresh_build
                || build_profile != "release"
                || !cli.cargo_artifact_emitted
                || cli.cargo_artifact_fresh
                || cli.allowed_failures != 0)
        {
            return Err(
                "operator authority requires a fresh release build, at least six hours, and zero allowed failures"
                    .to_string(),
            );
        }
        if cli.fresh_build && cli.cargo_artifact_fresh {
            return Err(
                "fresh_build cannot use a Cargo artifact reported as cached/fresh".to_string(),
            );
        }
        if cli.cycles == 0 {
            return Err("cycles must be > 0".to_string());
        }
        if cli.cancel_every == 0 {
            return Err(
                "cancel_every must be > 0; the soak must exercise cancellation".to_string(),
            );
        }
        if cli.rows < MIN_RELIABILITY_ROWS {
            return Err(format!(
                "rows must be >= {MIN_RELIABILITY_ROWS} so the cancellation workload spans multiple CPU tiles"
            ));
        }
        if cli.validation_candidates == 0 {
            return Err("validation_candidates must be > 0".to_string());
        }
        if cli.retained_samples < 4 {
            return Err("retained_samples must be >= 4".to_string());
        }
        if cli.retained_cycles == 0 {
            return Err("retained_cycles must be > 0".to_string());
        }
        if cli.warmup_cycles.saturating_add(3) > cli.cycles {
            return Err("warmup_cycles must leave at least three measured cycles".to_string());
        }
        if cli.cancel_every > cli.cycles {
            return Err("cancel_every must schedule at least one cancellation cycle".to_string());
        }

        let finite_non_negative = [
            (
                "max_private_slope_mib_per_cycle",
                cli.max_private_slope_mib_per_cycle,
            ),
            ("max_private_growth_mib", cli.max_private_growth_mib),
            (
                "max_working_set_slope_mib_per_cycle",
                cli.max_working_set_slope_mib_per_cycle,
            ),
            ("max_working_set_growth_mib", cli.max_working_set_growth_mib),
            ("max_handle_slope_per_cycle", cli.max_handle_slope_per_cycle),
        ];
        for (name, value) in finite_non_negative {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("{name} must be finite and >= 0"));
            }
        }
        if mode_label == "operator"
            && (cli.rows < MIN_OPERATOR_ROWS
                || cli.validation_candidates < MIN_OPERATOR_VALIDATION_CANDIDATES
                || cli.cancel_every != OPERATOR_CANCEL_EVERY
                || cli.warmup_cycles != 1
                || cli.max_private_slope_mib_per_cycle > 0.5
                || cli.max_private_growth_mib > 128.0
                || cli.max_working_set_slope_mib_per_cycle > 0.5
                || cli.max_working_set_growth_mib > 128.0
                || cli.max_handle_slope_per_cycle > 0.10
                || cli.max_handle_growth > 8
                || cli.max_catalog_growth != 0)
        {
            return Err(
                "operator authority requires the canonical workload and leak thresholds"
                    .to_string(),
            );
        }

        Ok(Self {
            out,
            isolation_root,
            run_id,
            cargo_target_dir,
            build_profile,
            cargo_build_target_env,
            cargo_target_triple,
            cargo_artifact_path,
            cargo_artifact_emitted: cli.cargo_artifact_emitted,
            cargo_artifact_fresh: cli.cargo_artifact_fresh,
            binary_sha256,
            source_manifest,
            source_manifest_sha256,
            launcher_manifest,
            launcher_manifest_sha256,
            fresh_build: cli.fresh_build,
            recovered_stale_isolations: cli.recovered_stale_isolations,
            mode_label,
            cycles: cli.cycles,
            duration_seconds: cli.duration_seconds,
            cancel_every: cli.cancel_every,
            rows: cli.rows,
            validation_candidates: cli.validation_candidates,
            warmup_cycles: cli.warmup_cycles,
            retained_samples: cli.retained_samples,
            retained_cycles: cli.retained_cycles,
            allowed_failures: cli.allowed_failures,
            thresholds: LeakThresholds {
                max_private_slope_bytes_per_cycle: cli.max_private_slope_mib_per_cycle * MIB,
                max_private_growth_bytes: cli.max_private_growth_mib * MIB,
                max_working_set_slope_bytes_per_cycle: cli.max_working_set_slope_mib_per_cycle
                    * MIB,
                max_working_set_growth_bytes: cli.max_working_set_growth_mib * MIB,
                max_handle_slope_per_cycle: cli.max_handle_slope_per_cycle,
                max_handle_growth: cli.max_handle_growth,
                max_catalog_growth: cli.max_catalog_growth,
            },
        })
    }
}

fn validated_sha256(value: &str, name: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.bytes().all(|value| value.is_ascii_hexdigit()) {
        return Err(format!("{name} must be exactly 64 hexadecimal characters"));
    }
    Ok(normalized)
}

#[derive(Debug, Clone, Copy)]
struct LeakThresholds {
    max_private_slope_bytes_per_cycle: f64,
    max_private_growth_bytes: f64,
    max_working_set_slope_bytes_per_cycle: f64,
    max_working_set_growth_bytes: f64,
    max_handle_slope_per_cycle: f64,
    max_handle_growth: u32,
    max_catalog_growth: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ProcessSample {
    cycle: usize,
    stage: String,
    elapsed_ms: u64,
    working_set_bytes: Option<u64>,
    private_bytes: Option<u64>,
    handle_count: Option<u32>,
}

#[derive(Debug)]
struct SampleWindow {
    total: usize,
    capacity: usize,
    first: Option<ProcessSample>,
    recent: VecDeque<ProcessSample>,
}

impl SampleWindow {
    fn new(capacity: usize) -> Self {
        Self {
            total: 0,
            capacity,
            first: None,
            recent: VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, sample: ProcessSample) {
        self.total = self.total.saturating_add(1);
        if self.first.is_none() {
            self.first = Some(sample.clone());
        }
        self.recent.push_back(sample);
        while self.recent.len() > self.capacity {
            self.recent.pop_front();
        }
    }

    fn retained(&self) -> Vec<ProcessSample> {
        let mut out = Vec::with_capacity(self.recent.len().saturating_add(1));
        if let Some(first) = self.first.as_ref() {
            if self
                .recent
                .front()
                .is_none_or(|recent_first| recent_first.elapsed_ms != first.elapsed_ms)
            {
                out.push(first.clone());
            }
        }
        out.extend(self.recent.iter().cloned());
        out
    }
}

#[derive(Debug, Default)]
struct TrendAccumulator {
    expected_samples: usize,
    measured_samples: usize,
    first: Option<f64>,
    last: Option<f64>,
    mean_x: f64,
    mean_y: f64,
    sum_squares_x: f64,
    sum_products_xy: f64,
}

impl TrendAccumulator {
    fn push(&mut self, cycle: usize, value: Option<f64>) {
        self.expected_samples = self.expected_samples.saturating_add(1);
        let Some(value) = value.filter(|value| value.is_finite()) else {
            return;
        };

        if self.first.is_none() {
            self.first = Some(value);
        }
        self.last = Some(value);
        self.measured_samples = self.measured_samples.saturating_add(1);
        let n = self.measured_samples as f64;
        let x = cycle as f64;
        let delta_x = x - self.mean_x;
        let delta_y = value - self.mean_y;
        self.mean_x += delta_x / n;
        self.mean_y += delta_y / n;
        self.sum_squares_x += delta_x * (x - self.mean_x);
        self.sum_products_xy += delta_x * (value - self.mean_y);
    }

    fn slope(&self) -> Option<f64> {
        if self.measured_samples < 2 || self.sum_squares_x.abs() <= f64::EPSILON {
            None
        } else {
            Some(self.sum_products_xy / self.sum_squares_x)
        }
    }
}

#[derive(Debug, Default)]
struct MonitoringTracker {
    working_set: TrendAccumulator,
    private_bytes: TrendAccumulator,
    handles: TrendAccumulator,
    cycle_complete_samples: usize,
    prior_cycle_elapsed_ms: Option<u64>,
    max_cycle_sample_gap_ms: u64,
}

impl MonitoringTracker {
    fn push(&mut self, sample: &ProcessSample, warmup_cycles: usize) {
        if sample.stage != "cycle_complete" {
            return;
        }
        let prior_elapsed_ms = self.prior_cycle_elapsed_ms.unwrap_or(0);
        self.max_cycle_sample_gap_ms = self
            .max_cycle_sample_gap_ms
            .max(sample.elapsed_ms.saturating_sub(prior_elapsed_ms));
        self.prior_cycle_elapsed_ms = Some(sample.elapsed_ms);
        self.cycle_complete_samples = self.cycle_complete_samples.saturating_add(1);
        if sample.cycle <= warmup_cycles {
            return;
        }
        self.working_set.push(
            sample.cycle,
            sample.working_set_bytes.map(|value| value as f64),
        );
        self.private_bytes
            .push(sample.cycle, sample.private_bytes.map(|value| value as f64));
        self.handles
            .push(sample.cycle, sample.handle_count.map(|value| value as f64));
    }

    fn finish(&mut self, elapsed_ms: u64) {
        let prior_elapsed_ms = self.prior_cycle_elapsed_ms.unwrap_or(0);
        self.max_cycle_sample_gap_ms = self
            .max_cycle_sample_gap_ms
            .max(elapsed_ms.saturating_sub(prior_elapsed_ms));
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct OwnedStorageStats {
    file_count: u64,
    directory_count: u64,
    total_bytes: u64,
    residual_artifact_or_temp_files: u64,
}

#[derive(Debug, Default)]
struct OwnedStorageTracker {
    sample_count_total: usize,
    file_count: TrendAccumulator,
    directory_count: TrendAccumulator,
    total_bytes: TrendAccumulator,
    max_residual_artifact_or_temp_files: u64,
}

impl OwnedStorageTracker {
    fn push(&mut self, cycle: usize, stats: OwnedStorageStats, warmup_cycles: usize) {
        self.sample_count_total = self.sample_count_total.saturating_add(1);
        self.max_residual_artifact_or_temp_files = self
            .max_residual_artifact_or_temp_files
            .max(stats.residual_artifact_or_temp_files);
        if cycle <= warmup_cycles {
            return;
        }
        self.file_count.push(cycle, Some(stats.file_count as f64));
        self.directory_count
            .push(cycle, Some(stats.directory_count as f64));
        self.total_bytes.push(cycle, Some(stats.total_bytes as f64));
    }
}

#[derive(Debug, Clone, Serialize)]
struct CycleSummary {
    cycle: usize,
    status: String,
    wall_ms: u64,
    loaded_data_id: String,
    load_source: String,
    loaded_rows: usize,
    optimization_combinations: usize,
    validation_candidate_count: usize,
    cancellation_probes: usize,
    cancellation_partial_processed: usize,
    cancellation_partial_total: usize,
    artifact_sha256: String,
    artifact_bytes: usize,
    catalog_before: usize,
    catalog_after_save: usize,
    catalog_after_cleanup: usize,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LeakMetricAssessment {
    supported: bool,
    complete: bool,
    expected_samples: usize,
    measured_samples: usize,
    first: Option<f64>,
    last: Option<f64>,
    growth: Option<f64>,
    slope_per_cycle: Option<f64>,
    max_growth: f64,
    max_slope_per_cycle: f64,
    passed: bool,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct MonitoringAssessment {
    sample_count_total: usize,
    retained_sample_count: usize,
    cycle_complete_sample_count: usize,
    max_cycle_sample_gap_ms: u64,
    warmup_cycles_excluded: usize,
    working_set: LeakMetricAssessment,
    private_bytes: LeakMetricAssessment,
    handles: LeakMetricAssessment,
    owned_storage: OwnedStorageAssessment,
}

#[derive(Debug, Clone, Serialize)]
struct OwnedStorageAssessment {
    sample_count_total: usize,
    max_residual_artifact_or_temp_files: u64,
    file_count: LeakMetricAssessment,
    directory_count: LeakMetricAssessment,
    total_bytes: LeakMetricAssessment,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct SoakSummary {
    schema_version: u32,
    kind: String,
    run_id: String,
    status: String,
    mode_label: String,
    started_at: String,
    completed_at: String,
    elapsed_ms: u64,
    timing_boundary: String,
    provenance: Value,
    termination: Value,
    authority: Value,
    offline_contract: Value,
    config: Value,
    counts: Value,
    catalog: Value,
    artifact_integrity: Value,
    cancellation: Value,
    managed_state: Value,
    monitoring: MonitoringAssessment,
    retained_samples: Vec<ProcessSample>,
    retained_cycles: Vec<CycleSummary>,
    failures: Vec<String>,
    isolation: Value,
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn ensure_no_reparse_ancestors(path: &Path, context: &str) -> Result<(), String> {
    let mut cursor = Some(path);
    while let Some(candidate) = cursor {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) if metadata_is_reparse(&metadata) => {
                return Err(format!(
                    "{context} must not use a reparse/symlink path or ancestor: {}",
                    candidate.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        cursor = candidate.parent();
    }
    Ok(())
}

fn ensure_no_reparse_tree(root: &Path, context: &str) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata_is_reparse(&metadata) {
            return Err(format!(
                "{context} contains a reparse/symlink entry: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(&path).map_err(|error| error.to_string())? {
                pending.push(entry.map_err(|error| error.to_string())?.path());
            }
        }
    }
    Ok(())
}

struct IsolationGuard {
    root: PathBuf,
    marker_nonce: String,
    app_data_root: Option<PathBuf>,
    armed: bool,
}

impl IsolationGuard {
    fn create(root: &Path) -> Result<Self, String> {
        validate_isolation_root_shape(root)?;
        ensure_no_reparse_ancestors(root, "isolation_root")?;
        let parent = root
            .parent()
            .ok_or_else(|| "isolation_root must have a parent".to_string())?;
        let canonical_parent = fs::canonicalize(parent).map_err(|error| error.to_string())?;
        if root.exists() {
            let mut entries = fs::read_dir(root).map_err(|e| e.to_string())?;
            if entries
                .next()
                .transpose()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err(format!(
                    "isolation_root must be absent or empty: {}",
                    root.display()
                ));
            }
        } else {
            fs::create_dir_all(root).map_err(|e| e.to_string())?;
        }
        ensure_no_reparse_ancestors(root, "isolation_root")?;
        let canonical_root = fs::canonicalize(root).map_err(|error| error.to_string())?;
        if canonical_root.parent() != Some(canonical_parent.as_path()) {
            return Err(format!(
                "resolved isolation_root escaped its direct parent: {}",
                canonical_root.display()
            ));
        }

        let marker_nonce = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let marker = json!({
            "kind": "vectorgrid_cpu_reliability_soak_isolation",
            "schema_version": 1,
            "pid": std::process::id(),
            "nonce": marker_nonce,
        });
        atomic_output::write_str(
            &canonical_root.join(ISOLATION_MARKER),
            &serde_json::to_string_pretty(&marker).map_err(|e| e.to_string())?,
        )?;
        ensure_no_reparse_tree(&canonical_root, "isolation_root")?;
        Ok(Self {
            // Keep the normalized lexical spelling supplied by the validated
            // CLI. Windows canonicalization commonly adds a `\\?\` prefix;
            // retaining it here would break lexical descendant checks against
            // Tauri's ordinary `C:\...` app-data path spelling.
            root: root.to_path_buf(),
            marker_nonce,
            app_data_root: None,
            armed: true,
        })
    }

    fn configure_process_environment(&self) -> Result<(), String> {
        ensure_no_reparse_tree(&self.root, "isolation_root")?;
        let local_app_data = self.root.join("local-app-data");
        let roaming_app_data = self.root.join("roaming-app-data");
        let temp = self.root.join("temp");
        for dir in [&local_app_data, &roaming_app_data, &temp] {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        ensure_no_reparse_tree(&self.root, "isolation_root")?;
        std::env::set_var("LOCALAPPDATA", &local_app_data);
        std::env::set_var("APPDATA", &roaming_app_data);
        std::env::set_var("TEMP", &temp);
        std::env::set_var("TMP", &temp);
        Ok(())
    }

    fn register_app_data_root(&mut self, app_data_root: &Path) -> Result<(), String> {
        validate_soak_app_data_root_shape(app_data_root)?;
        let canonical_isolation_root =
            fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let lexical_app_data_root = absolute_lexical_path(app_data_root)?;
        if lexical_app_data_root == self.root || !lexical_app_data_root.starts_with(&self.root) {
            return Err(format!(
                "soak app data root must be a strict descendant of isolation_root: {}",
                app_data_root.display()
            ));
        }
        fs::create_dir_all(app_data_root).map_err(|e| e.to_string())?;
        ensure_no_reparse_ancestors(app_data_root, "soak app data root")?;
        let canonical_app_data_root =
            fs::canonicalize(app_data_root).map_err(|error| error.to_string())?;
        if canonical_app_data_root == canonical_isolation_root
            || !canonical_app_data_root.starts_with(&canonical_isolation_root)
        {
            return Err(format!(
                "resolved soak app data root escaped isolation_root: {}",
                canonical_app_data_root.display()
            ));
        }
        let marker = json!({
            "kind": "vectorgrid_cpu_reliability_soak_app_data",
            "schema_version": 1,
            "pid": std::process::id(),
            "nonce": self.marker_nonce,
        });
        atomic_output::write_str(
            &canonical_app_data_root.join(APP_DATA_MARKER),
            &serde_json::to_string_pretty(&marker).map_err(|e| e.to_string())?,
        )?;
        ensure_no_reparse_tree(&canonical_app_data_root, "soak app data root")?;
        self.app_data_root = Some(canonical_app_data_root);
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), String> {
        if let Some(app_data_root) = self.app_data_root.as_deref() {
            cleanup_owned_app_data_root(app_data_root, &self.marker_nonce)?;
        }
        cleanup_owned_isolation_root(&self.root, &self.marker_nonce)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for IsolationGuard {
    fn drop(&mut self) {
        if self.armed {
            if let Some(app_data_root) = self.app_data_root.as_deref() {
                let _ = cleanup_owned_app_data_root(app_data_root, &self.marker_nonce);
            }
            let _ = cleanup_owned_isolation_root(&self.root, &self.marker_nonce);
        }
    }
}

struct CancelOnPartialProgress<'a> {
    cancel: &'a AtomicBool,
    events: AtomicUsize,
    partial_progress_observed: AtomicBool,
    processed_at_cancel: AtomicUsize,
    total_at_cancel: AtomicUsize,
}

impl<'a> CancelOnPartialProgress<'a> {
    fn new(cancel: &'a AtomicBool) -> Self {
        Self {
            cancel,
            events: AtomicUsize::new(0),
            partial_progress_observed: AtomicBool::new(false),
            processed_at_cancel: AtomicUsize::new(0),
            total_at_cancel: AtomicUsize::new(0),
        }
    }

    fn event_count(&self) -> usize {
        self.events.load(Ordering::Relaxed)
    }

    fn partial_progress(&self) -> Option<(usize, usize)> {
        self.partial_progress_observed
            .load(Ordering::Relaxed)
            .then(|| {
                (
                    self.processed_at_cancel.load(Ordering::Relaxed),
                    self.total_at_cancel.load(Ordering::Relaxed),
                )
            })
    }
}

impl ProgressSink for CancelOnPartialProgress<'_> {
    fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload) {
        self.events.fetch_add(1, Ordering::Relaxed);
        if payload.processed_pairs > 0
            && payload.processed_pairs < payload.total_pairs
            && !self.partial_progress_observed.swap(true, Ordering::Relaxed)
        {
            self.processed_at_cancel
                .store(payload.processed_pairs, Ordering::Relaxed);
            self.total_at_cancel
                .store(payload.total_pairs, Ordering::Relaxed);
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}

fn absolute_lexical_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(format!("path escapes its root: {}", path.display()));
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn validate_isolation_root_shape(root: &Path) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("isolation_root must resolve to an absolute path".to_string());
    }
    let leaf = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !leaf.to_ascii_lowercase().starts_with(ISOLATION_PREFIX) {
        return Err(format!(
            "isolation_root leaf must start with '{ISOLATION_PREFIX}': {}",
            root.display()
        ));
    }
    if root.components().count() < 4 {
        return Err(format!(
            "isolation_root is too broad for recursive cleanup: {}",
            root.display()
        ));
    }
    Ok(())
}

fn cleanup_owned_isolation_root(root: &Path, expected_nonce: &str) -> Result<(), String> {
    validate_isolation_root_shape(root)?;
    if !root.exists() {
        return Ok(());
    }
    ensure_no_reparse_ancestors(root, "owned isolation cleanup root")?;
    ensure_no_reparse_tree(root, "owned isolation cleanup root")?;
    let marker_path = root.join(ISOLATION_MARKER);
    let raw = fs::read_to_string(&marker_path).map_err(|error| {
        format!(
            "refusing to clean unmarked isolation root {}: {error}",
            root.display()
        )
    })?;
    let marker: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid isolation ownership marker: {e}"))?;
    if marker.get("kind").and_then(Value::as_str)
        != Some("vectorgrid_cpu_reliability_soak_isolation")
        || marker.get("nonce").and_then(Value::as_str) != Some(expected_nonce)
    {
        return Err(format!(
            "refusing to clean isolation root with mismatched ownership marker: {}",
            root.display()
        ));
    }
    ensure_no_reparse_tree(root, "owned isolation cleanup root")?;
    fs::remove_dir_all(root)
        .map_err(|e| format!("failed to clean isolation root {}: {e}", root.display()))
}

fn validate_soak_app_data_root_shape(root: &Path) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("soak app data root must be absolute".to_string());
    }
    let leaf = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !leaf.contains("cpusoak") {
        return Err(format!(
            "soak app data root leaf does not contain the owned cpusoak identifier: {}",
            root.display()
        ));
    }
    if root.components().count() < 4 {
        return Err(format!(
            "soak app data root is too broad for recursive cleanup: {}",
            root.display()
        ));
    }
    Ok(())
}

fn cleanup_owned_app_data_root(root: &Path, expected_nonce: &str) -> Result<(), String> {
    validate_soak_app_data_root_shape(root)?;
    if !root.exists() {
        return Ok(());
    }
    ensure_no_reparse_ancestors(root, "owned app-data cleanup root")?;
    ensure_no_reparse_tree(root, "owned app-data cleanup root")?;
    let marker_path = root.join(APP_DATA_MARKER);
    let raw = fs::read_to_string(&marker_path).map_err(|error| {
        format!(
            "refusing to clean unmarked soak app data root {}: {error}",
            root.display()
        )
    })?;
    let marker: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid soak app data ownership marker: {e}"))?;
    if marker.get("kind").and_then(Value::as_str)
        != Some("vectorgrid_cpu_reliability_soak_app_data")
        || marker.get("nonce").and_then(Value::as_str) != Some(expected_nonce)
    {
        return Err(format!(
            "refusing to clean soak app data root with mismatched ownership marker: {}",
            root.display()
        ));
    }
    ensure_no_reparse_tree(root, "owned app-data cleanup root")?;
    fs::remove_dir_all(root)
        .map_err(|e| format!("failed to clean soak app data root {}: {e}", root.display()))
}

fn write_synthetic_ohlcv_csv(path: &Path, rows: usize) -> Result<(), String> {
    use std::io::{BufWriter, Write};

    atomic_output::write_with(path, |file| {
        let mut out = BufWriter::new(file);
        writeln!(out, "timestamp,open,high,low,close,volume").map_err(|e| e.to_string())?;
        let base_ts = 1_700_000_000_000i64;
        for i in 0..rows {
            let ts = base_ts + (i as i64) * 60_000;
            let trend = 100.0 + (i as f64) * 0.002;
            let wave = ((i as f64) * 0.031).sin() * 0.45;
            let close = trend + wave;
            let open = close - 0.04;
            let high = close + 0.15;
            let low = close - 0.18;
            let volume = 10_000.0 + (i % 113) as f64;
            writeln!(
                out,
                "{ts},{open:.6},{high:.6},{low:.6},{close:.6},{volume:.2}"
            )
            .map_err(|e| e.to_string())?;
        }
        out.flush().map_err(|e| e.to_string())
    })
}

fn build_isolated_app(app_data_dir: &Path) -> Result<tauri::App, String> {
    if !app_data_dir.is_absolute() {
        return Err("isolated Tauri app-data identifier must be absolute".to_string());
    }
    if COMPILED_TAURI_CONFIG.is_some() {
        return Err(
            "CPU reliability soak refuses a compile-time TAURI_CONFIG override".to_string(),
        );
    }
    if cfg!(feature = "custom-protocol") {
        return Err(
            "CPU reliability soak must be built without custom-protocol so the maintainer context remains headless"
                .to_string(),
        );
    }
    let unique = format!(
        "VectorGridCpuSoak{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut context = tauri::generate_context!();
    if context.config().build.dev_url.is_none() || context.config().build.frontend_dist.is_none() {
        return Err(
            "attested maintainer Tauri context requires checked-in devUrl and frontendDist settings"
                .to_string(),
        );
    }
    context.config_mut().product_name = Some(unique.clone());
    // Tauri's Windows path resolver does not honor LOCALAPPDATA overrides.
    // An absolute identifier intentionally replaces the platform base path
    // when Tauri joins it, matching the repository's packaged-QA profile.
    context.config_mut().identifier = app_data_dir.to_string_lossy().into_owned();
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .build(context)
        .map_err(|e| e.to_string())
}

fn optimization_request(data_id: &str) -> DoubleMaRequest {
    DoubleMaRequest {
        backend: Backend::CpuOnly,
        data_id: data_id.to_string(),
        fast_range: (5, 20, 1),
        slow_range: (24, 80, 4),
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
        top_k: Some(16),
        include_all: Some(false),
        heatmap_bins: Some(16),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,
    }
}

fn cancellation_optimization_request(data_id: &str) -> DoubleMaRequest {
    let mut request = optimization_request(data_id);
    // With MIN_RELIABILITY_ROWS and the pinned 64 MiB CPU tile budget, the
    // 1,026 slow-period candidates cannot fit in one tile. Deliberately
    // overlapping ranges keep the first valid tile small while still
    // guaranteeing a positive partial progress callback before cancellation.
    request.fast_range = (1_000, 1_524, 1);
    request.slow_range = (500, 1_525, 1);
    request
}

fn validation_candidates(
    top: &[(
        ta_app_core::DoubleMaParamsResolved,
        ta_strategies::double_ma::Metrics,
    )],
    max_candidates: usize,
) -> Vec<DoubleMaValidationParams> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (params, _) in top {
        let key = (
            params.fast_ma_type.clone(),
            params.fast_len,
            params.slow_ma_type.clone(),
            params.slow_len,
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(DoubleMaValidationParams {
            fast_ma_type: params.fast_ma_type.clone(),
            slow_ma_type: params.slow_ma_type.clone(),
            fast_len: params.fast_len,
            slow_len: params.slow_len,
            fast_ma_params: params.fast_ma_params.clone(),
            slow_ma_params: params.slow_ma_params.clone(),
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
        });
        if out.len() >= max_candidates {
            break;
        }
    }
    out
}

fn validation_request(
    data_id: &str,
    candidates: Vec<DoubleMaValidationParams>,
    optimization_trials: usize,
    csv_path: &Path,
    seed: u64,
) -> DoubleMaValidationRequest {
    DoubleMaValidationRequest {
        data_id: data_id.to_string(),
        backend: Backend::CpuOnly,
        ma_source: "close".to_string(),
        dynamic_strategy_ir: None,
        fast_ma_params: None,
        slow_ma_params: None,
        max_candidates: candidates.len(),
        candidates,
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        monte_carlo: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed,
        validation_engine: ValidationEngine::Vectorized,
        iterative: IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: Some(ValidationArtifactContext {
            strategy_kind: "DoubleMa".to_string(),
            csv_path: Some(csv_path.to_string_lossy().to_string()),
            timeframe: Some("1m".to_string()),
            label: Some("CPU reliability soak".to_string()),
            optimization_trials: Some(optimization_trials),
            search_distribution: None,
            data_realism: None,
        }),
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn collect_owned_storage_stats(root: &Path) -> Result<OwnedStorageStats, String> {
    if !root.exists() {
        return Ok(OwnedStorageStats::default());
    }
    let mut stats = OwnedStorageStats::default();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata_is_reparse(&metadata) {
            return Err(format!(
                "owned storage contains an unexpected symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            stats.directory_count = stats.directory_count.saturating_add(1);
            for entry in fs::read_dir(&path).map_err(|error| error.to_string())? {
                pending.push(entry.map_err(|error| error.to_string())?.path());
            }
            continue;
        }
        if !metadata.is_file() {
            return Err(format!(
                "owned storage contains an unsupported entry: {}",
                path.display()
            ));
        }
        stats.file_count = stats.file_count.saturating_add(1);
        stats.total_bytes = stats.total_bytes.saturating_add(metadata.len());
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if file_name.ends_with(".tmp")
            || file_name.ends_with(".bak")
            || file_name.starts_with("vectorgrid_run_")
            || file_name.starts_with(".vectorgrid_run_")
        {
            stats.residual_artifact_or_temp_files =
                stats.residual_artifact_or_temp_files.saturating_add(1);
        }
    }
    Ok(stats)
}

fn combined_owned_storage_stats(roots: &[&Path]) -> Result<OwnedStorageStats, String> {
    let mut combined = OwnedStorageStats::default();
    for root in roots {
        let stats = collect_owned_storage_stats(root)?;
        combined.file_count = combined.file_count.saturating_add(stats.file_count);
        combined.directory_count = combined
            .directory_count
            .saturating_add(stats.directory_count);
        combined.total_bytes = combined.total_bytes.saturating_add(stats.total_bytes);
        combined.residual_artifact_or_temp_files = combined
            .residual_artifact_or_temp_files
            .saturating_add(stats.residual_artifact_or_temp_files);
    }
    Ok(combined)
}

fn validate_source_manifest(contents: &Value) -> Result<(), String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(1)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_source_manifest")
    {
        return Err("source manifest identity is invalid".to_string());
    }
    if contents.get("aggregate_algorithm").and_then(Value::as_str)
        != Some("sha256(relative_path_tab_bytes_tab_sha256_lf)")
    {
        return Err("source manifest aggregate algorithm is invalid".to_string());
    }
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "source manifest aggregate_sha256",
    )?;
    let files = contents
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "source manifest files must be an array".to_string())?;
    if files.is_empty()
        || contents.get("file_count").and_then(Value::as_u64) != Some(files.len() as u64)
    {
        return Err("source manifest file count is invalid".to_string());
    }
    let mut prior_path: Option<&str> = None;
    let mut canonical = String::new();
    let mut saw_vendored_embed_resource_manifest = false;
    let mut saw_vendored_embed_resource_patch_notes = false;
    let mut saw_vendored_embed_resource_source = false;
    let mut saw_vendored_embed_resource_package = false;
    for (index, file) in files.iter().enumerate() {
        let path = file
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "source manifest file path is missing".to_string())?;
        let normalized = Path::new(path);
        if normalized.is_absolute()
            || normalized
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(format!("source manifest path must be relative: {path}"));
        }
        if prior_path.is_some_and(|prior| prior >= path) {
            return Err("source manifest file paths must be strictly sorted".to_string());
        }
        prior_path = Some(path);
        saw_vendored_embed_resource_manifest |=
            path == "vendor/embed-resource-3.0.6/.cargo_vcs_info.json";
        saw_vendored_embed_resource_package |= path == "vendor/embed-resource-3.0.6/Cargo.toml";
        saw_vendored_embed_resource_patch_notes |=
            path == "vendor/embed-resource-3.0.6/LOCAL_PATCHES.md";
        saw_vendored_embed_resource_source |=
            path == "vendor/embed-resource-3.0.6/src/windows_msvc.rs";
        let bytes = file
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("source manifest byte count is invalid: {path}"))?;
        let sha256 = validated_sha256(
            file.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("source manifest sha256 for {path}"),
        )?;
        if file.get("sha256").and_then(Value::as_str) != Some(sha256.as_str()) {
            return Err(format!(
                "source manifest SHA-256 must use lowercase hexadecimal: {path}"
            ));
        }
        if index > 0 {
            canonical.push('\n');
        }
        canonical.push_str(&format!("{path}\t{bytes}\t{sha256}"));
    }
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    if actual_aggregate != expected_aggregate {
        return Err(format!(
            "source manifest aggregate mismatch: expected {expected_aggregate}, actual {actual_aggregate}"
        ));
    }
    if !saw_vendored_embed_resource_manifest
        || !saw_vendored_embed_resource_package
        || !saw_vendored_embed_resource_patch_notes
        || !saw_vendored_embed_resource_source
    {
        return Err(
            "source manifest omits the vendored exact resource-compiler implementation".to_string(),
        );
    }
    Ok(())
}

fn path_starts_with(candidate: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        candidate
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with(
                &(root
                    .to_string_lossy()
                    .trim_end_matches(['\\', '/'])
                    .to_ascii_lowercase()
                    + "\\"),
            )
    }
    #[cfg(not(windows))]
    {
        candidate.starts_with(root)
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn windows_ordinal_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn validate_cargo_configuration_provenance(contents: &Value) -> Result<(), String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(1)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_cargo_configuration")
        || contents.get("aggregate_algorithm").and_then(Value::as_str)
            != Some("sha256(absolute_path_tab_exists_tab_bytes_tab_sha256_lf)")
    {
        return Err("Cargo configuration provenance identity is invalid".to_string());
    }
    let cargo_home = contents
        .get("cargo_home")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo configuration provenance omits cargo_home".to_string())?;
    let cargo_home = absolute_lexical_path(Path::new(cargo_home))?;
    ensure_no_reparse_ancestors(&cargo_home, "Cargo home")?;
    let repo_root = contents
        .get("repo_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo configuration provenance omits repo_root".to_string())?;
    let repo_root = absolute_lexical_path(Path::new(repo_root))?;
    ensure_no_reparse_ancestors(&repo_root, "Cargo configuration repository")?;
    let runtime_cargo_home = std::env::var_os("CARGO_HOME")
        .ok_or_else(|| "runtime CARGO_HOME is missing".to_string())?;
    if absolute_lexical_path(Path::new(&runtime_cargo_home))? != cargo_home {
        return Err("runtime CARGO_HOME differs from launcher provenance".to_string());
    }

    let candidates = contents
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo configuration candidates must be an array".to_string())?;
    if candidates.is_empty()
        || contents.get("candidate_count").and_then(Value::as_u64) != Some(candidates.len() as u64)
    {
        return Err("Cargo configuration candidate count is invalid".to_string());
    }
    let mut existing_count = 0u64;
    let mut prior_path: Option<&str> = None;
    let mut canonical = String::new();
    let mut cargo_home_candidate_count = 0usize;
    let mut observed_paths = Vec::with_capacity(candidates.len());
    let mut observed_existing_paths = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let path = candidate
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "Cargo configuration candidate path is missing".to_string())?;
        if prior_path.is_some_and(|prior| prior >= path) {
            return Err("Cargo configuration candidate paths must be strictly sorted".to_string());
        }
        prior_path = Some(path);
        let candidate_path = absolute_lexical_path(Path::new(path))?;
        observed_paths.push(candidate_path.clone());
        ensure_no_reparse_ancestors(&candidate_path, "Cargo configuration candidate")?;
        if candidate_path.parent() == Some(cargo_home.as_path()) {
            cargo_home_candidate_count = cargo_home_candidate_count.saturating_add(1);
        }
        let expected_exists = candidate
            .get("exists")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("Cargo configuration exists flag is missing: {path}"))?;
        let actual_metadata = match fs::symlink_metadata(&candidate_path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        if expected_exists != actual_metadata.is_some() {
            return Err(format!(
                "Cargo configuration existence changed after build: {path}"
            ));
        }

        let (bytes_text, hash_text) = if expected_exists {
            let metadata = actual_metadata
                .ok_or_else(|| format!("Cargo configuration disappeared: {path}"))?;
            if !metadata.is_file() || metadata_is_reparse(&metadata) {
                return Err(format!(
                    "Cargo configuration candidate must be an ordinary file: {path}"
                ));
            }
            let expected_bytes = candidate
                .get("bytes")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("Cargo configuration byte count is invalid: {path}"))?;
            if metadata.len() != expected_bytes {
                return Err(format!("Cargo configuration byte count changed: {path}"));
            }
            let expected_sha256 = validated_sha256(
                candidate
                    .get("sha256")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                &format!("Cargo configuration SHA-256 for {path}"),
            )?;
            if candidate.get("sha256").and_then(Value::as_str) != Some(expected_sha256.as_str()) {
                return Err(format!(
                    "Cargo configuration SHA-256 must be lowercase: {path}"
                ));
            }
            let actual_sha256 = sha256_file(&candidate_path)?;
            if actual_sha256 != expected_sha256 {
                return Err(format!("Cargo configuration SHA-256 changed: {path}"));
            }
            existing_count = existing_count.saturating_add(1);
            observed_existing_paths.push(candidate_path.clone());
            (expected_bytes.to_string(), expected_sha256)
        } else {
            if !candidate.get("bytes").is_none_or(Value::is_null)
                || !candidate.get("sha256").is_none_or(Value::is_null)
            {
                return Err(format!(
                    "absent Cargo configuration candidate carries file metadata: {path}"
                ));
            }
            ("-".to_string(), "-".to_string())
        };
        if index > 0 {
            canonical.push('\n');
        }
        canonical.push_str(&format!(
            "{path}\t{}\t{bytes_text}\t{hash_text}",
            if expected_exists { "1" } else { "0" }
        ));
    }
    if cargo_home_candidate_count != 2
        || contents.get("existing_file_count").and_then(Value::as_u64) != Some(existing_count)
    {
        return Err("Cargo configuration candidate coverage is incomplete".to_string());
    }
    let mut expected_paths = Vec::new();
    let mut cursor = Some(repo_root.as_path());
    while let Some(directory) = cursor {
        expected_paths.push(directory.join(".cargo").join("config.toml"));
        expected_paths.push(directory.join(".cargo").join("config"));
        cursor = directory.parent();
    }
    expected_paths.push(cargo_home.join("config.toml"));
    expected_paths.push(cargo_home.join("config"));
    expected_paths.sort();
    expected_paths.dedup();
    observed_paths.sort();
    if observed_paths != expected_paths {
        return Err("Cargo configuration search-chain coverage is incomplete".to_string());
    }
    let workspace_config_path = repo_root.join(".cargo").join("config.toml");
    let declared_workspace_config = contents
        .get("workspace_config_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo configuration workspace_config_path is missing".to_string())?;
    let declared_workspace_config = absolute_lexical_path(Path::new(declared_workspace_config))?;
    let declared_workspace_sha256 = validated_sha256(
        contents
            .get("workspace_config_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "Cargo workspace configuration SHA-256",
    )?;
    if declared_workspace_config != workspace_config_path
        || observed_existing_paths != [workspace_config_path.clone()]
        || contents
            .get("external_existing_file_count")
            .and_then(Value::as_u64)
            != Some(0)
        || contents
            .get("release_authority_eligible")
            .and_then(Value::as_bool)
            != Some(true)
        || sha256_file(&workspace_config_path)? != declared_workspace_sha256
    {
        return Err("Cargo configuration release-authority allowlist is inconsistent".to_string());
    }
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "Cargo configuration aggregate_sha256",
    )?;
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    if actual_aggregate != expected_aggregate {
        return Err(format!(
            "Cargo configuration aggregate mismatch: expected {expected_aggregate}, actual {actual_aggregate}"
        ));
    }
    Ok(())
}

fn validate_toolchain_input_manifest(
    contents: &Value,
    rustc_sysroot: &Path,
    target_triple: &str,
) -> Result<(), String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(1)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_toolchain_inputs")
        || contents.get("aggregate_algorithm").and_then(Value::as_str)
            != Some("sha256(relative_path_tab_bytes_tab_sha256_lf)")
    {
        return Err("toolchain input manifest identity is invalid".to_string());
    }
    let root = contents
        .get("root")
        .and_then(Value::as_str)
        .ok_or_else(|| "toolchain input manifest root is missing".to_string())?;
    if absolute_lexical_path(Path::new(root))? != rustc_sysroot {
        return Err("toolchain input manifest root differs from rustc sysroot".to_string());
    }
    let files = contents
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "toolchain input manifest files must be an array".to_string())?;
    if files.is_empty()
        || contents.get("file_count").and_then(Value::as_u64) != Some(files.len() as u64)
    {
        return Err("toolchain input manifest file count is invalid".to_string());
    }
    let target_lib_prefix = format!("lib/rustlib/{target_triple}/lib/");
    let mut saw_bin = false;
    let mut saw_target_lib = false;
    let mut prior_path: Option<&str> = None;
    let mut canonical = String::new();
    let mut total_bytes = 0u64;
    for (index, file) in files.iter().enumerate() {
        let relative = file
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "toolchain input path is missing".to_string())?;
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(format!("toolchain input path must be relative: {relative}"));
        }
        if prior_path.is_some_and(|prior| prior >= relative) {
            return Err("toolchain input paths must be strictly sorted".to_string());
        }
        prior_path = Some(relative);
        saw_bin |= relative.starts_with("bin/");
        saw_target_lib |= relative.starts_with(&target_lib_prefix);
        let absolute = rustc_sysroot.join(relative_path);
        ensure_no_reparse_ancestors(&absolute, "toolchain input")?;
        let metadata = fs::symlink_metadata(&absolute).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata_is_reparse(&metadata) {
            return Err(format!(
                "toolchain input must be an ordinary file: {}",
                absolute.display()
            ));
        }
        let expected_bytes = file
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("toolchain input byte count is invalid: {relative}"))?;
        if metadata.len() != expected_bytes {
            return Err(format!("toolchain input byte count changed: {relative}"));
        }
        total_bytes = total_bytes.saturating_add(expected_bytes);
        let expected_sha256 = validated_sha256(
            file.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("toolchain input SHA-256 for {relative}"),
        )?;
        if file.get("sha256").and_then(Value::as_str) != Some(expected_sha256.as_str())
            || sha256_file(&absolute)? != expected_sha256
        {
            return Err(format!("toolchain input SHA-256 changed: {relative}"));
        }
        if index > 0 {
            canonical.push('\n');
        }
        canonical.push_str(&format!("{relative}\t{expected_bytes}\t{expected_sha256}"));
    }
    if !saw_bin
        || !saw_target_lib
        || contents.get("total_bytes").and_then(Value::as_u64) != Some(total_bytes)
    {
        return Err("toolchain input manifest coverage is incomplete".to_string());
    }
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "toolchain input aggregate_sha256",
    )?;
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    if actual_aggregate != expected_aggregate {
        return Err(format!(
            "toolchain input aggregate mismatch: expected {expected_aggregate}, actual {actual_aggregate}"
        ));
    }
    Ok(())
}

fn existing_native_path_list_directories(
    value: &str,
    environment_name: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut directories = Vec::<(String, PathBuf)>::new();
    let mut seen = HashSet::<String>::new();
    for raw_segment in value.split(';') {
        let mut segment = raw_segment.trim();
        if segment.len() >= 2 && segment.starts_with('"') && segment.ends_with('"') {
            segment = segment[1..segment.len() - 1].trim();
        }
        if segment.is_empty() {
            continue;
        }
        if segment
            .bytes()
            .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
            || !Path::new(segment).is_absolute()
        {
            return Err(format!(
                "{environment_name} contains a non-canonical path-list segment: {segment}"
            ));
        }
        let path = absolute_lexical_path(Path::new(segment))?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        ensure_no_reparse_ancestors(&path, &format!("{environment_name} path-list directory"))?;
        if !metadata.is_dir() || metadata_is_reparse(&metadata) {
            return Err(format!(
                "{environment_name} path-list segment is not an ordinary directory: {}",
                path.display()
            ));
        }
        let key = path.to_string_lossy().to_ascii_lowercase();
        if seen.insert(key.clone()) {
            directories.push((key, path));
        }
    }
    directories.sort_by(|left, right| windows_ordinal_cmp(&left.0, &right.0));
    Ok(directories.into_iter().map(|(_, path)| path).collect())
}

fn validate_native_input_manifest(
    contents: &Value,
    visual_studio_installation: &Path,
    vc_tools_version: &str,
    windows_sdk_root: &Path,
    environment_hashes: &BTreeMap<String, String>,
) -> Result<String, String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(3)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_native_inputs")
        || contents.get("aggregate_algorithm").and_then(Value::as_str)
            != Some(
                "sha256(path_environment_tab_name_tab_value_sha256_then_root_tab_label_tab_absolute_path_then_file_tab_root_label_tab_relative_path_tab_bytes_tab_sha256_lf)",
            )
    {
        return Err("native input manifest identity is invalid".to_string());
    }
    let allowed_path_environment_names =
        ["EXTERNAL_INCLUDE", "IFCPATH", "INCLUDE", "LIB", "LIBPATH"];
    let required_path_environment_names = ["EXTERNAL_INCLUDE", "INCLUDE", "LIB", "LIBPATH"];
    let path_environment = contents
        .get("path_environment")
        .and_then(Value::as_array)
        .ok_or_else(|| "native input path environment must be an array".to_string())?;
    if !matches!(path_environment.len(), 4 | 5)
        || contents
            .get("path_environment_count")
            .and_then(Value::as_u64)
            != Some(path_environment.len() as u64)
    {
        return Err("native input path-environment coverage is incomplete".to_string());
    }
    let mut path_environment_values = BTreeMap::<String, String>::new();
    let mut canonical = String::new();
    let mut prior_path_environment_name: Option<&str> = None;
    for (index, row) in path_environment.iter().enumerate() {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "native input path-environment name is missing".to_string())?;
        if !allowed_path_environment_names.contains(&name)
            || prior_path_environment_name.is_some_and(|prior| prior >= name)
        {
            return Err("native input path-environment names are not canonical".to_string());
        }
        prior_path_environment_name = Some(name);
        let value = row
            .get("value")
            .and_then(Value::as_str)
            .filter(|value| {
                !value.trim().is_empty()
                    && !value
                        .bytes()
                        .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
            })
            .ok_or_else(|| format!("native input path-environment value is invalid: {name}"))?;
        let value_sha256 = validated_sha256(
            row.get("value_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("native input path-environment SHA-256 for {name}"),
        )?;
        let actual_sha256 = format!("{:x}", Sha256::digest(value.as_bytes()));
        if value_sha256 != actual_sha256
            || environment_hashes.get(name).map(String::as_str) != Some(value_sha256.as_str())
        {
            return Err(format!(
                "native input path environment does not match pinned environment: {name}"
            ));
        }
        if index > 0 {
            canonical.push('\n');
        }
        canonical.push_str(&format!("path_environment\t{name}\t{value_sha256}"));
        path_environment_values.insert(name.to_string(), value.to_string());
    }
    if required_path_environment_names
        .iter()
        .any(|name| !path_environment_values.contains_key(*name))
    {
        return Err("native input path-environment coverage is incomplete".to_string());
    }

    let roots = contents
        .get("roots")
        .and_then(Value::as_array)
        .ok_or_else(|| "native input roots must be an array".to_string())?;
    if roots.is_empty()
        || contents.get("root_count").and_then(Value::as_u64) != Some(roots.len() as u64)
    {
        return Err("native input root count is invalid".to_string());
    }
    let mut root_map = BTreeMap::<String, (PathBuf, String)>::new();
    let mut prior_label: Option<&str> = None;
    for root in roots {
        let label = root
            .get("label")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "native input root label is missing".to_string())?;
        if prior_label.is_some_and(|prior| prior >= label) {
            return Err("native input roots must be strictly sorted".to_string());
        }
        prior_label = Some(label);
        let path_text = root
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("native input root path is missing: {label}"))?;
        if [label, path_text].iter().any(|value| {
            value
                .bytes()
                .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
        }) {
            return Err("native input root provenance contains control characters".to_string());
        }
        if !Path::new(path_text).is_absolute() {
            return Err(format!(
                "native input root path is not fully qualified: {label}"
            ));
        }
        let path = absolute_lexical_path(Path::new(path_text))?;
        ensure_no_reparse_ancestors(&path, &format!("native input root {label}"))?;
        if !path.is_dir() {
            return Err(format!("native input root is not a directory: {label}"));
        }
        canonical.push('\n');
        canonical.push_str(&format!("root\t{label}\t{path_text}"));
        root_map.insert(label.to_string(), (path, path_text.to_string()));
    }

    let required_roots = [
        (
            "vc_auxiliary_build",
            visual_studio_installation
                .join("VC")
                .join("Auxiliary")
                .join("Build"),
        ),
        (
            "vc_tools",
            visual_studio_installation
                .join("VC")
                .join("Tools")
                .join("MSVC")
                .join(vc_tools_version),
        ),
        (
            "vs_common7_tools",
            visual_studio_installation.join("Common7").join("Tools"),
        ),
        ("windows_sdk_bin", windows_sdk_root.join("bin")),
        ("windows_sdk_include", windows_sdk_root.join("Include")),
        ("windows_sdk_lib", windows_sdk_root.join("Lib")),
    ];
    for (label, expected) in required_roots {
        let actual = root_map
            .get(label)
            .ok_or_else(|| format!("native input root coverage is missing: {label}"))?;
        if !paths_equal(&actual.0, &expected) {
            return Err(format!(
                "native input root does not match the selected VS/SDK: {label}"
            ));
        }
    }
    let common7_tools = &root_map
        .get("vs_common7_tools")
        .ok_or_else(|| "native input root coverage is missing: vs_common7_tools".to_string())?
        .0;
    let vs_dev_cmd = common7_tools.join("VsDevCmd.bat");
    let vs_dev_cmd_tree = common7_tools.join("vsdevcmd");
    ensure_no_reparse_ancestors(&vs_dev_cmd, "Visual Studio VsDevCmd.bat")?;
    ensure_no_reparse_ancestors(&vs_dev_cmd_tree, "Visual Studio vsdevcmd tree")?;
    let vs_dev_cmd_metadata =
        fs::symlink_metadata(&vs_dev_cmd).map_err(|error| error.to_string())?;
    let vs_dev_cmd_tree_metadata =
        fs::symlink_metadata(&vs_dev_cmd_tree).map_err(|error| error.to_string())?;
    if !vs_dev_cmd_metadata.is_file()
        || metadata_is_reparse(&vs_dev_cmd_metadata)
        || !vs_dev_cmd_tree_metadata.is_dir()
        || metadata_is_reparse(&vs_dev_cmd_tree_metadata)
    {
        return Err(
            "selected Visual Studio Common7 tools omit ordinary VsDevCmd.bat/vsdevcmd inputs"
                .to_string(),
        );
    }
    for (label, expected) in [
        (
            "windows_sdk_references",
            windows_sdk_root.join("References"),
        ),
        (
            "windows_sdk_union_metadata",
            windows_sdk_root.join("UnionMetadata"),
        ),
    ] {
        match (expected.is_dir(), root_map.get(label)) {
            (true, Some(actual)) if paths_equal(&actual.0, &expected) => {}
            (false, None) => {}
            _ => {
                return Err(format!(
                    "optional native input root does not match selected Windows SDK: {label}"
                ));
            }
        }
    }
    match (
        environment_hashes.get("NETFXSDKDIR"),
        root_map.get("netfx_sdk"),
    ) {
        (None, None) => {}
        (Some(expected_hash), Some((_, path_text)))
            if format!("{:x}", Sha256::digest(path_text.as_bytes())) == *expected_hash => {}
        _ => {
            return Err(
                "optional NETFX native input root does not match pinned NETFXSDKDir".to_string(),
            );
        }
    }
    let allowed_fixed_labels = [
        "netfx_sdk",
        "vc_auxiliary_build",
        "vc_tools",
        "vs_common7_tools",
        "windows_sdk_bin",
        "windows_sdk_include",
        "windows_sdk_lib",
        "windows_sdk_references",
        "windows_sdk_union_metadata",
    ];
    let fixed_roots = root_map
        .iter()
        .filter(|(label, _)| allowed_fixed_labels.contains(&label.as_str()))
        .map(|(_, (path, _))| path)
        .collect::<Vec<_>>();
    let mut existing_path_directories = Vec::<(String, PathBuf)>::new();
    let mut seen_path_directories = HashSet::<String>::new();
    for (name, value) in &path_environment_values {
        for directory in existing_native_path_list_directories(value, name)? {
            let key = directory.to_string_lossy().to_ascii_lowercase();
            if seen_path_directories.insert(key.clone()) {
                existing_path_directories.push((key, directory));
            }
        }
    }
    existing_path_directories.sort_by(|left, right| windows_ordinal_cmp(&left.0, &right.0));
    let uncovered_directories = existing_path_directories
        .into_iter()
        .map(|(_, directory)| directory)
        .filter(|directory| {
            !fixed_roots
                .iter()
                .any(|root| paths_equal(directory, root) || path_starts_with(directory, root))
        })
        .collect::<Vec<_>>();
    for (index, expected_path) in uncovered_directories.iter().enumerate() {
        let label = format!("environment_path_{index:04}");
        let actual = root_map
            .get(&label)
            .ok_or_else(|| format!("native input root coverage is missing: {label}"))?;
        if !paths_equal(&actual.0, expected_path) {
            return Err(format!(
                "native input environment path root does not match pinned path lists: {label}"
            ));
        }
    }
    let expected_dynamic_count = uncovered_directories.len();
    if root_map.keys().any(|label| {
        if allowed_fixed_labels.contains(&label.as_str()) {
            return false;
        }
        label
            .strip_prefix("environment_path_")
            .and_then(|suffix| suffix.parse::<usize>().ok())
            .is_none_or(|index| {
                label.len() != "environment_path_0000".len() || index >= expected_dynamic_count
            })
    }) {
        return Err("native input manifest contains an unexpected root label".to_string());
    }

    let files = contents
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "native input files must be an array".to_string())?;
    let mut live_files = Vec::<(String, String, PathBuf)>::new();
    for (root_label, (root_path, _)) in &root_map {
        for (relative, path) in collect_ordinary_tree_files(root_path)? {
            live_files.push((root_label.clone(), relative, path));
        }
    }
    live_files.sort_by(|left, right| {
        windows_ordinal_cmp(&left.0, &right.0).then_with(|| windows_ordinal_cmp(&left.1, &right.1))
    });
    if files.is_empty()
        || files.len() != live_files.len()
        || contents.get("file_count").and_then(Value::as_u64) != Some(files.len() as u64)
    {
        return Err("native input file count is invalid".to_string());
    }
    let mut prior_key: Option<String> = None;
    let mut total_bytes = 0u64;
    for (file, (live_root_label, live_relative, live_path)) in files.iter().zip(live_files.iter()) {
        let root_label = file
            .get("root")
            .and_then(Value::as_str)
            .ok_or_else(|| "native input file root label is missing".to_string())?;
        let root = root_map
            .get(root_label)
            .ok_or_else(|| format!("native input file refers to an unknown root: {root_label}"))?;
        let relative = file
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "native input relative path is missing".to_string())?;
        if root_label != live_root_label || relative != live_relative {
            return Err("native input file rows omit or reorder live files".to_string());
        }
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
            || relative
                .bytes()
                .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
        {
            return Err(format!("native input path must be relative: {relative}"));
        }
        let key = format!("{root_label}\t{relative}");
        if prior_key
            .as_ref()
            .is_some_and(|prior| windows_ordinal_cmp(prior, &key).is_ge())
        {
            return Err("native input file rows must be strictly sorted".to_string());
        }
        prior_key = Some(key);
        let absolute = native_input_absolute_path(&root.0, relative_path)?;
        if !paths_equal(&absolute, live_path) {
            return Err(format!(
                "native input live file path differs from its attested root: root={}, relative={}, reconstructed={}, live={}",
                root.0.display(),
                relative,
                absolute.display(),
                live_path.display()
            ));
        }
        ensure_no_reparse_ancestors(&absolute, "native input file")?;
        let metadata = fs::symlink_metadata(&absolute).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata_is_reparse(&metadata) {
            return Err(format!(
                "native input must be an ordinary file: {}",
                absolute.display()
            ));
        }
        let expected_bytes = file.get("bytes").and_then(Value::as_u64).ok_or_else(|| {
            format!("native input byte count is invalid: {root_label}/{relative}")
        })?;
        if metadata.len() != expected_bytes {
            return Err(format!(
                "native input byte count changed: {root_label}/{relative}"
            ));
        }
        total_bytes = total_bytes.saturating_add(expected_bytes);
        let expected_sha256 = validated_sha256(
            file.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("native input SHA-256 for {root_label}/{relative}"),
        )?;
        if sha256_file(&absolute)? != expected_sha256 {
            return Err(format!(
                "native input SHA-256 changed: {root_label}/{relative}"
            ));
        }
        canonical.push('\n');
        canonical.push_str(&format!(
            "file\t{root_label}\t{relative}\t{expected_bytes}\t{expected_sha256}"
        ));
    }
    if contents.get("total_bytes").and_then(Value::as_u64) != Some(total_bytes) {
        return Err("native input total byte count is invalid".to_string());
    }
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "native input aggregate_sha256",
    )?;
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    if actual_aggregate != expected_aggregate {
        return Err(format!(
            "native input aggregate mismatch: expected {expected_aggregate}, actual {actual_aggregate}"
        ));
    }
    Ok(actual_aggregate)
}

fn collect_ordinary_tree_files(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if metadata_is_reparse(&metadata) {
                return Err(format!(
                    "attested package tree contains a reparse point: {}",
                    path.display()
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative.is_empty()
                    || relative
                        .bytes()
                        .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
                {
                    return Err("attested package tree path is not canonical".to_string());
                }
                files.push((relative, path));
            } else {
                return Err(format!(
                    "attested package tree contains a non-file entry: {}",
                    path.display()
                ));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn native_input_absolute_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    absolute_lexical_path(&root.join(relative))
}

fn validate_registry_package_tree(
    contents: &Value,
    cargo_home: &Path,
    expected_label: &str,
    expected_directory_name: &str,
    expected_manifest_path: &Path,
) -> Result<String, String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(1)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_registry_package_tree")
        || contents.get("aggregate_algorithm").and_then(Value::as_str)
            != Some(
                "sha256(root_tab_label_tab_absolute_path_then_file_tab_relative_path_tab_bytes_tab_sha256_lf)",
            )
        || contents.get("label").and_then(Value::as_str) != Some(expected_label)
    {
        return Err(format!(
            "Cargo registry package-tree identity is invalid: {expected_label}"
        ));
    }
    let root_text = contents
        .get("root_path")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Cargo registry package root is missing: {expected_label}"))?;
    if [root_text, expected_label].iter().any(|value| {
        value
            .bytes()
            .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
    }) {
        return Err(
            "Cargo registry package-tree provenance contains control characters".to_string(),
        );
    }
    let root = absolute_lexical_path(Path::new(root_text))?;
    ensure_no_reparse_ancestors(&root, "Cargo registry package root")?;
    let registry_src = cargo_home.join("registry").join("src");
    if root
        .file_name()
        .and_then(|value| value.to_str())
        .is_none_or(|value| value != expected_directory_name)
        || root
            .parent()
            .and_then(Path::parent)
            .is_none_or(|value| !paths_equal(value, &registry_src))
        || !root.is_dir()
        || !paths_equal(&root.join("Cargo.toml"), expected_manifest_path)
    {
        return Err(format!(
            "Cargo registry package root escaped pinned CARGO_HOME: {expected_label}"
        ));
    }
    let live_files = collect_ordinary_tree_files(&root)?;
    let evidence_files = contents
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Cargo registry package files are missing: {expected_label}"))?;
    if live_files.is_empty()
        || evidence_files.len() != live_files.len()
        || contents.get("file_count").and_then(Value::as_u64) != Some(live_files.len() as u64)
    {
        return Err(format!(
            "Cargo registry package file coverage is incomplete: {expected_label}"
        ));
    }
    let mut total_bytes = 0u64;
    let mut canonical = format!("root\t{expected_label}\t{root_text}");
    for (row, (live_relative, live_path)) in evidence_files.iter().zip(live_files.iter()) {
        let relative = row
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "Cargo registry package relative path is missing".to_string())?;
        if relative != live_relative {
            return Err(format!(
                "Cargo registry package rows are incomplete or unsorted: {expected_label}"
            ));
        }
        let metadata = fs::symlink_metadata(live_path).map_err(|error| error.to_string())?;
        let bytes = row
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| "Cargo registry package byte count is missing".to_string())?;
        let sha256 = validated_sha256(
            row.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "Cargo registry package file SHA-256",
        )?;
        if !metadata.is_file()
            || metadata_is_reparse(&metadata)
            || metadata.len() != bytes
            || sha256_file(live_path)? != sha256
        {
            return Err(format!(
                "Cargo registry package file changed: {expected_label}/{relative}"
            ));
        }
        total_bytes = total_bytes.saturating_add(bytes);
        canonical.push_str(&format!("\nfile\t{relative}\t{bytes}\t{sha256}"));
    }
    if contents.get("total_bytes").and_then(Value::as_u64) != Some(total_bytes) {
        return Err(format!(
            "Cargo registry package total bytes changed: {expected_label}"
        ));
    }
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "Cargo registry package-tree aggregate SHA-256",
    )?;
    if actual_aggregate != expected_aggregate {
        return Err(format!(
            "Cargo registry package-tree aggregate mismatch: {expected_label}"
        ));
    }
    Ok(actual_aggregate)
}

fn validate_manifest_file(
    package: &Value,
    expected_path: &Path,
    label: &str,
) -> Result<(String, u64, String), String> {
    let path_text = package
        .get("manifest_path")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label} manifest path is missing"))?;
    let path = absolute_lexical_path(Path::new(path_text))?;
    ensure_no_reparse_ancestors(&path, &format!("{label} manifest"))?;
    let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
    let bytes = package
        .get("manifest_bytes")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label} manifest byte count is missing"))?;
    let sha256 = validated_sha256(
        package
            .get("manifest_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        &format!("{label} manifest SHA-256"),
    )?;
    if !paths_equal(&path, expected_path)
        || !metadata.is_file()
        || metadata_is_reparse(&metadata)
        || metadata.len() != bytes
        || sha256_file(&path)? != sha256
    {
        return Err(format!(
            "Cargo did not resolve the canonical {label} manifest"
        ));
    }
    Ok((path_text.to_string(), bytes, sha256))
}

fn validate_build_chain_edge(
    edge: &Value,
    from: &str,
    declared_name: &str,
    resolved_name: &str,
    to: &str,
    kind: &str,
    requirement: &str,
    label: &str,
) -> Result<(), String> {
    if edge.get("from_package_id").and_then(Value::as_str) != Some(from)
        || edge.get("declared_dependency_name").and_then(Value::as_str) != Some(declared_name)
        || edge.get("resolved_dependency_name").and_then(Value::as_str) != Some(resolved_name)
        || edge.get("to_package_id").and_then(Value::as_str) != Some(to)
        || edge.get("kind").and_then(Value::as_str) != Some(kind)
        || !edge.get("target").is_some_and(Value::is_null)
        || edge.get("declared_requirement").and_then(Value::as_str) != Some(requirement)
    {
        return Err(format!("Cargo build-chain edge is invalid: {label}"));
    }
    Ok(())
}

fn expected_path_package_id(
    package_root: &Path,
    package_name: &str,
    package_version: &str,
) -> Result<String, String> {
    let package_root = absolute_lexical_path(package_root)?;
    let file_url = tauri::Url::from_directory_path(&package_root).map_err(|()| {
        format!(
            "could not encode Cargo path package root as a file URL: {}",
            package_root.display()
        )
    })?;
    Ok(format!(
        "path+{}#{package_name}@{package_version}",
        file_url.as_str().trim_end_matches('/')
    ))
}

fn validate_embed_resource_cargo_resolution(
    contents: &Value,
    repo_root: &Path,
    cargo_home: &Path,
    host_target_triple: &str,
    toolchain: &Value,
) -> Result<String, String> {
    let count_names = [
        "root_package_name_match_count",
        "tauri_build_package_name_match_count",
        "tauri_winres_package_name_match_count",
        "embed_resource_package_name_match_count",
        "root_to_tauri_build_declared_edge_match_count",
        "root_to_tauri_build_resolved_edge_match_count",
        "tauri_build_to_winres_declared_edge_match_count",
        "tauri_build_to_winres_resolved_edge_match_count",
        "winres_to_embed_declared_edge_match_count",
        "winres_to_embed_resolved_edge_match_count",
    ];
    if contents.get("schema_version").and_then(Value::as_u64) != Some(2)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_windows_build_dependency_chain")
        || contents.get("aggregate_algorithm").and_then(Value::as_str)
            != Some("sha256(canonical_tab_rows_lf)")
        || contents
            .get("metadata_format_version")
            .and_then(Value::as_u64)
            != Some(1)
        || count_names
            .iter()
            .any(|name| contents.get(name).and_then(Value::as_u64) != Some(1))
    {
        return Err("Cargo build-dependency-chain identity is invalid".to_string());
    }
    let metadata_output_sha256 = validated_sha256(
        contents
            .get("metadata_output_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "Cargo metadata output SHA-256",
    )?;
    let metadata_arguments = contents
        .get("metadata_arguments")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo metadata arguments must be an array".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "Cargo metadata arguments must be strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_metadata_arguments = [
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
        "--filter-platform",
        host_target_triple,
        "--no-default-features",
        "--features",
        "ta_desktop_demo_app/maintainer-tools",
    ];
    if metadata_arguments != expected_metadata_arguments {
        return Err(
            "Cargo metadata was not invoked for the exact locked/offline soak build selection"
                .to_string(),
        );
    }
    let root_features = contents
        .get("root_resolved_features")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo root resolved features are missing".to_string())?;
    if root_features.len() != 1 || root_features[0].as_str() != Some("maintainer-tools") {
        return Err("Cargo root features do not match the no-default maintainer build".to_string());
    }

    let workspace_root_text = contents
        .get("workspace_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo metadata workspace root is missing".to_string())?;
    let workspace_root = absolute_lexical_path(Path::new(workspace_root_text))?;
    let cargo_home_text = contents
        .get("cargo_home")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo metadata CARGO_HOME is missing".to_string())?;
    let declared_cargo_home = absolute_lexical_path(Path::new(cargo_home_text))?;
    ensure_no_reparse_ancestors(&workspace_root, "Cargo metadata workspace root")?;
    ensure_no_reparse_ancestors(&declared_cargo_home, "Cargo metadata CARGO_HOME")?;
    if !paths_equal(&workspace_root, repo_root)
        || !workspace_root.is_dir()
        || !paths_equal(&declared_cargo_home, cargo_home)
        || !declared_cargo_home.is_dir()
    {
        return Err("Cargo metadata roots differ from launcher provenance".to_string());
    }
    let workspace_member_count = contents
        .get("workspace_member_count")
        .and_then(Value::as_u64)
        .filter(|count| *count >= 5)
        .ok_or_else(|| "Cargo metadata workspace-member coverage is incomplete".to_string())?;

    let cargo = contents
        .get("cargo")
        .ok_or_else(|| "Cargo metadata executable attestation is missing".to_string())?;
    let cargo_path_text = cargo
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo metadata executable path is missing".to_string())?;
    let cargo_path = absolute_lexical_path(Path::new(cargo_path_text))?;
    ensure_no_reparse_ancestors(&cargo_path, "Cargo metadata executable")?;
    let cargo_metadata = fs::symlink_metadata(&cargo_path).map_err(|error| error.to_string())?;
    let cargo_bytes = cargo
        .get("bytes")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Cargo metadata executable byte count is missing".to_string())?;
    let cargo_sha256 = validated_sha256(
        cargo
            .get("sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "Cargo metadata executable SHA-256",
    )?;
    let toolchain_cargo_path = toolchain
        .get("cargo_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "toolchain cargo_path is missing".to_string())?;
    let toolchain_cargo_sha256 = validated_sha256(
        toolchain
            .get("cargo_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "toolchain Cargo SHA-256",
    )?;
    if !cargo_metadata.is_file()
        || metadata_is_reparse(&cargo_metadata)
        || cargo_metadata.len() != cargo_bytes
        || sha256_file(&cargo_path)? != cargo_sha256
        || !paths_equal(
            &cargo_path,
            &absolute_lexical_path(Path::new(toolchain_cargo_path))?,
        )
        || cargo_sha256 != toolchain_cargo_sha256
    {
        return Err("Cargo metadata did not use the pinned toolchain executable".to_string());
    }

    let root = contents
        .get("root_package")
        .ok_or_else(|| "root Cargo package attestation is missing".to_string())?;
    let root_package_id = root
        .get("package_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "root Cargo package ID is missing".to_string())?;
    let expected_root_package_id =
        expected_path_package_id(&repo_root.join("src-tauri"), "ta_desktop_demo_app", "0.1.0")?;
    if root.get("name").and_then(Value::as_str) != Some("ta_desktop_demo_app")
        || root.get("version").and_then(Value::as_str) != Some("0.1.0")
        || !root.get("source").is_some_and(Value::is_null)
        || root.get("workspace_member").and_then(Value::as_bool) != Some(true)
        || root_package_id != expected_root_package_id
        || root
            .get("manifest_repo_relative_path")
            .and_then(Value::as_str)
            != Some("src-tauri/Cargo.toml")
    {
        return Err("Cargo root package identity is invalid".to_string());
    }
    let (root_manifest_path, root_manifest_bytes, root_manifest_sha256) = validate_manifest_file(
        root,
        &repo_root.join("src-tauri").join("Cargo.toml"),
        "root package",
    )?;

    let registry_source = "registry+https://github.com/rust-lang/crates.io-index";
    let tauri_build = contents
        .get("tauri_build")
        .ok_or_else(|| "tauri-build package attestation is missing".to_string())?;
    let tauri_build_id = tauri_build
        .get("package_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "tauri-build package ID is missing".to_string())?;
    let tauri_build_manifest = tauri_build
        .get("manifest_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "tauri-build manifest path is missing".to_string())?;
    if tauri_build.get("name").and_then(Value::as_str) != Some("tauri-build")
        || tauri_build.get("version").and_then(Value::as_str) != Some("2.5.6")
        || tauri_build.get("source").and_then(Value::as_str) != Some(registry_source)
        || tauri_build_id
            != "registry+https://github.com/rust-lang/crates.io-index#tauri-build@2.5.6"
    {
        return Err("Cargo resolved an unexpected tauri-build package".to_string());
    }
    let tauri_build_manifest_path = absolute_lexical_path(Path::new(tauri_build_manifest))?;
    let tauri_build_tree_aggregate = validate_registry_package_tree(
        tauri_build
            .get("source_tree")
            .ok_or_else(|| "tauri-build source-tree attestation is missing".to_string())?,
        cargo_home,
        "tauri-build@2.5.6",
        "tauri-build-2.5.6",
        &tauri_build_manifest_path,
    )?;

    let tauri_winres = contents
        .get("tauri_winres")
        .ok_or_else(|| "tauri-winres package attestation is missing".to_string())?;
    let tauri_winres_id = tauri_winres
        .get("package_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "tauri-winres package ID is missing".to_string())?;
    let tauri_winres_manifest = tauri_winres
        .get("manifest_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "tauri-winres manifest path is missing".to_string())?;
    if tauri_winres.get("name").and_then(Value::as_str) != Some("tauri-winres")
        || tauri_winres.get("version").and_then(Value::as_str) != Some("0.3.5")
        || tauri_winres.get("source").and_then(Value::as_str) != Some(registry_source)
        || tauri_winres_id
            != "registry+https://github.com/rust-lang/crates.io-index#tauri-winres@0.3.5"
    {
        return Err("Cargo resolved an unexpected tauri-winres package".to_string());
    }
    let tauri_winres_manifest_path = absolute_lexical_path(Path::new(tauri_winres_manifest))?;
    let tauri_winres_tree_aggregate = validate_registry_package_tree(
        tauri_winres
            .get("source_tree")
            .ok_or_else(|| "tauri-winres source-tree attestation is missing".to_string())?,
        cargo_home,
        "tauri-winres@0.3.5",
        "tauri-winres-0.3.5",
        &tauri_winres_manifest_path,
    )?;

    let embed = contents
        .get("embed_resource")
        .ok_or_else(|| "embed-resource package attestation is missing".to_string())?;
    let embed_id = embed
        .get("package_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "embed-resource package ID is missing".to_string())?;
    let expected_embed_id = expected_path_package_id(
        &repo_root.join("vendor").join("embed-resource-3.0.6"),
        "embed-resource",
        "3.0.6",
    )?;
    if embed.get("name").and_then(Value::as_str) != Some("embed-resource")
        || embed.get("version").and_then(Value::as_str) != Some("3.0.6")
        || !embed.get("source").is_some_and(Value::is_null)
        || embed.get("workspace_member").and_then(Value::as_bool) != Some(true)
        || embed_id != expected_embed_id
        || embed
            .get("manifest_repo_relative_path")
            .and_then(Value::as_str)
            != Some("vendor/embed-resource-3.0.6/Cargo.toml")
    {
        return Err("Cargo vendored embed-resource package identity is invalid".to_string());
    }
    let (embed_manifest_path, embed_manifest_bytes, embed_manifest_sha256) =
        validate_manifest_file(
            embed,
            &repo_root
                .join("vendor")
                .join("embed-resource-3.0.6")
                .join("Cargo.toml"),
            "vendored embed-resource",
        )?;

    let root_to_build = contents
        .get("root_to_tauri_build_edge")
        .ok_or_else(|| "root-to-tauri-build edge is missing".to_string())?;
    validate_build_chain_edge(
        root_to_build,
        root_package_id,
        "tauri-build",
        "tauri_build",
        tauri_build_id,
        "build",
        "^2",
        "root -> tauri-build",
    )?;
    let build_to_winres = contents
        .get("tauri_build_to_winres_edge")
        .ok_or_else(|| "tauri-build-to-winres edge is missing".to_string())?;
    validate_build_chain_edge(
        build_to_winres,
        tauri_build_id,
        "tauri-winres",
        "tauri_winres",
        tauri_winres_id,
        "normal",
        "^0.3",
        "tauri-build -> tauri-winres",
    )?;
    let winres_to_embed = contents
        .get("winres_to_embed_edge")
        .ok_or_else(|| "winres-to-embed edge is missing".to_string())?;
    validate_build_chain_edge(
        winres_to_embed,
        tauri_winres_id,
        "embed-resource",
        "embed_resource",
        embed_id,
        "normal",
        "^3",
        "tauri-winres -> embed-resource",
    )?;

    for value in [
        workspace_root_text,
        cargo_home_text,
        cargo_path_text,
        root_package_id,
        root_manifest_path.as_str(),
        tauri_build_id,
        tauri_build_manifest,
        tauri_winres_id,
        tauri_winres_manifest,
        embed_id,
        embed_manifest_path.as_str(),
    ] {
        if value
            .bytes()
            .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
        {
            return Err("Cargo build-chain provenance contains control characters".to_string());
        }
    }

    let canonical = format!(
        "metadata_format_version\t1\nmetadata_output_sha256\t{metadata_output_sha256}\nmetadata_arguments\t{}\nworkspace_root\t{workspace_root_text}\ncargo_home\t{cargo_home_text}\nworkspace_member_count\t{workspace_member_count}\nroot_resolved_features\tmaintainer-tools\ncargo\t{cargo_path_text}\t{cargo_bytes}\t{cargo_sha256}\nroot_package_match_count\t1\nroot_package\t{root_package_id}\t0.1.0\tnull\t{root_manifest_path}\tsrc-tauri/Cargo.toml\t{root_manifest_bytes}\t{root_manifest_sha256}\ntauri_build_match_count\t1\ntauri_build\t{tauri_build_id}\t2.5.6\t{registry_source}\t{tauri_build_manifest}\t{tauri_build_tree_aggregate}\ntauri_winres_match_count\t1\ntauri_winres\t{tauri_winres_id}\t0.3.5\t{registry_source}\t{tauri_winres_manifest}\t{tauri_winres_tree_aggregate}\nembed_resource_match_count\t1\nembed_resource\t{embed_id}\t3.0.6\tnull\t{embed_manifest_path}\tvendor/embed-resource-3.0.6/Cargo.toml\t{embed_manifest_bytes}\t{embed_manifest_sha256}\nroot_to_tauri_build_declared_match_count\t1\nroot_to_tauri_build_resolved_match_count\t1\nroot_to_tauri_build_edge\t{root_package_id}\ttauri-build\ttauri_build\t{tauri_build_id}\tbuild\tnull\t^2\ntauri_build_to_winres_declared_match_count\t1\ntauri_build_to_winres_resolved_match_count\t1\ntauri_build_to_winres_edge\t{tauri_build_id}\ttauri-winres\ttauri_winres\t{tauri_winres_id}\tnormal\tnull\t^0.3\nwinres_to_embed_declared_match_count\t1\nwinres_to_embed_resolved_match_count\t1\nwinres_to_embed_edge\t{tauri_winres_id}\tembed-resource\tembed_resource\t{embed_id}\tnormal\tnull\t^3",
        metadata_arguments.join("\t")
    );
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "Cargo build-chain aggregate SHA-256",
    )?;
    if actual_aggregate != expected_aggregate {
        return Err(format!(
            "Cargo build-chain aggregate mismatch: expected {expected_aggregate}, actual {actual_aggregate}"
        ));
    }
    Ok(actual_aggregate)
}

fn parse_windows_sdk_version(value: &str) -> Result<[i32; 4], String> {
    let mut parts = value.split('.');
    let mut parsed = [0i32; 4];
    for component in &mut parsed {
        let part = parts
            .next()
            .filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| format!("Windows SDK version is malformed: {value}"))?;
        *component = part
            .parse::<i32>()
            .map_err(|_| format!("Windows SDK version component is out of range: {value}"))?;
    }
    if parts.next().is_some() {
        return Err(format!("Windows SDK version is malformed: {value}"));
    }
    Ok(parsed)
}

fn enumerate_resource_compiler_candidates(
    windows_sdk_root: &Path,
) -> Result<Vec<(String, PathBuf)>, String> {
    let bin_root = windows_sdk_root.join("bin");
    ensure_no_reparse_ancestors(&bin_root, "Windows SDK bin root")?;
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&bin_root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
        if metadata_is_reparse(&metadata) {
            return Err(format!(
                "Windows SDK candidate directory is a reparse point: {}",
                entry.path().display()
            ));
        }
        if !metadata.is_dir() {
            continue;
        }
        let version = entry.file_name().to_string_lossy().to_string();
        if parse_windows_sdk_version(&version).is_err() {
            continue;
        }
        let rc = entry.path().join("x64").join("rc.exe");
        let rc_metadata = match fs::symlink_metadata(&rc) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        ensure_no_reparse_ancestors(&rc, "Windows SDK resource compiler candidate")?;
        if !rc_metadata.is_file() || metadata_is_reparse(&rc_metadata) {
            return Err(format!(
                "Windows SDK resource compiler candidate is not ordinary: {}",
                rc.display()
            ));
        }
        candidates.push((version, absolute_lexical_path(&rc)?));
    }
    candidates.sort_by_key(|(version, _)| {
        parse_windows_sdk_version(version).expect("filtered numeric Windows SDK version")
    });
    Ok(candidates)
}

fn resolve_recorded_path_executable(
    path_value: &str,
    executable_name: &str,
) -> Result<PathBuf, String> {
    for raw_segment in path_value.split(';') {
        let mut segment = raw_segment.trim();
        if segment.len() >= 2 && segment.starts_with('"') && segment.ends_with('"') {
            segment = segment[1..segment.len() - 1].trim();
        }
        if segment.is_empty() {
            continue;
        }
        if segment
            .bytes()
            .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
            || !Path::new(segment).is_absolute()
        {
            return Err(format!(
                "recorded PATH contains a non-canonical segment: {segment}"
            ));
        }
        let directory = absolute_lexical_path(Path::new(segment))?;
        if !directory.is_dir() {
            continue;
        }
        let candidate = directory.join(executable_name);
        let metadata = match fs::symlink_metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        ensure_no_reparse_ancestors(&candidate, "PATH-resolved native tool")?;
        if !metadata.is_file() || metadata_is_reparse(&metadata) {
            return Err(format!(
                "PATH-resolved native tool is not an ordinary file: {}",
                candidate.display()
            ));
        }
        return absolute_lexical_path(&candidate);
    }
    Err(format!(
        "recorded PATH does not resolve required native tool: {executable_name}"
    ))
}

fn validate_path_tool_resolution(
    contents: &Value,
    environment_hashes: &BTreeMap<String, String>,
    attested_tools: &BTreeMap<String, (String, PathBuf, u64, String)>,
) -> Result<String, String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(1)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_path_tool_resolution")
        || contents.get("environment_name").and_then(Value::as_str) != Some("PATH")
        || contents.get("resolution_policy").and_then(Value::as_str)
            != Some("first_existing_ordinary_executable_in_recorded_path_order")
        || contents.get("aggregate_algorithm").and_then(Value::as_str)
            != Some(
                "sha256(path_value_sha256_then_name_tab_executable_tab_path_tab_bytes_tab_sha256_lf)",
            )
    {
        return Err("native PATH tool-resolution identity is invalid".to_string());
    }
    let path_value = contents
        .get("value")
        .and_then(Value::as_str)
        .filter(|value| {
            !value.trim().is_empty()
                && !value
                    .bytes()
                    .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
        })
        .ok_or_else(|| "recorded native PATH is invalid".to_string())?;
    let value_sha256 = validated_sha256(
        contents
            .get("value_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "recorded native PATH SHA-256",
    )?;
    if format!("{:x}", Sha256::digest(path_value.as_bytes())) != value_sha256
        || environment_hashes.get("PATH").map(String::as_str) != Some(value_sha256.as_str())
    {
        return Err("recorded native PATH differs from pinned environment".to_string());
    }
    let expected_names = ["cl", "lib", "link", "ml64", "mt", "rc"];
    let tools = contents
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "native PATH tool-resolution rows are missing".to_string())?;
    if tools.len() != expected_names.len()
        || contents.get("tool_count").and_then(Value::as_u64) != Some(tools.len() as u64)
    {
        return Err("native PATH tool-resolution coverage is incomplete".to_string());
    }
    let mut canonical = format!("path_value_sha256\t{value_sha256}");
    for (row, expected_name) in tools.iter().zip(expected_names.iter()) {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "native PATH tool name is missing".to_string())?;
        let executable_name = row
            .get("executable_name")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("native PATH executable name is missing: {name}"))?;
        let expected_executable_name = format!("{expected_name}.exe");
        let path_text = row
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("native PATH tool path is missing: {name}"))?;
        let path = absolute_lexical_path(Path::new(path_text))?;
        let resolved = resolve_recorded_path_executable(path_value, &expected_executable_name)?;
        let attested = attested_tools
            .get(*expected_name)
            .ok_or_else(|| format!("attested native tool is missing: {expected_name}"))?;
        let bytes = row
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("native PATH tool byte count is missing: {name}"))?;
        let sha256 = validated_sha256(
            row.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("native PATH tool SHA-256 for {name}"),
        )?;
        if name != *expected_name
            || executable_name != expected_executable_name
            || !paths_equal(&path, &resolved)
            || !paths_equal(&path, &attested.1)
            || path_text != attested.0
            || bytes != attested.2
            || sha256 != attested.3
        {
            return Err(format!(
                "recorded PATH does not first-resolve the exact attested native tool: {name}"
            ));
        }
        canonical.push_str(&format!(
            "\n{name}\t{executable_name}\t{path_text}\t{bytes}\t{sha256}"
        ));
    }
    let actual_aggregate = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    let expected_aggregate = validated_sha256(
        contents
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "native PATH tool-resolution aggregate SHA-256",
    )?;
    if actual_aggregate != expected_aggregate {
        return Err("native PATH tool-resolution aggregate mismatch".to_string());
    }
    Ok(actual_aggregate)
}

fn validate_compiled_resource_compiler_attestation(
    compiled_value: Option<&str>,
    expected_marker_value: &str,
    expected_marker_value_sha256: &str,
) -> Result<String, String> {
    let expected_marker_value_sha256 = validated_sha256(
        expected_marker_value_sha256,
        "compiled resource-compiler marker SHA-256",
    )?;
    let reconstructed_sha256 = format!("{:x}", Sha256::digest(expected_marker_value.as_bytes()));
    if reconstructed_sha256 != expected_marker_value_sha256 {
        return Err(
            "resource compiler invocation marker SHA-256 is internally inconsistent".to_string(),
        );
    }
    let compiled_value = compiled_value.ok_or_else(|| {
        "fresh soak binary omits the compiled resource-compiler attestation".to_string()
    })?;
    if compiled_value != expected_marker_value {
        return Err(
            "compiled resource-compiler attestation does not match the exact native invocation"
                .to_string(),
        );
    }
    let compiled_sha256 = format!("{:x}", Sha256::digest(compiled_value.as_bytes()));
    if compiled_sha256 != expected_marker_value_sha256 {
        return Err("compiled resource-compiler attestation SHA-256 mismatch".to_string());
    }
    Ok(compiled_sha256)
}

fn cargo_target_linker_environment_name(target_triple: &str) -> Result<String, String> {
    if target_triple.is_empty()
        || !target_triple
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("Cargo target triple cannot form a canonical linker environment name".into());
    }
    Ok(format!(
        "CARGO_TARGET_{}_LINKER",
        target_triple.replace('-', "_").to_ascii_uppercase()
    ))
}

fn validate_native_build_provenance(
    native: &Value,
    config: &ValidatedConfig,
    repo_root: &Path,
    cargo_home: &Path,
    toolchain: &Value,
) -> Result<String, String> {
    if native.get("schema_version").and_then(Value::as_u64) != Some(6)
        || native.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_native_build_provenance")
        || native.get("host_target_triple").and_then(Value::as_str)
            != Some(config.cargo_target_triple.as_str())
        || native.get("selection_policy").and_then(Value::as_str)
            != Some(
                "pinned_cmd_vcvarsall_plus_absolute_target_linker_plus_success_attested_rc_plus_locked_offline_build_chain",
            )
        || native
            .get("ambient_override_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("pinned_environment_applied")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("resource_compiler_execution_pinned")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("resource_compiler_candidate_set_attested")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("cargo_resolution_attested")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("resource_compiler_success_attested")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("resource_compiler_write_delete_lock_required")
            .and_then(Value::as_bool)
            != Some(true)
        || native
            .get("target_linker_path_pinned")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("native build provenance identity/policy is invalid".to_string());
    }
    let target_linker_environment_name =
        cargo_target_linker_environment_name(&config.cargo_target_triple)?;
    if native
        .get("target_linker_environment_name")
        .and_then(Value::as_str)
        != Some(target_linker_environment_name.as_str())
    {
        return Err("native target-linker environment name is invalid".to_string());
    }

    let system_root = native
        .get("system_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "native Windows system root is missing".to_string())?;
    let visual_studio_installation = native
        .get("visual_studio_installation")
        .and_then(Value::as_str)
        .ok_or_else(|| "native Visual Studio installation is missing".to_string())?;
    let vc_tools_version = native
        .get("vc_tools_version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "native VC tools version is missing".to_string())?;
    let windows_sdk_root = native
        .get("windows_sdk_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "native Windows SDK root is missing".to_string())?;
    let windows_sdk_version = native
        .get("windows_sdk_version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "native Windows SDK version is missing".to_string())?;
    let windows_sdk_version_key = parse_windows_sdk_version(windows_sdk_version)?;
    for (path, label) in [
        (system_root, "Windows system root"),
        (visual_studio_installation, "Visual Studio installation"),
        (windows_sdk_root, "Windows SDK root"),
    ] {
        let path = absolute_lexical_path(Path::new(path))?;
        ensure_no_reparse_ancestors(&path, label)?;
        if !path.is_dir() {
            return Err(format!("native {label} is not a directory"));
        }
    }
    let system_root_path = absolute_lexical_path(Path::new(system_root))?;
    let visual_studio_path = absolute_lexical_path(Path::new(visual_studio_installation))?;
    let windows_sdk_path = absolute_lexical_path(Path::new(windows_sdk_root))?;
    let vc_bin_path = visual_studio_path
        .join("VC")
        .join("Tools")
        .join("MSVC")
        .join(vc_tools_version)
        .join("bin")
        .join("Hostx64")
        .join("x64");
    let sdk_bin_path = windows_sdk_path
        .join("bin")
        .join(windows_sdk_version)
        .join("x64");

    let expected_tool_names = [
        "cl",
        "cmd",
        "lib",
        "link",
        "ml64",
        "mt",
        "rc",
        "vcvarsall",
        "vswhere",
    ];
    let tools = native
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "native build tools must be an array".to_string())?;
    if tools.len() != expected_tool_names.len() {
        return Err("native build tool coverage is incomplete".to_string());
    }
    let mut tool_canonical = String::new();
    let mut attested_tools = BTreeMap::<String, (String, PathBuf, u64, String)>::new();
    let mut command_processor_path_text: Option<String> = None;
    let mut resource_compiler_tool: Option<(String, PathBuf, u64, String)> = None;
    for (index, (tool, expected_name)) in tools.iter().zip(expected_tool_names.iter()).enumerate() {
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "native build tool name is missing".to_string())?;
        if name != *expected_name {
            return Err("native build tools are not complete and canonically sorted".to_string());
        }
        let path_text = tool
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("native build tool path is missing: {name}"))?;
        let path = absolute_lexical_path(Path::new(path_text))?;
        ensure_no_reparse_ancestors(&path, &format!("native build tool {name}"))?;
        let expected_path = match name {
            "cl" => Some(vc_bin_path.join("cl.exe")),
            "cmd" => Some(system_root_path.join("System32").join("cmd.exe")),
            "lib" => Some(vc_bin_path.join("lib.exe")),
            "link" => Some(vc_bin_path.join("link.exe")),
            "ml64" => Some(vc_bin_path.join("ml64.exe")),
            "mt" => Some(sdk_bin_path.join("mt.exe")),
            "rc" => Some(sdk_bin_path.join("rc.exe")),
            "vcvarsall" => Some(
                visual_studio_path
                    .join("VC")
                    .join("Auxiliary")
                    .join("Build")
                    .join("vcvarsall.bat"),
            ),
            "vswhere" => None,
            _ => return Err(format!("unexpected native build tool: {name}")),
        };
        if expected_path
            .as_ref()
            .is_some_and(|expected| !paths_equal(&path, expected))
        {
            return Err(format!(
                "native build tool escaped the selected Windows/VS/SDK path: {name}"
            ));
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!(
                "native build tool must be an ordinary file: {name}"
            ));
        }
        let expected_bytes = tool
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("native build tool byte count is missing: {name}"))?;
        if metadata.len() != expected_bytes {
            return Err(format!("native build tool byte count changed: {name}"));
        }
        let expected_sha256 = validated_sha256(
            tool.get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("native build tool SHA-256 for {name}"),
        )?;
        if sha256_file(&path)? != expected_sha256 {
            return Err(format!("native build tool SHA-256 changed: {name}"));
        }
        if index > 0 {
            tool_canonical.push('\n');
        }
        tool_canonical.push_str(&format!(
            "{name}\t{path_text}\t{expected_bytes}\t{expected_sha256}"
        ));
        if name == "rc" {
            resource_compiler_tool = Some((
                path_text.to_string(),
                path.clone(),
                expected_bytes,
                expected_sha256.clone(),
            ));
        }
        if name == "cmd" {
            command_processor_path_text = Some(path_text.to_string());
        }
        attested_tools.insert(
            name.to_string(),
            (path_text.to_string(), path, expected_bytes, expected_sha256),
        );
    }
    let resource_compiler_tool = resource_compiler_tool
        .ok_or_else(|| "native rc tool attestation is missing".to_string())?;
    let command_processor_path_text = command_processor_path_text
        .ok_or_else(|| "native cmd.exe attestation is missing".to_string())?;
    if native
        .get("vcvars_environment_selection_policy")
        .and_then(Value::as_str)
        != Some("all_changed_or_added_vcvars_names_plus_explicit_target_linker_and_rc_attestation")
    {
        return Err("native vcvars environment-selection policy is invalid".to_string());
    }
    let vcvars_environment_names = native
        .get("vcvars_environment_names")
        .and_then(Value::as_array)
        .ok_or_else(|| "native vcvars environment-name set is missing".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "native vcvars environment names must be strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if vcvars_environment_names.is_empty()
        || native
            .get("vcvars_environment_name_count")
            .and_then(Value::as_u64)
            != Some(vcvars_environment_names.len() as u64)
        || vcvars_environment_names
            .windows(2)
            .any(|window| window[0] >= window[1])
        || vcvars_environment_names.iter().any(|name| {
            matches!(
                *name,
                "COMSPEC" | "EMBED_RESOURCE_RC_PATH" | "EMBED_RESOURCE_RC_ATTESTATION_NONCE"
            ) || *name == target_linker_environment_name.as_str()
                || *name != name.to_ascii_uppercase()
                || name
                    .bytes()
                    .any(|byte| matches!(byte, b'\t' | b'\r' | b'\n'))
        })
    {
        return Err("native vcvars environment-name set is not canonical".to_string());
    }
    let mut expected_environment_names = vcvars_environment_names.clone();
    expected_environment_names.extend([
        target_linker_environment_name.as_str(),
        "COMSPEC",
        "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
        "EMBED_RESOURCE_RC_PATH",
    ]);
    expected_environment_names.sort_unstable();
    expected_environment_names.dedup();
    let environment = native
        .get("environment")
        .and_then(Value::as_array)
        .ok_or_else(|| "native build environment manifest is missing".to_string())?;
    if environment.len() != expected_environment_names.len() {
        return Err("native build environment coverage is incomplete".to_string());
    }
    let mut environment_canonical = String::new();
    let mut environment_hashes = BTreeMap::<String, String>::new();
    let required_environment_names = [
        "PATH",
        "COMSPEC",
        "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
        "EMBED_RESOURCE_RC_PATH",
        "EXTERNAL_INCLUDE",
        "FRAMEWORK40VERSION",
        "__DOTNET_ADD_64BIT",
        "__DOTNET_PREFERRED_BITNESS",
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "VSINSTALLDIR",
        "VCINSTALLDIR",
        "VCTOOLSINSTALLDIR",
        "VCTOOLSVERSION",
        "WINDOWSSDKDIR",
        "WINDOWSSDKVERSION",
        "UNIVERSALCRTSDKDIR",
        "UCRTVERSION",
        "VSCMD_ARG_HOST_ARCH",
        "VSCMD_ARG_TGT_ARCH",
        "VSCMD_ARG_VCVARS_VER",
        "VSCMD_ARG_WINSDK",
    ];
    for required in required_environment_names {
        if !expected_environment_names.contains(&required) {
            return Err(format!(
                "required pinned native environment name is missing: {required}"
            ));
        }
    }
    for (index, (row, expected_name)) in environment
        .iter()
        .zip(expected_environment_names.iter())
        .enumerate()
    {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "native build environment name is missing".to_string())?;
        if name != *expected_name {
            return Err("native build environment names are not canonical".to_string());
        }
        let expected_present = row
            .get("present")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("native build environment presence is missing: {name}"))?;
        if !expected_present {
            return Err(format!(
                "selected pinned native build environment entry is absent: {name}"
            ));
        }
        let expected_sha256 = row.get("value_sha256").and_then(Value::as_str);
        let expected_sha256 = validated_sha256(
            expected_sha256.unwrap_or_default(),
            &format!("native build environment SHA-256 for {name}"),
        )?;
        environment_hashes.insert(name.to_string(), expected_sha256.clone());
        if index > 0 {
            environment_canonical.push('\n');
        }
        environment_canonical.push_str(&format!(
            "{name}\t{}\t{}",
            expected_present, expected_sha256
        ));
    }
    let expected_environment_aggregate = validated_sha256(
        native
            .get("environment_aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "native build environment aggregate SHA-256",
    )?;
    let actual_environment_aggregate =
        format!("{:x}", Sha256::digest(environment_canonical.as_bytes()));
    if actual_environment_aggregate != expected_environment_aggregate {
        return Err("native build environment aggregate mismatch".to_string());
    }
    let expected_comspec_hash = format!(
        "{:x}",
        Sha256::digest(command_processor_path_text.as_bytes())
    );
    if environment_hashes.get("COMSPEC").map(String::as_str) != Some(expected_comspec_hash.as_str())
    {
        return Err("pinned COMSPEC does not match attested system cmd.exe".to_string());
    }
    let target_linker = attested_tools
        .get("link")
        .ok_or_else(|| "attested native target linker is missing".to_string())?;
    let expected_target_linker_hash = format!("{:x}", Sha256::digest(target_linker.0.as_bytes()));
    if environment_hashes
        .get(&target_linker_environment_name)
        .map(String::as_str)
        != Some(expected_target_linker_hash.as_str())
    {
        return Err(
            "absolute Cargo target-linker environment does not match attested link.exe".to_string(),
        );
    }
    let path_tool_resolution_aggregate = validate_path_tool_resolution(
        native
            .get("path_tool_resolution")
            .ok_or_else(|| "native PATH tool-resolution evidence is missing".to_string())?,
        &environment_hashes,
        &attested_tools,
    )?;

    let candidates = native
        .get("resource_compiler_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "native resource compiler candidates must be an array".to_string())?;
    if candidates.is_empty() {
        return Err("native resource compiler candidate coverage is empty".to_string());
    }
    let enumerated_candidates = enumerate_resource_compiler_candidates(&windows_sdk_path)?;
    if candidates.len() != enumerated_candidates.len() {
        return Err(
            "native resource compiler candidate evidence omits an installed SDK candidate"
                .to_string(),
        );
    }
    let mut candidate_canonical = String::new();
    let mut prior_version: Option<[i32; 4]> = None;
    let mut selected_resource_compiler: Option<(String, PathBuf, u64, String)> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let version = candidate
            .get("version")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "native resource compiler version is missing".to_string())?;
        let version_key = parse_windows_sdk_version(version)?;
        if prior_version.is_some_and(|prior| prior >= version_key) {
            return Err("native resource compiler candidates must be strictly sorted".to_string());
        }
        prior_version = Some(version_key);
        let path_text = candidate
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("resource compiler candidate path is missing: {version}"))?;
        let path = absolute_lexical_path(Path::new(path_text))?;
        if enumerated_candidates
            .get(index)
            .is_none_or(|(actual_version, actual_path)| {
                actual_version != version || !paths_equal(actual_path, &path)
            })
        {
            return Err(
                "native resource compiler candidate set differs from SDK enumeration".to_string(),
            );
        }
        ensure_no_reparse_ancestors(&path, "native resource compiler candidate")?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata_is_reparse(&metadata) {
            return Err(format!(
                "native resource compiler candidate must be an ordinary file: {version}"
            ));
        }
        let expected_bytes = candidate
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                format!("native resource compiler candidate byte count is missing: {version}")
            })?;
        if metadata.len() != expected_bytes {
            return Err(format!(
                "native resource compiler candidate byte count changed: {version}"
            ));
        }
        let expected_sha256 = validated_sha256(
            candidate
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("native resource compiler candidate SHA-256 for {version}"),
        )?;
        if sha256_file(&path)? != expected_sha256 {
            return Err(format!(
                "native resource compiler candidate SHA-256 changed: {version}"
            ));
        }
        if version == windows_sdk_version {
            if version_key != windows_sdk_version_key || selected_resource_compiler.is_some() {
                return Err(
                    "selected Windows SDK resource compiler candidate is ambiguous".to_string(),
                );
            }
            selected_resource_compiler = Some((
                path_text.to_string(),
                path,
                expected_bytes,
                expected_sha256.clone(),
            ));
        }
        if index > 0 {
            candidate_canonical.push('\n');
        }
        candidate_canonical.push_str(&format!(
            "{version}\t{path_text}\t{expected_bytes}\t{expected_sha256}"
        ));
    }
    let selected_resource_compiler = selected_resource_compiler.ok_or_else(|| {
        "selected Windows SDK resource compiler is missing from the candidate set".to_string()
    })?;
    let execution = native
        .get("resource_compiler_execution")
        .ok_or_else(|| "native resource compiler execution attestation is missing".to_string())?;
    if execution.get("environment_name").and_then(Value::as_str) != Some("EMBED_RESOURCE_RC_PATH") {
        return Err("native resource compiler override environment is invalid".to_string());
    }
    let execution_path_text = execution
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "native resource compiler execution path is missing".to_string())?;
    let execution_path = absolute_lexical_path(Path::new(execution_path_text))?;
    ensure_no_reparse_ancestors(&execution_path, "native resource compiler execution")?;
    let execution_bytes = execution
        .get("bytes")
        .and_then(Value::as_u64)
        .ok_or_else(|| "native resource compiler execution byte count is missing".to_string())?;
    let execution_sha256 = validated_sha256(
        execution
            .get("sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "native resource compiler execution SHA-256",
    )?;
    let expected_execution_path = absolute_lexical_path(
        &Path::new(windows_sdk_root)
            .join("bin")
            .join(windows_sdk_version)
            .join("x64")
            .join("rc.exe"),
    )?;
    if !paths_equal(&execution_path, &expected_execution_path)
        || !paths_equal(&execution_path, &resource_compiler_tool.1)
        || !paths_equal(&execution_path, &selected_resource_compiler.1)
        || execution_path_text != resource_compiler_tool.0
        || execution_path_text != selected_resource_compiler.0
        || execution_bytes != resource_compiler_tool.2
        || execution_bytes != selected_resource_compiler.2
        || execution_sha256 != resource_compiler_tool.3
        || execution_sha256 != selected_resource_compiler.3
    {
        return Err(
            "native resource compiler execution does not match the exact selected rc.exe"
                .to_string(),
        );
    }
    let expected_override_hash = format!("{:x}", Sha256::digest(execution_path_text.as_bytes()));
    if environment_hashes
        .get("EMBED_RESOURCE_RC_PATH")
        .map(String::as_str)
        != Some(expected_override_hash.as_str())
    {
        return Err(
            "native resource compiler override environment hash does not match its exact path"
                .to_string(),
        );
    }
    let resource_compiler_execution_canonical = format!(
        "EMBED_RESOURCE_RC_PATH\t{execution_path_text}\t{execution_bytes}\t{execution_sha256}"
    );
    let attestation_nonce = native
        .get("resource_compiler_attestation_nonce")
        .and_then(Value::as_str)
        .filter(|value| {
            value.len() == 32
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(|| "resource compiler attestation nonce is invalid".to_string())?;
    let expected_nonce_hash = format!("{:x}", Sha256::digest(attestation_nonce.as_bytes()));
    if environment_hashes
        .get("EMBED_RESOURCE_RC_ATTESTATION_NONCE")
        .map(String::as_str)
        != Some(expected_nonce_hash.as_str())
    {
        return Err(
            "resource compiler attestation nonce does not match the pinned environment".to_string(),
        );
    }
    let cargo_resolution = native
        .get("cargo_resolution")
        .ok_or_else(|| "Cargo build-chain provenance is missing".to_string())?;
    let cargo_resolution_aggregate = validate_embed_resource_cargo_resolution(
        cargo_resolution,
        repo_root,
        cargo_home,
        &config.cargo_target_triple,
        toolchain,
    )?;
    let root_package_id = cargo_resolution
        .pointer("/root_package/package_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "Cargo build-chain root package ID is missing".to_string())?;
    let native_input_aggregate = validate_native_input_manifest(
        native
            .get("native_inputs")
            .ok_or_else(|| "native input manifest is missing".to_string())?,
        &visual_studio_path,
        vc_tools_version,
        &windows_sdk_path,
        &environment_hashes,
    )?;
    let vcvars_environment_canonical = vcvars_environment_names.join("\t");
    let pre_execution_canonical = format!(
        "host_target_triple\t{}\nsystem_root\t{}\nvisual_studio_installation\t{}\nvc_tools_version\t{}\nwindows_sdk_root\t{}\nwindows_sdk_version\t{}\ntools\t{}\npath_tool_resolution\t{}\nresource_compiler_execution\t{}\nresource_compiler_candidates\t{}\ncargo_resolution\t{}\nnative_inputs\t{}\nvcvars_environment_names\t{}\nenvironment\t{}",
        config.cargo_target_triple,
        system_root,
        visual_studio_installation,
        vc_tools_version,
        windows_sdk_root,
        windows_sdk_version,
        tool_canonical,
        path_tool_resolution_aggregate,
        resource_compiler_execution_canonical,
        candidate_canonical,
        cargo_resolution_aggregate,
        native_input_aggregate,
        vcvars_environment_canonical,
        environment_canonical
    );
    let actual_pre_execution_aggregate =
        format!("{:x}", Sha256::digest(pre_execution_canonical.as_bytes()));
    let expected_pre_execution_aggregate = validated_sha256(
        native
            .get("pre_execution_aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "native pre-execution aggregate SHA-256",
    )?;
    if actual_pre_execution_aggregate != expected_pre_execution_aggregate {
        return Err("native pre-execution aggregate mismatch".to_string());
    }

    let invocation = native
        .get("resource_compiler_invocation")
        .ok_or_else(|| "resource compiler success invocation is missing".to_string())?;
    let invocation_out_dir_text = invocation
        .get("build_script_out_dir")
        .and_then(Value::as_str)
        .ok_or_else(|| "resource compiler build-script output directory is missing".to_string())?;
    let invocation_out_dir = absolute_lexical_path(Path::new(invocation_out_dir_text))?;
    ensure_no_reparse_ancestors(
        &invocation_out_dir,
        "resource compiler build-script output directory",
    )?;
    if !invocation_out_dir.is_dir()
        || !path_starts_with(&invocation_out_dir, &config.cargo_target_dir)
    {
        return Err(
            "resource compiler build-script output escaped the attested Cargo target".to_string(),
        );
    }
    let marker_value = format!("{attestation_nonce}|{execution_path_text}");
    let marker_value_sha256 = format!("{:x}", Sha256::digest(marker_value.as_bytes()));
    if invocation.get("schema_version").and_then(Value::as_u64) != Some(1)
        || invocation.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_resource_compiler_invocation")
        || invocation
            .get("build_script_event_match_count")
            .and_then(Value::as_u64)
            != Some(1)
        || invocation.get("marker_match_count").and_then(Value::as_u64) != Some(1)
        || invocation.get("package_id").and_then(Value::as_str) != Some(root_package_id)
        || invocation.get("package_name").and_then(Value::as_str) != Some("ta_desktop_demo_app")
        || invocation.get("target_triple").and_then(Value::as_str)
            != Some(config.cargo_target_triple.as_str())
        || invocation.get("environment_name").and_then(Value::as_str)
            != Some("VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION")
        || invocation.get("nonce").and_then(Value::as_str) != Some(attestation_nonce)
        || invocation
            .get("resource_compiler_path")
            .and_then(Value::as_str)
            != Some(execution_path_text)
        || invocation
            .get("resource_compiler_bytes")
            .and_then(Value::as_u64)
            != Some(execution_bytes)
        || invocation
            .get("resource_compiler_sha256")
            .and_then(Value::as_str)
            != Some(execution_sha256.as_str())
        || invocation
            .get("marker_value_sha256")
            .and_then(Value::as_str)
            != Some(marker_value_sha256.as_str())
        || invocation
            .get("file_lock_held_during_build")
            .and_then(Value::as_bool)
            != Some(true)
        || invocation
            .get("file_lock_share_mode")
            .and_then(Value::as_str)
            != Some("read_access_share_read_only_denies_write_delete")
    {
        return Err(
            "resource compiler success invocation does not match the exact app build".to_string(),
        );
    }
    let compiled_resource_compiler_attestation_sha256 =
        validate_compiled_resource_compiler_attestation(
            COMPILED_RESOURCE_COMPILER_ATTESTATION,
            &marker_value,
            &marker_value_sha256,
        )?;
    let invocation_canonical = format!(
        "{root_package_id}\tta_desktop_demo_app\t{}\t{invocation_out_dir_text}\tVECTORGRID_EMBED_RESOURCE_RC_ATTESTATION\t{attestation_nonce}\t{execution_path_text}\t{execution_bytes}\t{execution_sha256}\t{marker_value_sha256}\tread_access_share_read_only_denies_write_delete",
        config.cargo_target_triple
    );
    let aggregate_canonical =
        format!("{pre_execution_canonical}\nresource_compiler_invocation\t{invocation_canonical}");
    let expected_aggregate = validated_sha256(
        native
            .get("aggregate_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "native build provenance aggregate SHA-256",
    )?;
    let actual_aggregate = format!("{:x}", Sha256::digest(aggregate_canonical.as_bytes()));
    if actual_aggregate != expected_aggregate {
        return Err("native build provenance aggregate mismatch".to_string());
    }
    Ok(compiled_resource_compiler_attestation_sha256)
}

fn validate_launcher_manifest(
    contents: &Value,
    source_manifest: &Value,
    config: &ValidatedConfig,
) -> Result<Option<String>, String> {
    if contents.get("schema_version").and_then(Value::as_u64) != Some(1)
        || contents.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_launcher_manifest")
        || contents.get("run_id").and_then(Value::as_str) != Some(config.run_id.as_str())
    {
        return Err("launcher manifest identity is invalid".to_string());
    }
    let build = contents
        .get("build")
        .ok_or_else(|| "launcher manifest build section is missing".to_string())?;
    if build.get("profile").and_then(Value::as_str) != Some(config.build_profile.as_str())
        || build.get("fresh_build_invocation").and_then(Value::as_bool) != Some(config.fresh_build)
        || build.get("cargo_artifact_emitted").and_then(Value::as_bool)
            != Some(config.cargo_artifact_emitted)
        || build.get("cargo_artifact_fresh").and_then(Value::as_bool)
            != Some(config.cargo_artifact_fresh)
        || build.get("cargo_build_target_env").and_then(Value::as_str)
            != Some(config.cargo_build_target_env.as_str())
        || build.get("cargo_target_triple").and_then(Value::as_str)
            != Some(config.cargo_target_triple.as_str())
        || build
            .get("cargo_target_dir")
            .and_then(Value::as_str)
            .map(Path::new)
            != Some(config.cargo_target_dir.as_path())
        || build
            .get("cargo_artifact_path")
            .and_then(Value::as_str)
            .map(Path::new)
            != Some(config.cargo_artifact_path.as_path())
        || build
            .get("cargo_target_dir_cli_pinned")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build
            .get("cargo_target_triple_cli_pinned")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build
            .get("target_dir_initially_empty")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build.get("target_cleanup_marker").and_then(Value::as_str)
            != Some(".vectorgrid-cpu-reliability-soak-target-owner.json")
        || build
            .get("recovered_stale_cargo_targets")
            .and_then(Value::as_u64)
            .is_none()
        || build
            .get("compiler_override_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
        || build
            .get("native_build_override_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
        || build
            .get("native_build_environment_attested")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build
            .get("compiler_path_pinned_during_build")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build
            .get("target_linker_path_pinned_during_build")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build
            .get("cargo_execution_uses_toolchain_binary")
            .and_then(Value::as_bool)
            != Some(config.fresh_build)
        || build
            .get("profile_override_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
        || build
            .get("incremental_compilation_disabled")
            .and_then(Value::as_bool)
            != Some(true)
        || build
            .get("tauri_config_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
        || build
            .get("tauri_target_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
        || build
            .get("custom_protocol_disabled")
            .and_then(Value::as_bool)
            != Some(true)
        || build.get("context_asset_mode").and_then(Value::as_str)
            != Some("dev_url_no_embedded_frontend_assets")
        || build
            .get("frontend_dist_tree_effective")
            .and_then(Value::as_bool)
            != Some(false)
        || build
            .get("cargo_configuration_environment_cleared_except_allowlist")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("launcher manifest build provenance is inconsistent".to_string());
    }
    let cargo_target_root = build
        .get("cargo_target_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher manifest cargo_target_root is missing".to_string())?;
    let cargo_target_root = absolute_lexical_path(Path::new(cargo_target_root))?;
    if config.fresh_build
        && (config.cargo_target_dir.parent() != Some(cargo_target_root.as_path())
            || !config
                .cargo_target_dir
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("cpu-reliability-soak-")))
    {
        return Err("fresh Cargo target is not an owned direct child of its root".to_string());
    }
    let build_temp_dir = build
        .get("build_temp_dir")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher manifest build_temp_dir is missing".to_string())?;
    let build_temp_dir = absolute_lexical_path(Path::new(build_temp_dir))?;
    if build_temp_dir == config.cargo_target_dir
        || !build_temp_dir.starts_with(&config.cargo_target_dir)
    {
        return Err("launcher manifest build temp escaped Cargo target dir".to_string());
    }
    let cargo_arguments = build
        .get("cargo_arguments")
        .and_then(Value::as_array)
        .ok_or_else(|| "launcher manifest cargo_arguments must be an array".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "launcher manifest cargo_arguments must be strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let features = build
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| "launcher manifest features must be an array".to_string())?;
    if config.fresh_build {
        let mut expected_arguments = vec![
            "build",
            "--locked",
            "--offline",
            "--target-dir",
            config
                .cargo_target_dir
                .to_str()
                .ok_or_else(|| "cargo_target_dir must be valid Unicode".to_string())?,
            "--target",
            config.cargo_target_triple.as_str(),
            "-p",
            "ta_desktop_demo_app",
            "--no-default-features",
            "--features",
            "maintainer-tools",
            "--bin",
            "ta_cpu_reliability_soak",
        ];
        if config.build_profile == "release" {
            expected_arguments.push("--release");
        }
        expected_arguments.push("--message-format=json-render-diagnostics");
        if build.get("locked").and_then(Value::as_bool) != Some(true)
            || build.get("offline").and_then(Value::as_bool) != Some(true)
            || build.get("no_default_features").and_then(Value::as_bool) != Some(true)
            || features.len() != 1
            || features[0].as_str() != Some("maintainer-tools")
            || cargo_arguments != expected_arguments
        {
            return Err(
                "launcher manifest omits the exact locked/offline CPU build contract".to_string(),
            );
        }
    } else if build.get("locked").and_then(Value::as_bool) != Some(false)
        || build.get("offline").and_then(Value::as_bool) != Some(false)
        || build.get("no_default_features").and_then(Value::as_bool) != Some(false)
        || !features.is_empty()
        || !cargo_arguments.is_empty()
    {
        return Err("launcher manifest prebuilt-binary contract is inconsistent".to_string());
    }
    let binary = contents
        .get("binary")
        .ok_or_else(|| "launcher manifest binary section is missing".to_string())?;
    if binary.get("sha256").and_then(Value::as_str) != Some(config.binary_sha256.as_str())
        || binary.get("path").and_then(Value::as_str).map(Path::new)
            != Some(config.cargo_artifact_path.as_path())
    {
        return Err("launcher manifest binary SHA-256 is inconsistent".to_string());
    }
    let retained_binary_path = binary
        .get("retained_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher retained binary path is missing".to_string())?;
    let retained_binary_path = absolute_lexical_path(Path::new(retained_binary_path))?;
    if retained_binary_path.parent() != config.launcher_manifest.parent() {
        return Err(
            "retained binary must be a direct sibling of the launcher manifest".to_string(),
        );
    }
    ensure_no_reparse_ancestors(&retained_binary_path, "retained soak binary")?;
    let retained_metadata =
        fs::symlink_metadata(&retained_binary_path).map_err(|error| error.to_string())?;
    if !retained_metadata.is_file() || retained_metadata.file_type().is_symlink() {
        return Err("retained soak binary must be an ordinary file".to_string());
    }
    if sha256_file(&retained_binary_path)? != config.binary_sha256 {
        return Err("retained soak binary SHA-256 differs from the executable".to_string());
    }
    let source = contents
        .get("source_manifest")
        .ok_or_else(|| "launcher source manifest section is missing".to_string())?;
    if source.get("sha256").and_then(Value::as_str) != Some(config.source_manifest_sha256.as_str())
        || source.get("path").and_then(Value::as_str).map(Path::new)
            != Some(config.source_manifest.as_path())
        || source.get("aggregate_sha256").and_then(Value::as_str)
            != source_manifest
                .get("aggregate_sha256")
                .and_then(Value::as_str)
        || source.get("file_count").and_then(Value::as_u64)
            != source_manifest.get("file_count").and_then(Value::as_u64)
    {
        return Err("launcher source-manifest provenance is inconsistent".to_string());
    }
    let cargo_configuration = contents
        .get("cargo_configuration")
        .ok_or_else(|| "launcher Cargo configuration section is missing".to_string())?;
    validate_cargo_configuration_provenance(cargo_configuration)?;
    let repo_root = cargo_configuration
        .get("repo_root")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher Cargo configuration omits repo_root".to_string())?;
    let repo_root = absolute_lexical_path(Path::new(repo_root))?;
    ensure_no_reparse_ancestors(&repo_root, "launcher repository root")?;
    if !repo_root.is_dir() {
        return Err("launcher repository root is not a directory".to_string());
    }
    let cargo_home = cargo_configuration
        .get("cargo_home")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher Cargo configuration omits cargo_home".to_string())?;
    let cargo_home = absolute_lexical_path(Path::new(cargo_home))?;
    ensure_no_reparse_ancestors(&cargo_home, "launcher Cargo home")?;
    if !cargo_home.is_dir() {
        return Err("launcher Cargo home is not a directory".to_string());
    }
    let toolchain = contents
        .get("toolchain")
        .ok_or_else(|| "launcher toolchain section is missing".to_string())?;
    let rustc_sysroot = toolchain
        .get("rustc_sysroot")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher manifest rustc_sysroot is missing".to_string())?;
    let rustc_sysroot = absolute_lexical_path(Path::new(rustc_sysroot))?;
    for (path_name, sha_name) in [
        ("cargo_path", "cargo_sha256"),
        ("cargo_invocation_path", "cargo_invocation_sha256"),
        ("rustc_path", "rustc_sha256"),
        ("rustc_invocation_path", "rustc_invocation_sha256"),
    ] {
        let tool_path = toolchain
            .get(path_name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("launcher manifest {path_name} is missing"))?;
        let tool_path = absolute_lexical_path(Path::new(tool_path))?;
        let metadata = fs::symlink_metadata(&tool_path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!(
                "launcher manifest {path_name} must identify an ordinary file"
            ));
        }
        let expected_sha256 = validated_sha256(
            toolchain
                .get(sha_name)
                .and_then(Value::as_str)
                .unwrap_or_default(),
            &format!("launcher manifest {sha_name}"),
        )?;
        let actual_sha256 = sha256_file(&tool_path)?;
        if actual_sha256 != expected_sha256 {
            return Err(format!(
                "launcher manifest {path_name} SHA-256 mismatch: expected {expected_sha256}, actual {actual_sha256}"
            ));
        }
    }
    let expected_cargo_path = rustc_sysroot
        .join("bin")
        .join(format!("cargo{}", std::env::consts::EXE_SUFFIX));
    let expected_rustc_path = rustc_sysroot
        .join("bin")
        .join(format!("rustc{}", std::env::consts::EXE_SUFFIX));
    let declared_tool_path = |name: &str| -> Result<PathBuf, String> {
        let raw = toolchain
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("launcher manifest {name} is missing"))?;
        absolute_lexical_path(Path::new(raw))
    };
    let cargo_invocation_path = declared_tool_path("cargo_invocation_path")?;
    let rustc_invocation_path = declared_tool_path("rustc_invocation_path")?;
    if declared_tool_path("cargo_path")? != expected_cargo_path
        || declared_tool_path("rustc_path")? != expected_rustc_path
        || cargo_invocation_path != rustc_invocation_path
        || cargo_invocation_path
            .file_name()
            .and_then(|value| value.to_str())
            .is_none_or(|value| {
                !value.eq_ignore_ascii_case(&format!("rustup{}", std::env::consts::EXE_SUFFIX))
            })
    {
        return Err(
            "launcher tool selection does not bind direct sysroot Cargo/rustc through rustup discovery"
                .to_string(),
        );
    }
    if toolchain
        .get("cargo_version_verbose")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || toolchain
            .get("rustc_version_verbose")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || toolchain
            .get("selected_cargo_version_verbose")
            .and_then(Value::as_str)
            != toolchain
                .get("cargo_version_verbose")
                .and_then(Value::as_str)
        || toolchain
            .get("selected_rustc_version_verbose")
            .and_then(Value::as_str)
            != toolchain
                .get("rustc_version_verbose")
                .and_then(Value::as_str)
        || toolchain.get("host_target_triple").and_then(Value::as_str)
            != Some(config.cargo_target_triple.as_str())
        || contents
            .pointer("/host/logical_processor_count")
            .and_then(Value::as_u64)
            .is_none_or(|value| value == 0)
    {
        return Err("launcher environment/toolchain provenance is incomplete".to_string());
    }
    validate_toolchain_input_manifest(
        toolchain
            .get("input_manifest")
            .ok_or_else(|| "launcher toolchain input manifest is missing".to_string())?,
        &rustc_sysroot,
        &config.cargo_target_triple,
    )?;
    let compiled_resource_compiler_attestation_sha256 = if config.fresh_build {
        Some(validate_native_build_provenance(
            contents
                .get("native_build")
                .ok_or_else(|| "launcher native build provenance is missing".to_string())?,
            config,
            &repo_root,
            &cargo_home,
            toolchain,
        )?)
    } else if contents
        .get("native_build")
        .is_some_and(|value| !value.is_null())
    {
        return Err("prebuilt launcher must not claim fresh native build provenance".to_string());
    } else {
        None
    };
    Ok(compiled_resource_compiler_attestation_sha256)
}

fn load_execution_provenance(
    config: &ValidatedConfig,
    benchmark_mutex_contention_verified: bool,
) -> Result<Value, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable = fs::canonicalize(&executable).map_err(|error| error.to_string())?;
    let cargo_target_dir =
        fs::canonicalize(&config.cargo_target_dir).map_err(|error| error.to_string())?;
    if !path_starts_with(&executable, &cargo_target_dir) {
        return Err(format!(
            "running executable is outside the attested Cargo target directory: {}",
            executable.display()
        ));
    }
    let cargo_artifact_path =
        fs::canonicalize(&config.cargo_artifact_path).map_err(|error| error.to_string())?;
    if cargo_artifact_path != executable {
        return Err(format!(
            "running executable differs from Cargo-emitted artifact: expected {}, actual {}",
            cargo_artifact_path.display(),
            executable.display()
        ));
    }
    let actual_cargo_build_target = std::env::var("CARGO_BUILD_TARGET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "host-default".to_string());
    if actual_cargo_build_target != config.cargo_build_target_env {
        return Err(format!(
            "CARGO_BUILD_TARGET changed: expected {}, actual {}",
            config.cargo_build_target_env, actual_cargo_build_target
        ));
    }
    let actual_binary_sha256 = sha256_file(&executable)?;
    if actual_binary_sha256 != config.binary_sha256 {
        return Err(format!(
            "running binary SHA-256 mismatch: expected {}, actual {}",
            config.binary_sha256, actual_binary_sha256
        ));
    }

    let source_metadata =
        fs::symlink_metadata(&config.source_manifest).map_err(|error| error.to_string())?;
    if !source_metadata.is_file() || source_metadata.file_type().is_symlink() {
        return Err("source manifest must be an ordinary file".to_string());
    }
    let actual_source_manifest_sha256 = sha256_file(&config.source_manifest)?;
    if actual_source_manifest_sha256 != config.source_manifest_sha256 {
        return Err(format!(
            "source manifest SHA-256 mismatch: expected {}, actual {}",
            config.source_manifest_sha256, actual_source_manifest_sha256
        ));
    }
    let source_manifest_bytes =
        fs::read(&config.source_manifest).map_err(|error| error.to_string())?;
    let source_manifest: Value =
        serde_json::from_slice(&source_manifest_bytes).map_err(|error| error.to_string())?;
    validate_source_manifest(&source_manifest)?;

    let launcher_metadata =
        fs::symlink_metadata(&config.launcher_manifest).map_err(|error| error.to_string())?;
    if !launcher_metadata.is_file() || launcher_metadata.file_type().is_symlink() {
        return Err("launcher manifest must be an ordinary file".to_string());
    }
    let actual_launcher_manifest_sha256 = sha256_file(&config.launcher_manifest)?;
    if actual_launcher_manifest_sha256 != config.launcher_manifest_sha256 {
        return Err(format!(
            "launcher manifest SHA-256 mismatch: expected {}, actual {}",
            config.launcher_manifest_sha256, actual_launcher_manifest_sha256
        ));
    }
    let launcher_manifest_bytes =
        fs::read(&config.launcher_manifest).map_err(|error| error.to_string())?;
    let launcher_manifest: Value =
        serde_json::from_slice(&launcher_manifest_bytes).map_err(|error| error.to_string())?;
    let compiled_resource_compiler_attestation_sha256 =
        validate_launcher_manifest(&launcher_manifest, &source_manifest, config)?;
    let retained_binary_path = launcher_manifest
        .pointer("/binary/retained_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher retained binary path is missing".to_string())?;
    let retained_binary_path = absolute_lexical_path(Path::new(retained_binary_path))?;
    let retained_binary_sha256 = sha256_file(&retained_binary_path)?;
    let compiled_resource_compiler_attestation_matched =
        compiled_resource_compiler_attestation_sha256.is_some();

    Ok(json!({
        "cargo_target_dir": config.cargo_target_dir,
        "build_profile": config.build_profile,
        "cargo_build_target_env": config.cargo_build_target_env,
        "cargo_target_triple": config.cargo_target_triple,
        "cargo_artifact_path": cargo_artifact_path,
        "cargo_artifact_emitted": config.cargo_artifact_emitted,
        "cargo_artifact_fresh": config.cargo_artifact_fresh,
        "fresh_build": config.fresh_build,
        "binary": {
            "path": retained_binary_path,
            "execution_path": config.cargo_artifact_path,
            "sha256": actual_binary_sha256,
            "expected_sha256": config.binary_sha256,
            "matched_expected": true,
            "retained_copy_sha256": retained_binary_sha256,
            "retained_copy_matched_expected": retained_binary_sha256 == config.binary_sha256,
        },
        "source_manifest": {
            "path": config.source_manifest,
            "sha256": actual_source_manifest_sha256,
            "expected_sha256": config.source_manifest_sha256,
            "matched_expected": true,
            "contents": source_manifest,
        },
        "launcher_manifest": {
            "path": config.launcher_manifest,
            "sha256": actual_launcher_manifest_sha256,
            "expected_sha256": config.launcher_manifest_sha256,
            "matched_expected": true,
            "contents": launcher_manifest,
        },
        "compiled_resource_compiler_attestation": {
            "environment_name": "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION",
            "required": config.fresh_build,
            "present": COMPILED_RESOURCE_COMPILER_ATTESTATION.is_some(),
            "value_sha256": compiled_resource_compiler_attestation_sha256,
            "matched_native_invocation":
                compiled_resource_compiler_attestation_matched,
        },
        "benchmark_lock": {
            "inherited": std::env::var("TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD").ok().as_deref() == Some("1"),
            "mutex_contention_verified": benchmark_mutex_contention_verified,
            "owner_pid": std::env::var("TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID").ok(),
            "context": std::env::var("TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT").ok(),
        },
        "recovered_stale_isolations": config.recovered_stale_isolations,
    }))
}

#[cfg(windows)]
fn verify_inherited_benchmark_mutex_contention() -> Result<bool, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{
        CloseHandle, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

    const EXPECTED_MUTEX: &str = "Local\\ta_desktop_demo_benchmark_gate";
    if std::env::var("TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD")
        .ok()
        .as_deref()
        != Some("1")
        || std::env::var("TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME")
            .ok()
            .as_deref()
            != Some(EXPECTED_MUTEX)
    {
        return Err("CPU reliability soak requires the wrapper-owned benchmark mutex".to_string());
    }
    let name: Vec<u16> = std::ffi::OsStr::new(EXPECTED_MUTEX)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(format!(
            "failed to open benchmark mutex for contention verification: {}",
            std::io::Error::last_os_error()
        ));
    }
    let wait_result = unsafe { WaitForSingleObject(handle, 0) };
    let verified = match wait_result {
        WAIT_TIMEOUT => Ok(true),
        WAIT_OBJECT_0 | WAIT_ABANDONED => {
            unsafe {
                ReleaseMutex(handle);
            }
            Err("benchmark mutex is not held by the claimed wrapper owner".to_string())
        }
        WAIT_FAILED => Err(format!(
            "benchmark mutex contention verification failed: {}",
            std::io::Error::last_os_error()
        )),
        other => Err(format!(
            "unexpected benchmark mutex contention result: {other}"
        )),
    };
    unsafe {
        CloseHandle(handle);
    }
    verified
}

#[cfg(not(windows))]
fn verify_inherited_benchmark_mutex_contention() -> Result<bool, String> {
    Err("CPU reliability soak release authority is supported only on Windows".to_string())
}

fn credential_environment_is_clear() -> bool {
    const NAMES: &[&str] = &[
        "ALPACA_API_KEY",
        "ALPACA_API_KEY_ID",
        "ALPACA_API_SECRET",
        "ALPACA_API_SECRET_KEY",
        "ALPACA_KEY",
        "ALPACA_KEY_ID",
        "ALPACA_SECRET",
        "ALPACA_SECRET_KEY",
        "ALPACA_API_BASE_URL",
        "APCA_API_KEY",
        "APCA_API_KEY_ID",
        "APCA_API_SECRET",
        "APCA_API_SECRET_KEY",
        "APCA_API_BASE_URL",
        "POLYGON_KEY",
        "POLYGON_API_KEY",
        "POLYGON_TOKEN",
        "POLYGON_API_BASE_URL",
    ];
    NAMES.iter().all(|name| std::env::var_os(name).is_none())
}

fn compiler_override_environment_is_clear() -> bool {
    const NAMES: &[&str] = &[
        "RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "RUSTC",
        "CARGO_BUILD_RUSTC",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTUP_TOOLCHAIN",
        "RUSTC_BOOTSTRAP",
        "RUSTC_STAGE",
    ];
    NAMES.iter().all(|name| {
        std::env::var(name)
            .ok()
            .is_none_or(|value| value.trim().is_empty())
    }) && std::env::vars().all(|(name, value)| {
        let name = name.to_ascii_uppercase();
        !(name.starts_with("CARGO_TARGET_")
            && (name.ends_with("_RUSTFLAGS") || name.ends_with("_LINKER")))
            || value.trim().is_empty()
    })
}

fn native_build_override_environment_is_clear() -> bool {
    const FIXED_NAMES: &[&str] = &[
        "CC",
        "CXX",
        "CPP",
        "AR",
        "RANLIB",
        "LD",
        "LINKER",
        "ASM",
        "NASM",
        "WINDRES",
        "RC",
        "COMSPEC",
        "EMBED_RESOURCE_RC_PATH",
        "EMBED_RESOURCE_RC_ATTESTATION_NONCE",
        "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION",
        "CFLAGS",
        "CXXFLAGS",
        "CPPFLAGS",
        "LDFLAGS",
        "ARFLAGS",
        "ASMFLAGS",
        "CL",
        "_CL_",
        "LINK",
        "_LINK_",
        "CRATE_CC_NO_DEFAULTS",
        "CC_ENABLE_DEBUG_OUTPUT",
        "LIBSQLITE3_SYS_USE_PKG_CONFIG",
        "LIBSQLITE3_FLAGS",
        "SQLITE_MAX_VARIABLE_NUMBER",
        "SQLITE_MAX_EXPR_DEPTH",
        "SQLITE_MAX_COLUMN",
        "ZSTD_SYS_USE_PKG_CONFIG",
        "RING_PREGENERATE_ASM",
        "PERL_EXECUTABLE",
        "ICU4X_DATA_DIR",
        "REMOVE_UNUSED_COMMANDS",
        "__TAURI_WORKSPACE__",
        "RUSTC_STAGE",
        "LZMA_API_STATIC",
        "STATIC_VCRUNTIME",
        "CXXSTDLIB",
        "RANLIBFLAGS",
        "PKG_CONFIG",
        "NO_VCPKG",
        "VSTEL_MSBUILDPROJECTFULLPATH",
        "VSCMD_ARG_HOST_ARCH",
        "VSCMD_ARG_TGT_ARCH",
        "VSCMD_ARG_APP_PLAT",
        "VSCMD_ARG_VCVARS_VER",
        "VSCMD_ARG_WINSDK",
        "VSCMD_ARG_VCVARS_SPECTRE",
        "VSCMD_VER",
        "VSCMD_DEBUG",
        "VSCMD_SKIP_SENDTELEMETRY",
        "VSCMD_START_DIR",
        "COMMANDPROMPTTYPE",
        "PLATFORM",
        "PREFERREDTOOLARCHITECTURE",
        "__VSCMD_PREINIT_PATH",
        "WINDOWSLIBPATH",
        "NETFXSDKDIR",
        "INCLUDE",
        "EXTERNAL_INCLUDE",
        "IFCPATH",
        "LIB",
        "LIBPATH",
        "VSINSTALLDIR",
        "VCINSTALLDIR",
        "VCTOOLSINSTALLDIR",
        "VCTOOLSVERSION",
        "VCTOOLSREDISTDIR",
        "VCIDEINSTALLDIR",
        "VS170COMNTOOLS",
        "VISUALSTUDIOVERSION",
        "WINDOWSSDKDIR",
        "WINDOWSSDKVERSION",
        "WINDOWSSDK_EXECUTABLEPATH_X64",
        "WINDOWSSDK_EXECUTABLEPATH_X86",
        "WINDOWSSDKBINPATH",
        "WINDOWSSDKLIBVERSION",
        "WINDOWSSDKVERBINPATH",
        "UNIVERSALCRTSDKDIR",
        "UCRTVERSION",
        "DEVENVDIR",
        "EXTENSIONSDKDIR",
        "FRAMEWORKDIR",
        "FRAMEWORKDIR64",
        "FRAMEWORKVERSION",
        "FRAMEWORKVERSION64",
        "FRAMEWORK40VERSION",
        "__DOTNET_ADD_64BIT",
        "__DOTNET_PREFERRED_BITNESS",
    ];
    const TOOL_TOKENS: &[&str] = &[
        "CC",
        "CXX",
        "CPP",
        "AR",
        "RANLIB",
        "LD",
        "LINKER",
        "ASM",
        "NASM",
        "WINDRES",
        "RC",
        "CFLAGS",
        "CXXFLAGS",
        "CPPFLAGS",
        "LDFLAGS",
        "ARFLAGS",
        "ASMFLAGS",
        "CXXSTDLIB",
        "RANLIBFLAGS",
    ];
    std::env::vars().all(|(name, value)| {
        if value.trim().is_empty() {
            return true;
        }
        let upper = name.to_ascii_uppercase();
        if FIXED_NAMES.contains(&upper.as_str()) {
            return false;
        }
        let dependency_override = upper.starts_with("VSCMD_ARG_")
            || upper.starts_with("__VSCMD")
            || upper.starts_with("_VSCMD")
            || upper.starts_with("VSCMD")
            || upper.starts_with("VC_")
            || upper.starts_with("_VC_")
            || upper.starts_with("VCLIB_")
            || upper.starts_with("__DOTNET_")
            || upper.starts_with("__VCVARS")
            || upper.starts_with("VCVARS_")
            || upper.starts_with("VCINSTALL")
            || upper.starts_with("VCIDE")
            || upper.starts_with("VCTOOLS")
            || upper.starts_with("VISUALSTUDIO")
            || (upper.starts_with("VS") && upper.ends_with("COMNTOOLS"))
            || upper.starts_with("WINDOWSSDK")
            || upper.starts_with("WINDOWS_SDK")
            || upper.starts_with("UNIVERSALCRT")
            || upper.starts_with("UCRT")
            || upper.starts_with("FRAMEWORKDIR")
            || upper.starts_with("FRAMEWORKVERSION")
            || upper.starts_with("NETFXSDK")
            || upper.starts_with("EXTENSIONSDK")
            || upper.starts_with("DEVENV")
            || upper.ends_with("_NO_VCPKG")
            || [
                "PKG_CONFIG_",
                "VCPKG_",
                "VCPKGRS_",
                "OPENSSL_",
                "SQLITE3_",
                "LIBSQLITE3_",
                "ZSTD_",
                "RING_",
                "PERL_",
            ]
            .iter()
            .any(|prefix| upper.starts_with(prefix));
        !dependency_override
            && !TOOL_TOKENS.iter().any(|token| {
                upper.starts_with(&format!("{token}_")) || upper.ends_with(&format!("_{token}"))
            })
    })
}

fn build_environment_is_attested(config: &ValidatedConfig) -> Result<bool, String> {
    let cargo_target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .ok_or_else(|| "CARGO_TARGET_DIR is missing".to_string())?;
    let cargo_target_dir = absolute_lexical_path(Path::new(&cargo_target_dir))?;
    let temp = std::env::var_os("TEMP").ok_or_else(|| "TEMP is missing".to_string())?;
    let temp = absolute_lexical_path(Path::new(&temp))?;
    let tmp = std::env::var_os("TMP").ok_or_else(|| "TMP is missing".to_string())?;
    let tmp = absolute_lexical_path(Path::new(&tmp))?;
    Ok(cargo_target_dir == config.cargo_target_dir
        && temp == tmp
        && temp != config.cargo_target_dir
        && temp.starts_with(&config.cargo_target_dir)
        && std::env::var("CARGO_INCREMENTAL").ok().as_deref() == Some("0")
        && std::env::var("CARGO_BUILD_INCREMENTAL").ok().as_deref() == Some("false")
        && ["CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"]
            .iter()
            .all(|name| {
                std::env::var(name)
                    .ok()
                    .is_none_or(|value| value.trim().is_empty())
            })
        && [
            "TAURI_CONFIG",
            "TAURI_ENV_TARGET_TRIPLE",
            "CARGO_BUILD_TARGET",
        ]
        .iter()
        .all(|name| std::env::var_os(name).is_none())
        && std::env::vars().all(|(name, value)| {
            !name.to_ascii_uppercase().starts_with("CARGO_PROFILE_") || value.trim().is_empty()
        })
        && std::env::vars().all(|(name, value)| {
            let upper = name.to_ascii_uppercase();
            !upper.starts_with("CARGO_")
                || value.trim().is_empty()
                || matches!(
                    upper.as_str(),
                    "CARGO_HOME"
                        | "CARGO_TARGET_DIR"
                        | "CARGO_INCREMENTAL"
                        | "CARGO_BUILD_INCREMENTAL"
                        | "CARGO_NET_OFFLINE"
                        | "CARGO_TERM_COLOR"
                )
        })
        && std::env::vars().all(|(name, value)| {
            !name.to_ascii_uppercase().starts_with("TAURI_") || value.trim().is_empty()
        }))
}

fn external_http_proxy_is_trapped() -> bool {
    ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"]
        .iter()
        .all(|name| std::env::var(name).ok().as_deref() == Some("http://127.0.0.1:9"))
        && std::env::var("NO_PROXY").ok().as_deref() == Some("localhost,127.0.0.1,::1")
        && std::env::var("CARGO_NET_OFFLINE").ok().as_deref() == Some("true")
}

fn remove_owned_run_artifact(path: &Path, app_data_dir: &Path) -> Result<(), String> {
    let app_data_dir = fs::canonicalize(app_data_dir).map_err(|e| e.to_string())?;
    let artifact_parent = path
        .parent()
        .ok_or_else(|| "saved artifact has no parent".to_string())?;
    let artifact_parent = fs::canonicalize(artifact_parent).map_err(|e| e.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !artifact_parent.starts_with(&app_data_dir)
        || !file_name.starts_with("vectorgrid_run_")
        || path.extension().and_then(|value| value.to_str()) != Some("json")
    {
        return Err(format!(
            "refusing to remove non-soak run artifact: {}",
            path.display()
        ));
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}

fn load_source_label(source: &LoadSource) -> &'static str {
    match source {
        LoadSource::BinaryCache => "binary_cache",
        LoadSource::CsvParsedAndCached => "csv_parsed_and_cached",
        LoadSource::CsvParsedCacheWriteSkipped => "csv_parsed_cache_write_skipped",
        LoadSource::InMemoryBaseReuse => "in_memory_base_reuse",
        LoadSource::DerivedFreshResample => "derived_fresh_resample",
        LoadSource::DerivedInMemoryReuse => "derived_in_memory_reuse",
    }
}

fn run_cycle(
    cycle: usize,
    config: &ValidatedConfig,
    csv_path: &Path,
    cache_dir: &Path,
    state: &AppState,
    expected_data_id: &str,
    app_handle: tauri::AppHandle,
    app_data_dir: &Path,
) -> Result<CycleSummary, String> {
    let started = Instant::now();
    let catalog_before = rebuild_artifact_catalog_blocking(app_handle.clone())?
        .runs
        .len();
    let loaded = state.load_price_data(&csv_path.to_string_lossy(), cache_dir)?;
    if loaded.meta.len != config.rows {
        return Err(format!(
            "loaded row count changed: expected {}, got {}",
            config.rows, loaded.meta.len
        ));
    }
    let load_source = load_source_label(&loaded.load_source);
    if loaded.id != expected_data_id
        || !matches!(&loaded.load_source, LoadSource::InMemoryBaseReuse)
    {
        return Err(format!(
            "persistent managed AppState reuse changed: expected id={expected_data_id} and in_memory_base_reuse, got id={} and {load_source}",
            loaded.id
        ));
    }
    let candles = state.get_candles_for_run(&loaded.id)?;
    if candles.close.len() != config.rows {
        return Err(format!(
            "parsed candle count changed: expected {}, got {}",
            config.rows,
            candles.close.len()
        ));
    }
    drop(candles);
    let data_id = loaded.id.clone();
    let opt_request = optimization_request(&data_id);
    let mut cancellation_probes = 0usize;
    let mut cancellation_partial_processed = 0usize;
    let mut cancellation_partial_total = 0usize;

    if cycle % config.cancel_every == 0 {
        let run_guard = state.try_begin_run()?;
        let cancel = state.cancel_token();
        let candles = state.get_candles_for_run(&loaded.id)?;
        let sink = CancelOnPartialProgress::new(cancel.as_ref());
        let cancelled = ta_app_core::run_double_ma_optimization_blocking_with_candles(
            cancellation_optimization_request(&data_id),
            candles.as_ref(),
            cancel.as_ref(),
            Some(&sink),
        );
        drop(run_guard);
        if sink.event_count() == 0 {
            return Err("in-flight optimization cancellation emitted no progress".to_string());
        }
        let (processed, total) = sink.partial_progress().ok_or_else(|| {
            "in-flight optimization cancellation emitted no positive partial progress".to_string()
        })?;
        if processed == 0 || processed >= total {
            return Err(format!(
                "in-flight optimization cancellation boundary is invalid: processed={processed}, total={total}"
            ));
        }
        match cancelled {
            Err(error) if error.trim().eq_ignore_ascii_case("cancelled") => {
                cancellation_probes += 1;
                cancellation_partial_processed = processed;
                cancellation_partial_total = total;
            }
            Err(error) => {
                return Err(format!(
                    "in-flight optimization cancellation returned unexpected error: {error}"
                ));
            }
            Ok(_) => {
                return Err("in-flight optimization cancellation completed as success".to_string());
            }
        }
    }

    let optimization = {
        let _run_guard = state.try_begin_run()?;
        let cancel = state.cancel_token();
        let candles = state.get_candles_for_run(&loaded.id)?;
        ta_app_core::run_double_ma_optimization_blocking_with_candles(
            opt_request,
            candles.as_ref(),
            cancel.as_ref(),
            None,
        )?
    };
    if optimization.num_combos != EXPECTED_OPTIMIZATION_COMBINATIONS {
        return Err(format!(
            "optimization combination count changed: expected {}, got {}",
            EXPECTED_OPTIMIZATION_COMBINATIONS, optimization.num_combos
        ));
    }
    let candidates = validation_candidates(&optimization.top, config.validation_candidates);
    if candidates.len() != config.validation_candidates {
        return Err(format!(
            "optimization shortlist changed: expected {} unique validation candidates, got {}",
            config.validation_candidates,
            candidates.len()
        ));
    }
    let validation_request = validation_request(
        &data_id,
        candidates,
        optimization.num_combos,
        csv_path,
        cycle as u64,
    );

    if cycle % config.cancel_every == 0 {
        let cancelled = {
            let _run_guard = state.try_begin_run()?;
            state.request_cancel();
            let cancel = state.cancel_token();
            let candles = state.get_candles_for_run(&loaded.id)?;
            ta_app_core::run_double_ma_validation_blocking_with_candles(
                validation_request.clone(),
                candles.as_ref(),
                cancel.as_ref(),
                None,
            )
        };
        match cancelled {
            Err(error) if error.trim().eq_ignore_ascii_case("cancelled") => {
                cancellation_probes += 1;
            }
            Err(error) => {
                return Err(format!(
                    "pre-start validation cancellation returned unexpected error: {error}"
                ));
            }
            Ok(_) => {
                return Err("pre-start validation cancellation completed as success".to_string());
            }
        }
    }

    let validation = {
        let _run_guard = state.try_begin_run()?;
        let cancel = state.cancel_token();
        let candles = state.get_candles_for_run(&loaded.id)?;
        ta_app_core::run_double_ma_validation_blocking_with_candles(
            validation_request,
            candles.as_ref(),
            cancel.as_ref(),
            None,
        )?
    };
    if validation.candidates.len() != config.validation_candidates {
        return Err(format!(
            "validation candidate count changed: expected {}, got {}",
            config.validation_candidates,
            validation.candidates.len()
        ));
    }
    let timestamp = (chrono::Utc::now() + chrono::Duration::seconds(cycle as i64))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let artifact = json!({
        "kind": "vectorgrid_run_artifact",
        "ts": timestamp,
        "data": {
            "csv_path": csv_path.to_string_lossy(),
            "meta": {
                "len": config.rows,
                "timeframe": "1m",
            }
        },
        "dataset_meta": {
            "csv_path": csv_path.to_string_lossy(),
            "symbol": "SOAK",
            "timeframe": "1m",
            "len": config.rows,
        },
        "run": {
            "req": {
                "strategy_kind": "DoubleMa",
                "backend": {"backend": "CpuOnly"},
                "data_id": data_id,
            },
            "res": serde_json::to_value(&optimization).map_err(|e| e.to_string())?,
        },
        "optimization": {
            "req": {
                "strategy_kind": "DoubleMa",
                "backend": {"backend": "CpuOnly"},
            },
            "res": serde_json::to_value(&optimization).map_err(|e| e.to_string())?,
        },
        "validation": {
            "req": {
                "strategy_kind": "DoubleMa",
                "backend": {"backend": "CpuOnly"},
                "validation_context": {
                    "strategy_kind": "DoubleMa",
                    "csv_path": csv_path.to_string_lossy(),
                    "timeframe": "1m",
                    "optimization_trials": optimization.num_combos,
                }
            },
            "res": serde_json::to_value(&validation).map_err(|e| e.to_string())?,
        },
        "tags": ["cpu-reliability-soak"],
    });

    let mut saved_path: Option<PathBuf> = None;
    let artifact_result = (|| -> Result<(String, usize, usize), String> {
        let saved = save_run_artifact_blocking(artifact, app_handle.clone(), None)?;
        let path = PathBuf::from(&saved.path);
        saved_path = Some(path.clone());
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() != saved.bytes {
            return Err(format!(
                "saved artifact byte count changed: response={}, file={}",
                saved.bytes,
                bytes.len()
            ));
        }
        let parsed: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if parsed.get("kind").and_then(Value::as_str) != Some("vectorgrid_run_artifact") {
            return Err("saved artifact kind changed".to_string());
        }
        let hash = sha256_file(&path)?;
        let catalog_after_save = rebuild_artifact_catalog_blocking(app_handle.clone())?
            .runs
            .len();
        if catalog_after_save != catalog_before.saturating_add(1) {
            return Err(format!(
                "catalog did not grow by exactly one during cycle: before={catalog_before}, after_save={catalog_after_save}"
            ));
        }
        let hydration = tauri::async_runtime::block_on(load_run_artifact_hydration(
            saved.path,
            app_handle.clone(),
        ))?;
        if hydration.current_validation.is_none() {
            return Err("artifact reload omitted validation payload".to_string());
        }
        if PathBuf::from(&hydration.current_run.artifact_path) != path {
            return Err("artifact reload returned a different artifact path".to_string());
        }
        Ok((hash, saved.bytes, catalog_after_save))
    })();

    let cleanup_result = if let Some(path) = saved_path.as_deref() {
        remove_owned_run_artifact(path, app_data_dir).and_then(|_| {
            rebuild_artifact_catalog_blocking(app_handle.clone())
                .map(|snapshot| snapshot.runs.len())
        })
    } else {
        rebuild_artifact_catalog_blocking(app_handle.clone()).map(|snapshot| snapshot.runs.len())
    };

    let (artifact_sha256, artifact_bytes, catalog_after_save) = artifact_result?;
    let catalog_after_cleanup = cleanup_result?;
    if catalog_after_cleanup != catalog_before {
        return Err(format!(
            "catalog growth remained after cleanup: before={catalog_before}, after_cleanup={catalog_after_cleanup}"
        ));
    }

    Ok(CycleSummary {
        cycle,
        status: "passed".to_string(),
        wall_ms: started.elapsed().as_millis() as u64,
        loaded_data_id: loaded.id,
        load_source: load_source.to_string(),
        loaded_rows: loaded.meta.len,
        optimization_combinations: optimization.num_combos,
        validation_candidate_count: validation.candidates.len(),
        cancellation_probes,
        cancellation_partial_processed,
        cancellation_partial_total,
        artifact_sha256,
        artifact_bytes,
        catalog_before,
        catalog_after_save,
        catalog_after_cleanup,
        error: None,
    })
}

#[cfg(windows)]
fn sample_process(cycle: usize, stage: &str, elapsed_ms: u64) -> Result<ProcessSample, String> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

    let process = unsafe { GetCurrentProcess() };
    let mut counters: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let memory_ok = unsafe {
        GetProcessMemoryInfo(
            process,
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            counters.cb,
        )
    };
    if memory_ok == 0 {
        return Err(format!(
            "GetProcessMemoryInfo failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut handles = 0u32;
    let handles_ok = unsafe { GetProcessHandleCount(process, &mut handles) };
    if handles_ok == 0 {
        return Err(format!(
            "GetProcessHandleCount failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(ProcessSample {
        cycle,
        stage: stage.to_string(),
        elapsed_ms,
        working_set_bytes: Some(counters.WorkingSetSize as u64),
        private_bytes: Some(counters.PrivateUsage as u64),
        handle_count: Some(handles),
    })
}

#[cfg(not(windows))]
fn sample_process(cycle: usize, stage: &str, elapsed_ms: u64) -> Result<ProcessSample, String> {
    Ok(ProcessSample {
        cycle,
        stage: stage.to_string(),
        elapsed_ms,
        working_set_bytes: None,
        private_bytes: None,
        handle_count: None,
    })
}

#[cfg(test)]
fn linear_regression_slope(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|point| point.0).sum();
    let sum_y: f64 = points.iter().map(|point| point.1).sum();
    let sum_xx: f64 = points.iter().map(|point| point.0 * point.0).sum();
    let sum_xy: f64 = points.iter().map(|point| point.0 * point.1).sum();
    let denominator = n * sum_xx - sum_x * sum_x;
    if denominator.abs() <= f64::EPSILON {
        return None;
    }
    Some((n * sum_xy - sum_x * sum_y) / denominator)
}

#[cfg(test)]
fn assess_metric(
    samples: &[(f64, f64)],
    max_growth: f64,
    max_slope_per_cycle: f64,
) -> LeakMetricAssessment {
    let mut trend = TrendAccumulator::default();
    for (cycle, value) in samples {
        trend.push(*cycle as usize, Some(*value));
    }
    assess_trend(&trend, max_growth, max_slope_per_cycle)
}

fn assess_trend(
    trend: &TrendAccumulator,
    max_growth: f64,
    max_slope_per_cycle: f64,
) -> LeakMetricAssessment {
    if trend.expected_samples == 0 {
        return LeakMetricAssessment {
            supported: false,
            complete: false,
            expected_samples: 0,
            measured_samples: 0,
            first: None,
            last: None,
            growth: None,
            slope_per_cycle: None,
            max_growth,
            max_slope_per_cycle,
            passed: false,
            note: "no post-warmup process samples were scheduled".to_string(),
        };
    }
    let supported = trend.measured_samples > 0;
    let complete = trend.measured_samples == trend.expected_samples;
    let first = trend.first;
    let last = trend.last;
    let growth = first.zip(last).map(|(first, last)| last - first);
    let slope = trend.slope();
    let enough_samples = trend.measured_samples >= 3;
    let growth_passed = growth.is_some_and(|value| value <= max_growth);
    let slope_passed = slope.is_some_and(|value| value <= max_slope_per_cycle);
    let passed = supported && complete && enough_samples && growth_passed && slope_passed;
    LeakMetricAssessment {
        supported,
        complete,
        expected_samples: trend.expected_samples,
        measured_samples: trend.measured_samples,
        first,
        last,
        growth,
        slope_per_cycle: slope,
        max_growth,
        max_slope_per_cycle,
        passed,
        note: if !supported {
            "process metric is unavailable on this platform".to_string()
        } else if !complete {
            "one or more post-warmup process samples were unavailable".to_string()
        } else if !enough_samples {
            "at least three post-warmup samples are required".to_string()
        } else if passed {
            "full-run growth and online least-squares per-cycle slope are within thresholds"
                .to_string()
        } else {
            "full-run growth or online least-squares per-cycle slope exceeded its threshold"
                .to_string()
        },
    }
}

fn monitoring_assessment(
    sample_window: &SampleWindow,
    tracker: &MonitoringTracker,
    storage_tracker: &OwnedStorageTracker,
    warmup_cycles: usize,
    thresholds: LeakThresholds,
) -> MonitoringAssessment {
    let retained = sample_window.retained();
    let storage_file_count = assess_trend(
        &storage_tracker.file_count,
        MAX_OWNED_STORAGE_FILE_GROWTH,
        MAX_OWNED_STORAGE_FILE_SLOPE_PER_CYCLE,
    );
    let storage_directory_count = assess_trend(
        &storage_tracker.directory_count,
        MAX_OWNED_STORAGE_DIRECTORY_GROWTH,
        MAX_OWNED_STORAGE_DIRECTORY_SLOPE_PER_CYCLE,
    );
    let storage_total_bytes = assess_trend(
        &storage_tracker.total_bytes,
        MAX_OWNED_STORAGE_GROWTH_BYTES,
        MAX_OWNED_STORAGE_SLOPE_BYTES_PER_CYCLE,
    );
    let storage_passed = storage_file_count.passed
        && storage_directory_count.passed
        && storage_total_bytes.passed
        && storage_tracker.max_residual_artifact_or_temp_files == 0;
    MonitoringAssessment {
        sample_count_total: sample_window.total,
        retained_sample_count: retained.len(),
        cycle_complete_sample_count: tracker.cycle_complete_samples,
        max_cycle_sample_gap_ms: tracker.max_cycle_sample_gap_ms,
        warmup_cycles_excluded: warmup_cycles,
        working_set: assess_trend(
            &tracker.working_set,
            thresholds.max_working_set_growth_bytes,
            thresholds.max_working_set_slope_bytes_per_cycle,
        ),
        private_bytes: assess_trend(
            &tracker.private_bytes,
            thresholds.max_private_growth_bytes,
            thresholds.max_private_slope_bytes_per_cycle,
        ),
        handles: assess_trend(
            &tracker.handles,
            thresholds.max_handle_growth as f64,
            thresholds.max_handle_slope_per_cycle,
        ),
        owned_storage: OwnedStorageAssessment {
            sample_count_total: storage_tracker.sample_count_total,
            max_residual_artifact_or_temp_files: storage_tracker
                .max_residual_artifact_or_temp_files,
            file_count: storage_file_count,
            directory_count: storage_directory_count,
            total_bytes: storage_total_bytes,
            passed: storage_passed,
        },
    }
}

fn bounded_push<T>(values: &mut VecDeque<T>, value: T, capacity: usize) {
    values.push_back(value);
    while values.len() > capacity {
        values.pop_front();
    }
}

fn main() -> Result<(), String> {
    let config = ValidatedConfig::from_cli(Cli::parse())?;
    let benchmark_mutex_contention_verified = verify_inherited_benchmark_mutex_contention()?;
    let _benchmark_guard = benchmark_lock::acquire("CPU reliability soak")?;
    let provenance = load_execution_provenance(&config, benchmark_mutex_contention_verified)?;
    let compiled_resource_compiler_attestation_matched = provenance
        .pointer("/compiled_resource_compiler_attestation/matched_native_invocation")
        .and_then(Value::as_bool)
        .ok_or_else(|| "compiled resource-compiler attestation evidence is missing".to_string())?;
    if compiled_resource_compiler_attestation_matched != config.fresh_build {
        return Err(
            "compiled resource-compiler attestation requirement does not match build mode"
                .to_string(),
        );
    }
    if !credential_environment_is_clear() {
        return Err("provider/broker credential environment must be cleared".to_string());
    }
    if !compiler_override_environment_is_clear() {
        return Err("compiler override environment must be cleared".to_string());
    }
    if !native_build_override_environment_is_clear() {
        return Err("native compiler/linker/SDK override environment must be cleared".to_string());
    }
    if !build_environment_is_attested(&config)? {
        return Err("Cargo target/profile/temp build environment is not attested".to_string());
    }
    if !external_http_proxy_is_trapped() {
        return Err(
            "offline runtime requires the external HTTP proxy trap and CARGO_NET_OFFLINE=true"
                .to_string(),
        );
    }
    if std::env::var("VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE")
        .ok()
        .as_deref()
        != Some("1")
    {
        return Err(
            "offline runtime requires the QA profile override to disable repo .env credential fallback"
                .to_string(),
        );
    }
    if std::env::var("VECTORBT_CPU_RAM_BUDGET_MB").ok().as_deref() != Some("64") {
        return Err(
            "CPU reliability soak requires VECTORBT_CPU_RAM_BUDGET_MB=64 for deterministic cancellation tiling"
                .to_string(),
        );
    }
    let started_at = chrono::Utc::now();
    let started = Instant::now();
    let mut isolation = IsolationGuard::create(&config.isolation_root)?;
    isolation.configure_process_environment()?;

    let fixture_dir = config.isolation_root.join("fixture");
    let cache_dir = config.isolation_root.join("cache");
    fs::create_dir_all(&fixture_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let csv_path = fixture_dir.join("synthetic_ohlcv.csv");
    write_synthetic_ohlcv_csv(&csv_path, config.rows)?;

    let requested_app_data_dir = config
        .isolation_root
        .join("local-app-data")
        .join(format!("vectorgrid-cpusoak-{}", config.run_id));
    isolation.register_app_data_root(&requested_app_data_dir)?;
    let app = build_isolated_app(&requested_app_data_dir)?;
    let app_handle = app.handle().clone();
    let app_state = app.state::<AppState>();
    let app_data_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let canonical_requested_app_data_dir =
        fs::canonicalize(&requested_app_data_dir).map_err(|error| error.to_string())?;
    let canonical_resolved_app_data_dir =
        fs::canonicalize(&app_data_dir).map_err(|error| error.to_string())?;
    if canonical_resolved_app_data_dir != canonical_requested_app_data_dir {
        return Err(format!(
            "Tauri app_local_data_dir escaped the registered isolation root: expected {}, got {}",
            requested_app_data_dir.display(),
            app_data_dir.display()
        ));
    }

    let initial_catalog = rebuild_artifact_catalog_blocking(app_handle.clone())?
        .runs
        .len();
    let mut max_catalog_after_cleanup = initial_catalog;
    let mut max_catalog_transient = initial_catalog;
    let mut samples = SampleWindow::new(config.retained_samples);
    samples.push(sample_process(0, "baseline", 0)?);
    let initial_managed_load =
        app_state.load_price_data(&csv_path.to_string_lossy(), &cache_dir)?;
    if initial_managed_load.meta.len != config.rows
        || !matches!(
            &initial_managed_load.load_source,
            LoadSource::CsvParsedAndCached
        )
    {
        return Err(format!(
            "initial managed AppState load must parse and cache exactly {} rows; got rows={} source={}",
            config.rows,
            initial_managed_load.meta.len,
            load_source_label(&initial_managed_load.load_source)
        ));
    }
    let managed_state_data_id = initial_managed_load.id;
    let initial_managed_candles = app_state.get_candles_for_run(&managed_state_data_id)?;
    if initial_managed_candles.close.len() != config.rows {
        return Err("initial managed AppState candles changed row count".to_string());
    }
    drop(initial_managed_candles);
    let mut monitoring_tracker = MonitoringTracker::default();
    let mut storage_tracker = OwnedStorageTracker::default();
    let mut cycles = VecDeque::with_capacity(config.retained_cycles);
    let mut failure_messages = Vec::new();
    let mut failures_total = 0usize;
    let mut cycles_completed = 0usize;
    let mut artifact_checks = 0usize;
    let mut cancellation_probes = 0usize;
    let mut cancellation_probes_expected = 0usize;
    let mut cancellation_cycles_expected = 0usize;
    let mut partial_progress_cancellation_probes = 0usize;
    let mut pre_start_validation_cancellation_probes = 0usize;
    let managed_state_initial_loads = 1usize;
    let mut managed_state_in_memory_reuses = 0usize;
    let mut cycles_attempted = 0usize;
    let deadline = if config.duration_seconds > 0 {
        Some(Duration::from_secs(config.duration_seconds))
    } else {
        None
    };

    for cycle in 1..=config.cycles {
        if cycle > 1 && deadline.is_some_and(|duration| started.elapsed() >= duration) {
            break;
        }
        cycles_attempted = cycle;
        let cycle_result = run_cycle(
            cycle,
            &config,
            &csv_path,
            &cache_dir,
            app_state.inner(),
            &managed_state_data_id,
            app_handle.clone(),
            &app_data_dir,
        );
        match cycle_result {
            Ok(summary) => {
                managed_state_in_memory_reuses = managed_state_in_memory_reuses.saturating_add(1);
                cycles_completed += 1;
                artifact_checks += 1;
                cancellation_probes += summary.cancellation_probes;
                if cycle % config.cancel_every == 0 {
                    cancellation_probes_expected = cancellation_probes_expected.saturating_add(2);
                    cancellation_cycles_expected = cancellation_cycles_expected.saturating_add(1);
                    if summary.cancellation_partial_processed > 0
                        && summary.cancellation_partial_processed
                            < summary.cancellation_partial_total
                    {
                        partial_progress_cancellation_probes =
                            partial_progress_cancellation_probes.saturating_add(1);
                    }
                    if summary.cancellation_probes >= 2 {
                        pre_start_validation_cancellation_probes =
                            pre_start_validation_cancellation_probes.saturating_add(1);
                    }
                }
                // Scan the complete owned isolation tree so app data, cache, redirected
                // TEMP/APPDATA, fixture storage, and accumulating empty directories all
                // participate in the same constant-space full-run trend assessment.
                let storage = combined_owned_storage_stats(&[config.isolation_root.as_path()])?;
                storage_tracker.push(cycle, storage, config.warmup_cycles);
                max_catalog_transient = max_catalog_transient.max(summary.catalog_after_save);
                max_catalog_after_cleanup =
                    max_catalog_after_cleanup.max(summary.catalog_after_cleanup);
                bounded_push(&mut cycles, summary, config.retained_cycles);
            }
            Err(error) => {
                failures_total += 1;
                if failure_messages.len() < 32 {
                    failure_messages.push(format!("cycle {cycle}: {error}"));
                }
                bounded_push(
                    &mut cycles,
                    CycleSummary {
                        cycle,
                        status: "failed".to_string(),
                        wall_ms: 0,
                        loaded_data_id: String::new(),
                        load_source: String::new(),
                        loaded_rows: 0,
                        optimization_combinations: 0,
                        validation_candidate_count: 0,
                        cancellation_probes: 0,
                        cancellation_partial_processed: 0,
                        cancellation_partial_total: 0,
                        artifact_sha256: String::new(),
                        artifact_bytes: 0,
                        catalog_before: 0,
                        catalog_after_save: 0,
                        catalog_after_cleanup: 0,
                        error: Some(error),
                    },
                    config.retained_cycles,
                );
                if failures_total > config.allowed_failures {
                    break;
                }
            }
        }
        let sample = sample_process(
            cycle,
            "cycle_complete",
            started.elapsed().as_millis() as u64,
        )?;
        monitoring_tracker.push(&sample, config.warmup_cycles);
        samples.push(sample);
    }

    let final_catalog = rebuild_artifact_catalog_blocking(app_handle.clone())?
        .runs
        .len();
    let managed_run_slot_released = match app_state.try_begin_run() {
        Ok(guard) => {
            drop(guard);
            !app_state.cancel_token().load(Ordering::SeqCst)
        }
        Err(error) => {
            failures_total = failures_total.saturating_add(1);
            failure_messages.push(format!(
                "persistent managed AppState run slot remained occupied: {error}"
            ));
            false
        }
    };
    let (managed_state_invalidated_once, managed_state_lookup_rejected_after_invalidation) =
        match app_state.invalidate_dataset(&managed_state_data_id) {
            Ok(removed) if removed == [managed_state_data_id.clone()] => {
                let rejected = app_state
                    .get_candles_for_run(&managed_state_data_id)
                    .is_err();
                (true, rejected)
            }
            Ok(removed) => {
                failures_total = failures_total.saturating_add(1);
                failure_messages.push(format!(
                    "persistent managed AppState invalidation removed unexpected IDs: {removed:?}"
                ));
                (false, false)
            }
            Err(error) => {
                failures_total = failures_total.saturating_add(1);
                failure_messages.push(format!(
                    "persistent managed AppState final invalidation failed: {error}"
                ));
                (false, false)
            }
        };
    drop(app_handle);
    drop(app);
    let cleanup_result = isolation.cleanup();
    let cleanup_passed = cleanup_result.is_ok();
    if let Err(error) = cleanup_result {
        failures_total += 1;
        failure_messages.push(format!("isolation cleanup: {error}"));
    }

    let final_elapsed = started.elapsed();
    monitoring_tracker.finish(final_elapsed.as_millis() as u64);
    let monitoring = monitoring_assessment(
        &samples,
        &monitoring_tracker,
        &storage_tracker,
        config.warmup_cycles,
        config.thresholds,
    );
    let catalog_growth = final_catalog.saturating_sub(initial_catalog);
    let catalog_passed = catalog_growth <= config.thresholds.max_catalog_growth
        && max_catalog_after_cleanup.saturating_sub(initial_catalog)
            <= config.thresholds.max_catalog_growth;
    let cancellation_passed = cancellation_probes_expected > 0
        && cancellation_probes > 0
        && cancellation_probes >= cancellation_probes_expected
        && cancellation_cycles_expected > 0
        && partial_progress_cancellation_probes >= cancellation_cycles_expected
        && pre_start_validation_cancellation_probes >= cancellation_cycles_expected;
    let managed_state_passed = managed_state_initial_loads == 1
        && managed_state_in_memory_reuses == cycles_completed
        && managed_state_initial_loads.saturating_add(managed_state_in_memory_reuses)
            == cycles_completed.saturating_add(1)
        && managed_run_slot_released
        && managed_state_invalidated_once
        && managed_state_lookup_rejected_after_invalidation
        && failures_total == 0;
    let monitoring_passed = monitoring.working_set.passed
        && monitoring.private_bytes.passed
        && monitoring.handles.passed
        && monitoring.owned_storage.passed;
    let duration_reached = config.duration_seconds == 0
        || final_elapsed >= Duration::from_secs(config.duration_seconds);
    let duration_contract_passed = if config.mode_label == "operator" {
        config.duration_seconds >= MIN_OPERATOR_DURATION_SECONDS && duration_reached
    } else {
        config.duration_seconds == 0
    };
    let operator_sampling_contract_passed = config.mode_label != "operator"
        || (monitoring.working_set.measured_samples >= MIN_OPERATOR_MEASURED_CYCLES
            && monitoring.private_bytes.measured_samples >= MIN_OPERATOR_MEASURED_CYCLES
            && monitoring.handles.measured_samples >= MIN_OPERATOR_MEASURED_CYCLES
            && monitoring.owned_storage.file_count.measured_samples
                >= MIN_OPERATOR_MEASURED_CYCLES
            && monitoring.owned_storage.directory_count.measured_samples
                >= MIN_OPERATOR_MEASURED_CYCLES
            && monitoring.owned_storage.total_bytes.measured_samples
                >= MIN_OPERATOR_MEASURED_CYCLES
            && monitoring.max_cycle_sample_gap_ms <= MAX_OPERATOR_SAMPLE_GAP_MS);
    let status = if failures_total <= config.allowed_failures
        && cycles_completed > config.warmup_cycles.saturating_add(2)
        && artifact_checks == cycles_completed
        && catalog_passed
        && cancellation_passed
        && managed_state_passed
        && monitoring_passed
        && cleanup_passed
        && duration_contract_passed
        && operator_sampling_contract_passed
    {
        "passed"
    } else {
        "failed"
    };

    let retained_samples = samples.retained();
    let completed_at = chrono::Utc::now();
    let elapsed_ms = final_elapsed.as_millis() as u64;
    let operator_release_eligible = status == "passed"
        && config.mode_label == "operator"
        && failures_total == 0
        && config.allowed_failures == 0
        && config.build_profile == "release"
        && config.fresh_build
        && !config.cargo_artifact_fresh
        && config.duration_seconds >= MIN_OPERATOR_DURATION_SECONDS
        && duration_reached
        && managed_state_passed
        && operator_sampling_contract_passed
        && elapsed_ms >= MIN_OPERATOR_DURATION_SECONDS.saturating_mul(1_000);
    let termination_reason = if failures_total > config.allowed_failures {
        "failure_limit_exceeded"
    } else if duration_reached && config.duration_seconds > 0 {
        "duration_reached"
    } else if cycles_attempted >= config.cycles {
        "cycle_limit_reached"
    } else {
        "ended_before_authoritative_boundary"
    };
    let summary = SoakSummary {
        schema_version: 2,
        kind: "vectorgrid_cpu_reliability_soak_evidence".to_string(),
        run_id: config.run_id.clone(),
        status: status.to_string(),
        mode_label: config.mode_label.clone(),
        started_at: started_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        completed_at: completed_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        elapsed_ms,
        timing_boundary: "single_process_native_cpu_load_optimize_validate_artifact_reload_cancel"
            .to_string(),
        provenance,
        termination: json!({
            "reason": termination_reason,
            "cycles_attempted": cycles_attempted,
            "cycle_limit_reached": cycles_attempted >= config.cycles,
            "duration_required_seconds": config.duration_seconds,
            "duration_reached": duration_reached,
            "duration_contract_passed": duration_contract_passed,
            "operator_sampling_contract_passed": operator_sampling_contract_passed,
        }),
        authority: json!({
            "operator_release_eligible": operator_release_eligible,
            "minimum_operator_duration_seconds": MIN_OPERATOR_DURATION_SECONDS,
            "minimum_operator_measured_cycles": MIN_OPERATOR_MEASURED_CYCLES,
            "maximum_operator_sample_gap_ms": MAX_OPERATOR_SAMPLE_GAP_MS,
            "requires_fresh_release_build": true,
            "requires_full_run_trend_samples": true,
            "requires_bounded_cycle_sample_gap": true,
            "owned_storage_thresholds": {
                "max_growth_bytes": MAX_OWNED_STORAGE_GROWTH_BYTES,
                "max_slope_bytes_per_cycle": MAX_OWNED_STORAGE_SLOPE_BYTES_PER_CYCLE,
                "max_file_growth": MAX_OWNED_STORAGE_FILE_GROWTH,
                "max_file_slope_per_cycle": MAX_OWNED_STORAGE_FILE_SLOPE_PER_CYCLE,
                "max_directory_growth": MAX_OWNED_STORAGE_DIRECTORY_GROWTH,
                "max_directory_slope_per_cycle": MAX_OWNED_STORAGE_DIRECTORY_SLOPE_PER_CYCLE,
                "max_residual_artifact_or_temp_files": 0,
            },
            "short_mode_is_release_authority": false,
        }),
        offline_contract: json!({
            "provider_credentials_used": false,
            "provider_network_calls": false,
            "real_broker_orders": false,
            "computer_use": false,
            "synthetic_data_only": true,
            "credential_environment_cleared": true,
            "repo_dotenv_credential_fallback_disabled": true,
            "external_http_proxy_trapped": true,
            "cargo_dependency_resolution_offline": true,
            "compiler_override_environment_cleared": true,
            "native_build_override_environment_cleared": true,
            "native_build_environment_attested": config.fresh_build,
            "target_linker_path_pinned_during_build": config.fresh_build,
            "compiled_resource_compiler_attestation_matched":
                compiled_resource_compiler_attestation_matched,
            "build_environment_attested": true,
            "tauri_config_environment_cleared": true,
            "tauri_target_environment_cleared": true,
            "compiled_tauri_config_override_absent": COMPILED_TAURI_CONFIG.is_none(),
            "compiled_tauri_target_triple": COMPILED_TAURI_TARGET_TRIPLE,
        }),
        config: json!({
            "cycle_limit": config.cycles,
            "duration_seconds": config.duration_seconds,
            "cancel_every": config.cancel_every,
            "rows": config.rows,
            "cargo_target_triple": config.cargo_target_triple,
            "validation_candidates": config.validation_candidates,
            "cpu_tile_ram_budget_mb": 64,
            "warmup_cycles": config.warmup_cycles,
            "allowed_failures": config.allowed_failures,
            "retained_samples_limit": config.retained_samples,
            "retained_cycles_limit": config.retained_cycles,
            "thresholds": {
                "private_slope_bytes_per_cycle": config.thresholds.max_private_slope_bytes_per_cycle,
                "private_growth_bytes": config.thresholds.max_private_growth_bytes,
                "working_set_slope_bytes_per_cycle": config.thresholds.max_working_set_slope_bytes_per_cycle,
                "working_set_growth_bytes": config.thresholds.max_working_set_growth_bytes,
                "handle_slope_per_cycle": config.thresholds.max_handle_slope_per_cycle,
                "handle_growth": config.thresholds.max_handle_growth,
                "catalog_growth": config.thresholds.max_catalog_growth,
            }
        }),
        counts: json!({
            "cycles_completed": cycles_completed,
            "failures_total": failures_total,
            "retained_failures": failure_messages.len(),
        }),
        catalog: json!({
            "initial_count": initial_catalog,
            "max_transient_count": max_catalog_transient,
            "max_after_cleanup_count": max_catalog_after_cleanup,
            "final_count": final_catalog,
            "growth": catalog_growth,
            "max_allowed_growth": config.thresholds.max_catalog_growth,
            "passed": catalog_passed,
        }),
        artifact_integrity: json!({
            "artifacts_checked": artifact_checks,
            "expected": cycles_completed,
            "json_parse_hash_hydration_checked": true,
            "passed": artifact_checks == cycles_completed,
        }),
        cancellation: json!({
            "mode": "positive_partial_progress_optimization_cancel_and_pre_start_validation_cancel",
            "probes_observed": cancellation_probes,
            "probes_expected": cancellation_probes_expected,
            "cancellation_cycles_expected": cancellation_cycles_expected,
            "positive_partial_progress_probes_observed": partial_progress_cancellation_probes,
            "pre_start_validation_probes_observed": pre_start_validation_cancellation_probes,
            "passed": cancellation_passed,
        }),
        managed_state: json!({
            "lifecycle": "single_tauri_managed_app_state_full_run",
            "data_id": managed_state_data_id,
            "initial_load_source": "csv_parsed_and_cached",
            "initial_loads": managed_state_initial_loads,
            "in_memory_reuses": managed_state_in_memory_reuses,
            "reuse_count_semantics": "successful_cycle_reuses",
            "cycles_observed": managed_state_in_memory_reuses,
            "successful_cycle_load_operations": managed_state_initial_loads.saturating_add(managed_state_in_memory_reuses),
            "requires_zero_failed_cycles": true,
            "run_slot_released_after_final_cycle": managed_run_slot_released,
            "retained_through_last_process_sample": true,
            "product_run_guard_and_cancel_token_used": true,
            "invalidated_once_after_final_sample": managed_state_invalidated_once,
            "lookup_rejected_after_invalidation": managed_state_lookup_rejected_after_invalidation,
            "passed": managed_state_passed,
        }),
        monitoring,
        retained_samples,
        retained_cycles: cycles.into_iter().collect(),
        failures: failure_messages,
        isolation: json!({
            "source_app_data_touched": false,
            "requested_app_data_root": requested_app_data_dir,
            "resolved_app_data_root": app_data_dir,
            "resolved_app_data_strictly_contained": true,
            "root_cleaned": cleanup_passed,
            "app_data_root_cleaned": cleanup_passed,
            "cleanup_requires_owned_marker": true,
        }),
    };
    let summary_json = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    atomic_output::write_str(&config.out, &summary_json)?;
    println!(
        "CPU reliability soak evidence written to {}",
        config.out.display()
    );
    if status != "passed" {
        return Err(format!(
            "CPU reliability soak failed; inspect {}",
            config.out.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        assess_metric, cargo_target_linker_environment_name, cleanup_owned_isolation_root,
        collect_ordinary_tree_files, enumerate_resource_compiler_candidates,
        expected_path_package_id, linear_regression_slope, monitoring_assessment,
        native_input_absolute_path, parse_windows_sdk_version, paths_equal,
        resolve_recorded_path_executable, validate_compiled_resource_compiler_attestation,
        validate_embed_resource_cargo_resolution, validate_source_manifest, Cli, IsolationGuard,
        LeakThresholds, MonitoringTracker, OwnedStorageStats, OwnedStorageTracker, ProcessSample,
        SampleWindow, ValidatedConfig, ISOLATION_PREFIX, MIN_RELIABILITY_ROWS,
    };
    use clap::Parser;
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};

    fn temp_owned_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{ISOLATION_PREFIX}{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn compiled_resource_compiler_attestation_requires_the_exact_marker() {
        let expected = "0123456789abcdef0123456789abcdef|C:\\Windows Kits\\rc.exe";
        let expected_sha256 = format!("{:x}", Sha256::digest(expected.as_bytes()));
        assert_eq!(
            validate_compiled_resource_compiler_attestation(
                Some(expected),
                expected,
                &expected_sha256,
            )
            .expect("exact compiled marker"),
            expected_sha256
        );
        assert!(
            validate_compiled_resource_compiler_attestation(None, expected, &expected_sha256)
                .expect_err("missing compiled marker must fail")
                .contains("omits")
        );
        assert!(validate_compiled_resource_compiler_attestation(
            Some("wrong|C:\\Windows Kits\\rc.exe"),
            expected,
            &expected_sha256,
        )
        .expect_err("mismatched compiled marker must fail")
        .contains("does not match"));
    }

    #[test]
    fn cargo_target_linker_environment_name_is_canonical() {
        assert_eq!(
            cargo_target_linker_environment_name("x86_64-pc-windows-msvc")
                .expect("canonical target linker name"),
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"
        );
        assert!(cargo_target_linker_environment_name("bad/target").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn native_input_path_reconstructs_manifest_separators_as_native_separators() {
        let root = temp_owned_root("native-input-separators");
        let nested = root.join("CppCoreCheck");
        std::fs::create_dir_all(&nested).expect("native input directory");
        std::fs::write(nested.join("warnings.h"), b"fixture").expect("native input fixture");

        let files = collect_ordinary_tree_files(&root).expect("collect native input tree");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "CppCoreCheck/warnings.h");
        let reconstructed =
            native_input_absolute_path(&root, Path::new(&files[0].0)).expect("reconstruct path");
        assert!(paths_equal(&reconstructed, &files[0].1));

        std::fs::remove_dir_all(root).expect("remove native input fixture");
    }

    #[test]
    #[cfg(windows)]
    fn recorded_path_resolution_uses_the_first_existing_executable() {
        let root = temp_owned_root("path-resolution");
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(&first).expect("first PATH directory");
        std::fs::create_dir_all(&second).expect("second PATH directory");
        std::fs::write(second.join("cl.exe"), b"second").expect("second cl fixture");
        let path_value = format!("{};{}", first.display(), second.display());
        assert_eq!(
            resolve_recorded_path_executable(&path_value, "cl.exe")
                .expect("second directory resolves"),
            second.join("cl.exe")
        );
        std::fs::write(first.join("cl.exe"), b"first").expect("first cl fixture");
        assert_eq!(
            resolve_recorded_path_executable(&path_value, "cl.exe").expect("first directory wins"),
            first.join("cl.exe")
        );
        std::fs::remove_dir_all(&root).expect("remove PATH fixture");
    }

    fn registry_tree_fixture(
        root: &Path,
        label: &str,
        manifest_contents: &[u8],
    ) -> (Value, String, PathBuf) {
        std::fs::create_dir_all(root).expect("registry tree fixture");
        let manifest_path = root.join("Cargo.toml");
        std::fs::write(&manifest_path, manifest_contents).expect("registry manifest fixture");
        let root_text = root.to_string_lossy().to_string();
        let manifest_sha256 = format!("{:x}", Sha256::digest(manifest_contents));
        let canonical = format!(
            "root\t{label}\t{root_text}\nfile\tCargo.toml\t{}\t{manifest_sha256}",
            manifest_contents.len()
        );
        let aggregate_sha256 = format!("{:x}", Sha256::digest(canonical.as_bytes()));
        (
            json!({
                "schema_version": 1,
                "kind": "vectorgrid_cpu_reliability_soak_registry_package_tree",
                "aggregate_algorithm":
                    "sha256(root_tab_label_tab_absolute_path_then_file_tab_relative_path_tab_bytes_tab_sha256_lf)",
                "label": label,
                "root_path": root_text,
                "file_count": 1,
                "total_bytes": manifest_contents.len(),
                "files": [{
                    "path": "Cargo.toml",
                    "bytes": manifest_contents.len(),
                    "sha256": manifest_sha256,
                }],
                "aggregate_sha256": aggregate_sha256,
            }),
            aggregate_sha256,
            manifest_path,
        )
    }

    fn required_cli_args(root: &std::path::Path, out: &std::path::Path) -> Vec<String> {
        vec![
            "ta_cpu_reliability_soak".to_string(),
            "--out".to_string(),
            out.to_string_lossy().to_string(),
            "--isolation-root".to_string(),
            root.to_string_lossy().to_string(),
            "--run-id".to_string(),
            "unit-test-run".to_string(),
            "--cargo-target-dir".to_string(),
            root.with_extension("target").to_string_lossy().to_string(),
            "--build-profile".to_string(),
            "release".to_string(),
            "--cargo-build-target-env".to_string(),
            "host-default".to_string(),
            "--cargo-target-triple".to_string(),
            option_env!("TAURI_ENV_TARGET_TRIPLE")
                .unwrap_or("x86_64-pc-windows-msvc")
                .to_string(),
            "--cargo-artifact-path".to_string(),
            root.with_extension("target")
                .join("release")
                .join("ta_cpu_reliability_soak.exe")
                .to_string_lossy()
                .to_string(),
            "--cargo-artifact-emitted".to_string(),
            "true".to_string(),
            "--cargo-artifact-fresh".to_string(),
            "false".to_string(),
            "--binary-sha256".to_string(),
            "0".repeat(64),
            "--source-manifest".to_string(),
            root.with_extension("source.json")
                .to_string_lossy()
                .to_string(),
            "--source-manifest-sha256".to_string(),
            "1".repeat(64),
            "--launcher-manifest".to_string(),
            root.with_extension("launcher.json")
                .to_string_lossy()
                .to_string(),
            "--launcher-manifest-sha256".to_string(),
            "2".repeat(64),
        ]
    }

    #[test]
    fn slope_detects_linear_growth() {
        let slope = linear_regression_slope(&[(1.0, 10.0), (2.0, 14.0), (3.0, 18.0), (4.0, 22.0)])
            .expect("slope");
        assert!((slope - 4.0).abs() < 1e-9);
    }

    #[test]
    fn leak_assessment_fails_growth_or_slope_threshold() {
        let assessment = assess_metric(
            &[(1.0, 10.0), (2.0, 20.0), (3.0, 30.0), (4.0, 40.0)],
            100.0,
            5.0,
        );
        assert!(!assessment.passed);
        assert!(assessment.slope_per_cycle.unwrap_or_default() > 5.0);
    }

    #[test]
    fn leak_assessment_requires_three_post_warmup_samples() {
        let assessment = assess_metric(&[(1.0, 10.0), (2.0, 10.0)], 1.0, 1.0);
        assert!(!assessment.passed);
        assert!(assessment.note.contains("at least three"));
    }

    #[test]
    fn monitoring_uses_full_history_even_when_retained_evidence_is_bounded() {
        let mut samples = SampleWindow::new(4);
        let mut tracker = MonitoringTracker::default();
        let mut storage_tracker = OwnedStorageTracker::default();
        for cycle in 1..=100 {
            let sample = ProcessSample {
                cycle,
                stage: "cycle_complete".to_string(),
                elapsed_ms: cycle as u64,
                working_set_bytes: Some((cycle * 10) as u64),
                private_bytes: Some((cycle * 20) as u64),
                handle_count: Some(cycle as u32),
            };
            tracker.push(&sample, 1);
            storage_tracker.push(
                cycle,
                OwnedStorageStats {
                    file_count: 4,
                    directory_count: 3,
                    total_bytes: 1_024,
                    residual_artifact_or_temp_files: 0,
                },
                1,
            );
            samples.push(sample);
        }
        let assessment = monitoring_assessment(
            &samples,
            &tracker,
            &storage_tracker,
            1,
            LeakThresholds {
                max_private_slope_bytes_per_cycle: 100.0,
                max_private_growth_bytes: 10_000.0,
                max_working_set_slope_bytes_per_cycle: 100.0,
                max_working_set_growth_bytes: 10_000.0,
                max_handle_slope_per_cycle: 10.0,
                max_handle_growth: 1_000,
                max_catalog_growth: 0,
            },
        );
        assert_eq!(assessment.working_set.expected_samples, 99);
        assert_eq!(assessment.working_set.measured_samples, 99);
        assert_eq!(assessment.working_set.first, Some(20.0));
        assert_eq!(assessment.working_set.last, Some(1_000.0));
        assert!(assessment.retained_sample_count <= 5);
    }

    #[test]
    fn monitoring_fails_closed_when_any_metric_sample_is_missing() {
        let mut samples = SampleWindow::new(4);
        let mut tracker = MonitoringTracker::default();
        let mut storage_tracker = OwnedStorageTracker::default();
        for cycle in 1..=4 {
            let sample = ProcessSample {
                cycle,
                stage: "cycle_complete".to_string(),
                elapsed_ms: cycle as u64,
                working_set_bytes: (cycle != 3).then_some(100),
                private_bytes: Some(100),
                handle_count: Some(10),
            };
            tracker.push(&sample, 0);
            storage_tracker.push(
                cycle,
                OwnedStorageStats {
                    file_count: 4,
                    directory_count: 3,
                    total_bytes: 1_024,
                    residual_artifact_or_temp_files: 0,
                },
                0,
            );
            samples.push(sample);
        }
        let assessment = monitoring_assessment(
            &samples,
            &tracker,
            &storage_tracker,
            0,
            LeakThresholds {
                max_private_slope_bytes_per_cycle: 1.0,
                max_private_growth_bytes: 1.0,
                max_working_set_slope_bytes_per_cycle: 1.0,
                max_working_set_growth_bytes: 1.0,
                max_handle_slope_per_cycle: 1.0,
                max_handle_growth: 1,
                max_catalog_growth: 0,
            },
        );
        assert!(!assessment.working_set.complete);
        assert!(!assessment.working_set.passed);
        assert_eq!(assessment.working_set.expected_samples, 4);
        assert_eq!(assessment.working_set.measured_samples, 3);
    }

    #[test]
    fn monitoring_finish_includes_the_trailing_runtime_gap() {
        let mut tracker = MonitoringTracker::default();
        tracker.push(
            &ProcessSample {
                cycle: 1,
                stage: "cycle_complete".to_string(),
                elapsed_ms: 100,
                working_set_bytes: Some(100),
                private_bytes: Some(100),
                handle_count: Some(10),
            },
            0,
        );
        tracker.finish(1_000_000);
        assert_eq!(tracker.max_cycle_sample_gap_ms, 999_900);
    }

    #[test]
    fn owned_storage_residual_files_fail_monitoring() {
        let mut samples = SampleWindow::new(4);
        let mut tracker = MonitoringTracker::default();
        let mut storage_tracker = OwnedStorageTracker::default();
        for cycle in 1..=4 {
            let sample = ProcessSample {
                cycle,
                stage: "cycle_complete".to_string(),
                elapsed_ms: cycle as u64,
                working_set_bytes: Some(100),
                private_bytes: Some(100),
                handle_count: Some(10),
            };
            tracker.push(&sample, 0);
            samples.push(sample);
            storage_tracker.push(
                cycle,
                OwnedStorageStats {
                    file_count: 4,
                    directory_count: 3,
                    total_bytes: 1_024,
                    residual_artifact_or_temp_files: if cycle == 2 { 1 } else { 0 },
                },
                0,
            );
        }
        let assessment = monitoring_assessment(
            &samples,
            &tracker,
            &storage_tracker,
            0,
            LeakThresholds {
                max_private_slope_bytes_per_cycle: 1.0,
                max_private_growth_bytes: 1.0,
                max_working_set_slope_bytes_per_cycle: 1.0,
                max_working_set_growth_bytes: 1.0,
                max_handle_slope_per_cycle: 1.0,
                max_handle_growth: 1,
                max_catalog_growth: 0,
            },
        );
        assert!(!assessment.owned_storage.passed);
        assert_eq!(
            assessment.owned_storage.max_residual_artifact_or_temp_files,
            1
        );
    }

    #[test]
    fn source_manifest_rejects_an_incorrect_aggregate() {
        let manifest = json!({
            "schema_version": 1,
            "kind": "vectorgrid_cpu_reliability_soak_source_manifest",
            "aggregate_algorithm": "sha256(relative_path_tab_bytes_tab_sha256_lf)",
            "aggregate_sha256": "f".repeat(64),
            "file_count": 1,
            "files": [{
                "path": "Cargo.toml",
                "bytes": 1,
                "sha256": "0".repeat(64),
            }],
        });
        let error = validate_source_manifest(&manifest).expect_err("aggregate must fail");
        assert!(error.contains("aggregate mismatch"));
    }

    #[test]
    fn windows_sdk_versions_use_numeric_component_ordering() {
        assert!(
            parse_windows_sdk_version("10.0.9.0").expect("older SDK")
                < parse_windows_sdk_version("10.0.10.0").expect("newer SDK")
        );
        assert!(parse_windows_sdk_version("10.0.26100").is_err());
        assert!(parse_windows_sdk_version("10.0.+26100.0").is_err());
        assert!(parse_windows_sdk_version("10.0.2147483648.0").is_err());
    }

    #[test]
    fn cargo_resolution_attestation_binds_the_exact_windows_build_chain() {
        let root = temp_owned_root("cargo-resolution");
        let root_package_dir = root.join("src-tauri");
        let vendor = root.join("vendor").join("embed-resource-3.0.6");
        std::fs::create_dir_all(&root_package_dir).expect("root package fixture");
        std::fs::create_dir_all(&vendor).expect("vendor fixture");
        let cargo_home = root.join("cargo-home");
        let registry_index = cargo_home
            .join("registry")
            .join("src")
            .join("fixture-index");
        let cargo_path = root.join("cargo.exe");
        let cargo_contents = b"pinned cargo fixture";
        std::fs::write(&cargo_path, cargo_contents).expect("cargo fixture");
        let root_manifest_path = root_package_dir.join("Cargo.toml");
        let root_manifest_contents =
            b"[package]\nname = \"ta_desktop_demo_app\"\nversion = \"0.1.0\"\n";
        std::fs::write(&root_manifest_path, root_manifest_contents).expect("root manifest fixture");
        let embed_manifest_path = vendor.join("Cargo.toml");
        let embed_manifest_contents =
            b"[package]\nname = \"embed-resource\"\nversion = \"3.0.6\"\n";
        std::fs::write(&embed_manifest_path, embed_manifest_contents)
            .expect("embed manifest fixture");
        let (tauri_build_tree, tauri_build_tree_sha256, tauri_build_manifest_path) =
            registry_tree_fixture(
                &registry_index.join("tauri-build-2.5.6"),
                "tauri-build@2.5.6",
                b"[package]\nname = \"tauri-build\"\nversion = \"2.5.6\"\n",
            );
        let (tauri_winres_tree, tauri_winres_tree_sha256, tauri_winres_manifest_path) =
            registry_tree_fixture(
                &registry_index.join("tauri-winres-0.3.5"),
                "tauri-winres@0.3.5",
                b"[package]\nname = \"tauri-winres\"\nversion = \"0.3.5\"\n",
            );

        let root_text = root.to_string_lossy().to_string();
        let cargo_home_text = cargo_home.to_string_lossy().to_string();
        let cargo_path_text = cargo_path.to_string_lossy().to_string();
        let root_manifest_path_text = root_manifest_path.to_string_lossy().to_string();
        let embed_manifest_path_text = embed_manifest_path.to_string_lossy().to_string();
        let tauri_build_manifest_path_text =
            tauri_build_manifest_path.to_string_lossy().to_string();
        let tauri_winres_manifest_path_text =
            tauri_winres_manifest_path.to_string_lossy().to_string();
        let cargo_sha256 = format!("{:x}", Sha256::digest(cargo_contents));
        let root_manifest_sha256 = format!("{:x}", Sha256::digest(root_manifest_contents));
        let embed_manifest_sha256 = format!("{:x}", Sha256::digest(embed_manifest_contents));
        let metadata_output_sha256 = "a".repeat(64);
        let root_package_id =
            expected_path_package_id(&root_package_dir, "ta_desktop_demo_app", "0.1.0")
                .expect("root package ID");
        let embed_package_id =
            expected_path_package_id(&vendor, "embed-resource", "3.0.6").expect("embed package ID");
        let tauri_build_package_id =
            "registry+https://github.com/rust-lang/crates.io-index#tauri-build@2.5.6";
        let tauri_winres_package_id =
            "registry+https://github.com/rust-lang/crates.io-index#tauri-winres@0.3.5";
        let registry_source = "registry+https://github.com/rust-lang/crates.io-index";
        let metadata_arguments = [
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--filter-platform",
            "x86_64-pc-windows-msvc",
            "--no-default-features",
            "--features",
            "ta_desktop_demo_app/maintainer-tools",
        ];
        let canonical = format!(
            "metadata_format_version\t1\nmetadata_output_sha256\t{metadata_output_sha256}\nmetadata_arguments\t{}\nworkspace_root\t{root_text}\ncargo_home\t{cargo_home_text}\nworkspace_member_count\t5\nroot_resolved_features\tmaintainer-tools\ncargo\t{cargo_path_text}\t{}\t{cargo_sha256}\nroot_package_match_count\t1\nroot_package\t{root_package_id}\t0.1.0\tnull\t{root_manifest_path_text}\tsrc-tauri/Cargo.toml\t{}\t{root_manifest_sha256}\ntauri_build_match_count\t1\ntauri_build\t{tauri_build_package_id}\t2.5.6\t{registry_source}\t{tauri_build_manifest_path_text}\t{tauri_build_tree_sha256}\ntauri_winres_match_count\t1\ntauri_winres\t{tauri_winres_package_id}\t0.3.5\t{registry_source}\t{tauri_winres_manifest_path_text}\t{tauri_winres_tree_sha256}\nembed_resource_match_count\t1\nembed_resource\t{embed_package_id}\t3.0.6\tnull\t{embed_manifest_path_text}\tvendor/embed-resource-3.0.6/Cargo.toml\t{}\t{embed_manifest_sha256}\nroot_to_tauri_build_declared_match_count\t1\nroot_to_tauri_build_resolved_match_count\t1\nroot_to_tauri_build_edge\t{root_package_id}\ttauri-build\ttauri_build\t{tauri_build_package_id}\tbuild\tnull\t^2\ntauri_build_to_winres_declared_match_count\t1\ntauri_build_to_winres_resolved_match_count\t1\ntauri_build_to_winres_edge\t{tauri_build_package_id}\ttauri-winres\ttauri_winres\t{tauri_winres_package_id}\tnormal\tnull\t^0.3\nwinres_to_embed_declared_match_count\t1\nwinres_to_embed_resolved_match_count\t1\nwinres_to_embed_edge\t{tauri_winres_package_id}\tembed-resource\tembed_resource\t{embed_package_id}\tnormal\tnull\t^3",
            metadata_arguments.join("\t"),
            cargo_contents.len(),
            root_manifest_contents.len(),
            embed_manifest_contents.len(),
        );
        let aggregate_sha256 = format!("{:x}", Sha256::digest(canonical.as_bytes()));
        let mut resolution = json!({
            "schema_version": 2,
            "kind": "vectorgrid_cpu_reliability_soak_windows_build_dependency_chain",
            "aggregate_algorithm": "sha256(canonical_tab_rows_lf)",
            "metadata_format_version": 1,
            "metadata_output_sha256": metadata_output_sha256,
            "metadata_arguments": metadata_arguments,
            "workspace_root": root_text,
            "cargo_home": cargo_home_text,
            "workspace_member_count": 5,
            "root_resolved_features": ["maintainer-tools"],
            "cargo": {
                "path": cargo_path_text,
                "bytes": cargo_contents.len(),
                "sha256": cargo_sha256,
            },
            "root_package_name_match_count": 1,
            "root_package": {
                "package_id": root_package_id,
                "name": "ta_desktop_demo_app",
                "version": "0.1.0",
                "source": null,
                "manifest_path": root_manifest_path_text,
                "manifest_repo_relative_path": "src-tauri/Cargo.toml",
                "manifest_bytes": root_manifest_contents.len(),
                "manifest_sha256": root_manifest_sha256,
                "workspace_member": true,
            },
            "tauri_build_package_name_match_count": 1,
            "tauri_build": {
                "package_id": tauri_build_package_id,
                "name": "tauri-build",
                "version": "2.5.6",
                "source": registry_source,
                "manifest_path": tauri_build_manifest_path_text,
                "source_tree": tauri_build_tree,
            },
            "tauri_winres_package_name_match_count": 1,
            "tauri_winres": {
                "package_id": tauri_winres_package_id,
                "name": "tauri-winres",
                "version": "0.3.5",
                "source": registry_source,
                "manifest_path": tauri_winres_manifest_path_text,
                "source_tree": tauri_winres_tree,
            },
            "embed_resource_package_name_match_count": 1,
            "embed_resource": {
                "package_id": embed_package_id,
                "name": "embed-resource",
                "version": "3.0.6",
                "source": null,
                "manifest_path": embed_manifest_path_text,
                "manifest_repo_relative_path": "vendor/embed-resource-3.0.6/Cargo.toml",
                "manifest_bytes": embed_manifest_contents.len(),
                "manifest_sha256": embed_manifest_sha256,
                "workspace_member": true,
            },
            "root_to_tauri_build_declared_edge_match_count": 1,
            "root_to_tauri_build_resolved_edge_match_count": 1,
            "root_to_tauri_build_edge": {
                "from_package_id": root_package_id,
                "declared_dependency_name": "tauri-build",
                "resolved_dependency_name": "tauri_build",
                "to_package_id": tauri_build_package_id,
                "kind": "build",
                "target": null,
                "declared_requirement": "^2",
            },
            "tauri_build_to_winres_declared_edge_match_count": 1,
            "tauri_build_to_winres_resolved_edge_match_count": 1,
            "tauri_build_to_winres_edge": {
                "from_package_id": tauri_build_package_id,
                "declared_dependency_name": "tauri-winres",
                "resolved_dependency_name": "tauri_winres",
                "to_package_id": tauri_winres_package_id,
                "kind": "normal",
                "target": null,
                "declared_requirement": "^0.3",
            },
            "winres_to_embed_declared_edge_match_count": 1,
            "winres_to_embed_resolved_edge_match_count": 1,
            "winres_to_embed_edge": {
                "from_package_id": tauri_winres_package_id,
                "declared_dependency_name": "embed-resource",
                "resolved_dependency_name": "embed_resource",
                "to_package_id": embed_package_id,
                "kind": "normal",
                "target": null,
                "declared_requirement": "^3",
            },
            "aggregate_sha256": aggregate_sha256,
        });
        let toolchain = json!({
            "cargo_path": cargo_path_text,
            "cargo_sha256": cargo_sha256,
        });
        validate_embed_resource_cargo_resolution(
            &resolution,
            &root,
            &cargo_home,
            "x86_64-pc-windows-msvc",
            &toolchain,
        )
        .expect("exact Windows Cargo build chain must validate");

        let exact_resolution = resolution.clone();
        resolution["tauri_build_to_winres_edge"]["to_package_id"] =
            json!("registry+https://github.com/rust-lang/crates.io-index#wrong@0.0.0");
        assert!(validate_embed_resource_cargo_resolution(
            &resolution,
            &root,
            &cargo_home,
            "x86_64-pc-windows-msvc",
            &toolchain,
        )
        .expect_err("changed intermediate edge must fail")
        .contains("Cargo build-chain edge is invalid"));

        let mut tampered_tree = exact_resolution;
        tampered_tree["tauri_build"]["source_tree"]["files"][0]["sha256"] = json!("0".repeat(64));
        assert!(validate_embed_resource_cargo_resolution(
            &tampered_tree,
            &root,
            &cargo_home,
            "x86_64-pc-windows-msvc",
            &toolchain,
        )
        .expect_err("changed registry package file row must fail")
        .contains("Cargo registry package file changed"));
        std::fs::remove_dir_all(root).expect("remove Cargo-resolution fixture");
    }

    #[test]
    fn resource_compiler_candidate_enumeration_is_complete_and_numeric() {
        let root = temp_owned_root("resource-compiler-candidates");
        for version in ["10.0.10.0", "10.0.9.0"] {
            let x64 = root.join("bin").join(version).join("x64");
            std::fs::create_dir_all(&x64).expect("SDK candidate fixture");
            std::fs::write(x64.join("rc.exe"), version.as_bytes()).expect("rc fixture");
        }
        std::fs::create_dir_all(root.join("bin").join("not-a-version"))
            .expect("non-version fixture");

        let candidates =
            enumerate_resource_compiler_candidates(&root).expect("enumerate SDK candidates");
        assert_eq!(
            candidates
                .iter()
                .map(|(version, _)| version.as_str())
                .collect::<Vec<_>>(),
            vec!["10.0.9.0", "10.0.10.0"]
        );
        assert!(candidates
            .iter()
            .all(|(_, path)| path.ends_with(Path::new("x64").join("rc.exe"))));

        std::fs::remove_dir_all(root).expect("remove SDK candidate fixture");
    }

    #[test]
    fn operator_cli_rejects_relaxed_workload_and_allowed_failures() {
        let root = temp_owned_root("operator-policy");
        let out = root.with_extension("json");
        let mut canonical = required_cli_args(&root, &out);
        canonical.extend([
            "--mode-label".to_string(),
            "operator".to_string(),
            "--duration-seconds".to_string(),
            "21600".to_string(),
            "--fresh-build".to_string(),
            "--rows".to_string(),
            "20000".to_string(),
            "--max-private-slope-mib-per-cycle".to_string(),
            "0.5".to_string(),
            "--max-working-set-slope-mib-per-cycle".to_string(),
            "0.5".to_string(),
            "--max-handle-slope-per-cycle".to_string(),
            "0.10".to_string(),
        ]);
        let cli = Cli::try_parse_from(canonical.clone()).expect("canonical parse");
        ValidatedConfig::from_cli(cli).expect("canonical operator config");

        let mut allowed_failure = canonical.clone();
        allowed_failure.extend(["--allowed-failures".to_string(), "1".to_string()]);
        let cli = Cli::try_parse_from(allowed_failure).expect("allowed failure parse");
        assert!(ValidatedConfig::from_cli(cli)
            .expect_err("allowed failures must fail")
            .contains("zero allowed failures"));

        let rows_index = canonical
            .iter()
            .position(|value| value == "--rows")
            .expect("rows flag")
            + 1;
        canonical[rows_index] = MIN_RELIABILITY_ROWS.to_string();
        let cli = Cli::try_parse_from(canonical).expect("undersized parse");
        assert!(ValidatedConfig::from_cli(cli)
            .expect_err("undersized workload must fail")
            .contains("canonical workload"));
    }

    #[test]
    fn cli_rejects_output_inside_isolation_root() {
        let root = temp_owned_root("args");
        let args = required_cli_args(&root, &root.join("evidence.json"));
        let cli = Cli::try_parse_from(args).expect("parse");
        let error = ValidatedConfig::from_cli(cli).expect_err("must reject");
        assert!(error.contains("outside isolation_root"));
    }

    #[test]
    fn cli_rejects_disabled_cancellation() {
        let root = temp_owned_root("cancel");
        let out = root.with_extension("json");
        let mut args = required_cli_args(&root, &out);
        args.extend(["--cancel-every".to_string(), "0".to_string()]);
        let cli = Cli::try_parse_from(args).expect("parse");
        let error = ValidatedConfig::from_cli(cli).expect_err("must reject");
        assert!(error.contains("must exercise cancellation"));
    }

    #[test]
    fn cli_enforces_multi_tile_cancellation_row_minimum() {
        let root = temp_owned_root("row-minimum");
        let out = root.with_extension("json");
        let base = required_cli_args(&root, &out);

        let mut minimum = base.clone();
        minimum.extend(["--rows".to_string(), MIN_RELIABILITY_ROWS.to_string()]);
        let cli = Cli::try_parse_from(minimum).expect("minimum rows parse");
        ValidatedConfig::from_cli(cli).expect("minimum rows remain authoritative");

        let mut undersized = base;
        undersized.extend(["--rows".to_string(), (MIN_RELIABILITY_ROWS - 1).to_string()]);
        let cli = Cli::try_parse_from(undersized).expect("undersized rows parse");
        let error = ValidatedConfig::from_cli(cli).expect_err("undersized rows must fail");
        assert!(error.contains("spans multiple CPU tiles"));
    }

    #[test]
    fn cleanup_refuses_unmarked_tree() {
        let root = temp_owned_root("unmarked");
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(root.join("keep.txt"), b"keep").expect("sentinel");
        let error = cleanup_owned_isolation_root(&root, "wrong").expect_err("must refuse");
        assert!(error.contains("unmarked isolation root"));
        assert!(root.join("keep.txt").exists());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn cleanup_refuses_mismatched_nonce() {
        let root = temp_owned_root("nonce");
        let guard = IsolationGuard::create(&root).expect("owned root");
        let error = cleanup_owned_isolation_root(&root, "wrong").expect_err("must refuse");
        assert!(error.contains("mismatched ownership marker"));
        assert!(root.exists());
        drop(guard);
        assert!(!root.exists());
    }

    #[test]
    fn owned_guard_cleanup_removes_only_marked_tree() {
        let root = temp_owned_root("owned");
        let sibling =
            root.with_file_name(format!("{ISOLATION_PREFIX}sibling-{}", std::process::id()));
        std::fs::create_dir_all(&sibling).expect("sibling");
        std::fs::write(sibling.join("keep.txt"), b"keep").expect("sibling sentinel");
        let mut guard = IsolationGuard::create(&root).expect("guard");
        std::fs::write(root.join("remove.txt"), b"remove").expect("owned sentinel");
        guard.cleanup().expect("cleanup");
        assert!(!root.exists());
        assert!(sibling.join("keep.txt").exists());
        std::fs::remove_dir_all(sibling).ok();
    }

    #[test]
    fn app_data_registration_rejects_paths_outside_isolation_root() {
        let root = temp_owned_root("app-data-boundary");
        let outside = std::env::temp_dir().join(format!(
            "vectorgrid-cpusoak-outside-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let mut guard = IsolationGuard::create(&root).expect("guard");
        let error = guard
            .register_app_data_root(&outside)
            .expect_err("outside app data must be rejected");
        assert!(error.contains("strict descendant"));
        assert!(!outside.exists());
        drop(guard);
        assert!(!root.exists());
    }

    #[test]
    fn parser_accepts_safe_short_configuration() {
        let root = temp_owned_root("valid");
        let out = root.with_extension("json");
        let args = required_cli_args(&root, &out);
        let cli = Cli::try_parse_from(args).expect("parse");
        let config = ValidatedConfig::from_cli(cli).expect("validate");
        assert_eq!(config.cycles, 4);
    }
}
