# Codex implementation prompt — validation throughput / CUDA optimization / fallback behavior

Status note: the repo-valid implementation scope captured by this prompt has now been landed. Keep this file as the historical implementation brief, and use `validation_throughput_plan_disposition_matrix.md` as the live closure/status document.

Use the following prompt verbatim with a Codex agent that has access to the repository and the referenced guide documents.

```text
You are an implementation agent working inside my Rust/Tauri/TypeScript repository.

Your task is to implement the remaining repo-valid portions of the validation-throughput / CUDA-optimization / fallback-behavior plan described in these documents:

1. `gpt54_feature_review_map.md`
   - Use the rules from:
     - `## 4. Validation throughput and fallback behavior`
     - `## Prompt rules for GPT-5.4`
     - `## 16. Allocation policy, prefix initialization, and copy-elision discipline`
2. `validation_throughput_cuda_plan.md`
3. `validation_throughput_cuda_plan_addendum.md`
4. `validation_throughput_plan_disposition_matrix.md`

These documents are planning inputs for scope and priorities, but the current repository state overrides them where code has already landed or where a plan assumption is now stale.
If a checklist item is already implemented, duplicated elsewhere, or depends on telemetry/runtime hooks that do not exist yet, prefer the current repo state and implement only the still-valid portion.
Use `validation_throughput_plan_disposition_matrix.md` as the explicit keep/remove/defer audit when the plan docs and live code appear to disagree.

--------------------------------
PRIMARY OBJECTIVE
--------------------------------

Implement the still-valid performance/correctness plan for the Double-MA validation surface.

The goals are:
- improve validation throughput, especially for CUDA-executed code
- eliminate validation-stage wasted work and rerun amplification
- reduce unnecessary CPU/GPU memory copies and host allocations
- use uninitialized memory only where safe and only via shared allocation helpers when possible
- improve fallback transparency and benchmark fidelity
- preserve correctness, parity, determinism, and feature coverage
- do not remove features or break functionality

This is an implementation task, not a review task. Make code changes, tests, and benchmark/reporting changes. Do not stop at writing notes.

--------------------------------
REQUIRED READING ORDER
--------------------------------

Read these first before changing code:

1. `gpt54_feature_review_map.md`
   - internalize the bundle-4 ownership/scope
   - internalize the prompt rules and the allocation/copy-elision guidance
2. `validation_throughput_cuda_plan.md`
   - this is the main implementation plan
3. `validation_throughput_cuda_plan_addendum.md`
   - use sections 1–6 as correction material for the main plan
   - do not treat sections 7–8 as required work unless they are directly needed to unblock the core implementation
4. `validation_throughput_plan_disposition_matrix.md`
   - this is the explicit disposition audit for kept vs already-landed vs stale vs deferred work
5. then inspect the repo code starting from the files listed below

--------------------------------
CURRENT-STATE CORRECTIONS YOU MUST APPLY BEFORE CODING
--------------------------------

The plan docs predate some Tauri/frontend/request-shaping changes already present in the repo. Treat the live code as authoritative:

1. `src-tauri/src/validation_common.rs` already has canonical candidate key support via `candidate_key_canonical(...)`.
   - Reuse and preserve that helper rather than re-implementing a second Tauri-side canonical-key path.

2. `src-tauri/src/commands_validation.rs` already shapes Double-MA preserved-finalists cross reruns to keep the verdict-relevant stack only.
   - Preserve the current behavior that strips walk-forward, reopt walk-forward, SPA, perturb, and trade bootstrap while retaining the preserved-finalists CSCV/PBO + hard-gate path.
   - Existing preserved-finalists GPU route-preservation tests should continue to pass.

3. `web/src/services/validationRequestBuilder.ts` already defaults double-like cross mode to `preserved_finalists` and already summarizes the current preserved-finalists omissions.
   - Do not churn that summary text unless backend behavior actually changes.

4. The still-open Tauri/frontend/request-shaping items are narrower:
   - no-copy borrowed alignment in `src-tauri/src/commands_validation.rs`
   - normalized cross-dataset dedupe in `src-tauri/src/validation_common.rs`
   - candidate dedupe in `web/src/services/validationRequestBuilder.ts`
   - normalized dataset dedupe in `web/src/services/validationRequestBuilder.ts`
   - targeted tests for those remaining gaps

