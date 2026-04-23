# Codex implementation prompt — CDMC GPU optimization and correctness hardening

Use the following as the **full task prompt** for the OpenAI Codex agent.

---

You are working in a Rust/CUDA codebase. Your job is to **fully implement** the CDMC GPU optimization and correctness plan described in the attached markdown guides, while preserving functionality and avoiding feature regressions.

## Read these documents first and treat them as the implementation spec

The following attached markdown files are the authoritative guides for this task:

1. `cdmc_gpu_optimization_plan.md`
2. `cdmc_gpu_optimization_addendum.md`
3. `gpt54_feature_review_map.md`

Use `gpt54_feature_review_map.md` only as **scope control and prompt intent**. The implementation details come from the two CDMC markdown guides.

### Scope guard from the review map

This task is limited to:

- **Bundle 5: Strategy-local GPU-native optimization and validation routes**
- **CDMC strategy only**

Do **not** broaden this task to other strategies unless a tiny adjacent change is strictly required to support the CDMC implementation safely.

## Primary goal

Improve **performance, allocation discipline, copy elision, warm-path GPU efficiency, fallback transparency, and correctness** for the CDMC strategy’s GPU-native optimization and validation route.

The highest-priority goals are:

1. reduce unnecessary host↔device memory copies
2. reduce repeated allocation and per-call scratch churn
3. keep the CDMC GPU path device-native as far as possible
4. preserve correctness and parity with CPU/reference behavior
5. make validation benchmark routing trustworthy and explicit
6. use uninitialized memory **only where it is provably safe**
7. avoid removing features or breaking existing functionality

## Files to modify

These are the main repo files to update:

- `crates/app_core/src/cdmc.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

## Adjacent files you may inspect or modify only if truly needed

Read these for patterns and reuse if needed, but do **not** broaden the scope unnecessarily:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - use as the model for resident market-series reuse, scratch reuse, host readback patterns, and launch-cap caching
- `crates/app_core/src/dynamic_indicator_adapter.rs`
  - only if needed to confirm or improve combined `correlation_cycle` output handling or dispatch-key/output caching
- any shared alloc-policy helper or readback helper already present in the repo
  - only if needed for safe uninitialized host readback
- any file that defines or heavily uses `PreparedCudaIndicatorInputs`
  - only if needed to preserve or improve reuse of prepared device inputs in adjacent orchestration

If you modify an adjacent file, keep the change minimal, local, and directly justified by the CDMC plan.

## Non-goals / prohibitions

Do **not**:

- redesign the whole GPU stack
- broaden the work to other strategies
- remove overlays / stop-loss / take-profit features
- change behavior silently without tests or comments
- add risky `unsafe` usage unless the invariant is clear and documented
- switch host buffers with observable NaN-prefix semantics to uninitialized memory unless you can prove every element is written before any read
- rush into kernel-body redesign, fusion, inline log-return computation, or matrix transposition unless you have profiler-backed evidence and the change is still local and safe

If a deeper kernel-body optimization seems attractive but is beyond the first safe wave, leave a clear comment/TODO/design note rather than forcing it into this task.

## Required implementation approach

Follow this sequence:

1. read the attached markdown guides in full
2. inspect the target repo files and confirm current structures/callers
3. implement the **core first-wave changes**
4. add or update tests
5. update benchmarks/perf gates
6. only then consider clearly-labeled optional second-wave changes if they are still local and safe

If one subtask is blocked by a missing lower-level API, do **not** stop the whole task. Implement the safe partial improvement now and leave a focused TODO/hook for the remaining piece.

Example: if direct MAMA `first_output_valid` surfacing is not already available at the lower layer, still implement the MAMA runtime/scratch reuse and keep the host scan as a temporary correctness path.

---

# Detailed implementation requirements

## 1) `crates/app_core/src/cdmc.rs`

### 1.1 Add an explicit device-native prepared CDMC signal object

Implement a CUDA-gated prepared signal object for GPU-native CDMC validation/optimization flows.

Use the plan’s shape as the target semantic structure. If exact naming must differ slightly to fit the codebase, preserve the intent.

Preferred shape:

```rust
#[cfg(feature = "cuda")]
pub struct CdmcGpuPreparedSignals {
    pub dpo: CdmcPeriodSweepDevice,
    pub cycle_angle: CdmcPeriodSweepDevice,
    pub cycle_state: CdmcPeriodSweepDevice,
    pub mama: CdmcDeviceMatrix,
    pub fama: CdmcDeviceMatrix,
    pub decycler: CdmcPeriodSweepDevice,
    pub first_valid: usize,
}
```

Add a CUDA-gated helper like:

```rust
#[cfg(feature = "cuda")]
pub fn prepare_cdmc_gpu_validation_signals(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    periods: CdmcResolvedPeriods,
    cfg: &CdmcStrategyParams,
) -> Result<CdmcGpuPreparedSignals, String>
```

This function should:

- compute DPO sweep on device
- compute correlation-cycle `angle` output on device
- compute correlation-cycle `state` output on device
- compute MAMA/FAMA on device
- compute decycler sweep on device
- compute `first_valid` as the max of the relevant component first-valid positions
- return a device-native prepared object that downstream GPU paths can consume without materializing host signal vectors

### 1.2 Keep `compute_cdmc_signals(...)` as the host/reference path

Do **not** remove `compute_cdmc_signals(...)`. Keep it for:

- CPU/reference execution
- debugging
- parity checks
- tests that need host vectors

But make it explicit in comments/docstrings that it is **not** the preferred path for GPU optimization/validation.

Add a clear comment/docstring near the top of the function explaining:

- this is the host/reference CDMC signal path
- GPU optimization/validation should prefer the device-native prep path
- device pointers should be passed into `cdmc_gpu_kernel.rs`

### 1.3 Improve MAMA/FAMA device path without breaking correctness

Current problem: `compute_cdmc_mama_pair_cuda_device(...)` computes device MAMA/FAMA, then does an extra full host-side `mama_batch_into_host_f32(...)` pass just to determine first-valid. That wastes work.

Implement this in two levels:

#### Level A — required first-wave improvement

Add a cached CDMC MAMA runtime/scratch structure, ideally per device or equivalent local runtime reuse, containing the same semantics as:

```rust
#[cfg(feature = "cuda")]
struct CdmcMamaRuntime {
    cuda: CudaMama,
    len_capacity: usize,
    d_inv_dp: Option<DeviceBuffer<f32>>,
    d_mama: Option<DeviceBuffer<f32>>,
    d_fama: Option<DeviceBuffer<f32>>,
    d_fast_limit: Option<(u32, DeviceBuffer<f32>)>,
    d_slow_limit: Option<(u32, DeviceBuffer<f32>)>,
    host_mama: Vec<f32>,
    host_fama: Vec<f32>,
}
```

Implement capacity-aware reuse for:

- `CudaMama`
- `d_inv_dp`
- `d_mama`
- `d_fama`
- one-element fast-limit buffer
- one-element slow-limit buffer
- host scan scratch if the host pass still temporarily remains

Do **not** allocate these fresh on every hot call when device/capacity are unchanged.

#### Level B — remove the extra host pass if the lower layer supports it cleanly

If the lower-level CUDA MAMA path can expose `first_output_valid` directly, remove the extra host-side `mama_batch_into_host_f32(...)` pass entirely.

If it cannot be done cleanly within the current repo surface, then:

- keep the host pass temporarily for correctness
- reuse host scratch instead of reallocating each call
- leave a clear TODO/hook pointing to the remaining lower-level work needed to fully delete the host pass

### 1.4 Safe uninitialized-memory rule for MAMA host scan buffers

Do **not** switch `host_mama` / `host_fama` to uninitialized host memory unless you prove that the lower-level call writes every element before any read.

The NaN prefix currently matters for the `first_valid` scan. If any prefix can remain unwritten, keep NaN-filled reusable scratch.

### 1.5 Remove unnecessary full zero-fill in `build_cdmc_desired_positions(...)`

Refactor `build_cdmc_desired_positions(...)` to avoid full-vector zero initialization.

Required behavior:

- keep semantics identical
- allocate with capacity
- zero only the required prefix `[0..start_t)`
- `push(...)` tail values instead of indexing into a fully pre-zeroed vector
- do this without `unsafe`

Preferred pattern:

- `Vec::with_capacity(n)`
- `resize(start_t, 0i32)`
- push computed positions for the remainder

Be careful to preserve:

- `trade_on_next_bar`
- `allow_flip`
- `long_only`
- the current exit/opposing-vote logic
- threshold and zero-cross edge behavior

### 1.6 Replace tiny fixed-size heap allocations with arrays where possible

In the CDMC host/reference path and adjacent helper functions, replace tiny fixed-size `Vec`s used only as temporary parameter bundles with arrays or stack-friendly equivalents when practical.

The plan explicitly calls this out for places like:

- `compute_cdmc_signals(...)`
- `compute_decycler_f64(...)`

This is a small but safe cleanup.

### 1.7 Optional-but-good local cleanups in `cdmc.rs`

Implement these if they are straightforward and local:

- cache repeated dynamic-indicator descriptor/key/output resolution with `OnceLock` if the registry is effectively stable after startup
- investigate whether `correlation_cycle` `angle` and `state` outputs can be fetched without duplicate work in adjacent indicator dispatch code
- reduce `first_valid` scanning work in the host/reference path only if deterministic warmup rules are confirmed

Do **not** guess. If determinism is uncertain, keep the conservative path.

### 1.8 CUDA gating and API stability

All new GPU-only structures and helpers must be properly gated with the same feature strategy the codebase already uses.

Do not break CPU-only or non-CUDA builds.

---

## 2) `crates/app_core/src/cdmc_gpu_kernel.rs`

This file is the most important local performance surface.

### 2.1 Expand the market-series input surface to avoid forced host re-upload

Current problem: the wrapper always takes host `close_f32`, uploads it every call, allocates `d_lr` every call, computes log returns every call, then launches the metrics kernel.

Change the CDMC GPU signal input structure so it can support:

- host close data when that is all the caller has
- device-resident close data when already available
- device-resident log returns when already available
- cache-key-safe resident host-upload reuse when host close is reused across repeated evaluations

Preferred structure semantics:

```rust
pub struct CdmcGpuDeviceSignalMatrices<'a> {
    pub cols: usize,

    pub close_host_f32: Option<&'a [f32]>,
    pub close_host_cache_key: Option<u64>,

    pub close_device_ptr: Option<u64>,
    pub lr_device_ptr: Option<u64>,

    pub dpo_row_major_ptr: u64,
    pub dpo_rows: usize,
    pub dpo_stride: usize,

    pub cycle_angle_row_major_ptr: u64,
    pub cycle_state_row_major_ptr: u64,
    pub cycle_rows: usize,
    pub cycle_stride: usize,

    pub mama_ptr: u64,
    pub fama_ptr: u64,

    pub decycler_row_major_ptr: u64,
    pub decycler_rows: usize,
    pub decycler_stride: usize,
}
```

If the exact struct layout must differ slightly, keep the same meaning.

### 2.2 Important cache-key rule

Do **not** key host-upload reuse only by `host_ptr + len`.

If you implement resident reuse for host-provided close data, require an explicit trusted cache key such as `close_host_cache_key`.

Rules:

- if `lr_device_ptr` is provided, use it directly
- else if `close_device_ptr` is provided, compute/reuse `d_lr` from that device close
- else if `close_host_f32` is provided **and** a trusted cache key is provided, you may reuse cached `d_close`/`d_lr` if the key and length match
- else if only `close_host_f32` is provided with no trusted key, upload per call and do **not** retain a stale resident cache entry

This is a correctness guardrail.

### 2.3 Add runtime scratch reuse

Add reusable runtime scratch to `CdmcCudaRuntime` or an equivalent runtime-owned structure.

Target semantics:

```rust
struct CdmcResidentMarketSeries {
    cache_key: u64,
    len: usize,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
}

