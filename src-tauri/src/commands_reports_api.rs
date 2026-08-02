use super::commands_reports;
use crate::local_path_policy::{validate_shell_open_path, ShellOpenPurpose};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct SaveReportXlsxResponse {
    pub path: String,
    pub bytes: usize,
    pub artifact_updated: bool,
    pub artifact_update_warning: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveVcetTemporalReportXlsxRequestV1 {
    pub sealed_universe:
        super::commands_vcet_temporal_universe::VcetTemporalSealedDatasetUniverseV1,
    pub pre_final:
        super::commands_vcet_temporal_prefinal::ReduceVcetTemporalPreFinalShardsResponseV1,
    #[serde(default)]
    pub final_test:
        Option<super::commands_vcet_temporal_final::ReduceVcetTemporalFinalTestShardsResponseV1>,
    #[serde(default)]
    pub label: Option<String>,
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
    save_report_xlsx_with_app(report, max_rows, app).await
}

#[tauri::command]
pub async fn save_vcet_temporal_report_xlsx(
    req: SaveVcetTemporalReportXlsxRequestV1,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    save_vcet_temporal_report_xlsx_with_app(req, app).await
}

async fn save_vcet_temporal_report_xlsx_with_app<R: tauri::Runtime>(
    req: SaveVcetTemporalReportXlsxRequestV1,
    app: tauri::AppHandle<R>,
) -> Result<SaveReportXlsxResponse, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        commands_reports::save_vcet_temporal_report_xlsx_for_base(
            &req.sealed_universe,
            &req.pre_final,
            req.final_test.as_ref(),
            req.label.as_deref(),
            &base,
        )
    })
    .await
    .map_err(|error| format!("VCET temporal XLSX task failed: {error}"))?
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

#[tauri::command]
pub async fn open_reports_dir(app: tauri::AppHandle) -> Result<String, String> {
    open_reports_dir_with_app(app).await
}

async fn open_reports_dir_with_app<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let dir_string = reports_dir_string_for_app(&app)?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(dir_string.clone(), None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(dir_string)
}

fn reports_dir_string_for_app<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<String, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = commands_reports::reports_dir_for_base(&base)?;
    Ok(dir.to_string_lossy().to_string())
}

#[cfg(test)]
fn open_reports_dir_with_revealer<R: tauri::Runtime, F>(
    app: &tauri::AppHandle<R>,
    reveal: F,
) -> Result<String, String>
where
    F: FnOnce(String) -> Result<(), String>,
{
    let dir_string = reports_dir_string_for_app(app)?;
    reveal(dir_string.clone())?;
    Ok(dir_string)
}

