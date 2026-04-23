# Review Packet: `crates/app_core/src/rocb_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/rocb_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally solid, but not fully review-ready because it contains multiple unsafe CUDA-memory paths, thread-local runtime caching, and several implicit kernel-support restrictions that are only partially documented in-code`
- Recommended action bucket: `Benchmark-sensitive; do not touch yet`
- Sensitivity tags: `CUDA-runtime-critical`, `Unsafe`, `Strategy-local`, `Benchmark-sensitive`, `Semantics-sensitive`

## What This File Owns

`rocb_gpu_kernel.rs` owns the ROCB-specific CUDA runtime wrapper that evaluates ROCB metrics from already-prepared device signal matrices.

It owns:

- device-facing signal and combo batch contracts:
  - `RocbGpuDeviceSignalMatrices`: `29-40`
  - `RocbGpuComboBatch`: `42-47`
- reusable CUDA scratch/runtime state:
  - `RocbKernelScratch`: `50-82`
  - `RocbCudaRuntime`: `89-123`
  - thread-local runtime map: `125-127`
- host-side validation:
  - `validate_inputs`: `129-198`
- resident close/log-return staging:
  - `launch_compute_log_returns_f32`: `200-259`
  - `ensure_resident_close_lr`: `261-294`
- kernel dispatch and result decoding:
  - `eval_rocb_metrics_gpu_kernel_device_rows`: `299-532`
- file-local validation tests:
  - `534-615`

It does not own:

- ROCB signal construction:
  - `crates/app_core/src/rocb.rs`
- generic kernel module loading and launch-cap heuristics:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- composite orchestration of ROCB batches:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`

## Classification

- Public surface: effectively yes within the CUDA-enabled backend
  - composite CPU/validation ROCB paths call the exported evaluator directly
- Hot path: yes
  - this file is on the ROCB GPU evaluation path
- CUDA/runtime-critical: yes
- Strategy-local: yes
  - ROCB-specific kernel adapter
- Low-risk support code: no
  - unsafe memory operations and runtime caching make this a high-sensitivity file

## Direct Interactions

### Backend interaction map

- ROCB config and signal prep:
  - `crates/app_core/src/rocb.rs:10`
- shared CUDA helpers:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/standalone_gpu_runtime.rs`
- composite optimization GPU ROCB path:
  - `crates/app_core/src/composite_cpu.rs:6208-6237`
- composite validation GPU ROCB paths:
  - `crates/app_core/src/composite_validate.rs:2255-2282`
  - `3164-3211`
  - `3401-3559`

### Downstream verification impact if changed

- changing input validation affects all ROCB GPU batch callers
- changing resident-series caching affects both correctness and runtime cost
- changing kernel launch parameters or scratch-buffer reuse affects throughput and possibly parity

## Standards And Research Log

This packet was written after targeted web research focused on CUDA memory-access behavior, thread-local runtime storage, and the safety contract around uninitialized device buffers.

### Sources consulted

1. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file is a host-side launch/runtime adapter for GPU kernels
   - Standard informed:
     - host-side runtime code should preserve low-overhead buffer reuse and avoid unnecessary transfers

2. NVIDIA CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - the kernel path depends on launch/configuration behavior and memory hierarchy expectations from the shared module
   - Standard informed:
     - kernel/runtime changes should respect memory-hierarchy and throughput constraints rather than chase stylistic cleanup

3. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why it was consulted:
     - the runtime cache is stored per thread via `thread_local!`
   - Standard informed:
     - thread-local mutable state is appropriate here, but mutation discipline and lifetime assumptions should be explicit

4. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - the file uses `DeviceBuffer::uninitialized` and stream-associated buffer lifetime management
   - Standard informed:
     - uninitialized device memory is valid only if every element is fully written before read

## Findings

## Correctness And Safety

### 1. Unsafe device allocation and kernel-launch paths are locally disciplined, but under-documented

- Evidence:
  - kernel launches use `unsafe` argument marshaling: `241-257`, `477-515`
  - log-return staging uses `unsafe { DeviceBuffer::<f32>::uninitialized(...) }`: `279-280`
- Why this matters:
  - the current code appears to satisfy the required invariants, but none of them are explained near the unsafe blocks
  - future edits could invalidate those assumptions during cleanup
- Smallest justified fix:
  - add short safety comments documenting why the uninitialized buffer is fully written before any read and why the kernel argument layout is valid

### 2. Strategy feature restrictions are correct but surprisingly narrow

- Evidence:
  - GPU ROCB currently rejects execution overlays, stop-loss, and take-profit: `312-319`
  - this is enforced at runtime, not by type
- Why this matters:
  - the restriction is important for parity and kernel correctness
  - callers only learn about it when they reach this layer
- Smallest justified fix:
  - keep the rejection, but document it in rustdoc and consider pushing a preflight check higher up the call stack if ROCB GPU usage expands

## API And Naming

### 3. The runtime cache contract is implicit

