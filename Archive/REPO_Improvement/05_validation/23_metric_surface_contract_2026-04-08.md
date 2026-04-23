# Metric Surface Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current `optimization_metric_surface` contract so artifact persistence, history detail, and report generation can be improved without dropping or reinterpreting metric-surface metadata.

## Scope Boundaries

In scope:

- production of `optimization_metric_surface` in run artifacts
- hydration and normalization of metric-surface data
- history/detail display of metric-surface summaries
- report payload carry-through of metric-surface fields

Out of scope:

- inventing new optimization metrics
- options-specific metric surfaces
- changing ranking logic itself

## Current User-Facing Behavior

The app already carries metric-surface information across multiple layers:

- saved artifacts preserve `optimization_metric_surface`
- workflow artifact hydration keeps it available after reload
- history detail shows metric-surface summaries and detail strings
- artifact-built reports preserve metric-surface content in canonical field order

The current user-visible behavior is already more than a raw JSON dump. The frontend normalizes and labels the surface into meaningful summary text such as `Core17`, `Trade22`, or legacy-style variants plus field previews.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
  - saves and rehydrates `optimization_metric_surface`
- `web/src/stores/workflowArtifactsStore.ts`
  - keeps the hydrated metric surface attached to current workflow artifacts
- `web/src/stores/workflowArtifactsStore.test.ts`
  - regression coverage for artifact payload handling
- `web/src/services/historyService.ts`
  - normalizes metric-surface summaries/details for history detail
- `web/src/services/historyService.test.ts`
  - verifies detail normalization and carry-through
- `src-tauri/src/commands_reports.rs`
  - preserves metric-surface material inside report payloads

## Current Verification Surface

- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/historyService.test.ts`
- artifact/report tests in `src-tauri/src/run_artifacts.rs`
- report-builder coverage in `src-tauri/src/commands_reports.rs`

## Strengths Already Present

- the metric surface already survives save, hydrate, detail, and report flows
- normalization exists instead of leaking raw backend structure directly into the UI
- canonical field ordering is already preserved in report generation
- history detail can already explain the surface at summary and detail levels
- workflow artifact handling already treats metric surface as first-class metadata

## Weaknesses / Failure Modes

- there is no one document that states who produces the surface and who consumes it
- summary/detail formatting logic can drift between history, workflow detail, and report consumers
- canonical field ordering is important but currently implicit
- legacy and current surface families coexist, which makes silent label drift more likely
- a future save/hydrate path could drop the field without obvious compile errors

## Regressions To Avoid

- dropping metric-surface metadata on save, hydration, or history detail load
- reordering canonical fields silently in reports
- inventing mismatched labels across frontend surfaces
- conflating legacy and current metric-surface families without compatibility handling
- treating metric-surface data as optional noise when it is already a user-visible summary surface
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because `optimization_metric_surface` behaves like a cross-layer structured contract
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because metric-surface data is transported and persisted as JSON
- [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because canonical field order and deterministic structured output are important for stable artifact/report behavior
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because workflow artifact state should remain centralized instead of each UI consumer re-parsing the surface independently
- [Microsoft Excel specifications and limits](https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3)
  - relevant because workbook, sheet, row, and chart limits should be treated as part of the report contract rather than discovered after export regressions

## Proposed Improvement Approach

1. Define one producer/consumer contract for `optimization_metric_surface`:
   - producer
   - hydration path
   - history/detail consumers
   - report consumer
2. Document the canonical `surface` and `fields` structure, including legacy compatibility if present.
3. Centralize summary/detail formatting rules where possible so UI labels stay consistent.
4. Add one end-to-end regression that proves the same artifact surface survives:
   - save
   - hydrate
   - history detail
   - report build
5. Preserve canonical field order as a contract item, not an incidental implementation detail.
6. Treat validation/report code quality as part of the implementation scope: keep workbook/report contracts, metric semantics, export diagnostics, and regression coverage explicit enough for external contributors to trust.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current `optimization_metric_surface` structure and ownership.
- [x] Document how summary/detail strings are derived for history/detail surfaces.
- [x] Document canonical field ordering and compatibility expectations.
- [x] Verify the field survives artifact save, hydrate, and report generation unchanged.
- [x] Add or tighten one end-to-end regression that covers the full metric-surface path.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run relevant `cargo test -p ta_desktop_demo_app` report/artifact tests
- save a representative artifact, reload it, inspect history detail, and build a report from the artifact
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched validation/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any applicable output or workbook parity checks

## Rollout / Fallback / Compatibility Notes

- preserve current field names and canonical ordering unless a compatibility migration exists
- keep legacy-summary support until saved artifacts no longer need it
- prefer one shared formatter contract over multiple drifting UI summaries
- if the surface grows, add fields rather than redefining old ones silently
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Tightened [validationService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationService.ts) so `optimization_metric_surface` now survives the validation-service save/reapply path: it is read from the current run artifact snapshot, preserved in the rebuilt run artifact payload, reapplied into `workflowArtifactsStore`, and forwarded into report payload generation.
- Expanded [validationService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/validationService.test.ts) so validation persistence now explicitly asserts `config_fingerprint`, `optimization_metric_surface`, and the service-built default metric-surface-aware run artifact payload.
- Revalidated the consumer side in [workflowArtifactsStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowArtifactsStore.test.ts) and [historyService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.test.ts), which already prove hydration and saved-run detail formatting for metric-surface metadata.
- The final verification set for this item was `npm --prefix web run test -- src/services/validationService.test.ts src/stores/workflowArtifactsStore.test.ts src/services/historyService.test.ts`, plus `npm --prefix web run lint` and `npm --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
