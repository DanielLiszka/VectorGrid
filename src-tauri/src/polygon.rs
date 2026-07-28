use crate::redaction::redact_sensitive_text_with_values;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "polygon_settings.json";
const MANIFEST_FILE: &str = "polygon_manifest.json";
const API_BASE: &str = "https://api.polygon.io";
const BARS_PER_REQUEST_LIMIT: u32 = 50_000;
const CHUNK_DAYS: i64 = 30;
const KEYRING_USERNAME: &str = "polygon_api_key";
const FREE_PLAN_LOOKBACK_DAYS: i64 = 365 * 2;
const QA_PROFILE_OVERRIDE_ENV: &str = "VECTORGRID_DESKTOP_SMOKE_APP_DATA_OVERRIDE";
static POLYGON_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonSettings {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonDatasetMeta {
    pub market: String,
    pub display_symbol: String,
    pub polygon_ticker: String,
    #[serde(default = "default_polygon_timeframe")]
    pub timeframe: String,
    pub csv_path: String,
    pub min_ts: i64,
    pub max_ts: i64,
    pub last_updated_utc_ms: i64,
    #[serde(default)]
    pub row_count: u64,
    #[serde(default)]
    pub adjusted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<PolygonDatasetIntegrity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonDatasetIntegrity {
    pub checked_utc_ms: i64,
    pub parse_errors: u64,
    pub duplicates: u64,
    pub out_of_order: u64,
    pub same_day_gaps_gt_2min: u64,
    #[serde(default)]
    pub csv_size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csv_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolygonCacheManifest {
    pub datasets: BTreeMap<String, PolygonDatasetMeta>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonCacheInfo {
    pub cache_dir: String,
    pub has_api_key: bool,
    pub credential_status: PolygonCredentialStorageStatus,
    pub datasets: Vec<PolygonDatasetMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolygonCredentialStorageState {
    Missing,
    Keyring,
    JsonFileFallback,
    EnvFallback,
    DotenvFallback,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolygonCredentialStorageStatus {
    pub has_api_key: bool,
    pub state: PolygonCredentialStorageState,
    pub storage_backend: String,
    pub degraded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
struct PolygonApiKeyWithStatus {
    api_key: Option<String>,
    status: PolygonCredentialStorageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonDownloadRequest {
    pub market: String,
    pub symbol: String,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjusted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PolygonDownloadResult {
    pub csv_path: PathBuf,
    pub downloaded_bars: usize,
    pub used_cache_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Market {
    Stocks,
    Fx,
    Crypto,
}

impl Market {
    fn parse(s: &str) -> Result<Self, String> {
        let v = s.trim().to_ascii_lowercase();
        match v.as_str() {
            "stocks" | "stock" => Ok(Market::Stocks),
            "fx" | "forex" => Ok(Market::Fx),
            "crypto" => Ok(Market::Crypto),
            _ => Err("market must be one of: stocks, fx, crypto".to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Market::Stocks => "stocks",
            Market::Fx => "fx",
            Market::Crypto => "crypto",
        }
    }
}

fn default_polygon_timeframe() -> String {
    "1m".to_string()
}

#[derive(Clone, Debug)]
struct TimeframeSpec {
    label: String,
    multiplier: u32,
    timespan: &'static str,
    interval_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadCachePlan {
    FullHit,
    AppendOnly,
    FullRefresh,
}

fn parse_timeframe(raw: Option<&str>) -> Result<TimeframeSpec, String> {
    let s0 = raw.unwrap_or("1m");
    let s = s0.trim().to_ascii_lowercase();
    let s = if s.is_empty() { "1m".to_string() } else { s };

    let (num_str, unit) = s.split_at(s.len().saturating_sub(1));
    let unit = unit.chars().next().unwrap_or('m');
    let n: u32 = num_str
        .parse()
        .map_err(|_| format!("invalid timeframe '{s0}' (expected e.g. 1m, 2m, 2h)"))?;
    if n == 0 {
        return Err(format!("invalid timeframe '{s0}' (multiplier must be > 0)"));
    }

    let (timespan, interval_ms) = match unit {
        'm' => ("minute", (n as i64).saturating_mul(60_000)),
        'h' => ("hour", (n as i64).saturating_mul(3_600_000)),
        'd' => ("day", (n as i64).saturating_mul(86_400_000)),
        _ => {
            return Err(format!(
                "invalid timeframe '{s0}' (supported suffixes: m, h, d)"
            ))
        }
    };
    if interval_ms <= 0 {
        return Err(format!("invalid timeframe '{s0}' (interval must be > 0)"));
    }

    Ok(TimeframeSpec {
        label: format!("{n}{unit}"),
        multiplier: n,
        timespan,
        interval_ms,
    })
}

fn now_utc_ms() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    dur.as_millis() as i64
}

pub fn polygon_cache_dir(data_root_dir: &Path) -> PathBuf {
    data_root_dir.join("polygon_cache")
}

fn settings_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(SETTINGS_FILE)
}

fn manifest_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(MANIFEST_FILE)
}

fn polygon_dataset_key_and_file_name(
    market: Market,
    polygon_ticker: &str,
    adjusted: bool,
    tf_label: &str,
) -> (String, String) {
    if market == Market::Stocks && !adjusted {
        (
            format!("stocks_raw:{polygon_ticker}:{tf_label}"),
            format!(
                "polygon_stocks_raw_{}_{}.csv",
                sanitize_filename_component(polygon_ticker),
                tf_label
            ),
        )
    } else {
        (
            format!("{}:{polygon_ticker}:{tf_label}", market.as_str()),
            format!(
                "polygon_{}_{}_{}.csv",
                market.as_str(),
                sanitize_filename_component(polygon_ticker),
                tf_label
            ),
        )
    }
}

fn write_settings_file(cache_dir: &Path, api_key: &str) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key is empty.".to_string());
    }
    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let settings = PolygonSettings {
        api_key: key.to_string(),
    };
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(settings_path(cache_dir), json).map_err(|e| e.to_string())
}

fn read_settings_file(cache_dir: &Path) -> Result<Option<PolygonSettings>, String> {
    let p = settings_path(cache_dir);
    if !p.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let st: PolygonSettings = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if st.api_key.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(st))
}

fn keyring_service_name(app_identifier: &str) -> String {
    let id = app_identifier.trim();
    if id.is_empty() {
        "vectorgrid".to_string()
    } else {
        format!("{id}.polygon")
    }
}

fn direct_env_api_key_fallback() -> Option<String> {
    for name in ["POLYGON_KEY", "POLYGON_API_KEY"] {
        if let Ok(raw) = std::env::var(name) {
            let key = raw.trim().to_string();
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    None
}

fn inherited_credential_fallbacks_allowed_for(value: Option<&std::ffi::OsStr>) -> bool {
    value != Some(std::ffi::OsStr::new("1"))
}

fn inherited_credential_fallbacks_allowed() -> bool {
    inherited_credential_fallbacks_allowed_for(std::env::var_os(QA_PROFILE_OVERRIDE_ENV).as_deref())
}

fn dotenv_api_key_fallback_from_dir(start_dir: &Path) -> Option<String> {
    for dir in start_dir.ancestors().take(8) {
        let env_path = dir.join(".env");
        if !env_path.is_file() {
            continue;
        }
        if dir != start_dir && !dir.join("Cargo.toml").exists() && !dir.join(".git").exists() {
            continue;
        }
        if let Some(key) = parse_polygon_key_from_dotenv_file(&env_path) {
            return Some(key);
        }
    }
    None
}

fn parse_polygon_key_from_dotenv_file(path: &Path) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        if let Some(key) = parse_polygon_key_from_dotenv_line(line) {
            return Some(key);
        }
    }
    None
}

fn parse_polygon_key_from_dotenv_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let without_export = trimmed
        .strip_prefix("export ")
        .map(str::trim_start)
        .unwrap_or(trimmed);
    let (name, raw_value) = without_export.split_once('=')?;
    let normalized_name = name.trim();
    if !matches!(normalized_name, "POLYGON_KEY" | "POLYGON_API_KEY") {
        return None;
    }
    let value = raw_value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn maybe_migrate_settings_key_to_keyring(app_identifier: &str, cache_dir: &Path, key: &str) {
    let service = keyring_service_name(app_identifier);
    let Ok(entry) = keyring::Entry::new(&service, KEYRING_USERNAME) else {
        return;
    };
    if entry.set_password(key).is_err() {
        return;
    }
    match entry.get_password() {
        Ok(read_back) if read_back.trim() == key => {
            let _ = fs::remove_file(settings_path(cache_dir));
        }
        _ => {}
    }
}

pub fn set_api_key(app_identifier: &str, cache_dir: &Path, api_key: &str) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key is empty.".to_string());
    }

    let service = keyring_service_name(app_identifier);
    let entry = keyring::Entry::new(&service, KEYRING_USERNAME).map_err(|e| e.to_string())?;

    if entry.set_password(key).is_err() {
        write_settings_file(cache_dir, key)?;
        return Ok(());
    }

    let _ = fs::remove_file(settings_path(cache_dir));
    Ok(())
}

pub fn clear_api_key(app_identifier: &str, cache_dir: &Path) -> Result<(), String> {
    let service = keyring_service_name(app_identifier);
    if let Ok(entry) = keyring::Entry::new(&service, KEYRING_USERNAME) {
        let _ = entry.delete_credential();
    }
    let p = settings_path(cache_dir);
    if p.exists() {
        let _ = fs::remove_file(p);
    }
    Ok(())
}

pub fn read_api_key(app_identifier: &str, cache_dir: &Path) -> Result<Option<String>, String> {
    Ok(read_api_key_with_status(app_identifier, cache_dir)?.api_key)
}

fn read_api_key_with_status(
    app_identifier: &str,
    cache_dir: &Path,
) -> Result<PolygonApiKeyWithStatus, String> {
    let service = keyring_service_name(app_identifier);
    let mut keyring_unavailable = false;
    match keyring::Entry::new(&service, KEYRING_USERNAME) {
        Ok(entry) => {
            if let Ok(key) = entry.get_password() {
                let k = key.trim().to_string();
                if !k.is_empty() {
                    return Ok(api_key_status(
                        Some(k),
                        PolygonCredentialStorageState::Keyring,
                        "keyring",
                        false,
                        None,
                    ));
                }
            }
        }
        Err(_) => keyring_unavailable = true,
    }

    let st = read_settings_file(cache_dir)?;
    if let Some(st) = st {
        let key = st.api_key.trim().to_string();
        if !key.is_empty() {
            maybe_migrate_settings_key_to_keyring(app_identifier, cache_dir, &key);
            return Ok(api_key_status(
                Some(key),
                PolygonCredentialStorageState::JsonFileFallback,
                "json_file_fallback",
                true,
                Some(
                    "Polygon API key is stored in legacy JSON fallback; historical downloads remain allowed."
                        .to_string(),
                ),
            ));
        }
    }

    if inherited_credential_fallbacks_allowed() {
        if let Some(key) = direct_env_api_key_fallback() {
            return Ok(api_key_status(
                Some(key),
                PolygonCredentialStorageState::EnvFallback,
                "environment",
                true,
                Some("Polygon API key is sourced from a process environment fallback.".to_string()),
            ));
        }
        let cwd = std::env::current_dir().ok();
        if let Some(key) = cwd.as_deref().and_then(dotenv_api_key_fallback_from_dir) {
            return Ok(api_key_status(
                Some(key),
                PolygonCredentialStorageState::DotenvFallback,
                "dotenv",
                true,
                Some("Polygon API key is sourced from a .env fallback.".to_string()),
            ));
        }
    }

    if keyring_unavailable {
        Ok(api_key_status(
            None,
            PolygonCredentialStorageState::Unavailable,
            "unavailable",
            true,
            Some(
                "Polygon keyring backend is unavailable and no fallback API key was found."
                    .to_string(),
            ),
        ))
    } else {
        Ok(api_key_status(
            None,
            PolygonCredentialStorageState::Missing,
            "missing",
            false,
            None,
        ))
    }
}

fn api_key_status(
    api_key: Option<String>,
    state: PolygonCredentialStorageState,
    storage_backend: &str,
    degraded: bool,
    message: Option<String>,
) -> PolygonApiKeyWithStatus {
    PolygonApiKeyWithStatus {
        status: PolygonCredentialStorageStatus {
            has_api_key: api_key.is_some(),
            state,
            storage_backend: storage_backend.to_string(),
            degraded,
            message,
        },
        api_key,
    }
}

fn load_manifest(cache_dir: &Path) -> Result<PolygonCacheManifest, String> {
    let p = manifest_path(cache_dir);
    if !p.exists() {
        return Ok(PolygonCacheManifest::default());
    }
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let mut m: PolygonCacheManifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    for (k, ds) in m.datasets.iter_mut() {
        if ds.market == "stocks" && !ds.adjusted && k.starts_with("stocks:") {
            ds.adjusted = true;
        }
    }
    Ok(m)
}

fn polygon_temp_path(parent: &Path, target: &Path) -> Result<PathBuf, String> {
    let file_name = target
        .file_name()
        .ok_or_else(|| format!("Polygon cache path has no file name: {}", target.display()))?;
    let counter = POLYGON_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".{}.{}.{}.tmp", std::process::id(), nonce, counter));
    Ok(parent.join(temp_name))
}

fn create_polygon_staged_file(parent: &Path, target: &Path) -> Result<(PathBuf, fs::File), String> {
    for _ in 0..64 {
        let temp_path = polygon_temp_path(parent, target)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "create staged Polygon cache file {}: {error}",
                    temp_path.display()
                ));
            }
        }
    }

    Err(format!(
        "failed to allocate a unique staged Polygon cache path for {}",
        target.display()
    ))
}

