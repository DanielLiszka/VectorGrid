# Second-Pass Review: `crates/app_core/src/kqsr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kqsr_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kqsr_gpu_kernel.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this wrapper as a safe public API over raw pointer and thread-local runtime contracts, with reuse worth preserving but narrowing visibility remaining attractive.

## Stale Check

The live source still owns:

- thread-local runtime reuse
- resident close/log-return caches
- raw-pointer transport hidden behind a safe-looking API

The first-pass packet remains current.

## Second-Pass Research Log

1. The Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why consulted in second pass:
     - re-check expectations for safe wrappers over raw-pointer contracts
   - Additional constraint informed:
     - documentation or `pub(crate)` narrowing remains the cleanest hardening if no external consumer exists

2. `thread_local!`
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in second pass:
     - ground runtime cache affinity review
   - Additional constraint informed:
     - current thread-local reuse is still a deliberate performance contract

3. CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in second pass:
     - keep runtime reuse and transfer topology changes benchmark-aware
   - Additional constraint informed:
     - validation-path cost is still dominated by sync/copy/decode tail, not setup

4. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in second pass:
     - ground stream and launch contract sensitivity
   - Additional constraint informed:
     - the current synchronization and launch structure is still the right boundary to preserve

## Delta vs First Pass

### 1. Narrowing visibility to `pub(crate)` still looks like the cleanest hardening move if no external consumers exist

- Why this matters:
  - the raw-pointer contract remains the dominant risk, not the kernel behavior itself

### 2. Cache correctness still assumes input immutability for a given resident-series key

- Practical implication:
  - preserve reuse, but keep the immutability contract explicit if the API remains public

## Verification If Changed

- existing KQSR GPU validation and optimization parity tests with CUDA features enabled

## Regression And Feature-Preservation Constraints

- Preserve thread-local runtime reuse
- Preserve resident close/log-return caching
- Preserve current validation-path performance posture unless measured otherwise

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly reinforces that visibility/documentation hardening remains a better target than runtime-topology churn.
