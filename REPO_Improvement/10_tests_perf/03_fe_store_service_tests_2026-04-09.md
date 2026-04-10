# FE Store Service Tests Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because store tests are a major part of the current architecture
- [Vitest mocking guide](https://vitest.dev/guide/mocking)
  - relevant because service tests depend on controlled Tauri/runtime mocking
- [Vue state management guide](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because test ownership should match state ownership
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat store tests as the primary proof for derived state and action sequencing.
2. Treat service tests as the primary proof for request building, invocation shaping, and result normalization.
3. Keep component tests focused on UI behavior that consumes these layers.
4. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory the non-options store tests and their owning workflows.
- [x] Inventory the non-options service tests and their owning workflows.
- [x] Identify any critical derived state or request builder paths without direct tests.
- [x] Keep store/service coverage aligned with current architecture boundaries.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web test`
- run targeted store/service tests for changed workflows
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve existing useful tests while clarifying ownership
- add new store/service tests before relying on smoke coverage for new orchestration paths
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the verified store/service corpus rather than stale assumptions:
  - `npm.cmd --prefix web test` now passes with `121` files and `1059` tests, including the current non-options store and service layers
  - the passing suite covers request shaping, orchestration, blocked reasons, artifact hydration, settings bootstrap, history replay, validation/report flows, live execution, ML/meta orchestration, and task lifecycle state
- Tightened this layer during closure:
  - [executionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.test.ts) now explicitly pins `manual`, `best`, and `selected` execution parameter-source behavior
  - [executionStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.actions.test.ts) continues to prove concurrent runtime-subscription reuse and retry behavior
- Kept scope intentionally additive:
  - no service/store ownership moved
  - no orchestration tests were removed
- Acceptance evidence:
  - `npm.cmd --prefix web test`
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts`
  - `npm.cmd --prefix web run lint`
  - `set NODE_OPTIONS=--max-old-space-size=4096 && npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/02_fe_component_surface_tests_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/04_fe_tauri_bridge_tests_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/50_frontend_test_support_mocks_2026-04-09.md`
