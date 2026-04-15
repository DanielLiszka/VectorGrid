# Review Packet: `crates/app_core/src/carm_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/carm_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful CUDA runtime wrapper for CARM metrics, but not yet accept-as-is because one kernel-shape invariant is not validated on the Rust side, the public API exposes raw device-pointer contracts without documentation, and resident close/LR cache reuse currently depends on pointer identity rather than content/versioning`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `CUDA-runtime-critical`, `Public-surface`, `Pointer-contract-sensitive`, `Benchmark-sensitive`

## What This File Owns

`carm_gpu_kernel.rs` owns the CARM-specific GPU metrics wrapper that validates device-resident signal matrices, manages a per-device CUDA runtime cache, launches the CARM backtest metrics kernel, and decodes the resulting metric rows back into `Metrics`.

This file owns:

- public device-input structs:
  - `CarmGpuDeviceSignalMatrices`: `crates/app_core/src/carm_gpu_kernel.rs:33-49`
  - `CarmGpuComboBatch`: `51-56`
- per-device scratch/runtime state:
  - `CarmKernelScratch`: `59-91`
  - `CarmCudaRuntime`: `100-133`
  - thread-local runtime cache:
    - `135-137`
- device upload cache helper:
  - `ensure_uploaded_slice`: `148-166`
- Rust-side input validation:
  - `validate_inputs`: `168-256`
- resident close/log-return cache maintenance:
  - `launch_compute_log_returns_f32`: `258-301`
  - `ensure_resident_close_lr`: `303-338`
- strategy flag encoding and metric decoding:
  - `carm_kernel_flags`: `340-352`
  - `decode_metrics_row`: `354-356`
- public GPU metrics entry point:
  - `eval_carm_metrics_gpu_kernel_device_rows`: `358-585`
- local unit tests:
  - `588-651`

This file does not own:

- CARM signal generation or CPU desired-position semantics
  - that lives in `carm.rs`
- the CUDA kernel implementation itself
  - that lives in `crates/app_core/src/kernels/double_crossover.cu`
- generic device-buffer sizing helpers and close-series key definitions
  - those live in `standalone_gpu_runtime.rs`

## Classification

- Public surface: yes
  - `lib.rs` exports `pub mod carm_gpu_kernel` behind CUDA feature gates:
    - `crates/app_core/src/lib.rs:5-6`
- Hot path: yes
  - this function is used in optimization and validation GPU routes
- Runtime-critical: high
  - mistakes here can silently corrupt candidate metrics or invalidate CPU/GPU parity conclusions
- CUDA/runtime-critical: high
  - this file owns context selection, stream usage, scratch reuse, and launch argument assembly
- Strategy-local: yes
  - it is the CARM-specific GPU wrapper rather than generic kernel infrastructure
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- consumes the public CARM config contract:
  - `crates/app_core/src/carm.rs:19-40`
- delegates kernel/module utilities to shared GPU runtime helpers:
  - `crates/app_core/src/gpu_backtest_kernel.rs:6-9`
- depends on shared close-series caching and device-buffer helpers:
  - `crates/app_core/src/standalone_gpu_runtime.rs:6-38`
  - `crates/app_core/src/standalone_gpu_runtime.rs:48-86`
- launches the CARM kernel implemented in:
  - `crates/app_core/src/kernels/double_crossover.cu:2840-3045`
- called from composite CPU optimization:
  - `crates/app_core/src/composite_cpu.rs:42-43`
  - `crates/app_core/src/composite_cpu.rs:8731-8769`
- called from composite validation and advanced validation routes:
  - `crates/app_core/src/composite_validate.rs:27-28`
  - `crates/app_core/src/composite_validate.rs:7343-7376`
  - `crates/app_core/src/composite_validate.rs:7664-7744`

### Downstream verification impact if changed

- any change to input validation affects:
  - composite CPU GPU-kernel optimization route
  - composite validation GPU-kernel route
- any change to resident close/LR cache keying affects:
  - repeated GPU runs on reused datasets
  - potential parity debugging between repeated validation windows
- any change to public struct visibility or contract affects:
  - all internal GPU-call sites
  - any out-of-crate callers using the CUDA feature-gated public module

### Test anchors

- local tests:
  - `crates/app_core/src/carm_gpu_kernel.rs:588-651`
- higher-level parity tests:
  - `crates/app_core/src/composite_cpu.rs:17141-17492`
  - `crates/app_core/src/composite_validate.rs:22878-25003`

## Standards And Research Log

This packet was written after targeted external review focused on safe public API predictability for raw-pointer-adjacent interfaces, CUDA current-device/stream behavior, and device-buffer/stream synchronization expectations in `cust`.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a public safe API around raw device pointers and runtime-specific invariants
   - Standard informed:
     - public APIs should be predictable and dependable, especially when misuse can silently do the wrong thing

2. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - this file relies on uninitialized device buffers and stream-ordered async buffer management through shared helpers
   - Standard informed:
     - uninitialized device memory and async deallocation are valid only when stream-ordering and initialization assumptions are preserved

