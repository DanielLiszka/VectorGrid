# Vue UI ROI Implementation Plan (Performance-First)

Last updated: 2026-02-27 (US date)  
Scope: `web/` frontend only (Vue + Vite).  
Hard constraint: Keep `lightweight-charts` for charting.

## Objective

Implement high-ROI Vue ecosystem additions where they improve UX without regressing performance:

1. `@tanstack/vue-virtual` (table virtualization standardization)
2. `@floating-ui/vue` (stable anchored overlays/popovers)
3. `@vueuse/core` (composable-level perf + UX utilities; already present, expand usage)
4. `reka-ui` (accessible, unstyled primitives for overlays/dialog/popover)
5. `unocss` (optional, phased pilot only)

## Non-Negotiables

- No Rust/CUDA backend behavior changes.
- No Tauri command contract changes.
- No feature loss.
- No UI correctness regressions.
- No material frontend perf regressions.
- Keep `lightweight-charts`.

## Current Snapshot

- Vue app is active and fully owns runtime (`web/src/App.vue`, `web/src/main.ts`).
- `@vueuse/core` is already installed and pinned.
- Large-table virtualization is currently custom/manual in:
  - `web/src/components/TopWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
- Overlay surfaces with UX debt:
  - `web/src/components/WorkflowQuickSwitch.vue` (Reka popover pilot active; verify keyboard parity)
  - `web/src/components/TaskCenterOverlay.vue` (toggle/open behavior and layout interaction)
  - `web/src/components/CommandPaletteOverlay.vue` (Reka dialog pilot active; verify interaction coverage)

## Package Version Targets (plan baseline)

Checked on 2026-02-27 via `npm view`:

- `vue`: latest `3.5.29` (repo currently `3.5.28` in `web/package.json`)
- `vite`: latest `7.3.1` (already pinned)
- `@vueuse/core`: latest `14.2.1` (already pinned)
- `pinia`: latest `3.0.4` (already pinned)
- `lightweight-charts`: latest `5.1.0` (already pinned)
- `@tanstack/vue-virtual`: latest `3.13.19` (installed)
- `@floating-ui/vue`: latest `1.1.10` (installed)
- `reka-ui`: latest `2.8.2` (installed)
- `unocss`: latest `66.6.2` (not yet installed)

## Workstream 1: Virtualization Standardization (`@tanstack/vue-virtual`)

Goal: replace duplicated manual virtual row math with a shared, tested virtualizer layer.

### Checklist

- [x] Add dependency: `@tanstack/vue-virtual`.
- [x] Add shared composable: `web/src/composables/useVirtualTableRows.ts`.
- [x] Migrate `TopWorkspacePanel` to shared virtualizer.
- [x] Migrate `ValidationWorkspacePanel` to shared virtualizer.
- [x] Migrate `MetaWorkspacePanel` to shared virtualizer.
- [x] Preserve row selection, keyboard navigation, and scroll restoration behavior.
- [x] Keep spacer-row DOM structure stable for visual parity.
- [x] Add tests for virtual window bounds and row identity stability.

### Exit Criteria

- [x] No table renders more than visible rows + overscan.
- [x] No regressions in sort/select/export interactions.
- [ ] Scrolling remains smooth at large row counts.

## Workstream 2: Overlay/Popover Positioning (`@floating-ui/vue`)

Goal: remove detached/floating collisions and make overlays track anchors correctly.

### Checklist

- [x] Add dependency: `@floating-ui/vue`.
- [x] Create shared composable: `web/src/composables/useAnchoredOverlay.ts`.
- [x] Migrate Workflow Nav to anchored behavior (non-blocking, no detached overlap).
- [x] Make Task Center collapsed by default; keep explicit user toggle state persisted.
- [x] Stabilize Task Center placement with `autoUpdate` and viewport-aware middleware (`offset`, `flip`, `shift`).
- [x] Apply same positioning discipline to popovers/dropdowns where overlap is currently brittle. (Task Center + Workflow Nav + Analysis dock)
- [x] Add Escape/outside-click close and restore focus to trigger.

### Exit Criteria

- [ ] No overlay detaches during resize/scroll.
- [x] No overlay blocks critical controls by default.
- [ ] Keyboard/focus behavior is consistent and predictable.

## Workstream 3: Accessible Primitives (`reka-ui`)

Goal: replace custom overlay/dialog wiring where accessibility/focus handling is fragile.

### Checklist

- [x] Add dependency: `reka-ui`.
- [x] Pilot convert `CommandPaletteOverlay` to Reka dialog primitives.
- [x] Pilot convert one popover surface (Workflow Nav or Task Center controls) to Reka popover primitives.
- [x] Keep visual style system custom (unstyled primitives only).
- [x] Add interaction tests for focus trap, `Esc`, and tab order.

### Exit Criteria

- [x] Command palette and popover interactions pass keyboard-only smoke.
- [x] No regressions in existing shortcuts (`Ctrl+K`, etc.).

## Workstream 4: VueUse Expansion (`@vueuse/core`)

Goal: reduce custom utility code and stabilize high-frequency UI updates.

### Checklist

- [ ] Standardize debounced/throttled UI updates: (partial: task center persistence + top results filters + history saved-runs search/tags + optimize range fields + strategy cost/risk inputs + data/execution/settings/advanced inputs + validation/meta panel inputs now use `useDebounceFn`; repeated live event streams now dedupe-throttle)
  - `useDebounceFn` for text/filter inputs.
  - `useThrottleFn` for high-frequency progress/UI event rendering.
- [x] Standardize resize handling with `useResizeObserver` for chart/table containers.
- [x] Standardize event cleanup with `useEventListener`.
- [x] Standardize persisted view toggles with `useStorage` where applicable.
- [x] Audit watcher churn in overlays/tables and convert heavy paths to throttled composables.

### Exit Criteria

- [x] Less duplicated timer/listener code.
- [ ] No leaked listeners on mount/unmount cycles.
- [ ] Smoother perceived updates under event load.

## Workstream 5: Chart Host Hardening (`lightweight-charts`, no library swap)

Goal: keep chart updates responsive and predictable while preserving existing chart technology.

### Checklist

- [x] Introduce `web/src/composables/useLightweightChart.ts` for lifecycle ownership.
- [x] Ensure chart + series instances are `markRaw` and never deep-proxied. (drill chart host path)
- [x] Use resize observer-driven `applyOptions`/resize updates (batched when needed).
- [x] Keep data-update batching explicit to avoid unnecessary redraw churn.
- [ ] Preserve chart appearance and interaction parity.

### Exit Criteria

- [ ] No chart lifecycle leaks.
- [ ] No chart responsiveness regression on tab/workspace switches.

## Workstream 6: Styling Strategy (Optional Pilot: `unocss`)

Goal: improve iteration speed and consistency without destabilizing current CSS.

### Decision

Run as an opt-in pilot only, not full migration.

### Checklist

- [ ] Add `unocss` in pilot mode with minimal preset config.
- [ ] Restrict usage to new/isolated surfaces only (Task Center + Workflow Nav refresh).
- [ ] Keep existing `web/src/styles/app.css` as authoritative baseline during pilot.
- [ ] Compare build output size and developer ergonomics before broader adoption.

### Exit Criteria

- [ ] No CSS regression in existing panels.
- [ ] Measurable dev velocity gain or CSS simplification; otherwise defer.

## Rollout Order (Recommended)

1. Workstream 2 (`@floating-ui/vue`) - immediate UX pain and low backend risk.  
2. Workstream 4 (expand `@vueuse/core`) - low risk, immediate stability/perf gains.  
3. Workstream 1 (`@tanstack/vue-virtual`) - reduce table complexity and maintenance debt.  
4. Workstream 3 (`reka-ui`) - harden accessibility/focus for overlays/dialogs.  
5. Workstream 5 (chart host hardening) - targeted lifecycle/perf cleanup.  
6. Workstream 6 (`unocss`) - optional pilot last.

## Validation Gates (must pass after each workstream)

- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run build`
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- [x] Tauri startup smoke (CPU + CUDA build variants via release build + launch check)
- [ ] Manual in-app visual Tauri smoke (CPU + CUDA): overlay positioning/focus/scroll/chart parity

