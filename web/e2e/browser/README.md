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
npm --prefix web run test:e2e:explore
powershell -File scripts/run_frontend_e2e_gate.ps1
```

The workflow commands:

1. builds the frontend
2. starts a local Vite preview host
3. runs Playwright browser UX specs against the built app in mocked-Tauri mode

`test:browser:ux` is kept as a compatibility alias for `test:e2e:workflow`. The build-once frontend E2E gate and `test:e2e:qa` use the sliced QA lane: workbench smoke, action coverage, and core workflow coverage by default, with responsive/regression/exploratory coverage added through explicit gate switches.

## What this suite should own

- shell-level keyboard navigation in mocked-Tauri mode
- command-palette interaction journeys
- overlay focus-return journeys
- one representative dense-surface browser interaction when a UI modernization slice changes forms or results
- action manifest drift for visible `data-qa-action` controls
- shell layout budgets for dense desktop, laptop, and compact mobile viewports
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
