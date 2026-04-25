use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use std::path::Path;

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
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
#[serde(rename_all = "snake_case")]
pub enum LoadSource {
    BinaryCache,
    CsvParsedAndCached,
    CsvParsedCacheWriteSkipped,
    InMemoryBaseReuse,
    DerivedFreshResample,
    DerivedInMemoryReuse,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadPriceDataResponse {
    pub id: String,
    pub meta: PriceDataMeta,
    pub cache_hit: bool,
    pub cache_path: String,
    pub load_source: LoadSource,
}

#[derive(Debug, Clone)]
pub struct LoadedDataContext {
    pub csv_path: String,
    pub label: String,
    pub timeframe: Option<String>,
    pub source_identity: Option<data_cache::SourceIdentity>,
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

fn max_active_base_datasets() -> usize {
    env_usize("VECTORBT_MAX_ACTIVE_BASE_DATASETS", 1)
}

fn max_active_derived_datasets() -> usize {
    env_usize("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS", 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatasetRetentionKind {
    Base,
    Derived,
}

impl DatasetRetentionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Derived => "derived",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DatasetKind {
    Base,
    Derived {
        parent_data_id: String,
        timeframe: String,
    },
}

impl DatasetKind {
    fn retention_kind(&self) -> DatasetRetentionKind {
        match self {
            Self::Base => DatasetRetentionKind::Base,
            Self::Derived { .. } => DatasetRetentionKind::Derived,
        }
    }

    fn parent_data_id(&self) -> Option<&str> {
        match self {
            Self::Base => None,
            Self::Derived { parent_data_id, .. } => Some(parent_data_id.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatasetVersionMeta {
    version: u64,
    kind: DatasetKind,
    source_version: Option<u64>,
}

#[derive(Debug, Clone)]
struct DatasetEntry {
    candles: Arc<Candles>,
    version_meta: DatasetVersionMeta,
    last_access_seq: u64,
}

pub struct AppState {
    candles: Mutex<HashMap<String, DatasetEntry>>,
    loaded_data_context: Mutex<HashMap<String, LoadedDataContext>>,
    next_dataset_id: AtomicU64,
    access_seq: AtomicU64,
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    live_candle_stream: Mutex<Option<LiveCandleStreamTask>>,
    live_candle_stream_status: Mutex<Option<LiveCandleStreamInfo>>,
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
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status_at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accepted_bar_open_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accepted_bar_close_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accepted_bar_received_at_unix_ms: Option<i64>,
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn live_stream_info(
    market: String,
    symbol: String,
    delayed: bool,
    active: bool,
    status: impl Into<String>,
) -> LiveCandleStreamInfo {
    LiveCandleStreamInfo {
        market,
        symbol,
        delayed,
        active,
        status: status.into(),
        last_message: None,
        last_status_at_unix_ms: None,
        last_accepted_bar_open_unix_ms: None,
        last_accepted_bar_close_unix_ms: None,
        last_accepted_bar_received_at_unix_ms: None,
    }
}

fn live_stream_info_with_prior_diagnostics(
    market: String,
    symbol: String,
    delayed: bool,
    active: bool,
    status: impl Into<String>,
    prior: Option<&LiveCandleStreamInfo>,
) -> LiveCandleStreamInfo {
    let mut info = live_stream_info(market, symbol, delayed, active, status);
    if let Some(prior) = prior {
        info.last_message = prior.last_message.clone();
        info.last_status_at_unix_ms = prior.last_status_at_unix_ms;
        info.last_accepted_bar_open_unix_ms = prior.last_accepted_bar_open_unix_ms;
        info.last_accepted_bar_close_unix_ms = prior.last_accepted_bar_close_unix_ms;
        info.last_accepted_bar_received_at_unix_ms = prior.last_accepted_bar_received_at_unix_ms;
    }
    info
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            candles: Mutex::new(HashMap::new()),
            loaded_data_context: Mutex::new(HashMap::new()),
            next_dataset_id: AtomicU64::new(0),
            access_seq: AtomicU64::new(0),
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            live_candle_stream: Mutex::new(None),
            live_candle_stream_status: Mutex::new(None),
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
    fn next_dataset_id(&self) -> String {
        let id = self.next_dataset_id.fetch_add(1, Ordering::SeqCst) + 1;
        format!("data-{id}")
    }

    fn next_access_seq(&self) -> u64 {
        self.access_seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn dataset_version(&self, id: &str) -> Result<u64, String> {
        let map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        map.get(id)
            .map(|entry| entry.version_meta.version)
            .ok_or_else(|| format!("unknown data id: {id}"))
    }

    pub fn invalidate_dataset(&self, id: &str) -> Result<Vec<String>, String> {
        let removed = {
            let mut map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
            if !map.contains_key(id) {
                return Err(format!("unknown data id: {id}"));
            }
            let mut removed = Vec::new();
            remove_dataset_tree_locked(&mut map, id, &mut removed);
            removed
        };
        self.remove_loaded_context_entries(&removed)?;
        Ok(removed)
    }

    pub fn load_price_data(
        &self,
        path: &str,
        app_cache_dir: &Path,
    ) -> Result<LoadPriceDataResponse, String> {
        let pathbuf = Path::new(path);
        let source_identity = data_cache::identity_for_source(pathbuf)?;

        if let Some(existing) = self.find_matching_loaded_base(path, source_identity)? {
            return Ok(existing);
        }

        let outcome =
            data_cache::load_or_build_with_identity(pathbuf, app_cache_dir, source_identity)?;
        let mut candles = outcome.candles;
        normalize_timestamps_to_millis(&mut candles.timestamp)?;
        #[cfg(test)]
        run_load_before_insert_hook(self);
        if let Some(existing) = self.find_matching_loaded_base(path, outcome.source_identity)? {
            return Ok(existing);
        }
        let meta = build_price_data_meta(
            &candles,
            infer_timeframe_label(&candles.timestamp),
            Some(outcome.source_identity.size),
            Some(outcome.source_identity.mtime_utc_ms),
        );
        let id = self.next_dataset_id();
        let access_seq = self.next_access_seq();
        let evicted_ids = {
            let mut map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
            map.insert(
                id.clone(),
                DatasetEntry {
                    candles: Arc::new(candles),
                    version_meta: DatasetVersionMeta {
                        version: 1,
                        kind: DatasetKind::Base,
                        source_version: None,
                    },
                    last_access_seq: access_seq,
                },
            );
            let protect_ids = HashSet::from([id.clone()]);
            enforce_dataset_retention_locked(&mut map, &protect_ids)
        };
        self.remove_loaded_context_entries(&evicted_ids)?;

        let mut ctx_map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;
        ctx_map.insert(
            id.clone(),
            LoadedDataContext {
                csv_path: path.to_string(),
                label: dataset_label_from_path(path),
                timeframe: meta.timeframe.clone(),
                source_identity: Some(outcome.source_identity),
            },
        );
        let cache_path = outcome
            .cache_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let load_source = if outcome.cache_hit {
            LoadSource::BinaryCache
        } else if outcome.cache_path.is_some() {
            LoadSource::CsvParsedAndCached
        } else {
            LoadSource::CsvParsedCacheWriteSkipped
        };

        Ok(LoadPriceDataResponse {
            id,
            meta,
            cache_hit: outcome.cache_hit,
            cache_path,
            load_source,
        })
    }

    pub fn resample_loaded_data(
        &self,
        base_data_id: &str,
        timeframe: &str,
    ) -> Result<LoadPriceDataResponse, String> {
        let tf = ta_app_core::parse_timeframe_label(timeframe)?;
        let out_id = format!("{base_data_id}@{}", tf.label);
        let access_seq = self.next_access_seq();

        let (cached_meta, stale_removed_ids, base_version, next_derived_version) = {
            let mut map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
            let base_version = map
                .get(base_data_id)
                .map(|entry| entry.version_meta.version)
                .ok_or_else(|| format!("unknown data id: {base_data_id}"))?;

            if let Some(entry) = map.get(&out_id) {
                let is_fresh = match &entry.version_meta.kind {
                    DatasetKind::Derived {
                        parent_data_id,
                        timeframe,
                    } => {
                        parent_data_id == base_data_id
                            && timeframe == &tf.label
                            && entry.version_meta.source_version == Some(base_version)
                    }
                    DatasetKind::Base => false,
                };
                if is_fresh {
                    let meta = build_price_data_meta(
                        entry.candles.as_ref(),
                        Some(tf.label.clone()),
                        None,
                        None,
                    );
                    let current_version = entry.version_meta.version;
                    if let Some(entry_mut) = map.get_mut(&out_id) {
                        entry_mut.last_access_seq = access_seq;
                    }
                    (Some(meta), Vec::new(), base_version, current_version)
                } else {
                    let next_version = entry.version_meta.version.saturating_add(1);
                    let mut removed = Vec::new();
                    remove_dataset_tree_locked(&mut map, &out_id, &mut removed);
                    (None, removed, base_version, next_version)
                }
            } else {
                (None, Vec::new(), base_version, 1)
            }
        };
        self.remove_loaded_context_entries(&stale_removed_ids)?;

        if let Some(meta) = cached_meta {
            return Ok(LoadPriceDataResponse {
                id: out_id,
                meta,
                cache_hit: true,
                cache_path: String::new(),
                load_source: LoadSource::DerivedInMemoryReuse,
            });
        }

        let base = self.get_candles(base_data_id)?;
        let base_ctx = self.get_loaded_data_context(base_data_id)?;
        let derived = ta_app_core::resample_candles(base.as_ref(), tf.interval_ms)?;
        let meta = build_price_data_meta(&derived, Some(tf.label.clone()), None, None);
        #[cfg(test)]
        run_resample_before_insert_hook(self);

        enum ResampleInsertOutcome {
            Reused(PriceDataMeta),
            Inserted(Vec<String>),
        }

        let insert_outcome = {
            let mut map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
            let current_base_version = map
                .get(base_data_id)
                .map(|entry| entry.version_meta.version)
                .ok_or_else(|| format!("unknown data id: {base_data_id}"))?;
            if current_base_version != base_version {
                return Err(format!(
                    "base dataset changed during resample for {base_data_id}; retry the request"
                ));
            }

            if let Some(entry) = map.get_mut(&out_id) {
                let is_fresh = match &entry.version_meta.kind {
                    DatasetKind::Derived {
                        parent_data_id,
                        timeframe,
                    } => {
                        parent_data_id == base_data_id
                            && timeframe == &tf.label
                            && entry.version_meta.source_version == Some(base_version)
                    }
                    DatasetKind::Base => false,
                };
                if is_fresh {
                    entry.last_access_seq = access_seq;
                    let cached_meta = build_price_data_meta(
                        entry.candles.as_ref(),
                        Some(tf.label.clone()),
                        None,
                        None,
                    );
                    ResampleInsertOutcome::Reused(cached_meta)
                } else {
                    map.insert(
                        out_id.clone(),
                        DatasetEntry {
                            candles: Arc::new(derived),
                            version_meta: DatasetVersionMeta {
                                version: next_derived_version,
                                kind: DatasetKind::Derived {
                                    parent_data_id: base_data_id.to_string(),
                                    timeframe: tf.label.clone(),
                                },
                                source_version: Some(base_version),
                            },
                            last_access_seq: access_seq,
                        },
                    );
                    let protect_ids = HashSet::from([out_id.clone(), base_data_id.to_string()]);
                    ResampleInsertOutcome::Inserted(enforce_dataset_retention_locked(
                        &mut map,
                        &protect_ids,
                    ))
                }
            } else {
                map.insert(
                    out_id.clone(),
                    DatasetEntry {
                        candles: Arc::new(derived),
                        version_meta: DatasetVersionMeta {
                            version: next_derived_version,
                            kind: DatasetKind::Derived {
                                parent_data_id: base_data_id.to_string(),
                                timeframe: tf.label.clone(),
                            },
                            source_version: Some(base_version),
                        },
                        last_access_seq: access_seq,
                    },
                );
                let protect_ids = HashSet::from([out_id.clone(), base_data_id.to_string()]);
                ResampleInsertOutcome::Inserted(enforce_dataset_retention_locked(
                    &mut map,
                    &protect_ids,
                ))
            }
        };
        let load_source = match insert_outcome {
            ResampleInsertOutcome::Reused(meta) => {
                return Ok(LoadPriceDataResponse {
                    id: out_id,
                    meta,
                    cache_hit: true,
                    cache_path: String::new(),
                    load_source: LoadSource::DerivedInMemoryReuse,
                });
            }
            ResampleInsertOutcome::Inserted(evicted_ids) => {
                self.remove_loaded_context_entries(&evicted_ids)?;
                LoadSource::DerivedFreshResample
            }
        };

        let mut ctx_map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;
        ctx_map.insert(
            out_id.clone(),
            LoadedDataContext {
                csv_path: base_ctx.csv_path,
                label: base_ctx.label,
                timeframe: meta.timeframe.clone(),
                source_identity: base_ctx.source_identity,
            },
        );

        Ok(LoadPriceDataResponse {
            id: out_id,
            meta,
            cache_hit: false,
            cache_path: String::new(),
            load_source,
        })
    }

    pub fn get_candles(&self, id: &str) -> Result<Arc<Candles>, String> {
        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        let access_seq = self.next_access_seq();
        map.get_mut(id)
            .map(|entry| {
                entry.last_access_seq = access_seq;
                entry.candles.clone()
            })
            .ok_or_else(|| format!("unknown data id: {id}"))
    }

    pub fn get_loaded_data_context(&self, id: &str) -> Result<LoadedDataContext, String> {
        let map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;
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

        let access_seq = self.next_access_seq();
        let removed_ids = {
            let mut map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
            let descendant_ids = descendant_ids_locked(&map, data_id);

            let mutated = {
                let entry = map
                    .get_mut(data_id)
                    .ok_or_else(|| format!("unknown data id: {data_id}"))?;
                if entry.version_meta.kind != DatasetKind::Base {
                    return Err("live bar updates are only supported for base datasets".to_string());
                }
                let candles = Arc::make_mut(&mut entry.candles);
                if candles.timestamp.is_empty() || candles.close.is_empty() {
                    return Err("cannot upsert live bar into an empty dataset".to_string());
                }
                if !candles.fields.close {
                    return Err("dataset has no close field".to_string());
                }

                let ts = match classify_dataset_timeline(&candles.timestamp) {
                    DatasetTimelineKind::EpochMillis => time_open_unix_ms,
                    DatasetTimelineKind::EpochSeconds => time_open_unix_ms.div_euclid(1000),
                    DatasetTimelineKind::Relative => {
                        return Err(
                            "live bar updates require an epoch-based dataset timeline".to_string()
                        )
                    }
                };
                if ts <= 0 {
                    return Err("live bar timestamp normalization failed".to_string());
                }

                let last_ts = candles.timestamp.last().copied().unwrap_or(ts);
                let mutated = if ts > last_ts {
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
                    true
                } else {
                    let idx = if ts == last_ts {
                        candles.timestamp.len().saturating_sub(1)
                    } else if let Some(i) = candles.timestamp.iter().rposition(|&x| x == ts) {
                        i
                    } else {
                        usize::MAX
                    };

                    if idx == usize::MAX {
                        false
                    } else {
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
                        true
                    }
                };

                if mutated {
                    entry.version_meta.version = entry.version_meta.version.saturating_add(1);
                    entry.last_access_seq = access_seq;
                }
                mutated
            };

            if !mutated {
                Vec::new()
            } else {
                for child_id in &descendant_ids {
                    map.remove(child_id);
                }
                descendant_ids
            }
        };
        self.remove_loaded_context_entries(&removed_ids)?;
        Ok(())
    }

    pub fn try_begin_run(&self) -> Result<RunGuard, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("a run is already in progress".to_string());
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
    ) -> Result<Option<LiveCandleStreamInfo>, (String, LiveCandleStreamTask)> {
        let mut slot = match self.live_candle_stream.lock() {
            Ok(slot) => slot,
            Err(_) => return Err(("failed to lock live stream state".to_string(), task)),
        };
        let prior = slot.take().map(|prev| {
            let info = live_stream_info(
                prev.market,
                prev.symbol,
                prev.delayed,
                !prev.handle.inner().is_finished(),
                "started",
            );
            prev.handle.abort();
            info
        });
        *slot = Some(task);
        if let Ok(mut status) = self.live_candle_stream_status.lock() {
            *status = slot.as_ref().map(|active| {
                live_stream_info(
                    active.market.clone(),
                    active.symbol.clone(),
                    active.delayed,
                    true,
                    "started",
                )
            });
        }
        Ok(prior)
    }

    pub fn stop_live_candle_stream(&self) -> Result<Option<LiveCandleStreamInfo>, String> {
        let mut slot = self
            .live_candle_stream
            .lock()
            .map_err(|_| "failed to lock live stream state".to_string())?;
        let prior_status = self
            .live_candle_stream_status
            .lock()
            .ok()
            .and_then(|status| status.clone());
        let prior = slot.take().map(|prev| {
            let info = live_stream_info_with_prior_diagnostics(
                prev.market,
                prev.symbol,
                prev.delayed,
                !prev.handle.inner().is_finished(),
                "stopped",
                prior_status.as_ref(),
            );
            prev.handle.abort();
            info
        });
        if let Some(info) = prior.as_ref() {
            if let Ok(mut status) = self.live_candle_stream_status.lock() {
                let mut stopped = live_stream_info_with_prior_diagnostics(
                    info.market.clone(),
                    info.symbol.clone(),
                    info.delayed,
                    false,
                    "stopped",
                    prior_status.as_ref(),
                );
                stopped.last_message = Some("live candle stream stopped".to_string());
                stopped.last_status_at_unix_ms = Some(now_unix_ms());
                *status = Some(stopped);
            }
        }
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
            let finished = slot.take();
            if let Some(task) = finished {
                let prior_status = self
                    .live_candle_stream_status
                    .lock()
                    .ok()
                    .and_then(|status| status.clone());
                if let Ok(mut status) = self.live_candle_stream_status.lock() {
                    let mut closed = live_stream_info_with_prior_diagnostics(
                        task.market,
                        task.symbol,
                        task.delayed,
                        false,
                        "closed",
                        prior_status.as_ref(),
                    );
                    closed.last_message = Some("live candle stream closed".to_string());
                    closed.last_status_at_unix_ms = Some(now_unix_ms());
                    *status = Some(closed);
                }
            }
        }
        if let Some(task) = slot.as_ref() {
            if let Ok(status) = self.live_candle_stream_status.lock() {
                if let Some(last) = status.as_ref() {
                    if last.symbol == task.symbol && last.market == task.market {
                        let mut info = last.clone();
                        info.delayed = task.delayed;
                        info.active = true;
                        if info.status.trim().is_empty() {
                            info.status = "active".to_string();
                        }
                        return Ok(Some(info));
                    }
                }
            }
            return Ok(Some(live_stream_info(
                task.market.clone(),
                task.symbol.clone(),
                task.delayed,
                true,
                "active",
            )));
        }
        let status = self
            .live_candle_stream_status
            .lock()
            .map_err(|_| "failed to lock live stream status".to_string())?;
        Ok(status.clone())
    }

    pub fn record_live_candle_stream_status(
        &self,
        market: &str,
        symbol: &str,
        delayed: bool,
        status: &str,
        message: &str,
        at_unix_ms: i64,
    ) -> Result<(), String> {
        let normalized = status.trim().to_ascii_lowercase();
        let active = matches!(
            normalized.as_str(),
            "connecting" | "subscribed" | "started" | "active"
        );
        let mut slot = self
            .live_candle_stream_status
            .lock()
            .map_err(|_| "failed to lock live stream status".to_string())?;
        let prior = slot.as_ref().cloned();
        let mut next = live_stream_info_with_prior_diagnostics(
            market.trim().to_string(),
            symbol.trim().to_string(),
            delayed,
            active,
            normalized,
            prior.as_ref(),
        );
        let message = message.trim();
        next.last_message = if message.is_empty() {
            None
        } else {
            Some(message.to_string())
        };
        next.last_status_at_unix_ms = Some(at_unix_ms.max(0));
        *slot = Some(next);
        Ok(())
    }

    pub fn record_live_candle_stream_accepted_bar(
        &self,
        market: &str,
        symbol: &str,
        delayed: bool,
        time_open_unix_ms: i64,
        time_close_unix_ms: i64,
        received_at_unix_ms: i64,
    ) -> Result<(), String> {
        let mut slot = self
            .live_candle_stream_status
            .lock()
            .map_err(|_| "failed to lock live stream status".to_string())?;
        let prior = slot.as_ref().cloned();
        let status = prior
            .as_ref()
            .map(|info| info.status.trim().to_string())
            .filter(|status| !status.is_empty())
            .unwrap_or_else(|| "active".to_string());
        let mut next = live_stream_info_with_prior_diagnostics(
            market.trim().to_string(),
            symbol.trim().to_string(),
            delayed,
            true,
            status,
            prior.as_ref(),
        );
        next.last_accepted_bar_open_unix_ms = Some(time_open_unix_ms);
        next.last_accepted_bar_close_unix_ms = Some(time_close_unix_ms);
        next.last_accepted_bar_received_at_unix_ms = Some(received_at_unix_ms.max(0));
        *slot = Some(next);
        Ok(())
    }

    fn find_matching_loaded_base(
        &self,
        path: &str,
        source_identity: data_cache::SourceIdentity,
    ) -> Result<Option<LoadPriceDataResponse>, String> {
        let access_seq = self.next_access_seq();

        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        let ctx_map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;
        let mut selected_id = None;
        for (id, ctx) in ctx_map.iter() {
            if ctx.csv_path != path || ctx.source_identity != Some(source_identity) {
                continue;
            }
            let Some(entry) = map.get(id) else {
                continue;
            };
            if entry.version_meta.kind == DatasetKind::Base {
                selected_id = Some(id.clone());
                break;
            }
        }
        drop(ctx_map);

        let Some(id) = selected_id else {
            return Ok(None);
        };
        let Some(entry) = map.get_mut(&id) else {
            return Ok(None);
        };

        entry.last_access_seq = access_seq;
        let meta = build_price_data_meta(
            entry.candles.as_ref(),
            infer_timeframe_label(&entry.candles.timestamp),
            Some(source_identity.size),
            Some(source_identity.mtime_utc_ms),
        );

        Ok(Some(LoadPriceDataResponse {
            id,
            meta,
            cache_hit: true,
            cache_path: String::new(),
            load_source: LoadSource::InMemoryBaseReuse,
        }))
    }

    fn remove_loaded_context_entries(&self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut ctx_map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;
        for id in ids {
            ctx_map.remove(id);
        }
        Ok(())
    }
}

fn dataset_label_from_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let as_path = Path::new(trimmed);
    as_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| trimmed.to_string())
}

fn build_price_data_meta(
    candles: &Candles,
    timeframe: Option<String>,
    source_size_bytes: Option<u64>,
    source_mtime_utc_ms: Option<i64>,
) -> PriceDataMeta {
    let (min_ts, max_ts) = min_max_ts(&candles.timestamp);
    PriceDataMeta {
        len: candles.close.len(),
        has_open: candles.fields.open,
        has_high: candles.fields.high,
        has_low: candles.fields.low,
        has_close: candles.fields.close,
        has_volume: candles.fields.volume,
        min_ts,
        max_ts,
        timeframe,
        source_size_bytes,
        source_mtime_utc_ms,
    }
}

fn descendant_ids_locked(map: &HashMap<String, DatasetEntry>, root_id: &str) -> Vec<String> {
    fn visit(
        map: &HashMap<String, DatasetEntry>,
        root_id: &str,
        seen: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        let children: Vec<String> = map
            .iter()
            .filter_map(|(id, entry)| {
                (entry.version_meta.kind.parent_data_id() == Some(root_id)).then(|| id.clone())
            })
            .collect();
        for child_id in children {
            if seen.insert(child_id.clone()) {
                out.push(child_id.clone());
                visit(map, &child_id, seen, out);
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    visit(map, root_id, &mut seen, &mut out);
    out
}

fn remove_dataset_tree_locked(
    map: &mut HashMap<String, DatasetEntry>,
    root_id: &str,
    removed: &mut Vec<String>,
) {
    let mut ids = vec![root_id.to_string()];
    ids.extend(descendant_ids_locked(map, root_id));
    for id in ids {
        if map.remove(&id).is_some() {
            removed.push(id);
        }
    }
}

fn expanded_protect_ids_locked(
    map: &HashMap<String, DatasetEntry>,
    protect_ids: &HashSet<String>,
) -> HashSet<String> {
    let mut expanded = protect_ids.clone();
    let mut stack: Vec<String> = protect_ids.iter().cloned().collect();
    while let Some(id) = stack.pop() {
        let Some(entry) = map.get(&id) else {
            continue;
        };
        if let Some(parent_id) = entry.version_meta.kind.parent_data_id() {
            let parent_id = parent_id.to_string();
            if expanded.insert(parent_id.clone()) {
                stack.push(parent_id);
            }
        }
    }
    expanded
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatasetRetentionCandidate {
    id: String,
    arc_shared: bool,
    strong_count: usize,
    last_access_seq: u64,
    descendant_count: usize,
    subtree_size: usize,
    heuristic_rebuild_cost_nodes: usize,
    sharing_state: &'static str,
    reuse_outcome_hint: &'static str,
}

fn retention_candidates_locked(
    map: &HashMap<String, DatasetEntry>,
    kind: DatasetRetentionKind,
    protect_ids: &HashSet<String>,
) -> Vec<DatasetRetentionCandidate> {
    map.iter()
        .filter_map(|(id, entry)| {
            (entry.version_meta.kind.retention_kind() == kind && !protect_ids.contains(id)).then(
                || {
                    let strong_count = Arc::strong_count(&entry.candles);
                    let arc_shared = strong_count > 1;
                    let descendant_count = descendant_ids_locked(map, id).len();
                    let subtree_size = descendant_count.saturating_add(1);
                    let sharing_state = if arc_shared {
                        "arc_shared"
                    } else {
                        "exclusive"
                    };
                    let reuse_outcome_hint = if descendant_count > 0 {
                        "evicts_parent_tree"
                    } else if arc_shared {
                        "evicts_shared_leaf"
                    } else {
                        "evicts_exclusive_leaf"
                    };
                    DatasetRetentionCandidate {
                        id: id.clone(),
                        arc_shared,
                        strong_count,
                        last_access_seq: entry.last_access_seq,
                        descendant_count,
                        subtree_size,
                        heuristic_rebuild_cost_nodes: subtree_size,
                        sharing_state,
                        reuse_outcome_hint,
                    }
                },
            )
        })
        .collect()
}

fn eviction_candidate_locked(
    map: &HashMap<String, DatasetEntry>,
    kind: DatasetRetentionKind,
    protect_ids: &HashSet<String>,
) -> Option<String> {
    let mut candidates = retention_candidates_locked(map, kind, protect_ids);
    candidates.sort_by(|a, b| {
        a.arc_shared
            .cmp(&b.arc_shared)
            .then_with(|| a.last_access_seq.cmp(&b.last_access_seq))
            .then_with(|| a.id.cmp(&b.id))
    });
    let chosen = candidates.into_iter().next();
    if let Some(candidate) = chosen.as_ref() {
        log::debug!(
            "dataset retention heuristic selected candidate kind={} id={} subtree_size={} rebuild_cost_nodes={} sharing_state={} reuse_outcome_hint={} strong_count={} last_access_seq={}",
            kind.as_str(),
            candidate.id,
            candidate.subtree_size,
            candidate.heuristic_rebuild_cost_nodes,
            candidate.sharing_state,
            candidate.reuse_outcome_hint,
            candidate.strong_count,
            candidate.last_access_seq
        );
    }
    chosen.map(|candidate| candidate.id)
}

fn enforce_dataset_retention_locked(
    map: &mut HashMap<String, DatasetEntry>,
    protect_ids: &HashSet<String>,
) -> Vec<String> {
    let protect_ids = expanded_protect_ids_locked(map, protect_ids);
    let mut removed = Vec::new();

    loop {
        let base_count = map
            .values()
            .filter(|entry| entry.version_meta.kind.retention_kind() == DatasetRetentionKind::Base)
            .count();
        if base_count <= max_active_base_datasets() {
            break;
        }
        let Some(id) = eviction_candidate_locked(map, DatasetRetentionKind::Base, &protect_ids)
        else {
            break;
        };
        remove_dataset_tree_locked(map, &id, &mut removed);
    }

    loop {
        let derived_count = map
            .values()
            .filter(|entry| {
                entry.version_meta.kind.retention_kind() == DatasetRetentionKind::Derived
            })
            .count();
        if derived_count <= max_active_derived_datasets() {
            break;
        }
        let Some(id) = eviction_candidate_locked(map, DatasetRetentionKind::Derived, &protect_ids)
        else {
            break;
        };
        remove_dataset_tree_locked(map, &id, &mut removed);
    }

    removed
}

#[cfg(test)]
mod state_residency_tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::{Mutex, OnceLock};

    fn residency_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn make_trending_candles(len: usize, step: f64) -> Candles {
        make_trending_candles_with_base_ts(len, step, 1_700_000_000_000)
    }

    fn make_trending_candles_with_base_ts(len: usize, step: f64, base_ts: i64) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        for i in 0..len {
            ts.push(base_ts + (i as i64) * 60_000);
            let base = 100.0 + (i as f64) * step;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn write_candles_csv(path: &Path, candles: &Candles) {
        let mut out = String::new();
        out.push_str("timestamp,open,high,low,close,volume\n");
        for i in 0..candles.close.len() {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                candles.timestamp[i],
                candles.open[i],
                candles.high[i],
                candles.low[i],
                candles.close[i],
                candles.volume[i],
            ));
        }
        std::fs::write(path, out).expect("write candles csv");
    }

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

    fn write_relative_seconds_csv(path: &Path, len: usize) {
        let mut out = String::from("timestamp,open,high,low,close,volume\n");
        for i in 0..len {
            let base = 100.0 + (i as f64) * 0.01;
            out.push_str(&format!(
                "{i},{base},{},{},{},1000\n",
                base + 0.1,
                base - 0.1,
                base + 0.05
            ));
        }
        std::fs::write(path, out).expect("write relative seconds csv");
    }

    #[test]
    fn load_normalizes_second_based_csv_timestamps_to_millis() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_load_normalize_seconds");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("relative_seconds.csv");
        write_relative_seconds_csv(&csv, 1_800);

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load relative seconds csv");
        let candles = state.get_candles(&load.id).expect("loaded candles");

        assert_eq!(candles.timestamp[0], 0);
        assert_eq!(candles.timestamp[1], 1_000);
        assert_eq!(candles.timestamp[2], 2_000);
        assert_eq!(load.meta.timeframe.as_deref(), Some("1s"));
        assert_eq!(load.meta.max_ts, 1_799_000);

        let resampled = state
            .resample_loaded_data(&load.id, "15m")
            .expect("resample 15m");
        assert_eq!(resampled.meta.len, 2);
    }

    #[test]
    fn load_replaces_prior_dataset_and_discards_prior_derived_cache() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_load_replace");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv_a = temp.join("a.csv");
        let csv_b = temp.join("b.csv");
        write_candles_csv(&csv_a, &make_trending_candles(32, 0.2));
        write_candles_csv(&csv_b, &make_trending_candles(48, 0.4));

        let state = AppState::default();
        let load_a = state
            .load_price_data(csv_a.to_str().unwrap_or_default(), &cache_dir)
            .expect("load a");
        let derived_a = state
            .resample_loaded_data(&load_a.id, "5m")
            .expect("resample a");

        assert!(state.get_candles(&load_a.id).is_ok());
        assert!(state.get_candles(&derived_a.id).is_ok());

        let load_b = state
            .load_price_data(csv_b.to_str().unwrap_or_default(), &cache_dir)
            .expect("load b");

        assert!(state.get_candles(&load_a.id).is_err());
        assert!(state.get_candles(&derived_a.id).is_err());
        assert!(state.get_candles(&load_b.id).is_ok());
    }

    #[test]
    fn base_replacement_builds_a_new_derived_identity_for_the_new_base() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_load_replace_new_derived");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv_a = temp.join("a.csv");
        let csv_b = temp.join("b.csv");
        write_candles_csv(&csv_a, &make_trending_candles(32, 0.2));
        write_candles_csv(&csv_b, &make_trending_candles(48, 0.4));

        let state = AppState::default();
        let load_a = state
            .load_price_data(csv_a.to_str().unwrap_or_default(), &cache_dir)
            .expect("load a");
        let derived_a = state
            .resample_loaded_data(&load_a.id, "5m")
            .expect("resample a");

        let load_b = state
            .load_price_data(csv_b.to_str().unwrap_or_default(), &cache_dir)
            .expect("load b");
        let derived_b = state
            .resample_loaded_data(&load_b.id, "5m")
            .expect("resample b");

        assert_ne!(load_a.id, load_b.id);
        assert_ne!(derived_a.id, derived_b.id);
        assert!(state.get_candles(&derived_a.id).is_err());
        assert!(state.get_candles(&derived_b.id).is_ok());
        assert!(matches!(
            derived_b.load_source,
            LoadSource::DerivedFreshResample
        ));
    }

    #[test]
    fn base_live_update_replaces_arc_and_invalidates_active_derived_dataset() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_live_update");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        let candles = make_trending_candles(24, 0.3);
        write_candles_csv(&csv, &candles);

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load base");
        let derived = state
            .resample_loaded_data(&load.id, "5m")
            .expect("resample base");

        let before = state.get_candles(&load.id).expect("load base candles");
        let next_ts = before.timestamp.last().copied().unwrap_or(0) + 60_000;
        state
            .upsert_live_bar(&load.id, next_ts, 120.0, 121.0, 119.0, 120.5, Some(2_000.0))
            .expect("upsert live bar");

        let after = state.get_candles(&load.id).expect("updated base candles");
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!(after.close.len(), before.close.len() + 1);
        assert!(state.get_candles(&derived.id).is_err());
    }

    #[test]
    fn resample_reuses_active_derived_dataset_until_base_changes() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_resample_reuse");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_trending_candles(40, 0.25));

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load base");

        let first = state
            .resample_loaded_data(&load.id, "5m")
            .expect("first resample");
        let first_arc = state.get_candles(&first.id).expect("first derived");
        let second = state
            .resample_loaded_data(&load.id, "5m")
            .expect("second resample");
        let second_arc = state.get_candles(&second.id).expect("second derived");

        assert_eq!(first.id, second.id);
        assert!(Arc::ptr_eq(&first_arc, &second_arc));
        assert!(matches!(
            first.load_source,
            LoadSource::DerivedFreshResample
        ));
        assert!(first.cache_path.is_empty());
        assert!(matches!(
            second.load_source,
            LoadSource::DerivedInMemoryReuse
        ));
        assert!(second.cache_path.is_empty());
    }

    #[test]
    fn identical_base_load_reuses_existing_id_and_preserves_derived_cache() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_base_reuse");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_trending_candles(36, 0.2));

        let state = AppState::default();
        let first = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("first load");
        let first_arc = state.get_candles(&first.id).expect("first base");
        let derived = state
            .resample_loaded_data(&first.id, "5m")
            .expect("derived load");

        let second = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("second load");
        let second_arc = state.get_candles(&second.id).expect("second base");
        let loaded = state
            .get_loaded_data_context(&second.id)
            .expect("loaded context");

        assert_eq!(first.id, second.id);
        assert!(second.cache_hit);
        assert!(matches!(second.load_source, LoadSource::InMemoryBaseReuse));
        assert!(second.cache_path.is_empty());
        assert!(Arc::ptr_eq(&first_arc, &second_arc));
        assert!(state.get_candles(&derived.id).is_ok());
        assert_eq!(
            loaded.source_identity,
            Some(data_cache::identity_for_source(&csv).expect("source identity"))
        );
    }

    #[test]
    fn matching_base_load_updates_access_sequence() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_access_seq_reuse");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_trending_candles(28, 0.15));

        let state = AppState::default();
        let first = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("first load");

        let before_seq = {
            let map = state.candles.lock().expect("lock candles");
            map.get(&first.id)
                .map(|entry| entry.last_access_seq)
                .expect("base entry")
        };

        let second = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("second load");

        let after_seq = {
            let map = state.candles.lock().expect("lock candles");
            map.get(&first.id)
                .map(|entry| entry.last_access_seq)
                .expect("base entry")
        };

        assert_eq!(first.id, second.id);
        assert!(after_seq > before_seq);
    }

