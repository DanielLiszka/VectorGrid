# Third-pass consolidation — Live Execution Recovery And Reconnect

Date: 2026-04-11  
Bundle: `08_live_execution_recovery_and_reconnect_2026-04-11.md`

This consolidation validates the earlier first-pass and second-pass critiques against the bundle files and owner files in the packet. Unsupported suggestions from the earlier passes were dropped.

## 1. Surface understanding

The bundle is aimed at the right narrow problem: restart, reconnect, and stale-state hardening for the existing live-execution surface, not a broader trading-platform rewrite. The recovery doc is already explicit that this bundle is about the current live feed and runner, broker snapshot refresh and local runtime reconciliation, and the current single-stream / single-runner model. The umbrella live doc and `REPO_MAP.md` confirm the current surface is narrow: non-options only, Alpaca over REST, Polygon stocks live only, one active live stream slot, one active live runner slot, and an execution panel that still sits in the settings workspace rather than a dedicated live shell.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Purpose, Scope Boundaries, Current User-Facing Behavior, Proposed Improvement Approach
- `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior, Current Verification Surface
- `REPO_MAP.md` — Live execution, settings workspace note
- `lib.rs` — `run_tauri`

The merged conclusion is that the plan direction is correct, but the document is still missing several recovery-critical truths that are already visible in code: non-manual feed interruption does not currently disarm the runner, local runtime restart is not the same thing as broker truth, the runner is keyed to `data_id` and not just symbol, and the current UI/event model still over-compresses launch state, health state, and recovery state. Those are the items that should be fixed in the document before implementation starts.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Weaknesses / Failure Modes, Regressions To Avoid, Proposed Improvement Approach, Checklist
- `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`, `polygon_live_candle_stream_status`
- `commands_execution.rs` — `LiveExecutionRunnerStatus`, `start_live_execution_runner`, `on_live_execution_bar`, `get_live_execution_snapshot`
- `executionService.ts` — `parsePolygonLiveStatusEvent`, `refreshOrders`, `refreshStatuses`
- `executionStore.ts` — initial state, `applyPolygonLiveStatusEvent`, `applyLiveExecutionRunnerEvent`
- `state.rs` — `AppState`, `upsert_live_bar`, `replace_live_candle_stream`, `live_candle_stream_info`

No confirmed stale or nonexistent file reference is visible in the provided packet. The recovery doc’s current code surface names files that are present in the packet, while the related-doc references outside the packet remain unverified rather than provably stale.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current Code Surface, Related Docs
- `CONTEXT_FILE_LIST.txt`

## 2. Consolidated must-fix findings

### MF1. Non-manual feed interruption can leave the runner armed, so same-symbol reconnect can silently resume live behavior

- Status: confirmed doc gap
- Severity: critical
- Type: safety / operator-truth
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach, Checklist, Rollout / Fallback / Compatibility Notes; `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`, `handle_polygon_ws_text`; `commands_execution.rs` — `start_live_execution_runner`, `on_live_execution_bar`

**Why it matters**

The recovery doc says the reconnect model should be conservative, should avoid silent runner restart, and should require explicit operator review after interruption. Current code does not yet enforce that contract for non-manual feed termination. Manual feed stop explicitly stops the runner, and symbol-mismatch stream replacement also stops the runner. But websocket `Close`, stream `Err`, and `None` only emit `polygon_live_status` and break the loop; they do not stop or disarm the runner. At the same time, live bars are forwarded directly into `on_live_execution_bar`, and `on_live_execution_bar` will trigger execution whenever a matching runner slot still exists. That means a same-symbol feed restart after `closed` or `error` can resume the old runner without a fresh Start Runner action.

**Recommended change**

Add an explicit interruption contract to the document. For any non-manual feed termination (`closed`, `error`, stream ended), the plan should require one of two conservative behaviors: either the runner is stopped/disarmed immediately, or it remains present only in a review-required blocked state that ignores future bars until the operator explicitly re-arms it. The doc should name and test both `feed error -> same-symbol restart` and `feed closed -> same-symbol restart`.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach, Checklist, Rollout / Fallback / Compatibility Notes
- `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`
- `commands_execution.rs` — `start_live_execution_runner`, `on_live_execution_bar`

### MF2. The document does not yet separate “local runtime inactive” from “broker state known,” especially after relaunch

- Status: confirmed doc gap
- Severity: critical
- Type: operator-truth / correctness
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current User-Facing Behavior, Checklist, Verification Plan, Rollout / Fallback / Compatibility Notes; `lib.rs` — `run_tauri`; `state.rs` — `AppState::default`; `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`, `GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`; `executionService.ts` — `refreshStatuses`, `refreshOrders`; `executionStore.ts` — initial state

**Why it matters**

On relaunch, Tauri creates a fresh `AppState::default()`, so the live-feed slot starts empty again. The runner runtime is also process-local, but it is not stored in `AppState`; it lives in a module-level `OnceLock<Mutex<Option<...>>>` inside `commands_execution.rs`, so it is lost with process restart as well. The frontend store then boots into local defaults such as `liveActive=false`, `runnerActive=false`, empty order rows, and placeholder position/account hints. None of that means the broker is flat or even known. Broker truth only comes from `get_live_execution_snapshot`, which includes `fetched_at_unix_ms`, and the frontend’s `refreshStatuses()` call does not hit that broker path at all; it only refreshes live and runner state. Treating “local runtime inactive” as if it were “broker known and flat” would be operator-misleading.

**Recommended change**

Add a startup truth model and a broker freshness contract to the document. At minimum, the doc should distinguish `local inactive`, `broker unknown`, `broker stale`, `broker fresh/flat`, and `broker fresh/non-flat`. It should also require a fresh broker snapshot before any re-arm path and explicitly note that `refreshStatuses()` and broker reconciliation are separate operations today.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current User-Facing Behavior, Checklist, Verification Plan, Rollout / Fallback / Compatibility Notes
- `lib.rs` — `run_tauri`
- `state.rs` — `AppState::default`
- `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`, `GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`
- `executionService.ts` — `refreshOrders`, `refreshStatuses`
- `executionStore.ts` — initial state

### MF3. The recovery model still omits the strategy dataset / `data_id` dimension

- Status: confirmed doc gap
- Severity: high
- Type: correctness
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach, Checklist; `commands_execution.rs` — `strategy_data_id`, `start_live_execution_runner`; `state.rs` — `upsert_live_bar`; `REPO_MAP.md` — Data, Polygon, And Live Execution

**Why it matters**

The current recovery plan separates feed, runner, and broker snapshot state, but the runner is not keyed only to a symbol. It also requires a non-empty `data_id`, confirms that dataset exists before runner start, and then uses incoming live bars to mutate that dataset in place. `REPO_MAP.md` explicitly calls out that live bars mutate the loaded dataset by `data_id`, not raw CSV path, and can invalidate derived resamples. `state.rs` shows the same thing in code: `upsert_live_bar` edits the target dataset and removes descendant datasets when the base changes. A reconnect story that does not include dataset identity and validity can still be wrong even when symbol, runner, and broker state all look acceptable.

**Recommended change**

Expand the recovery matrix from three domains to four: feed, runner, broker snapshot freshness, and strategy dataset / `data_id` validity. The doc should block re-arm when `data_id` is missing, when the backing dataset is gone after relaunch, when a derived dataset was invalidated by live mutation, or when dataset identity and requested symbol no longer line up.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach, Checklist
- `commands_execution.rs` — `strategy_data_id`, `start_live_execution_runner`, `LiveExecutionRunnerConfig`
- `state.rs` — `upsert_live_bar`
- `REPO_MAP.md` — Data, Polygon, And Live Execution

### MF4. Current feed/status vocabulary still collapses “launched,” “connected,” and “healthy enough to trust” into one overly simple active state

- Status: confirmed doc gap
- Severity: high
- Type: operator-truth
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Weaknesses / Failure Modes, Regressions To Avoid, Checklist; `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_live_candle_stream_status`; `commands_execution.rs` — `validate_live_runner_stream_preconditions`; `executionService.ts` — `liveStatusHint`, `parsePolygonLiveStatusEvent`, `parseLiveStreamStatus`

**Why it matters**

The backend emits `started` as soon as the task is spawned, before websocket connection and subscription are proven, then later emits `connecting` and `subscribed`. The status command separately reports `active=true` whenever a live-stream slot exists. On the frontend, `parsePolygonLiveStatusEvent()` treats `started`, `connecting`, and `subscribed` as active. Runner start preconditions also rely on the stream being merely active, not demonstrably healthy. That makes the current active/off framing too weak to carry reconnect truth by itself.

**Recommended change**

Make the doc distinguish at least two layers:

1. feed lifecycle (`starting`, `connecting`, `subscribed`, `closed`, `error`, `stopped`), and
2. recovery posture (`armed`, `disarmed`, `review required`).

Do not let the document imply that boolean `active=true` is equivalent to healthy feed readiness.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Weaknesses / Failure Modes, Regressions To Avoid, Checklist
- `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_live_candle_stream_status`
- `commands_execution.rs` — `validate_live_runner_stream_preconditions`
- `executionService.ts` — `liveStatusHint`, `parsePolygonLiveStatusEvent`, `parseLiveStreamStatus`

### MF5. Broker freshness is acknowledged, but the plan still lacks a concrete freshness contract

- Status: confirmed doc gap
- Severity: high
- Type: correctness / operator-truth
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Purpose, Proposed Improvement Approach, Checklist; `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`; `executionService.ts` — `ExecutionSnapshotResponse`, `refreshOrders`, `refreshStatuses`

**Why it matters**

The backend already provides `fetched_at_unix_ms`, which is the obvious broker freshness anchor. But the current frontend snapshot interface omits it, `refreshOrders()` does not surface it, and `refreshStatuses()` does not query broker state at all. The recovery doc says stale-broker-state detection should exist, but it does not specify how freshness is measured, when refresh is mandatory, or how unknown/stale/fresh should be presented. Without that contract, the reconnect surface is at high risk of collapsing “not yet refreshed” into “flat enough.”

**Recommended change**

Make broker freshness first-class in the document:

- define the freshness source (`fetched_at_unix_ms`),
- define stale thresholds,
- define required refresh points after interruption and relaunch,
- define UI wording for broker unknown vs broker stale vs broker fresh.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Purpose, Proposed Improvement Approach, Checklist
- `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`
- `executionService.ts` — `ExecutionSnapshotResponse`, `refreshOrders`, `refreshStatuses`

## 3. Consolidated should-fix findings

### SF1. Recovery ownership should be explicit, because the current packet itself is slightly ambiguous about runner ownership

- Status: confirmed doc gap
- Severity: high
- Type: migration / maintainability
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current Code Surface, Public-repo quality focus; `REPO_MAP.md` — Tauri runtime state, Live execution; `state.rs`; `commands_execution.rs`; `executionService.ts`; `executionStore.ts`

**Why it matters**

The recovery doc correctly wants feed, runner, and broker state kept separate, but it does not map those concerns to actual code owners. `state.rs` clearly owns live-stream state, while `commands_execution.rs` clearly owns the live runner runtime through a module-level static slot. `REPO_MAP.md` simplifies that relationship in a way that can be read as `AppState` owning both. That ambiguity is exactly the kind of ownership drift that makes reconnect work riskier than it needs to be.

**Recommended change**

Add a small owner table:

- feed slot and feed lifecycle truth = `state.rs` + `commands_data.rs`,
- runner runtime truth = `commands_execution.rs`,
- broker truth = `get_live_execution_snapshot` in `commands_execution.rs`,
- operator-state projection = `executionService.ts` + `executionStore.ts`.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current Code Surface, Public-repo quality focus
- `REPO_MAP.md` — Tauri runtime state, Live execution
- `state.rs` — `AppState`, `replace_live_candle_stream`, `live_candle_stream_info`
- `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`, `get_live_execution_snapshot`
- `executionService.ts` — `refreshOrders`, `refreshStatuses`
- `executionStore.ts` — initial state, event application

### SF2. The runner event contract should be made explicit before recovery logic relies on it

- Status: likely issue
- Severity: medium
- Type: maintainability / verification
- Location: `executionService.ts` — `RUNNER_INACTIVE_EVENT_STATUSES`; `REPO_MAP.md` — Live execution; `commands_execution.rs` — runner event emissions

**Why it matters**

The frontend currently treats both `stopped` and `cancelled` as inactive runner events, but `REPO_MAP.md` says the shipped event vocabulary is `started`, `stopped`, `triggered`, `queued_latest`, `signal_processed`, `order_submitted`, and `error`. That mismatch is not the biggest safety problem in this bundle, but it is still a real contract gap. Recovery work should not quietly assume an event that the backend does not actually produce.

**Recommended change**

Decide whether `cancelled` should become a real backend event or should be removed from recovery expectations. Then align the doc, UI logic, and tests around that explicit choice.

Sources:
- `executionService.ts` — `RUNNER_INACTIVE_EVENT_STATUSES`, `parseRunnerEvent`
- `REPO_MAP.md` — Live execution
- `commands_execution.rs` — runner event emissions

### SF3. The verification surface is still underdeclared for the actual recovery risks in this UI surface

- Status: confirmed doc gap
- Severity: medium
- Type: verification / testing
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current Verification Surface, Verification Plan; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface

**Why it matters**

The recovery doc lists generic test surfaces, but the highest-risk cases are scenario-specific: `closed` / `error` feed interruption with the runner still armed, same-symbol feed restart, relaunch with broker state still unknown, `refreshStatuses()` leaving broker truth stale, and dataset invalidation while the runner is active. The umbrella live doc also includes `ExecutionWorkspacePanel.virtualization.test.ts` in the current verification surface for this same execution panel, while the recovery doc omits it. If recovery UI changes affect order rows, banners, or dense state displays, that omission can hide regressions.

**Recommended change**

Name the high-risk scenarios explicitly in the verification plan, and either include `ExecutionWorkspacePanel.virtualization.test.ts` here or state clearly that the planned changes do not affect that surface.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current Verification Surface, Verification Plan
- `05_live_execution_productization_2026-04-10.md` — Current Verification Surface

### SF4. The boundary between recovery-review state and durable session evidence is still fuzzy

- Status: confirmed doc gap
- Severity: medium
- Type: migration / maintainability
- Location: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach item 5; `05_live_execution_productization_2026-04-10.md` — Proposed Improvement Approach, Related Docs

**Why it matters**

The recovery doc says it should capture enough persisted or reviewable evidence for an operator to tell what survived an interruption. The umbrella live doc separately says the broader live productization effort should add a live-session evidence model and also points at a distinct `07_live_execution_session_audit_trail_2026-04-11.md` child bundle. Without a boundary note, bundle 08 can drift into durable artifact ownership instead of staying focused on restart/reconnect semantics.

**Recommended change**

State explicitly whether bundle 08 owns only lightweight recovery-review cues/state or whether durable persisted evidence is a dependency on bundle 07.

Sources:
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach
- `05_live_execution_productization_2026-04-10.md` — Proposed Improvement Approach, Related Docs

## 4. Optional improvements worth considering

### O1. Call out same-symbol feed restart separately from symbol-mismatch replacement

This is a small but high-value wording improvement because the current backend already treats those paths differently: symbol-mismatch replacement stops the runner, while same-symbol restart does not. The plan will stay clearer if it names both cases explicitly instead of talking only about generic mismatch recovery.

Sources:
- `commands_data.rs` — `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Scope Boundaries, Proposed Improvement Approach

