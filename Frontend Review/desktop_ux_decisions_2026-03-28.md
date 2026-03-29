# Desktop UX Decisions

Date: 2026-03-28

Purpose: explicit product decisions for the desktop-native UX surfaces in the current productization phase.

## Adopted

### Custom titlebar / window chrome

Decision: adopted.

Why:

- the app already runs in a Windows-first desktop context
- the shell has enough app-level context to justify a true desktop chrome layer
- this is a real workflow improvement, not ornamental chrome:
  - command palette access
  - run / stop action access
  - settings access
  - dataset and task posture visibility
  - native window controls

Current refs:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [tauri.conf.json](C:\Rust Projects\ta_desktop_demo\tauri.conf.json)

Follow-up still open:

- tighten main-window permissions beyond `core:default` if that can be done without destabilizing the current desktop baseline

### OS notifications

Decision: adopted.

Why:

- the app has long-running task flows that can finish outside the operator's immediate focus
- notification routing is already bounded to high-signal events and respects a user-facing preference

Current refs:

- [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)

## Deferred

### Tray / menu

Decision: deferred.

Why:

- there is not yet a proven background-presence workflow that clearly justifies it
- the command palette, titlebar, task center, and settings diagnostics already cover the current operator paths
- adding tray/menu now would increase desktop complexity faster than it would improve the product

Revisit when:

- background run control is a routine workflow
- the app needs reliable show/hide, quit, or quick-run actions while minimized

### Global shortcuts outside the focused webview

Decision: deferred.

Why:

- in-app shortcuts are already strong through the current shell and [GlobalShortcutBridge.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\GlobalShortcutBridge.vue)
- truly global shortcuts are only worth the additional permission surface if operators need out-of-focus control

Revisit when:

- the app needs a global “bring forward and open palette” behavior
- stop/run actions must be reachable while the app is unfocused

### Updater / relaunch UX

Decision: deferred.

Why:

- signing and release-pipeline prerequisites are not ready yet
- updater UX should not ship before the release topology is real

Revisit when:

- signing keys, updater endpoints, and release automation exist

## Quality Gates

### Accessibility gates

Decision: adopted.

Current refs:

- [web/eslint.config.js](C:\Rust Projects\ta_desktop_demo\web\eslint.config.js)
- [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)

Current behavior:

- lint includes `eslint-plugin-vuejs-accessibility`
- browser-side smoke scans cover:
  - workflow shell
  - settings workspace
  - command palette
  - task center overlay

### Bundle discipline

Decision: adopted.

Current refs:

- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- [vite.config.ts](C:\Rust Projects\ta_desktop_demo\web\vite.config.ts)

Current behavior:

- explicit build chunking remains in place
- `check:bundle-budget` guards the current bundle posture before heavier desktop tooling is considered
