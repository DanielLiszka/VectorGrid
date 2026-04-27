# Frontend End-to-End QA Automation Implementation Plan

## Additive QA Hardening Implemented

The current implementation keeps CI/CD out of scope and adds local, opt-in verification surfaces:

- Static QA drift checks: `npm --prefix web run check:qa-static`.
- Selector inventory and local summary report: `npm --prefix web run check:qa-report`.
- Visual baseline snapshots: `npm --prefix web run test:e2e:visual` and `npm --prefix web run test:e2e:visual:update` for intentional UI changes.
- WCAG-focused smoke: `npm --prefix web run a11y:smoke:wcag`.
- UI perf budget: `npm --prefix web run perf:ui:budget:capture`.
- Optional local gate flags: `-IncludeQaStatic`, `-IncludeQaReport`, `-IncludeVisualBaselines`, `-IncludeWcag22Smoke`, `-IncludeBaselineUi`, and `-IncludeUiPerfBudget` on `scripts/run_frontend_e2e_gate.ps1`.

These checks are additive: they do not change runtime application behavior, backend contracts, or default build output. When the UI changes intentionally, update `web/e2e/browser/qa/actionCoverageManifest.ts`, affected workflow specs, and visual snapshots in the same change.

## Intent

Build a comprehensive click-and-use QA automation system for the VectorGrid frontend so foreseeable user-facing workflow bugs are caught before release.

This plan is intentionally broader than unit, component, or service tests. It covers real browser interaction, deterministic mocked Tauri workflows, native desktop smoke, accessibility checks, visual/layout checks, action coverage accounting, and a repeatable bug-fix loop.

The target is not a mathematical guarantee that no user can ever find a bug. The target is a practical release gate where every product-owned workflow and every non-waived visible action is exercised by automation in at least one realistic user journey, with error states and recovery states covered where they matter.

## Definition Of Full Coverage

For this repo, "full comprehensive E2E QA coverage" means:

- Every primary workflow has automated happy-path, blocked-path, failure-path, and recovery-path coverage where those states exist.
- Every major workspace and workflow stage is opened by automation in desktop and compact browser viewports.
- Every visible action that is product-owned has a stable action id and is either exercised by at least one E2E workflow or explicitly waived with a reason.
- Every mocked Tauri command used by a covered workflow is strict by default, fixture-backed, and fails the test if the frontend calls an unexpected command.
- Browser tests fail on uncaught `pageerror`, relevant console errors, request failures, stuck busy states, broken focus traps, missing expected UI state, and critical layout overflow.
- Desktop smoke verifies the real Tauri shell and native integration without becoming the main dense workflow test layer.
- A bug discovered manually gets a reproduction workflow before or alongside the fix.

This definition makes coverage measurable. It does not depend on generic "click everything" crawlers as the release gate, because blind crawlers create false positives and miss domain expectations. Exploratory crawlers remain useful as a supplemental discovery tool.

## Current State

Existing local frontend E2E assets:

- `web/e2e/browser/`
  - Playwright specs for mocked-Tauri browser UX smoke.
  - Current examples cover shell navigation, Guided Mode, command palette, overlays, help tooltips, live review surfaces, and dense library behavior.
- `web/e2e/browser/support/mockTauri.ts`
  - Mocked Tauri harness with strict command response support.
- `web/e2e/browser/support/appHarness.ts`
  - Shared shell launch helpers.
- `web/e2e/desktop/`
  - WDIO/Tauri smoke harness for real desktop shell launch, titlebar, workspace navigation, command palette, task center, settings, geometry, and close behavior.
- `web/scripts/check-a11y-smoke.mjs`
  - Playwright + axe smoke scan.
- `web/scripts/capture-baseline-ui.mjs`
  - Broad screenshot baseline capture.
- `web/package.json`
  - Existing scripts: `test:browser:ux`, `test:desktop:smoke`, `test:desktop:geometry`, `a11y:smoke`, `baseline:ui`.

Important current repository issue:

- Root `.gitignore` currently ignores `web/e2e/`, `web/playwright.config.ts`, `web/wdio.desktop.conf.ts`, `web/tsconfig.wdio.json`, and some supporting scripts.
- The harness exists locally, but it is not durable repository coverage until these files are intentionally moved to tracked paths or explicitly unignored.

## Test Layer Strategy

### Layer 1: Browser Workflow QA

Primary lane for comprehensive click-and-use coverage.

Use Playwright against the built frontend with mocked Tauri enabled. This layer should own:

- Full product workflow matrix.
- Dense form and table interactions.
- Empty, blocked, success, and failure states.
- Command palette and keyboard routes.
- Responsive viewport checks.
- Strict mocked command contracts.
- Action coverage accounting.
- Regressions for user-discovered bugs.

Recommended command names:

```powershell
npm --prefix web run test:e2e:workflow
npm --prefix web run test:e2e:workflow:headed
npm --prefix web run test:e2e:workflow:debug
```

### Layer 2: Desktop Smoke

Native shell proof, not exhaustive workflow proof.

Use the existing WDIO/Tauri lane to verify:

- The desktop app launches into the real shell.
- The WebView loads the app.
- Native titlebar controls are present and usable.
- Desktop workspaces are reachable.
- Native shell overlays open and close.
- Settings and diagnostics expose desktop runtime facts.
- Packaged release smoke can attach to the shipped executable.

Keep this layer intentionally smaller than browser workflow QA. It is slower, more environment-dependent, and less deterministic.

Recommended command names:

```powershell
npm --prefix web run test:desktop:smoke
npm --prefix web run test:desktop:geometry
```

### Layer 3: Accessibility And Visual Evidence

Use existing a11y and screenshot lanes as supporting proof:

```powershell
npm --prefix web run a11y:smoke
npm --prefix web run baseline:ui
```

Add targeted accessibility scans inside critical workflow specs where interaction state matters, especially dialogs, command palette, guided mode, validation results, live safety confirmation, and mobile overlays.

### Layer 4: Exploratory Workflow Sweep

Supplemental discovery lane for broad "try visible actions" coverage.

This should not be a required release gate until it is domain-aware enough to avoid noisy false positives. Use it to identify missing workflow cases and unexpected runtime errors.

Recommended command name:

```powershell
npm --prefix web run test:e2e:explore
```

Artifacts should live under `output/playwright/`.

## Required Repository Changes

### 1. Make E2E Harness Files Trackable

Choose one approach:

- Preferred: unignore the existing E2E harness files in root `.gitignore`.
- Alternative: move durable E2E files to a tracked path and keep only generated artifacts ignored.

Recommended `.gitignore` adjustment:

```gitignore
!/web/playwright.config.ts
!/web/wdio.desktop.conf.ts
!/web/tsconfig.wdio.json
!/web/e2e/
!/web/e2e/**
!/web/scripts/desktop/
!/web/scripts/desktop/**
!/web/scripts/capture-baseline-ui.mjs
!/web/scripts/check-a11y-smoke.mjs
```

Keep generated outputs ignored:

```gitignore
/output/
/web/dist/
/web/node_modules/
/web/test-results/
/web/playwright-report/
```

### 2. Add Stable QA Selectors

Add stable attributes to product-owned interactive elements and high-signal state nodes.

Conventions:

- `data-qa-action`
  - Unique id for a user action.
  - Example: `workflow.optimize.run`, `validation.runSelected`, `commandPalette.open`.
- `data-qa-panel`
  - Stable id for major panels.
  - Example: `workflow.optimize`, `results.validation`, `workspace.live`.
- `data-qa-state`
  - Stable id for stateful output.
  - Example: `run.status`, `validation.verdict`, `dataset.summary`.
- `data-qa-workflow-step`
  - Stable id for workflow step controls if existing ids are insufficient.

Do not use QA selectors for styling. Do not encode display text in selector ids. Keep action ids stable across label changes.

### 3. Add Action Coverage Manifest

Create a manifest that maps product-owned visible actions to workflow coverage.

Suggested path:

```text
web/e2e/browser/qa/actionCoverageManifest.ts
```

Suggested shape:

```ts
export interface QaActionCoverageEntry {
  actionId: string
  surface: string
  required: boolean
  coveredBy: string[]
  waiver?: {
    reason: string
    owner: string
    expires?: string
  }
}
```

Add a Playwright or Node check that:

- Opens each canonical workflow state.
- Inventories visible `[data-qa-action]` elements.
- Fails if a required action is missing from the manifest.
- Fails if a required action has no `coveredBy` workflow.
- Fails if a waiver is expired.
- Emits a coverage report to `output/playwright/action-coverage.json`.

