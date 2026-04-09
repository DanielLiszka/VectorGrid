# Shell Overlay Composables Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the shared overlay and shell-mount composables so task-center visibility, return-focus handling, shell-host mounting, and related overlay behavior stay predictable without breaking current keyboard flows.

## Scope Boundaries

In scope:

- `useTaskCenterVisibility.ts` open/close/toggle persistence and return-focus behavior
- `useShellMount.ts` host-resolution behavior for teleported panels
- overlay-related shell persistence through `localStorage`
- shared overlay expectations used by Task Center and teleported shell panels

Out of scope:

- new overlay features not already present
- full redesign of the Task Center UI
- options-specific overlays

## Current User-Facing Behavior

The shell already relies on shared composables for overlay-like behavior. `useTaskCenterVisibility.ts` persists Task Center open state in `localStorage`, keeps a shared window-level `Ref<boolean>`, and remembers the return-focus target so `TaskCenterOverlay.vue` can restore focus on close. `useShellMount.ts` resolves pane-specific shell hosts, marks pane dataset flags, and lets teleported panels wait until their mount target exists. Those patterns are already used across shell panels such as data, settings, overview, top, validation, optimize, and debug.

The current result is functional and user-facing:

- task center open state survives across mounts and restores focus on close
- teleported panels do not render into a missing host immediately
- shell panels use a consistent mount-ready gate instead of duplicating DOM probing everywhere

## Current Code Surface

- `web/src/composables/useTaskCenterVisibility.ts`
- `web/src/composables/useShellMount.ts`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/DebugWorkspacePanel.vue`

## Current Verification Surface

- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `panel-level tests that mount shell hosts before teleported content`

## Strengths Already Present

- overlay state and return-focus logic are already centralized for Task Center
- teleported panels already use a shared mount-resolver instead of repeated ad hoc host lookup
- persistent open state is already bounded to the browser window rather than global process state
- focus restore is already part of the current interaction contract

## Weaknesses / Failure Modes

- overlay-oriented behavior is spread across multiple composables with no single implementation doc tying them together
- localStorage persistence and in-memory fallbacks are implicit rather than documented as a contract
- late-host mounting and unmount cleanup rely on shared assumptions that can break silently during shell refactors
- there is not yet a small policy that distinguishes overlay visibility state from panel mount-readiness state

## Regressions To Avoid

- breaking Task Center return-focus behavior
- mounting teleported shell panels before their host exists
- leaving stale pane dataset flags behind after unmount
- changing persisted overlay keys or semantics without compatibility handling

## External Research Summary

- [Vue Teleport](https://vuejs.org/guide/built-ins/teleport) is directly relevant because several shell panels and overlays depend on out-of-tree mounting.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant because open-state persistence and compatibility behavior depend on browser storage semantics.
- [WAI-ARIA Dialog Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) is relevant because focus restoration and modal overlay control are part of the accessibility contract even when the implementation detail lives in composables.

## Proposed Improvement Approach

1. Document overlay persistence, return-focus, and mount-readiness as distinct responsibilities rather than one generic “overlay behavior” bucket.
2. Freeze the current Task Center open-state contract, including storage key behavior and fallback behavior when `window` is unavailable.
3. Freeze the current shell-host mount contract for teleported panels, including when dataset markers are applied and removed.
4. Add direct verification for the few shared overlay/mount invariants that multiple panels depend on.
5. Keep overlay composables additive and shell-safe rather than introducing one giant shared primitive that merges unrelated concerns.

## Checklist

- [ ] Document the current Task Center storage key and open/close/toggle contract.
- [ ] Document return-focus capture and consumption semantics.
- [ ] Document shell-host resolution and mount-ready behavior for teleported panels.
- [ ] Document pane dataset marker expectations and cleanup.
- [ ] Add or tighten tests for focus restore, persisted open state, missing hosts, and unmount cleanup.
- [ ] Record which panels currently depend on `useShellMount.ts`.

## Verification Plan

- run `npm --prefix web test -- src/components/TaskCenterOverlay.test.ts`
- run `npm --prefix web test -- src/components/VueWorkspaceShell.test.ts`
- run targeted panel tests that depend on `useShellMount.ts` after any composable change
- perform one keyboard-only manual pass for Task Center open, close, and focus return

## Rollout / Fallback / Compatibility Notes

- preserve existing storage keys and return-focus semantics unless a compatibility change is documented explicitly
- prefer new tests and docs before merging overlay composables together
- keep shell-host mount gating opt-in and panel-specific where the repo already expects it

## Related Docs

- `REPO_Improvement/01_shell/16_ux_shell_mount_readiness_2026-04-09.md`
- `REPO_Improvement/01_shell/20_shell_shared_dialogs_2026-04-08.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-08.md`