struct CdmcCudaScratch {
    // capacity-managed scratch buffers
    d_lr: Option<DeviceBuffer<f32>>,
    d_combo_dpo: Option<DeviceBuffer<i32>>,
    d_combo_cycle: Option<DeviceBuffer<i32>>,
    d_combo_decycler: Option<DeviceBuffer<i32>>,
    d_combo_start: Option<DeviceBuffer<i32>>,
    d_combo_end: Option<DeviceBuffer<i32>>,
    d_out: Option<DeviceBuffer<f32>>,
    host_metrics: Vec<f32>,
}

struct CdmcKernelLaunchCache {
    device_caps: Option<DeviceLaunchCaps>,
    log_returns_caps: Option<KernelLaunchCaps>,
    metrics_caps: Option<KernelLaunchCaps>,
}

struct CdmcCudaRuntime {
    device_id: u32,
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    resident_close: Option<CdmcResidentMarketSeries>,
    scratch: CdmcCudaScratch,
    launch_cache: CdmcKernelLaunchCache,
}
```

Reuse these across repeated calls where capacity/device are compatible:

- `d_lr`
- `d_combo_dpo`
- `d_combo_cycle`
- `d_combo_decycler`
- `d_combo_start`
- `d_combo_end`
- `d_out`
- host metrics readback scratch

Do not allocate these fresh every evaluation when existing capacity is sufficient.

Use patterns from `gpu_backtest_kernel.rs` where possible rather than inventing a completely separate approach.

### 2.4 Refactor log-return launching to work from raw device pointers

Refactor `launch_compute_log_returns_f32(...)` so it can operate from device pointers / reusable buffers instead of forcing a fresh `DeviceBuffer`-owned close input path.

The launcher should be compatible with:

- a reusable resident `d_close`
- a caller-provided device `close` pointer
- a reusable or cached `d_lr`

### 2.5 Cache launch-cap metadata inside the runtime

Once the larger copy/allocation waste is removed, per-call launch metadata lookup becomes the next layer of overhead.

Cache at least:

- device launch caps
- kernel launch caps for `compute_log_returns_f32`
- kernel launch caps for `cdmc_backtest_metrics_f32`

It is okay to keep re-fetching function handles each call if that avoids lifetime complexity. The important thing is to avoid repeating the more expensive capability discovery work.

### 2.6 Strengthen input validation

Add stronger combo/window validation somewhere in the flow.

At minimum, validate cleanly that:

- `dpo_row_index[i]` is in range
- `cycle_row_index[i]` is in range
- `decycler_row_index[i]` is in range
- `start_t[i] >= 0`
- `end_t[i] >= 0`
- `start_t[i] < end_t[i]`
- `end_t[i] <= signals.cols`

You may choose whether the full O(combo_count) validation runs:

- always
- in debug builds
- in route probes / preflight checks only

Be intentional. The change is primarily for correctness and guardrails, especially once stateful caches are introduced.

### 2.7 Preserve blocker behavior for unsupported execution features

Do not change the current kernel limitation behavior for overlays / stop-loss / take-profit, except to keep it explicit and testable.

If the kernel currently rejects those configurations, keep that behavior intact and add tests for it.

### 2.8 Host readback and safe uninitialized memory

The plan allows uninitialized memory **only where it is provably safe**.

Apply these rules:

- device-only working buffers such as `d_lr`, `d_out`, `d_mama`, `d_fama` may remain uninitialized if the kernel/producer fully overwrites them before any read
- host metrics readback may use an uninitialized host buffer **only if** the copy/readback path fully overwrites every element before any access
- if a shared alloc-policy helper already exists for safe uninitialized host readback, reuse it
- otherwise, prefer a reusable `Vec<f32>` scratch buffer instead of inventing fragile new unsafe code

### 2.9 Optional second-wave notes to preserve in code comments/TODOs

If easy and local, leave design notes or TODOs for:

- possible AoS combo packing
- possible caching/compression of repeated row-index or window uploads across repeated validation folds
- optional combo sorting/bucketing for locality
- pinned host memory + async D2H only after simpler reuse wins land

Do not let these delay the first-wave changes.

---

## 3) `crates/app_core/benches/gpu_native_expansion_opt.rs`

### 3.1 Keep strong route-assertion discipline

This benchmark already does a useful route probe and asserts GPU use. Keep that discipline intact.

Do not weaken route assertions.

### 3.2 Align the CDMC benchmark request shape with validation

The plan calls out a mismatch between optimizer and validation benchmark shape.

Update `make_cdmc_req(...)` and any associated benchmark setup so that CDMC optimize and validate use the same intended MA-type shape, or explicitly provide two clearly documented benchmark variants if both shapes matter.

Preferred approach:

- choose a single aligned CDMC benchmark shape and document it
- simplest option: align to `ema/ema` if that is the validation shape already in use
- if both shapes matter, add two explicit CDMC benchmark variants rather than leaving a silent mismatch

### 3.3 Preserve CDMC benchmark comparability

After the GPU launcher/runtime refactor, keep the CDMC benchmark cases comparable across:

- small
- medium
- large

Do not change benchmark semantics casually.

### 3.4 If warm-path timing is the intended metric, keep route probes/warm-up outside timed loops

Avoid distorting Criterion measurements with extra preflight work inside the timed closure.

---

## 4) `crates/app_core/benches/composite_validation_workflow.rs`

### 4.1 Add explicit GPU assertions for validation timing samples

Current issue: route/fallback is printed but not asserted.

Fix this.

Any validation timing sample intended to represent GPU CDMC validation must assert that:

- the backend actually used GPU
- the expected route is selected
- any unexpected fallback causes the validation benchmark setup to fail rather than silently measuring CPU

### 4.2 Add a dedicated CDMC validation benchmark/profile

Do not rely only on the broad generic profile loop.

Add a dedicated CDMC validation benchmark/profile using a CDMC-shaped candle generator and explicit candidate construction.

Retain the generic loop if it is still useful, but make sure CDMC has its own direct validation evidence.

### 4.3 Align validation shape with optimizer shape

Use the same CDMC MA-type shape policy here as in `gpu_native_expansion_opt.rs`.

Do not leave optimize and validate silently incomparable.

### 4.4 Keep validation benchmark behavior deterministic and measurable

If you add warm-up/probe logic, keep it outside the timed measurement.

---

## 5) Tests you must add or update

Add or update tests so the new caching/reuse and route behavior are safe.

### 5.1 CPU-vs-GPU CDMC signal parity test

Add a parity test that compares CPU and GPU CDMC signals with sensible floating-point tolerance, covering at least:

- DPO
- correlation-cycle angle
- correlation-cycle state
- MAMA
- FAMA
- decycler
- `first_valid`

Use the existing CDMC smoke test as a starting point and turn it into a real parity test.

### 5.2 Threshold-edge desired-position tests

Add targeted tests for the cases most likely to show CPU/GPU drift or logic regressions:

- `phase_threshold` just below / at / above the gate
- `dpo_prev == 0.0` with `dpo_now > 0.0`
- `dpo_prev == 0.0` with `dpo_now < 0.0`
- `trade_on_next_bar = true`
- `allow_flip = true`

### 5.3 GPU blocker/fallback tests

Add tests that explicitly confirm the CDMC GPU kernel rejects or forces fallback for unsupported execution features exactly as intended:

- overlays
- stop-loss
- take-profit

### 5.4 Scratch-reuse repeatability test

After adding runtime scratch/state reuse, add a repeated-run test that executes the same GPU evaluation twice and asserts identical results.

This is important to catch stale-buffer and stale-cache bugs.

### 5.5 Input-validation tests for combo/windows

Add tests that invalid combo metadata is rejected cleanly:

- row indices out of range
- negative indices where invalid
- invalid `start_t` / `end_t`
- `start_t >= end_t`
- `end_t > cols`

### 5.6 Host-upload cache safety tests

If host-upload resident reuse is implemented, add tests that confirm:

- the resident host-upload cache is bypassed when no cache key is provided
- the cache refreshes or invalidates when the cache key changes
- mutating host-close contents without a matching cache key does not incorrectly reuse stale device data

### 5.7 Launch metadata cache correctness test

If launch-cap metadata is cached, add a repeated-run correctness test to ensure cached metadata does not alter output behavior.

---

## 6) Benchmark / perf verification requirements

After the code changes, run the relevant repo-appropriate formatting, tests, and benchmark checks.

At minimum, do the following if the repo supports them:

1. format the code
2. run targeted tests for the modified crate/files
3. run or at least compile the relevant benches
4. re-run the CDMC optimizer benchmark cases
5. re-run the CDMC validation benchmark cases

Use the repo’s actual commands rather than guessing if the workspace layout differs.

### Things to verify explicitly

- intended CDMC cases still route to GPU
- validation no longer silently falls back while still being timed as “GPU”
- repeated evaluation on the same candles benefits from reduced allocation/upload churn
- CPU/reference parity is preserved
- no feature regressions are introduced
- CPU-only / non-CUDA builds are not broken by missing cfg gates

### Cold vs warm timing

If the benchmark harness is intended to measure warm steady-state performance, keep cold-route probing and one-time setup outside the timed closure.

If first-hit latency matters too, do not mix it into the steady-state benchmark accidentally. Separate the measurement or at least document the distinction.

---

## 7) Optional second-wave changes (only after core changes land cleanly)

Only implement these if:

- core changes are done
- tests pass
- benchmark behavior is still trustworthy
- the change remains local and low-risk

Optional items from the addendum:

1. cache descriptor/output/param-key resolution in `cdmc.rs` with `OnceLock`
2. reduce `first_valid` scanning work in the host/reference path if deterministic warmup rules are confirmed
3. sort or bucket combos for better memory locality before kernel launch, restoring output order afterward
4. compress repeated `(start_t, end_t)` windows across repeated validation folds if it is clearly worthwhile and still local
5. introduce pinned host memory + async D2H only after simpler reuse wins are already in place
6. audit upstream `PreparedCudaIndicatorInputs` reuse if adjacent orchestration is rebuilding device inputs too often

Do **not** let these optional items derail the core implementation.

---

## 8) Implementation constraints and style requirements

- Prefer small, composable local refactors over large rewrites.
- Preserve existing public behavior unless the plan explicitly calls for a behavior change.
- If you change a struct/function signature, update all call sites and tests.
- Keep CUDA-related changes behind appropriate cfg gates.
- Be conservative with `unsafe` and document invariants immediately above any unsafe block.
- Reuse existing repo patterns for scratch growth, device residency, and benchmark route probing.
- Add comments/docstrings where needed so future maintainers understand:
  - host/reference vs device-native CDMC paths
  - cache invariants
  - why a buffer may safely remain uninitialized
  - why host-upload cache reuse requires a trusted cache key

---

## 9) Expected deliverables

When finished, provide:

1. a concise summary of what you changed, by file
2. a short explanation of how the new CDMC GPU path reduces copies/allocations
3. the list of tests added or updated
4. benchmark/perf verification notes
5. any intentionally deferred items, with reasons

If the lower-level direct MAMA `first_output_valid` surfacing is not currently possible, say so explicitly and note that you still implemented the runtime/scratch reuse so the remaining work is isolated.

---

## 10) Priority order

If you need an execution order, use this one:

### Tier 1 — mandatory first-wave work

1. `cdmc_gpu_kernel.rs`: reuse `d_close` / `d_lr` / combo buffers / `d_out` / host readback scratch
2. `cdmc.rs`: add explicit device-native CDMC prepared signals and stop relying on host-vector paths for GPU CDMC work
3. `composite_validation_workflow.rs`: add strict GPU route assertions and a dedicated CDMC validation profile
4. `cdmc.rs`: reduce MAMA prep cost with runtime/scratch reuse; remove the extra host pass only if lower-level support is available cleanly
5. `cdmc.rs`: remove unnecessary full zero-fill in `build_cdmc_desired_positions(...)`
6. `gpu_native_expansion_opt.rs`: align CDMC benchmark request shape with validation

### Tier 2 — strongly recommended follow-up in the same task if feasible

7. `cdmc_gpu_kernel.rs`: add cache-key-safe resident host-upload reuse
8. `cdmc_gpu_kernel.rs`: add combo/window validation strategy
9. `cdmc_gpu_kernel.rs`: cache device caps and kernel caps
10. `cdmc.rs`: replace repeated descriptor/key/output lookup with `OnceLock` if safe

### Tier 3 — optional extras only if still local and proven safe

11. reduce host/reference `first_valid` scanning work if deterministic
12. combo sorting/bucketing for locality
13. repeated-window compression
14. pinned host memory + async D2H
15. profiler-driven kernel-body changes

---

## Final instruction

Implement the CDMC plan comprehensively, but keep it **parity-safe, benchmark-aware, and local**. The objective is to improve performance and correctness without removing features or breaking functionality.

If a tradeoff appears, prefer the choice that:

- preserves correctness and route transparency
- reduces repeated copies and allocations on the warm path
- stays device-native for GPU CDMC work
- avoids risky speculative redesign

