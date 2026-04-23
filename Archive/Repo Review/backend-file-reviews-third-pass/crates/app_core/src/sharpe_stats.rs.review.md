# Third-Pass Review: `crates/app_core/src/sharpe_stats.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/sharpe_stats.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/sharpe_stats.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/sharpe_stats.rs.review.md`
- Second-pass action bucket: `Document only`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the PSR and DSR math helper surface still looked like the right framing under fresh research. The third pass found that the second-pass Document only call was too mild, because invalid or underspecified PSR input still returns a neutral-looking 0.5 while adjacent helpers use None for invalid-input paths.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether invalid statistical input should keep collapsing to an apparently valid probability
   - Third-pass constraint informed:
     - public helpers should not silently turn unknown into a seemingly meaningful middle value without explicit contract

2. Rust Book guidance on panic vs Result
   - Source: <https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html>
   - Why consulted in third pass:
     - compare the current sentinel behavior to clearer absence or error signaling
   - Third-pass constraint informed:
     - invalid-input handling should be semantically honest even if it remains non-panicking

## What Third Pass Sharpened

### 1. The neutral 0.5 fallback is stronger than a documentation issue
- Evidence:
  - probabilistic_sharpe_ratio returns 0.5 when inputs are non-finite or samples < 2.
  - minimum_track_record_length returns None for its own invalid-input path.
- Why this matters:
  - That asymmetry can make insufficient evidence look like a real middling probability instead of unavailable statistics.
- Practical implication:
  - Raise the bucket to Small surgical refactor and treat sentinel semantics as the first safe-now hardening target.

### 2. Adjacent helpers already model invalidity differently
- Why this matters:
  - The inconsistency makes the current 0.5 behavior harder to defend as intentional contract rather than accidental convenience.
- Practical implication:
  - If changed, prefer explicit absence or a separately documented sentinel over a neutral-looking probability.

## Implementation Priority

- First hardening step: make invalid or underspecified-input output semantically distinct from a real middling probability.
- Add focused tests for low-sample and non-finite-input paths before touching the formulas.

## Verification If Changed

- cargo test -p ta_app_core sharpe_stats -- --nocapture
- representative PSR and DSR tests for non-finite inputs and undersized sample counts

## Regression And Feature-Preservation Constraints

- Do not perturb the numeric formulas for valid-input PSR and DSR paths while fixing sentinel semantics.
- Keep valid-output probability ranges unchanged.
- Make any sentinel or absence contract explicit to downstream callers before rollout.

## Third-Pass Conclusion

`Second-pass bucket changed`

sharpe_stats.rs moves from the second-pass Document only bucket to Small surgical refactor. The third pass shows that returning a neutral-looking 0.5 for invalid or underspecified PSR input is a behavioral contract problem, not just a documentation gap.
