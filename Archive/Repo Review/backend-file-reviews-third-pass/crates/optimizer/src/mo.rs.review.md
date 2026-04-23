# Third-Pass Review: `crates/optimizer/src/mo.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/mo.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/mo.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/mo.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the multi-objective search-space owner still looked like the right framing under fresh research. The third pass kept the bucket but sharpened the most concrete contract defect: a public helper fabricates a random Bool when param_idx is out of range instead of rejecting the input.

## Third-Pass Research Log

1. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the current public helper is validating its inputs honestly
   - Third-pass constraint informed:
     - public helpers should not silently convert invalid index input into plausible random data

2. Rust slice sorting docs
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html>
   - Why consulted in third pass:
     - recheck downstream ordering assumptions in the multi-objective path
   - Third-pass constraint informed:
     - sampling and ranking helpers should keep invalid-input handling separate from normal result ordering

## What Third Pass Sharpened

### 1. sample_param still fabricates a plausible Bool on invalid index input
- Evidence:
  - The public helper falls back to a random Bool when param_idx is out of range.
- Why this matters:
  - That is a real API-contract smell under Rust validation guidance even if in-tree callers currently stay in bounds.
- Practical implication:
  - Keep the existing bucket, but treat invalid-index handling as the first bounded hardening target.

### 2. The larger bucket still fits because this sits inside a wider search-space contract
- Why this matters:
  - Fixing the invalid-index path is small, but the surrounding search-space and normalization behavior is still broader than a one-line cleanup.
- Practical implication:
  - If touched, fix invalid-index semantics without reopening the wider multi-objective design all at once.

## Implementation Priority

- First hardening step: reject or explicitly surface out-of-range parameter indexes.
- Then add focused fixtures around sampling and normalization edge cases.

## Verification If Changed

- cargo test -p ta_optimizer mo -- --nocapture
- representative multi-objective sampling tests with invalid parameter indexes and non-finite objective inputs

## Regression And Feature-Preservation Constraints

- Preserve current in-range sampling and normalization behavior while fixing invalid-index handling.
- Do not conflate input validation work with wider search-space redesign.
- Keep downstream result ordering stable if sampling contracts are tightened.

## Third-Pass Conclusion

`Bucket unchanged, but invalid-index contract sharpened`

mo.rs stays in Larger refactor but defer. The third pass mainly sharpens one concrete contract bug: sample_param still fabricates a plausible Bool for out-of-range indexes instead of surfacing invalid input.
