mod support;

use serde_json::json;
use ta_desktop_demo_app::commands::{
    list_artifact_library_blocking, rebuild_artifact_catalog_blocking, save_run_artifact_blocking,
    update_artifact_tags_blocking,
};
use tauri::Manager;

fn minimal_run_artifact(ts: &str) -> serde_json::Value {
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

#[test]
fn artifact_catalog_rebuilds_and_invalidates_after_json_mutation() {
    let app = support::build_mock_app();
    let handle = app.handle().clone();
    let base = handle
        .path()
        .app_local_data_dir()
        .expect("app local data dir");

    let saved = save_run_artifact_blocking(
        minimal_run_artifact("2026-04-27T12:00:00Z"),
        handle.clone(),
        None,
    )
    .expect("save run artifact");

    let rebuilt =
        rebuild_artifact_catalog_blocking(handle.clone()).expect("rebuild artifact catalog");
    assert_eq!(rebuilt.runs.len(), 1);
    assert!(base.join("artifact_catalog.sqlite3").is_file());

    let cached =
        list_artifact_library_blocking(handle.clone()).expect("list artifact library from catalog");
    assert_eq!(
        serde_json::to_value(&cached).expect("serialize cached snapshot"),
        serde_json::to_value(&rebuilt).expect("serialize rebuilt snapshot")
    );

    update_artifact_tags_blocking(saved.path, vec!["reviewed".to_string()], handle.clone())
        .expect("update artifact tags");

    let refreshed = list_artifact_library_blocking(handle.clone())
        .expect("list artifact library after JSON mutation");
    assert_eq!(refreshed.runs.len(), 1);
    assert_eq!(refreshed.runs[0].tags, vec!["reviewed".to_string()]);

    let _ = std::fs::remove_dir_all(base);
}
