# Vue + Vite Migration Master Checklist (Tauri v2)

Last updated: 2026-02-21 (US date)

## Goal

Transition from the current plain JS frontend (`dist/index.html`, `dist/app.js`, `dist/app.css`) to a Vue + Vite frontend without removing features or breaking behavior.

Primary UX goal: better user feedback for long-running tasks (progress, status, cancellation, completion/failure visibility).

## Implementation status (2026-02-21)

- [x] Vue + Vite workspace scaffolded in `web/`.
- [x] Tauri build/dev wiring switched to `web/` frontend pipeline.
- [x] Legacy UI runtime ported into Vite source (`web/src/legacy/template.html`, `web/src/legacy/app.js`, `web/src/styles/app.css`).
- [x] Vue-first runtime shell completed (`web/index.html` minimal entry + `web/src/App.vue` app shell + hidden compatibility-root host in `web/src/components/LegacyCompatibilityRoots.vue`).
- [x] Legacy runtime bridge entrypoint established (`initLegacyApp()` called from Vue host) to reduce implicit boot coupling.
- [x] Legacy action bridge added (`window.__VTA_LEGACY_ACTIONS__`) and adopted by run/validation/distributed Vue panels to reduce hidden button-id coupling while preserving legacy behavior.
- [x] Vue workspace panels/context docks now route button-triggered legacy actions through shared `invokeLegacyButton()` helper, removing direct DOM click boilerplate from component code.
- [x] Settings workspace action set (`btnSettings*`) promoted to named legacy actions and registered in the bridge (`settings.*`) to decouple Vue settings UX from hidden settings button wiring.
- [x] Settings market-data-dir set flow now supports structured `settings.setMarketDataDir` payloads (dir + move-existing) from Vue panels, reducing hidden-input coupling in settings UX.
- [x] Execution workspace action set (`btnExec*`) promoted to named legacy actions and registered in the bridge (`execution.*`) to decouple Vue execution UX from hidden execution button wiring.
- [x] Data workspace action set (`btnLoad`, `btnBrowseCsv`, `btnPolygonOpen`, `btnClear`, `btnApplyTimeframe`, and market-data-dir controls) promoted to named legacy actions and registered in the bridge (`data.*`).
- [x] Data workspace Vue panel now invokes named `data.*` actions with structured payloads (CSV path, timeframe, market-data-dir settings), reducing hidden-input/button orchestration; sample dataset bridge snapshots now include explicit CSV paths.
- [x] History workspace action set (`btnClearHistory`, `btnApplyHistory`, preset import/export, saved-run CRUD/actions, runs-dir open) promoted to named legacy actions and registered in the bridge (`history.*`).
- [x] History saved-run flows now accept structured action payloads (`path`/`tags`) for load/apply/rerun/reveal/delete/open-report/tag-save actions, reducing hidden-select coupling in Vue history UX.
- [x] Optimize/Validation/Analysis action set (preset + GPU control, cross-ticker controls, export/report actions, top-filter/heatmap/baseline controls, reports-dir open) promoted to named legacy actions and registered in the bridge (`optimize.*`, `validation.*`, `analysis.*`, `reports.*`).
- [x] Meta workspace action set (batch controls, meta run/report/reveal flows, ML model controls, Stage4 controls, saved-meta/stage4 saved-run managers) promoted to named legacy actions and registered in the bridge (`meta.*`).
- [x] Overview/Advanced helper controls (`btnOverviewEmptyPrimary/Secondary`, `btnCopyBest`, `btnBrowseExportAll`) promoted to named legacy actions and registered in the bridge (`overview.*`, `analysis.copyBest`, `advanced.browseExportAll`).
- [x] Global shortcut ownership moved to Vue (`GlobalShortcutBridge`), with legacy global shortcut registration disabled by startup flag and parity preserved via named legacy action invocations (including command palette open + drill trade navigation helpers).
- [x] Pinia integrated for Vue state (`taskStore`, `runtimeStore`) and runtime boot-status UI (`AppRuntimeBanner`) added.
- [x] Initial feature-domain bridge added: legacy Data context synced into Pinia (`dataStore`).
- [x] Additional feature-domain bridges added: optimize + validation + strategy + advanced context synced into Pinia (`optimizeStore`, `validationStore`, `strategyStore`, `advancedStore`).
- [x] Workflow/navigation bridge added: legacy workspace/workflow state synced into Pinia (`workflowStore`) with Vue quick-switch panel.
- [x] Analysis/history/settings bridges added: legacy context synced into Pinia (`analysisStore`, `historyStore`, `settingsStore`).
- [x] Execution/distributed bridges added: legacy context synced into Pinia (`executionStore`, `distributedStore`) with Vue overlays for live runner + distributed controls.
- [x] Run footer context bridged into Pinia (`optimizeStore`) with Vue-native run/cancel footer panel (`RunFooterPanel`) mounted into the legacy controls footer host.
- [x] Data tab UI migrated to a Vue-native panel (`DataWorkspacePanel`) mounted into the legacy controls tab host; legacy Data DOM remains hidden for behavior compatibility.
- [x] Optimize tab UI migrated to a Vue-native panel (`OptimizeWorkspacePanel`) mounted into the legacy controls tab host; core controls and MA-parameter hosts render in Vue while existing MA-parameter rendering logic renders into those Vue hosts.
- [x] Strategy tab core controls migrated to a Vue-native panel (`StrategyWorkspacePanel`) mounted into the legacy controls tab host; strategy-specific legacy blocks are now mounted inside the Vue panel host for compatibility.
- [x] Advanced tab core controls migrated to a Vue-native panel (`AdvancedWorkspacePanel`) mounted into the legacy controls tab host; VDPS + Composite-MO legacy blocks are now mounted inside the Vue panel host for compatibility.
- [x] Validation tab core controls + detailed options + cross-ticker dataset controls/list + shortlist table + raw JSON preview migrated to a Vue-native panel (`ValidationWorkspacePanel`) mounted into the legacy results tab host.
- [x] Validation report XLSX flow migrated to backend command path (`save_report_xlsx` with `build_report_xlsx` fallback), including auto-export toggle + max-row cap controls and artifact path persistence updates.
- [x] Overview tab migrated to a Vue-native panel (`OverviewWorkspacePanel`) mounted into the legacy results tab host; empty-state, best card/stats/details, and compare controls now render in Vue.
- [x] Top tab filter controls + sortable table migrated to a Vue-native panel (`TopWorkspacePanel`) mounted into the legacy results tab host; legacy table remains mounted but hidden for compatibility.
- [x] Pareto tab migrated to a Vue-native panel (`ParetoWorkspacePanel`) mounted into the legacy results tab host; axis controls, empty-state, and chart host render in Vue while existing Pareto drawing logic renders into the Vue host.
- [x] Drill tab migrated to a Vue-native panel (`DrillWorkspacePanel`) mounted into the legacy results tab host; drill empty-state and chart/stability/trades hosts render in Vue while existing drill/stability/trades drawing logic renders into those Vue hosts.
- [x] Heatmap tab migrated to a Vue-native panel (`HeatmapWorkspacePanel`) mounted into the legacy results tab host; controls, empty-state, and chart host render in Vue while existing heatmap drawing logic renders into the Vue host.
- [x] Meta tab strategy/filter controls + holdout/grouping tuning + batch controls/list/output + results table + compare flow + saved-meta/saved-stage4 managers + ML confidence/Stage4 run-tuning flows migrated to a Vue-native panel (`MetaWorkspacePanel`) mounted into the legacy results tab host.
- [x] Debug tab output migrated to a Vue-native panel (`DebugWorkspacePanel`) mounted into the legacy results tab host.
- [x] Settings workspace non-execution panes migrated to a Vue-native panel (`SettingsWorkspacePanel`) mounted into the legacy settings host; legacy non-execution Settings DOM remains hidden while the migrated execution pane remains mounted.
- [x] History tab UI migrated to a Vue-native panel (`HistoryWorkspacePanel`) mounted into the legacy tab host; legacy History DOM remains hidden for behavior compatibility.
- [x] Distributed tab UI migrated to a Vue-native panel (`DistributedWorkspacePanel`) mounted into the legacy tab host; legacy Distributed DOM remains hidden for behavior compatibility.
- [x] Execution settings pane UI migrated to a Vue-native panel (`ExecutionWorkspacePanel`) mounted into the legacy settings host; legacy Execution DOM remains hidden for behavior compatibility.
- [x] Execution panel now renders broker order rows in Vue (`executionStore.orderRows`) and hosts the legacy execution chart renderer in Vue mode (`#execTvChartVue`/`#execTvChartMetaVue` preferred by legacy runtime).
- [x] Keyboard shortcut parity now runs through Vue (`GlobalShortcutBridge`) while preserving legacy command/action execution paths; legacy global shortcut registration is disabled via startup flag for cutover safety.
- [x] Legacy global action registry on `window` removed; Vue legacy-action adapter now invokes runtime actions through module API (`invokeLegacyRuntimeAction`) with no DOM button-click fallback path.
- [x] Vue-native workspace shell host added (`VueWorkspaceShell`) and migrated panel Teleports now mount to Vue shell roots (`*VueShellMount`) instead of legacy template roots.
- [x] Workspace/workflow/controls/results tab navigation moved to typed runtime actions (`workflow.set*`) and consumed by Vue shell + quick-switch panels.
- [x] Command palette ownership moved to Vue (`CommandPaletteOverlay` + `commandPaletteStore`); global shortcuts now open/close the Vue palette directly.
- [x] Legacy bridge polling timers (`setInterval(syncNow, 1500)`) removed from compatibility bridges; sync now relies on observers/explicit updates.
- [x] Vue panel and context-dock command handlers converted from button-id dispatch (`clickById`) to typed named actions (`runAction('domain.action')`), removing button-id coupling from component templates.
- [x] Static UI surface parity test added (`web/src/legacy/uiSurfaceParity.test.ts`) covering workspace/workflow tabs, controls/results tabs, modal hosts, and required global shortcuts.
- [x] Workflow coverage parity test added (`web/src/legacy/workflowCoverageParity.test.ts`) ensuring all migrated workspace panels are async-registered, activation-gated, and mounted by `App.vue`.
- [x] Frontend progress-state throughput guard added (`web/src/stores/taskStorePerformance.test.ts`) to keep high-frequency task/progress updates lightweight.
- [x] Typed command registry and typed invoke wrapper added.
- [x] Command registry generation/check workflow added (`npm run gen:commands`, `npm run check:commands`) with stale-file gate in CI/lint flows.
- [x] Unknown literal command-name gate added (`scripts/check-known-tauri-commands.mjs`) and wired into `npm run lint`.
- [x] Normalized Tauri runtime error model added (`TauriRuntimeError`) and applied to invoke/event wrappers.
- [x] Global invoke instrumentation added to track all command executions.
- [x] Typed Tauri event wrapper added (`web/src/tauri/events.ts`) and used by instrumentation.
- [x] Progress/event subscriptions wired for `double_ma_progress`, `distributed_progress`, `live_execution_runner_event`, `polygon_live_status`, `polygon_live_bar`.
- [x] Vue task/progress overlay implemented (`web/src/App.vue`) with cancellation actions.
- [x] Task center lifecycle/status model expanded to include `idle|queued|running|cancelling|success|failed|cancelled`, with persistent timeline and toast notifications.
- [x] Storage compatibility adapter added (`web/src/utils/storage.ts`) with schema versioning + startup migration (`runStorageMigrations()` in `web/src/main.ts`).
- [x] `exactOptionalPropertyTypes` enabled with strict typing cleanup.
- [x] Output artifact structure parity checker added (`scripts/check_output_parity.mjs`) with tracked baseline/candidate pairs (`scripts/output_parity_pairs.json`) and frontend runner (`npm run parity:outputs`).
- [x] Error-path QA automation added (`web/src/tauri/errorPathQa.test.ts`) validating invalid input, missing API key, cancellation, and backend-failure handling via instrumented invoke + task center state.
- [x] Legacy `app.js` began modular decomposition (`core/dom`, `core/tauriRuntime`, `core/uiMotion`, `core/storageKeys`, `core/format`, `core/workflowModel`, `core/progressPhases`, `core/ranges`, `core/candleFields`, `core/tabs`, `core/artifactIdentity`, `core/toasts`, `core/path`, `core/download`, `core/heatmap`, `core/datasetMeta`, `core/progressSummary`).
- [x] Automated parity gate added (`npm run parity`) to compare Rust `generate_handler!` commands vs frontend registry.
- [x] Vite manual chunking added to split legacy/vendor bundles and remove large-chunk warning (`legacy-app`, `legacy-core`, `vendor-charts`).
- [x] `App.vue` now async-loads migrated workspace panels and mounts each on first relevant tab/workspace activation.
- [x] Top/Validation/Meta result tables now use row-window virtualization (spacer-row pattern) to cap DOM row rendering cost.
- [x] Frontend validation passing: `npm run typecheck`, `npm run test`, `npm run build`.
- [x] Workflow regression gate (`scripts/run_workflow_gate.ps1 -Mode candidate`) passing against baseline summary (`scripts/baselines/workflow_regression_baseline.json`).
- [x] CUDA workflow regression gate passing (`cargo run -p ta_desktop_demo_app --release --features cuda --bin ta_workflow_regression ...`).
- [x] Release rollback anchors prepared: git tag `pre-vue-vite-migration-2026-02-20` and branch `rollback/vue-vite-precutover-2026-02-20`.
- [x] Migration working branch anchor prepared: `migration/vue-vite-transition-2026-02-20`.
- [x] Migration freeze-scope guard added (`scripts/check_migration_freeze_scope.mjs`) to prevent unrelated refactor drift in the working tree.
- [x] Baseline sample output exporter added (`scripts/export_baseline_samples.mjs`) with generated artifacts under `scripts/baselines/samples/` (`top_rows.csv`, `validation_shortlist.csv`, `run_artifact.json`, `batch_summary.json`, `report.json`, `report.xlsx`, `manifest.json`).
- [x] Baseline UI capture script added (`web/scripts/capture-baseline-ui.mjs`) with generated tab/workspace screenshots and perf snapshot under `scripts/baselines/ui_capture/`.
- [x] Optimization-run memory capture script added (`scripts/capture_optimization_memory.ps1`) with sampled peak memory metrics under `scripts/baselines/ui_capture/optimization_memory_metrics.json`.
- [x] Latest captured perf snapshot: cold start `42ms`, TTI `571ms`, synthetic live-bar update `~0.0485ms/event`, peak optimization working set `39.38MB`.
- [x] Post-release telemetry/log review automation added (`scripts/post_release_telemetry_review.mjs`) with aggregated pass/fail report artifacts in `scripts/baselines/post_release/`.

## Non-Negotiables

- [x] No feature removal.
- [x] No backend command contract breakage.
- [x] Existing Rust/CUDA work remains the performance bottleneck, not frontend state/rendering.
- [x] CPU-only builds remain supported (no CUDA requirement introduced by frontend changes).
- [x] Existing artifact formats and local storage keys remain backward compatible.

