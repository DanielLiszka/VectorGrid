# Local Paper Brackets And Broker Watcher

Status: proposed
Risk: high
Primary value: make execution rehearsal more realistic while preserving safety

## Summary

Local paper mode currently warns or blocks when bracket child orders are required because local child-order simulation is not implemented. Alpaca reconciliation is also intentionally short-lived until a longer order/fill watcher exists. This task adds deterministic local paper bracket simulation and a safer broker watcher in separate phases.

## Current Anchors

- Execution command and policy handling: `src-tauri/src/commands_execution.rs`
- Local paper broker: `src-tauri/src/broker_paper.rs`
- Execution frontend service/store/UI: `web/src/services/executionService.ts`, `web/src/stores/executionStore.ts`, `web/src/components/ExecutionWorkspacePanel.vue`
- Existing implementation plan: `docs/paper_live_trading_improvement_plan.md`

## Additive Boundary

- Keep broker/live execution disabled unless user credentials and existing readiness gates pass.
- Start with local paper only.
- Preserve current "unsupported" block when `require_bracket=true` until child orders are actually simulated.
- Do not route local paper bracket behavior into live broker orders.
- Do not change quantity sizing semantics in the same slice unless required for correctness.

## Non-Goals

- No automatic strategy deployment.
- No real-money execution changes in the local paper phase.
- No broker-specific advanced order types outside the explicit bracket model.

## Implementation Checklist

### Phase 1: Bracket Contract

- [ ] Define local paper order states for parent, stop-loss child, take-profit child, cancel-linked sibling, filled, cancelled, rejected, and expired.
- [ ] Define how bracket children attach to long and short entries.
- [ ] Define fill ordering for bars where both stop and take-profit are touched.
- [ ] Define deterministic tie-break policy and expose it in diagnostics.
- [ ] Add request/response fields only if existing policy fields are insufficient.
- [ ] Add tests for no-bracket behavior to prove unchanged output.
- [ ] Require both take-profit limit and stop-loss stop prices for a full bracket.
- [ ] Reject or warn on extended-hours bracket simulation unless a later slice defines explicit support.
- [ ] Restrict simulated bracket time-in-force to `day` or `gtc` unless a later slice widens support.
- [ ] Define how partial take-profit fills reduce the remaining stop-loss child quantity.
- [ ] Define nested parent/legs response shape for local paper inspection.

### Phase 2: Local Paper Child-Order Simulation

- [ ] Add bracket child-order storage to the local paper broker state.
- [ ] Create child orders only after parent fill.
- [ ] Cancel sibling child when one child fills.
- [ ] Cancel remaining open group orders when any group order is cancelled.
- [ ] Apply stop/take-profit fills deterministically using available price path information.
- [ ] Persist enough local paper state to inspect parent/child links in UI.
- [ ] Keep unsupported warning for broker/provider modes that do not use the local simulator.

### Phase 3: Execution UI Review

- [ ] Show parent/child order relationships in execution review.
- [ ] Show bracket policy, fill assumption, and tie-break behavior.
- [ ] Show a clear warning when bracket settings are configured but not attached.
- [ ] Add dry-run previews for bracket behavior.
- [ ] Keep require-bracket failures visible and blocking until fully supported.

### Phase 4: Broker Watcher Foundation

- [ ] Add an idempotent watcher state model for external broker order ids and client order ids.
- [ ] Poll account/orders/fills with cancellation and backoff.
- [ ] Reconcile partial fills, filled orders, cancelled orders, and rejected orders.
- [ ] Reconcile independent order records and nested bracket/OCO legs into one local group model.
- [ ] Never place replacement orders automatically in the first watcher slice.
- [ ] Add stale watcher diagnostics and stop conditions.
- [ ] Keep watcher disabled by default for live broker paths until tested.

### Phase 5: Alpaca-Specific Watcher Integration

- [ ] Add Alpaca watcher adapter behind existing credential/readiness gates.
- [ ] Normalize broker statuses into the shared watcher model.
- [ ] Add idempotency checks for duplicate submit/retry attempts.
- [ ] Add fixture-backed tests for broker responses.
- [ ] Add UI diagnostics for last sync, pending orders, and mismatches.

## Validation Checklist

- [ ] `cargo test -p ta_desktop_demo_app broker_paper`
- [ ] `cargo test -p ta_desktop_demo_app commands_execution`
- [ ] `npm --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts`
- [ ] Add local paper bracket tests for parent fill, stop fill, take-profit fill, sibling cancel, tie-break, and require-bracket.
- [ ] Add watcher tests with mocked broker responses for partial fill, cancel, reject, stale, and duplicate idempotency cases.
- [ ] Run the repo quality gate with desktop typecheck before handoff.

## Regression Watchpoints

- Existing no-bracket local paper execution must remain byte-for-byte or semantically compatible where practical.
- Live broker submission must not inherit local simulation shortcuts.
- Cancellation must work during watcher polling.
- No secrets may be written to artifacts or logs.

## Definition Of Done

- Local paper can simulate bracket parent/child lifecycle deterministically.
- The UI exposes bracket assumptions and resulting child-order state.
- Broker watcher foundation is opt-in, cancellable, and idempotent.
- Tests cover safety-critical lifecycle cases.
