# Vue + Vite Phase 9B Checklist: Legacy Template Decommission

Last updated: 2026-02-21 (US date)
Parent: `VUE_VITE_MIGRATION_MASTER_CHECKLIST.md` (Phase 9)

## Goal

Systematically decommission unused hidden compatibility markup in `web/src/legacy/template.html` and reduce coupled legacy runtime code in `web/src/legacy/app.js` until removal is feasible.

## Scope

- In scope:
- hidden compatibility IDs and sections in `web/src/legacy/template.html`
- legacy runtime references tied only to removed compatibility controls
- related bridge observer cleanup
- related CSS cleanup in `web/src/styles/app.css`
- Out of scope:
- removal of required Vue shell Teleport/runtime mount surfaces
- backend command changes

## Non-Negotiables

- [x] No behavior regressions in migrated Vue surfaces.
- [x] No required compatibility ID removed without bridge/runtime replacement.
- [x] No hidden button-driven action path remains as a required control surface.

## Inventory + Classification

- [x] Generate ID inventory for `web/src/legacy/template.html`.
- [x] Classify each ID into:
- `required runtime host`
- `required bridge source`
- `decommission candidate`
- [x] Track every candidate with dependency notes (which bridge/runtime code references it).

## Decommission Order (Repeat by Slice)

- [x] Pick one bounded domain slice (Data, Optimize/Validate, Analysis/Meta, History, Execution, Distributed, Settings).
- [x] Replace bridge capability derivation from hidden button disabled-state with runtime signals (`data-*`, `running`, store state) where possible.
- [x] Remove corresponding hidden controls/sections from template.
- [x] Remove dead runtime listeners/handlers from `legacy/app.js`.
- [x] Remove dead bridge observers/constants tied to removed IDs.
- [x] Remove dead CSS selectors tied to removed IDs.
- [x] Update parity tests/assertions for removed IDs.

## Domain Tracks

### Data (in progress/completed baseline)
- [x] Confirm Data hidden control removal is fully stabilized (no fallback dependency).
- [x] Remove any remaining Data-only dead references in runtime/bridges/styles.

### Optimize + Validation
- [x] Remove hidden action controls once runtime-state capability derivation is in place.
- [x] Remove corresponding legacy listener setup and observer coupling.

### Analysis + Meta
- [x] Remove hidden action controls once bridge/state parity is signal-based.
- [x] Remove dead runtime branches tied to removed controls.

### History
- [x] Remove hidden action controls once runtime-state gating is independent of hidden buttons.
- [x] Remove associated legacy listener/observer baggage.

### Execution + Distributed
- [x] Remove hidden action controls once bridge capability derives from runtime/store state.
- [x] Remove dead runtime listener and disabled-state coupling paths.

### Settings/Utilities
- [x] Remove residual hidden settings utility markup not needed for compatibility.
- [x] Remove stale template anchors and style rules.

## Runtime Shrink Targets

- [x] Record `web/src/legacy/app.js` line count each slice.
- [x] Ensure each template decommission step also removes runtime code, not just markup.
- [x] Maintain or improve `legacy-app` chunk gzip trend over time.

## Latest Slice Log (2026-02-21: Execution + Distributed)

- [x] Bridge capability derivation for execution/distributed moved from hidden button disabled-state to runtime attributes (`data-running`, `data-exec-busy`).
- [x] Removed hidden template action IDs for execution/distributed (`btnExec*`, `btnDist*`) from `web/src/legacy/template.html`.
- [x] Removed dead hidden listener/disabled-state coupling from `web/src/legacy/app.js` for those removed IDs.
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed execution/distributed IDs remain absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` gzip improved to ~`134.16 kB`.
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18517`).

## Latest Slice Log (2026-02-21: Optimize + Validation)

- [x] Bridge capability derivation for optimize/validation moved from hidden button disabled-state to runtime attributes (`data-opt-can-*`, `data-val-can-*`, `data-running`).
- [x] Removed hidden template action IDs for optimize/validation (`btnApplyPreset`, `btnPresetKernelSmaAlma`, `btnRefreshGpuStatus`, `btnGpuParityCheck`, `btnRun`, `btnCancel`, `btnValidate`, `btnCrossRefresh`, `btnCrossSelectAll`, `btnCrossSelectNone`, `btnCrossSelectMatch`, `btnExportValidationCsv`, `btnExportValidationJson`, `btnExportReportJson`, `btnExportReportXlsx`).
- [x] Removed dead hidden listener/disabled-state coupling from `web/src/legacy/app.js` for those removed IDs.
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed optimize/validation IDs remain absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` gzip improved to ~`133.69 kB`.
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18455`).

## Latest Slice Log (2026-02-21: History)

