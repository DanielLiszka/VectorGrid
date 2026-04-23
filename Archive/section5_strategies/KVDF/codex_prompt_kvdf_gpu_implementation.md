# Prompt for Codex agent: implement the KVDF GPU optimization plan end-to-end

You are working inside a Rust repo that already contains KVDF strategy code, a KVDF CUDA scoring path, and Criterion benchmarks. Your job is to **fully implement the KVDF-only optimization/correctness plan** described in the attached markdown guides, while preserving existing functionality and keeping the work tightly scoped to the `## 5. Strategy-local GPU-native optimization and validation routes` bundle from the feature-review map.

## Read these documents first

These documents are authoritative for scope and implementation intent. Read them before changing code:

1. `gpt54_feature_review_map.md`
   - If present in this workspace, likely at: `/mnt/data/gpt54_feature_review_map.md`
   - Scope anchor: `## 5. Strategy-local GPU-native optimization and validation routes`
2. `kvdf_gpu_optimization_plan.md`
   - If present in this workspace, likely at: `/mnt/data/kvdf_gpu_optimization_plan.md`
3. `kvdf_gpu_optimization_plan_addendum.md`
   - If present in this workspace, likely at: `/mnt/data/kvdf_gpu_optimization_plan_addendum.md`
   - **Important:** when the addendum and the main plan differ in priority or specificity, **the addendum wins**.

If those exact paths do not exist, locate the files by basename in the repo/workspace and use those copies.

## Primary files to modify

Target these files first. If the repo paths differ slightly, locate them by basename/symbol:

- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

You may also modify closely related supporting files **only if required to complete the plan cleanly**:

- `crates/app_core/src/alloc_policy.rs` (or equivalent alloc helper module)
- upstream KVDF GPU row-builder / caller files that currently consume `compute_kvdf_signals(...)` and/or call `eval_kvdf_metrics_gpu_kernel_device_rows(...)`
- closely related test modules or inline `#[cfg(test)]` sections
- shared helper modules only when necessary for a parity-safe implementation

Do **not** broaden scope into unrelated strategies or generic rewrites.

## Hard constraints

Follow all of these constraints:

1. **KVDF only.** Stay strictly within the KVDF strategy-local GPU-native optimization/validation surface.
2. **Do not remove features.** Preserve existing CPU and GPU functionality.
3. **Prefer parity-safe improvements first.** Fix correctness and staging waste before speculative redesign.
4. **Keep CPU/reference behavior intact.** Add parallel GPU-prep surfaces rather than mutating CPU/reference APIs unless absolutely necessary.
5. **No silent benchmark contamination.** GPU benchmarks must assert they actually stayed on GPU.
6. **Minimize unnecessary memory copies.** Reduce host/device staging and avoid widening/narrowing churn.
7. **Use uninitialized memory only when fully justified.** Only use uninitialized allocation when every element is guaranteed to be written before any read.
8. **Prefer central alloc helpers.** If an `alloc_policy` helper exists, use it; if not, add one centrally rather than scattering ad hoc unsafe allocation code.
9. **No broad architecture rewrite.** Keep the work incremental, measurable, and compatible with existing code paths.
10. **Do not weaken tests.** Tighten route, parity, and blocker assertions where appropriate.
11. **Keep fallback visibility.** Make GPU-prep fallback reasons observable rather than silently swallowed.
12. **Use checked integer conversions.** Replace narrowing casts that can overflow.
13. **Be benchmark-aware.** Preserve and strengthen existing Criterion/perf-gate surfaces instead of inventing unrelated measurement harnesses.

## High-level goals

Implement the plan so that KVDF achieves all of the following:

- The GPU-prepared KVDF signal path does **not** unnecessarily materialize `HostF32 -> Vec<f64>` for GPU-bound workflows.
- The KVDF GPU scoring path can consume **resident close/log-return inputs** and reuse scratch buffers rather than reallocating/staging every call.
- KVDF GPU validation/optimizer benchmarks explicitly assert **actual GPU routing** and absence of hidden fallback contamination.
- KVDF CPU vs GPU semantics are aligned, especially around `dti_threshold` and route/blocker behavior.
- Uninitialized memory usage is adopted **only** in proven full-write cases.
- Larger KVDF validation benchmark shapes exist in the current benchmark harness.
- Existing features continue to work; correctness and parity improve.

