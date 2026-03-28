# Frontend Follow-up Evaluation

Date: 2026-03-28

Scope: remaining optional checklist items after the main frontend remediation pass.

Purpose:

- resolve the remaining package-evaluation backlog against the current repo state
- record the keyboard-only regression evidence after the latest shell/palette refinements
- classify desktop-native productization items without expanding the current implementation pass beyond frontend scope

## Keyboard-Only Regression Check

Method:

- reran the shell and command-palette path through Playwright interactive against the local Vite frontend
- used keyboard input only
- verified focus movement, selection state, and palette escape/return behavior

Verified:

- `Tab` enters the workspace tablist on `Workflow`
- `Home` / `End` move cleanly across the workspace tablist
- keyboard activation still switches workspace between `Workflow`, `Run Library`, and `Settings`
- `Tab` enters the workflow-step tablist on `Data`
- `Home` / `End` move cleanly across workflow steps from `Data` to `Finalizer`
- keyboard activation still switches workflow steps, including `Validation`
- `Ctrl+K` opens the command palette without a mouse
- keyboard search and `Enter` still navigate to `Workflow`, `Run Library`, and `Settings`
- `Escape` closes the command palette and restores focus to the previously focused workspace tab

Notes:

- this regression pass was focused on shell/workflow navigation rather than table-row review
- Top Results and Run Library row/list keyboard behavior remain covered separately in component tests

## Package Evaluation Decisions

### Adopted / Active

#### `reka-ui`

Decision: continue expanding it where it cleanly replaces custom headless interaction glue.

Why:

- it is already a real dependency in [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- it already powers the command palette dialog, workflow quick switch popover, and shared help primitive
- it fits the repo's current custom-shell + headless-primitives direction better than adding a full component suite

Current refs:

- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- [ContextHelpTip.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextHelpTip.vue)

Current conclusion:

- tooltip/help usage is already in place
- workflow quick switch is now also using `reka-ui` toolbar/roving-focus primitives
- shell tabs remain intentionally custom for now because they are tightly bound to the workspace shell layout

### Defer

#### `@tanstack/vue-table`

Decision: defer to the next real-gap phase; treat it as the highest-priority table migration candidate.

Why:

- the repo already uses [@tanstack/vue-virtual](C:\Rust Projects\ta_desktop_demo\web\package.json)
- Top Results, Validation, Meta, and History still carry increasingly bespoke table logic
- the migration is worthwhile only when the team is ready to standardize sorting, filtering, column state, and selection architecture across those panels

Refs:

- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
- [useVirtualTableRows.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useVirtualTableRows.ts)

Policy:

- if adopted later, keep `@tanstack/vue-virtual` and integrate the two instead of replacing virtualization

#### `vee-validate` with `@vee-validate/zod`

Decision: defer to a dedicated form-state migration.

Why:

- `zod` is already present, so the schema foundation exists
- the current frontend still relies heavily on store-driven fields and queued input writers, not schema-owned form objects
- adopting it now would be a broad architecture change rather than a small UX upgrade

Refs:

- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- [useQueuedInputWriter.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useQueuedInputWriter.ts)
- [StrategyWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StrategyWorkspacePanel.vue)
- [AdvancedWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AdvancedWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)

#### `vue-echarts` plus `echarts`

Decision: defer until the analysis surfaces genuinely need chart types that the current stack does not handle well.

Why:

- `lightweight-charts` remains the right tool for price/timeline work
- current Pareto and Heatmap views are already purpose-built and do not yet justify the weight of `echarts`
- the additive case remains valid for later non-price analytical visuals

Refs:

- [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
- [useLightweightChart.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useLightweightChart.ts)
- [ParetoWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ParetoWorkspacePanel.vue)
- [HeatmapWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HeatmapWorkspacePanel.vue)

#### Motion helpers such as `@vueuse/motion` or `@formkit/auto-animate`

Decision: defer unless repeated motion problems emerge that are awkward to solve with the current tokenized CSS path.

Why:

- the frontend already has shared motion tokens and reduced-motion-aware transitions in CSS
- a helper library is only justified if repeated overlay/disclosure/list transitions become cumbersome to maintain manually

Refs:

- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [ContextHelpTip.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextHelpTip.vue)

### Reject For Now

#### `ag-grid-vue3`

Decision: reject for now.

Why:

- the app is not presenting spreadsheet-grade datagrids
- the repo is explicitly using a custom shell and headless UI direction
- AG Grid would add weight and product-shape mismatch without solving the current highest-value UX issues

#### `notivue`

Decision: reject for now.

Why:

- the app already has a task-aware async feedback system
- replacing it with a generic notification package would duplicate state and UI rather than simplify it

Refs:

- [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)

#### `vue-sonner`

Decision: reject for now.

Why:

- same reason as `notivue`
- the app needs task-aware progress and lifecycle handling, not only generic toasts

#### `splitpanes`

Decision: reject for now.

Why:

- the current shell already has a custom split layout
- introducing another splitter layer would add churn without filling a missing UX capability

Refs:

- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue)

## Desktop Productization Track Evaluation

These items remain valid product opportunities, but they are not part of the current frontend-only implementation pass.

### Out Of Current Frontend-Only Scope

#### `@tauri-apps/plugin-window-state`

Decision: valid later, but out of current scope.

Why:

- this requires Tauri-side dependency/config/setup work
- the current desktop topology is still minimal

Refs:

- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)

Recorded later requirement:

- if adopted, restore the window before showing it so the launch path does not flash default geometry first

### Valid Later

#### Desktop-native notifications

Decision: valid later, after lifecycle taxonomy is finalized.

Why:

- the current app already exposes in-app task status, timeline, and toasts
- native notifications become more useful once background-window operator flows are explicitly defined

Refs:

- [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)

Recommended later lifecycle events:

- optimization finished
- optimization failed
- validation finished
- validation failed
- export/report save finished
- export/report save failed
- live execution runner stopped unexpectedly
- connection/auth failures that require operator attention

#### Updater UX

Decision: valid later, only after signing and release workflow are real.

Why:

- there is no updater plugin/config/signing pipeline in the current repo
- update UX should be treated as intentional product surface work, not background machinery

Refs:

- [src-tauri/Cargo.toml](C:\Rust Projects\ta_desktop_demo\src-tauri\Cargo.toml)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)

### Real Gap For The Next Phase

#### Diagnostics / logging surface

Decision: this is the desktop-track item most worth promoting into the next frontend gap phase.

Why:

- the repo already has partial diagnostics surfaces, but not one coherent in-product place to inspect runtime issues
- this can improve UX before any deeper native log plugin work lands

Existing fragments:

- [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [DebugWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DebugWorkspacePanel.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Later implementation rule:

- if logging/diagnostics is expanded, it should surface through a clear in-app diagnostics view rather than only invisible plumbing
