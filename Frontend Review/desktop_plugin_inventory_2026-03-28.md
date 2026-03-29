# Desktop Plugin Inventory

Date: 2026-03-28

Purpose: track adopted, deferred, and intentionally open desktop-native plugin decisions for the current Tauri productization pass.

## Adopted

### `tauri-plugin-window-state`

- Rust: [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- JS: [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- init: [lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- capability: [main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
- product role:
  - restore the main window before it becomes visible
  - persist window state on close and page teardown

### `tauri-plugin-log`

- Rust: [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- JS: [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- init: [lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- frontend bridge: [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- diagnostics UI: [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- capability: [main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
- product role:
  - unify frontend and backend runtime signals
  - keep an operator-facing diagnostics trail in-app
  - mirror logs to the browser console only in dev

### `tauri-plugin-notification`

- Rust: [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- JS: [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- init: [lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- frontend bridge: [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- policy UI: [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- capability: [main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
- product role:
  - opt-in OS notifications for high-signal task outcomes
  - keep the in-app task center as the source of truth

## Deferred

### `tauri-plugin-updater`

- reason:
  - release signing and updater endpoint posture are not ready yet
  - enabling updater before the release pipeline is real would add dead UX

### `tauri-plugin-process`

- reason:
  - only needed once updater/relaunch work is real

### `tauri-plugin-global-shortcut`

- reason:
  - in-app shortcut coverage is already strong
  - true global shortcuts should wait until titlebar/tray/background presence decisions are settled

## Intentionally Open Product Decisions

These are still valid, but they were intentionally left out of the current pass:

- tray / menu
- updater / relaunch flow
- global shortcuts outside the focused webview
- plugin-store migration away from localStorage

Adopted desktop UX decisions are tracked separately in:

- [desktop_ux_decisions_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\desktop_ux_decisions_2026-03-28.md)

## Notes

- the current plugin set improves trust and desktop persistence without changing backend strategy or CUDA behavior
- any later desktop plugin should be added only if it clearly improves operator workflow rather than just making the app look “more desktop”
