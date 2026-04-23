# Codex implementation prompt — VCC GPU optimization, validation, and correctness patch set

Copy/paste the prompt below into your OpenAI Codex agent.

---

You are implementing a **VCC-only, strategy-local GPU optimization and validation patch set** in an existing Rust repo.

Read these guidance files first and treat them as the implementation brief:

1. `/mnt/data/gpt54_feature_review_map.md`
   - Focus especially on:
     - `## Hard scope guard`
     - `## Current repo optimization posture`
     - `## 5. Strategy-local GPU-native optimization/validation routes`
     - `## 16. allocation policy, prefix initialization, and copy-elision discipline`
2. `/mnt/data/vcc_strategy_gpu_optimization_plan.md`
3. `/mnt/data/vcc_strategy_gpu_optimization_addendum.md`

If those files are mirrored elsewhere in the repo/workspace, prefer the most current copies, but keep the intent of the two VCC-specific guides intact.

## Objective

Implement the VCC GPU optimization/correctness/benchmark plan from the two VCC markdown guides, with a strong bias toward:

- **performance**
- **copy elision**
- **removing unnecessary staging/allocation**
- **correctness and parity preservation**
- **maintaining existing features and public behavior**
- **benchmark-first evidence rather than speculative redesign**

The implementation must stay **VCC-only** and **strategy-local**. Do **not** broaden this into a repo-wide GPU redesign unless a tiny shared helper/export is absolutely required.

## Primary repo files to modify

These are the main target files and should receive the bulk of the work:

- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

## Secondary files you may touch only if required

Touch these only when necessary to support the above implementation cleanly:

- `crates/app_core/src/lib.rs` or the relevant module export surface if bench/test code needs a new public helper such as `clear_vcc_gpu_runtime_cache(...)`
- `crates/app_core/src/alloc_policy.rs` **only if** a shared safe helper already exists or should be reused for prefix-init / uninitialized allocation; do not create ad-hoc unsafe local patterns if the shared helper does not exist
- any minimal test/bench support module needed to compile the new VCC tests/benches

Do **not** make unrelated changes elsewhere.

## Hard constraints

1. Preserve public behavior and features unless a change is explicitly required by the VCC guides.
2. Do not remove CPU fallbacks.
3. Do not remove any current VCC functionality.
4. Do not add new strategies.
5. Do not perform a broad architecture rewrite.
6. Do not claim GPU wins without route/fallback/parity evidence.
7. Keep feature gating correct, especially around `cuda` and `cuda-backtest-kernel`.
8. Prefer small, measurable, parity-safe changes over ambitious rewrites.
9. If a suggested helper name from the guides does not fit the codebase, adapt it while preserving the intended behavior.
10. If any part of the guide would require a much larger refactor than expected, implement the smallest safe version first, note the deviation, and keep moving.

## What success looks like

A successful patch set should do all of the following:

- Reduce repeated H→D staging and repeated allocation in the VCC GPU metric path.
- Avoid unnecessary D→H copies for the metrics-only VCC path.
- Reuse cached device/host buffers where safe.
- Add strict input validation so malformed VCC combo batches fail before the kernel launch.
- Make VCC GPU route/fallback behavior more visible.
- Add real optimizer and validation benchmark coverage for VCC.
- Add at least a small non-diagnostic parity gate for VCC GPU behavior.
- Preserve correctness and maintain current functionality.

## Mandatory implementation phases

Implement the work in the following order.

---

# Phase 1A — immediate correctness and copy-elision in `vcc_gpu_kernel.rs`

## File

- `crates/app_core/src/vcc_gpu_kernel.rs`

## Required edits

### 1. Split the internal executor into **metrics-only** vs **detailed** modes

Current problem:

- `eval_vcc_metrics_gpu_kernel_device_rows(...)` currently routes through the detailed path, so it still pays for slice-output allocation, D→H copy, and decode work even when the caller only needs `Vec<Metrics>`.

Implement:

- an internal execution mode such as:
  - `VccGpuEvalMode::MetricsOnly`
  - `VccGpuEvalMode::Detailed`