fn existing_regular_polygon_target(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                return Err(format!(
                    "refusing to replace symlink Polygon cache target {}",
                    path.display()
                ));
            }
            if !metadata.is_file() {
                return Err(format!(
                    "refusing to replace non-file Polygon cache target {}",
                    path.display()
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "inspect Polygon cache target {}: {error}",
            path.display()
        )),
    }
}

#[cfg(windows)]
fn replace_polygon_target_with_staged(staged: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let staged_wide: Vec<u16> = staged.as_os_str().encode_wide().chain(Some(0)).collect();
    let ok = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            staged_wide.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        Err(format!(
            "replace Polygon cache target {} with {}: {}",
            target.display(),
            staged.display(),
            io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_polygon_target_with_staged(staged: &Path, target: &Path) -> Result<(), String> {
    fs::rename(staged, target).map_err(|error| {
        format!(
            "replace Polygon cache target {} with {}: {error}",
            target.display(),
            staged.display()
        )
    })
}

fn publish_polygon_staged_file(
    staged: &Path,
    target: &Path,
    target_exists: bool,
) -> Result<(), String> {
    if target_exists {
        replace_polygon_target_with_staged(staged, target)
    } else {
        fs::rename(staged, target).map_err(|error| {
            format!(
                "publish staged Polygon cache file {} to {}: {error}",
                staged.display(),
                target.display()
            )
        })
    }
}

#[cfg(unix)]
fn sync_polygon_parent_dir(parent: &Path) -> Result<(), String> {
    fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|error| format!("sync Polygon cache directory {}: {error}", parent.display()))
}

#[cfg(not(unix))]
fn sync_polygon_parent_dir(_parent: &Path) -> Result<(), String> {
    Ok(())
}

fn write_polygon_file_atomically<T>(
    path: &Path,
    write_contents: impl FnOnce(&mut fs::File) -> Result<T, String>,
) -> Result<T, String> {
    let parent = path
        .parent()
        .filter(|entry| !entry.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "create Polygon cache directory {}: {error}",
            parent.display()
        )
    })?;
    let target_exists = existing_regular_polygon_target(path)?;
    let (staged_path, mut staged_file) = create_polygon_staged_file(parent, path)?;
    let write_result = (|| {
        let output = write_contents(&mut staged_file)?;
        staged_file.sync_all().map_err(|error| {
            format!(
                "sync staged Polygon cache file {}: {error}",
                staged_path.display()
            )
        })?;
        Ok::<T, String>(output)
    })();
    drop(staged_file);

    let output = match write_result {
        Ok(output) => output,
        Err(error) => {
            let _ = fs::remove_file(&staged_path);
            return Err(error);
        }
    };

    if let Err(error) = publish_polygon_staged_file(&staged_path, path, target_exists) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }
    sync_polygon_parent_dir(parent)?;
    Ok(output)
}

fn update_polygon_file_atomically<T>(
    path: &Path,
    update_staged: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    let parent = path
        .parent()
        .filter(|entry| !entry.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "create Polygon cache directory {}: {error}",
            parent.display()
        )
    })?;
    let target_exists = existing_regular_polygon_target(path)?;
    let (staged_path, mut staged_file) = create_polygon_staged_file(parent, path)?;

    let stage_result = (|| {
        if target_exists {
            let mut source = fs::File::open(path)
                .map_err(|error| format!("open Polygon cache file {}: {error}", path.display()))?;
            io::copy(&mut source, &mut staged_file).map_err(|error| {
                format!(
                    "copy Polygon cache file {} to staged file {}: {error}",
                    path.display(),
                    staged_path.display()
                )
            })?;
        }
        staged_file.sync_all().map_err(|error| {
            format!(
                "sync staged Polygon cache file {}: {error}",
                staged_path.display()
            )
        })
    })();
    drop(staged_file);
    if let Err(error) = stage_result {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }

    let output = match update_staged(&staged_path) {
        Ok(output) => output,
        Err(error) => {
            let _ = fs::remove_file(&staged_path);
            return Err(error);
        }
    };
    OpenOptions::new()
        .write(true)
        .open(&staged_path)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            let _ = fs::remove_file(&staged_path);
            format!(
                "sync updated staged Polygon cache file {}: {error}",
                staged_path.display()
            )
        })?;

    if let Err(error) = publish_polygon_staged_file(&staged_path, path, target_exists) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }
    sync_polygon_parent_dir(parent)?;
    Ok(output)
}

fn save_manifest(cache_dir: &Path, m: &PolygonCacheManifest) -> Result<(), String> {
    let json = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    let p = manifest_path(cache_dir);
    write_polygon_file_atomically(&p, |file| {
        file.write_all(json.as_bytes())
            .map_err(|error| format!("write staged Polygon manifest {}: {error}", p.display()))
    })
}

pub fn get_cache_info(app_identifier: &str, cache_dir: &Path) -> Result<PolygonCacheInfo, String> {
    let api_key = read_api_key_with_status(app_identifier, cache_dir)?;
    let manifest = load_manifest(cache_dir)?;
    Ok(PolygonCacheInfo {
        cache_dir: cache_dir.to_string_lossy().to_string(),
        has_api_key: api_key.status.has_api_key,
        credential_status: api_key.status,
        datasets: manifest.datasets.values().cloned().collect(),
    })
}

fn plan_download_cache_strategy(
    csv_exists: bool,
    cached_min_date: Option<NaiveDate>,
    cached_max_date: Option<NaiveDate>,
    cached_min_ts: i64,
    cached_max_ts: i64,
    integrity_clean: bool,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> DownloadCachePlan {
    let same_day_tail_request =
        csv_exists && cached_max_date == Some(to_date) && cached_max_ts > 0 && from_date == to_date;
    let have_full_range = match (cached_min_date, cached_max_date) {
        (Some(min_d), Some(max_d)) => csv_exists && from_date >= min_d && to_date <= max_d,
        _ => false,
    };
    if integrity_clean && have_full_range && !same_day_tail_request {
        return DownloadCachePlan::FullHit;
    }

    let can_append_only = match cached_min_date {
        Some(min_d) => {
            integrity_clean
                && csv_exists
                && cached_min_ts > 0
                && cached_max_ts > 0
                && from_date >= min_d
        }
        None => false,
    };
    if can_append_only {
        DownloadCachePlan::AppendOnly
    } else {
        DownloadCachePlan::FullRefresh
    }
}

fn integrity_is_clean(integrity: Option<&PolygonDatasetIntegrity>) -> bool {
    matches!(
        integrity,
        Some(PolygonDatasetIntegrity {
            parse_errors: 0,
            duplicates: 0,
            out_of_order: 0,
            same_day_gaps_gt_2min: 0,
            ..
        })
    )
}

fn csv_fingerprint(path: &Path) -> Result<(u64, String), String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("open CSV {}: {error}", path.display()))?;
    let expected_size = file
        .metadata()
        .map_err(|error| format!("inspect CSV {}: {error}", path.display()))?
        .len();
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut bytes_read = 0u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read CSV {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes_read = bytes_read.saturating_add(count as u64);
    }
    let final_size = file
        .metadata()
        .map_err(|error| format!("re-inspect CSV {}: {error}", path.display()))?
        .len();
    if bytes_read != expected_size || final_size != expected_size {
        return Err(format!(
            "CSV changed while computing its integrity fingerprint: {}",
            path.display()
        ));
    }
    Ok((expected_size, format!("{:x}", hasher.finalize())))
}

