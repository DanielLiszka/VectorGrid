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

fn read_expected_bindings_or_skip() -> Option<String> {
    let expected_path = command_bindings::default_bindings_path();
    match std::fs::read_to_string(&expected_path) {
        Ok(expected) => Some(expected),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let web_root = expected_path
                .ancestors()
                .nth(3)
                .unwrap_or(expected_path.as_path());
            if !web_root.exists() {
                eprintln!(
                    "skipping frontend Specta binding parity; {} is not present in this checkout",
                    web_root.display()
                );
                return None;
            }
            panic!(
                "failed to read {}; run npm --prefix web run gen:bindings first: {error}",
                expected_path.display()
            )
        }
        Err(error) => {
            panic!(
                "failed to read {}; run npm --prefix web run gen:bindings first: {error}",
                expected_path.display()
            )
        }
    }
}

#[test]
fn specta_bindings_are_current() {
    let Some(expected) = read_expected_bindings_or_skip() else {
        return;
    };

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
fn specta_binding_allowlist_is_documented_by_generated_output() {
    let Some(expected) = read_expected_bindings_or_skip() else {
        return;
    };

    for command in command_bindings::TYPED_BINDING_COMMANDS {
        assert!(
            expected.contains(&format!("TAURI_INVOKE(\"{command}\"")),
            "typed Specta binding output is missing {command}"
        );
    }

    assert_eq!(
        command_bindings::TYPED_BINDING_COMMANDS.len(),
        expected.matches("TAURI_INVOKE(\"").count(),
        "Specta bindings are a narrow typed DTO allowlist; add new commands intentionally"
    );
}

#[test]
#[ignore]
fn export_specta_bindings() {
    command_bindings::export_bindings_to_default_web_path().expect("export Specta bindings");
}
