# Desktop Presence And Chrome Implementation Tracking

Created: 2026-04-08

Prompt source: [desktop_presence_chrome_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt8/desktop_presence_chrome_direction_FINAL.md)

Scope: Prompt 8 grounded implementation only. This tracker converts the valid Prompt 8 findings into a constrained desktop implementation plan without expanding native surface area, reopening deferred tray/global-shortcut/updater decisions, or creating a second task/control system beside the current shell.

Status: Drafted on 2026-04-08. Objective checkpoint not yet implemented.

Owner posture: Objective-first, desktop-restraint-biased, foreground-workstation-biased.

## Purpose

Turn the grounded Prompt 8 desktop findings into a detailed implementation plan that:

- makes the current desktop presence model explicit
- preserves VectorGrid as a single-window, foreground-first workstation
- keeps [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue) focused on in-window shell actions and native window controls
- keeps [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) as the source of truth for task posture and follow-up actions
- keeps OS notifications bounded, visibility-aware, and subordinate to in-app task state
- keeps [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue) as the diagnostics and desktop-policy explanation surface
- closes the remaining high-confidence desktop-safety gap by adding a narrow active-work close guard

This is not a tray project, not a global-shortcut expansion, not an updater UX pass, and not a reason to broaden Tauri capability scope.

## Functionality Posture

This implementation must be additive and capability-preserving.

- Do not remove the custom titlebar, command-palette entry, settings entry, or current window controls from [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue).
- Do not remove [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue), desktop notifications, window-state restore, or in-app desktop diagnostics.
- Do not add tray behavior, hide-to-tray-on-close behavior, updater controls, relaunch/process UX, or true global shortcuts during the objective pass.
- Do not convert notifications into a second task-history or progress-stream surface.
- Do not add speculative Settings toggles for deferred native features.
- Do not broaden [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue) into an out-of-focus shortcut system.
- Do not expand Tauri capabilities or plugin surface unless a concrete blocker proves that a small desktop lifecycle hook cannot be implemented within the existing baseline.
- Preserve the current single `main` window posture from [desktop_capability_inventory_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/desktop_capability_inventory_2026-03-28.md).

## Progress Ledger

- [ ] Phase 0 complete: baseline desktop posture audit and evidence capture finished
- [ ] Phase 1 complete: desktop presence decision record and user-facing copy landed
- [ ] Phase 2 complete: narrow active-work close safety landed
- [ ] Phase 3 complete: ownership alignment and no-expansion review landed
- [ ] Phase 4 complete: desktop verification and evidence refresh landed
- [ ] Objective checkpoint complete and ready for clean commit
- [ ] Optional interpretive follow-up reviewed
- [ ] Targeted verification complete
- [ ] Final evidence and screenshots attached

## Implementation Outcome Target

At the end of the objective checkpoint:

- the repo should explicitly document that VectorGrid is a foreground-first, single-window desktop workstation
- the frontend should explain that minimize is standard window backgrounding, not tray behavior
- close should still exit by default during ordinary idle use
- close should warn only when meaningful active work would be cancelled, abandoned, or made surprising by immediate exit
- the titlebar, Task Center, notifications, and Settings surfaces should remain in clearly separated ownership lanes
- no tray, updater, relaunch, or global-shortcut surface should be added
- desktop behavior should become easier to reason about without increasing native complexity

## Checkpoint Strategy

### Checkpoint A — Objective Improvements

Land only the grounded, high-confidence changes first:

- baseline audit and evidence capture
- explicit desktop presence decision record
- concise Settings copy describing minimize/close/notification posture
- narrow close safety for meaningful active work
- ownership verification across titlebar, Task Center, notifications, and Settings
- desktop verification and evidence capture

Checkpoint A should be suitable for a clean commit on its own.

### Checkpoint B — Optional Interpretive Follow-Up

Only after Checkpoint A is stable:

- decide whether notification click-through should exist at all
- decide whether any additional user-facing desktop-presence copy belongs outside Settings
- decide whether close-guard trigger breadth should widen beyond the currently meaningful states
- revisit deferred tray/global-shortcut/updater surfaces only if operator evidence actually justifies them

Checkpoint B should be easy to split, stash, or revert without threatening the desktop baseline.

## Objective-First Execution Rule

Execute this work in the following order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Objective verification
7. Clean objective commit checkpoint
8. Optional interpretive follow-up

Do not mix speculative native features into the objective checkpoint.

## Source Basis