fn cached_csv_matches_clean_integrity(path: &Path, meta: Option<&PolygonDatasetMeta>) -> bool {
    let Some(meta) = meta else {
        return false;
    };
    let Some(integrity) = meta.integrity.as_ref() else {
        return false;
    };
    if !integrity_is_clean(Some(integrity))
        || meta.row_count == 0
        || meta.min_ts <= 0
        || meta.max_ts < meta.min_ts
        || integrity.csv_size_bytes == 0
    {
        return false;
    }
    let Some(expected_sha256) = integrity
        .csv_sha256
        .as_deref()
        .map(str::trim)
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
    else {
        return false;
    };
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() != integrity.csv_size_bytes {
        return false;
    }
    matches!(
        csv_fingerprint(path),
        Ok((size, actual_sha256))
            if size == integrity.csv_size_bytes
                && actual_sha256.eq_ignore_ascii_case(expected_sha256)
    )
}

fn integrity_after_download(
    cached_meta: Option<&PolygonDatasetMeta>,
    bytes_changed: bool,
) -> Option<PolygonDatasetIntegrity> {
    if bytes_changed {
        None
    } else {
        cached_meta.and_then(|meta| meta.integrity.clone())
    }
}

fn sanitize_filename_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.';
        out.push(if ok { ch } else { '_' });
    }
    if out.is_empty() {
        "_".to_string()
    } else {
        out
    }
}

