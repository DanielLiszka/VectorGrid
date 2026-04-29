# Local Paper Bracket Preview And Broker Observation Foundation

Status: phase 1 preview implementation in progress; watcher foundation remains planned
Risk: high if it mutates broker state or live broker behavior; moderate for pure preview/reporting
Primary value: show exact local paper bracket assumptions before submit and prepare a read-only order observation model

## Summary

The existing local paper broker can create deterministic bracket child orders after an opening parent fills. The remaining safe feature is a preview path that shows the would-be child legs before submit without changing `paper_broker/state.json`, and a read-only observation foundation that can later group broker parent/child/OCO records.

This plan deliberately does not add a long-lived Alpaca watcher. Alpaca documents bracket orders as linked entry, take-profit, and stop-loss orders where child exits activate after the parent fills; it also notes nested child legs can be fetched with `nested=true`. That supports a grouping/observation model, but not an automatic background loop in the same slice.

Sources:

- Alpaca order and bracket semantics: https://docs.alpaca.markets/docs/orders-at-alpaca
- Alpaca WebSocket order updates: https://docs.alpaca.markets/docs/websocket-streaming

## Current Anchors

- Backend execution request/response: `src-tauri/src/commands_execution.rs`
- Local paper broker state machine: `src-tauri/src/broker_paper.rs`
- Frontend execution service/store/UI: `web/src/services/executionService.ts`, `web/src/stores/executionStore.ts`, `web/src/components/ExecutionWorkspacePanel.vue`
- Existing parent plan: `03_local_paper_brackets_and_broker_watcher.md`

## Existing Behavior

- `preview_execution_policy_decision` evaluates signal, desired position, current local paper position, delta, policy, and warnings.
- `execute_strategy_signal_once` creates local paper child bracket orders only during actual local paper submit, only when opening a new position and both stop-loss and take-profit are present.
- Dry-run does not submit orders and currently does not expose child-order rows.
- Alpaca submit uses credential gates, stable client order ids, pre-submit duplicate checks, and short post-submit polling. No long-lived watcher exists.

## Additive Boundary

- Preview must never call local paper submit, bracket advance, save, live broker fetch, cancel, replace, or flatten.
- Preview rows must be labeled as simulated/not submitted.
- Submit-time policy evaluation remains authoritative; preview is advisory.
- Dry-run remains non-stateful unless an explicit future flag requests local paper state.
- Live Alpaca state must not be fetched during preview.
- Watcher foundation is read-only normalization/grouping only.

## Non-Goals

- No live broker order submission changes.
- No background polling loop.
- No automatic cancel/replace/flatten actions.
- No credential persistence or credential logging.
- No attempt to simulate Alpaca partial-fill behavior in local paper beyond the current documented approximation.

## Implementation Checklist

### Phase 1: Backend Preview Contract

- [x] Add serializable `ExecutionBracketPreview` and `ExecutionBracketPreviewLeg` structs.
- [x] Include symbol, target, current position, delta, entry side, simulated entry price, time-in-force, tie-break, and child legs.
- [x] Attach optional `bracket_preview` to `PreviewExecutionPolicyDecisionResponse`.
- [ ] Attach optional `bracket_preview` to `ExecuteStrategySignalResponse` for dry-run responses only when useful.
- [x] Use the existing `local_paper_bracket_spec_for_order` helper so preview and submit stay aligned.
- [x] Add warnings when one bound is missing, the target is not opening a position, or price is invalid.
- [x] Ensure local paper preview uses current local paper position but does not advance existing brackets.

### Phase 2: Frontend Preview Presentation

- [x] Parse optional `bracket_preview` in `executionService.ts`.
- [x] Add a compact preview summary to `previewExecutionRiskPolicy` status text.
- [ ] Show preview legs as "preview only" rather than broker-confirmed order rows.
- [x] Preserve 10 broker-order table columns through the store normalization path.
- [ ] Keep broker-review freshness unchanged for preview results.

### Phase 3: Observation Foundation

- [ ] Define a pure broker order observation shape for parent id, group id, leg, status, trigger, quantity, and source.
- [ ] Add grouping helper tests for independent parent/child records and nested leg-like inputs.
- [ ] Normalize statuses without triggering actions.
- [ ] Keep this helper disconnected from live polling until a separate watcher plan is approved.

### Phase 4: Future Watcher Design Gate

- [ ] Define cancellable lifecycle, stale thresholds, retry backoff, and stop conditions.
- [ ] Define credential handling and redaction rules.
- [ ] Define how WebSocket `trade_updates` and REST reconciliation interact.
- [ ] Define operator-visible mismatch states before any automatic action is considered.

## Validation Checklist

- [ ] `powershell -File scripts/run_execution_safety_gate.ps1`
- [ ] `cargo test -p ta_desktop_demo_app --lib commands_execution -- --test-threads=1`
- [ ] `cargo test -p ta_desktop_demo_app --lib broker_paper -- --test-threads=1`
- [ ] `npm --prefix web run test -- executionService ExecutionWorkspacePanel`
- [ ] `npm --prefix web run typecheck`
- [ ] Add or update tests proving preview does not mutate `paper_broker/state.json`.
- [ ] Add tests for long opening, short opening, missing one bound, non-opening delta, and local paper current-position use.

## Regression Watchpoints

- Preview must not mark broker review as fresh.
- Preview must not create optimistic submitted rows.
- Store normalization must not drop existing trigger/bracket/parent columns.
- Alpaca preview must keep warning that submit will re-check broker state.
- Local paper submit semantics must remain unchanged.

## Definition Of Done

- Users can inspect local paper bracket child-leg preview before submit.
- Preview and actual local paper submit use the same price derivation helper.
- Preview is clearly non-mutating and non-authoritative.
- The code has a read-only broker observation foundation for future watcher work.
