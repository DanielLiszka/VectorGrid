# Workflow Bootstrap Data Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to dataset bootstrap and restore sequencing so application startup stays fast, restore behavior stays truthful, and boot-time data readiness does not regress into duplicated or hidden work.

## Scope Boundaries

In scope:

- `web/src/main.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/stores/dataStore.ts` startup hydration and deferred affordances
- startup dataset snapshot persistence and restore behavior
- data-panel mount behavior that overlaps with boot

Out of scope:

- redesigning the entire workflow bootstrap system
- options-related startup flows
- changing backend data loading contracts

## Current User-Facing Behavior

The app already treats dataset readiness as part of startup. `web/src/main.ts` runs storage migrations, initializes stores, and then calls `applyWorkflowBootstrapFromStorage({ includeDeferredData: false })` through the workflow bootstrap path. That in turn drives `dataStore.hydrateStartupState()` and waits for critical dataset readiness before the shell is considered fully ready.

The current bootstrap path already separates critical and deferred work. `dataStore.ts` uses shared promises and one-shot flags for startup hydration and deferred hydration, so boot and panel mount do not duplicate restore work. The critical portion reads the startup snapshot, refreshes market-data-root information, optionally restores the saved base CSV, and reapplies the derived timeframe when needed. Sample discovery and similar affordances are deferred separately.

The persisted startup snapshot already carries more provenance than the restore path actively uses. It stores `autoRestoreDataset`, `baseCsvPath`, `selectedTimeframe`, `source`, `baseSourceSizeBytes`, `baseSourceMtimeUtcMs`, and `savedAtUnixMs`. At the moment, restore behavior primarily uses the auto-restore flag, path, source, and timeframe, while size and mtime remain persisted metadata rather than active restore gates.

## Current Code Surface

- `web/src/main.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/stores/dataStore.ts`
- `web/src/utils/storage.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/stores/settingsStore.ts`

## Current Verification Surface

- `web/src/main.test.ts`
- `web/src/services/workflowBootstrap.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/services/dataService.test.ts`

## Strengths Already Present

- boot already distinguishes critical data readiness from deferred affordances
- shared startup promises already prevent duplicate restore work across boot and panel mount
- persisted startup state already captures dataset-source hints beyond just a file path
- startup restore already cooperates with moved Polygon roots and rewritten dataset paths
- current tests already cover important boot and hydration behavior

## Weaknesses / Failure Modes

- bootstrap ownership is spread across `main.ts`, workflow bootstrap helpers, and the data store, which makes it easy to drift
- the current restore contract is stronger in storage than in runtime enforcement because stored size and mtime are not yet first-class restore gates
- `DataWorkspacePanel.vue` still triggers hydration on mount, which is safe today because of promise guards but remains a fragile duplication point
- there is no dedicated implementation doc freezing which startup work is critical, deferred, or forbidden on the critical path
- contributors can accidentally move cache scans or affordance loading onto the critical boot path

## Regressions To Avoid

- making startup scan the Polygon cache or sample inventory before the shell is ready
- losing the shared one-shot promise guards and causing duplicate restore work
- hiding restore failures behind a false-ready shell state
- breaking startup snapshot compatibility or alias migration behavior
- weakening moved-root or rewritten-path restore handling

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because startup bootstrap should remain store-driven rather than component-driven.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because desktop boot sequencing should keep frontend state authority explicit instead of scattering initialization logic.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant because startup dataset state is persisted in browser storage and must remain compatible and failure-tolerant.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the startup snapshot is effectively a persisted compatibility contract even if it is not yet formalized as schema.

## Proposed Improvement Approach

1. Freeze the startup contract in writing: what runs on the critical boot path, what is deferred, and what is explicitly forbidden.
2. Keep dataset-bootstrap authority in `dataStore.ts`, with workflow bootstrap as the caller rather than the owner of detailed restore logic.
3. Document the exact relationship between `applyWorkflowBootstrapFromStorage({ includeDeferredData: false })`, `hydrateStartupState()`, and `ensureStartupAffordancesLoaded()`.
4. Make the persisted snapshot fields explicit in the plan, including which are active restore gates today and which are only metadata.
5. Add targeted tests around concurrent boot-plus-panel hydration so future refactors cannot accidentally duplicate restore work.

## Checklist

- [ ] Document the critical boot sequence from `main.ts` through workflow bootstrap into the data store.
- [ ] Document the core-vs-deferred split inside `hydrateStartupState()`.
- [ ] Document which startup snapshot fields are active restore inputs versus passive metadata.
- [ ] Document the interaction between startup restore and moved Polygon roots.
- [ ] Add or strengthen tests covering concurrent bootstrap and panel-mount hydration.

## Verification Plan

- run `npm --prefix web test -- src/main.test.ts`
- run `npm --prefix web test -- src/services/workflowBootstrap.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- perform one manual startup cycle with auto-restore enabled and one with a moved Polygon root if the bootstrap path changes

## Rollout / Fallback / Compatibility Notes

- keep current startup snapshot keys and alias handling compatible
- preserve the current fast-path behavior where deferred affordances do not block shell readiness
- prefer additive truthfulness and stronger tests over changing the visible boot model
- if stronger provenance checks are introduced later, make them explicit rather than silently changing restore behavior

## Related Docs

- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-09.md`
- `REPO_Improvement/02_data/08_repo_data_browser_ux_2026-04-09.md`
- `REPO_Improvement/02_data/33_data_cache_state_backend_2026-04-09.md`
- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
