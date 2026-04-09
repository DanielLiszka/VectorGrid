use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

const DAY_MS: i64 = 86_400_000;

#[derive(Clone, Debug, PartialEq)]
pub struct TimeframeSpec {
    pub label: String,
    pub interval_ms: i64,
}

fn canonical_label_from_seconds(sec: i64) -> Option<&'static str> {
    match sec {
        60 => Some("1m"),
        120 => Some("2m"),
        180 => Some("3m"),
        300 => Some("5m"),
        900 => Some("15m"),
        1800 => Some("30m"),
        3600 => Some("1h"),
        7200 => Some("2h"),
        14_400 => Some("4h"),
        86_400 => Some("1d"),
        _ => None,
    }
}

pub fn parse_timeframe_label(raw: &str) -> Result<TimeframeSpec, String> {
    let s0 = raw.trim();
    if s0.is_empty() {
        return Err("timeframe is empty (expected e.g. '1m', '15m', '2h')".to_string());
    }
    let s = s0.to_ascii_lowercase();

    // Back-compat / convenience: accept seconds labels like "120s".
    if let Some(sec_str) = s.strip_suffix('s') {
        let sec: i64 = sec_str
            .parse()
            .map_err(|_| format!("invalid timeframe '{s0}' (expected e.g. 60s, 1m, 2h)"))?;
        if sec <= 0 {
            return Err(format!("invalid timeframe '{s0}' (seconds must be > 0)"));
        }
        let label = canonical_label_from_seconds(sec)
            .map(|x| x.to_string())
            .unwrap_or_else(|| format!("{sec}s"));
        return Ok(TimeframeSpec {
            label,
            interval_ms: sec
                .checked_mul(1_000)
                .ok_or_else(|| "timeframe overflows i64".to_string())?,
        });
    }

    let (num_str, unit_str) = s.split_at(s.len().saturating_sub(1));
    let unit = unit_str.chars().next().unwrap_or('m');
    let n: i64 = num_str
        .parse()
        .map_err(|_| format!("invalid timeframe '{s0}' (expected e.g. 1m, 15m, 2h)"))?;
    if n <= 0 {
        return Err(format!("invalid timeframe '{s0}' (multiplier must be > 0)"));
    }

    let interval_ms = match unit {
        'm' => n
            .checked_mul(60_000)
            .ok_or_else(|| "timeframe overflows i64".to_string())?,
        'h' => n
            .checked_mul(3_600_000)
            .ok_or_else(|| "timeframe overflows i64".to_string())?,
        'd' => n
            .checked_mul(DAY_MS)
            .ok_or_else(|| "timeframe overflows i64".to_string())?,
        _ => {
            return Err(format!(
                "invalid timeframe '{s0}' (supported suffixes: s, m, h, d)"
            ))
        }
    };
    if interval_ms <= 0 {
        return Err(format!("invalid timeframe '{s0}' (interval must be > 0)"));
    }

    Ok(TimeframeSpec {
        label: format!("{n}{unit}"),
        interval_ms,
    })
}

