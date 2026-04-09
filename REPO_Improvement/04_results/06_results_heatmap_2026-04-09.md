# Results Heatmap Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Heatmap panel so bin-based optimizer heatmaps, legend/range cues, export behavior, and Top-to-Heatmap drilldown remain readable, truthful, and accessible enough for real analysis work.

## Scope Boundaries

In scope:

- `web/src/components/HeatmapWorkspacePanel.vue`
- heatmap refresh/restore/render logic in `analysisService`
- legend, palette, range, marker, and crosshair behavior
- heatmap export-PNG control and status messaging

Out of scope:

- underlying optimizer heatmap generation semantics
- Pareto and Drilldown panels
- options-related heatmap surfaces

## Current User-Facing Behavior

The Heatmap panel already communicates prerequisites and current state. It tells users that heatmap bins must be above zero, publishes heatmap meta and status lines, exposes an Export PNG action, and provides explicit instructions that Top Results remains the non-pointer path for choosing another candidate.

The analysis service also does more than draw colored cells. It restores a visible heatmap when possible, rebuilds heatmap state from the current workflow run when needed, computes value ranges and palette mappings, renders a legend, tracks marker/crosshair behavior, and supports overlay and tooltip state.

That makes Heatmap a compound evidence surface with rendering, status, selection handoff, and export semantics.

## Current Code Surface

- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/HeatmapWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- heatmap-related artifact or report tests where heatmap shape changes

## Strengths Already Present

- the panel already communicates the prerequisite that heatmap bins must be enabled
- legend and range cues already exist alongside the bitmap visualization
- Top-results handoff is already explicit, which protects keyboard-driven selection flow
- the panel already exposes export behavior rather than hiding it in another surface
- analysis-service state already tracks more than a bare pixel buffer

## Weaknesses / Failure Modes

- heatmap rendering is canvas-heavy and can lose meaning if legend/range/meta text weakens
- readiness can fail for several different reasons unless status text stays specific
- marker and crosshair behavior can drift from the selected candidate if render state is not rebuilt carefully
- export availability can become misleading if it is detached from actual rendered state
- there is no dedicated implementation doc freezing what the panel must explain in addition to what it draws

## Regressions To Avoid

- rendering a heatmap without truthful prerequisite and status messaging
- weakening legend or range cues around the bitmap surface
- breaking Top-to-Heatmap drilldown expectations or selected-candidate markers
- exposing Export PNG when there is no valid rendered heatmap
- hiding empty-state causes behind a generic blank chart
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tabs Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because Heatmap is a focused analysis tab with a narrow responsibility.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because prerequisite, legend, and readiness messages are advisory live information.
- [Basic usage of canvas | MDN](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Basic_usage) is relevant because the heatmap is rendered on canvas and needs alternative explanatory support around it.
- [Canvas element accessibility guidance | MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/canvas) is relevant because a bitmap heatmap is not semantically self-describing.
- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because the panel explicitly relies on the Top results table as the keyboard-friendly selection partner for the visual heatmap.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Freeze Heatmap as a visual optimizer-evidence panel paired with Top Results for selection.
2. Treat prerequisite text, heatmap meta, legend labels, and status lines as first-class evidence, not optional decoration.
3. Document marker, crosshair, and selected-candidate semantics separately from color rendering.
4. Keep Export PNG tied to truthful rendered-state availability.
5. Add or tighten tests around prerequisite messaging, readiness, Top-results handoff, and export gating.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document heatmap prerequisites and empty-state distinctions.
- [ ] Document legend, range, palette, and meta responsibilities.
- [ ] Document marker/crosshair behavior and selected-candidate expectations.
- [ ] Document Export PNG availability rules.
- [ ] Add or tighten tests for readiness, handoff, and export gating.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/HeatmapWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- manually verify no-heatmap, ready-heatmap, and export-enabled states with a representative run
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current prerequisite text until all heatmap-generation routes and UI cues migrate together
- keep Top Results as the fallback navigation and selection surface even if direct cell interaction improves later
- prefer additive explanatory cues around the heatmap instead of reducing text to chart-only output
- if palette or range behavior changes, update legend/status tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/12_out_pareto_heatmap_readability_2026-04-09.md`
- `REPO_Improvement/04_results/19_chart_rendering_composables_2026-04-09.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
