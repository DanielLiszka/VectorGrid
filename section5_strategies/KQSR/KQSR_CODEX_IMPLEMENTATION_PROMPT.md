# Codex implementation prompt: KQSR GPU-native optimization, copy-elision, and correctness hardening

Use the following as the prompt for an OpenAI Codex agent.

---

You are implementing a **real code change**, not writing another review.

Your mission is to **fully implement the KQSR GPU-native optimization plan** in this Rust repo, with a strong bias toward:
- performance
- minimizing unnecessary memory copies
- safe uninitialized-memory usage where write-before-read is guaranteed
- correctness and parity
- preserving all existing features and behavior
- avoiding broad architecture rewrites unless a very small additive change is clearly required

Do **not** remove features. Do **not** break functionality. Do **not** change strategy behavior unless the change is a correctness fix that preserves intended semantics.

## 0. Read these documents first

These documents define the implementation target. Read them before editing code.

Primary plan docs:
- `/mnt/data/KQSR_GPU_NATIVE_OPTIMIZATION_IMPLEMENTATION_PLAN_v2.md`
- `/mnt/data/KQSR_GPU_NATIVE_OPTIMIZATION_IMPLEMENTATION_PLAN.md`
- `/mnt/data/KQSR_GPU_NATIVE_OPTIMIZATION_PLAN_ADDENDUM_V2.md`

Prompt/rules doc:
- `/mnt/data/gpt54_feature_review_map.md`

If those absolute paths are not present in your environment, search by basename and open the matching files.

### Source-of-truth precedence

If there is any tension between the docs, use this precedence:
1. `KQSR_GPU_NATIVE_OPTIMIZATION_IMPLEMENTATION_PLAN_v2.md`
2. `KQSR_GPU_NATIVE_OPTIMIZATION_PLAN_ADDENDUM_V2.md`
3. `KQSR_GPU_NATIVE_OPTIMIZATION_IMPLEMENTATION_PLAN.md`
4. `gpt54_feature_review_map.md` sections 5 and 16 as guardrails/rules

### Review/prompt rules to honor from the map doc

Use the spirit of **bundle 5** and **bundle 16**:
- focus on strategy-local GPU-native optimization and validation routes
- focus on allocation policy, prefix initialization, host-readback copy-elision, and safe `MaybeUninit` adoption
- prefer smaller, parity-safe changes before larger refactors
- prefer shared helpers in `alloc_policy` before introducing new local unsafe allocation code
- only use uninitialized allocation when every element is guaranteed to be written before any read
- keep benchmark fidelity and route/fallback transparency intact or improved

## 1. Files to modify

Primary repo files to modify:
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

These are the core files. The work should stay centered here.

## 2. Adjacent files you may inspect and reuse from, but only modify if truly necessary

