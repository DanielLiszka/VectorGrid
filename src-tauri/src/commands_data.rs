use crate::ma_registry::MaInfo;
use crate::polygon;
use crate::settings;
use crate::state::AppState;
use crate::state::BackgroundTaskState;
use crate::state::LiveCandleStreamTask;
use crate::state::LoadPriceDataResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Manager;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tauri::command]
pub fn load_price_data(
    path: String,
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
) -> Result<LoadPriceDataResponse, String> {
    crate::local_path_policy::ensure_csv_extension(Path::new(path.trim()))?;
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);
    state.load_price_data(&path, &cache_dir)
}

#[tauri::command]
pub fn resample_price_data(
    base_data_id: String,
    timeframe: String,
    state: tauri::State<AppState>,
) -> Result<LoadPriceDataResponse, String> {
    state.resample_loaded_data(&base_data_id, &timeframe)
}

#[derive(Debug, Clone, Serialize)]
pub struct ClearLoadedDataResponse {
    pub cleared: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_ids: Vec<String>,
}

fn clear_loaded_data_impl(
    base_data_id: &str,
    state: &AppState,
) -> Result<ClearLoadedDataResponse, String> {
    let base_id = base_data_id.trim();
    if base_id.is_empty() {
        return Ok(ClearLoadedDataResponse {
            cleared: false,
            removed_ids: Vec::new(),
        });
    }

    match state.invalidate_dataset(base_id) {
        Ok(removed_ids) => Ok(ClearLoadedDataResponse {
            cleared: !removed_ids.is_empty(),
            removed_ids,
        }),
        Err(error) if error.starts_with("unknown data id:") => Ok(ClearLoadedDataResponse {
            cleared: false,
            removed_ids: Vec::new(),
        }),
        Err(error) => Err(error),
    }
}

#[tauri::command]
pub fn clear_loaded_data(
    base_data_id: String,
    state: tauri::State<AppState>,
) -> Result<ClearLoadedDataResponse, String> {
    clear_loaded_data_impl(&base_data_id, state.inner())
}

fn list_moving_averages_impl() -> Vec<MaInfo> {
    crate::ma_registry::list_mas()
}

#[tauri::command]
pub async fn list_moving_averages() -> Result<Vec<MaInfo>, String> {
    tauri::async_runtime::spawn_blocking(list_moving_averages_impl)
        .await
        .map_err(|e| e.to_string())
}

fn list_dynamic_indicators_impl() -> Vec<ta_app_core::IndicatorDescriptor> {
    ta_app_core::list_dynamic_indicator_descriptors_cached().to_vec()
}

#[tauri::command]
pub async fn list_dynamic_indicators() -> Result<Vec<ta_app_core::IndicatorDescriptor>, String> {
    tauri::async_runtime::spawn_blocking(list_dynamic_indicators_impl)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct GpuSpeedupEntry {
    pub indicator_id: String,
    pub indicator_name: String,
    pub speedup: f64,
    pub cpu_ms: f64,
    pub cuda_ms: f64,
}

fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && matches!(chars.peek(), Some('"')) {
                    cell.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(cell.trim().to_string());
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }

    out.push(cell.trim().to_string());
    out
}

fn find_workspace_root_from(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors().take(8) {
        if ancestor.join("Cargo.toml").is_file() && ancestor.join("src-tauri").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            starts.push(p.to_path_buf());
        }
    }
    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let mut seen: HashSet<String> = HashSet::new();
    for start in starts {
        let key = start.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        if let Some(root) = find_workspace_root_from(&start) {
            return Some(root);
        }
    }

    find_repo_data_dir().and_then(|data_dir| data_dir.parent().map(|p| p.to_path_buf()))
}

fn load_gpu_speedups_csv() -> Result<Vec<GpuSpeedupEntry>, String> {
    let Some(root) = find_workspace_root() else {
        return Err(
            "unable to locate workspace root for crates/app_core/data/gpu_speedups.csv".to_string(),
        );
    };
    let csv_path = root
        .join("crates")
        .join("app_core")
        .join("data")
        .join("gpu_speedups.csv");
    if !csv_path.is_file() {
        return Err(format!(
            "gpu speedup table not found: {}",
            csv_path.to_string_lossy()
        ));
    }

    let raw = std::fs::read_to_string(&csv_path).map_err(|e| {
        format!(
            "failed to read gpu speedup table {}: {}",
            csv_path.to_string_lossy(),
            e
        )
    })?;
    let mut out: Vec<GpuSpeedupEntry> = Vec::new();

    for (line_idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line_idx == 0 && trimmed.to_ascii_lowercase().starts_with("indicator_id,") {
            continue;
        }
        let fields = parse_csv_fields(trimmed);
        if fields.len() < 5 {
            continue;
        }
        let indicator_id = fields[0].trim().to_string();
        if indicator_id.is_empty() {
            continue;
        }
        let indicator_name = fields[1].trim().to_string();
        let speedup = match fields[2].trim().parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => continue,
        };
        let cpu_ms = match fields[3].trim().parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => continue,
        };
        let cuda_ms = match fields[4].trim().parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => continue,
        };
        out.push(GpuSpeedupEntry {
            indicator_id,
            indicator_name,
            speedup,
            cpu_ms,
            cuda_ms,
        });
    }

    Ok(out)
}

#[tauri::command]
pub async fn list_gpu_speedups() -> Result<Vec<GpuSpeedupEntry>, String> {
    tauri::async_runtime::spawn_blocking(load_gpu_speedups_csv)
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct SampleCsv {
    pub id: String,
    pub label: String,
    pub path: String,
}

fn sample_csvs_from_root(root: &Path) -> Vec<SampleCsv> {
    let data_dir = find_repo_data_dir_from(root).unwrap_or_else(|| root.join("data"));
    let candidates: [(&str, &str, &str); 3] = [
        ("10k", "10k candles", "10kCandles.csv"),
        (
            "100k",
            "100k candles",
            "bitfinex btc-usd 100,000 candles ends 09-01-24.csv",
        ),
        ("1m", "1M candles", "1MillionCandles.csv"),
    ];

    let mut out: Vec<SampleCsv> = Vec::new();
    for (id, label, rel) in candidates {
        let path = data_dir.join(rel);
        if path.exists() {
            out.push(SampleCsv {
                id: id.to_string(),
                label: label.to_string(),
                path: path.to_string_lossy().to_string(),
            });
        }
    }
    out
}

#[tauri::command]
pub fn list_sample_csvs() -> Vec<SampleCsv> {
    let root: PathBuf = find_workspace_root().unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
    });
    sample_csvs_from_root(&root)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct RepoDataCsvEntry {
    pub csv_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mtime_utc_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct RepoDataCsvList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    pub csv_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub csv_entries: Vec<RepoDataCsvEntry>,
    pub truncated: bool,
}

fn find_repo_data_dir_from(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors().take(6) {
        if ancestor.join("Cargo.toml").is_file() && ancestor.join("data").is_dir() {
            return Some(ancestor.join("data"));
        }
    }
    None
}

fn find_repo_data_dir() -> Option<PathBuf> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            starts.push(p.to_path_buf());
        }
    }

    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let mut seen: HashSet<String> = HashSet::new();
    for s in starts {
        let key = s.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        if let Some(dir) = find_repo_data_dir_from(&s) {
            return Some(dir);
        }
    }
    None
}

fn is_csv_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("csv"))
        .unwrap_or(false)
}

