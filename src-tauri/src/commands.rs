use tauri::Emitter;
use tauri::Manager;

use crate::ma_registry::MaInfo;
use crate::polygon;
use crate::state::AppState;
use crate::state::LoadPriceDataResponse;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub use ta_app_core::{
    Backend, BackendOptimizationResult, BackendUsed, CompositeCpuDrilldownRequest,
    CompositeCpuLocalStabilityRequest, CompositeCpuRequest, CompositeMoRequest, CompositeMoResult,
    CompositeValidationRequest, CompositeValidationResult, DoubleMaCurves, DoubleMaDrilldownRequest,
    DoubleMaLocalStabilityRequest, DoubleMaParamsResolved, DoubleMaRequest, DoubleMaValidationRequest,
    DoubleMaValidationResult, LocalStabilityResult,
    MlConfidenceLabelConfig, MlConfidenceModel, MlConfidenceScope, MlConfidenceTrainConfig,
    MlConfidenceTrainMetrics,
    MetaSelectionRequest, MetaSelectionResult, OptimizationModeResolved,
};

#[derive(Debug, Clone, Serialize)]
pub struct GpuStatus {
    pub built_with_cuda: bool,
    pub built_with_cuda_backtest_kernel: bool,
    pub cuda_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CudaDeviceInfo {
    pub device_id: u32,
    pub name: String,
    pub total_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListCudaDevicesResponse {
    pub built_with_cuda: bool,
    pub cuda_available: bool,
    pub devices: Vec<CudaDeviceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_device_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(feature = "cuda")]
fn cuda_available_now() -> bool {
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn cuda_available_now() -> bool {
    false
}

#[tauri::command]
pub fn get_gpu_status() -> Result<GpuStatus, String> {
    Ok(GpuStatus {
        built_with_cuda: cfg!(feature = "cuda"),
        built_with_cuda_backtest_kernel: cfg!(feature = "cuda-backtest-kernel"),
        cuda_available: cuda_available_now(),
    })
}

#[cfg(feature = "cuda")]
fn list_cuda_devices_impl() -> Result<ListCudaDevicesResponse, String> {
    if !cuda_available_now() {
        return Ok(ListCudaDevicesResponse {
            built_with_cuda: true,
            cuda_available: false,
            devices: Vec::new(),
            recommended_device_id: None,
            error: Some("CUDA runtime not available (cuda_available() == false)".to_string()),
        });
    }

    use cust::{device::Device, prelude::CudaFlags};

    cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;

    let n = Device::num_devices().map_err(|e| e.to_string())?;
    let mut devices: Vec<CudaDeviceInfo> = Vec::with_capacity(n as usize);
    for i in 0..n {
        let dev = Device::get_device(i).map_err(|e| e.to_string())?;
        let name = dev.name().map_err(|e| e.to_string())?;
        let mem = dev.total_memory().map_err(|e| e.to_string())?;
        devices.push(CudaDeviceInfo {
            device_id: i as u32,
            name,
            total_memory_bytes: mem as u64,
        });
    }

    let mut recommended: Option<u32> = None;
    let mut best_mem: u64 = 0;
    for d in &devices {
        if recommended.is_none() || d.total_memory_bytes > best_mem {
            recommended = Some(d.device_id);
            best_mem = d.total_memory_bytes;
        }
    }

    Ok(ListCudaDevicesResponse {
        built_with_cuda: true,
        cuda_available: true,
        devices,
        recommended_device_id: recommended,
        error: None,
    })
}

#[cfg(not(feature = "cuda"))]
fn list_cuda_devices_impl() -> Result<ListCudaDevicesResponse, String> {
    Ok(ListCudaDevicesResponse {
        built_with_cuda: false,
        cuda_available: false,
        devices: Vec::new(),
        recommended_device_id: None,
        error: Some("App not built with CUDA feature".to_string()),
    })
}

#[tauri::command]
pub fn list_cuda_devices() -> Result<ListCudaDevicesResponse, String> {
    list_cuda_devices_impl()
}

#[tauri::command]
pub fn load_price_data(
    path: String,
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
) -> Result<LoadPriceDataResponse, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);
    state.load_price_data(&path, &cache_dir)
}

#[tauri::command]
pub fn list_moving_averages() -> Vec<MaInfo> {
    crate::ma_registry::list_mas()
}

#[derive(Debug, Clone, Serialize)]
pub struct SampleCsv {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[tauri::command]
pub fn list_sample_csvs() -> Vec<SampleCsv> {
    let root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let candidates: [(&str, &str, &str); 3] = [
        ("10k", "10k candles", "src/data/10kCandles.csv"),
        (
            "100k",
            "100k candles",
            "src/data/bitfinex btc-usd 100,000 candles ends 09-01-24.csv",
        ),
        ("1m", "1M candles", "src/data/1MillionCandles.csv"),
    ];

    let mut out: Vec<SampleCsv> = Vec::new();
    for (id, label, rel) in candidates {
        let path = root.join(rel);
        if path.exists() {
            out.push(SampleCsv {
                id: id.to_string(),
                label: label.to_string(),
                path: path.to_string_lossy().to_string(),
            });
        }
    }
    out
}

#[tauri::command]
pub fn polygon_get_cache_info(app: tauri::AppHandle) -> Result<polygon::PolygonCacheInfo, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = polygon::polygon_cache_dir(&base);
    polygon::get_cache_info(&app.config().identifier, &dir)
}

#[tauri::command]
pub fn polygon_set_api_key(api_key: String, app: tauri::AppHandle) -> Result<(), String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = polygon::polygon_cache_dir(&base);
    polygon::set_api_key(&app.config().identifier, &dir, &api_key)
}

#[tauri::command]
pub fn polygon_clear_api_key(app: tauri::AppHandle) -> Result<(), String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = polygon::polygon_cache_dir(&base);
    polygon::clear_api_key(&app.config().identifier, &dir)
}

#[tauri::command]
pub async fn polygon_validate_dataset(
    csv_path: String,
    app: tauri::AppHandle,
) -> Result<polygon::PolygonDatasetMeta, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = polygon::polygon_cache_dir(&base);
    tauri::async_runtime::spawn_blocking(move || polygon::validate_cached_dataset(&dir, &csv_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn polygon_repair_dataset(
    csv_path: String,
    app: tauri::AppHandle,
) -> Result<polygon::PolygonDatasetMeta, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = polygon::polygon_cache_dir(&base);
    let app_identifier = app.config().identifier.clone();
    tauri::async_runtime::spawn_blocking(move || {
        polygon::repair_cached_dataset(&app_identifier, &dir, &csv_path)
    })
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct PolygonDownloadAndLoadResponse {
    pub csv_path: String,
    pub downloaded_bars: usize,
    pub used_cache_only: bool,
    #[serde(flatten)]
    pub load: LoadPriceDataResponse,
}

#[tauri::command]
pub async fn polygon_download_1m_and_load(
    req: polygon::PolygonDownloadRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PolygonDownloadAndLoadResponse, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = polygon::polygon_cache_dir(&base);
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);
    let app_identifier = app.config().identifier.clone();
    let req2 = req.clone();

    let dl = tauri::async_runtime::spawn_blocking(move || {
        polygon::download_1m_to_cache(&app_identifier, &dir, &req2)
    })
    .await
    .map_err(|e| e.to_string())??;

    let csv_path = dl.csv_path.to_string_lossy().to_string();
    let load = state.load_price_data(&csv_path, &cache_dir)?;
    Ok(PolygonDownloadAndLoadResponse {
        csv_path,
        downloaded_bars: dl.downloaded_bars,
        used_cache_only: dl.used_cache_only,
        load,
    })
}

#[cfg(target_os = "windows")]
fn utf16z_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[tauri::command]
pub fn pick_csv_file() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
            OFN_HIDEREADONLY, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };

        let filter = "CSV Files (*.csv)\0*.csv\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        let mut file_buf = vec![0u16; 8192];

        let mut ofn = OPENFILENAMEW::default();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;

        let ok = unsafe { GetOpenFileNameW(&mut ofn) };
        if ok != 0 {
            return Ok(Some(utf16z_to_string(&file_buf)));
        }

        let err = unsafe { CommDlgExtendedError() };
        if err == 0 {
            return Ok(None);
        }
        return Err(format!(
            "Open file dialog failed (CommDlgExtendedError={err})."
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native file dialogs are only implemented on Windows in this demo. Please paste a CSV path.".to_string())
    }
}

#[tauri::command]
pub fn pick_save_csv(default_name: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            CommDlgExtendedError, GetSaveFileNameW, OFN_EXPLORER, OFN_HIDEREADONLY,
            OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };

        let filter = "CSV Files (*.csv)\0*.csv\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        let defext = "csv\0".encode_utf16().collect::<Vec<u16>>();

        let mut file_buf = vec![0u16; 8192];
        if let Some(name) = default_name {
            let name = name.trim();
            if !name.is_empty() {
                let w = name.encode_utf16().collect::<Vec<u16>>();
                let max = file_buf.len().saturating_sub(1);
                let n = w.len().min(max);
                file_buf[..n].copy_from_slice(&w[..n]);
                file_buf[n] = 0;
            }
        }

        let mut ofn = OPENFILENAMEW::default();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrDefExt = defext.as_ptr();
        ofn.Flags = OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_OVERWRITEPROMPT;

        let ok = unsafe { GetSaveFileNameW(&mut ofn) };
        if ok != 0 {
            return Ok(Some(utf16z_to_string(&file_buf)));
        }

        let err = unsafe { CommDlgExtendedError() };
        if err == 0 {
            return Ok(None);
        }
        return Err(format!(
            "Save file dialog failed (CommDlgExtendedError={err})."
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = default_name;
        Err("Native file dialogs are only implemented on Windows in this demo. Please type an export path.".to_string())
    }
}

#[tauri::command]
pub fn cancel_double_ma_optimization(state: tauri::State<AppState>) -> Result<(), String> {
    state.request_cancel();
    Ok(())
}

#[tauri::command]
pub fn cancel_current_run(state: tauri::State<AppState>) -> Result<(), String> {
    state.request_cancel();
    Ok(())
}

struct TauriProgressSink {
    app: tauri::AppHandle,
}

impl ta_app_core::progress::ProgressSink for TauriProgressSink {
    fn emit_double_ma_progress(&self, payload: ta_app_core::progress::DoubleMaProgressPayload) {
        let _ = self.app.emit("double_ma_progress", payload);
    }
}

struct OffsetProgressSink {
    app: tauri::AppHandle,
    base_offset: usize,
    total: usize,
    phase: &'static str,
}

impl ta_app_core::progress::ProgressSink for OffsetProgressSink {
    fn emit_double_ma_progress(&self, payload: ta_app_core::progress::DoubleMaProgressPayload) {
        let _ = self.app.emit(
            "double_ma_progress",
            ta_app_core::progress::DoubleMaProgressPayload {
                processed_pairs: self.base_offset.saturating_add(payload.processed_pairs),
                total_pairs: self.total,
                phase: self.phase,
            },
        );
    }
}

