# Vue + Vite Phase 9A Checklist: Legacy Runtime Ownership Cutover

> Status: Superseded by `VUE_VITE_100_PERCENT_CUTOVER_CHECKLIST.md` for final cutover tracking.

Last updated: 2026-02-21 (US date)
Parent: `VUE_VITE_MIGRATION_MASTER_CHECKLIST.md` (Phase 9)

## Goal

Move remaining runtime ownership out of `web/src/legacy/app.js` and into Vue/Pinia services/composables so Vue becomes the primary runtime owner (not just renderer).

## Scope

- In scope:
- `web/src/legacy/app.js` bootstrap ownership
- workflow/tab state-machine ownership still in legacy runtime
- remaining keyboard shortcut/runtime listener ownership in legacy runtime
- remaining cross-domain lifecycle coupling currently rooted in legacy init flow
- Out of scope:
- Rust/Tauri command contract changes
- backend algorithm changes (Rust/CUDA remains unchanged)
- visual redesign work

## Non-Negotiables

- [x] No feature removals.
- [x] No command/event/storage contract regressions.
- [x] Performance remains neutral or better (frontend stays lightweight).
- [x] CPU-only path remains fully functional.

## Current Runtime Ownership Inventory

- [x] Enumerate all remaining top-level runtime owners in `web/src/legacy/app.js` (bootstrap, state-machine transitions, lifecycle orchestration).
- [x] Map each owner to a target Vue/Pinia home (`workflowStore`, `settingsStore`, bridge service, or new composable).
- [x] Mark each owner as `migrate`, `wrap`, or `retain temporarily` with rationale.

## Remaining Runtime Ownership Map (2026-02-21)

- `legacy init orchestration` (`initLegacyApp` + `lateInitLegacyRuntime`): `retain temporarily` (compatibility bootstrap still required for hidden runtime surfaces); target `migrate` into Vue-side startup services once domain controllers are cut over.
- `workflow runtime sync + snapshot emission` (`syncWorkflowRuntimeFromDom`, `setControlsTabRuntime`, `setResultsTabRuntime`, `scheduleWorkflowSnapshotEmit`): `wrap` (Vue owns visible nav, legacy still sync adapter for compatibility state/events); target `workflowStore` + workflow bridge service finalization.
- `data/polygon compatibility controllers` (CSV load, market-data-dir, polygon cache/download modal handlers): `retain temporarily` (still primary command orchestrators invoked by Vue stores); target split into `dataStore` + data service composables.
- `optimize/validation orchestration` (run/cancel/validate lifecycle, readiness/progress rendering): `retain temporarily` (still command orchestration owner); target split into `optimizeStore` + `validationStore` runtime services.
- `analysis/meta rendering controllers` (top/drill/heatmap/meta batch lifecycle + compatibility snapshots): `retain temporarily` (still computes/feeds several legacy-rendered surfaces); target staged migration into `analysisStore` service adapters.
- `execution/distributed controllers` (live runner, worker control, status output orchestration): `retain temporarily` (still command-lifecycle owner); target staged split into `executionStore` + `distributedStore` services.
- `settings/history persistence adapters` (settings serialization, history artifact/saved-run loaders): `wrap` (Vue store owns UI state, legacy remains adapter for persisted compatibility formats); target progressive extraction into typed store persistence modules.

## Workstream A: Bootstrap Ownership

- [x] Split residual monolithic legacy bootstrap into explicit phases with boundaries:
- `runtime init`
- `domain init`
- `event subscription`
- `ui sync`
- [x] Move persisted startup restoration fully into Vue-side services where possible.
- [x] Ensure legacy init runs in compatibility mode only (no primary ownership for Vue-controlled flows).
- [x] Remove dead legacy bootstrap branches after each migration slice.

## Workstream B: Workflow/Tab State Machine