fn list_csvs_in_dir(
    root: &Path,
    recursive: bool,
    max_files: usize,
) -> Result<(Vec<PathBuf>, bool), String> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("failed to read dir {}: {}", dir.display(), e))?;
        let mut child_dirs: Vec<PathBuf> = Vec::new();
        let mut child_files: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    child_dirs.push(path);
                }
                continue;
            }
            if path.is_file() && is_csv_path(&path) {
                child_files.push(path);
            }
        }
        child_files.sort();
        for path in child_files {
            out.push(path);
            if out.len() >= max_files {
                out.sort();
                return Ok((out, true));
            }
        }
        if recursive {
            child_dirs.sort();
            for path in child_dirs.into_iter().rev() {
                stack.push(path);
            }
        }
    }
    out.sort();
    Ok((out, false))
}

fn build_repo_data_csv_list(
    dir: &Path,
    recursive: bool,
    max_files: usize,
    include_identity: bool,
) -> Result<RepoDataCsvList, String> {
    let (paths, truncated) = list_csvs_in_dir(dir, recursive, max_files)?;
    let mut csv_paths: Vec<String> = Vec::with_capacity(paths.len());
    let mut csv_entries: Vec<RepoDataCsvEntry> = if include_identity {
        Vec::with_capacity(paths.len())
    } else {
        Vec::new()
    };

    for path in paths {
        let csv_path = path.to_string_lossy().to_string();
        csv_paths.push(csv_path.clone());
        if include_identity {
            let (size, mtime) = match crate::data_cache::identity_for_source(&path) {
                Ok(id) => (Some(id.size), Some(id.mtime_utc_ms)),
                Err(_) => (None, None),
            };
            csv_entries.push(RepoDataCsvEntry {
                csv_path,
                source_size_bytes: size,
                source_mtime_utc_ms: mtime,
            });
        }
    }

    Ok(RepoDataCsvList {
        dir: Some(dir.to_string_lossy().to_string()),
        csv_paths,
        csv_entries,
        truncated,
    })
}

