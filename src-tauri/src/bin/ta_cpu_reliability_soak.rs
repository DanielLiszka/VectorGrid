use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
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
        return metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
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
                    .trim_end_matches(|value| value == '\\' || value == '/')
                    .to_ascii_lowercase()
                    + "\\"),
            )
    }
    #[cfg(not(windows))]
    {
        candidate.starts_with(root)
    }
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

fn validate_native_build_provenance(
    native: &Value,
    config: &ValidatedConfig,
) -> Result<(), String> {
    if native.get("schema_version").and_then(Value::as_u64) != Some(1)
        || native.get("kind").and_then(Value::as_str)
            != Some("vectorgrid_cpu_reliability_soak_native_build_provenance")
        || native.get("host_target_triple").and_then(Value::as_str)
            != Some(config.cargo_target_triple.as_str())
        || native.get("selection_policy").and_then(Value::as_str)
            != Some("vswhere_latest_vc_tools_plus_latest_windows_sdk_with_native_overrides_cleared")
        || native
            .get("override_environment_cleared")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("native build provenance identity/policy is invalid".to_string());
    }

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
    for (path, label) in [
        (visual_studio_installation, "Visual Studio installation"),
        (windows_sdk_root, "Windows SDK root"),
    ] {
        let path = absolute_lexical_path(Path::new(path))?;
        ensure_no_reparse_ancestors(&path, label)?;
        if !path.is_dir() {
            return Err(format!("native {label} is not a directory"));
        }
    }

    let expected_tool_names = ["cl", "lib", "link", "ml64", "mt", "rc", "vswhere"];
    let tools = native
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| "native build tools must be an array".to_string())?;
    if tools.len() != expected_tool_names.len() {
        return Err("native build tool coverage is incomplete".to_string());
    }
    let mut tool_canonical = String::new();
    let mut tool_parent_paths = Vec::new();
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
        if matches!(name, "cl" | "rc") {
            tool_parent_paths.push(
                path.parent()
                    .ok_or_else(|| format!("native build tool has no parent: {name}"))?
                    .to_path_buf(),
            );
        }
    }
    let path_prefixes = native
        .get("path_prefix_directories")
        .and_then(Value::as_array)
        .ok_or_else(|| "native path-prefix directories are missing".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "native path-prefix entries must be strings".to_string())
                .and_then(|value| absolute_lexical_path(Path::new(value)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if path_prefixes != tool_parent_paths {
        return Err(
            "native tool PATH prefix does not match the attested cl/rc locations".to_string(),
        );
    }

    let expected_environment_names = [
        "PATH",
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "VSINSTALLDIR",
        "VCINSTALLDIR",
        "VCToolsInstallDir",
        "VCToolsVersion",
        "VisualStudioVersion",
        "WindowsSdkDir",
        "WindowsSDKVersion",
        "UniversalCRTSdkDir",
        "UCRTVersion",
    ];
    let environment = native
        .get("environment")
        .and_then(Value::as_array)
        .ok_or_else(|| "native build environment manifest is missing".to_string())?;
    if environment.len() != expected_environment_names.len() {
        return Err("native build environment coverage is incomplete".to_string());
    }
    let mut environment_canonical = String::new();
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
        let current = std::env::var(name).ok();
        let expected_present = row
            .get("present")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("native build environment presence is missing: {name}"))?;
        if current.is_some() != expected_present {
            return Err(format!("native build environment presence changed: {name}"));
        }
        let current_sha256 = current
            .as_ref()
            .map(|value| format!("{:x}", Sha256::digest(value.as_bytes())));
        let expected_sha256 = row.get("value_sha256").and_then(Value::as_str);
        if current_sha256.as_deref() != expected_sha256 {
            return Err(format!("native build environment changed: {name}"));
        }
        if index > 0 {
            environment_canonical.push('\n');
        }
        environment_canonical.push_str(&format!(
            "{name}\t{}\t{}",
            expected_present,
            expected_sha256.unwrap_or_default()
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
    let aggregate_canonical = format!(
        "host_target_triple\t{}\nvisual_studio_installation\t{}\nvc_tools_version\t{}\nwindows_sdk_root\t{}\nwindows_sdk_version\t{}\ntools\t{}\nenvironment\t{}",
        config.cargo_target_triple,
        visual_studio_installation,
        vc_tools_version,
        windows_sdk_root,
        windows_sdk_version,
        tool_canonical,
        environment_canonical
    );
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
    Ok(())
}

fn validate_launcher_manifest(
    contents: &Value,
    source_manifest: &Value,
    config: &ValidatedConfig,
) -> Result<(), String> {
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
            .map(|value| Path::new(value))
            != Some(config.cargo_target_dir.as_path())
        || build
            .get("cargo_artifact_path")
            .and_then(Value::as_str)
            .map(|value| Path::new(value))
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
        || binary
            .get("path")
            .and_then(Value::as_str)
            .map(|value| Path::new(value))
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
        || source
            .get("path")
            .and_then(Value::as_str)
            .map(|value| Path::new(value))
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
    validate_cargo_configuration_provenance(
        contents
            .get("cargo_configuration")
            .ok_or_else(|| "launcher Cargo configuration section is missing".to_string())?,
    )?;
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
    if config.fresh_build {
        validate_native_build_provenance(
            contents
                .get("native_build")
                .ok_or_else(|| "launcher native build provenance is missing".to_string())?,
            config,
        )?;
    } else if contents
        .get("native_build")
        .is_some_and(|value| !value.is_null())
    {
        return Err("prebuilt launcher must not claim fresh native build provenance".to_string());
    }
    Ok(())
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
    validate_launcher_manifest(&launcher_manifest, &source_manifest, config)?;
    let retained_binary_path = launcher_manifest
        .pointer("/binary/retained_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher retained binary path is missing".to_string())?;
    let retained_binary_path = absolute_lexical_path(Path::new(retained_binary_path))?;
    let retained_binary_sha256 = sha256_file(&retained_binary_path)?;

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
            "execution_path": executable,
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
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "VSINSTALLDIR",
        "VCINSTALLDIR",
        "VCTOOLSINSTALLDIR",
        "VCTOOLSVERSION",
        "VISUALSTUDIOVERSION",
        "WINDOWSSDKDIR",
        "WINDOWSSDKVERSION",
        "UNIVERSALCRTSDKDIR",
        "UCRTVERSION",
        "DEVENVDIR",
        "EXTENSIONSDKDIR",
        "FRAMEWORKDIR",
        "FRAMEWORKVERSION",
        "FRAMEWORKVERSION64",
    ];
    const TOOL_TOKENS: &[&str] = &[
        "CC", "CXX", "CPP", "AR", "RANLIB", "LD", "LINKER", "ASM", "NASM", "WINDRES", "RC",
        "CFLAGS", "CXXFLAGS", "CPPFLAGS", "LDFLAGS", "ARFLAGS", "ASMFLAGS",
    ];
    std::env::vars().all(|(name, value)| {
        if value.trim().is_empty() {
            return true;
        }
        let upper = name.to_ascii_uppercase();
        if FIXED_NAMES.contains(&upper.as_str()) {
            return false;
        }
        !TOOL_TOKENS.iter().any(|token| {
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
    drop(app_state);
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
        assess_metric, cleanup_owned_isolation_root, linear_regression_slope,
        monitoring_assessment, validate_source_manifest, Cli, IsolationGuard, LeakThresholds,
        MonitoringTracker, OwnedStorageStats, OwnedStorageTracker, ProcessSample, SampleWindow,
        ValidatedConfig, ISOLATION_PREFIX, MIN_RELIABILITY_ROWS,
    };
    use clap::Parser;
    use serde_json::json;
    use std::path::PathBuf;

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