- [x] Confirm all workspace/workflow/controls/results/aggregate transitions are store-owned first, legacy-second.
- [x] Remove remaining legacy-side transition logic that mutates state for Vue-owned selections.
- [x] Remove legacy fallback transition paths no longer reachable in Vue-owner mode.
- [x] Keep event snapshot emission for bridge sync, but ensure it reflects Vue-owned truth.

## Workstream C: Keyboard + Runtime Input Ownership

- [x] Audit remaining global/document listeners registered by legacy runtime.
- [x] Move qualifying listeners to Vue bridges/services if still needed.
- [x] Keep only strictly compatibility-critical listeners in legacy runtime.
- [x] Remove legacy listener setup that no longer has template/runtime consumers.

## Workstream D: Domain Runtime Controllers

- [x] Identify legacy runtime handlers still acting as primary orchestrators for:
- data
- optimize/validate
- analysis/meta
- execution/distributed
- history/settings
- [x] Re-home orchestration into stores/services where safe.
- [x] Convert legacy handlers to thin compatibility adapters or remove when unused.

## Workstream E: Bridge Contract Tightening

- [x] Ensure all bridges read runtime-owned attributes/events rather than hidden control disabled-states where possible.
- [x] Eliminate brittle bridge dependencies on hidden DOM behavior when equivalent runtime signals exist.
- [x] Keep bridge snapshots deterministic and low-churn (coalesced sync only).

## Latest Slice Log (2026-02-21: Execution + Distributed Runtime Ownership)

- [x] `executionDistributedBridge` capability flags (`can*`) moved from hidden button disabled-state reads to runtime attributes (`data-running`, `data-exec-busy`).
- [x] Added explicit execution busy runtime signal emission in legacy runtime (`document.documentElement.dataset.execBusy` via `setExecBusy(...)`).
- [x] Removed legacy hidden execution/distributed button listener registration paths; runtime handlers are now called via direct Vue store runtime wrappers only.
- [x] Bridge observers remain coalesced and deterministic; button mutation observers were removed for execution/distributed slices.

## Latest Slice Log (2026-02-21: Optimize + Validation Runtime Ownership)

- [x] `optimizeValidationBridge` capability flags (`runDisabled`, `validateDisabled`, `can*`) moved from hidden button disabled-state reads to runtime-owned attributes (`data-opt-can-*`, `data-val-can-*`, `data-running`).
- [x] Added explicit optimize/validation runtime signal emission in legacy runtime (`setRuntimeFlag(...)` updates in `updateRunReadinessUi`, `updateValidateTabUi`, `updateStrategyUi`, `updateValidationExportButtons`, and `updateCrossMeta`).
- [x] Removed legacy hidden optimize/validation button listener registration paths; runtime handlers are now called via direct Vue store runtime wrappers only.
- [x] Bridge observers remain coalesced and deterministic; optimize/validation button mutation observers and button click listeners were removed in this slice.

## Latest Slice Log (2026-02-21: History Runtime Ownership)

- [x] History bridge capability flags moved from hidden button disabled-state reads to runtime-owned attributes (`data-hist-can-*`, `data-running`).
- [x] Added explicit History runtime signal emission in legacy runtime (`setRuntimeFlag(...)` updates in `syncSavedRunSelectionUi`).
- [x] Removed legacy hidden History button listener registration paths; runtime handlers are now called via direct Vue store runtime wrappers only.
- [x] Bridge observers remain coalesced and deterministic; History button mutation observers were removed in this slice.

## Latest Slice Log (2026-02-21: Analysis + Meta Runtime Ownership)

- [x] `analysisHistorySettingsBridge` Analysis/Meta capability flags moved from hidden button disabled-state reads to runtime-owned attributes (`data-ana-can-*`, `data-meta-can-*`, `data-running`).
- [x] Added explicit Analysis/Meta runtime signal emission in legacy runtime (`setRuntimeFlag(...)` updates in `setRunning`, `renderMetaSelection`, `updateMetaMlButtons`, `updateMetaStage4Buttons`, `syncBatchSummaryButtons`, `syncMetaSavedSelectionUi`, `syncStage4SavedSelectionUi`, and batch-stop lifecycle flow).
- [x] Removed legacy hidden Analysis/Meta button listener registration paths; runtime handlers are now called via direct Vue store runtime wrappers only.
- [x] Bridge observers remain coalesced and deterministic; Analysis/Meta button mutation observers were removed in this slice.

