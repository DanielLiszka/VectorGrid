use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use ta_app_core::OptionsExpressionArtifactV1;
use tauri::Manager;

use super::{
    commands_options_expression::{
        summarize_options_expression_artifact, OptionsExpressionArtifactSummary,
    },
    metric_surface::build_optimization_metric_surface_artifact_value,
    reveal_path_blocking, sanitize_filename_component,
};

const DATASET_META_SCHEMA_VERSION: u64 = 2;
const CONFIG_FINGERPRINT_SCHEMA_VERSION: u64 = 1;
const RESAMPLE_RULE_SESSION_RELATIVE_OHLCV: &str = "session_relative_ohlcv";
const RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP: &str = "day_first_timestamp";
const ARTIFACT_PATH_SUFFIX_LIMIT: u32 = 999;
const LIVE_SESSION_EVENT_LIMIT: usize = 64;
const LIVE_SESSION_SNAPSHOT_LIMIT: usize = 16;

fn is_run_artifact_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_run_artifact" | "vectorgrid_run_artifact"
    )
}

fn is_options_expression_artifact_kind(kind: &str) -> bool {
    matches!(kind.trim(), "vectorgrid_options_expression_v1")
}

fn is_live_session_artifact_kind(kind: &str) -> bool {
    matches!(kind.trim(), "vectorgrid_live_session_v1")
}

fn is_meta_selection_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_meta_selection" | "vectorgrid_meta_selection"
    )
}

fn is_meta_stage_result_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_meta_stage4_experimental"
            | "vectorgrid_meta_stage4_experimental"
            | "vectorbt_meta_stage5_ml_confidence"
            | "vectorgrid_meta_stage5_ml_confidence"
    )
}

fn is_ml_confidence_policy_eval_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_ml_confidence_policy_evaluation" | "vectorgrid_ml_confidence_policy_evaluation"
    )
}

fn is_mutable_library_artifact_kind(kind: &str) -> bool {
    is_run_artifact_kind(kind)
        || is_meta_selection_kind(kind)
        || is_meta_stage_result_kind(kind)
        || is_ml_confidence_policy_eval_kind(kind)
        || is_options_expression_artifact_kind(kind)
}

fn prune_old_artifacts_by_prefixes_in_dir(dir: &Path, keep_newest: usize, prefixes: &[&str]) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !prefixes.iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }
        paths.push(path);
    }

    paths.sort_by(|a, b| {
        let aa = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let bb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        bb.cmp(aa)
    });

    if paths.len() <= keep_newest {
        return;
    }

    for path in paths.into_iter().skip(keep_newest) {
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tags = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map_or(&[][..], |xs| xs.as_slice());
        let tagged = tags
            .iter()
            .any(|t| t.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false));
        if tagged {
            continue;
        }
        let _ = std::fs::remove_file(&path);
    }
}

fn is_meta_artifact_file_name(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.starts_with("vectorbt_meta_") || trimmed.starts_with("vectorgrid_meta_")
}

fn is_run_artifact_file_name(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.starts_with("vectorbt_run_") || trimmed.starts_with("vectorgrid_run_")
}

fn is_live_session_artifact_file_name(name: &str) -> bool {
    name.trim().starts_with("vectorgrid_live_session_")
}

