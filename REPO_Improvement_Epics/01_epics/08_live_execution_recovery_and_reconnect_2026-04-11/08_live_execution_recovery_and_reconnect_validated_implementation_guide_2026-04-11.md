# Live Execution Recovery And Reconnect Validated Implementation Guide

Last updated: 2026-04-11

Companion to:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md`

## Purpose

Freeze the validated recovery and reconnect decisions for the current narrow live implementation before any code changes begin.

## Restart Truth Model

After app relaunch:
- `AppState` is fresh
- no live-stream slot is assumed active
- the process-local runner runtime is gone
- broker state is unknown until a fresh broker snapshot is pulled

Local inactive does not mean broker flat.

## Owner Map

- Feed lifecycle truth:
  - `src-tauri/src/state.rs`
  - `src-tauri/src/commands_data.rs`
- Runner runtime truth:
  - `src-tauri/src/commands_execution.rs`
- Broker truth:
  - `get_live_execution_snapshot` in `src-tauri/src/commands_execution.rs`
- Operator-state projection:
  - `web/src/services/executionService.ts`
  - `web/src/stores/executionStore.ts`

## Four-Domain Recovery Matrix

The recovery model must track four domains separately:

- feed lifecycle
- runner state
- broker freshness/truth
- strategy dataset / `data_id` validity

No implementation may collapse these into one generic `connected` or `active` state.

## Feed Lifecycle Contract

Feed lifecycle states for v1:
- `starting`
- `connecting`
- `subscribed`
- `closed`
- `error`
- `stopped`

`active=true` is not enough to imply healthy review-ready feed state.

## Non-Manual Interruption Contract

For any non-manual feed interruption:
- websocket `closed`
- websocket `error`
- stream ended / `None`

The runner must not silently remain broker-armed.

The v1 policy is conservative:
- non-manual interruption forces a review-required inactive posture
- future bars are ignored until the operator explicitly re-arms
- same-symbol feed restart must not auto-resume the prior runner

## Broker Freshness Contract

Broker freshness comes only from:
- `GetLiveExecutionSnapshotResponse.fetched_at_unix_ms`

The recovery surface must distinguish:
- `broker_unknown`
- `broker_stale`
- `broker_fresh_flat`
- `broker_fresh_nonflat`

`refreshStatuses()` is not broker reconciliation and must never be presented as equivalent.

## Dataset / `data_id` Contract

Re-arm is blocked when:
- `data_id` is missing
- the backing dataset is missing after relaunch
- loaded dataset identity no longer matches the requested symbol
- a required derived dataset was invalidated by live-bar mutation and has not been re-established

## Event Contract

Bundle `08` must not depend on a frontend-only `cancelled` runner event as recovery truth.

For the first slice:
- rely on explicit backend/stored state for inactive/disarmed posture
- do not introduce broad new event vocabulary unless tests and docs are updated together

## Boundary To Bundle 07

Bundle `08` owns:
- transient review cues
- restart/reconnect semantics
- re-arm rules

Bundle `07` owns:
- durable saved session evidence

## Preconditions

Before code changes start, freeze these choices in the parent bundle doc:

- non-manual interruption always enters a review-required inactive posture
- no same-symbol restart may silently resume the prior runner
- broker freshness must be modeled explicitly and separately from local runtime state
- dataset / `data_id` validity is a first-class recovery domain

## Ordered Implementation Slices

1. Restart truth slice
   - make relaunch semantics explicit in backend and UI state
2. Interruption contract slice
   - handle `closed`, `error`, and stream-ended paths conservatively
3. Freshness and dataset slice
   - add broker freshness handling
   - add dataset / `data_id` validity checks to re-arm posture
4. UI recovery slice
   - surface `unknown`, `stale`, `review required`, and inactive states explicitly
5. Regression hardening slice
   - lock scenario tests for interruption, relaunch, and re-arm blocking

## File-Level Change Plan

- `src-tauri/src/commands_data.rs`
  - apply the non-manual interruption contract
  - preserve explicit distinction between manual stop and interruption
- `src-tauri/src/commands_execution.rs`
  - enforce runner disarm/review-required behavior after interruption
  - keep broker freshness accessible to the frontend
- `src-tauri/src/state.rs`
  - keep feed-slot behavior explicit and bounded to current single-stream scope
- `web/src/services/executionService.ts`
  - model broker freshness and restart truth distinctly from local live/runner status refresh
- `web/src/stores/executionStore.ts`
  - store recovery posture as explicit typed state, not just simplified hints
- `web/src/components/ExecutionWorkspacePanel.vue`
  - render recovery posture and re-arm requirements without implying broker certainty

## Acceptance Evidence

Required evidence:

- same-symbol restart after feed `error` does not silently resume the prior runner
- same-symbol restart after feed `closed` does not silently resume the prior runner
- relaunch lands in `local inactive` plus `broker unknown` until broker refresh
- stale broker snapshot is visibly distinct from fresh broker truth
- missing or invalid `data_id` blocks re-arm

## Closure Conditions

Do not mark the bundle complete until:

- interruption handling is conservative in both backend and frontend
- relaunch posture does not overclaim broker knowledge
- broker freshness uses the backend timestamp source only
- dataset / `data_id` validity is enforced in the re-arm path
- review cues stay inside the current narrow single-stream/single-runner scope

## Verification Requirements

Named scenario coverage is required for:
- feed `error` then same-symbol restart
- feed `closed` then same-symbol restart
- relaunch with broker still unknown
- stale broker snapshot after status refresh
- missing `data_id`
- invalidated dataset or derived-data mismatch
- no-auto-rearm assertions after interruption

Run:
- targeted Rust tests in `commands_data.rs`, `commands_execution.rs`, and `state.rs`
- targeted frontend tests in `executionService.test.ts`, `executionStore.actions.test.ts`, and `ExecutionWorkspacePanel.test.ts`
- `npm.cmd --prefix web run test:browser:ux` when recovery state becomes user-visible

## Rollback Rule

If reconnect UX becomes less truthful than the current manual-refresh posture, fall back to explicit inactive-after-interruption behavior and keep richer recovery logic out of the shipped path.
