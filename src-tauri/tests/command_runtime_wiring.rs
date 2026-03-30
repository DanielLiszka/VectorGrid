mod support;

use std::sync::atomic::Ordering;

use ta_desktop_demo_app::{
    commands::{cancel_background_task, cancel_current_run, cancel_double_ma_optimization},
    state::{AppState, BackgroundTaskState},
};
use tauri::Manager;

#[test]
fn cancel_background_task_sets_background_cancel_intent() {
    let app = support::build_mock_app();
    let cancel = app.state::<BackgroundTaskState>().cancel_token();

    assert!(!cancel.load(Ordering::SeqCst));
    cancel_background_task(app.state::<BackgroundTaskState>())
        .expect("cancel_background_task should succeed");
    assert!(cancel.load(Ordering::SeqCst));
}

#[test]
fn cancel_current_run_sets_app_cancel_intent() {
    let app = support::build_mock_app();
    let cancel = app.state::<AppState>().cancel_token();

    assert!(!cancel.load(Ordering::SeqCst));
    cancel_current_run(app.state::<AppState>()).expect("cancel_current_run should succeed");
    assert!(cancel.load(Ordering::SeqCst));
}

#[test]
fn cancel_double_ma_optimization_sets_app_cancel_intent() {
    let app = support::build_mock_app();
    let cancel = app.state::<AppState>().cancel_token();

    assert!(!cancel.load(Ordering::SeqCst));
    cancel_double_ma_optimization(app.state::<AppState>())
        .expect("cancel_double_ma_optimization should succeed");
    assert!(cancel.load(Ordering::SeqCst));
}
