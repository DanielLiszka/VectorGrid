# Shell Context Menus Help Plan

Last updated: 2026-04-08

## Purpose

Define the current shared context-menu and contextual-help contract so menus, tooltips, hover-card help, and help-mode behavior can be improved without regressing local discoverability or accessibility.

## Scope Boundaries

In scope:

- shared `ContextActionMenu`
- shared `ContextHelpTip`
- help-mode-driven tooltip/hover-card behavior
- current usage of these primitives in non-options shell panels

Out of scope:

- global command palette search
- full documentation content strategy
- options-specific contextual menus or help surfaces

## Current User-Facing Behavior

The shell already has two shared contextual-assistance primitives:

- `ContextActionMenu`
  - shared action menu with menu semantics, focus return, disabled trigger behavior, and destructive-item styling
- `ContextHelpTip`
  - shared contextual help surface that changes behavior based on help mode:
    - `full` uses hover-card style richer detail
    - `concise` uses tooltip summary
    - `off` hides help affordances

Current usage already exists in major panels such as data, execution, meta, and validation. Meta surfaces also reuse the shared context-action menu for saved artifact families.

## Current Code Surface

- `web/src/components/ContextActionMenu.vue`
  - shared dropdown-menu primitive for contextual actions
- `web/src/components/ContextActionMenu.test.ts`
  - verifies open/select/focus return, escape close, and disabled-trigger behavior
- `web/src/components/ContextHelpTip.vue`
  - shared help primitive using tooltip versus hover-card behavior
- `web/src/composables/useHelpMode.ts`
  - central help-mode state normalization
- `web/src/components/DataWorkspacePanel.vue`
  - consumes `ContextHelpTip`
- `web/src/components/ExecutionWorkspacePanel.vue`
  - consumes `ContextHelpTip`
- `web/src/components/MetaWorkspacePanel.vue`
  - consumes both shared contextual primitives
- `web/src/components/ValidationWorkspacePanel.vue`
  - consumes `ContextHelpTip`

## Current Verification Surface

- `web/src/components/ContextActionMenu.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- help-mode behavior through `web/src/composables/useHelpMode.ts`

## Strengths Already Present

- both shared primitives already exist and are reused
- menu semantics, escape handling, and focus return are already tested for the context menu
- contextual help already respects a global help-mode setting
- the help primitive already distinguishes richer and lighter help posture
- menu/help behavior is already integrated into real production panels

## Weaknesses / Failure Modes

- help and context menus are related in practice but not documented as one shell-assistance surface
- there is less direct coverage for `ContextHelpTip` than for `ContextActionMenu`
- future contributors could add ad hoc tooltips or local menus instead of reusing the shared primitives
- help-mode semantics can drift if new help consumers ignore `useHelpMode`
- contextual menu usage is currently concentrated in a smaller set of panels and can become inconsistent

## Regressions To Avoid

- breaking focus return or escape behavior in shared context menus
- regressing help-mode behavior so `off`, `concise`, and `full` no longer differ correctly
- adding one-off tooltip or menu implementations where shared primitives should be used
- making contextual help behave like a global help system instead of local, inline assistance
- weakening disabled-trigger behavior for context menus with no available actions

## External Research Summary

- [WAI-ARIA Authoring Practices: menu button](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/)
  - relevant because the shared action menu behaves like a contextual menu-button surface
- [WAI-ARIA Authoring Practices: tooltip](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
  - relevant because the help primitive already exposes lightweight tooltip behavior in concise mode
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because help posture should remain a centralized setting, not per-component guesswork
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because help mode is store-backed and reused across multiple shell surfaces

## Proposed Improvement Approach

1. Treat context menus and contextual help as one shell-assistance family with two shared primitives.
2. Freeze the current contracts:
   - `ContextActionMenu` for local action menus
   - `ContextHelpTip` for local explanatory guidance
3. Require new contextual assistance to use the shared primitives unless there is a real reason not to.
4. Add or tighten direct tests for `ContextHelpTip` if help-mode consumers change.
5. Preserve the distinction between local contextual help and global command/search affordances.

## Checklist

- [ ] Document current shared context-menu behavior and menu semantics.
- [ ] Document `full` / `concise` / `off` help-mode behavior.
- [ ] Document the current major consumer panels for both shared primitives.
- [ ] Add or tighten direct tests for `ContextHelpTip` if its behavior changes.
- [ ] Reuse shared primitives for new local menus/help instead of adding one-offs.
- [ ] Keep local contextual assistance distinct from global command/search surfaces.

## Verification Plan

- run `npm --prefix web test -- src/components/ContextActionMenu.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`
- manually verify help-mode switching and a representative context-menu flow in a panel that uses each primitive

## Rollout / Fallback / Compatibility Notes

- preserve current help-mode semantics unless a deliberate migration is defined
- keep focus-return and disabled-trigger behavior stable in `ContextActionMenu`
- if richer help surfaces are added later, keep `concise` and `off` modes meaningful
- update this doc and tests whenever shared contextual primitives change materially

## Related Docs

- `REPO_Improvement/01_shell/09_settings_appearance_layout_2026-04-08.md`
- `REPO_Improvement/01_shell/04_shell_command_palette_2026-04-08.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-08.md`
- `REPO_Improvement/01_shell/20_shell_shared_dialogs_2026-04-08.md`
