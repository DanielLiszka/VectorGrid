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

## Success Criteria

This shell pass is successful only if it produces all of the following:

- less duplicated top-shell context than the current implementation
- a clearer task-center entry within the main shell hierarchy
- preserved tab-first workstation vocabulary
- preserved keyboard-safe path through workspace tabs, workflow steps, controls/results tabs, and overlays
- preserved canonical capability reachability
- no noticeable increase in top-shell height or visual noise
- no regression in lazy-loading/bundle posture

## Product-Owner Decisions

These should be resolved before or during implementation, not left implicit.

- [ ] Decide whether the titlebar keeps one compact task/status launcher chip or collapses status even further.
- [ ] Decide the exact default visible context-strip fields.
  Current recommendation:
  - keep: `Dataset`, `Range`, `Strategy`, `Backend`, `Status`
  - move to overflow/disclosure: `Rows`, `Loaded ID`
- [ ] Decide the exact placement of the task-center launcher within the main shell/nav band.
- [ ] Confirm that perspectives remain in Settings/palette flows for this pass.
- [ ] Confirm that Settings + Execution continuity is deferred to a later focused follow-up rather than partially solved here.

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