--------------------------------
FILES TO MODIFY (REQUIRED)
--------------------------------

Treat these as the required implementation surface unless a listed item is already implemented and only needs to be preserved after code inspection:

Core app-core / Tauri / frontend files:
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`

Benchmark / measurement files:
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs` only if needed for shared telemetry visibility or shared GPU validation route coverage

Measurement notes:
- `ta_double_ma_validation_cost.rs` already reports coarse timing, route, fallback, candidate-count, and module-feasibility data. Extend that surface; do not rebuild it from scratch.
- The workflow benches already exist. Extend them only where needed for repo-valid telemetry/work-shape coverage.

--------------------------------
ADJACENT FILES YOU ARE ALLOWED / EXPECTED TO MODIFY IF NEEDED
--------------------------------

If exact-combo CUDA validation, multi-range batching, copy-elision, or safe uninitialized allocation cannot be completed cleanly inside the required files alone, inspect and modify the immediate backend/runtime helpers as needed.

Likely adjacent files include:
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/alloc_policy.rs`
- any immediate helper/backend/runtime file called by `gpu_fixed_candidate_metric_map(...)`
- any immediate helper/backend/runtime file used by the current GPU Double-MA validation/optimization path

Do not make broad architecture rewrites. Stay adjacent to the current path.

--------------------------------
MAPPING FROM ATTACHED FILENAMES TO REPO PATHS
--------------------------------

If you were given loose filenames rather than repo-relative paths, map them like this:
- `double_ma_validate.rs` -> `crates/app_core/src/double_ma_validate.rs`
- `validation_pipeline.rs` -> `crates/app_core/src/validation_pipeline.rs`
- `commands_validation.rs` -> `src-tauri/src/commands_validation.rs`
- `validation_common.rs` -> `src-tauri/src/validation_common.rs`
- `validationRequestBuilder.ts` -> `web/src/services/validationRequestBuilder.ts`
- `validationRequestBuilder.test.ts` -> `web/src/services/validationRequestBuilder.test.ts`
- `ta_double_ma_validation_cost.rs` -> `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `double_ma_validation_workflow.rs` -> `crates/app_core/benches/double_ma_validation_workflow.rs`
- `composite_validation_workflow.rs` -> `crates/app_core/benches/composite_validation_workflow.rs`

--------------------------------
NON-NEGOTIABLE CONSTRAINTS
--------------------------------

1. Preserve functionality.
   - Do not remove validation modules or change user-visible semantics unless the change only makes the system more truthful about actual route/work behavior.
   - Keep walk-forward, CSCV/PBO, SPA, perturb, trade bootstrap, cross-ticker, regime robustness, and friction stress features intact.

2. Preserve parity/correctness.
   - CPU and GPU results must remain within current parity tolerances.
   - Deterministic seeded paths must remain deterministic.
   - Do not silently change precision-sensitive logic.

3. Preserve route/fallback behavior unless the route itself is being correctly improved.
   - Keep fallback reasons explicit.
   - Keep non-CUDA paths working.
   - Maintain `#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]` boundaries.

4. Prefer smaller parity-safe changes before larger refactors.
   - Follow the phase ordering in the plan.

5. Minimize copies and allocations.
   - Prefer references/borrows over clones.
   - Prefer dense row-major buffers over nested hash maps when order is known.
   - Prefer shared scratch reuse over per-iteration allocation.
   - Prefer no-op borrowed views when alignment/slicing does not change the full range.

6. Only use uninitialized allocation when every element is guaranteed to be written before any read.
   - Prefer shared helpers in `crates/app_core/src/alloc_policy.rs`.
   - Keep a full-fill allocation where the fill value is observable behavior.
   - Annotate any unsafe/uninitialized path with a precise overwrite-before-read safety invariant.

7. Keep backward compatibility where reasonable.
   - If adding telemetry / profile fields to results, make them optional and serde-friendly.
   - Do not break existing request/response callers.

8. Do not add broad new product features, new strategies, new workflow panels, or speculative redesigns.

9. Keep benchmark claims on the correct timing boundary.
   - Use `ta_double_ma_validation_cost.rs` as the primary proof boundary for these validation changes.
   - Keep Criterion benches as supplementary evidence.

--------------------------------
IMPLEMENTATION PRIORITY
--------------------------------

Implement the core plan in this order:

Required core implementation:
1. Phase 0 — instrumentation and truthfulness
2. Phase 1 — remove hidden work and redundant CPU prep
3. Phase 2 — exact-combo host reshaping and dense result consumption
4. Phase 3 — exact-combo CUDA backend path
5. Phase 4 — multi-range batching
6. Phase 5 — CPU-tail shrinkage
7. Phase 6 — request shaping and frontend alignment
8. Remaining addendum items from sections 1–6 of `validation_throughput_cuda_plan_addendum.md` that are still missing after repo inspection

Do not spend this task on addendum sections 7–8 unless one of them is the minimum direct change needed to unblock the core path above.

--------------------------------
EXPLICIT REQUIRED CHANGES BY AREA
--------------------------------

==================================================
A. `crates/app_core/src/double_ma_validate.rs`
==================================================

This file is the center of the work.

Required changes:

1. Add execution/work-shape telemetry.
   - Add a `ValidationExecutionProfile` type.
   - Thread an optional `execution_profile` (or equivalent backward-compatible field) into the relevant validation result type.
   - Record at least these fields where knowable:
     - requested candidate count
     - unique candidate count
     - unique fast period count
     - unique slow period count
     - evaluated pairs on device
     - grid expansion ratio (`evaluated_pairs_on_device / unique_candidates`) when meaningful
     - GPU range-eval call count
     - range batch count / total range count
     - aux finalist analytics wall time
     - cpu-tail wall time if separable
     - GPU result materialization / host shaping time if separable
   - Keep field names consistent and serialization-friendly.

2. Add aux-work masking for post-pass finalist analytics.
   - Add `ValidationAuxWorkMask` (or equivalent) with at least:
     - full/default mask for normal runs
     - reduced mask for preserved-finalists cross reruns
   - Normal validation runs must keep current behavior.
   - Preserved-finalists cross reruns must skip non-gating aux finalist analytics such as:
     - `regime_slice_robustness`
     - `friction_stress`
   - This is an internal execution-mask change, not feature removal.

3. Replace serialization-based equality/keying where it is currently a correctness/determinism risk.
   - The addendum explicitly calls out `serialized_eq(...)` and raw `serde_json::to_string(...)` identity uses.
   - Replace them with one of:
     - structural equality where available/preferable
     - canonical/stable key helpers that sort object/map keys before comparison/stringification
   - Use one shared canonicalization approach for this surface; do not add multiple competing canonical-key schemes for the same candidate/config identity.
   - Precompute the first candidate’s canonical keys once when validating GPU uniformity.
   - Apply the same stable/canonical identity fix to any params key generation used by:
     - reopt-walk-forward finalist keys
     - SPA candidate keys
     - determinism tests
   - Do not rely on unstable `HashMap` iteration order for logical equality.

4. Precompute shared GPU start-gate work once.
   - The current GPU fixed-candidate path already requires uniform strategy/filter/MA-param conditions.
   - Precompute shared prerequisites once (for example regime-filter first valid index and Kaufman-stop first valid index or equivalent shared gate state).
   - Add a helper like `candidate_start_t_from_shared(...)`.
   - Stop recomputing shared regime/Kaufman work per candidate where only per-candidate MA warmup truly differs.

5. Introduce a prepared fixed-candidate validation plan.
   - Add a `PreparedGpuValidationPlan` (or equivalent) that preserves explicit combo order.
   - It should keep exact combo/pair order and any shared metadata needed for reuse across full-range and multi-range validation.

6. Introduce dense row-major range metrics storage.
   - Add `DenseRangeMetrics` (or equivalent) to replace per-range nested `HashMap<(usize, usize), HashMap<(u16,u16), Metrics>>` consumption where order is known.
   - Dense storage should be combo-indexed and range-indexed.
   - Consume dense storage in:
     - GPU walk-forward helpers
     - GPU CSCV helpers
     - GPU SPA helpers
     - GPU reopt-walk-forward helpers

7. Stop the “fixed-candidate GPU validation” path from doing implicit fast×slow grid work.
   - Today the code expands sparse finalist sets into unique fast/slow arrays and routes through optimization-shaped GPU APIs.
   - Implement an exact-combo GPU validation path so the device evaluates the explicit finalist combo list, not the Cartesian product of unique fast/slow periods.
   - `gpu_fixed_candidate_metric_map(...)` must stop being grid-shaped on device once the backend helper exists.
   - Expose the true `evaluated_pairs_on_device` through the execution profile.

