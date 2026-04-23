# Frontend Follow-up Evaluation

Date: 2026-03-28

Scope: remaining optional checklist items after the main frontend remediation pass.

Purpose:

- resolve the remaining package-evaluation backlog against the current repo state
- record the keyboard-only regression evidence after the latest shell/palette refinements
- classify desktop-native productization items without expanding the current implementation pass beyond frontend scope

Status note:

- the frontend-only follow-up phases in this file are complete
- the next active implementation plan now lives in `Frontend Review/Unfinished/desktop_productization_plan_2026-03-28.md`
- the workstation UX plan in `Frontend Review/workstation_behavior_plan_2026-03-28.md` is now complete for the frontend-only scope it defined
- the current "better use the stack we already ship" plan now lives in `Frontend Review/existing_stack_utilization_plan_2026-03-28.md`
- use that document as the source of truth for open Tauri/runtime desktop-productization work
- use the workstation-behavior plan as the implementation record for saved perspectives, compare/provenance, stale-state cues, density, and bundle-discipline follow-through
- use the existing-stack utilization plan as the source of truth for deeper use of the current table, overlay, search, icon, VueUse, and adopted desktop-plugin stack

## Next Implementation Checklist

This is the active open checklist for the next frontend wave. Unlike the completed remediation checklist in
`frontend_audit_2026-03-27.md`, the items below are intentionally still open.

### Phase 1: Lifecycle / Status Consistency

- [x] Audit remaining task-status, runtime-status, and alert-summary presentation across the major workflow panels.
- [x] Route any remaining duplicated task/runtime wording through the shared status-presentation helpers.
- [x] Standardize severity, tone, and status-detail rendering across:
  - [x] task center
  - [x] runtime banner
  - [x] run footer
  - [x] settings diagnostics
  - [x] any remaining workflow-local status strips that actually present shared task/runtime state
- [x] Define and document the user-facing lifecycle taxonomy for long-running operations so the same states are described consistently across the app.
- [x] Add focused regression coverage for any surfaces moved onto the shared lifecycle/status model.

Current implementation note:

- `statusPresentation.ts` now owns lifecycle labels, severity labels, humanized phase/source formatting, and shared timeline/alert descriptions
- `TaskCenterOverlay.vue` and `SettingsWorkspacePanel.vue` now use that shared presentation layer for recent timeline, alert, and task-posture rendering
- browser QA verified the task center and settings diagnostics surfaces after the change

### Phase 2: Data-Dense Surface Consistency

- [x] Audit remaining manual table/list surfaces and classify them as:
  - [x] already acceptable as list-style UI
  - [x] candidates for the shared headless matrix-table path
  - [x] intentionally bespoke
- [x] Finish migrating any high-ROI leftover matrix-style tables onto the shared headless matrix-table pattern.
- [x] Standardize selection, sorting, and summary semantics across dense results/review surfaces where behavior still drifts.
- [x] Add targeted regression coverage for any migrated table/list surface.

Current audit conclusion:

- the shared headless matrix-table path is already the active foundation for Top, Validation, Meta, and the execution orders table
- History run lists and cached-dataset cards are intentionally list-style surfaces rather than hidden table debt
- cached Polygon dataset cards now follow the same Arrow/Home/End keyboard pattern as the Run Library lists, so the list-style selection surfaces no longer drift in navigation behavior
- the remaining manual drilldown tables are imperative/chart-adjacent surfaces and do not currently justify a forced migration purely for checklist completion

### Phase 3: Form-Architecture Evaluation

- [x] Inventory the densest configuration forms across Strategy, Advanced, Data, and Validation.
- [x] Identify the best pilot surface for a schema-driven form migration rather than attempting a broad rewrite.
- [x] Evaluate `vee-validate` with `@vee-validate/zod` against that pilot surface.
- [x] Record whether a form-state migration is justified now, later, or not at all.

Current evaluation:

- Strategy is still the largest and riskiest form surface by far, with roughly 115 input/select/checkbox controls and substantial strategy-specific branching
- Validation is the next-heaviest setup surface at roughly 48 controls and still leans on store-driven queued writes
- Advanced is materially smaller at roughly 17 controls, but its conditional ranking controls are intertwined with existing store setters
- Data is the best low-risk pilot at roughly 14 controls because the CSV / Polygon setup is bounded, user-facing validation would be clearer there, and a failed pilot would not destabilize the main strategy authoring surface

Decision:

- a schema-driven migration is justified later, not now
- the current frontend phase should not broaden into a form-state rewrite
- if a form migration is started later, `DataWorkspacePanel.vue` is the preferred pilot surface before touching Strategy or Validation

### Phase 4: Diagnostics / Real-Gap Follow-Through

- [x] Re-evaluate the current Settings diagnostics surface after the lifecycle/status pass and identify missing operator-facing context.
- [x] Decide whether the current diagnostics view should remain Settings-scoped or graduate into a more explicit diagnostics surface later.
- [x] Keep the current in-app diagnostics path frontend-only unless scope is explicitly expanded to Tauri/plugin work.

Decision:

- keep diagnostics Settings-scoped for the current frontend phase
- graduate it into a first-class diagnostics workspace only if later Tauri/plugin logging work materially expands what the operator needs to inspect

### Phase 5: Deferred Desktop Productization Track

- [ ] Reassess `@tauri-apps/plugin-window-state` once Tauri-scope work is allowed.
- [ ] Reassess desktop-native notifications once background-operator workflows are clearly defined.
- [ ] Reassess updater UX only after signing/release workflow exists.

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

Decision: partially adopted already; keep it as the shared matrix-table foundation instead of treating it as a future wholesale migration.

Why:

- the repo already uses [@tanstack/vue-virtual](C:\Rust Projects\ta_desktop_demo\web\package.json)
- the repo already has a shared TanStack-backed headless matrix-table composable in [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts)
- the main matrix views already consume that pattern:
  - [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
  - [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
  - [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- the immediate remaining value is to finish consistency on any still-manual data tables or list-heavy inspection surfaces, not to replace whole panels

Refs:

- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
- [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts)
- [useVirtualTableRows.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useVirtualTableRows.ts)

Policy:

- keep `@tanstack/vue-virtual` and the headless matrix-table pattern together
- treat [web/package.json](C:\Rust Projects\ta_desktop_demo\web\package.json) as the explicit source of truth for the dependency instead of relying on transitive resolution

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

### Promoted Real Gap

#### Diagnostics / logging surface

Decision: promoted and implemented as the first real-gap pass inside the current frontend phase.

Why:

- the repo already had partial diagnostics surfaces, but not one coherent in-product place to inspect runtime issues
- this improved UX without waiting for deeper native log plugin work

Current refs:

- [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [DebugWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DebugWorkspacePanel.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [SettingsWorkspacePanel.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.test.ts)

Current implementation note:

- Settings now surfaces runtime state, latest task posture, recent timeline entries, recent alerts, and a direct task-center open action from the existing frontend stores.

Later implementation rule:

- if logging/diagnostics is expanded, it should surface through a clear in-app diagnostics view rather than only invisible plumbing
