# Backend/Frontend Command Surface Audit — 2026-04-13

## Scope

This audit answers a narrower and more useful question than "is all backend code exposed in the frontend?"

1. Are the backend Tauri commands registered in the frontend command registry?
2. Are those registered commands intentionally reachable from production frontend code?
3. Which backend functions are intentionally headless helpers and should not be treated as missing frontend exposure?

This is not a claim that every Rust function should be frontend-invokable. The meaningful audit boundary is the Tauri command surface rooted in [`src-tauri/src/lib.rs`](../../src-tauri/src/lib.rs), plus a short inventory of headless helpers that are backend-only by design.

## Executive Summary

- The current Tauri command surface is registry-consistent: the backend handler and generated frontend registry both contain `109` commands.
- I did not find audited `#[tauri::command]` functions in the `src-tauri/src/commands*.rs` surface that were missing from the backend `generate_handler!` list.
- The repo already has solid anti-drift checks for command-name parity:
  - [`web/scripts/generate-tauri-commands.mjs`](../../web/scripts/generate-tauri-commands.mjs)
  - [`web/scripts/check-tauri-parity.mjs`](../../web/scripts/check-tauri-parity.mjs)
  - [`web/scripts/check-known-tauri-commands.mjs`](../../web/scripts/check-known-tauri-commands.mjs)
- The real gap is not registry drift. The real gap is frontend reachability and ownership. A command can be present in the generated registry and still have no production UI path.
- The current production frontend appears to exercise roughly `101 / 109` registered commands.
- The highest-signal commands that look registered but not actually used by production frontend flows are:
  - `run_meta_stage4_experimental`
  - `format_dynamic_strategy_ir_json`
  - `save_batch_summary`
  - `list_meta_artifacts`
  - `list_stage4_artifacts`
  - `list_stage5_artifacts`
  - `list_ml_policy_eval_artifacts`
  - `list_options_expression_artifacts`

## Verification Performed

- `node ./scripts/generate-tauri-commands.mjs --check` from `web/`
  - Result: `OK: src/tauri/commands.ts is up to date (109 commands).`
- `node ./scripts/check-tauri-parity.mjs` from `web/`
  - Result: `Backend commands: 109`, `Frontend registry commands: 109`
- `node ./scripts/check-known-tauri-commands.mjs` from `web/`
  - Result: `OK: All literal invoke command names are known (109 registered commands).`

## Current Mapping

### 1. Registry and invoke boundary

- Backend registration is centralized in [`src-tauri/src/lib.rs`](../../src-tauri/src/lib.rs).
- Frontend command inventory is generated into [`web/src/tauri/commands.ts`](../../web/src/tauri/commands.ts).
- Runtime invocation is intentionally centralized in [`web/src/tauri/invoke.ts`](../../web/src/tauri/invoke.ts).
- Raw Tauri imports are intentionally constrained by [`web/src/tauri/runtimeWiringPolicy.test.ts`](../../web/src/tauri/runtimeWiringPolicy.test.ts).

This part is in good shape.

### 2. Backend area to frontend owner map

