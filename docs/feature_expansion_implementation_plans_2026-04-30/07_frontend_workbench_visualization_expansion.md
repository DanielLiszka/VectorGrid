# Frontend Workbench And Visualization Expansion Plan

Date: 2026-04-30

## Goal

Expand the Vue/Tauri frontend into a clearer public Simple Mode and a more powerful Advanced workbench without breaking command contracts, table performance, accessibility, or desktop release posture.

This is a release-readiness target. The root `README.md` remains the public status authority until the signed desktop path is actually promoted.

## Current Owner Files

Shell and modes:

- `web/src/App.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/utils/simpleWorkflowModel.ts`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/utils/workbenchNavigationModel.ts`
- `web/src/utils/workbenchActionModel.ts`

Tables and visuals:

- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/composables/useVirtualTableRows.ts`
- `web/src/components/visualizations/PriceChartPanel.vue`
- `web/src/visualizations/lightweightChartRuntime.ts`
- `web/src/services/executionChartService.ts`
- `web/src/visualizations/drillChartModel.ts`
- `web/src/components/visualizations/MetricDistributionChart.vue`
- `web/src/visualizations/metricDistributionModel.ts`

QA:

- `web/e2e/browser/qa/*`
- `web/scripts/check-action-coverage-manifest.ts`
- `web/scripts/check-qa-selector-inventory.mjs`
- `web/scripts/check-e2e-artifact-fixtures.mjs`
- `web/scripts/check-ui-perf-budget.mjs`

## Research Basis

- Vue recommends update optimizations and virtualizing large lists: <https://vuejs.org/guide/best-practices/performance>
- Vue accessibility guidance emphasizes semantic structure and labels: <https://vuejs.org/guide/best-practices/accessibility.html>
- Playwright recommends user-visible behavior, resilient locators, and web-first assertions: <https://playwright.dev/docs/best-practices>
- Vitest Browser Mode is a candidate for focus/layout-heavy component tests that need real browser APIs without full end-to-end workflow cost: <https://vitest.dev/guide/browser/>
- Lightweight Charts supports series markers for event annotations: <https://tradingview.github.io/lightweight-charts/tutorials/how_to/series-markers>
- ECharts tree-shakable imports require explicit renderer/component registration: <https://echarts.apache.org/handbook/en/basics/import/>
- TanStack Virtual is headless and keeps markup/style ownership in the app: <https://tanstack.com/virtual/latest/docs>
- WAI-ARIA Authoring Practices and WCAG 2.2 should guide dense-table, tabs, dialogs, and keyboard behavior before adding custom interaction patterns: <https://www.w3.org/WAI/ARIA/apg/> and <https://www.w3.org/TR/WCAG22/>

## Expansion Track A: Simple Mode Public Workflow Completion

### Problem

Simple Mode is the intended public first-run path for signed desktop releases. It needs a proof-backed happy path.

### Implementation

Current owners:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/utils/simpleWorkflowModel.ts`
- `web/src/services/simpleSummaryService.ts`
- `src-tauri/src/commands_simple_summary.rs`

Add:

- first-run sample dataset readiness state;
- CPU-safe optimization preset path;
- optional validation recommendation;
- copyable compact summary;
- save Markdown summary action;
- Advanced handoff preserving current run.

### Tests

- component unit tests for empty/error/success states;
- E2E happy path from sample load to optimization summary;
- E2E optional validation path;
- action coverage manifest entries.

### Acceptance Criteria

- A clean install can run the intended Simple path without dev tools.
- Simple Mode does not expose options/live/ML as public first-run features.
- Summary text avoids financial advice or production trading claims.

## Expansion Track B: Advanced Workbench Action Model

### Problem

Advanced Mode has many workspaces and actions. Expansion needs stable action identity and keyboard/accessibility coverage.

### Implementation

Current owners:

- `web/src/stores/commandActionStore.ts`
- `web/src/stores/commandPaletteStore.ts`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/ContextActionMenu.vue`
- `web/src/utils/shellCommandRegistry.ts`
- `web/src/utils/shellShortcutRegistry.ts`
- `web/src/components/workbench/*`

Add:

- action registry ownership per workspace;
- stable action IDs for QA;
- unavailable/requires-data/requires-result reasons;
- keyboard shortcut conflict check;
- command palette drift guard.

### Tests

- static QA action coverage;
- command palette action state tests;
- keyboard shortcut conflict tests;
- E2E action coverage.

## Expansion Track C: Chart Annotation Upgrade

### Problem

Trading/candidate charts should show more state without adding another charting library.

### Implementation

Current owners:

- `web/src/visualizations/lightweightChartRuntime.ts`
- `web/src/services/executionChartService.ts`
- `web/src/composables/useDrillPriceChartRuntime.ts`
- `web/src/visualizations/drillChartModel.ts`
- `web/src/components/visualizations/PriceChartPanel.vue`
- `web/src/components/CandidateDetailWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`

Add:

- entry/exit markers;
- validation split overlays when payload supports them;
- live order markers;
- position/risk price lines;
- feed freshness marker;
- compact accessible legend.

Use built-in series markers and price lines first. Use custom primitives only where built-ins cannot express the requirement.

### Tests

- chart model unit tests;
- runtime API mock tests;
- component test for legend and empty state;
- Playwright smoke with fixture data.

## Expansion Track D: Artifact Analytics Lab

### Problem

Large artifacts need exploratory analysis, but production tables should remain stable.

### Implementation

Use optional, feature-flagged lab:

- new: `web/src/services/artifactAnalyticsRows.ts`
- new: `web/src/components/ArtifactAnalyticsLab.vue`

Potential dependencies:

- `@finos/perspective`
- `@finos/perspective-viewer`
- `@finos/perspective-viewer-datagrid`

Flag:

- `VITE_VECTORGRID_ENABLE_PERSPECTIVE_LAB=true`

Rules:

- flag off by default;
- no normal workflow imports when off;
- load flattened JSON rows first;
- defer Arrow IPC until profiling proves JSON parsing cost.

### Tests

- flag-off render tests;
- row flattener tests;
- failed dynamic import state;
- bundle budget and UI perf budget.

## Expansion Track E: Accessibility And Responsive Matrix

### Problem

Advanced dense workspaces can regress keyboard and screen-reader behavior.

### Implementation

Add a representative matrix:

- Simple Mode first-run;
- optimization controls;
- validation table;
- history artifact actions;
- options panel;
- execution emergency controls;
- settings/privacy controls.

Use:

- `npm --prefix web run a11y:smoke:built`
- `npm --prefix web run a11y:smoke:wcag:built`
- responsive E2E lanes for changed surfaces.

### Acceptance Criteria

- Icon-only buttons have accessible names.
- Form controls have labels.
- Table controls remain keyboard reachable.
- Text does not overflow or overlap in tested viewport set.

## Second-Pass Expansion: Dense Table And Virtualization Policy

### Problem

Large optimization, validation, history, and options tables need performance without losing keyboard navigation, row identity, or screen-reader context.

### Current Anchors

- `web/src/composables/useVirtualTableRows.ts`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/components/visualizations/MetricDistributionChart.vue`

### Implementation

Add a table policy before introducing new table dependencies:

- stable row id required for every virtualized table;
- visible sort/filter state;
- deterministic default sort;
- keyboard reachable selection and row actions;
- accessible name for each action button;
- summary text for offscreen row counts;
- no virtualization on small tables where it complicates accessibility.

If TanStack Virtual is adopted for a new table:

- keep cell markup app-owned;
- wrap only the row-windowing behavior;
- add component tests for empty, small, large, and focused-row states;
- include the table in the frontend QA matrix before public use.

### Acceptance Criteria

- table text does not overlap at the representative viewport set;
- hidden virtual rows are not exposed as reachable controls;
- row actions remain stable for Playwright locators;
- screen-reader summary states row count, active sort, and selected row when selection exists.

## Second-Pass Expansion: Chart Accessibility Layer

### Problem

Price, distribution, and metric charts are visual-first. Advanced users still need textual summaries that explain the displayed evidence.

### Implementation

For each production chart, add a companion model:

- chart title and dataset identity;
- visible range;
- series list;
- marker counts;
- split-window or event annotation summary;
- top warnings and unavailable-data reasons;
- table fallback or export link where practical.

Do not make chart libraries responsible for product copy. Build summary text in app-owned utilities next to the chart model.

### Tests

- chart summary utility tests;
- component tests for empty, loading, and populated summaries;
- accessibility smoke includes at least one populated chart fixture.

## Non-Goals

- No new landing page.
- No marketing hero.
- No replacement of Vue/Pinia/Tauri architecture.
- No replacing production tables with Perspective.
- No replacing Lightweight Charts for trading charts.

## Verification

Targeted:

- `npm --prefix web run test`
- focused component/service tests for changed files.

Frontend gates:

- `npm --prefix web run typecheck`
- `npm --prefix web run lint`
- `npm --prefix web run build`
- `npm --prefix web run check:bundle-budget`
- `npm --prefix web run perf:ui:budget`
- `npm --prefix web run check:qa-static`
- `npm --prefix web run test:e2e:qa:built`
- `npm --prefix web run a11y:smoke:built`

Broader:

- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeBrowserWorkflowQa`
