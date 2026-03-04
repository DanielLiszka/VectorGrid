use crate::ma_registry::MaInfo;
use crate::polygon;
use crate::settings;
use crate::state::AppState;
use crate::state::LiveCandleStreamTask;
use crate::state::LoadPriceDataResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Manager;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tauri::command]
pub fn load_price_data(
    path: String,
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
) -> Result<LoadPriceDataResponse, String> {
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

#[tauri::command]
pub fn list_moving_averages() -> Vec<MaInfo> {
    crate::ma_registry::list_mas()
}

#[tauri::command]
pub fn list_dynamic_indicators() -> Vec<ta_app_core::IndicatorDescriptor> {
    ta_app_core::list_dynamic_indicator_descriptors_cached().to_vec()
}

#[derive(Debug, Clone, Serialize)]
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
        return Err("unable to locate workspace root for gpu_speedups.csv".to_string());
    };
    let csv_path = root.join("gpu_speedups.csv");
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

#[derive(Debug, Clone, Serialize)]
pub struct SampleCsv {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[tauri::command]
pub fn list_sample_csvs() -> Vec<SampleCsv> {
    let root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let candidates: [(&str, &str, &str); 3] = [
        ("10k", "10k candles", "src/data/10kCandles.csv"),
        (
            "100k",
            "100k candles",
            "src/data/bitfinex btc-usd 100,000 candles ends 09-01-24.csv",
        ),
        ("1m", "1M candles", "src/data/1MillionCandles.csv"),
    ];

    let mut out: Vec<SampleCsv> = Vec::new();
    for (id, label, rel) in candidates {
        let path = root.join(rel);
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

#[derive(Debug, Clone, Serialize)]
pub struct RepoDataCsvEntry {
    pub csv_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mtime_utc_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoDataCsvList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    pub csv_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub csv_entries: Vec<RepoDataCsvEntry>,
    pub truncated: bool,
}

fn find_repo_data_dir_from(start: &Path) -> Option<PathBuf> {
    // Walk up a few levels looking for a Cargo workspace root with a sibling `data/` folder.
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
    // Dev fallback (when running from source).
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
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    stack.push(path);
                }
                continue;
            }
            if path.is_file() && is_csv_path(&path) {
                out.push(path);
                if out.len() >= max_files {
                    out.sort();
                    return Ok((out, true));
                }
            }
        }
    }
    out.sort();
    Ok((out, false))
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

        let (paths, truncated) = list_csvs_in_dir(&dir, recursive, max_files)?;
        let mut csv_paths: Vec<String> = Vec::with_capacity(paths.len());
        let mut csv_entries: Vec<RepoDataCsvEntry> = if include_identity {
            Vec::with_capacity(paths.len())
        } else {
            Vec::new()
        };

        for p in paths {
            let s = p.to_string_lossy().to_string();
            csv_paths.push(s.clone());
            if include_identity {
                let (size, mtime) = match crate::data_cache::identity_for_source(&p) {
                    Ok(id) => (Some(id.size), Some(id.mtime_utc_ms)),
                    Err(_) => (None, None),
                };
                csv_entries.push(RepoDataCsvEntry {
                    csv_path: s,
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

fn market_data_dir_info(app: &tauri::AppHandle) -> Result<MarketDataDirInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn get_market_data_dir_info(app: tauri::AppHandle) -> Result<MarketDataDirInfo, String> {
    market_data_dir_info(&app)
}

#[tauri::command]
pub fn clear_market_data_dir(app: tauri::AppHandle) -> Result<MarketDataDirInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    settings::set_market_data_root_dir(&base, None)?;
    market_data_dir_info(&app)
}

#[tauri::command]
pub fn set_market_data_dir(
    dir: String,
    move_existing: Option<bool>,
    app: tauri::AppHandle,
) -> Result<MarketDataDirInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;

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

    // Best-effort check that it's writable.
    let probe = new_root.join(".vta_write_probe");
    std::fs::write(&probe, b"ok").map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&probe);

    let old_root = settings::resolve_market_data_root_dir(&base)?;
    if move_existing.unwrap_or(false) && old_root != new_root {
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

    settings::set_market_data_root_dir(&base, Some(&new_root))?;
    market_data_dir_info(&app)
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

fn normalize_symbol_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
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
    let payload = PolygonLiveStatusEvent {
        status: status.to_string(),
        message: message.into(),
        market: market.to_string(),
        symbol: symbol.to_string(),
        delayed,
        channel: channel.to_string(),
        at_unix_ms: now_unix_ms(),
    };
    let _ = app.emit(POLYGON_LIVE_STATUS_EVENT, payload);
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
    let sym = v.get("sym").and_then(|x| x.as_str()).unwrap_or(symbol);
    if !symbol.is_empty() && normalize_symbol_key(sym) != normalize_symbol_key(symbol) {
        return None;
    }
    let open = value_as_f64(v, "o")?;
    let high = value_as_f64(v, "h")?;
    let low = value_as_f64(v, "l")?;
    let close = value_as_f64(v, "c")?;
    let start = value_as_i64(v, "s")?;
    let end = value_as_i64(v, "e").unwrap_or_else(|| start.saturating_add(60_000 - 1));
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

    let ws = connect_async(ws_url.as_str()).await;
    let (mut stream, _) = match ws {
        Ok(v) => v,
        Err(e) => {
            emit_polygon_live_status(
                &app,
                &market,
                &symbol,
                delayed,
                &channel,
                "error",
                format!("connect failed: {e}"),
            );
            return;
        }
    };

    let auth_msg = serde_json::json!({
        "action": "auth",
        "params": api_key,
    })
    .to_string();
    if let Err(e) = stream.send(Message::Text(auth_msg.into())).await {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            "error",
            format!("auth send failed: {e}"),
        );
        return;
    }

    let sub_msg = serde_json::json!({
        "action": "subscribe",
        "params": channel,
    })
    .to_string();
    if let Err(e) = stream.send(Message::Text(sub_msg.into())).await {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            "error",
            format!("subscribe send failed: {e}"),
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
                    break;
                }
            }
            Some(Ok(Message::Close(frame))) => {
                let why = frame
                    .map(|f| f.reason.to_string())
                    .unwrap_or_else(|| "remote close".to_string());
                emit_polygon_live_status(&app, &market, &symbol, delayed, &channel, "closed", why);
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
    let replaced_prior = state.replace_live_candle_stream(LiveCandleStreamTask {
        handle: task,
        market: market.clone(),
        symbol: symbol.clone(),
        delayed,
    })?;

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
) -> Result<PolygonStopLiveCandleStreamResponse, String> {
    let prior = state.stop_live_candle_stream()?;
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
        active: stream.is_some(),
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

#[tauri::command]
pub async fn polygon_download_1m_and_load(
    req: polygon::PolygonDownloadRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PolygonDownloadAndLoadResponse, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let root = settings::resolve_market_data_root_dir(&base)?;
    let dir = polygon::polygon_cache_dir(&root);
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);
    let app_identifier = app.config().identifier.clone();

    // Always fetch/cache 1m candles; derive larger timeframes from that in-memory.
    let requested_tf = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("1m")
        .to_string();

    let mut req2 = req.clone();
    req2.timeframe = Some("1m".to_string());

    let dl = tauri::async_runtime::spawn_blocking(move || {
        polygon::download_1m_to_cache(&app_identifier, &dir, &req2)
    })
    .await
    .map_err(|e| e.to_string())??;

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

#[cfg(target_os = "windows")]
fn utf16z_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[tauri::command]
pub fn pick_csv_file() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
            OFN_HIDEREADONLY, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };

        let filter = "CSV Files (*.csv)\0*.csv\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut file_buf = vec![0u16; 8192];

        let mut ofn = OPENFILENAMEW::default();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;

        let ok = unsafe { GetOpenFileNameW(&mut ofn) };
        if ok != 0 {
            return Ok(Some(utf16z_to_string(&file_buf)));
        }

        let err = unsafe { CommDlgExtendedError() };
        if err == 0 {
            return Ok(None);
        }
        return Err(format!(
            "Open file dialog failed (CommDlgExtendedError={err})."
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native file dialogs are only implemented on Windows in this demo. Please paste a CSV path.".to_string())
    }
}

#[tauri::command]
pub fn pick_save_csv(default_name: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            CommDlgExtendedError, GetSaveFileNameW, OFN_EXPLORER, OFN_HIDEREADONLY,
            OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };

        let filter = "CSV Files (*.csv)\0*.csv\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        let defext = "csv\0".encode_utf16().collect::<Vec<u16>>();

        let mut file_buf = vec![0u16; 8192];
        if let Some(name) = default_name {
            let name = name.trim();
            if !name.is_empty() {
                let w = name.encode_utf16().collect::<Vec<u16>>();
                let max = file_buf.len().saturating_sub(1);
                let n = w.len().min(max);
                file_buf[..n].copy_from_slice(&w[..n]);
                file_buf[n] = 0;
            }
        }

        let mut ofn = OPENFILENAMEW::default();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrDefExt = defext.as_ptr();
        ofn.Flags = OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_OVERWRITEPROMPT;

        let ok = unsafe { GetSaveFileNameW(&mut ofn) };
        if ok != 0 {
            return Ok(Some(utf16z_to_string(&file_buf)));
        }

        let err = unsafe { CommDlgExtendedError() };
        if err == 0 {
            return Ok(None);
        }
        return Err(format!(
            "Save file dialog failed (CommDlgExtendedError={err})."
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = default_name;
        Err("Native file dialogs are only implemented on Windows in this demo. Please type an export path.".to_string())
    }
}

#[tauri::command]
pub fn pick_folder() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Com::{
            CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
        };
        use windows_sys::Win32::UI::Shell::{
            SHBrowseForFolderW, SHGetPathFromIDListW, BIF_EDITBOX, BIF_NEWDIALOGSTYLE,
            BIF_RETURNONLYFSDIRS, BROWSEINFOW,
        };

        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED as u32) };
        let did_init = hr >= 0;

        let title = "Select a folder\0".encode_utf16().collect::<Vec<u16>>();
        let mut display_name_buf = vec![0u16; 260];

        let mut bi: BROWSEINFOW = unsafe { std::mem::zeroed() };
        bi.pszDisplayName = display_name_buf.as_mut_ptr();
        bi.lpszTitle = title.as_ptr();
        bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX;

        let pidl = unsafe { SHBrowseForFolderW(&mut bi) };
        if pidl.is_null() {
            if did_init {
                unsafe { CoUninitialize() };
            }
            return Ok(None);
        }

        let mut path_buf = vec![0u16; 8192];
        let ok = unsafe { SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) };
        unsafe { CoTaskMemFree(pidl as _) };
        if did_init {
            unsafe { CoUninitialize() };
        }

        if ok == 0 {
            return Err("Folder dialog failed (no path returned).".to_string());
        }
        return Ok(Some(utf16z_to_string(&path_buf)));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native folder dialogs are only implemented on Windows in this demo.".to_string())
    }
}
