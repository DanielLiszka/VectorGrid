use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "app_settings.json";
static SETTINGS_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static SETTINGS_IO_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ArtifactRetentionSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_keep_newest: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_keep_newest: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage5_keep_newest: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_policy_eval_keep_newest: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_summary_keep_newest: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_summary_keep_newest: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_expression_keep_newest: Option<usize>,
}

impl ArtifactRetentionSettings {
    pub fn is_empty(&self) -> bool {
        self.run_keep_newest.is_none()
            && self.meta_keep_newest.is_none()
            && self.stage5_keep_newest.is_none()
            && self.ml_policy_eval_keep_newest.is_none()
            && self.batch_summary_keep_newest.is_none()
            && self.campaign_summary_keep_newest.is_none()
            && self.options_expression_keep_newest.is_none()
    }
}

const DEFAULT_MAX_ORDER_NOTIONAL: f64 = 1_000.0;
const DEFAULT_MAX_POSITION_NOTIONAL: f64 = 2_500.0;
const DEFAULT_MAX_DAILY_LOSS: f64 = 250.0;
const DEFAULT_MAX_DAILY_ORDERS: u32 = 20;

fn default_max_order_notional() -> Option<f64> {
    Some(DEFAULT_MAX_ORDER_NOTIONAL)
}

fn default_max_position_notional() -> Option<f64> {
    Some(DEFAULT_MAX_POSITION_NOTIONAL)
}

fn default_max_daily_loss() -> Option<f64> {
    Some(DEFAULT_MAX_DAILY_LOSS)
}

fn default_max_daily_orders() -> Option<u32> {
    Some(DEFAULT_MAX_DAILY_ORDERS)
}

fn default_allow_short() -> Option<bool> {
    Some(false)
}

fn default_require_bracket() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionRiskPolicySettings {
    #[serde(default = "default_max_order_notional")]
    pub max_order_notional: Option<f64>,
    #[serde(default = "default_max_position_notional")]
    pub max_position_notional: Option<f64>,
    #[serde(default = "default_max_daily_loss")]
    pub max_daily_loss: Option<f64>,
    #[serde(default = "default_max_daily_orders")]
    pub max_daily_orders: Option<u32>,
    #[serde(default = "default_allow_short")]
    pub allow_short: Option<bool>,
    #[serde(default = "default_require_bracket")]
    pub require_bracket: Option<bool>,
}

impl Default for ExecutionRiskPolicySettings {
    fn default() -> Self {
        Self {
            max_order_notional: default_max_order_notional(),
            max_position_notional: default_max_position_notional(),
            max_daily_loss: default_max_daily_loss(),
            max_daily_orders: default_max_daily_orders(),
            allow_short: default_allow_short(),
            require_bracket: default_require_bracket(),
        }
    }
}

impl ExecutionRiskPolicySettings {
    pub fn is_empty(&self) -> bool {
        self.max_order_notional.is_none()
            && self.max_position_notional.is_none()
            && self.max_daily_loss.is_none()
            && self.max_daily_orders.is_none()
            && self.allow_short.is_none()
            && self.require_bracket.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_data_root_dir: Option<String>,
    #[serde(default, skip_serializing_if = "ArtifactRetentionSettings::is_empty")]
    pub artifact_retention: ArtifactRetentionSettings,
    #[serde(default)]
    pub execution_risk_policy: ExecutionRiskPolicySettings,
}

pub fn settings_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join(SETTINGS_FILE)
}

fn try_exists_with_context(path: &Path, label: &str) -> Result<bool, String> {
    path.try_exists()
        .map_err(|e| format!("failed to check {label} {}: {e}", path.display()))
}

fn validate_market_data_root_dir_path(
    settings_file_path: &Path,
    market_data_root_dir: &Path,
) -> Result<(), String> {
    if !market_data_root_dir.is_absolute() {
        return Err(format!(
            "invalid market_data_root_dir in {} (must be an absolute path): '{}'",
            settings_file_path.display(),
            market_data_root_dir.display()
        ));
    }
    Ok(())
}

fn validate_positive_retention_value(
    settings_file_path: &Path,
    field_name: &str,
    value: Option<usize>,
) -> Result<(), String> {
    if matches!(value, Some(0)) {
        return Err(format!(
            "invalid artifact_retention.{field_name} in {} (must be a positive integer)",
            settings_file_path.display()
        ));
    }
    Ok(())
}

