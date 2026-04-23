# Second-Pass Review: `src-tauri/src/commands_reports.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a broad but still tractable report-export coordination hub and kept the action bucket small as long as the active validation-export path stayed distinct from retained legacy builders.

## Stale Check

The live source still has the same shape:

- active validation-export helpers
- artifact-backed report reconstruction and save-back
- retained legacy/reference workbook builders that are not on the current product path

The first-pass packet remains current.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm how strongly to treat the report DTO and helper outputs as backend export contracts
   - Additional constraint informed:
     - report reconstruction behavior is compatibility surface, not incidental composition

2. Rust `std::fs::write`
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why consulted in second pass:
     - check the current save ordering and whole-file write semantics
   - Additional constraint informed:
     - save-back should remain downstream of successful workbook generation

3. `rust_xlsxwriter` `Workbook`
   - Source: <https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/struct.Workbook.html>
   - Why consulted in second pass:
     - validate whether the active workbook-generation path implies any stronger change pressure
   - Additional constraint informed:
     - the current split between workbook assembly and downstream save-back remains defensible

## Delta vs First Pass

## Refined Compatibility Finding

### 1. `build_validation_report_from_artifact` is more passthrough projection than schema-normalizing adapter

- Evidence:
  - unknown JSON fields in `run.*`, `validation.*`, and `optimization_metric_surface` still pass through the reconstruction path
- Why this matters:
  - consumers may already rely on more of the saved artifact than the helper explicitly documents
- Practical implication:
  - changes here should be treated as compatibility-sensitive even when the Rust types compile cleanly

## Refined Architecture Finding

### 2. The retained legacy workbook builders strengthen the case for deferring broad cleanup rather than forcing it through the active validation path

- Evidence:
  - the active product path is already validation-specific and distinct from the legacy builders
- Why this matters:
  - broad workbook-architecture cleanup is still lower value than preserving the current working export surface
- Practical implication:
  - keep cleanup small and local unless the product path itself changes

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`

Add or retain negative-path coverage for:

- unsupported artifact kinds
- save-back warning branches
- artifact-backed export after workbook generation succeeds

## Regression And Feature-Preservation Constraints

- Keep the active validation-export path and current artifact-save ordering
- Preserve passthrough report reconstruction from saved artifacts
- Do not accidentally reactivate the legacy builders as the production path

## Second-Pass Conclusion

`Bucket unchanged, but artifact-backed export compatibility is sharper`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens the compatibility reading: report reconstruction is more passthrough than normalization, so even modest changes here should be treated as artifact-contract-sensitive.
