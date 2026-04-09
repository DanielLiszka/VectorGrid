# Report Validation Candidate Sheets Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation candidate-sheet structure so `Approved Candidates`, `Shortlist`, and related candidate-bearing sheets preserve column parity, status clarity, audit linkage, and readable high-density data presentation.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- candidate sheet columns, layout, freeze panes, and candidate/audit linkage

Out of scope:

- dashboard sheet design
- CSV shortlist export
- strategy execution logic

## Current User-Facing Behavior

The workbook already emits at least `Approved Candidates`, `Shortlist`, `Audit Data`, `Dashboard`, `Distributions`, and `Regime Audit`. Candidate sheets already include dense status and metrics columns such as approval, hard gate, profile, params summary, robust score, PSR/DSR, walk-forward metrics, CSCV, cross score, bootstrap data, regime summaries, friction summaries, warnings, fail reasons, and raw params JSON.

The sheet layout already applies zoom, frozen panes, and explicit column widths, so candidate sheets are intentionally designed rather than raw dumps.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- headless workbook-generation smoke tests

## Strengths Already Present

- candidate sheet headers and layout are already centralized in the workbook writer
- candidate rows are derived from a shared export model rather than duplicated per sheet
- many robustness/status metrics are already present in exported columns
- tests already verify row counts, many expected labels, and broad workbook content
- params JSON is already included for exact reproducibility

## Weaknesses / Failure Modes

- column sprawl makes drift between sheet semantics and test assumptions easy
- approved-vs-shortlist parity is mostly implicit and can break when columns are added
- long text fields such as warnings, fail reasons, and params JSON can degrade readability if widths or ordering drift
- audit-sheet references are visible in text but not yet fully documented as a candidate-sheet contract
- future cleanup work could delete a column that looks redundant but is part of a downstream workflow

## Regressions To Avoid

- changing candidate-sheet column order casually
- making approved and shortlist sheets report different semantics for the same column name
- removing params JSON, warnings, or fail reasons without an explicit replacement
- breaking freeze panes or basic width/readability defaults
- reducing candidate-sheet detail in a way that weakens downstream audit or reproducibility

## External Research Summary

- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because candidate sheets are high-density structured workbook surfaces and need stable worksheet behavior.
- [Chart examples - rust_xlsxwriter](https://rustxlsxwriter.github.io/examples/chart_secondary_axis.html) is relevant because candidate sheets feed dashboard and audit views; readable source sheets matter even when charts are the headline.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because params JSON is deliberately embedded in candidate sheets and should remain faithful to the structured source data.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because candidate sheet columns are effectively a tabular projection of a structured export schema.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because candidate-sheet usability should remain observable through exported behavior, not only code-level assumptions.

## Proposed Improvement Approach

1. Freeze a documented candidate-sheet column contract.
2. Separate required columns into groups:
   - identity and approval
   - in-sample metrics
   - robustness modules
   - ranking stability
   - warnings and reproducibility
3. Document which columns must remain identical between approved and shortlist sheets.
4. Keep audit linkage explicit from candidate sheets into regime/audit data surfaces.
5. Add tests that protect column presence, row parity, and core readability defaults.

## Checklist

- [ ] Document the required candidate-sheet columns and their purpose.
- [ ] Document which columns must remain identical between approved and shortlist sheets.
- [ ] Document text-field handling for warnings, fail reasons, and params JSON.
- [ ] Document freeze-pane and minimum width expectations.
- [ ] Add or tighten tests for required headers and row-count parity.
- [ ] Add or tighten tests for preserved warnings/fail-reason/params-json export.
- [ ] Manually inspect one workbook with long warning/fail text and one with many candidate rows.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- add or run targeted headless workbook XML assertions for required headers
- manually inspect candidate sheets in at least one large workbook and one compatibility-mode workbook

## Rollout / Fallback / Compatibility Notes

- preserve current high-density candidate detail unless a replacement field is introduced
- if columns are reordered, update tests and any downstream documentation in the same change
- prefer additive readability work such as grouping, widths, and notes over removing columns
- keep compatibility-mode candidate sheets viable even when some module columns are `not run`

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
- `REPO_Improvement/06_artifacts/14_report_validation_dashboard_2026-04-09.md`
