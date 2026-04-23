# Desktop Presence And Chrome Direction

Date: 2026-04-07

## 1. Executive summary

VectorGrid should keep the current desktop productization baseline and **not** expand native surface area just because the app is built with Tauri.

The repo already adopted the highest-value desktop moves for the current product shape:

- custom frameless titlebar and explicit main-window permissions
- window-state restore
- desktop log bridge and in-app diagnostics
- bounded OS notifications for high-signal task outcomes
- a single-main-window workstation model

The strongest remaining decision is not tray, global shortcuts, or updater UI. The strongest remaining decision is to make the **desktop presence model** explicit.

The recommended direction is:

- keep VectorGrid as a **single-window, foreground-first workstation**
- keep the **Task Center** as the operational source of truth
- keep **OS notifications** as background-only, high-signal complements
- keep **titlebar actions** focused on in-window command and shell control
- keep **tray/menu**, **true global shortcuts**, and **updater/relaunch UX** deferred
- adopt explicit **minimize** and **close** semantics now, with a narrow close guard for active work where silent exit would be surprising or unsafe

This direction strengthens workstation presence without creating a second native control system.

## 2. Final desktop-presence posture

### Product stance

VectorGrid is a **foreground-first research workstation**, not an ambient background utility and not a launcher-style desktop tool.

Its desktop model should remain:

- one main product window
- one primary in-app command system
- one primary task-status system
- bounded OS integration only where it improves real operator workflows

### Window-presence rules

#### Minimize

Minimize should behave like a normal desktop window state:

- the app remains alive while minimized
- long-running work may continue if the current runtime/task model already supports that behavior
- the operator can rely on Task Center state and optional OS notifications when the window is no longer visible

Minimize does **not** imply tray adoption or a special background-resident posture.

#### Close

Close should mean **exit the app by default**.

The app should not silently change “close” into “hide to tray” or “keep running invisibly” in the current product phase.

A narrow close guard is still justified where silent exit would be surprising or unsafe. The close guard should appear only when one of these conditions is true:

- an optimization, validation, export, or other meaningful long-running task is active and exit would cancel or abandon it
- the live runner is active
- another future workflow is introduced where silent exit would materially change operator expectations

The close guard should be brief, explicit, and action-oriented. It should not become a general annoyance on ordinary close behavior.

### Background-presence rules

VectorGrid should support **background awareness**, not **background identity**.

That means:

- notifications may surface important outcomes while the window is not visible
- the app may be minimized during long work
- the product should not add a tray-resident or always-on background persona unless operator evidence later shows that this is routine and valuable

## 3. Adopt now / defer / reject decisions

### Tray / menu

**Decision: defer**

Tray or menu support is not justified by the current workflow evidence.

The current shell already provides:

- titlebar actions
- command palette access
- Task Center state and actions
- settings diagnostics
- bounded notification routing

A tray should only be reconsidered if users regularly keep VectorGrid minimized during long-running workflows and repeatedly need quick show/hide, quick status, or quick quit behavior while the main shell is out of view.

**Reject for now:**

- hide-to-tray on close
- tray as a second navigation hub
- tray actions that duplicate titlebar or Task Center ownership

### Global shortcuts outside the focused webview

**Decision: defer**

The app already has strong in-window keyboard posture through the shell and focused shortcut bridge. True system-wide shortcuts should remain deferred until there is repeated evidence that operators need out-of-focus control.

Valid future cases would be very narrow, for example:

- bring the app forward and open the command palette
- a tightly-scoped emergency control for a future long-lived background workflow

Until then, do not add a global shortcut plugin or system-wide shortcut surface.

**Reject for now:**

- broad global hotkey coverage
- run/stop/export shortcuts that work everywhere on the system
- a second keyboard model that competes with the in-window shell

### Updater / relaunch posture

**Decision: defer**

Updater work should remain blocked behind real release topology:

- signing keys
- updater endpoints
- release automation
- explicit install/relaunch behavior

Do not expose updater UI before that infrastructure exists. In the current phase it would create visible product surface for an incomplete operational path.

**Reject for now:**

- settings-level updater controls
- “check for updates” UX without signed release infrastructure
- relaunch/process UX without a real update install flow

### Background / minimized presence model

**Decision: adopt now**

This is the one unresolved desktop-presence question that should be decided immediately.

Adopt the following policy:

- minimize keeps the app alive in the standard way
- close exits by default
- notifications remain supplemental and high-signal only
- the app does not remain hidden in the background after close
- active-work close guards are allowed only for genuinely meaningful workflows

This produces a clearer, more intentional desktop identity without expanding native surface area.

## 4. Relationship between titlebar, task center, notifications, and any new native affordances

### Titlebar

The titlebar is the **front-of-window shell surface**.

It should own:

- app/workflow context
- command palette entry
- settings entry
- primary run/stop posture where already present
- native window controls

It should not become:

- a dense status dashboard
- a tray substitute
- a second task-history surface

### Task Center

The Task Center is the **source of truth for task state**.

It should remain the authoritative location for:

- queued/running/cancelling/completed/attention grouping
- task lifecycle review
- cancellation actions
- related-view or follow-up actions
- recent operator-facing history

