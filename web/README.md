# VectorTA Web Frontend

Vue + Vite frontend workspace for the Tauri desktop app.

## Commands

```powershell
npm run typecheck
npm run lint
npm run parity
npm run parity:outputs
npm run test
npm run build
npm run dev
npm run gen:commands
```

## Runtime notes

- `index.html` is now a minimal Vue entrypoint (`#app`) with pre-hydration theme/layout bootstrap.
- `src/App.vue` is the Vue app shell and composes `LegacyCompatibilityRoots`, `TaskCenterOverlay`, boot/runtime banner, and bridge-backed context/workspace Vue surfaces; heavy migrated workspace panels are async-loaded on first activation.
- `src/components/LegacyCompatibilityRoots.vue` mounts the hidden compatibility-root fragment from `src/legacy/template.html`; Vue shell components own the visible layout.
- Legacy runtime startup uses explicit bridge entrypoint `initLegacyApp()` exported from `src/legacy/app.js`.
- `src/legacy/app.js` remains the compatibility controller logic while feature migration continues.
- Pinia is active for Vue state; task tracking is managed through `src/stores/taskStore.ts` and runtime boot state through `src/stores/runtimeStore.ts`.
- `src/main.ts` runs storage schema migrations from `src/utils/storage.ts` on startup while preserving all legacy localStorage keys.
- `src/legacy/bridges/dataContextBridge.ts` synchronizes full Data tab DOM state into `src/stores/dataStore.ts` for Vue-native rendering.
- `src/components/DataWorkspacePanel.vue` now renders a Vue-native Data controls tab UI teleported into the legacy tab host while legacy nodes remain hidden for compatibility.
- `src/legacy/bridges/optimizeValidationBridge.ts` synchronizes optimize + validation DOM state into `src/stores/optimizeStore.ts` and `src/stores/validationStore.ts`.
- `src/legacy/bridges/workflowBridge.ts` synchronizes workspace/workflow navigation state into `src/stores/workflowStore.ts` for Vue quick-switch controls (`src/components/WorkflowQuickSwitch.vue`).
- `src/legacy/bridges/analysisHistorySettingsBridge.ts` synchronizes analysis/history/settings DOM state into `src/stores/analysisStore.ts`, `src/stores/historyStore.ts`, and `src/stores/settingsStore.ts`.
- `src/legacy/bridges/executionDistributedBridge.ts` synchronizes distributed + execution DOM state into `src/stores/distributedStore.ts` and `src/stores/executionStore.ts`.
- Vue-native context overlays now include `src/components/AnalysisContextDock.vue` with delegated actions to legacy handlers.
- `src/components/RunFooterPanel.vue` now renders the run/cancel footer (run plan + readiness badges + status) as Vue-native UI teleported into the legacy controls footer host.
- `src/components/SettingsWorkspacePanel.vue` now renders the non-execution Settings workspace panes as Vue-native UI teleported into the legacy settings host while preserving the migrated Execution pane mount.
- `src/components/OptimizeWorkspacePanel.vue` now renders a Vue-native Optimize controls tab UI teleported into the legacy controls host, including Vue-owned MA-parameter hosts that legacy MA-parameter rendering logic now targets.
- `src/components/StrategyWorkspacePanel.vue` now renders core Strategy controls (cost/risk/constraints/exit) as Vue-native UI teleported into the legacy controls host, and mounts strategy-specific legacy blocks inside the Vue panel host for compatibility.
- `src/components/AdvancedWorkspacePanel.vue` now renders core Advanced controls (source/objective/mode/output shaping) as Vue-native UI teleported into the legacy controls host, and mounts VDPS + Composite-MO legacy blocks inside the Vue panel host for compatibility.
- `src/components/ValidationWorkspacePanel.vue` now renders Validation controls/actions, advanced options, cross-ticker dataset controls/list, shortlist table, raw JSON preview, and report XLSX export flow as Vue-native UI teleported into the legacy results host.
- `src/components/OverviewWorkspacePanel.vue` now renders Overview empty-state, best card/stats/details, and compare controls as Vue-native UI teleported into the legacy results host.
- `src/components/TopWorkspacePanel.vue` now renders Top filter controls and sortable Top results table as Vue-native UI teleported into the legacy results host while keeping legacy table DOM mounted but hidden for compatibility.
- `src/components/TopWorkspacePanel.vue`, `src/components/ValidationWorkspacePanel.vue`, and `src/components/MetaWorkspacePanel.vue` now render result rows through a virtualized row window (spacer-row pattern) to reduce large-table render cost.
- Legacy bridge sync for heavy table payloads now snapshots Top/Validation/Meta/Batch rows only when those tabs are active, reducing background DOM polling cost.
- `src/components/ParetoWorkspacePanel.vue` now renders Pareto axis controls, empty-state, and chart host as Vue-native UI teleported into the legacy results host, while legacy Pareto drawing logic renders into that Vue host.
- `src/components/DrillWorkspacePanel.vue` now renders Drill empty-state and visualization hosts (drill chart, stability, trades) as Vue-native UI teleported into the legacy results host, while existing drill/stability/trades drawing logic renders into those Vue hosts.
- `src/components/HeatmapWorkspacePanel.vue` now renders Heatmap controls, empty-state, and chart host as Vue-native UI teleported into the legacy results host, while legacy heatmap drawing logic renders into that Vue host.
- `src/components/MetaWorkspacePanel.vue` now renders Meta strategy/filter controls, holdout/grouping tuning controls, batch controls/list/output, summary context, meta results table, compare selector/summary, saved-meta manager, saved-stage4 manager, and ML confidence + Stage4 run-tuning controls as Vue-native UI teleported into the legacy results host.
- Validation + Meta tabs now hide all legacy sibling DOM in Vue mode (no passthrough visibility exceptions remain for those tabs).
- `src/components/DebugWorkspacePanel.vue` now renders the Debug tab JSON output as a Vue-native panel teleported into the legacy results host.
- `src/components/HistoryWorkspacePanel.vue` now renders a Vue-native History tab UI (session history + saved runs + detail card) teleported into the legacy tab host while legacy nodes remain hidden for compatibility.
- `src/components/DistributedWorkspacePanel.vue` now renders a Vue-native Distributed tab UI teleported into the legacy controls tab host while legacy nodes remain hidden for compatibility.
- `src/components/ExecutionWorkspacePanel.vue` now renders a Vue-native Paper/Live Execution pane teleported into the legacy Settings workspace host, including a Vue-rendered broker orders table and a Vue execution-chart host (`#execTvChartVue`) that legacy chart rendering now prefers.
- `src/tauri/instrumentation.ts` patches Tauri invoke and subscribes to progress events.
- `src/stores/taskStore.ts` now includes full task lifecycle status (`idle|queued|running|cancelling|success|failed|cancelled`), persistent timeline history, and toast notifications.
- `src/tauri/runtimeError.ts` defines normalized runtime errors (`TauriRuntimeError`) used by `src/tauri/invoke.ts` and `src/tauri/events.ts` for consistent invoke/listen failure handling.
- `src/tauri/commands.ts` is generated from Rust `generate_handler!` via `npm run gen:commands`; `npm run check:commands` and `scripts/check-known-tauri-commands.mjs` gate command drift and unknown literal command strings.
- `src/legacy/uiSurfaceParity.test.ts` statically gates workspace/workflow tabs, controls/results tab sets, command-palette/polygon modal hosts, and required global shortcuts.
- `npm run parity:outputs` runs `scripts/check_output_parity.mjs` against representative JSON/CSV/XLSX artifact pairs to verify output-structure compatibility.
