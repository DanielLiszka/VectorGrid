# Paper And Live Trading Improvement Plan

Last updated: 2026-04-24

## Purpose

This document turns the current paper/live trading audit into an implementation plan.

The goal is to move VectorGrid from a conservative execution prototype into a safer,
more testable trading subsystem with:

- deterministic order sizing
- a stateful local paper broker
- stronger Alpaca order reconciliation
- backend-authoritative risk limits
- deterministic mock-broker and mock-feed verification
- clearer operator recovery workflows

This plan is intentionally implementation-facing. It should be updated as work lands.

## Implementation Status

Landed on 2026-04-24:

- Deterministic 1e-4 quantity normalization in backend and frontend request shaping.
- Backend execution policy evaluator with explicit caps for max order notional,
  max position notional, short exposure, and required brackets.
- JSON-backed local paper broker state under app-local data with immediate market fills,
  orders, fills, positions, account cash, and realized/unrealized PnL.
- Local paper execution routing through `execute_strategy_signal_once` and the live
  runner when `execution_target=local_paper`.
- Local paper snapshot, reset, cancel-open-orders, and flatten commands.
- Frontend execution target control: `Dry-run`, `Local paper`, `Alpaca paper`,
  and `Alpaca live`.
- Targeted Rust and frontend execution tests for the shipped slice.

## Current State Summary

Current live execution is real but intentionally narrow:

- Strategy coverage is limited to `DoubleMa` and `CompositeCpu` execution payloads.
- Dry-run computes the latest desired signal but does not persist a simulated broker account.
- Paper and live broker modes use Alpaca REST.
- Live automation is triggered by Polygon stock minute aggregate bars.
- Only one live stream and one live execution runner can be active at a time.
- Broker-armed execution requires Alpaca credentials, while dry-run does not.
- Live Alpaca automation is blocked on delayed Polygon streams.
- Open-order guards prevent a new broker delta when existing open orders conflict.
- Safety controls exist for current-symbol open-order cancellation, position flattening,
  and local emergency halt.
- Live-session evidence artifacts are persisted for broker-armed execution and redact
  sensitive payloads.

Known issues and gaps:

- Backend quantity quantization currently has a failing unit test around midpoint
  rounding at 1e-4 boundaries.
- Dry-run is signal-only rather than a stateful paper-trading simulator.
- Alpaca orders are submitted optimistically and require manual refresh for broker truth.
- There is no `client_order_id` idempotency strategy.
- There is no order/fill watcher for partial fills, terminal states, or timeout handling.
- Risk limits are mostly operator-facing UI posture, not a backend policy engine.
- There is no deterministic mock Alpaca/Polygon integration harness.
- Live execution still reuses drilldown request shapes instead of strategy-local execution
  adapters.

## Guiding Principles

1. Broker-facing behavior must be backend-authoritative.
   Frontend confirmations are useful, but backend commands must enforce final safety rules.

2. Paper trading should model broker state, not only signal intent.
   A useful paper mode needs positions, cash, open orders, fills, fees, slippage, and
   session evidence.

3. Live submission should be idempotent and recoverable.
   Any retryable network or app restart path must avoid duplicate broker orders.

4. Reconciliation should prefer broker truth over local optimism.
   Optimistic UI rows are acceptable only as temporary placeholders.

5. Execution code should become strategy-local where practical.
   Shared execution plumbing should stay shared, while signal adapters and live support
   rules should live near each strategy family.

6. Verification should not require real money, real credentials, or live vendor uptime.
   Mock broker/feed tests should cover most safety-critical logic.

## Target Architecture

### Execution Layers

Recommended long-term shape:

1. `ExecutionIntent`
   - Strategy-local desired-position output.
   - Contains symbol, target exposure, confidence/provenance, last evaluated bar, and
     strategy identity.

2. `ExecutionPolicy`
   - Backend risk-policy evaluation before any order.
   - Produces allow/block decisions with machine-readable reasons.

3. `BrokerAdapter`
   - Common trait for live Alpaca, Alpaca paper, and local paper broker.
   - Owns account snapshot, orders, positions, submit/cancel/flatten, and reconciliation.

4. `ExecutionSession`
   - Durable session state and audit events.
   - Records lifecycle, decisions, submitted orders, fills, policy blocks, broker snapshots,
     and recovery requirements.