Recommended command:

```powershell
npm --prefix web run test:e2e:action-coverage
```

### 4. Expand Mocked Tauri Scenario Fixtures

Move command responses out of individual specs into named scenario fixtures.

Suggested path:

```text
web/e2e/browser/fixtures/
```

Suggested fixture families:

- `emptyApp.ts`
- `datasetLoaded.ts`
- `optimizationSuccess.ts`
- `optimizationFailure.ts`
- `validationSuccess.ts`
- `validationDatasetMismatch.ts`
- `artifactHydrated.ts`
- `libraryWithManyRuns.ts`
- `aggregateMetaReady.ts`
- `mlSignalReady.ts`
- `optionsExpressionReady.ts`
- `liveExecutionReady.ts`
- `liveExecutionBrokerFailure.ts`
- `settingsDiagnostics.ts`

Fixtures should include:

- Tauri command result map.
- Expected command calls.
- Optional emitted progress events.
- Optional starting localStorage/sessionStorage.
- Expected visible state after hydration.

The mocked Tauri harness should stay strict by default. Unexpected invokes are workflow bugs unless the scenario explicitly opts out.

## Proposed File Layout

```text
web/e2e/browser/
  qa/
    workflowMatrix.spec.ts
    actionCoverage.spec.ts
    responsiveWorkflow.spec.ts
    regressionRepros.spec.ts
    exploratorySweep.spec.ts
    actionCoverageManifest.ts
  fixtures/
    emptyApp.ts
    datasetLoaded.ts
    optimizationSuccess.ts
    optimizationFailure.ts
    validationSuccess.ts
    validationDatasetMismatch.ts
    artifactHydrated.ts
    libraryWithManyRuns.ts
    aggregateMetaReady.ts
    mlSignalReady.ts
    optionsExpressionReady.ts
    liveExecutionReady.ts
    liveExecutionBrokerFailure.ts
  support/
    appHarness.ts
    mockTauri.ts
    workflowRunner.ts
    qaAssertions.ts
    actionInventory.ts
    viewportMatrix.ts
web/e2e/desktop/
  specs/
    desktop-shell-readiness.spec.ts
    desktop-workspace-navigation.spec.ts
    desktop-command-palette.spec.ts
    desktop-task-center.spec.ts
    desktop-settings-diagnostics.spec.ts
    desktop-titlebar-native-controls.spec.ts
    desktop-window-geometry.spec.ts
```

## Global Failure Rules

Every browser workflow spec should install the same failure guards:

- Fail on uncaught `pageerror`.
- Fail on unexpected console `error`.
- Fail on unexpected Tauri command invoke.
- Fail on failed network request for local app assets.
- Fail when app readiness never resolves.
- Fail when a loading/busy control remains busy after its expected completion window.
- Fail when an expected dialog does not close.
- Fail when focus is lost after closing modal overlays.
- Fail when primary content has unexpected horizontal scroll at target viewports.
- Fail when required status or error text is missing after an action.

Console filtering must be explicit. A known benign console message should be allowlisted with a comment and a removal condition.

## Workflow Matrix

### Shell And Global Navigation

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `shell.launch.advanced` | First advanced shell load | Titlebar, workspace bar, workflow step bar, footer, active workflow state |
| `shell.launch.guided` | First-run Guided Mode | Guided grid visible, advanced-only chrome hidden, primary next action shown |
| `shell.workspace.tabs` | Navigate all workspaces | Workflow, Library, Aggregate, Pro, Live, Settings panels visible |
| `shell.workflow.steps` | Navigate all workflow steps | Data, Optimize, Validate, Analyze, Finalize state transitions |
| `shell.commandPalette.navigation` | Open palette and navigate to surfaces | Search, keyboard movement, execute, focus return |
| `shell.taskCenter` | Open and close task center | Sections visible, Escape closes, focus returns |
| `shell.settings.modeSwitch` | Switch Guided/Advanced modes | Correct chrome and persistence |
| `shell.keyboard` | Keyboard-only workspace and workflow navigation | Arrow/Home/End behavior and selected state |

### Data Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `data.empty` | No dataset loaded | Blocked actions explain missing data |
| `data.sample.load` | Load sample data through mocked command | Dataset summary, context bar, next action enabled |
| `data.csv.import.success` | Import CSV success fixture | Status, row count, timeframe, downstream readiness |
| `data.csv.import.failure` | Import CSV parse failure | Error visible, app recovers, no unhandled error |
| `data.provider.settings` | Provider key/status UI | Key state and cache controls render safely |
| `data.repair.gaps` | Dataset validation/repair surface | Gap status visible, repair action result surfaced |

### Strategy And Configuration Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `strategy.doubleMa.basic` | Configure DoubleMa defaults | Fields editable, summary updates, run enabled with data |
| `strategy.compositeTrend` | Configure CompositeTrend route policy | ADX toggle route copy, backend target shown |
| `strategy.breakoutMomentum` | Configure BreakoutMomentum route policy | Route and capability summary consistent |
| `strategy.regimeHybrid` | Configure RegimeHybrid | Hybrid capability summary visible |
| `strategy.compositeMo` | Multi-objective config | Objective controls, Pareto readiness |
| `strategy.dynamicBuilder` | Dynamic strategy builder surface | Builder inputs update preview and blocked states |
| `strategy.invalidInputs` | Invalid ranges/params | Validation messages, run disabled, recovery after correction |

### Optimization Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `optimize.blocked.noData` | Run optimization without data | Disabled or blocked reason visible |
| `optimize.success.doubleMa` | Successful DoubleMa optimization | Progress, route, top candidate, overview, charts/tables |
| `optimize.success.compositeTrend` | Successful CompositeTrend optimization | Route metadata and result table visible |
| `optimize.failure.backend` | Backend returns optimize failure | Error visible, no stuck spinner, retry possible |
| `optimize.cancel` | Cancel in-flight optimization | Cancel state visible, controls recover |
| `optimize.artifact.saved` | Save latest run artifact | Saved status and library visibility |
| `optimize.titlebarPrimary` | Stage-aware titlebar primary action | Correct action fires for optimize stage |

### Validation Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `validation.blocked.noCandidate` | No selected candidate | Blocked reason visible |
| `validation.blocked.datasetMismatch` | Saved artifact but matching dataset missing | Error is shown in UI, no `pageerror` |
| `validation.success.selected` | Validate selected candidate | Verdict, metrics, table, report action |
| `validation.failure.backend` | Backend validation failure | Failure state shown, controls recover |
| `validation.compare` | Compare validation results | Compare summary and selected baseline provenance |
| `validation.reportExport` | Export/open report actions | Mocked commands called and status visible |
| `validation.titlebarPrimary` | Stage-aware validate action | Conventional validate path fires from titlebar |

### Analysis And Result Review Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `results.overview` | Open overview after optimization | Summary metrics and selected candidate visible |
| `results.topTable` | Top results table interactions | Sort/filter/select row, virtual scroll stable |
| `results.heatmap` | Heatmap render and keyboard candidate detail | Canvas non-empty, keyboard selection, Enter candidate detail |
| `results.pareto` | Pareto render and selection | Canvas non-empty, point list, selected summary |
| `results.candidateDetail` | Candidate Detail | Chart host, trade table, marker selection summary |
| `results.export` | Export/copy actions | Status or mocked command confirmation |
| `results.emptyStates` | Analysis panels before data/results | Action-oriented empty states |

### Library And Artifact Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `library.empty` | No saved runs | Empty state and next actions |
| `library.manyRuns` | Many saved runs | Virtualized scroll host, row selection, no page scroll hijack |
| `library.openRun` | Load saved run artifact | Hydration state, context bar, result panels |
| `library.refresh` | Refresh history | Strict command call and status |
| `library.deleteOrReveal` | Delete/reveal/open actions where available | Confirmation/status and recovery |

### Aggregate, Meta, And Campaign Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `aggregate.empty` | Aggregate workspace without artifacts | Blocked reason and next actions |
| `aggregate.ready` | Aggregate artifacts available | Meta table and summary visible |
| `aggregate.compare` | Compare current and saved meta artifact | Provenance and compare summary |
| `meta.stage5.ready` | Stage 5 ML confidence surface | Key controls, blocked/success states |
| `campaigns.basic` | Campaign orchestration surface | Setup controls and status rendering |
| `campaigns.failure` | Campaign backend failure | Error visible, no stuck running state |

