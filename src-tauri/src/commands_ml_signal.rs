use super::validation_common;
use crate::state::{AppState, LoadedDataContext};
use serde::Serialize;
use std::path::Component;
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
        let expected = validation_common::normalize_path_key(expected_csv_path);
        let actual = validation_common::normalize_path_key(&loaded.csv_path);
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

fn ml_signal_models_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    Ok(base.join("models").join("ml_signal"))
}

fn validate_bundle_id(bundle_id: &str) -> Result<String, String> {
    let trimmed = bundle_id.trim();
    if trimmed.is_empty() {
        return Err("bundle_id is required".to_string());
    }

    let mut components = Path::new(trimmed).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(component)), None)
            if Path::new(trimmed).as_os_str() == component =>
        {
            Ok(trimmed.to_string())
        }
        _ => Err("bundle_id must be exactly one normal path segment".to_string()),
    }
}

fn bundle_manifest_path(models_dir: &Path, bundle_id: &str) -> Result<PathBuf, String> {
    let bundle_id = validate_bundle_id(bundle_id)?;
    Ok(models_dir.join(bundle_id).join("manifest.json"))
}

fn missing_bundle_status(
    models_dir: &Path,
    manifest_path: &Path,
    bundle_id: String,
) -> MlSignalModelStatusResponse {
    MlSignalModelStatusResponse {
        models_dir: models_dir.to_string_lossy().to_string(),
        status: MlSignalModelBundleStatus {
            bundle_id,
            manifest_path: manifest_path.to_string_lossy().to_string(),
            state: "missing".to_string(),
            manifest: None,
            issues: vec!["manifest missing".to_string()],
        },
    }
}

