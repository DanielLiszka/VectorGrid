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
    metric_surface::build_optimization_metric_surface_artifact_value, reveal_path_blocking,
    sanitize_filename_component,
};

const DATASET_META_SCHEMA_VERSION: u64 = 2;
const CONFIG_FINGERPRINT_SCHEMA_VERSION: u64 = 1;
const RESAMPLE_RULE_SESSION_RELATIVE_OHLCV: &str = "session_relative_ohlcv";
const RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP: &str = "day_first_timestamp";

fn is_run_artifact_kind(kind: &str) -> bool {
    matches!(
        kind.trim(),
        "vectorbt_run_artifact" | "vectorgrid_run_artifact"
    )
}

fn is_options_expression_artifact_kind(kind: &str) -> bool {
    matches!(kind.trim(), "vectorgrid_options_expression_v1")
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

fn is_meta_artifact_file_name(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.starts_with("vectorbt_meta_") || trimmed.starts_with("vectorgrid_meta_")
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
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub strategy_kind: String,
    pub timeframe: String,
    pub required_tag: String,
    pub groups: usize,
    pub artifacts_used: usize,
    pub holdout_datasets: usize,
    pub report_xlsx_path: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage4ArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
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
    pub csv_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_meta: Option<Value>,
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
pub struct RunArtifactDetailResponse {
    pub ts: String,
    pub artifact_path: String,
    pub strategy_kind: String,
    pub timeframe: String,
    pub csv_path: String,
    pub report_path: String,
    pub has_validation: bool,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_meta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_signal: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimization_metric_surface: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_req: Option<Value>,
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
    pub ml_policy_summary: String,
    pub ml_policy_sweep_summary: String,
    pub ml_run_verdict_summary: String,
    pub ml_top_candidates_summary: String,
    pub params_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaArtifactReplayResponse {
    pub ts: String,
    pub report_xlsx_path: String,
    pub report_xlsx_bytes: usize,
    pub meta_json_path: String,
    pub meta_json_bytes: usize,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage5ArtifactReplayResponse {
    pub ts: String,
    pub stage4_json_path: String,
    pub stage4_json_bytes: usize,
    pub stage5_json_path: String,
    pub stage5_json_bytes: usize,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MlPolicyEvalArtifactReplayResponse {
    pub ts: String,
    pub eval_json_path: String,
    pub eval_json_bytes: usize,
    pub result: Value,
}

pub(super) fn runs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(super) fn suggest_meta_artifact_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_meta", ts, "json")
}

pub(super) fn suggest_stage5_artifact_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_stage5", ts, "json")
}

pub(super) fn suggest_ml_policy_eval_artifact_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_ml_confidence_eval", ts, "json")
}

pub(super) fn suggest_batch_summary_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_batch", ts, "json")
}

pub(super) fn suggest_campaign_summary_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorgrid_campaign_summary", ts, "json")
}

