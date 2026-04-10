# FE Results Surface Adequacy Plan

Last updated: 2026-04-10

## Purpose

Define the frontend test strategy for results-surface adequacy so overview, top, pareto, heatmap, drilldown, and related result tabs remain readable, truthful, and fit for decision-making.

## Scope Boundaries

In scope:

- results-panel component tests
- results-related browser review expectations
- adequacy of empty states, warnings, and dense result tables

Out of scope:

- backend result generation
- validation workbook exports
- options results surfaces

## Current User-Facing Behavior

Users rely on the built app to interpret optimization and analysis results through multiple panels, tabs, tables, and charts. The repo already has test coverage for many of these surfaces, but adequacy as a user-facing quality bar still spans several files rather than one documented contract.

## Current Code Surface

- `web/src/components/OverviewWorkspacePanel.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ParetoWorkspacePanel.test.ts`
- `web/src/components/HeatmapWorkspacePanel.test.ts`
- `web/src/components/DrillWorkspacePanel.test.ts`
- `web/src/components/AnalysisContextDock.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/components/OptimizeWorkspacePanel.test.ts`
- `web/e2e/browser/shell-ux.spec.ts`

## Current Verification Surface

- `npm --prefix web test`
- targeted results-panel component tests
- `npm --prefix web run test:browser:ux`

## Strengths Already Present

- key result tabs already have direct component coverage
- the repo already has a dedicated results-display adequacy planning lane
- browser smoke already exists for higher-level shell interaction proof

## Weaknesses / Failure Modes

- adequacy is broader than “component renders”
- result surfaces can degrade in readability without obvious functional breakage
- dense tables and chart-heavy views need more explicit quality ownership than generic test presence

## Regressions To Avoid

- treating a rendering smoke as proof that a results surface is still decision-usable
- allowing warnings, empty states, or provenance cues to regress silently
- losing coverage on one results tab because another higher-level smoke still passes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Test Utils guide](https://test-utils.vuejs.org/guide/)
  - relevant because direct panel tests remain the narrowest proof for rendered adequacy cues
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because browser proof should target user-visible workflows rather than duplicate unit assertions
- [Playwright aria snapshots](https://playwright.dev/docs/aria-snapshots)
  - relevant because accessibility-structure checks can help guard higher-level adequacy surfaces
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat results adequacy as a layered contract:
   - component proof for visible structure and cues
   - browser proof for shell-level navigation and cross-panel integrity
2. Keep provenance, warnings, empty states, and readability in scope, not just raw data presence.
3. Expand coverage when a results surface becomes denser or more decision-critical.
4. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Map each major results surface to at least one direct component test owner.
- [x] Keep empty-state, stale/provenance, and warning cues explicitly covered.
- [x] Use browser proof only for cross-panel interaction and navigation.
- [x] Add tests when new result tabs or docked decision surfaces appear.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web test`
- run targeted results-panel tests
- run `npm --prefix web run test:browser:ux` after major results-surface changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- keep component-level adequacy assertions even if browser coverage expands
- avoid snapshot-heavy testing that hides the actual decision-surface contract
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on current-state results evidence instead of inferred coverage:
  - targeted results-surface component proof passed across `OverviewWorkspacePanel`, `TopWorkspacePanel`, `ParetoWorkspacePanel`, `HeatmapWorkspacePanel`, `DrillWorkspacePanel`, `AnalysisContextDock`, `ValidationWorkspacePanel`, and `OptimizeWorkspacePanel`
  - browser UX smoke passed and still proves cross-panel shell navigation without replacing the lower component layer
  - accessibility smoke and baseline capture both passed, so the adequacy lane now includes user-visible artifact refresh instead of only Vitest proof
- The current adequacy contract remains properly layered:
  - direct component tests own visible structure, empty states, warning cues, provenance chips, dock content, and dense table behavior
  - browser smoke stays limited to end-user navigation and overlay interaction rather than duplicating component assertions
  - artifact capture remains supplemental evidence for public-repo review and regression inspection
- Kept scope intentionally additive:
  - no results-surface behavior was loosened to satisfy the doc
  - no existing component or smoke proofs were removed
  - no backend/report scope was widened from this frontend adequacy closure
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/OverviewWorkspacePanel.test.ts src/components/TopWorkspacePanel.test.ts src/components/ParetoWorkspacePanel.test.ts src/components/HeatmapWorkspacePanel.test.ts src/components/DrillWorkspacePanel.test.ts src/components/AnalysisContextDock.test.ts src/components/ValidationWorkspacePanel.test.ts src/components/OptimizeWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run a11y:smoke`
  - `npm.cmd --prefix web run baseline:ui`

## Related Docs

- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/02_fe_component_surface_tests_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/19_desktop_artifact_capture_2026-04-09.md`
