# Codex implementation prompt: CARM GPU-native optimization + validation hardening

Copy/paste the prompt below into your OpenAI Codex agent.

---

You are working inside a Rust repo that contains a CARM strategy implementation and related GPU/benchmark code.

Your task is to **fully implement the CARM-specific optimization, performance, memory-copy reduction, fallback-hardening, and correctness improvements** described in the attached/spec files below, while preserving current features and behavior:

## Read these spec documents first

Treat these as the implementation spec, in this precedence order:

1. `CARM_GPU_NATIVE_OPTIMIZATION_PLAN_ADDENDUM.md`
2. `CARM_GPU_NATIVE_OPTIMIZATION_PLAN_FULL.md`
3. `gpt54_feature_review_map.md` — only the section `## 5. Strategy-local GPU-native optimization and validation routes`

If the addendum refines or corrects the full plan, **the addendum wins**.

## Repo files to modify

The primary implementation scope is limited to the CARM strategy and its strategy-local benchmarks. Modify these canonical repo files (or their exact equivalents if paths differ):

- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

You may make **small, necessary, CARM-adjacent edits** in supporting files if required to compile or to thread new CARM-specific data through cleanly, for example:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- any CARM-specific caller/build path that must pass a new cache key, trace object, or `CarmSignalsF32`

Do **not** broaden this into a repo-wide architecture rewrite.

## High-level objective

Improve **performance, optimization, and correctness** for the **CARM** strategy, especially for the **CUDA-executed path**, without removing features or breaking functionality.

Primary goals:

- reduce unnecessary host↔device copies
- reduce repeated allocations and repeated staging work
- use uninitialized memory **only where safe and fully overwritten before read**
- improve warm-path performance for repeated same-series runs
- make GPU-labeled benchmarks fail on silent CPU fallback
- preserve CPU/GPU parity and existing route blockers
- improve correctness validation around bounds, thresholds, and fallback behavior

## Important constraints

- This work is **CARM-only**. Do not drift into other strategies unless a tiny shared helper change is required.
- The Rust-side CARM GPU entrypoint exists, but the actual CUDA body for `carm_backtest_metrics_f32` may or may not be present in the repo. If it is **not** present, do **not** invent kernel-body changes.
- If the actual CUDA source **is** present, you may inspect it **only** for consistency with flag bits / interface assumptions. Do **not** make speculative kernel-body tuning changes unless the evidence is strong and the change is tightly scoped.
- Keep the optimization-first, parity-preserving, benchmark-aware posture from the spec docs.
- Do not remove overlays / stop-loss / take-profit blockers unless you are also implementing full parity support for them. Otherwise keep the blockers explicit.
- Do not silently rely on pointer+length cache identity as a production correctness assumption unless the caller contract explicitly guarantees immutability.
- Do not use uninitialized memory for non-POD / drop-bearing host types. In particular, do **not** use uninitialized `Vec<Metrics>`.

## Implementation priorities

Implement the work in this order:

### Phase 1 — mandatory, highest ROI, lowest risk

1. `carm_gpu_kernel.rs`
   - add reusable device scratch/buffer ownership
   - remove repeated allocations where possible
   - add safe close/log-return reuse
   - add reusable host readback storage without zero-fill
   - strengthen validation and checked conversions
2. `carm.rs`
   - remove the CPU fallback detour through `Vec<f64>` in `compute_indicator_host_f32(...)`
   - avoid repeated failed CUDA attempts in a single CARM prep request
   - add the `f32` aggregate signal path
3. `gpu_native_expansion_opt.rs`
   - hard-fail GPU-labeled CARM optimization benches on fallback
   - align CARM request shaping and warm-path measurement intent
4. `composite_validation_workflow.rs`
   - hard-fail GPU-labeled CARM validation benches on fallback
   - add a dedicated CARM GPU validation profile and better CARM-specific benchmark input shaping

### Phase 2 — implement if cleanly supported by the repo with local changes

- fallback tracing / route metadata helpers for CARM signal prep
- explicit cold vs warm benchmark separation and cache reset hooks
- request clone microbench if clone/setup cost is non-trivial
- dedicated CARM prep microbench if it can be added cleanly
- small CARM-only telemetry counters behind a non-default cfg or benchmark/test context
- OTTO paired-output optimization **only if** the adapter already supports a clean multi-output path

### Phase 3 — only if already supported or trivially local; otherwise document and stop

Do **not** force these unless the repo already makes them straightforward and safe:

- prepared/session-style CARM GPU execution object
- pinned host memory + async copy overlap
- combo bucketing/sorting for row-locality
- narrower packed combo-index representations
- device-side shortlist/top-k reduction to avoid full metrics readback
- device-resident indicator-output path for CARM if it requires broader runtime redesign

