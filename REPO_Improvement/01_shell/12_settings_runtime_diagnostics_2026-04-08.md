# Settings Runtime Diagnostics Plan

Last updated: 2026-04-09

## Purpose

Define the current Settings runtime-diagnostics surface so boot state, desktop log state, notification permission, window-state readiness, and task posture summaries can be improved without breaking the current diagnostics truth the shell already exposes.

## Scope Boundaries

In scope:

- runtime-diagnostics sections inside `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/stores/runtimeStore.ts`
- `web/src/services/desktopProductization.ts`
- runtime/task summaries shown in Settings

Out of scope:

- global runtime banner layout
- task-center deep interaction model
- new desktop plugins or new diagnostics categories

## Current User-Facing Behavior

The settings workspace already doubles as a practical desktop diagnostics page. It shows runtime phase, boot detail, boot error detail, desktop-notification permission, window-state readiness, recent desktop logs, recent timeline entries, recent alerts, latest task outcome, task posture counts, and a generated desktop diagnostics snapshot string. It also exposes a user action to request desktop-notification permission and a direct path into the task center. That means Settings is already the shipped diagnostics console for the non-options desktop shell.

## Current Code Surface

- `web/src/components/SettingsWorkspacePanel.vue`
  - renders runtime diagnostics, task posture summaries, desktop logs, and diagnostics-copy snapshot content
- `web/src/components/SettingsWorkspacePanel.test.ts`
  - verifies user-visible rendering of settings/runtime content
- `web/src/stores/runtimeStore.ts`
  - owns boot phase/stage/error, desktop logs, window-state readiness, and notification permission
- `web/src/services/desktopProductization.ts`
  - feeds desktop logs, window-state metadata, and notification permission into the runtime store
- `web/src/services/desktopProductization.test.ts`
  - verifies desktop logging and notification-routing behavior
- `web/src/main.ts`
  - drives booting/ready/error state that Settings reads
- `web/src/main.test.ts`
  - verifies boot lifecycle staging
- `web/src/stores/taskStore.ts`
  - supplies task posture, timeline, alerts, and latest task data that Settings summarizes

## Current Verification Surface

- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/stores/domainStores.test.ts`
- `web/src/main.test.ts`

## Strengths Already Present

- diagnostics already reuse centralized runtime/task stores instead of scraping the DOM
- the settings page exposes both concise status labels and richer detail strings
- desktop logs are bounded and recent-first
- notification permission and window-state readiness are already user-visible
- the diagnostics snapshot export is already structured enough to be operationally useful

## Weaknesses / Failure Modes

- Settings currently mixes configuration and operational diagnostics in one large panel
- several diagnostics sections summarize `taskStore` truth indirectly, which can make ownership feel blurry
- runtime and desktop diagnostics wording is already important, but not yet documented as user-facing contract
- the copyable diagnostics snapshot is valuable but currently implicit rather than specified
- the component carries a large amount of diagnostics support logic inline

## Regressions To Avoid

- weakening boot-error detail or collapsing it into a generic string
- losing notification-permission visibility or request flow
- changing desktop-log ordering or filter behavior silently
- breaking task posture summaries that Settings already surfaces
- turning the diagnostics snapshot into an incomplete or misleading summary
- sending OS notifications while the window is visible or preferences are off
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because the page summarizes desktop/runtime state from centralized stores
- [Tauri Plugin: Notification](https://v2.tauri.app/plugin/notification/)
  - relevant because notification permission is part of the visible diagnostics surface
- [Tauri Plugin: Window State](https://v2.tauri.app/plugin/window-state/)
  - relevant because Settings surfaces window-state persistence readiness
- [MDN: ARIA `status` Role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because several diagnostics areas are read as live status output
- [Pinia: State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because diagnostics truth is already store-backed
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Treat Settings diagnostics as one documented operational surface with explicit subsections:
   - runtime boot
   - desktop readiness
   - task posture
   - recent timeline/alerts
   - desktop logs
   - copyable diagnostics snapshot
2. Document which fields are authoritative vs summarized:
   - `runtimeStore`
   - `taskStore`
   - derived summary composables
3. Extract future diagnostics-copy and card-state helpers into a dedicated composable/service instead of growing the component further.
4. Keep improvements additive:
   - clearer copy
   - stronger tests
   - better export structure
   - but no loss of current diagnostics coverage
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the runtime diagnostics sections currently shown in Settings.
- [ ] Document which store fields feed each section.
- [ ] Document notification-permission and window-state readiness semantics.
- [ ] Document desktop-log filtering and retention behavior as seen in Settings.
- [ ] Document the current diagnostics snapshot output as a supported troubleshooting surface.
- [ ] Add or tighten tests if diagnostics wording, grouping, or actions change.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web run test -- src/stores/domainStores.test.ts`
- run `npm --prefix web run test -- src/main.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current diagnostics coverage even if the Settings layout is refactored
- keep runtime/task sources centralized; do not move diagnostics truth into the component
- if a diagnostics section is split out later, keep the Settings summary intact until replacement UX exists
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
