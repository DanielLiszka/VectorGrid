# Shell Context Menus Help Plan

Last updated: 2026-04-09

## Purpose

Define the current shell contextual-help posture, and explicitly record the absence of a shared context-menu subsystem, so guidance surfaces can be improved without inventing behavior that the shell does not currently have.

## Scope Boundaries

In scope:

- contextual-help surfaces driven by `ContextHelpTip.vue`
- help-mode behavior and shell guidance copy
- current shell posture regarding shared context menus

Out of scope:

- inventing a new context-menu framework
- panel-specific rich help content outside the current shared help component
- options-specific help systems

## Current User-Facing Behavior

The shell already has a shared contextual-help surface:

- `ContextHelpTip.vue` renders help affordances
- help mode controls whether guidance is full, concise, or off
- full help uses a richer hover-card posture
- concise help uses a tooltip posture
- off removes the help affordance entirely

This shared help surface is already used in multiple workspaces, including:

- data
- execution
- meta
- validation

The current shell does **not** appear to have a first-class shared context-menu subsystem. So the honest current contract is:

- shared contextual help exists
- shared context menus do not currently exist as a documented shell primitive

## Current Code Surface

- `web/src/components/ContextHelpTip.vue`
  - shared help affordance with full/concise/off behavior
- `web/src/composables/useHelpMode.ts`
  - central help-mode state consumption
- `web/src/stores/settingsStore.ts`
  - source of the `helpMode` preference
- consumers including:
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
- shell copy in settings/history that clarifies local vs global help/search posture

## Current Verification Surface

- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- indirect coverage through consumer panel tests
- there is currently no dedicated `ContextHelpTip` test

## Strengths Already Present

- there is already one shared contextual-help component
- help mode is centralized instead of scattered
- the shell already distinguishes full, concise, and off guidance modes
- contextual help is reusable across multiple workflows without bespoke implementations

## Weaknesses / Failure Modes

- there is no direct `ContextHelpTip` regression test
- “context menus” and “contextual help” are easy to conflate unless documented clearly
- help-mode expectations across shell surfaces are not yet written down in one place
- future contributors may invent ad hoc help or menu surfaces because the current boundary is implicit

## Regressions To Avoid

- breaking full/concise/off help-mode behavior
- removing shared contextual help in favor of inconsistent local implementations
- pretending a shared context-menu subsystem exists when it does not
- weakening shell guidance copy around local versus global actions

## External Research Summary

- [WAI-ARIA tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
  - relevant because concise contextual help is already tooltip-like
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because help mode should remain centralized and shared
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because help-mode behavior is already store-backed
- [WAI-ARIA modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the shell already uses other shared interaction surfaces and should keep help/menu responsibilities distinct from modal behavior

## Proposed Improvement Approach

1. Freeze the current truth explicitly:
   - shared contextual help exists
   - shared shell context menus do not currently exist as a generalized subsystem
2. Document help-mode behavior and the shared-help component contract.
3. Add a dedicated `ContextHelpTip` test.
4. If a future shared context-menu surface is added, document it as a new subsystem instead of silently folding it into this help contract.

## Checklist

- [ ] Document current full/concise/off help behavior.
- [ ] Document the current shared help component and its consumers.
- [ ] Document the absence of a shared context-menu subsystem explicitly.
- [ ] Add direct regression coverage for `ContextHelpTip.vue`.
- [ ] Preserve centralized help-mode behavior in the settings store.

## Verification Plan

- add and run a dedicated `ContextHelpTip` test
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`
- run representative consumer panel tests that use shared contextual help

## Rollout / Fallback / Compatibility Notes

- keep this doc honest to the current shell state
- if a future context-menu subsystem is introduced, split it into a new implementation doc rather than rewriting history here
- preserve help-mode compatibility even if contextual-help presentation evolves

## Related Docs

- `REPO_Improvement/01_shell/09_settings_appearance_layout_2026-04-09.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-09.md`
- `REPO_Improvement/01_shell/20_shell_shared_dialogs_2026-04-09.md`
- `REPO_Improvement/00_meta/11_archive_policy_2026-04-08.md`