#[cfg(test)]
fn is_meta_stage_artifact_file_name(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.starts_with("vectorbt_stage4_")
        || trimmed.starts_with("vectorbt_meta_stage4_")
        || trimmed.starts_with("vectorbt_stage5_")
        || trimmed.starts_with("vectorbt_meta_stage5_")
        || trimmed.starts_with("vectorgrid_stage4_")
        || trimmed.starts_with("vectorgrid_meta_stage4_")
        || trimmed.starts_with("vectorgrid_stage5_")
        || trimmed.starts_with("vectorgrid_meta_stage5_")
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveRunArtifactResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveBatchSummaryResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLineageSummary {
    pub parent_artifact_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent_artifact_kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lineage_relation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteProvenanceSummary {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub requested_route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resolved_route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub realized_route: String,
    pub route_changed: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fallback_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub strategy_kind: String,
    pub config_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_fingerprint_version: Option<u32>,
    pub has_validation: bool,
    pub report_xlsx_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_exists: Option<bool>,
    pub csv_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub timeframe: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_timeframe: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub derived_timeframe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resample_applied: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resample_rule: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resample_anchor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mtime_utc_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_content_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_provenance: Option<RouteProvenanceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<ArtifactLineageSummary>,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_feature_schema_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_calibration_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_threshold_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_producer_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_native_backend_used: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_native_fallback_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_score_contract_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_source_bundle_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_score_space: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_score_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_score_contract_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_score_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub session_id: String,
    pub mode: String,
    pub symbol: String,
    pub strategy_kind: String,
    pub data_id: String,
    pub dry_run: bool,
    pub param_source: String,
    pub session_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalized_at_unix_ms: Option<i64>,
    pub event_count: usize,
    pub broker_snapshot_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionEventDetail {
    pub provenance: String,
    pub status: String,
    pub message: String,
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_open_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub action: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub order_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_qty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_submitted: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionBrokerSnapshotDetail {
    pub source_command: String,
    pub mode: String,
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account_status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub currency: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub position_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_qty: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub position_side: String,
    pub open_order_count: usize,
    pub order_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionArtifactDetailResponse {
    pub ts: String,
    pub artifact_path: String,
    pub session_id: String,
    pub session_state: String,
    pub mode: String,
    pub symbol: String,
    pub qty: f64,
    pub dry_run: bool,
    pub strategy_kind: String,
    pub data_id: String,
    pub param_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalized_at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_run_artifact_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_run_config_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_run_route_provenance: Option<RouteProvenanceSummary>,
    pub event_count: usize,
    pub broker_snapshot_count: usize,
    pub events: Vec<LiveSessionEventDetail>,
    pub broker_snapshots: Vec<LiveSessionBrokerSnapshotDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub strategy_kind: String,
    pub timeframe: String,
    pub required_tag: String,
    pub groups: usize,
    pub artifacts_used: usize,
    pub holdout_datasets: usize,
    pub report_xlsx_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_exists: Option<bool>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage4ArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub strategy_kind: String,
    pub timeframe: String,
    pub source_meta_json_path: String,
    pub groups_selected: usize,
    pub groups_scored: usize,
    pub groups_considered: usize,
    pub min_ml_confidence: f64,
    pub top_k: usize,
    pub model_healthy: bool,
    pub tags: Vec<String>,
}

pub type Stage5ArtifactSummary = Stage4ArtifactSummary;

#[derive(Debug, Clone, Serialize)]
pub struct MlPolicyEvalArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub strategy_kind: String,
    pub timeframe: String,
    pub artifacts_evaluated: usize,
    pub artifacts_considered: usize,
    pub best_policy: String,
    pub stage5_precision_at_k: f64,
    pub stage5_vs_meta_precision_delta: f64,
    pub stage5_vs_ml_precision_delta: f64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct ArtifactScanEntry {
    path: PathBuf,
    file_name: String,
    bytes: u64,
    value: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLibrarySnapshot {
    pub runs: Vec<RunArtifactSummary>,
    pub meta: Vec<MetaArtifactSummary>,
    pub stage5: Vec<Stage5ArtifactSummary>,
    pub policy_evals: Vec<MlPolicyEvalArtifactSummary>,
    pub options_expression: Vec<OptionsExpressionArtifactSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowArtifactRunPayload {
    pub ts: String,
    pub artifact_path: String,
    pub report_xlsx_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_exists: Option<bool>,
    pub csv_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub config_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_fingerprint_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_meta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_provenance: Option<RouteProvenanceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<ArtifactLineageSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_signal: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimization_metric_surface: Option<Value>,
    pub tags: Vec<String>,
    pub req: Value,
    pub res: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowArtifactValidationPayload {
    pub ts: String,
    pub req: Value,
    pub res: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowArtifactsHydrationPayload {
    pub current_run: WorkflowArtifactRunPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_validation: Option<WorkflowArtifactValidationPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestRunArtifactHydrationResponse {
    pub summary: RunArtifactSummary,
    pub hydration: WorkflowArtifactsHydrationPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunArtifactDetailResponse {
    pub ts: String,
    pub artifact_path: String,
    pub kind: String,
    pub strategy_kind: String,
    pub timeframe: String,
    pub csv_path: String,
    pub report_xlsx_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_exists: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub report_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_path_exists: Option<bool>,
    pub has_validation: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub config_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_fingerprint_version: Option<u32>,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_meta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_provenance: Option<RouteProvenanceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<ArtifactLineageSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_signal: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimization_metric_surface: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_req: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_res: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_req: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_res: Option<Value>,
    pub ml_signal_stage: String,
    pub ml_bundle_id: String,
    pub ml_score_cache_path: String,
    pub ml_score_manifest_path: String,
    pub ml_feature_schema_path: String,
    pub ml_feature_schema_version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_feature_schema_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_export_manifest_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_source_bundle_manifest_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_source_bundle_manifest_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_calibration_state: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_threshold_policy: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_producer_kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_native_backend_requested: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_native_backend_used: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_native_library_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_native_gpu_device_id: Option<u32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_native_fallback_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_score_contract_version: Option<u32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_score_space: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_score_scope: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_score_contract_summary: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ml_score_note: String,
    pub ml_policy_summary: String,
    pub ml_policy_sweep_summary: String,
    pub ml_run_verdict_summary: String,
    pub ml_top_candidates_summary: String,
    pub params_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaArtifactReplayResponse {
    pub ts: String,
    pub kind: String,
    pub report_xlsx_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_exists: Option<bool>,
    pub report_xlsx_bytes: usize,
    pub meta_json_path: String,
    pub meta_json_bytes: usize,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage5ArtifactReplayResponse {
    pub ts: String,
    pub kind: String,
    pub stage4_json_path: String,
    pub stage4_json_bytes: usize,
    pub stage5_json_path: String,
    pub stage5_json_bytes: usize,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MlPolicyEvalArtifactReplayResponse {
    pub ts: String,
    pub kind: String,
    pub eval_json_path: String,
    pub eval_json_bytes: usize,
    pub result: Value,
}

pub(super) fn runs_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(super) fn suggest_meta_artifact_path(
    ts: &str,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_meta", ts, "json")
}

pub(super) fn suggest_stage5_artifact_path(
    ts: &str,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_stage5", ts, "json")
}

pub(super) fn suggest_ml_policy_eval_artifact_path(
    ts: &str,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_ml_confidence_eval", ts, "json")
}

pub(super) fn suggest_batch_summary_path(
    ts: &str,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_batch_summary", ts, "json")
}

pub(super) fn suggest_campaign_summary_path(
    ts: &str,
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_campaign_summary", ts, "json")
}

fn suggest_ts_file_path(
    dir: PathBuf,
    prefix: &str,
    ts: &str,
    ext: &str,
) -> Result<PathBuf, String> {
    suggest_ts_file_path_with_limit(dir, prefix, ts, ext, ARTIFACT_PATH_SUFFIX_LIMIT)
}

fn suggest_ts_file_path_with_limit(
    dir: PathBuf,
    prefix: &str,
    ts: &str,
    ext: &str,
    max_suffix: u32,
) -> Result<PathBuf, String> {
    let safe_ts = sanitize_filename_component(ts);
    let base_name = format!("{prefix}_{safe_ts}");
    let path = dir.join(format!("{base_name}.{ext}"));
    if !path.exists() {
        return Ok(path);
    }

    for i in 2..=max_suffix {
        let candidate = dir.join(format!("{base_name}_{i}.{ext}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "no available artifact filename slots for {}",
        dir.join(format!("{base_name}.{ext}")).to_string_lossy()
    ))
}

fn run_artifact_path<R: tauri::Runtime>(
    artifact: &Value,
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let ts = artifact
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_run", ts, "json")
}

fn existing_run_artifact_target_path<R: tauri::Runtime>(
    existing_path: &str,
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let trimmed = existing_path.trim();
    if trimmed.is_empty() {
        return Err("existing artifact path is empty".to_string());
    }

    let path = PathBuf::from(trimmed);
    let dir = runs_dir(app)?;
    let dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "artifact path is missing a parent directory".to_string())?;
    let parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !parent.starts_with(&dir) {
        return Err("path is outside app runs dir".to_string());
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "artifact path is missing a filename".to_string())?;

    if path.exists() {
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let value: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
        if !is_run_artifact_kind(kind) {
            return Err(format!("unsupported artifact kind: {kind}"));
        }
    } else if !is_run_artifact_file_name(file_name) {
        return Err(format!(
            "existing artifact path must target a saved run filename: {}",
            path.to_string_lossy()
        ));
    }

    Ok(path)
}

fn normalized_config_fingerprint_fields(artifact: &Value) -> (String, Option<u32>) {
    let config_fingerprint = artifact
        .get("config_fingerprint")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| compute_config_fingerprint_from_artifact(artifact));
    let config_fingerprint_version = artifact
        .get("config_fingerprint_version")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .or_else(|| {
            if config_fingerprint.starts_with("cfg1-") {
                Some(1)
            } else {
                None
            }
        });
    (config_fingerprint, config_fingerprint_version)
}

fn run_artifact_file_recency_key(file_name: &str) -> Option<(String, u32)> {
    let stem = Path::new(file_name).file_stem()?.to_str()?.trim();
    let ts_and_suffix = stem
        .strip_prefix("vectorbt_run_")
        .or_else(|| stem.strip_prefix("vectorgrid_run_"))?;
    if ts_and_suffix.is_empty() {
        return None;
    }

    if let Some((base_ts, suffix)) = ts_and_suffix.rsplit_once('_') {
        if let Ok(parsed_suffix) = suffix.parse::<u32>() {
            if parsed_suffix >= 2 && !base_ts.is_empty() {
                return Some((base_ts.to_string(), parsed_suffix));
            }
        }
    }

    Some((ts_and_suffix.to_string(), 1))
}

fn normalized_run_artifact_ts_from_value(artifact: &Value) -> String {
    normalize_text_value(artifact.get("ts"))
}

fn fallback_run_artifact_ts_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .and_then(run_artifact_file_recency_key)
        .map(|(base_ts, _)| base_ts)
        .unwrap_or_default()
}

fn resolved_run_artifact_ts(path: &Path, artifact: &Value) -> String {
    let ts = normalized_run_artifact_ts_from_value(artifact);
    if !ts.is_empty() {
        return ts;
    }
    fallback_run_artifact_ts_from_path(path)
}

fn ensure_path_is_within_dir(path: &Path, dir: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "artifact path does not exist: {}",
            path.to_string_lossy()
        ));
    }

    let dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !path.starts_with(&dir) {
        return Err("path is outside app runs dir".to_string());
    }
    Ok(())
}

fn non_empty_string(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn resolve_run_artifact_strategy_kind(v: &Value) -> String {
    non_empty_string(v.pointer("/run/req/strategy_kind"))
        .or_else(|| non_empty_string(v.pointer("/validation/req/validation_context/strategy_kind")))
        .or_else(|| non_empty_string(v.pointer("/validation/req/strategy_kind")))
        .or_else(|| non_empty_string(v.pointer("/ml_signal/stage")).map(|_| "MlSignal".to_string()))
        .unwrap_or_default()
}

fn resolve_run_artifact_csv_path(v: &Value) -> String {
    non_empty_string(v.get("csv_path"))
        .or_else(|| non_empty_string(v.pointer("/data/csv_path")))
        .or_else(|| non_empty_string(v.pointer("/dataset_meta/csv_path")))
        .or_else(|| non_empty_string(v.pointer("/validation/req/validation_context/csv_path")))
        .unwrap_or_default()
}

fn resolve_run_artifact_timeframe(v: &Value) -> String {
    non_empty_string(v.pointer("/dataset_meta/timeframe"))
        .or_else(|| non_empty_string(v.pointer("/validation/req/validation_context/timeframe")))
        .unwrap_or_default()
}

fn split_backend_route_segments(value: &str) -> Vec<String> {
    value
        .split("->")
        .map(|segment| segment.trim().to_string())
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn normalize_requested_route(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    match value {
        Value::String(raw) => raw.trim().to_string(),
        Value::Object(row) => row
            .get("backend")
            .and_then(Value::as_str)
            .or_else(|| row.get("kind").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string(),
        _ => String::new(),
    }
}

fn normalize_backend_used_route(requested: &str, backend_used: &str) -> String {
    match backend_used.trim() {
        "Cpu" if requested.starts_with("Cpu") => requested.to_string(),
        "Cpu" => "CpuOnly".to_string(),
        "Gpu" if requested.starts_with("Gpu") => requested.to_string(),
        "Gpu" => "Gpu".to_string(),
        other => other.to_string(),
    }
}

fn extract_route_provenance_from_value(v: Option<&Value>) -> Option<RouteProvenanceSummary> {
    if let Some(existing) = v.and_then(Value::as_object) {
        let requested_route = existing
            .get("requested_route")
            .and_then(Value::as_str)
            .or_else(|| existing.get("requestedRoute").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string();
        let resolved_route = existing
            .get("resolved_route")
            .and_then(Value::as_str)
            .or_else(|| existing.get("resolvedRoute").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string();
        let realized_route = existing
            .get("realized_route")
            .and_then(Value::as_str)
            .or_else(|| existing.get("realizedRoute").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string();
        let fallback_reason = existing
            .get("fallback_reason")
            .and_then(Value::as_str)
            .or_else(|| existing.get("fallbackReason").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string();
        let route_changed = existing
            .get("route_changed")
            .and_then(Value::as_bool)
            .or_else(|| existing.get("routeChanged").and_then(Value::as_bool))
            .unwrap_or(false);
        if !requested_route.is_empty()
            || !resolved_route.is_empty()
            || !realized_route.is_empty()
            || !fallback_reason.is_empty()
            || route_changed
        {
            return Some(RouteProvenanceSummary {
                requested_route,
                resolved_route,
                realized_route,
                route_changed,
                fallback_reason,
            });
        }
    }
    None
}

fn extract_route_provenance_from_artifact(v: &Value) -> Option<RouteProvenanceSummary> {
    if let Some(existing) = extract_route_provenance_from_value(v.pointer("/route_provenance")) {
        return Some(existing);
    }

    let requested_route = normalize_requested_route(v.pointer("/run/req/backend"));
    let backend_route = non_empty_string(v.pointer("/run/res/backend_route")).unwrap_or_default();
    let backend_used = non_empty_string(v.pointer("/run/res/backend_used")).unwrap_or_default();
    let fallback_reason =
        non_empty_string(v.pointer("/run/res/backend_fallback_reason")).unwrap_or_default();
    let segments = split_backend_route_segments(&backend_route);
    let resolved_route = segments
        .first()
        .cloned()
        .unwrap_or_else(|| requested_route.clone());
    let realized_route = segments
        .last()
        .cloned()
        .unwrap_or_else(|| normalize_backend_used_route(&requested_route, &backend_used));
    let route_changed = !fallback_reason.is_empty()
        || segments.len() > 1
        || (!requested_route.is_empty()
            && !realized_route.is_empty()
            && requested_route != realized_route);
    if requested_route.is_empty()
        && resolved_route.is_empty()
        && realized_route.is_empty()
        && fallback_reason.is_empty()
        && !route_changed
    {
        return None;
    }
    Some(RouteProvenanceSummary {
        requested_route,
        resolved_route,
        realized_route,
        route_changed,
        fallback_reason,
    })
}

fn live_session_artifact_path<R: tauri::Runtime>(
    artifact: &Value,
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let ts = artifact
        .get("ts")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_live_session", ts, "json")
}

fn write_json_pretty(path: &Path, value: &Value) -> Result<(), String> {
    let raw = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, raw).map_err(|e| e.to_string())
}

fn sanitize_live_session_route_provenance(value: Option<&Value>) -> Option<Value> {
    extract_route_provenance_from_value(value)
        .and_then(|summary| serde_json::to_value(summary).ok())
}

fn sanitize_live_session_event_value(value: &Value) -> Value {
    serde_json::json!({
        "provenance": normalize_text_value(value.get("provenance")),
        "status": normalize_text_value(value.get("status")),
        "message": normalize_text_value(value.get("message")),
        "symbol": normalize_text_value(value.get("symbol")),
        "at_unix_ms": value.get("at_unix_ms").and_then(Value::as_i64),
        "bar_open_unix_ms": value.get("bar_open_unix_ms").and_then(Value::as_i64),
        "action": normalize_text_value(value.get("action")),
        "order_id": normalize_text_value(value.get("order_id")),
        "delta_qty": value.get("delta_qty").and_then(Value::as_f64),
        "order_submitted": value.get("order_submitted").and_then(Value::as_bool),
        "warnings": normalize_string_list(value.get("warnings")),
    })
}

fn sanitize_live_session_broker_snapshot_value(value: &Value) -> Value {
    serde_json::json!({
        "source_command": normalize_text_value(value.get("source_command")),
        "mode": normalize_text_value(value.get("mode")),
        "symbol": normalize_text_value(value.get("symbol")),
        "at_unix_ms": value.get("at_unix_ms").and_then(Value::as_i64),
        "account_status": normalize_text_value(value.get("account_status")),
        "currency": normalize_text_value(value.get("currency")),
        "position_state": normalize_text_value(value.get("position_state")),
        "position_qty": value.get("position_qty").and_then(Value::as_f64),
        "position_side": normalize_text_value(value.get("position_side")),
        "open_order_count": value
            .get("open_order_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "order_count": value.get("order_count").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn sanitize_live_session_artifact_seed(value: &Value) -> Value {
    let ts = normalize_text_value(value.get("ts"));
    let created_at_unix_ms = value
        .get("created_at_unix_ms")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let session_state = {
        let state = normalize_text_value(value.get("session_state"));
        if state.is_empty() {
            "active".to_string()
        } else {
            state
        }
    };
    serde_json::json!({
        "kind": "vectorgrid_live_session_v1",
        "ts": if ts.is_empty() { chrono::Utc::now().to_rfc3339() } else { ts },
        "session_id": normalize_text_value(value.get("session_id")),
        "session_state": session_state,
        "created_at_unix_ms": created_at_unix_ms,
        "mode": normalize_text_value(value.get("mode")),
        "symbol": normalize_text_value(value.get("symbol")),
        "qty": value.get("qty").and_then(Value::as_f64).unwrap_or(0.0),
        "dry_run": value.get("dry_run").and_then(Value::as_bool).unwrap_or(false),
        "strategy_kind": normalize_text_value(value.get("strategy_kind")),
        "data_id": normalize_text_value(value.get("data_id")),
        "param_source": normalize_text_value(value.get("param_source")),
        "source_run_artifact_path": normalize_text_value(value.get("source_run_artifact_path")),
        "source_run_config_fingerprint": normalize_text_value(value.get("source_run_config_fingerprint")),
        "source_run_route_provenance": sanitize_live_session_route_provenance(value.get("source_run_route_provenance")),
        "events": value
            .get("events")
            .and_then(Value::as_array)
            .map(|events| {
                events
                    .iter()
                    .map(sanitize_live_session_event_value)
                    .take(LIVE_SESSION_EVENT_LIMIT)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        "broker_snapshots": value
            .get("broker_snapshots")
            .and_then(Value::as_array)
            .map(|snapshots| {
                snapshots
                    .iter()
                    .map(sanitize_live_session_broker_snapshot_value)
                    .take(LIVE_SESSION_SNAPSHOT_LIMIT)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    })
}

fn with_live_session_artifact_mutation<R: tauri::Runtime, T>(
    path: String,
    app: tauri::AppHandle<R>,
    mutate: impl FnOnce(&mut Value) -> Result<T, String>,
) -> Result<T, String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path.trim());
    ensure_path_is_within_dir(&path, &dir)?;
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut value: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
    if !is_live_session_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    let result = mutate(&mut value)?;
    write_json_pretty(&path, &value)?;
    Ok(result)
}

fn extract_artifact_lineage_from_artifact(v: &Value) -> Option<ArtifactLineageSummary> {
    let row = v.as_object()?;
    let parent_artifact_path = row
        .get("parent_artifact_path")
        .and_then(Value::as_str)
        .or_else(|| row.get("parentArtifactPath").and_then(Value::as_str))
        .or_else(|| {
            row.get("lineage")
                .and_then(Value::as_object)
                .and_then(|lineage| {
                    lineage
                        .get("parent_artifact_path")
                        .and_then(Value::as_str)
                        .or_else(|| lineage.get("parentArtifactPath").and_then(Value::as_str))
                })
        })
        .unwrap_or("")
        .trim()
        .to_string();
    let parent_artifact_kind = row
        .get("parent_artifact_kind")
        .and_then(Value::as_str)
        .or_else(|| row.get("parentArtifactKind").and_then(Value::as_str))
        .or_else(|| {
            row.get("lineage")
                .and_then(Value::as_object)
                .and_then(|lineage| {
                    lineage
                        .get("parent_artifact_kind")
                        .and_then(Value::as_str)
                        .or_else(|| lineage.get("parentArtifactKind").and_then(Value::as_str))
                })
        })
        .unwrap_or("")
        .trim()
        .to_string();
    let lineage_relation = row
        .get("lineage_relation")
        .and_then(Value::as_str)
        .or_else(|| row.get("lineageRelation").and_then(Value::as_str))
        .or_else(|| {
            row.get("lineage")
                .and_then(Value::as_object)
                .and_then(|lineage| {
                    lineage
                        .get("lineage_relation")
                        .and_then(Value::as_str)
                        .or_else(|| lineage.get("lineageRelation").and_then(Value::as_str))
                })
        })
        .unwrap_or("")
        .trim()
        .to_string();
    if parent_artifact_path.is_empty()
        && parent_artifact_kind.is_empty()
        && lineage_relation.is_empty()
    {
        return None;
    }
    Some(ArtifactLineageSummary {
        parent_artifact_path,
        parent_artifact_kind,
        lineage_relation,
    })
}

fn normalize_dataset_meta(ds: &mut serde_json::Map<String, Value>, csv_path: &str) {
    if !csv_path.trim().is_empty() {
        ds.entry("csv_path".to_string())
            .or_insert_with(|| Value::String(csv_path.to_string()));
    }

    ds.entry("schema_version".to_string())
        .or_insert_with(|| Value::Number(DATASET_META_SCHEMA_VERSION.into()));

    let timeframe = non_empty_string(ds.get("timeframe"));
    let mut base_timeframe = non_empty_string(ds.get("base_timeframe"));
    let mut derived_timeframe =
        non_empty_string(ds.get("derived_timeframe")).or_else(|| timeframe.clone());

    if timeframe.is_none() {
        if let Some(tf) = &derived_timeframe {
            ds.insert("timeframe".to_string(), Value::String(tf.clone()));
        }
    }
    if derived_timeframe.is_none() {
        if let Some(tf) = &timeframe {
            derived_timeframe = Some(tf.clone());
            ds.insert("derived_timeframe".to_string(), Value::String(tf.clone()));
        }
    } else if ds.get("derived_timeframe").is_none() {
        if let Some(tf) = &derived_timeframe {
            ds.insert("derived_timeframe".to_string(), Value::String(tf.clone()));
        }
    }

    if base_timeframe.is_none() {
        if let Some(tf) = &derived_timeframe {
            base_timeframe = Some(tf.clone());
            ds.insert("base_timeframe".to_string(), Value::String(tf.clone()));
        }
    } else if ds.get("base_timeframe").is_none() {
        if let Some(tf) = &base_timeframe {
            ds.insert("base_timeframe".to_string(), Value::String(tf.clone()));
        }
    }

    let resample_applied = match (&base_timeframe, &derived_timeframe) {
        (Some(base), Some(derived)) => base != derived,
        _ => false,
    };
    if ds
        .get("resample_applied")
        .and_then(|v| v.as_bool())
        .is_none()
    {
        ds.insert(
            "resample_applied".to_string(),
            Value::Bool(resample_applied),
        );
    }

    if derived_timeframe.is_some() {
        if non_empty_string(ds.get("resample_rule")).is_none() {
            ds.insert(
                "resample_rule".to_string(),
                Value::String(RESAMPLE_RULE_SESSION_RELATIVE_OHLCV.to_string()),
            );
        }
        if non_empty_string(ds.get("resample_anchor")).is_none() {
            ds.insert(
                "resample_anchor".to_string(),
                Value::String(RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP.to_string()),
            );
        }
    }
}

pub(super) fn save_run_artifact_blocking<R: tauri::Runtime>(
    mut artifact: Value,
    app: tauri::AppHandle<R>,
    existing_path: Option<String>,
) -> Result<SaveRunArtifactResponse, String> {
    let path = match existing_path.as_deref() {
        Some(value) => existing_run_artifact_target_path(value, &app)?,
        None => run_artifact_path(&artifact, &app)?,
    };

    // Best-effort enrichment for reproducibility and batch "skip completed" correctness.
    if let Value::Object(root) = &mut artifact {
        // Inject minimal environment metadata if missing.
        {
            let env_v = root
                .entry("env".to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if !env_v.is_object() {
                *env_v = Value::Object(serde_json::Map::new());
            }
            if let Some(env) = env_v.as_object_mut() {
                let pkg = app.package_info();
                env.entry("app_name".to_string())
                    .or_insert_with(|| Value::String(pkg.name.clone()));
                env.entry("app_version".to_string())
                    .or_insert_with(|| Value::String(pkg.version.to_string()));
                env.entry("os".to_string())
                    .or_insert_with(|| Value::String(std::env::consts::OS.to_string()));
                env.entry("arch".to_string())
                    .or_insert_with(|| Value::String(std::env::consts::ARCH.to_string()));
            }
        }

        if root
            .get("config_fingerprint_version")
            .and_then(|v| v.as_u64())
            .is_none()
        {
            let cfg = root
                .get("config_fingerprint")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let cfg_ver = if cfg.starts_with("cfg1-") {
                1u64
            } else {
                CONFIG_FINGERPRINT_SCHEMA_VERSION
            };
            root.insert(
                "config_fingerprint_version".to_string(),
                Value::Number(cfg_ver.into()),
            );
        }

        // Ensure dataset identity is present (size + mtime) so artifacts can be matched even if files change.
        let csv_path = root
            .get("data")
            .and_then(|v| v.get("csv_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if !csv_path.is_empty() {
            let ds_v = root
                .entry("dataset_meta".to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if !ds_v.is_object() {
                *ds_v = Value::Object(serde_json::Map::new());
            }
            if let Some(ds) = ds_v.as_object_mut() {
                normalize_dataset_meta(ds, &csv_path);

                let has_size = ds
                    .get("source_size_bytes")
                    .and_then(|v| v.as_u64())
                    .is_some();
                let has_mtime = ds
                    .get("source_mtime_utc_ms")
                    .and_then(|v| v.as_i64())
                    .is_some();
                let has_fp = ds
                    .get("source_content_fingerprint")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
                if !has_size || !has_mtime {
                    if let Ok(id) = crate::data_cache::identity_for_source(Path::new(&csv_path)) {
                        if !has_size {
                            ds.insert(
                                "source_size_bytes".to_string(),
                                Value::Number(serde_json::Number::from(id.size)),
                            );
                        }
                        if !has_mtime {
                            ds.insert(
                                "source_mtime_utc_ms".to_string(),
                                Value::Number(serde_json::Number::from(id.mtime_utc_ms)),
                            );
                        }
                    }
                }
                if !has_fp {
                    if let Ok(fp) =
                        crate::data_cache::content_fingerprint_for_source(Path::new(&csv_path))
                    {
                        ds.insert("source_content_fingerprint".to_string(), Value::String(fp));
                    }
                }
            }
        }

        if !root.contains_key("route_provenance") {
            if let Some(route_provenance) =
                extract_route_provenance_from_artifact(&Value::Object(root.clone()))
            {
                if let Ok(value) = serde_json::to_value(route_provenance) {
                    root.insert("route_provenance".to_string(), value);
                }
            }
        }
        if !root.contains_key("lineage") {
            if let Some(lineage) =
                extract_artifact_lineage_from_artifact(&Value::Object(root.clone()))
            {
                if let Ok(value) = serde_json::to_value(lineage) {
                    root.insert("lineage".to_string(), value);
                }
            }
        }
    }

    if let Some(metric_surface) = build_optimization_metric_surface_artifact_value(&artifact) {
        if let Value::Object(root) = &mut artifact {
            root.insert("optimization_metric_surface".to_string(), metric_surface);
        }
    }

    let bytes = serde_json::to_vec_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    // Retention policy: keep newest N untagged runs, but never delete tagged runs.
    prune_old_run_artifacts_blocking(&app, 200);
    Ok(SaveRunArtifactResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

pub(super) fn save_batch_summary_blocking(
    summary: Value,
    app: tauri::AppHandle,
) -> Result<SaveBatchSummaryResponse, String> {
    let ts = summary
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let path = suggest_batch_summary_path(ts, &app)?;
    let bytes = serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    prune_old_batch_summary_artifacts_blocking(&app, 100);
    Ok(SaveBatchSummaryResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn prune_old_artifacts_by_prefix<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    keep_newest: usize,
    prefix: &str,
) {
    let Ok(dir) = runs_dir(app) else { return };
    prune_old_artifacts_by_prefixes_in_dir(&dir, keep_newest, &[prefix]);
}

fn prune_old_run_artifacts_blocking<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    keep_newest: usize,
) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_run_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_run_");
}

pub(super) fn prune_old_meta_artifacts_blocking<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    keep_newest: usize,
) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_meta_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_meta_");
}

pub(super) fn prune_old_stage4_artifacts_blocking<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    keep_newest: usize,
) {
    let Ok(dir) = runs_dir(app) else { return };
    prune_old_artifacts_by_prefixes_in_dir(
        &dir,
        keep_newest,
        &[
            "vectorgrid_stage4_",
            "vectorgrid_meta_stage4_",
            "vectorgrid_stage5_",
            "vectorgrid_meta_stage5_",
            "vectorbt_stage4_",
            "vectorbt_meta_stage4_",
            "vectorbt_stage5_",
            "vectorbt_meta_stage5_",
        ],
    );
}

pub(super) fn prune_old_ml_policy_eval_artifacts_blocking(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    keep_newest: usize,
) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_ml_confidence_eval_");
    prune_old_artifacts_by_prefix(
        app,
        keep_newest,
        "vectorgrid_ml_confidence_policy_evaluation_",
    );
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_ml_confidence_eval_");
    prune_old_artifacts_by_prefix(
        app,
        keep_newest,
        "vectorbt_ml_confidence_policy_evaluation_",
    );
}

pub(super) fn prune_old_batch_summary_artifacts_blocking(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    keep_newest: usize,
) {
    let Ok(dir) = runs_dir(app) else { return };
    prune_old_artifacts_by_prefixes_in_dir(
        &dir,
        keep_newest,
        &[
            "vectorgrid_batch_",
            "vectorgrid_batch_summary_",
            "vectorbt_batch_",
            "vectorbt_batch_summary_",
        ],
    );
}

pub(super) fn prune_old_campaign_summary_artifacts_blocking(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    keep_newest: usize,
) {
    let Ok(dir) = runs_dir(app) else { return };
    prune_old_artifacts_by_prefixes_in_dir(
        &dir,
        keep_newest,
        &["vectorgrid_campaign_summary_", "vectorbt_campaign_summary_"],
    );
}

pub(super) fn prune_old_options_expression_artifacts_in_dir(dir: &Path, keep_newest: usize) {
    prune_old_artifacts_by_prefixes_in_dir(dir, keep_newest, &["vectorgrid_options_expression_"]);
}

fn read_json_value(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn scan_artifact_library(
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<Vec<ArtifactScanEntry>, String> {
    let dir = runs_dir(app)?;
    let mut paths: Vec<(PathBuf, String)> = Vec::new();

    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(file_name) = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
        else {
            continue;
        };
        paths.push((path, file_name));
    }

    paths.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

    let mut out = Vec::with_capacity(paths.len());
    for (path, file_name) in paths {
        let Some(entry) = read_artifact_scan_entry(path, file_name) else {
            continue;
        };
        out.push(entry);
    }

    Ok(out)
}

fn read_artifact_scan_entry(path: PathBuf, file_name: String) -> Option<ArtifactScanEntry> {
    let bytes = std::fs::metadata(&path).ok()?.len();
    let value = read_json_value(&path)?;
    Some(ArtifactScanEntry {
        path,
        file_name,
        bytes,
        value,
    })
}

fn run_artifact_selection_key(entry: &ArtifactScanEntry) -> (String, String, String) {
    (
        resolved_run_artifact_ts(&entry.path, &entry.value),
        entry.file_name.clone(),
        entry.path.to_string_lossy().to_string(),
    )
}

fn load_latest_run_artifact_entry_blocking(
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<Option<ArtifactScanEntry>, String> {
    let dir = runs_dir(app)?;
    let mut candidate_paths: Vec<(PathBuf, String)> = Vec::new();

    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(file_name) = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
        else {
            continue;
        };
        if !is_run_artifact_file_name(&file_name) {
            continue;
        }
        candidate_paths.push((path, file_name));
    }

    candidate_paths.sort_by(|lhs, rhs| {
        let lhs_key = run_artifact_file_recency_key(&lhs.1);
        let rhs_key = run_artifact_file_recency_key(&rhs.1);
        rhs_key
            .cmp(&lhs_key)
            .then_with(|| rhs.1.cmp(&lhs.1))
            .then_with(|| rhs.0.cmp(&lhs.0))
    });
    for (path, file_name) in candidate_paths {
        let Some(entry) = read_artifact_scan_entry(path, file_name) else {
            continue;
        };
        let kind = entry
            .value
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("");
        if is_run_artifact_kind(kind) {
            return Ok(Some(entry));
        }
    }

    Ok(scan_artifact_library(app)?
        .into_iter()
        .filter(|entry| {
            let kind = entry
                .value
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("");
            is_run_artifact_kind(kind)
        })
        .max_by(|lhs, rhs| run_artifact_selection_key(lhs).cmp(&run_artifact_selection_key(rhs))))
}

fn load_artifact_entry_checked(
    path: String,
    app: tauri::AppHandle<impl tauri::Runtime>,
) -> Result<ArtifactScanEntry, String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path.trim());
    ensure_path_is_within_dir(&path, &dir)?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "artifact path is missing a filename".to_string())?;
    let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    Ok(ArtifactScanEntry {
        path,
        file_name,
        bytes,
        value,
    })
}

fn normalized_tags(v: &Value) -> Vec<String> {
    v.get("tags")
        .and_then(|x| x.as_array())
        .map(|xs| {
            xs.iter()
                .filter_map(|entry| entry.as_str().map(|s| s.trim().to_string()))
                .filter(|tag| !tag.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn clone_value_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

fn normalize_text_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => {
            if let Some(integer) = number.as_i64() {
                integer.to_string()
            } else if let Some(integer) = number.as_u64() {
                integer.to_string()
            } else if let Some(float) = number.as_f64() {
                if float.fract() == 0.0 {
                    (float as i64).to_string()
                } else {
                    float.to_string()
                }
            } else {
                number.to_string()
            }
        }
        Some(Value::Bool(boolean)) => boolean.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').trim().to_string(),
    }
}

fn normalize_report_xlsx_fields(value: Option<&Value>) -> (String, Option<bool>) {
    let path = normalize_text_value(value);
    let exists = if path.is_empty() {
        None
    } else {
        Some(Path::new(&path).exists())
    };
    (path, exists)
}

fn normalize_optional_text_value(value: Option<&Value>) -> Option<String> {
    let text = normalize_text_value(value);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn normalize_u32_value(value: Option<&Value>) -> Option<u32> {
    match value {
        Some(Value::Number(number)) => number.as_u64().map(|value| value as u32),
        Some(Value::String(text)) => text.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn ml_signal_text_value(artifact: &Value, field: &str) -> String {
    normalize_text_value(artifact.pointer(&format!("/ml_signal/{field}")))
}

fn ml_signal_optional_text_value(artifact: &Value, field: &str) -> Option<String> {
    normalize_optional_text_value(artifact.pointer(&format!("/ml_signal/{field}")))
}

fn ml_signal_u32_value(artifact: &Value, field: &str) -> Option<u32> {
    normalize_u32_value(artifact.pointer(&format!("/ml_signal/{field}")))
}

fn ml_signal_provenance_text_value(artifact: &Value, field: &str) -> String {
    ml_signal_optional_provenance_text_value(artifact, field).unwrap_or_default()
}

fn ml_signal_optional_provenance_text_value(artifact: &Value, field: &str) -> Option<String> {
    ml_signal_optional_text_value(artifact, field).or_else(|| {
        normalize_optional_text_value(
            artifact.pointer(&format!("/ml_signal/score_cache_provenance/{field}")),
        )
    })
}

fn ml_signal_provenance_u32_value(artifact: &Value, field: &str) -> Option<u32> {
    ml_signal_u32_value(artifact, field).or_else(|| {
        normalize_u32_value(artifact.pointer(&format!("/ml_signal/score_cache_provenance/{field}")))
    })
}

fn canonical_ml_score_space_text(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "probability" | "probabilities" => "Probability".to_string(),
        "rawmargin" | "raw_margin" | "raw-margin" | "margin" => "RawMargin".to_string(),
        _ => raw.trim().to_string(),
    }
}

fn canonical_ml_score_scope_text(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fulldataset" | "full_dataset" | "full-dataset" => "FullDataset".to_string(),
        "calibrationslice" | "calibration_slice" | "calibration-slice" => {
            "CalibrationSlice".to_string()
        }
        "walkforwardtest" | "walkforward_test" | "walk-forward-test" | "walk_forward_test" => {
            "WalkforwardTest".to_string()
        }
        "custompartial" | "custom_partial" | "custom-partial" => "CustomPartial".to_string(),
        _ => raw.trim().to_string(),
    }
}

fn normalize_ml_score_text(value: Option<&Value>, scope: bool) -> Option<String> {
    let raw = normalize_text_value(value);
    if raw.is_empty() {
        return None;
    }
    Some(if scope {
        canonical_ml_score_scope_text(&raw)
    } else {
        canonical_ml_score_space_text(&raw)
    })
}

fn extract_ml_score_space(artifact: &Value) -> Option<String> {
    for pointer in [
        "/ml_signal/score_space",
        "/validation/res/score_summary/score_space",
        "/run/res/score_space",
    ] {
        if let Some(text) = normalize_ml_score_text(artifact.pointer(pointer), false) {
            return Some(text);
        }
    }
    None
}

fn extract_ml_score_scope(artifact: &Value) -> Option<String> {
    for pointer in [
        "/ml_signal/score_scope",
        "/validation/res/score_summary/score_scope",
        "/run/res/score_scope",
    ] {
        if let Some(text) = normalize_ml_score_text(artifact.pointer(pointer), true) {
            return Some(text);
        }
    }
    None
}

fn format_ml_score_contract_summary(
    score_space: Option<&str>,
    score_scope: Option<&str>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(space) = score_space.filter(|value| !value.trim().is_empty()) {
        parts.push(space.trim().to_string());
    }
    if let Some(scope) = score_scope.filter(|value| !value.trim().is_empty()) {
        parts.push(scope.trim().to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" • "))
    }
}

fn format_ml_score_note(score_space: Option<&str>, score_scope: Option<&str>) -> Option<String> {
    let mut notes = Vec::new();
    match score_space.map(|value| value.trim()) {
        Some("RawMargin") => notes.push(
            "Raw-margin scores are ranking-oriented only; threshold validation expects probability columns."
                .to_string(),
        ),
        _ => {}
    }
    match score_scope.map(|value| value.trim()) {
        Some("WalkforwardTest") => {
            notes.push("Held-out walk-forward test scores.".to_string());
        }
        Some("CalibrationSlice") => notes.push(
            "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence."
                .to_string(),
        ),
        Some("FullDataset") => notes.push(
            "Full-dataset scores; saved validation metrics are in-sample, not holdout-like evidence."
                .to_string(),
        ),
        Some("CustomPartial") => {
            notes.push("Partial-score cache; saved metrics only apply to the scored subset.".to_string())
        }
        _ => {}
    }
    if notes.is_empty() {
        None
    } else {
        Some(notes.join(" "))
    }
}

fn normalize_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| normalize_text_value(Some(item)))
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn format_range_label(value: Option<&Value>) -> String {
    let Some(range) = value.and_then(Value::as_object) else {
        return "-".to_string();
    };
    let start = range.get("start").and_then(Value::as_f64);
    let end = range.get("end").and_then(Value::as_f64);
    let step = range.get("step").and_then(Value::as_f64);
    let (Some(start), Some(end)) = (start, end) else {
        return "-".to_string();
    };
    let start = start.floor() as i64;
    let end = end.floor() as i64;
    match step.filter(|step| step.is_finite() && *step > 0.0) {
        Some(step) => format!("{start}-{end} x{}", step.floor() as i64),
        None => format!("{start}-{end}"),
    }
}

fn format_sweep_axis(value: Option<&Value>) -> String {
    let Some(axis) = value.and_then(Value::as_object) else {
        return String::new();
    };
    let start = normalize_text_value(axis.get("start"));
    let end = normalize_text_value(axis.get("end"));
    let step = normalize_text_value(axis.get("step"));
    if start.is_empty() && end.is_empty() && step.is_empty() {
        return String::new();
    }
    [start, end, step]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn format_ml_policy_summary(value: Option<&Value>) -> String {
    let Some(policy) = value.and_then(Value::as_object) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for (key, label) in [
        ("t_long", "t_long"),
        ("t_short", "t_short"),
        ("delta", "delta"),
        ("max_hold_bars", "hold"),
    ] {
        let text = normalize_text_value(policy.get(key));
        if !text.is_empty() {
            parts.push(format!("{label}={text}"));
        }
    }
    parts.join(" | ")
}

fn format_ml_policy_sweep_summary(value: Option<&Value>) -> String {
    let Some(sweep) = value.and_then(Value::as_object) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for (key, label) in [("sweep_size", "size"), ("top_k", "top_k")] {
        let text = normalize_text_value(sweep.get(key));
        if !text.is_empty() {
            parts.push(format!("{label}={text}"));
        }
    }
    for key in ["best_label", "best_robust_score"] {
        let text = normalize_text_value(sweep.get(key));
        if !text.is_empty() {
            let label = if key == "best_robust_score" {
                "score"
            } else {
                "best"
            };
            parts.push(format!("{label}={text}"));
        }
    }
    parts.join(" | ")
}

fn format_ml_run_verdict_summary(validation_res: Option<&Value>) -> String {
    let verdict = validation_res
        .and_then(|value| value.pointer("/run_verdict"))
        .and_then(Value::as_object);
    let Some(verdict) = verdict else {
        return String::new();
    };
    let mut parts = Vec::new();
    for (key, label) in [
        ("approved_candidate_count", "approved"),
        ("candidates_survived_hard_gates", "survived"),
        ("candidates_considered", "considered"),
    ] {
        let text = normalize_text_value(verdict.get(key));
        if !text.is_empty() {
            parts.push(format!("{label}={text}"));
        }
    }
    parts.join(" | ")
}

fn format_ml_top_candidates_summary(validation_res: Option<&Value>) -> String {
    let candidate = validation_res
        .and_then(|value| value.pointer("/candidates"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_object);
    let Some(candidate) = candidate else {
        return String::new();
    };

    let label = candidate
        .get("label")
        .or_else(|| {
            candidate
                .get("params")
                .and_then(Value::as_object)
                .and_then(|params| params.get("label"))
        })
        .map(|value| normalize_text_value(Some(value)))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "candidate".to_string());

    let mut parts = vec![label];
    for key in ["robust_score", "psr", "dsr"] {
        let text = normalize_text_value(candidate.get(key));
        if !text.is_empty() {
            let label = match key {
                "robust_score" => "robust",
                other => other,
            };
            parts.push(format!("{label}={text}"));
        }
    }
    parts.join(" | ")
}

fn build_req_snapshot(run_req: Option<&Value>, validation_req: Option<&Value>) -> String {
    let Some(run_req) = run_req.and_then(Value::as_object) else {
        return "Request unavailable.".to_string();
    };

    let strategy_kind = normalize_text_value(run_req.get("strategy_kind"));
    if strategy_kind == "MlSignal" {
        let mut lines = vec!["strategy: MlSignal".to_string()];
        for (key, label) in [
            ("data_id", "data id"),
            ("dataId", "data id"),
            ("input_path", "score input"),
            ("inputPath", "score input"),
            ("output_path", "dataset/score output"),
            ("outputPath", "dataset/score output"),
            ("score_cache_path", "score cache"),
            ("scoreCachePath", "score cache"),
            ("model_bundle_id", "model bundle"),
            ("modelBundleId", "model bundle"),
        ] {
            let text = normalize_text_value(run_req.get(key));
            if !text.is_empty() && !lines.iter().any(|line| line == &format!("{label}: {text}")) {
                lines.push(format!("{label}: {text}"));
            }
        }

        if let Some(validation_req) = validation_req.and_then(Value::as_object) {
            if let Some(candidate) = validation_req
                .get("candidates")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_object)
            {
                for (key, label) in [
                    ("t_long", "policy t_long"),
                    ("t_short", "policy t_short"),
                    ("delta", "policy delta"),
                    ("max_hold_bars", "policy max_hold_bars"),
                ] {
                    let text = normalize_text_value(candidate.get(key));
                    if !text.is_empty() {
                        lines.push(format!("{label}: {text}"));
                    }
                }
            } else {
                for (key, label) in [
                    ("t_long", "sweep t_long"),
                    ("t_short", "sweep t_short"),
                    ("delta", "sweep delta"),
                    ("max_hold_bars", "sweep hold"),
                ] {
                    let axis = format_sweep_axis(validation_req.get(key));
                    if !axis.is_empty() {
                        lines.push(format!("{label}: {axis}"));
                    }
                }
                let top_k = normalize_text_value(validation_req.get("top_k"));
                if !top_k.is_empty() {
                    lines.push(format!("sweep top_k: {top_k}"));
                }
            }
        }

        return lines.join("\n");
    }

    let backend = run_req
        .get("backend")
        .and_then(Value::as_object)
        .and_then(|backend| backend.get("backend"))
        .map(|value| normalize_text_value(Some(value)))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "CpuOnly".to_string());
    let objective = run_req
        .get("objective")
        .map(|value| normalize_text_value(Some(value)))
        .filter(|value| !value.is_empty())
        .or_else(|| {
            run_req
                .get("objectives")
                .and_then(Value::as_object)
                .and_then(|objectives| objectives.get("maximize"))
                .map(|value| normalize_text_value(Some(value)))
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "-".to_string());
    let ma_source = normalize_text_value(run_req.get("ma_source"));
    let ma_source = if ma_source.is_empty() {
        "close".to_string()
    } else {
        ma_source
    };
    let fast_ma = normalize_string_list(run_req.get("fast_ma_types"));
    let slow_ma = normalize_string_list(run_req.get("slow_ma_types"));
    let starting_equity = run_req.get("starting_equity").and_then(Value::as_f64);

    let mut lines = vec![
        format!(
            "strategy: {}",
            if strategy_kind.is_empty() {
                "-"
            } else {
                strategy_kind.as_str()
            }
        ),
        format!("backend: {backend}"),
        format!("objective: {objective}"),
        format!("ma source: {ma_source}"),
        format!(
            "fast range: {}",
            format_range_label(run_req.get("fast_range"))
        ),
        format!(
            "slow range: {}",
            format_range_label(run_req.get("slow_range"))
        ),
        format!(
            "fast ma: {}",
            if fast_ma.is_empty() {
                "-".to_string()
            } else {
                fast_ma.join(", ")
            }
        ),
        format!(
            "slow ma: {}",
            if slow_ma.is_empty() {
                "-".to_string()
            } else {
                slow_ma.join(", ")
            }
        ),
    ];

    if let Some(starting_equity) = starting_equity.filter(|value| value.is_finite() && *value > 0.0)
    {
        let formatted = if starting_equity.fract() == 0.0 {
            format!("{}", starting_equity as i64)
        } else {
            starting_equity.to_string()
        };
        lines.push(format!("starting equity: {formatted}"));
    }

    lines.join("\n")
}

fn build_workflow_artifacts_hydration_payload(
    artifact_path: &str,
    artifact: &Value,
) -> Result<WorkflowArtifactsHydrationPayload, String> {
    let run_req = artifact
        .pointer("/run/req")
        .cloned()
        .ok_or_else(|| "Saved run artifact is missing run.req.".to_string())?;
    let run_res = artifact
        .pointer("/run/res")
        .cloned()
        .ok_or_else(|| "Saved run artifact is missing run.res.".to_string())?;
    let resolved_ts = resolved_run_artifact_ts(Path::new(artifact_path), artifact);
    let (config_fingerprint, config_fingerprint_version) =
        normalized_config_fingerprint_fields(artifact);
    let (report_xlsx_path, report_xlsx_exists) =
        normalize_report_xlsx_fields(artifact.get("report_xlsx_path"));
    let current_run = WorkflowArtifactRunPayload {
        ts: resolved_ts.clone(),
        artifact_path: artifact_path.to_string(),
        report_xlsx_path,
        report_xlsx_exists,
        csv_path: resolve_run_artifact_csv_path(artifact),
        config_fingerprint,
        config_fingerprint_version,
        dataset_meta: artifact.pointer("/dataset_meta").cloned(),
        route_provenance: extract_route_provenance_from_artifact(artifact),
        lineage: extract_artifact_lineage_from_artifact(artifact),
        ml_signal: artifact.pointer("/ml_signal").cloned(),
        optimization_metric_surface: artifact.pointer("/optimization_metric_surface").cloned(),
        tags: normalized_tags(artifact),
        req: run_req,
        res: run_res,
    };
    let current_validation =
        artifact
            .pointer("/validation/res")
            .cloned()
            .map(|res| WorkflowArtifactValidationPayload {
                ts: resolved_ts,
                req: clone_value_or_null(artifact.pointer("/validation/req")),
                res,
            });

    Ok(WorkflowArtifactsHydrationPayload {
        current_run,
        current_validation,
    })
}

fn build_run_artifact_detail_response(
    artifact_path: &str,
    artifact: &Value,
) -> Result<RunArtifactDetailResponse, String> {
    let resolved_ts = resolved_run_artifact_ts(Path::new(artifact_path), artifact);
    let dataset_meta = artifact.pointer("/dataset_meta").cloned();
    let ml_signal = artifact.pointer("/ml_signal").cloned();
    let optimization_metric_surface = artifact.pointer("/optimization_metric_surface").cloned();
    let run_req = artifact.pointer("/run/req").cloned();
    let run_res = artifact.pointer("/run/res").cloned();
    let validation_req = artifact.pointer("/validation/req").cloned();
    let validation_res = artifact.pointer("/validation/res").cloned();
    let (config_fingerprint, config_fingerprint_version) =
        normalized_config_fingerprint_fields(artifact);
    let ml_score_space = extract_ml_score_space(artifact).unwrap_or_default();
    let ml_score_scope = extract_ml_score_scope(artifact).unwrap_or_default();
    let ml_score_contract_summary = format_ml_score_contract_summary(
        (!ml_score_space.is_empty()).then_some(ml_score_space.as_str()),
        (!ml_score_scope.is_empty()).then_some(ml_score_scope.as_str()),
    )
    .unwrap_or_default();
    let ml_score_note = format_ml_score_note(
        (!ml_score_space.is_empty()).then_some(ml_score_space.as_str()),
        (!ml_score_scope.is_empty()).then_some(ml_score_scope.as_str()),
    )
    .unwrap_or_default();
    let (report_xlsx_path, report_xlsx_exists) =
        normalize_report_xlsx_fields(artifact.get("report_xlsx_path"));

    Ok(RunArtifactDetailResponse {
        ts: resolved_ts,
        artifact_path: artifact_path.to_string(),
        kind: normalize_text_value(artifact.get("kind")),
        strategy_kind: resolve_run_artifact_strategy_kind(artifact),
        timeframe: resolve_run_artifact_timeframe(artifact),
        csv_path: resolve_run_artifact_csv_path(artifact),
        report_xlsx_path: report_xlsx_path.clone(),
        report_xlsx_exists,
        report_path: report_xlsx_path,
        report_path_exists: report_xlsx_exists,
        has_validation: validation_res.is_some(),
        config_fingerprint,
        config_fingerprint_version,
        tags: normalized_tags(artifact),
        dataset_meta,
        route_provenance: extract_route_provenance_from_artifact(artifact),
        lineage: extract_artifact_lineage_from_artifact(artifact),
        ml_signal: ml_signal.clone(),
        optimization_metric_surface,
        run_req: run_req.clone(),
        run_res: run_res.clone(),
        validation_req: validation_req.clone(),
        validation_res: validation_res.clone(),
        ml_signal_stage: normalize_text_value(artifact.pointer("/ml_signal/stage")),
        ml_bundle_id: normalize_text_value(artifact.pointer("/ml_signal/bundle_id")),
        ml_score_cache_path: normalize_text_value(artifact.pointer("/ml_signal/score_cache_path")),
        ml_score_manifest_path: normalize_text_value(
            artifact.pointer("/ml_signal/score_manifest_path"),
        ),
        ml_feature_schema_path: normalize_text_value(
            artifact.pointer("/ml_signal/feature_schema_path"),
        ),
        ml_feature_schema_version: normalize_text_value(
            artifact.pointer("/ml_signal/feature_schema_version"),
        ),
        ml_feature_schema_hash: ml_signal_text_value(artifact, "feature_schema_hash"),
        ml_export_manifest_path: ml_signal_text_value(artifact, "export_manifest_path"),
        ml_source_bundle_manifest_path: ml_signal_text_value(
            artifact,
            "source_bundle_manifest_path",
        ),
        ml_source_bundle_manifest_hash: ml_signal_text_value(
            artifact,
            "source_bundle_manifest_hash",
        ),
        ml_calibration_state: ml_signal_text_value(artifact, "calibration_state"),
        ml_threshold_policy: ml_signal_text_value(artifact, "threshold_policy"),
        ml_producer_kind: ml_signal_text_value(artifact, "producer_kind"),
        ml_native_backend_requested: ml_signal_provenance_text_value(
            artifact,
            "native_backend_requested",
        ),
        ml_native_backend_used: ml_signal_provenance_text_value(artifact, "native_backend_used"),
        ml_native_library_path: ml_signal_provenance_text_value(artifact, "native_library_path"),
        ml_native_gpu_device_id: ml_signal_provenance_u32_value(artifact, "native_gpu_device_id"),
        ml_native_fallback_reason: ml_signal_provenance_text_value(
            artifact,
            "native_fallback_reason",
        ),
        ml_score_contract_version: ml_signal_u32_value(artifact, "score_contract_version"),
        ml_score_space,
        ml_score_scope,
        ml_score_contract_summary,
        ml_score_note,
        ml_policy_summary: format_ml_policy_summary(artifact.pointer("/ml_signal/policy")),
        ml_policy_sweep_summary: format_ml_policy_sweep_summary(
            artifact.pointer("/ml_signal/policy_sweep"),
        ),
        ml_run_verdict_summary: format_ml_run_verdict_summary(validation_res.as_ref()),
        ml_top_candidates_summary: format_ml_top_candidates_summary(validation_res.as_ref()),
        params_text: build_req_snapshot(run_req.as_ref(), validation_req.as_ref()),
    })
}

fn build_live_session_event_detail(value: &Value) -> Option<LiveSessionEventDetail> {
    let provenance = normalize_text_value(value.get("provenance"));
    let status = normalize_text_value(value.get("status"));
    let message = normalize_text_value(value.get("message"));
    if provenance.is_empty() && status.is_empty() && message.is_empty() {
        return None;
    }
    Some(LiveSessionEventDetail {
        provenance,
        status,
        message,
        symbol: normalize_text_value(value.get("symbol")),
        at_unix_ms: value.get("at_unix_ms").and_then(Value::as_i64),
        bar_open_unix_ms: value.get("bar_open_unix_ms").and_then(Value::as_i64),
        action: normalize_text_value(value.get("action")),
        order_id: normalize_text_value(value.get("order_id")),
        delta_qty: value.get("delta_qty").and_then(Value::as_f64),
        order_submitted: value.get("order_submitted").and_then(Value::as_bool),
        warnings: normalize_string_list(value.get("warnings")),
    })
}

fn build_live_session_broker_snapshot_detail(
    value: &Value,
) -> Option<LiveSessionBrokerSnapshotDetail> {
    let source_command = normalize_text_value(value.get("source_command"));
    let symbol = normalize_text_value(value.get("symbol"));
    let mode = normalize_text_value(value.get("mode"));
    if source_command.is_empty() && symbol.is_empty() && mode.is_empty() {
        return None;
    }
    Some(LiveSessionBrokerSnapshotDetail {
        source_command,
        mode,
        symbol,
        at_unix_ms: value.get("at_unix_ms").and_then(Value::as_i64),
        account_status: normalize_text_value(value.get("account_status")),
        currency: normalize_text_value(value.get("currency")),
        position_state: normalize_text_value(value.get("position_state")),
        position_qty: value.get("position_qty").and_then(Value::as_f64),
        position_side: normalize_text_value(value.get("position_side")),
        open_order_count: value
            .get("open_order_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        order_count: value
            .get("order_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
    })
}

fn build_live_session_artifact_detail_response(
    artifact_path: &str,
    artifact: &Value,
) -> Result<LiveSessionArtifactDetailResponse, String> {
    let kind = artifact.get("kind").and_then(Value::as_str).unwrap_or("");
    if !is_live_session_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }

    let ts = normalize_text_value(artifact.get("ts"));
    let events = artifact
        .get("events")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(build_live_session_event_detail)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let broker_snapshots = artifact
        .get("broker_snapshots")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(build_live_session_broker_snapshot_detail)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(LiveSessionArtifactDetailResponse {
        ts: if ts.is_empty() {
            chrono::Utc::now().to_rfc3339()
        } else {
            ts
        },
        artifact_path: artifact_path.to_string(),
        session_id: normalize_text_value(artifact.get("session_id")),
        session_state: normalize_text_value(artifact.get("session_state")),
        mode: normalize_text_value(artifact.get("mode")),
        symbol: normalize_text_value(artifact.get("symbol")),
        qty: artifact.get("qty").and_then(Value::as_f64).unwrap_or(0.0),
        dry_run: artifact
            .get("dry_run")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        strategy_kind: normalize_text_value(artifact.get("strategy_kind")),
        data_id: normalize_text_value(artifact.get("data_id")),
        param_source: normalize_text_value(artifact.get("param_source")),
        created_at_unix_ms: artifact.get("created_at_unix_ms").and_then(Value::as_i64),
        finalized_at_unix_ms: artifact.get("finalized_at_unix_ms").and_then(Value::as_i64),
        source_run_artifact_path: normalize_text_value(artifact.get("source_run_artifact_path")),
        source_run_config_fingerprint: normalize_text_value(
            artifact.get("source_run_config_fingerprint"),
        ),
        source_run_route_provenance: extract_route_provenance_from_value(
            artifact.get("source_run_route_provenance"),
        ),
        event_count: events.len(),
        broker_snapshot_count: broker_snapshots.len(),
        events,
        broker_snapshots,
    })
}

fn build_live_session_artifact_summary(
    path: &Path,
    bytes: u64,
    artifact: &Value,
) -> Option<LiveSessionArtifactSummary> {
    let kind = artifact.get("kind").and_then(Value::as_str).unwrap_or("");
    if !is_live_session_artifact_kind(kind) {
        return None;
    }
    let ts = normalize_text_value(artifact.get("ts"));
    let event_count = artifact
        .get("events")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let broker_snapshot_count = artifact
        .get("broker_snapshots")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    Some(LiveSessionArtifactSummary {
        ts,
        path: path.to_string_lossy().to_string(),
        bytes,
        session_id: normalize_text_value(artifact.get("session_id")),
        mode: normalize_text_value(artifact.get("mode")),
        symbol: normalize_text_value(artifact.get("symbol")),
        strategy_kind: normalize_text_value(artifact.get("strategy_kind")),
        data_id: normalize_text_value(artifact.get("data_id")),
        dry_run: artifact
            .get("dry_run")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        param_source: normalize_text_value(artifact.get("param_source")),
        session_state: normalize_text_value(artifact.get("session_state")),
        created_at_unix_ms: artifact.get("created_at_unix_ms").and_then(Value::as_i64),
        finalized_at_unix_ms: artifact.get("finalized_at_unix_ms").and_then(Value::as_i64),
        event_count,
        broker_snapshot_count,
    })
}

fn build_run_artifact_summary(path: &Path, bytes: u64, v: &Value) -> Option<RunArtifactSummary> {
    let kind = v
        .get("kind")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if !is_run_artifact_kind(&kind) {
        return None;
    }

    let ts = resolved_run_artifact_ts(path, v);
    let strategy_kind = resolve_run_artifact_strategy_kind(v);
    let (config_fingerprint, config_fingerprint_version) = normalized_config_fingerprint_fields(v);
    let has_validation = v.pointer("/validation/res").is_some();
    let (report_xlsx_path, report_xlsx_exists) =
        normalize_report_xlsx_fields(v.get("report_xlsx_path"));
    let csv_path = resolve_run_artifact_csv_path(v);
    let timeframe = resolve_run_artifact_timeframe(v);
    let base_timeframe = v
        .pointer("/dataset_meta/base_timeframe")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let derived_timeframe = v
        .pointer("/dataset_meta/derived_timeframe")
        .and_then(|x| x.as_str())
        .or_else(|| {
            v.pointer("/dataset_meta/timeframe")
                .and_then(|x| x.as_str())
        })
        .unwrap_or("")
        .to_string();
    let resample_applied = v
        .pointer("/dataset_meta/resample_applied")
        .and_then(|x| x.as_bool());
    let resample_rule = v
        .pointer("/dataset_meta/resample_rule")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let resample_anchor = v
        .pointer("/dataset_meta/resample_anchor")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let source_size_bytes = v
        .pointer("/dataset_meta/source_size_bytes")
        .and_then(|x| x.as_u64());
    let source_mtime_utc_ms = v
        .pointer("/dataset_meta/source_mtime_utc_ms")
        .and_then(|x| x.as_i64());
    let source_content_fingerprint = v
        .pointer("/dataset_meta/source_content_fingerprint")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let route_provenance = extract_route_provenance_from_artifact(v);
    let lineage = extract_artifact_lineage_from_artifact(v);
    let ml_score_space = extract_ml_score_space(v);
    let ml_score_scope = extract_ml_score_scope(v);
    let ml_score_contract_summary =
        format_ml_score_contract_summary(ml_score_space.as_deref(), ml_score_scope.as_deref());
    let ml_score_note = format_ml_score_note(ml_score_space.as_deref(), ml_score_scope.as_deref());

    Some(RunArtifactSummary {
        ts,
        path: path.to_string_lossy().to_string(),
        bytes,
        kind,
        strategy_kind,
        config_fingerprint,
        config_fingerprint_version,
        has_validation,
        report_xlsx_path,
        report_xlsx_exists,
        csv_path,
        timeframe,
        base_timeframe,
        derived_timeframe,
        resample_applied,
        resample_rule,
        resample_anchor,
        source_size_bytes,
        source_mtime_utc_ms,
        source_content_fingerprint,
        route_provenance,
        lineage,
        tags: normalized_tags(v),
        ml_feature_schema_hash: ml_signal_optional_text_value(v, "feature_schema_hash"),
        ml_calibration_state: ml_signal_optional_text_value(v, "calibration_state"),
        ml_threshold_policy: ml_signal_optional_text_value(v, "threshold_policy"),
        ml_producer_kind: ml_signal_optional_text_value(v, "producer_kind"),
        ml_native_backend_used: ml_signal_optional_provenance_text_value(v, "native_backend_used"),
        ml_native_fallback_reason: ml_signal_optional_provenance_text_value(
            v,
            "native_fallback_reason",
        ),
        ml_score_contract_version: ml_signal_u32_value(v, "score_contract_version"),
        ml_source_bundle_manifest_hash: ml_signal_optional_text_value(
            v,
            "source_bundle_manifest_hash",
        ),
        ml_score_space,
        ml_score_scope,
        ml_score_contract_summary,
        ml_score_note,
    })
}

fn build_options_expression_artifact_summary(
    path: &Path,
    bytes: u64,
    v: &Value,
) -> Option<OptionsExpressionArtifactSummary> {
    let kind = v
        .get("kind")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if !is_options_expression_artifact_kind(kind) {
        return None;
    }

    let artifact = serde_json::from_value::<OptionsExpressionArtifactV1>(v.clone()).ok()?;
    Some(summarize_options_expression_artifact(
        path, bytes, &artifact,
    ))
}

fn build_meta_artifact_summary(path: &Path, bytes: u64, v: &Value) -> Option<MetaArtifactSummary> {
    let kind = v
        .get("kind")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if !is_meta_selection_kind(&kind) {
        return None;
    }
    let (report_xlsx_path, report_xlsx_exists) =
        normalize_report_xlsx_fields(v.get("report_xlsx_path"));

    Some(MetaArtifactSummary {
        ts: v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        bytes,
        kind,
        strategy_kind: v
            .pointer("/req/strategy_kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        timeframe: v
            .pointer("/req/timeframe")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        required_tag: v
            .pointer("/req/required_tag")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        groups: v
            .pointer("/res/groups")
            .and_then(|x| x.as_array())
            .map(|xs| xs.len())
            .unwrap_or(0),
        artifacts_used: v
            .pointer("/res/artifacts_used")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        holdout_datasets: v
            .pointer("/res/holdout/datasets")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        report_xlsx_path,
        report_xlsx_exists,
        tags: normalized_tags(v),
    })
}

fn build_stage5_artifact_summary(
    path: &Path,
    bytes: u64,
    v: &Value,
) -> Option<Stage5ArtifactSummary> {
    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    if !is_meta_stage_result_kind(kind) {
        return None;
    }

    Some(Stage4ArtifactSummary {
        ts: v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        bytes,
        kind: kind.to_string(),
        strategy_kind: v
            .pointer("/res/strategy_kind")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/req/strategy_kind").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string(),
        timeframe: v
            .pointer("/res/timeframe")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/req/timeframe").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string(),
        source_meta_json_path: v
            .pointer("/res/source_meta_json_path")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/req/source_meta_path").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string(),
        groups_selected: v
            .pointer("/res/groups_selected")
            .and_then(|x| x.as_u64())
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                v.pointer("/res/selected_groups")
                    .and_then(|x| x.as_array())
                    .map(|xs| xs.len())
                    .unwrap_or(0)
            }),
        groups_scored: v
            .pointer("/res/groups_scored")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        groups_considered: v
            .pointer("/res/groups_considered")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        min_ml_confidence: v
            .pointer("/res/min_ml_confidence")
            .and_then(|x| x.as_f64())
            .or_else(|| v.pointer("/req/min_ml_confidence").and_then(|x| x.as_f64()))
            .unwrap_or(0.0),
        top_k: v
            .pointer("/res/top_k")
            .and_then(|x| x.as_u64())
            .or_else(|| v.pointer("/req/top_k").and_then(|x| x.as_u64()))
            .unwrap_or(0) as usize,
        model_healthy: v
            .pointer("/res/model_healthy")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        tags: normalized_tags(v),
    })
}

fn build_ml_policy_eval_summary(
    path: &Path,
    bytes: u64,
    v: &Value,
) -> Option<MlPolicyEvalArtifactSummary> {
    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    if !is_ml_confidence_policy_eval_kind(kind) {
        return None;
    }

    Some(MlPolicyEvalArtifactSummary {
        ts: v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        bytes,
        kind: kind.to_string(),
        strategy_kind: v
            .pointer("/res/strategy_kind")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/req/strategy_kind").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string(),
        timeframe: v
            .pointer("/res/timeframe")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/req/timeframe").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string(),
        artifacts_evaluated: v
            .pointer("/res/artifacts_evaluated")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        artifacts_considered: v
            .pointer("/res/artifacts_considered")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize,
        best_policy: v
            .pointer("/res/best_policy")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        stage5_precision_at_k: v
            .pointer("/res/stage5_blended/precision_at_k")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        stage5_vs_meta_precision_delta: v
            .pointer("/res/stage5_vs_meta_precision_delta")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        stage5_vs_ml_precision_delta: v
            .pointer("/res/stage5_vs_ml_precision_delta")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        tags: normalized_tags(v),
    })
}

pub(super) fn list_artifact_library_blocking(
    app: tauri::AppHandle,
) -> Result<ArtifactLibrarySnapshot, String> {
    let entries = scan_artifact_library(&app)?;
    let mut runs = Vec::new();
    let mut meta = Vec::new();
    let mut stage5 = Vec::new();
    let mut policy_evals = Vec::new();
    let mut options_expression = Vec::new();

    for entry in entries {
        if let Some(summary) = build_run_artifact_summary(&entry.path, entry.bytes, &entry.value) {
            runs.push(summary);
        }
        if let Some(summary) = build_meta_artifact_summary(&entry.path, entry.bytes, &entry.value) {
            meta.push(summary);
        }
        if let Some(summary) = build_stage5_artifact_summary(&entry.path, entry.bytes, &entry.value)
        {
            stage5.push(summary);
        }
        if let Some(summary) = build_ml_policy_eval_summary(&entry.path, entry.bytes, &entry.value)
        {
            policy_evals.push(summary);
        }
        if let Some(summary) =
            build_options_expression_artifact_summary(&entry.path, entry.bytes, &entry.value)
        {
            options_expression.push(summary);
        }
    }

    runs.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));
    meta.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));
    stage5.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));
    policy_evals.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));
    options_expression.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));

    Ok(ArtifactLibrarySnapshot {
        runs,
        meta,
        stage5,
        policy_evals,
        options_expression,
    })
}

pub(super) fn load_meta_artifacts_with_paths_blocking(
    app: &tauri::AppHandle,
) -> Result<Vec<(PathBuf, Value)>, String> {
    let mut out = Vec::new();
    for entry in scan_artifact_library(app)? {
        if !is_meta_artifact_file_name(&entry.file_name) {
            continue;
        }
        if !is_meta_selection_kind(
            entry
                .value
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ) {
            continue;
        }
        out.push((entry.path, entry.value));
    }
    out.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    Ok(out)
}

pub(super) fn load_meta_artifacts_blocking(app: &tauri::AppHandle) -> Result<Vec<Value>, String> {
    Ok(load_meta_artifacts_with_paths_blocking(app)?
        .into_iter()
        .map(|(_, value)| value)
        .collect())
}

pub(super) fn load_artifact_value_checked_blocking(
    path: String,
    app: tauri::AppHandle<impl tauri::Runtime>,
) -> Result<Value, String> {
    Ok(load_artifact_entry_checked(path, app)?.value)
}

pub(super) fn load_run_artifact_hydration_blocking<R: tauri::Runtime>(
    path: String,
    app: tauri::AppHandle<R>,
) -> Result<WorkflowArtifactsHydrationPayload, String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_run_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    build_workflow_artifacts_hydration_payload(entry.path.to_string_lossy().as_ref(), &entry.value)
}

pub(super) fn load_latest_run_artifact_summary_blocking<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<RunArtifactSummary>, String> {
    let Some(entry) = load_latest_run_artifact_entry_blocking(&app)? else {
        return Ok(None);
    };
    Ok(build_run_artifact_summary(
        &entry.path,
        entry.bytes,
        &entry.value,
    ))
}

pub(super) fn load_latest_run_artifact_hydration_blocking<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<LatestRunArtifactHydrationResponse>, String> {
    let Some(entry) = load_latest_run_artifact_entry_blocking(&app)? else {
        return Ok(None);
    };
    let summary = build_run_artifact_summary(&entry.path, entry.bytes, &entry.value)
        .ok_or_else(|| "latest run artifact is not a supported run artifact".to_string())?;
    let hydration = build_workflow_artifacts_hydration_payload(
        entry.path.to_string_lossy().as_ref(),
        &entry.value,
    )?;
    Ok(Some(LatestRunArtifactHydrationResponse {
        summary,
        hydration,
    }))
}

pub(super) fn create_live_session_artifact_blocking<R: tauri::Runtime>(
    artifact: Value,
    app: tauri::AppHandle<R>,
) -> Result<SaveRunArtifactResponse, String> {
    let artifact = sanitize_live_session_artifact_seed(&artifact);
    let path = live_session_artifact_path(&artifact, &app)?;
    write_json_pretty(&path, &artifact)?;
    let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len() as usize;
    Ok(SaveRunArtifactResponse {
        path: path.to_string_lossy().to_string(),
        bytes,
    })
}

pub(super) fn append_live_session_event_blocking<R: tauri::Runtime>(
    path: String,
    event: Value,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    with_live_session_artifact_mutation(path, app, move |artifact| {
        let next = sanitize_live_session_event_value(&event);
        let events = artifact
            .as_object_mut()
            .ok_or_else(|| "live session artifact root must be an object".to_string())?
            .entry("events")
            .or_insert_with(|| Value::Array(Vec::new()));
        let items = events
            .as_array_mut()
            .ok_or_else(|| "live session events must be an array".to_string())?;
        items.push(next);
        if items.len() > LIVE_SESSION_EVENT_LIMIT {
            let overflow = items.len() - LIVE_SESSION_EVENT_LIMIT;
            items.drain(0..overflow);
        }
        Ok(())
    })
}

pub(super) fn append_live_session_broker_snapshot_blocking<R: tauri::Runtime>(
    path: String,
    snapshot: Value,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    with_live_session_artifact_mutation(path, app, move |artifact| {
        let next = sanitize_live_session_broker_snapshot_value(&snapshot);
        let snapshots = artifact
            .as_object_mut()
            .ok_or_else(|| "live session artifact root must be an object".to_string())?
            .entry("broker_snapshots")
            .or_insert_with(|| Value::Array(Vec::new()));
        let items = snapshots
            .as_array_mut()
            .ok_or_else(|| "live session broker snapshots must be an array".to_string())?;
        items.push(next);
        if items.len() > LIVE_SESSION_SNAPSHOT_LIMIT {
            let overflow = items.len() - LIVE_SESSION_SNAPSHOT_LIMIT;
            items.drain(0..overflow);
        }
        Ok(())
    })
}

pub(super) fn finalize_live_session_artifact_blocking<R: tauri::Runtime>(
    path: String,
    session_state: String,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    with_live_session_artifact_mutation(path, app, move |artifact| {
        let root = artifact
            .as_object_mut()
            .ok_or_else(|| "live session artifact root must be an object".to_string())?;
        let next_state = session_state.trim();
        root.insert(
            "session_state".to_string(),
            Value::String(if next_state.is_empty() {
                "finalized".to_string()
            } else {
                next_state.to_string()
            }),
        );
        root.insert(
            "finalized_at_unix_ms".to_string(),
            Value::Number(serde_json::Number::from(
                chrono::Utc::now().timestamp_millis(),
            )),
        );
        Ok(())
    })
}

pub(super) fn load_run_artifact_detail_blocking<R: tauri::Runtime>(
    path: String,
    app: tauri::AppHandle<R>,
) -> Result<RunArtifactDetailResponse, String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_run_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    build_run_artifact_detail_response(entry.path.to_string_lossy().as_ref(), &entry.value)
}

pub(super) fn load_live_session_artifact_detail_blocking<R: tauri::Runtime>(
    path: String,
    app: tauri::AppHandle<R>,
) -> Result<LiveSessionArtifactDetailResponse, String> {
    let entry = load_artifact_entry_checked(path, app)?;
    build_live_session_artifact_detail_response(entry.path.to_string_lossy().as_ref(), &entry.value)
}

pub(super) fn load_meta_artifact_replay_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<MetaArtifactReplayResponse, String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_meta_selection_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    let (report_xlsx_path, report_xlsx_exists) =
        normalize_report_xlsx_fields(entry.value.get("report_xlsx_path"));
    Ok(MetaArtifactReplayResponse {
        ts: normalize_text_value(entry.value.get("ts")),
        kind: normalize_text_value(entry.value.get("kind")),
        report_xlsx_path,
        report_xlsx_exists,
        report_xlsx_bytes: 0,
        meta_json_path: entry.path.to_string_lossy().to_string(),
        meta_json_bytes: entry.bytes as usize,
        result: clone_value_or_null(entry.value.pointer("/res")),
    })
}

pub(super) fn load_stage5_artifact_replay_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<Stage5ArtifactReplayResponse, String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_meta_stage_result_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    let path = entry.path.to_string_lossy().to_string();
    Ok(Stage5ArtifactReplayResponse {
        ts: normalize_text_value(entry.value.get("ts")),
        kind: kind.to_string(),
        stage4_json_path: path.clone(),
        stage4_json_bytes: entry.bytes as usize,
        stage5_json_path: path,
        stage5_json_bytes: entry.bytes as usize,
        result: clone_value_or_null(entry.value.pointer("/res")),
    })
}

pub(super) fn load_ml_policy_eval_artifact_replay_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<MlPolicyEvalArtifactReplayResponse, String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_ml_confidence_policy_eval_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    Ok(MlPolicyEvalArtifactReplayResponse {
        ts: normalize_text_value(entry.value.get("ts")),
        kind: kind.to_string(),
        eval_json_path: entry.path.to_string_lossy().to_string(),
        eval_json_bytes: entry.bytes as usize,
        result: clone_value_or_null(entry.value.pointer("/res")),
    })
}

fn open_runs_dir_blocking_with_revealer<R: tauri::Runtime, F>(
    app: tauri::AppHandle<R>,
    reveal: F,
) -> Result<String, String>
where
    F: FnOnce(String) -> Result<(), String>,
{
    let dir = runs_dir(&app)?;
    let dir_string = dir.to_string_lossy().to_string();
    reveal(dir_string.clone())?;
    Ok(dir_string)
}

pub(super) fn open_runs_dir_blocking<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    open_runs_dir_blocking_with_revealer(app, reveal_path_blocking)
}

fn canonical_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(xs) => {
            let mut out = String::from("[");
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(x));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                let v = map.get(*k).unwrap_or(&Value::Null);
                out.push_str(&canonical_json(v));
            }
            out.push('}');
            out
        }
    }
}

