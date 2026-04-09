# Analysis Store And Service Plan

Last updated: 2026-04-08

## Purpose

Define the current analysis-layer split so `analysisService` remains the result projection/execution engine and `analysisStore` remains the action/state boundary without losing current shortlist, baseline, drilldown, heatmap, meta, and ML behavior.

## Scope Boundaries

In scope:

- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- overview/top/drill/heatmap/result projection behavior
- baseline compare and cached follow-up computations

Out of scope:

- options result surfaces
- Rust-side drilldown or stability implementations
- new analysis modes that do not already exist

## Current User-Facing Behavior

The analysis layer already does substantial work. It builds overview winner framing, ranked top-table rows, compare summaries, drilldown and local-stability views, heatmap state, export outputs, and several meta/stage4/stage5/ML result surfaces. It also contains protections users already depend on: no fabricated shortlist when `top_k=0`, selection preservation where possible, stale-response suppression for expensive follow-ups, cached rerender behavior, and special handling so `MlSignal` artifacts do not masquerade as optimizer result tables. The store itself is mostly a dispatch and state carrier, but it also carries a large amount of meta/finalizer state.

## Current Code Surface

- `web/src/services/analysisService.ts`
  - result projection engine, exports, compare logic, drilldown/stability follow-ups, and rendering helpers
- `web/src/services/analysisService.test.ts`
  - broad coverage of top/overview/drill/stability/heatmap/compare semantics
- `web/src/stores/analysisStore.ts`
  - action/state layer for results, meta, stage4/stage5, and ML-facing analysis state
- `web/src/stores/analysisStore.actions.test.ts`
  - action-routing and store-level behavior coverage
- `web/src/components/OverviewWorkspacePanel.vue`
  - consumes winner/compare overview state
- `web/src/components/TopWorkspacePanel.vue`
  - consumes shortlist, filters, compare tray, and table state
- `web/src/services/workflowArtifactsSync.ts`
  - can trigger analysis refresh after artifact hydration

## Current Verification Surface

- `web/src/services/analysisService.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`
- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`

## Strengths Already Present

- analysis projection is already centralized instead of being spread across components
- current tests already lock in several subtle behaviors that matter to the user
- expensive follow-up work already uses caching and stale-response protection
- the layer already respects important workflow truth, such as `top_k=0` and `MlSignal` special cases
- overview/top surfaces already have differentiated responsibilities

## Weaknesses / Failure Modes

- `analysisService.ts` is very large and mixes view-model building with execution follow-ups and rendering concerns
- `analysisStore.ts` carries both results state and large meta/finalizer/ML state families
- there is repeated `asRecord`/`text`/`num` normalization logic that obscures true payload contracts
- module-scope caches and sequence counters are effective but easy to misuse during refactors
- results ownership can be blurred because the store is both UI state carrier and meta/finalizer dispatch surface

## Regressions To Avoid

- fabricating shortlist rows when `top_k=0`
- losing selection guardrails when filters or column visibility change
- breaking score-mode-derived columns or best-summary text
- weakening stale-response suppression for drilldown or stability calls
- causing unnecessary heatmap base rerenders on restore
- allowing `MlSignal` artifacts to populate normal optimizer result surfaces
- breaking current Stage 4 / Stage 5 routing behavior already pinned by tests

## External Research Summary

- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the analysis layer is a shared domain state boundary, not component-local state
- [Pinia: Core Concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because store ownership and action routing are central to this layer
- [TanStack Table: Column Visibility](https://tanstack.com/table/latest/docs/guide/column-visibility)
  - relevant because Top Results already persists and reacts to column visibility state
- [TanStack Virtual: Introduction](https://tanstack.com/virtual/latest/docs/introduction)
  - relevant because shortlist rendering already relies on virtual-row behavior
- [WAI-ARIA APG: Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
  - relevant because shortlist interaction semantics depend on grid-like keyboard and selection behavior

## Proposed Improvement Approach

1. Document the current analysis split into clear families:
   - winner/overview projection
   - shortlist/table projection
   - baseline compare
   - drilldown/stability follow-ups
   - heatmap rendering state
   - meta/finalizer/ML state
2. Preserve the service as the projection engine, but separate plan sections for:
   - pure data shaping
   - async command follow-ups
   - DOM/canvas rendering helpers
3. Document module-scope caches and sequence guards explicitly before refactoring them.
4. Treat current tests as contract, not incidental coverage.

## Checklist

- [ ] Document which parts of analysis state are results-only versus meta/finalizer state.
- [ ] Document overview/top/drill/heatmap ownership boundaries.
- [ ] Document baseline compare loading and summary behavior.
- [ ] Document cache, in-flight dedupe, and stale-response protection behavior.
- [ ] Document `top_k=0` and `MlSignal` special-case guardrails.
- [ ] Add or tighten tests if analysis projection or store routing changes.

## Verification Plan

- run `npm --prefix web test -- src/services/analysisService.test.ts src/stores/analysisStore.actions.test.ts`
- run `npm --prefix web test -- src/components/OverviewWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/TopWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current result semantics before splitting `analysisService.ts` or `analysisStore.ts`
- keep pure refactors honest by rerunning the existing broad analysis test surface
- if meta/finalizer state is separated later, maintain the current public store action surface until the migration is complete

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
