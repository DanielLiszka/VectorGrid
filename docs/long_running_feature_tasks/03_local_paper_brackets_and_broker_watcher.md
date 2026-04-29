# Local Paper Brackets And Broker Watcher

Status: local paper bracket simulation implemented; long-lived live broker watcher deferred
Risk: high
Primary value: make execution rehearsal more realistic while preserving safety

## Summary

Local paper mode now supports deterministic bracket child-order simulation for opening positions when both stop-loss and take-profit settings are present. Alpaca reconciliation remains intentionally short-lived; a longer order/fill watcher should remain a separate live-broker scope because it changes external-broker automation risk.

Implemented slice:

- Local paper parent orders can create linked stop-loss and take-profit child orders.
- Child orders are persisted with parent/group/leg/tie-break metadata.
- Future bar high/low processing fills the first touched child order and cancels the sibling.
- If both stop and take-profit are touched in one bar, the simulator uses `stop_loss_first`.
- Local paper cancel-open-orders and flatten-position paths cancel open local child orders.
- Frontend broker-order review now shows trigger, bracket leg/tie-break, and parent/group linkage.

Deferred live-broker watcher scope:

- No long-lived Alpaca background watcher was added in this slice.
- Existing Alpaca intent idempotency and short post-submit reconciliation remain the active live-broker safety boundary.

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

- [x] Define local paper order states for parent, stop-loss child, take-profit child, cancel-linked sibling, filled, and cancelled.
- [x] Define how bracket children attach to long and short entries.
- [x] Define fill ordering for bars where both stop and take-profit are touched.
- [x] Define deterministic tie-break policy and expose it in diagnostics.
- [x] Add request/response fields only if existing policy fields are insufficient.
- [x] Add tests for no-bracket behavior to prove unchanged output.
- [x] Require both take-profit limit and stop-loss stop prices for a full bracket.
- [x] Keep local paper bracket simulation separate from live/Alpaca bracket behavior.
- [x] Restrict simulated bracket time-in-force to `day` or `gtc` unless a later slice widens support.
- [x] Define first-slice child fills as all-or-none because bar data does not provide child-order partial-fill volume.
- [x] Define parent/group/leg response shape for local paper inspection.

### Phase 2: Local Paper Child-Order Simulation

- [x] Add bracket child-order storage to the local paper broker state.
- [x] Create child orders only after parent fill.
- [x] Cancel sibling child when one child fills.
- [x] Cancel open local paper child orders during local cancel/flatten safety actions.
- [x] Apply stop/take-profit fills deterministically using available bar high/low information.
- [x] Persist enough local paper state to inspect parent/child links in UI.
- [x] Keep local simulator behavior isolated from Alpaca paper/live provider modes.

### Phase 3: Execution UI Review

- [x] Show parent/child order relationships in execution review.
- [x] Show bracket trigger, leg, and tie-break behavior in broker order rows.
- [x] Show a clear warning when only one bracket bound is configured and no full bracket can be attached.
- [ ] Add a dedicated dry-run preview for exact local child-order rows before submit.
- [x] Allow `require_bracket=true` in local paper when both bracket bounds are present and the simulator can attach child orders.

### Phase 4: Broker Watcher Foundation

- [x] Preserve existing idempotent Alpaca intent model for external broker order ids and client order ids.
- [x] Preserve existing short post-submit polling for submitted order status.
- [x] Preserve existing reconciliation handling for partial, filled, cancelled, rejected, timeout, and unknown states.
- [ ] Reconcile independent order records and nested bracket/OCO legs into one local group model.
- [x] Never place replacement orders automatically in the first watcher slice.
- [ ] Add a long-lived cancellable watcher loop with stale diagnostics and stop conditions.
- [x] Keep long-lived watcher disabled for live broker paths until separately tested.

### Phase 5: Alpaca-Specific Watcher Integration

- [x] Keep Alpaca submit/reuse/reconcile behavior behind existing credential/readiness gates.
- [x] Normalize broker statuses into the existing reconciliation event model.
- [x] Add idempotency checks for duplicate submit/retry attempts.
- [x] Preserve existing fixture/mock-backed tests for submit/retry/reconcile behavior.
- [ ] Add long-lived watcher UI diagnostics for last sync, pending orders, and mismatches.

## Validation Checklist

- [x] `cargo test -p ta_desktop_demo_app --lib broker_paper -- --test-threads=1`
- [x] `cargo test -p ta_desktop_demo_app --lib commands_execution -- --test-threads=1`
- [x] `npm --prefix web run test -- executionService executionStore.actions ExecutionWorkspacePanel`
- [x] Add local paper bracket tests for parent fill, stop fill, sibling cancel, tie-break, and cancel-open-orders.
- [x] Existing watcher/idempotency tests cover partial fill, terminal statuses, duplicate submit avoidance, and unresolved-intent fail-closed behavior.
- [x] Run the repo quality gate with desktop typecheck before handoff.

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
