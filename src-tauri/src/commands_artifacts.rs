use super::run_artifacts::{
    self, MetaArtifactSummary, MlPolicyEvalArtifactSummary, RunArtifactSummary, SaveBatchSummaryResponse,
    SaveRunArtifactResponse, Stage4ArtifactSummary, Stage5ArtifactSummary,
};
use serde_json::Value;

#[tauri::command]
pub async fn save_run_artifact(
    artifact: Value,
    app: tauri::AppHandle,
) -> Result<SaveRunArtifactResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::save_run_artifact_blocking(artifact, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_batch_summary(
    summary: Value,
    app: tauri::AppHandle,
) -> Result<SaveBatchSummaryResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::save_batch_summary_blocking(summary, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_run_artifacts(app: tauri::AppHandle) -> Result<Vec<RunArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::list_run_artifacts_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_meta_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<MetaArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::list_meta_artifacts_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_artifact_tags(
    path: String,
    tags: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::update_artifact_tags_blocking(path, tags, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_stage4_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<Stage4ArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::list_stage4_artifacts_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_stage5_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<Stage5ArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::list_stage5_artifacts_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_ml_policy_eval_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<MlPolicyEvalArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::list_ml_policy_eval_artifacts_blocking(app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_run_artifact(path: String, app: tauri::AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_run_artifact_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_run_artifact(path: String, app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::delete_run_artifact_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_runs_dir(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::open_runs_dir_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}
