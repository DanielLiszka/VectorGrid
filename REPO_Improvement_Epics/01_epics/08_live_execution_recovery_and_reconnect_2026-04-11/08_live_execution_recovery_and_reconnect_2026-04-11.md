# Live Execution Recovery And Reconnect Plan

Last updated: 2026-04-11

## Purpose

Define a measured plan to harden recovery, reconnect, and restart behavior for the current live-execution implementation so the discrete live mode behaves predictably across feed interruptions, app restarts, and broker-state refresh gaps.

## Scope Boundaries

In scope:

- reconnect and restart posture for the current live feed and runner
- symbol/feed/runner mismatch recovery
- broker snapshot refresh and local runtime reconciliation after interruptions
- explicit restart behavior for the current single-stream, single-runner model

Out of scope:

- high-availability or distributed failover
- multi-symbol live orchestration
- options live trading
- broad broker expansion

## Current User-Facing Behavior

The current live implementation has real runtime status, runner slot ownership, and live-stream preconditions. It can tell the operator when live feed or runner are active, and it can refresh broker snapshots.

But recovery semantics are still thin:

- app restart posture is not yet elevated as a discrete live-mode rule set
- reconnect expectations are spread across status refresh, live stream state, and runner state
- one active runner slot and one active live stream slot make recovery behavior simple, but the implications are not yet productized

## Current Code Surface

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/lib.rs`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- targeted Rust tests in `src-tauri/src/commands_execution.rs`
- targeted Rust tests in `src-tauri/src/state.rs`

## Strengths Already Present

- current single-slot live stream and runner model keeps the state space bounded
- runner start already requires a matching active live stream
- state already exposes live-stream replacement, stop, and info helpers
- runner generation and cancel handling already protect against stale in-flight work

## Weaknesses / Failure Modes

- restart and reconnect posture is not yet an explicit operator contract
- live feed health, runner health, and broker health are related but distinct states
- current UI hints do not yet form a full recovery model for the operator
- stale runtime assumptions after reconnect can be more dangerous in a discrete live mode than in a settings-side experiment surface

## Regressions To Avoid

- auto-resuming live or broker-armed actions after restart without explicit operator intent
- hiding feed/run/broker mismatch states behind overly simple "connected" messaging
- introducing reconnect behavior that bypasses current symbol-match or dry-run/live guardrails
- widening the one-runner or one-stream model implicitly while trying to improve recovery
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri state management](https://v2.tauri.app/develop/state-management/)
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [Polygon WebSocket quickstart](https://polygon.io/docs/websocket/quickstart?auth=signup)
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)

## Proposed Improvement Approach

1. Define discrete live-mode restart and reconnect states explicitly.
2. Keep the recovery model conservative:
   - no silent broker re-arming
   - no silent runner restart into live mode
   - explicit operator review after interruption
3. Separate the recovery domains:
   - live feed state
   - runner state
   - broker snapshot state
4. Add clear mismatch and stale-state detection for the current one-symbol mode.
5. Capture enough persisted or reviewable evidence that an operator can tell what state survived an interruption.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define startup behavior for discrete live mode after app relaunch.
- [ ] Define the feed/runner/broker recovery state matrix for the current scope.
- [ ] Decide whether interrupted live mode always re-enters a safe inactive posture by default.
- [ ] Define stale-symbol, stale-feed, and stale-broker snapshot detection rules.
- [ ] Define explicit operator prompts or review steps required after interruption.
- [ ] Preserve current single-stream and single-runner constraints unless a narrower follow-up doc widens them.
- [ ] Add or strengthen tests around reconnect, mismatch, and stale-state parsing.
- [ ] Add or strengthen Rust tests around stream replacement, runner stop, and recovery-safe state transitions.
- [ ] Define rollback conditions if the reconnect model becomes more confusing than the current manual-refresh posture.

## Verification Plan

- rerun targeted execution-service, execution-store, and execution-panel tests near touched files
- rerun targeted Rust tests in `src-tauri/src/commands_execution.rs`, `src-tauri/src/state.rs`, and any touched live-stream owner modules
- rerun `npm.cmd --prefix web run test:browser:ux` if reconnect/restart UX changes become user-visible
- rerun `npm.cmd --prefix web run test:desktop:smoke` if discrete live-mode startup or relaunch behavior changes
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
- capture concrete acceptance evidence such as recovery-state screenshots, restart notes, and any new session/reconnect artifacts introduced by the work

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only during individual recovery-slice changes.
- Final validation must include targeted unit/integration tests for feed lifecycle, runner disarm logic, broker-freshness handling, dataset-validity handling, and recovery messaging, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for recovery-state rendering, plus a paper/live-mode manual smoke path covering disconnect, stale-state, and restart flows without silent re-arm.
- Final validation must include performance evaluation for any touched refresh, reconnect, or recovery path so the final state does not introduce avoidable polling or render regressions.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollout / Fallback / Compatibility Notes

- prefer explicit inactive-after-interruption posture over aggressive automatic recovery
- keep current manual refresh and status commands available even if richer reconnect behavior is added
- if a reconnect path cannot preserve operator clarity, fall back to a safer re-arm-required design
- do not widen this plan into multi-symbol orchestration or distributed runtime recovery

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/07_live_candle_stream_data_2026-04-08.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
