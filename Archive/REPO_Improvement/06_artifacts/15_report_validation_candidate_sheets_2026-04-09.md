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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the required candidate-sheet columns and their purpose.
- [x] Document which columns must remain identical between approved and shortlist sheets.
- [x] Document text-field handling for warnings, fail reasons, and params JSON.
- [x] Document freeze-pane and minimum width expectations.
- [x] Add or tighten tests for required headers and row-count parity.
- [x] Add or tighten tests for preserved warnings/fail-reason/params-json export.
- [x] Manually inspect one workbook with long warning/fail text and one with many candidate rows.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- add or run targeted headless workbook XML assertions for required headers
- manually inspect candidate sheets in at least one large workbook and one compatibility-mode workbook
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve current high-density candidate detail unless a replacement field is introduced
- if columns are reordered, update tests and any downstream documentation in the same change
- prefer additive readability work such as grouping, widths, and notes over removing columns
- keep compatibility-mode candidate sheets viable even when some module columns are `not run`
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
- `REPO_Improvement/06_artifacts/14_report_validation_dashboard_2026-04-09.md`

## Execution Notes

- Closed as a verification-led candidate-sheet contract item. The existing validation workbook tests already pin required sheets, row counts, shortlist cap semantics, high-density candidate detail, and strategy-agnostic profile-family coverage.
- Re-verified candidate-sheet presence and shortlist/approved parity through `src-tauri/tests/validation_xlsx_report_tests.rs` plus the shortlist CSV smoke in `src-tauri/src/commands_reports.rs`.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests` and `cargo test -p ta_desktop_demo_app validation_shortlist_csv_smoke --lib`.

