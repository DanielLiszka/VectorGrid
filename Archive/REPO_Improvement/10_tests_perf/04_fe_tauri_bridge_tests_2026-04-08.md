# Frontend Tauri Bridge Tests Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for frontend Tauri-bridge testing so invoke wrappers, event wrappers, instrumentation, generated command coverage, and mock IPC behavior remain trustworthy as the desktop shell evolves.

## Scope Boundaries

In scope:

- frontend tests for `invoke.ts`
- frontend tests for `events.ts`
- frontend tests for `instrumentation.ts`
- mock IPC harness behavior
- generated-command smoke coverage

Out of scope:

- backend Rust command tests
- browser/UI tests unrelated to the Tauri bridge
- options-specific runtime paths

## Current User-Facing Behavior

This layer is mostly invisible when healthy, but it underpins visible runtime behavior such as:

- command dispatch
- runtime error normalization
- shared event subscriptions
- instrumentation parity with backend channels
- mock-desktop behavior in frontend tests

Current frontend test coverage already includes:

- full registered-command invoke smoke
- `invokeByName()` unknown-command rejection
- runtime presence checks
- mock IPC success/failure normalization
- shared event listener reuse
- sibling-listener safety when one handler throws
- instrumentation channel parity checks

## Current Code Surface

- `web/src/tauri/invoke.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/test/tauriMocks.ts`
- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/invoke.mockIPC.test.ts`
- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/tauri/commands.ts`
- `web/scripts/generate-tauri-commands.mjs`

## Current Verification Surface

- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/invoke.mockIPC.test.ts`
- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- feature tests that rely on the mock Tauri bridge indirectly

## Strengths Already Present

- bridge tests already exercise both typed and by-name invocation paths
- mock IPC already supports strict unknown-command failure
- event wrapper tests already verify shared-listener reuse and reset behavior
- instrumentation parity is already asserted for core backend event channels
- generated command inventory is already part of the frontend runtime surface

## Weaknesses / Failure Modes

- command-generation freshness can still drift if regeneration is skipped after Rust changes
- bridge tests can miss feature-level bypasses of wrapper APIs
- mock IPC can accidentally become looser than production expectations
- event and instrumentation tests currently prove some invariants but not the full wrapper-only policy

## Regressions To Avoid

- stale generated command inventories
- raw Tauri API usage bypassing wrappers without test detection
- mock IPC accepting unknown commands silently
- listener-leak regressions across tests
- new event channels landing without instrumentation parity coverage
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because bridge tests should reflect the command IPC contract the app actually depends on
- [Tauri: Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/)
  - relevant because event-wrapper tests and instrumentation parity depend on Rust-to-frontend event delivery
- [Tauri Mock APIs reference](https://v2.tauri.app/reference/javascript/api/namespacemocks/)
  - relevant because the current test harness is fundamentally a mocked Tauri runtime
- [Vitest Mocking Guide](https://vitest.dev/guide/mocking.html)
  - relevant because bridge tests use mocked modules and mocked IPC heavily
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because many bridge tests ultimately validate store-facing effects in a mocked runtime
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document the frontend Tauri bridge test matrix in layers:
   - command inventory smoke
   - invoke wrapper behavior
   - event wrapper behavior
   - instrumentation parity
   - mock harness strictness
2. Make command-generation freshness a first-class test concern tied to the generated command inventory.
3. Add guardrails against raw Tauri API bypass in feature code.
4. Expand parity tests when new command/event surfaces are added.
5. Keep mock IPC strict by default unless a test intentionally opts into looser behavior.
6. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the bridge-test layers and what each one protects.
- [x] Document generated-command freshness and regeneration expectations.
- [x] Document mock IPC strictness and unknown-command failure expectations.
- [x] Document event-wrapper and instrumentation parity coverage expectations.
- [x] Add tests or checks discouraging raw Tauri API bypass in feature code.
- [x] Add parity tests when new commands/events are introduced.
- [x] Add tests for teardown/reset behavior where coverage is still implicit.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm test -- src/tauri/invoke.contract.test.ts`
- run `npm test -- src/tauri/invoke.mockIPC.test.ts`
- run `npm test -- src/tauri/events.test.ts`
- run `npm test -- src/tauri/instrumentation.test.ts`
- run feature tests that rely on `tauriMocks` when bridge behavior changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve strict mock IPC by default
- keep generated command inventory and runtime wrapper tests in sync with Rust command changes
- if the bridge grows, extend the current test matrix instead of replacing it with one broad smoke test
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the verified current Tauri bridge contract:
  - the invoke/event/instrumentation subset now passes explicitly via `npm.cmd --prefix web run test -- src/tauri/invoke.contract.test.ts src/tauri/invoke.mockIPC.test.ts src/tauri/events.test.ts src/tauri/instrumentation.test.ts`
  - `npm.cmd --prefix web run lint` continues to enforce generated-command freshness through `check:commands` and `check-known-tauri-commands.mjs`
- Tightened bridge authority during closure:
  - [instrumentation.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/tauri/instrumentation.ts) now imports the shared `TRACKED_TAURI_COMMANDS` registry from [taskStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/taskStore.ts) instead of maintaining a duplicate tracked-command list
- Kept scope intentionally additive:
  - no raw Tauri API bypasses were introduced
  - no mock IPC loosened
  - no generated command inventory changed
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/tauri/invoke.contract.test.ts src/tauri/invoke.mockIPC.test.ts src/tauri/events.test.ts src/tauri/instrumentation.test.ts`
  - `npm.cmd --prefix web run lint`
  - `set NODE_OPTIONS=--max-old-space-size=4096 && npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/10_tests_perf/20_tauri_runtime_wiring_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
