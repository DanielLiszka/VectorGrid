# Second-Pass Review: `crates/app_core/src/double_ma_backend.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_backend.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_backend.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as small policy plumbing where the main work is documenting environment-flag and support-check semantics clearly.

## Stale Check

The live source still provides:

- env-flag interpretation for hybrid backend enablement
- top-k capability helpers
- backend blocker descriptions

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `std::env::var`
   - Source: <https://doc.rust-lang.org/std/env/fn.var.html>
   - Why consulted in second pass:
     - verify exact env-read behavior for the hybrid backend toggle
   - Additional constraint informed:
     - anything other than `"0"` or `"false"` still enables the feature

2. CUDA Programming Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/index.html>
   - Why consulted in second pass:
     - ground the backend-capability context these helpers are documenting
   - Additional constraint informed:
     - this file is describing policy/capability boundaries, not implementing runtime logic

## Delta vs First Pass

### 1. Empty-string or unexpected env values still enable the hybrid backend

- Why this matters:
  - the flag is intentionally permissive, but that permissiveness should stay documented rather than assumed away

### 2. `top_k == 0` is already normalized to `1`, not treated as unset

- Practical implication:
  - preserve this as explicit policy unless callers are migrated together

## Verification If Changed

- add unit tests for env values `""`, `"0"`, `"false"`, and `"FALSE"`
- add `top_k` cases for `0`, `1`, `256`, and `257`

## Regression And Feature-Preservation Constraints

- Preserve current env-flag spellings unless a breaking change is explicitly documented
- Preserve top-k support boundaries and blocker semantics
- Keep this file policy/documentation-oriented rather than widening its execution role

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that the hybrid env flag is intentionally permissive and that `top_k == 0` already means "minimum valid request," not "unset."