## Performance Signoff Checklist

- [ ] Table scroll remains smooth with large datasets (manual visual confirmation pending).
- [ ] Command palette open/close latency remains near-instant (manual visual confirmation pending).
- [x] Overlay interactions do not block critical workflow controls by default.
- [x] Task Center remains collapsed by default on first launch (user can opt in).
- [x] No measurable increase in workflow gate runtime attributable to frontend changes.

## Progress Log

### 2026-02-27 - Task Center render-path optimization pass (collapsed fast path + stable heads)

- Reduced overlay render allocations while preserving behavior:
  - `web/src/components/TaskCenterOverlay.vue`
  - `activeTasks` now resolves snapshots only when Task Center is expanded.
  - Added `headItems(...)` helper so preview lists reuse source arrays when below cap (no unnecessary `slice` allocation).
  - Applied to:
    - history preview
    - events preview
    - timeline preview
    - toast preview
- Validation:
  - `npm --prefix web run test -- src/components/TaskCenterOverlay.test.ts src/stores/taskStorePerformance.test.ts src/stores/taskStoreEventThrottle.test.ts`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 348ms vs 345ms baseline threshold still green)

### 2026-02-27 - Store-path optimization pass (task center active-order + task-store hot paths)

- Reduced repeated sort/filter work in Task Center/task-store hot paths:
  - `web/src/stores/taskStore.ts`
    - Added `activeOrder` tracking in store state (newest-first task id list).
    - Updated active-task lifecycle updates to maintain `activeOrder`:
      - `beginTrackedCommand`
      - `completeTrackedCommand`
      - `failTrackedCommand`
    - Reworked hot-path derivations to single-pass order scans:
      - `refreshCenterStatus`
      - `newestProgressTask`
      - `activeTasksSnapshot` (sort-free fast path; repair fallback for stale order).
    - Added `activeTasksCount()` helper for badge count without building/sorting snapshots.
    - Reduced small allocations:
      - `dismissToast` now uses indexed splice (no full-array filter copy).
      - `formatCommandLabel` now memoizes results (bounded cache).
  - `web/src/components/TaskCenterOverlay.vue`
    - Running badge now uses `activeTasksCount()` instead of depending on sorted snapshot length.
