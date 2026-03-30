use ta_desktop_demo_app::state::{AppState, BackgroundTaskState};

pub fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
    tauri::test::mock_builder()
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("failed to build tauri mock app")
}
