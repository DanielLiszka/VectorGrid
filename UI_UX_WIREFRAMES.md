# UI/UX Redesign Wireframes (Low Fidelity)

## 1) Purpose

This document translates `UI_UX_COMPLETE_REDESIGN_MASTER_PLAN.md` into low-fidelity, implementation-ready wireframes.

Goals:

- Keep all existing capabilities.
- Reduce cognitive load in the default user path.
- Define concrete screen structures before HTML/CSS/JS rewrites.

---

## 2) Global App Shell

All primary screens share one shell:

```text
+--------------------------------------------------------------------------------------+
| VectorTA Optimizer                                                Theme  Effects  ?  |
+--------------------------------------------------------------------------------------+
| Workflow | Run Library | Aggregate Analysis | Pro Tools | Settings                   |
+--------------------------------------------------------------------------------------+
| Context Strip: Dataset | Timeframe | Strategy | Backend | Last Run | Status          |
+--------------------------------------------------------------------------------------+
|                                                                                      |
|  Active workspace content                                                            |
|                                                                                      |
+--------------------------------------------------------------------------------------+
| Sticky Action Bar (contextual): Primary CTA | Secondary | Export | Command Palette  |
+--------------------------------------------------------------------------------------+
```

Notes:

- Only one top-level navigation row.
- Context strip is compact and always visible.
- Sticky action bar changes by workspace.

---

## 3) Workspace A: Workflow (Default)

### 3.1 Workflow Frame

```text
+--------------------------------------------------------------------------------------+
| Stepper: [1 Data Source] -> [2 Strategy Setup] -> [3 Optimization] -> [4 Validation]|
|          -> [5 Summary]                                                              |
+--------------------------------------------------------------------------------------+
| Left: Step content (form)                    | Right: Live context + hints          |
|                                               | - dataset summary                    |
|                                               | - strategy summary                   |
|                                               | - latest run summary                 |
+--------------------------------------------------------------------------------------+
| Back                         Continue / Run / Validate / Finish                       |
+--------------------------------------------------------------------------------------+
```

### 3.2 Step 1: Data Source

```text
+-----------------------------------------------+--------------------------------------+
| Data Source                                   | Current Data                         |
|                                               | - Symbol / file                      |
| [ Download from Polygon ] [ Load CSV ]        | - Timeframe                          |
|                                               | - Date range                         |
| Polygon Quick Form:                           | - Rows                               |
| - Market                                      | - Fields availability                |
| - Symbol                                      |                                      |
| - Timeframe                                   |                                      |
| - From / To                                   |                                      |
| [ Download and Load ]                         |                                      |
|                                               |                                      |
| Advanced (collapsed):                         |                                      |
| - Cache/storage options                       |                                      |
| - Timeframe override                          |                                      |
+-----------------------------------------------+--------------------------------------+
```

Rules:

- Show exactly two primary entry buttons (`Polygon`, `CSV`).
- Keep storage/cache controls collapsed by default.
- After load success, auto-enable Continue.

### 3.3 Step 2: Strategy Setup

```text
+-----------------------------------------------+--------------------------------------+
| Strategy Setup                                | Estimated Search Size                |
|                                               | - Pair count                         |
| Strategy: [Double MA / Composite CPU / MO]    | - Estimated runtime                  |
| Preset:   [Quick / Balanced / Deep]           | - Backend recommendation             |
| Backend:  [Auto / CPU / GPU sweep / GPU kern] |                                      |
|                                               | Compatibility                        |
| MA Fast Range: start/end/step                 | - Missing required fields            |
| MA Slow Range: start/end/step                 | - Disabled options reason            |
| MA Types: fast / slow                         |                                      |
|                                               |                                      |
| Advanced (collapsed):                         |                                      |
| - Strategy params (commission/eps/flags)      |                                      |
| - Objective/mode/sampler                      |                                      |
| - Composite extras                            |                                      |
+-----------------------------------------------+--------------------------------------+
```

Rules:

- Essentials only visible at first.
- Explain disabled controls with short inline reason.
- Keep deep sampler options under Advanced.

### 3.4 Step 3: Optimization

```text
+--------------------------------------------------------------------------------------+
| Optimization Run                                                                      |
|                                                                                       |
| Status: Running / Completed / Failed / Cancelled                                      |
| Progress: [===================-----] 72%   Stage: evaluate_grid                       |
| Metrics: evals/sec | elapsed | backend | combos | best-so-far                         |
|                                                                                       |
| [ Run Optimization ] [ Cancel ] [ Open Command Palette ]                              |
+--------------------------------------------------------------------------------------+
| Best-So-Far Card (live): score, pnl, sharpe, drawdown, params                         |
+--------------------------------------------------------------------------------------+
| Log (collapsed by default)                                                            |
+--------------------------------------------------------------------------------------+
```

Rules:

- One dominant primary CTA.
- Minimal moving text while running.
- Promote result quality, not raw logs.

### 3.5 Step 4: Validation

```text
+-----------------------------------------------+--------------------------------------+
| Validation                                    | Validation Snapshot                  |
|                                               | - candidates validated               |
| Quick mode:                                   | - robust shortlist size              |
| - Candidates                                  | - CSCV/PBO summary                   |
| - Seed                                        | - cross-ticker summary               |
| [ Run Validation ]                            |                                      |
|                                               | Export                               |
| Advanced (collapsed):                         | [ CSV ] [ JSON ] [ Report JSON ]     |
| - Walk-forward options                        | [ Report XLSX ]                      |
| - Perturbation options                        |                                      |
| - Monte Carlo options                         |                                      |
| - CSCV options                                |                                      |
| - Cross-ticker dataset picker                 |                                      |
+-----------------------------------------------+--------------------------------------+
```

