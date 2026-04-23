# History Artifact Workflow Implementation Tracking

Created: 2026-04-05
Prompt source: `Frontend Review/Prompt5/history_artifact_workflow_direction_FINAL.md`

## Purpose

This document tracks the implementation of the grounded Prompt 5 direction for frontend run-artifact, history, and aggregate-artifact UX.

The implementation target is:

- tighten and standardize History as the canonical Run Library for saved run artifacts
- keep Session History available, but visually and behaviorally secondary to Saved Runs
- keep Meta as the contextual aggregate-artifact surface rather than merging it into History
- preserve every backend-backed artifact capability path
- standardize shared artifact-browsing behavior where overlap is real
- bring destructive-action safety in Meta up to the same explicit-confirmation standard already used in History

This is a frontend UX tightening and consolidation pass, not a shell-IA rewrite and not a backend artifact-contract rewrite.

## Functionality Posture

This implementation is additive and functionality-preserving.

Must remain true throughout the work:

- no run-artifact capability loses a clear frontend path
- `Load`, `Apply`, and `Rerun` remain distinct run-artifact actions
- local Saved Runs search remains local to History and stays subordinate to `Ctrl+K`
- `workflowArtifactsStore.ts` remains the canonical active run/validation handoff layer
- Meta continues to own contextual aggregate-artifact browsing for Meta / Stage 5 / policy-eval families
- destructive actions remain explicit and safe
- keyboard-safe selection and focus return behavior remain intact

This pass must not:

- create a new top-level Artifact Center
- merge Meta fully into History
- flatten run-artifact actions into a generic `Open`
- remove report, reveal, delete, tag-editing, rerun, or runs-folder reachability
- introduce a giant generic cross-family artifact store that hides meaningful run-vs-aggregate differences

## Progress Ledger

- [x] Phase 0 complete: baseline audit and evidence capture finished
- [x] Phase 1 complete: capability and ownership audit landed
- [x] Phase 2 complete: History Run Library tightening landed
- [x] Phase 3 complete: Meta destructive-safety parity landed
- [x] Phase 4 complete: shared artifact-browser contract groundwork landed
- [x] Phase 5 complete: Meta saved-artifact consolidation landed
- [x] Objective checkpoint complete and ready for clean commit
- [ ] Optional interpretive follow-up reviewed
- [x] Targeted verification complete
- [x] Final evidence and screenshots attached

## Implementation Notes (2026-04-05)

Objective Prompt 5 work landed in:

- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [MetaWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.test.ts)

What shipped:

- History now reads more clearly as the Run Library through stronger Saved Runs weighting and lighter Session History posture.
- History’s selected-run detail plane and the new options-expression surface were preserved.
- Meta saved artifacts now use one family-switch-driven browser instead of three repeated management blocks.
- Meta delete actions now route through the shared confirmation dialog instead of executing directly from the overflow menu.
- Family-specific load/report/reveal/delete semantics remain intact.

Verification notes:

- automated verification is green: targeted component tests, full frontend suite, lint, typecheck, build, a11y smoke, baseline UI capture, and bundle-budget
- generic baseline capture covers the overall Library and Aggregate workspaces
- the saved-family switch and delete-confirmation specifics are primarily evidenced through targeted component tests because the generic static baseline harness does not populate saved aggregate artifacts under Tauri mocks

## Implementation Outcome Target

After the objective checkpoint:

- History clearly reads as the canonical Run Library
- Saved Runs is the primary managed-artifact surface within History
- Session History remains available and useful, but no longer competes equally with Saved Runs
- Meta still owns aggregate-artifact review, but its saved-artifact UX no longer feels like three repeated mini-management forms
- History and Meta share a more consistent list/select/detail/action contract where they overlap
- delete feels equally explicit and trustworthy in History and Meta
- run-artifact detail remains the decision surface for inspecting artifacts before acting on them
- current workflow artifact handoff behavior remains stable

## Checkpoint Strategy