pub fn resample_candles(candles: &Candles, interval_ms: i64) -> Result<Candles, String> {
    if interval_ms <= 0 {
        return Err("interval_ms must be > 0".to_string());
    }
    if candles.timestamp.is_empty() {
        return Err("candles is empty".to_string());
    }
    let n = candles.timestamp.len();
    if candles.open.len() != n
        || candles.high.len() != n
        || candles.low.len() != n
        || candles.close.len() != n
        || candles.volume.len() != n
    {
        return Err("candles vectors have inconsistent lengths".to_string());
    }

    // Validate monotonic timestamps (required for stable resampling).
    for i in 1..n {
        if candles.timestamp[i] < candles.timestamp[i - 1] {
            return Err(
                "timestamps must be sorted ascending to resample (try repairing the dataset)"
                    .to_string(),
            );
        }
    }

    let mut out_ts: Vec<i64> = Vec::new();
    let mut out_open: Vec<f64> = Vec::new();
    let mut out_high: Vec<f64> = Vec::new();
    let mut out_low: Vec<f64> = Vec::new();
    let mut out_close: Vec<f64> = Vec::new();
    let mut out_vol: Vec<f64> = Vec::new();

    let mut has_open_any = false;
    let mut has_high_any = false;
    let mut has_low_any = false;
    let mut has_vol_any = false;

    let mut cur_day = i64::MIN;
    let mut session_origin: i64 = 0;
    let mut cur_bucket_start: i64 = 0;

    let mut bucket_has_any = false;
    let mut acc_open = f64::NAN;
    let mut acc_high = f64::NAN;
    let mut acc_low = f64::NAN;
    let mut acc_close = f64::NAN;
    let mut acc_vol = f64::NAN;
    let mut saw_open = false;
    let mut saw_high = false;
    let mut saw_low = false;
    let mut saw_vol = false;

    let mut flush = |ts_bucket: i64,
                     o: f64,
                     h: f64,
                     l: f64,
                     c: f64,
                     v: f64,
                     saw_o: bool,
                     saw_h: bool,
                     saw_l: bool,
                     saw_v: bool| {
        out_ts.push(ts_bucket);
        out_open.push(o);
        out_high.push(h);
        out_low.push(l);
        out_close.push(c);
        out_vol.push(v);
        if saw_o {
            has_open_any = true;
        }
        if saw_h {
            has_high_any = true;
        }
        if saw_l {
            has_low_any = true;
        }
        if saw_v {
            has_vol_any = true;
        }
    };

    for i in 0..n {
        let ts = candles.timestamp[i];
        let day = ts.div_euclid(DAY_MS);
        if day != cur_day {
            if bucket_has_any {
                flush(
                    cur_bucket_start,
                    acc_open,
                    acc_high,
                    acc_low,
                    acc_close,
                    acc_vol,
                    saw_open,
                    saw_high,
                    saw_low,
                    saw_vol,
                );
            }
            cur_day = day;
            session_origin = ts;
            bucket_has_any = false;
        }

        let bucket_index = (ts - session_origin).div_euclid(interval_ms);
        let bucket_start = session_origin + bucket_index.saturating_mul(interval_ms);

        if !bucket_has_any || bucket_start != cur_bucket_start {
            if bucket_has_any {
                flush(
                    cur_bucket_start,
                    acc_open,
                    acc_high,
                    acc_low,
                    acc_close,
                    acc_vol,
                    saw_open,
                    saw_high,
                    saw_low,
                    saw_vol,
                );
            }

            cur_bucket_start = bucket_start;
            bucket_has_any = true;

            acc_open = f64::NAN;
            acc_high = f64::NAN;
            acc_low = f64::NAN;
            acc_vol = f64::NAN;
            saw_open = false;
            saw_high = false;
            saw_low = false;
            saw_vol = false;
        }

        // Open: first finite open in bucket (only if the source actually has open).
        if candles.fields.open && !saw_open {
            let o = candles.open[i];
            if o.is_finite() {
                acc_open = o;
                saw_open = true;
            }
        }

        // High/low: track max/min over finite values (only if present).
        if candles.fields.high {
            let h = candles.high[i];
            if h.is_finite() {
                acc_high = if saw_high { acc_high.max(h) } else { h };
                saw_high = true;
            }
        }
        if candles.fields.low {
            let l = candles.low[i];
            if l.is_finite() {
                acc_low = if saw_low { acc_low.min(l) } else { l };
                saw_low = true;
            }
        }

        // Close: last close in bucket (required).
        let c = candles.close[i];
        if !c.is_finite() {
            return Err(format!("close is not finite at index {i}"));
        }
        acc_close = c;

        // Volume: sum over finite values (only if present).
        if candles.fields.volume {
            let v = candles.volume[i];
            if v.is_finite() {
                acc_vol = if saw_vol { acc_vol + v } else { v };
                saw_vol = true;
            }
        }
    }

    if bucket_has_any {
        flush(
            cur_bucket_start,
            acc_open,
            acc_high,
            acc_low,
            acc_close,
            acc_vol,
            saw_open,
            saw_high,
            saw_low,
            saw_vol,
        );
    }

    if out_ts.is_empty() {
        return Err("resample produced no output".to_string());
    }

    let fields = CandleFieldFlags {
        open: candles.fields.open && has_open_any,
        close: true,
        high: candles.fields.high && has_high_any,
        low: candles.fields.low && has_low_any,
        volume: candles.fields.volume && has_vol_any,
    };

    // Keep output vectors aligned: for missing fields, preserve NaNs.
    // For high/low/open/volume, leave NaN where the field isn't present.
    if !fields.open {
        out_open.iter_mut().for_each(|x| *x = f64::NAN);
    }
    if !fields.high {
        out_high.iter_mut().for_each(|x| *x = f64::NAN);
    }
    if !fields.low {
        out_low.iter_mut().for_each(|x| *x = f64::NAN);
    }
    if !fields.volume {
        out_vol.iter_mut().for_each(|x| *x = f64::NAN);
    }

    Ok(Candles::new_with_fields(
        out_ts, out_open, out_high, out_low, out_close, out_vol, fields,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timeframe_label_accepts_seconds_and_canonicalizes_common() {
        assert_eq!(
            parse_timeframe_label("120s").unwrap(),
            TimeframeSpec {
                label: "2m".to_string(),
                interval_ms: 120_000
            }
        );
        assert_eq!(
            parse_timeframe_label("7s").unwrap(),
            TimeframeSpec {
                label: "7s".to_string(),
                interval_ms: 7_000
            }
        );
    }

    #[test]
    fn resample_rejects_unsorted_timestamps() {
        let candles = Candles::new(
            vec![120_000, 60_000],
            vec![10.0, 11.0],
            vec![11.0, 12.0],
            vec![9.0, 10.0],
            vec![10.5, 11.5],
            vec![1.0, 2.0],
        );

        let error =
            resample_candles(&candles, 60_000).expect_err("unsorted timestamps should fail");
        assert!(error.contains("sorted ascending"));
    }

    #[test]
    fn resample_anchors_to_first_bar_of_day() {
        // Two "days" of minute bars. Day 0 starts at 09:30 (in ms from epoch day 0).
        let day0_start = 9 * 3_600_000 + 30 * 60_000; // 09:30
        let day1_start = day0_start + DAY_MS;

        let mut ts = Vec::new();
        let mut o = Vec::new();
        let mut h = Vec::new();
        let mut l = Vec::new();
        let mut c = Vec::new();
        let mut v = Vec::new();

        // Day 0: 5 minutes (09:30..09:34)
        for i in 0..5 {
            ts.push(day0_start + (i as i64) * 60_000);
            let px = 100.0 + i as f64;
            o.push(px);
            h.push(px + 1.0);
            l.push(px - 1.0);
            c.push(px + 0.5);
            v.push(10.0);
        }
        // Day 1: 3 minutes (09:30..09:32)
        for i in 0..3 {
            ts.push(day1_start + (i as i64) * 60_000);
            let px = 200.0 + i as f64;
            o.push(px);
            h.push(px + 1.0);
            l.push(px - 1.0);
            c.push(px + 0.5);
            v.push(20.0);
        }

        let candles = Candles::new(ts, o, h, l, c, v);

        // 2h buckets within each day should start at 09:30 (not midnight, not 08:00).
        let out = resample_candles(&candles, 2 * 3_600_000).unwrap();
        assert_eq!(out.timestamp[0], day0_start);
        assert_eq!(out.timestamp[1], day1_start);
        assert!(out.close[0].is_finite());
        assert!(out.volume[0].is_finite());
        assert_eq!(out.volume[0], 5.0 * 10.0);
        assert_eq!(out.volume[1], 3.0 * 20.0);
    }

    #[test]
    fn resample_ohlc_volume_for_2m() {
        let day0_start = 9 * 3_600_000 + 30 * 60_000; // 09:30
        let mut ts = Vec::new();
        let mut o = Vec::new();
        let mut h = Vec::new();
        let mut l = Vec::new();
        let mut c = Vec::new();
        let mut v = Vec::new();

        // 5 minute bars
        for i in 0..5 {
            ts.push(day0_start + (i as i64) * 60_000);
            o.push(10.0 + i as f64);
            h.push(11.0 + i as f64);
            l.push(9.0 + i as f64);
            c.push(10.5 + i as f64);
            v.push(1.0 + i as f64);
        }
        let candles = Candles::new(ts, o, h, l, c, v);
        let out = resample_candles(&candles, 120_000).unwrap();

        // Buckets: [0..1], [2..3], [4]
        assert_eq!(out.timestamp.len(), 3);
        assert_eq!(out.timestamp[0], day0_start);
        assert_eq!(out.timestamp[1], day0_start + 2 * 60_000);
        assert_eq!(out.timestamp[2], day0_start + 4 * 60_000);

        // First bucket: open from 0, close from 1, high=max(11,12)=12, low=min(9,10)=9, vol=1+2=3
        assert_eq!(out.open[0], 10.0);
        assert_eq!(out.close[0], 11.5);
        assert_eq!(out.high[0], 12.0);
        assert_eq!(out.low[0], 9.0);
        assert_eq!(out.volume[0], 3.0);
    }
}
