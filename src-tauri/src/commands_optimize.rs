use super::{
    BackendOptimizationResult, CompositeCpuRequest, CompositeMoRequest, CompositeMoResult,
    DoubleMaRequest,
};
use crate::state::AppState;
use crate::state::BackgroundTaskState;
use ta_optimizer::OptimizationMode;
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

fn normalize_product_double_ma_optimization_mode(req: &DoubleMaRequest) -> OptimizationMode {
    let grid_outputs_requested = req.include_all.unwrap_or(false)
        || req.heatmap_bins.unwrap_or(0) > 0
        || req
            .export_all_csv_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());

    match req.mode {
        OptimizationMode::Grid => OptimizationMode::Grid,
        mode if !grid_outputs_requested => mode,
        _ => OptimizationMode::Grid,
    }
}

fn normalize_product_composite_cpu_optimization_mode(_mode: OptimizationMode) -> OptimizationMode {
    OptimizationMode::Grid
}

#[tauri::command]
pub async fn run_double_ma_optimization(
    mut req: DoubleMaRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    req.mode = normalize_product_double_ma_optimization_mode(&req);
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
    req.mode = normalize_product_composite_cpu_optimization_mode(req.mode);
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

#[cfg(test)]
mod tests {
    use super::{
        normalize_product_composite_cpu_optimization_mode,
        normalize_product_double_ma_optimization_mode,
    };
    use ta_app_core::{Backend, DoubleMaRequest};
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;

    fn base_double_ma_req() -> DoubleMaRequest {
        DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "x".to_string(),
            fast_range: (5, 25, 1),
            slow_range: (20, 100, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Auto,
            top_k: Some(20),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
        }
    }

    #[test]
    fn double_ma_product_mode_normalization_preserves_non_grid_when_outputs_allow_it() {
        let mut req = base_double_ma_req();
        req.mode = OptimizationMode::GpuVdps;
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::GpuVdps
        );

        req.mode = OptimizationMode::CoarseToFine;
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::CoarseToFine
        );

        req.mode = OptimizationMode::Auto;
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Auto
        );
    }

    #[test]
    fn double_ma_product_mode_normalization_pins_grid_outputs_to_grid() {
        let mut req = base_double_ma_req();
        req.mode = OptimizationMode::GpuVdps;
        req.heatmap_bins = Some(64);
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );

        req.heatmap_bins = Some(0);
        req.include_all = Some(true);
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );

        req.include_all = Some(false);
        req.export_all_csv_path = Some("out.csv".to_string());
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );
    }

    #[test]
    fn composite_cpu_product_mode_normalization_stays_grid_only() {
        assert_eq!(
            normalize_product_composite_cpu_optimization_mode(OptimizationMode::Grid),
            OptimizationMode::Grid
        );
        assert_eq!(
            normalize_product_composite_cpu_optimization_mode(OptimizationMode::CoarseToFine),
            OptimizationMode::Grid
        );
        assert_eq!(
            normalize_product_composite_cpu_optimization_mode(OptimizationMode::GpuVdps),
            OptimizationMode::Grid
        );
        assert_eq!(
            normalize_product_composite_cpu_optimization_mode(OptimizationMode::Auto),
            OptimizationMode::Grid
        );
    }
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
