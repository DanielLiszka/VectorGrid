# FE Store Service Tests Plan

Last updated: 2026-04-09

## Purpose

Define the current store/service verification layer so request shaping, orchestration, derived blocked reasons, and artifact/result normalization stay protected below the component and smoke layers.

## Scope Boundaries

In scope:

- `web/src/stores/*.test.ts`
- `web/src/services/*.test.ts`
- non-options composable/service/store test ownership

Out of scope:

- direct component rendering tests
- browser and desktop smoke
- options-specific store/service tests

## Current User-Facing Behavior

The repo already depends heavily on store/service correctness for user-visible behavior: optimize/validation/execution state, history reloads, ML/meta orchestration, settings bootstrap, and Tauri invocation all pass through tested store/service layers.

## Current Code Surface

- `web/src/stores/*.test.ts`
  - current store action/state coverage for data, optimize, validation, execution, settings, history, workflow artifacts, task lifecycle, and related domains
- `web/src/services/*.test.ts`
  - current service coverage for optimize, validation, execution, history, data, settings, ML/meta, and artifact sync flows
- `web/package.json`
  - `npm run test`

## Current Verification Surface

- `npm --prefix web test`
- targeted store and service test runs

## Strengths Already Present

- core orchestration logic is already tested beneath the component layer
- blocked-reason and state-derivation logic already has dedicated store tests
- request builder and service normalization behavior already has good coverage

## Weaknesses / Failure Modes

- store and service responsibility boundaries can blur in a large repo
- high mock usage can create false confidence if not balanced against component/smoke proof
- broad service coverage can hide missing tests for a new derived state branch

## Regressions To Avoid

- shifting core orchestration proof upward into component tests only
- allowing request-builder or result-normalization changes without store/service updates
- duplicating service assertions in components while leaving service tests weak

## External Research Summary

- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because store tests are a major part of the current architecture
- [Vitest mocking guide](https://vitest.dev/guide/mocking)
  - relevant because service tests depend on controlled Tauri/runtime mocking
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because test ownership should match state ownership

## Proposed Improvement Approach

1. Treat store tests as the primary proof for derived state and action sequencing.
2. Treat service tests as the primary proof for request building, invocation shaping, and result normalization.
3. Keep component tests focused on UI behavior that consumes these layers.

## Checklist

- [ ] Inventory the non-options store tests and their owning workflows.
- [ ] Inventory the non-options service tests and their owning workflows.
- [ ] Identify any critical derived state or request builder paths without direct tests.
- [ ] Keep store/service coverage aligned with current architecture boundaries.

## Verification Plan

- run `npm --prefix web test`
- run targeted store/service tests for changed workflows

## Rollout / Fallback / Compatibility Notes

- preserve existing useful tests while clarifying ownership
- add new store/service tests before relying on smoke coverage for new orchestration paths

## Related Docs

- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/02_fe_component_surface_tests_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/50_frontend_test_support_mocks_DATE.md`
