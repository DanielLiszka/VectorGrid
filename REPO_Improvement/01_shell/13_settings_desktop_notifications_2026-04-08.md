# Settings Desktop Notifications Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Notifications plugin](https://v2.tauri.app/plugin/notification/)
  - relevant because the current implementation follows Tauri’s permission-check, permission-request, then send flow
- [Tauri State Management](https://v2.tauri.app/develop/state-management/)
  - relevant because permission/readiness state is store-backed and should remain centralized
- [Pinia state management guidance](https://pinia.vuejs.org/core-concepts/)
  - relevant because notification preference and runtime state should remain separate but synchronized surfaces
- [MDN ARIA status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because permission and readiness messages behave like status output in Settings and runtime diagnostics
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

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
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the three notification modes as a stable user-facing contract.
- [x] Document the current important-success command allowlist.
- [x] Document visibility suppression and the difference between task-center truth and OS notification supplementation.
- [x] Document permission request and permission-label semantics.
- [x] Document how runtime logs capture permission or delivery failures.
- [x] Add or tighten tests if the allowlist, permission flow, or visibility rules change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- manually verify Settings still shows the three modes, permission label, and request button
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the current mode names unless a migration is written deliberately
- preserve non-throwing behavior if notification delivery fails
- preserve current task-center-first posture even if richer desktop integration is considered later
- if new commands are added to the important-success allowlist, update this doc and tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
