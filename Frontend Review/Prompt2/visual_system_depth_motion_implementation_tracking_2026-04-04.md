# Visual System, Depth, And Motion Implementation Tracking

Date: 2026-04-04

Scope: implementation tracker for the grounded Prompt 2 visual-system direction in [visual_system_depth_motion_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt2\visual_system_depth_motion_direction_FINAL.md).

Status: implemented and verified

Owner posture: frontend visual-system refinement only. Do not broaden into shell information-architecture redesign, backend contract changes, or product-mode decisions.

## Purpose

This document turns the valid Prompt 2 conclusions into an execution tracker with:

- explicit phase ordering
- objective-first checkpointing
- concrete file ownership
- implementation checklist items
- clear non-goals
- verification gates and evidence requirements

This tracker is for executing the grounded Prompt 2 direction:

- keep `Signal Foundry` and treat this as a tightening pass, not a redesign
- warm the light theme and retune semantic tokens rather than expanding the palette aggressively
- complete the missing shared depth/token pieces already implied by the repo
- move tab grammar away from pill chrome toward a flatter underline-led state model
- reduce routine shell/titlebar blur and glow
- align dense review, overlay, and diagnostics surfaces around one clearer depth language
- preserve Prompt 1 shell structure and the frontend functional-coverage guardrail

## Functionality Posture

This plan is additive and refinement-oriented in nature.

It is intended to:

- improve visual hierarchy
- complete shared token coverage
- normalize shared states and motion usage
- tighten shell, review, overlay, and diagnostics presentation

It is not intended to:

- remove backend-backed functionality
- hide existing capabilities behind deeper navigation
- change workflow ownership or product modes
- replace current frontend surfaces with narrower feature access

If an implementation choice would trade away capability reachability for visual cleanliness, that choice is out of scope for this plan.

## Progress Ledger

- [x] Phase 0 complete: baseline locked and evidence captured
- [x] Phase 1 complete: objective shared-token and tab-grammar changes landed
- [x] Phase 2 complete: objective shell/titlebar visual tightening landed
- [x] Phase 3 complete: objective dense-review surface alignment landed
- [x] Objective checkpoint complete and ready for clean commit
- [x] Phase 4 complete: overlay and diagnostics visual alignment landed
- [x] Phase 5 complete: subjective weighting refinements landed
- [x] Subjective refinement checkpoint complete
- [x] Targeted verification complete
- [x] Final evidence and screenshots attached

## Implementation Outcome Target

Target surfaces:

- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)
  - warmer light theme
  - completed shared token set
  - explicit shared font tokens
  - flatter shared tab grammar
  - stricter blur/elevation discipline
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
  - calmer resting chrome
  - less pill-shaped shell tabs
  - more recessed context-strip presentation
  - task-center launcher reads as a tray entry, not a gadget
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
  - quieter routine chrome
  - reduced accent-glow and raised-sheet feel at rest
- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
  - shared token cleanup
  - selection/focus language preserved and tightened
  - remove drift caused by local visual special-casing
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
  - clearer tiering between summary and table
  - shared disclosure and dense-table styling
  - no forced sticky-selection model unless later implementation truly adds row-driven downstream review
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
  - stronger separation reserved for overlay panel, not the launcher
  - less floating-widget feel
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - diagnostics surfaces read like evidence sheets instead of dashboard tiles
  - visual relation to task/runtime surfaces becomes clearer

## Checkpoint Strategy

This tracker is intentionally split into two implementation checkpoints.

### Checkpoint A — Objective Improvements

This checkpoint should contain only changes that are structurally useful and objectively testable.

Examples:

- add missing shared token definitions actually referenced by components
- introduce explicit shared font tokens
- retune light-theme base values away from cold SaaS white
- replace pill-led tab grammar with flatter underline-led shared states
- reduce routine shell/titlebar blur and accent-shadow intensity
- unify shared disclosure styling and dense-table framing
- preserve current capability reachability and keyboard paths

Commit goal:

- produce a clean git commit after these changes land and pass verification
- this is the baseline visual-system improvement commit that should remain even if later aesthetic weighting is reverted

### Checkpoint B — Subjective Refinements

This checkpoint should contain changes that are still reasonable, but are more about weighting, tone, and feel than structural correctness.

