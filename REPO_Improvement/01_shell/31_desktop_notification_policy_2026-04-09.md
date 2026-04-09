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

## External Research Summary

- [Tauri Notification Plugin](https://v2.tauri.app/plugin/notification/) is relevant because the shipped desktop behavior depends on its permission and delivery APIs.
- [MDN Notifications API](https://developer.mozilla.org/en-US/docs/Web/API/Notifications_API) is relevant because user expectations around permission prompts and background delivery are shaped by the broader notification model.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because policy clarity depends on keeping settings, runtime permission state, and task history synchronized through shared state instead of component-local copies.

## Proposed Improvement Approach

1. Document the exact policy inputs: mode, document visibility, task terminal state, command allowlist, and permission posture.
2. Freeze the current important-command success allowlist and require explicit review for future changes.
3. Clarify the difference between policy eligibility and actual notification delivery success.
4. Keep Task Center as the first-class status surface and treat notifications as supplemental attention cues.
5. Add direct checks wherever settings wording and policy logic can drift apart.

## Checklist

- [ ] Document the three desktop notification modes and their current semantics.
- [ ] Document the current success-command allowlist for notifications.
- [ ] Document background-only suppression when the app is visible.
- [ ] Document permission refresh and manual permission request behavior.
- [ ] Add or tighten tests for visible-window suppression, failures-only mode, and unsupported platforms.
- [ ] Keep Task Center referenced as the authoritative detail surface in UI copy and docs.

## Verification Plan

- run `npm --prefix web test -- src/utils/desktopNotificationPolicy.test.ts`
- run `npm --prefix web test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`
- perform one desktop smoke check with the window hidden and visible to confirm suppression behavior after policy changes

## Rollout / Fallback / Compatibility Notes

- prefer additive policy clarification and tests before changing the allowlist or notification modes
- avoid changing mode names or stored values without a compatibility migration
- keep unsupported-platform behavior explicit rather than silently pretending notifications are available

## Related Docs

- `REPO_Improvement/01_shell/24_shell_desktop_productization_2026-04-09.md`
- `REPO_Improvement/01_shell/13_settings_desktop_notifications_2026-04-08.md`
- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
