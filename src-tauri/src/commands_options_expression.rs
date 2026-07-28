use crate::commands::options_parent_adapters::{
    build_supported_options_parent_replay, build_supported_options_parent_signal_manifest,
    supported_options_parent_family_from_artifact, supported_options_parent_strategy_list,
};
use crate::data_cache;
use crate::settings;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{hash_map::DefaultHasher, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::UNIX_EPOCH;
use ta_app_core::options_types::{
    OptionsExpressionContractQuoteQualitySummary, OptionsExpressionCoverageAdequacyVerdict,
    OptionsExpressionDatasetProvenance, OptionsExpressionDatasetQuoteQualitySummary,
    OptionsExpressionMetrics, OptionsExpressionValidationDatasetPolicy,
    OptionsExpressionValidationPlan, OptionsExpressionValidationPlanMode,
    OptionsExpressionValidationRequest, OptionsExpressionValidationResult,
    OptionsExpressionValidationTupleSelection, OptionsExpressionValidationVerdict,
};
use ta_app_core::{
    analyze_options_expression_feasibility_blocking,
    compute_double_ma_drilldown_blocking_with_candles, dataset_manifest_path,
    inspect_options_expression_event_path_blocking, inspect_options_expression_tuple_blocking,
    load_imported_options_dataset_from_csv, parse_timeframe_label, read_options_dataset_manifest,
    resample_candles, run_options_expression_blocking, validate_options_expression_tuples_blocking,
    ImportedOptionsDatasetCsvBuildRequest, OptionsExpressionArtifactV1,
    OptionsExpressionEventPathInspectionResult, OptionsExpressionFeasibilityReport,
    OptionsExpressionRunRequest, OptionsExpressionRunResult,
    OptionsExpressionTupleInspectionResult, OptionsPriceSemantics,
    OPTIONS_EXPRESSION_ARTIFACT_VERSION, OPTIONS_EXPRESSION_VALIDATION_ARTIFACT_VERSION,
};
use tauri::Manager;

const OPTIONS_REPLAY_CACHE_LIMIT: usize = 4;
const OPTIONS_DATASET_REPLAY_CACHE_VERSION: u32 = 1;
static OPTIONS_OUTPUT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn ensure_options_expression_not_cancelled(
    cancel: Option<&AtomicBool>,
    phase: &str,
) -> Result<(), String> {
    if cancel.is_some_and(|token| token.load(Ordering::SeqCst)) {
        return Err(format!(
            "options expression operation cancelled before {phase}"
        ));
    }
    Ok(())
}

struct OptionsOutputStageGuard {
    path: PathBuf,
    published: bool,
}

impl OptionsOutputStageGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            published: false,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn mark_published(&mut self) {
        self.published = true;
    }

    fn preserve_for_recovery(&mut self) {
        self.published = true;
    }
}

