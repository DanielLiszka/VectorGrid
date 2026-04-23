# Execution Order / Position Review Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because the review surface ultimately reflects broker order state and order lifecycle semantics
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
  - relevant because position review should stay truthful about what is confirmed at the broker versus inferred locally
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because optimistic review rows and broker-refresh rows depend on explicit store ownership and reconciliation
- [Tauri State Management](https://v2.tauri.app/es/develop/state-management/)
  - relevant because broker snapshot and runtime review surfaces cross a desktop/backend state boundary
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the review surface as symbol-scoped account/review, not as a full portfolio explorer.
2. Keep order-table, position, chart, and raw-preview responsibilities separate in the doc.
3. Make optimistic-versus-confirmed review behavior explicit.
4. Preserve the difference between “unknown/unfetched” and “confirmed flat.”
5. Document the current table schema and the richer backend payload it intentionally reduces.
6. Keep raw preview in scope as a diagnostic asset until a richer review surface exists.
7. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current review actions and their scope.
- [x] Document current order-table columns and what they intentionally omit.
- [x] Document position summary states, including `Position: -` versus confirmed `flat`.
- [x] Document optimistic row insertion and later broker-refresh reconciliation.
- [x] Document chart-context and raw-output roles in the review surface.
- [x] Preserve component tests for empty and populated order tables.
- [x] Preserve service/store tests for broker refresh and optimistic row behavior.
- [x] Preserve targeted backend snapshot tests if payload fields change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/executionService.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run targeted snapshot/review tests in `src-tauri/src/commands_execution.rs`
- manually verify:
  - empty review state
  - optimistic one-shot row insertion
  - broker refresh after execution
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the symbol-scoped review contract until a broader broker-review surface is intentionally added
- keep optimistic row behavior explicit if UI copy is refined
- prefer additive table/detail expansion over changing the meaning of existing review states
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by tightening review-state truth anchors in [ExecutionWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts):
  - added explicit coverage for the unfetched placeholder review state
  - locked `Position: -` as distinct from broker-confirmed `Position: flat`
  - kept the chart/review placeholder copy explicit before any broker refresh occurs
- Existing review coverage continues to anchor the rest of the contract:
  - empty and populated broker-order table rendering
  - confirmed-flat mapping on broker refresh
  - optimistic-row replacement after broker-confirmed refresh
  - raw JSON preview remaining visible alongside the reduced review table
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts src/utils/executionRisk.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/04_results/13_out_drilldown_stability_2026-04-08.md`
