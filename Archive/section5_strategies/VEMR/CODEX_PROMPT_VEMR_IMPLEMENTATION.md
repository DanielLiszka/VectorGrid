You are a coding agent working inside this Rust repo. Your job is to **fully implement the VEMR strategy-local GPU-native optimization, validation, and correctness-hardening plan** described in the markdown specs listed below. The work must stay focused on **VEMR only** and must prioritize **performance, lower copy overhead, warm-path reuse, and correctness** without removing features or breaking functionality.

## Read these documents first

These are the implementation specs. Read them before editing code.

1. `gpt54_feature_review_map.md`
   - Follow the spirit of **`## 5. Strategy-local GPU-native optimization and validation routes`**.
   - Also apply the review-map’s guidance around **allocation policy**, **copy-elision**, **overwrite-before-read**, and **benchmark/route evidence** where it directly affects VEMR.
2. `VEMR_GPU_native_optimization_plan.md`
   - This is the primary implementation guide.
3. `VEMR_GPU_native_optimization_plan_addendum.md`
   - This contains missing items, correctness guards, and extra performance improvements that should be folded into the implementation.

Important: if the workspace contains older draft variants of those docs (for example lowercase variants or `*_v2.md`), treat the two exact files above as the authoritative spec unless you need older drafts only for context.

## Primary files to inspect and modify

These are the main files in scope:

- `vemr.rs`
- `vemr_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

## Adjacent files you may inspect and edit only if necessary

Keep adjacent edits minimal and directly tied to VEMR implementation/wiring.

- the missing VEMR orchestration callsite(s) that currently build the GPU signal matrices or separately request Bollinger upper/middle/lower sweeps
- any shared allocation/copy-elision helper file such as `alloc_policy.rs` or equivalent, if the repo already has a canonical uninitialized-readback helper
- `gpu_backtest_kernel.rs` or related runtime/cache helpers, only as a model for grow-only scratch reuse, resident market-series caching, or launch-cap metadata patterns
- targeted VEMR test files or benchmark plumbing files if needed to wire tests/benches cleanly

Do **not** spread this into unrelated strategies or broad refactors. Keep the scope strategy-local to VEMR.

## High-level objective

Implement the full VEMR-local plan so that:

1. unnecessary indicator recomputation is removed,
2. unnecessary host↔device copies are reduced,
3. repeated device allocations in warm paths are reused,
4. host readback buffers stop zero-filling when they are fully overwritten,
5. VEMR-specific benchmark coverage exists and asserts the GPU route where appropriate,
6. correctness safeguards are tightened, especially around invalid combo indices/ranges and CPU/GPU parity,
7. existing features, blockers, fallback behavior, and public behavior are preserved unless a change is explicitly a correctness fix.

## Core rules and constraints

1. **Do not remove features.**
   - Preserve all current VEMR functionality, current route blockers, and CPU fallback behavior.
   - Do not remove overlays/SL/TP blockers from the VEMR GPU route unless the current codebase already supports them correctly and tests prove it.

2. **Do not break existing callsites.**
   - If you add a better bundled API, keep compatibility wrappers for old single-output helpers unless you can prove all callsites are migrated safely.

3. **Use uninitialized memory only when overwrite-before-read is provable.**
   - Prefer an existing shared helper if the repo already has one.
   - Only use `MaybeUninit`/uninitialized host vectors for buffers that are fully overwritten by `copy_to(...)` or equivalent before any read.
   - Do **not** mechanically replace every `vec![0; n]` or every initialized vector.
   - Keep behaviorally observable warmup values initialized if they matter semantically.

4. **Prefer grow-only reuse for scratch buffers.**
   - Only allocate when `required > capacity`.
   - Do not shrink on smaller subsequent calls.
   - Reuse scratch across repeated VEMR evals.

5. **Favor existing repo patterns over cargo-culting snippets.**
   - The markdown plan includes example code/snippets. Adapt them to the real codebase, naming, lifetimes, traits, and helper utilities already present.
   - If the repo already has a canonical helper for uninitialized copies, resident caches, or launch caps, reuse it rather than duplicating logic.

6. **Do not overreach into risky kernel rewrites unless clearly justified.**
   - The main implementation must focus on safe, local, high-payoff changes.
   - Do **not** fuse log-return generation into the main VEMR kernel unless the repository already makes that trivial and safe, all tests pass, and you can demonstrate it is still within scope. Otherwise leave it as a follow-up note/TODO.
   - Do **not** do pinned memory / async copy overlap / broad new CUDA specializations unless they are genuinely easy, low-risk, and the repo already supports them cleanly.

7. **Maintain feature gating.**
   - Respect existing `#[cfg(feature = "cuda")]`, `#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]`, and any existing bench/test gating patterns.

