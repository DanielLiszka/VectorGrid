pub mod commands;
pub mod data_cache;
pub mod ma_registry;
pub mod polygon;
pub mod settings;
pub mod state;

use commands::*;
use state::AppState;
use state::BackgroundTaskState;

pub fn run_tauri() {
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .setup(|app| {
            let window_cfg = app.config().app.windows.get(0).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "missing window config")
            })?;

            let data_dir = std::env::temp_dir().join("vectorgrid").join("webview2");

            tauri::WebviewWindowBuilder::from_config(app.handle(), &window_cfg)?
                .data_directory(data_dir)
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_price_data,
            resample_price_data,
            list_sample_csvs,
            list_repo_data_csvs,
            get_market_data_dir_info,
            set_market_data_dir,
            clear_market_data_dir,
            polygon_get_cache_info,
            polygon_set_api_key,
            polygon_clear_api_key,
            polygon_start_live_candle_stream,
            polygon_stop_live_candle_stream,
            polygon_live_candle_stream_status,
            polygon_validate_dataset,
            polygon_repair_dataset,
            polygon_download_1m_and_load,
            execute_strategy_signal_once,
            start_live_execution_runner,
            stop_live_execution_runner,
            live_execution_runner_status,
            get_live_execution_snapshot,
            test_alpaca_connection,
            pick_csv_file,
            pick_save_csv,
            pick_folder,
            cancel_background_task,
            run_double_ma_optimization,
            run_composite_cpu_optimization,
            run_composite_mo_optimization,
            run_composite_validation,
            run_double_ma_validation,
            build_report_xlsx,
            build_validation_shortlist_csv,
            save_report_xlsx,
            open_reports_dir,
            open_runs_dir,
            reveal_path,
            open_path,
            set_window_effect,
            run_meta_selection,
            run_meta_stage4_experimental,
            run_meta_stage5_ml_confidence,
            export_ml_signal_dataset,
            generate_ml_signal_score_cache,
            run_ml_signal_validation,
            run_ml_signal_policy_sweep,
            list_ml_signal_model_bundles,
            ml_signal_model_status,
            ml_confidence_model_status,
            train_ml_confidence_model,
            apply_ml_confidence_model,
            delete_ml_confidence_model,
            evaluate_ml_confidence_policy,
            save_run_artifact,
            save_batch_summary,
            list_run_artifacts,
            list_meta_artifacts,
            list_stage4_artifacts,
            list_stage5_artifacts,
            list_ml_policy_eval_artifacts,
            update_artifact_tags,
            load_run_artifact,
            delete_run_artifact,
            get_price_bars,
            compute_double_ma_drilldown,
            compute_composite_cpu_drilldown,
            compute_double_ma_local_stability,
            compute_composite_cpu_local_stability,
            cancel_current_run,
            cancel_double_ma_optimization,
            list_moving_averages,
            list_dynamic_indicators,
            list_gpu_speedups,
            get_gpu_status,
            preview_dynamic_double_ma_strategy_ir_json,
            format_dynamic_strategy_ir_json,
            encode_dynamic_strategy_constant_bits
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
