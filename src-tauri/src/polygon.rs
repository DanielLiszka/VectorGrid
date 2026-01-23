use chrono::{Duration, NaiveDate};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "polygon_settings.json";
const MANIFEST_FILE: &str = "polygon_manifest.json";
const API_BASE: &str = "https://api.polygon.io";
const BARS_PER_REQUEST_LIMIT: u32 = 50_000;
const CHUNK_DAYS: i64 = 30;
const KEYRING_USERNAME: &str = "polygon_api_key";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonSettings {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonDatasetMeta {
    pub market: String,
    pub display_symbol: String,
    pub polygon_ticker: String,
    pub csv_path: String,
    pub min_ts: i64,
    pub max_ts: i64,
    pub last_updated_utc_ms: i64,
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

fn now_utc_ms() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    dur.as_millis() as i64
}

pub fn polygon_cache_dir(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join("polygon_cache")
}

fn settings_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(SETTINGS_FILE)
}

fn manifest_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(MANIFEST_FILE)
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
        "vectorta_optimizer".to_string()
    } else {
        format!("{id}.polygon")
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
        return Ok(None);
    };
    let key = st.api_key.trim().to_string();
    if key.is_empty() {
        return Ok(None);
    }

    if let Ok(entry) = keyring::Entry::new(&service, KEYRING_USERNAME) {
        if entry.set_password(&key).is_ok() {
            let _ = fs::remove_file(settings_path(cache_dir));
        }
    }

    Ok(Some(key))
}

fn load_manifest(cache_dir: &Path) -> Result<PolygonCacheManifest, String> {
    let p = manifest_path(cache_dir);
    if !p.exists() {
        return Ok(PolygonCacheManifest::default());
    }
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let m: PolygonCacheManifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
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
) -> Result<Vec<AggBar>, String> {
    let ticker = encode_path_segment(polygon_ticker);
    let url = format!("{API_BASE}/v2/aggs/ticker/{ticker}/range/1/minute/{from}/{to}");

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

pub fn download_1m_to_cache(
    app_identifier: &str,
    cache_dir: &Path,
    req: &PolygonDownloadRequest,
) -> Result<PolygonDownloadResult, String> {
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

    let dataset_key = format!("{}:{}:1m", market.as_str(), polygon_ticker);
    let file_name = format!(
        "polygon_{}_{}_1m.csv",
        market.as_str(),
        sanitize_filename_component(&polygon_ticker)
    );
    let csv_path = cache_dir.join(file_name);

    let mut used_cache_only = false;
    let mut downloaded = 0usize;

    let adjusted = market == Market::Stocks;

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

    // Approximate requested range in ms. We don't need precise intraday bounds; this is used for cache decisions.
    let req_from_ms = from_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "invalid from date".to_string())?
        .and_utc()
        .timestamp_millis();
    let req_to_ms = to_date
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| "invalid to date".to_string())?
        .and_utc()
        .timestamp_millis();

    let have_full_range = min_ts > 0 && max_ts > 0 && req_from_ms >= min_ts && req_to_ms <= max_ts;

    if have_full_range && csv_path.exists() {
        used_cache_only = true;
    } else {
        let client = Client::builder()
            .user_agent("vectorta-optimizer/0.1 (tauri)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        // Append-only: if we have some data and the request starts within what we have, fetch only newer bars.
        // Otherwise, refresh the full requested range and overwrite.
        let do_append_only = csv_path.exists() && min_ts > 0 && max_ts > 0 && req_from_ms >= min_ts;

        if do_append_only {
            // Fetch from (cached max timestamp + 1 minute) up to requested to.
            // We convert cached max_ts to a date for Polygon requests.
            let next_ms = max_ts + 60_000;
            let next_dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(next_ms)
                .ok_or_else(|| "cached timestamp out of range".to_string())?;
            let mut chunk_from = next_dt.date_naive();
            let chunk_end = to_date;

            while chunk_from <= chunk_end {
                let chunk_to =
                    std::cmp::min(chunk_from + Duration::days(CHUNK_DAYS - 1), chunk_end);
                let from_s = chunk_from.format("%Y-%m-%d").to_string();
                let to_s = chunk_to.format("%Y-%m-%d").to_string();
                let bars =
                    fetch_chunk(&client, &api_key, &polygon_ticker, &from_s, &to_s, adjusted)?;
                downloaded += append_bars(&csv_path, &bars, &mut min_ts, &mut max_ts)?;
                chunk_from = chunk_to + Duration::days(1);
            }
        } else {
            // Full refresh for the requested range.
            let mut all: Vec<AggBar> = Vec::new();
            let mut chunk_from = from_date;
            while chunk_from <= to_date {
                let chunk_to = std::cmp::min(chunk_from + Duration::days(CHUNK_DAYS - 1), to_date);
                let from_s = chunk_from.format("%Y-%m-%d").to_string();
                let to_s = chunk_to.format("%Y-%m-%d").to_string();
                let mut bars =
                    fetch_chunk(&client, &api_key, &polygon_ticker, &from_s, &to_s, adjusted)?;
                all.append(&mut bars);
                chunk_from = chunk_to + Duration::days(1);
            }
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
        csv_path: csv_path.to_string_lossy().to_string(),
        min_ts,
        max_ts,
        last_updated_utc_ms: now_utc_ms(),
    };
    manifest.datasets.insert(dataset_key, meta);
    save_manifest(cache_dir, &manifest)?;

    Ok(PolygonDownloadResult {
        csv_path,
        downloaded_bars: downloaded,
        used_cache_only,
    })
}