#[tauri::command]
pub async fn reveal_path(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let path = validate_generic_or_market_folder(&app, &path, ShellOpenPurpose::GenericReveal)?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_path(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let path = validate_generic_or_market_folder(&app, &path, ShellOpenPurpose::GenericOpen)?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

fn validate_generic_or_market_folder<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &str,
    purpose: ShellOpenPurpose,
) -> Result<std::path::PathBuf, String> {
    match validate_shell_open_path(app, path, purpose) {
        Ok(path) => Ok(path),
        Err(generic_error) => {
            validate_shell_open_path(app, path, ShellOpenPurpose::MarketDataFolderOpen)
                .map_err(|_| generic_error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::commands_vcet_temporal_final::temporal_report_final_fixture;
    use super::super::commands_vcet_temporal_prefinal::temporal_report_pre_final_fixture;
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

    fn assert_temporal_report_receipt<R: tauri::Runtime>(
        response: &SaveReportXlsxResponse,
        app: &tauri::AppHandle<R>,
        expected_stage: &str,
        expected_label: &str,
        universe_sha256: &str,
    ) {
        let path = Path::new(&response.path);
        let base = app.path().app_local_data_dir().expect("app local data dir");
        assert!(path.is_file(), "temporal report must exist");
        assert!(path.starts_with(base.join("reports")));
        let short_hash = universe_sha256
            .trim()
            .strip_prefix("sha256:")
            .unwrap_or(universe_sha256.trim())
            .chars()
            .take(12)
            .collect::<String>();
        let expected_prefix =
            format!("vectorgrid_vcet_temporal_{expected_stage}_{expected_label}_{short_hash}");
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 report filename");
        assert!(file_name.starts_with(&expected_prefix), "{file_name}");
        assert!(file_name.ends_with(".xlsx"));
        let bytes = std::fs::read(path).expect("read temporal report");
        assert!(
            bytes.starts_with(b"PK\x03\x04"),
            "report must begin with a ZIP local-file header"
        );
        assert_eq!(response.bytes, bytes.len());
        assert!(!response.artifact_updated);
        assert!(response.artifact_update_warning.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn build_report_xlsx_async_matches_blocking_builder() {
        let report = minimal_report(&unique_suffix("build_report"));
        let wrapped = build_report_xlsx(report.clone(), Some(1))
            .await
            .expect("async xlsx");
        // Keep this parity check sensitive to wall-clock metadata accidentally
        // leaking back into the production workbook.
        tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
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
    async fn save_vcet_temporal_report_xlsx_async_persists_pending_final_evidence() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let (sealed_universe, pre_final) = temporal_report_pre_final_fixture();
        let label = unique_suffix("pending_temporal");
        let response = save_vcet_temporal_report_xlsx_with_app(
            SaveVcetTemporalReportXlsxRequestV1 {
                sealed_universe: sealed_universe.clone(),
                pre_final,
                final_test: None,
                label: Some(label.clone()),
            },
            handle.clone(),
        )
        .await
        .expect("save pending-final temporal report through async command path");

        assert_temporal_report_receipt(
            &response,
            &handle,
            "selection",
            &label,
            &sealed_universe.universe_sha256,
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn save_vcet_temporal_report_xlsx_async_persists_complete_final_evidence() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let (sealed_universe, pre_final) = temporal_report_pre_final_fixture();
        let final_test = temporal_report_final_fixture(&sealed_universe, &pre_final);
        let label = unique_suffix("complete_temporal");
        let response = save_vcet_temporal_report_xlsx_with_app(
            SaveVcetTemporalReportXlsxRequestV1 {
                sealed_universe: sealed_universe.clone(),
                pre_final,
                final_test: Some(final_test),
                label: Some(label.clone()),
            },
            handle.clone(),
        )
        .await
        .expect("save complete-final temporal report through async command path");

        assert_temporal_report_receipt(
            &response,
            &handle,
            "final",
            &label,
            &sealed_universe.universe_sha256,
        );
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
    fn open_reports_dir_with_revealer_creates_reports_dir_and_returns_it() {
        let app = build_mock_app();
        let handle = app.handle().clone();
        let mut revealed = None;

        let opened = open_reports_dir_with_revealer(&handle, |path| {
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

    #[test]
    fn reports_dir_string_for_app_creates_reports_dir() {
        let app = build_mock_app();
        let handle = app.handle().clone();

        let opened = reports_dir_string_for_app(&handle).expect("reports dir");

        assert!(Path::new(&opened).is_dir());
        let base = handle
            .path()
            .app_local_data_dir()
            .expect("app local data dir");
        assert!(Path::new(&opened).starts_with(base.join("reports")));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn open_path_async_surfaces_missing_path_errors() {
        let err = commands_reports::existing_path(
            "C:/definitely-missing/nonexistent-report.xlsx".to_string(),
        )
        .expect_err("missing path");
        assert!(err.contains("path does not exist"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reveal_path_async_surfaces_missing_path_errors() {
        let err = commands_reports::existing_path(
            "C:/definitely-missing/nonexistent-report.xlsx".to_string(),
        )
        .expect_err("missing path");
        assert!(err.contains("path does not exist"));
    }
}
