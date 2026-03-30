mod support;

use std::sync::atomic::Ordering;

use tauri::Manager;
use ta_desktop_demo_app::state::{AppState, BackgroundTaskState};

#[test]
fn mock_runtime_app_resolves_managed_state() {
    let app = support::build_mock_app();

    let app_state = app.state::<AppState>();
    let background_state = app.state::<BackgroundTaskState>();

    assert!(
        !app_state.cancel_token().load(Ordering::SeqCst),
        "app cancellation flag should start cleared"
    );
    assert!(
        !background_state.cancel_token().load(Ordering::SeqCst),
        "background cancellation flag should start cleared"
    );
}
