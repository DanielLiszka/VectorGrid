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

## Checklist

- [ ] Document the required top-level report fields.
- [ ] Document the required substructures for `data`, `run`, and `validation`.
- [ ] Document when `optimization_metric_surface` is present and how it is preserved.
- [ ] Add or tighten tests for direct report JSON and artifact-reconstructed report parity.
- [ ] Add or tighten tests for dataset-meta projection into report data.
- [ ] Clarify user-facing distinction between report JSON and validation JSON export.
- [ ] Consider a formal JSON Schema document or schema-like test fixture for `vectorgrid_report`.

## Verification Plan

- run targeted `commands_reports.rs` tests
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- add or run one focused artifact-reconstruction parity test
- manually export report JSON, validation JSON, and one artifact-backed report to compare shapes

## Rollout / Fallback / Compatibility Notes

- preserve the current top-level `vectorgrid_report` envelope
- avoid schema changes that would make older saved reports unusable for workbook generation
- if a formal schema is added, keep it additive and backfill tests before relying on it operationally
- keep user-facing export labels explicit so report JSON is not mistaken for raw validation output

## Related Docs

- `REPO_Improvement/06_artifacts/16_report_route_fallback_2026-04-09.md`
- `REPO_Improvement/06_artifacts/18_report_shortlist_csv_2026-04-09.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`

