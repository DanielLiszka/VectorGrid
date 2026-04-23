# Second-Pass Review: `crates/app_core/src/vcc_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vcc_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vcc_gpu_kernel.rs.review.md`
- First-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Second-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a hot CUDA kernel wrapper where scratch, upload-signature reuse, and raw-pointer assumptions should not be touched casually.

## Stale Check

The live source still owns:

- signature-based upload skipping
- resident-series reuse
- raw-pointer and reinterpret-cast assumptions

The first-pass packet remains current.

## Second-Pass Research Log

1. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep launch/runtime and resident-memory assumptions benchmark-aware
   - Additional constraint informed:
     - signature-based skipping and resident reuse remain core behavior

2. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in second pass:
     - ground raw-pointer and launch-shape assumptions
   - Additional constraint informed:
     - implicit same-device and fully-overwritten-buffer assumptions remain the real safety contract

3. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - verify reuse and lifetime expectations
   - Additional constraint informed:
     - current buffer policy remains correctness- and perf-coupled

## Delta vs First Pass

### 1. Upload-signature caching and resident-series reuse are even more central than the first pass implied

### 2. The safest near-term improvement remains commentary, not behavioral change

- Practical implication:
  - keep the do-not-touch-yet bucket

## Verification If Changed

- `cargo test -p ta_app_core vcc_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core composite_validate --features cuda,cuda-backtest-kernel -- --nocapture`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

- Preserve signature-based upload skipping, resident-series reuse, and launch behavior
- Do not touch scratch-buffer policy without CUDA correctness and perf verification
- Keep wrapper safety assumptions explicit if comments are improved

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Benchmark-sensitive; do not touch yet` bucket remains correct. The second pass mainly reinforces that upload-signature reuse and resident-series caching are central behavior, not optional micro-optimizations.