fn fnv1a64_hex_utf16(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let prime: u64 = 0x100000001b3;
    for u in s.encode_utf16() {
        h ^= u as u64;
        h = h.wrapping_mul(prime);
    }
    format!("{h:016x}")
}

pub(super) fn compute_config_fingerprint_from_artifact(artifact: &Value) -> String {
    let mut run_req = artifact.pointer("/run/req").cloned().unwrap_or(Value::Null);
    if let Value::Object(map) = &mut run_req {
        map.remove("data_id");
        map.remove("dataId");
    }

    let mut val_req = artifact
        .pointer("/validation/req")
        .cloned()
        .unwrap_or(Value::Null);
    if let Value::Object(map) = &mut val_req {
        map.remove("data_id");
        map.remove("dataId");
        map.remove("candidates");
        map.remove("admission_provenance");

        if let Some(cross) = map.get_mut("cross_ticker").and_then(|v| v.as_object_mut()) {
            if let Some(ds) = cross.get_mut("datasets").and_then(|v| v.as_array_mut()) {
                let mut out: Vec<Value> = Vec::with_capacity(ds.len());
                for d in ds.iter() {
                    let p = d
                        .get("csv_path")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .trim();
                    if p.is_empty() {
                        continue;
                    }
                    out.push(serde_json::json!({ "csv_path": p }));
                }
                out.sort_by(|a, b| {
                    let aa = a.get("csv_path").and_then(|x| x.as_str()).unwrap_or("");
                    let bb = b.get("csv_path").and_then(|x| x.as_str()).unwrap_or("");
                    aa.cmp(bb)
                });
                *ds = out;
            }
        }
    }

    let payload = serde_json::json!({
        "version": 1,
        "run_req": run_req,
        "validation_req": val_req,
    });
    let canon = canonical_json(&payload);
    let hex = fnv1a64_hex_utf16(&canon);
    format!("cfg1-fnv1a64-{hex}")
}