- [x] Bridge capability derivation for History moved from hidden button disabled-state to runtime attributes (`data-hist-can-*`, `data-running`).
- [x] Removed hidden template action IDs for History (`btnApplyHistory`, `btnExportPreset`, `btnImportPreset`, `btnClearHistory`, `btnSaveSavedRunTags`, `btnRefreshSavedRuns`, `btnLoadSavedRun`, `btnApplySavedRun`, `btnRerunSavedRun`, `btnOpenSavedRunReport`, `btnRevealSavedRun`, `btnDeleteSavedRun`).
- [x] Removed dead hidden listener/disabled-state coupling from `web/src/legacy/app.js` for those removed IDs.
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed History IDs remain absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` gzip improved to ~`133.57 kB`.
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18417`).

## Latest Slice Log (2026-02-21: Analysis + Meta)

- [x] Bridge capability derivation for Analysis/Meta moved from hidden button disabled-state to runtime attributes (`data-ana-can-*`, `data-meta-can-*`, `data-running`).
- [x] Removed hidden template action IDs for Analysis/Meta (`btnCopyBest`, `btnExportCsv`, `btnExportJson`, `btnSetBaseline`, `btnClearBaseline`, `btnClearTopFilters`, `btnHeatmapExportPng`, `btnRunBatch`, `btnBatchStop`, `btnBatchRefresh`, `btnBatchSelectAll`, `btnBatchSelectNone`, `btnBatchRetryFailed`, `btnBatchRevealSummary`, `btnRunMeta`, `btnOpenMetaReport`, `btnRevealMetaFiles`, `btnMetaMlRefresh`, `btnMetaMlTrain`, `btnMetaMlApply`, `btnMetaMlDelete`, `btnRunMetaStage4`, `btnRevealMetaStage4`, `btnSaveMetaSavedTags`, `btnRefreshMetaSaved`, `btnLoadMetaSaved`, `btnOpenMetaSavedReport`, `btnRevealMetaSaved`, `btnDeleteMetaSaved`, `btnSaveStage4SavedTags`, `btnRefreshStage4Saved`, `btnLoadStage4Saved`, `btnRevealStage4Saved`, `btnDeleteStage4Saved`).
- [x] Removed dead hidden listener/disabled-state coupling from `web/src/legacy/app.js` for those removed IDs.
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed Analysis/Meta IDs remain absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` gzip improved to ~`133.56 kB`.
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18319`).

## Latest Slice Log (2026-02-21: Overview Empty-State Controls)

- [x] Removed hidden template action IDs for Overview empty-state controls (`btnOverviewEmptyPrimary`, `btnOverviewEmptySecondary`).
- [x] Replaced bridge/state derivation for Overview empty-state action labels/actions/enabled state with runtime attributes (`data-overview-primary-*`, `data-overview-secondary-*`) instead of hidden button reads.
- [x] Removed dead hidden listener wiring in `web/src/legacy/app.js` (`initOverviewEmptyUi` and overview button handlers).
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed Overview IDs remain absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` gzip improved to ~`133.48 kB`.
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18277`).

## Latest Slice Log (2026-02-21: Advanced MO Reset Control)

- [x] Removed hidden template action ID for Advanced Composite-MO reset (`btnMoResetSearchSpace`) from `web/src/legacy/template.html`.
- [x] Replaced UI capability coupling with runtime-owned flag (`data-adv-can-reset-mo-search-space`) emitted from `web/src/legacy/app.js` and consumed by `advancedContextBridge` + `advancedStore` + `AdvancedWorkspacePanel.vue`.
- [x] Removed dead hidden listener/disabled-state coupling from `web/src/legacy/app.js`; Advanced reset now runs through direct runtime handler (`handleAdvancedResetMoSearchSpaceAction`) and runtime export wrapper.
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed Advanced MO reset ID remains absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.51 kB` (gzip ~`133.60 kB`), `legacy-template` ~`159.93 kB` (gzip ~`43.84 kB`).
- [x] Runtime line-count for this exact slice was not captured before shutdown; latest subsequent snapshot is recorded in the next slice.

## Latest Slice Log (2026-02-21: Polygon Modal Action-ID Decommission)

- [x] Removed remaining Polygon modal `btn*` IDs from `web/src/legacy/template.html` (`btnPolygonClose`, `btnPolygonSaveKey`, `btnPolygonClearKey`, `btnPolygonDownloadLoad`), replacing them with semantic `data-polygon-*` action selectors.
- [x] Removed dead legacy runtime coupling to Polygon action IDs in `web/src/legacy/app.js` (`setRunning` disable list + listener registration), replacing it with selector-based helpers (`polygonModalActionButton`, `polygonActionButton`, `setPolygonModalActionButtonsDisabled`).
- [x] Removed stale legacy `btnPolygonOpen` compatibility branch from `initPolygonUi`.
- [x] Updated `web/src/legacy/uiSurfaceParity.test.ts` to assert removed Polygon modal action IDs remain absent.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.73 kB` (gzip ~`133.59 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18299`).

