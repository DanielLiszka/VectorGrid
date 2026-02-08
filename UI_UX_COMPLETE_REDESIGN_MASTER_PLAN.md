# UI Complete Redesign Research + Master Plan

## 1) Purpose

This document defines how to fully redesign the desktop UI so it is:

- Much easier for first-time users.
- Faster for repeat users.
- Visually polished and desktop-grade.
- Fully backward-compatible in capability (no feature removal).

Primary target flow:

1. Download data via Polygon.
2. Run backtest optimization.
3. Validate that specific optimization.
4. Run post-analysis across many runs to produce aggregate conclusions for the same strategy and timeframe context.

## 2) Non-Negotiables

- Keep all current functionality available.
- Keep CPU-only environments fully supported.
- Preserve backend command behavior (`invoke(...)`) and core run artifacts.
- Avoid breaking keyboard-centric usage (`Ctrl+K`, run/validate/export shortcuts).
- Maintain existing power-user capabilities: distributed, batch, meta, debug.

## 3) Current-State Findings (From Existing UI)

Observed structure:

- Workflow tabs: `data`, `optimize`, `validate`, `analyze`, `history`, `settings` (`dist/app.js`).
- Controls tabs: `data`, `opt`, `strategy`, `advanced`, `distributed` (`dist/index.html`).
- Results tabs: `overview`, `top`, `pareto`, `validate`, `drill`, `heatmap`, `history`, `meta`, `debug` (`dist/index.html`).

Main usability issues:

- Navigation is split across multiple tab systems, so users must learn structure before completing tasks.
- Too many visible controls at once, especially in optimize/advanced/validation sections.
- Heavy explanatory text is always visible, creating visual noise and weak information hierarchy.
- Related flows are separated across surfaces (single-run workflow vs run library vs meta aggregation).
- Power features are exposed too early for novice users.

What already works well:

- Strong functional coverage end-to-end.
- Good command palette and quick actions.
- Existing status/progress/toast mechanisms.
- Existing artifact model (history/saved runs/meta artifacts).

## 4) Redesign Strategy (High Level)

Use a dual-surface model:

1. Guided Workspace (default)
2. Pro Workspace (advanced tools)

Guided Workspace becomes the default home for 90% of usage:

- Data -> Optimize -> Validate -> Analyze.

Pro Workspace contains advanced and low-frequency tools:

- Distributed runs, debug JSON, deep parameter controls, specialist toggles.

This preserves all features while dramatically lowering cognitive load in the default path.

## 5) Information Architecture (Target)

Top-level navigation (single primary nav bar):

1. `Workflow`
2. `Run Library`
3. `Aggregate Analysis`
4. `Pro Tools`
5. `Settings`

### 5.1 Workflow (primary day-to-day)

Sub-steps displayed as a stepper:

1. Data Source
2. Strategy Setup
3. Optimization
4. Validation
5. Summary

### 5.2 Run Library

- Saved runs list, filtering, tags, rerun, apply-to-form, open report, reveal file.
- Compare baseline/current.

### 5.3 Aggregate Analysis

- Batch generation pipeline.
- Meta analysis over saved artifacts.
- Holdout evaluation.
- Cross-run comparison and shortlist output.

### 5.4 Pro Tools

- Distributed worker/leader flows.
- Deep optimization controls.
- Debug JSON and diagnostics.

### 5.5 Settings

- Theme/effects.
- Storage/cache paths.
- Defaults and preferences.

## 6) User Flow Definitions

### 6.1 First-time flow (novice)

1. Open app -> lands in `Workflow / Data Source`.
2. Click `Polygon Download`.
3. Select symbol/timeframe/date range -> `Download & Load`.
4. Click `Continue`.
5. Review compact strategy presets.
6. Click `Run Optimization`.
7. See live progress with clear stage labels.
8. See best result card.
9. Click `Run Validation`.
10. See pass/fail robustness summary.
11. Save run with suggested tags.

### 6.2 Repeat flow (practitioner)

1. Open app -> return to last workspace.
2. Use `Ctrl+K` for quick action.
3. Load cached dataset or saved run.
4. Execute optimization/validation quickly.
5. Export report.

### 6.3 Research flow (portfolio/meta)