pub(super) fn list_run_artifacts_blocking<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<RunArtifactSummary>, String> {
    let mut runs = Vec::new();
    for entry in scan_artifact_library(&app)? {
        if let Some(summary) = build_run_artifact_summary(&entry.path, entry.bytes, &entry.value) {
            runs.push(summary);
        }
    }
    runs.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));
    Ok(runs)
}

pub(super) fn list_live_session_artifacts_blocking<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<LiveSessionArtifactSummary>, String> {
    let mut sessions = Vec::new();
    for entry in scan_artifact_library(&app)? {
        if !is_live_session_artifact_file_name(&entry.file_name) {
            continue;
        }
        if let Some(summary) =
            build_live_session_artifact_summary(&entry.path, entry.bytes, &entry.value)
        {
            sessions.push(summary);
        }
    }
    sessions.sort_by(|lhs, rhs| rhs.ts.cmp(&lhs.ts));
    Ok(sessions)
}

pub(super) fn list_meta_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<MetaArtifactSummary>, String> {
    Ok(list_artifact_library_blocking(app)?.meta)
}

pub(super) fn list_stage4_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<Stage4ArtifactSummary>, String> {
    Ok(list_artifact_library_blocking(app)?.stage5)
}

pub(super) fn list_stage5_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<Stage5ArtifactSummary>, String> {
    Ok(list_artifact_library_blocking(app)?.stage5)
}

