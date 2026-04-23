# Frontend UI/UX Redesign Implementation Plan

## Intent

Redesign the Vue/Tauri frontend so the app exposes the same backend capability through a clearer, faster, and more decision-oriented workstation. The backend command surface should remain stable unless a frontend improvement exposes a real backend contract gap. The main work is presentation, flow, interaction design, visualization architecture, and verification.

This plan covers the full frontend surface:

- Desktop shell, navigation, titlebar, task center, command palette, footer, settings, and perspectives.
- Data loading, strategy setup, optimization, validation, analysis, history, aggregate/meta, ML Signal, and live execution surfaces.
- TradingView Lightweight Charts usage for drilldown and execution review.
- Canvas heatmap and Pareto rendering.
- Accessibility, responsive layout, performance, and test coverage.

## Current Frontend Inventory

Primary shell composition lives in:

- `web/src/App.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/styles/app.css`

Workflow state and presentation levers already exist in:

- `web/src/stores/workflowStore.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/utils/core/workflowModel.ts`
- `web/src/composables/useUiReadiness.ts`

Visualization and dense result surfaces live mainly in:

- `web/src/services/analysisService.ts`
- `web/src/composables/useLightweightChart.ts`
- `web/src/services/executionChartService.ts`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`

Existing verification assets include:

- `web/package.json` scripts for `typecheck`, `test`, `build`, browser UX, desktop smoke, baseline UI, and accessibility smoke.
- `web/e2e/browser/` mocked-Tauri Playwright specs.
- `web/e2e/desktop/` WDIO/Tauri desktop smoke harness.
- `web/e2e/accessibility-review-matrix.md` manual accessibility review guidance.
- Component and service tests close to most major panels.

## Current Problems To Solve

1. The shell is dense and repetitive.

   The titlebar, shell header, workspace tabs, workflow-step tabs, context strip, panel tabs, and footer all compete for orientation/status. This consumes vertical space and makes the main task feel secondary.

2. Backend capability is exposed as implementation structure.

   Labels like `Workflow`, `Strategy Setup`, `Optimize`, `Winner Overview`, `Meta`, `Debug`, `Build posture`, and `View posture` expose internal concepts. Users should see a guided decision path: load data, choose approach, run, triage, validate, review risk, export or execute.

3. Empty and blocked states are too passive.

   Several panels reserve large blank areas and say the app is ready, even when a required input is missing. Empty states should offer the next valid action and explain why unavailable actions are blocked.

4. Controls, chips, tabs, badges, and buttons look too similar.

   The UI uses many pill-like elements with comparable visual weight. This reduces scan speed and weakens primary/secondary action hierarchy.

5. Visualization code is mixed with service/store mutation.

   Pareto and heatmap rendering are imperative canvas functions inside `analysisService.ts`. They directly query DOM IDs, mutate stores, bind event handlers, and render canvas. This makes UI iteration risky and limits component-level testability.

6. Lightweight chart usage is fragmented.

   `useLightweightChart.ts` is a small lifecycle helper, while execution chart logic owns a separate singleton runtime in `executionChartService.ts`, and drilldown charting remains tied to analysis service rendering. The app needs a shared chart surface architecture.

7. Chart and heatmap accessibility is uneven.

   The panels include helpful ARIA labels and keyboard handoffs to tables, but chart interactions are still mostly pointer-first. Heatmap/Pareto need keyboard selection, textual summaries, and stable accessible fallbacks.

8. Visual style reads as calm but overly beige and low-contrast.

   The current palette is workable, but the whole application reads close to one family of soft neutrals. A trading workstation should feel quieter than marketing UI, but still needs stronger hierarchy, sharper status colors, and better table/chart contrast.

## Product Design Direction

The target UI should feel like an operational trading research workstation, not a form collection. It should prioritize:

- Fast orientation: where am I, what is loaded, what can I do next?
- Clear decision progression: input -> run -> triage -> validate -> inspect -> export/execute.
- Progressive disclosure: default views show the useful 20 percent; advanced knobs are available but not front-loaded.
- Dense but organized data: tables, charts, and metadata should align around the selected candidate/run.
- One primary action at a time: the app should not offer multiple equally prominent run/validate/export controls without context.
- Backend parity: every current command remains reachable, either in the streamlined workflow, command palette, settings, or an advanced drawer.

## Target Frontend Architecture

### 1. Workbench Shell

Replace the current stacked navigation density with a workbench shell:

- Compact desktop titlebar for app identity, global search/command palette, settings, task center, and native window controls.
- Single workspace rail or segmented top nav for major modes: Research, Library, Aggregate, Live, Settings.
- In Research, use the workflow pipeline as the primary structure: Data, Configure, Optimize, Validate, Analyze, Finalize.
- Move secondary controls/results tabs inside the relevant stage as local subnavigation.
- Replace the wide context strip with a compact run context bar that shows only high-signal state: dataset, strategy, route, readiness, selected run.
- Convert lower run footer into a contextual action/status bar with one primary action, progress, and task summary.

Candidate files:

- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/utils/core/workflowModel.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/stores/settingsStore.ts`

