# Advanced Store Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the advanced store so objective and mode ownership, shortlist state, VDPS state, Composite MO search-space state, and export-path helpers remain explicit and maintainable.

## Scope Boundaries

In scope:

- `web/src/stores/advancedStore.ts`
- ownership of advanced-search inputs
- alias normalization for advanced fields
- export browsing and Composite MO search-space reset helpers
- snapshot and history rehydration for advanced state

Out of scope:

- advanced panel presentation details by themselves
- strategy-store ownership
- backend request legality beyond the store contract

## Current User-Facing Behavior

The advanced store already owns the main advanced-search state surface for the product. That includes objective, mode, MA source, metric-contract controls, shortlist controls, top-K and heatmap controls, export-all path state, VDPS settings, and the large Composite MO search-space surface.

The store also carries real behavior. It normalizes legacy mode tokens such as `GpuProgressive` back into `Grid`, performs alias mapping for several snake_case field ids, supports export-path browsing, and exposes a reset action for Composite MO search space. Snapshot rehydration already restores a broad portion of this state.

Because this store is both large and behavior-bearing, it needs its own implementation plan distinct from the advanced panel and request builder.

## Current Code Surface

- `web/src/stores/advancedStore.ts`
- `web/src/stores/advancedStore.actions.test.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/optimizeService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/advancedService.ts`

## Current Verification Surface

- `web/src/stores/advancedStore.actions.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- advanced-store ownership is already centralized
- alias normalization for advanced field ids already exists
- export browsing and MO search-space reset behavior already live in the store boundary
- snapshot and history rehydration already exist
- the store already groups diverse advanced-search families under one canonical surface

## Weaknesses / Failure Modes

- the store mixes several advanced-search families and can become a generic catch-all without documentation
- ownership boundaries between optimize store and advanced store still have drift risk
- alias normalization and legacy mode collapsing are easy to break accidentally
- export browsing and MO reset behavior are real actions but easy to forget during refactors
- snapshot compatibility can drift if field groups are reorganized without a contract

## Regressions To Avoid

- losing alias normalization for advanced field ids
- breaking `GpuProgressive*` to `Grid` compatibility behavior
- moving request-building or legality logic into the store
- breaking export-path browsing or Composite MO search-space reset helpers
- changing snapshot replay behavior without coordinated updates

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because this store is a canonical advanced-search ownership boundary.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because alias normalization and behavioral helpers are store-level regression surfaces.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because advanced-store ownership should stay explicit relative to optimize and strategy stores.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because snapshot and history payloads behave like serialized state contracts.

## Proposed Improvement Approach

1. Document the advanced store by state family rather than one flat field list.
2. Freeze alias normalization and legacy mode compatibility as explicit behavior.
3. Keep request building and legality in the service or builder layer, not the store.
4. Document export browsing and MO reset helpers as part of the store contract.
5. Add or tighten tests around behavior-bearing helpers and round-trip state replay.

## Checklist

- [ ] Document advanced-store ownership by state family.
- [ ] Document alias normalization and legacy mode compatibility.
- [ ] Document export-path browsing and MO search-space reset behavior.
- [ ] Document snapshot and history replay expectations.
- [ ] Add or tighten tests for store behavior and ownership boundaries.

## Verification Plan

- run `npm --prefix web test -- src/stores/advancedStore.actions.test.ts`
- run `npm --prefix web test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/services/historyService.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current alias and legacy mode compatibility until migrations are deliberate
- keep the store as the owner of advanced state, not advanced legality
- prefer grouping and documentation before splitting the store
- update snapshot and history consumers together if field groups move

## Related Docs

- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/28_recommended_defaults_2026-04-09.md`
