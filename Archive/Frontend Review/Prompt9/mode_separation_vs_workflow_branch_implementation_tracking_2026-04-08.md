# Mode Separation Vs Workflow Branch Implementation Tracking

Created: 2026-04-08

Prompt source: [mode_separation_vs_workflow_branch_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt9/mode_separation_vs_workflow_branch_direction_FINAL.md)

Scope: grounded Prompt 9 implementation only. This tracker converts the repo-backed Prompt 9 findings into a constrained implementation plan for `MlSignal` shell-mode separation while preserving the already-real options follow-up lane and avoiding speculative shell taxonomy growth.

Status: Implemented on 2026-04-08. Objective checkpoint complete and verified.

Owner posture: objective-first, additive, shell-truthfulness-biased, anti-proliferation-biased.

## Purpose

Turn the grounded Prompt 9 findings into a detailed implementation plan that:

- introduces a real shell `mode` concept only where the repo clearly needs one
- separates `MlSignal` from the conventional strategy selector and conventional workflow vocabulary
- preserves the current single-shell product rather than forking the app into multiple shells or windows
- keeps `Run Library` shared across conventional and `MlSignal` work
- keeps options expression as a contextual follow-up lane rather than forcing it into a top-level shell mode
- preserves the existing options host in [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- makes persisted restore and artifact reopen deterministic once shell mode exists
- avoids inventing a generic persisted `branch` primitive before the repo actually needs one

This is not an options-shell rewrite, not a tray/menu project, not a second navigation-framework pass, and not a reason to widen backend contracts or artifact formats solely to satisfy frontend taxonomy.

## Functionality Posture

This implementation must be additive and capability-preserving.

- Do not remove current `MlSignal` capabilities until the shell-mode replacement is fully in place.
- Do not remove the existing options-expression follow-up lane in [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue).
- Do not force options into the standard [DataWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DataWorkspacePanel.vue) or the standard optimize strategy selector during the objective pass.
- Do not add a top-level `options` shell mode during the objective pass.
- Do not add a generic persisted `branch` state key during the objective pass.
- Do not fork `Run Library` into mode-specific copies.
- Do not create a separate Tauri window for `MlSignal`.
- Do not widen backend commands, saved-artifact formats, or restore contracts unless a concrete frontend blocker proves it is necessary.
- Preserve conventional `Finalize` ownership of the existing post-meta ML-confidence / Stage 5 flow from [mlsignal_mode_separation_plan_2026-03-31.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md).

## Progress Ledger

- [x] Phase 0 complete: baseline shell/mode/options audit finished
- [x] Phase 1 complete: shell `mode` state model and vocabulary landed
- [x] Phase 2 complete: restore and persistence migration landed
- [x] Phase 3 complete: `MlSignal` shell entry and panel ownership landed
- [x] Phase 4 complete: history/artifact routing and options-lane preservation landed
- [x] Phase 5 complete: verification and evidence refresh landed
- [x] Objective checkpoint complete and ready for clean commit
- [x] Optional interpretive follow-up reviewed
- [x] Targeted verification complete
- [x] Final evidence and screenshots attached

## Implementation Outcome Target

At the end of the objective checkpoint:

- the shell should carry a real `mode` concept for:
  - `conventional`
  - `ml-signal`
- the shell should stop pretending `MlSignal` is only another optimize strategy
- the conventional strategy selector should no longer be responsible for entering `MlSignal`
- conventional workflow restore should reopen into conventional shell posture
- `MlSignal` artifacts should reopen into `MlSignal` shell posture
- `Run Library` should remain shared and mode-aware rather than duplicated
- the current options-expression follow-up lane should remain intact and truthful
- options should remain out of the top-level shell taxonomy in this pass
- no generic `branch` primitive should be added

## Checkpoint Strategy

### Checkpoint A — Objective Improvements

Land only the grounded, high-confidence changes first:

- baseline audit of shell/mode/options posture
- real `mode` key in the shell model
- mode-aware persistence and bootstrap restore
- `MlSignal` shell entry and panel routing
- removal of `MlSignal` from the conventional strategy selector after shell mode exists
- shared `Run Library` routing updates
- explicit preservation of the current options follow-up lane
- verification and evidence capture

Checkpoint A should be suitable for a clean commit on its own.

### Checkpoint B — Optional Interpretive Follow-Up

Only after Checkpoint A is stable:

- refine exact visible placement/weighting of the mode switch
- add explicit mode actions to `WorkflowQuickSwitch.vue`
- add explicit mode commands to the command palette
- refine the exact step labels inside `MlSignal` mode
- consider contextual options launch points beyond History detail

Checkpoint B should be easy to split, stash, or revert without threatening the shell/state-model baseline.

## Objective-First Execution Rule

Execute this work in the following order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Objective verification
8. Clean objective commit checkpoint
9. Optional interpretive follow-up

Do not mix speculative options-shell work into the objective `MlSignal` mode split.

## Source Basis

- [mode_separation_vs_workflow_branch_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt9/mode_separation_vs_workflow_branch_direction_FINAL.md)
- [mlsignal_mode_separation_plan_2026-03-31.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md)
- [options_expression_optimizer_viable_plan_2026-04-02.md](C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)
- [options_expression_future_scope_roadmap_2026-04-07.md](C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)
- [workflowModel.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/core/workflowModel.ts)
- [workflowStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowStore.ts)
- [workflowBootstrap.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/workflowBootstrap.ts)
- [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [WorkflowQuickSwitch.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/WorkflowQuickSwitch.vue)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [OptimizeWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue)
- [MlSignalWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue)
- [DataWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DataWorkspacePanel.vue)
- [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue)
- [optimizeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optimizeStore.ts)
- [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)
- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [OptionsExpressionPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [optionsExpressionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [REPO_MAP.md](C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)

## Execution Guardrails

- Keep one application shell.
- Keep `MlSignal` as the only new shell-mode concept in the objective pass.
- Keep options out of the top-level shell mode model in the objective pass.
- Keep `Run Library` shared.
- Keep the current History-hosted options-expression lane intact.
- Keep options-specific data handling separate from the standard `Data` shell until there is explicit evidence that unification is correct.
- Keep conventional `Finalize` in the conventional shell path.
- Preserve deterministic restore and artifact reopen behavior.
- Avoid adding another full product taxonomy axis beyond `mode`.
- Avoid artifact-format churn or backend-command churn unless a concrete blocker proves it is necessary.

## Explicit Working Decisions

### 1. Shell mode model

- Add a real shell `mode` concept.
- The objective pass supports exactly:
  - `conventional`
  - `ml-signal`
- Do not add `options` as a shell mode now.

### 2. Conventional workflow ownership

- Conventional remains the default shell mode.
- Conventional keeps the current broad workflow model:
  - `Data`
  - `Optimize`
  - `Validate`
  - `Analyze`
  - `Finalize`
- Conventional `Finalize` keeps ownership of the existing post-meta ML-confidence / Stage 5 path.

### 3. `MlSignal` ownership

- `MlSignal` stops being entered primarily through the conventional strategy selector.
- `MlSignal` gets its own shell-mode posture.
- `MlSignalWorkspacePanel.vue` remains the canonical ML working surface in the first slice.
- The repo may keep one ML panel in the first slice; decomposition into multiple ML-specific panels is not required for the objective pass.

### 4. `Run Library` and history ownership

- `Run Library` remains shared.
- Artifact reopen and rerun become mode-aware.
- Shared artifact browsing is preserved; no ML-only or options-only duplicate library is created in the objective pass.

### 5. Options ownership

- Options remains a contextual follow-up lane, not a top-level mode.
- The current canonical host remains [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue).
- The current options data path remains separate from the standard data shell in this pass.
- Do not move options into the standard optimize strategy selector in this pass.

### 6. Persistence model

- `workflowStore.ts` and `workflowBootstrap.ts` gain a real persisted `mode`.
- Legacy restore falls back to `conventional` unless legacy state or artifact context makes `ml-signal` unambiguous.
- Do not add a generic persisted `branch` key in this pass.

## Expected Edit Inventory

Primary expected edit targets:

- [workflowModel.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/core/workflowModel.ts)
- [workflowStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowStore.ts)
- [workflowBootstrap.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/workflowBootstrap.ts)
- `web/src/utils/core/storageKeys.ts`
- [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [OptimizeWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue)
- [MlSignalWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue)
- [optimizeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optimizeStore.ts)
- [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)

Likely review-only or narrow-touch targets:

- [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue)
- [DataWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DataWorkspacePanel.vue)
- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [OptionsExpressionPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [optionsExpressionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [WorkflowQuickSwitch.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/WorkflowQuickSwitch.vue)
- [CommandPaletteOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/CommandPaletteOverlay.vue)
- `web/src/stores/workflowArtifactsStore.ts`

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Goal: record the current shell/model posture before changing vocabulary or persistence.

Checklist:

- [x] Record the current shell vocabulary in [workflowModel.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/core/workflowModel.ts):
  - no `mode` key
  - one workflow vocabulary
  - one persisted shell model
- [x] Record current `workflowStore.ts` runtime/persistence behavior:
  - tab memory is keyed by workflow only
  - restore does not include `mode`
- [x] Record current `workflowBootstrap.ts` restore behavior and legacy fallback rules
- [x] Record current `MlSignal` special-casing in:
  - [optimizeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optimizeStore.ts)
  - [OptimizeWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue)
  - [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue)
- [x] Record current history-driven `MlSignal` restore/rerun behavior in [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)
- [x] Record current options host behavior:
  - [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue) hosts [OptionsExpressionPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
  - [optionsExpressionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts) treats options as a follow-up workflow
- [x] Confirm options is not currently a shell mode or optimize strategy
- [x] Capture before-state evidence for:
  - conventional workflow shell
  - `MlSignal` entered via strategy selector
  - History-hosted options follow-up

Acceptance criteria:

- [x] The current shell/state/options baseline is explicitly written down before implementation
- [x] The repo-backed distinction between `MlSignal` and options is documented
- [x] There is a stable before-state reference for shell, ML entry, and options host behavior

## Phase 1 — Shell Vocabulary And State Model

Goal: add a real shell `mode` concept without widening the shell taxonomy further.

Primary files:

- [workflowModel.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/core/workflowModel.ts)
- [workflowStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowStore.ts)
- `web/src/utils/core/storageKeys.ts`

Checklist:

- [x] Add a `ShellMode` type and normalization helpers
- [x] Add allowed values for:
  - `conventional`
  - `ml-signal`
- [x] Add mode-aware workflow-step maps or equivalent mode-aware step helpers
- [x] Add mode-aware subtitles or equivalent mode-aware subtitle derivation
- [x] Add mode-aware allowed-tab/default-tab resolution where current workflow-only rules are no longer sufficient
- [x] Add a real `mode` field to `workflowStore.ts`
- [x] Add `selectMode()` or equivalent mode-selection action
- [x] Namespace remembered tab posture by mode + workflow rather than workflow alone
- [x] Add a persisted storage key for mode
- [x] Keep current workspace vocabulary stable:
  - `workflow`
  - `library`
  - `aggregate`
  - `pro`
  - `settings`
- [x] Do not add a generic `branch` shell key

Acceptance criteria:

- [x] The shell model can represent conventional and `MlSignal` without relying on `strategyKind`
- [x] The current workspace model remains intact
- [x] Tab memory is no longer implicitly shared across conventional and `MlSignal`

## Phase 2 — Restore And Migration

Goal: make last-session restore and artifact reopen deterministic once mode exists.

Primary files:

- [workflowBootstrap.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/workflowBootstrap.ts)
- [workflowStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowStore.ts)
- `web/src/utils/core/storageKeys.ts`

Checklist:

- [x] Update bootstrap state shape to include `mode`
- [x] Validate restored mode against the allowed mode set
- [x] Migrate legacy persisted state safely:
  - trust explicit restored mode when present
  - infer `ml-signal` only when legacy strategy/artifact context makes it unambiguous
  - otherwise fall back to `conventional`
- [x] Preserve existing split-fraction and tab-fallback behavior where still valid
- [x] Ensure invalid or incomplete stored state cannot restore into an impossible shell posture
- [x] Keep old sessions backward-compatible instead of forcing manual resets

Acceptance criteria:

- [x] Restarting the app restores into a valid shell mode deterministically
- [x] Legacy sessions do not randomly switch into `MlSignal`
- [x] Mode migration does not break existing workspace/tab restore

## Phase 3 — `MlSignal` Shell Entry And Panel Ownership

Goal: stop entering `MlSignal` primarily through the conventional strategy selector.

Primary files:

- [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [OptimizeWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue)
- [MlSignalWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue)
- [optimizeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optimizeStore.ts)

Checklist:

- [x] Expose explicit shell entry into `MlSignal` mode
- [x] Keep the mode-entry mechanism truthful and minimal
- [x] Update [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue) so ML surfaces mount from shell mode rather than from `strategyKind === "MlSignal"` alone
- [x] Remove the embedded `MlSignalWorkspacePanel.vue` primary-entry responsibility from [OptimizeWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue)
- [x] Return [OptimizeWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue) to conventional optimize ownership
- [x] Keep [MlSignalWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue) fully operational as the canonical ML surface
- [x] Remove `MlSignal` from `STRATEGY_OPTIONS_DEFAULT` only after shell mode exists and ML panel routing is stable
- [x] Remove ML-specific run/validate entry dependence on the conventional strategy selector once shell mode owns that decision
- [x] Keep conventional users out of `MlSignal` by default

Acceptance criteria:

- [x] `MlSignal` no longer reads as just another optimize strategy
- [x] Conventional optimize UI is no longer responsible for being the main ML entry path
- [x] `MlSignal` capabilities remain intact after the shell split

## Phase 4 — Shared Library Routing And Options Preservation

Goal: make history/artifact restore mode-aware while preserving the already-real options follow-up lane.

Primary files:

- [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)
- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [optionsExpressionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue)
- [DataWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DataWorkspacePanel.vue)

Checklist:

- [x] Update history reopen logic so conventional artifacts route into conventional shell posture
- [x] Update history reopen logic so `MlSignal` artifacts route into `MlSignal` shell posture
- [x] Update history rerun logic so shell mode is selected before ML-specific hydration or rerun dispatch
- [x] Preserve current shared `Run Library` ownership
- [x] Confirm [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue) remains truthful for both conventional and `MlSignal` after shell mode exists
- [x] Confirm [DataWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DataWorkspacePanel.vue) remains the conventional / ML shared data surface only where repo-backed
- [x] Explicitly preserve the current options follow-up host in [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [x] Confirm no objective-pass edit moves options into:
  - a top-level shell mode
  - the standard optimize strategy selector
  - the standard shared data shell
- [x] Confirm options still opens from concrete saved-run context after the `MlSignal` mode work

Acceptance criteria:

- [x] Saved conventional artifacts reopen into conventional shell posture
- [x] Saved ML artifacts reopen into `MlSignal` shell posture
- [x] The options follow-up lane still works and remains context-hosted
- [x] No duplicate Run Library or options shell taxonomy is introduced

## Phase 5 — Verification And Evidence Refresh

Goal: verify the shell split across frontend, persistence, history, and desktop shell surfaces.

Checklist:

- [x] Run frontend verification:
  - `npm --prefix web run lint`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `npm --prefix web run a11y:smoke`
  - `npm --prefix web run baseline:ui`
- [x] Run desktop verification:
  - `npm --prefix web run test:desktop:smoke`
- [x] Verify conventional shell startup/restore
- [x] Verify `MlSignal` shell startup/restore
- [x] Verify conventional artifact reopen routes into conventional mode
- [x] Verify ML artifact reopen routes into `MlSignal` mode
- [x] Verify conventional optimize strategy selector no longer exposes `MlSignal`
- [x] Verify the shared `Run Library` remains intact
- [x] Verify the current options-expression History follow-up still opens and behaves correctly
- [x] Capture refreshed evidence under `scripts/baselines/ui_capture` for:
  - conventional workflow shell
  - `MlSignal` shell
  - conventional artifact reopen
  - ML artifact reopen
  - current History-hosted options follow-up remaining intact, with targeted component coverage used where the browser preview has no ready-state saved artifact

Acceptance criteria:

- [x] All required verification surfaces pass or any known failures are explicitly documented as unrelated
- [x] Restore and artifact reopen are deterministic across conventional and `MlSignal`
- [x] Screenshot evidence exists for the changed shell/posture surfaces

Verification notes:

- Browser evidence was captured in [prompt9_conventional_shell_manual.png](C:/Rust%20Projects/ta_desktop_demo/scripts/baselines/ui_capture/prompt9_conventional_shell_manual.png), [prompt9_ml_signal_shell_manual.png](C:/Rust%20Projects/ta_desktop_demo/scripts/baselines/ui_capture/prompt9_ml_signal_shell_manual.png), and [prompt9_run_library_manual.png](C:/Rust%20Projects/ta_desktop_demo/scripts/baselines/ui_capture/prompt9_run_library_manual.png).
- Shared History-hosted options follow-up preservation is asserted by [HistoryWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.test.ts) because the browser preview starts with an empty run library and no ready-state saved artifact to launch the options follow-up surface directly.
- Desktop smoke passed while an existing Vite preview instance was already occupying port `1420`; the suite attached successfully, executed all specs, and exited cleanly.

## Objective Verification Checklist

- [x] The shell has a real `mode` concept
- [x] Conventional remains the default mode
- [x] `MlSignal` is no longer primarily entered as a strategy option
- [x] Conventional `Finalize` remains conventional
- [x] `Run Library` remains shared
- [x] Options remains a contextual follow-up lane
- [x] No generic persisted `branch` primitive was added
- [x] No `options` top-level mode was added

## Targeted Test Additions Checklist

- [x] Add or update `workflowModel` tests for mode-aware normalization and allowed combinations
- [x] Add or update `workflowStore` tests for mode persistence and mode-scoped tab memory
- [x] Add or update `workflowBootstrap` tests for legacy migration and restore behavior
- [x] Add or update `OptimizeWorkspacePanel` / `MlSignalWorkspacePanel` / shell tests for explicit ML mode entry
- [x] Add or update tests proving `MlSignal` is removed from the conventional strategy selector after mode split
- [x] Add or update `historyService` tests for conventional vs ML artifact reopen/rerun routing
- [x] Add or update tests that preserve the current History-hosted options follow-up behavior
- [x] Add or update desktop smoke coverage if shell mode becomes a visible desktop-shell control

## Manual Review Checklist

- [x] Review the default conventional shell state
- [x] Review explicit entry into `MlSignal` mode
- [x] Review last-session restore for conventional mode
- [x] Review last-session restore for `MlSignal` mode
- [x] Review conventional artifact reopen from `Run Library`
- [x] Review ML artifact reopen from `Run Library`
- [x] Review current History-hosted options follow-up after the mode split, using targeted component coverage because the browser preview has no ready-state saved artifact
- [x] Review that no new shell clutter or speculative taxonomy appeared

## Guardrail Review Checklist

- [x] No `options` top-level shell mode landed
- [x] No generic persisted `branch` key landed
- [x] No duplicate Run Library surface landed
- [x] No second shell/window landed for `MlSignal`
- [x] No options migration into the standard strategy selector landed
- [x] No forced options migration into the standard shared data shell landed
- [x] No backend/artifact churn landed without narrow documented justification

## Defers And Rejects

- [x] Defer generic persisted `branch` abstraction until more than one real branch needs shell-level treatment
- [x] Defer `options` as a top-level shell mode
- [x] Defer separate Tauri windows for `MlSignal`
- [x] Defer options launch points from `Validate` or `Analyze` until a dedicated options UX pass proves them out
- [x] Reject keeping `MlSignal` permanently embedded as a conventional strategy
- [x] Reject forcing options into the standard optimize strategy selector
- [x] Reject forcing options into the standard shared data shell in this pass
- [x] Reject product taxonomy growth beyond grounded shell needs

## Optional Interpretive Follow-Up Checklist

- [ ] Decide the exact visible placement and weight of the shell mode control
- [ ] Add explicit mode actions to [WorkflowQuickSwitch.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/WorkflowQuickSwitch.vue)
- [ ] Add explicit mode commands to `CommandPaletteOverlay.vue`
- [ ] Refine the exact user-facing step labels inside `MlSignal` mode
- [ ] Decide whether [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue) needs stronger mode-specific CTA copy beyond the objective pass
- [ ] Decide whether options deserves contextual entry points from `Validate` or `Analyze` in addition to History detail
- [ ] Revisit whether any future options shell concept is needed only if the current History-hosted host proves insufficient

## Final Pass/Fail Criteria

Pass if:

- `MlSignal` is represented truthfully as a shell mode
- the state model and restore model are deterministic
- conventional users no longer enter ML through the standard strategy selector
- the shared `Run Library` remains intact
- the current options follow-up lane remains intact
- no speculative `options` mode or generic `branch` model leaks into the objective checkpoint

Fail if:

- shell restore becomes less predictable
- `MlSignal` still depends primarily on `strategyKind === "MlSignal"` hacks after the split
- options behavior regresses because of the ML shell work
- the implementation adds taxonomy faster than it adds truth
- the implementation widens into an options-shell rewrite instead of the grounded `MlSignal` mode split
