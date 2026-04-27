use super::run_artifacts::{
    self, ArtifactLibrarySnapshot, LatestRunArtifactHydrationResponse,
    LiveSessionArtifactDetailResponse, LiveSessionArtifactSummary, MetaArtifactReplayResponse,
    MetaArtifactSummary, MlPolicyEvalArtifactReplayResponse, MlPolicyEvalArtifactSummary,
    RunArtifactDetailResponse, RunArtifactRepairResponse, RunArtifactSummary,
    SaveBatchSummaryResponse, SaveRunArtifactResponse, Stage4ArtifactSummary,
    Stage5ArtifactReplayResponse, Stage5ArtifactSummary, WorkflowArtifactsHydrationPayload,
};
use serde_json::Value;

#[tauri::command]
pub async fn save_run_artifact(
    artifact: Value,
    existing_path: Option<String>,
    app: tauri::AppHandle,
) -> Result<SaveRunArtifactResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::save_run_artifact_blocking(artifact, app, existing_path)
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
pub async fn list_live_session_artifacts(
    app: tauri::AppHandle,
) -> Result<Vec<LiveSessionArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::list_live_session_artifacts_blocking(app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_latest_run_artifact_summary(
    app: tauri::AppHandle,
) -> Result<Option<RunArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_latest_run_artifact_summary_blocking(app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_latest_run_artifact_hydration(
    app: tauri::AppHandle,
) -> Result<Option<LatestRunArtifactHydrationResponse>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_latest_run_artifact_hydration_blocking(app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_artifact_library(
    app: tauri::AppHandle,
) -> Result<ArtifactLibrarySnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::list_artifact_library_blocking(app))
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
pub async fn load_artifact(path: String, app: tauri::AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_run_artifact_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_run_artifact_hydration(
    path: String,
    app: tauri::AppHandle,
) -> Result<WorkflowArtifactsHydrationPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_run_artifact_hydration_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_run_artifact_detail(
    path: String,
    app: tauri::AppHandle,
) -> Result<RunArtifactDetailResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_run_artifact_detail_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn repair_run_artifact(
    path: String,
    dry_run: bool,
    write_target: Option<String>,
    app: tauri::AppHandle,
) -> Result<RunArtifactRepairResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::repair_run_artifact_blocking(path, dry_run, write_target, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_live_session_artifact_detail(
    path: String,
    app: tauri::AppHandle,
) -> Result<LiveSessionArtifactDetailResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_live_session_artifact_detail_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_meta_artifact_replay(
    path: String,
    app: tauri::AppHandle,
) -> Result<MetaArtifactReplayResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_meta_artifact_replay_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_stage5_artifact_replay(
    path: String,
    app: tauri::AppHandle,
) -> Result<Stage5ArtifactReplayResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_stage5_artifact_replay_blocking(path, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_ml_policy_eval_artifact_replay(
    path: String,
    app: tauri::AppHandle,
) -> Result<MlPolicyEvalArtifactReplayResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_artifacts::load_ml_policy_eval_artifact_replay_blocking(path, app)
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
pub async fn delete_artifact(path: String, app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || run_artifacts::delete_artifact_blocking(path, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_runs_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = run_artifacts::runs_dir(&app)?;
    let dir_string = dir.to_string_lossy().to_string();
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(dir_string.clone(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(dir_string)
}
