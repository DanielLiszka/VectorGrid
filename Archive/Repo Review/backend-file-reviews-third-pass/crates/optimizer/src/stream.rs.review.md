# Third-Pass Review: `crates/optimizer/src/stream.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/stream.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/stream.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/stream.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the streaming top-k and merge owner still looked like the right framing under fresh research. The third pass kept the bucket but sharpened the merge-contract defect: incompatible detail policies can still be partially merged, with one side include_all rows silently dropped.

## Third-Pass Research Log

1. Rust BinaryHeap docs
   - Source: <https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html>
   - Why consulted in third pass:
     - recheck how top-k aggregation semantics interact with merge behavior
   - Third-pass constraint informed:
     - heap-backed top-k behavior should not hide incompatible detail-policy merges

2. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the current merge helper is validating compatibility strongly enough
   - Third-pass constraint informed:
     - merge helpers should reject incompatible detail policies instead of silently dropping data

## What Third Pass Sharpened

### 1. merge still allows partial merge across incompatible detail policies
- Evidence:
  - merge does not enforce include_all compatibility before appending and merging output state.
- Why this matters:
  - One side full row set can be lost while num_combos, best, and top are still merged as if the detail policy matched.
- Practical implication:
  - Keep the existing bucket, but make detail-policy compatibility the first safe-now hardening target.

### 2. Heatmap mismatch is only one instance of a broader compatibility problem
- Why this matters:
  - The stronger issue is partial merge despite incompatible detail policy, not only heatmap shape mismatch.
- Practical implication:
  - If hardened, lead with compatibility checks before touching heap or ordering internals.

## Implementation Priority

- Reject incompatible include_all and detail-policy merges before any deeper merge cleanup.
- Add fixtures that prove full-row retention and merge compatibility rules explicitly.

## Verification If Changed

- cargo test -p ta_optimizer stream -- --nocapture
- representative merge tests that mix detail-policy and heatmap configurations

## Regression And Feature-Preservation Constraints

- Preserve current top-k ordering behavior for compatible merges.
- Do not silently discard full-row data when merge policies differ.
- Keep heap and ranking internals stable while tightening merge compatibility checks.

## Third-Pass Conclusion

`Bucket unchanged, but merge-compatibility sharpened`

stream.rs stays in Small surgical refactor. The third pass mainly sharpens the defect from partial merge behavior to a more specific contract problem: incompatible detail policies can still be partially merged while silently dropping one side full-row data.
