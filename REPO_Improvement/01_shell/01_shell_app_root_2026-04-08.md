# Shell App Root Plan

Last updated: 2026-04-08

## Purpose

Define the current `App.vue` root-composition contract so shell layering, panel mounting, and root-level visibility gates can be improved without breaking the existing desktop shell topology.

## Scope Boundaries

In scope:

- `web/src/App.vue` root composition
- root-level mount order of titlebar, shell, overlays, runtime banner, docks, and panels
- root-level lazy-panel mounting and workspace/workflow gating

Out of scope:

- deep behavior of individual workspace panels
- options-related root surfaces
- global app boot sequence outside root composition

## Current User-Facing Behavior

`App.vue` already acts as a real shell router. Current behavior includes:

- the startup animation overlay remains imported but is disabled behind `startupAnimationEnabled = false`
- the desktop titlebar, workspace shell, command palette, global shortcut bridge, runtime banner, task center, docks, panels, and footer are mounted from one root
- most heavyweight panels are async components
- some panels (`data`, `optimize`, `overview`) are mounted lazily once active, then kept mounted
- workspace and workflow state decide which docks and panels exist at all
- the workflow quick switch is dev-only and can be enabled by query param or local storage

This makes `App.vue` more than a trivial root. It is the root composition contract for the current desktop shell.

## Current Code Surface

- `web/src/App.vue`
  - root mount order, async imports, panel gating, lazy mount-once behavior
- `web/src/components/VueWorkspaceShell.vue`
  - shell header/grid/tab structure mounted by the root
- `web/src/components/DesktopTitlebar.vue`
  - desktop chrome surface mounted above the shell
- `web/src/components/AppRuntimeBanner.vue`
  - runtime boot/error banner mounted globally
- `web/src/components/TaskCenterOverlay.vue`
  - global task overlay mounted from the root
- `web/src/main.ts`
  - boot path that mounts the Vue app and runtime stores before `App.vue` takes over
- `web/src/main.test.ts`
  - verifies critical boot readiness behavior

## Current Verification Surface

- `web/src/main.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- dock-specific tests such as `web/src/components/AnalysisContextDock.test.ts`

## Strengths Already Present

- root composition is centralized instead of spread across multiple entrypoints
- async component loading is already used for heavier panels
- root gates already prevent most panels from mounting outside the right shell posture
- lazy mount-once behavior reduces remount churn for selected panels
- startup animation is disabled safely without deleting the component

## Weaknesses / Failure Modes

- root composition is dense and therefore easy to change without documenting the layering contract
- mount order matters for overlays and runtime status, but that order is not yet described in one doc
- some panels are mount-once while others are transient, and the rule is implicit
- dev-only workflow quick-switch behavior can leak into root complexity if not documented clearly

## Regressions To Avoid

- re-enabling the startup overlay unintentionally
- breaking the current root mount order for titlebar, shell, overlays, and footer
- changing root panel gating so panels mount in the wrong workspace/workflow context
- turning mount-once panels back into remount-on-every-visit surfaces without deliberate review
- leaking dev-only quick-switch behavior into production posture

## External Research Summary

- [Vue async components](https://vuejs.org/guide/components/async.html)
  - relevant because `App.vue` already uses async component boundaries for root-managed panels
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because root composition depends on centralized store truth rather than ad hoc component state
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
  - relevant because the app root sits on top of runtime/bootstrap state that must remain centralized
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because root-layer regressions are best caught through realistic shell-level verification, not only narrow unit checks

## Proposed Improvement Approach

1. Freeze the root-layer topology:
   - titlebar
   - workspace shell
   - overlays
   - runtime banner
   - docks
   - panels
   - footer
2. Document which root-managed panels are:
   - always mounted
   - lazily mounted once
   - conditionally mounted transiently
3. Keep root gating declarative and store-backed.
4. Preserve the current disabled-startup-overlay state unless a deliberate product decision reopens it.

## Checklist

- [ ] Document the current root mount order and why it matters.
- [ ] Document which panels are async and which are lazy mount-once.
- [ ] Document root-level gating for docks and major panels.
- [ ] Document dev-only workflow quick-switch behavior separately from production shell behavior.
- [ ] Preserve the current disabled startup-overlay posture unless intentionally changed.

## Verification Plan

- run `npm --prefix web test -- src/main.test.ts`
- run `npm --prefix web test -- src/components/VueWorkspaceShell.test.ts`
- run `npm --prefix web test -- src/components/DesktopTitlebar.test.ts`
- manually verify root shell layering, dock gating, and no-startup-overlay behavior

## Rollout / Fallback / Compatibility Notes

- keep root changes additive and layering-safe
- if a root surface is moved or removed later, update this doc and the shell verification surfaces together
- preserve current workspace/workflow gates unless a replacement is equally explicit

## Related Docs

- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/19_startup_overlay_disposition_DATE.md`