- Validation:
  - `npm --prefix web run test -- src/stores/taskStorePerformance.test.ts src/stores/taskStoreEventThrottle.test.ts src/components/TaskCenterOverlay.test.ts`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 349ms vs 345ms baseline threshold still green)

### 2026-02-27 - Store-path optimization pass (token + cache-prune cleanup)

- Reduced small but hot allocation patterns in store utility paths:
  - `web/src/stores/validationStore.ts`
    - Replaced `crossFilterTokens` map/filter chain with a single-pass loop.
  - `web/src/stores/historyStore.ts`
    - Replaced `filterTokens` map/filter chain with a single-pass loop.
    - Added shared `pruneSavedRunDetailCache(...)` helper and removed duplicated set-building/cache-prune blocks in:
      - `saveSavedRunTags`
      - `refreshSavedRuns`
      - `deleteSavedRun`
    - Preserved behavior, with stable cache identity when nothing is pruned.
- Validation:
  - `npm --prefix web run test -- src/stores/historyStore.actions.test.ts src/stores/validationStore.actions.test.ts`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 351ms vs 345ms baseline threshold still green)

### 2026-02-27 - Store-path optimization pass (validation cross-dataset selection loops)

- Refactored cross-dataset selection/update actions to reduce repeated `map/filter` passes and temporary allocations:
  - `web/src/stores/validationStore.ts`
  - Added one-pass helpers for:
    - cross-meta composition from counts
    - filtered selection application + changed-row extraction
  - Updated actions:
    - `crossRefresh`
    - `crossSelectAll`
    - `crossSelectNone`
    - `crossSelectMatch`
    - `toggleCrossDataset`
  - Preserved external behavior and backend call order used by tests.
- Validation:
  - `npm --prefix web run test -- src/stores/validationStore.actions.test.ts`
  - `npm --prefix web run test -- src/stores/domainStores.test.ts`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 348ms vs 345ms baseline threshold still green)

### 2026-02-27 - Store-path optimization pass (history + batch selection)

- Reduced redundant array/object work in core store paths:
  - `web/src/stores/historyStore.ts`
    - Removed full-copy return in no-filter path for saved-runs filtering.
    - Removed repeated `normalizeEntry` remap on every `syncSavedRunsView` call.
    - Kept sanitized raw arrays (`markRaw`) for option and saved-run lists.
  - `web/src/stores/analysisStore.ts`
    - Reworked `toggleBatchDataset` to targeted index replacement (`slice` + single row update) with incremental selected-count update.
    - Preserved existing behavior and backend sync call semantics.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 352ms vs 345ms baseline threshold still green)

### 2026-02-27 - Reactivity optimization pass (history/task-center allocation cleanup)

- Reduced transient allocation churn in Task Center cancel flow:
  - `web/src/components/TaskCenterOverlay.vue`
  - Replaced object-spread `busyCancelIds` map updates with `shallowRef<Set<string>>` and explicit add/remove helpers.
