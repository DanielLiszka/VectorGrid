## Purpose

Define the improvement plan for saved-run history detail actions so the history panel remains a reliable reopen/apply/rerun/report/reveal/delete workspace instead of a brittle collection of loosely coupled buttons.

## Scope Boundaries

In scope:

- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/services/historyService.test.ts`

Out of scope:

- options-related history behavior
- Tauri-side artifact deletion/listing internals except where they define the action contract
- adding brand-new history features outside the current action set

## Current User-Facing Behavior

The saved-run detail pane already supports a substantial set of actions:

- load saved run
- apply saved run into the current workflow
- rerun saved run
- open saved run report
- reveal saved run on disk
- delete saved run
- save tags

This is already a real operational workspace, not just a viewer. The risk is that these actions span UI state, store policy, and service side effects, so drift can make the panel feel unreliable even when each individual function still exists.

## Current Code Surface

- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/historyService.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`

Key current ownership:

- the component owns action affordances, confirmation flow, and panel composition
- the store owns enable/disable state, selected run detail, and action dispatch
- the service owns actual load/apply/rerun/open/reveal/delete side effects

## Current Verification Surface

- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/stores/historyStore.search.test.ts`
- `web/src/services/historyService.test.ts`

## Strengths Already Present

- action boundaries are already separated into component/store/service layers
- the detail pane exposes enough metadata to make action context understandable
- confirmation already exists for destructive actions
- state gating already prevents obviously invalid operations in several cases
- tests already exist at the store, service, and component layers

## Weaknesses / Failure Modes

- detail actions depend on coordinated state across multiple layers
- it is easy for enablement rules and actual side-effect rules to drift apart
- status messaging can become inconsistent across load/apply/rerun/open/reveal/delete paths
- ML-signal and conventional runs already create branching behavior that can make actions harder to reason about
- a small UI cleanup can accidentally break a high-value workflow like rerun or reveal

## Regressions To Avoid

- breaking load/apply/rerun semantics while “simplifying” the panel
- making action enablement look available when service preconditions still fail
- weakening destructive-action confirmation
- breaking report/reveal actions for runs that still have valid artifact/report paths
- letting one action mutate more workflow state than intended

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the history panel depends on state shared across component, store, and service layers.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because current reliability depends heavily on store-driven action tests.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because history actions ultimately cross into Rust commands for artifact loading, opening, revealing, or deleting.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because saved-run detail payloads act like structured contracts and directly drive action availability.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because the history detail pane is an action-heavy, stateful interface where confirmability and keyboard clarity matter.

## Proposed Improvement Approach

1. Treat the history detail pane as a command surface with explicit action contracts.
2. For each action, document:
   - required preconditions
   - state mutations
   - expected status messaging
   - failure behavior
3. Tighten parity between store enablement logic and service-side precondition checks.
4. Keep destructive and filesystem actions explicit and confirmable.
5. Expand tests around action sequencing, especially:
   - selecting a run
   - loading/applying it
   - rerunning it
   - opening/revealing/deleting it

## Checklist

- [ ] Document each saved-run detail action and its preconditions.
- [ ] Audit store enablement flags against service-side reality.
- [ ] Add or tighten tests for action sequencing from the selected-run state.
- [ ] Add or tighten tests for destructive-action confirmation and cancellation.
- [ ] Normalize status-line behavior so successes and failures are explainable.
- [ ] Confirm ML-signal-specific branches do not weaken the conventional run-action path.

## Verification Plan

- run `HistoryWorkspacePanel` component tests
- run `historyStore.actions` tests
- run `historyStore.search` tests when selection/search interactions are touched
- run `historyService` tests for load/apply/rerun/open/reveal/delete behavior

## Rollout / Fallback / Compatibility Notes

- preserve existing actions and naming unless a stronger contract requires clarification
- prefer better gating, wording, and tests over action-surface churn
- if a cleanup makes the panel less predictable, keep the current explicit action set and harden it instead

## Related Docs

- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/06_artifacts/26_history_service_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/19_report_save_open_reveal_2026-04-09.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-08.md`
