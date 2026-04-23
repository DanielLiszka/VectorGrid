# Report JSON Contract Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to report JSON export so `vectorgrid_report` payload shape, field presence, artifact reconstruction, and frontend JSON download behavior remain stable and auditable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `web/src/services/validationService.ts`
- report JSON structure used for direct export and artifact-backed workbook reconstruction

Out of scope:

- workbook visual layout
- raw run artifact schema beyond its role as a report source
- options-related JSON/report payloads

## Current User-Facing Behavior

The validation workspace already supports `Export report JSON` and `Export validation JSON`. The backend also reconstructs a `vectorgrid_report` payload from a saved artifact when performing artifact-backed workbook export. The report payload already includes `version`, `kind`, `ts`, `data`, `optimization_metric_surface`, `run`, and `validation`.

This means report JSON is already both a user-facing export and an internal contract for workbook generation.

## Current Code Surface

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/validationService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `src-tauri/src/commands_reports.rs` tests
- `web/src/services/validationService.test.ts`
- any artifact reconstruction tests that inspect report payload shape

## Strengths Already Present

- report JSON already has a named/versioned top-level kind
- artifact-backed report reconstruction already exists
- dataset meta is already normalized into report `data.meta` and `data.ctx`
- optimization metric surface is already carried through when present
- frontend export already produces pretty JSON downloads for direct inspection

## Weaknesses / Failure Modes

- the report schema is implicit rather than formally documented
- direct JSON export and artifact-reconstructed report generation can drift if fields are added unevenly
- field-order expectations are only partially captured through tests and canonicalization helpers elsewhere
- `validation.res` is required for workbook generation, but that requirement is not yet frozen as a report-contract rule
- report JSON and validation JSON exports can be confused because both are offered in the same workspace

## Regressions To Avoid

- changing top-level report shape casually
- dropping `kind` or `version`
- weakening artifact-backed report reconstruction
- losing dataset meta or optimization metric surface information during report reconstruction
- making exported report JSON unsuitable for later workbook generation or debugging
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because report payload compatibility depends on clear JSON structural semantics.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because deterministic ordering/canonicalization principles help keep report and artifact reconstruction stable.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because `vectorgrid_report` should be treated as an explicit schema, not just an ad hoc object.
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because report JSON is exchanged through typed command boundaries and then reused by backend workbook builders.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because current report payload construction depends on synchronized workflow artifact and validation state.

## Proposed Improvement Approach

1. Freeze a documented `vectorgrid_report` schema.
2. Distinguish clearly between:
   - report JSON
   - validation JSON
   - saved run artifact JSON
3. Keep artifact-backed workbook generation dependent on the same documented report contract.
4. Add or tighten tests for top-level shape, required sections, and artifact reconstruction parity.
5. Keep the contract additive: new fields may be added, but required current fields should not be removed casually.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the required top-level report fields.
- [x] Document the required substructures for `data`, `run`, and `validation`.
- [x] Document when `optimization_metric_surface` is present and how it is preserved.
- [x] Add or tighten tests for direct report JSON and artifact-reconstructed report parity.
- [x] Add or tighten tests for dataset-meta projection into report data.
- [x] Clarify user-facing distinction between report JSON and validation JSON export.
- [x] Consider a formal JSON Schema document or schema-like test fixture for `vectorgrid_report`.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `commands_reports.rs` tests
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- add or run one focused artifact-reconstruction parity test
- manually export report JSON, validation JSON, and one artifact-backed report to compare shapes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve the current top-level `vectorgrid_report` envelope
- avoid schema changes that would make older saved reports unusable for workbook generation
- if a formal schema is added, keep it additive and backfill tests before relying on it operationally
- keep user-facing export labels explicit so report JSON is not mistaken for raw validation output
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/16_report_route_fallback_2026-04-09.md`
- `REPO_Improvement/06_artifacts/18_report_shortlist_csv_2026-04-09.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`

## Execution Notes

- Closed as a report-JSON contract item. The repo already keeps `vectorgrid_report` as the named/versioned report envelope used both for direct export and artifact-backed workbook reconstruction.
- Re-verified direct export status behavior in `web/src/services/validationService.test.ts` and artifact-backed report reconstruction/dataset-meta preservation in `src-tauri/src/commands_reports.rs`.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app artifact_report_builder_uses_persisted_dataset_meta --lib` and `npm.cmd --prefix web run test -- src/services/validationService.test.ts`.

