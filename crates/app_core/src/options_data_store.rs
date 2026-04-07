use crate::options_types::{
    ImportedOptionsDataset, OptionType, OptionsContractSeries, OptionsDatasetManifestV1,
    OptionsPriceRow, OptionsPriceSemantics, OPTIONS_DATASET_MANIFEST_VERSION,
};
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder, StringRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportedOptionsDatasetCsvBuildRequest {
    pub options_dataset_id: String,
    pub dataset_path: String,
    pub root_symbol: Option<String>,
    pub provider_name: String,
    pub provider_source_surface: String,
    pub provider_entitlement_profile: Option<String>,
    pub quote_resolution: String,
    pub price_semantics: OptionsPriceSemantics,
    pub calendar_id: String,
    pub timezone: String,
    pub normalization_version: u32,
    pub quote_semantics_version: u32,
    pub corporate_action_mapping_version: u32,
    pub contract_reference_source: String,
    pub contract_reference_as_of_utc: Option<String>,
    pub underlier_reference_source: String,
    pub aggregate_price_field: Option<String>,
}

#[derive(Clone, Debug)]
struct NormalizedCsvRow {
    root_symbol: String,
    contract_id: String,
    option_type: OptionType,
    expiration_date: String,
    expiration_timestamp_ms: i64,
    strike: f64,
    multiplier: f64,
    row: OptionsPriceRow,
}

#[derive(Clone, Debug, Default)]
struct HeaderIndex {
    root_symbol: Option<usize>,
    contract_id: usize,
    option_type: usize,
    expiration_date: Option<usize>,
    expiration_timestamp_ms: usize,
    strike: usize,
    multiplier: Option<usize>,
    timestamp_ms: usize,
    open: usize,
    high: usize,
    low: usize,
    close: usize,
    underlying_price: usize,
    bid: Option<usize>,
    ask: Option<usize>,
    mid: Option<usize>,
    volume: Option<usize>,
    open_interest: Option<usize>,
    quote_age_ms: Option<usize>,
    session_id: Option<usize>,
}

fn normalize_header_name(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '.'], "_")
}

fn header_index(headers: &StringRecord) -> Result<HeaderIndex, String> {
    let mut by_name: BTreeMap<String, usize> = BTreeMap::new();
    for (idx, header) in headers.iter().enumerate() {
        by_name.insert(normalize_header_name(header), idx);
    }
    let require = |name: &str| {
        by_name
            .get(name)
            .copied()
            .ok_or_else(|| format!("normalized options CSV is missing required column '{name}'"))
    };
    Ok(HeaderIndex {
        root_symbol: by_name.get("root_symbol").copied(),
        contract_id: require("contract_id")?,
        option_type: require("option_type")?,
        expiration_date: by_name.get("expiration_date").copied(),
        expiration_timestamp_ms: require("expiration_timestamp_ms")?,
        strike: require("strike")?,
        multiplier: by_name.get("multiplier").copied(),
        timestamp_ms: require("timestamp_ms")?,
        open: require("open")?,
        high: require("high")?,
        low: require("low")?,
        close: require("close")?,
        underlying_price: require("underlying_price")?,
        bid: by_name.get("bid").copied(),
        ask: by_name.get("ask").copied(),
        mid: by_name.get("mid").copied(),
        volume: by_name.get("volume").copied(),
        open_interest: by_name.get("open_interest").copied(),
        quote_age_ms: by_name.get("quote_age_ms").copied(),
        session_id: by_name.get("session_id").copied(),
    })
}

fn field<'a>(record: &'a StringRecord, idx: usize, name: &str) -> Result<&'a str, String> {
    record
        .get(idx)
        .map(str::trim)
        .ok_or_else(|| format!("missing field '{name}' in normalized options CSV row"))
}

fn parse_optional_f64(record: &StringRecord, idx: Option<usize>) -> Result<Option<f64>, String> {
    let Some(idx) = idx else {
        return Ok(None);
    };
    let raw = record.get(idx).map(str::trim).unwrap_or_default();
    if raw.is_empty() {
        return Ok(None);
    }
    let value = raw
        .parse::<f64>()
        .map_err(|_| format!("invalid floating-point value '{raw}'"))?;
    if !value.is_finite() {
        return Err(format!("non-finite floating-point value '{raw}'"));
    }
    Ok(Some(value))
}