Examples:

- exact warmth/contrast values within the approved light-theme direction
- exact shell/titlebar visual quietness beyond the objective blur/glow reductions
- exact task-center launcher calmness at idle
- exact border/shadow weighting for overlay and diagnostics surfaces
- exact how-far-to-push surface separation between `surfaceSunken`, `surfaceWorking`, and `surfaceFocus`

Workflow goal:

- make these changes after Checkpoint A is safely committed
- keep them isolated enough that they can be stashed, split, or reverted if they harm the UI/UX

## Objective-First Execution Rule

Execution order for this tracker is locked:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Objective verification
6. Clean objective commit checkpoint
7. Phase 4
8. Phase 5

Implementation rule:

- do not begin Phase 4 or Phase 5 until the Objective Checkpoint is complete
- treat Phase 1 through Phase 3 as the default “must-land” work
- treat Phase 4 as additive-but-secondary
- treat Phase 5 as optional subjective polish

## Source Basis

Primary source:

- [visual_system_depth_motion_direction_FINAL.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt2\visual_system_depth_motion_direction_FINAL.md)

Supporting repo policy/context:

- [frontend_theme_proposal_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Unfinished\frontend_theme_proposal_2026-03-28.md)
- [frontend_depth_ux_implementation_brief_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Unfinished\frontend_depth_ux_implementation_brief_2026-03-28.md)
- [frontend_functional_coverage_matrix_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\frontend_functional_coverage_matrix_2026-03-28.md)
- [shell_hierarchy_workstation_implementation_tracking_2026-04-03.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Prompt1\shell_hierarchy_workstation_implementation_tracking_2026-04-03.md)

Primary implementation files:

- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [ConfirmationDialog.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ConfirmationDialog.vue)

## Execution Guardrails

- Treat Prompt 1 shell hierarchy as implemented baseline. Do not reopen shell IA in this pass.
- Keep `Signal Foundry` as the visual direction. This pass is a tightening pass, not a redesign.
- Preserve at least one clear frontend path for every backend-backed capability.
- Preserve current keyboard-safe tab semantics, focus management, overlay behavior, and dense-workflow readability.
- Do not add a large new token taxonomy beyond the minimal shared tokens this pass actually needs.
- Do not force font packaging or a custom font asset rollout in this pass.
- Do not force Validation to mimic Top’s exact sticky-selection model unless Validation later becomes selection-driven in the code.
- Do not broaden Prompt 2 into Prompt 6 execution/settings information-architecture work.
- Do not introduce motion-first “premiumization,” chart restyling, or decorative glassmorphism.

## Explicit Working Decisions

These are the working decisions for implementation in this pass. Treat them as locked defaults unless the product owner explicitly overrides them.

### 1. Theme direction

Decision:

- keep `Signal Foundry`
- warm the light theme toward the mineral/paper direction already proposed
- keep dark theme in graphite/slate territory
- keep blue as the only primary action/focus/selection accent
- keep green success-only and amber warning-only

Implementation meaning:

- this is a retune of current semantic values, not a palette expansion project

### 2. Shared token scope

Decision:

- add `--surface-overlay`
- add explicit `--font-sans` and `--font-mono` shared tokens
- optionally add `--surface-shell` only if shell/titlebar normalization truly needs it after Phase 1 inspection

Implementation meaning:

- do not add broad new token families unless a concrete component gap requires them
- fix shared-token omissions before component-local visual patches

### 3. Tab grammar

Decision:

- replace pill-led shared tab presentation with flatter base tracks, quieter inactive tabs, and underline-led active states
- keep the active-state cue anchored in underline, text emphasis, and restrained field tint

Implementation meaning:

- this applies to shared tab patterns first, then shell-specific tab variants
- inactive tabs should not rely on pill silhouette or shadow for identity

### 4. Depth and blur discipline

Decision:

- routine shell chrome and routine working panes become flatter and calmer at rest
- overlays keep the strongest separation
- blur remains allowed, but routine shell/titlebar usage should be reduced

Implementation meaning:

- treat blur and shadow as support for hierarchy, not as default chrome

### 5. Dense review surfaces

Decision:

- Top remains the current pilot for sticky selection summary, row rail, and clearer selected-state grammar
- Validation should inherit clearer tiering, disclosure styling, and dense-table treatment
- Validation should not be forced into a sticky-selection model in this pass

