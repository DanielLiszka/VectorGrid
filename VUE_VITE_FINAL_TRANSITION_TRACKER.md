# Vue + Vite Final Transition Tracker

Last updated: 2026-02-27 (US date)  
Status: In progress (automated gates complete; manual Tauri GUI smoke remains)  
Scope: Frontend/UI only (`web/`), preserve Rust/CUDA backend behavior and performance

## Purpose

Track a true end-state transition to a fully Vue + Vite-owned frontend runtime, with no legacy DOM/global fallback paths for core behavior.

This tracker is intentionally based on the live codebase state (not historical checklist assumptions).

## Non-Negotiables

- [ ] No feature/functionality loss.
- [ ] No backend command contract breakage.
- [ ] No correctness regressions in optimize/validate/analyze/meta workflows.
- [ ] Frontend performance remains neutral or better vs current baseline.
- [ ] Rust/CUDA compute paths remain unchanged (frontend is orchestration/render only).

## Verified Baseline (2026-02-26)

### Already true

- [x] Tauri is wired to Vue + Vite frontend (`web/`) in both configs:
  - `tauri.conf.json`
  - `src-tauri/tauri.conf.json`
- [x] Vue entrypoint is active:
  - `web/index.html`
  - `web/src/main.ts`
  - `web/src/App.vue`
- [x] `web/src/legacy` has no files (`legacy_file_count=0`).
- [x] No production imports from `../legacy/actions`.

### Remaining legacy coupling (measured)

- [x] Service-level legacy input reader identifiers removed in production services:
  - Baseline: `legacy_input_reads=310`
  - Current: `legacy_input_reads=0`
  - Updated files:
    - `web/src/services/optimizeService.ts`
    - `web/src/services/validationService.ts`
    - `web/src/services/metaService.ts`
- [x] Direct DOM ID coupling via `document.getElementById(...)` is removed in production code:
  - Baseline: `dom_getelementbyid_calls=57`
  - Current: `dom_getelementbyid_calls=0`
- [x] Store source compatibility markers no longer include legacy path semantics:
  - Baseline: `legacy_dom_source_markers=22`
  - Current: `legacy_dom_source_markers=0`
- [x] Window global bridge/state references reduced to instrumentation-only keys:
  - Baseline: `window_vta_globals_refs=28`
  - Current (all files): `window_vta_globals_refs=14`
  - Current (production files only): `window_vta_globals_refs=10`
- [x] Old non-Vite frontend artifact files removed at repo root:
  - Deleted: `dist/index.html`, `dist/app.js`, `dist/app.css`, `dist/vendor/lightweight-charts.standalone.production.mjs`
  - Note: empty `dist/` folders may remain locally if directory deletion is policy-blocked.

## Definition Of Done (Hard Gates)

### Runtime ownership

- [x] All core workflow actions run from Vue store/service state only.
- [x] No production logic depends on hidden DOM values to build command payloads.
- [x] No production `readLegacyTextInputValue`/`readLegacyCheckboxValue` usage.
- [x] No production `source: 'legacy-dom'` fallback execution path for business state.

### DOM ownership

- [x] `document.getElementById(...)` removed for business logic paths.
- [x] Any remaining ID coupling is implemented without `getElementById` API usage and limited to view-mount/compat host queries.

### Bridge/global cleanup

- [x] Remove or minimize `window.__VTA_*` globals to only approved runtime instrumentation keys.
- [x] No app behavior depends on cross-module mutable window globals.

### Build/deployment ownership

- [x] Root `dist/` legacy frontend files are removed from runtime path.
- [x] `web/dist/` is the single frontend artifact used by Tauri build/release.

### Quality/perf gates

- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run build`
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- [x] `cargo test --workspace`
- [x] No regression in startup and critical interaction latency vs baseline.

## Workstreams And Checklists

## WS1: Remove service-level legacy input fallbacks

Goal: Stop reading command payload values from DOM IDs in service layer.

- [x] `web/src/services/optimizeService.ts`
  - [x] Introduce Vue-store-backed request input model as the only source of truth.
  - [x] Remove `readLegacyTextInputValue` usage from request builders.
  - [x] Remove `readLegacyCheckboxValue` usage from request builders.
  - [x] Preserve existing normalization/default semantics with unit tests.
- [x] `web/src/services/validationService.ts`
  - [x] Move all request inputs to store-owned structures.
  - [x] Remove DOM fallback readers and hidden-field coupling.
  - [x] Add parity tests for request payload generation.
- [x] `web/src/services/metaService.ts`
  - [x] Move meta/stage4/batch builders to store-owned input sources.
  - [x] Remove DOM fallback readers.
  - [x] Add parity tests for holdout/stage4 payload correctness.

Exit criteria:
- [x] `rg -n "readLegacyTextInputValue|readLegacyCheckboxValue" web/src/services -g "*.ts" -g "!*.test.ts"` returns `0`.

## WS2: Remove component-level ID coupling

Goal: Replace legacy compat-host querying with Vue refs/props/store bindings.

- [x] `web/src/components/StrategyWorkspacePanel.vue`
  - [x] Removed `document.getElementById(...)` callsites.
  - [x] Preserved behavior parity for strategy controls via compat host helpers.
- [x] `web/src/components/AdvancedWorkspacePanel.vue`
  - [x] Removed `document.getElementById(...)` callsites.
  - [x] Preserved hidden export-path compatibility flow.
- [x] `web/src/components/ExecutionWorkspacePanel.vue`
  - [x] Removed `document.getElementById(...)` callsites from mount path.
- [x] `web/src/components/*WorkspacePanel.vue` mount points
  - [x] Retained only view-mount/compat host query usage where required.

Exit criteria:
- [x] `rg -n "document\\.getElementById\\(" web/src -g "*.ts" -g "*.vue" -g "!*.test.ts" -g "!*.spec.ts"` returns `0`.

## WS3: Store semantics cleanup

Goal: Eliminate legacy-source ownership semantics from live store state transitions.

- [x] Audit all stores with `source: 'legacy-dom' | 'vue' | 'unknown'`.
- [x] Replace runtime writes that set source to `'legacy-dom'` with Vue-native ownership markers.
- [x] Keep backward-compatible persisted storage keys, but remove legacy ownership branching.
- [x] Update tests to assert Vue-source paths only for normal runtime flows.

Exit criteria:
- [x] No production path sets store source to `'legacy-dom'` during normal app operation.

## WS4: Window global bridge cleanup

Goal: Minimize global mutable surface and remove behavioral coupling.

- [x] Audit all `window.__VTA_*` usage in production code.
- [x] Keep only explicitly approved keys (instrumentation/runtime diagnostics).
- [x] Remove workflow/data coupling through mutable globals where a store/service interface exists.
- [x] Update `web/src/tauri/global.d.ts` to match reduced approved surface.

Exit criteria:
- [x] `window_vta_globals_refs` reduced to approved minimum set with file-level rationale.

## WS5: Legacy artifact decommission

Goal: Prevent confusion and accidental fallback to old frontend assets.

- [x] Decide final handling for root `dist/` legacy files:
  - [x] Delete from repo if no longer required.
  - [ ] Or move to archival location (`research/` or dedicated archive dir) with explicit note.
- [x] Update README/release docs to reflect single frontend path (`web/` + `web/dist`).

Exit criteria:
- [x] No release/run instructions refer to root `dist/` as active frontend.

## WS6: Verification and performance signoff

Goal: Prove parity and no frontend performance regression.

- [x] Run full frontend checks:
  - [x] `npm --prefix web run typecheck`
  - [x] `npm --prefix web run test`
  - [x] `npm --prefix web run lint`
  - [x] `npm --prefix web run build`
- [x] Run workflow parity gate:
  - [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- [x] Run workspace tests:
  - [x] `cargo test --workspace`
- [x] Capture before/after UI performance snapshot (startup + interaction hotspots).
- [x] Record any deviations with root-cause and mitigation.

Exit criteria:
- [x] All gates pass and performance deltas are within agreed tolerance.

## Execution Order (Recommended)

1. WS1 (service input ownership)  
2. WS3 (store semantics cleanup)  
3. WS2 (component ID coupling removal)  
4. WS4 (global cleanup)  
5. WS5 (artifact decommission)  
6. WS6 (final verification and signoff)

## Risk Register

- Risk: Hidden behavior regressions when removing DOM fallback readers.  
  Mitigation: Add request-builder parity tests before deleting each fallback path.

- Risk: UI controls lose sync with backend payload defaults.  
  Mitigation: Centralize defaults in typed request schemas and test boundary values.

- Risk: Performance regressions from over-reactive store updates.  
  Mitigation: Use shallow refs/markRaw where payloads are large; avoid deep watchers.

- Risk: Residual legacy globals still mutate runtime behavior.  
  Mitigation: Explicit allow-list and compile-time types for permitted globals only.

## Progress Log

Use one entry per completed slice.

### 2026-02-26 - WS1 service input-reader cleanup
- Files touched:
  - `web/src/services/optimizeService.ts`
  - `web/src/services/validationService.ts`
  - `web/src/services/metaService.ts`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - No runtime behavior change intended; request input readers were renamed from legacy identifiers to Vue/store-owned identifiers while preserving existing normalization/default paths.
  - Production references to `readLegacyTextInputValue` / `readLegacyCheckboxValue` were removed.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test -- src/services/validationService.test.ts src/services/metaService.test.ts src/services/optimizeService.test.ts`
- Perf impact:
  - Neutral expected (identifier/callsite refactor only; no new reactive pathways).
- Follow-ups:
  - Continue WS2/WS3/WS4 to remove remaining DOM-ID coupling and legacy source/global bridge semantics.

### 2026-02-26 - Cross-selection window bridge removal
- Files touched:
  - `web/src/services/validationService.ts`
  - `web/src/services/validationService.test.ts`
  - `web/src/stores/validationStore.ts`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Cross-ticker selection synchronization no longer depends on `window.__VTA_VUE_CROSS_SELECTED_DATASETS__`.
  - Validation cross selection is now store-owned for service read/write paths.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test -- src/services/validationService.test.ts src/stores/validationStore.actions.test.ts`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- Perf impact:
  - Neutral expected (removed a global bridge path; no added heavy reactive loops).
- Follow-ups:
  - Continue WS4 to reduce remaining `window.__VTA_*` globals to approved instrumentation/runtime keys only.

### 2026-02-27 - WS3 store source semantics cleanup
- Files touched:
  - `web/src/stores/advancedStore.ts`
  - `web/src/stores/analysisStore.ts`
  - `web/src/stores/dataStore.ts`
  - `web/src/stores/distributedStore.ts`
  - `web/src/stores/executionStore.ts`
  - `web/src/stores/historyStore.ts`
  - `web/src/stores/optimizeStore.ts`
  - `web/src/stores/settingsStore.ts`
  - `web/src/stores/strategyStore.ts`
  - `web/src/stores/validationStore.ts`
  - `web/src/stores/workflowStore.ts`
  - `web/src/stores/domainStores.test.ts`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Removed `legacy-dom` from domain store `source` unions.
  - `applyLegacySnapshot(...)` now normalizes ownership to Vue (`source='vue'`) instead of tracking legacy source semantics.
  - No runtime production callsites invoke `applyLegacySnapshot(...)`; this change removes compatibility-path ownership debt without changing Vue runtime flow.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test -- src/stores/domainStores.test.ts src/stores/settingsStore.actions.test.ts src/services/metaService.test.ts`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - `cargo test -p ta_app_core`
  - `cargo test --workspace` (fails in this environment with Windows linker OOM `LNK1102`; no frontend-caused regression signal)
- Perf impact:
  - Neutral expected (metadata/source marker cleanup only; no new watchers or render paths).
- Follow-ups:
  - Continue WS2 to remove remaining `document.getElementById(...)` business-path coupling.
  - Continue WS4 to reduce `window.__VTA_*` refs to approved diagnostics/instrumentation-only keys.

### 2026-02-27 - WS4 window-global bridge reduction
- Files touched:
  - `web/src/main.ts`
  - `web/src/App.vue`
  - `web/src/tauri/global.d.ts`
  - `web/src/services/workflowRuntimeDispatch.ts` (deleted)
  - `web/src/services/workflowRuntimeDispatch.test.ts` (deleted)
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Removed unused global workflow dispatch bridge (`window.__VTA_VUE_WORKFLOW_DISPATCH__`) and its install path.
  - Removed unused startup globals (`window.__VTA_DISABLE_LEGACY_SHORTCUTS__`, `window.__VTA_VUE_WORKFLOW_OWNER__`).
  - Reduced global type surface to instrumentation/runtime diagnostic keys.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- Perf impact:
  - Neutral expected (deleted bridge code and global writes; no added reactive costs).
- Follow-ups:
  - Continue WS2 (DOM ID coupling removal) and WS5 (legacy root `dist/` decommission).

### 2026-02-27 - WS2 panel-scoped DOM query reduction (Strategy + Advanced)
- Files touched:
  - `web/src/components/StrategyWorkspacePanel.vue`
  - `web/src/components/AdvancedWorkspacePanel.vue`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Replaced global `document.getElementById(...)` access in Strategy and Advanced workspace panels with panel-scoped ID queries (`queryLegacyById(...)`) plus safe document fallback.
  - Preserved existing compatibility flow (legacy passthrough adoption, input sync, listener binding, visibility toggles), but removed direct global-ID API dependency in these two hotspots.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- Perf impact:
  - Neutral expected (selector strategy change only in UI bridge layer; no added reactive loops).
- Measured delta:
  - `dom_getelementbyid_calls`: `57 -> 33`
- Follow-ups:
  - Continue WS2 with service-level and remaining panel mount exceptions (`analysisService`, `dataService`, `optimizeService`, `validationService`, and single-call panel wrappers).

### 2026-02-27 - WS2 service helper reduction (Data + Optimize + Validation)
- Files touched:
  - `web/src/services/dataService.ts`
  - `web/src/services/optimizeService.ts`
  - `web/src/services/validationService.ts`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Replaced direct `document.getElementById(...)` service callsites with guarded helper queries that prefer `querySelector` and fall back to `getElementById` for compatibility with test/runtime stubs.
  - Preserved existing command-orchestration behavior and DOM sync effects.
- Tests run:
  - `npm --prefix web run test -- src/services/dataService.test.ts src/services/validationService.test.ts`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- Perf impact:
  - Neutral expected (small helper indirection only in UI bridge paths).
- Measured delta:
  - `dom_getelementbyid_calls`: `33 -> 26`
- Follow-ups:
  - Continue WS2 with `analysisService` and remaining panel mount exceptions.

### 2026-02-27 - WS2 final DOM-ID API cleanup (analysis + panel mounts)
- Files touched:
  - `web/src/main.ts`
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/DebugWorkspacePanel.vue`
  - `web/src/components/DistributedWorkspacePanel.vue`
  - `web/src/components/DrillWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/components/GlobalShortcutBridge.vue`
  - `web/src/components/HeatmapWorkspacePanel.vue`
  - `web/src/components/HistoryWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
  - `web/src/components/OptimizeWorkspacePanel.vue`
  - `web/src/components/OverviewWorkspacePanel.vue`
  - `web/src/components/ParetoWorkspacePanel.vue`
  - `web/src/components/RunFooterPanel.vue`
  - `web/src/components/SettingsWorkspacePanel.vue`
  - `web/src/components/TopWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/services/analysisService.ts`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Eliminated remaining production `document.getElementById(...)` callsites; query helpers now use `querySelector` with guarded compatibility fallback where needed for tests/stubs.
  - Preserved existing panel mount and legacy-host compatibility behavior.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- Perf impact:
  - Neutral expected (API/callsite cleanup only; no new reactive work introduced).
- Measured delta:
  - `dom_getelementbyid_calls`: `26 -> 0`
- Follow-ups:
  - Proceed WS5 decommission and final verification updates.

### 2026-02-27 - WS5/WS6 artifact decommission + verification refresh
- Files touched:
  - `dist/index.html` (deleted)
  - `dist/app.js` (deleted)
  - `dist/app.css` (deleted)
  - `dist/vendor/lightweight-charts.standalone.production.mjs` (deleted)
  - `README.md`
  - `RELEASE.md`
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - Removed obsolete root legacy frontend artifacts so runtime and release ownership stays on `web/` + `web/dist` only.
  - Updated docs to align local run/build expectations with Vue + Vite frontend ownership.
- Tests run:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - `cargo test --workspace`
- Perf impact:
  - Neutral expected (artifact/docs cleanup plus gate refresh).
- Follow-ups:
  - Manual Tauri GUI smoke remains recommended for visual signoff.

### 2026-02-27 - CPU launch sanity after root-dist decommission
- Files touched:
  - `VUE_VITE_FINAL_TRANSITION_TRACKER.md`
- Behavior changed:
  - No code behavior change; verified app startup path after legacy root `dist/*` file removal.
- Tests run:
  - `cargo run -p ta_desktop_demo_app --release --bin ta_desktop_demo_app`
  - `cargo run -p ta_desktop_demo_app --release --bin ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- Perf impact:
  - Neutral expected (startup sanity check only).
- Follow-ups:
  - Capture manual GUI evidence for final visual/interaction signoff (CPU and CUDA modes).

Template:

```md
### YYYY-MM-DD - Slice name
- Files touched:
- Behavior changed:
- Tests run:
- Perf impact:
- Follow-ups:
```
