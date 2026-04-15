# Second-Pass Review: `crates/app_core/src/stats_common.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/stats_common.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/stats_common.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as a tiny shared-statistics helper where the real risk is accidental semantics drift in quantile and median conventions.

## Stale Check

The live source still:

- uses rounded-index quantile policy
- uses simple upper-midpoint median for even-length `u32` samples
- returns forgiving defaults on empty or non-finite inputs

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why consulted in second pass:
     - reassess how much of the helper behavior is policy rather than math inevitability
   - Additional constraint informed:
     - quantile and median conventions should be treated as documented policy choices

2. Rust slice `partition_point`
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point>
   - Why consulted in second pass:
     - provide standard-library context for selection conventions
   - Additional constraint informed:
     - simple selection policy remains acceptable as long as it stays stable

## Delta vs First Pass

### 1. The rounded-index quantile convention is definitely policy, not accidental shortcut

### 2. Forgiving zero/default fallbacks are not mathematically neutral even if they are operationally convenient

- Practical implication:
  - keep the file in `Document only` and avoid silent convention drift

## Verification If Changed

- `cargo test -p ta_app_core stats_common::tests:: -- --nocapture`
- representative validation/stat-summary tests if quantile behavior changes

## Regression And Feature-Preservation Constraints

- Do not silently change quantile or median conventions
- Keep empty/non-finite fallback outputs stable unless all consumers move together
- Treat even small formula or rounding changes as user-visible semantics shifts

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that quantile and median choices here are explicit policy and that even the forgiving defaults are not mathematically neutral.
