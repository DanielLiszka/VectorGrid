# Shell Hierarchy And Workstation Implementation Tracking

Date: 2026-04-03

Scope: implementation tracker for the Prompt 1 shell/workstation direction in [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md).

Status: not started

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

- [ ] Phase 0 complete: baseline locked and evidence captured
- [ ] Phase 1 complete: objective shell hierarchy changes landed
- [ ] Phase 2 complete: objective titlebar reduction landed
- [ ] Phase 3 complete: objective shell guardrails and continuity checks landed
- [ ] Objective checkpoint complete and ready for clean commit
- [ ] Phase 4 complete: subjective shell weighting/refinement landed
- [ ] Phase 5 complete: subjective wording/alignment refinements landed
- [ ] Subjective refinement checkpoint complete
- [ ] Verification complete
- [ ] Final evidence and screenshots attached

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

- [ ] Titlebar chip burden reduced from three always-visible chips to at most one optional compact chip slot
- [ ] Default context-strip fields reduced from seven to five visible items
- [ ] `Rows` and `Loaded ID` moved out of the default context strip
- [ ] Shell header persistent descriptive copy reduced from heading + caption + subtitle to heading + subtitle
- [ ] A task-center launcher is integrated into the main shell header/nav area
- [ ] The shell-level task-center launcher reliably opens the task center
- [ ] Keyboard regressions introduced: `0`
- [ ] Lost canonical capability paths introduced: `0`
- [ ] New shell bands introduced: `0`

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

## Phase 1 — Objective Shell Hierarchy Changes

Goal: land the objective shell-structure improvements in [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue) without changing the workflow model.

Primary files:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- possibly [workflowStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\workflowStore.ts) if a small launcher state hook is needed

Checklist:

- [ ] Add or integrate a visible task-center launcher in the main shell/nav band.
- [ ] Ensure the launcher is present in the shell/nav band.
- [ ] Ensure the launcher reliably opens the task center.
- [ ] Reduce shell-header contextual noise.
- [ ] Remove the persistent caption from the shell header.
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
  - [ ] ensure launcher reliably opens task center
  - [ ] reduce default context-strip items from seven to five
  - [ ] add overflow/disclosure for `Rows` and `Loaded ID`
  - [ ] keep current tab roles and keyboard handlers intact
- [ ] `workflowStore.ts` only if needed
  - [ ] add minimal coordination only if shell launcher behavior cannot remain local/UI-owned

Acceptance criteria:

- [ ] Workspace/workflow rows remain primary orientation
- [ ] Header contains less persistent descriptive copy than before
- [ ] Task-center launcher is visible in the main shell hierarchy
- [ ] Task-center launcher opens the task center reliably
- [ ] Context strip shows only the agreed default fields
- [ ] No new shell band is introduced

## Phase 2 — Objective Titlebar Reduction

Goal: remove the objectively redundant titlebar burden in [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue) while preserving core actions and window controls.

Primary files:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- possibly [useTaskLifecycleSummary.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useTaskLifecycleSummary.ts)

Checklist:

- [ ] Collapse the current chip cluster.
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
  - titlebar shows no more than one compact task-center/status chip slot
  - dataset detail is no longer carried in titlebar chrome
  - Search, Settings, contextual Run/Stop, and window controls remain

Per-file sub-checklist:

- [ ] `DesktopTitlebar.vue`
  - [ ] remove dataset chip
  - [ ] remove wide status chip
  - [ ] reduce chip region to a single compact task-center/status slot or empty placeholder area pending subjective refinement
  - [ ] keep Search button
  - [ ] keep Settings button
  - [ ] keep contextual Run/Stop action
  - [ ] preserve drag region behavior
  - [ ] preserve native window control behavior
- [ ] `useTaskLifecycleSummary.ts` only if needed
  - [ ] provide a shorter summary suitable for one compact chip

Acceptance criteria:

- [ ] Titlebar no longer reads like a second dashboard row
- [ ] Titlebar retains its core operational actions
- [ ] No desktop-window-control regressions are introduced

## Phase 3 — Objective Guardrails, Continuity, And Checkpoint Readiness

Goal: confirm the shell still obeys its guardrails, document deferred continuity work, and prepare the objective checkpoint commit.

Primary files:

- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)
- [workflowModel.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\core\workflowModel.ts)

Checklist:

- [ ] Confirm the task center remains the detailed async-work destination.
- [ ] Reconfirm the current `settingsWorkspaceActive` behavior and its UX implications.
- [ ] Record whether Settings + Execution is intentionally combined for now or merely deferred.
- [ ] Defer structural workspace changes unless this pass uncovers a blocker.
- [ ] Prepare objective-checkpoint verification and commit readiness.

Before/after behavior:

- Before:
  - the shell has structural duplication and an unresolved Settings continuity question
- After:
  - task center remains the detailed destination
  - Settings continuity is explicitly deferred rather than implicitly "fixed"
  - the objective shell changes are isolated and ready for a clean checkpoint commit

Per-file sub-checklist:

- [ ] `TaskCenterOverlay.vue`
  - [ ] keep detailed task-center role unchanged
  - [ ] confirm shell integration did not break the existing detailed destination role
- [ ] `App.vue`
  - [ ] reconfirm current mount behavior