### ML Signal Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `mlSignal.empty` | Missing bundle/data | Blocked reason visible |
| `mlSignal.optimize.success` | ML Signal optimization success | Bundle status, results, validation path |
| `mlSignal.validation.failure` | ML validation failure | Error visible, store state updates, no unhandled rejection |
| `mlSignal.cache` | Score cache generation/status | Command result and status text |
| `mlSignal.policySweep` | Policy sweep surface | Controls, results, empty/failure states |

### Options Expression Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `options.empty` | Missing parent/options data | Blocked reason visible |
| `options.parentArtifact` | Parent artifact selected | Provenance summary and next action |
| `options.optimize.success` | Options expression optimize success | Tuple summary, feasibility, result table |
| `options.validation.success` | Embedded validation success | Validation verdict and provenance guard |
| `options.provenanceMismatch` | Parent/data/manifest mismatch | Guard message visible, no unhandled error |
| `options.reopenCompare` | Reopen saved options artifact | Artifact-scoped compare behavior |

### Live Execution Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `live.empty` | No credentials/data/stream | Readiness matrix and blocked reasons |
| `live.paper.ready` | Paper mode credentials fixture | Account status, supported strategy, dry-run readiness |
| `live.refreshOrders` | Refresh broker snapshot | Orders table, flat/position summary |
| `live.savedSessions` | Refresh saved live sessions | Session list, events, broker snapshots |
| `live.cancelOrders.confirm` | Cancel open orders confirmation | Broker-facing copy and non-flatten warning |
| `live.flatten.confirm` | Flatten confirmation | Risk copy and confirmation behavior |
| `live.emergencyHalt` | Emergency halt | High-risk confirmation and status |
| `live.runner.streamMismatch` | Stream symbol mismatch | Runner start blocked and explained |
| `live.brokerFailure` | Broker command failure | Error visible, no hidden armed state |

### Settings And Diagnostics Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `settings.general` | Settings workspace opens | Mode, density, theme, persistence controls |
| `settings.diagnostics` | Diagnostics panel | Runtime, GPU/build, command registry status |
| `settings.window` | Window/native settings | Desktop-specific controls visible in desktop smoke |
| `settings.reset` | Reset/clear state action | Confirmation and recovery |

### Responsive And Mobile Workflows

| Workflow id | Scenario | Required checks |
| --- | --- | --- |
| `responsive.mobile.guided` | Guided mode at compact width | No clipped primary actions |
| `responsive.mobile.commandPalette` | Command palette at compact width | Search usable, key chips wrap, results visible |
| `responsive.mobile.workflowNav` | Workflow navigation at compact width | No horizontal content overflow |
| `responsive.table.dense` | Dense tables at narrower viewport | Scroll is contained in table host |
| `responsive.dialogs` | Task center and confirmations at compact width | Dialogs fit and focus trap works |

## Viewport Matrix

Run the full workflow matrix at desktop size by default:

- `1600x1000`

Run a critical subset at:

- `1366x768`
- `1024x768`
- `390x844`

The compact subset should include Guided Mode, command palette, workflow navigation, validation failure state, task center, live confirmation dialogs, and dense table/heatmap panels.

## Assertion Standards

Each workflow case should include:

- Starting scenario name.
- Exact user action sequence.
- Expected visible state after each major action.
- Expected status/error text for failure states.
- Expected command invocations where relevant.
- Expected focus behavior for dialogs/overlays.
- Busy-state recovery assertion.
- Console/page error assertion.
- Optional screenshot on failure through Playwright config.

Prefer visible user-facing assertions over direct Pinia state inspection. Use app-internal state only for test setup or diagnostics.

## Bug-Fix Workflow

When QA automation finds or reproduces a bug:

1. Add or update the smallest E2E workflow that reproduces the issue.
2. Confirm the workflow fails for the right reason.
3. Fix the product code.
4. Add lower-level unit/component coverage only if the bug is better pinned there too.
5. Re-run the targeted E2E workflow.
6. Re-run the relevant suite lane.
7. Keep the repro workflow permanently unless it is superseded by broader coverage.

This keeps "a button does not work" bugs from returning.

## CI And Local Gate Policy

Recommended local gates:

```powershell
npm --prefix web run typecheck
npm --prefix web run lint
npm --prefix web run test
npm --prefix web run test:e2e:workflow
npm --prefix web run a11y:smoke
```

Recommended release candidate gates:

```powershell
npm --prefix web run typecheck
npm --prefix web run lint
npm --prefix web run parity
npm --prefix web run test
npm --prefix web run test:e2e:workflow
npm --prefix web run test:e2e:action-coverage
npm --prefix web run a11y:smoke
npm --prefix web run baseline:ui
npm --prefix web run test:desktop:smoke
```

Build-once browser QA gate for local handoff and release rehearsals:

```powershell
powershell -File scripts/run_frontend_e2e_gate.ps1
powershell -File scripts/run_frontend_e2e_gate.ps1 -IncludeResponsive -IncludeRegressions -IncludeA11ySmoke
powershell -File scripts/run_repo_quality_gate.ps1 -IncludeBrowserWorkflowQa -IncludeBrowserResponsiveE2E -IncludeBrowserRegressionE2E
```

The default frontend E2E gate runs the sliced QA lane: workbench smoke, action coverage, and core workflow coverage. `-IncludeResponsive`, `-IncludeRegressions`, and `-IncludeExploratory` add the focused optional slices without duplicating the core lane; `-IncludeFullWorkflow` runs the full browser spec set instead.

Recommended nightly or pre-release expanded gates:

```powershell
npm --prefix web run test:e2e:workflow:full
npm --prefix web run test:e2e:explore
npm --prefix web run test:desktop:geometry
powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeBrowserWorkflowQa
```

Desktop smoke should be required for release candidates, but not necessarily for every small frontend-only PR on machines without the correct EdgeDriver/Tauri driver setup.

## Flake Policy

Flaky tests are product risk. Treat them as failed engineering work, not as normal noise.

Rules:

- No arbitrary long sleeps when a deterministic state assertion is possible.
- Every async workflow waits on visible state, event emission, command completion, or specific busy-state transition.
- A flaky test must be fixed, quarantined with owner/date/reason, or removed if it tests no product value.
- Quarantined workflows still count as uncovered in the action coverage report unless there is another stable workflow covering the same action.
- Test retries in CI are diagnostic only. A test that passes only on retry should still be investigated.

## Implementation Phases

### Phase 0: Promote Existing Harness To Durable Coverage

- [x] Unignore or move `web/e2e/`, Playwright config, WDIO config, and E2E support scripts.
- [x] Confirm `npm --prefix web run test:browser:ux` runs from a clean checkout.
- [x] Rename or alias `test:browser:ux` to the new workflow QA command without breaking the existing command.
- [x] Ensure browser and desktop artifacts write only under `output/`.

Exit criteria:

- Existing E2E harness files are tracked.
- Existing browser UX and desktop smoke commands still pass.

### Phase 1: Shared QA Infrastructure

- [x] Add `qaAssertions.ts` with global console/page/request/stuck-busy guards.
- [x] Add `workflowRunner.ts` for scenario setup and common action execution.
- [ ] Add fixture modules for every fully populated dataset, optimization, validation, library, aggregate, ML, options, live, and settings state. Current additive slice includes validation success/failure, saved options artifacts, and live safety readiness.
- [x] Keep mocked Tauri strict by default.
- [x] Add action inventory helper for `[data-qa-action]`.

Exit criteria:

- New workflows can be added with minimal duplicated setup.
- Unknown mocked Tauri invokes fail the relevant test.

### Phase 2: Stable Action Selectors And Coverage Manifest

- [ ] Add `data-qa-action` to all primary shell, workflow, result, live, and settings controls.
- [ ] Add `data-qa-panel` and `data-qa-state` to high-signal assertions.
- [x] Add `actionCoverageManifest.ts`.
- [x] Add action coverage test/report.
- [x] Require every required action to be covered or waived.

Exit criteria:

- Action coverage report can identify orphaned buttons and missing workflow coverage.
- Adding a new product-owned action without a manifest entry fails the coverage check.

### Phase 3: Core Workflow Matrix

- [x] Implement shell/global navigation workflows.
- [x] Implement baseline data workflows.
- [x] Implement baseline strategy configuration navigation workflows.
- [x] Implement baseline optimization blocked/failure workflows.
- [x] Implement baseline validation workflows for both dataset-mismatch failure and conventional validation success.
- [ ] Implement analysis/result review workflows.
- [x] Implement baseline library/artifact workflows.
- [x] Add compact viewport coverage for critical overlays and workflow navigation.