### O2. Add one sentence that bundle 08 should not assume a dedicated live shell yet

The current repo still mounts the execution panel under the settings workspace, and the umbrella live doc says surface-promotion decisions belong to a separate bounded slice. A brief note in bundle 08 would help keep reconnect work from quietly assuming bundle 09 has already landed.

Sources:
- `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior, Related Docs
- `REPO_MAP.md` — settings workspace note

### O3. Prefer a small recovery-review state over a broad event-bus expansion if contributor clarity is the priority

If the team wants to keep runner cancellation intentionally silent on the event bus, a minimal review-required state in the store/service layer may be a lower-risk way to surface interruption truth than inventing a wider event contract. That is not mandatory, but it is a good additive option to keep in view.

Sources:
- `REPO_MAP.md` — Live execution
- `executionService.ts` — `refreshStatuses`
- `executionStore.ts` — initial state, event application

## 5. Suggestions to reject as out of scope or unsupported

1. **Do not widen this bundle into multi-symbol orchestration, distributed failover, options live trading, or broad broker expansion.** Those are explicitly outside the recovery bundle’s scope.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Scope Boundaries, Rollout / Fallback / Compatibility Notes.

2. **Do not turn the recovery bundle into a multi-broker, multi-strategy, HFT, OMS, or portfolio-dashboard program.** The umbrella live doc is explicit that the current posture is narrow and should remain so until the current broker/feed path is productized honestly.  
   Sources: `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior, Proposed Improvement Approach, Rollout / Fallback / Compatibility Notes.

