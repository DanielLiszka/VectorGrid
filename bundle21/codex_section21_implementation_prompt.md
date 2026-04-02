# Codex implementation prompt — Section 21 live task/event throughput and user-visible run responsiveness

## Mission

Implement only the remaining repo-valid Section 21 work for live task/event throughput and user-visible run responsiveness.

Keep the first wave focused on:

1. mounting the existing task/event instrumentation in the actual app runtime
2. making the mounted task instrumentation authoritative for the app’s real invoke path without flooding task history
3. reducing duplicate listener fanout and no-op runtime state churn
4. making cancellation/live-runner state reach user-visible footer/task-center/execution surfaces promptly

This is not a shell redesign and not a websocket/reconnect rewrite.

---

## Read first

Before editing code, read:

1. `bundle21/section21_live_task_event_throughput_and_user_visible_run_responsiveness_plan.md`
2. `gpt54_feature_review_map.md` section `## 21. Live task/event throughput and user-visible run responsiveness`
3. `REPO_MAP.md`
4. `web/src/stores/taskStore.ts`
5. `web/src/tauri/instrumentation.ts`
6. `web/src/tauri/invoke.ts`
7. `web/src/stores/executionStore.ts`

Trust the current code and the repo-state corrections below over older review wording.

---

## Repo-state validation and scope corrections

### Already true in this repo

- [x] `web/src/stores/taskStore.ts` already provides bounded active/history/event/timeline state, debounced persistence, and throttles for progress, live-runner, Polygon status, and Polygon bar event fanout.
- [x] `web/src/stores/taskStore.lifecycle.test.ts` already covers queued/running/cancelling/completed posture behavior.
- [x] `web/src/stores/taskStorePerformance.test.ts` already covers high-volume progress handling.
- [x] `web/src/stores/taskStoreEventThrottle.test.ts` already covers repeated live-runner / Polygon status event coalescing.
- [x] `web/src/services/runControlService.ts` already requests both `cancel_current_run` and `cancel_background_task`, with legacy fallback to `cancel_double_ma_optimization`.
- [x] `web/src/services/executionService.ts` already owns runtime-event parsing, direct execution event subscriptions, lean broker snapshot flags, and optimistic order-row behavior.
- [x] `web/src/stores/executionStore.ts` already owns the execution workspace’s live/runner view state, per-action pending flags, and runtime-event reconciliation.
- [x] `src-tauri/src/commands_execution.rs` already owns live-runner lifecycle, generation-safe replacement, pending-bar coalescing, cancellation checks, and runner event emission.
- [x] `src-tauri/src/commands_data.rs` already owns Polygon live status/bar emission and live stream start/stop/status command handling.
- [x] `src-tauri/src/state.rs` already owns the live-stream task slot and background-task cancellation token state.
- [x] `web/src/components/RunFooterPanel.vue` and `web/src/components/TaskCenterOverlay.vue` already consume store-backed task posture; they do not need a first-wave redesign.
- [x] `web/src/tauri/instrumentation.ts` already exists and is already covered by focused tests.
- [x] `web/src/tauri/invoke.ts` is the actual production invoke surface used by the app’s services and stores.

### Repo-valid scope corrections

- [x] The current Tauri owner of live Polygon event emission is `src-tauri/src/commands_data.rs`, not `src-tauri/src/polygon.rs`.
- [x] The current user-visible execution responsiveness path includes `web/src/stores/executionStore.ts`, not just `executionService.ts`.
- [x] Broad backend live-runner correctness work from earlier sections is already landed; Section 21 should not reopen it without new evidence.
- [x] The existing bounded `taskStore.ts` plus backend bar/run coalescing already cover much of the raw throughput concern; a large store redesign is not the first-wave answer.

### Still-open repo-valid work