## Latest Slice Log (2026-02-21: Overview Empty-State Runtime Ownership)

- [x] Overview empty-state action metadata moved from hidden button DOM state to runtime-owned attributes (`data-overview-primary-*`, `data-overview-secondary-*`) in legacy runtime.
- [x] Added explicit Overview action runtime emission via `setOverviewActionState(...)` from `updateOverviewEmptyUi`.
- [x] Removed legacy hidden Overview button listener registration paths (`initOverviewEmptyUi` and related handlers).
- [x] Bridge observers remain coalesced and deterministic; Overview button observers were removed in this slice.

## Latest Slice Log (2026-02-21: Advanced MO Reset Runtime Ownership)

- [x] Advanced Composite-MO reset capability moved from hidden button disabled-state to runtime-owned flag (`data-adv-can-reset-mo-search-space`) emitted from `updateMoRangeBuilderUi` in legacy runtime.
- [x] Added explicit Advanced runtime reset handler (`handleAdvancedResetMoSearchSpaceAction`) and direct runtime export (`invokeLegacyAdvancedResetMoSearchSpaceRuntime`) used by `advancedStore`.
- [x] `advancedContextBridge` snapshot ownership now reads runtime flag (`advCanResetMoSearchSpace`) instead of hidden control state.
- [x] Removed legacy hidden Advanced reset button listener/disable ownership path tied to `btnMoResetSearchSpace`.

## Latest Slice Log (2026-02-21: Polygon Modal Action Selector Ownership)

- [x] Legacy Polygon modal action listener ownership moved from hardcoded hidden button IDs to runtime semantic selectors (`data-polygon-modal-action`, `data-polygon-action`).
- [x] Removed stale `btnPolygonOpen` compatibility styling branch from `initPolygonUi`, reducing legacy runtime dependence on removed hidden controls.
- [x] Run-state disable behavior for Polygon modal actions is now selector-driven (`setPolygonModalActionButtonsDisabled`) instead of ID-list ownership in `setRunning`.

## Latest Slice Log (2026-02-21: Legacy Listener Audit + Cleanup)

- [x] Audited remaining global/document listener ownership in `web/src/legacy/app.js`: only one document-level `keydown` listener remains (Escape-to-close for Polygon modal compatibility).
- [x] Removed dead listener scaffolding from `initLegacyApp` (`addHiddenClickListener`) that no longer had any runtime consumers.
- [x] Removed stale hidden-control ownership branch from Data timeframe UI (`btnApplyTimeframe`), tightening runtime-to-template contract and reducing no-op compatibility paths.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`) with updated build + line-count snapshots recorded in parent checklist.

## Latest Slice Log (2026-02-21: Polygon Escape Ownership Cutover)

- [x] Removed the last legacy global/document listener from `web/src/legacy/app.js` (document-level Escape handler in `initLegacyPrimaryListeners`).
- [x] Added explicit data-domain runtime close action export (`invokeLegacyDataClosePolygonRuntime`) and corresponding action/store wrapper path for Vue ownership.
- [x] Moved Escape-key Polygon modal close behavior into Vue `GlobalShortcutBridge` (capture-phase shortcut owner), keeping modal close UX while consolidating keyboard ownership in Vue.
- [x] Listener audit outcome: `web/src/legacy/app.js` now has no `document.addEventListener` / `window.addEventListener` registrations.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`) with updated build + line-count snapshots recorded in parent checklist.

## Latest Slice Log (2026-02-21: Polygon Modal Listener Ownership Stabilization)

