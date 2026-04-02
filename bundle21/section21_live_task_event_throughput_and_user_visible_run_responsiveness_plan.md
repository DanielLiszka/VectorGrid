# Section 21 implementation plan — live task/event throughput and user-visible run responsiveness

## Goal

Improve perceived responsiveness of task status, cancellation, and live runtime updates without turning this section into a shell redesign.

This first wave should stay focused on:

- making the existing task/event instrumentation authoritative in the real app runtime
- keeping listener and store fanout cheap under repeated live runner / Polygon events
- making cancellation posture reach footer/task-center surfaces promptly
- suppressing unnecessary execution-store rewrites for unchanged runtime state

This is not a websocket architecture rewrite and not a general UI redesign pass.

## Repo-grounded current state

### What is already implemented

- `web/src/stores/taskStore.ts` already centralizes generic task lifecycle, event history, timeline, toasts, and shared progress state.
- `web/src/stores/taskStore.ts` already has bounded collections, debounced persistence, and explicit throttles for:
  - `double_ma_progress`
  - `live_execution_runner_event`
  - `polygon_live_status`
  - `polygon_live_bar`
- `web/src/stores/taskStore.lifecycle.test.ts` already covers queued/running/cancelling/completed posture behavior.
- `web/src/stores/taskStorePerformance.test.ts` already exercises high-volume progress throughput.
- `web/src/stores/taskStoreEventThrottle.test.ts` already covers repeated live-runner and Polygon-status coalescing.
- `web/src/tauri/instrumentation.ts` already exists and already has focused test coverage for invoke lifecycle wrapping plus shared task/event sinks.
- `web/src/tauri/invoke.mockIPC.test.ts` and `web/src/tauri/invoke.contract.test.ts` already cover the current invoke wrapper behavior.
- `web/src/tauri/invoke.ts` is the real production invoke boundary used by frontend services and stores.
- `web/src/services/executionService.ts` already owns execution-specific runtime-event parsing and subscription helpers.
- `web/src/stores/executionStore.ts` already owns the execution workspace’s live/runner status, per-action pending flags, optimistic order-row insertion, and runtime-event reconciliation.
- `web/src/services/runControlService.ts` already fans cancellation across `cancel_current_run`, legacy fallback, and `cancel_background_task`.
- `web/src/components/RunFooterPanel.vue` and `web/src/components/TaskCenterOverlay.vue` already consume store-backed task posture; they are not doing the event wiring themselves.
- `src-tauri/src/commands_execution.rs` already provides:
  - generation-safe live-runner replacement
  - latest-bar coalescing while the runner is inflight
  - structured `live_execution_runner_event` emission
  - explicit runner start/stop/status commands
- `src-tauri/src/commands_data.rs` already owns Polygon live websocket start/stop/status commands plus `polygon_live_status` and `polygon_live_bar` emission.
- `src-tauri/src/state.rs` already clears finished live websocket tasks when `live_candle_stream_info()` is queried.

### Where ownership or responsiveness is still split

- `web/src/tauri/instrumentation.ts` is not mounted from the real app bootstrap path, so its generic task lifecycle and task-event feed are not authoritative in the shipped runtime yet.
- even if mounted as-is, the current invoke wrapping logic still targets the legacy global Tauri invoke surface rather than the app’s real `invokeCommand(...)` path in `web/src/tauri/invoke.ts`.
- `web/src/tauri/events.ts` is still only a thin wrapper over Tauri `listen`; it does not provide shared per-channel ownership when multiple consumers subscribe to the same event.
- `web/src/tauri/instrumentation.ts` and `web/src/services/executionService.ts` currently subscribe to overlapping runtime channels:
  - `polygon_live_status`
  - `live_execution_runner_event`
- `web/src/services/runControlService.ts` requests cancellation, but only `web/src/components/TaskCenterOverlay.vue` currently performs optimistic `cancelling` posture updates.
- `web/src/stores/executionStore.ts` still rewrites user-visible status fields on every accepted runtime event even when the normalized visible state is unchanged.
- the review-map section names `src-tauri/src/polygon.rs` as a primary file, but the actual live event owner is `src-tauri/src/commands_data.rs` plus `src-tauri/src/state.rs`.

## Current repo-valid problems

1. The task/event instrumentation exists but is not authoritative in production.
   - nothing in `web/src/main.ts` or `web/src/services/desktopProductization.ts` mounts `installTauriInstrumentation()`
   - the app’s services/stores call `invokeCommand(...)`, so the current legacy-global patch path is not sufficient on its own