---

# Required implementation work

## 1) `kvdf.rs`

### Required correctness fixes

1. **Fix `dti_threshold` CPU/GPU semantic mismatch immediately.**
   - The current CPU path uses the raw threshold while the GPU wrapper normalizes with `abs()`. This can cause CPU/GPU divergence for negative inputs.
   - Tighten validation so `dti_threshold` must be **finite and strictly > 0**.
   - Add a shared normalization helper (for example `normalized_kvdf_dti_threshold(...)`) and use the same normalized semantics in both CPU and GPU code.
   - Preserve existing feature behavior except for the stricter validation of invalid threshold inputs.

2. Add/strengthen tests that reject non-positive or non-finite `dti_threshold` values.

### Required performance/prep-path work

3. **Cache resolved dynamic indicator output ids** with `OnceLock`.
   - Introduce a cached bundle (for example `KvdfOutputIds`) for:
     - Keltner upper / middle / lower
     - VWAP value
     - DTI value
     - DevStop value
     - FVG trailing stop upper / lower
   - Replace repeated `resolve_output_id(...)` calls with cached ids.

4. **Replace outer parameter `Vec`s with fixed arrays/slices** where practical in KVDF signal-build helpers.
   - The goal is to reduce repeated tiny heap allocations in the signal prep path.
   - Keep compatibility with downstream APIs that still need owned strings/values.

5. **Add a GPU-preferred `f32` prep surface for KVDF.**
   - Keep the current `compute_kvdf_signals(...) -> KvdfSignals` API intact for CPU/reference use.
   - Add a parallel GPU-bound surface, e.g.:
     - `KvdfSignalsF32`
     - `KvdfPrepTraceEntry`
     - `KvdfPrepTrace`
     - `series_to_f32(...)`
     - `compute_indicator_f32_for_gpu(...)`
     - `compute_kvdf_signals_f32_for_gpu(...)`
   - This new path must:
     - prefer CUDA indicator execution when the backend prefers GPU and prepared CUDA inputs exist,
     - request/consume `HostF32` without immediately widening to `Vec<f64>` for GPU-bound work,
     - fall back to CPU when needed,
     - record per-indicator `backend_used` and `fallback_reason`,
     - compute `first_valid`,
     - preserve CPU/reference entrypoints.

6. **Keep fallback visibility.**
   - If CUDA indicator prep fails or prepared inputs are missing, record the fallback reason in the trace rather than silently hiding it.

7. **Optionally but preferably:** add a strict signal-length validation helper before switching any CPU fallback loops to direct indexing.
   - Example intent: verify all signal vectors match `close.len()` before any direct indexing optimization.
   - Only use direct indexing where the invariant is explicitly established.

8. **Optionally but preferably:** if `first_valid` can be computed during conversion/build rather than rescanning later, do so.

### Required tests in or near `kvdf.rs`

9. Add/strengthen tests for:
   - `KvdfSignalsF32` parity vs the existing CPU/reference signal path,
   - prep-trace visibility when GPU is preferred but CUDA prep is unavailable,
   - stricter `dti_threshold` validation,
   - existing smoke coverage remaining intact.

10. **Strengthen the current GPU smoke test.**
    - The existing FVG assertions are too weak.
    - Make them prove the surfaces have meaningful shape/content, e.g. finite-or-NaN invariants and at least some finite values where expected.
    - If the new prep-trace exists in this test path, assert supported indicator families actually used CUDA when appropriate rather than silently falling back.

---

## 2) `kvdf_gpu_kernel.rs`

This file is the highest-priority performance surface after the `dti_threshold` fix.

### Required correctness and validation work

1. **Strengthen `validate_inputs(...)` early.**
   Add cheap, explicit pre-launch validation for all of the following:
   - non-empty series,
   - equal combo-array lengths,
   - `stride >= cols` for all row-major matrices,
   - row-counts > 0,
   - row indices within bounds,
   - `start_t <= end_t`,
   - `end_t <= cols`,
   - checked `usize -> i32` conversions,
   - device-close / host-close input invariants once the new interface exists.

