# Execution Order / Position Review Plan

Last updated: 2026-04-09

## Purpose

Define the current execution order/position review contract so broker refresh, order-table rendering, position summary behavior, chart context, and optimistic-versus-confirmed state remain explicit and truthful.

## Scope Boundaries

In scope:

- broker order refresh
- position summary rendering
- order-table review surface
- chart-context and raw-output review surfaces tied to execution review
- optimistic rows versus broker-confirmed rows

Out of scope:

- live runner automation logic beyond what appears in review
- adding a full portfolio review surface
- options workflows

## Current User-Facing Behavior

The execution review surface currently centers on `Test account` and `Refresh orders`. Refresh is symbol-scoped and populates:

- broker-order rows
- a single position summary string
- chart metadata/context
- raw JSON output preview

The UI then shows runtime status, chart context, broker orders, and output preview together. Current behavior means:

- this is a symbol-focused review surface, not a full portfolio workstation
- empty review state, optimistic order feedback, and broker-confirmed refresh are different situations
- `Position: -` is not the same as a confirmed flat position
- the order table is a review surface over selected fields, not a full broker schema dump

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
  - account/review controls, position display, order table, chart context, and raw preview
- `web/src/services/executionService.ts`
  - broker refresh request shaping, optimistic row insertion, and review-surface updates
- `web/src/stores/executionStore.ts`
  - runtime review state, order rows, and derived summary strings
- `src-tauri/src/commands_execution.rs`
  - broker snapshot command and backend review payload shaping

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- targeted execution tests in `src-tauri/src/commands_execution.rs`

## Strengths Already Present

- the UI already separates review actions from execution actions
- broker refresh returns more than just orders, which helps align orders with position and chart context
- the store/service path already distinguishes optimistic local updates from broker refresh
- the order table uses a stable, readable subset of fields instead of overwhelming raw broker payloads

## Weaknesses / Failure Modes

- “Refresh orders” can sound broader than it is; it is not a full account refresh
- the backend snapshot has richer fields than the UI currently surfaces, so docs can easily over-promise review completeness
- chart context can remain blank or placeholder-backed
- optimistic rows can briefly coexist with later broker-confirmed rows, which can confuse users if not explained
- empty symbol handling can degrade the review model into a less truthful pseudo-portfolio view

## Regressions To Avoid

- describing the current review surface as a full portfolio or account-review system
- hiding the distinction between optimistic and broker-confirmed order rows
- collapsing `Position: -` and confirmed `flat` into the same state
- removing or weakening the raw preview without replacing its diagnostic value

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because the review surface ultimately reflects broker order state and order lifecycle semantics
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
  - relevant because position review should stay truthful about what is confirmed at the broker versus inferred locally
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because optimistic review rows and broker-refresh rows depend on explicit store ownership and reconciliation
- [Tauri State Management](https://v2.tauri.app/es/develop/state-management/)
  - relevant because broker snapshot and runtime review surfaces cross a desktop/backend state boundary

## Proposed Improvement Approach

1. Document the review surface as symbol-scoped account/review, not as a full portfolio explorer.
2. Keep order-table, position, chart, and raw-preview responsibilities separate in the doc.
3. Make optimistic-versus-confirmed review behavior explicit.
4. Preserve the difference between “unknown/unfetched” and “confirmed flat.”
5. Document the current table schema and the richer backend payload it intentionally reduces.
6. Keep raw preview in scope as a diagnostic asset until a richer review surface exists.

## Checklist

- [ ] Document the current review actions and their scope.
- [ ] Document current order-table columns and what they intentionally omit.
- [ ] Document position summary states, including `Position: -` versus confirmed `flat`.
- [ ] Document optimistic row insertion and later broker-refresh reconciliation.
- [ ] Document chart-context and raw-output roles in the review surface.
- [ ] Preserve component tests for empty and populated order tables.
- [ ] Preserve service/store tests for broker refresh and optimistic row behavior.
- [ ] Preserve targeted backend snapshot tests if payload fields change.

## Verification Plan

- run `npm --prefix web test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/executionService.test.ts`
- run `npm --prefix web test -- src/stores/executionStore.actions.test.ts`
- run targeted snapshot/review tests in `src-tauri/src/commands_execution.rs`
- manually verify:
  - empty review state
  - optimistic one-shot row insertion
  - broker refresh after execution

## Rollout / Fallback / Compatibility Notes

- preserve the symbol-scoped review contract until a broader broker-review surface is intentionally added
- keep optimistic row behavior explicit if UI copy is refined
- prefer additive table/detail expansion over changing the meaning of existing review states

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_DATE.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/04_results/13_out_drilldown_stability_2026-04-08.md`
