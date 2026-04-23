# Tauri Runtime Wiring Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for frontend Tauri runtime wiring and its verification so command invocation, event subscription, lifecycle hooks, reset behavior, and runtime error normalization remain dependable across desktop and test contexts.

## Scope Boundaries

In scope:

- `web/src/tauri/invoke.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/instrumentation.ts`
- runtime wiring tests
- wrapper-only command/event access expectations

Out of scope:

- backend Tauri command implementations
- feature-specific UI behavior above the wiring layer
- options-specific runtime paths

## Current User-Facing Behavior

Users do not interact with the wiring layer directly, but many visible behaviors depend on it:

- command start/finish/error tracking
- progress updates from Rust
- live execution runner events
- Polygon live-stream status
- normalized runtime errors
- test-safe and browser-safe no-Tauri behavior

The current wiring layer already does useful work:

- the frontend command union is generated from the Rust `generate_handler!` set
- `invokeCommand()` wraps raw Tauri invoke calls
- `invokeByName()` rejects unknown commands before IPC dispatch
- lifecycle hooks support `beforeInvoke`, `afterSuccess`, and `afterError`
- `listenEvent()` multiplexes listeners per channel
- instrumentation installs shared listeners and invoke hooks
- reset helpers exist for tests

## Current Code Surface

- `web/src/tauri/invoke.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/commands.ts`
- `web/scripts/generate-tauri-commands.mjs`
- `web/src/test/tauriMocks.ts`
- `src-tauri/src/lib.rs`
- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/stores/taskStore.ts`
- `web/src/main.ts`

## Current Verification Surface

- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/invoke.mockIPC.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `web/src/stores/taskStorePerformance.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- service tests that depend on runtime wiring indirectly

## Strengths Already Present

- raw invoke/listen calls are already wrapped rather than scattered across the app
- lifecycle hooks are defensive and best-effort
- non-Tauri runtime checks already exist
- shared event multiplexing already avoids duplicate subscriptions
- test reset utilities already exist for the wiring layer
- command names are already generated rather than hand-maintained by default
- test mocks can already fail unknown commands instead of tolerating silent drift

## Weaknesses / Failure Modes

- tracked commands and event channels still have duplication risk
- feature code can still regress by bypassing wrappers unless the rule is documented and checked
- hot-module disposal and retry semantics are subtle enough to decay
- runtime errors can become inconsistent if normalization is bypassed
- the test surface can miss wiring regressions if only feature-level tests are updated
- generated command lists can go stale if Rust command changes are not followed by regeneration
- argument-name drift across the JS/Rust boundary can evade casual smoke tests

## Regressions To Avoid

- direct use of raw `invoke` or `listen` in feature code
- listener leaks across hot reload or test resets
- missing runtime-error normalization on invoke failure
- breaking browser/non-Tauri test behavior
- desynchronizing tracked command/event registries between instrumentation and stores
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because `invokeCommand()` is the wrapper around this contract
- [Tauri: Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/)
  - relevant because `listenEvent()` and instrumentation depend on event delivery semantics
