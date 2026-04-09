# Polygon Dataset Listing Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to manifest-backed Polygon dataset listing so lazy loading, ordering, selected-path persistence, and cache metadata remain explicit and stable.

## Scope Boundaries

In scope:

- manifest-backed `polygon_get_cache_info` listing
- frontend lazy loading and refresh behavior
- selected-path persistence and default-selection rules
- listing order and recency subtleties after downloads

Out of scope:

- new data providers
- eager startup loading of the Polygon cache
- options workflows

## Current User-Facing Behavior

Current listing is manifest-backed and lazy. `polygon_get_cache_info` in `src-tauri/src/commands_data.rs` reads the current Polygon cache manifest and returns `cacheDir`, `hasApiKey`, and `datasets`. `web/src/stores/dataStore.ts` only loads that info when the Polygon panel opens or refreshes, and `selectPolygonDatasetPath()` preserves or chooses the selected path. One subtle current behavior is ordering: backend listing follows `BTreeMap` key order, but the store may temporarily unshift a freshly downloaded dataset to the top before the next refresh snaps it back to manifest order.

Current user-facing behavior therefore includes:

- lazy listing rather than startup-time eager scans
- visible cache-dir and API-key state alongside dataset rows
- persisted selected-path handling with graceful fallback to the first dataset when necessary
- a UI ordering experience that can differ temporarily after downloads

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.test.ts`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/DataWorkspacePanel.test.ts`
- `src-tauri/src/polygon.rs`

## Strengths Already Present

- listing already depends on one manifest-backed backend path
- lazy loading avoids expensive startup scans
- selected-path behavior is already centralized in the data store
- cache metadata and listing rows already flow together in one response surface

## Weaknesses / Failure Modes

- listing order versus recency order is subtle and not yet documented as a contract
- selected-path persistence and fallback rules are store-owned but easy to change accidentally
- the listing contract also carries `cacheDir` and `hasApiKey`, which can be forgotten if contributors think only about datasets
- there is no dedicated implementation doc explaining why lazy load is the current posture

## Regressions To Avoid

- turning listing into an eager startup scan
- dropping `cacheDir` or `hasApiKey` from the response without updating consumers
- breaking selected-path fallback and leaving no valid selection after refresh
- changing order semantics without deciding between manifest order and recency order explicitly

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because listing is a shared response contract, not just a panel detail.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because selection and lazy-load semantics already rely on one store-owned source of truth.
- [WAI-ARIA Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the frontend already supports keyboard-oriented cached-dataset selection and status updates.

## Proposed Improvement Approach

1. Document the listing response contract, including non-dataset metadata fields.
2. Freeze lazy-load semantics and explain why listing is not a startup responsibility.
3. Document selected-path preservation and fallback behavior clearly.
4. Call out current ordering behavior and decide whether it should stay manifest-key-based, recency-based, or intentionally mixed.
5. Keep listing behavior aligned with Data panel selection UX and refresh flows.

## Checklist

- [ ] Document `cacheDir`, `hasApiKey`, and `datasets` as one response contract.
- [ ] Document lazy-load and refresh behavior.
- [ ] Document selected-path preservation and fallback selection.
- [ ] Document current ordering behavior and known recency-versus-manifest subtlety.
- [ ] Add or tighten tests if listing order or lazy-load semantics change.

## Verification Plan

- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- run `npm --prefix web test -- src/components/DataWorkspacePanel.test.ts`
- manually verify listing open, refresh, selection, and post-download behavior after any change

## Rollout / Fallback / Compatibility Notes

- keep listing lazy by default unless there is a strong product reason to change it
- prefer documenting ordering semantics before changing them
- preserve the full response shape for current consumers

## Related Docs

- `REPO_Improvement/02_data/15_polygon_manifest_schema_2026-04-09.md`
- `REPO_Improvement/02_data/21_polygon_selection_ux_2026-04-09.md`
- `REPO_Improvement/02_data/11_polygon_root_settings_2026-04-09.md`
