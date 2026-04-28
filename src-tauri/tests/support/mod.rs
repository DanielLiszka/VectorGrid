use ta_desktop_demo_app::state::{AppState, BackgroundTaskState};

fn unique_mock_app_token() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("vectorgrid-test-{}-{nanos}", std::process::id())
}

pub fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
    let token = unique_mock_app_token();
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().product_name = Some(token.clone());
    context.config_mut().identifier = format!("com.vectorgrid.tests.{}", token.replace('-', ""));

    tauri::test::mock_builder()
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .build(context)
        .expect("failed to build tauri mock app")
}