Exit criteria:

- A conventional user can be simulated from launch through data, strategy, optimize, validate, analyze, and reopen artifact.
- Known validation mismatch and service failure paths are covered.

### Phase 4: Advanced Product Surface Matrix

- [ ] Add aggregate/meta workflows.
- [ ] Add ML Signal workflows.
- [x] Add baseline options-expression validation workflow for saved artifact preview/run/reveal.
- [x] Add baseline live execution safety workflows.
- [ ] Add settings/diagnostics workflows.
- [ ] Add representative failure/recovery cases for each advanced surface.

Exit criteria:

- Every major workspace and advanced product family has at least one happy-path and one blocked/failure-path workflow.

### Phase 5: Desktop Shell Reinforcement

- [x] Keep desktop smoke focused on native shell proof.
- [x] Add one desktop spec that confirms the browser workflow-critical workspaces are reachable in the real shell.
- [ ] Add packaged smoke documentation for release candidate use.
- [ ] Keep geometry opt-in until stable enough for default release gates.

Exit criteria:

- Browser workflow QA proves dense behavior.
- Desktop smoke proves the same app shell is reachable in the native runtime.

### Phase 6: Local Reporting

- [x] Add package scripts for workflow, action coverage, full workflow, exploratory, and headed/debug lanes.
- [x] Add documented local gates for PR-style and release-candidate-style coverage. CI/CD remains explicitly out of scope.
- [x] Emit JSON coverage/static reports and retain Playwright traces/screenshots on failure.
- [x] Add summary output with workflow count, action coverage count, waived actions, failed actions, and artifact paths.

Exit criteria:

- A maintainer can see which workflows and actions are covered from a single report.
- Failed runs include enough artifact detail to reproduce the issue.

### Phase 7: Maintenance And Governance

- [x] Add a short E2E contribution guide under `web/e2e/browser/README.md`.
- [x] Require new UI actions to include a `data-qa-action` id and manifest entry through documented local checks.
- [x] Require user-discovered bugs to add an E2E repro in the contribution guide.
- [ ] Review waivers monthly or before release.
- [ ] Keep exploratory sweep failures triaged into either product bugs, harness bugs, or new manifest waivers.

Exit criteria:

- QA coverage grows with the product instead of becoming stale.

## Implementation Appendix A: Package Scripts

Add these scripts to `web/package.json`. Keep the existing `test:browser:ux` name as a compatibility alias until contributors migrate.

```json
{
  "scripts": {
    "test:e2e:workflow": "npm run build && playwright test --project=chromium --grep @workflow",
    "test:e2e:workflow:full": "npm run build && playwright test --project=chromium",
    "test:e2e:workflow:headed": "npm run build && playwright test --project=chromium --headed --grep @workflow",
    "test:e2e:workflow:debug": "npm run build && playwright test --project=chromium --debug --grep @workflow",
    "test:e2e:action-coverage": "npm run build && playwright test e2e/browser/qa/actionCoverage.spec.ts --project=chromium",
    "test:e2e:responsive": "npm run build && playwright test e2e/browser/qa/responsiveWorkflow.spec.ts --project=chromium",
    "test:e2e:regressions": "npm run build && playwright test e2e/browser/qa/regressionRepros.spec.ts --project=chromium",
    "test:e2e:explore": "npm run build && playwright test e2e/browser/qa/exploratorySweep.spec.ts --project=chromium",
    "test:browser:ux": "npm run test:e2e:workflow"
  }
}
```

If the repo keeps the current lighter browser UX command, use this instead:

```json
{
  "scripts": {
    "test:browser:ux": "npm run build && playwright test --grep @smoke",
    "test:e2e:workflow": "npm run build && playwright test --grep @workflow"
  }
}
```

Recommended Playwright tags:

| Tag | Meaning | Gate |
| --- | --- | --- |
| `@smoke` | Fast shell and one or two critical workflows | default frontend confidence |
| `@workflow` | Main deterministic click-and-use workflows | PR and release candidate |
| `@responsive` | Compact and alternate viewport proof | release candidate |
| `@a11y` | Interaction-state accessibility checks | release candidate |
| `@regression` | Permanent user-discovered bug repros | PR and release candidate |
| `@explore` | Domain-aware crawler/discovery checks | nightly or manual |
| `@desktop` | WDIO/Tauri native shell coverage | release candidate on provisioned Windows |

## Implementation Appendix B: Required `.gitignore` Change

The current root `.gitignore` ignores `/docs/` and `/web/e2e/`. That is acceptable for local research notes, but not for a durable QA suite or this plan if the team wants it versioned.

Minimum unignore block:

```gitignore
# Durable QA and current implementation docs
!/docs/REPO_MAP.md
!/docs/frontend_end_to_end_qa_automation_plan.md
!/web/playwright.config.ts
!/web/wdio.desktop.conf.ts
!/web/tsconfig.wdio.json
!/web/e2e/
!/web/e2e/**
!/web/scripts/desktop/
!/web/scripts/desktop/**
!/web/scripts/capture-baseline-ui.mjs
!/web/scripts/check-a11y-smoke.mjs
```

Keep these generated paths ignored:

```gitignore
/output/
/web/dist/
/web/node_modules/
/web/test-results/
/web/playwright-report/
/web/.playwright/
```

Do not unignore broad research/archive folders as part of this QA work. Keep the change tightly scoped to durable QA harness files and the implementation plan.

## Implementation Appendix C: Initial QA Action ID Inventory

The app already exposes stable navigation selectors such as `data-workspace`, `data-workflow-step`, `data-ui-mode`, `data-mode`, and `data-tab`. These should remain supported. Add `data-qa-action` to product-owned actions so coverage can be measured independently of labels and CSS.

### Shell And Global Actions

| Current surface | Current selector or label | Required `data-qa-action` |
| --- | --- | --- |
| Desktop titlebar command palette | `aria-label="Open command palette"` | `shell.commandPalette.open` |
| Desktop titlebar settings | `aria-label="Open settings workspace"` | `shell.settings.open` |
| Desktop titlebar primary action | `.titlebarPrimary` | `shell.primaryAction.run` |
| Minimize window | `aria-label="Minimize window"` | `shell.window.minimize` |
| Maximize/restore window | `aria-label="Maximize window"` / `Restore window` | `shell.window.toggleMaximize` |
| Close window | `aria-label="Close window"` | `shell.window.close` |
| UI mode Guided | `#uiModeBar [data-ui-mode="guided"]` | `shell.uiMode.guided` |
| UI mode Advanced | `#uiModeBar [data-ui-mode="advanced"]` | `shell.uiMode.advanced` |
| Workspace Research | `#workspaceBar [data-workspace="workflow"]` | `shell.workspace.workflow` |
| Workspace Library | `#workspaceBar [data-workspace="library"]` | `shell.workspace.library` |
| Workspace Aggregate | `#workspaceBar [data-workspace="aggregate"]` | `shell.workspace.aggregate` |
| Workspace Diagnostics | `#workspaceBar [data-workspace="pro"]` | `shell.workspace.diagnostics` |
| Workspace Live | `#workspaceBar [data-workspace="live"]` | `shell.workspace.live` |
| Workspace Settings | `#workspaceBar [data-workspace="settings"]` | `shell.workspace.settings` |
| Mode Conventional | `#modeBar [data-mode="conventional"]` | `shell.mode.conventional` |
| Mode ML Signal | `#modeBar [data-mode="ml-signal"]` | `shell.mode.mlSignal` |
| Workflow Data | `#workflowStepBar [data-workflow-step="data"]` | `shell.workflowStep.data` |
| Workflow Optimize | `#workflowStepBar [data-workflow-step="optimize"]` | `shell.workflowStep.optimize` |
| Workflow Validate | `#workflowStepBar [data-workflow-step="validate"]` | `shell.workflowStep.validate` |
| Workflow Analyze | `#workflowStepBar [data-workflow-step="analyze"]` | `shell.workflowStep.analyze` |
| Workflow Finalize | `#workflowStepBar [data-workflow-step="finalize"]` | `shell.workflowStep.finalize` |
| Task center launcher | `.taskCenterLauncher` | `shell.taskCenter.open` |
| Setup/results divider | `.workspaceDivider` | `shell.layout.resizeSplit` |
| Setup Data tab | `#controlsTabBar [data-tab="data"]` | `shell.controlsTab.data` |
| Setup Optimize tab | `#controlsTabBar [data-tab="opt"]` | `shell.controlsTab.optimize` |
| Setup Strategy tab | `#controlsTabBar [data-tab="strategy"]` | `shell.controlsTab.strategy` |
| Setup Advanced tab | `#controlsTabBar [data-tab="advanced"]` | `shell.controlsTab.advanced` |
| Results Overview tab | `#resultsTabBar [data-tab="overview"]` | `shell.resultsTab.overview` |
| Results Top tab | `#resultsTabBar [data-tab="top"]` | `shell.resultsTab.top` |
| Results Pareto tab | `#resultsTabBar [data-tab="pareto"]` | `shell.resultsTab.pareto` |
| Results Validate tab | `#resultsTabBar [data-tab="validate"]` | `shell.resultsTab.validate` |
| Results Candidate Detail tab | `#resultsTabBar [data-tab="drill"]` | `shell.resultsTab.drill` |
| Results Heatmap tab | `#resultsTabBar [data-tab="heatmap"]` | `shell.resultsTab.heatmap` |
| Footer details | Footer `Details` button | `footer.taskCenter.details` |
| Footer primary action | Footer run/validate button | `footer.primaryAction.run` |

