use crate::{polygon, settings};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use ta_app_core::{
    dataset_manifest_path, load_imported_options_dataset_from_csv, read_options_dataset_manifest,
    scan_options_dataset_manifests, write_options_dataset_manifest,
    ImportedOptionsDatasetCsvBuildRequest, OptionsDatasetManifestV1, OptionsPriceSemantics,
};
use tauri::Manager;

fn default_provider_name() -> String {
    "massive".to_string()
}

fn default_provider_source_surface() -> String {
    "minute_aggregate".to_string()
}

fn default_quote_resolution() -> String {
    "1m".to_string()
}

fn default_price_semantics() -> OptionsPriceSemantics {
    OptionsPriceSemantics::MinuteAggregate
}

fn default_calendar_id() -> String {
    "market_hours_utc".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_contract_reference_source() -> String {
    "massive_contract_reference".to_string()
}

fn default_underlier_reference_source() -> String {
    "underlying_dataset".to_string()
}

fn canonicalize_provider_name(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(default_provider_name());
    }
    match normalized.as_str() {
        "massive" | "polygon" | "polygon.io" | "polygon_io" | "polygonio" => {
            Ok("massive".to_string())
        }
        _ => Err(
            "options dataset imports currently assume Polygon/Massive-derived data only"
                .to_string(),
        ),
    }
}

fn canonicalize_provider_source_surface(
    raw: &str,
    price_semantics: OptionsPriceSemantics,
) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(match price_semantics {
            OptionsPriceSemantics::MinuteAggregate => "minute_aggregate".to_string(),
            OptionsPriceSemantics::TopOfBookQuote => "top_of_book_quote".to_string(),
            OptionsPriceSemantics::DerivedMidSnapshot => "derived_mid_snapshot".to_string(),
        });
    }
    match normalized.as_str() {
        "minute_aggregate" | "minute_aggregates" | "aggregate" | "aggregates" => {
            Ok("minute_aggregate".to_string())
        }
        "top_of_book_quote" | "top_of_book_quotes" | "quote" | "quotes" => {
            Ok("top_of_book_quote".to_string())
        }
        "derived_mid_snapshot" | "derived_mid" | "mid_snapshot" | "mid" => {
            Ok("derived_mid_snapshot".to_string())
        }
        _ => Err(
            "options dataset imports currently support Polygon/Massive minute aggregates, top-of-book quotes, or derived mid snapshots only"
                .to_string(),
        ),
    }
}