Rules:

- Default view shows only commonly needed options.
- Full option parity preserved in Advanced.

### 3.6 Step 5: Summary

```text
+--------------------------------------------------------------------------------------+
| Run Summary                                                                            |
+--------------------------------------------------------------------------------------+
| Best Candidate Card      | Top Metrics (PnL, Sharpe, DD, Trades, Exposure)           |
+--------------------------------------------------------------------------------------+
| Tabs: Overview | Top | Pareto | Drilldown | Heatmap                                   |
+--------------------------------------------------------------------------------------+
| Visualization/Table area                                                               |
+--------------------------------------------------------------------------------------+
| Save Run Tags | Open Report | Reveal Files | Compare Baseline                          |
+--------------------------------------------------------------------------------------+
```

Rules:

- Keep analysis tabs here for continuity.
- Separate action buttons from data display region.

---

## 4) Workspace B: Run Library

```text
+--------------------------------------------------------------------------------------+
| Run Library                                                                            |
+--------------------------------------------------------------------------------------+
| Filters: search | tags | strategy | timeframe | date range                            |
+--------------------------------------------------------------------------------------+
| Runs Table (left, 60-70%)                 | Selected Run Panel (right, 30-40%)       |
| - ts                                       | - key metadata                            |
| - strategy                                 | - params snapshot                         |
| - timeframe                                | - validation presence                     |
| - tags                                     | - report links                            |
| - score                                    | - actions                                 |
|                                            |   [Load] [Apply] [Rerun]                  |
|                                            |   [Open Report] [Reveal] [Delete]         |
+--------------------------------------------------------------------------------------+
| Compare Row: Baseline select + summary                                                |
+--------------------------------------------------------------------------------------+
```

Rules:

- Keep bulk management in one place.
- Right panel actions reduce row-level button clutter.

---

## 5) Workspace C: Aggregate Analysis

```text
+--------------------------------------------------------------------------------------+
| Aggregate Analysis                                                                     |
+--------------------------------------------------------------------------------------+
| Subtabs: Batch Generation | Meta Selection | Compare Meta                              |
+--------------------------------------------------------------------------------------+
| Batch Generation                                                                      |
| - dataset search/filter/select                                                        |
| - tags + holdout config                                                               |
| - [Run Batch] [Stop] [Retry Failed] [Reveal Summary]                                 |
+--------------------------------------------------------------------------------------+
| Meta Selection                                                                        |
| - strategy/timeframe/tag filters                                                      |
| - min samples / top K / grouping                                                      |
| - holdout eval controls                                                               |
| - [Run Meta Analysis]                                                                 |
| - output table: robust groups + scores + params                                      |
| - export/open/reveal actions                                                          |
+--------------------------------------------------------------------------------------+
| Compare Meta                                                                          |
| - baseline meta select                                                                |
| - summary diff                                                                        |
+--------------------------------------------------------------------------------------+
```

Rules:

- Place batch and meta together because they are one pipeline.
- Keep ML-confidence controls in an "Experimental" collapse under Meta Selection.

---

## 6) Workspace D: Pro Tools

```text
+--------------------------------------------------------------------------------------+
| Pro Tools                                                                              |
+--------------------------------------------------------------------------------------+
| Cards:                                                                                |
| [Optimization Advanced] [Debug JSON]                                                  |
+--------------------------------------------------------------------------------------+
| Optimization Advanced card                                                            |
| - deep controls moved out of default workflow                                         |
+--------------------------------------------------------------------------------------+
| Debug card                                                                            |
| - raw state/view output                                                               |
| - copy/export debug payload                                                           |
+--------------------------------------------------------------------------------------+
```

Rules:

- High power, lower frequency.
- Discoverable from command palette and deep links.

---

## 7) Workspace E: Settings

```text
+--------------------------------------------------------------------------------------+
| Settings                                                                               |
+--------------------------------------------------------------------------------------+
| Appearance: theme, window effect, density                                             |
| Data + storage: cache path, market data dir, open folder                              |
| Defaults: startup workspace, default strategy preset                                  |
| Keyboard: shortcut reference                                                          |
+--------------------------------------------------------------------------------------+
```

---

## 8) Responsive Rules

- >= 1280px: dual-column where specified.
- 1024-1279px: tighter spacing, retain dual-column where possible.
- <= 1023px: single-column stack; sticky action bar persists.
- Table-heavy screens keep horizontal scroll with sticky header.

---

## 9) Interaction States (All Workspaces)

- Empty: concise explanation + one clear next action.
- Loading: skeletons for table/chart areas and progress text.
- Error: inline error summary with recovery action.
- Success: compact confirmation and next suggested action.

---

## 10) Mapping to Existing Feature Surfaces

Implementation must preserve parity with current major surfaces:

- Data loading + Polygon modal.
- Optimize + strategy + advanced controls.
- Run/cancel/progress.
- Validation full option set + exports.
- Analysis tabs (overview/top/pareto/drill/heatmap).
- History/saved runs + compare/rerun.
- Batch + meta + holdout + ML confidence.
- Debug + advanced diagnostics.

---

## 11) Build Order (Wireframe to Code)

1. Implement new shell + top nav + context strip.
2. Implement Workflow stepper with Data + Strategy + Optimization.
3. Implement Validation + Summary steps.
4. Implement Run Library.
5. Implement Aggregate Analysis.
6. Move advanced/debug into Pro Tools.
7. Finalize Settings migration.

This order keeps core workflow usable throughout migration.
