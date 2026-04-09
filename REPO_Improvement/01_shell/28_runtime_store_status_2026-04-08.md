# Runtime Store Status Plan

Last updated: 2026-04-08

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
- `web/src/stores/domainStores.test.ts`
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

## External Research Summary

- [Pinia: State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because `runtimeStore` is a state-ownership boundary, not just a convenient object
- [Pinia: Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because boot and desktop-runtime transitions are currently expressed through store actions and should remain disciplined
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because this shell-level runtime state is shared across multiple surfaces and should stay centralized
- [Tauri: State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because the frontend runtime store mirrors desktop/runtime state that is initialized around the Tauri app lifecycle

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

## Checklist

- [ ] Enumerate all current `runtimeStore` fields and getters.
- [ ] Document which code path currently writes each field.
- [ ] Document boot lifecycle semantics for `idle`, `booting`, `ready`, and `error`.
- [ ] Document desktop log ordering, retention cap, and clearing behavior.
- [ ] Document notification-permission and window-state readiness semantics.
- [ ] Document the current role of `legacyMounted`.
- [ ] Document the ownership boundary between `runtimeStore` and `taskStore`.
- [ ] Add or tighten tests if runtime-field ownership or wording changes.

## Verification Plan

- run `npm --prefix web test -- src/main.test.ts`
- run `npm --prefix web test -- src/stores/domainStores.test.ts`
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/desktopProductization.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current field names and shell semantics unless a coordinated migration is planned
- keep runtime status additive; do not remove current visible state just to simplify the store
- if runtime state must move, update all current consumers in the same change

## Related Docs

- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
