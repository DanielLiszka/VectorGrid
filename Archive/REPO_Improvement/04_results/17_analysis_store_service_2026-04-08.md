# Analysis Store And Service Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

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
5. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document which parts of analysis state are results-only versus meta/finalizer state.
- [x] Document overview/top/drill/heatmap ownership boundaries.
- [x] Document baseline compare loading and summary behavior.
- [x] Document cache, in-flight dedupe, and stale-response protection behavior.
- [x] Document `top_k=0` and `MlSignal` special-case guardrails.
- [x] Add or tighten tests if analysis projection or store routing changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/analysisService.test.ts src/stores/analysisStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/OverviewWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/TopWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current result semantics before splitting `analysisService.ts` or `analysisStore.ts`
- keep pure refactors honest by rerunning the existing broad analysis test surface
- if meta/finalizer state is separated later, maintain the current public store action surface until the migration is complete
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the analysis-layer split with [analysisService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/analysisService.test.ts) and [analysisStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/analysisStore.actions.test.ts), which continue to cover shortlist projection, compare actions, drill/stability follow-ups, heatmap behavior, and meta/finalizer action routing.
- The main remaining structural gap from the backend audit is not a broken surface but the lack of Tauri integration tests for the analysis commands that feed this service/store pair.

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