fn parse_pair_symbol(symbol: &str) -> Result<(String, String), String> {
    let s = symbol.trim().to_ascii_uppercase().replace(' ', "");
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() != 2 {
            return Err("symbol must look like BASE/QUOTE (e.g. BTC/USD).".to_string());
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else if s.len() == 6 {
        Ok((s[0..3].to_string(), s[3..6].to_string()))
    } else {
        Err("symbol must look like BASE/QUOTE (e.g. BTC/USD).".to_string())
    }
}

fn normalize_symbol(market: Market, symbol: &str) -> Result<(String, String), String> {
    let raw = symbol.trim();
    if raw.is_empty() {
        return Err("symbol is required".to_string());
    }

    if raw.contains(':') {
        let t = raw.trim().to_ascii_uppercase();
        let display = match market {
            Market::Stocks => t.clone(),
            Market::Fx | Market::Crypto => {
                let parts: Vec<&str> = t.split(':').collect();
                let rest = parts.last().unwrap_or(&t.as_str()).to_string();
                if rest.len() == 6 {
                    format!("{}/{}", &rest[0..3], &rest[3..6])
                } else {
                    rest
                }
            }
        };
        return Ok((t, display));
    }

    match market {
        Market::Stocks => {
            if raw.contains('/') {
                return Err("stocks symbol must be a single ticker (e.g. AAPL).".to_string());
            }
            let t = raw.trim().to_ascii_uppercase();
            Ok((t.clone(), t))
        }
        Market::Fx => {
            let (base, quote) = parse_pair_symbol(raw)?;
            Ok((format!("C:{base}{quote}"), format!("{base}/{quote}")))
        }
        Market::Crypto => {
            let (base, quote) = parse_pair_symbol(raw)?;
            Ok((format!("X:{base}{quote}"), format!("{base}/{quote}")))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AggResponse {
    results: Option<Vec<AggBar>>,
    status: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AggBar {
    t: i64,
    o: f64,
    c: f64,
    h: f64,
    l: f64,
    v: f64,
}

fn encode_path_segment(seg: &str) -> String {
    seg.replace(':', "%3A")
}

fn parse_retry_after_ms(value: &str, now: DateTime<Utc>, max_wait_ms: u64) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds.saturating_mul(1000).min(max_wait_ms));
    }
    let parsed = DateTime::parse_from_rfc2822(trimmed)
        .or_else(|_| DateTime::parse_from_rfc3339(trimmed))
        .ok()?
        .with_timezone(&Utc);
    if parsed <= now {
        return Some(0);
    }
    Some(parsed.signed_duration_since(now).num_milliseconds().max(0) as u64)
        .map(|ms| ms.min(max_wait_ms))
}

fn compact_polygon_detail(raw: &str) -> String {
    const MAX_DETAIL_CHARS: usize = 320;
    let mut detail = String::with_capacity(raw.len().min(MAX_DETAIL_CHARS));
    let mut len = 0usize;
    let mut truncated = false;
    for part in raw.split_whitespace() {
        if part.is_empty() {
            continue;
        }
        if !detail.is_empty() {
            if len >= MAX_DETAIL_CHARS {
                truncated = true;
                break;
            }
            detail.push(' ');
            len = len.saturating_add(1);
        }
        for ch in part.chars() {
            if len >= MAX_DETAIL_CHARS {
                truncated = true;
                break;
            }
            detail.push(ch);
            len = len.saturating_add(1);
        }
        if truncated {
            break;
        }
    }
    if truncated {
        detail.push_str("...");
    }
    detail
}

fn polygon_http_error_message(status: reqwest::StatusCode, body: &str) -> String {
    let reason = match status.as_u16() {
        400 => "Polygon rejected the request parameters. Check the symbol, market, date range, and entitlement for the requested data.",
        401 | 403 => "Polygon rejected the API key or the account is not entitled for this endpoint, symbol, or date range.",
        404 => "Polygon did not recognize the requested ticker or endpoint.",
        429 => "Polygon rate limit reached. Wait and retry, reduce the date range, or load an already cached dataset.",
        500..=599 => "Polygon returned a temporary server error. Retry later or reduce the request size.",
        _ => "Polygon request failed.",
    };
    let detail = compact_polygon_detail(body);
    if detail.is_empty() {
        format!("{reason} HTTP status: {status}.")
    } else {
        format!("{reason} HTTP status: {status}. Provider detail: {detail}")
    }
}

fn polygon_payload_error_message(label: &str, value: &str) -> String {
    let detail = compact_polygon_detail(value);
    if detail.is_empty() {
        format!("Polygon returned an unexpected {label}.")
    } else {
        format!("Polygon returned {label}: {detail}")
    }
}

fn polygon_no_bars_message() -> String {
    "Polygon returned no aggregate bars for that request. Check the ticker, market, date range, subscription recency, and whether eligible trades existed during the selected interval.".to_string()
}

fn fetch_chunk(
    client: &Client,
    api_key: &str,
    polygon_ticker: &str,
    from: &str,
    to: &str,
    adjusted: bool,
    tf: &TimeframeSpec,
) -> Result<Vec<AggBar>, String> {
    let ticker = encode_path_segment(polygon_ticker);
    let url = format!(
        "{API_BASE}/v2/aggs/ticker/{ticker}/range/{}/{}/{from}/{to}",
        tf.multiplier, tf.timespan
    );

    #[derive(Serialize)]
    struct AggQuery<'a> {
        adjusted: bool,
        sort: &'a str,
        limit: u32,
        #[serde(rename = "apiKey")]
        api_key: &'a str,
    }

    let query = AggQuery {
        adjusted,
        sort: "asc",
        limit: BARS_PER_REQUEST_LIMIT,
        api_key,
    };

    let mut last_err: Option<String> = None;
    for attempt in 0..=6u32 {
        let resp = client.get(&url).query(&query).send();
        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(redact_sensitive_text_with_values(&e.to_string(), [api_key]));
                if attempt < 6 {
                    std::thread::sleep(std::time::Duration::from_millis(
                        (1_000u64.saturating_mul(1u64 << attempt)).min(30_000),
                    ));
                    continue;
                }
                return Err(format!(
                    "Polygon request failed: {}",
                    last_err.unwrap_or_default()
                ));
            }
        };

        let status = resp.status();
        if status.is_success() {
            let parsed: AggResponse = resp
                .json()
                .map_err(|e| redact_sensitive_text_with_values(&e.to_string(), [api_key]))?;
            if let Some(err) = parsed.error {
                let err = redact_sensitive_text_with_values(&err, [api_key]);
                return Err(polygon_payload_error_message("an API error", &err));
            }
            let st = parsed.status.unwrap_or_default();
            if !st.is_empty() && !st.eq_ignore_ascii_case("OK") {
                let st = redact_sensitive_text_with_values(&st, [api_key]);
                return Err(polygon_payload_error_message("status", &st));
            }
            return Ok(parsed.results.unwrap_or_default());
        }

        let retry_after_ms = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| parse_retry_after_ms(s, Utc::now(), 120_000));

        let body = redact_sensitive_text_with_values(&resp.text().unwrap_or_default(), [api_key]);
        let retriable = status.as_u16() == 429 || status.is_server_error();
        last_err = Some(polygon_http_error_message(status, &body));
        if retriable && attempt < 6 {
            let backoff_ms = (1_000u64.saturating_mul(1u64 << attempt)).min(30_000);
            let wait_ms = retry_after_ms.unwrap_or(backoff_ms).max(backoff_ms);
            std::thread::sleep(std::time::Duration::from_millis(wait_ms));
            continue;
        }

        return Err(polygon_http_error_message(status, &body));
    }

    Err(format!(
        "Polygon request failed after retries: {}",
        last_err.unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn write_csv_header_if_needed(path: &Path) -> Result<(), String> {
    if path.exists() && fs::metadata(path).map(|m| m.len()).unwrap_or(0) > 0 {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut f = BufWriter::new(fs::File::create(path).map_err(|e| e.to_string())?);
    f.write_all(b"timestamp,open,close,high,low,volume\n")
        .map_err(|e| e.to_string())?;
    f.flush().map_err(|e| e.to_string())
}

fn append_bars(
    path: &Path,
    bars: &[AggBar],
    min_ts: &mut i64,
    max_ts: &mut i64,
) -> Result<usize, String> {
    if bars.is_empty() {
        return Ok(0);
    }
    write_csv_header_if_needed(path)?;
    let f = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut w = BufWriter::new(f);
    let mut wrote = 0usize;
    for b in bars {
        if b.t <= *max_ts {
            continue;
        }
        if *min_ts == 0 || b.t < *min_ts {
            *min_ts = b.t;
        }
        if b.t > *max_ts {
            *max_ts = b.t;
        }
        writeln!(w, "{},{},{},{},{},{}", b.t, b.o, b.c, b.h, b.l, b.v)
            .map_err(|e| e.to_string())?;
        wrote += 1;
    }
    w.flush().map_err(|e| e.to_string())?;
    Ok(wrote)
}

fn overwrite_bars(path: &Path, all_bars: &[AggBar]) -> Result<(usize, i64, i64), String> {
    write_polygon_file_atomically(path, |file| {
        let mut w = BufWriter::new(file);
        w.write_all(b"timestamp,open,close,high,low,volume\n")
            .map_err(|e| e.to_string())?;
        let mut min_ts = 0i64;
        let mut max_ts = 0i64;
        let mut wrote = 0usize;
        for b in all_bars {
            if min_ts == 0 || b.t < min_ts {
                min_ts = b.t;
            }
            if b.t > max_ts {
                max_ts = b.t;
            }
            writeln!(w, "{},{},{},{},{},{}", b.t, b.o, b.c, b.h, b.l, b.v)
                .map_err(|e| e.to_string())?;
            wrote += 1;
        }
        w.flush().map_err(|e| e.to_string())?;
        Ok((wrote, min_ts, max_ts))
    })
}

fn csv_day_id_utc(ts_ms: i64) -> i64 {
    ts_ms.div_euclid(86_400_000)
}

fn scan_csv_integrity(
    path: &Path,
    timeframe_interval_ms: i64,
) -> Result<(u64, i64, i64, PolygonDatasetIntegrity), String> {
    let fingerprint_before = csv_fingerprint(path)?;
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let r = BufReader::new(f);

    let mut row_count: u64 = 0;
    let mut min_ts: i64 = 0;
    let mut max_ts: i64 = 0;

    let mut parse_errors: u64 = 0;
    let mut duplicates: u64 = 0;
    let mut out_of_order: u64 = 0;
    let mut gaps: u64 = 0;

    let mut prev_ts: Option<i64> = None;
    let mut prev_day: i64 = 0;
    let gap_threshold_ms = timeframe_interval_ms.saturating_mul(2).max(120_000);

    for (i, line) in r.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if i == 0 && t.to_ascii_lowercase().starts_with("timestamp") {
            continue;
        }

        let ts = match parse_csv_bar(t) {
            Some(bar) => bar.t,
            None => {
                parse_errors = parse_errors.saturating_add(1);
                continue;
            }
        };

        if let Some(prev) = prev_ts {
            let diff = ts - prev;
            if diff == 0 {
                duplicates = duplicates.saturating_add(1);
            } else if diff < 0 {
                out_of_order = out_of_order.saturating_add(1);
            } else if diff > gap_threshold_ms {
                let day = csv_day_id_utc(ts);
                if day == prev_day {
                    gaps = gaps.saturating_add(1);
                }
            }
        }
        prev_ts = Some(ts);
        prev_day = csv_day_id_utc(ts);

        if min_ts == 0 || ts < min_ts {
            min_ts = ts;
        }
        if ts > max_ts {
            max_ts = ts;
        }
        row_count = row_count.saturating_add(1);
    }

    if row_count == 0 || min_ts == 0 || max_ts == 0 {
        return Err("dataset CSV has no valid rows".to_string());
    }
    let fingerprint_after = csv_fingerprint(path)?;
    if fingerprint_before != fingerprint_after {
        return Err(format!(
            "dataset CSV changed while it was being validated: {}",
            path.display()
        ));
    }
    let (csv_size_bytes, csv_sha256) = fingerprint_after;

    Ok((
        row_count,
        min_ts,
        max_ts,
        PolygonDatasetIntegrity {
            checked_utc_ms: now_utc_ms(),
            parse_errors,
            duplicates,
            out_of_order,
            same_day_gaps_gt_2min: gaps,
            csv_size_bytes,
            csv_sha256: Some(csv_sha256),
        },
    ))
}

fn parse_csv_bar(line: &str) -> Option<AggBar> {
    let mut it = line.split(',');
    let t = it.next()?.trim().parse::<i64>().ok()?;
    let o = it.next()?.trim().parse::<f64>().ok()?;
    let c = it.next()?.trim().parse::<f64>().ok()?;
    let h = it.next()?.trim().parse::<f64>().ok()?;
    let l = it.next()?.trim().parse::<f64>().ok()?;
    let v = it.next()?.trim().parse::<f64>().ok()?;
    Some(AggBar { t, o, c, h, l, v })
}

fn compute_same_day_gaps_gt_threshold(sorted: &[AggBar], threshold_ms: i64) -> u64 {
    let mut gaps = 0u64;
    let mut prev_ts: Option<i64> = None;
    let mut prev_day: i64 = 0;
    for b in sorted {
        let ts = b.t;
        if let Some(prev) = prev_ts {
            let diff = ts - prev;
            if diff > threshold_ms {
                let day = csv_day_id_utc(ts);
                if day == prev_day {
                    gaps = gaps.saturating_add(1);
                }
            }
        }
        prev_ts = Some(ts);
        prev_day = csv_day_id_utc(ts);
    }
    gaps
}

fn collect_same_day_gap_dates(sorted: &[AggBar], threshold_ms: i64) -> Vec<NaiveDate> {
    let mut days: BTreeSet<NaiveDate> = BTreeSet::new();
    let mut prev_ts: Option<i64> = None;
    let mut prev_day: i64 = 0;
    for b in sorted {
        let ts = b.t;
        if let Some(prev) = prev_ts {
            let diff = ts - prev;
            if diff > threshold_ms {
                let day = csv_day_id_utc(ts);
                if day == prev_day {
                    if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts) {
                        days.insert(dt.date_naive());
                    }
                }
            }
        }
        prev_ts = Some(ts);
        prev_day = csv_day_id_utc(ts);
    }
    days.into_iter().collect()
}

pub fn validate_cached_dataset(
    cache_dir: &Path,
    csv_path: &str,
) -> Result<PolygonDatasetMeta, String> {
    let (p, cache_root) = canonical_cached_dataset_csv_path(cache_dir, csv_path)?;

    let mut manifest = load_manifest(cache_dir)?;
    let Some(key) = find_manifest_dataset_key_by_canonical_csv_path(&manifest, &cache_root, &p)
    else {
        return Err("dataset not found in Polygon manifest".to_string());
    };

    let tf_label = manifest
        .datasets
        .get(&key)
        .map(|d| d.timeframe.as_str())
        .unwrap_or("1m");
    let tf = parse_timeframe(Some(tf_label))?;
    let (rows, min_ts, max_ts, integrity) = scan_csv_integrity(&p, tf.interval_ms)?;
    let Some(ds) = manifest.datasets.get_mut(&key) else {
        return Err("dataset not found in Polygon manifest".to_string());
    };

    ds.csv_path = p.to_string_lossy().to_string();
    ds.row_count = rows;
    ds.min_ts = min_ts;
    ds.max_ts = max_ts;
    ds.integrity = Some(integrity);

    let out = ds.clone();
    save_manifest(cache_dir, &manifest)?;
    Ok(out)
}

pub fn repair_cached_dataset(
    app_identifier: &str,
    cache_dir: &Path,
    csv_path: &str,
) -> Result<PolygonDatasetMeta, String> {
    let (p, cache_root) = canonical_cached_dataset_csv_path(cache_dir, csv_path)?;

    let mut manifest = load_manifest(cache_dir)?;
    let Some(key) = find_manifest_dataset_key_by_canonical_csv_path(&manifest, &cache_root, &p)
    else {
        return Err("dataset not found in Polygon manifest".to_string());
    };

    let ds0 = manifest
        .datasets
        .get(&key)
        .cloned()
        .ok_or_else(|| "dataset not found in Polygon manifest".to_string())?;
    let tf = parse_timeframe(Some(&ds0.timeframe))?;
    let gap_threshold_ms = tf.interval_ms.saturating_mul(2).max(120_000);
    let market = Market::parse(&ds0.market)?;
    let polygon_ticker = ds0.polygon_ticker.clone();
    let adjusted = market == Market::Stocks && ds0.adjusted;

    let f = fs::File::open(&p).map_err(|e| e.to_string())?;
    let r = BufReader::new(f);

    const MAX_REPAIR_ROWS: usize = 2_000_000;
    let mut bars: Vec<AggBar> = Vec::new();
    let mut parse_errors: u64 = 0;

    for (i, line) in r.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if i == 0 && t.to_ascii_lowercase().starts_with("timestamp") {
            continue;
        }

        if let Some(b) = parse_csv_bar(t) {
            bars.push(b);
            if bars.len() > MAX_REPAIR_ROWS {
                return Err(format!(
                    "dataset is too large to repair in-place (>{MAX_REPAIR_ROWS} rows). Delete and re-download instead."
                ));
            }
        } else {
            parse_errors = parse_errors.saturating_add(1);
        }
    }

    if bars.is_empty() {
        return Err("dataset CSV has no valid rows".to_string());
    }

    bars.sort_by_key(|b| b.t);
    bars.dedup_by_key(|b| b.t);

    let gappy_days = collect_same_day_gap_dates(&bars, gap_threshold_ms);
    if !gappy_days.is_empty() {
        let api_key = match read_api_key(app_identifier, cache_dir) {
            Ok(k) => k,
            Err(_) => None,
        };
        if let Some(api_key) = api_key {
            const MAX_GAP_REPAIR_DAYS: usize = 32;
            if gappy_days.len() <= MAX_GAP_REPAIR_DAYS {
                if let Ok(client) = Client::builder()
                    .user_agent("vectorgrid/0.1 (tauri)")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                {
                    for day in &gappy_days {
                        let d = day.format("%Y-%m-%d").to_string();
                        if let Ok(mut fetched) =
                            fetch_chunk(&client, &api_key, &polygon_ticker, &d, &d, adjusted, &tf)
                        {
                            bars.append(&mut fetched);
                            if bars.len() > MAX_REPAIR_ROWS {
                                break;
                            }
                        }
                    }
                    bars.sort_by_key(|b| b.t);
                    bars.dedup_by_key(|b| b.t);
                }
            }
        }
    }

    let gaps = compute_same_day_gaps_gt_threshold(&bars, gap_threshold_ms);
    let (wrote, new_min, new_max) = overwrite_bars(&p, &bars)?;
    let (csv_size_bytes, csv_sha256) = csv_fingerprint(&p)?;

    let Some(ds) = manifest.datasets.get_mut(&key) else {
        return Err("dataset not found in Polygon manifest".to_string());
    };
    ds.csv_path = p.to_string_lossy().to_string();
    ds.row_count = wrote as u64;
    ds.min_ts = new_min;
    ds.max_ts = new_max;
    ds.last_updated_utc_ms = now_utc_ms();
    ds.integrity = Some(PolygonDatasetIntegrity {
        checked_utc_ms: now_utc_ms(),
        parse_errors,
        duplicates: 0,
        out_of_order: 0,
        same_day_gaps_gt_2min: gaps,
        csv_size_bytes,
        csv_sha256: Some(csv_sha256),
    });

    let out = ds.clone();
    save_manifest(cache_dir, &manifest)?;
    Ok(out)
}