fn validate_artifact_retention_settings(
    settings_file_path: &Path,
    retention: &ArtifactRetentionSettings,
) -> Result<(), String> {
    validate_positive_retention_value(
        settings_file_path,
        "run_keep_newest",
        retention.run_keep_newest,
    )?;
    validate_positive_retention_value(
        settings_file_path,
        "meta_keep_newest",
        retention.meta_keep_newest,
    )?;
    validate_positive_retention_value(
        settings_file_path,
        "stage5_keep_newest",
        retention.stage5_keep_newest,
    )?;
    validate_positive_retention_value(
        settings_file_path,
        "ml_policy_eval_keep_newest",
        retention.ml_policy_eval_keep_newest,
    )?;
    validate_positive_retention_value(
        settings_file_path,
        "batch_summary_keep_newest",
        retention.batch_summary_keep_newest,
    )?;
    validate_positive_retention_value(
        settings_file_path,
        "campaign_summary_keep_newest",
        retention.campaign_summary_keep_newest,
    )?;
    validate_positive_retention_value(
        settings_file_path,
        "options_expression_keep_newest",
        retention.options_expression_keep_newest,
    )?;
    Ok(())
}

fn validate_positive_f64_setting(
    settings_file_path: &Path,
    field_name: &str,
    value: Option<f64>,
) -> Result<(), String> {
    if value.is_some_and(|v| !v.is_finite() || v <= 0.0) {
        return Err(format!(
            "invalid execution_risk_policy.{field_name} in {} (must be a finite positive number)",
            settings_file_path.display()
        ));
    }
    Ok(())
}

fn validate_execution_risk_policy_settings(
    settings_file_path: &Path,
    policy: &ExecutionRiskPolicySettings,
) -> Result<(), String> {
    validate_positive_f64_setting(
        settings_file_path,
        "max_order_notional",
        policy.max_order_notional,
    )?;
    validate_positive_f64_setting(
        settings_file_path,
        "max_position_notional",
        policy.max_position_notional,
    )?;
    validate_positive_f64_setting(settings_file_path, "max_daily_loss", policy.max_daily_loss)?;
    if matches!(policy.max_daily_orders, Some(0)) {
        return Err(format!(
            "invalid execution_risk_policy.max_daily_orders in {} (must be a positive integer)",
            settings_file_path.display()
        ));
    }
    Ok(())
}

fn settings_io_guard() -> MutexGuard<'static, ()> {
    SETTINGS_IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn load_settings_unlocked(app_local_data_dir: &Path) -> Result<AppSettings, String> {
    if try_exists_with_context(app_local_data_dir, "settings directory")? {
        let metadata = fs::metadata(app_local_data_dir).map_err(|e| {
            format!(
                "failed to inspect settings directory {}: {e}",
                app_local_data_dir.display()
            )
        })?;
        if !metadata.is_dir() {
            return Err(format!(
                "invalid settings directory {}: expected a directory",
                app_local_data_dir.display()
            ));
        }
    }
    let p = settings_path(app_local_data_dir);
    if !try_exists_with_context(&p, "settings file")? {
        return Ok(AppSettings::default());
    }
    let file = File::open(&p).map_err(|e| format!("failed to read {}: {e}", p.display()))?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| format!("failed to parse {}: {e}", p.display()))
}

pub fn load_settings(app_local_data_dir: &Path) -> Result<AppSettings, String> {
    let _guard = settings_io_guard();
    load_settings_unlocked(app_local_data_dir)
}

fn save_settings_unlocked(app_local_data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    fs::create_dir_all(app_local_data_dir).map_err(|e| e.to_string())?;
    let p = settings_path(app_local_data_dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    write_settings_file_atomically(&p, json.as_bytes())
}

pub fn save_settings(app_local_data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    let _guard = settings_io_guard();
    save_settings_unlocked(app_local_data_dir, settings)
}

fn mutate_settings<F>(app_local_data_dir: &Path, mutate: F) -> Result<AppSettings, String>
where
    F: FnOnce(&mut AppSettings) -> Result<(), String>,
{
    let _guard = settings_io_guard();
    let mut settings = load_settings_unlocked(app_local_data_dir)?;
    mutate(&mut settings)?;
    save_settings_unlocked(app_local_data_dir, &settings)?;
    Ok(settings)
}

fn settings_temp_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "settings path is missing a parent directory".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "settings path is missing a filename".to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let sequence = SETTINGS_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        nonce,
        sequence
    )))
}