fn parse_optional_i64(record: &StringRecord, idx: Option<usize>) -> Result<Option<i64>, String> {
    let Some(idx) = idx else {
        return Ok(None);
    };
    let raw = record.get(idx).map(str::trim).unwrap_or_default();
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse::<i64>()
        .map(Some)
        .map_err(|_| format!("invalid integer value '{raw}'"))
}

fn parse_optional_string(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    idx.and_then(|col| record.get(col))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_root_symbol(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn parse_option_type(raw: &str) -> Result<OptionType, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "call" | "c" => Ok(OptionType::Call),
        "put" | "p" => Ok(OptionType::Put),
        other => Err(format!(
            "invalid option_type '{other}' (expected call/put or c/p)"
        )),
    }
}

fn derive_expiration_date(expiration_timestamp_ms: i64) -> Result<String, String> {
    let dt = DateTime::<Utc>::from_timestamp_millis(expiration_timestamp_ms)
        .ok_or_else(|| format!("invalid expiration_timestamp_ms {expiration_timestamp_ms}"))?;
    Ok(dt.format("%Y-%m-%d").to_string())
}

fn parse_row(
    record: &StringRecord,
    header: &HeaderIndex,
    explicit_root_symbol: Option<&str>,
) -> Result<NormalizedCsvRow, String> {
    let root_symbol = if let Some(symbol) = explicit_root_symbol {
        normalize_root_symbol(symbol)
    } else {
        normalize_root_symbol(field(
            record,
            header.root_symbol.ok_or_else(|| {
                "normalized options CSV is missing required column 'root_symbol'".to_string()
            })?,
            "root_symbol",
        )?)
    };
    if root_symbol.is_empty() {
        return Err("root_symbol is empty".to_string());
    }
    let contract_id = field(record, header.contract_id, "contract_id")?.to_string();
    if contract_id.trim().is_empty() {
        return Err("contract_id is empty".to_string());
    }
    let option_type = parse_option_type(field(record, header.option_type, "option_type")?)?;
    let expiration_timestamp_ms = field(
        record,
        header.expiration_timestamp_ms,
        "expiration_timestamp_ms",
    )?
    .parse::<i64>()
    .map_err(|_| "invalid expiration_timestamp_ms".to_string())?;
    let expiration_date = if let Some(idx) = header.expiration_date {
        let raw = field(record, idx, "expiration_date")?;
        if raw.is_empty() {
            derive_expiration_date(expiration_timestamp_ms)?
        } else {
            raw.to_string()
        }
    } else {
        derive_expiration_date(expiration_timestamp_ms)?
    };
    let strike = field(record, header.strike, "strike")?
        .parse::<f64>()
        .map_err(|_| "invalid strike".to_string())?;
    if !strike.is_finite() || strike <= 0.0 {
        return Err("strike must be finite and > 0".to_string());
    }
    let multiplier = match header.multiplier {
        Some(idx) => {
            let raw = field(record, idx, "multiplier")?;
            if raw.is_empty() {
                100.0
            } else {
                raw.parse::<f64>()
                    .map_err(|_| "invalid multiplier".to_string())?
            }
        }
        None => 100.0,
    };
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err("multiplier must be finite and > 0".to_string());
    }
    let timestamp_ms = field(record, header.timestamp_ms, "timestamp_ms")?
        .parse::<i64>()
        .map_err(|_| "invalid timestamp_ms".to_string())?;
    let open = field(record, header.open, "open")?
        .parse::<f64>()
        .map_err(|_| "invalid open".to_string())?;
    let high = field(record, header.high, "high")?
        .parse::<f64>()
        .map_err(|_| "invalid high".to_string())?;
    let low = field(record, header.low, "low")?
        .parse::<f64>()
        .map_err(|_| "invalid low".to_string())?;
    let close = field(record, header.close, "close")?
        .parse::<f64>()
        .map_err(|_| "invalid close".to_string())?;
    let underlying_price = field(record, header.underlying_price, "underlying_price")?
        .parse::<f64>()
        .map_err(|_| "invalid underlying_price".to_string())?;
    for (label, value) in [
        ("open", open),
        ("high", high),
        ("low", low),
        ("close", close),
        ("underlying_price", underlying_price),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("{label} must be finite and >= 0"));
        }
    }

    Ok(NormalizedCsvRow {
        root_symbol,
        contract_id,
        option_type,
        expiration_date,
        expiration_timestamp_ms,
        strike,
        multiplier,
        row: OptionsPriceRow {
            timestamp_ms,
            open,
            high,
            low,
            close,
            underlying_price,
            bid: parse_optional_f64(record, header.bid)?,
            ask: parse_optional_f64(record, header.ask)?,
            mid: parse_optional_f64(record, header.mid)?,
            volume: parse_optional_f64(record, header.volume)?,
            open_interest: parse_optional_f64(record, header.open_interest)?,
            quote_age_ms: parse_optional_i64(record, header.quote_age_ms)?,
            session_id: parse_optional_string(record, header.session_id),
        },
    })
}