5. `Runner`
   - Feed-triggered orchestration.
   - Should consume bars, evaluate strategy, evaluate policy, submit/reconcile through
     the broker adapter, and persist every material event.

### Proposed Backend Modules

Keep existing files stable while moving toward this shape incrementally:

- `src-tauri/src/commands_execution.rs`
  - Current Tauri command boundary.
  - Keep command names stable.
  - Initially host new policy and adapter calls, then split large internals into modules.

- `src-tauri/src/execution_policy.rs`
  - New backend-authoritative risk-policy rules.
  - Pure logic with unit tests.

- `src-tauri/src/execution_quantity.rs`
  - New deterministic decimal-like quantity normalization helpers.
  - Shared by manual execution, runner execution, local paper broker, and tests.

- `src-tauri/src/broker_alpaca.rs`
  - New Alpaca REST adapter once `commands_execution.rs` becomes too large.
  - Owns URL construction, request payloads, response normalization, and idempotency.

- `src-tauri/src/broker_paper.rs`
  - New local stateful paper broker.
  - Owns account state, fills, order book, and persistence.

- `src-tauri/src/execution_session.rs`
  - Optional future split for live-session artifact creation/update/finalization.

- `web/src/services/executionService.ts`
  - Frontend request normalization and Tauri command invocation.

- `web/src/stores/executionStore.ts`
  - Operator state, availability, recovery posture, and broker-review presentation.

- `web/src/components/ExecutionWorkspacePanel.vue`
  - Execution controls, safety matrix, review surfaces, and saved evidence display.

## Phase 0: Fix Deterministic Quantity Handling

### Problem

Order quantity handling rounds to 1e-4 units in both frontend and backend. The backend
unit test for midpoint behavior currently fails for `0.00015`, returning `0.0001`
instead of the expected `0.0002`.

This is likely floating-point midpoint representation, not a trading strategy issue.
It still matters because quantity normalization sits directly before broker submission.

### Implementation

1. Add a deterministic quantity normalization helper.
   - Accept `f64` for current command compatibility.
   - Convert through a stable decimal representation before rounding.
   - Prefer integer micro-units or string-based decimal parsing rather than `x * 10000.0`.
   - Reject non-finite, zero, negative, and too-small-after-quantization values.

2. Use the helper everywhere execution quantities are normalized.
   - Backend request normalization.
   - Flatten/order payload helpers if needed.
   - Local paper broker once added.

3. Keep frontend and backend validation aligned.
   - Frontend can continue normalizing for UX.
   - Backend remains source of truth.

### Acceptance Criteria

- `0.00005` normalizes to `0.0001`.
- `0.000149` normalizes to `0.0001`.
- `0.00015` normalizes to `0.0002`.
- Tiny quantities below the minimum still fail clearly.
- Rust execution unit tests pass.

## Phase 1: Add A Stateful Local Paper Broker

### Problem

Current dry-run is signal-only. It tells the operator what would happen, but it does
not behave like a broker account. That limits repeatable testing, runner confidence,
and operator learning.

### Product Semantics

Add a new local broker mode that is separate from Alpaca paper:

- `dry_run=true`
  - No state mutation.
  - Signal preview only.
  - No broker credentials.

- `broker=local_paper`
  - Mutates a local simulated account.
  - No external broker credentials.
  - Produces paper orders, fills, positions, and session evidence.

- `broker=alpaca_paper`
  - Existing Alpaca paper API.

- `broker=alpaca_live`
  - Existing Alpaca live API.

The frontend can keep the current `mode` and `dryRun` controls initially, but the
backend model should move toward an explicit broker target to avoid ambiguity.

### Paper Broker State

Persist under app-local data, for example:

- `app_local_data_dir/execution/paper_broker/state.json`
- `app_local_data_dir/execution/paper_broker/sessions/<session_id>.json`

Suggested account fields:

- account id
- currency
- starting cash
- current cash
- buying power
- portfolio value
- realized PnL
- unrealized PnL
- updated timestamp

Suggested position fields:

- symbol
- quantity
- average entry price
- market price
- market value
- unrealized PnL
- last fill timestamp

Suggested order fields:

- local order id
- optional client order id
- symbol
- side
- quantity
- order type
- time in force
- submitted timestamp
- status
- filled quantity
- average fill price
- stop/take-profit linkage when simulated bracket orders exist

Suggested fill fields:

- fill id
- order id
- symbol
- side
- quantity
- price
- fee
- slippage
- timestamp

### Fill Model

Start simple and explicit:

- Market orders fill at the latest available live bar close.
- Optional slippage can be configured in basis points.
- Optional fixed or percent commission can be configured.
- Bracket orders can be simulated after entry fills:
  - If both stop-loss and take-profit are present, create linked child orders.
  - Evaluate child triggers against subsequent bar high/low.
  - If both stop and take-profit are crossed in one bar, use a documented conservative
    ordering rule.

### Commands

Add backend commands after the core model exists:

- `paper_broker_get_state`
- `paper_broker_reset_state`
- `paper_broker_get_snapshot`
- `paper_broker_list_orders`
- `paper_broker_cancel_open_orders`
- `paper_broker_flatten_position`

Then route existing execution commands through the paper broker when local paper is active.

### UI Updates

Add visible distinction between:

- Signal dry-run
- Local paper broker
- Alpaca paper broker
- Alpaca live broker

The execution workspace should show:

- active broker target
- whether the action will mutate local state or external broker state
- paper account cash/equity
- paper position
- paper open orders
- reset-paper-account confirmation

### Acceptance Criteria

- Running the same signal repeatedly in local paper mode changes simulated position only
  when the target position changes.
- Local paper orders and fills appear in the order review table.
- Local paper sessions are saved as execution evidence.
- Local paper reset is explicit and confirmed.
- No Alpaca credentials are required for local paper broker mode.
- Existing dry-run remains non-mutating.

## Phase 2: Add Alpaca Reconciliation And Idempotency

### Problem

Current Alpaca submission returns an order id, then the UI marks an optimistic local row
and asks the operator to refresh broker state. This is a reasonable prototype pattern,
but live trading needs stronger broker truth and retry behavior.

### Idempotency

Generate and persist a `client_order_id` for every broker-facing order.

Recommended format:

```text
vg_<session_id>_<bar_open_unix_ms>_<strategy_kind>_<symbol>_<intent_hash>
```

Rules:

- Build from stable session and intent data.
- Persist before submitting.
- On retry, query Alpaca for the same client order id before submitting again.
- Treat an existing order with matching client id as the authoritative submission.

### Order Watcher

Add a reconciliation loop after each broker submission:

- Poll order by id/client id until terminal state or timeout.
- Track partial fills.
- Refresh position after terminal fill/cancel/reject.
- Persist each transition as a session event.

Initial terminal states:

- filled
- canceled
- expired
- rejected
- replaced

Intermediate states:

- new
- accepted
- pending_new
- partially_filled
- pending_cancel
- pending_replace
- held
- stopped
- suspended
- calculated

### Runner Behavior

The runner should not blindly submit new deltas while a previous broker order is
unresolved unless policy explicitly allows it.

Recommended initial policy:

- If an order for the current symbol is open and reconciles to the target, do not submit.
- If an order is open and does not reconcile to the target, block and require manual review.
- If order metadata is incomplete, block and require manual review.
- If a prior submission has unknown final state, block further broker submissions for
  that symbol until refreshed.

### UI Updates

Show order lifecycle states distinctly:

- submitted
- accepted
- partial fill
- filled
- canceled
- rejected
- stale/unknown

Replace generic optimistic text with:

- `Local optimistic: awaiting broker confirmation`
- `Broker confirmed: open`
- `Broker confirmed: filled`
- `Broker state unknown: refresh required`

### Acceptance Criteria

- A broker-facing retry does not duplicate an Alpaca order if the prior request actually
  reached Alpaca.
- Runner blocks on unresolved conflicting open orders.
- UI distinguishes optimistic state from broker-confirmed state.
- Session evidence records submit, reconcile, fill, reject, and timeout events.

## Phase 3: Backend Risk Policy Engine

### Problem

Current safety controls are useful, but risk is not centralized. Live order approval
should go through a backend policy engine before any non-dry-run order.

### Policy Inputs

The policy engine should evaluate:

- broker target
- symbol
- side
- requested quantity
- current position
- open orders
- account equity/cash/buying power
- latest market price
- live feed status
- broker snapshot freshness
- strategy kind
- parameter source
- source run fingerprint
- validation/holdout context when available
- session counters

### Initial Policy Rules

Add conservative defaults:

- Block if symbol is blank.
- Block if quantity is invalid after backend normalization.
- Block live Alpaca automation on delayed feed.
- Block live submission if broker snapshot is stale beyond threshold.
- Block if account status is not tradable/active.
- Block if buying power is unavailable for broker-armed buy orders.
- Block if projected notional exceeds `max_order_notional`.
- Block if projected symbol exposure exceeds `max_symbol_notional`.
- Block if daily submitted order count exceeds `max_daily_orders`.
- Block if session error count exceeds `max_session_errors`.
- Block if previous action requires broker review.
- Block if source run fingerprint is missing for non-manual broker-armed execution.
- Block if validation context is missing for live mode unless operator explicitly enables
  a separate advanced override.

### Policy Configuration

Persist policy in app settings:

- `max_order_notional`
- `max_symbol_notional`
- `max_daily_orders`
- `max_session_errors`
- `broker_snapshot_stale_ms`
- `allow_live_without_validation`
- `allowed_symbols`
- `blocked_symbols`
- `allow_short`
- `allow_fractional_qty`
- `require_brackets_for_opening_live_positions`

Defaults should be conservative. Live mode should start with small limits.

### Commands

Suggested commands:

- `get_execution_risk_policy`
- `set_execution_risk_policy`
- `reset_execution_risk_policy`
- `preview_execution_policy_decision`

The preview command lets the frontend show operator-facing blocked reasons before the
actual submit path, but the submit path must evaluate the same policy again.

### Acceptance Criteria

- Every non-dry-run execution path calls the policy engine before submission.
- Policy decisions are serialized into session evidence.
- UI preflight reasons match backend policy block reasons.
- Live mode can be configured conservatively without code changes.

## Phase 4: Feed And Runner Hardening

### Problem

The runner currently processes Polygon live bars and coalesces pending bars. It disarms
on feed interruption, which is good. It still needs stronger data freshness and market
session semantics.

### Improvements

1. Track feed heartbeat.
   - Record last message time and last accepted bar time.
   - Mark feed stale if no message arrives within a configured threshold.

2. Track bar completeness.
   - Polygon minute aggregates may update over time.
   - Ensure runner only triggers on accepted bar semantics, or document that it acts on
     the aggregate event as received.

3. Add market-hours awareness.
   - At minimum, block live broker automation outside regular session unless explicitly
     allowed.
   - Later, support extended-hours policy intentionally.

4. Persist runner decisions for dry-run and local paper.
   - Dry-run runner sessions should still produce non-sensitive evidence.
   - Local paper runner sessions should preserve simulated order/fill state.

5. Add recovery states.
   - `feed_interrupted`
   - `broker_unknown`
   - `policy_blocked`
   - `order_timeout`
   - `manual_review_required`

### Acceptance Criteria

- Runner refuses to arm when feed state is stale.
- Runner records policy blocks as first-class events.
- Dry-run runner session evidence is available for review.
- Feed interruption requires explicit recovery before re-arming broker submission.

## Phase 5: Strategy-Local Execution Adapters

### Problem

Live execution currently reuses drilldown request shapes. This is convenient but makes
execution eligibility and strategy semantics less explicit.

### Proposed Shape

Introduce a strategy-local adapter trait or equivalent functions:

```text
StrategyExecutionAdapter
  - strategy_kind
  - supported_for_live_execution
  - build_execution_request
  - evaluate_execution_intent
  - risk_metadata
```

Start with:

- `DoubleMa`
- `CompositeCpu`

Then extend only when the strategy has:

- deterministic latest-signal semantics
- tested drilldown/latest-intent parity
- clear long/short behavior
- enough validation evidence to justify execution
- frontend support copy and blocked reasons

### Acceptance Criteria

- Execution support matrix is code-backed, not only copy-backed.
- Unsupported strategies fail closed with clear reasons.
- Adding a new live-supported strategy does not require reading all of
  `commands_execution.rs`.

## Phase 6: Mock Broker And Mock Feed Harness

### Problem

Most live/paper safety behavior should be testable without real Alpaca/Polygon services.