#[tauri::command]
pub async fn run_double_ma_optimization(
    req: DoubleMaRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sink = TauriProgressSink { app: app2 };
        ta_app_core::run_double_ma_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_composite_cpu_optimization(
    req: CompositeCpuRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sink = TauriProgressSink { app: app2 };
        ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_composite_mo_optimization(
    req: CompositeMoRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CompositeMoResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sink = TauriProgressSink { app: app2 };
        ta_app_core::run_composite_mo_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_composite_validation(
    req: CompositeValidationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CompositeValidationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let sink = TauriProgressSink { app: app2 };
        let mut base_req = req;
        let ma_source = base_req.ma_source.clone();
        let fast_ma_params = base_req.fast_ma_params.clone();
        let slow_ma_params = base_req.slow_ma_params.clone();
        let seed = base_req.seed;
        let cross_cfg = base_req.cross_ticker.take();
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            base_req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )?;

        if let Some(cross_cfg) = cross_cfg {
            apply_cross_ticker_validation(
                &mut res,
                cross_cfg,
                &ma_source,
                fast_ma_params,
                slow_ma_params,
                seed,
                &app_cache_dir,
                cancel2.as_ref(),
                Some(sink.app.clone()),
            )?;
        }
        res.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn apply_cross_ticker_validation(
    res: &mut CompositeValidationResult,
    mut cross_cfg: ta_app_core::CrossTickerConfig,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    app_for_progress: Option<tauri::AppHandle>,
) -> Result<(), String> {
    // Sanitize dataset list (dedup + cap).
    cross_cfg
        .datasets
        .retain(|d| !d.csv_path.trim().is_empty());
    let mut seen: HashSet<String> = HashSet::with_capacity(cross_cfg.datasets.len() * 2);
    cross_cfg
        .datasets
        .retain(|d| seen.insert(d.csv_path.trim().to_string()));
    if cross_cfg.max_datasets == 0 {
        return Ok(());
    }
    if cross_cfg.datasets.len() > cross_cfg.max_datasets {
        cross_cfg.datasets.truncate(cross_cfg.max_datasets);
    }
    if cross_cfg.datasets.is_empty() || res.candidates.is_empty() {
        return Ok(());
    }

    #[derive(Clone, Default)]
    struct Acc {
        sharpes: Vec<f64>,
        dds: Vec<f64>,
        pnls: Vec<f64>,
        trades: Vec<u32>,
        pnl_pos: usize,
    }

    let mut key_to_idx: HashMap<String, usize> = HashMap::with_capacity(res.candidates.len() * 2);
    for (i, c) in res.candidates.iter().enumerate() {
        let key = serde_json::to_string(&c.params).map_err(|e| e.to_string())?;
        key_to_idx.insert(key, i);
    }

    let total_candidates = res.candidates.len();
    let cross_total = total_candidates.saturating_mul(cross_cfg.datasets.len());
    let mut accs: Vec<Acc> = vec![Acc::default(); total_candidates];
    let mut datasets_meta: Vec<ta_app_core::CrossTickerDatasetMeta> =
        Vec::with_capacity(cross_cfg.datasets.len());

    let mk_label = |csv_path: &str, label: &str| -> String {
        let l = label.trim();
        if !l.is_empty() {
            return l.to_string();
        }
        Path::new(csv_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("dataset")
            .to_string()
    };

    for (di, ds) in cross_cfg.datasets.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let csv_path = ds.csv_path.trim();
        if csv_path.is_empty() {
            continue;
        }
        let outcome = crate::data_cache::load_or_build(Path::new(csv_path), app_cache_dir)?;
        let candles2 = outcome.candles;
        datasets_meta.push(ta_app_core::CrossTickerDatasetMeta {
            label: mk_label(csv_path, &ds.label),
            csv_path: csv_path.to_string(),
            bars: candles2.close.len(),
        });

        let req2 = ta_app_core::CompositeValidationRequest {
            data_id: format!("cross-{di}"),
            ma_source: ma_source.to_string(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
            candidates: res.candidates.iter().map(|c| c.params.clone()).collect(),
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed,
            max_candidates: total_candidates,
        };
        let offset = di.saturating_mul(total_candidates);

        let res2 = if let Some(app) = app_for_progress.as_ref() {
            let sink2 = OffsetProgressSink {
                app: app.clone(),
                base_offset: offset,
                total: cross_total,
                phase: "composite_validate_cross",
            };
            ta_app_core::run_composite_validation_blocking_with_candles(
                req2,
                &candles2,
                cancel,
                Some(&sink2),
            )?
        } else {
            ta_app_core::run_composite_validation_blocking_with_candles(req2, &candles2, cancel, None)?
        };

        for c in &res2.candidates {
            let key = serde_json::to_string(&c.params).map_err(|e| e.to_string())?;
            let Some(&idx) = key_to_idx.get(&key) else {
                continue;
            };
            accs[idx].sharpes.push(c.in_sample.sharpe);
            accs[idx].dds.push(c.in_sample.max_dd);
            accs[idx].pnls.push(c.in_sample.pnl);
            accs[idx].trades.push(c.in_sample.trades);
            if c.in_sample.pnl > 0.0 {
                accs[idx].pnl_pos = accs[idx].pnl_pos.saturating_add(1);
            }
        }
    }

    // Aggregate stats.
    let quantile = |sorted: &[f64], q: f64| -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        if sorted.len() == 1 {
            return if sorted[0].is_finite() { sorted[0] } else { 0.0 };
        }
        let qq = q.clamp(0.0, 1.0);
        let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
        let v = sorted[idx.min(sorted.len() - 1)];
        if v.is_finite() { v } else { 0.0 }
    };
    let median = |sorted: &[f64]| -> f64 { quantile(sorted, 0.5) };
    let worst_min = |xs: &[f64]| -> f64 {
        xs.iter()
            .copied()
            .fold(f64::INFINITY, |a, b| if b < a { b } else { a })
            .if_finite_or(0.0)
    };
    let worst_max = |xs: &[f64]| -> f64 {
        xs.iter()
            .copied()
            .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a })
            .if_finite_or(0.0)
    };
    trait FiniteOr {
        fn if_finite_or(self, fallback: f64) -> f64;
    }
    impl FiniteOr for f64 {
        fn if_finite_or(self, fallback: f64) -> f64 {
            if self.is_finite() {
                self
            } else {
                fallback
            }
        }
    }

    let median_u32 = |xs: &[u32]| -> u32 {
        if xs.is_empty() {
            return 0;
        }
        let mut ys = xs.to_vec();
        ys.sort_unstable();
        ys[ys.len() / 2]
    };
    let worst_min_u32 = |xs: &[u32]| -> u32 { xs.iter().copied().min().unwrap_or(0) };

    for (i, cand) in res.candidates.iter_mut().enumerate() {
        let acc = &mut accs[i];
        let ds_n = acc.sharpes.len();
        if ds_n == 0 {
            continue;
        }

        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));

        let sharpe_median = median(&acc.sharpes);
        let max_dd_worst = worst_max(&acc.dds);
        cand.cross_ticker = Some(ta_app_core::CrossTickerStats {
            datasets: ds_n,
            sharpe_median,
            sharpe_worst: worst_min(&acc.sharpes),
            max_dd_median: median(&acc.dds),
            max_dd_worst,
            pnl_median: median(&acc.pnls),
            pnl_worst: worst_min(&acc.pnls),
            trades_median: median_u32(&acc.trades),
            trades_worst: worst_min_u32(&acc.trades),
            pnl_positive_frac: acc.pnl_pos as f64 / ds_n as f64,
            score: sharpe_median - 2.0 * max_dd_worst,
        });
    }

    // Mark cross-ticker Pareto front using (Sharpe median, MaxDD worst).
    for i in 0..res.candidates.len() {
        let Some(si) = res.candidates[i].cross_ticker.as_ref() else {
            continue;
        };
        let xi = si.sharpe_median;
        let yi = si.max_dd_worst;
        let mut dominated = false;
        for j in 0..res.candidates.len() {
            if i == j {
                continue;
            }
            let Some(sj) = res.candidates[j].cross_ticker.as_ref() else {
                continue;
            };
            let xj = sj.sharpe_median;
            let yj = sj.max_dd_worst;
            if xj >= xi && yj <= yi && (xj > xi || yj < yi) {
                dominated = true;
                break;
            }
        }
        res.candidates[i].cross_ticker_pareto = Some(!dominated);
    }

    res.cross_ticker = Some(ta_app_core::CrossTickerValidationSummary {
        datasets: datasets_meta,
    });
    Ok(())
}

#[tauri::command]
pub async fn run_double_ma_validation(
    req: DoubleMaValidationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<DoubleMaValidationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let sink = TauriProgressSink { app: app2 };
        let mut base_req = req;
        let ma_source = base_req.ma_source.clone();
        let fast_ma_params = base_req.fast_ma_params.clone();
        let slow_ma_params = base_req.slow_ma_params.clone();
        let seed = base_req.seed;
        let cross_cfg = base_req.cross_ticker.take();
        let mut res = ta_app_core::run_double_ma_validation_blocking_with_candles(
            base_req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )?;

        if let Some(cross_cfg) = cross_cfg {
            apply_cross_ticker_validation_double_ma(
                &mut res,
                cross_cfg,
                &ma_source,
                fast_ma_params,
                slow_ma_params,
                seed,
                &app_cache_dir,
                cancel2.as_ref(),
                Some(sink.app.clone()),
            )?;
        }
        res.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn apply_cross_ticker_validation_double_ma(
    res: &mut DoubleMaValidationResult,
    mut cross_cfg: ta_app_core::CrossTickerConfig,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    app_for_progress: Option<tauri::AppHandle>,
) -> Result<(), String> {
    // Sanitize dataset list (dedup + cap).
    cross_cfg
        .datasets
        .retain(|d| !d.csv_path.trim().is_empty());
    let mut seen: HashSet<String> = HashSet::with_capacity(cross_cfg.datasets.len() * 2);
    cross_cfg
        .datasets
        .retain(|d| seen.insert(d.csv_path.trim().to_string()));
    if cross_cfg.max_datasets == 0 {
        return Ok(());
    }
    if cross_cfg.datasets.len() > cross_cfg.max_datasets {
        cross_cfg.datasets.truncate(cross_cfg.max_datasets);
    }
    if cross_cfg.datasets.is_empty() || res.candidates.is_empty() {
        return Ok(());
    }

    #[derive(Clone, Default)]
    struct Acc {
        sharpes: Vec<f64>,
        dds: Vec<f64>,
        pnls: Vec<f64>,
        trades: Vec<u32>,
        pnl_pos: usize,
    }

    fn canonicalize_json_value(v: &Value) -> Value {
        match v {
            Value::Array(xs) => Value::Array(xs.iter().map(canonicalize_json_value).collect()),
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys {
                    if let Some(v2) = map.get(k) {
                        out.insert(k.clone(), canonicalize_json_value(v2));
                    }
                }
                Value::Object(out)
            }
            _ => v.clone(),
        }
    }

    let mut key_to_idx: HashMap<String, usize> = HashMap::with_capacity(res.candidates.len() * 2);
    for (i, c) in res.candidates.iter().enumerate() {
        let v = serde_json::to_value(&c.params).map_err(|e| e.to_string())?;
        let key = serde_json::to_string(&canonicalize_json_value(&v)).map_err(|e| e.to_string())?;
        key_to_idx.insert(key, i);
    }

    let total_candidates = res.candidates.len();
    let cross_total = total_candidates.saturating_mul(cross_cfg.datasets.len());
    let mut accs: Vec<Acc> = vec![Acc::default(); total_candidates];
    let mut datasets_meta: Vec<ta_app_core::CrossTickerDatasetMeta> =
        Vec::with_capacity(cross_cfg.datasets.len());

    let mk_label = |csv_path: &str, label: &str| -> String {
        let l = label.trim();
        if !l.is_empty() {
            return l.to_string();
        }
        Path::new(csv_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("dataset")
            .to_string()
    };

    for (di, ds) in cross_cfg.datasets.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let csv_path = ds.csv_path.trim();
        if csv_path.is_empty() {
            continue;
        }
        let outcome = crate::data_cache::load_or_build(Path::new(csv_path), app_cache_dir)?;
        let candles2 = outcome.candles;
        datasets_meta.push(ta_app_core::CrossTickerDatasetMeta {
            label: mk_label(csv_path, &ds.label),
            csv_path: csv_path.to_string(),
            bars: candles2.close.len(),
        });

        let req2 = ta_app_core::DoubleMaValidationRequest {
            data_id: format!("cross-{di}"),
            ma_source: ma_source.to_string(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
            candidates: res.candidates.iter().map(|c| c.params.clone()).collect(),
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed,
            max_candidates: total_candidates,
        };
        let offset = di.saturating_mul(total_candidates);

        let res2 = if let Some(app) = app_for_progress.as_ref() {
            let sink2 = OffsetProgressSink {
                app: app.clone(),
                base_offset: offset,
                total: cross_total,
                phase: "double_ma_validate_cross",
            };
            ta_app_core::run_double_ma_validation_blocking_with_candles(
                req2,
                &candles2,
                cancel,
                Some(&sink2),
            )?
        } else {
            ta_app_core::run_double_ma_validation_blocking_with_candles(req2, &candles2, cancel, None)?
        };

        for c in &res2.candidates {
            let v = serde_json::to_value(&c.params).map_err(|e| e.to_string())?;
            let key = serde_json::to_string(&canonicalize_json_value(&v)).map_err(|e| e.to_string())?;
            let Some(&idx) = key_to_idx.get(&key) else {
                continue;
            };
            accs[idx].sharpes.push(c.in_sample.sharpe);
            accs[idx].dds.push(c.in_sample.max_dd);
            accs[idx].pnls.push(c.in_sample.pnl);
            accs[idx].trades.push(c.in_sample.trades);
            if c.in_sample.pnl > 0.0 {
                accs[idx].pnl_pos = accs[idx].pnl_pos.saturating_add(1);
            }
        }
    }

    // Aggregate stats.
    let quantile = |sorted: &[f64], q: f64| -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        if sorted.len() == 1 {
            return if sorted[0].is_finite() { sorted[0] } else { 0.0 };
        }
        let qq = q.clamp(0.0, 1.0);
        let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
        let v = sorted[idx.min(sorted.len() - 1)];
        if v.is_finite() { v } else { 0.0 }
    };
    let median = |sorted: &[f64]| -> f64 { quantile(sorted, 0.5) };
    let worst_min = |xs: &[f64]| -> f64 {
        xs.iter()
            .copied()
            .fold(f64::INFINITY, |a, b| if b < a { b } else { a })
            .if_finite_or(0.0)
    };
    let worst_max = |xs: &[f64]| -> f64 {
        xs.iter()
            .copied()
            .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a })
            .if_finite_or(0.0)
    };
    trait FiniteOr {
        fn if_finite_or(self, fallback: f64) -> f64;
    }
    impl FiniteOr for f64 {
        fn if_finite_or(self, fallback: f64) -> f64 {
            if self.is_finite() { self } else { fallback }
        }
    }

    let median_u32 = |xs: &[u32]| -> u32 {
        if xs.is_empty() {
            return 0;
        }
        let mut ys = xs.to_vec();
        ys.sort_unstable();
        ys[ys.len() / 2]
    };
    let worst_min_u32 = |xs: &[u32]| -> u32 { xs.iter().copied().min().unwrap_or(0) };

    for (i, cand) in res.candidates.iter_mut().enumerate() {
        let acc = &mut accs[i];
        let ds_n = acc.sharpes.len();
        if ds_n == 0 {
            continue;
        }

        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));

        let sharpe_median = median(&acc.sharpes);
        let max_dd_worst = worst_max(&acc.dds);
        cand.cross_ticker = Some(ta_app_core::CrossTickerStats {
            datasets: ds_n,
            sharpe_median,
            sharpe_worst: worst_min(&acc.sharpes),
            max_dd_median: median(&acc.dds),
            max_dd_worst,
            pnl_median: median(&acc.pnls),
            pnl_worst: worst_min(&acc.pnls),
            trades_median: median_u32(&acc.trades),
            trades_worst: worst_min_u32(&acc.trades),
            pnl_positive_frac: acc.pnl_pos as f64 / ds_n as f64,
            score: sharpe_median - 2.0 * max_dd_worst,
        });
    }

    // Mark cross-ticker Pareto front using (Sharpe median, MaxDD worst).
    for i in 0..res.candidates.len() {
        let Some(si) = res.candidates[i].cross_ticker.as_ref() else {
            continue;
        };
        let xi = si.sharpe_median;
        let yi = si.max_dd_worst;
        let mut dominated = false;
        for j in 0..res.candidates.len() {
            if i == j {
                continue;
            }
            let Some(sj) = res.candidates[j].cross_ticker.as_ref() else {
                continue;
            };
            let xj = sj.sharpe_median;
            let yj = sj.max_dd_worst;
            if xj >= xi && yj <= yi && (xj > xi || yj < yi) {
                dominated = true;
                break;
            }
        }
        res.candidates[i].cross_ticker_pareto = Some(!dominated);
    }

    res.cross_ticker = Some(ta_app_core::CrossTickerValidationSummary {
        datasets: datasets_meta,
    });
    Ok(())
}