pub fn load_imported_options_dataset_from_csv(
    req: &ImportedOptionsDatasetCsvBuildRequest,
) -> Result<ImportedOptionsDataset, String> {
    let dataset_path = Path::new(req.dataset_path.trim());
    if !dataset_path.is_file() {
        return Err(format!(
            "options dataset CSV does not exist: {}",
            dataset_path.display()
        ));
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(dataset_path)
        .map_err(|e| format!("failed to open options dataset CSV: {e}"))?;
    let headers = reader
        .headers()
        .map_err(|e| format!("failed to read CSV headers: {e}"))?
        .clone();
    let header = header_index(&headers)?;

    let explicit_root_symbol = req
        .root_symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut grouped: BTreeMap<String, OptionsContractSeries> = BTreeMap::new();
    let mut row_count = 0usize;
    let mut coverage_start: Option<i64> = None;
    let mut coverage_end: Option<i64> = None;
    let mut inferred_root_symbol: Option<String> = explicit_root_symbol.map(normalize_root_symbol);

    for result in reader.records() {
        let record =
            result.map_err(|e| format!("failed to read normalized options CSV row: {e}"))?;
        let row = parse_row(&record, &header, explicit_root_symbol)?;
        if let Some(current) = inferred_root_symbol.as_ref() {
            if current != &row.root_symbol {
                return Err(format!(
                    "root_symbol mismatch: dataset already inferred '{}', saw '{}'",
                    current, row.root_symbol
                ));
            }
        } else {
            inferred_root_symbol = Some(row.root_symbol.clone());
        }
        let start = coverage_start.get_or_insert(row.row.timestamp_ms);
        if row.row.timestamp_ms < *start {
            *start = row.row.timestamp_ms;
        }
        let end = coverage_end.get_or_insert(row.row.timestamp_ms);
        if row.row.timestamp_ms > *end {
            *end = row.row.timestamp_ms;
        }
        row_count += 1;
        grouped
            .entry(row.contract_id.clone())
            .and_modify(|series| series.rows.push(row.row.clone()))
            .or_insert_with(|| OptionsContractSeries {
                root_symbol: row.root_symbol.clone(),
                contract_id: row.contract_id,
                option_type: row.option_type,
                expiration_date: row.expiration_date,
                expiration_timestamp_ms: row.expiration_timestamp_ms,
                strike: row.strike,
                multiplier: row.multiplier,
                rows: vec![row.row],
            });
    }

    if row_count == 0 {
        return Err("normalized options CSV is empty".to_string());
    }

    let root_symbol = inferred_root_symbol
        .ok_or_else(|| "failed to infer root_symbol for dataset".to_string())?;
    let coverage_start =
        coverage_start.ok_or_else(|| "failed to derive coverage_start from dataset".to_string())?;
    let coverage_end =
        coverage_end.ok_or_else(|| "failed to derive coverage_end from dataset".to_string())?;

    let mut contracts: Vec<OptionsContractSeries> = grouped.into_values().collect();
    for contract in &mut contracts {
        contract
            .rows
            .sort_by(|a, b| a.timestamp_ms.cmp(&b.timestamp_ms));
        contract
            .rows
            .dedup_by(|a, b| a.timestamp_ms == b.timestamp_ms);
    }

    let manifest = OptionsDatasetManifestV1 {
        version: OPTIONS_DATASET_MANIFEST_VERSION,
        options_dataset_id: req.options_dataset_id.clone(),
        root_symbol,
        provider_name: req.provider_name.clone(),
        provider_source_surface: req.provider_source_surface.clone(),
        provider_entitlement_profile: req.provider_entitlement_profile.clone(),
        quote_resolution: req.quote_resolution.clone(),
        price_semantics: req.price_semantics,
        calendar_id: req.calendar_id.clone(),
        timezone: req.timezone.clone(),
        coverage_start,
        coverage_end,
        normalization_version: req.normalization_version,
        quote_semantics_version: req.quote_semantics_version,
        corporate_action_mapping_version: req.corporate_action_mapping_version,
        contract_reference_source: req.contract_reference_source.clone(),
        contract_reference_as_of_utc: req.contract_reference_as_of_utc.clone(),
        underlier_reference_source: req.underlier_reference_source.clone(),
        dataset_path: req.dataset_path.clone(),
        row_count,
        contract_count: contracts.len(),
        aggregate_price_field: req.aggregate_price_field.clone(),
    };
    Ok(ImportedOptionsDataset {
        manifest,
        contracts,
    })
}

pub fn read_options_dataset_manifest(path: &Path) -> Result<OptionsDatasetManifestV1, String> {
    let bytes = std::fs::read(path).map_err(|e| {
        format!(
            "failed to read options dataset manifest {}: {e}",
            path.display()
        )
    })?;
    serde_json::from_slice::<OptionsDatasetManifestV1>(&bytes).map_err(|e| {
        format!(
            "failed to parse options dataset manifest {}: {e}",
            path.display()
        )
    })
}

pub fn write_options_dataset_manifest(
    path: &Path,
    manifest: &OptionsDatasetManifestV1,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize options dataset manifest: {e}"))?;
    std::fs::write(path, bytes).map_err(|e| {
        format!(
            "failed to write options dataset manifest {}: {e}",
            path.display()
        )
    })
}

pub fn scan_options_dataset_manifests(dir: &Path) -> Result<Vec<OptionsDatasetManifestV1>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut manifests: Vec<OptionsDatasetManifestV1> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        manifests.push(read_options_dataset_manifest(&manifest_path)?);
    }
    manifests.sort_by(|a, b| a.options_dataset_id.cmp(&b.options_dataset_id));
    Ok(manifests)
}