Implementation meaning:

- align Validation with the shared visual language
- do not invent a fake row-selection narrative where the current workflow does not need one

### 6. Settings and task/runtime surfaces

Decision:

- task center overlay may keep stronger elevation than routine shell chrome
- task-center launcher should become calmer at idle
- Settings diagnostics should read like evidence blocks with compact semantic states

Implementation meaning:

- improve cohesion between task, runtime, and diagnostics surfaces without changing their IA ownership

## Out Of Scope

- shell hierarchy redesign
- left-sidebar/dashboard-shell pivots
- command-palette navigation redesign
- workflow/mode separation work
- custom-font asset packaging
- chart-library changes
- execution/settings IA changes that belong to Prompt 6
- force-fitting Validation into Top’s exact interaction model

## Phase Plan

### Phase 0 — Baseline lock and visual inventory

Goal:

- capture the current visual baseline before any tightening work begins

Checklist:

- [x] Capture before-state screenshots for:
  - Workflow / Optimize / Top
  - Workflow / Validate
  - Settings
  - Task Center
- [x] Capture both light and dark themes
- [x] Capture Standard and Dense density modes
- [x] Capture reduced-motion behavior for tabs and overlays
- [x] Note current tab grammar, blur usage, and routine elevation hotspots
- [x] Note any components already relying on missing shared tokens

Acceptance:

- there is a stable before-state visual record for all Prompt 2 target surfaces

### Phase 1 — Shared tokens, palette, and tab grammar

Primary files:

- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)

Goal:

- land the highest-value, objective shared visual-system fixes first

Checklist:

- [x] Add `--surface-overlay` in both themes
- [x] Add `--font-sans` and `--font-mono` and route shared typography through them
- [x] Warm the light theme base values toward the approved paper/mineral direction
- [x] Retune warning toward brass/amber while keeping semantic meaning strict
- [x] Review whether `--surface-shell` is needed; add only if real shell/titlebar normalization requires it
- [x] Rewrite shared tab grammar away from pill-led active states
- [x] Reduce inactive tab shadowing and routine raised feel
- [x] Keep reduced-motion behavior intact
- [x] Replace or normalize any stray raw transition timings encountered in shared primitives

Acceptance:

- shared tokens are complete for current component usage
- tabs no longer read primarily as pill buttons
- light theme is warmer without harming readability

### Phase 2 — Shell and titlebar visual tightening

Primary files:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)

Goal:

- visually support the implemented Prompt 1 shell hierarchy without changing the shell structure

Checklist:

- [x] Align workspace/workflow/controls/results tabs to the new shared grammar
- [x] Reduce shell/titlebar blur and accent-glow intensity at rest
- [x] Reduce routine raised-sheet feeling in the shell header and titlebar
- [x] Make the context strip read more like a recessed support band than a telemetry shelf
- [x] Keep the task-center launcher legible while reducing gadget-like idle weight
- [x] Keep titlebar actions clear without making the titlebar itself a visually active dashboard band

Acceptance:

- shell chrome feels calmer at rest and clearer only where state changes matter
- Prompt 1 structural improvements remain intact

### Phase 3 — Dense review surface alignment

Primary files:

- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)

Goal:

- tighten shared review-surface language without forcing false interaction symmetry

Checklist:

- [x] Keep Top’s selection rail and sticky summary language intact, but align it to the finalized shared tokens
- [x] Fix any Top styles relying on missing or ad hoc token assumptions
- [x] Make Validation summary and table read as one analytical unit through clearer tiering
- [x] Align Validation disclosure, details, column controls, and dense-table framing to shared patterns
- [x] Ensure dense-table overflow shadows and selected/hover states remain readable in both themes
- [x] Do not add sticky-selection UI to Validation unless a real downstream row-selection dependency is introduced

Acceptance:

- Top remains the stronger selection-driven review surface
- Validation becomes more visually coherent without pretending to be Top

### Objective Checkpoint

Checklist:

- [x] Phase 1 complete
- [x] Phase 2 complete
- [x] Phase 3 complete
- [x] Objective verification passed
- [x] Clean commit checkpoint prepared

### Phase 4 — Overlay and diagnostics alignment

