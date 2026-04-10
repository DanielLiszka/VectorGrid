# Execution Safety Modes Plan

Last updated: 2026-04-10

## Purpose

Define the current execution safety-mode contract so broker mode, dry-run posture, live-feed mode, one-shot versus runner behavior, and current frontend/backend guardrails remain explicit and regression-resistant.

## Scope Boundaries

In scope:

- `paper` versus `live`
- `dryRun` versus broker-armed submission
- delayed live-feed posture
- one-shot versus auto-runner posture
- current safety prompts and blocked-reason model
- bracket-order attachment conditions

Out of scope:

- redesigning the execution workspace
- adding new brokers
- options workflows

## Current User-Facing Behavior

The current execution safety posture is a composition of several orthogonal mode choices:

- broker mode:
  - `paper`
  - `live`
- submission posture:
  - `dryRun=true`
  - broker-armed
- data posture:
  - delayed live feed by default
- execution path:
  - one-shot execution
  - auto-runner

The UI surfaces these through posture cards, safety copy, and confirmations. Current behavior means:

- `paper + dryRun=false` still sends broker paper orders
- `live + dryRun=true` remains simulation only
- non-dry-run requires Alpaca credentials
- only `DoubleMa` and `CompositeCpu` are currently supported
- bracket orders are only attached when opening a new position and both stop-loss and take-profit are present

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
  - safety posture UI and confirmation entrypoints
- `web/src/stores/executionStore.ts`
  - blocked reasons, mode labels, and posture-derived display state
- `web/src/services/executionService.ts`
  - preflight validation, confirmation-sensitive actions, and frontend-only runner/live-feed guardrails
- `src-tauri/src/commands_execution.rs`
  - authoritative execution validation, signal submission, runner control, and safety rules
- `web/src/utils/executionRisk.ts`
  - frontend risk-related helpers used by the execution surface

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/utils/executionRisk.test.ts`
- targeted execution tests in `src-tauri/src/commands_execution.rs`

## Strengths Already Present

- the product already distinguishes paper/live and dry-run/broker-armed explicitly
- frontend and backend both contribute guardrails instead of trusting one layer only
- blocked reasons are already materialized for the user instead of hidden
- bracket-order attachment rules are already narrow and explicit
- zero-delta and unsupported-strategy cases already degrade safely

## Weaknesses / Failure Modes

- some safety checks are frontend-only rather than backend invariants, especially around live-feed-active and live-symbol-match posture before runner start
- paper/live and dry-run/broker-armed combinations are easy to misunderstand if docs are too shallow
- delayed live-feed defaults and current stock-only assumptions can be overlooked
- the UI can imply a single safety mode when the real posture is a matrix of modes

## Regressions To Avoid

- weakening dry-run safety or paper/live separation
- allowing unsupported strategies or invalid risk overrides past current guardrails
- silently broadening bracket-order attachment rules
- documenting frontend-only checks as if they were guaranteed backend invariants
- making cancellation or runner-stop behavior less explicit than it is now
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because broker submission and buying-power/order-state semantics bound what “live” or “paper” can truthfully mean
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
  - relevant because execution safety depends on acting against current broker-confirmed position, not only local intent
- [Tauri State Management](https://v2.tauri.app/es/develop/state-management/)
  - relevant because single-runner posture and runtime safety state are authoritative on the Rust side
- [Dialog | Tauri](https://v2.tauri.app/plugin/dialog/)
  - relevant because risky transitions rely on explicit confirmations in the desktop UX
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document safety posture as a matrix, not a single mode switch.
2. Separate frontend guardrails from backend invariants in the plan.
3. Preserve the exact meaning of the main mode combinations:
   - paper + dry-run
   - paper + broker-armed
   - live + dry-run
   - live + broker-armed
4. Keep live-feed posture and runner posture documented as distinct dependencies rather than one “automation mode.”
5. Preserve narrow bracket-order semantics and zero-delta hold behavior as explicit safety features.
6. Tie any safety-copy or posture refactor to store/service/backend tests together.
7. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current mode matrix and how the main combinations behave.
- [x] Document which current safety checks are frontend-only and which are backend-authoritative.
- [x] Document non-dry-run credential requirements.
- [x] Document current strategy support limits.
- [x] Document bracket-order attachment requirements.
- [x] Document zero-delta and unsupported-strategy safe-degradation behavior.
- [x] Preserve tests for dry-run, broker-armed confirmation, blocked reasons, and live-runner preflight checks.
- [x] Preserve targeted backend tests for normalization, gating, and cancellation markers.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/executionService.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web run test -- src/utils/executionRisk.test.ts`
- run targeted execution tests in `src-tauri/src/commands_execution.rs`
- manually verify paper/live and dry-run/broker-armed combinations in the execution panel
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current conservative defaults:
  - `paper`
  - `dryRun=true`
  - delayed live feed
- if safety checks move across layers later, document that explicitly instead of silently changing which side enforces them
- prefer additive safety messaging over hidden mode simplification
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by extending the current safety-mode acceptance surface without changing behavior:
  - [ExecutionWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts) now proves the live broker-armed posture renders live-specific safety copy and the `Submit live order once` label
  - [executionRisk.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/utils/executionRisk.test.ts) now covers string-backed and invalid fallback boolean parsing for persisted or replayed strategy flags
- Existing coverage continues to anchor the rest of the safety matrix:
  - broker-armed confirmation gating
  - live-feed and runner blocked reasons
  - runner/live symbol preflight behavior
  - bracket-order attachment guards and cancellation markers in [commands_execution.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_execution.rs)
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts src/utils/executionRisk.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app commands_execution --lib`

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