If any Phase 3 item is not clearly local and safe, leave it unimplemented and mention it explicitly in your final report.

---

## Detailed implementation requirements

### 1) `crates/app_core/src/carm_gpu_kernel.rs`

#### A. Reusable GPU scratch and memory-copy reduction

Implement runtime-owned reusable scratch for:

- `close_f32`
- log returns (`d_lr`)
- combo arrays:
  - `fast_row_index`
  - `otto_row_index`
  - `start_t`
  - `end_t`
- output metrics buffer
- host readback buffer for raw `f32` metrics rows

Use a helper similar to `ReusableDeviceBuffer<T>`.

Preferred behavior:

- **grow-only capacity reuse** if the local `cust` API cleanly supports copying a shorter slice into a larger device allocation via logical prefix semantics
- acceptable fallback: exact-size reuse if grow-only reuse is awkward in this repo

Use `DeviceBuffer::uninitialized(...)` only for scratch/output buffers that are fully written before being read.

#### B. Reuse close/log-return state safely

Add warm-path reuse so repeated runs on the **same close series** do not re-upload the close array and do not recompute log returns.

However, do **not** rely on pointer+length alone as a silent correctness assumption.

Preferred implementation:

- extend `CarmGpuDeviceSignalMatrices` with something like `close_cache_key: Option<u64>`
- store corresponding cache identity in the runtime
- reuse only when the stable identity matches

If adding a cache key to call sites is impractical, you may only use pointer+length reuse behind a clearly documented immutable-input contract or benchmark-only assumption. Do not make stale-device-data correctness risks invisible.

Also add:

- `clear_carm_gpu_runtime_cache(device_id: Option<u32>)`
- use this helper where useful for tests/bench intent or error recovery

#### C. Strengthen input validation

The validation logic must do more than current shape/null checks.

Implement or tighten validation for:

- non-empty series
- `close_f32.len() == cols`
- non-zero rows/strides
- `mixed_stride >= cols`
- `fast_stride >= cols`
- `otto_stride >= cols`
- non-null row-major pointers
- combo array length equality
- `fast_row_index[k]` in `[0, fast_rows)`
- `otto_row_index[k]` in `[0, otto_rows)`
- `start_t >= 0`
- `end_t >= 0`
- `start_t <= end_t`
- `end_t < cols`
- checked `usize -> i32` conversion for every launch argument / dimension sent to the kernel

Add a helper like:

- `checked_i32(name: &str, value: usize) -> Result<i32, String>`

Replace every raw `as i32` that participates in launch arguments or kernel dimensions.

#### D. Improve host readback storage and uninitialized-memory usage

For the host raw metrics readback buffer:

- keep `Vec<f32>` in the runtime
- do **not** zero-fill it before `copy_to(...)`
- use the corrected pattern from the addendum, i.e. clear → reserve_exact(len) if needed → `unsafe { set_len(len) }`
- document the safety invariant: `f32` is POD and the device-to-host copy fully overwrites the initialized length before read

Do **not** apply this pattern to `Vec<Metrics>`.

Keep decode of `Metrics` safe, using `Vec::with_capacity(...)` + `push(...)` or equivalent safe construction.

#### E. Borrowing and launch-path cleanup

Refactor `launch_compute_log_returns_f32(...)` to accept `&Module` and `&Stream` instead of `&mut CarmCudaRuntime` if that makes scratch reuse and borrowing clean.

If clean and ergonomic in this repo, also consider caching launch-cap metadata. Do **not** force caching `cust::Function` handles if lifetimes become messy.

#### F. Replace magic kernel flags with named constants

Replace literal flag bits with named constants, e.g.:

- `FLAG_LONG_ONLY`
- `FLAG_NO_FLIP`
- `FLAG_TRADE_ON_NEXT_BAR`
- `FLAG_ENABLE_CARM_EXIT_VOTES`

If the actual CUDA kernel source is present, mirror the constants or at least verify consistency.

#### G. Tests in `carm_gpu_kernel.rs`

Add/expand pure-Rust tests for:

- valid basic shapes pass
- combo vector length mismatch fails
- out-of-range `fast_row_index` fails
- out-of-range `otto_row_index` fails
- negative `start_t` / `end_t` fails
- `start_t > end_t` fails
- `end_t >= cols` fails
- inadequate stride fails
- overlay/SL/TP blockers remain enforced where practical

Keep these lightweight and not dependent on a real CUDA device where possible.

---

### 2) `crates/app_core/src/carm.rs`

#### A. Remove the `Vec<f64>` fallback detour

Add `series_to_f32(...)` and rewrite `compute_indicator_host_f32(...)` so that when CUDA is not used or CUDA fails, CPU fallback goes directly to `Vec<f32>`.