- Reduced deep reactive conversion and copy pressure in history data paths:
  - `web/src/stores/historyStore.ts`
  - Added raw-array normalization for option/saved-run arrays via `markRaw`.
  - Replaced `savedRunDetailCache` object-spread replacement with direct keyed assignment.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 353ms vs 345ms baseline threshold still green)

### 2026-02-27 - Reactivity optimization pass (watcher consolidation + top filter queue unification)

- Consolidated per-field draft sync watchers into batched `watch(() => [...])` blocks to reduce watcher/effect count in:
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/SettingsWorkspacePanel.vue`
  - `web/src/components/AdvancedWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/components/OptimizeWorkspacePanel.vue`
  - `web/src/components/StrategyWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
  - `web/src/components/HistoryWorkspacePanel.vue`
  - `web/src/components/HeatmapWorkspacePanel.vue`
  - `web/src/components/ParetoWorkspacePanel.vue`
  - `web/src/components/TopWorkspacePanel.vue`
- Replaced `TopWorkspacePanel` filter queue cloning path with shared `useQueuedInputWriter` map-backed queue:
  - Preserved legacy debounce contract (`waitMs: 130`, `maxWaitMs: 360`) to keep test/UX timing stable.
- Validation:
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`

### 2026-02-27 - Reactivity optimization pass (shared queued input writer)

- Added shared composable:
  - `web/src/composables/useQueuedInputWriter.ts`
- Refactored debounce-heavy panels to use shared non-reactive queued writer (Map-backed) instead of per-keystroke object cloning:
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/components/OptimizeWorkspacePanel.vue`
  - `web/src/components/StrategyWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
  - `web/src/components/SettingsWorkspacePanel.vue`
  - `web/src/components/AdvancedWorkspacePanel.vue`
- Behavior changes:
  - Preserved existing debounce/commit semantics and flush-before-action behavior.
  - Reduced reactive churn by removing repeated `queuedInputValues = { ...queuedInputValues, [id]: value }` allocations during typing.
  - Standardized queued-input flush lifecycle through one composable.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 370ms vs 345ms baseline threshold still green)

### 2026-02-27 - Tauri CPU/CUDA startup smoke pass

- Startup smoke performed for both release variants:
  - `cargo build -p ta_desktop_demo_app --release`
  - `cargo build -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel`
  - Launch smoke (12s alive check) for each built variant passed:
    - `cpu_launch_ok_started_and_running`
    - `cuda_launch_ok_started_and_running`
- Status:
  - Binary startup parity for CPU/CUDA variants is confirmed from CLI.
  - Remaining sign-off tasks are manual visual/interaction checks in the running app (overlay detach/focus parity, large-table scroll smoothness, chart interaction parity).

### 2026-02-27 - Workstream 4 settings/advanced input debounce pass

- Refactored high-frequency input writes:
  - `web/src/components/SettingsWorkspacePanel.vue`
  - `web/src/components/AdvancedWorkspacePanel.vue`
- Behavior changes:
  - Settings panel market-data path input now queues and debounces store writes with immediate `change` commit.
  - Advanced panel heatmap/top-k/export-path inputs now queue and debounce store writes with immediate `change` commit.
  - Pending queued values flush on unmount in both panels.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 345ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 validation/meta input debounce pass

- Refactored high-frequency input writes:
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
- Behavior changes:
  - Validation panel numeric/text inputs now queue and debounce store writes with immediate `change` commits.
  - Validation action buttons now flush pending queued values before invoking store actions.
  - Meta panel numeric/text inputs now queue and debounce store writes with immediate `change` commits.
  - Meta action buttons now flush pending queued values before invoking store actions.
  - Pending queued values flush on unmount in both panels.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 359ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 data/execution/settings input debounce pass

- Refactored high-frequency input writes:
  - `web/src/components/DataWorkspacePanel.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
- Behavior changes:
  - Data panel `csvPath` and `marketDataDir` now debounce keystroke writes, with immediate commit on `change`.
  - Execution panel order/risk/API/manual inputs now debounce keystroke writes with immediate `change` commit.
  - Execution action buttons now flush pending queued inputs before invoking backend commands.
  - Pending queued values flush on unmount in all three panels.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 346ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 strategy input debounce pass

- Refactored high-frequency strategy input writes:
  - `web/src/components/StrategyWorkspacePanel.vue`
