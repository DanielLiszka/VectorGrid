# Desktop Capability Inventory

Date: 2026-03-28

Purpose: reviewable capability baseline for the desktop productization track.

## Stable Window Labels

- `main`
  - the only current product window
  - bound to the `main-window` capability
- `dev-*`
  - reserved pattern for any future diagnostics or inspection windows
  - bound to the currently inactive `dev-window` capability

## Enabled Capabilities

- `main-window`
  - file: [main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
  - windows: `main`
  - permissions:
    - `core:default`
    - `core:window:allow-start-dragging`
    - `core:window:allow-minimize`
    - `core:window:allow-toggle-maximize`
    - `core:window:allow-close`
    - `log:default`
    - `notification:default`
    - `window-state:default`

## Reserved Capabilities

- `dev-window`
  - file: [dev.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\dev.json)
  - windows: `dev-*`
  - purpose: future diagnostics or inspection windows only
  - currently not enabled in config

## Command Surface By Workflow

### Data, file selection, and cache paths

- `load_price_data`
- `resample_price_data`
- `list_sample_csvs`
- `list_repo_data_csvs`
- `get_market_data_dir_info`
- `set_market_data_dir`
- `clear_market_data_dir`
- `pick_csv_file`
- `pick_save_csv`
- `pick_folder`
- `open_reports_dir`
- `open_runs_dir`
- `reveal_path`
- `open_path`

Trust notes:
- user-directed filesystem flows
- path-opening commands are high-trust because they cross out of the app shell

### Market-data integration and live feed

- `polygon_get_cache_info`
- `polygon_set_api_key`
- `polygon_clear_api_key`
- `polygon_start_live_candle_stream`
- `polygon_stop_live_candle_stream`
- `polygon_live_candle_stream_status`
- `polygon_validate_dataset`
- `polygon_repair_dataset`
- `polygon_download_1m_and_load`

Trust notes:
- external network and credential-adjacent operations
- high-trust because they affect cache integrity and live data ingress

### Live execution and brokerage posture

- `execute_strategy_signal_once`
- `start_live_execution_runner`
- `stop_live_execution_runner`
- `live_execution_runner_status`
- `get_live_execution_snapshot`
- `test_alpaca_connection`

Trust notes:
- highest-trust desktop surface
- credentials, live-run posture, and broker connectivity

### Optimization and validation

- `run_double_ma_optimization`
- `run_composite_cpu_optimization`
- `run_composite_mo_optimization`
- `run_composite_validation`
- `run_double_ma_validation`
- `cancel_background_task`
- `cancel_current_run`
- `cancel_double_ma_optimization`

Trust notes:
- core workload surface
- long-running execution, cancellations, and result generation

### Reports, exports, and artifact persistence

- `build_report_xlsx`
- `build_validation_shortlist_csv`
- `save_report_xlsx`
- `save_run_artifact`
- `save_batch_summary`
- `list_run_artifacts`
- `list_meta_artifacts`
- `list_stage4_artifacts`
- `list_stage5_artifacts`
- `list_ml_policy_eval_artifacts`
- `update_artifact_tags`
- `load_run_artifact`
- `delete_run_artifact`

Trust notes:
- filesystem write/delete paths
- review-sensitive because they affect saved operator outputs

### Meta and ML workflows

- `run_meta_selection`
- `run_meta_stage4_experimental`
- `run_meta_stage5_ml_confidence`
- `export_ml_signal_dataset`
- `generate_ml_signal_score_cache`
- `run_ml_signal_validation`
- `run_ml_signal_policy_sweep`
- `list_ml_signal_model_bundles`
- `ml_signal_model_status`
- `ml_confidence_model_status`
- `train_ml_confidence_model`
- `apply_ml_confidence_model`
- `delete_ml_confidence_model`
- `evaluate_ml_confidence_policy`

Trust notes:
- long-running compute plus persisted model state
- advanced/power-user surface

### Analysis and drilldown

- `get_price_bars`
- `compute_double_ma_drilldown`
- `compute_composite_cpu_drilldown`
- `compute_double_ma_local_stability`
- `compute_composite_cpu_local_stability`

Trust notes:
- read-heavy analytical surface
- lower system risk than live execution or artifact deletion

### Runtime and capability metadata

- `set_window_effect`
- `list_moving_averages`
- `list_dynamic_indicators`
- `list_gpu_speedups`
- `get_gpu_status`
- `preview_dynamic_double_ma_strategy_ir_json`
- `format_dynamic_strategy_ir_json`
- `encode_dynamic_strategy_constant_bits`

Trust notes:
- mixed routine/advanced surface
- `set_window_effect` is the only direct desktop-shell mutation in the current invoke set

## Review Notes

- the desktop capability files currently scope core/plugin permissions, not individual invoke commands
- the shipped custom titlebar now uses explicit `core:window` permissions for drag/minimize/maximize/close behavior on the `main-window` capability
- custom command granularity remains a later hardening step and should only happen after the current desktop behavior baseline is stable
- high-stakes frontend entry points for live execution, filesystem exports, and cache path management must remain exposed while desktop productization lands
