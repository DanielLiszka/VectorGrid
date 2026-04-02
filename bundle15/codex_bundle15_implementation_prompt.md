# OpenAI Codex Implementation Prompt — Bundle 15: Live Execution, Polygon Streaming, and Runtime-Control Responsiveness

## Purpose

Use this prompt for the validated `bundle15` implementation scope only.

The original plan and addendum are only partially current. This file is the repo-grounded tracker and implementation prompt that should be followed for Bundle 15 work in this repository.

## Authoritative inputs

Read these first, in this order:

1. `live_execution_polygon_runtime_control_addendum.md`
2. `live_execution_polygon_runtime_control_improvement_plan.md`
3. `REPO_MAP.md`
4. `AGENTS.md`

When the markdown plan disagrees with the current repo, trust the repo. This prompt records those repo-state overrides.

## Repo-state validation

- [x] The bundle is still directionally valid for the current repo.
- [x] The active backend ownership is `src-tauri/src/commands_execution.rs` for live execution and broker snapshots, plus `src-tauri/src/commands_data.rs` for Polygon live-stream commands, with `src-tauri/src/state.rs` only as adjacent runtime state.
- [x] The active frontend ownership is `web/src/services/executionService.ts`, `web/src/stores/executionStore.ts`, `web/src/services/runControlService.ts`, and their existing tests.
- [x] The current Tauri command surface is already established; Bundle 15 does not require a new command by default.
- [x] `PolygonLiveCandleStreamStatusResponse` includes `stream` on the backend, and the frontend now types and consumes it for runtime truth.
- [x] `AppState::live_candle_stream_info()` clears finished websocket-task handles, and the registration-failure path now aborts newly spawned websocket tasks instead of leaking them.
- [x] `src-tauri/src/commands_data.rs` emits websocket lifecycle events such as `connecting`, `subscribed`, `closed`, and `error`, and now also emits explicit manual `started` and `stopped` status events.
- [x] `ExecuteStrategySignalResponse` already includes `delta_qty`; the old plan item to add it on the backend is stale.
- [x] `src-tauri/src/commands_execution.rs` now uses generation-safe runner ownership, cancellation state, and one-slot latest-bar coalescing instead of symbol-only identity and unconditional busy-drop behavior.
- [x] `src-tauri/src/commands_execution.rs` now uses a shared timed blocking `reqwest` client and slimmer broker snapshot fetch flags instead of always fetching the full account/orders/positions set.
- [x] `src-tauri/src/commands_execution.rs` and `src-tauri/src/commands_data.rs` now compare symbols with punctuation-preserving canonical helpers, so `BRK.B` and `BRKB` no longer collapse.
- [x] `web/src/services/executionService.ts` now consumes `stream` metadata from live status, no longer forces an immediate broker refresh after non-dry-run `runOnce()`, and no longer fakes runtime shutdown inside `clearKeys()`.
- [x] `web/src/stores/executionStore.ts` now uses per-action pending state, derived `isBusy`, and keeps stop/status controls responsive during unrelated work.
- [x] `web/src/services/runControlService.ts` now overlaps the background cancellation probe with the primary or legacy cancel path.
- [x] The repo already has relevant tests in `web/src/services/executionService.test.ts`, `web/src/services/runControlService.test.ts`, `web/src/stores/executionStore.actions.test.ts`, and `src-tauri/src/commands_execution.rs`; a brand-new store test file is optional, not a prerequisite for coverage to exist.
- [x] `web/src/tauri/events.ts` already has a usable `listenEvent(...)` helper; a bulk event-helper rewrite is optional, not required.

## Active implementation scope

Only the unchecked items below are in active implementation scope.

### Backend runner correctness and live-stream lifecycle