fn canonical_cached_dataset_csv_path(
    cache_dir: &Path,
    csv_path: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let trimmed = csv_path.trim();
    if trimmed.is_empty() {
        return Err("dataset CSV path is empty".to_string());
    }

    let canonical_cache_dir = fs::canonicalize(cache_dir)
        .map_err(|e| format!("Polygon cache directory is inaccessible: {e}"))?;
    let canonical_csv_path = fs::canonicalize(Path::new(trimmed))
        .map_err(|_| format!("dataset file does not exist: {trimmed}"))?;

    if !canonical_csv_path.starts_with(&canonical_cache_dir) {
        return Err("dataset CSV must be inside the Polygon cache directory".to_string());
    }
    if !canonical_csv_path.is_file() {
        return Err("dataset path is not a file".to_string());
    }
    let is_csv = canonical_csv_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"));
    if !is_csv {
        return Err("dataset path must reference a CSV file".to_string());
    }

    Ok((canonical_csv_path, canonical_cache_dir))
}

fn find_manifest_dataset_key_by_canonical_csv_path(
    manifest: &PolygonCacheManifest,
    canonical_cache_dir: &Path,
    canonical_csv_path: &Path,
) -> Option<String> {
    manifest.datasets.iter().find_map(|(key, ds)| {
        let raw = ds.csv_path.trim();
        if raw.is_empty() {
            return None;
        }
        let entry = fs::canonicalize(Path::new(raw)).ok()?;
        if !entry.starts_with(canonical_cache_dir) || entry != canonical_csv_path {
            return None;
        }
        Some(key.clone())
    })
}

fn build_top_off_request_for_dataset(
    dataset: &PolygonDatasetMeta,
    now_date: NaiveDate,
) -> Result<PolygonDownloadRequest, String> {
    if dataset.max_ts <= 0 {
        return Err("cached Polygon dataset has no valid max timestamp".to_string());
    }

    let last_date = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(dataset.max_ts)
        .map(|dt| dt.date_naive())
        .ok_or_else(|| "cached Polygon dataset max timestamp is out of range".to_string())?;
    let earliest_allowed = now_date - Duration::days(FREE_PLAN_LOOKBACK_DAYS);
    if last_date < earliest_allowed {
        return Err(format!(
            "Polygon top-off requires the last dataset date to be within the free-plan lookback window. Last dataset date: {last_date}, earliest allowed: {earliest_allowed}."
        ));
    }

    let timeframe = if dataset.timeframe.trim().is_empty() {
        default_polygon_timeframe()
    } else {
        dataset.timeframe.trim().to_string()
    };
    let from_date = std::cmp::min(last_date, now_date);
    Ok(PolygonDownloadRequest {
        market: dataset.market.trim().to_string(),
        symbol: dataset.polygon_ticker.trim().to_string(),
        from: from_date.format("%Y-%m-%d").to_string(),
        to: now_date.format("%Y-%m-%d").to_string(),
        adjusted: Some(dataset.adjusted),
        timeframe: Some(timeframe),
    })
}

pub fn top_off_cached_dataset(
    app_identifier: &str,
    cache_dir: &Path,
    csv_path: &str,
    cancel: Option<&AtomicBool>,
) -> Result<PolygonDownloadResult, String> {
    let (p, cache_root) = canonical_cached_dataset_csv_path(cache_dir, csv_path)?;
    let manifest = load_manifest(cache_dir)?;
    let key = find_manifest_dataset_key_by_canonical_csv_path(&manifest, &cache_root, &p)
        .ok_or_else(|| "dataset not found in Polygon manifest".to_string())?;
    let dataset = manifest
        .datasets
        .get(&key)
        .cloned()
        .ok_or_else(|| "dataset not found in Polygon manifest".to_string())?;
    let now_date = chrono::Utc::now().date_naive();
    let req = build_top_off_request_for_dataset(&dataset, now_date)?;
    download_1m_to_cache(app_identifier, cache_dir, &req, cancel)
}

fn check_cancelled(cancel: Option<&AtomicBool>) -> Result<(), String> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err("cancelled".to_string());
    }
    Ok(())
}

