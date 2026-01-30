# UI Regression Checklist (Manual)

Use this after UI changes to confirm we didn’t break core workflows.

## Setup

- Start app: `cargo run -p ta_desktop_demo_app --release`
- Ensure there is at least one local CSV dataset available.
- (Optional) Configure Polygon API key if testing Polygon flows.

## Theme / Layout

- Theme selector works: Light ↔ Dark ↔ System (persists after restart).
- Controls panel resize works (drag divider) and persists after restart.
- Controls panel collapse/expand works and does not break tab state.
- Status area remains readable (no clipped text) at 1280×720 and smaller.

## Data tab

- Browse CSV opens picker; selected path appears in `CSV path`.
- Load CSV loads dataset and populates:
  - `Loaded data id`
  - `Rows`
  - `Dataset`
  - `Timeframe / range`
  - field badges + hints
- Clear resets dataset + run state (Overview shows empty state again).
- Recent CSV list updates after loading.

## Polygon modal

- Open Polygon modal from Data tab.
- Save/Clear API key works.
- Download & Load:
  - creates/loads CSV from cache
  - closes modal
  - updates dataset meta
- Cross-ticker dataset list refreshes and actions (Load/Update/Reveal/Validate/Repair) still work.

## Optimize tab (Double MA)

- Select Double MA strategy.
- Switch backend among `CpuOnly`, `GpuOnly`, `GpuKernel`, `Auto` (if available).
- Changing MA types or MA source updates constraints correctly.
- Run optimization:
  - RUN button disables and Cancel enables
  - Status updates (and toast on success/failure)
  - Results tabs populate (Overview + Top + Heatmap/Drilldown if available)

## Optimize tab (Composite)

- Select Composite CPU and Composite MO strategies.
- Verify backend/mode controls disable appropriately for composite strategies.
- Run optimization produces results and populates Overview/Top/Pareto.

## Validation

- Validate tab hint is correct for current strategy.
- Run validation (composite only) updates validation output + tables.
- Auto-run/auto-export toggles behave as expected.

## Exports / Artifacts

- Export Top CSV and Export JSON work from Overview.
- Open latest report/meta report buttons enable when paths exist.
- Reports folder opens; saved report exists.
- Saved runs list refreshes, tag editing persists, delete works.

## History / Compare

- History select lists runs; Apply / Re-run operate correctly.
- Compare baseline selection:
  - Set baseline = current works
  - Clear baseline works
  - Compare output updates and remains readable

## Batch

- Batch datasets list loads and selection controls work.
- Start batch run, stop requested works, retry failed works.
- Reveal batch summary works.

## Distributed

- Worker start/stop/status works (local).
- Leader “run assets” flow works (connects to workers, aggregates).
- Leader “tiled” flow works (splits search-space, aggregates).
- Cancel distributed run works.

## Keyboard shortcuts / command palette (if enabled)

- `Ctrl+K` opens command palette; typing filters actions; Enter runs.
- `Esc` closes command palette (and Polygon modal if open).
- `Ctrl+O` triggers CSV browse.
- `Ctrl+Enter` triggers Run optimization (when available).

