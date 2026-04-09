# Frontend Test Matrix Plan

Last updated: 2026-04-08

## Purpose

Define the non-options frontend test matrix so components, stores, services, Tauri bridges, browser UX flows, and desktop smoke coverage each have a clear ownership role.

## Scope Boundaries

In scope:

- Vitest component, store, service, composable, and Tauri-bridge tests
- browser and desktop smoke harness placement
- mock boundaries and `mockIPC` usage
- gap tracking for non-options frontend flows

Out of scope:

- options-related tests
- backend-only Rust verification
- generic code-coverage percentage reporting

## Current User-Facing Behavior

The app already ships with a broad frontend test surface, but there is no single matrix explaining which layer should protect which user-visible behavior. Current testing already spans:

- component tests such as `HistoryWorkspacePanel.test.ts`, `OptimizeWorkspacePanel.test.ts`, `ExecutionWorkspacePanel.test.ts`, and `MlSignalWorkspacePanel.test.ts`
- service tests such as `historyService.test.ts`, `workflowBootstrap.test.ts`, `executionService.test.ts`, `validationService.test.ts`, and `mlSignalService.test.ts`
- store tests such as `optimizeStore.actions.test.ts`, `workflowStore.test.ts`, and `executionStore.actions.test.ts`
- Tauri bridge and instrumentation tests such as `commands.test.ts`, `events.test.ts`, and `instrumentation.mockIPC.test.ts`
- browser and desktop smoke harnesses documented under `web/e2e/browser/README.md` and `web/e2e/desktop/README.md`

## Current Code Surface

- `web/package.json`
  - defines `test`, browser UX scripts, and desktop smoke scripts
- `web/src/components/*.test.ts`
  - covers workspace panels and other user-facing component behavior
- `web/src/stores/*.test.ts`
  - covers derived blocked reasons, action sequencing, and persisted UI state
- `web/src/services/*.test.ts`
  - covers request shaping, Tauri invocation, workflow orchestration, and artifact/result normalization
- `web/src/tauri/*.test.ts`
  - covers command registration, event wiring, instrumentation, and `mockIPC` boundaries
- `web/src/test/tauriMocks.ts`
  - centralizes a large share of frontend-side mock IPC behavior
- `web/e2e/browser/README.md`
  - documents browser UX smoke expectations
- `web/e2e/desktop/README.md`
  - documents the desktop smoke harness and execution pattern

## Current Verification Surface

- `npm --prefix web test`
- `npm --prefix web run test:browser:ux`
- `npm --prefix web run test:desktop:smoke`
- targeted script variants such as `test:desktop:smoke:spec`

## Strengths Already Present

- the repo already has meaningful unit/integration-style Vitest coverage across multiple frontend layers
- the Tauri bridge is not treated as invisible; it already has dedicated tests and instrumentation checks
- browser and desktop smoke layers already exist and are documented
- several critical non-options workflows already have both service/store tests and panel tests rather than relying on a single layer

## Weaknesses / Failure Modes

- coverage is broad enough that ownership can become fuzzy without a matrix
- some user-visible behaviors span component + store + service + Tauri bridge, making it easy to test the same thing twice while missing another contract entirely
- `mockIPC`-heavy tests can create false confidence if they are not balanced against higher-layer smoke coverage
- browser and desktop smoke coverage can be overtrusted if it is not explicitly treated as a complement to unit/service tests

## Regressions To Avoid

- replacing narrow deterministic tests with broader but weaker smoke tests
- adding new UI behavior without deciding which layer owns its primary protection
- allowing command-bridge and artifact-hydration paths to drift because “some higher layer probably covers it”
- treating the matrix as a coverage vanity metric instead of an ownership map

## External Research Summary

- [Vitest mocking guide](https://vitest.dev/guide/mocking) is relevant because a large share of this repo’s frontend tests depend on deliberate mocking boundaries.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because store tests are a core part of the current frontend verification strategy.
- [Playwright best practices](https://playwright.dev/docs/best-practices) is relevant because browser/desktop smoke flows should target user-visible contracts rather than recreate unit tests.
- [Vue state-management guidance](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because test ownership should follow the same boundaries as actual state ownership.

## Proposed Improvement Approach

1. Inventory every existing non-options frontend test by layer and owning workflow.
2. Map each major user-facing surface to a primary test layer and any secondary smoke coverage.
3. Call out where a layer is over-mocked, under-tested, or incorrectly carrying responsibilities better owned elsewhere.
4. Use the matrix as the gate for future test additions so new coverage is intentional rather than reactive.

## Checklist

- [ ] Inventory all non-options component tests and the panels/features they cover.
- [ ] Inventory all non-options store, service, composable, and Tauri-bridge tests.
- [ ] Map browser UX smoke and desktop smoke to the unit/service layers they complement.
- [ ] Identify flows that currently depend too heavily on `mockIPC`.
- [ ] Identify flows that currently have only smoke coverage or only service coverage.
- [ ] Define primary ownership for history, validation, execution, ML Signal, and artifact-related frontend flows.
- [ ] Keep the matrix updated when new tests or new non-options surfaces are added.
- [ ] Use the matrix to justify future Wave docs instead of adding tests ad hoc.

## Verification Plan

- run `npm --prefix web test`
- run `npm --prefix web run test:browser:ux`
- run `npm --prefix web run test:desktop:smoke`
- verify the written matrix still matches the actual test-file inventory

## Rollout / Fallback / Compatibility Notes

- keep the matrix descriptive before trying to reshape the test suite around it
- preserve existing useful tests even if ownership is later reassigned
- do not collapse distinct test tiers into one generic status bucket

## Related Docs

- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_DATE.md`
- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_DATE.md`
- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_DATE.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
