# Live Execution Broker Position And Order Reconciliation Plan

Last updated: 2026-04-12

## Purpose

Define a measured plan to make broker position, open-order, and desired-position reconciliation more explicit for the current live-execution implementation before any broader live-scope expansion.

## Scope Boundaries

In scope:

- reconciliation between desired local action, broker position, and open broker orders
- current single-symbol live mode only
- Alpaca snapshot and order/position review for the existing execution surface
- partial-fill, stale-open-order, and delta-submission clarity

Out of scope:

- portfolio-level order management
- multi-broker reconciliation
- options live trading
- multi-symbol live orchestration

## Current User-Facing Behavior

The current execution surface already refreshes orders and positions, and the live runner currently targets the delta between desired position and broker-confirmed current position.

What is still thin is the explicit reconciliation model:

- open orders, current position, and intended action are visible, but not yet synthesized into one operator-facing state
- optimistic order rows are useful, but they are not the same thing as a canonical reconciliation view
- partial fills or stale open orders can make the operator mentally reconcile too much of the state by hand

## Current Code Surface

- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- targeted Rust tests in `src-tauri/src/commands_execution.rs`

## Strengths Already Present

- broker snapshot and order review already exist
- the current order-submission path already uses delta-to-target logic
- the frontend already distinguishes optimistic rows from refreshed broker state
- the current scope is narrow enough that reconciliation can stay explicit and operator-readable

## Weaknesses / Failure Modes

- there is no single reconciliation summary for desired versus broker state
- partial fills and stale open orders can leave the operator unsure whether the next action is safe
- current refresh actions help, but they still leave interpretation largely manual
- a discrete live mode needs clearer broker-state truth than the current review-only posture provides

## Regressions To Avoid

- masking broker uncertainty behind overconfident reconciliation summaries
- collapsing optimistic local events into broker-confirmed truth
- adding smart order behavior that exceeds the current Alpaca/live-mode operating envelope
- weakening current review-table detail to make reconciliation look simpler
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
- [Orders at Alpaca](https://docs.alpaca.markets/docs/orders-at-alpaca)
- [Tauri state management](https://v2.tauri.app/develop/state-management/)

## Proposed Improvement Approach

1. Treat reconciliation as a first-class discrete live-mode truth layer.
2. Build an explicit operator-facing state from:
   - desired strategy action
   - broker position
   - open orders
   - most recent signal or runner event
3. Keep provenance explicit so the operator can tell:
   - broker-confirmed
   - optimistic local
   - stale or unknown
4. Start with clarity and review, not automated "smart OMS" behavior.
5. Preserve the current single-symbol and single-broker assumptions while hardening the reconciliation model.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the current reconciliation state model for desired action, broker position, and open orders.
- [x] Define how stale broker snapshot data is represented to the operator.
- [x] Define partial-fill and open-order states that should block or warn before the next action.
- [x] Define how optimistic rows relate to canonical reconciliation state.
- [x] Preserve current delta-submission semantics while improving review clarity.
- [x] Add or strengthen frontend tests around reconciliation summaries, warnings, and stale-state presentation.
- [x] Add or strengthen Rust tests around position/order snapshot interpretation and safe delta handling.
- [x] Define rollback conditions if the new reconciliation layer becomes less truthful than the current explicit review tables.

## Verification Plan

- rerun targeted execution service, store, and panel tests near touched files
- rerun targeted Rust tests in `src-tauri/src/commands_execution.rs`
- rerun `npm.cmd --prefix web run test:browser:ux` if reconciliation summaries or warnings become user-visible in the live shell
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
- capture concrete acceptance evidence such as reconciliation screenshots, stale-state examples, and test output proving optimistic versus broker-confirmed separation

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only during typed-model rollout.
- Final validation must include targeted unit/integration tests for symbol scoping, freshness ownership, order derivation, invalidation rules, and reconciliation rendering, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for the visible reconciliation surface, plus a paper/live-mode manual smoke path proving the final model does not overclaim broker truth or widen submission risk.
- Final validation must include performance evaluation for any touched broker-refresh, reconciliation-build, or review-render path so the final state does not regress operator responsiveness.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Execution Notes

- Reconciliation is now a first-class review layer instead of an implied reading of the raw broker table:
  - `web/src/stores/executionStore.ts` derives explicit reconciliation states and advisories from broker freshness, open orders, optimistic rows, and position state
  - `web/src/components/ExecutionWorkspacePanel.vue` renders those summaries and warnings without weakening the existing order-table detail
  - `web/src/services/executionService.ts` continues to preserve broker-confirmed versus optimistic provenance instead of collapsing them into one status
- The implementation intentionally preserved the current delta-submission backend semantics:
  - no new smart OMS behavior was introduced
  - the operator-facing change is clarity, not hidden order-management automation
- This epic closed for the current single-symbol Alpaca-backed review scope:
  - reconciliation now surfaces stale/open/flat/pending posture directly
  - the detailed broker order table remains the fallback truth surface
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run a11y:smoke`
  - `npm.cmd --prefix web run test:desktop:smoke`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`

## Rollout / Fallback / Compatibility Notes

- start with additive reconciliation summaries and warnings before changing order-submission behavior
- preserve the current review tables as a detailed fallback even if a higher-level reconciliation summary is added
- if a reconciliation state cannot be made trustworthy, keep the current manual-refresh/operator-review model for that case
- do not widen this plan into portfolio OMS or multi-broker state aggregation

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `06_live_execution_operator_safety_controls_2026-04-11.md`
- `07_live_execution_session_audit_trail_2026-04-11.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
