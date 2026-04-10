# Report Path And Reopen Flow Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to result-surface report-path and reopen flows so users can tell when a report exists, where it lives, and how to reopen or reveal it without leaving the results workflow confused about file ownership.

## Scope Boundaries

In scope:

- result-surface cues that a report or saved artifact exists
- reopen and reveal actions connected to current or saved runs
- history/library output relevant to report reopening
- visible path and availability messaging tied to result review

Out of scope:

- report generation internals
- artifact schema details outside reopen implications
- options-related report flows

## Current User-Facing Behavior

Report and saved-run reopen behavior already exists, but it is split across current-run actions and the History workspace. Results review can lead into exported CSV/JSON output and eventually into saved-run or report-opening actions, while the History surface exposes detail metadata plus actions to load, apply, rerun, open reports, reveal artifacts, and delete saved runs.

That means report reopening is already part of the product, but the visible flow is distributed. Users can reopen or reveal files, yet the result surfaces themselves do not have one explicit implementation doc defining how path and reopen cues should support analysis confidence.

This doc exists to keep that flow truthful: report availability, path ownership, and reopen actions should feel like part of the analysis workflow, not hidden library plumbing.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`

## Current Verification Surface

- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- artifact/report path tests in `REPO_Improvement/06_artifacts` and `10_tests_perf`

## Strengths Already Present

- saved-run and report reopen actions already exist
- the History surface already exposes detailed run metadata and actions
- the workflow already tracks current-run artifact context
- reveal/open/report flows already exist as first-class user actions rather than hidden filesystem assumptions

## Weaknesses / Failure Modes

- report availability is distributed across surfaces and can feel indirect
- users may not know whether a report exists for the currently reviewed result
- visible path ownership can be unclear when current-run and saved-run contexts differ
- reopen and reveal flows can become library-only features instead of part of results trust
- there is no dedicated result-facing doc freezing how report-path cues should be surfaced

## Regressions To Avoid

- hiding report availability behind the History workspace only
- presenting reopen actions without clear ownership of what file will open
- breaking reveal/open flows when saved-run identity changes
- letting result surfaces imply a report exists when only exports or raw artifacts exist
- weakening the connection between current analysis and saved-run reopen behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because reopen flow quality depends on visible actions and user path clarity.
- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role) is relevant because report availability and reopen success should be conveyed as visible status, not inferred silently.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because reopen flows depend on shared current-run and saved-run state rather than component-local file knowledge.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because report reopen behavior depends on stable artifact metadata and path contracts across layers.
- [WAI Tables Tutorial](https://www.w3.org/WAI/tutorials/tables/)
  - relevant because dense results tables and supporting evidence surfaces need semantic structure and scannability in a public-facing product

## Proposed Improvement Approach

1. Document how report existence and reopen availability should surface during result review.
2. Keep current-run and saved-run reopen flows distinct but visibly related.
3. Treat reveal/open/report actions as trust-building workflow steps, not just convenience buttons.
4. Ensure visible messaging explains which artifact or report is being opened or revealed.
5. Add or tighten tests whenever report-availability cues or reopen actions change.
6. Treat results-surface code quality as part of the implementation scope: preserve semantic tables/charts, readable evidence labeling, accessible density, and contributor-friendly rendering/state boundaries.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document current-run versus saved-run report reopen flows.
- [x] Document visible cues that a report exists and is reopenable.
- [x] Document path/reveal/open action semantics in result-review contexts.
- [x] Document fallback behavior when a report path is absent or stale.
- [x] Add or tighten tests for report open and reveal action visibility.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/HistoryWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run targeted history-store action tests if reopen or reveal semantics change
- manually verify current-run and saved-run reopen flows on a real saved artifact set
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched result surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required browser, desktop, or accessibility smoke verification

## Rollout / Fallback / Compatibility Notes

- preserve current reopen and reveal actions while clarifying their visible ownership
- prefer additive path/reopen cues over hiding actions that already work
- if report generation becomes more conditional later, surface that distinction explicitly before changing reopen affordances
- keep History and result-surface messaging aligned whenever artifact-path semantics change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated report-path and reopen behavior through [HistoryWorkspacePanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.test.ts), [historyService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/historyService.test.ts), [historyStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/historyStore.actions.test.ts), and [commandActionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/commandActionStore.test.ts).
- The current evidence confirms current-run fallback report paths, saved-run report open/reveal actions, and stale-path fallback to `reveal_path` are still visible and truthful.
- No production code change was required in this tranche because the current report-path/reopen contract is already satisfied by the existing results/history action surface.

## Related Docs

- `REPO_Improvement/04_results/10_out_overview_decision_surface_2026-04-09.md`
- `REPO_Improvement/06_artifacts/19_report_save_open_reveal_2026-04-09.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
