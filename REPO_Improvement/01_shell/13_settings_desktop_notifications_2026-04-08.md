# Settings Desktop Notifications Plan

Last updated: 2026-04-08

## Purpose

Define the current desktop-notification contract so settings UX, task-routing policy, permission handling, and runtime diagnostics can be improved without regressing the current foreground-first behavior.

## Scope Boundaries

In scope:

- desktop-notification preference UX in `web/src/components/SettingsWorkspacePanel.vue`
- notification permission/readiness state in `web/src/stores/runtimeStore.ts`
- task-to-notification routing policy in `web/src/utils/desktopNotificationPolicy.ts`
- delivery/request logic in `web/src/services/desktopProductization.ts`

Out of scope:

- tray behavior or hide-to-tray behavior
- task-center deep interaction design
- new notification channels, action buttons, or reminder systems

## Current User-Facing Behavior

The app already exposes a real desktop-notification surface in Settings. Users can choose one of three modes:

- `Off`
- `Failures and cancellations`
- `Important completions and failures`

The current shipped behavior is intentionally conservative:

- OS notifications are supplemental to the in-app task center
- notifications are suppressed while the document is visible
- success notifications are limited to an allowlist of high-signal commands
- failures and cancellations remain visible even when success routing is stricter
- Settings shows current permission state and exposes a `Request OS notification permission` action

The copy in Settings already establishes the product posture: VectorGrid stays foreground-first and should not turn routine background work into notification spam.

## Current Code Surface

- `web/src/components/SettingsWorkspacePanel.vue`
  - renders the desktop-notification mode select, explanatory copy, permission label, and permission-request action
- `web/src/components/SettingsWorkspacePanel.test.ts`
  - verifies the notification control is rendered and permission can be requested
- `web/src/stores/settingsStore.ts`
  - persists the `settingsDesktopNotifications` selection through `desktopNotificationMode`
- `web/src/stores/runtimeStore.ts`
  - stores notification permission state and desktop diagnostic logs
- `web/src/utils/desktopNotificationPolicy.ts`
  - owns the current allowlist and the `off` / `failures` / `important` routing decision
- `web/src/services/desktopProductization.ts`
  - checks permission, requests permission, sends notifications, and writes delivery/permission outcomes to runtime logs
- `web/src/services/desktopProductization.test.ts`
  - verifies important-success delivery, visibility suppression, and `off` suppression

## Current Verification Surface

- `web/src/services/desktopProductization.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/main.test.ts`

## Strengths Already Present

- policy is centralized instead of being scattered across task producers
- visibility suppression is already explicit and tested
- the success allowlist keeps notifications high-signal
- permission status is already user-visible instead of hidden inside plugin errors
- failed delivery and permission outcomes are already logged into runtime diagnostics

## Weaknesses / Failure Modes

- the set of “important” commands is embedded in code and not documented as contract
- success-routing policy and Settings copy can drift if command coverage changes silently
- permission state, user preference, and actual delivery capability are related but not documented separately
- visibility-only suppression may be too implicit for future contributors who confuse visible, focused, and minimized states
- there is no dedicated implementation doc that states what counts as regression for notification behavior

## Regressions To Avoid

- sending OS notifications while the desktop surface is visible
- ignoring the `off` preference
- broadening success notifications to noisy commands without an explicit policy change
- removing or weakening permission visibility in Settings
- breaking the runtime diagnostic logs that currently explain delivery or permission failures

## External Research Summary

- [Tauri Notifications plugin](https://v2.tauri.app/plugin/notification/)
  - relevant because the current implementation follows Tauri’s permission-check, permission-request, then send flow
- [Tauri State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because permission/readiness state is store-backed and should remain centralized
- [Pinia state management guidance](https://pinia.vuejs.org/core-concepts/)
  - relevant because notification preference and runtime state should remain separate but synchronized surfaces
- [MDN ARIA status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because permission and readiness messages behave like status output in Settings and runtime diagnostics

## Proposed Improvement Approach

1. Freeze the current notification behavior into one explicit contract:
   - mode meanings
   - visibility gating
   - permission semantics
   - important-command eligibility
2. Separate the three layers clearly in the implementation plan:
   - user preference
   - runtime permission/readiness
   - delivery policy
3. Keep the notification policy foreground-first:
   - the task center stays authoritative
   - OS notifications remain supplemental
4. Add explicit coverage for command-allowlist drift:
   - document the current command set
   - add a targeted test whenever the allowlist changes
5. Prefer additive clarity:
   - clearer copy
   - stronger diagnostics
   - better tests
   - but no new notification classes unless they are separately justified

## Checklist

- [ ] Document the three notification modes as a stable user-facing contract.
- [ ] Document the current important-success command allowlist.
- [ ] Document visibility suppression and the difference between task-center truth and OS notification supplementation.
- [ ] Document permission request and permission-label semantics.
- [ ] Document how runtime logs capture permission or delivery failures.
- [ ] Add or tighten tests if the allowlist, permission flow, or visibility rules change.

## Verification Plan

- run `npm --prefix web test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`
- manually verify Settings still shows the three modes, permission label, and request button

## Rollout / Fallback / Compatibility Notes

- keep the current mode names unless a migration is written deliberately
- preserve non-throwing behavior if notification delivery fails
- preserve current task-center-first posture even if richer desktop integration is considered later
- if new commands are added to the important-success allowlist, update this doc and tests in the same change

## Related Docs

- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