- keep the **public API stable**:
  - `eval_vcc_metrics_gpu_kernel_device_rows_detailed(...)` must still return detailed results
  - `eval_vcc_metrics_gpu_kernel_device_rows(...)` must still return `Vec<Metrics>`
- refactor the shared executor so that in `MetricsOnly`:
  - it does **not** allocate/resize a host slice readback buffer
  - it does **not** perform the D→H copy for slice metrics
  - it does **not** decode `step_return_sum` / `step_return_sumsq`
- keep the kernel ABI unchanged in phase 1 if needed
  - if the kernel still requires a valid `slice_out` device pointer, it is acceptable to keep the device allocation temporarily while skipping host readback
  - only skip device-side `slice_out` allocation later if the ABI safely supports it

This is a top-priority optimization.

### 2. Add strict public-entry validation for combo row indices and time windows

Current problem:

- `validate_inputs_device_rows(...)` checks lengths/shapes but does not validate:
  - `fast_row_index[i]` in range
  - `trend_row_index[i]` in range
  - `start_t`, `metric_start_t`, `end_t` non-negative and ordered
  - `end_t <= cols`

Implement:

- keep `validate_inputs_device_rows(...)` for matrix/length checks
- add a second validator (name can vary) that validates combo row/time ranges
- call it before launching the kernel
- this validation should be **always-on** for the public entrypoint unless the existing codebase strongly dictates otherwise

Required checks:

- `fast_row_index[i] >= 0 && fast_row_index[i] < fast_rows`
- `trend_row_index[i] >= 0 && trend_row_index[i] < trend_rows`
- `start_t[i] >= 0`
- `metric_start_t[i] >= 0`
- `end_t[i] >= 0`
- `start_t[i] <= metric_start_t[i] <= end_t[i]`
- `end_t[i] <= cols`

### 3. Tighten scalar conversion safety

Replace unsafe/narrowing numeric casts with checked conversions where the guides call for them.

At minimum, use checked `i32::try_from(...)` for values such as:

- `signals.cols`
- `signals.fast_rows`
- `signals.trend_rows`
- `combo_count`
- `out_stride`

Do not change semantics; just remove avoidable overflow risk.

### 4. Reuse kernel function lookup per eval call

Current problem:

- `pack_rows_u64_ptr_f32` is currently looked up repeatedly
- `vcc_backtest_metrics_f32` is also looked up in the hot path

Implement a minimal safe improvement:

- resolve the function handles once per eval call
- pass the already-resolved handle into pack-launch helpers
- do **not** force a lifetime-heavy long-lived function cache if that becomes awkward or risky

### 5. Optionally reuse start buffers if `start_t == metric_start_t`

If the caller frequently passes identical buffers for `start_t` and `metric_start_t`, reuse the same device buffer where that is clearly safe and low-complexity.

This is optional but worthwhile.

---

# Phase 1B — warm-path staging reuse in `vcc_gpu_kernel.rs`

## File

- `crates/app_core/src/vcc_gpu_kernel.rs`

## Required edits

### 6. Extend `VccCudaRuntime` with reusable VCC-specific cache state

The current runtime is too thin and recreates too much work per call.

Implement reusable state inside `VccCudaRuntime` (names can vary, but cover these responsibilities):

- cached price-state device buffers
- cached packed fast squeeze matrix
- cached packed fast momentum matrix
- cached packed trend matrix
- reusable combo scratch (device input vectors, device output buffers, host readback scratch)

A shape close to the guide is acceptable, for example:

- `CachedPriceState`
- `CachedPackedMatrix`
- `VccComboScratch`
- optional `HostReadbackScratch`

### 7. Persist device price-state buffers

Current problem:

- the VCC path recomputes and reuploads:
  - `close_f32`
  - `lr_f32`
  - `lr_f64`
  on every call

Implement:

- reusable device buffers for these arrays
- a cache key / identity policy so they are reused across repeated calls for the same candle data

Cache identity policy:

- **preferred**: explicit epoch/generation ID if feasible with current call sites
- **acceptable fallback**: pointer + length identity plus debug-only assertions/checksums and explicit documentation of the limitation
- do **not** silently rely on pointer + length without at least documenting/debug-hardening the mutation risk