fn classify_bundle_status_from_manifest(
    models_dir: &Path,
    manifest_path: &Path,
    bundle_id: String,
) -> Result<MlSignalModelStatusResponse, String> {
    match std::fs::File::open(manifest_path) {
        Ok(_) => Ok(MlSignalModelStatusResponse {
            models_dir: models_dir.to_string_lossy().to_string(),
            status: classify_ml_signal_manifest(manifest_path)?,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(missing_bundle_status(models_dir, manifest_path, bundle_id))
        }
        Err(error) => Err(format!(
            "open ml_signal manifest {}: {error}",
            manifest_path.display()
        )),
    }
}

fn has_nonempty_optional_text(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn reject_renderer_supplied_score_cache_paths(
    req: &MlSignalScoreCacheBuildRequest,
) -> Result<(), String> {
    if has_nonempty_optional_text(req.source_bundle_manifest_path.as_deref()) {
        return Err(
            "source_bundle_manifest_path is resolved by the backend from model_bundle_id."
                .to_string(),
        );
    }
    if has_nonempty_optional_text(req.native_library_path.as_deref()) {
        return Err(
            "native_library_path is resolved by the backend from the model bundle or packaged resources."
                .to_string(),
        );
    }
    Ok(())
}

fn normalize_ml_signal_output_path(
    raw: &str,
    expected_ext: &str,
    field_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    let path = PathBuf::from(trimmed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .unwrap_or_default();
    if file_name.is_empty() {
        return Err(format!(
            "{field_name} must point to a file: {}",
            path.to_string_lossy()
        ));
    }
    match path.extension().and_then(|value| value.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case(expected_ext) => Ok(path),
        _ => Err(format!(
            "{field_name} must use .{expected_ext} extension: {}",
            path.to_string_lossy()
        )),
    }
}

fn validate_ml_signal_dataset_output_path(raw: &str, state: &AppState) -> Result<String, String> {
    let path = normalize_ml_signal_output_path(raw, "csv", "output_path")?;
    state.consume_issued_csv_save_path(&path).map_err(|_| {
        "ML Signal dataset output path must be selected with the native CSV save dialog."
            .to_string()
    })?;
    Ok(path.to_string_lossy().to_string())
}

fn validate_ml_signal_score_cache_output_path(
    raw: &str,
    state: &AppState,
) -> Result<String, String> {
    let path = normalize_ml_signal_output_path(raw, "json", "output_path")?;
    state.consume_issued_json_save_path(&path).map_err(|_| {
        "ML Signal score-cache output path must be selected with the native JSON save dialog."
            .to_string()
    })?;
    Ok(path.to_string_lossy().to_string())
}

fn resolve_bundle_relative_path(
    manifest_path: &Path,
    raw: &str,
    field_name: &str,
) -> Result<PathBuf, String> {
    let candidate = Path::new(raw.trim());
    if candidate.is_absolute() {
        return Err(format!(
            "{field_name} must be relative to the ML model bundle: {}",
            candidate.display()
        ));
    }
    if candidate.components().any(|component| {
        !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Err(format!(
            "{field_name} must stay within the ML model bundle: {}",
            candidate.display()
        ));
    }
    Ok(manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(candidate))
}

#[cfg(target_os = "windows")]
const PACKAGED_ML_SIGNAL_NATIVE_LIBRARY_REL_PATH: &str = "native/catboostmodel.dll";
#[cfg(target_os = "linux")]
const PACKAGED_ML_SIGNAL_NATIVE_LIBRARY_REL_PATH: &str = "native/libcatboostmodel.so";
#[cfg(target_os = "macos")]
const PACKAGED_ML_SIGNAL_NATIVE_LIBRARY_REL_PATH: &str = "native/libcatboostmodel.dylib";

fn packaged_ml_signal_native_library_candidate(resource_dir: &Path) -> Option<String> {
    let candidate = resource_dir.join(PACKAGED_ML_SIGNAL_NATIVE_LIBRARY_REL_PATH);
    candidate
        .is_file()
        .then(|| candidate.to_string_lossy().to_string())
}

fn enrich_score_cache_request_from_source_manifest(
    req: &mut MlSignalScoreCacheBuildRequest,
    manifest_path: &Path,
) -> Result<(), String> {
    let manifest = read_ml_signal_manifest(manifest_path).map_err(|error| {
        format!(
            "read source bundle manifest {}: {error}",
            manifest_path.display()
        )
    })?;

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
    if req
        .native_library_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        if let Some(candidate) = manifest
            .native_library_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| resolve_bundle_relative_path(manifest_path, value, "native_library_path"))
            .transpose()?
            .filter(|value| value.is_file())
        {
            req.native_library_path = Some(candidate.to_string_lossy().to_string());
        }
    }

    Ok(())
}

fn packaged_ml_signal_native_library_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<String> {
    app.path()
        .resource_dir()
        .ok()
        .and_then(|resource_dir| packaged_ml_signal_native_library_candidate(&resource_dir))
}

#[tauri::command]
pub async fn export_ml_signal_dataset(
    mut req: MlSignalDatasetExportRequest,
    state: tauri::State<'_, AppState>,
) -> Result<MlSignalDatasetExportResult, String> {
    let _run_guard = state.try_begin_run()?;
    req.output_path = validate_ml_signal_dataset_output_path(&req.output_path, &state)?;
    let cancel = state.cancel_token();
    let candles = state.get_candles_for_run(&req.data_id)?;

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
    reject_renderer_supplied_score_cache_paths(&req)?;
    req.output_path = validate_ml_signal_score_cache_output_path(&req.output_path, &state)?;
    let cancel = state.cancel_token();
    let candles = state.get_candles_for_run(&req.data_id)?;
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
            let manifest_path = bundle_manifest_path(&models_dir, bundle_id)?;
            if manifest_path.is_file() {
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
        .map(str::to_string)
    {
        enrich_score_cache_request_from_source_manifest(&mut req, Path::new(&manifest_path))?;
    }
    if req
        .native_library_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        req.native_library_path = packaged_ml_signal_native_library_path(&app);
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
    let candles = state.get_candles_for_run(&req.data_id)?;

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
    let candles = state.get_candles_for_run(&req.data_id)?;

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
    list_ml_signal_model_bundles_with_app(app).await
}

async fn list_ml_signal_model_bundles_with_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
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
    ml_signal_model_status_with_app(bundle_id, app).await
}

async fn ml_signal_model_status_with_app<R: tauri::Runtime>(
    bundle_id: String,
    app: tauri::AppHandle<R>,
) -> Result<MlSignalModelStatusResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bundle_id = validate_bundle_id(&bundle_id)?;
        let models_dir = ml_signal_models_dir(&app)?;
        let manifest_path = bundle_manifest_path(&models_dir, &bundle_id)?;
        classify_bundle_status_from_manifest(&models_dir, &manifest_path, bundle_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BackgroundTaskState;
    use std::path::Path;
    use tauri::test::{mock_builder, mock_context, noop_assets};

    fn temp_dir_path(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("ta_ml_signal_{name}_{nonce}"))
    }

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        mock_builder()
            .manage(AppState::default())
            .manage(BackgroundTaskState::default())
            .build(mock_context(noop_assets()))
            .expect("failed to build tauri mock app")
    }

    #[test]
    fn packaged_ml_signal_native_library_candidate_prefers_resource_native_dir() {
        let root = temp_dir_path("packaged_native_candidate");
        let native_dir = root.join("native");
        std::fs::create_dir_all(&native_dir).expect("create native dir");
        let library_path = native_dir.join(
            Path::new(PACKAGED_ML_SIGNAL_NATIVE_LIBRARY_REL_PATH)
                .file_name()
                .expect("library file name"),
        );
        std::fs::write(&library_path, b"dll").expect("write native library");

        let resolved = packaged_ml_signal_native_library_candidate(&root).expect("resolve");
        assert_eq!(Path::new(&resolved), library_path.as_path());

        let _ = std::fs::remove_file(&library_path);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn packaged_ml_signal_native_library_candidate_ignores_directories() {
        let root = temp_dir_path("packaged_native_directory");
        let library_dir = root.join(Path::new(PACKAGED_ML_SIGNAL_NATIVE_LIBRARY_REL_PATH));
        std::fs::create_dir_all(&library_dir).expect("create directory-shaped candidate");

        let resolved = packaged_ml_signal_native_library_candidate(&root);
        assert!(resolved.is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_bundle_id_rejects_non_normal_path_shapes() {
        assert_eq!(validate_bundle_id("bundle-a").as_deref(), Ok("bundle-a"));
        assert!(validate_bundle_id("").is_err());
        assert!(validate_bundle_id("bundle-a/nested").is_err());
        assert!(validate_bundle_id("../bundle-a").is_err());
        assert!(validate_bundle_id("bundle-a/").is_err());
    }

    #[test]
    fn validate_ml_signal_context_rejects_path_and_timeframe_mismatches() {
        let loaded = LoadedDataContext {
            csv_path: "C:/data/loaded.csv".to_string(),
            label: "loaded".to_string(),
            timeframe: Some("1h".to_string()),
            source_identity: None,
        };
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "MlSignal".to_string(),
            csv_path: Some("C:/data/expected.csv".to_string()),
            timeframe: Some("4h".to_string()),
            label: Some("expected dataset".to_string()),
            optimization_trials: None,
            search_distribution: None,
            data_realism: None,
        };

        let err = validate_ml_signal_context(Some(&ctx), &loaded).expect_err("mismatch");
        assert!(err.contains("expected dataset"));
        assert!(err.contains("C:/data/expected.csv"));

        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "MlSignal".to_string(),
            csv_path: Some("C:/data/loaded.csv".to_string()),
            timeframe: Some("4h".to_string()),
            label: None,
            optimization_trials: None,
            search_distribution: None,
            data_realism: None,
        };
        let err = validate_ml_signal_context(Some(&ctx), &loaded).expect_err("timeframe mismatch");
        assert!(err.contains("timeframe"));
        assert!(err.contains("4h"));
        assert!(err.contains("1h"));
    }

    #[test]
    fn validate_ml_signal_context_allows_unknown_loaded_timeframe() {
        let loaded = LoadedDataContext {
            csv_path: "C:/data/loaded.csv".to_string(),
            label: "loaded".to_string(),
            timeframe: None,
            source_identity: None,
        };
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "MlSignal".to_string(),
            csv_path: Some("C:/data/loaded.csv".to_string()),
            timeframe: Some("4h".to_string()),
            label: None,
            optimization_trials: None,
            search_distribution: None,
            data_realism: None,
        };

        validate_ml_signal_context(Some(&ctx), &loaded).expect("unknown timeframe should bypass");
    }

    #[test]
    fn ml_signal_output_path_requires_expected_extension() {
        let csv = normalize_ml_signal_output_path(
            "C:/exports/ml_signal_dataset.csv",
            "csv",
            "output_path",
        )
        .expect("csv output path");
        assert_eq!(csv, PathBuf::from("C:/exports/ml_signal_dataset.csv"));

        let err = normalize_ml_signal_output_path(
            "C:/exports/ml_signal_score_cache.csv",
            "json",
            "output_path",
        )
        .expect_err("score cache must be json");
        assert!(err.contains(".json"));
    }

    #[test]
    fn ml_signal_dataset_output_requires_native_csv_save_token() {
        let state = AppState::default();
        let path = temp_dir_path("dataset_save_token").join("ml_dataset.csv");
        let raw = path.to_string_lossy().to_string();

        let err = validate_ml_signal_dataset_output_path(&raw, &state)
            .expect_err("unissued path should fail");
        assert!(err.contains("native CSV save dialog"));

        state
            .record_issued_csv_save_path(path.clone())
            .expect("record issued csv save path");
        assert_eq!(
            validate_ml_signal_dataset_output_path(&raw, &state).expect("issued path accepted"),
            raw
        );
        validate_ml_signal_dataset_output_path(&raw, &state)
            .expect_err("token should be single use");
    }

    #[test]
    fn ml_signal_score_cache_output_requires_native_json_save_token() {
        let state = AppState::default();
        let path = temp_dir_path("score_cache_save_token").join("ml_score_cache.json");
        let raw = path.to_string_lossy().to_string();

        let err = validate_ml_signal_score_cache_output_path(&raw, &state)
            .expect_err("unissued path should fail");
        assert!(err.contains("native JSON save dialog"));

        state
            .record_issued_json_save_path(path.clone())
            .expect("record issued json save path");
        assert_eq!(
            validate_ml_signal_score_cache_output_path(&raw, &state).expect("issued path accepted"),
            raw
        );
        validate_ml_signal_score_cache_output_path(&raw, &state)
            .expect_err("token should be single use");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_ml_signal_model_bundles_does_not_create_missing_models_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let models_dir = ml_signal_models_dir(&handle).expect("models dir");
        let _ = std::fs::remove_dir_all(&models_dir);

        let bundles = list_ml_signal_model_bundles_with_app(handle.clone())
            .await
            .expect("list bundles");

        assert!(bundles.is_empty());
        assert!(!models_dir.exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ml_signal_model_status_reports_missing_without_creating_models_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let models_dir = ml_signal_models_dir(&handle).expect("models dir");
        let _ = std::fs::remove_dir_all(&models_dir);

        let status = ml_signal_model_status_with_app("bundle-a".to_string(), handle.clone())
            .await
            .expect("missing bundle status");

        assert_eq!(status.status.bundle_id, "bundle-a");
        assert_eq!(status.status.state, "missing");
        assert!(status
            .status
            .issues
            .iter()
            .any(|issue| issue.contains("manifest missing")));
        assert!(!models_dir.exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ml_signal_model_status_rejects_invalid_bundle_id_shapes() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let err = ml_signal_model_status_with_app("../bundle-a".to_string(), handle)
            .await
            .expect_err("invalid bundle id");
        assert!(err.contains("bundle_id must be exactly one normal path segment"));
    }

    fn score_cache_request_template() -> MlSignalScoreCacheBuildRequest {
        MlSignalScoreCacheBuildRequest {
            data_id: "data-1".to_string(),
            input_path: String::new(),
            output_path: "C:/tmp/score_cache.json".to_string(),
            model_bundle_id: Some("bundle-a".to_string()),
            feature_schema_path: None,
            feature_schema_hash: None,
            export_manifest_path: None,
            source_bundle_manifest_path: None,
            calibration_state: None,
            threshold_policy: None,
            producer_kind: None,
            score_contract_version: None,
            score_scope: None,
            allow_non_walkforward_score_scope: false,
            native_backend_preference:
                ta_app_core::ml_signal::MlSignalInferenceBackendPreference::NativeCpu,
            native_library_path: None,
            native_gpu_device_id: None,
            native_prediction_batch_size: None,
            native_library_discovery_policy:
                ta_app_core::ml_signal::MlSignalNativeLibraryDiscoveryPolicy::Production,
        }
    }

    #[test]
    fn score_cache_command_rejects_renderer_supplied_source_manifest_path() {
        let mut req = score_cache_request_template();
        req.source_bundle_manifest_path = Some("C:/models/bundle-a/manifest.json".to_string());

        let err = reject_renderer_supplied_score_cache_paths(&req)
            .expect_err("renderer manifest path must be rejected");

        assert!(err.contains("source_bundle_manifest_path"));
        assert!(err.contains("model_bundle_id"));
    }

    #[test]
    fn score_cache_command_rejects_renderer_supplied_native_library_path() {
        let mut req = score_cache_request_template();
        req.native_library_path = Some("C:/catboost/catboostmodel.dll".to_string());

        let err = reject_renderer_supplied_score_cache_paths(&req)
            .expect_err("renderer native path must be rejected");

        assert!(err.contains("native_library_path"));
        assert!(err.contains("model bundle"));
    }

    #[test]
    fn enrich_score_cache_request_from_source_manifest_errors_on_invalid_manifest() {
        let dir = temp_dir_path("invalid_source_manifest");
        std::fs::create_dir_all(&dir).expect("create manifest dir");
        let manifest_path = dir.join("manifest.json");
        std::fs::write(&manifest_path, b"{not-json").expect("write invalid manifest");
        let mut req = score_cache_request_template();

        let err = enrich_score_cache_request_from_source_manifest(&mut req, &manifest_path)
            .expect_err("invalid manifest should error");
        assert!(err.contains("read source bundle manifest"));
        assert!(err.contains("manifest.json"));

        let _ = std::fs::remove_file(&manifest_path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn enrich_score_cache_request_from_source_manifest_rejects_absolute_native_library() {
        let dir = temp_dir_path("absolute_native_library");
        let bundle_dir = dir.join("bundle-a");
        std::fs::create_dir_all(&bundle_dir).expect("create bundle dir");
        let manifest_path = bundle_dir.join("manifest.json");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "bundle_id": "bundle-a",
                "schema_version": 1,
                "feature_schema_version": 1,
                "model_family": "catboost_binary_pair",
                "model_format": "cbm",
                "long_model_path": "model_long.cbm",
                "short_model_path": "model_short.cbm",
                "native_library_path": "C:/catboost/catboostmodel.dll"
            }))
            .expect("serialize manifest"),
        )
        .expect("write manifest");
        let mut req = score_cache_request_template();

        let err = enrich_score_cache_request_from_source_manifest(&mut req, &manifest_path)
            .expect_err("absolute native library path should be rejected");
        assert!(err.contains("native_library_path"));
        assert!(err.contains("relative"));

        let _ = std::fs::remove_file(&manifest_path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn enrich_score_cache_request_from_source_manifest_ignores_directory_native_library() {
        let dir = temp_dir_path("directory_native_library");
        let bundle_dir = dir.join("bundle-a");
        let native_dir = bundle_dir.join("native").join("catboostmodel.dll");
        std::fs::create_dir_all(&native_dir).expect("create directory-shaped native candidate");
        let manifest_path = bundle_dir.join("manifest.json");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "bundle_id": "bundle-a",
                "schema_version": 1,
                "feature_schema_version": 1,
                "model_family": "catboost_binary_pair",
                "model_format": "cbm",
                "long_model_path": "model_long.cbm",
                "short_model_path": "model_short.cbm",
                "native_library_path": "native/catboostmodel.dll"
            }))
            .expect("serialize manifest"),
        )
        .expect("write manifest");
        let mut req = score_cache_request_template();

        enrich_score_cache_request_from_source_manifest(&mut req, &manifest_path)
            .expect("manifest enrichment");
        assert!(req.native_library_path.is_none());

        let _ = std::fs::remove_file(&manifest_path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
