# Vue + Vite 100% Cutover Checklist

Last updated: 2026-02-26 (US date)
Status: In progress
Scope: Frontend/UI only (`web/`), keep Rust/CUDA backend behavior intact

## Objective

Complete a true 100% frontend transition to Vue + Vite by removing runtime dependence on `web/src/legacy/app.js` and `web/src/legacy/template.html` while preserving functionality, correctness, and performance.

## Non-Negotiables

- [x] No feature loss.
- [x] No backend command contract breakage.
- [x] No storage compatibility breakage for existing users.
- [x] No regression in Rust/CUDA compute paths (frontend remains orchestration/render layer only).
- [ ] Frontend performance remains neutral-or-better vs current hybrid baseline.

## Current Blockers Baseline (2026-02-24)

- [x] `web/src/legacy/app.js` still exists and is runtime-loaded.
- [x] `web/src/App.vue` still mounts `LegacyRuntimeBridge` and `LegacyCompatibilityRoots`.
- [x] 8 production stores still import `../legacy/actions`.
- [x] 11 production stores still contain `document.getElementById(...)` mutation paths.
- [x] 10 compatibility bridge files still exist under `web/src/legacy/bridges/`.
- [x] Legacy code footprint remains substantial (`web/src/legacy/app.js` ~16.5k lines; `web/src/legacy` has 35 files).

## Hard Definition Of Done (100% Complete)

### Runtime ownership
- [x] `web/src/App.vue` no longer imports or renders `LegacyRuntimeBridge`.
- [x] `web/src/App.vue` no longer imports or renders `LegacyCompatibilityRoots`.
- [x] No runtime import of `web/src/legacy/app.js`.
- [x] No runtime dependency on `web/src/legacy/template.html`.
- [x] Vue/Pinia stores and services are the only UI state owners.

### Action/command ownership
- [x] No production store/component imports from `web/src/legacy/actions.ts`.
- [x] All user actions route through typed Vue service modules + `invokeCommand`.
- [x] All request builders currently in `legacy/app.js` are ported to pure TS modules under Vue-owned code.

### DOM ownership
- [x] No store-level direct DOM mutation (`document.getElementById`) for business actions.
- [x] No `vta:*` snapshot event dependence for core behavior.
- [x] Any remaining direct DOM access is only for isolated view-only adapters with explicit rationale.

### Legacy removal
- [x] `web/src/legacy/bridges/*` removed (or reduced to zero runtime imports).
- [x] `web/src/legacy/actions.ts` removed (or no production imports).
- [x] `web/src/legacy/app.js` removed from production bundle path.
- [x] `web/src/legacy/template.html` removed from production bundle path.

