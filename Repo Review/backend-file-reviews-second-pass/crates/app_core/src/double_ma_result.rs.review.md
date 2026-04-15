# Second-Pass Review: `crates/app_core/src/double_ma_result.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_result.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_result.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this result-shaping file as mostly sound but contract-sensitive because omission behavior and matrix metadata carry outward-facing semantics.

## Stale Check

The live source still:

- builds backend result DTOs
- conditionally includes matrix metadata
- uses tolerant omission for some MA-parameter metadata

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground omission and invariant-hiding review for boundary helpers
   - Additional constraint informed:
     - silent omission on internal mismatch remains the sharper risk than broad result redesign

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - reassess how failure or omission should be described
   - Additional constraint informed:
     - non-`Grid` metadata omission remains contract-correct; the risk is silent suppression when `Grid` mapping unexpectedly fails

## Delta vs First Pass

### 1. Silent `unwrap_or(None)` suppression is primarily a diagnosability problem, not core optimization correctness

- Why this matters:
  - period lengths and MA labels still survive; the dropped information is follow-up fidelity and troubleshooting detail
- Practical implication:
  - routing the signal into diagnostics is safer than converting it into a new hard-fail boundary

## Verification If Changed

- existing `build_backend_result_*` tests in this module
- new tests for out-of-range MA variant IDs
- new tests for `Grid`-mode matrix-index suppression

## Regression And Feature-Preservation Constraints

- Preserve current successful result shaping and DTO fields
- Keep non-`Grid` results free of matrix metadata
- Do not turn omission paths into fatal errors unless callers move together

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly narrows the concern: this is still more a diagnosability and contract-transparency issue than a core correctness failure.
