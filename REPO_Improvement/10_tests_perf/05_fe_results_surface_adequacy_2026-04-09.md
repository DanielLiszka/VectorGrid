# FE Results Surface Adequacy Plan

Last updated: 2026-04-09

## Purpose

Define the frontend test strategy for results-surface adequacy so overview, top, pareto, heatmap, drilldown, and related result tabs remain readable, truthful, and fit for decision-making.

## Scope Boundaries

In scope:

- results-panel component tests
- results-related browser review expectations
- adequacy of empty states, warnings, and dense result tables

Out of scope:

- backend result generation
- validation workbook exports
- options results surfaces

## Current User-Facing Behavior

Users rely on the built app to interpret optimization and analysis results through multiple panels, tabs, tables, and charts. The repo already has test coverage for many of these surfaces, but adequacy as a user-facing quality bar still spans several files rather than one documented contract.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/components/AnalysisContextDock.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/components/OptimizeWorkspacePanel.test.ts`
- `web/e2e/browser/shell-ux.spec.ts`

## Current Verification Surface

- `npm --prefix web test`
- targeted results-panel component tests
- `npm --prefix web run test:browser:ux`

## Strengths Already Present

- key result tabs already have direct component coverage
- the repo already has a dedicated results-display adequacy planning lane
- browser smoke already exists for higher-level shell interaction proof

## Weaknesses / Failure Modes

- adequacy is broader than “component renders”
- result surfaces can degrade in readability without obvious functional breakage
- dense tables and chart-heavy views need more explicit quality ownership than generic test presence

## Regressions To Avoid

- treating a rendering smoke as proof that a results surface is still decision-usable
- allowing warnings, empty states, or provenance cues to regress silently
- losing coverage on one results tab because another higher-level smoke still passes

## External Research Summary

- [Vue Test Utils guide](https://test-utils.vuejs.org/guide/)
  - relevant because direct panel tests remain the narrowest proof for rendered adequacy cues
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because browser proof should target user-visible workflows rather than duplicate unit assertions
- [Playwright aria snapshots](https://playwright.dev/docs/aria-snapshots)
  - relevant because accessibility-structure checks can help guard higher-level adequacy surfaces

## Proposed Improvement Approach

1. Treat results adequacy as a layered contract:
   - component proof for visible structure and cues
   - browser proof for shell-level navigation and cross-panel integrity
2. Keep provenance, warnings, empty states, and readability in scope, not just raw data presence.
3. Expand coverage when a results surface becomes denser or more decision-critical.

## Checklist

- [ ] Map each major results surface to at least one direct component test owner.
- [ ] Keep empty-state, stale/provenance, and warning cues explicitly covered.
- [ ] Use browser proof only for cross-panel interaction and navigation.
- [ ] Add tests when new result tabs or docked decision surfaces appear.

## Verification Plan

- run `npm --prefix web test`
- run targeted results-panel tests
- run `npm --prefix web run test:browser:ux` after major results-surface changes

## Rollout / Fallback / Compatibility Notes

- keep component-level adequacy assertions even if browser coverage expands
- avoid snapshot-heavy testing that hides the actual decision-surface contract

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/02_fe_component_surface_tests_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_DATE.md`
- `REPO_Improvement/10_tests_perf/19_desktop_artifact_capture_DATE.md`
