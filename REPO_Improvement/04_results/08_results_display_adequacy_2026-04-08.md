# Results Display Adequacy Plan

Last updated: 2026-04-08

## Purpose

Define whether the current result surfaces expose enough evidence to support real optimization and validation decisions.

## Scope Boundaries

In scope:

- Overview, Top, Pareto, Drilldown, Heatmap, and related analysis surfaces
- evidence density and readability
- stale-data and path/report cues

Out of scope:

- options result surfaces
- workbook output quality (handled separately)

## Current User-Facing Behavior

The app exposes multiple result tabs and context docks, but the evidence needed to trust a candidate is split across panes. Some surfaces are strong for scanability, while others can be too sparse or too presentation-first when users need decision-grade detail.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`
- `web/src/services/analysisService.ts`

## Current Verification Surface

- results-related component tests
- browser/desktop UX harnesses
- saved artifact replay/manual review

## Strengths Already Present

- there are multiple views for ranking, comparison, charts, and drilldown
- the app already has context docks and artifact hydration
- advanced metrics and validation evidence exist elsewhere in the system

## Weaknesses / Failure Modes

- the most important decision signals may be split across tabs
- chart/readability choices can become decorative instead of analytical
- stale-data and report-path cues can be too implicit
- missing-data states can hide whether a module was skipped, unavailable, or simply not run

## Regressions To Avoid

- removing dense evidence in the name of simplification
- hiding warning/fallback context from the main decision surfaces
- making tables or charts less explorable for large result sets
- desynchronizing visible results from hydrated artifact data

## External Research Summary

- [Playwright Best Practices](https://playwright.dev/docs/best-practices) supports evaluating what end users can actually see and act on.
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) reinforces chart/data-fit, titles, labels, and customization as first-class clarity concerns.
- [Microsoft Support: Creating charts from start to finish](https://support.microsoft.com/en-us/office/creating-charts-from-start-to-finish-36f8a2b7-0f74-4d40-942d-bc2e42436d73) reinforces choosing a chart type that matches the underlying data and arranging data clearly.

## Proposed Improvement Approach

1. Define the key user questions each result pane must answer.
2. Identify evidence that must be visible without forcing unnecessary tab-hopping.
3. Clarify `not run`, `n/a`, skipped, filtered, and stale states.
4. Treat table density, chart labeling, and comparison flows as decision-support surfaces, not polish-only details.

## Checklist

- [ ] Define the minimum evidence set for trusting an optimization winner.
- [ ] Document what Overview must summarize vs defer to other panes.
- [ ] Document what Top must expose for scanability and compare actions.
- [ ] Document what Pareto/Heatmap/Drilldown must explain, not just render.
- [ ] Document stale-data and artifact/report cues that must be visible.
- [ ] Document empty-state and missing-module language requirements.
- [ ] Add or tighten UX tests for high-value result behaviors.
- [ ] Verify large tables remain readable and navigable.

## Verification Plan

- run representative component tests for Overview/Top/Pareto/Drill/Heatmap
- run browser UX smoke for result-surface interactions
- manually review one optimization run and one hydrated artifact replay

## Rollout / Fallback / Compatibility Notes

- preserve existing result tabs unless a clear consolidation plan exists
- prefer additive evidence and clearer messaging over hiding existing detail
- if information moves between panes, keep discoverability and compare flows intact

## Related Docs

- `REPO_Improvement/04_results/10_out_overview_decision_surface_DATE.md`
- `REPO_Improvement/04_results/20_matrix_table_state_DATE.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