| Backend area | Backend modules | Representative commands | Frontend owner(s) | Status |
| --- | --- | --- | --- | --- |
| Data loading and resampling | [`src-tauri/src/commands_data.rs`](../../src-tauri/src/commands_data.rs) | `load_price_data`, `resample_price_data`, `clear_loaded_data`, `list_sample_csvs`, `list_repo_data_csvs` | [`web/src/services/dataService.ts`](../../web/src/services/dataService.ts), [`web/src/services/metaService.ts`](../../web/src/services/metaService.ts) | Covered |
| Market-data directory and Polygon historical ingest | [`src-tauri/src/commands_data.rs`](../../src-tauri/src/commands_data.rs) | `get_market_data_dir_info`, `set_market_data_dir`, `clear_market_data_dir`, `polygon_set_api_key`, `polygon_download_1m_and_load`, `polygon_top_off_dataset`, `polygon_validate_dataset`, `polygon_repair_dataset` | [`web/src/services/dataService.ts`](../../web/src/services/dataService.ts) | Covered |
| Polygon live candle stream | [`src-tauri/src/commands_data.rs`](../../src-tauri/src/commands_data.rs) | `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`, `polygon_live_candle_stream_status` | [`web/src/services/executionService.ts`](../../web/src/services/executionService.ts) | Covered |
| File dialogs and folder pickers | [`src-tauri/src/commands_data.rs`](../../src-tauri/src/commands_data.rs) | `pick_csv_file`, `pick_save_csv`, `pick_folder` | [`web/src/services/dataService.ts`](../../web/src/services/dataService.ts), [`web/src/services/optionsExpressionService.ts`](../../web/src/services/optionsExpressionService.ts), [`web/src/components/MlSignalWorkspacePanel.vue`](../../web/src/components/MlSignalWorkspacePanel.vue), [`web/src/stores/advancedStore.ts`](../../web/src/stores/advancedStore.ts) | Covered |
| Options dataset import/download | [`src-tauri/src/commands_options_data.rs`](../../src-tauri/src/commands_options_data.rs) | `download_polygon_options_dataset`, `import_options_dataset`, `list_options_datasets`, `load_options_dataset_manifest` | [`web/src/services/optionsExpressionService.ts`](../../web/src/services/optionsExpressionService.ts) | Covered |
| Optimization execution | [`src-tauri/src/commands_optimize.rs`](../../src-tauri/src/commands_optimize.rs) | `run_double_ma_optimization`, `run_composite_cpu_optimization`, `run_composite_mo_optimization` | [`web/src/services/optimizeService.ts`](../../web/src/services/optimizeService.ts) | Covered through command resolution |
| Run cancellation | [`src-tauri/src/commands_optimize.rs`](../../src-tauri/src/commands_optimize.rs) | `cancel_background_task`, `cancel_current_run`, `cancel_double_ma_optimization` | [`web/src/services/runControlService.ts`](../../web/src/services/runControlService.ts), [`web/src/stores/taskStore.ts`](../../web/src/stores/taskStore.ts) | Covered |
| Validation | [`src-tauri/src/commands_validation.rs`](../../src-tauri/src/commands_validation.rs) | `run_double_ma_validation`, `run_composite_validation` | [`web/src/services/validationService.ts`](../../web/src/services/validationService.ts) | Covered through command resolution |
| Drilldown, stability, and chart bars | [`src-tauri/src/commands_analysis.rs`](../../src-tauri/src/commands_analysis.rs), [`src-tauri/src/commands_validation.rs`](../../src-tauri/src/commands_validation.rs) | `compute_double_ma_drilldown`, `compute_composite_cpu_drilldown`, `compute_double_ma_local_stability`, `compute_composite_cpu_local_stability`, `get_price_bars` | [`web/src/services/analysisService.ts`](../../web/src/services/analysisService.ts), [`web/src/services/executionChartService.ts`](../../web/src/services/executionChartService.ts) | Covered |
| Run artifacts and history | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | `save_run_artifact`, `list_run_artifacts`, `load_run_artifact`, `load_run_artifact_hydration`, `load_run_artifact_detail`, `update_artifact_tags`, `delete_run_artifact`, `open_runs_dir` | [`web/src/services/optimizeService.ts`](../../web/src/services/optimizeService.ts), [`web/src/services/validationService.ts`](../../web/src/services/validationService.ts), [`web/src/services/mlSignalService.ts`](../../web/src/services/mlSignalService.ts), [`web/src/services/historyService.ts`](../../web/src/services/historyService.ts), [`web/src/services/settingsService.ts`](../../web/src/services/settingsService.ts) | Covered |
| Artifact library, live-session, and replay | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | `list_live_session_artifacts`, `load_live_session_artifact_detail`, `list_artifact_library`, `load_meta_artifact_replay`, `load_stage5_artifact_replay`, `load_ml_policy_eval_artifact_replay` | [`web/src/services/executionService.ts`](../../web/src/services/executionService.ts), [`web/src/services/metaService.ts`](../../web/src/services/metaService.ts), [`web/src/services/optionsExpressionService.ts`](../../web/src/services/optionsExpressionService.ts) | Covered |
| Reports and export | [`src-tauri/src/commands_reports_api.rs`](../../src-tauri/src/commands_reports_api.rs) | `build_report_xlsx`, `build_validation_shortlist_csv`, `save_report_xlsx`, `save_report_xlsx_from_artifact`, `open_reports_dir`, `open_path`, `reveal_path` | [`web/src/services/validationService.ts`](../../web/src/services/validationService.ts), [`web/src/services/settingsService.ts`](../../web/src/services/settingsService.ts), [`web/src/utils/openPath.ts`](../../web/src/utils/openPath.ts), [`web/src/services/dataService.ts`](../../web/src/services/dataService.ts), [`web/src/services/metaService.ts`](../../web/src/services/metaService.ts) | Covered |
| Meta selection and ML confidence | [`src-tauri/src/commands_meta.rs`](../../src-tauri/src/commands_meta.rs) | `ml_confidence_model_status`, `train_ml_confidence_model`, `apply_ml_confidence_model`, `delete_ml_confidence_model`, `run_meta_stage5_ml_confidence`, `evaluate_ml_confidence_policy`, `run_meta_selection` | [`web/src/services/metaService.ts`](../../web/src/services/metaService.ts) | Covered |
| ML signal mode | [`src-tauri/src/commands_ml_signal.rs`](../../src-tauri/src/commands_ml_signal.rs) | `export_ml_signal_dataset`, `generate_ml_signal_score_cache`, `run_ml_signal_validation`, `run_ml_signal_policy_sweep`, `list_ml_signal_model_bundles`, `ml_signal_model_status` | [`web/src/services/mlSignalService.ts`](../../web/src/services/mlSignalService.ts) | Covered |
| Options expression workflow | [`src-tauri/src/commands_options_expression.rs`](../../src-tauri/src/commands_options_expression.rs) | `run_options_expression`, `analyze_options_expression_feasibility`, `load_options_expression_artifact`, `save_options_csv`, `inspect_options_expression_tuple`, `inspect_options_expression_event_path` | [`web/src/services/optionsExpressionService.ts`](../../web/src/services/optionsExpressionService.ts) | Covered |
| Live execution and broker control | [`src-tauri/src/commands_execution.rs`](../../src-tauri/src/commands_execution.rs) | `execute_strategy_signal_once`, `start_live_execution_runner`, `stop_live_execution_runner`, `live_execution_runner_status`, `get_live_execution_snapshot`, `cancel_live_execution_open_orders`, `flatten_live_execution_position`, `emergency_halt_live_execution`, `test_alpaca_connection`, `alpaca_get_credential_status`, `alpaca_set_api_credentials`, `alpaca_clear_api_credentials` | [`web/src/services/executionService.ts`](../../web/src/services/executionService.ts) | Covered |
| Desktop/system and advanced dynamic tooling | [`src-tauri/src/commands_system.rs`](../../src-tauri/src/commands_system.rs), [`src-tauri/src/commands_dynamic.rs`](../../src-tauri/src/commands_dynamic.rs) | `set_window_effect`, `exit_application`, `get_gpu_status`, `preview_dynamic_double_ma_strategy_ir_json`, `encode_dynamic_strategy_constant_bits` | [`web/src/services/settingsService.ts`](../../web/src/services/settingsService.ts), [`web/src/components/DesktopTitlebar.vue`](../../web/src/components/DesktopTitlebar.vue), [`web/src/services/optimizeService.ts`](../../web/src/services/optimizeService.ts) | Covered except one orphaned formatter command noted below |