### Checkpoint A — Objective Improvements

Complete these first and checkpoint them together:

- baseline audit and evidence capture
- capability and ownership audit
- History Run Library tightening
- Meta delete-confirmation parity
- shared artifact-browser contract groundwork
- Meta saved-artifact consolidation
- objective verification

Checkpoint A should produce a clean, independently useful improvement set even if no optional follow-up work is ever landed.

### Checkpoint B — Optional Interpretive Follow-Up

Only after Checkpoint A is stable:

- refine exact action weighting between visible actions, overflow actions, and page utilities
- decide whether destructive dialogs should disable backdrop dismissal
- decide whether History needs sticky selected-run summary behavior
- decide whether to add compact trust/status chips
- decide the exact degree of shared component extraction versus keeping the pattern mostly contractual

Checkpoint B should be easy to split, stash, or revert without threatening artifact reachability.

## Objective-First Execution Rule

Implement in this order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Objective verification
8. Clean objective commit checkpoint
9. Optional interpretive follow-up

If a Phase 4 or Phase 5 change starts forcing speculative shared abstractions, stop and reduce the extraction level until the objective requirements are still met.

## Source Basis

This tracker is grounded in the following repo evidence:

- [history_artifact_workflow_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt5/history_artifact_workflow_direction_FINAL.md)
- [frontend_target_comps_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_target_comps_2026-03-28.md)
- [frontend_design_system_spec_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_design_system_spec_2026-03-28.md)
- [frontend_functional_coverage_matrix_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_functional_coverage_matrix_2026-03-28.md)
- [REPO_MAP.md](C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)
- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [ConfirmationDialog.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ConfirmationDialog.vue)
- [ContextActionMenu.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ContextActionMenu.vue)
- [historyStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/historyStore.ts)
- [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)
- [workflowArtifactsStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowArtifactsStore.ts)
- [metaService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.ts)
- [analysisStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.ts)

## Execution Guardrails

- Keep Prompt 4 results-surface ownership unchanged. History and Meta must not become replacements for current-run results triage.
- Keep backend contracts fixed unless a narrow frontend bug requires a matching store/service adapter change.
- Preserve every existing artifact family supported today:
  - run artifacts
  - Meta artifacts
  - Stage 5 artifacts
  - policy-eval artifacts
- Prefer shared behavior contracts over deep genericization.
- Treat delete as the highest-stakes action in this scope.
- Preserve existing report/reveal/open-folder affordances even if their weighting changes later.
- Preserve keyboard-safe list selection and deterministic focus return after overlay close.

## Explicit Working Decisions

### 1. Ownership boundary

- History remains the canonical frontend surface for run-artifact management.
- Meta remains the contextual surface for aggregate-artifact browsing and follow-up actions.
- No new shell-level artifact workspace is introduced in this pass.

### 2. History role

- History should read library-first, not activity-log-first.
- Saved Runs is the primary artifact-management surface.
- Session History remains visible and useful, but secondary.
- The selected-run detail region remains the canonical operator decision surface before acting.

### 3. Meta role

- Meta keeps contextual ownership of saved Meta / Stage 5 / policy-eval artifacts.
- Meta should stop behaving like three separate repeated saved-artifact management forms.
- Meta should converge toward a shared browse/select/detail/action pattern with family-specific adapters.

### 4. Action semantics

- `Load`, `Apply`, and `Rerun` remain separate visible run-artifact actions.
- Report-open and reveal actions remain clearly reachable.
- Tag editing remains selected-item-scoped.
- Runs-folder access remains clearly reachable at the workspace level.

### 5. Destructive-safety model

- Clear Session and delete actions remain explicit.
- History keeps its shared confirmation-dialog flow.
- Meta must gain equivalent explicit confirmation for delete actions.
- Focus return after confirmation close must stay deterministic.

### 6. Workflow artifact handoff