impl Drop for OptionsOutputStageGuard {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RunOptionsExpressionResponse {
    pub artifact_path: String,
    pub artifact: OptionsExpressionArtifactV1,
    pub result: OptionsExpressionRunResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeOptionsExpressionFeasibilityResponse {
    pub replay_build_ms: u64,
    pub total_preview_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_build_timing: Option<OptionsExpressionReplayBuildTimingBreakdown>,
    pub report: OptionsExpressionFeasibilityReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidateOptionsExpressionArtifactResponse {
    pub artifact_path: String,
    pub replay_build_ms: u64,
    pub validation_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_build_timing: Option<OptionsExpressionReplayBuildTimingBreakdown>,
    pub result: OptionsExpressionValidationResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeOptionsExpressionValidationFeasibilityResponse {
    pub replay_build_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_build_timing: Option<OptionsExpressionReplayBuildTimingBreakdown>,
    pub artifact_path: String,
    pub source_artifact_hash: String,
    pub options_dataset_id: String,
    pub options_dataset_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_dataset_normalized_rows_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_dataset_manifest_semantic_sha256: Option<String>,
    pub dataset_policy: OptionsExpressionValidationDatasetPolicy,
    pub replacement_dataset: bool,
    pub validation_plan: OptionsExpressionValidationPlan,
    pub tuple_selection: OptionsExpressionValidationTupleSelection,
    pub selected_tuple_indexes: Vec<usize>,
    pub selected_tuple_count: usize,
    pub price_semantics: OptionsPriceSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_provenance: Option<OptionsExpressionDatasetProvenance>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct OptionsExpressionReplayBuildTimingBreakdown {
    pub total_us: u64,
    pub parent_artifact_validation_us: u64,
    pub source_resolution_us: u64,
    pub parent_signal_process_cache_lookup_us: u64,
    #[serde(default)]
    pub parent_signal_process_cache_hit: bool,
    pub persisted_signal_manifest_cache_lookup_us: u64,
    #[serde(default)]
    pub persisted_signal_manifest_cache_hit: bool,
    pub candle_replay_load_us: u64,
    pub parent_replay_and_drilldown_us: u64,
    pub signal_manifest_materialization_us: u64,
    pub options_dataset_manifest_validation_us: u64,
    pub options_dataset_process_cache_lookup_us: u64,
    #[serde(default)]
    pub options_dataset_process_cache_hit: bool,
    pub options_dataset_persisted_cache_lookup_us: u64,
    #[serde(default)]
    pub options_dataset_persisted_cache_hit: bool,
    pub options_dataset_csv_load_us: u64,
    pub request_normalization_us: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveOptionsCsvRequest {
    pub path: String,
    pub csv_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveOptionsCsvResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsExpressionArtifactLeaderSummary {
    pub search_leader_tuple_index: Option<usize>,
    pub robust_leader_tuple_index: Option<usize>,
    pub holdout_leader_tuple_index: Option<usize>,
    pub sealed_leader_tuple_index: Option<usize>,
    pub stress_leader_tuple_index: Option<usize>,
    pub distinct_leader_count: usize,
    pub search_vs_robust_disagree: bool,
    pub robust_vs_holdout_disagree: Option<bool>,
    pub robust_vs_sealed_disagree: Option<bool>,
    pub robust_leader_reached_sealed_finalist: Option<bool>,
    pub robust_leader_sealed_final_rank: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsExpressionArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub parent_artifact_id: String,
    pub parent_strategy_kind: String,
    pub root_symbol: String,
    pub options_dataset_id: String,
    pub backend_used: String,
    pub backend_fallback_reason: Option<String>,
    pub price_semantics: OptionsPriceSemantics,
    pub request_backend_preference: String,
    pub request_objective_profile: String,
    pub request_fill_model: String,
    pub request_structure_family: String,
    pub request_split_assignment_mode: String,
    pub coverage_adequacy_verdict: OptionsExpressionCoverageAdequacyVerdict,
    pub coverage_non_promotable: bool,
    pub search_overlap_rate: f64,
    pub holdout_overlap_rate: f64,
    pub sealed_test_overlap_rate: f64,
    pub search_candidate_coverage_rate: f64,
    pub holdout_candidate_coverage_rate: f64,
    pub sealed_test_candidate_coverage_rate: f64,
    pub search_baseline_outcome_coverage_rate: f64,
    pub holdout_baseline_outcome_coverage_rate: f64,
    pub sealed_test_baseline_outcome_coverage_rate: f64,
    pub tuple_count: usize,
    pub best_robust_score: f64,
    pub best_selection_expectancy_per_premium: f64,
    pub best_holdout_expectancy_per_premium: Option<f64>,
    pub best_sealed_expectancy_per_premium: Option<f64>,
    pub best_stress_expectancy_per_premium: Option<f64>,
    pub best_sealed_stress_expectancy_per_premium: Option<f64>,
    pub best_harsher_stress_expectancy_per_premium: Option<f64>,
    pub best_sealed_harsher_stress_expectancy_per_premium: Option<f64>,
    pub best_extreme_stress_expectancy_per_premium: Option<f64>,
    pub best_sealed_extreme_stress_expectancy_per_premium: Option<f64>,
    pub leader_summary: OptionsExpressionArtifactLeaderSummary,
    pub sealed_finalist_budget: usize,
    pub sealed_evaluated_tuple_count: usize,
    pub top_validation_verdict: OptionsExpressionValidationVerdict,
    pub validated_tuple_count: usize,
    pub stress_fragile_tuple_count: usize,
    pub supported_holdout_tuple_count: usize,
    pub supported_sealed_tuple_count: usize,
    pub supported_stress_tuple_count: usize,
    pub has_holdout_metrics: bool,
    pub has_sealed_evaluation_metrics: bool,
    pub has_stress_metrics: bool,
    pub has_harsher_stress_metrics: bool,
    pub has_extreme_stress_metrics: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exactness_semantics: Option<Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InspectOptionsExpressionTupleRequest {
    pub artifact_path: String,
    pub tuple_index: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InspectOptionsExpressionEventPathRequest {
    pub artifact_path: String,
    pub tuple_index: usize,
    pub event_index: usize,
}

#[derive(Clone)]
struct CachedParentSignalReplay {
    cache_key: String,
    parent_artifact_id: String,
    signal_manifest: Arc<ta_app_core::OptionsSignalEventManifestV1>,
}

#[derive(Clone)]
struct CachedOptionsDatasetReplay {
    cache_key: String,
    options_dataset_hash: String,
    dataset: Arc<ta_app_core::ImportedOptionsDataset>,
}

#[derive(Clone)]
struct OptionsExpressionReplayInputsBuildResult {
    inputs: OptionsExpressionReplayInputs,
    timing: OptionsExpressionReplayBuildTimingBreakdown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedOptionsDatasetReplayCacheV1 {
    version: u32,
    options_dataset_hash: String,
    dataset: PersistedImportedOptionsDatasetV1,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedImportedOptionsDatasetV1 {
    manifest: PersistedOptionsDatasetManifestV1,
    #[serde(default)]
    contracts: Vec<PersistedOptionsContractSeriesV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedOptionsDatasetManifestV1 {
    version: u32,
    options_dataset_id: String,
    root_symbol: String,
    provider_name: String,
    provider_source_surface: String,
    #[serde(default)]
    provider_entitlement_profile: Option<String>,
    quote_resolution: String,
    price_semantics: OptionsPriceSemantics,
    calendar_id: String,
    timezone: String,
    coverage_start: i64,
    coverage_end: i64,
    normalization_version: u32,
    quote_semantics_version: u32,
    corporate_action_mapping_version: u32,
    contract_reference_source: String,
    #[serde(default)]
    contract_reference_as_of_utc: Option<String>,
    underlier_reference_source: String,
    dataset_path: String,
    row_count: usize,
    contract_count: usize,
    #[serde(default)]
    aggregate_price_field: Option<String>,
    #[serde(default)]
    normalized_rows_sha256: Option<String>,
    #[serde(default)]
    manifest_semantic_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedOptionsContractSeriesV1 {
    root_symbol: String,
    contract_id: String,
    option_type: ta_app_core::options_types::OptionType,
    expiration_date: String,
    expiration_timestamp_ms: i64,
    strike: f64,
    multiplier: f64,
    #[serde(default)]
    rows: Vec<PersistedOptionsPriceRowV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedOptionsPriceRowV1 {
    timestamp_ms: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    underlying_price: f64,
    #[serde(default)]
    bid: Option<f64>,
    #[serde(default)]
    ask: Option<f64>,
    #[serde(default)]
    mid: Option<f64>,
    #[serde(default)]
    volume: Option<f64>,
    #[serde(default)]
    open_interest: Option<f64>,
    #[serde(default)]
    quote_age_ms: Option<i64>,
    #[serde(default)]
    session_id: Option<String>,
}

impl From<&ta_app_core::ImportedOptionsDataset> for PersistedImportedOptionsDatasetV1 {
    fn from(dataset: &ta_app_core::ImportedOptionsDataset) -> Self {
        Self {
            manifest: PersistedOptionsDatasetManifestV1::from(&dataset.manifest),
            contracts: dataset
                .contracts
                .iter()
                .map(PersistedOptionsContractSeriesV1::from)
                .collect(),
        }
    }
}

impl From<PersistedImportedOptionsDatasetV1> for ta_app_core::ImportedOptionsDataset {
    fn from(dataset: PersistedImportedOptionsDatasetV1) -> Self {
        Self {
            manifest: dataset.manifest.into(),
            contracts: dataset.contracts.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&ta_app_core::options_types::OptionsDatasetManifestV1>
    for PersistedOptionsDatasetManifestV1
{
    fn from(manifest: &ta_app_core::options_types::OptionsDatasetManifestV1) -> Self {
        Self {
            version: manifest.version,
            options_dataset_id: manifest.options_dataset_id.clone(),
            root_symbol: manifest.root_symbol.clone(),
            provider_name: manifest.provider_name.clone(),
            provider_source_surface: manifest.provider_source_surface.clone(),
            provider_entitlement_profile: manifest.provider_entitlement_profile.clone(),
            quote_resolution: manifest.quote_resolution.clone(),
            price_semantics: manifest.price_semantics,
            calendar_id: manifest.calendar_id.clone(),
            timezone: manifest.timezone.clone(),
            coverage_start: manifest.coverage_start,
            coverage_end: manifest.coverage_end,
            normalization_version: manifest.normalization_version,
            quote_semantics_version: manifest.quote_semantics_version,
            corporate_action_mapping_version: manifest.corporate_action_mapping_version,
            contract_reference_source: manifest.contract_reference_source.clone(),
            contract_reference_as_of_utc: manifest.contract_reference_as_of_utc.clone(),
            underlier_reference_source: manifest.underlier_reference_source.clone(),
            dataset_path: manifest.dataset_path.clone(),
            row_count: manifest.row_count,
            contract_count: manifest.contract_count,
            aggregate_price_field: manifest.aggregate_price_field.clone(),
            normalized_rows_sha256: manifest.normalized_rows_sha256.clone(),
            manifest_semantic_sha256: manifest.manifest_semantic_sha256.clone(),
        }
    }
}

impl From<PersistedOptionsDatasetManifestV1>
    for ta_app_core::options_types::OptionsDatasetManifestV1
{
    fn from(manifest: PersistedOptionsDatasetManifestV1) -> Self {
        Self {
            version: manifest.version,
            options_dataset_id: manifest.options_dataset_id,
            root_symbol: manifest.root_symbol,
            provider_name: manifest.provider_name,
            provider_source_surface: manifest.provider_source_surface,
            provider_entitlement_profile: manifest.provider_entitlement_profile,
            quote_resolution: manifest.quote_resolution,
            price_semantics: manifest.price_semantics,
            calendar_id: manifest.calendar_id,
            timezone: manifest.timezone,
            coverage_start: manifest.coverage_start,
            coverage_end: manifest.coverage_end,
            normalization_version: manifest.normalization_version,
            quote_semantics_version: manifest.quote_semantics_version,
            corporate_action_mapping_version: manifest.corporate_action_mapping_version,
            contract_reference_source: manifest.contract_reference_source,
            contract_reference_as_of_utc: manifest.contract_reference_as_of_utc,
            underlier_reference_source: manifest.underlier_reference_source,
            dataset_path: manifest.dataset_path,
            row_count: manifest.row_count,
            contract_count: manifest.contract_count,
            aggregate_price_field: manifest.aggregate_price_field,
            normalized_rows_sha256: manifest.normalized_rows_sha256,
            manifest_semantic_sha256: manifest.manifest_semantic_sha256,
        }
    }
}

impl From<&ta_app_core::options_types::OptionsContractSeries> for PersistedOptionsContractSeriesV1 {
    fn from(series: &ta_app_core::options_types::OptionsContractSeries) -> Self {
        Self {
            root_symbol: series.root_symbol.clone(),
            contract_id: series.contract_id.clone(),
            option_type: series.option_type,
            expiration_date: series.expiration_date.clone(),
            expiration_timestamp_ms: series.expiration_timestamp_ms,
            strike: series.strike,
            multiplier: series.multiplier,
            rows: series
                .rows
                .iter()
                .map(PersistedOptionsPriceRowV1::from)
                .collect(),
        }
    }
}

impl From<PersistedOptionsContractSeriesV1> for ta_app_core::options_types::OptionsContractSeries {
    fn from(series: PersistedOptionsContractSeriesV1) -> Self {
        Self {
            root_symbol: series.root_symbol,
            contract_id: series.contract_id,
            option_type: series.option_type,
            expiration_date: series.expiration_date,
            expiration_timestamp_ms: series.expiration_timestamp_ms,
            strike: series.strike,
            multiplier: series.multiplier,
            rows: series.rows.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&ta_app_core::options_types::OptionsPriceRow> for PersistedOptionsPriceRowV1 {
    fn from(row: &ta_app_core::options_types::OptionsPriceRow) -> Self {
        Self {
            timestamp_ms: row.timestamp_ms,
            open: row.open,
            high: row.high,
            low: row.low,
            close: row.close,
            underlying_price: row.underlying_price,
            bid: row.bid,
            ask: row.ask,
            mid: row.mid,
            volume: row.volume,
            open_interest: row.open_interest,
            quote_age_ms: row.quote_age_ms,
            session_id: row.session_id.clone(),
        }
    }
}

impl From<PersistedOptionsPriceRowV1> for ta_app_core::options_types::OptionsPriceRow {
    fn from(row: PersistedOptionsPriceRowV1) -> Self {
        Self {
            timestamp_ms: row.timestamp_ms,
            open: row.open,
            high: row.high,
            low: row.low,
            close: row.close,
            underlying_price: row.underlying_price,
            bid: row.bid,
            ask: row.ask,
            mid: row.mid,
            volume: row.volume,
            open_interest: row.open_interest,
            quote_age_ms: row.quote_age_ms,
            session_id: row.session_id,
        }
    }
}

fn parent_signal_cache() -> &'static Mutex<Vec<CachedParentSignalReplay>> {
    static CACHE: OnceLock<Mutex<Vec<CachedParentSignalReplay>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

fn options_dataset_cache() -> &'static Mutex<Vec<CachedOptionsDatasetReplay>> {
    static CACHE: OnceLock<Mutex<Vec<CachedOptionsDatasetReplay>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

fn non_empty_string(v: Option<&Value>) -> Option<String> {
    v.and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn serialize_scalar_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|entry| entry.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn extract_optional_serialized_field<T: serde::Serialize>(
    value: &T,
    field_name: &str,
) -> Option<Value> {
    serde_json::to_value(value)
        .ok()
        .and_then(|entry| {
            entry
                .as_object()
                .and_then(|object| object.get(field_name).cloned())
        })
        .filter(|entry| !entry.is_null())
}

fn resolve_options_parent_strategy_kind_from_saved_artifact_within_dir(
    artifact: &OptionsExpressionArtifactV1,
    allowed_dir: Option<&Path>,
) -> String {
    let parent_artifact_path = artifact.request.parent_artifact_path.trim();
    if parent_artifact_path.is_empty() {
        return String::new();
    }
    let parent_path = Path::new(parent_artifact_path);
    if !parent_path.is_file() {
        return String::new();
    }
    if let Some(dir) = allowed_dir {
        if ensure_path_is_within_dir(parent_path, dir).is_err() {
            return String::new();
        }
    }
    let parent_artifact = match read_artifact_value(parent_path) {
        Ok(value) => value,
        Err(_) => return String::new(),
    };
    if let Some(family) = supported_options_parent_family_from_artifact(&parent_artifact) {
        return family.strategy_kind().to_string();
    }
    non_empty_string(parent_artifact.pointer("/run/req/strategy_kind")).unwrap_or_default()
}

fn max_metric_or_default<I>(values: I) -> f64
where
    I: Iterator<Item = f64>,
{
    values
        .max_by(|left, right| left.total_cmp(right))
        .unwrap_or(0.0)
}

fn file_cache_stamp(path: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("failed to read metadata for {}: {e}", path.display()))?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis())
        .unwrap_or(0);
    Ok(format!(
        "{}|{}|{}",
        path.display(),
        metadata.len(),
        modified_ms
    ))
}

fn saturating_elapsed_us(start: std::time::Instant) -> u64 {
    start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

fn insert_parent_signal_cache_entry(entry: CachedParentSignalReplay) {
    let mut cache = parent_signal_cache()
        .lock()
        .expect("parent signal cache poisoned");
    cache.retain(|existing| existing.cache_key != entry.cache_key);
    cache.push(entry);
    if cache.len() > OPTIONS_REPLAY_CACHE_LIMIT {
        let overflow = cache.len() - OPTIONS_REPLAY_CACHE_LIMIT;
        cache.drain(0..overflow);
    }
}

fn load_parent_signal_cache_entry(
    cache_key: &str,
) -> Option<(String, Arc<ta_app_core::OptionsSignalEventManifestV1>)> {
    let mut cache = parent_signal_cache()
        .lock()
        .expect("parent signal cache poisoned");
    let position = cache
        .iter()
        .position(|entry| entry.cache_key == cache_key)?;
    let entry = cache.remove(position);
    let result = (
        entry.parent_artifact_id.clone(),
        Arc::clone(&entry.signal_manifest),
    );
    cache.push(entry);
    Some(result)
}

fn insert_options_dataset_cache_entry(entry: CachedOptionsDatasetReplay) {
    let mut cache = options_dataset_cache()
        .lock()
        .expect("options dataset cache poisoned");
    cache.retain(|existing| existing.cache_key != entry.cache_key);
    cache.push(entry);
    if cache.len() > OPTIONS_REPLAY_CACHE_LIMIT {
        let overflow = cache.len() - OPTIONS_REPLAY_CACHE_LIMIT;
        cache.drain(0..overflow);
    }
}

fn load_options_dataset_cache_entry(
    cache_key: &str,
) -> Option<(String, Arc<ta_app_core::ImportedOptionsDataset>)> {
    let mut cache = options_dataset_cache()
        .lock()
        .expect("options dataset cache poisoned");
    let position = cache
        .iter()
        .position(|entry| entry.cache_key == cache_key)?;
    let entry = cache.remove(position);
    let result = (
        entry.options_dataset_hash.clone(),
        Arc::clone(&entry.dataset),
    );
    cache.push(entry);
    Some(result)
}

fn resolve_run_artifact_kind(value: &Value) -> String {
    non_empty_string(value.get("kind")).unwrap_or_default()
}

fn resolve_csv_path(value: &Value) -> String {
    non_empty_string(value.get("csv_path"))
        .or_else(|| non_empty_string(value.pointer("/data/csv_path")))
        .or_else(|| non_empty_string(value.pointer("/dataset_meta/csv_path")))
        .or_else(|| non_empty_string(value.pointer("/validation/req/validation_context/csv_path")))
        .unwrap_or_default()
}

fn resolve_timeframe(value: &Value) -> String {
    non_empty_string(value.pointer("/dataset_meta/timeframe"))
        .or_else(|| non_empty_string(value.pointer("/validation/req/validation_context/timeframe")))
        .unwrap_or_default()
}

fn resolve_adjustment_mode(value: &Value) -> String {
    non_empty_string(value.pointer("/dataset_meta/adjustment_mode"))
        .unwrap_or_else(|| "raw".to_string())
}

fn normalize_symbol_root(raw: &str) -> String {
    raw.trim()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_uppercase()
}

fn derive_symbol_root(csv_path: &str) -> String {
    let stem = std::path::Path::new(csv_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("UNKNOWN");
    let upper = stem.to_ascii_uppercase();
    let stripped = [
        "_1M",
        "_5M",
        "_15M",
        "_30M",
        "_60M",
        "_1H",
        "_4H",
        "_1D",
        "_1W",
        "_DAILY",
        "_RAW",
        "_ADJUSTED",
    ]
    .iter()
    .find_map(|suffix| upper.strip_suffix(suffix))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or(upper.as_str());
    normalize_symbol_root(stripped)
}

fn resolve_symbol_root(value: &Value, csv_path: &str) -> String {
    [
        "/dataset_meta/display_symbol",
        "/dataset_meta/symbol",
        "/data/display_symbol",
        "/data/symbol",
        "/run/req/symbol",
    ]
    .iter()
    .find_map(|pointer| non_empty_string(value.pointer(pointer)))
    .map(|value| normalize_symbol_root(&value))
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| derive_symbol_root(csv_path))
}

fn build_source_dataset_identity_hash(
    csv_path: &std::path::Path,
    source_identity: data_cache::SourceIdentity,
) -> String {
    format!(
        "{}::{}::{}",
        csv_path.to_string_lossy().to_ascii_lowercase(),
        source_identity.size,
        source_identity.mtime_utc_ms
    )
}

fn build_dataset_hash(source_content_fingerprint: &str, timeframe: &str) -> String {
    format!("{source_content_fingerprint}::{timeframe}")
}

fn options_datasets_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    let dir = market_root.join("options_research").join("datasets");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn options_replay_cache_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    let dir = market_root.join("options_research").join("replay_cache");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn signal_manifest_replay_cache_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let dir = options_replay_cache_dir(app_local_data_dir)?.join("signal_manifests");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn options_dataset_replay_cache_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let dir = options_replay_cache_dir(app_local_data_dir)?.join("datasets");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn runs_dir(app_local_data_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let dir = app_local_data_dir.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn ensure_path_is_within_dir(path: &std::path::Path, dir: &std::path::Path) -> Result<(), String> {
    let canonical_dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let canonical_path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err(format!(
            "path {} is outside the allowed directory {}",
            path.display(),
            dir.display()
        ));
    }
    Ok(())
}

fn sanitize_dataset_id_component(raw: &str) -> String {
    let out: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    out.trim_matches('_').to_string()
}

fn sanitize_dataset_id(raw: &str) -> Result<String, String> {
    let sanitized = sanitize_dataset_id_component(raw);
    if sanitized.is_empty() {
        return Err("options_dataset_id resolved to an empty value".to_string());
    }
    Ok(sanitized)
}

fn cache_key_digest(cache_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    cache_key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn json_value_digest(value: &Value) -> Result<String, String> {
    let mut hasher = DefaultHasher::new();
    serde_json::to_vec(value)
        .map_err(|e| format!("failed to serialize json value for hashing: {e}"))?
        .hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn signal_manifest_replay_cache_path(
    app_local_data_dir: &std::path::Path,
    parent_artifact_id: &str,
    cache_key: &str,
) -> Result<std::path::PathBuf, String> {
    let dir = signal_manifest_replay_cache_dir(app_local_data_dir)?;
    let safe_parent = sanitize_dataset_id_component(parent_artifact_id);
    let stem = if safe_parent.is_empty() {
        "parent".to_string()
    } else {
        safe_parent
    };
    Ok(dir.join(format!(
        "vectorgrid_options_signal_manifest_{}_{}.json",
        stem,
        cache_key_digest(cache_key)
    )))
}

fn options_dataset_replay_cache_path(
    app_local_data_dir: &std::path::Path,
    dataset_id: &str,
    cache_key: &str,
) -> Result<std::path::PathBuf, String> {
    let dir = options_dataset_replay_cache_dir(app_local_data_dir)?;
    let safe_dataset = sanitize_dataset_id_component(dataset_id);
    let stem = if safe_dataset.is_empty() {
        "dataset".to_string()
    } else {
        safe_dataset
    };
    Ok(dir.join(format!(
        "vectorgrid_options_dataset_{}_{}.bin",
        stem,
        cache_key_digest(cache_key)
    )))
}

fn options_dataset_replay_cache_legacy_json_path(
    app_local_data_dir: &std::path::Path,
    dataset_id: &str,
    cache_key: &str,
) -> Result<std::path::PathBuf, String> {
    let dir = options_dataset_replay_cache_dir(app_local_data_dir)?;
    let safe_dataset = sanitize_dataset_id_component(dataset_id);
    let stem = if safe_dataset.is_empty() {
        "dataset".to_string()
    } else {
        safe_dataset
    };
    Ok(dir.join(format!(
        "vectorgrid_options_dataset_{}_{}.json",
        stem,
        cache_key_digest(cache_key)
    )))
}

fn load_signal_manifest_replay_cache(
    app_local_data_dir: &std::path::Path,
    parent_artifact_id: &str,
    cache_key: &str,
) -> Result<Option<ta_app_core::OptionsSignalEventManifestV1>, String> {
    let path =
        signal_manifest_replay_cache_path(app_local_data_dir, parent_artifact_id, cache_key)?;
    if !path.is_file() {
        return Ok(None);
    }
    let dir = signal_manifest_replay_cache_dir(app_local_data_dir)?;
    ensure_path_is_within_dir(&path, &dir)?;
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "failed to read cached signal manifest {}: {e}",
            path.display()
        )
    })?;
    match serde_json::from_slice::<ta_app_core::OptionsSignalEventManifestV1>(&bytes) {
        Ok(manifest) => Ok(Some(manifest)),
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            Ok(None)
        }
    }
}

fn persist_signal_manifest_replay_cache(
    app_local_data_dir: &std::path::Path,
    parent_artifact_id: &str,
    cache_key: &str,
    manifest: &ta_app_core::OptionsSignalEventManifestV1,
) -> Result<(), String> {
    let path =
        signal_manifest_replay_cache_path(app_local_data_dir, parent_artifact_id, cache_key)?;
    let dir = signal_manifest_replay_cache_dir(app_local_data_dir)?;
    if !path.starts_with(&dir) {
        return Err(format!(
            "cached signal manifest path {} is outside {}",
            path.display(),
            dir.display()
        ));
    }
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize cached signal manifest: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| {
        format!(
            "failed to write cached signal manifest {}: {e}",
            path.display()
        )
    })
}

fn load_options_dataset_replay_cache(
    app_local_data_dir: &std::path::Path,
    dataset_id: &str,
    cache_key: &str,
) -> Result<Option<(String, ta_app_core::ImportedOptionsDataset)>, String> {
    let path = options_dataset_replay_cache_path(app_local_data_dir, dataset_id, cache_key)?;
    let dir = options_dataset_replay_cache_dir(app_local_data_dir)?;
    let mut invalidate_binary_cache = false;

    if path.is_file() {
        ensure_path_is_within_dir(&path, &dir)?;
        let file = std::fs::File::open(&path).map_err(|e| {
            format!(
                "failed to open cached options dataset {}: {e}",
                path.display()
            )
        })?;
        let reader = BufReader::new(file);
        match bincode::deserialize_from::<_, PersistedOptionsDatasetReplayCacheV1>(reader) {
            Ok(cached) => {
                if cached.version != OPTIONS_DATASET_REPLAY_CACHE_VERSION {
                    invalidate_binary_cache = true;
                } else {
                    return Ok(Some((cached.options_dataset_hash, cached.dataset.into())));
                }
            }
            Err(_) => {
                invalidate_binary_cache = true;
            }
        }
    }

    let legacy_path =
        options_dataset_replay_cache_legacy_json_path(app_local_data_dir, dataset_id, cache_key)?;
    if !legacy_path.is_file() {
        if invalidate_binary_cache {
            let _ = std::fs::remove_file(&path);
        }
        return Ok(None);
    }
    ensure_path_is_within_dir(&legacy_path, &dir)?;
    let bytes = std::fs::read(&legacy_path).map_err(|e| {
        format!(
            "failed to read cached options dataset {}: {e}",
            legacy_path.display()
        )
    })?;
    let cached =
        serde_json::from_slice::<PersistedOptionsDatasetReplayCacheV1>(&bytes).map_err(|e| {
            format!(
                "failed to deserialize cached options dataset {}: {e}",
                legacy_path.display()
            )
        })?;
    if cached.version != OPTIONS_DATASET_REPLAY_CACHE_VERSION {
        return Ok(None);
    }
    let options_dataset_hash = cached.options_dataset_hash;
    let dataset: ta_app_core::ImportedOptionsDataset = cached.dataset.into();
    persist_options_dataset_replay_cache(
        app_local_data_dir,
        dataset_id,
        cache_key,
        &options_dataset_hash,
        &dataset,
    )?;
    Ok(Some((options_dataset_hash, dataset)))
}

fn persist_options_dataset_replay_cache(
    app_local_data_dir: &std::path::Path,
    dataset_id: &str,
    cache_key: &str,
    options_dataset_hash: &str,
    dataset: &ta_app_core::ImportedOptionsDataset,
) -> Result<(), String> {
    let path = options_dataset_replay_cache_path(app_local_data_dir, dataset_id, cache_key)?;
    let dir = options_dataset_replay_cache_dir(app_local_data_dir)?;
    if !path.starts_with(&dir) {
        return Err(format!(
            "cached options dataset path {} is outside {}",
            path.display(),
            dir.display()
        ));
    }
    let payload = PersistedOptionsDatasetReplayCacheV1 {
        version: OPTIONS_DATASET_REPLAY_CACHE_VERSION,
        options_dataset_hash: options_dataset_hash.to_string(),
        dataset: dataset.into(),
    };
    let file = std::fs::File::create(&path).map_err(|e| {
        format!(
            "failed to create cached options dataset {}: {e}",
            path.display()
        )
    })?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, &payload).map_err(|e| {
        format!(
            "failed to write cached options dataset {}: {e}",
            path.display()
        )
    })
}

fn build_import_request_from_manifest(
    manifest: &ta_app_core::OptionsDatasetManifestV1,
    dataset_path: &std::path::Path,
) -> ImportedOptionsDatasetCsvBuildRequest {
    ImportedOptionsDatasetCsvBuildRequest {
        options_dataset_id: manifest.options_dataset_id.clone(),
        dataset_path: dataset_path.to_string_lossy().to_string(),
        root_symbol: Some(manifest.root_symbol.clone()),
        provider_name: manifest.provider_name.clone(),
        provider_source_surface: manifest.provider_source_surface.clone(),
        provider_entitlement_profile: manifest.provider_entitlement_profile.clone(),
        quote_resolution: manifest.quote_resolution.clone(),
        price_semantics: manifest.price_semantics,
        calendar_id: manifest.calendar_id.clone(),
        timezone: manifest.timezone.clone(),
        normalization_version: manifest.normalization_version,
        quote_semantics_version: manifest.quote_semantics_version,
        corporate_action_mapping_version: manifest.corporate_action_mapping_version,
        contract_reference_source: manifest.contract_reference_source.clone(),
        contract_reference_as_of_utc: manifest.contract_reference_as_of_utc.clone(),
        underlier_reference_source: manifest.underlier_reference_source.clone(),
        aggregate_price_field: manifest.aggregate_price_field.clone(),
    }
}

fn dataset_rows_path_from_manifest_path(
    manifest_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let dataset_dir = manifest_path.parent().ok_or_else(|| {
        format!(
            "options dataset manifest {} is missing a parent directory",
            manifest_path.display()
        )
    })?;
    let rows_path = dataset_dir.join("rows.csv");
    if !rows_path.is_file() {
        return Err(format!(
            "options dataset rows file does not exist: {}",
            rows_path.display()
        ));
    }
    ensure_path_is_within_dir(&rows_path, dataset_dir)?;
    Ok(rows_path)
}

fn suggest_options_expression_artifact_path(
    app_local_data_dir: &std::path::Path,
    ts: &str,
) -> Result<std::path::PathBuf, String> {
    suggest_options_json_artifact_path_with_limit(
        app_local_data_dir,
        ts,
        "vectorgrid_options_expression",
        999,
    )
}

fn suggest_options_json_artifact_path_with_limit(
    app_local_data_dir: &std::path::Path,
    ts: &str,
    prefix: &str,
    max_suffix: u32,
) -> Result<std::path::PathBuf, String> {
    let runs_dir = runs_dir(app_local_data_dir)?;
    std::fs::create_dir_all(&runs_dir).map_err(|e| e.to_string())?;
    let safe_ts = super::sanitize_filename_component(ts);
    let base_name = format!("{prefix}_{safe_ts}");
    let path = runs_dir.join(format!("{base_name}.json"));
    if !path.exists() {
        return Ok(path);
    }
    for suffix in 2..=max_suffix {
        let candidate = runs_dir.join(format!("{base_name}_{suffix}.json"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "no available options artifact filename slots for {}",
        runs_dir.join(format!("{base_name}.json")).display()
    ))
}

fn suggest_options_expression_validation_artifact_path(
    app_local_data_dir: &std::path::Path,
    ts: &str,
) -> Result<std::path::PathBuf, String> {
    suggest_options_json_artifact_path_with_limit(
        app_local_data_dir,
        ts,
        "vectorgrid_options_validation",
        999,
    )
}

fn options_output_staging_path(parent: &Path, target: &Path) -> Result<PathBuf, String> {
    let file_name = target
        .file_name()
        .ok_or_else(|| format!("options output path has no file name: {}", target.display()))?;
    let counter = OPTIONS_OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut staged_name = OsString::from(".");
    staged_name.push(file_name);
    staged_name.push(format!(".{}.{}.{}.tmp", std::process::id(), nanos, counter));
    Ok(parent.join(staged_name))
}

fn existing_regular_options_output_target(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "refusing to replace non-regular options output target {}",
                    path.display()
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "failed to inspect options output target {}: {error}",
            path.display()
        )),
    }
}

#[cfg(windows)]
fn reserve_options_output_backup_path(
    final_path: &Path,
) -> Result<OptionsOutputStageGuard, String> {
    let parent = final_path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    for _ in 0..64 {
        let candidate = options_output_staging_path(parent, final_path)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                drop(file);
                fs::remove_file(&candidate).map_err(|error| {
                    format!(
                        "failed to prepare options output recovery backup {}: {error}",
                        candidate.display()
                    )
                })?;
                return Ok(OptionsOutputStageGuard::new(candidate));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to reserve options output recovery backup {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "failed to allocate a unique recovery backup for options output {}",
        final_path.display()
    ))
}

#[cfg(windows)]
fn restore_options_output_backup_after_replace_failure(
    backup: &mut OptionsOutputStageGuard,
    final_path: &Path,
    replace_error: &io::Error,
    unexpected_state: bool,
) -> String {
    match fs::rename(backup.path(), final_path) {
        Ok(()) => format!(
            "failed to atomically replace options output {}; restored the previous target after \
             {}Windows error {replace_error}",
            final_path.display(),
            if unexpected_state { "unexpected " } else { "" }
        ),
        Err(restore_error) => {
            let recovery_path = backup.path().to_path_buf();
            backup.preserve_for_recovery();
            format!(
                "failed to atomically replace options output {} ({replace_error}); failed to \
                 restore the previous target ({restore_error}); the previous target is preserved \
                 for recovery at {}",
                final_path.display(),
                recovery_path.display()
            )
        }
    }
}

#[cfg(windows)]
fn replace_existing_options_output(staged_path: &Path, final_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    // Supplying a backup path is important here. ReplaceFileW can otherwise report
    // ERROR_UNABLE_TO_MOVE_REPLACEMENT after it has already removed the original target.
    // With a backup, the documented partial-move state keeps the original at this path so
    // that we can restore it before returning an error.
    let mut backup = reserve_options_output_backup_path(final_path)?;
    let final_wide: Vec<u16> = final_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let staged_wide: Vec<u16> = staged_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let backup_wide: Vec<u16> = backup
        .path()
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        ReplaceFileW(
            final_wide.as_ptr(),
            staged_wide.as_ptr(),
            backup_wide.as_ptr(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        let replace_error = io::Error::last_os_error();

        // ERROR_UNABLE_TO_MOVE_REPLACEMENT_2 (1177) means Windows moved the old
        // target to the supplied backup but could not move the staged replacement.
        if replace_error.raw_os_error() == Some(1177) {
            return Err(restore_options_output_backup_after_replace_failure(
                &mut backup,
                final_path,
                &replace_error,
                false,
            ));
        }

        // With a backup name supplied, all other documented ReplaceFileW failures
        // retain the original target name. Verify that invariant before allowing
        // the staging guards to clean up.
        match existing_regular_options_output_target(final_path) {
            Ok(true) => {
                return Err(format!(
                    "failed to atomically replace options output {}: {replace_error}",
                    final_path.display()
                ));
            }
            Ok(false) => {}
            Err(target_error) => {
                backup.preserve_for_recovery();
                return Err(format!(
                    "failed to atomically replace options output {} ({replace_error}); \
                     could not verify the previous target ({target_error}); recovery backup: {}",
                    final_path.display(),
                    backup.path().display()
                ));
            }
        }

        // Defensive reconciliation for an undocumented partial state: restore the
        // previous target when the backup exists, or publish the already-synced
        // replacement if Windows consumed the original without creating a backup.
        if backup.path().is_file() {
            return Err(restore_options_output_backup_after_replace_failure(
                &mut backup,
                final_path,
                &replace_error,
                true,
            ));
        }
        if staged_path.is_file() {
            fs::rename(staged_path, final_path).map_err(|publish_error| {
                format!(
                    "failed to atomically replace options output {} ({replace_error}) and failed \
                     to publish the synchronized replacement ({publish_error})",
                    final_path.display()
                )
            })?;
            return Ok(());
        }

        return Err(format!(
            "failed to atomically replace options output {} ({replace_error}); neither the \
             previous target nor synchronized replacement could be located",
            final_path.display()
        ));
    }

    // ReplaceFileW leaves the old target at the backup name on success. It is no
    // longer needed once the synchronized replacement owns the final name.
    drop(backup);
    Ok(())
}

#[cfg(not(windows))]
fn replace_existing_options_output(staged_path: &Path, final_path: &Path) -> Result<(), String> {
    fs::rename(staged_path, final_path).map_err(|error| {
        format!(
            "failed to atomically replace options output {}: {error}",
            final_path.display()
        )
    })
}

fn publish_options_output(
    staged_path: &Path,
    final_path: &Path,
    target_exists: bool,
) -> Result<(), String> {
    if target_exists {
        replace_existing_options_output(staged_path, final_path)
    } else {
        fs::rename(staged_path, final_path).map_err(|error| {
            format!(
                "failed to atomically publish options output {}: {error}",
                final_path.display()
            )
        })
    }
}

#[cfg(not(windows))]
fn sync_options_output_parent_directory(parent: &Path) -> Result<(), String> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            format!(
                "failed to sync options output directory {} after publication: {error}",
                parent.display()
            )
        })
}

#[cfg(windows)]
fn sync_options_output_parent_directory(_parent: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn preserve_existing_options_output_permissions(
    staged_path: &Path,
    final_path: &Path,
    target_exists: bool,
) -> Result<(), String> {
    if !target_exists {
        return Ok(());
    }
    let permissions = fs::metadata(final_path)
        .map_err(|error| {
            format!(
                "failed to inspect existing options output permissions {}: {error}",
                final_path.display()
            )
        })?
        .permissions();
    fs::set_permissions(staged_path, permissions).map_err(|error| {
        format!(
            "failed to preserve options output permissions for {}: {error}",
            final_path.display()
        )
    })?;
    File::open(staged_path)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            format!(
                "failed to sync staged options output metadata {}: {error}",
                staged_path.display()
            )
        })
}

#[cfg(not(unix))]
fn preserve_existing_options_output_permissions(
    _staged_path: &Path,
    _final_path: &Path,
    _target_exists: bool,
) -> Result<(), String> {
    Ok(())
}

fn write_options_output_with_publish(
    path: &Path,
    context: &str,
    write_contents: impl FnOnce(&mut BufWriter<File>) -> Result<(), String>,
    publish: impl FnOnce(&Path, &Path, bool) -> Result<(), String>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create {context} directory {}: {error}",
            parent.display()
        )
    })?;
    let target_exists = existing_regular_options_output_target(path)?;

    let (staged_path, staged_file) = {
        let mut staged = None;
        for _ in 0..64 {
            let candidate = options_output_staging_path(parent, path)?;
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => {
                    staged = Some((candidate, file));
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "failed to create staged {context} {}: {error}",
                        candidate.display()
                    ));
                }
            }
        }
        staged.ok_or_else(|| {
            format!(
                "failed to allocate a unique staged {context} for {}",
                path.display()
            )
        })?
    };

    let mut staged = OptionsOutputStageGuard::new(staged_path);
    let mut writer = BufWriter::new(staged_file);
    let stage_result = (|| -> Result<(), String> {
        write_contents(&mut writer)?;
        writer.flush().map_err(|error| {
            format!(
                "failed to flush staged {context} {}: {error}",
                staged.path().display()
            )
        })?;
        writer.get_ref().sync_all().map_err(|error| {
            format!(
                "failed to sync staged {context} {}: {error}",
                staged.path().display()
            )
        })
    })();
    drop(writer);
    stage_result?;

    preserve_existing_options_output_permissions(staged.path(), path, target_exists)?;
    publish(staged.path(), path, target_exists)?;
    staged.mark_published();
    if let Err(error) = sync_options_output_parent_directory(parent) {
        log::warn!(
            "options output was published successfully but its parent directory sync failed: {}",
            error
        );
    }
    Ok(())
}

fn write_options_output_bytes_atomically(
    path: &Path,
    bytes: &[u8],
    context: &str,
) -> Result<(), String> {
    write_options_output_with_publish(
        path,
        context,
        |writer| {
            writer.write_all(bytes).map_err(|error| {
                format!(
                    "failed to write staged {context} for {}: {error}",
                    path.display()
                )
            })
        },
        publish_options_output,
    )
}

fn read_artifact_value(path: &std::path::Path) -> Result<Value, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read saved artifact {}: {e}", path.display()))?;
    serde_json::from_slice::<Value>(&bytes)
        .map_err(|e| format!("failed to parse saved artifact {}: {e}", path.display()))
}

fn load_replayed_candles(
    app_local_data_dir: &std::path::Path,
    artifact: &Value,
    csv_path: &str,
    timeframe: &str,
) -> Result<
    (
        vector_ta::utilities::data_loader::Candles,
        data_cache::SourceIdentity,
        String,
    ),
    String,
> {
    let csv_pathbuf = std::path::Path::new(csv_path);
    let source_identity = data_cache::identity_for_source(csv_pathbuf)?;
    let source_content_fingerprint = data_cache::content_fingerprint_for_source(csv_pathbuf)?;
    let cache_dir = data_cache::app_data_cache_dir(app_local_data_dir);
    let mut candles =
        data_cache::load_or_build_with_identity(csv_pathbuf, &cache_dir, source_identity)?.candles;

    let resample_applied = artifact
        .pointer("/dataset_meta/resample_applied")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let base_timeframe = non_empty_string(artifact.pointer("/dataset_meta/base_timeframe"));
    let derived_timeframe = non_empty_string(artifact.pointer("/dataset_meta/derived_timeframe"))
        .or_else(|| non_empty_string(artifact.pointer("/dataset_meta/timeframe")));

    if !timeframe.trim().is_empty()
        && (resample_applied || base_timeframe.as_deref() != Some(timeframe))
    {
        let spec = parse_timeframe_label(timeframe)?;
        candles = resample_candles(&candles, spec.interval_ms)?;
    } else if let (Some(base), Some(derived)) =
        (base_timeframe.as_deref(), derived_timeframe.as_deref())
    {
        if base != derived {
            let spec = parse_timeframe_label(derived)?;
            candles = resample_candles(&candles, spec.interval_ms)?;
        }
    }

    Ok((candles, source_identity, source_content_fingerprint))
}

fn build_options_expression_artifact(
    artifact_path: &std::path::Path,
    parent_artifact_id: String,
    parent_artifact_hash: String,
    options_dataset_hash: String,
    signal_manifest_id: String,
    dataset_provenance: Option<OptionsExpressionDatasetProvenance>,
    request: OptionsExpressionRunRequest,
    result: &OptionsExpressionRunResult,
) -> OptionsExpressionArtifactV1 {
    let options_dataset_normalized_rows_sha256 = dataset_provenance
        .as_ref()
        .and_then(|provenance| provenance.normalized_rows_sha256.clone());
    let options_dataset_manifest_semantic_sha256 = dataset_provenance
        .as_ref()
        .and_then(|provenance| provenance.manifest_semantic_sha256.clone());
    OptionsExpressionArtifactV1 {
        version: OPTIONS_EXPRESSION_ARTIFACT_VERSION,
        kind: "vectorgrid_options_expression_v1".to_string(),
        ts: chrono::Utc::now().to_rfc3339(),
        artifact_path: artifact_path.to_string_lossy().to_string(),
        parent_artifact_id,
        parent_artifact_hash,
        root_symbol: result.diagnostics.root_symbol.clone(),
        options_dataset_id: result.diagnostics.options_dataset_id.clone(),
        options_dataset_hash,
        options_dataset_normalized_rows_sha256,
        options_dataset_manifest_semantic_sha256,
        signal_manifest_id,
        dataset_provenance,
        backend_used: result.backend_used.clone(),
        backend_fallback_reason: result.backend_fallback_reason.clone(),
        price_semantics: result.price_semantics,
        exactness_semantics: Some(result.exactness_semantics),
        request,
        diagnostics: result.diagnostics.clone(),
        staging: result.staging.clone(),
        overlap_window: result.overlap_window.clone(),
        coverage_adequacy: result.coverage_adequacy.clone(),
        split_invalid_reasons: result.split_invalid_reasons.clone(),
        selection_metrics: result.ranked_results.clone(),
    }
}

const OPTIONS_QUOTE_QUALITY_STALE_THRESHOLD_MS: i64 = 15 * 60 * 1_000;

#[derive(Default)]
struct QuoteQualityCounts {
    row_count: usize,
    any_quote_row_count: usize,
    bid_row_count: usize,
    ask_row_count: usize,
    bid_ask_row_count: usize,
    mid_row_count: usize,
    quote_age_row_count: usize,
    stale_quote_row_count: usize,
    zero_bid_row_count: usize,
    locked_bid_ask_row_count: usize,
    crossed_bid_ask_row_count: usize,
    invalid_bid_ask_row_count: usize,
    max_quote_age_ms: Option<i64>,
    spread_bps_values: Vec<f64>,
}

fn quote_quality_ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn quote_row_spread_bps(row: &ta_app_core::OptionsPriceRow) -> Option<f64> {
    let bid = row.bid?;
    let ask = row.ask?;
    if !bid.is_finite() || !ask.is_finite() || bid < 0.0 || ask < bid {
        return None;
    }
    let midpoint = (bid + ask) * 0.5;
    if midpoint <= 0.0 || !midpoint.is_finite() {
        return None;
    }
    Some((ask - bid) / midpoint * 10_000.0)
}

fn accumulate_quote_quality_row(
    counts: &mut QuoteQualityCounts,
    row: &ta_app_core::OptionsPriceRow,
    stale_threshold_ms: i64,
) {
    counts.row_count = counts.row_count.saturating_add(1);
    let has_bid = row.bid.is_some();
    let has_ask = row.ask.is_some();
    let has_mid = row.mid.is_some();
    if has_bid || has_ask || has_mid {
        counts.any_quote_row_count = counts.any_quote_row_count.saturating_add(1);
    }
    if has_bid {
        counts.bid_row_count = counts.bid_row_count.saturating_add(1);
    }
    if has_ask {
        counts.ask_row_count = counts.ask_row_count.saturating_add(1);
    }
    if has_bid && has_ask {
        counts.bid_ask_row_count = counts.bid_ask_row_count.saturating_add(1);
    }
    if has_mid {
        counts.mid_row_count = counts.mid_row_count.saturating_add(1);
    }
    if let Some(bid) = row.bid {
        if bid.is_finite() && bid == 0.0 {
            counts.zero_bid_row_count = counts.zero_bid_row_count.saturating_add(1);
        }
    }
    if let (Some(bid), Some(ask)) = (row.bid, row.ask) {
        if !bid.is_finite() || !ask.is_finite() || bid < 0.0 || ask < 0.0 {
            counts.invalid_bid_ask_row_count = counts.invalid_bid_ask_row_count.saturating_add(1);
        } else if bid > ask {
            counts.crossed_bid_ask_row_count = counts.crossed_bid_ask_row_count.saturating_add(1);
            counts.invalid_bid_ask_row_count = counts.invalid_bid_ask_row_count.saturating_add(1);
        } else if bid == ask {
            counts.locked_bid_ask_row_count = counts.locked_bid_ask_row_count.saturating_add(1);
        }
    }
    if let Some(age_ms) = row.quote_age_ms {
        counts.quote_age_row_count = counts.quote_age_row_count.saturating_add(1);
        counts.max_quote_age_ms = Some(counts.max_quote_age_ms.unwrap_or(age_ms).max(age_ms));
        if age_ms > stale_threshold_ms {
            counts.stale_quote_row_count = counts.stale_quote_row_count.saturating_add(1);
        }
    }
    if let Some(spread_bps) = quote_row_spread_bps(row) {
        counts.spread_bps_values.push(spread_bps);
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) * 0.5)
    } else {
        Some(values[mid])
    }
}

fn max_value(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .reduce(f64::max)
}

fn quote_quality_warnings(counts: &QuoteQualityCounts) -> Vec<String> {
    let mut warnings = Vec::new();
    if counts.invalid_bid_ask_row_count > 0 {
        warnings.push(
            "Invalid bid/ask rows detected; treat quote-aware replay as advisory until provider cleanup is confirmed."
                .to_string(),
        );
    }
    if counts.crossed_bid_ask_row_count > 0 {
        warnings.push(
            "Crossed bid/ask rows detected; provider cleanup or row exclusion is recommended before promotion."
                .to_string(),
        );
    }
    if counts.locked_bid_ask_row_count > 0 {
        warnings.push(
            "Locked bid/ask rows detected; spreads may understate execution cost.".to_string(),
        );
    }
    if counts.zero_bid_row_count > 0 {
        warnings.push(
            "Zero-bid rows detected; liquidity and admission filters should be reviewed before promotion."
                .to_string(),
        );
    }
    warnings
}

fn contract_quote_quality_summary(
    contract_id: &str,
    counts: QuoteQualityCounts,
) -> OptionsExpressionContractQuoteQualitySummary {
    let warnings = quote_quality_warnings(&counts);
    OptionsExpressionContractQuoteQualitySummary {
        contract_id: contract_id.to_string(),
        row_count: counts.row_count,
        any_quote_row_count: counts.any_quote_row_count,
        bid_ask_row_count: counts.bid_ask_row_count,
        mid_row_count: counts.mid_row_count,
        quote_age_row_count: counts.quote_age_row_count,
        stale_quote_row_count: counts.stale_quote_row_count,
        zero_bid_row_count: counts.zero_bid_row_count,
        locked_bid_ask_row_count: counts.locked_bid_ask_row_count,
        crossed_bid_ask_row_count: counts.crossed_bid_ask_row_count,
        invalid_bid_ask_row_count: counts.invalid_bid_ask_row_count,
        any_quote_coverage_rate: quote_quality_ratio(counts.any_quote_row_count, counts.row_count),
        bid_ask_coverage_rate: quote_quality_ratio(counts.bid_ask_row_count, counts.row_count),
        mid_coverage_rate: quote_quality_ratio(counts.mid_row_count, counts.row_count),
        stale_quote_rate: quote_quality_ratio(counts.stale_quote_row_count, counts.row_count),
        mean_spread_bps: mean(&counts.spread_bps_values),
        max_spread_bps: max_value(&counts.spread_bps_values),
        max_quote_age_ms: counts.max_quote_age_ms,
        warnings,
    }
}

fn dataset_quote_quality_from_imported_dataset(
    dataset: &ta_app_core::ImportedOptionsDataset,
) -> Option<OptionsExpressionDatasetQuoteQualitySummary> {
    let stale_threshold_ms = OPTIONS_QUOTE_QUALITY_STALE_THRESHOLD_MS;
    let mut global = QuoteQualityCounts::default();
    let mut per_contract = Vec::with_capacity(dataset.contracts.len());

    for contract in &dataset.contracts {
        let mut contract_counts = QuoteQualityCounts::default();
        for row in &contract.rows {
            accumulate_quote_quality_row(&mut contract_counts, row, stale_threshold_ms);
            accumulate_quote_quality_row(&mut global, row, stale_threshold_ms);
        }
        per_contract.push(contract_quote_quality_summary(
            &contract.contract_id,
            contract_counts,
        ));
    }

    let should_include = !matches!(
        dataset.manifest.price_semantics,
        OptionsPriceSemantics::MinuteAggregate
    ) || global.any_quote_row_count > 0
        || global.quote_age_row_count > 0;
    if !should_include {
        return None;
    }

    let mean_spread_bps = mean(&global.spread_bps_values);
    let max_spread_bps = max_value(&global.spread_bps_values);
    let median_spread_bps = median(&mut global.spread_bps_values);

    Some(OptionsExpressionDatasetQuoteQualitySummary {
        price_semantics: dataset.manifest.price_semantics,
        row_count: global.row_count,
        contract_count: dataset.contracts.len(),
        any_quote_row_count: global.any_quote_row_count,
        bid_row_count: global.bid_row_count,
        ask_row_count: global.ask_row_count,
        bid_ask_row_count: global.bid_ask_row_count,
        mid_row_count: global.mid_row_count,
        quote_age_row_count: global.quote_age_row_count,
        stale_quote_threshold_ms: Some(stale_threshold_ms),
        stale_quote_row_count: global.stale_quote_row_count,
        zero_bid_row_count: global.zero_bid_row_count,
        locked_bid_ask_row_count: global.locked_bid_ask_row_count,
        crossed_bid_ask_row_count: global.crossed_bid_ask_row_count,
        invalid_bid_ask_row_count: global.invalid_bid_ask_row_count,
        any_quote_coverage_rate: quote_quality_ratio(global.any_quote_row_count, global.row_count),
        bid_ask_coverage_rate: quote_quality_ratio(global.bid_ask_row_count, global.row_count),
        mid_coverage_rate: quote_quality_ratio(global.mid_row_count, global.row_count),
        stale_quote_rate: quote_quality_ratio(global.stale_quote_row_count, global.row_count),
        mean_spread_bps,
        median_spread_bps,
        max_spread_bps,
        max_quote_age_ms: global.max_quote_age_ms,
        per_contract,
        warnings: quote_quality_warnings(&global),
    })
}

fn dataset_provenance_from_imported_dataset(
    dataset: &ta_app_core::ImportedOptionsDataset,
) -> OptionsExpressionDatasetProvenance {
    OptionsExpressionDatasetProvenance {
        provider_name: dataset.manifest.provider_name.trim().to_string(),
        provider_source_surface: dataset.manifest.provider_source_surface.trim().to_string(),
        quote_resolution: dataset.manifest.quote_resolution.trim().to_string(),
        aggregate_price_field: dataset
            .manifest
            .aggregate_price_field
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        contract_reference_as_of_utc: dataset
            .manifest
            .contract_reference_as_of_utc
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        normalized_rows_sha256: dataset
            .manifest
            .normalized_rows_sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        manifest_semantic_sha256: dataset
            .manifest
            .manifest_semantic_sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        quote_quality: dataset_quote_quality_from_imported_dataset(dataset),
    }
}

fn canonical_options_provider_identity(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "massive" | "polygon" | "polygon.io" | "polygon_io" | "polygonio" => {
            "polygon.io".to_string()
        }
        other => other.to_string(),
    }
}

fn best_tuple_index_by_metric(
    tuples: &[ta_app_core::options_types::OptionsExpressionTupleResult],
    metric: impl Fn(&ta_app_core::options_types::OptionsExpressionTupleResult) -> Option<f64>,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (index, entry) in tuples.iter().enumerate() {
        let Some(value) = metric(entry) else {
            continue;
        };
        if !value.is_finite() {
            continue;
        }
        match best {
            None => best = Some((index, value)),
            Some((best_index, best_value)) => {
                if value > best_value
                    || (value.total_cmp(&best_value).is_eq() && index < best_index)
                {
                    best = Some((index, value));
                }
            }
        }
    }
    best.map(|(index, _)| index)
}

fn supported_metric_expectancy(metrics: Option<&OptionsExpressionMetrics>) -> Option<f64> {
    metrics
        .filter(|metrics| metrics.has_supported_trades())
        .map(|metrics| metrics.expectancy_per_premium)
}

fn refresh_options_expression_artifact_validation_verdicts(
    artifact: &mut OptionsExpressionArtifactV1,
) {
    for entry in &mut artifact.selection_metrics {
        entry.validation_verdict = entry.inferred_validation_verdict();
    }
}

fn build_artifact_leader_summary(
    tuples: &[ta_app_core::options_types::OptionsExpressionTupleResult],
) -> OptionsExpressionArtifactLeaderSummary {
    let search_leader_tuple_index = best_tuple_index_by_metric(tuples, |entry| {
        Some(entry.selection_metrics.expectancy_per_premium)
    });
    let robust_leader_tuple_index =
        best_tuple_index_by_metric(tuples, |entry| Some(entry.robust_score));
    let holdout_leader_tuple_index = best_tuple_index_by_metric(tuples, |entry| {
        entry
            .supported_holdout_metrics()
            .map(|metrics| metrics.expectancy_per_premium)
    });
    let sealed_leader_tuple_index = best_tuple_index_by_metric(tuples, |entry| {
        entry
            .supported_sealed_evaluation_metrics()
            .map(|metrics| metrics.expectancy_per_premium)
    });
    let stress_leader_tuple_index = best_tuple_index_by_metric(tuples, |entry| {
        entry
            .strongest_validation_stress_metrics()
            .map(|metrics| metrics.expectancy_per_premium)
    });
    let distinct_leader_count = [
        search_leader_tuple_index,
        robust_leader_tuple_index,
        holdout_leader_tuple_index,
        sealed_leader_tuple_index,
        stress_leader_tuple_index,
    ]
    .into_iter()
    .flatten()
    .collect::<BTreeSet<_>>()
    .len();
    let robust_leader = robust_leader_tuple_index.and_then(|index| tuples.get(index));
    OptionsExpressionArtifactLeaderSummary {
        search_leader_tuple_index,
        robust_leader_tuple_index,
        holdout_leader_tuple_index,
        sealed_leader_tuple_index,
        stress_leader_tuple_index,
        distinct_leader_count,
        search_vs_robust_disagree: matches!(
            (search_leader_tuple_index, robust_leader_tuple_index),
            (Some(search), Some(robust)) if search != robust
        ),
        robust_vs_holdout_disagree: match (robust_leader_tuple_index, holdout_leader_tuple_index) {
            (Some(robust), Some(holdout)) => Some(robust != holdout),
            _ => None,
        },
        robust_vs_sealed_disagree: match (robust_leader_tuple_index, sealed_leader_tuple_index) {
            (Some(robust), Some(sealed)) => Some(robust != sealed),
            _ => None,
        },
        robust_leader_reached_sealed_finalist: robust_leader.map(|entry| entry.sealed_finalist),
        robust_leader_sealed_final_rank: robust_leader
            .filter(|entry| entry.sealed_finalist)
            .map(|entry| entry.sealed_final_rank),
    }
}

pub(crate) fn summarize_options_expression_artifact(
    path: &std::path::Path,
    bytes: u64,
    artifact: &OptionsExpressionArtifactV1,
) -> OptionsExpressionArtifactSummary {
    summarize_options_expression_artifact_with_runs_dir(path, bytes, artifact, None)
}

fn summarize_options_expression_artifact_with_runs_dir(
    path: &std::path::Path,
    bytes: u64,
    artifact: &OptionsExpressionArtifactV1,
    runs_dir: Option<&std::path::Path>,
) -> OptionsExpressionArtifactSummary {
    let parent_strategy_kind =
        resolve_options_parent_strategy_kind_from_saved_artifact_within_dir(artifact, runs_dir);
    let leader_summary = build_artifact_leader_summary(&artifact.selection_metrics);
    let best_robust_score = max_metric_or_default(
        artifact
            .selection_metrics
            .iter()
            .map(|entry| entry.robust_score),
    );
    let best_selection_expectancy_per_premium = max_metric_or_default(
        artifact
            .selection_metrics
            .iter()
            .map(|entry| entry.selection_metrics.expectancy_per_premium),
    );
    let best_holdout_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| supported_metric_expectancy(entry.holdout_metrics.as_ref()))
        .max_by(|left, right| left.total_cmp(right));
    let best_sealed_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| supported_metric_expectancy(entry.sealed_evaluation_metrics.as_ref()))
        .max_by(|left, right| left.total_cmp(right));
    let best_stress_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| supported_metric_expectancy(entry.stress_metrics.as_ref()))
        .max_by(|left, right| left.total_cmp(right));
    let best_sealed_stress_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| supported_metric_expectancy(entry.sealed_stress_metrics.as_ref()))
        .max_by(|left, right| left.total_cmp(right));
    let best_harsher_stress_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| supported_metric_expectancy(entry.harsher_stress_metrics.as_ref()))
        .max_by(|left, right| left.total_cmp(right));
    let best_sealed_harsher_stress_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| {
            supported_metric_expectancy(entry.sealed_harsher_stress_metrics.as_ref())
        })
        .max_by(|left, right| left.total_cmp(right));
    let best_extreme_stress_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| supported_metric_expectancy(entry.extreme_stress_metrics.as_ref()))
        .max_by(|left, right| left.total_cmp(right));
    let best_sealed_extreme_stress_expectancy_per_premium = artifact
        .selection_metrics
        .iter()
        .filter_map(|entry| {
            supported_metric_expectancy(entry.sealed_extreme_stress_metrics.as_ref())
        })
        .max_by(|left, right| left.total_cmp(right));
    let sealed_evaluated_tuple_count = artifact
        .selection_metrics
        .iter()
        .filter(|entry| entry.sealed_finalist || entry.sealed_evaluation_metrics.is_some())
        .count();
    let supported_holdout_tuple_count = artifact
        .selection_metrics
        .iter()
        .filter(|entry| entry.supported_holdout_metrics().is_some())
        .count();
    let supported_sealed_tuple_count = artifact
        .selection_metrics
        .iter()
        .filter(|entry| entry.supported_sealed_evaluation_metrics().is_some())
        .count();
    let supported_stress_tuple_count = artifact
        .selection_metrics
        .iter()
        .filter(|entry| entry.strongest_validation_stress_metrics().is_some())
        .count();
    let top_validation_verdict = artifact
        .selection_metrics
        .first()
        .map(|entry| entry.inferred_validation_verdict())
        .unwrap_or_default();
    let validated_tuple_count = artifact
        .selection_metrics
        .iter()
        .filter(|entry| {
            let verdict = entry.inferred_validation_verdict();
            !matches!(verdict, OptionsExpressionValidationVerdict::SearchOnly)
        })
        .count();
    let stress_fragile_tuple_count = artifact
        .selection_metrics
        .iter()
        .filter(|entry| {
            let verdict = entry.inferred_validation_verdict();
            matches!(verdict, OptionsExpressionValidationVerdict::StressFragile)
        })
        .count();
    let has_holdout_metrics = artifact
        .selection_metrics
        .iter()
        .any(|entry| entry.holdout_metrics.is_some());
    let has_sealed_evaluation_metrics = artifact
        .selection_metrics
        .iter()
        .any(|entry| entry.sealed_evaluation_metrics.is_some());
    let has_stress_metrics = artifact
        .selection_metrics
        .iter()
        .any(|entry| entry.stress_metrics.is_some() || entry.sealed_stress_metrics.is_some());
    let has_harsher_stress_metrics = artifact.selection_metrics.iter().any(|entry| {
        entry.harsher_stress_metrics.is_some() || entry.sealed_harsher_stress_metrics.is_some()
    });
    let has_extreme_stress_metrics = artifact.selection_metrics.iter().any(|entry| {
        entry.extreme_stress_metrics.is_some() || entry.sealed_extreme_stress_metrics.is_some()
    });
    OptionsExpressionArtifactSummary {
        ts: artifact.ts.clone(),
        path: path.to_string_lossy().to_string(),
        bytes,
        kind: artifact.kind.clone(),
        parent_artifact_id: artifact.parent_artifact_id.clone(),
        parent_strategy_kind,
        root_symbol: artifact.root_symbol.clone(),
        options_dataset_id: artifact.options_dataset_id.clone(),
        backend_used: artifact.backend_used.clone(),
        backend_fallback_reason: artifact.backend_fallback_reason.clone(),
        price_semantics: artifact.price_semantics,
        request_backend_preference: serialize_scalar_to_string(
            &artifact.request.backend_preference,
        ),
        request_objective_profile: artifact
            .request
            .objective_config
            .profile
            .as_str()
            .to_string(),
        request_fill_model: serialize_scalar_to_string(&artifact.request.fill_model),
        request_structure_family: serialize_scalar_to_string(&artifact.request.structure_family),
        request_split_assignment_mode: serialize_scalar_to_string(
            &artifact.request.split_assignment_mode,
        ),
        coverage_adequacy_verdict: artifact.coverage_adequacy.verdict,
        coverage_non_promotable: artifact.coverage_adequacy.non_promotable,
        search_overlap_rate: artifact.overlap_window.search_overlap_rate,
        holdout_overlap_rate: artifact.overlap_window.holdout_overlap_rate,
        sealed_test_overlap_rate: artifact.overlap_window.sealed_test_overlap_rate,
        search_candidate_coverage_rate: artifact.coverage_adequacy.search_candidate_coverage_rate,
        holdout_candidate_coverage_rate: artifact.coverage_adequacy.holdout_candidate_coverage_rate,
        sealed_test_candidate_coverage_rate: artifact
            .coverage_adequacy
            .sealed_test_candidate_coverage_rate,
        search_baseline_outcome_coverage_rate: artifact
            .coverage_adequacy
            .search_baseline_outcome_coverage_rate,
        holdout_baseline_outcome_coverage_rate: artifact
            .coverage_adequacy
            .holdout_baseline_outcome_coverage_rate,
        sealed_test_baseline_outcome_coverage_rate: artifact
            .coverage_adequacy
            .sealed_test_baseline_outcome_coverage_rate,
        tuple_count: artifact.selection_metrics.len(),
        best_robust_score,
        best_selection_expectancy_per_premium,
        best_holdout_expectancy_per_premium,
        best_sealed_expectancy_per_premium,
        best_stress_expectancy_per_premium,
        best_sealed_stress_expectancy_per_premium,
        best_harsher_stress_expectancy_per_premium,
        best_sealed_harsher_stress_expectancy_per_premium,
        best_extreme_stress_expectancy_per_premium,
        best_sealed_extreme_stress_expectancy_per_premium,
        leader_summary,
        sealed_finalist_budget: artifact.request.sealed_finalist_count,
        sealed_evaluated_tuple_count,
        top_validation_verdict,
        validated_tuple_count,
        stress_fragile_tuple_count,
        supported_holdout_tuple_count,
        supported_sealed_tuple_count,
        supported_stress_tuple_count,
        has_holdout_metrics,
        has_sealed_evaluation_metrics,
        has_stress_metrics,
        has_harsher_stress_metrics,
        has_extreme_stress_metrics,
        exactness_semantics: extract_optional_serialized_field(artifact, "exactness_semantics"),
    }
}