### Command Palette Actions

Each palette row should expose `data-qa-action="commandPalette.command.<command-id>"`.

Initial command ids from the current command registry:

| Command id | Required `data-qa-action` |
| --- | --- |
| `run` | `commandPalette.command.run` |
| `validate` | `commandPalette.command.validate` |
| `browse_csv` | `commandPalette.command.browseCsv` |
| `open_polygon` | `commandPalette.command.openPolygon` |
| `export_csv` | `commandPalette.command.exportCsv` |
| `export_json` | `commandPalette.command.exportJson` |
| `open_latest_report` | `commandPalette.command.openLatestReport` |
| `open_latest_meta_report` | `commandPalette.command.openLatestMetaReport` |
| `toggle_controls` | `commandPalette.command.toggleControls` |
| `cancel` | `commandPalette.command.cancel` |
| `switch_guided_mode` | `commandPalette.command.switchGuidedMode` |
| `switch_advanced_mode` | `commandPalette.command.switchAdvancedMode` |
| `go_step_data` | `commandPalette.command.goStepData` |
| `go_step_optimize` | `commandPalette.command.goStepOptimize` |
| `go_step_validate` | `commandPalette.command.goStepValidate` |
| `go_step_analyze` | `commandPalette.command.goStepAnalyze` |
| `go_step_finalize` | `commandPalette.command.goStepFinalize` |
| `go_workspace_library` | `commandPalette.command.goWorkspaceLibrary` |
| `go_workspace_aggregate` | `commandPalette.command.goWorkspaceAggregate` |
| `go_workspace_pro` | `commandPalette.command.goWorkspaceDiagnostics` |
| `go_workspace_live` | `commandPalette.command.goWorkspaceLive` |
| `go_workspace_settings` | `commandPalette.command.goWorkspaceSettings` |

### Data Workspace Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Browse CSV | `data.csv.browse` |
| Toggle Polygon section | `data.polygon.toggle` |
| Load CSV from draft | `data.csv.loadDraft` |
| Clear data | `data.dataset.clear` |
| Apply recent CSV path | `data.csv.applyRecent` |
| Load sample dataset | `data.sample.load` |
| Refresh Polygon datasets/status | `data.polygon.refresh` |
| Close Polygon panel | `data.polygon.close` |
| Save Polygon API key | `data.polygon.apiKey.save` |
| Clear Polygon API key | `data.polygon.apiKey.clear` |
| Download Polygon dataset | `data.polygon.download` |
| Select cached Polygon dataset | `data.polygon.dataset.select` |
| Load selected Polygon dataset | `data.polygon.dataset.load` |
| Reveal selected Polygon dataset | `data.polygon.dataset.reveal` |
| Repair selected Polygon dataset | `data.polygon.dataset.repair` |
| Apply timeframe | `data.timeframe.apply` |
| Open market data directory | `data.storage.openMarketDataDir` |
| Open storage settings | `data.storage.openSettings` |

### Strategy, Optimize, And Advanced Setup Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Apply safer execution defaults | `strategy.execution.applySaferDefaults` |
| Apply strategy preset | `optimize.preset.apply` |
| Preset GPU kernel backend | `optimize.backend.presetKernel` |
| Refresh GPU status | `optimize.gpu.refreshStatus` |
| Run GPU parity check | `optimize.gpu.runParityCheck` |
| Add dynamic builder indicator | `optimize.dynamic.indicator.add` |
| Remove dynamic builder indicator | `optimize.dynamic.indicator.remove` |
| Add dynamic condition | `optimize.dynamic.condition.add` |
| Remove dynamic condition | `optimize.dynamic.condition.remove` |
| Add dynamic group | `optimize.dynamic.group.add` |
| Remove dynamic group | `optimize.dynamic.group.remove` |
| Preview dynamic builder | `optimize.dynamic.previewBuilder` |
| Preview dynamic IR JSON | `optimize.dynamic.previewIrJson` |
| Run optimization | `optimize.run` |
| Run validation from optimize panel | `optimize.validation.run` |
| Browse export-all path | `advanced.exportAll.browse` |
| Reset multi-objective search space | `advanced.multiObjective.resetSearchSpace` |

### Result Review And Analysis Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Overview empty primary action | `overview.empty.primary` |
| Overview empty secondary action | `overview.empty.secondary` |
| Review live execution | `overview.live.review` |
| Review Top results | `overview.top.review` |
| Copy best candidate | `overview.best.copy` |
| Export Top CSV | `overview.top.exportCsv` |
| Export Top JSON | `overview.top.exportJson` |
| Set compare baseline | `overview.compare.setBaseline` |
| Clear compare baseline | `overview.compare.clearBaseline` |
| Clear top filters | `top.filters.clear` |
| Open Candidate Detail | `top.selected.openCandidateDetail` |
| Review overview | `top.selected.reviewOverview` |
| Pin selected top candidate | `top.selected.pin` |
| Clear pinned top candidates | `top.pins.clear` |
| Restore top table default columns | `top.columns.restoreDefaults` |
| Sort top table column | `top.table.sortColumn` |
| Select top row | `top.table.selectRow` |
| Pin top row | `top.table.pinRow` |
| Replace pinned candidate with current | `top.pin.replaceWithCurrent` |
| Unpin candidate | `top.pin.remove` |
| Heatmap export PNG | `heatmap.exportPng` |
| Heatmap review top results | `heatmap.empty.reviewTopResults` |
| Heatmap candidate detail from cell | `heatmap.cell.candidateDetail` |
| Candidate Detail review Top results | `drill.empty.reviewTopResults` |
| Candidate Detail select trade row | `drill.trade.selectRow` |
| Pareto select point | `pareto.point.select` |

### Validation Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Run validation | `validation.run` |
| Export shortlist CSV | `validation.export.shortlistCsv` |
| Export validation JSON | `validation.export.validationJson` |
| Export report JSON | `validation.export.reportJson` |
| Export report XLSX | `validation.export.reportXlsx` |
| Open reports folder | `validation.reports.openFolder` |
| Refresh cross-ticker list | `validation.crossTicker.refresh` |
| Select all cross-ticker datasets | `validation.crossTicker.selectAll` |
| Select no cross-ticker datasets | `validation.crossTicker.selectNone` |
| Select matching cross-ticker datasets | `validation.crossTicker.selectMatching` |
| Restore validation table default columns | `validation.columns.restoreDefaults` |

### Library And Artifact Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Select session history run | `library.session.select` |
| Apply session | `library.session.apply` |
| Export preset | `library.session.exportPreset` |
| Import preset | `library.session.importPreset` |
| Clear session | `library.session.clear` |
| Save saved-run tags | `library.savedRun.tags.save` |
| Select saved run | `library.savedRun.select` |
| Refresh saved runs | `library.savedRun.refresh` |
| Load saved run | `library.savedRun.load` |
| Apply saved run | `library.savedRun.apply` |
| Rerun saved run | `library.savedRun.rerun` |
| Open saved run report | `library.savedRun.openReport` |
| Reveal saved run | `library.savedRun.reveal` |
| Delete saved run | `library.savedRun.delete` |
| Open runs directory | `library.runsDir.open` |

