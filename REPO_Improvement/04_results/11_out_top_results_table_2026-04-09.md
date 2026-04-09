# Output Top Results Table Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Top-results output surface so the shortlist table remains a decision-grade triage tool with clear ranking, candidate context, filter truth, and compare-ready actions.

## Scope Boundaries

In scope:

- `web/src/components/TopWorkspacePanel.vue`
- shortlist output shown in the ranked table, compare tray, and selected-candidate review card
- filter meta, stale cues, guardrails, and selected-candidate summaries
- analysis-store and analysis-service state that feed the table output

Out of scope:

- Overview winner framing
- drilldown, Pareto, and heatmap rendering internals
- options-related top-result tables

## Current User-Facing Behavior

The Top panel already functions as the shortlist triage surface for the app. It exposes ranked rows, filters, selected-candidate review copy, stale cues, guardrails, pinning into compare, and actions that hand off to Overview or Drilldown.

It is also the keyboard-friendliest results surface. Pareto, Drilldown, and Heatmap all explicitly tell the user to return to Top Results when they need structured candidate navigation instead of direct chart interaction. That makes the top table more than a list; it is the core navigational output surface for ranked candidates.

## Current Code Surface

- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/useVirtualTableRows.ts`

## Current Verification Surface

- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- virtualization and shell tests that cover results-tab behavior

## Strengths Already Present

- the table already supports dense candidate review with filters and selection
- selected-candidate framing already exists beside the table, not just inside rows
- stale cues and guardrails already explain when the shortlist is incomplete or constrained
- compare-ready actions already sit next to the shortlist workflow
- the table already anchors other result tabs as the main navigation surface

## Weaknesses / Failure Modes

- output clarity depends on several sub-surfaces staying aligned: table rows, selected-candidate review, stale cues, and compare tray
- dense output can become harder to trust if filter state and shortlist guardrails are not visible enough
- virtualization and column hiding can accidentally obscure critical decision metrics
- the table can degrade into a presentation of scores without enough row identity and context
- there is no dedicated implementation doc freezing what the Top output surface must show to remain analyst-usable

## Regressions To Avoid

- reducing the visibility of stale cues, guardrails, or filter meta
- hiding important ranking context behind overly aggressive column reduction
- weakening keyboard navigation or row-to-review handoff behavior
- detaching the selected-candidate review card from the actual row state
- treating the table as just another tab instead of the central ranked-navigation surface
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Grid Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) is relevant because this is a dense interactive table that must remain keyboard-coherent and navigable.
- [Table Pattern | WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/patterns/table/) is relevant because the panel still needs to preserve clear tabular meaning even with richer interactions.
- [ARIA: status role | MDN](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Roles/status_role) is relevant because selected-candidate summaries, stale cues, and guardrails are dynamic advisory information.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because shortlist output, selection state, and compare state must stay driven by shared state.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because much of the value here comes from deterministic state transitions, not static markup alone.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Freeze Top Results as the canonical ranked-navigation and shortlist-triage output surface.
2. Document the output in layers: ranked table, selected-candidate review card, stale/guardrail messaging, and compare tray.
3. Keep filter meta and shortlist guardrails visible enough to explain why the user is or is not seeing specific rows.
4. Treat keyboard navigation and compare readiness as core output quality, not secondary polish.
5. Add or tighten tests around selected-candidate summaries, stale cues, compare output, and handoff actions.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the Top table’s role as the canonical ranked-navigation surface.
- [ ] Document selected-candidate review output and its coupling to row selection.
- [ ] Document stale cues, guardrails, and filter-meta requirements.
- [ ] Document compare-ready output and handoff actions into Overview and Drilldown.
- [ ] Add or tighten tests for shortlist output clarity, selection, compare, and navigation behavior.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/TopWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/analysisService.test.ts`
- manually verify shortlist readability with filters, hidden columns, pinned rows, and keyboard navigation
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve Top Results as the canonical shortlist surface even if other result tabs get richer
- keep current table-state keys and selected-candidate output behavior stable until all consumers migrate together
- prefer additive row/context evidence over flattening the table into a score-only grid
- if dense-table behavior changes later, update virtualization and selection tests in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/03_results_top_compare_2026-04-09.md`
- `REPO_Improvement/04_results/09_results_analysis_contract_2026-04-09.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
