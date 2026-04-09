# Results Evidence Gaps And Missing Data Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to evidence-gap and missing-data messaging so result surfaces distinguish unavailable evidence, skipped work, incomplete runs, and truly absent data without misleading users.

## Scope Boundaries

In scope:

- result-surface status lines, guardrails, and advisories
- missing-evidence messaging across Overview, Top, Pareto, Drilldown, and Heatmap
- current-run versus partial-run gaps that affect decision confidence
- user-facing cues for unavailable compare, shortlist, chart, or report evidence

Out of scope:

- validation workbook content
- artifact-library browsing outside result-surface implications
- options-related evidence messaging

## Current User-Facing Behavior

The current results workflow already carries several kinds of missing-evidence messaging. Overview can show “no analyzed winner yet.” Top can explain that `top_k = 0` returned only the best candidate. Drilldown can say a Top row must be selected first. Heatmap explicitly depends on `heatmap_bins > 0`. Compare review explains when a baseline is missing or unavailable.

This is already better than a generic “no data” message, but the evidence story is still distributed. Some gaps are caused by workflow configuration, some by missing saved-run artifacts, some by selection state, and some by the fact that a given run simply did not produce that surface.

That means evidence-gap messaging is a real decision-support concern: users need to know whether a surface is absent because they have not run something, because they configured it away, because they filtered it out, or because a saved artifact does not contain it.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/composables/useUiReadiness.ts`

## Current Verification Surface

- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`

## Strengths Already Present

- the repo already uses more than one missing-state message
- analysis state already tracks guardrails and status lines for constrained results
- chart surfaces already explain prerequisite settings
- compare flows already tell the user when a baseline cannot be resolved
- readiness is already centralized enough to keep some consistency

## Weaknesses / Failure Modes

- missing evidence is still described in several places, which makes drift likely
- user-facing copy can blur “not run” and “not available for this run”
- a blank surface can still appear if a guardrail or status path breaks
- partial saved-run hydration can make a surface look empty without clear provenance
- there is not yet a dedicated implementation plan freezing evidence-gap vocabulary and expectations

## Regressions To Avoid

- collapsing all missing-result cases into generic “no data” language
- hiding configuration-caused gaps such as `top_k = 0` or `heatmap_bins = 0`
- treating missing saved-run content as though the app simply failed to load
- dropping compare or shortlist guardrail messages
- leaving canvases or tables empty without an explanatory status line
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because evidence-gap truth should come from shared state, not panel-local guesses.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because guardrails, status lines, and readiness states already live in shared stores and services.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because most evidence-gap behavior is deterministic and should be locked in via state-driven tests.
- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because users need clear signals when to stay on a tab and when to move to another one for missing evidence.
- [MDN `<canvas>` element reference](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/canvas) is relevant because chart panels need strong textual fallback when the chart has nothing meaningful to render.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Define the result-surface evidence-gap vocabulary explicitly: no data, no run, no selection, configured away, unavailable in artifact, filtered away, and stale.
2. Map each major panel to the evidence gaps it must explain.
3. Keep chart and table surfaces from ever failing silently; they should always carry a status or advisory line when meaningful evidence is absent.
4. Treat compare/baseline missing states as first-class evidence gaps rather than generic errors.
5. Add or tighten tests whenever status copy or guardrail logic changes.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the canonical evidence-gap vocabulary.
- [ ] Document panel-specific evidence-gap cases and required messages.
- [ ] Document configuration-caused gaps separately from true absence of data.
- [ ] Document saved-run and artifact-related evidence gaps.
- [ ] Add or tighten tests for guardrails, status lines, and empty-state messaging.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run representative result-panel component tests
- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- run readiness-related tests if `useUiReadiness` or shared status logic changes
- manually review one current run and one saved artifact replay for evidence-gap truthfulness
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification

## Rollout / Fallback / Compatibility Notes

- preserve current informative messages until replacement copy is verified to be equally explicit
- prefer additive clarification over shortening copy into vagueness
- keep surface-specific guidance intact even if shared readiness logic is refactored
- if a new result surface is added, classify its missing-evidence states using the same vocabulary
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/04_results/07_results_empty_states_2026-04-09.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/15_dataset_stale_provenance_cues_2026-04-09.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