fn canonicalize_contract_reference_source(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(default_contract_reference_source());
    }
    match normalized.as_str() {
        "massive_contract_reference"
        | "polygon_contract_reference"
        | "polygon_io_contract_reference"
        | "contracts"
        | "all_contracts" => Ok("massive_contract_reference".to_string()),
        _ => Err(
            "options dataset imports currently assume Polygon/Massive contract-reference metadata only"
                .to_string(),
        ),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptionsDatasetImportRequest {
    pub csv_path: String,
    #[serde(default)]
    pub options_dataset_id: Option<String>,
    #[serde(default)]
    pub root_symbol: Option<String>,
    #[serde(default = "default_provider_name")]
    pub provider_name: String,
    #[serde(default = "default_provider_source_surface")]
    pub provider_source_surface: String,
    #[serde(default)]
    pub provider_entitlement_profile: Option<String>,
    #[serde(default = "default_quote_resolution")]
    pub quote_resolution: String,
    #[serde(default = "default_price_semantics")]
    pub price_semantics: OptionsPriceSemantics,
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub normalization_version: Option<u32>,
    #[serde(default)]
    pub quote_semantics_version: Option<u32>,
    #[serde(default)]
    pub corporate_action_mapping_version: Option<u32>,
    #[serde(default = "default_contract_reference_source")]
    pub contract_reference_source: String,
    #[serde(default)]
    pub contract_reference_as_of_utc: Option<String>,
    #[serde(default = "default_underlier_reference_source")]
    pub underlier_reference_source: String,
    #[serde(default)]
    pub aggregate_price_field: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolygonOptionsDatasetDownloadRequest {
    pub root_symbol: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub options_dataset_id: Option<String>,
    #[serde(default)]
    pub contract_type: Option<String>,
    #[serde(default)]
    pub expiration_date_from: Option<String>,
    #[serde(default)]
    pub expiration_date_to: Option<String>,
    #[serde(default)]
    pub max_dte_days: Option<u16>,
    #[serde(default)]
    pub strike_min: Option<f64>,
    #[serde(default)]
    pub strike_max: Option<f64>,
    #[serde(default)]
    pub max_contracts: Option<usize>,
    #[serde(default)]
    pub provider_entitlement_profile: Option<String>,
    #[serde(default)]
    pub contract_reference_as_of_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonOptionsDatasetDownloadResponse {
    pub manifest: OptionsDatasetManifestV1,
    pub contracts_discovered: usize,
    pub contracts_selected: usize,
    pub contracts_with_rows: usize,
    pub downloaded_option_rows: usize,
    pub skipped_contracts: usize,
    pub underlying_csv_path: String,
    pub used_underlying_cache_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct OptionsContractsResponse {
    results: Option<Vec<PolygonOptionContractRef>>,
    next_url: Option<String>,
    status: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolygonOptionContractRef {
    ticker: String,
    underlying_ticker: Option<String>,
    contract_type: String,
    expiration_date: String,
    strike_price: f64,
    #[serde(default)]
    shares_per_contract: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolygonOptionsAggResponse {
    results: Option<Vec<PolygonOptionsAggBar>>,
    status: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolygonOptionsAggBar {
    t: i64,
    o: f64,
    c: f64,
    h: f64,
    l: f64,
    #[serde(default)]
    v: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionContractTypeFilter {
    Both,
    Call,
    Put,
}

impl OptionContractTypeFilter {
    fn parse(raw: Option<&str>) -> Result<Self, String> {
        let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(Self::Both);
        };
        match raw.to_ascii_lowercase().as_str() {
            "both" | "all" => Ok(Self::Both),
            "call" | "calls" | "c" => Ok(Self::Call),
            "put" | "puts" | "p" => Ok(Self::Put),
            other => Err(format!(
                "contract_type must be one of: both, call, put (got '{other}')"
            )),
        }
    }

    fn query_value(self) -> Option<&'static str> {
        match self {
            Self::Both => None,
            Self::Call => Some("call"),
            Self::Put => Some("put"),
        }
    }

    fn matches_contract(self, raw: &str) -> bool {
        match (self, raw.trim().to_ascii_lowercase().as_str()) {
            (Self::Both, "call") | (Self::Both, "c") => true,
            (Self::Both, "put") | (Self::Both, "p") => true,
            (Self::Call, "call") | (Self::Call, "c") => true,
            (Self::Put, "put") | (Self::Put, "p") => true,
            _ => false,
        }
    }

    fn normalized_row_value(raw: &str) -> Result<&'static str, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "call" | "c" => Ok("call"),
            "put" | "p" => Ok("put"),
            other => Err(format!(
                "unsupported Polygon option contract_type '{other}'"
            )),
        }
    }
}

const POLYGON_API_BASE: &str = "https://api.polygon.io";
const POLYGON_OPTIONS_CONTRACTS_LIMIT: usize = 1000;
const POLYGON_OPTIONS_BARS_PER_REQUEST_LIMIT: u32 = 50_000;
const POLYGON_OPTIONS_CHUNK_DAYS: i64 = 14;
const POLYGON_OPTIONS_DEFAULT_MAX_CONTRACTS: usize = 400;

fn sanitize_id_component(raw: &str) -> String {
    let out: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    out.trim_matches('_').to_string()
}

fn sanitize_dataset_id(raw: &str) -> Result<String, String> {
    let sanitized = sanitize_id_component(raw);
    if sanitized.is_empty() {
        return Err("options_dataset_id resolved to an empty value".to_string());
    }
    Ok(sanitized)
}

fn derive_dataset_id(req: &OptionsDatasetImportRequest) -> String {
    if let Some(explicit) = req
        .options_dataset_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return sanitize_id_component(explicit);
    }
    if let Some(root) = req
        .root_symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!(
            "{}_{}",
            sanitize_id_component(root),
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
    }
    let stem = std::path::Path::new(req.csv_path.trim())
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("options_dataset");
    format!(
        "{}_{}",
        sanitize_id_component(stem),
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    )
}

fn derive_polygon_download_dataset_id(req: &PolygonOptionsDatasetDownloadRequest) -> String {
    if let Some(explicit) = req
        .options_dataset_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return sanitize_id_component(explicit);
    }
    let root = sanitize_id_component(&req.root_symbol);
    let from = sanitize_id_component(&req.from);
    let to = sanitize_id_component(&req.to);
    let mut dataset_id = format!("{root}_options_1m_{from}_{to}");

    if let Some(contract_type) = req.contract_type.as_deref() {
        match OptionContractTypeFilter::parse(Some(contract_type)) {
            Ok(OptionContractTypeFilter::Call) => dataset_id.push_str("_call"),
            Ok(OptionContractTypeFilter::Put) => dataset_id.push_str("_put"),
            _ => {}
        }
    }
    if let Some(expiration_date_from) = req
        .expiration_date_from
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        dataset_id.push_str("_exp_from_");
        dataset_id.push_str(&sanitize_id_component(expiration_date_from));
    }
    if let Some(expiration_date_to) = req
        .expiration_date_to
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        dataset_id.push_str("_exp_to_");
        dataset_id.push_str(&sanitize_id_component(expiration_date_to));
    }
    if let Some(max_dte_days) = req.max_dte_days {
        dataset_id.push_str(&format!("_max_dte_{max_dte_days}"));
    }
    if let Some(strike_min) = req.strike_min {
        dataset_id.push_str("_strike_min_");
        dataset_id.push_str(&sanitize_id_component(&strike_min.to_string()));
    }
    if let Some(strike_max) = req.strike_max {
        dataset_id.push_str("_strike_max_");
        dataset_id.push_str(&sanitize_id_component(&strike_max.to_string()));
    }
    if let Some(max_contracts) = req.max_contracts {
        dataset_id.push_str(&format!("_max_contracts_{max_contracts}"));
    }
    if let Some(as_of) = req
        .contract_reference_as_of_utc
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        dataset_id.push_str("_as_of_");
        dataset_id.push_str(&sanitize_id_component(as_of));
    }

    dataset_id
}

fn resolve_options_datasets_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    Ok(market_root.join("options_research").join("datasets"))
}

fn options_datasets_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let dir = resolve_options_datasets_dir(app_local_data_dir)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn dataset_csv_copy_path(dataset_dir: &std::path::Path) -> std::path::PathBuf {
    dataset_dir.join("rows.csv")
}

fn polygon_options_contracts_url() -> String {
    format!("{POLYGON_API_BASE}/v3/reference/options/contracts")
}

fn encode_polygon_path_segment(seg: &str) -> String {
    seg.replace(':', "%3A")
}

fn parse_date_yyyy_mm_dd(raw: &str, label: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        .map_err(|_| format!("{label} must be YYYY-MM-DD"))
}

fn normalize_root_symbol(raw: &str) -> Result<String, String> {
    let normalized = raw
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '-')
        .collect::<String>()
        .to_ascii_uppercase();
    if normalized.is_empty() {
        return Err("root_symbol is empty".to_string());
    }
    Ok(normalized)
}