3. **Do not assume recovery work requires a broad shell/router rewrite.** The repo still uses a settings-mounted execution surface today, and the umbrella doc already splits surface promotion into a different child bundle.  
   Sources: `05_live_execution_productization_2026-04-10.md` — Current User-Facing Behavior, Related Docs; `REPO_MAP.md` — settings workspace note.

4. **Do not spend effort “fixing” stale file references in this packet unless new evidence appears.** No confirmed stale or nonexistent file reference was found in the provided bundle files.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Current Code Surface, Related Docs; `CONTEXT_FILE_LIST.txt`.

## 6. Recommended doc edits

1. Add a short **Current restart truth model** subsection near the top of the recovery doc. It should say exactly what survives relaunch and what does not: fresh `AppState`, no retained live-stream slot, no retained process-local runner runtime, and broker truth available only after a fresh snapshot call.  
   Sources: `state.rs` — `AppState::default`; `lib.rs` — `run_tauri`; `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`, `get_live_execution_snapshot`.

2. Replace the current three-domain recovery framing with a **four-domain recovery matrix**: feed lifecycle, runner state, broker freshness/truth, and strategy dataset / `data_id` validity.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach, Checklist; `commands_execution.rs` — `strategy_data_id`, `start_live_execution_runner`; `state.rs` — `upsert_live_bar`; `REPO_MAP.md` — Data, Polygon, And Live Execution.

