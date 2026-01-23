use tauri::Emitter;
use tauri::Manager;

use crate::ma_registry::MaInfo;
use crate::polygon;
use crate::state::AppState;
use crate::state::LoadPriceDataResponse;
use serde::Serialize;
use std::path::PathBuf;

pub use ta_app_core::{
    Backend, BackendOptimizationResult, BackendUsed, CompositeCpuDrilldownRequest,
    CompositeCpuRequest, DoubleMaCurves, DoubleMaDrilldownRequest, DoubleMaParamsResolved,
    DoubleMaRequest, OptimizationModeResolved,
};

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

struct TauriProgressSink {
    app: tauri::AppHandle,
}

impl ta_app_core::progress::ProgressSink for TauriProgressSink {
    fn emit_double_ma_progress(&self, payload: ta_app_core::progress::DoubleMaProgressPayload) {
        let _ = self.app.emit("double_ma_progress", payload);
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
