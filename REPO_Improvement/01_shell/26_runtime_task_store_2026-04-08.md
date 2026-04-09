# Runtime Task Store Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the task store so tracked-command lifecycle, progress ingestion, event/timeline/toast retention, overall-progress state, and persistence boundaries remain explicit and testable.

## Scope Boundaries

In scope:

- `web/src/stores/taskStore.ts`
- task lifecycle helpers
- progress-event ingestion
- event/timeline/toast/history retention and persistence
- cancel-command mapping

Out of scope:

- overlay presentation details owned by task-center UI docs
- backend command execution logic
- options surfaces

## Current User-Facing Behavior

The task store already acts as the main task/runtime ledger for the shell. Current state families include:

- active tasks
- active order
- recent history
- events
- timeline
- toasts
- `centerStatus`
- `overallProgress`

Current task lifecycle and ingestion behavior includes:

- `beginTrackedCommand()`
- `completeTrackedCommand()`
- `failTrackedCommand()`
- `markTaskCancelling()`
- `restoreTaskRunning()`
- `applyDoubleMaProgress()`
- `applyResearchCampaignProgress()`
- live runner / Polygon status / Polygon bar event recording
- task posture snapshots
- cancellation-command mapping

Persistence and retention are already bounded with explicit limits for:

- history
- events
- timeline
- toasts

and persisted via `TASK_TIMELINE_STORAGE_KEY`.

## Current Code Surface

- `web/src/stores/taskStore.ts`
- `web/src/utils/storage.ts`
- `web/src/stores/runtimePinia.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`

## Current Verification Surface

- `web/src/stores/taskStore.lifecycle.test.ts`
- `web/src/stores/taskStoreEventThrottle.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `web/src/stores/taskStorePerformance.test.ts`
- `web/src/services/runControlService.test.ts`
- `web/src/tauri/errorPathQa.test.ts`

## Strengths Already Present

- the store already centralizes task lifecycle instead of scattering it across features
- progress-event throttling already exists for noisy channels
- explicit retention limits already exist
- persistence is bounded and versioned
- background services can resolve Pinia through `runtimePinia`
- cancel-command mapping is explicit instead of heuristic

## Weaknesses / Failure Modes

- the store owns many concerns at once:
  - lifecycle
  - progress
  - history
  - timeline
  - toasts
  - persistence
- tracked-command authority can drift if related lists are duplicated elsewhere
- persistence, throttling, and cancellation restore paths are subtle enough to regress quietly
- helpers mutate shared singleton state outside component setup, which requires disciplined contract documentation

## Regressions To Avoid

- unbounded growth in history/events/timeline/toasts
- breaking `centerStatus` derivation
- losing progress throttling and causing event spam churn
- desynchronizing cancel-command mappings from tracked command behavior
- breaking runtime Pinia resolution for background-service callers

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because this store is a central shared-state authority with nontrivial action flow
- [Pinia Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because lifecycle transitions and progress ingestion should remain action/helper driven
- [Window.localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because the task center persists history/timeline state into browser storage
- [Tauri: Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/)
  - relevant because progress and live-status updates arrive as Rust-originated frontend events

## Proposed Improvement Approach

1. Split the store contract into explicit state families:
   - active lifecycle
   - recent history
   - event/timeline/toast feeds
   - overall progress
   - persistence
2. Define task lifecycle transitions explicitly:
   - queued
   - running
   - cancelling
   - success
   - failed
   - cancelled
3. Lock retention and persistence behavior as contract items, not implementation details.
4. Document event-throttle policies and why different channels have different intervals.
5. Expand tests around cancellation restore, persistence hydration, and command/posture summary invariants.

## Checklist

- [ ] Document task-store state families and invariants.
- [ ] Document lifecycle transitions and helper ownership.
- [ ] Document retention limits and persistence behavior.
- [ ] Document overall-progress semantics and event-throttle policies.
- [ ] Document cancel-command mapping expectations.
- [ ] Add tests for persistence hydration and bounded retention.
- [ ] Add tests for cancellation restore and terminal-state transitions.
- [ ] Add tests for tracked-command posture snapshots and summary counts.

## Verification Plan

- run `npm test -- src/stores/taskStore.lifecycle.test.ts`
- run `npm test -- src/stores/taskStoreEventThrottle.test.ts`
- run `npm test -- src/stores/taskStoreCampaignProgress.test.ts`
- run `npm test -- src/stores/taskStorePerformance.test.ts`
- run `npm test -- src/services/runControlService.test.ts`
- manually verify task-center, footer, and settings diagnostics after representative runs

## Rollout / Fallback / Compatibility Notes

- preserve the current task-store external behavior even if internals are later split
- keep bounded persistence as a non-negotiable safety rail
- if command lists are unified later, keep the current user-visible lifecycle behavior unchanged first

## Related Docs

- `REPO_Improvement/01_shell/05_shell_task_center_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