## Gaps: Registered But Not Meaningfully Exposed in Production Frontend

These commands are present in the generated frontend registry, but I did not find meaningful production call sites for them.

| Command | Backend location | Current frontend situation | Notes |
| --- | --- | --- | --- |
| `run_meta_stage4_experimental` | [`src-tauri/src/commands_meta.rs`](../../src-tauri/src/commands_meta.rs) | Appears in task tracking and notification metadata, but not in an actual production invoke path | [`web/src/services/metaService.ts`](../../web/src/services/metaService.ts) currently aliases `runStage4()` to `runStage5()`, so Stage 4 backend execution is effectively not surfaced |
| `format_dynamic_strategy_ir_json` | [`src-tauri/src/commands_dynamic.rs`](../../src-tauri/src/commands_dynamic.rs) | Present in registry and task metadata only | No production invoke site found; likely stale or incomplete advanced tooling surface |
| `save_batch_summary` | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | Test-only evidence in [`web/src/services/metaService.test.ts`](../../web/src/services/metaService.test.ts) | No production frontend owner found |
| `list_meta_artifacts` | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | Registry-only | Frontend uses `list_artifact_library` instead |
| `list_stage4_artifacts` | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | Registry-only | Frontend uses `list_artifact_library` instead |
| `list_stage5_artifacts` | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | Registry-only | Frontend uses `list_artifact_library` instead |
| `list_ml_policy_eval_artifacts` | [`src-tauri/src/commands_artifacts.rs`](../../src-tauri/src/commands_artifacts.rs) | Registry-only | Frontend uses `list_artifact_library` instead |
| `list_options_expression_artifacts` | [`src-tauri/src/commands_options_expression.rs`](../../src-tauri/src/commands_options_expression.rs) | Registry-only | [`web/src/services/optionsExpressionService.ts`](../../web/src/services/optionsExpressionService.ts) uses `list_artifact_library` and slices the `optionsExpression` section |