8. **Benchmark and test what you change.**
   - Add or update targeted tests.
   - Add VEMR Criterion coverage.
   - Ensure VEMR “GPU” benches fail loudly if they silently become CPU/fallback benches when those cases are expected to stay on GPU.

## Mandatory implementation tasks by file

---

# 1) `vemr.rs`

Implement the following.

## 1.1 Add a bundled Bollinger sweep result and helper

Add a bundled result type equivalent to the plan’s intent, for example:

- `VemrBollingerSweepDevice { upper, middle, lower }`

Then add a new helper equivalent to:

- `compute_vemr_bollinger_sweeps_cuda_device(...) -> Result<VemrBollingerSweepDevice, String>`

Requirements:

- Perform the direct CUDA Bollinger sweep once.
- Materialize all three outputs (`upper`, `middle`, `lower`) from that one run.
- Reuse the current `PreparedCudaIndicatorInputs` / `CudaBollingerBands` / `VemrDeviceMatrix` / `VemrPeriodSweepDevice` patterns already present in the file.
- Preserve existing semantics for period range normalization and fallback logic.

## 1.2 Keep compatibility wrappers

If existing code still expects separate helpers, preserve thin wrappers for:

- `compute_vemr_bollinger_upper_sweep_cuda_device(...)`
- `compute_vemr_bollinger_middle_sweep_cuda_device(...)`
- `compute_vemr_bollinger_lower_sweep_cuda_device(...)`

They may call the new bundled helper and return the relevant field.

## 1.3 Find and update the actual VEMR GPU callsite(s)

This is mandatory. Search the repo for the code that currently requests Bollinger upper/middle/lower separately for VEMR GPU prep/orchestration. Update that callsite so it performs **one** bundled Bollinger sweep call and passes the resulting three matrices onward.

Do not stop at adding the helper only; the benefit must be realized in the actual orchestration path.

## 1.4 Cache VEMR indicator keys/ids once

Add a `OnceLock`-style cache for resolved VEMR indicator param/output keys/ids, following the plan’s intent.

Requirements:

- Cache all VEMR-related resolved keys/ids once per process.
- Use that cache in `compute_vemr_fast_signals(...)` and `compute_vemr_risk_signals(...)`.
- Replace tiny heap-allocated parameter `Vec`s with stack arrays where the callee only needs a slice.
- Avoid repeated string/key resolution on the hot path.

## 1.5 Optional low-priority CPU-path fill optimization

Consider the `build_vemr_desired_positions(...)` prefix-init / overwrite-only allocation optimization from the plan.

Rules:

- Only implement this if overwrite-before-read is crystal clear and tests cover it.
- Prefer an existing shared helper if the repo has one.
- If the code would become fragile or semantically ambiguous, leave behavior unchanged and add a short TODO/note instead.

This is lower priority than the GPU staging changes.

---

# 2) `vemr_gpu_kernel.rs`

This is the main hot path. Implement the following carefully.

## 2.1 Strengthen `validate_inputs(...)`

The addendum calls this out as a missed correctness guard. Add validation for at least:

- `z_row_index < zscore_rows`
- `bb_row_index < bb_rows`
- `atr_row_index < atr_rows`
- `start_t >= 0`
- `end_t >= 0`
- `start_t <= end_t`
- `end_t <= cols`

Requirements:

- Reject invalid inputs early with clear error messages.
- Preserve existing shape/pointer checks.
- Add tests for these negative cases.

## 2.2 Expand `VemrGpuDeviceSignalMatrices` to support resident inputs

Modify the input struct/API so the VEMR GPU path can consume either:

- a host close slice, or
- a device-resident close pointer, and optionally
- a device-resident log-return pointer

Use names that fit the codebase, but the capabilities should match the plan, for example fields conceptually like:

- `close_host_f32: Option<&[f32]>`
- `close_device_ptr: Option<u64>` or equivalent device-pointer type
- `lr_device_ptr: Option<u64>` or equivalent device-pointer type

Requirements:

- Preserve backward compatibility where possible.
- Update validation logic to handle valid host-only and device-resident modes.
- Ensure the path is explicit and unambiguous: host close path, device close path, optional precomputed lr path.

## 2.3 Modify log-return launch integration

Update the log-return helper path so VEMR can derive log returns from an already resident device close pointer instead of forcing a host upload every call.

Requirements:

- Adapt `launch_compute_log_returns_f32(...)` or the equivalent internal launch path so it can launch from a raw/resident device close pointer when available.
- Preserve correctness and existing kernel launch behavior.

## 2.4 Add a bridge-stage resident `close` / `lr` cache inside `VemrCudaRuntime`

Even if upstream callers are not all immediately updated to pass device pointers, implement an internal runtime cache so repeated evals can reuse uploaded price data and derived log returns.

Add a resident-series cache conceptually equivalent to:

- `VemrSeriesCacheKey`
- `VemrResidentSeries { d_close, d_lr, key }`

Invalidation policy:

- Prefer a caller-provided generation/revision ID if the repo already has one available cleanly.
- Otherwise use a lightweight fingerprint plus length.
- Cache invalidation must correctly refresh when the underlying price series changes.

Requirements:

- Stable repeated evals on the same series should reuse `d_close` and `d_lr`.
- Same length but changed contents must invalidate correctly.
- Increased length should grow buffers once and then reuse them.

## 2.5 Add scratch-buffer reuse to `VemrCudaRuntime`

Implement a scratch block similar in spirit to the plan/addendum.

Buffers to reuse should include at least:

- close scratch (if still needed)
- lr scratch
- combo index buffers (`z`, `bb`, `atr`, `start`, `end`)
- metrics output buffer
- combo tags
- candidate-selection/top-k scratch
- top-k metrics readback staging buffers on device if applicable

Requirements:

- Use **grow-only** reuse, not exact-shape-only reuse.
- Only allocate when requested length exceeds capacity.
- Do not shrink on smaller requests.
- Reuse scratch across repeated calls via the existing per-device runtime cache.

## 2.6 Cache combo tags explicitly

Implement explicit combo-tag caching for the device-top-k route.

Requirements:

- Keep a grow-only device combo-tag buffer.
- Only refill when the combo count grows or the buffer contents are invalid.
- Avoid one host allocation and one H2D upload per evaluation when the combo count is unchanged.

## 2.7 Cache launch-cap / function metadata per runtime

Implement the safe version of metadata caching.

Requirements:

- Cache device launch caps once per runtime.
- Cache per-kernel launch metadata / preferred launch choices once if the existing code structure allows it cleanly.
- Avoid repeated capability detection and repeated symbol/launch-shape work on every eval.
- Do **not** introduce unsafe lifetime tangles around raw CUDA function handles if the current abstractions make that brittle.

## 2.8 Use uninitialized host readback where safe

Replace zero-filled host readback vectors that are immediately fully overwritten by device `copy_to(...)` calls.

Apply this to the VEMR readback paths called out in the plan/addendum, including the equivalents of:

- `host_combo_tags`
- `host_top_metrics`
- `host_out`

Requirements:

- Prefer an existing repo helper (for example from `alloc_policy.rs`) if present.
- Otherwise add a small local helper for uninitialized host allocation + safe `assume_init` conversion.
- Add a brief safety comment explaining why overwrite-before-read is guaranteed.
- Do **not** broaden `MaybeUninit` usage to buffers whose initial values are behaviorally observed.

## 2.9 Preserve existing route behavior and blockers

Do not change the feature behavior of:

- overlays blocking the VEMR GPU route
- stop-loss blocking the VEMR GPU route
- take-profit blocking the VEMR GPU route

Add tests/bench-route assertions so these blockers remain visible and intentional.

---

# 3) `gpu_native_expansion_opt.rs`

This file needs real VEMR optimizer coverage.

## 3.1 Add VEMR imports and request builders

Add VEMR support in the optimizer bench file, including:

- importing `VemrStrategyParams`
- adding a VEMR request builder similar to `make_vemr_req(...)`

Then add explicit VEMR request shapes that differentiate the GPU route shapes called out in the plan:

1. **pure device-top-k path**
   - `top_k = Some(20)`
   - `include_all = Some(false)`
   - `heatmap_bins = Some(0)`
2. **heatmap / broader readback path**
   - `top_k = Some(20)`
   - `include_all = Some(false)` or `Some(true)` depending on repo semantics
   - `heatmap_bins = Some(64)`

Be careful here: do **not** accidentally benchmark the wrong route shape by blindly copying `base_req(...)` defaults. The point is to separate the pure device-top-k VEMR path from the broader result-shaping path.

## 3.2 Add a VEMR-shaped candle generator

Add a VEMR-appropriate synthetic candle generator instead of reusing a generic trend-only shape.

Requirements:

- make it mean-reversion friendly / oscillatory enough to be relevant to VEMR
- keep it deterministic
- follow the style already used by existing strategy-specific bench generators

