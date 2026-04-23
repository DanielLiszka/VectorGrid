# Repo Data Browser UX Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the current data-workspace browsing and dataset-selection UX so CSV loading, sample selection, Polygon-cache inspection, and storage affordances stay clear without removing current capabilities.

## Scope Boundaries

In scope:

- Data workspace browse/load controls
- sample CSV affordances
- Polygon cache listing and selection controls
- storage-folder affordances and related hints

Out of scope:

- new data sources beyond current CSV and Polygon lanes
- options datasets
- full redesign of the shell layout

## Current User-Facing Behavior

`web/src/components/DataWorkspacePanel.vue` already exposes a real data-ingestion control surface. Users can browse a CSV, apply a recent path, load curated sample CSVs, open or close the Polygon section, inspect cached Polygon datasets, select a cached dataset, request validation or repair, and jump to storage settings. The panel also shows range, row count, field count, field hints, dataset context, status line, and timestamps.

Current UX already includes meaningful details:

- sample CSV cards can be shown or hidden based on availability
- Polygon cached datasets are navigable and announce the current selection
- storage-root hints surface the effective market-data root and cache path
- dataset context and field hints give users immediate ingest-quality feedback

## Current Code Surface

- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/DataWorkspacePanel.test.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- the Data workspace already exposes multiple ingestion and cache-management paths in one panel
- selection state, status text, and metadata hints are already surfaced to the user
- the Polygon selection list already has keyboard-navigation coverage
- storage actions and settings handoff already exist instead of forcing manual file-system work

## Weaknesses / Failure Modes

- the panel combines many responsibilities, so behavior is easy to change without a shared UX contract
- repo/browse/sample/Polygon flows can blur together without explicit terminology rules
- dataset-context, field-hint, and status-line semantics are useful but not yet frozen in one doc
- there is no single UX plan that states what a “ready”, “restore pending”, or “selected cached dataset” posture should look like

## Regressions To Avoid

- removing current browse/sample/Polygon capabilities in the name of simplification
- breaking keyboard selection and status announcements for cached Polygon datasets
- making storage-root or dataset-context cues less visible
- allowing data-panel copy and actual data-store behavior to drift apart
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because current data UX depends on one store-backed source of truth for several related controls.
- [Tauri Dialog Plugin](https://v2.tauri.app/plugin/dialog/) is relevant because CSV and folder browsing already depend on explicit pick-file/pick-folder flows.
- [WAI-ARIA Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because cached-dataset list navigation and selection behavior should remain keyboard-legible as the data browser evolves.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the current user tasks the Data panel already supports and keep them additive.
2. Freeze selection-state, status-line, and metadata-summary behavior for CSV and Polygon paths separately.
3. Clarify panel language so repo CSV, sample CSV, picked CSV, and Polygon cached dataset flows stay distinct.
4. Keep keyboard and announcement behavior first-class for cached dataset selection.
5. Add UX-level checks anywhere the panel relies on implicit copy or status semantics.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document each current user task supported by the Data workspace.
- [x] Document selection, summary-chip, and status-line behavior for loaded datasets.
- [x] Document current Polygon cached-dataset selection and announcement behavior.
- [x] Document the storage-settings handoff and folder affordances.
- [x] Add or tighten tests for any copy, keyboard, or selection contract changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/DataWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- perform one manual keyboard and mouse pass across the Data workspace after UI changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep all current browse and Polygon controls unless an equivalent safer path is already in place
- prefer clarifying current UX before compressing or restyling it
- maintain keyboard selection behavior and visible status cues as non-negotiable baseline behavior
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added a broad component-level contract test in `web/src/components/DataWorkspacePanel.test.ts` that freezes the current combined UX surface: dataset summary chips, sample dataset affordances, recent paths, dataset context, field hints, effective storage hints, and status text now render together under test instead of relying on ad hoc manual checks.
- Preserved the existing Polygon keyboard-selection path and kept the data panel’s shipped scope intact; this slice hardened current affordances rather than introducing a new repo-browser mode.
- Added an additive summary improvement in `web/src/components/DataWorkspacePanel.vue`: when a derived timeframe is active, the panel now surfaces `Base timeframe: ...` so current-vs-base context is explicit without removing any existing chip or control.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/DataWorkspacePanel.test.ts src/stores/dataStore.actions.test.ts src/services/dataService.test.ts src/services/optimizeService.test.ts src/services/validationService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo check -p ta_app_core`.

## Related Docs

- `REPO_Improvement/02_data/01_repo_csv_inventory_2026-04-09.md`
- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-09.md`
- `REPO_Improvement/02_data/21_polygon_selection_ux_2026-04-09.md`
