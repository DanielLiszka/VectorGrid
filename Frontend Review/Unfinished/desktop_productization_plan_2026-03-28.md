# Desktop Productization Plan

Date: 2026-03-28

Scope: next-phase implementation plan after the completed `web/`-only remediation pass.

This plan covers the repo changes that are still valid from the recent external review, but were intentionally outside the frontend-only implementation scope:

- Tauri runtime/config cleanup
- desktop-native UX polish
- bounded security/hardening work that is still worth doing for a self-hosted desktop app
- productization-quality diagnostics, notifications, and update flows
- quality gates that keep the growing desktop surface from regressing

This plan does not include:

- backend strategy/compute rewrites
- CUDA/runtime-kernel work
- another broad frontend visual redesign
- removal of any currently exposed backend-backed user capability

## Execution Guardrails

These are mandatory while executing this plan:

- do not remove frontend access to any existing backend-backed user capability without explicit approval
- keep backend contracts fixed unless scope is explicitly expanded
- prefer additive desktop/Tauri productization over broad rewrites of the current Vue shell
- make desktop features reversible where practical:
  - startup-affecting changes should be easy to disable behind config or a small isolated patch
  - plugin adoption should not entangle unrelated UI work
- keep the command-routing model unified:
  - shell buttons
  - command palette
  - tray/menu actions
  - titlebar actions
  - notifications that open/focus the app
  should resolve through the same command/action ownership model where possible

## Platform Policy

The repo currently runs primarily in a Windows development context. Desktop productization should therefore be implemented as:

- Windows-first
- cross-platform-safe where practical
- graceful no-op or reduced behavior where the platform does not support the same capability

Concrete expectations:

- window-state, logging, updater, and notifications should be platform-neutral if the plugin supports them
- titlebar/chrome and window effects must not assume identical behavior across Windows and macOS
- tray/menu behavior should be implemented only if the resulting UX is acceptable on the target platforms actually being shipped
- if a feature is intentionally Windows-only in practice, document that explicitly rather than pretending it is uniformly cross-platform

## Why This Is A Separate Plan

The current frontend shell is already in a materially better state:

- keyboard-first shell navigation is in place
- the command palette, task center, and diagnostics surfaces are real
- dense tables now have a shared headless table path
- depth/elevation tokens, help mode, and bundle splitting already exist

The main remaining delta is no longer ordinary Vue UI debt. It is the desktop/Tauri layer:

- canonical vs mirror Tauri config ownership needed to be made explicit
- a capability model and CSP baseline still needed to exist in repo
- desktop plugins for window state, logging, and notifications were still missing
- the app still feels more like a polished web shell than a fully productized desktop app

## Current Status

This plan is now partially implemented.

Completed in the current pass:

- Phase 1 config/runtime cleanup
- Phase 2 capability inventory and CSP baseline
- Phase 3 window-state and launch polish
- Phase 4 logging and diagnostics plumbing
- Phase 5 OS notifications
- the shipping portion of Phase 6 desktop chrome:
  - custom frameless titlebar
  - shell-mounted window controls
  - desktop-shell accessibility verification for the new chrome
- the quality-gate portion of Phase 8:
  - `eslint-plugin-vuejs-accessibility`
  - Playwright + axe smoke coverage for core desktop shell flows

Still intentionally open:

- the remaining decisions in Phase 6 desktop chrome and presence:
  - tray/menu
  - global shortcuts
- Phase 7 updater and relaunch
- the remaining decisions in Phase 8:
  - Pro editor/diff evaluation
  - any optional heavy power-user tooling

Rollback posture for the completed slice:

- disable the desktop startup service by removing [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts) from [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
- remove the three adopted plugins from [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml) and [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- remove `visible: false` and the `security` block from the canonical and mirrored Tauri configs if startup/CSP behavior regresses
- remove the `main-window` capability from [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json) if capability scoping proves to be the source of a desktop launch regression

Verification completed for the current slice:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`
- `npm --prefix web run check:bundle-budget`
- `cargo check -p ta_desktop_demo_app`
- `node scripts/check_tauri_config_sync.mjs`

## Current Repo Baseline

Current evidence:

- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [tauri.conf.json](C:\Rust Projects\ta_desktop_demo\tauri.conf.json)
- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [web/index.html](C:\Rust Projects\ta_desktop_demo\web\index.html)
- [web/src/tauri/invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts)
- [web/src/components/GlobalShortcutBridge.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\GlobalShortcutBridge.vue)
- [web/src/components/DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [web/src/components/TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [web/scripts/check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)
- [web/eslint.config.js](C:\Rust Projects\ta_desktop_demo\web\eslint.config.js)

Current state summary:

- the Tauri app exposes a large invoke surface through [lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json) is now the canonical config, while [tauri.conf.json](C:\Rust Projects\ta_desktop_demo\tauri.conf.json) is a compatibility mirror kept in sync by [check_tauri_config_sync.mjs](C:\Rust Projects\ta_desktop_demo\scripts\check_tauri_config_sync.mjs)
- `src-tauri/capabilities/` now exists with:
  - [main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json)
  - [dev.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\dev.json)
- CSP is now configured in the canonical and mirrored Tauri configs
- [web/index.html](C:\Rust Projects\ta_desktop_demo\web\index.html) now uses the module-based [preflight.ts](C:\Rust Projects\ta_desktop_demo\web\src\preflight.ts) bootstrap path
- `withGlobalTauri` is disabled and the runtime path is module-first in:
  - [invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts)
  - [events.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\events.ts)
- adopted desktop plugins are now:
  - window state
  - log
  - notification
- current desktop chrome posture:
  - frameless window decorations are enabled in the canonical and mirrored Tauri configs
  - [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue) is now the primary window chrome, with drag-region support, window controls, command-palette access, and run/status context
  - [main.json](C:\Rust Projects\ta_desktop_demo\src-tauri\capabilities\main.json) now grants only the explicit `core:window` permissions needed for drag/minimize/maximize/close behavior
  - desktop presence is now explicit and foreground-first:
    - minimize remains standard window backgrounding
    - close exits by default
    - a narrow close guard is allowed only for meaningful active work
    - close does not hide the app to the tray
  - the titlebar and shell landmarks now pass the browser-side accessibility smoke gate
- still deferred:
  - updater
  - process
  - global shortcut
  - tray/menu
- accessibility/browser quality gates now include:
  - `eslint-plugin-vuejs-accessibility` in the frontend lint stack
  - [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs) for core shell/settings/palette/task-center smoke scans
- the app already has the right frontend anchor points for native polish:
  - Task Center
  - command palette
  - Settings diagnostics
  - custom desktop titlebar
  - window effect settings
  - global in-app shortcut bridge
- current inventory artifacts:
  - [desktop_capability_inventory_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\desktop_capability_inventory_2026-03-28.md)
  - [desktop_plugin_inventory_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\desktop_plugin_inventory_2026-03-28.md)
  - [desktop_ux_decisions_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\Unfinished\desktop_ux_decisions_2026-03-28.md)

## Product Principles

1. Keep the app self-hosted friendly.
   Security hardening matters, but should not be treated like a browser-SaaS emergency.

2. Prefer product-quality desktop UX over novelty.
   Window restore, diagnostics, notifications, and clear command routing matter more than ornamental chrome.

3. Preserve functional coverage.
   Do not remove frontend access to backend-backed capabilities while adding desktop polish.

4. Keep the shell modular.
   New desktop features should connect to the existing command/task/status model rather than create parallel systems.

5. Ship the lowest-risk highest-ROI items first.

6. Treat startup stability as a product feature.
   Desktop plugins and config hardening must not make app launch more fragile than the current baseline.

## Success Criteria

This plan is complete when:

- Tauri config ownership is unambiguous
- the app no longer depends on `window.__TAURI__` as its primary happy path
- a basic capability model exists for the main desktop surface
- CSP is enabled with a bundled-only asset posture
- the window restores cleanly between launches
- high-signal long-running workflow events can surface as OS notifications
- runtime/frontend/backend logs have a defined in-app inspection path
- tray/menu/titlebar decisions are explicit and implemented or intentionally rejected
- desktop presence semantics are explicit:
  - minimize is standard
  - close exits by default
  - active-work close confirmation is narrow rather than routine
- updater readiness is documented and either implemented or deferred behind release-signing prerequisites
- desktop additions ship with accessibility and regression gates
- the desktop productization work preserves the currently documented frontend functional-coverage matrix

## Cross-Phase Deliverables

These outputs should exist by the end of the plan:

- a documented canonical Tauri config decision
- a capability inventory artifact or markdown summary tied to the current command surface
- a plugin inventory documenting:
  - adopted
  - deferred
  - rejected
  with reasons
- a desktop UX decision record for:
  - titlebar
  - tray/menu
  - global shortcuts
  - notifications
- a regression checklist covering:
  - launch
  - focus/keyboard
  - task lifecycle
  - functional coverage
  - basic artifact/report flows

## Implementation Order

1. Config and runtime-baseline cleanup
2. Capability inventory and CSP readiness
3. Window-state restoration
4. Logging and diagnostics plumbing
5. OS notifications
6. Desktop chrome and background-presence work
7. Updater/relaunch flow
8. Power-user tooling and quality gates

## Phase 1: Config And Runtime Baseline

Goal: remove ambiguity and make the current desktop topology explicit before adding plugins.

Primary files:

- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [tauri.conf.json](C:\Rust Projects\ta_desktop_demo\tauri.conf.json)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [web/src/tauri/invoke.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\invoke.ts)
- [web/index.html](C:\Rust Projects\ta_desktop_demo\web\index.html)

Deliverables:

- choose one canonical Tauri config location and document it
- resolve root-vs-`src-tauri` config drift
- audit whether `withGlobalTauri` is still needed
- move the webview bootstrap toward a CSP-safe path
- document stable window labels and any future multi-window intent before capability work begins
- document a rollback posture for startup-affecting config changes

Acceptance:

- only one config is treated as canonical
- no accidental identity/build drift remains
- the invoke path works through module imports without relying on globals
- startup behavior remains equivalent or better than the current baseline

## Phase 2: Capability Inventory And CSP

Goal: add bounded desktop hardening that is still worth doing even for a self-hosted app.

Primary files:

- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [src-tauri/src/commands.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\commands.rs)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [web/index.html](C:\Rust Projects\ta_desktop_demo\web\index.html)

Deliverables:

- create `src-tauri/capabilities/`
- start with at least:
  - `main.json`
  - `dev.json`
- scope main-window permissions to normal operation only
- keep any richer debug permissions separated
- add an initial CSP that supports the bundled frontend without remote assets
- keep fonts/scripts/styles bundled and local
- produce a repo-local command-to-capability inventory so capability decisions are reviewable later

Notes:

- because this is primarily self-hosted, the goal is sane bounded power, not enterprise theater
- updater-specific capability work can wait until updater work is real

Acceptance:

- a missing/overbroad capability is a visible config choice, not an implicit default
- the app runs under an explicit CSP
- the inline bootstrap path is either removed or handled in a CSP-compatible way
- the capability model is understandable from the repo without reverse-engineering the full codebase

## Phase 3: Window State And Launch Polish

Goal: make the desktop app feel persistent and stable between sessions.

Primary files:

- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [web/src/App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [web/src/components/VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Deliverables:

- add `@tauri-apps/plugin-window-state`
- restore size/position/maximized state before showing the window
- prevent first-launch geometry flash
- expose a small Settings-level explanation of restore behavior if needed
- decide what should happen if restore data is corrupt or invalid

Acceptance:

- relaunch restores the prior window state
- launch does not visibly flash a default-sized window before snapping to the saved state
- invalid persisted geometry fails safely rather than trapping the app off-screen

## Phase 4: Logging And Diagnostics

Goal: make failures and long-running workflows easier to trust and debug.

Primary files:

- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [web/src/components/TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [web/src/components/AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)

Deliverables:

- add `@tauri-apps/plugin-log`
- define the frontend attach-console/dev posture
- define how Rust-side logs are surfaced
- expand the current diagnostics surface only where it materially improves operator understanding
- define retention/verbosity expectations so diagnostics do not become a noisy dump

Acceptance:

- frontend and backend runtime signals have one coherent inspection path
- diagnostics remain product-facing rather than raw developer plumbing
- the diagnostics surface remains usable without overwhelming the operator

## Phase 5: OS Notifications

Goal: bridge high-signal task outcomes out of the foreground webview.

Primary files:

- [web/src/stores/taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)
- [web/src/components/TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Deliverables:

- add `@tauri-apps/plugin-notification`
- define the user-facing notification policy
- gate notifications to high-signal events only
- keep in-app status as the source of truth
- define permission-denied and unsupported-platform behavior

Initial candidate events:

- optimization finished
- optimization failed
- validation finished
- validation failed
- report/export finished
- report/export failed
- live runner stopped unexpectedly

Acceptance:

- notifications are opt-in or at least user-respectful
- they do not duplicate every in-app state change
- the event taxonomy matches the existing task lifecycle model
- notification failure does not break the in-app lifecycle UX

## Phase 6: Desktop Chrome And Presence

Goal: decide which desktop-native affordances are worth carrying into the product.

Primary files:

- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)
- [web/src/App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [web/src/components/VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)
- [web/src/components/GlobalShortcutBridge.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\GlobalShortcutBridge.vue)

Deliverables:

- make an explicit decision on custom titlebar/chrome
- if adopted:
  - implement a drag region and minimal window controls
  - scope window permissions tightly
- evaluate tray/menu based on actual long-running/background workflow value
- if tray/menu is adopted, route actions through the same command system used by the shell/palette
- evaluate global shortcuts only for a very small, clearly justified set
- define which of these surfaces are Windows-first only versus expected cross-platform features

Acceptance:

- titlebar/tray/global-shortcut work exists because it improves real workflows, not because it is “desktop-ish”
- app-level actions are not duplicated across multiple inconsistent command systems
- focus, keyboard behavior, and existing shell accessibility do not regress

## Phase 7: Updater And Relaunch

Goal: add product-maturity update behavior only after topology is disciplined enough to support it.

Primary files:

- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [RELEASE.md](C:\Rust Projects\ta_desktop_demo\RELEASE.md)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Deliverables:

- add updater/process plugins only after signing/release decisions are real
- configure endpoints and public key
- define the in-app “check / download / install / relaunch” experience
- define the user-visible behavior for:
  - no update available
  - update check failed
  - download failed
  - install deferred

Acceptance:

- updater UX is intentional and inspectable
- release/signing prerequisites are documented and enforced
- updater work does not weaken the earlier capability/CSP discipline

## Phase 8: Power-User Tooling And Quality Gates

Goal: add the most defensible “premium” improvements without bloating the core shell.

Primary files:

- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- [web/vite.config.ts](C:\Rust Projects\ta_desktop_demo\web\vite.config.ts)
- [web/src/components/SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [web/src/components/CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)

Deliverables:

- add automated accessibility gates:
  - `eslint-plugin-vuejs-accessibility`
  - `@axe-core/playwright`
- keep bundle analysis and lazy-loading discipline explicit as new desktop features land
- evaluate a Pro-surface JSON/editor/diff path only if it materially helps artifact/config review
- define bundle-budget expectations for any heavy optional desktop tooling

Optional later tools, only if justified:

- `vue-codemirror`
- `jsondiffpatch`

Acceptance:

- accessibility regressions are caught earlier than manual review
- any new heavy tooling is lazy-loaded and bounded to the surfaces that need it
- optional power-user tooling does not bloat the base desktop shell

## Implementation Checklist

### Phase 1: Config And Runtime Baseline

- [x] Choose the canonical Tauri config file and remove or retire conflicting config drift.
- [x] Reconcile product name, identifier, and build-path differences between [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json) and [tauri.conf.json](C:\Rust Projects\ta_desktop_demo\tauri.conf.json).
- [x] Audit all uses of `window.__TAURI__` and confirm the module-import invoke path is sufficient.
- [x] Turn off `withGlobalTauri` once the module-import path is confirmed.
- [x] Replace or refactor the inline bootstrap in [web/index.html](C:\Rust Projects\ta_desktop_demo\web\index.html) so CSP can be enabled cleanly.
- [x] Document stable window labels and any planned additional windows before capability work begins.
- [x] Record rollback instructions for this config-cleanup slice in case startup regresses.

### Phase 2: Capability Inventory And CSP

- [x] Create `src-tauri/capabilities/`.
- [x] Add at least `main.json` and `dev.json`.
- [x] Inventory the current invoke command surface by workflow and trust level.
- [x] Record the capability inventory in a repo-local markdown artifact or equivalent reviewable file.
- [x] Scope the main desktop window to only the permissions it actually needs.
- [x] Keep richer debug/diagnostic permissions isolated from the main window.
- [x] Add an initial restrictive CSP to the canonical Tauri config.
- [x] Verify the app still runs with bundled-only assets and no remote script/style dependencies.

### Phase 3: Window State And Launch Polish

- [x] Add `@tauri-apps/plugin-window-state` Rust and JS dependencies.
- [x] Restore window size/position/maximized state on startup.
- [x] Prevent visible launch flash before restored geometry is applied.
- [x] Handle invalid or off-screen restored geometry safely.
- [x] Verify restored state across normal close/relaunch and maximized-window relaunch.

### Phase 4: Logging And Diagnostics

- [x] Add `@tauri-apps/plugin-log` Rust and JS dependencies.
- [x] Define the dev-only console-attach posture.
- [x] Route Rust-side logs into a consistent diagnostics path.
- [x] Extend the existing Settings diagnostics surface only where it materially improves operator understanding.
- [x] Decide retention/verbosity expectations for the in-app diagnostics view.
- [x] Verify diagnostics still make sense without developer tooling attached.

### Phase 5: OS Notifications

- [x] Add `@tauri-apps/plugin-notification` Rust and JS dependencies.
- [x] Define which lifecycle events are notification-worthy.
- [x] Add a user-facing notification preference if the signal volume warrants it.
- [x] Bridge high-signal task outcomes from the existing task model to OS notifications.
- [x] Define permission-denied and unsupported-platform fallback behavior.
- [x] Verify notifications do not spam routine progress updates.

### Phase 6: Desktop Chrome And Presence

- [x] Decide whether custom titlebar/chrome is worth shipping for this app.
- [x] If yes, implement a minimal titlebar with drag region and window controls.
- [x] If yes, scope only the required window permissions.
- [x] Make the desktop presence model explicit:
  - minimize remains standard
  - close exits by default
  - active-work close confirmation stays narrow
- [x] Decide whether tray/menu meaningfully improves long-running workflow UX.
- [ ] If yes, add tray/menu quick actions tied to the existing command system.
- [x] Reassess global shortcuts only after tray/titlebar decisions are settled.
- [ ] If global shortcuts are adopted, keep the set minimal and user-justified.
- [x] Document Windows-first versus cross-platform expectations for titlebar/tray/global-shortcut behavior.

### Phase 7: Updater And Relaunch

- [ ] Confirm signing/release workflow exists before enabling updater UX work.
- [ ] Add updater/process plugins only after signing prerequisites are in place.
- [ ] Configure endpoints, public key, and updater artifacts.
- [ ] Add an in-app check/download/install/relaunch flow.
- [ ] Define the user-facing failure/defer states for updater UX.
- [ ] Verify updater failure states are inspectable and non-destructive.

### Phase 8: Power-User Tooling And Quality Gates

- [x] Add `eslint-plugin-vuejs-accessibility` to the frontend lint stack.
- [x] Add `@axe-core/playwright` checks to the browser-side quality path.
- [x] Keep bundle analysis and lazy-load boundaries explicit as desktop features are added.
- [ ] Evaluate whether a Pro editor/diff surface is still justified after desktop polish lands.
- [ ] If justified, implement `CodeMirror` and/or config/result diffing as a lazy-loaded Pro-only surface.
- [x] Define bundle-budget expectations before adopting any heavy optional tooling.

## Priority Notes

Highest-confidence ROI:

1. config cleanup
2. capability/CSP baseline
3. window-state restore
4. logging/diagnostics
5. OS notifications

Conditional but valid:

6. custom titlebar
7. tray/menu
8. global shortcuts
9. updater/relaunch
10. power-user editor/diff tooling

## Verification Expectations

At minimum for each implementation slice:

- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- targeted browser verification for any new desktop-facing shell behavior
- update the relevant tracking markdown so the plan status remains reviewable

Recommended per-slice Tauri verification:

- `cargo test -p ta_desktop_demo_app`
- targeted launch verification of the desktop shell
- manual confirmation that existing high-stakes frontend capability entry points still exist:
  - data import / cache management
  - optimization run / cancel
  - validation run / review
  - report/export open/reveal flows
  - execution settings and runner controls

For Tauri-scope slices, also verify:

- desktop startup path
- plugin initialization behavior
- settings persistence where applicable
- that no existing backend-backed workflow capability disappeared from the frontend