- [x] `src-tauri/src/commands_execution.rs`: runner generation and cancellation state now make stop and same-symbol restart authoritative, and stale in-flight tasks no longer mutate the current runtime after replacement.
- [x] `src-tauri/src/commands_execution.rs`: one-slot latest-bar coalescing now replaces unconditional `skipped_busy` drop behavior.
- [x] `src-tauri/src/commands_execution.rs`: cancellation checks now bracket expensive runner execution stages and the pre-submit boundary so stop wins whenever it arrives early enough.
- [x] `src-tauri/src/commands_execution.rs`: stop and stale-generation paths no longer collapse pre-submit cancellation into generic runner errors; stop/stale replacement remains the authoritative visible outcome.
- [x] `src-tauri/src/commands_data.rs` plus `src-tauri/src/commands_execution.rs`: manual live stop now also stops the runner, and symbol-changing live replacement tears down the prior runner during the replacement path.
- [x] `src-tauri/src/commands_data.rs` and adjacent `src-tauri/src/state.rs`: if live-stream registration fails after spawn, the spawned websocket task is now aborted and the failure is returned without leaking a running task.
- [x] `src-tauri/src/commands_data.rs`: bounded connect, auth-send, and subscribe-send timeouts now guard live websocket startup.
- [x] `src-tauri/src/commands_data.rs`: explicit manual `started` and `stopped` live-status events now complement the existing websocket lifecycle events.

### Symbol correctness and snapshot latency

- [x] `src-tauri/src/commands_execution.rs` and `src-tauri/src/commands_data.rs`: punctuation-stripping symbol match helpers are replaced with shared canonical comparison that trims whitespace, uppercases, and preserves meaningful punctuation such as `.`, `-`, `_`, `/`, and `:`.
- [x] `src-tauri/src/commands_execution.rs`: symbol-filtered order refresh now scans deeply enough before client-side filtering so truncated results are not silently presented as if they were complete.
- [x] `src-tauri/src/commands_execution.rs`: a shared timed Alpaca HTTP client helper is now reused across one-shot execution, connection test, and broker snapshot refresh paths.
- [x] `src-tauri/src/commands_execution.rs`: broker snapshot refresh now supports smaller optional fetch flags for the frontend and uses the single-symbol position endpoint when a symbol is present.

### Frontend execution service and store responsiveness

- [x] `web/src/services/executionService.ts`: the backend `stream` payload from `polygon_live_candle_stream_status` is now typed and parsed, status hints surface symbol plus delayed/live metadata, and `startRunner()` rejects mismatched active live-stream symbols.
- [x] `web/src/services/executionService.ts`: `refreshOrders()` now uses the slimmer snapshot request shape, with `include_account=false` and only the remaining small additive flags needed by the current UI.
- [x] `web/src/services/executionService.ts`: the fake runtime reset is removed from `clearKeys()`, so credential clearing no longer forces live or runner state to off.
- [x] `web/src/services/executionService.ts`: the immediate post-submit broker snapshot in `runOnce()` is replaced with a lower-latency path that still shows truthful optimistic order feedback.
- [x] `web/src/services/executionService.ts` plus the existing events helper: a runtime event subscription helper now covers `polygon_live_status` and `live_execution_runner_event`.
- [x] `web/src/stores/executionStore.ts`: the single global busy gate is replaced by per-action pending state, with `isBusy` retained only as a derived compatibility field.
- [x] `web/src/stores/executionStore.ts`: `stopLive`, `stopRunner`, and status refresh remain usable while other execution actions are pending.
- [x] `web/src/stores/executionStore.ts`: explicit live-stream identity such as `liveSymbol` and `liveDelayed` is now tracked, and `canStartRunner` derives from pending state plus matching live/runner symbol truth.
- [x] `web/src/stores/executionStore.ts`: live and runner state now reconcile from runtime events instead of relying solely on manual refreshes.
- [x] `web/src/stores/executionStore.ts`: runtime truth remains separate from credential state, so `clearKeys()` clears credentials and output only.
- [x] `web/src/services/runControlService.ts`: the best-effort background cancellation probe now overlaps the primary or legacy cancel path while preserving fallback semantics when commands are unavailable.

### Tests and verification prep

- [x] `web/src/services/executionService.test.ts`: coverage now includes live-symbol mismatch rejection, slimmer snapshot request flags, truthful `clearKeys()` behavior, and the new `runOnce()` responsiveness path.
- [x] `web/src/services/runControlService.test.ts`: nth-call ordering assumptions are removed and the tests now assert concurrent/background cancellation semantics instead.
- [x] `web/src/stores/executionStore.actions.test.ts` and/or a new `web/src/stores/executionStore.test.ts`: coverage now includes per-action pending behavior, stop/status responsiveness during another action, and runtime event reconciliation.
- [x] `src-tauri/src/commands_execution.rs` tests: deterministic helper-level coverage now exercises canonical symbol matching, cancellation/generation gating, and pending-bar coalescing.
- [x] `src-tauri/src/commands_data.rs` tests: focused coverage now exercises punctuation-preserving live symbol normalization.