- `workflowArtifactsStore.ts` remains the canonical active-artifact handoff layer.
- History load/apply/rerun flows must continue to work with existing store/service semantics.
- No frontend artifact UX change should break current run/validation artifact hydration.

### 7. Search and detail model

- Saved Runs search remains local, Fuse-backed, and list-scoped.
- Rich detail remains selected-item-scoped rather than row-inline.
- History detail vocabulary remains the strongest baseline for future aggregate-artifact detail depth.

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Goal:
Record the current behavior and evidence before any tightening or consolidation work starts.

Checklist:

- [x] Capture current screenshots for History default state, History selected-run state, and History delete confirmation
- [x] Capture current screenshots for Meta Saved Meta, Saved Stage 5, and Saved Policy Evaluation states
- [x] Record the current visible action set for selected run artifacts in History
- [x] Record the current delete path in History and the current delete path in Meta
- [x] Record the current local-search, tags, and selected-detail behavior for Saved Runs
- [x] Record current Meta saved-artifact repetition across the three saved families
- [x] Record current focus-return behavior for History confirmation close
- [x] Record current report/reveal/open-folder affordance locations

Acceptance criteria:

- there is a stable before-state reference for every surface touched in later phases
- the current capability map is documented before any visual weighting changes are made

## Phase 1 — Capability And Ownership Audit

Goal:
Lock the exact frontend ownership and capability-preservation boundaries before UI edits start.

Primary files:

- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [historyStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/historyStore.ts)
- [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)
- [workflowArtifactsStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowArtifactsStore.ts)
- [metaService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.ts)
- [analysisStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.ts)

Checklist:

- [x] Audit which artifact actions are canonical in History versus contextual in Meta
- [x] Audit the exact selected-run detail fields already backed by `historyStore.ts`
- [x] Audit which aggregate-artifact actions and metadata are family-specific in Meta
- [x] Audit how History load/apply/rerun flows touch `workflowArtifactsStore.ts`
- [x] Audit whether any current report/reveal/delete action is only reachable through one surface
- [x] Audit where confirmation behavior currently exists and where it is missing
- [x] Confirm no Prompt 4 results ownership is accidentally being moved into History or Meta
- [x] Avoid backend contract changes during this phase

Acceptance criteria:

- ownership boundaries are explicit before any consolidation work begins
- the implementation team knows exactly which capabilities must remain reachable in History and in Meta

## Phase 2 — History Run Library Tightening

Goal:
Make History read more clearly as the canonical Run Library while preserving all current run-artifact capabilities.

Primary files:

- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [historyStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/historyStore.ts)

Checklist:

- [x] Preserve History’s library-first framing and strengthen it only where wording or weighting is still ambiguous
- [x] Preserve Saved Runs as the primary managed-artifact pane
- [x] Preserve Session History as a secondary transient recall pane
- [x] Preserve local Saved Runs search and tags editing
- [x] Preserve selected-run detail as the operator decision surface before acting
- [x] Preserve visible `Load`, `Apply`, and `Rerun` semantics
- [x] Preserve report-open, reveal, delete, and runs-folder reachability
- [x] Preserve current request-snapshot troubleshooting disclosure
- [x] Preserve keyboard-safe list selection and selected-item detail sync
- [x] Avoid turning History into a generic file-manager surface

Acceptance criteria:

- History unmistakably reads as the primary run-artifact management surface
- Session History remains available without competing equally with Saved Runs
- no run-artifact action becomes hidden or semantically flattened

## Phase 3 — Meta Destructive-Safety Parity

Goal:
Bring Meta delete behavior up to the same explicit-confirmation standard already used in History.

Primary files:

- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [ConfirmationDialog.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ConfirmationDialog.vue)
- [analysisStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.ts)
- [metaService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.ts)

Checklist:

- [x] Add explicit confirmation before deleting saved Meta artifacts
- [x] Add explicit confirmation before deleting saved Stage 5 artifacts
- [x] Add explicit confirmation before deleting saved policy-eval artifacts
- [x] Show enough artifact identity in the confirmation body to avoid ambiguous deletion
- [x] Preserve clear cancel and confirm paths
- [x] Preserve deterministic focus return after dialog close
- [x] Preserve current non-delete secondary actions such as refresh, report, and reveal
- [x] Avoid introducing direct destructive execution from menu selection

Acceptance criteria:

- History and Meta now share the same high-level delete-safety posture
- delete is never a one-click action from Meta overflow menus

## Phase 4 — Shared Artifact-Browser Contract Groundwork

Goal:
Standardize the overlapping browse/select/detail/action behavior without prematurely over-generalizing the artifact architecture.

Primary files:

- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [ContextActionMenu.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ContextActionMenu.vue)
- new `web/src/components/artifacts/*` only if reuse is clearly proven
- small helper/composable modules only if they are reused across at least two artifact families

Checklist:

- [x] Define the shared behavior contract for artifact list selection
- [x] Define the shared behavior contract for selected-item detail rendering
- [x] Define the shared behavior contract for selected-item secondary actions
- [x] Define the shared behavior contract for tag-editing where artifact families support tags
- [x] Define the shared confirmation-flow contract for destructive actions
- [x] Reuse History’s stronger detail model as the baseline where overlap exists
- [x] Keep family-specific stores and backend semantics separate
- [x] Avoid introducing a giant generic artifact store
- [x] Avoid introducing a shell-level Artifact Center

Acceptance criteria:

- History and Meta can converge on the same interaction model without losing family-specific semantics
- shared code, if any, stays narrow and behavior-focused

## Phase 5 — Meta Saved-Artifact Consolidation

Goal:
Reduce the repeated Saved Meta / Saved Stage 5 / Saved Policy Evaluation management pattern while keeping Meta contextual.

Primary files:

- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [analysisStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.ts)
- [metaService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.ts)

Checklist:

- [x] Replace repeated saved-artifact management structure with one clearer reused browse/select/detail/action pattern
- [x] Preserve separate support for Saved Meta, Saved Stage 5, and Saved Policy Evaluation families
- [x] Preserve family-specific load/report/reveal/delete behavior where it differs
- [x] Preserve tags editing for saved aggregate artifacts
- [x] Preserve contextual aggregate ownership inside Meta
- [x] Keep Meta from reading like a second canonical Run Library
- [x] Keep the resulting structure keyboard-safe and selection-first
- [x] Avoid reducing Meta to a raw select-and-act utility strip

Acceptance criteria:

- Meta no longer feels like three repeated mini-management forms
- aggregate-artifact behavior is more consistent without losing contextual ownership

## Objective Verification Checklist

Run after completing the objective phases:

- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run build`
- [x] `npm --prefix web run a11y:smoke`
- [x] `npm --prefix web run baseline:ui`
- [x] `npm --prefix web run check:bundle-budget` if shared helpers or startup-facing code weight changes materially

Manual surface review:

- [x] History default empty/non-empty states
- [x] History Saved Runs search and tag editing
- [x] History selected-run detail hydration
- [x] History `Load`
- [x] History `Apply`
- [x] History `Rerun`
- [x] History `Open report`
- [x] History `Reveal`
- [x] History delete confirmation
- [x] History clear-session confirmation
- [x] History runs-folder utility
- [x] Meta Saved Meta load/report/reveal/delete flow
- [x] Meta Saved Stage 5 load/reveal/delete flow
- [x] Meta Saved Policy Evaluation load/reveal/delete flow
- [x] Light theme review
- [x] Dark theme review
- [x] Standard density review
- [x] Dense mode review
- [x] Reduced-motion review
- [x] Keyboard traversal through list selection, action menus, and confirmation close

Guardrail review:

- [x] Confirm `Load`, `Apply`, and `Rerun` remain distinct in History
- [x] Confirm local Saved Runs search remains local and `Ctrl+K` remains global
- [x] Confirm `workflowArtifactsStore.ts` handoff behavior still works after History load/apply/rerun flows
- [x] Confirm no artifact family lost report/reveal/delete reachability
- [x] Confirm Meta remains contextual rather than becoming a second run-library command center

## Targeted Test Additions

Store/service coverage:

- [x] `historyService` tests for `Load` / `Apply` / `Rerun` semantic separation
- [x] `historyService` tests for report/reveal/delete error handling
- [x] `historyStore` tests for search determinism and selection/detail synchronization
- [x] Meta service/store coverage for delete-confirmation-triggered flows if that control path changes materially

Component coverage:

- [x] `HistoryWorkspacePanel.test.ts` coverage for selected-item action enablement and confirmation flow
- [x] `HistoryWorkspacePanel.test.ts` coverage for keyboard selection and focus return after confirmation
- [x] `MetaWorkspacePanel.test.ts` coverage for delete confirmation parity
- [x] `MetaWorkspacePanel.test.ts` coverage for whichever consolidated saved-artifact selection model is adopted
- [ ] `ConfirmationDialog.test.ts` coverage if destructive-dialog behavior changes
- [x] `ContextActionMenu.test.ts` coverage if action-menu keyboard or focus behavior changes

## UX Pass/Fail Criteria

Pass if:

- operators can quickly tell that History is the canonical run-artifact library
- Saved Runs feels primary without making Session History unusable
- Meta artifact browsing feels more intentional and less repetitive
- delete feels explicit and trustworthy everywhere
- the selected detail surface remains strong enough to support trust before acting
- no artifact capability path becomes harder to discover than before

Fail if:

- History loses clear `Load` / `Apply` / `Rerun` semantics
- Meta delete remains effectively one click from overflow
- a new shared abstraction makes aggregate and run artifacts feel artificially identical
- operators must bounce between History and Meta just to complete one artifact task
- library cleanup makes report/reveal/runs-folder affordances harder to find

## Explicit Defers And Rejects

Defer:

- a top-level Artifact Center
- shell-level IA changes for artifact management
- richer stale-context or provenance diffing beyond current artifact metadata
- deep generic artifact-store refactors before the shared behavior contract proves stable
- broad backend artifact-schema unification

Reject:

- merging Meta fully into History
- flattening run-artifact actions into a generic `Open`
- direct delete from Meta action menus without explicit confirmation
- removing local library search in favor of another pseudo-global search surface
- overloading rows with large inline action clusters
- replacing detail cards with raw JSON/debug dumps as the primary review model

## Optional Interpretive Follow-Up Checklist

- [ ] Decide the final visible-versus-secondary weighting for `Refresh`, `Reveal`, `Delete`, and `Runs folder` inside History
- [ ] Decide whether destructive dialogs in this workflow should disable backdrop-click dismissal
- [ ] Decide whether Meta should use a family switch, segmented control, or equivalent compact selector for saved-artifact family selection
- [ ] Decide whether History should gain a sticky selected-run summary for very long detail views
- [ ] Decide whether compact trust/status chips should be added when fully supported by deterministic artifact metadata
- [ ] Decide the exact degree of component extraction versus keeping the shared artifact-browser pattern mostly contractual
- [ ] Decide whether further copy/visual demotion of Session History is still needed after the objective pass

## Expected Edit Inventory

Likely touched files:

- [HistoryWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [ConfirmationDialog.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ConfirmationDialog.vue)
- [ContextActionMenu.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ContextActionMenu.vue)
- [historyStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/historyStore.ts)
- [historyService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.ts)
- [analysisStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.ts)
- [metaService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/metaService.ts)

Potential new files only if reuse is clearly justified:

- `web/src/components/artifacts/*`
- a small artifact view-model helper or composable

Files expected to remain behaviorally unchanged unless a narrow regression fix requires touch:

- [workflowArtifactsStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/workflowArtifactsStore.ts)
- [RunFooterPanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue)
