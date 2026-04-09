# Metric Surface Contract Plan

Last updated: 2026-04-08

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

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because `optimization_metric_surface` behaves like a cross-layer structured contract
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because metric-surface data is transported and persisted as JSON
- [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because canonical field order and deterministic structured output are important for stable artifact/report behavior
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because workflow artifact state should remain centralized instead of each UI consumer re-parsing the surface independently

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

## Checklist

- [ ] Document the current `optimization_metric_surface` structure and ownership.
- [ ] Document how summary/detail strings are derived for history/detail surfaces.
- [ ] Document canonical field ordering and compatibility expectations.
- [ ] Verify the field survives artifact save, hydrate, and report generation unchanged.
- [ ] Add or tighten one end-to-end regression that covers the full metric-surface path.

## Verification Plan

- run `npm --prefix web test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run relevant `cargo test -p ta_desktop_demo_app` report/artifact tests
- save a representative artifact, reload it, inspect history detail, and build a report from the artifact

## Rollout / Fallback / Compatibility Notes

- preserve current field names and canonical ordering unless a compatibility migration exists
- keep legacy-summary support until saved artifacts no longer need it
- prefer one shared formatter contract over multiple drifting UI summaries
- if the surface grows, add fields rather than redefining old ones silently

## Related Docs

- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/06_artifacts/09_workflow_artifact_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
