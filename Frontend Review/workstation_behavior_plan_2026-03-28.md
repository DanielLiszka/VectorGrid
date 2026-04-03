# Workstation Behavior Plan

Date: 2026-03-28

Scope: later-phase UX/product plan for the most valid operator-workstation ideas that emerged after the main frontend remediation pass.

Implementation status: complete for the frontend-only workstation scope described here. The remaining desktop-native work still lives in `Unfinished/desktop_productization_plan_2026-03-28.md`.

This plan is intentionally separate from:

- `frontend_audit_2026-03-27.md`, which is now primarily a completed remediation ledger
- `Unfinished/frontend_followup_evaluation_2026-03-28.md`, which records the completed frontend follow-up work and package decisions
- `Unfinished/desktop_productization_plan_2026-03-28.md`, which owns Tauri/runtime-native desktop work such as window state, notifications, tray/menu, updater, and CSP/capabilities

This plan is for higher-level workstation behavior improvements inside the current frontend product surface.

## Why This Exists

The frontend is no longer mainly in "fix obvious debt" mode.

The next meaningful gains are not more generic cleanup. They are operator-facing behaviors that make the app feel like a research workstation:

- named perspectives instead of one remembered shell state
- pinned compare flows instead of one selected row at a time
- better explanation/provenance surfaces instead of plain metric dumps
- trust cues when results are stale or context has drifted
- clearer lifecycle orchestration for long-running work

These ideas are valid for this repo because there are already real foundations in place:

- shell/workflow memory in [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts)
- compare/baseline/summary state in [analysisStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\analysisStore.ts)
- selected-result summaries in [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- shared lifecycle/status presentation in [statusPresentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\statusPresentation.ts)
- task orchestration state in [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)

## Non-Goals

This plan does not include:

- Tauri-native titlebar/tray/menu/updater/plugin work
- backend strategy or command-surface expansion
- another broad visual redesign
- replacing `lightweight-charts`
- decorative interactivity that does not improve decision-making

## Product Principles

1. Preserve backend-backed capability coverage.
   Regrouping controls is fine. Silently removing user reachability is not.

2. Favor decision support over decoration.
   The UI should feel deeper because it is more legible and more trustworthy, not because it has more chrome.

3. Treat state memory as product value.
   Users should be able to return to a working posture, not rebuild it every session.

4. Make review work faster than authoring work is noisy.
   Optimization tools generate more candidates and artifacts than a user can reason through one at a time.

5. Keep additions incremental and reversible.
   Prefer extending the current shell/store model over inventing parallel frameworks.

## Success Criteria

This plan is successful when:

- users can save and restore named workstation perspectives
- results can be pinned and compared without losing the current selection workflow
- selected-result surfaces explain why a candidate deserves attention
- stale/provenance cues make it obvious when context should not be trusted blindly
- task progress feels staged and navigable rather than merely "currently running"
- density is a deliberate user-level choice rather than incidental CSS compaction
- the added behavior does not regress current keyboard access, functional coverage, or baseline performance

## Implementation Order

1. Saved perspectives
2. Compare/pin workflow
3. Decision-support summaries and provenance
4. Stale/dirty cues
5. Task lifecycle orchestration
6. Density system
7. Bundle-discipline follow-through

## Phase 1: Saved Perspectives

Goal: turn the existing remembered shell state into user-managed named perspectives.

Primary files:

- [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Deliverables:

- define a perspective model that can safely store:
  - workspace
  - workflow and workflow step
  - controls/results/aggregate tab selection
  - split size
  - density mode
  - selected high-level review context if safe to restore
- support named built-in starter perspectives such as:
  - Optimize
  - Validate
  - Top Results Triage
  - Execution Review
  - Run Library
- support user-created and user-renamed perspectives
- provide clear restore, overwrite, duplicate, and delete actions

Acceptance:

- restoring a perspective returns the user to a recognizably stable workstation posture
- a broken or outdated perspective fails safely instead of corrupting shell state
- perspective restore does not remove or hide backend-backed capabilities

## Phase 2: Pin / Compare Workflow

Goal: move beyond a single selected result into a real candidate comparison flow.

Primary files:

- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [analysisStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\analysisStore.ts)
- [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
- [workflowModel.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\core\workflowModel.ts)

Deliverables:

- allow pinning a small number of candidates from Top Results
- add a compact compare tray or compare strip that does not dominate the workspace
- support compare actions from:
  - Top Results rows
  - relevant saved-run/review surfaces where the same comparison semantics make sense
- provide quick replace/remove actions for pinned candidates
- define keyboard behavior for pinning and compare focus movement

Acceptance:

- users can compare multiple candidates without losing the current primary row-selection flow
- compare state is visually obvious but does not crowd the main analysis surfaces
- compare behavior is consistent with existing aggregate/meta compare concepts rather than competing with them

## Phase 3: Decision-Support Summaries And Provenance

Goal: make the selected candidate summary read like an analyst note instead of a metric dump.

Primary files:

- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [analysisStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\analysisStore.ts)
- relevant downstream review surfaces under [web/src/components](C:\Rust Projects\ta_desktop_demo\web\src\components)

Deliverables:

- upgrade selected-result summaries to explain why a result is noteworthy
- support compact provenance/confidence ribbons such as:
  - out-of-sample stable
  - low sample
  - slippage sensitive
  - regime concentrated
  - validation stale
  - dataset age/source
- keep summary language concise and operator-oriented
- ensure summary/provenance cues are derived from real state, not invented heuristics

Acceptance:

- selected-result summaries communicate rank/context/reason, not just raw values
- provenance chips are compact and meaningful
- the same candidate is not described differently across nearby review surfaces without reason

## Phase 4: Stale / Dirty Cues

Goal: tell the user when the current review state is no longer trustworthy at face value.

Primary files:

- [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts)
- [validationStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\validationStore.ts)
- [historyStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\historyStore.ts)
- [analysisStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\analysisStore.ts)
- [executionStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\executionStore.ts)

Deliverables:

- define the most important stale/dirty states, for example:
  - strategy/config changed since result generation
  - validation older than current parameters
  - dataset/cache changed since artifact creation
  - execution config differs from the reviewed profile
  - current selected row invalidated by new filter/sort state
- surface those states as compact banners/chips/summary warnings rather than modal interruptions
- document when stale state is informational versus blocking

Acceptance:

- stale cues improve trust without flooding the shell
- users can tell the difference between "informational drift" and "unsafe to proceed"
- stale markers are driven by deterministic state transitions where practical

Current stale-state taxonomy:

- informational drift:
  - prior shortlist selection no longer exists after filter/sort refresh
  - pinned candidate is hidden by the current shortlist filters
  - pinned candidate is unavailable in the current shortlist/run context
- blocking/unsafe:
  - no new blocking stale markers were added in this frontend-only phase
  - execution safety remains owned by the existing execution/live-risk surfaces rather than generic review banners
- presentation policy:
  - stale cues stay inline as compact advisory chips/banners
  - no modal interruptions were added for review-state drift

## Phase 5: Task Lifecycle Orchestration

Goal: make long-running work feel orchestrated rather than merely active.

Primary files:

- [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [statusPresentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\statusPresentation.ts)

Deliverables:

- group or summarize tasks by meaningful lifecycle posture:
  - queued
  - running
  - completed
  - failed/cancelled
- expose clearer stage labels where the underlying task model can support them
- add direct "open related result" or equivalent completion actions where appropriate
- define notification-routing policy for later desktop work:
  - toast only
  - OS only
  - both

Acceptance:

- the operator can understand what is waiting, what is active, and what just finished without scanning raw event history
- task-completion actions reduce navigation friction instead of creating more branching UI

Current policy note:

- until the desktop productization track adds OS notifications, task lifecycle routing stays in-app first:
  - task center + footer + diagnostics remain the primary surfaces
  - high-signal completion/failure states may later be mirrored to the OS layer
  - user-facing notification preferences should remain a later settings/productization concern, not a duplicated task-store concern

## Phase 6: Density System

Goal: make density a deliberate user preference instead of incidental compaction.

Primary files:

- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- table/list-heavy panels under [web/src/components](C:\Rust Projects\ta_desktop_demo\web\src\components)

Deliverables:

- define user-facing density modes:
  - Comfortable
  - Standard
  - Dense
- apply them consistently to:
  - row height
  - chip density
  - metadata spacing
  - panel spacing
  - summary strip compactness
- store density with perspectives and global settings

Acceptance:

- density changes feel intentional across the shell instead of per-component hacks
- Dense mode improves throughput without breaking readability or focus treatment

## Phase 7: Bundle-Discipline Follow-Through

Goal: preserve the improved frontend baseline as richer workstation behaviors are added.

Primary files:

- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- [vite.config.ts](C:\Rust Projects\ta_desktop_demo\web\vite.config.ts)
- heavy panels and optional analysis surfaces under [web/src/components](C:\Rust Projects\ta_desktop_demo\web\src\components)

Deliverables:

- define a bundle budget policy for the main entry and major lazy chunks
- keep heavy analytical surfaces lazy-loaded
- define an eviction/unmount policy for any future heavy hidden panels if retention becomes a memory problem

Acceptance:

- new workstation behavior does not quietly re-bloat initial load
- performance regression checks remain reviewable instead of anecdotal

Current bundle policy:

- enforce the budget with `npm run check:bundle-budget`
- thresholds:
  - entry chunk: `<= 600 kB`
  - largest lazy chunk: `<= 550 kB`
  - total JavaScript output: `<= 2800 kB`
- current posture:
  - non-blocking overlays and heavy review surfaces stay lazy-loaded
  - hidden heavy-panel eviction is explicitly deferred for now because active-only mounting plus the current budget checks are sufficient

## Later / Optional Ideas

These ideas are reasonable, but should remain secondary until the phases above are complete:

- explicit authoring/review/operate mode model
- analyst notes or annotations on saved runs
- isolated component-state sandboxing through Storybook or Histoire
- split-pane layout tooling if the perspective system clearly needs it

## Checklist

### Saved Perspectives

- [x] Define a safe perspective-state model.
- [x] Add built-in starter perspectives.
- [x] Support create, rename, duplicate, overwrite, and delete.
- [x] Persist and restore split size and density with perspectives.
- [x] Verify restore behavior survives stale/missing tabs safely.

### Pin / Compare Workflow

- [x] Define the maximum pinned-candidate model.
- [x] Add pin-from-row actions in Top Results.
- [x] Add a compact compare tray/strip.
- [x] Add quick replace/remove actions.
- [x] Add keyboard support and regression coverage.

### Decision Support / Provenance

- [x] Upgrade selected-result summaries to explain why the result matters.
- [x] Add compact provenance/confidence ribbons.
- [x] Keep summary wording deterministic and consistent across surfaces.
- [x] Add regression coverage for summary/provenance rendering.

### Stale / Dirty Cues

- [x] Define the highest-value stale states.
- [x] Add informational stale markers.
- [x] Add blocking/unsafe markers only where truly warranted.
- [x] Document the stale-state taxonomy for future contributors.

### Task Lifecycle Orchestration

- [x] Improve grouped task posture in the Task Center.
- [x] Add clearer stage/related-result actions where supported.
- [x] Define notification-routing policy for later desktop integration.
- [x] Add regression coverage for the new task-grouping logic.

### Density System

- [x] Define Comfortable / Standard / Dense.
- [x] Apply density consistently to major table/list/panel surfaces.
- [x] Persist density in settings.
- [x] Persist density in saved perspectives.

### Bundle Discipline

- [x] Define a bundle budget policy.
- [x] Keep future heavy review surfaces lazy.
- [x] Decide whether hidden heavy-panel eviction is needed later.
