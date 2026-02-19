use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use std::path::Path;

use serde::Serialize;
use vector_ta::utilities::data_loader::Candles;

use crate::data_cache;

#[derive(Debug, Clone, Serialize)]
pub struct PriceDataMeta {
    pub len: usize,
    pub has_open: bool,
    pub has_high: bool,
    pub has_low: bool,
    pub has_close: bool,
    pub has_volume: bool,
    pub min_ts: i64,
    pub max_ts: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mtime_utc_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadPriceDataResponse {
    pub id: String,
    pub meta: PriceDataMeta,
    pub cache_hit: bool,
    pub cache_path: String,
}

pub struct AppState {
    pub candles: Mutex<HashMap<String, Candles>>,
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    live_candle_stream: Mutex<Option<LiveCandleStreamTask>>,
}

pub struct LiveCandleStreamTask {
    pub handle: tauri::async_runtime::JoinHandle<()>,
    pub market: String,
    pub symbol: String,
    pub delayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveCandleStreamInfo {
    pub market: String,
    pub symbol: String,
    pub delayed: bool,
    pub active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            candles: Mutex::new(HashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            live_candle_stream: Mutex::new(None),
        }
    }
}

pub struct RunGuard {
    running: Arc<AtomicBool>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl AppState {
    pub fn load_price_data(
        &self,
        path: &str,
        app_cache_dir: &Path,
    ) -> Result<LoadPriceDataResponse, String> {
        let pathbuf = Path::new(path);
        let outcome = data_cache::load_or_build(pathbuf, app_cache_dir)?;
        let candles = outcome.candles;
        let (min_ts, max_ts) = min_max_ts(&candles.timestamp);
        let timeframe = infer_timeframe_label(&candles.timestamp);
        let meta = PriceDataMeta {
            len: candles.close.len(),
            has_open: candles.fields.open,
            has_high: candles.fields.high,
            has_low: candles.fields.low,
            has_close: candles.fields.close,
            has_volume: candles.fields.volume,
            min_ts,
            max_ts,
            timeframe,
            source_size_bytes: Some(outcome.source_identity.size),
            source_mtime_utc_ms: Some(outcome.source_identity.mtime_utc_ms),
        };
        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        let id = format!("data-{}", map.len() + 1);
        map.insert(id.clone(), candles);
        Ok(LoadPriceDataResponse {
            id,
            meta,
            cache_hit: outcome.cache_hit,
            cache_path: outcome
                .cache_path
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        })
    }

    pub fn resample_loaded_data(
        &self,
        base_data_id: &str,
        timeframe: &str,
    ) -> Result<LoadPriceDataResponse, String> {
        let tf = ta_app_core::parse_timeframe_label(timeframe)?;
        let out_id = format!("{base_data_id}@{}", tf.label);

        // Fast path: reuse an existing derived dataset if present.
        {
            let map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
            if let Some(c) = map.get(&out_id) {
                let (min_ts, max_ts) = min_max_ts(&c.timestamp);
                let meta = PriceDataMeta {
                    len: c.close.len(),
                    has_open: c.fields.open,
                    has_high: c.fields.high,
                    has_low: c.fields.low,
                    has_close: c.fields.close,
                    has_volume: c.fields.volume,
                    min_ts,
                    max_ts,
                    timeframe: Some(tf.label),
                    source_size_bytes: None,
                    source_mtime_utc_ms: None,
                };
                return Ok(LoadPriceDataResponse {
                    id: out_id,
                    meta,
                    cache_hit: true,
                    cache_path: String::new(),
                });
            }
        }

        let base = self.get_candles(base_data_id)?;
        let derived = ta_app_core::resample_candles(&base, tf.interval_ms)?;
        let (min_ts, max_ts) = min_max_ts(&derived.timestamp);
        let meta = PriceDataMeta {
            len: derived.close.len(),
            has_open: derived.fields.open,
            has_high: derived.fields.high,
            has_low: derived.fields.low,
            has_close: derived.fields.close,
            has_volume: derived.fields.volume,
            min_ts,
            max_ts,
            timeframe: Some(tf.label),
            source_size_bytes: None,
            source_mtime_utc_ms: None,
        };

        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        map.insert(out_id.clone(), derived);

        Ok(LoadPriceDataResponse {
            id: out_id,
            meta,
            cache_hit: false,
            cache_path: String::new(),
        })
    }

    pub fn get_candles(&self, id: &str) -> Result<Candles, String> {
        let map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        map.get(id)
            .cloned()
            .ok_or_else(|| format!("unknown data id: {id}"))
    }

    pub fn upsert_live_bar(
        &self,
        data_id: &str,
        time_open_unix_ms: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
    ) -> Result<(), String> {
        if !open.is_finite()
            || !high.is_finite()
            || !low.is_finite()
            || !close.is_finite()
            || high < low
        {
            return Err("invalid live bar values".to_string());
        }
        if time_open_unix_ms <= 0 {
            return Err("invalid live bar timestamp".to_string());
        }

        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        let candles = map
            .get_mut(data_id)
            .ok_or_else(|| format!("unknown data id: {data_id}"))?;
        if candles.timestamp.is_empty() || candles.close.is_empty() {
            return Err("cannot upsert live bar into an empty dataset".to_string());
        }
        if !candles.fields.close {
            return Err("dataset has no close field".to_string());
        }

        let use_ms = candles
            .timestamp
            .last()
            .copied()
            .map(timestamp_uses_millis)
            .unwrap_or(true);
        let ts = if use_ms {
            time_open_unix_ms
        } else {
            time_open_unix_ms.div_euclid(1000)
        };
        if ts <= 0 {
            return Err("live bar timestamp normalization failed".to_string());
        }

        let last_ts = candles.timestamp.last().copied().unwrap_or(ts);
        if ts > last_ts {
            candles.timestamp.push(ts);
            if candles.fields.open {
                candles.open.push(open);
            }
            if candles.fields.high {
                candles.high.push(high);
            }
            if candles.fields.low {
                candles.low.push(low);
            }
            candles.close.push(close);
            if candles.fields.volume {
                candles
                    .volume
                    .push(volume.filter(|v| v.is_finite()).unwrap_or(0.0));
            }
            return Ok(());
        }

        let idx = if ts == last_ts {
            candles.timestamp.len().saturating_sub(1)
        } else if let Some(i) = candles.timestamp.iter().rposition(|&x| x == ts) {
            i
        } else {
            // Stale/out-of-order bar not present in the current dataset.
            return Ok(());
        };

        if candles.fields.open && idx < candles.open.len() {
            candles.open[idx] = open;
        }
        if candles.fields.high && idx < candles.high.len() {
            candles.high[idx] = high;
        }
        if candles.fields.low && idx < candles.low.len() {
            candles.low[idx] = low;
        }
        if idx < candles.close.len() {
            candles.close[idx] = close;
        }
        if candles.fields.volume && idx < candles.volume.len() {
            candles.volume[idx] = volume.filter(|v| v.is_finite()).unwrap_or(0.0);
        }
        Ok(())
    }

    pub fn try_begin_run(&self) -> Result<RunGuard, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("an optimization run is already in progress".to_string());
        }
        self.cancel.store(false, Ordering::SeqCst);
        Ok(RunGuard {
            running: self.running.clone(),
        })
    }

    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn replace_live_candle_stream(
        &self,
        task: LiveCandleStreamTask,
    ) -> Result<Option<LiveCandleStreamInfo>, String> {
        let mut slot = self
            .live_candle_stream
            .lock()
            .map_err(|_| "failed to lock live stream state".to_string())?;
        let prior = slot.take().map(|prev| {
            let info = LiveCandleStreamInfo {
                market: prev.market,
                symbol: prev.symbol,
                delayed: prev.delayed,
                active: !prev.handle.inner().is_finished(),
            };
            prev.handle.abort();
            info
        });
        *slot = Some(task);
        Ok(prior)
    }