1. Go to `Aggregate Analysis`.
2. Select strategy + timeframe filter + required tag.
3. Run batch artifact generation (optional).
4. Run meta analysis.
5. Review ranked robust groups.
6. Compare against prior meta baseline.
7. Export report and shortlist.

Note on wording:

- The UI should clarify whether users mean:
  - same candle timeframe across many symbols/date windows, or
  - cross-timeframe comparison (1m vs 5m vs 1h).
- Both modes can be supported with explicit toggles.

## 7) Feature Preservation Matrix

| Current capability | Keep? | New home | UX treatment |
|---|---|---|---|
| CSV load/browse/recent | Yes | Workflow -> Data Source | Primary action block |
| Polygon download/cache/manage | Yes | Workflow -> Data Source | Primary modal with concise presets |
| Timeframe override/apply | Yes | Workflow -> Data Source | Secondary controls in collapsible "Data Options" |
| Strategy/backend/ranges | Yes | Workflow -> Strategy Setup | Essential controls visible, advanced collapsed |
| GPU status/parity check | Yes | Pro Tools + context card | Hidden by default; surfaced when GPU selected |
| Advanced optimization controls | Yes | Pro Tools -> Optimization Advanced | Moved out of default view |
| Run optimization/cancel/progress | Yes | Workflow -> Optimization | Sticky action bar + stage progress |
| Overview/Top/Pareto/Heatmap/Drilldown | Yes | Workflow -> Summary + Analyze | Organized as analysis sub-tabs |
| Validation full option set | Yes | Workflow -> Validation | Basic options default, full options expandable |
| Validation exports | Yes | Workflow -> Validation | Export cluster |
| History, compare, rerun | Yes | Run Library | Core table + detail panel |
| Saved runs/tagging/delete | Yes | Run Library | Bulk actions + item drawer |
| Batch runs | Yes | Aggregate Analysis | Guided batch panel |
| Meta selection/holdout/grouping | Yes | Aggregate Analysis | Structured form with presets |
| ML confidence meta tooling | Yes | Aggregate Analysis -> Advanced | Hidden behind "Experimental" section |
| Distributed worker/leader | Yes | Pro Tools -> Distributed | Advanced panel |
| Debug output | Yes | Pro Tools -> Debug | Text view with copy/export |
| Theme/window effects | Yes | Settings | Compact appearance panel |

## 8) Progressive Disclosure Rules

- Default screen only shows controls needed for current step.
- Advanced controls appear behind one explicit "Advanced" section per step.
- Experimental controls are grouped under "Experimental".
- Inline helper text is short and contextual; long explanations move to tooltips or side help.
- Empty states always include one primary next action.

## 9) Visual + Content System Direction

Visual direction:

- Clean, light-first desktop surface with strong contrast and restrained accent color.
- Distinct hierarchy using spacing, card depth, and typographic scale.
- Fewer simultaneously visible panels.

Typography/content direction:

- Replace long paragraphs with short labels and one-line helper copy.
- Use consistent terminology:
  - "Optimization Run"
  - "Validation"
  - "Aggregate Analysis"
  - "Run Library"
- Keep technical jargon available but de-emphasized until needed.

Motion/feedback:

- Subtle transitions when moving between workflow steps.
- Clear run lifecycle states: `Ready`, `Running`, `Completed`, `Failed`, `Cancelled`.
- Toasts only for significant outcomes, not every minor state update.

## 10) UX Research Plan

### 10.1 Research questions

1. Can a new user complete Polygon -> Optimize -> Validate without guidance?
2. Do users correctly understand where aggregate analysis lives?
3. Which controls are used frequently vs rarely?
4. Which labels cause confusion (for example "Meta", "Holdout", "Cross-ticker")?

### 10.2 Methods

- Heuristic review of current UI (expert pass).
- Task-based usability tests (5-8 participants, two cohorts):
  - novice quant/trader users,
  - existing power users.
- Instrumented event telemetry for local dev sessions (non-sensitive UI interactions only).
- A/B layout review with static mockups before implementation.

### 10.3 Core usability tasks

