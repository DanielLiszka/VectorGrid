# Validation Throughput Plan Disposition Matrix

Audit date: 2026-03-22

This file records how `validation_throughput_cuda_plan.md` and `validation_throughput_cuda_plan_addendum.md` map to the current repo state.

Disposition buckets:
- `Kept in prompt`: still-open work that remains valid and should be implemented.
- `Already implemented`: the repo already has this behavior or supporting surface; the prompt should preserve/reuse it rather than re-implement it.
- `Stale/removed`: the original plan item should not be treated as active required work in the current repo state.
- `Optional/deferred`: valid ideas, but not part of the required implementation scope for the current prompt.

## Implementation Checklist

1. Status: Done
   Scope: Land canonical identity fixes, aux-work masking, finalist-series reuse, scratch reuse, alignment/dedupe, and validation telemetry/reporting surfaces.

2. Status: Done
   Scope: Route `GpuKernel` fixed-candidate validation through exact-combo execution for full-range validation.

3. Status: Done
   Scope: Route `GpuKernel` fixed-candidate validation through exact-combo execution for ranged walk-forward / CSCV / SPA / reopt validation.

4. Status: Done
   Scope: Remove the optimizer-shaped full-range `GpuHybridKernel` validation detour.

5. Status: Done
   Scope: Batch `GpuKernel` validation ranges through one prepared execution instead of re-entering the GPU path once per range.
   Status note: implemented by routing dense ranged validation through one prepared exact-combo multi-range kernel execution in `crates/app_core/src/double_ma_validate.rs` and `crates/app_core/src/gpu_backtest_kernel.rs`.

6. Status: Done
   Scope: Add the equivalent exact-combo ranged validation path for the `GpuHybridKernel` route.
   Status note: ranged hybrid validation now uses `validate_double_ma_gpu_hybrid_kernel_exact_combos(...)` in `crates/app_core/src/double_ma.rs`, preserving fallback behavior while avoiding grid-shaped ranged execution.

7. Status: Done
   Scope: Update validation execution-profile telemetry and cost/bench outputs once items 5 and 6 land so batching and hybrid exact-combo work shape are reflected truthfully end to end.
   Status note: sparse-finalist probes now report batched ranged execution truthfully, including `gpu_range_eval_calls=5` versus `gpu_range_count=105` on the CUDA workflow bench.

8. Status: Done
   Scope: Add focused parity, route/fallback, and throughput coverage for items 5 and 6.
   Status note: focused CUDA tests now assert batched ranged-call counts on both the `GpuKernel` and `GpuHybridKernel` validation routes.

## Kept in Prompt

- No remaining repo-valid required checklist items are open in this bundle. The primary remaining concern is environment-dependent perf-gate interpretation, which is tracked separately from implementation closure.

## Already Implemented

- Sparse finalist order is preserved through the host-side validation plan and dense range materialization in `crates/app_core/src/double_ma_validate.rs`.

- `GpuKernel` fixed-candidate validation now uses the explicit-combo batch kernel path in `crates/app_core/src/double_ma_validate.rs` and `crates/app_core/src/gpu_backtest_kernel.rs` for both full-range and ranged walk-forward / CSCV / SPA / reopt passes, so sparse finalists no longer expand into a device-side grid on that route.

- Dense combo-indexed range metrics now back GPU walk-forward, CSCV, SPA, and reopt-walk-forward consumption in `crates/app_core/src/double_ma_validate.rs`, and ranged `GpuKernel` validation now batches the full range schedule through one prepared execution instead of re-entering once per range.

- Serialization-based equality/keying has been replaced with canonical JSON-key ordering in `crates/app_core/src/double_ma_validate.rs`, and the same canonical identity now backs reopt/SPA keys.

- Shared GPU start-gate precompute now exists in `crates/app_core/src/double_ma_validate.rs`, and candidate warmup uses `candidate_start_t_from_shared(...)` rather than recomputing shared regime/Kaufman prerequisites per candidate.

- Validation execution-profile telemetry is now truthful about current backend work shape in `crates/app_core/src/double_ma_validate.rs`, `src-tauri/src/bin/ta_double_ma_validation_cost.rs`, and `crates/app_core/benches/double_ma_validation_workflow.rs`. Sparse-finalist route probes now report the actual grid work on device rather than over-claiming exact-combo execution.

- The optimizer-shaped full-range `GpuHybridKernel` validation detour has been removed from `crates/app_core/src/double_ma_validate.rs`; the hybrid validation path now uses the ranged hybrid entry for full-range execution too.

- Ranged `GpuHybridKernel` validation now uses exact-combo execution via `validate_double_ma_gpu_hybrid_kernel_exact_combos(...)` in `crates/app_core/src/double_ma.rs`, preserving the route/fallback policy while avoiding grid-shaped ranged validation work.

- Aux-work masking now exists in `crates/app_core/src/double_ma_validate.rs`, and preserved-finalists cross reruns are wired through it from `src-tauri/src/commands_validation.rs`.