### Mock Broker

Add an in-process or local HTTP mock broker that supports:

- account fetch
- position fetch
- order list
- order submit
- order cancel
- configurable fills, partial fills, rejections, and delayed terminal states

Test scenarios:

- happy-path buy fill
- happy-path sell/flatten fill
- partial fill followed by fill
- submit timeout but order exists by client id
- submit network failure before order exists
- rejected order
- open order conflict
- incomplete open-order metadata
- cancel failure after partial cancellation

### Mock Feed

Add deterministic feed events:

- normal increasing minute bars
- duplicate bar timestamps
- out-of-order bars
- feed close
- feed error
- delayed-feed live-mode rejection

### Verification Commands

Add one lightweight targeted script:

```powershell
powershell -File scripts/run_execution_safety_gate.ps1
```

The gate should run:

- Rust execution unit tests
- Rust paper-broker tests
- Rust mock broker/feed integration tests
- frontend execution service/store/component tests

### Acceptance Criteria

- Safety-critical behavior is covered without live credentials.
- The execution gate is documented in `REPO_MAP.md`.
- The gate can run without CUDA.

## Phase 7: UX And Operator Workflow Improvements

### Improvements

1. Rename modes to reduce ambiguity.
   - `Signal dry-run`
   - `Local paper`
   - `Alpaca paper`
   - `Alpaca live`

2. Add an arming workflow for live mode.
   - Live broker submission requires a time-limited arming action.
   - Arming expires after a short period or after any feed/broker interruption.
   - Arming state is visible in the posture cards.

3. Add risk policy editor.
   - Keep defaults conservative.
   - Show effective limits in the execution workspace.
   - Require confirmation for loosening live limits.

4. Add recovery checklist.
   - After feed interruption, order timeout, or emergency halt, show exactly what must be
     reviewed before re-arming.

5. Add saved session comparison.
   - Compare dry-run/local-paper/broker-armed sessions by symbol, strategy, result,
     order count, fill count, and policy blocks.

### Acceptance Criteria

- Operators can tell at a glance whether an action is non-mutating, local-state-only,
  paper broker, or live broker.
- Live submission cannot be triggered accidentally by simply unchecking dry-run.
- Recovery requirements are concrete and persistent until satisfied.

## Phase 8: Documentation And Release Readiness

### Documentation

Update:

- `REPO_MAP.md`
  - execution module map
  - execution safety gate
  - supported broker targets

- `README.md`
  - high-level trading disclaimers and setup

- `web/README.md`
  - frontend execution test commands

- `AGENTS.md`
  - only if new execution invariants become repo policy

### Release Checklist

Before release with expanded trading features:

- Verify no secrets persist in logs, local storage, run artifacts, or session artifacts.
- Verify local paper state reset is explicit.
- Verify live mode defaults are conservative.
- Verify mock execution safety gate passes.
- Verify manual broker recovery path is documented.
- Verify app close guard still blocks closing while a live runner is active.

## Comprehensive Checklist

Use this checklist as the implementation tracker. Check items only after code, tests,
and documentation have landed.

### Phase 0: Quantity Determinism

- [x] Add deterministic backend quantity normalization helper.
- [x] Replace floating-point midpoint rounding in execution request normalization.
- [x] Add tests for boundary quantities: `0.00005`, `0.000149`, `0.00015`, `1.23455`.
- [x] Confirm frontend and backend quantity behavior match.
- [x] Run targeted Rust execution quantity tests.

### Phase 1: Local Paper Broker

- [x] Define local paper broker state schema.
- [x] Add paper account persistence under app-local data.
- [x] Add order, fill, and position models.
- [x] Implement market-order fill model.
- [x] Implement fee/slippage configuration.
- [ ] Implement bracket-order simulation or explicitly defer it behind a clear unsupported status.
- [x] Add paper broker snapshot command.
- [x] Add paper broker reset command.
- [x] Add local paper cancel/flatten safety commands.
- [x] Route local paper execution through the execution command path.
- [x] Persist local paper session evidence through existing live-session artifacts.
- [x] Add frontend mode copy distinguishing signal dry-run from local paper.
- [x] Add tests for buy, sell, reset, persistence, and execution-target request shaping.
- [ ] Add UI confirmation around paper broker reset before exposing reset prominently.
- [ ] Add explicit local paper account panel.