Do **not** route CPU fallback through `compute_indicator_f64(...)`.

The new behavior must:

- avoid a temporary `Vec<f64>`
- avoid the cast-back detour
- avoid a second failed CUDA attempt indirectly triggered through `compute_indicator_f64(...)`

#### B. Add per-request CUDA failure short-circuiting

A single CARM signal-prep request can attempt several indicator computations:

- coppock
- ao
- apo
- rocr
- otto/hott
- otto/lott

If the first CUDA attempt fails for a route-wide reason, do not keep retrying CUDA for the remaining indicators in the same request.

Implement a small per-request state object such as `CarmGpuAttemptState` and thread it through the `*_f32` CARM signal-prep helpers.

The state should:

- start with `allow_cuda = true`
- record the first CUDA failure reason
- disable further CUDA attempts after the first route-wide failure

Use this for the `HostF32` path at minimum.

#### C. Add an `f32` aggregate signal path

Add a `CarmSignalsF32` struct that aggregates:

- coppock
- ao
- rocr
- apo
- otto_hott
- otto_lott
- `first_valid`

Add `compute_carm_signals_f32(...)` that builds this aggregate from the existing `*_f32` helpers.

Update CARM GPU-prep callers to prefer this aggregate where applicable so the CARM GPU route can stay on `f32` surfaces end-to-end as much as possible.

#### D. Optional trace surface for fallback visibility

If easy to add locally, introduce a lightweight trace object such as `CarmIndicatorRouteTrace` and/or a traced `compute_indicator_host_f32_with_trace(...)` helper so CARM can surface:

- preferred_gpu
- attempted_cuda
- used_cuda
- fallback_reason

Keep the existing API working; a wrapper that discards the trace is fine.

#### E. OTTO paired-output optimization

Audit whether the dynamic indicator adapter already supports computing OTTO once and extracting both `hott` and `lott`.

If the adapter already supports a clean multi-output path:

- implement a CARM-specific paired-output helper
- use it in both `compute_carm_otto_signals(...)` and `compute_carm_otto_signals_f32(...)`

If multi-output is **not** already supported cleanly, do **not** invent a large adapter redesign here. Leave the current dual path intact and mention the limitation in your final report.

#### F. Preserve semantics; audit threshold mismatch

There is a possible semantic mismatch between:

- OTTO prep using `cfg.otto_threshold.abs().max(0.6)`
- strategy decisions comparing against raw `cfg.otto_threshold`

Do not casually change behavior.

Instead:

- audit the intent
- if it is clearly intentional, document it and add tests
- if it appears accidental but changing it would risk behavior drift, prefer tests + documentation unless you can prove the safe correction

#### G. Add correctness/parity tests in `carm.rs`

Add or expand tests for:

- `trade_on_next_bar`
- `long_only`
- `allow_flip` if practical
- boundary equality semantics:
  - `rocr == 1.0`
  - `apo == 0.0`
  - `coppock == 0.0`
  - `otto_spread == +threshold`
  - `otto_spread == -threshold`
- any intended OTTO threshold clamp semantics

The goal is to harden correctness and fp32 boundary behavior without changing intended CPU reference semantics.

---

### 3) `crates/app_core/benches/gpu_native_expansion_opt.rs`

#### A. GPU-route assertions must be mandatory

Add an `assert_gpu_backend(...)` helper and use it:

- for the route probe
- for every measured GPU result in CARM GPU-labeled benches

GPU-labeled CARM optimization benches must fail if they silently run on CPU.

#### B. Align CARM request shaping with the plan

The CARM optimization bench should not inherit generic MA-type shaping if that is not faithful.

For `make_carm_req(...)`:

- set fast MA type to `ema`
- set slow MA type to `ema`
- keep ranges strategy-appropriate

Also consider using two builders if useful:

- product-fidelity CARM request
- perf-only CARM request

For the perf-only builder, it is acceptable to set `heatmap_bins = None` if heatmap work is not part of the measurement question.

#### C. Warm each case before timing it

Add an explicit warm-up run per case before Criterion timing begins.

The CARM benchmark should distinguish between:

- initial route probe
- per-case warm run
- measured run(s)

#### D. Cold vs warm clarity

If practical, add explicit naming or grouping for cold vs warm CARM GPU measurements.

At minimum, make the warm intent clear. If you add true cold measurements, use `clear_carm_gpu_runtime_cache(...)` intentionally.

#### E. Optional steady-state / clone microbench

If cleanly supported, add:

- dedicated steady-state CARM GPU benchmark
- optional request-clone-only microbench if clone/setup cost is materially affecting interpretation

---

### 4) `crates/app_core/benches/composite_validation_workflow.rs`

