# Shell Hierarchy And Workstation Implementation Tracking

Date: 2026-04-03

Scope: implementation tracker for the Prompt 1 shell/workstation direction in [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md).

Status: not started

Owner posture: frontend UI/UX shell refinement only. Do not broaden into backend contract changes, sidebar redesign, or unrelated productization work.

## Purpose

This document turns the adopted Prompt 1 shell direction into an execution tracker with:

- explicit phase ordering
- concrete file ownership
- implementation checklist items
- product-owner decisions that must be resolved
- verification gates and evidence requirements

This tracker is for executing the shell recommendation:

- keep the tab-first workstation shell
- integrate the task center more clearly into the main shell hierarchy
- quiet the titlebar
- tighten the context strip
- keep the command palette secondary but strong
- keep perspectives out of permanent shell chrome for now
- keep the dev quick switch dev-only

## Progress Ledger

- [ ] Phase 0 complete: baseline locked and evidence captured
- [ ] Phase 1 complete: shell hierarchy clarified in `VueWorkspaceShell.vue`
- [ ] Phase 2 complete: titlebar quieted in `DesktopTitlebar.vue`
- [ ] Phase 3 complete: task vocabulary aligned across shell surfaces
- [ ] Phase 4 complete: palette/perspective posture confirmed
- [ ] Phase 5 complete: Settings continuity documented for later work
- [ ] Verification complete
- [ ] Final evidence and screenshots attached

## Source Basis

Primary source:

- [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md)

Supporting repo policy/context:

- [frontend_design_system_spec_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\frontend_design_system_spec_2026-03-28.md)
- [frontend_target_comps_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\frontend_target_comps_2026-03-28.md)
- [frontend_functional_coverage_matrix_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\frontend_functional_coverage_matrix_2026-03-28.md)
- [workstation_behavior_plan_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\workstation_behavior_plan_2026-03-28.md)
- [existing_stack_utilization_plan_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\existing_stack_utilization_plan_2026-03-28.md)

Primary implementation files:

- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)
- [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)
- [statusPresentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\statusPresentation.ts)
- [workflowModel.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\core\workflowModel.ts)

## Execution Guardrails

- Keep workspace tabs and workflow-step tabs as the primary visible orientation system.
- Do not introduce a generic left sidebar or dashboard-shell pivot.
- Do not make the command palette the primary navigation model.
- Do not promote `WorkflowQuickSwitch.vue` into production shell navigation.
- Do not add permanent shell-level perspective chrome in this pass.
- Preserve at least one clear frontend path for every backend-backed capability.
- Preserve current keyboard-safe tab semantics, focus management, and overlay behavior.
- Preserve active-only/lazy posture for heavyweight surfaces unless there is a strong reason to change it.
- Treat the task center issue as a shell-integration problem, not as proof that the product lacks a visible task-center surface entirely.

## Explicit Working Decisions

These are the working decisions for implementation in this pass. Treat them as locked defaults unless the product owner explicitly overrides them.

### 1. Task-center launcher placement

Decision:

- add a visible shell-level task-center launcher inside the shell header area owned by `VueWorkspaceShell.vue`
- place it on the same visual band as the workspace navigation, aligned to the right edge of the workspace row when horizontal space allows
- if the row wraps on narrower widths, the launcher remains directly below the workspace row and directly above the workflow-step row

Implementation meaning:

- do not create a new separate shell band only for task controls
- do not rely solely on the floating `TaskCenterOverlay.vue` toggle or Settings entry for shell visibility

### 2. Shell header reduction

Decision:

- keep the `VectorGrid` heading in `VueWorkspaceShell.vue`
- remove the persistent `Guided default workflow with power tools on demand.` caption from the default shell header
- keep the current `shellSubtitle` as the only persistent secondary line in the shell header

Implementation meaning:

- the shell header should stop competing with the titlebar for descriptive copy
- if later testing shows the subtitle is still too much, the subtitle can be reduced in styling, but not removed in this pass

### 3. Context-strip default fields

Decision:

- visible by default:
  - `Dataset`
  - `Range`
  - `Strategy`
  - `Backend`
  - `Status`
- moved into secondary disclosure/overflow by default:
  - `Rows`
  - `Loaded ID`

Implementation meaning:

- the default context strip should show five items, not seven
- the overflow path must remain keyboard reachable
- overflow content should not create another full-width telemetry shelf

### 4. Context-strip interaction model

Decision:

- context-strip items remain read-only in this pass
- do not make context items clickable unless a later pass explicitly standardizes canonical jump behavior

