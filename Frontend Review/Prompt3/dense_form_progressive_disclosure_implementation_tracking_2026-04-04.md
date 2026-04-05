# Dense Form Progressive Disclosure Implementation Tracking

Date: 2026-04-04

Scope: implementation tracker for the grounded Prompt 3 form-architecture direction in `Frontend Review/Prompt3/dense_form_progressive_disclosure_direction_FINAL.md`.

Status: planned

Owner posture: frontend form-architecture refinement only. Do not broaden into backend contract changes, shell information-architecture redesign, product-mode decisions, or a schema-owned form-state migration.

## Purpose

This document turns the grounded Prompt 3 conclusions into an execution tracker with:

- explicit phase ordering
- objective-first checkpointing
- concrete file ownership
- implementation checklist items
- capability-preservation guardrails
- verification gates and evidence requirements
- optional end-of-pass items for the more interpretive recommendations

This tracker is for executing the valid Prompt 3 direction:

- keep dense authoring and configuration surfaces as single-page expert workstations
- preserve the current store-owned + queued-input-write model
- stay inside the already-landed Prompt 2 visual/disclosure baseline
- reorganize existing controls and support copy rather than introducing a new local styling system
- keep Data load/inspect/timeframe posture obvious and keep deeper directory management canonical in Settings
- tighten the Strategy shared cost/risk/exit spine without hiding real tuning
- consolidate Advanced rather than redesign it
- fix Validation's current sequencing problem so important modules remain reachable before the first run
- preserve backend-backed capability reachability on every surface

## Functionality Posture

This plan is additive and restructuring-oriented in nature.

It is intended to:

- improve scanability of dense panels
- make important controls and active non-default state easier to read
- reduce repeated support text and low-value scan noise
- preserve expert speed while using shallower disclosure
- improve sequencing between setup work and review work

It is not intended to:

- remove backend-backed controls as a cleanliness shortcut
- split the panels into wizard steps
- introduce a schema/form-library rewrite
- replace the current queued-write ownership model
- relitigate Prompt 1 shell hierarchy or Prompt 2 visual-system decisions
- hide canonical Data, Strategy, Advanced, or Validation actions behind menus or secondary surfaces

If an implementation choice would trade away capability reachability for a cleaner-looking panel, that choice is out of scope for this plan.

## Progress Ledger

- [ ] Phase 0 complete: baseline audit and evidence capture finished
- [ ] Phase 1 complete: shared dense-form foundation and helper audit landed
- [ ] Phase 2 complete: objective Validation sequencing changes landed
- [ ] Phase 3 complete: objective Strategy spine tightening landed
- [ ] Phase 4 complete: objective Data posture cleanup landed
- [ ] Phase 5 complete: objective Advanced consolidation landed
- [ ] Objective checkpoint complete and ready for clean commit
- [ ] Optional interpretive follow-up reviewed
- [ ] Targeted verification complete
- [ ] Final evidence and screenshots attached

## Implementation Outcome Target

Target surfaces:

- `web/src/styles/app.css`
  - only minimal shared dense-form utility refinements
  - no second token system
  - no panel-local disclosure grammar
- `web/src/components/ValidationWorkspacePanel.vue`
  - pre-run modules and automation become reachable before results exist
  - setup-time versus result-time emphasis becomes clearer
  - Prompt 2 table/review treatment remains intact
- `web/src/components/StrategyWorkspacePanel.vue`
  - shared cost/risk/exit spine remains obvious
  - common controls stay visible
  - low-frequency support/explanation is consolidated rather than repeated
- `web/src/components/DataWorkspacePanel.vue`
  - load / inspect / timeframe flow remains obvious
  - schema/provenance remains visible
  - storage-settings handoff remains explicit
- `web/src/components/AdvancedWorkspacePanel.vue`
  - ranking, output, and sampler groups read more clearly
  - guidance is consolidated
  - active non-default state is easier to read without reopening multiple details
