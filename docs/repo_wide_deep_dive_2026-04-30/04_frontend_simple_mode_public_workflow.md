# Frontend Simple Mode Public Workflow Implementation Plan

## Surface

Primary files:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/utils/simpleWorkflowModel.ts`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/ActionStatusBar.vue`
- `web/src/services/dataService.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/e2e/*`
- Current browser QA fixtures live under `web/e2e/browser/qa/*`; new Simple Mode specs should be added there unless the E2E layout changes.

## Current Strengths

- Simple Mode already exists as a distinct public-facing workflow rather than a hidden styling variant.
- The public path maps to the real workflow: load data, choose a strategy, optimize, optionally validate, then review artifacts.
- Readiness and action state are modeled in shared frontend utilities instead of scattered copy.
- Recent polish work reduced noisy advanced controls and made the guided flow more practical for new users.

## Improvement Plan

### SIMPLE-01: True Simple Mode Happy-Path E2E

Risk: Low.

Problem:
Simple Mode is the main public workflow, but the test authority should prove a clean user can complete the whole path from data to optimization to optional validation.

Implementation:

- Add a deterministic fixture dataset small enough for fast E2E.
- Prefer adding the scenario to `web/e2e/browser/qa/workflowMatrix.spec.ts`, because that file is already included in the default frontend E2E gate.
- If `web/e2e/browser/qa/simpleModeHappyPath.spec.ts` (new) is created instead, update `web/package.json` and `scripts/run_frontend_e2e_gate.ps1` so the spec is actually run.
- Add an E2E test that starts in Simple Mode and completes:
  - sample or fixture data selection
  - strategy selection
  - optimization start and completion
  - optional validation start and completion
  - summary/recent artifact visibility
- Assert action status text and disabled/enabled transitions, not only component existence.
- Keep the test out of GPU-only paths.

Files:

- `web/e2e/browser/qa/simpleModeHappyPath.spec.ts` (new) or `web/e2e/browser/qa/workflowMatrix.spec.ts`
- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/utils/simpleWorkflowModel.ts`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `scripts/run_frontend_e2e_gate.ps1`

Verification:

- `npm --prefix web run test -- src/utils/simpleWorkflowModel.test.ts`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`

### SIMPLE-02: First-Run Sample Data Contract

Risk: Low/medium.

Problem:
A public user should be able to experience the workflow without already understanding CSV format, Polygon credentials, or cache state.

Implementation:

- Harden the existing bundled sample path rather than adding a duplicate path.
- Keep the sample explicitly labeled as demonstration data.
- Ensure sample data produces a known optimization result range and a known validation result range.
- Store the sample's provenance in saved artifacts.
- Document where the sample lives and how release gates verify it.

Files:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/resources/*`
- `docs/PUBLIC_RELEASE.md`

Verification:

- `npm --prefix web run test -- src/stores/dataStore.actions.test.ts src/components/GuidedWorkspacePanel.test.ts`
- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta`

### SIMPLE-03: Action Coverage And QA IDs

Risk: Low.

Problem:
Some public actions are still easier to verify by text than by stable semantic action IDs. This makes E2E tests fragile.

Implementation:

- Add product-owned `data-qa-action` attributes for Simple Mode primary actions.
- Extend the action coverage inventory so new Simple Mode actions have either coverage or an owned waiver with reason and expiration.
- Do not require zero waivers immediately; require no new unowned/unexpired waivers and prioritize burning down guided-flow waivers.
- Keep labels free to change without breaking E2E tests.

Files:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/scripts/check-action-coverage-manifest.ts`
- `web/scripts/check-qa-selector-inventory.mjs`
- `web/e2e/browser/qa/simpleModeHappyPath.spec.ts` (new)

Verification:

- `npm --prefix web run check:qa-static`
- `npm --prefix web run test:e2e:action-coverage`

### SIMPLE-04: Public Strategy Personas

Risk: Low.

Problem:
Simple Mode should be fully capable but not expose every advanced optimization nuance by default.

Implementation:

- Map strategies into public-friendly descriptions:
  - trend following
  - composite CPU research mode
  - later scope: breakout/momentum and regime-aware trend, if Simple Mode support is intentionally expanded beyond the current `DoubleMa` and `CompositeCpu` support in `simpleWorkflowModel.ts`
  - options research, if enabled in Simple Mode later
- Show only the minimum parameters required for a credible run.
- Keep advanced parameter editing available through Advanced Mode.
- Add a short "what this run will do" summary generated from the request builder, not hard-coded text.

Files:

- `web/src/utils/simpleWorkflowModel.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/services/optimizeRequestBuilder.ts`

Verification:

- `npm --prefix web run test -- src/utils/simpleWorkflowModel.test.ts src/services/optimizeRequestBuilder.test.ts`

### SIMPLE-05: Optional Validation Result Summary

Risk: Low.

Problem:
Simple Mode users do not need full statistical detail, but they do need a clear validation result and whether acceleration was used.

Implementation:

- Add a compact validation outcome block:
  - verdict
  - out-of-sample/holdout result
  - main robustness warning count
  - execution status such as "GPU accelerated", "hybrid", or "CPU fallback"
  - link to Advanced detail
- Reuse existing validation result metadata rather than adding new backend behavior.
- Add the missing `validationAccelerationStatus` or equivalent computed source into `GuidedWorkspacePanel.vue`; do not duplicate route parsing in the component.

Files:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`

Verification:

- `npm --prefix web run test -- src/components/GuidedWorkspacePanel.test.ts src/services/validationService.test.ts`

### SIMPLE-06: Mode Transition Continuity

Risk: Low.

Problem:
Switching between Simple and Advanced should not feel like starting a different application.

Implementation:

- Preserve selected dataset, strategy, latest run, validation summary, and current artifact when toggling modes.
- Add tests for Simple to Advanced and Advanced to Simple transitions.
- Ensure hidden Advanced-only settings do not silently change Simple defaults.

Files:

- `web/src/stores/settingsStore.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/App.vue`
- `web/src/components/workbench/WorkbenchShell.vue`

Verification:

- `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts src/stores/workflowStore.test.ts`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- Add browser-level continuity coverage for Simple to Advanced and Advanced to Simple transitions.

## Acceptance Criteria For This Guide

- A clean browser E2E path can complete Simple Mode data selection, optimization, optional validation, and artifact review.
- Simple Mode remains fully capable for backtest optimization and optional validation, but does not expose live execution controls.
- Public-facing status text is generated from actual service/readiness models, not duplicated copy that can drift.
- Mode switching preserves dataset, strategy, run, and validation context.
- Simple Mode primary actions are represented in `web/e2e/browser/qa/actionCoverageManifest.ts`.

## Deferred Higher-Risk Work

- Making options expression a public Simple Mode path before options data quality and replay semantics are fully productized.
- Changing Simple Mode optimization defaults without benchmark and validation evidence.
- Hiding Advanced Mode entirely for public builds.
