# Second-Pass Review: `crates/optimizer/src/grid.rs`

- Review date: `2026-04-14`
- Source file: `crates/optimizer/src/grid.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/grid.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as small and test-covered but contract-sensitive because its non-finite score normalization intentionally mirrors stream aggregation.

## Stale Check

The live source still:

- clamps non-finite scores to `NEG_INFINITY`
- returns `top_scores` while leaving `all_scores` unset even when `all` is populated

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground explicit scoring-behavior documentation for a small public entry point
   - Additional constraint informed:
     - the current asymmetry is observable result-shape contract, not incidental implementation detail

## Delta vs First Pass

### 1. `grid_search()` is clearly a compatibility path, not just a simple path

- Why this matters:
  - it mirrors stream scoring and also preserves its own result-shape asymmetry
- Practical implication:
  - keep cleanup small and documentation-heavy

## Verification If Changed

- `cargo test -p ta_optimizer -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve non-finite score normalization parity with `StreamAggregator`
- Preserve current `best`, `top`, `top_scores`, and `all` shaping unless downstream consumers move together

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that `grid_search()` is a compatibility surface whose result-shape asymmetry is now part of the DTO contract.
