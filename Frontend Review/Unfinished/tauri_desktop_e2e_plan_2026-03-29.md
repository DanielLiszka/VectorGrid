# Tauri Desktop E2E Plan

Date: 2026-03-29

Scope: repo-specific implementation plan for adding true desktop end-to-end frontend testing to the Rust Tauri app using WebdriverIO + `tauri-driver`.

Goal: achieve strong automated end-to-end coverage of the frontend as it actually runs inside the Tauri desktop shell, not only in browser-only or mocked-webview mode.

Decision: start with `WebdriverIO + tauri-driver` only.

Deferred on purpose:

- `@wdio/tauri-service`
- `@wdio/tauri-plugin`

Reason:

- the first gap is lack of a real native-desktop smoke harness
- `WebdriverIO + tauri-driver` is sufficient for the first useful layer of desktop E2E coverage
- the newer WDIO Tauri service/plugin layer is better treated as a later enhancement for deeper execute/mocking and deterministic failure-path tests

## Why This Is A Separate Plan

The repo already has meaningful frontend/browser-side QA:

- `vitest` component and service coverage under [web/src](C:\Rust Projects\ta_desktop_demo\web\src)
- browser accessibility smoke in [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)
- browser baseline capture in [capture-baseline-ui.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\capture-baseline-ui.mjs)

Those are valuable, but they do not fully prove desktop-native behavior such as:

- frameless titlebar wiring
- native window controls
- launch visibility and shell readiness inside Tauri
- real desktop-plugin bridge presence
- differences between browser preview behavior and real Tauri window behavior

This plan exists to close that specific gap.

## Current Repo Baseline

Current entry points and constraints:

- Tauri app crate: [src-tauri](C:\Rust Projects\ta_desktop_demo\src-tauri)
- canonical Tauri config: [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- frontend package/scripts: [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- browser smoke scripts:
  - [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)
  - [capture-baseline-ui.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\capture-baseline-ui.mjs)
- current desktop-native surfaces worth smoke-testing first:
  - [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
  - [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
  - [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
  - [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
  - [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)

Current gaps:

- no repo-local `WebdriverIO` desktop harness
- no `tauri-driver` orchestration path
- no deterministic desktop E2E seed mode
- no dedicated native smoke suite for:
  - window controls
  - titlebar drag/control presence
  - desktop diagnostics/plugin readiness
  - desktop-task surfaces in the real shell

## Product Principles

1. Test the real shipped desktop shell first.
   Favor native-shell smoke confidence over premature deep mocking.

2. Keep the first harness deterministic.
   Desktop E2E should not depend on live market data, real broker activity, or external API success.

3. Prefer Windows-first execution.
   The repo is currently developed primarily on Windows, and Tauri WebDriver support aligns with that.

4. Keep browser QA and desktop QA separate.
   Do not replace the current Playwright-based browser smoke suite; add a distinct desktop layer above it.

5. Do not overbuild the first harness.
   The first useful version is:
   - launch
   - navigate
   - interact
   - assert
   - capture evidence

## Non-Goals

This plan does not include:

- replacing the existing `vitest` or Playwright browser checks
- immediately adopting `@wdio/tauri-service` or `@wdio/tauri-plugin`
- testing tray/updater/global-shortcut flows before the base harness exists
- broad backend or Tauri command refactors just to support testing
- fragile pixel-perfect screenshot gating for every desktop flow

## Platform Policy

This harness should be implemented as:

- Windows-first
- local-dev usable
- CI-friendly later

Concrete expectations:

- initial setup should assume Windows + Edge WebDriver compatibility for Tauri WebDriver automation
- the first successful harness may remain Windows-only until proven stable
- cross-platform generalization should be deferred until after the Windows harness is trustworthy

## Recommended Repo Shape

Suggested first layout:

- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
  - add desktop E2E scripts
- `web/wdio.desktop.conf.mjs`
  - primary WebdriverIO config for Tauri desktop E2E
- `web/e2e-desktop/specs/`
  - first-pass WDIO specs
- `web/e2e-desktop/helpers/`
  - app launch, seeding, driver, and assertion helpers
- `web/scripts/run-desktop-smoke.mjs`
  - optional wrapper to orchestrate `cargo tauri dev` / `tauri-driver` / WDIO execution
- `output/wdio/`
  - artifacts such as screenshots, logs, and failure captures

This keeps desktop E2E close to the current frontend tooling while keeping it distinct from the browser-only Playwright scripts.

## Required Launch Strategy

Start with one supported launch mode only:

- `cargo tauri dev` from [src-tauri](C:\Rust Projects\ta_desktop_demo\src-tauri)

Reason:

- fastest iteration loop
- easiest way to validate current desktop wiring
- avoids complicating the first harness with built-app packaging issues

Later expansion:

- built-app / release-candidate desktop smoke can be added after the dev-mode harness is stable

## Deterministic Test-State Strategy

The desktop E2E harness should not rely on live user state.

Required first-pass seeding rules:

- seed the minimal deterministic frontend state needed for smoke scenarios
- avoid live Polygon/download/runtime dependencies
- avoid requiring real reports/runs to exist on disk for base smoke

Preferred first-pass approach:

- a narrow test mode such as `VTA_DESKTOP_E2E=1`
- frontend/runtime reads this only for deterministic seed/bootstrap behavior
- seed:
  - current workspace
  - task-center history/timeline
  - settings diagnostics data where needed
  - any shortlist/history fixtures used by desktop smoke

Acceptable first-pass fallback:

- controlled localStorage/bootstrap seeding if it stays isolated and deterministic

Do not:

- couple the harness to the user's real local profile
- require production-like data imports just to validate shell integrity

## Phase 1: Harness Foundation

Goal: make desktop E2E runnable locally with minimal manual setup.

Deliverables:

- add `WebdriverIO` desktop E2E dependencies to [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- add a desktop E2E config file
- define npm scripts such as:
  - `test:e2e:desktop`
  - `test:e2e:desktop:headed`
  - optional `test:e2e:desktop:spec -- <name>`
- document required local prerequisites:
  - `tauri-driver`
  - matching Windows driver dependency
- ensure artifacts land in a stable repo-local output folder

Acceptance:

- one command launches the Tauri app and the desktop test runner
- failures produce usable logs/screenshots
- local developers do not have to hand-wire multiple terminals to run the suite

## Phase 2: Deterministic Seed Mode

Goal: make desktop smoke tests reliable and repeatable.

Deliverables:

- define the deterministic seed mechanism
- add a minimal E2E bootstrap fixture
- prove at least these smoke states can be reproduced reliably:
  - clean launch
  - populated task-center state
  - settings diagnostics state

Acceptance:

- the first smoke specs do not depend on real external services
- reruns are stable on the same machine
- the harness does not mutate the user’s long-lived local state unexpectedly

## Phase 3: Core Desktop Smoke Coverage

Goal: cover the highest-value user-facing desktop workflows first.

Required first smoke specs:

1. Launch and shell readiness
   - app launches
   - main window becomes visible
   - workspace shell is interactive

2. Titlebar and window controls
   - custom titlebar is present
   - window controls are present
   - command-palette affordance is reachable

3. Workspace navigation
   - switch between:
     - `Workflow`
     - `Run Library`
     - `Settings`

4. Command palette
   - open
   - filter
   - execute one safe navigation command
   - close and restore focus

5. Task Center
   - open
   - validate task posture sections render
   - validate recent outcomes / events / timeline render
   - close cleanly

6. Settings diagnostics
   - diagnostics surface renders
   - desktop notification controls render
   - desktop log controls render
   - copy/clear/filter actions are reachable

Acceptance:

- all six flows run without manual intervention
- failures are attributable from captured artifacts
- no flow depends on live trading/data infrastructure

## Phase 4: Desktop Bridge And Persistence Coverage

Goal: verify the already-adopted desktop-native bridges are actually surfaced correctly.

Deliverables:

- smoke coverage for:
  - window-state readiness surface
  - desktop log bridge surface
  - desktop notification mode controls
- validate that the diagnostics copy/export path remains functional
- validate task-center and settings diagnostics stay aligned in the desktop shell

Acceptance:

- the desktop-specific surfaces are not merely mounted; they are interactable and testable
- regressions in desktop bridge visibility are caught by the suite

## Phase 5: CI And Maintenance Policy

Goal: keep the new harness useful instead of letting it become flaky dead weight.

Deliverables:

- define whether the desktop smoke suite runs:
  - on every PR
  - nightly
  - manually
  - or in a slower Windows-only gate
- document allowed flakiness threshold:
  - effectively zero for the launch/navigation smoke path
- define artifact retention expectations for failures
- document when a new desktop feature must add or update a smoke spec

Acceptance:

- the suite has a clearly defined place in repo verification
- ownership is clear
- failures are actionable rather than noisy

## Explicit Defers

Do not include these in the first implementation:

- `@wdio/tauri-service`
- `@wdio/tauri-plugin`
- tray/menu E2E
- updater flows
- real OS notification delivery assertions
- drag-gesture precision testing
- live broker/data execution flows

These can be layered later once the base harness is stable.

## Future Expansion Path

After the base `WebdriverIO + tauri-driver` harness is stable, the next justified additions are:

1. `@wdio/tauri-service`
   - for cleaner Tauri-aware test ergonomics

2. `@wdio/tauri-plugin`
   - for deeper execute/mocking and deterministic failure-path tests

3. release-build desktop smoke
   - for true packaging/release validation

That order matters. Do not invert it.

## Success Criteria

This plan is successful when:

- the repo has a repeatable native desktop smoke harness
- the harness covers the highest-risk desktop UI flows
- the harness is deterministic enough to trust on repeated local runs
- the harness catches regressions that browser-only Playwright cannot
- the first implementation stays simpler than a full native/mock-heavy test framework

## Open Checklist

### Phase 1: Harness Foundation

- [ ] Choose the exact WDIO config location and desktop E2E folder layout.
- [ ] Add `WebdriverIO` desktop E2E npm dependencies in [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json).
- [ ] Add the first desktop E2E config file.
- [ ] Add npm scripts for headless/headed desktop E2E execution.
- [ ] Define the local prerequisite setup for `tauri-driver` and Windows driver dependency.
- [ ] Define the desktop artifact output folder.

### Phase 2: Deterministic Seed Mode

- [ ] Decide whether the first harness uses a test env flag, controlled bootstrap seed, or another deterministic mechanism.
- [ ] Add a minimal deterministic seed path for desktop smoke.
- [ ] Prove seeded task-center state can be reproduced.
- [ ] Prove seeded settings-diagnostics state can be reproduced.
- [ ] Confirm the harness does not mutate long-lived user state unexpectedly.

### Phase 3: Core Desktop Smoke Coverage

- [ ] Add a launch-and-shell-readiness smoke spec.
- [ ] Add a titlebar/window-controls presence smoke spec.
- [ ] Add a workspace-navigation smoke spec.
- [ ] Add a command-palette smoke spec.
- [ ] Add a task-center smoke spec.
- [ ] Add a settings-diagnostics smoke spec.

### Phase 4: Desktop Bridge And Persistence Coverage

- [ ] Add smoke assertions for window-state readiness visibility.
- [ ] Add smoke assertions for desktop-log bridge visibility and controls.
- [ ] Add smoke assertions for desktop-notification mode controls.
- [ ] Add smoke coverage for diagnostics snapshot copy/export behavior.
- [ ] Verify task-center and settings diagnostics remain aligned in the real desktop shell.

### Phase 5: CI And Maintenance Policy

- [ ] Decide where desktop smoke runs in repo verification.
- [ ] Document artifact capture and failure-handling expectations.
- [ ] Define when future desktop features must update the desktop smoke suite.
- [ ] Add the final checklist/status outcome back into repo routing docs.

## Recommended Starting Point

Start with Phase 1 only.

Do not begin by adding `@wdio/tauri-service` or `@wdio/tauri-plugin`.

The first useful milestone is:

- one command
- real Tauri app launch
- one passing launch/navigation smoke spec
- screenshots/logs on failure