3. Add an explicit **non-manual interruption contract** that names `closed`, `error`, and stream-ended paths and states whether they disarm the runner or block it pending explicit re-arm.  
   Sources: `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`; `commands_execution.rs` — `on_live_execution_bar`.

4. Add a **broker freshness contract** that uses `fetched_at_unix_ms`, defines stale/unknown/fresh states, and explains that `refreshStatuses()` is not the same thing as broker reconciliation.  
   Sources: `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`; `executionService.ts` — `refreshOrders`, `refreshStatuses`.

5. Add an **owner map** so the doc names the actual backend/frontend owners instead of relying on high-level routing shorthand.  
   Sources: `REPO_MAP.md` — Tauri runtime state, Live execution; `state.rs` — `AppState`; `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`.

6. Expand the **Verification Plan** with named scenario tests: same-symbol restart after `error`, same-symbol restart after `closed`, relaunch with broker unknown, stale broker snapshot after status refresh, missing `data_id`, invalidated derived dataset, and no-auto-rearm assertions. Either include `ExecutionWorkspacePanel.virtualization.test.ts` or explain why it is out of scope.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Verification Plan; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface.

7. Add one sentence clarifying whether bundle 08 owns only **review cues** or also owns **durable evidence**, so it does not sprawl into bundle 07.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach; `05_live_execution_productization_2026-04-10.md` — Proposed Improvement Approach, Related Docs.

