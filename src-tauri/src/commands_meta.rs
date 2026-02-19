use super::{
    build_meta_report_xlsx_bytes, quantile, run_artifacts, sanitize_filename_component,
    TauriProgressSink,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ta_app_core::{
    MetaSelectionRequest, MetaSelectionResult, MlConfidenceLabelConfig, MlConfidenceModel,
    MlConfidenceScope, MlConfidenceTrainConfig, MlConfidenceTrainMetrics,
};
use tauri::Manager;
#[derive(Debug, Clone, Serialize)]
pub struct MlConfidenceModelStatus {
    pub exists: bool,
    pub model_path: String,
    pub bytes: usize,
    pub available_meta_artifacts: usize,
    pub available_labeled_rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_meta_ts_available: Option<String>,
    pub stale: bool,
    pub retrain_recommended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrain_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<MlConfidenceModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainMlConfidenceResponse {
    pub model_path: String,
    pub model_bytes: usize,
    pub model: MlConfidenceModel,
    pub metrics: MlConfidenceTrainMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyMlConfidenceResponse {
    pub model_path: String,
    pub confidences: Vec<Option<f64>>,
    pub warnings: Vec<String>,
}

fn default_stage4_top_k() -> usize {
    10
}

fn default_stage4_min_ml_confidence() -> f64 {
    0.55
}

fn default_stage4_meta_weight() -> f64 {
    0.60
}

fn default_stage4_ml_weight() -> f64 {
    0.35
}

fn default_stage4_uncertainty_weight() -> f64 {
    0.05
}

fn default_stage4_accept_confidence() -> f64 {
    0.70
}

fn default_stage4_review_confidence() -> f64 {
    0.55
}

fn default_stage4_max_accept_uncertainty() -> f64 {
    0.45
}

fn default_stage4_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetaStage4Request {
    pub strategy_kind: String,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub source_meta_path: Option<String>,
    #[serde(default = "default_stage4_top_k")]
    pub top_k: usize,
    /// Probability threshold in [0,1]. Values in (1,100] are interpreted as percentages.
    #[serde(default = "default_stage4_min_ml_confidence")]
    pub min_ml_confidence: f64,
    #[serde(default = "default_stage4_meta_weight")]
    pub meta_weight: f64,
    #[serde(default = "default_stage4_ml_weight")]
    pub ml_weight: f64,
    #[serde(default = "default_stage4_uncertainty_weight")]
    pub uncertainty_weight: f64,
    #[serde(default = "default_stage4_accept_confidence")]
    pub accept_confidence: f64,
    #[serde(default = "default_stage4_review_confidence")]
    pub review_confidence: f64,
    #[serde(default = "default_stage4_max_accept_uncertainty")]
    pub max_accept_uncertainty: f64,
    #[serde(default = "default_stage4_true")]
    pub auto_train_model: bool,
    #[serde(default = "default_stage4_true")]
    pub auto_retrain_stale: bool,
    #[serde(default = "default_stage4_true")]
    pub require_healthy_model: bool,
    #[serde(default = "default_stage4_true")]
    pub fallback_to_meta_if_empty: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaStage4Result {
    pub strategy_kind: String,
    pub timeframe: String,
    pub source_meta_json_path: String,
    pub model_path: String,
    pub model_healthy: bool,
    pub model_trained_now: bool,
    pub groups_considered: usize,
    pub groups_scored: usize,
    pub groups_selected: usize,
    pub min_ml_confidence: f64,
    pub top_k: usize,
    pub meta_weight: f64,
    pub ml_weight: f64,
    pub uncertainty_weight: f64,
    pub accept_confidence: f64,
    pub review_confidence: f64,
    pub max_accept_uncertainty: f64,
    pub decision_counts: Stage4DecisionCounts,
    pub selected_groups: Vec<Value>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage4DecisionCounts {
    pub accept: usize,
    pub review: usize,
    pub reject: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunMetaStage4Response {
    pub ts: String,
    pub stage4_json_path: String,
    pub stage4_json_bytes: usize,
    pub result: MetaStage4Result,
}

fn normalize_ml_scope(mut scope: MlConfidenceScope) -> Result<MlConfidenceScope, String> {
    scope.strategy_kind = scope.strategy_kind.trim().to_string();
    scope.timeframe = normalize_timeframe_label(&scope.timeframe);
    if scope.strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }
    if scope.timeframe.is_empty() {
        return Err("timeframe is required (e.g. '1m')".to_string());
    }
    Ok(scope)
}

fn ml_confidence_models_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("models").join("confidence");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn ml_confidence_model_path(
    scope: &MlConfidenceScope,
    app: &tauri::AppHandle,
) -> Result<std::path::PathBuf, String> {
    let dir = ml_confidence_models_dir(app)?;
    let sk = sanitize_filename_component(&scope.strategy_kind);
    let tf = sanitize_filename_component(&scope.timeframe);
    Ok(dir.join(format!("ml_confidence_{sk}_{tf}.json")))
}

fn load_meta_artifacts_with_paths_blocking(
    app: &tauri::AppHandle,
) -> Result<Vec<(PathBuf, Value)>, String> {
    let dir = run_artifacts::runs_dir(app)?;
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("vectorbt_meta_") {
            continue;
        }
        paths.push(path);
    }
    paths.sort();

    let mut out: Vec<(PathBuf, Value)> = Vec::with_capacity(paths.len());
    for p in paths {
        let raw = match std::fs::read_to_string(&p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push((p, v));
    }
    Ok(out)
}

fn load_meta_artifacts_blocking(app: &tauri::AppHandle) -> Result<Vec<Value>, String> {
    let metas = load_meta_artifacts_with_paths_blocking(app)?;
    Ok(metas.into_iter().map(|(_p, v)| v).collect())
}

fn stage4_training_metas_excluding_source(
    metas_with_paths: Vec<(PathBuf, Value)>,
    source_meta_path: &Path,
) -> (Vec<Value>, usize) {
    let source_canon = std::fs::canonicalize(source_meta_path).ok();
    let mut out: Vec<Value> = Vec::with_capacity(metas_with_paths.len());
    let mut excluded = 0usize;

    for (path, meta) in metas_with_paths {
        let matches_source = if let Some(src) = source_canon.as_ref() {
            std::fs::canonicalize(&path)
                .map(|p| p == *src)
                .unwrap_or(path == source_meta_path)
        } else {
            path == source_meta_path
        };
        if matches_source {
            excluded = excluded.saturating_add(1);
            continue;
        }
        out.push(meta);
    }

    (out, excluded)
}

fn try_load_ml_confidence_model(path: &Path) -> Option<MlConfidenceModel> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<MlConfidenceModel>(&raw).ok()
}

fn should_block_stage4_for_unhealthy_model(
    require_healthy_model: bool,
    model_health: Option<bool>,
) -> bool {
    require_healthy_model && matches!(model_health, Some(false))
}

fn stage4_autotrain_failure_fallback(
    train_err: String,
    model_path: &Path,
    existing_model_loaded: bool,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if existing_model_loaded {
        warnings.push(format!(
            "Auto-train failed ({train_err}); falling back to existing model at '{}'.",
            model_path.to_string_lossy()
        ));
        return Ok(());
    }
    Err(format!(
        "Auto-train failed and no usable existing model is available: {train_err}"
    ))
}

fn ensure_path_within_dir(path: &Path, dir: &Path) -> Result<(), String> {
    let dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !path.starts_with(&dir) {
        return Err("path is outside app runs dir".to_string());
    }
    Ok(())
}

fn suggest_stage4_artifact_path(ts: &str, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = run_artifacts::runs_dir(app)?;
    let safe_ts = sanitize_filename_component(ts);
    let base_name = format!("vectorbt_stage4_{safe_ts}");

    let mut path = dir.join(format!("{base_name}.json"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.json"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn normalize_ml_threshold(raw: f64) -> f64 {
    if !raw.is_finite() {
        return 0.0;
    }
    if raw > 1.0 && raw <= 100.0 {
        return (raw / 100.0).clamp(0.0, 1.0);
    }
    raw.clamp(0.0, 1.0)
}

fn normalize_non_negative(raw: f64, default_v: f64) -> f64 {
    if raw.is_finite() && raw >= 0.0 {
        raw
    } else {
        default_v
    }
}

fn stable_logit(p: f64) -> f64 {
    let pp = p.clamp(1e-9, 1.0 - 1e-9);
    (pp / (1.0 - pp)).ln()
}

fn ml_uncertainty_from_prob(p: f64) -> f64 {
    // 4*p*(1-p) in [0,1], maximum at p=0.5.
    (4.0 * p * (1.0 - p)).clamp(0.0, 1.0)
}

fn mean_std(xs: &[f64]) -> (f64, f64) {
    if xs.is_empty() {
        return (0.0, 1.0);
    }
    let n = xs.len() as f64;
    let mean = xs.iter().copied().sum::<f64>() / n;
    let var = xs
        .iter()
        .copied()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    let std = var.sqrt();
    if std.is_finite() && std > 1e-12 {
        (mean, std)
    } else {
        (mean, 1.0)
    }
}

fn zscore(x: f64, mean: f64, std: f64) -> f64 {
    (x - mean) / std
}

fn decision_bucket_label(
    req: &MetaStage4Request,
    p: Option<f64>,
    unc: Option<f64>,
) -> &'static str {
    let Some(pp) = p else {
        return "reject";
    };
    let uu = unc.unwrap_or(1.0);
    if pp >= req.accept_confidence && uu <= req.max_accept_uncertainty {
        "accept"
    } else if pp >= req.review_confidence {
        "review"
    } else {
        "reject"
    }
}

fn extract_meta_scope(meta: &Value) -> Option<(String, String)> {
    let strat = meta
        .pointer("/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| meta.pointer("/res/strategy_kind").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();
    let tf_raw = meta
        .pointer("/req/timeframe")
        .and_then(|v| v.as_str())
        .or_else(|| meta.pointer("/res/timeframe").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();
    let tf = normalize_timeframe_label(&tf_raw);
    if strat.is_empty() || tf.is_empty() {
        return None;
    }
    Some((strat, tf))
}

fn valid_holdout_row(row: &Value) -> bool {
    let dataset_ok = row
        .get("dataset")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !dataset_ok {
        return false;
    }
    let score = row.get("score_median").and_then(|v| v.as_f64());
    if score.is_none() || !score.unwrap().is_finite() {
        return false;
    }
    let dd = row
        .get("max_dd_worst")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    dd.is_finite()
}

fn group_has_required_ml_features(g: &Value) -> bool {
    g.get("samples").and_then(|v| v.as_u64()).is_some()
        && g.get("pass_rate").and_then(|v| v.as_f64()).is_some()
        && g.get("robust_score_p10").and_then(|v| v.as_f64()).is_some()
        && g.get("robust_score_median")
            .and_then(|v| v.as_f64())
            .is_some()
        && g.get("max_dd_p90").and_then(|v| v.as_f64()).is_some()
        && g.get("meta_score").and_then(|v| v.as_f64()).is_some()
}

fn scope_has_holdout_rows_for_ml(
    metas: &[Value],
    scope: &MlConfidenceScope,
    cfg: &MlConfidenceLabelConfig,
) -> bool {
    let scope_tf = normalize_timeframe_label(&scope.timeframe);
    let scope_sk = scope.strategy_kind.trim();

    for meta in metas {
        let kind = meta.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if kind != "vectorbt_meta_selection" {
            continue;
        }
        let Some((sk, tf)) = extract_meta_scope(meta) else {
            continue;
        };
        if sk != scope_sk || tf != scope_tf {
            continue;
        }
        let groups = meta
            .pointer("/res/groups")
            .and_then(|v| v.as_array())
            .map(|xs| xs.as_slice())
            .unwrap_or(&[]);
        for g in groups {
            if !group_has_required_ml_features(g) {
                continue;
            }
            let Some(rows) = g.get("holdout_rows").and_then(|v| v.as_array()) else {
                continue;
            };
            if rows.len() < cfg.min_holdout_datasets {
                continue;
            }
            if rows.iter().any(valid_holdout_row) {
                return true;
            }
        }
    }

    false
}

fn count_labeled_rows_in_meta(
    meta: &Value,
    cfg: &MlConfidenceLabelConfig,
    use_holdout_rows: bool,
) -> usize {
    let groups = meta
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .map(|xs| xs.as_slice())
        .unwrap_or(&[]);

    let mut rows = 0usize;
    for g in groups {
        if !group_has_required_ml_features(g) {
            continue;
        }

        if use_holdout_rows {
            let Some(holdout_rows) = g.get("holdout_rows").and_then(|v| v.as_array()) else {
                continue;
            };
            if holdout_rows.len() < cfg.min_holdout_datasets {
                continue;
            }
            for r in holdout_rows {
                if valid_holdout_row(r) {
                    rows = rows.saturating_add(1);
                }
            }
        } else {
            let Some(h) = g.get("holdout") else {
                continue;
            };
            let datasets = h.get("datasets").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if datasets < cfg.min_holdout_datasets {
                continue;
            }
            let pass_rate = h.get("pass_rate").and_then(|v| v.as_f64());
            let score_p10 = h.get("score_p10").and_then(|v| v.as_f64());
            if pass_rate.is_none() || score_p10.is_none() {
                continue;
            }
            let max_dd_p90 = h.get("max_dd_p90").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let mut ok = pass_rate.unwrap() >= cfg.min_holdout_pass_rate
                && score_p10.unwrap() >= cfg.min_holdout_score_p10;
            if let Some(dd_max) = cfg.max_holdout_max_dd_p90 {
                ok = ok && max_dd_p90 <= dd_max;
            }
            let _ = ok; // label value isn't needed for counting; just gate for validity.

            rows = rows.saturating_add(1);
        }
    }
    rows
}

fn summarize_meta_artifacts_for_scope(
    metas: &[Value],
    scope: &MlConfidenceScope,
    cfg: &MlConfidenceLabelConfig,
) -> (usize, Option<String>, usize) {
    let mut count = 0usize;
    let mut newest: Option<String> = None;
    let mut labeled_rows = 0usize;

    let scope_tf = normalize_timeframe_label(&scope.timeframe);
    let scope_sk = scope.strategy_kind.trim();
    let use_holdout_rows = scope_has_holdout_rows_for_ml(metas, scope, cfg);

    for meta in metas {
        let kind = meta.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if kind != "vectorbt_meta_selection" {
            continue;
        }
        let Some((sk, tf)) = extract_meta_scope(meta) else {
            continue;
        };
        if sk != scope_sk || tf != scope_tf {
            continue;
        }
        count = count.saturating_add(1);
        if let Some(ts) = meta
            .get("ts")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            let is_newest = newest
                .as_ref()
                .map(|cur| ts.as_str() > cur.as_str())
                .unwrap_or(true);
            if is_newest {
                newest = Some(ts);
            }
        }
        labeled_rows =
            labeled_rows.saturating_add(count_labeled_rows_in_meta(meta, cfg, use_holdout_rows));
    }

    (count, newest, labeled_rows)
}

fn ml_confidence_model_status_blocking(
    scope: MlConfidenceScope,
    app: &tauri::AppHandle,
) -> Result<MlConfidenceModelStatus, String> {
    let scope = normalize_ml_scope(scope)?;
    let path = ml_confidence_model_path(&scope, app)?;
    let model_path = path.to_string_lossy().to_string();
    let exists = path.exists();

    let mut bytes = 0usize;
    let mut model: Option<MlConfidenceModel> = None;
    let mut error: Option<String> = None;
    let mut label_cfg = MlConfidenceLabelConfig::default();

    if exists {
        bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len() as usize;
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match serde_json::from_str::<MlConfidenceModel>(&raw) {
            Ok(m) => {
                label_cfg = m.label.clone();
                model = Some(m);
            }
            Err(e) => {
                error = Some(e.to_string());
            }
        }
    }

    let metas = load_meta_artifacts_blocking(app).unwrap_or_default();
    let (available_meta_artifacts, newest_meta_ts_available, available_labeled_rows) =
        summarize_meta_artifacts_for_scope(&metas, &scope, &label_cfg);

    // Staleness/retrain policy: conservative defaults to avoid surprise compute.
    let mut stale = false;
    let mut retrain_recommended = false;
    let mut retrain_reason: Option<String> = None;

    if model.is_none() {
        if !exists {
            retrain_recommended = true;
            retrain_reason = Some("No model saved for this scope.".to_string());
        } else if error.is_some() {
            retrain_recommended = true;
            retrain_reason =
                Some("Model exists but failed to load; retrain recommended.".to_string());
        }
    } else if let Some(m) = model.as_ref() {
        let meta = m.meta.as_ref();
        if meta.is_none() {
            stale = available_meta_artifacts > 0;
            if stale && available_labeled_rows >= 25 {
                retrain_recommended = true;
                retrain_reason = Some(
                    "Legacy model (missing training metadata); retrain recommended.".to_string(),
                );
            } else if stale {
                retrain_reason = Some(
                    "Legacy model (missing training metadata); retrain when more labels exist."
                        .to_string(),
                );
            }
        } else if let Some(mm) = meta {
            if let (Some(used), Some(avail)) = (
                mm.newest_meta_ts_used.as_ref(),
                newest_meta_ts_available.as_ref(),
            ) {
                if avail.as_str() > used.as_str() {
                    stale = true;
                }
            } else if newest_meta_ts_available.is_some() {
                stale = true;
            }
            if let Some(used_count) = mm.meta_artifacts_used {
                if available_meta_artifacts > used_count {
                    stale = true;
                }
            } else if available_meta_artifacts > 0 {
                stale = true;
            }

            let trained_rows = mm.trained_rows;
            let new_rows = available_labeled_rows.saturating_sub(trained_rows);
            let threshold = std::cmp::max(25, trained_rows / 10);

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let cooldown_ok = now_ms.saturating_sub(mm.trained_at_ms) > 60_000; // 60s cooldown

            if stale && new_rows >= threshold && cooldown_ok {
                retrain_recommended = true;
                retrain_reason = Some(format!(
                    "Model stale: new labeled rows {new_rows} (threshold {threshold})."
                ));
            } else if stale && new_rows > 0 {
                retrain_reason = Some(format!(
                    "Model stale: new labeled rows {new_rows} (<{threshold})."
                ));
            } else if stale && !cooldown_ok {
                retrain_reason = Some("Model stale but recently trained (cooldown).".to_string());
            }
        }
    }

    Ok(MlConfidenceModelStatus {
        exists,
        model_path,
        bytes,
        available_meta_artifacts,
        available_labeled_rows,
        newest_meta_ts_available,
        stale,
        retrain_recommended,
        retrain_reason,
        model,
        error,
    })
}

#[tauri::command]
pub async fn ml_confidence_model_status(
    scope: MlConfidenceScope,
    app: tauri::AppHandle,
) -> Result<MlConfidenceModelStatus, String> {
    tauri::async_runtime::spawn_blocking(move || ml_confidence_model_status_blocking(scope, &app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn train_ml_confidence_model(
    scope: MlConfidenceScope,
    cfg: Option<MlConfidenceTrainConfig>,
    app: tauri::AppHandle,
) -> Result<TrainMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let cfg = cfg.unwrap_or_default();
        let metas = load_meta_artifacts_blocking(&app)?;
        let (model, metrics) =
            ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope, &metas, cfg)?;

        let path = ml_confidence_model_path(&model.scope, &app)?;
        let bytes = serde_json::to_vec_pretty(&model).map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

        Ok(TrainMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            model_bytes: bytes.len(),
            model,
            metrics,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_ml_confidence_model(
    scope: MlConfidenceScope,
    groups: Vec<Value>,
    app: tauri::AppHandle,
) -> Result<ApplyMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path(&scope, &app)?;
        if !path.exists() {
            return Err("ML confidence model not found. Train it first.".to_string());
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let model: MlConfidenceModel = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

        let mut warnings: Vec<String> = Vec::new();
        if normalize_timeframe_label(&model.scope.timeframe) != scope.timeframe
            || model.scope.strategy_kind != scope.strategy_kind
        {
            warnings.push("Model scope does not match request scope. Results may be unreliable.".to_string());
        }
        if let Some(meta) = model.meta.as_ref() {
            if !meta.healthy {
                warnings.push("Model health is poor (AUC/Brier gates failed). Treat ML confidence as untrustworthy.".to_string());
            }
        }

        let confidences: Vec<Option<f64>> = groups
            .iter()
            .map(|g| ta_app_core::predict_ml_confidence_for_group(&model, g))
            .collect();

        Ok(ApplyMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            confidences,
            warnings,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_ml_confidence_model(
    scope: MlConfidenceScope,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path(&scope, &app)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn normalize_meta_stage4_request(mut req: MetaStage4Request) -> Result<MetaStage4Request, String> {
    req.strategy_kind = req.strategy_kind.trim().to_string();
    if req.strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }

    req.timeframe = req
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|s| !s.is_empty());

    req.source_meta_path = req
        .source_meta_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    req.top_k = req.top_k.max(1);
    req.min_ml_confidence = normalize_ml_threshold(req.min_ml_confidence);
    req.accept_confidence = normalize_ml_threshold(req.accept_confidence);
    req.review_confidence = normalize_ml_threshold(req.review_confidence);
    req.max_accept_uncertainty = normalize_ml_threshold(req.max_accept_uncertainty);
    if req.review_confidence > req.accept_confidence {
        req.review_confidence = req.accept_confidence;
    }

    req.meta_weight = normalize_non_negative(req.meta_weight, default_stage4_meta_weight());
    req.ml_weight = normalize_non_negative(req.ml_weight, default_stage4_ml_weight());
    req.uncertainty_weight =
        normalize_non_negative(req.uncertainty_weight, default_stage4_uncertainty_weight());
    let sum_w = req.meta_weight + req.ml_weight + req.uncertainty_weight;
    if !(sum_w.is_finite() && sum_w > 0.0) {
        req.meta_weight = default_stage4_meta_weight();
        req.ml_weight = default_stage4_ml_weight();
        req.uncertainty_weight = default_stage4_uncertainty_weight();
    } else {
        req.meta_weight /= sum_w;
        req.ml_weight /= sum_w;
        req.uncertainty_weight /= sum_w;
    }

    if !req.auto_train_model {
        req.auto_retrain_stale = false;
    }
    Ok(req)
}

fn resolve_meta_stage4_source(
    req: &MetaStage4Request,
    app: &tauri::AppHandle,
) -> Result<(PathBuf, Value, String), String> {
    let runs_dir = run_artifacts::runs_dir(app)?;
    let strategy_kind = req.strategy_kind.trim();
    let req_tf = req.timeframe.as_deref().unwrap_or("");

    if let Some(path_raw) = req.source_meta_path.as_deref() {
        let path = PathBuf::from(path_raw);
        ensure_path_within_dir(&path, &runs_dir)?;
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
        if kind != "vectorbt_meta_selection" {
            return Err(format!(
                "source_meta_path must point to kind='vectorbt_meta_selection' (got '{kind}')."
            ));
        }
        let Some((sk, tf)) = extract_meta_scope(&v) else {
            return Err(
                "source meta artifact is missing scope metadata (strategy/timeframe)".to_string(),
            );
        };
        if sk != strategy_kind {
            return Err(format!(
                "source meta strategy mismatch: expected '{strategy_kind}', got '{sk}'."
            ));
        }
        if !req_tf.is_empty() && tf != req_tf {
            return Err(format!(
                "source meta timeframe mismatch: expected '{req_tf}', got '{tf}'."
            ));
        }
        return Ok((path, v, tf));
    }

    let metas = load_meta_artifacts_with_paths_blocking(app)?;
    let mut chosen: Option<(String, PathBuf, Value, String)> = None;
    for (path, v) in metas {
        let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
        if kind != "vectorbt_meta_selection" {
            continue;
        }
        let Some((sk, tf)) = extract_meta_scope(&v) else {
            continue;
        };
        if sk != strategy_kind {
            continue;
        }
        if !req_tf.is_empty() && tf != req_tf {
            continue;
        }
        let ts = v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let take = chosen
            .as_ref()
            .map(|(best_ts, _, _, _)| ts.as_str() > best_ts.as_str())
            .unwrap_or(true);
        if take {
            chosen = Some((ts, path, v, tf));
        }
    }

    if let Some((_ts, path, v, tf)) = chosen {
        return Ok((path, v, tf));
    }
    if req_tf.is_empty() {
        Err(format!(
            "No saved meta artifact found for strategy_kind='{strategy_kind}'. Run meta analysis first."
        ))
    } else {
        Err(format!(
            "No saved meta artifact found for strategy_kind='{strategy_kind}' timeframe='{req_tf}'."
        ))
    }
}

fn group_rank_score(group: &Value) -> f64 {
    group
        .get("final_score")
        .and_then(|v| v.as_f64())
        .or_else(|| group.get("meta_score").and_then(|v| v.as_f64()))
        .unwrap_or(f64::NEG_INFINITY)
}

fn run_meta_stage4_experimental_blocking(
    req: MetaStage4Request,
    app: tauri::AppHandle,
) -> Result<RunMetaStage4Response, String> {
    let req = normalize_meta_stage4_request(req)?;
    let (source_meta_path, source_meta, meta_tf) = resolve_meta_stage4_source(&req, &app)?;
    let timeframe = req.timeframe.clone().unwrap_or(meta_tf);
    let scope = normalize_ml_scope(MlConfidenceScope {
        strategy_kind: req.strategy_kind.clone(),
        timeframe,
    })?;

    let mut warnings: Vec<String> = Vec::new();
    let mut model_trained_now = false;
    let model_path = ml_confidence_model_path(&scope, &app)?;
    let mut model = try_load_ml_confidence_model(&model_path);

    let mut status = ml_confidence_model_status_blocking(scope.clone(), &app)?;
    let should_train = req.auto_train_model
        && (!status.exists
            || status.error.is_some()
            || (req.auto_retrain_stale && status.retrain_recommended));
    if should_train {
        let metas_with_paths = load_meta_artifacts_with_paths_blocking(&app)?;
        let (metas, excluded_source_count) =
            stage4_training_metas_excluding_source(metas_with_paths, &source_meta_path);
        if excluded_source_count > 0 {
            warnings.push(format!(
                "Auto-train excluded {excluded_source_count} source meta artifact(s) to avoid train/score leakage."
            ));
        }
        let cfg = MlConfidenceTrainConfig::default();
        match ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope.clone(), &metas, cfg)
        {
            Ok((trained_model, _metrics)) => {
                let bytes = serde_json::to_vec_pretty(&trained_model).map_err(|e| e.to_string())?;
                std::fs::write(&model_path, &bytes).map_err(|e| e.to_string())?;
                model = Some(trained_model);
                model_trained_now = true;
                status = ml_confidence_model_status_blocking(scope.clone(), &app)?;
            }
            Err(e) => {
                stage4_autotrain_failure_fallback(e, &model_path, model.is_some(), &mut warnings)?;
            }
        }
    }

    let model: MlConfidenceModel = if let Some(m) = model {
        m
    } else {
        if !model_path.exists() {
            return Err(
                "ML confidence model not found for this scope. Train one first (or enable auto-train)."
                    .to_string(),
            );
        }
        let model_raw = std::fs::read_to_string(&model_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&model_raw).map_err(|e| e.to_string())?
    };
    let model_health = model.meta.as_ref().map(|m| m.healthy);
    let model_healthy = model_health.unwrap_or(false);
    if should_block_stage4_for_unhealthy_model(req.require_healthy_model, model_health) {
        return Err(
            "ML model is unhealthy for this scope. Retrain with more labeled rows or disable strict healthy-model requirement."
                .to_string(),
        );
    }
    match model_health {
        Some(true) => {}
        Some(false) => {
            warnings.push(
                "Using an unhealthy ML model (AUC/Brier gates failed). Treat Stage 4 ranking as experimental."
                    .to_string(),
            );
        }
        None => {
            warnings.push(
                "Using a legacy ML model without health metadata; strict healthy-model gate is skipped for compatibility."
                    .to_string(),
            );
        }
    }
    if status.stale {
        warnings.push("ML model is stale relative to available meta artifacts.".to_string());
    }

    let groups = source_meta
        .pointer("/res/groups")
        .and_then(|v| v.as_array())
        .map(|xs| xs.as_slice())
        .unwrap_or(&[]);
    if groups.is_empty() {
        return Err("Source meta artifact has no groups to rank.".to_string());
    }

    #[derive(Clone)]
    struct Stage4Entry {
        ml: Option<f64>,
        ml_logit: Option<f64>,
        ml_uncertainty: Option<f64>,
        meta_score: f64,
        final_rank_score: Option<f64>,
        group: Value,
    }

    let mut entries: Vec<Stage4Entry> = Vec::with_capacity(groups.len());
    let mut groups_scored = 0usize;
    let mut groups_bad_meta_score = 0usize;
    for g in groups {
        let ml = ta_app_core::predict_ml_confidence_for_group(&model, g).filter(|p| p.is_finite());
        if ml.is_some() {
            groups_scored = groups_scored.saturating_add(1);
        }
        let ml_logit = ml.map(stable_logit);
        let ml_uncertainty = ml.map(ml_uncertainty_from_prob);
        let raw_meta_score = group_rank_score(g);
        let meta_score = if raw_meta_score.is_finite() {
            raw_meta_score
        } else {
            groups_bad_meta_score = groups_bad_meta_score.saturating_add(1);
            0.0
        };
        let mut g2 = g.clone();
        if let Some(obj) = g2.as_object_mut() {
            if let Some(p) = ml {
                obj.insert("ml_confidence".to_string(), serde_json::json!(p));
            } else {
                obj.insert("ml_confidence".to_string(), Value::Null);
            }
            if let Some(u) = ml_uncertainty {
                obj.insert("ml_uncertainty".to_string(), serde_json::json!(u));
            } else {
                obj.insert("ml_uncertainty".to_string(), Value::Null);
            }
            obj.insert("final_rank_score".to_string(), Value::Null);
        }
        entries.push(Stage4Entry {
            ml,
            ml_logit,
            ml_uncertainty,
            meta_score,
            final_rank_score: None,
            group: g2,
        });
    }

    if groups_bad_meta_score > 0 {
        warnings.push(format!(
            "{} group(s) had non-finite meta score; fallback value 0.0 was used.",
            groups_bad_meta_score
        ));
    }

    let candidate_indices: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.ml.filter(|p| *p >= req.min_ml_confidence).map(|_| i))
        .collect();

    if !candidate_indices.is_empty() {
        let meta_vals: Vec<f64> = candidate_indices
            .iter()
            .map(|&i| entries[i].meta_score)
            .collect();
        let ml_vals: Vec<f64> = candidate_indices
            .iter()
            .filter_map(|&i| entries[i].ml_logit)
            .collect();
        let unc_vals: Vec<f64> = candidate_indices
            .iter()
            .filter_map(|&i| entries[i].ml_uncertainty)
            .collect();

        let (meta_mean, meta_std) = mean_std(&meta_vals);
        let (ml_mean, ml_std) = mean_std(&ml_vals);
        let (unc_mean, unc_std) = mean_std(&unc_vals);

        for &i in &candidate_indices {
            let z_meta = zscore(entries[i].meta_score, meta_mean, meta_std);
            let z_ml = zscore(entries[i].ml_logit.unwrap_or(0.0), ml_mean, ml_std);
            let z_unc = zscore(entries[i].ml_uncertainty.unwrap_or(0.0), unc_mean, unc_std);
            let final_rank =
                req.meta_weight * z_meta + req.ml_weight * z_ml - req.uncertainty_weight * z_unc;
            entries[i].final_rank_score = Some(final_rank);
            if let Some(obj) = entries[i].group.as_object_mut() {
                obj.insert(
                    "final_rank_score".to_string(),
                    serde_json::json!(final_rank),
                );
                obj.insert("stage4_fallback".to_string(), Value::Bool(false));
            }
        }
    }

    let mut selected_groups: Vec<Value> = Vec::new();
    if !candidate_indices.is_empty() {
        let mut ranked = candidate_indices;
        ranked.sort_by(|&a, &b| {
            let af = entries[a].final_rank_score.unwrap_or(f64::NEG_INFINITY);
            let bf = entries[b].final_rank_score.unwrap_or(f64::NEG_INFINITY);
            let ap = entries[a].ml.unwrap_or(-1.0);
            let bp = entries[b].ml.unwrap_or(-1.0);
            bf.total_cmp(&af)
                .then_with(|| bp.total_cmp(&ap))
                .then_with(|| entries[b].meta_score.total_cmp(&entries[a].meta_score))
        });
        selected_groups = ranked
            .into_iter()
            .take(req.top_k)
            .map(|i| entries[i].group.clone())
            .collect();
    } else if req.fallback_to_meta_if_empty {
        let mut fallback_indices: Vec<usize> = (0..entries.len()).collect();
        fallback_indices.sort_by(|&a, &b| entries[b].meta_score.total_cmp(&entries[a].meta_score));
        selected_groups = fallback_indices
            .into_iter()
            .take(req.top_k)
            .map(|i| {
                let mut g = entries[i].group.clone();
                if let Some(obj) = g.as_object_mut() {
                    obj.insert("stage4_fallback".to_string(), Value::Bool(true));
                    if !obj
                        .get("final_rank_score")
                        .map(|v| v.is_number())
                        .unwrap_or(false)
                    {
                        obj.insert(
                            "final_rank_score".to_string(),
                            serde_json::json!(entries[i].meta_score),
                        );
                    }
                }
                g
            })
            .collect();
        warnings.push(format!(
            "No groups met min_ml_confidence={:.3}. Fell back to meta-score ranking.",
            req.min_ml_confidence
        ));
    }

    let mut decision_counts = Stage4DecisionCounts {
        accept: 0,
        review: 0,
        reject: 0,
    };
    for (i, g) in selected_groups.iter_mut().enumerate() {
        if let Some(obj) = g.as_object_mut() {
            obj.insert(
                "stage4_rank".to_string(),
                Value::Number(serde_json::Number::from((i + 1) as u64)),
            );
            if !obj
                .get("final_rank_score")
                .map(|v| v.is_number())
                .unwrap_or(false)
            {
                let fallback_rank = obj
                    .get("final_score")
                    .and_then(|v| v.as_f64())
                    .or_else(|| obj.get("meta_score").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                obj.insert(
                    "final_rank_score".to_string(),
                    serde_json::json!(fallback_rank),
                );
            }
            let ml = obj.get("ml_confidence").and_then(|v| v.as_f64());
            let unc = obj.get("ml_uncertainty").and_then(|v| v.as_f64());
            let bucket = decision_bucket_label(&req, ml, unc);
            obj.insert("decision_bucket".to_string(), serde_json::json!(bucket));
            match bucket {
                "accept" => decision_counts.accept = decision_counts.accept.saturating_add(1),
                "review" => decision_counts.review = decision_counts.review.saturating_add(1),
                _ => decision_counts.reject = decision_counts.reject.saturating_add(1),
            }
        }
    }

    if groups_scored < groups.len() {
        warnings.push(format!(
            "ML model produced confidences for {}/{} groups.",
            groups_scored,
            groups.len()
        ));
    }

    let ts = chrono::Utc::now().to_rfc3339();
    let result = MetaStage4Result {
        strategy_kind: scope.strategy_kind.clone(),
        timeframe: scope.timeframe.clone(),
        source_meta_json_path: source_meta_path.to_string_lossy().to_string(),
        model_path: model_path.to_string_lossy().to_string(),
        model_healthy,
        model_trained_now,
        groups_considered: groups.len(),
        groups_scored,
        groups_selected: selected_groups.len(),
        min_ml_confidence: req.min_ml_confidence,
        top_k: req.top_k,
        meta_weight: req.meta_weight,
        ml_weight: req.ml_weight,
        uncertainty_weight: req.uncertainty_weight,
        accept_confidence: req.accept_confidence,
        review_confidence: req.review_confidence,
        max_accept_uncertainty: req.max_accept_uncertainty,
        decision_counts,
        selected_groups,
        warnings,
    };

    let out_obj = serde_json::json!({
        "version": 1,
        "kind": "vectorbt_meta_stage4_experimental",
        "ts": ts,
        "tags": [],
        "req": req,
        "res": &result,
    });
    let out_bytes = serde_json::to_vec_pretty(&out_obj).map_err(|e| e.to_string())?;
    let out_path = suggest_stage4_artifact_path(&ts, &app)?;
    std::fs::write(&out_path, &out_bytes).map_err(|e| e.to_string())?;
    run_artifacts::prune_old_stage4_artifacts_blocking(&app, 100);

    Ok(RunMetaStage4Response {
        ts,
        stage4_json_path: out_path.to_string_lossy().to_string(),
        stage4_json_bytes: out_bytes.len(),
        result,
    })
}

#[tauri::command]
pub async fn run_meta_stage4_experimental(
    req: MetaStage4Request,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMetaStage4Response, String> {
    let _run_guard = state.try_begin_run()?;
    tauri::async_runtime::spawn_blocking(move || run_meta_stage4_experimental_blocking(req, app))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct RunMetaSelectionResponse {
    pub ts: String,
    pub report_xlsx_path: String,
    pub report_xlsx_bytes: usize,
    pub meta_json_path: String,
    pub meta_json_bytes: usize,
    pub result: MetaSelectionResult,
}

#[tauri::command]
pub async fn run_meta_selection(
    req: MetaSelectionRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMetaSelectionResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || run_meta_selection_blocking(req, app2, cancel2))
        .await
        .map_err(|e| e.to_string())?
}

fn reports_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn suggest_meta_report_path(
    ts: &str,
    app: &tauri::AppHandle,
    ext: &'static str,
) -> Result<std::path::PathBuf, String> {
    let dir = reports_dir(app)?;
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorbt_meta_{safe_ts}");
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

fn run_meta_selection_blocking(
    mut req: MetaSelectionRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunMetaSelectionResponse, String> {
    let dir = run_artifacts::runs_dir(&app)?;
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with("vectorbt_meta_") {
            continue;
        }
        paths.push(path);
    }
    paths.sort();

    let sink = TauriProgressSink { app: app.clone() };

    let mut artifacts: Vec<Value> = Vec::with_capacity(paths.len());
    for (i, p) in paths.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let raw = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        artifacts.push(v);
        ta_app_core::progress::emit_double_ma_progress(
            Some(&sink),
            i + 1,
            paths.len(),
            "meta_read",
        );
    }

    let supports_holdout = req.strategy_kind == "CompositeCpu"
        || req.strategy_kind == "CompositeMo"
        || req.strategy_kind == "DoubleMa"
        || req.strategy_kind == "CompositeTrend"
        || req.strategy_kind == "RegimeHybrid"
        || req.strategy_kind == "BreakoutMomentum";
    let holdout_tag = if req.eval_holdout && supports_holdout {
        req.holdout_tag
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    } else {
        None
    };

    let (train_artifacts, holdout_artifacts) = if let Some(tag) = holdout_tag.as_deref() {
        let mut train: Vec<Value> = Vec::with_capacity(artifacts.len());
        let mut holdout: Vec<Value> = Vec::new();
        for a in artifacts {
            if artifact_has_tag(&a, tag) {
                holdout.push(a);
            } else {
                train.push(a);
            }
        }
        (train, holdout)
    } else {
        (artifacts, Vec::new())
    };

    // Parity guard: if timeframe filter isn't set, infer it from the candidate pool.
    // This prevents mixing artifacts across multiple timeframes in meta-selection.
    let mut auto_tf_warning: Option<String> = None;
    let tf_set = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .is_some();
    if !tf_set {
        let mut uniq: HashSet<String> = HashSet::new();
        let mut tfs: Vec<String> = Vec::new();
        let mut missing = 0usize;

        for a in &train_artifacts {
            if !artifact_matches_meta_req(a, &req) {
                continue;
            }
            let raw = a
                .pointer("/dataset_meta/timeframe")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tf = normalize_timeframe_label(raw);
            if tf.is_empty() {
                missing += 1;
                continue;
            }
            if uniq.insert(tf.clone()) {
                tfs.push(tf);
            }
        }

        if tfs.len() == 1 {
            let tf = tfs[0].clone();
            req.timeframe = Some(tf.clone());
            if missing > 0 {
                auto_tf_warning = Some(format!(
                    "Detected timeframe='{tf}'. Excluded {missing} artifact(s) missing dataset_meta.timeframe."
                ));
            }
        } else if tfs.len() > 1 {
            tfs.sort();
            let list = tfs.join(", ");
            let miss = if missing > 0 {
                format!(" + {missing} missing timeframe")
            } else {
                String::new()
            };
            return Err(format!(
                "Meta selection requires a timeframe filter (found multiple timeframes: {list}{miss})."
            ));
        } else if missing > 0 {
            return Err(format!(
                "Meta selection requires a timeframe filter ({missing} candidate artifact(s) are missing dataset_meta.timeframe)."
            ));
        }
    }

    let mut result = ta_app_core::meta_select_from_artifacts(&req, &train_artifacts, Some(&sink));
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    if let Some(w) = auto_tf_warning {
        result.warnings.push(w);
    }

    if req.eval_holdout && !supports_holdout {
        result.warnings.push(format!(
            "Holdout evaluation is not supported for strategy_kind='{}'. Skipping.",
            req.strategy_kind
        ));
    }

    if let Some(tag) = holdout_tag.as_deref() {
        // Optional: evaluate only a subset of groups to reduce expensive holdout compute.
        // Default remains evaluating all groups.
        let mut selected_groups: Option<Vec<usize>> = None;
        let top_n = req.holdout_eval_top_n.unwrap_or(0);
        let explore_n = req.holdout_eval_explore_n;
        if (top_n > 0 || explore_n > 0) && !result.groups.is_empty() {
            let total = result.groups.len();
            let mut rank_by = req
                .holdout_eval_rank_by
                .as_deref()
                .unwrap_or("auto")
                .trim()
                .to_ascii_lowercase();
            if rank_by.is_empty() {
                rank_by = "auto".to_string();
            }

            // Try ML ranking if requested/auto and a healthy model exists.
            let mut ml_model: Option<MlConfidenceModel> = None;
            if rank_by != "meta" {
                if let Some(tf) = req
                    .timeframe
                    .as_deref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    let scope = MlConfidenceScope {
                        strategy_kind: req.strategy_kind.clone(),
                        timeframe: normalize_timeframe_label(tf),
                    };
                    if let Ok(path) = ml_confidence_model_path(&scope, &app) {
                        if path.exists() {
                            if let Ok(raw) = std::fs::read_to_string(&path) {
                                if let Ok(model) = serde_json::from_str::<MlConfidenceModel>(&raw) {
                                    let healthy =
                                        model.meta.as_ref().map(|m| m.healthy).unwrap_or(false);
                                    if healthy {
                                        ml_model = Some(model);
                                    } else {
                                        result.warnings.push(
                                            "Holdout selection: ML model exists but is unhealthy/missing health metadata; falling back to meta score.".to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if rank_by != "ml" && rank_by != "meta" && rank_by != "auto" {
                result.warnings.push(format!(
                    "Holdout selection: unknown holdout_eval_rank_by='{rank_by}', using auto."
                ));
                rank_by = "auto".to_string();
            }

            let using_ml = ml_model.is_some() && (rank_by == "ml" || rank_by == "auto");
            if rank_by == "ml" && !using_ml {
                result.warnings.push(
                    "Holdout selection: requested ML ranking but no healthy model is available; falling back to meta score.".to_string(),
                );
            }

            let mut selected: Vec<usize> = Vec::new();
            if using_ml {
                let model = ml_model.as_ref().unwrap();
                let mut scored: Vec<(usize, f64)> = Vec::with_capacity(total);
                for (i, g) in result.groups.iter().enumerate() {
                    let v = serde_json::json!({
                        "samples": g.samples,
                        "pass_rate": g.pass_rate,
                        "robust_score_p10": g.robust_score_p10,
                        "robust_score_median": g.robust_score_median,
                        "max_dd_p90": g.max_dd_p90,
                        "cross_score_median": g.cross_score_median,
                        "meta_score": g.meta_score,
                    });
                    let p = ta_app_core::predict_ml_confidence_for_group(model, &v).unwrap_or(-1.0);
                    scored.push((i, p));
                }
                scored.sort_by(|a, b| b.1.total_cmp(&a.1));

                let exploit_n = top_n.min(total);
                for (i, _p) in scored.iter().take(exploit_n) {
                    selected.push(*i);
                }

                if explore_n > 0 {
                    let mut rem: Vec<(usize, f64)> = scored
                        .iter()
                        .filter(|(i, _)| !selected.contains(i))
                        .map(|(i, p)| (*i, (p - 0.5).abs()))
                        .collect();
                    rem.sort_by(|a, b| a.1.total_cmp(&b.1));
                    for (i, _) in rem
                        .into_iter()
                        .take(explore_n.min(total.saturating_sub(selected.len())))
                    {
                        selected.push(i);
                    }
                }
            } else {
                if explore_n > 0 {
                    result.warnings.push(
                        "Holdout selection: explore_n is ignored unless ML ranking is available."
                            .to_string(),
                    );
                }
                let mut scored: Vec<(usize, f64)> = result
                    .groups
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (i, g.meta_score))
                    .collect();
                scored.sort_by(|a, b| b.1.total_cmp(&a.1));
                let exploit_n = top_n.min(total);
                for (i, _s) in scored.iter().take(exploit_n) {
                    selected.push(*i);
                }
            }

            selected.sort();
            selected.dedup();
            if !selected.is_empty() && selected.len() < total {
                result.warnings.push(format!(
                    "Holdout evaluation: evaluating {}/{} groups (top_n={}, explore_n={}, rank_by={}).",
                    selected.len(),
                    total,
                    top_n,
                    explore_n,
                    req.holdout_eval_rank_by.as_deref().unwrap_or("auto")
                ));
                selected_groups = Some(selected);
            }
        }

        apply_holdout_evaluation(
            &mut result,
            &req,
            &holdout_artifacts,
            &app,
            cancel.as_ref(),
            &sink,
            tag,
            selected_groups.as_deref(),
        )?;
    }

    let ts = chrono::Utc::now().to_rfc3339();

    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 0, 1, "meta_export");
    let xlsx_bytes = build_meta_report_xlsx_bytes(&ts, &result)?;
    let report_path = suggest_meta_report_path(&ts, &app, "xlsx")?;
    std::fs::write(&report_path, &xlsx_bytes).map_err(|e| e.to_string())?;
    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 1, 1, "meta_export");

    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 0, 1, "meta_save");
    let meta_obj = serde_json::json!({
        "version": 1,
        "kind": "vectorbt_meta_selection",
        "ts": ts,
        "report_xlsx_path": report_path.to_string_lossy().to_string(),
        "tags": [],
        "req": req,
        "res": result,
    });
    let meta_bytes = serde_json::to_vec_pretty(&meta_obj).map_err(|e| e.to_string())?;
    let meta_path = run_artifacts::suggest_meta_artifact_path(&ts, &app)?;
    std::fs::write(&meta_path, &meta_bytes).map_err(|e| e.to_string())?;
    // Retention policy: keep newest N untagged meta runs; tags preserve older runs.
    run_artifacts::prune_old_meta_artifacts_blocking(&app, 100);
    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 1, 1, "meta_save");

    Ok(RunMetaSelectionResponse {
        ts,
        report_xlsx_path: report_path.to_string_lossy().to_string(),
        report_xlsx_bytes: xlsx_bytes.len(),
        meta_json_path: meta_path.to_string_lossy().to_string(),
        meta_json_bytes: meta_bytes.len(),
        result,
    })
}

fn artifact_has_tag(artifact: &Value, tag: &str) -> bool {
    let tag = tag.trim();
    if tag.is_empty() {
        return false;
    }
    let tags = artifact
        .get("tags")
        .and_then(|v| v.as_array())
        .map_or(&[][..], |xs| xs.as_slice());
    tags.iter()
        .filter_map(|v| v.as_str())
        .any(|t| t.trim() == tag)
}

fn normalize_timeframe_label(raw: &str) -> String {
    let s0 = raw.trim().to_ascii_lowercase();
    if s0.is_empty() {
        return String::new();
    }
    // Back-compat: older artifacts may store inferred timeframes like "120s".
    if let Some(sec_str) = s0.strip_suffix('s') {
        if let Ok(sec) = sec_str.parse::<i64>() {
            let mapped = match sec {
                60 => Some("1m"),
                120 => Some("2m"),
                180 => Some("3m"),
                300 => Some("5m"),
                900 => Some("15m"),
                1800 => Some("30m"),
                3600 => Some("1h"),
                7200 => Some("2h"),
                14_400 => Some("4h"),
                86_400 => Some("1d"),
                _ => None,
            };
            if let Some(lbl) = mapped {
                return lbl.to_string();
            }
        }
    }
    s0
}

fn artifact_matches_meta_req(artifact: &Value, req: &MetaSelectionRequest) -> bool {
    let kind = artifact.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if kind != "vectorbt_run_artifact" {
        return false;
    }

    let strat = artifact
        .pointer("/run/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| {
            artifact
                .pointer("/run/req/strategyKind")
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");
    if strat != req.strategy_kind {
        return false;
    }

    if let Some(tf) = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let tf2 = artifact
            .pointer("/dataset_meta/timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if normalize_timeframe_label(tf2) != normalize_timeframe_label(tf) {
            return false;
        }
    }

    if let Some(tag) = req
        .required_tag
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if !artifact_has_tag(artifact, tag) {
            return false;
        }
    }

    artifact.pointer("/validation/res/candidates").is_some()
}

fn artifact_csv_path(artifact: &Value) -> Option<String> {
    let p = artifact
        .pointer("/data/csv_path")
        .and_then(|v| v.as_str())
        .or_else(|| {
            artifact
                .pointer("/dataset_meta/csv_path")
                .and_then(|v| v.as_str())
        })?;
    let s = p.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn artifact_dataset_label(artifact: &Value, csv_path: &str) -> String {
    let lbl = artifact
        .pointer("/dataset_meta/label")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(l) = lbl {
        return l;
    }
    Path::new(csv_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("holdout")
        .to_string()
}

#[derive(Default, Clone)]
struct HoldoutAcc {
    scores: Vec<f64>,
    sharpes: Vec<f64>,
    max_dds: Vec<f64>,
    pnls: Vec<f64>,
    passes: usize,
}

fn apply_holdout_evaluation(
    result: &mut MetaSelectionResult,
    req: &MetaSelectionRequest,
    holdout_artifacts: &[Value],
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
    sink: &TauriProgressSink,
    holdout_tag: &str,
    selected_groups: Option<&[usize]>,
) -> Result<(), String> {
    if result.groups.is_empty() {
        return Ok(());
    }

    let selected_mask: Option<Vec<bool>> = selected_groups.map(|idxs| {
        let mut m = vec![false; result.groups.len()];
        for &i in idxs {
            if i < m.len() {
                m[i] = true;
            }
        }
        m
    });
    let is_selected = |i: usize, m: Option<&Vec<bool>>| m.map(|mm| mm[i]).unwrap_or(true);

    let heavy = matches!(req.holdout_mode, ta_app_core::MetaHoldoutMode::Heavy);
    let (hv_seed, hv_wf, hv_perturb, hv_mc, hv_engine, hv_iterative) = if heavy {
        if let Some(cfg) = req.holdout_validation.clone() {
            (
                cfg.seed,
                cfg.walk_forward,
                cfg.perturb,
                cfg.trade_bootstrap,
                cfg.validation_engine,
                cfg.iterative,
            )
        } else {
            (
                1u64,
                None,
                None,
                None,
                ta_app_core::ValidationEngine::Vectorized,
                ta_app_core::IterativeSimConfig::default(),
            )
        }
    } else {
        (
            1u64,
            None,
            None,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
        )
    };

    // Filter to holdout artifacts that match the request and actually have validation results.
    let holdout: Vec<&Value> = holdout_artifacts
        .iter()
        .filter(|a| artifact_matches_meta_req(a, req))
        .collect();
    if holdout.is_empty() {
        return Ok(());
    }

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let mut uniq_paths: Vec<(String, String)> = Vec::new(); // (csv_path, label)
    let mut seen: HashSet<String> = HashSet::new();
    for art in &holdout {
        let Some(csv_path) = artifact_csv_path(art) else {
            continue;
        };
        if !seen.insert(csv_path.clone()) {
            continue;
        }
        uniq_paths.push((csv_path.clone(), artifact_dataset_label(art, &csv_path)));
    }
    if uniq_paths.is_empty() {
        return Ok(());
    }

    let mut idx_by_key: HashMap<String, usize> = HashMap::with_capacity(result.groups.len());
    for (i, g) in result.groups.iter().enumerate() {
        idx_by_key.insert(g.key.clone(), i);
    }

    // Group the shortlist by MA config so we can evaluate a batch at once.
    let mut groups_by_cfg: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, g) in result.groups.iter().enumerate() {
        if !is_selected(i, selected_mask.as_ref()) {
            continue;
        }
        let cfg_key = serde_json::to_string(&serde_json::json!({
            "ma_source": &g.ma_source,
            "fast_ma_params": &g.fast_ma_params,
            "slow_ma_params": &g.slow_ma_params,
        }))
        .unwrap_or_else(|_| "cfg".to_string());
        groups_by_cfg.entry(cfg_key).or_default().push(i);
    }
    if groups_by_cfg.is_empty() {
        return Ok(());
    }

    let mut accs: Vec<HoldoutAcc> = vec![HoldoutAcc::default(); result.groups.len()];
    let mut holdout_rows: Vec<Vec<ta_app_core::HoldoutDatasetRow>> = if req.save_holdout_rows {
        vec![Vec::new(); result.groups.len()]
    } else {
        Vec::new()
    };
    let mut skipped = 0usize;

    let total = uniq_paths.len();
    for (i, (csv_path, label)) in uniq_paths.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let candles = match crate::data_cache::load_or_build(Path::new(&csv_path), &cache_dir) {
            Ok(outcome) => outcome.candles,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let mut local: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); result.groups.len()]; // (score, sharpe, max_dd, pnl)
        let mut ok = true;

        for idxs in groups_by_cfg.values() {
            let Some(first) = idxs.first() else { continue };
            let g0 = &result.groups[*first];

            if req.strategy_kind == "DoubleMa"
                || req.strategy_kind == "CompositeTrend"
                || req.strategy_kind == "RegimeHybrid"
                || req.strategy_kind == "BreakoutMomentum"
            {
                let mut candidates: Vec<ta_app_core::DoubleMaValidationParams> = Vec::new();
                candidates.reserve(idxs.len().saturating_mul(3));
                for &gi in idxs {
                    let g = &result.groups[gi];
                    let reps = if !g.representatives.is_empty() {
                        g.representatives.as_slice()
                    } else {
                        std::slice::from_ref(&g.params)
                    };
                    for rv in reps {
                        let p = match serde_json::from_value::<ta_app_core::DoubleMaValidationParams>(
                            rv.clone(),
                        ) {
                            Ok(v) => v,
                            Err(_) => {
                                let o = rv.as_object().ok_or_else(|| {
                                    "double-ma representative is not an object".to_string()
                                })?;
                                let as_u16 = |k: &str, d: u16| -> u16 {
                                    o.get(k)
                                        .and_then(|v| v.as_u64())
                                        .map(|v| v.min(u16::MAX as u64) as u16)
                                        .unwrap_or(d)
                                };
                                ta_app_core::DoubleMaValidationParams {
                                    fast_ma_type: o
                                        .get("fast_ma_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("sma")
                                        .to_string(),
                                    slow_ma_type: o
                                        .get("slow_ma_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("sma")
                                        .to_string(),
                                    fast_len: as_u16("fast_len", 10),
                                    slow_len: as_u16("slow_len", 30),
                                    fast_ma_params: o
                                        .get("fast_ma_params")
                                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                                    slow_ma_params: o
                                        .get("slow_ma_params")
                                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                                    strategy: o
                                        .get("strategy")
                                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                                        .unwrap_or_default(),
                                    halftrend_filter: o
                                        .get("halftrend_filter")
                                        .or_else(|| o.get("halftrend"))
                                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                                }
                            }
                        };
                        candidates.push(p);
                    }
                }

                let max_candidates = candidates.len();
                let val_req = ta_app_core::DoubleMaValidationRequest {
                    data_id: label.clone(),
                    ma_source: g0.ma_source.clone(),
                    fast_ma_params: g0.fast_ma_params.clone(),
                    slow_ma_params: g0.slow_ma_params.clone(),
                    candidates,
                    walk_forward: hv_wf.clone(),
                    perturb: hv_perturb.clone(),
                    trade_bootstrap: hv_mc.clone(),
                    cross_ticker: None,
                    cscv_pbo: None,
                    seed: hv_seed,
                    max_candidates,
                    validation_engine: hv_engine,
                    iterative: hv_iterative.clone(),
                    min_psr: None,
                    min_dsr: None,
                };

                let val_res = match ta_app_core::run_double_ma_validation_blocking_with_candles(
                    val_req, &candles, cancel, None,
                ) {
                    Ok(r) => r,
                    Err(_) => {
                        ok = false;
                        break;
                    }
                };

                for cand in val_res.candidates {
                    let params_v = serde_json::to_value(&cand.params).map_err(|e| e.to_string())?;
                    let key = ta_app_core::meta_group_key_for_request(
                        req,
                        &params_v,
                        &g0.ma_source,
                        &g0.fast_ma_params,
                        &g0.slow_ma_params,
                    );
                    let Some(&gi) = idx_by_key.get(&key) else {
                        continue;
                    };

                    let base_metrics = cand.iterative_in_sample.as_ref().unwrap_or(&cand.in_sample);
                    let (score, sharpe, max_dd, pnl) = if heavy {
                        if hv_engine == ta_app_core::ValidationEngine::Iterative {
                            let sharpe = base_metrics.sharpe;
                            let max_dd = base_metrics.max_dd;
                            let pnl = base_metrics.pnl;
                            (sharpe - 2.0 * max_dd, sharpe, max_dd, pnl)
                        } else {
                            let score = cand.robust_score;
                            if let Some(w) = cand.walk_forward.as_ref() {
                                (score, w.sharpe_median, w.max_dd_worst, w.pnl_median)
                            } else {
                                (
                                    score,
                                    base_metrics.sharpe,
                                    base_metrics.max_dd,
                                    base_metrics.pnl,
                                )
                            }
                        }
                    } else {
                        let sharpe = base_metrics.sharpe;
                        let max_dd = base_metrics.max_dd;
                        let pnl = base_metrics.pnl;
                        (sharpe - 2.0 * max_dd, sharpe, max_dd, pnl)
                    };

                    local[gi].push((score, sharpe, max_dd, pnl));
                }
            } else {
                let mut candidates: Vec<ta_app_core::CompositeParams> = Vec::new();
                candidates.reserve(idxs.len().saturating_mul(3));
                for &gi in idxs {
                    let g = &result.groups[gi];
                    let reps = if !g.representatives.is_empty() {
                        g.representatives.as_slice()
                    } else {
                        std::slice::from_ref(&g.params)
                    };
                    for rv in reps {
                        let p: ta_app_core::CompositeParams =
                            serde_json::from_value(rv.clone()).map_err(|e| e.to_string())?;
                        candidates.push(p);
                    }
                }

                let max_candidates = candidates.len();
                let val_req = ta_app_core::CompositeValidationRequest {
                    data_id: label.clone(),
                    ma_source: g0.ma_source.clone(),
                    fast_ma_params: g0.fast_ma_params.clone(),
                    slow_ma_params: g0.slow_ma_params.clone(),
                    candidates,
                    walk_forward: hv_wf.clone(),
                    perturb: hv_perturb.clone(),
                    trade_bootstrap: hv_mc.clone(),
                    cross_ticker: None,
                    cscv_pbo: None,
                    seed: hv_seed,
                    max_candidates,
                    validation_engine: hv_engine,
                    iterative: hv_iterative.clone(),
                    min_psr: None,
                    min_dsr: None,
                };

                let val_res = match ta_app_core::run_composite_validation_blocking_with_candles(
                    val_req, &candles, cancel, None,
                ) {
                    Ok(r) => r,
                    Err(_) => {
                        ok = false;
                        break;
                    }
                };

                for cand in val_res.candidates {
                    let params_v = serde_json::to_value(&cand.params).map_err(|e| e.to_string())?;
                    let key = ta_app_core::meta_group_key_for_request(
                        req,
                        &params_v,
                        &g0.ma_source,
                        &g0.fast_ma_params,
                        &g0.slow_ma_params,
                    );
                    let Some(&gi) = idx_by_key.get(&key) else {
                        continue;
                    };

                    let base_metrics = cand.iterative_in_sample.as_ref().unwrap_or(&cand.in_sample);
                    let (score, sharpe, max_dd, pnl) = if heavy {
                        if hv_engine == ta_app_core::ValidationEngine::Iterative {
                            let sharpe = base_metrics.sharpe;
                            let max_dd = base_metrics.max_dd;
                            let pnl = base_metrics.pnl;
                            (sharpe - 2.0 * max_dd, sharpe, max_dd, pnl)
                        } else {
                            let score = cand.robust_score;
                            if let Some(w) = cand.walk_forward.as_ref() {
                                (score, w.sharpe_median, w.max_dd_worst, w.pnl_median)
                            } else {
                                (
                                    score,
                                    base_metrics.sharpe,
                                    base_metrics.max_dd,
                                    base_metrics.pnl,
                                )
                            }
                        }
                    } else {
                        let sharpe = base_metrics.sharpe;
                        let max_dd = base_metrics.max_dd;
                        let pnl = base_metrics.pnl;
                        (sharpe - 2.0 * max_dd, sharpe, max_dd, pnl)
                    };

                    local[gi].push((score, sharpe, max_dd, pnl));
                }
            }
        }

        if !ok {
            skipped += 1;
            continue;
        }

        for (gi, rows) in local.into_iter().enumerate() {
            if rows.is_empty() {
                continue;
            }

            let mut scores: Vec<f64> = Vec::with_capacity(rows.len());
            let mut sharpes: Vec<f64> = Vec::with_capacity(rows.len());
            let mut max_dds: Vec<f64> = Vec::with_capacity(rows.len());
            let mut pnls: Vec<f64> = Vec::with_capacity(rows.len());
            for (score, sharpe, max_dd, pnl) in rows {
                scores.push(score);
                sharpes.push(sharpe);
                max_dds.push(max_dd);
                pnls.push(pnl);
            }
            scores.sort_by(|a, b| a.total_cmp(b));
            sharpes.sort_by(|a, b| a.total_cmp(b));
            max_dds.sort_by(|a, b| a.total_cmp(b));
            pnls.sort_by(|a, b| a.total_cmp(b));

            let score = quantile(&scores, 0.5);
            let score_p10 = quantile(&scores, 0.10);
            let sharpe = quantile(&sharpes, 0.5);
            // Conservative: for risk, take worst-of reps.
            let max_dd = max_dds.last().copied().unwrap_or(0.0);
            let pnl = quantile(&pnls, 0.5);
            let pass = score_p10 > 0.0 && sharpe > 0.0;

            accs[gi].scores.push(score);
            accs[gi].sharpes.push(sharpe);
            accs[gi].max_dds.push(max_dd);
            accs[gi].pnls.push(pnl);
            if pass {
                accs[gi].passes = accs[gi].passes.saturating_add(1);
            }

            if req.save_holdout_rows && !holdout_rows.is_empty() {
                holdout_rows[gi].push(ta_app_core::HoldoutDatasetRow {
                    dataset: label.clone(),
                    score_median: score,
                    score_p10,
                    sharpe_median: sharpe,
                    max_dd_worst: max_dd,
                    pnl_median: pnl,
                    pass,
                });
            }
        }

        ta_app_core::progress::emit_double_ma_progress(Some(sink), i + 1, total, "meta_holdout");
    }

    ta_app_core::progress::emit_double_ma_progress(Some(sink), total, total, "meta_holdout");

    let datasets = total.saturating_sub(skipped);
    if datasets == 0 {
        return Ok(());
    }

    result.holdout = Some(ta_app_core::HoldoutSummary { datasets, skipped });

    for (i, g) in result.groups.iter_mut().enumerate() {
        let mut acc = accs[i].clone();
        if acc.scores.is_empty() {
            continue;
        }
        acc.scores.sort_by(|a, b| a.total_cmp(b));
        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.max_dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));

        let pass_rate = (acc.passes as f64) / (datasets as f64);
        let score_median = quantile(&acc.scores, 0.5);
        let score_p10 = quantile(&acc.scores, 0.10);
        let sharpe_median = quantile(&acc.sharpes, 0.5);
        let max_dd_p90 = quantile(&acc.max_dds, 0.90);
        let pnl_median = quantile(&acc.pnls, 0.5);

        g.holdout = Some(ta_app_core::HoldoutAggregate {
            datasets,
            pass_rate,
            score_median,
            score_p10,
            sharpe_median,
            max_dd_p90,
            pnl_median,
        });

        if req.save_holdout_rows && !holdout_rows.is_empty() {
            g.holdout_rows = std::mem::take(&mut holdout_rows[i]);
        }
    }

    for g in &mut result.groups {
        let base = g.meta_score;
        let final_score = if let Some(h) = g.holdout.as_ref() {
            base + req.weights.w_holdout_pass_rate * h.pass_rate
                + req.weights.w_holdout_p10 * h.score_p10
                - req.weights.w_holdout_dd_p90 * h.max_dd_p90
        } else {
            base
        };
        g.final_score = Some(final_score);
    }

    result.groups.sort_by(|a, b| {
        let a_eval = a.holdout.is_some();
        let b_eval = b.holdout.is_some();
        if a_eval != b_eval {
            return b_eval.cmp(&a_eval);
        }
        let ascore = a.final_score.unwrap_or(a.meta_score);
        let bscore = b.final_score.unwrap_or(b.meta_score);
        bscore
            .total_cmp(&ascore)
            .then_with(|| b.meta_score.total_cmp(&a.meta_score))
    });

    if skipped > 0 {
        result.warnings.push(format!(
            "Holdout evaluation: skipped {skipped} dataset(s) (missing/invalid CSV path or parse error) for tag='{holdout_tag}'."
        ));
    }
    result.warnings.push(
        "Final group ordering uses holdout-adjusted score when holdout is available.".to_string(),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        should_block_stage4_for_unhealthy_model, stage4_autotrain_failure_fallback,
        stage4_training_metas_excluding_source,
    };
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn stage4_training_set_excludes_source_meta_artifact() {
        let source = PathBuf::from("runs/vectorbt_meta_source.json");
        let other = PathBuf::from("runs/vectorbt_meta_other.json");
        let metas = vec![
            (
                source.clone(),
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:00:00Z"}),
            ),
            (
                other,
                json!({"kind":"vectorbt_meta_selection","ts":"2026-01-01T00:01:00Z"}),
            ),
        ];

        let (filtered, excluded) = stage4_training_metas_excluding_source(metas, &source);
        assert_eq!(excluded, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].get("ts").and_then(|v| v.as_str()),
            Some("2026-01-01T00:01:00Z")
        );
    }

    #[test]
    fn stage4_health_gate_blocks_only_explicit_unhealthy_models() {
        assert!(should_block_stage4_for_unhealthy_model(true, Some(false)));
        assert!(!should_block_stage4_for_unhealthy_model(true, Some(true)));
        assert!(!should_block_stage4_for_unhealthy_model(true, None));
        assert!(!should_block_stage4_for_unhealthy_model(false, Some(false)));
    }

    #[test]
    fn stage4_autotrain_failure_falls_back_to_existing_model() {
        let mut warnings: Vec<String> = Vec::new();
        let model_path = PathBuf::from("models/confidence/ml_confidence_test_1m.json");
        let out = stage4_autotrain_failure_fallback(
            "simulated training error".to_string(),
            &model_path,
            true,
            &mut warnings,
        );

        assert!(out.is_ok());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("falling back to existing model"));
    }

    #[test]
    fn stage4_autotrain_failure_errors_without_existing_model() {
        let mut warnings: Vec<String> = Vec::new();
        let model_path = PathBuf::from("models/confidence/ml_confidence_test_1m.json");
        let err = stage4_autotrain_failure_fallback(
            "simulated training error".to_string(),
            &model_path,
            false,
            &mut warnings,
        )
        .unwrap_err();

        assert!(warnings.is_empty());
        assert!(err.contains("no usable existing model"));
    }
}
