# Polygon Selection UX Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to cached-Polygon selection UX so list ordering, keyboard behavior, selection persistence, and metadata summaries stay clear without removing current controls.

## Scope Boundaries

In scope:

- Data panel cached-dataset selection list
- selected-path persistence and announcement text
- keyboard selection behavior and button semantics
- selection-related metadata summaries and action enablement

Out of scope:

- generic repo CSV browser UX
- new dataset management features not already present
- options workflows

## Current User-Facing Behavior

`DataWorkspacePanel.vue` already exposes a real cached-Polygon selection UX. Dataset cards render as buttons with pressed state tied to `polygonSelectedPath`, clicking a row updates selection, and keyboard navigation is covered in tests. `dataStore.ts` derives `polygonSelectedMeta` and ensures there is always a valid selected path when datasets are available. The selection state then drives load, validate, and repair action enablement.

Current user-facing behavior includes:

- lazy-loaded cached dataset rows with a visible pressed-state contract
- a metadata summary string for the currently selected dataset
- keyboard movement across selection rows in tests
- action buttons that depend on the selected cached dataset path

## Current Code Surface

- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Current Verification Surface

- `web/src/components/DataWorkspacePanel.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/services/dataService.test.ts`

## Strengths Already Present

- selection logic already lives in the data store rather than in the panel alone
- the panel already has keyboard and pressed-state coverage for cached dataset selection
- selection metadata text is already surfaced instead of leaving the user to infer context from a path alone
- action enablement already keys off actual selected-path state

## Weaknesses / Failure Modes

- selection UX depends on listing order subtleties that are not yet documented
- there is no dedicated doc that freezes the current button/pressed-state contract and metadata summary behavior
- selection, listing, and action enablement are closely related and easy to change independently by accident
- the panel currently merges listing and action semantics in one dense area of UI

## Regressions To Avoid

- breaking keyboard selection and pressed-state behavior
- letting selected-path persistence drift from the displayed metadata summary
- allowing action buttons to operate on stale or invalid selected paths
- making selection-dependent status text less clear
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA Button Pattern](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Attributes/aria-pressed) is relevant because the current selection rows use button semantics with a pressed state rather than a separate listbox model.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because selection persistence already depends on one store-owned source of truth.
- [WAI-ARIA Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because keyboard traversal and status announcement rules still need to remain legible as the list evolves.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the current selection contract as implemented rather than recasting it as a different widget model.
2. Freeze selected-path persistence, metadata-summary, and action-enable rules together.
3. Document current keyboard behavior and the relationship between listing refresh and selection fallback.
4. Keep selection UX distinct from repo CSV browsing and other data-ingest surfaces.
5. Add focused tests if list ordering or button semantics change.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document current button/pressed-state selection behavior.
- [x] Document selected-path and `polygonSelectedMeta` behavior.
- [x] Document action enablement based on selected cached dataset.
- [x] Document keyboard movement and focus expectations for the selection rows.
- [x] Add or tighten tests if selection semantics or ordering change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/DataWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- perform one keyboard-only and one mouse selection pass in the Data panel after UX edits
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current selection semantics unless a new explicit accessibility model is adopted
- keep selection store-owned even if the panel layout changes
- avoid merging repo CSV browsing into this contract without a separate scope decision
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Tightened `web/src/components/DataWorkspacePanel.test.ts` so the cached-Polygon selection contract is now explicit at the panel boundary: pressed-state rendering, selection-status copy, keyboard navigation, and enabled selection-scoped actions are all asserted together.
- Kept the runtime behavior store-owned and unchanged. This slice only hardened the acceptance evidence around the existing `polygonSelectedPath` / `polygonSelectedMeta` contract instead of moving selection logic into the component.
- Preserved the split between selection UX and broader repo CSV browsing; no unrelated data-panel behavior was widened or removed.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/DataWorkspacePanel.test.ts src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/utils/core/datasetMeta.test.ts src/services/historyService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
- `REPO_Improvement/02_data/08_repo_data_browser_ux_2026-04-09.md`
- `REPO_Improvement/02_data/18_polygon_dataset_validate_2026-04-09.md`