- Evidence:
  - `ROCB_GPU_RUNTIME` is a thread-local `HashMap<u32, RocbCudaRuntime>`: `125-127`
  - `ensure_resident_close_lr` conditionally reuses cached close/log-return buffers based on `CloseSeriesKey`: `261-294`
- Why this matters:
  - this is a good performance pattern, but the lifetime and invalidation semantics are only readable from the implementation
- Smallest justified fix:
  - add comments documenting that reuse is keyed by host/device identity and series length, and that cache entries are thread-local by design

## Error Handling

### 4. Validation is strong for geometry, but diagnostics remain kernel-centric rather than workflow-centric

- Evidence:
  - `validate_inputs` checks pointer validity, row bounds, strides, and time windows with explicit errors: `129-198`
- Why this matters:
  - this is acceptable for a strategy-local kernel adapter
  - however, if ROCB GPU adoption grows, these messages may be hard to correlate with upstream candidate context
- Smallest justified fix:
  - leave the local checks intact
  - only add richer context at the caller boundary, not inside the hot kernel adapter itself

## Module Boundaries And Technical Debt

### 5. ROCB-specific runtime code correctly reuses shared primitives, but duplicates the overall pattern seen in sibling GPU adapters

- Evidence:
  - this file reuses `standalone_gpu_runtime` helpers and `gpu_backtest_kernel` launch helpers
  - yet it still maintains its own ROCB-specific thread-local runtime, scratch state, and dispatch function
- Why this matters:
  - the duplication is understandable because each strategy has its own matrix layout and restrictions
  - broad deduplication here would be high-risk and easy to get wrong
- Recommended posture:
  - document the pattern
  - defer any abstraction attempt until multiple strategy-local GPU adapters can be unified without hiding invariants

## Docs And Examples

### 6. The file has essentially no explanatory comments despite being unsafe and runtime-critical

- Evidence:
  - there is no rustdoc on the exported evaluator or the device-input structs
  - there are no safety comments on the unsafe blocks
- Why this matters:
  - this makes later maintenance riskier than it needs to be
- Smallest justified fix:
  - add short rustdoc and safety comments without changing logic

## Tests And Missing Verification

### 7. Input-validation tests exist, but runtime-cache and parity behavior are not exercised here

- Evidence:
  - tests cover invalid time windows and row bounds: `534-615`
- Missing coverage:
  - resident-series reuse versus refresh
  - behavior when `close_device_ptr == 0` and host upload fallback is used
  - parity versus CPU ROCB metrics for a bounded candidate set
- Smallest justified fix:
  - leave broad parity testing to higher-level CUDA workflow coverage
  - add local cache-behavior tests only if the runtime reuse logic changes

## Performance, Allocation, And CUDA Concerns

### 8. This file is exactly the kind of place where clean-looking refactors can create regressions

- Evidence:
  - scratch buffers are intentionally reused: `50-82`, `337-395`
  - combo uploads are signature-gated for some inputs: `377-395`
  - resident close/log-return state is cached: `261-294`
- Why this matters:
  - these patterns are performance-motivated and consistent with CUDA best-practice guidance
  - replacing them with more abstract but less explicit code risks extra transfers, alloc/free churn, or incorrect stream ordering
- Recommended posture:
  - benchmark-sensitive; do not touch unless a bug or measured regression requires it

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for the device-input structs and exported evaluator
  - add safety comments near `unsafe` blocks
  - document the thread-local/runtime-cache policy
- Defer:
  - runtime abstraction or deduplication with sibling strategy adapters
  - launch-shape retuning without measurement

## Technical-Debt Reduction Register

- Safe now:
  - comment the unsafe invariants
  - state the explicit unsupported-strategy features
- Defer:
  - cross-strategy GPU runtime unification
- Do not casually remove:
  - scratch-buffer reuse
  - signature-based upload skipping
  - resident close/log-return caching

## Performance Opportunity Register

- Safe structural work:
  - documentation only
- Benchmark-sensitive:
  - any change to caching keys, launch shapes, scratch allocation policy, or upload-skipping logic

## Recommended Action

`Benchmark-sensitive; do not touch yet`

The current implementation is the right kind of explicit for a strategy-local GPU adapter. The useful improvement is documentation of invariants, not refactoring of the runtime structure.

## Exact Local Verification Required If Changed

1. File-local ROCB GPU validation tests:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel rocb_gpu_kernel::tests:: -- --nocapture`
2. ROCB GPU optimization path:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
3. ROCB GPU validation path:
   - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`
4. If runtime caching, upload skipping, or launch shape changes:
   - run the relevant GPU perf gate before accepting the change

### Verification run for this packet

- Not run in this pass:
  - CUDA-featured ROCB kernel coverage was not executed as part of this documentation-only review pass

## Regression Risk And Feature-Preservation Constraints

- Do not change buffer-allocation or cache-reuse policy casually; that is where throughput wins are currently preserved
- Do not widen supported strategy features at this layer unless kernel semantics are extended and validated first
- Do not abstract away the unsafe invariants unless the replacement keeps stream ordering, full-write-before-read, and pointer-lifetime guarantees equally explicit
