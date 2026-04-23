# Shell Hierarchy And Workstation Implementation Tracking

Date: 2026-04-03

Scope: implementation tracker for the Prompt 1 shell/workstation direction in [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md).

Status: implemented and verified on 2026-04-03

Owner posture: frontend UI/UX shell refinement only. Do not broaden into backend contract changes, sidebar redesign, or unrelated productization work.

## Purpose

This document turns the adopted Prompt 1 shell direction into an execution tracker with:

- explicit phase ordering
- objective-first checkpointing
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

- [x] Phase 0 complete: baseline locked and evidence captured
- [x] Phase 1 complete: objective shell hierarchy changes landed
- [x] Phase 2 complete: objective titlebar reduction landed
- [x] Phase 3 complete: objective shell guardrails and continuity checks landed
- [x] Objective checkpoint complete and ready for clean commit
- [x] Phase 4 complete: subjective shell weighting/refinement landed
- [x] Phase 5 complete: subjective wording/alignment refinements landed
- [x] Subjective refinement checkpoint complete
- [x] Targeted verification complete
- [x] Final evidence and screenshots attached

## Implementation Outcome

Implemented surfaces:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
  - removed the persistent header caption
  - added a shell-level task-center launcher on the workspace-navigation band
  - reduced the default context strip to five visible fields
  - moved `Rows` and `Loaded ID` into compact overflow disclosure
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
  - removed dataset and wide status duplication
  - reduced the titlebar to identity, search, settings, run/stop, and one compact task signal
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
  - aligned open/close behavior with shared task-center visibility state and explicit return-focus handling
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - moved task-center open behavior onto the shared visibility helper
- [useTaskCenterVisibility.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskCenterVisibility.ts)
  - added shared task-center visibility, local-storage persistence, and return-focus plumbing for shell launchers and the overlay

Verification evidence:

- `npm run typecheck`
- `npm.cmd run test -- src/components/VueWorkspaceShell.test.ts src/components/DesktopTitlebar.test.ts src/components/TaskCenterOverlay.test.ts src/components/SettingsWorkspacePanel.test.ts`
- `npm run build`
- `npm.cmd run a11y:smoke`
- `npm.cmd run baseline:ui`
- `npm.cmd run test:desktop:smoke`

Artifacts captured:

- `web/scripts/baselines/ui_capture/`
- `web/scripts/baselines/ui_capture/perf_metrics.json`

## Checkpoint Strategy

This tracker is intentionally split into two implementation checkpoints.

### Checkpoint A — Objective Improvements

This checkpoint should contain only changes that are structurally useful and objectively testable.

Examples:

- remove duplicated shell/header copy
- reduce default context-strip item count
- move low-value metadata behind disclosure
- integrate a main-shell task-center launcher
- make the shell launcher reliably open the task center
- remove obviously redundant titlebar chips
- preserve keyboard paths and capability reachability

Commit goal:

- produce a clean git commit after these changes land and pass verification
- this is the baseline improvement commit that should be kept even if later subjective polish is reverted

### Checkpoint B — Subjective Refinements

This checkpoint should contain changes that are still reasonable, but are more about weighting, language, or feel than structural correctness.

Examples:

- whether one compact titlebar task/status chip is better than zero
- exact launcher visual weighting
- subtitle styling refinement
- whether `Status` should remain visible in the default context strip
- wording polish across titlebar/footer/task center
- optional palette terminology polish or perspective command additions

Workflow goal:

- make these changes after Checkpoint A is safely committed
- keep them isolated enough that they can be stashed, split, or reverted if they harm the UI/UX

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
- exact visual weighting of the launcher is deferred to the subjective-refinement checkpoint

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
- whether `Status` remains in the five-item default set is a subjective-refinement checkpoint question only if the objective pass still feels noisy

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
- whether the compact operational affordance is one chip or zero persistent chips is deferred to the subjective-refinement checkpoint

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

## Objective Improvement Targets

These are the concrete targets that should be achieved before the first checkpoint commit.

- [x] Titlebar chip burden reduced from three always-visible chips to at most one optional compact chip slot
- [x] Default context-strip fields reduced from seven to five visible items
- [x] `Rows` and `Loaded ID` moved out of the default context strip
- [x] Shell header persistent descriptive copy reduced from heading + caption + subtitle to heading + subtitle
- [x] A task-center launcher is integrated into the main shell header/nav area
- [x] The shell-level task-center launcher reliably opens the task center
- [x] Keyboard regressions introduced: `0`
- [x] Lost canonical capability paths introduced: `0`
- [x] New shell bands introduced: `0`

## Product-Owner Overrides

These are not open design questions by default. They are override points only if product wants to deviate from the working decisions above.

- [ ] Override task-center launcher placement
- [ ] Override titlebar compact-chip posture
- [ ] Override default visible context-strip fields
- [ ] Override perspective deferral
- [ ] Override Settings + Execution deferral

