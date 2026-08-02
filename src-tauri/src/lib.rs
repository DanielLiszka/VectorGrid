pub mod broker_paper;
pub mod command_bindings;
pub mod commands;
pub mod data_cache;
pub mod desktop_menu;
pub mod desktop_tray;
pub mod execution_mocks;
pub mod execution_policy;
pub mod execution_quantity;
pub mod local_path_policy;
pub mod ma_registry;
pub mod polygon;
#[cfg(any(feature = "desktop-ui", test))]
mod qa_profile;
pub mod redaction;
pub mod release_readiness;
pub mod settings;
pub mod state;

#[cfg(feature = "desktop-ui")]
use commands::*;
#[cfg(feature = "desktop-ui")]
use log::LevelFilter;
#[cfg(feature = "desktop-ui")]
use state::AppState;
#[cfg(feature = "desktop-ui")]
use state::BackgroundTaskState;
#[cfg(feature = "desktop-ui")]
use tauri::Manager;
#[cfg(feature = "desktop-ui")]
use tauri_plugin_log::{Target, TargetKind};
#[cfg(any(feature = "desktop-ui", test))]
use tauri_plugin_window_state::StateFlags;
#[cfg(feature = "desktop-ui")]
use tauri_plugin_window_state::WindowExt;

#[cfg(feature = "desktop-ui")]
const MAIN_WINDOW_LABEL: &str = "main";

#[cfg(any(feature = "desktop-ui", test))]
fn main_window_restore_flags() -> StateFlags {
    StateFlags::all() & !StateFlags::VISIBLE
}

#[cfg(feature = "desktop-ui")]
fn focus_existing_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        log::warn!("single-instance focus requested before the main window existed");
        return;
    };
    if let Err(error) = window.unminimize() {
        log::warn!("failed to unminimize main window for single-instance activation: {error}");
    }
    if let Err(error) = window.show() {
        log::warn!("failed to show main window for single-instance activation: {error}");
    }
    if let Err(error) = window.set_focus() {
        log::warn!("failed to focus main window for single-instance activation: {error}");
    }
}

#[cfg(all(
    feature = "desktop-ui",
    feature = "cuda",
    feature = "cuda-backtest-kernel"
))]
fn spawn_options_expression_gpu_prewarm() {
    let builder = std::thread::Builder::new().name("options-expression-gpu-prewarm".to_string());
    if let Err(error) = builder.spawn(|| match ta_app_core::preferred_cuda_device_id_result() {
        Ok(Some(device_id)) => {
            if let Err(error) = ta_app_core::prewarm_options_expression_gpu_runtime(device_id) {
                log::warn!(
                    "options-expression GPU runtime prewarm failed for device {device_id}: {error}"
                );
            } else {
                log::info!("options-expression GPU runtime prewarmed for device {device_id}");
            }
        }
        Ok(None) => {}
        Err(error) => {
            log::warn!("options-expression GPU runtime prewarm skipped: {error}");
        }
    }) {
        log::warn!("failed to spawn options-expression GPU runtime prewarm thread: {error}");
    }
}

