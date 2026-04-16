use super::commands_reports;
use serde::Serialize;
use serde_json::Value;
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct SaveReportXlsxResponse {
    pub path: String,
    pub bytes: usize,
    pub artifact_updated: bool,
    pub artifact_update_warning: String,
}

/// Build a validation workbook and return it as base64-encoded XLSX bytes.
///
/// This command is transport-oriented: it returns encoded workbook content rather than a saved path.
#[tauri::command]
pub async fn build_report_xlsx(report: Value, max_rows: Option<u32>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::build_report_xlsx_blocking(report, max_rows)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Build the shortlist export and return CSV text, not a persisted file path.
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
    save_report_xlsx_with_app(report, max_rows, app).await
}

async fn save_report_xlsx_with_app<R: tauri::Runtime>(
    report: Value,
    max_rows: Option<u32>,
    app: tauri::AppHandle<R>,
) -> Result<SaveReportXlsxResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::save_report_xlsx_blocking(report, max_rows, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Build and save a report workbook from a persisted artifact path.
///
/// This may update persisted artifact metadata by writing back `report_xlsx_path`. When that save-back
/// fails after the workbook itself was written, `artifact_update_warning` carries the non-fatal warning.
#[tauri::command]
pub async fn save_report_xlsx_from_artifact(
    path: String,
    max_rows: Option<u32>,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    save_report_xlsx_from_artifact_with_app(path, max_rows, app).await
}

async fn save_report_xlsx_from_artifact_with_app<R: tauri::Runtime>(
    path: String,
    max_rows: Option<u32>,
    app: tauri::AppHandle<R>,
) -> Result<SaveReportXlsxResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::save_report_xlsx_from_artifact_blocking(path, max_rows, app)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reveal the app-local reports directory in the host shell and return that directory path.
#[tauri::command]
pub async fn open_reports_dir(app: tauri::AppHandle) -> Result<String, String> {
    open_reports_dir_with_app(app).await
}

async fn open_reports_dir_with_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || open_reports_dir_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

fn open_reports_dir_blocking_with_revealer<R: tauri::Runtime, F>(
    app: tauri::AppHandle<R>,
    reveal: F,
) -> Result<String, String>
where
    F: FnOnce(String) -> Result<(), String>,
{
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = commands_reports::reports_dir_for_base(&base)?;
    let dir_string = dir.to_string_lossy().to_string();
    reveal(dir_string.clone())?;
    Ok(dir_string)
}

fn open_reports_dir_blocking<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    open_reports_dir_blocking_with_revealer(app, commands_reports::reveal_path_blocking)
}

/// Reveal an existing path via the shared shell-launch helper.
#[tauri::command]
pub async fn reveal_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || commands_reports::reveal_path_blocking(path))
        .await
        .map_err(|e| e.to_string())?
}

