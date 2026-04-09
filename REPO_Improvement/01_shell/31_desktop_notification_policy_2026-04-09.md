# Desktop Notification Policy Plan

Last updated: 2026-04-09

## Purpose

Define the current desktop-notification policy so permission posture, important-command routing, and background-only delivery can be improved without spamming users or weakening Task Center as the primary status surface.

## Scope Boundaries

In scope:

- desktop notification mode settings
- permission refresh and manual permission request behavior
- task classification into notification intent
- background-only and terminal-status delivery rules

Out of scope:

- new notification categories not already present
- mobile/browser push systems
- options-specific notification behavior

## Current User-Facing Behavior

The app already applies a bounded notification policy. `settingsStore.ts` persists a `desktopNotificationMode` of `off`, `failures`, or `important`. `desktopProductization.ts` refreshes permission state on startup, allows manual permission requests, watches the latest task in `taskStore.history`, and only calls `sendNotification()` when the task resolves to a `DesktopNotificationIntent`. `desktopNotificationPolicy.ts` suppresses notifications when the document is visible, only considers terminal tasks, filters success notifications to an explicit allowlist of important commands, and keeps Task Center as the primary detail surface.

Current user-facing posture therefore includes:

- notifications do not fire while the window is visible
- “failures” mode suppresses success notifications entirely
- “important” mode allows success notifications only for a bounded set of commands
- the settings panel shows current permission state and offers an explicit request action

## Current Code Surface

- `web/src/utils/desktopNotificationPolicy.ts`
- `web/src/utils/desktopNotificationPolicy.test.ts`
- `web/src/services/desktopProductization.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/components/SettingsWorkspacePanel.vue`

## Current Verification Surface

- `web/src/utils/desktopNotificationPolicy.test.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`

## Strengths Already Present

- success notifications are already gated by an explicit important-command allowlist
- background-only delivery already avoids duplicate on-screen noise while the app is in focus
- permission posture is already visible in the UI and runtime diagnostics
- Task Center remains the canonical detail surface instead of trying to encode full state into OS notifications

## Weaknesses / Failure Modes

- the important-command set is code-owned but not yet frozen in a dedicated implementation doc
- policy logic depends on task labels, visibility, and store timing, so regressions can present as duplicate or missing notifications
- settings copy and policy rules can drift if the allowlist changes without UI updates
- the runtime currently tracks permission state but not a richer notification delivery audit

## Regressions To Avoid

- sending notifications while the app window is visible
- loosening success-notification filtering so routine commands become noisy
- bypassing permission checks or treating unsupported platforms as granted
- making OS notifications the only place where important task outcomes are visible
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Notification Plugin](https://v2.tauri.app/plugin/notification/) is relevant because the shipped desktop behavior depends on its permission and delivery APIs.
- [MDN Notifications API](https://developer.mozilla.org/en-US/docs/Web/API/Notifications_API) is relevant because user expectations around permission prompts and background delivery are shaped by the broader notification model.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because policy clarity depends on keeping settings, runtime permission state, and task history synchronized through shared state instead of component-local copies.
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Document the exact policy inputs: mode, document visibility, task terminal state, command allowlist, and permission posture.
2. Freeze the current important-command success allowlist and require explicit review for future changes.
3. Clarify the difference between policy eligibility and actual notification delivery success.
4. Keep Task Center as the first-class status surface and treat notifications as supplemental attention cues.
5. Add direct checks wherever settings wording and policy logic can drift apart.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the three desktop notification modes and their current semantics.
- [ ] Document the current success-command allowlist for notifications.
- [ ] Document background-only suppression when the app is visible.
- [ ] Document permission refresh and manual permission request behavior.
- [ ] Add or tighten tests for visible-window suppression, failures-only mode, and unsupported platforms.
- [ ] Keep Task Center referenced as the authoritative detail surface in UI copy and docs.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/utils/desktopNotificationPolicy.test.ts`
- run `npm --prefix web run test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- perform one desktop smoke check with the window hidden and visible to confirm suppression behavior after policy changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- prefer additive policy clarification and tests before changing the allowlist or notification modes
- avoid changing mode names or stored values without a compatibility migration
- keep unsupported-platform behavior explicit rather than silently pretending notifications are available
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/24_shell_desktop_productization_2026-04-09.md`
- `REPO_Improvement/01_shell/13_settings_desktop_notifications_2026-04-08.md`
- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