### Phase 2: Alpaca Reconciliation

- [ ] Add stable `client_order_id` generation.
- [ ] Persist pending submission intent before broker submit.
- [ ] Query by client order id before retry submission.
- [ ] Add order-status polling after submission.
- [ ] Record partial-fill and terminal-state events.
- [ ] Refresh position after terminal order states.
- [ ] Block new broker submissions when prior order state is unresolved.
- [ ] Add UI states for optimistic, broker-open, partial, filled, rejected, and unknown.
- [ ] Add tests for retry without duplicate submission.

### Phase 3: Risk Policy

- [x] Define execution risk-policy config skeleton.
- [ ] Persist policy settings.
- [x] Add policy evaluation module.
- [x] Enforce explicit policy caps before broker submit.
- [ ] Add policy preview command.
- [x] Add policy block outcomes to execution responses and session events.
- [ ] Add frontend risk-policy summary.
- [ ] Add frontend blocked reasons from policy preview.
- [x] Add tests for max notional, no-short, bracket-required, and invalid price policy blocks.
- [ ] Add tests for stale broker snapshot, account status, open order conflict, live delayed feed, and validation-required live mode.

### Phase 4: Feed And Runner Hardening

- [ ] Track last feed message timestamp.
- [ ] Track last accepted bar timestamp.
- [ ] Add feed stale threshold.
- [ ] Block runner arming on stale feed.
- [ ] Add market-hours policy check.
- [ ] Persist dry-run runner session evidence.
- [ ] Persist local paper runner session evidence.
- [ ] Add recovery states for interruption, policy block, order timeout, and broker unknown.
- [ ] Add tests for duplicate, stale, out-of-order, and interrupted feed events.

### Phase 5: Strategy-Local Adapters

- [ ] Define strategy execution adapter shape.
- [ ] Move DoubleMa execution intent construction behind adapter.
- [ ] Move CompositeCpu execution intent construction behind adapter.
- [ ] Add explicit unsupported-strategy response shape.
- [ ] Add code-backed execution support matrix.
- [ ] Add tests that unsupported strategies fail closed.
- [ ] Document adapter ownership in `REPO_MAP.md`.

### Phase 6: Mock Harness

- [ ] Add mock broker implementation.
- [ ] Add mock feed implementation.
- [ ] Add mock broker tests for fills, partial fills, rejects, timeouts, and cancel failures.
- [ ] Add mock feed tests for normal, duplicate, out-of-order, close, and error events.
- [ ] Add `scripts/run_execution_safety_gate.ps1`.
- [ ] Add execution safety gate to `REPO_MAP.md`.
- [ ] Ensure the gate runs without CUDA or real credentials.

### Phase 7: UX

- [x] Rename/clarify execution modes in the UI.
- [ ] Add time-limited live arming flow.
- [ ] Add live arming expiration after feed/broker interruption.
- [ ] Add risk-policy editor or read-only first slice.
- [ ] Add recovery checklist UI.
- [ ] Add local paper account panel.
- [ ] Add saved execution-session comparison view.
- [ ] Add accessibility tests for new controls and recovery states.

### Phase 8: Release Readiness

- [ ] Update `REPO_MAP.md`.
- [ ] Update user-facing setup docs.
- [ ] Update frontend testing docs.
- [ ] Add release checklist entries for paper/live trading.
- [ ] Verify no secrets persist in artifacts or logs.
- [ ] Verify close guard still blocks active live runner shutdown.
- [x] Run frontend execution tests.
- [x] Run targeted Rust execution tests.
- [ ] Run execution safety gate.

## Suggested Implementation Order

1. Fix quantity normalization.
2. Add backend risk-policy preview and enforcement skeleton.
3. Add local paper broker state and snapshot commands.
4. Route manual local paper execution through the paper broker.
5. Add local paper runner persistence.
6. Add Alpaca idempotency and order watcher.
7. Add mock broker/feed integration tests.
8. Refactor strategy-local execution adapters.
9. Polish UX naming, arming, and recovery flows.

This order keeps safety fixes early, gives the app a testable non-live execution target
before deeper Alpaca automation, and avoids coupling the paper broker to a large strategy
adapter refactor.
