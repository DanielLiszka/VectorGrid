use super::{
    BackendOptimizationResult, CompositeCpuRequest, CompositeMoRequest, CompositeMoResult,
    DoubleMaRequest,
};
use crate::state::AppState;
use crate::state::BackgroundTaskState;
use crate::state::RunGuard;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use ta_optimizer::OptimizationMode;
use tauri::Emitter;
use vector_ta::utilities::data_loader::Candles;

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

pub(crate) fn normalize_product_double_ma_optimization_mode(
    req: &DoubleMaRequest,
) -> OptimizationMode {
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

pub(crate) fn normalize_product_composite_cpu_optimization_mode(
    _mode: OptimizationMode,
) -> OptimizationMode {
    OptimizationMode::Grid
}

fn begin_optimization_run(
    state: &AppState,
    data_id: &str,
) -> Result<(RunGuard, Arc<AtomicBool>, Arc<Candles>), String> {
    let run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(data_id)?;
    Ok((run_guard, cancel, candles))
}

fn normalize_export_all_csv_save_path(raw: Option<String>) -> Result<Option<PathBuf>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(trimmed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .unwrap_or_default();
    if file_name.is_empty() {
        return Err(format!(
            "export_all_csv_path must point to a file: {}",
            path.to_string_lossy()
        ));
    }
    crate::local_path_policy::ensure_csv_extension(&path)?;
    Ok(Some(path))
}

fn validate_export_all_csv_path_for_command(
    raw: Option<String>,
    state: &AppState,
) -> Result<Option<String>, String> {
    let Some(path) = normalize_export_all_csv_save_path(raw)? else {
        return Ok(None);
    };
    state.consume_issued_csv_save_path(&path)?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn run_double_ma_optimization(
    mut req: DoubleMaRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    req.export_all_csv_path =
        validate_export_all_csv_path_for_command(req.export_all_csv_path.take(), &state)?;
    req.mode = normalize_product_double_ma_optimization_mode(&req);
    let (_run_guard, cancel, candles) = begin_optimization_run(&state, &req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope("engine", "tauri_double_ma_request");
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
    let (_run_guard, cancel, candles) = begin_optimization_run(&state, &req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope("engine", "tauri_composite_cpu_request");
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
        begin_optimization_run, normalize_export_all_csv_save_path,
        normalize_product_composite_cpu_optimization_mode,
        normalize_product_double_ma_optimization_mode, validate_export_all_csv_path_for_command,
    };
    use crate::state::AppState;
    use std::sync::atomic::Ordering;
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
            score_mode: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Auto,
            top_k: Some(20),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: Default::default(),
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,

            hurdle_tau: None,
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
    fn double_ma_product_mode_normalization_preserves_explicit_grid_mode() {
        let mut req = base_double_ma_req();
        req.mode = OptimizationMode::Grid;
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );

        req.include_all = Some(true);
        req.heatmap_bins = Some(64);
        req.export_all_csv_path = Some("out.csv".to_string());
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );
    }

    #[test]
    fn double_ma_product_mode_normalization_pins_auto_to_grid_when_outputs_are_requested() {
        let mut req = base_double_ma_req();
        req.mode = OptimizationMode::Auto;
        req.include_all = Some(true);
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );

        req.include_all = Some(false);
        req.heatmap_bins = Some(32);
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::Grid
        );
    }

    #[test]
    fn double_ma_product_mode_normalization_ignores_whitespace_export_path() {
        let mut req = base_double_ma_req();
        req.mode = OptimizationMode::GpuVdps;
        req.export_all_csv_path = Some("   ".to_string());
        assert_eq!(
            normalize_product_double_ma_optimization_mode(&req),
            OptimizationMode::GpuVdps
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

    #[test]
    fn export_all_csv_path_normalization_treats_whitespace_as_absent() {
        assert_eq!(
            normalize_export_all_csv_save_path(Some("   ".to_string())).expect("normalize"),
            None
        );
        assert_eq!(
            validate_export_all_csv_path_for_command(Some("   ".to_string()), &AppState::default())
                .expect("validate"),
            None
        );
    }

    #[test]
    fn export_all_csv_path_validation_requires_csv_extension() {
        let err = normalize_export_all_csv_save_path(Some("C:/tmp/out.txt".to_string()))
            .expect_err("non-csv export path should fail");
        assert!(err.contains(".csv"), "unexpected error: {err}");
    }

    #[test]
    fn export_all_csv_path_validation_requires_save_dialog_token() {
        let temp = std::env::temp_dir().join(format!(
            "ta_desktop_demo_export_all_unissued_{}_{}.csv",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let state = AppState::default();
        let err = validate_export_all_csv_path_for_command(
            Some(temp.to_string_lossy().to_string()),
            &state,
        )
        .expect_err("unissued export path should fail");
        assert!(
            err.contains("native save dialog"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn export_all_csv_path_validation_consumes_save_dialog_token_once() {
        let temp = std::env::temp_dir().join(format!(
            "ta_desktop_demo_export_all_issued_{}_{}.csv",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let state = AppState::default();
        state
            .record_issued_csv_save_path(temp.clone())
            .expect("record save token");

        let accepted = validate_export_all_csv_path_for_command(
            Some(temp.to_string_lossy().to_string()),
            &state,
        )
        .expect("issued export path should pass");
        assert_eq!(accepted, Some(temp.to_string_lossy().to_string()));

        validate_export_all_csv_path_for_command(Some(temp.to_string_lossy().to_string()), &state)
            .expect_err("save token should be single-use");
    }

    #[test]
    fn begin_optimization_run_releases_run_guard_when_data_lookup_fails() {
        let state = AppState::default();
        state.request_cancel();

        let err = match begin_optimization_run(&state, "missing-data-id") {
            Ok(_) => panic!("missing dataset should fail before the blocking optimizer starts"),
            Err(err) => err,
        };
        assert!(err.contains("unknown data id"));
        assert!(
            !state.cancel_token().load(Ordering::SeqCst),
            "acquiring the run guard should still clear any prior cancel intent"
        );

        let _guard = state
            .try_begin_run()
            .expect("run guard must be released when setup fails early");
    }
}

#[tauri::command]
pub async fn run_composite_mo_optimization(
    req: CompositeMoRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CompositeMoResult, String> {
    let (_run_guard, cancel, candles) = begin_optimization_run(&state, &req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope("engine", "tauri_composite_mo_request");
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