## Subjective Refinement Questions

These should be answered only after the objective checkpoint is implemented and reviewed.

- [ ] Should the titlebar keep one compact task/status chip or none at all?
- [ ] Does the shell-level task-center launcher need lighter or stronger visual weight?
- [ ] Should `shellSubtitle` keep its current emphasis or be visually softened?
- [ ] Should `Status` remain in the default visible context strip after the objective cleanup?
- [ ] Does wording need additional tightening across titlebar, footer, task center, and palette?

## Phase 0 — Preflight And Baseline Capture

Goal: lock the intended shell scope and collect before-state evidence.

- [x] Re-read [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md) and confirm no scope drift.
- [x] Capture baseline screenshots for:
  - [x] Workflow / Data
  - [x] Workflow / Optimize
  - [x] Workflow / Analyze
  - [x] Run Library
  - [x] Aggregate Analysis
  - [x] Settings
- [x] Record current top-shell duplication points to compare after implementation:
  - [x] titlebar identity/subtitle
  - [x] titlebar chips
  - [x] context strip
  - [x] footer
  - [x] task center entry points
- [x] Confirm the existing visible task-center launcher behavior in [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue) so the shell change is framed as integration, not duplication.

Acceptance criteria:

- [x] Baseline screenshots captured and stored
- [x] Existing shell duplication points documented in notes or task log
- [x] Working decisions above remain accepted or overridden explicitly

## Phase 1 — Objective Shell Hierarchy Changes

Goal: land the objective shell-structure improvements in [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue) without changing the workflow model.

Primary files:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- possibly [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts) if a small launcher state hook is needed

Checklist:

- [x] Add or integrate a visible task-center launcher in the main shell/nav band.
- [x] Ensure the launcher is present in the shell/nav band.
- [x] Ensure the launcher reliably opens the task center.
- [x] Reduce shell-header contextual noise.
- [x] Remove the persistent caption from the shell header.
- [x] Tighten the default visible context-strip items.
- [x] Add overflow/disclosure treatment for secondary context metadata if needed.
- [x] Keep current tab semantics intact:
  - [x] workspace tablist
  - [x] workflow-step tablist
  - [x] controls tablist
  - [x] results tablist
- [x] Preserve current selection announcements and shell accessibility semantics.
- [x] Preserve split-versus-stacked layout behavior.
- [x] Avoid turning the context strip into a new action bar.

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

- [x] `VueWorkspaceShell.vue`
  - [x] remove persistent header caption
  - [x] preserve `VectorGrid` heading
  - [x] preserve subtitle line
  - [x] add shell-level task-center launcher
  - [x] ensure launcher reliably opens task center
  - [x] reduce default context-strip items from seven to five
  - [x] add overflow/disclosure for `Rows` and `Loaded ID`
  - [x] keep current tab roles and keyboard handlers intact
- [x] `workflowStore.ts` only if needed
  - [x] add minimal coordination only if shell launcher behavior cannot remain local/UI-owned

Acceptance criteria:

- [x] Workspace/workflow rows remain primary orientation
- [x] Header contains less persistent descriptive copy than before
- [x] Task-center launcher is visible in the main shell hierarchy
- [x] Task-center launcher opens the task center reliably
- [x] Context strip shows only the agreed default fields
- [x] No new shell band is introduced

## Phase 2 — Objective Titlebar Reduction

Goal: remove the objectively redundant titlebar burden in [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue) while preserving core actions and window controls.

Primary files:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- possibly [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)

Checklist:

- [x] Collapse the current chip cluster.
- [x] Remove or reduce dataset emphasis in the titlebar if that information is already better expressed in the shell context strip.
- [x] Keep Search access.
- [x] Keep Settings access.
- [x] Keep one context-aware primary action.
- [x] Refine primary action wording/weighting so it reads as contextual rather than universally primary.
- [x] Preserve desktop drag, minimize, maximize/restore, and close behavior.
- [x] Preserve focus styling and desktop-runtime availability behavior.

Before/after behavior:

- Before:
  - titlebar shows Dataset, Tasks, and Status chips simultaneously
  - titlebar competes with the shell header/context strip for context ownership
- After:
  - titlebar shows no more than one compact task-center/status chip slot
  - dataset detail is no longer carried in titlebar chrome
  - Search, Settings, contextual Run/Stop, and window controls remain

Per-file sub-checklist:

- [x] `DesktopTitlebar.vue`
  - [x] remove dataset chip
  - [x] remove wide status chip
  - [x] reduce chip region to a single compact task-center/status slot or empty placeholder area pending subjective refinement
  - [x] keep Search button
  - [x] keep Settings button
  - [x] keep contextual Run/Stop action
  - [x] preserve drag region behavior
  - [x] preserve native window control behavior