fn expiration_timestamp_ms_from_date(expiration_date: &str) -> Result<i64, String> {
    let date = parse_date_yyyy_mm_dd(expiration_date, "expiration_date")?;
    let dt = date
        .and_time(
            NaiveTime::from_hms_opt(23, 59, 0)
                .ok_or_else(|| "failed to construct synthetic expiration timestamp".to_string())?,
        )
        .and_utc();
    Ok(dt.timestamp_millis())
}

fn format_contract_reference_as_of_utc(
    explicit: Option<&str>,
    fallback_date: &NaiveDate,
) -> Option<String> {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| Some(format!("{}T23:59:00Z", fallback_date.format("%Y-%m-%d"))))
}

fn underlying_reference_source_label(root_symbol: &str) -> String {
    format!("polygon_stocks_1m_raw:{root_symbol}")
}

fn build_polygon_download_import_request(
    req: &PolygonOptionsDatasetDownloadRequest,
    dataset_id: &str,
    dataset_path: &std::path::Path,
    root_symbol: &str,
    as_of: &NaiveDate,
) -> ImportedOptionsDatasetCsvBuildRequest {
    ImportedOptionsDatasetCsvBuildRequest {
        options_dataset_id: dataset_id.to_string(),
        dataset_path: dataset_path.to_string_lossy().to_string(),
        root_symbol: Some(root_symbol.to_string()),
        provider_name: default_provider_name(),
        provider_source_surface: default_provider_source_surface(),
        provider_entitlement_profile: req.provider_entitlement_profile.clone(),
        quote_resolution: default_quote_resolution(),
        price_semantics: OptionsPriceSemantics::MinuteAggregate,
        calendar_id: default_calendar_id(),
        timezone: default_timezone(),
        normalization_version: 1,
        quote_semantics_version: 1,
        corporate_action_mapping_version: 1,
        contract_reference_source: default_contract_reference_source(),
        contract_reference_as_of_utc: format_contract_reference_as_of_utc(
            req.contract_reference_as_of_utc.as_deref(),
            as_of,
        ),
        underlier_reference_source: underlying_reference_source_label(root_symbol),
        aggregate_price_field: Some("close".to_string()),
    }
}

fn read_underlying_close_map(csv_path: &std::path::Path) -> Result<BTreeMap<i64, f64>, String> {
    let file = std::fs::File::open(csv_path).map_err(|e| {
        format!(
            "failed to open underlying Polygon CSV {}: {e}",
            csv_path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let mut out = BTreeMap::new();
    for (line_idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line_idx == 0 && trimmed.to_ascii_lowercase().starts_with("timestamp") {
            continue;
        }
        let mut cols = trimmed.split(',');
        let ts = cols
            .next()
            .map(str::trim)
            .ok_or_else(|| "underlying Polygon CSV row missing timestamp".to_string())?
            .parse::<i64>()
            .map_err(|_| "invalid timestamp in underlying Polygon CSV".to_string())?;
        let _open = cols.next();
        let close = cols
            .next()
            .map(str::trim)
            .ok_or_else(|| "underlying Polygon CSV row missing close".to_string())?
            .parse::<f64>()
            .map_err(|_| "invalid close in underlying Polygon CSV".to_string())?;
        if close.is_finite() && close >= 0.0 {
            out.insert(ts, close);
        }
    }
    if out.is_empty() {
        return Err("underlying Polygon CSV had no usable rows".to_string());
    }
    Ok(out)
}

fn underlying_price_at_or_before(
    by_timestamp: &BTreeMap<i64, f64>,
    timestamp_ms: i64,
) -> Option<f64> {
    by_timestamp
        .range(..=timestamp_ms)
        .next_back()
        .map(|(_, price)| *price)
}

fn client_with_polygon_defaults() -> Result<Client, String> {
    Client::builder()
        .user_agent("vectorgrid/0.1 (tauri options)")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())
}

fn parse_retry_after_ms(value: &str, now: DateTime<Utc>, max_wait_ms: u64) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds.saturating_mul(1000).min(max_wait_ms));
    }
    let retry_at = DateTime::parse_from_rfc2822(trimmed).ok()?;
    let delay_ms = retry_at
        .with_timezone(&Utc)
        .signed_duration_since(now)
        .num_milliseconds()
        .max(0) as u64;
    Some(delay_ms.min(max_wait_ms))
}

fn send_json_with_retry<T, F>(make_request: F) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
    F: Fn() -> reqwest::blocking::RequestBuilder,
{
    const MAX_RETRY_WAIT_MS: u64 = 30_000;
    let mut last_err: Option<String> = None;
    for attempt in 0..=6u32 {
        let resp = make_request().send();
        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(e.to_string());
                if attempt < 6 {
                    std::thread::sleep(std::time::Duration::from_millis(
                        (1_000u64.saturating_mul(1u64 << attempt)).min(MAX_RETRY_WAIT_MS),
                    ));
                    continue;
                }
                return Err(format!(
                    "Polygon/Massive request failed: {}",
                    last_err.unwrap_or_default()
                ));
            }
        };

        let status = resp.status();
        if status.is_success() {
            return resp.json::<T>().map_err(|e| e.to_string());
        }

        let retry_after_ms = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|value| parse_retry_after_ms(value, Utc::now(), MAX_RETRY_WAIT_MS));
        let body = resp.text().unwrap_or_default();
        let retriable = status.as_u16() == 429 || status.is_server_error();
        last_err = Some(format!("({status}) {body}"));
        if retriable && attempt < 6 {
            let backoff_ms = (1_000u64.saturating_mul(1u64 << attempt)).min(MAX_RETRY_WAIT_MS);
            let wait_ms = retry_after_ms.unwrap_or(backoff_ms).max(backoff_ms);
            std::thread::sleep(std::time::Duration::from_millis(wait_ms));
            continue;
        }
        return Err(format!(
            "Polygon/Massive request failed {}: {}",
            status, body
        ));
    }
    Err(format!(
        "Polygon/Massive request failed: {}",
        last_err.unwrap_or_else(|| "unknown error".to_string())
    ))
}