---

## 1) Current Baseline Inventory (Source of Truth)

### 1.1 Frontend runtime (today)

- [x] `web/dist/index.html` (UI structure)
- [x] `web/dist/assets/*` (styles/theme/layout + logic/event handling/invoke wiring)
- [x] `web/src/legacy/vendor/lightweight-charts.standalone.production.mjs` (chart source)
- [x] `web/dist/assets/vendor-charts-*.js` (bundled chart chunk)

### 1.2 Tauri wiring

- [x] `src-tauri/tauri.conf.json` uses `frontendDist: "../web/dist"` with `beforeDevCommand`/`beforeBuildCommand` wired to `web/`.
- [x] `tauri.conf.json` uses `frontendDist: "./web/dist"` with `devUrl` set.
- [x] `src-tauri/src/lib.rs` exposes 68 invoke commands via `generate_handler!`.

### 1.3 UI surface areas to preserve

- [x] Workspace tabs: `workflow`, `library`, `aggregate`, `pro`, `settings`
- [x] Workflow steps: `data`, `optimize`, `validate`, `analyze`
- [x] Legacy workflow tabs: `data`, `optimize`, `validate`, `analyze`, `history`, `settings`
- [x] Controls tabs: `data`, `opt`, `strategy`, `advanced`, `distributed`
- [x] Results tabs: `overview`, `top`, `pareto`, `validate`, `drill`, `heatmap`, `history`, `meta`, `debug`
- [x] Modals: Polygon downloader, command palette
- [x] Execution panel and chart overlays

### 1.4 Keyboard shortcuts to preserve

