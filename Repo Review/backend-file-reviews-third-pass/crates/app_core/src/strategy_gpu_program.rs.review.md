# Third-Pass Review: `crates/app_core/src/strategy_gpu_program.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_gpu_program.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/strategy_gpu_program.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/strategy_gpu_program.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the Rust-side owner of the GPU strategy ABI still looked like the right framing under fresh research. The third pass kept the bucket but sharpened the main issue from fingerprint stability toward mirrored ABI safety: Rust-side ABI structs still mirror CUDA-side layouts without an explicit size and offset assertion layer.

## Third-Pass Research Log

1. Rust Reference type layout
   - Source: <https://doc.rust-lang.org/reference/type-layout.html>
   - Why consulted in third pass:
     - recheck what Rust is and is not promising for layout-sensitive structs
   - Third-pass constraint informed:
     - cross-language ABI mirrors need explicit validation beyond informal structural similarity

2. CUDA C++ Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>
   - Why consulted in third pass:
     - ground the paired Rust and CUDA ABI review in the live kernel contract
   - Third-pass constraint informed:
     - kernel-owner and Rust-owner sides should validate layout and numeric agreement explicitly

3. Rust DefaultHasher docs
   - Source: <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
   - Why consulted in third pass:
     - confirm that fingerprint stability is still a secondary concern relative to ABI truth
   - Third-pass constraint informed:
     - hash-derived fingerprints should not be treated as the only integrity check for cross-language ABI safety

## What Third Pass Sharpened

### 1. Mirrored ABI structs still lack explicit layout assertions
- Evidence:
  - Rust-side ABI structs are hand-maintained mirrors of the CUDA-side structs.
  - The current probe flow validates counts and a narrow checksum surface, not full size and offset agreement.
- Why this matters:
  - That makes layout drift a sharper risk than the second-pass packet emphasized.
- Practical implication:
  - Keep the existing bucket, but make explicit ABI layout validation the first safe-now hardening target.

### 2. Fingerprint integrity is helpful but not sufficient
- Why this matters:
  - A matching fingerprint does not prove full struct layout agreement across languages.
- Practical implication:
  - If this area changes, add layout and offset assertions before revisiting cache-fingerprint policy.

## Implementation Priority

- Add explicit size, offset, and layout validation between Rust-side ABI structs and the CUDA owner.
- Only after that should fingerprint or cache policy be revisited.

## Verification If Changed

- cargo test -p ta_app_core strategy_gpu_program -- --nocapture
- representative ABI and probe tests that fail on layout drift rather than only count drift

## Regression And Feature-Preservation Constraints

- Preserve current executable schema and version identifiers unless both owners move together.
- Do not rely on fingerprint checks alone as proof of ABI correctness.
- Keep cross-language numeric encodings stable until explicit validation is in place.

## Third-Pass Conclusion

`Bucket unchanged, but ABI risk sharpened`

strategy_gpu_program.rs stays in Small surgical refactor. The third pass mainly tightens the main risk from fingerprint stability to mirrored Rust and CUDA ABI without explicit layout assertions, which is the higher-value place to harden first.