### 2. Design System Layer

Introduce a small local component system before broad page changes:

- `AppButton.vue`: primary, secondary, subtle, danger, icon-only, loading, and disabled-reason support.
- `AppTabs.vue`: consistent keyboard behavior and selected-state rendering.
- `AppStatusChip.vue`: semantic status display, never confused with clickable controls.
- `AppPanel.vue`: unframed section/pane variants with consistent padding, radius, and headings.
- `AppEmptyState.vue`: title, body, primary action, secondary action, blocked reason, and optional illustration.
- `AppMetricStrip.vue`: compact metrics with label, value, trend/tone, and title overflow behavior.
- `AppToolbar.vue`: compact action rows for tables/charts.
- `AppDataTableShell.vue`: shared table header, column controls, summary, virtualized viewport, empty state, and keyboard hints.

Use existing stores and composables first. Do not add a third-party UI framework unless the current component patterns cannot support the target.

Candidate files:

- New `web/src/components/ui/*`
- `web/src/styles/app.css`
- Existing panel components as consumers

### 3. Smart Workflow Presentation

Create a readiness-driven workflow controller that turns raw store state into UI decisions:

- Extend `useUiReadiness.ts` into a richer `useWorkflowReadiness.ts` or `useWorkbenchState.ts`.
- Provide canonical stage status, next action, blocked reason, and recommended destination.
- Use this in titlebar/footer/panels instead of each panel inventing copy and actions.
- Make empty states action-oriented: load CSV, open Polygon, run optimization, review top results, run validation, inspect library, open reports.

Candidate files:

- `web/src/composables/useUiReadiness.ts`
- `web/src/stores/dataStore.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/components/*WorkspacePanel.vue`

### 4. Shared Visualization Runtime

Create a visualization layer with explicit model -> view boundaries:

- `web/src/visualizations/chartTheme.ts`: map app theme tokens to chart colors.
- `web/src/visualizations/lightweightChartRuntime.ts`: shared create/dispose/resize/series/markers behavior.
- `web/src/visualizations/priceSeriesModel.ts`: normalize price bars, indicators, trades, markers.
- `web/src/visualizations/heatmapModel.ts`: parse backend heatmap payload, plateau mode, ranges, palette, markers, tooltip text.
- `web/src/visualizations/paretoModel.ts`: axis options, point model, selected point, keyboard navigation.
- Vue components own DOM and rendering hosts. Services should return data models, not mutate DOM.

Initial components:

- `PriceChartPanel.vue`
- `HeatmapCanvas.vue`
- `HeatmapLegend.vue`
- `ParetoScatter.vue`
- `VisualizationToolbar.vue`
- `ChartEmptyState.vue`

This is the key technical change for charting and heatmap work.

## Backend Capability Preservation Matrix

The redesign must preserve reachability for every existing backend workflow. The UI can reorganize access, but it must not remove capability.

| Capability family | Current surface | Target presentation | Must remain reachable through |
| --- | --- | --- | --- |
| CSV dataset load | Data panel | Guided data-source chooser | Data stage, command palette |
| Polygon dataset download/load/validate/repair | Data panel | Polygon source drawer plus cache review | Data stage, Settings storage, command palette |
| Market-data folder management | Data and Settings | Settings-first storage management with Data shortcut | Settings, Data shortcut |
| Strategy selection and presets | Optimize/Strategy panels | Configure stage with default and advanced strategy detail | Configure stage, command palette |
| Optimization route/backend selection | Optimize/Advanced panels | Route summary with advanced override | Configure/Optimize advanced drawer |
| Optimization run/cancel | Titlebar/footer/Optimize | Single contextual primary action and task center cancel | Action status bar, titlebar compact action, task center, command palette |
| Top K, heatmap bins, shortlist knobs | Advanced panel | Advanced optimization drawer with presets | Configure/Optimize advanced drawer |
| Top results review/filter/sort/export | Top panel | Candidate review workspace | Analyze stage, command palette exports |
| Candidate drilldown | Drill panel | Candidate detail view tied to selected candidate | Analyze stage, Top row action, chart selection |
| Pareto review | Pareto panel | Candidate tradeoff visualization | Analyze stage, chart toolbar, Top fallback |
| Heatmap review/export | Heatmap panel | Robustness/parameter-surface visualization | Analyze stage, chart toolbar, command palette export |
| Validation setup/run/export | Validation panel | Validation stage with recommended pack plus advanced checks | Validate stage, command palette |
| Validation report open/reveal | Validation/history/actions | Report actions in validation result summary | Validate stage, Library, command palette |
| Saved run library | History panel | Library workspace | Library workspace, command palette |
| Aggregate/meta/batch/saved/compare | Meta panel/aggregate workspace | Aggregate workspace organized by artifact task | Aggregate workspace |
| ML Signal stage actions | ML Signal shell/panels | Mode-specific pipeline labels and stage actions | Research mode switch, command palette |
| Live broker review | Live workspace | Safety-first live workspace | Live workspace, task center |
| Live runner safety controls | Execution panel | Dedicated safety matrix | Live workspace, confirmation dialogs |
| Settings, diagnostics, logs | Settings/debug/task center | Settings grouped by user task; diagnostics secondary | Settings, Task Center |
| Window controls and desktop runtime state | Titlebar/runtime banner | Compact titlebar and runtime notice only when relevant | Titlebar, runtime banner |