    #[test]
    fn live_upsert_uses_epoch_millis_for_pre_2001_datasets() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_pre_2001_epoch_millis");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(
            &csv,
            &make_trending_candles_with_base_ts(24, 0.2, 631_152_000_000),
        );

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load base");
        let before = state.get_candles(&load.id).expect("base candles");
        let next_ts = before.timestamp.last().copied().unwrap_or(0) + 60_000;

        state
            .upsert_live_bar(&load.id, next_ts, 120.0, 121.0, 119.0, 120.5, Some(2_000.0))
            .expect("upsert live bar");

        let after = state.get_candles(&load.id).expect("updated base candles");
        assert_eq!(after.timestamp.last().copied(), Some(next_ts));
    }

    #[test]
    fn live_upsert_rejects_relative_timeline_datasets() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_reject_relative_live_upsert");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("relative_seconds.csv");
        write_relative_seconds_csv(&csv, 1_800);

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load relative seconds csv");

        let err = state
            .upsert_live_bar(
                &load.id,
                1_700_000_000_000,
                120.0,
                121.0,
                119.0,
                120.5,
                Some(2_000.0),
            )
            .expect_err("relative timelines should be rejected");
        assert!(err.contains("epoch-based dataset timeline"));
    }

    #[test]
    fn live_upsert_rejects_derived_dataset_ids() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_reject_derived_live_upsert");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_trending_candles(24, 0.2));

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load base");
        let derived = state
            .resample_loaded_data(&load.id, "5m")
            .expect("derive 5m");

        let err = state
            .upsert_live_bar(
                &derived.id,
                1_700_000_000_000,
                120.0,
                121.0,
                119.0,
                120.5,
                Some(2_000.0),
            )
            .expect_err("derived datasets should reject live upserts");
        assert!(err.contains("only supported for base datasets"));
    }

    #[test]
    fn load_second_check_reuses_base_inserted_during_inflight_load() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_load_second_check");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_trending_candles(48, 0.25));

        let state = AppState::default();
        let csv_path = csv.to_string_lossy().to_string();
        let cache_dir_clone = cache_dir.clone();
        install_load_before_insert_hook(Box::new(move |state| {
            state
                .load_price_data(&csv_path, &cache_dir_clone)
                .expect("hooked concurrent load");
        }));

        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("outer load should reuse hooked insert");

        let map = state.candles.lock().expect("lock candles");
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&load.id));
        assert!(matches!(load.load_source, LoadSource::InMemoryBaseReuse));
    }

    #[test]
    fn resample_recheck_rejects_parent_version_changes_before_insert() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let temp = make_temp_dir("state_resample_parent_recheck");
        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("base.csv");
        write_candles_csv(&csv, &make_trending_candles(36, 0.2));

        let state = AppState::default();
        let load = state
            .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
            .expect("load base");
        let base_arc = state.get_candles(&load.id).expect("base candles");
        let next_ts = base_arc.timestamp.last().copied().unwrap_or_default() + 60_000;
        let base_id = load.id.clone();
        install_resample_before_insert_hook(Box::new(move |state| {
            state
                .upsert_live_bar(&base_id, next_ts, 200.0, 201.0, 199.0, 200.5, Some(3_000.0))
                .expect("hooked base update");
        }));

        let err = state
            .resample_loaded_data(&load.id, "5m")
            .expect_err("resample should reject stale parent snapshot");
        assert!(err.contains("changed during resample"));
        assert!(state.get_candles(&format!("{}@5m", load.id)).is_err());
    }

    #[test]
    fn retention_limits_can_be_overridden_by_env() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let prev_base = std::env::var("VECTORBT_MAX_ACTIVE_BASE_DATASETS").ok();
        let prev_derived = std::env::var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS").ok();

        std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", "2");
        std::env::set_var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS", "3");

        assert_eq!(max_active_base_datasets(), 2);
        assert_eq!(max_active_derived_datasets(), 3);

        if let Some(value) = prev_base {
            std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", value);
        } else {
            std::env::remove_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS");
        }

        if let Some(value) = prev_derived {
            std::env::set_var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS", value);
        } else {
            std::env::remove_var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS");
        }
    }

    #[test]
    fn retention_protects_parent_base_when_only_child_is_protected() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let prev_base = std::env::var("VECTORBT_MAX_ACTIVE_BASE_DATASETS").ok();
        let prev_derived = std::env::var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS").ok();
        std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", "1");
        std::env::set_var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS", "1");

        let shared = Arc::new(make_trending_candles(8, 0.2));
        let mut map = HashMap::new();
        map.insert(
            "base-a".to_string(),
            DatasetEntry {
                candles: shared.clone(),
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Base,
                    source_version: Some(1),
                },
                last_access_seq: 1,
            },
        );
        map.insert(
            "base-b".to_string(),
            DatasetEntry {
                candles: shared.clone(),
                version_meta: DatasetVersionMeta {
                    version: 2,
                    kind: DatasetKind::Base,
                    source_version: Some(2),
                },
                last_access_seq: 2,
            },
        );
        map.insert(
            "base-b@5m".to_string(),
            DatasetEntry {
                candles: shared,
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Derived {
                        parent_data_id: "base-b".to_string(),
                        timeframe: "5m".to_string(),
                    },
                    source_version: Some(2),
                },
                last_access_seq: 3,
            },
        );

        let protected = HashSet::from(["base-b@5m".to_string()]);
        let removed = enforce_dataset_retention_locked(&mut map, &protected);

        assert_eq!(removed, vec!["base-a".to_string()]);
        assert!(map.contains_key("base-b"));
        assert!(map.contains_key("base-b@5m"));

        if let Some(value) = prev_base {
            std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", value);
        } else {
            std::env::remove_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS");
        }
        if let Some(value) = prev_derived {
            std::env::set_var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS", value);
        } else {
            std::env::remove_var("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS");
        }
    }

    #[test]
    fn retention_prefers_evicting_unshared_entries_before_arc_shared_entries() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let prev_base = std::env::var("VECTORBT_MAX_ACTIVE_BASE_DATASETS").ok();
        std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", "1");

        let shared = Arc::new(make_trending_candles(8, 0.2));
        let shared_external = shared.clone();
        let mut map = HashMap::new();
        map.insert(
            "shared-hot".to_string(),
            DatasetEntry {
                candles: shared,
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Base,
                    source_version: Some(1),
                },
                last_access_seq: 1,
            },
        );
        map.insert(
            "cold-unshared".to_string(),
            DatasetEntry {
                candles: Arc::new(make_trending_candles(8, 0.4)),
                version_meta: DatasetVersionMeta {
                    version: 2,
                    kind: DatasetKind::Base,
                    source_version: Some(2),
                },
                last_access_seq: 2,
            },
        );

        let removed = enforce_dataset_retention_locked(&mut map, &HashSet::new());

        assert_eq!(removed, vec!["cold-unshared".to_string()]);
        assert!(map.contains_key("shared-hot"));
        assert_eq!(Arc::strong_count(&shared_external), 2);

        if let Some(value) = prev_base {
            std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", value);
        } else {
            std::env::remove_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS");
        }
    }

    #[test]
    fn retention_candidates_expose_tree_and_sharing_heuristics_observationally() {
        let shared = Arc::new(make_trending_candles(8, 0.2));
        let shared_external = shared.clone();
        let mut map = HashMap::new();
        map.insert(
            "base-parent".to_string(),
            DatasetEntry {
                candles: shared.clone(),
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Base,
                    source_version: Some(1),
                },
                last_access_seq: 3,
            },
        );
        map.insert(
            "base-parent@5m".to_string(),
            DatasetEntry {
                candles: shared.clone(),
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Derived {
                        parent_data_id: "base-parent".to_string(),
                        timeframe: "5m".to_string(),
                    },
                    source_version: Some(1),
                },
                last_access_seq: 4,
            },
        );
        map.insert(
            "base-cold".to_string(),
            DatasetEntry {
                candles: Arc::new(make_trending_candles(8, 0.4)),
                version_meta: DatasetVersionMeta {
                    version: 2,
                    kind: DatasetKind::Base,
                    source_version: Some(2),
                },
                last_access_seq: 1,
            },
        );

        let mut candidates =
            retention_candidates_locked(&map, DatasetRetentionKind::Base, &HashSet::new());
        candidates.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(Arc::strong_count(&shared_external), 4);
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0],
            DatasetRetentionCandidate {
                id: "base-cold".to_string(),
                arc_shared: false,
                strong_count: 1,
                last_access_seq: 1,
                descendant_count: 0,
                subtree_size: 1,
                heuristic_rebuild_cost_nodes: 1,
                sharing_state: "exclusive",
                reuse_outcome_hint: "evicts_exclusive_leaf",
            }
        );
        assert_eq!(
            candidates[1],
            DatasetRetentionCandidate {
                id: "base-parent".to_string(),
                arc_shared: true,
                strong_count: 4,
                last_access_seq: 3,
                descendant_count: 1,
                subtree_size: 2,
                heuristic_rebuild_cost_nodes: 2,
                sharing_state: "arc_shared",
                reuse_outcome_hint: "evicts_parent_tree",
            }
        );
    }

    #[test]
    fn retention_candidate_tie_breaks_by_id_after_share_and_access_seq() {
        let shared = Arc::new(make_trending_candles(8, 0.2));
        let mut map = HashMap::new();
        for id in ["base-b", "base-a"] {
            map.insert(
                id.to_string(),
                DatasetEntry {
                    candles: shared.clone(),
                    version_meta: DatasetVersionMeta {
                        version: 1,
                        kind: DatasetKind::Base,
                        source_version: Some(1),
                    },
                    last_access_seq: 9,
                },
            );
        }

        let chosen = eviction_candidate_locked(&map, DatasetRetentionKind::Base, &HashSet::new());
        assert_eq!(chosen.as_deref(), Some("base-a"));
    }

    #[test]
    fn retention_eviction_removes_parent_and_descendants_together() {
        let _env_guard = residency_env_lock().lock().expect("lock env");
        let prev_base = std::env::var("VECTORBT_MAX_ACTIVE_BASE_DATASETS").ok();
        std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", "1");

        let drop_tree = Arc::new(make_trending_candles(8, 0.2));
        let keep_tree = Arc::new(make_trending_candles(8, 0.4));
        let _keep_tree_external = keep_tree.clone();
        let mut map = HashMap::new();
        map.insert(
            "base-keep".to_string(),
            DatasetEntry {
                candles: keep_tree,
                version_meta: DatasetVersionMeta {
                    version: 2,
                    kind: DatasetKind::Base,
                    source_version: Some(2),
                },
                last_access_seq: 10,
            },
        );
        map.insert(
            "base-drop".to_string(),
            DatasetEntry {
                candles: drop_tree.clone(),
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Base,
                    source_version: Some(1),
                },
                last_access_seq: 1,
            },
        );
        map.insert(
            "base-drop@5m".to_string(),
            DatasetEntry {
                candles: drop_tree,
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Derived {
                        parent_data_id: "base-drop".to_string(),
                        timeframe: "5m".to_string(),
                    },
                    source_version: Some(1),
                },
                last_access_seq: 2,
            },
        );

        let removed = enforce_dataset_retention_locked(&mut map, &HashSet::new());

        assert_eq!(
            removed,
            vec!["base-drop".to_string(), "base-drop@5m".to_string()]
        );
        assert!(map.contains_key("base-keep"));
        assert!(!map.contains_key("base-drop"));
        assert!(!map.contains_key("base-drop@5m"));

        if let Some(value) = prev_base {
            std::env::set_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS", value);
        } else {
            std::env::remove_var("VECTORBT_MAX_ACTIVE_BASE_DATASETS");
        }
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

