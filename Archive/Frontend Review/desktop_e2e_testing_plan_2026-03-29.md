# Desktop E2E Testing Plan

Date: 2026-03-29

Scope: implementation-spec plan for adding a real desktop end-to-end smoke harness for the Tauri app using `WebdriverIO + tauri-driver`.

Status: implemented for the first-wave Windows desktop smoke scope.

This document upgrades the earlier roadmap into a concrete implementation spec. It now also records the implemented first-wave harness, the repo-specific launch adjustments that were required in practice, and the current verification status.

## Current Verification Status

Implemented in repo:

- [web/wdio.desktop.conf.ts](C:\Rust Projects\ta_desktop_demo\web\wdio.desktop.conf.ts)
- [web/tsconfig.wdio.json](C:\Rust Projects\ta_desktop_demo\web\tsconfig.wdio.json)
- [web/e2e/desktop/specs](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs)
- [web/e2e/desktop/support](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\support)
- [web/scripts/desktop](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop)
- [.github/workflows/desktop-smoke.yml](C:\Rust Projects\ta_desktop_demo\.github\workflows\desktop-smoke.yml)

Verified locally on Windows:

- `npm --prefix web run test:desktop:smoke`
  - green on `2026-03-30`
  - green both with the normal managed-build path and with the optional `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1` fast path
  - the suite launched the real Tauri shell through `tauri-driver`
  - all six first-wave desktop smoke specs passed

Important repo-specific caveat:

- the earlier local blocker was not a Rust compile failure; it was a Windows process-launch bug in [build-smoke-app.ts](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop\build-smoke-app.ts) when spawning `npm.cmd`
- that helper now runs Windows `.cmd` commands through `cmd.exe /d /s /c`, so the managed fresh-build path works again locally
- hosted CI still needs its own green proof on `windows-latest`; until then, treat the local passing run as the canonical evidence

Important implemented launch adjustment:

- under `tauri-driver`, this repo’s debug desktop session initially surfaced a blank shell context
- the harness now starts a managed Vite preview host at `http://127.0.0.1:1420/`
- if the initial WebDriver context is `about:blank`, the harness explicitly navigates that desktop webview to the preview host before asserting shell readiness
- this keeps the smoke suite useful for real frontend/runtime validation without relying on `cargo tauri dev`

## Why This Exists

The repo already has strong browser-side confidence:

- `vitest` coverage in [web/src](C:\Rust Projects\ta_desktop_demo\web\src)
- browser accessibility smoke in [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)
- screenshot/baseline coverage in [capture-baseline-ui.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\capture-baseline-ui.mjs)
- targeted browser QA artifacts under [output/playwright/ui-review](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review)

What is still missing is the native desktop layer:

- launch of the real Tauri shell
- visibility and readiness of the custom frameless window
- titlebar control behavior in the real desktop runtime
- startup/config/capability mistakes that browser-only testing cannot catch

The goal is strong end-to-end confidence for the frontend as it runs inside the actual Rust Tauri desktop app, not only inside a browser.

## Repo Baseline

Important repo facts that shape this plan:

- the Rust workspace root is [Cargo.toml](C:\Rust Projects\ta_desktop_demo\Cargo.toml)
- the Tauri app crate is [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- the canonical Tauri config is [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- the frontend toolchain lives in [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- the app window is created manually and starts hidden:
  - `visible: false` in [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
  - startup/show behavior in [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- desktop plugin/capability work is already in place:
  - [src-tauri/capabilities/main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
  - [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
  - [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- `withGlobalTauri` is already disabled in [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)

Important consequence:

- this harness must wait for shell readiness rather than assuming process-spawn implies a visible usable window

## Goal

Create a repeatable Windows-first desktop smoke suite that verifies the highest-value user-facing frontend workflows against the actual Tauri desktop app.

The target is not deep backend/system correctness. The target is:

- the Tauri shell launches correctly
- the frontend renders correctly inside the desktop runtime
- key desktop interactions remain functional
- the core frontend workflows still work when exercised through the real desktop app

## Non-Goals

This first harness does not attempt to cover:

- replacement of `vitest` component/service tests
- replacement of existing Playwright browser checks
- deep backend mocking through `@wdio/tauri-plugin`
- tray/updater/notification full automation
- GPU/backtest correctness
- file dialogs
- live-data flows
- a large visual snapshot matrix

## Official Reference Baseline

This plan follows the official Tauri WebDriver path and the official WebdriverIO example shape, but adapts both to this repo’s workspace layout and existing desktop shell.

Reference sources:

- Tauri WebDriver overview: `https://v2.tauri.app/develop/tests/webdriver/`
- Tauri WebdriverIO example: `https://tauri.app/develop/tests/webdriver/example/webdriverio/`
- Tauri WebDriver CI guide: `https://v2.tauri.app/develop/tests/webdriver/ci/`

Important official points that affect this repo:

- `tauri-driver` is the official bridge
- Tauri’s WebdriverIO example uses a small package set:
  - `@wdio/cli`
  - `@wdio/local-runner`
  - `@wdio/mocha-framework`
  - `@wdio/spec-reporter`
- the official examples launch a built app binary through `tauri:options.application`
- the Windows CI path depends on a matching `msedgedriver`

## Platform Policy

This harness is Windows-first.

Reason:

- the user’s development environment is Windows
- Tauri desktop WebDriver support is practical here
- the repo’s immediate need is native smoke coverage on the user’s actual platform

Linux/macOS can be considered later, but they must not complicate the first working harness.

## Primary Stack Decision

Primary first-wave stack:

- `WebdriverIO`
- `tauri-driver`
- matching `msedgedriver.exe` on Windows

Deferred stack:

- `@wdio/tauri-service`
- `@wdio/tauri-plugin`

Reason:

- `tauri-driver` gives real desktop automation against the shipped Tauri shell
- `WebdriverIO` gives a mature runner and assertion model
- the newer WDIO/Tauri plugin layers are useful later for deeper app-aware testing, but they add complexity that is not justified before the base harness exists and proves its boundaries

## Exact First-Wave Implementation Decisions

### Language and config shape

Use TypeScript for the WDIO config and specs.

Why:

- the repo is already TypeScript-heavy
- the first harness benefits from typed helpers/selectors
- this avoids a JS-only island in an otherwise TS frontend toolchain

Implementation choice:

- WDIO config file: `web/wdio.desktop.conf.ts`
- separate TS config: `web/tsconfig.wdio.json`
- specs under `web/e2e/desktop/specs/*.spec.ts`
- support helpers under `web/e2e/desktop/support/*.ts`

### App launch topology

The canonical local and CI launch path is:

1. build the frontend explicitly
2. build the Tauri app crate explicitly in debug
3. start a managed preview host on `127.0.0.1:1420`
4. start `tauri-driver`
5. let `tauri-driver` launch the desktop app binary
6. if the initial desktop context is blank, navigate it to the preview host
7. run WDIO against that real desktop app session

This means:

- do not use `cargo tauri dev` as the base automation path
- do not depend on Vite dev-server timing or HMR
- do not separately launch the app outside `tauri-driver`

### Debug vs release

Use `debug` first.

Reason:

- faster iteration in this Rust workspace
- easier startup failure diagnosis
- same desktop-shell/titlebar/capability/runtime code path
- enough fidelity for smoke coverage

Release builds are a later-tier addition only:

- nightly
- candidate build verification
- pre-release gating

## Exact First-Wave Package Set

Add these to [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json) `devDependencies`:

- `@wdio/cli`
- `@wdio/local-runner`
- `@wdio/mocha-framework`
- `@wdio/spec-reporter`
- `@wdio/globals`
- `tsx`
- `@types/mocha`

Reasoning:

- `@wdio/cli`
  - WDIO runner entrypoint
- `@wdio/local-runner`
  - local single-machine execution
- `@wdio/mocha-framework`
  - simplest first-wave spec model
- `@wdio/spec-reporter`
  - readable minimal output
- `@wdio/globals`
  - explicit imports/types fit this repo’s TS style better than hidden globals
- `tsx`
  - run `wdio.desktop.conf.ts` and TS helpers/specs without adding heavier TS runtime glue
- `@types/mocha`
  - typed `describe` / `it` usage in spec files

Do not add in wave 1:

- `@wdio/tauri-service`
- `@wdio/tauri-plugin`
- `cross-env`
- extra reporters
- separate `webdriverio` direct dependency unless implementation proves it is required beyond the runner package set

## Exact First-Wave File Layout

Add these files:

- [web/wdio.desktop.conf.ts](C:\Rust Projects\ta_desktop_demo\web\wdio.desktop.conf.ts)
  - main WDIO desktop config
- [web/tsconfig.wdio.json](C:\Rust Projects\ta_desktop_demo\web\tsconfig.wdio.json)
  - TS config for WDIO config, specs, and support helpers
- [web/e2e/desktop/specs/desktop-shell-readiness.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-shell-readiness.spec.ts)
- [web/e2e/desktop/specs/desktop-workspace-navigation.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-workspace-navigation.spec.ts)
- [web/e2e/desktop/specs/desktop-command-palette.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-command-palette.spec.ts)
- [web/e2e/desktop/specs/desktop-task-center.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-task-center.spec.ts)
- [web/e2e/desktop/specs/desktop-settings-diagnostics.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-settings-diagnostics.spec.ts)
- [web/e2e/desktop/specs/desktop-titlebar-native-controls.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-titlebar-native-controls.spec.ts)
- [web/e2e/desktop/support/appPaths.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\support\appPaths.ts)
  - resolve repo root, target binary, and artifact directories
- [web/e2e/desktop/support/selectors.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\support\selectors.ts)
  - stable selectors/accessible names used by the specs
- [web/e2e/desktop/support/waits.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\support\waits.ts)
  - shell readiness waits and deterministic polling helpers
- [web/e2e/desktop/support/artifacts.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\support\artifacts.ts)
  - failure screenshot/log capture helpers
- [web/scripts/desktop/build-smoke-app.ts](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop\build-smoke-app.ts)
  - explicit frontend + app debug build helper
- [web/scripts/desktop/verify-prereqs.ts](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop\verify-prereqs.ts)
  - confirm `tauri-driver` and `msedgedriver` availability and echo versions

Keep the first wave under `web/`.

Reason:

- the JS toolchain already lives there
- this keeps desktop smoke close to existing frontend scripts and package management
- it avoids inventing a second Node tool root at the repo root

## Exact npm Script Plan

Add these scripts to [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json):

- `test:desktop:smoke`
  - `wdio run ./wdio.desktop.conf.ts`
- `test:desktop:smoke:debug`
  - `wdio run ./wdio.desktop.conf.ts --logLevel debug`
- `test:desktop:smoke:spec`
  - `wdio run ./wdio.desktop.conf.ts --spec`

Recommended usage:

- from repo root:
  - `npm --prefix web run test:desktop:smoke`
- from `web/`:
  - `npm run test:desktop:smoke`

Do not add a `headed` variant in wave 1.

Reason:

- the Windows Tauri smoke run is already a real desktop window
- a special `headed` concept adds no value here

## Canonical Local Launch Path

The harness should build the app this way:

1. `npm --prefix web run build`
2. `cargo build -p ta_desktop_demo_app`
3. start a preview host on `127.0.0.1:1420`

Then let `tauri-driver` launch:

- `C:\Rust Projects\ta_desktop_demo\target\debug\ta_desktop_demo_app.exe`

Do not separately start the desktop app from the smoke wrapper.

Do not use `cargo tauri dev` as the default smoke topology.

Reason:

- the repo already hit real Tauri CLI cwd/path ambiguity
- the app uses `visible: false` plus restore-before-show behavior, so deterministic startup matters
- a built binary is more stable than a dev-server path for automation

## WDIO Config Requirements

The first-wave [web/wdio.desktop.conf.ts](C:\Rust Projects\ta_desktop_demo\web\wdio.desktop.conf.ts) should follow this shape:

- host: `127.0.0.1`
- port: `4444`
- `maxInstances: 1`
- `framework: 'mocha'`
- `reporters: ['spec']`
- specs:
  - `./e2e/desktop/specs/**/*.spec.ts`
- one suite named `smoke`
- capability shape:
  - only one capability
  - `'tauri:options': { application: resolvedDebugBinaryPath }`

Required runner behavior:

- `onPrepare`
  - verify prerequisites
  - run the explicit build helper
  - start the preview host
  - spawn `tauri-driver`
  - pipe driver stdout/stderr to a log file
- `onComplete`
  - stop the preview host
  - stop `tauri-driver`
- `afterTest`
  - on failure, capture screenshot and useful metadata

Implementation notes:

- keep `maxInstances: 1`
- do not run parallel sessions
- do not add cross-browser capabilities
- do not add a dev-server launch path to the default config

## Windows Prerequisites

Local machine requirements:

- Rust stable
- Node version compatible with [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- `tauri-driver`
- matching `msedgedriver.exe`
- Microsoft Edge installed

Recommended local install steps:

1. install `tauri-driver`
   - `cargo install tauri-driver --locked`
2. install a matching EdgeDriver
   - preferred: download or generate the driver for the currently installed Edge version
   - optional local helper: `msedgedriver-tool`

Important policy:

- do not commit an EdgeDriver binary into the repo
- do not rely on a stale driver staying on `PATH` forever
- the prerequisite helper should print the detected versions so failures are diagnosable
- local fallback is supported through:
  - repo-root `msedgedriver.exe`
  - `VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER`

## Selector Strategy

The first-wave harness must use stable accessible labels and existing shell IDs before introducing any new test-only hooks.

Priority order:

1. existing `aria-label`
2. existing stable IDs
3. existing role + name combinations
4. only then consider adding new stable test hooks

Use these current selectors first:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
  - `aria-label="Desktop titlebar"`
  - `aria-label="Open command palette"`
  - `aria-label="Open settings workspace"`
  - `aria-label="Minimize window"`
  - `aria-label="Maximize window"`
  - `aria-label="Restore window"`
  - `aria-label="Close window"`
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
  - `aria-label="Workspace shell header"`
  - `#workspaceBar`
  - `#settingsShellGrid`
  - `#runFooterShell`
- [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
  - `aria-label="Session history runs"`
- [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
  - `aria-label="Meta analysis sections"`
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
  - `aria-label="Command palette"`
  - `aria-label="Command search"`
  - `aria-label="Available commands"`
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
  - `aria-controls="vta-task-center-panel"`
  - `aria-label="Task center"`
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - `Desktop notifications`
  - `Runtime and task center`
  - `Native desktop`
  - `Desktop logs`
  - `Open task center`
  - `Copy diagnostics snapshot`
  - `Window geometry persists`

Selector anti-patterns for wave 1:

- DOM-depth selectors
- CSS class selectors used only for styling
- fuzzy text scraping across the whole page
- selectors that depend on chart canvas internals

## State And Determinism Policy

First-wave desktop smoke should stay deterministic without adding new app-side test hooks.

Rules:

- do not depend on live market data
- do not depend on broker connectivity
- do not depend on long optimization runs
- do not depend on notification permission dialogs
- do not depend on file dialogs

First-wave specs must navigate explicitly to the workspace they need.

Do not assume:

- the default workspace
- pristine persisted webview state
- specific prior task history

If local state drift becomes a real source of flake later, add a dedicated disposable test profile. Do not block the first harness on that work.

## Exact First-Wave Spec Matrix

### 1. `desktop-shell-readiness.spec.ts`

Purpose:

- prove the desktop app launches and the shell is actually ready

Assertions:

- `[aria-label="Desktop titlebar"]` is visible
- `[aria-label="Workspace shell header"]` is visible
- `#workspaceBar` exists and is a usable tablist
- workspace labels are present:
  - `Workflow`
  - `Run Library`
  - `Aggregate Analysis`
  - `Settings`
- `#runFooterShell` exists

### 2. `desktop-workspace-navigation.spec.ts`

Purpose:

- prove the desktop shell can navigate through the major workspaces

Assertions:

- click `Run Library`
  - corresponding workspace tab becomes selected
  - `aria-label="Session history runs"` appears
- click `Aggregate Analysis`
  - corresponding workspace tab becomes selected
  - `aria-label="Meta analysis sections"` appears
- click `Settings`
  - corresponding workspace tab becomes selected
  - `#settingsShellGrid` appears
  - `Runtime and task center` appears

Wave-1 policy:

- use click navigation first
- do not duplicate browser-side keyboard roving-focus coverage here

### 3. `desktop-command-palette.spec.ts`

Purpose:

- prove the palette works inside the real desktop runtime

Assertions:

- open from `Open command palette` in the titlebar
- `aria-label="Command palette"` is visible
- `aria-label="Command search"` is focused
- `aria-label="Available commands"` exists
- type `settings`
- verify `Go to Settings` is visible
- activate it
- palette closes
- Settings workspace becomes selected
- reopen and close with `Escape`

Wave-1 policy:

- do not assert fuzzy ranking order
- do not open via keyboard shortcut as the only path

### 4. `desktop-task-center.spec.ts`

Purpose:

- prove the mounted desktop task surface is openable and closable in the real shell

Assertions:

- open the launcher targeting `vta-task-center-panel`
- verify `aria-label="Task center"` is visible
- verify section headings exist:
  - `Active posture`
  - `Recent outcomes`
  - `Events`
  - `Timeline`
- close with button or `Escape`
- verify launcher returns to collapsed state

Wave-1 policy:

- do not seed or wait on real tasks

### 5. `desktop-settings-diagnostics.spec.ts`

Purpose:

- prove the desktop diagnostics/config surface survives real Tauri startup

Assertions:

- navigate to Settings
- verify `Desktop notifications`
- verify `Runtime and task center`
- verify `Native desktop`
- verify `Desktop logs`
- verify `Open task center`
- verify `Copy diagnostics snapshot`
- verify native-desktop detail text contains `Window geometry persists`

Wave-1 policy:

- do not trigger OS notification permission UI

### 6. `desktop-titlebar-native-controls.spec.ts`

Purpose:

- prove at least one native-only behavior that browser-only testing cannot prove

Required assertions:

- `Minimize window` button is visible and enabled
- `Maximize window` button is visible and enabled
- `Close window` button is visible and enabled
- click maximize
  - wait until the control reads `Restore window`
- click restore
  - wait until the control reads `Maximize window`

Wave-1 exclusions:

- do not automate minimize in the green path
- do not automate close in the green path
- do not automate drag/move in the green path

## Artifact Policy

All desktop smoke artifacts should live under:

- [output/wdio/desktop-smoke](C:\Rust Projects\ta_desktop_demo\output\wdio\desktop-smoke)

Capture by default:

- per-failure screenshots
- WDIO runner log
- `tauri-driver` stdout/stderr log
- a short environment summary:
  - Node version
  - Rust version
  - `tauri-driver` version
  - `msedgedriver` version if detectable

Do not block wave 1 on capturing every app-internal log file.

That can come later if startup failures prove hard to diagnose.

## Flake Policy

The default green path must stay narrow.

Rules:

- no hard sleeps unless no deterministic wait exists
- prefer `waitUntil` on shell readiness markers
- keep titlebar drag verification out of the default suite until proven stable
- keep minimize/close automation out of the default suite until proven stable
- keep OS permission prompts out of the default suite
- keep real run execution out of the default suite

If a native assertion is flaky:

- quarantine it outside the default smoke suite
- do not normalize flake as “acceptable”

## CI Rollout Strategy

This harness should not be dropped directly into the current [ci.yml](C:\Rust Projects\ta_desktop_demo\.github\workflows\ci.yml) job set.

Recommended rollout:

### Stage 1

- local only
- implementation and stabilization

### Stage 2

- add a separate Windows-only workflow:
  - `.github/workflows/desktop-smoke.yml`
- trigger:
  - `workflow_dispatch`
- build and run the debug desktop smoke suite
- always upload artifacts on failure

### Stage 3

- add scheduled or opt-in PR runs
- keep non-blocking

### Stage 4

- only after repeated stable runs:
  - add path-filtered `pull_request` coverage for:
    - `web/**`
    - `src-tauri/**`
    - `.github/workflows/**`
    - desktop smoke harness files

### Stage 5

- only after stability is proven:
  - consider making desktop smoke required

## Exact CI Job Shape

When CI is added, the Windows job should:

1. checkout
2. install Rust stable
3. install Node
4. install web dependencies
5. run `npm --prefix web run build`
6. run `cargo build -p ta_desktop_demo_app`
7. install `tauri-driver`
8. install or fetch matching `msedgedriver`
9. run `npm --prefix web run test:desktop:smoke`
10. upload `output/wdio/desktop-smoke/**` on failure

Keep this in a separate workflow first.

## Repo Entry Points

Open these first when implementing this plan:

- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [src-tauri/capabilities/main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- [web/scripts/check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)
- [web/scripts/capture-baseline-ui.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\capture-baseline-ui.mjs)
- [web/src/components/DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [web/src/components/VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [web/src/components/TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [.github/workflows/ci.yml](C:\Rust Projects\ta_desktop_demo\.github\workflows\ci.yml)

## Phase Plan

### Phase 1: Harness Bootstrap

Goal: establish a repo-local desktop test runner that can launch the Tauri app under automation on Windows.

Deliverables:

- add the exact WDIO package set to [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- add [web/wdio.desktop.conf.ts](C:\Rust Projects\ta_desktop_demo\web\wdio.desktop.conf.ts)
- add [web/tsconfig.wdio.json](C:\Rust Projects\ta_desktop_demo\web\tsconfig.wdio.json)
- add prerequisite/build helpers under [web/scripts/desktop](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop)
- document local prerequisite setup and expected binary/driver paths

Acceptance:

- one smoke spec can launch the desktop app and reach the main shell
- failure output includes useful logs/screenshots
- another contributor can follow the local setup without reverse-engineering cwd assumptions

### Phase 2: Desktop Launch And Shell Readiness

Goal: verify that the desktop shell launches into a usable state.

Deliverables:

- `desktop-shell-readiness.spec.ts`
- deterministic shell readiness waits
- startup artifact capture on failure

Acceptance:

- the suite can assert shell readiness without arbitrary sleeps
- startup failures surface clearly in test output

### Phase 3: Core Frontend Desktop Smoke Flows

Goal: cover the highest-value user-facing frontend workflows in the real desktop runtime.

Deliverables:

- `desktop-workspace-navigation.spec.ts`
- `desktop-command-palette.spec.ts`
- `desktop-task-center.spec.ts`
- `desktop-settings-diagnostics.spec.ts`

Acceptance:

- each flow runs through the real desktop runtime
- the suite stays deterministic and low-dependency

### Phase 4: Native Desktop Assertions

Goal: add the narrow set of native assertions that browser-only testing cannot prove.

Deliverables:

- `desktop-titlebar-native-controls.spec.ts`
- maximize/restore assertion in the required suite
- explicit defer decision on drag/minimize/close automation

Acceptance:

- the suite proves at least one real native-only behavior beyond browser parity
- native flake is not normalized into the default green path

### Phase 5: CI And Maintenance

Goal: make desktop smoke sustainable without turning it into a maintenance burden.

Deliverables:

- separate Windows desktop-smoke workflow plan
- artifact upload policy
- local troubleshooting doc
- deferred deeper WDIO/Tauri-plugin path clearly recorded

Acceptance:

- contributors know how to run desktop smoke locally
- CI can run it on Windows when desired
- the suite remains small enough to stay trustworthy

## Success Criteria

This plan is successful when:

- the repo has a repeatable desktop smoke harness using `WebdriverIO + tauri-driver`
- the harness launches `target/debug/ta_desktop_demo_app.exe` rather than relying on `cargo tauri dev`
- startup, shell, task center, command palette, settings diagnostics, and titlebar behavior all have baseline desktop coverage
- the suite catches regressions that browser-only tests miss
- the harness is deterministic enough to be useful locally and in Windows CI

## Open Checklist

### Phase 1: Harness Bootstrap

- [x] Add the first-wave WDIO devDependencies to [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json).
- [x] Add [web/wdio.desktop.conf.ts](C:\Rust Projects\ta_desktop_demo\web\wdio.desktop.conf.ts).
- [x] Add [web/tsconfig.wdio.json](C:\Rust Projects\ta_desktop_demo\web\tsconfig.wdio.json).
- [x] Add [web/scripts/desktop/verify-prereqs.ts](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop\verify-prereqs.ts).
- [x] Add [web/scripts/desktop/build-smoke-app.ts](C:\Rust Projects\ta_desktop_demo\web\scripts\desktop\build-smoke-app.ts).
- [x] Add the desktop smoke npm scripts to [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json).
- [x] Document local setup for `tauri-driver` and `msedgedriver`.

### Phase 2: Desktop Launch And Shell Readiness

- [x] Add [web/e2e/desktop/specs/desktop-shell-readiness.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-shell-readiness.spec.ts).
- [x] Add reusable shell readiness waits.
- [x] Capture screenshots/logs on launch failure.
- [x] Verify the custom titlebar is present in the real desktop runtime.

### Phase 3: Core Frontend Desktop Smoke Flows

- [x] Add [web/e2e/desktop/specs/desktop-workspace-navigation.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-workspace-navigation.spec.ts).
- [x] Add [web/e2e/desktop/specs/desktop-command-palette.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-command-palette.spec.ts).
- [x] Add [web/e2e/desktop/specs/desktop-task-center.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-task-center.spec.ts).
- [x] Add [web/e2e/desktop/specs/desktop-settings-diagnostics.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-settings-diagnostics.spec.ts).
- [x] Keep first-wave coverage deterministic and free of live-data dependencies.

### Phase 4: Native Desktop Assertions

- [x] Add [web/e2e/desktop/specs/desktop-titlebar-native-controls.spec.ts](C:\Rust Projects\ta_desktop_demo\web\e2e\desktop\specs\desktop-titlebar-native-controls.spec.ts).
- [x] Automate maximize/restore in the default suite.
- [x] Explicitly defer drag/minimize/close automation unless proven stable.
- [x] Keep flaky native assertions out of the default green path.

### Phase 5: CI And Maintenance

- [x] Add a separate Windows-first desktop smoke workflow plan.
- [x] Add failure artifact upload for `output/wdio/desktop-smoke/**`.
- [x] Document local run instructions and troubleshooting.
- [x] Keep `@wdio/tauri-service` and `@wdio/tauri-plugin` documented as deferred work.
- [x] Reassess PR-gating only after repeated stable local/manual CI runs.

## Recommended Starting Point

Start with Phase 1 and Phase 2 only.

Reason:

- they produce the first real value quickly
- they expose the true integration friction
- they prove whether the official `WebdriverIO + tauri-driver` route is sufficient for this repo before any deeper WDIO/Tauri-plugin work is considered
