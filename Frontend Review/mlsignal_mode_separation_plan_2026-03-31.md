# MlSignal Mode Separation Plan (2026-03-31)

Status: deferred

## Goal

Turn `MlSignal` into a distinct top-level user mode instead of a strategy option inside the conventional optimization workflow.

This plan is intentionally deferred. It documents the target architecture and implementation checklist so the change can be picked up later without re-deciding the product model from scratch.

## Product Decision

Target user model:

- Conventional workflow remains the default mode.
- `MlSignal` becomes a separate mode in the same application shell.
- Shared foundations such as dataset loading, Polygon.io, history, and file reveal/open behavior may remain shared underneath.
- The post-meta ML-confidence finalizer stays part of the conventional workflow. It is not part of `MlSignal`.

## Conventional Workflow

Target conventional workflow:

1. Data
2. Optimize
3. Validate
4. Analyze
5. Finalize

Notes:

- `Finalize` is the existing post-meta ML-confidence / Stage 5 surface.
- Meta selection remains part of the conventional analysis/finalizer family.
- The conventional workflow should continue to default to normal strategy optimization, not `MlSignal`.

## MlSignal Workflow

Target `MlSignal` workflow:

1. Data
2. Dataset export
3. Model bundle and score-cache preparation
4. Validation
5. Policy sweep
6. History / artifact review

Notes:

- `MlSignal` is not just another optimization strategy.
- It has different inputs, artifacts, and lifecycle semantics.
- It should not be forced into optimizer-native concepts like Top rows, Pareto, or heatmap surfaces when those do not apply.

## Current State

Current repo behavior:

- the app already defaults into the conventional workflow
- `MlSignal` is currently implemented as a special `strategyKind`
- selecting `MlSignal` swaps the optimize surface into a dedicated `MlSignal` panel
- `MlSignal` artifacts already bypass standard optimizer analysis views
- the conventional post-meta ML-confidence finalizer is already wired separately in the Meta / Finalizer surfaces

Current consequence:

- the product intent is already split, but the shell vocabulary still implies `MlSignal` is only a strategy variant
- this creates mental-model drift for users and architectural drift for contributors

## Design Principles

- Keep one application shell. Do not fork the app into two separate codebases.
- Separate user modes before separating implementation. Shared stores/services may stay shared where they truly overlap.
- Do not move the conventional ML-confidence finalizer into `MlSignal`.
- Avoid backend contract churn in the first refactor slice unless the shell split exposes a real mismatch.
- Preserve history and artifact loading behavior during the transition.
- Keep persisted workflow restoration deterministic and backward-compatible.

## High-Level File Surface

Primary frontend files likely affected:

- `web/src/utils/core/workflowModel.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/WorkflowQuickSwitch.vue`
- `web/src/App.vue`
- `web/src/stores/optimizeStore.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Secondary repo surfaces to review before implementation:

- `REPO_MAP.md`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/preflight.ts`

## Proposed Implementation Shape

Preferred direction:

- introduce an explicit top-level `MlSignal` mode in the shell model
- keep the conventional workflow mode as the default
- route `MlSignal` through its own allowed controls/results combinations
- remove `MlSignal` from the normal strategy selector once the dedicated mode exists
- keep shared data context and shared artifact/history plumbing where it reduces duplication

Not required in the first slice:

- a separate Tauri window
- separate data stores for shared dataset context
- backend command renames
- backend artifact-format changes

## Phase Plan

### Phase 1: Shell Vocabulary

Define `MlSignal` as a real mode in the shell model instead of a strategy special-case.

Deliverables:

- new mode/workspace/workflow vocabulary chosen and documented
- allowed tab combinations updated
- mode subtitles and shell labels updated

Acceptance:

- the shell can represent conventional mode and `MlSignal` mode without overloading `strategyKind`

### Phase 2: Routing And Persistence

Update workflow restoration and persistence to remember the selected mode safely.

Deliverables:

- storage bootstrap supports the new mode
- legacy persisted state falls back cleanly
- command palette and quick-switch surfaces can navigate into the new mode

Acceptance:

- restarting the app restores the last valid mode without corrupting tab selection