### Aggregate, Meta, And Campaign Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Select aggregate tab | `aggregate.tab.select` |
| Run batch generation | `aggregate.batch.run` |
| Stop batch generation | `aggregate.batch.stop` |
| Refresh batch queue | `aggregate.batch.refresh` |
| Select all batch datasets | `aggregate.batch.selectAll` |
| Select no batch datasets | `aggregate.batch.selectNone` |
| Retry failed batch items | `aggregate.batch.retryFailed` |
| Resume incomplete batch items | `aggregate.batch.resumeIncomplete` |
| Reveal batch summary | `aggregate.batch.revealSummary` |
| Select saved artifact family | `aggregate.saved.family.select` |
| Save selected artifact tags | `aggregate.saved.tags.save` |
| Load selected artifact | `aggregate.saved.loadSelected` |
| Refresh ML confidence | `aggregate.ml.refresh` |
| Train ML confidence | `aggregate.ml.train` |
| Apply ML confidence | `aggregate.ml.apply` |
| Delete ML confidence artifact | `aggregate.ml.delete` |
| Run Stage 5 | `aggregate.stage5.run` |
| Run ML policy evaluation | `aggregate.stage5.policyEvaluation.run` |
| Reveal Stage 5 files | `aggregate.stage5.reveal` |
| Run Meta selection | `aggregate.meta.run` |
| Open Meta report | `aggregate.meta.openReport` |
| Reveal Meta files | `aggregate.meta.revealFiles` |
| Restore Meta table default columns | `aggregate.meta.columns.restoreDefaults` |

### Options Expression Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Refresh options datasets | `options.datasets.refresh` |
| Import options dataset | `options.dataset.import` |
| Download options dataset | `options.dataset.download` |
| Reveal selected dataset path | `options.dataset.revealSelected` |
| Reveal latest download underlying path | `options.dataset.revealLatestUnderlying` |
| Preview feasibility | `options.feasibility.preview` |
| Run current expression | `options.expression.run` |
| Refresh options artifacts | `options.artifacts.refresh` |
| Load options artifact | `options.artifacts.load` |
| Reveal active artifact | `options.artifacts.revealActive` |
| Export active CSV | `options.artifacts.exportCsv` |
| Load artifact from comparison | `options.comparison.loadArtifact` |
| Focus tuple index | `options.tuple.focus` |
| Select tuple | `options.tuple.select` |
| Load selected tuple inspection | `options.inspection.loadTuple` |
| Export active inspection CSV | `options.inspection.exportCsv` |
| Show all inspection rows | `options.inspection.showAllRows` |
| Select inspection row | `options.inspection.selectRow` |
| Load selected event path | `options.eventPath.load` |
| Export active event path CSV | `options.eventPath.exportCsv` |

### Live Execution Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Preview risk policy | `live.risk.previewPolicy` |
| Save risk policy | `live.risk.savePolicy` |
| Test account | `live.account.test` |
| Refresh orders | `live.account.refreshOrders` |
| Reset local paper | `live.paper.reset` |
| Run once | `live.runner.runOnce` |
| Start live stream | `live.stream.start` |
| Stop live stream | `live.stream.stop` |
| Start runner | `live.runner.start` |
| Stop runner | `live.runner.stop` |
| Cancel open orders | `live.safety.cancelOpenOrders` |
| Flatten position | `live.safety.flattenPosition` |
| Emergency halt | `live.safety.emergencyHalt` |
| Clear keys | `live.credentials.clearKeys` |
| Refresh saved sessions | `live.savedSessions.refresh` |
| Select saved session evidence | `live.savedSessions.select` |

### Guided Mode Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Guided primary next action | `guided.primaryAction.run` |
| Switch to Advanced Mode | `guided.advancedMode.open` |
| Select guided step | `guided.step.select` |
| Open aggregate from guided checklist | `guided.aggregate.open` |
| Browse CSV | `guided.data.csv.browse` |
| Load CSV | `guided.data.csv.load` |
| Load sample | `guided.data.sample.load` |
| Open Polygon | `guided.data.polygon.open` |
| Apply timeframe | `guided.data.timeframe.apply` |
| Clear data | `guided.data.clear` |
| Open market data directory | `guided.data.marketDataDir.open` |
| Choose DoubleMa template | `guided.strategy.template.doubleMa` |
| Choose CompositeCpu template | `guided.strategy.template.compositeCpu` |
| Continue to optimize | `guided.strategy.continueToOptimize` |
| Open Advanced optimize | `guided.optimize.advanced.open` |
| Run optimization | `guided.optimize.run` |
| Cancel optimization | `guided.optimize.cancel` |
| Continue to validation | `guided.optimize.continueToValidate` |
| Run validation | `guided.validation.run` |
| Continue to trade readiness | `guided.validation.continueToTrade` |
| Open Advanced validation | `guided.validation.advanced.open` |
| Run once | `guided.trade.runOnce` |
| Start live | `guided.trade.live.start` |
| Stop live | `guided.trade.live.stop` |
| Start runner | `guided.trade.runner.start` |
| Stop runner | `guided.trade.runner.stop` |
| Open Advanced live | `guided.trade.advanced.open` |

### Settings And Diagnostics Actions

| Current surface | Required `data-qa-action` |
| --- | --- |
| Request desktop notifications | `settings.notifications.request` |
| Browse market data directory | `settings.marketDataDir.browse` |
| Apply market data directory | `settings.marketDataDir.apply` |
| Reset market data directory | `settings.marketDataDir.reset` |
| Open market data directory | `settings.marketDataDir.open` |
| Open workflow data directory | `settings.workflowData.open` |
| Refresh artifact retention info | `settings.retention.refresh` |
| Apply artifact retention drafts | `settings.retention.apply` |
| Reset artifact retention drafts | `settings.retention.reset` |
| Apply selected perspective | `settings.perspective.apply` |
| Overwrite selected perspective | `settings.perspective.overwrite` |
| Create perspective | `settings.perspective.create` |
| Rename perspective | `settings.perspective.rename` |
| Duplicate perspective | `settings.perspective.duplicate` |
| Delete perspective | `settings.perspective.delete` |
| Open runs directory | `settings.runsDir.open` |
| Open reports directory | `settings.reportsDir.open` |
| Open Polygon modal | `settings.polygon.open` |
| Go to Library | `settings.navigation.library` |
| Go to Aggregate | `settings.navigation.aggregate` |
| Open task center | `settings.diagnostics.taskCenter.open` |
| Copy diagnostics snapshot | `settings.diagnostics.copySnapshot` |
| Clear recent alerts | `settings.diagnostics.clearAlerts` |
| Clear desktop logs | `settings.diagnostics.clearDesktopLogs` |

## Implementation Appendix D: Starter Action Coverage Manifest

The first manifest should include high-risk actions before lower-value maintenance actions. This keeps Phase 2 useful even before every button is annotated.

```ts
export interface QaActionCoverageEntry {
  actionId: string
  surface: string
  required: boolean
  priority: 'critical' | 'important' | 'extended'
  coveredBy: string[]
  waiver?: {
    reason: string
    owner: string
    expires?: string
  }
}

export const actionCoverageManifest: QaActionCoverageEntry[] = [
  {
    actionId: 'shell.commandPalette.open',
    surface: 'desktop titlebar',
    required: true,
    priority: 'critical',
    coveredBy: ['shell.commandPalette.navigation', 'responsive.mobile.commandPalette'],
  },
  {
    actionId: 'shell.workspace.live',
    surface: 'workspace navigation',
    required: true,
    priority: 'critical',
    coveredBy: ['shell.workspace.tabs', 'live.empty'],
  },
  {
    actionId: 'data.csv.browse',
    surface: 'data workspace',
    required: true,
    priority: 'critical',
    coveredBy: ['data.csv.import.success', 'guided.data.load'],
  },
  {
    actionId: 'optimize.run',
    surface: 'optimize workspace',
    required: true,
    priority: 'critical',
    coveredBy: ['optimize.success.doubleMa', 'optimize.failure.backend'],
  },
  {
    actionId: 'validation.run',
    surface: 'validation workspace',
    required: true,
    priority: 'critical',
    coveredBy: ['validation.success.selected', 'validation.blocked.datasetMismatch'],
  },
  {
    actionId: 'top.selected.openCandidateDetail',
    surface: 'top results',
    required: true,
    priority: 'critical',
    coveredBy: ['results.candidateDetail'],
  },
  {
    actionId: 'library.savedRun.load',
    surface: 'run library',
    required: true,
    priority: 'critical',
    coveredBy: ['library.openRun'],
  },
  {
    actionId: 'live.safety.emergencyHalt',
    surface: 'live execution',
    required: true,
    priority: 'critical',
    coveredBy: ['live.emergencyHalt'],
  },
  {
    actionId: 'options.expression.run',
    surface: 'options expression',
    required: true,
    priority: 'important',
    coveredBy: ['options.optimize.success'],
  },
  {
    actionId: 'settings.diagnostics.clearDesktopLogs',
    surface: 'settings diagnostics',
    required: false,
    priority: 'extended',
    coveredBy: [],
    waiver: {
      reason: 'Maintenance action. Covered after diagnostics workflow stabilization.',
      owner: 'frontend-qa',
      expires: '2026-06-01',
    },
  },
]
```

