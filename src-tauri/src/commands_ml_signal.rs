use crate::state::{AppState, LoadedDataContext};
use serde::Serialize;
use std::path::{Path, PathBuf};
use ta_app_core::{
    build_ml_signal_score_cache_from_csv, classify_ml_signal_manifest,
    export_ml_signal_dataset_with_candles, list_ml_signal_model_bundle_summaries,
    read_ml_signal_manifest, run_ml_signal_policy_sweep_blocking_with_candles,
    run_ml_signal_validation_blocking_with_candles, MlSignalDatasetExportRequest,
    MlSignalDatasetExportResult, MlSignalModelBundleStatus, MlSignalModelBundleSummary,
    MlSignalPolicySweepRequest, MlSignalPolicySweepResult, MlSignalScoreCacheBuildRequest,
    MlSignalScoreCacheBuildResult, MlSignalValidationRequest, MlSignalValidationResult,
};
use tauri::Manager;

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalModelStatusResponse {
    pub models_dir: String,
    pub status: MlSignalModelBundleStatus,
}

fn normalize_timeframe_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    ta_app_core::parse_timeframe_label(trimmed)
        .map(|tf| tf.label)
        .unwrap_or_else(|_| trimmed.to_ascii_lowercase())
}

fn normalize_path_key(raw: &str) -> String {
    raw.trim().replace('/', "\\").to_ascii_lowercase()
}

fn validate_ml_signal_context(
    ctx: Option<&ta_app_core::ValidationArtifactContext>,
    loaded: &LoadedDataContext,
) -> Result<(), String> {
    let Some(ctx) = ctx else {
        return Ok(());
    };

    if ctx.strategy_kind.trim() != "MlSignal" {
        return Err(format!(
            "validation strategy_kind '{}' is incompatible with ml_signal validation",
            ctx.strategy_kind.trim()
        ));
    }

    if let Some(expected_csv_path) = ctx.csv_path.as_deref() {
        let expected = normalize_path_key(expected_csv_path);
        let actual = normalize_path_key(&loaded.csv_path);
        if !expected.is_empty() && !actual.is_empty() && expected != actual {
            let label = ctx
                .label
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(expected_csv_path.trim());
            return Err(format!(
                "validation requires dataset '{label}' ({expected_csv_path}), but loaded data id points to '{}'",
                loaded.csv_path
            ));
        }
    }

    if let Some(expected_timeframe) = ctx.timeframe.as_deref() {
        let expected = normalize_timeframe_label(expected_timeframe);
        let actual = loaded
            .timeframe
            .as_deref()
            .map(normalize_timeframe_label)
            .unwrap_or_default();
        if !expected.is_empty() && !actual.is_empty() && expected != actual {
            return Err(format!(
                "validation requires timeframe '{expected}', but loaded data id has timeframe '{actual}'"
            ));
        }
    }

    Ok(())
}

fn ml_signal_models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("models").join("ml_signal");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn bundle_manifest_path(models_dir: &Path, bundle_id: &str) -> PathBuf {
    models_dir.join(bundle_id.trim()).join("manifest.json")
}