Acceptance rule: each implementation PR that removes, renames, or relocates a visible control must update this matrix or include an equivalent PR-local checklist showing where the capability moved.

## Target Information Architecture

Target major workspaces:

- `Research`: the primary pipeline for data, configuration, optimization, validation, analysis, and finalization.
- `Library`: saved runs, reports, reopen, rerun, compare, export, reveal.
- `Aggregate`: multi-run and meta-artifact analysis.
- `Live`: broker-facing review, live session evidence, safety controls.
- `Settings`: appearance, workbench behavior, storage, notifications, retention, diagnostics.

Target Research stages:

| Stage | Current equivalent | Default view | Secondary/advanced access |
| --- | --- | --- | --- |
| Data | Workflow Data Source | Source chooser, loaded dataset profile, schema summary | Polygon cache tools, storage shortcut |
| Configure | Strategy Setup / Optimize controls | Strategy, objective, route recommendation, key sweep ranges | Full strategy config, backend override, advanced optimizer knobs |
| Optimize | Run control / Overview | Run readiness, launch/cancel, progress, result summary | Task center, run logs, route diagnostics |
| Validate | Validation | Recommended validation pack and current candidate context | Advanced validation methods and cross-dataset selection |
| Analyze | Overview/Top/Pareto/Drill/Heatmap | Candidate review dashboard with top table and linked visuals | Pareto, heatmap, drilldown, exports, comparison |
| Finalize | Meta/Stage 5 | Artifact promotion, ML confidence, final review | Batch/meta/debug detail |

Navigation rules:

- Major workspace navigation should be reachable with left/right keyboard movement and command palette commands.
- Research stage navigation should preserve local state when moving between stages.
- Result subviews should preserve selected candidate and filter context.
- Advanced drawers must not reset user input when opened/closed.
- Command palette should remain the escape hatch for power users and relocated controls.

## Screen-Level Target Specs

### Desktop Titlebar

- Keep fixed native-window controls.
- Show app identity, current workspace/stage, global command/search, settings, task center, and one compact primary action.
- Hide verbose subtitles unless they are critical status or runtime errors.
- Show disabled primary action with a tooltip or status detail that explains the blocker.

### Run Context Bar

- Replace the broad context strip with compact state cells:
  - Dataset and timeframe.
  - Strategy and objective.
  - Route/backend.
  - Selected run/candidate.
  - Readiness or current task.
- Only show an overflow menu for secondary facts such as loaded ID, row count, route provenance, and artifact path.

### Action Status Bar

- Own the current primary action for the active stage.
- Show progress, task summary, and latest important outcome.
- Provide `Details` to open Task Center instead of expanding into a large permanent footer.
- For blocked states, show the next corrective action, not a generic disabled button.

### Data Stage

- First screen: choose CSV, Polygon, or recent/cache.
- After load: show dataset card, timeframe, row count, fields, warnings, and schema.
- Storage controls remain available but secondary.
- Recent datasets should be selectable without reading filesystem paths first.

### Configure/Optimize Stage

- Show recommended path first: strategy, objective, preset, route recommendation, run readiness.
- Advanced details use section disclosures grouped by strategy, route, sweep, shortlist, heatmap, and GPU diagnostics.
- GPU/CUDA copy should say what route will actually be used and why.

### Analyze Stage

- Default to a candidate review dashboard:
  - Winner summary.
  - Top candidate table.
  - Selected candidate metrics.
  - Validation status if available.
  - Linked drilldown/pareto/heatmap affordances.