Implementation meaning:

- this pass is about hierarchy and noise reduction, not about adding more shell actions

### 5. Titlebar end state

Decision:

- keep:
  - brand/logo
  - workspace/workflow summary
  - subtitle
  - Search
  - Settings
  - one contextual Run/Stop action
  - window controls
- remove the current three-chip burden
- replace it with at most one compact task-center launcher/status chip
- remove dataset detail from the titlebar
- remove the wide status chip from the titlebar

Implementation meaning:

- the titlebar is desktop chrome plus one compact operational affordance
- the titlebar is not the canonical place to inspect detailed task/runtime state

### 6. Task-language hierarchy

Decision:

- detailed task review belongs to `TaskCenterOverlay.vue`
- shell-level task summaries may appear in:
  - the shell launcher
  - one compact titlebar chip
  - the workflow footer
- shell-level summaries must be short and must not all repeat the same long-form status text

Implementation meaning:

- task center: detailed destination
- titlebar: compact signal only
- footer: workflow-scoped status only
- context strip: short status only, if still useful after refinement

### 7. Palette and perspective posture

Decision:

- `CommandPaletteOverlay.vue` remains the global go/do accelerator
- no shell-first or palette-first navigation pivot
- no permanent perspective switcher in shell chrome for this pass
- perspectives remain in Settings and optional palette commands only
- `WorkflowQuickSwitch.vue` remains dev-only

### 8. Settings continuity posture

Decision:

- do not change the workspace model or the `settingsWorkspaceActive` arrangement in this pass
- document the current Settings + Execution relationship and leave structural change to a later focused follow-up

## Out Of Scope

- adding a left sidebar or alternate shell paradigm
- restructuring workspace or workflow enums
- altering backend command ownership
- changing the floating Task Center overlay into the primary shell affordance
- adding persistent shell-level perspective chrome
- solving Settings + Execution topology in this pass
- startup animation, execution safety, or `MlSignal` shell work beyond incidental consistency checks

## Success Criteria

This shell pass is successful only if it produces all of the following:

- less duplicated top-shell context than the current implementation
- a clearer task-center entry within the main shell hierarchy
- preserved tab-first workstation vocabulary
- preserved keyboard-safe path through workspace tabs, workflow steps, controls/results tabs, and overlays
- preserved canonical capability reachability
- no noticeable increase in top-shell height or visual noise
- no regression in lazy-loading/bundle posture

## Product-Owner Overrides

These are not open design questions by default. They are override points only if product wants to deviate from the working decisions above.

- [ ] Override task-center launcher placement
- [ ] Override titlebar compact-chip posture
- [ ] Override default visible context-strip fields
- [ ] Override perspective deferral
- [ ] Override Settings + Execution deferral

## Phase 0 — Preflight And Baseline Capture

Goal: lock the intended shell scope and collect before-state evidence.

- [ ] Re-read [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md) and confirm no scope drift.
- [ ] Capture baseline screenshots for:
  - [ ] Workflow / Data
  - [ ] Workflow / Optimize
  - [ ] Workflow / Analyze
  - [ ] Run Library
  - [ ] Aggregate Analysis
  - [ ] Settings
- [ ] Record current top-shell duplication points to compare after implementation:
  - [ ] titlebar identity/subtitle
  - [ ] titlebar chips
  - [ ] context strip
  - [ ] footer
  - [ ] task center entry points
- [ ] Confirm the existing visible task-center launcher behavior in [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue) so the shell change is framed as integration, not duplication.

Acceptance criteria:

- [ ] Baseline screenshots captured and stored
- [ ] Existing shell duplication points documented in notes or task log
- [ ] Working decisions above remain accepted or overridden explicitly

## Phase 1 — Clarify The Visible Shell Hierarchy

Goal: make the main shell navigation and context layers clearer in [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue) without changing the workflow model.

Primary files:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- possibly [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts) if a small launcher state hook is needed

Checklist:

- [ ] Add or integrate a visible task-center launcher in the main shell/nav band.
- [ ] Ensure the launcher reads as secondary to workspace/workflow selection, not as a third primary navigation system.
- [ ] Reduce shell-header contextual noise.
- [ ] Reassess whether the current `VectorGrid` + caption + subtitle stack is too tall or too repetitive once the titlebar remains visible.
- [ ] Tighten the default visible context-strip items.
- [ ] Add overflow/disclosure treatment for secondary context metadata if needed.
- [ ] Keep current tab semantics intact:
  - [ ] workspace tablist
  - [ ] workflow-step tablist
  - [ ] controls tablist
  - [ ] results tablist