fn fetch_polygon_option_contracts(
    client: &Client,
    api_key: &str,
    root_symbol: &str,
    contract_type: OptionContractTypeFilter,
    expiration_date_from: &NaiveDate,
    expiration_date_to: &NaiveDate,
    as_of: &NaiveDate,
) -> Result<Vec<PolygonOptionContractRef>, String> {
    let mut out = Vec::new();
    let mut next_url: Option<String> = None;
    loop {
        let response: OptionsContractsResponse = if let Some(url) = next_url.as_ref() {
            send_json_with_retry(|| client.get(url).query(&[("apiKey", api_key)]))?
        } else {
            send_json_with_retry(|| {
                let mut req = client.get(polygon_options_contracts_url()).query(&[
                    ("underlying_ticker", root_symbol),
                    ("as_of", &as_of.format("%Y-%m-%d").to_string()),
                    (
                        "expiration_date.gte",
                        &expiration_date_from.format("%Y-%m-%d").to_string(),
                    ),
                    (
                        "expiration_date.lte",
                        &expiration_date_to.format("%Y-%m-%d").to_string(),
                    ),
                    ("sort", "expiration_date"),
                    ("order", "asc"),
                    ("limit", &POLYGON_OPTIONS_CONTRACTS_LIMIT.to_string()),
                    ("apiKey", api_key),
                ]);
                if let Some(contract_type) = contract_type.query_value() {
                    req = req.query(&[("contract_type", contract_type)]);
                }
                req
            })?
        };

        if let Some(error) = response.error {
            return Err(format!("Polygon/Massive options contracts error: {error}"));
        }
        let status = response.status.unwrap_or_default();
        if !status.is_empty() && !status.eq_ignore_ascii_case("ok") {
            return Err(format!(
                "Polygon/Massive options contracts status: {status}"
            ));
        }
        if let Some(results) = response.results {
            out.extend(results);
        }
        match response.next_url {
            Some(url) if !url.trim().is_empty() => next_url = Some(url),
            _ => break,
        }
    }
    Ok(out)
}

fn fetch_polygon_option_agg_chunk(
    client: &Client,
    api_key: &str,
    contract_ticker: &str,
    from: &NaiveDate,
    to: &NaiveDate,
) -> Result<Vec<PolygonOptionsAggBar>, String> {
    let ticker = encode_polygon_path_segment(contract_ticker);
    let url = format!(
        "{POLYGON_API_BASE}/v2/aggs/ticker/{ticker}/range/1/minute/{}/{}",
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d")
    );
    let response: PolygonOptionsAggResponse = send_json_with_retry(|| {
        client.get(&url).query(&[
            ("adjusted", "false"),
            ("sort", "asc"),
            ("limit", &POLYGON_OPTIONS_BARS_PER_REQUEST_LIMIT.to_string()),
            ("apiKey", api_key),
        ])
    })?;
    if let Some(error) = response.error {
        return Err(format!(
            "Polygon/Massive options aggregate error for {contract_ticker}: {error}"
        ));
    }
    let status = response.status.unwrap_or_default();
    if !status.is_empty() && !status.eq_ignore_ascii_case("ok") {
        return Err(format!(
            "Polygon/Massive options aggregate status for {contract_ticker}: {status}"
        ));
    }
    Ok(response.results.unwrap_or_default())
}

fn fetch_polygon_option_aggs(
    client: &Client,
    api_key: &str,
    contract_ticker: &str,
    from: &NaiveDate,
    to: &NaiveDate,
) -> Result<Vec<PolygonOptionsAggBar>, String> {
    let mut chunk_from = *from;
    let mut out = Vec::new();
    while chunk_from <= *to {
        let chunk_to = std::cmp::min(
            chunk_from + Duration::days(POLYGON_OPTIONS_CHUNK_DAYS - 1),
            *to,
        );
        let mut bars = fetch_polygon_option_agg_chunk(
            client,
            api_key,
            contract_ticker,
            &chunk_from,
            &chunk_to,
        )?;
        out.append(&mut bars);
        chunk_from = chunk_to + Duration::days(1);
    }
    out.sort_by_key(|bar| bar.t);
    out.dedup_by_key(|bar| bar.t);
    Ok(out)
}

fn contract_matches_filters(
    contract: &PolygonOptionContractRef,
    contract_type: OptionContractTypeFilter,
    strike_min: Option<f64>,
    strike_max: Option<f64>,
) -> Result<bool, String> {
    if !contract_type.matches_contract(&contract.contract_type) {
        return Ok(false);
    }
    if let Some(min) = strike_min {
        if contract.strike_price < min {
            return Ok(false);
        }
    }
    if let Some(max) = strike_max {
        if contract.strike_price > max {
            return Ok(false);
        }
    }
    if !contract.strike_price.is_finite() || contract.strike_price <= 0.0 {
        return Err(format!(
            "contract '{}' returned invalid strike {}",
            contract.ticker, contract.strike_price
        ));
    }
    Ok(true)
}

fn write_normalized_options_csv_header(
    writer: &mut BufWriter<std::fs::File>,
) -> Result<(), String> {
    writer
        .write_all(
            b"root_symbol,contract_id,option_type,expiration_date,expiration_timestamp_ms,strike,multiplier,timestamp_ms,open,high,low,close,underlying_price,volume,open_interest,session_id\n",
        )
        .map_err(|e| e.to_string())
}

