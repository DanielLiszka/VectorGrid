# Shell Docks And Shortcuts Plan

Last updated: 2026-04-08

## Purpose

Define the current shell-level dock, overlay, and shortcut contract so command palette behavior, task-center behavior, context-dock summaries, and keyboard-driven shell actions can be improved without breaking focus flow, accessibility, or workflow truthfulness.

## Scope Boundaries

In scope:

- `web/src/App.vue`
- `web/src/components/GlobalShortcutBridge.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/OptimizeContextDock.vue`
- `web/src/components/AnalysisContextDock.vue`
- `web/src/components/HistoryContextDock.vue`
- `web/src/components/ValidationContextDock.vue`
- `web/src/composables/useTaskCenterVisibility.ts`

Out of scope:

- options overlays or options-specific shortcuts
- strategy-specific execution semantics
- adding brand-new shell actions that do not already exist

## Current User-Facing Behavior

The app already has a real shell-level quick-action layer. `Ctrl+K` opens a command palette with fuzzy search, combobox/listbox semantics, and keyboard selection. `Esc` closes the palette, closes the Polygon modal when it is open, clears drill selection, and can cancel an active run. `Ctrl+O`, `Ctrl+Shift+O`, `Ctrl+\`, `Ctrl+Enter`, `Ctrl+Shift+V`, `Ctrl+Shift+C`, `Ctrl+Shift+J`, `Ctrl+Shift+R`, and `Ctrl+Shift+M` already map to concrete workflow actions. The task center is also a real shell control, with persisted open/closed state, focus return, drag repositioning, and a dialog-style overlay panel. Separate optimize, analysis, history, and validation docks expose compressed state summaries and route the user back to the authoritative workspace when needed. `DesktopTitlebar` remains the visible shell chrome and also owns close-guard behavior when long-running work or live execution is active.

## Current Code Surface

- `web/src/App.vue`
  - mounts shell chrome, overlays, bridges, and docks
- `web/src/components/GlobalShortcutBridge.vue`
  - owns document-level keyboard shortcut dispatch and precedence rules
- `web/src/components/GlobalShortcutBridge.test.ts`
  - verifies current shortcut routing and `Escape` precedence
- `web/src/components/CommandPaletteOverlay.vue`
  - owns searchable shell-action inventory, grouped commands, and keyboard navigation
- `web/src/components/CommandPaletteOverlay.test.ts`
  - verifies focus, combobox/listbox semantics, fuzzy search, and keyboard travel
- `web/src/components/TaskCenterOverlay.vue`
  - owns task-center launcher, persistence, focus trap, drag state, and task posture views
- `web/src/components/TaskCenterOverlay.test.ts`
  - verifies persistence, focus handling, alert/timeline rendering, and routed actions
- `web/src/components/DesktopTitlebar.vue`
  - owns visible shell search/settings/run controls and desktop close behavior
- `web/src/components/DesktopTitlebar.test.ts`
  - verifies titlebar behaviors, including close-guard conditions
- `web/src/components/OptimizeContextDock.vue`
- `web/src/components/AnalysisContextDock.vue`
- `web/src/components/HistoryContextDock.vue`
- `web/src/components/ValidationContextDock.vue`
  - current summary-dock surfaces for workflow context
- `web/src/composables/useTaskCenterVisibility.ts`
  - owns shared task-center visibility state and persistence key

## Current Verification Surface

- `web/src/components/GlobalShortcutBridge.test.ts`
- `web/src/components/CommandPaletteOverlay.test.ts`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/AnalysisContextDock.test.ts`

## Strengths Already Present

- shortcut behavior is already centralized instead of being scattered across components
- command-palette semantics are already accessible and keyboard-driven
- task-center open state and return-focus behavior are already explicit
- the shell already distinguishes compressed summary surfaces from authoritative workspaces
- titlebar close safety is already aware of long-running work and live execution

## Weaknesses / Failure Modes

- shortcut coverage is split between the command palette inventory and `GlobalShortcutBridge`, so drift is possible
- dock visibility and placement rules are implicit and spread across separate components
- persistence keys for shell surfaces are stable in practice but not yet documented as shell contract
- `Escape` behavior already has layered precedence, but that precedence is not written down anywhere
- analysis uses persisted overlay state while the other docks are hard-coded fixed cards

## Regressions To Avoid

- breaking focus return when closing the task center or command palette
- letting keyboard shortcuts fire while the command palette is already handling navigation
- changing `Escape` precedence so palette/polygon close behavior regresses
- breaking persisted task-center open state or current dock persistence
- breaking the multiline-editor guard on `Ctrl+Enter`
- weakening titlebar close confirmation when live execution or long tasks are active

## External Research Summary

- [WAI-ARIA APG: Dialog (Modal) Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the task center and command palette both act like managed overlay/dialog surfaces
- [WAI-ARIA APG: Listbox Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
  - relevant because the command palette already exposes listbox semantics for command selection
- [MDN: `Window.localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because task-center and dock open state are persisted locally
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because these shell surfaces depend on centralized store truth instead of ad hoc component state

## Proposed Improvement Approach

1. Document the current shell action registry in one place:
   - keyboard shortcut
   - corresponding palette command
   - enabled/disabled gate
   - routed store/service action
2. Document overlay precedence and focus rules:
   - command palette
   - task center
   - Polygon modal
   - drill selection clear
   - cancel behavior
3. Document each dock as a summary surface:
   - when it is visible
   - which store fields it summarizes
   - which actions it may launch
   - which authoritative workspace still owns editing
4. Use future improvements to converge dock state handling and placement instead of leaving one persisted dock and several one-off fixed cards.

## Checklist

- [ ] Document the current keyboard shortcut inventory and precedence rules.
- [ ] Document which command-palette entries map to the same actions as global shortcuts.
- [ ] Document task-center persistence keys, return-focus behavior, and drag-state behavior.
- [ ] Document current optimize/analysis/history/validation dock visibility and action boundaries.
- [ ] Document titlebar close-guard interaction with active work.
- [ ] Add or tighten tests if shortcut routing, focus handling, or dock visibility changes.

## Verification Plan

- run `npm --prefix web test -- src/components/GlobalShortcutBridge.test.ts`
- run `npm --prefix web test -- src/components/CommandPaletteOverlay.test.ts`
- run `npm --prefix web test -- src/components/TaskCenterOverlay.test.ts`
- run `npm --prefix web test -- src/components/DesktopTitlebar.test.ts`
- run `npm --prefix web test -- src/components/AnalysisContextDock.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve existing shortcuts unless the command palette and tests are updated in the same change
- keep dock improvements summary-first; do not move editing responsibilities into docks casually
- preserve current persistence keys unless a coordinated storage migration is implemented

## Related Docs

- `REPO_Improvement/01_shell/05_shell_task_center_2026-04-08.md`
- `REPO_Improvement/01_shell/25_command_palette_store_2026-04-08.md`
- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`
