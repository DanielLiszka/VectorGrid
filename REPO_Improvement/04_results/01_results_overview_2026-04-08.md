# Results Overview Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the Overview results panel so winner framing, empty states, execution-risk cues, baseline comparison, and next-step guidance stay focused and useful instead of becoming a generic duplication of the top-results table.

## Scope Boundaries

In scope:

- `web/src/components/OverviewWorkspacePanel.vue`
- overview-stage empty/pending/ready states
- winner card and baseline review surface
- execution-risk notice
- overview actions and summary chips

Out of scope:

- detailed ranked-table behavior
- Pareto/heatmap/drilldown panels
- options result surfaces

## Current User-Facing Behavior

The Overview panel already acts as a winner-framing surface rather than a full ranking surface. Current behavior includes:

- empty state when no results are available
- pending state when data is loaded but no analyzed winner exists yet
- ready state showing:
  - winner title/kicker/meta
  - best-stat cards
  - winner details
  - execution-risk notice when applicable
  - next-step guidance to review Top Results
  - baseline comparison controls and summary
  - export/copy actions

The panel already distinguishes “review the winner” from “triage the shortlist.”

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/composables/useUiReadiness.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/stores/workflowStore.ts`

## Current Verification Surface

- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- analysis-store and analysis-service tests that feed overview state

## Strengths Already Present

- the panel already has a clear role distinct from Top Results
- empty and pending states already exist
- execution-risk guidance already ties result review to downstream actionability
- baseline comparison already lives in Overview rather than cluttering the top table
- actions already support copy/export and handoff to Top Results

## Weaknesses / Failure Modes

- overview meaning can drift if it starts absorbing too much shortlist behavior
- empty-state and pending-state copy could diverge from actual readiness semantics
- winner framing depends on several stores, which makes ownership easy to blur
- execution-risk cues can become stale if not kept aligned with the active run

## Regressions To Avoid

- turning Overview into a second top-results panel
- losing the distinction between no-data, no-run, and ready states
- removing execution-risk cues when they are still relevant
- making baseline comparison less visible or less truthful
- surfacing stale winner framing after run changes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because Overview is one panel in a larger results tab system and should remain a distinct, coherent tabpanel
- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because overview status lines and empty/pending-state copy are live explanatory status
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because the panel should be verified through visible winner framing and guidance, not hidden implementation state
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because Overview is a coordinated reader of shared result/history/workflow state
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Document Overview as a dedicated winner-framing panel with three clear states:
   - empty
   - pending
   - ready
2. Keep the panel’s responsibilities narrow:
   - winner framing
   - execution-risk guidance
   - baseline comparison
   - next-step handoff
3. Treat Overview copy, chips, and action placement as contract items, not incidental layout.
4. Align overview-stage logic with `useUiReadiness` and analysis-store truth.
5. Expand tests around state transitions, baseline controls, and winner handoff to Top Results.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document empty, pending, and ready Overview states.
- [ ] Document winner-card fields and their intended role.
- [ ] Document execution-risk notice behavior and when it should appear.
- [ ] Document baseline comparison controls and summary behavior.
- [ ] Document Overview-to-Top handoff actions.
- [ ] Add tests for empty/pending/ready state transitions.
- [ ] Add tests for execution-risk notice visibility.
- [ ] Add tests for baseline review and export/copy actions.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/components/OverviewWorkspacePanel.test.ts`
- run relevant shell and analysis tests that feed overview state
- manually verify no-data, no-run, and ready states in the results column
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current focused role of Overview rather than widening it into a general results dashboard
- keep Top Results as the shortlist-triage surface and Overview as the winner-framing surface
- if additional winner diagnostics are added later, keep the current state model intact first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/04_results/02_results_top_table_2026-04-08.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`