fn write_polygon_options_contract_rows(
    writer: &mut BufWriter<std::fs::File>,
    root_symbol: &str,
    contract: &PolygonOptionContractRef,
    underlying_by_timestamp: &BTreeMap<i64, f64>,
    bars: &[PolygonOptionsAggBar],
) -> Result<usize, String> {
    let option_type = OptionContractTypeFilter::normalized_row_value(&contract.contract_type)?;
    let expiration_timestamp_ms = expiration_timestamp_ms_from_date(&contract.expiration_date)?;
    let multiplier = contract.shares_per_contract.unwrap_or(100.0);
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err(format!(
            "contract '{}' returned invalid shares_per_contract {}",
            contract.ticker, multiplier
        ));
    }
    let mut wrote = 0usize;
    for bar in bars {
        let Some(underlying_price) = underlying_price_at_or_before(underlying_by_timestamp, bar.t)
        else {
            continue;
        };
        if !bar.o.is_finite()
            || !bar.h.is_finite()
            || !bar.l.is_finite()
            || !bar.c.is_finite()
            || bar.o < 0.0
            || bar.h < 0.0
            || bar.l < 0.0
            || bar.c < 0.0
        {
            continue;
        }
        let session_id = chrono::DateTime::<Utc>::from_timestamp_millis(bar.t)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "".to_string());
        writeln!(
            writer,
            "{root_symbol},{},{option_type},{},{},{},{},{},{},{},{},{},{},{},,{}",
            contract.ticker,
            contract.expiration_date,
            expiration_timestamp_ms,
            contract.strike_price,
            multiplier,
            bar.t,
            bar.o,
            bar.h,
            bar.l,
            bar.c,
            underlying_price,
            bar.v.unwrap_or(0.0),
            session_id
        )
        .map_err(|e| e.to_string())?;
        wrote += 1;
    }
    Ok(wrote)
}

fn download_polygon_options_dataset_into_store(
    req: &PolygonOptionsDatasetDownloadRequest,
    app_identifier: &str,
    app_local_data_dir: &std::path::Path,
) -> Result<PolygonOptionsDatasetDownloadResponse, String> {
    let root_symbol = normalize_root_symbol(&req.root_symbol)?;
    let from_date = parse_date_yyyy_mm_dd(&req.from, "from")?;
    let to_date = parse_date_yyyy_mm_dd(&req.to, "to")?;
    if to_date < from_date {
        return Err("to date must be >= from date".to_string());
    }
    let contract_type = OptionContractTypeFilter::parse(req.contract_type.as_deref())?;
    let max_contracts = req
        .max_contracts
        .unwrap_or(POLYGON_OPTIONS_DEFAULT_MAX_CONTRACTS)
        .max(1);
    let expiration_date_from = if let Some(value) = req
        .expiration_date_from
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parse_date_yyyy_mm_dd(value, "expiration_date_from")?
    } else {
        from_date
    };
    let expiration_date_to = if let Some(value) = req
        .expiration_date_to
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parse_date_yyyy_mm_dd(value, "expiration_date_to")?
    } else {
        to_date + Duration::days(i64::from(req.max_dte_days.unwrap_or(7)))
    };
    if expiration_date_to < expiration_date_from {
        return Err("expiration_date_to must be >= expiration_date_from".to_string());
    }
    if let (Some(strike_min), Some(strike_max)) = (req.strike_min, req.strike_max) {
        if strike_min > strike_max {
            return Err("strike_min must be <= strike_max".to_string());
        }
    }

    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    let polygon_cache_dir = polygon::polygon_cache_dir(&market_root);
    let api_key = polygon::read_api_key(app_identifier, &polygon_cache_dir)?
        .ok_or_else(|| "Polygon API key is not set. Enter your key and click Save.".to_string())?;
    let client = client_with_polygon_defaults()?;
    let as_of = req
        .contract_reference_as_of_utc
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.get(0..10))
        .map(|value| parse_date_yyyy_mm_dd(value, "contract_reference_as_of_utc"))
        .transpose()?
        .unwrap_or(to_date);

    let discovered_contracts = fetch_polygon_option_contracts(
        &client,
        &api_key,
        &root_symbol,
        contract_type,
        &expiration_date_from,
        &expiration_date_to,
        &as_of,
    )?;
    let contracts_discovered = discovered_contracts.len();
    let mut selected_contracts = Vec::new();
    for contract in discovered_contracts {
        if normalize_root_symbol(
            contract
                .underlying_ticker
                .as_deref()
                .unwrap_or(&root_symbol),
        )? != root_symbol
        {
            continue;
        }
        if contract_matches_filters(&contract, contract_type, req.strike_min, req.strike_max)? {
            selected_contracts.push(contract);
        }
    }
    if selected_contracts.is_empty() {
        return Err(
            "Polygon/Massive returned no option contracts matching the requested filters"
                .to_string(),
        );
    }
    if selected_contracts.len() > max_contracts {
        return Err(format!(
            "selected {} option contracts, which exceeds max_contracts={} for the first-wave backend path; narrow expirations or strikes",
            selected_contracts.len(),
            max_contracts
        ));
    }

    let datasets_dir = options_datasets_dir(app_local_data_dir)?;
    let dataset_id = derive_polygon_download_dataset_id(req);
    let dataset_dir = datasets_dir.join(&dataset_id);
    if dataset_dir.exists() {
        return Err(format!(
            "options dataset id '{}' already exists at {}",
            dataset_id,
            dataset_dir.display()
        ));
    }

    let underlying_download = polygon::download_1m_to_cache(
        app_identifier,
        &polygon_cache_dir,
        &polygon::PolygonDownloadRequest {
            market: "stocks".to_string(),
            symbol: root_symbol.clone(),
            from: req.from.clone(),
            to: req.to.clone(),
            adjusted: Some(false),
            timeframe: Some("1m".to_string()),
        },
        None,
    )?;
    let underlying_csv_path = underlying_download.csv_path.clone();
    let underlying_by_timestamp = read_underlying_close_map(&underlying_csv_path)?;

    let result = (|| {
        std::fs::create_dir_all(&dataset_dir).map_err(|e| e.to_string())?;
        let dataset_csv = dataset_csv_copy_path(&dataset_dir);
        let file = std::fs::File::create(&dataset_csv).map_err(|e| {
            format!(
                "failed to create normalized options dataset CSV {}: {e}",
                dataset_csv.display()
            )
        })?;
        let mut writer = BufWriter::new(file);
        write_normalized_options_csv_header(&mut writer)?;

        let mut downloaded_option_rows = 0usize;
        let mut contracts_with_rows = 0usize;
        let mut skipped_contracts = 0usize;
        for contract in &selected_contracts {
            let bars = fetch_polygon_option_aggs(
                &client,
                &api_key,
                &contract.ticker,
                &from_date,
                &to_date,
            )?;
            if bars.is_empty() {
                skipped_contracts += 1;
                continue;
            }
            let wrote = write_polygon_options_contract_rows(
                &mut writer,
                &root_symbol,
                contract,
                &underlying_by_timestamp,
                &bars,
            )?;
            if wrote == 0 {
                skipped_contracts += 1;
                continue;
            }
            contracts_with_rows += 1;
            downloaded_option_rows += wrote;
        }
        writer.flush().map_err(|e| e.to_string())?;

        if downloaded_option_rows == 0 {
            return Err(
                "Polygon/Massive returned no usable 1m option rows for the requested filters"
                    .to_string(),
            );
        }

        let build_req = build_polygon_download_import_request(
            req,
            &dataset_id,
            &dataset_csv,
            &root_symbol,
            &as_of,
        );
        let dataset = load_imported_options_dataset_from_csv(&build_req)?;
        let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
        write_options_dataset_manifest(&manifest_path, &dataset.manifest)?;
        Ok(PolygonOptionsDatasetDownloadResponse {
            manifest: dataset.manifest,
            contracts_discovered,
            contracts_selected: selected_contracts.len(),
            contracts_with_rows,
            downloaded_option_rows,
            skipped_contracts,
            underlying_csv_path: underlying_csv_path.to_string_lossy().to_string(),
            used_underlying_cache_only: underlying_download.used_cache_only,
        })
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dataset_dir);
    }
    result
}

