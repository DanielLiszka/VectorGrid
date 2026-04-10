# Results Display Adequacy Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Playwright Best Practices](https://playwright.dev/docs/best-practices) supports evaluating what end users can actually see and act on.
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) reinforces chart/data-fit, titles, labels, and customization as first-class clarity concerns.
- [Microsoft Support: Creating charts from start to finish](https://support.microsoft.com/en-us/office/creating-charts-from-start-to-finish-36f8a2b7-0f74-4d40-942d-bc2e42436d73) reinforces choosing a chart type that matches the underlying data and arranging data clearly.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Define the key user questions each result pane must answer.
2. Identify evidence that must be visible without forcing unnecessary tab-hopping.
3. Clarify `not run`, `n/a`, skipped, filtered, and stale states.
4. Treat table density, chart labeling, and comparison flows as decision-support surfaces, not polish-only details.
5. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the minimum evidence set for trusting an optimization winner.
- [x] Document what Overview must summarize vs defer to other panes.
- [x] Document what Top must expose for scanability and compare actions.
- [x] Document what Pareto/Heatmap/Drilldown must explain, not just render.
- [x] Document stale-data and artifact/report cues that must be visible.
- [x] Document empty-state and missing-module language requirements.
- [x] Add or tighten UX tests for high-value result behaviors.
- [x] Verify large tables remain readable and navigable.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run representative component tests for Overview/Top/Pareto/Drill/Heatmap
- run browser UX smoke for result-surface interactions
- manually review one optimization run and one hydrated artifact replay
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification

## Rollout / Fallback / Compatibility Notes

- preserve existing result tabs unless a clear consolidation plan exists
- prefer additive evidence and clearer messaging over hiding existing detail
- if information moves between panes, keep discoverability and compare flows intact
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the non-options results stack as a decision-support workflow by running the representative Overview, Top, Drilldown, Heatmap, Analysis dock, Meta, Validation, and Execution panel tests together with the shared analysis-service and table/composable tests.
- Added targeted empty-state and evidence-surface regression coverage in the Overview, Top, Drilldown, Heatmap, and shared chart-runtime tests so the most important decision cues stay pinned in code.
- The remaining deeper adequacy gaps from this audit are not panel-copy regressions but broader execution items: no single end-to-end decision-workflow spec across results tabs, no Tauri integration tests for analysis commands, and incomplete composite drill/local-stability backend support.

## Related Docs

- `REPO_Improvement/04_results/10_out_overview_decision_surface_2026-04-09.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