8. Remove the optimizer-shaped full-range detour for `GpuHybridKernel` validation if the adjacent backend supports it.
   - Full-range fixed-candidate validation should use a narrower validation-focused path rather than building synthetic optimization requests where possible.

9. Batch ranges instead of re-entering the GPU path once per range.
   - Add a multi-range execution path for exact combos.
   - Batch all walk-forward / CSCV / SPA / reopt ranges into one prepared execution when possible.
   - Reduce kernel-launch / host-orchestration / metric-map rebuilding overhead.

10. Reduce cloning and host reshaping overhead.
   - Move owned GPU metrics out of containers where possible instead of cloning.
   - Prefer `Vec`/dense storage over repeated `HashMap` materialization.
   - Only replace `BTreeSet` uniqueness gathering with `Vec + sort_unstable + dedup` when it is a measured local win and does not make the code harder to reason about.

11. Add a post-pass `SeriesCache`.
   - Cache built candidate series artifacts for finalists.
   - The cache must return references or borrowed ownership, not deep clones of `CandidateSeries`.
   - Do not accidentally amplify memory by cloning large owned series values.

12. Add `eval_metrics_window_with_cfg(...)` or equivalent.
   - This is needed for friction-stress reuse where only cost config changes.

13. Rewrite `compute_friction_stress_summaries(...)`.
   - Reuse cached base series/state.
   - Do not rebuild full series for each finalist × scenario if only fee/spread/slippage changes.
   - Reuse MA/regime/Kaufman state and only vary execution/metric config where possible.

14. Reuse cached finalist series for post-pass modules.
   - Apply reuse to any other post-pass finalists analytics that currently rebuild the same series repeatedly.

15. Optionally optimize `compute_regime_slice_robustness_stats(...)` if profiling still shows it after caching.
   - Keep this secondary to exact-combo, batching, masking, and series-cache reuse.

16. Ensure all new CUDA-only helpers remain behind the existing feature gates.
   - Non-CUDA builds must continue to compile and return appropriate error/fallback behavior.

17. Add/adjust tests in this file for:
   - canonical equality / deterministic identity with permuted `HashMap` key order
   - exact-combo parity vs prior behavior
   - dense-range parity vs prior range-map behavior
   - aux-mask omission on preserved-finalists cross reruns
   - friction-stress cache reuse parity
   - route/fallback reasons unchanged except where more truthful route metadata is now exposed
   - GPU eligibility tests for logically equal param maps inserted in different key orders

Primary functions/helpers to inspect first in this file:
- `gpu_fixed_candidate_metric_map(...)`
- `try_prepare_gpu_fixed_candidate_validation(...)`
- `compute_gpu_walk_forward_stats_for_candidates(...)`
- `compute_gpu_cscv_slice_stats_for_candidates(...)`
- `compute_gpu_spa_slice_sums_for_candidates(...)`
- `compute_gpu_reopt_walk_forward_stats_for_candidates(...)`
- `serialized_eq(...)`
- `candidate_start_t(...)`
- `run_double_ma_validation_blocking_with_candles(...)`
- `compute_friction_stress_summaries(...)`
- `compute_regime_slice_robustness_stats(...)` (or equivalent)


==================================================
B. ADJACENT GPU RUNTIME / BACKEND FILES
==================================================

Expected if needed to complete exact-combo + multi-range batching cleanly:
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- immediate helper/backend files invoked by the current Double-MA GPU validation/optimization path

Required behavior:

1. Add an exact-combo fixed-candidate GPU helper.
   - Accept explicit combo lists, not only fast/slow ranges that imply a grid.
   - Return results in combo order.
   - Support full-range validation.

2. Add a multi-range exact-combo GPU helper if feasible in the current backend architecture.
   - Accept a precomputed range schedule and combo list.
   - Return a dense `[range][combo]`-shaped result or something that can be flattened into that with minimal host overhead.

3. Keep copy and allocation discipline tight.
   - Reuse device scratch where possible.
   - Prefer shared allocation helpers.
   - If host readback buffers are fully overwritten by the device, do not prefill them unnecessarily.

4. Only use safe uninitialized allocations where overwrite-before-read is guaranteed.
   - Prefer `alloc_policy` helpers if such helpers already exist or can be extended safely.

