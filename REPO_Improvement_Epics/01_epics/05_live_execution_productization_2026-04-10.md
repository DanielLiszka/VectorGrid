# Live Execution Productization Plan

Last updated: 2026-04-10

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
- the execution panel currently lives in the settings workspace rather than as a clearly elevated operating surface

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
- targeted Rust tests in `src-tauri/src/commands_execution.rs`
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run test:desktop:smoke`

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
2. Decide whether the execution surface should remain settings-mounted or move to a more deliberate dedicated workflow surface.
3. Add operator-focused controls and state clarity in bounded slices:
   - flatten / stop / emergency halt semantics
   - broker/order/position refresh clarity
   - runner recovery and reconnect posture
4. Add a live-session evidence model so important live actions are inspectable after the fact rather than only visible as transient status text.
5. Keep success criteria explicit:
   - stronger safety and diagnostics for the current supported live scope
   - no ambiguity about supported symbols, strategies, broker mode, or runner/feed state
   - no regression in existing dry-run and paper workflows

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the intended product posture for live execution in the current repo: narrow MVP hardening versus broader trading-surface expansion.
- [ ] Record the current operator journey for dry-run, paper, and live modes before changing implementation.
- [ ] Decide whether the execution surface stays under settings or moves to a dedicated workflow/home.
- [ ] Define the minimum operator safety controls that should become first-class in the current scope.
- [ ] Define the minimum live-session evidence or audit artifact that should exist for runner activity and broker-facing actions.
- [ ] Preserve the current explicit single-stream and single-runner constraints unless a narrower follow-up doc widens them.
- [ ] Preserve the current strategy and broker scope unless a narrower follow-up doc explicitly expands them.
- [ ] Add or strengthen frontend and Tauri tests around the chosen safety/productization slices.
- [ ] Define rollback conditions if a live-execution UX expansion increases operator ambiguity or weakens the existing dry-run/paper guardrails.
- [ ] Record any required cross-doc sync in first-tier live, shell, and tests/perf docs when the operator workflow changes.

## Verification Plan

- rerun targeted frontend execution-store/service/component tests near any touched live-execution surface
- rerun targeted Rust command tests near `commands_execution.rs` and any touched live-stream/runtime state surfaces
- rerun `npm.cmd --prefix web run test:browser:ux` for user-visible live-execution workflow changes
- rerun `npm.cmd --prefix web run test:desktop:smoke` if the execution surface moves or desktop wiring changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck`
- capture concrete acceptance evidence for closure, including updated operator-flow notes, screenshots where needed, and any saved live-session evidence/artifact examples introduced by the epic

## Rollout / Fallback / Compatibility Notes

- start with posture, safety, and evidence improvements before broker or strategy expansion
- keep current dry-run and paper flows intact even if later live-facing UX is rolled back
- prefer explicit narrow support statements over vague “live trading” marketing language
- do not widen this epic into new broker integrations or options/live variants unless a separate follow-up doc explicitly owns that scope

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-09.md`
- `REPO_Improvement/09_live/03_execution_ui_state_2026-04-09.md`
- `REPO_Improvement/09_live/04_alpaca_execution_path_2026-04-09.md`
- `REPO_Improvement/09_live/05_polygon_live_streams_2026-04-09.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