## Important Clarification: Backend Helpers That Are Intentionally Not Frontend Commands

The backend also contains several headless helper surfaces that should not be treated as missing frontend exposure just because they are re-exported from [`src-tauri/src/commands.rs`](../../src-tauri/src/commands.rs).

High-signal backend-only helper families include:

- Research-campaign headless helpers in [`src-tauri/src/commands_campaign.rs`](../../src-tauri/src/commands_campaign.rs)
- Options dataset headless import/download helpers in [`src-tauri/src/commands_options_data.rs`](../../src-tauri/src/commands_options_data.rs)
- Options expression headless run/load/save/list helpers in [`src-tauri/src/commands_options_expression.rs`](../../src-tauri/src/commands_options_expression.rs)
- Report byte-builder helper in [`src-tauri/src/commands_reports.rs`](../../src-tauri/src/commands_reports.rs)
- Cross-ticker validation headless helper in [`src-tauri/src/commands_validation.rs`](../../src-tauri/src/commands_validation.rs)

These functions are useful backend/runtime entrypoints for binaries, headless flows, and orchestration, but they are not missing frontend wiring unless product explicitly requires a UI path for them.

## Why Existing Parity Checks Miss the Real Gap

Current safeguards are good at name parity and wrapper discipline, but weak at proving true feature exposure.

- [`web/scripts/generate-tauri-commands.mjs`](../../web/scripts/generate-tauri-commands.mjs) and [`web/scripts/check-tauri-parity.mjs`](../../web/scripts/check-tauri-parity.mjs) only prove that the registry names match.
- [`web/scripts/check-known-tauri-commands.mjs`](../../web/scripts/check-known-tauri-commands.mjs) only checks literal command names.
- [`web/src/tauri/invoke.contract.test.ts`](../../web/src/tauri/invoke.contract.test.ts) loops over the generated registry with a mocked invoke layer, so it does not prove that a real frontend workflow reaches a command.
- Several important commands are invoked through resolved command names, not direct literals. That means simple grep will undercount valid coverage unless service-level routing is inspected.

The repo therefore needs a maintained "frontend owner" map if the goal is product-surface completeness rather than registry consistency.

## Recommendations

1. Treat the eight commands above as the immediate cleanup set.
   - Either wire them to real UI/service flows or remove/deprecate them from the frontend-facing Tauri surface.
2. Add a lightweight maintained owner map near the Tauri boundary.
   - A small checked-in table keyed by command name, backend module, frontend owner service, and intended UI entrypoint would prevent future ambiguity.
3. Extend parity tooling with a reachability audit.
   - A script should flag commands that are present in `web/src/tauri/commands.ts` but absent from production `web/src/services`, `web/src/stores`, `web/src/components`, and utility call paths.
4. Keep backend-only helpers explicitly labeled as headless/runtime-only.
   - That avoids repeating the current confusion caused by mixed re-exports in [`src-tauri/src/commands.rs`](../../src-tauri/src/commands.rs).

## Bottom Line

If the question is "are backend Tauri commands exposed to the frontend registry?", the answer is yes.

If the question is "does the production frontend appear to intentionally surface the entire registered backend command set?", the answer is no. The current evidence points to a small but real set of registered-orphaned commands, plus a broader documentation gap around command ownership and intended UI entrypoints.
