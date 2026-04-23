# Review Packet: `crates/app_core/src/vemr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vemr_gpu_kernel.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful CUDA companion module, but not fully review-ready because scratch-buffer overwrite assumptions, top-k staging contracts, and runtime-cache behavior are under-documented`
- Recommended action bucket: `Benchmark-sensitive; do not touch yet`
- Sensitivity tags: `CUDA-runtime-critical`, `Benchmark-sensitive`, `Unsafe-adjacent`, `Top-k-sensitive`

## What This File Owns

`vemr_gpu_kernel.rs` owns the dedicated VEMR CUDA metric-evaluation path:

- device signal matrix and combo batch DTOs: `30-64`
- scratch-buffer and runtime cache types: `66-220`
- input validation and kernel launch helpers: `222-663`
- full-vs-top-k evaluation path: `664-1014`
- file-local tests: `1015-1079`

## Classification

- Public surface: yes
  - exported from `crates/app_core/src/lib.rs`
- Hot path: yes
- CUDA/runtime-critical: yes
- Strategy-local: yes
  - VEMR-only GPU companion

## Direct Interactions

### Backend interaction map

- main consumers:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- strategy-local staging:
  - `crates/app_core/src/vemr.rs`
- shared utilities:
  - `crates/app_core/src/alloc_policy.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`

### Downstream verification impact if changed

- changing scratch policy can affect throughput and allocation stability
- changing top-k staging can affect shortlist ordering and standalone extras
- changing validation or launch behavior can affect CPU/GPU parity and route fallback

## Standards And Research Log

This packet was written after targeted web research focused on CUDA runtime reuse, block-level sorting, and uninitialized buffer safety.

### Sources consulted

1. NVIDIA CUDA C Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file owns runtime reuse and repeated host/device transfers
   - Standard informed:
     - reuse changes should be treated as benchmark-sensitive

2. `cub::BlockRadixSort` documentation
   - Source: <https://nvidia.github.io/cccl/cub/api/classcub_1_1BlockRadixSort.html>
   - Why it was consulted:
     - the kernel path uses block-local top-k selection backed by CUB radix sort
   - Standard informed:
     - hard-coded block/thread assumptions and shared-memory use should remain explicit

3. Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - the module uses uninitialized device buffers and host buffers resized for overwrite
   - Standard informed:
     - safety assumptions need to stay adjacent to the allocation paths

## Findings

## Correctness And Safety

### 1. Scratch-buffer reuse relies on overwrite-before-read guarantees that are not explained in the file

- Evidence:
  - uninitialized device buffers are created in `ensure_len_f32` and `ensure_len_i32`: `98-122`
  - kernel/eval path reuses them across launches: `664-1014`
- Why this matters:
  - the allocations are fine only if every consumed element is always overwritten before any readback
- Smallest justified fix:
  - add comments next to the scratch helpers explaining the overwrite guarantee and expected readers

### 2. Top-k result staging is semantically important and easy to break accidentally

- Evidence:
  - `VemrGpuEvalResult` exposes both `Full` and `TopK`: `60-63`
  - top-k staging buffers and limits live at `66-90` and `664-1014`
- Why this matters:
  - consumer behavior differs depending on whether the kernel returns full metrics or shortlisted rows
- Smallest justified fix:
  - document the contract around `VEMR_DEVICE_TOPK_LIMIT` and the meaning of the `TopK` variant

## Module Boundaries And Technical Debt

### 3. Runtime cache, validation, top-k selection, and decode logic are collapsed into one module

- Evidence:
  - runtime cache types: `66-220`
  - validation: `222-351`
  - kernel launches and decode: `353-1014`
- Why this matters:
  - the file is maintainable only if readers know which sections are structural and which are algorithmic
- Smallest justified fix:
  - document section ownership now; defer extraction

## Tests And Missing Verification

### 4. File-local tests validate guards and buffer-growth behavior, but not live CUDA execution in this review pass

- Evidence:
  - tests cover row bounds, eval ranges, host buffer growth, and stride validation: `1015-1079`
- Missing coverage:
  - live kernel execution requires feature-gated composite CPU/validation tests and benches
- Smallest justified fix:
  - require CUDA parity plus bench coverage for any behavior change

## Performance And CUDA Concerns

### 5. The thread-local runtime cache and top-k kernels are explicitly measurement-sensitive

- Evidence:
  - thread-local runtime cache lives at `220`
  - block-local top-k kernels depend on CUB sort and shared memory
  - dedicated benches exist under `crates/app_core/benches/vemr_opt.rs`
- Why this matters:
  - “cleanup” here can change both latency and shortlist ordering
- Smallest justified fix:
  - keep changes limited to documentation unless perf gates are part of the work

## Code-Quality Improvement Register

- Safe now:
  - document scratch-buffer overwrite assumptions
  - document `Full` vs `TopK` result semantics
  - document runtime-cache ownership split
- Coordination required:
  - extracting runtime helpers
  - changing top-k selection or result-shaping behavior
- Defer:
  - any generic GPU abstraction rewrite

## Technical-Debt Reduction Register

- Safe now:
  - comments and rustdoc
- Defer:
  - structural extraction
  - top-k pipeline changes
- Do not casually remove:
  - thread-local runtime cache
  - top-k fast path
  - current validation guards

## Performance Opportunity Register

- Safe structural work:
  - documentation only
- Benchmark-sensitive:
  - scratch sizing policy
  - block/thread sizing
  - top-k staging and decode flow

## Recommended Action

`Benchmark-sensitive; do not touch yet`

The useful work here is making the scratch and top-k contracts explicit. Any structural change should wait for a CUDA-specific pass with parity and bench coverage.

## Exact Local Verification Required If Changed

1. File-local guard tests:
   - `cargo test -p ta_app_core vemr_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
2. Representative GPU-standalone coverage:
   - `cargo test -p ta_app_core composite_cpu::tests::vemr_gpu_kernel_supports_standalone_shortlist_extras --features cuda,cuda-backtest-kernel -- --nocapture`
   - `cargo test -p ta_app_core composite_cpu::tests::vemr_gpu_kernel_supports_standalone_confidence_rerank_without_include_all --features cuda,cuda-backtest-kernel -- --nocapture`
3. Representative bench coverage:
   - `cargo bench -p ta_app_core --bench vemr_opt --features cuda,cuda-backtest-kernel`

### Verification run for this packet

- Not run in this review pass:
  - CUDA-gated `vemr_gpu_kernel` tests
  - CUDA parity tests and benches
- Related default-build verification already passed:
  - `cargo test -p ta_app_core vemr::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Preserve the current `Full` vs `TopK` contract unless downstream shortlist consumers are reverified
- Do not change scratch allocation or runtime cache policy without CUDA measurement
- Preserve validation and decode behavior so VEMR GPU requests keep their current ranking and fallback semantics