### Phase 3: Panel Mounting

Mount `MlSignal` from the shell model rather than from `strategyKind === "MlSignal"`.

Deliverables:

- `App.vue` and `VueWorkspaceShell.vue` mount the right surfaces for the new mode
- optimize panel no longer owns `MlSignal` mode switching
- `MlSignal` panel remains operational

Acceptance:

- entering `MlSignal` mode shows the `MlSignal` panel without pretending it is a standard strategy workflow

### Phase 4: Strategy Selector Cleanup

Remove `MlSignal` from the conventional strategy selector once the separate mode is stable.

Deliverables:

- conventional strategy list no longer includes `MlSignal`
- strategy help text no longer frames `MlSignal` as a strategy variant
- any legacy history reruns still restore correctly

Acceptance:

- conventional users do not encounter `MlSignal` in normal strategy setup

### Phase 5: Shared Artifact And History Review

Confirm history, rerun, load, and analysis behavior still makes sense across both modes.

Deliverables:

- saved `MlSignal` artifacts reopen into `MlSignal` mode
- saved conventional artifacts reopen into conventional mode
- analysis surfaces continue to exclude `MlSignal` from optimizer-native views unless explicitly adapted later

Acceptance:

- artifact restoration chooses the correct mode automatically

### Phase 6: UX Polish And Naming Cleanup

Finish the shell-level terminology so the split is obvious to users.

Deliverables:

- mode labels, subtitles, and help copy aligned
- `MlSignal` mode copy no longer describes itself as a strategy alternative
- residual contributor-facing comments/docs updated where necessary

Acceptance:

- users can explain the difference between conventional workflow and `MlSignal` mode without reading implementation details

## Checklist

- [ ] Decide whether `MlSignal` should be represented as a workflow step, workspace, or separate shell mode key.
- [ ] Document the chosen shell vocabulary in `workflowModel.ts`.
- [ ] Add `MlSignal` mode defaults and allowed tab combinations.
- [ ] Update `workflowStore.ts` to persist and restore the new mode safely.
- [ ] Update `workflowBootstrap.ts` to migrate legacy persisted state into the new mode model.
- [ ] Update `VueWorkspaceShell.vue` labels, subtitles, and navigation handling.
- [ ] Update `WorkflowQuickSwitch.vue` to navigate the new mode.
- [ ] Update `CommandPaletteOverlay.vue` commands to expose the new mode explicitly.
- [ ] Update `App.vue` panel-mount logic so `MlSignal` mounts from mode selection, not `strategyKind`.
- [ ] Remove `MlSignal` from the standard strategy selector in `optimizeStore.ts` once the dedicated mode exists.
- [ ] Update `OptimizeWorkspacePanel.vue` so its special-case `MlSignal` branch is no longer the primary entry path.
- [ ] Keep `MlSignalWorkspacePanel.vue` operational with the new shell entry path.
- [ ] Review `RunFooterPanel.vue` so primary actions remain correct in both modes.
- [ ] Ensure `historyService.ts` restores conventional artifacts into conventional mode.
- [ ] Ensure `historyService.ts` restores `MlSignal` artifacts into `MlSignal` mode.
- [ ] Reconfirm `analysisService.ts` behavior for `MlSignal` artifacts after the mode split.
- [ ] Reconfirm `workflowArtifactsStore.ts` remains adequate for both modes.
- [ ] Update `REPO_MAP.md` to reflect the adopted mode split once implementation starts.
- [ ] Add or update focused tests for shell navigation, persistence, and artifact restoration.
- [ ] Run frontend verification after implementation: `typecheck`, `test`, `build`, `a11y:smoke`, `baseline:ui`.

## Non-Goals For The First Implementation

- native in-app CatBoost training
- native in-app CatBoost inference
- merging `MlSignal` outputs into optimizer-native Top / Pareto / heatmap analysis by default
- large backend refactors unrelated to shell separation

## Key Risk

The main risk is not visual regression. It is state-model confusion:

- persisted shell state
- history rerun behavior
- artifact restoration
- duplicated or conflicting navigation labels

That is why this work should begin with shell vocabulary and restoration logic rather than with visual polish.