/// Open an existing path via the shared shell-launch helper.
#[tauri::command]
pub async fn open_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || commands_reports::open_path_blocking(path))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, BackgroundTaskState};
    use serde_json::{json, Value};
    use std::path::Path;

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .manage(AppState::default())
            .manage(BackgroundTaskState::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("failed to build tauri mock app")
    }

    fn unique_suffix(prefix: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{prefix}_{nanos}")
    }

    fn minimal_report(ts: &str) -> Value {
        json!({
          "version": 1,
          "kind": "vectorgrid_report",
          "ts": ts,
          "validation": {
            "res": {
              "data_id": "test",
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "sma",
                    "fast_len": 9,
                    "slow_ma_type": "ema",
                    "slow_len": 21
                  },
                  "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
                  "robust_score": 1.23,
                  "robust_pareto": true
                }
              ]
            }
          }
        })
    }

    fn minimal_validation_result() -> Value {
        json!({
          "data_id": "test",
          "candidates": [
            {
              "params": {
                "fast_ma_type": "sma",
                "fast_len": 9,
                "slow_ma_type": "ema",
                "slow_len": 21
              },
              "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
              "robust_score": 1.23,
              "robust_pareto": true
            }
          ]
        })
    }

    fn minimal_artifact(ts: &str) -> Value {
        json!({
          "kind": "vectorgrid_run_artifact",
          "ts": ts,
          "dataset_meta": {
            "csv_path": "C:/data/AAPL.csv",
            "timeframe": "1h",
            "label": "AAPL artifact",
            "symbol": "AAPL"
          },
          "run": {
            "req": { "strategy_kind": "DoubleMa" },
            "res": { "top": [] }
          },
          "validation": {
            "req": { "candidates": [] },
            "res": {
              "data_id": "artifact-test",
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "sma",
                    "fast_len": 9,
                    "slow_ma_type": "ema",
                    "slow_len": 21
                  },
                  "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
                  "robust_score": 1.23,
                  "robust_pareto": true
                }
              ]
            }
          }
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_report_xlsx_async_matches_blocking_builder() {
        let report = minimal_report(&unique_suffix("build_report"));
        let wrapped = build_report_xlsx(report.clone(), Some(1))
            .await
            .expect("async xlsx");
        let direct =
            commands_reports::build_report_xlsx_blocking(report, Some(1)).expect("blocking xlsx");
        assert_eq!(wrapped, direct);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_validation_shortlist_csv_async_matches_blocking_builder() {
        let validation_res = minimal_validation_result();
        let wrapped = build_validation_shortlist_csv(validation_res.clone())
            .await
            .expect("async csv");
        let direct = commands_reports::build_validation_shortlist_csv_blocking(validation_res)
            .expect("blocking csv");
        assert_eq!(wrapped, direct);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn save_report_xlsx_async_writes_under_app_local_reports_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let ts = unique_suffix("save_report");

        let SaveReportXlsxResponse {
            path,
            bytes,
            artifact_updated,
            artifact_update_warning,
        } = save_report_xlsx_with_app(minimal_report(&ts), Some(1), handle.clone())
            .await
            .expect("async save");
        assert!(Path::new(&path).is_file());
        assert!(path.contains("reports"));
        assert!(path.ends_with(".xlsx"));
        assert!(bytes > 0);
        assert!(!artifact_updated);
        assert!(artifact_update_warning.is_empty());
        assert!(path.contains(&ts.replace(':', "-")));

        let base = handle
            .path()
            .app_local_data_dir()
            .expect("app local data dir");
        assert!(Path::new(&path).starts_with(base.join("reports")));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn save_report_xlsx_from_artifact_async_writes_report_from_runs_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let base = handle
            .path()
            .app_local_data_dir()
            .expect("app local data dir");
        let runs_dir = base.join("runs");
        std::fs::create_dir_all(&runs_dir).expect("runs dir");

        let ts = unique_suffix("artifact");
        let artifact_path = runs_dir.join(format!("vectorgrid_run_{ts}.json"));
        std::fs::write(
            &artifact_path,
            serde_json::to_vec_pretty(&minimal_artifact(&ts)).expect("artifact json"),
        )
        .expect("write artifact");

        let saved = save_report_xlsx_from_artifact_with_app(
            artifact_path.to_string_lossy().to_string(),
            Some(1),
            handle.clone(),
        )
        .await
        .expect("artifact-backed save");

        assert!(Path::new(&saved.path).is_file());
        assert!(saved.path.contains("reports"));
        assert!(saved.bytes > 0);
        assert!(saved.artifact_updated);
        assert!(saved.artifact_update_warning.is_empty());

        let updated_artifact = std::fs::read_to_string(&artifact_path).expect("read artifact");
        let updated_artifact: Value =
            serde_json::from_str(&updated_artifact).expect("parse artifact");
        assert_eq!(
            updated_artifact
                .get("report_xlsx_path")
                .and_then(Value::as_str),
            Some(saved.path.as_str())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn save_report_xlsx_from_artifact_async_rejects_unsupported_artifact_kind() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let base = handle
            .path()
            .app_local_data_dir()
            .expect("app local data dir");
        let runs_dir = base.join("runs");
        std::fs::create_dir_all(&runs_dir).expect("runs dir");

        let ts = unique_suffix("unsupported_kind");
        let artifact_path = runs_dir.join(format!("vectorgrid_run_{ts}.json"));
        std::fs::write(
            &artifact_path,
            serde_json::to_vec_pretty(&json!({
                "kind": "vectorgrid_meta_artifact",
                "ts": ts,
                "selection": { "selected": [] }
            }))
            .expect("artifact json"),
        )
        .expect("write artifact");

        let err = save_report_xlsx_from_artifact_with_app(
            artifact_path.to_string_lossy().to_string(),
            Some(1),
            handle,
        )
        .await
        .expect_err("unsupported artifact kind should fail");
        assert!(err.contains("unsupported artifact kind"));
    }

    #[test]
    fn open_reports_dir_blocking_with_revealer_creates_reports_dir_and_returns_it() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let mut revealed = None;

        let opened = open_reports_dir_blocking_with_revealer(handle.clone(), |path| {
            revealed = Some(path);
            Ok(())
        })
        .expect("open reports dir");

        assert_eq!(revealed, Some(opened.clone()));
        assert!(Path::new(&opened).is_dir());
        let base = handle
            .path()
            .app_local_data_dir()
            .expect("app local data dir");
        assert!(Path::new(&opened).starts_with(base.join("reports")));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn open_reports_dir_with_app_async_creates_reports_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let opened = open_reports_dir_with_app(handle.clone())
            .await
            .expect("open reports dir");

        assert!(Path::new(&opened).is_dir());
        let base = handle
            .path()
            .app_local_data_dir()
            .expect("app local data dir");
        assert!(Path::new(&opened).starts_with(base.join("reports")));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn open_path_async_surfaces_missing_path_errors() {
        let err = open_path("C:/definitely-missing/nonexistent-report.xlsx".to_string())
            .await
            .expect_err("missing path");
        assert!(err.contains("path does not exist"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reveal_path_async_surfaces_missing_path_errors() {
        let err = reveal_path("C:/definitely-missing/nonexistent-report.xlsx".to_string())
            .await
            .expect_err("missing path");
        assert!(err.contains("path does not exist"));
    }
}
