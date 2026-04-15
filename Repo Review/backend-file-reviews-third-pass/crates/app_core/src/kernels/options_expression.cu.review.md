# Third-Pass Review: `crates/app_core/src/kernels/options_expression.cu`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kernels/options_expression.cu`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kernels/options_expression.cu.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/options_expression.cu.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a benchmark-sensitive CUDA kernel contract surface still looked like the right framing under fresh research.

## Third-Pass Research Log

1. CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in third pass:
     - reconfirm launch, ABI, and memory-shape assumptions at the kernel boundary
   - Third-pass constraint informed:
     - entry names, argument layout, and numeric conventions must stay stable unless the paired Rust owner moves with them

2. CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in third pass:
     - recheck whether the right third-pass guidance is redesign or extra validation
   - Third-pass constraint informed:
     - for hot kernels, additional validation and probes are usually safer than structural churn

3. Rust Reference type layout
   - Source: <https://doc.rust-lang.org/reference/type-layout.html>
   - Why consulted in third pass:
     - ground the paired Rust/CUDA ABI discussion for kernel-owner files
   - Third-pass constraint informed:
     - cross-language layout assumptions need explicit validation rather than hand-waved compatibility

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; the kernel remains more dangerous to touch than it looks
- Why this matters:
  - The second-pass call of `Bucket unchanged` still fits.
  - The compatibility-sensitive parts remain entry names, numeric encodings, and memory-shape expectations.
- Practical implication:
  - Keep the current bucket and prefer probe/assert coverage over redesign unless paired kernel-owner changes are being benchmarked together.

### 2. Validation coverage is higher ROI than cosmetic kernel cleanup
- Why this matters:
  - Kernel edits can perturb performance or ABI without obvious source-level signal.
- Practical implication:
  - If touched, add validation around launch and output invariants first.

## Implementation Priority

- Extend validation/probe coverage before changing kernel structure.
- Preserve paired Rust/CUDA contract assumptions unless both sides move together.

## Verification If Changed

- cargo test -p ta_app_core gpu_backtest_kernel -- --nocapture
- representative GPU parity and performance-sensitive smoke for the owning kernel path

## Regression And Feature-Preservation Constraints

- Preserve kernel entry names, numeric conventions, and memory-shape expectations.
- Do not redesign hot kernels without paired ABI validation and benchmark review.
- Prefer extra probes/assertions to structural churn for this layer.

## Third-Pass Conclusion

`Bucket unchanged`

options_expression.cu stays in `Small surgical refactor`. The third pass mostly confirms that stronger validation is safer than kernel redesign for this file.
