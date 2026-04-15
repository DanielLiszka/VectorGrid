# Review Packet: `crates/app_core/src/cdmc_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cdmc_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful CUDA metrics wrapper for CDMC, but not yet accept-as-is because the public safe API exposes undocumented raw device-pointer contracts, MAMA/FAMA shape assumptions are not representable in the input type, and resident close/LR cache reuse still depends on pointer identity rather than content/versioning`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `CUDA-runtime-critical`, `Public-surface`, `Pointer-contract-sensitive`, `Benchmark-sensitive`

## What This File Owns

`cdmc_gpu_kernel.rs` owns the CDMC-specific GPU metrics bridge that validates device-resident signal matrices, manages per-device CUDA runtime state, reuses scratch uploads and resident close/log-return buffers, launches the CDMC backtest metrics kernel, and decodes the resulting rows into `Metrics`.

This file owns:

- public device-input structs:
  - `CdmcGpuDeviceSignalMatrices`: `crates/app_core/src/cdmc_gpu_kernel.rs:33-49`
  - `CdmcGpuComboBatch`: `51-57`
- per-device scratch/runtime state:
  - `CdmcKernelScratch`: `60-94`
  - `CdmcCudaRuntime`: `103-136`
  - thread-local runtime cache:
    - `138-140`
- upload and validation helpers:
  - `ensure_uploaded_slice`: `151-169`
  - `validate_inputs`: `171-269`
- resident close/log-return cache:
  - `launch_compute_log_returns_f32`: `271-314`
  - `ensure_resident_close_lr`: `316-350`
- flag encoding and metric decode:
  - `cdmc_kernel_flags`: `352-364`
  - `decode_metrics_row`: `366-368`
- public GPU metrics entry point:
  - `eval_cdmc_metrics_gpu_kernel_device_rows`: `370-619`
- local unit tests:
  - `622-690`

This file does not own:

- CDMC signal generation or public strategy config semantics
  - that lives in `cdmc.rs`
- the actual CUDA kernel implementation
  - that lives in `crates/app_core/src/kernels/double_crossover.cu`
- generic close-series keying and buffer-sizing helpers
  - those live in `standalone_gpu_runtime.rs`

## Classification

- Public surface: yes
  - `lib.rs` exports `pub mod cdmc_gpu_kernel` behind CUDA feature gates
- Hot path: yes
  - this wrapper is used in GPU optimization and validation routes
- Runtime-critical: high
  - mistakes here can distort metrics or parity expectations
- CUDA/runtime-critical: high
  - this file owns context selection, scratch reuse, launch argument assembly, and stream synchronization
- Strategy-local: yes
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- consumes CDMC config semantics, especially `phase_threshold`:
  - `crates/app_core/src/cdmc.rs:30-42`
  - `crates/app_core/src/cdmc.rs:865-984`
- launches and marshals arguments for the CDMC kernel:
  - `crates/app_core/src/kernels/double_crossover.cu:2168-2355`
- depends on shared GPU runtime helpers and close-series keying:
  - `crates/app_core/src/gpu_backtest_kernel.rs:6-9`
  - `crates/app_core/src/standalone_gpu_runtime.rs:8-38`
  - `crates/app_core/src/standalone_gpu_runtime.rs:48-86`
- used by composite CPU optimization:
  - `crates/app_core/src/composite_cpu.rs:10163-10195`
- used by composite validation and advanced validation routes:
  - `crates/app_core/src/composite_validate.rs:8688-8738`
  - `crates/app_core/src/composite_validate.rs:8924-8998`
  - `crates/app_core/src/composite_validate.rs:9215-9242`
- smoke-covered indirectly from Tauri validation routing:
  - `src-tauri/src/commands_validation.rs:3527`

### Downstream verification impact if changed

- any change to pointer/shape contracts affects:
  - composite CPU GPU-kernel optimization
  - composite validation GPU-kernel evaluation
- any change to resident close/LR cache keying affects:
  - repeated validation windows and repeated optimization runs on reused datasets
- any change to phase-threshold handoff affects:
  - CPU/GPU parity against `cdmc.rs`

### Test anchors

- local tests:
  - `crates/app_core/src/cdmc_gpu_kernel.rs:622-690`
- higher-level parity tests:
  - `crates/app_core/src/composite_cpu.rs:18162-18345`
  - `crates/app_core/src/composite_validate.rs:23586-25324`

## Standards And Research Log

This packet was written after targeted external review focused on public safe API predictability for raw device-pointer interfaces, `cust` buffer/stream safety expectations, and CUDA current-device and memory-reuse behavior.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a safe public API around raw device pointers and runtime invariants
   - Standard informed:
     - public APIs should be predictable and dependable, especially when misuse can silently do the wrong thing

2. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - this file relies on uninitialized device buffers and stream-ordered device memory reuse through shared helpers
   - Standard informed:
     - uninitialized device memory is valid only when initialization and ordering assumptions are explicit and preserved

