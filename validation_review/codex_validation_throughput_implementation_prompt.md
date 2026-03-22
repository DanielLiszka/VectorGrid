# Codex implementation prompt — validation throughput / CUDA optimization / fallback behavior

Use the following prompt verbatim with a Codex agent that has access to the repository and the referenced guide documents.

```text
You are an implementation agent working inside my Rust/Tauri/TypeScript repository.

Your task is to fully implement the validation-throughput / CUDA-optimization / fallback-behavior plan described in these documents:

1. `gpt54_feature_review_map.md`
   - Use the rules from:
     - `## 4. Validation throughput and fallback behavior`
     - `## Prompt rules for GPT-5.4`
     - `## 16. Allocation policy, prefix initialization, and copy-elision discipline`
2. `validation_throughput_cuda_plan.md`
3. `validation_throughput_cuda_plan_addendum.md`

These documents are the source of truth for scope and priorities.

--------------------------------
PRIMARY OBJECTIVE
--------------------------------

Implement the performance/correctness plan end-to-end for the Double-MA validation surface.

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
   - this contains additional required fixes and optional stretch items
4. then inspect the repo code starting from the files listed below

--------------------------------
FILES TO MODIFY (REQUIRED)
--------------------------------

Treat these as the required implementation surface unless a listed item truly ends up being a no-op after code inspection:

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
- `crates/app_core/benches/composite_validation_workflow.rs`

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
8. Required addendum items from sections 1–6 of `validation_throughput_cuda_plan_addendum.md`

Stretch implementation (only after the above is complete and compiling/tests passing):
- optional addendum sections 7A–8C if they are low-risk and directly supported by the existing backend/runtime
- if stretch items are not implemented, leave the core patch clean and do not block completion on them

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
   - Replace `BTreeSet` uniqueness gathering with `Vec + sort_unstable + dedup` where appropriate and safe.

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

5. If a compact internal validation DTO is feasible for batched range validation, you may add it.
   - Example: a `ValidationMetricLite`-style internal result for the batched path.
   - Keep `f64` fields unless there is already a validated precision rationale for `f32`.
   - Do not change the public in-sample result contract unless necessary.

6. Do not break the current optimizer route while adding the validation-specific path.
   - Reuse backend code where it helps.
   - Avoid duplicating large amounts of logic if a validation-specific thin wrapper can call into shared kernels.

7. Optional stretch only after the core patch is stable:
   - pinned host staging buffers and async H2D/D2H copies
   - persistent device scratch / memory-pool reuse
   - CUDA Graph capture for repeated launch shapes
   - device-resident prepared-plan indicator/state cache

Do not spend the whole task on these stretch items before the core exact-combo and range-batching work is landed.


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


==================================================
D. `src-tauri/src/commands_validation.rs`
==================================================

Required changes:

1. Remove unnecessary candle copying during cross-ticker alignment.
   - Introduce a borrowed-vs-owned aligned candle representation (or equivalent) so full-range no-op alignment does not clone full OHLCV arrays.
   - Update call sites to consume borrowed aligned data via `.as_ref()` / equivalent.

2. Route preserved-finalists cross reruns through the aux-masked validation entry.
   - Normal runs keep full aux work.
   - Preserved-finalists reruns skip non-gating aux finalist analytics.

3. Keep current route/fallback policy intact.
   - Reduce hidden work, but do not regress fallback transparency.

4. Add/update tests for:
   - borrowed no-op alignment path
   - preserved-finalists rerun using reduced aux mask
   - unchanged route/fallback behavior outside the intended aux-work reduction


==================================================
E. `src-tauri/src/validation_common.rs`
==================================================

Required changes:

1. Reduce trade-processing allocations.
   - Sort `acc.trades` in place where possible.
   - Remove avoidable extra median/trade cloning.

2. Improve candidate identity stability.
   - Keep canonical/stable key fallback if needed.
   - Avoid unstable/plain JSON serialization-based identity where that is a correctness or determinism risk.

3. Normalize dataset path keys during cross-ticker dedupe.
   - The addendum explicitly calls this out.
   - Ensure slash/case/normalization differences do not cause duplicate reruns when they refer to the same dataset path.

4. Add/update tests for:
   - unchanged cross-ticker stats after the allocation reductions
   - normalized-path dedupe behavior
   - candidate identity parity / determinism


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

3. Update preserved-finalists summary wording after the aux mask lands.
   - The summary should reflect the actual lighter rerun shape.
   - Do not over-claim exact-combo GPU behavior unless the backend path truly changed and the summary text is consistent with reality.

4. Keep backend-selection policy unchanged unless the backend path genuinely changes.


==================================================
G. `web/src/services/validationRequestBuilder.test.ts`
==================================================

Required changes:

Add/update test coverage for:
- duplicate-candidate dedupe
- duplicate cross-dataset dedupe using normalized paths
- preserved-finalists summary wording after aux-mask landing
- any summary text change that now mentions exact-combo GPU validation or more truthful fallback details


==================================================
H. `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
==================================================

This is the primary measurement/proof boundary for the validation-throughput work.

Required changes:

1. Thread the new execution-profile fields into the report JSON and stdout summaries.
2. Print / serialize at least:
   - requested candidates
   - unique candidates
   - evaluated pairs on device
   - grid expansion ratio
   - range batches / total ranges
   - aux finalist analytics wall time
   - cpu-tail wall time if available
   - gpu result materialization time if available
   - GPU range-eval call count if available
   - cold-vs-warm marker if that is already available or easy to add
3. Keep the validation-cost harness as the primary evidence surface.
4. Preserve existing report structure where reasonable; add fields compatibly.


==================================================
I. `crates/app_core/benches/double_ma_validation_workflow.rs`
==================================================

Required changes:

1. Keep the existing CPU workflow bench as CPU regression coverage.
2. Add a CUDA-gated sparse-finalist validation bench that makes pair-expansion/work-shape visible.
3. Print execution-profile work-shape fields when present.
4. Do not treat this bench as the sole proof boundary for CUDA validation wins.


==================================================
J. `crates/app_core/benches/composite_validation_workflow.rs`
==================================================

Required changes:

1. Add at least one sparse-finalist composite GPU bench that makes pair-expansion/work shape visible.
2. Print execution-profile work-shape fields when present.
3. Keep this as supplementary evidence, not the sole Double-MA validation proof boundary.

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
IMPORTANT OPTIONAL ITEMS FROM THE ADDENDUM
--------------------------------

After the core patch is complete, you may implement these if they are straightforward and low-risk in the existing codebase:

1. Internal compact validation DTO for batched range validation (`ValidationMetricLite`-style), keeping `f64`.
2. Pinned host staging buffers and async transfers.
3. Persistent device scratch / memory pool reuse.
4. CUDA Graph capture for repeated prepared-plan launch shapes.
5. Device-resident prepared-plan indicator/state cache.
6. Larger borrowed candle-view refactor (`CandlesView<'a>` or equivalent) only if it stays local and low-risk.
7. Deterministic CPU-tail parallelism only after caching is in place, and only for non-RNG-sensitive work with stable ordering.
8. Stable candidate IDs generated once at request ingress if it naturally fits the code while replacing repeated textual key regeneration.

These are stretch goals, not blockers for the core patch.

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
   - any benchmark/report output changes that are testable
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
- `ta_double_ma_validation_cost.rs` exposes the new work-shape/throughput telemetry

If you encounter a genuine backend blocker that prevents the exact-combo or multi-range device implementation, do not abandon the task. Land the maximum safe subset, instrument the remaining gap clearly, and explain the blocker precisely in the final report.
```
