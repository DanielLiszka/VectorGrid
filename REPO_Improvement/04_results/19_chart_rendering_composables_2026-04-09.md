# Chart Rendering Composables Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to chart-rendering composables and shared chart runtime helpers so result-panel canvases and lightweight-chart hosts stay stable, resize-safe, and easier to evolve without regressions.

## Scope Boundaries

In scope:

- shared chart runtime helpers used by results panels
- `useLightweightChart` ownership, resize handling, and disposal behavior
- panel-level rendering patterns that depend on shared chart helpers
- cross-panel rendering stability for Drilldown and related chart surfaces

Out of scope:

- strategy-specific chart data generation
- non-results charting outside current analysis panels
- options-related chart surfaces

## Current User-Facing Behavior

The user sees several chart surfaces across results, but the rendering runtime beneath them is partly shared. Drilldown uses a shared lightweight-chart helper for host binding, resize observation, and disposal. Heatmap and other chart panels also depend on consistent mount and rerender behavior coordinated through analysis services and panel wrappers.

That means chart stability is not purely component-local. A regression in shared chart runtime behavior can surface as broken resize, stale chart hosts, or inconsistent cleanup across multiple result panels.

This doc exists to keep the shared rendering layer explicit so improvements to chart output do not quietly destabilize the panels that depend on it.

## Current Code Surface

- `web/src/composables/useLightweightChart.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`

## Current Verification Surface

- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- `web/src/components/ParetoWorkspacePanel.test.ts`
- targeted `analysisService` tests covering rerender and restore behavior

## Strengths Already Present

- the shared lightweight-chart helper already centralizes host, chart, resize, and disposal behavior
- result panels already separate mount wrappers from most rendering logic
- analysis services already coordinate rerender and restore behavior for some chart panels
- shared helpers already prefer explicit disposal instead of implicit leaked runtime state

## Weaknesses / Failure Modes

- shared chart runtime behavior is easy to overlook because the visible panels own the UI
- host resize and disposal regressions can appear only after navigation or repeated rerendering
- panel rendering contracts are distributed across components and `analysisService`
- there is no dedicated implementation doc freezing the shared chart-runtime obligations
- future chart work could bypass shared helpers and create inconsistent runtime behavior across panels

## Regressions To Avoid

- leaking chart instances or resize observers across tab switches
- breaking host resize behavior for shared chart surfaces
- bypassing shared helpers in ways that fragment chart lifecycle handling
- leaving stale chart hosts bound after result-panel rerender or dispose
- making panel rendering behavior depend on hidden global state rather than explicit runtime helpers

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because chart runtime state should stay explicit and coordinated, not hidden in component-local side effects.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because shared rendering regressions often show up only in user-visible navigation and rerender flows.
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) is relevant indirectly because a readable chart depends on stable rendering lifecycle, not just visual styling.
- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because chart runtimes need to survive panel switching without leaking or misbinding.

## Proposed Improvement Approach

1. Freeze the shared chart-runtime responsibilities explicitly.
2. Keep host binding, resize observation, and disposal behavior centralized in shared helpers where possible.
3. Document which rendering behaviors remain panel-local versus shared-runtime obligations.
4. Treat rerender, restore, and tab-switch behavior as first-class rendering correctness concerns.
5. Add or tighten tests whenever shared chart helpers or panel rendering lifecycles change.

## Checklist

- [ ] Document shared chart-runtime responsibilities and boundaries.
- [ ] Document resize-observer and disposal expectations.
- [ ] Document panel-local versus shared rendering responsibilities.
- [ ] Document rerender and restore expectations for chart-bearing result panels.
- [ ] Add or tighten tests for chart lifecycle, rerender, and tab-switch stability.

## Verification Plan

- run `npm --prefix web test -- src/components/DrillWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/HeatmapWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/ParetoWorkspacePanel.test.ts`
- run targeted `analysisService` tests if chart lifecycle handling changes

## Rollout / Fallback / Compatibility Notes

- preserve the current shared-helper pattern before introducing panel-specific chart runtime forks
- prefer additive shared helpers over duplicating lifecycle logic in each panel
- if new chart surfaces are added later, route them through the shared lifecycle model unless there is a clear exception
- keep chart-runtime changes synchronized with panel rerender and restore semantics

## Related Docs

- `REPO_Improvement/04_results/04_results_pareto_2026-04-09.md`
- `REPO_Improvement/04_results/05_results_drilldown_2026-04-09.md`
- `REPO_Improvement/04_results/06_results_heatmap_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/05_fe_results_surface_adequacy_DATE.md`
