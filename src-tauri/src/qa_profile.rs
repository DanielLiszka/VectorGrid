use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tauri::Manager;

pub const APP_IDENTIFIER: &str = "dev.vectoralpha.vectorgrid";
pub const PROFILE_ENABLE_ENV: &str = "VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE";
pub const PROFILE_ROOT_ENV: &str = "VECTORGRID_DESKTOP_SMOKE_ISOLATED_PROFILE_ROOT";
pub const APP_DATA_DIR_ENV: &str = "VECTORGRID_DESKTOP_SMOKE_APP_DATA_DIR";
pub const RUN_ID_ENV: &str = "VECTORGRID_DESKTOP_SMOKE_RUN_ID";
pub const PROFILE_MARKER_NAME: &str = ".vectorgrid-desktop-smoke-profile.json";
pub const PROFILE_ATTESTATION_NAME: &str = ".vectorgrid-qa-profile-attestation.json";

const PROFILE_MARKER_KIND: &str = "vectorgrid-desktop-smoke-profile";
const PROFILE_ATTESTATION_KIND: &str = "vectorgrid-qa-profile-attestation";
const PROFILE_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug)]
pub struct QaProfileOverride {
    root: PathBuf,
    app_data_dir: PathBuf,
    run_id: String,
    marker_created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileMarker {
    schema_version: u32,
    kind: String,
    run_id: String,
    app_identifier: String,
    app_data_dir: String,
    created_at: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedProfilePaths {
    app_config_dir: String,
    app_data_dir: String,
    app_local_data_dir: String,
    app_cache_dir: String,
    app_log_dir: String,
    webview_data_dir: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileAttestation {
    schema_version: u32,
    kind: String,
    run_id: String,
    marker_created_at: String,
    original_identifier: String,
    effective_identifier: String,
    isolation_root: String,
    app_data_dir: String,
    resolved_paths: ResolvedProfilePaths,
}

impl QaProfileOverride {
    pub fn from_environment() -> Result<Option<Self>, String> {
        let enable = std::env::var_os(PROFILE_ENABLE_ENV);
        let root = std::env::var_os(PROFILE_ROOT_ENV);
        let app_data_dir = std::env::var_os(APP_DATA_DIR_ENV);
        let run_id = std::env::var_os(RUN_ID_ENV);
        Self::from_environment_values(
            enable.as_deref(),
            root.as_deref(),
            app_data_dir.as_deref(),
            run_id.as_deref(),
        )
    }

    fn from_environment_values(
        enable: Option<&OsStr>,
        root: Option<&OsStr>,
        app_data_dir: Option<&OsStr>,
        run_id: Option<&OsStr>,
    ) -> Result<Option<Self>, String> {
        // The run id is present in every desktop-smoke process. It is not an
        // activation signal by itself, so ordinary WDIO lanes retain the
        // production identifier and path behavior.
        if enable.is_none() && root.is_none() && app_data_dir.is_none() {
            return Ok(None);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (enable, root, app_data_dir, run_id);
            return Err(
                "the QA app-data override is supported only by the packaged Windows harness"
                    .to_string(),
            );
        }

        #[cfg(target_os = "windows")]
        {
            let enable = required_utf8(PROFILE_ENABLE_ENV, enable)?;
            if enable != "1" {
                return Err(format!(
                    "{PROFILE_ENABLE_ENV} must be exactly 1 when any QA profile override variable is present"
                ));
            }
            let root = PathBuf::from(required_utf8(PROFILE_ROOT_ENV, root)?);
            let app_data_dir = PathBuf::from(required_utf8(APP_DATA_DIR_ENV, app_data_dir)?);
            let run_id = required_utf8(RUN_ID_ENV, run_id)?;
            validate_run_id(&run_id)?;
            Self::load(root, app_data_dir, run_id).map(Some)
        }
    }

    #[cfg(target_os = "windows")]
    fn load(root: PathBuf, app_data_dir: PathBuf, run_id: String) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err(format!("{PROFILE_ROOT_ENV} must be an absolute path"));
        }
        if !app_data_dir.is_absolute() {
            return Err(format!("{APP_DATA_DIR_ENV} must be an absolute path"));
        }

        ensure_ordinary_directory(&root, "QA isolation root")?;
        ensure_ordinary_directory(&app_data_dir, "QA app-data directory")?;

        let canonical_root = fs::canonicalize(&root)
            .map_err(|error| format!("failed to canonicalize QA isolation root: {error}"))?;
        let canonical_app_data = fs::canonicalize(&app_data_dir)
            .map_err(|error| format!("failed to canonicalize QA app-data directory: {error}"))?;
        if !is_strict_descendant(&canonical_app_data, &canonical_root) {
            return Err(format!(
                "QA app-data directory must be strictly inside the sentinel-owned root: {}",
                app_data_dir.display()
            ));
        }

        let expected_app_data = root.join("LocalAppData").join(APP_IDENTIFIER);
        let canonical_expected = fs::canonicalize(&expected_app_data).map_err(|error| {
            format!(
                "expected QA app-data layout is missing at {}: {error}",
                expected_app_data.display()
            )
        })?;
        if !paths_equal(&canonical_app_data, &canonical_expected) {
            return Err(format!(
                "QA app-data directory must use the exact sentinel layout {}",
                expected_app_data.display()
            ));
        }

        let marker_path = root.join(PROFILE_MARKER_NAME);
        ensure_ordinary_file(&marker_path, "QA profile ownership marker")?;
        let marker_bytes = fs::read(&marker_path)
            .map_err(|error| format!("failed to read QA profile ownership marker: {error}"))?;
        let marker: ProfileMarker = serde_json::from_slice(&marker_bytes)
            .map_err(|error| format!("QA profile ownership marker is malformed: {error}"))?;
        if marker.schema_version != PROFILE_SCHEMA_VERSION {
            return Err(format!(
                "QA profile marker schema must be {PROFILE_SCHEMA_VERSION}, found {}",
                marker.schema_version
            ));
        }
        if marker.kind != PROFILE_MARKER_KIND {
            return Err("QA profile marker kind does not match this application".to_string());
        }
        if marker.run_id != run_id {
            return Err("QA profile marker belongs to a different run".to_string());
        }
        if marker.app_identifier != APP_IDENTIFIER {
            return Err("QA profile marker app identifier does not match VectorGrid".to_string());
        }
        if marker.created_at.trim().is_empty() {
            return Err("QA profile marker createdAt must not be empty".to_string());
        }

        let marker_app_data = PathBuf::from(&marker.app_data_dir);
        if !marker_app_data.is_absolute() {
            return Err("QA profile marker appDataDir must be absolute".to_string());
        }
        let canonical_marker_app_data = fs::canonicalize(&marker_app_data)
            .map_err(|error| format!("failed to canonicalize marker appDataDir: {error}"))?;
        if !paths_equal(&canonical_marker_app_data, &canonical_app_data) {
            return Err(
                "QA profile marker appDataDir does not match the requested override".to_string(),
            );
        }

        let profile = Self {
            // Preserve the harness-provided absolute spelling for the runtime
            // identifier and attestation. Windows canonicalization adds a
            // verbatim `\\?\` prefix, which is correct for validation but
            // would make the cross-process JSON contract needlessly differ
            // from the marker/environment path.
            root,
            app_data_dir,
            run_id,
            marker_created_at: marker.created_at,
        };
        for (path, purpose) in [
            (profile.log_dir(), "QA log directory"),
            (profile.webview_data_dir(), "QA WebView data directory"),
        ] {
            fs::create_dir_all(&path)
                .map_err(|error| format!("failed to create {purpose}: {error}"))?;
            ensure_ordinary_directory(&path, purpose)?;
            let canonical_path = fs::canonicalize(&path)
                .map_err(|error| format!("failed to canonicalize {purpose}: {error}"))?;
            if !is_strict_descendant(&canonical_path, &canonical_app_data) {
                return Err(format!(
                    "{purpose} escaped the approved QA app-data directory: {}",
                    path.display()
                ));
            }
        }
        Ok(profile)
    }

    pub fn apply_to_context<R: tauri::Runtime>(
        &self,
        context: &mut tauri::Context<R>,
    ) -> Result<String, String> {
        let original_identifier = context.config().identifier.clone();
        if original_identifier != APP_IDENTIFIER {
            return Err(format!(
                "refusing QA app-data override for unexpected identifier {original_identifier:?}"
            ));
        }
        context.config_mut().identifier = path_text(&self.app_data_dir);
        Ok(original_identifier)
    }

    pub fn log_dir(&self) -> PathBuf {
        self.app_data_dir.join("logs")
    }

    pub fn webview_data_dir(&self) -> PathBuf {
        self.app_data_dir.join("webview2")
    }

    pub fn verify_and_attest<R: tauri::Runtime>(
        &self,
        app: &tauri::App<R>,
        original_identifier: &str,
    ) -> Result<(), String> {
        if original_identifier != APP_IDENTIFIER {
            return Err("QA profile original identifier changed before setup".to_string());
        }
        let effective_identifier = app.config().identifier.clone();
        if !paths_equal(
            Path::new(&effective_identifier),
            self.app_data_dir.as_path(),
        ) {
            return Err(
                "QA profile effective identifier is not the approved app-data path".to_string(),
            );
        }

        fs::create_dir_all(self.log_dir())
            .map_err(|error| format!("failed to create isolated QA log directory: {error}"))?;
        ensure_ordinary_directory(&self.log_dir(), "QA log directory")?;

        let resolver = app.path();
        let app_config_dir = resolver
            .app_config_dir()
            .map_err(|error| format!("failed to resolve QA app config directory: {error}"))?;
        let app_data_dir = resolver
            .app_data_dir()
            .map_err(|error| format!("failed to resolve QA app data directory: {error}"))?;
        let app_local_data_dir = resolver
            .app_local_data_dir()
            .map_err(|error| format!("failed to resolve QA app local-data directory: {error}"))?;
        let app_cache_dir = resolver
            .app_cache_dir()
            .map_err(|error| format!("failed to resolve QA app cache directory: {error}"))?;
        let app_log_dir = resolver
            .app_log_dir()
            .map_err(|error| format!("failed to resolve QA app log directory: {error}"))?;

        for (label, actual) in [
            ("app_config_dir", app_config_dir.as_path()),
            ("app_data_dir", app_data_dir.as_path()),
            ("app_local_data_dir", app_local_data_dir.as_path()),
            ("app_cache_dir", app_cache_dir.as_path()),
        ] {
            if !paths_equal(actual, &self.app_data_dir) {
                return Err(format!(
                    "Tauri {label} escaped the approved QA app-data directory: {}",
                    actual.display()
                ));
            }
        }
        if !paths_equal(&app_log_dir, &self.log_dir()) {
            return Err(format!(
                "Tauri app_log_dir escaped the approved QA app-data directory: {}",
                app_log_dir.display()
            ));
        }

        let resolved_paths = ResolvedProfilePaths {
            app_config_dir: path_text(&app_config_dir),
            app_data_dir: path_text(&app_data_dir),
            app_local_data_dir: path_text(&app_local_data_dir),
            app_cache_dir: path_text(&app_cache_dir),
            app_log_dir: path_text(&app_log_dir),
            webview_data_dir: path_text(&self.webview_data_dir()),
        };
        let attestation = ProfileAttestation {
            schema_version: PROFILE_SCHEMA_VERSION,
            kind: PROFILE_ATTESTATION_KIND.to_string(),
            run_id: self.run_id.clone(),
            marker_created_at: self.marker_created_at.clone(),
            original_identifier: original_identifier.to_string(),
            effective_identifier,
            isolation_root: path_text(&self.root),
            app_data_dir: path_text(&self.app_data_dir),
            resolved_paths,
        };
        let bytes = serde_json::to_vec_pretty(&attestation)
            .map_err(|error| format!("failed to serialize QA profile attestation: {error}"))?;
        let attestation_path = self.app_data_dir.join(PROFILE_ATTESTATION_NAME);
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&attestation_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                ensure_ordinary_file(&attestation_path, "existing QA profile attestation")?;
                let existing = fs::read(&attestation_path).map_err(|read_error| {
                    format!(
                        "failed to read existing QA profile attestation {}: {read_error}",
                        attestation_path.display()
                    )
                })?;
                let existing: ProfileAttestation = serde_json::from_slice(&existing).map_err(
                    |parse_error| {
                        format!(
                            "existing QA profile attestation is malformed and will not be replaced: {parse_error}"
                        )
                    },
                )?;
                if existing != attestation {
                    return Err(
                        "existing QA profile attestation does not match this launch and will not be replaced"
                            .to_string(),
                    );
                }
                return Ok(());
            }
            Err(error) => {
                return Err(format!(
                    "failed to create QA profile attestation {}: {error}",
                    attestation_path.display()
                ));
            }
        };
        file.write_all(&bytes)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("failed to persist QA profile attestation: {error}"))
    }
}