2. Cancellation can be backend-fast but still feel frontend-slow.
   - `web/src/services/runControlService.ts` returns request success quickly, but shared task posture is not updated there
   - footer and other non-overlay cancellation paths therefore wait for later lifecycle completion/failure to reflect the state change
3. The same runtime channels can be listened to more than once in the frontend once the instrumentation path is mounted.
   - `web/src/tauri/instrumentation.ts` needs the task/event feed
   - `web/src/services/executionService.ts` needs execution-specific runtime status
   - `web/src/tauri/events.ts` does not yet provide shared per-channel ownership
4. Execution runtime events still cause avoidable store churn.
   - `web/src/stores/executionStore.ts` updates `statusLine`, `statusHint`, runner/live flags, and timestamps on every accepted event
   - repeated `queued_latest`, repeated connection-state messages, or repeated identical live-status payloads can create extra reactive work even when nothing user-meaningful changed
5. Backend event emission is mostly already shaped correctly.
   - `src-tauri/src/commands_execution.rs` already coalesces pending bars instead of queueing unlimited work
   - `src-tauri/src/commands_data.rs` already emits explicit started/connecting/subscribed/closed/error transitions
   - this section should not assume a backend event-contract rewrite unless frontend-side fixes prove insufficient

## First-wave implementation scope

### Phase 1 — authoritative runtime instrumentation wiring

- Mount `installTauriInstrumentation()` from a real app bootstrap path once Pinia/runtime binding exists.
- Make the generic task lifecycle instrumentation authoritative for the app’s real `invokeCommand(...)` path.
- Keep the mounted path idempotent and HMR-safe.
- Keep non-Tauri and mockIPC/test contexts working.
- If mounting the real invoke path would otherwise flood task history with low-signal probes or utility commands, add a narrow tracked-command policy rather than reopening the whole task UI.

### Phase 2 — shared frontend listener ownership

- Upgrade `web/src/tauri/events.ts` from a thin wrapper into a shared per-channel listener owner for repeated subscribers to the same Tauri event channel.
- Keep the exported `listenEvent(...)` shape stable for callers.
- Ensure multiple consumers of:
  - `polygon_live_status`
  - `live_execution_runner_event`
  - `double_ma_progress`
  can share one underlying native Tauri listener per channel.
- Ensure teardown is ref-counted so the native listener is removed only when the last frontend subscriber detaches.
- Update `web/src/tauri/instrumentation.ts` and `web/src/services/executionService.ts` only as needed to rely on that shared wrapper.

### Phase 3 — cancellation posture responsiveness

- Add task-store helpers or a narrow run-control hook that immediately marks relevant active tracked tasks as `cancelling` when the user requests cancellation.
- Use that shared path from `web/src/services/runControlService.ts`.
- Reuse that shared path from `web/src/components/TaskCenterOverlay.vue` instead of keeping a separate direct-cancel logic branch there.
- If the cancellation request itself fails, restore task posture back to `running` with truthful warning messaging.
- Keep cancellation behavior additive:
  - do not remove the existing backend cancel commands
  - do not remove the current legacy fallback path

### Phase 4 — execution-store status churn reduction

- Add normalized no-op guards for live/runner status application in `web/src/stores/executionStore.ts` or `web/src/services/executionService.ts`.
- Skip store mutations when the incoming live-status or runner-status event does not change the user-visible state.
- Preserve meaningful transitions:
  - started/stopped/error/subscribed changes
  - `runnerActive` truthfulness
  - optimistic order-row prepends when a real execution payload arrives
- Prefer normalized signature comparison over broad throttling when possible so real state changes remain immediate.

### Phase 5 — backend follow-up only if evidence demands it

- Inspect `src-tauri/src/commands_execution.rs` and `src-tauri/src/commands_data.rs` for any purely redundant emits that remain noisy after the frontend fixes.
- If a backend change is needed, keep it narrow:
  - suppress only duplicate/no-op churn
  - preserve current event names and payload compatibility
  - keep `src-tauri/src/polygon.rs` out of the core implementation path unless a supporting helper there is directly involved

## Explicit defers

- Do not use this section for a general task-center or footer redesign.
- Do not move unrelated execution UX into this bundle.
- Do not redesign optimization/validation progress wording outside the task/event surfaces touched here.
- Do not add a new combined runtime-status Tauri command in the first wave.
- Do not rewrite `src-tauri/src/polygon.rs` broadly; live-event ownership is in `src-tauri/src/commands_data.rs`.
- Do not broaden the work into Polygon cache/download performance, broker payload redesign, or chart rendering.
- Do not remove existing task timeline/history persistence.
- Do not replace `taskStore` persistence with a new storage engine.
- Do not unify `taskStore.ts` and `executionStore.ts` into one giant store in the first wave.
- Do not add stopwatch-style latency instrumentation or new benchmark harnesses unless the current test/evidence surfaces prove insufficient.

