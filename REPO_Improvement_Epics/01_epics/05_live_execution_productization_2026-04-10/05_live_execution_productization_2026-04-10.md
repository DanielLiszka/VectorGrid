# Live Execution Productization Plan

Last updated: 2026-04-12

## Purpose

Define a measured epic to expand and harden the current live-execution implementation from a narrow, real MVP into a more deliberate productized trading surface without pretending it is already a broad broker or multi-strategy platform.

## Scope Boundaries

In scope:

- live-execution workflow productization for the current non-options surface
- single-symbol live feed and runner ergonomics, recovery, and safety hardening
- live execution UI placement, auditability, and operator-facing controls
- current Alpaca and Polygon integration posture
- session evidence, diagnostics, and operator-state clarity

Out of scope:

- options-related live trading
- multi-broker expansion unless the current broker/productization path proves stable first
- unmanaged high-frequency trading ambitions
- broad strategy-surface expansion beyond current supported live-execution families unless explicitly justified

## Current User-Facing Behavior

The current live-execution surface is real, not a placeholder:

- one-shot execution exists
- a live runner exists
- broker connectivity and snapshots exist
- live Polygon candle streaming exists
- the frontend has preflight gating, confirmation prompts, and status/event handling

But it is intentionally narrow:

- live execution strategy coverage is limited to `DoubleMa` and `CompositeCpu`
- broker integration is Alpaca over REST
- live streaming is Polygon stocks only
- only one active live stream slot and one active live runner slot exist at a time
- the execution panel now lives in a dedicated `live` workspace rather than a settings-mounted shell

## Current Runtime Reality

