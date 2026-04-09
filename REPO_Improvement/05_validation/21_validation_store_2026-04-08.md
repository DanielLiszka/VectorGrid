# Validation Store Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the validation store so validation configuration, summary state, cross-dataset selection state, export affordances, and service dispatch remain explicit and stable across the validation workspace.

## Scope Boundaries

In scope:

- `web/src/stores/validationStore.ts`
- validation configuration defaults
- validation summary/result state
- cross-dataset UI state and selection behavior
- export/report action dispatch

Out of scope:

- low-level execution logic in the validation service
- backend validation internals
- options flows

## Current User-Facing Behavior

The validation store already owns a broad but coherent validation UI state. Current families include:

- configuration inputs:
  - candidates
  - admission policy
  - seed
  - module toggles and numeric settings
- result summaries:
  - route summary
  - category summary
  - warning/skipped/cross summaries
  - table columns and rows
  - output preview
- cross-dataset selection state:
  - datasets
  - selected count
  - total count
  - search
  - cross meta
- export/report affordances
- `statusLine`
- `validateDisabled`
- `source`
- `updatedAtUnixMs`

The store already dispatches the service layer for:

- validation execution
- report/json/csv export
- reports-folder open
- cross-dataset refresh
- cross-dataset selection mutations

## Current Code Surface

- `web/src/stores/validationStore.ts`
- `web/src/stores/validationStore.actions.test.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/stores/runtimePinia.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `web/src/stores/validationStore.actions.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- validation-service tests that consume store state

## Strengths Already Present

- the store already centralizes validation UI state rather than leaving it embedded in the panel
- default validation posture is already opinionated and explicit
- cross-dataset batch selection helpers already exist
- service dispatch is already lazy-loaded and separate from raw state ownership
- `source` and `updatedAtUnixMs` already help distinguish Vue-originated mutations

## Weaknesses / Failure Modes

- the store contains a large number of fields, which makes ownership easy to blur
- snapshot restoration and current-state mutation can drift if not documented together
- cross-dataset batch actions are subtle enough to regress without obvious type failures
- service-dispatch actions are intentionally thin, which makes it easy to forget that their contract still matters

## Regressions To Avoid

- weakening current validation defaults without explicit intent
- breaking cross-dataset selection semantics or stale-selection cleanup
- desynchronizing validation panel controls from store state
- dropping `source` / `updatedAtUnixMs` freshness semantics
- bypassing service dispatch through direct component logic

## External Research Summary

- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because the validation store is a large shared state contract that benefits from explicit initial state
- [Pinia Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because configuration changes and service dispatch should remain action-driven
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because this store’s contract is best protected with direct store/action tests
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the validation workspace depends on centralized state rather than component-local mutation

## Proposed Improvement Approach

1. Break the store contract into state families:
   - configuration inputs
   - result summaries
   - cross-dataset selection state
   - export/report affordances
   - status/freshness
2. Document which actions mutate local state only and which actions dispatch the service layer.
3. Treat cross-dataset selection helpers as first-class behavior with explicit rules for:
   - select all
   - select none
   - select match
   - stale selection cleanup
4. Lock the current default validation posture in the doc so changes are intentional.
5. Expand tests around defaults, cross-selection helpers, and snapshot restore behavior.

## Checklist

- [ ] Document validation-store state families and field ownership.
- [ ] Document the current default validation posture and why it exists.
- [ ] Document local mutation actions versus service-dispatch actions.
- [ ] Document cross-dataset search, meta, and selection helper behavior.
- [ ] Document freshness fields (`source`, `updatedAtUnixMs`) and how they are used.
- [ ] Add tests for default posture and snapshot restore.
- [ ] Add tests for cross-select-all/none/match and stale-selection cleanup.
- [ ] Add tests proving service-dispatch actions remain thin wrappers over service methods.

## Verification Plan

- run `npm test -- src/stores/validationStore.actions.test.ts`
- run `npm test -- src/components/ValidationWorkspacePanel.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- manually verify validation-control state, cross-dataset actions, and export buttons in the panel

## Rollout / Fallback / Compatibility Notes

- preserve current store field names and broad defaults unless a deliberate migration is written
- keep service dispatch in the store rather than duplicating it in the component
- if the store is later split, preserve current UI behavior and cross-dataset helper semantics first

## Related Docs

- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`