- Pareto and heatmap are visual tools for exploration; Top Results remains the exact retained-candidate path.

### Validate Stage

- Split into setup and results.
- Recommended validation pack should be one-click when inputs are ready.
- Advanced methods remain available but grouped.
- Results summary should distinguish admission decision, warnings, route, bootstrap, regime, friction, and cross-dataset evidence.

### Library/Aggregate

- Library should optimize for saved artifact review: search, selected run summary, rerun, load, compare, export, reveal.
- Aggregate should optimize for multi-run shortlist/meta decisions and keep debug output secondary.

### Live

- Keep conservative live behavior.
- Broker account review, position/orders, chart, and safety matrix should be visually distinct.
- Destructive actions keep confirmation dialogs with exact scope and non-scope copy.

## Detailed Component Backlog

### New UI Components

| Component | Purpose | Initial tests |
| --- | --- | --- |
| `AppButton.vue` | Standard buttons, icons, loading, disabled reason | variant rendering, disabled reason, icon label |
| `AppIconButton.vue` | Icon-only controls with required accessible names | accessible name required, focus style |
| `AppTabs.vue` | Shared tablist keyboard behavior | arrow/home/end selection, aria-selected |
| `AppStatusChip.vue` | Non-clickable semantic status | tone rendering, no button semantics |
| `AppPanel.vue` | Consistent section/pane shell | heading association, density classes |
| `AppEmptyState.vue` | Action-oriented empty/blocked states | primary/secondary actions, blocked reason |
| `AppMetricStrip.vue` | Compact metric summaries | overflow/title behavior, tone classes |
| `AppToolbar.vue` | Dense action groups for charts/tables | wrapping, icon/text layout |
| `AppDataTableShell.vue` | Shared virtual table scaffold | column controls slot, empty state, keyboard hint |
| `RunContextBar.vue` | Compact run/workflow context | state cells, overflow facts |
| `ActionStatusBar.vue` | Contextual primary action and task summary | blocked/running/ready states |

### New Visualization Components

| Component | Purpose | Initial tests |
| --- | --- | --- |
| `PriceChartPanel.vue` | Shared Lightweight Charts host for drill/live | lifecycle, resize, marker update |
| `ChartEmptyState.vue` | Truthful chart unavailable states | state copy, action routing |
| `HeatmapCanvas.vue` | Heatmap rendering, selection, tooltip, keyboard | render model, selection, export event |
| `HeatmapLegend.vue` | Reusable legend with labels | range labels, palette rendering |
| `ParetoScatter.vue` | Pareto canvas/SVG visualization | point selection, axis labels, keyboard nav |
| `VisualizationToolbar.vue` | Mode, export, fit, selected item actions | mode change, export disabled state |

## Data And Visualization Model Contracts

### Price Chart Model

Create model functions that accept backend/service data and return plain serializable chart input:

```ts
interface PriceChartModel {
  symbol: string
  datasetId: string
  bars: Array<{ time: number; open: number; high: number; low: number; close: number }>
  overlays: Array<{ id: string; label: string; points: Array<{ time: number; value: number }> }>
  markers: Array<{ id: string; time: number; position: 'aboveBar' | 'belowBar'; color: string; text: string }>
  selectedMarkerId: string
  status: 'ready' | 'blocked' | 'loading' | 'error'
  message: string
}
```

Rules:

- Fetching can remain in services, but Vue chart components own DOM lifecycle.
- Chart components receive models and emit selections.
- No chart runtime should be a singleton unless it is explicitly keyed by chart instance.

### Heatmap Model

Create model functions that return:

```ts
interface HeatmapViewModel {
  mode: 'objective' | 'plateau'
  rows: number
  cols: number
  cells: Array<{
    row: number
    col: number
    fast: number
    slow: number
    value: number | null
    displayValue: number | null
    label: string
    sampleCount?: number | null
    objectiveValue?: number | null
  }>
  range: { min: number; mid: number; max: number; label: string }
  palette: 'objective-diverging' | 'plateau-sequential'
  markers: Array<{ row: number; col: number; label: string; rowId?: string }>
  selectedCell: { row: number; col: number } | null
  status: string
}
```

Rules:

- Plateau scoring must stay deterministic and covered by model tests.
- Rendering should support device-pixel-ratio scaling.
- Keyboard selection uses arrow keys over populated cells and updates the same selected-cell state as pointer hover/click.
- Export PNG comes from the component render surface, but permission/enabled state remains store-driven.

### Pareto Model

Create model functions that return:

```ts
interface ParetoViewModel {
  xKey: string
  yKey: string
  xLabel: string
  yLabel: string
  points: Array<{
    id: string
    rowId: string
    rank: number
    x: number
    y: number
    selected: boolean
    frontier: boolean
    summary: string
  }>
  xRange: { min: number; max: number }
  yRange: { min: number; max: number }
  status: string
}
```

Rules:

- Axis option generation stays pure and testable.
- Selection emits a row/candidate identity rather than directly calling drilldown services.
- Keyboard navigation cycles through visible points by rank and nearest-neighbor movement.

## State Fixtures For Development And Tests

Create reusable mocked frontend state fixtures under `web/e2e/browser/fixtures/` or `web/src/test/fixtures/`:

| Fixture | Purpose |
| --- | --- |
| `empty-workspace` | No dataset, no run, no validation |
| `csv-loaded` | Dataset loaded, no optimization |
| `optimize-ready` | Dataset and strategy ready, run not started |
| `optimization-running` | Active task with progress |
| `results-basic` | Best result and top table without heatmap |
| `results-heatmap` | Top table, heatmap payload, Pareto rows, drill curves |
| `validation-ready` | Current run with no validation result |
| `validation-results` | Validation result table and report actions |
| `library-populated` | Saved runs with selected saved artifact |
| `aggregate-populated` | Meta/batch/saved/compare artifacts |
| `ml-signal-ready` | ML Signal mode through each stage |
| `live-review` | Broker snapshot, orders, price bars, live session artifact |
| `runtime-warning` | Runtime banner/task-center warning state |

Each fixture should support:

- Unit/component tests where possible.
- Browser UX mocked-Tauri state.
- Baseline UI screenshot capture.

## Test Matrix

| Area | Unit/component | Browser UX | A11y smoke/manual | Desktop smoke |
| --- | --- | --- | --- | --- |
| UI primitives | Required | Optional | Manual spot check | No |
| Shell/nav/titlebar/footer | Required | Required | Required | Required if titlebar/native behavior changes |
| Data stage | Required | Required | Manual data slice | Desktop only for file dialog/open-folder behavior |
| Configure/Optimize | Required | Required | Manual form review | No unless native runtime copy changes |
| Task center/command palette | Required | Required | Required overlay slice | Required if titlebar trigger behavior changes |
| Overview/Top/Validation tables | Required | Required seeded dense surface | Required dense surface slice | No unless open/reveal path changes |
| Lightweight chart components | Required with mocked library | Required chart state/fallback | Manual chart alternative review | Optional live shell proof |
| Heatmap/Pareto | Required model + component | Required ready/empty state | Required keyboard/text alternative review | No |
| Library/Aggregate | Required | Required artifact review | Manual reopen/review slice | Required for open/reveal path |
| Live execution | Required | Required mocked broker journey | Manual live status slice | Required for native/live safety paths |
| Settings/perspectives | Required | Required for perspective restore | Manual settings slice | Required for window/native settings |

Minimum broad-change command set:

```powershell
npm --prefix web run typecheck
npm --prefix web run test
npm --prefix web run build
npm --prefix web run a11y:smoke:managed
npm --prefix web run test:browser:ux
```

Add desktop smoke when native-shell behavior is touched:

```powershell
npm --prefix web run test:desktop:smoke
```

## Performance Budgets

Initial budgets should be measured from current baseline and tightened after the first redesign slice:

| Surface | Budget target |
| --- | --- |
| App shell first useful paint | No slower than current baseline by more than 10 percent |
| Baseline UI capture runtime | No slower than current baseline by more than 15 percent |
| Large virtualized table scroll | No visible row jank with representative top/validation/history rows |
| Heatmap render | 512x512 model renders within one animation frame on representative payloads where possible |
| Pareto render | 512 points render and hit-test without perceptible delay |
| Live chart update | Synthetic live-bar event average remains within existing baseline range |
| Bundle size | No large third-party UI/chart additions without explicit justification |

Instrumentation:

- Preserve `scripts/capture-baseline-ui.mjs` perf metrics.
- Add focused timing around heatmap/Pareto model building if those become slow.
- Avoid deep reactive storage of large chart arrays. Prefer shallow refs or plain model snapshots.

## Accessibility Requirements

Global:

- All icon-only controls require accessible names and visible focus.
- Disabled controls need discoverable blocked reasons where practical.
- Keyboard users must be able to complete the main workflow without pointer-only chart interactions.
- Avoid relying on color alone for status.
- Color contrast should be checked manually because the current axe smoke disables `color-contrast`.

Charts and visualizations:

- Provide textual summary for current chart state.
- Provide table/list fallback or linked table path for chart selections.
- Heatmap and Pareto require keyboard selection or an equivalent keyboard-first route through Top Results.
- Tooltip content must also be available in a persistent selected summary, not only hover.