fn settings_file_metadata_if_exists(path: &Path) -> Result<Option<std::fs::Metadata>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "settings overwrite target must not be a symlink: {}",
                    path.display()
                ));
            }
            if !metadata.is_file() {
                return Err(format!(
                    "settings overwrite target must be a file: {}",
                    path.display()
                ));
            }
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("failed to inspect {}: {error}", path.display())),
    }
}

#[cfg(windows)]
fn replace_existing_settings_file(path: &Path, temp_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    fn wide_null(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let replaced = wide_null(path);
    let replacement = wide_null(temp_path);
    let ok = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_existing_settings_file(path: &Path, temp_path: &Path) -> Result<(), String> {
    std::fs::rename(temp_path, path).map_err(|e| e.to_string())
}

fn write_settings_file_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp_path = settings_temp_path(path)?;
    let write_temp_result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| format!("failed to create {}: {e}", temp_path.display()))?;
        file.write_all(bytes)
            .map_err(|e| format!("failed to write {}: {e}", temp_path.display()))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync {}: {e}", temp_path.display()))
    })();
    if let Err(error) = write_temp_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error);
    }

    let replace_result = (|| -> Result<(), String> {
        if settings_file_metadata_if_exists(path)?.is_some() {
            replace_existing_settings_file(path, &temp_path)
        } else {
            std::fs::rename(&temp_path, path).map_err(|e| e.to_string())
        }
    })();
    if let Err(error) = replace_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("failed to write {}: {error}", path.display()));
    }
    Ok(())
}

pub fn configured_market_data_root_dir(
    app_local_data_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    let st = load_settings(app_local_data_dir)?;
    let Some(raw) = st.market_data_root_dir else {
        return Ok(None);
    };
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let p = PathBuf::from(s);
    validate_market_data_root_dir_path(&settings_path(app_local_data_dir), &p)?;
    Ok(Some(p))
}

pub fn resolve_market_data_root_dir(app_local_data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(p) = configured_market_data_root_dir(app_local_data_dir)? {
        return Ok(p);
    }
    Ok(app_local_data_dir.to_path_buf())
}

pub fn configured_artifact_retention_settings(
    app_local_data_dir: &Path,
) -> Result<ArtifactRetentionSettings, String> {
    let st = load_settings(app_local_data_dir)?;
    validate_artifact_retention_settings(
        &settings_path(app_local_data_dir),
        &st.artifact_retention,
    )?;
    Ok(st.artifact_retention)
}

pub fn configured_execution_risk_policy_settings(
    app_local_data_dir: &Path,
) -> Result<ExecutionRiskPolicySettings, String> {
    let st = load_settings(app_local_data_dir)?;
    validate_execution_risk_policy_settings(
        &settings_path(app_local_data_dir),
        &st.execution_risk_policy,
    )?;
    Ok(st.execution_risk_policy)
}

pub fn set_market_data_root_dir(
    app_local_data_dir: &Path,
    dir: Option<&Path>,
) -> Result<AppSettings, String> {
    mutate_settings(app_local_data_dir, |settings| {
        if let Some(dir) = dir {
            validate_market_data_root_dir_path(&settings_path(app_local_data_dir), dir)?;
        }
        settings.market_data_root_dir = dir.map(|p| p.to_string_lossy().to_string());
        Ok(())
    })
}

pub fn set_artifact_retention_settings(
    app_local_data_dir: &Path,
    retention: &ArtifactRetentionSettings,
) -> Result<AppSettings, String> {
    mutate_settings(app_local_data_dir, |settings| {
        validate_artifact_retention_settings(&settings_path(app_local_data_dir), retention)?;
        settings.artifact_retention = retention.clone();
        Ok(())
    })
}

pub fn set_execution_risk_policy_settings(
    app_local_data_dir: &Path,
    policy: &ExecutionRiskPolicySettings,
) -> Result<AppSettings, String> {
    mutate_settings(app_local_data_dir, |settings| {
        validate_execution_risk_policy_settings(&settings_path(app_local_data_dir), policy)?;
        settings.execution_risk_policy = policy.clone();
        Ok(())
    })
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)
                .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))?;
        } else {
            let entry_kind = if ty.is_symlink() {
                "symlink"
            } else {
                "unsupported special entry"
            };
            return Err(format!(
                "unsupported entry type during copy {} -> {}: {entry_kind}",
                from.display(),
                to.display()
            ));
        }
    }
    Ok(())
}