- Friction-stress series/state reuse now exists in `crates/app_core/src/double_ma_validate.rs` via finalist-series caching plus `eval_metrics_window_with_cfg(...)`.

- Trade-bootstrap and SPA scratch reuse now exists in `crates/app_core/src/validation_pipeline.rs`.

- Borrowed/no-copy cross-ticker alignment now exists in `src-tauri/src/commands_validation.rs`.

- Backend normalized cross-dataset dedupe now exists in `src-tauri/src/validation_common.rs`.

- Frontend candidate dedupe and normalized cross-dataset dedupe now exist in `web/src/services/validationRequestBuilder.ts`, with coverage in `web/src/services/validationRequestBuilder.test.ts`.

- Validation-cost telemetry extension now exists in `src-tauri/src/bin/ta_double_ma_validation_cost.rs` via execution-profile summary output.

- Sparse-finalist workflow-bench evidence now exists in `crates/app_core/benches/double_ma_validation_workflow.rs`, including CUDA-gated sparse-finalist route probes that print execution-profile work-shape fields.

- Canonical candidate keys already exist on the Tauri/common side. `src-tauri/src/validation_common.rs` already provides `candidate_key_canonical(...)` plus canonical JSON ordering logic, and `src-tauri/src/commands_validation.rs` already uses it for cross-ticker matching.

- Preserved-finalists cross-rerun request shaping is already present. `src-tauri/src/commands_validation.rs` already strips walk-forward, reopt walk-forward, SPA, perturb, and trade bootstrap from preserved-finalists reruns while keeping the preserved CSCV/PBO + hard-gate stack.

- Preserved-finalists frontend defaulting and current summary wording are already present. `web/src/services/validationRequestBuilder.ts` already defaults double-like cross mode to `preserved_finalists` and already summarizes the current omission set; existing assertions live in `web/src/services/validationRequestBuilder.test.ts`.

- The primary validation-cost harness already exists. `src-tauri/src/bin/ta_double_ma_validation_cost.rs` already reports coarse timing, route/fallback, candidate counts, and module-feasibility summaries; the required work is to extend it, not rebuild it.

- Workflow benches already exist. `crates/app_core/benches/double_ma_validation_workflow.rs` and `crates/app_core/benches/composite_validation_workflow.rs` are already established coverage surfaces and should be extended only where needed.

- Broad parity/fallback test coverage already exists in the core validation surface. `crates/app_core/src/double_ma_validate.rs` already contains substantial CPU/GPU parity and route/fallback tests for in-sample, walk-forward, CSCV, SPA, reopt, perturb, bootstrap, and hybrid-route behavior.

## Stale/Removed

- Do not treat the plan docs as unconditional source of truth. The live repo state now overrides stale plan assumptions, especially on Tauri/frontend request shaping and reporting surfaces.

- Do not re-implement a second Tauri-side canonical-key path. That work already landed in `src-tauri/src/validation_common.rs`, so the prompt should preserve/reuse it instead of duplicating it.

- Do not treat preserved-finalists summary churn as required work before backend aux-mask behavior actually changes. The current summary in `web/src/services/validationRequestBuilder.ts` matches the current backend behavior; the older plan wording about immediately extending it to regime/friction omissions is premature until aux masking lands.

- Do not require rebuilding the validation-cost harness or workflow benches from scratch. Those surfaces already exist and only need compatible extensions.

- Do not require a new broad composite sparse-finalist benchmark matrix as part of the core prompt. `crates/app_core/benches/composite_validation_workflow.rs` should only be touched if it directly helps shared telemetry visibility or shared GPU validation route coverage.

- Do not expand this bundle into repo-wide identity cleanup. Similar serialization-keying patterns may exist elsewhere, but this prompt should stay scoped to Double-MA validation and its immediate helpers.

## Optional/Deferred

- `BTreeSet` to `Vec + sort_unstable + dedup` replacement is a valid micro-optimization, but it should stay opportunistic and measured rather than a hard checklist requirement.

- Compact validation DTOs, pinned host staging, async H2D/D2H copies, persistent device scratch pools, CUDA Graph capture, and device-resident prepared-plan caches remain optional backend-adjacent follow-ons. They may help later, but they are not required to implement the current valid core plan.

- A larger borrowed `CandlesView<'a>` refactor remains optional. The current valid requirement is the local no-copy alignment improvement, not a broad signature rewrite.

- Deterministic CPU-tail parallelism remains optional and should wait until caching/reuse work is in place and measured.

- Stable candidate IDs generated once at request ingress remain optional. The current prompt only needs stable/canonical identity where the current validation surface actually requires it.

## Prompt Coverage Note

- `validation_review/codex_validation_throughput_implementation_prompt.md` is now best treated as the historical implementation brief for the work that has landed. This matrix is the current source of truth for closure status.

- If future repo changes reopen or extend this area, update both the prompt and this matrix so the prompt continues to describe only still-open valid work.
