use std::fs::File;

use csv::{ReaderBuilder, StringRecord, Trim};
use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

#[derive(Clone, Copy, Debug)]
struct ColumnIdx {
    ts: usize,
    close: usize,
    open: Option<usize>,
    high: Option<usize>,
    low: Option<usize>,
    volume: Option<usize>,
}

impl ColumnIdx {
    fn max_idx(&self) -> usize {
        let mut m = self.ts.max(self.close);
        if let Some(i) = self.open {
            m = m.max(i);
        }
        if let Some(i) = self.high {
            m = m.max(i);
        }
        if let Some(i) = self.low {
            m = m.max(i);
        }
        if let Some(i) = self.volume {
            m = m.max(i);
        }
        m
    }
}

fn normalize_header(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_ascii_lowercase()
}

fn find_idx(headers: &[String], candidates: &[&str]) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        for &c in candidates {
            if h == c {
                return Some(i);
            }
        }
    }
    None
}

fn infer_column_mapping(headers: &[String]) -> Result<(ColumnIdx, CandleFieldFlags), String> {
    let header_len = headers.len();
    if header_len < 2 {
        return Err("CSV must have at least 2 columns: timestamp, close (or t, c)".to_string());
    }

    // If we can map timestamp+close by name, assume header-based format (supports Polygon raw CSV:
    // v,vw,o,c,h,l,t,n,... as well as timestamp,open,close,high,low,volume).
    let idx_ts_named = find_idx(headers, &["timestamp", "t", "time", "ts"]);
    let idx_close_named = find_idx(headers, &["close", "c"]);
    let use_named = idx_ts_named.is_some() && idx_close_named.is_some();

    let (idx, fields) = if use_named {
        let ts = idx_ts_named.unwrap();
        let close = idx_close_named.unwrap();
        let open = find_idx(headers, &["open", "o"]);
        let high = find_idx(headers, &["high", "h"]);
        let low = find_idx(headers, &["low", "l"]);
        let volume = find_idx(headers, &["volume", "v", "vol"]);
        (
            ColumnIdx {
                ts,
                close,
                open,
                high,
                low,
                volume,
            },
            CandleFieldFlags {
                open: open.is_some(),
                close: true,
                high: high.is_some(),
                low: low.is_some(),
                volume: volume.is_some(),
            },
        )
    } else {
        // Fallback: VectorTA positional format.
        // - 2 cols: timestamp, close
        // - 3+ cols: timestamp, open, close, high?, low?, volume?
        let open = if header_len >= 3 { Some(1usize) } else { None };
        let close = if header_len >= 3 { 2usize } else { 1usize };
        let high = if header_len > 3 { Some(3usize) } else { None };
        let low = if header_len > 4 { Some(4usize) } else { None };
        let volume = if header_len > 5 { Some(5usize) } else { None };
        (
            ColumnIdx {
                ts: 0,
                close,
                open,
                high,
                low,
                volume,
            },
            CandleFieldFlags {
                open: open.is_some(),
                close: true,
                high: high.is_some(),
                low: low.is_some(),
                volume: volume.is_some(),
            },
        )
    };

    Ok((idx, fields))
}

fn parse_i64_flexible(s: &str) -> Result<i64, String> {
    let t = s.trim();
    if t.is_empty() {
        return Err("empty".to_string());
    }
    if let Ok(v) = t.parse::<i64>() {
        return Ok(v);
    }
    let f = t
        .parse::<f64>()
        .map_err(|_| "not an integer or float".to_string())?;
    if !f.is_finite() {
        return Err("not finite".to_string());
    }
    let r = f.round();
    if r < (i64::MIN as f64) || r > (i64::MAX as f64) {
        return Err("out of i64 range".to_string());
    }
    Ok(r as i64)
}

fn parse_f64_or_nan(s: Option<&str>) -> Result<f64, String> {
    let Some(raw) = s else {
        return Ok(f64::NAN);
    };
    let t = raw.trim();
    if t.is_empty() {
        return Ok(f64::NAN);
    }
    t.parse::<f64>().map_err(|_| format!("invalid float '{t}'"))
}

fn csv_record_is_empty(record: &StringRecord) -> bool {
    record.iter().all(|field| field.trim().is_empty())
}

fn csv_record_line_num(record: &StringRecord, fallback: usize) -> usize {
    record
        .position()
        .and_then(|position| usize::try_from(position.line()).ok())
        .filter(|line_num| *line_num > 0)
        .unwrap_or(fallback)
}

