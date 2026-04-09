# FE Dense Table Virtualization Plan

Last updated: 2026-04-09

## Purpose

Define the verification strategy for dense table virtualization so large result and validation tables remain performant and correct under windowed rendering.

## Scope Boundaries

In scope:

- frontend dense-table virtualization behavior
- table state and row-window rendering correctness
- browser/component verification for large rendered tables

Out of scope:

- backend pagination or data generation
- workbook export tables
- options-related tables

## Current User-Facing Behavior

The repo already treats large result surfaces as lazy/windowed where appropriate. Users see dense top-table and validation-table surfaces that must feel responsive while still preserving row identity, sorting/filtering cues, and visible selection state.

## Current Code Surface

- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/package.json`
  - includes `@tanstack/vue-virtual` and related build/test tooling

## Current Verification Surface

- `npm --prefix web test`
- targeted component tests for dense table panels
- browser UX smoke when dense surface behavior changes materially

## Strengths Already Present

- virtualization is already part of the intended frontend product posture
- dense result surfaces already have direct panel coverage
- the repo uses dedicated virtualization/table libraries instead of ad hoc DOM scaling

## Weaknesses / Failure Modes

- windowed rendering can look correct in small tests while failing on larger result sets
- virtualization regressions often appear as selection/focus/readability bugs rather than total failures
- size/perf and correctness need to be tested together

## Regressions To Avoid

- losing row identity or visible state during virtualization changes
- replacing deterministic table tests with only manual browser review
- treating virtualization as a pure performance concern with no UX contract

## External Research Summary

- [TanStack Virtual Vue docs](https://tanstack.com/virtual/latest/docs/framework/vue/overview)
  - relevant because the repo uses virtualized table/windowing concepts that should be tested with library-aware expectations
- [TanStack Table Vue docs](https://tanstack.com/table/latest/docs/framework/vue/guide/table-state)
  - relevant because virtualization interacts with table state, sorting, and visible rows
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because higher-level browser proof should validate user-visible behavior, not implementation details

## Proposed Improvement Approach

1. Keep direct component tests as the primary proof for row rendering, state retention, and visible cues.
2. Use browser proof only for representative dense-surface journeys.
3. Treat virtualization correctness and responsiveness as one contract.

## Checklist

- [ ] Document which surfaces are currently virtualized or size-sensitive.
- [ ] Keep row identity, sorting/filtering, and visible-state behavior covered in tests.
- [ ] Add representative large-data fixtures where current tests are too small.
- [ ] Use browser proof for representative dense-surface interaction after major changes.

## Verification Plan

- run `npm --prefix web test`
- run targeted table-panel tests
- run `npm --prefix web run test:browser:ux` after dense table behavior changes

## Rollout / Fallback / Compatibility Notes

- preserve current table-state behavior while improving virtualization coverage
- prefer additive large-fixture tests over replacing small deterministic tests

## Related Docs

- `REPO_Improvement/01_shell/17_ux_table_virtualization_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/05_fe_results_surface_adequacy_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/08_fe_ui_baseline_capture_DATE.md`
