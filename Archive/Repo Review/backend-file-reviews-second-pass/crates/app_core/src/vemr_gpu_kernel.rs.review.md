# Second-Pass Review: `crates/app_core/src/vemr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/vemr_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vemr_gpu_kernel.rs.review.md`
- First-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Second-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a hot CUDA shortlist/runtime wrapper where sparse top-k behavior, scratch reuse, and raw-pointer assumptions should not be touched casually.

## Stale Check

The live source still relies on:

- uninitialized scratch and host buffers
- resident close/LR cache reuse
- paired `close_f32` and `close_device_ptr` inputs

The first-pass packet remains current.

## Second-Pass Research Log

1. `MaybeUninit`
   - Source: <https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>
   - Why consulted in second pass:
     - re-check uninitialized-value UB boundaries for host scratch handling
   - Additional constraint informed:
     - every later-read element must still be fully overwritten first

2. `cust::DeviceBuffer`
   - Source: <https://docs.rs/cust/latest/cust/memory/struct.DeviceBuffer.html>
   - Why consulted in second pass:
     - verify device-buffer allocation semantics
   - Additional constraint informed:
     - uninitialized device allocations remain sound only under full-overwrite discipline

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/archive/12.6.2/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep reuse and transfer-sensitive policy benchmark-aware
   - Additional constraint informed:
     - current sparse top-k flow and runtime reuse remain worth preserving

4. CUB `BlockRadixSort`
   - Source: <https://nvidia.github.io/cccl/cub/api/classcub_1_1BlockRadixSort.html>
   - Why consulted in second pass:
     - ground shortlist-critical top-k implementation sensitivity
   - Additional constraint informed:
     - the top-k path is still a tuning-sensitive core contract

## Delta vs First Pass

### 1. The uninitialized scratch contract is even stricter than the first pass implied

- Why this matters:
  - both host-side and device-side uninitialized buffers are only sound if every returned element is fully overwritten before decode

### 2. Resident-series cache correctness still assumes input immutability for a given key

- Why this matters:
  - in-place mutation of the same series can still reuse stale cached state

## Verification If Changed

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel vemr_gpu_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`
- rerun the GPU perf gate if cache or scratch policy changes

## Regression And Feature-Preservation Constraints

- Keep current sparse top-k flow and runtime reuse
- Do not replace scratch reuse with blanket initialization unless GPU perf gates justify it
- Preserve current `Full` vs `TopK` result semantics

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Benchmark-sensitive; do not touch yet` bucket remains correct. The second pass mainly clarifies that overwrite-before-read discipline remains the live safety contract for both host and device scratch paths, and that resident-series reuse still depends on immutable inputs.