Any future native affordance must remain subordinate to the Task Center rather than creating a competing task system.

### Notifications

Notifications are **supplemental background signals**.

They should stay:

- high-signal only
- permission-aware
- background-only when the window is not visible
- subordinate to in-app status and history

They should not attempt to mirror every progress change or become the main task UX.

### Settings diagnostics

Settings remains the **environment and runtime explanation surface**.

It should continue to explain:

- notification permission and notification mode
- desktop log state
- window-state persistence state
- runtime posture and diagnostics
- the current desktop presence model

Settings should not become a parking lot for speculative tray, updater, or global-shortcut toggles before those features are real.

### New native affordances

No new native affordance should be added now.

The first future candidate, if later justified, is **notification click-through to focus the app and open a deterministic related view**. That should only ship if it can reuse existing task-related ownership rather than inventing new routing rules.

## 5. Config/code touch points

### Primary implementation files

- `web/src/components/DesktopTitlebar.vue`
  - keep the shell-facing action set concise
  - add the narrow close guard if active-work confirmation is implemented

- `web/src/components/TaskCenterOverlay.vue`
  - remain the source of truth for task posture and follow-up actions
  - preserve related-view ownership for any future notification click-through behavior

- `web/src/components/SettingsWorkspacePanel.vue`
  - add explicit copy describing the desktop presence model
  - keep diagnostics, notification policy, and runtime explanation here
  - do not add speculative controls for deferred native features

- `web/src/components/GlobalShortcutBridge.vue`
  - preserve focused-window keyboard behavior only
  - do not promote it to a true global-shortcut surface yet

- `web/src/services/desktopProductization.ts`
  - keep visibility-aware notification routing as the current OS bridge
  - later, optionally extend only if deterministic notification follow-up is adopted

### Config and policy docs

- `src-tauri/tauri.conf.json`
  - no tray or updater additions in the current phase

- `desktop_productization_plan_2026-03-28.md`
  - update the open checklist and decision record to reflect the explicit presence posture

- `desktop_ux_decisions_2026-03-28.md`
  - add the foreground-first presence decision and close/minimize semantics

No capability expansion is required for the recommended near-term work.

## 6. Implementation order

### Phase 1: decision record and copy

1. Update the desktop decision docs to record the final presence posture.
2. Add concise user-facing explanation in Settings:
   - the app is foreground-first
   - notifications are supplemental
   - close exits by default
   - minimize is standard window backgrounding, not tray behavior

### Phase 2: narrow close safety

3. Add close interception in `DesktopTitlebar.vue` only for high-confidence active-work cases.
4. Keep the confirmation narrow and explicit.
5. Do not add a general “run in background” or “minimize to tray” option.

### Phase 3: optional later refinement

6. Revisit notification click-through only if it can deterministically focus the app and open the same related view already owned by the Task Center.

### Explicitly not in scope now

- tray/menu implementation
- true global shortcut adoption
- updater UI or relaunch flow
- background-resident mode

## 7. Validation plan

### Required verification

For any implementation slice in this area, run:

- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `cargo check -p ta_desktop_demo_app` when Tauri/runtime wiring changes

### Manual desktop checks

Verify:

- minimize behavior while long-running work is active
- close behavior during:
  - idle state
  - active optimization/validation/export
  - active live-runner state if applicable
- notifications while the window is visible versus backgrounded
- no regression in access to:
  - titlebar actions
  - Task Center task control
  - settings diagnostics
  - execution/runtime utilities

### Regression guardrails

The recommended slice fails if it causes any of the following:

- close/minimize semantics become less predictable
- duplicate task ownership appears between titlebar, Task Center, notifications, or new native surfaces
- settings becomes cluttered with speculative feature toggles
- operators lose a clear in-app path to current desktop/runtime controls

## 8. Evidence that would justify revisiting deferred items

### Tray / menu reconsideration

Revisit tray only if multiple conditions become true:

- users routinely minimize VectorGrid during long-running work
- users repeatedly need quick show/hide or quick quit while the main window is out of focus
- the product gains a meaningful status-at-a-glance need while the main shell is not foregrounded

### Global shortcut reconsideration

Revisit true global shortcuts only if there is repeated demand for:

- global “bring forward and open palette” behavior
- one or two narrowly-defined out-of-focus controls
- a background-capable workflow that genuinely benefits from system-wide keyboard invocation

### Updater / relaunch reconsideration

Revisit updater only after release infrastructure is real:

- signing keys exist and are operationally owned
- updater endpoints exist
- release automation exists
- the app is being distributed often enough that manual update posture is now a real support cost

## Final recommendation

Do **not** add more native surface now.

Instead, complete the desktop productization pass by making the current workstation presence model explicit:

- one main window
- foreground-first posture
- standard minimize behavior
- close exits by default
- narrow close guard for meaningful active work
- Task Center as source of truth
- notifications as bounded complements
- tray, global shortcuts, and updater still deferred until operator evidence justifies them

This is the strongest fit for the repo because it makes the desktop shell feel more intentional without creating a second, partially-justified native control system.
