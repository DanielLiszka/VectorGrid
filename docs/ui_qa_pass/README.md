# UI QA Pass

This folder tracks the app-wide UI QA and polish pass for VectorGrid.

The goal is not to remove capabilities. The goal is to present the existing capabilities through a cohesive, workflow-aware desktop interface so the app does not expose every backend function at once.

## Product Direction

VectorGrid should feel like a serious technical research instrument:

- dense but disciplined
- precise rather than decorative
- workflow-led rather than function-led
- public-facing without hiding advanced capability
- diagnostic detail available on demand instead of always visible

Avoid drifting toward either extreme:

- not a generic SaaS dashboard
- not a raw internal debug console
- not a screen-by-screen collection of unrelated fixes

## Core Problem

The app currently supports multiple substantial workflows:

- conventional backtest optimization
- validation and report generation
- meta-analysis and cross-run research
- CatBoost / ML signal research
- options-expression optimization and validation
- live/paper execution support
- library/history/replay workflows
- settings, diagnostics, GPU/runtime management, and data management

The UI should not make every workflow, backend capability, and diagnostic fact compete for attention on every screen. Each screen needs a current workflow lens, a canonical state, and progressive disclosure for advanced or unrelated details.

## Folder Contents

- `ui_presentation_standard.md`
  - app-wide visual, copy, state, and progressive-disclosure rules
- `workflow_visibility_model.md`
  - workflow-mode inventory and rules for showing, collapsing, or hiding feature groups
- `review_tracker.md`
  - checklist for the UI QA pass, screenshot triage, implementation, and verification

## Manual QA Workflow

The lightweight screenshot workflow remains valid.

Use `docs/Manual_QA/` for raw user screenshots and observations. This folder is for the analysis and implementation plan derived from those screenshots.

For each screenshot-backed finding, classify it as:

- `State clarity`
- `Visual hierarchy`
- `Copy density`
- `Progressive disclosure`
- `Workflow relevance`
- `Component consistency`
- `Layout rhythm`
- `Color semantics`
- `Public language`
- `Accessibility or interaction`

## Change Policy

Prefer presentation changes over feature removal.

When a feature is not relevant to the active workflow, first consider:

1. Hide it until the workflow requires it.
2. Collapse it into a summary.
3. Move it to the inspector.
4. Move it to diagnostics.
5. Move explanatory detail to a tooltip or help popover.
6. Keep it visible only if it directly supports the user's current task.

Downstream changes are expected. UI changes may require updates to component tests, Playwright tests, screenshots, selectors, accessibility assertions, and wording expectations.
