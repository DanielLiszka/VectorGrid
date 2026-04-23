# Second-Pass Review: `src-tauri/src/commands_reports_validation_xlsx.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_validation_xlsx.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_validation_xlsx.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a mostly sound workbook renderer whose main sensitivity is visible export semantics rather than architecture.

## Stale Check

The live source still has the same visible contracts:

- mixed numeric/text placeholder cells
- fixed sheet names and dashboard links
- workbook structure driven from the validation-common model

The first-pass packet remains current.

## Second-Pass Research Log

1. `rust_xlsxwriter` `Worksheet`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
   - Why consulted in second pass:
     - verify typed writes and blank-cell behavior
   - Additional constraint informed:
     - mixed-type columns are a deliberate contract choice, not a renderer limitation

2. `rust_xlsxwriter` `Url`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/struct.Url.html>
   - Why consulted in second pass:
     - confirm sheet-link and internal hyperlink behavior
   - Additional constraint informed:
     - sheet names and link targets are stronger compatibility tokens than the first pass emphasized

3. `rust_xlsxwriter` `IgnoreError`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/enum.IgnoreError.html>
   - Why consulted in second pass:
     - assess whether number-as-text warning suppression is available if the mixed-type contract stays
   - Additional constraint informed:
     - warning-suppression exists, so the current visible output remains a policy choice

## Delta vs First Pass

### 1. `"not run"` and `"n/a"` in numeric columns are deliberate workbook semantics, not crate-imposed compromise

- Why this matters:
  - changing them would alter how users read and sort exported sheets
- Practical implication:
  - treat any placeholder normalization as visible export-policy work

### 2. Sheet names and internal links are tighter contract tokens than the first pass made explicit

- Why this matters:
  - workbook link integrity depends on those names remaining stable
- Practical implication:
  - even "cosmetic" sheet-title changes need explicit workbook-level verification

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_validation_xlsx -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`
- manual workbook-open smoke test

## Regression And Feature-Preservation Constraints

- Preserve current sheet names and dashboard link targets
- Preserve shortlist ordering text unless intentionally changed
- Treat placeholder semantics as visible export contract

## Second-Pass Conclusion

`Bucket unchanged, but visible workbook semantics are more explicit`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the mixed-type placeholder cells and internal sheet-link names are deliberate export semantics, not incidental implementation detail.
