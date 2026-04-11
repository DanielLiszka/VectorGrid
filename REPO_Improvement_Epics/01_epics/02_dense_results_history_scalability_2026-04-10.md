# Dense Results And History Scalability Plan

Last updated: 2026-04-10

## Purpose

Define a measured epic to keep dense result surfaces and history/library tables responsive under heavier real-world loads without weakening readability, keyboard access, or table-truth semantics.

## Scope Boundaries

In scope:

- large Top, Validation, Overview, and History table behavior
- viewport ownership, virtualization thresholds, and dense-scroll correctness
- representative dense fixtures and browser-side scale journeys
- keyboard, focus, and a11y behavior on virtualized dense tables

Out of scope:

- changing result semantics or validation verdict semantics
- artifact-library backend indexing except where it directly shapes dense-table load volume
- broad shell redesign unrelated to dense-surface scale
- options-related dense surfaces

## Current User-Facing Behavior

The repo already has several dense result surfaces and at least one clear virtualization pattern in the execution workspace. The first-tier work also hardened some of the Top and Validation virtualization behavior, but the broader dense-surface story remains spread across multiple panels with different ownership patterns.

That means the product can be functionally correct while still degrading under larger shortlist, validation, and history loads if viewport ownership, scroll-host semantics, and virtualization assumptions drift out of sync.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/composables/useVirtualTableRows.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`

## Current Verification Surface

- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/components/ValidationWorkspacePanel.virtualization.test.ts`
- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run a11y:smoke`
- `npm.cmd --prefix web run baseline:ui`

## Strengths Already Present

- the repo already has a reusable headless-table and virtual-row pattern
- component-level test coverage exists on the main result surfaces
- browser UX and a11y smoke lanes already exist and can be extended with dense fixtures

## Weaknesses / Failure Modes

- virtualization can look correct in synthetic tests while still binding to the wrong scroll host in real shell layouts
- dense tables can regress keyboard/focus behavior even when row rendering is faster
- panel-specific table behavior can drift if each surface owns its own scroll/height assumptions
- history/library and result tables can regress separately if fixture sizes remain too small

## Regressions To Avoid

- making tables faster by weakening visible-row truth, selection persistence, or keyboard access
- mixing large visual refactors with dense-surface performance changes
- introducing virtualization that depends on unstable layout assumptions or shell-overflow accidents
- letting browser-only tests substitute for targeted component and viewport-ownership tests
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Introduction | TanStack Virtual Docs](https://tanstack.com/virtual/latest/docs/introduction)
  - relevant because the repo already uses a headless virtualization approach and should keep scroll-element ownership explicit
- [Virtualize large lists with react-window | web.dev](https://web.dev/articles/virtualize-long-lists-react-window)
  - relevant because dense-table work needs a clear mental model for why only visible rows should render
- [content-visibility | MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/content-visibility)
  - relevant because some dense-surface wins may come from reducing off-screen layout work even outside strict row virtualization

## Proposed Improvement Approach

1. Define the dense scenarios that matter:
   - large Top shortlist
   - large Validation candidate table
   - large History/library lists
   - mixed shell layouts where the outer pane also scrolls
2. Standardize table viewport ownership and row-height contracts across the dense panels instead of letting each panel infer scroll behavior differently.
3. Add representative large fixtures so virtualization proofs stop depending on tiny or synthetic row counts.
4. Add one browser dense-surface journey that validates scroll, focus, and visible-row truth under a realistically heavy surface.
5. Keep success criteria explicit:
   - better responsiveness on the chosen dense scenarios
   - no a11y regressions
   - no focus/selection regressions

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the dense fixtures and target row counts for Top, Validation, and History surfaces.
- [ ] Record current responsiveness and scroll-host ownership before changing implementation.
- [ ] Standardize which element owns scrolling and height constraints for each dense panel.
- [ ] Reuse the existing virtual-row pattern where it is already proven instead of inventing new table mechanics per panel.
- [ ] Strengthen component tests so they validate real viewport ownership and not only synthetic row skipping.
- [ ] Add at least one browser dense-surface journey that catches scroll/focus regressions under real load.
- [ ] Verify keyboard navigation, focus restoration, and screen-reader-visible structure after virtualization or layout changes.
- [ ] Keep row selection, sort order, and visible summary copy behaviorally unchanged unless explicitly widened.
- [ ] Record rollback conditions if a dense-surface optimization makes debugging or accessibility materially worse.
- [ ] Record any required cross-doc sync in first-tier shell/results/history docs if operator behavior changes.

## Verification Plan

- rerun targeted Vitest component coverage for each touched dense panel
- rerun `npm.cmd --prefix web run test:browser:ux`
- rerun `npm.cmd --prefix web run a11y:smoke`
- rerun `npm.cmd --prefix web run baseline:ui` if visible table layout or density changes
- run the relevant repo quality gates for any touched surfaces, including `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck`
- compare pre/post dense-scenario timings or trace evidence for the selected fixtures
- capture concrete acceptance evidence for closure, including fixture sizes, browser journey output, and any screenshots needed to prove scroll-host ownership

## Rollout / Fallback / Compatibility Notes

- land viewport-ownership fixes before deeper virtualization changes when the current scroll host is ambiguous
- prefer one shared dense-table pattern over per-panel improvisation
- keep the current dense surfaces functional for smaller datasets even if a large-load optimization is later rolled back
- do not widen this epic into artifact-library backend indexing except where that load directly shapes visible dense-surface behavior

## Related Docs

- `REPO_Improvement/01_shell/17_ux_table_virtualization_2026-04-09.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/06_fe_dense_table_virtualization_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/07_fe_a11y_smoke_2026-04-09.md`
