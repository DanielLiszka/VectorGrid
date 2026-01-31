use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ta_desktop_demo_app::settings;

fn unique_temp_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("ta_desktop_demo_{name}_{ts}"))
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
fn relative_market_data_dir_is_rejected() {
    let base = unique_temp_dir("reject_relative");
    std::fs::create_dir_all(&base).unwrap();

    let p = settings::settings_path(&base);
    std::fs::write(&p, r#"{ "market_data_root_dir": "relative\\path" }"#).unwrap();

    let err = settings::configured_market_data_root_dir(&base).unwrap_err();
    assert!(err.contains("must be an absolute path"));
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