- [ ] Preserve current selection announcements and shell accessibility semantics.
- [ ] Preserve split-versus-stacked layout behavior.
- [ ] Avoid turning the context strip into a new action bar.

Before/after behavior:

- Before:
  - `VueWorkspaceShell.vue` shows a brand block, header caption, subtitle, workspace tabs, workflow-step tabs, and a seven-field context strip
  - no task-center affordance is integrated into the shell/nav band
- After:
  - `VueWorkspaceShell.vue` shows a simpler shell header
  - the header caption is removed from the persistent shell
  - a task-center launcher is integrated into the shell/nav hierarchy
  - the context strip defaults to five fields with secondary metadata behind disclosure

Per-file sub-checklist:

- [ ] `VueWorkspaceShell.vue`
  - [ ] remove persistent header caption
  - [ ] preserve `VectorGrid` heading
  - [ ] preserve subtitle line
  - [ ] add shell-level task-center launcher
  - [ ] keep launcher visually secondary to workspace/workflow tabs
  - [ ] reduce default context-strip items from seven to five
  - [ ] add overflow/disclosure for `Rows` and `Loaded ID`
  - [ ] keep current tab roles and keyboard handlers intact
- [ ] `workflowStore.ts` only if needed
  - [ ] add minimal coordination only if shell launcher behavior cannot remain local/UI-owned

Acceptance criteria:

- [ ] Workspace/workflow rows remain primary orientation
- [ ] Header is visibly quieter than before
- [ ] Task-center launcher is visible in the main shell hierarchy
- [ ] Context strip shows only the agreed default fields
- [ ] No new shell band is introduced

## Phase 2 — Quiet The Titlebar

Goal: turn [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue) back into desktop chrome plus one compact operational affordance.

Primary files:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- possibly [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)

Checklist:

- [ ] Collapse the current chip cluster.
- [ ] Decide whether one compact task/status launcher chip remains permanently visible.
- [ ] Remove or reduce dataset emphasis in the titlebar if that information is already better expressed in the shell context strip.
- [ ] Keep Search access.
- [ ] Keep Settings access.
- [ ] Keep one context-aware primary action.
- [ ] Refine primary action wording/weighting so it reads as contextual rather than universally primary.
- [ ] Preserve desktop drag, minimize, maximize/restore, and close behavior.
- [ ] Preserve focus styling and desktop-runtime availability behavior.

Before/after behavior:

- Before:
  - titlebar shows Dataset, Tasks, and Status chips simultaneously
  - titlebar competes with the shell header/context strip for context ownership
- After:
  - titlebar shows at most one compact task-center/status chip
  - dataset detail is no longer carried in titlebar chrome
  - Search, Settings, contextual Run/Stop, and window controls remain

Per-file sub-checklist:

- [ ] `DesktopTitlebar.vue`
  - [ ] remove dataset chip
  - [ ] remove wide status chip
  - [ ] replace three-chip cluster with one compact task-center/status launcher chip or compact chip cluster capped at one always-visible item
  - [ ] keep Search button
  - [ ] keep Settings button
  - [ ] keep contextual Run/Stop action
  - [ ] preserve drag region behavior
  - [ ] preserve native window control behavior
- [ ] `useTaskLifecycleSummary.ts` only if needed
  - [ ] provide a shorter summary suitable for one compact chip

Acceptance criteria:

- [ ] Titlebar no longer reads like a second dashboard row
- [ ] One-glance operational signal remains available
- [ ] No desktop-window-control regressions are introduced

## Phase 3 — Align Task Vocabulary Across Surfaces

Goal: make the task center the obvious detailed destination while removing redundant task/status language elsewhere.

Primary files:

- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)
- [statusPresentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\statusPresentation.ts)

Checklist:

- [ ] Align summary wording across titlebar, footer, task center, and any shell launcher text.
- [ ] Ensure any shell launcher opens the task center consistently.
- [ ] Preserve the task center as the detailed async-work destination.
- [ ] Keep the footer compact and workflow-scoped.
- [ ] Avoid repeating the same task status in titlebar, context strip, footer, and task center unless each surface has a clear distinct role.
- [ ] Recheck whether the context strip still needs a default `Status` item after launcher/titlebar/footer refinement.

Before/after behavior:

- Before:
  - task language is split across titlebar chips, floating task-center toggle, footer, and other shell surfaces
- After:
  - the task center is the explicit detailed destination
  - titlebar and shell launcher use short summary language
  - footer remains workflow-scoped and compact

Per-file sub-checklist:

- [ ] `TaskCenterOverlay.vue`
  - [ ] keep detailed task-center role unchanged
  - [ ] confirm launcher/deep-link behavior remains reliable
- [ ] `RunFooterPanel.vue`
  - [ ] keep footer compact
  - [ ] avoid duplicating detailed task-center language
- [ ] `DesktopTitlebar.vue`
  - [ ] reuse short task summary text only
- [ ] `statusPresentation.ts`
  - [ ] centralize wording where possible
- [ ] `useTaskLifecycleSummary.ts`
  - [ ] ensure shell-safe short summaries exist if required

Acceptance criteria:

- [ ] Task center is the obvious detailed async-work destination
- [ ] Top-shell task wording is shorter and less repetitive
- [ ] Footer still helps workflow progress without becoming another task dashboard

## Phase 4 — Keep Palette Secondary And Perspectives Deferred

Goal: prevent shell accretion while preserving expert speed.

Primary files:

- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [perspectiveSearch.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\perspectiveSearch.ts) only if needed

Checklist:

- [ ] Keep `WorkflowQuickSwitch.vue` dev-only.
- [ ] Confirm no production shell path begins to depend on quick-switch behavior.
- [ ] Audit command-palette labels so they match visible shell terminology.
- [ ] Optionally add palette commands for perspective restore/switch only if they clearly support the existing Settings-owned perspective model.
- [ ] Do not add a permanent perspective band, tab, or top-shell control in this pass.
- [ ] Confirm Settings remains the canonical visible perspective-management home.

Before/after behavior:

- Before:
  - palette is already strong
  - quick switch is dev-only
  - perspectives live outside permanent shell chrome
- After:
  - same overall posture remains
  - wording and commands align more cleanly with the visible shell

Per-file sub-checklist:

- [ ] `CommandPaletteOverlay.vue`
  - [ ] audit labels for shell-language consistency
  - [ ] remove stray terminology like `sidebar` if present
  - [ ] only add perspective commands if they reinforce the Settings-owned model
- [ ] `WorkflowQuickSwitch.vue`
  - [ ] confirm no production surface depends on it
- [ ] `SettingsWorkspacePanel.vue`
  - [ ] keep perspectives as the canonical visible management surface

Acceptance criteria:

- [ ] Palette remains powerful but visibly secondary
- [ ] No permanent perspective chrome is added
- [ ] Quick switch remains dev-only

## Phase 5 — Address Settings Continuity Later, Not Via Shell Accretion

Goal: document the follow-up work needed around Settings + Execution without bloating the shell now.

Primary files:

- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)
- [workflowModel.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\core\workflowModel.ts)

Checklist:

- [ ] Reconfirm the current `settingsWorkspaceActive` behavior and its UX implications.
- [ ] Record whether Settings + Execution is intentionally a combined operator workspace or only a temporary shell arrangement.
- [ ] Defer structural workspace changes unless this pass uncovers a blocking shell contradiction.
- [ ] If needed, write a small follow-up note or link to Prompt 6 / Prompt 8 work rather than solving this implicitly inside Prompt 1 implementation.

Before/after behavior:

- Before:
  - `settingsWorkspaceActive` mounts both Settings and Execution surfaces
- After:
  - this behavior is explicitly documented as either intentional for now or a later follow-up target
  - no accidental shell workaround is added

Per-file sub-checklist:

- [ ] `App.vue`
  - [ ] reconfirm current mount behavior
- [ ] `SettingsWorkspacePanel.vue`
  - [ ] note any shell continuity implications discovered during Prompt 1 work
- [ ] `ExecutionWorkspacePanel.vue`
  - [ ] note any shell continuity implications discovered during Prompt 1 work
- [ ] `workflowModel.ts`
  - [ ] do not change shell/workspace model in this pass unless a blocker forces it

Acceptance criteria:

- [ ] Settings continuity is documented clearly
- [ ] No shell bloat is introduced to mask the issue
- [ ] Any follow-up is explicitly linked to later prompt work

## File-Level Touch Points

Implementation is expected to touch some or all of the following:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)
- [statusPresentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\statusPresentation.ts)
- [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts) only if a small coordination hook is justified

## Expected Edit Inventory

This is the intended edit inventory for this pass.

- [ ] `VueWorkspaceShell.vue`
  - [ ] header reduction
  - [ ] task-center launcher integration
  - [ ] context-strip default-item reduction
  - [ ] context overflow/disclosure
- [ ] `DesktopTitlebar.vue`
  - [ ] chip reduction
  - [ ] compact task/status launcher behavior
  - [ ] contextual action wording check
