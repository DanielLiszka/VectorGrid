use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "app_settings.json";

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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ExecutionRiskPolicySettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_order_notional: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_position_notional: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_short: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_bracket: Option<bool>,
}

impl ExecutionRiskPolicySettings {
    pub fn is_empty(&self) -> bool {
        self.max_order_notional.is_none()
            && self.max_position_notional.is_none()
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
    #[serde(default, skip_serializing_if = "ExecutionRiskPolicySettings::is_empty")]
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
    Ok(())
}

pub fn load_settings(app_local_data_dir: &Path) -> Result<AppSettings, String> {
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
    let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("failed to parse {}: {e}", p.display()))
}

pub fn save_settings(app_local_data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    fs::create_dir_all(app_local_data_dir).map_err(|e| e.to_string())?;
    let p = settings_path(app_local_data_dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&p, json).map_err(|e| format!("failed to write {}: {e}", p.display()))
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
    let mut st = load_settings(app_local_data_dir)?;
    if let Some(dir) = dir {
        validate_market_data_root_dir_path(&settings_path(app_local_data_dir), dir)?;
    }
    st.market_data_root_dir = dir.map(|p| p.to_string_lossy().to_string());
    save_settings(app_local_data_dir, &st)?;
    Ok(st)
}

pub fn set_artifact_retention_settings(
    app_local_data_dir: &Path,
    retention: &ArtifactRetentionSettings,
) -> Result<AppSettings, String> {
    validate_artifact_retention_settings(&settings_path(app_local_data_dir), retention)?;
    let mut st = load_settings(app_local_data_dir)?;
    st.artifact_retention = retention.clone();
    save_settings(app_local_data_dir, &st)?;
    Ok(st)
}

pub fn set_execution_risk_policy_settings(
    app_local_data_dir: &Path,
    policy: &ExecutionRiskPolicySettings,
) -> Result<AppSettings, String> {
    validate_execution_risk_policy_settings(&settings_path(app_local_data_dir), policy)?;
    let mut st = load_settings(app_local_data_dir)?;
    st.execution_risk_policy = policy.clone();
    save_settings(app_local_data_dir, &st)?;
    Ok(st)
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
            allow_short: Some(false),
            require_bracket: Some(true),
        }
    }

    #[test]
    fn execution_risk_policy_defaults_are_unset_when_not_configured() {
        let base = unique_temp_dir("execution_risk_policy_defaults");
        let configured = configured_execution_risk_policy_settings(&base).unwrap();
        assert!(configured.is_empty());
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
        assert!(configured.is_empty());
        let _ = std::fs::remove_dir_all(&base);
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
}
