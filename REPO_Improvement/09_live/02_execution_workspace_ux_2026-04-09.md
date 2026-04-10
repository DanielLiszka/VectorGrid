# Execution Workspace UX Plan

Last updated: 2026-04-10

## Purpose

Define the current execution workspace UX contract so layout, posture copy, confirmation flows, operator review surfaces, and settings-workspace embedding remain explicit and stable.

## Scope Boundaries

In scope:

- `ExecutionWorkspacePanel.vue`
- current execution panel information architecture
- confirmation patterns for risky actions
- current relationship between the execution panel and the settings workspace
- operator-facing status/review copy

Out of scope:

- backend execution order logic except where UX depends on it
- live runner internals beyond what the panel exposes
- options workflows

## Current User-Facing Behavior

The execution panel is currently mounted inside the settings workspace rather than exposed as a fully separate live-only workspace. The shipped UX groups:

- posture cards
- safety copy
- account/review actions
- manual execution controls
- live automation controls
- runtime status
- chart context
- broker orders
- raw output preview

Current behavior also distinguishes between actions that run immediately and actions that require confirmation:

- dry-run one-shot execution runs immediately
- broker-armed one-shot execution requires confirmation
- live-feed start requires confirmation
- runner start requires confirmation
- key clearing requires confirmation

## Current Code Surface

- `web/src/components/ExecutionWorkspacePanel.vue`
  - panel layout, posture copy, confirmation dialogs, and result/review surfaces
- `web/src/stores/executionStore.ts`
  - UX-ready posture labels, blocked reasons, and runtime-derived display state
- `web/src/services/executionService.ts`
  - action orchestration and confirmation-sensitive flows
- `web/src/components/ConfirmationDialog.vue`
  - shared confirmation primitive used by risky execution actions
- `web/src/App.vue`
  - current mounting arrangement that places execution inside the settings workspace

## Current Verification Surface

- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`

## Strengths Already Present

- the panel already communicates posture clearly through separate paper/live, dry-run/broker, and live automation sections
- risky actions already route through confirmation instead of executing silently
- runtime status, chart context, broker orders, and raw output are visible together in one operator workflow
- blocked reasons already exist in store/service logic instead of forcing users to infer why an action is unavailable

## Weaknesses / Failure Modes

- calling this a standalone execution workspace is slightly misleading because it is currently embedded inside settings
- the chart area is still placeholder-backed, which can make the review surface look more complete than it is
- “Remember keys” behavior is tied to successful broker-facing actions, not just the checkbox toggle, which is easy to misunderstand
- the panel carries many concerns at once, so later layout changes can easily break confirmation or review affordances

## Regressions To Avoid

- removing existing confirmation steps for broker-armed or automation-start actions
- collapsing posture sections so paper/live and dry-run/broker distinctions become less explicit
- making the embedded settings-workspace context less clear without actually moving the feature
- hiding blocked reasons or confirmation explanations in the name of visual simplification
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the panel is store-driven and its UX depends on explicit state ownership rather than local component improvisation
- [Dialog | Tauri](https://v2.tauri.app/plugin/dialog/)
  - relevant because confirmation and file/path dialogs are part of the desktop interaction model and should remain explicit in the UX contract
- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because the UX must accurately distinguish simulated actions from broker-submitted actions
- [Tauri State Management](https://v2.tauri.app/es/develop/state-management/)
  - relevant because desktop/runtime state is authoritative outside the panel itself and the UX should reflect that boundary
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the panel’s current information architecture in the exact order users encounter it.
2. Make confirmation ownership explicit for each risky action.
3. Preserve the current posture vocabulary:
   - paper/live
   - dry-run/broker-armed
   - one-shot/runner
   - live-feed on/off
4. Document the current settings-workspace embedding so future UX cleanup work does not misdescribe the current product.
5. Keep review surfaces grouped with action surfaces because that is how operators currently reason about the tool.
6. Treat the placeholder chart and key-persistence behavior as important UX caveats rather than incidental bugs.
7. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current execution panel layout groups in order.
- [x] Document which actions currently require confirmation and which do not.
- [x] Document posture and safety-copy semantics shown to the user.
- [x] Document the current settings-workspace embedding and its UX implications.
- [x] Document current chart/review surface limitations.
- [x] Document “Remember keys” persistence behavior accurately.
- [x] Preserve component tests for dry-run, broker-armed confirmation, and review rendering.
- [x] Preserve store/service tests for blocked-reason and preflight behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/executionService.test.ts`
- manually exercise dry-run one-shot, broker-armed one-shot, live-feed start, runner start, and key clear flows
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep current confirmation safeguards intact during UX cleanup
- preserve the current panel/store/service ownership split until a larger workspace move is intentional
- if the execution panel later becomes a standalone workspace, document that as a product-structure change rather than silently rewriting current assumptions
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by hardening the existing execution UX regression surface in [ExecutionWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts):
  - added explicit confirmation coverage for `Start live`, `Start runner`, and `Clear keys`
  - preserved the existing direct dry-run one-shot behavior while keeping broker-armed one-shot actions confirmation-gated
- Kept scope intentionally additive:
  - no panel layout groups moved
  - no confirmation copy was removed
  - no settings-workspace embedding behavior changed
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/01_shell/24_shell_desktop_productization_2026-04-08.md`