- [ ] `TaskCenterOverlay.vue`
  - [ ] verify/open-link consistency only
- [ ] `RunFooterPanel.vue`
  - [ ] wording/alignment check only
- [ ] `CommandPaletteOverlay.vue`
  - [ ] terminology cleanup
  - [ ] optional perspective command check
- [ ] `WorkflowQuickSwitch.vue`
  - [ ] confirm no production dependence; likely no code changes
- [ ] `App.vue`
  - [ ] continuity confirmation only unless integration requires a small hookup
- [ ] `useTaskLifecycleSummary.ts`
  - [ ] short-summary support only if needed
- [ ] `statusPresentation.ts`
  - [ ] wording normalization only if needed

## Verification Checklist

### Required repo checks

- [ ] `npm --prefix web run lint`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run test`
- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run parity`
- [ ] `npm --prefix web run a11y:smoke`
- [ ] `npm --prefix web run baseline:ui`
- [ ] `npm --prefix web run check:bundle-budget`

Run when the slice changes titlebar/window-controls or true desktop-shell behavior:

- [ ] `npm --prefix web run typecheck:desktop`
- [ ] `npm --prefix web run test:desktop:smoke`

### Keyboard path proof

- [ ] Workspace tab row traversal still works.
- [ ] Workflow-step tab row traversal still works.
- [ ] Controls tab row traversal still works.
- [ ] Results tab row traversal still works.
- [ ] Command palette open/close and focus return still work.
- [ ] Task-center launcher open/close and focus return still work.
- [ ] No new launcher, disclosure, or overflow element creates a dead-end focus path.

### Capability-preservation proof

- [ ] Data path remains obvious.
- [ ] Optimize run/cancel path remains obvious.
- [ ] Validation run/export/report path remains obvious.
- [ ] Analysis results navigation remains obvious.
- [ ] Run Library remains obvious.
- [ ] Aggregate Analysis remains obvious.
- [ ] Settings remains obvious.
- [ ] Task center remains obvious.

### Responsive shell proof

- [ ] Wide desktop checked.
- [ ] Common laptop width checked.
- [ ] Narrower supported desktop width checked.
- [ ] Split-to-stacked transition remains clean.
- [ ] Top shell does not become taller or noisier at smaller widths.

### Desktop-shell proof

- [ ] Window drag behavior still works.
- [ ] Maximize/restore still works.
- [ ] Focus styling still works.
- [ ] Titlebar button usability still works.
- [ ] Overlay stacking with command palette/task center still works.

### Visual baseline proof

- [ ] Workflow / Data captured
- [ ] Workflow / Optimize captured
- [ ] Workflow / Analyze captured
- [ ] Run Library captured
- [ ] Aggregate Analysis captured
- [ ] Settings captured

## UX Pass/Fail Checklist

- [ ] Less duplicated top-shell context than before
- [ ] Clearer task-center entry in the main shell
- [ ] Preserved keyboard-safe path
- [ ] Preserved canonical capability reachability
- [ ] No noticeable increase in shell height or visual noise
- [ ] No regression in lazy-loading/bundle posture
- [ ] Titlebar no longer duplicates dataset/tasks/status at full weight
- [ ] Context strip no longer feels like a telemetry wall
- [ ] Palette still reads as an accelerator, not as required navigation

## Risks To Watch During Execution

- [ ] Competing primary surfaces still remain after cleanup.
- [ ] Perspectives start creeping into permanent shell chrome.
- [ ] The context strip becomes too dynamic or too dense.
- [ ] Canonical capabilities become harder to find.
- [ ] Keyboard regressions appear around new launcher/disclosure elements.
- [ ] Settings continuity gets papered over instead of documented for later focused work.

## Conservative Fallback Path

If the full shell pass proves too risky, the fallback implementation is:

- [ ] keep the shell structure almost exactly as-is
- [ ] add or better integrate a visible task-center launcher only
- [ ] reduce titlebar chips slightly
- [ ] trim the context strip to a smaller default set
- [ ] leave perspectives fully in Settings/palette

## Completion Note

Mark this tracker complete only when:

- implementation tasks are checked off
- verification tasks are checked off
- screenshots/evidence are collected
- any deferred product-owner choices are recorded explicitly
- the resulting shell still reads as a quieter workstation shell rather than a broader dashboard shell

## Change Log

- [ ] Initial explicit implementation-spec pass added
- [ ] Working decisions locked
- [ ] Acceptance criteria expanded
- [ ] Per-file edit inventory expanded