#[tauri::command]
pub async fn compute_double_ma_drilldown(
    req: DoubleMaDrilldownRequest,
    state: tauri::State<'_, AppState>,
) -> Result<DoubleMaCurves, String> {
    let candles = state.get_candles(&req.data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        ta_app_core::compute_double_ma_drilldown_blocking_with_candles(req, &candles)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn compute_composite_cpu_drilldown(
    req: CompositeCpuDrilldownRequest,
    state: tauri::State<'_, AppState>,
) -> Result<DoubleMaCurves, String> {
    let candles = state.get_candles(&req.data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        ta_app_core::compute_composite_cpu_drilldown_blocking_with_candles(req, &candles)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn compute_double_ma_local_stability(
    req: DoubleMaLocalStabilityRequest,
    state: tauri::State<'_, AppState>,
) -> Result<LocalStabilityResult, String> {
    let candles = state.get_candles(&req.data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        ta_app_core::compute_double_ma_local_stability_blocking_with_candles(req, &candles)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn compute_composite_cpu_local_stability(
    req: CompositeCpuLocalStabilityRequest,
    state: tauri::State<'_, AppState>,
) -> Result<LocalStabilityResult, String> {
    let candles = state.get_candles(&req.data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        ta_app_core::compute_composite_cpu_local_stability_blocking_with_candles(req, &candles)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn build_report_xlsx(report: Value) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || build_report_xlsx_blocking(report))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveReportXlsxResponse {
    pub path: String,
    pub bytes: usize,
}

#[tauri::command]
pub async fn build_validation_shortlist_csv(validation_res: Value) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || build_validation_shortlist_csv_blocking(validation_res))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_report_xlsx(
    report: Value,
    max_rows: Option<u32>,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    tauri::async_runtime::spawn_blocking(move || save_report_xlsx_blocking(report, max_rows, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_reports_dir(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || open_reports_dir_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

fn open_reports_dir_blocking(app: tauri::AppHandle) -> Result<String, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    reveal_path_blocking(dir.to_string_lossy().to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn reveal_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || reveal_path_blocking(path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_path(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_path_blocking(path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_window_effect(
    effect: Option<String>,
    dark: Option<bool>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    let eff = effect.unwrap_or_else(|| "none".to_string()).trim().to_lowercase();

    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_blur, apply_mica, clear_acrylic, clear_blur, clear_mica};

        let _ = clear_mica(&window);
        let _ = clear_acrylic(&window);
        let _ = clear_blur(&window);

        let tint = match dark {
            Some(true) => Some((18u8, 18u8, 18u8, 180u8)),
            Some(false) => Some((245u8, 245u8, 245u8, 180u8)),
            None => None,
        };

        match eff.as_str() {
            "" | "none" => Ok(()),
            "mica" => apply_mica(&window, dark).map_err(|e| e.to_string()),
            "acrylic" => apply_acrylic(&window, tint).map_err(|e| e.to_string()),
            "blur" => apply_blur(&window, tint).map_err(|e| e.to_string()),
            _ => Err(format!("Unknown window effect: {eff}")),
        }?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{
            apply_vibrancy, clear_vibrancy, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
            NSVisualEffectState,
        };

        let _ = clear_vibrancy(&window);
        match eff.as_str() {
            "" | "none" => Ok(()),
            "vibrancy" => apply_vibrancy(
                &window,
                NSVisualEffectMaterial::Sidebar,
                NSVisualEffectBlendingMode::BehindWindow,
                NSVisualEffectState::Active,
            )
            .map_err(|e| e.to_string()),
            _ => Err(format!("Unknown window effect: {eff}")),
        }?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (eff, dark, window);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveRunArtifactResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveBatchSummaryResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub strategy_kind: String,
    pub config_fingerprint: String,
    pub has_validation: bool,
    pub report_xlsx_path: String,
    pub csv_path: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub strategy_kind: String,
    pub timeframe: String,
    pub required_tag: String,
    pub groups: usize,
    pub artifacts_used: usize,
    pub holdout_datasets: usize,
    pub report_xlsx_path: String,
    pub tags: Vec<String>,
}

#[tauri::command]
pub async fn save_run_artifact(
    artifact: Value,
    app: tauri::AppHandle,
) -> Result<SaveRunArtifactResponse, String> {
    tauri::async_runtime::spawn_blocking(move || save_run_artifact_blocking(artifact, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_batch_summary(
    summary: Value,
    app: tauri::AppHandle,
) -> Result<SaveBatchSummaryResponse, String> {
    tauri::async_runtime::spawn_blocking(move || save_batch_summary_blocking(summary, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_run_artifacts(app: tauri::AppHandle) -> Result<Vec<RunArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || list_run_artifacts_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_meta_artifacts(app: tauri::AppHandle) -> Result<Vec<MetaArtifactSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || list_meta_artifacts_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_artifact_tags(
    path: String,
    tags: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || update_artifact_tags_blocking(path, tags, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_run_artifact(path: String, app: tauri::AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || load_run_artifact_blocking(path, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_run_artifact(path: String, app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || delete_run_artifact_blocking(path, app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_runs_dir(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || open_runs_dir_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct MlConfidenceModelStatus {
    pub exists: bool,
    pub model_path: String,
    pub bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<MlConfidenceModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainMlConfidenceResponse {
    pub model_path: String,
    pub model_bytes: usize,
    pub model: MlConfidenceModel,
    pub metrics: MlConfidenceTrainMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyMlConfidenceResponse {
    pub model_path: String,
    pub confidences: Vec<Option<f64>>,
    pub warnings: Vec<String>,
}

fn normalize_ml_scope(mut scope: MlConfidenceScope) -> Result<MlConfidenceScope, String> {
    scope.strategy_kind = scope.strategy_kind.trim().to_string();
    scope.timeframe = normalize_timeframe_label(&scope.timeframe);
    if scope.strategy_kind.is_empty() {
        return Err("strategy_kind is required".to_string());
    }
    if scope.timeframe.is_empty() {
        return Err("timeframe is required (e.g. '1m')".to_string());
    }
    Ok(scope)
}

fn ml_confidence_models_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("models").join("confidence");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn ml_confidence_model_path(
    scope: &MlConfidenceScope,
    app: &tauri::AppHandle,
) -> Result<std::path::PathBuf, String> {
    let dir = ml_confidence_models_dir(app)?;
    let sk = sanitize_filename_component(&scope.strategy_kind);
    let tf = sanitize_filename_component(&scope.timeframe);
    Ok(dir.join(format!("ml_confidence_{sk}_{tf}.json")))
}

fn load_meta_artifacts_blocking(app: &tauri::AppHandle) -> Result<Vec<Value>, String> {
    let dir = runs_dir(app)?;
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("vectorbt_meta_") {
            continue;
        }
        paths.push(path);
    }
    paths.sort();

    let mut out: Vec<Value> = Vec::with_capacity(paths.len());
    for p in paths {
        let raw = match std::fs::read_to_string(&p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push(v);
    }
    Ok(out)
}

#[tauri::command]
pub async fn ml_confidence_model_status(
    scope: MlConfidenceScope,
    app: tauri::AppHandle,
) -> Result<MlConfidenceModelStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path(&scope, &app)?;
        let model_path = path.to_string_lossy().to_string();
        if !path.exists() {
            return Ok(MlConfidenceModelStatus {
                exists: false,
                model_path,
                bytes: 0,
                model: None,
                error: None,
            });
        }

        let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len() as usize;
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match serde_json::from_str::<MlConfidenceModel>(&raw) {
            Ok(model) => Ok(MlConfidenceModelStatus {
                exists: true,
                model_path,
                bytes,
                model: Some(model),
                error: None,
            }),
            Err(e) => Ok(MlConfidenceModelStatus {
                exists: true,
                model_path,
                bytes,
                model: None,
                error: Some(e.to_string()),
            }),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn train_ml_confidence_model(
    scope: MlConfidenceScope,
    cfg: Option<MlConfidenceTrainConfig>,
    app: tauri::AppHandle,
) -> Result<TrainMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let cfg = cfg.unwrap_or_default();
        let metas = load_meta_artifacts_blocking(&app)?;
        let (model, metrics) =
            ta_app_core::train_ml_confidence_model_from_meta_artifacts(scope, &metas, cfg)?;

        let path = ml_confidence_model_path(&model.scope, &app)?;
        let bytes = serde_json::to_vec_pretty(&model).map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

        Ok(TrainMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            model_bytes: bytes.len(),
            model,
            metrics,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_ml_confidence_model(
    scope: MlConfidenceScope,
    groups: Vec<Value>,
    app: tauri::AppHandle,
) -> Result<ApplyMlConfidenceResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path(&scope, &app)?;
        if !path.exists() {
            return Err("ML confidence model not found. Train it first.".to_string());
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let model: MlConfidenceModel = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

        let mut warnings: Vec<String> = Vec::new();
        if normalize_timeframe_label(&model.scope.timeframe) != scope.timeframe
            || model.scope.strategy_kind != scope.strategy_kind
        {
            warnings.push("Model scope does not match request scope. Results may be unreliable.".to_string());
        }

        let confidences: Vec<Option<f64>> = groups
            .iter()
            .map(|g| ta_app_core::predict_ml_confidence_for_group(&model, g))
            .collect();

        Ok(ApplyMlConfidenceResponse {
            model_path: path.to_string_lossy().to_string(),
            confidences,
            warnings,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_ml_confidence_model(
    scope: MlConfidenceScope,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let scope = normalize_ml_scope(scope)?;
        let path = ml_confidence_model_path(&scope, &app)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
pub struct RunMetaSelectionResponse {
    pub ts: String,
    pub report_xlsx_path: String,
    pub report_xlsx_bytes: usize,
    pub meta_json_path: String,
    pub meta_json_bytes: usize,
    pub result: MetaSelectionResult,
}

#[tauri::command]
pub async fn run_meta_selection(
    req: MetaSelectionRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunMetaSelectionResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || run_meta_selection_blocking(req, app2, cancel2))
        .await
        .map_err(|e| e.to_string())?
}

fn runs_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn reports_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn suggest_meta_artifact_path(ts: &str, app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = runs_dir(app)?;
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorbt_meta_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.json"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.json"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn suggest_batch_summary_path(ts: &str, app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = runs_dir(app)?;
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorbt_batch_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.json"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.json"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn suggest_meta_report_path(
    ts: &str,
    app: &tauri::AppHandle,
    ext: &'static str,
) -> Result<std::path::PathBuf, String> {
    let dir = reports_dir(app)?;
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorbt_meta_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.{ext}"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.{ext}"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn run_meta_selection_blocking(
    mut req: MetaSelectionRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunMetaSelectionResponse, String> {
    let dir = runs_dir(&app)?;
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with("vectorbt_meta_") {
            continue;
        }
        paths.push(path);
    }
    paths.sort();

    let sink = TauriProgressSink { app: app.clone() };

    let mut artifacts: Vec<Value> = Vec::with_capacity(paths.len());
    for (i, p) in paths.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let raw = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        artifacts.push(v);
        ta_app_core::progress::emit_double_ma_progress(Some(&sink), i + 1, paths.len(), "meta_read");
    }

    let supports_holdout = req.strategy_kind == "CompositeCpu" || req.strategy_kind == "CompositeMo";
    let holdout_tag = if req.eval_holdout && supports_holdout {
        req.holdout_tag
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    } else {
        None
    };

    let (train_artifacts, holdout_artifacts) = if let Some(tag) = holdout_tag.as_deref() {
        let mut train: Vec<Value> = Vec::with_capacity(artifacts.len());
        let mut holdout: Vec<Value> = Vec::new();
        for a in artifacts {
            if artifact_has_tag(&a, tag) {
                holdout.push(a);
            } else {
                train.push(a);
            }
        }
        (train, holdout)
    } else {
        (artifacts, Vec::new())
    };

    // Parity guard: if timeframe filter isn't set, infer it from the candidate pool.
    // This prevents mixing artifacts across multiple timeframes in meta-selection.
    let mut auto_tf_warning: Option<String> = None;
    let tf_set = req
        .timeframe
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .is_some();
    if !tf_set {
        let mut uniq: HashSet<String> = HashSet::new();
        let mut tfs: Vec<String> = Vec::new();
        let mut missing = 0usize;

        for a in &train_artifacts {
            if !artifact_matches_meta_req(a, &req) {
                continue;
            }
            let raw = a
                .pointer("/dataset_meta/timeframe")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tf = normalize_timeframe_label(raw);
            if tf.is_empty() {
                missing += 1;
                continue;
            }
            if uniq.insert(tf.clone()) {
                tfs.push(tf);
            }
        }

        if tfs.len() == 1 {
            let tf = tfs[0].clone();
            req.timeframe = Some(tf.clone());
            if missing > 0 {
                auto_tf_warning = Some(format!(
                    "Detected timeframe='{tf}'. Excluded {missing} artifact(s) missing dataset_meta.timeframe."
                ));
            }
        } else if tfs.len() > 1 {
            tfs.sort();
            let list = tfs.join(", ");
            let miss = if missing > 0 {
                format!(" + {missing} missing timeframe")
            } else {
                String::new()
            };
            return Err(format!(
                "Meta selection requires a timeframe filter (found multiple timeframes: {list}{miss})."
            ));
        } else if missing > 0 {
            return Err(format!(
                "Meta selection requires a timeframe filter ({missing} candidate artifact(s) are missing dataset_meta.timeframe)."
            ));
        }
    }

    let mut result = ta_app_core::meta_select_from_artifacts(&req, &train_artifacts, Some(&sink));
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    if let Some(w) = auto_tf_warning {
        result.warnings.push(w);
    }

    if req.eval_holdout && !supports_holdout {
        result.warnings.push(format!(
            "Holdout evaluation is not supported for strategy_kind='{}'. Skipping.",
            req.strategy_kind
        ));
    }

    if let Some(tag) = holdout_tag.as_deref() {
        apply_holdout_evaluation(
            &mut result,
            &req,
            &holdout_artifacts,
            &app,
            cancel.as_ref(),
            &sink,
            tag,
        )?;
    }

    let ts = chrono::Utc::now().to_rfc3339();

    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 0, 1, "meta_export");
    let xlsx_bytes = build_meta_report_xlsx_bytes(&ts, &result)?;
    let report_path = suggest_meta_report_path(&ts, &app, "xlsx")?;
    std::fs::write(&report_path, &xlsx_bytes).map_err(|e| e.to_string())?;
    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 1, 1, "meta_export");

    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 0, 1, "meta_save");
    let meta_obj = serde_json::json!({
        "version": 1,
        "kind": "vectorbt_meta_selection",
        "ts": ts,
        "report_xlsx_path": report_path.to_string_lossy().to_string(),
        "tags": [],
        "req": req,
        "res": result,
    });
    let meta_bytes = serde_json::to_vec_pretty(&meta_obj).map_err(|e| e.to_string())?;
    let meta_path = suggest_meta_artifact_path(&ts, &app)?;
    std::fs::write(&meta_path, &meta_bytes).map_err(|e| e.to_string())?;
    // Retention policy: keep newest N untagged meta runs; tags preserve older runs.
    prune_old_meta_artifacts_blocking(&app, 100);
    ta_app_core::progress::emit_double_ma_progress(Some(&sink), 1, 1, "meta_save");

    Ok(RunMetaSelectionResponse {
        ts,
        report_xlsx_path: report_path.to_string_lossy().to_string(),
        report_xlsx_bytes: xlsx_bytes.len(),
        meta_json_path: meta_path.to_string_lossy().to_string(),
        meta_json_bytes: meta_bytes.len(),
        result,
    })
}

fn artifact_has_tag(artifact: &Value, tag: &str) -> bool {
    let tag = tag.trim();
    if tag.is_empty() {
        return false;
    }
    let tags = artifact
        .get("tags")
        .and_then(|v| v.as_array())
        .map_or(&[][..], |xs| xs.as_slice());
    tags.iter()
        .filter_map(|v| v.as_str())
        .any(|t| t.trim() == tag)
}

fn normalize_timeframe_label(raw: &str) -> String {
    let s0 = raw.trim().to_ascii_lowercase();
    if s0.is_empty() {
        return String::new();
    }
    // Back-compat: older artifacts may store inferred timeframes like "120s".
    if let Some(sec_str) = s0.strip_suffix('s') {
        if let Ok(sec) = sec_str.parse::<i64>() {
            let mapped = match sec {
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
            };
            if let Some(lbl) = mapped {
                return lbl.to_string();
            }
        }
    }
    s0
}

fn artifact_matches_meta_req(artifact: &Value, req: &MetaSelectionRequest) -> bool {
    let kind = artifact.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if kind != "vectorbt_run_artifact" {
        return false;
    }

    let strat = artifact
        .pointer("/run/req/strategy_kind")
        .and_then(|v| v.as_str())
        .or_else(|| artifact.pointer("/run/req/strategyKind").and_then(|v| v.as_str()))
        .unwrap_or("");
    if strat != req.strategy_kind {
        return false;
    }

    if let Some(tf) = req.timeframe.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let tf2 = artifact
            .pointer("/dataset_meta/timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if normalize_timeframe_label(tf2) != normalize_timeframe_label(tf) {
            return false;
        }
    }

    if let Some(tag) = req
        .required_tag
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if !artifact_has_tag(artifact, tag) {
            return false;
        }
    }

    artifact.pointer("/validation/res/candidates").is_some()
}

fn artifact_csv_path(artifact: &Value) -> Option<String> {
    let p = artifact
        .pointer("/data/csv_path")
        .and_then(|v| v.as_str())
        .or_else(|| artifact.pointer("/dataset_meta/csv_path").and_then(|v| v.as_str()))?;
    let s = p.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn artifact_dataset_label(artifact: &Value, csv_path: &str) -> String {
    let lbl = artifact
        .pointer("/dataset_meta/label")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(l) = lbl {
        return l;
    }
    Path::new(csv_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("holdout")
        .to_string()
}

#[derive(Default, Clone)]
struct HoldoutAcc {
    scores: Vec<f64>,
    sharpes: Vec<f64>,
    max_dds: Vec<f64>,
    pnls: Vec<f64>,
    passes: usize,
}

fn apply_holdout_evaluation(
    result: &mut MetaSelectionResult,
    req: &MetaSelectionRequest,
    holdout_artifacts: &[Value],
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
    sink: &TauriProgressSink,
    holdout_tag: &str,
) -> Result<(), String> {
    if result.groups.is_empty() {
        return Ok(());
    }

    let heavy = matches!(req.holdout_mode, ta_app_core::MetaHoldoutMode::Heavy);
    let (hv_seed, hv_wf, hv_perturb, hv_mc) = if heavy {
        if let Some(cfg) = req.holdout_validation.clone() {
            (cfg.seed, cfg.walk_forward, cfg.perturb, cfg.trade_bootstrap)
        } else {
            (1u64, None, None, None)
        }
    } else {
        (1u64, None, None, None)
    };

    // Filter to holdout artifacts that match the request and actually have validation results.
    let holdout: Vec<&Value> = holdout_artifacts
        .iter()
        .filter(|a| artifact_matches_meta_req(a, req))
        .collect();
    if holdout.is_empty() {
        return Ok(());
    }

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let mut uniq_paths: Vec<(String, String)> = Vec::new(); // (csv_path, label)
    let mut seen: HashSet<String> = HashSet::new();
    for art in &holdout {
        let Some(csv_path) = artifact_csv_path(art) else {
            continue;
        };
        if !seen.insert(csv_path.clone()) {
            continue;
        }
        uniq_paths.push((csv_path.clone(), artifact_dataset_label(art, &csv_path)));
    }
    if uniq_paths.is_empty() {
        return Ok(());
    }

    let mut idx_by_key: HashMap<String, usize> = HashMap::with_capacity(result.groups.len());
    for (i, g) in result.groups.iter().enumerate() {
        idx_by_key.insert(g.key.clone(), i);
    }

    // Group the shortlist by MA config so we can evaluate a batch at once.
    let mut groups_by_cfg: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, g) in result.groups.iter().enumerate() {
        let cfg_key = serde_json::to_string(&serde_json::json!({
            "ma_source": &g.ma_source,
            "fast_ma_params": &g.fast_ma_params,
            "slow_ma_params": &g.slow_ma_params,
        }))
        .unwrap_or_else(|_| "cfg".to_string());
        groups_by_cfg.entry(cfg_key).or_default().push(i);
    }

    let mut accs: Vec<HoldoutAcc> = vec![HoldoutAcc::default(); result.groups.len()];
    let mut skipped = 0usize;

    let total = uniq_paths.len();
    for (i, (csv_path, label)) in uniq_paths.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let candles = match crate::data_cache::load_or_build(Path::new(&csv_path), &cache_dir) {
            Ok(outcome) => outcome.candles,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let mut local: Vec<Vec<(f64, f64, f64, f64)>> = vec![Vec::new(); result.groups.len()]; // (score, sharpe, max_dd, pnl)
        let mut ok = true;

        for idxs in groups_by_cfg.values() {
            let Some(first) = idxs.first() else { continue };
            let g0 = &result.groups[*first];

            let mut candidates: Vec<ta_app_core::CompositeParams> = Vec::new();
            candidates.reserve(idxs.len().saturating_mul(3));
            for &gi in idxs {
                let g = &result.groups[gi];
                let reps = if !g.representatives.is_empty() {
                    g.representatives.as_slice()
                } else {
                    std::slice::from_ref(&g.params)
                };
                for rv in reps {
                    let p: ta_app_core::CompositeParams =
                        serde_json::from_value(rv.clone()).map_err(|e| e.to_string())?;
                    candidates.push(p);
                }
            }

            let max_candidates = candidates.len();
            let val_req = ta_app_core::CompositeValidationRequest {
                data_id: label.clone(),
                ma_source: g0.ma_source.clone(),
                fast_ma_params: g0.fast_ma_params.clone(),
                slow_ma_params: g0.slow_ma_params.clone(),
                candidates,
                walk_forward: hv_wf.clone(),
                perturb: hv_perturb.clone(),
                trade_bootstrap: hv_mc.clone(),
                cross_ticker: None,
                cscv_pbo: None,
                seed: hv_seed,
                max_candidates,
            };

            let val_res = match ta_app_core::run_composite_validation_blocking_with_candles(
                val_req,
                &candles,
                cancel,
                None,
            ) {
                Ok(r) => r,
                Err(_) => {
                    ok = false;
                    break;
                }
            };

            for cand in val_res.candidates {
                let params_v = serde_json::to_value(&cand.params).map_err(|e| e.to_string())?;
                let key = ta_app_core::meta_group_key_for_request(
                    req,
                    &params_v,
                    &g0.ma_source,
                    &g0.fast_ma_params,
                    &g0.slow_ma_params,
                );
                let Some(&gi) = idx_by_key.get(&key) else { continue };

                let (score, sharpe, max_dd, pnl) = if heavy {
                    let score = cand.robust_score;
                    if let Some(w) = cand.walk_forward.as_ref() {
                        (score, w.sharpe_median, w.max_dd_worst, w.pnl_median)
                    } else {
                        (score, cand.in_sample.sharpe, cand.in_sample.max_dd, cand.in_sample.pnl)
                    }
                } else {
                    let sharpe = cand.in_sample.sharpe;
                    let max_dd = cand.in_sample.max_dd;
                    let pnl = cand.in_sample.pnl;
                    (sharpe - 2.0 * max_dd, sharpe, max_dd, pnl)
                };

                local[gi].push((score, sharpe, max_dd, pnl));
            }
        }

        if !ok {
            skipped += 1;
            continue;
        }

        for (gi, rows) in local.into_iter().enumerate() {
            if rows.is_empty() {
                continue;
            }

            let mut scores: Vec<f64> = Vec::with_capacity(rows.len());
            let mut sharpes: Vec<f64> = Vec::with_capacity(rows.len());
            let mut max_dds: Vec<f64> = Vec::with_capacity(rows.len());
            let mut pnls: Vec<f64> = Vec::with_capacity(rows.len());
            for (score, sharpe, max_dd, pnl) in rows {
                scores.push(score);
                sharpes.push(sharpe);
                max_dds.push(max_dd);
                pnls.push(pnl);
            }
            scores.sort_by(|a, b| a.total_cmp(b));
            sharpes.sort_by(|a, b| a.total_cmp(b));
            max_dds.sort_by(|a, b| a.total_cmp(b));
            pnls.sort_by(|a, b| a.total_cmp(b));

            let score = quantile(&scores, 0.5);
            let sharpe = quantile(&sharpes, 0.5);
            // Conservative: for risk, take worst-of reps.
            let max_dd = max_dds.last().copied().unwrap_or(0.0);
            let pnl = quantile(&pnls, 0.5);

            accs[gi].scores.push(score);
            accs[gi].sharpes.push(sharpe);
            accs[gi].max_dds.push(max_dd);
            accs[gi].pnls.push(pnl);
            if score > 0.0 {
                accs[gi].passes = accs[gi].passes.saturating_add(1);
            }
        }

        ta_app_core::progress::emit_double_ma_progress(Some(sink), i + 1, total, "meta_holdout");
    }

    ta_app_core::progress::emit_double_ma_progress(Some(sink), total, total, "meta_holdout");

    let datasets = total.saturating_sub(skipped);
    if datasets == 0 {
        return Ok(());
    }

    result.holdout = Some(ta_app_core::HoldoutSummary { datasets, skipped });

    for (i, g) in result.groups.iter_mut().enumerate() {
        let mut acc = accs[i].clone();
        if acc.scores.is_empty() {
            continue;
        }
        acc.scores.sort_by(|a, b| a.total_cmp(b));
        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.max_dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));

        let pass_rate = (acc.passes as f64) / (datasets as f64);
        let score_median = quantile(&acc.scores, 0.5);
        let score_p10 = quantile(&acc.scores, 0.10);
        let sharpe_median = quantile(&acc.sharpes, 0.5);
        let max_dd_p90 = quantile(&acc.max_dds, 0.90);
        let pnl_median = quantile(&acc.pnls, 0.5);

        g.holdout = Some(ta_app_core::HoldoutAggregate {
            datasets,
            pass_rate,
            score_median,
            score_p10,
            sharpe_median,
            max_dd_p90,
            pnl_median,
        });
    }

    if skipped > 0 {
        result.warnings.push(format!(
            "Holdout evaluation: skipped {skipped} dataset(s) (missing/invalid CSV path or parse error) for tag='{holdout_tag}'."
        ));
    }

    Ok(())
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn run_artifact_path(artifact: &Value, app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = runs_dir(app)?;

    let ts = artifact.get("ts").and_then(|v| v.as_str()).unwrap_or("unknown");
    let safe_ts = sanitize_filename_component(ts);
    Ok(dir.join(format!("vectorbt_run_{safe_ts}.json")))
}

fn ensure_path_is_within_dir(path: &std::path::Path, dir: &std::path::Path) -> Result<(), String> {
    let dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !path.starts_with(&dir) {
        return Err("path is outside app runs dir".to_string());
    }
    Ok(())
}

fn save_run_artifact_blocking(
    artifact: Value,
    app: tauri::AppHandle,
) -> Result<SaveRunArtifactResponse, String> {
    let path = run_artifact_path(&artifact, &app)?;
    let bytes = serde_json::to_vec_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    // Retention policy: keep newest N untagged runs, but never delete tagged runs.
    prune_old_run_artifacts_blocking(&app, 200);
    Ok(SaveRunArtifactResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn save_batch_summary_blocking(
    summary: Value,
    app: tauri::AppHandle,
) -> Result<SaveBatchSummaryResponse, String> {
    let ts = summary.get("ts").and_then(|v| v.as_str()).unwrap_or("unknown");
    let path = suggest_batch_summary_path(ts, &app)?;
    let bytes = serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(SaveBatchSummaryResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn prune_old_run_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    let Ok(dir) = runs_dir(app) else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else { return };

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("vectorbt_run_") {
            continue;
        }
        paths.push(path);
    }

    // Newest first (ISO timestamps sort lexicographically).
    paths.sort_by(|a, b| {
        let aa = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let bb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        bb.cmp(aa)
    });

    if paths.len() <= keep_newest {
        return;
    }

    for path in paths.into_iter().skip(keep_newest) {
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tags = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map_or(&[][..], |xs| xs.as_slice());
        let tagged = tags.iter().any(|t| t.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false));
        if tagged {
            continue;
        }
        let _ = std::fs::remove_file(&path);
    }
}

fn prune_old_meta_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    let Ok(dir) = runs_dir(app) else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else { return };

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("vectorbt_meta_") {
            continue;
        }
        paths.push(path);
    }

    // Newest first (ISO timestamps sort lexicographically).
    paths.sort_by(|a, b| {
        let aa = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let bb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        bb.cmp(aa)
    });

    if paths.len() <= keep_newest {
        return;
    }

    for path in paths.into_iter().skip(keep_newest) {
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tags = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map_or(&[][..], |xs| xs.as_slice());
        let tagged = tags.iter().any(|t| t.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false));
        if tagged {
            continue;
        }
        let _ = std::fs::remove_file(&path);
    }
}

fn open_runs_dir_blocking(app: tauri::AppHandle) -> Result<String, String> {
    let dir = runs_dir(&app)?;
    reveal_path_blocking(dir.to_string_lossy().to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

fn canonical_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(xs) => {
            let mut out = String::from("[");
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(x));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                let v = map.get(*k).unwrap_or(&Value::Null);
                out.push_str(&canonical_json(v));
            }
            out.push('}');
            out
        }
    }
}

fn fnv1a64_hex_utf16(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let prime: u64 = 0x100000001b3;
    for u in s.encode_utf16() {
        h ^= u as u64;
        h = h.wrapping_mul(prime);
    }
    format!("{h:016x}")
}

fn compute_config_fingerprint_from_artifact(artifact: &Value) -> String {
    let mut run_req = artifact.pointer("/run/req").cloned().unwrap_or(Value::Null);
    if let Value::Object(map) = &mut run_req {
        map.remove("data_id");
        map.remove("dataId");
    }

    let mut val_req = artifact.pointer("/validation/req").cloned().unwrap_or(Value::Null);
    if let Value::Object(map) = &mut val_req {
        map.remove("data_id");
        map.remove("dataId");
        map.remove("candidates");

        if let Some(cross) = map.get_mut("cross_ticker").and_then(|v| v.as_object_mut()) {
            if let Some(ds) = cross.get_mut("datasets").and_then(|v| v.as_array_mut()) {
                let mut out: Vec<Value> = Vec::with_capacity(ds.len());
                for d in ds.iter() {
                    let p = d.get("csv_path").and_then(|x| x.as_str()).unwrap_or("").trim();
                    if p.is_empty() {
                        continue;
                    }
                    out.push(serde_json::json!({ "csv_path": p }));
                }
                out.sort_by(|a, b| {
                    let aa = a.get("csv_path").and_then(|x| x.as_str()).unwrap_or("");
                    let bb = b.get("csv_path").and_then(|x| x.as_str()).unwrap_or("");
                    aa.cmp(bb)
                });
                *ds = out;
            }
        }
    }

    let payload = serde_json::json!({
        "version": 1,
        "run_req": run_req,
        "validation_req": val_req,
    });
    let canon = canonical_json(&payload);
    let hex = fnv1a64_hex_utf16(&canon);
    format!("cfg1-fnv1a64-{hex}")
}

fn list_run_artifacts_blocking(app: tauri::AppHandle) -> Result<Vec<RunArtifactSummary>, String> {
    let dir = runs_dir(&app)?;
    let mut out: Vec<RunArtifactSummary> = Vec::new();

    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with("vectorbt_meta_") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let bytes = meta.len();

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let ts = v.get("ts").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let kind = v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if kind != "vectorbt_run_artifact" {
            continue;
        }
        let strategy_kind = v
            .pointer("/run/req/strategy_kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let config_fingerprint = v
            .get("config_fingerprint")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| compute_config_fingerprint_from_artifact(&v));
        let has_validation = v.pointer("/validation/res").is_some();
        let report_xlsx_path = v
            .get("report_xlsx_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let csv_path = v
            .pointer("/data/csv_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let tags: Vec<String> = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map(|xs| {
                xs.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        out.push(RunArtifactSummary {
            ts,
            path: path.to_string_lossy().to_string(),
            bytes,
            kind,
            strategy_kind,
            config_fingerprint,
            has_validation,
            report_xlsx_path,
            csv_path,
            tags,
        });
    }

    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

fn list_meta_artifacts_blocking(app: tauri::AppHandle) -> Result<Vec<MetaArtifactSummary>, String> {
    let dir = runs_dir(&app)?;
    let mut out: Vec<MetaArtifactSummary> = Vec::new();

    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("vectorbt_meta_") {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let bytes = meta.len();

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let kind = v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if kind != "vectorbt_meta_selection" {
            continue;
        }

        let ts = v.get("ts").and_then(|x| x.as_str()).unwrap_or("").to_string();

        let strategy_kind = v
            .pointer("/req/strategy_kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let timeframe = v
            .pointer("/req/timeframe")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let required_tag = v
            .pointer("/req/required_tag")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let groups = v
            .pointer("/res/groups")
            .and_then(|x| x.as_array())
            .map(|xs| xs.len())
            .unwrap_or(0);
        let artifacts_used = v
            .pointer("/res/artifacts_used")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        let holdout_datasets = v
            .pointer("/res/holdout/datasets")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        let report_xlsx_path = v
            .get("report_xlsx_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let tags: Vec<String> = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map(|xs| {
                xs.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        out.push(MetaArtifactSummary {
            ts,
            path: path.to_string_lossy().to_string(),
            bytes,
            strategy_kind,
            timeframe,
            required_tag,
            groups,
            artifacts_used,
            holdout_datasets,
            report_xlsx_path,
            tags,
        });
    }

    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

fn load_run_artifact_blocking(path: String, app: tauri::AppHandle) -> Result<Value, String> {
    let dir = runs_dir(&app)?;
    let path = std::path::PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn delete_run_artifact_blocking(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = std::path::PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

fn update_artifact_tags_blocking(path: String, tags: Vec<String>, app: tauri::AppHandle) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = std::path::PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    if kind != "vectorbt_run_artifact" && kind != "vectorbt_meta_selection" {
        return Err(format!("unsupported artifact kind: {kind}"));
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for t in tags {
        let tt = t.trim();
        if tt.is_empty() {
            continue;
        }
        let key = tt.to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(tt.to_string());
    }

    let tags_v = serde_json::to_value(out).map_err(|e| e.to_string())?;
    if let Value::Object(obj) = &mut v {
        obj.insert("tags".to_string(), tags_v);
    }

    let bytes = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())
}

fn reveal_path_blocking(path: String) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.to_string_lossy()));
    }

    #[cfg(target_os = "windows")]
    {
        if path.is_dir() {
            Command::new("explorer")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            let arg = format!("/select,\"{}\"", path.to_string_lossy());
            Command::new("explorer").arg(arg).spawn().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        if path.is_dir() {
            Command::new("open")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("open")
                .arg("-R")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf()
        };
        Command::new("xdg-open")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn open_path_blocking(path: String) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.to_string_lossy()));
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn sanitize_filename_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => out.push(ch),
            ':' => out.push('-'),
            _ => out.push('_'),
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        // Avoid excessively long filenames on Windows.
        out.chars().take(96).collect()
    }
}

fn suggest_report_path(report: &Value, app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ts = report
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorbt_report_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.xlsx"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.xlsx"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn build_report_xlsx_blocking(report: Value) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = build_report_xlsx_bytes(&report, None)?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn build_validation_shortlist_csv_blocking(validation_res: Value) -> Result<String, String> {
    build_validation_shortlist_csv_text(&validation_res)
}

fn build_validation_shortlist_csv_text(validation_res: &Value) -> Result<String, String> {
    const CSCV_RANK_FRAC_MIN: f64 = 0.55;

    let candidates = validation_res
        .get("candidates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing validation_res.candidates".to_string())?;

    let has_cross = candidates
        .iter()
        .any(|c| c.get("cross_ticker").and_then(|v| v.as_object()).is_some());

    let strict: Vec<&Value> = candidates
        .iter()
        .filter(|cand| {
            let robust = cand.get("robust_pareto").and_then(|v| v.as_bool()).unwrap_or(false);
            if !robust {
                return false;
            }
            if !has_cross {
                return true;
            }
            cand.get("cross_ticker_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();

    let cscv = validation_res.get("cscv_pbo").and_then(|v| v.as_object());
    let cscv_threshold = cscv
        .and_then(|o| o.get("threshold").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let cscv_combos = cscv
        .and_then(|o| o.get("combinations").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let has_cscv = cscv_combos > 0
        && candidates
            .iter()
            .any(|c| c.get("cscv_pbo").and_then(|v| v.as_object()).is_some());

    let base_rows: Vec<&Value> = if strict.is_empty() {
        candidates.iter().collect()
    } else {
        strict
    };

    let mut rows: Vec<&Value> = if has_cscv {
        let filtered: Vec<&Value> = base_rows
            .iter()
            .copied()
            .filter(|cand| {
                let st = cand.get("cscv_pbo").and_then(|v| v.as_object());
                let sel = st
                    .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if sel <= 0.0 {
                    return false;
                }
                let rank = st
                    .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if rank < CSCV_RANK_FRAC_MIN {
                    return false;
                }
                let metric = st
                    .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(f64::NEG_INFINITY);
                metric >= cscv_threshold
            })
            .collect();
        if filtered.is_empty() {
            base_rows
        } else {
            filtered
        }
    } else {
        base_rows
    };

    if rows.is_empty() {
        return Err("validation_res has no candidates".to_string());
    }

    let mut lines: Vec<String> = Vec::with_capacity(rows.len().saturating_add(1));
    lines.push("rank,fast_ma,fast_len,slow_ma,slow_len,robust_score,robust_pareto,cross_score,cross_pareto,cscv_selected_frac,cscv_oos_rank_frac_p50,cscv_oos_sharpe_p50,in_sample_pnl,in_sample_sharpe,in_sample_max_dd,in_sample_trades,params_json".to_string());

    rows.sort_by(|a, b| {
        let ra = a.get("robust_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let rb = b.get("robust_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sa = a.get("cscv_pbo").and_then(|v| v.as_object());
        let sb = b.get("cscv_pbo").and_then(|v| v.as_object());
        let a_rank = sa
            .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_rank = sb
            .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let a_sel = sa
            .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_sel = sb
            .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let a_m = sa
            .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_m = sb
            .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        rb.total_cmp(&ra)
            .then_with(|| b_rank.total_cmp(&a_rank))
            .then_with(|| b_sel.total_cmp(&a_sel))
            .then_with(|| b_m.total_cmp(&a_m))
    });

    for (i, cand) in rows.into_iter().enumerate() {
        let params = cand.get("params").and_then(|v| v.as_object());
        let in_sample = cand.get("in_sample").and_then(|v| v.as_object());
        let cross = cand.get("cross_ticker").and_then(|v| v.as_object());
        let cscv = cand.get("cscv_pbo").and_then(|v| v.as_object());

        let fast_ma = params
            .and_then(|p| p.get("fast_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let fast_len = params
            .and_then(|p| p.get("fast_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let slow_ma = params
            .and_then(|p| p.get("slow_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let slow_len = params
            .and_then(|p| p.get("slow_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let robust_score = cand.get("robust_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let robust_pareto = cand
            .get("robust_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cross_score = cross.and_then(|c| c.get("score").and_then(|v| v.as_f64()));
        let cross_pareto = cand
            .get("cross_ticker_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cscv_sel = cscv.and_then(|c| c.get("is_selected_frac").and_then(|v| v.as_f64()));
        let cscv_rank = cscv.and_then(|c| c.get("oos_rank_frac_p50").and_then(|v| v.as_f64()));
        let cscv_sharpe =
            cscv.and_then(|c| c.get("oos_metric_p50").and_then(|v| v.as_f64()));

        let pnl = in_sample
            .and_then(|m| m.get("pnl").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let sharpe = in_sample
            .and_then(|m| m.get("sharpe").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let max_dd = in_sample
            .and_then(|m| m.get("max_dd").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let trades = in_sample
            .and_then(|m| m.get("trades").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let params_json = cand.get("params").cloned().unwrap_or(Value::Null);
        let params_json_str = serde_json::to_string(&params_json).unwrap_or_else(|_| "{}".to_string());

        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            i.saturating_add(1),
            csv_escape(fast_ma),
            fast_len,
            csv_escape(slow_ma),
            slow_len,
            robust_score,
            if robust_pareto { 1 } else { 0 },
            cross_score
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            if cross_pareto { 1 } else { 0 },
            cscv_sel.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
            cscv_rank.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
            cscv_sharpe.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
            pnl,
            sharpe,
            max_dd,
            trades,
            csv_escape(&params_json_str),
        ));
    }

    Ok(lines.join("\n"))
}

fn save_report_xlsx_blocking(
    report: Value,
    max_rows: Option<u32>,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    let max_rows = max_rows
        .map(|n| n as usize)
        .filter(|n| *n > 0);
    let bytes = build_report_xlsx_bytes(&report, max_rows)?;
    let path = suggest_report_path(&report, &app)?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(SaveReportXlsxResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn build_report_xlsx_bytes(report: &Value, max_rows: Option<usize>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{
        Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartSolidFill, ChartType, Color,
        ConditionalFormat3ColorScale, ConditionalFormatDataBar, ConditionalFormatFormula, Format,
        FormatAlign, FormatBorder, Workbook, XlsxError,
    };

    const CSCV_RANK_FRAC_MIN: f64 = 0.55;

    let ts = report
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let validation_res = report
        .pointer("/validation/res")
        .ok_or_else(|| "missing report.validation.res".to_string())?;

    let data_id = validation_res
        .get("data_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let candidates = validation_res
        .get("candidates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing validation.res.candidates".to_string())?;

    let has_cross = candidates
        .iter()
        .any(|c| c.get("cross_ticker").and_then(|v| v.as_object()).is_some());

    let cscv = validation_res.get("cscv_pbo").and_then(|v| v.as_object());
    let cscv_threshold = cscv
        .and_then(|o| o.get("threshold").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let cscv_combos = cscv
        .and_then(|o| o.get("combinations").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let has_cscv = cscv_combos > 0
        && candidates
            .iter()
            .any(|c| c.get("cscv_pbo").and_then(|v| v.as_object()).is_some());

    // Match the frontend shortlist rule:
    // - If cross-ticker was run, shortlist = robust_pareto && cross_ticker_pareto
    // - Otherwise shortlist = robust_pareto
    // - If shortlist is empty, fall back to all candidates.
    let shortlist_base: Vec<&Value> = candidates
        .iter()
        .filter(|cand| {
            let robust = cand
                .get("robust_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !robust {
                return false;
            }
            if !has_cross {
                return true;
            }
            cand.get("cross_ticker_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();

    let base_rows: Vec<&Value> = if shortlist_base.is_empty() {
        candidates.iter().collect()
    } else {
        shortlist_base
    };

    let mut all_rows: Vec<&Value> = if has_cscv {
        let filtered: Vec<&Value> = base_rows
            .iter()
            .copied()
            .filter(|cand| {
                let st = cand.get("cscv_pbo").and_then(|v| v.as_object());
                let sel = st
                    .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if sel <= 0.0 {
                    return false;
                }
                let rank = st
                    .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if rank < CSCV_RANK_FRAC_MIN {
                    return false;
                }
                let metric = st
                    .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(f64::NEG_INFINITY);
                metric >= cscv_threshold
            })
            .collect();
        if filtered.is_empty() {
            base_rows
        } else {
            filtered
        }
    } else {
        base_rows
    };

    all_rows.sort_by(|a, b| {
        let ra = a.get("robust_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let rb = b.get("robust_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sa = a.get("cscv_pbo").and_then(|v| v.as_object());
        let sb = b.get("cscv_pbo").and_then(|v| v.as_object());
        let a_rank = sa
            .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_rank = sb
            .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let a_sel = sa
            .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_sel = sb
            .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let a_m = sa
            .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_m = sb
            .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        rb.total_cmp(&ra)
            .then_with(|| b_rank.total_cmp(&a_rank))
            .then_with(|| b_sel.total_cmp(&a_sel))
            .then_with(|| b_m.total_cmp(&a_m))
    });

    // Keep a full copy for distribution charts, even if the table is capped by `max_rows`.
    let mut rows: Vec<&Value> = all_rows.clone();
    if let Some(n) = max_rows {
        if rows.len() > n {
            rows.truncate(n);
        }
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .set_name("Shortlist")
        .map_err(|e: XlsxError| e.to_string())?;

    let title_fmt = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);

    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0B1220))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    let num6_fmt = Format::new().set_num_format("0.000000");
    let int_fmt = Format::new().set_num_format("0");

    let ok_badge = Format::new()
        .set_font_color(Color::RGB(0x10B981))
        .set_background_color(Color::RGB(0x06181A))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let bad_badge = Format::new()
        .set_font_color(Color::RGB(0xFB7185))
        .set_background_color(Color::RGB(0x1D0B12))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    // Title + meta.
    worksheet
        .merge_range(0, 0, 0, 45, "VectorTA Report", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(1, 0, "ts")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 1, &ts)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(2, 0, "data_id")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(2, 1, &data_id)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_url_with_text(1, 3, "internal:Dashboard!A1", "Dashboard")
        .map_err(|e: XlsxError| e.to_string())?;

    // Table headers.
    let header_row: u32 = 4;
    let headers: Vec<&str> = vec![
        "Rank",
        "Fast MA",
        "Fast Len",
        "Slow MA",
        "Slow Len",
        "Robust Score",
        "Robust Pareto",
        "Cross Score",
        "Cross Pareto",
        "PnL",
        "Sharpe",
        "MaxDD",
        "Trades",
        "HT amp",
        "HT atr",
        "HT ch_dev",
        "MACZ fast",
        "MACZ slow",
        "MACZ sig",
        "MACZ z",
        "MACZ stdev",
        "MACZ a",
        "MACZ b",
        "MACZ gamma",
        "MACZ lag",
        "MACZ vol",
        "OTT period",
        "OTT %",
        "OTT MA",
        "Commission",
        "EpsRel",
        "LongOnly",
        "AllowFlip",
        "TradeNext",
        "params_json",
        "WF sharpe_med",
        "WF sharpe_worst",
        "WF dd_worst",
        "MC prob_pnl_pos",
        "MC pnl_p50",
        "MC dd_p95",
        "Cross sharpe_med",
        "Cross dd_worst",
        "CSCV sel_frac",
        "CSCV oos_rank_p50",
        "CSCV oos_sharpe_p50",
    ];
    for (col, name) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(header_row, col as u16, *name, &header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Column widths for readability.
    let _ = worksheet.set_column_width(0, 6);
    let _ = worksheet.set_column_width(1, 14);
    let _ = worksheet.set_column_width(2, 9);
    let _ = worksheet.set_column_width(3, 14);
    let _ = worksheet.set_column_width(4, 9);
    let _ = worksheet.set_column_width(5, 14);
    let _ = worksheet.set_column_width(6, 14);
    let _ = worksheet.set_column_width(7, 14);
    let _ = worksheet.set_column_width(8, 14);
    let _ = worksheet.set_column_width(9, 12);
    let _ = worksheet.set_column_width(10, 12);
    let _ = worksheet.set_column_width(11, 12);
    let _ = worksheet.set_column_width(12, 10);
    let _ = worksheet.set_column_width(13, 9);
    let _ = worksheet.set_column_width(14, 9);
    let _ = worksheet.set_column_width(15, 10);
    let _ = worksheet.set_column_width(16, 9);
    let _ = worksheet.set_column_width(17, 9);
    let _ = worksheet.set_column_width(18, 9);
    let _ = worksheet.set_column_width(19, 9);
    let _ = worksheet.set_column_width(20, 11);
    let _ = worksheet.set_column_width(21, 9);
    let _ = worksheet.set_column_width(22, 9);
    let _ = worksheet.set_column_width(23, 10);
    let _ = worksheet.set_column_width(24, 9);
    let _ = worksheet.set_column_width(25, 9);
    let _ = worksheet.set_column_width(26, 10);
    let _ = worksheet.set_column_width(27, 9);
    let _ = worksheet.set_column_width(28, 10);
    let _ = worksheet.set_column_width(29, 11);
    let _ = worksheet.set_column_width(30, 9);
    let _ = worksheet.set_column_width(31, 10);
    let _ = worksheet.set_column_width(32, 10);
    let _ = worksheet.set_column_width(33, 10);
    let _ = worksheet.set_column_width(34, 60);
    let _ = worksheet.set_column_width(35, 14);
    let _ = worksheet.set_column_width(36, 15);
    let _ = worksheet.set_column_width(37, 12);
    let _ = worksheet.set_column_width(38, 15);
    let _ = worksheet.set_column_width(39, 12);
    let _ = worksheet.set_column_width(40, 12);
    let _ = worksheet.set_column_width(41, 16);
    let _ = worksheet.set_column_width(42, 14);
    let _ = worksheet.set_column_width(43, 14);
    let _ = worksheet.set_column_width(44, 16);
    let _ = worksheet.set_column_width(45, 18);

    let start_row = header_row + 1;
    worksheet
        .set_freeze_panes(start_row, 1)
        .map_err(|e: XlsxError| e.to_string())?;

    for (i, cand) in rows.iter().enumerate() {
        let r = start_row + i as u32;
        let cand = *cand;

        let params = cand.get("params").unwrap_or(&Value::Null);
        let metrics = cand.get("in_sample").unwrap_or(&Value::Null);

        let fast_ma = params
            .get("fast_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let slow_ma = params
            .get("slow_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fast_len = params
            .get("fast_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let slow_len = params
            .get("slow_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let robust_pareto = cand
            .get("robust_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cross_score = cand
            .get("cross_ticker")
            .and_then(|v| v.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cross_pareto = cand
            .get("cross_ticker_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let pnl = metrics.get("pnl").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sharpe = metrics
            .get("sharpe")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let max_dd = metrics
            .get("max_dd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let trades = metrics
            .get("trades")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let halftrend = params.get("halftrend").unwrap_or(&Value::Null);
        let ht_amp = halftrend
            .get("amplitude")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let ht_atr = halftrend
            .get("atr_period")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let ht_dev = halftrend
            .get("channel_deviation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let macz = params.get("macz").unwrap_or(&Value::Null);
        let macz_fast = macz
            .get("fast_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_slow = macz
            .get("slow_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_sig = macz
            .get("signal_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_z = macz
            .get("lengthz")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_stdev = macz
            .get("length_stdev")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_a = macz.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let macz_b = macz.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let macz_gamma = macz
            .get("gamma")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let macz_lag = macz
            .get("use_lag")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let macz_vol = macz
            .get("use_volume")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let ott = params.get("ott").unwrap_or(&Value::Null);
        let ott_period = ott
            .get("period")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let ott_pct = ott
            .get("percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let ott_ma = ott.get("ma_type").and_then(|v| v.as_str()).unwrap_or("");

        let strategy = params.get("strategy").unwrap_or(&Value::Null);
        let commission = strategy
            .get("commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let eps_rel = strategy
            .get("eps_rel")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let long_only = strategy
            .get("long_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let allow_flip = strategy
            .get("allow_flip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let trade_next = strategy
            .get("trade_on_next_bar")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string());

        let wf = cand.get("walk_forward").unwrap_or(&Value::Null);
        let wf_sharpe_med = wf
            .get("sharpe_median")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wf_sharpe_worst = wf
            .get("sharpe_worst")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wf_dd_worst = wf
            .get("max_dd_worst")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mc = cand.get("trade_bootstrap").unwrap_or(&Value::Null);
        let mc_prob = mc
            .get("prob_pnl_positive")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let mc_p50 = mc.get("pnl_p50").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mc_dd_p95 = mc
            .get("max_dd_p95")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let cross = cand.get("cross_ticker").unwrap_or(&Value::Null);
        let cross_sharpe_med = cross
            .get("sharpe_median")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cross_dd_worst = cross
            .get("max_dd_worst")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let cscv = cand.get("cscv_pbo").unwrap_or(&Value::Null);
        let cscv_sel = cscv
            .get("is_selected_frac")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cscv_rank = cscv
            .get("oos_rank_frac_p50")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cscv_sharpe = cscv
            .get("oos_metric_p50")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        worksheet
            .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 1, fast_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 2, fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 3, slow_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 4, slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 5, robust_score, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string_with_format(
                r,
                6,
                if robust_pareto { "yes" } else { "no" },
                if robust_pareto { &ok_badge } else { &bad_badge },
            )
            .map_err(|e: XlsxError| e.to_string())?;

        if has_cross {
            worksheet
                .write_number_with_format(r, 7, cross_score, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string_with_format(
                    r,
                    8,
                    if cross_pareto { "yes" } else { "no" },
                    if cross_pareto { &ok_badge } else { &bad_badge },
                )
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            worksheet
                .write_string_with_format(r, 7, "-", &bad_badge)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string_with_format(r, 8, "-", &bad_badge)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        worksheet
            .write_number_with_format(r, 9, pnl, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 10, sharpe, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 11, max_dd, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 12, trades as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 13, ht_amp as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 14, ht_atr as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 15, ht_dev, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 16, macz_fast as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 17, macz_slow as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 18, macz_sig as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 19, macz_z as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 20, macz_stdev as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 21, macz_a, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 22, macz_b, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 23, macz_gamma, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 24, if macz_lag { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 25, if macz_vol { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 26, ott_period as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 27, ott_pct, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 28, ott_ma)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 29, commission, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 30, eps_rel, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 31, if long_only { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 32, if allow_flip { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 33, if trade_next { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_string(r, 34, &params_json)
            .map_err(|e: XlsxError| e.to_string())?;

        if wf.is_object() {
            worksheet
                .write_number_with_format(r, 35, wf_sharpe_med, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 36, wf_sharpe_worst, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 37, wf_dd_worst, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if mc.is_object() {
            worksheet
                .write_number_with_format(r, 38, mc_prob, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 39, mc_p50, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 40, mc_dd_p95, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if cross.is_object() {
            worksheet
                .write_number_with_format(r, 41, cross_sharpe_med, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 42, cross_dd_worst, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if cscv.is_object() {
            worksheet
                .write_number_with_format(r, 43, cscv_sel, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 44, cscv_rank, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 45, cscv_sharpe, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    if !rows.is_empty() {
        let row_min = start_row;
        let row_max = start_row + rows.len().saturating_sub(1) as u32;
        let last_col = headers.len().saturating_sub(1) as u16;

        worksheet
            .autofilter(header_row, 0, row_max, last_col)
            .map_err(|e: XlsxError| e.to_string())?;

        let cf_bar_green = ConditionalFormatDataBar::new().set_fill_color("63BE7B");
        let cf_bar_blue = ConditionalFormatDataBar::new().set_fill_color("5A8AC6");

        let cf_good = ConditionalFormat3ColorScale::new()
            .set_minimum_color("F8696B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("63BE7B");
        let cf_bad = ConditionalFormat3ColorScale::new()
            .set_minimum_color("63BE7B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("F8696B");

        // Key metric heatmaps for quick visual scanning.
        worksheet
            .add_conditional_format(row_min, 5, row_max, 5, &cf_bar_green)
            .map_err(|e: XlsxError| e.to_string())?;
        if has_cross {
            worksheet
                .add_conditional_format(row_min, 7, row_max, 7, &cf_bar_blue)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        worksheet
            .add_conditional_format(row_min, 9, row_max, 9, &cf_good)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 10, row_max, 10, &cf_good)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 11, row_max, 11, &cf_bad)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 38, row_max, 38, &cf_bar_green)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 40, row_max, 40, &cf_bad)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 42, row_max, 42, &cf_bad)
            .map_err(|e: XlsxError| e.to_string())?;

        // Row/flag highlighting (recommended + Pareto).
        let excel_first_row = row_min + 1;
        let rec_row_fmt = Format::new()
            .set_bold()
            .set_border_top(FormatBorder::Medium)
            .set_border_bottom(FormatBorder::Medium);
        let rec_row_rule = format!("=$A{}=1", excel_first_row);
        let rec_row_cf = ConditionalFormatFormula::new()
            .set_rule(rec_row_rule.as_str())
            .set_format(&rec_row_fmt);
        worksheet
            .add_conditional_format(row_min, 0, row_max, last_col, &rec_row_cf)
            .map_err(|e: XlsxError| e.to_string())?;

        let rank_rec_fmt = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_font_color(Color::RGB(0x0B1220))
            .set_background_color(Color::RGB(0xFFEB84))
            .set_border(FormatBorder::Thin);
        let rank_pareto_fmt = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_font_color(Color::RGB(0x0B1220))
            .set_background_color(Color::RGB(0x63BE7B))
            .set_border(FormatBorder::Thin);

        let rank_rec_rule = format!("=$A{}=1", excel_first_row);
        let rank_rec_cf = ConditionalFormatFormula::new()
            .set_rule(rank_rec_rule.as_str())
            .set_format(&rank_rec_fmt)
            .set_stop_if_true(true);
        worksheet
            .add_conditional_format(row_min, 0, row_max, 0, &rank_rec_cf)
            .map_err(|e: XlsxError| e.to_string())?;

        let pareto_rule = if has_cross {
            format!(
                "=AND($G{}=\"yes\",$I{}=\"yes\")",
                excel_first_row, excel_first_row
            )
        } else {
            format!("=$G{}=\"yes\"", excel_first_row)
        };
        let rank_pareto_cf = ConditionalFormatFormula::new()
            .set_rule(pareto_rule.as_str())
            .set_format(&rank_pareto_fmt);
        worksheet
            .add_conditional_format(row_min, 0, row_max, 0, &rank_pareto_cf)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let chart_range = if rows.is_empty() {
        None
    } else {
        Some((start_row, start_row + rows.len().saturating_sub(1) as u32))
    };
    let recommended = rows.first().copied();

    let _ = worksheet;

    // Dashboard summary (active sheet on open).
    let dashboard = workbook.add_worksheet();
    dashboard
        .set_name("Dashboard")
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard.set_active(true);
    dashboard.set_zoom(110);

    let label_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0B1220))
        .set_border(FormatBorder::Thin);
    let value_fmt = Format::new().set_border(FormatBorder::Thin);

    let _ = dashboard.set_column_width(0, 24);
    let _ = dashboard.set_column_width(1, 42);
    let _ = dashboard.set_column_width(2, 24);
    let _ = dashboard.set_column_width(3, 18);

    dashboard
        .merge_range(0, 0, 0, 7, "VectorTA Report", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    dashboard
        .write_string_with_format(1, 0, "ts", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(1, 1, &ts, &value_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(2, 0, "data_id", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(2, 1, &data_id, &value_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(cscv) = validation_res.get("cscv_pbo").and_then(|v| v.as_object()) {
        let combos = cscv
            .get("combinations")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let pbo = cscv.get("pbo").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let skipped = cscv
            .get("skipped_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let text = if !skipped.is_empty() {
            format!("skipped: {skipped}")
        } else if combos > 0 {
            format!("pbo={:.3} (cs={})", pbo, combos)
        } else {
            "not available".to_string()
        };

        dashboard
            .write_string_with_format(3, 0, "CSCV/PBO", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(3, 1, &text, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(4, 0, "Recommended", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(cand) = recommended {
        let params = cand.get("params").unwrap_or(&Value::Null);
        let metrics = cand.get("in_sample").unwrap_or(&Value::Null);

        let fast_ma = params
            .get("fast_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let slow_ma = params
            .get("slow_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fast_len = params
            .get("fast_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let slow_len = params
            .get("slow_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cross_score = cand
            .get("cross_ticker")
            .and_then(|v| v.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let pnl = metrics.get("pnl").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sharpe = metrics
            .get("sharpe")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let max_dd = metrics
            .get("max_dd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let trades = metrics
            .get("trades")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string());

        dashboard
            .write_string_with_format(5, 0, "Fast MA", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(5, 1, fast_ma, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(5, 2, "Fast Len", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(5, 3, fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(6, 0, "Slow MA", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(6, 1, slow_ma, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(6, 2, "Slow Len", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(6, 3, slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(7, 0, "Robust score", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(7, 1, robust_score, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(7, 2, "Cross score", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        if has_cross {
            dashboard
                .write_number_with_format(7, 3, cross_score, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            dashboard
                .write_string_with_format(7, 3, "-", &value_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        dashboard
            .write_string_with_format(8, 0, "PnL", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(8, 1, pnl, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(8, 2, "Trades", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(8, 3, trades as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(9, 0, "Sharpe", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(9, 1, sharpe, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(9, 2, "MaxDD", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(9, 3, max_dd, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(11, 0, "params_json", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(11, 1, &params_json, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if let Some((row_min, row_max)) = chart_range {
        let excel_first_row = row_min + 1;
        let shortlist_link = format!("internal:Shortlist!A{}", excel_first_row);
        let _ = dashboard.write_url_with_text(4, 1, shortlist_link.as_str(), "Go to Shortlist");

        // Robust score chart.
        let col_rank = 0;
        let col_robust = 5;
        let mut score_chart = Chart::new(ChartType::Column);
        score_chart
            .add_series()
            .set_categories(("Shortlist", row_min, col_rank, row_max, col_rank))
            .set_values(("Shortlist", row_min, col_robust, row_max, col_robust));
        score_chart.title().set_name("Robust score by rank");
        score_chart.x_axis().set_name("Rank");
        score_chart.y_axis().set_name("Robust score");

        if has_cross {
            let col_cross = 7;
            score_chart
                .add_series()
                .set_categories(("Shortlist", row_min, col_rank, row_max, col_rank))
                .set_values(("Shortlist", row_min, col_cross, row_max, col_cross));
            score_chart
                .legend()
                .set_position(rust_xlsxwriter::ChartLegendPosition::Bottom);
        } else {
            score_chart.legend().set_hidden();
        }

        dashboard
            .insert_chart_with_offset(4, 5, &score_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;

        // Risk/return scatter: Sharpe vs MaxDD (best in top-left).
        let col_sharpe = 10;
        let col_max_dd = 11;
        let mut rr_chart = Chart::new(ChartType::Scatter);
        rr_chart
            .add_series()
            .set_categories(("Shortlist", row_min, col_max_dd, row_max, col_max_dd))
            .set_values(("Shortlist", row_min, col_sharpe, row_max, col_sharpe));
        rr_chart.title().set_name("Sharpe vs MaxDD");
        rr_chart.x_axis().set_name("MaxDD");
        rr_chart.y_axis().set_name("Sharpe");
        rr_chart.legend().set_hidden();

        dashboard
            .insert_chart_with_offset(20, 0, &rr_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // ---------------------------------------------------------------------
    // Distributions (derived from the full shortlist before `max_rows` cap).
    // ---------------------------------------------------------------------
    use std::collections::BTreeMap;

    let mut ma_pair_stats: BTreeMap<(String, String), (usize, f64)> = BTreeMap::new();
    let mut fast_len_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut slow_len_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut points: Vec<(u32, u32, f64)> = Vec::with_capacity(all_rows.len());

    for cand in all_rows.iter() {
        let params = cand.get("params").unwrap_or(&Value::Null);
        let fast_ma = params
            .get("fast_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let slow_ma = params
            .get("slow_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fast_len = params
            .get("fast_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let slow_len = params
            .get("slow_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if !fast_ma.is_empty() && !slow_ma.is_empty() {
            let entry = ma_pair_stats
                .entry((fast_ma.clone(), slow_ma.clone()))
                .or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += robust_score;
        }
        if fast_len > 0 {
            *fast_len_counts.entry(fast_len).or_insert(0) += 1;
        }
        if slow_len > 0 {
            *slow_len_counts.entry(slow_len).or_insert(0) += 1;
        }
        if fast_len > 0 && slow_len > 0 {
            points.push((slow_len, fast_len, robust_score));
        }
    }

    let mut pair_rows: Vec<(String, usize, f64)> = ma_pair_stats
        .into_iter()
        .map(|((fast, slow), (count, sum))| {
            let avg = if count > 0 { sum / count as f64 } else { 0.0 };
            (format!("{fast}/{slow}"), count, avg)
        })
        .collect();
    pair_rows.sort_by(|a, b| b.1.cmp(&a.1));

    let mut scores: Vec<f64> = points.iter().map(|(_, _, s)| *s).collect();
    scores.sort_by(|a, b| a.total_cmp(b));
    let q = |xs: &[f64], p: f64| -> f64 {
        if xs.is_empty() {
            return 0.0;
        }
        let idx = ((xs.len().saturating_sub(1) as f64) * p).round() as usize;
        xs[idx.min(xs.len() - 1)]
    };
    let q1 = q(&scores, 0.33);
    let q2 = q(&scores, 0.66);

    let mut low: Vec<(u32, u32)> = Vec::new();
    let mut mid: Vec<(u32, u32)> = Vec::new();
    let mut high: Vec<(u32, u32)> = Vec::new();
    for (slow_len, fast_len, score) in points {
        if score <= q1 {
            low.push((slow_len, fast_len));
        } else if score <= q2 {
            mid.push((slow_len, fast_len));
        } else {
            high.push((slow_len, fast_len));
        }
    }

    let dist = workbook.add_worksheet();
    dist.set_name("Distributions")
        .map_err(|e: XlsxError| e.to_string())?;
    dist.set_zoom(110);

    let _ = dist.set_column_width(0, 18);
    let _ = dist.set_column_width(1, 10);
    let _ = dist.set_column_width(2, 12);
    let _ = dist.set_column_width(4, 10);
    let _ = dist.set_column_width(5, 10);
    let _ = dist.set_column_width(7, 10);
    let _ = dist.set_column_width(8, 10);

    dist.merge_range(0, 0, 0, 12, "Distributions", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    dist.write_string_with_format(1, 0, "Candidates (full shortlist)", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(1, 1, all_rows.len() as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dist.write_url_with_text(1, 3, "internal:Dashboard!A1", "Dashboard");
    let _ = dist.write_url_with_text(1, 4, "internal:Shortlist!A1", "Shortlist");

    // MA pair frequency table.
    let pair_header_row: u32 = 3;
    dist.write_string_with_format(pair_header_row, 0, "MA Pair", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 1, "Count", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 2, "Avg robust", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    for (i, (pair, count, avg)) in pair_rows.iter().enumerate() {
        let r = pair_header_row + 1 + i as u32;
        dist.write_string(r, 0, pair)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 1, *count as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 2, *avg, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Length count tables.
    dist.write_string_with_format(pair_header_row, 4, "Fast Len", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 5, "Count", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, (len, count)) in fast_len_counts.iter().enumerate() {
        let r = pair_header_row + 1 + i as u32;
        dist.write_number_with_format(r, 4, *len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 5, *count as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dist.write_string_with_format(pair_header_row, 7, "Slow Len", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 8, "Count", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, (len, count)) in slow_len_counts.iter().enumerate() {
        let r = pair_header_row + 1 + i as u32;
        dist.write_number_with_format(r, 7, *len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 8, *count as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Charts.
    if !pair_rows.is_empty() {
        let row_min = pair_header_row + 1;
        let row_max = pair_header_row + pair_rows.len() as u32;
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_categories(("Distributions", row_min, 0, row_max, 0))
            .set_values(("Distributions", row_min, 1, row_max, 1));
        chart.title().set_name("MA pair counts");
        chart.x_axis().set_name("Count");
        chart.y_axis().set_name("MA pair");
        chart.legend().set_hidden();
        dist.insert_chart_with_offset(3, 10, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !fast_len_counts.is_empty() {
        let row_min = pair_header_row + 1;
        let row_max = pair_header_row + fast_len_counts.len() as u32;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_categories(("Distributions", row_min, 4, row_max, 4))
            .set_values(("Distributions", row_min, 5, row_max, 5));
        chart.title().set_name("Fast length counts");
        chart.x_axis().set_name("Fast Len");
        chart.y_axis().set_name("Count");
        chart.legend().set_hidden();
        dist.insert_chart_with_offset(20, 10, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !slow_len_counts.is_empty() {
        let row_min = pair_header_row + 1;
        let row_max = pair_header_row + slow_len_counts.len() as u32;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_categories(("Distributions", row_min, 7, row_max, 7))
            .set_values(("Distributions", row_min, 8, row_max, 8));
        chart.title().set_name("Slow length counts");
        chart.x_axis().set_name("Slow Len");
        chart.y_axis().set_name("Count");
        chart.legend().set_hidden();
        dist.insert_chart_with_offset(37, 10, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Fast vs slow scatter with robust buckets (low/mid/high).
    let scatter_header_row: u32 = 20;
    dist.write_string_with_format(scatter_header_row, 0, "slow_len_low", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 1, "fast_len_low", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 2, "slow_len_mid", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 3, "fast_len_mid", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 4, "slow_len_high", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 5, "fast_len_high", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    let scatter_row_min = scatter_header_row + 1;
    for (i, (slow_len, fast_len)) in low.iter().enumerate() {
        let r = scatter_row_min + i as u32;
        dist.write_number_with_format(r, 0, *slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 1, *fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, (slow_len, fast_len)) in mid.iter().enumerate() {
        let r = scatter_row_min + i as u32;
        dist.write_number_with_format(r, 2, *slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 3, *fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, (slow_len, fast_len)) in high.iter().enumerate() {
        let r = scatter_row_min + i as u32;
        dist.write_number_with_format(r, 4, *slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 5, *fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !low.is_empty() || !mid.is_empty() || !high.is_empty() {
        let mut marker_low = ChartMarker::new();
        marker_low
            .set_type(ChartMarkerType::Circle)
            .set_size(6)
            .set_format(
                ChartFormat::new()
                    .set_solid_fill(ChartSolidFill::new().set_color("#F8696B")),
            );
        let mut marker_mid = ChartMarker::new();
        marker_mid
            .set_type(ChartMarkerType::Circle)
            .set_size(6)
            .set_format(
                ChartFormat::new()
                    .set_solid_fill(ChartSolidFill::new().set_color("#FFEB84")),
            );
        let mut marker_high = ChartMarker::new();
        marker_high
            .set_type(ChartMarkerType::Circle)
            .set_size(6)
            .set_format(
                ChartFormat::new()
                    .set_solid_fill(ChartSolidFill::new().set_color("#63BE7B")),
            );

        let mut chart = Chart::new(ChartType::Scatter);
        if !low.is_empty() {
            let row_max = scatter_row_min + low.len().saturating_sub(1) as u32;
            chart
                .add_series()
                .set_name("Low robust")
                .set_categories(("Distributions", scatter_row_min, 0, row_max, 0))
                .set_values(("Distributions", scatter_row_min, 1, row_max, 1))
                .set_marker(&marker_low);
        }
        if !mid.is_empty() {
            let row_max = scatter_row_min + mid.len().saturating_sub(1) as u32;
            chart
                .add_series()
                .set_name("Mid robust")
                .set_categories(("Distributions", scatter_row_min, 2, row_max, 2))
                .set_values(("Distributions", scatter_row_min, 3, row_max, 3))
                .set_marker(&marker_mid);
        }
        if !high.is_empty() {
            let row_max = scatter_row_min + high.len().saturating_sub(1) as u32;
            chart
                .add_series()
                .set_name("High robust")
                .set_categories(("Distributions", scatter_row_min, 4, row_max, 4))
                .set_values(("Distributions", scatter_row_min, 5, row_max, 5))
                .set_marker(&marker_high);
        }
        chart.title().set_name("Fast vs Slow length (bucketed by robust score)");
        chart.x_axis().set_name("Slow Len");
        chart.y_axis().set_name("Fast Len");
        chart.legend().set_position(rust_xlsxwriter::ChartLegendPosition::Bottom);

        dist.insert_chart_with_offset(20, 16, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e: XlsxError| e.to_string())
}

fn build_meta_report_xlsx_bytes(ts: &str, meta: &MetaSelectionResult) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{
        Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError,
    };

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .set_name("Meta")
        .map_err(|e: XlsxError| e.to_string())?;

    let title_fmt = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);

    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0B1220))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    let num6_fmt = Format::new().set_num_format("0.000000");
    let num3_fmt = Format::new().set_num_format("0.000");
    let pct_fmt = Format::new().set_num_format("0.0%");
    let int_fmt = Format::new().set_num_format("0");

    worksheet
        .merge_range(0, 0, 0, 22, "VectorTA Meta Report", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(1, 0, "ts")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 1, ts)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(2, 0, "strategy_kind")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(2, 1, &meta.strategy_kind)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(3, 0, "timeframe")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(3, 1, meta.timeframe.as_deref().unwrap_or(""))
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(h) = meta.holdout.as_ref() {
        worksheet
            .write_string(4, 0, "holdout_datasets")
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(4, 1, h.datasets as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(4, 3, "holdout_skipped")
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(4, 4, h.skipped as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    worksheet
        .write_string(1, 3, "artifacts_used")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(1, 4, meta.artifacts_used as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(2, 3, "candidates_considered")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(2, 4, meta.candidates_considered as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(3, 3, "runtime_ms")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(3, 4, meta.runtime_ms as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    let header_row: u32 = 5;
    let headers: [&str; 23] = [
        "Rank",
        "Samples",
        "Pass rate",
        "Meta score",
        "Robust p10",
        "Robust med",
        "MaxDD p90",
        "Cross med",
        "H pass",
        "H score p10",
        "H score med",
        "H DD p90",
        "Fast MA",
        "Fast Len",
        "Slow MA",
        "Slow Len",
        "HT amp",
        "HT atr",
        "HT ch_dev",
        "MACZ fast",
        "MACZ slow",
        "OTT period",
        "OTT pct",
    ];
    for (c, h) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(header_row, c as u16, *h, &header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    for (i, row) in meta.groups.iter().enumerate() {
        let r = header_row + 1 + (i as u32);

        let p = row.params.as_object();
        let fast_ma = p
            .and_then(|p| p.get("fast_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let slow_ma = p
            .and_then(|p| p.get("slow_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let fast_len = p
            .and_then(|p| p.get("fast_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let slow_len = p
            .and_then(|p| p.get("slow_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let ht = p.and_then(|p| p.get("halftrend").and_then(|v| v.as_object()));
        let ht_amp = ht
            .and_then(|h| h.get("amplitude").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let ht_atr = ht
            .and_then(|h| h.get("atr_period").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let ht_dev = ht
            .and_then(|h| h.get("channel_deviation").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        let macz = p.and_then(|p| p.get("macz").and_then(|v| v.as_object()));
        let macz_fast = macz
            .and_then(|m| m.get("fast_length").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let macz_slow = macz
            .and_then(|m| m.get("slow_length").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let ott = p.and_then(|p| p.get("ott").and_then(|v| v.as_object()));
        let ott_period = ott
            .and_then(|o| o.get("period").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let ott_pct = ott
            .and_then(|o| o.get("percent").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        worksheet
            .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 1, row.samples as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 2, row.pass_rate, &pct_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 3, row.meta_score, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 4, row.robust_score_p10, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 5, row.robust_score_median, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 6, row.max_dd_p90, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 7, row.cross_score_median, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        if let Some(h) = row.holdout.as_ref() {
            worksheet
                .write_number_with_format(r, 8, h.pass_rate, &pct_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 9, h.score_p10, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 10, h.score_median, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 11, h.max_dd_p90, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            worksheet
                .write_string(r, 8, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string(r, 9, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string(r, 10, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string(r, 11, "-")
                .map_err(|e: XlsxError| e.to_string())?;
        }

        worksheet
            .write_string(r, 12, fast_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 13, fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 14, slow_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 15, slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 16, ht_amp as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 17, ht_atr as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 18, ht_dev, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 19, macz_fast as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 20, macz_slow as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 21, ott_period as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 22, ott_pct, &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e: XlsxError| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_trending_candles(len: usize, step: f64) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

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

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn write_candles_csv(path: &Path, candles: &vector_ta::utilities::data_loader::Candles) {
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
        std::fs::write(path, out).expect("write csv");
    }

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let name = format!("{prefix}_{}_{}", std::process::id(), uniq);
        p.push(name);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    #[test]
    fn validation_shortlist_csv_smoke() {
        let validation_res = json!({
          "data_id": "test",
          "candidates": [
            {
              "params": { "fast_ma_type": "sma", "fast_len": 9, "slow_ma_type": "ema", "slow_len": 21 },
              "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
              "robust_score": 123.0,
              "robust_pareto": true,
              "cross_ticker": { "score": 0.7 },
              "cross_ticker_pareto": true
            }
          ]
        });

        let csv = build_validation_shortlist_csv_text(&validation_res).expect("csv");
        let mut it = csv.lines();
        let header = it.next().unwrap_or_default();
        assert_eq!(header.split(',').count(), 17);
        let row = it.next().unwrap_or_default();
        assert_eq!(row.split(',').count(), 17);
        assert!(csv.contains("rank,fast_ma,fast_len"));
        assert!(csv.contains("params_json"));
    }

    #[test]
    fn report_xlsx_bytes_smoke() {
        let report = json!({
          "version": 1,
          "kind": "vectorbt_report",
          "ts": "2026-01-26T00:00:00Z",
          "validation": {
            "res": {
              "data_id": "test",
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "sma",
                    "fast_len": 9,
                    "slow_ma_type": "sma",
                    "slow_len": 21,
                    "halftrend": { "amplitude": 2, "atr_period": 14, "channel_deviation": 2.0 },
                    "macz": {},
                    "ott": {},
                    "strategy": { "commission": 0.0, "eps_rel": 0.0, "long_only": true, "allow_flip": true, "trade_on_next_bar": true }
                  },
                  "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10, "exposure": 0.5, "net_exposure": 0.1 },
                  "robust_score": 1.23,
                  "robust_pareto": true
                }
              ]
            }
          }
        });

        let bytes = build_report_xlsx_bytes(&report, Some(50)).expect("xlsx bytes");
        assert!(bytes.len() > 512);
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn cross_ticker_validation_smoke_populates_stats() {
        let base_dir = make_temp_dir("vectorta_cross_ticker_test");
        let app_cache_dir = base_dir.join("data_cache");
        std::fs::create_dir_all(&app_cache_dir).expect("create cache dir");

        let ds1 = base_dir.join("ds1.csv");
        let ds2 = base_dir.join("ds2.csv");

        let candles1 = make_trending_candles(1024, 0.10);
        let candles2 = make_trending_candles(1024, 0.06);
        write_candles_csv(&ds1, &candles1);
        write_candles_csv(&ds2, &candles2);

        let cancel = std::sync::atomic::AtomicBool::new(false);

        let base_params = ta_app_core::CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            strategy: ta_strategies::double_ma::StrategyConfig::default(),
        };

        let req = ta_app_core::CompositeValidationRequest {
            data_id: "base".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base_params],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
        };

        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req,
            &candles1,
            &cancel,
            None,
        )
        .expect("base validation");

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: ds1.to_string_lossy().to_string(),
                    label: "ds1".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: ds2.to_string_lossy().to_string(),
                    label: "ds2".to_string(),
                },
            ],
            max_datasets: 8,
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            "close",
            None,
            None,
            1,
            &app_cache_dir,
            &cancel,
            None,
        )
        .expect("cross validation");

        let summary = res.cross_ticker.as_ref().expect("summary");
        assert_eq!(summary.datasets.len(), 2);
        let cand = &res.candidates[0];
        let cross = cand.cross_ticker.as_ref().expect("candidate cross");
        assert_eq!(cross.datasets, 2);
        assert!(cand.cross_ticker_pareto.is_some());
    }
}
