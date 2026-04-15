# Backend GPT-5.4 Pro Evaluation Plan

Last updated: `2026-04-14`

## Purpose

This document plans a final adversarial GPT-5.4 Pro review pass on top of the completed third-pass backend review corpus. It is meant to be comprehensive: every third-pass packet in `Repo Review/backend-file-reviews-third-pass/` is assigned a GPT-5.4 Pro evaluation tier so the sweep can expand well beyond the original high-ROI shortlist without degenerating into an unstructured all-116-file rerun.

## Recommended Scope

Recommended current GPT-5.4 Pro sweep: `Tier 1 + Tier 2`.

- `Tier 1`: must-review files where the current packet is high-blast-radius, bucket-shifted, durability-sensitive, validation-sensitive, or GPU ABI/runtime-sensitive.
- `Tier 2`: broad-review files that still have nontrivial contract value, but are less likely than Tier 1 to overturn the current packet.
- `Tier 3`: lower-yield files to hold unless the scope intentionally widens to all `116` files.

Current counts:
- Tier 1: `47` files
- Tier 2: `52` files
- Tier 3: `17` files
- Recommended current GPT-5.4 Pro set: `99` files
- Full third-pass corpus: `116` files

## Tier Rules

- Put a file in `Tier 1` if any of these are true: the third-pass bucket is `Larger refactor but defer`, the third-pass bucket is `Benchmark-sensitive; do not touch yet`, the file was in the targeted third-pass set, the file had a bucket change across passes, or the third pass found a concrete high-risk contract/persistence/runtime/GPU-ABI issue that deserves adversarial pressure-testing.
- Put a file in `Tier 2` if the current packet is still materially actionable but is lower-risk than Tier 1. In practice this means most remaining `Small surgical refactor` packets plus a few `Document only` shared-contract surfaces.
- Put a file in `Tier 3` if the current packet is `Accept as-is`, a low-yield `Document only`, or a thin wrapper / re-export / support seam whose likely GPT-5.4 Pro upside is limited relative to the noise risk.

## Review Bundle Standard

For each file sent to GPT-5.4 Pro, attach:
- the live source file
- the current third-pass review packet
- the second-pass packet when the bucket changed, the current packet is subtle, or the third-pass packet explicitly says it tightened earlier wording
- only the 1-3 adjacent owner files that are necessary to understand the contract

## Prompt Standard

Use this prompt skeleton for each GPT-5.4 Pro review:

```text
Review the attached source file and attached review packet as a final adversarial pass.

Your task:
1. Identify anything the review packet still missed.
2. Identify any claims that are overstated, weakly supported, or likely to cause regressions if implemented.
3. Distinguish:
   - safe-now hardening
   - larger redesign still deferred
4. If the current action bucket is wrong, say so and explain why.
5. Focus on real contract, edge-case, performance, and compatibility risks.
6. Do not restate the packet. Challenge it.
7. Be conservative: preserve current behavior unless a bug or contract weakness is clear.
```

## File Inventory

Every third-pass packet appears exactly once below.

## Tier 1

Files to send to GPT-5.4 Pro in the first wave. These are the highest-value adversarial-review targets and should be treated as the mandatory set if the scope is still short of all `116` files.

Count: `47`

### `src-tauri/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `src-tauri/src/commands_analysis.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 2 | `src-tauri/src/commands_campaign.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 3 | `src-tauri/src/commands_data.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 4 | `src-tauri/src/commands_execution.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 5 | `src-tauri/src/commands_meta.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 6 | `src-tauri/src/commands_ml_signal.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 7 | `src-tauri/src/commands_options_data.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 8 | `src-tauri/src/commands_options_expression.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 9 | `src-tauri/src/commands_reports_api.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 10 | `src-tauri/src/commands_reports_shared.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 11 | `src-tauri/src/commands_reports_validation_common.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 12 | `src-tauri/src/commands_validation.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 13 | `src-tauri/src/run_artifacts.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 14 | `src-tauri/src/validation_common.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |

### `crates/app_core/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/app_core/src/alloc_policy.rs` | `Larger refactor but defer` | current packet is larger-refactor/defer |
| 2 | `crates/app_core/src/cdmc.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 3 | `crates/app_core/src/composite_profile_route.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file; third-pass bucket changed |
| 4 | `crates/app_core/src/composite_validate.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; third-pass bucket changed; current packet is larger-refactor/defer |
| 5 | `crates/app_core/src/cuda_device.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 6 | `crates/app_core/src/double_ma_validate.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 7 | `crates/app_core/src/double_ma.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 8 | `crates/app_core/src/gpu_backtest_kernel.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 9 | `crates/app_core/src/iterative_sim.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 10 | `crates/app_core/src/kernels/double_crossover.cu` | `Benchmark-sensitive; do not touch yet` | previous targeted or custom high-risk third-pass file; current packet is benchmark-sensitive |
| 11 | `crates/app_core/src/ml_confidence.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 12 | `crates/app_core/src/options_data_store.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 13 | `crates/app_core/src/options_expression_gpu.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 14 | `crates/app_core/src/options_expression.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 15 | `crates/app_core/src/options_signal_manifest.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 16 | `crates/app_core/src/rocb_gpu_kernel.rs` | `Benchmark-sensitive; do not touch yet` | current packet is benchmark-sensitive |
| 17 | `crates/app_core/src/sharpe_stats.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file; third-pass bucket changed |
| 18 | `crates/app_core/src/strategy_dynamic.rs` | `Larger refactor but defer` | current packet is larger-refactor/defer |
| 19 | `crates/app_core/src/strategy_gpu_program.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 20 | `crates/app_core/src/strategy_gpu_runtime.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |
| 21 | `crates/app_core/src/strategy_presets.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file; third-pass bucket changed |
| 22 | `crates/app_core/src/validation_pipeline.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 23 | `crates/app_core/src/validation_stats.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file; third-pass bucket changed |
| 24 | `crates/app_core/src/vcc_gpu_kernel.rs` | `Benchmark-sensitive; do not touch yet` | current packet is benchmark-sensitive |
| 25 | `crates/app_core/src/vcc.rs` | `Larger refactor but defer` | current packet is larger-refactor/defer |
| 26 | `crates/app_core/src/vcet_gpu_kernel.rs` | `Benchmark-sensitive; do not touch yet` | current packet is benchmark-sensitive |
| 27 | `crates/app_core/src/vemr_gpu_kernel.rs` | `Benchmark-sensitive; do not touch yet` | current packet is benchmark-sensitive |
| 28 | `crates/app_core/src/vemr.rs` | `Larger refactor but defer` | current packet is larger-refactor/defer |

### `crates/optimizer/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/optimizer/src/mo.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 2 | `crates/optimizer/src/stream.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |

### `crates/strategies/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/strategies/src/composite_cpu/mod.rs` | `Larger refactor but defer` | current packet is larger-refactor/defer |
| 2 | `crates/strategies/src/double_ma/mod.rs` | `Larger refactor but defer` | previous targeted or custom high-risk third-pass file; current packet is larger-refactor/defer |
| 3 | `crates/strategies/src/records.rs` | `Small surgical refactor` | previous targeted or custom high-risk third-pass file |


## Tier 2

Files to include in the current broadened GPT-5.4 Pro sweep after Tier 1. These are still worth reviewing now because the current packet is active and nontrivial, but they are less likely than Tier 1 to contain a bucket-level surprise.

Count: `52`

### `src-tauri/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `src-tauri/src/commands_optimize.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 2 | `src-tauri/src/commands_reports_csv.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 3 | `src-tauri/src/commands_reports_meta_xlsx.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 4 | `src-tauri/src/commands_reports_validation_xlsx.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 5 | `src-tauri/src/commands_reports.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 6 | `src-tauri/src/data_cache.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 7 | `src-tauri/src/settings.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 8 | `src-tauri/src/state.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |

### `crates/app_core/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/app_core/src/asr_gpu_kernel.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 2 | `crates/app_core/src/asr.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 3 | `crates/app_core/src/carm_gpu_kernel.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 4 | `crates/app_core/src/carm.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 5 | `crates/app_core/src/cdmc_gpu_kernel.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 6 | `crates/app_core/src/composite_cpu.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 7 | `crates/app_core/src/composite_gpu_metric_decode.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 8 | `crates/app_core/src/composite_mo.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 9 | `crates/app_core/src/cscv_pbo.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 10 | `crates/app_core/src/csv_compat.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 11 | `crates/app_core/src/double_ma_backend.rs` | `Document only` | document-only, but shared contract/default surface |
| 12 | `crates/app_core/src/double_ma_mode.rs` | `Document only` | document-only, but shared contract/default surface |
| 13 | `crates/app_core/src/double_ma_result.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 14 | `crates/app_core/src/double_ma_strategy.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 15 | `crates/app_core/src/double_ma_vdps.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 16 | `crates/app_core/src/dynamic_indicator_adapter.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 17 | `crates/app_core/src/indicator_catalog.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 18 | `crates/app_core/src/indicator_factory.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 19 | `crates/app_core/src/kernels/options_expression.cu` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 20 | `crates/app_core/src/kqsr_gpu_kernel.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 21 | `crates/app_core/src/kqsr.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 22 | `crates/app_core/src/kvdf_gpu_kernel.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 23 | `crates/app_core/src/kvdf.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 24 | `crates/app_core/src/lib.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 25 | `crates/app_core/src/ma_policy.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 26 | `crates/app_core/src/matrix_index.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 27 | `crates/app_core/src/meta_select.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 28 | `crates/app_core/src/mkcs_gpu_kernel.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 29 | `crates/app_core/src/mkcs.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 30 | `crates/app_core/src/ml_signal.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 31 | `crates/app_core/src/options_types.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 32 | `crates/app_core/src/prescan_predictive.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 33 | `crates/app_core/src/progress.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 34 | `crates/app_core/src/request_normalization.rs` | `Document only` | document-only, but shared contract/default surface |
| 35 | `crates/app_core/src/rocb.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 36 | `crates/app_core/src/standalone_gpu_runtime.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 37 | `crates/app_core/src/strategy_ir.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 38 | `crates/app_core/src/strategy_plan.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 39 | `crates/app_core/src/timeframe.rs` | `Document only` | document-only, but shared contract/default surface |
| 40 | `crates/app_core/src/validation_core.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 41 | `crates/app_core/src/validation_shared.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 42 | `crates/app_core/src/vcet.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |
| 43 | `crates/app_core/src/vram_ma.rs` | `Document only` | document-only, but shared contract/default surface |

### `crates/optimizer/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/optimizer/src/grid.rs` | `Small surgical refactor` | active small-surgical packet on a nontrivial contract surface |


## Tier 3

Files to hold unless the scope deliberately widens to a full all-files GPT-5.4 Pro pass. They are still listed here so nothing is omitted from planning, but they are the lowest-yield tranche.

Count: `17`

### `src-tauri/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `src-tauri/src/commands_artifacts.rs` | `Accept as-is` | accept-as-is packet |
| 2 | `src-tauri/src/commands_dynamic.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 3 | `src-tauri/src/commands_system.rs` | `Document only` | low-yield thin wrapper, re-export, or support seam |
| 4 | `src-tauri/src/commands.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 5 | `src-tauri/src/lib.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 6 | `src-tauri/src/ma_registry.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 7 | `src-tauri/src/main.rs` | `Accept as-is` | accept-as-is packet |
| 8 | `src-tauri/src/metric_surface.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 9 | `src-tauri/src/options_parent_adapters.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 10 | `src-tauri/src/polygon.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |

### `crates/app_core/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/app_core/src/double_ma_ma_kind.rs` | `Accept as-is` | accept-as-is packet |
| 2 | `crates/app_core/src/double_ma_test_support.rs` | `Document only` | low-yield thin wrapper, re-export, or support seam |
| 3 | `crates/app_core/src/nvtx.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |
| 4 | `crates/app_core/src/stats_common.rs` | `Document only` | document-only, lower-yield packet |
| 5 | `crates/app_core/src/test_support.rs` | `Small surgical refactor` | low-yield thin wrapper, re-export, or support seam |

### `crates/optimizer/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/optimizer/src/lib.rs` | `Document only` | low-yield thin wrapper, re-export, or support seam |

### `crates/strategies/src`

| # | Source file | Third-pass bucket | Why this tier |
| --- | --- | --- | --- |
| 1 | `crates/strategies/src/lib.rs` | `Document only` | low-yield thin wrapper, re-export, or support seam |


## Execution Notes

- Run Tier 1 and Tier 2 in parallel, but normalize the outputs back into this repo with a single style: challenge the packet, call out regression risk, and separate safe-now hardening from deferred redesign.
- If GPT-5.4 Pro disagrees with a current packet, prefer explicit evidence over model confidence. The point of this pass is to challenge the packet, not to automatically replace it.
- If Tier 3 is later pulled in, keep it as a distinct expansion phase so the lower-yield files do not obscure the higher-value review outcomes.