- [x] Removed Vue-owner fallback gating from Polygon modal action listeners in `initLegacyPrimaryListeners` (`save-key`, `clear-key`, `download-load`, modal close/backdrop close), so modal actions remain functional under Vue runtime ownership.
- [x] Eliminated remaining dead hidden-listener scaffolding from this path (`bindHiddenClickListeners` branch), further reducing legacy runtime branch complexity.
- [x] Listener audit remains clean: no global `document/window` listeners in `web/src/legacy/app.js`.
- [x] Validation gates passed for this slice (`typecheck`, `test`, `lint`, `build`) with updated build + line-count snapshots recorded in parent checklist.

## Latest Slice Log (2026-02-21: Quick-Actions Runtime Metadata Ownership)

- [x] Replaced legacy dataset write ownership for latest report/meta report paths in `web/src/legacy/app.js` (`updateQuickActionsUi`) with explicit runtime event emission (`vta:quick-actions-snapshot`).
- [x] Added Vue-side bridge ownership via `web/src/legacy/bridges/quickActionsBridge.ts` and integrated it into `LegacyRuntimeBridge` bootstrap so command metadata is store-owned (`commandActionStore`) from startup onward.
- [x] Command palette + global shortcut handlers now gate “open latest report/meta report” on store-owned availability flags (`canOpenLatestReport`, `canOpenLatestMetaReport`) instead of unconditional invoke attempts.
- [x] Added targeted runtime ownership coverage (`quickActionsBridge.test.ts`, `commandActionStore.test.ts`) and kept validation gates green (`typecheck`, `test`, `lint`, `build`) with parent checklist snapshots updated.

## Latest Slice Log (2026-02-21: Workflow + Bootstrap Final Cutover)

- [x] Workflow transition ownership tightened in `web/src/legacy/app.js`: `applyWorkflow(...)` now dispatches `workflow.select` to Vue first and returns on accepted dispatch, preventing legacy-side primary mutation during Vue-owner mode.
- [x] Vue-owner transition fallback mutation removed from `syncWorkflowToActiveTabs(...)`; in Vue-owner mode it now performs runtime sync-only (`syncWorkflowRuntimeFromDom` + snapshot emit), eliminating legacy-driven tab/workflow rewrites.
- [x] `workflowBridge` now performs coalesced, runtime-attribute-driven sync (`MutationObserver` on `data-workflow*`/`data-controls-tab`/`data-results-tab`/`data-aggregate-tab`) and no longer depends on removed DOM fallback nodes.
- [x] Legacy bootstrap explicitly phased in `LegacyRuntimeBridge.vue` (`runtime init` -> `domain init` -> `event subscription` -> `ui sync`) with persisted workflow/settings restoration applied via Vue services in the `ui sync` phase.
- [x] Settings/data bridge stale-ID dependencies removed (`themeSelect`, `effectSelect`, `workspaceCtxStatus`), and dead selectors targeting removed template IDs were removed from `web/src/styles/app.css`.
- [x] Validation + parity evidence for this cutover: `npm --prefix web run typecheck`, `npm --prefix web run test` (`29` files / `106` tests), `npm --prefix web run lint`, `npm --prefix web run build`, `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (PASS), and `cargo test --workspace` (PASS).

## Validation Gates (Required Per Slice)

- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run build`
- [x] Record build-size snapshot in parent checklist.
- [x] Record `web/src/legacy/app.js` line-count snapshot in parent checklist.

## Regression Smoke Matrix (Minimum)

- [x] Load CSV / apply timeframe / clear data.
- [x] Run / cancel optimization.
- [x] Run validation + exports.
- [x] Analysis top/pareto/drill/heatmap interactions.
- [x] Meta batch + ML + stage4 flows.
- [x] Execution live/runner controls.
- [x] Distributed worker + run/cancel flows.

## Exit Criteria (Phase 9A Done)

- [x] Legacy runtime no longer owns primary workflow/tab/bootstrap behavior.
- [x] Legacy runtime acts only as compatibility adapter for unavoidable remaining paths.
- [x] No remaining direct ownership blockers for deleting large parts of `legacy/app.js`.
- [x] All validation gates green with documented snapshots.
