# Frontend Advanced Workbench And Visualizations Implementation Plan

## Surface

Primary files:

- `web/src/components/workbench/*`
- `web/src/components/*WorkspacePanel.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/ActionStatusBar.vue`
- `web/src/visualizations/*`
- `web/src/stores/*`
- `web/src/services/*`
- `web/e2e/*`

## Current Strengths

- Advanced Mode exposes a rich workbench for data, optimization, validation, ML, options, portfolio, execution, and reporting.
- The workbench has explicit shell, rail, inspector, command, and status surfaces.
- Visualization and table surfaces already have some targeted tests and performance-aware structure.
- The application has a broad frontend QA script set.

## Improvement Plan

### ADV-01: Product-Owned Action Registry

Risk: Low.

Problem:
Advanced Mode has many commands. Coverage can drift when actions are added in panels without registering stable IDs or QA coverage.

Implementation:

- Centralize action ID definitions for rail actions, command palette actions, panel toolbar actions, and status bar actions.
- Start from the current action authority files instead of inventing a parallel registry:
  - `web/src/utils/shellCommandRegistry.ts`
  - `web/src/utils/workbenchActionModel.ts`
  - `web/e2e/browser/qa/actionCoverageManifest.ts`
  - `web/scripts/check-action-coverage-manifest.ts`
- Require every user-visible command to declare:
  - stable ID
  - label
  - owning panel
  - availability predicate
  - QA selector
- Add a static test that fails on unregistered button-like actions unless explicitly waived with a reason.

Files:

- `web/src/utils/shellCommandRegistry.ts`
- `web/src/utils/workbenchActionModel.ts`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/workbench/*`
- `web/scripts/check-action-coverage-manifest.ts`

Verification:

- `npm --prefix web run test -- src/utils/shellCommandRegistry.test.ts src/utils/workbenchActionModel.test.ts src/components/CommandPaletteOverlay.test.ts`
- `npm --prefix web run check:qa-static`

### ADV-02: Populated Visualization E2E

Risk: Low.

Problem:
Charts and canvases can render structurally while still being blank, clipped, or disconnected from selections.

Implementation:

- Add populated E2E coverage for the most important visualizations:
  - equity curve
  - validation distribution
  - optimization heatmap or candidate table selection
  - options payoff surface if enabled
- Use deterministic fixture data.
- Include nonblank canvas or SVG pixel checks where the visualization library permits it.
- Assert selection and hover details when practical.

Files:

- `web/e2e/browser/qa/advancedVisualizations.spec.ts` (new, requires gate/package-script wiring) or an existing included browser QA spec
- `web/src/visualizations/*`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`

Verification:

- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `npm --prefix web run test:e2e:visual`

Gate note:

`test:e2e:visual` currently targets visual baselines. A new populated visualization spec must either be added to an existing included spec, run under `test:e2e:workflow:full`, or wired into `web/package.json` and `scripts/run_frontend_e2e_gate.ps1`.

### ADV-03: Command Palette Drift Guard

Risk: Low.

Problem:
The command palette should stay aligned with workbench navigation and panel action state.

Implementation:

- Build command palette entries from the same registry used by rail/panel actions.
- Add tests for hidden/disabled/runnable states.
- Include search alias coverage for the most common user terms:
  - data
  - optimize
  - validate
  - reports
  - polygon
  - options
  - ml
  - execution

Files:

- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/utils/shellCommandRegistry.ts`
- `web/src/utils/workbenchActionModel.ts`
- `web/src/components/workbench/*`

Verification:

- `npm --prefix web run test -- src/components/CommandPaletteOverlay.test.ts src/utils/shellCommandRegistry.test.ts src/utils/workbenchActionModel.test.ts`

### ADV-04: Dense Table And Large Result Performance Budget

Risk: Low/medium.

Problem:
Advanced users can generate large candidate sets, validation matrices, and artifact histories. The UI should stay responsive.

Implementation:

- Add fixture-driven tests for large candidate tables and artifact history views.
- Record expected row counts, render time budget, and interaction budget.
- Keep virtualization dimensions stable so rows do not reflow during hover/selection.
- Add a local profiling note for developers.

Files:

- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/composables/useVirtualTableRows.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedTableDensity.ts`
- `web/src/services/artifactHistoryPerf.test.ts`
- `web/src/services/validationReportPerf.test.ts`
- `web/src/services/*Perf.test.ts` or a new focused perf fixture if the surface has no existing perf test

Verification:

- `npm --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts src/components/HistoryWorkspacePanel.test.ts src/services/artifactHistoryPerf.test.ts src/services/validationReportPerf.test.ts`
- `npm --prefix web run perf:ui:budget`

### ADV-05: Saved Workflow Replay In The UI

Risk: Medium.

Problem:
Advanced Mode should make it clear when a saved artifact can be replayed exactly, approximately, or only inspected.

Implementation:

- Show replay class and missing-context reasons in the artifact/history detail view.
- Add a guarded "rebuild request from artifact" command only when the artifact is exact or compatible.
- Display schema migration warnings without blocking inspection.

Files:

- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `src-tauri/src/run_artifacts.rs`

Verification:

- `npm --prefix web run test -- src/services/historyService.test.ts src/components/HistoryWorkspacePanel.test.ts`
- `cargo test -p ta_desktop_demo_app --lib run_artifacts`

### ADV-06: Advanced Accessibility And Keyboard Matrix

Risk: Low.

Problem:
Advanced Mode has many dense controls. Keyboard and screen-reader behavior should be intentionally checked, not incidental.

Implementation:

- Add an accessibility matrix for:
  - app shell navigation
  - command palette
  - tabbed workbench panels
  - modal/dialog flows
  - large tables
  - chart summary alternatives
- Add E2E checks for keyboard-only navigation through the main panels.
- Add `aria-label` and summary text where charts are not inherently accessible.

Files:

- `web/src/components/workbench/*`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/visualizations/*`
- `web/e2e/browser/qa/accessibilityKeyboard.spec.ts` (new, requires gate/package-script wiring)

Verification:

- `npm --prefix web run a11y:smoke`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`

Gate note:

`a11y:smoke` is axe smoke coverage. Keyboard traversal coverage must live in an E2E lane that is explicitly run.

## Acceptance Criteria For This Guide

- New command/action work uses existing command and action authorities before adding new registries.
- E2E specs live under the current `web/e2e/browser/qa` layout unless the whole QA structure is intentionally changed.
- Visual tests assert populated, nonblank chart/canvas states rather than only mounted components.
- Large-table changes preserve stable row dimensions and pass the UI performance budget.
- Accessibility checks use the existing `a11y:smoke` and browser QA lanes before new scripts are introduced.

## Deferred Higher-Risk Work

- Major navigation model changes.
- Replacing chart libraries or table virtualization primitives.
- Making Advanced Mode use a different backend request model than Simple Mode.
