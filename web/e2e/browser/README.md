# Browser UX Smoke

Deterministic mocked-Tauri browser interaction coverage for the built frontend.

## Purpose

This suite fills the gap between:

- component and unit tests for exact interaction contracts
- broad browser scripts such as `a11y:smoke` and `baseline:ui`
- the real-shell WDIO desktop smoke harness

Browser UX smoke should prove small app-level journeys that are easier to express in a real browser than in component tests, while staying lighter and more deterministic than desktop smoke.

Manual accessibility review guidance for the same non-options surfaces lives in [accessibility-review-matrix.md](../accessibility-review-matrix.md). Use browser review as the default manual slice unless the change crosses into native desktop behavior.

## Run locally

From the repo root:

```powershell
npm --prefix web run test:browser:ux
```

This command:

1. builds the frontend
2. starts a local Vite preview host
3. runs Playwright browser UX specs against the built app in mocked-Tauri mode

## What this suite should own

- shell-level keyboard navigation in mocked-Tauri mode
- command-palette interaction journeys
- overlay focus-return journeys
- one representative dense-surface browser interaction when a UI modernization slice changes forms or results

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
