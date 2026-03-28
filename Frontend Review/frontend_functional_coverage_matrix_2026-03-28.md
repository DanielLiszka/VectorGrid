# Frontend Functional Coverage Matrix

Date: 2026-03-28

Scope: current Vue/Tauri frontend under `web/`

Purpose:

- inventory user-meaningful backend-backed capability currently exposed in the frontend
- identify the primary user path for each capability
- identify duplicate or redundant entry points that may be safe to consolidate later
- prevent UX cleanup from silently removing backend functionality from user reach

Primary code anchors:

- [web/README.md](C:\Rust Projects\ta_desktop_demo\web\README.md)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [commands.ts](C:\Rust Projects\ta_desktop_demo\web\src\tauri\commands.ts)
- [lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)

## Data

Capabilities:

- CSV/sample loading
- timeframe resampling
- market-data-root inspection/set/clear
- Polygon API key set/clear
- Polygon cache inspection
- Polygon 1m download + load
- Polygon dataset validate/repair

Primary frontend path:

- [DataWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DataWorkspacePanel.vue)
- [dataService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\dataService.ts)

Secondary or duplicate entry points:

- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue) duplicates market-data-root controls
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue) exposes a Polygon-oriented shortcut path

Current consolidation note:

- market-data-root controls are now intentionally consolidated
- `SettingsWorkspacePanel.vue` is the canonical surface for editing or resetting the market-data root
- `DataWorkspacePanel.vue` still exposes the active folder and a direct open-folder path, but now routes deeper directory management to Settings
- Polygon support actions should keep one canonical surface, with command-palette access treated as a shortcut rather than a second primary workflow

Interaction posture:

- routine: CSV/sample loading, timeframe resampling
- advanced: market-data-root management, Polygon cache/API-key management
- high-stakes: none

## Optimization

Capabilities:

- start optimization
- cancel current optimization / current run
- build request payloads and route preview
- dynamic strategy IR preview / formatting / encoding
- GPU status and speedup inspection
- moving-average / indicator catalog loading

Primary frontend path:

- [OptimizeWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\OptimizeWorkspacePanel.vue)
- [optimizeService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\optimizeService.ts)

Secondary or duplicate entry points:

- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [AnalysisContextDock.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AnalysisContextDock.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)

Current consolidation note:

- optimization launch is intentionally duplicated today
- future cleanup should preserve one canonical primary run path and treat footer/dock/palette access as shortcuts

Interaction posture:

- routine: start optimization, cancel current run
- advanced: route preview, GPU status/speedup inspection, catalog inspection
- high-stakes: none

## Validation

Capabilities:

- start validation
- cross-dataset selection
- validation table/report generation
- validation shortlist/report export
- open reports directory

Primary frontend path:

- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
- [validationService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\validationService.ts)

Secondary or duplicate entry points:

- [ValidationContextDock.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationContextDock.vue)
- [AnalysisContextDock.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AnalysisContextDock.vue)
- [OptimizeWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\OptimizeWorkspacePanel.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue) duplicates some folder-opening affordances

Current consolidation note:

- validation launch/report actions remain canonical in `ValidationWorkspacePanel.vue`
- `ValidationContextDock.vue` now acts as a context + navigation shortcut into the validation step instead of duplicating the full validation action set
- command-palette and dock access remain shortcuts, not competing primary surfaces

Interaction posture:

- routine: start validation, review validation table
- advanced: shortlist export, report generation, cross-dataset selection
- high-stakes: none

## Analysis

Capabilities:

- overview review
- Top shortlist filtering/sorting/selection
- Top export
- Pareto refresh/export
- Heatmap refresh/export
- drilldown and local stability analysis
- workflow-run refresh into analysis views

Primary frontend path:

- [OverviewWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\OverviewWorkspacePanel.vue)
- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ParetoWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ParetoWorkspacePanel.vue)
- [DrillWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DrillWorkspacePanel.vue)
- [HeatmapWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HeatmapWorkspacePanel.vue)
- [analysisService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\analysisService.ts)

Secondary or duplicate entry points:

- [AnalysisContextDock.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AnalysisContextDock.vue)
- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)

Current consolidation note:

- export and navigation shortcuts are duplicated across overview, docks, and palette
- those are likely safe to rationalize into one primary analysis workflow plus optional shortcuts

Interaction posture:

- routine: shortlist review, chart navigation, drilldown
- advanced: exports, local stability analysis
- high-stakes: none

## Artifacts / History

Capabilities:

- save run artifact
- list saved run artifacts
- load saved run artifact
- apply saved run
- rerun saved run
- update artifact tags
- delete saved run artifact
- reveal run path
- open report
- open runs directory

Primary frontend path:

- [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
- [historyService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\historyService.ts)

Secondary or duplicate entry points:

- [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue) exposes overlapping artifact CRUD/review patterns

Current consolidation note:

- History and Meta both manage artifacts in similar ways
- a shared artifact-browser abstraction is the strongest current consolidation candidate

Interaction posture:

- routine: list, load, reveal, open report
- advanced: rerun, tag editing, artifact deletion
- high-stakes: delete saved run artifact

## Execution

Capabilities:

- Alpaca/account connection test
- manual signal execution
- Polygon live stream start/stop/status
- live runner start/stop/status
- execution snapshot/order refresh
- stored-key persistence / clearing

Primary frontend path:

- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)
- [executionService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\executionService.ts)

Secondary or duplicate entry points:

- none significant today

Current consolidation note:

- this surface is already relatively concentrated
- focus should stay on safer grouping and clearer affordances, not on removing controls

Interaction posture:

- routine: snapshot/order refresh, account test
- advanced: stored-key persistence, stream control
- high-stakes: manual signal execution, live runner start/stop

## Settings / Runtime

Capabilities:

- window-effect changes
- runs/reports folder utilities
- market-data-root management
- runtime-oriented navigation / diagnostics helpers

Primary frontend path:

- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [settingsService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\settingsService.ts)

Secondary or duplicate entry points:

- Data duplicates market-data-root controls
- History / Validation / Meta duplicate some folder-opening affordances

Current consolidation note:

- Settings should remain the canonical home for environment/runtime utilities unless a workflow-specific control is essential in-context

Interaction posture:

- routine: folder utilities
- advanced: market-data-root management, window effects
- high-stakes: none

## ML Signal Branch

Capabilities:

- ML dataset export
- score-cache generation
- model-bundle and model-status inspection
- ML validation
- ML policy sweep

Primary frontend path:

- [MlSignalWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MlSignalWorkspacePanel.vue)
- [mlSignalService.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\mlSignalService.ts)

Secondary or duplicate entry points:

- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue) aliases some of the primary CTA behavior
- History rerun path can also reactivate ML-driven workflows

Current consolidation note:

- ML-specific actions should keep one canonical panel, with footer access treated as a contextual shortcut

Interaction posture:

- routine: model-bundle/status inspection
- advanced: dataset export, score-cache generation, validation, policy sweep
- high-stakes: none

## Current Consolidation Priorities

The most likely safe consolidation targets today are:

1. one canonical optimization launcher path per workflow, with footer/dock/palette treated as shortcuts
2. one shared artifact-management abstraction reused by History and Meta
3. any later reports-folder or runtime-utility duplication between workflow panels and Settings

## Implemented Consolidations

These consolidations are intentional and should not be mistaken for lost frontend capability:

1. Market-data-root management is centralized in `SettingsWorkspacePanel.vue`.
   - `DataWorkspacePanel.vue` still exposes the current folder and a direct open-folder path.
   - Backend-backed market-data-root functionality remains exposed through Settings.

2. Validation dock actions are consolidated into `ValidationWorkspacePanel.vue`.
   - `ValidationContextDock.vue` now serves as context plus navigation.
   - Backend-backed validation run/export/report functionality remains exposed in the primary validation panel.

## Regression Rule

Any future UX cleanup should preserve:

- at least one clear frontend path for every capability listed above
- clear discoverability for high-stakes capabilities
- documentation of any removed duplicate entry point as an intentional consolidation
