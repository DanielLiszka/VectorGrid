# DR-02 Implementation Plan — Artifact Compare and Lineage MVP

Last updated: 2026-04-09

Status: proposed bounded additive implementation plan derived from the accepted `DR-02` research response.

## Source Inputs

- research response: `responses/deep-research-report (1).md`
- synthesis judgment: `Adopt`
- synthesis anchor: `FIRST_FIVE_RESPONSE_SYNTHESIS_2026-04-09.md`

## Purpose

Add a minimal, trustworthy artifact-lineage and run-compare capability to the existing non-options artifact/history flow without introducing a separate experiment-management platform.

## Repo-Fit Summary

The repo already has strong run-artifact metadata:

- `config_fingerprint`
- normalized `dataset_meta`
- `source_content_fingerprint`
- route and fallback notes inside optimization and validation outputs

What is still missing is a general non-options surface that answers:

- what artifact came from what earlier artifact or run
- what materially changed between run A and run B
- whether dataset identity, route, or fallback assumptions changed across saved runs

This plan stays additive by extending existing artifact headers and the current History/library surfaces instead of introducing a new artifact graph system.

## Scope Boundaries

In scope:

- non-options run, validation, meta, and adjacent derived artifact families where the parent is already obvious
- compare-friendly summary extraction for saved runs
- a bounded two-run compare surface in the existing history/library workflow

Out of scope:

- options-expression artifact lineage
- a full graph explorer UI
- arbitrary many-to-many experiment DAGs
- notebook-style experiment tracking or remote sync

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/components/OverviewWorkspacePanel.vue`

## Current Strengths To Preserve

- current artifact schema compatibility
- current saved-run hydration and replay behavior
- existing winner/baseline compare flow in Overview
- current History selected-run detail actions

## Problems This Plan Solves

- saved runs can already be inspected, but there is no explicit parent-child lineage contract in the main non-options artifact flow
- compare behavior is scattered and winner-focused rather than artifact-history-focused
- request/fingerprint metadata exists, but there is no bounded summary surface that turns it into “what changed?” answers

## Proposed Outcome

The MVP should deliver three practical outcomes:

1. optional lineage fields for derived artifacts where the parent is already known
2. a compact compare header extracted from artifact content already persisted today
3. a History/library compare section that shows only meaningful differences between two selected artifacts

## Implementation Slices

### Slice 1 — Additive lineage header fields

Add optional lineage metadata to artifact families where the relationship is unambiguous.

Recommended additive fields:

- `parent_artifact_path`
- `parent_artifact_kind`
- `lineage_relation`

Initial supported relations:

- `validation_of`
- `derived_from_run`
- `derived_from_artifact_set`

Guardrails:

- all lineage fields must remain optional
- old artifacts must continue to load unchanged
- do not block save/load if lineage is unavailable

### Slice 2 — Compare-friendly artifact header extraction

Teach the backend/frontend summary layer to expose a compact compare header for the currently selected artifact.

The header should be derived from already-saved content where possible:

- strategy kind
- config fingerprint
- dataset identity and source-content fingerprint
- timeframe and resample identity
- backend route and fallback reason
- report path / validation presence when relevant

This should be summary extraction, not a second persistence format.

### Slice 3 — Two-run compare workflow in History

Extend the current History/library surface so the user can pick a baseline artifact and compare it against the selected artifact.

The first compare surface should answer:

- request/config changed?
- dataset changed?
- route/fallback changed?
- validation/report attachment changed?

The first compare surface should not attempt:

- arbitrary field-by-field JSON diff dumps
- full artifact payload side-by-side rendering
- graph visualizations

### Slice 4 — Shared compare-summary formatting

Where possible, reuse compare-summary formatting logic between:

- History artifact compare
- existing Overview baseline compare

If the two surfaces need different copy, keep the normalization shared and the wording layer separate.

## Data / Contract Notes

- prefer optional additive fields over schema rewrites
- keep existing artifact-path and hydration contracts stable
- do not split run identity across multiple new files
- avoid introducing a separate artifact index until the MVP proves its value

## UI / UX Notes

The initial History compare surface should be compact and decision-oriented:

- selected artifact summary
- baseline selector
- concise diff summary
- explicit “same / changed” lines for core identity areas

The UI should privilege clarity over exhaustiveness.

## Checklist

- keep lineage fields optional and additive
- preserve old artifact load, hydration, and replay behavior
- add parent linkage only where the parent is already obvious
- expose a compare-friendly summary header from existing persisted content
- add a bounded two-artifact compare flow in History
- limit compare output to meaningful request, dataset, route, fallback, and validation/report differences
- avoid introducing a separate experiment-management subsystem
- add targeted backend and History tests for new lineage and compare-summary behavior
- verify frontend lint, frontend typecheck, targeted frontend tests, and `cargo check --workspace`

## Verification Plan

- targeted `run_artifacts.rs` tests for optional lineage fields and summary extraction
- targeted History service/store tests for compare-state behavior
- targeted History workspace component tests for baseline selection and diff display
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- targeted `npm --prefix web run test -- ...` for touched history/analysis services and components
- `cargo check --workspace`

## Regression Guardrails

- do not break existing saved-run load/hydration
- do not require lineage for save/load success
- do not turn compare into a large always-on state machine
- keep the MVP non-options only

## Exit Criteria

- derived artifacts can optionally point at a parent artifact without breaking older files
- a saved artifact can be compared against another saved artifact in History
- the compare output clearly reports request, dataset, and route/fallback changes
- existing artifact replay and History interactions remain intact

## Related Docs

- `research/vectorgrid_deep_research_program_2026-04-09/FIRST_FIVE_RESPONSE_SYNTHESIS_2026-04-09.md`
- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/06_artifacts/26_history_service_store_2026-04-08.md`

## Deferred Follow-Ups

- full artifact graph visualization
- compare support for non-run aggregate artifact families beyond the first obvious cases
- richer candidate-level lineage once candidate identity is frozen