- [x] `Ctrl+K` command palette
- [x] `Ctrl+Enter` run optimization
- [x] `Ctrl+Shift+V` run validation
- [x] `Ctrl+Shift+C` export top rows CSV
- [x] `Ctrl+O` browse CSV
- [x] `Ctrl+\` toggle controls

### 1.5 Event channels already used

- [x] `double_ma_progress`
- [x] `distributed_progress`
- [x] `live_execution_runner_event`
- [x] `polygon_live_bar`
- [x] `polygon_live_status`

### 1.6 Persistence keys to preserve

- [x] `vectorbt_controls_tab_v1`
- [x] `vectorbt_results_tab_v1`
- [x] `vectorbt_settings_v1`
- [x] `vectorbt_history_v1`
- [x] `vectorbt_recent_csv_v1`
- [x] `vectorbt_last_batch_summary_path_v1`
- [x] `vectorbt_exec_keys_v1`
- [x] `vta_workflow_tab_v1`
- [x] `vta_workflow_state_v1`
- [x] `vta_workspace_tab_v1`
- [x] `vta_workflow_step_v1`
- [x] `vta_workflow_advanced_v1`
- [x] `vta_aggregate_tab_v1`
- [x] `vta_controls_width_v1`
- [x] `vta_controls_collapsed_v1`
- [x] `vta_window_effect_v1`
- [x] `vta_theme_pref_v1`
- [x] `vta_dist_settings_v1`

### 1.7 Backend command parity checklist (68 commands)

#### Data / files / paths
- [x] `load_price_data`
- [x] `resample_price_data`
- [x] `list_sample_csvs`
- [x] `list_repo_data_csvs`
- [x] `get_market_data_dir_info`
- [x] `set_market_data_dir`
- [x] `clear_market_data_dir`
- [x] `pick_csv_file`
- [x] `pick_save_csv`
- [x] `pick_folder`
- [x] `open_reports_dir`
- [x] `open_runs_dir`
- [x] `reveal_path`
- [x] `open_path`

#### Polygon data + live feed
- [x] `polygon_get_cache_info`
- [x] `polygon_set_api_key`
- [x] `polygon_clear_api_key`
- [x] `polygon_start_live_candle_stream`
- [x] `polygon_stop_live_candle_stream`
- [x] `polygon_live_candle_stream_status`
- [x] `polygon_validate_dataset`
- [x] `polygon_repair_dataset`
- [x] `polygon_download_1m_and_load`

#### Execution runner
- [x] `execute_strategy_signal_once`
- [x] `start_live_execution_runner`
- [x] `stop_live_execution_runner`
- [x] `live_execution_runner_status`
- [x] `get_live_execution_snapshot`
- [x] `test_alpaca_connection`

#### Optimization / validation / control
- [x] `run_double_ma_optimization`
- [x] `run_composite_cpu_optimization`
- [x] `run_composite_mo_optimization`
- [x] `run_composite_validation`
- [x] `run_double_ma_validation`
- [x] `cancel_current_run`
- [x] `cancel_double_ma_optimization`
- [x] `cancel_background_task`
- [x] `list_moving_averages`
- [x] `get_gpu_status`
- [x] `list_cuda_devices`

#### Distributed
- [x] `distributed_start_worker_server`
- [x] `distributed_stop_worker_server`
- [x] `distributed_worker_server_status`
- [x] `distributed_run_double_ma_assets`
- [x] `distributed_run_double_ma_tiled`

#### Reports
- [x] `build_report_xlsx`
- [x] `build_validation_shortlist_csv`
- [x] `save_report_xlsx`

#### Meta + ML
- [x] `run_meta_selection`
- [x] `run_meta_stage4_experimental`
- [x] `ml_confidence_model_status`
- [x] `train_ml_confidence_model`
- [x] `apply_ml_confidence_model`
- [x] `delete_ml_confidence_model`

#### Artifacts
- [x] `save_run_artifact`
- [x] `save_batch_summary`
- [x] `list_run_artifacts`
- [x] `list_meta_artifacts`
- [x] `list_stage4_artifacts`
- [x] `update_artifact_tags`
- [x] `load_run_artifact`
- [x] `delete_run_artifact`

#### Analysis/drilldown
- [x] `get_price_bars`
- [x] `compute_double_ma_drilldown`
- [x] `compute_composite_cpu_drilldown`
- [x] `compute_double_ma_local_stability`
- [x] `compute_composite_cpu_local_stability`

#### UI/system integration
- [x] `set_window_effect`

### 1.8 Existing coverage gap to explicitly verify

These backend commands exist but are not currently referenced in `dist/app.js` by name in the current code scan:

- [x] `build_report_xlsx`
- [x] `save_report_xlsx`
- [x] `cancel_background_task`

---

## 2) Recommended Target Stack (Version-Pinned Snapshot)

Snapshot collected on 2026-02-19 via npm registry lookups.

### 2.1 Core

- [x] `vue@3.5.28`
- [x] `vite@7.3.1`
- [x] `@vitejs/plugin-vue@6.0.4`
- [x] `typescript@5.9.3`
- [x] `@tauri-apps/api@2.10.1`
- [x] `@tauri-apps/cli@2.10.0`

### 2.2 State/testing

- [x] `pinia@3.0.4`
- [x] `vitest@4.0.18`
- [x] `@vue/test-utils@2.4.6`

### 2.3 Recommended tooling

- [x] `eslint@10.0.0`
- [x] `@typescript-eslint/parser@8.56.0`
- [x] `@typescript-eslint/eslint-plugin@8.56.0`
- [x] `eslint-plugin-vue@10.8.0`
- [x] `prettier@3.8.1`
- [x] `vite-plugin-checker@0.12.0`

### 2.4 Optional but useful

- [x] `vue-router@5.0.2` (if you want route/deep-link architecture; optional for tab-only app)
- [x] `@vueuse/core@14.2.1` (quality composables)
- [x] `zod@4.3.6` (runtime schema validation for IPC payloads)
- [x] `@playwright/test@1.58.2` (E2E)
- [x] `lightweight-charts@5.1.0` (if moving off vendored file)

### 2.5 Environment constraints

- [x] Node runtime pinned to Vite requirement: `^20.19.0 || >=22.12.0`
- [x] Package manager chosen and pinned (`npm` in `web/package.json`)
- [x] Lockfile committed

---

## 3) Migration Strategy (Recommended)

Use an incremental "strangler" migration:

- Keep Rust backend unchanged first.
- Build a typed IPC layer in Vue app.
- Port one feature vertical at a time.
- Maintain strict parity gates after every slice.
- Do not merge a slice that regresses workflows, artifacts, or keyboard/mouse flows.

---

## 4) Execution Checklist (Phased)

## Phase 0: Pre-Migration Guardrails

- [x] Create migration branch.
- [x] Freeze unrelated frontend refactors during migration window.
- [x] Capture baseline screenshots/videos for each major tab/workflow.
- [x] Record baseline performance metrics:
- [x] app cold start time
- [x] time-to-interactive
- [x] memory footprint during optimization run
- [x] chart update responsiveness during live feed
- [x] Export baseline sample outputs:
- [x] top rows CSV
- [x] validation shortlist CSV
- [x] run artifact JSON
- [x] batch summary JSON
- [x] report JSON/XLSX (if used)

## Phase 1: Scaffold Vue + Vite App

- [x] Add `web/` frontend workspace.
- [x] Add `web/package.json` scripts:
- [x] `dev`
- [x] `build`
- [x] `preview`
- [x] `typecheck`
- [x] `lint`
- [x] `test`
- [x] Configure Vite build output to `web/dist`.
- [x] Preserve/align entrypoint handling through Vite + Tauri config (`frontendDist` points to built `web/dist`).
- [x] Align Tauri config:
- [x] `beforeDevCommand` runs frontend dev server
- [x] `devUrl` points to dev server
- [x] `beforeBuildCommand` runs frontend build
- [x] `frontendDist` points to built assets
- [x] Add TypeScript strict mode (`noImplicitAny`, `strictNullChecks`, `exactOptionalPropertyTypes`).

## Phase 2: Typed Tauri IPC Adapter

- [x] Create `web/src/tauri/invoke.ts` with strongly typed command wrappers.
- [x] Create `web/src/tauri/events.ts` wrappers for `listen`/unsubscribe lifecycle.
- [x] Add normalized error model for invoke/event failures.
- [x] Add contract tests for all command wrappers (names + payload shape smoke tests).
- [x] Add a command registry file generated from `src-tauri/src/lib.rs` command names.
- [x] Enforce "no unknown command string" lint rule in frontend.

## Phase 3: State Architecture

- [x] Create Pinia stores (or equivalent) by domain:
- [x] Foundation stores in place: `runtimeStore`, `taskStore`
- [x] `workflowStore` (legacy DOM bridge-backed)
- [x] `dataStore` (legacy DOM bridge-backed)
- [x] `optimizeStore` (legacy DOM bridge-backed)
- [x] `validationStore` (legacy DOM bridge-backed)
- [x] `analysisStore` (legacy DOM bridge-backed)
- [x] `historyStore` (legacy DOM bridge-backed)
- [x] `settingsStore` (legacy DOM bridge-backed)
- [x] `executionStore` (legacy DOM bridge-backed)
- [x] `distributedStore` (legacy DOM bridge-backed)
- [x] Keep chart objects and large arrays out of deep reactivity (`markRaw`, `shallowRef`).
- [x] Add persistence adapter preserving existing keys (1.6 list).
- [x] Add storage schema versioning + migration helpers.

## Phase 4: UX Feedback Infrastructure (Priority)

- [x] Add global task center model:
- [x] `idle | queued | running | cancelling | success | failed | cancelled`
- [x] `progressPct`, `phase`, `message`, `startedAt`, `endedAt`, `error`
- [x] Add top-level progress bar + per-task card.
- [x] Add cancellable actions where backend supports cancellation.
- [x] Show explicit "busy" states for buttons/controls while running.
- [x] Add persistent run timeline (start, phase transitions, completion, failure).
- [x] Add toast/snackbar notifications for completion and errors.
- [x] Normalize progress event payload adapters for:
- [x] `double_ma_progress`
- [x] `distributed_progress`
- [x] `live_execution_runner_event`
- [x] `polygon_live_status`
- [x] Throttle high-frequency UI updates (target ~4-10Hz for logs/progress).

## Phase 5: Feature Slice Migration (No Big-Bang)

### Slice A: Data + Dataset Context
- [x] CSV load/browse
- [x] sample/repo datasets
- [x] timeframe resampling/apply
- [x] dataset summary/columns display
- [x] market data folder management

### Slice B: Strategy Setup + Optimization
- [x] backend/device selection
- [x] MA type/range editing
- [x] presets
- [x] run/cancel path
- [x] progress mapping + table population

### Slice C: Validation + Reports
- [x] validation run
- [x] shortlist CSV export/build
- [x] report build/save flow parity
- [x] parity checks between old/new output files

### Slice D: Analysis/Drilldown/Heatmap
- [x] get price bars
- [x] drilldown computation
- [x] local stability computation
- [x] charts + markers + overlays

### Slice E: Artifacts/History/Meta
- [x] save/load/delete run artifacts
- [x] tags update
- [x] meta/stage4 artifact flows
- [x] batch summary persistence

### Slice F: Distributed + Execution Runner
- [x] distributed worker server controls
- [x] asset/tiled distributed runs
- [x] live execution runner controls + status stream
- [x] broker order table refresh + chart overlay

### Slice G: Global Settings + Command Palette
- [x] theme/effect settings
- [x] workspace behavior toggles
- [x] keyboard shortcut parity
- [x] command palette commands parity

## Phase 6: Performance Hardening

- [x] Verify no expensive watchers on large tables/charts.
- [x] Virtualize long tables/lists.
- [x] Defer/lazy-load heavy panels not initially visible.
- [x] Keep chart updates imperative; avoid full component rerender loops.
- [x] Compare baseline metrics from Phase 0 and confirm no regressions.

## Phase 7: QA Gates Before Cutover

- [x] Command parity: all 68 commands verified (manual + automated smoke).
- [x] Event parity: all 5 event channels verified.
- [x] Storage parity: all keys preserved/migrated.
- [x] Output parity: exported artifacts match expected structure.
- [x] UI workflow parity: all tabs/panels reachable and functional.
- [x] Error-path QA: invalid inputs, missing API keys, cancelled runs, backend failures.
- [x] CPU-only run verified.
- [x] CUDA run verified (if environment available).

## Phase 8: Release + Rollback

- [x] Tag pre-migration release commit.
- [x] Keep rollback branch/patch ready.
- [x] Release notes include known deltas and migration notes.
- [x] Post-release telemetry/log review for regressions.

## Phase 9: Full Legacy Runtime Cutover (Pending)

These items are required for a true non-hybrid frontend where Vue is the runtime owner (not just the presentation layer):

Dedicated Phase 9 execution checklists:
- `VUE_VITE_PHASE9_RUNTIME_OWNERSHIP_CHECKLIST.md` (Phase 9A: runtime ownership migration)
- `VUE_VITE_PHASE9_TEMPLATE_DECOMMISSION_CHECKLIST.md` (Phase 9B: template/runtime decommission)

- [x] Trim hidden compatibility roots (`legacy/template.html`) down to the minimum required IDs/elements now that `LegacyHost` has been removed from `App.vue`.
- [x] Migrate remaining `legacy/app.js` runtime ownership (bootstrapping, command palette, shortcut wiring, tab/workflow state machine) into Vue services/composables.
- [x] Remove hidden-input/hidden-button state mirrors from all Vue panels and drive command execution from typed Vue actions/stores.
- [x] Remove `window.__VTA_LEGACY_ACTIONS__` fallback button-click path after all mapped actions have Vue-native implementations.
- [x] Decommission unused markup in `web/src/legacy/template.html` and continue reducing `web/src/legacy/app.js` until it can be removed.

Progress notes:

- [x] `window.__VTA_LEGACY_ACTIONS__` references removed from frontend runtime and typings.
- [x] `invokeLegacyButton()` fallback API removed from `web/src/legacy/actions.ts`; all Vue command dispatch now goes through typed named runtime actions only.
- [x] Vue component templates no longer dispatch commands by hidden button-id (`clickById`); they dispatch named typed actions (`runAction('domain.action')`).
- [x] Workflow/workspace/navigation command ownership moved to typed action layer (`workflow.set*`) and consumed by Vue shell/quick-switch.
- [x] Legacy bridge timer polling removed (`setInterval(syncNow, 1500)` => observer-driven updates).
- [x] Remaining Vue-side DOM click bridges removed for tab/sort/list interactions (`workflow.setResultsTab`, `workflow.setAggregateTab`, `analysis.sortTopTable`, `analysis.selectTopRow`, `analysis.rerenderDrill`, `validation.toggleCrossDataset`, `meta.toggleBatchDataset`, `data.loadSampleCsv`).
- [x] Legacy runtime bootstrap/bridge attachment split from markup host (`LegacyRuntimeBridge.vue` + template-only `LegacyCompatibilityRoots.vue`) so runtime ownership can continue migrating without template-coupled startup logic.
- [x] `LegacyHost` removed from `App.vue`; `legacy/template.html` now mounts as a hidden compatibility-root fragment (no `html/head/body` wrappers or inline startup script ownership in Vue runtime).
- [x] Legacy template compatibility cleanup: removed obsolete `*VueMount` placeholders and stale IDs; current `legacy/template.html` ID audit reports zero unreferenced IDs across `web/src`.
- [x] Legacy command-palette modal host removed from `legacy/template.html`; runtime `openCmdPalette`/`closeCmdPalette` now route through Vue bridge events (`vta:command-palette-open`/`...-close`) with legacy DOM as fallback.
- [x] Legacy command-palette runtime implementation removed from `legacy/app.js` (legacy command list/render/input handlers deleted); `legacy/app` now keeps only a thin compatibility bridge to Vue command-palette state.
- [x] All remaining Vue component field updates migrated from `ui.setInputValue`/`ui.setSelectValue`/`ui.setCheckboxValue` to domain-scoped typed actions (`*.setField` with `kind` payloads).
- [x] Legacy `ui.set*` action names removed from frontend typed unions and `LEGACY_ACTION_REGISTRY`; those handlers are now internal-only via domain `*.setField` compatibility dispatch.
- [x] Quick-bar and keyboard-shortcut command paths in `legacy/app.js` now call typed runtime action handlers directly (no hidden button `.click()` proxy for run/cancel/validate/export/browse/open-report flows).
- [x] Workflow bridge moved from DOM `MutationObserver` polling to event-driven snapshots (`vta:workflow-snapshot`) emitted by legacy workflow state transitions, reducing observer churn and tightening Vue runtime ownership boundaries.
- [x] Workflow navigation action handlers now short-circuit no-op transitions (same workspace/workflow/step/controls/results/aggregate selections), and Vue workflow senders guard against re-dispatching already-active selections.
- [x] Legacy context bridges now coalesce rapid observer/input bursts to one sync per frame (`createCoalescedSync`), reducing repeated Pinia snapshot writes during high-frequency DOM update phases.
- [x] Legacy workflow/aggregate/workspace nav bars now support headless mode (`attachListeners: false`) and run without click/keyboard listener binding when Vue is the active workflow owner (`window.__VTA_VUE_WORKFLOW_OWNER__ = true`).
- [x] Legacy workflow/aggregate initialization no longer hard-depends on nav bar markup presence; bootstrap state restore/apply now still executes in headless mode even if those legacy bars are absent.
- [x] Workflow subtitle is now tracked in runtime state (`CURRENT_WORKFLOW_SUBTITLE`) and emitted via workflow snapshot events, removing strict dependency on reading `#workflowSubtitle` DOM text for Vue workflow-store sync.
- [x] `legacy/template.html` is now a hidden compatibility-root fragment (no `html/head/body` wrappers or inline startup-script ownership); `App.vue` mounts `LegacyCompatibilityRoots` instead of `LegacyHost`.
- [x] `execution.setField` now applies core execution field updates directly in runtime (no synthetic input/change dispatch for those fields) and emits `vta:execution-snapshot` so the execution bridge can sync immediately.
- [x] `distributed.setField` now applies distributed field updates directly in runtime (no synthetic input/change dispatch for those fields) and emits `vta:distributed-snapshot` so the distributed bridge can sync immediately.
- [x] `data.setField` now applies core data field updates directly in runtime (`csvPath`, `marketDataDir`, `dataTimeframe`, `marketDataMoveExisting`, `autoRestoreDataset`) and emits `vta:data-snapshot` so the data bridge can sync immediately.
- [x] `history.setField` now applies history selector/search/tag updates directly in runtime (`historySelect`, `savedRunsSelect`, `savedRunsSearch`, `savedRunsTags`) and emits `vta:history-snapshot` so the history bridge can sync immediately.
- [x] `optimize.setField` now applies optimize field updates directly in runtime (`strategyKind`/backend/device/objective/mode/ranges/MA types) with explicit side-effect updates (run-plan, pairs-preview, MA param refresh, backend constraints) and emits `vta:optimize-snapshot`.
- [x] `strategy.setField` now applies strategy field updates directly in runtime (cost/risk/constraints/exit fields), runs cost-model/strategy-ui side effects, and emits `vta:strategy-snapshot`.
- [x] `advanced.setField` now applies advanced field updates directly in runtime (`maSource`/`objective`/`mode`/`heatmapBins`/`topK`/`includeAll`/`exportAllCsvPath`), runs run-plan/MA-source side effects, and emits `vta:advanced-snapshot`.
- [x] `validation.setField` now applies validation field updates directly in runtime (core numeric/search fields + validation toggles), preserves validation/run-plan side effects, and emits `vta:validation-snapshot`.
- [x] `analysis.setField` now applies analysis field updates directly in runtime (top filters, compare selector, Pareto axes, heatmap controls), preserves compare/filter/heatmap render side effects, and emits `vta:analysis-snapshot`.
- [x] `meta.setField` now applies meta/batch/ML/Stage4 field updates directly in runtime (including search/select/toggle fields), preserves selection/search/holdout/grouping/compare side effects, and reuses `vta:analysis-snapshot` for bridge sync.
- [x] Vue checkbox field dispatch now sends explicit `checked` payloads for Data/Execution/Settings panels + context docks (`data.setField`, `execution.setField`, `settings.setField`) instead of implicit runtime toggles, reducing hidden-state mirror coupling.
- [x] Hidden compatibility template trimmed by removing legacy workspace/workflow nav bars, workflow subtitle/advanced-toggle row, and workspace context strip from `legacy/template.html`; parity coverage now asserts workspace/workflow tab surfaces in `VueWorkspaceShell.vue` while keeping only required controls/results tab-bar checks in legacy template tests.
- [x] Legacy checkbox `*.setField` handlers (`data`, `validation`, `meta`, `settings`, `execution`) now require explicit `checked` payloads and no longer infer toggle intent from current hidden DOM state.
- [x] Legacy runtime now mirrors controls/results tab selections into root attributes (`data-controls-tab`, `data-results-tab`), and domain bridges prefer these runtime-owned attributes (with DOM active-button fallback), reducing strict dependency on hidden tab-bar class mutation state.
- [x] Runtime tab mutations now flow through fallback-aware helpers (`setControlsTabRuntime`, `setResultsTabRuntime`) so Vue workflow/actions can keep controls/results tab state synchronized even when legacy tab-bar markup is absent.
- [x] Legacy `controlsTabBar`/`resultsTabBar` markup removed from `legacy/template.html`; runtime tab ownership now runs headless via root tab attributes + helper fallbacks, and parity tests assert controls/results tab surfaces from `VueWorkspaceShell.vue`.
- [x] Domain bridges dropped hidden `controlsTabBar`/`resultsTabBar` mutation observers and now sync tab activation primarily from runtime-owned root attributes (`data-controls-tab`/`data-results-tab`), reducing hidden DOM churn.
- [x] Legacy tab-system bootstrap (`initTabSystem` for controls/results) decommissioned from `legacy/app.js`; controls/results tab state is now initialized and persisted through headless runtime helpers + workflow state, reducing `legacy-core`/`legacy-app` bundle weight.
- [x] Obsolete legacy tab-system module removed (`web/src/legacy/core/tabs.js`) after controls/results tab ownership moved to headless runtime state.
- [x] Legacy global shortcut registration (`initGlobalShortcuts`) removed from `legacy/app.js`; parity coverage now validates required shortcuts in `GlobalShortcutBridge.vue` (Vue-owned shortcut runtime).
- [x] Validation/meta dataset selection mirrors (`valCrossSelected`, `batchSelected`) moved from hidden compatibility inputs to runtime-owned arrays in `legacy/app.js`, with settings persistence restored via explicit `readSettingsFromUi` payload fields and backward-compatible load/apply parsing.
- [x] Settings workspace runtime actions now call typed handlers directly (`handleData*` / `handleHistoryOpenRunsDirAction` / `handleOpenReportsDirAction`) instead of hidden legacy button `.click()` proxy forwarding.
- [x] `data.loadSampleCsv` runtime action now resolves sample paths from a runtime cache (`SAMPLE_CSV_ENTRIES`) and loads directly via `loadCsv`, removing hidden sample-button `.click()` proxy dependency.
- [x] History preset import no longer depends on static hidden `#presetFile` markup in `legacy/template.html`; runtime now uses File System Access picker when available with dynamic input fallback for compatibility.
- [x] Runtime `setRunning` now mirrors run-state to root attribute (`data-running`) and settings bridge sync prefers runtime-owned root/main control state (`themeSelect`/`effectSelect`/`marketDataDir`/`marketDataMoveExisting`) with hidden-settings-button fallback, reducing strict dependency on hidden settings markup.
- [x] Legacy Settings-only compatibility panes (appearance/data/utilities/shortcut list) removed from `legacy/template.html`; only execution compatibility surface remains in that hidden section while Vue owns visible Settings UI.
- [x] Workflow/tab UI command ownership in Vue components now routes through `workflowStore` actions (`selectWorkspace`/`selectWorkflow`/`selectWorkflowStep`/`selectControlsTab`/`selectResultsTab`/`selectAggregateTab`) instead of direct `invokeLegacyAction('workflow.set*')` calls.
- [x] Added focused workflow-store action tests (`web/src/stores/workflowStore.test.ts`) to lock normalized tab/workspace dispatch semantics and legacy-runtime action bridging behavior.
- [x] Latest frontend build after workflow-store ownership rewiring: `legacy-app` chunk ~`522.42 kB` (gzip ~`137.44 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`170.95 kB` (gzip ~`45.98 kB`).
- [x] Hidden compatibility Quick Bar markup (`quickBar`, `btnQa*`, `quickBarHint`) removed from `legacy/template.html`, and corresponding quick-bar runtime wiring/readiness mirrors removed from `legacy/app.js`.
- [x] Hidden compatibility quick-actions report controls (`quickActions`, `btnOpenLatestReport`, `btnOpenLatestMetaReport`) removed from `legacy/template.html`; report-open actions remain available through typed Vue command surfaces (palette/shortcuts/stores).
- [x] Shared command execution paths are now store-owned via `commandActionStore` (typed action methods), and `CommandPaletteOverlay`/`GlobalShortcutBridge`/`RunFooterPanel` no longer call `invokeLegacyAction` directly.
- [x] Additional context-dock command controls (`DataContextDock`/`OptimizeContextDock`/`ValidationContextDock`/`AnalysisContextDock`) now dispatch through `commandActionStore`, further reducing direct legacy invoke usage in Vue panels.
- [x] `DataWorkspacePanel` command + field mutation dispatch moved into typed `dataStore` actions (`set*Field`, load/browse/timeframe/storage/sample actions); panel script no longer calls `invokeLegacyAction` directly.
- [x] `OptimizeWorkspacePanel` command + field mutation dispatch moved into typed `optimizeStore` actions (`set*Field`, preset/GPU utilities, run/validate); panel script no longer calls `invokeLegacyAction` directly.
- [x] Added command-action store tests (`web/src/stores/commandActionStore.test.ts`) to lock dispatch semantics for run/cancel/validate, shared export/report toggles, and drill navigation actions.
- [x] Added `dataStore` command-dispatch tests (`web/src/stores/dataStore.actions.test.ts`) covering typed field mutations + load/storage/sample command forwarding semantics.
- [x] Added `optimizeStore` command-dispatch tests (`web/src/stores/optimizeStore.actions.test.ts`) covering typed optimize field updates, preset/GPU utility actions, and run/validate dispatch.
- [x] Vue component direct `invokeLegacyAction(...)` callsites currently reduced to `58` (down from `63` before `data`/`optimize` workspace panel action-store migration slice).
- [x] Latest frontend build after data+optimize store action migration: `legacy-app` ~`521.18 kB` (gzip ~`137.07 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.29 kB` (gzip ~`45.71 kB`).
- [x] `ValidationWorkspacePanel` command + field mutation dispatch moved into typed `validationStore` actions (`set*Field`, export/report actions, cross-dataset controls, run-validation); panel script no longer calls `invokeLegacyAction` directly.
- [x] `HistoryWorkspacePanel` command + field mutation dispatch moved into typed `historyStore` actions (`set*Field`, session actions, saved-run actions, runs-directory actions); panel script no longer calls `invokeLegacyAction` directly.
- [x] `SettingsWorkspacePanel` + `SettingsContextDock` command + field mutation dispatch moved into typed `settingsStore` actions (`set*Field`, market-data dir actions, utilities/nav actions); both scripts no longer call `invokeLegacyAction` directly.
- [x] `ExecutionWorkspacePanel` + `ExecutionContextDock` command + field mutation dispatch moved into typed `executionStore` actions (`set*Field`, execution run/control actions); both scripts no longer call `invokeLegacyAction` directly.
- [x] Added `validationStore` and `historyStore` command-dispatch tests (`web/src/stores/validationStore.actions.test.ts`, `web/src/stores/historyStore.actions.test.ts`) to lock action payload normalization and command routing behavior.
- [x] Added `settingsStore` command-dispatch tests (`web/src/stores/settingsStore.actions.test.ts`) covering typed field updates plus market-data/utilities/navigation action routing.
- [x] Added `executionStore` command-dispatch tests (`web/src/stores/executionStore.actions.test.ts`) covering execution field updates and execution run/control action routing.
- [x] Vue component direct `invokeLegacyAction(...)` callsites currently reduced to `43` (down from `58` before `validation`/`history` workspace panel action-store migration slice).
- [x] Latest frontend build after validation+history store action migration: `legacy-app` ~`522.98 kB` (gzip ~`137.46 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.29 kB` (gzip ~`45.71 kB`).
- [x] Vue component direct `invokeLegacyAction(...)` callsites currently reduced to `34` after settings-panel/dock action-store migration.
- [x] Latest frontend build after settings store action migration: `legacy-app` ~`523.79 kB` (gzip ~`137.57 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.29 kB` (gzip ~`45.71 kB`).
- [x] Vue component direct `invokeLegacyAction(...)` callsites currently reduced to `26` after execution-panel/dock action-store migration.
- [x] Latest frontend build after execution store action migration: `legacy-app` ~`524.36 kB` (gzip ~`137.60 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.29 kB` (gzip ~`45.71 kB`).
- [x] Analysis surface command + field mutation dispatch moved into typed `analysisStore` actions (`set*Field`, top-table controls, overview actions, heatmap export, drill rerender, meta/batch/ML/stage4 action groups); `TopWorkspacePanel`/`OverviewWorkspacePanel`/`HeatmapWorkspacePanel`/`DrillWorkspacePanel`/`ParetoWorkspacePanel`/`MetaWorkspacePanel` no longer call `invokeLegacyAction` directly.
- [x] Strategy/Advanced/Distributed surfaces now dispatch through typed stores (`strategyStore`, `advancedStore`, `distributedStore`), and `StrategyWorkspacePanel`/`AdvancedWorkspacePanel`/`DistributedWorkspacePanel`/`DistributedContextDock` no longer call `invokeLegacyAction` directly.
- [x] `HistoryContextDock` command dispatch moved into typed `historyStore` actions; context dock no longer calls `invokeLegacyAction` directly.
- [x] Added store action tests for newly migrated domains: `analysisStore.actions.test.ts` (expanded), `advancedStore.actions.test.ts`, `distributedStore.actions.test.ts`, `strategyStore.actions.test.ts`.
- [x] Added component action-ownership parity test (`web/src/legacy/componentActionOwnership.test.ts`) to enforce no direct `invokeLegacyAction(...)` calls inside Vue component files.
- [x] Vue component direct `invokeLegacyAction(...)` callsites currently reduced to `0` in `web/src/components` after analysis/meta/strategy/advanced/distributed/history context migration.
- [x] Latest frontend build after full component action-store migration: `legacy-app` ~`527.55 kB` (gzip ~`138.14 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.29 kB` (gzip ~`45.71 kB`).
- [x] Legacy template decommission pass removed unreferenced compatibility IDs (`legacyCompatibilityRoot`, `savedRunDetailCard`) from `web/src/legacy/template.html`; parity test suite remains green.
- [x] Latest frontend build after template ID cleanup: `legacy-app` ~`527.55 kB` (gzip ~`138.14 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.24 kB` (gzip ~`45.68 kB`).
- [x] Workflow/workspace bootstrap ownership advanced into Vue service: added `web/src/services/workflowBootstrap.ts` and applied persisted workflow/workspace/step/controls/results state via `LegacyRuntimeBridge` after legacy bridge attach.
- [x] Legacy workflow/workspace init now skips localStorage restore when Vue is workflow owner (`initWorkflowNav`/`initWorkspaceNav` called with `restoreFromStorage: false` under `window.__VTA_VUE_WORKFLOW_OWNER__`), reducing `legacy/app.js` startup ownership while preserving non-Vue fallback behavior.
- [x] Added workflow-bootstrap service tests (`web/src/services/workflowBootstrap.test.ts`) covering key normalization and action dispatch behavior.
- [x] Latest frontend test status after workflow-bootstrap migration: `26` files / `80` tests passing.
- [x] Latest frontend build after workflow-bootstrap migration: `legacy-app` ~`527.66 kB` (gzip ~`138.21 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.24 kB` (gzip ~`45.68 kB`).
- [x] Legacy workflow/workspace nav-bar DOM wiring decommissioned from `web/src/legacy/app.js` (`WORKFLOW_UI`/`WORKSPACE_NAV_UI`/`WORKFLOW_STEP_NAV_UI` maps and associated button listener/focus logic removed); workflow runtime now runs headless for these paths under Vue ownership.
- [x] `initWorkflowNav`/`initWorkspaceNav` simplified to headless state bootstrap + runtime synchronization (no `workflowTabBar`/`workspaceBar`/`workflowStepBar` listener setup), reducing legacy coupling to removed template markup.
- [x] Latest frontend build after headless workflow-nav decommission: `legacy-app` ~`524.61 kB` (gzip ~`137.56 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.24 kB` (gzip ~`45.68 kB`).
- [x] Legacy aggregate-tab bar DOM wiring decommissioned from `web/src/legacy/app.js` (`setAggregateTab` no longer mutates bar button classes and `initAggregateTabUi` no longer binds aggregate bar listeners), with aggregate runtime state kept headless via `CURRENT_AGGREGATE_TAB` + root dataset.
- [x] Analysis bridge aggregate-tab sync now reads root `data-aggregate-tab` state (with dataset mutation observation) instead of relying on legacy aggregate button `.active` classes.
- [x] Latest frontend build after aggregate headless decommission + bridge sync update: `legacy-app` ~`523.70 kB` (gzip ~`137.23 kB`), `legacy-core` ~`23.30 kB` (gzip ~`8.03 kB`), `legacy-template` ~`169.24 kB` (gzip ~`45.68 kB`).
- [x] Legacy controls/results tab-system compatibility scaffolding removed from `web/src/legacy/app.js` (dropped `CONTROLS_TABS`/`RESULTS_TABS` branches and `wrapTabSet` path); tab ownership now stays in runtime root datasets + typed workflow actions.
- [x] Added explicit controls/results tab normalizers in `web/src/legacy/core/workflowModel.js` (`normalizeControlsTab`, `normalizeResultsTab`) and wired workflow action handlers to these guards.
- [x] Added Vue-owned settings bootstrap service (`web/src/services/settingsBootstrap.ts`) to restore persisted `theme` / `window effect` / `workflow advanced` / `controls collapsed` through typed `settings.setField` actions at startup.
- [x] Legacy theme/effect/workflow-advanced bootstrap in `web/src/legacy/app.js` now supports `restoreFromStorage` gating and skips localStorage restore under Vue workflow ownership; `LegacyRuntimeBridge` now applies settings bootstrap service after workflow bootstrap.
- [x] Added settings-bootstrap service tests (`web/src/services/settingsBootstrap.test.ts`) covering storage normalization + typed action dispatch semantics.
- [x] Latest frontend test status after tab/runtime + settings bootstrap migration: `27` files / `84` tests passing.
- [x] Latest frontend build after tab/runtime + settings bootstrap migration: `legacy-app` ~`522.95 kB` (gzip ~`137.03 kB`), `legacy-core` ~`23.65 kB` (gzip ~`8.08 kB`), `legacy-template` ~`169.24 kB` (gzip ~`45.68 kB`).
- [x] Hidden compatibility header/splitter controls decommissioned from `web/src/legacy/template.html` (`btnToggleControls`, `themeSelect`, `effectSelect`, `mainSplitter` removed); parity tests now assert those IDs remain absent.
- [x] `initThemeUi` / `initWindowEffectUi` in `web/src/legacy/app.js` now apply theme/effect defaults even when legacy select controls are absent, preserving runtime bootstrap behavior after template cleanup.
- [x] Latest frontend build after compatibility header/splitter removal: `legacy-app` ~`522.94 kB` (gzip ~`137.04 kB`), `legacy-core` ~`23.65 kB` (gzip ~`8.08 kB`), `legacy-template` ~`168.38 kB` (gzip ~`45.35 kB`).
- [x] Workflow bootstrap service now restores persisted aggregate-tab state (`vta_aggregate_tab_v1`) with workspace-aware dispatch: active aggregate workspace uses `workflow.setAggregateTab`; non-aggregate restore uses a non-navigating prime action.
- [x] Added runtime action `workflow.primeAggregateTab` in `web/src/legacy/app.js` (plus typed action-union update in `web/src/legacy/actions.ts`) to preserve aggregate-tab persistence without forcing workspace navigation during startup bootstrap.
- [x] Expanded workflow-bootstrap coverage (`web/src/services/workflowBootstrap.test.ts`) to validate aggregate-tab normalization and dispatch behavior; latest frontend tests: `27` files / `85` tests passing.
- [x] Latest frontend build after aggregate bootstrap ownership update: `legacy-app` ~`523.05 kB` (gzip ~`137.06 kB`), `legacy-core` ~`23.65 kB` (gzip ~`8.08 kB`), `legacy-template` ~`168.38 kB` (gzip ~`45.35 kB`).
- [x] Legacy settings mirror scaffolding in `web/src/legacy/app.js` decommissioned: `syncSettingsWorkspaceUi` reduced to no-op compatibility shim and `initSettingsWorkspaceUi` trimmed to minimal host guard.
- [x] Removed stale legacy settings-mirror field dependencies from settings actions (`handleSettingsSetMarketDataDirAction`, `handleSettingsSetFieldAction`), so runtime now reads/writes only active compatibility fields (`marketDataDir`, `marketDataMoveExisting`) for Settings storage actions.
- [x] Latest frontend build after settings-mirror decommission: `legacy-app` ~`519.31 kB` (gzip ~`136.38 kB`), `legacy-core` ~`23.65 kB` (gzip ~`8.09 kB`), `legacy-template` ~`168.38 kB` (gzip ~`45.35 kB`).
- [x] Legacy controls-splitter runtime decommissioned from `web/src/legacy/app.js`: removed width/splitter/button plumbing (`CONTROLS_WIDTH_KEY`, min/max bounds, pointer/keyboard resize handlers, `initControlsSplitter`) after corresponding compatibility markup removal; `ui.toggleControls` now toggles collapsed state directly.
- [x] Latest frontend build after controls-splitter decommission: `legacy-app` ~`517.31 kB` (gzip ~`135.74 kB`), `legacy-core` ~`23.61 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.38 kB` (gzip ~`45.35 kB`).
- [x] Removed obsolete settings-bridge stubs from legacy runtime (`syncSettingsWorkspaceUi`, `initSettingsWorkspaceUi`) and dropped remaining no-op callsites; data-field handlers no longer branch on decommissioned settings-mirror IDs.
- [x] Removed obsolete compatibility `settingsWorkspaceCard` anchor from `web/src/legacy/template.html` and pruned corresponding CSS visibility rules from `web/src/styles/app.css`.
- [x] Latest frontend build after settings-anchor decommission: `legacy-app` ~`517.27 kB` (gzip ~`135.73 kB`), `legacy-core` ~`23.61 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.33 kB` (gzip ~`45.33 kB`).
- [x] Removed dead splitter CSS rules from `web/src/styles/app.css` (`.splitter`, `.splitterGrip`, and all `#mainSplitter` selectors) after splitter markup/runtime decommission.
- [x] Latest frontend CSS bundle after splitter-style cleanup: `assets/index-*.css` ~`38.01 kB` (gzip ~`8.06 kB`), down from ~`38.85 kB` (gzip ~`8.21 kB`) before this pass.
- [x] `Ctrl+\` / command-palette “Toggle controls” path is now Vue-owned in `commandActionStore` (`settings.setField` dispatch) rather than legacy `ui.toggleControls`; removed `ui.toggleControls` from `legacy/actions.ts` and `legacy/app.js` action registry.
- [x] Latest frontend build after `ui.toggleControls` action decommission: `legacy-app` ~`517.06 kB` (gzip ~`135.74 kB`), `legacy-core` ~`23.61 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.33 kB` (gzip ~`45.33 kB`).
- [x] Removed legacy command-palette action bridge path (`ui.openCommandPalette`) from `commandActionStore`, `legacy/actions.ts`, and `legacy/app.js`; Vue palette now opens/closes directly from `commandPaletteStore` only.
- [x] Removed obsolete legacy command-palette compatibility globals/events wiring (`__VTA_VUE_CMD_PALETTE_*`, `vta:command-palette-open/close`) from `CommandPaletteOverlay.vue` and `web/src/tauri/global.d.ts`.
- [x] Latest frontend build after command-palette bridge decommission: `legacy-app` ~`516.70 kB` (gzip ~`135.61 kB`), `legacy-core` ~`23.61 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.33 kB` (gzip ~`45.33 kB`).
- [x] Removed recurring Teleport host polling (`setInterval(resolveMount, 500)`) from all Vue workspace/footer panels; mount resolution is now single-shot after `nextTick`, reducing background timer churn while preserving hidden-compat host behavior.
- [x] Settings/workflow bootstrap services now route through Pinia store actions (`useSettingsStore`, `useWorkflowStore`) instead of direct `invokeLegacyAction(...)` calls in service layer.
- [x] `settingsStore` now applies immediate Vue-native state updates for `settings.setField` mutations (theme/effect/workflow-advanced/controls-collapsed/market-data-dir/move-existing), including root dataset + localStorage synchronization and native window-effect apply requests, while retaining legacy runtime dispatch for parity.
- [x] `advancedStore` and `strategyStore` now apply immediate Vue-native state updates for typed field setters before compatibility dispatch, reducing UI feedback latency and tightening Vue runtime ownership.
- [x] Added `workflowStore.primeAggregateTab(...)` so aggregate bootstrap priming remains workflow-store owned even when aggregate workspace is not active.
- [x] Remaining non-legacy runtime `invokeLegacyAction(...)` callsites reduced from `14` files to `12` files by removing service-layer dispatch dependencies.
- [x] Latest frontend validation after this migration slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `85` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after Teleport/store/bootstrap migration slice: `legacy-app` ~`523.57 kB` (gzip ~`137.52 kB`), `legacy-core` ~`23.61 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.33 kB` (gzip ~`45.33 kB`).
- [x] `workflowStore` now owns normalized tab runtime state (`controlsTab`, `resultsTab`, `aggregateTab`) in addition to workspace/workflow/workflow-step, with typed workflow actions updating these fields before compatibility dispatch.
- [x] Workflow bridge snapshot payloads now include controls/results/aggregate tab attributes from root runtime state, so Vue tab ownership remains synchronized even when legacy-originated transitions occur.
- [x] `App.vue` lazy-panel activation and `VueWorkspaceShell.vue` tab rendering now source active controls/results tabs from `workflowStore` rather than domain-bridge `activeControlsTab` / `activeResultsTab` fan-in.
- [x] Workspace panel visibility gates for Data/Optimize/Strategy/Advanced/Distributed and Overview/Top/Pareto/Validate/Drill/Heatmap/Meta/Debug now consume `workflowStore` controls/results tabs, reducing UI visibility coupling to per-domain legacy snapshot timing.
- [x] Latest frontend validation after workflow-tab ownership migration: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `85` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after workflow-tab ownership migration: `legacy-app` ~`524.31 kB` (gzip ~`137.70 kB`), `legacy-core` ~`23.61 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.33 kB` (gzip ~`45.33 kB`).
- [x] Workflow runtime ownership migrated into `workflowStore`: `selectWorkspace`/`selectWorkflow`/`selectWorkflowStep`/`selectControlsTab`/`selectResultsTab`/`selectAggregateTab` now run Vue-native workspace/workflow/tab state-machine transitions (including localStorage + root `data-*` sync) without dispatching `workflow.set*` legacy runtime actions.
- [x] Vue startup now initializes workflow runtime attrs from store state (`workflowStore.syncRuntimeState()` in `App.vue`), so compatibility bridges consume Vue-owned root tab/workspace attributes from first render.
- [x] Workflow bridge now runs in Vue-owner mode: incoming `vta:workflow-snapshot` events only hydrate tab/subtitle fields (`controlsTab`/`resultsTab`/`aggregateTab`/`workflowSubtitle`) and no longer overwrite Vue-owned workspace/workflow/workflow-step state.
- [x] Settings navigation actions are now Vue-native: `settingsStore.gotoLibrary()` / `settingsStore.gotoAggregate()` route through `workflowStore.selectWorkspace(...)` instead of `settings.goto*` legacy runtime actions.
- [x] Remaining context-dock tab display/visibility dependencies moved to workflow store (`AnalysisContextDock` and `DistributedContextDock` now read `workflowStore.resultsTab` / `workflowStore.controlsTab`).
- [x] Remaining non-legacy runtime `invokeLegacyAction(...)` callsites reduced from `12` files to `11` files after removing workflow-store and settings-navigation legacy dispatch dependencies.
- [x] Added/updated Vue ownership regression tests for this slice: `workflowStore.test.ts`, `workflowBootstrap.test.ts`, and `settingsStore.actions.test.ts`.
- [x] Latest frontend validation after Vue workflow state-machine migration: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Legacy workflow runtime action handlers removed from `web/src/legacy/app.js` and typed `LegacyActionName` unions (`workflow.setWorkspace`, `workflow.setWorkflow`, `workflow.setWorkflowStep`, `workflow.setControlsTab`, `workflow.setResultsTab`, `workflow.setAggregateTab`, `workflow.primeAggregateTab`) after Vue workflow-store ownership cutover.
- [x] Legacy settings navigation runtime actions removed from `web/src/legacy/app.js` and typed unions (`settings.gotoLibrary`, `settings.gotoAggregate`) since settings navigation is now store-owned (`workflowStore.selectWorkspace(...)`).
- [x] Workflow bridge subtitle fallback now prefers Vue-owned root runtime state (`data-workflow-subtitle`) before hidden legacy DOM text, further decoupling workflow sync from compatibility markup.
- [x] Latest frontend build after workflow-action decommission slice: `legacy-app` ~`528.51 kB` (gzip ~`138.82 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`168.33 kB` (gzip ~`45.33 kB`).
- [x] Hidden compatibility aggregate tab-bar markup removed from `web/src/legacy/template.html` (`aggregateTabBar`), leaving aggregate tab state fully root-dataset/workflow-store driven.
- [x] Legacy context bridges dropped fallback queries to removed hidden tab bars (`controlsTabBar`/`resultsTabBar`/`aggregateTabBar`) and now read tab state directly from root runtime attributes.
- [x] `MetaWorkspacePanel` aggregate-tab surface now reads `workflowStore.aggregateTab` as its source of truth instead of analysis bridge mirror state, tightening Vue runtime ownership.
- [x] Removed obsolete aggregate-tab compatibility CSS rule (`.aggregateTabBar`) from `web/src/styles/app.css`.
- [x] Latest frontend validation after aggregate-compat decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after aggregate-compat decommission slice: `legacy-app` ~`527.45 kB` (gzip ~`138.74 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.85 kB` (gzip ~`45.23 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] Settings/report wrapper action ownership shifted out of legacy runtime: removed typed/runtime action paths for `settings.browseMarketDataDir`, `settings.setMarketDataDir`, `settings.resetMarketDataDir`, `settings.openMarketDataDir`, `settings.openWorkflowData`, `settings.openRunsDir`, `settings.openReportsDir`, `settings.openPolygonModal`, `reports.openDir`, and `history.openRunsDir`; Vue stores now route through domain actions (`data.*`), workflow store (`selectWorkspace('workflow','data')`), and direct Tauri commands (`open_runs_dir`, `open_reports_dir`) as appropriate.
- [x] Removed obsolete settings-wrapper bridge handlers plus legacy runtime action-registry entries for settings/report/runs-directory wrappers from `web/src/legacy/app.js`, reducing legacy runtime surface and shrinking `legacy/app.js` line count (`19174` -> `19111`).
- [x] Latest frontend validation after settings/report wrapper decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after settings/report/runs-dir wrapper decommission slice: `legacy-app` ~`526.42 kB` (gzip ~`138.43 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.85 kB` (gzip ~`45.23 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] Report-open command ownership moved to Vue store runtime path handling: `commandActionStore.openLatestReport()` / `openLatestMetaReport()` now use root runtime dataset paths with direct Tauri commands (`open_path` fallback `reveal_path`) instead of legacy runtime actions.
- [x] Legacy runtime now mirrors latest run/meta report paths into root attributes (`data-latest-report-path`, `data-latest-meta-report-path`) from `updateQuickActionsUi()`, so Vue command surfaces can open latest artifacts without `reports.openLatest*` action dispatch.
- [x] Removed legacy report-open runtime action names from typed unions and `LEGACY_ACTION_REGISTRY` (`reports.openLatest`, `reports.openLatestMeta`) plus obsolete init listener wiring.
- [x] Latest frontend validation after report-open ownership migration: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after report-open ownership migration: `legacy-app` ~`525.79 kB` (gzip ~`138.38 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.85 kB` (gzip ~`45.23 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced further from `19111` to `19072` after report-open runtime action decommission.
- [x] Removed obsolete hidden-button click listener ownership for runs/reports directory wrappers from legacy bootstrap (`btnOpenRunsDir`, `btnOpenReportsDir`) and deleted dead helper handlers (`handleHistoryOpenRunsDirAction`, `handleOpenReportsDirAction`) after Vue stores moved to direct Tauri commands.
- [x] Latest frontend validation after hidden runs/reports listener decommission: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after hidden runs/reports listener decommission: `legacy-app` ~`525.29 kB` (gzip ~`138.29 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.85 kB` (gzip ~`45.23 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `19072` to `19044` after removing dead runs/reports directory listener/handler code.
- [x] Removed final legacy quick-action UI remnants for report-open paths from runtime (`btnOpenLatestReport`, `btnOpenLatestMetaReport`, `quickActionsMeta` branches), keeping `updateQuickActionsUi()` focused on root runtime dataset synchronization only.
- [x] Latest frontend validation after quick-action remnant cleanup: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after quick-action remnant cleanup: `legacy-app` ~`524.91 kB` (gzip ~`138.17 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.85 kB` (gzip ~`45.23 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `19044` to `19020` after quick-action report UI branch cleanup.
- [x] `commandActionStore` runtime dispatch ownership moved to domain stores (`dataStore`, `optimizeStore`, `analysisStore`, `validationStore`) plus direct path-open commands; removed direct `invokeLegacyAction(...)` dependency from `commandActionStore`.
- [x] Added missing domain action coverage to support command-store delegation (`optimizeStore.cancelOptimization`, `analysisStore.drillClearTradeSelection` / `drillPrevTrade` / `drillNextTrade`) and updated command-action tests for store-delegated dispatch semantics.
- [x] Remaining non-legacy runtime `invokeLegacyAction(...)` callsites reduced from `11` store files to `10` store files after command-store delegation cleanup.
- [x] Latest frontend validation after command-store delegation migration: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after command-store delegation migration: `legacy-app` ~`525.10 kB` (gzip ~`137.94 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.85 kB` (gzip ~`45.23 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] Hidden compatibility directory-open buttons removed from `legacy/template.html` (`btnOpenReportsDir`, `btnOpenRunsDir`) now that Vue stores own those actions via direct Tauri commands.
- [x] Validation/history bridges now derive `canOpenReportsDir`/`canOpenRunsDir` from runtime-owned root run-state (`data-running`) instead of hidden button disabled-state, and observe root running-attribute changes for sync.
- [x] Removed dead runtime dependencies on removed directory-open button IDs from `legacy/app.js` (`RUNNING_DISABLE_IDS` entries and stale saved-run selection disable branch).
- [x] Latest frontend validation after directory-button template/bridge cleanup: `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run test` (`27` files / `86` tests), and `npm --prefix web run build` all passing.
- [x] Latest frontend build after directory-button template/bridge cleanup: `legacy-app` ~`525.25 kB` (gzip ~`138.11 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.67 kB` (gzip ~`45.18 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `19020` to `19016` after directory-button runtime cleanup.
- [x] Settings field mutation ownership moved fully to Vue store runtime: `settingsStore.setInputField` / `setSelectField` / `setCheckboxField` now apply Vue-local state/root-dataset + compatibility field sync directly, with no `settings.setField` legacy action dispatch.
- [x] Settings utility delegation now routes through domain store actions (`useDataStore`) for market-data directory and Polygon modal flows, removing direct `invokeLegacyAction(...)` dependency from `settingsStore`.
- [x] Removed `settings.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and from legacy runtime action registry/handler surface in `web/src/legacy/app.js` (`handleSettingsSetFieldAction` + registry entry removed).
- [x] Updated settings/command/bootstrap tests for Vue-owned settings runtime semantics (`settingsStore.actions.test.ts`, `commandActionStore.test.ts`, `settingsBootstrap.test.ts`) with explicit non-DOM test harness stubs.
- [x] Remaining non-legacy runtime `invokeLegacyAction(...)` callsites reduced from `10` store files to `9` store files after settings-store decommission.
- [x] Latest frontend validation after settings-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after settings-action decommission slice: `legacy-app` ~`524.27 kB` (gzip ~`137.97 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.67 kB` (gzip ~`45.18 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `19016` to `17098` after removing legacy settings-field action handler/registry ownership in this slice.
- [x] Advanced export-path browse flow ownership moved to Vue store command path: `advancedStore.browseExportAll()` now uses direct Tauri `pick_save_csv` command and then updates runtime through `advanced.setField('exportAllCsvPath', ...)` compatibility dispatch.
- [x] Removed `advanced.browseExportAll` from typed legacy action unions (`web/src/legacy/actions.ts`) and from legacy runtime button/action ownership in `web/src/legacy/app.js` (deleted `handleAdvancedBrowseExportAllAction`, stale button listener wiring, and registry entry).
- [x] Updated advanced action tests for command-driven browse semantics (`advancedStore.actions.test.ts`) with command mock coverage for `pick_save_csv`.
- [x] Latest frontend validation after advanced browse-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after advanced browse-action decommission slice: `legacy-app` ~`523.97 kB` (gzip ~`137.94 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.67 kB` (gzip ~`45.18 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `17098` to `17079` after advanced browse-action/runtime cleanup.
- [x] Removed obsolete hidden compatibility export-browse button markup from `web/src/legacy/template.html` (`btnBrowseExportAll`) and added UI parity guard assertion in `web/src/legacy/uiSurfaceParity.test.ts`.
- [x] Advanced compatibility bridge now derives `canBrowseExportAll` from runtime-owned export-path input state (`exportAllCsvPath` disabled state) and root running attribute, with dedicated attribute observers; removed stale dependency on removed button ID in `web/src/legacy/bridges/advancedContextBridge.ts`.
- [x] Removed dead runtime references to removed export-browse button in `web/src/legacy/app.js` (`RUNNING_DISABLE_IDS` entry and `updateStrategyUi` button disable branch).
- [x] Latest frontend validation after advanced export-button template/bridge decommission: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after advanced export-button template/bridge decommission: `legacy-app` ~`523.95 kB` (gzip ~`137.93 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.62 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.94 kB` (gzip ~`8.05 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `17079` to `17076` after export-button runtime cleanup.
- [x] Removed additional unused compatibility ID markup from `web/src/legacy/template.html` (`controlsCard`, `resultsCard`, `aggregateSummaryGrid`) while keeping hidden compatibility DOM structure intact.
- [x] Pruned dead CSS selectors tied to removed compatibility IDs in `web/src/styles/app.css` (`#controlsCard`/`#resultsCard` visibility + footer/muted/mono rules), reducing legacy style coupling.
- [x] Updated static UI parity guard to assert removed compatibility IDs remain absent (`web/src/legacy/uiSurfaceParity.test.ts`).
- [x] Latest frontend validation after compatibility-ID/CSS decommission: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after compatibility-ID/CSS decommission: `legacy-app` ~`523.95 kB` (gzip ~`137.93 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Strategy field mutation ownership moved to Vue runtime-compatible DOM events in `strategyStore`: `setInputField` now dispatches native `input` events and `setSelectField` dispatches native `change` events against compatibility controls instead of `invokeLegacyAction('strategy.setField', ...)`.
- [x] Removed `strategy.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`STRATEGY_TEXT_FIELDS`, `handleStrategySetFieldAction`, and registry entry removed).
- [x] Updated strategy store action test coverage for DOM-event dispatch semantics (`strategyStore.actions.test.ts`) using explicit document harness stubs.
- [x] Remaining non-legacy runtime `invokeLegacyAction(...)` callsites reduced from `9` store files to `8` store files after strategy-store decommission.
- [x] Latest frontend validation after strategy-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after strategy-action decommission slice: `legacy-app` ~`523.07 kB` (gzip ~`137.91 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `17076` to `17024` after removing legacy strategy-field action ownership.
- [x] Distributed field mutation ownership moved to Vue runtime-compatible DOM events in `distributedStore`: `setInputField` now dispatches native `input` events and `setSelectField` dispatches native `change` events against compatibility controls instead of `invokeLegacyAction('distributed.setField', ...)`.
- [x] Removed `distributed.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`DISTRIBUTED_TEXT_FIELDS`, `handleDistributedSetFieldAction`, and registry entry removed).
- [x] Updated distributed store action test coverage for DOM-event dispatch semantics (`distributedStore.actions.test.ts`) using explicit document harness stubs while preserving legacy-action coverage for distributed run controls.
- [x] Latest frontend validation after distributed field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after distributed field-action decommission slice: `legacy-app` ~`522.18 kB` (gzip ~`137.79 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18834` to `18803` after removing distributed field-action runtime ownership in this slice.
- [x] Execution field mutation ownership moved to Vue runtime-compatible DOM events in `executionStore`: text/select setters now dispatch native `input`/`change` events and checkbox setter dispatches native `change` against compatibility controls instead of `invokeLegacyAction('execution.setField', ...)`.
- [x] Removed `execution.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`EXECUTION_TEXT_FIELDS`, `EXECUTION_CHECK_FIELDS`, `handleExecutionSetFieldAction`, and registry entry removed).
- [x] Updated execution store action test coverage for DOM-event dispatch semantics (`executionStore.actions.test.ts`) using explicit document harness stubs while preserving legacy-action coverage for execution control/run actions.
- [x] Latest frontend validation after execution field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after execution field-action decommission slice: `legacy-app` ~`521.51 kB` (gzip ~`137.79 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18803` to `18745` after removing execution field-action runtime ownership in this slice.
- [x] Data field mutation ownership moved to Vue runtime-compatible DOM events in `dataStore`: text/select setters now dispatch native `input`/`change` events and checkbox setter dispatches native `change` against compatibility controls instead of `invokeLegacyAction('data.setField', ...)`.
- [x] Removed `data.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`handleDataSetFieldAction` + registry entry removed, with dead `DATA_*_FIELDS` sets removed).
- [x] Added direct runtime listeners for `dataTimeframe` (`updateDataTimeframeUi`) and `marketDataDir`/`marketDataMoveExisting` (`saveSettingsSoon`) so existing side effects remain intact after data field-action decommission.
- [x] Updated data store action test coverage for DOM-event dispatch semantics (`dataStore.actions.test.ts`) using explicit document harness stubs while preserving legacy-action coverage for load/browse/sample flows.
- [x] Latest frontend validation after data field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after data field-action decommission slice: `legacy-app` ~`521.37 kB` (gzip ~`137.82 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18745` to `18711` after removing data field-action runtime ownership in this slice.
- [x] History field mutation ownership moved to Vue runtime-compatible DOM events in `historyStore`: `setInputField` now dispatches native `input` events and `setSelectField` dispatches native `change` events against compatibility controls instead of `invokeLegacyAction('history.setField', ...)`.
- [x] Removed `history.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`HISTORY_SELECT_FIELDS`, `HISTORY_INPUT_FIELDS`, `handleHistorySetFieldAction`, and registry entry removed).
- [x] Added explicit history selection/filter persistence listeners in `web/src/legacy/app.js` for `historySelect`, `savedRunsSelect`, `savedRunsSearch`, and `savedRunsTags` (`saveSettingsSoon`) to preserve settings-save behavior after history field-action decommission.
- [x] Updated history store action test coverage for DOM-event dispatch semantics (`historyStore.actions.test.ts`) using explicit document harness stubs while preserving legacy-action coverage for session/saved-run command actions.
- [x] Latest frontend validation after history field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after history field-action decommission slice: `legacy-app` ~`520.75 kB` (gzip ~`137.61 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18711` to `18656` after removing history field-action runtime ownership in this slice.
- [x] Optimize field mutation ownership moved to Vue runtime-compatible DOM events in `optimizeStore`: `setInputField` now dispatches native `input` events and `setSelectField` dispatches native `change` events against compatibility controls instead of `invokeLegacyAction('optimize.setField', ...)`.
- [x] Removed `optimize.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`OPTIMIZE_TEXT_FIELDS`, `handleOptimizeSetFieldAction`, and registry entry removed).
- [x] Updated optimize store action test coverage for DOM-event dispatch semantics (`optimizeStore.actions.test.ts`) using explicit document harness stubs while preserving legacy-action coverage for run/preset/GPU actions.
- [x] Latest frontend validation after optimize field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after optimize field-action decommission slice: `legacy-app` ~`519.83 kB` (gzip ~`137.44 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18656` to `18576` after removing optimize field-action runtime ownership in this slice.
- [x] Validation field mutation ownership moved to Vue runtime-compatible DOM events in `validationStore`: `setInputField` now dispatches native `input` events and checkbox setter dispatches native `change` events against compatibility controls instead of `invokeLegacyAction('validation.setField', ...)`.
- [x] Removed `validation.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned the legacy runtime handler/registry path in `web/src/legacy/app.js` (`VALIDATION_TEXT_FIELDS`, `VALIDATION_CHECK_FIELDS`, `handleValidationSetFieldAction`, and registry entry removed).
- [x] Updated validation store action test coverage for DOM-event dispatch semantics (`validationStore.actions.test.ts`) using explicit document harness stubs while preserving legacy-action coverage for validation exports/cross-dataset actions.
- [x] Latest frontend validation after validation field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `86` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after validation field-action decommission slice: `legacy-app` ~`518.86 kB` (gzip ~`137.32 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18576` to `18489` after removing validation field-action runtime ownership in this slice.
- [x] Analysis/meta field mutation ownership moved to Vue runtime-compatible DOM events in `analysisStore`: analysis/meta text setters now dispatch native `input`/`change`, and meta checkbox setter dispatches native `change`, replacing `invokeLegacyAction('analysis.setField'/'meta.setField', ...)`.
- [x] Removed `analysis.setField` and `meta.setField` from typed legacy action unions (`web/src/legacy/actions.ts`) and decommissioned their legacy runtime handler/registry paths in `web/src/legacy/app.js` (`ANALYSIS_TEXT_FIELDS`, `META_TEXT_FIELDS`, `META_CHECK_FIELDS`, `handleAnalysisSetFieldAction`, `handleMetaSetFieldAction`, and both registry entries removed).
- [x] Updated analysis store action tests for DOM-event dispatch semantics (`analysisStore.actions.test.ts`) with explicit document harness stubs; total frontend tests increased from `86` to `88`.
- [x] Remaining typed legacy `*.setField` action entries in `web/src/legacy/actions.ts` are now `0` (fully decommissioned).
- [x] Latest frontend validation after analysis/meta field-action decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`27` files / `88` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after analysis/meta field-action decommission slice: `legacy-app` ~`515.52 kB` (gzip ~`136.58 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18489` to `18332` after removing analysis/meta field-action runtime ownership in this slice.
- [x] Added Vue workflow runtime dispatch service (`web/src/services/workflowRuntimeDispatch.ts`) and global callback (`window.__VTA_VUE_WORKFLOW_DISPATCH__`) so legacy runtime workflow/tab mutations can route through Pinia-owned workflow actions.
- [x] `App.vue` now installs/uninstalls the workflow runtime dispatcher; Vue workflow state remains the authoritative runtime source while legacy compatibility runtime can request typed tab/workflow transitions.
- [x] `workflowStore` selection actions now accept optional `{ persist?: boolean }` commit options so legacy `save: false` callsites keep non-persisting behavior parity while still updating runtime state.
- [x] Legacy workflow runtime setters (`setWorkspace`, `setControlsTabRuntime`, `setResultsTabRuntime`, `setAggregateTab`) now delegate to Vue-owned workflow dispatch when Vue ownership is enabled, with legacy fallback preserved for non-Vue ownership mode.
- [x] Added Vue-owned workflow dataset observer in `web/src/legacy/app.js` to keep legacy compatibility locals (`CURRENT_WORKFLOW*`, workspace/aggregate/subtitle state) synchronized from root dataset attributes without reclaiming ownership.
- [x] `initLegacyApp()` workflow bootstrap gating tightened for Vue-owner mode: workflow/workspace restore initializers now skip legacy state-machine ownership while still initializing shared theme/effect/ui wiring.
- [x] Added workflow runtime dispatch tests (`web/src/services/workflowRuntimeDispatch.test.ts`) covering dispatcher registration, store action routing, aggregate tab routing, and non-persisting `save=false` parity semantics.
- [x] Latest frontend validation after workflow runtime-dispatch ownership slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Latest frontend build after workflow runtime-dispatch ownership slice: `legacy-app` ~`516.79 kB` (gzip ~`136.98 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after workflow runtime-dispatch ownership slice: `16612`.
- [x] Removed stale runtime coupling to legacy-only removed compatibility IDs in `web/src/legacy/app.js`: dead `themeSelect`/`effectSelect` synchronization, dead `btnToggleWorkflowAdvanced` button behavior, dead `workflowSubtitle` element writes, and dead `workspaceCtx*` context-strip rendering logic.
- [x] `updateWorkspaceContextStrip()` is now explicitly Vue-shell-owned/no-op in legacy runtime; legacy callers remain safe while no longer binding to removed `workspaceCtx*` IDs.
- [x] `initThemeUi` / `initWindowEffectUi` now run storage/bootstrap behavior without attaching listeners to removed hidden settings selectors, preserving runtime state without dead element assumptions.
- [x] Extended static UI parity guard assertions (`web/src/legacy/uiSurfaceParity.test.ts`) to enforce continued absence of removed IDs (`btnToggleWorkflowAdvanced`, `workflowSubtitle`, and all `workspaceCtx*` nodes).
- [x] Validation after dead-ID runtime decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after dead-ID runtime decommission slice: `legacy-app` ~`514.75 kB` (gzip ~`136.48 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `16612` to `16549` after dead-ID runtime coupling cleanup.
- [x] Extracted legacy post-init startup bootstrap into explicit exported runtime hook `runLegacyPostBootstrapStartup()` in `web/src/legacy/app.js` (settings restore/defaults, async startup initializers, status refresh, startup dataset restore scheduling).
- [x] `initLegacyApp()` now focuses on listener/runtime wiring and invokes post-bootstrap startup only in non-Vue-owner fallback mode, reducing hidden bootstrap ownership inside legacy init for Vue mode.
- [x] Vue runtime bridge (`web/src/components/LegacyRuntimeBridge.vue`) now explicitly triggers legacy post-bootstrap startup after `initLegacyApp()`, moving bootstrap sequencing ownership into Vue bridge orchestration.
- [x] Validation after post-bootstrap ownership extraction slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after post-bootstrap ownership extraction slice: `legacy-app` ~`514.83 kB` (gzip ~`136.52 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.55 kB` (gzip ~`45.17 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after post-bootstrap ownership extraction slice: `16555`.
- [x] Removed legacy aggregate-visibility runtime toggling from `setAggregateTab` in `web/src/legacy/app.js` (`batchWrap` / `aggregateMetaSection` / `metaSavedWrap` / `metaCompareWrap` style/open mutation paths), preserving only aggregate key state + persistence behavior.
- [x] Decommissioned unused aggregate compatibility wrapper IDs in `web/src/legacy/template.html` (`batchWrap`, `aggregateMetaSection`, `metaSavedWrap`, `metaCompareWrap`) now that Vue owns aggregate/meta surface visibility.
- [x] Extended static parity assertions in `web/src/legacy/uiSurfaceParity.test.ts` to keep removed aggregate wrapper IDs absent.
- [x] Validation after aggregate-wrapper/runtime decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after aggregate-wrapper/runtime decommission slice: `legacy-app` ~`514.35 kB` (gzip ~`136.36 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.47 kB` (gzip ~`45.14 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `16555` to `16527` after aggregate wrapper/runtime cleanup.
- [x] Removed additional dead compatibility IDs in `web/src/legacy/template.html` (`controlsTabPanels`, `resultsTabPanels`) after confirming no runtime references outside parity tests.
- [x] Updated static UI parity guard (`web/src/legacy/uiSurfaceParity.test.ts`) to enforce continued absence of `controlsTabPanels` / `resultsTabPanels`.
- [x] Validation after controls/results wrapper ID cleanup: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after controls/results wrapper ID cleanup: `legacy-app` ~`514.35 kB` (gzip ~`136.36 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Removed dead `#resultsTabBar` runtime selector branch from `web/src/legacy/app.js` (results tab bar compatibility markup is already decommissioned); heatmap fallback now relies only on headless tab state + Vue-owned tab surface.
- [x] `initMarketDataDirUi` and `initDistributedUi` now skip legacy hidden-button click listener registration when Vue workflow ownership is active; Vue store actions continue using typed runtime action dispatch without duplicate DOM listener ownership.
- [x] Extended static parity guard assertions in `web/src/legacy/uiSurfaceParity.test.ts` to enforce no residual `resultsTabBar` runtime coupling inside `web/src/legacy/app.js`.
- [x] Validation after legacy hidden-button listener/runtime selector decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after legacy hidden-button listener/runtime selector decommission slice: `legacy-app` ~`514.03 kB` (gzip ~`136.22 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line count reduced from `18298` to `18294` in this decommission slice.
- [x] Added `addHiddenClickListener(...)` fallback-only binder inside `initLegacyApp()` so hidden compatibility button listeners are not registered while Vue owns workflow/runtime.
- [x] Migrated `initLegacyApp()` hidden button listener wiring to `addHiddenClickListener(...)` across run/cancel/validate, validation export/cross controls, history/saved-run actions, meta/stage4 actions, and baseline actions; non-click input/change listeners remain intact for compatibility field/state synchronization.
- [x] Validation after `initLegacyApp` hidden-click decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after `initLegacyApp` hidden-click decommission slice: `legacy-app` ~`512.52 kB` (gzip ~`136.16 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after `initLegacyApp` hidden-click decommission slice: `18297`.
- [x] Added fallback-only hidden-click binder in `initLegacyPrimaryListeners()` (`bindHiddenClickListeners` + `addPrimaryHiddenClickListener`) so hidden compatibility controls do not register click/escape modal handlers in Vue-owner mode.
- [x] Migrated `initLegacyPrimaryListeners()` hidden button listener wiring (`btnLoad`/`btnApplyTimeframe`/`btnBrowseCsv`/`btnPolygon*`/`btnExec*`/`btnPresetKernelSmaAlma`/`btnRefreshGpuStatus`/`btnGpuParityCheck`/`btnClear`) to fallback-only binding while retaining non-click field listeners for compatibility state sync.
- [x] Validation after `initLegacyPrimaryListeners` hidden-click decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `90` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after `initLegacyPrimaryListeners` hidden-click decommission slice: `legacy-app` ~`512.16 kB` (gzip ~`136.17 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after `initLegacyPrimaryListeners` hidden-click decommission slice: `18306`.
- [x] Overview empty-state command ownership moved from legacy action registry to Vue store orchestration: `analysisStore.overviewEmptyPrimary()` / `overviewEmptySecondary()` now route via `runOverviewEmptyAction(...)` using `workflowStore`, `dataStore`, and `optimizeStore` actions instead of `invokeLegacyAction('overview.empty*')`.
- [x] Removed `overview.emptyPrimary` / `overview.emptySecondary` from typed legacy action union (`web/src/legacy/actions.ts`) and from `LEGACY_ACTION_REGISTRY` in `web/src/legacy/app.js`.
- [x] Added/updated analysis action tests to cover Vue-routed overview action paths (`browse_csv`, `polygon`, `go_opt`, `run_opt`, `cancel_run`) in `web/src/stores/analysisStore.actions.test.ts`.
- [x] Validation after overview action-ownership decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `92` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after overview action-ownership decommission slice: `legacy-app` ~`512.61 kB` (gzip ~`136.50 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after overview action-ownership decommission slice: `18304`.
- [x] `initOverviewEmptyUi()` and `initBestCardUi()` now register hidden compatibility click listeners only in non-Vue-owner fallback mode (`isVueWorkflowOwner()` gating), removing additional duplicate hidden click ownership under Vue runtime.
- [x] Validation after overview/best-card fallback-listener gating slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `92` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after overview/best-card fallback-listener gating slice: `legacy-app` ~`512.63 kB` (gzip ~`136.50 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after overview/best-card fallback-listener gating slice: `18305`.
- [x] Data clear/timeframe action ownership moved off legacy action registry: `dataStore.clearData()` and `dataStore.applyTimeframe()` now call direct runtime wrappers (`invokeLegacyDataClearRuntime`, `invokeLegacyDataApplyTimeframeRuntime`) from `web/src/legacy/actions.ts`.
- [x] Added explicit direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyDataClearRuntime`, `invokeLegacyDataApplyTimeframeRuntime`) and removed `data.clear` / `data.applyTimeframe` from `LEGACY_ACTION_REGISTRY`.
- [x] Removed `data.clear` / `data.applyTimeframe` from typed `LegacyActionName` union in `web/src/legacy/actions.ts`; current union count is now `49` entries.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (direct runtime wrapper coverage) and `web/src/stores/dataStore.actions.test.ts` (clear/apply-timeframe runtime wrapper assertions).
- [x] Validation after data clear/timeframe action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `95` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after data clear/timeframe action-registry decommission slice: `legacy-app` ~`513.01 kB` (gzip ~`136.60 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after data clear/timeframe action-registry decommission slice: `18311`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `103` to `101` in this slice.
- [x] Optimize run/cancel/validate action ownership moved off legacy action registry: `optimizeStore.runOptimization()` / `cancelOptimization()` / `runValidation()` and `validationStore.runValidation()` now call direct runtime wrappers (`invokeLegacyOptimizeRunRuntime`, `invokeLegacyOptimizeCancelRuntime`, `invokeLegacyOptimizeValidateRuntime`) from `web/src/legacy/actions.ts`.
- [x] Added explicit optimize direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyOptimizeRunRuntime`, `invokeLegacyOptimizeCancelRuntime`, `invokeLegacyOptimizeValidateRuntime`) and removed `optimize.run` / `optimize.cancel` / `optimize.validate` from `LEGACY_ACTION_REGISTRY`.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (optimize wrapper coverage) and updated `web/src/stores/optimizeStore.actions.test.ts`, `web/src/stores/validationStore.actions.test.ts`, `web/src/stores/commandActionStore.test.ts`, and `web/src/stores/analysisStore.actions.test.ts` for direct optimize-runtime dispatch expectations.
- [x] Validation after optimize action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `98` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after optimize action-registry decommission slice: `legacy-app` ~`513.57 kB` (gzip ~`136.69 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after optimize action-registry decommission slice: `18320`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `101` to `93` in this slice.
- [x] Remaining optimize utility actions (`optimize.applyPreset`, `optimize.presetKernel`, `optimize.refreshGpuStatus`, `optimize.gpuParityCheck`) moved off legacy action registry: `optimizeStore` now dispatches direct runtime wrappers for preset/GPU utility actions.
- [x] Added explicit optimize utility direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyOptimizeApplyPresetRuntime`, `invokeLegacyOptimizePresetKernelRuntime`, `invokeLegacyOptimizeRefreshGpuStatusRuntime`, `invokeLegacyOptimizeGpuParityCheckRuntime`) and removed the corresponding `optimize.*` utility entries from `LEGACY_ACTION_REGISTRY`.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (optimize utility wrapper coverage) and updated `web/src/stores/optimizeStore.actions.test.ts` for direct utility-runtime dispatch expectations.
- [x] Validation after optimize utility action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `102` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after optimize utility action-registry decommission slice: `legacy-app` ~`514.35 kB` (gzip ~`136.73 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after optimize utility action-registry decommission slice: `18332`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `93` to `89` in this slice.
- [x] Execution action ownership moved off legacy action registry: `executionStore` run/control actions (`testConnection`, `runOnce`, `refreshOrders`, `startLive`, `stopLive`, `startRunner`, `stopRunner`, `clearKeys`) now call direct runtime wrappers.
- [x] Added explicit execution direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyExecution*Runtime`) and removed `execution.*` action entries from `LEGACY_ACTION_REGISTRY`.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (execution wrapper coverage) and updated `web/src/stores/executionStore.actions.test.ts` for direct execution-runtime dispatch expectations.
- [x] Validation after execution action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `103` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after execution action-registry decommission slice: `legacy-app` ~`515.89 kB` (gzip ~`136.87 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after execution action-registry decommission slice: `18356`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `89` to `81` in this slice.
- [x] Distributed action ownership moved off legacy action registry: `distributedStore` control actions (`generateToken`, worker start/stop/status, polygon fill/current CSV, run assets/tiles, cancel) now call direct runtime wrappers.
- [x] Added explicit distributed direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyDistributed*Runtime`) and removed `distributed.*` action entries from `LEGACY_ACTION_REGISTRY`.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (distributed wrapper coverage) and updated `web/src/stores/distributedStore.actions.test.ts` for direct distributed-runtime dispatch expectations.
- [x] Validation after distributed action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after distributed action-registry decommission slice: `legacy-app` ~`517.63 kB` (gzip ~`137.03 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after distributed action-registry decommission slice: `18383`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `81` to `72` in this slice.
- [x] History action ownership moved off legacy action registry: `historyStore` session/saved-run actions now call direct runtime wrappers (`invokeLegacyHistory*Runtime`) instead of `invokeLegacyAction('history.*')`.
- [x] Added explicit history direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyHistory*Runtime`) and removed `history.*` action entries from `LEGACY_ACTION_REGISTRY`.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (history wrapper coverage) and updated `web/src/stores/historyStore.actions.test.ts` for direct history-runtime dispatch expectations.
- [x] Validation after history action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `105` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after history action-registry decommission slice: `legacy-app` ~`519.95 kB` (gzip ~`137.21 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after history action-registry decommission slice: `18419`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `72` to `60` in this slice.
- [x] Validation action ownership moved off legacy action registry: `validationStore` export and cross-dataset actions now call direct runtime wrappers (`invokeLegacyValidation*Runtime`) instead of `invokeLegacyAction('validation.*')`.
- [x] Added explicit validation direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyValidation*Runtime`) and removed `validation.*` action entries from `LEGACY_ACTION_REGISTRY`.
- [x] Updated action/store tests for this slice: expanded `web/src/legacy/actions.test.ts` (validation wrapper coverage), updated `web/src/stores/validationStore.actions.test.ts`, and updated `web/src/stores/commandActionStore.test.ts` (`exportValidationShortlistCsv` now routes through validation runtime wrapper path).
- [x] Validation after validation action-registry decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `106` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after validation action-registry decommission slice: `legacy-app` ~`521.68 kB` (gzip ~`137.35 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after validation action-registry decommission slice: `18446`.
- [x] Non-legacy runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `60` to `51` in this slice.
- [x] Data load/browse/Polygon/market-data-dir action ownership moved off legacy action registry: `dataStore` now dispatches direct runtime wrappers for `data.load`, `data.loadSampleCsv`, `data.browseCsv`, `data.openPolygon`, `data.browseMarketDataDir`, `data.setMarketDataDir`, `data.resetMarketDataDir`, and `data.openMarketDataDir`.
- [x] Added explicit data direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyDataLoadRuntime`, `invokeLegacyDataLoadSampleCsvRuntime`, `invokeLegacyDataBrowseCsvRuntime`, `invokeLegacyDataOpenPolygonRuntime`, `invokeLegacyDataBrowseMarketDataDirRuntime`, `invokeLegacyDataSetMarketDataDirRuntime`, `invokeLegacyDataResetMarketDataDirRuntime`, `invokeLegacyDataOpenMarketDataDirRuntime`) and removed the corresponding `data.*` entries from `LEGACY_ACTION_REGISTRY`.
- [x] Analysis/meta action ownership moved off legacy action registry: `analysisStore` now dispatches direct runtime wrappers (`invokeLegacyAnalysis*Runtime`, `invokeLegacyMeta*Runtime`) for top-table/heatmap/drill and meta/batch/stage4/ml actions.
- [x] Added explicit analysis/meta direct-runtime exports in `web/src/legacy/app.js` (`invokeLegacyAnalysis*Runtime`, `invokeLegacyMeta*Runtime`); removed the final `analysis.*`/`meta.*` registry entries; and removed `LEGACY_ACTION_REGISTRY` from `web/src/legacy/app.js`.
- [x] Added compatibility runtime dispatcher `invokeLegacyRuntimeAction(...)` in `web/src/legacy/app.js` as a thin switch-based bridge over direct runtime exports (no registry object) to preserve legacy adapter/test contracts during transition.
- [x] Updated action/store tests for these slices: expanded `web/src/stores/dataStore.actions.test.ts`, `web/src/stores/analysisStore.actions.test.ts`, `web/src/stores/commandActionStore.test.ts`, `web/src/stores/settingsStore.actions.test.ts`, and `web/src/legacy/actions.test.ts` for direct wrapper dispatch expectations.
- [x] Validation after data + analysis/meta action-registry decommission slices: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `107` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after data + analysis/meta action-registry decommission slices: `legacy-app` ~`532.42 kB` (gzip ~`138.33 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after final action-registry decommission slices: `18752`.
- [x] Non-test runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `51` to `1` (compatibility shim in `web/src/legacy/actions.ts`).
- [x] Removed compatibility legacy action shim from runtime bridge: deleted `invokeLegacyAction`/`LegacyActionName` bridge path from `web/src/legacy/actions.ts` and removed `invokeLegacyRuntimeAction(...)` from `web/src/legacy/app.js`.
- [x] Updated remaining store/action test harnesses to stop importing/mocking `invokeLegacyAction` (`dataStore.actions.test.ts`, `optimizeStore.actions.test.ts`, `validationStore.actions.test.ts`, `historyStore.actions.test.ts`, `executionStore.actions.test.ts`, `settingsStore.actions.test.ts`, `distributedStore.actions.test.ts`, `legacy/actions.test.ts`).
- [x] Validation after compatibility-shim removal slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after compatibility-shim removal slice: `legacy-app` ~`530.60 kB` (gzip ~`137.95 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`167.43 kB` (gzip ~`45.11 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after compatibility-shim removal slice: `18581`.
- [x] Non-test runtime `invokeLegacyAction(...)` callsites across `web/src` reduced from `1` to `0` (remaining string reference is parity-test guard only in `componentActionOwnership.test.ts`).
- [x] Hidden legacy Data action controls removed from compatibility template (`btnBrowseCsv`, `btnPolygonOpen`, `btnLoad`, `btnClear`, `btnBrowseMarketDataDir`, `btnSetMarketDataDir`, `btnResetMarketDataDir`, `btnOpenMarketDataDir`, `btnApplyTimeframe`) to reduce hidden button-coupling in Vue-owner mode.
- [x] `dataContextBridge` now derives Data action capability flags (`can*`) from runtime state (`data-running` + loaded dataset state) instead of hidden button disabled-state observers.
- [x] Removed Data bridge button observer coupling (`DATA_BUTTON_IDS` observer path) and switched root attribute observation to include `data-running` changes for capability sync.
- [x] Updated parity guard assertions in `web/src/legacy/uiSurfaceParity.test.ts` to enforce continued absence of removed hidden Data action button IDs.
- [x] Validation after hidden Data-action template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after hidden Data-action template decommission slice: `legacy-app` ~`530.22 kB` (gzip ~`137.91 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`166.73 kB` (gzip ~`45.01 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Removed dead hidden-button wiring for decommissioned Data controls in `web/src/legacy/app.js` (`initLegacyPrimaryListeners` Data button listeners + `initMarketDataDirUi` button listeners).
- [x] Trimmed decommissioned Data button IDs from `setRunning(...)` disable list in `web/src/legacy/app.js` after template removal.
- [x] Validation after Data listener/runtime cleanup slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after Data listener/runtime cleanup slice: `legacy-app` ~`528.99 kB` (gzip ~`137.55 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`166.73 kB` (gzip ~`45.01 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Data listener/runtime cleanup slice: `18559`.
- [x] Hidden legacy Execution/Distributed action controls removed from compatibility template (`btnExec*` and `btnDist*` sets), keeping Vue panels/context docks as the only action surfaces for those domains.
- [x] `executionDistributedBridge` now derives capability flags from runtime state attributes (`data-running`, `data-exec-busy`) instead of hidden button disabled-state observers; removed `DIST_BUTTON_IDS`/`EXEC_BUTTON_IDS` bridge observer coupling.
- [x] Added explicit execution runtime signal attribute updates in `setExecBusy(...)` (`document.documentElement.dataset.execBusy`) and removed dead hidden button disabled/listener wiring in `web/src/legacy/app.js` (setRunning id list coupling, distributed hidden-button listeners in `initDistributedUi`, execution hidden click listeners).
- [x] Updated parity guard assertions in `web/src/legacy/uiSurfaceParity.test.ts` to enforce continued absence of removed hidden Execution/Distributed action button IDs.
- [x] Validation after hidden Execution/Distributed template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after hidden Execution/Distributed template decommission slice: `legacy-app` ~`514.36 kB` (gzip ~`134.16 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`165.32 kB` (gzip ~`44.74 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Execution/Distributed listener-runtime cleanup slice: `18517`.
- [x] Hidden legacy Optimize/Validation action controls removed from compatibility template (`btnApplyPreset`, `btnPresetKernelSmaAlma`, `btnRefreshGpuStatus`, `btnGpuParityCheck`, `btnRun`, `btnCancel`, `btnValidate`, `btnCrossRefresh`, `btnCrossSelectAll`, `btnCrossSelectNone`, `btnCrossSelectMatch`, `btnExportValidationCsv`, `btnExportValidationJson`, `btnExportReportJson`, `btnExportReportXlsx`).
- [x] `optimizeValidationBridge` capability derivation moved off hidden button disabled-state and now reads runtime-owned attributes (`data-opt-can-*`, `data-val-can-*`, `data-running`); removed optimize/validation button observer/listener coupling from the bridge.
- [x] Added explicit optimize/validation runtime flags in `web/src/legacy/app.js` at owning update points (`updateRunReadinessUi`, `updateValidateTabUi`, `updateStrategyUi`, `updateValidationExportButtons`, `updateCrossMeta`) and removed dead hidden listener wiring for removed optimize/validation controls.
- [x] Removed dead runtime branches tied only to removed optimize/validation action buttons in `web/src/legacy/app.js` (run/cancel/validate/export/cross/preset hidden-button branches) and updated parity assertions to enforce continued absence.
- [x] Validation after hidden Optimize/Validation template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after hidden Optimize/Validation template decommission slice: `legacy-app` ~`512.43 kB` (gzip ~`133.69 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`163.95 kB` (gzip ~`44.49 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Optimize/Validation listener-runtime cleanup slice: `18455`.
- [x] Hidden legacy History action controls removed from compatibility template (`btnApplyHistory`, `btnExportPreset`, `btnImportPreset`, `btnClearHistory`, `btnSaveSavedRunTags`, `btnRefreshSavedRuns`, `btnLoadSavedRun`, `btnApplySavedRun`, `btnRerunSavedRun`, `btnOpenSavedRunReport`, `btnRevealSavedRun`, `btnDeleteSavedRun`).
- [x] `analysisHistorySettingsBridge` History capability derivation moved off hidden button disabled-state and now reads runtime-owned attributes (`data-hist-can-*`, `data-running`); removed History button observer coupling from the bridge.
- [x] Added explicit History runtime flags in `web/src/legacy/app.js` (`syncSavedRunSelectionUi`) and removed dead hidden listener wiring for removed History controls in `initLegacyApp`.
- [x] Removed dead runtime branches tied only to removed History action buttons in `web/src/legacy/app.js` (saved-run hidden button disable wiring + `setRunning(...)` id-list entries) and updated parity assertions to enforce continued absence.
- [x] Validation after hidden History template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after hidden History template decommission slice: `legacy-app` ~`512.14 kB` (gzip ~`133.57 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`163.22 kB` (gzip ~`44.38 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after History listener-runtime cleanup slice: `18417`.
- [x] Hidden legacy Analysis/Meta action controls removed from compatibility template (`btnCopyBest`, `btnExportCsv`, `btnExportJson`, `btnSetBaseline`, `btnClearBaseline`, `btnClearTopFilters`, `btnHeatmapExportPng`, `btnRunBatch`, `btnBatchStop`, `btnBatchRefresh`, `btnBatchSelectAll`, `btnBatchSelectNone`, `btnBatchRetryFailed`, `btnBatchRevealSummary`, `btnRunMeta`, `btnOpenMetaReport`, `btnRevealMetaFiles`, `btnMetaMlRefresh`, `btnMetaMlTrain`, `btnMetaMlApply`, `btnMetaMlDelete`, `btnRunMetaStage4`, `btnRevealMetaStage4`, `btnSaveMetaSavedTags`, `btnRefreshMetaSaved`, `btnLoadMetaSaved`, `btnOpenMetaSavedReport`, `btnRevealMetaSaved`, `btnDeleteMetaSaved`, `btnSaveStage4SavedTags`, `btnRefreshStage4Saved`, `btnLoadStage4Saved`, `btnRevealStage4Saved`, `btnDeleteStage4Saved`).
- [x] `analysisHistorySettingsBridge` Analysis/Meta capability derivation moved off hidden button disabled-state and now reads runtime-owned attributes (`data-ana-can-*`, `data-meta-can-*`, `data-running`); removed Analysis/Meta button observer coupling.
- [x] Added explicit Analysis/Meta runtime capability flags in `web/src/legacy/app.js` (`setRunning`, `renderMetaSelection`, `updateMetaMlButtons`, `updateMetaStage4Buttons`, `syncBatchSummaryButtons`, `syncMetaSavedSelectionUi`, `syncStage4SavedSelectionUi`, batch-stop lifecycle updates) and removed dead hidden listener wiring for removed Analysis/Meta controls in `initLegacyApp`.
- [x] Removed dead runtime branches tied only to removed Analysis/Meta action buttons in `web/src/legacy/app.js` (hidden disable/listener paths + `setRunning(...)` id-list entries) and updated parity assertions to enforce continued absence.
- [x] Validation after hidden Analysis/Meta template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after hidden Analysis/Meta template decommission slice: `legacy-app` ~`511.16 kB` (gzip ~`133.56 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`160.26 kB` (gzip ~`43.93 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Analysis/Meta listener-runtime cleanup slice: `18319`.
- [x] Hidden legacy Overview empty-state action controls removed from compatibility template (`btnOverviewEmptyPrimary`, `btnOverviewEmptySecondary`).
- [x] `analysisHistorySettingsBridge` Overview empty-state action derivation moved off hidden button reads and now uses runtime-owned attributes (`data-overview-primary-*`, `data-overview-secondary-*`), with bridge observers updated accordingly.
- [x] Added explicit Overview runtime action state emission in `web/src/legacy/app.js` (`setOverviewActionState(...)` within `updateOverviewEmptyUi`) and removed dead hidden Overview button listener wiring.
- [x] Validation after hidden Overview template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after hidden Overview template decommission slice: `legacy-app` ~`510.90 kB` (gzip ~`133.48 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`160.06 kB` (gzip ~`43.87 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Overview listener-runtime cleanup slice: `18277`.
- [x] Hidden Advanced MO reset control removed from compatibility template (`btnMoResetSearchSpace`), with Advanced capability/enablement moved to runtime-owned signal (`data-adv-can-reset-mo-search-space`) and Vue store ownership (`advancedStore.canResetMoSearchSpace` + `AdvancedWorkspacePanel` reset action button).
- [x] Added explicit Advanced MO reset runtime ownership path in `web/src/legacy/app.js` (`handleAdvancedResetMoSearchSpaceAction`, `invokeLegacyAdvancedResetMoSearchSpaceRuntime`) and removed dead hidden button disable/listener coupling for the removed control.
- [x] Validation after Advanced MO reset decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after Advanced MO reset decommission slice: `legacy-app` ~`511.51 kB` (gzip ~`133.60 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.93 kB` (gzip ~`43.84 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Hidden Polygon modal action button IDs removed from compatibility template (`btnPolygonClose`, `btnPolygonSaveKey`, `btnPolygonClearKey`, `btnPolygonDownloadLoad`), replaced with semantic runtime action selectors (`data-polygon-modal-action`, `data-polygon-action`).
- [x] Removed legacy runtime coupling to Polygon action button IDs in `web/src/legacy/app.js` (`setRunning` ID list + listener binding), added selector-based action helpers (`polygonModalActionButton`, `polygonActionButton`, `setPolygonModalActionButtonsDisabled`), and dropped stale `btnPolygonOpen` compatibility branch in `initPolygonUi`.
- [x] Validation after Polygon modal action-ID decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after Polygon modal action-ID decommission slice: `legacy-app` ~`511.73 kB` (gzip ~`133.59 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Polygon modal action-ID decommission slice: `18299`.
- [x] Removed final stale hidden Data-control coupling from runtime (`btnApplyTimeframe` branch in `updateDataTimeframeUi`), so all literal `$(...)` IDs referenced by `web/src/legacy/app.js` now resolve to existing compatibility-template IDs (no dangling hidden button references).
- [x] Removed dead `initLegacyApp` listener scaffold (`addHiddenClickListener`) after hidden-button decommission slices; remaining document-level listener ownership is now limited to Polygon modal Escape handling.
- [x] Validation after Data dead-control/runtime-listener cleanup slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after Data dead-control/runtime-listener cleanup slice: `legacy-app` ~`511.71 kB` (gzip ~`133.58 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Data dead-control/runtime-listener cleanup slice: `18291`.
- [x] Polygon modal Escape-key ownership moved from legacy runtime to Vue shortcut owner: removed legacy document-level Escape listener in `initLegacyPrimaryListeners`, added direct runtime close export (`invokeLegacyDataClosePolygonRuntime`), and wired `GlobalShortcutBridge` Escape handling through `dataStore.closePolygon` when modal is open.
- [x] Legacy runtime now has no global document/window listener registrations (`document.addEventListener` / `window.addEventListener`) in `web/src/legacy/app.js`.
- [x] Validation after Polygon Escape ownership cutover slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after Polygon Escape ownership cutover slice: `legacy-app` ~`511.82 kB` (gzip ~`133.55 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Polygon Escape ownership cutover slice: `18288`.
- [x] Polygon modal action listeners in `initLegacyPrimaryListeners` are now always registered under Vue ownership (removed `bindHiddenClickListeners` fallback gating), fixing functional risk where modal save/clear/download actions could be disabled in Vue-owner mode.
- [x] Removed dead listener-branch scaffolding tied to the old fallback-gated Polygon listener path while keeping existing modal behavior and compatibility IDs intact.
- [x] Validation after Polygon modal listener-ownership stabilization slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after Polygon modal listener-ownership stabilization slice: `legacy-app` ~`511.79 kB` (gzip ~`133.54 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after Polygon modal listener-ownership stabilization slice: `18278`.
- [x] Static compatibility toast host markup removed from `web/src/legacy/template.html` (`toastHost`); toast rendering now relies on runtime host creation fallback in `web/src/legacy/core/toasts.js`, reducing hidden template anchors.
- [x] Updated parity guard assertions in `web/src/legacy/uiSurfaceParity.test.ts` to enforce continued absence of `toastHost`.
- [x] Validation after toast-host template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after toast-host template decommission slice: `legacy-app` ~`511.79 kB` (gzip ~`133.54 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.85 kB` (gzip ~`43.80 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after toast-host template decommission slice: `18278`.
- [x] Removed stale compatibility anchor IDs from `web/src/legacy/template.html` (`runFooter`, `stabilityTable`, `tradeTable`, `execOrdersTable`) while preserving the underlying markup/behavior; updated parity assertions to enforce continued absence.
- [x] Exact ID-reference audit across `web/src` now reports zero exact-unreferenced template IDs after stale-anchor cleanup.
- [x] Validation after stale-anchor template decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`28` files / `104` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after stale-anchor template decommission slice: `legacy-app` ~`511.79 kB` (gzip ~`133.54 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.79 kB` (gzip ~`43.79 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after stale-anchor template decommission slice: `18278`.
- [x] Quick-actions runtime metadata ownership moved off legacy root dataset writes: `updateQuickActionsUi` in `web/src/legacy/app.js` now emits `vta:quick-actions-snapshot` events (report/meta report paths), and Vue command action ownership is store-backed (`commandActionStore.applyLegacyQuickActionsSnapshot`) via new `attachQuickActionsBridge`.
- [x] Command palette/shortcuts now consume store-owned latest report availability (`canOpenLatestReport`, `canOpenLatestMetaReport`) instead of unconditional/no-op path opens, reducing hidden DOM coupling and improving UX feedback.
- [x] Validation after quick-actions runtime ownership slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`29` files / `106` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Build after quick-actions runtime ownership slice: `legacy-app` ~`514.12 kB` (gzip ~`134.14 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.79 kB` (gzip ~`43.79 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after quick-actions runtime ownership slice: `16469`.
- [x] Workflow state-machine Vue ownership tightened: `applyWorkflow(...)` in `web/src/legacy/app.js` now dispatches to Vue `workflow.select` first and returns on accepted dispatch, preventing legacy primary mutation in Vue-owner mode.
- [x] Vue-owner legacy tab inference fallback decommissioned in `syncWorkflowToActiveTabs(...)`: Vue-owner path now runs runtime sync-only (`syncWorkflowRuntimeFromDom` + snapshot emit) instead of mutating workflow/tab state.
- [x] Bootstrap ownership phased explicitly in `LegacyRuntimeBridge.vue`: `runtime init` -> `domain init` -> `event subscription` -> `ui sync`, with workflow/settings persisted startup restoration now applied in Vue-side bootstrap sync phase.
- [x] Bridge contract tightening completed for stale removed IDs: `analysisHistorySettingsBridge` dropped `themeSelect`/`effectSelect` dependency; `dataContextBridge` dropped `workspaceCtxStatus` dependency.
- [x] Stale CSS selector decommission completed for removed compatibility IDs in `web/src/styles/app.css` (`#effectSelect`, `#btnToggleWorkflowAdvanced`, `#controlsTabBar`, `#resultsTabBar` removed).
- [x] Validation after workflow/bootstrap/style decommission slice: `npm --prefix web run typecheck`, `npm --prefix web run test` (`29` files / `106` tests), `npm --prefix web run lint`, and `npm --prefix web run build` all passing.
- [x] Additional regression/performance gates after this slice: `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` PASS (`runtime_not_regressed_50pct` gate passed) and `cargo test --workspace` PASS.
- [x] Build after workflow/bootstrap/style decommission slice: `legacy-app` ~`514.21 kB` (gzip ~`134.17 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.79 kB` (gzip ~`43.79 kB`), `assets/index-*.css` ~`35.34 kB` (gzip ~`7.70 kB`).
- [x] Current `web/src/legacy/app.js` line count snapshot after workflow/bootstrap/style decommission slice: `16475`.

---

## 5) Proposed Frontend Structure

```text
web/
  src/
    app/
      App.vue
      main.ts
    tauri/
      invoke.ts
      events.ts
      commands.ts
    stores/
      data.ts
      optimize.ts
      validation.ts
      analysis.ts
      history.ts
      settings.ts
      distributed.ts
      execution.ts
      tasks.ts
    features/
      data/
      optimize/
      validate/
      analyze/
      history/
      settings/
      distributed/
      execution/
    components/
      charts/
      tables/
      feedback/
      layout/
    utils/
      storage.ts
      formatting.ts
      errors.ts
      perf.ts
```

---

## 6) Definition of Done

- [x] Users can perform every currently supported workflow in the Vue UI.
- [x] No command/event/storage regressions.
- [x] Task progress/feedback is materially better than current UI.
- [x] Performance is neutral or better vs baseline.
- [x] Old data/artifacts/settings continue to load correctly.

---

## 7) Refresh Commands (Run Before Starting Implementation)

```powershell
# Refresh package versions
npm view vue version
npm view vite version
npm view @vitejs/plugin-vue version
npm view @tauri-apps/api version
npm view @tauri-apps/cli version

# Confirm Vite Node engine requirement
npm view vite engines --json
```

---

## 8) Source Links (Docs + Release Channels)

- Tauri frontend options (`Vite` recommendation): https://v2.tauri.app/start/frontend/
- Tauri create project/templates (`vanilla`, `vue`, `react`, etc.): https://v2.tauri.app/start/create-project/
- Tauri calling Rust (invoke): https://v2.tauri.app/develop/calling-rust/
- Tauri events/calling frontend: https://v2.tauri.app/develop/calling-frontend/
- Tauri plugins overview: https://v2.tauri.app/plugin/
- Vue docs: https://vuejs.org/
- Vue performance guide: https://vuejs.org/guide/best-practices/performance
- Vite docs: https://vite.dev/guide/
- Vite 7 announcement: https://vite.dev/blog/announcing-vite7
- npm package index (version pages):
- https://www.npmjs.com/package/vue
- https://www.npmjs.com/package/vite
- https://www.npmjs.com/package/@vitejs/plugin-vue
- https://www.npmjs.com/package/@tauri-apps/api
- https://www.npmjs.com/package/@tauri-apps/cli
