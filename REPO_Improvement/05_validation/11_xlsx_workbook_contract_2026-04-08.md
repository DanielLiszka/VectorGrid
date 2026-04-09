# XLSX Workbook Contract Plan

Last updated: 2026-04-08

## Purpose

Define the current validation-workbook contract so sheet inventory, chart inventory, placeholder semantics, row-cap behavior, compatibility mode, and zero-survivor behavior remain stable and testable.

## Scope Boundaries

In scope:

- workbook-level structure and required sheets
- chart inventory and chart-to-sheet dependencies
- row-cap behavior and truncation notes
- `not run`, `n/a`, compatibility-derived, and zero-survivor semantics
- build/save/openability expectations for validation XLSX output

Out of scope:

- broader dashboard-presentation critique beyond contract-level workbook guarantees
- options reports

## Current User-Facing Behavior

Users can build or save a validation workbook from current validation results or saved artifacts. The current workbook is not a single-sheet export. It is a multi-sheet package with explicit candidate tables, distributions, dashboard material, and regime audit content. Current tests show the workbook includes these sheet names:

- `Approved Candidates`
- `Shortlist`
- `Distributions`
- `Audit Data`
- `Dashboard`
- `Regime Audit`

The workbook also carries chart content and current tests verify titles such as:

- `Approval state counts`
- `Top rejection / review reasons`
- `Profile counts`
- `Robust score distribution by rank`
- `Sharpe vs MaxDD`
- `WF Sharpe efficiency by robust rank`

The repo already exposes important user-facing semantics that the final implementation doc should preserve:

- `Approved Candidates` can be truncated to a visible cap with an explicit note
- `Shortlist` can be visibly capped while charts and audit sheets still use the full row set
- compatibility mode can derive approval labels when newer verdict fields are absent
- zero-survivor workbooks still emit fallback charts instead of degrading into an empty shell
- placeholders such as `not run` and `n/a` are written intentionally, not accidentally

## Current Code Surface

- `src-tauri/src/commands_reports_validation_xlsx.rs`
  - owns workbook construction, sheet naming, chart insertion, placeholder writers, candidate-sheet layout, dashboard/distribution/regime-audit generation, visible-cap notes, and compatibility-mode notes
- `src-tauri/src/commands_reports_validation_common.rs`
  - supplies report-model material and strings such as `not run` summaries used by the workbook builder
- `src-tauri/src/commands_reports_api.rs`
  - exposes the build/save command surface used by the app
- `src-tauri/tests/validation_xlsx_report_tests.rs`
  - verifies required sheets, chart titles, row counts, placeholder strings, sparse scenarios, compatibility behavior, and zero-survivor fallback charts

Repo-grounded invariants that should be captured explicitly:

- `build_validation_report_xlsx()` writes six workbook sheets in a fixed order
- visible-cap notes differ between approved rows and shortlist rows
- compatibility mode changes explanatory text, not just data population
- the workbook active tab is currently the dashboard (`activeTab="4"` in tests)
- zero-survivor reports must still produce fallback charts and a clear run-verdict explanation

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
  - workbook structural regression coverage
- report command coverage through `src-tauri/src/commands_reports_api.rs`
- headless report-build paths used by the desktop/report workflow

## Strengths Already Present

- workbook generation is centralized in one Rust builder rather than fragmented across multiple surfaces
- tests already inspect ZIP/XML content directly instead of only checking that a file was emitted
- the current tests check real user-facing sheet names and chart titles
- sparse and degraded cases are already represented, including zero-survivor output and placeholder semantics

## Weaknesses / Failure Modes

- the contract is richer than the current doc, so changes could still be made without a single human-readable inventory of required sheets and notes
- XML openability alone is weaker than semantic correctness; a workbook can open while still mislabeling compatibility mode or placeholder meaning
- row-cap messaging, full-data chart usage, and compatibility-derived approval labels are easy to break in subtle ways because they are explanatory behaviors, not just schema fields
- the distinction between `not run`, `n/a`, omitted values, and compatibility-derived values is not yet formalized in one document

## Regressions To Avoid

- silently renaming or removing required sheets
- changing chart titles or chart source sheets without updating tests and docs
- breaking workbook openability in Excel
- losing visible-cap disclosure or making the shortlist look complete when charts actually use a wider row set
- dropping compatibility-mode notes or zero-survivor fallback behavior

## External Research Summary

- [rust_xlsxwriter documentation](https://rustxlsxwriter.github.io/index.html)
  - relevant because the current builder depends on a library that explicitly supports multi-worksheet XLSX generation and chart creation
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because worksheet/chart limits and workbook memory constraints bound what a robust export contract should assume
- [Microsoft: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2)
  - relevant because chart usefulness depends on correct data selection, titles, axis labeling, and series configuration, not just chart existence

## Proposed Improvement Approach

1. Define the workbook contract in three layers:
   - workbook inventory
   - sheet-level invariants
   - chart/placeholder semantics
2. Record the exact required sheet set and which user question each sheet answers.
3. Record the current chart set, chart titles, and which sheet/range families feed them.
4. Define placeholder semantics explicitly:
   - `not run`
   - `n/a`
   - omitted / absent
   - compatibility-derived values
5. Treat visible-cap notes, compatibility notes, and zero-survivor fallback charts as first-class contract items, not implementation trivia.

## Checklist

- [ ] Enumerate the required workbook sheets and current order.
- [ ] Enumerate the required chart titles and chart families.
- [ ] Document approved-row and shortlist visible-cap behavior.
- [ ] Document how `Approved Candidates`, `Shortlist`, and `Audit Data` differ in scope.
- [ ] Document `Dashboard` and `Distributions` responsibilities separately.
- [ ] Document `Regime Audit` expectations and when it may contain `n/a` versus `not run`.
- [ ] Define compatibility-mode semantics when newer verdict fields are absent.
- [ ] Define zero-survivor fallback requirements.
- [ ] Add or tighten tests anywhere a sheet/chart/placeholder invariant is still implicit.
- [ ] Verify the workbook still opens in Excel after any contract-affecting change.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- build one representative workbook with a rich report payload
- build one sparse or zero-survivor workbook
- inspect workbook XML for sheet/chart inventory and open both files in Excel for sanity

## Rollout / Fallback / Compatibility Notes

- preserve workbook openability first, but do not treat openability as the whole contract
- preserve current sheet names unless a compatibility migration is written explicitly
- if chart inventory changes, update both the workbook contract doc and regression tests in the same change
- prefer additive workbook metadata and notes over silent reinterpretation of existing cells

## Related Docs

- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/06_artifacts/14_report_validation_dashboard_DATE.md`