#[derive(Clone)]
struct OptionsExpressionReplayInputs {
    parent_artifact_id: String,
    parent_artifact_hash: String,
    normalized_req: OptionsExpressionRunRequest,
    signal_manifest: Arc<ta_app_core::OptionsSignalEventManifestV1>,
    dataset: Arc<ta_app_core::ImportedOptionsDataset>,
    options_dataset_hash: String,
}

fn non_empty_hash(value: Option<&String>) -> Option<&str> {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn saved_artifact_normalized_rows_sha256(artifact: &OptionsExpressionArtifactV1) -> Option<&str> {
    non_empty_hash(artifact.options_dataset_normalized_rows_sha256.as_ref()).or_else(|| {
        artifact
            .dataset_provenance
            .as_ref()
            .and_then(|provenance| non_empty_hash(provenance.normalized_rows_sha256.as_ref()))
    })
}

fn saved_artifact_manifest_semantic_sha256(artifact: &OptionsExpressionArtifactV1) -> Option<&str> {
    non_empty_hash(artifact.options_dataset_manifest_semantic_sha256.as_ref()).or_else(|| {
        artifact
            .dataset_provenance
            .as_ref()
            .and_then(|provenance| non_empty_hash(provenance.manifest_semantic_sha256.as_ref()))
    })
}

fn compare_exact_replay_optional_hash(
    label: &str,
    saved: Option<&str>,
    current: Option<&str>,
    dataset_id: &str,
) -> Result<(), String> {
    if let (Some(saved), Some(current)) = (saved, current) {
        if saved != current {
            return Err(format!(
                "saved artifact {label} '{}' no longer matches the currently imported dataset {label} '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
                saved, current, dataset_id
            ));
        }
    }
    Ok(())
}

fn optional_replay_identity_hash_drift(
    artifact: &OptionsExpressionArtifactV1,
    replay: &OptionsExpressionReplayInputs,
) -> bool {
    matches!(
        (
            saved_artifact_normalized_rows_sha256(artifact),
            non_empty_hash(replay.dataset.manifest.normalized_rows_sha256.as_ref())
        ),
        (Some(saved), Some(current)) if saved != current
    ) || matches!(
        (
            saved_artifact_manifest_semantic_sha256(artifact),
            non_empty_hash(replay.dataset.manifest.manifest_semantic_sha256.as_ref())
        ),
        (Some(saved), Some(current)) if saved != current
    )
}

fn validate_saved_artifact_replay_provenance(
    artifact: &OptionsExpressionArtifactV1,
    replay: &OptionsExpressionReplayInputs,
) -> Result<(), String> {
    if replay.parent_artifact_id != artifact.parent_artifact_id {
        return Err(format!(
            "saved artifact parent id '{}' no longer matches the replay parent id '{}'",
            artifact.parent_artifact_id, replay.parent_artifact_id
        ));
    }
    if !artifact.parent_artifact_hash.trim().is_empty()
        && replay.parent_artifact_hash != artifact.parent_artifact_hash
    {
        return Err(format!(
            "saved artifact parent hash '{}' no longer matches the replay parent hash '{}' for parent '{}'; reopen the original parent artifact or rerun the options expression optimization",
            artifact.parent_artifact_hash, replay.parent_artifact_hash, artifact.parent_artifact_id
        ));
    }
    if replay.normalized_req.options_dataset_id != artifact.options_dataset_id {
        return Err(format!(
            "saved artifact dataset id '{}' no longer matches the replay dataset id '{}'",
            artifact.options_dataset_id, replay.normalized_req.options_dataset_id
        ));
    }
    if !artifact.signal_manifest_id.trim().is_empty()
        && replay.signal_manifest.manifest_id != artifact.signal_manifest_id
    {
        return Err(format!(
            "saved artifact signal manifest id '{}' no longer matches the replay signal manifest id '{}' for parent '{}'; rerun the options expression optimization from the original parent artifact",
            artifact.signal_manifest_id, replay.signal_manifest.manifest_id, artifact.parent_artifact_id
        ));
    }
    if replay.options_dataset_hash != artifact.options_dataset_hash {
        return Err(format!(
            "saved artifact dataset hash '{}' no longer matches the currently imported dataset hash '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
            artifact.options_dataset_hash, replay.options_dataset_hash, artifact.options_dataset_id
        ));
    }
    compare_exact_replay_optional_hash(
        "normalized_rows_sha256",
        saved_artifact_normalized_rows_sha256(artifact),
        non_empty_hash(replay.dataset.manifest.normalized_rows_sha256.as_ref()),
        &artifact.options_dataset_id,
    )?;
    compare_exact_replay_optional_hash(
        "manifest_semantic_sha256",
        saved_artifact_manifest_semantic_sha256(artifact),
        non_empty_hash(replay.dataset.manifest.manifest_semantic_sha256.as_ref()),
        &artifact.options_dataset_id,
    )?;
    if replay.dataset.manifest.price_semantics != artifact.price_semantics {
        return Err(format!(
            "saved artifact price semantics '{:?}' no longer match the currently imported dataset price semantics '{:?}' for '{}'; re-import the original dataset or rerun the options expression optimization",
            artifact.price_semantics,
            replay.dataset.manifest.price_semantics,
            artifact.options_dataset_id
        ));
    }
    if let Some(expected_provenance) = artifact.dataset_provenance.as_ref() {
        let actual_provenance = dataset_provenance_from_imported_dataset(replay.dataset.as_ref());
        let expected_provider_name = expected_provenance.provider_name.trim();
        if !expected_provider_name.is_empty()
            && canonical_options_provider_identity(&actual_provenance.provider_name)
                != canonical_options_provider_identity(expected_provider_name)
        {
            return Err(format!(
                "saved artifact Polygon.io provider identity '{}' no longer matches the currently imported dataset provider identity '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
                expected_provider_name, actual_provenance.provider_name, artifact.options_dataset_id
            ));
        }
        let expected_provider_surface = expected_provenance.provider_source_surface.trim();
        if !expected_provider_surface.is_empty()
            && actual_provenance.provider_source_surface != expected_provider_surface
        {
            return Err(format!(
                "saved artifact provider_source_surface '{}' no longer matches the currently imported dataset provider_source_surface '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
                expected_provider_surface,
                actual_provenance.provider_source_surface,
                artifact.options_dataset_id
            ));
        }
        let expected_quote_resolution = expected_provenance.quote_resolution.trim();
        if !expected_quote_resolution.is_empty()
            && actual_provenance.quote_resolution != expected_quote_resolution
        {
            return Err(format!(
                "saved artifact quote_resolution '{}' no longer matches the currently imported dataset quote_resolution '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
                expected_quote_resolution,
                actual_provenance.quote_resolution,
                artifact.options_dataset_id
            ));
        }
        if let Some(expected_aggregate_price_field) = expected_provenance
            .aggregate_price_field
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let actual_aggregate_price_field = actual_provenance
                .aggregate_price_field
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("");
            if actual_aggregate_price_field != expected_aggregate_price_field {
                return Err(format!(
                    "saved artifact aggregate_price_field '{}' no longer matches the currently imported dataset aggregate_price_field '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
                    expected_aggregate_price_field,
                    actual_aggregate_price_field,
                    artifact.options_dataset_id
                ));
            }
        }
        if let Some(expected_contract_reference_as_of_utc) = expected_provenance
            .contract_reference_as_of_utc
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let actual_contract_reference_as_of_utc = actual_provenance
                .contract_reference_as_of_utc
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("");
            if actual_contract_reference_as_of_utc != expected_contract_reference_as_of_utc {
                return Err(format!(
                    "saved artifact contract_reference_as_of_utc '{}' no longer matches the currently imported dataset contract_reference_as_of_utc '{}' for '{}'; re-import the original dataset or rerun the options expression optimization",
                    expected_contract_reference_as_of_utc,
                    actual_contract_reference_as_of_utc,
                    artifact.options_dataset_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_saved_artifact_parent_signal_replay_provenance(
    artifact: &OptionsExpressionArtifactV1,
    replay: &OptionsExpressionReplayInputs,
) -> Result<(), String> {
    if replay.parent_artifact_id != artifact.parent_artifact_id {
        return Err(format!(
            "saved artifact parent id '{}' no longer matches the replay parent id '{}'",
            artifact.parent_artifact_id, replay.parent_artifact_id
        ));
    }
    if !artifact.parent_artifact_hash.trim().is_empty()
        && replay.parent_artifact_hash != artifact.parent_artifact_hash
    {
        return Err(format!(
            "saved artifact parent hash '{}' no longer matches the replay parent hash '{}' for parent '{}'; reopen the original parent artifact or rerun the options expression optimization",
            artifact.parent_artifact_hash, replay.parent_artifact_hash, artifact.parent_artifact_id
        ));
    }
    if !artifact.signal_manifest_id.trim().is_empty()
        && replay.signal_manifest.manifest_id != artifact.signal_manifest_id
    {
        return Err(format!(
            "saved artifact signal manifest id '{}' no longer matches the replay signal manifest id '{}' for parent '{}'; rerun the options expression optimization from the original parent artifact",
            artifact.signal_manifest_id, replay.signal_manifest.manifest_id, artifact.parent_artifact_id
        ));
    }
    Ok(())
}

fn validate_options_expression_validation_replay_provenance(
    artifact: &OptionsExpressionArtifactV1,
    replay: &OptionsExpressionReplayInputs,
    dataset_policy: OptionsExpressionValidationDatasetPolicy,
) -> Result<bool, String> {
    match dataset_policy {
        OptionsExpressionValidationDatasetPolicy::RequireExactReplay => {
            validate_saved_artifact_replay_provenance(artifact, replay)?;
            Ok(false)
        }
        OptionsExpressionValidationDatasetPolicy::AllowSameRootReplacement => {
            validate_saved_artifact_parent_signal_replay_provenance(artifact, replay)?;
            if !replay
                .dataset
                .manifest
                .root_symbol
                .eq_ignore_ascii_case(&artifact.root_symbol)
            {
                return Err(format!(
                    "replacement validation requires the same root symbol as the saved artifact: saved '{}' vs replay '{}'",
                    artifact.root_symbol, replay.dataset.manifest.root_symbol
                ));
            }
            if replay.dataset.manifest.price_semantics != artifact.price_semantics {
                return Err(format!(
                    "same-root replacement validation requires matching price semantics: saved '{:?}' vs replay '{:?}'. Use explicit cross-lane comparison for Polygon.io realism-lane comparisons.",
                    artifact.price_semantics, replay.dataset.manifest.price_semantics
                ));
            }
            Ok(
                replay.normalized_req.options_dataset_id != artifact.options_dataset_id
                    || replay.options_dataset_hash != artifact.options_dataset_hash
                    || optional_replay_identity_hash_drift(artifact, replay),
            )
        }
        OptionsExpressionValidationDatasetPolicy::AllowExplicitCrossLaneComparison => {
            validate_saved_artifact_parent_signal_replay_provenance(artifact, replay)?;
            if !replay
                .dataset
                .manifest
                .root_symbol
                .eq_ignore_ascii_case(&artifact.root_symbol)
            {
                return Err(format!(
                    "cross-lane validation requires the same root symbol as the saved artifact: saved '{}' vs replay '{}'",
                    artifact.root_symbol, replay.dataset.manifest.root_symbol
                ));
            }
            Ok(
                replay.normalized_req.options_dataset_id != artifact.options_dataset_id
                    || replay.options_dataset_hash != artifact.options_dataset_hash
                    || replay.dataset.manifest.price_semantics != artifact.price_semantics
                    || optional_replay_identity_hash_drift(artifact, replay),
            )
        }
    }
}

fn validate_event_path_replay_consistency(
    expected_tuple: &ta_app_core::options_types::OptionsTupleSpec,
    tuple_inspection: &OptionsExpressionTupleInspectionResult,
    event_path_inspection: &OptionsExpressionEventPathInspectionResult,
) -> Result<(), String> {
    if tuple_inspection.tuple_result.tuple != *expected_tuple {
        return Err(format!(
            "recomputed tuple at index {} does not match the saved artifact ordering",
            tuple_inspection.tuple_index
        ));
    }
    if event_path_inspection.tuple_index != tuple_inspection.tuple_index {
        return Err(format!(
            "event-path replay tuple index {} does not match tuple inspection tuple index {}",
            event_path_inspection.tuple_index, tuple_inspection.tuple_index
        ));
    }
    if event_path_inspection.event_index != event_path_inspection.row.event_index {
        return Err(format!(
            "event-path replay event index {} does not match row event index {}",
            event_path_inspection.event_index, event_path_inspection.row.event_index
        ));
    }
    if event_path_inspection.event_id != event_path_inspection.row.event_id {
        return Err(
            "event-path replay event id does not match the returned row event id".to_string(),
        );
    }
    if event_path_inspection.point_count != event_path_inspection.points.len() {
        return Err(format!(
            "event-path replay point_count {} does not match point vector length {}",
            event_path_inspection.point_count,
            event_path_inspection.points.len()
        ));
    }
    let expected_row = tuple_inspection
        .rows
        .get(event_path_inspection.event_index)
        .ok_or_else(|| {
            format!(
                "event_index {} is out of range for recomputed tuple inspection with {} row{}",
                event_path_inspection.event_index,
                tuple_inspection.rows.len(),
                if tuple_inspection.rows.len() == 1 {
                    ""
                } else {
                    "s"
                }
            )
        })?;
    let expected_row_value = serde_json::to_value(expected_row)
        .map_err(|e| format!("failed to serialize recomputed tuple inspection row: {e}"))?;
    let event_path_row_value = serde_json::to_value(&event_path_inspection.row)
        .map_err(|e| format!("failed to serialize recomputed event-path row: {e}"))?;
    if event_path_row_value != expected_row_value {
        return Err(format!(
            "recomputed event path row for tuple {} event {} drifted from tuple inspection replay state",
            tuple_inspection.tuple_index, event_path_inspection.event_index
        ));
    }
    Ok(())
}

fn build_options_expression_replay_inputs_with_timing(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsExpressionReplayInputsBuildResult, String> {
    let _nvtx_scope = ta_app_core::nvtx::scope("engine", "options_replay_build_total");
    let total_start = std::time::Instant::now();
    let mut replay_timing = OptionsExpressionReplayBuildTimingBreakdown::default();

    let _parent_artifact_scope =
        ta_app_core::nvtx::scope("engine", "options_parent_artifact_validation");
    let parent_artifact_start = std::time::Instant::now();
    let runs_dir = runs_dir(app_local_data_dir)?;
    let parent_artifact_path = std::path::Path::new(req.parent_artifact_path.trim());
    if !parent_artifact_path.is_file() {
        return Err(format!(
            "parent artifact does not exist: {}",
            parent_artifact_path.display()
        ));
    }
    ensure_path_is_within_dir(parent_artifact_path, &runs_dir)?;
    let artifact = read_artifact_value(parent_artifact_path)?;
    let parent_artifact_hash = json_value_digest(&artifact)?;
    let kind = resolve_run_artifact_kind(&artifact);
    if !crate::commands::run_artifacts::is_run_artifact_kind(&kind) {
        return Err(format!(
            "options expression parent must be a saved run artifact, found kind '{}'",
            kind
        ));
    }
    let parent_family = supported_options_parent_family_from_artifact(&artifact);
    if parent_family.is_none() {
        return Err(format!(
            "options expression currently supports {} parent artifacts only, found '{}'",
            supported_options_parent_strategy_list(),
            non_empty_string(artifact.pointer("/run/req/strategy_kind"))
                .or_else(|| {
                    non_empty_string(
                        artifact.pointer("/validation/req/validation_context/strategy_kind"),
                    )
                })
                .or_else(|| non_empty_string(artifact.pointer("/validation/req/strategy_kind")))
                .unwrap_or_default()
                .trim()
        ));
    }
    replay_timing.parent_artifact_validation_us = saturating_elapsed_us(parent_artifact_start);

    let _source_resolution_scope = ta_app_core::nvtx::scope("engine", "options_source_resolution");
    let source_resolution_start = std::time::Instant::now();
    let csv_path = resolve_csv_path(&artifact);
    if csv_path.trim().is_empty() {
        return Err("parent artifact is missing dataset CSV path".to_string());
    }
    let timeframe = resolve_timeframe(&artifact);
    let adjustment_mode = resolve_adjustment_mode(&artifact);
    let source_csv_path = std::path::Path::new(&csv_path);
    let parent_artifact_id = parent_artifact_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("parent_artifact")
        .to_string();
    let parent_cache_key = format!(
        "{}|source={}|timeframe={}|candidate={}",
        file_cache_stamp(parent_artifact_path)?,
        file_cache_stamp(source_csv_path)?,
        timeframe.trim(),
        req.parent_candidate_index
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    replay_timing.source_resolution_us = saturating_elapsed_us(source_resolution_start);

    let _parent_signal_process_cache_scope =
        ta_app_core::nvtx::scope("engine", "options_parent_signal_process_cache_lookup");
    let parent_signal_process_cache_start = std::time::Instant::now();
    let process_cached_parent = load_parent_signal_cache_entry(&parent_cache_key);
    replay_timing.parent_signal_process_cache_lookup_us =
        saturating_elapsed_us(parent_signal_process_cache_start);

    let (parent_artifact_id, signal_manifest) =
        if let Some((cached_parent_artifact_id, cached_manifest)) = process_cached_parent {
            replay_timing.parent_signal_process_cache_hit = true;
            (cached_parent_artifact_id, cached_manifest)
        } else {
            let _persisted_signal_manifest_scope = ta_app_core::nvtx::scope(
                "engine",
                "options_persisted_signal_manifest_cache_lookup",
            );
            let persisted_signal_manifest_start = std::time::Instant::now();
            let persisted_manifest = load_signal_manifest_replay_cache(
                app_local_data_dir,
                &parent_artifact_id,
                &parent_cache_key,
            )?;
            replay_timing.persisted_signal_manifest_cache_lookup_us =
                saturating_elapsed_us(persisted_signal_manifest_start);
            if let Some(cached_manifest) = persisted_manifest {
                replay_timing.persisted_signal_manifest_cache_hit = true;
                let signal_manifest = Arc::new(cached_manifest);
                insert_parent_signal_cache_entry(CachedParentSignalReplay {
                    cache_key: parent_cache_key.clone(),
                    parent_artifact_id: parent_artifact_id.clone(),
                    signal_manifest: Arc::clone(&signal_manifest),
                });
                (parent_artifact_id, signal_manifest)
            } else {
                let _candle_replay_scope =
                    ta_app_core::nvtx::scope("engine", "options_candle_replay_load");
                let candle_replay_start = std::time::Instant::now();
                let (candles, source_identity, source_content_fingerprint) =
                    load_replayed_candles(app_local_data_dir, &artifact, &csv_path, &timeframe)?;
                replay_timing.candle_replay_load_us = saturating_elapsed_us(candle_replay_start);

                let _parent_replay_scope =
                    ta_app_core::nvtx::scope("engine", "options_parent_replay_and_drilldown");
                let parent_replay_and_drilldown_start = std::time::Instant::now();
                let source_dataset_identity_hash =
                    build_source_dataset_identity_hash(source_csv_path, source_identity);
                let dataset_hash = build_dataset_hash(&source_content_fingerprint, &timeframe);
                let symbol_root = resolve_symbol_root(&artifact, &csv_path);
                let replay =
                    build_supported_options_parent_replay(&artifact, req.parent_candidate_index)?;
                let curves = compute_double_ma_drilldown_blocking_with_candles(
                    replay.drilldown_request,
                    &candles,
                )?;
                replay_timing.parent_replay_and_drilldown_us =
                    saturating_elapsed_us(parent_replay_and_drilldown_start);

                let _signal_manifest_materialization_scope =
                    ta_app_core::nvtx::scope("engine", "options_signal_manifest_materialization");
                let signal_manifest_materialization_start = std::time::Instant::now();
                let signal_manifest = Arc::new(build_supported_options_parent_signal_manifest(
                    replay.family,
                    &parent_artifact_id,
                    &kind,
                    &source_dataset_identity_hash,
                    &source_content_fingerprint,
                    &dataset_hash,
                    &timeframe,
                    None,
                    &adjustment_mode,
                    &symbol_root,
                    replay.trade_on_next_bar,
                    &candles,
                    &curves,
                )?);
                persist_signal_manifest_replay_cache(
                    app_local_data_dir,
                    &parent_artifact_id,
                    &parent_cache_key,
                    signal_manifest.as_ref(),
                )?;
                insert_parent_signal_cache_entry(CachedParentSignalReplay {
                    cache_key: parent_cache_key,
                    parent_artifact_id: parent_artifact_id.clone(),
                    signal_manifest: Arc::clone(&signal_manifest),
                });
                replay_timing.signal_manifest_materialization_us =
                    saturating_elapsed_us(signal_manifest_materialization_start);
                (parent_artifact_id, signal_manifest)
            }
        };

    let _dataset_manifest_validation_scope =
        ta_app_core::nvtx::scope("engine", "options_dataset_manifest_validation");
    let dataset_manifest_validation_start = std::time::Instant::now();
    let datasets_dir = options_datasets_dir(app_local_data_dir)?;
    let dataset_id = sanitize_dataset_id(req.options_dataset_id.trim())?;
    let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
    if !manifest_path.is_file() {
        return Err(format!("options dataset '{}' is not imported", dataset_id));
    }
    let dataset_manifest = read_options_dataset_manifest(&manifest_path)?;
    if !dataset_manifest
        .price_semantics
        .supports_fill_model(req.fill_model)
    {
        return Err(format!(
            "fill model '{:?}' is incompatible with options dataset price semantics '{:?}'. allowed fill models: {}",
            req.fill_model,
            dataset_manifest.price_semantics,
            dataset_manifest.price_semantics.allowed_fill_models_label()
        ));
    }
    if matches!(
        dataset_manifest.price_semantics,
        OptionsPriceSemantics::MinuteAggregate
    ) && dataset_manifest
        .aggregate_price_field
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| !value.eq_ignore_ascii_case("close"))
        .unwrap_or(false)
    {
        return Err(
            "minute-aggregate options datasets currently require aggregate_price_field='close'"
                .to_string(),
        );
    }
    let dataset_rows_path = dataset_rows_path_from_manifest_path(&manifest_path)?;
    replay_timing.options_dataset_manifest_validation_us =
        saturating_elapsed_us(dataset_manifest_validation_start);

    let dataset_cache_key = format!(
        "{}|{}",
        file_cache_stamp(&manifest_path)?,
        file_cache_stamp(&dataset_rows_path)?
    );
    let _options_dataset_process_cache_scope =
        ta_app_core::nvtx::scope("engine", "options_dataset_process_cache_lookup");
    let options_dataset_process_cache_start = std::time::Instant::now();
    let process_cached_dataset = load_options_dataset_cache_entry(&dataset_cache_key);
    replay_timing.options_dataset_process_cache_lookup_us =
        saturating_elapsed_us(options_dataset_process_cache_start);

    let (options_dataset_hash, dataset) = if let Some((cached_hash, cached_dataset)) =
        process_cached_dataset
    {
        replay_timing.options_dataset_process_cache_hit = true;
        (cached_hash, cached_dataset)
    } else {
        let _options_dataset_persisted_cache_scope =
            ta_app_core::nvtx::scope("engine", "options_dataset_persisted_cache_lookup");
        let options_dataset_persisted_cache_start = std::time::Instant::now();
        let persisted_dataset =
            load_options_dataset_replay_cache(app_local_data_dir, &dataset_id, &dataset_cache_key)?;
        replay_timing.options_dataset_persisted_cache_lookup_us =
            saturating_elapsed_us(options_dataset_persisted_cache_start);
        if let Some((cached_hash, cached_dataset)) = persisted_dataset {
            replay_timing.options_dataset_persisted_cache_hit = true;
            let dataset = Arc::new(cached_dataset);
            insert_options_dataset_cache_entry(CachedOptionsDatasetReplay {
                cache_key: dataset_cache_key,
                options_dataset_hash: cached_hash.clone(),
                dataset: Arc::clone(&dataset),
            });
            (cached_hash, dataset)
        } else {
            let _options_dataset_csv_load_scope =
                ta_app_core::nvtx::scope("engine", "options_dataset_csv_load");
            let options_dataset_csv_load_start = std::time::Instant::now();
            let options_dataset_hash =
                data_cache::content_fingerprint_for_source(&dataset_rows_path)?;
            let dataset = Arc::new(load_imported_options_dataset_from_csv(
                &build_import_request_from_manifest(&dataset_manifest, &dataset_rows_path),
            )?);
            persist_options_dataset_replay_cache(
                app_local_data_dir,
                &dataset_id,
                &dataset_cache_key,
                &options_dataset_hash,
                dataset.as_ref(),
            )?;
            replay_timing.options_dataset_csv_load_us =
                saturating_elapsed_us(options_dataset_csv_load_start);
            insert_options_dataset_cache_entry(CachedOptionsDatasetReplay {
                cache_key: dataset_cache_key,
                options_dataset_hash: options_dataset_hash.clone(),
                dataset: Arc::clone(&dataset),
            });
            (options_dataset_hash, dataset)
        }
    };

    let _request_normalization_scope =
        ta_app_core::nvtx::scope("engine", "options_request_normalization");
    let request_normalization_start = std::time::Instant::now();
    let mut normalized_req = req;
    normalized_req.options_dataset_id = dataset_id;
    replay_timing.request_normalization_us = saturating_elapsed_us(request_normalization_start);
    replay_timing.total_us = saturating_elapsed_us(total_start);

    Ok(OptionsExpressionReplayInputsBuildResult {
        inputs: OptionsExpressionReplayInputs {
            parent_artifact_id,
            parent_artifact_hash,
            normalized_req,
            signal_manifest,
            dataset,
            options_dataset_hash,
        },
        timing: replay_timing,
    })
}

fn build_options_expression_replay_inputs(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsExpressionReplayInputs, String> {
    Ok(build_options_expression_replay_inputs_with_timing(req, app_local_data_dir)?.inputs)
}

fn run_options_expression_blocking_impl(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
    cancel: Option<&AtomicBool>,
) -> Result<RunOptionsExpressionResponse, String> {
    let _nvtx_scope = ta_app_core::nvtx::scope("engine", "options_expression_run_total");
    ensure_options_expression_not_cancelled(cancel, "replay preparation")?;
    let replay = build_options_expression_replay_inputs(req, app_local_data_dir)?;
    ensure_options_expression_not_cancelled(cancel, "optimization")?;
    let result = run_options_expression_blocking(
        replay.normalized_req.clone(),
        replay.signal_manifest.as_ref(),
        replay.dataset.as_ref(),
    )?;
    ensure_options_expression_not_cancelled(cancel, "artifact publication")?;
    let ts = chrono::Utc::now().to_rfc3339();
    let _artifact_library_lock =
        crate::commands::run_artifacts::acquire_artifact_library_process_lock_for_app_local_data(
            app_local_data_dir,
        )?;
    let artifact_path = suggest_options_expression_artifact_path(app_local_data_dir, &ts)?;
    let artifact = build_options_expression_artifact(
        &artifact_path,
        replay.parent_artifact_id,
        replay.parent_artifact_hash,
        replay.options_dataset_hash,
        replay.signal_manifest.manifest_id.clone(),
        Some(dataset_provenance_from_imported_dataset(
            replay.dataset.as_ref(),
        )),
        replay.normalized_req,
        &result,
    );
    let bytes = serde_json::to_vec_pretty(&artifact)
        .map_err(|e| format!("failed to serialize options expression artifact: {e}"))?;
    ensure_options_expression_not_cancelled(cancel, "artifact publication")?;
    crate::commands::run_artifacts::invalidate_artifact_catalog_for_base_under_process_lock(
        app_local_data_dir,
    )?;
    crate::commands::run_artifacts::publish_new_artifact_file_atomically(&artifact_path, &bytes)?;
    let runs_dir = app_local_data_dir.join("runs");
    let retention =
        crate::commands::run_artifacts::artifact_retention_policy_for_base(app_local_data_dir);
    crate::commands::run_artifacts::prune_old_options_expression_artifacts_in_dir_under_process_lock(
        &runs_dir,
        retention.options_expression_keep_newest,
    );

    Ok(RunOptionsExpressionResponse {
        artifact_path: artifact_path.to_string_lossy().to_string(),
        artifact,
        result,
    })
}

fn analyze_options_expression_feasibility_blocking_impl(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<AnalyzeOptionsExpressionFeasibilityResponse, String> {
    let _nvtx_scope = ta_app_core::nvtx::scope("engine", "options_expression_feasibility_total");
    let preview_start = std::time::Instant::now();
    let replay_start = std::time::Instant::now();
    let replay = build_options_expression_replay_inputs_with_timing(req, app_local_data_dir)?;
    let replay_build_ms = replay_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let report = analyze_options_expression_feasibility_blocking(
        replay.inputs.normalized_req,
        replay.inputs.signal_manifest.as_ref(),
        replay.inputs.dataset.as_ref(),
    )?;
    let total_preview_ms = preview_start
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    Ok(AnalyzeOptionsExpressionFeasibilityResponse {
        replay_build_ms,
        total_preview_ms,
        replay_build_timing: Some(replay.timing),
        report,
    })
}

pub fn analyze_options_expression_feasibility_blocking_headless(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<AnalyzeOptionsExpressionFeasibilityResponse, String> {
    analyze_options_expression_feasibility_blocking_impl(req, app_local_data_dir)
}

pub fn run_options_expression_blocking_headless(
    req: OptionsExpressionRunRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<RunOptionsExpressionResponse, String> {
    run_options_expression_blocking_impl(req, app_local_data_dir, None)
}

pub fn inspect_options_expression_tuple_blocking_headless(
    req: OptionsExpressionRunRequest,
    tuple_index: usize,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsExpressionTupleInspectionResult, String> {
    let replay = build_options_expression_replay_inputs(req, app_local_data_dir)?;
    inspect_options_expression_tuple_blocking(
        replay.normalized_req,
        tuple_index,
        replay.signal_manifest.as_ref(),
        replay.dataset.as_ref(),
    )
}

pub fn inspect_options_expression_event_path_blocking_headless(
    req: OptionsExpressionRunRequest,
    tuple_index: usize,
    event_index: usize,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsExpressionEventPathInspectionResult, String> {
    let replay = build_options_expression_replay_inputs(req, app_local_data_dir)?;
    inspect_options_expression_event_path_blocking(
        replay.normalized_req,
        tuple_index,
        event_index,
        replay.signal_manifest.as_ref(),
        replay.dataset.as_ref(),
    )
}

pub fn list_options_expression_artifacts_blocking_headless(
    app_local_data_dir: &std::path::Path,
) -> Result<Vec<OptionsExpressionArtifactSummary>, String> {
    let runs_dir = runs_dir(app_local_data_dir)?;
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<OptionsExpressionArtifactSummary> = Vec::new();
    for entry in std::fs::read_dir(&runs_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if ensure_path_is_within_dir(&path, &runs_dir).is_err() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if !file_name.starts_with("vectorgrid_options_expression_") {
            continue;
        }
        let bytes = entry.metadata().map_err(|e| e.to_string())?.len();
        let Ok(value) = read_artifact_value(&path) else {
            continue;
        };
        let Ok(artifact) = serde_json::from_value::<OptionsExpressionArtifactV1>(value) else {
            continue;
        };
        if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
            continue;
        }
        out.push(summarize_options_expression_artifact_with_runs_dir(
            &path,
            bytes,
            &artifact,
            Some(&runs_dir),
        ));
    }
    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

pub fn load_options_expression_artifact_blocking_headless(
    artifact_path: &std::path::Path,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsExpressionArtifactV1, String> {
    if !artifact_path.is_file() {
        return Err(format!(
            "options expression artifact does not exist: {}",
            artifact_path.display()
        ));
    }
    let runs_dir = runs_dir(app_local_data_dir)?;
    ensure_path_is_within_dir(artifact_path, &runs_dir)?;
    let value = read_artifact_value(artifact_path)?;
    let mut artifact =
        serde_json::from_value::<OptionsExpressionArtifactV1>(value).map_err(|e| {
            format!(
                "failed to deserialize options expression artifact {}: {e}",
                artifact_path.display()
            )
        })?;
    if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
        return Err(format!(
            "unsupported options expression artifact kind: {}",
            artifact.kind
        ));
    }
    refresh_options_expression_artifact_validation_verdicts(&mut artifact);
    Ok(artifact)
}

fn load_options_expression_artifact_with_hash(
    artifact_path: &std::path::Path,
    app_local_data_dir: &std::path::Path,
) -> Result<(OptionsExpressionArtifactV1, String), String> {
    if !artifact_path.is_file() {
        return Err(format!(
            "options expression artifact does not exist: {}",
            artifact_path.display()
        ));
    }
    let runs_dir = runs_dir(app_local_data_dir)?;
    ensure_path_is_within_dir(artifact_path, &runs_dir)?;
    let value = read_artifact_value(artifact_path)?;
    let source_artifact_hash = json_value_digest(&value)?;
    let mut artifact =
        serde_json::from_value::<OptionsExpressionArtifactV1>(value).map_err(|e| {
            format!(
                "failed to deserialize options expression artifact {}: {e}",
                artifact_path.display()
            )
        })?;
    if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
        return Err(format!(
            "unsupported options expression artifact kind: {}",
            artifact.kind
        ));
    }
    refresh_options_expression_artifact_validation_verdicts(&mut artifact);
    Ok((artifact, source_artifact_hash))
}

fn effective_validation_replay_request(
    artifact: &OptionsExpressionArtifactV1,
    req: &OptionsExpressionValidationRequest,
) -> Result<OptionsExpressionRunRequest, String> {
    let mut replay_req = artifact.request.clone();
    let requested_dataset_id = req.options_dataset_id.trim();
    if !requested_dataset_id.is_empty() {
        replay_req.options_dataset_id = requested_dataset_id.to_string();
    }
    if let Some(fill_model) = req.fill_model_override {
        replay_req.fill_model = fill_model;
    }
    match req.validation_plan.mode {
        OptionsExpressionValidationPlanMode::OriginalEmbeddedSplits => {}
        OptionsExpressionValidationPlanMode::SessionChronologicalReplay => {
            replay_req.split_assignment_mode =
                ta_app_core::OptionsSplitAssignmentMode::SessionChronological;
        }
        OptionsExpressionValidationPlanMode::DateRangeHoldout => {
            let start = req.validation_plan.start_timestamp_ms.ok_or_else(|| {
                "date_range_holdout validation requires start_timestamp_ms".to_string()
            })?;
            let end = req.validation_plan.end_timestamp_ms.ok_or_else(|| {
                "date_range_holdout validation requires end_timestamp_ms".to_string()
            })?;
            if start >= end {
                return Err(
                    "date_range_holdout validation requires start_timestamp_ms < end_timestamp_ms"
                        .to_string(),
                );
            }
        }
        OptionsExpressionValidationPlanMode::AnchoredWalkForward
        | OptionsExpressionValidationPlanMode::RollingSessionWalkForward => {
            let train = req.validation_plan.train_session_count.unwrap_or(0);
            let validation = req.validation_plan.validation_session_count.unwrap_or(0);
            let step = req.validation_plan.step_session_count.unwrap_or(validation);
            if train == 0 || validation == 0 || step == 0 {
                return Err(
                    "walk-forward validation requires positive train_session_count, validation_session_count, and step_session_count"
                        .to_string(),
                );
            }
        }
    }
    Ok(replay_req)
}

fn validation_session_key(event: &ta_app_core::OptionsSignalEvent) -> String {
    event
        .session_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            format!(
                "utc_day:{}",
                event.earliest_tradable_timestamp.div_euclid(86_400_000)
            )
        })
}

fn validation_plan_preview_notes(
    plan: &OptionsExpressionValidationPlan,
    signal_manifest: &ta_app_core::OptionsSignalEventManifestV1,
) -> Vec<String> {
    match plan.mode {
        OptionsExpressionValidationPlanMode::OriginalEmbeddedSplits => {
            vec!["Validation will reuse the saved artifact's embedded split policy.".to_string()]
        }
        OptionsExpressionValidationPlanMode::SessionChronologicalReplay => vec![
            "Validation will replay selected tuples with session-chronological splits.".to_string(),
        ],
        OptionsExpressionValidationPlanMode::DateRangeHoldout => {
            if let (Some(start), Some(end)) = (plan.start_timestamp_ms, plan.end_timestamp_ms) {
                let event_count = signal_manifest
                    .events
                    .iter()
                    .filter(|event| {
                        event.earliest_tradable_timestamp >= start
                            && event.earliest_tradable_timestamp < end
                    })
                    .count();
                vec![format!(
                    "Validation will assign {} event{} in the end-exclusive date range [{start}, {end}) to holdout.",
                    event_count,
                    if event_count == 1 { "" } else { "s" }
                )]
            } else {
                vec!["Validation date-range holdout requires start and end timestamps.".to_string()]
            }
        }
        OptionsExpressionValidationPlanMode::AnchoredWalkForward
        | OptionsExpressionValidationPlanMode::RollingSessionWalkForward => {
            let mut sessions = BTreeSet::new();
            for event in &signal_manifest.events {
                sessions.insert(validation_session_key(event));
            }
            let mode = match plan.mode {
                OptionsExpressionValidationPlanMode::AnchoredWalkForward => "anchored_walk_forward",
                OptionsExpressionValidationPlanMode::RollingSessionWalkForward => {
                    "rolling_session_walk_forward"
                }
                _ => unreachable!(),
            };
            vec![format!(
                "Validation will evaluate retained tuples with {mode} over {} ordered session{} using train={}, validation={}, step={}.",
                sessions.len(),
                if sessions.len() == 1 { "" } else { "s" },
                plan.train_session_count.unwrap_or(0),
                plan.validation_session_count.unwrap_or(0),
                plan.step_session_count
                    .or(plan.validation_session_count)
                    .unwrap_or(0)
            )]
        }
    }
}

fn select_options_expression_validation_tuple_indexes(
    artifact: &OptionsExpressionArtifactV1,
    selection: &OptionsExpressionValidationTupleSelection,
) -> Result<Vec<usize>, String> {
    let tuple_count = artifact.selection_metrics.len();
    let mut selected = BTreeSet::new();
    if selection.include_sealed_finalists {
        for (index, tuple) in artifact.selection_metrics.iter().enumerate() {
            if tuple.sealed_finalist {
                selected.insert(index);
            }
        }
    }
    if selection.include_robust_leader {
        if let Some(index) = best_tuple_index_by_metric(&artifact.selection_metrics, |entry| {
            Some(entry.robust_score)
        }) {
            selected.insert(index);
        }
    }
    if selection.include_holdout_leader {
        if let Some(index) = best_tuple_index_by_metric(&artifact.selection_metrics, |entry| {
            entry
                .supported_holdout_metrics()
                .map(|metrics| metrics.expectancy_per_premium)
        }) {
            selected.insert(index);
        }
    }
    for index in &selection.tuple_indexes {
        if *index >= tuple_count {
            return Err(format!(
                "validation tuple index {} is out of range for saved artifact with {} tuple{}",
                index,
                tuple_count,
                if tuple_count == 1 { "" } else { "s" }
            ));
        }
        selected.insert(*index);
    }
    if selected.is_empty() && tuple_count > 0 {
        selected.insert(0);
    }
    Ok(selected.into_iter().collect())
}

fn selected_options_expression_validation_tuples(
    artifact: &OptionsExpressionArtifactV1,
    selection: &OptionsExpressionValidationTupleSelection,
) -> Result<Vec<(usize, ta_app_core::OptionsExpressionTupleResult)>, String> {
    let indexes = select_options_expression_validation_tuple_indexes(artifact, selection)?;
    indexes
        .into_iter()
        .map(|index| {
            artifact
                .selection_metrics
                .get(index)
                .cloned()
                .map(|tuple| (index, tuple))
                .ok_or_else(|| {
                    format!(
                        "validation tuple index {} is out of range for saved artifact",
                        index
                    )
                })
        })
        .collect()
}

fn build_options_expression_validation_result(
    validation_artifact_path: &std::path::Path,
    source_artifact_path: &std::path::Path,
    source_artifact_hash: String,
    artifact: &OptionsExpressionArtifactV1,
    replay: &OptionsExpressionReplayInputs,
    req: OptionsExpressionValidationRequest,
    replacement_dataset: bool,
    evaluation: ta_app_core::OptionsExpressionValidationEvaluationResult,
) -> OptionsExpressionValidationResult {
    OptionsExpressionValidationResult {
        version: OPTIONS_EXPRESSION_VALIDATION_ARTIFACT_VERSION,
        kind: "vectorgrid_options_expression_validation_v1".to_string(),
        ts: chrono::Utc::now().to_rfc3339(),
        validation_artifact_path: validation_artifact_path.to_string_lossy().to_string(),
        source_artifact_path: source_artifact_path.to_string_lossy().to_string(),
        source_artifact_hash,
        parent_artifact_id: artifact.parent_artifact_id.clone(),
        parent_artifact_hash: artifact.parent_artifact_hash.clone(),
        signal_manifest_id: replay.signal_manifest.manifest_id.clone(),
        root_symbol: replay.dataset.manifest.root_symbol.clone(),
        options_dataset_id: replay.normalized_req.options_dataset_id.clone(),
        options_dataset_hash: replay.options_dataset_hash.clone(),
        options_dataset_normalized_rows_sha256: replay
            .dataset
            .manifest
            .normalized_rows_sha256
            .clone(),
        options_dataset_manifest_semantic_sha256: replay
            .dataset
            .manifest
            .manifest_semantic_sha256
            .clone(),
        dataset_policy: req.dataset_policy,
        replacement_dataset,
        validation_plan: req.validation_plan,
        tuple_selection: req.tuple_selection,
        price_semantics: replay.dataset.manifest.price_semantics,
        dataset_provenance: Some(dataset_provenance_from_imported_dataset(
            replay.dataset.as_ref(),
        )),
        diagnostics: evaluation.diagnostics,
        overlap_window: evaluation.overlap_window,
        coverage_adequacy: evaluation.coverage_adequacy,
        lane_summaries: evaluation.lane_summaries,
        acceptance_summary: evaluation.acceptance_summary,
        tuple_results: evaluation.tuple_results,
    }
}

fn validate_options_expression_artifact_blocking_impl(
    req: OptionsExpressionValidationRequest,
    app_local_data_dir: &std::path::Path,
    cancel: Option<&AtomicBool>,
) -> Result<ValidateOptionsExpressionArtifactResponse, String> {
    ensure_options_expression_not_cancelled(cancel, "validation artifact loading")?;
    let source_artifact_path = std::path::PathBuf::from(req.artifact_path.trim());
    let (artifact, source_artifact_hash) =
        load_options_expression_artifact_with_hash(&source_artifact_path, app_local_data_dir)?;
    let source_tuples =
        selected_options_expression_validation_tuples(&artifact, &req.tuple_selection)?;
    let replay_req = effective_validation_replay_request(&artifact, &req)?;
    let replay_start = std::time::Instant::now();
    let replay =
        build_options_expression_replay_inputs_with_timing(replay_req, app_local_data_dir)?;
    ensure_options_expression_not_cancelled(cancel, "validation")?;
    let replay_build_ms = replay_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let replacement_dataset = validate_options_expression_validation_replay_provenance(
        &artifact,
        &replay.inputs,
        req.dataset_policy,
    )?;
    let validation_start = std::time::Instant::now();
    let evaluation = validate_options_expression_tuples_blocking(
        replay.inputs.normalized_req.clone(),
        &source_tuples,
        replay.inputs.signal_manifest.as_ref(),
        replay.inputs.dataset.as_ref(),
        &req.validation_plan,
        req.min_valid_trades.or(artifact.request.min_valid_trades),
    )?;
    ensure_options_expression_not_cancelled(cancel, "validation artifact publication")?;
    let validation_ms = validation_start
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let ts = chrono::Utc::now().to_rfc3339();
    let _artifact_library_lock =
        crate::commands::run_artifacts::acquire_artifact_library_process_lock_for_app_local_data(
            app_local_data_dir,
        )?;
    let validation_artifact_path =
        suggest_options_expression_validation_artifact_path(app_local_data_dir, &ts)?;
    let result = build_options_expression_validation_result(
        &validation_artifact_path,
        &source_artifact_path,
        source_artifact_hash,
        &artifact,
        &replay.inputs,
        req,
        replacement_dataset,
        evaluation,
    );
    let bytes = serde_json::to_vec_pretty(&result)
        .map_err(|e| format!("failed to serialize options validation artifact: {e}"))?;
    ensure_options_expression_not_cancelled(cancel, "validation artifact publication")?;
    crate::commands::run_artifacts::invalidate_artifact_catalog_for_base_under_process_lock(
        app_local_data_dir,
    )?;
    crate::commands::run_artifacts::publish_new_artifact_file_atomically(
        &validation_artifact_path,
        &bytes,
    )?;
    let runs_dir = app_local_data_dir.join("runs");
    let retention =
        crate::commands::run_artifacts::artifact_retention_policy_for_base(app_local_data_dir);
    crate::commands::run_artifacts::prune_old_options_validation_artifacts_in_dir_under_process_lock(
        &runs_dir,
        retention.options_expression_keep_newest,
    );
    Ok(ValidateOptionsExpressionArtifactResponse {
        artifact_path: validation_artifact_path.to_string_lossy().to_string(),
        replay_build_ms,
        validation_ms,
        replay_build_timing: Some(replay.timing),
        result,
    })
}

pub fn validate_options_expression_artifact_blocking_headless(
    req: OptionsExpressionValidationRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<ValidateOptionsExpressionArtifactResponse, String> {
    validate_options_expression_artifact_blocking_impl(req, app_local_data_dir, None)
}

fn analyze_options_expression_validation_feasibility_blocking_impl(
    req: OptionsExpressionValidationRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<AnalyzeOptionsExpressionValidationFeasibilityResponse, String> {
    let source_artifact_path = std::path::PathBuf::from(req.artifact_path.trim());
    let (artifact, source_artifact_hash) =
        load_options_expression_artifact_with_hash(&source_artifact_path, app_local_data_dir)?;
    let selected_tuple_indexes =
        select_options_expression_validation_tuple_indexes(&artifact, &req.tuple_selection)?;
    let replay_req = effective_validation_replay_request(&artifact, &req)?;
    let replay_start = std::time::Instant::now();
    let replay =
        build_options_expression_replay_inputs_with_timing(replay_req, app_local_data_dir)?;
    let replay_build_ms = replay_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let replacement_dataset = validate_options_expression_validation_replay_provenance(
        &artifact,
        &replay.inputs,
        req.dataset_policy,
    )?;
    let mut notes = Vec::new();
    if replacement_dataset {
        notes.push(
            "Validation will use an explicitly selected replacement Polygon.io dataset."
                .to_string(),
        );
    } else {
        notes.push(
            "Validation is eligible for exact replay against the saved Polygon.io dataset."
                .to_string(),
        );
    }
    notes.extend(validation_plan_preview_notes(
        &req.validation_plan,
        replay.inputs.signal_manifest.as_ref(),
    ));
    Ok(AnalyzeOptionsExpressionValidationFeasibilityResponse {
        replay_build_ms,
        replay_build_timing: Some(replay.timing),
        artifact_path: source_artifact_path.to_string_lossy().to_string(),
        source_artifact_hash,
        options_dataset_id: replay.inputs.normalized_req.options_dataset_id,
        options_dataset_hash: replay.inputs.options_dataset_hash,
        options_dataset_normalized_rows_sha256: replay
            .inputs
            .dataset
            .manifest
            .normalized_rows_sha256
            .clone(),
        options_dataset_manifest_semantic_sha256: replay
            .inputs
            .dataset
            .manifest
            .manifest_semantic_sha256
            .clone(),
        dataset_policy: req.dataset_policy,
        replacement_dataset,
        validation_plan: req.validation_plan,
        tuple_selection: req.tuple_selection,
        selected_tuple_count: selected_tuple_indexes.len(),
        selected_tuple_indexes,
        price_semantics: replay.inputs.dataset.manifest.price_semantics,
        dataset_provenance: Some(dataset_provenance_from_imported_dataset(
            replay.inputs.dataset.as_ref(),
        )),
        notes,
    })
}

pub fn analyze_options_expression_validation_feasibility_blocking_headless(
    req: OptionsExpressionValidationRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<AnalyzeOptionsExpressionValidationFeasibilityResponse, String> {
    analyze_options_expression_validation_feasibility_blocking_impl(req, app_local_data_dir)
}

pub fn save_options_csv_blocking_headless(
    req: SaveOptionsCsvRequest,
) -> Result<SaveOptionsCsvResponse, String> {
    let output_path = normalize_options_csv_save_path(&req.path)?;
    write_options_csv_to_path(&output_path, &req.csv_text)
}

fn normalize_options_csv_save_path(raw_path: &str) -> Result<PathBuf, String> {
    let normalized_path = raw_path.trim();
    if normalized_path.is_empty() {
        return Err("CSV save path is required.".to_string());
    }
    let output_path = PathBuf::from(normalized_path);
    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .unwrap_or_default();
    if file_name.is_empty() {
        return Err(format!(
            "CSV save path must point to a file: {}",
            output_path.display()
        ));
    }
    match output_path.extension().and_then(|value| value.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("csv") => {}
        Some(_) => {
            return Err(format!(
                "CSV save path must use a .csv extension: {}",
                output_path.display()
            ));
        }
        None => {
            return Err(format!(
                "CSV save path must use a .csv extension: {}",
                output_path.display()
            ));
        }
    }
    Ok(output_path)
}

fn write_options_csv_to_path(
    output_path: &Path,
    csv_text: &str,
) -> Result<SaveOptionsCsvResponse, String> {
    write_options_output_bytes_atomically(output_path, csv_text.as_bytes(), "options CSV export")?;
    Ok(SaveOptionsCsvResponse {
        path: output_path.to_string_lossy().to_string(),
        bytes: csv_text.as_bytes().len(),
    })
}

#[tauri::command]
pub async fn run_options_expression(
    req: OptionsExpressionRunRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunOptionsExpressionResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope("engine", "tauri_options_expression_request");
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        run_options_expression_blocking_impl(req, &app_local_data_dir, Some(cancel.as_ref()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn analyze_options_expression_feasibility(
    req: OptionsExpressionRunRequest,
    app: tauri::AppHandle,
) -> Result<AnalyzeOptionsExpressionFeasibilityResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope =
            ta_app_core::nvtx::scope("engine", "tauri_options_expression_feasibility_request");
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        analyze_options_expression_feasibility_blocking_impl(req, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn validate_options_expression_artifact(
    req: OptionsExpressionValidationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ValidateOptionsExpressionArtifactResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope =
            ta_app_core::nvtx::scope("engine", "tauri_options_expression_validation_request");
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        validate_options_expression_artifact_blocking_impl(
            req,
            &app_local_data_dir,
            Some(cancel.as_ref()),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn analyze_options_expression_validation_feasibility(
    req: OptionsExpressionValidationRequest,
    app: tauri::AppHandle,
) -> Result<AnalyzeOptionsExpressionValidationFeasibilityResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope(
            "engine",
            "tauri_options_expression_validation_feasibility_request",
        );
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        analyze_options_expression_validation_feasibility_blocking_impl(req, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_options_expression_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<OptionsExpressionArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        list_options_expression_artifacts_blocking_headless(&app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_options_expression_artifact(
    path: String,
    app: tauri::AppHandle,
) -> Result<OptionsExpressionArtifactV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let artifact_path = std::path::Path::new(path.trim());
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        load_options_expression_artifact_blocking_headless(artifact_path, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_options_csv(
    req: SaveOptionsCsvRequest,
    state: tauri::State<'_, AppState>,
) -> Result<SaveOptionsCsvResponse, String> {
    let output_path = normalize_options_csv_save_path(&req.path)?;
    state.consume_issued_csv_save_path(&output_path)?;
    let req = SaveOptionsCsvRequest {
        path: output_path.to_string_lossy().to_string(),
        csv_text: req.csv_text,
    };
    tauri::async_runtime::spawn_blocking(move || save_options_csv_blocking_headless(req))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn inspect_options_expression_tuple(
    req: InspectOptionsExpressionTupleRequest,
    app: tauri::AppHandle,
) -> Result<OptionsExpressionTupleInspectionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let artifact_path = std::path::Path::new(req.artifact_path.trim());
        if !artifact_path.is_file() {
            return Err(format!(
                "options expression artifact does not exist: {}",
                artifact_path.display()
            ));
        }
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let runs_dir = runs_dir(&app_local_data_dir)?;
        ensure_path_is_within_dir(artifact_path, &runs_dir)?;
        let value = read_artifact_value(artifact_path)?;
        let artifact =
            serde_json::from_value::<OptionsExpressionArtifactV1>(value).map_err(|e| {
                format!(
                    "failed to deserialize options expression artifact {}: {e}",
                    artifact_path.display()
                )
            })?;
        if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
            return Err(format!(
                "unsupported options expression artifact kind: {}",
                artifact.kind
            ));
        }
        let expected_tuple = artifact
            .selection_metrics
            .get(req.tuple_index)
            .ok_or_else(|| {
                format!(
                    "tuple_index {} is out of range for saved artifact with {} tuple{}",
                    req.tuple_index,
                    artifact.selection_metrics.len(),
                    if artifact.selection_metrics.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            })?
            .tuple
            .clone();
        let replay =
            build_options_expression_replay_inputs(artifact.request.clone(), &app_local_data_dir)?;
        validate_saved_artifact_replay_provenance(&artifact, &replay)?;
        let inspection = inspect_options_expression_tuple_blocking(
            replay.normalized_req,
            req.tuple_index,
            replay.signal_manifest.as_ref(),
            replay.dataset.as_ref(),
        )?;
        if inspection.tuple_result.tuple != expected_tuple {
            return Err(format!(
                "recomputed tuple at index {} does not match the saved artifact ordering",
                req.tuple_index
            ));
        }
        Ok(inspection)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn inspect_options_expression_event_path(
    req: InspectOptionsExpressionEventPathRequest,
    app: tauri::AppHandle,
) -> Result<OptionsExpressionEventPathInspectionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let artifact_path = std::path::Path::new(req.artifact_path.trim());
        if !artifact_path.is_file() {
            return Err(format!(
                "options expression artifact does not exist: {}",
                artifact_path.display()
            ));
        }
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let runs_dir = runs_dir(&app_local_data_dir)?;
        ensure_path_is_within_dir(artifact_path, &runs_dir)?;
        let value = read_artifact_value(artifact_path)?;
        let artifact =
            serde_json::from_value::<OptionsExpressionArtifactV1>(value).map_err(|e| {
                format!(
                    "failed to deserialize options expression artifact {}: {e}",
                    artifact_path.display()
                )
            })?;
        if artifact.kind.trim() != "vectorgrid_options_expression_v1" {
            return Err(format!(
                "unsupported options expression artifact kind: {}",
                artifact.kind
            ));
        }
        let expected_tuple = artifact
            .selection_metrics
            .get(req.tuple_index)
            .ok_or_else(|| {
                format!(
                    "tuple_index {} is out of range for saved artifact with {} tuple{}",
                    req.tuple_index,
                    artifact.selection_metrics.len(),
                    if artifact.selection_metrics.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            })?
            .tuple
            .clone();
        let replay =
            build_options_expression_replay_inputs(artifact.request.clone(), &app_local_data_dir)?;
        validate_saved_artifact_replay_provenance(&artifact, &replay)?;
        let tuple_inspection = inspect_options_expression_tuple_blocking(
            replay.normalized_req.clone(),
            req.tuple_index,
            replay.signal_manifest.as_ref(),
            replay.dataset.as_ref(),
        )?;
        let inspection = inspect_options_expression_event_path_blocking(
            replay.normalized_req,
            req.tuple_index,
            req.event_index,
            replay.signal_manifest.as_ref(),
            replay.dataset.as_ref(),
        )?;
        if inspection.row.event_id.trim().is_empty() {
            return Err("recomputed event path inspection returned an empty event id".to_string());
        }
        if inspection.row.status == ta_app_core::OptionsExpressionEventStatus::CandidateMissing
            && !inspection.points.is_empty()
        {
            return Err(
                "candidate-missing event path inspection returned path points unexpectedly"
                    .to_string(),
            );
        }
        validate_event_path_replay_consistency(&expected_tuple, &tuple_inspection, &inspection)?;
        Ok(inspection)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ta_app_core::{
        options_types::{
            ImportedOptionsDataset, OptionType, OptionsContractSeries, OptionsDatasetManifestV1,
            OptionsExpressionValidationDatasetPolicy, OptionsExpressionValidationPlanMode,
            OptionsExpressionValidationRequest, OptionsExpressionValidationTupleSelection,
            OptionsExpressionValidationVerdict, OptionsFillModelFamily, OptionsMoneynessBucket,
            OptionsPriceRow, OptionsStructureFamily, OptionsTupleSpec,
        },
        OptionsEvaluationSplit, OptionsExpressionArtifactV1, OptionsExpressionDiagnostics,
        OptionsExpressionFeasibilityReport, OptionsExpressionMetrics, OptionsExpressionRunRequest,
        OptionsExpressionTupleResult, OptionsPriceSemantics, OptionsSignalEvent,
        OptionsSignalEventManifestV1, SignalSide, OPTIONS_EXPRESSION_ARTIFACT_VERSION,
        OPTIONS_SIGNAL_MANIFEST_VERSION,
    };

    fn temp_app_local_data_dir(test_name: &str) -> std::path::PathBuf {
        let unique = format!(
            "ta_options_expr_cmds_{}_{}",
            test_name,
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn options_artifact_path_suggestion_fails_closed_when_slots_are_exhausted() {
        let base = temp_app_local_data_dir("artifact_path_slots");
        let runs = runs_dir(&base).expect("runs dir");
        std::fs::create_dir_all(&runs).expect("create runs dir");

        for prefix in [
            "vectorgrid_options_expression",
            "vectorgrid_options_validation",
        ] {
            let timestamp = format!("2026-07-28-{prefix}");
            let safe_timestamp = super::super::sanitize_filename_component(&timestamp);
            let base_name = format!("{prefix}_{safe_timestamp}");
            for suffix in [None, Some(2), Some(3)] {
                let file_name = match suffix {
                    None => format!("{base_name}.json"),
                    Some(suffix) => format!("{base_name}_{suffix}.json"),
                };
                std::fs::write(runs.join(file_name), b"occupied").expect("occupy artifact slot");
            }

            let error = suggest_options_json_artifact_path_with_limit(&base, &timestamp, prefix, 3)
                .expect_err("exhausted slots must not return an existing path");
            assert!(
                error.contains("no available options artifact filename slots"),
                "unexpected error: {error}"
            );
        }

        std::fs::remove_dir_all(base).expect("remove temp dir");
    }

    fn options_output_staging_siblings(target: &Path) -> Vec<PathBuf> {
        let file_name = target
            .file_name()
            .expect("target file name")
            .to_string_lossy();
        let prefix = format!(".{file_name}.");
        std::fs::read_dir(target.parent().expect("target parent"))
            .expect("read target parent")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_default();
                name.starts_with(&prefix) && name.ends_with(".tmp")
            })
            .collect()
    }

    #[cfg(windows)]
    fn create_test_file_symlink(
        link: &std::path::Path,
        target: &std::path::Path,
    ) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }

    #[cfg(unix)]
    fn create_test_file_symlink(
        link: &std::path::Path,
        target: &std::path::Path,
    ) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    fn sample_signal_manifest() -> OptionsSignalEventManifestV1 {
        OptionsSignalEventManifestV1 {
            version: OPTIONS_SIGNAL_MANIFEST_VERSION,
            manifest_id: "manifest".to_string(),
            parent_artifact_id: "parent_artifact".to_string(),
            parent_run_kind: "vectorgrid_run_artifact".to_string(),
            source_dataset_identity_hash: "identity".to_string(),
            source_content_fingerprint: "fingerprint".to_string(),
            dataset_hash: "dataset_hash".to_string(),
            timeframe: "1m".to_string(),
            resample_spec_hash: None,
            adjustment_mode: "raw".to_string(),
            symbol_root: "SPY".to_string(),
            calendar_id: "market_hours_utc".to_string(),
            runtime_dataset_id: None,
            events: vec![OptionsSignalEvent {
                event_id: "evt-1".to_string(),
                parent_artifact_id: "parent_artifact".to_string(),
                parent_run_kind: "vectorgrid_run_artifact".to_string(),
                source_dataset_identity_hash: "identity".to_string(),
                source_content_fingerprint: "fingerprint".to_string(),
                dataset_hash: "dataset_hash".to_string(),
                timeframe: "1m".to_string(),
                resample_spec_hash: None,
                adjustment_mode: "raw".to_string(),
                symbol_root: "SPY".to_string(),
                side: SignalSide::Bullish,
                decision_timestamp: 1_700_000_000_000,
                earliest_tradable_timestamp: 1_700_000_060_000,
                decision_bar_index: 0,
                earliest_tradable_bar_index: 1,
                raw_underlier_price: 100.0,
                adjusted_underlier_price: None,
                base_entry_timestamp: Some(1_700_000_060_000),
                base_exit_timestamp: Some(1_700_000_120_000),
                base_exit_bar_index: Some(2),
                fold: OptionsEvaluationSplit::Search,
                session_id: Some("2026-04-07".to_string()),
                regime_tag: None,
                confidence_tag: None,
                calendar_id: "market_hours_utc".to_string(),
            }],
        }
    }

    fn write_parent_strategy_artifact(
        temp_root: &std::path::Path,
        strategy_kind: &str,
    ) -> std::path::PathBuf {
        let parent_artifact_path = temp_root.join("vectorgrid_run_1.json");
        std::fs::write(
            &parent_artifact_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "run": {
                    "req": {
                        "strategy_kind": strategy_kind
                    }
                }
            }))
            .expect("serialize parent artifact"),
        )
        .expect("write parent artifact");
        parent_artifact_path
    }

    fn sample_summary_request(
        parent_artifact_path: &std::path::Path,
        options_dataset_id: &str,
        fill_model: OptionsFillModelFamily,
    ) -> OptionsExpressionRunRequest {
        serde_json::from_value(json!({
            "parent_artifact_path": parent_artifact_path.to_string_lossy(),
            "options_dataset_id": options_dataset_id,
            "fill_model": match fill_model {
                OptionsFillModelFamily::AggregateResearch => "aggregate_research",
                OptionsFillModelFamily::QuoteConservative => "quote_conservative",
                OptionsFillModelFamily::QuoteMid => "quote_mid",
            }
        }))
        .expect("request")
    }

    fn sample_summary_artifact(
        parent_artifact_path: &std::path::Path,
        options_dataset_id: &str,
        price_semantics: OptionsPriceSemantics,
        fill_model: OptionsFillModelFamily,
    ) -> OptionsExpressionArtifactV1 {
        OptionsExpressionArtifactV1 {
            version: OPTIONS_EXPRESSION_ARTIFACT_VERSION,
            kind: "vectorgrid_options_expression_v1".to_string(),
            ts: "2026-04-07T00:00:00Z".to_string(),
            artifact_path: "C:/runs/vectorgrid_options_expression_1.json".to_string(),
            parent_artifact_id: "vectorgrid_run_1".to_string(),
            parent_artifact_hash: "parent-hash-1".to_string(),
            root_symbol: "SPY".to_string(),
            options_dataset_id: options_dataset_id.to_string(),
            options_dataset_hash: "hash-1".to_string(),
            options_dataset_normalized_rows_sha256: Some(format!(
                "rows-sha256-{options_dataset_id}"
            )),
            options_dataset_manifest_semantic_sha256: Some(format!(
                "manifest-semantic-sha256-{options_dataset_id}"
            )),
            signal_manifest_id: sample_signal_manifest().manifest_id,
            dataset_provenance: Some(dataset_provenance_from_imported_dataset(
                &sample_replay_dataset(options_dataset_id, price_semantics),
            )),
            backend_used: "cpu_reference".to_string(),
            backend_fallback_reason: None,
            price_semantics,
            exactness_semantics: Some(
                price_semantics.exactness_semantics_for_backend("cpu_reference"),
            ),
            request: sample_summary_request(parent_artifact_path, options_dataset_id, fill_model),
            diagnostics: OptionsExpressionDiagnostics {
                parent_artifact_path: parent_artifact_path.to_string_lossy().to_string(),
                options_dataset_id: options_dataset_id.to_string(),
                root_symbol: "SPY".to_string(),
                total_events: 10,
                bullish_events: 6,
                bearish_events: 4,
                search_events: 6,
                holdout_events: 2,
                sealed_test_events: 2,
                dataset_contracts: 3,
                dataset_rows: 42,
                coverage_start: None,
                coverage_end: None,
            },
            staging: None,
            overlap_window: ta_app_core::options_types::OptionsExpressionOverlapWindowSummary {
                contract_reference_as_of_utc: Some("2026-04-06T00:00:00Z".to_string()),
                total_events_in_dataset_window: 10,
                search_events_in_dataset_window: 6,
                holdout_events_in_dataset_window: 2,
                sealed_test_events_in_dataset_window: 2,
                total_events_before_dataset_window: 0,
                total_events_after_dataset_window: 0,
                search_events_before_dataset_window: 0,
                search_events_after_dataset_window: 0,
                holdout_events_before_dataset_window: 0,
                holdout_events_after_dataset_window: 0,
                sealed_test_events_before_dataset_window: 0,
                sealed_test_events_after_dataset_window: 0,
                total_events_before_contract_reference_as_of: 0,
                search_events_before_contract_reference_as_of: 0,
                holdout_events_before_contract_reference_as_of: 0,
                sealed_test_events_before_contract_reference_as_of: 0,
                total_overlap_rate: 1.0,
                search_overlap_rate: 1.0,
                holdout_overlap_rate: 1.0,
                sealed_test_overlap_rate: 1.0,
            },
            coverage_adequacy:
                ta_app_core::options_types::OptionsExpressionCoverageAdequacySummary {
                    verdict: OptionsExpressionCoverageAdequacyVerdict::Healthy,
                    non_promotable: false,
                    reasons: vec!["Overlap and split coverage are currently adequate.".to_string()],
                    search_candidate_coverage_rate: 1.0,
                    holdout_candidate_coverage_rate: 1.0,
                    sealed_test_candidate_coverage_rate: 1.0,
                    search_baseline_outcome_coverage_rate: 1.0,
                    holdout_baseline_outcome_coverage_rate: 1.0,
                    sealed_test_baseline_outcome_coverage_rate: 1.0,
                    dominant_invalid_reason: None,
                    dominant_invalid_reason_share: 0.0,
                },
            split_invalid_reasons:
                ta_app_core::options_types::OptionsExpressionSplitInvalidReasonSummary {
                    search_invalid_reasons: vec![],
                    holdout_invalid_reasons: vec![],
                    sealed_test_invalid_reasons: vec![],
                },
            selection_metrics: vec![],
        }
    }

    fn sample_summary_tuple(
        delay_minutes: u16,
        robust_score: f64,
        selection_expectancy: f64,
        holdout_expectancy: Option<f64>,
        sealed_expectancy: Option<f64>,
        stress_expectancy: Option<f64>,
        sealed_finalist: bool,
        sealed_final_rank: usize,
    ) -> OptionsExpressionTupleResult {
        let metrics = |expectancy_per_premium: f64| OptionsExpressionMetrics {
            valid_trades: 5,
            invalid_events: 0,
            invalid_rate: 0.0,
            expectancy_per_trade: expectancy_per_premium * 100.0,
            expectancy_per_premium,
            mean_return_on_premium: expectancy_per_premium,
            return_volatility_on_premium: 0.0,
            median_return_on_premium: expectancy_per_premium / 2.0,
            win_rate: 0.6,
            total_pnl_per_contract: expectancy_per_premium * 400.0,
        };
        OptionsExpressionTupleResult {
            tuple: OptionsTupleSpec {
                structure_family: OptionsStructureFamily::LongSingle,
                delay_minutes,
                dte_days: 1,
                moneyness_bucket: OptionsMoneynessBucket::Atm,
                hold_minutes: 15,
                target_return: Some(0.25),
                stop_return: Some(0.25),
                exit_template: ta_app_core::OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                fill_model: OptionsFillModelFamily::AggregateResearch,
            },
            pre_sealed_rank: delay_minutes as usize,
            sealed_finalist,
            sealed_final_rank,
            selection_metrics: metrics(selection_expectancy),
            holdout_metrics: holdout_expectancy.map(metrics),
            sealed_evaluation_metrics: sealed_expectancy.map(metrics),
            stress_metrics: stress_expectancy.map(metrics),
            sealed_stress_metrics: None,
            harsher_stress_metrics: None,
            sealed_harsher_stress_metrics: None,
            extreme_stress_metrics: None,
            sealed_extreme_stress_metrics: None,
            validation_verdict: if sealed_finalist {
                OptionsExpressionValidationVerdict::SealedFinalist
            } else {
                OptionsExpressionValidationVerdict::HoldoutSupported
            },
            objective_score_components: None,
            robust_score,
        }
    }

    fn sample_feasibility_report() -> OptionsExpressionFeasibilityReport {
        OptionsExpressionFeasibilityReport {
            diagnostics: OptionsExpressionDiagnostics {
                parent_artifact_path: "artifact.json".to_string(),
                options_dataset_id: "dataset".to_string(),
                root_symbol: "SPY".to_string(),
                total_events: 10,
                bullish_events: 10,
                bearish_events: 0,
                search_events: 6,
                holdout_events: 2,
                sealed_test_events: 2,
                dataset_contracts: 3,
                dataset_rows: 42,
                coverage_start: None,
                coverage_end: None,
            },
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            exactness_semantics: OptionsPriceSemantics::MinuteAggregate
                .exactness_semantics_for_backend("cpu_reference"),
            objective_config: Default::default(),
            fill_model: OptionsFillModelFamily::AggregateResearch,
            best_available_backend: "cpu_reference".to_string(),
            auto_backend_decision: Default::default(),
            compiled_tuple_count: 1,
            per_event_candidate_slots: 1,
            per_candidate_exit_summaries: 1,
            candidate_slot_count: 1,
            compiled_candidate_count: 1,
            candidate_density: 1.0,
            invalid_slot_count: 0,
            invalid_slot_rate: 0.0,
            baseline_exit_summary_count: 1,
            baseline_compiled_exit_summary_count: 1,
            baseline_exit_summary_density: 1.0,
            conservative_exit_summary_count: 1,
            conservative_compiled_exit_summary_count: 1,
            conservative_exit_summary_density: 1.0,
            harsher_exit_summary_count: 1,
            harsher_compiled_exit_summary_count: 1,
            harsher_exit_summary_density: 1.0,
            projected_event_tuple_evaluations: 1,
            structural_cache_bytes_estimate: 0,
            baseline_summary_bytes_estimate: 0,
            conservative_summary_bytes_estimate: 0,
            harsher_summary_bytes_estimate: 0,
            timings: Default::default(),
            event_coverage: Default::default(),
            overlap_window: Default::default(),
            split_invalid_reasons: Default::default(),
            coverage_adequacy: Default::default(),
            cache_stats: Default::default(),
            invalid_reasons: Vec::new(),
            axis_coverage: Default::default(),
        }
    }

    #[test]
    fn signal_manifest_replay_cache_round_trips_to_disk() {
        let app_local_data_dir = temp_app_local_data_dir("signal_manifest_cache_round_trip");
        let manifest = sample_signal_manifest();
        let cache_key = "artifact|source=csv|timeframe=1m|candidate=none";

        persist_signal_manifest_replay_cache(
            &app_local_data_dir,
            "parent_artifact",
            cache_key,
            &manifest,
        )
        .expect("persist cache");
        let loaded =
            load_signal_manifest_replay_cache(&app_local_data_dir, "parent_artifact", cache_key)
                .expect("load cache")
                .expect("cached manifest");

        assert_eq!(loaded.manifest_id, manifest.manifest_id);
        assert_eq!(loaded.events.len(), 1);
        assert_eq!(loaded.events[0].event_id, "evt-1");

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn signal_manifest_replay_cache_treats_corrupt_payload_as_cache_miss() {
        let app_local_data_dir = temp_app_local_data_dir("signal_manifest_cache_corrupt");
        let cache_key = "artifact|source=csv|timeframe=1m|candidate=none";
        let path =
            signal_manifest_replay_cache_path(&app_local_data_dir, "parent_artifact", cache_key)
                .expect("cache path");
        std::fs::write(&path, br#"{"manifest_id":"broken""#).expect("write corrupt cache");

        let loaded =
            load_signal_manifest_replay_cache(&app_local_data_dir, "parent_artifact", cache_key)
                .expect("load corrupt cache");

        assert!(loaded.is_none());
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn options_dataset_replay_cache_round_trips_to_disk() {
        let app_local_data_dir = temp_app_local_data_dir("options_dataset_cache_round_trip");
        let dataset_id = "spy_dataset";
        let cache_key = "manifest-stamp|rows-stamp";
        let options_dataset_hash = "dataset_hash";
        let dataset = sample_replay_dataset(dataset_id, OptionsPriceSemantics::MinuteAggregate);

        persist_options_dataset_replay_cache(
            &app_local_data_dir,
            dataset_id,
            cache_key,
            options_dataset_hash,
            &dataset,
        )
        .expect("persist cache");
        let (loaded_hash, loaded_dataset) =
            load_options_dataset_replay_cache(&app_local_data_dir, dataset_id, cache_key)
                .expect("load cache")
                .expect("cached dataset");

        assert_eq!(loaded_hash, options_dataset_hash);
        assert_eq!(loaded_dataset.manifest.options_dataset_id, dataset_id);
        assert_eq!(
            loaded_dataset.manifest.price_semantics,
            OptionsPriceSemantics::MinuteAggregate
        );
        assert_eq!(
            loaded_dataset.manifest.row_count,
            dataset.manifest.row_count
        );
        assert_eq!(loaded_dataset.contracts.len(), 1);
        assert_eq!(loaded_dataset.contracts[0].rows.len(), 2);
        assert_eq!(loaded_dataset.contracts[0].rows[0].quote_age_ms, None);
        assert_eq!(
            loaded_dataset.contracts[0].rows[1].session_id.as_deref(),
            Some("2026-04-03-rth")
        );

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn options_dataset_replay_cache_loads_legacy_json_payload() {
        let app_local_data_dir = temp_app_local_data_dir("options_dataset_cache_legacy_json");
        let dataset_id = "spy_dataset";
        let cache_key = "manifest-stamp|rows-stamp";
        let path = options_dataset_replay_cache_legacy_json_path(
            &app_local_data_dir,
            dataset_id,
            cache_key,
        )
        .expect("legacy cache path");
        let binary_path =
            options_dataset_replay_cache_path(&app_local_data_dir, dataset_id, cache_key)
                .expect("binary cache path");
        let dataset = sample_replay_dataset(dataset_id, OptionsPriceSemantics::MinuteAggregate);
        let payload = PersistedOptionsDatasetReplayCacheV1 {
            version: OPTIONS_DATASET_REPLAY_CACHE_VERSION,
            options_dataset_hash: "dataset_hash".to_string(),
            dataset: (&dataset).into(),
        };
        std::fs::write(
            &path,
            serde_json::to_vec(&payload).expect("serialize legacy payload"),
        )
        .expect("write legacy cache");

        let (loaded_hash, loaded_dataset) =
            load_options_dataset_replay_cache(&app_local_data_dir, dataset_id, cache_key)
                .expect("load cache")
                .expect("cached dataset");

        assert_eq!(loaded_hash, "dataset_hash");
        assert_eq!(loaded_dataset.manifest.options_dataset_id, dataset_id);
        assert!(binary_path.is_file());

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn options_dataset_replay_cache_ignores_version_mismatch() {
        let app_local_data_dir = temp_app_local_data_dir("options_dataset_cache_version_mismatch");
        let dataset_id = "spy_dataset";
        let cache_key = "manifest-stamp|rows-stamp";
        let path = options_dataset_replay_cache_path(&app_local_data_dir, dataset_id, cache_key)
            .expect("cache path");
        let stale_payload = PersistedOptionsDatasetReplayCacheV1 {
            version: OPTIONS_DATASET_REPLAY_CACHE_VERSION + 1,
            options_dataset_hash: "dataset_hash".to_string(),
            dataset: (&sample_replay_dataset(dataset_id, OptionsPriceSemantics::MinuteAggregate))
                .into(),
        };
        let file = std::fs::File::create(&path).expect("create stale cache");
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &stale_payload).expect("serialize stale payload");

        let loaded = load_options_dataset_replay_cache(&app_local_data_dir, dataset_id, cache_key)
            .expect("load cache");
        assert!(loaded.is_none());

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn options_dataset_replay_cache_ignores_corrupt_binary_and_uses_legacy_json() {
        let app_local_data_dir = temp_app_local_data_dir("options_dataset_cache_corrupt_binary");
        let dataset_id = "spy_dataset";
        let cache_key = "manifest-stamp|rows-stamp";
        let binary_path =
            options_dataset_replay_cache_path(&app_local_data_dir, dataset_id, cache_key)
                .expect("binary cache path");
        let legacy_path = options_dataset_replay_cache_legacy_json_path(
            &app_local_data_dir,
            dataset_id,
            cache_key,
        )
        .expect("legacy cache path");
        let dataset = sample_replay_dataset(dataset_id, OptionsPriceSemantics::MinuteAggregate);
        let payload = PersistedOptionsDatasetReplayCacheV1 {
            version: OPTIONS_DATASET_REPLAY_CACHE_VERSION,
            options_dataset_hash: "dataset_hash".to_string(),
            dataset: (&dataset).into(),
        };
        std::fs::write(&binary_path, [0x01, 0x02, 0x03, 0x04]).expect("write corrupt binary");
        std::fs::write(
            &legacy_path,
            serde_json::to_vec(&payload).expect("serialize legacy payload"),
        )
        .expect("write legacy cache");

        let (loaded_hash, loaded_dataset) =
            load_options_dataset_replay_cache(&app_local_data_dir, dataset_id, cache_key)
                .expect("load cache")
                .expect("cached dataset");

        assert_eq!(loaded_hash, "dataset_hash");
        assert_eq!(loaded_dataset.manifest.options_dataset_id, dataset_id);
        assert_eq!(loaded_dataset.contracts.len(), 1);

        let reread = load_options_dataset_replay_cache(&app_local_data_dir, dataset_id, cache_key)
            .expect("reload cache")
            .expect("cached dataset after rewrite");
        assert_eq!(reread.0, "dataset_hash");
        assert_eq!(reread.1.contracts[0].rows.len(), 2);

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn analyze_feasibility_response_serializes_replay_build_timing_presence_and_absence() {
        let with_timing = AnalyzeOptionsExpressionFeasibilityResponse {
            replay_build_ms: 12,
            total_preview_ms: 34,
            replay_build_timing: Some(OptionsExpressionReplayBuildTimingBreakdown {
                total_us: 12_000,
                options_dataset_persisted_cache_hit: true,
                options_dataset_csv_load_us: 0,
                ..Default::default()
            }),
            report: sample_feasibility_report(),
        };
        let with_timing_json = serde_json::to_value(&with_timing).expect("serialize response");
        assert_eq!(with_timing_json["replay_build_ms"], 12);
        assert_eq!(with_timing_json["total_preview_ms"], 34);
        assert_eq!(
            with_timing_json["replay_build_timing"]["options_dataset_persisted_cache_hit"],
            true
        );
        assert_eq!(
            with_timing_json["replay_build_timing"]["options_dataset_csv_load_us"],
            0
        );

        let without_timing = AnalyzeOptionsExpressionFeasibilityResponse {
            replay_build_ms: 12,
            total_preview_ms: 34,
            replay_build_timing: None,
            report: sample_feasibility_report(),
        };
        let without_timing_json =
            serde_json::to_value(&without_timing).expect("serialize response");
        let object = without_timing_json
            .as_object()
            .expect("response serialized as object");
        assert!(!object.contains_key("replay_build_timing"));
    }

    #[test]
    fn options_parent_strategy_kind_support_matches_current_double_like_scope() {
        assert!(
            crate::commands::options_parent_adapters::is_supported_options_parent_strategy_kind(
                "DoubleMa"
            )
        );
        assert!(
            crate::commands::options_parent_adapters::is_supported_options_parent_strategy_kind(
                "CompositeTrend"
            )
        );
        assert!(
            crate::commands::options_parent_adapters::is_supported_options_parent_strategy_kind(
                "RegimeHybrid"
            )
        );
        assert!(
            crate::commands::options_parent_adapters::is_supported_options_parent_strategy_kind(
                "BreakoutMomentum"
            )
        );
        assert!(
            !crate::commands::options_parent_adapters::is_supported_options_parent_strategy_kind(
                "MlSignal"
            )
        );
        assert!(
            !crate::commands::options_parent_adapters::is_supported_options_parent_strategy_kind(
                "CompositeMo"
            )
        );
    }

    #[test]
    fn successful_options_output_csv_save_atomically_replaces_existing_target() {
        let temp_root = temp_app_local_data_dir("save_options_csv");
        let output_path = temp_root.join("exports").join("options_results.csv");
        std::fs::create_dir_all(output_path.parent().expect("export parent"))
            .expect("create export parent");
        std::fs::write(&output_path, "old,valid\r\n").expect("write existing valid CSV");
        let req = SaveOptionsCsvRequest {
            path: output_path.to_string_lossy().to_string(),
            csv_text: "col_a,col_b\r\n1,2\r\n".to_string(),
        };

        let response = save_options_csv_blocking_headless(req).expect("save csv");

        assert_eq!(response.path, output_path.to_string_lossy());
        assert_eq!(response.bytes, "col_a,col_b\r\n1,2\r\n".len());
        let saved = std::fs::read_to_string(&output_path).expect("read saved csv");
        assert_eq!(saved, "col_a,col_b\r\n1,2\r\n");
        assert!(options_output_staging_siblings(&output_path).is_empty());

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn cancelled_options_output_preserves_existing_target_and_cleans_staging() {
        let temp_root = temp_app_local_data_dir("atomic_output_cancelled");
        let output_path = temp_root.join("artifact.json");
        std::fs::write(&output_path, b"{\"valid\":true}\n").expect("write existing artifact");

        let error = write_options_output_with_publish(
            &output_path,
            "test artifact",
            |writer| {
                writer
                    .write_all(b"{\"partial\":")
                    .map_err(|error| error.to_string())?;
                assert_eq!(options_output_staging_siblings(&output_path).len(), 1);
                Err("cancelled".to_string())
            },
            publish_options_output,
        )
        .expect_err("cancelled output must fail");

        assert_eq!(error, "cancelled");
        assert_eq!(
            std::fs::read(&output_path).expect("read preserved artifact"),
            b"{\"valid\":true}\n"
        );
        assert!(options_output_staging_siblings(&output_path).is_empty());

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn pre_cancelled_options_expression_phase_fails_closed() {
        let cancel = AtomicBool::new(true);
        let error = ensure_options_expression_not_cancelled(Some(&cancel), "artifact publication")
            .expect_err("pre-cancelled phase must not continue");

        assert_eq!(
            error,
            "options expression operation cancelled before artifact publication"
        );
    }

    #[test]
    fn failed_options_output_publish_preserves_existing_target_and_cleans_staging() {
        let temp_root = temp_app_local_data_dir("atomic_output_publish_failure");
        let output_path = temp_root.join("artifact.json");
        std::fs::write(&output_path, b"{\"valid\":true}\n").expect("write existing artifact");

        let error = write_options_output_with_publish(
            &output_path,
            "test artifact",
            |writer| {
                writer
                    .write_all(b"{\"replacement\":true}\n")
                    .map_err(|error| error.to_string())
            },
            |_staged_path, _final_path, _target_exists| {
                Err("simulated publication failure".to_string())
            },
        )
        .expect_err("publication must fail");

        assert_eq!(error, "simulated publication failure");
        assert_eq!(
            std::fs::read(&output_path).expect("read preserved artifact"),
            b"{\"valid\":true}\n"
        );
        assert!(options_output_staging_siblings(&output_path).is_empty());

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[cfg(windows)]
    #[test]
    fn locked_options_output_replacement_preserves_existing_target_and_cleans_staging() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

        let temp_root = temp_app_local_data_dir("atomic_output_locked_target");
        let output_path = temp_root.join("artifact.json");
        let previous = b"{\"valid\":true}\n";
        std::fs::write(&output_path, previous).expect("write existing artifact");
        let locked_target = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&output_path)
            .expect("open target without delete sharing");

        let error = write_options_output_bytes_atomically(
            &output_path,
            b"{\"replacement\":true}\n",
            "test artifact",
        )
        .expect_err("locked target must reject replacement");

        assert!(error.contains("failed to atomically replace options output"));
        assert_eq!(
            std::fs::read(&output_path).expect("read preserved target"),
            previous
        );
        assert!(options_output_staging_siblings(&output_path).is_empty());

        drop(locked_target);
        std::fs::remove_dir_all(temp_root).expect("remove test directory");
    }

    #[test]
    fn save_options_csv_blocking_headless_rejects_non_csv_paths() {
        let temp_root = temp_app_local_data_dir("save_options_csv_rejects_non_csv");
        let output_path = temp_root.join("exports").join("options_results.txt");
        let req = SaveOptionsCsvRequest {
            path: output_path.to_string_lossy().to_string(),
            csv_text: "col_a,col_b\r\n1,2\r\n".to_string(),
        };

        let err = save_options_csv_blocking_headless(req).expect_err("non-csv path");

        assert!(err.contains(".csv"));
        assert!(!output_path.exists());

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn summarize_options_expression_artifact_reports_validation_verdict_counts() {
        let temp_root = std::env::temp_dir().join(format!(
            "vectorgrid_options_summary_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let parent_artifact_path = temp_root.join("vectorgrid_run_1.json");
        std::fs::write(
            &parent_artifact_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "run": {
                    "req": {
                        "strategy_kind": "DoubleMa"
                    }
                }
            }))
            .expect("serialize parent artifact"),
        )
        .expect("write parent artifact");
        let request: OptionsExpressionRunRequest = serde_json::from_value(json!({
            "parent_artifact_path": parent_artifact_path.to_string_lossy(),
            "options_dataset_id": "spy_options"
        }))
        .expect("request");
        let tuple = OptionsExpressionTupleResult {
            tuple: OptionsTupleSpec {
                structure_family: OptionsStructureFamily::LongSingle,
                delay_minutes: 0,
                dte_days: 1,
                moneyness_bucket: OptionsMoneynessBucket::Atm,
                hold_minutes: 15,
                target_return: Some(0.25),
                stop_return: Some(0.25),
                exit_template: ta_app_core::OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                fill_model: OptionsFillModelFamily::AggregateResearch,
            },
            pre_sealed_rank: 1,
            sealed_finalist: true,
            sealed_final_rank: 1,
            selection_metrics: OptionsExpressionMetrics {
                valid_trades: 5,
                invalid_events: 1,
                invalid_rate: 0.1667,
                expectancy_per_trade: 8.0,
                expectancy_per_premium: 0.20,
                mean_return_on_premium: 0.16,
                return_volatility_on_premium: 0.05,
                median_return_on_premium: 0.15,
                win_rate: 0.60,
                total_pnl_per_contract: 40.0,
            },
            holdout_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade: 4.0,
                expectancy_per_premium: 0.09,
                mean_return_on_premium: 0.08,
                return_volatility_on_premium: 0.02,
                median_return_on_premium: 0.07,
                win_rate: 0.50,
                total_pnl_per_contract: 8.0,
            }),
            sealed_evaluation_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade: 4.0,
                expectancy_per_premium: 0.09,
                mean_return_on_premium: 0.08,
                return_volatility_on_premium: 0.02,
                median_return_on_premium: 0.07,
                win_rate: 0.50,
                total_pnl_per_contract: 8.0,
            }),
            stress_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade: 2.0,
                expectancy_per_premium: 0.04,
                mean_return_on_premium: 0.035,
                return_volatility_on_premium: 0.01,
                median_return_on_premium: 0.03,
                win_rate: 0.50,
                total_pnl_per_contract: 4.0,
            }),
            sealed_stress_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade: 2.0,
                expectancy_per_premium: 0.03,
                mean_return_on_premium: 0.025,
                return_volatility_on_premium: 0.01,
                median_return_on_premium: 0.02,
                win_rate: 0.50,
                total_pnl_per_contract: 4.0,
            }),
            harsher_stress_metrics: None,
            sealed_harsher_stress_metrics: None,
            extreme_stress_metrics: None,
            sealed_extreme_stress_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade: -1.0,
                expectancy_per_premium: -0.02,
                mean_return_on_premium: -0.015,
                return_volatility_on_premium: 0.01,
                median_return_on_premium: -0.01,
                win_rate: 0.50,
                total_pnl_per_contract: -2.0,
            }),
            validation_verdict: OptionsExpressionValidationVerdict::SearchOnly,
            objective_score_components: None,
            robust_score: 0.14,
        };
        let artifact = OptionsExpressionArtifactV1 {
            version: OPTIONS_EXPRESSION_ARTIFACT_VERSION,
            kind: "vectorgrid_options_expression_v1".to_string(),
            ts: "2026-04-07T00:00:00Z".to_string(),
            artifact_path: "C:/runs/vectorgrid_options_expression_1.json".to_string(),
            parent_artifact_id: "vectorgrid_run_1".to_string(),
            parent_artifact_hash: "parent-hash-1".to_string(),
            root_symbol: "SPY".to_string(),
            options_dataset_id: "spy_options".to_string(),
            options_dataset_hash: "hash-1".to_string(),
            options_dataset_normalized_rows_sha256: Some("rows-sha256-spy_options".to_string()),
            options_dataset_manifest_semantic_sha256: Some(
                "manifest-semantic-sha256-spy_options".to_string(),
            ),
            signal_manifest_id: sample_signal_manifest().manifest_id,
            dataset_provenance: Some(dataset_provenance_from_imported_dataset(
                &sample_replay_dataset("spy_options", OptionsPriceSemantics::MinuteAggregate),
            )),
            backend_used: "cpu_reference".to_string(),
            backend_fallback_reason: None,
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            exactness_semantics: Some(
                OptionsPriceSemantics::MinuteAggregate
                    .exactness_semantics_for_backend("cpu_reference"),
            ),
            request,
            diagnostics: OptionsExpressionDiagnostics {
                parent_artifact_path: "C:/runs/vectorgrid_run_1.json".to_string(),
                options_dataset_id: "spy_options".to_string(),
                root_symbol: "SPY".to_string(),
                total_events: 10,
                bullish_events: 6,
                bearish_events: 4,
                search_events: 6,
                holdout_events: 2,
                sealed_test_events: 2,
                dataset_contracts: 3,
                dataset_rows: 42,
                coverage_start: None,
                coverage_end: None,
            },
            staging: None,
            overlap_window: ta_app_core::options_types::OptionsExpressionOverlapWindowSummary {
                contract_reference_as_of_utc: Some("2026-04-06T00:00:00Z".to_string()),
                total_events_in_dataset_window: 10,
                search_events_in_dataset_window: 6,
                holdout_events_in_dataset_window: 2,
                sealed_test_events_in_dataset_window: 2,
                total_events_before_dataset_window: 0,
                total_events_after_dataset_window: 0,
                search_events_before_dataset_window: 0,
                search_events_after_dataset_window: 0,
                holdout_events_before_dataset_window: 0,
                holdout_events_after_dataset_window: 0,
                sealed_test_events_before_dataset_window: 0,
                sealed_test_events_after_dataset_window: 0,
                total_events_before_contract_reference_as_of: 0,
                search_events_before_contract_reference_as_of: 0,
                holdout_events_before_contract_reference_as_of: 0,
                sealed_test_events_before_contract_reference_as_of: 0,
                total_overlap_rate: 1.0,
                search_overlap_rate: 1.0,
                holdout_overlap_rate: 1.0,
                sealed_test_overlap_rate: 1.0,
            },
            coverage_adequacy:
                ta_app_core::options_types::OptionsExpressionCoverageAdequacySummary {
                    verdict: OptionsExpressionCoverageAdequacyVerdict::Healthy,
                    non_promotable: false,
                    reasons: vec!["Overlap and split coverage are currently adequate.".to_string()],
                    search_candidate_coverage_rate: 1.0,
                    holdout_candidate_coverage_rate: 1.0,
                    sealed_test_candidate_coverage_rate: 1.0,
                    search_baseline_outcome_coverage_rate: 1.0,
                    holdout_baseline_outcome_coverage_rate: 1.0,
                    sealed_test_baseline_outcome_coverage_rate: 1.0,
                    dominant_invalid_reason: None,
                    dominant_invalid_reason_share: 0.0,
                },
            split_invalid_reasons:
                ta_app_core::options_types::OptionsExpressionSplitInvalidReasonSummary {
                    search_invalid_reasons: vec![
                        ta_app_core::options_types::OptionsCandidateInvalidReasonCount {
                            reason:
                                ta_app_core::OptionsCandidateInvalidReason::NoExpirationInBucket,
                            count: 1,
                        },
                    ],
                    holdout_invalid_reasons: vec![],
                    sealed_test_invalid_reasons: vec![],
                },
            selection_metrics: vec![tuple],
        };

        let summary = summarize_options_expression_artifact(
            std::path::Path::new("C:/runs/vectorgrid_options_expression_1.json"),
            1024,
            &artifact,
        );

        assert_eq!(
            summary.top_validation_verdict,
            OptionsExpressionValidationVerdict::StressFragile
        );
        assert_eq!(summary.parent_strategy_kind, "DoubleMa");
        assert_eq!(summary.request_backend_preference, "auto");
        assert_eq!(summary.request_objective_profile, "balanced");
        assert_eq!(summary.request_fill_model, "aggregate_research");
        assert_eq!(summary.request_structure_family, "long_single");
        assert_eq!(summary.request_split_assignment_mode, "event_chronological");
        assert_eq!(
            summary.coverage_adequacy_verdict,
            OptionsExpressionCoverageAdequacyVerdict::Healthy
        );
        assert!(!summary.coverage_non_promotable);
        assert_eq!(summary.search_overlap_rate, 1.0);
        assert_eq!(summary.search_candidate_coverage_rate, 1.0);
        assert_eq!(summary.search_baseline_outcome_coverage_rate, 1.0);
        assert_eq!(summary.validated_tuple_count, 1);
        assert_eq!(summary.stress_fragile_tuple_count, 1);
        assert_eq!(summary.supported_holdout_tuple_count, 1);
        assert_eq!(summary.supported_sealed_tuple_count, 1);
        assert_eq!(summary.supported_stress_tuple_count, 1);

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir(&temp_root);
    }

    #[test]
    fn summarize_options_expression_artifact_preserves_all_negative_maxima() {
        let temp_root = temp_app_local_data_dir("summary_all_negative_maxima");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let mut artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.selection_metrics = vec![
            sample_summary_tuple(0, -0.40, -0.20, Some(-0.12), None, None, false, 0),
            sample_summary_tuple(1, -0.25, -0.35, Some(-0.18), None, None, false, 0),
        ];

        let summary = summarize_options_expression_artifact(
            std::path::Path::new("C:/runs/vectorgrid_options_expression_negative.json"),
            1024,
            &artifact,
        );

        assert_eq!(summary.best_robust_score, -0.25);
        assert_eq!(summary.best_selection_expectancy_per_premium, -0.20);

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn load_options_expression_artifact_blocking_headless_refreshes_tuple_verdicts() {
        let app_local_data_dir = temp_app_local_data_dir("artifact_validation_refresh");
        let runs = runs_dir(&app_local_data_dir).expect("runs dir");
        let parent_artifact_path = write_parent_strategy_artifact(&app_local_data_dir, "DoubleMa");
        let artifact_path = runs.join("vectorgrid_options_expression_legacy.json");
        let mut artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.artifact_path = artifact_path.to_string_lossy().to_string();
        artifact.selection_metrics = vec![OptionsExpressionTupleResult {
            tuple: OptionsTupleSpec {
                structure_family: OptionsStructureFamily::LongSingle,
                delay_minutes: 0,
                dte_days: 1,
                moneyness_bucket: OptionsMoneynessBucket::Atm,
                hold_minutes: 15,
                target_return: Some(0.25),
                stop_return: Some(0.25),
                exit_template: ta_app_core::OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                fill_model: OptionsFillModelFamily::AggregateResearch,
            },
            pre_sealed_rank: 1,
            sealed_finalist: true,
            sealed_final_rank: 1,
            selection_metrics: OptionsExpressionMetrics {
                valid_trades: 5,
                invalid_events: 1,
                invalid_rate: 0.1667,
                expectancy_per_trade: 8.0,
                expectancy_per_premium: 0.20,
                mean_return_on_premium: 0.16,
                return_volatility_on_premium: 0.05,
                median_return_on_premium: 0.15,
                win_rate: 0.60,
                total_pnl_per_contract: 40.0,
            },
            holdout_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 0,
                invalid_events: 2,
                invalid_rate: 1.0,
                expectancy_per_trade: 0.0,
                expectancy_per_premium: 0.0,
                mean_return_on_premium: 0.0,
                return_volatility_on_premium: 0.0,
                median_return_on_premium: 0.0,
                win_rate: 0.0,
                total_pnl_per_contract: 0.0,
            }),
            sealed_evaluation_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 0,
                invalid_events: 1,
                invalid_rate: 1.0,
                expectancy_per_trade: 0.0,
                expectancy_per_premium: 0.0,
                mean_return_on_premium: 0.0,
                return_volatility_on_premium: 0.0,
                median_return_on_premium: 0.0,
                win_rate: 0.0,
                total_pnl_per_contract: 0.0,
            }),
            stress_metrics: Some(OptionsExpressionMetrics {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade: 2.0,
                expectancy_per_premium: 0.04,
                mean_return_on_premium: 0.035,
                return_volatility_on_premium: 0.01,
                median_return_on_premium: 0.03,
                win_rate: 0.50,
                total_pnl_per_contract: 4.0,
            }),
            sealed_stress_metrics: None,
            harsher_stress_metrics: None,
            sealed_harsher_stress_metrics: None,
            extreme_stress_metrics: None,
            sealed_extreme_stress_metrics: None,
            validation_verdict: OptionsExpressionValidationVerdict::SealedFinalist,
            objective_score_components: None,
            robust_score: 0.14,
        }];
        std::fs::write(
            &artifact_path,
            serde_json::to_vec_pretty(&artifact).expect("serialize artifact"),
        )
        .expect("write artifact");

        let loaded =
            load_options_expression_artifact_blocking_headless(&artifact_path, &app_local_data_dir)
                .expect("load artifact");

        assert_eq!(
            loaded.selection_metrics[0].validation_verdict,
            OptionsExpressionValidationVerdict::SearchOnly
        );

        let _ = std::fs::remove_dir_all(app_local_data_dir);
    }

    #[test]
    fn listing_options_expression_artifacts_does_not_reopen_parent_artifacts_outside_runs_dir() {
        let app_local_data_dir = temp_app_local_data_dir("options_artifact_parent_boundary");
        let runs = runs_dir(&app_local_data_dir).expect("runs dir");
        let external_parent_dir = temp_app_local_data_dir("options_artifact_parent_external");
        let external_parent_artifact =
            write_parent_strategy_artifact(&external_parent_dir, "DoubleMa");
        let artifact_path = runs.join("vectorgrid_options_expression_boundary.json");
        let mut artifact = sample_summary_artifact(
            &external_parent_artifact,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.artifact_path = artifact_path.to_string_lossy().to_string();
        std::fs::write(
            &artifact_path,
            serde_json::to_vec_pretty(&artifact).expect("serialize artifact"),
        )
        .expect("write artifact");

        let listed =
            list_options_expression_artifacts_blocking_headless(&app_local_data_dir).expect("list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].path, artifact_path.to_string_lossy());
        assert!(listed[0].parent_strategy_kind.is_empty());

        let _ = std::fs::remove_dir_all(app_local_data_dir);
        let _ = std::fs::remove_dir_all(external_parent_dir);
    }

    #[test]
    fn listing_options_expression_artifacts_skips_symlinked_entries_outside_runs_dir() {
        let app_local_data_dir = temp_app_local_data_dir("options_artifact_symlink_boundary");
        let runs = runs_dir(&app_local_data_dir).expect("runs dir");
        let external_dir = temp_app_local_data_dir("options_artifact_symlink_external");
        let external_parent_artifact = write_parent_strategy_artifact(&external_dir, "DoubleMa");
        let external_artifact_path =
            external_dir.join("vectorgrid_options_expression_outside.json");
        let mut artifact = sample_summary_artifact(
            &external_parent_artifact,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.artifact_path = external_artifact_path.to_string_lossy().to_string();
        std::fs::write(
            &external_artifact_path,
            serde_json::to_vec_pretty(&artifact).expect("serialize external artifact"),
        )
        .expect("write external artifact");
        let linked_path = runs.join("vectorgrid_options_expression_symlink.json");
        if create_test_file_symlink(&linked_path, &external_artifact_path).is_err() {
            let _ = std::fs::remove_dir_all(app_local_data_dir);
            let _ = std::fs::remove_dir_all(external_dir);
            return;
        }

        let listed =
            list_options_expression_artifacts_blocking_headless(&app_local_data_dir).expect("list");

        assert!(listed.is_empty());

        let _ = std::fs::remove_dir_all(app_local_data_dir);
        let _ = std::fs::remove_dir_all(external_dir);
    }

    #[test]
    fn summarize_options_expression_artifact_reports_fragmented_retained_leaders() {
        let temp_root = std::env::temp_dir().join(format!(
            "vectorgrid_options_summary_leaders_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let mut artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.selection_metrics = vec![
            sample_summary_tuple(0, 0.40, 0.20, Some(0.05), Some(0.07), Some(0.03), true, 2),
            sample_summary_tuple(1, 0.30, 0.25, Some(0.11), Some(0.06), Some(0.02), false, 0),
            sample_summary_tuple(5, 0.20, 0.18, Some(0.04), Some(0.12), Some(0.08), true, 1),
        ];

        let summary = summarize_options_expression_artifact(
            std::path::Path::new("C:/runs/vectorgrid_options_expression_1.json"),
            1024,
            &artifact,
        );

        assert_eq!(summary.leader_summary.search_leader_tuple_index, Some(1));
        assert_eq!(summary.leader_summary.robust_leader_tuple_index, Some(0));
        assert_eq!(summary.leader_summary.holdout_leader_tuple_index, Some(1));
        assert_eq!(summary.leader_summary.sealed_leader_tuple_index, Some(2));
        assert_eq!(summary.leader_summary.stress_leader_tuple_index, Some(2));
        assert_eq!(summary.leader_summary.distinct_leader_count, 3);
        assert!(summary.leader_summary.search_vs_robust_disagree);
        assert_eq!(
            summary.leader_summary.robust_vs_holdout_disagree,
            Some(true)
        );
        assert_eq!(summary.leader_summary.robust_vs_sealed_disagree, Some(true));
        assert_eq!(
            summary.leader_summary.robust_leader_reached_sealed_finalist,
            Some(true)
        );
        assert_eq!(
            summary.leader_summary.robust_leader_sealed_final_rank,
            Some(2)
        );

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir(&temp_root);
    }

    #[test]
    fn summarize_options_expression_artifact_uses_lower_retained_index_for_ties_and_absent_lanes() {
        let temp_root = std::env::temp_dir().join(format!(
            "vectorgrid_options_summary_tie_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let mut artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.selection_metrics = vec![
            sample_summary_tuple(0, 0.30, 0.20, None, None, None, false, 0),
            sample_summary_tuple(1, 0.30, 0.20, None, None, None, false, 0),
        ];

        let summary = summarize_options_expression_artifact(
            std::path::Path::new("C:/runs/vectorgrid_options_expression_1.json"),
            1024,
            &artifact,
        );

        assert_eq!(summary.leader_summary.search_leader_tuple_index, Some(0));
        assert_eq!(summary.leader_summary.robust_leader_tuple_index, Some(0));
        assert_eq!(summary.leader_summary.holdout_leader_tuple_index, None);
        assert_eq!(summary.leader_summary.sealed_leader_tuple_index, None);
        assert_eq!(summary.leader_summary.stress_leader_tuple_index, None);
        assert_eq!(summary.leader_summary.distinct_leader_count, 1);
        assert!(!summary.leader_summary.search_vs_robust_disagree);
        assert_eq!(summary.leader_summary.robust_vs_holdout_disagree, None);
        assert_eq!(summary.leader_summary.robust_vs_sealed_disagree, None);
        assert_eq!(
            summary.leader_summary.robust_leader_reached_sealed_finalist,
            Some(false)
        );
        assert_eq!(summary.leader_summary.robust_leader_sealed_final_rank, None);

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir(&temp_root);
    }

    #[test]
    fn summarize_options_expression_artifact_preserves_price_semantics_provenance() {
        let temp_root = std::env::temp_dir().join(format!(
            "vectorgrid_options_summary_semantics_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");

        for (options_dataset_id, price_semantics, fill_model, expected_fill_model) in [
            (
                "spy_mid",
                OptionsPriceSemantics::DerivedMidSnapshot,
                OptionsFillModelFamily::AggregateResearch,
                "aggregate_research",
            ),
            (
                "spy_quotes",
                OptionsPriceSemantics::TopOfBookQuote,
                OptionsFillModelFamily::QuoteConservative,
                "quote_conservative",
            ),
        ] {
            let artifact = sample_summary_artifact(
                &parent_artifact_path,
                options_dataset_id,
                price_semantics,
                fill_model,
            );
            let summary = summarize_options_expression_artifact(
                std::path::Path::new("C:/runs/vectorgrid_options_expression_1.json"),
                1024,
                &artifact,
            );
            assert_eq!(summary.price_semantics, price_semantics);
            assert_eq!(summary.options_dataset_id, options_dataset_id);
            assert_eq!(summary.request_fill_model, expected_fill_model);
            let exactness = summary
                .exactness_semantics
                .as_ref()
                .expect("exactness semantics");
            assert_eq!(
                exactness
                    .get("median_replay_contract")
                    .and_then(Value::as_str),
                Some("ranking_irrelevant_deferred")
            );
            match price_semantics {
                OptionsPriceSemantics::TopOfBookQuote => {
                    assert_eq!(
                        exactness.get("exactness_class").and_then(Value::as_str),
                        Some("quote_class")
                    );
                    assert_eq!(
                        exactness
                            .get("retained_frontier_certification")
                            .and_then(Value::as_str),
                        Some("certified_exact")
                    );
                }
                OptionsPriceSemantics::MinuteAggregate
                | OptionsPriceSemantics::DerivedMidSnapshot => {
                    assert_eq!(
                        exactness.get("exactness_class").and_then(Value::as_str),
                        Some("aggregate_class")
                    );
                    assert_eq!(
                        exactness
                            .get("retained_frontier_certification")
                            .and_then(Value::as_str),
                        Some("provisional_missing_extreme_lane")
                    );
                }
            }
        }

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir(&temp_root);
    }

    fn sample_replay_dataset(
        options_dataset_id: &str,
        price_semantics: OptionsPriceSemantics,
    ) -> ImportedOptionsDataset {
        ImportedOptionsDataset {
            manifest: OptionsDatasetManifestV1 {
                version: 1,
                options_dataset_id: options_dataset_id.to_string(),
                root_symbol: "SPY".to_string(),
                provider_name: "massive".to_string(),
                provider_source_surface: match price_semantics {
                    OptionsPriceSemantics::MinuteAggregate => "minute_aggregate".to_string(),
                    OptionsPriceSemantics::DerivedMidSnapshot => "derived_mid_snapshot".to_string(),
                    OptionsPriceSemantics::TopOfBookQuote => "top_of_book_quote".to_string(),
                },
                provider_entitlement_profile: Some("options_business".to_string()),
                quote_resolution: "1m".to_string(),
                price_semantics,
                calendar_id: "market_hours_utc".to_string(),
                timezone: "UTC".to_string(),
                coverage_start: 1,
                coverage_end: 2,
                normalization_version: 1,
                quote_semantics_version: 1,
                corporate_action_mapping_version: 1,
                contract_reference_source: "massive_contract_reference".to_string(),
                contract_reference_as_of_utc: Some("2026-04-06T00:00:00Z".to_string()),
                underlier_reference_source: "underlier_dataset".to_string(),
                dataset_path: "C:/options/spy.csv".to_string(),
                row_count: 2,
                contract_count: 1,
                aggregate_price_field: if matches!(
                    price_semantics,
                    OptionsPriceSemantics::MinuteAggregate
                ) {
                    Some("close".to_string())
                } else {
                    None
                },
                normalized_rows_sha256: Some(format!("rows-sha256-{options_dataset_id}")),
                manifest_semantic_sha256: Some(format!(
                    "manifest-semantic-sha256-{options_dataset_id}"
                )),
            },
            contracts: vec![OptionsContractSeries {
                root_symbol: "SPY".to_string(),
                contract_id: "SPY-2026-04-03-C-645".to_string(),
                option_type: OptionType::Call,
                expiration_date: "2026-04-03".to_string(),
                expiration_timestamp_ms: 1_775_210_400_000,
                strike: 645.0,
                multiplier: 100.0,
                rows: vec![
                    OptionsPriceRow {
                        timestamp_ms: 1_775_191_200_000,
                        open: 2.1,
                        high: 2.4,
                        low: 1.9,
                        close: 2.2,
                        underlying_price: 643.5,
                        bid: Some(2.0),
                        ask: Some(2.4),
                        mid: Some(2.2),
                        volume: Some(150.0),
                        open_interest: Some(1_250.0),
                        quote_age_ms: None,
                        session_id: None,
                    },
                    OptionsPriceRow {
                        timestamp_ms: 1_775_191_260_000,
                        open: 2.2,
                        high: 2.6,
                        low: 2.1,
                        close: 2.5,
                        underlying_price: 644.1,
                        bid: None,
                        ask: None,
                        mid: None,
                        volume: None,
                        open_interest: None,
                        quote_age_ms: Some(90_000),
                        session_id: Some("2026-04-03-rth".to_string()),
                    },
                ],
            }],
        }
    }

    fn sample_replay_inputs_for_artifact(
        artifact: &OptionsExpressionArtifactV1,
        options_dataset_hash: &str,
        price_semantics: OptionsPriceSemantics,
    ) -> OptionsExpressionReplayInputs {
        let mut normalized_req = artifact.request.clone();
        normalized_req.options_dataset_id = artifact.options_dataset_id.clone();
        OptionsExpressionReplayInputs {
            parent_artifact_id: artifact.parent_artifact_id.clone(),
            parent_artifact_hash: artifact.parent_artifact_hash.clone(),
            normalized_req,
            signal_manifest: Arc::new(sample_signal_manifest()),
            dataset: Arc::new(sample_replay_dataset(
                &artifact.options_dataset_id,
                price_semantics,
            )),
            options_dataset_hash: options_dataset_hash.to_string(),
        }
    }

    #[test]
    fn dataset_provenance_includes_quote_quality_summary_for_quote_rows() {
        let dataset = sample_replay_dataset("spy_quotes", OptionsPriceSemantics::TopOfBookQuote);

        let provenance = dataset_provenance_from_imported_dataset(&dataset);
        assert_eq!(
            provenance.normalized_rows_sha256.as_deref(),
            Some("rows-sha256-spy_quotes")
        );
        assert_eq!(
            provenance.manifest_semantic_sha256.as_deref(),
            Some("manifest-semantic-sha256-spy_quotes")
        );
        let quote_quality = provenance
            .quote_quality
            .expect("quote quality should be present for quote datasets");

        assert_eq!(quote_quality.row_count, 2);
        assert_eq!(quote_quality.contract_count, 1);
        assert_eq!(quote_quality.any_quote_row_count, 1);
        assert_eq!(quote_quality.bid_ask_row_count, 1);
        assert_eq!(quote_quality.mid_row_count, 1);
        assert_eq!(quote_quality.quote_age_row_count, 1);
        assert_eq!(quote_quality.stale_quote_row_count, 0);
        assert_eq!(quote_quality.invalid_bid_ask_row_count, 0);
        assert_eq!(quote_quality.crossed_bid_ask_row_count, 0);
        assert!(quote_quality.warnings.is_empty());
        assert_eq!(quote_quality.per_contract.len(), 1);
        assert_eq!(
            quote_quality.per_contract[0].contract_id,
            "SPY-2026-04-03-C-645"
        );
        assert!((quote_quality.bid_ask_coverage_rate - 0.5).abs() < 1e-12);
        assert!(quote_quality.mean_spread_bps.unwrap_or_default() > 1_800.0);
    }

    #[test]
    fn dataset_quote_quality_flags_advisory_bid_ask_conditions() {
        let mut dataset =
            sample_replay_dataset("spy_quote_advisory", OptionsPriceSemantics::TopOfBookQuote);
        let rows = &mut dataset.contracts[0].rows;
        rows[0].bid = Some(2.4);
        rows[0].ask = Some(2.0);
        rows[1].bid = Some(0.0);
        rows[1].ask = Some(0.0);
        rows[1].mid = Some(0.0);

        let quote_quality = dataset_quote_quality_from_imported_dataset(&dataset)
            .expect("quote quality should be present");

        assert_eq!(quote_quality.invalid_bid_ask_row_count, 1);
        assert_eq!(quote_quality.crossed_bid_ask_row_count, 1);
        assert_eq!(quote_quality.locked_bid_ask_row_count, 1);
        assert_eq!(quote_quality.zero_bid_row_count, 1);
        assert!(quote_quality
            .warnings
            .iter()
            .any(|entry| { entry.contains("Invalid bid/ask rows detected") }));
        assert!(quote_quality.per_contract[0]
            .warnings
            .iter()
            .any(|entry| entry.contains("Zero-bid rows detected")));
    }

    #[test]
    fn saved_artifact_replay_provenance_accepts_matching_dataset_hash_and_semantics() {
        let temp_root = std::env::temp_dir().join(format!(
            "vectorgrid_options_replay_provenance_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_quotes",
            OptionsPriceSemantics::TopOfBookQuote,
            OptionsFillModelFamily::QuoteConservative,
        );
        let replay = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::TopOfBookQuote,
        );

        validate_saved_artifact_replay_provenance(&artifact, &replay)
            .expect("matching provenance should pass");

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir(&temp_root);
    }

    #[test]
    fn saved_artifact_replay_provenance_rejects_dataset_hash_or_semantics_drift() {
        let temp_root = std::env::temp_dir().join(format!(
            "vectorgrid_options_replay_provenance_drift_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_mid",
            OptionsPriceSemantics::DerivedMidSnapshot,
            OptionsFillModelFamily::AggregateResearch,
        );

        let hash_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-2",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        let hash_error = validate_saved_artifact_replay_provenance(&artifact, &hash_drift)
            .expect_err("hash drift should fail");
        assert!(hash_error.contains("dataset hash"));
        assert!(hash_error.contains("spy_mid"));

        let semantics_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::TopOfBookQuote,
        );
        let semantics_error =
            validate_saved_artifact_replay_provenance(&artifact, &semantics_drift)
                .expect_err("semantics drift should fail");
        assert!(semantics_error.contains("price semantics"));
        assert!(semantics_error.contains("spy_mid"));

        let mut parent_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        parent_drift.parent_artifact_id = "vectorgrid_run_2".to_string();
        let parent_error = validate_saved_artifact_replay_provenance(&artifact, &parent_drift)
            .expect_err("parent drift should fail");
        assert!(parent_error.contains("parent id"));

        let mut parent_hash_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        parent_hash_drift.parent_artifact_hash = "parent-hash-2".to_string();
        let parent_hash_error =
            validate_saved_artifact_replay_provenance(&artifact, &parent_hash_drift)
                .expect_err("parent hash drift should fail");
        assert!(parent_hash_error.contains("parent hash"));

        let mut signal_manifest_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        Arc::make_mut(&mut signal_manifest_drift.signal_manifest).manifest_id =
            "manifest-2".to_string();
        let signal_manifest_error =
            validate_saved_artifact_replay_provenance(&artifact, &signal_manifest_drift)
                .expect_err("signal manifest drift should fail");
        assert!(signal_manifest_error.contains("signal manifest id"));

        let mut provider_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        Arc::make_mut(&mut provider_drift.dataset)
            .manifest
            .provider_name = "other_provider".to_string();
        let provider_error = validate_saved_artifact_replay_provenance(&artifact, &provider_drift)
            .expect_err("provider provenance drift should fail");
        assert!(provider_error.contains("provider identity"));

        let mut row_hash_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        Arc::make_mut(&mut row_hash_drift.dataset)
            .manifest
            .normalized_rows_sha256 = Some("rows-sha256-drift".to_string());
        let row_hash_error = validate_saved_artifact_replay_provenance(&artifact, &row_hash_drift)
            .expect_err("row SHA-256 drift should fail exact replay");
        assert!(row_hash_error.contains("normalized_rows_sha256"));

        let mut manifest_hash_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        Arc::make_mut(&mut manifest_hash_drift.dataset)
            .manifest
            .manifest_semantic_sha256 = Some("manifest-semantic-sha256-drift".to_string());
        let manifest_hash_error =
            validate_saved_artifact_replay_provenance(&artifact, &manifest_hash_drift)
                .expect_err("manifest semantic SHA-256 drift should fail exact replay");
        assert!(manifest_hash_error.contains("manifest_semantic_sha256"));

        let mut legacy_artifact = artifact.clone();
        legacy_artifact.options_dataset_normalized_rows_sha256 = None;
        legacy_artifact.options_dataset_manifest_semantic_sha256 = None;
        if let Some(provenance) = legacy_artifact.dataset_provenance.as_mut() {
            provenance.normalized_rows_sha256 = None;
            provenance.manifest_semantic_sha256 = None;
        }
        validate_saved_artifact_replay_provenance(&legacy_artifact, &row_hash_drift)
            .expect("legacy artifacts without content-grade hashes should remain replayable");

        let mut contract_reference_drift = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        Arc::make_mut(&mut contract_reference_drift.dataset)
            .manifest
            .contract_reference_as_of_utc = Some("2026-04-07T00:00:00Z".to_string());
        let contract_reference_error =
            validate_saved_artifact_replay_provenance(&artifact, &contract_reference_drift)
                .expect_err("contract reference provenance drift should fail");
        assert!(contract_reference_error.contains("contract_reference_as_of_utc"));

        let _ = std::fs::remove_file(&parent_artifact_path);
        let _ = std::fs::remove_dir(&temp_root);
    }

    #[test]
    fn validation_tuple_selection_includes_default_leaders_and_explicit_indexes() {
        let temp_root = temp_app_local_data_dir("validation_tuple_selection");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let mut artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        artifact.selection_metrics = vec![
            sample_summary_tuple(0, 0.60, 0.10, Some(0.05), None, Some(0.04), false, 0),
            sample_summary_tuple(1, 0.30, 0.20, Some(0.30), None, Some(0.10), false, 0),
            sample_summary_tuple(2, 0.20, 0.15, Some(0.10), Some(0.25), Some(0.08), true, 1),
        ];

        let selection = OptionsExpressionValidationTupleSelection {
            include_sealed_finalists: true,
            include_robust_leader: true,
            include_holdout_leader: true,
            tuple_indexes: vec![1],
        };
        let indexes =
            select_options_expression_validation_tuple_indexes(&artifact, &selection).unwrap();

        assert_eq!(indexes, vec![0, 1, 2]);
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn validation_replay_request_accepts_date_range_and_walk_forward_plans() {
        let temp_root = temp_app_local_data_dir("validation_plan_request");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );

        let date_req = OptionsExpressionValidationRequest {
            artifact_path: parent_artifact_path.to_string_lossy().to_string(),
            options_dataset_id: String::new(),
            dataset_policy: OptionsExpressionValidationDatasetPolicy::RequireExactReplay,
            tuple_selection: OptionsExpressionValidationTupleSelection::default(),
            validation_plan: OptionsExpressionValidationPlan {
                mode: OptionsExpressionValidationPlanMode::DateRangeHoldout,
                start_timestamp_ms: Some(1_700_000_000_000),
                end_timestamp_ms: Some(1_700_086_400_000),
                ..OptionsExpressionValidationPlan::default()
            },
            fill_model_override: None,
            min_valid_trades: None,
        };
        let replay = effective_validation_replay_request(&artifact, &date_req)
            .expect("date range replay request");
        assert_eq!(replay.options_dataset_id, "spy_options");

        let rolling_req = OptionsExpressionValidationRequest {
            validation_plan: OptionsExpressionValidationPlan {
                mode: OptionsExpressionValidationPlanMode::RollingSessionWalkForward,
                train_session_count: Some(2),
                validation_session_count: Some(1),
                step_session_count: Some(1),
                ..OptionsExpressionValidationPlan::default()
            },
            ..date_req
        };
        effective_validation_replay_request(&artifact, &rolling_req)
            .expect("rolling replay request");

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn validation_replay_request_rejects_incomplete_plan_inputs() {
        let temp_root = temp_app_local_data_dir("validation_plan_request_rejects");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        let req = OptionsExpressionValidationRequest {
            artifact_path: parent_artifact_path.to_string_lossy().to_string(),
            options_dataset_id: String::new(),
            dataset_policy: OptionsExpressionValidationDatasetPolicy::RequireExactReplay,
            tuple_selection: OptionsExpressionValidationTupleSelection::default(),
            validation_plan: OptionsExpressionValidationPlan {
                mode: OptionsExpressionValidationPlanMode::DateRangeHoldout,
                start_timestamp_ms: Some(2),
                end_timestamp_ms: Some(1),
                ..OptionsExpressionValidationPlan::default()
            },
            fill_model_override: None,
            min_valid_trades: None,
        };

        let error = effective_validation_replay_request(&artifact, &req)
            .expect_err("inverted date range should fail");
        assert!(error.contains("start_timestamp_ms < end_timestamp_ms"));

        let req = OptionsExpressionValidationRequest {
            validation_plan: OptionsExpressionValidationPlan {
                mode: OptionsExpressionValidationPlanMode::AnchoredWalkForward,
                train_session_count: Some(1),
                validation_session_count: None,
                step_session_count: Some(1),
                ..OptionsExpressionValidationPlan::default()
            },
            ..req
        };
        let error = effective_validation_replay_request(&artifact, &req)
            .expect_err("incomplete walk-forward plan should fail");
        assert!(error.contains("positive train_session_count"));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn validation_replay_policy_requires_explicit_replacement_or_cross_lane() {
        let temp_root = temp_app_local_data_dir("validation_replay_policy");
        let parent_artifact_path = write_parent_strategy_artifact(&temp_root, "DoubleMa");
        let artifact = sample_summary_artifact(
            &parent_artifact_path,
            "spy_options",
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch,
        );
        let exact = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-1",
            OptionsPriceSemantics::MinuteAggregate,
        );
        assert_eq!(
            validate_options_expression_validation_replay_provenance(
                &artifact,
                &exact,
                OptionsExpressionValidationDatasetPolicy::RequireExactReplay,
            )
            .unwrap(),
            false
        );

        let mut replacement = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-2",
            OptionsPriceSemantics::MinuteAggregate,
        );
        replacement.normalized_req.options_dataset_id = "spy_options_replacement".to_string();
        Arc::make_mut(&mut replacement.dataset)
            .manifest
            .options_dataset_id = "spy_options_replacement".to_string();
        let exact_error = validate_options_expression_validation_replay_provenance(
            &artifact,
            &replacement,
            OptionsExpressionValidationDatasetPolicy::RequireExactReplay,
        )
        .expect_err("exact replay should reject replacement dataset");
        assert!(exact_error.contains("dataset id"));
        assert_eq!(
            validate_options_expression_validation_replay_provenance(
                &artifact,
                &replacement,
                OptionsExpressionValidationDatasetPolicy::AllowSameRootReplacement,
            )
            .unwrap(),
            true
        );

        let mut cross_lane = sample_replay_inputs_for_artifact(
            &artifact,
            "hash-3",
            OptionsPriceSemantics::DerivedMidSnapshot,
        );
        cross_lane.normalized_req.options_dataset_id = "spy_options_mid".to_string();
        Arc::make_mut(&mut cross_lane.dataset)
            .manifest
            .options_dataset_id = "spy_options_mid".to_string();
        let same_root_error = validate_options_expression_validation_replay_provenance(
            &artifact,
            &cross_lane,
            OptionsExpressionValidationDatasetPolicy::AllowSameRootReplacement,
        )
        .expect_err("same-root replacement should reject cross-lane semantics");
        assert!(same_root_error.contains("matching price semantics"));
        assert_eq!(
            validate_options_expression_validation_replay_provenance(
                &artifact,
                &cross_lane,
                OptionsExpressionValidationDatasetPolicy::AllowExplicitCrossLaneComparison,
            )
            .unwrap(),
            true
        );

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn validate_event_path_replay_consistency_rejects_row_drift() {
        let tuple_result =
            sample_summary_tuple(0, 0.30, 0.20, Some(0.10), Some(0.08), Some(0.04), true, 1);
        let expected_tuple = tuple_result.tuple.clone();
        let tuple_row = ta_app_core::options_types::OptionsExpressionEventDetailRow {
            event_index: 0,
            event_id: "evt-1".to_string(),
            split: OptionsEvaluationSplit::Search,
            side: SignalSide::Bullish,
            decision_timestamp: 1_700_000_000_000,
            earliest_tradable_timestamp: 1_700_000_060_000,
            requested_entry_timestamp: 1_700_000_060_000,
            raw_underlier_price: 100.0,
            base_exit_timestamp: Some(1_700_000_120_000),
            status: ta_app_core::options_types::OptionsExpressionEventStatus::Executed,
            candidate_invalid_reason: None,
            contract_id: Some("SPY-2026-04-03-C-645".to_string()),
            companion_contract_id: None,
            expiration_date: Some("2026-04-03".to_string()),
            strike: Some(645.0),
            companion_strike: None,
            multiplier: Some(100.0),
            entry_timestamp: Some(1_700_000_060_000),
            entry_option_price: Some(2.2),
            companion_entry_option_price: None,
            entry_underlier_price: Some(100.5),
            exit_timestamp: Some(1_700_000_120_000),
            exit_option_price: Some(2.5),
            companion_exit_option_price: None,
            premium_paid: Some(2.2),
            pnl_per_contract: Some(30.0),
            return_on_premium: Some(0.13636),
            entry_lag_minutes: Some(1),
            minutes_to_expiry_at_entry: Some(390),
            realized_hold_minutes: Some(60),
            path_option_high: Some(2.6),
            path_option_low: Some(2.1),
            path_underlier_high: Some(101.0),
            path_underlier_low: Some(99.8),
            exit_reason: Some(ta_app_core::options_types::OptionsExpressionExitReason::TargetHit),
        };
        let tuple_inspection = OptionsExpressionTupleInspectionResult {
            tuple_index: 0,
            tuple_count: 1,
            tuple_result,
            total_events: 1,
            executed_events: 1,
            candidate_missing_events: 0,
            outcome_missing_events: 0,
            rows: vec![tuple_row.clone()],
        };
        let mut drifted_row = tuple_row.clone();
        drifted_row.status =
            ta_app_core::options_types::OptionsExpressionEventStatus::OutcomeMissing;
        let event_path_inspection =
            ta_app_core::options_types::OptionsExpressionEventPathInspectionResult {
                tuple_index: 0,
                event_index: 0,
                event_id: "evt-1".to_string(),
                row: drifted_row,
                point_count: 1,
                points: vec![
                    ta_app_core::options_types::OptionsExpressionEventPathPoint {
                        timestamp_ms: 1_700_000_060_000,
                        option_open: 2.2,
                        option_high: 2.6,
                        option_low: 2.1,
                        option_close: 2.5,
                        underlying_price: 100.5,
                    },
                ],
            };

        let error = validate_event_path_replay_consistency(
            &expected_tuple,
            &tuple_inspection,
            &event_path_inspection,
        )
        .expect_err("row drift should fail");

        assert!(error.contains("drifted from tuple inspection replay state"));
    }
}
