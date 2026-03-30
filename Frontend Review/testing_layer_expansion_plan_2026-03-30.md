# Testing Layer Expansion Plan

Date: 2026-03-30

Scope: repo-specific implementation plan for the valid follow-up testing ideas that remain after the first-wave `WebdriverIO + tauri-driver` desktop smoke harness.

Status: audited and adjusted to match the current repo/runtime shape.

## Goal

Expand testing coverage in ways that are objectively useful for this repo without destabilizing the already-green desktop smoke harness.

The target additions are:

- a frontend `mockIPC` layer for deterministic UI/runtime-path tests
- a Rust mock-runtime layer for command/state wiring tests that do not need a real WebView
- a small, low-flake hardening pass for the existing desktop smoke suite

This plan intentionally does **not** replace:

- existing `vitest` component/service tests
- the current Playwright browser-side QA scripts
- the current `WebdriverIO + tauri-driver` desktop smoke harness

## Why This Is Valid For This Repo

Current repo state:

- a real desktop smoke harness already exists and is green:
  - [wdio.desktop.conf.ts](C:\Rust Projects\ta_desktop_demo\web\wdio.desktop.conf.ts)
  - [web/e2e/desktop/specs](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs)
  - [desktop_e2e_testing_plan_2026-03-29.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\desktop_e2e_testing_plan_2026-03-29.md)
