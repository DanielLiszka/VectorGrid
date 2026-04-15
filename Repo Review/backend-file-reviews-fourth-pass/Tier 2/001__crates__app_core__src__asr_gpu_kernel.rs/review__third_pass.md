# Third-Pass Review: `crates/app_core/src/asr_gpu_kernel.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/asr_gpu_kernel.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/asr_gpu_kernel.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/asr_gpu_kernel.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a strategy-local GPU wrapper where ownership and lifetime are the real contracts still looked like the right framing under fresh research.

## Third-Pass Research Log

1. CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in third pass:
     - recheck device-memory ownership, launch, and ABI assumptions in the wrapper layer
   - Third-pass constraint informed:
     - device residency and launch-shape assumptions should stay explicit and benchmark-stable

2. CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in third pass:
     - reassess whether the wrapper should move work around or keep the existing ownership model
   - Third-pass constraint informed:
     - same-device ownership, overwrite-before-read, and launch-shape stability matter more than code-style cleanup

3. Rust thread_local docs
   - Source: <https://doc.rust-lang.org/std/macro.thread_local.html>
   - Why consulted in third pass:
     - ground any cache-scope discussion that crosses Rust-side wrapper state
   - Third-pass constraint informed:
     - thread-local caches should not be overdescribed as process-global guarantees

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; the wrapper contract is still about ownership, not algorithm changes
- Why this matters:
  - The second-pass call of `Bucket unchanged` remains proportionate.
  - These files still derive most of their risk from device ownership, keepalive lifetime, and cache immutability rather than from the underlying strategy math.
- Practical implication:
  - Keep the existing bucket and preserve strategy-locality unless you are also revalidating performance and parity.

### 2. Benchmark-sensitive behavior is easier to regress than the code size suggests
- Why this matters:
  - Seemingly small wrapper changes can perturb upload reuse, launch shape, or overwrite-before-read assumptions.
- Practical implication:
  - Lead with targeted lifetime/ownership assertions and performance-sensitive tests before any wrapper cleanup.

## Implementation Priority

- Add or tighten ownership/lifetime assertions before refactoring.
- Preserve strategy-local GPU organization unless a measured cross-strategy simplification is being attempted.

## Verification If Changed

- cargo test -p ta_app_core asr_gpu_kernel -- --nocapture
- representative GPU upload/probe/parity coverage for the owning strategy family

## Regression And Feature-Preservation Constraints

- Preserve same-device ownership and keepalive expectations.
- Do not change cache or upload semantics without parity and performance review.
- Avoid genericizing strategy-local wrapper logic unless benchmarks and diagnostics are revalidated.

## Third-Pass Conclusion

`Bucket unchanged`

asr_gpu_kernel.rs stays in `Small surgical refactor`. The third pass mainly confirms that the real contract here is ownership and lifetime discipline, not broader algorithm refactor.
