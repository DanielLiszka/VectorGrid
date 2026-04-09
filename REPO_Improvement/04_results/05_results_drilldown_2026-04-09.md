# Results Drilldown Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Drilldown panel so selected-candidate rendering, chart/status synchronization, stability context, and trade-table evidence remain coherent and decision-grade.

## Scope Boundaries

In scope:

- `web/src/components/DrillWorkspacePanel.vue`
- drilldown render pipeline and caches in `analysisService`
- drill meta/status, stability, and trade-list presentation
- Top-to-Drill handoff and selected-candidate dependency

Out of scope:

- Top shortlist selection mechanics themselves
- heatmap and Pareto rendering
- options-related drilldown surfaces

## Current User-Facing Behavior

The Drilldown panel is already a multi-surface evidence view, not just a single chart. It depends on a selected Top-result candidate, then renders a price-and-trades chart, equity envelope, drawdown envelope, optional stability table, and trade list. It also reports drill meta, drill status, stability status, and supporting chart instructions.

The panel already makes its dependency explicit by telling users to select a Top row, and it keeps a strong “Review Top results” handoff so the shortlist stays the primary selection surface. The analysis service also caches drill curves, stability output, and drill price bars, which means correctness depends on both rendering and cache invalidation behavior.

That makes Drilldown a contract between selection identity, analysis-service execution, chart rendering, and fallback status copy.

## Current Code Surface

- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- targeted command and analysis tests for drilldown and local stability flows

## Strengths Already Present

- the panel already includes multiple complementary evidence surfaces
- selection dependency is already explained instead of hidden
- drill and stability status lines already exist as distinct advisory channels
- analysis-service caching already protects the panel from needless recomputation
- the panel already keeps shortlist navigation anchored in Top Results

## Weaknesses / Failure Modes

- drilldown correctness depends on row identity, run identity, cached curves, and rendered state all staying aligned
- several chart surfaces are canvas or DOM-rendered and can become presentation-heavy without enough explanatory context
- failure or empty cases can look visually similar unless drill/stability/trade statuses stay precise
- the trade table and stability table can silently fall out of sync with the selected candidate if render-state invalidation weakens
- there is no dedicated implementation doc freezing drilldown as a combined evidence surface rather than a chart-only tab

## Regressions To Avoid

- rendering stale drill or stability data after selection or run changes
- weakening or removing explicit Top-to-Drill handoff guidance
- hiding trade-table or stability-table failure states behind generic empty charts
- breaking drill cache invalidation or render-state capture assumptions
- reducing the detail available in drill meta, drill status, or trade/stability summaries

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because Drilldown is one tab in a larger analysis workflow and should keep a distinct responsibility.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because drill, stability, and trade advisories are live informational updates that should not pull focus.
- [Basic usage of canvas | MDN](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Basic_usage) is relevant because several drill charts are canvas-based and need surrounding explanatory content.
- [Canvas element accessibility guidance | MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/canvas) is relevant because bitmap charts alone do not expose enough semantic detail.
- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the stability and trade tables are interactive evidence surfaces that need consistent keyboard and focus behavior.

## Proposed Improvement Approach

1. Freeze Drilldown as a selected-candidate evidence panel composed of charts, stability context, and trade evidence.
2. Document the dependency chain explicitly: Top selection -> invocation building -> cached drill/stability computation -> rendered chart/table state.
3. Treat drill meta, drill status, and stability status as mandatory truth channels, not optional captions.
4. Preserve alternative guidance and tabular evidence around canvas-heavy charts.
5. Add or tighten tests around selection changes, rerender behavior, cache-hit semantics, and Top-results handoff.

## Checklist

- [ ] Document the Drilldown dependency on selected Top-result identity.
- [ ] Document chart, stability, and trade-table responsibilities separately.
- [ ] Document drill/stability status semantics and failure-state expectations.
- [ ] Document cache and rerender assumptions that keep selected-candidate state truthful.
- [ ] Add or tighten tests for rerender behavior, selection changes, and Top-results handoff.

## Verification Plan

- run `npm --prefix web test -- src/components/DrillWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/analysisService.test.ts`
- run targeted drilldown and local-stability tests in the backend where request contracts change
- manually verify selection-change, cached rerender, and failure-state behavior in the UI

## Rollout / Fallback / Compatibility Notes

- preserve the explicit dependency on Top selection until a better canonical selection model exists
- keep drill/stability/trade status channels stable even if chart rendering changes
- prefer additive detail over collapsing Drilldown into a single simplified chart
- if rendering pipelines are refactored later, update cache invalidation and selected-candidate tests in the same change

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/13_out_drilldown_stability_DATE.md`
- `REPO_Improvement/04_results/19_chart_rendering_composables_DATE.md`
- `REPO_Improvement/07_strats/14_analysis_drilldown_stability_2026-04-08.md`