- [ ] `SettingsWorkspacePanel.vue`
  - [ ] note continuity implications only
- [ ] `ExecutionWorkspacePanel.vue`
  - [ ] note continuity implications only
- [ ] `workflowModel.ts`
  - [ ] do not change shell/workspace model in objective pass unless a blocker forces it

Acceptance criteria:

- [ ] Task center is the obvious detailed async-work destination
- [ ] Settings continuity is documented, not papered over
- [ ] Objective shell changes are isolated enough for a clean git commit

## Objective Checkpoint Commit

The implementation should stop here for the first commit checkpoint.

Commit should happen only after all of the following are true:

- [ ] Phases 0 through 3 are complete
- [ ] Objective Improvement Targets are complete
- [ ] Verification checklist for the objective scope passes
- [ ] No unresolved blocker remains in shell navigation, titlebar behavior, or context-strip structure
- [ ] The diff contains only objective structural improvements and explicit documentation updates

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

- [ ] Decide whether the titlebar keeps one compact task/status chip or none.
- [ ] Refine launcher visual weight if needed.
- [ ] Refine subtitle emphasis if needed.
- [ ] Re-evaluate whether `Status` remains in the default visible context strip.
- [ ] Align short-form task wording across titlebar, shell launcher, footer, and task center.
- [ ] Keep `WorkflowQuickSwitch.vue` dev-only.
- [ ] Confirm no production shell path begins to depend on quick-switch behavior.
- [ ] Audit command-palette labels so they match visible shell terminology.
- [ ] Optionally add palette commands for perspective restore/switch only if they clearly support the existing Settings-owned perspective model.
- [ ] Do not add a permanent perspective band, tab, or top-shell control in this pass.
- [ ] Confirm Settings remains the canonical visible perspective-management home.

Before/after behavior:

- Before:
  - objective shell structure is improved but visual weighting and short-form wording may still need tuning
- After:
  - same overall posture remains
  - subjective weighting and wording are cleaner without changing the core objective structure

Per-file sub-checklist:

- [ ] `DesktopTitlebar.vue`
  - [ ] decide one compact chip versus zero persistent chips
  - [ ] refine chip wording and weight if retained
- [ ] `VueWorkspaceShell.vue`
  - [ ] refine task-center launcher visual weight
  - [ ] refine subtitle emphasis if needed
  - [ ] re-evaluate `Status` default visibility if still noisy
- [ ] `TaskCenterOverlay.vue`
  - [ ] align short-form naming only if needed
- [ ] `RunFooterPanel.vue`
  - [ ] align short-form wording only if needed
- [ ] `statusPresentation.ts`
  - [ ] centralize refined short-form wording only if needed
- [ ] `useTaskLifecycleSummary.ts`
  - [ ] provide refined short summaries only if needed
- [ ] `CommandPaletteOverlay.vue`
  - [ ] audit labels for shell-language consistency
  - [ ] remove stray terminology like `sidebar` if present
  - [ ] only add perspective commands if they reinforce the Settings-owned model
- [ ] `WorkflowQuickSwitch.vue`
  - [ ] confirm no production surface depends on it
- [ ] `SettingsWorkspacePanel.vue`
  - [ ] keep perspectives as the canonical visible management surface

Acceptance criteria:

- [ ] Subjective changes do not alter the core objective shell structure
- [ ] Subjective changes are isolated enough to stash or revert independently
- [ ] Palette remains powerful but visibly secondary
- [ ] No permanent perspective chrome is added
- [ ] Quick switch remains dev-only

## Phase 5 — Subjective Refinement Review And Defer Log

Goal: record what subjective refinements were kept, what was rejected, and what remains deferred.

Primary files:

- [shell_hierarchy_workstation_implementation_tracking_2026-04-03.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_implementation_tracking_2026-04-03.md)
- [shell_hierarchy_workstation_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_direction_FINAL.md)

Checklist:

- [ ] Record which subjective refinements were kept.
- [ ] Record which subjective refinements were rejected or reverted.
- [ ] Record whether a later Prompt 6 / Prompt 8 follow-up is needed for Settings continuity.
- [ ] Record whether any palette/perspective refinement was deferred.
- [ ] Confirm the subjective checkpoint can be stashed, split, or reverted cleanly if needed.

Before/after behavior:

- Before:
  - objective checkpoint is complete
  - subjective refinements may still be under evaluation
- After:
  - subjective keeps/rejects/defers are documented explicitly
  - later follow-up needs are recorded without muddying the objective checkpoint

Per-file sub-checklist:

- [ ] tracking doc
  - [ ] log subjective kept items
  - [ ] log subjective rejected items
  - [ ] log subjective deferred items
- [ ] final shell direction doc if needed
  - [ ] add brief note if implementation materially changed any recommendation

Acceptance criteria:

- [ ] Subjective changes are explicitly separated from objective improvements
- [ ] Later follow-up needs are documented clearly
- [ ] No subjective change is required to preserve the objective gains

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
  - [ ] short-summary support only if needed and preferably in subjective refinement
- [ ] `statusPresentation.ts`
  - [ ] wording normalization only if needed and preferably in subjective refinement

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

Objective-checkpoint minimum:

- [ ] `npm --prefix web run lint`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run test`
- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run a11y:smoke`
- [ ] `npm --prefix web run baseline:ui`

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