## 3.3 Add VEMR Criterion benches

Add at minimum:

- a VEMR CPU benchmark
- a VEMR GPU benchmark for the pure device-top-k path
- a VEMR GPU benchmark for the heatmap/full-readback path

Update `criterion_group!(...)` accordingly.

## 3.4 Add optional benchmark shapes that improve route evidence

If practical, also add:

- a VEMR blocked-route benchmark/request shape that intentionally trips overlays/SL/TP blockers and asserts/logs the fallback clearly
- a VEMR non-default z-score configuration benchmark to quantify the fast-path vs non-default path difference

These are valuable, but the primary requirement is that VEMR optimizer Criterion coverage exists and explicitly distinguishes top-k vs broader result-shaping routes.

---

# 4) `composite_validation_workflow.rs`

Tighten the VEMR validation benchmark so it is real evidence, not a silent-fallback benchmark.

## 4.1 Add CUDA availability guard

Match the CUDA availability guard pattern already used elsewhere so the GPU validation bench does not misbehave on non-CUDA environments.

## 4.2 Add a VEMR-shaped validation candle generator

Add a VEMR-relevant synthetic candle generator and use it for VEMR validation cases rather than only the generic validation candle shape.

## 4.3 Update `bench_profile(...)`

Extend the validation benchmark helper so it can:

- accept an `expect_gpu` flag (or equivalent)
- use VEMR-specific candle generation for VEMR cases
- route-probe before benchmark iteration
- assert the GPU backend/route for VEMR cases that are supposed to stay on GPU

## 4.4 Add stronger VEMR validation coverage

At minimum include VEMR cases equivalent to:

- `50k / 8 candidates`
- `50k / 32 candidates`
- `100k / 32 candidates`

Requirements:

- keep the existing smaller case for continuity
- add the larger cases so candidate-count scaling and bars×candidates scaling are visible
- fail loudly if a VEMR case expected to stay on GPU silently becomes CPU/fallback

---

# 5) Tests and parity work

Implement the test hardening from the plan and addendum.

## 5.1 `vemr.rs` tests

Add or extend tests for `build_vemr_desired_positions(...)` branch behavior, covering at least:

- `confirm_bars`
- `require_band_touch`
- `allow_flip`
- `trade_on_next_bar`

Also keep/extend the period mapping tests.

## 5.2 `vemr_gpu_kernel.rs` tests

Add unit tests for:

- invalid `z_row_index`
- invalid `bb_row_index`
- invalid `atr_row_index`
- negative `start_t`
- `end_t < start_t`
- `end_t > cols`
- host-close vs device-close validation modes
- resident close/lr cache reuse and invalidation behavior if the runtime API is testable directly
- any new uninitialized readback helper safety assumptions that can be tested in a lightweight way

## 5.3 Integration/parity tests

Add targeted CPU/GPU parity tests for VEMR, especially around threshold-sensitive behavior:

- values near `z_entry`
- values near `z_exit`
- `require_band_touch`
- `confirm_bars > 0`
- `allow_flip`
- `trade_on_next_bar`
- ATR stop distance behavior if the route supports it on the compared path

The purpose is to catch `f64` vs `f32` threshold drift and ensure the optimized staging changes do not break semantics.

## 5.4 Route-blocker tests

Add tests that confirm the VEMR GPU route still blocks and/or clearly falls back when:

- execution overlays are enabled
- stop loss is enabled
- take profit is enabled

The goal is not to remove those blockers; it is to ensure they remain explicit and visible.

---

# 6) Optional beyond-scope improvements

These are allowed **only if** they are low-risk, easy in the current repo, and do not derail the main work. Otherwise leave them as follow-up notes.

Allowed only if cheap and safe:

- shaping the VEMR runtime/cache API so it aligns with existing `gpu_backtest_kernel` warm-path conventions
- using a caller-provided generation ID instead of a content fingerprint for resident series invalidation if such an ID already exists nearby
- removing a small host-side top-k sort only if the kernel/device path already guarantees the desired order and tests prove it

Do **not** spend the task on risky extras like:

- fusing log-return generation into the main VEMR kernel
- broad new CUDA specializations for non-default z-score configs
- pinned memory / async copy overlap experiments

unless those are already almost done in the repo and clearly safe.

---

# 7) Performance-specific guidance

The main interest is performance, optimization, and correctness. Keep these priorities in mind while editing.

1. **Biggest expected wins first**
   - bundled Bollinger sweeps
   - resident close/lr reuse
   - grow-only scratch reuse
   - uninitialized host readback for full-overwrite buffers
   - explicit VEMR benchmarks and GPU-route assertions