- browser-side coverage is already strong:
  - [package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
  - [capture-baseline-ui.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\capture-baseline-ui.mjs)
  - [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)
- frontend unit/service/store test coverage is already broad under [web/src](C:\Rust Projects\ta_desktop_demo\web\src)
- Rust/unit coverage is already broad in `crates/*` and `src-tauri/src/*`

What is still missing:

- no current usage of `@tauri-apps/api/mocks` / `mockIPC`
- no current usage of `tauri::test`, `MockRuntime`, or `mock_builder`
- no current explicit desktop-smoke assertion for `setWindowRect`-driven window geometry behavior

That makes the highest-value next additions:

1. deterministic Tauri-API mocking on the frontend
2. deterministic Tauri command/runtime wiring tests on the Rust side
3. one narrowly useful desktop geometry assertion that is lower risk than drag simulation

## Non-Goals

This plan does **not** include:

- replacing the current official `tauri-driver` harness with `@wdio/tauri-service`
- adopting `@wdio/tauri-plugin` immediately
- macOS automation work
- OS-level automation for dialogs/menus/permissions
- large new desktop E2E coverage matrices
- screenshot-diff expansion at the desktop-window level

Those may be revisited later, but they are not currently the highest-ROI additions for this repo.

## Regression Policy

This plan must not introduce new flake or degrade the current green desktop-smoke path.

Required guardrails:

- keep the current `WebdriverIO + tauri-driver` harness as the default real-app path
- keep new desktop assertions narrow and deterministic
- do not add native-drag automation to the default suite
- do not add `withGlobalTauri` back to production config for test convenience
- do not require `@wdio/tauri-service` or `@wdio/tauri-plugin` unless the current harness proves a real limitation
- keep mock-driven tests isolated from real desktop smoke tests

## Verification Gate

For changes under this plan, the minimum verification stack is:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`

For Rust-side mock-runtime additions, also run:

- `cargo test -p ta_desktop_demo_app`

If a change touches desktop-smoke TypeScript files directly, also run:

- `npm --prefix web run typecheck:desktop`

If a change touches desktop-smoke TypeScript files directly, the current Windows-first local smoke commands should be:

- `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1 npm --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-window-geometry.spec.ts`
- one follow-up full-suite pass only after the spec is stable locally:
  - `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1 npm --prefix web run test:desktop:smoke`

Do not assume the existing remote desktop-smoke workflow is currently a reliable validation path for this plan. That workflow still performs a fresh `cargo build -p ta_desktop_demo_app`, and the current desktop harness only has a documented local skip-build workaround.

## Build-Ready Implementation Decisions

These decisions are intentionally fixed here so the implementation work does not have to keep re-deciding the same structure.

### Frontend mock layer dependency policy

Do **not** add a new npm dependency for frontend Tauri mocks in the first wave.

Reason:

- `mockIPC`, `clearMocks`, and `mockWindows` already come from the existing `@tauri-apps/api` package
- the missing piece is structure and usage, not package installation

### Frontend test-support file layout

Use this exact first-wave file layout:

- `web/src/test/tauriMocks.ts`
- `web/src/test/desktopWindowMocks.ts`

Optional later split only if the helper grows materially:

- `web/src/test/tauriMockAssertions.ts`

Do **not** create a broad repo-root `tests/` tree for this layer.

Reason:

- this repo’s frontend tests already live close to implementation under `web/src`
- keeping the Tauri mock support in `web/src/test/` keeps imports and Vitest resolution simple

### Frontend test file naming policy

Use these naming rules:

- add new direct Tauri-IPC-focused tests as `*.mockIPC.test.ts`
- keep existing pure component/service/store tests unchanged unless there is a strong reason to extend them
- do not force non-IPC store coverage into the `mockIPC` bucket just for naming consistency

Examples:

- `web/src/tauri/invoke.mockIPC.test.ts`
- `web/src/tauri/instrumentation.mockIPC.test.ts` only if explicit Tauri-global bridging is added for test setup

Reason:

- this prevents the new middle layer from being confused with plain unit tests
- it keeps the new layer easy to search and reason about

### Rust-side layout decision

Keep the current `src-tauri/Cargo.toml` policy as the starting point:

- `[lib] test = false`

That means the first Rust-side additions should **not** assume inline unit tests in `src-tauri/src/*.rs` are the right place.

Preferred first-wave layout:

- `src-tauri/tests/mock_runtime_smoke.rs`
- `src-tauri/tests/command_runtime_wiring.rs`

If those integration tests need shared setup helpers, add:

- `src-tauri/tests/support/mod.rs`

Only if integration tests prove insufficient should implementation consider changing the lib test policy or adding a minimal test-support export surface.

Reason:

- this is the least risky path
- it avoids changing current crate test-target policy before it is necessary
- it keeps the mock-runtime layer clearly separate from the main app sources

### Desktop geometry smoke file decision

Use a new dedicated spec file:

- `web/e2e/desktop/specs/desktop-window-geometry.spec.ts`

Do **not** fold geometry assertions into [desktop-titlebar-native-controls.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-titlebar-native-controls.spec.ts).

Reason:

- window-rect behavior is conceptually different from titlebar button behavior
- a dedicated spec is easier to disable or isolate if geometry assertions ever become flaky

### CI rollout decision

Use this staged rollout:

1. local-only implementation and stabilization
2. no assumption of remote desktop-smoke validation until the fresh-build blocker is resolved or a separate known-good-binary workflow exists
3. no new always-on PR gating for this plan until:
   - the unrelated fresh-build blocker is resolved
   - the geometry smoke remains green locally across multiple runs
   - the new frontend and Rust test layers are green in the normal repo verification stack

This means:

- do not change the current CI policy as part of the first implementation pass
- treat CI expansion as a follow-up after the local layers are proven stable
- treat new `src-tauri/tests/*` coverage as local-only until a dedicated `cargo test -p ta_desktop_demo_app` workflow step exists

## Phase 1: Frontend `mockIPC` Foundation

### Objective

Add a shared frontend test helper layer that lets `vitest` tests simulate Tauri IPC and window/runtime context without launching a real desktop app.

### Why It Matters

This repo has a large amount of frontend behavior that depends on Tauri-side success, failure, and event conditions, but most of it is still tested by direct service/store mocking instead of Tauri IPC semantics.

That leaves a gap between:

- pure service/store unit tests
- full desktop smoke tests

`mockIPC` fills that gap efficiently.

### Target file area

- [web/src/tauri](C:\Rust Projects\ta_desktop_demo\web\src\tauri)
- [web/src/services](C:\Rust Projects\ta_desktop_demo\web\src\services)
- [web/src/stores](C:\Rust Projects\ta_desktop_demo\web\src\stores)

### Recommended implementation shape

Add shared helpers under a dedicated frontend test-support path, for example:

- `web/src/test/tauriMocks.ts`
- `web/src/test/desktopWindowMocks.ts`

Those helpers should wrap:

- `mockIPC`
- `clearMocks`
- `mockWindows` when window-label simulation is useful

The helpers should make it easy to:

- return successful command payloads
- force known command failures
- assert that expected command names and payload shapes were requested
- simulate multiple windows only where desktop code actually cares about window labels or current-window identity

Repo-specific setup guardrails:

- any test that touches Tauri mocks must opt into `jsdom` explicitly at the file level
- any test that uses `mockWindows()` must install window mocks before importing modules that read `@tauri-apps/api/window` at module scope
- prefer dynamic imports in those test files after mock setup, rather than importing the target module at the top of the file
- tests that touch runtime Pinia singletons or persisted storage must explicitly install/reset Pinia and stub storage per file

### Exact helper API contract

The first helper layer should expose a small, explicit API.

Recommended shape:

```ts
export interface MockIpcCall {
  command: string
  args: Record<string, unknown> | undefined
}

export type MockIpcResolver =
  | unknown
  | Error
  | ((args: Record<string, unknown> | undefined, call: MockIpcCall) => unknown | Promise<unknown>)

export interface MockIpcHarness {
  calls: MockIpcCall[]
  clear(): void
  expectCalled(command: string, partialArgs?: Record<string, unknown>): void
  expectCallCount(command: string, count: number): void
}

export function installMockIpc(
  resolvers: Record<string, MockIpcResolver>,
  options?: { allowUnknown?: boolean },
): MockIpcHarness

export function mockCommandSuccess(command: string, value: unknown): MockIpcHarness

export function mockCommandFailure(command: string, error: string | Error): MockIpcHarness

export function resetTauriApiMocks(): void
```

For window helpers:

```ts
export function mockDesktopWindows(current: string, ...others: string[]): void
export function resetMockWindows(): void
```

If a test needs to exercise code that reads `window.__TAURI__` or `globalThis.isTauri` rather than `@tauri-apps/api/core`, that bridging must be explicit in the helper or in the individual test. `mockIPC` alone is not enough for those modules in this repo.

### Cleanup policy

Every `mockIPC` test file should follow this pattern:

- install mocks in `beforeEach`
- call `resetTauriApiMocks()` in `afterEach`
- clear any window mocks in `afterEach`

Do not allow test files to rely on previous mock state.

### Unknown-command policy

The default helper behavior should be:

- fail fast on unknown command names

Reason:

- this catches command drift early
- it avoids false-green tests where the UI asks for the wrong command

### First-wave frontend targets

Prioritize code that is closest to the Tauri bridge:

- [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts)
- [instrumentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\instrumentation.ts)
- [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- [runControlService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\runControlService.ts)
- [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)

Actual first-wave repo-safe target order:

1. [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts)
2. [runControlService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\runControlService.ts) only if the missing best-effort path is not cleaner in the existing test file
3. [instrumentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\instrumentation.ts) only if explicit `window.__TAURI__` and `globalThis.isTauri` bridging is added
4. keep [desktopProductization.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.test.ts) on direct plugin/module mocks unless a real IPC seam is missing
5. keep task lifecycle additions in plain store tests unless a direct IPC seam is genuinely what needs coverage

### Exact first-wave file plan

Create or extend these exact files first:

- add `web/src/test/tauriMocks.ts`
- add `web/src/test/desktopWindowMocks.ts`
- add `web/src/tauri/invoke.mockIPC.test.ts`
- extend [runControlService.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\runControlService.test.ts) if the only missing behavior is the background-task best-effort path
- add `web/src/tauri/instrumentation.mockIPC.test.ts` only if test setup explicitly bridges `window.__TAURI__`, `globalThis.isTauri`, and event mocks
- extend [desktopProductization.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.test.ts) only if a true `mockIPC` case is needed there

Do **not** add `web/src/stores/taskStore.mockIPC.test.ts` in the first wave unless a concrete task-store seam is shown to depend on direct Tauri IPC. The default assumption is that task lifecycle additions belong in ordinary store tests.

Do **not** start by rewriting:

- [invoke.contract.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.contract.test.ts)
- [commands.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\commands.test.ts)

Those should remain as the simpler contract/registry checks they already are.

### Acceptance

- the repo has one shared `mockIPC` helper layer instead of ad hoc per-test mocking
- at least the main Tauri bridge modules have direct mockIPC-backed tests
- new tests validate both success and failure behavior
- new tests stay in `vitest`, not the desktop E2E suite

## Phase 2: High-Value Frontend Tauri-Path Tests

### Objective

Use the new `mockIPC` layer to cover the frontend states that are currently most expensive to validate through real desktop E2E.

### Highest-value targets

1. Command error normalization and propagation
   - [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts)
   - [runtimeError.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\runtimeError.ts)

   Important repo-specific note:
   `invokeCommand()` currently returns the Tauri promise from inside `try` without awaiting it. Real async rejections bypass the local `catch`, so the planned error-normalization scenarios are not achievable until that wrapper behavior is corrected.

2. Task lifecycle transitions driven by realistic command/event outcomes
   - [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)
   - [runControlService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\runControlService.ts)

   Repo-specific narrowing:
   keep the first wave on `runControlService`; task-store lifecycle coverage should remain plain store tests unless direct IPC is demonstrably the missing seam.

3. Desktop plugin availability / fallback behavior
   - [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)

4. Window-sensitive frontend behavior only where real logic depends on it
   - current titlebar / desktop shell bridge surfaces
   - only use `mockWindows` where there is real branching based on window identity

### Exact scenario matrix

#### `web/src/tauri/invoke.mockIPC.test.ts`

Required scenarios:

- successful invoke path returns the mocked payload
- async rejected IPC promise is normalized into `TauriRuntimeError`
- thrown string error is normalized into `TauriRuntimeError`
- thrown `Error` object is normalized into `TauriRuntimeError`
- unknown command path through `invokeByName` still fails before IPC
- command arguments are forwarded unchanged to the IPC mock layer

Implementation guardrail:

- if these tests fail because async rejections bypass local normalization, fix [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts) first instead of weakening the assertions

#### `web/src/tauri/instrumentation.mockIPC.test.ts`

Required scenarios:

- only implement this file if test setup explicitly bridges:
  - `window.__TAURI__`
  - `globalThis.isTauri`
  - mocked event listeners
- invoke patch begins a tracked command when a mocked invoke starts
- mocked success path completes the tracked command
- mocked failure path fails the tracked command
- optional event listeners ignore malformed payloads
- valid mocked progress/event payloads reach the expected store handlers

Do not attempt to mock every event channel in one test. Keep one focused case per event family.

Do not assume `mockIPC` alone can drive this module in the current repo.

#### `web/src/services/runControlService.mockIPC.test.ts`

Required scenarios:

- only add this file if the missing behavior is not cleaner to cover by extending [runControlService.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\runControlService.test.ts)
- primary `cancel_current_run` success path
- fallback to `cancel_double_ma_optimization` when primary fails
- best-effort `cancel_background_task` call still occurs
- total failure path throws `Cancellation command is unavailable.`

#### `web/src/stores/taskStore.mockIPC.test.ts`

Deferred in the first wave unless a direct Tauri IPC seam is shown to be the missing coverage.

Default repo-safe approach:

- keep task lifecycle additions in normal store tests with explicit Pinia and storage setup
- avoid coupling task-store coverage to instrumentation/Tauri-global behavior unless that is exactly what is under test

#### `desktopProductization.test.ts`

Only extend if needed for a true bridge scenario that current module mocking cannot express more cleanly.

The default assumption is:

- keep existing module-level mocks there unless a real `mockIPC` case becomes necessary

### What not to do

- do not convert broad UI component tests into Tauri-mock tests if plain Vue/component tests are already sufficient
- do not mock every command in the app just because it is possible
- do not make `mockIPC` the default test style for simple store/service logic

### Acceptance

- the repo has targeted Tauri-bridge tests that prove failure-path behavior without launching the desktop shell
- the tested surfaces are the ones that currently sit between unit tests and desktop smoke
- no browser-only or desktop-smoke test was deleted to make room for this layer

## Phase 3: Rust Mock-Runtime / Command Wiring Layer

### Objective

Add targeted Rust-side tests that validate Tauri command/state wiring without requiring a real WebView or full desktop launch.

### Why It Matters

This repo already has many Rust tests, but there is currently no visible usage of:

- `tauri::test`
- `MockRuntime`
- `mock_builder`

That means there is still a gap between:

- core Rust logic tests
- real desktop smoke tests

### Best first targets

Use mock-runtime testing only for command/state wiring that benefits from it:

- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [src-tauri/src/state.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\state.rs)
- [src-tauri/src/commands_optimize.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\commands_optimize.rs)

Do **not** start the first wave in:

- [src-tauri/src/commands_validation.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\commands_validation.rs)
- [src-tauri/src/commands_execution.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\commands_execution.rs)

Those surfaces already depend on datasets, events, handles, or external behavior and are not good minimal mock-runtime starters.

### Exact Rust-side implementation direction

Because [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml) currently sets `[lib] test = false`, the first wave should use integration tests under `src-tauri/tests/`.

Use this exact starting layout:

- `src-tauri/tests/support/mod.rs`
- `src-tauri/tests/mock_runtime_smoke.rs`
- `src-tauri/tests/command_runtime_wiring.rs`

The support module should provide:

- a minimal mock-runtime app builder helper
- managed state installation for:
  - `AppState`
  - `BackgroundTaskState`
- any tiny convenience wrappers needed to obtain an `AppHandle`
- optional mock webview-window creation for tests that need a `main` window label

Do **not** create a large exported test-support surface from the library in the first pass.

Important repo-specific boundary:

- without a reusable production builder helper, these tests should not claim full production invoke-registry coverage
- the realistic first-wave scope is managed-state resolution plus direct command behavior over that state

### Exact first-wave Rust targets

#### `src-tauri/tests/mock_runtime_smoke.rs`

Required first scenarios:

- a mock-runtime app can be built with the repo’s core managed states
- the app-side state objects can be resolved from the mock app handle

Optional scenario only if the support helper explicitly creates the window:

- the `main` window label assumptions required by the frontend remain intact at the config/runtime boundary

Do not assume a `main` window exists automatically. Production config sets `create: false`, and the real app creates the window manually in `setup`.

#### `src-tauri/tests/command_runtime_wiring.rs`

Required first scenarios:

- `cancel_background_task` mutates `BackgroundTaskState` as expected
- `cancel_current_run` and `cancel_double_ma_optimization` toggle `AppState` cancellation intent without needing a real webview

Only after those are green should this layer expand to event-emitting command wrappers.

Important naming/expectation guardrail:

- this file validates state + command smoke, not the exact production invoke registry
- do not describe it as full command-handler coverage unless the repo later accepts either:
  - a shared exported builder helper, or
  - deliberate test-side duplication of the production handler list

### Explicit defer rule for Rust mock-runtime

If the first integration-test attempt proves that the current crate shape makes mock-runtime setup disproportionately expensive, stop and reassess before changing:

- `[lib] test = false`
- public visibility of internal app wiring
- plugin setup in the main app builder

The goal is targeted confidence, not a major `src-tauri` refactor in the name of tests.

Existing repo note:

- `src-tauri/src/*` already contains dormant inline `#[cfg(test)]` modules that are not part of the effective test surface because `[lib] test = false`
- this plan does not reactivate them in the first wave
- if they become important later, handle that as a separate migration/cleanup task

### Good candidate behaviors

- command registration/wiring assumptions
- state bootstrap behavior that does not need the real webview
- command error mapping that should stay deterministic
- app-state transitions that are currently awkward to prove from the frontend

### What not to do

- do not rewrite existing pure Rust unit tests into mock-runtime tests
- do not use mock runtime where a normal function test is already enough
- do not attempt to validate real rendering or WebView behavior in this layer

### Acceptance

- the repo gains targeted `tauri::test` / mock-runtime tests in `src-tauri`
- those tests cover real command/runtime wiring behavior that was not previously exercised
- `cargo test -p ta_desktop_demo_app` remains viable for this added layer

## Phase 4: Desktop Smoke Hardening

### Objective

Add a small amount of native-shell hardening to the existing desktop smoke suite without making it flaky.

### Recommended addition

Add one geometry-focused assertion using WebDriver geometry APIs:

- `getWindowRect`
- `setWindowSize` for the first wave

This is more reliable than simulating drag gestures on the custom titlebar and avoids unnecessary move-state variance with persisted window state.

### Candidate spec target

- [desktop-titlebar-native-controls.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-titlebar-native-controls.spec.ts)
  or
- a new dedicated spec such as `desktop-window-geometry.spec.ts`

Use the new dedicated spec.

### Exact geometry assertion contract

The first geometry spec should do exactly this:

1. call `waitForShellReady()`
2. read the starting rect with `browser.getWindowRect()`
3. set a deterministic alternate size:
   - `width: 1180`
   - `height: 700`
4. assert the returned/current rect is close to that target
   - allow only a small OS rounding tolerance
5. assert these shell surfaces are still visible:
   - titlebar
   - workspace bar
   - run footer
6. open the `settings` workspace and assert [settingsGrid](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue) is still visible
7. restore the original rect in `finally`

Reason:

- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json) currently defaults to `1280x720`
- `1180x700` is a small deterministic change that still exercises layout resilience
- resize-only is lower risk than moving the window while window-state persistence is active

### Hard fail / soft fail policy

The geometry spec should be part of the local default suite only if:

- it is green repeatedly on the user’s Windows machine
- it is green in repeated local opt-in runs using the current skip-build desktop workflow

If it proves flaky, the first response is:

- isolate it from the default green path

Do **not** immediately loosen all geometry assertions until they become meaningless.

### What to assert

- the desktop app window can be resized/repositioned through WebDriver window APIs
- the desktop app window can be resized through WebDriver window APIs
- the shell remains rendered and usable after the geometry change
- responsive shell/layout assumptions still hold at one alternate size

### What to avoid

- no drag-region mouse automation in the default green suite
- no minimize/close behavior that interferes with session stability unless proven stable
- no OS-dialog automation here

### Acceptance

- one stable geometry-oriented assertion exists in the desktop smoke suite
- no flake is introduced into the current green path

## Phase 5: Conditional Future Evaluation

### Objective

Record the valid but conditional next steps from the guide without making them current required work.

### Deferred-but-valid items

1. `@wdio/tauri-service`
2. `@wdio/tauri-plugin`
3. macOS-capable embedded/in-app WebDriver paths

### Trigger conditions

Only evaluate those if one of these becomes true:

- the current `tauri-driver` harness hits real limitations around logs/mocks/driver management
- macOS desktop E2E becomes a real project requirement
- the current harness becomes expensive enough to maintain that the service/plugin path offers a clear reduction in complexity

### Acceptance

- these items remain documented as conditional follow-up, not automatic scope creep

### Exact evaluation threshold for deferred WDIO plugin work

Do not revisit `@wdio/tauri-service` / `@wdio/tauri-plugin` until at least one of these is true:

- the current harness needs backend/frontend log capture beyond what artifacts provide
- the current harness needs deterministic backend command mocking that cannot be covered efficiently by `mockIPC`
- driver setup/maintenance becomes a recurring team cost
- macOS desktop E2E becomes an active requirement

## Open Checklist

### Phase 1: Frontend `mockIPC` Foundation

- [ ] Add a shared frontend Tauri-mocking helper layer under `web/src/test/`.
- [ ] Add wrapper helpers for `mockIPC` and `clearMocks`.
- [ ] Add `mockWindows` helpers only where window-label simulation is actually useful.
- [ ] Document the helper usage pattern near the new test-support layer.
- [ ] Use fail-fast unknown-command behavior by default.
- [ ] Add cleanup helpers so every test file can fully reset IPC and window mocks in `afterEach`.

### Phase 2: High-Value Frontend Tauri-Path Tests

- [ ] Add `web/src/tauri/invoke.mockIPC.test.ts`.
- [ ] Cover success, thrown-string error, thrown-Error object, unknown-command, and args-forwarding cases for [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts).
- [ ] Fix [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts) promise rejection normalization if the new `mockIPC` tests prove the current async path bypasses `TauriRuntimeError`.
- [ ] Add `web/src/tauri/instrumentation.mockIPC.test.ts` only if the test setup explicitly bridges `window.__TAURI__`, `globalThis.isTauri`, and mocked event listeners.
- [ ] Cover tracked-command begin/complete/fail flows and one valid/one malformed event case for [instrumentation.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\instrumentation.ts) only if that bridge exists.
- [ ] Extend [runControlService.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\runControlService.test.ts) or add `runControlService.mockIPC.test.ts`, whichever is the lower-risk way to cover the missing background-task best-effort path.
- [ ] Keep task lifecycle additions in normal store tests unless a direct Tauri IPC seam is shown to be the missing coverage for [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts).
- [ ] Extend [desktopProductization.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.test.ts) only if a real `mockIPC` case is still missing after the above files exist.
- [ ] Keep these tests deterministic and independent of live data or real desktop startup.
- [ ] Leave [invoke.contract.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.contract.test.ts) and [commands.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\commands.test.ts) as simpler contract checks.

### Phase 3: Rust Mock-Runtime / Command Wiring Layer

- [ ] Keep `[lib] test = false` unchanged in the first pass.
- [ ] Add `src-tauri/tests/support/mod.rs`.
- [ ] Add `src-tauri/tests/mock_runtime_smoke.rs`.
- [ ] Add `src-tauri/tests/command_runtime_wiring.rs`.
- [ ] Add a minimal mock-runtime app-builder helper in `src-tauri/tests/support/mod.rs`.
- [ ] Add a smoke assertion that managed `AppState` and `BackgroundTaskState` are accessible from the mock app.
- [ ] Add a `main`-window assertion only if the test support helper explicitly creates that window in setup.
- [ ] Add a state + command smoke assertion for `cancel_background_task`.
- [ ] Add a state + command smoke assertion for `cancel_current_run`.
- [ ] Add a state + command smoke assertion for `cancel_double_ma_optimization`.
- [ ] Keep pure logic tests as plain unit tests where mock runtime adds no value.
- [ ] Keep validation/execution command modules out of the first wave.
- [ ] Do not claim full production invoke-registry coverage unless the repo later accepts a shared builder helper or test-side handler duplication.
- [ ] Reassess crate test-target policy only if the integration-test route proves insufficient.

### Phase 4: Desktop Smoke Hardening

- [ ] Add `web/e2e/desktop/specs/desktop-window-geometry.spec.ts`.
- [ ] Use `getWindowRect` plus `setWindowSize` rather than drag simulation.
- [ ] Resize to `1180x700` without adding move-state assertions in the first wave.
- [ ] Verify titlebar, workspace bar, and run footer remain visible after the geometry change.
- [ ] Verify the Settings workspace still renders correctly after the geometry change.
- [ ] Restore the original rect in `finally`.
- [ ] Keep drag automation out of the default green suite.
- [ ] Keep the geometry spec opt-in until it proves stable across repeated local runs.
- [ ] Keep any flaky native-only assertion out of PR-gating.

### Phase 5: Conditional Evaluation Only

- [ ] Record `@wdio/tauri-service` as deferred unless the current harness proves a real limitation.
- [ ] Record `@wdio/tauri-plugin` as deferred unless deeper execute/mock/log features become worth the added app/test complexity.
- [ ] Record macOS driver-provider work as deferred unless macOS desktop E2E becomes a real requirement.
- [ ] Do not expand CI gating for this plan until the unrelated fresh-build blocker is resolved and the new layers remain stable locally.

## Success Criteria

This plan is successful when:

- the repo has a new deterministic middle test layer on both the frontend and Rust sides
- the current desktop smoke harness remains the thin real-app layer rather than absorbing every scenario
- desktop smoke remains green and low-flake
- the plan does not promise production-builder or remote-CI coverage that the current repo shape cannot yet support
- no regression is introduced in current frontend QA and verification workflows
