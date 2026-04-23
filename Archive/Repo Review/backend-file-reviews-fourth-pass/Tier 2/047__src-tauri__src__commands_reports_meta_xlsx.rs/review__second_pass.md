# Second-Pass Review: `src-tauri/src/commands_reports_meta_xlsx.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_meta_xlsx.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_meta_xlsx.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this workbook builder as mostly sound but semantics-sensitive because visible placeholder choices, quantile math, and workbook structure all surface directly to users.

## Stale Check

The live source still retains the same main characteristics:

- chart-heavy in-memory workbook generation
- placeholder strings like `"-"` in visible cells
- local quantile and median helper behavior that affects exported values

The first-pass packet is still current.

## Second-Pass Research Log

1. `rust_xlsxwriter` `Workbook`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/struct.Workbook.html>
   - Why consulted in second pass:
     - verify whether the in-memory save posture is itself a strong problem signal
   - Additional constraint informed:
     - `save_to_buffer()` remains acceptable until there is a measured memory issue

2. `rust_xlsxwriter` `Worksheet`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/worksheet/struct.Worksheet.html>
   - Why consulted in second pass:
     - confirm whether blank-cell support exists and whether placeholders are a deliberate workbook choice
   - Additional constraint informed:
     - the visible `"-"` placeholders are contract choice, not library limitation

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground the reusable output-contract review for a shared report builder
   - Additional constraint informed:
     - visible workbook semantics should be treated as stable output contract

## Delta vs First Pass

## Refined Placeholder Finding

### 1. The visible `"-"` placeholders are a deliberate workbook contract choice, not a crate limitation

- Evidence:
  - the workbook crate supports blank cells explicitly
- Why this matters:
  - changing placeholders would be a user-visible export semantics change, not a neutral cleanup
- Practical implication:
  - preserve the current convention unless workbook consumers deliberately approve a different display rule

## Refined Statistics Finding

### 2. Local quantile and median logic remain the highest-value small cleanup target because this surface already has multiple quantile implementations nearby

- Evidence:
  - the exported workbook still depends on locally computed summary statistics
- Why this matters:
  - statistics drift is more likely than structural workbook failure here
- Practical implication:
  - if this file is touched, prioritize correctness and consistency of visible stats over architecture churn

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_meta_xlsx -- --nocapture`

Add or retain verification for:

- all five sheets
- chart parts in the workbook ZIP
- even-sized median fixtures

## Regression And Feature-Preservation Constraints

- Keep workbook sheet names and persisted `report_xlsx_path` behavior stable
- Treat any median/quantile fix as semantics-sensitive because exported values will change
- Do not silently change visible placeholder conventions without workbook-level verification

## Second-Pass Conclusion

`Bucket unchanged, but the workbook display contract is clearer`

The first-pass `Small surgical refactor` bucket remains correct. The second pass clarifies that placeholder cells are a deliberate visible contract and that local statistics logic remains the most plausible bounded cleanup target.