1. Download and load a Polygon dataset.
2. Run optimization with defaults.
3. Validate and export report.
4. Save/tag run and rerun from library.
5. Run aggregate analysis filtered by strategy/timeframe/tag.
6. Find and use distributed tools.

### 10.4 Success thresholds

- >= 90% task completion for primary workflow without moderator help.
- <= 30 seconds median to find "Run Validation" after optimization completes.
- >= 80% users correctly locate aggregate analysis on first attempt.
- Reduced number of visible controls in primary workflow by >= 40% at rest state.

## 11) Technical Implementation Plan

Constraints:

- UI is static files in `dist/` (`index.html`, `app.css`, `app.js`).
- Need safe incremental migration with minimal backend disruption.

Approach:

1. Introduce a view-controller layer for new IA while keeping existing IDs functional.
2. Add adapter functions so existing handlers can be reused while UI containers change.
3. Move to component-like DOM sections (still vanilla JS) for each top-level workspace.
4. Introduce content tokens (labels/help text) in a single JS map for consistency.
5. Keep command palette action registry as a first-class navigation fallback.

Compatibility strategy:

- During transition, support both old and new tab mappings behind a feature flag.
- Preserve `localStorage` keys where practical and add migration logic for renamed state.
- Keep export/report/history artifact formats unchanged.

## 12) Delivery Phases

### Phase 0 - Baseline and instrumentation

- Capture before screenshots and completion timings.
- Lock regression checklist baseline (`UI_REGRESSION_CHECKLIST.md`).
- Add lightweight UX telemetry hooks in dev mode.

### Phase 1 - IA shell and navigation rewrite

- Implement new top-level nav and workspace containers.
- Route existing actions into new containers with minimal visual redesign.
- Keep all legacy handlers operational.

### Phase 2 - Workflow redesign (core path)

- Build stepper: Data Source -> Strategy Setup -> Optimization -> Validation -> Summary.
- Apply progressive disclosure.
- Introduce concise status strip and run context card.

### Phase 3 - Library and aggregate analysis redesign

- Build Run Library table/detail layout.
- Build Aggregate Analysis page with:
  - batch setup,
  - meta settings,
  - results ranking + compare.

### Phase 4 - Pro tools consolidation

- Move distributed/debug/deep controls to Pro Tools.
- Ensure discoverability via command palette and direct links from relevant contexts.

### Phase 5 - Visual polish pass

- Final spacing/typography/token tuning.
- Accessibility and keyboard audit.
- Empty/loading/error state polish.

### Phase 6 - Hardening and release

- Full regression checklist run.
- Smoke tests for Polygon + optimize + validation + meta.
- Performance pass on large tables and chart rendering.

## 13) QA and Acceptance Criteria

Must-pass acceptance:

1. User can complete Polygon -> Optimize -> Validate -> Export in one linear flow.
2. Aggregate analysis across many runs is explicit and easy to access.
3. No existing capabilities are removed.
4. No critical regression in run artifacts, report export, or history behavior.
5. UI remains responsive at 1280x720 and common laptop resolutions.

Manual QA:

- Continue using `UI_REGRESSION_CHECKLIST.md`.
- Add a redesign-specific checklist once Phase 2 is complete.

## 14) Risks and Mitigations

Risk: Feature discoverability drops for power users.

- Mitigation: Pro Tools workspace + command palette shortcuts + deep links.

Risk: Refactor breaks existing event wiring.

- Mitigation: preserve IDs during transition and migrate in controlled slices.

Risk: Redesign increases implementation complexity in plain JS.

- Mitigation: introduce small modular controller files while staying in current stack.

Risk: New terminology confuses existing users.

- Mitigation: transition labels with dual wording for one release cycle.

## 15) Open Decisions

1. Should aggregate analysis live as a separate top-level workspace, or under Run Library as a tab?
2. Should cross-timeframe analysis be first-class in V1 redesign, or a V2 extension?
3. Should distributed tooling be hidden by default unless enabled in settings?
4. Should we keep window effects in the main header or move fully into Settings?

## 16) Immediate Next Step

Produce low-fidelity wireframes for:

1. Workflow stepper page.
2. Run Library page.
3. Aggregate Analysis page.
4. Pro Tools page.

Then validate the wireframes against the feature preservation matrix before any major HTML restructuring.
