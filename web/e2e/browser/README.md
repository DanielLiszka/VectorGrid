# Browser Workflow QA

Deterministic mocked-Tauri browser interaction coverage for the built frontend.

## Purpose

This suite fills the gap between:

- component and unit tests for exact interaction contracts
- broad browser scripts such as `a11y:smoke` and `baseline:ui`
- the real-shell WDIO desktop smoke harness

Browser workflow QA should prove app-level journeys that are easier to express in a real browser than in component tests, while staying lighter and more deterministic than desktop smoke. The broader QA lane now includes action-selector inventory, responsive workflow checks, shell layout regression budgets, and regression repro specs.

Scenario fixtures are strict in both directions: unexpected mocked Tauri invokes fail in the mock layer, and any fixture `expectedInvokes` entry must be observed before the workflow can pass.

Manual accessibility review guidance for the same non-options surfaces lives in [accessibility-review-matrix.md](../accessibility-review-matrix.md). Use browser review as the default manual slice unless the change crosses into native desktop behavior.

## Run locally

From the repo root:

```powershell
npm --prefix web run test:browser:ux
npm --prefix web run test:e2e:workflow
npm --prefix web run test:e2e:qa
npm --prefix web run test:e2e:action-coverage
npm --prefix web run test:e2e:responsive
npm --prefix web run test:e2e:regressions
npm --prefix web run test:e2e:visual
npm --prefix web run test:e2e:explore
npm --prefix web run check:qa-static
npm --prefix web run check:qa-report
npm --prefix web run a11y:smoke:wcag
npm --prefix web run perf:ui:budget:capture
powershell -File scripts/run_frontend_e2e_gate.ps1
```

The workflow commands:

1. builds the frontend
2. starts a local Vite preview host
3. runs Playwright browser UX specs against the built app in mocked-Tauri mode

`test:browser:ux` is kept as a compatibility alias for `test:e2e:workflow`. The build-once frontend E2E gate and `test:e2e:qa` use the sliced QA lane: workbench smoke, action coverage, and core workflow coverage by default, with responsive/regression/exploratory coverage added through explicit gate switches.

Optional additive checks:

- `check:qa-static` validates that source `data-qa-action` IDs stay represented in the manifest, writes a selector inventory report, verifies curated artifact fixtures remain Polygon.io-only, and checks that Tauri capabilities/CSP do not widen accidentally.
- `check:qa-report` runs the static QA lane, verifies the latest UI perf budget, and writes `output/qa/local-qa-summary.md`.
- `test:e2e:visual` compares a small set of high-signal shell screenshots against Playwright snapshots. Refresh intentional UI changes with `npm --prefix web run test:e2e:visual:update`.
- `a11y:smoke:wcag` adds color contrast plus WCAG 2.2-focused target-size and keyboard focus checks on top of the axe smoke scan.
- `perf:ui:budget:capture` refreshes the UI baseline capture and verifies coarse cold-start, TTI, heap, capture-runtime, and synthetic live-chart event budgets.
- `powershell -File scripts/run_frontend_e2e_gate.ps1 -IncludeQaStatic -IncludeVisualBaselines -IncludeWcag22Smoke -IncludeBaselineUi -IncludeUiPerfBudget -IncludeQaReport` runs these optional QA additions after a build.

When a UI change intentionally renames, removes, or relocates an action, update `qa/actionCoverageManifest.ts` and refresh affected workflow/visual baselines in the same change. The static check should fail fast for source actions that moved without manifest updates, while the runtime action-coverage spec catches dynamic/conditional controls.

## Contribution Rules

- Add a stable `data-qa-action` to every new product-owned command button, action menu item, or mode-switching control.
- Add or update `qa/actionCoverageManifest.ts` in the same change. Required actions need at least one workflow id in `coveredBy`; intentionally uncovered actions need an owner, reason, and waiver expiry.
- Add new mocked Tauri responses to a named scenario fixture. Keep fixtures strict and provider-specific; options fixtures must remain Polygon.io-only.
- Add an E2E regression workflow for every user-discovered bug that can be reproduced in the browser harness.
- Refresh visual snapshots only for intentional UI changes, and review the changed PNGs before handoff.
- Use `npm --prefix web run check:qa-report` before handoff for local-only QA summary evidence.

## What this suite should own

- shell-level keyboard navigation in mocked-Tauri mode
- command-palette interaction journeys
- overlay focus-return journeys
- one representative dense-surface browser interaction when a UI modernization slice changes forms or results
- action manifest drift for visible `data-qa-action` controls
- static manifest drift for source-level `data-qa-action` controls
- shell layout budgets for dense desktop, laptop, and compact mobile viewports
- a small visual baseline set for representative desktop/mobile shell states
- curated saved-run/options artifact fixture packs that exercise realistic reopen and follow-up validation paths
- known regression repros that should be exercised by clicking the actual UI
- non-destructive exploratory navigation sweeps across safe shell and overlay actions
- command observation for user journeys where a button must dispatch a specific Tauri command

## What this suite should not own

- broad accessibility scanning
- broad screenshot/performance artifact capture
- real native-shell proof
- broker/live-service integration
- large snapshot matrices

## Layer boundary

- component and unit tests:
  - exact interaction contracts
  - state-driven rendering rules
  - local keyboard/focus behavior
- browser UX smoke:
  - deterministic app-level keyboard and focus journeys in mocked-Tauri mode
- manual accessibility review matrix:
  - representative browser-first keyboard, naming, dense-surface, and reopen-review slices
- desktop smoke:
  - real Tauri shell launch/readiness and native-shell integrity
- `baseline:ui`:
  - broad review evidence and coarse performance artifacts
