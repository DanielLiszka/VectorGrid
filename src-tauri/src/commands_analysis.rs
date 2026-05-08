use super::{
    CompositeCpuDrilldownRequest, CompositeCpuDrilldownResult, CompositeCpuLocalStabilityRequest,
    DoubleMaCurves, DoubleMaDrilldownRequest, DoubleMaLocalStabilityRequest, LocalStabilityResult,
};
use crate::state::AppState;

#[tauri::command]
pub async fn compute_double_ma_drilldown(
    req: DoubleMaDrilldownRequest,
    state: tauri::State<'_, AppState>,
) -> Result<DoubleMaCurves, String> {
    let _run_guard = state.try_begin_run()?;
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
) -> Result<CompositeCpuDrilldownResult, String> {
    let _run_guard = state.try_begin_run()?;
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
    let _run_guard = state.try_begin_run()?;
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
    let _run_guard = state.try_begin_run()?;
    let candles = state.get_candles(&req.data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        ta_app_core::compute_composite_cpu_local_stability_blocking_with_candles(req, &candles)
    })
    .await
    .map_err(|e| e.to_string())?
}
