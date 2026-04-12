pub mod commands;
pub mod data_cache;
pub mod ma_registry;
pub mod polygon;
pub mod settings;
pub mod state;

use commands::*;
use state::AppState;
use state::BackgroundTaskState;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_window_state::{StateFlags, WindowExt};

pub fn run_tauri() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .setup(|app| {
            let window_cfg = app.config().app.windows.get(0).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "missing window config")
            })?;

            let data_dir = std::env::temp_dir().join("vectorgrid").join("webview2");

            let window = tauri::WebviewWindowBuilder::from_config(app.handle(), &window_cfg)?
                .data_directory(data_dir)
                .build()?;

            if let Err(error) = window.restore_state(StateFlags::all()) {
                log::warn!("failed to restore main window state: {error}");
            }

            if let Err(error) = window.show() {
                log::error!("failed to show main window: {error}");
                return Err(error.into());
            }

            log::info!("main window initialized");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_price_data,
            resample_price_data,
            clear_loaded_data,
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
            polygon_top_off_dataset,
            download_polygon_options_dataset,
            import_options_dataset,
            list_options_datasets,
            load_options_dataset_manifest,
            execute_strategy_signal_once,
            start_live_execution_runner,
            stop_live_execution_runner,
            live_execution_runner_status,
            get_live_execution_snapshot,
            test_alpaca_connection,
            alpaca_get_credential_status,
            alpaca_set_api_credentials,
            alpaca_clear_api_credentials,
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
            save_report_xlsx_from_artifact,
            open_reports_dir,
            open_runs_dir,
            reveal_path,
            open_path,
            exit_application,
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
            run_research_campaign,
            list_run_artifacts,
            load_latest_run_artifact_summary,
            load_latest_run_artifact_hydration,
            list_artifact_library,
            list_meta_artifacts,
            list_stage4_artifacts,
            list_stage5_artifacts,
            list_ml_policy_eval_artifacts,
            update_artifact_tags,
            load_run_artifact,
            load_run_artifact_hydration,
            load_run_artifact_detail,
            load_meta_artifact_replay,
            load_stage5_artifact_replay,
            load_ml_policy_eval_artifact_replay,
            delete_run_artifact,
            run_options_expression,
            analyze_options_expression_feasibility,
            list_options_expression_artifacts,
            load_options_expression_artifact,
            save_options_csv,
            inspect_options_expression_tuple,
            inspect_options_expression_event_path,
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