- [x] `useTaskLifecycleSummary.ts` only if needed
  - [x] provide a shorter summary suitable for one compact chip

Acceptance criteria:

- [x] Titlebar no longer reads like a second dashboard row
- [x] Titlebar retains its core operational actions
- [x] No desktop-window-control regressions are introduced

## Phase 3 — Objective Guardrails, Continuity, And Checkpoint Readiness

Goal: confirm the shell still obeys its guardrails, document deferred continuity work, and prepare the objective checkpoint commit.

Primary files:

- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)
- [workflowModel.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\core\workflowModel.ts)

Checklist:

- [x] Confirm the task center remains the detailed async-work destination.
- [x] Reconfirm the current `settingsWorkspaceActive` behavior and its UX implications.
- [x] Record whether Settings + Execution is intentionally combined for now or merely deferred.
- [x] Defer structural workspace changes unless this pass uncovers a blocker.
- [x] Prepare objective-checkpoint verification and commit readiness.

Before/after behavior:

- Before:
  - the shell has structural duplication and an unresolved Settings continuity question
- After:
  - task center remains the detailed destination
  - Settings continuity is explicitly deferred rather than implicitly "fixed"
  - the objective shell changes are isolated and ready for a clean checkpoint commit

Per-file sub-checklist:

- [x] `TaskCenterOverlay.vue`
  - [x] keep detailed task-center role unchanged
  - [x] confirm shell integration did not break the existing detailed destination role
- [x] `App.vue`
  - [x] reconfirm current mount behavior
- [x] `SettingsWorkspacePanel.vue`
  - [x] note continuity implications only
- [x] `ExecutionWorkspacePanel.vue`
  - [x] note continuity implications only
- [x] `workflowModel.ts`
  - [x] do not change shell/workspace model in objective pass unless a blocker forces it

Acceptance criteria:

- [x] Task center is the obvious detailed async-work destination
- [x] Settings continuity is documented, not papered over
- [x] Objective shell changes are isolated enough for a clean git commit

## Objective Checkpoint Commit

The implementation should stop here for the first commit checkpoint.

Commit should happen only after all of the following are true:

- [x] Phases 0 through 3 are complete
- [x] Objective Improvement Targets are complete
- [x] Verification checklist for the objective scope passes
- [x] No unresolved blocker remains in shell navigation, titlebar behavior, or context-strip structure
- [x] The diff contains only objective structural improvements and explicit documentation updates

Suggested commit posture:

- objective shell hierarchy integration
- objective titlebar reduction
- objective context-strip reduction
- no subjective wording/weighting polish beyond what was required to make the structural changes coherent

## Phase 4 — Subjective Shell Weighting And Wording Refinement

Goal: refine the shell's feel and weighting after the objective checkpoint is safely committed.

Primary files:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)
- [statusPresentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\statusPresentation.ts)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [perspectiveSearch.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\perspectiveSearch.ts) only if needed

Checklist:

- [x] Decide whether the titlebar keeps one compact task/status chip or none.
- [x] Refine launcher visual weight if needed.
- [x] Refine subtitle emphasis if needed.
- [x] Re-evaluate whether `Status` remains in the default visible context strip.
- [x] Align short-form task wording across titlebar, shell launcher, footer, and task center.
- [x] Keep `WorkflowQuickSwitch.vue` dev-only.
- [x] Confirm no production shell path begins to depend on quick-switch behavior.
- [x] Audit command-palette labels so they match visible shell terminology.
- [x] Optionally add palette commands for perspective restore/switch only if they clearly support the existing Settings-owned perspective model.
- [x] Do not add a permanent perspective band, tab, or top-shell control in this pass.
- [x] Confirm Settings remains the canonical visible perspective-management home.

Before/after behavior:

- Before:
  - objective shell structure is improved but visual weighting and short-form wording may still need tuning
- After:
  - same overall posture remains
  - subjective weighting and wording are cleaner without changing the core objective structure

Per-file sub-checklist:

- [x] `DesktopTitlebar.vue`
  - [x] decide one compact chip versus zero persistent chips
  - [x] refine chip wording and weight if retained
- [x] `VueWorkspaceShell.vue`
  - [x] refine task-center launcher visual weight
  - [x] refine subtitle emphasis if needed
  - [x] re-evaluate `Status` default visibility if still noisy
- [x] `TaskCenterOverlay.vue`
  - [x] align short-form naming only if needed
- [x] `RunFooterPanel.vue`
  - [x] align short-form wording only if needed
- [x] `statusPresentation.ts`
  - [x] centralize refined short-form wording only if needed
- [x] `useTaskLifecycleSummary.ts`
  - [x] provide refined short summaries only if needed
- [x] `CommandPaletteOverlay.vue`
  - [x] audit labels for shell-language consistency
  - [x] remove stray terminology like `sidebar` if present
  - [x] only add perspective commands if they reinforce the Settings-owned model