Manifest rules:

- `critical` actions cannot be waived for release candidates unless a product owner explicitly removes the feature from release scope.
- `important` actions can be temporarily waived only with an owner and expiry.
- `extended` actions can be waived for short periods but still appear in coverage reports.
- A waiver expiry date is interpreted as end of day local release timezone.

## Implementation Appendix E: Fixture Schema And Examples

Use named fixtures instead of building command responses inline in every spec.

```ts
export interface MockTauriScenario {
  name: string
  description: string
  localStorage?: Record<string, string>
  sessionStorage?: Record<string, string>
  commandResults: Record<string, unknown>
  expectedInvokes?: string[]
  emitAfterLoad?: Array<{
    event: string
    payload: unknown
  }>
}
```

### Empty App Fixture

```ts
export const emptyAppScenario: MockTauriScenario = {
  name: 'empty-app',
  description: 'Fresh app with no dataset, no latest run, and no saved live sessions.',
  commandResults: {
    load_latest_run_artifact_hydration: null,
    load_latest_run_artifact_summary: null,
    list_live_session_artifacts: [],
    load_live_session_artifact_detail: null,
    get_gpu_status: {
      cuda_feature: false,
      cuda_backtest_kernel_feature: false,
      device_count: 0,
      devices: [],
      message: 'CUDA not enabled in this test fixture.',
    },
  },
}
```

### Optimization Success Fixture

```ts
export const optimizationSuccessScenario: MockTauriScenario = {
  name: 'optimization-success-double-ma',
  description: 'Loaded dataset and successful conventional DoubleMa optimization.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    load_latest_run_artifact_hydration: null,
    load_csv: {
      data_id: 'fixture_aapl_1m',
      rows: 500,
      timeframe: '1m',
      label: 'AAPL fixture',
      range: '2026-04-01 -> 2026-04-02',
    },
    run_optimization: {
      status: 'ok',
      route: 'CpuOnly',
      selected_index: 0,
      top: [
        {
          row_id: 'cand_1',
          fast: 8,
          slow: 32,
          pnl: 0.12,
          sharpe: 1.4,
          max_dd: 0.05,
          trades: 18,
        },
      ],
      heatmap: {
        fast_values: [8, 12],
        slow_values: [32, 48],
        values: [1.4, 1.2, 1.1, 1.5],
      },
    },
  },
  expectedInvokes: ['load_csv', 'run_optimization'],
}
```

### Validation Dataset Mismatch Fixture

```ts
export const validationDatasetMismatchScenario: MockTauriScenario = {
  name: 'validation-dataset-mismatch',
  description: 'Latest optimization artifact exists but the matching dataset is not loaded.',
  commandResults: {
    load_latest_run_artifact_hydration: {
      data_id: 'fixture_saved_dataset',
      strategy_kind: 'DoubleMa',
      selected_candidate: {
        fast: 8,
        slow: 32,
      },
    },
    run_validation: Promise.reject(
      new Error('Load the same dataset used by the optimization artifact before running validation.'),
    ),
  },
  expectedInvokes: ['load_latest_run_artifact_hydration', 'run_validation'],
}
```

If promises in fixture objects are awkward for Playwright serialization, represent failures explicitly:

```ts
{
  run_validation: {
    __mockFailure: 'Load the same dataset used by the optimization artifact before running validation.'
  }
}
```

Then have `installMockedTauri` convert `__mockFailure` into a rejected invoke.

## Implementation Appendix F: Shared QA Assertions

Every workflow spec should call a shared guard before navigation.

```ts
export interface QaGuardOptions {
  allowedConsoleErrors?: RegExp[]
  failOnRequestFailure?: boolean
  busyTimeoutMs?: number
}

export function installQaGuards(page: Page, options: QaGuardOptions = {}) {
  const consoleErrors: string[] = []
  const pageErrors: string[] = []
  const requestFailures: string[] = []

  page.on('console', (msg) => {
    if (msg.type() !== 'error') return
    const text = msg.text()
    if ((options.allowedConsoleErrors ?? []).some((pattern) => pattern.test(text))) return
    consoleErrors.push(text)
  })

  page.on('pageerror', (error) => {
    pageErrors.push(error.stack || error.message)
  })

  if (options.failOnRequestFailure ?? true) {
    page.on('requestfailed', (request) => {
      const url = request.url()
      if (url.startsWith('data:') || url.startsWith('blob:')) return
      requestFailures.push(`${request.failure()?.errorText || 'failed'} ${url}`)
    })
  }

  return {
    async assertClean() {
      expect(pageErrors, 'uncaught page errors').toEqual([])
      expect(consoleErrors, 'unexpected console errors').toEqual([])
      expect(requestFailures, 'failed app requests').toEqual([])
    },
  }
}
```

Add focused helpers:

```ts
export async function expectNoHorizontalDocumentOverflow(page: Page) {
  const overflow = await page.evaluate(() => {
    const root = document.documentElement
    return root.scrollWidth - root.clientWidth
  })
  expect(overflow, 'document horizontal overflow px').toBeLessThanOrEqual(2)
}

export async function expectNoStuckBusyState(page: Page, timeoutMs = 5000) {
  await expect
    .poll(async () => {
      return page.locator('[aria-busy="true"], button:has-text("Running"), button:has-text("Cancelling")').count()
    }, { timeout: timeoutMs })
    .toBe(0)
}
```

## Implementation Appendix G: Workflow Runner Skeleton

Use a small runner so each workflow reads like a user story.

```ts
export interface WorkflowCase {
  id: string
  title: string
  tags: string[]
  scenario: MockTauriScenario
  viewport?: { width: number; height: number }
  run: (ctx: WorkflowContext) => Promise<void>
}

export interface WorkflowContext {
  page: Page
  context: BrowserContext
  guard: ReturnType<typeof installQaGuards>
  action: (id: string) => Locator
  state: (id: string) => Locator
  panel: (id: string) => Locator
}

export function defineWorkflow(testCase: WorkflowCase) {
  test(`${testCase.title} ${testCase.tags.join(' ')}`, async ({ page, context }) => {
    if (testCase.viewport) await page.setViewportSize(testCase.viewport)
    const guard = installQaGuards(page)
    await installMockedTauriScenario(context, testCase.scenario)
    await gotoMockedShell(page, context)

    const ctx: WorkflowContext = {
      page,
      context,
      guard,
      action: (id) => page.locator(`[data-qa-action="${id}"]`),
      state: (id) => page.locator(`[data-qa-state="${id}"]`),
      panel: (id) => page.locator(`[data-qa-panel="${id}"]`),
    }

    await testCase.run(ctx)
    await guard.assertClean()
  })
}
```

Example workflow:

```ts
defineWorkflow({
  id: 'validation.blocked.datasetMismatch',
  title: 'validation explains dataset mismatch without unhandled errors',
  tags: ['@workflow', '@regression'],
  scenario: validationDatasetMismatchScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.validate').click()
    await action('validation.run').click()
    await expect(page.getByText('Validation failed:')).toBeVisible()
    await expect(page.getByText('Load the same dataset used by the optimization artifact')).toBeVisible()
  },
})
```

## Implementation Appendix H: Action Coverage Test Skeleton

