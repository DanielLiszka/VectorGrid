# UI QA Review Tracker

This tracker is for the app-wide UI QA and polish pass.

## Status

- [x] Create UI QA documentation folder.
- [x] Define presentation standard.
- [x] Define workflow visibility model.
- [ ] Inventory current primary screens and workflow states.
- [ ] Review the first-load research screen against the standard.
- [ ] Implement the first coherent screen-state cleanup.
- [ ] Update affected unit/component tests.
- [ ] Update affected Playwright tests.
- [ ] Capture before/after screenshots.
- [ ] Apply the same standard to the next workflow surface.

## Initial Finding

Screenshot source:

- `docs/Manual_QA/Screenshot 2026-05-08 202850.jpg`

Observed issue:

- The screen is functionally dense and domain-specific, but it exposes too many independent widget states and too much meta/diagnostic information at once.

Primary classification:

- `State clarity`
- `Visual hierarchy`
- `Copy density`
- `Progressive disclosure`
- `Workflow relevance`

Current visible state conflict:

- `Validation not run`
- `Completed`
- `Latest result`
- `Tasks completed`
- `no_data`
- `Load data first`
- `0 rows`
- `No rows`

Preferred dominant state:

> Validation blocked: load a dataset. A previous optimization result is available, but validation needs active candle data.

## First Implementation Target

Target screen:

- Research workspace, validation step, missing-dataset state.

Goal:

- Make the blocked validation state decisive without removing validation, optimization, GPU, export, or diagnostic capability.

Expected direction:

- Center panel shows a dominant blocked state.
- Full validation settings collapse while the dataset prerequisite is missing.
- Left optimization controls collapse into a selected-strategy or previous-result snapshot while validation is active.
- Right inspector shows selected candidate, requirements, and missing dataset reason without repeating every local widget state.
- Raw status values move to diagnostics or tooltip/details.

Candidate public copy:

- `Validation blocked`
- `Load a dataset before running validation.`
- `Previous optimization available: ALMA 38 vs ALMA 252.`
- `Dataset: missing`
- `Rows: 0`
- `Gate policy: PSR >= 0.80, DSR >= 0.60, CSCV <= 16`

Candidate diagnostic/details copy:

- `raw_recommendation=no_candidate_survived`
- `strategy=DoubleMa`
- `combos=38605`
- `runtime_ms=...`
- `cuda_available=true`
- `gpu_speedups.csv policy details`

## Review Checklist

Use this checklist for each major screen.

### State

- [ ] One canonical state drives visible status.
- [ ] Conflicting local widget states are removed or subordinated.
- [ ] Blocked, ready, running, completed, and failed states have distinct copy.
- [ ] Previous results are clearly labeled as previous, not current readiness.

### Workflow Relevance

- [ ] Only active-workflow controls are visible by default.
- [ ] Unrelated modes are reachable but not occupying primary space.
- [ ] Previous-step controls collapse into snapshots after the user advances.
- [ ] Advanced settings are collapsed unless actively being edited.

### Visual Hierarchy

- [ ] One dominant message/status area.
- [ ] One primary action.
- [ ] Secondary actions are visually quiet.
- [ ] Inspector content adds value instead of repeating main-panel state.
- [ ] Nested cards are reduced.

### Copy

- [ ] Public UI avoids raw enum/request/internal names.
- [ ] Helper text is short and task-oriented.
- [ ] Definitions and rationale move to help/tooltip/details.
- [ ] Diagnostics remain available but clearly diagnostic.

### Interaction

- [ ] Disabled controls have an understandable reason.
- [ ] Hidden/collapsed content remains discoverable when relevant.
- [ ] Keyboard and screen-reader access remain intact.
- [ ] Tooltips do not contain the only critical instruction.

### Verification

- [ ] Component tests updated for new visible copy/state.
- [ ] Store/service tests updated if state derivation changes.
- [ ] Playwright selectors updated only when necessary.
- [ ] E2E tests cover the canonical blocked/ready/completed state.
- [ ] Visual screenshot evidence captured before and after.

## Screen Inventory

- [ ] First-load research shell
- [ ] Data source step
- [ ] Conventional optimization setup
- [ ] Optimization results / top candidates
- [ ] Heatmap and 3D heatmap views
- [ ] Candidate detail / Lightweight Chart drilldown
- [ ] Validation setup
- [ ] Validation results / reports
- [ ] Meta-analysis workspace
- [ ] ML Signal / CatBoost workspace
- [ ] Options expression workspace
- [ ] Library/history workspace
- [ ] Live/paper execution workspace
- [ ] Portfolio workspace
- [ ] Settings and diagnostics workspace
- [ ] Global titlebar/action bar/status bar
- [ ] Right inspector
- [ ] Bottom status/action bar

## Manual QA Log

Add entries here as screenshots are reviewed.

### 2026-05-08 Screenshot 202850

- Source: `docs/Manual_QA/Screenshot 2026-05-08 202850.jpg`
- Surface: Research workspace, validation step, missing dataset after previous optimization.
- Verdict: Not broken, but too much redundant state and visible technical metadata.
- Proposed treatment: Establish canonical blocked validation state; collapse unavailable validation machinery; move diagnostics and raw backend strings out of primary UI.
