# Review Packet: `crates/app_core/src/asr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/asr_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Valuable ASR GPU evaluation module with strong shape validation and compaction logic, but not yet accept-as-is because device-row pointer validation is incomplete and cache invalidation currently depends on undocumented immutability assumptions`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `CUDA-runtime-critical`, `Unsafe-sensitive`, `GPU-cache-sensitive`, `Benchmark-sensitive`

## What This File Owns

`asr_gpu_kernel.rs` owns the CUDA-backed ASR metrics evaluation path once ASR signal matrices have already been prepared.

This file owns:

- public GPU input DTOs for host-row-major and device-row-pointer modes:
  - `AsrGpuSignalMatrices`: `crates/app_core/src/asr_gpu_kernel.rs:27-40`
  - `AsrGpuDeviceSignalMatrices`: `42-55`
  - `AsrGpuComboBatch`: `57-61`
- runtime scratch/cache state:
  - `AsrKernelScratch`: `67-98`
  - `AsrCudaRuntime`: `137-179`
  - thread-local runtime cache:
    - `181-183`
- upload/cache helpers:
  - `ensure_uploaded_slice`: `185-203`
  - `ensure_resident_close_lr`: `497-535`
- input validation for both GPU entry modes:
  - `validate_inputs`: `205-289`
  - `validate_inputs_device_rows`: `291-367`
- row-compaction planning for device-row mode:
  - `should_compact_rows`: `369-371`
  - `collect_used_rows_sorted`: `373-391`
  - `remap_compacted_combo_rows`: `393-416`
  - `gather_selected_row_ptrs`: `418-424`
  - `prepare_compacted_row_plan`: `426-450`
- kernel launches:
  - `launch_compute_log_returns_f32`: `452-495`
  - `launch_pack_rows_u64_ptr_f32`: `537-585`
- public evaluation entry points:
  - `eval_asr_metrics_gpu_kernel`: `602-844`
  - `eval_asr_metrics_gpu_kernel_device_rows`: `846-1367`
- local tests:
  - `1369-1470+`

This file does not own:

- ASR signal generation itself
  - that lives in `crates/app_core/src/asr.rs`
- the generic CUDA runtime primitives it reuses
  - those live in `gpu_backtest_kernel.rs` and `standalone_gpu_runtime.rs`
- the kernel PTX/CUDA source
  - that lives under `crates/app_core/src/kernels/`

## Classification

- Public surface: internal/shared backend API
- Hot path: yes
  - this is directly on the GPU validation/optimization metric path
- Runtime-critical: high
- CUDA/runtime-critical: high
- Strategy-local: yes
  - ASR-specific GPU evaluation
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- consumes ASR strategy params from:
  - `crates/app_core/src/asr.rs`
- reuses generic CUDA/runtime support from:
  - `crates/app_core/src/gpu_backtest_kernel.rs:6-10`
  - `crates/app_core/src/standalone_gpu_runtime.rs:3-7`
- is consumed by composite CPU/validation GPU routes:
  - `crates/app_core/src/composite_cpu.rs:28-29`
  - `crates/app_core/src/composite_cpu.rs:2602-2633`
  - `crates/app_core/src/composite_validate.rs:15-16`
  - `crates/app_core/src/composite_validate.rs:999-1028`
  - `crates/app_core/src/composite_validate.rs:1633-1680`
  - `crates/app_core/src/composite_validate.rs:1936-2009`

### Downstream verification impact if changed

- any change to input validation affects:
  - composite CPU GPU-eval route
  - composite validation GPU-eval route
- any change to row compaction or scratch reuse affects:
  - GPU performance characteristics
  - correctness of reused packed matrices
- any change to runtime caching affects:
  - repeated same-device evaluations within a thread

### Test anchors

- device-row validation and compaction tests:
  - `crates/app_core/src/asr_gpu_kernel.rs:1394-1469`

## Standards And Research Log

This packet was written after targeted external review focused on CUDA kernel-launch/runtime guidance, device-buffer ownership APIs, and internal Rust API contract clarity for raw-pointer GPU helpers.

### Sources consulted

1. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file is explicitly about GPU staging, reuse, and launch efficiency
   - Standard informed:
     - compacted uploads and scratch reuse are legitimate, but correctness and diagnosability must remain explicit

2. NVIDIA CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - raw device pointers are passed to kernels from this module
   - Standard informed:
     - raw device-pointer inputs need strict validation at the host boundary before launch

3. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - this file manages multiple reusable `DeviceBuffer` scratch allocations
   - Standard informed:
     - device-buffer reuse is fine, but the cache key must reflect the real data dependency

4. `cust::stream::Stream`
   - Source: <https://docs.rs/cust/latest/cust/stream/struct.Stream.html>
   - Why it was consulted:
     - all uploads and launches occur through a cached stream
   - Standard informed:
     - stream reuse is appropriate, but synchronization and ownership boundaries should stay obvious

5. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes shared internal DTOs and evaluation entry points used by multiple modules
   - Standard informed:
     - raw-pointer-bearing shared APIs should make invariants explicit rather than implicit

## Findings

## Correctness And Safety

### 1. `validate_inputs_device_rows` does not reject null fast/slow row pointers

- Evidence:
  - the device-row validator checks lengths, combo indices, window ranges, and only the Kaufman row pointers for nonzero:
    - `crates/app_core/src/asr_gpu_kernel.rs:291-367`
  - fast/slow row pointer arrays are later gathered and uploaded as-is:
    - `418-424`
    - `978-1057`
  - packed-row launches then consume those uploaded pointers without additional null checks:
    - `1059-1157`
