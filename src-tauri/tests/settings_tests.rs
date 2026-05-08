use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ta_desktop_demo_app::settings;

struct TempPath {
    path: PathBuf,
}

impl TempPath {
    fn new(name: &str) -> Self {
        Self {
            path: unique_temp_dir(name),
        }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if self.path.is_dir() {
            let _ = std::fs::remove_dir_all(&self.path);
        } else if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("ta_desktop_demo_{name}_{ts}"))
}

fn sample_artifact_retention_settings() -> settings::ArtifactRetentionSettings {
    settings::ArtifactRetentionSettings {
        run_keep_newest: Some(25),
        meta_keep_newest: Some(15),
        stage5_keep_newest: Some(12),
        ml_policy_eval_keep_newest: Some(9),
        batch_summary_keep_newest: Some(18),
        campaign_summary_keep_newest: Some(11),
        options_expression_keep_newest: Some(7),
    }
}

#[test]
fn resolve_defaults_to_app_local_dir_when_unset() {
    let base = unique_temp_dir("resolve_default");
    let configured = settings::configured_market_data_root_dir(&base).unwrap();
    assert!(configured.is_none());

    let resolved = settings::resolve_market_data_root_dir(&base).unwrap();
    assert_eq!(resolved, base);
}

#[test]
fn set_persists_and_resolves_market_data_dir() {
    let base = unique_temp_dir("set_persists");
    let data_root = unique_temp_dir("data_root");

    std::fs::create_dir_all(&data_root).unwrap();
    settings::set_market_data_root_dir(&base, Some(&data_root)).unwrap();

    let configured = settings::configured_market_data_root_dir(&base)
        .unwrap()
        .expect("configured dir");
    assert_eq!(configured, data_root);

    let resolved = settings::resolve_market_data_root_dir(&base).unwrap();
    assert_eq!(resolved, data_root);
}

#[test]
fn clearing_market_data_dir_restores_default_resolution() {
    let base = unique_temp_dir("clear_market_data_dir");
    let data_root = unique_temp_dir("clear_market_data_dir_root");

    std::fs::create_dir_all(&data_root).unwrap();
    settings::set_market_data_root_dir(&base, Some(&data_root)).unwrap();
    settings::set_market_data_root_dir(&base, None).unwrap();

    let configured = settings::configured_market_data_root_dir(&base).unwrap();
    assert!(configured.is_none());

    let resolved = settings::resolve_market_data_root_dir(&base).unwrap();
    assert_eq!(resolved, base);
}

#[test]
fn relative_market_data_dir_is_rejected() {
    let base = unique_temp_dir("reject_relative");
    std::fs::create_dir_all(&base).unwrap();

    let p = settings::settings_path(&base);
    std::fs::write(&p, r#"{ "market_data_root_dir": "relative\\path" }"#).unwrap();

    let err = settings::configured_market_data_root_dir(&base).unwrap_err();
    assert!(err.contains("must be an absolute path"));
}

#[test]
fn set_market_data_dir_rejects_relative_path() {
    let base = TempPath::new("set_market_data_dir_rejects_relative_path");

    let err =
        settings::set_market_data_root_dir(base.path(), Some(std::path::Path::new("relative")))
            .unwrap_err();
    assert!(err.contains("must be an absolute path"));
    assert!(!settings::settings_path(base.path()).exists());
}

#[test]
fn artifact_retention_defaults_are_unset_when_not_configured() {
    let base = unique_temp_dir("artifact_retention_defaults");
    let configured = settings::configured_artifact_retention_settings(&base).unwrap();
    assert!(configured.is_empty());
}

#[test]
fn set_persists_and_resolves_artifact_retention_settings() {
    let base = unique_temp_dir("artifact_retention_set_persists");
    let retention = sample_artifact_retention_settings();

    settings::set_artifact_retention_settings(&base, &retention).unwrap();

    let configured = settings::configured_artifact_retention_settings(&base).unwrap();
    assert_eq!(configured, retention);
}

#[test]
fn clearing_artifact_retention_settings_restores_default_resolution() {
    let base = unique_temp_dir("artifact_retention_clear");
    let retention = sample_artifact_retention_settings();

    settings::set_artifact_retention_settings(&base, &retention).unwrap();
    settings::set_artifact_retention_settings(
        &base,
        &settings::ArtifactRetentionSettings::default(),
    )
    .unwrap();

    let configured = settings::configured_artifact_retention_settings(&base).unwrap();
    assert!(configured.is_empty());
}

#[test]
fn set_artifact_retention_settings_rejects_zero_values() {
    let base = TempPath::new("artifact_retention_rejects_zero");
    let mut retention = sample_artifact_retention_settings();
    retention.stage5_keep_newest = Some(0);

    let err = settings::set_artifact_retention_settings(base.path(), &retention).unwrap_err();
    assert!(err.contains("artifact_retention.stage5_keep_newest"));
    assert!(!settings::settings_path(base.path()).exists());
}

#[test]
fn configured_artifact_retention_settings_reject_invalid_persisted_values() {
    let base = TempPath::new("artifact_retention_invalid_persisted");

    std::fs::create_dir_all(base.path()).unwrap();
    std::fs::write(
        settings::settings_path(base.path()),
        r#"{
  "artifact_retention": {
    "run_keep_newest": 0
  }
}"#,
    )
    .unwrap();

    let err = settings::configured_artifact_retention_settings(base.path()).unwrap_err();
    assert!(err.contains("artifact_retention.run_keep_newest"));
    assert!(err.contains("positive integer"));
}