Inspect these first before inventing local helpers or duplicate unsafe patterns:
- `crates/app_core/src/alloc_policy.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- any adjacent strategy GPU kernel that already uses the relevant shared helper patterns

Only modify adjacent files if one of the following is true:
- a shared helper already exists but is not publicly reusable and exposing it is the safest path
- a tiny adjacent addition is needed to avoid duplicating unsafe allocation/copy logic
- a tiny adjacent improvement is needed for a clearly safe KQSR-local optimization already called out in the plan docs

Do **not** turn this into a broad cross-repo redesign.

## 3. Hard constraints and non-goals

### Must preserve
- existing public/user-visible functionality
- existing route behavior unless improved by clearer fallback/accounting
- existing wrapper entrypoints unless the docs explicitly call for additive warm-path APIs
- feature-gated builds (`cuda`, `cuda-backtest-kernel`, and non-CUDA builds)
- existing strategy semantics (`long_only`, `allow_flip`, `trade_on_next_bar`, etc.)

### Do not do
- do not remove features
- do not invent new strategies or new product/UI features
- do not require callers to migrate immediately to new APIs if a wrapper can delegate internally
- do not add unsafe uninitialized allocation unless write-before-read is proven
- do not assume pinned-memory/async APIs exist without checking the actual crate versions/APIs in the repo
- do not break no-CUDA compilation paths
- do not silently change benchmark meaning; if a benchmark changes scope, make that explicit in the benchmark name/output

## 4. High-level implementation target

Implement the plan in phases, but actually make the code changes now.

### Required implementation set
Treat the following as required unless the repo structure makes a specific item impossible without a broad unrelated refactor:
- phase 0, 1, 2, and 3 items from the v2 plan
- the checklist items that are not marked optional/later
- the addendum items that are low-risk and directly aligned with the main goal

### Optional-but-strongly-preferred implementation set
Implement these too if the repo already supports them cleanly and they do not require broad redesign:
- pinned host readback for full metrics readback
- async stream-aware upload/allocation/readback if supported by the current `cust`/CUDA stack in this repo
- KQSR route accounting / transfer telemetry in bench/debug paths
- output-ID caching and parameter-construction cleanup in `kqsr.rs`
- combo-buffer scratch reuse even on the convenience-wrapper path

### Optional-later items (do not let these block the main implementation)
Only do these if they fall out naturally and safely from existing code, or if the repo already has a pattern ready to copy:
- multi-output indicator collapse across adjacent dynamic-indicator layers
- fully device-resident end-to-end indicator outputs
- reduced readback surface (objective-only / top-k)
- padded row stride experiments
- combo reordering for row locality
- dedicated GPU executor thread / architectural runtime redesign

If optional-later work is not clearly safe and local, leave it out.

## 5. Detailed implementation instructions by file

---

## 5A. `crates/app_core/src/kqsr.rs`

Implement the following.

### 5A.1 Add direct CPU-to-`f32` fallback and prevent duplicate failed CUDA attempts

The docs explicitly call out that the current `compute_indicator_f32(...)` fallback is wasteful because it can:
1. try CUDA
2. fail
3. call `compute_indicator_f64(...)`
4. try CUDA again there
5. then do CPU compute
6. then convert `Vec<f64>` to `Vec<f32>`

Fix this.

#### Required changes
- add a `series_to_f32(...)` helper analogous to the existing `series_to_f64(...)`
- change `compute_indicator_f32(...)` so that, after a failed or unavailable CUDA path, it computes the indicator **directly on CPU** and converts the result **directly to `Vec<f32>`**
- do **not** call `compute_indicator_f64(...)` from `compute_indicator_f32(...)` on the fallback path
- preserve existing error behavior and output semantics
- keep this behind the existing CUDA feature gating conventions used by the file

### 5A.2 Reduce repeated output-ID and parameter churn

Implement the small but safe hot-path cleanup from the addendum.

#### Required or strongly preferred
- cache resolved output IDs for the KQSR outputs using `OnceLock<String>` or an equivalent safe static lazy mechanism
- centralize KQSR indicator-output resolution so repeated `resolve_output_id(...)` work is not done on every signal build
- centralize KQSR parameter construction so the same key/value `IndicatorParamKV` vectors are not rebuilt in a scattered way everywhere

Use the actual indicator/output set present in this file, including the KQSR signal families already reviewed in the docs, such as:
- `kdj` → `j`
- `srsi` → `k`, `d`
- `ift_rsi`
- `qqe` → fast/slow outputs as used here
- `prb` → upper/lower outputs as used here

If the adjacent indicator types can only own `String`, avoid broad type redesign. Just centralize and reduce repeated construction locally.

### 5A.3 Replace repeated first-valid scans with one-pass helpers

Implement one-pass helper(s) such as `first_valid_quad_f32(...)` / `first_valid_quad_f64(...)` or an equivalent helper that preserves current semantics.

The goal is to avoid repeated `position(...)` / repeated full scans across long signal vectors when determining the first index where all required series are usable.

Requirements:
- preserve behavior
- preserve NaN handling semantics
- use a single forward pass where possible
- update both the `f64` and `f32` KQSR signal-building paths if they both currently do repeated scans

### 5A.4 Tighten `build_kqsr_desired_positions(...)` without changing semantics

Implement the loop-tightening improvements from the plan.

Requirements:
- preserve exact trading semantics
- keep the output buffer zero-initialized where zero is the behaviorally correct prefix/default state
- do not use uninitialized output here if any pre-overwrite values are behaviorally observable
- replace repeated `.get(...).copied().unwrap_or(...)` style access in the hot loop with direct indexing after suitable up-front length assertions/checks
- keep or add `debug_assert!` / explicit checks as appropriate

You must explicitly preserve behavior for at least these flags/modes:
- `long_only = true`
- `allow_flip = false`
- `trade_on_next_bar = true`

### 5A.5 Add route accounting / perf trace hooks if feasible without API churn

If it can be added locally and cleanly, implement optional route accounting and/or perf tracing, especially for bench/debug use.

Preferred shape:
- `KqsrIndicatorRouteSummary`
- optionally `KqsrPerfTrace`

Capture at least:
- GPU attempts
- GPU successes
- CPU fallbacks
- ideally H2D bytes, D2H bytes, and kernel launch count if it can be threaded through cleanly

Do this additively. Do not force major public API churn just to add counters.

### 5A.6 Tests to add in `kqsr.rs`

Add or strengthen tests for:
- CPU `f32` path parity against CPU reference behavior
- GPU signal path parity against CPU reference (behind appropriate feature flags)
- descriptor/output-ID drift for `resolve_output_id(...)`
- behavior parity after loop tightening, especially:
  - `long_only = true`
  - `allow_flip = false`
  - `trade_on_next_bar = true`

If exact helper names differ, adapt to the local test style.

---

## 5B. `crates/app_core/src/kqsr_gpu_kernel.rs`

This is the main warm-path optimization area. Implement the performance-first uploaded path while preserving the existing wrapper path.

### 5B.1 Preserve the current wrapper API, but add an explicit warm/uploaded path

Keep the existing entrypoint working.

Add additive warm-path types/functions following the plan’s intent. Use the names from the docs where sensible, but adapt to the repo’s actual types if needed.

Preferred/additive types:
- `KqsrGpuSignalDeviceMatrices` or the nearest existing equivalent
- `KqsrGpuUploadedSeries`
- `KqsrGpuUploadedComboBatch`
- `KqsrScratch`

Preferred/additive functions:
- `upload_kqsr_series(...)`
- `upload_kqsr_combo_batch(...)`
- `eval_kqsr_metrics_gpu_kernel_uploaded(...)`
- keep `eval_kqsr_metrics_gpu_kernel_device_rows(...)`, but refactor it into a convenience wrapper that delegates to the uploaded warm path

### 5B.2 Reuse resident uploaded market series

Implement a reusable uploaded-series path so repeated evaluations on the same dataset do not repeatedly:
- upload `close_f32`
- allocate `d_lr`
- recompute / upload anything that can be made resident once per dataset

Requirements:
- upload `close_f32` once per dataset where possible
- compute/store `d_lr` once per dataset where possible
- make the uploaded series object reusable across repeated evaluations
- validate that uploaded resources belong to the active CUDA device/context before use

Strongly preferred:
- store `device_id` in uploaded objects
- make uploaded objects intentionally `!Send` / `!Sync` unless cross-thread support is explicitly designed
  - a standard safe pattern here is `PhantomData<Rc<()>>`

### 5B.3 Reuse uploaded combo batches and combo scratch

Implement reusable combo upload support.

Requirements:
- add uploaded combo-batch support for repeated candidate evaluation
- also add combo-buffer scratch reuse even on the convenience-wrapper path where practical
- avoid repeated `DeviceBuffer::from_slice(...)` churn for combo arrays when scratch reuse or uploaded combos can be used

### 5B.4 Reuse output scratch

Implement reusable output scratch so repeated evaluations do not reallocate `d_out` every call.

Requirements:
- add `d_out` scratch reuse with capacity tracking
- ensure correctness when combo/output size changes
- keep the logic clear and safe

### 5B.5 Minimize avoidable host-side prefills and copies

Implement copy-elision carefully.

#### Required rule
Only use uninitialized host/device memory when the code guarantees that every element is written before any read.

#### Required or strongly preferred changes
- confirm that the KQSR metrics kernel writes every output element for the copied-back metrics region on every path
- after that confirmation, replace `vec![0.0f32; out_len]` or equivalent prefill-before-copy with the shared uninitialized host-readback helper from `alloc_policy`, or the repo’s shared equivalent
- prefer shared helpers before writing new local unsafe code

#### Strongly preferred if supported cleanly by the repo
- use pinned host readback (`LockedBuffer` or repo equivalent) when full readback is still required
- decode metrics directly from the pinned host slice instead of copying through a second host vector

### 5B.6 Use async stream-aware upload/allocation/readback if supported

This is strongly preferred if the repo/CUDA stack supports it cleanly.

Check whether the current repo and `cust` version support these APIs or equivalents:
- `DeviceBuffer::from_slice_async(...)`
- `DeviceBuffer::uninitialized_async(...)`
- stream-ordered async copy back to host
- pinned host buffers for async D2H
- any shared repo helper such as `kernel_async_alloc_enabled()`

If supported and consistent with existing repo practice:
- use async upload helpers on the runtime stream
- use async device allocation for reusable buffers where appropriate
- use stream-ordered async D2H and a single final synchronize before decode

If the repo does not support this cleanly, keep the logic synchronous rather than inventing fragile local wrappers.

### 5B.7 Strengthen host-side validation and narrowing safety

Add the validation/hardening from the main plan and addendum.

Requirements:
- validate `fast_stride >= cols`
- validate `slow_stride >= cols`
- validate combo indices against `fast_rows` and `slow_rows`
- validate `start_t` / `end_t` against `cols` and ordering semantics expected by the kernel
- validate row-stride products with `checked_mul` where appropriate
- use checked `i32::try_from(...)` conversions for every launch scalar or size/count that is narrowed to `i32`
- keep the resulting error messages clear and strategy-specific

### 5B.8 Cache warm-path metadata if it is low-risk

If the runtime currently repeats launch-cap probing, kernel function lookup, or similar warm-path setup work, cache it after first successful use when this is easy and safe.

Examples from the plan:
- launch-cap metadata
- kernel handles/function handles

Do not overcomplicate this. Upload/copy elimination is still the higher priority.

### 5B.9 Keep the thread-local runtime design for now, but harden/document ownership

Do not redesign the runtime into a new executor architecture in this pass.

But do:
- keep in mind the `thread_local!` runtime duplication trade-off
- ensure uploaded objects are tied to the correct device/context
- avoid making cross-thread use appear safe if it is not
- add a short comment if helpful explaining that reuse is thread-local by design for now

### 5B.10 Tests to add in `kqsr_gpu_kernel.rs`

Add or strengthen tests for:
- combo bounds validation
- invalid stride handling
- wrapper vs uploaded-path parity on the same inputs if feasible
- repeated uploaded-path launches producing stable results if feasible

Do not block the main implementation on exotic GPU test infrastructure, but add the best coverage the repo already supports.

---

## 5C. `crates/app_core/benches/gpu_native_expansion_opt.rs`

Strengthen KQSR benchmark fidelity.

### 5C.1 Match CPU/GPU KQSR case coverage

Implement matched KQSR CPU and GPU case lists so the CPU/GPU crossover is visible.

Requirements:
- add a shared `KQSR_CASES` list or equivalent helper
- include small / medium / large pair-grid cases for both CPU and GPU bench paths
- update `bench_kqsr_cpu(...)` or equivalent so it uses matched cases rather than omitting the large case

### 5C.2 Add KQSR signal-build benchmarks

Add KQSR-specific signal-build microbenches so the cost of signal preparation is visible separately from the final optimization route.

Strongly preferred:
- CPU signal-build benchmark
- GPU signal-build benchmark
- ideally separate or at least make visible:
  - fast signal build
  - slow signal build
  - end-to-end signal build

### 5C.3 Add warm-path benchmark coverage if the uploaded API is exposed

If you implement the uploaded warm path, add at least one benchmark branch that measures:
- convenience wrapper path
- uploaded warm path

If pinned readback / async paths were implemented, add a benchmark branch for those too if the benchmark file can support it without becoming messy.

### 5C.4 Keep/add route and transfer visibility

If you implemented route accounting and/or transfer telemetry, print or otherwise surface it in the KQSR-specific benchmark output.

Keep benchmark naming clear so anyone reading the output can tell whether it is:
- wrapper path
- uploaded warm path
- full readback vs pinned readback
- sync vs async

---

## 5D. `crates/app_core/benches/composite_validation_workflow.rs`

Strengthen KQSR validation coverage.

### 5D.1 Add heavier KQSR validation profiles

Implement the validation-benchmark additions from the plan.

Requirements:
- add CPU KQSR validation profile at `100_000 / 32`
- add GPU KQSR validation profile at `100_000 / 32`

Strongly preferred if stable and low-risk:
- add CPU/GPU KQSR profile at `100_000 / 64`

### 5D.2 Preserve route visibility

Keep existing route probe printing.

If you implemented KQSR-local route accounting and/or transfer telemetry, surface it here too when it can be done cleanly.

## 6. Specific safety and correctness rules

These rules are mandatory.

### Uninitialized memory / copy-elision
- prefer shared repo helpers from `alloc_policy` before adding any new local unsafe helper
- only use uninitialized allocation if every element is guaranteed to be written before any read
- keep full/zero/prefix initialization where the initialized value is behaviorally observable
- do not change the semantics of NaN-prefix / warmup regions if they are part of the strategy contract

### CUDA / device ownership
- uploaded device resources must not be casually cross-thread or cross-device reused unless explicitly supported
- if the safest path is to mark uploaded objects `!Send` / `!Sync`, do it
- validate `device_id` or equivalent affinity before warm-path evaluation

### Benchmarks
- do not claim a benchmark is “GPU” if a meaningful part of the path can silently fall back to CPU without that being visible
- if route accounting exists, surface it
- keep CPU and GPU case coverage comparable where the benchmark is intended to compare them

### Scope discipline
- do not expand into product features, UI work, or unrelated strategy redesign
- do not let optional later ideas block the main file-local improvements

## 7. Acceptance criteria

Your implementation should satisfy these criteria as far as the repo/environment allows.

### Core correctness and behavior
- existing functionality is preserved
- no feature removal
- no obvious strategy-semantic drift
- wrapper API still works
- uploaded warm path produces the same metrics as the wrapper path on the same inputs within normal tolerance

### Performance-oriented structure
- repeated KQSR evaluation no longer always re-uploads `close_f32`, re-allocates `d_lr`, re-allocates all combo buffers, and re-allocates `d_out`
- `compute_indicator_f32(...)` no longer falls through to `compute_indicator_f64(...)` for CPU fallback
- repeated first-valid scans are eliminated or materially reduced
- unnecessary host prefill before device readback is removed where safe
- pinned readback / async transfer is used if the repo supports it cleanly and you implement it

### Validation/hardening
- combo and stride bounds are validated
- narrowing conversions are checked
- row-stride arithmetic is protected with checked math where appropriate
- tests are added or strengthened for the changed behavior

### Benchmark fidelity
- `gpu_native_expansion_opt.rs` has matched KQSR CPU/GPU case coverage
- KQSR signal-build microbench coverage exists
- `composite_validation_workflow.rs` includes heavier KQSR profiles
- route/transfer visibility is improved if you add the accounting hooks

## 8. Workflow to follow

Use this execution order.

1. Read the plan docs and inspect the target files.
2. Inspect adjacent shared helpers before writing any new unsafe allocation/copy code.
3. Implement tests or strengthen parity coverage early where practical.
4. Implement the low-risk `kqsr.rs` changes.
5. Implement the main `kqsr_gpu_kernel.rs` warm-path/upload/reuse changes.
6. Implement validation and benchmark improvements.
7. Implement pinned/async improvements if the repo supports them cleanly.
8. Run formatting, compile/tests/benches as far as the environment allows.
9. Fix warnings and obvious cleanup.

## 9. Commands / verification

Use the repo’s actual workspace/package names. If the exact package names differ, inspect `Cargo.toml` and adapt. Prefer running the narrowest relevant checks first.

At minimum, try to run the closest equivalents of:
- `cargo fmt --all`
- `cargo test` for KQSR-related tests
- `cargo check` or `cargo test` for the affected crate without CUDA features
- `cargo check` / `cargo test` with CUDA features if the environment supports it
- benchmark builds for the touched bench files if feasible

If the environment cannot actually execute CUDA tests/benches, still:
- ensure feature gating is correct
- compile-check as much as possible
- explain exactly what could not be run and why

## 10. Final output expected from you

When you finish, provide:
1. a concise summary of what you changed
2. the list of files modified
3. the main performance improvements implemented
4. the main correctness/safety guardrails added
5. the tests/checks you ran and their results
6. any optional-later items you intentionally did **not** implement
7. any repo/API limitations that blocked a clean implementation

## 11. Important implementation bias

If you have to choose, prioritize this order:
1. correctness and parity
2. removal of repeated upload/allocation/copy waste
3. safe reuse of uploaded buffers and scratch
4. benchmark fidelity and route visibility
5. async/pinned enhancements
6. deeper optional experiments

## 12. Concrete checklist to treat as actionable work

Use this as a literal implementation checklist.

### `kqsr.rs`
- implement `series_to_f32(...)`
- change `compute_indicator_f32(...)` to direct CPU compute on fallback
- ensure the duplicate failed-CUDA-attempt pattern is removed from the `f32` fallback path
- add output-ID caching (`OnceLock<String>` or equivalent) for KQSR indicator outputs
- centralize KQSR output-ID resolution
- centralize KQSR param construction enough to reduce scattered repeated `IndicatorParamKV` allocation churn
- add/implement one-pass `first_valid_*` helper(s)
- replace repeated first-valid scans in both `f64` and `f32` paths where present
- tighten `build_kqsr_desired_positions(...)` with direct indexing and preserved semantics
- add parity tests for CPU `f32`
- add GPU signal parity tests where supported
- add descriptor-drift tests for resolved output IDs
- add behavior-parity tests for `long_only`, `allow_flip`, `trade_on_next_bar`
- optionally add route/perf trace hooks if clean

### `kqsr_gpu_kernel.rs`
- add uploaded-series type
- add uploaded-combo type
- add scratch type with reusable `d_out`
- add combo scratch reuse on wrapper path where practical
- add uploaded warm-path evaluation function
- make existing wrapper delegate to uploaded/warm path internally
- reuse uploaded `close` / `d_lr`
- reuse uploaded combos
- confirm kernel full-write coverage for output before using uninitialized readback
- replace wasteful host prefill-before-copy with shared uninitialized helper where safe
- use pinned host readback if supported and safe
- use async stream-aware upload/allocation/readback if supported and aligned with repo patterns
- add device/context affinity validation
- make uploaded GPU objects `!Send` / `!Sync` unless explicit support exists
- add stride/bounds/overflow validation
- use checked `i32` conversions for launch scalars
- cache launch metadata/function handles if easy and low-risk
- add tests for combo bounds / stride validation
- add wrapper-vs-uploaded parity tests if feasible
- add repeated-uploaded-launch consistency tests if feasible

### `gpu_native_expansion_opt.rs`
- add matched KQSR CPU/GPU case coverage including the large case
- add KQSR signal-build microbenches
- add uploaded warm-path benchmark branch if exposed
- add pinned/async benchmark branches if implemented and clean
- surface route/transfer metadata if available

### `composite_validation_workflow.rs`
- add KQSR CPU `100_000 / 32`
- add KQSR GPU `100_000 / 32`
- add `100_000 / 64` if stable and low-risk
- keep route probe output
- surface route/transfer metadata if available

## 13. Last guardrail

Do not stop at analysis or TODO comments. Make the code changes.

However, if a specific optional item from the docs would require a broad unrelated redesign, skip that item, keep the main implementation intact, and explicitly call it out in your final summary.