- [Vitest Mocking Guide](https://vitest.dev/guide/mocking.html)
  - relevant because the wiring layer is heavily verified through mocked runtime APIs
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because some runtime-wiring guarantees should still be checked through visible shell behavior, not just unit mocks
- [Tauri Mock APIs reference](https://v2.tauri.app/reference/javascript/api/namespacemocks/)
  - relevant because the test layer depends on realistic mocked command/event behavior
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document a wrapper-only runtime policy:
   - feature code calls `invokeCommand()` / `listenEvent()`
   - feature code does not call raw Tauri APIs directly
   - command names come from the generated union, not ad hoc strings
2. Define the wiring stack explicitly:
   - invoke wrapper
   - event wrapper
   - instrumentation installer
   - store consumers
3. Document reset and teardown expectations for:
   - tests
   - hot reload
   - non-Tauri contexts
4. Define error-normalization expectations so every invoke failure is shaped consistently.
5. Expand tests around wrapper bypass risks, retry behavior, reset behavior, and channel multiplexing.
6. Treat command-generation freshness as part of the runtime-wiring contract, not a separate optional maintenance task.
7. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the wrapper-only runtime policy.
- [x] Document invoke lifecycle hook behavior and token flow.
- [x] Document shared-event listener multiplexing and teardown behavior.
- [x] Document runtime error-normalization expectations.
- [x] Document non-Tauri fallback behavior and no-op event wiring.
- [x] Document generated-command freshness and regeneration expectations.
- [x] Document mock IPC strictness expectations for tests.
- [x] Add tests for reset/hot-dispose/retry behavior.
- [x] Add tests for duplicate-listener prevention.
- [x] Add tests or lintable checks preventing raw Tauri API bypass in feature code.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm test -- src/tauri/events.test.ts`
- run `npm test -- src/tauri/instrumentation.test.ts`
- run `npm test -- src/tauri/invoke.contract.test.ts`
- run `npm test -- src/tauri/invoke.mockIPC.test.ts`
- run `npm test -- src/stores/taskStoreCampaignProgress.test.ts`
- run `npm test -- src/stores/taskStorePerformance.test.ts`
- run `npm test -- src/services/workflowArtifactsSync.test.ts`
- run a desktop smoke flow to confirm progress/live events still reach visible UI surfaces
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve current wrapper signatures where possible because many tests and services already depend on them
- prefer additive guardrails over hard rewrites of the runtime layer
- if tracked-command registries are unified later, keep compatibility with current event names first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the current frontend runtime-wiring surface after verification-focused hardening rather than a broad runtime rewrite.
- Wrapper-only runtime policy is now explicitly enforced:
  - raw `@tauri-apps/api/core` and `@tauri-apps/api/event` imports remain confined to `web/src/tauri/invoke.ts` and `web/src/tauri/events.ts`
  - `web/src/tauri/runtimeWiringPolicy.test.ts` scans non-test source files and fails if feature code bypasses that wrapper layer
- Invoke lifecycle/token flow remains the owned path:
  - `invokeCommand()` and `invokeByName()` still centralize command dispatch and runtime-error normalization
  - `setInvokeLifecycleHooks()` remains the single instrumentation hook point for `beforeInvoke`, `afterSuccess`, and `afterError`
  - `installTauriInstrumentation()` continues to install those hooks once and `resetTauriInstrumentationForTests()` clears them explicitly
- Shared event listener behavior remains covered and documented by current tests:
  - `listenEvent()` multiplexes one native listener per channel
  - sibling subscribers stay alive when one subscriber throws
  - explicit reset tears down native listeners
  - non-Tauri contexts still return a safe `null`/no-op result instead of throwing
- Retry and reset behavior are now directly tested:
  - `web/src/tauri/instrumentation.retry.test.ts` verifies deferred event subscription retries until the event API becomes available
  - the same test verifies explicit reset clears pending retry work and lifecycle hooks
- Command freshness and strict mock IPC expectations remain part of the runtime contract:
  - `npm.cmd --prefix web run lint` passed `generate-tauri-commands.mjs --check`
  - strict mock-IPC/runtime failure handling remains covered by `invoke.mockIPC`, `instrumentation.mockIPC`, and `errorPathQa`
- No `README.md` or `REPO_MAP.md` update was required for this item because the implementation only tightened runtime verification and wrapper ownership; user-facing workflow routing did not change.
- Acceptance evidence for this closure:
  - `npm.cmd --prefix web run test -- src/tauri/events.test.ts src/tauri/invoke.contract.test.ts src/tauri/invoke.mockIPC.test.ts src/tauri/instrumentation.test.ts src/tauri/instrumentation.mockIPC.test.ts src/tauri/instrumentation.retry.test.ts src/tauri/runtimeWiringPolicy.test.ts src/tauri/errorPathQa.test.ts src/stores/taskStoreCampaignProgress.test.ts src/stores/taskStorePerformance.test.ts src/services/workflowArtifactsSync.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - companion visible-shell evidence retained from the same frontend tranche because this item changed verification/guardrails only, not desktop runtime behavior:
    - `npm.cmd --prefix web run test:desktop:smoke`
    - artifact path: `output/wdio/desktop-smoke/2026-04-10T17-40-18-604Z/`

## Related Docs

- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
