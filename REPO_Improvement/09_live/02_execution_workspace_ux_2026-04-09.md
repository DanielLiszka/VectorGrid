# Execution Workspace UX Plan

Last updated: 2026-04-09

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

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the panel is store-driven and its UX depends on explicit state ownership rather than local component improvisation
- [Dialog | Tauri](https://v2.tauri.app/plugin/dialog/)
  - relevant because confirmation and file/path dialogs are part of the desktop interaction model and should remain explicit in the UX contract
- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because the UX must accurately distinguish simulated actions from broker-submitted actions
- [Tauri State Management](https://v2.tauri.app/es/develop/state-management/)
  - relevant because desktop/runtime state is authoritative outside the panel itself and the UX should reflect that boundary

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

## Checklist

- [ ] Document the current execution panel layout groups in order.
- [ ] Document which actions currently require confirmation and which do not.
- [ ] Document posture and safety-copy semantics shown to the user.
- [ ] Document the current settings-workspace embedding and its UX implications.
- [ ] Document current chart/review surface limitations.
- [ ] Document “Remember keys” persistence behavior accurately.
- [ ] Preserve component tests for dry-run, broker-armed confirmation, and review rendering.
- [ ] Preserve store/service tests for blocked-reason and preflight behavior.

## Verification Plan

- run `npm --prefix web test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web test -- src/services/executionService.test.ts`
- manually exercise dry-run one-shot, broker-armed one-shot, live-feed start, runner start, and key clear flows

## Rollout / Fallback / Compatibility Notes

- keep current confirmation safeguards intact during UX cleanup
- preserve the current panel/store/service ownership split until a larger workspace move is intentional
- if the execution panel later becomes a standalone workspace, document that as a product-structure change rather than silently rewriting current assumptions

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_DATE.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_DATE.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/01_shell/24_shell_desktop_productization_2026-04-08.md`