Dense tables:

- Virtual spacer rows must stay hidden from assistive technology.
- Column controls need clear names.
- Selected row/candidate state must be announced or visible in adjacent summary.

Overlays:

- Command palette, task center, confirmations, and context menus need focus trap and focus return.
- Escape must close overlays consistently.

## Phase Exit Checklist

Every phase should exit with:

- User-facing behavior summary.
- Files changed.
- Backend actions preserved or relocated.
- Tests run and skipped tests explained.
- Before/after screenshot or baseline reference for layout-heavy work.
- Accessibility notes for touched slices.
- Known follow-up items.

Do not merge a phase if:

- A backend action disappears without replacement.
- Main keyboard navigation regresses.
- Empty states lose the next valid action.
- Chart/table performance visibly regresses without a mitigation plan.
- A broad visual change lacks screenshot evidence.

## Phased Implementation

### Phase 0: Baseline, Scope Lock, And Visual Evidence

Goal: freeze the current behavior and collect visual/accessibility evidence before changing broad UI surfaces.

Tasks:

- Run or refresh existing UI screenshot baselines using `npm --prefix web run baseline:ui` when Playwright browsers are available.
- Capture representative states: no data, data loaded, optimization results, validation results, heatmap-ready, drilldown-ready, library populated, live review, settings.
- Record current app layout issues under `output/manual_a11y_review/` or a dedicated UI review note.
- Define non-negotiable backend access inventory: all Tauri commands and major actions that must remain reachable.

Acceptance:

- A visual baseline exists for current state.
- A checklist exists for backend actions that must remain reachable after redesign.
- No frontend behavior changed yet.

### Phase 1: Design Tokens And UI Primitives

Goal: make later redesign work consistent and lower-risk.

Tasks:

- Refactor `app.css` tokens into clearer groups: color, typography, spacing, density, elevation, chart colors, semantic tones.
- Reduce dominant beige/cream dependence while preserving light/dark/system themes.
- Standardize radii and density. Keep compact operational controls; avoid decorative card nesting.
- Add `components/ui` primitives listed above.
- Migrate one low-risk panel, likely Settings or Data, to prove the primitives.

Files:

- `web/src/styles/app.css`
- `web/src/components/ui/*`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.vue`

Tests:

- Add primitive component tests.
- Keep existing Settings/Data tests passing.
- Run `npm --prefix web run typecheck` and focused Vitest specs.

Acceptance:

- Buttons, chips, tabs, panels, and empty states have distinct roles and styling.
- No backend action is removed.
- Settings/Data prove the component direction before shell-wide adoption.

### Phase 2: Shell And Information Architecture Redesign

Goal: make the app easier to understand before changing individual panels.

Tasks:

- Simplify `DesktopTitlebar.vue`: identity, global command/search, task center, settings, primary action, window controls.
- Simplify `VueWorkspaceShell.vue`: one major workspace nav plus one workflow pipeline within Research.
- Replace the current context strip with a compact `RunContextBar.vue`.
- Rework `RunFooterPanel.vue` into `ActionStatusBar.vue` with one primary action, progress, and blocked reason.
- Preserve split layout and `splitFraction`, but make collapse/expand controls easier to discover.
- Move posture/debug/advanced diagnostics behind Settings, Task Center, or command palette instead of permanently visible chrome.

Files:

- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/stores/workflowStore.ts`
- `web/src/utils/core/workflowModel.ts`

Tests:

- Update `VueWorkspaceShell.test.ts`, `DesktopTitlebar.test.ts`, `RunFooterPanel.test.ts`.
- Update `web/e2e/browser/shell-ux.spec.ts`.
- Add keyboard assertions for the new major nav and workflow pipeline.

Acceptance:

- Orientation remains clear with less vertical chrome.
- A user can reach every current workspace and workflow step by keyboard.
- Primary action and blocked reason are unambiguous.

### Phase 3: Workflow Panel Streamlining

Goal: make each workflow stage present the intelligent default path first, with advanced capability still available.

Tasks:

- Data: combine CSV/Polygon/cache selection into a guided source chooser, then show dataset profile and schema after load.
- Configure/Optimize: make strategy, route, objective, range, and advanced sweep settings progressive. Surface recommended defaults and explain unavailable GPU paths only when relevant.
- Validate: split setup from results. Default to a recommended validation pack, with advanced bootstrap/reopt/cross-dataset settings in disclosure panels.
- Analyze: make Overview a real decision dashboard. Top, Drilldown, Pareto, and Heatmap should share selected candidate context.
- Finalize/Meta: rename and organize around artifacts, model confidence, stage 5 ranking, and policy evaluation.
- Library/Aggregate: make saved-run review a first-class comparison surface with clear actions to load, compare, rerun, export, reveal.
- Live: keep live execution conservative and broker-safety-first, but align visual hierarchy with the rest of the app.

