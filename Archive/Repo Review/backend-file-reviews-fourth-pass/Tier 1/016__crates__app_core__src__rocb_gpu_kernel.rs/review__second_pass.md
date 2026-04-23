# Second-Pass Review: `crates/app_core/src/rocb_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/rocb_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/rocb_gpu_kernel.rs.review.md`
- First-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Second-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a hot CUDA adapter where runtime reuse and scratch policy dominate the risk profile.

## Stale Check

The live source still uses:

- thread-local runtime ownership
- scratch-buffer reuse
- upload-signature skipping and resident close/LR cache reuse

The first-pass packet remains current.

## Second-Pass Research Log

1. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - confirm uninitialized allocation and async-drop semantics
   - Additional constraint informed:
     - overwrite-before-read remains the live safety contract

2. `cust::DeviceCopy`
   - Source: <https://docs.rs/cust/latest/cust/memory/trait.DeviceCopy.html>
   - Why consulted in second pass:
     - ground the buffer layout and transfer expectations
   - Additional constraint informed:
     - current wrapper assumptions remain low-level and benchmark-sensitive

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep reuse and transfer patterns performance-aware
   - Additional constraint informed:
     - scratch-buffer reuse and signature skipping remain central rather than optional

## Delta vs First Pass

### 1. Thread-local runtime ownership is the right shape for this adapter and should stay strategy-local

### 2. The safe near-term improvement remains comments and contract clarity, not behavior change

- Practical implication:
  - keep this file in the "do not touch yet" bucket

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel rocb_gpu_kernel::tests:: -- --nocapture`
- ROCB-related composite GPU paths

## Regression And Feature-Preservation Constraints

- Keep scratch-buffer reuse
- Keep thread-local runtime ownership
- Preserve host-upload fallback and resident close/log-return cache semantics

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Benchmark-sensitive; do not touch yet` bucket remains correct. The second pass mainly reinforces that the active contracts are still scratch reuse, thread-local runtime ownership, and overwrite-before-read safety.