const MIN_EPOCH_SECONDS: i64 = 315_532_800;
const MIN_EPOCH_MILLIS: i64 = MIN_EPOCH_SECONDS * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatasetTimelineKind {
    EpochMillis,
    EpochSeconds,
    Relative,
}

fn max_abs_ts(ts: &[i64]) -> i64 {
    let mut max_abs = 0i64;
    for &value in ts {
        let abs = value.saturating_abs();
        if abs > max_abs {
            max_abs = abs;
        }
    }
    max_abs
}

fn positive_median_delta(ts: &[i64]) -> Option<i64> {
    if ts.len() < 2 {
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
    Some(deltas[deltas.len() / 2])
}

fn timestamp_uses_millis(ts: i64) -> bool {
    ts.abs() >= MIN_EPOCH_MILLIS
}

fn classify_dataset_timeline(ts: &[i64]) -> DatasetTimelineKind {
    let max_abs = max_abs_ts(ts);
    if timestamp_uses_millis(max_abs) {
        DatasetTimelineKind::EpochMillis
    } else if max_abs >= MIN_EPOCH_SECONDS {
        DatasetTimelineKind::EpochSeconds
    } else {
        DatasetTimelineKind::Relative
    }
}

fn timestamps_look_like_seconds(ts: &[i64]) -> bool {
    let max_abs = max_abs_ts(ts);
    if max_abs >= MIN_EPOCH_MILLIS {
        return false;
    }
    if max_abs >= MIN_EPOCH_SECONDS {
        return true;
    }
    positive_median_delta(ts).is_some_and(|median| median < 1_000)
}

fn normalize_timestamps_to_millis(ts: &mut [i64]) -> Result<(), String> {
    if !timestamps_look_like_seconds(ts) {
        return Ok(());
    }
    for value in ts.iter_mut() {
        *value = value
            .checked_mul(1_000)
            .ok_or_else(|| "timestamp normalization overflowed i64".to_string())?;
    }
    Ok(())
}

fn infer_timeframe_label(ts: &[i64]) -> Option<String> {
    if ts.len() < 3 {
        return None;
    }

    let median = positive_median_delta(ts)?;

    let sec = if timestamps_look_like_seconds(ts) {
        median
    } else {
        (median as f64 / 1000.0).round() as i64
    };
    if sec <= 0 {
        return None;
    }

    let tol = (sec / 50).max(1);
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

#[cfg(test)]
mod state_live_stream_tests {
    use super::*;
    use std::time::Duration;

    fn make_live_stream_task(symbol: &str, delayed: bool) -> LiveCandleStreamTask {
        LiveCandleStreamTask {
            handle: tauri::async_runtime::spawn(async move {
                std::thread::sleep(Duration::from_millis(25));
            }),
            market: "stocks".to_string(),
            symbol: symbol.to_string(),
            delayed,
        }
    }

    #[test]
    fn live_candle_stream_replacement_and_stop_preserve_prior_metadata() {
        let state = AppState::default();

        let first = make_live_stream_task("AAPL", true);
        let replaced = match state.replace_live_candle_stream(first) {
            Ok(replaced) => replaced,
            Err((error, _task)) => panic!("replace first live stream failed: {error}"),
        };
        assert!(replaced.is_none());

        let active = state
            .live_candle_stream_info()
            .expect("live stream info after first start")
            .expect("active stream");
        assert_eq!(active.market, "stocks");
        assert_eq!(active.symbol, "AAPL");
        assert!(active.delayed);
        assert!(active.active);
        assert_eq!(active.status, "started");

        state
            .record_live_candle_stream_status(
                "stocks",
                "AAPL",
                true,
                "subscribed",
                "subscribed to minute aggregates",
                100,
            )
            .expect("record live stream status");
        state
            .record_live_candle_stream_accepted_bar("stocks", "AAPL", true, 1_000, 1_999, 2_100)
            .expect("record accepted live bar");
        let active = state
            .live_candle_stream_info()
            .expect("live stream info after bar")
            .expect("active stream with diagnostics");
        assert_eq!(
            active.last_message.as_deref(),
            Some("subscribed to minute aggregates")
        );
        assert_eq!(active.last_status_at_unix_ms, Some(100));
        assert_eq!(active.last_accepted_bar_open_unix_ms, Some(1_000));
        assert_eq!(active.last_accepted_bar_close_unix_ms, Some(1_999));
        assert_eq!(active.last_accepted_bar_received_at_unix_ms, Some(2_100));

        let replaced = match state.replace_live_candle_stream(make_live_stream_task("MSFT", false))
        {
            Ok(Some(replaced)) => replaced,
            Ok(None) => panic!("expected prior stream metadata on replacement"),
            Err((error, _task)) => panic!("replace second live stream failed: {error}"),
        };
        assert_eq!(replaced.symbol, "AAPL");
        assert!(replaced.delayed);

        let active = state
            .live_candle_stream_info()
            .expect("live stream info after replacement")
            .expect("replacement stream");
        assert_eq!(active.symbol, "MSFT");
        assert!(!active.delayed);
        assert!(active.active);
        assert_eq!(active.status, "started");
        assert_eq!(active.last_accepted_bar_received_at_unix_ms, None);

        state
            .record_live_candle_stream_accepted_bar("stocks", "MSFT", false, 2_000, 2_999, 3_100)
            .expect("record replacement accepted live bar");
        let stopped = state
            .stop_live_candle_stream()
            .expect("stop live stream")
            .expect("stopped stream metadata");
        assert_eq!(stopped.symbol, "MSFT");
        assert!(!stopped.delayed);
        let status = state
            .live_candle_stream_info()
            .expect("live stream info")
            .expect("last-known stopped status");
        assert_eq!(status.symbol, "MSFT");
        assert!(!status.active);
        assert_eq!(status.status, "stopped");
        assert_eq!(
            status.last_message.as_deref(),
            Some("live candle stream stopped")
        );
        assert_eq!(status.last_accepted_bar_received_at_unix_ms, Some(3_100));
    }

    #[test]
    fn live_candle_stream_info_clears_finished_tasks_lazily() {
        let state = AppState::default();
        let task = LiveCandleStreamTask {
            handle: tauri::async_runtime::spawn(async {}),
            market: "stocks".to_string(),
            symbol: "AAPL".to_string(),
            delayed: true,
        };

        match state.replace_live_candle_stream(task) {
            Ok(_) => {}
            Err((error, _task)) => panic!("replace finished live stream task failed: {error}"),
        }
        std::thread::sleep(Duration::from_millis(10));

        let status = state
            .live_candle_stream_info()
            .expect("live stream info after finished task")
            .expect("last-known closed status");
        assert_eq!(status.symbol, "AAPL");
        assert!(!status.active);
        assert_eq!(status.status, "closed");
    }
}

#[cfg(test)]
mod state_guard_tests {
    use super::*;

    #[test]
    fn run_guard_rejects_reentry_and_resets_on_drop() {
        let state = AppState::default();
        state.request_cancel();

        let guard = state
            .try_begin_run()
            .expect("first run guard should acquire");
        assert!(
            !state.cancel_token().load(Ordering::SeqCst),
            "acquiring the run guard should clear prior cancel intent"
        );
        assert!(
            state.try_begin_run().is_err(),
            "second run guard acquisition should be rejected while active"
        );
        let err = match state.try_begin_run() {
            Ok(_guard) => panic!("reentrant run guard acquisition should fail"),
            Err(err) => err,
        };
        assert_eq!(err, "a run is already in progress");

        drop(guard);

        let _guard = state
            .try_begin_run()
            .expect("run guard should be reacquirable after drop");
    }

    #[test]
    fn run_guard_uses_generic_run_wording() {
        let state = AppState::default();
        let _guard = state
            .try_begin_run()
            .expect("first run guard should acquire");

        let err = match state.try_begin_run() {
            Ok(_guard) => panic!("second run guard should be rejected"),
            Err(err) => err,
        };
        assert_eq!(err, "a run is already in progress");
    }

    #[test]
    fn background_task_guard_rejects_reentry_and_resets_on_drop() {
        let state = BackgroundTaskState::default();
        state.request_cancel();

        let guard = state
            .try_begin()
            .expect("first background task guard should acquire");
        assert!(
            !state.cancel_token().load(Ordering::SeqCst),
            "acquiring the background guard should clear prior cancel intent"
        );
        assert!(
            state.try_begin().is_err(),
            "second background guard acquisition should be rejected while active"
        );

        drop(guard);

        let _guard = state
            .try_begin()
            .expect("background guard should be reacquirable after drop");
    }
}

#[cfg(test)]
type AppStateTestHook = Box<dyn FnOnce(&AppState) + Send + 'static>;

#[cfg(test)]
fn load_before_insert_hook_slot() -> &'static Mutex<Option<AppStateTestHook>> {
    static HOOK: std::sync::OnceLock<Mutex<Option<AppStateTestHook>>> = std::sync::OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn resample_before_insert_hook_slot() -> &'static Mutex<Option<AppStateTestHook>> {
    static HOOK: std::sync::OnceLock<Mutex<Option<AppStateTestHook>>> = std::sync::OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn install_load_before_insert_hook(hook: AppStateTestHook) {
    *load_before_insert_hook_slot()
        .lock()
        .expect("lock load-before-insert hook") = Some(hook);
}

#[cfg(test)]
fn install_resample_before_insert_hook(hook: AppStateTestHook) {
    *resample_before_insert_hook_slot()
        .lock()
        .expect("lock resample-before-insert hook") = Some(hook);
}

#[cfg(test)]
fn run_load_before_insert_hook(state: &AppState) {
    let hook = load_before_insert_hook_slot()
        .lock()
        .expect("lock load-before-insert hook")
        .take();
    if let Some(hook) = hook {
        hook(state);
    }
}

#[cfg(test)]
fn run_resample_before_insert_hook(state: &AppState) {
    let hook = resample_before_insert_hook_slot()
        .lock()
        .expect("lock resample-before-insert hook")
        .take();
    if let Some(hook) = hook {
        hook(state);
    }
}