    pub fn stop_live_candle_stream(&self) -> Result<Option<LiveCandleStreamInfo>, String> {
        let mut slot = self
            .live_candle_stream
            .lock()
            .map_err(|_| "failed to lock live stream state".to_string())?;
        let prior = slot.take().map(|prev| {
            let info = LiveCandleStreamInfo {
                market: prev.market,
                symbol: prev.symbol,
                delayed: prev.delayed,
                active: !prev.handle.inner().is_finished(),
            };
            prev.handle.abort();
            info
        });
        Ok(prior)
    }

    pub fn live_candle_stream_info(&self) -> Result<Option<LiveCandleStreamInfo>, String> {
        let mut slot = self
            .live_candle_stream
            .lock()
            .map_err(|_| "failed to lock live stream state".to_string())?;
        if slot
            .as_ref()
            .is_some_and(|s| s.handle.inner().is_finished())
        {
            slot.take();
            return Ok(None);
        }
        Ok(slot.as_ref().map(|s| LiveCandleStreamInfo {
            market: s.market.clone(),
            symbol: s.symbol.clone(),
            delayed: s.delayed,
            active: true,
        }))
    }
}

pub struct BackgroundTaskState {
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

pub struct BackgroundTaskGuard {
    running: Arc<AtomicBool>,
}

impl Drop for BackgroundTaskGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Default for BackgroundTaskState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl BackgroundTaskState {
    pub fn try_begin(&self) -> Result<BackgroundTaskGuard, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("a background task is already in progress".to_string());
        }
        self.cancel.store(false, Ordering::SeqCst);
        Ok(BackgroundTaskGuard {
            running: self.running.clone(),
        })
    }

    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