#### A. GPU-route assertions must be mandatory here too

Add `assert_gpu_backend(...)` and use it:

- after every GPU route probe
- inside every GPU-labeled measured closure

Do not allow CARM GPU validation benches to silently benchmark CPU fallback.

#### B. Add CARM-specific candles

The generic synthetic candles are not enough for strategy-local CARM evaluation.

Add a CARM-like momentum/regime candle generator similar to the one described in the spec docs and use it for dedicated CARM GPU validation profiling.

#### C. Add a dedicated CARM GPU validation group

Add a dedicated benchmark group for CARM GPU validation, using:

- `Backend::GpuKernel { device_id: 0 }`
- CARM-specific candles
- route probe + warm run + measured run
- GPU assertions on all probe/warm/measured runs

Include walk-forward / reopt / cscv / spa settings similar to the plan unless repo constraints require small adjustments.

#### D. Candidate shaping

Keep the existing `make_carm_candidates(...)` if continuity matters, but add a more diverse CARM candidate builder if possible for the dedicated CARM group.

It is reasonable to diversify mildly across:

- fast/slow lengths
- `rocr_period`
- `otto_period`
- `otto_threshold`

Do not explode combinatorics unnecessarily.

#### E. Optional benchmark-fidelity add-ons

If easy and local, add:

- explicit cold vs warm naming
- cache-reset use for cold benches
- request-clone-only microbench or at least a way to quantify clone cost separately
- direct CARM prep microbench if there is a clean place for it

---

## Memory and unsafe-code rules

Follow these rules exactly:

1. Prefer reducing copies before adding more complexity.
2. Reuse device allocations where possible.
3. Reuse host `Vec<f32>` readback storage without zero-fill if it is fully overwritten before read.
4. Use uninitialized memory only for:
   - POD host buffers that are fully overwritten before read, like raw `f32`
   - device scratch/output buffers that kernels/copies fully overwrite before read
5. Do **not** use uninitialized memory for:
   - `Vec<Metrics>`
   - any non-POD / drop-bearing type
   - any buffer that is only partially overwritten before read
6. Every `unsafe` block must have a short comment explaining the invariant that makes it safe.

---

## What not to do

- Do not remove existing features.
- Do not change broad product behavior outside CARM.
- Do not add UI or unrelated workflow changes.
- Do not remove route blockers for overlays / SL / TP.
- Do not remove validation checks to save CPU time.
- Do not silently assume pointer+length cache identity is always correct.
- Do not invent unsupported multi-output adapter APIs if they do not already exist.
- Do not fabricate kernel-body optimizations if the actual CUDA source is missing.
- Do not claim wins without running the relevant tests/benches you can run locally.

---

## Validation and benchmark commands

Run as much of the following as your environment supports.

At minimum:

- `cargo fmt`
- targeted tests for the modified modules
- `cargo test` for affected crate(s) if feasible

If CUDA bench environment is available, run:

- `VECTORBT_GPU_NATIVE_BENCH_ONLY=carm cargo bench --bench gpu_native_expansion_opt --features cuda,cuda-backtest-kernel`
- `cargo bench --bench composite_validation_workflow --features cuda,cuda-backtest-kernel`

If the environment does **not** support CUDA execution, still:

- compile/check the CUDA-gated codepaths as far as possible
- run non-CUDA unit tests
- leave exact bench commands and expected benchmark deltas in the final report

If you add cold-vs-warm cache-reset benches, document exactly how you reset runtime state.

---

## Required final deliverable from you

After making the changes, provide a concise but complete implementation report with:

1. **Files changed**
2. **What changed in each file**
3. **Which required items were implemented**
4. **Which optional items were implemented**
5. **Which optional/speculative items were intentionally not implemented and why**
6. **Tests/benches run**
7. **Any compile/bench blockers**
8. **Any correctness-sensitive decisions**
   - cache-key choice for close/log-return reuse
   - OTTO threshold semantics
   - whether OTTO paired-output was supported
   - whether actual CUDA kernel source was found

---

## Success criteria

The work is only complete if all of the following are true:

- GPU-labeled CARM benches hard-fail on silent CPU fallback
- repeated same-series CARM GPU runs stop re-uploading unchanged close data (or conservatively avoid unsafe reuse if stable identity cannot be proven)
- repeated same-series CARM GPU runs stop recomputing log returns unnecessarily
- per-call combo/output allocation churn is materially reduced
- host metrics readback no longer zero-initializes before full overwrite
- `compute_indicator_host_f32(...)` no longer detours through `Vec<f64>`
- repeated failed CUDA attempts inside one CARM signal-prep request are prevented
- correctness/boundary tests are stronger than before
- no intended features or route blockers are removed

Implement the code now.

