# Results Empty States Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to results empty states so each panel distinguishes missing prerequisites, no-run conditions, filtered-out results, and unavailable modules with truthful, actionable language.

## Scope Boundaries

In scope:

- empty, pending, and “not available” states across Overview, Top, Pareto, Drilldown, and Heatmap
- status text, empty-state copy, and next-action guidance
- state distinctions driven by `useUiReadiness`, `analysisStore`, and workflow-run presence

Out of scope:

- detailed chart rendering once a panel is ready
- validation report empty states
- options-related result surfaces

## Current User-Facing Behavior

The results area already contains multiple distinct empty-state patterns. Overview distinguishes empty versus pending winner states. Top distinguishes no data, no run, filtered-empty, and top-k-disabled situations. Pareto distinguishes missing data versus no optimization frontier. Drilldown tells the user to select a Top result. Heatmap explains that bins above zero are required.

This is already better than a single generic “no data” message, but the empty-state logic is spread across multiple components and a shared readiness model. If those distinctions drift, users can no longer tell whether they need to load data, rerun optimization, clear filters, enable heatmaps, or simply select a different candidate.

## Current Code Surface

- `web/src/composables/useUiReadiness.ts`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`

## Current Verification Surface

- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- readiness-aware shell tests

## Strengths Already Present

- the app already uses more than one empty-state message across result tabs
- Top already has the richest distinction between missing conditions and filtered states
- action-oriented handoffs already exist in several panels
- readiness logic is already centralized enough to support consistent distinctions
- status-line surfaces already exist for advisory follow-up text

## Weaknesses / Failure Modes

- empty-state truth is distributed across several panels and can drift if one panel changes wording or readiness interpretation
- some states are visually similar even when their required next action is different
- panel-specific prerequisites can be obscured if generic copy replaces specific guidance
- missing-module, skipped, stale, and not-run conditions can blur together without a dedicated contract
- there is no dedicated implementation doc freezing the current distinction vocabulary

## Regressions To Avoid

- collapsing distinct empty states into one generic message
- losing action-oriented guidance about what the user should do next
- mislabeling filtered-empty or top-k-disabled states as “no results”
- hiding panel-specific prerequisites such as Top selection or Heatmap bins
- moving copy away from live status regions where dynamic updates are currently announced

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because each results tab should explain its own unavailable state without confusing the overall tab system.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because empty-state and readiness text is advisory dynamic information that should not steal focus.
- [MDN Accessibility: HTML](https://developer.mozilla.org/en-US/docs/Learn/Accessibility/HTML) is relevant because clear language and semantic structure matter most when a panel has no substantive chart or table content yet.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because readiness distinctions should come from shared workflow and analysis state instead of panel-local guesses.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because empty-state correctness is mostly a state-mapping contract that benefits from deterministic tests.

## Proposed Improvement Approach

1. Define a shared empty-state vocabulary across results tabs: no data, no run, filtered empty, disabled by settings, selection required, and unavailable for this run type.
2. Keep panel-specific prerequisites explicit instead of replacing them with generic copy.
3. Treat next-action guidance as part of the contract for each state, not optional supporting text.
4. Align empty-state wording with `useUiReadiness` and analysis-store truth rather than component-local heuristics.
5. Add or tighten tests around the highest-risk distinctions, especially Top filter states and chart-panel prerequisites.

## Checklist

- [ ] Document the shared empty-state vocabulary used across result tabs.
- [ ] Document panel-specific prerequisite states for Overview, Top, Pareto, Drilldown, and Heatmap.
- [ ] Document when a state should include an explicit next action.
- [ ] Document where live status text should remain attached to dynamic empty-state changes.
- [ ] Add or tighten tests for distinct empty-state transitions across the result tabs.

## Verification Plan

- run `npm --prefix web test -- src/components/OverviewWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/TopWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/ParetoWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/DrillWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/HeatmapWorkspacePanel.test.ts`
- manually verify panel-specific next-action guidance with no data, loaded data, and completed runs

## Rollout / Fallback / Compatibility Notes

- preserve current panel-specific specificity rather than flattening to a shared generic copy block
- if a new readiness state is introduced later, update every affected results panel in one change
- keep status-line and empty-state language synchronized so one does not contradict the other
- prefer additive clarification over removing existing actionable hints

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/15_dataset_stale_provenance_cues_DATE.md`
