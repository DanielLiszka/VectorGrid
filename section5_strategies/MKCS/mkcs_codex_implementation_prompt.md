# Codex implementation prompt: MKCS GPU optimization, copy-elision, and correctness plan

Use the prompt below with an OpenAI Codex agent.

---

You are working in an existing Rust/CUDA workspace. Your task is to **implement the MKCS strategy-local GPU optimization, benchmark, and correctness plan** described in the local Markdown specs.

## Read these documents first, in this order

1. `gpt54_feature_review_map.md`
   - Read **only** the scope and the section:
     - `## 5. Strategy-local GPU-native optimization and validation routes`
   - Treat this file as the **scope guardrail**.

2. `mkcs_gpu_optimization_review_plan_v2.md`
   - This is the **authoritative implementation plan**.

3. `mkcs_gpu_optimization_review_plan.md`
   - This is the original plan. Use it only for extra detail where it does **not** conflict with v2.

4. `mkcs_gpu_optimization_review_plan_addendum.md` (if present in the workspace)
   - Treat it as historical/supplemental context only.
   - If anything conflicts, **v2 wins**.

## Scope guardrails

Stay tightly inside the posture described by the review-map Section 5 scope:

- **one strategy at a time**
- **MKCS only**
- focus on:
  - strategy-local hot paths
  - CPU/GPU payoff
  - staging and fallback behavior
  - per-strategy benchmark coverage
  - parity-safe improvements
- do **not** drift into:
  - unrelated strategies
  - broad architecture rewrites
  - UI work
  - speculative new knobs
  - GPU expansion claims without route/fallback/parity evidence

## Primary repo files to inspect and modify

These are the core files expected to change:

- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

## Adjacent files you may touch only if required

You may make the **smallest possible** supporting edits in adjacent files if they are needed to fully land the MKCS plan, especially for device-native sweep or combined-output support:

- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- any small local alloc/cuda helper module already used by the above files
- any tightly related CUDA/runtime helper module already referenced by the MKCS path

Do **not** broaden the scope beyond what is minimally necessary to compile and complete the MKCS-local implementation.

## Hard constraints

These are mandatory:

- preserve existing features and behavior
- do **not** remove functionality
- do **not** break MKCS CPU fallback behavior
- do **not** break unsupported-feature rejection behavior
- do **not** widen the change into unrelated strategy work
- do **not** add generic architecture redesigns
- keep the implementation **benchmark-first** and **parity-preserving**
- reduce unnecessary copies and allocations
- use uninitialized memory only when it is provably safe
- prefer smaller, safe changes over large speculative rewrites
- when docs conflict: `mkcs_gpu_optimization_review_plan_v2.md` wins

## Mission

Implement the MKCS plan so that the codebase materially improves the following, without losing features or correctness:

1. **Reduce staging waste and unnecessary memory movement**
   - especially GPU -> host -> GPU round-trips
   - especially repeated host->device uploads for unchanged market data or unchanged combo data
   - especially repeated host zero-fills before full overwrite

2. **Reduce allocation churn**
   - reuse device scratch
   - reuse host readback scratch where safe
   - avoid rebuilding small hot-loop vectors and IDs repeatedly

3. **Improve correctness and route clarity**
   - strengthen device/shape/input validation
   - make cache identity safe
   - preserve or improve fallback transparency
   - preserve parity-sensitive threshold behavior

4. **Improve MKCS benchmark and validation evidence**
   - add optimizer benches for MKCS
   - add heavier MKCS validation benches
   - separate cold-path vs steady-state evidence
   - keep route/fallback visibility

5. **Improve MKCS tests**
   - add CPU/GPU parity tests
   - add threshold-edge tests
   - add device-mismatch and cache-key regression tests

## Important implementation note

The Markdown plans contain example code snippets. Treat them as **intent and shape guidance**, not rigid literal text. Adapt them to the actual types, modules, helper APIs, and feature gates that exist in the repo.

If the current APIs differ, preserve the **design intent** from the plan:

- eliminate avoidable copies
- reuse buffers
- keep MKCS-local scope
- preserve parity and functionality
- add the missing benchmark/test coverage

---

# Required implementation work

## 1) `crates/app_core/src/mkcs.rs`

### A. Cache resolved indicator keys and output IDs

Implement a cached strategy-local resolver for MKCS indicator IDs and output IDs so the file does not repeatedly scan descriptors and allocate temporary strings.

The intended cached values include at least:

- stddev period param key
- stddev output ID
- Donchian period param key
- Donchian upper output ID
- Donchian lower output ID

Use a safe one-time cache such as `OnceLock` or an equivalent existing pattern in the repo.

### B. Reuse the period parameter vector in the CUDA host-row sweep helper

In or around:

- `compute_indicator_period_sweep_cuda_host_rows(...)`

avoid building a fresh `Vec<IndicatorParamKV>` and a fresh period-key string for every period in the sweep loop.

Requirements:

- allocate the parameter vector once
- mutate only the period value each iteration
- preserve exact output semantics
- keep validation of row length vs expected `cols`

### C. Tighten the CPU reference path without changing semantics

In or around:

- `build_mkcs_desired_positions(...)`
- any helper threshold math used by it

perform the safe CPU reference cleanup described in the plans:

- use direct indexing after up-front length validation or debug assertions
- hoist config flags and thresholds outside the loop
- avoid repeated `.get(...).copied().unwrap_or(...)` style overhead in the hot loop
- compare `abs(stddev)` against `abs(close) * threshold` instead of doing repeated divisions, while preserving semantics and finite checks
- preserve behavior for:
  - `trade_on_next_bar`
  - `long_only`
  - `allow_flip`
  - entry/exit logic

### D. Keep CPU/reference output and GPU-prep output separate

The v2 plan explicitly calls out that MKCS should stop paying unnecessary `f32 -> f64` expansion when the next consumer is a GPU path.

Implement a cleaner split such as:

- a CPU/reference path that returns `Vec<f64>`
- a CUDA-prep path that returns `Vec<f32>` or device-native output where appropriate

Apply this in the MKCS helpers so that GPU preparation does not go through an unnecessary `Vec<f64>` expansion just to feed a GPU scoring path later.

### E. Improve Donchian handling

Implement the best safe improvement supported by the current repo APIs:

Preferred outcome:
- a **combined Donchian path** that retrieves upper/lower together for MKCS prep and/or CPU fallback

If the current dynamic indicator adapter already supports or can trivially support multi-output retrieval, land it.

If the adapter cannot yet do that cleanly:
- keep the two-output path correct
- still apply ID caching and param reuse
- avoid unnecessary repeated allocations
- keep the code structured so a combined output path can be dropped in cleanly later

### F. Implement the long-term sweep fix if the current repo support makes it feasible

Preferred outcome:
- a true **device-native sweep path** that avoids GPU -> host -> GPU for MKCS period sweeps

This means that, where the current runtime/adapter stack allows it, you should:

- request device output (e.g. `CudaOutputTarget::DeviceF32` or equivalent)
- assemble or copy rows directly into a contiguous device matrix
- return a `MkcsPeriodSweepDevice` without materializing the whole sweep on host first

If that requires a small adjacent change in `dynamic_indicator_adapter.rs` or a closely related helper, make the smallest viable change.

If the repo truly cannot support the full device-native sweep without an invasive redesign, do **not** block the rest of the implementation. In that case:

- keep the current path correct
- still land all allocation/caching improvements
- keep the helper structure ready for a later device-native upgrade
- document the remaining adjacent API limitation in your final summary

### G. Preserve fallback correctness and improve transparency where possible

In or around:

- `compute_indicator_f64(...)`
- MKCS CUDA-prep helpers

preserve the current fallback behavior, but do not silently hide route cliffs if there is an obvious local way to surface or log a fallback reason without widening the architecture.

Do **not** break callers that rely on CPU fallback.

### H. Do **not** use uninitialized memory for semantic output vectors

Specifically:

- do **not** uninitialize the `desired` output vector in `build_mkcs_desired_positions(...)`
- do **not** use uninitialized memory for any partially written semantic series

That vector has meaningful zero state and should remain explicitly initialized.

---

## 2) `crates/app_core/src/mkcs_gpu_kernel.rs`

This file is the main launcher/runtime optimization target.

### A. Add MKCS-local reusable scratch to the runtime

Extend `MkcsCudaRuntime` (or its equivalent current runtime holder) with reusable scratch state for at least:

- device close buffer
- device log-return buffer
- device combo buffers:
  - stddev row indices
  - breakout row indices
  - `start_t`
  - `end_t`
- device output buffer
- reusable host readback scratch

This runtime scratch must be reused across repeated MKCS eval calls on the same device.

### B. Fix cache identity correctness: do not reuse market data by pointer+length alone

This is important.

Do **not** decide close-buffer reuse only from host pointer + length.
That is not a safe identity and can produce stale GPU data if a caller mutates the same allocation.

