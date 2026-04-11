# Frontend Test Support Mocks Plan

Last updated: 2026-04-10

## Purpose

Define the non-options frontend-side Tauri and desktop-window mock layer used by unit, service, and instrumentation tests.

## Scope Boundaries

In scope:

- the support mocks in `web/src/test/tauriMocks.ts` and `web/src/test/desktopWindowMocks.ts`
- their use in non-options `invoke.mockIPC`, instrumentation, and error-path tests
- mocked IPC, mocked event, and mocked desktop-window behavior boundaries
- the distinction between frontend test-support mocks and browser-harness support mocks

Out of scope:

- options-related frontend tests
- the Playwright browser-harness support layer, which has a separate doc
- real desktop smoke behavior
- general backend mocking outside the frontend test surface

## Current User-Facing Behavior

The repo already has a focused frontend test-support mock layer that wraps `@tauri-apps/api/mocks`, installs legacy bridge shims when needed, records IPC calls, supports strict or permissive unknown-command handling, and provides minimal desktop-window mocking. This is a strong foundation, but it needs a doc so the unit/service/instrumentation test layer stays realistic and does not quietly diverge from the browser or desktop harnesses.

## Current Code Surface

- `web/src/test/tauriMocks.ts` provides mock IPC installation, bridge reset, and harness helpers.
- `web/src/test/desktopWindowMocks.ts` provides minimal mocked desktop-window behavior for tests that need it.
- `web/src/tauri/invoke.mockIPC.test.ts`, `instrumentation.mockIPC.test.ts`, and `errorPathQa.test.ts` are key current consumers.
- `web/src/tauri/invoke.ts` is the real bridge surface these mocks are standing in for.

## Current Mock Inventory And Coverage

- `web/src/test/tauriMocks.ts`
  - currently exported helper surface:
    - `resetTauriApiMocks()`
    - `installLegacyTauriWindowBridge()`
    - `installMockIpc()`
    - `mockCommandSuccess()`
    - `mockCommandFailure()`
  - currently exercised in-tree by targeted tests:
    - `installMockIpc()`
    - `resetTauriApiMocks()`
- `web/src/test/desktopWindowMocks.ts`
  - currently exported helper surface:
    - `mockDesktopWindows()`
    - `resetMockWindows()`
  - current direct in-tree consumers:
    - none found in the current search surface
- direct test consumers of the mock IPC layer today:
  - `web/src/tauri/invoke.mockIPC.test.ts`
  - `web/src/tauri/instrumentation.mockIPC.test.ts`
  - `web/src/tauri/errorPathQa.test.ts`

## Current Verification Surface

- `npm --prefix web run test -- src/tauri/invoke.mockIPC.test.ts src/tauri/instrumentation.mockIPC.test.ts src/tauri/errorPathQa.test.ts`
- broader frontend test runs with `npm --prefix web test` when mock-layer behavior changes
- comparison against browser or desktop harness behavior when a mock assumption is in doubt

## Strengths Already Present

- the mock layer already centralizes common Tauri mocking behavior instead of duplicating it per test
- strict unknown-command behavior helps catch accidental contract drift
- tests already use the mock layer to exercise success, failure, cancellation, and instrumentation paths
- desktop-window mocking is kept lightweight rather than pretending to be a full desktop runtime

## Weaknesses / Failure Modes

- frontend mocks can drift away from real runtime behavior if they are not constrained
- allowing unknown commands in some tests can become a loophole if used indiscriminately
- the mock layer is convenient enough that it can attract scenarios that should move to browser or desktop harnesses
- it is easy to conflate unit-test mock guarantees with native-shell guarantees

## Regressions To Avoid

- letting frontend test-support mocks become a second unofficial runtime
- adding permissive behavior to make tests easy instead of accurate
- using frontend mocks as evidence for real desktop runtime behavior
- splitting similar mock helpers across many files and losing a single contract surface
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri mock APIs](https://v2.tauri.app/reference/javascript/api/namespacecore/#mockipc) are relevant because the current frontend support layer is built around the official Tauri mock mechanism.
- [Vitest mocking guide](https://vitest.dev/guide/mocking) is relevant because these helpers exist to make unit and service tests deterministic without overreaching.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because several consumers of the mocks also depend on store-driven state instrumentation.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant by contrast because real browser or desktop journeys should move out of the unit mock layer when appropriate.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the frontend test-support mocks narrow, deterministic, and explicitly subordinate to browser and desktop harnesses.
2. Preserve strict unknown-command defaults unless a test has a clear reason to allow unknown commands.
3. Use the mock layer to test frontend behavior around invocation and instrumentation, not to fake whole desktop workflows.
4. Review mock growth against the real `invoke` surface and adjacent higher-level harnesses.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory the current mock helpers and the tests that depend on them.
- [x] Document strict versus permissive unknown-command behavior.
- [x] Document when a scenario belongs in browser or desktop harnesses instead of this layer.
- [x] Keep this mock plan non-options only.
- [x] Keep desktop-window mocking intentionally minimal.
- [x] Cross-check mock assumptions against the real Tauri bridge when it changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the targeted mock-driven tests after changing the support layer
- run the broader frontend test suite when shared mock behavior changes
- compare suspicious mock-only passes against browser or desktop harness behavior
- inspect the real `invoke` implementation before broadening the support mocks
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the centralized support-mock layer and improve documentation around it
- prefer tightening existing helpers over creating parallel mock systems
- if a mock must grow significantly, reevaluate whether the test belongs at a higher layer
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closure date: `2026-04-10`
- Code changes were not required; the mock IPC layer, real invoke boundary, and targeted tests were already in place.
- Verified with:
  - `npm --prefix web run test -- src/tauri/invoke.mockIPC.test.ts src/tauri/instrumentation.mockIPC.test.ts src/tauri/errorPathQa.test.ts`
  - `npm --prefix web run lint`
  - `npm --prefix web run typecheck`
  - `node -v`
  - `npm -v`
- Verification results:
  - targeted Vitest: `3` files passed, `16` tests passed
  - `lint`: passed and re-ran generated-command plus known-command checks successfully
  - `typecheck`: passed
  - Node: `v24.4.1`
  - npm: `11.6.4`
- Inventory cross-check notes:
  - strict versus permissive unknown-command behavior remains centralized in `installMockIpc()` via `allowUnknown`
  - browser or desktop harnesses remain the correct destination when a test needs real runtime behavior rather than deterministic invoke/event stubbing
  - `installLegacyTauriWindowBridge()`, `mockCommandSuccess()`, `mockCommandFailure()`, `mockDesktopWindows()`, and `resetMockWindows()` currently appear unexercised by direct in-tree consumers; this is a residual risk to watch rather than a blocker for this documentation closeout
- No `README.md`, `REPO_MAP.md`, or release-note update was required because this item closed on inventory and verification evidence rather than changed runtime behavior.
- Output artifact location: none beyond terminal stdout from the verification commands.
- Refresh trigger for this check: any change to `web/src/test/tauriMocks.ts`, `web/src/test/desktopWindowMocks.ts`, `web/src/tauri/invoke.ts`, or the targeted mock-driven tests.

## Related Docs

- `REPO_Improvement/10_tests_perf/12_browser_support_mocks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/21_tauri_mock_runtime_smoke_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_2026-04-09.md`