Files:

- `DataWorkspacePanel.vue`
- `OptimizeWorkspacePanel.vue`
- `StrategyWorkspacePanel.vue`
- `AdvancedWorkspacePanel.vue`
- `ValidationWorkspacePanel.vue`
- `OverviewWorkspacePanel.vue`
- `TopWorkspacePanel.vue`
- `HistoryWorkspacePanel.vue`
- `MetaWorkspacePanel.vue`
- `ExecutionWorkspacePanel.vue`

Tests:

- Focused component tests per migrated panel.
- Browser UX journey for data -> optimize -> analyze empty/result transitions.
- Browser UX journey for library reopen and compare.

Acceptance:

- Empty states always show a useful next action.
- Advanced backend controls remain reachable but no longer dominate the default path.
- Results and validation are easier to interpret without reading raw metadata blocks.

### Phase 4: TradingView Lightweight Charts Modernization

Goal: create a shared, reliable chart experience for price, trades, indicators, and execution review.

Tasks:

- Expand `useLightweightChart.ts` or replace it with a chart runtime module that supports multiple independent chart instances.
- Remove singleton chart runtime from `executionChartService.ts`.
- Move chart DOM ownership into Vue chart components.
- Create a shared price chart component for Drilldown and Live Execution.
- Support synchronized theme colors, resize, fit content, marker updates, selected trade/candidate state, and loading/error states.
- Provide keyboard-accessible marker navigation through companion controls/table handoffs.
- Keep backend price bar fetching via existing Tauri commands.

Files:

- `web/src/composables/useLightweightChart.ts`
- `web/src/services/executionChartService.ts`
- `web/src/services/analysisService.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- New `web/src/visualizations/*`
- New chart components under `web/src/components/visualizations/*`

Tests:

- Expand `useLightweightChart.test.ts`.
- Expand `executionChartService.test.ts` for model-only behavior.
- Add component tests for chart host lifecycle and marker updates with mocked `lightweight-charts`.
- Add browser UX coverage for chart fallback and execution review surface.

Acceptance:

- Multiple chart instances can mount/unmount without singleton leakage.
- Drilldown and live execution share chart behavior and theme.
- Chart unavailable states are truthful and action-oriented.

### Phase 5: Heatmap And Pareto Renderer Redesign

Goal: turn heatmap and Pareto from service-owned canvases into testable, interactive Vue visualization components.

Tasks:

- Extract heatmap parsing, plateau scoring, range scaling, marker placement, and tooltip text from `analysisService.ts` into `heatmapModel.ts`.
- Create `HeatmapCanvas.vue` that owns canvas refs, resize, DPR scaling, pointer handlers, keyboard selection, and export PNG.
- Add a textual cell summary panel for hover/keyboard-selected cells.
- Add a small accessible table/list fallback for selected or top heatmap cells.
- Make objective/plateau mode clearer: raw objective surface vs derived robustness surface.
- Extract Pareto point generation and axis modeling into `paretoModel.ts`.
- Create `ParetoScatter.vue` with keyboard point navigation and selected-point summary.
- Consider OffscreenCanvas only after model/component split, and only if baseline rendering performance is inadequate.

Files:

- `web/src/services/analysisService.ts`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- New `web/src/visualizations/heatmapModel.ts`
- New `web/src/visualizations/paretoModel.ts`
- New `web/src/components/visualizations/HeatmapCanvas.vue`
- New `web/src/components/visualizations/ParetoScatter.vue`

Tests:

- Move current heatmap service tests into model tests where possible.
- Add component tests for render state, mode switching, export enablement, selection, keyboard handoff.
- Add browser UX spec for heatmap/Pareto empty and ready states using mocked run payloads.

Acceptance:

- Heatmap/Pareto rendering no longer depends on global DOM ID lookup from services.
- Pointer and keyboard interactions route to the same selected-cell/selected-point model.
- Export PNG still works.
- Top Results remains the exact retained-candidate path.

### Phase 6: Dense Tables And Candidate Review

Goal: make result triage faster and less visually noisy.

Tasks:

- Standardize virtualized table shell across Top, Validation, History, Execution, Meta.
- Keep column visibility persistence but make controls smaller and predictable.
- Add pinned selection summary, compare tray, and candidate decision notes as reusable patterns.
- Improve metric formatting and semantic tones: profit, risk, drawdown, validation pass/fail, route changes.
- Add quick filters that match real review tasks: profitable, drawdown bounded, validation admitted, route fallback, robust candidates.

Files:

- `TopWorkspacePanel.vue`
- `ValidationWorkspacePanel.vue`
- `HistoryWorkspacePanel.vue`
- `ExecutionWorkspacePanel.vue`
- `MetaWorkspacePanel.vue`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/composables/useVirtualTableRows.ts`

Tests:

- Existing virtualization tests stay required.
- Add common table shell tests.
- Add browser UX dense-surface review spec with seeded rows.

Acceptance:

- Large tables remain performant.
- Candidate selection is visible across charts/tables.
- Column management is available without dominating default layout.

### Phase 7: Settings, Perspectives, And Personalization

Goal: use existing settings features to support the redesigned UX rather than adding parallel state.

Tasks:

- Reframe Settings around Appearance, Workbench, Storage, Notifications, Retention, Diagnostics.
- Make `WorkstationPerspective` a visible power feature: save/restore layouts for research, validation, live, debug.
- Add density preview and make density affect the new UI primitives consistently.
- Keep advanced controls toggles, but tie them to progressive disclosure in panels.

Files:

- `SettingsWorkspacePanel.vue`
- `settingsStore.ts`
- `workflowStore.ts`

Tests:

- Settings action tests.
- Perspective restore tests.
- Browser UX test for saving/applying a perspective if practical in mocked mode.

Acceptance:

- Users can keep a streamlined default and still save power-user workspaces.
- Settings no longer reads like a diagnostics dump by default.

### Phase 8: Verification And Rollout

Goal: ship the redesign safely over multiple PRs without losing backend access.

Required gates for most phases:

- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`

Required gates for shell/navigation/panel UX phases:

- `npm --prefix web run a11y:smoke:managed`
- `npm --prefix web run test:browser:ux`
- Relevant manual slice from `web/e2e/accessibility-review-matrix.md`

Required gates for Tauri/native titlebar, window, notifications, open/reveal path, or packaged behavior:

- `npm --prefix web run test:desktop:smoke`
- Packaged smoke when release-facing behavior changes.

Recommended visual evidence:

- `npm --prefix web run baseline:ui`
- Save before/after screenshots under `output/playwright/frontend-review/` or the existing baseline directory.
- Record notable visual/a11y observations under `output/manual_a11y_review/`.

Known local prerequisite:

- Playwright browser binaries or an approved installed browser channel must be available for visual/browser automation. If missing, install browsers outside the normal code change or run the existing WDIO desktop smoke path.

## Rollout Strategy

Use feature-sized PRs rather than a single rewrite:

1. Add design primitives and migrate one panel.
2. Redesign shell chrome and action/status bar.
3. Migrate data/configure/optimize panels.
4. Migrate overview/top/validation dense review patterns.
5. Refactor Lightweight Charts into shared runtime and components.
6. Refactor heatmap/Pareto into visualization models and components.
7. Polish settings, perspectives, task center, command palette, and live execution.
8. Run full browser/desktop/a11y/baseline verification and resolve regressions.

Each PR should include:

- A visible user-facing goal.
- Explicit list of backend actions preserved.
- Focused tests near touched components/services.
- Screenshot or baseline evidence for layout-heavy changes.
- Manual a11y note when navigation, charting, dense tables, overlays, or native shell behavior changes.

## Risks And Mitigations

- Risk: broad UI changes break backend command access.
  - Mitigation: maintain a backend-action reachability checklist and command palette coverage.

- Risk: service-to-component visualization refactor changes analysis behavior.
  - Mitigation: extract model functions first, preserve existing tests, add golden model tests for heatmap/Pareto outputs.

- Risk: chart rendering regressions are not caught by unit tests.
  - Mitigation: browser UX specs with mocked payloads plus screenshot baselines.

- Risk: accessibility worsens as visual density improves.
  - Mitigation: keep keyboard paths, ARIA labels, manual review matrix, and add chart/table keyboard selection tests.

- Risk: performance regresses with richer components.
  - Mitigation: keep virtualization, track baseline UI metrics, avoid unnecessary reactive deep objects, and profile large tables/charts before optimizing.

## Success Criteria

- A user can complete the existing backend workflow with fewer ambiguous choices: load data, configure/run, review top candidates, inspect charts/heatmap, validate, export or move to live review.
- Every current backend capability remains reachable.
- The first viewport shows the app state and next action without redundant chrome.
- Result surfaces are visually denser but easier to scan.
- Lightweight chart, heatmap, and Pareto rendering are component-owned and testable.
- Keyboard users can navigate shell, tables, overlays, and chart alternatives without losing orientation.
- The frontend passes typecheck, unit tests, build, browser UX, accessibility smoke, and desktop smoke for native-touching changes.