fn format_csv_read_error(context: &str, error: csv::Error) -> String {
    let line_num = error
        .position()
        .and_then(|position| usize::try_from(position.line()).ok())
        .filter(|line| *line > 0);
    let detail = error.to_string();
    match line_num {
        Some(line_num) => format!("failed to read CSV {context} at line {line_num}: {detail}"),
        None => format!("failed to read CSV {context}: {detail}"),
    }
}

/// CSV reader used by the desktop app.
///
/// Supports:
/// - VectorTA positional CSV: `timestamp,open,close,high,low,volume` (or just `timestamp,close`)
/// - Polygon "raw aggregates" CSV: `v,vw,o,c,h,l,t,n,...` (maps `t,o,c,h,l,v`)
pub fn read_candles_from_csv_compat(file_path: &str) -> Result<Candles, String> {
    let file = File::open(file_path).map_err(|e| format!("failed to open CSV: {e}"))?;
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(Trim::All)
        .from_reader(file);
    let mut records = reader.records();

    let header_record = loop {
        match records.next() {
            Some(Ok(record)) => {
                if csv_record_is_empty(&record) {
                    continue;
                }
                break record;
            }
            Some(Err(error)) => return Err(format_csv_read_error("header", error)),
            None => return Err("CSV is empty".to_string()),
        }
    };

    let headers: Vec<String> = header_record.iter().map(normalize_header).collect();

    let (idx, fields_hdr) = infer_column_mapping(&headers)?;
    let wanted_max = idx.max_idx();

    let mut timestamp: Vec<i64> = Vec::new();
    let mut open: Vec<f64> = Vec::new();
    let mut high: Vec<f64> = Vec::new();
    let mut low: Vec<f64> = Vec::new();
    let mut close: Vec<f64> = Vec::new();
    let mut volume: Vec<f64> = Vec::new();

    let mut has_open_any = false;
    let mut has_high_any = false;
    let mut has_low_any = false;
    let mut has_volume_any = false;

    for (row_idx, record) in records.enumerate() {
        let record = record.map_err(|error| format_csv_read_error("record", error))?;
        if csv_record_is_empty(&record) {
            continue;
        }
        let line_num = csv_record_line_num(&record, row_idx + 2);

        if record.len() <= wanted_max {
            if record.get(idx.ts).is_none() {
                return Err(format!("missing timestamp column at line {line_num}"));
            }
            if record.get(idx.close).is_none() {
                return Err(format!("missing close column at line {line_num}"));
            }
        }

        let ts_raw = record
            .get(idx.ts)
            .ok_or_else(|| format!("missing timestamp column at line {line_num}"))?;
        let ts = parse_i64_flexible(ts_raw).map_err(|e| {
            format!(
                "invalid timestamp '{}' at line {line_num} ({e})",
                ts_raw.trim(),
            )
        })?;

        let c_raw = record
            .get(idx.close)
            .ok_or_else(|| format!("missing close column at line {line_num}"))?;
        let c = c_raw
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("invalid close '{}' at line {line_num}", c_raw.trim()))?;
        if !c.is_finite() {
            return Err(format!("close is not finite at line {line_num}"));
        }

        let o = parse_f64_or_nan(idx.open.and_then(|index| record.get(index)))
            .map_err(|e| format!("{e} at line {line_num}"))?;
        let h = parse_f64_or_nan(idx.high.and_then(|index| record.get(index)))
            .map_err(|e| format!("{e} at line {line_num}"))?;
        let l = parse_f64_or_nan(idx.low.and_then(|index| record.get(index)))
            .map_err(|e| format!("{e} at line {line_num}"))?;
        let v = parse_f64_or_nan(idx.volume.and_then(|index| record.get(index)))
            .map_err(|e| format!("{e} at line {line_num}"))?;

        if o.is_finite() {
            has_open_any = true;
        }
        if h.is_finite() {
            has_high_any = true;
        }
        if l.is_finite() {
            has_low_any = true;
        }
        if v.is_finite() {
            has_volume_any = true;
        }

        timestamp.push(ts);
        close.push(c);
        open.push(o);
        high.push(h);
        low.push(l);
        volume.push(v);
    }

    if timestamp.is_empty() {
        return Err("CSV has no data rows".to_string());
    }

    let fields = CandleFieldFlags {
        open: fields_hdr.open && has_open_any,
        close: true,
        high: fields_hdr.high && has_high_any,
        low: fields_hdr.low && has_low_any,
        volume: fields_hdr.volume && has_volume_any,
    };

    Ok(Candles::new_with_fields(
        timestamp, open, high, low, close, volume, fields,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_temp_csv(prefix: &str, csv: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "{prefix}_{}_{}.csv",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));
        std::fs::write(&tmp, csv).expect("write tmp csv");
        tmp
    }

    #[test]
    fn parses_polygon_raw_csv_columns() {
        let csv = concat!(
            "v,vw,o,c,h,l,t,n,a,op\n",
            "13468.0,23.2143,23.2143,23.2143,23.2143,23.2143,1350460800000,2.0,,\n",
            "3192.0,23.2318,23.2318,23.2318,23.2318,23.2318,1350460920000,1.0,,\n"
        );
        let tmp = write_temp_csv("vta_csv_compat_test_polygon", csv);

        let candles = read_candles_from_csv_compat(tmp.to_str().unwrap()).expect("parse");
        assert_eq!(candles.timestamp.len(), 2);
        assert_eq!(candles.timestamp[0], 1350460800000);
        assert_eq!(candles.open[0], 23.2143);
        assert_eq!(candles.close[0], 23.2143);
        assert_eq!(candles.high[0], 23.2143);
        assert_eq!(candles.low[0], 23.2143);
        assert_eq!(candles.volume[0], 13468.0);
        assert!(candles.fields.volume);
        assert!(candles.fields.high);
        assert!(candles.fields.low);
    }

    #[test]
    fn parses_named_alias_headers_and_drops_all_blank_optional_fields() {
        let csv = concat!(
            "time,o,c,h,l,v\n",
            "1350460800000,23.2143,23.5143,,,\n",
            "1350460920000,23.2318,23.5318,,,\n"
        );
        let tmp = write_temp_csv("vta_csv_compat_test_named_aliases", csv);

        let candles =
            read_candles_from_csv_compat(tmp.to_str().unwrap()).expect("parse named aliases");
        assert_eq!(candles.timestamp, vec![1350460800000, 1350460920000]);
        assert_eq!(candles.open[0], 23.2143);
        assert_eq!(candles.close[1], 23.5318);
        assert!(candles.fields.open);
        assert!(candles.fields.close);
        assert!(!candles.fields.high);
        assert!(!candles.fields.low);
        assert!(!candles.fields.volume);
        assert!(candles.high.iter().all(|value| value.is_nan()));
        assert!(candles.low.iter().all(|value| value.is_nan()));
        assert!(candles.volume.iter().all(|value| value.is_nan()));
    }

    #[test]
    fn falls_back_to_positional_layout_when_named_headers_are_not_recognized() {
        let csv = concat!(
            "timestamp,opening,closing,high,low,volume\n",
            "1350460800000,23.2143,23.5143,23.8143,23.0143,100.0\n"
        );
        let tmp = write_temp_csv("vta_csv_compat_test_positional_fallback", csv);

        let candles =
            read_candles_from_csv_compat(tmp.to_str().unwrap()).expect("parse positional fallback");
        assert_eq!(candles.timestamp[0], 1350460800000);
        assert_eq!(candles.open[0], 23.2143);
        assert_eq!(candles.close[0], 23.5143);
        assert_eq!(candles.high[0], 23.8143);
        assert_eq!(candles.low[0], 23.0143);
        assert_eq!(candles.volume[0], 100.0);
        assert!(candles.fields.open);
        assert!(candles.fields.high);
        assert!(candles.fields.low);
        assert!(candles.fields.volume);
    }

    #[test]
    fn reports_invalid_timestamps_clearly() {
        let csv = concat!("timestamp,close\n", "not_a_timestamp,23.5143\n");
        let tmp = write_temp_csv("vta_csv_compat_test_bad_timestamp", csv);

        let error = read_candles_from_csv_compat(tmp.to_str().unwrap())
            .expect_err("invalid timestamp should fail");
        assert!(error.contains("invalid timestamp"));
    }

    #[test]
    fn parses_quoted_headers_and_unused_fields_with_commas() {
        let csv = concat!(
            "\"timestamp\",\"close\",\"note\"\n",
            "\"1350460800000\",\"23.5143\",\"hello, world\"\n"
        );
        let tmp = write_temp_csv("vta_csv_compat_test_quoted_fields", csv);

        let candles =
            read_candles_from_csv_compat(tmp.to_str().unwrap()).expect("parse quoted csv");
        assert_eq!(candles.timestamp, vec![1350460800000]);
        assert_eq!(candles.close, vec![23.5143]);
        assert!(!candles.fields.open);
        assert!(!candles.fields.high);
        assert!(!candles.fields.low);
        assert!(!candles.fields.volume);
    }
}