- Behavior changes:
  - Cost/risk/constraint + Kaufman input fields now queue and debounce store writes instead of committing on each keystroke.
  - Added immediate `change` commits to avoid stale values when users transition directly into actions.
  - Pending queued values flush on unmount for correctness parity.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 346ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 optimize-range debounce pass

- Refactored high-frequency optimize range writes:
  - `web/src/components/OptimizeWorkspacePanel.vue`
- Behavior changes:
  - `fast/slow` range inputs (`start/end/step`) now queue and debounce store writes instead of committing on each keystroke.
  - Pending queued values flush before `Run optimization` / `Run validation` actions and on unmount to preserve correctness.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 352ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 history input debounce pass

- Refactored high-frequency saved-runs filter writes:
  - `web/src/components/HistoryWorkspacePanel.vue`
- Behavior changes:
  - Saved-runs search and tags inputs now debounce store writes with `useDebounceFn` instead of committing on every keystroke.
  - Pending edits flush on unmount to preserve correctness across workspace transitions.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 355ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 2 + 3 overlay/accessibility hardening

- Refactored overlay surface:
  - `web/src/components/AnalysisContextDock.vue`
- Added interaction coverage:
  - `web/src/components/AnalysisContextDock.test.ts`
  - `web/src/components/CommandPaletteOverlay.test.ts`
  - `web/src/components/TaskCenterOverlay.test.ts`
- Behavior changes:
  - Analysis dock now defaults collapsed and opens from an anchored toggle button (`useAnchoredOverlay` + `useStorage`).
  - Analysis dock force-closes when workflow context is not eligible.
  - Command palette now has deterministic `Tab` / `Shift+Tab` focus containment handling in-dialog.
  - Task Center default-collapsed state + persisted toggle behavior is now explicitly covered by component tests.
- Validation:
  - `npm --prefix web run test -- CommandPaletteOverlay.test.ts`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test` (49 files / 204 tests passed)
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 356ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 2 anchored overlay pass

- Added dependency:
  - `@floating-ui/vue@1.1.10`
- Added shared composable:
  - `web/src/composables/useAnchoredOverlay.ts`
- Refactored overlay surfaces:
  - `web/src/components/TaskCenterOverlay.vue`
  - `web/src/components/WorkflowQuickSwitch.vue`
- Behavior changes:
  - Task Center now defaults collapsed and persists open/closed state (`useStorage`).
  - Workflow Nav changed from always-floating panel to toggle-triggered anchored panel.
  - Both overlays close on outside-click and `Escape`.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; runtime gate still green)

### 2026-02-27 - Workstream 1 shared virtualization pass

- Added dependency:
  - `@tanstack/vue-virtual@3.13.19`
- Added shared composable:
  - `web/src/composables/useVirtualTableRows.ts`
- Refactored table virtualization surfaces:
  - `web/src/components/TopWorkspacePanel.vue`
  - `web/src/components/ValidationWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
- Behavior changes:
  - Replaced duplicated manual virtualization math with a shared `@tanstack/vue-virtual` path.
  - Preserved spacer-row rendering strategy and row-index-based action behavior.
  - Kept initial mount scroll reset behavior via `scrollToTop()`.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; runtime gate still green)

### 2026-02-27 - Workstream 3 Reka primitives pilot

- Added dependency:
  - `reka-ui@2.8.2`
- Refactored dialog + popover surfaces:
  - `web/src/components/CommandPaletteOverlay.vue` (Reka Dialog primitives)
  - `web/src/components/WorkflowQuickSwitch.vue` (Reka Popover primitives)
