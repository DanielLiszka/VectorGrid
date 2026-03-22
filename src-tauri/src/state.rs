use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
            loaded_data_context: Mutex::new(HashMap::new()),
            next_dataset_id: AtomicU64::new(0),
            access_seq: AtomicU64::new(0),
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
        let candles = outcome.candles;
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
            let ctx = self.get_loaded_data_context(&out_id).ok();
            return Ok(LoadPriceDataResponse {
                id: out_id,
                meta,
                cache_hit: true,
                cache_path: ctx.map(|loaded| loaded.csv_path).unwrap_or_default(),
            });
        }

        let base = self.get_candles(base_data_id)?;
        let base_ctx = self.get_loaded_data_context(base_data_id)?;
        let derived = ta_app_core::resample_candles(base.as_ref(), tf.interval_ms)?;
        let meta = build_price_data_meta(&derived, Some(tf.label.clone()), None, None);

        let evicted_ids = {
            let mut map = self
                .candles
                .lock()
                .map_err(|_| "failed to lock candles map".to_string())?;
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
            enforce_dataset_retention_locked(&mut map, &protect_ids)
        };
        self.remove_loaded_context_entries(&evicted_ids)?;

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
                let candles = Arc::make_mut(&mut entry.candles);
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

    fn find_matching_loaded_base(
        &self,
        path: &str,
        source_identity: data_cache::SourceIdentity,
    ) -> Result<Option<LoadPriceDataResponse>, String> {
        let access_seq = self.next_access_seq();

        let ctx_map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;
        let candidate_id = ctx_map.iter().find_map(|(id, ctx)| {
            (ctx.timeframe.is_none()
                && ctx.csv_path == path
                && ctx.source_identity == Some(source_identity))
            .then(|| id.clone())
        });
        drop(ctx_map);

        let Some(id) = candidate_id else {
            return Ok(None);
        };

        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
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

fn eviction_candidate_locked(
    map: &HashMap<String, DatasetEntry>,
    kind: DatasetRetentionKind,
    protect_ids: &HashSet<String>,
) -> Option<String> {
    let mut candidates: Vec<(bool, u64, String)> =
        map.iter()
            .filter_map(|(id, entry)| {
                (entry.version_meta.kind.retention_kind() == kind && !protect_ids.contains(id))
                    .then(|| {
                        (
                            Arc::strong_count(&entry.candles) > 1,
                            entry.last_access_seq,
                            id.clone(),
                        )
                    })
            })
            .collect();
    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    candidates.into_iter().next().map(|(_, _, id)| id)
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

    fn make_trending_candles(len: usize, step: f64) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);
        let base_ts: i64 = 1_700_000_000_000;
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

    #[test]
    fn load_replaces_prior_dataset_and_discards_prior_derived_cache() {
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
    fn base_live_update_replaces_arc_and_invalidates_active_derived_dataset() {
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
    }

    #[test]
    fn identical_base_load_reuses_existing_id_and_preserves_derived_cache() {
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
        assert!(Arc::ptr_eq(&first_arc, &second_arc));
        assert!(state.get_candles(&derived.id).is_ok());
        assert_eq!(
            loaded.source_identity,
            Some(data_cache::identity_for_source(&csv).expect("source identity"))
        );
    }

    #[test]
    fn matching_base_load_updates_access_sequence() {
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
    fn retention_limits_can_be_overridden_by_env() {
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
