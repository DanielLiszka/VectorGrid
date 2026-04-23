# FE Component Surface Tests Plan

Last updated: 2026-04-10

## Purpose

Define the current component-test ownership layer so user-facing Vue panels and overlays keep strong deterministic interaction coverage without being over-reliant on higher-level smoke tests.

## Scope Boundaries

In scope:

- `web/src/components/*.test.ts`
- panel, overlay, titlebar, and shell component tests
- component-local interaction and rendering contracts

Out of scope:

- store-only and service-only tests
- browser UX smoke and desktop smoke
- options-specific component tests

## Current User-Facing Behavior

The repo already has broad component coverage for the non-options shell: workflow shell, workspace panels, results tabs, history, execution, task center, command palette, settings, and titlebar behavior are all represented in direct component tests.

## Current Code Surface

- `web/src/components/*.test.ts`
  - current component-surface suite including `VueWorkspaceShell`, `OptimizeWorkspacePanel`, `ValidationWorkspacePanel`, `ExecutionWorkspacePanel`, `HistoryWorkspacePanel`, `DesktopTitlebar`, `TaskCenterOverlay`, `CommandPaletteOverlay`, and related results-panel tests
- `web/src/test/tauriMocks.ts`
  - shared mocked-Tauri helpers for component-level tests
- `web/package.json`
  - `npm run test`

## Current Verification Surface

- `npm --prefix web test`
- targeted component test runs under `web/src/components/*.test.ts`

## Strengths Already Present

- the repo already tests real user-facing panels instead of only stores/services
- shell chrome, overlays, and results surfaces are already represented
- component tests complement rather than replace store/service layers

## Weaknesses / Failure Modes

- broad component coverage can still miss ownership boundaries if not documented
- component tests can become brittle if they reassert store/service details that belong elsewhere
- smoke layers can be overtrusted and cause component gaps to persist

## Regressions To Avoid

- moving user-visible interaction proof entirely into smoke tests
- letting component tests become snapshots of implementation details instead of UI contracts
- leaving new shell/panel changes without direct deterministic coverage
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue Test Utils guide](https://test-utils.vuejs.org/guide/)
  - relevant because these tests are the primary component-level contract layer
- [Vitest mocking guide](https://vitest.dev/guide/mocking)
  - relevant because component tests rely on deliberate mock boundaries
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because component tests should stay narrower than browser smoke
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat component tests as the primary protection for panel/overlay rendering and local interaction semantics.
2. Keep store/service responsibilities out of component assertions unless they are user-visible.
3. Use browser/desktop smoke only as a complement for cross-component journeys.
4. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory all non-options component tests and their owning surfaces.
- [x] Identify uncovered user-facing components.
- [x] Keep component assertions focused on visible behavior, focus flow, copy, and accessibility-relevant structure.
- [x] Add targeted component tests when new shell/panel behavior is introduced.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web test`
- run targeted component test files for changed panels or overlays
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve useful existing component tests while tightening ownership boundaries
- prefer adding narrow tests over moving coverage upward into smoke layers
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the verified component-surface suite rather than inferred UI coverage:
  - `npm.cmd --prefix web test` now passes with `121` test files and `1059` tests
  - the current component layer is directly visible in the passing `src/components/*.test.ts` suite, including shell, titlebar, overlays, workflow panels, results panels, context docks, settings, history, validation, execution, ML, and debug surfaces
- The component layer remains appropriately bounded:
  - component tests carry visible rendering, copy, focus, and interaction semantics
  - store/service orchestration remains owned by the lower-layer docs instead of being duplicated here
  - browser smoke stays complementary rather than replacing deterministic panel coverage
- Kept scope intentionally additive:
  - no component tests were removed
  - no panel behavior was weakened to satisfy the doc
- Acceptance evidence:
  - `npm.cmd --prefix web test`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run lint`
  - `set NODE_OPTIONS=--max-old-space-size=4096 && npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/03_fe_store_service_tests_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
