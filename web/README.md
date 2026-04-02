# VectorGrid Web Frontend

Vue + Vite frontend workspace for the Tauri desktop app.

## Commands

```powershell
npm run typecheck
npm run lint
npm run parity
npm run parity:outputs
npm run test
npm run build
npm run typecheck:desktop
npm run dev
npm run gen:commands
npm run test:desktop:smoke
```

- `npm run parity` checks frontend/backend command-surface drift.
- `npm run parity:outputs` is structural report-artifact parity for the configured representative JSON/XLSX outputs.

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
- `npm run parity:outputs` deterministically regenerates current headless JSON/XLSX candidate artifacts via `ta_headless_validate`, writes a provenance sidecar, and then runs `scripts/check_output_parity.mjs` for structural output compatibility against the checked-in sample baselines.
- `wdio.desktop.conf.ts` plus `e2e/desktop/` own the Windows-first `WebdriverIO + tauri-driver` desktop smoke harness for the real Tauri shell.
- The desktop smoke harness starts its own preview host on `127.0.0.1:1420` and bootstraps blank desktop contexts to that URL when `tauri-driver` initially attaches to `about:blank`.
- Desktop smoke local run instructions and prerequisite notes live in `e2e/desktop/README.md`.
