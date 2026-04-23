# Backend Review Third-Pass Master Tracker

Last updated: `2026-04-14`

## Purpose

This document tracks the full third-pass backend review campaign.

The third pass started as a targeted high-ROI sweep and was then expanded to cover every existing second-pass review packet that still lacked a third-pass packet. The goal of the expanded pass is not to manufacture novelty for every file; it is to reopen the live source, apply fresh web research where it improves confidence, and sharpen any missed edge cases, compatibility boundaries, or low-regression hardening opportunities.

## Scope

This tracker now covers every existing second-pass packet. There are `116` planned third-pass packets, each under `Repo Review/backend-file-reviews-third-pass/`, and the completion target equals `116`.

## Third-Pass Quality Bar

Every third-pass packet should meet this bar:

- reopen the live source file instead of trusting earlier packet wording
- use fresh official or primary-source web research where it improves confidence
- explicitly call out what the second pass missed, overstated, or underweighted
- prefer bounded implementation guidance over generic refactor language
- preserve current capability and avoid regression-prone advice
- separate safe-now hardening from larger redesign still deferred

## Packet Index

### `src-tauri/src`

| # | Source file | Second-pass packet | Third-pass packet | Status |
| --- | --- | --- | --- | --- |
| 1 | `src-tauri/src/commands_analysis.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_analysis.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_analysis.rs.review.md` | `Reviewed` |
| 2 | `src-tauri/src/commands_artifacts.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_artifacts.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_artifacts.rs.review.md` | `Reviewed` |
| 3 | `src-tauri/src/commands_campaign.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_campaign.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_campaign.rs.review.md` | `Reviewed` |
| 4 | `src-tauri/src/commands_data.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_data.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_data.rs.review.md` | `Reviewed` |
| 5 | `src-tauri/src/commands_dynamic.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_dynamic.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_dynamic.rs.review.md` | `Reviewed` |
| 6 | `src-tauri/src/commands_execution.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_execution.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_execution.rs.review.md` | `Reviewed` |
| 7 | `src-tauri/src/commands_meta.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_meta.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_meta.rs.review.md` | `Reviewed` |
| 8 | `src-tauri/src/commands_ml_signal.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_ml_signal.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_ml_signal.rs.review.md` | `Reviewed` |
| 9 | `src-tauri/src/commands_optimize.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_optimize.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_optimize.rs.review.md` | `Reviewed` |
| 10 | `src-tauri/src/commands_options_data.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_data.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_options_data.rs.review.md` | `Reviewed` |
| 11 | `src-tauri/src/commands_options_expression.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_expression.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_options_expression.rs.review.md` | `Reviewed` |
| 12 | `src-tauri/src/commands_reports_api.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_api.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_api.rs.review.md` | `Reviewed` |
| 13 | `src-tauri/src/commands_reports_csv.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_csv.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_csv.rs.review.md` | `Reviewed` |
| 14 | `src-tauri/src/commands_reports_meta_xlsx.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_meta_xlsx.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_meta_xlsx.rs.review.md` | `Reviewed` |
| 15 | `src-tauri/src/commands_reports_shared.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_shared.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_shared.rs.review.md` | `Reviewed` |
| 16 | `src-tauri/src/commands_reports_validation_common.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_common.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_validation_common.rs.review.md` | `Reviewed` |
| 17 | `src-tauri/src/commands_reports_validation_xlsx.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports_validation_xlsx.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports_validation_xlsx.rs.review.md` | `Reviewed` |
| 18 | `src-tauri/src/commands_reports.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_reports.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_reports.rs.review.md` | `Reviewed` |
| 19 | `src-tauri/src/commands_system.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_system.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_system.rs.review.md` | `Reviewed` |
| 20 | `src-tauri/src/commands_validation.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_validation.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_validation.rs.review.md` | `Reviewed` |
| 21 | `src-tauri/src/commands.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands.rs.review.md` | `Reviewed` |
| 22 | `src-tauri/src/data_cache.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/data_cache.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/data_cache.rs.review.md` | `Reviewed` |
| 23 | `src-tauri/src/lib.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/lib.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/lib.rs.review.md` | `Reviewed` |
| 24 | `src-tauri/src/ma_registry.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/ma_registry.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/ma_registry.rs.review.md` | `Reviewed` |
| 25 | `src-tauri/src/main.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/main.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/main.rs.review.md` | `Reviewed` |
| 26 | `src-tauri/src/metric_surface.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/metric_surface.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/metric_surface.rs.review.md` | `Reviewed` |
| 27 | `src-tauri/src/options_parent_adapters.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/options_parent_adapters.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/options_parent_adapters.rs.review.md` | `Reviewed` |
| 28 | `src-tauri/src/polygon.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/polygon.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/polygon.rs.review.md` | `Reviewed` |
| 29 | `src-tauri/src/run_artifacts.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/run_artifacts.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/run_artifacts.rs.review.md` | `Reviewed` |
| 30 | `src-tauri/src/settings.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/settings.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/settings.rs.review.md` | `Reviewed` |
| 31 | `src-tauri/src/state.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/state.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/state.rs.review.md` | `Reviewed` |
| 32 | `src-tauri/src/validation_common.rs` | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/validation_common.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/validation_common.rs.review.md` | `Reviewed` |

### `crates/app_core/src`

| # | Source file | Second-pass packet | Third-pass packet | Status |
| --- | --- | --- | --- | --- |
| 33 | `crates/app_core/src/alloc_policy.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/alloc_policy.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/alloc_policy.rs.review.md` | `Reviewed` |
| 34 | `crates/app_core/src/asr_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/asr_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/asr_gpu_kernel.rs.review.md` | `Reviewed` |
| 35 | `crates/app_core/src/asr.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/asr.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/asr.rs.review.md` | `Reviewed` |
| 36 | `crates/app_core/src/carm_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/carm_gpu_kernel.rs.review.md` | `Reviewed` |
| 37 | `crates/app_core/src/carm.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/carm.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/carm.rs.review.md` | `Reviewed` |
| 38 | `crates/app_core/src/cdmc_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cdmc_gpu_kernel.rs.review.md` | `Reviewed` |
| 39 | `crates/app_core/src/cdmc.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cdmc.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cdmc.rs.review.md` | `Reviewed` |
| 40 | `crates/app_core/src/composite_cpu.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_cpu.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_cpu.rs.review.md` | `Reviewed` |
| 41 | `crates/app_core/src/composite_gpu_metric_decode.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_gpu_metric_decode.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_gpu_metric_decode.rs.review.md` | `Reviewed` |
| 42 | `crates/app_core/src/composite_mo.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_mo.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_mo.rs.review.md` | `Reviewed` |
| 43 | `crates/app_core/src/composite_profile_route.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_profile_route.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_profile_route.rs.review.md` | `Reviewed` |
| 44 | `crates/app_core/src/composite_validate.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/composite_validate.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/composite_validate.rs.review.md` | `Reviewed` |
| 45 | `crates/app_core/src/cscv_pbo.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cscv_pbo.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cscv_pbo.rs.review.md` | `Reviewed` |
| 46 | `crates/app_core/src/csv_compat.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/csv_compat.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/csv_compat.rs.review.md` | `Reviewed` |
| 47 | `crates/app_core/src/cuda_device.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cuda_device.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cuda_device.rs.review.md` | `Reviewed` |
| 48 | `crates/app_core/src/double_ma_backend.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_backend.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_backend.rs.review.md` | `Reviewed` |
| 49 | `crates/app_core/src/double_ma_ma_kind.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_ma_kind.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_ma_kind.rs.review.md` | `Reviewed` |
| 50 | `crates/app_core/src/double_ma_mode.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_mode.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_mode.rs.review.md` | `Reviewed` |
| 51 | `crates/app_core/src/double_ma_result.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_result.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_result.rs.review.md` | `Reviewed` |
| 52 | `crates/app_core/src/double_ma_strategy.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_strategy.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_strategy.rs.review.md` | `Reviewed` |
| 53 | `crates/app_core/src/double_ma_test_support.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_test_support.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_test_support.rs.review.md` | `Reviewed` |
| 54 | `crates/app_core/src/double_ma_validate.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_validate.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_validate.rs.review.md` | `Reviewed` |
| 55 | `crates/app_core/src/double_ma_vdps.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_vdps.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_vdps.rs.review.md` | `Reviewed` |
| 56 | `crates/app_core/src/double_ma.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma.rs.review.md` | `Reviewed` |
| 57 | `crates/app_core/src/dynamic_indicator_adapter.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/dynamic_indicator_adapter.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/dynamic_indicator_adapter.rs.review.md` | `Reviewed` |
| 58 | `crates/app_core/src/gpu_backtest_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/gpu_backtest_kernel.rs.review.md` | `Reviewed` |
| 59 | `crates/app_core/src/indicator_catalog.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/indicator_catalog.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/indicator_catalog.rs.review.md` | `Reviewed` |
| 60 | `crates/app_core/src/indicator_factory.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/indicator_factory.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/indicator_factory.rs.review.md` | `Reviewed` |
| 61 | `crates/app_core/src/iterative_sim.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/iterative_sim.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/iterative_sim.rs.review.md` | `Reviewed` |
| 62 | `crates/app_core/src/kernels/double_crossover.cu` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/double_crossover.cu.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kernels/double_crossover.cu.review.md` | `Reviewed` |
| 63 | `crates/app_core/src/kernels/options_expression.cu` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/options_expression.cu.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kernels/options_expression.cu.review.md` | `Reviewed` |
| 64 | `crates/app_core/src/kqsr_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kqsr_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kqsr_gpu_kernel.rs.review.md` | `Reviewed` |
| 65 | `crates/app_core/src/kqsr.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kqsr.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kqsr.rs.review.md` | `Reviewed` |
| 66 | `crates/app_core/src/kvdf_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kvdf_gpu_kernel.rs.review.md` | `Reviewed` |
| 67 | `crates/app_core/src/kvdf.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kvdf.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kvdf.rs.review.md` | `Reviewed` |
| 68 | `crates/app_core/src/lib.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/lib.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/lib.rs.review.md` | `Reviewed` |
| 69 | `crates/app_core/src/ma_policy.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ma_policy.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ma_policy.rs.review.md` | `Reviewed` |
| 70 | `crates/app_core/src/matrix_index.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/matrix_index.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/matrix_index.rs.review.md` | `Reviewed` |
| 71 | `crates/app_core/src/meta_select.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/meta_select.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/meta_select.rs.review.md` | `Reviewed` |
| 72 | `crates/app_core/src/mkcs_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/mkcs_gpu_kernel.rs.review.md` | `Reviewed` |
| 73 | `crates/app_core/src/mkcs.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/mkcs.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/mkcs.rs.review.md` | `Reviewed` |
| 74 | `crates/app_core/src/ml_confidence.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ml_confidence.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ml_confidence.rs.review.md` | `Reviewed` |
| 75 | `crates/app_core/src/ml_signal.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ml_signal.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ml_signal.rs.review.md` | `Reviewed` |
| 76 | `crates/app_core/src/nvtx.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/nvtx.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/nvtx.rs.review.md` | `Reviewed` |
| 77 | `crates/app_core/src/options_data_store.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_data_store.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_data_store.rs.review.md` | `Reviewed` |
| 78 | `crates/app_core/src/options_expression_gpu.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_expression_gpu.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_expression_gpu.rs.review.md` | `Reviewed` |
| 79 | `crates/app_core/src/options_expression.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_expression.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_expression.rs.review.md` | `Reviewed` |
| 80 | `crates/app_core/src/options_signal_manifest.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_signal_manifest.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_signal_manifest.rs.review.md` | `Reviewed` |
| 81 | `crates/app_core/src/options_types.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_types.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_types.rs.review.md` | `Reviewed` |
| 82 | `crates/app_core/src/prescan_predictive.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/prescan_predictive.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/prescan_predictive.rs.review.md` | `Reviewed` |
| 83 | `crates/app_core/src/progress.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/progress.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/progress.rs.review.md` | `Reviewed` |
| 84 | `crates/app_core/src/request_normalization.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/request_normalization.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/request_normalization.rs.review.md` | `Reviewed` |
| 85 | `crates/app_core/src/rocb_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/rocb_gpu_kernel.rs.review.md` | `Reviewed` |
| 86 | `crates/app_core/src/rocb.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/rocb.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/rocb.rs.review.md` | `Reviewed` |
| 87 | `crates/app_core/src/sharpe_stats.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/sharpe_stats.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/sharpe_stats.rs.review.md` | `Reviewed` |
| 88 | `crates/app_core/src/standalone_gpu_runtime.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md` | `Reviewed` |
| 89 | `crates/app_core/src/stats_common.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/stats_common.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/stats_common.rs.review.md` | `Reviewed` |
| 90 | `crates/app_core/src/strategy_dynamic.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_dynamic.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_dynamic.rs.review.md` | `Reviewed` |
| 91 | `crates/app_core/src/strategy_gpu_program.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_program.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_gpu_program.rs.review.md` | `Reviewed` |
| 92 | `crates/app_core/src/strategy_gpu_runtime.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_runtime.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_gpu_runtime.rs.review.md` | `Reviewed` |
| 93 | `crates/app_core/src/strategy_ir.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_ir.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_ir.rs.review.md` | `Reviewed` |
| 94 | `crates/app_core/src/strategy_plan.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_plan.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_plan.rs.review.md` | `Reviewed` |
| 95 | `crates/app_core/src/strategy_presets.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_presets.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/strategy_presets.rs.review.md` | `Reviewed` |
| 96 | `crates/app_core/src/test_support.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/test_support.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/test_support.rs.review.md` | `Reviewed` |
| 97 | `crates/app_core/src/timeframe.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/timeframe.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/timeframe.rs.review.md` | `Reviewed` |
| 98 | `crates/app_core/src/validation_core.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_core.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_core.rs.review.md` | `Reviewed` |
| 99 | `crates/app_core/src/validation_pipeline.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_pipeline.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_pipeline.rs.review.md` | `Reviewed` |
| 100 | `crates/app_core/src/validation_shared.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_shared.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_shared.rs.review.md` | `Reviewed` |
| 101 | `crates/app_core/src/validation_stats.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_stats.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_stats.rs.review.md` | `Reviewed` |
| 102 | `crates/app_core/src/vcc_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcc_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcc_gpu_kernel.rs.review.md` | `Reviewed` |
| 103 | `crates/app_core/src/vcc.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcc.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcc.rs.review.md` | `Reviewed` |
| 104 | `crates/app_core/src/vcet_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcet_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcet_gpu_kernel.rs.review.md` | `Reviewed` |
| 105 | `crates/app_core/src/vcet.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcet.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcet.rs.review.md` | `Reviewed` |
| 106 | `crates/app_core/src/vemr_gpu_kernel.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vemr_gpu_kernel.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vemr_gpu_kernel.rs.review.md` | `Reviewed` |
| 107 | `crates/app_core/src/vemr.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vemr.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vemr.rs.review.md` | `Reviewed` |
| 108 | `crates/app_core/src/vram_ma.rs` | `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vram_ma.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vram_ma.rs.review.md` | `Reviewed` |

### `crates/optimizer/src`

| # | Source file | Second-pass packet | Third-pass packet | Status |
| --- | --- | --- | --- | --- |
| 109 | `crates/optimizer/src/grid.rs` | `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/grid.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/grid.rs.review.md` | `Reviewed` |
| 110 | `crates/optimizer/src/lib.rs` | `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/lib.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/lib.rs.review.md` | `Reviewed` |
| 111 | `crates/optimizer/src/mo.rs` | `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/mo.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/mo.rs.review.md` | `Reviewed` |
| 112 | `crates/optimizer/src/stream.rs` | `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/stream.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/stream.rs.review.md` | `Reviewed` |

### `crates/strategies/src`

| # | Source file | Second-pass packet | Third-pass packet | Status |
| --- | --- | --- | --- | --- |
| 113 | `crates/strategies/src/composite_cpu/mod.rs` | `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/composite_cpu/mod.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/strategies/src/composite_cpu/mod.rs.review.md` | `Reviewed` |
| 114 | `crates/strategies/src/double_ma/mod.rs` | `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/double_ma/mod.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/strategies/src/double_ma/mod.rs.review.md` | `Reviewed` |
| 115 | `crates/strategies/src/lib.rs` | `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/lib.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/strategies/src/lib.rs.review.md` | `Reviewed` |
| 116 | `crates/strategies/src/records.rs` | `Repo Review/backend-file-reviews-second-pass/crates/strategies/src/records.rs.review.md` | `Repo Review/backend-file-reviews-third-pass/crates/strategies/src/records.rs.review.md` | `Reviewed` |

## Completion Notes

- The expanded third pass is now repo-wide across the full second-pass corpus of `116` files.
- Existing high-ROI packets remain valid; the expansion adds carry-forward packets for the rest and upgrades buckets only where the source justified it.
- The third-pass packet count should equal `116`.
