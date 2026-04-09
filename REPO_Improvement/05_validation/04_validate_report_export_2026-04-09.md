# Validation Report Export Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation report export so JSON/XLSX output, report-path reuse, folder-open flows, and fallback behavior remain explicit, reliable, and high quality.

## Scope Boundaries

In scope:

- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- validation report JSON/XLSX export and reports-folder actions

Out of scope:

- workbook sheet/chart internals already covered by XLSX-specific docs
- artifact-library indexing beyond the report-path fields it consumes
- options-related report export flows

## Current User-Facing Behavior

The validation workspace already exposes `Export shortlist CSV`, `Export validation JSON`, `Export report JSON`, `Export report XLSX`, and `Open reports folder`. The validation service prefers saving an XLSX report through artifact-backed Tauri commands when a run artifact is available, falls back to direct report generation when needed, and preserves the returned `reportXlsxPath` for later reopen flows.

The current export path is deliberately layered: validation run -> build report payload -> attempt XLSX save -> preserve returned path -> update workflow artifacts. There is also an explicit fallback path where no validation artifact exists but a Stage 3 meta workbook path is available; in that case the current UX opens the Stage 3 report instead of pretending a validation workbook was created.

## Current Code Surface

- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

## Current Verification Surface

- `web/src/services/validationService.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports.rs` tests

## Strengths Already Present

- export actions already exist as explicit, separate affordances
- the service already prefers artifact-backed saving over throwaway download-only behavior
- auto-export failure is already treated as non-fatal during validation runs
- Stage 3 XLSX fallback is explicit instead of silently doing nothing
- report path persistence is already wired back into workflow artifacts

## Weaknesses / Failure Modes

- the current export stack has several branches that are easy to misunderstand:
  - artifact-backed save
  - direct save
  - base64/download fallback
  - Stage 3 workbook open
- users can confuse “validation ran” with “validation workbook exists and is reopenable”
- report quality, report availability, and report-path persistence are currently coupled but not documented as one contract
- silent drift between JSON export, XLSX output, and status-line language would be hard to spot without dedicated tests

## Regressions To Avoid

- breaking the artifact-backed export precedence
- hiding export/report failures behind a fake success state
- losing `reportXlsxPath` updates after a successful export
- conflating Stage 3 fallback with true validation-report generation
- degrading output quality in saved XLSX reports while leaving status messages unchanged
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because the current XLSX path relies on a builder library whose fidelity and worksheet semantics affect user-visible output quality.
- [Chart: Using a secondary axis - rust_xlsxwriter example](https://rustxlsxwriter.github.io/examples/chart_secondary_axis.html) is relevant because chart/readability decisions in exported workbooks should be treated as output-quality concerns, not incidental formatting.
- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because report JSON already behaves like an explicit contract shared across service, Tauri command, and artifact persistence flows.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because command/event boundaries affect how report generation status and follow-up actions should be surfaced to the frontend.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because export affordances and status-line outcomes should stay observable from end-user flows, not only from low-level unit tests.
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Write one explicit export contract covering JSON export, XLSX export, report-path persistence, and folder-open actions.
2. Distinguish clearly between:
   - report built
   - report saved with a stable path
   - report downloaded without a stable path
   - Stage 3 workbook opened as a fallback
3. Keep report-path updates coupled to successful save operations only.
4. Treat workbook quality/readability as a validation-surface requirement, not just a backend formatting detail.
5. Add tests around every important branch so status lines, persisted paths, and actions stay aligned.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the validation report export state machine and branch outcomes.
- [ ] Document the precedence order among artifact-backed save, direct save, download fallback, and Stage 3 open fallback.
- [ ] Document which actions should set or preserve `reportXlsxPath`.
- [ ] Document the relationship between successful XLSX save and workflow-artifact updates.
- [ ] Document the expected status-line language for success, partial success, and fallback cases.
- [ ] Add or tighten tests for auto-export failure, artifact-backed save, direct fallback, and Stage 3 fallback.
- [ ] Add or tighten tests for preserved report-path behavior after successful export.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/validationService.test.ts`
- run `npm --prefix web run test -- src/components/ValidationWorkspacePanel.test.ts`
- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- run targeted `commands_reports.rs` report-builder tests
- manually verify JSON export, XLSX export, and reports-folder open behavior from the validation tab
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve current export affordances and broad behavior first
- prefer additive status clarity over changing export semantics
- keep Stage 3 fallback explicit until a better dedicated validation fallback exists
- if report-path semantics change, update artifact persistence and history reopen flows in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
