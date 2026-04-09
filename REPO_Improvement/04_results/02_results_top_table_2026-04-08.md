# Results Top Table Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the Top Results table so ranking, filtering, column visibility, virtualization, selection, pinning, stale cues, and navigation remain analyst-usable and consistent with the current winner workflow.

## Scope Boundaries

In scope:

- `web/src/components/TopWorkspacePanel.vue`
- top-table state in `analysisStore`
- persisted column visibility
- virtualized row rendering
- top-row selection and pinning behavior

Out of scope:

- Overview winner framing
- Pareto/heatmap/drilldown rendering internals
- options tables

## Current User-Facing Behavior

The Top Results panel already exposes more than a simple list. Current behavior includes:

- readiness-aware empty states
- sortable columns
- persisted column visibility
- virtualized row rendering for dense tables
- row selection
- pinning current rows into compare state
- filter inputs for:
  - minimum PnL
  - minimum Sharpe
  - maximum drawdown
  - minimum trades
  - minimum exposure
- stale cues
- selection guardrails
- keyboard navigation for rows
- handoff actions back to Overview or into Drilldown

The panel already acts as the shortlist-triage surface for the workflow.

## Current Code Surface

- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/services/analysisService.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/useVirtualTableRows.ts`
- `web/src/composables/useQueuedInputWriter.ts`

## Current Verification Surface

- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/services/analysisService.test.ts`
- shell tests that verify results-tab behavior

## Strengths Already Present

- the panel already has persisted column visibility
- virtualization is already part of the rendering path
- selection, pinning, and stale cues already exist
- keyboard row navigation is already implemented
- the panel already distinguishes shortlist triage from Overview winner framing

## Weaknesses / Failure Modes

- the top-table contract is wide enough that regressions can hide in small UI changes
- filter state, selection state, and stale-cue state can drift if not documented together
- virtualization and persisted column state are easy to break subtly
- the panel could become harder to use if keyboard and pinning flows diverge from table state

## Regressions To Avoid

- losing persisted column visibility or restore-default behavior
- breaking virtualized row rendering for dense result sets
- decoupling row selection from drilldown/overview handoff
- hiding stale cues or selection guardrails that currently explain data quality/state
- making keyboard navigation less capable than the current row interaction model
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Column Visibility Guide](https://tanstack.com/table/latest/docs/guide/column-visibility)
  - relevant because the current panel already persists and restores visible column subsets
- [Sorting Guide](https://tanstack.com/table/latest/docs/guide/sorting)
  - relevant because column-driven ranking changes are part of the top-table interaction model
- [TanStack Virtual Introduction](https://tanstack.com/virtual/latest/docs/introduction)
  - relevant because the panel already uses virtualization for dense shortlist rendering
- [Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)
  - relevant because a dense interactive results table still needs coherent keyboard and focus behavior
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Document the Top Results panel as a shortlist-triage contract with these sub-surfaces:
   - filters
   - table
   - selection
   - pinning/compare
   - stale cues
   - handoff actions
2. Define which parts of the table state are persisted versus ephemeral.
3. Treat selection and pinning as first-class workflow actions, not incidental row clicks.
4. Lock keyboard interaction and virtualization expectations together so performance improvements do not degrade usability.
5. Expand tests around column persistence, filter guardrails, selection, and pinning.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document filter fields, filter meta, and selection guardrail behavior.
- [ ] Document column visibility persistence and restore-default behavior.
- [ ] Document row selection, pinning, and compare interactions.
- [ ] Document stale-cue and provenance-chip behavior.
- [ ] Document keyboard navigation and drilldown/overview handoff behavior.
- [ ] Add tests for persisted column visibility and restore-default.
- [ ] Add tests for row selection, pinning, and keyboard interaction.
- [ ] Add tests for stale cues, guardrails, and empty-state transitions.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/components/TopWorkspacePanel.test.ts`
- run `npm test -- src/services/analysisService.test.ts`
- manually verify dense shortlist rendering, column toggling, pinning, and drilldown handoff in the UI
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the panel’s shortlist-triage role rather than collapsing it into Overview
- keep persisted column-state behavior backward compatible with existing local state keys where possible
- if deeper table features are added later, keep current keyboard and pinning flows intact first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/04_results/01_results_overview_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/06_fe_dense_table_virtualization_2026-04-09.md`
