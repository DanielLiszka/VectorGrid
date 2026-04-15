# Second-Pass Review: `crates/app_core/src/gpu_backtest_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/gpu_backtest_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/gpu_backtest_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as one of the most performance- and contract-sensitive CUDA runtime owners in the repo, with handle lifetimes, scratch retention, and runtime reuse all demanding caution.

## Stale Check

The live source still owns:

- type-erased probe/runtime preparation
- thread-local runtime and scratch state
- pinned-memory staging and kernel launch topology

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground public helper contract requirements
   - Additional constraint informed:
     - `prepare_cuda_kernel_runtime_for_probe()` still behaves like a real public handle API

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - reassess runtime/scratch affinity and ownership semantics
   - Additional constraint informed:
     - same-thread/runtime-affinity should remain explicit if the public probe handle survives

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep pinned-buffer and stream reuse changes benchmark-aware
   - Additional constraint informed:
     - current reuse topology remains worth preserving

4. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in second pass:
     - ground stream/runtime-handle sharp edges
   - Additional constraint informed:
     - public type-erased runtime handles are still unusually sharp

## Delta vs First Pass

### 1. `prepare_cuda_kernel_runtime_for_probe()` is more than a "sharp helper"; it is effectively a public runtime-handle API

- Why this matters:
  - callers can easily miss same-thread and runtime-affinity expectations
- Practical implication:
  - documentation or narrowing visibility still looks like the safest bounded improvement

### 2. `DYNAMIC_GPU_METRICS_SCRATCH` remains effectively unbounded by output shape

- Why this matters:
  - retention pressure is still a real operational risk, not a hypothetical one
- Practical implication:
  - keep memory/scratch changes measurement-backed

## Verification If Changed

- rerun existing CUDA uploaded-batch, staged-market, and exact-kernel tests
- run the GPU perf gate for runtime-path changes

## Regression And Feature-Preservation Constraints

- Preserve pinned-memory staging, stream topology, and launch heuristics unless benchmarked
- Preserve current runtime reuse behavior
- Do not weaken scratch or fallback observability while refactoring handle surfaces

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that the probe-runtime helper is still effectively a public handle API, and that scratch retention remains a real runtime-behavior concern.