- [x] Mount `web/src/tauri/instrumentation.ts` from a real app bootstrap path so task lifecycle and task/event channels actually populate in the shipped runtime.
- [x] Make the mounted instrumentation authoritative for the real `web/src/tauri/invoke.ts` / `invokeCommand(...)` path the app actually uses.
- [x] Add a narrow tracked-command policy so the mounted runtime path tracks real workflow and execution lifecycle commands without flooding shared task history with low-signal status probes.
- [x] Keep the instrumentation install idempotent and HMR-safe so it does not create duplicate invoke patches or duplicate listeners.
- [x] Add shared per-channel listener ownership in `web/src/tauri/events.ts` so multiple consumers of the same runtime event do not create duplicate native listeners.
- [x] Centralize optimistic cancellation posture updates through shared task-store / run-control logic instead of leaving `TaskCenterOverlay.vue` as the only place that marks tasks `cancelling`.
- [x] Add narrow no-op guards in `web/src/stores/executionStore.ts` so repeated identical live-runner / Polygon status events do not trigger unnecessary reactive rewrites.
- [x] Verify cancellation/run-state posture is truthful in footer/task-center surfaces once runtime instrumentation is actually mounted.
- [x] Add only the targeted tests needed to cover the mounted instrumentation path, shared listener ownership, and the new equality/no-op guards.

### Explicit defer / exclude items

- [x] Do not use this section for a shell redesign, task-center redesign, or run-footer redesign.
- [x] Do not rewrite Polygon websocket architecture, reconnect/backoff policy, or `src-tauri/src/polygon.rs` cache/download ownership.
- [x] Do not replace `taskStore` persistence with a new storage engine.
- [x] Do not unify `taskStore.ts` and `executionStore.ts` into one giant store in the first wave.
- [x] Do not introduce a new combined runtime-status command unless the existing commands prove insufficient.
- [x] Do not add stopwatch-style latency instrumentation or new benchmark harnesses unless current test/evidence surfaces prove insufficient.
- [x] Do not reopen earlier bundle work around live runner replacement, symbol matching, broker snapshot slimming, or general execution UX redesign unless new evidence appears.

---

## File scope

### Primary implementation files

- `web/src/main.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/invoke.ts`
- `web/src/tauri/events.ts`
- `web/src/stores/taskStore.ts`
- `web/src/stores/executionStore.ts`
- `web/src/services/runControlService.ts`
- `web/src/services/executionService.ts`

### Primary verification files

