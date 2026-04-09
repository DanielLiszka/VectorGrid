# Pareto And Heatmap Readability Output Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the visible Pareto and Heatmap chart output so labels, legend guidance, empty-state meaning, and Top-table handoff keep both chart surfaces analytically readable.

## Scope Boundaries

In scope:

- visible Pareto chart output and axis-selector framing
- visible Heatmap chart output, legend, and palette explanation
- output-level readability of supporting copy, labels, and actions
- user guidance that keeps charts secondary to shortlist review

Out of scope:

- raw chart-state derivation in stores and services
- drilldown chart behavior
- options-related chart surfaces

## Current User-Facing Behavior

The Pareto and Heatmap panels already present themselves as optional analytical lenses rather than the main shortlist surface. Both panels include explicit “Review Top results” handoffs. Pareto exposes axis selectors and a Pareto metadata line, while Heatmap exposes a palette explanation, legend, and conditional PNG export.

The visible chart output already contains useful guidance, but readability is still distributed across selector copy, metadata, legend text, and advisory text. That means chart usefulness depends as much on surrounding output clarity as on the rendered canvas itself.

This doc exists to freeze that requirement: the chart image alone is not enough. The visible labels, range explanations, and handoff cues are part of the chart contract.

## Current Code Surface

- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- targeted `analysisService` tests for Pareto and Heatmap refresh behavior

## Strengths Already Present

- both panels already include explicit Top-results handoff
- both panels already have visible metadata or legend context
- Heatmap already explains its palette instead of leaving colors uninterpreted
- Pareto already keeps axis choice visible and editable
- both panels already warn the user when the chart is unavailable

## Weaknesses / Failure Modes

- visible chart guidance can become too thin if future work focuses only on the canvas
- label and legend wording can drift from actual chart semantics
- Pareto axis choices can remain technically valid but analytically weak without output guidance
- Heatmap palette and legend can look self-explanatory when they are not
- the visible chart output can become pointer-first if Top handoff cues lose prominence

## Regressions To Avoid

- hiding chart meaning inside hover-only or tooltip-only cues
- weakening axis/legend/palette explanations in the visible output
- making the chart the only practical way to continue analysis
- collapsing readiness warnings into vague “no chart” output
- presenting chart output that looks authoritative when settings did not actually enable that surface
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) is relevant because chart-fit, labels, and visible scale cues are core readability requirements.
- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because readable chart output also depends on clear panel-level context within the broader results workflow.
- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role) is relevant because chart metadata and readiness cues are part of the visible analytical output.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because readability should be judged by rendered output and user paths, not internal chart state alone.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Treat chart readability as the sum of canvas output plus visible framing copy.
2. Document the minimum visible cues required for Pareto and Heatmap to be understandable.
3. Preserve explicit Top-results handoff so the charts stay optional analytical lenses.
4. Keep legend, palette, and axis cues synchronized with actual chart semantics.
5. Add or tighten tests whenever visible chart guidance or export affordances change.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document minimum visible readability cues for Pareto output.
- [ ] Document minimum visible readability cues for Heatmap output.
- [ ] Document axis, legend, and palette language as part of the output contract.
- [ ] Document Top-results handoff visibility requirements.
- [ ] Add or tighten tests for rendered chart guidance and readiness output.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/ParetoWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/HeatmapWorkspacePanel.test.ts`
- manually review Pareto and Heatmap on recent runs with real data
- capture before/after screenshots if chart framing or legend copy changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification

## Rollout / Fallback / Compatibility Notes

- preserve the current Top-first workflow even if chart output becomes richer
- prefer additive labels and clearer legend guidance over reducing visible analytical context
- if chart palette or axis options evolve later, update visible framing and help text in the same change
- keep readability improvements compatible with both fresh and hydrated result views
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/04_results/04_results_pareto_2026-04-09.md`
- `REPO_Improvement/04_results/06_results_heatmap_2026-04-09.md`
- `REPO_Improvement/04_results/19_chart_rendering_composables_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/05_fe_results_surface_adequacy_2026-04-09.md`