3. `cust::stream::Stream`
   - Source: <https://docs.rs/cust/latest/cust/stream/struct.Stream.html>
   - Why it was consulted:
     - this file creates a non-blocking stream, launches kernels, and synchronizes before host decode
   - Standard informed:
     - stream synchronization is the boundary that makes previously queued work safe to consume on the host

4. NVIDIA CUDA C++ Programming Guide, multi-device stream behavior
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>
   - Why it was consulted:
     - this file sets current context and launches kernels on per-device streams
   - Standard informed:
     - kernels and streams are device-associated, so current-context correctness is a real contract, not incidental glue

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file explicitly tries to reuse resident device data and scratch buffers to avoid redundant work
   - Standard informed:
     - avoiding redundant transfers and preserving efficient memory reuse is valuable, but cache invalidation must still remain correct

## Findings

## Correctness And Safety

### 1. Rust-side validation misses the kernel's `mixed_row = idx` invariant, so malformed calls can degrade into zero metrics instead of an error

- Evidence:
  - the Rust validator checks `fast_row_index`, `otto_row_index`, and time windows, but never checks combo count against `mixed_rows`:
    - `crates/app_core/src/carm_gpu_kernel.rs:168-256`
  - the CUDA kernel derives `mixed_row` from the combo index, not from an explicit combo field:
    - `crates/app_core/src/kernels/double_crossover.cu:2868-2870`
  - the kernel guards `mixed_row >= mixed_rows` by zero-filling the output row and returning:
    - `crates/app_core/src/kernels/double_crossover.cu:2877-2901`
  - the Rust wrapper then decodes every returned row as ordinary metrics:
    - `crates/app_core/src/carm_gpu_kernel.rs:577-583`
- Why it matters:
  - the safe wrapper currently allows a caller to pass more combos than mixed-signal rows
  - in that case the kernel returns zero metrics for out-of-range combos instead of surfacing a contract violation
  - because this file exposes a safe public API, that mismatch should be rejected before launch rather than converted into plausible-looking output
- Smallest justified fix:
  - require `combos.fast_row_index.len() <= signals.mixed_rows` in `validate_inputs`
  - add a focused unit test for combo-count-versus-mixed-row mismatch
- Review judgment:
  - safe-now change

## API And Naming

### 1. The public safe API exposes raw device-pointer fields and device-association invariants without documenting them

- Evidence:
  - `CarmGpuDeviceSignalMatrices` exposes raw `u64` device pointers for all signal matrices:
    - `crates/app_core/src/carm_gpu_kernel.rs:33-49`
  - `eval_carm_metrics_gpu_kernel_device_rows` is a safe public function taking those structs directly:
    - `358-364`
  - `lib.rs` exports the entire module publicly under CUDA feature gates:
    - `crates/app_core/src/lib.rs:5-6`
- Why it matters:
  - callers must already know that:
    - pointers must be valid device pointers
    - pointers must belong to the selected `device_id`
    - row-major shapes and strides must match kernel expectations
    - the close slice and optional device pointer must refer to the same logical series
  - none of that contract is expressed in doc comments or type distinctions
- Smallest justified fix:
  - add explicit doc comments to the two public structs and the public entry point describing pointer validity, device association, and shape invariants
- Deferred compatibility review:
  - if there are no intended external consumers, consider narrowing visibility to `pub(crate)` or wrapping raw pointers in a validated internal type
- Review judgment:
  - objective public-contract debt; documentation is safe-now, visibility tightening is compatibility-sensitive

## Error Handling

### 1. The kernel has an internal invalid-row zero-fill fallback, but the Rust wrapper does not treat that path as an error surface

- Evidence:
  - invalid kernel inputs inside the device function write zeros instead of propagating an error:
    - `crates/app_core/src/kernels/double_crossover.cu:2877-2901`
  - the wrapper always decodes returned rows as `Metrics` without checking for a sentinel or kernel-side error marker:
    - `crates/app_core/src/carm_gpu_kernel.rs:577-583`
- Why it matters:
  - once the Rust-side validator misses an invariant, the failure mode is silent-looking low metrics rather than a rejected call
  - that is especially dangerous in ranking/optimization code, where zero metrics can look like a legitimate but weak candidate
- Smallest justified fix:
  - fix the Rust-side validation gap first
  - do not add device-side sentinel decoding unless a broader kernel error-signaling policy is adopted across GPU wrappers
- Review judgment:
  - safe-now to harden via validation; larger kernel error-policy changes should stay coordinated

## Module Boundaries And Technical Debt

### 1. Resident close/log-return cache invalidation depends on pointer identity, which assumes the underlying series is immutable for the cache lifetime

- Evidence:
  - this file keys resident close/LR reuse through `CloseSeriesKey::from_host_and_device`:
    - `crates/app_core/src/carm_gpu_kernel.rs:307`
  - the shared key includes only `len`, `host_ptr`, and `device_ptr`:
    - `crates/app_core/src/standalone_gpu_runtime.rs:8-21`
  - if the key matches, this file reuses the cached `ResidentCloseReturns`:
    - `crates/app_core/src/carm_gpu_kernel.rs:308-337`
