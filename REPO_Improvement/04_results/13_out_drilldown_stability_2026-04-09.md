# Drilldown And Stability Output Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the visible Drilldown and Stability output so deeper proof surfaces remain readable, properly scoped, and clearly tied to the currently selected candidate.

## Scope Boundaries

In scope:

- visible Drilldown output hierarchy
- stability section visibility and explanatory status/meta lines
- trade-table and trade-stat output clarity
- candidate-selection and proof-depth cues around Drilldown

Out of scope:

- shortlist selection mechanics in Top results
- validation outputs
- options-related drill or stability surfaces

## Current User-Facing Behavior

The Drilldown panel already renders several proof layers at once: the price-and-trades chart, equity and drawdown envelopes, a stability section, and a bounded trade table. It also keeps explicit copy telling the user to select a Top row first and preserves a visible return path to Top results.

The visible output therefore already implies a proof narrative: choose a candidate in Top, inspect deeper behavior in Drilldown, then use stability and trade evidence to decide whether the candidate is robust enough to keep.

The risk is not that Drilldown is empty. The risk is that visible proof output becomes inconsistent, partial, or visually dense without enough explanation to tell the user what is actually being shown.

## Current Code Surface

- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/composables/useLightweightChart.ts`

## Current Verification Surface

- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`

## Strengths Already Present

- the panel already includes multiple proof layers instead of a single chart
- stability and trade output already exist as visible sections
- the panel already states that Top results is the source of candidate selection
- status and meta lines already exist to frame the visible proof
- bounded trade-table behavior already keeps the surface from exploding unboundedly

## Weaknesses / Failure Modes

- multiple proof sections can create false completeness if some sections fail silently
- the visible output hierarchy can become chart-heavy and explanation-light
- stability output can be misread if status/meta lines are reduced or hidden
- bounded trade-table output can hide truncation and selection semantics
- users can mistake a currently selected candidate’s output for general strategy truth

## Regressions To Avoid

- hiding stability or trade-status context while still presenting the output as ready
- making Drilldown look like a primary selection surface rather than a proof surface
- removing visible guidance about candidate selection source
- weakening trade-table or stability-section labeling
- presenting partial or truncated output without explanation

## External Research Summary

- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) is relevant because multi-chart analytical output needs explicit framing and labeling to remain readable.
- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role) is relevant because drill and stability status lines are visible proof cues.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because output adequacy has to be assessed through rendered proof surfaces, not internal drill state alone.
- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because Drilldown must remain a coherent specialized panel in the overall analysis sequence.

## Proposed Improvement Approach

1. Freeze Drilldown as a proof surface with a deliberate visible hierarchy.
2. Document the minimum visible explanation required for each proof section.
3. Keep stability and trade-level output clearly identified as candidate-specific evidence.
4. Preserve selection-source guidance and Top handoff so the panel does not become a substitute shortlist.
5. Add or tighten tests whenever visible proof sections or their framing text change.

## Checklist

- [ ] Document the visible hierarchy of Drilldown proof sections.
- [ ] Document stability status/meta output requirements.
- [ ] Document bounded trade-table and trade-stat output expectations.
- [ ] Document Top-selection dependency and handoff behavior.
- [ ] Add or tighten tests for visible proof-section presence and guidance.

## Verification Plan

- run `npm --prefix web test -- src/components/DrillWorkspacePanel.test.ts`
- run targeted `analysisService` tests if stability or trade output changes
- manually review Drilldown on a candidate with visible stability and trade sections
- capture screenshots if section hierarchy or explanatory status text changes

## Rollout / Fallback / Compatibility Notes

- preserve the current proof-first Drilldown structure even if richer visuals are added later
- prefer additive explanation over removing existing status or section framing
- if some proof sections become optional later, document the output-state distinction explicitly
- keep visible Drilldown output compatible with both fresh-run and hydrated-run flows

## Related Docs

- `REPO_Improvement/04_results/05_results_drilldown_2026-04-09.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/19_chart_rendering_composables_2026-04-09.md`
- `REPO_Improvement/07_strats/14_analysis_drilldown_stability_2026-04-08.md`
