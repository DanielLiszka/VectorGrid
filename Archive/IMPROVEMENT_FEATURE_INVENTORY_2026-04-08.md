# Improvement Feature Inventory

Last updated: 2026-04-08

## Purpose

This document is the improvement-only inventory for the current shipped and in-repo VectorGrid application.

It is intentionally scoped to:

- existing features
- existing workflows
- existing infrastructure
- existing quality/performance/reliability surfaces

It is intentionally not a net-new feature roadmap.

## How To Use This With REPO_MAP

[REPO_MAP.md](/C:/Rust Projects/ta_desktop_demo/REPO_MAP.md) is the right starting point because it is the canonical routing document for the current repo. Use it to find where a subsystem lives. Use this inventory to decide what the subsystem is and whether the work should be:

- polish
- hardening
- simplification
- correctness work
- performance work
- test coverage
- documentation

In short:

- `REPO_MAP.md` tells you where to go.
- this document tells you what already exists and therefore what can be improved without expanding scope.

## Inventory Rules

- improvement means strengthening an existing surface, not inventing a new product lane
- hidden or partially implemented surfaces still count if they are already in the repo
- disabled surfaces still count if they are part of current behavior or can be re-enabled without fresh product design

## 1. Desktop Shell And Global UX

### 1.1 App shell composition

Current surface:

- [App.vue](/C:/Rust Projects/ta_desktop_demo/web/src/App.vue)
- [VueWorkspaceShell.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [DesktopTitlebar.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue)
- [RunFooterPanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/RunFooterPanel.vue)
- [AppRuntimeBanner.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [TaskCenterOverlay.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [CommandPaletteOverlay.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/CommandPaletteOverlay.vue)
- [GlobalShortcutBridge.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/GlobalShortcutBridge.vue)
- [WorkflowQuickSwitch.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/WorkflowQuickSwitch.vue)

Existing capability:

- desktop-native shell composition
- runtime banner and task center overlays
- titlebar integration
- command palette
- quick workflow switching in dev-gated mode
- footer-level run state visibility

### 1.2 Startup and boot-state experience

Current surface:

- [main.ts](/C:/Rust Projects/ta_desktop_demo/web/src/main.ts)
- [runtimeStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts)
- [StartupAnimationOverlay.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)

Existing capability:

- staged runtime boot tracking
- ready/error boot reporting
- startup overlay component still exists
- startup overlay currently disabled at the top-level mount site

### 1.3 Settings and workstation behavior

Current surface:

- [SettingsWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [settingsStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/settingsStore.ts)
- [desktopProductization.ts](/C:/Rust Projects/ta_desktop_demo/web/src/services/desktopProductization.ts)

Existing capability:

- desktop runtime status visibility
- workstation settings and preferences
- notification/window-state integration
- operator-facing diagnostics inside the app

## 2. Core Optimization Workflow

### 2.1 Data loading and dataset preparation

Current surface:

- [DataWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/DataWorkspacePanel.vue)
- [dataStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/dataStore.ts)
- [commands_data.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_data.rs)

Existing capability:

- CSV loading
- sample/repo dataset discovery
- resampling
- data directory selection
- Polygon-backed dataset download/top-off/repair/validation

### 2.2 Optimization request shaping

Current surface:

- [OptimizeWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/OptimizeWorkspacePanel.vue)
- [StrategyWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/StrategyWorkspacePanel.vue)
- [AdvancedWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/AdvancedWorkspacePanel.vue)
- [optimizeStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/optimizeStore.ts)
- [strategyStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/strategyStore.ts)
- [advancedStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/advancedStore.ts)
- [optimizeRequestBuilder.ts](/C:/Rust Projects/ta_desktop_demo/web/src/services/optimizeRequestBuilder.ts)
- [strategyOptimizationCapabilities.ts](/C:/Rust Projects/ta_desktop_demo/web/src/utils/strategyOptimizationCapabilities.ts)

Existing capability:

- full optimization parameter entry
- route-aware legality checks
- strategy-specific capability shaping
- request normalization
- GPU/CPU/hybrid route selection support

### 2.3 Optimization execution

Current surface:

- [commands_optimize.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_optimize.rs)
- [double_ma.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs)
- [composite_profile_route.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_profile_route.rs)
- [gpu_backtest_kernel.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/gpu_backtest_kernel.rs)

Existing capability:

- exact-grid optimization
- GPU kernel route for `DoubleMa`
- `GpuVdps`
- hybrid/product-route policy for composite-style profiles
- CPU fallback

## 3. Results Surfaces

### 3.1 Overview/top/pareto

Current surface:

- [OverviewWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/OverviewWorkspacePanel.vue)
- [TopWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/TopWorkspacePanel.vue)
- [ParetoWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/ParetoWorkspacePanel.vue)

Existing capability:

- aggregate run overview
- ranked top-result views
- pareto-style views where supported
- virtualized row rendering for heavy result tables

### 3.2 Validation/drilldown/heatmap

Current surface:

- [ValidationWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/ValidationWorkspacePanel.vue)
- [DrillWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/DrillWorkspacePanel.vue)
- [HeatmapWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/HeatmapWorkspacePanel.vue)
- [commands_validation.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_validation.rs)
- [commands_analysis.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_analysis.rs)

Existing capability:

- validation result review
- drilldown/local-stability analysis
- heatmap rendering and overlays
- price-bar extraction for validation/review

### 3.3 Meta/debug/history results

Current surface:

- [MetaWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/MetaWorkspacePanel.vue)
- [DebugWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/DebugWorkspacePanel.vue)
- [HistoryWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/HistoryWorkspacePanel.vue)
- [HistoryContextDock.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/HistoryContextDock.vue)
- [historyStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/historyStore.ts)
- [workflowArtifactsStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/workflowArtifactsStore.ts)
- [commands_artifacts.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_artifacts.rs)

Existing capability:

- run artifact browsing
- hydrated replay loading
- artifact tagging
- library/history search and inspection
- meta result review
- debug/result diagnostics

## 4. Validation And Robustness Workflow

### 4.1 Double-MA validation engine

Current surface:

- [double_ma_validate.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_validate.rs)
- [validation_pipeline.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_pipeline.rs)
- [validation_shared.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_shared.rs)
- [cscv_pbo.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cscv_pbo.rs)

Existing capability:

- walk-forward
- reopt walk-forward
- CSCV/PBO
- SPA
- perturbation/trade bootstrap
- validation verdicting
- CPU fallback and partial GPU-backed validation support

### 4.2 Cross-ticker/meta validation

Current surface:

- [commands_validation.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_validation.rs)
- [commands_meta.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_meta.rs)
- [meta_select](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/meta_select.rs)
- [ml_confidence](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ml_confidence.rs)

Existing capability:

- cross-ticker validation
- meta selection
- stage 4 / stage 5 experimental meta flows
- ML confidence model training/application/evaluation

### 4.3 Fidelity audit harness

Current surface:

- [real_data_fidelity_audit.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/bin/real_data_fidelity_audit.rs)

Existing capability:

- exact vs `GpuVdps` optimization fidelity comparison
- baseline vs cheaper validation fidelity comparison
- holdout checking for optimization and validation winners
- JSON audit artifact generation

## 5. Strategy Families And Specialized Workflows

### 5.1 Double-MA family

Current surface:

- [double_ma.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma.rs)
- [crates/strategies/src/double_ma/mod.rs](/C:/Rust Projects/ta_desktop_demo/crates/strategies/src/double_ma/mod.rs)

Existing capability:

- flagship exact-grid engine
- mature GPU path
- validation and drilldown support

### 5.2 Composite and advanced strategy families

Current surface:

- [strategy_presets.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/strategy_presets.rs)
- [composite_cpu](/C:/Rust Projects/ta_desktop_demo/crates/strategies/src/composite_cpu/mod.rs)
- composite strategy modules under [crates/app_core/src](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src)

Existing capability:

- composite profile routing through existing product lanes
- multiple specialized strategy families in app-core
- mixed GPU/CPU coverage depending on family

### 5.3 CompositeMo

Current surface:

- [run_composite_mo_optimization](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_optimize.rs)
- [crates/app_core/src/composite_mo.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/composite_mo.rs)

Existing capability:

- multi-objective optimization lane
- CPU-bound execution path

### 5.4 ML Signal

Current surface:

- [MlSignalShellPanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/MlSignalShellPanel.vue)
- [MlSignalWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue)
- [commands_ml_signal.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_ml_signal.rs)
- [ml_signal.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ml_signal.rs)

Existing capability:

- ML signal dataset export
- score-cache generation
- validation
- policy sweep
- model bundle discovery/status

### 5.5 Options expression workflow

Current surface:

- [OptionsExpressionPanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- [optionsExpressionStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [optionsExpressionService.ts](/C:/Rust Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [commands_options_expression.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [commands_options_data.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
- [options_expression.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [options_expression_gpu.rs](/C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)

Existing capability:

- options dataset import/download
- options expression execution
- feasibility analysis
- tuple inspection
- event-path inspection
- artifact listing/loading
- CSV saving

## 6. Live Execution And Runtime Operations

### 6.1 Live execution runner

Current surface:

- [ExecutionWorkspacePanel.vue](/C:/Rust Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [executionStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [commands_execution.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_execution.rs)

Existing capability:

- test connection
- single signal execution
- live runner start/stop/status
- live snapshot inspection
- risk override/spec handling

### 6.2 Background tasks and runtime instrumentation

Current surface:

- [taskStore.ts](/C:/Rust Projects/ta_desktop_demo/web/src/stores/taskStore.ts)
- [instrumentation.ts](/C:/Rust Projects/ta_desktop_demo/web/src/tauri/instrumentation.ts)
- [runtimeError.ts](/C:/Rust Projects/ta_desktop_demo/web/src/tauri/runtimeError.ts)

Existing capability:

- task lifecycle tracking
- event/invoke instrumentation
- error normalization
- user-visible task state and toast surfaces

## 7. Desktop Integrations And System Utilities

### 7.1 Window/system integration

Current surface:

- [src-tauri/src/lib.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs)
- [commands_system.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs)
- [desktopProductization.ts](/C:/Rust Projects/ta_desktop_demo/web/src/services/desktopProductization.ts)

Existing capability:

- desktop logging
- notifications
- window state persistence
- window effect setting
- GPU status query

### 7.2 File/report utilities

Current surface:

- [commands_reports_api.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports_api.rs)
- [commands_reports.rs](/C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_reports.rs)

Existing capability:

- report generation
- shortlist CSV generation
- report saving
- path reveal/open utilities
- reports/runs directory opening

## 8. Testing, Perf, And Release Surfaces

### 8.1 Frontend verification

Current surface:

- component/store/service tests under [web/src](/C:/Rust Projects/ta_desktop_demo/web/src)
- browser UX tests under [web/e2e/browser](/C:/Rust Projects/ta_desktop_demo/web/e2e/browser)
- desktop smoke harness under [web/e2e/desktop](/C:/Rust Projects/ta_desktop_demo/web/e2e/desktop)

Existing capability:

- Vitest unit coverage
- browser UX coverage
- Tauri desktop smoke coverage

### 8.2 Backend/runtime/perf gates

Current surface:

- [run_cpu_perf_gate.ps1](/C:/Rust Projects/ta_desktop_demo/scripts/run_cpu_perf_gate.ps1)
- [run_gpu_perf_gate.ps1](/C:/Rust Projects/ta_desktop_demo/scripts/run_gpu_perf_gate.ps1)
- [run_workflow_gate.ps1](/C:/Rust Projects/ta_desktop_demo/scripts/run_workflow_gate.ps1)
- [run_vectorbt_vs_vectorgrid_faceoff.ps1](/C:/Rust Projects/ta_desktop_demo/scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1)

Existing capability:

- CPU perf gate
- GPU perf gate
- workflow gate
- VectorBT comparison runs
- Nsight/CUPTI profiling scripts

### 8.3 Command and output parity

Current surface:

- [web/scripts/generate-tauri-commands.mjs](/C:/Rust Projects/ta_desktop_demo/web/scripts/generate-tauri-commands.mjs)
- [scripts/check_output_parity.mjs](/C:/Rust Projects/ta_desktop_demo/scripts/check_output_parity.mjs)
- [scripts/run_output_parity_check.mjs](/C:/Rust Projects/ta_desktop_demo/scripts/run_output_parity_check.mjs)

Existing capability:

- generated command-surface parity
- output-structure parity checks

## 9. Documentation And Operational Guidance

Current surface:

- [REPO_MAP.md](/C:/Rust Projects/ta_desktop_demo/REPO_MAP.md)
- [AGENTS.md](/C:/Rust Projects/ta_desktop_demo/AGENTS.md)
- [README.md](/C:/Rust Projects/ta_desktop_demo/README.md)
- [web/README.md](/C:/Rust Projects/ta_desktop_demo/web/README.md)
- workflow/research/frontend review docs across the repo

Existing capability:

- high-level routing
- repo policy
- product framing
- multiple implementation ledgers and research trackers

## Improvement-Only Master Area List

This is the condensed list to treat as the standing improvement inventory:

1. Desktop shell and overlays
2. Startup/boot behavior
3. Settings and workstation ergonomics
4. Data loading and dataset prep
5. Optimization request shaping
6. Optimization execution correctness and clarity
7. Overview/top/pareto results
8. Validation/drilldown/heatmap results
9. History/artifacts/meta/debug surfaces
10. Validation robustness pipeline
11. Cross-ticker/meta/ML confidence
12. Fidelity audit and evidence tooling
13. Double-MA core engine
14. Composite and specialized strategy families
15. CompositeMo
16. ML Signal
17. Options expression workflow
18. Live execution runner
19. Task lifecycle and runtime instrumentation
20. Desktop/system integration
21. Reports/file operations
22. Frontend test coverage
23. Backend/perf/profiling gates
24. Command/output parity
25. Documentation and operational guidance