#[tauri::command]
pub async fn export_ml_signal_dataset(
    req: MlSignalDatasetExportRequest,
    state: tauri::State<'_, AppState>,
) -> Result<MlSignalDatasetExportResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    tauri::async_runtime::spawn_blocking(move || {
        export_ml_signal_dataset_with_candles(req, &candles, cancel.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn generate_ml_signal_score_cache(
    mut req: MlSignalScoreCacheBuildRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<MlSignalScoreCacheBuildResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;
    if req
        .source_bundle_manifest_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        if let Some(bundle_id) = req
            .model_bundle_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let models_dir = ml_signal_models_dir(&app)?;
            let manifest_path = bundle_manifest_path(&models_dir, bundle_id);
            if manifest_path.exists() {
                req.source_bundle_manifest_path = Some(manifest_path.to_string_lossy().to_string());
                let bundle_dir = manifest_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(models_dir.clone());
                if req
                    .feature_schema_path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
                {
                    let candidate = bundle_dir.join("feature_schema.json");
                    if candidate.exists() {
                        req.feature_schema_path = Some(candidate.to_string_lossy().to_string());
                    }
                }
                if req
                    .export_manifest_path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
                {
                    let candidate = bundle_dir.join("export_manifest.json");
                    if candidate.exists() {
                        req.export_manifest_path = Some(candidate.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    if let Some(manifest_path) = req
        .source_bundle_manifest_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Ok(manifest) = read_ml_signal_manifest(Path::new(manifest_path)) {
            if req.score_contract_version.is_none() {
                req.score_contract_version = manifest
                    .score_contract
                    .as_ref()
                    .and_then(|value| value.as_object())
                    .and_then(|row| {
                        row.get("score_contract_version")
                            .or_else(|| row.get("scoreContractVersion"))
                    })
                    .and_then(|value| value.as_u64())
                    .map(|value| value as u32);
            }
            if req.calibration_state.is_none() {
                req.calibration_state = manifest
                    .calibration_summary
                    .as_ref()
                    .and_then(|value| value.as_object())
                    .and_then(|row| {
                        row.get("calibration_state")
                            .or_else(|| row.get("calibrationState"))
                            .or_else(|| row.get("status"))
                            .or_else(|| row.get("kind"))
                    })
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        manifest
                            .calibration_path
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(|_| "calibration_attached".to_string())
                    });
            }
            if req.producer_kind.is_none() {
                let model_family = manifest.model_family.trim();
                if !model_family.is_empty() {
                    req.producer_kind = Some(model_family.to_string());
                }
            }
            if req.feature_schema_hash.is_none() {
                req.feature_schema_hash = manifest
                    .source_feature_schema_hash
                    .as_ref()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
            }
            if req.threshold_policy.is_none() {
                req.threshold_policy = manifest
                    .threshold_policy_default
                    .as_ref()
                    .and_then(|value| value.as_object())
                    .and_then(|row| {
                        row.get("kind")
                            .or_else(|| row.get("threshold_policy"))
                            .or_else(|| row.get("thresholdPolicy"))
                    })
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
            }
        }
    }

    tauri::async_runtime::spawn_blocking(move || {
        build_ml_signal_score_cache_from_csv(req, &candles, cancel.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_ml_signal_validation(
    req: MlSignalValidationRequest,
    state: tauri::State<'_, AppState>,
) -> Result<MlSignalValidationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let loaded = state.get_loaded_data_context(&req.data_id)?;
    validate_ml_signal_context(req.validation_context.as_ref(), &loaded)?;
    let candles = state.get_candles(&req.data_id)?;

    tauri::async_runtime::spawn_blocking(move || {
        run_ml_signal_validation_blocking_with_candles(req, &candles, cancel.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_ml_signal_policy_sweep(
    req: MlSignalPolicySweepRequest,
    state: tauri::State<'_, AppState>,
) -> Result<MlSignalPolicySweepResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let loaded = state.get_loaded_data_context(&req.data_id)?;
    validate_ml_signal_context(req.validation_context.as_ref(), &loaded)?;
    let candles = state.get_candles(&req.data_id)?;

    tauri::async_runtime::spawn_blocking(move || {
        run_ml_signal_policy_sweep_blocking_with_candles(req, &candles, cancel.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_ml_signal_model_bundles(
    app: tauri::AppHandle,
) -> Result<Vec<MlSignalModelBundleSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = ml_signal_models_dir(&app)?;
        list_ml_signal_model_bundle_summaries(&dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ml_signal_model_status(
    bundle_id: String,
    app: tauri::AppHandle,
) -> Result<MlSignalModelStatusResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bundle_id = bundle_id.trim().to_string();
        if bundle_id.is_empty() {
            return Err("bundle_id is required".to_string());
        }
        let models_dir = ml_signal_models_dir(&app)?;
        let manifest_path = bundle_manifest_path(&models_dir, &bundle_id);
        if !manifest_path.exists() {
            return Ok(MlSignalModelStatusResponse {
                models_dir: models_dir.to_string_lossy().to_string(),
                status: MlSignalModelBundleStatus {
                    bundle_id,
                    manifest_path: manifest_path.to_string_lossy().to_string(),
                    state: "missing".to_string(),
                    manifest: None,
                    issues: vec!["manifest missing".to_string()],
                },
            });
        }
        Ok(MlSignalModelStatusResponse {
            models_dir: models_dir.to_string_lossy().to_string(),
            status: classify_ml_signal_manifest(&manifest_path)?,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