2. **Limit unnecessary copies**
   - avoid host→device re-upload of unchanged close data
   - avoid recomputing unchanged log returns
   - avoid repeated combo-tag uploads
   - avoid repeated device allocations on warm paths
   - avoid host zero-fill before full readback overwrite

3. **Use uninitialized memory conservatively and intentionally**
   - safe for device readback buffers that are fully overwritten
   - not safe as a blanket policy

4. **Preserve correctness under optimization**
   - add guards before touching hot paths
   - add tests alongside performance changes
   - do not trade away determinism or route visibility

---

# 8) Execution order

Implement in roughly this order unless the real repo forces a different dependency order.

## Phase A — correctness + biggest performance wins

1. inspect the current VEMR call graph and find the actual orchestration callsite(s)
2. add bundled Bollinger sweeps in `vemr.rs`
3. migrate the actual VEMR GPU prep/orchestrator to use the bundled sweep once per request
4. strengthen `validate_inputs(...)`
5. add resident close/lr support and bridge-stage runtime caching
6. add grow-only scratch reuse
7. add combo-tag caching
8. add uninitialized host readback for safe buffers

## Phase B — benchmark coverage and route evidence

1. add VEMR optimizer Criterion coverage in `gpu_native_expansion_opt.rs`
2. separate pure device-top-k and heatmap/full-readback VEMR bench shapes
3. add VEMR validation GPU-route assertions and larger scaling cases in `composite_validation_workflow.rs`
4. add at least one blocked-route case and one non-default z-score bench/test if practical

## Phase C — smaller/local cleanup

1. add `OnceLock` key/id caching and stack-array param slices in `vemr.rs`
2. only then consider the optional `build_vemr_desired_positions(...)` allocation micro-optimization if still worthwhile and clearly safe

## Phase D — tests, parity, and final verification

1. add/extend all targeted tests
2. run formatting and compile/test steps
3. run targeted VEMR benches or bench-compilation checks
4. summarize results and any remaining follow-ups

---

# 9) Validation and commands

Use the repo’s normal toolchain, but at minimum do as much of the following as the workspace supports:

- format the changed Rust files
- run targeted tests for VEMR and GPU kernel validation
- run benchmark builds or targeted Criterion benches for the new VEMR cases if benches are runnable in this environment
- if full GPU execution is not possible in the environment, still ensure the code compiles under the relevant feature flags and summarize what could not be runtime-verified

Suggested validation targets (adapt to the actual crate/package names and test paths):

- VEMR unit tests
- `vemr_gpu_kernel` tests
- parity/integration tests for VEMR
- benchmark compilation for:
  - `gpu_native_expansion_opt`
  - `composite_validation_workflow`

If you cannot run a GPU bench in the environment, still make sure the code compiles, the route assertions are correct, and the new request builders/bench wiring are syntactically and semantically sound.

---

# 10) Deliverables

When finished, provide:

1. a concise summary of the files changed
2. a bullet list of the implemented improvements by file
3. notes on any adjacent files that had to be changed and why
4. the validation/tests/bench commands you ran and the results
5. any measurement-gated follow-ups you intentionally left out and why

Be explicit about whether you found and updated the missing VEMR orchestration callsite that was separately requesting Bollinger upper/middle/lower. That wiring change is required for the main optimization to actually take effect.

---

# 11) Non-negotiable acceptance criteria

Do not consider the task complete unless all of the following are true, except where the repo genuinely lacks the needed adjacent wiring and you document exactly why.

- VEMR has a bundled Bollinger sweep path that computes upper/middle/lower in one run.
- The actual VEMR GPU orchestration path uses that bundled sweep instead of three separate calls.
- `validate_inputs(...)` rejects invalid combo row indices and invalid time ranges.
- The VEMR GPU path can consume device-resident close data, directly or via a runtime resident-series bridge.
- The VEMR GPU path can reuse derived log returns across repeated evals when the series is unchanged.
- VEMR scratch buffers are reused with a grow-only policy.
- Combo-tag uploads are cached/reused.
- Safe uninitialized host readback is used where buffers are fully overwritten.
- VEMR optimizer Criterion coverage exists.
- VEMR validation Criterion coverage exists and asserts the GPU route for VEMR GPU cases.
- VEMR route blockers remain visible and tested.
- CPU/GPU parity coverage is improved around threshold-sensitive behavior.
- Existing VEMR features and behavior are preserved aside from explicit correctness fixes.

Work end-to-end and implement the plan rather than only summarizing it.
