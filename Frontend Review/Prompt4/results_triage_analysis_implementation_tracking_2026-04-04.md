# Results Triage And Analysis Implementation Tracking

Date: 2026-04-04

Scope: implementation tracker for the grounded Prompt 4 direction in `Frontend Review/Prompt4/results_triage_analysis_direction_FINAL.md`.

Status: planned

Owner posture: frontend analysis-workflow refinement only. Do not broaden into backend contract changes, run-library/history consolidation, shell information-architecture redesign, Prompt 3 control-surface reassignment, or product-mode work.

## Purpose

This document turns the grounded Prompt 4 conclusions into an execution tracker with:

- explicit phase ordering
- objective-first checkpointing
- concrete component/store/service ownership
- capability-preservation checklists
- review-path and keyboard-path guardrails
- verification and evidence requirements
- a separate optional section for the more interpretive Prompt 4 ideas

This tracker is for executing the valid Prompt 4 direction:

- keep the current analysis composition of Overview, Top, Drill, Pareto, and Heatmap
- keep Prompt 3’s boundary intact so validation setup, modules/automation policy, and validation export/report ownership remain canonical in `ValidationWorkspacePanel.vue`
- strengthen Top as the primary shortlist triage surface without hollowing out Overview’s winner-framing and baseline-review role
- keep Drill as the follow-on inspection surface rather than a second candidate browser
- keep Pareto and Heatmap as contextual chart lenses with explicit keyboard-safe handoff to and from Top
- preserve existing backend-backed analysis capabilities, especially Top export, Heatmap export, drilldown, local stability analysis, and workflow-run refresh into analysis views

## Functionality Posture

This plan is additive and coordination-oriented in nature.

It is intended to:

- clarify which analysis surface owns which review responsibility
- reduce duplicated review chrome where it adds noise instead of confidence
- strengthen the keyboard-safe review path between shortlist selection and deeper chart inspection
- preserve the existing review surfaces while making their roles more obvious
- improve compare, provenance, and stale-state weighting without removing capabilities

It is not intended to:

- remove backend-backed analysis capabilities as a simplification shortcut
- move validation run/report/export ownership into analysis surfaces
- turn Drill, Pareto, or Heatmap into independent candidate-navigation systems
- turn Overview into a second shortlist or Top into a chart dashboard shell
- reopen Prompt 1 shell hierarchy or Prompt 2 visual-system decisions
- introduce a new panel-local styling grammar for analysis

If an implementation choice would reduce analysis reachability, weaken the non-pointer review path, or blur the Prompt 3 setup/review boundary, that choice is out of scope for this plan.

## Progress Ledger

- [ ] Phase 0 complete: baseline audit and evidence capture finished
- [ ] Phase 1 complete: analysis ownership and shared-state audit landed
- [ ] Phase 2 complete: Overview and Top hierarchy tightening landed
- [ ] Phase 3 complete: Drill, Pareto, and Heatmap handoff polish landed
- [ ] Phase 4 complete: compare / provenance / stale cue weighting alignment landed
- [ ] Objective checkpoint complete and ready for clean commit
- [ ] Optional interpretive follow-up reviewed
- [ ] Targeted verification complete
- [ ] Final evidence and screenshots attached

## Implementation Outcome Target

Target surfaces:

- `web/src/components/OverviewWorkspacePanel.vue`
  - stays narrow and decisive
  - keeps winner framing, baseline review, and execution-risk context visible
  - does not expand into a second shortlist or compare workspace
- `web/src/components/TopWorkspacePanel.vue`
  - remains the richest shortlist triage surface
  - preserves filtering, sorting, selection, selected-candidate summary, compare, and stale cues
  - makes the point-of-choice review posture clearer than it is today
- `web/src/components/DrillWorkspacePanel.vue`
  - remains the deep inspection surface
  - keeps explicit Top handoff
  - does not gain chart-owned candidate selection
