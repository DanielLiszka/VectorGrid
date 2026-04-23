# Second-Pass Review: `crates/app_core/src/validation_stats.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_stats.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_stats.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as statistical-policy glue whose main risk is undocumented assumption drift rather than code complexity.

## Stale Check

The live source still:

- hardcodes PSR/DSR assumption paths
- applies probability gates conservatively
- normalizes thresholds into bounded ranges rather than rejecting them

The first-pass packet remains current.

## Second-Pass Research Log

1. DSR methodology references
   - Source: same methodology references used in the first-pass DSR review
   - Why consulted in second pass:
     - re-check the meaning of the probability-gate assumptions
   - Additional constraint informed:
     - the hardcoded assumptions remain policy, not simple plumbing

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground documentation expectations for hidden statistical assumptions
   - Additional constraint informed:
     - normalization-vs-rejection behavior should stay explicit

## Delta vs First Pass

### 1. `passes_probability_gates()` treating missing PSR/DSR as failure is an explicit conservative gate policy

### 2. `sanitize_probability_gate()` still normalizes invalid inputs rather than rejecting them

- Why this matters:
  - this is active behavior, not neutral sanitation

## Verification If Changed

- `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`
- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current conservative missing-stat behavior
- Do not casually change the hardcoded PSR/DSR assumptions
- Preserve bounded `[0,1]` normalization unless callers are updated

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that missing-stat failure and threshold normalization are active gate policy, not merely helper behavior.