fn copy_dataset_into_store(
    req: &OptionsDatasetImportRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsDatasetManifestV1, String> {
    let datasets_dir = options_datasets_dir(app_local_data_dir)?;
    let dataset_id = derive_dataset_id(req);
    if dataset_id.is_empty() {
        return Err("options_dataset_id resolved to an empty value".to_string());
    }
    let dataset_dir = datasets_dir.join(&dataset_id);
    if dataset_dir.exists() {
        return Err(format!(
            "options dataset id '{}' already exists at {}",
            dataset_id,
            dataset_dir.display()
        ));
    }
    let source_csv = std::path::Path::new(req.csv_path.trim());
    if !source_csv.is_file() {
        return Err(format!(
            "options dataset CSV does not exist: {}",
            source_csv.display()
        ));
    }

    let import_result = (|| {
        std::fs::create_dir_all(&dataset_dir).map_err(|e| e.to_string())?;
        let copied_csv = dataset_csv_copy_path(&dataset_dir);
        std::fs::copy(source_csv, &copied_csv)
            .map_err(|e| format!("failed to copy options dataset CSV into store: {e}"))?;
        let provider_name = canonicalize_provider_name(&req.provider_name)?;
        let provider_source_surface = canonicalize_provider_source_surface(
            &req.provider_source_surface,
            req.price_semantics,
        )?;
        let contract_reference_source =
            canonicalize_contract_reference_source(&req.contract_reference_source)?;

        let aggregate_price_field = req.aggregate_price_field.clone().or_else(|| {
            if matches!(req.price_semantics, OptionsPriceSemantics::MinuteAggregate) {
                Some("close".to_string())
            } else {
                None
            }
        });
        if matches!(req.price_semantics, OptionsPriceSemantics::MinuteAggregate)
            && aggregate_price_field
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| !value.eq_ignore_ascii_case("close"))
                .unwrap_or(false)
        {
            return Err(
                "minute-aggregate imports currently require aggregate_price_field='close'"
                    .to_string(),
            );
        }

        let build_req = ImportedOptionsDatasetCsvBuildRequest {
            options_dataset_id: dataset_id.clone(),
            dataset_path: copied_csv.to_string_lossy().to_string(),
            root_symbol: req.root_symbol.clone(),
            provider_name,
            provider_source_surface,
            provider_entitlement_profile: req.provider_entitlement_profile.clone(),
            quote_resolution: req.quote_resolution.clone(),
            price_semantics: req.price_semantics,
            calendar_id: req.calendar_id.clone(),
            timezone: req.timezone.clone(),
            normalization_version: req.normalization_version.unwrap_or(1),
            quote_semantics_version: req.quote_semantics_version.unwrap_or(1),
            corporate_action_mapping_version: req.corporate_action_mapping_version.unwrap_or(1),
            contract_reference_source,
            contract_reference_as_of_utc: req.contract_reference_as_of_utc.clone(),
            underlier_reference_source: req.underlier_reference_source.clone(),
            aggregate_price_field,
        };
        let dataset = load_imported_options_dataset_from_csv(&build_req)?;
        let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
        write_options_dataset_manifest(&manifest_path, &dataset.manifest)?;
        Ok(dataset.manifest)
    })();

    if import_result.is_err() {
        let _ = std::fs::remove_dir_all(&dataset_dir);
    }
    import_result
}