- `web/src/components/ParetoWorkspacePanel.vue`
  - remains a frontier lens
  - keeps axis selection and chart click-through
  - preserves explicit Top handoff
- `web/src/components/HeatmapWorkspacePanel.vue`
  - remains a parameter-landscape lens
  - keeps Heatmap PNG export, chart click-through, and Top handoff
- `web/src/stores/analysisStore.ts`
  - may gain only narrow derived-state helpers that support clearer surface weighting
  - should not become a dumping ground for presentation-only local styling choices
- `web/src/services/analysisService.ts`
  - remains the canonical analysis orchestration layer
  - may take narrow adjustments for handoff text, refresh coordination, or contextual analysis metadata

Support-layer files only if needed:

- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/utils/iconography.ts`

## Checkpoint Strategy

This tracker is intentionally split into two implementation checkpoints.

### Checkpoint A — Objective Improvements

This checkpoint contains changes that are structurally useful and strongly grounded in the repo.

Examples:

- keep Overview narrow and baseline-oriented
- keep Top as the primary shortlist triage surface
- preserve the keyboard-safe Top-to-Drill review path
- preserve explicit Top handoff in Pareto and Heatmap
- preserve analysis exports and validation ownership boundaries
- clarify compare / provenance / stale cue weighting without removing any backend-backed path

Commit goal:

- produce a clean git commit after these changes land and pass verification
- this is the baseline Prompt 4 analysis-workflow commit that should remain even if later interpretive refinements are reverted

### Checkpoint B — Optional Interpretive Follow-Up

This checkpoint contains the more design-judgment-heavy recommendations from Prompt 4.

Examples:

- the exact degree of Top primacy versus Overview shared ownership
- contextual selected/pinned emphasis in Pareto and Heatmap renderers
- a compact current-candidate header at the top of Drill
- reducing duplicated export affordances if the result is genuinely cleaner
- deciding whether heavier analysis-shell expansion should stay deferred longer term

Workflow goal:

- make these changes only after Checkpoint A is safely committed
- keep them isolated enough that they can be split, stashed, or reverted if they reduce clarity

## Objective-First Execution Rule

Execution order for this tracker is locked:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Objective verification
7. Clean objective commit checkpoint
8. Optional interpretive follow-up

Implementation rule:

- do not begin the optional section until the objective checkpoint is complete
- treat Phase 1 through Phase 4 as the default must-land work
- treat the optional section at the end as deliberately deferrable

## Source Basis

Primary source:

- `Frontend Review/Prompt4/results_triage_analysis_direction_FINAL.md`

Supporting repo policy/context:

- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/frontend_audit_2026-03-27.md`
- `Frontend Review/workstation_behavior_plan_2026-03-28.md`
- `Frontend Review/Prompt3/dense_form_progressive_disclosure_implementation_tracking_2026-04-04.md`