If adding explicit epochs to `VccGpuDeviceSignalMatrices<'_>` is feasible without blowing up the patch size, do it. Otherwise use the acceptable fallback and document it.

### 8. Persist packed fast/trend matrices across repeated calls

Current problem:

- when row-major host matrices are not provided, the code uploads row-pointer lists and repacks rows into contiguous device matrices on every call

Implement cache-aware packed-matrix reuse for:

- fast squeeze rows
- fast momentum rows
- trend rows

Support both cases:

- source provided as host row-major matrix
- source provided as device row pointers requiring pack-kernel staging

The cache key must be strong enough to avoid stale reuse.

### 9. Reuse combo scratch and host readback buffers

Current problem:

- combo vectors and outputs are reallocated frequently

Implement reusable scratch storage for:

- `d_fast_idx`
- `d_trend_idx`
- `d_start_t`
- `d_metric_start_t`
- `d_end_t`
- `d_out`
- `d_slice_out` (detailed path only if practical)
- host output readback scratch
- host slice readback scratch (detailed path only)

Reallocation policy:

- reuse buffers if capacity is sufficient
- grow when needed
- do not shrink eagerly in the hot path

### 10. Add a deterministic cache-clear helper

Add a helper to clear the VCC thread-local runtime cache so cold vs warm benchmarks are deterministic.

Preferred shape:

- `clear_vcc_gpu_runtime_cache(device_id: Option<u32>)`

If benches/tests need it publicly, export it minimally from the crate surface.

Also document/reflect the fact that the cache is **thread-local** and therefore warm-path reuse is thread-affine.

### 11. Synchronize once per eval and keep stream use coherent

Where possible, keep uploads/launches on the runtime stream and synchronize once after launch before readback.

If the current `cust` version supports safe stream-bound async upload/allocation patterns, use them judiciously. If not, keep synchronous operations but still reuse buffers.

Do not add multi-stream complexity in this patch.

---

# Phase 1C — memory policy and copy-elision rules

## Files

- primarily `crates/app_core/src/vcc_gpu_kernel.rs`
- optionally `crates/app_core/src/alloc_policy.rs` if a shared helper already exists and is clearly the right abstraction

## Required policy

You must minimize unnecessary copies, but you must do it safely.

### Safe places to use uninitialized allocation now

These are acceptable early targets:

- device packed matrices that are fully written by the pack kernel before read
- device output buffers fully written by `vcc_backtest_metrics_f32`
- host vectors built with `Vec::with_capacity(...)` + `push(...)`

### Safe only if a shared repo helper already exists

These can use uninitialized allocation **only** through an existing shared helper/pattern already present in `alloc_policy.rs` or equivalent:

- prefix-init vectors where only the prefix is observably initialized and the tail is guaranteed overwrite-only
- host readback vectors for `copy_to(...)` if the helper is known-safe and already accepted in the repo

### Avoid in this first patch unless a shared helper already exists

Do **not** add ad-hoc local unsafe `MaybeUninit` patterns for:

- host readback buffers for `copy_to(...)`
- vectors whose values may be observed before guaranteed overwrite
- `build_vcc_desired_positions(...)` tail initialization unless a repo-standard helper exists

### Required practical approach

- first implement reuse of host readback buffers
- if the repo already has a standard helper for safe uninitialized readback/prefix-init, migrate to it
- otherwise leave the zero-fill/reuse in place for host readback rather than adding one-off unsafe code

---

# Phase 2 — route transparency and CPU-side cleanup in `vcc.rs`

## File

- `crates/app_core/src/vcc.rs`

## Required edits

### 12. Add VCC CUDA route metadata

Current problem:

- VCC helpers can silently fall back, so a GPU-labeled path may not be using the intended route

Implement a small route-metadata layer.

Preferred shape:

- `VccCudaRouteMeta`
- optionally `VccCudaRouteKind` with richer route classification

Good route-kind distinctions include:

- CPU-only
- GPU device-input path used
- GPU host-input path used
- CPU fallback because prepared CUDA inputs were absent
- CPU fallback because descriptor policy blocked GPU
- CPU fallback because a CUDA attempt failed