3. `cust::stream::Stream`
   - Source: <https://docs.rs/cust/latest/cust/stream/struct.Stream.html>
   - Why it was consulted:
     - this file launches kernels on a non-blocking stream and synchronizes before decoding host metrics
   - Standard informed:
     - host consumption is only sound after explicit stream synchronization of queued work

4. NVIDIA CUDA C++ Programming Guide, multi-device stream behavior
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>
   - Why it was consulted:
     - this file switches current context and launches kernels against device-associated streams
   - Standard informed:
     - current-device correctness is a real contract for stream-associated kernel launches

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file intentionally reuses scratch allocations and resident close data to avoid redundant work
   - Standard informed:
     - reuse is valuable, but cache invalidation and pointer contracts must remain explicit and correct

## Findings

## Correctness And Safety

### 1. The public input type cannot express or validate the MAMA/FAMA extent invariant even though the kernel assumes both series are length `cols`

- Evidence:
  - `CdmcGpuDeviceSignalMatrices` carries `mama_ptr` and `fama_ptr` as bare pointers with no rows, cols, or stride metadata:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:42-49`
  - Rust-side validation only checks that the two pointers are nonzero:
    - `171-189`
  - the CUDA kernel indexes both arrays as if they are flat series of length `T`:
    - `crates/app_core/src/kernels/double_crossover.cu:2225-2226`
    - `crates/app_core/src/kernels/double_crossover.cu:2279-2280`
- Why it matters:
  - the safe public wrapper cannot reject a malformed caller that provides a shorter or mismatched MAMA/FAMA allocation
  - because the invariant is not even representable in the input type, misuse crosses from “bad config” into “bad device-memory contract”
- Smallest justified fix:
  - document the exact MAMA/FAMA extent requirement immediately on the public struct and entry point
- Deferred stronger fix:
  - add explicit MAMA/FAMA shape metadata or wrap these pointers in a validated internal type
  - if external consumers are not intended, narrow visibility after compatibility review
- Review judgment:
  - real public-contract weakness; documentation is safe-now, stronger typing is a larger follow-up

### 2. Resident close/log-return cache invalidation depends on pointer identity rather than content identity

- Evidence:
  - this file keys resident close/LR reuse with `CloseSeriesKey::from_host_and_device`:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:320`
  - the shared key includes only `len`, `host_ptr`, and `device_ptr`:
    - `crates/app_core/src/standalone_gpu_runtime.rs:8-21`
  - if the key matches, the cached `ResidentCloseReturns` is reused:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:321-349`
- Why it matters:
  - if the caller mutates the close series in place or reuses the same address for new contents, cached log returns can become stale while the key still appears valid
  - this is especially important for repeated optimization and validation runs on reused buffers
- Smallest justified fix:
  - document the immutability assumption right next to the cache path
- Deferred stronger fix:
  - move to a caller-provided cache key, content signature, or explicit generation/versioning scheme
- Review judgment:
  - correctness-sensitive, but the robust fix is benchmark-sensitive

## API And Naming

### 1. The module is public, but no in-repo out-of-crate consumer was found for its raw-pointer API

- Evidence:
  - `lib.rs` exports `pub mod cdmc_gpu_kernel`:
    - `crates/app_core/src/lib.rs:8-9`
  - repo search found only internal `app_core` call sites and no external workspace consumer of:
    - `CdmcGpuDeviceSignalMatrices`
    - `CdmcGpuComboBatch`
    - `eval_cdmc_metrics_gpu_kernel_device_rows`
- Why it matters:
  - public visibility increases the burden to document and stabilize a raw device-pointer contract
  - if the wrapper is really internal infrastructure, wider visibility is unnecessary risk surface
- Smallest justified fix:
  - if semver/consumer review confirms no intended external use, consider narrowing visibility to `pub(crate)`
  - otherwise add Rustdoc that treats the API as a raw-contract boundary
- Review judgment:
  - compatibility-sensitive visibility question

## Error Handling

### 1. Host-side runtime error checking is good, but malformed pointer contracts can still fail beyond what the wrapper can presently diagnose

- Evidence:
  - runtime creation, function lookup, uploads, kernel launch, and synchronization are all checked:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:114-119`
    - `271-314`
    - `414-589`
  - however, the wrapper cannot validate MAMA/FAMA extent because it lacks metadata:
    - `33-49`
    - `171-189`
- Why it matters:
  - this file handles ordinary CUDA runtime failures responsibly
  - the weaker boundary is structural: some malformed pointer/shape inputs are impossible to reject before launch
- Smallest justified fix:
  - document the pointer/shape invariants and keep the rest of the error handling unchanged

## Module Boundaries And Technical Debt

### 1. The launch-argument ABI is manually mirrored from the CUDA kernel and is not pinned by local contract comments or checks