fn min_max_ts(ts: &[i64]) -> (i64, i64) {
    if ts.is_empty() {
        return (0, 0);
    }
    let mut min = i64::MAX;
    let mut max = i64::MIN;
    for &t in ts {
        if t < min {
            min = t;
        }
        if t > max {
            max = t;
        }
    }
    (min, max)
}

fn timestamp_uses_millis(ts: i64) -> bool {
    ts.abs() >= 1_000_000_000_000
}

fn infer_timeframe_label(ts: &[i64]) -> Option<String> {
    if ts.len() < 3 {
        return None;
    }

    let sample = (ts.len() - 1).min(1024);
    let mut deltas: Vec<i64> = Vec::with_capacity(sample);
    for i in 1..=sample {
        let d = ts[i].saturating_sub(ts[i - 1]).abs();
        if d > 0 {
            deltas.push(d);
        }
    }
    if deltas.is_empty() {
        return None;
    }
    deltas.sort_unstable();
    let median = deltas[deltas.len() / 2];

    // Heuristic: Polygon data uses ms epoch, many CSVs use seconds. Treat "big" deltas as ms.
    let sec = if median >= 10_000 {
        (median as f64 / 1000.0).round() as i64
    } else {
        median
    };
    if sec <= 0 {
        return None;
    }

    // Match common timeframes within a small tolerance.
    let tol = (sec / 50).max(1); // ~2% tolerance, min 1s.
    let match_secs = |target: i64| (sec - target).abs() <= tol;

    let label = if match_secs(60) {
        "1m"
    } else if match_secs(120) {
        "2m"
    } else if match_secs(180) {
        "3m"
    } else if match_secs(300) {
        "5m"
    } else if match_secs(900) {
        "15m"
    } else if match_secs(1800) {
        "30m"
    } else if match_secs(3600) {
        "1h"
    } else if match_secs(7200) {
        "2h"
    } else if match_secs(14_400) {
        "4h"
    } else if match_secs(86_400) {
        "1d"
    } else {
        return Some(format!("{sec}s"));
    };
    Some(label.to_string())
}