If keeping the richer enum becomes awkward, at least capture:

- whether GPU was attempted
- whether GPU was used
- whether device-inputs were used
- fallback reason string if present

### 13. Capture route/fallback reason in `compute_indicator_f64_with_warmup(...)`

Implement the metadata in the internal series wrapper returned by `compute_indicator_f64_with_warmup(...)`.

Requirements:

- preserve current public behavior
- do not remove CPU fallback
- record enough metadata to distinguish successful CUDA use from fallback
- include reason text where available

### 14. Keep public API compatibility wrappers intact

If you introduce new `_with_route` or internal helper variants, keep the existing public helpers intact and have them delegate to the new versions.

Potential surfaces include the CUDA direct/device sweep helpers such as:

- `compute_vcc_ttm_squeeze_sweep_cuda_device(...)`
- `compute_vcc_squeeze_momentum_sweep_cuda_device(...)`
- `compute_vcc_fast_dual_sweep_cuda_device(...)`
- `compute_vcc_ttm_trend_sweep_cuda_device(...)`

Use wrappers rather than breaking call sites.

### 15. Remove tiny avoidable heap allocations in fixed-size param construction

In the VCC reference/parity helpers, replace small fixed-size `Vec<IndicatorParamKV>` constructions with stack arrays where the parameter count is statically known.

At minimum check and update:

- `compute_vcc_fast_signals(...)`
- `compute_vcc_trend_signals(...)`
- any nearby static-parameter helper with a tiny `Vec` allocation in the VCC path

Use `.as_slice()` on arrays.

### 16. Only use prefix-init for `build_vcc_desired_positions(...)` if the repo already has the right helper

Current code zero-fills the whole vector. That is safe but not optimal.

Implement the optimization only if a **shared** repo-standard helper already exists (for example in `alloc_policy.rs`).

If no such helper exists, leave the full zero-init in place for this patch.

Do **not** add ad-hoc unsafe here.

### 17. Add at least one real parity gate in `vcc.rs` tests

The guides make clear that VCC already has diagnostic-only parity knowledge, especially around momentum FP32 behavior.

Promote at least one small case into a non-ignored test.

Implement at least:

- a small `vcc_momentum_roundtrip_is_fp32_stable(...)` gate on a known-stable fixture
- a small GPU metric parity test comparing CPU/reference behavior vs GPU metric path within a documented tolerance

If practical, also add:

- a cache reuse test or deterministic cache-clear test

Keep these tests small and fast enough to be realistic for targeted runs.

---

# Phase 3 — VCC optimizer benchmark coverage in `gpu_native_expansion_opt.rs`

## File

- `crates/app_core/benches/gpu_native_expansion_opt.rs`

## Required edits

### 18. Add VCC imports and request builder

This file currently has no VCC optimizer Criterion coverage.

Add the necessary import(s), e.g. `VccStrategyParams`, and add a VCC request builder analogous to the other strategies.

Implement:

- `make_vcc_req(...)`

### 19. Add a VCC-specific synthetic candle generator

Use a synthetic candle shape that actually exercises squeeze/compression + breakout/release + trend/breakdown behavior. The guide includes an example regime structure; follow that spirit even if you adapt the exact numbers.

Implement:

- a VCC-specific candle generator such as `make_vcc_breakout_candles(...)`

### 20. Add baseline VCC CPU and GPU Criterion groups

At minimum add:

- `bench_vcc_cpu(...)`
- `bench_vcc_gpu_kernel(...)`

Reuse the existing route-probe/assert pattern already used in this benchmark file.

Requirements for the GPU-labeled VCC bench:

- perform the route probe
- assert that the actual backend used is GPU
- keep the benchmark naming consistent with the file’s existing style

### 21. Add VCC cache-layer benchmark modes where feasible

If this file’s harness shape supports it cleanly, add explicit labels for at least one VCC candle size to separate:

- `cold`
- `warm_same_rows`
- `warm_new_rows`
- `warm_new_combos_only`

Use `clear_vcc_gpu_runtime_cache(Some(0))` (or equivalent) to force cold runs.