- [desktop_presence_chrome_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt8/desktop_presence_chrome_direction_FINAL.md)
- [desktop_productization_plan_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/desktop_productization_plan_2026-03-28.md)
- [desktop_ux_decisions_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/desktop_ux_decisions_2026-03-28.md)
- [desktop_capability_inventory_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/desktop_capability_inventory_2026-03-28.md)
- [desktop_plugin_inventory_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/desktop_plugin_inventory_2026-03-28.md)
- [frontend_functional_coverage_matrix_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_functional_coverage_matrix_2026-03-28.md)
- [frontend_design_system_spec_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_design_system_spec_2026-03-28.md)
- [REPO_MAP.md](C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)
- [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue)
- [desktopProductization.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.ts)
- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [taskStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/taskStore.ts)
- [src-tauri/tauri.conf.json](C:/Rust%20Projects/ta_desktop_demo/src-tauri/tauri.conf.json)

## Execution Guardrails

- Keep the app’s desktop identity centered on one main product window.
- Keep minimize behavior standard and unsurprising.
- Keep close behavior direct by default when nothing meaningful is active.
- Keep any close guard narrow, short, and action-oriented.
- Keep [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) as the authoritative task-state surface.
- Keep OS notifications visibility-aware and high-signal only in [desktopProductization.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.ts).
- Keep [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue) as the desktop-policy and diagnostics explanation surface.
- Keep [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue) document-focused only.
- Avoid capability expansion, plugin addition, or multi-window growth during the objective pass.
- Preserve current desktop quality gates and do not skip desktop smoke verification.

## Explicit Working Decisions

### 1. Desktop presence model

- VectorGrid stays a foreground-first workstation.
- The current product remains one main application window.
- The app does not become a tray-resident or launcher-style utility in this pass.
- Background awareness is allowed through minimize and bounded notifications.
- Background identity is not adopted in this pass.

### 2. Titlebar ownership

- [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue) remains the front-of-window shell surface.
- It continues to own:
  - command-palette entry
  - settings entry
  - current run/stop posture where already present
  - native window controls
- It does not become:
  - a tray substitute
  - a second task-history surface
  - a broader native-control surface for deferred features

### 3. Task Center ownership

- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) remains the source of truth for active/recent task posture.
- Related-view and follow-up routing remain subordinate to the Task Center.
- No new native affordance may create a competing task-status system during the objective pass.

### 4. Notification ownership

- Notifications remain supplemental background signals only.
- Notification routing remains permission-aware and visibility-aware in [desktopProductization.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.ts).
- Notifications do not become:
  - a full progress stream
  - a surrogate task center
  - a broader workflow control surface

### 5. Settings ownership

- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue) remains the environment, diagnostics, and desktop-policy explanation surface.
- The objective pass may add concise explanatory copy here for:
  - foreground-first desktop posture
  - notification posture
  - standard minimize behavior
  - close-exits-default behavior
- Settings must not receive speculative controls for tray, updater, relaunch, or global shortcuts.

### 6. Deferred native surfaces

- Tray/menu remains deferred.
- True global shortcuts outside the focused webview remain deferred.
- Updater/relaunch UX remains deferred.
- No new plugin or capability work for those areas belongs in the objective pass.

### 7. Close and minimize safety model

- Minimize remains a normal window action.
- Close exits the app by default.
- Add a narrow close guard only when a current meaningful workflow would make silent exit surprising or unsafe.
- The close guard trigger set should be grounded in repo-backed live states, starting with:
  - active meaningful long-running tasks from the tracked task surface
  - an active live execution runner
- Do not show the close guard during ordinary idle desktop use.
- Do not add a “run in background” or “close to tray” option.

## Expected Edit Inventory

Primary expected edit targets:

- [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [desktop_productization_plan_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/desktop_productization_plan_2026-03-28.md)
- [desktop_ux_decisions_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/desktop_ux_decisions_2026-03-28.md)

Likely review-only or narrow-touch targets:

- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [desktopProductization.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.ts)
- [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue)
- [src-tauri/tauri.conf.json](C:/Rust%20Projects/ta_desktop_demo/src-tauri/tauri.conf.json)
- [desktop_capability_inventory_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/desktop_capability_inventory_2026-03-28.md)
- [desktop_plugin_inventory_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/desktop_plugin_inventory_2026-03-28.md)

Potential narrow implementation helper if the close guard must cover non-titlebar close paths:

- a small desktop lifecycle hook in the existing frontend app surface

That helper, if needed, should stay narrow and should not become a new desktop framework layer.

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Goal: record the current desktop behavior and lock the grounded ownership boundaries before any implementation changes.

Checklist:

- [ ] Record the current titlebar window-control behavior:
  - minimize path
  - close path
  - absence of a close guard today
- [ ] Record the current desktop presence baseline:
  - one main product window
  - frameless custom titlebar
  - no tray behavior
  - no updater UI
  - no true global shortcuts
- [ ] Confirm current notification routing behavior in [desktopProductization.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.ts):
  - permission-aware
  - visibility-aware
  - high-signal only
- [ ] Confirm [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) remains the authoritative in-app task surface
- [ ] Confirm [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue) already owns desktop diagnostics and notification policy explanation
- [ ] Confirm [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue) remains document-focused only
- [ ] Confirm [src-tauri/tauri.conf.json](C:/Rust%20Projects/ta_desktop_demo/src-tauri/tauri.conf.json) still defines only the main window and does not add tray/updater posture
- [ ] Capture before-state evidence for:
  - titlebar controls
  - desktop diagnostics section
  - notifications settings section
  - any close behavior currently visible in desktop smoke or manual QA

Acceptance criteria:

- [ ] The current desktop baseline is explicitly documented before code changes land
- [ ] The repo-backed ownership split between titlebar, Task Center, notifications, and Settings is written down
- [ ] There is a stable before-state reference for titlebar and Settings desktop surfaces

## Phase 1 — Decision Record And Presence Copy

Goal: make the grounded desktop presence model explicit in both project docs and concise user-facing diagnostics copy.

Primary files:

- [desktop_productization_plan_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/desktop_productization_plan_2026-03-28.md)
- [desktop_ux_decisions_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/desktop_ux_decisions_2026-03-28.md)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)

