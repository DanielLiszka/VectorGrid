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

## External Research Summary

- [WAI-ARIA Button Pattern](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Attributes/aria-pressed) is relevant because the current selection rows use button semantics with a pressed state rather than a separate listbox model.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because selection persistence already depends on one store-owned source of truth.
- [WAI-ARIA Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because keyboard traversal and status announcement rules still need to remain legible as the list evolves.

## Proposed Improvement Approach

1. Document the current selection contract as implemented rather than recasting it as a different widget model.
2. Freeze selected-path persistence, metadata-summary, and action-enable rules together.
3. Document current keyboard behavior and the relationship between listing refresh and selection fallback.
4. Keep selection UX distinct from repo CSV browsing and other data-ingest surfaces.
5. Add focused tests if list ordering or button semantics change.

## Checklist

- [ ] Document current button/pressed-state selection behavior.
- [ ] Document selected-path and `polygonSelectedMeta` behavior.
- [ ] Document action enablement based on selected cached dataset.
- [ ] Document keyboard movement and focus expectations for the selection rows.
- [ ] Add or tighten tests if selection semantics or ordering change.

## Verification Plan

- run `npm --prefix web test -- src/components/DataWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- perform one keyboard-only and one mouse selection pass in the Data panel after UX edits

## Rollout / Fallback / Compatibility Notes

- preserve current selection semantics unless a new explicit accessibility model is adopted
- keep selection store-owned even if the panel layout changes
- avoid merging repo CSV browsing into this contract without a separate scope decision

## Related Docs

- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
- `REPO_Improvement/02_data/08_repo_data_browser_ux_2026-04-09.md`
- `REPO_Improvement/02_data/18_polygon_dataset_validate_2026-04-09.md`