#[cfg(feature = "desktop-ui")]
pub fn run_tauri() {
    let qa_override = qa_profile::QaProfileOverride::from_environment()
        .unwrap_or_else(|error| panic!("refusing unsafe QA app-data override: {error}"));
    let mut context = tauri::generate_context!();
    let qa_original_identifier = qa_override
        .as_ref()
        .map(|profile| profile.apply_to_context(&mut context))
        .transpose()
        .unwrap_or_else(|error| panic!("refusing unsafe QA app-data override: {error}"));

    let mut builder = tauri::Builder::default();
    if qa_override.is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_existing_main_window(app);
        }));
    }

    let log_file_target =
        qa_override
            .as_ref()
            .map_or(TargetKind::LogDir { file_name: None }, |profile| {
                TargetKind::Folder {
                    path: profile.log_dir(),
                    file_name: None,
                }
            });
    builder = builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level_for(
                    "tao::platform_impl::platform::event_loop::runner",
                    LevelFilter::Error,
                )
                .level_for(
                    "tao::platform_impl::windows::event_loop::runner",
                    LevelFilter::Error,
                )
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(log_file_target),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init());

    let qa_setup_override = qa_override.clone();
    builder
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .skip_initial_state(MAIN_WINDOW_LABEL)
                .build(),
        )
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .manage(desktop_menu::DesktopMenuState::default())
        .setup(move |app| {
            if let Some(profile) = qa_setup_override.as_ref() {
                let original_identifier = qa_original_identifier.as_deref().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "QA app-data override is missing its original identifier",
                    )
                })?;
                profile
                    .verify_and_attest(app, original_identifier)
                    .map_err(|error| {
                        std::io::Error::new(std::io::ErrorKind::PermissionDenied, error)
                    })?;
            }

            let window_cfg = app.config().app.windows.get(0).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "missing window config")
            })?;

            let data_dir = qa_setup_override.as_ref().map_or_else(
                || std::env::temp_dir().join("vectorgrid").join("webview2"),
                qa_profile::QaProfileOverride::webview_data_dir,
            );

            let window = tauri::WebviewWindowBuilder::from_config(app.handle(), &window_cfg)?
                .data_directory(data_dir)
                .build()?;

            if let Err(error) = window.restore_state(main_window_restore_flags()) {
                log::warn!("failed to restore main window state: {error}");
            }

            if let Err(error) = window.show() {
                log::error!("failed to show main window: {error}");
                return Err(error.into());
            }
            if let Err(error) = desktop_menu::install_desktop_menu(app.handle()) {
                log::warn!("failed to install desktop menu: {error}");
            }
            if let Err(error) = desktop_tray::install_desktop_tray(app.handle()) {
                log::warn!("failed to install desktop tray: {error}");
            }

            #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
            spawn_options_expression_gpu_prewarm();

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
            get_artifact_retention_info,
            set_artifact_retention_settings,
            clear_artifact_retention_settings,
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
            cancel_live_execution_open_orders,
            cancel_local_paper_open_orders,
            flatten_live_execution_position,
            flatten_local_paper_position,
            emergency_halt_live_execution,
            start_live_execution_runner,
            stop_live_execution_runner,
            live_execution_runner_status,
            get_live_execution_snapshot,
            get_local_paper_execution_snapshot,
            get_execution_strategy_support_matrix,
            reset_local_paper_broker,
            test_alpaca_connection,
            alpaca_get_credential_status,
            alpaca_set_api_credentials,
            alpaca_clear_api_credentials,
            get_execution_risk_policy,
            set_execution_risk_policy,
            reset_execution_risk_policy,
            preview_execution_policy_decision,
            pick_csv_file,
            pick_save_csv,
            pick_save_json,
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
            save_vcet_temporal_report_xlsx,
            save_simple_summary_markdown,
            open_reports_dir,
            open_runs_dir,
            reveal_path,
            open_path,
            exit_application,
            set_window_effect,
            set_shell_menu_command_states,
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
            plan_vcet_temporal_validation,
            execute_vcet_temporal_validation,
            attest_vcet_temporal_datasets,
            seal_vcet_temporal_dataset_universe,
            execute_vcet_temporal_pre_final_shard,
            reduce_vcet_temporal_pre_final_shards,
            load_vcet_temporal_pre_final_winner_lock,
            execute_vcet_temporal_final_test_shard,
            reduce_vcet_temporal_final_test_shards,
            load_vcet_temporal_final_test_shard,
            prepare_vcet_temporal_campaign,
            run_vcet_temporal_campaign_selection,
            open_vcet_temporal_campaign_final_test,
            get_vcet_temporal_campaign_status,
            list_run_artifacts,
            list_live_session_artifacts,
            load_latest_run_artifact_summary,
            load_latest_run_artifact_hydration,
            list_artifact_library,
            rebuild_artifact_catalog,
            list_meta_artifacts,
            list_stage4_artifacts,
            list_stage5_artifacts,
            list_ml_policy_eval_artifacts,
            update_artifact_tags,
            load_artifact,
            load_run_artifact,
            load_run_artifact_hydration,
            load_run_artifact_detail,
            open_run_artifact_report,
            repair_run_artifact,
            load_live_session_artifact_detail,
            load_meta_artifact_replay,
            load_stage5_artifact_replay,
            load_ml_policy_eval_artifact_replay,
            delete_run_artifact,
            delete_artifact,
            run_options_expression,
            analyze_options_expression_feasibility,
            validate_options_expression_artifact,
            analyze_options_expression_validation_feasibility,
            analyze_portfolio_feasibility,
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
        .run(context)
        .expect("error while running tauri application");
}

#[cfg(not(feature = "desktop-ui"))]
pub fn run_tauri() {
    eprintln!("desktop UI is disabled; rebuild with default features to run the Tauri shell");
}

#[cfg(test)]
mod tests {
    use super::main_window_restore_flags;
    use tauri_plugin_window_state::StateFlags;

    #[test]
    fn main_window_restore_flags_excludes_visible() {
        let flags = main_window_restore_flags();
        assert!(!flags.contains(StateFlags::VISIBLE));
        assert!(flags.contains(StateFlags::SIZE));
        assert!(flags.contains(StateFlags::POSITION));
    }
}
