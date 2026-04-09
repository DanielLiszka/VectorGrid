# Report Route Fallback Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to report route/fallback persistence so validation route evidence survives artifact-backed export and remains consistent between saved artifacts, rebuilt reports, and workbook execution notes.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `web/src/services/validationService.ts`
- artifact-backed report export of validation route and fallback metadata

Out of scope:

- optimization route selection logic
- history/detail UI beyond route metadata preservation
- options-related fallback/report behavior

## Current User-Facing Behavior

When a report is exported from a persisted run artifact, the backend rebuilds a `vectorgrid_report` shape from artifact data and then writes the validation workbook. The workbook already surfaces backend route, fallback reason, route notes, expected execution, and top warnings.

That means route/fallback explanation is intended to be preserved even when the user no longer exports directly from the current in-memory workflow state.

## Current Code Surface

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `web/src/services/validationService.ts`
- `src-tauri/tests/validation_xlsx_report_tests.rs`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports.rs` tests for artifact-backed report reconstruction
- `web/src/services/validationService.test.ts`

## Strengths Already Present

- artifact-backed report export already exists as a first-class path
- validation report reconstruction already preserves `run`, `validation`, and dataset meta sections
- workbook tests already exercise route/fallback and expected-execution text extensively
- frontend export logic already prefers artifact-backed export when possible
- route/fallback text is already visible rather than hidden in raw JSON only

## Weaknesses / Failure Modes

- route/fallback preservation depends on report reconstruction staying aligned with the validation artifact shape
- artifact-backed export and in-memory export can drift if one path gains extra report enrichment and the other does not
- route/fallback fields are strongly relied on by workbook text but are not yet documented as a persistence contract
- current tests are strong on workbook text, but weaker on proving artifact-backed and direct export parity for route metadata
- route evidence can be lost indirectly if validation artifact persistence changes upstream

## Regressions To Avoid

- losing backend route/fallback fields when exporting from a saved artifact
- making artifact-backed reports materially different from direct in-memory reports for route evidence
- weakening workbook execution notes during report reconstruction
- breaking report export for runs that have validation data but no other optional artifact sections
- forcing users to rerun validation just to recover route/fallback explanation
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because direct export and artifact-backed export are separate command paths that should return equivalent user-facing semantics.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because route/fallback metadata moves through JSON report/artifact payloads before workbook rendering.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because route/fallback evidence should be preserved deterministically across report reconstruction and artifact persistence.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because route/fallback fields should be treated as part of the report contract, not optional incidental text.
- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because the end result is a stable workbook artifact whose explanatory sheets depend on preserved structured metadata.

## Proposed Improvement Approach

1. Freeze route/fallback metadata as required report fields whenever validation data exists.
2. Document direct-export and artifact-backed-export parity expectations.
3. Add or tighten tests that compare route/fallback behavior across both export paths.
4. Keep workbook execution-note rendering driven from reconstructed structured fields, not custom artifact-specific shortcuts.
5. Preserve additive route evidence even for degraded or zero-survivor runs.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the route/fallback fields that must survive artifact-backed export.
- [ ] Document parity expectations between direct export and artifact-backed export.
- [ ] Add or tighten tests for artifact-backed export route/fallback preservation.
- [ ] Add or tighten tests for zero-survivor and degraded-route artifact-backed exports.
- [ ] Confirm route-note and expected-execution sections read from reconstructed report fields only.
- [ ] Confirm frontend artifact-backed export preference remains intact.
- [ ] Document fallback behavior when validation data is absent and Stage 3 meta export is used instead.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- run targeted `commands_reports.rs` tests
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- manually compare one direct export and one artifact-backed export from the same validation result
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve current artifact-backed export preference
- avoid adding route-note enrichment only to one export path
- if report reconstruction fields are changed, update both the workbook writer and the parity tests together
- keep Stage 3 fallback explicit rather than silently substituting a validation workbook when validation data is missing
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/16_xlsx_route_note_status_2026-04-09.md`
- `REPO_Improvement/05_validation/17_xlsx_open_reveal_flows_2026-04-09.md`
- `REPO_Improvement/06_artifacts/19_report_save_open_reveal_2026-04-09.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-09.md`

