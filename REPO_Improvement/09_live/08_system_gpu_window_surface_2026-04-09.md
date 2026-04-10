# System GPU Window Surface Plan

Last updated: 2026-04-10

## Purpose

Define the current desktop system surface around GPU diagnostics, window effects, native titlebar behavior, desktop bridges, and persisted window state so improvements stay additive and platform-truthful.

## Scope Boundaries

In scope:

- GPU status diagnostics exposed to the frontend
- desktop window-effect selection and runtime application
- native titlebar controls and guarded close behavior
- desktop logging, notification, and window-state bridges
- the current WebView/user-data and desktop-runtime assumptions that affect these surfaces

Out of scope:

- optimization/backend GPU implementation details
- live execution command semantics
- options-related desktop surfaces

## Current User-Facing Behavior

The desktop build exposes a system-oriented surface that mixes several adjacent concerns:

- the Settings/diagnostics area can query whether the app was built with CUDA features and whether a CUDA device is currently available
- the UI can persist and reapply a window effect preference, but the accepted effect values vary by OS
- the custom titlebar owns minimize, maximize, close, command-palette, and settings shortcuts
- close is guarded when the live runner or protected long-running tasks are active
- desktop logging, OS-notification routing, and window-state persistence only activate when the app is actually running in Tauri, not in plain browser mode

This surface is desktop-adjacent, but it is not one single feature. GPU diagnostics, window chrome, productization bridges, and close protection need to stay documented as separate contracts that merely meet in the same user-facing area.

## Current Code Surface

- `src-tauri/src/commands_system.rs`
  - owns `get_gpu_status` and `set_window_effect`
- `src-tauri/src/lib.rs`
  - registers the system command surface and desktop plugins
- `src-tauri/tauri.conf.json`
  - carries window creation/default desktop configuration
- `src-tauri/capabilities/main.json`
  - defines the desktop permission/capability envelope
- `web/src/services/desktopProductization.ts`
  - installs window-state saving, desktop log routing, and OS-notification integration
- `web/src/components/DesktopTitlebar.vue`
  - owns native titlebar controls and guarded close UX
- `web/src/components/DesktopTitlebar.test.ts`
  - covers titlebar interaction behavior
- `web/src/stores/settingsStore.ts`
  - persists the window-effect preference and diagnostics-facing desktop settings
- `web/src/services/settingsService.ts`
  - mediates settings persistence and bootstrap
- `web/src/preflight.ts`
  - desktop/browser runtime preflight handoff
- `web/src/tauri/invoke.ts`
  - gates Tauri runtime availability and frontend invoke behavior

## Current Verification Surface

- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/services/desktopProductization.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `src-tauri/tests/settings_tests.rs`
- desktop smoke coverage in `web/e2e/desktop/specs/desktop-titlebar-native-controls.spec.ts`
- desktop smoke coverage in `web/e2e/desktop/specs/desktop-window-geometry.spec.ts`

## Strengths Already Present

- GPU status is already a dedicated backend command instead of hidden in unrelated command payloads
- the desktop bridges already no-op cleanly when Tauri is unavailable
- close protection is already explicit and user-facing rather than implicit process behavior
- window-state persistence is already centralized in one desktop productization layer
- the titlebar already reflects current workflow context instead of being inert chrome

## Weaknesses / Failure Modes

- the system surface combines unrelated responsibilities, which makes drift easy if it is described too loosely
- the frontend can expose effect choices that the current OS/backend rejects or ignores
- `get_gpu_status` is diagnostic truth, not a guarantee that a given workflow route will use the GPU
- persisted desktop/window preferences live alongside browser-local settings, while actual runtime application is Tauri-only
- the WebView data/user-data location matters for what persists where, but that boundary is not obvious from the UI
- close behavior can look like generic window-close semantics even though it is intentionally more conservative because of runners and protected tasks

## Regressions To Avoid

