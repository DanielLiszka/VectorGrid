# VectorGrid Web Frontend

Vue + Vite frontend workspace for the Tauri desktop app.

## Public Status

This frontend is currently internal work-in-progress. It remains in the repository so the backend/frontend contract is visible, but it is not a supported public application for the current backend/CLI preview release.

For public usage, start from the root `README.md` backend CLI commands. Use the commands below only when intentionally working on the frontend.

## Commands

```powershell
npm run typecheck
npm run lint
npm run parity
npm run test
npm run build
npm run dev
npm run gen:commands
```

- `npm run parity` checks frontend/backend command-surface drift.
- Execution workspace changes should also run the repo-root targeted gate:
  `powershell -ExecutionPolicy Bypass -File ../scripts/run_execution_safety_gate.ps1`.

## Runtime notes

- `index.html` is now a minimal Vue entrypoint (`#app`) with pre-hydration theme/layout bootstrap.
- `src/App.vue` is the Vue app shell; legacy compatibility mounts are removed.
- Pinia is active for Vue state; task tracking is managed through `src/stores/taskStore.ts` and runtime boot state through `src/stores/runtimeStore.ts`.
- `src/main.ts` runs storage schema migrations from `src/utils/storage.ts` on startup while preserving all legacy localStorage keys.
- `src/components/DataWorkspacePanel.vue`, `src/components/OptimizeWorkspacePanel.vue`, `src/components/StrategyWorkspacePanel.vue`, and `src/components/AdvancedWorkspacePanel.vue` own controls rendering.
- `src/components/ValidationWorkspacePanel.vue`, `src/components/OverviewWorkspacePanel.vue`, `src/components/TopWorkspacePanel.vue`, `src/components/ParetoWorkspacePanel.vue`, `src/components/DrillWorkspacePanel.vue`, `src/components/HeatmapWorkspacePanel.vue`, `src/components/MetaWorkspacePanel.vue`, and `src/components/DebugWorkspacePanel.vue` own results rendering.
- `src/components/TopWorkspacePanel.vue`, `src/components/ValidationWorkspacePanel.vue`, and `src/components/MetaWorkspacePanel.vue` now render result rows through a virtualized row window (spacer-row pattern) to reduce large-table render cost.
- `src/components/HistoryWorkspacePanel.vue` and `src/components/ExecutionWorkspacePanel.vue` own their workspace rendering paths.
- `src/tauri/instrumentation.ts` patches Tauri invoke and subscribes to progress events.
- `src/stores/taskStore.ts` now includes full task lifecycle status (`idle|queued|running|cancelling|success|failed|cancelled`), persistent timeline history, and toast notifications.
- `src/tauri/runtimeError.ts` defines normalized runtime errors (`TauriRuntimeError`) used by `src/tauri/invoke.ts` and `src/tauri/events.ts` for consistent invoke/listen failure handling.
- `src/tauri/commands.ts` is generated from Rust `generate_handler!` via `npm run gen:commands`; `npm run check:commands` and `scripts/check-known-tauri-commands.mjs` gate command drift and unknown literal command strings.
- `scripts/check-tauri-parity.mjs` enforces command parity between backend `generate_handler!` and frontend command registry.