- [x] `WorkflowQuickSwitch.vue`
  - [x] confirm no production surface depends on it
- [x] `SettingsWorkspacePanel.vue`
  - [x] keep perspectives as the canonical visible management surface

Acceptance criteria:

- [x] Subjective changes do not alter the core objective shell structure
- [x] Subjective changes are isolated enough to stash or revert independently
- [x] Palette remains powerful but visibly secondary
- [x] No permanent perspective chrome is added
- [x] Quick switch remains dev-only

## Phase 5 — Subjective Refinement Review And Defer Log

Goal: record what subjective refinements were kept, what was rejected, and what remains deferred.

Primary files:

- [shell_hierarchy_workstation_implementation_tracking_2026-04-03.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_implementation_tracking_2026-04-03.md)
- [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md)

Checklist:

- [x] Record which subjective refinements were kept.
- [x] Record which subjective refinements were rejected or reverted.
- [x] Record whether a later Prompt 6 / Prompt 8 follow-up is needed for Settings continuity.
- [x] Record whether any palette/perspective refinement was deferred.
- [x] Confirm the subjective checkpoint can be stashed, split, or reverted cleanly if needed.

Before/after behavior:

- Before:
  - objective checkpoint is complete
  - subjective refinements may still be under evaluation
- After:
  - subjective keeps/rejects/defers are documented explicitly
  - later follow-up needs are recorded without muddying the objective checkpoint

Subjective outcomes recorded:

- kept:
  - visible `Status` in the five-field default context strip
  - shell subtitle with softened visual emphasis
  - shell-level task-center launcher with lighter idle visual weight
- rejected:
  - restoring a persistent titlebar task/status chip
  - restoring dataset or wide status duplication in the titlebar
  - adding permanent perspective chrome to the shell
- deferred:
  - deeper Settings + Execution topology changes
  - additional palette/perspective enhancement beyond current posture

Per-file sub-checklist:

- [x] tracking doc
  - [x] log subjective kept items
  - [x] log subjective rejected items
  - [x] log subjective deferred items
- [x] final shell direction doc if needed
  - [x] add brief note if implementation materially changed any recommendation

Acceptance criteria:

- [x] Subjective changes are explicitly separated from objective improvements
- [x] Later follow-up needs are documented clearly
- [x] No subjective change is required to preserve the objective gains

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

- [x] `VueWorkspaceShell.vue`
  - [x] header reduction
  - [x] task-center launcher integration
  - [x] context-strip default-item reduction
  - [x] context overflow/disclosure
- [x] `DesktopTitlebar.vue`
  - [x] chip reduction
  - [x] compact task/status launcher behavior
  - [x] contextual action wording check
- [x] `TaskCenterOverlay.vue`
  - [x] verify/open-link consistency only
- [ ] `RunFooterPanel.vue`
  - [ ] wording/alignment check only
- [ ] `CommandPaletteOverlay.vue`
  - [ ] terminology cleanup
  - [ ] optional perspective command check
- [x] `WorkflowQuickSwitch.vue`
  - [x] confirm no production dependence; likely no code changes
- [x] `App.vue`
  - [x] continuity confirmation only unless integration requires a small hookup
- [x] `useTaskLifecycleSummary.ts`
  - [x] short-summary support only if needed and preferably in subjective refinement
- [x] `statusPresentation.ts`
  - [x] wording normalization only if needed and preferably in subjective refinement

## Verification Checklist

### Required repo checks

- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run test`
  - current full-suite failures are outside this shell pass:
  - `src/services/metaService.test.ts`
  - `src/stores/settingsStore.actions.test.ts`
  - `src/tauri/commands.test.ts`
- [x] `npm --prefix web run build`
- [x] `npm --prefix web run parity`
- [x] `npm --prefix web run a11y:smoke`
- [x] `npm --prefix web run baseline:ui`
- [ ] `npm --prefix web run check:bundle-budget`
  - current failure: entry chunk `index-C_FwwUaB.js` is `631.3 kB` against a `600 kB` limit

Run when the slice changes titlebar/window-controls or true desktop-shell behavior:

- [x] `npm --prefix web run typecheck:desktop`
- [x] `npm --prefix web run test:desktop:smoke`

Objective-checkpoint minimum:

- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run typecheck`
- [x] targeted shell/titlebar/task-center/settings Vitest slice
- [x] `npm --prefix web run build`
- [x] `npm --prefix web run a11y:smoke`
- [x] `npm --prefix web run baseline:ui`

Subjective-refinement recommended:

- [ ] rerun visual baseline capture after subjective polish
- [ ] rerun desktop smoke if titlebar behavior or stacking changed again

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
- [ ] Objective-first / subjective-last checkpoint strategy added
