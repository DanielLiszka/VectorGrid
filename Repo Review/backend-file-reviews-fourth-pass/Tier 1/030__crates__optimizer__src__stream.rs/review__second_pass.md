# Second-Pass Review: `crates/optimizer/src/stream.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/stream.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/stream.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a clear but semantics-sensitive aggregator because merge compatibility and heap-order assumptions are observable API behavior.

## Stale Check

The live source still:

- merges partial aggregators
- uses `BinaryHeap`-based top-k storage
- finalizes output by score only

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `BinaryHeap`
   - Source: <https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html>
   - Why consulted in second pass:
     - re-check ordering guarantees and limits
   - Additional constraint informed:
     - equal-score ordering is not a stable semantic guarantee

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground merge-compatibility contract review
   - Additional constraint informed:
     - partial merge semantics should stay explicit if retained

## Delta vs First Pass

### 1. `merge()` is not actually all-or-nothing

- Why this matters:
  - it merges combo counts, `all`, `best`, and heap state before bailing out on heatmap-shape mismatch
- Practical implication:
  - incompatible heatmaps currently degrade into a partial merge rather than a full no-op

### 2. Equal-score ordering in finalized `top` is not a stable guarantee

- Why this matters:
  - `BinaryHeap` iteration order is arbitrary and final sorting only keys by score
- Practical implication:
  - if deterministic tie-order is ever required, it needs an explicit tiebreak key

## Verification If Changed

- `cargo test -p ta_optimizer -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current score normalization and merge compatibility rules unless the API shape changes deliberately
- Preserve current partial-merge semantics unless explicitly redesigned
- If deterministic tie-order becomes required, add an explicit tiebreak key rather than assuming heap behavior

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that heatmap mismatch currently yields a partial merge, and that equal-score ordering is not a stable semantic guarantee.
