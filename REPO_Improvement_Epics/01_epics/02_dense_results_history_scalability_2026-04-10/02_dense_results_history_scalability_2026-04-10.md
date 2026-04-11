# Dense Results And History Scalability Plan

Last updated: 2026-04-10

## Purpose

Define a measured epic to keep dense result surfaces and history/library tables responsive under heavier real-world loads without weakening readability, keyboard access, or table-truth semantics.

## Scope Boundaries

In scope:

- dense matrix-result behavior for Top and Validation
- dense library/list behavior for History
- large-run winner/baseline review behavior for Overview
- viewport ownership, virtualization contracts, and dense-scroll correctness
- representative dense fixtures and browser-side scale journeys
- keyboard, focus, and a11y behavior on virtualized dense tables

Out of scope:

- changing result semantics or validation verdict semantics
- artifact-library backend indexing except where it directly shapes dense-table load volume
- broad shell redesign unrelated to dense-surface scale
- options-related dense surfaces

## Current User-Facing Behavior

The repo already has several dense surfaces, but they are not one uniform “large table” family. Top and Validation are the clearest matrix-style dense surfaces. History is a saved-run/library interaction surface with its own store-driven behavior. Overview is a winner/baseline review surface that may appear under large-run context but is not the same interaction model as Top or Validation. Execution is useful as a mechanical virtualization reference, not as a semantic interaction template.

That means the product can be functionally correct while still degrading under larger shortlist, validation, and history loads if viewport ownership, scroll-host semantics, preserved state, and virtualization assumptions drift out of sync.

## Current Code Surface

- `web/src/App.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/composables/useShellMount.ts`
- `web/src/composables/useVirtualTableRows.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/components/ValidationWorkspacePanel.virtualization.test.ts`
- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/composables/useHeadlessMatrixTable.test.ts`
- `web/src/composables/useVirtualTableRows.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/stores/historyStore.search.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `npm.cmd --prefix web run test:browser:ux`
- `npm.cmd --prefix web run a11y:smoke`
- `npm.cmd --prefix web run baseline:ui`

## Surface Contract Focus

- Top Results: dense matrix review with selection, keyboard movement, and virtual-row truth requirements
- Validation Results: dense matrix review with persisted column posture and virtual-row truth requirements
- History: dense saved-run/library list behavior with search, selected-run state, and reopen correctness
- Overview: large-run winner/baseline review that must stay readable without being forced into the same interaction contract as Top/Validation
- Execution: shared-helper reference and regression surface only unless a narrower follow-up widens scope

## Strengths Already Present

- the repo already has a reusable headless-table and virtual-row pattern
- component-level test coverage exists on the main result surfaces
- browser UX and a11y smoke lanes already exist and can be extended with dense fixtures

## Weaknesses / Failure Modes

- virtualization can look correct in synthetic tests while still binding to the wrong scroll host in real shell layouts
- dense tables can regress keyboard/focus behavior even when row rendering is faster
- panel-specific table behavior can drift if each surface owns its own scroll/height assumptions
- history/library and result tables can regress separately if fixture sizes remain too small
- shared helper changes can regress Meta or Execution even when they are not primary implementation targets

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
   - large-run Overview review where row density is not the only cost
2. Define a per-surface contract instead of one flattened table contract:
   - scroll owner
   - interaction role
   - preserved state
   - `rowHeightPx` / `overscan` assumptions
   - whether stable row identity is required before claiming focus restoration
3. Standardize viewport ownership where shared behavior is truly shared instead of letting each panel infer scroll behavior differently.
4. Add representative large fixtures so virtualization proofs stop depending on tiny or synthetic row counts.
5. Add browser proofs that validate scroll-host ownership, visible-row truth, and focus behavior under a realistically heavy surface.
6. Keep success criteria explicit:
   - better responsiveness on the chosen dense scenarios
   - no a11y regressions
   - no focus/selection regressions

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the dense fixtures and target row counts for Top, Validation, and History surfaces, including the source of each fixture and any wide-table pressure.
- [ ] Record current responsiveness and scroll-host ownership before changing implementation.
- [ ] Standardize which element owns scrolling and height constraints for each dense panel or state explicitly why a panel remains intentionally different.
- [ ] Record the per-surface contract for Top, Validation, History, and Overview: role model, preserved state, scroll owner, and required proof.
- [ ] Reuse the existing virtual-row pattern where it is already proven instead of inventing new table mechanics per panel.
- [ ] Strengthen component tests so they validate real viewport ownership and not only synthetic row skipping.
- [ ] Add browser proof for scroll-host ownership and visible-row truth on every changed dense interaction family.
- [ ] Verify keyboard navigation, focus restoration, and screen-reader-visible structure after virtualization or layout changes, and require stable row identity before claiming focus-restoration guarantees across reorder/filter changes.
- [ ] Keep preserved state behaviorally unchanged per surface: Top review state, Validation column posture, History search/selection state, and Overview review context unless explicitly widened.
- [ ] Record rollback conditions if a dense-surface optimization makes debugging, visible-row truth, or accessibility materially worse.
- [ ] Record any required cross-doc sync in first-tier shell/results/history docs if operator behavior changes.

## Verification Plan

- rerun targeted Vitest component coverage for each touched dense panel
- rerun targeted helper and store tests when shared dense helpers or history owners change
- rerun `npm.cmd --prefix web run test:browser:ux`
- rerun `npm.cmd --prefix web run a11y:smoke`
- rerun `npm.cmd --prefix web run baseline:ui` if visible table layout or density changes
- run the relevant repo quality gates for any touched surfaces, including `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck`
- compare pre/post dense-scenario timings or trace evidence for the selected fixtures
- capture concrete acceptance evidence for closure, including fixture sizes, browser proof of the bound scroll element, visible-row proof, and any screenshots needed to prove scroll-host ownership

## Rollout / Fallback / Compatibility Notes

- land viewport-ownership fixes before deeper virtualization changes when the current scroll host is ambiguous
- prefer one shared dense-table pattern over per-panel improvisation only where the interaction model is genuinely shared
- if `useHeadlessMatrixTable.ts` or `useVirtualTableRows.ts` changes, regression-check Meta and Execution even when they are not primary implementation targets
- keep the current dense surfaces functional for smaller datasets even if a large-load optimization is later rolled back
- do not widen this epic into artifact-library backend indexing except where that load directly shapes visible dense-surface behavior

## Related Docs

- `REPO_Improvement/01_shell/17_ux_table_virtualization_2026-04-09.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/06_fe_dense_table_virtualization_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/07_fe_a11y_smoke_2026-04-09.md`
