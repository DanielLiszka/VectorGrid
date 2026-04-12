# Live Execution Operator Safety Controls Plan

Last updated: 2026-04-12

## Purpose

Define a measured plan to turn live execution into a clearer discrete operating mode with first-class operator safety controls, without widening broker or strategy scope before the current path is hardened.

## Scope Boundaries

In scope:

- first-class operator safety controls for the current live-execution surface
- discrete live-mode control semantics for the existing non-options workflow
- stop runner, stop live feed, flatten, cancel-open-orders, and emergency-halt semantics
- confirmation posture, disabled-state logic, and operator-facing blocked reasons
- current Alpaca and Polygon-backed live path only

Out of scope:

- options live trading
- multi-broker expansion
- multi-symbol parallel live execution
- replacing the current strategy families with a broader live platform

## Current User-Facing Behavior

The current live surface already supports one-shot execution, live-feed start/stop, runner start/stop, broker snapshot refresh, and broker connectivity checks. That makes the feature real, not placeholder.

But the operator safety layer is still thin for a discrete live mode:

- there is no first-class flatten action
- there is no first-class cancel-open-orders action
- runner stop is not the same thing as a broker/order safety halt
- current safety posture is spread across blocked reasons, confirmations, and status text rather than a compact operator-control model

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/executionRisk.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/lib.rs`

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/utils/executionRisk.test.ts`
- targeted Rust tests in `src-tauri/src/commands_execution.rs`
- targeted Rust tests in `src-tauri/src/commands_data.rs` and `src-tauri/src/state.rs` when stop-feed or runner/feed interaction behavior changes
- `npm.cmd --prefix web run parity`
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run a11y:smoke`
- `npm.cmd --prefix web run test:desktop:smoke`

## Safety Control Matrix Requirement

The shipped control model for this bundle must separate five actions explicitly:

- stop runner
- stop live feed
- flatten current position
- cancel open orders
- emergency halt / disarm posture

Each action needs an explicit contract for:

- what it does
- what it does not do
- symbol scope
- runner interaction
- feed interaction
- persistence
- confirmation requirements
- blocked-state rules

## Strengths Already Present

- dry-run, paper, and live posture already exist
- runner start already requires an active matching live stream
- blocked reasons already surface why actions are unavailable
- current scope limits are explicit enough to build on safely

## Weaknesses / Failure Modes

- operators can stop the runner without clearly flattening broker exposure
- the UI does not currently elevate emergency or broker-facing safety actions to first-class controls
- optimistic local state can move faster than explicit operator understanding of broker state
- current safety semantics are real but not compactly modeled as a discrete live-mode control set

## Regressions To Avoid

- weakening current dry-run or paper safety while adding live controls
- introducing broker-facing actions without explicit confirmation and disabled-state rules
- implying that runner stop automatically cancels orders or flattens exposure when it does not
- widening live scope before the current single-symbol, single-runner model is safer
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
- [Tauri Dialog](https://v2.tauri.app/plugin/dialog/)
- [Tauri state management](https://v2.tauri.app/develop/state-management/)

## Proposed Improvement Approach

1. Treat live execution as a discrete operating mode with a dedicated safety-control model, not just an extension of the settings-side execution panel.
2. Define and separate the current operator actions:
   - stop runner
   - stop live feed
   - flatten current position
   - cancel open orders
   - emergency halt / disarm posture
3. Make every safety action explicit about what it does not do.
4. Add backend-authoritative contracts for any new broker-facing safety action.
5. Freeze runner-active and feed-active interaction rules before shipping controls:
   - flatten while runner active
   - cancel open orders while runner active
   - stop feed while runner active
   - emergency halt while a runner exists
6. Keep the initial scope to current Alpaca + Polygon + single-symbol live mode, with current-symbol-only semantics unless a narrower follow-up doc explicitly widens scope.
7. Define a confirmation contract that restates the actual symbol, live/paper posture, feed state, parameter source, and risk posture inside the modal itself.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the discrete live-mode safety-control matrix for the current supported scope.
- [x] Record the exact semantics of stop runner versus stop feed versus flatten versus cancel-open-orders.
- [x] Decide whether emergency halt is only session-local or should persist until explicit operator re-arm.
- [x] Define dedicated backend request/response contracts for flatten, cancel-open-orders, and any shipped emergency halt/disarm behavior.
- [x] Freeze symbol scope for new broker-facing controls and avoid blank-symbol broadening unless a narrower follow-up explicitly widens it.
- [x] Define confirmation-copy requirements for every broker-facing safety action.
- [x] Define how bracket-order and partial-failure behavior is surfaced for flatten and cancel-open-orders.
- [x] Define disabled-state and blocked-reason rules for the new safety controls.
- [x] Keep new controls backend-authoritative where they affect broker state.
- [x] Preserve current dry-run and paper behavior while adding live-mode safety actions.
- [x] Add or strengthen frontend tests around safety-button availability, confirmation flow, and operator-state clarity.
- [x] Add or strengthen Rust tests around flatten/cancel/stop semantics, runner/feed interaction, and safe degradation.
- [x] Record rollback conditions if the new control model increases operator ambiguity or weakens current safety posture.

## Verification Plan

- rerun targeted execution-store, execution-service, risk-helper, and execution-panel tests near touched files
- rerun targeted Rust tests in `src-tauri/src/commands_execution.rs`, `src-tauri/src/commands_data.rs`, and `src-tauri/src/state.rs` when ownership crosses those boundaries
- rerun `npm.cmd --prefix web run parity` when the Tauri command surface changes
- rerun `npm.cmd --prefix web run test:browser:ux` for any live-mode UI/control changes
- rerun `npm.cmd --prefix web run a11y:smoke` for modal, danger-control, or status-presentation changes
- rerun `npm.cmd --prefix web run test:desktop:smoke` if the live controls move or surface in a dedicated desktop shell
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
- capture concrete acceptance evidence such as screenshots, state-transition notes, and any new session/safety artifacts introduced by the work

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only per control slice.
- Final validation must include targeted unit/integration tests for every touched execution-store, execution-service, Tauri command, and runtime-control surface, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for the operator controls and confirmation flows, plus a paper/live-mode manual smoke path proving the final control matrix does not silently widen risk.
- Final validation must include performance evaluation for any touched refresh, status, or control-trigger path so safety controls do not materially degrade the live surface.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollout / Fallback / Compatibility Notes

- start with explicit semantics and confirmations before adding new control breadth
- preserve the current one-runner and one-live-stream constraints unless a narrower follow-up doc explicitly widens them
- if a safety control cannot be made backend-authoritative yet, keep it out of the shipped live-mode control set
- keep shell-promotion work separate unless a narrower follow-up intentionally widens this bundle beyond safety semantics
- do not widen this plan into broker expansion or options live trading

## Execution Notes

- Implemented a five-action safety matrix directly in the live execution surface so stop runner, stop live feed, cancel open orders, flatten position, and emergency halt all state what they do, what they do not do, and how scope is constrained.
- Added backend-authoritative Tauri commands for `cancel_live_execution_open_orders`, `flatten_live_execution_position`, and `emergency_halt_live_execution` in `src-tauri/src/commands_execution.rs`, with current-symbol-only semantics and explicit runner-off requirements for broker-facing safety actions.
- Preserved current dry-run and paper behavior. The new broker-facing controls only light up when symbol and effective credentials are present, while emergency halt remains a local disarm path that stops the feed and runner but does not flatten or cancel broker state.
- Surfaced confirmation copy and blocked reasons in `ExecutionWorkspacePanel.vue` / `executionStore.ts`, and added targeted frontend coverage plus browser UX proof for the new control contract.
- Verification run for closure:
  - `cargo test -p ta_desktop_demo_app --lib`
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run parity`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run a11y:smoke`
  - `npm.cmd --prefix web run test:desktop:smoke`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
