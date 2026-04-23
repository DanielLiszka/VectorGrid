# Second-Pass Review: `crates/app_core/src/sharpe_stats.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/sharpe_stats.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/sharpe_stats.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as a small statistical helper whose main need is explicit documentation of approximations and sentinel behavior.

## Stale Check

The live source still:

- uses approximation formulas
- returns sentinel-style values rather than `Result`
- feeds validation statistics rather than owning methodology end to end

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why consulted in second pass:
     - reassess numerical fallback behavior documentation
   - Additional constraint informed:
     - sentinel outputs remain a deliberate contract choice

2. Rust `Result` docs
   - Source: <https://doc.rust-lang.org/std/result/>
   - Why consulted in second pass:
     - contrast explicit error-returning API shape with the current sentinel-returning behavior
   - Additional constraint informed:
     - this file intentionally does not use a `Result`-style contract

## Delta vs First Pass

### 1. The real contract is the sentinel behavior on invalid or undersized inputs, not the formula bodies

### 2. `minimum_track_record_length` is the most semantics-sensitive helper here

- Why this matters:
  - callers can interpret `None` as "not enough evidence," not only "mathematically impossible"

## Verification If Changed

- `cargo test -p ta_app_core sharpe_stats::tests:: -- --nocapture`
- `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Keep current fallback outputs unless downstream validation gates move together
- Do not swap approximation formulas casually
- Preserve deterministic outputs for identical inputs

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that sentinel behavior on invalid inputs is the central contract to document here.