- the current shell exposes a discrete `live` workspace; `App.vue` mounts `SettingsWorkspacePanel.vue` only for `settings` and `ExecutionWorkspacePanel.vue` only for `live`
- `AppState` currently owns the live stream slot and loaded dataset mutation path, while the live runner runtime itself currently lives in `commands_execution.rs`
- Alpaca credentials can now resolve from backend secure storage, while legacy frontend persistence remains a compatibility/migration concern rather than the primary broker-secret path
- chart/account review now uses explicit symbol-scoped broker review, typed chart-surface states, and broker-confirmed marker handling instead of a single placeholder summary
- live session evidence, interruption finalization, and recovery-review posture are now part of the shipped runtime contract

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/lib.rs`

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/components/ExecutionWorkspacePanel.virtualization.test.ts`
- `web/src/stores/workflowStore.test.ts`
- `web/src/services/workflowBootstrap.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/src\App.test.ts`
- `web/e2e/browser/execution-review-surface.spec.ts`
- `web/e2e/browser/shell-ux.spec.ts`
- targeted Rust tests in `src-tauri/src/commands_execution.rs`
- targeted Rust tests in `src-tauri/src/commands_data.rs`, `src-tauri/src/state.rs`, and `src-tauri/src/polygon.rs` when the live stream, key storage, or dataset mutation path is touched
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run test:desktop:smoke`

## Operator Truth Invariants

- runner events and optimistic order rows are provisional; broker snapshots remain authoritative for order and position confirmation
- chart/account review surfaces must preserve provenance rather than imply broker-confirmed chart overlays that do not yet exist
- no reconnect, restart, or feed-interruption path should silently leave broker-facing automation effectively re-armed without an explicit operator decision
- the current single-stream and single-runner constraints remain product boundaries until a narrower follow-up doc widens them

## Strengths Already Present

- the live surface already has real backend commands and real event flow
- the frontend already distinguishes dry-run, paper, and live modes
- symbol/feed/runner preflight checks already exist
- current scope limits are explicit enough to build on rather than hiding them

## Weaknesses / Failure Modes

- the surface can look more mature than it is because the core loop is real but the operating envelope is still narrow
- a settings-mounted execution panel understates the seriousness of live actions while also making the feature feel secondary
- operator actions such as flatten/kill-switch/cancel-order style controls are not first-class today
- session/audit evidence and recovery posture are thinner than the rest of the repo’s artifact-heavy workflow culture
- single-slot live stream and runner constraints are real product boundaries that should be reflected clearly

## Regressions To Avoid

- widening live-execution scope faster than safety, diagnostics, and rollback posture can support
- adding broker-facing controls without explicit operator-state clarity and confirmation rules
- weakening current dry-run/paper/live distinctions while trying to make the feature feel more polished
- claiming multi-strategy or multi-broker maturity before the existing single-broker path is productized
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [About Trading API](https://docs.alpaca.markets/docs/trading-api)
  - relevant because the current broker integration is Alpaca and the productization work should stay honest about the broker contract already in use
- [Overview | Stocks WebSocket](https://massive.com/docs/websocket/stocks/overview)
  - relevant because the current live-feed path depends on Polygon/Massive stock WebSocket behavior and the repo only supports that live market/feed today
- [State Management | Tauri](https://v2.tauri.app/develop/state-management/)
  - relevant because the live stream, runner slots, and recovery posture depend on explicit shared state ownership in the Tauri layer

## Proposed Improvement Approach

1. Productize the current narrow live path before broadening scope:
   - make the operating posture explicit
   - make safety actions explicit
   - make session evidence explicit
2. Treat live execution as a discrete operating mode, not only a settings-mounted execution panel.
3. Establish prerequisite hardening before surface promotion:
   - secure credential storage for the current Alpaca path
   - truthful broker position/order reconciliation
   - truthful chart/account review completion
4. Decide whether the execution surface should remain settings-mounted or move to a more deliberate dedicated workflow surface only after those prerequisites are explicit.
5. Add operator-focused controls and state clarity in bounded slices:
   - flatten / stop / emergency halt semantics
   - broker/order/position refresh clarity
   - runner recovery and reconnect posture
6. Add a live-session evidence model so important live actions are inspectable after the fact rather than only visible as transient status text.
7. Keep success criteria explicit:
   - stronger safety and diagnostics for the current supported live scope
   - no ambiguity about supported symbols, strategies, broker mode, or runner/feed state
   - no regression in existing dry-run and paper workflows

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the intended product posture for live execution in the current repo: narrow MVP hardening versus broader trading-surface expansion.
- [x] Record the current operator journey for dry-run, paper, and live modes before changing implementation.
- [x] Freeze the current runtime-owner map for the live stream slot, runner runtime, credential storage, and dataset mutation path before changing behavior.
- [x] Make secure credential storage an explicit prerequisite before broader live-surface promotion.
- [x] Make broker-confirmed versus provisional state truth explicit before broadening operator controls.
- [x] Decide whether the execution surface stays under settings or moves to a dedicated workflow/home only after credential, reconciliation, and review-surface prerequisites are explicit.
- [x] Define the minimum operator safety controls that should become first-class in the current scope.
- [x] Define the minimum live-session evidence or audit artifact that should exist for runner activity and broker-facing actions.
- [x] Define the no-silent-rearm rule for reconnect, restart, and feed interruption behavior.
- [x] Preserve the current explicit single-stream and single-runner constraints unless a narrower follow-up doc widens them.
- [x] Preserve the current strategy and broker scope unless a narrower follow-up doc explicitly expands them.
- [x] Add or strengthen frontend and Tauri tests around the chosen safety/productization slices.
- [x] Define rollback conditions if a live-execution UX expansion increases operator ambiguity or weakens the existing dry-run/paper guardrails.
- [x] Record any required cross-doc sync in first-tier live, shell, and tests/perf docs when the operator workflow changes.

## Execution Notes

- Completed the live hardening program as bounded slices: operator safety controls, session audit trail, recovery/reconnect hardening, broker position/order reconciliation, secure credential storage, chart/account review completion, and shell promotion into a discrete `live` workspace.
- Preserved the narrow product boundary: `DoubleMa` and `CompositeCpu`, Alpaca REST, Polygon stocks, one active stream, and one active runner.
- Promoted secure backend credential resolution, symbol-scoped broker review, broker-confirmed chart markers, interruption-driven runner disarm/finalization, and explicit recovery-review posture as product contracts rather than advisory behavior.
- Moved execution out of the settings shell into a dedicated live workspace while keeping the support envelope visible at point of use and preserving current blocked reasons, confirmations, and operator monitoring detail.
- Synced current-state docs in `REPO_MAP.md`, `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`, and `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`.
- Verified the integrated state with targeted frontend/runtime tests, browser UX, desktop smoke, accessibility smoke, workflow gate candidate timing, and the repo quality gate.

## Verification Plan

- rerun targeted frontend execution-store/service/component tests near any touched live-execution surface
- rerun targeted Rust command tests near `commands_execution.rs` and any touched live-stream/runtime state surfaces
- rerun `npm.cmd --prefix web run test:browser:ux` for user-visible live-execution workflow changes
- rerun `npm.cmd --prefix web run test:desktop:smoke` if the execution surface moves or desktop wiring changes
- rerun `npm.cmd --prefix web run a11y:smoke` when confirmation posture, status presentation, or dedicated live-surface behavior changes
- rerun reconnect/interruption, secret migration/precedence, and provisional-versus-confirmed truth scenarios for any touched live-surface contract
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck`
- capture concrete acceptance evidence for closure, including updated operator-flow notes, screenshots where needed, and any saved live-session evidence/artifact examples introduced by the epic

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only slice by slice during development.
- Final validation must include targeted unit/integration tests for every touched frontend, Tauri, and core-runtime surface, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for any user-visible workflow or operator-surface change, plus a paper/live-mode manual smoke path when broker-facing behavior or state transitions changed.
- Final validation must include performance evaluation for any touched startup, refresh, chart, reconciliation, hydration, or broker-status path so the final state does not regress responsiveness or runtime cost.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollout / Fallback / Compatibility Notes

- start with posture, credential hardening, operator truth, and evidence improvements before broker or strategy expansion
- keep current dry-run and paper flows intact even if later live-facing UX is rolled back
- prefer explicit narrow support statements over vague “live trading” marketing language
- preserve historical route/order/position provenance rather than rewriting prior live-session truth into newer vocabulary
- do not widen this epic into new broker integrations or options/live variants unless a separate follow-up doc explicitly owns that scope

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/06_broker_connectivity_2026-04-09.md`
- `REPO_Improvement/09_live/07_live_candle_stream_data_2026-04-08.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
- `06_live_execution_operator_safety_controls_2026-04-11.md`
- `07_live_execution_session_audit_trail_2026-04-11.md`
- `08_live_execution_recovery_and_reconnect_2026-04-11.md`
- `09_live_mode_discrete_surface_promotion_2026-04-11.md`
- `10_live_execution_broker_position_order_reconciliation_2026-04-11.md`
- `11_live_execution_secure_credential_storage_2026-04-11.md`
- `12_live_execution_chart_and_account_review_completion_2026-04-11.md`
