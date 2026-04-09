# XLSX Dashboard Quality Plan

Last updated: 2026-04-09

## Purpose

Define the quality bar for the validation workbook dashboard sheet so that it remains decision-useful rather than merely decorative.

## Scope Boundaries

In scope:

- dashboard summary content
- warning/recommendation visibility
- chart usefulness
- route/fallback and module-status presentation

Out of scope:

- workbook structure as a whole
- non-XLSX result panes

## Current User-Facing Behavior

The validation workbook already includes a true dashboard sheet rather than a plain title page. That sheet currently carries recommendation and approval summaries, internal links to the main detail sheets, route/cross/module/fallback summaries, warning summaries, shortlist count, plateau/drift/efficiency summaries when available, and at least two audit-derived charts (`Robust score by rank` and `Sharpe vs MaxDD`). Even so, the sheet’s decision hierarchy is still implicit in code and regression tests rather than written as a quality contract.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

## Current Verification Surface

- workbook XML tests
- manual workbook review in Excel

## Strengths Already Present

- the workbook already includes multiple summary/chart concepts
- route notes, recommendation labels, and module outcomes already exist in the source model
- the report builder already computes rich summary fields
- the dashboard already links to `Approved Candidates`, `Shortlist`, `Distributions`, and `Audit Data`
- sparse and degraded cases already surface `not run` labels rather than blank cells in many slots
- the regression suite already checks route-note visibility, skipped-module visibility, shortlist count, and zero-survivor fallback behavior

## Weaknesses / Failure Modes

- the dashboard can drift toward completeness without clarity
- warnings and degraded-module states may not receive enough visual priority
- charts can survive structurally while becoming analytically weak
- it is easy to add summary rows without preserving an explicit “what should the reviewer decide from this sheet first” hierarchy
- internal link targets and chart placement can drift even while the workbook still opens
- the dashboard and `Distributions` sheet together form a decision surface, but the division of responsibilities is not documented clearly

## Regressions To Avoid

- hiding the final recommendation or approval distribution behind clutter
- burying route/fallback or omitted-module notes
- keeping charts that do not answer a real review question
- implying confidence when the underlying run had thin evidence or skipped modules
- removing sheet links or dashboard summaries that currently spare the reviewer from opening raw tables first
- replacing explicit degraded evidence text with empty cells or overly polished phrasing
- letting `Dashboard` duplicate `Distributions` poorly instead of using the two sheets intentionally
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) reinforces clear chart/data matching, titles, and readable presentation.
- [Microsoft Support: Creating charts from start to finish](https://support.microsoft.com/en-us/office/creating-charts-from-start-to-finish-36f8a2b7-0f74-4d40-942d-bc2e42436d73) reinforces selecting data and chart types intentionally rather than by default.
- [rust_xlsxwriter](https://rustxlsxwriter.github.io/index.html) is relevant because the implementation can support explicit dashboard layout and formatting decisions rather than accidental defaults.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Define the dashboard as the first-review sheet for analyst decisions, not as a dumping ground for every computed statistic.
2. Lock the dashboard summary stack in priority order:
   - final recommendation / approval posture
   - warnings and degraded-evidence markers
   - route / fallback / module-status summaries
   - shortlist and evidence counts
   - navigation links to supporting sheets
3. Define which current charts remain mandatory because they answer a clear question:
   - `Robust score by rank`
   - `Sharpe vs MaxDD`
   - selected distribution charts on the companion `Distributions` sheet such as `Approval state counts`, `Profile counts`, and drift/efficiency charts when data exists
4. Explicitly define the behavior for zero-survivor, compatibility-mode, skipped-module, and thin-evidence cases so the dashboard remains honest under degraded runs.
5. Keep the dashboard distinct from `Audit Data`: summary and decision cues belong on `Dashboard`, while raw supporting fields stay in the audit sheet.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the dashboard’s required top-level summary blocks.
- [ ] Define what must always be visible without opening another sheet.
- [ ] Define which warnings and route notes deserve high-visibility placement.
- [ ] Define which charts are mandatory, optional, or removable.
- [ ] Define the exact role split between `Dashboard`, `Distributions`, and `Audit Data`.
- [ ] Define the dashboard behavior for zero-survivor and thin-evidence cases.
- [ ] Define compatibility-mode copy requirements when verdict fields are missing.
- [ ] Define formatting priorities for readability over density.
- [ ] Add XML or content checks for key dashboard labels where feasible.
- [ ] Manually review one representative workbook after any dashboard change.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- inspect a representative workbook in Excel
- verify recommendation/warning content survives sparse and degraded cases
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve the workbook even if the dashboard needs to degrade gracefully
- prefer clearer labels and fewer but better charts over adding more summary decoration
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/06_artifacts/14_report_validation_dashboard_2026-04-09.md`