- Why it matters:
  - pointer identity is not the same as content identity
  - if a host or device buffer is mutated in place, or an address is reused for different contents across calls, the cached log returns can go stale while still appearing valid to the key
- Smallest justified fix:
  - document the immutability assumption immediately next to the cache path
- Deferred stronger fix:
  - move to a caller-provided cache key, content signature, or explicit generation/versioning scheme
- Review judgment:
  - real cache-correctness risk, but the robust fix is benchmark-sensitive because content hashing or stronger keying adds overhead

## Docs And Examples

### 1. There are no public docs explaining the required shape and device invariants for this CUDA wrapper

- Missing documentation topics:
  - what `mixed_rows`, `fast_rows`, `otto_rows`, and their strides mean
  - that combo count is coupled to mixed-row indexing in the current kernel contract
  - when `close_device_ptr` may be zero and when it must refer to the same logical series as `close_f32`
  - that the function currently supports no execution overlays, stop-loss, or take-profit:
    - `crates/app_core/src/carm_gpu_kernel.rs:369-376`
- Smallest justified fix:
  - add concise Rustdoc to the public structs and entry point

## Tests And Missing Verification

### 1. The default filtered test command for this file exercises nothing because the whole module is feature-gated

- Verified in this review pass:
  - `cargo test -p ta_app_core carm_gpu_kernel -- --nocapture`
    - matched `0` tests because `carm_gpu_kernel.rs` is gated behind `cuda` and `cuda-backtest-kernel`
- Why it matters:
  - it is easy to think the file has been tested when the filtered default run did not compile or execute its unit tests at all

### 2. Local unit coverage is narrow relative to the wrapper's actual contract

- Existing strengths:
  - basic validation coverage for stride, row bounds, invalid windows, and i32 overflow:
    - `crates/app_core/src/carm_gpu_kernel.rs:588-651`
- Missing tests found in this review pass:
  - combo-count greater than `mixed_rows`
  - public contract documentation tests or compile-time examples for shape invariants
  - resident close/LR cache invalidation assumptions
  - end-to-end mismatch behavior if wrong-device or mismatched logical-series inputs are provided

### 3. Exact local verification required if changed

- if editing this file only at the Rust-contract level:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_stride_smaller_than_cols -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_out_of_range_combo_rows -- --nocapture`
  - add a new mixed-row mismatch test and run it directly
- if editing launch, cache, or parity-sensitive behavior:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_gpu_kernel_major_parity_vs_cpu -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_gpu_kernel_strategy_flag_parity_cases -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm_validation_gpu_kernel_start_t_matches_cpu_series -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. The resident-series cache and scratch-buffer reuse are valuable and should not be flattened into per-call reallocation

- Evidence:
  - this file caches:
    - per-device runtime/module/stream state
    - resident close and log-return buffers
    - uploaded combo vectors by signature
    - output scratch buffers
- Why it matters:
  - these reuse paths are aligned with CUDA best-practice goals around avoiding redundant transfers and allocations
  - removing them would likely regress optimization and validation throughput
- Review stance:
  - keep the reuse structure
  - harden its invariants rather than simplifying it away

### 2. Stronger cache keys are desirable, but changing them is benchmark-sensitive

- Evidence:
  - current close/LR cache reuse is pointer-based:
    - `crates/app_core/src/standalone_gpu_runtime.rs:8-21`
  - uploaded combo reuse uses a full slice signature:
    - `crates/app_core/src/carm_gpu_kernel.rs:148-166`
- Why it matters:
  - moving close-series reuse to content hashing or stronger versioning would improve correctness
  - it could also add measurable overhead on large series if done naively
- Review judgment:
  - benchmark-sensitive; do not redesign cache keying without measuring representative workloads

## Recommended Action

### Accept as-is

- no

### Document only

- document the public pointer/shape/device invariants on `CarmGpuDeviceSignalMatrices`, `CarmGpuComboBatch`, and `eval_carm_metrics_gpu_kernel_device_rows`
- document the resident close/LR cache immutability assumption

### Small surgical refactor

- add Rust-side validation that combo count does not exceed `mixed_rows`
- add a direct unit test covering that invariant

### Larger refactor but defer

- wrap raw device pointers in a validated internal type or narrow visibility to `pub(crate)` after compatibility review
- redesign resident close/LR cache keying around a stronger logical-series identity

### Benchmark-sensitive; do not touch yet

- do not remove scratch-buffer reuse, resident close/LR caching, or uploaded-combo signature reuse without profiling
- do not add heavy content hashing to the hot path without measuring the tradeoff

## Safe-Now Vs Defer-Later Summary

### Safe now

- validate combo count versus `mixed_rows`
- add the missing unit test for that contract
- add public Rustdoc explaining the raw-pointer, device, and shape invariants

### Defer until compatibility or benchmark review

- visibility narrowing for the public CUDA wrapper API
- redesign of close/LR cache identity beyond pointer-based keying