- `web/src/stores/optimizeStore.ts`
  - only narrow derived-summary/support helpers if needed
  - no ownership rewrite
- `web/src/stores/validationStore.ts`
  - only narrow sequencing/support helpers if needed
  - no ownership rewrite
- `web/src/composables/useQueuedInputWriter.ts`
  - baseline ownership stays intact
  - only narrow support changes if implementation proves they are needed

## Checkpoint Strategy

This tracker is intentionally split into two implementation checkpoints.

### Checkpoint A — Objective Improvements

This checkpoint should contain only changes that are structurally useful and objectively testable.

Examples:

- keep all four panels single-page
- preserve the current queued-write model
- make Validation modules reachable before first run
- preserve the canonical Data versus Settings boundary for market-data folder management
- keep shared Strategy cost/risk/exit controls visible
- consolidate repeated Advanced guidance without losing reachability
- preserve keyboard paths and backend-backed capability coverage

Commit goal:

- produce a clean git commit after these changes land and pass verification
- this is the baseline form-architecture improvement commit that should remain even if later interpretive refinements are reverted

### Checkpoint B — Optional Interpretive Follow-Up

This checkpoint should contain only changes that are plausible but more design-judgment-heavy than structurally necessary.

Examples:

- the exact Strategy cutoff between always-visible constraints and one `More constraints` disclosure
- whether result-time export emphasis should move fully into the Validation results plane
- whether Advanced should act as a pattern pilot before broader rollout
- whether the product should enforce a hard disclosure-depth budget such as "one click down" everywhere
- whether an additional summary-chip variant is worth introducing for active non-default advanced state

Workflow goal:

- make these changes only after Checkpoint A is safely committed
- keep them isolated enough that they can be stashed, split, or reverted if they harm speed or clarity

## Objective-First Execution Rule

Execution order for this tracker is locked:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Objective verification
8. Clean objective commit checkpoint
9. Optional interpretive follow-up

Implementation rule:

- do not begin optional interpretive work until the Objective Checkpoint is complete
- treat Phase 1 through Phase 5 as the default must-land work
- treat the optional section at the end of this tracker as deliberately deferrable

## Source Basis

Primary source:

- `Frontend Review/Prompt3/dense_form_progressive_disclosure_direction_FINAL.md`

Supporting repo policy/context:

- `Frontend Review/Unfinished/frontend_followup_evaluation_2026-03-28.md`
- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/Prompt1/shell_hierarchy_workstation_implementation_tracking_2026-04-03.md`
- `Frontend Review/Prompt2/visual_system_depth_motion_implementation_tracking_2026-04-04.md`

Primary implementation files:

- `web/src/styles/app.css`
- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/composables/useQueuedInputWriter.ts`

## Execution Guardrails

- Treat Prompt 1 shell hierarchy as implemented baseline. Do not reopen shell IA in this pass.
- Treat Prompt 2 visual-system and disclosure language as implemented baseline. Do not add panel-local tab, card, or disclosure systems.
- Keep all four surfaces as single-page expert workstations.
- Preserve at least one clear frontend path for every backend-backed capability touched by regrouping.
- Do not broaden this pass into a schema-driven or library-driven form-state rewrite.
- Do not replace the current queued-input-write model with blur-only commit semantics or local component form stores.
- Do not hide Data provenance, schema, cached dataset selection, or timeframe controls just to make the page quieter.
- Do not force Validation into a Top-style sticky-selection interaction model.
- Do not move canonical market-data-folder management out of Settings.
- Do not move canonical validation exports out of Validation.
- Keep keyboard-safe traversal and disclosure focus behavior intact across density modes.

## Explicit Working Decisions

These are the working decisions for implementation in this pass. Treat them as locked defaults unless the product owner explicitly overrides them.

### 1. Form ownership model

Decision:

- keep the store-owned + queued-input-write model
- allow only narrow support helpers or derived summaries during this pass

Implementation meaning:

- `useQueuedInputWriter.ts` remains the architectural baseline
- `optimizeStore.ts` and `validationStore.ts` may gain derived support helpers, but not a new ownership model

