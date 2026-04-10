# XLSX Sheet Chart Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current validation workbook sheet/chart contract so workbook structure, active-tab behavior, chart inventory, and sparse/compatibility semantics can be improved without breaking the exports users already rely on.

## Scope Boundaries

In scope:

- validation workbook sheet inventory and ordering
- required versus conditional chart inventory
- active-tab behavior and cap-note behavior
- sparse, compatibility, and zero-survivor workbook semantics

Out of scope:

- broader report UX critique outside workbook structure
- options reports
- non-validation Stage 3 meta report design

## Current User-Facing Behavior

The current validation export path already produces a structured workbook rather than a simple table dump. Current behavior includes:

- six worksheet surfaces in a fixed sequence
- `Dashboard` as the active tab
- approved and shortlist row-cap notes
- compatibility messaging for legacy artifact/report shapes
- zero-survivor fallback charts instead of an empty workbook
- extra plateau/drift/efficiency chart material when the underlying validation surface contains the necessary data

This means current workbook behavior is already contract-rich. Users can open the workbook and expect stable sheet names, stable chart titles, and meaningful sparse-case handling.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_xlsx.rs`
  - owns validation workbook construction, worksheet ordering, chart insertion, cap notes, and sparse-case handling
- `src-tauri/src/commands_reports_validation_common.rs`
  - supplies normalized report-model material used by the workbook builder
- `src-tauri/src/commands_reports.rs`
  - exposes current save/build entry points and artifact-backed workbook generation
- `src-tauri/tests/validation_xlsx_report_tests.rs`
  - regression coverage for sheet names, chart titles, row counts, sparse cases, compatibility behavior, and zero-survivor output

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- workbook-builder unit coverage inside `src-tauri/src/commands_reports_validation_xlsx.rs`
- headless report-build helpers through `src-tauri/src/commands_reports.rs`

## Strengths Already Present

- workbook generation is centralized in one builder path
- regression tests inspect actual workbook ZIP/XML contents
- fixed sheet names and chart titles are already covered
- sparse, compatibility, and zero-survivor behavior already have direct tests
- the workbook already distinguishes mandatory and data-dependent visual material

## Weaknesses / Failure Modes

- the exact sheet inventory and order are still spread across code and tests rather than one human-readable contract
- active-tab behavior is important but easy to miss when changing workbook construction
- chart inventory is partly conditional, but current mandatory vs conditional boundaries are not documented explicitly
- cap-note wording and “full data for charts vs capped visible rows” semantics are easy to regress subtly
- compatibility behavior can degrade if new fields are assumed everywhere

## Regressions To Avoid

- renaming or removing shipped sheet names or chart titles silently
- letting capped shortlist rows become the source for charts or audit data
- dropping compatibility or zero-survivor fallback behavior
- changing the active tab without deliberate review
- collapsing conditional charts into mandatory ones or vice versa without tests
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [rust_xlsxwriter documentation](https://rustxlsxwriter.github.io/index.html)
  - relevant because the current builder depends on workbook/worksheet/chart semantics provided by this library
- [Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook structure and chart/data limits constrain safe export behavior
- [Microsoft chart creation guidance](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2)
  - relevant because chart usefulness depends on correct titles, axes, and data selection, not only chart existence
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because the workbook builder consumes report JSON whose sparse/compatibility semantics must remain stable

## Proposed Improvement Approach

1. Freeze the workbook contract in three layers:
   - worksheet inventory and order
   - mandatory versus conditional chart inventory
   - sparse/compatibility semantics
2. Record which sheets are always present and which charts depend on richer metric surfaces.
3. Document current `Dashboard` active-tab behavior and row-cap note expectations as first-class contract items.
4. Preserve the distinction between:
   - visible row caps
   - full-data chart/audit derivation
5. Add or tighten tests around:
   - fixed sheet order
   - active tab
   - conditional chart families
   - note wording when caps or compatibility mode apply
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Enumerate the current worksheet set and fixed order.
- [x] Enumerate mandatory chart titles and conditional chart families separately.
- [x] Document the `Dashboard` active-tab contract.
- [x] Document approved/shortlist cap-note behavior and what data charts actually use.
- [x] Document compatibility-mode messaging and zero-survivor fallback output.
- [x] Add or tighten regression tests for active tab, conditional charts, and cap-note semantics.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- build one representative workbook and one sparse/zero-survivor workbook
- inspect workbook XML for worksheet order, active tab, and chart inventory
- open the files in Excel for sanity after any structural change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve current sheet names and chart titles unless a deliberate compatibility break is approved
- keep conditional-chart logic explicit rather than silently skipping mandatory charts
- preserve artifact-backed export behavior and Stage 3 fallback behavior outside this validation-specific contract
- if workbook structure changes, update tests and this document in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Confirmed the sheet and chart contract remains locked by the current workbook regression suite. The Rust tests still prove fixed worksheet order, `Dashboard` active-tab behavior, mandatory chart titles, conditional chart families, sparse cases, and compatibility-mode workbook output.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests` and `cargo test -p ta_desktop_demo_app commands_reports --lib`.

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
