# Report Validation Dashboard Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the validation report dashboard sheet so the first-open workbook view remains readable, decision-oriented, and consistent across zero-survivor, compatibility, and normal shortlisted runs.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- dashboard summary blocks, chart set, active-sheet behavior, and dashboard copy

Out of scope:

- candidate-sheet column design already covered by candidate-sheet docs
- meta-analysis workbook design
- frontend validation workspace presentation

## Current User-Facing Behavior

The validation workbook already opens on `Dashboard`. Tests assert the active tab, the presence of dashboard chart titles such as `Approval state counts`, `Top rejection / review reasons`, `Profile counts`, `Robust score distribution by rank`, `Sharpe vs MaxDD`, and `WF Sharpe efficiency by robust rank`, plus extensive execution, shortlist, robustness, and regime/fallback text.

The dashboard therefore already acts as the executive summary view for the workbook.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- headless workbook-generation tests through `build_report_xlsx_bytes_headless`

## Strengths Already Present

- the dashboard is already the active sheet on open
- summary charts already exist for approval state, rejection reasons, profile counts, score distribution, and risk/return
- zero-survivor runs already emit fallback charts instead of an empty workbook
- execution notes, shortlist semantics, and route/fallback explanation are already surfaced in dashboard text
- tests already validate a large portion of dashboard presence and copy

## Weaknesses / Failure Modes

- the dashboard has grown into a dense contract surface and can drift if chart titles, summary cards, and explanatory text are changed independently
- zero-survivor and compatibility-mode dashboards can become inconsistent with normal shortlisted dashboards
- card ordering and minimum required blocks are implied by tests, not yet documented as a deliberate dashboard contract
- chart presence is tested more strongly than layout/readability rules
- a future workbook cleanup could accidentally remove important decision text because it looks redundant

## Regressions To Avoid

- changing the active-sheet default away from `Dashboard`
- dropping fallback charts for zero-survivor runs
- removing execution notes, shortlist semantics, or approval-state summary text
- making chart titles drift from the existing exported contract without coordinated test updates
- weakening compatibility-mode dashboard coverage
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because the dashboard is an intentional workbook UX surface and should preserve high-fidelity layout behavior.
- [Chart examples - rust_xlsxwriter](https://rustxlsxwriter.github.io/examples/chart_secondary_axis.html) is relevant because the dashboard already depends on readable chart composition and stable series/category labeling.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because dashboard content is derived from structured export-model fields that should stay contract-driven.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because dashboard usefulness should remain observable as user-facing behavior, not just internal XML output.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because frontend export triggers and workbook expectations should remain coupled through explicit tests, not ad hoc assumptions.

## Proposed Improvement Approach

1. Freeze the dashboard as a required workbook surface with a documented minimum block set.
2. Document mandatory chart titles and mandatory summary blocks.
3. Document the zero-survivor and compatibility-mode dashboard minimums separately.
4. Keep dashboard text decision-oriented: recommendation, execution notes, shortlist semantics, and top warning should stay visible.
5. Expand tests so chart presence, active-sheet behavior, and minimum explanatory blocks are all intentional.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the required dashboard summary cards and sections.
- [ ] Document the required dashboard chart set and titles.
- [ ] Document zero-survivor dashboard fallback behavior.
- [ ] Document compatibility-mode dashboard minimum behavior.
- [ ] Add or tighten tests for active-sheet selection and mandatory chart titles.
- [ ] Add or tighten tests for required execution-note and shortlist-semantics text.
- [ ] Manually review readability of one large shortlisted workbook and one zero-survivor workbook.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- run focused headless report tests if separate report-builder tests are added
- manually open generated workbooks covering normal, compatibility, and zero-survivor cases
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve the dashboard as the workbook landing view
- prefer additive dashboard cleanup over removing decision text
- if chart titles or card labels change, update workbook tests and any downstream parsing assumptions together
- keep zero-survivor dashboards informative rather than sparse
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/05_validation/16_xlsx_route_note_status_2026-04-09.md`
- `REPO_Improvement/06_artifacts/15_report_validation_candidate_sheets_2026-04-09.md`