Primary files:

- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [ConfirmationDialog.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ConfirmationDialog.vue)

Goal:

- reserve the strongest visual elevation for true overlays and make diagnostics/support surfaces visually coherent

Checklist:

- [x] Keep overlay panel separation stronger than routine shell chrome
- [x] Reduce floating-widget feel in the task-center launcher/toggle
- [x] Keep task-center panel visually distinct as an assistant tray
- [x] Make Settings diagnostics read more like evidence blocks than dashboard tiles
- [x] Align compact semantic badges/chips across task, runtime, and diagnostics surfaces
- [x] Keep command palette and confirmation dialogs crisp and overlay-like rather than glassy or theatrical

Acceptance:

- overlays own the strongest elevation
- diagnostics and task surfaces feel like related parts of one product

### Phase 5 — Subjective weighting and finish pass

Primary files:

- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Goal:

- adjust visual weighting only after the objective baseline is safe

Checklist:

- [x] Tune exact warmth/contrast values within the approved palette direction
- [x] Tune exact shell/titlebar calmness without losing clarity
- [x] Tune exact idle emphasis of the task-center launcher
- [x] Tune exact overlay border/shadow weight
- [x] Tune exact separation between `surfaceSunken`, `surfaceWorking`, and `surfaceFocus`

Acceptance:

- the UI feels more intentional without looking decorative or noisy
- refinements remain easy to stash or revert independently

## File-Level Notes

### [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)

- highest-value file in the pass
- should own the palette retune, missing token additions, shared font tokens, tab grammar rewrite, and blur/elevation discipline

### [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)

- should consume the shared grammar rather than inventing shell-local tab logic
- should become calmer at rest without reopening Prompt 1 hierarchy choices

### [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)

- should remain compact operational chrome
- should reduce routine blur/glow while preserving clarity and affordance weighting

### [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)

- remains the best pilot for shared selection/focus language
- should not depend on undefined shared tokens after this pass

### [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)

- should gain stronger tiering and cleaner dense-table styling
- should not be forced into sticky-selection UX unless the workflow becomes selection-driven

## Verification Plan

### Required repo checks

- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run build`
- [x] `npm --prefix web run a11y:smoke`
- [x] `npm --prefix web run baseline:ui`

### Required manual review set

- [x] Workflow / Optimize / Top
- [x] Workflow / Validate
- [x] Settings
- [x] Task Center
- [x] light and dark themes
- [x] Standard and Dense density modes
- [x] reduced-motion behavior
- [x] active-tab clarity
- [x] selected-row clarity
- [x] overlay hierarchy

### Guardrail review

- [x] Re-check frontend functional-coverage guardrail after any decluttering around tabs, disclosures, diagnostics, or overlays
- [x] Re-check keyboard paths for tabs, dense tables, and overlays
- [x] Re-check Prompt 1 shell hierarchy remains visually supported rather than undermined

## Evidence Captured

- Existing repo baseline captures dated 2026-02-26 and 2026-03-28 were used as historical before-state reference for shell/workspace posture.
- Refreshed Prompt 2 baseline artifacts were captured on 2026-04-04 under [scripts/baselines/ui_capture](C:\Rust Projects\ta_desktop_demo\scripts\baselines\ui_capture), including:
  - `workflow_optimize_results_top.png`
  - `workflow_step_validate.png`
  - `workspace_settings.png`
  - `workspace_workflow.png`
  - `fullpage_final.png`
  - `perf_metrics.json`
- Live manual captures were added for surfaces not fully represented by the baseline script:
  - `task_center_open_manual.png`
  - `task_center_open_dark_dense_reduce.png`
  - `settings_diagnostics_manual.png`

## Risks

- warming the light theme too far and reducing analytical contrast
- flattening tabs too far and losing state clarity
- reducing shell/titlebar blur and shadow too far and making the app feel dull rather than precise
- overapplying Top-style review affordances to Validation where the interaction model does not justify them
- broadening Prompt 2 into later-prompt IA work

## Fallback Path

If subjective refinements regress clarity:

- keep Checkpoint A
- stash or revert Checkpoint B
- keep the shared token completion, light-theme retune, tab-grammar correction, and blur/elevation discipline changes
- defer further weighting changes until after a targeted visual review
