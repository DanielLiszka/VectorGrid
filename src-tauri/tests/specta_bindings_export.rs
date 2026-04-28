use std::path::PathBuf;

use ta_desktop_demo_app::command_bindings;

fn unique_temp_bindings_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vectorgrid_specta_bindings_{}_{}.ts",
        std::process::id(),
        nanos
    ))
}

#[test]
fn specta_bindings_are_current() {
    let expected_path = command_bindings::default_bindings_path();
    let expected = std::fs::read_to_string(&expected_path).unwrap_or_else(|error| {
        panic!(
            "failed to read {}; run npm --prefix web run gen:bindings first: {error}",
            expected_path.display()
        )
    });

    let generated_path = unique_temp_bindings_path();
    command_bindings::export_bindings(&generated_path).expect("export temporary Specta bindings");
    let generated =
        std::fs::read_to_string(&generated_path).expect("read temporary Specta bindings");
    let _ = std::fs::remove_file(&generated_path);

    assert_eq!(
        generated, expected,
        "web/src/tauri/generatedBindings.ts is stale; run npm --prefix web run gen:bindings"
    );
}

#[test]
#[ignore]
fn export_specta_bindings() {
    command_bindings::export_bindings_to_default_web_path().expect("export Specta bindings");
}
