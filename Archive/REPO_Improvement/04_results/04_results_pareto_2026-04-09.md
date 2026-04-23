# Results Pareto Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Pareto results panel so axis selection, frontier rendering, chart guidance, and Top-to-Pareto handoff remain accurate, readable, and useful for multi-objective review.

## Scope Boundaries

In scope:

- `web/src/components/ParetoWorkspacePanel.vue`
- Pareto axis selectors and fallback option behavior
- Pareto chart metadata, empty states, and top-results handoff
- Pareto rendering and refresh logic in `analysisService`

Out of scope:

- non-Pareto overview or shortlist rendering
- detailed drilldown charts
- options-related Pareto surfaces

## Current User-Facing Behavior

The Pareto panel already behaves like a dedicated frontier-review surface. It exposes X/Y axis selectors, refreshes from the current workflow run when mounted or revisited, and shows status/meta text describing the chosen metrics and plotted range.

When results are unavailable, the panel already distinguishes between “no data” and “no run” states. When results exist, it still includes a strong handoff back to Top Results, explicitly telling the user that Top is the keyboard-friendly way to move between candidates while the chart is better suited to direct point interaction.

The analysis service also supports fallback axis-option behavior and builds the visible frontier from optimization results rather than relying on static chart-only state.

## Current Code Surface

- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- shell/workflow tests that cover results-tab navigation

## Strengths Already Present

- the panel already separates frontier review from shortlist navigation
- axis selectors already tolerate missing option lists by using resolved fallback options
- empty-state language already distinguishes missing prerequisites from missing results
- the chart already publishes human-readable range metadata
- the panel already keeps a clear handoff back to Top Results

## Weaknesses / Failure Modes

- axis validity depends on current result shape and can drift if option rebuilding is incomplete
- canvas-based frontier rendering is harder to keep accessible than table-based review
- point interaction can become misleading if users do not understand that Top remains the canonical shortlist navigator
- the current panel can remain visually empty for several different reasons unless meta/status text stays precise
- there is no dedicated implementation doc freezing what the panel must explain versus what the chart merely draws

## Regressions To Avoid

- breaking axis-option fallback and leaving the panel with unusable selectors
- hiding the Top-results handoff that currently preserves keyboard-friendly navigation
- rendering a frontier without truthful meta/range text
- overstating Pareto readiness when the run does not actually contain frontier data
- weakening empty-state distinctions between missing data, no run, and missing frontier points
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because Pareto is one analysis tab in a larger results system and needs a clear tab-specific responsibility.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because range metadata and readiness messages are advisory updates that should not steal focus.
- [Basic usage of canvas | MDN](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Basic_usage) is relevant because the Pareto chart is canvas-driven and should retain alternative explanatory content around the bitmap surface.
- [Canvas element accessibility guidance | MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/canvas) is relevant because a canvas alone does not expose enough semantic meaning for chart review.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because axis choice, frontier data, and status text should stay in shared state rather than fragment across the panel and chart helpers.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Freeze Pareto as the frontier-review panel, with Top remaining the canonical shortlist-navigation panel.
2. Document axis-selector validity, fallback option behavior, and how defaults are restored when result metrics change.
3. Treat range/meta/status text as essential evidence, not as optional chart chrome.
4. Preserve alternative guidance around the canvas so keyboard-only and assistive-technology users can still understand the panel’s state and next action.
5. Add or tighten tests around axis-option rebuilding, empty-state language, and Top-results handoff.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document Pareto panel responsibility versus Top Results responsibility.
- [x] Document axis selector validity, fallback options, and default-selection rules.
- [x] Document frontier meta/range text and readiness behavior.
- [x] Document canvas accessibility support and explanatory fallback expectations.
- [x] Add or tighten tests for axis changes, empty states, and Top-results handoff.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/ParetoWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- manually verify no-data, no-run, and ready Pareto states with different axis choices
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve Top Results as the fallback navigation path even if direct point interaction improves later
- keep existing axis ids and meta semantics stable until all consumers migrate together
- prefer additive explanatory text over silent chart-only changes
- if a richer chart library is adopted later, keep current readiness and handoff behavior intact first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the Pareto panel as the current frontier-review surface with Top Results remaining the shortlist-navigation fallback. `ParetoWorkspacePanel.test.ts` and `analysisService.test.ts` remain green, which keeps axis fallback, readiness language, and Top-results handoff behavior aligned with the documented contract.
- No code change was required in this tranche because the current Pareto panel responsibilities and state/readiness behavior already satisfy the plan at the current scope.

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/07_results_empty_states_2026-04-09.md`
- `REPO_Improvement/04_results/12_out_pareto_heatmap_readability_2026-04-09.md`
- `REPO_Improvement/04_results/19_chart_rendering_composables_2026-04-09.md`
