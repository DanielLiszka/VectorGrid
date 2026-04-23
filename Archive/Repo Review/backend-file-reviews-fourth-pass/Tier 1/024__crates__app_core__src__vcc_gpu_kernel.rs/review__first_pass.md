# Review Packet: `crates/app_core/src/vcc_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcc_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Runtime-critical CUDA module; review-useful, but not safely cleanup-ready because scratch/upload/cache contracts are mostly implicit`
- Recommended action bucket: `Benchmark-sensitive; do not touch yet`
- Sensitivity tags: `CUDA-runtime`, `Unsafe`, `Scratch-cache`, `Benchmark-sensitive`

## What This File Owns

`vcc_gpu_kernel.rs` owns VCC-specific GPU metric evaluation against precomputed device signal matrices:

- device-input and combo batch structs: `28-58`
- scratch/runtime state: `65-190`
- upload/signature helpers and input validation: `192-464`
- kernel launch/evaluation logic: `465-985`
- file-local CUDA tests: `986-1123`

## Classification

- Public surface: effectively yes within CUDA builds
- Hot path: yes
- CUDA/runtime-critical: yes
- Unsafe/FFI-adjacent: yes
  - kernel launches, raw pointer passing, buffer reuse, and `unsafe` slices

## Direct Interactions

### Backend interaction map

- strategy-local config source:
  - `crates/app_core/src/vcc.rs`
- major consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- shared runtime helpers:
  - `crates/app_core/src/standalone_gpu_runtime.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`

### Downstream verification impact if changed

- changing buffer reuse or upload-signature rules changes kernel launch cost and possibly correctness
- changing metric-row decode or detailed-mode behavior changes validation/report outputs

## Standards And Research Log

This packet was written after targeted web research focused on CUDA runtime guidance and device-buffer safety.

### Sources consulted

1. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file is a scratch-buffered GPU launch/runtime path
   - Standard informed:
     - buffer reuse and launch-shape decisions are benchmark-sensitive and should not be edited casually

2. NVIDIA CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why it was consulted:
     - the file launches kernels with raw device pointers and per-device runtime state
   - Standard informed:
     - kernel-argument and memory-lifetime assumptions should stay explicit

3. `cust::memory::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why it was consulted:
     - the module reuses `DeviceBuffer` allocations heavily
   - Standard informed:
     - scratch reuse is fine, but initialization and lifetime assumptions should be clearly justified

## Findings

## Runtime Safety And Contract Clarity

### 1. Scratch-buffer reuse and upload-signature caching are central to performance, but their contract is mostly implicit

- Evidence:
  - `VccKernelScratch` stores both device buffers and upload signatures: `65-92`
  - uploads are skipped based on signatures in `ensure_uploaded_slice`: `212-230`
- Why this matters:
  - this is likely the right optimization, but it is easy to damage accidentally during “cleanup”
- Smallest justified fix:
  - document which buffers are allowed to be stale/reused and which signatures guard correctness

### 2. The module contains multiple `unsafe` launch and pointer-manipulation sites without nearby safety commentary

- Evidence:
  - raw pointer reinterpretation in `f32_upload_signature`: `207-209`
  - kernel-launch blocks at multiple call sites such as `363`, `408`, `850`
- Why this matters:
  - these may be correct, but the safety proof is not local
  - future edits become riskier when the invariants are implicit
- Smallest justified fix:
  - add short safety comments ahead of the highest-risk `unsafe` sections

## Tests And Verification

### 3. File-local tests exist, but they are CUDA-feature-gated and were not executed in this default-build review pass

- Evidence:
  - tests exist under `986-1123`
  - the file itself is gated behind `all(feature = "cuda", feature = "cuda-backtest-kernel")`
- Why this matters:
  - review recommendations for this file should stay conservative unless CUDA verification is run in the matching build mode
- Safe-now conclusion:
  - avoid refactors here without explicit CUDA verification

## Code-Quality Improvement Register

- Safe now:
  - add safety comments
  - document scratch/upload contracts
- Defer:
  - changing scratch reuse, launch shapes, or metric decode paths

## Technical-Debt Reduction Register

- Safe now:
  - documentation/comments only
- Defer:
  - runtime restructuring
- Do not casually remove:
  - signature-based upload skips
  - resident-series reuse
  - detailed-mode output support

## Performance Opportunity Register

- Safe structural work:
  - none without CUDA verification
- Benchmark-sensitive:
  - almost everything in this file

## Recommended Action

`Benchmark-sensitive; do not touch yet`

This file is clearly performance-oriented runtime code. The only safe near-term work is localized safety/documentation hardening backed by explicit CUDA verification.

## Exact Local Verification Required If Changed

1. CUDA file-local tests:
   - `cargo test -p ta_app_core vcc_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
2. Representative composite CUDA path:
   - `cargo test -p ta_app_core composite_validate --features cuda,cuda-backtest-kernel -- --nocapture`
3. GPU perf gate when available:
   - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

### Verification run for this packet

- Not run in this review pass:
  - CUDA verification was not executed for this file

## Regression Risk And Feature-Preservation Constraints

- Do not change scratch allocation or upload-signature behavior without CUDA correctness and perf verification
- Do not simplify `unsafe` sections unless the replacement preserves kernel-argument and device-lifetime invariants exactly
