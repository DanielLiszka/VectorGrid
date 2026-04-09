# FE Component Surface Tests Plan

Last updated: 2026-04-09

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

## External Research Summary

- [Vue Test Utils guide](https://test-utils.vuejs.org/guide/)
  - relevant because these tests are the primary component-level contract layer
- [Vitest mocking guide](https://vitest.dev/guide/mocking)
  - relevant because component tests rely on deliberate mock boundaries
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because component tests should stay narrower than browser smoke

## Proposed Improvement Approach

1. Treat component tests as the primary protection for panel/overlay rendering and local interaction semantics.
2. Keep store/service responsibilities out of component assertions unless they are user-visible.
3. Use browser/desktop smoke only as a complement for cross-component journeys.

## Checklist

- [ ] Inventory all non-options component tests and their owning surfaces.
- [ ] Identify uncovered user-facing components.
- [ ] Keep component assertions focused on visible behavior, focus flow, copy, and accessibility-relevant structure.
- [ ] Add targeted component tests when new shell/panel behavior is introduced.

## Verification Plan

- run `npm --prefix web test`
- run targeted component test files for changed panels or overlays

## Rollout / Fallback / Compatibility Notes

- preserve useful existing component tests while tightening ownership boundaries
- prefer adding narrow tests over moving coverage upward into smoke layers

## Related Docs

- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/03_fe_store_service_tests_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_DATE.md`
- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_DATE.md`