## Explicit exclusions and deferred items

These items were mentioned in the original plan or addendum but are not part of the active mandatory implementation scope for the current repo state.

- [x] Do not require a new combined `execution_runtime_status` command.
- [x] Do not require merging `upsert_live_bar(...)` and `get_candles(...)`.
- [x] Do not require deterministic broker `client_order_id` idempotency in this bundle.
- [x] Do not require suspended or terminal runner health classification beyond truthful current event and status reporting.
- [x] Do not require a `requested` / `connected` / `subscribed` / `streaming` phase model beyond the current event vocabulary unless it falls out trivially from the mandatory work.
- [x] Do not broaden `src-tauri/src/polygon.rs` into a reconnect/backoff or `Retry-After` rewrite for this bundle.
- [x] Do not require `src-tauri/src/commands_system.rs` or other desktop-runtime files; this bundle stays in the existing live-execution and live-stream surfaces.
- [x] Do not require command-registry changes or `web/src/tauri/commands.ts` regeneration unless a new command is unexpectedly added.
- [x] Do not require `web/src/tauri/events.ts` bulk-helper work; the existing `listenEvent(...)` helper is already sufficient if reused.
- [x] Do not treat the addendum's authoritative backend `symbol <-> data_id` guard as mandatory in the current repo state. `AppState::LoadedDataContext` does not currently carry trustworthy dataset-symbol metadata for generic CSV loads, so enforcing that contract now would require a broader data-loading expansion than this bundle warrants.
- [x] Do not require `src-tauri/src/state.rs` cleanup for finished websocket tasks unless a new registration-failure code path needs it; `live_candle_stream_info()` already clears finished handles today.
- [x] Do not treat backend `stream` support on `polygon_live_candle_stream_status()` as missing; only frontend consumption is outstanding.
- [x] Do not treat backend `delta_qty` on `ExecuteStrategySignalResponse` as missing; only optional frontend use is outstanding.
- [x] Do not add a new benchmark harness or mandatory `ta_workflow_regression.rs` expansion for this bundle.

## Adjacent inspect-only surfaces

Inspect these as needed, but do not expand scope unless a tiny supporting change becomes strictly necessary.

- `src-tauri/src/state.rs`
  - for live-stream task ownership and registration-failure cleanup
- `web/src/components/ExecutionWorkspacePanel.vue`
  - only if a tiny mount-lifecycle follow-up is needed for runtime subscription ownership
- `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs`
  - only if the command surface unexpectedly changes
- `web/src/tauri/events.ts`
  - only if a tiny helper addition is cleaner than keeping subscriptions inside `executionService.ts`

## Implementation guardrails

- Preserve the current execution workflow surface and existing strategy coverage.
- Keep changes local and parity-safe; avoid broker-feature expansion or broad runtime rewrites.
- Prefer the smallest repo-fitting implementation when the original plan proposes multiple shapes.
- Keep manual refresh as the authoritative broker-truth action even if optimistic UI feedback is added.
- If optimistic order rows are added, label them in a way that does not imply confirmed broker fill state.
- If an optional item starts expanding scope, defer it and record it rather than forcing it into the patch.

## Verification checklist

- [x] `cargo test -p ta_desktop_demo_app commands_execution`
- [x] `cargo test -p ta_desktop_demo_app commands_data`
- [x] `npm --prefix web run test -- --run src/services/executionService.test.ts src/services/runControlService.test.ts src/stores/executionStore.actions.test.ts`
- [x] No new store test file was added, so no extra targeted Vitest file needed to be included.
- [x] `npm --prefix web run typecheck`
- [x] The command surface did not change unexpectedly, so `npm --prefix web run check:commands` was not required for this bundle.
- [x] `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` was executed. The run was non-green under external machine load, so treat it as an environment-qualified perf read rather than a functional bundle blocker.

## Final response requirements for the implementation run

When implementing this bundle, the final response should:

- say whether Bundle 15 is fully complete for the validated scope
- list the actual files changed
- state which verification commands were run and which passed
- call out any intentionally deferred optional items that were left untouched
- mention any unrelated pre-existing warnings or failures that are not bundle blockers