## 7. Recommended implementation order

1. **Write the restart truth model first.** Before any code change, make the doc explicit about what survives relaunch, what becomes unknown, and what requires refresh.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Checklist; `lib.rs` — `run_tauri`; `state.rs` — `AppState::default`; `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`, `get_live_execution_snapshot`.

2. **Lock the non-manual interruption contract second.** Decide whether interruption disarms the runner or blocks it until re-arm. This is the highest-risk safety choice in the bundle.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Proposed Improvement Approach, Rollout / Fallback / Compatibility Notes; `commands_data.rs` — `run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`; `commands_execution.rs` — `on_live_execution_bar`.

3. **Expand the recovery matrix to include broker freshness and dataset identity third.** The plan should not proceed with a three-domain model once the code clearly depends on `data_id` and timestamped broker snapshots.  
   Sources: `commands_execution.rs` — `GetLiveExecutionSnapshotResponse`, `strategy_data_id`, `start_live_execution_runner`; `state.rs` — `upsert_live_bar`; `REPO_MAP.md` — Data, Polygon, And Live Execution.

4. **Resolve the owner and event contracts fourth.** Decide whether the runner event vocabulary changes, and document the real code owners so implementation does not drift.  
   Sources: `REPO_MAP.md` — Tauri runtime state, Live execution; `commands_execution.rs` — `LIVE_EXECUTION_RUNNER`; `executionService.ts` — `RUNNER_INACTIVE_EVENT_STATUSES`.

5. **Only then implement UI wording and review cues.** Once the semantics are locked, the frontend can distinguish off vs unknown vs stale vs fresh and can surface recovery-review posture without faking certainty.  
   Sources: `executionService.ts` — `liveStatusHint`, `parsePolygonLiveStatusEvent`, `refreshOrders`, `refreshStatuses`; `executionStore.ts` — initial state, event application.

6. **Add targeted tests before any polish or follow-on epic integration.** The first safety bar should be scenario tests for interruption, relaunch, stale broker truth, and dataset invalidation. Then optional refinements such as richer review cues or bundle-07/09 integrations can follow.  
   Sources: `08_live_execution_recovery_and_reconnect_2026-04-11.md` — Verification Plan; `05_live_execution_productization_2026-04-10.md` — Current Verification Surface.