2. Introduce a checked conversion helper (e.g. `to_i32_checked(...)`) and replace unchecked narrowing casts.

3. **Replace raw flag bits with named constants** for kernel contract clarity.
   - Do not leave magic numbers if the wrapper currently uses them.

4. Keep the blocker behavior for unsupported GPU features (execution overlays / stop-loss / take-profit), but add stronger tests around it.

### Required residency, scratch, and copy-reduction work

5. **Add runtime-owned scratch reuse.**
   The runtime currently caches context/module/stream but not hot buffers. Add reusable scratch storage, for example:
   - `d_close`
   - `d_lr`
   - `d_combo_fast`
   - `d_combo_slow`
   - `d_combo_dti`
   - `d_combo_start`
   - `d_combo_end`
   - `d_out`
   - reused host readback storage
   - optional last-seen residency keys

   A shape like `KvdfKernelScratch` owned by `KvdfCudaRuntime` is appropriate.

6. **Cache device launch caps in the runtime** rather than re-detecting them every call.

7. **Extend the KVDF GPU input surface to support resident market data.**
   Implement a parity-safe interface that allows the caller to provide:
   - a host close slice when needed,
   - an optional device-resident close pointer,
   - an optional device-resident log-return pointer.

   A straightforward way is to extend `KvdfGpuDeviceSignalMatrices` with fields such as:
   - `close_f32: Option<&[f32]>`
   - `close_device_ptr: u64` or an equivalent typed/raw pointer wrapper
   - `lr_device_ptr: u64` or equivalent
   - `close_key: Option<KvdfMarketResidencyKey>` if safe upload skipping is implemented

   If a cleaner enum-based input source model is better in the actual codebase, use that, but keep semantics equivalent.

8. **Add a raw-pointer/device-pointer log-return launcher path** so log returns do not always require a host-uploaded close series.

9. **Resolve close/lr pointers once per call** and then launch the metrics kernel.
   - Skip host close upload when a valid device-resident close pointer is provided.
   - Skip log-return recomputation when a valid device-resident log-return pointer is provided.

10. **Differentiate reuse from copy elision.**
    - Buffer reuse alone is always okay if overwritten.
    - Skipping an upload is only safe when you know the existing device buffer still holds the correct logical data.

11. **Add explicit residency keys for safe copy elision.**
    - Example shapes:
      - `KvdfMarketResidencyKey { series_id, cols }`
      - `KvdfComboResidencyKey { batch_id, combo_count }`
    - Only skip close/combo uploads when the caller provides a matching key and the runtime scratch still holds that exact logical content.
    - Do **not** skip uploads based only on `len` or shape.

12. **Stage implementation in the safest order:**
    - first reuse scratch allocations,
    - then add close reuse,
    - then add copy elision based on explicit residency keys and/or device-native inputs.

### Required uninitialized-memory discipline

13. **Use uninitialized allocations only in proven full-write cases.**
    Apply this rule explicitly:
    - `d_lr` may remain uninitialized only if the log-return kernel writes every element before any read.
    - `d_out` may remain uninitialized only if the metrics kernel writes every output slot before any read.
    - host readback may use uninitialized memory only if the device copy overwrites the full logical buffer.

14. **Prefer a shared alloc helper** from `alloc_policy.rs` (or equivalent). If no such helper exists, add one centrally rather than writing scattered local unsafe code.

15. **Before switching host readback to uninitialized allocation, add a full-write proof test.**
    A sentinel-style test is fine:
    - initialize output with a sentinel,
    - launch kernel,
    - copy back,
    - assert no sentinel remains in any logical output slot.

### Required tests in or near `kvdf_gpu_kernel.rs`

16. Add/strengthen tests for:
    - empty-combo early return,
    - blocker path for execution overlays / stop-loss / take-profit,
    - device-close validation path once the new interface exists,
    - full-write proof for `d_out` before adopting uninitialized host readback,
    - scratch-reuse correctness across repeated launches,
    - flag-coverage CPU vs GPU parity fixture.

