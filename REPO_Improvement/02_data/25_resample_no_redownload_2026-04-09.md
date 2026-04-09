# Resample No Redownload Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the no-redownload derived-timeframe posture so users and future contributors understand when the app reuses loaded base data instead of fetching again.

## Scope Boundaries

In scope:

- base-load reuse for derived timeframes
- 1m-cache-then-resample behavior for Polygon
- data-panel timeframe hints and load-source messaging
- derived-load reuse versus fresh resample status

Out of scope:

- remote provider-side resample APIs
- new cache layers beyond the current loaded-data map
- options workflows

## Current User-Facing Behavior

The app already ships a no-redownload resample posture. `web/src/stores/dataStore.ts` tells the user that higher timeframes are derived “with no extra downloads.” `src-tauri/src/state.rs` resamples from the loaded base dataset in memory. `polygon_download_1m_and_load` and `polygon_top_off_dataset` in `src-tauri/src/commands_data.rs` explicitly fetch/cache `1m` first, then resample to the requested display timeframe in memory. Load-source labels already distinguish `in_memory_base_reuse`, `derived_fresh_resample`, and `derived_in_memory_reuse`.

Current user-facing behavior includes:

- applying a higher timeframe to a loaded base dataset does not trigger another provider download
- Polygon requested timeframes still depend on a cached `1m` base under the hood
- the data panel already explains the current “no extra downloads” posture in its timeframe hint

## Current Code Surface

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/timeframe.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `src-tauri/src/state.rs`
- `crates/app_core/src/timeframe.rs`

## Strengths Already Present

- the app already avoids wasteful repeat downloads for derived timeframe changes
- load-source labels already distinguish reuse from fresh derived computation
- Polygon commands already encode the 1m-first architecture instead of pretending the provider cache stores every requested timeframe
- timeframe hints already communicate the no-redownload posture to users

## Weaknesses / Failure Modes

- the no-redownload rule is important but spread across UI hints, service normalization, command logic, and state reuse
- requested timeframe versus cached base timeframe can still be misunderstood without a dedicated doc
- load-source labels are helpful but not yet frozen as a documented contract
- derived reuse and fresh derived computation are close enough that they can be conflated in future copy changes

## Regressions To Avoid

- re-downloading provider data when a base dataset is already loaded and sufficient
- changing UI copy so users think timeframe changes fetch more provider data than they actually do
- breaking load-source labeling for reuse versus fresh derived computation
- letting provider-side timeframe requests leak into the cached-base contract without review

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because the current architecture deliberately fetches one base cadence and derives larger intervals locally.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because no-redownload behavior depends on one store-owned base dataset source of truth.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because resample correctness still depends on stable chronological base data, not repeated fetches.

## Proposed Improvement Approach

1. Document no-redownload behavior as a deliberate product rule, not an implementation accident.
2. Freeze the difference between `in_memory_base_reuse`, `derived_fresh_resample`, and `derived_in_memory_reuse`.
3. Tie Polygon’s 1m-first cache posture directly to the user-facing timeframe hint.
4. Keep service/store copy aligned with backend behavior whenever load-source semantics change.
5. Add targeted checks if any change threatens to re-fetch data unnecessarily.

## Checklist

- [ ] Document the no-redownload posture for local CSV and Polygon paths.
- [ ] Document the current load-source labels and meanings.
- [ ] Document the 1m-first Polygon architecture and requested-timeframe resample behavior.
- [ ] Document the current timeframe hint copy in the Data panel.
- [ ] Add or tighten tests if load-source semantics or hints change.

## Verification Plan

- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- run targeted state/timeframe tests for derived reuse behavior
- manually apply multiple timeframes to one loaded dataset after any resample-flow refactor

## Rollout / Fallback / Compatibility Notes

- preserve the current no-redownload posture unless a replacement architecture is intentionally adopted
- treat load-source labels and timeframe hints as user-facing contract
- prefer better docs and tests before changing resample-copy language

## Related Docs

- `REPO_Improvement/02_data/24_resample_rules_2026-04-09.md`
- `REPO_Improvement/02_data/13_polygon_request_contract_2026-04-09.md`
- `REPO_Improvement/02_data/20_polygon_dataset_topoff_2026-04-09.md`