#[test]
fn move_dir_all_moves_contents() {
    let src = unique_temp_dir("move_src");
    let dst = unique_temp_dir("move_dst");

    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();

    settings::move_dir_all(&src, &dst).unwrap();

    assert!(!src.exists());
    assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello");
}

#[test]
fn move_dir_all_rejects_nested_destination() {
    let src = TempPath::new("move_dir_all_rejects_nested_destination_src");
    let dst = src.path().join("nested").join("dst");

    std::fs::create_dir_all(src.path()).unwrap();
    std::fs::write(src.path().join("a.txt"), b"hello").unwrap();

    let err = settings::move_dir_all(src.path(), &dst).unwrap_err();
    assert!(err.contains("nested destination"));
    assert!(src.path().exists());
    assert!(!dst.exists());
}

#[test]
fn load_settings_reports_malformed_json() {
    let base = TempPath::new("load_settings_reports_malformed_json");

    std::fs::create_dir_all(base.path()).unwrap();
    std::fs::write(
        settings::settings_path(base.path()),
        r#"{ "market_data_root_dir": "#,
    )
    .unwrap();

    let err = settings::load_settings(base.path()).unwrap_err();
    assert!(err.contains("failed to parse"));
    assert!(err.contains("app_settings.json"));
}

#[test]
fn load_settings_reports_invalid_base_path_state() {
    let base = TempPath::new("load_settings_reports_invalid_base_path_state");

    std::fs::write(base.path(), b"not a directory").unwrap();

    let err = settings::load_settings(base.path()).unwrap_err();
    assert!(err.contains("invalid settings directory"));
    assert!(err.contains(&base.path().display().to_string()));
}

#[test]
fn save_settings_fails_when_base_path_is_a_file() {
    let base = TempPath::new("save_settings_fails_when_base_path_is_a_file");

    std::fs::write(base.path(), b"not a directory").unwrap();

    let err = settings::save_settings(base.path(), &settings::AppSettings::default()).unwrap_err();
    assert!(!err.is_empty());
    assert!(!settings::settings_path(base.path()).exists());
}

#[test]
fn save_settings_replaces_existing_file_without_leaving_temp_files() {
    let base = TempPath::new("save_settings_replaces_existing_file_without_leaving_temp_files");
    std::fs::create_dir_all(base.path()).unwrap();

    settings::save_settings(base.path(), &settings::AppSettings::default()).unwrap();
    let mut next = settings::AppSettings::default();
    next.artifact_retention.run_keep_newest = Some(3);
    settings::save_settings(base.path(), &next).unwrap();

    let loaded = settings::load_settings(base.path()).unwrap();
    assert_eq!(loaded.artifact_retention.run_keep_newest, Some(3));
    let temp_files = std::fs::read_dir(base.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("app_settings.json")
                && entry.path().extension().and_then(|value| value.to_str()) == Some("tmp")
        })
        .count();
    assert_eq!(temp_files, 0);
}

#[cfg(unix)]
#[test]
fn move_dir_all_fallback_rejects_unsupported_symlink_entries() {
    use std::os::unix::fs::symlink;

    let src = TempPath::new("move_dir_all_fallback_rejects_unsupported_symlink_entries_src");
    let dst = TempPath::new("move_dir_all_fallback_rejects_unsupported_symlink_entries_dst");

    std::fs::create_dir_all(src.path()).unwrap();
    std::fs::create_dir_all(dst.path()).unwrap();
    std::fs::write(dst.path().join("occupied.txt"), b"existing").unwrap();
    std::fs::write(src.path().join("target.txt"), b"target").unwrap();
    symlink(src.path().join("target.txt"), src.path().join("link.txt")).unwrap();

    let err = settings::move_dir_all(src.path(), dst.path()).unwrap_err();
    assert!(err.contains("unsupported entry type"));
    assert!(src.path().exists());
    assert!(src.path().join("link.txt").exists());
    assert_eq!(
        std::fs::read(dst.path().join("occupied.txt")).unwrap(),
        b"existing"
    );
}
