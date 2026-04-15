# Third-Pass Review: `crates/optimizer/src/grid.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/grid.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/grid.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/grid.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a shared optimizer result-shape and aggregation surface still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Rust BinaryHeap docs
   - Source: <https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html>
   - Why consulted in third pass:
     - recheck ordering and top-k aggregation assumptions in shared optimizer helpers
   - Third-pass constraint informed:
     - heap and ranking behavior should stay explicit because callers depend on emitted ordering semantics

2. Rust slice sorting docs
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html>
   - Why consulted in third pass:
     - ground the guidance around deterministic ranking and output order
   - Third-pass constraint informed:
     - sorting and emitted order are part of observable optimizer behavior once results leave the crate

3. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - confirm that result-shape compatibility is still the main third-pass concern
   - Third-pass constraint informed:
     - shared optimizer DTO shapes should move deliberately and with downstream coordination

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; result-shape compatibility remains the main risk
- Why this matters:
  - The second-pass call of `Bucket unchanged` still looks right.
  - These files still behave like shared optimizer contracts, not overlooked algorithmic liabilities.
- Practical implication:
  - Keep the current bucket and preserve emitted ordering, score-shape, and DTO behavior unless all consumers move together.

### 2. Aggregation helpers should remain explicit about what they do and do not preserve
- Why this matters:
  - Shared optimizer helpers are easy places for partial merge or ordering assumptions to leak into product behavior.
- Practical implication:
  - If changed, pair the work with fixtures that pin current ordering and emitted-shape semantics.

## Implementation Priority

- Prefer fixtures that pin current output ordering and DTO behavior before refactoring.
- Keep shared optimizer helpers conservative unless downstream consumers are updated in lockstep.

## Verification If Changed

- cargo test -p ta_optimizer grid -- --nocapture
- representative top-k/ordering/output-shape tests for optimizer consumers

## Regression And Feature-Preservation Constraints

- Preserve current emitted ordering and result-shape semantics.
- Avoid cleanup that changes what data is retained or omitted without coordinated consumer changes.
- Treat shared optimizer DTOs as active compatibility surfaces.

## Third-Pass Conclusion

`Bucket unchanged`

grid.rs stays in `Small surgical refactor`. The third pass mainly confirms that output-shape compatibility remains the real constraint here.