pub fn download_1m_to_cache(
    app_identifier: &str,
    cache_dir: &Path,
    req: &PolygonDownloadRequest,
    cancel: Option<&AtomicBool>,
) -> Result<PolygonDownloadResult, String> {
    check_cancelled(cancel)?;
    let market = Market::parse(&req.market)?;
    let (polygon_ticker, display_symbol) = normalize_symbol(market, &req.symbol)?;

    let from_date = NaiveDate::parse_from_str(req.from.trim(), "%Y-%m-%d")
        .map_err(|_| "from date must be YYYY-MM-DD".to_string())?;
    let to_date = NaiveDate::parse_from_str(req.to.trim(), "%Y-%m-%d")
        .map_err(|_| "to date must be YYYY-MM-DD".to_string())?;
    if to_date < from_date {
        return Err("to date must be >= from date".to_string());
    }

    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let mut manifest = load_manifest(cache_dir)?;

    let adjusted = if market == Market::Stocks {
        req.adjusted.unwrap_or(true)
    } else {
        false
    };
    let tf = parse_timeframe(req.timeframe.as_deref())?;

    let (dataset_key, file_name) =
        polygon_dataset_key_and_file_name(market, &polygon_ticker, adjusted, &tf.label);
    let csv_path = cache_dir.join(file_name);

    let mut downloaded = 0usize;

    if let Some(meta) = manifest.datasets.get(&dataset_key) {
        let min_ts = meta.min_ts;
        let max_ts = meta.max_ts;
        if min_ts > 0 && max_ts > 0 {
            let want_full_refresh = false;
            let _ = want_full_refresh;
        }
    }

    let cached_meta = manifest.datasets.get(&dataset_key).cloned();
    let mut min_ts = cached_meta.as_ref().map(|m| m.min_ts).unwrap_or(0);
    let mut max_ts = cached_meta.as_ref().map(|m| m.max_ts).unwrap_or(0);
    let mut row_count = cached_meta.as_ref().map(|m| m.row_count).unwrap_or(0);

    let cached_min_date =
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(min_ts).map(|dt| dt.date_naive());
    let cached_max_date =
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(max_ts).map(|dt| dt.date_naive());

    let cache_plan = plan_download_cache_strategy(
        csv_path.exists(),
        cached_min_date,
        cached_max_date,
        min_ts,
        max_ts,
        cached_csv_matches_clean_integrity(&csv_path, cached_meta.as_ref()),
        from_date,
        to_date,
    );

    if cache_plan == DownloadCachePlan::FullHit {
        return Ok(PolygonDownloadResult {
            csv_path,
            downloaded_bars: 0,
            used_cache_only: true,
        });
    } else {
        let api_key = read_api_key(app_identifier, cache_dir)?.ok_or_else(|| {
            "Polygon API key is not set. Enter your key and click Save.".to_string()
        })?;
        let client = Client::builder()
            .user_agent("vectorgrid/0.1 (tauri)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        if cache_plan == DownloadCachePlan::AppendOnly {
            let next_ms = max_ts.saturating_add(tf.interval_ms);
            let next_dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(next_ms)
                .ok_or_else(|| "cached timestamp out of range".to_string())?;
            update_polygon_file_atomically(&csv_path, |staged_path| {
                let mut chunk_from = next_dt.date_naive();
                let chunk_end = to_date;

                while chunk_from <= chunk_end {
                    check_cancelled(cancel)?;
                    let chunk_to =
                        std::cmp::min(chunk_from + Duration::days(CHUNK_DAYS - 1), chunk_end);
                    let from_s = chunk_from.format("%Y-%m-%d").to_string();
                    let to_s = chunk_to.format("%Y-%m-%d").to_string();
                    let bars = fetch_chunk(
                        &client,
                        &api_key,
                        &polygon_ticker,
                        &from_s,
                        &to_s,
                        adjusted,
                        &tf,
                    )?;
                    check_cancelled(cancel)?;
                    let wrote = append_bars(staged_path, &bars, &mut min_ts, &mut max_ts)?;
                    downloaded += wrote;
                    if row_count > 0 {
                        row_count = row_count.saturating_add(wrote as u64);
                    }
                    chunk_from = chunk_to + Duration::days(1);
                }
                check_cancelled(cancel)?;
                Ok(())
            })?;
        } else {
            let mut all: Vec<AggBar> = Vec::new();
            let mut chunk_from = from_date;
            while chunk_from <= to_date {
                check_cancelled(cancel)?;
                let chunk_to = std::cmp::min(chunk_from + Duration::days(CHUNK_DAYS - 1), to_date);
                let from_s = chunk_from.format("%Y-%m-%d").to_string();
                let to_s = chunk_to.format("%Y-%m-%d").to_string();
                let mut bars = fetch_chunk(
                    &client,
                    &api_key,
                    &polygon_ticker,
                    &from_s,
                    &to_s,
                    adjusted,
                    &tf,
                )?;
                check_cancelled(cancel)?;
                all.append(&mut bars);
                chunk_from = chunk_to + Duration::days(1);
            }
            check_cancelled(cancel)?;
            if all.is_empty() {
                return Err(polygon_no_bars_message());
            }

            all.sort_by_key(|b| b.t);
            all.dedup_by_key(|b| b.t);
            let (wrote, new_min, new_max) = overwrite_bars(&csv_path, &all)?;
            downloaded += wrote;
            min_ts = new_min;
            max_ts = new_max;
            row_count = wrote as u64;
        }
    }

    if min_ts == 0 || max_ts == 0 {
        return Err(polygon_no_bars_message());
    }

    let meta = PolygonDatasetMeta {
        market: market.as_str().to_string(),
        display_symbol,
        polygon_ticker: polygon_ticker.clone(),
        timeframe: tf.label.clone(),
        csv_path: csv_path.to_string_lossy().to_string(),
        min_ts,
        max_ts,
        last_updated_utc_ms: now_utc_ms(),
        row_count,
        adjusted,
        integrity: integrity_after_download(cached_meta.as_ref(), downloaded > 0),
    };
    manifest.datasets.insert(dataset_key, meta);
    save_manifest(cache_dir, &manifest)?;

    Ok(PolygonDownloadResult {
        csv_path,
        downloaded_bars: downloaded,
        used_cache_only: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let name = format!("{prefix}_{}_{}", std::process::id(), now_utc_ms().max(1));
        p.push(name);
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    fn polygon_temp_artifacts(dir: &Path, target: &Path) -> Vec<PathBuf> {
        let target_name = target
            .file_name()
            .and_then(|name| name.to_str())
            .expect("target file name");
        let prefix = format!(".{target_name}.");
        fs::read_dir(dir)
            .expect("read temp directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".tmp"))
            })
            .collect()
    }

    #[test]
    fn atomic_polygon_write_failure_preserves_existing_file_and_cleans_stage() {
        let dir = make_temp_dir("vectorta_polygon_atomic_failure");
        let target = dir.join("polygon_manifest.json");
        fs::write(&target, b"old-manifest").expect("seed target");

        let result: Result<(), String> = write_polygon_file_atomically(&target, |file| {
            file.write_all(b"new-but-incomplete")
                .map_err(|error| error.to_string())?;
            Err("injected write failure".to_string())
        });

        assert_eq!(
            result.expect_err("write must fail"),
            "injected write failure"
        );
        assert_eq!(
            fs::read(&target).expect("read preserved target"),
            b"old-manifest"
        );
        assert!(polygon_temp_artifacts(&dir, &target).is_empty());
    }

    #[test]
    fn staged_polygon_update_cancellation_preserves_existing_file_and_cleans_stage() {
        let dir = make_temp_dir("vectorta_polygon_atomic_cancel");
        let target = dir.join("polygon_stocks_TEST_1m.csv");
        let original = b"timestamp,open,close,high,low,volume\n1700000000000,1,1,1,1,1\n";
        fs::write(&target, original).expect("seed target");
        let cancel = AtomicBool::new(false);

        let result: Result<(), String> = update_polygon_file_atomically(&target, |staged| {
            let mut file = OpenOptions::new()
                .append(true)
                .open(staged)
                .map_err(|error| error.to_string())?;
            file.write_all(b"1700000060000,2,2,2,2,2\n")
                .map_err(|error| error.to_string())?;
            file.flush().map_err(|error| error.to_string())?;
            cancel.store(true, Ordering::Relaxed);
            check_cancelled(Some(&cancel))?;
            Ok(())
        });

        assert_eq!(result.expect_err("update must cancel"), "cancelled");
        assert_eq!(fs::read(&target).expect("read preserved target"), original);
        assert!(polygon_temp_artifacts(&dir, &target).is_empty());
    }

    #[test]
    fn save_manifest_atomically_replaces_existing_file_without_temp_leaks() {
        let cache_dir = make_temp_dir("vectorta_polygon_atomic_manifest");
        let target = manifest_path(&cache_dir);
        let mut first = PolygonCacheManifest::default();
        first.datasets.insert(
            "stocks:FIRST:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "FIRST".to_string(),
                polygon_ticker: "FIRST".to_string(),
                timeframe: "1m".to_string(),
                csv_path: cache_dir.join("first.csv").to_string_lossy().to_string(),
                min_ts: 1,
                max_ts: 1,
                last_updated_utc_ms: 1,
                row_count: 1,
                adjusted: true,
                integrity: None,
            },
        );
        save_manifest(&cache_dir, &first).expect("save first manifest");

        let mut second = PolygonCacheManifest::default();
        second.datasets.insert(
            "stocks:SECOND:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "SECOND".to_string(),
                polygon_ticker: "SECOND".to_string(),
                timeframe: "1m".to_string(),
                csv_path: cache_dir.join("second.csv").to_string_lossy().to_string(),
                min_ts: 2,
                max_ts: 2,
                last_updated_utc_ms: 2,
                row_count: 1,
                adjusted: true,
                integrity: None,
            },
        );
        save_manifest(&cache_dir, &second).expect("replace manifest");

        let loaded = load_manifest(&cache_dir).expect("load replaced manifest");
        assert_eq!(
            loaded.datasets.keys().cloned().collect::<Vec<_>>(),
            vec!["stocks:SECOND:1m"]
        );
        assert!(polygon_temp_artifacts(&cache_dir, &target).is_empty());
    }

    #[test]
    fn clean_manifest_fingerprint_rejects_truncated_and_same_size_corrupt_csv() {
        let cache_dir = make_temp_dir("vectorta_polygon_fingerprint");
        let csv_path = cache_dir.join("polygon_stocks_TEST_1m.csv");
        let original =
            b"timestamp,open,close,high,low,volume\n1700000000000,1,1,1,1,1\n1700000060000,2,2,2,2,2\n";
        fs::write(&csv_path, original).expect("seed csv");
        let (row_count, min_ts, max_ts, integrity) =
            scan_csv_integrity(&csv_path, 60_000).expect("scan seeded csv");
        let meta = PolygonDatasetMeta {
            market: "stocks".to_string(),
            display_symbol: "TEST".to_string(),
            polygon_ticker: "TEST".to_string(),
            timeframe: "1m".to_string(),
            csv_path: csv_path.to_string_lossy().to_string(),
            min_ts,
            max_ts,
            last_updated_utc_ms: 1,
            row_count,
            adjusted: true,
            integrity: Some(integrity),
        };
        assert!(cached_csv_matches_clean_integrity(&csv_path, Some(&meta)));

        let mut same_size_corruption = original.to_vec();
        let value = same_size_corruption
            .iter()
            .rposition(|byte| *byte == b'2')
            .expect("numeric value to corrupt");
        same_size_corruption[value] = b'9';
        fs::write(&csv_path, &same_size_corruption).expect("write same-size corruption");
        assert_eq!(
            fs::metadata(&csv_path).expect("corrupt metadata").len(),
            original.len() as u64
        );
        assert!(!cached_csv_matches_clean_integrity(&csv_path, Some(&meta)));

        fs::write(&csv_path, &original[..original.len() / 2]).expect("truncate csv");
        assert!(!cached_csv_matches_clean_integrity(&csv_path, Some(&meta)));
    }

    #[test]
    fn parse_retry_after_ms_accepts_seconds_and_http_dates() {
        let now = DateTime::parse_from_rfc3339("2026-04-30T12:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);

        assert_eq!(parse_retry_after_ms("2", now, 120_000), Some(2_000));
        assert_eq!(parse_retry_after_ms("600", now, 120_000), Some(120_000));
        assert_eq!(
            parse_retry_after_ms("Thu, 30 Apr 2026 12:00:03 GMT", now, 120_000),
            Some(3_000)
        );
        assert_eq!(
            parse_retry_after_ms("Thu, 30 Apr 2026 11:59:59 GMT", now, 120_000),
            Some(0)
        );
        assert_eq!(parse_retry_after_ms("not a date", now, 120_000), None);
    }

    #[test]
    fn polygon_http_error_message_classifies_common_failures() {
        let auth = polygon_http_error_message(
            reqwest::StatusCode::FORBIDDEN,
            r#"{"status":"ERROR","error":"not entitled"}"#,
        );
        assert!(auth.contains("not entitled"));
        assert!(auth.contains("HTTP status: 403 Forbidden"));

        let rate_limit = polygon_http_error_message(reqwest::StatusCode::TOO_MANY_REQUESTS, "");
        assert!(rate_limit.contains("rate limit"));
        assert!(rate_limit.contains("reduce the date range"));

        let no_bars = polygon_no_bars_message();
        assert!(no_bars.contains("no aggregate bars"));
        assert!(no_bars.contains("eligible trades"));
    }

    #[test]
    fn polygon_validate_and_repair_detects_and_fixes_duplicates_and_ordering() {
        let cache_dir = make_temp_dir("vectorta_polygon_test");
        let csv_path = cache_dir.join("polygon_stocks_TEST_1m.csv");
        let csv_path_s = csv_path.to_string_lossy().to_string();

        let base_ts: i64 = 1_700_000_000_000;
        let csv = format!(
            "timestamp,open,close,high,low,volume\n\
{},{},{},{},{},{}\n\
{},{},{},{},{},{}\n\
{},{},{},{},{},{}\n\
{},{},{},{},{},{}\n\
{},{},{},{},{},{}\n\
badline\n",
            base_ts,
            1.0,
            1.0,
            1.0,
            1.0,
            10.0,
            base_ts + 60_000,
            1.0,
            1.0,
            1.0,
            1.0,
            10.0,
            base_ts + 60_000,
            1.0,
            1.0,
            1.0,
            1.0,
            10.0,
            base_ts + 30_000,
            1.0,
            1.0,
            1.0,
            1.0,
            10.0,
            base_ts + 240_000,
            1.0,
            1.0,
            1.0,
            1.0,
            10.0,
        );
        fs::write(&csv_path, csv).expect("write csv");

        let mut manifest = PolygonCacheManifest::default();
        manifest.datasets.insert(
            "stocks:TEST:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "TEST".to_string(),
                polygon_ticker: "TEST".to_string(),
                timeframe: "1m".to_string(),
                csv_path: csv_path_s.clone(),
                min_ts: 0,
                max_ts: 0,
                last_updated_utc_ms: 0,
                row_count: 0,
                adjusted: true,
                integrity: None,
            },
        );
        save_manifest(&cache_dir, &manifest).expect("save manifest");

        let meta = validate_cached_dataset(&cache_dir, &csv_path_s).expect("validate");
        let integ = meta.integrity.expect("integrity");
        assert_eq!(integ.parse_errors, 1);
        assert_eq!(integ.duplicates, 1);
        assert_eq!(integ.out_of_order, 1);
        assert_eq!(integ.same_day_gaps_gt_2min, 1);

        let repaired =
            repair_cached_dataset("vectorta_test", &cache_dir, &csv_path_s).expect("repair");
        let integ2 = repaired.integrity.expect("integrity after repair");
        assert_eq!(integ2.parse_errors, 1);
        assert_eq!(integ2.duplicates, 0);
        assert_eq!(integ2.out_of_order, 0);
        assert_eq!(integ2.same_day_gaps_gt_2min, 1);
        assert_eq!(repaired.row_count, 4);
        assert!(repaired.min_ts > 0);
        assert!(repaired.max_ts > repaired.min_ts);

        let meta2 =
            validate_cached_dataset(&cache_dir, &csv_path_s).expect("validate after repair");
        let integ3 = meta2.integrity.expect("integrity after re-validate");
        assert_eq!(integ3.duplicates, 0);
        assert_eq!(integ3.out_of_order, 0);
        assert_eq!(integ3.same_day_gaps_gt_2min, 1);

        let txt = fs::read_to_string(&csv_path).expect("read repaired csv");
        assert!(txt.to_ascii_lowercase().starts_with("timestamp,open,close"));
    }

    #[test]
    fn build_top_off_request_uses_cached_dataset_identity_and_now_date() {
        let req = build_top_off_request_for_dataset(
            &PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "AAPL".to_string(),
                polygon_ticker: "AAPL".to_string(),
                timeframe: "1m".to_string(),
                csv_path: "C:/cache/polygon_stocks_AAPL_1m.csv".to_string(),
                min_ts: 0,
                max_ts: 1_735_689_600_000,
                last_updated_utc_ms: 0,
                row_count: 0,
                adjusted: true,
                integrity: None,
            },
            NaiveDate::from_ymd_opt(2026, 4, 6).expect("valid date"),
        )
        .expect("build top-off request");

        assert_eq!(req.market, "stocks");
        assert_eq!(req.symbol, "AAPL");
        assert_eq!(req.from, "2025-01-01");
        assert_eq!(req.to, "2026-04-06");
        assert_eq!(req.adjusted, Some(true));
        assert_eq!(req.timeframe.as_deref(), Some("1m"));
    }

    #[test]
    fn build_top_off_request_rejects_dataset_older_than_free_plan_window() {
        let err = build_top_off_request_for_dataset(
            &PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "AAPL".to_string(),
                polygon_ticker: "AAPL".to_string(),
                timeframe: "1m".to_string(),
                csv_path: "C:/cache/polygon_stocks_AAPL_1m.csv".to_string(),
                min_ts: 0,
                max_ts: 1_640_995_200_000,
                last_updated_utc_ms: 0,
                row_count: 0,
                adjusted: true,
                integrity: None,
            },
            NaiveDate::from_ymd_opt(2026, 4, 6).expect("valid date"),
        )
        .expect_err("old dataset should be rejected");

        assert!(err.contains("free-plan lookback window"));
    }

    #[test]
    fn read_api_key_falls_back_to_env_vars_when_cache_is_empty() {
        let cache_dir = make_temp_dir("vectorta_polygon_key_env_test");
        let app_identifier = format!(
            "dev.vectoralpha.vectorgrid.test.{}.{}",
            std::process::id(),
            now_utc_ms().max(1)
        );
        let env_key = "polygon-env-fallback-test-key";
        std::env::set_var("POLYGON_KEY", env_key);
        std::env::remove_var("POLYGON_API_KEY");

        let resolved = read_api_key(&app_identifier, &cache_dir).expect("read api key");
        let with_status =
            read_api_key_with_status(&app_identifier, &cache_dir).expect("read api key status");

        std::env::remove_var("POLYGON_KEY");
        assert_eq!(resolved.as_deref(), Some(env_key));
        assert_eq!(with_status.api_key.as_deref(), Some(env_key));
        assert_eq!(
            with_status.status.state,
            PolygonCredentialStorageState::EnvFallback
        );
        assert!(with_status.status.degraded);
    }

    #[test]
    fn parse_polygon_key_from_dotenv_line_accepts_spaced_and_quoted_assignment() {
        let parsed = parse_polygon_key_from_dotenv_line(r#"POLYGON_KEY = "polygon-dotenv-key""#);
        assert_eq!(parsed.as_deref(), Some("polygon-dotenv-key"));
    }

    #[test]
    fn qa_profile_disables_inherited_credential_fallbacks() {
        assert!(!inherited_credential_fallbacks_allowed_for(Some(
            std::ffi::OsStr::new("1"),
        )));
        assert!(inherited_credential_fallbacks_allowed_for(None));
        assert!(inherited_credential_fallbacks_allowed_for(Some(
            std::ffi::OsStr::new("0"),
        )));
    }

    #[test]
    fn dotenv_api_key_fallback_from_dir_reads_repo_style_env_file() {
        let dir = make_temp_dir("vectorta_polygon_dotenv_test");
        fs::write(
            dir.join(".env"),
            "# comment\nPOLYGON_KEY = \"polygon-dotenv-key\"\n",
        )
        .expect("write .env");

        let resolved = dotenv_api_key_fallback_from_dir(&dir);

        assert_eq!(resolved.as_deref(), Some("polygon-dotenv-key"));
    }

    #[test]
    fn normalize_symbol_supports_market_specific_and_native_formats() {
        let (stocks_ticker, stocks_display) =
            normalize_symbol(Market::Stocks, " aapl ").expect("normalize stock");
        assert_eq!(stocks_ticker, "AAPL");
        assert_eq!(stocks_display, "AAPL");

        let (fx_ticker, fx_display) =
            normalize_symbol(Market::Fx, "eur/usd").expect("normalize fx");
        assert_eq!(fx_ticker, "C:EURUSD");
        assert_eq!(fx_display, "EUR/USD");

        let (crypto_ticker, crypto_display) =
            normalize_symbol(Market::Crypto, "btcusd").expect("normalize crypto");
        assert_eq!(crypto_ticker, "X:BTCUSD");
        assert_eq!(crypto_display, "BTC/USD");

        let (native_ticker, native_display) =
            normalize_symbol(Market::Crypto, "x:btcusd").expect("normalize native ticker");
        assert_eq!(native_ticker, "X:BTCUSD");
        assert_eq!(native_display, "BTC/USD");
    }

    #[test]
    fn polygon_dataset_key_and_file_name_distinguish_adjusted_and_raw_stocks() {
        let (adjusted_key, adjusted_file) =
            polygon_dataset_key_and_file_name(Market::Stocks, "AAPL", true, "1m");
        let (raw_key, raw_file) =
            polygon_dataset_key_and_file_name(Market::Stocks, "AAPL", false, "1m");
        let (fx_key, fx_file) =
            polygon_dataset_key_and_file_name(Market::Fx, "C:EURUSD", false, "5m");

        assert_eq!(adjusted_key, "stocks:AAPL:1m");
        assert_eq!(adjusted_file, "polygon_stocks_AAPL_1m.csv");
        assert_eq!(raw_key, "stocks_raw:AAPL:1m");
        assert_eq!(raw_file, "polygon_stocks_raw_AAPL_1m.csv");
        assert_eq!(fx_key, "fx:C:EURUSD:5m");
        assert_eq!(fx_file, "polygon_fx_C_EURUSD_5m.csv");
    }

    #[test]
    fn plan_download_cache_strategy_distinguishes_hit_append_and_refresh() {
        let min_d = NaiveDate::from_ymd_opt(2024, 1, 1).expect("min date");
        let max_d = NaiveDate::from_ymd_opt(2024, 1, 31).expect("max date");

        let full_hit = plan_download_cache_strategy(
            true,
            Some(min_d),
            Some(max_d),
            1,
            2,
            true,
            NaiveDate::from_ymd_opt(2024, 1, 10).expect("from"),
            NaiveDate::from_ymd_opt(2024, 1, 20).expect("to"),
        );
        assert_eq!(full_hit, DownloadCachePlan::FullHit);

        let append_only = plan_download_cache_strategy(
            true,
            Some(min_d),
            Some(max_d),
            1,
            2,
            true,
            NaiveDate::from_ymd_opt(2024, 1, 1).expect("from"),
            NaiveDate::from_ymd_opt(2024, 2, 15).expect("to"),
        );
        assert_eq!(append_only, DownloadCachePlan::AppendOnly);

        let full_refresh = plan_download_cache_strategy(
            true,
            Some(min_d),
            Some(max_d),
            1,
            2,
            true,
            NaiveDate::from_ymd_opt(2023, 12, 1).expect("from"),
            NaiveDate::from_ymd_opt(2024, 1, 20).expect("to"),
        );
        assert_eq!(full_refresh, DownloadCachePlan::FullRefresh);

        let same_day_tail =
            plan_download_cache_strategy(true, Some(min_d), Some(max_d), 1, 2, true, max_d, max_d);
        assert_eq!(same_day_tail, DownloadCachePlan::AppendOnly);
    }

    #[test]
    fn plan_download_cache_strategy_requires_clean_integrity_for_hit_and_append_paths() {
        let min_d = NaiveDate::from_ymd_opt(2024, 1, 1).expect("min date");
        let max_d = NaiveDate::from_ymd_opt(2024, 1, 31).expect("max date");

        let dirty_full_hit = plan_download_cache_strategy(
            true,
            Some(min_d),
            Some(max_d),
            1,
            2,
            false,
            NaiveDate::from_ymd_opt(2024, 1, 10).expect("from"),
            NaiveDate::from_ymd_opt(2024, 1, 20).expect("to"),
        );
        assert_eq!(dirty_full_hit, DownloadCachePlan::FullRefresh);

        let dirty_append = plan_download_cache_strategy(
            true,
            Some(min_d),
            Some(max_d),
            1,
            2,
            false,
            NaiveDate::from_ymd_opt(2024, 1, 1).expect("from"),
            NaiveDate::from_ymd_opt(2024, 2, 15).expect("to"),
        );
        assert_eq!(dirty_append, DownloadCachePlan::FullRefresh);
    }

    #[test]
    fn integrity_after_download_clears_stale_metadata_when_bytes_change() {
        let cached_meta = PolygonDatasetMeta {
            market: "stocks".to_string(),
            display_symbol: "AAPL".to_string(),
            polygon_ticker: "AAPL".to_string(),
            timeframe: "1m".to_string(),
            csv_path: "dummy.csv".to_string(),
            min_ts: 1,
            max_ts: 2,
            last_updated_utc_ms: 3,
            row_count: 2,
            adjusted: true,
            integrity: Some(PolygonDatasetIntegrity {
                checked_utc_ms: 4,
                parse_errors: 0,
                duplicates: 0,
                out_of_order: 0,
                same_day_gaps_gt_2min: 0,
                csv_size_bytes: 123,
                csv_sha256: Some("a".repeat(64)),
            }),
        };

        assert!(integrity_after_download(Some(&cached_meta), true).is_none());
        assert!(integrity_after_download(Some(&cached_meta), false).is_some());
    }

    #[test]
    fn download_1m_to_cache_full_hit_is_read_only_and_does_not_require_api_key() {
        let cache_dir = make_temp_dir("vectorta_polygon_full_hit");
        let (dataset_key, file_name) =
            polygon_dataset_key_and_file_name(Market::Stocks, "AAPL", true, "1m");
        let csv_path = cache_dir.join(&file_name);
        fs::write(
            &csv_path,
            "timestamp,open,close,high,low,volume\n1704067200000,1,1,1,1,1\n1706659200000,1,1,1,1,1\n",
        )
        .expect("seed csv");
        let (_, _, _, integrity) =
            scan_csv_integrity(&csv_path, 60_000).expect("fingerprint seeded csv");

        let mut manifest = PolygonCacheManifest::default();
        manifest.datasets.insert(
            dataset_key,
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "AAPL".to_string(),
                polygon_ticker: "AAPL".to_string(),
                timeframe: "1m".to_string(),
                csv_path: csv_path.to_string_lossy().to_string(),
                min_ts: 1_704_067_200_000,
                max_ts: 1_706_659_200_000,
                last_updated_utc_ms: 111,
                row_count: 2,
                adjusted: true,
                integrity: Some(integrity),
            },
        );
        save_manifest(&cache_dir, &manifest).expect("save manifest");
        let before_manifest =
            fs::read_to_string(manifest_path(&cache_dir)).expect("read manifest before");

        let result = download_1m_to_cache(
            "dev.vectoralpha.vectorgrid",
            &cache_dir,
            &PolygonDownloadRequest {
                market: "stocks".to_string(),
                symbol: "AAPL".to_string(),
                from: "2024-01-10".to_string(),
                to: "2024-01-20".to_string(),
                adjusted: Some(true),
                timeframe: Some("1m".to_string()),
            },
            None,
        )
        .expect("full-hit cache read should succeed without api key");

        assert!(result.used_cache_only);
        assert_eq!(result.downloaded_bars, 0);
        let after_manifest =
            fs::read_to_string(manifest_path(&cache_dir)).expect("read manifest after");
        assert_eq!(before_manifest, after_manifest);
    }

    #[test]
    fn validate_cached_dataset_counts_rows_with_broken_numeric_fields_as_parse_errors() {
        let cache_dir = make_temp_dir("vectorta_polygon_validate_parse_errors");
        let csv_path = cache_dir.join("polygon_stocks_TEST_1m.csv");
        let csv_path_s = csv_path.to_string_lossy().to_string();
        fs::write(
            &csv_path,
            "timestamp,open,close,high,low,volume\n1700000000000,1,1,1,1,10\n1700000060000,bad,1,1,1,10\n",
        )
        .expect("write csv");

        let mut manifest = PolygonCacheManifest::default();
        manifest.datasets.insert(
            "stocks:TEST:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "TEST".to_string(),
                polygon_ticker: "TEST".to_string(),
                timeframe: "1m".to_string(),
                csv_path: csv_path_s.clone(),
                min_ts: 1_700_000_000_000,
                max_ts: 1_700_000_060_000,
                last_updated_utc_ms: 1,
                row_count: 2,
                adjusted: true,
                integrity: None,
            },
        );
        save_manifest(&cache_dir, &manifest).expect("save manifest");

        let validated = validate_cached_dataset(&cache_dir, &csv_path_s).expect("validate dataset");
        let integrity = validated.integrity.expect("integrity should be populated");
        assert_eq!(validated.row_count, 1);
        assert_eq!(integrity.parse_errors, 1);
    }

    #[test]
    fn load_manifest_upgrades_legacy_adjusted_stock_entries() {
        let cache_dir = make_temp_dir("vectorta_polygon_manifest_upgrade");
        let manifest_json = r#"{
  "datasets": {
    "stocks:AAPL:1m": {
      "market": "stocks",
      "display_symbol": "AAPL",
      "polygon_ticker": "AAPL",
      "timeframe": "1m",
      "csv_path": "C:/cache/polygon_stocks_AAPL_1m.csv",
      "min_ts": 1700000000000,
      "max_ts": 1700000600000,
      "last_updated_utc_ms": 1700000700000,
      "row_count": 500
    }
  }
}"#;
        fs::write(manifest_path(&cache_dir), manifest_json).expect("write manifest");

        let manifest = load_manifest(&cache_dir).expect("load manifest");
        let dataset = manifest
            .datasets
            .get("stocks:AAPL:1m")
            .expect("legacy adjusted dataset");
        assert!(dataset.adjusted);
    }

    #[test]
    fn get_cache_info_reports_api_key_presence_from_legacy_settings_file() {
        let cache_dir = make_temp_dir("vectorta_polygon_cache_info_key");
        let app_identifier = format!(
            "dev.vectoralpha.vectorgrid.cache-info.{}.{}",
            std::process::id(),
            now_utc_ms().max(1)
        );
        write_settings_file(&cache_dir, "polygon-legacy-file-key").expect("write settings");

        let info = get_cache_info(&app_identifier, &cache_dir).expect("cache info");

        assert!(info.has_api_key);
        assert!(info.credential_status.has_api_key);
        assert_eq!(
            info.credential_status.state,
            PolygonCredentialStorageState::JsonFileFallback
        );
        assert_eq!(info.credential_status.storage_backend, "json_file_fallback");
        assert!(info.credential_status.degraded);
        assert!(info.datasets.is_empty());
        assert_eq!(info.cache_dir, cache_dir.to_string_lossy().to_string());
    }

    #[test]
    fn get_cache_info_returns_manifest_datasets_in_key_order() {
        let cache_dir = make_temp_dir("vectorta_polygon_cache_info_order");
        let app_identifier = format!(
            "dev.vectoralpha.vectorgrid.cache-order.{}.{}",
            std::process::id(),
            now_utc_ms().max(1)
        );
        let mut manifest = PolygonCacheManifest::default();
        manifest.datasets.insert(
            "stocks:MSFT:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "MSFT".to_string(),
                polygon_ticker: "MSFT".to_string(),
                timeframe: "1m".to_string(),
                csv_path: "C:/cache/MSFT.csv".to_string(),
                min_ts: 1,
                max_ts: 2,
                last_updated_utc_ms: 10,
                row_count: 2,
                adjusted: true,
                integrity: None,
            },
        );
        manifest.datasets.insert(
            "stocks:AAPL:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "AAPL".to_string(),
                polygon_ticker: "AAPL".to_string(),
                timeframe: "1m".to_string(),
                csv_path: "C:/cache/AAPL.csv".to_string(),
                min_ts: 1,
                max_ts: 2,
                last_updated_utc_ms: 20,
                row_count: 2,
                adjusted: true,
                integrity: None,
            },
        );
        save_manifest(&cache_dir, &manifest).expect("save manifest");

        let info = get_cache_info(&app_identifier, &cache_dir).expect("cache info");
        let symbols: Vec<&str> = info
            .datasets
            .iter()
            .map(|row| row.display_symbol.as_str())
            .collect();

        assert_eq!(symbols, vec!["AAPL", "MSFT"]);
        assert_eq!(info.cache_dir, cache_dir.to_string_lossy().to_string());
    }

    #[test]
    fn validate_and_repair_require_manifest_membership() {
        let cache_dir = make_temp_dir("vectorta_polygon_manifest_requirement");
        let csv_path = cache_dir.join("detached.csv");
        let csv_path_s = csv_path.to_string_lossy().to_string();
        fs::write(
            &csv_path,
            "timestamp,open,close,high,low,volume\n1700000000000,1,1,1,1,1\n",
        )
        .expect("write csv");

        let validate_err =
            validate_cached_dataset(&cache_dir, &csv_path_s).expect_err("validate should fail");
        let repair_err = repair_cached_dataset("vectorta_test", &cache_dir, &csv_path_s)
            .expect_err("repair should fail");

        assert!(validate_err.contains("dataset not found in Polygon manifest"));
        assert!(repair_err.contains("dataset not found in Polygon manifest"));
    }

    #[test]
    fn cached_dataset_commands_reject_manifest_paths_outside_cache() {
        let cache_dir = make_temp_dir("vectorta_polygon_cache_root");
        let external_dir = make_temp_dir("vectorta_polygon_external");
        let csv_path = external_dir.join("outside.csv");
        let csv_path_s = csv_path.to_string_lossy().to_string();
        fs::write(
            &csv_path,
            "timestamp,open,close,high,low,volume\n1700000000000,1,1,1,1,1\n",
        )
        .expect("write outside csv");

        let mut manifest = PolygonCacheManifest::default();
        manifest.datasets.insert(
            "stocks:TEST:1m".to_string(),
            PolygonDatasetMeta {
                market: "stocks".to_string(),
                display_symbol: "TEST".to_string(),
                polygon_ticker: "TEST".to_string(),
                timeframe: "1m".to_string(),
                csv_path: csv_path_s.clone(),
                min_ts: 1_700_000_000_000,
                max_ts: 1_700_000_000_000,
                last_updated_utc_ms: 1,
                row_count: 1,
                adjusted: true,
                integrity: None,
            },
        );
        save_manifest(&cache_dir, &manifest).expect("save manifest");

        let validate_err =
            validate_cached_dataset(&cache_dir, &csv_path_s).expect_err("validate should fail");
        let repair_err = repair_cached_dataset("vectorta_test", &cache_dir, &csv_path_s)
            .expect_err("repair should fail");
        let top_off_err = top_off_cached_dataset("vectorta_test", &cache_dir, &csv_path_s, None)
            .expect_err("top-off should fail");

        assert!(validate_err.contains("inside the Polygon cache directory"));
        assert!(repair_err.contains("inside the Polygon cache directory"));
        assert!(top_off_err.contains("inside the Polygon cache directory"));
    }
}