Primary implementation files:

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`
- `web/src/services/analysisService.ts`

## Execution Guardrails

- Treat Prompt 3 as landed baseline. Validation setup, modules/automation policy, and validation export/report ownership stay canonical in `ValidationWorkspacePanel.vue`.
- Keep the current analysis composition of Overview, Top, Drill, Pareto, and Heatmap. Do not collapse surfaces together just because they share data.
- Preserve at least one clear frontend path for every backend-backed analysis capability in the coverage matrix.
- Keep the keyboard-safe review path obvious. Chart click-through remains additive, not required.
- Do not create a chart-owned candidate-navigation model in Drill, Pareto, or Heatmap.
- Do not remove Top CSV/JSON export or Heatmap PNG export.
- Do not move baseline comparison out of Overview unless a future explicit product decision overrides that boundary.
- Do not introduce new panel-local styling systems that bypass the shared Prompt 2 visual language.
- Do not broaden this pass into History / Meta / artifact-browser redesign; that belongs to Prompt 5.

## Explicit Working Decisions

These are the working defaults for implementation in this pass. Treat them as locked unless the product owner explicitly overrides them.

### 1. Setup versus review boundary

Decision:

- Prompt 3 setup ownership remains intact
- analysis surfaces sharpen review and handoff only

Implementation meaning:

- no validation run/export/report reassignment
- no setup modules/automation controls added to Overview, Top, Drill, Pareto, or Heatmap

### 2. Overview role

Decision:

- Overview remains a narrow decision-header and baseline-review plane

Implementation meaning:

- keep winner framing, baseline selection, compare details, and execution-risk context
- do not add pinned compare trays, richer candidate switching, or chart-heavy responsibilities

### 3. Top role

Decision:

- Top is the primary shortlist triage surface for this pass

Implementation meaning:

- keep filtering, sorting, selection, selected-candidate context, compare, provenance, and stale cues here
- any hierarchy tightening should make Top faster to scan rather than adding new tools

### 4. Drill role

Decision:

- Drill remains the deep inspection surface, not a second browser

Implementation meaning:

- preserve rerender, stability, trade navigation, and Top handoff
- avoid introducing drill-local shortlist management

### 5. Pareto and Heatmap role

Decision:

- Pareto and Heatmap remain contextual chart lenses

Implementation meaning:

- preserve click-to-drill and explicit Top handoff
- avoid expanding them into separate control-heavy analysis workbenches

### 6. Compare / provenance / stale weighting

Decision:

- compare, provenance, and stale cues remain weighted by surface rather than duplicated equally everywhere

Implementation meaning:

- keep the richest compare and candidate-choice cues in Top
- keep baseline comparison in Overview
- keep other surfaces lighter and more contextual

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Purpose:

- lock the current review-surface state before hierarchy changes begin
- record which analysis capabilities and shortcuts are actually canonical today

Checklist:

- [ ] Capture current screenshots for Overview, Top, Drill, Pareto, and Heatmap in light theme
- [ ] Capture at least one Dense-mode screenshot for Top and one chart-heavy surface
- [ ] Record where Top CSV/JSON export currently appears
- [ ] Record where Heatmap PNG export currently appears
- [ ] Record where baseline comparison currently lives
- [ ] Record current Top selected-candidate, compare, and stale-cue presentation
- [ ] Record current Drill, Pareto, and Heatmap Top handoff copy
- [ ] Record current keyboard-safe path cues across chart-heavy surfaces

Acceptance criteria:

- a before-state exists for every touched analysis surface
- current canonical export/handoff points are explicitly listed before regrouping begins

## Phase 1 — Analysis Ownership And Shared-State Audit

Purpose:

- identify which analysis behaviors are already store/service-owned and which are still local fallback presentation
- avoid solving ownership questions with unnecessary store rewrites

Primary files:

- `web/src/stores/analysisStore.ts`
- `web/src/services/analysisService.ts`

Checklist:

- [ ] Audit which selected-candidate fields are canonical in `analysisStore.ts` versus locally synthesized in `TopWorkspacePanel.vue`
- [ ] Audit compare state ownership between `analysisStore.ts`, `analysisService.ts`, and Overview/Top
- [ ] Audit stale-cue generation versus stale-cue presentation
- [ ] Add only narrow derived-state helpers if hierarchy work needs clearer shared analysis summaries
- [ ] Avoid moving local fallback presentation into the store unless a concrete coordination bug requires it
- [ ] Avoid any backend or Tauri contract changes

Acceptance criteria:

- analysis coordination remains mostly store/service-backed
- any store/service edits are narrow support changes, not architecture rewrites
- hierarchy work can proceed without ambiguity about export and handoff ownership

## Phase 2 — Overview And Top Hierarchy Tightening

Purpose:

- make review ownership obvious at the point where candidate choice actually happens
- strengthen distinction between winner framing and shortlist triage without removing either role

Primary files:

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- optionally `web/src/stores/analysisStore.ts`
- optionally `web/src/services/analysisService.ts`

Checklist:

- [ ] Preserve Overview winner framing, baseline selection, compare details, and execution-risk context
- [ ] Preserve Overview Top CSV/JSON export reachability unless a clearly equivalent analysis-surface path is maintained
- [ ] Tighten Overview so it reads as a decision header rather than a second shortlist surface
- [ ] Preserve Top shortlist filters, sorting, row selection, compare actions, and selected-candidate context
- [ ] Keep Top compare compact and subordinate to shortlist scanning
- [ ] Keep Top stale cues inline and local to selected/compare context
- [ ] Preserve selected-candidate provenance and decision-note visibility in Top
- [ ] Keep column posture controls available but visually secondary to triage work
- [ ] Avoid duplicating the same compare / provenance / stale-state chrome with equal weight in both Overview and Top

Acceptance criteria:

- Overview still frames the current winner and baseline comparison clearly
- Top is clearly the fastest place to choose the next candidate for deeper inspection
- no existing analysis export or compare path becomes harder to reach

## Phase 3 — Drill, Pareto, And Heatmap Handoff Polish

Purpose:

- keep chart-heavy surfaces useful while preserving Top as the non-pointer navigation spine

Primary files:

- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- optionally `web/src/services/analysisService.ts`

Checklist:

- [ ] Preserve explicit `Review Top results` handoff in Drill
- [ ] Preserve explicit `Review Top results` handoff in Pareto
- [ ] Preserve explicit `Review Top results` handoff in Heatmap
- [ ] Preserve Drill rerender, local stability, and trade-navigation behavior
- [ ] Preserve Pareto axis selection and click-to-drill behavior
- [ ] Preserve Heatmap PNG export and click-to-drill behavior
- [ ] Keep chart-assist copy clear that chart interaction is additive and Top remains the keyboard-safe selection path
- [ ] Avoid introducing chart-local candidate switching, compare trays, or second-selection state

Acceptance criteria:

- chart-heavy surfaces still support direct drill interaction without becoming the only viable navigation path
- Top handoff remains explicit and easy to find
- drilldown and local stability analysis remain intact

## Phase 4 — Compare / Provenance / Stale Cue Weighting Alignment

Purpose:

- reduce repeated review chrome while keeping trust and explanation strong at the right surface

Primary files:

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- optionally `web/src/stores/analysisStore.ts`
- optionally `web/src/services/analysisService.ts`

Checklist:

- [ ] Keep baseline comparison strongest in Overview
- [ ] Keep selected-candidate provenance strongest in Top
- [ ] Keep pinned compare state strongest in Top
- [ ] Keep stale-state messaging advisory and inline rather than modal
- [ ] Reduce duplicated compare/provenance/stale chrome on Drill, Pareto, and Heatmap where it competes with the main task
- [ ] Preserve any existing candidate or artifact context needed to keep charts understandable
- [ ] Confirm no analysis surface loses the cues needed for trust, explanation, or safe handoff

Acceptance criteria:

- compare, provenance, and stale cues are weighted by surface rather than repeated indiscriminately
- trust and review confidence improve without adding more chrome
- the setup/review boundary from Prompt 3 remains intact

## Objective Verification Checklist

Required repo checks:

- [ ] `npm --prefix web run lint`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run test`
- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run a11y:smoke`
- [ ] `npm --prefix web run baseline:ui`
- [ ] `npm --prefix web run check:bundle-budget` if a phase changes shared dense-surface behavior or increases startup-facing code weight

Required manual review:

- [ ] Overview: winner framing, baseline selection, compare details, execution-risk review, export reachability
- [ ] Top: filter/sort/select flow, compare tray, stale cues, selected-candidate context, column-visibility posture
- [ ] Drill: rerender after tab reveal, Top handoff, stability visibility, trade navigation
- [ ] Pareto: axis changes, visible frontier rendering, click-through to Drill, Top handoff clarity
- [ ] Heatmap: ready/empty states, click-through to Drill, PNG export, Top handoff clarity
- [ ] Light theme review
- [ ] Dark theme review
- [ ] Standard density review
- [ ] Dense mode review
- [ ] Reduced-motion review
- [ ] Keyboard traversal through Top and all chart-heavy handoff paths

Guardrail review:

- [ ] Re-check the functional-coverage matrix for every touched analysis path
- [ ] Confirm Validation still acts as the canonical validation run/export/report surface
- [ ] Confirm Top CSV/JSON export remains reachable from analysis
- [ ] Confirm Heatmap PNG export remains reachable from analysis
- [ ] Confirm Drill, Pareto, and Heatmap still preserve explicit non-pointer return-to-Top guidance

## Targeted Test Additions

Add focused tests only where behavior actually changes.

Checklist:

- [ ] Overview test coverage if export placement or baseline-review hierarchy changes
- [ ] Top test coverage if compare/stale/provenance hierarchy changes materially
- [ ] Drill test coverage if handoff copy or current-selection context changes
- [ ] Pareto test coverage if handoff or click-through behavior changes
- [ ] Heatmap test coverage if export or click-through behavior changes
- [ ] Keyboard/focus regression coverage for any changed handoff or disclosure regions

## UX Pass/Fail Criteria

Pass:

- candidate-choice responsibility is clearer than before
- chart-heavy surfaces remain useful without competing for control ownership
- compare, provenance, and stale cues feel better weighted instead of more duplicated
- the keyboard-safe path from shortlist review to deeper inspection stays obvious
- analysis cleanup does not encroach on Prompt 3 setup ownership

Fail:

- Overview and Top still feel like equally weighted competing control planes
- Drill, Pareto, or Heatmap become required for normal candidate navigation
- analysis exports or compare paths become harder to discover
- validation ownership becomes blurrier after the analysis pass
- the redesign adds more review chrome than it removes

## Explicit Defers And Rejects

Defer:

- Prompt 5 history / artifact-browser changes
- Prompt 9 mode-separation decisions
- shell or titlebar changes that belong to Prompt 1 or Prompt 2
- backend contract changes for analysis payloads
- major charting-library replacement decisions

Reject:

- turning analysis into a chart-only exploration workflow
- moving validation setup, modules/automation policy, or validation export/report ownership into analysis
- creating a second candidate browser inside Drill, Pareto, or Heatmap
- introducing panel-local analysis styling systems that bypass the shared Prompt 2 language
- removing exports or drilldown paths as a declutter shortcut

## Optional Interpretive Follow-Up Checklist

Only consider these items after the objective checkpoint is complete and committed.

- [ ] Decide the exact degree of Top primacy versus Overview shared ownership if future product direction wants a stronger single-command-center interpretation
- [ ] Decide whether more of Top’s fallback selected-candidate presentation logic should move from local component logic into `analysisStore.ts` / `analysisService.ts`
- [ ] Decide whether selected and pinned candidates should receive stronger contextual emphasis inside Pareto rendering
- [ ] Decide whether selected and pinned candidates should receive stronger contextual emphasis inside Heatmap marker/overlay rendering
- [ ] Decide whether Drill should gain a compact current-candidate context header sourced from existing analysis-store state
- [ ] Decide whether duplicate analysis export affordances should be quieted further, as long as capability reachability remains explicit
- [ ] Reconfirm that heavier long-term defers such as a split-bench analysis shell or second permanent compare pane should stay out of scope

## Expected Edit Inventory

Expected touched files for the objective pass:

- `Frontend Review/Prompt4/results_triage_analysis_implementation_tracking_2026-04-04.md`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- optionally `web/src/stores/analysisStore.ts`
- optionally `web/src/services/analysisService.ts`

If implementation begins touching Validation, shell navigation, artifact browsers, or backend contracts directly, stop and verify that the work has not drifted beyond Prompt 4 scope.
