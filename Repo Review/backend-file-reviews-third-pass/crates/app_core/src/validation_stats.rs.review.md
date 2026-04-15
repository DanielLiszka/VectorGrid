# Third-Pass Review: `crates/app_core/src/validation_stats.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_stats.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_stats.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_stats.rs.review.md`
- Second-pass action bucket: `Document only`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the validation-layer probability gate adapter still looked like the right framing under fresh research. The third pass found that the second-pass Document only call was too mild, because this layer wraps the weaker sharpe_stats sentinel behavior into bounded Some values and then treats them as real gate inputs.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the current Option API is semantically honest
   - Third-pass constraint informed:
     - optional statistical outputs should represent real absence rather than manufactured bounded values whenever possible

2. Rust Book guidance on panic vs Result
   - Source: <https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html>
   - Why consulted in third pass:
     - compare the current sentinel wrapping to clearer absence or error signaling
   - Third-pass constraint informed:
     - non-panicking helpers still need semantically honest missing-data behavior

## What Third Pass Sharpened

### 1. The Option surface is semantically weaker than it looks
- Evidence:
  - compute_psr_dsr clamps finite PSR and DSR outputs into Some values.
  - Those inputs can already be sentinel-style probabilities from sharpe_stats rather than real evidence-backed statistics.
- Why this matters:
  - This overstates how often PSR and DSR are truly available.
- Practical implication:
  - Raise the bucket to Small surgical refactor and make semantic absence distinct from bounded sentinel values.

### 2. Gate evaluation then treats sentinel values as real evidence
- Evidence:
  - passes_probability_gates compares the optional values directly against thresholds.
- Why this matters:
  - That can make missing or weakly supported statistics participate in pass and fail gating as if they were fully real.
- Practical implication:
  - If hardened, start by fixing availability semantics before touching gate thresholds or broader validation flow.

## Implementation Priority

- First hardening step: propagate real unavailable semantics instead of manufactured bounded probabilities.
- Then add gate-path tests for missing, invalid, and underspecified probability inputs.

## Verification If Changed

- cargo test -p ta_app_core validation_stats -- --nocapture
- representative validation-gate tests for missing or underspecified PSR and DSR inputs

## Regression And Feature-Preservation Constraints

- Do not change threshold comparison policy for valid statistics while fixing availability semantics.
- Keep bounded probability behavior intact for truly valid-input paths.
- Coordinate any changed absence contract with validation consumers before rollout.

## Third-Pass Conclusion

`Second-pass bucket changed`

validation_stats.rs moves from the second-pass Document only bucket to Small surgical refactor. The third pass shows that the current Option API mostly manufactures bounded values rather than surfacing real absence, so this is a contract problem, not just a documentation gap.
