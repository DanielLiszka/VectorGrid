# Codex implementation prompt: ROCB GPU optimization, validation, and correctness work

Use this prompt with an OpenAI Codex-style coding agent to implement the ROCB strategy-local GPU optimization and validation plan end to end.

---

You are working in a Rust/CUDA repository. Your job is to **fully implement the ROCB optimization plan** described in the repo documents below, while preserving functionality and correctness.

## Documents you must read before editing code

Read these in this order and treat them with this precedence:

1. `ROCB_GPU_optimization_plan_v2.md` — **primary merged source of truth**
2. `ROCB_GPU_optimization_plan_addendum.md` — use to catch any nuance or detail not fully reflected in v2
3. `ROCB_GPU_optimization_plan.md` — original plan; use only to verify no detail was lost
4. `gpt54_feature_review_map.md` — especially:
   - `## 5. Strategy-local GPU-native optimization and validation routes`
   - the repo-wide optimization-first / parity-preserving scope guard
   - the benchmark-first posture
   - the allocation / copy-elision guidance where relevant

If there is any conflict, prefer:

`ROCB_GPU_optimization_plan_v2.md` > `ROCB_GPU_optimization_plan_addendum.md` > `ROCB_GPU_optimization_plan.md` > `gpt54_feature_review_map.md`

The implementation is **strictly limited to the ROCB strategy surface** unless a small shared helper or a nearby call-site must be updated to realize the ROCB plan safely.

## Primary files that must be inspected and likely modified

Locate the repo versions of these files and update them:

- `rocb.rs`
- `rocb_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

If the repo uses canonical paths, they are likely equivalent to something like:

- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

## Additional files you are allowed to inspect and modify if needed

Do not ignore these if they are needed to wire the new ROCB hot path correctly:

- any ROCB optimizer or validation call-site that invokes the ROCB GPU kernel path
- any shared allocation helper such as `alloc_policy.rs` or equivalent host-readback helper module
- `gpu_backtest_kernel.rs` or closely adjacent shared runtime/residency code if needed for alignment or reuse of an existing approved pattern
- nearby ROCB tests or the crate’s test modules
- any CUDA feature-gated helper module required for the ROCB path to compile cleanly

Only touch additional files when the edit is directly required to implement or safely wire the ROCB plan.

---

# Objective

Implement the ROCB plan so that ROCB becomes more GPU-resident and avoids unnecessary staging overhead, while remaining parity-safe and benchmarkable.

The priorities are:

1. **Performance**
   - reduce repeated host-to-device copies
   - reduce repeated device allocations
   - reduce repeated host allocations / zero-fill where safe
   - reuse resident GPU data across repeated scoring calls
   - improve warm-path ROCB performance, not just cold-path microbench behavior

2. **Correctness**
   - preserve ROCB behavior and public functionality
   - preserve current unsupported-route behavior where explicitly blocked
   - improve validation and error reporting for malformed GPU inputs
   - maintain CPU/GPU parity within appropriate tolerance

3. **Safety**
   - prefer repo-approved allocation helpers and reuse patterns
   - use uninitialized memory only where sound and already sanctioned by repo patterns
   - do **not** introduce unsound host-side `Vec::set_len(...)` + slice materialization patterns
   - document invariants around any `unsafe` code you retain or add

4. **Benchmark fidelity**
   - add ROCB-specific optimizer and validation benches
   - separate warm-path reuse from cold-ish setup where practical
   - ensure the intended GPU route is actually exercised and not silently benchmarking a fallback path

---

# Non-goals / constraints

These are hard constraints.

- Do **not** add new ROCB features.
- Do **not** add new user-facing optimization knobs.
- Do **not** remove features.
- Do **not** change ROCB trading semantics unless required to fix a correctness bug and clearly justified.
- Do **not** do a broad architecture rewrite.
- Do **not** generalize this into a multi-strategy redesign unless a tiny shared helper adoption is clearly the safest way to implement the ROCB plan.
- Keep the current GPU-kernel blocker behavior for unsupported overlays / stop-loss / take-profit unless you are explicitly implementing full support, which is **not** in scope here.
- Keep backward compatibility where practical by preserving existing public entrypoints as wrappers while migrating hot-path callers to new uploaded/reuse APIs.

---

# What “done” means

The work is not complete until all of the following are true:

## Functional

- ROCB GPU metric outputs match the CPU reference path within an agreed tolerance on representative test cases.
- ROCB prep diagnostics can distinguish CPU, CUDA host-readback, and fallback-to-CPU cases.
- Existing ROCB behavior remains intact.
- Existing wrapper-style entrypoints continue to work.

## Performance

- repeated ROCB scoring on the same candle set does **not** re-upload `close_f32` every call
- repeated ROCB scoring on the same candle set does **not** recompute log returns every call
- repeated ROCB scoring on the same candidate batch does **not** re-upload combo vectors every call
- repeated ROCB scoring does **not** allocate a fresh device output buffer every call
- repeated ROCB scoring does **not** allocate a fresh zero-filled host metrics vector every call

## Benchmarking

- ROCB has optimizer CPU and GPU benches in `gpu_native_expansion_opt.rs`
- ROCB has heavier validation coverage in `composite_validation_workflow.rs`
- ROCB has at least one dedicated heavier GPU validation workflow case

## Safety

- no unsound host-side uninitialized readback pattern was introduced
- all GPU launch integer conversions are checked
- uploaded/reused buffers validate device id and size consistency
- malformed combo/window inputs fail early on the host side with clear messages

---

# Required implementation tasks

Implement the following, adapting naming and exact signatures to the repo’s actual code where needed. The code snippets in the plan docs are guidance, not guaranteed copy-paste solutions. Preserve repo style.

## 1) `rocb_gpu_kernel.rs`

This file is the highest-priority hot path. Implement the runtime reuse and staging reductions first.

### 1.1 Add or refine a runtime access helper

Add a helper similar to `with_runtime(device_id, |rt| { ... })` so ROCB code shares a clean, single runtime lookup / initialization path.

Requirements:

- lazily create the per-device runtime
- set the CUDA context current before use
- return descriptive errors
- make the larger refactor simpler and less repetitive

### 1.2 Add reusable runtime scratch

Add runtime-owned scratch that reuses output buffers across launches.

Expected shape:

- reusable device output buffer (`d_out`) with grow-only capacity behavior
- reusable host metrics buffer with grow-only behavior
- helper methods to:
  - ensure capacity
  - return raw device pointer for output
  - copy output back into the reusable host slice

Requirements:

- do **not** allocate `d_out` every call once capacity is sufficient
- do **not** allocate or zero-fill a new host metrics vector every call once capacity is sufficient
- if the repo provides a shared helper like `alloc_uninit_copy(...)` or an approved `alloc_policy` utility for host readback, prefer that helper over custom ROCB-local unsafe code
- if such a helper is unavailable or not appropriate, use safe grow-only host-buffer reuse

### 1.3 Add reusable uploaded market state

Add a device-resident ROCB market upload object, e.g. `RocbGpuUploadedMarket`, that owns:

- `d_close`
- `d_lr`
- device id
- series length / column count

Implement a helper like `upload_rocb_market_to_device(device_id, close_f32)` that:

- uploads close once
- computes log returns once
- synchronizes appropriately
- returns an object that can be reused across repeated scoring launches

Requirements:

- validate non-empty input
- validate consistent device usage
- use device-side uninitialized allocation for `d_lr` if that is already the established repo pattern

### 1.4 Add reusable uploaded combo state

Add a device-resident combo upload object, e.g. `RocbGpuUploadedCombos`, that owns:

- device id
- combo count
- device buffers for:
  - `channel_row_index`
  - `atr_row_index`
  - `start_t`
  - `end_t`

Implement a helper like `upload_rocb_combo_batch_to_device(device_id, combos)`.

Requirements:

- validate vector lengths match before upload
- make the object reusable across repeated validation / finalist scoring loops

### 1.5 Add a new uploaded-input hot-path entrypoint

Add a new entrypoint similar to:

- `eval_rocb_metrics_gpu_kernel_uploaded(...)`

It should accept the uploaded market object, uploaded combo object, and device-resident signal matrices.

Requirements:

- validate uploaded objects match the requested device id
- validate uploaded market length matches signal matrix columns
- preserve the current explicit blocker for unsupported overlays / stop-loss / take-profit
- reuse runtime scratch for output
- decode metrics from the reusable host metrics buffer
- keep the existing non-uploaded entrypoint as a compatibility wrapper during migration

### 1.6 Strengthen prelaunch validation

Implement stronger host-side validation before upload or kernel launch.

At minimum validate:

- non-empty series
- `close_f32.len() == cols`
- non-null device pointers for signal matrices
- non-zero row/stride shapes
- `channel_stride >= cols`
- `atr_stride >= cols`
- combo vector lengths all match
- each `channel_row_index` is within bounds
- each `atr_row_index` is within bounds
- `start_t` / `end_t` are non-negative
- `start_t` / `end_t` are within bounds of the series length
- `start_t` / `end_t` obey the kernel’s evaluation-window semantics

Important:

- **Verify whether `end_t` is inclusive or exclusive in the current kernel semantics before finalizing the checks and tests.**
- The earlier plan examples assumed exclusive `end_t`; if the actual kernel uses inclusive semantics, adapt the validation and tests accordingly.

### 1.7 Replace lossy `as i32` launch-time casts with checked conversions

Convert all kernel-launch integers using `i32::try_from(...)` (or a small checked helper) before building kernel argument arrays.

This includes values like:

- series length
- row counts
- strides
- combo count
- metrics stride

Preserve current behavior for `max_trade_bars` if it already fits naturally, but keep it explicit and safe.

### 1.8 Cache function handles and launch metadata if cleanly possible

This is optional-but-recommended after the bigger wins land.

If the runtime structure can safely cache them, cache:

- the log-returns kernel function handle
- the ROCB metrics kernel function handle
- device launch caps
- per-kernel launch caps

Use the repo’s existing patterns for this if available.

Do this only if it integrates cleanly with the existing runtime ownership / lifetime model.

### 1.9 Prefer repo-approved host-readback copy-elision helpers

If the repo already has an approved helper in an allocation policy or shared GPU module for host readback into uninitialized or partially initialized memory, use that instead of writing a ROCB-local bespoke unsafe path.

Hard safety rule:

- **Do not** introduce `Vec::with_capacity(len)` + `set_len(len)` + `copy_to(v.as_mut_slice())` or anything equivalent unless the repo already wraps that pattern in a reviewed safe abstraction.
- If no approved helper exists, use safe host-buffer reuse.

### 1.10 Keep a compatibility wrapper

Retain the existing convenience / compatibility entrypoint, but make it a wrapper around the uploaded path so existing callers do not break while the true hot-path call-sites are migrated.

---

## 2) `rocb.rs`

This file needs better route diagnostics, a more explicit GPU-prep path, and some CPU-side allocation cleanup.

### 2.1 Add prep diagnostics types

Add small ROCB-local route and diagnostics types, similar to:

- `RocbPrepRoute`
- `RocbPrepDiagnostics`

These should capture whether each ROCB prep leg used:

- CPU
- CUDA host readback
- CUDA fallback to CPU
- optionally a distinct device-sweep route if the type is used by the caller

Diagnostics should be able to store fallback reasons.

### 2.2 Add a traced scalar helper next to the current helper

Add a traced variant of `compute_indicator_f64(...)`, e.g. `compute_indicator_f64_traced(...)`, that returns:

- output values
- route used
- optional fallback reason

Requirements:

- preserve the current scalar helper as a thin wrapper if that is the least disruptive approach
- do not remove backward-compatible behavior
- record missing prepared CUDA inputs explicitly as a fallback reason when the backend prefers GPU but preparation is unavailable
- record CUDA execution errors explicitly before falling back to CPU

### 2.3 Add traced breakout and ATR helpers

Add traced variants of:

- `compute_rocb_breakout_signals(...)`
- `compute_rocb_risk_signals(...)`

These should return both the signal structs and diagnostics, while preserving the existing non-traced public API as wrappers if that best matches repo conventions.

### 2.4 Add a bundled ROCB GPU sweep prep helper

Add a ROCB-native prep helper that prepares all sweep matrices together, similar to:

- `RocbGpuPreparedSweeps`
- `prepare_rocb_gpu_sweeps(...)`

This helper should bundle:

- Donchian upper sweep
- Donchian lower sweep
- ATR sweep

Requirements:

- validate / assert that column counts and device ids align
- make it obvious that this is the intended GPU-native ROCB prep route for optimization and validation
- keep the device-sweep path explicit so higher-level ROCB call-sites do not accidentally keep using the scalar host-readback path

### 2.5 Add an in-place desired-position builder

Add a reusable-buffer variant of the CPU reference builder, e.g. `build_rocb_desired_positions_into(...)`.

Important correction from the earlier draft:

- do **not** just `resize(n, 0)` and overwrite later; that still pays full-tail zero-fill
- instead use a prefix-only zeroing strategy and then append the computed tail

Preferred safe shape:

- `out.clear()`
- reserve only if needed
- zero only the required prefix up to `start_t.min(n)`
- then `push(...)` per-bar for the computed tail

This is specifically to reduce unnecessary overwrite-only initialization while keeping safe Rust semantics.

### 2.6 Keep the old allocating builder as a wrapper

Preserve the current `build_rocb_desired_positions(...)` API by making it call the in-place builder internally.

### 2.7 Add the period-based `first_valid` fast path where valid

Where the code already knows the expected warmup boundary from the indicator period, prefer a direct `period - 1` style fast path rather than scanning large vectors for `first_finite_idx_local(...)`, as long as that is consistent with the actual indicator semantics in this repo.

Use caution:

- only apply this where the semantics are truly equivalent
- validate with tests if there is any ambiguity

### 2.8 Avoid small repeated heap work where it is easy

Reduce easy CPU-side overhead where it does not complicate the code:

- avoid repeated indicator-name / output-id resolution work where practical
- avoid heap-allocating one-element `Vec<IndicatorParamKV>` if the call sites can accept stack arrays / slices cleanly
- do not over-engineer this; keep it simple and idiomatic

---

## 3) `gpu_native_expansion_opt.rs`

This benchmark file currently lacks ROCB coverage and must be expanded.

### 3.1 Add ROCB imports and request builders

Add ROCB-specific imports and a ROCB request builder.

### 3.2 Add a ROCB-friendly synthetic candle generator

Do **not** rely only on a flat or overly smooth generic series. The ROCB benchmark data should include:

- quiet regime
- directional breakout regime
- reversal / choppy regime
- enough volatility variation to exercise ATR behavior
- enough duration to exercise `max_trade_bars`

Use the benchmark generators from the plan docs as a starting point, but adapt to repo conventions.

### 3.3 Add ROCB optimizer benches

Add at minimum:

- ROCB CPU optimizer bench
- ROCB GPU-kernel optimizer bench

Ensure ROCB is included in the relevant `criterion_group!`.

### 3.4 Add warm-path ROCB GPU benches if feasible

If practical within the repo’s benchmark structure, split ROCB GPU benches into:

- cold-ish / setup-included
- warm / uploaded-market-and-combos-reused

The warm case should upload once outside `b.iter(...)` and repeatedly launch scoring against the same resident data so the reuse work is actually measurable.

If this is too invasive for the current harness, add the baseline ROCB CPU/GPU benches first and then a narrowly scoped warm-path ROCB bench.

### 3.5 Verify the benchmark is measuring the intended route

For ROCB GPU benches, ensure the route probe is not silently measuring a fallback path.

Prefer one of these, in order of stability:

1. assert `BackendUsed::Gpu` and assert fallback reason is absent on intended ROCB GPU cases
2. if the exact route string is stable in this repo, assert the expected ROCB GPU route string too
3. continue printing route metadata for debugging, but do not rely only on printing if assertions can be made safely

---

## 4) `composite_validation_workflow.rs`

ROCB validation coverage is currently too light relative to the plan.

### 4.1 Extend ROCB CPU validation coverage

Add a heavier ROCB CPU validation profile in addition to the existing lighter profile.

Target shapes from the plan docs are acceptable starting points, such as:

- `rocb`, `50_000`, `8` candidates (already present or equivalent)
- `rocb`, `100_000`, `32` candidates

### 4.2 Extend ROCB GPU validation coverage

Add a heavier ROCB GPU validation profile in addition to the lighter generic profile.

Acceptable starting points:

- `rocb`, `100_000`, `32` candidates

### 4.3 Add a dedicated heavier ROCB GPU validation workflow group

Add a dedicated ROCB-specific heavy GPU validation benchmark analogous to the existing heavier strategy-specific pattern in the repo.

Suggested shapes from the plan docs:

- `100_000` candles / `64` candidates with walk-forward + CSCV
- a heavier in-sample case around `192` candidates
- optionally an even larger case if the harness supports it and it remains practical

### 4.4 Assert the route where feasible

Do not only print validation route metadata.

Prefer to assert that the intended ROCB GPU validation route actually used GPU and did not unexpectedly fall back, while keeping route printing for debug context.

Use stable assertions: `backend_used == BackendUsed::Gpu` and no fallback reason, unless the repo already has a stable exact route string convention you can safely assert.

---

## 5) Outer ROCB hot-path wiring outside the four main files

The uploaded/reuse APIs only pay off if the real ROCB scoring loop uses them.

Search for the actual ROCB optimization / validation caller(s), especially callers of the current ROCB GPU scoring entrypoint.

Your goal is to migrate the real repeated loop(s) so they follow this pattern:

1. prepare CUDA indicator inputs once per candle set
2. prepare ROCB GPU sweeps once per candle set / period-range configuration
3. upload ROCB market once per candle set
4. build combo row-index arrays once per candidate batch
5. upload combos once per candidate batch
6. repeatedly launch the uploaded ROCB scoring path using reused market, sweeps, combos, and output scratch

Requirements:

- search for actual call-sites instead of assuming only the wrapper entrypoint is used
- integrate the uploaded path where the candle set and candidate batch lifetimes naturally exist
- if multiple callers exist, prioritize the hottest repeated loop(s)
- keep the legacy wrapper entrypoint for compatibility and easier incremental migration

If the true hot-path call-site cannot be safely migrated without a broader refactor, then:

- still implement the uploaded path and wrapper
- wire as much as safely possible in the nearest ROCB-specific loop
- document the remaining integration opportunity in your final summary

---

# Required tests

Add or update tests in the repo’s normal style. Use feature gating where necessary.

## 1. ROCB GPU-vs-CPU parity tests

Add at least these parity cases if the repo’s test environment supports them:

- single-combo parity
- multi-combo parity
- `trade_on_next_bar = true` parity
- `long_only = true` parity

Use representative candles and compare `Metrics` fields within an appropriate tolerance.

## 2. ROCB prep diagnostics tests

Add tests that verify:

- CPU-only backend reports CPU route
- missing prepared CUDA inputs records a fallback reason when GPU would otherwise be preferred
- successful CUDA host-readback path reports the expected route when the relevant feature path is active

## 3. Desired-position builder equivalence tests

Add a test ensuring the in-place builder matches the allocating builder exactly.

Also add a repeated-call test to ensure buffer reuse does not leave stale values behind.

## 4. GPU blocker tests

Add tests that verify the ROCB GPU path still rejects unsupported overlays / stop-loss / take-profit configurations early and clearly.

## 5. Prelaunch validation tests

Add tests for malformed GPU input batches:

- invalid row indices
- negative `start_t` / `end_t`
- out-of-bounds windows
- invalid ordering based on the kernel’s actual window semantics
- oversize integer conversion failures where practical

---

# Required safety rules around uninitialized memory and copies

This part is critical. Follow it exactly.

## Allowed / preferred

- device-side uninitialized buffers where they are fully overwritten before read and already part of repo practice
- grow-only reusable host buffers
- repo-approved helpers for host readback copy-elision / uninitialized allocation, if such helpers already exist
- prefix-only initialization when the prefix must be zeroed and the tail is fully written later

## Not allowed

- unsound host-side `set_len` tricks
- custom ROCB-local unsafe host `MaybeUninit` handling if the repo already has a reviewed shared helper and you can use that instead
- introducing unsafe code without documenting why it is sound and how the buffer is fully written before any read

If you find an existing shared helper in the repo, prefer aligning ROCB with that helper instead of inventing a new strategy-local pattern.

---

# Optional but worthwhile extensions

These are **not** higher priority than the main file-by-file tasks, but they are worth doing if they fit cleanly and do not destabilize the change.

## 1. Align ROCB with the shared resident-market pattern

Inspect `gpu_backtest_kernel.rs` and related shared runtime code. If there is already an approved resident-market upload pattern that can be reused or mirrored cleanly for ROCB, align with it.

Only do this if it is a small, direct improvement and not a broad redesign.

## 2. Cache function handles and launch metadata in the ROCB runtime

Do this if it is straightforward and lifetime-safe.

## 3. Improve benchmark structure to distinguish cold and warm ROCB cases

Do this after the baseline ROCB benchmark coverage exists.

## 4. Tighten small CPU-side overhead in `rocb.rs`

Examples:

- cache indicator-name resolution where low-risk
- avoid one-element heap vectors where stack arrays / slices are acceptable
- avoid full-tail zero-fill in reusable builders

---

# Suggested implementation order

Follow this order unless the repo structure forces a better one.

## Phase 1: benchmark coverage and observability

1. add ROCB optimizer benches in `gpu_native_expansion_opt.rs`
2. add heavier ROCB validation benches in `composite_validation_workflow.rs`
3. add ROCB prep diagnostics in `rocb.rs`

## Phase 2: hot-path staging and reuse

1. add runtime helper
2. add reusable runtime scratch
3. add uploaded market object and upload helper
4. add uploaded combo object and upload helper
5. add uploaded ROCB evaluation entrypoint
6. strengthen prelaunch validation
7. replace unchecked integer casts
8. keep compatibility wrapper

## Phase 3: wire the real caller(s)

1. locate actual ROCB GPU optimization/validation call-sites
2. switch the hottest repeated loop(s) to the uploaded/reuse path
3. preserve compatibility wrappers for any lagging call-sites

## Phase 4: CPU-side cleanup and optional refinements

1. add in-place desired-position builder
2. keep allocating builder as wrapper
3. apply small CPU-side overhead improvements in `rocb.rs`
4. add optional warm-path ROCB optimizer benches
5. add runtime metadata/function caching if clean

---

# Validation and command-running expectations

Use the repo’s actual crate names, features, and paths. Do not assume commands blindly; adapt them after inspection.

At minimum:

1. run formatting on changed files
2. run the narrowest relevant tests for ROCB and the touched modules
3. run feature-gated ROCB GPU tests if the environment supports CUDA and the repo is configured for it
4. ensure the modified benches compile
5. if bench execution is practical, run at least a small ROCB benchmark sample or the route probe paths

Examples of the kind of commands you may need to adapt:

- `cargo fmt`
- targeted `cargo test` for the crate/module containing ROCB
- targeted `cargo test --features cuda,cuda-backtest-kernel ...` for ROCB GPU tests
- `cargo bench --bench gpu_native_expansion_opt --no-run`
- `cargo bench --bench composite_validation_workflow --no-run`

Prefer narrow commands over full-workspace brute force unless necessary.

If the environment cannot execute CUDA benches/tests, still:

- compile the feature-gated code if possible
- run CPU-side tests
- leave clear notes in your final summary about what could not be executed locally

---

# Implementation checklist

Use this as a completion checklist while you work.

## `rocb_gpu_kernel.rs`

- [ ] add/refine runtime helper
- [ ] add reusable scratch struct
- [ ] add reusable device output allocation logic
- [ ] add reusable host metrics buffer logic
- [ ] use repo-approved host-readback helper if available and appropriate
- [ ] add uploaded market struct
- [ ] add market upload helper
- [ ] compute and store log returns once per uploaded market
- [ ] add uploaded combos struct
- [ ] add combo upload helper
- [ ] add uploaded ROCB eval entrypoint
- [ ] keep current wrapper entrypoint for compatibility
- [ ] strengthen input and combo/window validation
- [ ] replace unchecked launch-time integer casts
- [ ] preserve unsupported overlay/SL/TP blocker behavior
- [ ] optionally cache kernel handles / launch caps if clean

## `rocb.rs`

- [ ] add prep route enum
- [ ] add prep diagnostics struct
- [ ] add traced scalar indicator helper
- [ ] keep existing scalar helper as wrapper if practical
- [ ] add traced breakout helper
- [ ] add traced ATR/risk helper
- [ ] add bundled ROCB GPU sweep prep helper and struct
- [ ] add in-place desired-position builder
- [ ] keep allocating builder as wrapper
- [ ] avoid full-tail zero-fill in reusable builder
- [ ] add period-based `first_valid` fast path where valid
- [ ] reduce easy repeated heap / resolution overhead where safe
- [ ] record fallback reasons explicitly

## `gpu_native_expansion_opt.rs`

- [ ] import ROCB params/types
- [ ] add ROCB-oriented synthetic candles
- [ ] add ROCB request builder
- [ ] add ROCB CPU optimizer bench
- [ ] add ROCB GPU optimizer bench
- [ ] add ROCB to criterion groups
- [ ] assert intended GPU route where safely possible
- [ ] optionally add warm-path ROCB GPU bench

## `composite_validation_workflow.rs`

- [ ] add heavier ROCB CPU validation case(s)
- [ ] add heavier ROCB GPU validation case(s)
- [ ] add dedicated heavier ROCB GPU workflow group
- [ ] add ROCB benchmark to criterion groups
- [ ] assert intended GPU route where safely possible

## Wiring / outer call-sites

- [ ] locate the actual ROCB GPU caller(s)
- [ ] migrate repeated ROCB scoring loop(s) to the uploaded/reuse path
- [ ] preserve wrappers for compatibility
- [ ] document any remaining caller not migrated if a broader refactor would be required

## Tests

- [ ] parity: single-combo GPU vs CPU
- [ ] parity: multi-combo GPU vs CPU
- [ ] parity: `trade_on_next_bar`
- [ ] parity: `long_only`
- [ ] prep diagnostics tests
- [ ] desired-position builder equivalence test
- [ ] desired-position builder repeated-call reuse test
- [ ] GPU blocker tests
- [ ] malformed GPU input validation tests

## Safety / review

- [ ] no unsound host-side uninitialized readback introduced
- [ ] all new unsafe code, if any, is documented and justified
- [ ] uploaded/reused buffers validate device and size consistency
- [ ] output buffers only grow and are safely reused
- [ ] fallback diagnostics remain correct
- [ ] GPU route is actually exercised in intended benches

---

# Final output you must provide after making changes

When you finish, provide a concise but complete implementation summary with:

1. **Files changed**
2. **What was implemented in each file**
3. **Which plan items were completed**
4. **Any deviations from the plan and why**
5. **Commands run**
6. **Test / benchmark results obtained locally**
7. **Anything that could not be executed locally**
8. **Any remaining low-risk optional follow-up work**

Do not stop at analysis only. Make the code changes.