- `web/src/stores/taskStore.lifecycle.test.ts`
- `web/src/stores/taskStoreEventThrottle.test.ts`
- `web/src/stores/taskStorePerformance.test.ts`
- `web/src/tauri/events.test.ts`
- `web/src/tauri/invoke.mockIPC.test.ts`
- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/tauri/instrumentation.mockIPC.test.ts`
- `web/src/tauri/errorPathQa.test.ts`
- `web/src/services/runControlService.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`

### Inspect-only / conditional files

- `web/src/components/RunFooterPanel.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/RunFooterPanel.test.ts`
- `web/src/components/TaskCenterOverlay.test.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_optimize.rs`

### Keep unchanged unless evidence forces it

- current backend live-runner lifecycle semantics
- current Polygon live stream command surface
- current bounded `taskStore` persistence shape

---

## Hard constraints

- Keep the first wave additive.
- Prefer mounting and tightening the existing instrumentation path over inventing a second task lifecycle mechanism.
- Treat identical repeated runtime events as a no-op when user-visible state does not change.
- Keep CPU-only builds and non-Tauri test contexts working.
- Do not turn this into a broad frontend architecture refactor.

### Ownership guard

Treat these as the current owners:

- `web/src/tauri/instrumentation.ts`
  - generic invoke lifecycle wrapping and task/event subscriptions
- `web/src/tauri/invoke.ts`
  - actual production Tauri invoke boundary used by the app
- `web/src/tauri/events.ts`
  - shared Tauri event subscription boundary
- `web/src/stores/taskStore.ts`
  - generic task lifecycle, timeline, toast, and bounded task/event history
- `web/src/stores/executionStore.ts`
  - execution workspace live/runner user-visible state
- `src-tauri/src/commands_execution.rs`
  - live runner lifecycle and runner event payloads
- `src-tauri/src/commands_data.rs`
  - Polygon live stream status/bar payloads and live stream lifecycle

---

## Implementation checklist

### Phase 1 — runtime wiring

- [x] Install `installTauriInstrumentation()` from a real app bootstrap path after Pinia/runtime binding is available.
- [x] Make the task lifecycle instrumentation authoritative for the real `invokeCommand(...)` path the app uses.
- [x] Add a narrow tracked-command policy so the mounted runtime path skips low-signal probes while still tracking real workflow and execution lifecycle commands.
- [x] Keep instrumentation install idempotent across re-entry, HMR, and non-Tauri test contexts.
- [x] Verify the installed runtime path still degrades cleanly when Tauri APIs are unavailable.

### Phase 2 — task/event throughput

- [x] Preserve the current bounded/throttled `taskStore.ts` fanout behavior.
- [x] Add shared per-channel listener ownership in `web/src/tauri/events.ts`.
- [x] Ensure `web/src/tauri/instrumentation.ts` and `web/src/services/executionService.ts` share one underlying native listener per overlapping event channel.
- [x] Centralize optimistic cancellation posture updates through shared run-control / task-store logic.
- [x] Touch `web/src/stores/taskStore.ts` only for the narrow compatibility and cancellation-propagation guards needed once the instrumentation path is active.

### Phase 3 — execution status churn

- [x] Add only the missing no-op guards needed for repeated identical live/runner status events in `web/src/stores/executionStore.ts`.
- [x] Preserve optimistic order-row insertion for real execution payloads.

### Phase 4 — user-visible run responsiveness

- [x] Make footer/task-center task posture truthful once invoke lifecycle tracking is mounted in the real runtime.
- [x] Keep cancellation request feedback immediate without inventing a second authoritative state path.
- [x] Keep execution workspace live/runner status responsive without forcing manual refresh churn after each runtime event.

### Phase 5 — tests / fallout

- [x] Add or adjust instrumentation tests to cover the real mounted runtime path.
- [x] Add or adjust tests for shared listener ownership now that `events.ts` multiplexes native listeners.
- [x] Add or adjust execution-store tests for repeated identical runtime events becoming a no-op.

### Conditional backend follow-up

- [x] Inspect `src-tauri/src/commands_execution.rs` for any remaining purely redundant runner emits after the frontend fixes are in place; no backend change was required in this first wave.
- [x] Inspect `src-tauri/src/commands_data.rs` for any remaining purely redundant Polygon-status emits after the frontend fixes are in place; no backend change was required in this first wave.
- [x] Keep backend event-emission semantics unchanged because current payloads are already sufficient for safe frontend no-op detection.

### Conditional verification

- [x] `npm --prefix web run test -- --run src/components/RunFooterPanel.test.ts src/components/TaskCenterOverlay.test.ts` was not required because the mounted instrumentation and cancellation-posture changes did not alter those component contracts.
- [x] `cargo test -p ta_desktop_demo_app commands_execution` was not required because backend runner-event logic stayed unchanged.
- [x] `cargo test -p ta_desktop_demo_app commands_data` was not required because backend Polygon live-status emission stayed unchanged.
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

## Verification checklist

- [x] `npm --prefix web run test -- --run src/stores/taskStore.lifecycle.test.ts src/stores/taskStoreEventThrottle.test.ts src/stores/taskStorePerformance.test.ts`
- [x] `npm --prefix web run test -- --run src/tauri/invoke.mockIPC.test.ts src/tauri/invoke.contract.test.ts`
- [x] `npm --prefix web run test -- --run src/tauri/instrumentation.test.ts src/tauri/instrumentation.mockIPC.test.ts src/tauri/errorPathQa.test.ts src/tauri/events.test.ts`
- [x] `npm --prefix web run test -- --run src/services/runControlService.test.ts src/services/executionService.test.ts src/stores/executionStore.actions.test.ts`
- [x] `npm --prefix web run typecheck`

---

## Acceptance criteria

- [x] Task lifecycle tracking and event subscriptions are active in the real app runtime, not only in isolated tests.
- [x] Mounted task instrumentation follows the app’s real invoke path and does not flood shared task history with low-signal status probes.
- [x] The task/event path remains bounded and throttled under repeated runtime events.
- [x] Shared overlapping runtime-event consumers no longer create duplicate native listeners.
- [x] `executionStore.ts` no longer performs obvious no-op rewrites for identical repeated live/runner status payloads.
- [x] Footer/task-center task posture becomes truthful for queued/running/cancelling/completed work once commands run through the mounted app.
- [x] Cancellation and live-runner state feel prompt without a broad UI redesign or backend command rewrite.