- Behavior changes:
  - Command palette now uses framework-managed dialog layering and close semantics.
  - Workflow Nav now uses Popover primitives with collision-aware placement.
  - Existing visual style remains custom CSS (unstyled primitive approach).
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run lint`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; runtime gate still green)

### 2026-02-27 - Workstream 3 interaction smoke tests

- Added test environment dependency:
  - `jsdom@28.1.0` (dev dependency)
- Added component interaction tests:
  - `web/src/components/CommandPaletteOverlay.test.ts`
  - `web/src/components/WorkflowQuickSwitch.test.ts`
- Coverage in this pass:
  - Command palette closes on `Escape` in jsdom-mounted interaction path.
  - Workflow nav popover toggles and persists open-state in storage.

### 2026-02-27 - Workstream 4 utilities (partial)

- Refactored event/listener lifecycle:
  - `web/src/components/GlobalShortcutBridge.vue` now uses `useEventListener` for keydown wiring/cleanup.
- Refactored persistence debounce path:
  - `web/src/stores/taskStore.ts` now uses `useDebounceFn` for task-center persistence scheduling.
- Intent:
  - Reduce custom timer/listener code in high-frequency UI support paths while keeping behavior stable.

### 2026-02-27 - Workstream 5 chart host hardening (partial)

- Added chart lifecycle composable:
  - `web/src/composables/useLightweightChart.ts`
- Refactored drill chart host wiring:
  - `web/src/services/analysisService.ts`
- Behavior changes:
  - Drill chart lifecycle ownership moved to shared runtime helper.
  - Chart + series instances now use `markRaw` ownership path in the drill chart host path.
  - Host resize path now uses `useResizeObserver`-driven `applyOptions`.

### 2026-02-27 - Workstream 5 chart batching pass

- Refactored drill chart render application path:
  - `web/src/services/analysisService.ts`
- Behavior changes:
  - Replaced immediate multi-step drill series updates with a single animation-frame commit path.
  - Removed redundant clear+set sequences during normal render updates (single `setData` per series per commit).
  - Added pending-frame cancellation on chart clear/dispose to prevent stale redraw work.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 346ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 debounced filter input pass

- Refactored high-frequency table filter writes:
  - `web/src/components/TopWorkspacePanel.vue`
- Added regression coverage:
  - `web/src/components/TopWorkspacePanel.test.ts`
- Behavior changes:
  - Top-results filter inputs now queue and debounce store writes instead of committing on every keystroke.
  - Pending filter edits are flushed on unmount to preserve correctness across panel/workspace transitions.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 358ms vs 345ms baseline threshold still green)

### 2026-02-27 - Workstream 4 resize observer standardization

- Refactored shared virtual table resize path:
  - `web/src/composables/useVirtualTableRows.ts`
- Behavior changes:
  - Added `useResizeObserver` on the table scroll container to trigger virtualizer re-measure on container size changes.
  - Keeps visible-row window calculations stable during panel resize/layout shifts.
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; runtime gate stayed green)

### 2026-02-27 - Workstream 3 + 4 hardening follow-up

- Added keyboard regression tests for global shortcuts:
  - `web/src/components/GlobalShortcutBridge.test.ts`
- Expanded interaction coverage now includes:
  - `Ctrl+K` opens command palette
  - `Escape` closes command palette
  - `Escape` polygon-modal precedence
  - `Escape` drill-clear + cancel path
  - Guarded `Ctrl+O` behavior when browse is disabled
- Reduced high-frequency event churn in task center feed:
  - `web/src/stores/taskStore.ts`
  - Added dedupe-throttle for `live_execution_runner_event` and `polygon_live_status` repeats while preserving distinct updates.
- Added store-level throttle tests:
  - `web/src/stores/taskStoreEventThrottle.test.ts`
- Validation:
  - `npm --prefix web run typecheck`
  - `npm --prefix web run lint`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (pass; 350ms vs 345ms baseline threshold still green)

## Risks and Mitigations

- Risk: introducing multiple UI primitives increases complexity.  
  Mitigation: start with one primitive library (`reka-ui`) and shared composables.

- Risk: virtualization migration breaks row behavior parity.  
  Mitigation: migrate one table at a time with snapshot + interaction tests.

- Risk: overlay behavior differences upset current users.  
  Mitigation: preserve keyboard shortcuts and add persisted preferences.

- Risk: unocss pilot causes style drift.  
  Mitigation: keep pilot scoped and reversible.

## Documentation Sources (official)

- Vue performance best practices: https://vuejs.org/guide/best-practices/performance
- Vue advanced reactivity (`shallowRef`, `markRaw`): https://vuejs.org/api/reactivity-advanced.html
- Vue built-in directives (`v-memo`): https://vuejs.org/api/built-in-directives
- Vue async components (`defineAsyncComponent`): https://vuejs.org/guide/components/async
- Vite performance guide: https://vite.dev/guide/performance.html
- VueUse docs: https://vueuse.org/
- TanStack Virtual Vue docs: https://tanstack.com/virtual/latest/docs/framework/vue
- Floating UI Vue docs: https://floating-ui.com/docs/vue
- Reka UI getting started: https://reka-ui.com/docs/overview/getting-started
- UnoCSS docs: https://unocss.dev/
- Lightweight Charts docs: https://tradingview.github.io/lightweight-charts/