fn suggest_ts_file_path(
    dir: PathBuf,
    prefix: &str,
    ts: &str,
    ext: &str,
) -> Result<PathBuf, String> {
    let safe_ts = sanitize_filename_component(ts);
    let base_name = format!("{prefix}_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.{ext}"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.{ext}"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn run_artifact_path(artifact: &Value, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = runs_dir(app)?;

    let ts = artifact
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let safe_ts = sanitize_filename_component(ts);
    Ok(dir.join(format!("vectorgrid_run_{safe_ts}.json")))
}

fn ensure_path_is_within_dir(path: &Path, dir: &Path) -> Result<(), String> {
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

pub(super) fn save_run_artifact_blocking(
    mut artifact: Value,
    app: tauri::AppHandle,
) -> Result<SaveRunArtifactResponse, String> {
    let path = run_artifact_path(&artifact, &app)?;

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
    Ok(SaveBatchSummaryResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn prune_old_artifacts_by_prefix(app: &tauri::AppHandle, keep_newest: usize, prefix: &str) {
    let Ok(dir) = runs_dir(app) else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
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
        if !name.starts_with(prefix) {
            continue;
        }
        paths.push(path);
    }

    // Newest first (ISO timestamps sort lexicographically).
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

fn prune_old_run_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_run_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_run_");
}

pub(super) fn prune_old_meta_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_meta_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_meta_");
}

pub(super) fn prune_old_stage4_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_stage4_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_meta_stage4_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_stage5_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorgrid_meta_stage5_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_stage4_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_meta_stage4_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_stage5_");
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_meta_stage5_");
}

pub(super) fn prune_old_ml_policy_eval_artifacts_blocking(
    app: &tauri::AppHandle,
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

fn read_json_value(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn scan_artifact_library(app: &tauri::AppHandle) -> Result<Vec<ArtifactScanEntry>, String> {
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
        let bytes = match std::fs::metadata(&path) {
            Ok(meta) => meta.len(),
            Err(_) => continue,
        };
        let Some(value) = read_json_value(&path) else {
            continue;
        };
        out.push(ArtifactScanEntry {
            path,
            file_name,
            bytes,
            value,
        });
    }

    Ok(out)
}

fn load_artifact_entry_checked(
    path: String,
    app: tauri::AppHandle,
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
    let ts = normalize_text_value(artifact.get("ts"));
    let current_run = WorkflowArtifactRunPayload {
        ts: if ts.is_empty() {
            chrono::Utc::now().to_rfc3339()
        } else {
            ts.clone()
        },
        artifact_path: artifact_path.to_string(),
        report_xlsx_path: normalize_text_value(artifact.get("report_xlsx_path")),
        csv_path: resolve_run_artifact_csv_path(artifact),
        dataset_meta: artifact.pointer("/dataset_meta").cloned(),
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
                ts: current_run.ts.clone(),
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
    let ts = normalize_text_value(artifact.get("ts"));
    let dataset_meta = artifact.pointer("/dataset_meta").cloned();
    let ml_signal = artifact.pointer("/ml_signal").cloned();
    let optimization_metric_surface = artifact.pointer("/optimization_metric_surface").cloned();
    let run_req = artifact.pointer("/run/req").cloned();
    let validation_req = artifact.pointer("/validation/req").cloned();
    let validation_res = artifact.pointer("/validation/res").cloned();

    Ok(RunArtifactDetailResponse {
        ts: if ts.is_empty() {
            chrono::Utc::now().to_rfc3339()
        } else {
            ts
        },
        artifact_path: artifact_path.to_string(),
        strategy_kind: resolve_run_artifact_strategy_kind(artifact),
        timeframe: resolve_run_artifact_timeframe(artifact),
        csv_path: resolve_run_artifact_csv_path(artifact),
        report_path: normalize_text_value(artifact.get("report_xlsx_path")),
        has_validation: validation_res.is_some(),
        tags: normalized_tags(artifact),
        dataset_meta,
        ml_signal: ml_signal.clone(),
        optimization_metric_surface,
        run_req: run_req.clone(),
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
        ml_policy_summary: format_ml_policy_summary(artifact.pointer("/ml_signal/policy")),
        ml_policy_sweep_summary: format_ml_policy_sweep_summary(
            artifact.pointer("/ml_signal/policy_sweep"),
        ),
        ml_run_verdict_summary: format_ml_run_verdict_summary(validation_res.as_ref()),
        ml_top_candidates_summary: format_ml_top_candidates_summary(validation_res.as_ref()),
        params_text: build_req_snapshot(run_req.as_ref(), validation_req.as_ref()),
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

    let ts = v
        .get("ts")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let strategy_kind = resolve_run_artifact_strategy_kind(v);
    let config_fingerprint = v
        .get("config_fingerprint")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| compute_config_fingerprint_from_artifact(v));
    let config_fingerprint_version = v
        .get("config_fingerprint_version")
        .and_then(|x| x.as_u64())
        .map(|x| x as u32)
        .or_else(|| {
            if config_fingerprint.starts_with("cfg1-") {
                Some(1)
            } else {
                None
            }
        });
    let has_validation = v.pointer("/validation/res").is_some();
    let report_xlsx_path = v
        .get("report_xlsx_path")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
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
        tags: normalized_tags(v),
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
    Some(summarize_options_expression_artifact(path, bytes, &artifact))
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

    Some(MetaArtifactSummary {
        ts: v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        bytes,
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
        report_xlsx_path: v
            .get("report_xlsx_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
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
    app: tauri::AppHandle,
) -> Result<Value, String> {
    Ok(load_artifact_entry_checked(path, app)?.value)
}

pub(super) fn load_run_artifact_hydration_blocking(
    path: String,
    app: tauri::AppHandle,
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

pub(super) fn load_run_artifact_detail_blocking(
    path: String,
    app: tauri::AppHandle,
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
    Ok(MetaArtifactReplayResponse {
        ts: normalize_text_value(entry.value.get("ts")),
        report_xlsx_path: normalize_text_value(entry.value.get("report_xlsx_path")),
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
        eval_json_path: entry.path.to_string_lossy().to_string(),
        eval_json_bytes: entry.bytes as usize,
        result: clone_value_or_null(entry.value.pointer("/res")),
    })
}

pub(super) fn open_runs_dir_blocking(app: tauri::AppHandle) -> Result<String, String> {
    let dir = runs_dir(&app)?;
    reveal_path_blocking(dir.to_string_lossy().to_string())?;
    Ok(dir.to_string_lossy().to_string())
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

pub(super) fn list_run_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<RunArtifactSummary>, String> {
    Ok(list_artifact_library_blocking(app)?.runs)
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

pub(super) fn delete_run_artifact_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

pub(super) fn update_artifact_tags_blocking(
    path: String,
    tags: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    if !is_run_artifact_kind(kind)
        && !is_meta_selection_kind(kind)
        && !is_meta_stage_result_kind(kind)
        && !is_ml_confidence_policy_eval_kind(kind)
        && !is_options_expression_artifact_kind(kind)
    {
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
        build_run_artifact_detail_response, build_workflow_artifacts_hydration_payload,
        canonical_json, compute_config_fingerprint_from_artifact, is_meta_artifact_file_name,
        is_meta_stage_artifact_file_name, is_options_expression_artifact_kind,
        is_run_artifact_kind, normalize_dataset_meta, resolve_run_artifact_csv_path,
        resolve_run_artifact_strategy_kind, resolve_run_artifact_timeframe,
    };
    use serde_json::json;

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
    fn run_artifact_kind_alias_accepts_legacy_and_vectorgrid_names() {
        assert!(is_run_artifact_kind("vectorbt_run_artifact"));
        assert!(is_run_artifact_kind("vectorgrid_run_artifact"));
        assert!(!is_run_artifact_kind("vectorbt_meta_selection"));
    }

    #[test]
    fn options_expression_artifact_kind_alias_accepts_vectorgrid_name() {
        assert!(is_options_expression_artifact_kind(
            "vectorgrid_options_expression_v1"
        ));
        assert!(!is_options_expression_artifact_kind("vectorgrid_run_artifact"));
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
        assert_eq!(detail.report_path, "C:/runs/report.xlsx");
        assert_eq!(detail.ml_signal_stage, "validation");
        assert_eq!(detail.ml_bundle_id, "bundle-a");
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
}