#[tauri::command]
pub async fn list_repo_data_csvs(
    max_files: Option<usize>,
    recursive: Option<bool>,
    include_identity: Option<bool>,
) -> Result<RepoDataCsvList, String> {
    let max_files = max_files.unwrap_or(20_000).clamp(1, 200_000);
    let recursive = recursive.unwrap_or(false);
    let include_identity = include_identity.unwrap_or(false);

    tauri::async_runtime::spawn_blocking(move || {
        let Some(dir) = find_repo_data_dir() else {
            return Ok(RepoDataCsvList {
                dir: None,
                csv_paths: Vec::new(),
                csv_entries: Vec::new(),
                truncated: false,
            });
        };
        build_repo_data_csv_list(&dir, recursive, max_files, include_identity)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketDataDirInfo {
    pub default_root_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_root_dir: Option<String>,
    pub effective_root_dir: String,
    pub polygon_cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRetentionSettingsDto {
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

impl From<settings::ArtifactRetentionSettings> for ArtifactRetentionSettingsDto {
    fn from(value: settings::ArtifactRetentionSettings) -> Self {
        Self {
            run_keep_newest: value.run_keep_newest,
            meta_keep_newest: value.meta_keep_newest,
            stage5_keep_newest: value.stage5_keep_newest,
            ml_policy_eval_keep_newest: value.ml_policy_eval_keep_newest,
            batch_summary_keep_newest: value.batch_summary_keep_newest,
            campaign_summary_keep_newest: value.campaign_summary_keep_newest,
            options_expression_keep_newest: value.options_expression_keep_newest,
        }
    }
}

impl From<ArtifactRetentionSettingsDto> for settings::ArtifactRetentionSettings {
    fn from(value: ArtifactRetentionSettingsDto) -> Self {
        Self {
            run_keep_newest: value.run_keep_newest,
            meta_keep_newest: value.meta_keep_newest,
            stage5_keep_newest: value.stage5_keep_newest,
            ml_policy_eval_keep_newest: value.ml_policy_eval_keep_newest,
            batch_summary_keep_newest: value.batch_summary_keep_newest,
            campaign_summary_keep_newest: value.campaign_summary_keep_newest,
            options_expression_keep_newest: value.options_expression_keep_newest,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArtifactRetentionValueSet {
    pub run_keep_newest: usize,
    pub meta_keep_newest: usize,
    pub stage5_keep_newest: usize,
    pub ml_policy_eval_keep_newest: usize,
    pub batch_summary_keep_newest: usize,
    pub campaign_summary_keep_newest: usize,
    pub options_expression_keep_newest: usize,
}

impl From<crate::commands::run_artifacts::ArtifactRetentionPolicy> for ArtifactRetentionValueSet {
    fn from(value: crate::commands::run_artifacts::ArtifactRetentionPolicy) -> Self {
        Self {
            run_keep_newest: value.run_keep_newest,
            meta_keep_newest: value.meta_keep_newest,
            stage5_keep_newest: value.stage5_keep_newest,
            ml_policy_eval_keep_newest: value.ml_policy_eval_keep_newest,
            batch_summary_keep_newest: value.batch_summary_keep_newest,
            campaign_summary_keep_newest: value.campaign_summary_keep_newest,
            options_expression_keep_newest: value.options_expression_keep_newest,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArtifactRetentionInfo {
    pub defaults: ArtifactRetentionValueSet,
    pub configured: ArtifactRetentionSettingsDto,
    pub env_overrides: ArtifactRetentionSettingsDto,
    pub effective: ArtifactRetentionValueSet,
}

fn market_data_dir_info(app: &tauri::AppHandle) -> Result<MarketDataDirInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    market_data_dir_info_for_base(&base)
}

fn market_data_dir_info_for_base(base: &Path) -> Result<MarketDataDirInfo, String> {
    let configured = settings::configured_market_data_root_dir(&base)?;
    let effective = settings::resolve_market_data_root_dir(&base)?;
    Ok(MarketDataDirInfo {
        default_root_dir: base.to_string_lossy().to_string(),
        configured_root_dir: configured.map(|p| p.to_string_lossy().to_string()),
        effective_root_dir: effective.to_string_lossy().to_string(),
        polygon_cache_dir: polygon::polygon_cache_dir(&effective)
            .to_string_lossy()
            .to_string(),
    })
}

fn artifact_retention_info(app: &tauri::AppHandle) -> Result<ArtifactRetentionInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    artifact_retention_info_for_base(&base)
}

fn artifact_retention_info_for_base_with_env_overrides(
    base: &Path,
    env_overrides: settings::ArtifactRetentionSettings,
) -> Result<ArtifactRetentionInfo, String> {
    let configured = settings::configured_artifact_retention_settings(base)?;
    let defaults = crate::commands::run_artifacts::artifact_retention_defaults();
    let effective =
        crate::commands::run_artifacts::artifact_retention_policy_for_base_with_env_overrides(
            base,
            &env_overrides,
        );
    Ok(ArtifactRetentionInfo {
        defaults: defaults.into(),
        configured: configured.into(),
        env_overrides: env_overrides.into(),
        effective: effective.into(),
    })
}

fn artifact_retention_info_for_base(base: &Path) -> Result<ArtifactRetentionInfo, String> {
    let env_overrides = crate::commands::run_artifacts::artifact_retention_env_overrides();
    artifact_retention_info_for_base_with_env_overrides(base, env_overrides)
}

#[tauri::command]
pub fn get_market_data_dir_info(app: tauri::AppHandle) -> Result<MarketDataDirInfo, String> {
    market_data_dir_info(&app)
}

#[tauri::command]
pub fn get_artifact_retention_info(app: tauri::AppHandle) -> Result<ArtifactRetentionInfo, String> {
    artifact_retention_info(&app)
}

#[tauri::command]
pub fn clear_market_data_dir(app: tauri::AppHandle) -> Result<MarketDataDirInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    clear_market_data_dir_for_base(&base)
}

fn clear_market_data_dir_for_base(base: &Path) -> Result<MarketDataDirInfo, String> {
    settings::set_market_data_root_dir(base, None)?;
    market_data_dir_info_for_base(base)
}

fn clear_artifact_retention_settings_for_base(
    base: &Path,
) -> Result<ArtifactRetentionInfo, String> {
    settings::set_artifact_retention_settings(
        base,
        &settings::ArtifactRetentionSettings::default(),
    )?;
    artifact_retention_info_for_base(base)
}

#[tauri::command]
pub fn clear_artifact_retention_settings(
    app: tauri::AppHandle,
) -> Result<ArtifactRetentionInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    clear_artifact_retention_settings_for_base(&base)
}

fn probe_directory_writable(dir: &Path) -> Result<(), String> {
    for attempt in 0..8u32 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(u128::from(attempt));
        let probe = dir.join(format!(
            ".vta_write_probe_{}_{}_{}",
            std::process::id(),
            nonce,
            attempt
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
        {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err(format!(
        "failed to verify directory writability: {}",
        dir.display()
    ))
}

fn set_market_data_dir_for_base(
    base: &Path,
    dir: &str,
    move_existing: bool,
) -> Result<MarketDataDirInfo, String> {
    let raw = dir.trim();
    if raw.is_empty() {
        return Err("market data directory is empty".to_string());
    }
    let new_root = PathBuf::from(raw);
    if !new_root.is_absolute() {
        return Err("market data directory must be an absolute path".to_string());
    }

    std::fs::create_dir_all(&new_root).map_err(|e| e.to_string())?;
    if !new_root.is_dir() {
        return Err(format!(
            "market data directory is not a directory: {}",
            new_root.display()
        ));
    }

    probe_directory_writable(&new_root)?;

    let old_root = settings::resolve_market_data_root_dir(&base)?;
    if move_existing && old_root != new_root {
        let old_cache = polygon::polygon_cache_dir(&old_root);
        let new_cache = polygon::polygon_cache_dir(&new_root);
        if new_cache.starts_with(&old_cache) || old_cache.starts_with(&new_cache) {
            return Err("refusing to move polygon cache into itself".to_string());
        }
        if old_cache.exists() {
            if new_cache.exists() {
                let mut it = std::fs::read_dir(&new_cache).map_err(|e| e.to_string())?;
                if it.next().is_some() {
                    return Err(format!(
                        "destination already exists and is not empty: {}",
                        new_cache.display()
                    ));
                }
                let _ = std::fs::remove_dir_all(&new_cache);
            }
            settings::move_dir_all(&old_cache, &new_cache)?;
        }
    }

    settings::set_market_data_root_dir(base, Some(&new_root))?;
    market_data_dir_info_for_base(base)
}

fn set_artifact_retention_settings_for_base(
    base: &Path,
    settings_dto: ArtifactRetentionSettingsDto,
) -> Result<ArtifactRetentionInfo, String> {
    let settings = settings::ArtifactRetentionSettings::from(settings_dto);
    settings::set_artifact_retention_settings(base, &settings)?;
    artifact_retention_info_for_base(base)
}

#[tauri::command]
pub fn set_market_data_dir(
    dir: String,
    move_existing: Option<bool>,
    app: tauri::AppHandle,
) -> Result<MarketDataDirInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    set_market_data_dir_for_base(&base, &dir, move_existing.unwrap_or(false))
}

#[tauri::command]
pub fn set_artifact_retention_settings(
    settings: ArtifactRetentionSettingsDto,
    app: tauri::AppHandle,
) -> Result<ArtifactRetentionInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    set_artifact_retention_settings_for_base(&base, settings)
}

#[tauri::command]
pub fn polygon_get_cache_info(app: tauri::AppHandle) -> Result<polygon::PolygonCacheInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    polygon::get_cache_info(&app.config().identifier, &dir)
}

#[tauri::command]
pub fn polygon_set_api_key(api_key: String, app: tauri::AppHandle) -> Result<(), String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    polygon::set_api_key(&app.config().identifier, &dir, &api_key)
}

#[tauri::command]
pub fn polygon_clear_api_key(app: tauri::AppHandle) -> Result<(), String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    polygon::clear_api_key(&app.config().identifier, &dir)
}

const POLYGON_WS_STOCKS_URL: &str = "wss://socket.polygon.io/stocks";
const POLYGON_WS_DELAYED_STOCKS_URL: &str = "wss://delayed.polygon.io/stocks";
const POLYGON_LIVE_BAR_EVENT: &str = "polygon_live_bar";
const POLYGON_LIVE_STATUS_EVENT: &str = "polygon_live_status";

#[derive(Debug, Clone, Deserialize)]
pub struct PolygonStartLiveCandleStreamRequest {
    pub symbol: String,
    #[serde(default)]
    pub market: Option<String>,
    #[serde(default)]
    pub delayed: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonStartLiveCandleStreamResponse {
    pub started: bool,
    pub market: String,
    pub symbol: String,
    pub delayed: bool,
    pub channel: String,
    pub ws_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_prior: Option<crate::state::LiveCandleStreamInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonStopLiveCandleStreamResponse {
    pub stopped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior: Option<crate::state::LiveCandleStreamInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonLiveCandleStreamStatusResponse {
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<crate::state::LiveCandleStreamInfo>,
}

#[derive(Debug, Clone, Serialize)]
struct PolygonLiveStatusEvent {
    status: String,
    message: String,
    market: String,
    symbol: String,
    delayed: bool,
    channel: String,
    at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
struct PolygonLiveBarEvent {
    market: String,
    symbol: String,
    delayed: bool,
    channel: String,
    timeframe: String,
    time_open_unix_ms: i64,
    time_close_unix_ms: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vwap: Option<f64>,
}

fn now_unix_ms() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(dur.as_millis()).unwrap_or(0)
}

fn normalize_live_stock_symbol(s: &str) -> Result<String, String> {
    let up = s.trim().to_ascii_uppercase().replace(' ', "");
    if up.is_empty() {
        return Err("symbol is required (e.g. AAPL)".to_string());
    }
    let out: String = up
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();
    if out.is_empty() {
        return Err("symbol is invalid (e.g. AAPL)".to_string());
    }
    Ok(out)
}

fn value_as_i64(v: &serde_json::Value, key: &str) -> Option<i64> {
    match v.get(key) {
        Some(serde_json::Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|x| x as i64)),
        Some(serde_json::Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn value_as_f64(v: &serde_json::Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn emit_polygon_live_status(
    app: &tauri::AppHandle,
    market: &str,
    symbol: &str,
    delayed: bool,
    channel: &str,
    status: &str,
    message: impl Into<String>,
) {
    let at_unix_ms = now_unix_ms();
    let message = message.into();
    let state = app.state::<AppState>();
    let _ = state
        .record_live_candle_stream_status(market, symbol, delayed, status, &message, at_unix_ms);
    let payload = PolygonLiveStatusEvent {
        status: status.to_string(),
        message,
        market: market.to_string(),
        symbol: symbol.to_string(),
        delayed,
        channel: channel.to_string(),
        at_unix_ms,
    };
    let _ = app.emit(POLYGON_LIVE_STATUS_EVENT, payload);
}

fn should_force_runner_review_posture(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "closed" | "error"
    )
}

fn disarm_runner_after_nonmanual_feed_interruption(
    app: &tauri::AppHandle,
    status: &str,
    message: &str,
) {
    if should_force_runner_review_posture(status) {
        let _ = super::commands_execution::stop_live_execution_runner_for_interruption(app.clone());
        let _ = super::commands_execution::record_live_session_interruption_and_finalize(
            app, status, message,
        );
    }
}

fn normalize_polygon_live_bar(
    v: &serde_json::Value,
    market: &str,
    symbol: &str,
    delayed: bool,
    channel: &str,
) -> Option<PolygonLiveBarEvent> {
    let ev = v.get("ev")?.as_str()?;
    if ev != "AM" {
        return None;
    }
    let sym = v.get("sym").and_then(|x| x.as_str())?;
    if !symbol.is_empty() && !super::commands_execution::symbol_matches(sym, symbol) {
        return None;
    }
    let open = value_as_f64(v, "o")?;
    let high = value_as_f64(v, "h")?;
    let low = value_as_f64(v, "l")?;
    let close = value_as_f64(v, "c")?;
    if !(open.is_finite() && high.is_finite() && low.is_finite() && close.is_finite()) {
        return None;
    }
    let start = value_as_i64(v, "s")?;
    let end = value_as_i64(v, "e").unwrap_or_else(|| start.saturating_add(60_000 - 1));
    if start <= 0 || end < start {
        return None;
    }
    Some(PolygonLiveBarEvent {
        market: market.to_string(),
        symbol: sym.to_string(),
        delayed,
        channel: channel.to_string(),
        timeframe: "1m".to_string(),
        time_open_unix_ms: start,
        time_close_unix_ms: end,
        open,
        high,
        low,
        close,
        volume: value_as_f64(v, "v"),
        vwap: value_as_f64(v, "vw"),
    })
}

fn handle_polygon_ws_text(
    app: &tauri::AppHandle,
    text: &str,
    market: &str,
    symbol: &str,
    delayed: bool,
    channel: &str,
) {
    let parsed: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };
    let Some(items) = parsed.as_array() else {
        return;
    };
    for item in items {
        if let Some(status) = item.get("status").and_then(|x| x.as_str()) {
            let msg = item
                .get("message")
                .and_then(|x| x.as_str())
                .unwrap_or(status);
            emit_polygon_live_status(app, market, symbol, delayed, channel, status, msg);
            continue;
        }
        if let Some(bar) = normalize_polygon_live_bar(item, market, symbol, delayed, channel) {
            let state = app.state::<AppState>();
            let _ = state.record_live_candle_stream_accepted_bar(
                market,
                &bar.symbol,
                delayed,
                bar.time_open_unix_ms,
                bar.time_close_unix_ms,
                now_unix_ms(),
            );
            let _ = app.emit(POLYGON_LIVE_BAR_EVENT, bar.clone());
            super::commands_execution::on_live_execution_bar(
                app,
                super::commands_execution::LiveExecutionRunnerBar {
                    symbol: bar.symbol.clone(),
                    time_open_unix_ms: bar.time_open_unix_ms,
                    open: bar.open,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    volume: bar.volume,
                },
            );
        }
    }
}

async fn run_polygon_live_candle_stream(
    app: tauri::AppHandle,
    ws_url: String,
    api_key: String,
    market: String,
    symbol: String,
    delayed: bool,
    channel: String,
) {
    emit_polygon_live_status(
        &app,
        &market,
        &symbol,
        delayed,
        &channel,
        "connecting",
        format!("connecting to {ws_url}"),
    );

    let ws = timeout(Duration::from_secs(10), connect_async(ws_url.as_str())).await;
    let (mut stream, _) = match ws {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            emit_polygon_live_status(
                &app,
                &market,
                &symbol,
                delayed,
                &channel,
                "error",
                format!("connect failed: {e}"),
            );
            disarm_runner_after_nonmanual_feed_interruption(
                &app,
                "error",
                &format!("connect failed: {e}"),
            );
            return;
        }
        Err(_) => {
            emit_polygon_live_status(
                &app,
                &market,
                &symbol,
                delayed,
                &channel,
                "error",
                "connect timed out",
            );
            disarm_runner_after_nonmanual_feed_interruption(&app, "error", "connect timed out");
            return;
        }
    };

    let auth_msg = serde_json::json!({
        "action": "auth",
        "params": api_key,
    })
    .to_string();
    let auth_send = timeout(
        Duration::from_secs(5),
        stream.send(Message::Text(auth_msg.into())),
    )
    .await;
    if let Err(_) = auth_send {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            "error",
            "auth send timed out",
        );
        disarm_runner_after_nonmanual_feed_interruption(&app, "error", "auth send timed out");
        return;
    }
    if let Ok(Err(e)) = auth_send {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            "error",
            format!("auth send failed: {e}"),
        );
        disarm_runner_after_nonmanual_feed_interruption(
            &app,
            "error",
            &format!("auth send failed: {e}"),
        );
        return;
    }

    let sub_msg = serde_json::json!({
        "action": "subscribe",
        "params": channel,
    })
    .to_string();
    let subscribe_send = timeout(
        Duration::from_secs(5),
        stream.send(Message::Text(sub_msg.into())),
    )
    .await;
    if let Err(_) = subscribe_send {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            "error",
            "subscribe send timed out",
        );
        disarm_runner_after_nonmanual_feed_interruption(&app, "error", "subscribe send timed out");
        return;
    }
    if let Ok(Err(e)) = subscribe_send {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            "error",
            format!("subscribe send failed: {e}"),
        );
        disarm_runner_after_nonmanual_feed_interruption(
            &app,
            "error",
            &format!("subscribe send failed: {e}"),
        );
        return;
    }

    emit_polygon_live_status(
        &app,
        &market,
        &symbol,
        delayed,
        &channel,
        "subscribed",
        "subscribed to minute aggregates",
    );

    loop {
        match stream.next().await {
            Some(Ok(Message::Text(text))) => {
                handle_polygon_ws_text(&app, text.as_ref(), &market, &symbol, delayed, &channel);
            }
            Some(Ok(Message::Binary(bytes))) => {
                if let Ok(text) = std::str::from_utf8(bytes.as_ref()) {
                    handle_polygon_ws_text(&app, text, &market, &symbol, delayed, &channel);
                }
            }
            Some(Ok(Message::Ping(payload))) => {
                if stream.send(Message::Pong(payload)).await.is_err() {
                    emit_polygon_live_status(
                        &app,
                        &market,
                        &symbol,
                        delayed,
                        &channel,
                        "closed",
                        "pong send failed",
                    );
                    disarm_runner_after_nonmanual_feed_interruption(
                        &app,
                        "closed",
                        "pong send failed",
                    );
                    break;
                }
            }
            Some(Ok(Message::Close(frame))) => {
                let why = frame
                    .map(|f| f.reason.to_string())
                    .unwrap_or_else(|| "remote close".to_string());
                emit_polygon_live_status(&app, &market, &symbol, delayed, &channel, "closed", &why);
                disarm_runner_after_nonmanual_feed_interruption(&app, "closed", &why);
                break;
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                emit_polygon_live_status(
                    &app,
                    &market,
                    &symbol,
                    delayed,
                    &channel,
                    "error",
                    format!("stream read failed: {e}"),
                );
                disarm_runner_after_nonmanual_feed_interruption(
                    &app,
                    "error",
                    &format!("stream read failed: {e}"),
                );
                break;
            }
            None => {
                emit_polygon_live_status(
                    &app,
                    &market,
                    &symbol,
                    delayed,
                    &channel,
                    "closed",
                    "stream ended",
                );
                disarm_runner_after_nonmanual_feed_interruption(&app, "closed", "stream ended");
                break;
            }
        }
    }
}

#[tauri::command]
pub async fn polygon_start_live_candle_stream(
    req: PolygonStartLiveCandleStreamRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PolygonStartLiveCandleStreamResponse, String> {
    let market = req.market.unwrap_or_else(|| "stocks".to_string());
    let market = market.trim().to_ascii_lowercase();
    if market != "stocks" {
        return Err("live candle stream currently supports market=stocks only".to_string());
    }
    let symbol = normalize_live_stock_symbol(&req.symbol)?;
    let delayed = req.delayed.unwrap_or(true);

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    let api_key = polygon::read_api_key(&app.config().identifier, &dir)?.ok_or_else(|| {
        "Polygon API key is missing. Save one in Data > Polygon first.".to_string()
    })?;
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        return Err("Polygon API key is empty. Save one in Data > Polygon first.".to_string());
    }

    let ws_url = if delayed {
        POLYGON_WS_DELAYED_STOCKS_URL
    } else {
        POLYGON_WS_STOCKS_URL
    }
    .to_string();
    let channel = format!("AM.{symbol}");

    let app2 = app.clone();
    let market2 = market.clone();
    let symbol2 = symbol.clone();
    let channel2 = channel.clone();
    let ws_url2 = ws_url.clone();
    let task = tauri::async_runtime::spawn(async move {
        run_polygon_live_candle_stream(app2, ws_url2, api_key, market2, symbol2, delayed, channel2)
            .await;
    });
    let replaced_prior = match state.replace_live_candle_stream(LiveCandleStreamTask {
        handle: task,
        market: market.clone(),
        symbol: symbol.clone(),
        delayed,
    }) {
        Ok(prior) => prior,
        Err((error, task)) => {
            task.handle.abort();
            return Err(error);
        }
    };

    if replaced_prior
        .as_ref()
        .is_some_and(|prior| !super::commands_execution::symbol_matches(&prior.symbol, &symbol))
    {
        let _ = super::commands_execution::stop_live_execution_runner(app.clone());
    }

    emit_polygon_live_status(
        &app,
        &market,
        &symbol,
        delayed,
        &channel,
        "started",
        "live candle stream started",
    );

    Ok(PolygonStartLiveCandleStreamResponse {
        started: true,
        market,
        symbol,
        delayed,
        channel,
        ws_url,
        replaced_prior,
    })
}

#[tauri::command]
pub fn polygon_stop_live_candle_stream(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PolygonStopLiveCandleStreamResponse, String> {
    let prior = state.stop_live_candle_stream()?;
    if let Some(info) = prior.as_ref() {
        let channel = format!("AM.{}", info.symbol);
        emit_polygon_live_status(
            &app,
            &info.market,
            &info.symbol,
            info.delayed,
            &channel,
            "stopped",
            "live candle stream stopped",
        );
        let _ = super::commands_execution::stop_live_execution_runner(app.clone());
    }
    Ok(PolygonStopLiveCandleStreamResponse {
        stopped: prior.is_some(),
        prior,
    })
}

#[tauri::command]
pub fn polygon_live_candle_stream_status(
    state: tauri::State<'_, AppState>,
) -> Result<PolygonLiveCandleStreamStatusResponse, String> {
    let stream = state.live_candle_stream_info()?;
    Ok(PolygonLiveCandleStreamStatusResponse {
        active: stream.as_ref().is_some_and(|info| info.active),
        stream,
    })
}

#[tauri::command]
pub async fn polygon_validate_dataset(
    csv_path: String,
    app: tauri::AppHandle,
) -> Result<polygon::PolygonDatasetMeta, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    tauri::async_runtime::spawn_blocking(move || polygon::validate_cached_dataset(&dir, &csv_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn polygon_repair_dataset(
    csv_path: String,
    app: tauri::AppHandle,
) -> Result<polygon::PolygonDatasetMeta, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    let app_identifier = app.config().identifier.clone();
    tauri::async_runtime::spawn_blocking(move || {
        polygon::repair_cached_dataset(&app_identifier, &dir, &csv_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonDownloadAndLoadResponse {
    pub csv_path: String,
    pub downloaded_bars: usize,
    pub used_cache_only: bool,
    pub base_load: LoadPriceDataResponse,
    #[serde(flatten)]
    pub load: LoadPriceDataResponse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolygonTopOffRequest {
    pub csv_path: String,
    #[serde(default)]
    pub timeframe: Option<String>,
}

fn begin_background_download(
    bg: &BackgroundTaskState,
) -> Result<
    (
        crate::state::BackgroundTaskGuard,
        std::sync::Arc<std::sync::atomic::AtomicBool>,
    ),
    String,
> {
    let guard = bg.try_begin()?;
    let cancel = bg.cancel_token();
    Ok((guard, cancel))
}

#[tauri::command]
pub async fn polygon_download_1m_and_load(
    req: polygon::PolygonDownloadRequest,
    state: tauri::State<'_, AppState>,
    bg: tauri::State<'_, BackgroundTaskState>,
    app: tauri::AppHandle,
) -> Result<PolygonDownloadAndLoadResponse, String> {
    let (_bg_guard, cancel) = begin_background_download(bg.inner())?;
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);
    let app_identifier = app.config().identifier.clone();

    let requested_tf = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("1m")
        .to_string();

    let mut req2 = req.clone();
    req2.timeframe = Some("1m".to_string());

    let cancel2 = cancel.clone();
    let dl = tauri::async_runtime::spawn_blocking(move || {
        polygon::download_1m_to_cache(&app_identifier, &dir, &req2, Some(cancel2.as_ref()))
    })
    .await
    .map_err(|e| e.to_string())??;

    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let csv_path = dl.csv_path.to_string_lossy().to_string();
    let base_load = state.load_price_data(&csv_path, &cache_dir)?;
    let load = if requested_tf.eq_ignore_ascii_case("1m") {
        base_load.clone()
    } else {
        state.resample_loaded_data(&base_load.id, &requested_tf)?
    };
    Ok(PolygonDownloadAndLoadResponse {
        csv_path,
        downloaded_bars: dl.downloaded_bars,
        used_cache_only: dl.used_cache_only,
        base_load,
        load,
    })
}

#[tauri::command]
pub async fn polygon_top_off_dataset(
    req: PolygonTopOffRequest,
    state: tauri::State<'_, AppState>,
    bg: tauri::State<'_, BackgroundTaskState>,
    app: tauri::AppHandle,
) -> Result<PolygonDownloadAndLoadResponse, String> {
    let (_bg_guard, cancel) = begin_background_download(bg.inner())?;
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);
    let app_identifier = app.config().identifier.clone();
    let csv_path = req.csv_path.trim().to_string();
    if csv_path.is_empty() {
        return Err("select a cached Polygon dataset first".to_string());
    }

    let requested_tf = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("1m")
        .to_string();
    let csv_path2 = csv_path.clone();
    let cancel2 = cancel.clone();
    let dl = tauri::async_runtime::spawn_blocking(move || {
        polygon::top_off_cached_dataset(&app_identifier, &dir, &csv_path2, Some(cancel2.as_ref()))
    })
    .await
    .map_err(|e| e.to_string())??;

    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let base_load = state.load_price_data(&csv_path, &cache_dir)?;
    let load = if requested_tf.eq_ignore_ascii_case("1m") {
        base_load.clone()
    } else {
        state.resample_loaded_data(&base_load.id, &requested_tf)?
    };
    Ok(PolygonDownloadAndLoadResponse {
        csv_path,
        downloaded_bars: dl.downloaded_bars,
        used_cache_only: dl.used_cache_only,
        base_load,
        load,
    })
}

#[tauri::command]
pub fn pick_csv_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    app.dialog()
        .file()
        .add_filter("CSV Files", &["csv"])
        .blocking_pick_file()
        .map(|path| {
            path.into_path()
                .map_err(|e| e.to_string())
                .and_then(validate_picked_csv_path)
                .map(|path| path.to_string_lossy().to_string())
        })
        .transpose()
}

fn validate_picked_csv_path(path: PathBuf) -> Result<PathBuf, String> {
    let validated = crate::local_path_policy::canonical_existing_path(&path.to_string_lossy())?;
    crate::local_path_policy::ensure_csv_extension(&validated.canonical)?;
    Ok(validated.canonical)
}

fn normalize_save_csv_path(path: PathBuf) -> Result<PathBuf, String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("csv") => Ok(path),
        Some(_) => Err(format!(
            "save path must use .csv extension: {}",
            path.to_string_lossy()
        )),
        None => {
            let mut with_ext = path;
            with_ext.set_extension("csv");
            Ok(with_ext)
        }
    }
}

#[tauri::command]
pub fn pick_save_csv(
    default_name: Option<String>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut builder = app.dialog().file().add_filter("CSV Files", &["csv"]);
    if let Some(name) = default_name {
        let name = name.trim();
        if !name.is_empty() {
            builder = builder.set_file_name(name);
        }
    }
    builder
        .blocking_save_file()
        .map(|path| {
            path.into_path()
                .map_err(|e| e.to_string())
                .and_then(normalize_save_csv_path)
                .map(|path| path.to_string_lossy().to_string())
        })
        .transpose()
}

#[tauri::command]
pub fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|path| {
            path.into_path()
                .map(|path| path.to_string_lossy().to_string())
                .map_err(|e| e.to_string())
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::artifact_retention_info_for_base;
    use super::artifact_retention_info_for_base_with_env_overrides;
    use super::begin_background_download;
    use super::build_repo_data_csv_list;
    use super::clear_artifact_retention_settings_for_base;
    use super::clear_loaded_data_impl;
    use super::clear_market_data_dir_for_base;
    use super::find_repo_data_dir_from;
    use super::list_csvs_in_dir;
    use super::load_gpu_speedups_csv;
    use super::market_data_dir_info_for_base;
    use super::normalize_polygon_live_bar;
    use super::normalize_save_csv_path;
    use super::parse_csv_fields;
    use super::sample_csvs_from_root;
    use super::set_artifact_retention_settings_for_base;
    use super::set_market_data_dir_for_base;
    use super::should_force_runner_review_posture;
    use super::validate_picked_csv_path;
    use crate::state::AppState;
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use vector_ta::utilities::data_loader::Candles;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        p.push(format!("{prefix}_{}_{}", std::process::id(), uniq));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    fn sample_retention_settings_dto() -> super::ArtifactRetentionSettingsDto {
        super::ArtifactRetentionSettingsDto {
            run_keep_newest: Some(21),
            meta_keep_newest: Some(17),
            stage5_keep_newest: Some(13),
            ml_policy_eval_keep_newest: Some(11),
            batch_summary_keep_newest: Some(19),
            campaign_summary_keep_newest: Some(9),
            options_expression_keep_newest: Some(7),
        }
    }

    #[test]
    fn background_download_slot_rejects_reentry() {
        let bg = crate::state::BackgroundTaskState::default();
        let (_guard, cancel) =
            begin_background_download(&bg).expect("background download slot should acquire");
        assert!(
            !cancel.load(std::sync::atomic::Ordering::SeqCst),
            "background download slot should clear prior cancel intent"
        );
        let err = match begin_background_download(&bg) {
            Ok(_) => panic!("second background download slot acquisition should fail"),
            Err(err) => err,
        };
        assert!(err.contains("background task"));
    }

    #[test]
    fn nonmanual_feed_interruptions_force_review_required_runner_posture() {
        assert!(should_force_runner_review_posture("closed"));
        assert!(should_force_runner_review_posture("error"));
        assert!(should_force_runner_review_posture(" CLOSED "));
        assert!(!should_force_runner_review_posture("stopped"));
        assert!(!should_force_runner_review_posture("started"));
    }

    fn make_candles(len: usize) -> Candles {
        let mut timestamp = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut volume = Vec::with_capacity(len);
        for idx in 0..len {
            let base = 100.0 + idx as f64 * 0.5;
            timestamp.push(1_700_000_000_000i64 + idx as i64 * 60_000);
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.25);
            volume.push(1_000.0 + idx as f64);
        }
        Candles::new(timestamp, open, high, low, close, volume)
    }

    fn write_candles_csv(path: &Path, candles: &Candles) {
        let mut out = String::from("timestamp,open,high,low,close,volume\n");
        for idx in 0..candles.close.len() {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                candles.timestamp[idx],
                candles.open[idx],
                candles.high[idx],
                candles.low[idx],
                candles.close[idx],
                candles.volume[idx]
            ));
        }
        std::fs::write(path, out).expect("write candles csv");
    }

    #[test]
    fn clear_loaded_data_impl_is_idempotent_for_empty_or_unknown_ids() {
        let state = AppState::default();

        let empty = clear_loaded_data_impl("", &state).expect("empty clear");
        let unknown = clear_loaded_data_impl("data-unknown", &state).expect("unknown clear");

        assert!(!empty.cleared);
        assert!(empty.removed_ids.is_empty());
        assert!(!unknown.cleared);
        assert!(unknown.removed_ids.is_empty());
    }

    #[test]
    fn clear_loaded_data_impl_removes_base_dataset_and_descendants() {
        let temp = make_temp_dir("commands_data_clear");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_candles(32));

        let state = AppState::default();
        let base = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load base");
        let derived = state
            .resample_loaded_data(&base.id, "5m")
            .expect("resample base");

        let cleared = clear_loaded_data_impl(&base.id, &state).expect("clear loaded data");

        assert!(cleared.cleared);
        assert!(cleared.removed_ids.contains(&base.id));
        assert!(cleared.removed_ids.contains(&derived.id));
        assert!(state.get_candles(&base.id).is_err());
        assert!(state.get_candles(&derived.id).is_err());
    }

    #[test]
    fn market_data_dir_helpers_report_default_set_and_clear_state() {
        let base = make_temp_dir("commands_data_market_data_info");
        let custom_root = make_temp_dir("commands_data_market_data_custom");

        let default_info = market_data_dir_info_for_base(&base).expect("default market data info");
        assert_eq!(
            default_info.default_root_dir,
            base.to_string_lossy().to_string()
        );
        assert_eq!(default_info.configured_root_dir, None);
        assert_eq!(
            default_info.effective_root_dir,
            base.to_string_lossy().to_string()
        );
        assert_eq!(
            default_info.polygon_cache_dir,
            crate::polygon::polygon_cache_dir(&base)
                .to_string_lossy()
                .to_string()
        );

        let set_info = set_market_data_dir_for_base(
            &base,
            custom_root.to_str().expect("custom root path"),
            false,
        )
        .expect("set market data dir");
        assert_eq!(
            set_info.configured_root_dir,
            Some(custom_root.to_string_lossy().to_string())
        );
        assert_eq!(
            set_info.effective_root_dir,
            custom_root.to_string_lossy().to_string()
        );
        assert_eq!(
            set_info.polygon_cache_dir,
            crate::polygon::polygon_cache_dir(&custom_root)
                .to_string_lossy()
                .to_string()
        );

        let cleared = clear_market_data_dir_for_base(&base).expect("clear market data dir");
        assert_eq!(cleared.configured_root_dir, None);
        assert_eq!(
            cleared.effective_root_dir,
            base.to_string_lossy().to_string()
        );
    }

    #[test]
    fn set_market_data_dir_for_base_rejects_empty_and_relative_roots() {
        let base = make_temp_dir("commands_data_market_data_invalid");

        let empty_err = set_market_data_dir_for_base(&base, "   ", false)
            .expect_err("empty market data dir should fail");
        assert!(empty_err.contains("market data directory is empty"));

        let relative_err = set_market_data_dir_for_base(&base, "relative\\path", false)
            .expect_err("relative market data dir should fail");
        assert!(relative_err.contains("must be an absolute path"));
    }

    #[test]
    fn artifact_retention_helpers_report_default_set_and_clear_state() {
        let base = make_temp_dir("commands_data_artifact_retention_info");

        let defaults =
            artifact_retention_info_for_base(&base).expect("default artifact retention info");
        assert_eq!(
            defaults.defaults,
            super::ArtifactRetentionValueSet::from(
                crate::commands::run_artifacts::artifact_retention_defaults()
            )
        );
        assert_eq!(
            defaults.configured,
            super::ArtifactRetentionSettingsDto::from(
                crate::settings::ArtifactRetentionSettings::default()
            )
        );
        assert_eq!(
            defaults.effective,
            super::ArtifactRetentionValueSet::from(
                crate::commands::run_artifacts::artifact_retention_defaults()
            )
        );

        let configured =
            set_artifact_retention_settings_for_base(&base, sample_retention_settings_dto())
                .expect("set artifact retention settings");
        assert_eq!(configured.configured.run_keep_newest, Some(21));
        assert_eq!(configured.effective.run_keep_newest, 21);
        assert_eq!(configured.effective.campaign_summary_keep_newest, 9);

        let env_info = artifact_retention_info_for_base_with_env_overrides(
            &base,
            crate::settings::ArtifactRetentionSettings {
                run_keep_newest: Some(5),
                options_expression_keep_newest: Some(3),
                ..crate::settings::ArtifactRetentionSettings::default()
            },
        )
        .expect("artifact retention info with env overrides");
        assert_eq!(env_info.configured.run_keep_newest, Some(21));
        assert_eq!(env_info.env_overrides.run_keep_newest, Some(5));
        assert_eq!(
            env_info.env_overrides.options_expression_keep_newest,
            Some(3)
        );
        assert_eq!(env_info.effective.run_keep_newest, 5);
        assert_eq!(env_info.effective.options_expression_keep_newest, 3);

        let cleared = clear_artifact_retention_settings_for_base(&base)
            .expect("clear artifact retention settings");
        assert!(cleared.configured.run_keep_newest.is_none());
        assert_eq!(
            cleared.effective,
            super::ArtifactRetentionValueSet::from(
                crate::commands::run_artifacts::artifact_retention_defaults()
            )
        );
    }

    #[test]
    fn set_artifact_retention_settings_for_base_rejects_zero_values() {
        let base = make_temp_dir("commands_data_artifact_retention_invalid");
        let mut settings = sample_retention_settings_dto();
        settings.batch_summary_keep_newest = Some(0);

        let err = set_artifact_retention_settings_for_base(&base, settings)
            .expect_err("zero retention setting should fail");
        assert!(err.contains("artifact_retention.batch_summary_keep_newest"));
    }

    #[test]
    fn set_market_data_dir_for_base_moves_polygon_cache_when_requested() {
        let base = make_temp_dir("commands_data_market_data_move");
        let new_root = make_temp_dir("commands_data_market_data_move_target");
        let old_cache = crate::polygon::polygon_cache_dir(&base);
        let old_file = old_cache.join("bars").join("manifest.json");

        std::fs::create_dir_all(old_file.parent().expect("old cache parent"))
            .expect("create old cache parent");
        std::fs::write(&old_file, br#"{"dataset":"cached"}"#).expect("write old cache file");

        let info =
            set_market_data_dir_for_base(&base, new_root.to_str().expect("new root path"), true)
                .expect("move polygon cache to new root");
        let new_cache = crate::polygon::polygon_cache_dir(&new_root);
        let new_file = new_cache.join("bars").join("manifest.json");

        assert_eq!(
            info.effective_root_dir,
            new_root.to_string_lossy().to_string()
        );
        assert!(
            !old_cache.exists(),
            "old polygon cache should be moved away"
        );
        assert_eq!(
            std::fs::read(&new_file).expect("read moved cache file"),
            br#"{"dataset":"cached"}"#
        );
    }

    #[test]
    fn set_market_data_dir_for_base_rejects_nonempty_destination_cache_on_move() {
        let base = make_temp_dir("commands_data_market_data_move_conflict");
        let new_root = make_temp_dir("commands_data_market_data_move_conflict_target");
        let old_cache = crate::polygon::polygon_cache_dir(&base);
        let new_cache = crate::polygon::polygon_cache_dir(&new_root);

        std::fs::create_dir_all(&old_cache).expect("create old cache dir");
        std::fs::write(old_cache.join("cache.json"), br#"{"old":true}"#)
            .expect("write old cache file");
        std::fs::create_dir_all(&new_cache).expect("create new cache dir");
        std::fs::write(new_cache.join("cache.json"), br#"{"new":true}"#)
            .expect("write new cache file");

        let err =
            set_market_data_dir_for_base(&base, new_root.to_str().expect("new root path"), true)
                .expect_err("nonempty destination cache should fail");

        assert!(err.contains("destination already exists and is not empty"));
        assert!(
            old_cache.exists(),
            "old polygon cache should remain in place"
        );
        assert!(
            new_cache.join("cache.json").exists(),
            "destination cache should remain untouched",
        );
    }

    #[test]
    fn find_repo_data_dir_from_prefers_workspace_data_folder() {
        let temp = make_temp_dir("commands_data_repo_root");
        let data_dir = temp.join("data");
        let nested = temp.join("src-tauri").join("src").join("deep");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(temp.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("write workspace cargo");

        let resolved = find_repo_data_dir_from(&nested).expect("find repo data dir");
        assert_eq!(resolved, data_dir);
    }

    #[test]
    fn find_repo_data_dir_from_returns_none_without_workspace_data_folder() {
        let temp = make_temp_dir("commands_data_repo_root_missing");
        let nested = temp.join("src-tauri").join("src");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(temp.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("write workspace cargo");

        assert!(find_repo_data_dir_from(&nested).is_none());
    }

    #[test]
    fn sample_csvs_from_root_skips_missing_files_and_preserves_order() {
        let temp = make_temp_dir("commands_data_sample_csvs");
        let data_dir = temp.join("data");
        std::fs::create_dir_all(&data_dir).expect("create sample data dir");
        std::fs::write(data_dir.join("10kCandles.csv"), "timestamp,close\n1,1\n")
            .expect("write 10k sample");
        std::fs::write(
            data_dir.join("bitfinex btc-usd 100,000 candles ends 09-01-24.csv"),
            "timestamp,close\n1,1\n",
        )
        .expect("write 100k sample");

        let rows = sample_csvs_from_root(&temp);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, vec!["10k", "100k"]);
        assert_eq!(rows[0].label, "10k candles");
        assert_eq!(
            Path::new(&rows[0].path)
                .file_name()
                .and_then(|value| value.to_str()),
            Some("10kCandles.csv")
        );
    }

    #[test]
    fn list_csvs_in_dir_respects_recursive_flag_and_sorted_output() {
        let temp = make_temp_dir("commands_data_repo_csvs");
        let nested = temp.join("nested");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(temp.join("b.csv"), "timestamp,close\n1,1\n").expect("write b");
        std::fs::write(temp.join("a.csv"), "timestamp,close\n1,1\n").expect("write a");
        std::fs::write(nested.join("c.csv"), "timestamp,close\n1,1\n").expect("write c");
        std::fs::write(nested.join("ignore.txt"), "not csv").expect("write txt");

        let (shallow, shallow_truncated) =
            list_csvs_in_dir(&temp, false, 10).expect("list shallow csvs");
        let shallow_names: Vec<String> = shallow
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(shallow_names, vec!["a.csv", "b.csv"]);
        assert!(!shallow_truncated);

        let (recursive, recursive_truncated) =
            list_csvs_in_dir(&temp, true, 10).expect("list recursive csvs");
        let recursive_names: Vec<String> = recursive
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(recursive_names, vec!["a.csv", "b.csv", "c.csv"]);
        assert!(!recursive_truncated);
    }

    #[test]
    fn list_csvs_in_dir_truncates_at_the_requested_cap() {
        let temp = make_temp_dir("commands_data_repo_csvs_cap");
        std::fs::write(temp.join("c.csv"), "timestamp,close\n1,1\n").expect("write c");
        std::fs::write(temp.join("a.csv"), "timestamp,close\n1,1\n").expect("write a");
        std::fs::write(temp.join("b.csv"), "timestamp,close\n1,1\n").expect("write b");

        let (paths, truncated) = list_csvs_in_dir(&temp, false, 2).expect("list capped csvs");
        let names: Vec<String> = paths
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.csv", "b.csv"]);
        assert!(truncated);
    }

    #[test]
    fn validate_picked_csv_path_accepts_existing_csv_and_rejects_other_extensions() {
        let temp = make_temp_dir("commands_data_pick_csv");
        let csv = temp.join("picked.CSV");
        let txt = temp.join("picked.txt");
        std::fs::write(&csv, "timestamp,close\n1,1\n").expect("write csv");
        std::fs::write(&txt, "timestamp,close\n1,1\n").expect("write txt");

        let validated = validate_picked_csv_path(csv.clone()).expect("csv accepted");
        assert_eq!(validated, csv.canonicalize().expect("canonical csv"));

        let err = validate_picked_csv_path(txt).expect_err("txt should fail");
        assert!(err.contains(".csv"));
    }

    #[test]
    fn normalize_save_csv_path_appends_missing_extension_and_rejects_unsupported_extension() {
        let bare = PathBuf::from("C:/exports/vectorgrid_report");
        assert_eq!(
            normalize_save_csv_path(bare).expect("bare path gets csv"),
            PathBuf::from("C:/exports/vectorgrid_report.csv")
        );
        assert_eq!(
            normalize_save_csv_path(PathBuf::from("C:/exports/vectorgrid_report.CSV"))
                .expect("existing csv kept"),
            PathBuf::from("C:/exports/vectorgrid_report.CSV")
        );

        let err = normalize_save_csv_path(PathBuf::from("C:/exports/vectorgrid_report.xlsx"))
            .expect_err("xlsx should fail");
        assert!(err.contains(".csv"));
    }

    #[test]
    fn build_repo_data_csv_list_populates_identity_when_requested() {
        let temp = make_temp_dir("commands_data_repo_identity");
        let alpha = temp.join("alpha.csv");
        let beta = temp.join("beta.csv");
        std::fs::write(&alpha, "timestamp,close\n1,1\n").expect("write alpha csv");
        std::fs::write(&beta, "timestamp,close\n2,2\n").expect("write beta csv");

        let without_identity =
            build_repo_data_csv_list(&temp, false, 10, false).expect("build repo csv list");
        assert_eq!(
            without_identity.csv_paths,
            vec![
                alpha.to_string_lossy().to_string(),
                beta.to_string_lossy().to_string()
            ]
        );
        assert!(without_identity.csv_entries.is_empty());
        assert!(!without_identity.truncated);

        let with_identity =
            build_repo_data_csv_list(&temp, false, 10, true).expect("build repo csv list");
        assert_eq!(with_identity.csv_entries.len(), 2);
        assert_eq!(
            with_identity.csv_entries[0].csv_path,
            with_identity.csv_paths[0]
        );
        assert!(with_identity.csv_entries[0].source_size_bytes.is_some());
        assert!(with_identity.csv_entries[0].source_mtime_utc_ms.is_some());
        assert_eq!(with_identity.dir, Some(temp.to_string_lossy().to_string()));
    }

    #[test]
    fn parse_csv_fields_handles_quoted_commas_and_escaped_quotes() {
        let fields = parse_csv_fields(
            "alma,\"ALMA, fast\",\"42.9\",\"158.25\",\"3.68\",\"quoted \"\"note\"\"\"",
        );
        assert_eq!(
            fields,
            vec![
                "alma".to_string(),
                "ALMA, fast".to_string(),
                "42.9".to_string(),
                "158.25".to_string(),
                "3.68".to_string(),
                "quoted \"note\"".to_string(),
            ]
        );
    }

    #[test]
    fn load_gpu_speedups_csv_reads_workspace_speedup_table() {
        let rows = load_gpu_speedups_csv().expect("load gpu speedups csv");
        assert!(
            !rows.is_empty(),
            "expected at least one GPU speedup row in the workspace csv"
        );
        for row in rows {
            assert!(
                !row.indicator_id.trim().is_empty(),
                "indicator id should not be blank"
            );
            assert!(row.speedup.is_finite(), "speedup should stay finite");
            assert!(row.cpu_ms.is_finite(), "cpu_ms should stay finite");
            assert!(row.cuda_ms.is_finite(), "cuda_ms should stay finite");
        }
    }

    #[test]
    fn list_moving_averages_returns_nonempty_unique_ids() {
        let rows = super::list_moving_averages_impl();
        assert!(!rows.is_empty(), "expected at least one moving average");

        let mut ids = std::collections::HashSet::new();
        for row in rows {
            let id = row.id.trim();
            assert!(!id.is_empty(), "moving average id should not be blank");
            assert!(
                ids.insert(id.to_string()),
                "moving average ids should be unique"
            );
        }
    }

    #[test]
    fn list_dynamic_indicators_returns_only_dynamic_strategy_eligible_descriptors() {
        let rows = super::list_dynamic_indicators_impl();
        assert!(!rows.is_empty(), "expected at least one dynamic indicator");

        let expected = ta_app_core::list_dynamic_indicator_descriptors_cached();
        assert_eq!(rows.len(), expected.len());

        let mut ids = std::collections::HashSet::new();
        for row in rows {
            let id = row.id.trim();
            assert!(!id.is_empty(), "indicator id should not be blank");
            assert!(
                row.dynamic_strategy_eligible,
                "exported dynamic indicator should be eligible"
            );
            assert!(ids.insert(id.to_string()), "indicator ids should be unique");
        }
    }

    #[test]
    fn normalize_polygon_live_bar_preserves_symbol_punctuation() {
        let accepted = json!({
            "ev": "AM",
            "sym": "BRK.B",
            "o": 10.0,
            "h": 11.0,
            "l": 9.5,
            "c": 10.5,
            "s": 1_700_000_000_000i64,
        });
        let rejected = json!({
            "ev": "AM",
            "sym": "BRKB",
            "o": 10.0,
            "h": 11.0,
            "l": 9.5,
            "c": 10.5,
            "s": 1_700_000_000_000i64,
        });

        assert!(
            normalize_polygon_live_bar(&accepted, "stocks", "BRK.B", true, "AM.BRK.B").is_some()
        );
        assert!(
            normalize_polygon_live_bar(&rejected, "stocks", "BRK.B", true, "AM.BRK.B").is_none()
        );
    }

    #[test]
    fn normalize_polygon_live_bar_requires_vendor_symbol_and_finite_prices() {
        let missing_symbol = json!({
            "ev": "AM",
            "o": 10.0,
            "h": 11.0,
            "l": 9.5,
            "c": 10.5,
            "s": 1_700_000_000_000i64,
        });
        let invalid_close = json!({
            "ev": "AM",
            "sym": "SPY",
            "o": 10.0,
            "h": 11.0,
            "l": 9.5,
            "c": "NaN",
            "s": 1_700_000_000_000i64,
        });

        assert!(
            normalize_polygon_live_bar(&missing_symbol, "stocks", "SPY", true, "AM.SPY").is_none()
        );
        assert!(
            normalize_polygon_live_bar(&invalid_close, "stocks", "SPY", true, "AM.SPY").is_none()
        );
    }

    #[test]
    fn normalize_polygon_live_bar_rejects_inverted_time_window() {
        let inverted = json!({
            "ev": "AM",
            "sym": "SPY",
            "o": 10.0,
            "h": 11.0,
            "l": 9.5,
            "c": 10.5,
            "s": 1_700_000_060_000i64,
            "e": 1_700_000_000_000i64,
        });

        assert!(normalize_polygon_live_bar(&inverted, "stocks", "SPY", true, "AM.SPY").is_none());
    }
}