pub fn import_options_dataset_blocking_headless(
    req: OptionsDatasetImportRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsDatasetManifestV1, String> {
    copy_dataset_into_store(&req, app_local_data_dir)
}

#[tauri::command]
pub async fn import_options_dataset(
    req: OptionsDatasetImportRequest,
    app: tauri::AppHandle,
) -> Result<OptionsDatasetManifestV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        copy_dataset_into_store(&req, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_options_datasets(
    app: tauri::AppHandle,
) -> Result<Vec<OptionsDatasetManifestV1>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let datasets_dir = resolve_options_datasets_dir(&app_local_data_dir)?;
        if !datasets_dir.is_dir() {
            return Ok(Vec::new());
        }
        scan_options_dataset_manifests(&datasets_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_options_dataset_manifest(
    options_dataset_id: String,
    app: tauri::AppHandle,
) -> Result<OptionsDatasetManifestV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let datasets_dir = resolve_options_datasets_dir(&app_local_data_dir)?;
        let dataset_id = sanitize_dataset_id(options_dataset_id.trim())?;
        let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
        if !manifest_path.is_file() {
            return Err(format!(
                "options dataset manifest not found for '{}' at {}",
                dataset_id,
                manifest_path.display()
            ));
        }
        read_options_dataset_manifest(&manifest_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn download_polygon_options_dataset_blocking_headless(
    req: PolygonOptionsDatasetDownloadRequest,
    app_identifier: &str,
    app_local_data_dir: &std::path::Path,
) -> Result<PolygonOptionsDatasetDownloadResponse, String> {
    download_polygon_options_dataset_into_store(&req, app_identifier, app_local_data_dir)
}

#[tauri::command]
pub async fn download_polygon_options_dataset(
    req: PolygonOptionsDatasetDownloadRequest,
    app: tauri::AppHandle,
) -> Result<PolygonOptionsDatasetDownloadResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let app_identifier = app.config().identifier.clone();
        download_polygon_options_dataset_into_store(&req, &app_identifier, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_local_data_dir(test_name: &str) -> std::path::PathBuf {
        let unique = format!(
            "ta_options_data_cmds_{}_{}",
            test_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn contract_type_filter_parses_common_aliases() {
        assert!(matches!(
            OptionContractTypeFilter::parse(None).expect("default"),
            OptionContractTypeFilter::Both
        ));
        assert!(matches!(
            OptionContractTypeFilter::parse(Some("call")).expect("call"),
            OptionContractTypeFilter::Call
        ));
        assert!(matches!(
            OptionContractTypeFilter::parse(Some("P")).expect("put"),
            OptionContractTypeFilter::Put
        ));
    }

    #[test]
    fn underlying_price_lookup_uses_latest_available_bar() {
        let mut map = BTreeMap::new();
        map.insert(1000, 500.0);
        map.insert(2000, 501.0);
        assert_eq!(underlying_price_at_or_before(&map, 999), None);
        assert_eq!(underlying_price_at_or_before(&map, 1000), Some(500.0));
        assert_eq!(underlying_price_at_or_before(&map, 1500), Some(500.0));
        assert_eq!(underlying_price_at_or_before(&map, 2500), Some(501.0));
    }

    #[test]
    fn polygon_download_dataset_id_defaults_to_root_and_range() {
        let req = PolygonOptionsDatasetDownloadRequest {
            root_symbol: "SPY".to_string(),
            from: "2026-04-01".to_string(),
            to: "2026-04-05".to_string(),
            options_dataset_id: None,
            contract_type: None,
            expiration_date_from: None,
            expiration_date_to: None,
            max_dte_days: None,
            strike_min: None,
            strike_max: None,
            max_contracts: None,
            provider_entitlement_profile: None,
            contract_reference_as_of_utc: None,
        };
        assert_eq!(
            derive_polygon_download_dataset_id(&req),
            "spy_options_1m_2026_04_01_2026_04_05"
        );
    }

    #[test]
    fn polygon_download_dataset_id_adds_material_filter_suffixes() {
        let req = PolygonOptionsDatasetDownloadRequest {
            root_symbol: "SPY".to_string(),
            from: "2026-04-01".to_string(),
            to: "2026-04-05".to_string(),
            options_dataset_id: None,
            contract_type: Some("call".to_string()),
            expiration_date_from: Some("2026-04-03".to_string()),
            expiration_date_to: Some("2026-04-12".to_string()),
            max_dte_days: Some(14),
            strike_min: Some(410.5),
            strike_max: Some(420.0),
            max_contracts: Some(50),
            provider_entitlement_profile: None,
            contract_reference_as_of_utc: Some("2026-04-05T23:59:00Z".to_string()),
        };

        assert_eq!(
            derive_polygon_download_dataset_id(&req),
            "spy_options_1m_2026_04_01_2026_04_05_call_exp_from_2026_04_03_exp_to_2026_04_12_max_dte_14_strike_min_410_5_strike_max_420_max_contracts_50_as_of_2026_04_05t23_59_00z"
        );
    }

    #[test]
    fn contract_type_filter_both_skips_unsupported_provider_values() {
        assert!(OptionContractTypeFilter::Both.matches_contract("call"));
        assert!(OptionContractTypeFilter::Both.matches_contract("p"));
        assert!(!OptionContractTypeFilter::Both.matches_contract("other"));
    }

    #[test]
    fn provider_source_surface_defaults_follow_price_semantics() {
        assert_eq!(
            canonicalize_provider_source_surface("", OptionsPriceSemantics::MinuteAggregate)
                .expect("minute aggregate default"),
            "minute_aggregate"
        );
        assert_eq!(
            canonicalize_provider_source_surface("", OptionsPriceSemantics::DerivedMidSnapshot)
                .expect("derived mid default"),
            "derived_mid_snapshot"
        );
        assert_eq!(
            canonicalize_provider_source_surface("", OptionsPriceSemantics::TopOfBookQuote)
                .expect("quote default"),
            "top_of_book_quote"
        );
    }

    #[test]
    fn polygon_download_import_request_freezes_aggregate_lane_invariants() {
        let req = PolygonOptionsDatasetDownloadRequest {
            root_symbol: "SPY".to_string(),
            from: "2026-04-01".to_string(),
            to: "2026-04-05".to_string(),
            options_dataset_id: None,
            contract_type: None,
            expiration_date_from: None,
            expiration_date_to: None,
            max_dte_days: None,
            strike_min: None,
            strike_max: None,
            max_contracts: None,
            provider_entitlement_profile: Some("options_business".to_string()),
            contract_reference_as_of_utc: Some("2026-04-05T23:59:00Z".to_string()),
        };
        let dataset_path = std::path::Path::new("C:/datasets/spy_options/rows.csv");
        let as_of = NaiveDate::from_ymd_opt(2026, 4, 5).expect("as_of");

        let build_req = build_polygon_download_import_request(
            &req,
            "spy_options_1m",
            dataset_path,
            "SPY",
            &as_of,
        );

        assert_eq!(build_req.options_dataset_id, "spy_options_1m");
        assert_eq!(build_req.dataset_path, dataset_path.to_string_lossy());
        assert_eq!(build_req.root_symbol.as_deref(), Some("SPY"));
        assert_eq!(build_req.provider_name, "massive");
        assert_eq!(build_req.provider_source_surface, "minute_aggregate");
        assert_eq!(build_req.quote_resolution, "1m");
        assert_eq!(
            build_req.price_semantics,
            OptionsPriceSemantics::MinuteAggregate
        );
        assert_eq!(
            build_req.contract_reference_source,
            "massive_contract_reference"
        );
        assert_eq!(
            build_req.contract_reference_as_of_utc.as_deref(),
            Some("2026-04-05T23:59:00Z")
        );
        assert_eq!(
            build_req.underlier_reference_source,
            "polygon_stocks_1m_raw:SPY"
        );
        assert_eq!(build_req.aggregate_price_field.as_deref(), Some("close"));
    }

    #[test]
    fn minute_aggregate_import_rejects_non_close_aggregate_price_field() {
        let app_local_data_dir = temp_app_local_data_dir("aggregate_price_field_reject");
        let source_csv = app_local_data_dir.join("source.csv");
        std::fs::write(&source_csv, "contract_id,timestamp_ms,close\nstub,1,1.0\n")
            .expect("write source csv");

        let req = OptionsDatasetImportRequest {
            csv_path: source_csv.to_string_lossy().to_string(),
            options_dataset_id: Some("spy_bad_aggregate_field".to_string()),
            root_symbol: Some("SPY".to_string()),
            provider_name: "massive".to_string(),
            provider_source_surface: "minute_aggregate".to_string(),
            provider_entitlement_profile: None,
            quote_resolution: "1m".to_string(),
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            calendar_id: "market_hours_utc".to_string(),
            timezone: "UTC".to_string(),
            normalization_version: Some(1),
            quote_semantics_version: Some(1),
            corporate_action_mapping_version: Some(1),
            contract_reference_source: "massive_contract_reference".to_string(),
            contract_reference_as_of_utc: None,
            underlier_reference_source: "underlying_dataset".to_string(),
            aggregate_price_field: Some("open".to_string()),
        };

        let error =
            import_options_dataset_blocking_headless(req, &app_local_data_dir).expect_err("reject");
        assert!(error.contains("aggregate_price_field='close'"));

        let datasets_dir = options_datasets_dir(&app_local_data_dir).expect("datasets dir");
        let manifest_path = dataset_manifest_path(&datasets_dir, "spy_bad_aggregate_field");
        assert!(!manifest_path.exists());

        let _ = std::fs::remove_dir_all(&app_local_data_dir);
    }

    #[test]
    fn parse_retry_after_ms_accepts_seconds_and_http_date_and_clamps() {
        let now = DateTime::parse_from_rfc3339("2026-04-01T12:00:00Z")
            .expect("parse now")
            .with_timezone(&Utc);

        assert_eq!(parse_retry_after_ms("7", now, 30_000), Some(7_000));
        assert_eq!(
            parse_retry_after_ms("Wed, 01 Apr 2026 12:00:05 GMT", now, 30_000),
            Some(5_000)
        );
        assert_eq!(
            parse_retry_after_ms("Wed, 01 Apr 2026 12:10:00 GMT", now, 30_000),
            Some(30_000)
        );
    }

    #[test]
    fn resolve_options_datasets_dir_does_not_create_directories() {
        let app_local_data_dir = temp_app_local_data_dir("read_only_datasets_dir");
        let datasets_dir = resolve_options_datasets_dir(&app_local_data_dir).expect("resolve dir");

        assert!(!datasets_dir.exists());

        let _ = std::fs::remove_dir_all(&app_local_data_dir);
    }

    #[test]
    fn polygon_download_preflight_rejects_inverted_strike_range_before_io() {
        let app_local_data_dir = temp_app_local_data_dir("polygon_strike_range_preflight");
        let req = PolygonOptionsDatasetDownloadRequest {
            root_symbol: "SPY".to_string(),
            from: "2026-04-01".to_string(),
            to: "2026-04-05".to_string(),
            options_dataset_id: None,
            contract_type: None,
            expiration_date_from: None,
            expiration_date_to: None,
            max_dte_days: None,
            strike_min: Some(430.0),
            strike_max: Some(420.0),
            max_contracts: None,
            provider_entitlement_profile: None,
            contract_reference_as_of_utc: None,
        };

        let error = download_polygon_options_dataset_blocking_headless(
            req,
            "com.vectorgrid.tests",
            &app_local_data_dir,
        )
        .expect_err("invalid strike range should fail");
        assert!(error.contains("strike_min must be <= strike_max"));

        let _ = std::fs::remove_dir_all(&app_local_data_dir);
    }
}
