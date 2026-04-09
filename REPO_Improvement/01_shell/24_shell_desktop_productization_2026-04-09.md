# Shell Desktop Productization Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the current desktop-native shell bridges so logging, window-state persistence, notification routing, and runtime diagnostics stay additive and testable without disturbing the web-first shell behavior.

## Scope Boundaries

In scope:

- desktop startup bridge initialization
- window-state persistence
- desktop log attachment and console mirroring
- task-to-notification routing and runtime diagnostics exposure

Out of scope:

- new desktop plugins not already adopted in the repo
- OS-specific feature expansion outside the current desktop surface
- options workflows

## Current User-Facing Behavior

Desktop productization is already a real runtime layer. `web/src/main.ts` calls `initializeDesktopProductization()` during startup after settings and workflow bootstrap. `web/src/services/desktopProductization.ts` attaches Tauri log streaming, optional dev-only console mirroring, window-state persistence, desktop notification permission refresh, and notification routing based on task completion and visibility. The runtime store records desktop logs, permission state, and window-state attachment, while `SettingsWorkspacePanel.vue` exposes those diagnostics and a manual permission request path.

That means the app already has a meaningful desktop shell contract:

- browser-only execution can skip desktop bridges through `tauriRuntimeAvailable()`
- desktop startup failures are downgraded to warnings rather than fatal boot errors
- Task Center remains the source of truth while OS notifications are supplemental
- the settings panel is already the user-facing diagnostics surface for desktop behavior

## Current Code Surface

- `web/src/main.ts`
- `web/src/services/desktopProductization.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.test.ts`

## Current Verification Surface

- `web/src/main.test.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`

## Strengths Already Present

- desktop bridges are already centralized in one service rather than scattered across components
- the runtime store already captures desktop logs, permission posture, and window-state metadata
- desktop behavior is already visible to the user through settings diagnostics instead of staying hidden
- desktop startup does not block the shell on non-fatal bridge issues

## Weaknesses / Failure Modes

- desktop productization mixes several responsibilities in one service file
- there is no single implementation doc that freezes which desktop bridges are required versus optional
- notification routing, logging, and window-state persistence each depend on shared runtime/task/settings assumptions that can drift
- the current degraded-startup posture is test-covered but not yet documented as a product policy

## Regressions To Avoid

- turning non-fatal desktop bridge errors into app-killing failures
- breaking the web/runtime-available guard and making browser-side tests depend on desktop plugins
- losing desktop logs or window-state diagnostics from the settings surface
- reversing the current “Task Center first, OS notifications second” posture
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the desktop shell bridges sit at the frontend/runtime boundary and should stay explicit and testable.
- [Tauri Notification Plugin](https://v2.tauri.app/plugin/notification/) is relevant because current desktop behavior already depends on permission and delivery semantics from that plugin.
- [Tauri Window State Plugin](https://v2.tauri.app/plugin/window-state/) is relevant because window-state persistence is already a shipped desktop responsibility rather than a future idea.
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Document the current desktop bridge inventory and classify each bridge as required, optional, dev-only, or degraded-safe.
2. Separate startup ordering from long-lived watcher behavior so desktop boot semantics are easier to reason about.
3. Freeze the current diagnostic contract between desktop bridges, runtime store, and settings diagnostics.
4. Keep browser-safe guards explicit and verify that desktop productization remains no-op safe when Tauri is unavailable.
5. Add a policy note that OS notifications complement Task Center instead of replacing it.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Inventory every current desktop bridge installed by `desktopProductization.ts`.
- [ ] Document the startup order and degraded-failure semantics for each bridge.
- [ ] Document how desktop diagnostics flow into `runtimeStore` and the settings panel.
- [ ] Record which behaviors are dev-only, desktop-only, or always-on.
- [ ] Add or tighten tests anywhere plugin failure currently relies on implicit behavior.
- [ ] Preserve browser-safe startup and desktop-unavailable handling.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/main.test.ts`
- run `npm --prefix web run test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- perform one manual desktop smoke pass covering logs, notification posture, and persisted window state after any refactor
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the current bridges intact unless an equivalent replacement is documented and verified
- prefer extracting responsibilities only after the current contract is frozen in docs and tests
- maintain the current warning-first posture for non-fatal desktop bridge failures
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/01_shell/13_settings_desktop_notifications_2026-04-08.md`
