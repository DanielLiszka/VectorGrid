# Matrix Table State Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to shared matrix-table state so non-options result and review tables preserve the right state, migrate safely when columns change, and remain accessible and predictable across panels.

## Scope Boundaries

In scope:

- shared headless matrix-table state
- persisted column-state behavior
- stable column identity and migration/reset rules
- a11y and UX implications for sort/visibility/state persistence

Out of scope:

- options result tables
- redesigning every result table visually
- replacing the current headless-table stack wholesale

## Current User-Facing Behavior

Several non-options panels share table infrastructure for dense matrix-like data. Column visibility persistence already exists, but other state such as sorting, selection, pinning, and panel-specific behaviors can remain transient or inconsistently owned.

## Current Code Surface

- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/utils/core/storageKeys.ts`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`

## Current Verification Surface

- panel-specific component tests for Top, Validation, Meta, and Execution workspaces
- table behavior tests tied to shared composables
- browser UX smoke for dense table surfaces

## Strengths Already Present

- there is already a shared headless table layer
- persisted column visibility exists instead of each panel inventing its own storage
- multiple panels already benefit from common table behavior
- table state is at least partly centralized in composables rather than embedded in template logic

## Weaknesses / Failure Modes

- only part of table state is persisted today
- default column keys such as `col_n` are vulnerable to reorder/add/remove drift
- some non-results consumers reuse the same table composables, which increases the blast radius of changes
- `aria-sort` behavior is manual and could drift from actual sort state
- reset/migration semantics for stored table state are not obvious

## Regressions To Avoid

- breaking persisted column preferences for existing users without a reset/migration plan
- making large tables slower or less navigable
- losing sort or visibility truthfulness across hydration and replay
- introducing a global table-state model that incorrectly couples unrelated panels
- weakening keyboard/a11y behavior while chasing persistence features

## External Research Summary

- [TanStack Table state guide for Vue](https://tanstack.com/table/latest/docs/framework/vue/guide/table-state) is relevant because the current table stack uses shared headless state patterns that benefit from explicit ownership.
- [TanStack column visibility API](https://tanstack.com/table/v8/docs/api/features/column-visibility) is relevant for documenting stable persistence boundaries.
- [WAI ARIA sortable table example](https://www.w3.org/WAI/ARIA/apg/patterns/table/examples/sortable-table/) is relevant for truthful sort-state announcements and controls.
- [MDN: `localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant for persistence and migration behavior.

## Proposed Improvement Approach

1. Define which table state is intentionally shared, which is panel-local, and which is persisted.
2. Require stable explicit column identifiers for persisted state instead of weak position-derived keys.
3. Document reset and migration behavior for stored table preferences when columns are renamed, split, or removed.
4. Tighten a11y expectations so visual sort state, stored sort state, and ARIA announcements match.
5. Expand tests around add/remove/reorder scenarios and state persistence across hydration/replay.

## Checklist

- [ ] Inventory which panels use the shared matrix-table composables.
- [ ] Define shared vs panel-local vs persisted table state.
- [ ] Require stable column identifiers for persisted state.
- [ ] Document migration/reset behavior for stored column data.
- [ ] Audit `aria-sort` and keyboard behavior against actual sort state.
- [ ] Add tests for column add/remove/reorder compatibility.
- [ ] Add tests for persistence boundaries across Top, Validation, Meta, and Execution panels.
- [ ] Verify dense-table performance and scanability are not degraded.

## Verification Plan

- run targeted component tests for Top, Validation, Meta, and Execution panels
- run any composable-level table-state tests
- run browser UX smoke on at least one dense table flow
- manually verify stored-state migration behavior if column identifiers or persistence rules change

## Rollout / Fallback / Compatibility Notes

- preserve existing column-visibility persistence where possible
- if stored-state migration is too ambiguous, prefer an explicit reset path over silent corruption
- treat new persisted state as additive and panel-aware rather than forcing one global table preference model

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/11_out_top_results_table_DATE.md`
- `REPO_Improvement/04_results/18_context_docks_DATE.md`
- `REPO_Improvement/05_validation/03_validate_results_table_DATE.md`
- `REPO_Improvement/10_tests_perf/06_fe_dense_table_virtualization_DATE.md`