pub(super) fn list_ml_policy_eval_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<MlPolicyEvalArtifactSummary>, String> {
    Ok(list_artifact_library_blocking(app)?.policy_evals)
}

pub(super) fn load_run_artifact_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    load_artifact_value_checked_blocking(path, app)
}

pub(super) fn delete_run_artifact_blocking<R: tauri::Runtime>(
    path: String,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_run_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    std::fs::remove_file(&entry.path).map_err(|e| e.to_string())
}

pub(super) fn delete_artifact_blocking<R: tauri::Runtime>(
    path: String,
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    let entry = load_artifact_entry_checked(path, app)?;
    let kind = entry
        .value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_mutable_library_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }
    std::fs::remove_file(&entry.path).map_err(|e| e.to_string())
}

pub(super) fn update_artifact_tags_blocking(
    path: String,
    tags: Vec<String>,
    app: tauri::AppHandle<impl tauri::Runtime>,
) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    if !is_mutable_library_artifact_kind(kind) {
        return Err(format!("unsupported artifact kind: {kind}"));
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for t in tags {
        let tt = t.trim();
        if tt.is_empty() {
            continue;
        }
        let key = tt.to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(tt.to_string());
    }

    let tags_v = serde_json::to_value(out).map_err(|e| e.to_string())?;
    if let Value::Object(obj) = &mut v {
        obj.insert("tags".to_string(), tags_v);
    }

    let bytes = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::metric_surface::build_optimization_metric_surface_artifact_value;
    use super::{
        append_live_session_broker_snapshot_blocking, append_live_session_event_blocking,
        build_meta_artifact_summary, build_ml_policy_eval_summary,
        build_run_artifact_detail_response, build_run_artifact_summary,
        build_stage5_artifact_summary, build_workflow_artifacts_hydration_payload, canonical_json,
        compute_config_fingerprint_from_artifact, create_live_session_artifact_blocking,
        delete_artifact_blocking, delete_run_artifact_blocking, ensure_path_is_within_dir,
        extract_artifact_lineage_from_artifact, extract_route_provenance_from_artifact,
        finalize_live_session_artifact_blocking, is_meta_artifact_file_name,
        is_meta_stage_artifact_file_name, is_options_expression_artifact_kind,
        is_run_artifact_file_name, is_run_artifact_kind, list_live_session_artifacts_blocking,
        list_run_artifacts_blocking, load_artifact_value_checked_blocking,
        load_latest_run_artifact_hydration_blocking, load_latest_run_artifact_summary_blocking,
        load_live_session_artifact_detail_blocking, load_run_artifact_detail_blocking,
        normalize_dataset_meta, open_runs_dir_blocking_with_revealer,
        prune_old_meta_artifacts_blocking, resolve_run_artifact_csv_path,
        resolve_run_artifact_strategy_kind, resolve_run_artifact_timeframe, runs_dir,
        save_run_artifact_blocking, suggest_meta_artifact_path, suggest_ts_file_path_with_limit,
        update_artifact_tags_blocking,
    };
    use crate::state::{AppState, BackgroundTaskState};
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        let unique = unique_token("run_artifacts_app");
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().product_name = Some(unique.clone());
        context.config_mut().identifier = format!("com.test.{unique}");
        tauri::test::mock_builder()
            .manage(AppState::default())
            .manage(BackgroundTaskState::default())
            .build(context)
            .expect("failed to build tauri mock app")
    }

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("{prefix}_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn minimal_run_artifact(ts: &str) -> Value {
        json!({
            "kind": "vectorgrid_run_artifact",
            "ts": ts,
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m"
            },
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            },
            "validation": {
                "req": { "candidates": [] },
                "res": { "candidates": [] }
            }
        })
    }

    fn unique_token(prefix: &str) -> String {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{prefix}_{unique}")
    }

    #[test]
    fn canonical_json_is_order_invariant_for_objects() {
        let a = json!({
            "b": 2,
            "a": 1,
            "nested": { "z": 9, "m": 3 }
        });
        let b = json!({
            "nested": { "m": 3, "z": 9 },
            "a": 1,
            "b": 2
        });
        assert_eq!(canonical_json(&a), canonical_json(&b));
    }

    #[test]
    fn config_fingerprint_ignores_data_ids_candidates_and_cross_dataset_order() {
        let base = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "data_id": "D1",
                    "dataId": "legacy_d1"
                }
            },
            "validation": {
                "req": {
                    "data_id": "V1",
                    "dataId": "legacy_v1",
                    "candidates": [1, 2, 3],
                    "admission_policy": "scalar_union",
                    "admission_provenance": {
                        "requested_policy": "scalar_union",
                        "applied_policy": "scalar_union",
                        "source_kind": "top_rows",
                        "source_candidate_count": 3,
                        "pre_dedupe_count": 3,
                        "post_dedupe_count": 3,
                        "post_cap_count": 3,
                        "heads": [
                            { "head": "primary", "objective": "Sharpe", "candidate_count": 3 }
                        ]
                    },
                    "cross_ticker": {
                        "datasets": [
                            { "csv_path": "b.csv", "label": "B" },
                            { "csv_path": "a.csv", "label": "A" }
                        ]
                    }
                }
            }
        });

        let variant = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "data_id": "D2",
                    "dataId": "legacy_d2"
                }
            },
            "validation": {
                "req": {
                    "data_id": "V2",
                    "dataId": "legacy_v2",
                    "candidates": [99, 100],
                    "admission_policy": "scalar_union",
                    "admission_provenance": {
                        "requested_policy": "scalar_union",
                        "applied_policy": "scalar_union",
                        "source_kind": "best_params_fallback",
                        "source_candidate_count": 2,
                        "pre_dedupe_count": 2,
                        "post_dedupe_count": 2,
                        "post_cap_count": 2,
                        "heads": [
                            { "head": "primary", "objective": "Sharpe", "candidate_count": 2 }
                        ]
                    },
                    "cross_ticker": {
                        "datasets": [
                            { "csv_path": "a.csv", "label": "A2" },
                            { "csv_path": "b.csv", "label": "B2" }
                        ]
                    }
                }
            }
        });

        let fa = compute_config_fingerprint_from_artifact(&base);
        let fb = compute_config_fingerprint_from_artifact(&variant);
        assert_eq!(fa, fb);
    }

    #[test]
    fn dataset_meta_normalization_backfills_resample_identity() {
        let mut ds = serde_json::Map::new();
        ds.insert("timeframe".to_string(), json!("1h"));
        normalize_dataset_meta(&mut ds, "x.csv");

        assert_eq!(ds.get("csv_path").and_then(|v| v.as_str()), Some("x.csv"));
        assert_eq!(ds.get("schema_version").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            ds.get("base_timeframe").and_then(|v| v.as_str()),
            Some("1h")
        );
        assert_eq!(
            ds.get("derived_timeframe").and_then(|v| v.as_str()),
            Some("1h")
        );
        assert_eq!(
            ds.get("resample_applied").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            ds.get("resample_rule").and_then(|v| v.as_str()),
            Some("session_relative_ohlcv")
        );
        assert_eq!(
            ds.get("resample_anchor").and_then(|v| v.as_str()),
            Some("day_first_timestamp")
        );
    }

    #[test]
    fn run_artifact_resolution_falls_back_to_validation_context_metadata() {
        let artifact = json!({
            "validation": {
                "req": {
                    "validation_context": {
                        "strategy_kind": "Carm",
                        "csv_path": "C:/Rust Projects/ta_desktop_demo/data/AAPL.csv",
                        "timeframe": "1h"
                    }
                }
            }
        });

        assert_eq!(resolve_run_artifact_strategy_kind(&artifact), "Carm");
        assert_eq!(
            resolve_run_artifact_csv_path(&artifact),
            "C:/Rust Projects/ta_desktop_demo/data/AAPL.csv"
        );
        assert_eq!(resolve_run_artifact_timeframe(&artifact), "1h");
    }

    #[test]
    fn run_artifact_resolution_prefers_run_req_and_dataset_meta_when_present() {
        let artifact = json!({
            "run": { "req": { "strategy_kind": "Mkcs" } },
            "data": { "csv_path": "C:/data/from_run.csv" },
            "dataset_meta": { "timeframe": "15m" },
            "validation": {
                "req": {
                    "validation_context": {
                        "strategy_kind": "Carm",
                        "csv_path": "C:/data/from_context.csv",
                        "timeframe": "1h"
                    }
                }
            }
        });

        assert_eq!(resolve_run_artifact_strategy_kind(&artifact), "Mkcs");
        assert_eq!(
            resolve_run_artifact_csv_path(&artifact),
            "C:/data/from_run.csv"
        );
        assert_eq!(resolve_run_artifact_timeframe(&artifact), "15m");
    }

    #[test]
    fn dataset_meta_normalization_marks_resampled_when_base_and_derived_differ() {
        let mut ds = serde_json::Map::new();
        ds.insert("base_timeframe".to_string(), json!("1m"));
        ds.insert("derived_timeframe".to_string(), json!("1h"));
        normalize_dataset_meta(&mut ds, "");

        assert_eq!(ds.get("timeframe").and_then(|v| v.as_str()), Some("1h"));
        assert_eq!(
            ds.get("resample_applied").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn dataset_meta_normalization_keeps_existing_fingerprint_and_resample_identity() {
        let mut ds = serde_json::Map::new();
        ds.insert("csv_path".to_string(), json!("C:/data/AAPL.csv"));
        ds.insert("timeframe".to_string(), json!("15m"));
        ds.insert("base_timeframe".to_string(), json!("1m"));
        ds.insert(
            "source_content_fingerprint".to_string(),
            json!("fnv1a64-full-deadbeefdeadbeef"),
        );
        normalize_dataset_meta(&mut ds, "ignored.csv");

        assert_eq!(ds.get("schema_version").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            ds.get("csv_path").and_then(|v| v.as_str()),
            Some("C:/data/AAPL.csv")
        );
        assert_eq!(
            ds.get("base_timeframe").and_then(|v| v.as_str()),
            Some("1m")
        );
        assert_eq!(
            ds.get("derived_timeframe").and_then(|v| v.as_str()),
            Some("15m")
        );
        assert_eq!(
            ds.get("resample_applied").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            ds.get("resample_rule").and_then(|v| v.as_str()),
            Some("session_relative_ohlcv")
        );
        assert_eq!(
            ds.get("resample_anchor").and_then(|v| v.as_str()),
            Some("day_first_timestamp")
        );
        assert_eq!(
            ds.get("source_content_fingerprint")
                .and_then(|v| v.as_str()),
            Some("fnv1a64-full-deadbeefdeadbeef")
        );
    }

    #[test]
    fn run_artifact_kind_alias_accepts_legacy_and_vectorgrid_names() {
        assert!(is_run_artifact_kind("vectorbt_run_artifact"));
        assert!(is_run_artifact_kind("vectorgrid_run_artifact"));
        assert!(!is_run_artifact_kind("vectorbt_meta_selection"));
    }

    #[test]
    fn run_artifact_file_name_alias_accepts_legacy_and_vectorgrid_prefixes() {
        assert!(is_run_artifact_file_name("vectorbt_run_2026-03-13.json"));
        assert!(is_run_artifact_file_name("vectorgrid_run_2026-03-13.json"));
        assert!(!is_run_artifact_file_name(
            "vectorgrid_meta_2026-03-13.json"
        ));
    }

    #[test]
    fn options_expression_artifact_kind_alias_accepts_vectorgrid_name() {
        assert!(is_options_expression_artifact_kind(
            "vectorgrid_options_expression_v1"
        ));
        assert!(!is_options_expression_artifact_kind(
            "vectorgrid_run_artifact"
        ));
    }

    #[test]
    fn meta_artifact_file_name_alias_accepts_legacy_and_vectorgrid_prefixes() {
        assert!(is_meta_artifact_file_name("vectorbt_meta_2026-03-13.json"));
        assert!(is_meta_artifact_file_name(
            "vectorgrid_meta_2026-03-13.json"
        ));
        assert!(!is_meta_artifact_file_name(
            "vectorgrid_run_2026-03-13.json"
        ));
    }

    #[test]
    fn meta_stage_file_name_alias_accepts_legacy_and_vectorgrid_prefixes() {
        assert!(is_meta_stage_artifact_file_name(
            "vectorbt_stage5_2026-03-13.json"
        ));
        assert!(is_meta_stage_artifact_file_name(
            "vectorgrid_stage5_2026-03-13.json"
        ));
        assert!(is_meta_stage_artifact_file_name(
            "vectorgrid_meta_stage4_2026-03-13.json"
        ));
        assert!(!is_meta_stage_artifact_file_name(
            "vectorgrid_meta_2026-03-13.json"
        ));
    }

    #[test]
    fn workflow_hydration_payload_matches_store_shape() {
        let artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "1h"
            },
            "tags": ["alpha", "beta"],
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            },
            "validation": {
                "req": { "candidates": [] },
                "res": { "candidates": [] }
            }
        });

        let payload = build_workflow_artifacts_hydration_payload(
            "C:/runs/vectorgrid_run_test.json",
            &artifact,
        )
        .expect("hydration payload");
        let encoded = serde_json::to_value(payload).expect("serialize");

        assert_eq!(
            encoded
                .pointer("/currentRun/artifact_path")
                .and_then(|value| value.as_str()),
            Some("C:/runs/vectorgrid_run_test.json")
        );
        assert_eq!(
            encoded
                .pointer("/currentRun/csv_path")
                .and_then(|value| value.as_str()),
            Some("C:/data/AAPL.csv")
        );
        assert_eq!(
            encoded
                .pointer("/currentValidation/req/candidates")
                .and_then(|value| value.as_array())
                .map(|items| items.len()),
            Some(0)
        );
    }

    #[test]
    fn workflow_hydration_payload_threads_metric_surface_metadata() {
        let mut artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": {
                        "pnl": 1.0,
                        "sharpe": 0.5,
                        "volatility": 0.25,
                        "max_dd": 0.2,
                        "trades": 8,
                        "exposure": 0.6,
                        "net_exposure": 0.2,
                        "downside_deviation": 0.1,
                        "ulcer_index": 0.08,
                        "omega": 1.2,
                        "downside_frequency": 0.15,
                        "downside_severity": 0.04,
                        "long_mean_return": 0.03,
                        "short_mean_return": -0.02,
                        "kappa_tau": 0.2,
                        "underwater_fraction": 0.18,
                        "max_underwater_duration_bars": 11
                    },
                    "top": []
                }
            }
        });
        let metric_surface =
            build_optimization_metric_surface_artifact_value(&artifact).expect("metric surface");
        if let Some(root) = artifact.as_object_mut() {
            root.insert("optimization_metric_surface".to_string(), metric_surface);
        }

        let payload = build_workflow_artifacts_hydration_payload(
            "C:/runs/vectorgrid_run_test.json",
            &artifact,
        )
        .expect("hydration payload");
        let encoded = serde_json::to_value(payload).expect("serialize");

        assert_eq!(
            encoded
                .pointer("/currentRun/optimization_metric_surface/surface")
                .and_then(|value| value.as_str()),
            Some("core17")
        );
        assert_eq!(
            encoded
                .pointer("/currentRun/optimization_metric_surface/fields")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                }),
            Some(
                [
                    "pnl",
                    "trades",
                    "max_dd",
                    "mean",
                    "std",
                    "exposure",
                    "net_exposure",
                    "downside_deviation",
                    "ulcer_index",
                    "omega",
                    "downside_frequency",
                    "downside_severity",
                    "long_mean_return",
                    "short_mean_return",
                    "kappa_tau",
                    "underwater_fraction",
                    "max_underwater_duration_bars",
                ]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
            )
        );
    }

    #[test]
    fn route_provenance_extracts_requested_resolved_realized_and_fallback_reason() {
        let artifact = json!({
            "run": {
                "req": {
                    "backend": { "backend": "GpuOnly", "device_id": 0 }
                },
                "res": {
                    "backend_route": "GpuKernel -> CpuOnly",
                    "backend_used": "Cpu",
                    "backend_fallback_reason": "cuda unavailable"
                }
            }
        });

        let provenance =
            extract_route_provenance_from_artifact(&artifact).expect("route provenance");

        assert_eq!(provenance.requested_route, "GpuOnly");
        assert_eq!(provenance.resolved_route, "GpuKernel");
        assert_eq!(provenance.realized_route, "CpuOnly");
        assert!(provenance.route_changed);
        assert_eq!(provenance.fallback_reason, "cuda unavailable");
    }

    #[test]
    fn artifact_lineage_extracts_parent_fields_from_legacy_and_nested_shapes() {
        let artifact = json!({
            "parent_artifact_path": "C:/runs/parent.json",
            "parent_artifact_kind": "vectorgrid_run_artifact",
            "lineage": {
                "lineage_relation": "derived_from"
            }
        });

        let lineage = extract_artifact_lineage_from_artifact(&artifact).expect("lineage");

        assert_eq!(lineage.parent_artifact_path, "C:/runs/parent.json");
        assert_eq!(lineage.parent_artifact_kind, "vectorgrid_run_artifact");
        assert_eq!(lineage.lineage_relation, "derived_from");
    }

    #[test]
    fn workflow_hydration_payload_keeps_metric_surface_absent_for_legacy_artifacts() {
        let artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo"
                },
                "res": {
                    "best_metrics": {
                        "pnl": 1.0,
                        "sharpe": 0.5
                    },
                    "top": []
                }
            }
        });

        let payload = build_workflow_artifacts_hydration_payload(
            "C:/runs/vectorgrid_run_test.json",
            &artifact,
        )
        .expect("hydration payload");
        let encoded = serde_json::to_value(payload).expect("serialize");

        assert!(encoded
            .pointer("/currentRun/optimization_metric_surface")
            .is_none());
    }

    #[test]
    fn workflow_hydration_payload_preserves_report_tags_and_ml_signal_without_validation() {
        let artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/SPY.csv",
                "timeframe": "1d",
                "label": "SPY"
            },
            "ml_signal": {
                "stage": "validation",
                "bundle_id": "bundle-a",
                "score_cache_path": "C:/runs/score_cache.json"
            },
            "tags": [" holdout ", "alpha"],
            "run": {
                "req": { "strategy_kind": "MlSignal" },
                "res": { "matched_rows": 123 }
            }
        });

        let payload = build_workflow_artifacts_hydration_payload(
            "C:/runs/vectorgrid_run_test.json",
            &artifact,
        )
        .expect("hydration payload");
        let encoded = serde_json::to_value(payload).expect("serialize");

        assert_eq!(
            encoded
                .pointer("/currentRun/report_xlsx_path")
                .and_then(|value| value.as_str()),
            Some("C:/runs/report.xlsx")
        );
        assert_eq!(
            encoded
                .pointer("/currentRun/tags")
                .and_then(|value| value.as_array())
                .map(|items| items
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["holdout", "alpha"])
        );
        assert_eq!(
            encoded
                .pointer("/currentRun/ml_signal/bundle_id")
                .and_then(|value| value.as_str()),
            Some("bundle-a")
        );
        assert!(encoded.pointer("/currentValidation").is_none());
    }

    #[test]
    fn run_artifact_surfaces_report_existence_for_summary_detail_and_hydration() {
        let dir = make_temp_dir("run_artifact_report_exists");
        let report_path = dir.join("report.xlsx");
        std::fs::write(&report_path, b"report").expect("write report");
        let report_path_text = report_path.to_string_lossy().to_string();
        let artifact = json!({
            "kind": "vectorgrid_run_artifact",
            "ts": "2026-04-02T12:00:00Z",
            "report_xlsx_path": report_path_text,
            "dataset_meta": {
                "csv_path": "C:/data/SPY.csv",
                "timeframe": "1d",
            },
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            }
        });

        let summary = build_run_artifact_summary(
            Path::new("C:/runs/vectorgrid_run_test.json"),
            256,
            &artifact,
        )
        .expect("summary");
        let detail =
            build_run_artifact_detail_response("C:/runs/vectorgrid_run_test.json", &artifact)
                .expect("detail");
        let hydration = build_workflow_artifacts_hydration_payload(
            "C:/runs/vectorgrid_run_test.json",
            &artifact,
        )
        .expect("hydration");
        let encoded = serde_json::to_value(hydration).expect("serialize");

        assert_eq!(summary.report_xlsx_exists, Some(true));
        assert_eq!(detail.report_xlsx_exists, Some(true));
        assert_eq!(detail.report_xlsx_path, report_path_text);
        assert_eq!(detail.report_path_exists, Some(true));
        assert_eq!(
            encoded
                .pointer("/currentRun/report_xlsx_exists")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn run_artifact_summary_backfills_fingerprint_version_and_dataset_identity_for_legacy_kind() {
        let artifact = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-04-02T12:00:00Z",
            "config_fingerprint": "cfg1-fnv1a64-0011223344556677",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/SPY.csv",
                "timeframe": "15m",
                "base_timeframe": "1m",
                "derived_timeframe": "15m",
                "resample_applied": true,
                "resample_rule": "session_relative_ohlcv",
                "resample_anchor": "day_first_timestamp",
                "source_content_fingerprint": "fnv1a64-full-beadbeadbeadbead"
            },
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            },
            "validation": {
                "res": { "candidates": [] }
            },
            "tags": ["legacy", "resampled"]
        });

        let summary = build_run_artifact_summary(
            std::path::Path::new("C:/runs/vectorbt_run_test.json"),
            4096,
            &artifact,
        )
        .expect("summary");

        assert_eq!(summary.kind, "vectorbt_run_artifact");
        assert_eq!(summary.config_fingerprint_version, Some(1));
        assert_eq!(summary.report_xlsx_path, "C:/runs/report.xlsx");
        assert_eq!(summary.csv_path, "C:/data/SPY.csv");
        assert_eq!(summary.timeframe, "15m");
        assert_eq!(summary.base_timeframe, "1m");
        assert_eq!(summary.derived_timeframe, "15m");
        assert_eq!(summary.resample_applied, Some(true));
        assert_eq!(summary.resample_rule, "session_relative_ohlcv");
        assert_eq!(summary.resample_anchor, "day_first_timestamp");
        assert_eq!(
            summary.source_content_fingerprint.as_deref(),
            Some("fnv1a64-full-beadbeadbeadbead")
        );
        assert!(summary.has_validation);
    }

    #[test]
    fn run_artifact_summary_surfaces_ml_signal_provenance_fields_when_present() {
        let artifact = json!({
            "kind": "vectorgrid_run_artifact",
            "ts": "2026-04-02T12:00:00Z",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "ml_signal": {
                "feature_schema_hash": "fnv1a64-feature-0011",
                "calibration_state": "calibrated_probability",
                "threshold_policy": "desktop_probability_threshold_only",
                "producer_kind": "python_research_harness",
                "score_cache_provenance": {
                    "native_backend_used": "native_cpu",
                    "native_fallback_reason": "EnableGPUEvaluation unavailable"
                },
                "score_contract_version": 3,
                "source_bundle_manifest_hash": "fnv1a64-bundle-aabbccdd",
                "score_space": "Probability",
                "score_scope": "CalibrationSlice"
            },
            "run": {
                "req": { "strategy_kind": "MlSignal" },
                "res": { "matched_rows": 123 }
            }
        });

        let summary = build_run_artifact_summary(
            std::path::Path::new("C:/runs/vectorgrid_run_test.json"),
            4096,
            &artifact,
        )
        .expect("summary");

        assert_eq!(
            summary.ml_feature_schema_hash.as_deref(),
            Some("fnv1a64-feature-0011")
        );
        assert_eq!(
            summary.ml_calibration_state.as_deref(),
            Some("calibrated_probability")
        );
        assert_eq!(
            summary.ml_threshold_policy.as_deref(),
            Some("desktop_probability_threshold_only")
        );
        assert_eq!(
            summary.ml_producer_kind.as_deref(),
            Some("python_research_harness")
        );
        assert_eq!(
            summary.ml_native_backend_used.as_deref(),
            Some("native_cpu")
        );
        assert_eq!(
            summary.ml_native_fallback_reason.as_deref(),
            Some("EnableGPUEvaluation unavailable")
        );
        assert_eq!(summary.ml_score_contract_version, Some(3));
        assert_eq!(
            summary.ml_source_bundle_manifest_hash.as_deref(),
            Some("fnv1a64-bundle-aabbccdd")
        );
        assert_eq!(summary.ml_score_space.as_deref(), Some("Probability"));
        assert_eq!(summary.ml_score_scope.as_deref(), Some("CalibrationSlice"));
        assert_eq!(
            summary.ml_score_contract_summary.as_deref(),
            Some("Probability • CalibrationSlice")
        );
        assert_eq!(
            summary.ml_score_note.as_deref(),
            Some(
                "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence."
            )
        );
    }

    #[test]
    fn run_artifact_detail_response_formats_ml_summaries() {
        let artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/SPY.csv",
                "timeframe": "1d"
            },
            "ml_signal": {
                "stage": "validation",
                "bundle_id": "bundle-a",
                "score_cache_path": "C:/runs/cache.json",
                "score_manifest_path": "C:/runs/cache.manifest.json",
                "feature_schema_path": "C:/runs/schema.json",
                "feature_schema_version": 1,
                "feature_schema_hash": "fnv1a64-feature-0011",
                "export_manifest_path": "C:/runs/export.manifest.json",
                "source_bundle_manifest_path": "C:/runs/bundle.manifest.json",
                "source_bundle_manifest_hash": "fnv1a64-bundle-aabbccdd",
                "calibration_state": "calibrated_probability",
                "threshold_policy": "desktop_probability_threshold_only",
                "producer_kind": "python_research_harness",
                "score_cache_provenance": {
                    "native_backend_requested": "native_cuda",
                    "native_backend_used": "native_cpu",
                    "native_library_path": "C:/catboost/catboostmodel.dll",
                    "native_gpu_device_id": 1,
                    "native_fallback_reason": "EnableGPUEvaluation unavailable"
                },
                "score_contract_version": 3,
                "score_space": "Probability",
                "score_scope": "CalibrationSlice",
                "policy": {
                    "t_long": 0.6,
                    "t_short": 0.55,
                    "delta": 0.1,
                    "max_hold_bars": 10
                },
                "policy_sweep": {
                    "sweep_size": 27,
                    "top_k": 4,
                    "best_label": "best_sweep",
                    "best_robust_score": 1.44
                }
            },
            "tags": ["ml", "research"],
            "run": {
                "req": {
                    "strategy_kind": "MlSignal",
                    "data_id": "data-ml",
                    "input_path": "C:/scores/ml_scores.csv",
                    "output_path": "C:/runs/cache.json",
                    "model_bundle_id": "bundle-a"
                },
                "res": { "matched_rows": 123 }
            },
            "validation": {
                "req": {
                    "candidates": [
                        {
                            "t_long": 0.6,
                            "t_short": 0.55,
                            "delta": 0.1,
                            "max_hold_bars": 10
                        }
                    ]
                },
                "res": {
                    "run_verdict": {
                        "approved_candidate_count": 1,
                        "candidates_survived_hard_gates": 1,
                        "candidates_considered": 27
                    },
                    "candidates": [
                        {
                            "label": "best_sweep",
                            "robust_score": 1.0,
                            "psr": 0.7,
                            "dsr": 0.6
                        }
                    ]
                }
            }
        });

        let detail =
            build_run_artifact_detail_response("C:/runs/vectorgrid_run_test.json", &artifact)
                .expect("detail payload");

        assert_eq!(detail.strategy_kind, "MlSignal");
        assert_eq!(detail.timeframe, "1d");
        assert_eq!(detail.csv_path, "C:/data/SPY.csv");
        assert_eq!(detail.report_xlsx_path, "C:/runs/report.xlsx");
        assert_eq!(detail.report_xlsx_exists, Some(false));
        assert_eq!(detail.report_path, "C:/runs/report.xlsx");
        assert_eq!(detail.ml_signal_stage, "validation");
        assert_eq!(detail.ml_bundle_id, "bundle-a");
        assert_eq!(detail.ml_feature_schema_hash, "fnv1a64-feature-0011");
        assert_eq!(
            detail.ml_export_manifest_path,
            "C:/runs/export.manifest.json"
        );
        assert_eq!(
            detail.ml_source_bundle_manifest_path,
            "C:/runs/bundle.manifest.json"
        );
        assert_eq!(
            detail.ml_source_bundle_manifest_hash,
            "fnv1a64-bundle-aabbccdd"
        );
        assert_eq!(detail.ml_calibration_state, "calibrated_probability");
        assert_eq!(
            detail.ml_threshold_policy,
            "desktop_probability_threshold_only"
        );
        assert_eq!(detail.ml_producer_kind, "python_research_harness");
        assert_eq!(detail.ml_native_backend_requested, "native_cuda");
        assert_eq!(detail.ml_native_backend_used, "native_cpu");
        assert_eq!(
            detail.ml_native_library_path,
            "C:/catboost/catboostmodel.dll"
        );
        assert_eq!(detail.ml_native_gpu_device_id, Some(1));
        assert_eq!(
            detail.ml_native_fallback_reason,
            "EnableGPUEvaluation unavailable"
        );
        assert_eq!(detail.ml_score_contract_version, Some(3));
        assert_eq!(detail.ml_score_space, "Probability");
        assert_eq!(detail.ml_score_scope, "CalibrationSlice");
        assert_eq!(
            detail.ml_score_contract_summary,
            "Probability • CalibrationSlice"
        );
        assert_eq!(
            detail.ml_score_note,
            "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence."
        );
        assert_eq!(
            detail.ml_policy_summary,
            "t_long=0.6 | t_short=0.55 | delta=0.1 | hold=10"
        );
        assert_eq!(
            detail.ml_policy_sweep_summary,
            "size=27 | top_k=4 | best=best_sweep | score=1.44"
        );
        assert_eq!(
            detail.ml_run_verdict_summary,
            "approved=1 | survived=1 | considered=27"
        );
        assert_eq!(
            detail.ml_top_candidates_summary,
            "best_sweep | robust=1 | psr=0.7 | dsr=0.6"
        );
        assert!(detail.params_text.contains("strategy: MlSignal"));
        assert!(detail.params_text.contains("policy t_long: 0.6"));
    }

    #[test]
    fn run_artifact_detail_response_preserves_dataset_identity_and_fingerprint() {
        let artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "config_fingerprint": "cfg1-fnv1a64-0011223344556677",
            "dataset_meta": {
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m",
                "base_timeframe": "1m",
                "derived_timeframe": "15m",
                "resample_applied": true,
                "resample_rule": "session_relative_ohlcv",
                "resample_anchor": "day_first_timestamp",
                "source_size_bytes": 2048,
                "source_mtime_utc_ms": 1700003000000i64,
                "source_content_fingerprint": "fnv1a64-full-deadbeefdeadbeef"
            },
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            }
        });

        let detail =
            build_run_artifact_detail_response("C:/runs/vectorgrid_run_test.json", &artifact)
                .expect("detail payload");

        assert_eq!(detail.csv_path, "C:/data/AAPL.csv");
        assert_eq!(detail.timeframe, "15m");
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("base_timeframe"))
                .and_then(|value| value.as_str()),
            Some("1m")
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("derived_timeframe"))
                .and_then(|value| value.as_str()),
            Some("15m")
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("resample_applied"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("resample_rule"))
                .and_then(|value| value.as_str()),
            Some("session_relative_ohlcv")
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("resample_anchor"))
                .and_then(|value| value.as_str()),
            Some("day_first_timestamp")
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("source_size_bytes"))
                .and_then(|value| value.as_u64()),
            Some(2048)
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("source_mtime_utc_ms"))
                .and_then(|value| value.as_i64()),
            Some(1700003000000)
        );
        assert_eq!(
            detail
                .dataset_meta
                .as_ref()
                .and_then(|value| value.get("source_content_fingerprint"))
                .and_then(|value| value.as_str()),
            Some("fnv1a64-full-deadbeefdeadbeef")
        );

        let encoded = serde_json::to_value(detail).expect("serialize detail");
        assert!(encoded.pointer("/mlFeatureSchemaHash").is_none());
        assert!(encoded.pointer("/mlScoreContractVersion").is_none());
        assert!(encoded.pointer("/mlScoreSpace").is_none());
        assert!(encoded.pointer("/mlScoreScope").is_none());
        assert!(encoded.pointer("/mlScoreContractSummary").is_none());
        assert!(encoded.pointer("/mlScoreNote").is_none());
    }

    #[test]
    fn run_artifact_missing_ts_uses_consistent_filename_fallback_across_summary_detail_and_hydration(
    ) {
        let artifact = json!({
            "kind": "vectorgrid_run_artifact",
            "dataset_meta": {
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m"
            },
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            },
            "validation": {
                "req": { "max_candidates": 16 },
                "res": { "rows": [] }
            }
        });
        let path = Path::new("C:/runs/vectorgrid_run_2026-04-09T12-30-00Z_2.json");

        let summary = build_run_artifact_summary(path, 512, &artifact).expect("summary");
        let detail = build_run_artifact_detail_response(path.to_string_lossy().as_ref(), &artifact)
            .expect("detail");
        let hydration =
            build_workflow_artifacts_hydration_payload(path.to_string_lossy().as_ref(), &artifact)
                .expect("hydration");

        assert_eq!(summary.ts, "2026-04-09T12-30-00Z");
        assert_eq!(detail.ts, "2026-04-09T12-30-00Z");
        assert_eq!(hydration.current_run.ts, "2026-04-09T12-30-00Z");
        assert_eq!(
            hydration
                .current_validation
                .as_ref()
                .map(|value| value.ts.as_str()),
            Some("2026-04-09T12-30-00Z")
        );
    }

    #[test]
    fn run_artifact_summary_backfills_cfg1_fingerprint_version_when_field_is_absent() {
        let artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "kind": "vectorgrid_run_artifact",
            "config_fingerprint": "cfg1-fnv1a64-0011223344556677",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m",
                "base_timeframe": "1m",
                "derived_timeframe": "15m",
                "resample_applied": true
            },
            "run": {
                "req": { "strategy_kind": "DoubleMa" },
                "res": { "top": [] }
            }
        });

        let summary = build_run_artifact_summary(
            Path::new("C:/runs/vectorgrid_run_test.json"),
            512,
            &artifact,
        )
        .expect("summary");

        assert_eq!(summary.kind, "vectorgrid_run_artifact");
        assert_eq!(summary.config_fingerprint, "cfg1-fnv1a64-0011223344556677");
        assert_eq!(summary.config_fingerprint_version, Some(1));
        assert_eq!(summary.csv_path, "C:/data/AAPL.csv");
        assert_eq!(summary.timeframe, "15m");
        assert_eq!(summary.base_timeframe, "1m");
        assert_eq!(summary.derived_timeframe, "15m");
        assert_eq!(summary.resample_applied, Some(true));
        assert_eq!(summary.report_xlsx_path, "C:/runs/report.xlsx");
    }

    #[test]
    fn meta_artifact_summary_marks_missing_report_paths_as_unopenable() {
        let dir = make_temp_dir("meta_artifact_missing_report");
        let report_path = dir.join("missing_report.xlsx");
        let artifact = json!({
            "kind": "vectorgrid_meta_selection",
            "ts": "2026-04-02T12:00:00Z",
            "report_xlsx_path": report_path.to_string_lossy().to_string(),
            "req": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "required_tag": "baseline",
            },
            "res": {
                "groups": [],
                "artifacts_used": 0,
                "holdout": { "datasets": 0 }
            }
        });

        let summary = build_meta_artifact_summary(
            Path::new("C:/runs/vectorgrid_meta_test.json"),
            128,
            &artifact,
        )
        .expect("summary");

        assert_eq!(summary.report_xlsx_exists, Some(false));
    }

    #[test]
    fn run_artifact_detail_response_includes_metric_surface_metadata() {
        let mut artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "dataset_meta": {
                "csv_path": "C:/data/SPY.csv",
                "timeframe": "1d"
            },
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": {
                        "pnl": 1.0,
                        "sharpe": 0.5,
                        "volatility": 0.25,
                        "max_dd": 0.2,
                        "trades": 8,
                        "exposure": 0.6,
                        "net_exposure": 0.2,
                        "downside_deviation": 0.1,
                        "ulcer_index": 0.08,
                        "omega": 1.2,
                        "downside_frequency": 0.15,
                        "downside_severity": 0.04,
                        "long_mean_return": 0.03,
                        "short_mean_return": -0.02,
                        "kappa_tau": 0.2,
                        "underwater_fraction": 0.18,
                        "max_underwater_duration_bars": 11
                    },
                    "top": []
                }
            }
        });
        let metric_surface =
            build_optimization_metric_surface_artifact_value(&artifact).expect("metric surface");
        if let Some(root) = artifact.as_object_mut() {
            root.insert("optimization_metric_surface".to_string(), metric_surface);
        }

        let detail =
            build_run_artifact_detail_response("C:/runs/vectorgrid_run_test.json", &artifact)
                .expect("detail payload");

        assert_eq!(
            detail
                .optimization_metric_surface
                .as_ref()
                .and_then(|value| value.get("surface"))
                .and_then(|value| value.as_str()),
            Some("core17")
        );
    }

    #[test]
    fn run_artifact_detail_response_includes_trade22_metric_surface_metadata() {
        let mut artifact = json!({
            "ts": "2026-04-02T12:00:00Z",
            "dataset_meta": {
                "csv_path": "C:/data/SPY.csv",
                "timeframe": "1d"
            },
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "metric_contract": "trade_22",
                    "include_trade_metrics": true
                },
                "res": {
                    "best_metrics": {
                        "pnl": 1.0,
                        "sharpe": 0.5,
                        "volatility": 0.25,
                        "max_dd": 0.2,
                        "trades": 8,
                        "exposure": 0.6,
                        "net_exposure": 0.2,
                        "downside_deviation": 0.1,
                        "ulcer_index": 0.08,
                        "omega": 1.2,
                        "downside_frequency": 0.15,
                        "downside_severity": 0.04,
                        "long_mean_return": 0.03,
                        "short_mean_return": -0.02,
                        "kappa_tau": 0.2,
                        "underwater_fraction": 0.18,
                        "max_underwater_duration_bars": 11,
                        "closed_trades": 7,
                        "winning_trades": 5,
                        "losing_trades": 2,
                        "gross_profit": 2.1,
                        "gross_loss": 0.9
                    },
                    "top": []
                }
            }
        });
        let metric_surface =
            build_optimization_metric_surface_artifact_value(&artifact).expect("metric surface");
        if let Some(root) = artifact.as_object_mut() {
            root.insert("optimization_metric_surface".to_string(), metric_surface);
        }

        let detail =
            build_run_artifact_detail_response("C:/runs/vectorgrid_run_test.json", &artifact)
                .expect("detail payload");

        assert_eq!(
            detail
                .optimization_metric_surface
                .as_ref()
                .and_then(|value| value.get("surface"))
                .and_then(|value| value.as_str()),
            Some("trade22")
        );
        assert_eq!(
            detail
                .optimization_metric_surface
                .as_ref()
                .and_then(|value| value.get("fields"))
                .and_then(|value| value.as_array())
                .map(|items| items.len()),
            Some(22)
        );
    }

    #[test]
    fn run_artifact_summary_preserves_schema_identity_and_resample_fields() {
        let artifact = json!({
            "kind": "vectorbt_run_artifact",
            "ts": "2026-04-09T12:30:00Z",
            "config_fingerprint": "cfg1-fnv1a64-aabbccdd00112233",
            "report_xlsx_path": "C:/runs/report.xlsx",
            "dataset_meta": {
                "csv_path": "C:/data/AAPL.csv",
                "timeframe": "15m",
                "base_timeframe": "1m",
                "derived_timeframe": "15m",
                "resample_applied": true,
                "resample_rule": "session_relative_ohlcv",
                "resample_anchor": "day_first_timestamp",
                "source_size_bytes": 2048,
                "source_mtime_utc_ms": 1712000000000i64,
                "source_content_fingerprint": "sha256:test"
            },
            "tags": ["alpha", "beta"],
            "run": {
                "req": { "strategy_kind": "CompositeTrend" },
                "res": { "top": [] }
            },
            "validation": {
                "res": { "rows": [] }
            }
        });

        let summary = build_run_artifact_summary(
            Path::new("C:/runs/vectorgrid_run_test.json"),
            512,
            &artifact,
        )
        .expect("summary");

        assert_eq!(summary.kind, "vectorbt_run_artifact");
        assert_eq!(summary.strategy_kind, "CompositeTrend");
        assert_eq!(summary.config_fingerprint, "cfg1-fnv1a64-aabbccdd00112233");
        assert_eq!(summary.config_fingerprint_version, Some(1));
        assert_eq!(summary.report_xlsx_path, "C:/runs/report.xlsx");
        assert_eq!(summary.csv_path, "C:/data/AAPL.csv");
        assert_eq!(summary.timeframe, "15m");
        assert_eq!(summary.base_timeframe, "1m");
        assert_eq!(summary.derived_timeframe, "15m");
        assert_eq!(summary.resample_applied, Some(true));
        assert_eq!(summary.resample_rule, "session_relative_ohlcv");
        assert_eq!(summary.resample_anchor, "day_first_timestamp");
        assert_eq!(summary.source_size_bytes, Some(2048));
        assert_eq!(summary.source_mtime_utc_ms, Some(1712000000000));
        assert_eq!(
            summary.source_content_fingerprint.as_deref(),
            Some("sha256:test")
        );
        assert!(summary.has_validation);
        assert_eq!(summary.tags, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn save_run_artifact_blocking_uses_suffixes_for_same_timestamp_collisions() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let ts = unique_token("same_ts_save");

        let first = save_run_artifact_blocking(minimal_run_artifact(&ts), handle.clone(), None)
            .expect("first save");
        let second = save_run_artifact_blocking(minimal_run_artifact(&ts), handle.clone(), None)
            .expect("second save");

        assert_ne!(first.path, second.path);
        assert!(first.path.ends_with(&format!("vectorgrid_run_{ts}.json")));
        assert!(second
            .path
            .ends_with(&format!("vectorgrid_run_{ts}_2.json")));
        assert!(Path::new(&first.path).is_file());
        assert!(Path::new(&second.path).is_file());
    }

    #[test]
    fn save_run_artifact_blocking_backfills_route_provenance_and_lineage() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let ts = unique_token("route_lineage_backfill");
        let artifact = json!({
            "kind": "vectorgrid_run_artifact",
            "ts": ts,
            "parent_artifact_path": "C:/runs/parent_run.json",
            "parent_artifact_kind": "vectorgrid_run_artifact",
            "lineage_relation": "compare_baseline",
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "backend": "GpuOnly"
                },
                "res": {
                    "backend_route": "GpuKernel -> CpuOnly",
                    "backend_used": "Cpu",
                    "backend_fallback_reason": "cuda unavailable",
                    "top": []
                }
            }
        });

        let saved = save_run_artifact_blocking(artifact, handle, None).expect("save artifact");
        let raw = std::fs::read_to_string(&saved.path).expect("read saved artifact");
        let saved_value: Value = serde_json::from_str(&raw).expect("parse saved artifact");

        assert_eq!(
            saved_value
                .pointer("/route_provenance/requestedRoute")
                .and_then(|value| value.as_str()),
            Some("GpuOnly")
        );
        assert_eq!(
            saved_value
                .pointer("/route_provenance/realizedRoute")
                .and_then(|value| value.as_str()),
            Some("CpuOnly")
        );
        assert_eq!(
            saved_value
                .pointer("/lineage/parentArtifactPath")
                .and_then(|value| value.as_str()),
            Some("C:/runs/parent_run.json")
        );
        assert_eq!(
            saved_value
                .pointer("/lineage/lineageRelation")
                .and_then(|value| value.as_str()),
            Some("compare_baseline")
        );
    }

    #[test]
    fn save_run_artifact_blocking_overwrites_existing_path_when_provided() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let ts = unique_token("overwrite_existing_artifact");

        let saved = save_run_artifact_blocking(minimal_run_artifact(&ts), handle.clone(), None)
            .expect("initial save");

        let mut updated = minimal_run_artifact(&ts);
        if let Some(root) = updated.as_object_mut() {
            root.insert(
                "report_xlsx_path".to_string(),
                Value::String("C:/reports/updated.xlsx".to_string()),
            );
        }

        let overwritten =
            save_run_artifact_blocking(updated, handle.clone(), Some(saved.path.clone()))
                .expect("overwrite existing artifact");

        assert_eq!(overwritten.path, saved.path);
        let dir = runs_dir(&handle).expect("runs dir");
        let files: Vec<_> = std::fs::read_dir(&dir)
            .expect("runs dir entries")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(files.len(), 1);

        let persisted =
            load_artifact_value_checked_blocking(saved.path.clone(), handle).expect("load saved");
        assert_eq!(
            persisted.get("report_xlsx_path").and_then(Value::as_str),
            Some("C:/reports/updated.xlsx")
        );
    }

    #[test]
    fn save_run_artifact_blocking_errors_for_invalid_existing_path_instead_of_falling_back() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let ts = unique_token("overwrite_invalid_existing_artifact");
        let outside_dir = make_temp_dir("run_artifact_invalid_overwrite_target");
        let invalid_target = outside_dir.join("outside_runs_dir.json");

        let err = save_run_artifact_blocking(
            minimal_run_artifact(&ts),
            handle.clone(),
            Some(invalid_target.to_string_lossy().to_string()),
        )
        .expect_err("invalid overwrite target should fail");

        assert!(err.contains("outside app runs dir"));
        let dir = runs_dir(&handle).expect("runs dir");
        let files: Vec<_> = std::fs::read_dir(&dir)
            .expect("runs dir entries")
            .filter_map(Result::ok)
            .collect();
        assert!(
            files.is_empty(),
            "invalid overwrite target should not create a fresh artifact"
        );
    }

    #[test]
    fn save_run_artifact_blocking_overwrites_existing_legacy_run_file_when_kind_is_valid() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let legacy_path = dir.join("legacy_saved_run.json");
        std::fs::write(
            &legacy_path,
            serde_json::to_vec_pretty(&minimal_run_artifact("2026-04-10T12:00:00Z"))
                .expect("artifact json"),
        )
        .expect("write legacy run artifact");

        let mut updated = minimal_run_artifact("2026-04-10T12:00:00Z");
        if let Some(root) = updated.as_object_mut() {
            root.insert(
                "report_xlsx_path".to_string(),
                Value::String("C:/reports/legacy-updated.xlsx".to_string()),
            );
        }

        let overwritten = save_run_artifact_blocking(
            updated,
            handle.clone(),
            Some(legacy_path.to_string_lossy().to_string()),
        )
        .expect("overwrite legacy run artifact");

        assert_eq!(PathBuf::from(&overwritten.path), legacy_path);
        let persisted = load_artifact_value_checked_blocking(overwritten.path, handle)
            .expect("load overwritten legacy artifact");
        assert_eq!(
            persisted.get("report_xlsx_path").and_then(Value::as_str),
            Some("C:/reports/legacy-updated.xlsx")
        );
    }

    #[test]
    fn save_run_artifact_blocking_rejects_non_run_existing_path_inside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let target = dir.join("vectorgrid_meta_existing.json");
        std::fs::write(
            &target,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_selection",
                "ts": "2026-04-10T12:00:00Z"
            }))
            .expect("meta json"),
        )
        .expect("write meta artifact");

        let err = save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T12:00:00Z"),
            handle.clone(),
            Some(target.to_string_lossy().to_string()),
        )
        .expect_err("non-run overwrite target should fail");

        assert!(err.contains("unsupported artifact kind"));
        let persisted =
            load_artifact_value_checked_blocking(target.to_string_lossy().to_string(), handle)
                .expect("load persisted meta artifact");
        assert_eq!(
            persisted.get("kind").and_then(Value::as_str),
            Some("vectorgrid_meta_selection")
        );
    }

    #[test]
    fn save_run_artifact_blocking_rejects_missing_non_run_filename_when_existing_path_is_provided()
    {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let target = dir.join("vectorgrid_meta_missing.json");

        let err = save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T12:00:00Z"),
            handle,
            Some(target.to_string_lossy().to_string()),
        )
        .expect_err("non-run filename should fail");

        assert!(err.contains("must target a saved run filename"));
        assert!(
            !target.exists(),
            "failed overwrite should not create a file"
        );
    }

    #[test]
    fn load_latest_run_artifact_summary_prefers_latest_run_file_without_scanning_meta_history() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T10:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save older run");
        let latest = save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T12:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save latest run");
        let newer_meta =
            suggest_meta_artifact_path("2026-04-10T13:00:00Z", &handle).expect("meta path");
        std::fs::write(
            &newer_meta,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_selection",
                "ts": "2026-04-10T13:00:00Z",
            }))
            .expect("meta json"),
        )
        .expect("write newer meta");

        let summary = load_latest_run_artifact_summary_blocking(handle)
            .expect("latest run summary")
            .expect("summary should exist");

        assert_eq!(summary.ts, "2026-04-10T12:00:00Z");
        assert_eq!(summary.path, latest.path);
    }

    #[test]
    fn load_latest_run_artifact_summary_prefers_newer_timestamp_across_supported_prefixes() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let older_vectorgrid = dir.join("vectorgrid_run_2026-04-10T12-00-00Z.json");
        let newer_vectorbt = dir.join("vectorbt_run_2026-04-10T13-00-00Z.json");

        std::fs::write(
            &older_vectorgrid,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "ts": "2026-04-10T12:00:00Z",
                "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            }))
            .expect("vectorgrid json"),
        )
        .expect("write vectorgrid run");
        std::fs::write(
            &newer_vectorbt,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorbt_run_artifact",
                "ts": "2026-04-10T13:00:00Z",
                "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            }))
            .expect("vectorbt json"),
        )
        .expect("write vectorbt run");

        let summary = load_latest_run_artifact_summary_blocking(handle)
            .expect("latest run summary")
            .expect("summary should exist");

        assert_eq!(summary.ts, "2026-04-10T13:00:00Z");
        assert_eq!(summary.path, newer_vectorbt.to_string_lossy().to_string());
        assert_eq!(summary.kind, "vectorbt_run_artifact");
    }

    #[test]
    fn load_latest_run_artifact_summary_falls_back_to_full_scan_for_nonstandard_run_filenames() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let legacy_path = runs_dir(&handle)
            .expect("runs dir")
            .join("legacy_latest.json");
        std::fs::write(
            &legacy_path,
            serde_json::to_vec_pretty(&minimal_run_artifact("2026-04-10T14:00:00Z"))
                .expect("artifact json"),
        )
        .expect("write legacy-named run artifact");

        let summary = load_latest_run_artifact_summary_blocking(handle)
            .expect("latest run summary")
            .expect("summary should exist");

        assert_eq!(summary.ts, "2026-04-10T14:00:00Z");
        assert_eq!(summary.path, legacy_path.to_string_lossy().to_string());
    }

    #[test]
    fn load_latest_run_artifact_summary_prefers_resolved_ts_over_path_order_for_nonstandard_runs() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let missing_ts_path = dir.join("zzz_legacy_missing_ts.json");
        let explicit_ts_path = dir.join("aaa_legacy_explicit_ts.json");
        std::fs::write(
            &missing_ts_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            }))
            .expect("missing-ts json"),
        )
        .expect("write missing-ts run artifact");
        std::fs::write(
            &explicit_ts_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "ts": "2026-04-10T14:00:00Z",
                "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            }))
            .expect("explicit-ts json"),
        )
        .expect("write explicit-ts run artifact");

        let summary = load_latest_run_artifact_summary_blocking(handle)
            .expect("latest run summary")
            .expect("summary should exist");

        assert_eq!(summary.ts, "2026-04-10T14:00:00Z");
        assert_eq!(summary.path, explicit_ts_path.to_string_lossy().to_string());
    }

    #[test]
    fn load_latest_run_artifact_summary_returns_none_when_no_run_artifacts_exist() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let summary = load_latest_run_artifact_summary_blocking(handle).expect("latest summary");

        assert!(summary.is_none());
    }

    #[test]
    fn load_latest_run_artifact_hydration_prefers_latest_run_file_without_scanning_meta_history() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T10:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save older run");
        let latest = save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T12:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save latest run");
        let newer_meta =
            suggest_meta_artifact_path("2026-04-10T13:00:00Z", &handle).expect("meta path");
        std::fs::write(
            &newer_meta,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_selection",
                "ts": "2026-04-10T13:00:00Z",
            }))
            .expect("meta json"),
        )
        .expect("write newer meta");

        let response = load_latest_run_artifact_hydration_blocking(handle)
            .expect("latest hydration")
            .expect("response should exist");

        assert_eq!(response.summary.ts, "2026-04-10T12:00:00Z");
        assert_eq!(response.summary.path, latest.path);
        assert_eq!(response.hydration.current_run.artifact_path, latest.path);
    }

    #[test]
    fn load_latest_run_artifact_hydration_reuses_summary_fingerprint_normalization() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T12:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save run");

        let response = load_latest_run_artifact_hydration_blocking(handle)
            .expect("latest hydration")
            .expect("response should exist");

        assert_eq!(
            response.hydration.current_run.config_fingerprint,
            response.summary.config_fingerprint
        );
        assert_eq!(
            response.hydration.current_run.config_fingerprint_version,
            response.summary.config_fingerprint_version
        );
        assert!(!response
            .hydration
            .current_run
            .config_fingerprint
            .trim()
            .is_empty());
    }

    #[test]
    fn load_latest_run_artifact_hydration_falls_back_to_full_scan_for_nonstandard_run_filenames() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let legacy_path = runs_dir(&handle)
            .expect("runs dir")
            .join("legacy_latest.json");
        std::fs::write(
            &legacy_path,
            serde_json::to_vec_pretty(&minimal_run_artifact("2026-04-10T14:00:00Z"))
                .expect("artifact json"),
        )
        .expect("write legacy-named run artifact");

        let response = load_latest_run_artifact_hydration_blocking(handle)
            .expect("latest hydration")
            .expect("response should exist");

        assert_eq!(response.summary.ts, "2026-04-10T14:00:00Z");
        assert_eq!(
            response.summary.path,
            legacy_path.to_string_lossy().to_string()
        );
        assert_eq!(
            response.hydration.current_run.artifact_path,
            legacy_path.to_string_lossy().to_string()
        );
    }

    #[test]
    fn load_latest_run_artifact_hydration_prefers_resolved_ts_over_path_order_for_nonstandard_runs()
    {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let missing_ts_path = dir.join("zzz_legacy_missing_ts.json");
        let explicit_ts_path = dir.join("aaa_legacy_explicit_ts.json");
        std::fs::write(
            &missing_ts_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            }))
            .expect("missing-ts json"),
        )
        .expect("write missing-ts run artifact");
        std::fs::write(
            &explicit_ts_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "ts": "2026-04-10T14:00:00Z",
                "run": { "req": { "strategy_kind": "DoubleMa" }, "res": { "top": [] } },
            }))
            .expect("explicit-ts json"),
        )
        .expect("write explicit-ts run artifact");

        let response = load_latest_run_artifact_hydration_blocking(handle)
            .expect("latest hydration")
            .expect("response should exist");

        assert_eq!(response.summary.ts, "2026-04-10T14:00:00Z");
        assert_eq!(
            response.summary.path,
            explicit_ts_path.to_string_lossy().to_string()
        );
        assert_eq!(
            response.hydration.current_run.artifact_path,
            explicit_ts_path.to_string_lossy().to_string()
        );
    }

    #[test]
    fn load_latest_run_artifact_hydration_returns_none_when_no_run_artifacts_exist() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let response =
            load_latest_run_artifact_hydration_blocking(handle).expect("latest hydration");

        assert!(response.is_none());
    }

    #[test]
    fn list_run_artifacts_blocking_returns_only_run_summaries_from_mixed_library() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let older = save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T10:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save older run");
        let newer = save_run_artifact_blocking(
            minimal_run_artifact("2026-04-10T12:00:00Z"),
            handle.clone(),
            None,
        )
        .expect("save newer run");
        let meta_path =
            suggest_meta_artifact_path("2026-04-10T13:00:00Z", &handle).expect("meta path");
        std::fs::write(
            &meta_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_selection",
                "ts": "2026-04-10T13:00:00Z",
            }))
            .expect("meta json"),
        )
        .expect("write meta artifact");

        let runs = list_run_artifacts_blocking(handle).expect("run summaries");

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].path, newer.path);
        assert_eq!(runs[1].path, older.path);
        assert!(runs.iter().all(|entry| entry.kind.contains("run_artifact")));
    }

    #[test]
    fn load_run_artifact_detail_blocking_reports_missing_paths_inside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let missing = runs_dir(&handle)
            .expect("runs dir")
            .join(format!("{}_missing.json", unique_token("run_artifact")));

        let err = load_run_artifact_detail_blocking(missing.to_string_lossy().to_string(), handle)
            .expect_err("missing path should fail");

        assert!(err.contains("artifact path does not exist"));
        assert!(err.contains("missing.json"));
    }

    #[test]
    fn load_run_artifact_detail_blocking_rejects_paths_outside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let outside_dir = make_temp_dir("run_artifact_outside_load");
        let outside_path = outside_dir.join("outside.json");
        std::fs::write(
            &outside_path,
            serde_json::to_vec_pretty(&minimal_run_artifact("2026-04-10T12:00:00Z"))
                .expect("artifact json"),
        )
        .expect("write outside artifact");

        let err =
            load_run_artifact_detail_blocking(outside_path.to_string_lossy().to_string(), handle)
                .expect_err("outside path should fail");

        assert_eq!(err, "path is outside app runs dir");
    }

    #[test]
    fn delete_run_artifact_blocking_rejects_paths_outside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let outside_dir = make_temp_dir("run_artifact_outside_delete");
        let outside_path = outside_dir.join("outside.json");
        std::fs::write(&outside_path, b"{}").expect("write outside artifact");

        let err = delete_run_artifact_blocking(outside_path.to_string_lossy().to_string(), handle)
            .expect_err("outside delete should fail");

        assert_eq!(err, "path is outside app runs dir");
    }

    #[test]
    fn delete_run_artifact_blocking_rejects_non_run_artifacts_inside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let target = dir.join("vectorgrid_meta_delete.json");
        std::fs::write(
            &target,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_selection",
                "ts": "2026-04-10T12:00:00Z"
            }))
            .expect("meta json"),
        )
        .expect("write meta artifact");

        let err = delete_run_artifact_blocking(target.to_string_lossy().to_string(), handle)
            .expect_err("non-run delete should fail");

        assert!(err.contains("unsupported artifact kind"));
        assert!(target.exists(), "non-run artifact should remain in place");
    }

    #[test]
    fn delete_artifact_blocking_accepts_meta_stage5_and_policy_eval_artifacts_inside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");

        for (file_name, artifact) in [
            (
                "vectorgrid_meta_delete.json",
                json!({
                    "kind": "vectorgrid_meta_selection",
                    "ts": "2026-04-10T12:00:00Z"
                }),
            ),
            (
                "vectorgrid_stage5_delete.json",
                json!({
                    "kind": "vectorgrid_meta_stage5_ml_confidence",
                    "ts": "2026-04-10T12:05:00Z"
                }),
            ),
            (
                "vectorgrid_policy_eval_delete.json",
                json!({
                    "kind": "vectorgrid_ml_confidence_policy_evaluation",
                    "ts": "2026-04-10T12:10:00Z"
                }),
            ),
        ] {
            let target = dir.join(file_name);
            std::fs::write(
                &target,
                serde_json::to_vec_pretty(&artifact).expect("artifact json"),
            )
            .expect("write artifact");

            delete_artifact_blocking(target.to_string_lossy().to_string(), handle.clone())
                .expect("aggregate artifact delete should succeed");
            assert!(
                !target.exists(),
                "artifact should be deleted: {}",
                target.display()
            );
        }
    }

    #[test]
    fn delete_artifact_blocking_rejects_live_session_artifacts_inside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let dir = runs_dir(&handle).expect("runs dir");
        let target = dir.join("vectorgrid_live_session_delete.json");
        std::fs::write(
            &target,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_live_session_v1"
            }))
            .expect("session json"),
        )
        .expect("write live session artifact");

        let err = delete_artifact_blocking(target.to_string_lossy().to_string(), handle)
            .expect_err("live session delete should fail");

        assert!(err.contains("unsupported artifact kind"));
        assert!(
            target.exists(),
            "unsupported artifact should remain in place"
        );
    }

    #[test]
    fn update_artifact_tags_blocking_rejects_paths_outside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let outside_dir = make_temp_dir("run_artifact_outside_tags");
        let outside_path = outside_dir.join("outside.json");
        std::fs::write(
            &outside_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_run_artifact",
                "ts": "2026-04-10T12:00:00Z",
                "tags": ["old"]
            }))
            .expect("artifact json"),
        )
        .expect("write outside artifact");

        let err = update_artifact_tags_blocking(
            outside_path.to_string_lossy().to_string(),
            vec!["new".to_string()],
            handle,
        )
        .expect_err("outside tag update should fail");

        assert_eq!(err, "path is outside app runs dir");
    }

    #[test]
    fn open_runs_dir_blocking_with_revealer_creates_runs_dir_and_returns_it() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let mut revealed = None;

        let opened = open_runs_dir_blocking_with_revealer(handle.clone(), |path| {
            revealed = Some(path);
            Ok(())
        })
        .expect("open runs dir");

        assert_eq!(revealed, Some(opened.clone()));
        assert!(Path::new(&opened).is_dir());
        assert_eq!(
            Path::new(&opened),
            runs_dir(&handle).expect("runs dir").as_path()
        );
    }

    #[test]
    fn update_artifact_tags_trims_and_dedupes_case_insensitively() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let path =
            suggest_meta_artifact_path("2026-04-09T12-00-00Z", &handle).expect("artifact path");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_selection",
                "ts": "2026-04-09T12:00:00Z",
                "tags": ["old"]
            }))
            .expect("json"),
        )
        .expect("write artifact");

        update_artifact_tags_blocking(
            path.to_string_lossy().to_string(),
            vec![
                " Holdout ".to_string(),
                "holdout".to_string(),
                "".to_string(),
                "Review".to_string(),
                "review".to_string(),
            ],
            handle,
        )
        .expect("update tags");

        let updated: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read"))
            .expect("artifact json");
        assert_eq!(updated.get("tags"), Some(&json!(["Holdout", "Review"])));
    }

    #[test]
    fn build_stage5_artifact_summary_preserves_stage4_stage5_alias_contract() {
        let path = Path::new("C:/runs/vectorgrid_stage5_2026-04-09T12-30-00Z.json");
        let artifact = json!({
            "kind": "vectorgrid_meta_stage5_ml_confidence",
            "ts": "2026-04-09T12:30:00Z",
            "req": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "source_meta_path": "C:/runs/vectorgrid_meta_source.json",
                "top_k": 10,
                "min_ml_confidence": 0.55
            },
            "res": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "source_meta_json_path": "C:/runs/vectorgrid_meta_source.json",
                "groups_selected": 2,
                "groups_scored": 4,
                "groups_considered": 6,
                "top_k": 10,
                "min_ml_confidence": 0.55,
                "model_healthy": true
            },
            "tags": ["stage5", "final"]
        });

        let summary = build_stage5_artifact_summary(path, 256, &artifact).expect("summary");
        assert_eq!(summary.path, path.to_string_lossy().to_string());
        assert_eq!(summary.strategy_kind, "CompositeMo");
        assert_eq!(summary.timeframe, "1m");
        assert_eq!(
            summary.source_meta_json_path,
            "C:/runs/vectorgrid_meta_source.json"
        );
        assert_eq!(summary.groups_selected, 2);
        assert_eq!(summary.groups_scored, 4);
        assert_eq!(summary.groups_considered, 6);
        assert_eq!(summary.top_k, 10);
        assert!((summary.min_ml_confidence - 0.55).abs() < f64::EPSILON);
        assert!(summary.model_healthy);
        assert_eq!(
            summary.tags,
            vec!["stage5".to_string(), "final".to_string()]
        );
    }

    #[test]
    fn build_ml_policy_eval_summary_preserves_saved_eval_contract_fields() {
        let path = Path::new("C:/runs/vectorgrid_ml_confidence_eval_2026-04-09T12-45-00Z.json");
        let artifact = json!({
            "kind": "vectorgrid_ml_confidence_policy_evaluation",
            "ts": "2026-04-09T12:45:00Z",
            "res": {
                "strategy_kind": "CompositeMo",
                "timeframe": "1m",
                "artifacts_evaluated": 3,
                "artifacts_considered": 4,
                "best_policy": "stage5_blended",
                "stage5_blended": {
                    "precision_at_k": 1.0
                },
                "stage5_vs_meta_precision_delta": 0.10,
                "stage5_vs_ml_precision_delta": 0.20
            },
            "tags": ["policy", "offline"]
        });

        let summary = build_ml_policy_eval_summary(path, 320, &artifact).expect("summary");
        assert_eq!(summary.path, path.to_string_lossy().to_string());
        assert_eq!(summary.strategy_kind, "CompositeMo");
        assert_eq!(summary.timeframe, "1m");
        assert_eq!(summary.artifacts_evaluated, 3);
        assert_eq!(summary.artifacts_considered, 4);
        assert_eq!(summary.best_policy, "stage5_blended");
        assert!((summary.stage5_precision_at_k - 1.0).abs() < f64::EPSILON);
        assert!((summary.stage5_vs_meta_precision_delta - 0.10).abs() < f64::EPSILON);
        assert!((summary.stage5_vs_ml_precision_delta - 0.20).abs() < f64::EPSILON);
        assert_eq!(
            summary.tags,
            vec!["policy".to_string(), "offline".to_string()]
        );
    }

    #[test]
    fn prune_old_meta_artifacts_preserves_tagged_older_entries() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let older_kept =
            suggest_meta_artifact_path("2026-04-09T10-00-00Z", &handle).expect("older");
        let older_pruned =
            suggest_meta_artifact_path("2026-04-09T11-00-00Z", &handle).expect("middle");
        let newest = suggest_meta_artifact_path("2026-04-09T12-00-00Z", &handle).expect("newest");

        for (path, tags) in [
            (&older_kept, json!(["holdout"])),
            (&older_pruned, json!([])),
            (&newest, json!([])),
        ] {
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&json!({
                    "kind": "vectorgrid_meta_selection",
                    "ts": path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default(),
                    "tags": tags,
                }))
                .expect("json"),
            )
            .expect("write artifact");
        }

        prune_old_meta_artifacts_blocking(&handle, 1);

        assert!(older_kept.exists(), "tagged artifact should be preserved");
        assert!(
            !older_pruned.exists(),
            "untagged older artifact should be pruned"
        );
        assert!(newest.exists(), "newest artifact should be retained");
    }

    #[test]
    fn suggest_ts_file_path_errors_when_suffix_slots_are_exhausted() {
        let dir = make_temp_dir("suggest_ts_file_path_exhausted");
        std::fs::write(dir.join("vectorgrid_run_2026-04-10.json"), b"taken").expect("seed base");
        std::fs::write(dir.join("vectorgrid_run_2026-04-10_2.json"), b"taken")
            .expect("seed slot 2");
        std::fs::write(dir.join("vectorgrid_run_2026-04-10_3.json"), b"taken")
            .expect("seed slot 3");

        let err = suggest_ts_file_path_with_limit(dir, "vectorgrid_run", "2026-04-10", "json", 3)
            .expect_err("slots should exhaust");
        assert!(err.contains("no available artifact filename slots"));
        assert!(err.contains("vectorgrid_run_2026-04-10.json"));
    }

    #[test]
    fn ensure_path_is_within_dir_rejects_missing_paths_with_explicit_message() {
        let dir = make_temp_dir("ensure_path_within_dir");
        let missing = dir.join("missing.json");

        let err = ensure_path_is_within_dir(&missing, &dir).expect_err("missing path");
        assert!(err.contains("artifact path does not exist"));
        assert!(err.contains("missing.json"));
    }

    #[test]
    fn live_session_artifact_round_trip_redacts_and_lists_review_data() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let saved = create_live_session_artifact_blocking(
            json!({
                "ts": "2026-04-12T15:30:00Z",
                "session_id": "live-session-aapl-1",
                "session_state": "active",
                "mode": "paper",
                "symbol": "AAPL",
                "qty": 2.0,
                "dry_run": false,
                "strategy_kind": "DoubleMa",
                "data_id": "data_aapl",
                "param_source": "selected",
                "source_run_artifact_path": "C:/runs/vectorgrid_run_2026-04-11T15-00-00Z.json",
                "source_run_config_fingerprint": "cfg1-fnv1a64-abc123",
                "source_run_route_provenance": {
                    "requested_route": "GpuOnly",
                    "resolved_route": "GpuKernel",
                    "realized_route": "CpuOnly",
                    "route_changed": true,
                    "fallback_reason": "cuda unavailable",
                    "ignored_nested": { "heads": ["x"] }
                },
                "alpaca_api_key": "should_not_persist",
                "alpaca_api_secret": "should_not_persist",
                "persistedApiKey": "should_not_persist",
                "persistedApiSecret": "should_not_persist",
                "outputPreview": "{\"secrets\":true}",
                "account_number": "ABC123",
                "events": [
                    {
                        "provenance": "local_control",
                        "status": "start_runner",
                        "message": "runner armed",
                        "symbol": "AAPL",
                        "at_unix_ms": 1_900_000_000_000_i64,
                        "alpaca_api_key": "nope",
                        "outputPreview": "hide me"
                    }
                ],
                "broker_snapshots": [
                    {
                        "source_command": "get_live_execution_snapshot",
                        "mode": "paper",
                        "symbol": "AAPL",
                        "at_unix_ms": 1_900_000_000_500_i64,
                        "account_status": "ACTIVE",
                        "currency": "USD",
                        "position_state": "flat",
                        "position_qty": 0.0,
                        "position_side": "",
                        "open_order_count": 0,
                        "order_count": 1,
                        "account_number": "ABC123",
                        "cash": "25000"
                    }
                ]
            }),
            handle.clone(),
        )
        .expect("create live session artifact");

        append_live_session_event_blocking(
            saved.path.clone(),
            json!({
                "provenance": "submission_attempt",
                "status": "order_submitted",
                "message": "submitted buy order",
                "symbol": "AAPL",
                "at_unix_ms": 1_900_000_001_000_i64,
                "action": "buy",
                "order_id": "ord_123",
                "delta_qty": 2.0,
                "order_submitted": true,
                "warnings": ["paper only"],
                "apiSecret": "should_not_persist",
                "raw_payload": { "secret": true }
            }),
            handle.clone(),
        )
        .expect("append event");
        append_live_session_broker_snapshot_blocking(
            saved.path.clone(),
            json!({
                "source_command": "test_alpaca_connection",
                "mode": "paper",
                "symbol": "AAPL",
                "at_unix_ms": 1_900_000_001_500_i64,
                "account_status": "ACTIVE",
                "currency": "USD",
                "position_state": "flat",
                "position_qty": 0.0,
                "position_side": "",
                "open_order_count": 0,
                "order_count": 0,
                "buying_power": "100000",
                "account_id": "acct_123"
            }),
            handle.clone(),
        )
        .expect("append snapshot");
        finalize_live_session_artifact_blocking(
            saved.path.clone(),
            "completed".to_string(),
            handle.clone(),
        )
        .expect("finalize session");

        let stored =
            load_artifact_value_checked_blocking(saved.path.clone(), handle.clone()).expect("load");
        assert_eq!(
            stored.get("kind").and_then(Value::as_str),
            Some("vectorgrid_live_session_v1")
        );
        assert!(stored.get("alpaca_api_key").is_none());
        assert!(stored.get("alpaca_api_secret").is_none());
        assert!(stored.get("persistedApiKey").is_none());
        assert!(stored.get("persistedApiSecret").is_none());
        assert!(stored.get("outputPreview").is_none());
        assert!(stored.get("account_number").is_none());
        assert!(stored.pointer("/events/0/alpaca_api_key").is_none());
        assert!(stored.pointer("/events/1/apiSecret").is_none());
        assert!(stored
            .pointer("/broker_snapshots/0/account_number")
            .is_none());
        assert!(stored.pointer("/broker_snapshots/1/account_id").is_none());
        let detail = load_live_session_artifact_detail_blocking(saved.path.clone(), handle.clone())
            .expect("detail");
        assert_eq!(detail.session_id, "live-session-aapl-1");
        assert_eq!(detail.session_state, "completed");
        assert_eq!(detail.symbol, "AAPL");
        assert_eq!(detail.event_count, 2);
        assert_eq!(detail.broker_snapshot_count, 2);
        assert_eq!(detail.events[1].status, "order_submitted");
        assert_eq!(detail.events[1].action, "buy");
        assert_eq!(detail.events[1].order_id, "ord_123");
        assert_eq!(detail.events[1].warnings, vec!["paper only".to_string()]);
        assert_eq!(
            detail.broker_snapshots[0].source_command,
            "get_live_execution_snapshot"
        );
        assert_eq!(
            detail.broker_snapshots[1].source_command,
            "test_alpaca_connection"
        );
        assert_eq!(
            detail
                .source_run_route_provenance
                .as_ref()
                .map(|value| value.requested_route.as_str()),
            Some("GpuOnly")
        );
        assert_eq!(
            detail
                .source_run_route_provenance
                .as_ref()
                .map(|value| value.resolved_route.as_str()),
            Some("GpuKernel")
        );
        assert_eq!(
            detail
                .source_run_route_provenance
                .as_ref()
                .map(|value| value.realized_route.as_str()),
            Some("CpuOnly")
        );
        assert_eq!(
            detail
                .source_run_route_provenance
                .as_ref()
                .map(|value| value.fallback_reason.as_str()),
            Some("cuda unavailable")
        );
        assert_eq!(
            detail
                .source_run_route_provenance
                .as_ref()
                .map(|value| value.route_changed),
            Some(true)
        );

        let sessions = list_live_session_artifacts_blocking(handle).expect("list sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].path, saved.path);
        assert_eq!(sessions[0].session_state, "completed");
        assert_eq!(sessions[0].event_count, 2);
        assert_eq!(sessions[0].broker_snapshot_count, 2);
    }

    #[test]
    fn live_session_event_retention_keeps_latest_entries_only() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let saved = create_live_session_artifact_blocking(
            json!({
                "ts": "2026-04-12T15:31:00Z",
                "session_id": "live-session-retention-events",
                "mode": "paper",
                "symbol": "AAPL",
                "qty": 1.0,
                "dry_run": true,
                "strategy_kind": "DoubleMa",
                "data_id": "data_aapl"
            }),
            handle.clone(),
        )
        .expect("create session");

        for index in 0..(super::LIVE_SESSION_EVENT_LIMIT + 5) {
            append_live_session_event_blocking(
                saved.path.clone(),
                json!({
                    "provenance": "runner_lifecycle",
                    "status": format!("status_{index}"),
                    "message": format!("message_{index}"),
                    "symbol": "AAPL",
                    "at_unix_ms": index as i64
                }),
                handle.clone(),
            )
            .expect("append retained event");
        }

        let detail =
            load_live_session_artifact_detail_blocking(saved.path, handle).expect("detail");
        assert_eq!(detail.events.len(), super::LIVE_SESSION_EVENT_LIMIT);
        assert_eq!(detail.event_count, super::LIVE_SESSION_EVENT_LIMIT);
        assert_eq!(detail.events[0].status, "status_5");
        assert_eq!(
            detail.events.last().map(|event| event.status.as_str()),
            Some("status_68")
        );
    }

    #[test]
    fn live_session_snapshot_retention_keeps_latest_entries_only() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let saved = create_live_session_artifact_blocking(
            json!({
                "ts": "2026-04-12T15:32:00Z",
                "session_id": "live-session-retention-snapshots",
                "mode": "paper",
                "symbol": "AAPL",
                "qty": 1.0,
                "dry_run": true,
                "strategy_kind": "DoubleMa",
                "data_id": "data_aapl"
            }),
            handle.clone(),
        )
        .expect("create session");

        for index in 0..(super::LIVE_SESSION_SNAPSHOT_LIMIT + 3) {
            append_live_session_broker_snapshot_blocking(
                saved.path.clone(),
                json!({
                    "source_command": format!("snapshot_{index}"),
                    "mode": "paper",
                    "symbol": "AAPL",
                    "at_unix_ms": index as i64,
                    "account_status": "ACTIVE",
                    "currency": "USD",
                    "position_state": "flat",
                    "position_qty": 0.0,
                    "position_side": "",
                    "open_order_count": index,
                    "order_count": index + 1
                }),
                handle.clone(),
            )
            .expect("append retained snapshot");
        }

        let detail =
            load_live_session_artifact_detail_blocking(saved.path, handle).expect("detail");
        assert_eq!(
            detail.broker_snapshots.len(),
            super::LIVE_SESSION_SNAPSHOT_LIMIT
        );
        assert_eq!(
            detail.broker_snapshot_count,
            super::LIVE_SESSION_SNAPSHOT_LIMIT
        );
        assert_eq!(detail.broker_snapshots[0].source_command, "snapshot_3");
        assert_eq!(
            detail
                .broker_snapshots
                .last()
                .map(|snapshot| snapshot.source_command.as_str()),
            Some("snapshot_18")
        );
    }

    #[test]
    fn load_live_session_artifact_detail_rejects_paths_outside_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let outside_dir = make_temp_dir("live_session_outside_load");
        let outside_path = outside_dir.join("outside.json");
        std::fs::write(
            &outside_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_live_session_v1",
                "ts": "2026-04-12T12:00:00Z"
            }))
            .expect("artifact json"),
        )
        .expect("write outside artifact");

        let err = load_live_session_artifact_detail_blocking(
            outside_path.to_string_lossy().to_string(),
            handle,
        )
        .expect_err("outside path should fail");

        assert_eq!(err, "path is outside app runs dir");
    }
}