5. Do not break the current optimizer route while adding the validation-specific path.
   - Reuse backend code where it helps.
   - Avoid duplicating large amounts of logic if a validation-specific thin wrapper can call into shared kernels.

Do not spend this task on backend-adjacent stretch work such as compact DTOs, pinned-host staging, async copies, memory pools, CUDA Graphs, or device-resident prepared-plan caches unless one of them is strictly required to land the exact-combo or multi-range path.


==================================================
C. `crates/app_core/src/validation_pipeline.rs`
==================================================

Required changes:

1. Add reusable stationary-bootstrap scratch.
   - Add `TradeBootstrapScratch` (or equivalent).
   - Add a helper like `stationary_bootstrap_indices_into(...)`.
   - Add `compute_trade_bootstrap_stats_with_scratch(...)` while keeping the existing public wrapper for compatibility.
   - Ensure determinism matches the current seeded implementation.

2. Reuse stationary-bootstrap scratch in SPA too.
   - The addendum explicitly calls out that SPA has the same repeated scratch-allocation pattern.
   - Apply the same scratch-reuse logic there.

3. Optionally flatten SPA prep if profiling shows it matters after core changes.
   - Consider a flat row-major diff buffer rather than nested vectors.
   - Add a direct helper like `slice_return_sums_for_series_into(...)` to avoid extra allocation/collection when only sums are required.
   - Keep this lower priority than exact-combo, batching, and cache reuse.

4. Keep public behavior unchanged.
   - Any new helper should preserve the public wrapper/API unless a small compatible extension is required.

5. Add/update tests for:
   - parity of scratch-backed trade bootstrap vs current implementation
   - deterministic behavior with fixed seed
   - parity of SPA scratch reuse
   - parity of any flat row-major SPA prep optimization
   - build on the existing `validation_pipeline.rs` unit-test module rather than adding separate CLI/bench-style tests for this area


==================================================
D. `src-tauri/src/commands_validation.rs`
==================================================

Required changes:

1. Remove unnecessary candle copying during cross-ticker alignment.
   - Introduce a borrowed-vs-owned aligned candle representation (or equivalent) so full-range no-op alignment does not clone full OHLCV arrays.
   - Update call sites to consume borrowed aligned data via `.as_ref()` / equivalent.

2. Preserve the current preserved-finalists cross-rerun request shaping, and only extend it if backend aux-mask support is added.
   - Keep the existing behavior that already omits walk-forward, reopt walk-forward, SPA, perturb, and trade bootstrap for preserved-finalists reruns.
   - Keep the existing preserved-finalists retention of CSCV/PBO and hard-gate inputs.
   - If a new backend aux-masked validation entry is added, wire preserved-finalists reruns through it without regressing the current preserved-stack behavior.

3. Keep current route/fallback policy intact.
   - Reduce hidden work, but do not regress fallback transparency.

4. Add/update tests for:
   - borrowed no-op alignment path
   - any new backend aux-mask integration if you add it
   - existing preserved-finalists route/fallback behavior remaining intact


==================================================
E. `src-tauri/src/validation_common.rs`
==================================================

Required changes:

1. Reduce trade-processing allocations.
   - Sort `acc.trades` in place where possible.
   - Remove avoidable extra median/trade cloning.

2. Preserve and reuse the existing canonical candidate-key path.
   - `candidate_key_canonical(...)` already exists here and is already used by cross-ticker orchestration.
   - Do not replace it with a second local identity scheme unless adjacent callers truly require broader reuse.

3. Normalize dataset path keys during cross-ticker dedupe.
   - The addendum explicitly calls this out.
   - Ensure slash/case/normalization differences do not cause duplicate reruns when they refer to the same dataset path.

4. Add/update tests for:
   - unchanged cross-ticker stats after the allocation reductions
   - normalized-path dedupe behavior
   - candidate identity parity / determinism only if you touch the canonical-key helper or its call sites


==================================================
F. `web/src/services/validationRequestBuilder.ts`
==================================================

Required changes:

1. Deduplicate validation candidates after the current validity filter.
   - Use a stable canonical key that includes all semantically relevant candidate fields, including:
     - MA types and lengths
     - MA params
     - strategy config / strategy-local config
     - filter config
   - Do not accidentally merge semantically distinct candidates.

