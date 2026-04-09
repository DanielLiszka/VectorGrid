# UX Table Virtualization Plan

Last updated: 2026-04-09

## Purpose

Define the current shell table-virtualization contract so large result tables can be improved without breaking row identity, spacer math, or persisted column-state behavior.

## Scope Boundaries

In scope:

- `useVirtualTableRows.ts`
- `usePersistedMatrixColumnState.ts`
- shell panels that consume virtualization for large row sets

Out of scope:

- ranking or filtering logic for the rows themselves
- backend pagination
- options-specific table surfaces

## Current User-Facing Behavior

The shell already uses virtualization for large table-like surfaces instead of rendering every row eagerly. Current behavior includes:

- virtual row windows for top results, validation results, meta results, and execution order rows
- computed top and bottom spacer sizes
- scroll-to-top support
- persisted column visibility state with restore-default behavior
- automatic visibility for newly added columns so existing persisted layouts do not hide them silently

So virtualization is already a user-facing performance and usability contract, not just an implementation detail.

## Current Code Surface

- `web/src/composables/useVirtualTableRows.ts`
  - virtual item mapping, spacer computation, resize remeasure, and scroll reset
- `web/src/composables/useVirtualTableRows.test.ts`
  - verifies row mapping, spacer math, row identity stability, and scroll-to-top behavior
- `web/src/composables/usePersistedMatrixColumnState.ts`
  - persisted column visibility state and default-restoration behavior
- consuming panels including:
  - `web/src/components/TopWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`

## Current Verification Surface

- `web/src/composables/useVirtualTableRows.test.ts`
- panel-level tests that depend on virtualized tables
- persistence behavior through existing shell/store tests

## Strengths Already Present

- virtualization is already extracted into a reusable composable
- resize remeasure behavior already exists
- row identity is preserved when source objects remain stable
- persisted column visibility is already normalized and backward-safe for new columns
- restore-default behavior already removes persisted state when the view matches defaults

## Weaknesses / Failure Modes

- virtualization and persisted column-state behavior are related in practice but documented separately only in code
- the current row-key strategy uses row index, which is simple but worth treating as a contract
- there is no one doc that states which large panels are expected to remain virtualized
- over-scan and row-height choices are implementation constants rather than documented posture

## Regressions To Avoid

- rendering large result tables eagerly after refactors
- breaking spacer math or scroll-to-top behavior
- losing stable row references when source rows are unchanged
- hiding newly added columns because persisted layouts are treated too rigidly
- changing virtualization assumptions without panel-level review

## External Research Summary

- [TanStack Virtual introduction](https://tanstack.com/virtual/latest/docs/introduction)
  - relevant because the current virtualization composable is built on TanStack Virtual behavior and expectations
- [Vue performance best practices](https://vuejs.org/guide/best-practices/performance.html)
  - relevant because large table surfaces are one of the clearest shell-side performance pressure points
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because column-state and shell-table behavior should remain testable at the composable/store boundary
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because virtualization regressions often show up under real scroll and interaction flows

## Proposed Improvement Approach

1. Freeze virtualization as the default posture for the current large result tables.
2. Document the current composable contract:
   - row-window mapping
   - spacer computation
   - remeasure behavior
   - scroll reset
3. Document persisted column-state rules alongside virtualization since users experience them together.
4. Preserve automatic visibility for newly introduced columns.
5. Add panel-level smoke checks if any table surface changes its virtualization strategy.

## Checklist

- [ ] Document which panels currently rely on virtualization.
- [ ] Document current virtual-row, spacer, and scroll-reset behavior.
- [ ] Document persisted column-visibility normalization and restore-default semantics.
- [ ] Preserve identity stability for unchanged source rows.
- [ ] Add or tighten regression coverage when row-key or overscan assumptions change.

## Verification Plan

- run `npm --prefix web test -- src/composables/useVirtualTableRows.test.ts`
- run representative panel tests for top, validation, meta, and execution tables
- manually verify scroll, row rendering, and column visibility persistence on large result sets

## Rollout / Fallback / Compatibility Notes

- keep virtualization opt-out exceptional and deliberate
- if row-key strategy changes later, preserve user-visible stability before optimizing implementation details
- if column-state schema changes, keep new columns visible by default unless the contract changes explicitly

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/08_ml/04_ml_validation_surface_2026-04-08.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
