# UX Accessibility Copy Regression Plan

Last updated: 2026-04-09

## Purpose

Define the current shell accessibility and operator-copy contract so wording, live-region semantics, and help text can be improved without degrading the shell’s current accessibility posture or user guidance.

## Scope Boundaries

In scope:

- shell-level accessibility and copy surfaces
- command palette semantics
- shell tab semantics
- runtime/status and confirmation messaging
- help-mode-driven contextual guidance

Out of scope:

- deep accessibility review of every strategy/result panel
- options-specific copy
- visual design changes that do not affect semantics or guidance

## Current User-Facing Behavior

The shell already contains intentionally meaningful operator copy and semantics:

- command palette uses combobox/listbox semantics and announces result counts
- workspace and workflow navigation use tab semantics
- runtime and diagnostic surfaces expose status/live-region style messaging
- confirmation dialogs present explicit consequences and focus management
- local search surfaces in history and settings clearly say they stay local and point global command search to `Ctrl+K`
- help mode (`full`, `concise`, `off`) already changes how contextual guidance appears

This means shell copy is already part of the product contract. The text is not decoration; it tells the user what is local, what is global, what is safe, and what action a control will take.

## Current Code Surface

- `web/src/components/CommandPaletteOverlay.vue`
  - combobox/listbox semantics, search summary, and command copy
- `web/src/components/CommandPaletteOverlay.test.ts`
  - accessibility and keyboard regression coverage
- `web/src/components/VueWorkspaceShell.vue`
  - tab semantics, selection announcements, and shell posture text
- `web/src/components/DesktopTitlebar.vue`
  - close-confirm copy and shell action labeling
- `web/src/components/ConfirmationDialog.vue`
  - shared alert-dialog structure and focus behavior
- `web/src/components/ContextHelpTip.vue`
  - help-mode-driven contextual guidance
- `web/src/composables/useHelpMode.ts`
  - central help-mode behavior
- `web/src/components/SettingsWorkspacePanel.vue`
  - local-search and guidance copy for settings surfaces
- `web/src/components/HistoryWorkspacePanel.vue`
  - local-search guidance for saved runs
- `web/src/components/AppRuntimeBanner.vue`
  - runtime status/error copy

## Current Verification Surface

- `web/src/components/CommandPaletteOverlay.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`

## Strengths Already Present

- several shell surfaces already have explicit semantic tests instead of only visual checks
- local versus global search scope is already clearly communicated
- confirmation dialogs already manage focus and consequence wording
- help mode is already centralized instead of being ad hoc per component
- runtime and shell state surfaces already expose status text deliberately

## Weaknesses / Failure Modes

- copy and semantics are spread across many shell components with no single regression contract
- contextual help has a real mode system, but its shell-wide expectations are not documented in one place
- accessibility regressions can arrive through wording changes, not just missing attributes
- some shell surfaces depend on indirect coverage rather than dedicated accessibility tests

## Regressions To Avoid

- weakening tab, dialog, combobox, listbox, or status semantics already present
- removing or diluting critical operator guidance such as local-search scope or close consequences
- letting help-mode behavior drift between shell surfaces
- introducing ambiguous copy around shell navigation, reports, or destructive actions

## External Research Summary

- [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because shell navigation already uses tablists and tabpanels
- [WAI-ARIA modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because shared confirmation surfaces are already modal and focus-managed
- [MDN ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because runtime and shell announcements behave like status surfaces
- [WAI-ARIA tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
  - relevant because contextual help is already part of the shell guidance system

## Proposed Improvement Approach

1. Freeze a shell-level accessibility and copy contract.
2. Treat operator wording as regression-sensitive in these areas:
   - local versus global search
   - destructive confirmation wording
   - runtime/status announcements
   - contextual help posture
3. Keep semantic ownership tied to the current shared shell components.
4. Add targeted tests where semantics exist but are only indirectly covered today.
5. Prefer additive clarification over churning the shell’s working vocabulary.

## Checklist

- [ ] Document the shell’s current semantic surfaces and their owners.
- [ ] Document local/global search guidance as a contract item.
- [ ] Document close-confirm and runtime-status wording as regression-sensitive copy.
- [ ] Document help-mode-driven contextual guidance expectations.
- [ ] Add or tighten tests when shell semantics or operator copy changes.

## Verification Plan

- run `npm --prefix web test -- src/components/CommandPaletteOverlay.test.ts`
- run `npm --prefix web test -- src/components/VueWorkspaceShell.test.ts`
- run `npm --prefix web test -- src/components/DesktopTitlebar.test.ts`
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/HistoryWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep shell copy changes deliberate and test-backed
- if wording changes alter scope or meaning, update tests and this doc together
- prefer strengthening existing semantic patterns over inventing inconsistent new ones

## Related Docs

- `REPO_Improvement/01_shell/04_shell_command_palette_2026-04-09.md`
- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/20_shell_shared_dialogs_2026-04-09.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-09.md`
