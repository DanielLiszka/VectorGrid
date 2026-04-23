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

- `web/src/composables/useShellMount.test.ts`
- indirect panel/component tests that rely on successful teleport readiness
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/src/components/DrillWorkspacePanel.test.ts`

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue async components](https://vuejs.org/guide/components/async.html)
  - relevant because many shell panels are lazy and mount into teleported hosts after shell composition is ready
- [Vue performance best practices](https://vuejs.org/guide/best-practices/performance.html)
  - relevant because deferred shell mounting should stay explicit and low-overhead
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because mount readiness should stay infrastructural rather than becoming another state authority
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because shell-mount issues often show up best in real rendered flows
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

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
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document current `useShellMount` behavior and options.
- [x] Document which panels depend on shell mount readiness.
- [x] Add dedicated test coverage for mount-ready and cleanup behavior.
- [x] Preserve pane dataset marker add/remove behavior.
- [x] Keep host selectors synchronized with shell mount targets.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/composables/useShellMount.test.ts`
- run `npm --prefix web run test -- src/components/DrillWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- run representative panel tests that depend on teleport readiness
- manually verify panel hosts still receive teleported content after shell changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep `useShellMount` simple and infrastructural
- if host resolution becomes more complex later, preserve existing panel selectors until replacement coverage exists
- prefer explicit diagnostics over silent fallback rendering into the wrong host
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/22_shell_debug_workspace_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