Checklist:

- [ ] Update the desktop productization plan to record the explicit foreground-first presence posture
- [ ] Update the desktop UX decisions doc to record:
  - standard minimize behavior
  - close exits by default
  - tray/global shortcuts/updater still deferred
- [ ] Add concise Settings copy explaining:
  - the app is foreground-first
  - Task Center is the source of truth
  - notifications are supplemental
  - minimize does not imply tray behavior
  - close exits the app by default
- [ ] Keep the new copy informational rather than turning Settings into a new policy-control surface
- [ ] Confirm the copy does not promise functionality that does not exist
- [ ] Confirm the copy does not add speculative toggles for deferred native features

Acceptance criteria:

- [ ] The desktop docs explicitly reflect the adopted presence posture
- [ ] Settings explains the desktop posture without adding new feature controls
- [ ] The in-app wording matches current behavior or clearly scoped near-term objective behavior

## Phase 2 — Narrow Active-Work Close Safety

Goal: close the main remaining desktop-safety gap without changing the app’s foreground-first identity.

Primary files:

- [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue)
- [taskStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/taskStore.ts)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)

Potential narrow-touch files if needed:

- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- a small desktop lifecycle hook in the frontend app surface

Checklist:

- [ ] Define the minimal close-guard trigger set using current repo-backed live state
- [ ] Start with the highest-confidence trigger conditions:
  - active meaningful long-running tracked work
  - active live execution runner
- [ ] Keep idle close unguarded
- [ ] Implement the close guard for the titlebar close control
- [ ] If required to cover non-titlebar close requests, add a narrow desktop close-request hook rather than a broad desktop abstraction
- [ ] Reuse an existing confirmation pattern if it fits the desktop close decision cleanly
- [ ] Ensure close-guard copy is brief and explicit about the consequence of exiting now
- [ ] Provide a clear primary exit action and a clear cancel/stay action
- [ ] Keep minimize behavior unchanged
- [ ] Do not add any “keep running in background” or “close to tray” option
- [ ] Do not broaden the trigger set to minor or routine states just to look safer

Acceptance criteria:

- [ ] Ordinary close remains direct and unsurprising
- [ ] Meaningful active-work close is no longer silent
- [ ] The close guard does not appear during idle routine use
- [ ] The implementation does not add tray semantics, multi-window semantics, or broader native complexity

## Phase 3 — Ownership Alignment And No-Expansion Review

Goal: confirm that the desktop implementation still respects the current ownership boundaries and does not smuggle in deferred native scope.

Primary files:

- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue)
- [desktopProductization.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.ts)
- [src-tauri/tauri.conf.json](C:/Rust%20Projects/ta_desktop_demo/src-tauri/tauri.conf.json)

Checklist:

- [ ] Confirm the Task Center remains the authoritative task surface after the close-safety work
- [ ] Confirm notifications remain bounded complements and are not turned into a richer control system
- [ ] Confirm Settings remains the diagnostics/explanation surface rather than a parking lot for desktop toggles
- [ ] Confirm [GlobalShortcutBridge.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue) remains focused-window only
- [ ] Confirm no tray/menu surface was added
- [ ] Confirm no updater/relaunch UI was added
- [ ] Confirm no global-shortcut plugin or capability path was added
- [ ] Confirm [src-tauri/tauri.conf.json](C:/Rust%20Projects/ta_desktop_demo/src-tauri/tauri.conf.json) did not gain speculative native surface additions

Acceptance criteria:

- [ ] Ownership remains clear between titlebar, Task Center, notifications, and Settings
- [ ] No deferred native feature leaked into the objective checkpoint
- [ ] Desktop capability scope remains stable unless a concrete blocker required a narrowly justified change

## Phase 4 — Verification And Evidence Refresh

Goal: verify the objective desktop pass across frontend, desktop, and accessibility surfaces.

Checklist:

- [ ] Run frontend verification:
  - `npm --prefix web run lint`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `npm --prefix web run parity`
  - `npm --prefix web run a11y:smoke`
  - `npm --prefix web run baseline:ui`
  - `npm --prefix web run check:bundle-budget`
- [ ] Run desktop verification:
  - `npm --prefix web run test:desktop:smoke`
  - `cargo check -p ta_desktop_demo_app`
- [ ] Verify ordinary close behavior when idle
- [ ] Verify guarded close behavior when meaningful work is active
- [ ] Verify minimize behavior during idle and during long-running work
- [ ] Verify the Task Center remains usable and authoritative after close/minimize policy updates
- [ ] Verify desktop notification behavior remains background-only and visibility-aware
- [ ] Capture refreshed evidence under `scripts/baselines/ui_capture` for:
  - titlebar desktop controls
  - Settings desktop-presence copy
  - guarded close prompt
  - desktop idle close/minimize state if reproducible in smoke or manual QA

Acceptance criteria:

- [ ] All required verification surfaces pass or any known failures are explicitly documented as unrelated
- [ ] Desktop behavior matches the documented foreground-first posture
- [ ] Screenshot evidence exists for the changed surfaces

## Objective Verification Checklist

- [ ] Titlebar still exposes the expected shell and window-control actions
- [ ] Close exits by default when idle
- [ ] Meaningful active-work close is guarded
- [ ] Minimize remains standard and unsurprising
- [ ] Task Center remains the source of truth for task posture
- [ ] Notifications remain bounded complements
- [ ] Settings explains the desktop presence model without growing speculative controls
- [ ] No tray/global-shortcut/updater surface was added

## Targeted Test Additions Checklist

- [ ] Add or update titlebar tests for ordinary close behavior
- [ ] Add or update tests for guarded close behavior when a meaningful active state is present
- [ ] Add or update tests for Settings desktop-presence copy if that copy is added
- [ ] Add or update any desktop smoke path necessary to prove idle-close versus guarded-close behavior

## Manual Review Checklist

- [ ] Review idle close from the titlebar button
- [ ] Review minimize from the titlebar button
- [ ] Review guarded close while a tracked long-running task is active
- [ ] Review guarded close while the live runner is active
- [ ] Review Settings desktop diagnostics and presence copy
- [ ] Review notification behavior while the window is visible versus backgrounded
- [ ] Review that no new desktop clutter or speculative toggles appeared

## Guardrail Review Checklist

- [ ] No tray/menu implementation landed
- [ ] No hide-to-tray-on-close behavior landed
- [ ] No updater/relaunch UI landed
- [ ] No true global shortcuts landed
- [ ] No second task-status system appeared beside the Task Center
- [ ] No capability expansion landed without narrow documented justification
- [ ] No multi-window product posture was introduced

## Defers And Rejects

- [ ] Defer tray/menu implementation until real operator evidence justifies it
- [ ] Defer true global shortcuts outside the focused webview
- [ ] Defer updater/relaunch UX until signed release topology exists
- [ ] Reject hide-to-tray-on-close for the current product phase
- [ ] Reject speculative Settings toggles for deferred native features
- [ ] Reject notifications as a parallel task-management system

## Optional Interpretive Follow-Up Checklist

- [ ] Evaluate notification click-through only if it can deterministically focus the app and reuse existing Task Center related-view ownership
- [ ] Revisit whether any additional desktop-presence copy belongs outside Settings
- [ ] Revisit the exact breadth of close-guard trigger states if future workflows make exit semantics less obvious
- [ ] Revisit tray/menu only if operators routinely keep the app minimized during long-running work and need fast show/hide/quit status paths
- [ ] Revisit true global shortcuts only if there is repeated out-of-focus demand for a tightly-scoped bring-forward or emergency control path
- [ ] Revisit updater/relaunch only after signing, endpoints, and release automation are real

## Final Pass/Fail Criteria

Pass if:

- the desktop presence model is explicit
- the remaining close-safety gap is addressed narrowly
- titlebar, Task Center, notifications, and Settings keep clear ownership boundaries
- no deferred native surface leaks into the objective checkpoint
- desktop and frontend verification remain green

Fail if:

- close/minimize semantics become less predictable
- the close guard becomes routine annoyance instead of a narrow safety measure
- Settings grows speculative feature toggles
- notifications or the titlebar begin duplicating Task Center ownership
- the implementation drifts into tray/global-shortcut/updater scope
