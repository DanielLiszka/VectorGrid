use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use std::path::Path;

use serde::Serialize;
use vector_ta::utilities::data_loader::Candles;

use crate::data_cache;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PriceDataMeta {
    pub len: usize,
    pub has_open: bool,
    pub has_high: bool,
    pub has_low: bool,
    pub has_close: bool,
    pub has_volume: bool,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            candles: Mutex::new(HashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
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
        let meta = PriceDataMeta {
            len: candles.close.len(),
            has_open: candles.fields.open,
            has_high: candles.fields.high,
            has_low: candles.fields.low,
            has_close: candles.fields.close,
            has_volume: candles.fields.volume,
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

    pub fn get_candles(&self, id: &str) -> Result<Candles, String> {
        let map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        map.get(id)
            .cloned()
            .ok_or_else(|| format!("unknown data id: {id}"))
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
}