fn required_utf8(name: &str, value: Option<&OsStr>) -> Result<String, String> {
    let value = value.ok_or_else(|| format!("{name} is required for the QA app-data override"))?;
    let value = value
        .to_str()
        .ok_or_else(|| format!("{name} must contain valid Unicode"))?
        .trim();
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(value.to_string())
}

fn validate_run_id(run_id: &str) -> Result<(), String> {
    let bytes = run_id.as_bytes();
    let first_is_valid = bytes.first().is_some_and(u8::is_ascii_alphanumeric);
    let all_are_valid = bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !first_is_valid || bytes.len() > 128 || !all_are_valid {
        return Err(format!(
            "{RUN_ID_ENV} must be 1-128 ASCII letters, digits, dots, underscores, or hyphens and start with a letter or digit"
        ));
    }
    Ok(())
}

fn ensure_ordinary_directory(path: &Path, purpose: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("{purpose} is unavailable at {}: {error}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(format!(
            "{purpose} must be an ordinary directory, not a file, symlink, junction, or other reparse point: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_ordinary_file(path: &Path, purpose: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("{purpose} is unavailable at {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
        return Err(format!(
            "{purpose} must be an ordinary file, not a directory, symlink, or other reparse point: {}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(target_os = "windows"))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(target_os = "windows")]
fn normalized_path_key(path: &Path) -> String {
    let mut value = path_text(path).replace('/', "\\");
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        value = format!(r"\\{rest}");
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        value = rest.to_string();
    }
    value.trim_end_matches('\\').to_lowercase()
}

#[cfg(not(target_os = "windows"))]
fn normalized_path_key(path: &Path) -> String {
    path_text(path)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    normalized_path_key(left) == normalized_path_key(right)
}

fn is_strict_descendant(candidate: &Path, root: &Path) -> bool {
    let candidate = normalized_path_key(candidate);
    let root = normalized_path_key(root);
    candidate != root
        && candidate
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with(std::path::MAIN_SEPARATOR))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::OsStr,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let nonce = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vectorgrid-qa-profile-{label}-{}-{timestamp}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test root must be created");
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_marker(root: &Path, app_data_dir: &Path, run_id: &str) {
        let marker = serde_json::json!({
            "schemaVersion": PROFILE_SCHEMA_VERSION,
            "kind": PROFILE_MARKER_KIND,
            "runId": run_id,
            "appIdentifier": APP_IDENTIFIER,
            "appDataDir": path_text(app_data_dir),
            "createdAt": "2026-07-25T00:00:00.000Z",
        });
        fs::write(
            root.join(PROFILE_MARKER_NAME),
            serde_json::to_vec_pretty(&marker).expect("marker must serialize"),
        )
        .expect("marker must be written");
    }

    #[test]
    fn run_id_alone_does_not_activate_override() {
        let result = QaProfileOverride::from_environment_values(
            None,
            None,
            None,
            Some(OsStr::new("ordinary-smoke-run")),
        )
        .expect("run id alone must be accepted");
        assert!(result.is_none());
    }

    #[test]
    fn partial_override_fails_closed() {
        let error = QaProfileOverride::from_environment_values(
            Some(OsStr::new("1")),
            Some(OsStr::new(r"C:\isolated")),
            None,
            Some(OsStr::new("qa-run")),
        )
        .expect_err("partial override must fail");
        assert!(error.contains(APP_DATA_DIR_ENV));
    }

    #[test]
    fn invalid_run_id_fails_closed() {
        let error = QaProfileOverride::from_environment_values(
            Some(OsStr::new("1")),
            Some(OsStr::new(r"C:\isolated")),
            Some(OsStr::new(
                r"C:\isolated\LocalAppData\dev.vectoralpha.vectorgrid",
            )),
            Some(OsStr::new(r"..\escape")),
        )
        .expect_err("unsafe run id must fail");
        assert!(error.contains(RUN_ID_ENV));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn valid_owned_profile_is_accepted() {
        let root = TestRoot::new("valid");
        let app_data_dir = root.0.join("LocalAppData").join(APP_IDENTIFIER);
        fs::create_dir_all(&app_data_dir).expect("app-data directory must be created");
        write_marker(&root.0, &app_data_dir, "valid-run");

        let profile = QaProfileOverride::from_environment_values(
            Some(OsStr::new("1")),
            Some(root.0.as_os_str()),
            Some(app_data_dir.as_os_str()),
            Some(OsStr::new("valid-run")),
        )
        .expect("valid profile must parse")
        .expect("valid profile must activate");

        assert!(paths_equal(&profile.app_data_dir, &app_data_dir));
        assert!(is_strict_descendant(&profile.app_data_dir, &profile.root));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn marker_mismatch_fails_closed() {
        let root = TestRoot::new("mismatch");
        let app_data_dir = root.0.join("LocalAppData").join(APP_IDENTIFIER);
        fs::create_dir_all(&app_data_dir).expect("app-data directory must be created");
        write_marker(&root.0, &app_data_dir, "other-run");

        let error = QaProfileOverride::from_environment_values(
            Some(OsStr::new("1")),
            Some(root.0.as_os_str()),
            Some(app_data_dir.as_os_str()),
            Some(OsStr::new("current-run")),
        )
        .expect_err("marker mismatch must fail");
        assert!(error.contains("different run"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn app_data_outside_owned_root_fails_closed() {
        let root = TestRoot::new("inside");
        let outside = TestRoot::new("outside");
        let app_data_dir = outside.0.join("LocalAppData").join(APP_IDENTIFIER);
        fs::create_dir_all(&app_data_dir).expect("outside app-data directory must be created");
        write_marker(&root.0, &app_data_dir, "outside-run");

        let error = QaProfileOverride::from_environment_values(
            Some(OsStr::new("1")),
            Some(root.0.as_os_str()),
            Some(app_data_dir.as_os_str()),
            Some(OsStr::new("outside-run")),
        )
        .expect_err("outside app-data path must fail");
        assert!(error.contains("strictly inside"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tauri_resolvers_are_attested_inside_owned_profile() {
        let root = TestRoot::new("resolver");
        let app_data_dir = root.0.join("LocalAppData").join(APP_IDENTIFIER);
        fs::create_dir_all(&app_data_dir).expect("app-data directory must be created");
        write_marker(&root.0, &app_data_dir, "resolver-run");
        let profile = QaProfileOverride::from_environment_values(
            Some(OsStr::new("1")),
            Some(root.0.as_os_str()),
            Some(app_data_dir.as_os_str()),
            Some(OsStr::new("resolver-run")),
        )
        .expect("valid profile must parse")
        .expect("valid profile must activate");

        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().identifier = APP_IDENTIFIER.to_string();
        let original_identifier = profile
            .apply_to_context(&mut context)
            .expect("context override must be accepted");
        let app = tauri::test::mock_builder()
            .build(context)
            .expect("mock app must build");
        profile
            .verify_and_attest(&app, &original_identifier)
            .expect("resolver attestation must pass");
        profile
            .verify_and_attest(&app, &original_identifier)
            .expect("a matching second launch attestation must be reusable");

        let attestation_path = app_data_dir.join(PROFILE_ATTESTATION_NAME);
        let bytes = fs::read(&attestation_path).expect("attestation must exist");
        let attestation: ProfileAttestation =
            serde_json::from_slice(&bytes).expect("attestation must parse");
        assert_eq!(attestation.schema_version, PROFILE_SCHEMA_VERSION);
        assert_eq!(attestation.kind, PROFILE_ATTESTATION_KIND);
        assert_eq!(attestation.run_id, "resolver-run");
        assert!(paths_equal(
            Path::new(&attestation.resolved_paths.app_local_data_dir),
            &app_data_dir
        ));
        assert!(paths_equal(
            Path::new(&attestation.resolved_paths.app_log_dir),
            &app_data_dir.join("logs")
        ));

        let mismatched = serde_json::json!({
            "schemaVersion": PROFILE_SCHEMA_VERSION,
            "kind": PROFILE_ATTESTATION_KIND,
            "runId": "different-run",
        });
        fs::write(
            &attestation_path,
            serde_json::to_vec_pretty(&mismatched).expect("mismatch must serialize"),
        )
        .expect("mismatch must be written");
        let error = profile
            .verify_and_attest(&app, &original_identifier)
            .expect_err("a mismatched existing attestation must fail closed");
        assert!(error.contains("malformed") || error.contains("does not match"));
    }
}