## Concrete implementation checklist

### Runtime instrumentation wiring

- [ ] Mount `installTauriInstrumentation()` from a real app bootstrap path after Pinia/runtime binding is available.
- [ ] Make the task lifecycle instrumentation authoritative for the real `invokeCommand(...)` path the app uses.
- [ ] Add a narrow tracked-command policy only if mounting the real runtime path would otherwise flood shared task history with low-signal probes.
- [ ] Keep the mounted instrumentation idempotent across re-entry, HMR, and non-Tauri test contexts.
- [ ] Add or update focused tests for the real mounted runtime path.

### Shared listener ownership

- [ ] Add shared per-channel listener multiplexing to `web/src/tauri/events.ts`.
- [ ] Keep `listenEvent(...)` API-compatible for existing callers.
- [ ] Ensure `web/src/tauri/instrumentation.ts` and `web/src/services/executionService.ts` share one underlying native listener per event channel.
- [ ] Add or update focused tests for listener sharing and teardown behavior.

### Cancellation responsiveness

- [ ] Add task-store helpers for optimistic `cancelling` posture updates on active cancellable tasks.
- [ ] Wire `web/src/services/runControlService.ts` to mark active tasks as `cancelling` when cancellation is accepted.
- [ ] Reuse that shared path from `web/src/components/TaskCenterOverlay.vue`.
- [ ] Restore task posture to `running` when the cancellation request itself fails.
- [ ] Preserve current command fallback behavior and error wording.

### Execution-store churn reduction

- [ ] Add no-op guards so repeated normalized live-status events do not rewrite `executionStore` state.
- [ ] Add no-op guards so repeated normalized runner events do not rewrite `executionStore` state.
- [ ] Preserve optimistic order-row insertion for real execution payloads.
- [ ] Touch `web/src/components/RunFooterPanel.vue` and `web/src/components/TaskCenterOverlay.vue` only if the new store behavior requires narrow compatibility adjustments.

### Conditional backend follow-up

- [ ] Inspect `src-tauri/src/commands_execution.rs` for any remaining purely redundant runner emits after frontend fixes.
- [ ] Inspect `src-tauri/src/commands_data.rs` for any remaining purely redundant Polygon-status emits after frontend fixes.
- [ ] Make backend event-emission changes only if evidence shows frontend-side fixes are insufficient.

## Verification plan

- `npm --prefix web run test -- --run src/stores/taskStore.lifecycle.test.ts src/stores/taskStorePerformance.test.ts src/stores/taskStoreEventThrottle.test.ts`
- `npm --prefix web run test -- --run src/services/runControlService.test.ts src/services/executionService.test.ts src/stores/executionStore.actions.test.ts`
- `npm --prefix web run test -- --run src/components/RunFooterPanel.test.ts src/components/TaskCenterOverlay.test.ts`
- `npm --prefix web run test -- --run src/tauri/invoke.mockIPC.test.ts src/tauri/invoke.contract.test.ts` if `web/src/tauri/invoke.ts` changes
- `npm --prefix web run test -- --run src/tauri/instrumentation.test.ts src/tauri/instrumentation.mockIPC.test.ts src/tauri/errorPathQa.test.ts`
- `npm --prefix web run test -- --run src/tauri/events.test.ts` if a new shared-listener test file is added
- `npm --prefix web run typecheck`
- `cargo test -p ta_desktop_demo_app commands_execution` only if backend runner-event logic changes
- `cargo test -p ta_desktop_demo_app commands_data` only if backend Polygon live-status emission changes
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` if backend event/cancellation changes could affect workflow-visible timing or live-status control behavior

## Acceptance criteria

- task lifecycle tracking and task-event subscriptions are active in the real app runtime, not only in isolated tests
- mounted task instrumentation follows the app’s real invoke path and does not flood shared task history with low-signal status probes
- repeated frontend consumers of the same Tauri runtime event channel no longer create duplicate native listeners
- cancel requests surface immediate `cancelling` posture in shared task UI and revert truthfully if the request itself fails
- repeated identical runner/live status events do not trigger redundant execution-store writes
- current event names, payload compatibility, task timeline/history behavior, and execution flows remain intact
- the bundle stays on throughput/responsiveness ownership, not on general shell redesign