### 2. Panel model

Decision:

- keep Data, Strategy, Advanced, and Validation as single-page expert workstations
- improve grouping and sequencing inside each panel instead of splitting work into wizard steps

Implementation meaning:

- no step-by-step flow conversion
- no hidden sub-pages
- no dependency on a future schema migration

### 3. Shared visual baseline

Decision:

- Prompt 2 visual and disclosure rules remain fixed baseline
- any new support-region or summary-state utility must be shared and minimal

Implementation meaning:

- prefer reusing existing `panelSummaryChip`, `panelDetails`, support-region, and surface classes
- if `app.css` needs additions, keep them small and reusable

### 4. Data surface boundary

Decision:

- keep Data focused on loading, cached dataset selection, provenance, and timeframe
- keep deeper market-data-folder management canonical in Settings

Implementation meaning:

- Data keeps current-folder visibility and open-folder utility
- Settings remains the place for set/reset/move directory management

### 5. Strategy structure

Decision:

- keep shared cost, risk/sizing, and key constraints/exits near the top of the panel
- preserve shared controls before strategy-specific parameter families

Implementation meaning:

- the current Strategy spine is directionally right and should be tightened, not replaced
- long-tail explanation and low-frequency support text can move behind disclosure
- common controls must remain directly reachable

### 6. Advanced structure

Decision:

- treat Advanced as a consolidation pass, not a redesign
- preserve clear grouping around ranking, output, and sampler state

Implementation meaning:

- repeated guidance should be consolidated
- important route/output controls stay directly reachable
- active non-default state should be legible without digging through multiple details blocks

### 7. Validation sequencing

Decision:

- make modules and automation reachable before first run
- preserve the current results summary/table as the primary post-run review plane
- do not force sticky-selection workflow into Validation

Implementation meaning:

- the objective defect is sequencing, not table styling
- setup-time configuration should not depend on `showValidationResults`

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Purpose:

- lock the current panel state before structural changes begin
- identify which controls, actions, and exports are truly canonical and cannot be demoted accidentally

Checklist:

- [ ] Capture current screenshots for Data, Strategy, Advanced, and Validation in light theme
- [ ] Capture at least one Dense-mode screenshot for each touched surface
- [ ] Record which controls are currently always visible on each panel
- [ ] Record which support/disclosure patterns already exist on each panel
- [ ] Record current export/action placement in Validation
- [ ] Record current Data versus Settings directory-management ownership
- [ ] Record current Strategy and Advanced summary-chip coverage

Acceptance criteria:

- a before-state exists for each touched panel
- current canonical actions and exports are explicitly listed before regrouping begins

## Phase 1 — Shared Foundation And Helper Audit

Purpose:

- make only the minimum shared changes needed to support the panel work
- avoid solving panel problems with panel-local styling drift

Primary files:

- `web/src/styles/app.css`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/composables/useQueuedInputWriter.ts`

Checklist:

- [ ] Audit whether existing Prompt 2 disclosure styles are sufficient for the new panel regrouping
- [ ] Add only minimal shared support-region spacing/border utilities if the current classes are insufficient
- [ ] Add only minimal summary-state helpers if panels need clearer active non-default echoes
- [ ] Keep all additions reusable across at least two panels
- [ ] Avoid any new token family or panel-only visual grammar
- [ ] Avoid changing queued-write ownership or debounce behavior unless a concrete bug blocks the panel work

Acceptance criteria:

- shared CSS remains the only styling foundation
- no panel introduces a one-off disclosure/card system
- any store/composable changes are narrow support changes rather than architecture rewrites

## Phase 2 — Validation Objective Sequencing Fix

Purpose:

- fix the clearest grounded structural problem first
- make pre-run validation setup complete before results exist

Primary file:

- `web/src/components/ValidationWorkspacePanel.vue`

Checklist:

- [ ] Keep the current setup pane visibly canonical for running validation
- [ ] Move or duplicate modules/automation controls so they are reachable before the first run
- [ ] Keep cross-dataset selection reachable before execution
- [ ] Preserve the results summary/table as the dominant post-run review plane
- [ ] Preserve the current Prompt 2 summary/table styling and column-details treatment
- [ ] Preserve canonical validation exports and reports-folder access on this surface
- [ ] Keep debug/output preview subordinate to the operational setup and results planes
- [ ] Preserve current queued-write behavior for validation inputs

Acceptance criteria:

- a user can configure modules, automation, and cross-dataset choices before the first run
- validation run/export/report actions remain clearly reachable
- post-run results still read as the dominant review plane
- no sticky row-selection model is introduced

## Phase 3 — Strategy Shared-Spine Tightening

Purpose:

- keep the Strategy panel's current shared-spine logic, but reduce scan noise and repeated support burden

Primary file:

- `web/src/components/StrategyWorkspacePanel.vue`

Checklist:

- [ ] Preserve visible summary chips at the top of the panel
- [ ] Preserve visible cost preset and core execution-cost inputs
- [ ] Preserve visible risk/sizing essentials such as equity, stop-loss, take-profit, and signal timing controls
- [ ] Audit repeated explanatory copy and move only the long-tail guidance behind shared disclosure where appropriate
- [ ] Keep frequently used constraints/exits directly reachable
- [ ] Preserve shared controls before strategy-specific parameter families
- [ ] Preserve execution-risk notice behavior
- [ ] Avoid hiding real backend-backed tuning behind collapsed regions without a visible active-state echo

Acceptance criteria:

- the panel still reads as cost -> risk/sizing -> constraints/exits -> strategy families
- common controls remain directly reachable
- the amount of repeated paragraph-style support text is reduced
- no backend-backed control loses a clear path

## Phase 4 — Data Posture Cleanup

Purpose:

- keep Data simple and operator-facing while clarifying what belongs here versus in Settings

Primary file:

- `web/src/components/DataWorkspacePanel.vue`

Checklist:

- [ ] Preserve visible CSV import and recent/sample dataset entry points
- [ ] Preserve visible Polygon download/cache surface
- [ ] Preserve visible cached dataset selection and keyboard-safe list behavior
- [ ] Preserve visible dataset context and timeframe controls
- [ ] Preserve visible schema/columns and provenance context
- [ ] Keep current-folder visibility and open-folder utility on the page
- [ ] Keep Manage-in-Settings handoff explicit for deeper market-data-folder changes
- [ ] Reduce any support-copy weight that competes with the loading/provenance/timeframe flow

Acceptance criteria:

- the panel still reads as load data -> inspect loaded context -> choose timeframe
- Data and Settings ownership of storage controls stays unambiguous
- schema/provenance remains readable rather than demoted into hidden support content

## Phase 5 — Advanced Consolidation

Purpose:

- reduce repeated guidance and make active non-default state easier to scan without redesigning the panel

Primary file:

- `web/src/components/AdvancedWorkspacePanel.vue`

Checklist:

- [ ] Preserve visible summary chips at the top of the panel
- [ ] Preserve clear grouping around ranking, output, and sampler state
- [ ] Keep objective, route/mode, top K, heatmap, include-all, export path, and sampler reset directly reachable when supported
- [ ] Consolidate repeated inline guidance where multiple detail blocks are communicating the same decision boundary
- [ ] Add clearer active non-default state echoes only if they materially reduce re-opening details
- [ ] Preserve capability summaries for locked or route-managed settings
- [ ] Avoid turning Advanced into a wizard, sub-tabbed surface, or panel-local mini-product

Acceptance criteria:

- the panel reads more clearly as ranking -> output -> sampler state
- route/output controls remain directly reachable
- repeated explanation is reduced without losing essential guidance

## Objective Verification Checklist

Required repo checks:

- [ ] `npm --prefix web run lint`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run test`
- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run a11y:smoke`
- [ ] `npm --prefix web run baseline:ui`
- [ ] `npm --prefix web run check:bundle-budget` if a phase touches shared helpers or increases startup-facing code weight

Required manual review:

- [ ] Data: CSV load, Polygon open/close, cached dataset selection, load selected, validate selected, repair selected, timeframe apply
- [ ] Strategy: one simple strategy family and one heavier family, plus non-default constraints/exits
- [ ] Advanced: default route, exact route, and a case with active non-default shortlist diagnostics
- [ ] Validation: pre-run setup, enabled modules before first run, cross-dataset mode, post-run results table, exports, and reports-folder access
- [ ] Light theme review
- [ ] Dark theme review
- [ ] Standard density review
- [ ] Dense mode review
- [ ] Reduced-motion review
- [ ] Keyboard traversal through visible and disclosed regions

Guardrail review:

- [ ] Re-check the functional-coverage matrix for every regrouped capability path
- [ ] Confirm Data still exposes the current-folder view and open-folder utility while Settings remains canonical for directory management
- [ ] Confirm Validation still acts as the canonical run/export/report surface
- [ ] Confirm no Strategy or Advanced control that materially affects request construction became unreachable or too deeply buried

## Targeted Test Additions

Add focused tests only where behavior actually changes.

Checklist:

- [ ] Validation test coverage for modules/automation being reachable before first run
- [ ] Validation test coverage for export/report actions remaining reachable in the canonical surface
- [ ] Strategy test coverage if disclosure/regrouping auto-reveals active non-default state
- [ ] Advanced test coverage if new derived non-default summaries or support lines are added
- [ ] Keyboard/focus regression coverage for any newly introduced details/disclosure regions

## UX Pass/Fail Criteria

Pass:

- setup-heavy work is easier to scan without extra navigation
- expert users can still perform common work without extra clicks
- active non-default state is easier to understand at a glance
- pre-run versus post-run responsibilities are clearer in Validation
- Data versus Settings ownership is clearer, not blurrier

Fail:

- common controls move behind repeated disclosure clicks
- Validation setup still depends on results existing
- Data provenance or schema becomes harder to inspect
- Strategy or Advanced lose clear reachability for real tuning
- newly regrouped sections require relearning where canonical actions live

## Explicit Defers And Rejects

Defer:

- `vee-validate` / `@vee-validate/zod` adoption
- a schema-owned form migration
- shell or titlebar changes that belong to Prompt 1 or Prompt 2
- execution/settings IA changes that belong to Prompt 6
- any mode-separation work that belongs to Prompt 9

Reject:

- wizardizing Data, Strategy, Advanced, or Validation
- broad store-ownership rewrites disguised as form cleanup
- panel-local tab/disclosure/card systems that bypass Prompt 2
- hiding canonical run/export actions inside menus
- moving market-data-folder management out of Settings
- forcing Validation into Top-style sticky selection

## Optional Interpretive Follow-Up Checklist

Only consider these items after the objective checkpoint is complete and committed.

- [ ] Decide the exact cutoff for which Strategy constraints remain always visible versus move into one shared `More constraints` disclosure
- [ ] Decide whether Validation result-time exports should move visually closer to the results plane instead of remaining grouped with setup-time actions
- [ ] Decide whether Advanced should serve as a low-risk pattern pilot before broader regrouping, rather than following the default objective order
- [ ] Decide whether the product should adopt a strict disclosure-depth budget such as "one click down" as a repo-wide rule
- [ ] Decide whether a distinct non-default summary-chip treatment is materially better than the current shared chip language

## Expected Edit Inventory

Expected touched files for the objective pass:

- `Frontend Review/Prompt3/dense_form_progressive_disclosure_implementation_tracking_2026-04-04.md`
- `web/src/styles/app.css`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- optionally `web/src/stores/optimizeStore.ts`
- optionally `web/src/stores/validationStore.ts`
- optionally `web/src/composables/useQueuedInputWriter.ts`

If implementation begins touching unrelated shell or mode-management files, stop and verify that the work has not drifted beyond Prompt 3 scope.