2. Deduplicate cross datasets by normalized path.
   - Do this on the frontend request-building side as well, not only backend-side.

3. Preserve the current preserved-finalists preview semantics unless backend behavior truly changes.
   - The existing summary already describes the current lighter rerun shape.
   - Do not over-claim exact-combo GPU behavior unless the backend path truly changed and the summary text is consistent with reality.

4. Keep backend-selection policy unchanged unless the backend path genuinely changes.


==================================================
G. `web/src/services/validationRequestBuilder.test.ts`
==================================================

Required changes:

Add/update test coverage for:
- duplicate-candidate dedupe
- duplicate cross-dataset dedupe using normalized paths
- keep the current preserved-finalists summary assertions passing unless backend behavior actually changes
- any summary text change only if backend semantics truly change


==================================================
H. `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
==================================================

This is the primary measurement/proof boundary for the validation-throughput work.

Required changes:

1. Preserve the existing coarse timing/route/fallback/candidate-count report shape and extend it compatibly.
2. If `DoubleMaValidationResult` gains an execution-profile field, thread the knowable work-shape fields into the report JSON and stdout summaries.
3. Prefer these added fields where they are actually available from the result/runtime:
   - requested candidates
   - unique candidates
   - unique fast periods
   - unique slow periods
   - evaluated pairs on device
   - one expansion-ratio field name, kept consistent end to end
   - GPU range-eval call count and/or range batch count / total ranges
   - aux finalist analytics wall time
   - cpu-tail wall time if available
   - GPU result materialization / host-shaping time if available
4. Do not invent estimated transport-byte or cold/warm-cache telemetry in this task unless the runtime already exposes a truthful source for it.
5. Keep the validation-cost harness as the primary evidence surface.


==================================================
I. `crates/app_core/benches/double_ma_validation_workflow.rs`
==================================================

Required changes:

1. Keep the existing CPU workflow bench as CPU regression coverage.
2. Add a CUDA-gated sparse-finalist validation bench that makes pair-expansion/work-shape visible.
3. Keep the existing route-probe output and extend it to print execution-profile work-shape fields when present.
4. Do not treat this bench as the sole proof boundary for CUDA validation wins.


==================================================
J. `crates/app_core/benches/composite_validation_workflow.rs`
==================================================

Required changes:

1. Keep the existing composite validation benches as supplementary evidence.
2. Only modify this file if it directly helps shared telemetry visibility or shared GPU validation route coverage.
3. If this file is touched, prefer extending an existing GPU bench/profile printout to include execution-profile work-shape fields when present instead of adding broad new benchmark matrices.

--------------------------------
COPY-ELISION / MEMORY / UNINITIALIZED-ALLOCATION RULES
--------------------------------

Apply these rules throughout the implementation:

1. Good candidates for copy elimination:
   - no-op full-range alignment where borrowed candle data can be reused
   - dense result buffers where host order is known
   - per-range host maps that can become row-major buffers
   - repeated scratch vectors in trade bootstrap and SPA bootstrap
   - host readback buffers that are fully overwritten by device copies

2. Good candidates for safe uninitialized allocation only if the invariants hold:
   - dense row-major metric/result buffers that are completely filled before read
   - device-readback host buffers that are fully populated by `copy_to(...)` / equivalent
   - prefix-initialized series where only the warm prefix needs explicit `NaN` / `0.0` and the remainder is overwrite-only

3. Do NOT use uninitialized allocation when:
   - any element may be read before guaranteed write
   - the fill value is behaviorally observable
   - a simpler borrow/reference change removes the need for allocation entirely

4. Prefer helpers in `alloc_policy.rs`.
   - Extend shared helpers if needed.
   - Avoid ad-hoc unsafe allocation patterns sprinkled through multiple files.

5. Every unsafe or `MaybeUninit` path must include a clear overwrite-before-read safety explanation.

--------------------------------
OPTIONAL ITEMS
--------------------------------

Ignore the addendum’s optional backend-adjacent stretch layer for this task unless a small part of it is the minimum change needed to land the core exact-combo, batching, masking, or scratch-reuse work.

--------------------------------
TESTING / VALIDATION REQUIREMENTS
--------------------------------

You must add/update tests so the patch proves correctness rather than only compiling.

Required validation before declaring completion:

1. Existing tests still pass in the affected areas.
2. New or updated tests cover:
   - canonical/stable equality and keying for semantically equal map-containing configs
   - GPU eligibility when logically equal param maps differ only in insertion order
   - exact-combo parity on sparse finalist sets
   - dense-range parity vs prior per-range map behavior
   - preserved-finalists aux masking
   - borrowed no-op candle alignment
   - cross-dataset normalized-path dedupe
   - friction-stress series-cache reuse parity
   - trade-bootstrap scratch reuse parity
   - SPA scratch reuse parity
   - benchmark/report changes only where they are naturally testable in existing nearby test modules
3. Re-run parity-sensitive validation paths where applicable:
   - main metrics parity CPU vs GPU
   - walk-forward parity
   - CSCV parity
   - SPA parity
   - reopt-walk-forward parity
   - route/fallback reason tests
4. Capture before/after evidence from `ta_double_ma_validation_cost.rs` if possible in the repo environment.
   - At minimum, ensure the new telemetry fields are emitted.
   - Prefer to demonstrate reduced `evaluated_pairs_on_device` and reduced `gpu_range_eval_calls` on sparse-finalist workloads.
5. Extend existing nearby test modules instead of creating fragile snapshot tests for Criterion output or CLI stdout unless there is already a stable testing pattern for that surface.

--------------------------------
IMPLEMENTATION ORDER / WORKFLOW
--------------------------------

Use this working order:

Step 1. Inspect the target functions listed above and the two implementation-plan markdown files.
Step 2. Land execution-profile telemetry, canonical/stable equality/keying, and aux masking.
Step 3. Land shared-start-gate precompute and host-side dense result consumption.
Step 4. Implement the exact-combo GPU backend path.
Step 5. Implement multi-range batching.
Step 6. Implement CPU-tail caching/scratch reuse and cross-ticker copy-elision.
Step 7. Update frontend dedupe/summaries and backend/common dedupe normalization.
Step 8. Update the validation-cost harness and workflow benches.
Step 9. Run tests/benches and fix regressions.
Step 10. Only then consider optional stretch items.

At every step, keep the tree compiling.

--------------------------------
WHAT NOT TO DO
--------------------------------

- Do not remove modules/features to make benchmarks faster.
- Do not replace the validation surface with a new architecture.
- Do not change public semantics under the guise of optimization.
- Do not add unrelated UI/product features.
- Do not parallelize RNG-dependent paths casually.
- Do not switch important metric paths from `f64` to `f32` without an explicit parity rationale.
- Do not keep raw `serde_json::to_string(...)` equality/keying where map-order instability matters.
- Do not add local unsafe allocation code if an existing/shared helper can do the job.

--------------------------------
DELIVERABLE FORMAT
--------------------------------

When you finish, your final response must include:

1. A concise summary of what you changed, grouped by phase.
2. A list of files modified.
3. A short explanation of any adjacent backend/runtime files you had to touch and why.
4. A test/benchmark report:
   - commands run
   - whether they passed
   - any notable perf/telemetry evidence
5. A short list of optional stretch items completed vs intentionally left out.
6. Any remaining blocker/risk that still requires deeper adjacent runtime work.

Do not respond with only a plan. Implement the code.

--------------------------------
SUCCESS CRITERIA
--------------------------------

The task is complete only when all of the following are true:

- the code compiles
- the targeted tests pass
- the primary affected validation behaviors still work
- preserved-finalists cross reruns no longer perform hidden non-gating aux work
- canonical/stable equality fixes remove map-order nondeterminism risk
- sparse finalist GPU validation no longer evaluates an unnecessary implicit fast×slow grid on device (or, if a deeper backend blocker truly prevents this, the patch clearly surfaces the blocker and still lands all surrounding scaffolding/telemetry)
- multi-range validation no longer re-enters the GPU path once per range when the backend supports batching
- unnecessary memory copies are reduced, especially no-op full-range alignment copies and repeated scratch allocations
- safe uninitialized allocation is only used where overwrite-before-read is guaranteed
- `ta_double_ma_validation_cost.rs` exposes the new work-shape/throughput telemetry that is truthfully available from the validation result/runtime

If you encounter a genuine backend blocker that prevents the exact-combo or multi-range device implementation, do not abandon the task. Land the maximum safe subset, instrument the remaining gap clearly, and explain the blocker precisely in the final report.
```