Implement the stronger identity design from v2:

- add a market-series abstraction such as `MkcsGpuMarketSeries`
- include:
  - series length
  - a caller-supplied stable `market_series_key`
  - optional host `close_f32` slice
  - optional device close pointer
  - optional device log-return pointer
  - optional device identity
- extend `MkcsGpuDeviceSignalMatrices` (or the equivalent signal carrier) to include this market-series data and signal-matrix device ownership

The runtime should:

- prefer device-resident close when supplied
- prefer device-resident log returns when supplied
- only upload host close when no device close is supplied
- only recompute log returns when no valid reusable device `lr` is available for the same `market_series_key`

### C. Strengthen host-side input validation before launch

Augment validation in or around `validate_inputs(...)` so it covers all of the following:

- non-empty series
- `cols` / close length consistency
- non-null row-major pointers
- non-zero row counts and strides
- `stddev_stride == cols`
- `breakout_stride == cols`
- combo vector length consistency
- combo row indices are in range
- combo time ranges are valid (`start_t <= end_t < len`)
- device ownership is consistent:
  - signal matrix device matches eval device
  - market device matches signal device when device pointers are supplied
- any host-to-kernel integer conversions use `i32::try_from(...)` or equivalent checked conversion

Return explicit, useful errors. Do not leave these as silent UB or unchecked casts.

### D. Rework the log-return launch helper so scratch-owned buffers are easy to use

The plans call out that once buffers live inside runtime scratch, a raw-pointer launch helper or an equivalent borrow-friendly API is usually cleaner.

Update the log-return launch path so the runtime can:

- ensure `d_close`
- ensure `d_lr`
- launch the log-return kernel
- then launch the MKCS metrics kernel

without borrow conflicts or unnecessary reallocation.

### E. Add explicit uploaded-combo support with device affinity

Implement a separate uploaded-combo type such as `MkcsUploadedComboBatch` with explicit device ownership.

Requirements:

- upload-once constructor from host combos on a specific device
- store device ID in the uploaded batch
- reject cross-device use explicitly
- keep the uploaded-combo eval path separate from the one-shot host-combo path
- do **not** create/upload a fresh uploaded-combo batch inside the one-shot path each call

The intention is:

- one-shot path = convenient scratch-upload path
- uploaded path = host combo upload paid once, then reused repeatedly

### F. Replace zero-filled host readback with safe reusable or uninitialized POD scratch

The plans explicitly call out the current zero-fill before full overwrite as unnecessary.

Implement the safest local improvement supported by the repo:

Preferred order:
1. use an existing shared `alloc_policy` or equivalent helper for fully overwritten POD vectors if the repo already has one
2. otherwise add a narrowly scoped local helper or reusable `Vec<f32>` scratch that is documented safe because the D2H copy fully overwrites every element before any read

Rules:

- use this only for POD/fully overwritten buffers
- do not generalize unsafe behavior more widely than needed
- add comments explaining why the unsafe usage is valid

### G. Cache device launch metadata where it is safe and simple

The v2 plan recommends caching lighter-weight launch metadata rather than forcing a tricky self-referential design.

Implement safe caching such as:

- device launch caps
- chosen block sizes for common kernels

if doing so is straightforward in the current runtime.

Do **not** create a brittle self-referential `Module`/`Function` storage design just to cache function handles.

### H. Preserve unsupported-feature rejection behavior

The current MKCS GPU path rejects execution overlays, stop-loss, and take-profit.
Preserve that behavior.

Improve error clarity if you can do so locally, but do not broaden support in this task.

### I. Optional but only after core path is stable

Only if it is low-risk and clearly worthwhile after the core path compiles/tests:

- add a score-only / objective-only readback path

This is a second-tier optional improvement. Do **not** let it destabilize the core implementation.

---

## 3) `crates/app_core/benches/gpu_native_expansion_opt.rs`

This file currently lacks MKCS optimizer-side coverage and must be expanded.

### A. Add MKCS synthetic candle generation

Add a synthetic candle generator suitable for MKCS squeeze/breakout behavior.
Use a deterministic construction patterned similarly to the existing per-strategy synthetic generators in the file.

### B. Add MKCS request construction

Add a `make_mkcs_req(...)` builder consistent with the existing request-builder style in the file.

### C. Add end-to-end MKCS CPU and GPU benches

Add MKCS optimizer benches to the criterion group, consistent with the existing file patterns.

