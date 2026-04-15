# Third-Pass Review: `crates/app_core/src/kernels/double_crossover.cu`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kernels/double_crossover.cu`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kernels/double_crossover.cu.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kernels/double_crossover.cu.review.md`
- Second-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Third-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the CUDA half of the dynamic-strategy ABI contract still looked like the right framing under fresh research. The third pass kept the bucket but sharpened the primary risk from symbol drift toward layout and encoding drift across the Rust and CUDA boundary.

## Third-Pass Research Log

1. CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in third pass:
     - reconfirm the kernel-side ABI and argument-layout expectations
   - Third-pass constraint informed:
     - kernel entry points that mirror host-side ABI should validate layout and encoding agreement explicitly

2. CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why consulted in third pass:
     - recheck whether the safest third-pass guidance is redesign or extra validation
   - Third-pass constraint informed:
     - for hot kernels, validation and probe coverage are safer first moves than structural churn

3. Rust Reference type layout
   - Source: <https://doc.rust-lang.org/reference/type-layout.html>
   - Why consulted in third pass:
     - ground the paired host and kernel layout discussion from the Rust side
   - Third-pass constraint informed:
     - cross-language ABI assumptions need explicit checks, not only partial probes

## What Third Pass Sharpened

### 1. Layout and numeric encoding drift are sharper risks than symbol drift alone
- Evidence:
  - The kernel relies on parallel hand-written ABI struct definitions and numeric encodings shared with Rust.
  - The current probe only samples metadata and a narrow checksum surface.
- Why this matters:
  - Some layout or encoding drift can survive until runtime misbehavior even if symbols and coarse metadata still line up.
- Practical implication:
  - Keep the existing bucket and treat explicit ABI validation as the safer high-ROI hardening step.

## Implementation Priority

- Add stronger ABI and layout validation before touching kernel structure.
- Keep any future kernel cleanup paired with measured probe and parity coverage.

## Verification If Changed

- cargo test -p ta_app_core strategy_gpu_program -- --nocapture
- representative kernel probe and parity coverage for the dynamic-strategy path

## Regression And Feature-Preservation Constraints

- Preserve kernel entry names, numeric encodings, and paired Rust ABI assumptions.
- Do not redesign the kernel without validating the Rust and CUDA boundary together.
- Favor extra probes and assertions over structural churn for this layer.

## Third-Pass Conclusion

`Bucket unchanged, but ABI drift sharpened`

double_crossover.cu stays in Benchmark-sensitive; do not touch yet. The third pass mainly tightens the main risk from symbol drift to layout and encoding drift across the Rust and CUDA ABI boundary, which is the safer place to harden first.
