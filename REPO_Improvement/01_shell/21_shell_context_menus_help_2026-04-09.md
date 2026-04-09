# Shell Context Menus Help Plan

Last updated: 2026-04-09

## Purpose

Define the current shared context-menu and contextual-help contract so menus, tooltips, hover-card help, and help-mode behavior can be improved without regressing local discoverability or accessibility.

## Scope Boundaries

In scope:

- shared `ContextActionMenu`
- shared `ContextHelpTip`
- help-mode behavior and shell guidance copy
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
  - shared help affordance with full/concise/off behavior
- `web/src/composables/useHelpMode.ts`
  - central help-mode state consumption
- `web/src/stores/settingsStore.ts`
  - source of the `helpMode` preference
- current consumers include:
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`

## Current Verification Surface

- `web/src/components/ContextActionMenu.test.ts`
- `web/src/components/ContextHelpTip.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- indirect coverage through consumer panel tests

## Strengths Already Present

- both shared primitives already exist and are reused
- menu semantics, escape handling, and focus return are already tested for the context menu
- help mode is centralized instead of scattered
- the shell already distinguishes full, concise, and off guidance modes
- contextual help is reusable across multiple workflows without bespoke implementations

## Weaknesses / Failure Modes

- there is no direct `ContextHelpTip` regression test
- help and context menus are related in practice but not documented as one shell-assistance surface
- help-mode expectations across shell surfaces are not yet written down in one place
- future contributors may invent ad hoc help or menu surfaces because the current boundary is implicit

## Regressions To Avoid

- breaking full/concise/off help-mode behavior
- breaking focus return or escape behavior in shared context menus
- removing shared contextual help in favor of inconsistent local implementations
- adding one-off tooltip or menu implementations where shared primitives should be used
- weakening shell guidance copy around local versus global actions
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA Authoring Practices: menu button](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/)
  - relevant because the shared action menu behaves like a contextual menu-button surface
- [WAI-ARIA tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
  - relevant because concise contextual help is already tooltip-like
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because help mode should remain centralized and shared
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because help-mode behavior is already store-backed

## Proposed Improvement Approach

1. Treat context menus and contextual help as one shell-assistance family with two shared primitives.
2. Freeze the current contracts:
   - `ContextActionMenu` for local action menus
   - `ContextHelpTip` for local explanatory guidance
3. Document help-mode behavior and the shared-help component contract.
4. Add a dedicated `ContextHelpTip` test.
5. Require new contextual assistance to use the shared primitives unless there is a real reason not to.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document current full/concise/off help behavior.
- [x] Document the current shared help component and its consumers.
- [x] Document the current shared context-menu behavior and consumers.
- [x] Add direct regression coverage for `ContextHelpTip.vue`.
- [x] Preserve centralized help-mode behavior in the settings store.
- [x] Reuse shared primitives for new local menus/help instead of adding one-offs.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/ContextHelpTip.test.ts`
- run `npm --prefix web run test -- src/components/ContextActionMenu.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- run representative consumer panel tests that use shared contextual help
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep this doc honest to the current shell state
- preserve help-mode compatibility even if contextual-help presentation evolves
- if richer contextual assistance is added later, keep the shared primitive boundaries explicit rather than silently broadening them
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/09_settings_appearance_layout_2026-04-09.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-09.md`
- `REPO_Improvement/01_shell/20_shell_shared_dialogs_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