At minimum add:

- MKCS CPU bench
- MKCS GPU bench

Use the existing bench-target filtering pattern so MKCS can be selectively enabled if the file already supports that.

### D. Add MKCS phase-split benches

Add phase benches that separate:

- stddev sweep prep
- Donchian upper sweep prep
- Donchian lower sweep prep
- score-only evaluation

If uploaded combos are implemented, also add a score-only benchmark for that path.

The purpose is to make it obvious whether the remaining cost is prep, score, or full-route orchestration.

### E. Separate cold-path and steady-state evidence

Do not only benchmark a single blended number.

For MKCS-specific prep and score benches:

- perform one warm-up call before measuring the steady-state path
- set an explicit `warm_up_time(Duration::from_secs(2))` if the file’s Criterion style allows it
- keep the repeated measured body focused on steady-state cost

### F. Add useful throughput-derived reporting if feasible in the current harness

Where practical, report or print at least one or more of:

- bars/sec
- candidates/sec
- `(bars * candidates)/sec`

Do not contort the harness into a redesign to do this. Keep it simple and local.

---

## 4) `crates/app_core/benches/composite_validation_workflow.rs`

This file needs stronger MKCS validation coverage.

### A. Improve `make_mkcs_candidates(...)`

The v2 plan explicitly calls out that the current MKCS validation candidates vary too little.

Expand `make_mkcs_candidates(...)` so it does **not** only vary MA lengths.

It should also vary MKCS-local settings such as:

- `compression_threshold`
- `reexpansion_threshold`
- `map_fast_to_stddev_period`
- `map_slow_to_breakout_lookback`
- direct `stddev_period`
- direct `breakout_lookback`

Use deterministic candidate construction. Keep the values realistic and bounded.

### B. Add larger generic MKCS profiles

Add MKCS to the larger generic profile coverage so it can be compared against the other strategies that already have larger benchmark shapes.

At minimum add generic MKCS 100k coverage where that fits the existing benchmark structure.

### C. Add dedicated heavier MKCS GPU validation groups

Mirror the existing dedicated strategy-specific GPU validation style (e.g. the ASR heavier groups) with MKCS-specific groups.

At minimum add something equivalent to:

- `100k / 64 candidates` with workflow-style validation (e.g. WF + CSCV if that is how the existing file structures it)
- a heavier in-sample candidate-count case (e.g. `192 candidates`)

Follow the local style and naming conventions already used in the file.

### D. Keep route/fallback visibility

Preserve or add route-probe output so that the benchmark still tells the user:

- backend used
- backend route
- fallback reason

This is part of the review-map scope and should not be lost.

### E. Keep refactors small

If you can reduce duplication between the dedicated ASR and MKCS validation groups with a small helper, that is fine.

Do **not** turn this into a benchmark framework rewrite.

---

# Required test work

Add or extend tests in `mkcs.rs` and/or `mkcs_gpu_kernel.rs` (and only minimal adjacent files if absolutely necessary) so the new implementation is covered.

## Required test categories

### 1. CPU vs GPU parity test on deterministic candles

Add a deterministic parity test that compares MKCS CPU vs GPU outputs/metrics on a small, stable input set.

### 2. Threshold-edge parity tests

Because MKCS is threshold-sensitive and the GPU path is fp32-oriented, add focused tests around threshold boundaries.

Cover at least:

- compression threshold ± epsilon
- reexpansion threshold ± epsilon

Use both:

- absolute tolerance
- relative tolerance

Do not rely on fixed absolute tolerance alone.

### 3. Strategy flag behavior where feasible

Add or extend parity/regression coverage for combinations involving:

- `long_only`
- `allow_flip`
- `trade_on_next_bar`

Preserve semantics.

### 4. Stale close-cache regression test

Add a regression test that guards against the incorrect pointer+length reuse idea.

The intent:

- the same allocation can be reused with different values
- the runtime must not treat that as identical market data unless the caller’s `market_series_key` also remains intentionally unchanged
- the caching contract must be explicit and safe

### 5. Uploaded-combo wrong-device rejection test

Add a test that verifies an uploaded combo batch created on one device is rejected on another device.

### 6. Invalid combo / time-range validation tests

Add tests for:

- out-of-range stddev row index
- out-of-range breakout row index
- invalid `start_t` / `end_t`
- stride/shape mismatch if feasible to express locally

### 7. Unsupported-feature rejection remains explicit

Add or preserve coverage that MKCS GPU kernel rejects unsupported overlays / SL / TP combinations with a clear error.

