use std::fs::File;
use std::io::{BufRead, BufReader};

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

/// CSV reader used by the desktop app.
///
/// Supports:
/// - VectorTA positional CSV: `timestamp,open,close,high,low,volume` (or just `timestamp,close`)
/// - Polygon "raw aggregates" CSV: `v,vw,o,c,h,l,t,n,...` (maps `t,o,c,h,l,v`)
pub fn read_candles_from_csv_compat(file_path: &str) -> Result<Candles, String> {
    let file = File::open(file_path).map_err(|e| format!("failed to open CSV: {e}"))?;
    let mut lines = BufReader::new(file).lines();

    let header_line = loop {
        match lines.next() {
            Some(Ok(line)) => {
                let t = line.trim();
                if t.is_empty() {
                    continue;
                }
                break t.to_string();
            }
            Some(Err(e)) => return Err(format!("failed to read CSV header: {e}")),
            None => return Err("CSV is empty".to_string()),
        }
    };

    let headers: Vec<String> = header_line
        .split(',')
        .map(normalize_header)
        .collect();

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

    for (line_idx, line) in lines.enumerate() {
        let line = line.map_err(|e| format!("failed to read CSV line {}: {e}", line_idx + 2))?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        let mut ts_s: Option<&str> = None;
        let mut c_s: Option<&str> = None;
        let mut o_s: Option<&str> = None;
        let mut h_s: Option<&str> = None;
        let mut l_s: Option<&str> = None;
        let mut v_s: Option<&str> = None;

        let need_open = idx.open.is_some();
        let need_high = idx.high.is_some();
        let need_low = idx.low.is_some();
        let need_vol = idx.volume.is_some();

        let mut got_ts = false;
        let mut got_c = false;
        let mut got_o = !need_open;
        let mut got_h = !need_high;
        let mut got_l = !need_low;
        let mut got_v = !need_vol;

        for (i, part) in t.split(',').enumerate() {
            if i == idx.ts {
                ts_s = Some(part);
                got_ts = true;
            }
            if i == idx.close {
                c_s = Some(part);
                got_c = true;
            }
            if idx.open == Some(i) {
                o_s = Some(part);
                got_o = true;
            }
            if idx.high == Some(i) {
                h_s = Some(part);
                got_h = true;
            }
            if idx.low == Some(i) {
                l_s = Some(part);
                got_l = true;
            }
            if idx.volume == Some(i) {
                v_s = Some(part);
                got_v = true;
            }

            if i >= wanted_max && got_ts && got_c && got_o && got_h && got_l && got_v {
                break;
            }
        }

        let ts_raw = ts_s.ok_or_else(|| format!("missing timestamp column at line {}", line_idx + 2))?;
        let ts = parse_i64_flexible(ts_raw)
            .map_err(|e| format!("invalid timestamp '{}' at line {} ({e})", ts_raw.trim(), line_idx + 2))?;

        let c_raw = c_s.ok_or_else(|| format!("missing close column at line {}", line_idx + 2))?;
        let c = c_raw
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("invalid close '{}' at line {}", c_raw.trim(), line_idx + 2))?;
        if !c.is_finite() {
            return Err(format!("close is not finite at line {}", line_idx + 2));
        }

        let o = parse_f64_or_nan(o_s).map_err(|e| format!("{e} at line {}", line_idx + 2))?;
        let h = parse_f64_or_nan(h_s).map_err(|e| format!("{e} at line {}", line_idx + 2))?;
        let l = parse_f64_or_nan(l_s).map_err(|e| format!("{e} at line {}", line_idx + 2))?;
        let v = parse_f64_or_nan(v_s).map_err(|e| format!("{e} at line {}", line_idx + 2))?;

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

    #[test]
    fn parses_polygon_raw_csv_columns() {
        let tmp = std::env::temp_dir().join(format!(
            "vta_csv_compat_test_{}.csv",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));

        let csv = concat!(
            "v,vw,o,c,h,l,t,n,a,op\n",
            "13468.0,23.2143,23.2143,23.2143,23.2143,23.2143,1350460800000,2.0,,\n",
            "3192.0,23.2318,23.2318,23.2318,23.2318,23.2318,1350460920000,1.0,,\n"
        );
        std::fs::write(&tmp, csv).expect("write tmp csv");

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
}

