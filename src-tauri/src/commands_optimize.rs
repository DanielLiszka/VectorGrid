use super::{
    BackendOptimizationResult, CompositeCpuRequest, CompositeMoRequest, CompositeMoResult,
    DoubleMaRequest,
};
use crate::state::AppState;
use crate::state::BackgroundTaskState;
use tauri::Emitter;

#[tauri::command]
pub fn cancel_background_task(bg: tauri::State<'_, BackgroundTaskState>) -> Result<(), String> {
    bg.request_cancel();
    Ok(())
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

pub(crate) struct TauriProgressSink {
    pub(crate) app: tauri::AppHandle,
}

impl ta_app_core::progress::ProgressSink for TauriProgressSink {
    fn emit_double_ma_progress(&self, payload: ta_app_core::progress::DoubleMaProgressPayload) {
        let _ = self.app.emit("double_ma_progress", payload);
    }
}

pub(crate) struct OffsetProgressSink {
    pub(crate) app: tauri::AppHandle,
    pub(crate) base_offset: usize,
    pub(crate) total: usize,
    pub(crate) phase: &'static str,
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
    mut req: DoubleMaRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    req.mode = ta_optimizer::OptimizationMode::Grid;
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
    mut req: CompositeCpuRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    req.mode = ta_optimizer::OptimizationMode::Grid;
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