```ts
test('all visible QA actions are covered or waived @workflow', async ({ page, context }) => {
  await installMockedTauriScenario(context, allSurfacesVisibleScenario)
  await gotoMockedShell(page, context)

  const discovered = new Set<string>()
  for (const state of canonicalActionInventoryStates) {
    await state.open(page)
    const ids = await page.locator('[data-qa-action]').evaluateAll((nodes) =>
      nodes
        .map((node) => String((node as HTMLElement).dataset.qaAction || '').trim())
        .filter(Boolean),
    )
    ids.forEach((id) => discovered.add(id))
  }

  const entries = new Map(actionCoverageManifest.map((entry) => [entry.actionId, entry]))
  const unknown = [...discovered].filter((id) => !entries.has(id))
  const uncovered = [...entries.values()].filter((entry) =>
    entry.required && entry.coveredBy.length === 0 && !entry.waiver,
  )
  const expiredWaivers = [...entries.values()].filter((entry) =>
    entry.waiver?.expires && Date.parse(entry.waiver.expires) < Date.now(),
  )

  await writeActionCoverageReport({ discovered: [...discovered], unknown, uncovered, expiredWaivers })

  expect(unknown, 'visible actions missing from manifest').toEqual([])
  expect(uncovered, 'required actions without coverage').toEqual([])
  expect(expiredWaivers, 'expired action coverage waivers').toEqual([])
})
```

Canonical action inventory states should include:

- advanced shell workflow/data
- advanced workflow/optimize with controls tabs visible
- advanced workflow/analyze with result tabs visible
- validation step with artifact loaded
- library with saved runs
- aggregate with saved artifacts
- ML Signal mode
- options expression surface visible
- live workspace
- settings diagnostics
- guided mode at each guided step
- compact mobile command palette

## Implementation Appendix I: Pass/Fail Thresholds

These thresholds should be encoded in shared helpers instead of repeated in specs.

| Risk | Default threshold | Failure condition |
| --- | --- | --- |
| Page errors | 0 | Any uncaught `pageerror` fails |
| Console errors | 0 unless allowlisted | Any unallowlisted console `error` fails |
| Unexpected Tauri invokes | 0 | Any strict mocked invoke miss fails |
| Local asset request failures | 0 | Any failed app asset request fails |
| Document horizontal overflow | `<= 2px` | More than 2px overflow fails target viewport |
| Critical action click completion | `<= 5000ms` | Expected state not visible within timeout |
| Busy-state recovery after mocked success/failure | `<= 5000ms` | Busy controls/spinners still active |
| Dialog close after Escape/cancel | `<= 1000ms` | Dialog remains visible |
| Focus return after overlay close | exact expected element | Focus missing or wrong target |
| Canvas nonblank check | at least 1 percent non-background pixels | Heatmap/Pareto/chart canvas appears blank |
| Action coverage unknown ids | 0 | Visible action missing from manifest |
| Required uncovered actions | 0 | Required action has no coverage and no waiver |
| Expired waivers | 0 | Waiver expiry date has passed |
| Critical action waivers in release gate | 0 | Critical action waived for release candidate |

Console allowlist format:

```ts
export const allowedConsoleErrors = [
  {
    pattern: /ResizeObserver loop completed/,
    reason: 'Chromium ResizeObserver noise if it appears in this environment.',
    removeBy: '2026-06-01',
  },
]
```

Every allowlist entry needs a reason and removal date.

## Implementation Appendix J: Priority Rollout

### Critical First Pass

Implement these first because they cover the most user-visible regressions:

1. `shell.launch.advanced`
2. `shell.workspace.tabs`
3. `shell.commandPalette.navigation`
4. `data.empty`
5. `data.sample.load`
6. `strategy.doubleMa.basic`
7. `optimize.blocked.noData`
8. `optimize.success.doubleMa`
9. `optimize.failure.backend`
10. `validation.blocked.noCandidate`
11. `validation.blocked.datasetMismatch`
12. `validation.success.selected`
13. `results.topTable`
14. `results.heatmap`
15. `results.candidateDetail`
16. `library.openRun`
17. `live.empty`
18. `live.emergencyHalt`
19. `settings.diagnostics`
20. `responsive.mobile.commandPalette`

### Important Second Pass

1. Guided full path.
2. CompositeTrend route policy.
3. BreakoutMomentum route policy.
4. RegimeHybrid route policy.
5. Pareto result selection.
6. Validation exports.
7. Library many-runs virtual scrolling.
8. Aggregate saved artifacts.
9. Stage 5 ML confidence actions.
10. Options expression happy path.
11. Live saved sessions.
12. Desktop packaged smoke.

### Extended Third Pass

1. Dynamic strategy builder.
2. CompositeMo and advanced multi-objective settings.
3. Campaign workflows.
4. Options tuple inspection and event path.
5. Full action coverage for maintenance/settings actions.
6. Exploratory domain sweep.
7. Alternate viewport matrix.

## Implementation Appendix K: CI Examples

Example PR lane:

```yaml
name: frontend-qa-pr
on:
  pull_request:
    paths:
      - "web/**"
      - "src-tauri/**"
      - "docs/frontend_end_to_end_qa_automation_plan.md"

jobs:
  browser-workflow:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: web/package-lock.json
      - run: npm --prefix web ci
      - run: npm --prefix web run typecheck
      - run: npm --prefix web run lint
      - run: npm --prefix web run test
      - run: npm --prefix web run test:e2e:workflow
      - run: npm --prefix web run test:e2e:action-coverage
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: playwright-artifacts
          path: output/playwright/
```

Example release candidate lane:

```yaml
name: frontend-qa-release-candidate
on:
  workflow_dispatch:

jobs:
  browser-and-desktop:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: web/package-lock.json
      - uses: dtolnay/rust-toolchain@stable
      - run: npm --prefix web ci
      - run: npm --prefix web run typecheck
      - run: npm --prefix web run lint
      - run: npm --prefix web run parity
      - run: npm --prefix web run test
      - run: npm --prefix web run test:e2e:workflow:full
      - run: npm --prefix web run test:e2e:action-coverage
      - run: npm --prefix web run a11y:smoke
      - run: npm --prefix web run baseline:ui
      - run: npm --prefix web run test:desktop:smoke
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: frontend-qa-output
          path: output/
```

Desktop smoke on hosted CI may need explicit `tauri-driver` and matching EdgeDriver setup. If that is unstable, keep desktop smoke as a required local release-candidate gate until the runner image is pinned.

## Implementation Appendix L: Required Code Review Checklist

Every frontend PR that adds or changes UI actions must answer:

- Did every new product-owned action get a stable `data-qa-action`?
- Did every new high-signal panel/state get `data-qa-panel` or `data-qa-state` where useful?
- Was the action coverage manifest updated?
- Does at least one workflow cover the new action?
- If the action is waived, does the waiver have owner, reason, and expiry?
- Does the workflow fail on page errors and unexpected console errors?
- Does the workflow assert visible user-facing state after the action?
- Does the workflow cover failure or blocked state if the backend can reject the action?
- Does compact viewport need coverage?
- Does desktop smoke need an update, or is mocked browser coverage sufficient?

## Coverage Reporting

The workflow QA report should include:

- Total workflows.
- Passed/failed/skipped workflows.
- Total discovered actions.
- Required actions covered.
- Required actions waived.
- Required actions uncovered.
- Expired waivers.
- Unexpected console/page/request errors.
- Unexpected Tauri invokes.
- Slowest workflows.
- Artifact directory.

Example report shape:

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-04-25T00:00:00.000Z",
  "workflows": {
    "total": 84,
    "passed": 84,
    "failed": 0,
    "skipped": 0
  },
  "actions": {
    "discovered": 210,
    "required": 198,
    "covered": 198,
    "waived": 12,
    "uncovered": []
  },
  "errors": {
    "pageErrors": 0,
    "consoleErrors": 0,
    "unexpectedInvokes": 0
  }
}
```

## Acceptance Criteria

This initiative is complete when:

- E2E harness files are tracked and runnable from a clean checkout.
- Browser workflow QA covers every workflow family listed in this plan.
- Every required visible action is covered or explicitly waived.
- Critical compact viewport workflows pass.
- A11y smoke passes.
- Desktop smoke passes on a correctly provisioned Windows machine.
- Release candidate gate documentation includes the E2E commands.
- At least one previously found workflow bug is represented as a permanent E2E regression.

## Practical Limits

Even a comprehensive E2E suite cannot cover every possible market dataset, every user input value, every GPU/runtime combination, every OS state, or every live broker response. The system should instead make the product hard to break in ordinary use by covering:

- All intended user journeys.
- All visible actions.
- Representative data states.
- Representative backend success/failure states.
- Representative viewport and keyboard paths.
- Real native shell launch and navigation.

The action coverage manifest is the key control. It turns "full coverage" from a vague goal into an enforceable contract.