If implementing all four modes cleanly in this file becomes awkward, keep the baseline CPU/GPU VCC groups here and implement the more detailed cache-layer separation in `composite_validation_workflow.rs`, but try to provide at least one explicit cold/warm distinction here if possible.

### 22. Add optional param-shape coverage if the harness supports it cleanly

If practical in this file, add a VCC benchmark variant that changes non-period VCC config affecting branch behavior, such as:

- `require_momentum_accel`
- `confirm_bars`
- `max_trade_bars`

If it fits better in the validation benchmark file, put the richer param-shape coverage there instead.

### 23. Update `criterion_group!`

Include the new VCC benchmark functions in the benchmark group.

---

# Phase 4 — VCC validation benchmark coverage in `composite_validation_workflow.rs`

## File

- `crates/app_core/benches/composite_validation_workflow.rs`

## Required edits

### 24. Add a small reusable GPU-route assertion helper

The GPU validation benches currently print route info but do not consistently assert that the VCC path is truly using GPU.

Add a helper such as:

- `assert_gpu_route(...)`

Use it after route probes and inside GPU-labeled benchmark iterations.

### 25. Add VCC-specific validation candles

Implement a VCC-specific validation candle generator that creates alternating compressed/expanded regimes and trend transitions.

Implement:

- a helper such as `make_vcc_validation_candles(...)`

### 26. Add a wider VCC candidate generator

The current `make_vcc_candidates(n)` uses `VccStrategyParams::default()` and under-stresses unique-row packing and branch-shape diversity.

Implement at least one wider VCC candidate helper, such as:

- `make_vcc_candidates_wide(n)`

This should broaden period diversity.

### 27. Add VCC param-shape candidate coverage

Implement another VCC candidate helper that varies VCC behavior-shaping parameters such as:

- `require_momentum_accel`
- `confirm_bars`
- `max_trade_bars`

A name like `make_vcc_candidates_shape_mix(n)` is fine.

This is important because VCC kernel behavior depends on more than just period widths.

### 28. Tighten the existing GPU profile closure

Inside the existing GPU profile bench closure, keep the route logging but also assert the route for GPU-labeled groups.

This should apply to VCC and ideally the other strategies too if the pattern remains clean and non-invasive.

At minimum, VCC-labeled GPU groups must assert `BackendUsed::Gpu`.

### 29. Add a dedicated heavy VCC GPU validation group

Add a dedicated VCC GPU validation benchmark group modeled after the existing heavy ASR pattern, but VCC-specific.

It should include at least:

- a heavier 100k-ish VCC GPU workflow case
- a heavier in-sample case with a larger candidate count
- route probe logging + GPU route assertion

### 30. Add larger VCC profile cases

Add at least one larger VCC profile case so VCC is represented beyond the light generic profile shape.

Examples from the guide include:

- `bench_profile("vcc", 100_000, make_vcc_candidates_wide(32));`

and GPU analogues.

### 31. Add explicit cold/warm/wide-rows labels for VCC validation benches

Use the new cache-clear helper to add explicit labels such as:

- `gpu_kernel/vcc/.../warm`
- `gpu_kernel/vcc/.../cold`
- `gpu_kernel/vcc/.../wide_rows`

If full coverage is too invasive, prioritize:

1. warm baseline
2. cold baseline
3. wide-row stress case

---

# Phase 5 — optional but recommended micro-optimizations beyond the strict core prompt

These are still performance-aligned and allowed if the primary phases are complete and verified.

## Optional improvement A — stream-bound async copies/allocations

If the current `cust` version clearly supports safe stream-bound async copies/allocations for the reused VCC buffers, prefer that inside the existing non-blocking runtime stream.

Do **not** add multi-stream or overlap complexity.

## Optional improvement B — richer route metadata

If phase 2 is stable, prefer the richer `VccCudaRouteKind` style metadata over a single generic fallback string.

## Optional improvement C — explicit cache debug/test helpers

If useful for deterministic tests/benches, add small debug/test helpers under an appropriate cfg, for example cache stats or a helper that confirms cache reuse behavior.

## Optional improvement D — future shared resident-market alignment

