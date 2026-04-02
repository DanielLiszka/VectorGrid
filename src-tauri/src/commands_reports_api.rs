use super::commands_reports;
use serde::Serialize;
use serde_json::Value;
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct SaveReportXlsxResponse {
    pub path: String,
    pub bytes: usize,
}

#[tauri::command]
pub async fn build_report_xlsx(report: Value, max_rows: Option<u32>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::build_report_xlsx_blocking(report, max_rows)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn build_validation_shortlist_csv(validation_res: Value) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::build_validation_shortlist_csv_blocking(validation_res)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_report_xlsx(
    report: Value,
    max_rows: Option<u32>,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::save_report_xlsx_blocking(report, max_rows, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_report_xlsx_from_artifact(
    path: String,
    max_rows: Option<u32>,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::save_report_xlsx_from_artifact_blocking(path, max_rows, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_reports_dir(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || open_reports_dir_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

fn open_reports_dir_blocking(app: tauri::AppHandle) -> Result<String, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    commands_reports::reveal_path_blocking(dir.to_string_lossy().to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn reveal_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || commands_reports::reveal_path_blocking(path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || commands_reports::open_path_blocking(path))
        .await
        .map_err(|e| e.to_string())?
}
