# System GPU Window Surface Plan

Last updated: 2026-04-09

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

## External Research Summary

- [Tauri window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
  - relevant because titlebar actions, maximize/minimize, and close-request behavior must follow Tauri window semantics
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
  - relevant because the desktop/native surface should stay within explicit capability boundaries instead of assuming unrestricted host access
- [Tauri window-state plugin](https://tauri.app/reference/javascript/window-state/)
  - relevant because the repo already persists window geometry/state through this plugin
- [Microsoft WebView2 user data folder](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder)
  - relevant because browser-backed persistence and desktop runtime behavior depend on where the WebView profile data lives

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

## Checklist

- [ ] Document the current GPU diagnostics payload and what it does not guarantee.
- [ ] Document the current OS-specific window-effect matrix and rejection behavior.
- [ ] Document how the custom titlebar maps to real Tauri window actions.
- [ ] Document guarded-close behavior for runner-active and protected-task states.
- [ ] Document when desktop productization bridges are active versus unavailable.
- [ ] Document current window-state persistence behavior and where it is sourced from.
- [ ] Document WebView/runtime persistence assumptions that affect settings and desktop state.
- [ ] Add or tighten tests if effect options, titlebar actions, or desktop bridge activation rules change.

## Verification Plan

- run `npm --prefix web test -- src/components/DesktopTitlebar.test.ts`
- run `npm --prefix web test -- src/services/desktopProductization.test.ts`
- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`
- run `cargo test -p ta_desktop_demo_app --test settings_tests`
- run `npm --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-titlebar-native-controls.spec.ts`

## Rollout / Fallback / Compatibility Notes

- keep diagnostics and desktop chrome clarifications additive; do not silently remove current UI affordances
- if unsupported effect values are pruned later, keep migration copy for stored legacy values
- keep browser-mode fallback behavior intact so non-Tauri verification still works
- preserve current close-guard semantics until runner/task cancellation contracts are explicitly revised

## Related Docs

- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/01_shell/24_shell_desktop_productization_2026-04-08.md`
- `REPO_Improvement/09_live/10_run_guards_cancel_tokens_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/15_desktop_native_titlebar_DATE.md`