Do **not** implement a shared repo-wide resident-market redesign now, but you may leave clearly marked TODOs or comments noting that VCC’s local price-state cache could later align with the broader shared resident-market pattern once VCC’s local benefits are measured.

---

# Explicit “do not do this first” rules

Do **not** spend the patch budget on these before the primary work is complete:

- a brand-new benchmark framework
- multi-stream scheduling for VCC
- speculative kernel rewrites to remove row packing entirely
- fusing or replacing vector_ta sweep APIs before parity is stronger
- broad app-wide GPU residency redesign
- removing CPU fallbacks
- removing current diagnostics/tests

Higher-risk future ideas such as direct row-remap kernel consumption or deeper shared price-state residency should only be considered after the safe staging/cache work and benchmark additions are in place.

---

# Implementation details to honor from the guides

Make sure the final code reflects the following specific intentions from the guides:

1. **Metrics-only vs detailed split is mandatory.**
   - The basic VCC metrics function must stop paying host slice readback cost.

2. **Strict combo/time validation is mandatory.**
   - Invalid row indices and time windows must fail before kernel launch.

3. **Thread-local cache limitation must be explicit.**
   - Warm-path wins are thread-affine under the current design.

4. **Use uninitialized memory only where every element is guaranteed written before read.**
   - Device outputs/packed matrices: good candidates.
   - Host readbacks: only via accepted shared helper or leave as reused initialized buffers.

5. **Keep kernel ABI stable in the first pass unless a later improvement is obviously safe.**

6. **Benchmark separation matters.**
   - Do not report only one undifferentiated VCC GPU timing mode after introducing caching.

7. **VCC param-shape matters.**
   - Default-only candidates are not enough to expose branch/divergence costs.

8. **Route evidence matters.**
   - GPU-labeled benches should assert they really used GPU.

---

# Deliverables

## Code deliverables

Modify the target files so the repo now contains the implemented VCC changes.

## Validation deliverables

Run the narrowest relevant checks available in the repo and report results.

At minimum, attempt the most relevant subset of:

- `cargo test` / targeted `cargo test -p <crate>` for the VCC tests you add
- `cargo check --tests --benches` for the affected crate(s)
- targeted benchmark compilation and, if feasible, execution for:
  - `gpu_native_expansion_opt`
  - `composite_validation_workflow`

Use the correct crate/package names and feature flags for this repo. If the CUDA benches cannot run in the environment, still make sure they compile and explain what blocked execution.

When relevant, use the needed features, likely including:

- `cuda`
- `cuda-backtest-kernel`

## Final report format

When you finish, provide:

1. a concise summary of what changed, file by file
2. the major performance improvements implemented
3. the copy-elision / uninitialized-memory choices you made and why they are safe
4. any deviations from the guides and why
5. test/bench/check results
6. any remaining follow-up items, clearly marked as optional

---

# Patch quality bar

Before finishing, verify all of the following:

- [ ] `vcc_gpu_kernel.rs` no longer forces host slice readback on the metrics-only path
- [ ] public VCC GPU metric APIs remain intact
- [ ] combo row/time validation exists and is used before kernel launch
- [ ] `VccCudaRuntime` now reuses meaningful VCC-specific buffers/state
- [ ] price-state reuse exists
- [ ] packed-matrix reuse exists
- [ ] combo/output scratch reuse exists
- [ ] a deterministic VCC cache-clear helper exists for cold/warm benches
- [ ] route/fallback metadata in `vcc.rs` is more visible than before
- [ ] tiny fixed-size param heap allocations in VCC helpers have been removed where practical
- [ ] at least one non-ignored VCC parity gate was added
- [ ] VCC optimizer Criterion coverage now exists
- [ ] VCC validation GPU coverage is stronger and asserts GPU route
- [ ] VCC benchmark labeling separates at least some cache layers (cold/warm/etc.)
- [ ] no VCC features were removed
- [ ] CPU fallback behavior still exists
- [ ] feature-gated builds remain correct

If something on the checklist cannot be completed cleanly, finish the rest, explain exactly why, and leave the smallest reasonable follow-up.

