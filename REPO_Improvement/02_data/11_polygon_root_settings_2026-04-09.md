# Polygon Root Settings Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to market-data root and Polygon-cache root handling so root changes, cache-path hints, and downstream dataset rewrites remain explicit and non-destructive.

## Scope Boundaries

In scope:

- configured market-data root directory settings
- effective root and Polygon cache directory derivation
- browse/set/reset/open folder behavior
- path rewriting when cached Polygon roots move

Out of scope:

- new storage backends
- remote object storage
- options-specific data roots

## Current User-Facing Behavior

The current app already supports configurable market-data roots. `src-tauri/src/settings.rs` stores an optional `market_data_root_dir`, resolves an effective root, and supports moving data when requested. `src-tauri/src/commands_data.rs` exposes `get_market_data_dir_info`, `set_market_data_dir`, and `clear_market_data_dir`, and `web/src/services/dataService.ts` plus `web/src/stores/dataStore.ts` normalize that into root hints, cache-dir text, and folder actions. The store also rewrites persisted Polygon startup-state paths when the cache root changes so previously selected datasets remain addressable.

Current shipped behavior includes:

- default root versus configured root distinction
- explicit `polygonCacheDir` in the response contract
- open/reveal folder affordances from the frontend
- optional move-existing behavior when switching market-data roots

## Current Code Surface

- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/stores/settingsStore.ts`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`

## Strengths Already Present

- root resolution is already centralized in backend settings code
- frontend code already receives normalized root info instead of reconstructing paths itself
- the app already exposes set/reset/browse/open root actions
- Polygon startup-state rewriting already acknowledges that cache roots can move

## Weaknesses / Failure Modes

- root semantics span backend settings, data service normalization, and store rewrite logic
- move-existing behavior and path-rewrite behavior are useful but not yet frozen in one doc
- settings-store and data-store surfaces both touch root-related behavior, which increases drift risk
- the current policy for when a root change should invalidate or rewrite selected datasets is implicit

## Regressions To Avoid

- losing access to cached Polygon datasets after a legitimate root move
- silently changing the effective root without updating UI hints
- breaking open/reveal folder behavior by changing which directory is treated as canonical
- moving data destructively without explicit user intent

## External Research Summary

- [Tauri Dialog Plugin](https://v2.tauri.app/plugin/dialog/) is relevant because root configuration already depends on explicit user-picked folders.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because root-info responses and startup snapshots behave like persisted contracts.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant because moved-root handling intersects with persisted startup restore state on the frontend.

## Proposed Improvement Approach

1. Document the difference between default root, configured root, effective root, and Polygon cache root.
2. Freeze move-existing semantics and current open/reveal behavior.
3. Document how root changes interact with selected Polygon dataset paths and startup restore snapshots.
4. Keep root-derived hints and settings-store synchronization explicit.
5. Add or tighten tests wherever root changes can rewrite stored paths or visible hints.

## Checklist

- [ ] Document every root-related field returned to the frontend.
- [ ] Document set/reset/browse/open behaviors and current fallback posture.
- [ ] Document move-existing semantics and current safety expectations.
- [ ] Document path rewriting for moved Polygon roots.
- [ ] Add or tighten tests for root moves and settings-store synchronization if behavior changes.

## Verification Plan

- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`
- perform one manual root-change flow with and without move-existing after root-related edits

## Rollout / Fallback / Compatibility Notes

- preserve current non-destructive defaults when changing roots
- prefer explicit rewrite behavior over silent invalidation where the app already supports safe rewrites
- avoid introducing a new canonical root meaning without updating both data and settings surfaces together

## Related Docs

- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-09.md`
- `REPO_Improvement/02_data/12_polygon_api_key_DATE.md`
- `REPO_Improvement/01_shell/11_settings_storage_shortcuts_2026-04-08.md`