- Evidence:
  - the Rust wrapper builds the argument list manually:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:531-559`
  - it must match the kernel signature exactly:
    - `crates/app_core/src/kernels/double_crossover.cu:2168-2194`
- Why it matters:
  - ABI drift between Rust-side argument order and CUDA-side kernel signature is easy to introduce and hard to detect by inspection alone
  - this is the kind of maintenance hazard that benefits from very explicit locality of explanation
- Smallest justified fix:
  - add a short comment above the launch-argument block that names the matching CUDA kernel and warns that argument order is ABI-sensitive
- Review judgment:
  - safe-now documentation hardening

### 2. The phase-threshold handoff intentionally mirrors `cdmc.rs`'s absolute-value policy, so any semantic change must be coordinated

- Evidence:
  - this wrapper passes `cdmc.phase_threshold.abs()` to the kernel:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:525-526`
  - the kernel uses `fabsf(phase_threshold)`:
    - `crates/app_core/src/kernels/double_crossover.cu:2258`
  - `cdmc.rs` uses the same `abs()` normalization in CPU desired-position generation:
    - `crates/app_core/src/cdmc.rs:883`
- Why it matters:
  - this file is not the place to unilaterally change threshold semantics
  - any contract change belongs with the main `cdmc.rs` strategy review and CPU/GPU parity validation
- Review judgment:
  - semantics-sensitive; document now, coordinate later

## Docs And Examples

### 1. The public structs and entry point do not document the critical invariants that callers must satisfy

- Missing documentation topics:
  - all raw pointers must refer to valid device memory on the selected `device_id`
  - `mama_ptr` and `fama_ptr` must each address at least `cols` `f32` values
  - DPO/cycle/decycler row-major matrices must have shapes matching their declared rows and strides
  - `close_f32` and `close_device_ptr` must refer to the same logical close series
  - cache reuse assumes the close series is immutable for the lifetime of the cache key

## Tests And Missing Verification

### 1. The default filtered test command for this file exercises nothing because the entire module is feature-gated

- Verified in this review pass:
  - `cargo test -p ta_app_core cdmc_gpu_kernel -- --nocapture`
    - matched `0` tests because `cdmc_gpu_kernel.rs` is gated behind `cuda` and `cuda-backtest-kernel`

### 2. Existing local tests are narrow relative to the actual pointer contract

- Existing strengths:
  - row-index, stride, time-window, and overflow validation:
    - `crates/app_core/src/cdmc_gpu_kernel.rs:622-690`
- Missing tests found in this review pass:
  - null pointer rejection for each required signal surface
  - explicit documentation/validation of the MAMA/FAMA extent invariant
  - resident close/LR cache invalidation assumptions
  - any direct contract pinning around launch-ABI drift

### 3. Exact local verification required if changed

- if editing local validation/docs only:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_stride_smaller_than_cols -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_out_of_range_combo_rows -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_rejects_invalid_windows -- --nocapture`
- if editing launch, cache, or parity-sensitive behavior:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_gpu_kernel_major_parity_vs_cpu -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_optimization_gpu_kernel_sanity_matches_cpu_small_grid -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_validation_gpu_kernel_matches_cpu_metrics -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc_validation_gpu_kernel_start_t_matches_cpu_series -- --nocapture`
  - `cargo test -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel cross_ticker_cdmc_gpu_kernel_smoke_applies_stats -- --nocapture`

## Performance, Allocation, And CUDA Concerns

### 1. The current scratch-buffer reuse and resident-series cache are worth preserving

- Evidence:
  - this file reuses:
    - per-device runtime/module/stream state
    - resident close and log-return buffers
    - uploaded combo vectors by signature
    - output scratch buffers
- Why it matters:
  - these are the right kinds of reuse for a GPU-evaluation wrapper
  - removing them would likely regress optimization and validation throughput
- Review stance:
  - keep the reuse structure and harden the contracts instead of flattening it into per-call allocations

### 2. Stronger cache keys or richer pointer metadata would improve safety, but they are benchmark-sensitive changes

- Why it matters:
  - better cache keys or richer metadata would improve correctness and diagnosability
  - they can also add more CPU work, more copying, or larger public types on a hot path
- Review judgment:
  - benchmark-sensitive; measure before redesigning these contracts materially

## Recommended Action

### Accept as-is

- no

### Document only

- add Rustdoc for all public pointer, device, and shape invariants
- document the immutability assumption behind resident close/LR reuse
- add a short ABI-sensitivity comment above the launch-argument block

### Small surgical refactor

- add missing null-pointer tests
- if keeping the API public, tighten documentation immediately

### Larger refactor but defer

- add explicit MAMA/FAMA shape metadata or wrap the raw pointers in a validated internal type
- reconsider narrowing visibility to `pub(crate)` after compatibility review
- strengthen close/LR cache identity beyond pointer-based keying

### Benchmark-sensitive; do not touch yet

- do not remove scratch reuse or resident close/LR caching without profiling
- do not redesign cache keying or pointer metadata on the hot path without measurement

## Safe-Now Vs Defer-Later Summary

### Safe now

- document the raw pointer and cache invariants
- add direct tests around pointer nullability and current validation contracts
- add an ABI warning comment on the kernel launch argument block

### Defer until compatibility or benchmark review

- public-visibility narrowing
- richer MAMA/FAMA metadata or typed wrappers
- stronger cache-key identity for resident close/LR reuse

