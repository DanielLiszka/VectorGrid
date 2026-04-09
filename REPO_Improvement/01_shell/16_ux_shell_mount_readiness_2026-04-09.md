# UX Shell Mount Readiness Plan

Last updated: 2026-04-09

## Purpose

Define the current shell-mount readiness contract so teleported workspace panels and mount markers can be improved without breaking the shell’s current panel-host model.

## Scope Boundaries

In scope:

- `useShellMount.ts`
- teleport-ready gating for shell panels
- pane dataset markers and mount-host coordination

Out of scope:

- deep panel behavior after mount
- root app composition outside shell mount targets
- options-specific mount hosts

## Current User-Facing Behavior

Many shell panels already defer their teleported content until the intended mount host exists. Current behavior includes:

- waiting until after mount/next tick before resolving the target
- exposing `mountReady` to gate teleport rendering
- optionally tagging the pane root with a dataset flag
- cleaning up pane dataset markers on unmount

This composable is already used broadly across the shell:

- data
- optimize
- strategy
- advanced
- settings
- history
- drill
- debug
- meta
- validation
- footer

So shell-mount readiness is already a cross-shell infrastructure surface.

## Current Code Surface

- `web/src/composables/useShellMount.ts`
  - mount-host discovery, pane-root tagging, and cleanup
- workspace panels that consume it, including:
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/OptimizeWorkspacePanel.vue`
  - `web/src/components/SettingsWorkspacePanel.vue`
  - `web/src/components/HistoryWorkspacePanel.vue`
  - `web/src/components/DebugWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/RunFooterPanel.vue`
- `web/src/components/VueWorkspaceShell.vue`
  - defines the mount hosts that these teleports target

## Current Verification Surface

- indirect panel/component tests that rely on successful teleport readiness
- `web/src/components/VueWorkspaceShell.test.ts`
- there is currently no dedicated `useShellMount` test surface

## Strengths Already Present

- teleport mounting is already guarded instead of assuming hosts exist immediately
- pane dataset markers already provide lightweight mount-state diagnostics
- cleanup behavior already removes dataset markers on unmount
- the composable is simple and widely reusable

## Weaknesses / Failure Modes

- there is no dedicated test for `useShellMount`
- readiness relies on `nextTick` plus DOM query timing, which is practical but implicit
- failures to find a mount host are currently silent
- mount-host and pane-marker behavior are shared shell infrastructure, but not yet documented as contract

## Regressions To Avoid

- rendering teleported content before the host exists
- leaving stale pane dataset markers behind after unmount
- changing mount-host selectors in the shell without updating dependent panels
- making panel mount readiness depend on arbitrary timing instead of explicit host discovery

## External Research Summary

- [Vue async components](https://vuejs.org/guide/components/async.html)
  - relevant because many shell panels are lazy and mount into teleported hosts after shell composition is ready
- [Vue performance best practices](https://vuejs.org/guide/best-practices/performance.html)
  - relevant because deferred shell mounting should stay explicit and low-overhead
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because mount readiness should stay infrastructural rather than becoming another state authority
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because shell-mount issues often show up best in real rendered flows

## Proposed Improvement Approach

1. Freeze the host-readiness contract for teleported shell panels.
2. Document the responsibilities clearly:
   - host discovery
   - mount gating
   - pane dataset tagging
   - cleanup
3. Add a dedicated composable test for `useShellMount`.
4. Keep selector-based host lookup explicit and synchronized with `VueWorkspaceShell`.
5. If mount failures need operator visibility later, add explicit diagnostics rather than silent timing hacks.

## Checklist

- [ ] Document current `useShellMount` behavior and options.
- [ ] Document which panels depend on shell mount readiness.
- [ ] Add dedicated test coverage for mount-ready and cleanup behavior.
- [ ] Preserve pane dataset marker add/remove behavior.
- [ ] Keep host selectors synchronized with shell mount targets.

## Verification Plan

- add and run a dedicated `useShellMount` unit test
- run `npm --prefix web test -- src/components/VueWorkspaceShell.test.ts`
- run representative panel tests that depend on teleport readiness
- manually verify panel hosts still receive teleported content after shell changes

## Rollout / Fallback / Compatibility Notes

- keep `useShellMount` simple and infrastructural
- if host resolution becomes more complex later, preserve existing panel selectors until replacement coverage exists
- prefer explicit diagnostics over silent fallback rendering into the wrong host

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/22_shell_debug_workspace_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