17. **Add a strong parity fixture** that exercises non-default behavior, not just default config.
    At minimum cover:
    - one fixed candle set,
    - one KVDF candidate,
    - `long_only = true`,
    - `allow_flip = false`,
    - `trade_on_next_bar = true`,
    - nonzero commission,
    - non-default positive `dti_threshold`,
    - CPU reference metrics vs GPU metrics with documented tolerances.

18. Replace the local GPU wrapper `dti_threshold.abs()` logic with the shared normalized helper from `kvdf.rs` (or a shared helper module) so CPU and GPU use identical semantics.

---

## 3) `gpu_native_expansion_opt.rs`

### Required work

1. Keep existing KVDF optimizer GPU route assertions strict.
   - Do not weaken them.
   - If practical, additionally assert `backend_fallback_reason.is_none()` for GPU benchmark routes.

2. If the current repo version still lacks large/shape-parity KVDF CPU coverage relative to GPU coverage, add the missing KVDF CPU case(s). If current CPU/GPU KVDF shape coverage is already symmetric, keep it symmetric and do not invent redundant cases.

3. If the new KVDF prep trace metadata is available in the route probe, print it in debug/probe output without making the benchmark noisy by default.

4. Keep KVDF-specific candle generation / request shaping aligned with the existing strategy-local benchmark style.

### Required validation of benchmark behavior

5. Re-run KVDF-only optimizer benchmarks after each major phase and confirm:
   - route stays on GPU,
   - fallback reason is absent for GPU benchmark routes,
   - benchmark labels remain coherent.

---

## 4) `composite_validation_workflow.rs`

### Required work

1. **Import and assert actual GPU backend usage** in GPU validation route probes.
   - Do not merely print the route summary; fail if the route fell back.
   - Also assert fallback reason is absent for intended GPU benchmark paths when practical.

2. **Add a KVDF-specific GPU validation workflow benchmark at `100k / 64 candidates`.**
   - This is a required coverage gap to close.
   - Mirror the existing style of the ASR benchmark group rather than inventing a new harness family.

3. **Add at least one larger KVDF profile benchmark case** beyond the existing small KVDF validation shape.
   - Preferred: `100k / 32` and optionally `100k / 64`, depending on current harness style and runtime cost.

4. Add/use KVDF-shaped candles for KVDF-specific validation groups.

5. If the new KVDF prep trace is available, include it in route probes/debug output where useful.

### Required validation of benchmark behavior

6. Re-run KVDF validation benchmarks and confirm:
   - GPU route is asserted, not merely logged,
   - new KVDF `100k / 64` workflow case compiles and runs,
   - larger KVDF profile case(s) exist and use KVDF-shaped candles.

---

## 5) Upstream caller migration

If the repo contains upstream KVDF row-builders/callers that currently:
- consume `compute_kvdf_signals(...)` for GPU-bound work, or
- call `eval_kvdf_metrics_gpu_kernel_device_rows(...)`,

then update those callers so the new performance path is actually used.

### Required migration behavior

1. GPU-bound KVDF row-builders should prefer the new `KvdfSignalsF32` path instead of forcing GPU-prepared results through `Vec<f64>`.
2. Provide `close_device_ptr` / device-close input when close data is already resident.
3. Provide `lr_device_ptr` when log returns are already resident or precomputed.
4. If copy elision is implemented, plumb residency keys through callers only when the caller can guarantee key correctness.
5. Keep CPU/reference callers on the existing `KvdfSignals` path unless there is a strong reason to consolidate.

If the exact caller files are not obvious, locate them by symbol/reference and patch all affected call sites so the new code path is reachable in real execution, not just in isolated tests.

---

# Ordering and priority

Implement in this order unless the actual code structure forces a small adjustment:

## Phase 1: highest-priority correctness and low-risk ROI

1. Fix `dti_threshold` CPU/GPU consistency.
2. Strengthen `validate_inputs(...)`.
3. Replace raw flag bits with named constants.
4. Cache `device_caps` in `KvdfCudaRuntime`.
5. Add scratch reuse for `d_lr`, combo buffers, `d_out`, and host metrics.
6. Add GPU validation route assertions.
7. Add KVDF `100k / 64` GPU validation benchmark case.

## Phase 2: copy-elision and prep cleanup