### Quality gates
- [x] `npm --prefix web run typecheck` passes.
- [x] `npm --prefix web run test` passes.
- [x] `npm --prefix web run lint` passes.
- [x] `npm --prefix web run build` passes.
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` passes.
- [x] `cargo test --workspace` passes.

### Performance gates
- [x] Startup and interaction metrics are within agreed tolerance vs hybrid baseline.
- [x] Large-table views remain virtualized.
- [x] Large payload state uses shallow/non-deep reactive patterns (`shallowRef`, `markRaw`) where appropriate.
- [x] No measurable increase in optimization/validation command latency attributable to frontend orchestration.

## Execution Plan (Phased Checklist)

## Phase 0: Freeze + Measurement

- [ ] Create a migration branch/tag checkpoint.
- [x] Capture baseline metrics (startup, TTI, task update throughput, memory snapshot).
- [x] Capture baseline functional smoke report across all workflow tabs.
- [x] Lock scope: frontend runtime ownership refactor only (no backend algorithm changes).

Exit criteria:
- [x] Baseline artifacts stored and linked in this document.

## Phase 1: Vue Service Layer First

- [x] Add transitional compatibility service adapter (`web/src/services/legacyRuntimeActions.ts`) so stores depend on `services/*` instead of direct `legacy/*` imports.
- [x] Create/standardize domain services in `web/src/services/*`:
- [x] `dataService`
- [x] `optimizeService`
- [x] `validationService`
- [x] `analysisService`
- [x] `historyService`
- [x] `settingsService`
- [x] `executionService`
- [x] `metaService`
- [x] Move command payload/response normalization into these services.
- [x] Add unit tests for each service command path and error normalization.

Exit criteria:
- [x] Stores can consume typed services without needing legacy action wrappers.

## Phase 2: Remove Legacy Actions From Stores

- [x] `web/src/stores/dataStore.ts` migrated off `../legacy/actions`.
- [x] `web/src/stores/optimizeStore.ts` migrated off `../legacy/actions`.
- [x] `web/src/stores/validationStore.ts` migrated off `../legacy/actions`.
- [x] `web/src/stores/analysisStore.ts` migrated off `../legacy/actions`.
- [x] `web/src/stores/historyStore.ts` migrated off `../legacy/actions`.
- [x] `web/src/stores/settingsStore.ts` migrated off `../legacy/actions` (no direct import present in current implementation).
- [x] `web/src/stores/executionStore.ts` migrated off `../legacy/actions`.
- [x] `web/src/stores/advancedStore.ts` migrated off `../legacy/actions` (or folded into service-backed modules).

Exit criteria:
- [x] `rg -l "from '../legacy/actions'" web/src/stores -g "*.ts" -g "!*.test.ts"` returns no hits.

## Phase 3: Port Legacy Request Builders + Runtime State

- [x] Port optimization request builders out of `legacy/app.js`.
- [x] Port validation request builders out of `legacy/app.js`.
- [x] Port meta/batch/stage4 request builders out of `legacy/app.js`.
- [x] Move run artifact ownership (`CURRENT_RUN` equivalents) into Pinia/domain store.
- [x] Move validation artifact ownership (`CURRENT_VALIDATION` equivalents) into Pinia/domain store.
- [x] Move top/filter/select state ownership fully into Vue stores.
- [x] Remove reliance on hidden DOM state for options/enablement.

Exit criteria:
- [x] Core workflows run from Vue-owned state only.

## Phase 4: Remove Bridge-Driven Snapshot Ownership

- [x] Replace `dataContextBridge` with direct store/service updates.
- [x] Replace `optimizeValidationBridge` with direct store/service updates.
- [x] Replace `analysisHistorySettingsBridge` with direct store/service updates.
- [x] Replace `strategyContextBridge` with direct store/service updates.
- [x] Replace `advancedContextBridge` with direct store/service updates.
- [x] Replace `workflowBridge` with direct workflow store ownership.
- [x] Remove compatibility event dependence (`vta:*`) for core behavior.

Exit criteria:
- [x] `web/src/legacy/bridges/*` are not required by runtime app boot.

## Phase 5: Eliminate Store-Level DOM Mutation Paths

- [x] Remove direct DOM writes from `workflowStore`.
- [x] Remove direct DOM writes from `validationStore`.
- [x] Remove direct DOM writes from `dataStore`.
- [x] Remove direct DOM writes from `advancedStore`.
- [x] Remove direct DOM writes from `executionStore`.
- [x] Remove direct DOM writes from `strategyStore`.
- [x] Remove direct DOM writes from `optimizeStore`.
- [x] Remove direct DOM writes from `settingsStore`.
- [x] Remove direct DOM writes from `analysisStore`.
- [x] Remove direct DOM writes from `historyStore`.

Exit criteria:
- [x] `rg -l "document.getElementById" web/src/stores -g "*.ts" -g "!*.test.ts"` returns no production hits (or documented explicit exceptions).

## Phase 6: Remove Legacy Runtime Boot

- [x] Remove `<LegacyRuntimeBridge />` from `web/src/App.vue`.
- [x] Remove `<LegacyCompatibilityRoots />` from `web/src/App.vue`.
- [x] Remove runtime import path to `legacy/app.js`.
- [x] Remove runtime import path to `legacy/template.html`.
- [x] Delete or quarantine obsolete legacy files.

Exit criteria:
- [x] App runs fully in Tauri with Vue runtime only.

## Phase 7: Clean Tests + Dead Code + Docs

- [x] Update tests that mock legacy actions to mock service modules instead.
- [x] Remove deprecated legacy action tests no longer relevant.
- [x] Remove dead CSS/selectors tied to compatibility template.
- [x] Update architecture docs to reflect Vue-native ownership.
- [x] Mark legacy migration docs as archived/superseded where appropriate.

Exit criteria:
- [x] Test suite and docs describe only Vue-owned runtime paths.

## Domain Acceptance Matrix (Must All Pass)

### Workflow + Navigation
- [x] Workspace/workflow/controls/results transitions work without legacy runtime.
- [x] Command palette + shortcuts remain fully functional.

### Data + Strategy + Optimize
- [x] CSV load, Polygon flows, timeframe, run/cancel, validation trigger parity.
- [x] MA/strategy configuration parity preserved.

### Validation + Reports
- [x] Validation run parity preserved.
- [x] CSV/JSON/XLSX exports parity preserved.
- [x] Reports-folder open/reveal/open-path parity preserved.

### Analysis + Drill + Heatmap
- [x] Top sorting/filtering/select parity preserved.
- [x] Drill charts + stability + heatmap parity preserved.

### History
- [x] Apply/import/export/clear and saved-run CRUD/rerun parity preserved.

### Meta + Stage4 + ML
- [x] Batch flows, meta selection, stage4 actions, ML actions parity preserved.

### Execution
- [x] Runner control and status parity preserved.

## Performance Checklist (Frontend)

- [x] Keep heavy datasets as shallow/raw state where deep reactivity is unnecessary.
- [x] Keep row virtualization enabled for large result tables.
- [x] Throttle/coalesce high-frequency progress/live updates.
- [x] Keep task center updates O(1)-ish per event path.
- [x] Re-capture perf snapshots after each major phase.
- [x] Compare against baseline and document deltas.

Latest recorded delta (workflow gate, candidate):
- `target/workflow_regression_summary.json` runtime: `384ms`
- baseline (`scripts/baselines/workflow_regression_baseline.json`) runtime: `345ms`
- gate result: `runtime_not_regressed_50pct` = pass

Latest UI capture (`scripts/baselines/ui_capture/perf_metrics.json`):
- `app_cold_start_ms`: `40`
- `time_to_interactive_ms`: `566`
- `chart_live_feed_responsiveness.avg_ms_per_event`: `0.002` (`events`: `200`)
- post-release telemetry review (`scripts/baselines/post_release/telemetry_review.json`): `17/17` checks passed

## Risk Register (Track During Execution)

- [ ] Risk: silent feature drift while replacing legacy request builders.
- Mitigation: parity tests and per-domain smoke matrix.
- [ ] Risk: runtime errors from missing implicit legacy globals.
- Mitigation: explicit typed store state + service contracts before deletion.
- [ ] Risk: UI perf regressions from deep reactive wrapping.
- Mitigation: shallow refs, `markRaw`, virtualization, perf gate per phase.
- [ ] Risk: migration fatigue from broad scope.
- Mitigation: strict vertical slices with phase exit criteria.

## Cutover Sign-Off Checklist

- [x] All phase exit criteria met.
- [x] All domain acceptance checks passed.
- [x] All quality gates passed.
- [x] Performance gates passed.
- [ ] Manual Tauri smoke run verified (CPU-only and CUDA-enabled where applicable).
- [x] Release notes updated for migration completion.

## Tracking Fields

- Owner:
- Current phase: Final sign-off (manual Tauri smoke pending).
- Last completed item: Restored baseline UI capture by re-aligning selectors to Vue shell IDs/data attributes, re-ran `npm --prefix web run baseline:ui`, re-ran web quality gates (`typecheck`, `test`, `lint`, `build`, `parity`, `parity:outputs`), re-ran workflow gate (`scripts/run_workflow_gate.ps1 -Mode candidate`), and regenerated post-release telemetry review (`node scripts/post_release_telemetry_review.mjs`) with `17/17` checks passed. Also ran CPU app launch sanity (`cargo run -p ta_desktop_demo_app --release --bin ta_desktop_demo_app`) which remained alive until CLI timeout (expected GUI event loop behavior), and verified CLI startup path returns successfully with `--help`.
- Blockers: No automated-gate blocker remains; remaining sign-off work is manual Tauri smoke (CPU-only now, CUDA-enabled later).
- Next 3 tasks:
  - Run manual Tauri smoke on CPU-only build and attach screenshots/logs.
  - Run manual Tauri smoke on CUDA-enabled build and attach screenshots/logs.
  - Finalize cutover sign-off after manual smoke evidence is attached.
