# Second-Pass Review: `crates/app_core/src/kernels/double_crossover.cu`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kernels/double_crossover.cu`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kernels/double_crossover.cu.review.md`
- First-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Second-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this translation unit as one of the most sensitive files in the repo because Rust/CUDA ABI agreement, kernel naming, and top-k behavior all converge here.

## Stale Check

The live source still serves as:

- the compiled CUDA translation unit for the shared GPU backtest path
- the owner of shortlist-critical top-k behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/archive/12.6.2/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep launch and tuning sensitivity explicit
   - Additional constraint informed:
     - broad cleanup here remains riskier than commentary or measurement-backed change

2. CUB `BlockRadixSort`
   - Source: <https://nvidia.github.io/cccl/cub/api/classcub_1_1BlockRadixSort.html>
   - Why consulted in second pass:
     - ground the shortlist-critical top-k region
   - Additional constraint informed:
     - the top-k section remains especially tuning-sensitive

3. local `crates/app_core/build.rs`
   - Why consulted in second pass:
     - confirm the actual compilation and symbol-validation behavior
   - Additional constraint informed:
     - only a fixed subset of required PTX symbols is validated at build time

## Delta vs First Pass

### 1. This file is even more central than the first pass implied because build-time validation only checks a subset of symbol drift

- Why this matters:
  - ABI drift or kernel-name drift outside the checked symbol list can still survive until runtime launch failure
- Practical implication:
  - keep the do-not-touch-yet bucket

## Verification If Changed

- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`
- representative strategy-family CUDA tests
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

- Keep Rust/CUDA ABI layouts and kernel entry names aligned
- Preserve current top-k behavior and launch-shape assumptions unless GPU parity and perf are revalidated together
- Do not "simplify" the top-k region without measurement

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Benchmark-sensitive; do not touch yet` bucket remains correct. The second pass mainly clarifies that build-time symbol validation only covers part of the true ABI surface, which makes casual kernel drift riskier than the first pass already suggested.
