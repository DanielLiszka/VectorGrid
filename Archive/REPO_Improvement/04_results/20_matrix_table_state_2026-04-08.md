# Matrix Table State Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [TanStack Table state guide for Vue](https://tanstack.com/table/latest/docs/framework/vue/guide/table-state) is relevant because the current table stack uses shared headless state patterns that benefit from explicit ownership.
- [TanStack column visibility API](https://tanstack.com/table/v8/docs/api/features/column-visibility) is relevant for documenting stable persistence boundaries.
- [WAI ARIA sortable table example](https://www.w3.org/WAI/ARIA/apg/patterns/table/examples/sortable-table/) is relevant for truthful sort-state announcements and controls.
- [MDN: `localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant for persistence and migration behavior.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Define which table state is intentionally shared, which is panel-local, and which is persisted.
2. Require stable explicit column identifiers for persisted state instead of weak position-derived keys.
3. Document reset and migration behavior for stored table preferences when columns are renamed, split, or removed.
4. Tighten a11y expectations so visual sort state, stored sort state, and ARIA announcements match.
5. Expand tests around add/remove/reorder scenarios and state persistence across hydration/replay.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory which panels use the shared matrix-table composables.
- [x] Define shared vs panel-local vs persisted table state.
- [x] Require stable column identifiers for persisted state.
- [x] Document migration/reset behavior for stored column data.
- [x] Audit `aria-sort` and keyboard behavior against actual sort state.
- [x] Add tests for column add/remove/reorder compatibility.
- [x] Add tests for persistence boundaries across Top, Validation, Meta, and Execution panels.
- [x] Verify dense-table performance and scanability are not degraded.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted component tests for Top, Validation, Meta, and Execution panels
- run any composable-level table-state tests
- run browser UX smoke on at least one dense table flow
- manually verify stored-state migration behavior if column identifiers or persistence rules change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve existing column-visibility persistence where possible
- if stored-state migration is too ambiguous, prefer an explicit reset path over silent corruption
- treat new persisted state as additive and panel-aware rather than forcing one global table preference model
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the shared table-state layer through [useHeadlessMatrixTable.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/composables/useHeadlessMatrixTable.test.ts), [usePersistedMatrixColumnState.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/composables/usePersistedMatrixColumnState.test.ts), and the Top, Validation, Meta, and Execution panel tests.
- The remaining structural gap from the backend/frontend audit is that Meta and Validation still persist positional fallback keys like `col_0` and `col_1` instead of stable semantic keys. That is now a concrete follow-up item rather than an unknown.

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/11_out_top_results_table_2026-04-09.md`
- `REPO_Improvement/04_results/18_context_docks_2026-04-08.md`
- `REPO_Improvement/05_validation/03_validate_results_table_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/06_fe_dense_table_virtualization_2026-04-09.md`
