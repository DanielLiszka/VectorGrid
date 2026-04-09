# Data Service And Store Plan

Last updated: 2026-04-08

## Purpose

Define the current frontend data-layer split so `dataService` remains the Tauri/normalization boundary and `dataStore` remains the workflow state owner for loaded data, derived data, Polygon cache state, startup restore, and clear/reset flows.

## Scope Boundaries

In scope:

- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- data startup restore and deferred hydration
- Polygon cache/list/download/top-off/validate/repair flows

Out of scope:

- options datasets
- Rust-side data cache internals
- brand-new data providers

## Current User-Facing Behavior

The data layer already behaves like a two-part contract. `dataService` normalizes Tauri responses, enforces required-path checks, and keeps browse/sample/open/close helpers thin and stable. `dataStore` owns the real workflow: load/browse/sample actions, base-vs-derived dataset state, startup snapshot restore, deferred hydration, Polygon cache visibility, exact-range top-off behavior, moved-root rewriting for Polygon datasets, and workflow-artifact clearing on reset. The UI already depends on that split even though it is not written down explicitly.

## Current Code Surface

- `web/src/services/dataService.ts`
  - normalization of load/cache/Polygon payloads and filesystem/command wrappers
- `web/src/services/dataService.test.ts`
  - service-level normalization and command-wrapper coverage
- `web/src/stores/dataStore.ts`
  - frontend state owner for loaded data, derived data, startup restore, and Polygon flows
- `web/src/stores/dataStore.actions.test.ts`
  - store-level coverage for startup restore, Polygon flows, root rewriting, and clear-data behavior
- `web/src/components/DataWorkspacePanel.vue`
  - consumes current data-store state and action methods
- `web/src/utils/core/datasetMeta.ts`
  - artifact-oriented dataset metadata helper fed by data-store state

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/DataWorkspacePanel.test.ts`

## Strengths Already Present

- service normalization already happens at the boundary instead of leaking `unknown` payloads into components
- store state already preserves base-vs-derived identity and not just a single active dataset
- startup restore is already bounded and deferred rather than eager and destructive
- Polygon top-off behavior already preserves the active derived timeframe when it should
- clear-data flows already clear workflow-artifact state instead of leaving stale analysis attached

## Weaknesses / Failure Modes

- `dataStore.ts` is large enough that ownership boundaries are easy to lose
- several state updates reuse similar result-application logic across load/download/top-off flows
- `setControlsBusy()` style capability toggling is explicit but verbose and easy to drift
- `openPolygon()` and `closePolygon()` are intentionally thin, which makes their contract easy to underestimate
- market-data-root rewriting logic is correct but specialized enough to be fragile if changed casually

## Regressions To Avoid

- making browse/sample flows throw instead of remaining user-safe
- collapsing base and derived dataset state into one opaque active row
- eagerly scanning or restoring Polygon state during startup before the store is ready
- breaking exact-range top-off behavior or derived-timeframe preservation
- rewriting non-Polygon paths when market-data root changes
- leaving workflow artifacts intact after data reset

## External Research Summary

- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because `dataStore` is already a domain state boundary rather than component-local state
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because most of the data workflow is already action-driven
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because `dataService` is a command-wrapper boundary
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV remains a real external dataset contract in the non-options workflow
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because dataset identity/provenance uses filesystem metadata downstream

## Proposed Improvement Approach

1. Make the service/store split explicit:
   - `dataService` owns normalization and command wrappers
   - `dataStore` owns workflow state and orchestration
2. Break the store into documented subdomains before any code refactor:
   - startup restore
   - base/derived dataset session state
   - Polygon cache and dataset selection
   - market-data-root management
3. Consolidate repeated result-application logic only when current behavior is preserved exactly.
4. Treat workflow-artifact clearing and dataset identity as non-negotiable parts of the data contract.

## Checklist

- [ ] Document the current responsibility split between `dataService` and `dataStore`.
- [ ] Document startup restore and deferred hydration behavior.
- [ ] Document base vs derived dataset fields and status semantics.
- [ ] Document Polygon cache/list/download/top-off/repair/validate flows.
- [ ] Document market-data-root rewrite behavior for cached Polygon datasets.
- [ ] Document clear-data side effects on workflow artifacts.
- [ ] Add or tighten tests if store/service boundaries or result-application logic change.

## Verification Plan

- run `npm --prefix web test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts`
- run `npm --prefix web test -- src/components/DataWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current state semantics before attempting any structural split of `dataStore.ts`
- keep service normalization backward-compatible with current Tauri payload shapes
- if no-op service helpers are removed or formalized, update every store caller in the same change

## Related Docs

- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/02_data/26_derived_dataset_identity_2026-04-08.md`
