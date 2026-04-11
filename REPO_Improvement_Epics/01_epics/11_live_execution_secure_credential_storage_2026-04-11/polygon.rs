use chrono::{Duration, NaiveDate};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "polygon_settings.json";
const MANIFEST_FILE: &str = "polygon_manifest.json";
const API_BASE: &str = "https://api.polygon.io";
const BARS_PER_REQUEST_LIMIT: u32 = 50_000;
const CHUNK_DAYS: i64 = 30;
const KEYRING_USERNAME: &str = "polygon_api_key";
const FREE_PLAN_LOOKBACK_DAYS: i64 = 365 * 2;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolygonCacheManifest {
    pub datasets: BTreeMap<String, PolygonDatasetMeta>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonCacheInfo {
    pub cache_dir: String,
    pub has_api_key: bool,
    pub datasets: Vec<PolygonDatasetMeta>,
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

fn env_api_key_fallback() -> Option<String> {
    for name in ["POLYGON_KEY", "POLYGON_API_KEY"] {
        if let Ok(raw) = std::env::var(name) {
            let key = raw.trim().to_string();
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    let cwd = std::env::current_dir().ok()?;
    dotenv_api_key_fallback_from_dir(&cwd)
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
        _ => {
            // Keep the legacy fallback file in place unless the keyring round-trip is proven.
        }
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
        // Fallback to a local file if keyring is unavailable.
        write_settings_file(cache_dir, key)?;
        return Ok(());
    }

    // Best-effort cleanup of legacy plaintext key storage.
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
    let service = keyring_service_name(app_identifier);
    if let Ok(entry) = keyring::Entry::new(&service, KEYRING_USERNAME) {
        if let Ok(key) = entry.get_password() {
            let k = key.trim().to_string();
            if !k.is_empty() {
                return Ok(Some(k));
            }
        }
    }

    // Fallback (and migrate) from legacy disk storage.
    let st = read_settings_file(cache_dir)?;
    let Some(st) = st else {
        return Ok(env_api_key_fallback());
    };
    let key = st.api_key.trim().to_string();
    if key.is_empty() {
        return Ok(env_api_key_fallback());
    }

    maybe_migrate_settings_key_to_keyring(app_identifier, cache_dir, &key);

    Ok(Some(key))
}

fn load_manifest(cache_dir: &Path) -> Result<PolygonCacheManifest, String> {
    let p = manifest_path(cache_dir);
    if !p.exists() {
        return Ok(PolygonCacheManifest::default());
    }
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let mut m: PolygonCacheManifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    // Back-compat: earlier manifests didn't persist this and only supported adjusted stocks.
    for (k, ds) in m.datasets.iter_mut() {
        if ds.market == "stocks" && !ds.adjusted && k.starts_with("stocks:") {
            ds.adjusted = true;
        }
    }
    Ok(m)
}

fn save_manifest(cache_dir: &Path, m: &PolygonCacheManifest) -> Result<(), String> {
    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    let p = manifest_path(cache_dir);
    let tmp = p.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    if p.exists() {
        let _ = fs::remove_file(&p);
    }
    fs::rename(tmp, p).map_err(|e| e.to_string())
}

pub fn get_cache_info(app_identifier: &str, cache_dir: &Path) -> Result<PolygonCacheInfo, String> {
    let api_key = read_api_key(app_identifier, cache_dir)?;
    let manifest = load_manifest(cache_dir)?;
    Ok(PolygonCacheInfo {
        cache_dir: cache_dir.to_string_lossy().to_string(),
        has_api_key: api_key.is_some(),
        datasets: manifest.datasets.values().cloned().collect(),
    })
}

fn plan_download_cache_strategy(
    csv_exists: bool,
    cached_min_date: Option<NaiveDate>,
    cached_max_date: Option<NaiveDate>,
    cached_min_ts: i64,
    cached_max_ts: i64,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> DownloadCachePlan {
    let have_full_range = match (cached_min_date, cached_max_date) {
        (Some(min_d), Some(max_d)) => csv_exists && from_date >= min_d && to_date <= max_d,
        _ => false,
    };
    if have_full_range {
        return DownloadCachePlan::FullHit;
    }

    let can_append_only = match cached_min_date {
        Some(min_d) => csv_exists && cached_min_ts > 0 && cached_max_ts > 0 && from_date >= min_d,
        None => false,
    };
    if can_append_only {
        DownloadCachePlan::AppendOnly
    } else {
        DownloadCachePlan::FullRefresh
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

    // Allow power users to paste Polygon-form tickers (e.g. X:BTCUSD, C:EURUSD).
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
    // Polygon tickers may include ':', which must be percent-encoded inside the URL path.
    seg.replace(':', "%3A")
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
                last_err = Some(e.to_string());
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
            let parsed: AggResponse = resp.json().map_err(|e| e.to_string())?;
            if let Some(err) = parsed.error {
                return Err(format!("Polygon API error: {err}"));
            }
            let st = parsed.status.unwrap_or_default();
            if !st.is_empty() && st.to_ascii_uppercase() != "OK" {
                return Err(format!("Polygon API status: {st}"));
            }
            return Ok(parsed.results.unwrap_or_default());
        }

        let retry_after_ms = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|secs| secs.saturating_mul(1000));

        let body = resp.text().unwrap_or_default();
        let retriable = status.as_u16() == 429 || status.is_server_error();
        last_err = Some(format!("({status}) {body}"));
        if retriable && attempt < 6 {
            let backoff_ms = (1_000u64.saturating_mul(1u64 << attempt)).min(30_000);
            let wait_ms = retry_after_ms.unwrap_or(backoff_ms).max(backoff_ms);
            std::thread::sleep(std::time::Duration::from_millis(wait_ms));
            continue;
        }

        return Err(format!("Polygon API request failed {}: {}", status, body));
    }

    Err(format!(
        "Polygon API request failed: {}",
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let f = fs::File::create(path).map_err(|e| e.to_string())?;
    let mut w = BufWriter::new(f);
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
}

fn csv_day_id_utc(ts_ms: i64) -> i64 {
    ts_ms.div_euclid(86_400_000)
}

fn scan_csv_integrity(
    path: &Path,
    timeframe_interval_ms: i64,
) -> Result<(u64, i64, i64, PolygonDatasetIntegrity), String> {
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

        let ts_str = t.split(',').next().unwrap_or("").trim();
        if ts_str.is_empty() {
            continue;
        }
        let ts: i64 = match ts_str.parse() {
            Ok(v) => v,
            Err(_) => {
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
    let p = Path::new(csv_path);
    if !p.exists() {
        return Err(format!("dataset file does not exist: {csv_path}"));
    }

    let mut manifest = load_manifest(cache_dir)?;
    let mut key: Option<String> = None;
    for (k, ds) in &manifest.datasets {
        if ds.csv_path == csv_path {
            key = Some(k.clone());
            break;
        }
    }
    let Some(key) = key else {
        return Err("dataset not found in Polygon manifest".to_string());
    };

    let tf_label = manifest
        .datasets
        .get(&key)
        .map(|d| d.timeframe.as_str())
        .unwrap_or("1m");
    let tf = parse_timeframe(Some(tf_label))?;
    let (rows, min_ts, max_ts, integrity) = scan_csv_integrity(p, tf.interval_ms)?;
    let Some(ds) = manifest.datasets.get_mut(&key) else {
        return Err("dataset not found in Polygon manifest".to_string());
    };

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
    let p = Path::new(csv_path);
    if !p.exists() {
        return Err(format!("dataset file does not exist: {csv_path}"));
    }

    let mut manifest = load_manifest(cache_dir)?;
    let mut key: Option<String> = None;
    for (k, ds) in &manifest.datasets {
        if ds.csv_path == csv_path {
            key = Some(k.clone());
            break;
        }
    }
    let Some(key) = key else {
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

    let f = fs::File::open(p).map_err(|e| e.to_string())?;
    let r = BufReader::new(f);

    // Hard limit to avoid accidental huge memory usage; for bigger datasets, a full re-download is safer.
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

    // Attempt to selectively refetch days with intra-day gaps (if API key is available).
    let gappy_days = collect_same_day_gap_dates(&bars, gap_threshold_ms);
    if !gappy_days.is_empty() {
        // Best-effort: do not fail the repair if the API key can't be read or requests fail.
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
    let (wrote, new_min, new_max) = overwrite_bars(p, &bars)?;

    let Some(ds) = manifest.datasets.get_mut(&key) else {
        return Err("dataset not found in Polygon manifest".to_string());
    };
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
    });

    let out = ds.clone();
    save_manifest(cache_dir, &manifest)?;
    Ok(out)
}

fn find_manifest_dataset_by_csv_path<'a>(
    manifest: &'a PolygonCacheManifest,
    csv_path: &str,
) -> Option<&'a PolygonDatasetMeta> {
    let target = csv_path.trim();
    if target.is_empty() {
        return None;
    }
    manifest
        .datasets
        .values()
        .find(|ds| ds.csv_path.trim() == target)
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
    let manifest = load_manifest(cache_dir)?;
    let dataset = find_manifest_dataset_by_csv_path(&manifest, csv_path)
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

    let api_key = read_api_key(app_identifier, cache_dir)?
        .ok_or_else(|| "Polygon API key is not set. Enter your key and click Save.".to_string())?;

    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let mut manifest = load_manifest(cache_dir)?;

    let adjusted = if market == Market::Stocks {
        req.adjusted.unwrap_or(true)
    } else {
        false
    };
    let tf = parse_timeframe(req.timeframe.as_deref())?;

    // Keep back-compat: adjusted stocks keep the original key/path. Unadjusted stocks get their own cache entry.
    let (dataset_key, file_name) =
        polygon_dataset_key_and_file_name(market, &polygon_ticker, adjusted, &tf.label);
    let csv_path = cache_dir.join(file_name);

    let mut used_cache_only = false;
    let mut downloaded = 0usize;

    if let Some(meta) = manifest.datasets.get(&dataset_key) {
        let min_ts = meta.min_ts;
        let max_ts = meta.max_ts;
        if min_ts > 0 && max_ts > 0 {
            // If the user requests older history than we have, do a full refresh.
            // Otherwise, only append newer bars.
            let want_full_refresh = false; // date-range based refresh is handled below
            let _ = want_full_refresh;
        }
    }

    // Decide refresh strategy by date request vs cached coverage (append-only if possible).
    let cached_meta = manifest.datasets.get(&dataset_key).cloned();
    let mut min_ts = cached_meta.as_ref().map(|m| m.min_ts).unwrap_or(0);
    let mut max_ts = cached_meta.as_ref().map(|m| m.max_ts).unwrap_or(0);
    let mut row_count = cached_meta.as_ref().map(|m| m.row_count).unwrap_or(0);

    // Cache coverage decisions are made at the date granularity (Polygon aggs API uses dates).
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
        from_date,
        to_date,
    );

    if cache_plan == DownloadCachePlan::FullHit {
        used_cache_only = true;
    } else {
        let client = Client::builder()
            .user_agent("vectorgrid/0.1 (tauri)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        // Append-only: if we have some data and the request starts within what we have, fetch only newer bars.
        // Otherwise, refresh the full requested range and overwrite.
        if cache_plan == DownloadCachePlan::AppendOnly {
            // Fetch from (cached max timestamp + 1 minute) up to requested to.
            // We convert cached max_ts to a date for Polygon requests.
            let next_ms = max_ts.saturating_add(tf.interval_ms);
            let next_dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(next_ms)
                .ok_or_else(|| "cached timestamp out of range".to_string())?;
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
                let wrote = append_bars(&csv_path, &bars, &mut min_ts, &mut max_ts)?;
                downloaded += wrote;
                if row_count > 0 {
                    row_count = row_count.saturating_add(wrote as u64);
                }
                chunk_from = chunk_to + Duration::days(1);
            }
        } else {
            // Full refresh for the requested range.
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
                return Err("Polygon returned no data for that range.".to_string());
            }
            // Ensure ascending and unique by timestamp.
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
        // If we never wrote data, treat as an error so the user knows why load won't work.
        return Err("No bars available (empty dataset).".to_string());
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
        integrity: cached_meta.and_then(|m| m.integrity),
    };
    manifest.datasets.insert(dataset_key, meta);
    save_manifest(cache_dir, &manifest)?;

    Ok(PolygonDownloadResult {
        csv_path,
        downloaded_bars: downloaded,
        used_cache_only,
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
                max_ts: 1_735_689_600_000, // 2025-01-01T00:00:00Z
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
                max_ts: 1_640_995_200_000, // 2022-01-01T00:00:00Z
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

        std::env::remove_var("POLYGON_KEY");
        assert_eq!(resolved.as_deref(), Some(env_key));
    }

    #[test]
    fn parse_polygon_key_from_dotenv_line_accepts_spaced_and_quoted_assignment() {
        let parsed = parse_polygon_key_from_dotenv_line(r#"POLYGON_KEY = "polygon-dotenv-key""#);
        assert_eq!(parsed.as_deref(), Some("polygon-dotenv-key"));
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
            NaiveDate::from_ymd_opt(2023, 12, 1).expect("from"),
            NaiveDate::from_ymd_opt(2024, 1, 20).expect("to"),
        );
        assert_eq!(full_refresh, DownloadCachePlan::FullRefresh);
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
    fn find_manifest_dataset_by_csv_path_matches_trimmed_paths() {
        let mut manifest = PolygonCacheManifest::default();
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
                last_updated_utc_ms: 0,
                row_count: 2,
                adjusted: true,
                integrity: None,
            },
        );

        let found =
            find_manifest_dataset_by_csv_path(&manifest, "  C:/cache/AAPL.csv  ").expect("match");
        assert_eq!(found.display_symbol, "AAPL");
        assert!(find_manifest_dataset_by_csv_path(&manifest, "   ").is_none());
        assert!(find_manifest_dataset_by_csv_path(&manifest, "C:/cache/MSFT.csv").is_none());
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
}