8. Add `KvdfOutputIds` caching in `kvdf.rs`.
9. Replace outer parameter `Vec`s with fixed arrays/slices.
10. Add the `KvdfSignalsF32` / prep-trace GPU-prep surface.
11. Migrate GPU-bound callers to use the new `f32` path.
12. Add device-native close input.
13. Add optional device-native log-return input.
14. Add residency keys for safe upload skipping.

## Phase 3: uninitialized readback and stronger tests

15. Add full-write proof tests.
16. Route host readback through a central uninitialized alloc helper if full-write guarantees are established.
17. Strengthen smoke/parity/flag-coverage tests.
18. Add scratch-reuse correctness tests.

## Phase 4: optional warm-path work (only after all required work is green)

These are **stretch goals**. Implement them only if they are clean, measurable, and do not destabilize the required work:

- pinned async DtoH readback,
- async device copies/allocations where the CUDA API surface supports it cleanly,
- CUDA graph replay for stable repeated KVDF launch shapes,
- closer alignment with the broader shared residency model used elsewhere in the repo,
- multi-output dynamic-indicator family helpers if adjacent files prove repeated family recomputation waste,
- borrowed parameter surfaces in dynamic indicator dispatch,
- CPU desired-position micro-optimization if still measurable after the GPU-path fixes.

If any stretch goal is too invasive or too uncertain, do **not** force it. Leave it out and mention it in your final report as deferred.

---

# Acceptance criteria

The implementation is not done until all of the following are true:

1. KVDF optimizer GPU benches still route to GPU and assert that fact.
2. KVDF validation GPU benches still route to GPU and assert that fact.
3. Intended GPU benchmark paths do not carry a fallback reason.
4. The GPU-bound KVDF prep path no longer widens CUDA `HostF32` outputs into `Vec<f64>` unnecessarily.
5. The KVDF scoring kernel can consume resident close/log-return inputs when available.
6. Scratch reuse is in place for repeated KVDF GPU launches.
7. Copy elision, if implemented, is guarded by explicit residency keys or device-native provenance rather than shape-only guesses.
8. `dti_threshold` semantics are identical across CPU and GPU paths.
9. Strong parity tests pass for non-default flag combinations.
10. Larger KVDF validation workload shapes exist in the current validation benchmark harness.
11. All new uninitialized memory usage is centrally justified by documented full-write guarantees.
12. CPU/reference behavior remains intact.
13. No existing feature was removed.

---

# What to run before finalizing

Use the repo’s existing commands/feature flags, but at minimum run the closest targeted equivalents of:

1. relevant KVDF unit tests,
2. targeted GPU-kernel tests for KVDF,
3. `gpu_native_expansion_opt` with KVDF enabled,
4. `composite_validation_workflow` with KVDF coverage enabled,
5. compile/test with the necessary CUDA feature flags that the repo already uses (for example `cuda`, `cuda-backtest-kernel`, or repo equivalents),
6. a non-CUDA build path if your changes touched shared CPU/reference code.

If exact commands differ in the repo, infer and use the closest correct ones.

---

# Final response format

When you finish, provide a concise but complete implementation report with:

1. **Files changed**
   - list every modified file
2. **What was implemented**
   - grouped by `kvdf.rs`, `kvdf_gpu_kernel.rs`, benches, and any supporting file
3. **Correctness fixes**
   - explicitly mention the `dti_threshold` fix and validation changes
4. **Performance changes**
   - explicitly mention staging/copy reductions, scratch reuse, residency support, and any uninitialized-memory changes
5. **Tests/benches run**
   - exact commands or closest equivalents
6. **Results**
   - whether GPU routes stayed on GPU, whether fallback reasons were absent, and any observed benchmark/perf deltas
7. **Anything deferred**
   - only for stretch goals or truly blocked optional items

Do not claim optional/stretch items were implemented unless they actually were.

---

# Practical reminders

- Start from correctness and route truth, then reduce copies.
- Reuse allocations first; only skip uploads when provenance is explicit.
- Keep CPU/reference APIs stable where possible.
- Avoid local unsafe allocation one-offs; centralize them.
- Tighten assertions rather than loosening them.
- Do not drift into other strategies.

Implement the plan fully and carefully.
