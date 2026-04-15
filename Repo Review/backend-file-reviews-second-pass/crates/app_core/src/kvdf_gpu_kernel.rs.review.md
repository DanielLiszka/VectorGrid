# Second-Pass Review: `crates/app_core/src/kvdf_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kvdf_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kvdf_gpu_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this wrapper as the KVDF twin of the KQSR GPU wrapper: reuse is valuable, but the safe-looking public API still hides raw-pointer and cache-invariance contracts.

## Stale Check

The live source still owns:

- thread-local runtime reuse
- resident-series caching
- raw-pointer transport with host-upload fallback

The first-pass packet remains current.

## Second-Pass Research Log

1. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why consulted in second pass:
     - ground safe-wrapper expectations around raw pointer behavior
   - Additional constraint informed:
     - the public contract is still sharper than the API shape suggests

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - re-check per-thread runtime and cache semantics
   - Additional constraint informed:
     - current thread-local reuse remains a deliberate performance contract

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep runtime reuse and transfer topology changes benchmark-aware
   - Additional constraint informed:
     - reuse and resident-series caching should remain intact unless measured otherwise

4. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in second pass:
     - ground stream and launch synchronization assumptions
   - Additional constraint informed:
     - explicit synchronization remains the correct host-read boundary

## Delta vs First Pass

### 1. `close_device_ptr == 0` is still an explicit public behavior switch into host-upload fallback

- Why this matters:
  - it is part of the API contract, not just an implementation detail

### 2. Resident-series cache correctness still assumes immutability for a given key

- Practical implication:
  - preserve reuse, but keep the immutability contract explicit if the wrapper stays public

## Verification If Changed

- existing KVDF GPU validation, CSCV, reopt, and optimization parity tests with CUDA features enabled

## Regression And Feature-Preservation Constraints

- Preserve runtime caching and resident close/log-return reuse
- Preserve current `f32` GPU path
- Do not remove reuse or widen the raw-pointer contract without profiling and documentation

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that host-upload fallback and input immutability are active API semantics, not incidental implementation detail.
