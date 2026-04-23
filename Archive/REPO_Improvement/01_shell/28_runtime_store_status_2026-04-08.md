# Runtime Store Status Plan

Last updated: 2026-04-09

## Purpose

Define the current `runtimeStore` status contract so boot state, desktop-runtime state, and shell-facing runtime truth stay stable and do not drift across `main.ts`, banner UI, settings UI, and desktop-productization hooks.

## Scope Boundaries

In scope:

- `web/src/stores/runtimeStore.ts`
- boot phase and boot stage state
- runtime error capture
- desktop log/status state
- desktop notification and window-state readiness flags

Out of scope:

- task-center task lifecycle state in `taskStore`
- options surfaces
- adding new runtime phases or new shell features

## Current User-Facing Behavior

The app currently exposes one shared runtime status surface for the shell. During startup, `main.ts` drives `bootPhase` and `bootStage` through `booting`, `ready`, or `error`. The runtime banner uses that state to show boot progress or boot failure details. The settings workspace also reuses the same store to show boot state, desktop notification permission, desktop window-state readiness, and recent desktop log lines. The result is that the shell already has one de facto runtime source of truth, but that contract is implicit rather than documented.

## Current Code Surface

- `web/src/stores/runtimeStore.ts`
  - owns `bootPhase`, `bootStage`, `bootStartedAt`, `bootReadyAt`, `bootError`, `bootErrorDetail`, desktop log storage, window-state readiness, and desktop-notification permission state
- `web/src/main.ts`
  - drives the boot lifecycle with `markBooting`, `setBootStage`, `markReady`, and `markError`
- `web/src/main.test.ts`
  - verifies the current boot progression and ready-state timing
- `web/src/components/AppRuntimeBanner.vue`
  - consumes `runtimeStore` boot fields to render the runtime banner
- `web/src/components/SettingsWorkspacePanel.vue`
  - exposes runtime phase, desktop logs, notification permission, and task-center/context details to the user
- `web/src/services/desktopProductization.ts`
  - writes desktop runtime state into the store, including log-stream attachment, notification permission, and window-state persistence readiness
- `web/src/stores/domainStores.test.ts`
  - covers basic runtime-store action behavior
- `web/src/components/SettingsWorkspacePanel.test.ts`
  - verifies user-visible rendering of runtime state and desktop logs

## Current Verification Surface

- `web/src/main.test.ts`
- `web/src/stores/runtimeStore.test.ts`
- `web/src/stores/domainStores.test.ts`
- `web/src/components/AppRuntimeBanner.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/components/StartupAnimationOverlay.test.ts`
- `web/src/services/desktopProductization.test.ts`

## Strengths Already Present

- `runtimeStore` is already a single Pinia store instead of fragmented shell-local refs
- boot timing and error capture are already explicit fields, not inferred from DOM state
- desktop log retention is intentionally bounded with `MAX_DESKTOP_LOGS = 24`
- settings and runtime banner already consume the same source of truth
- `markError()` already preserves both a user-facing error string and a richer detail string

## Weaknesses / Failure Modes

- runtime truth is split conceptually between `runtimeStore` and `taskStore`, but that boundary is not documented clearly
- `legacyMounted` is easy to overlook because it is part of shell state but not part of the visible runtime banner contract
- boot state is small enough to look simple, but multiple shell surfaces rely on the exact wording and timing semantics
- desktop logs are operationally important, but their retention policy and ordering contract are not currently written down
- notification permission and window-state readiness are shell-facing status fields, but they are updated indirectly through `desktopProductization.ts`

## Regressions To Avoid

- adding new runtime fields without deciding whether they belong in `runtimeStore` or `taskStore`
- changing boot-phase wording or semantics without updating banner/settings tests
- weakening current error-detail capture into a single flat string
- changing desktop-log ordering or retention silently
- letting desktop-productization state bypass the store and become component-local
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia: State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because `runtimeStore` is a state-ownership boundary, not just a convenient object
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because boot and desktop-runtime transitions are currently expressed through store actions and should remain disciplined
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because this shell-level runtime state is shared across multiple surfaces and should stay centralized
- [Tauri: State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because the frontend runtime store mirrors desktop/runtime state that is initialized around the Tauri app lifecycle
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Define the store contract by field family:
   - boot lifecycle
   - boot timing
   - boot failure details
   - desktop runtime logs
   - window-state readiness
   - desktop notification permission
2. Document which writer owns each field today:
   - `main.ts`
   - `desktopProductization.ts`
   - direct runtime-store actions from tests or shell flows
3. Define what `runtimeStore` owns that `taskStore` does not, so shell status truth does not get duplicated later.
4. Treat the current visible consumers as part of the contract: runtime banner, settings workspace, and startup tests.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- added direct `web/src/stores/runtimeStore.test.ts` coverage for legacy mount state, desktop log retention and clearing, console-bridge state, window-state readiness, and notification-permission setters
- verified the current ownership boundary remains intact across `main.ts`, `desktopProductization.ts`, `AppRuntimeBanner.vue`, and `SettingsWorkspacePanel.vue`
- kept the runtime truth split explicit: `runtimeStore` owns boot/runtime diagnostics and desktop readiness state, while `taskStore` remains the task-center lifecycle source of truth

## Checklist

- [x] Enumerate all current `runtimeStore` fields and getters.
- [x] Document which code path currently writes each field.
- [x] Document boot lifecycle semantics for `idle`, `booting`, `ready`, and `error`.
- [x] Document desktop log ordering, retention cap, and clearing behavior.
- [x] Document notification-permission and window-state readiness semantics.
- [x] Document the current role of `legacyMounted`.
- [x] Document the ownership boundary between `runtimeStore` and `taskStore`.
- [x] Add or tighten tests if runtime-field ownership or wording changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/main.test.ts`
- run `npm --prefix web run test -- src/stores/domainStores.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/desktopProductization.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current field names and shell semantics unless a coordinated migration is planned
- keep runtime status additive; do not remove current visible state just to simplify the store
- if runtime state must move, update all current consumers in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