- Why it matters:
  - a zero row pointer in `fast_squeeze_rows`, `fast_momentum_rows`, `slow_angle_rows`, `slow_er_rows`, or `slow_z_rows` can pass validation and reach the GPU pack kernel
  - that is a raw-pointer safety gap at the host boundary
- Smallest justified fix:
  - validate that every referenced fast/slow row pointer is nonzero before upload
  - add a focused test for null row pointers in the device-row path

## API And Naming

### 1. The device-row API currently relies on undocumented immutability assumptions for cached packed matrices

- Evidence:
  - row-pointer uploads are keyed by pointer signatures:
    - `crates/app_core/src/asr_gpu_kernel.rs:978-1057`
  - packed-matrix cache invalidation then keys only on `(row-pointer-signature, cols)`:
    - `1059-1157`
- Why it matters:
  - if producers ever reuse the same device row pointers while mutating the underlying row contents in place, this cache will treat the packed rows as unchanged
  - current callers likely treat these matrices as immutable, but that contract is not encoded in the API
- Smallest justified fix:
  - document that row-pointer inputs must point to immutable row contents for the lifetime of reuse
  - or add an explicit generation/snapshot identity from the producer if future mutation-in-place is expected
- Review judgment:
  - real cache-correctness contract, currently implicit

## Error Handling

### 1. Default test discovery is misleading because this file is feature-gated

- Evidence:
  - the file is gated by:
    - `#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]`
  - running `cargo test -p ta_app_core asr_gpu_kernel -- --nocapture` in this review pass matched `0` tests without those features enabled
- Why it matters:
  - it is easy to believe this module has no unit tests or that a verification command succeeded meaningfully when it did not actually exercise the local tests
- Smallest justified fix:
  - encode the feature-bearing verification command in the packet/maintainer guidance
  - consider a small non-CUDA test shim for the pure planning helpers if broader default coverage is desired

## Module Boundaries And Technical Debt

### 1. The row-major and device-row entry points duplicate a large amount of kernel-launch plumbing

- Evidence:
  - `eval_asr_metrics_gpu_kernel`: `602-844`
  - `eval_asr_metrics_gpu_kernel_device_rows`: `846-1367`
- Why it matters:
  - the duplication is understandable because the staging paths differ
  - however, it increases the risk that a bug fix lands in one path and not the other
- Smallest justified fix:
  - keep the two entry points, but factor only the truly shared launch/decode tail if a future change has to touch both anyway
- Review judgment:
  - defer broad deduplication unless the write set is already open

## Docs And Examples

### 1. The raw-pointer-bearing DTOs need stronger local contract notes

- Missing documentation topics:
  - non-null requirement for all row pointers
  - immutability expectation for cached device-row inputs
  - feature-gated test execution requirements

## Tests And Missing Verification

### 1. Existing tests cover planning logic but not the full kernel runtime

- Existing strengths:
  - out-of-range combo rows:
    - `crates/app_core/src/asr_gpu_kernel.rs:1394-1409`
  - invalid windows:
    - `1411-1435`
  - compact-row remapping:
    - `1443-1469`
- Review observation:
  - the file-local tests exist, but they are compiled only when `cuda` and `cuda-backtest-kernel` features are enabled

### 2. Important gaps remain

- Missing tests found in this review pass:
  - null fast/slow row pointer rejection
  - stale-cache behavior if row pointers are reused with changed underlying contents
  - parity between row-major and device-row entry points for the same logical inputs

### 3. Exact local verification required if changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel validate_inputs_device_rows_rejects_out_of_range_combo_rows -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel prepare_compacted_row_plan_remaps_sparse_rows -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`
- if kernel-launch or scratch reuse changes:
  - run the repo's GPU perf gate before landing

## Performance, Allocation, And CUDA Concerns

### 1. Row compaction and scratch reuse are valuable and should be preserved

- Evidence:
  - row compaction is explicitly planned through `prepare_compacted_row_plan`
  - cached scratch buffers/signatures avoid redundant uploads
- Review judgment:
  - this is the right performance posture for sparse row reuse

### 2. The compaction threshold is heuristic and should remain benchmark-driven

- Evidence:
  - `should_compact_rows` hardcodes `<= 0.85`:
    - `crates/app_core/src/asr_gpu_kernel.rs:369-371`
- Why it matters:
  - this is a throughput heuristic, not a correctness rule
- Review judgment:
  - benchmark-sensitive; do not churn without measurement

## Code-Quality Improvement Register

- Safe now:
  - validate nonzero fast/slow row pointers
  - document the packed-row cache immutability contract
  - make feature-gated test expectations explicit
- Defer:
  - broad consolidation of the two entry points
  - retuning the compaction threshold without GPU benchmarks

## Technical-Debt Reduction Register

- Safe to reduce now:
  - incomplete raw-pointer validation
  - undocumented cache invariants
- Defer because it is throughput-sensitive:
  - compaction-threshold tuning
  - larger launch-path deduplication
- Useful debt that should not be “cleaned up” casually:
  - thread-local per-device runtime reuse
  - scratch-buffer signatures and upload skipping

## Performance Opportunity Register

- Safe structural improvement:
  - none urgent beyond correctness hardening
- Benchmark-sensitive or semantics-sensitive:
  - compaction heuristic tuning
  - changes to resident close/lr cache keys
  - changing row-pack reuse policy

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve both row-major and device-row ASR GPU evaluation routes
  - preserve sparse-row compaction and runtime reuse
- Regression risk if changed carelessly:
  - raw device pointer launches can become unsafe
  - packed-row cache can become stale or over-invalidated
  - GPU throughput can regress
- Safe change posture:
  - tighten validation and document cache contracts before changing performance heuristics