## Latest Slice Log (2026-02-21: Data Dead-Control Cleanup)

- [x] Removed final stale hidden Data-control coupling in runtime (`btnApplyTimeframe` path) from `updateDataTimeframeUi` in `web/src/legacy/app.js`.
- [x] Verified literal ID coupling hygiene: all `$(...)` literal IDs referenced by `web/src/legacy/app.js` now exist in `web/src/legacy/template.html` (no dangling hidden-control IDs).
- [x] Removed unused compatibility listener scaffold from `initLegacyApp` (`addHiddenClickListener`) after earlier hidden-button decommission.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.71 kB` (gzip ~`133.58 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18291`).

## Latest Slice Log (2026-02-21: Polygon Escape Listener Decommission)

- [x] Removed legacy document-level Escape listener from `web/src/legacy/app.js` (`initLegacyPrimaryListeners`), eliminating remaining global listener coupling in legacy runtime.
- [x] Re-homed Escape-close behavior for Polygon modal to Vue shortcut owner (`GlobalShortcutBridge`) via new data runtime close wrapper (`invokeLegacyDataClosePolygonRuntime`) and `dataStore.closePolygon`.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.82 kB` (gzip ~`133.55 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18288`).

## Latest Slice Log (2026-02-21: Polygon Modal Runtime Listener Cleanup)

- [x] Removed dead Vue-owner fallback gating branch in `web/src/legacy/app.js` for Polygon modal action listeners (`bindHiddenClickListeners`), keeping only active listener registration paths.
- [x] Preserved compatibility-template shape while simplifying runtime ownership (no new hidden control dependencies introduced).
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.79 kB` (gzip ~`133.54 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.98 kB` (gzip ~`43.85 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18278`).

## Latest Slice Log (2026-02-21: Toast Host Template Decommission)

- [x] Removed static hidden compatibility toast host markup (`id="toastHost"`) from `web/src/legacy/template.html`; toast runtime fallback in `web/src/legacy/core/toasts.js` now remains the single host-creation path.
- [x] Updated parity assertion in `web/src/legacy/uiSurfaceParity.test.ts` to enforce continued absence of `toastHost` in compatibility template.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.79 kB` (gzip ~`133.54 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.85 kB` (gzip ~`43.80 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18278`).

## Latest Slice Log (2026-02-21: Stale Anchor ID Decommission)

- [x] Removed stale compatibility-only wrapper IDs from `web/src/legacy/template.html` while preserving structure/behavior: `runFooter`, `stabilityTable`, `tradeTable`, `execOrdersTable`.
- [x] Updated parity assertion coverage in `web/src/legacy/uiSurfaceParity.test.ts` to enforce continued absence of removed stale IDs.
- [x] Re-ran exact ID-reference audit across `web/src`: zero exact-unreferenced template IDs remain after this slice.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`511.79 kB` (gzip ~`133.54 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.79 kB` (gzip ~`43.79 kB`), `assets/index-*.css` ~`37.43 kB` (gzip ~`7.97 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`18278`).

## Latest Slice Log (2026-02-21: Settings Utility + Stale Style Cleanup)

- [x] Removed stale settings/decommissioned-ID bridge dependencies in `analysisHistorySettingsBridge` (`themeSelect`, `effectSelect`) and in `dataContextBridge` (`workspaceCtxStatus`), replacing them with runtime-root dataset/status ownership.
- [x] Removed dead CSS selector blocks targeting deleted compatibility IDs from `web/src/styles/app.css` (`#effectSelect`, `#btnToggleWorkflowAdvanced`, `#controlsTabBar`, `#resultsTabBar`).
- [x] Verified style-surface parity: only non-template ID selectors left in CSS are Vue-hosted heatmap canvas/overlay IDs (`#heatmapCanvasVue`, `#heatmapOverlayVue`).
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`).
- [x] Build/size snapshot recorded in parent checklist; `legacy-app` ~`514.21 kB` (gzip ~`134.17 kB`), `legacy-core` ~`23.62 kB` (gzip ~`8.07 kB`), `legacy-template` ~`159.79 kB` (gzip ~`43.79 kB`), `assets/index-*.css` ~`35.34 kB` (gzip ~`7.70 kB`).
- [x] `web/src/legacy/app.js` line-count snapshot recorded in parent checklist (`16475`).

## Validation Gates (Required Per Slice)

- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run build`
- [x] Update `web/src/legacy/uiSurfaceParity.test.ts` as necessary.
- [x] Record build-size + line-count snapshot in parent checklist.

## Exit Criteria (Phase 9B Done)

- [x] Hidden compatibility template reduced to minimal required host/signal surfaces.
- [x] No removable, unneeded legacy template sections remain.
- [x] `legacy/app.js` reduced with matching removal of dead listener/observer/runtime branches.
- [x] Parent Phase 9 decommission item can be checked complete.