pub fn dataset_manifest_path(root_dir: &Path, dataset_id: &str) -> PathBuf {
    root_dir.join(dataset_id).join("manifest.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options_types::OptionsPriceSemantics;

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

    fn write_dataset(path: &Path) {
        let data = concat!(
            "root_symbol,contract_id,option_type,expiration_date,expiration_timestamp_ms,strike,multiplier,timestamp_ms,open,high,low,close,underlying_price,volume,open_interest\n",
            "SPY,SPY_C_001,call,2026-04-10,1775779200000,500,100,1775160000000,2.0,2.3,1.9,2.1,498.0,1000,2000\n",
            "SPY,SPY_C_001,call,2026-04-10,1775779200000,500,100,1775160060000,2.1,2.4,2.0,2.2,498.5,1100,2000\n",
            "SPY,SPY_P_001,put,2026-04-10,1775779200000,495,100,1775160000000,1.8,1.9,1.7,1.75,498.0,800,1500\n"
        );
        std::fs::write(path, data).expect("write dataset csv");
    }

    #[test]
    fn load_imported_dataset_groups_contract_rows() {
        let temp = make_temp_dir("options_dataset_load");
        let csv_path = temp.join("normalized.csv");
        write_dataset(&csv_path);
        let req = ImportedOptionsDatasetCsvBuildRequest {
            options_dataset_id: "spy_1m".to_string(),
            dataset_path: csv_path.to_string_lossy().to_string(),
            root_symbol: None,
            provider_name: "massive".to_string(),
            provider_source_surface: "minute_aggregate".to_string(),
            provider_entitlement_profile: None,
            quote_resolution: "1m".to_string(),
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            calendar_id: "market_hours_utc".to_string(),
            timezone: "UTC".to_string(),
            normalization_version: 1,
            quote_semantics_version: 1,
            corporate_action_mapping_version: 1,
            contract_reference_source: "massive_contract_reference".to_string(),
            contract_reference_as_of_utc: None,
            underlier_reference_source: "underlying_dataset".to_string(),
            aggregate_price_field: Some("close".to_string()),
        };

        let dataset = load_imported_options_dataset_from_csv(&req).expect("dataset");
        assert_eq!(dataset.manifest.root_symbol, "SPY");
        assert_eq!(dataset.manifest.row_count, 3);
        assert_eq!(dataset.manifest.contract_count, 2);
        assert_eq!(dataset.contracts[0].rows.len(), 2);
    }
}