- implying GPU diagnostics are the same thing as route eligibility or benchmark performance
- broadening the effect picker without matching backend/platform support
- weakening the guarded-close behavior for runner-active or protected-task states
- making desktop bridges run in browser mode or fail noisily outside Tauri
- changing window-state persistence or WebView data-location behavior without an explicit compatibility review
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
  - relevant because titlebar actions, maximize/minimize, and close-request behavior must follow Tauri window semantics
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
  - relevant because the desktop/native surface should stay within explicit capability boundaries instead of assuming unrestricted host access
- [Tauri window-state plugin](https://tauri.app/reference/javascript/window-state/)
  - relevant because the repo already persists window geometry/state through this plugin
- [Microsoft WebView2 user data folder](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder)
  - relevant because browser-backed persistence and desktop runtime behavior depend on where the WebView profile data lives
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document the current system surface as four related sub-contracts:
   - GPU diagnostics
   - window effects
   - native titlebar/close behavior
   - desktop productization bridges
2. Make the OS/platform effect matrix explicit so the frontend only promises what the backend can currently honor.
3. Keep `get_gpu_status` positioned as a diagnostics command, not as a promise about route choice.
4. Document the desktop/browser split for log routing, notification permission, and window-state persistence.
5. Preserve the current guarded-close behavior until any future live/runtime contract explicitly changes it.
6. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current GPU diagnostics payload and what it does not guarantee.
- [x] Document the current OS-specific window-effect matrix and rejection behavior.
- [x] Document how the custom titlebar maps to real Tauri window actions.
- [x] Document guarded-close behavior for runner-active and protected-task states.
- [x] Document when desktop productization bridges are active versus unavailable.
- [x] Document current window-state persistence behavior and where it is sourced from.
- [x] Document WebView/runtime persistence assumptions that affect settings and desktop state.
- [x] Add or tighten tests if effect options, titlebar actions, or desktop bridge activation rules change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/components/DesktopTitlebar.test.ts`
- run `npm --prefix web run test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- run `cargo test -p ta_desktop_demo_app --test settings_tests`
- run `npm --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-titlebar-native-controls.spec.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep diagnostics and desktop chrome clarifications additive; do not silently remove current UI affordances
- if unsupported effect values are pruned later, keep migration copy for stored legacy values
- keep browser-mode fallback behavior intact so non-Tauri verification still works
- preserve current close-guard semantics until runner/task cancellation contracts are explicitly revised
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by tightening the system-surface truth anchors without widening desktop behavior:
  - [commands_system.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_system.rs) now has a direct smoke test for `get_gpu_status()` so the shipped diagnostics payload is covered at the command boundary, not only indirectly through optimize flows
  - existing desktop tests already anchor the rest of the contract:
    - titlebar action mapping and guarded-close behavior in [DesktopTitlebar.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.test.ts)
    - desktop bridge activation, notification handling, and window-state metadata in [desktopProductization.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/desktopProductization.test.ts)
    - persisted settings/window-effect application in [settingsStore.actions.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/settingsStore.actions.test.ts), [settingsService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/settingsService.test.ts), and [settings_tests.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/tests/settings_tests.rs)
- Kept scope intentionally additive:
  - no titlebar affordances were removed
  - no window-effect options were broadened
  - no browser-mode fallback behavior changed
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app commands_system --lib`
  - `npm.cmd --prefix web run test -- src/components/DesktopTitlebar.test.ts src/services/desktopProductization.test.ts src/stores/settingsStore.actions.test.ts src/services/settingsService.test.ts`
  - `cargo test -p ta_desktop_demo_app --test settings_tests`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `cargo check -p ta_desktop_demo_app`

## Related Docs

- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/01_shell/24_shell_desktop_productization_2026-04-08.md`
- `REPO_Improvement/09_live/10_run_guards_cancel_tokens_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/15_desktop_native_titlebar_2026-04-09.md`
