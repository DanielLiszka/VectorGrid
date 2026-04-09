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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current Task Center storage key and open/close/toggle contract.
- [ ] Document return-focus capture and consumption semantics.
- [ ] Document shell-host resolution and mount-ready behavior for teleported panels.
- [ ] Document pane dataset marker expectations and cleanup.
- [ ] Add or tighten tests for focus restore, persisted open state, missing hosts, and unmount cleanup.
- [ ] Record which panels currently depend on `useShellMount.ts`.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/TaskCenterOverlay.test.ts`
- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- run targeted panel tests that depend on `useShellMount.ts` after any composable change
- perform one keyboard-only manual pass for Task Center open, close, and focus return
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve existing storage keys and return-focus semantics unless a compatibility change is documented explicitly
- prefer new tests and docs before merging overlay composables together
- keep shell-host mount gating opt-in and panel-specific where the repo already expects it
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/16_ux_shell_mount_readiness_2026-04-09.md`
- `REPO_Improvement/01_shell/20_shell_shared_dialogs_2026-04-09.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-09.md`