---

# Memory, allocation, and unsafe-use rules

This task explicitly cares about copy-elision and safe use of uninitialized memory.

## Allowed patterns

These are acceptable **only when fully overwritten before read**:

- device buffers allocated with uninitialized constructors for kernel outputs or copy destinations
- reusable host POD scratch buffers whose length is set and then fully overwritten by a D2H copy before any read

## Strong preferences

- if the repo already has an `alloc_policy` or similar helper for fully overwritten vectors/buffers, prefer that instead of inventing a new unsafe helper
- prefer buffer reuse over repeated alloc/free
- keep unsafe usage tiny, local, and documented
- add comments that state why the memory is fully overwritten before read

## Forbidden / do not do this

- do not uninitialize semantic output vectors like MKCS desired positions
- do not use uninitialized memory for partially written buffers
- do not use uninitialized memory for non-POD types or types with drop glue
- do not spread unsafe helpers broadly unless the repo already has a standard pattern for this exact use case

---

# Optional second-tier work (only after the core implementation is complete and passing)

These items are in the plans but are explicitly lower-priority and should not block the main implementation.

Only implement them if they are low-risk, compile-safe, and clearly supported by the current repo after the core work is done:

1. MKCS objective-only / score-only readback path
2. MKCS-local surface cache for reused sweeps
3. pinned host buffers for remaining unavoidable transfers
4. multi-stream overlap for independent prep surfaces
5. CUDA Graph replay for repeated same-shape scoring
6. dedicated MKCS prep kernel
7. a measured CPU/GPU route threshold

Important constraints for these optional items:

- do **not** implement a CPU/GPU route threshold blindly; only do so if the new benchmark evidence clearly supports it and there is an obvious narrow integration point
- do **not** add these optional features if they would broaden the scope or destabilize correctness
- if you do not implement them, mention them in the final summary as optional follow-ups only

---

# Acceptance criteria

Your implementation is complete only when all of the following are true:

1. The code compiles.
2. The main MKCS files are updated in line with the v2 plan.
3. MKCS staging/allocation waste is materially reduced.
4. One-shot and uploaded-combo GPU paths are clearly separated.
5. Cache identity is safe (`market_series_key` or equivalent), not pointer+length only.
6. MKCS input/device validation is stronger and explicit.
7. MKCS optimizer benches exist and compile.
8. MKCS heavier validation benches exist and compile.
9. MKCS tests cover parity, thresholds, device mismatch, and cache correctness.
10. No existing feature or behavior is removed.
11. Unsupported overlays / SL / TP rejection remains intact.
12. Unsafe/uninitialized memory usage, if any, is narrowly scoped and documented.

---

# Suggested execution order

Implement in this order unless the actual repo structure forces a small reordering:

1. inspect the docs and current MKCS code
2. land correctness-safe runtime/scratch changes in `mkcs_gpu_kernel.rs`
3. land `mkcs.rs` caching / CPU hot-loop cleanup / output-path cleanup
4. land the uploaded-combo path
5. land benchmark coverage in `gpu_native_expansion_opt.rs`
6. land heavier MKCS validation coverage in `composite_validation_workflow.rs`
7. land or finish the device-native sweep / combined Donchian support if the adjacent API support is feasible
8. add/extend tests
9. run formatting, tests, and relevant benches

---

# Validation / commands

Run the relevant local validation commands. Adapt exact package names and feature flags to the repo if needed.

At minimum, do the equivalent of:

- `cargo fmt --all`
- targeted MKCS-related tests
- targeted tests for any adjacent support file you touched
- MKCS optimizer benches in `gpu_native_expansion_opt.rs`
- MKCS validation benches in `composite_validation_workflow.rs`

If the workspace uses feature flags for CUDA, include the existing CUDA-related feature set already used by these files and benches (for example `cuda` and `cuda-backtest-kernel`, if that matches the workspace).

Do not silently skip failing commands. Report what ran and what did not.

---

# Final report format

When you finish, provide a concise but complete implementation report with these sections:

1. **Files changed**
2. **What changed by file**
3. **Any adjacent support files touched and why**
4. **Unsafe / uninitialized memory uses and why they are safe**
5. **Tests added or updated**
6. **Benchmarks added or updated**
7. **Commands run and results**
8. **Anything not implemented and why**
   - only include this if something was genuinely blocked by missing adjacent infrastructure or intentionally left as optional second-tier work

Do not end with a vague summary. Be specific.

