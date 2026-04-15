# Second-Pass Review: `crates/app_core/src/double_ma_ma_kind.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_ma_kind.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_ma_kind.rs.review.md`
- First-pass action bucket: `Accept as-is`
- Second-pass action bucket: `Accept as-is`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as a good small validator and accepted it as-is.

## Stale Check

The live source still performs:

- case-insensitive MA lookup
- period-based and single-output eligibility checks
- explicit rejection on unsupported MA kinds

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - verify argument-validation expectations for a public helper
   - Additional constraint informed:
     - error strings here are effectively user-facing contract because `double_ma.rs` routes requests through this helper

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground failure-condition clarity
   - Additional constraint informed:
     - the helper remains small enough that the main concern is preserving accurate rejection behavior

3. Rust Book test organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why consulted in second pass:
     - confirm good placement for additional small validator coverage
   - Additional constraint informed:
     - registry-driven tests should stay close to the owner

## Delta vs First Pass

### 1. Case-insensitive lookup is a deliberate contract behavior, not convenience noise

- Why this matters:
  - tightening it to exact match would change accepted request behavior

### 2. Additional negative tests, if added, should stay registry-driven

- Practical implication:
  - avoid hard-coding specific upstream MA IDs that may drift

## Verification If Changed

- `cargo test -p ta_app_core validate_ma_kind -- --nocapture`
- add registry-driven tests for non-period-based and multi-output rejection

## Regression And Feature-Preservation Constraints

- Keep period-based and single-output as the Double-MA acceptance rule
- Preserve case-insensitive lookup
- Preserve current rejection wording unless downstream tests/UI move with it

## Second-Pass Conclusion

`No material delta from first pass`

The first-pass `Accept as-is` bucket remains correct. The second pass mainly clarifies that case-insensitive lookup and current rejection wording are active contract behavior worth preserving.
