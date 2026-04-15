# Second-Pass Review: `crates/app_core/src/vcet_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcet_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vcet_gpu_kernel.rs.review.md`
- First-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Second-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a sharp GPU wrapper where keepalive ownership, scratch reuse, and batch-preparation semantics dominate the risk profile.

## Stale Check

The live source still relies on:

- keepalive-backed row-pointer ownership
- uninitialized buffer reuse
- deduped key-set preparation feeding combo indices

The first-pass packet remains current.

## Second-Pass Research Log

1. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - reassess launch and buffer reuse sensitivity
   - Additional constraint informed:
     - current scratch/runtime cache behavior remains central

2. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/index.html>
   - Why consulted in second pass:
     - ground overwrite-before-read and launch-shape assumptions
   - Additional constraint informed:
     - current batch and launch logic is still sharp enough to leave untouched

3. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why consulted in second pass:
     - re-check lifetime-sensitive storage and overwrite-before-read expectations
   - Additional constraint informed:
     - keepalive fields remain semantic ownership, not dead storage

## Delta vs First Pass

### 1. The keepalive fields are even more clearly semantic ownership than the first pass implied

- Why this matters:
  - exported row-pointer tables are only valid while the device-matrix owners live

### 2. `PreparedVcetGpuKernelBatch::prepare()` coupling between deduped key sets and combo indices remains part of the batch contract

- Practical implication:
  - preserve current batch semantics unless CUDA parity and perf are revalidated together

## Verification If Changed

- `cargo test -p ta_app_core vcet_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core composite_cpu::tests::vcet_gpu_kernel_request_runs_when_cuda_available --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core composite_cpu::tests::vcet_gpu_kernel_major_parity_vs_cpu --features cuda,cuda-backtest-kernel -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve keepalive-backed pointer ownership
- Preserve scratch/runtime cache behavior
- Do not retune table building or launch logic without CUDA parity and perf coverage

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Benchmark-sensitive; do not touch yet` bucket remains correct. The second pass mainly clarifies that keepalive ownership and deduped-key-set coupling are core batch contract, not incidental implementation details.