pub fn move_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    if !try_exists_with_context(src, "source directory")? {
        return Ok(());
    }
    if dst.starts_with(src) {
        return Err(format!(
            "refusing to move {} into itself or a nested destination {}",
            src.display(),
            dst.display()
        ));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_all(src, dst)?;
            fs::remove_dir_all(src).map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("ta_desktop_demo_settings_{name}_{ts}"))
    }

    fn sample_execution_risk_policy_settings() -> ExecutionRiskPolicySettings {
        ExecutionRiskPolicySettings {
            max_order_notional: Some(500.0),
            max_position_notional: Some(1_000.0),
            max_daily_loss: Some(125.0),
            max_daily_orders: Some(10),
            allow_short: Some(false),
            require_bracket: Some(true),
        }
    }

    fn disabled_execution_risk_policy_settings() -> ExecutionRiskPolicySettings {
        ExecutionRiskPolicySettings {
            max_order_notional: None,
            max_position_notional: None,
            max_daily_loss: None,
            max_daily_orders: None,
            allow_short: None,
            require_bracket: None,
        }
    }

    #[test]
    fn execution_risk_policy_defaults_are_conservative_when_not_configured() {
        let base = unique_temp_dir("execution_risk_policy_defaults");
        let configured = configured_execution_risk_policy_settings(&base).unwrap();
        assert_eq!(configured, ExecutionRiskPolicySettings::default());
        assert_eq!(configured.max_order_notional, Some(1_000.0));
        assert_eq!(configured.max_position_notional, Some(2_500.0));
        assert_eq!(configured.max_daily_loss, Some(250.0));
        assert_eq!(configured.max_daily_orders, Some(20));
        assert_eq!(configured.allow_short, Some(false));
        assert_eq!(configured.require_bracket, Some(true));
    }

    #[test]
    fn set_persists_and_resolves_execution_risk_policy_settings() {
        let base = unique_temp_dir("execution_risk_policy_set_persists");
        let policy = sample_execution_risk_policy_settings();

        set_execution_risk_policy_settings(&base, &policy).unwrap();

        let configured = configured_execution_risk_policy_settings(&base).unwrap();
        assert_eq!(configured, policy);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn clearing_execution_risk_policy_settings_restores_default_resolution() {
        let base = unique_temp_dir("execution_risk_policy_clear");
        let policy = sample_execution_risk_policy_settings();

        set_execution_risk_policy_settings(&base, &policy).unwrap();
        set_execution_risk_policy_settings(&base, &ExecutionRiskPolicySettings::default()).unwrap();

        let configured = configured_execution_risk_policy_settings(&base).unwrap();
        assert_eq!(configured, ExecutionRiskPolicySettings::default());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn partial_legacy_policy_inherits_new_conservative_fields() {
        let base = unique_temp_dir("execution_risk_policy_partial_legacy");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            settings_path(&base),
            r#"{
  "execution_risk_policy": {
    "max_order_notional": 750
  }
}"#,
        )
        .unwrap();

        let configured = configured_execution_risk_policy_settings(&base).unwrap();
        assert_eq!(configured.max_order_notional, Some(750.0));
        assert_eq!(configured.max_position_notional, Some(2_500.0));
        assert_eq!(configured.max_daily_loss, Some(250.0));
        assert_eq!(configured.max_daily_orders, Some(20));
        assert_eq!(configured.allow_short, Some(false));
        assert_eq!(configured.require_bracket, Some(true));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn legacy_missing_policy_object_and_fields_use_conservative_defaults() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"market_data_root_dir":"C:/market-data"}"#).unwrap();
        assert_eq!(
            settings.execution_risk_policy,
            ExecutionRiskPolicySettings::default()
        );

        let partial: ExecutionRiskPolicySettings =
            serde_json::from_str(r#"{"allow_short":true}"#).unwrap();
        assert_eq!(partial.allow_short, Some(true));
        assert_eq!(partial.max_order_notional, Some(1_000.0));
        assert_eq!(partial.max_position_notional, Some(2_500.0));
        assert_eq!(partial.max_daily_loss, Some(250.0));
        assert_eq!(partial.max_daily_orders, Some(20));
        assert_eq!(partial.require_bracket, Some(true));
    }

    #[test]
    fn explicit_null_policy_fields_round_trip_as_disabled() {
        let base = unique_temp_dir("execution_risk_policy_explicit_null");
        let disabled = disabled_execution_risk_policy_settings();

        set_execution_risk_policy_settings(&base, &disabled).unwrap();

        let raw = std::fs::read_to_string(settings_path(&base)).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let persisted = json
            .get("execution_risk_policy")
            .and_then(serde_json::Value::as_object)
            .expect("explicit disabled policy object");
        for field in [
            "max_order_notional",
            "max_position_notional",
            "max_daily_loss",
            "max_daily_orders",
            "allow_short",
            "require_bracket",
        ] {
            assert!(
                persisted.get(field).is_some_and(serde_json::Value::is_null),
                "{field} should persist as explicit null"
            );
        }
        assert_eq!(
            configured_execution_risk_policy_settings(&base).unwrap(),
            disabled
        );

        let market_root = base.join("market-data");
        set_market_data_root_dir(&base, Some(&market_root)).unwrap();
        assert_eq!(
            configured_execution_risk_policy_settings(&base).unwrap(),
            disabled
        );

        save_settings(&base, &load_settings(&base).unwrap()).unwrap();
        assert_eq!(
            configured_execution_risk_policy_settings(&base).unwrap(),
            disabled
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn concurrent_distinct_settings_mutations_do_not_lose_updates() {
        let base = Arc::new(unique_temp_dir("concurrent_distinct_mutations"));
        let barrier = Arc::new(Barrier::new(4));
        let market_root = base.join("market-data");
        let retention = ArtifactRetentionSettings {
            run_keep_newest: Some(17),
            ..ArtifactRetentionSettings::default()
        };
        let risk = sample_execution_risk_policy_settings();

        let market_thread = {
            let base = Arc::clone(&base);
            let barrier = Arc::clone(&barrier);
            let market_root = market_root.clone();
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..50 {
                    set_market_data_root_dir(base.as_path(), Some(&market_root)).unwrap();
                }
            })
        };
        let retention_thread = {
            let base = Arc::clone(&base);
            let barrier = Arc::clone(&barrier);
            let retention = retention.clone();
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..50 {
                    set_artifact_retention_settings(base.as_path(), &retention).unwrap();
                }
            })
        };
        let risk_thread = {
            let base = Arc::clone(&base);
            let barrier = Arc::clone(&barrier);
            let risk = risk.clone();
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..50 {
                    set_execution_risk_policy_settings(base.as_path(), &risk).unwrap();
                }
            })
        };

        barrier.wait();
        market_thread.join().unwrap();
        retention_thread.join().unwrap();
        risk_thread.join().unwrap();

        let settings = load_settings(base.as_path()).unwrap();
        assert_eq!(
            settings.market_data_root_dir.as_deref(),
            Some(market_root.to_string_lossy().as_ref())
        );
        assert_eq!(settings.artifact_retention, retention);
        assert_eq!(settings.execution_risk_policy, risk);
        let _: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(settings_path(base.as_path())).unwrap())
                .expect("settings remain parseable");
        let _ = std::fs::remove_dir_all(base.as_path());
    }

    #[test]
    fn set_execution_risk_policy_rejects_invalid_notional_caps() {
        let base = unique_temp_dir("execution_risk_policy_rejects_invalid_notional");
        let mut policy = sample_execution_risk_policy_settings();
        policy.max_order_notional = Some(0.0);

        let err = set_execution_risk_policy_settings(&base, &policy).unwrap_err();
        assert!(err.contains("execution_risk_policy.max_order_notional"));
        assert!(err.contains("finite positive number"));
        assert!(!settings_path(&base).exists());
    }

    #[test]
    fn configured_execution_risk_policy_rejects_invalid_persisted_values() {
        let base = unique_temp_dir("execution_risk_policy_invalid_persisted");

        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            settings_path(&base),
            r#"{
  "execution_risk_policy": {
    "max_position_notional": -1
  }
}"#,
        )
        .unwrap();

        let err = configured_execution_risk_policy_settings(&base).unwrap_err();
        assert!(err.contains("execution_risk_policy.max_position_notional"));
        assert!(err.contains("finite positive number"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn set_execution_risk_policy_rejects_invalid_daily_limits() {
        let base = unique_temp_dir("execution_risk_policy_invalid_daily");
        let mut policy = sample_execution_risk_policy_settings();
        policy.max_daily_orders = Some(0);
        let err = set_execution_risk_policy_settings(&base, &policy).unwrap_err();
        assert!(err.contains("execution_risk_policy.max_daily_orders"));

        policy.max_daily_orders = Some(5);
        policy.max_daily_loss = Some(f64::NAN);
        let err = set_execution_risk_policy_settings(&base, &policy).unwrap_err();
        assert!(err.contains("execution_risk_policy.max_daily_loss"));
        assert!(!settings_path(&base).exists());
    }
}
