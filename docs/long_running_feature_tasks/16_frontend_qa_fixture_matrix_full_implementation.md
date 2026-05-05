# Frontend QA Fixture Matrix Full Implementation

Status: implemented locally on 2026-05-04
Risk: low to moderate; primarily E2E flake risk and fixture drift risk
Primary value: durable browser and desktop QA coverage for the product workflows most likely to regress during frontend iteration

## Summary

This plan fully implements the frontend QA fixture matrix program described in the earlier frontend QA plans. The goal is to move from useful but fragmented browser E2E coverage to a structured, checkable QA system where fixtures, workflows, visible actions, evidence artifacts, and optional desktop smoke all have clear ownership.

The implementation is additive. It must not change trading logic, optimizer ranking, validation scoring, CUDA behavior, broker behavior, saved artifact semantics, or default release authority. The value comes from better regression detection and faster diagnosis.

## Implementation Result

The fixture matrix program is now implemented as a local QA infrastructure slice.

- Fixture ownership is explicit through `web/e2e/browser/fixtures/types.ts`, family modules, and `fixtures/index.ts`.
- Browser workflows now launch registered `MockTauriScenario` objects and enforce `expectedInvokes` / `forbiddenInvokes`.
- The fixture matrix gate emits `output/qa/e2e-fixture-matrix.json` and fails local/unregistered workflow scenarios, unused registered scenarios, oversized fixture JSON packs, invalid family membership, duplicate names, secret-like fixture values, and expected/forbidden command overlap.
- Action coverage now fails `coveredBy` references that do not match real workflow IDs.
- Validation, optimization, settings, ML Signal, live, options, artifact-library, responsive, and failure-state workflows have deterministic browser coverage.
- Aggregate Meta run/open/reveal coverage, Aggregate Batch campaign lifecycle coverage, Composite profile route-summary coverage, and the Task Center progress lifecycle are now represented by registered deterministic fixtures.
- Aggregate saved-run tag updates now have deterministic action coverage instead of a manifest waiver.
- The mocked Tauri bridge supports delayed command completion and scheduled command events for realistic long-running browser workflows.
- QA summary/dashboard now include fixture-family evidence and fail on missing, failed, or stale prerequisite QA reports.
- QA summary/dashboard now include expected/actual visual snapshot evidence with file hashes; missing or extra snapshots fail, while stale snapshots are reported as review context.
- Optional desktop smoke has an expanded suite and stale-summary protection without changing the default desktop smoke lane.
- Required workflow backlog coverage is now machine-checkable with explicit aliases and no remaining deferred entries.
- Dataset sample-load success, live stream stop, command-palette workflow actions, and contextual-help tooltip role coverage are first-class registered workflows.
- Accessibility smoke now emits structured JSON evidence and includes validation workflow coverage.
- The frontend E2E gate `-IncludeQaReport` path now writes the full static/perf summary and dashboard report.
- Runtime action coverage artifacts include report metadata, viewport context, and Playwright JSON reporter output for easier review.

External implementation references used:

- Playwright fixture guidance: https://playwright.dev/docs/test-fixtures
- Playwright visual comparison guidance: https://playwright.dev/docs/test-snapshots
- Playwright reporter guidance: https://playwright.dev/docs/test-reporters
- Tauri WebDriver guidance: https://v2.tauri.app/develop/tests/webdriver/

## Validation Evidence

Passed locally:

- `npm --prefix web run build`
- `npm --prefix web run typecheck`
- `npm --prefix web run typecheck:desktop`
- `npm --prefix web run check:e2e-fixture-matrix`
- `npm --prefix web run check:qa-actions`
- `npm --prefix web run check:qa-report`
- `npm --prefix web run test -- src/services/optimizeService.test.ts`
- `npm --prefix web run test:e2e:workflow:core:built`
- `npm --prefix web run test:e2e:regressions:built`
- `npm --prefix web run test:e2e:action-coverage:built`
- `npm --prefix web run test:e2e:responsive:built`
- `npm --prefix web run test:e2e:workflow:fixtures:built`
- `npm --prefix web run test:e2e:help-tooltips:built`
- `npm --prefix web run test:e2e:visual:built`
- `npm --prefix web run a11y:smoke:built`
- `npm --prefix web run test:desktop:smoke:expanded`
- `npm --prefix web run qa:dashboard`

Current generated QA evidence:

- Fixture matrix: 36 registered scenarios, 52 registered workflow references, 0 local workflow scenarios.
- Required workflow backlog: 22 covered, 0 deferred, 0 missing.
- Action manifest: 109 source QA actions represented by 162 manifest entries; 105 of 105 required entries covered.
- Regression browser lane: 32 passed.
- Core workflow browser lane: 10 passed.
- Runtime action coverage browser lane: 2 passed.
- Responsive/layout browser lane: 7 passed.
- Focused fixture workflow lane: 44 passed.
- Help tooltip browser lane: 5 passed.
- Visual baseline browser lane: 4 passed.
- Accessibility smoke: passed across 6 scanned surfaces.
- Expanded desktop smoke lane: 8 passed.

## Current Anchors

- Browser fixtures: `web/e2e/browser/fixtures/index.ts` plus domain modules under `web/e2e/browser/fixtures/`
- Browser harness: `web/e2e/browser/support/workflowRunner.ts`
- Mocked Tauri bridge: `web/e2e/browser/support/mockTauri.ts`
- QA assertions: `web/e2e/browser/support/qaAssertions.ts`
- Workflow specs: `web/e2e/browser/qa/workflowMatrix.spec.ts`
- Regression specs: `web/e2e/browser/qa/regressionRepros.spec.ts`
- Responsive specs: `web/e2e/browser/qa/responsiveWorkflow.spec.ts`
- Help tooltip specs: `web/e2e/browser/help-tooltips.spec.ts`
- Action manifest: `web/e2e/browser/qa/actionCoverageManifest.ts`
- Static QA scripts:
  - `web/scripts/check-action-coverage-manifest.ts`
  - `web/scripts/check-qa-selector-inventory.mjs`
  - `web/scripts/check-e2e-artifact-fixtures.mjs`
  - `web/scripts/write-local-qa-summary.mjs`
  - `web/scripts/write-frontend-qa-dashboard.mjs`
- Desktop smoke harness:
  - `web/wdio.desktop.conf.ts`
  - `web/e2e/desktop/`
- Existing parent plans:
  - `docs/frontend_end_to_end_qa_automation_plan.md`
  - `docs/long_running_feature_tasks/08_frontend_qa_automation_expansion.md`
  - `docs/long_running_feature_tasks/12_frontend_qa_fixture_matrix_ci_artifacts_desktop_smoke.md`

## Product Target

The finished QA system should answer these questions locally and repeatably:

1. Which product-owned workflows are covered?
2. Which visible actions are covered, waived, or drifting?
3. Which mocked backend states are represented by named fixtures?
4. Which Tauri commands are expected for each workflow?
5. Which surfaces have browser evidence for empty, blocked, success, failure, and recovery states?
6. Which optional desktop checks ran, and where are their artifacts?
7. What evidence should a reviewer inspect when a UI change claims it is safe?

## Additive Boundary

Allowed:

- Reorganize E2E fixtures into named family modules.
- Add fixture metadata, static checks, dashboard fields, and browser workflows.
- Add stable `data-qa-*` selectors where the product owns the action or state.
- Add mocked Tauri command responses for deterministic workflows.
- Add optional desktop smoke suites and summary artifacts.
- Add local scripts and package scripts for fixture and QA evidence checks.
- Update docs that explain how frontend QA evidence should be produced.

Not allowed in this plan:

- No trading calculation changes.
- No optimizer ranking changes.
- No validation verdict or evidence algorithm changes.
- No CUDA route, kernel, PTX, benchmark, or performance-baseline changes.
- No broker mutation behavior changes.
- No active CI reactivation.
- No tests that require live provider, broker, internet, or GPU access.
- No sleeps as the primary synchronization mechanism. Use locators, state assertions, event evidence, or deterministic mocked command completion.

## Definitions

**Fixture family**

A domain grouping for mocked app state and Tauri command responses. Examples: `empty`, `dataset`, `optimization`, `validation`, `artifactLibrary`, `options`, `mlSignal`, `campaign`, `liveExecution`, and `settings`.

**Scenario**

A named mocked app state that a browser workflow can launch. A scenario owns local storage, session storage, command result maps, expected invokes, and fixture metadata.

**Workflow**

A Playwright test case declared through `defineWorkflow`. A workflow must use one scenario, exercise a product path, and assert meaningful UI state.

**Action coverage**

The manifest-backed accounting that maps visible product actions to one or more workflows, or to an explicit waiver with owner and expiry.

**QA dashboard**

The local JSON and Markdown evidence written under `output/qa/` that summarizes fixture matrix health, workflow coverage, action coverage, selector inventory, visual evidence, and optional desktop smoke.

## Implementation Plan

### Phase 0: Baseline Inventory

Goal: freeze current behavior before changing fixture layout.

Checklist:

- [ ] Run `npm --prefix web run check:qa-static` and save the important failure/pass summary in the implementation notes.
- [ ] Run `npm --prefix web run qa:dashboard` and inspect `output/qa/frontend-qa-dashboard.md`.
- [ ] Inventory every `defineWorkflow({ id })` in `web/e2e/browser/qa/`.
- [ ] Inventory every exported scenario in `web/e2e/browser/fixtures/scenarios.ts`.
- [ ] Inventory current visible `data-qa-action`, `data-qa-panel`, and `data-qa-state` selectors using `check:qa-selectors`.
- [ ] Identify large inline command responses that should move into fixture family modules.
- [ ] Record current action manifest waivers that are expired or near expiry.

Expected output:

- A short local implementation note in the final handoff, not a committed generated artifact.
- No product code changes in this phase.

### Phase 1: Fixture Type Contract

Goal: make fixture ownership explicit and statically checkable.

Add:

- `web/e2e/browser/fixtures/types.ts`
- `web/e2e/browser/fixtures/base.ts`
- `web/e2e/browser/fixtures/index.ts`

Recommended type shape:

```ts
export type FixtureFamilyId =
  | 'empty'
  | 'dataset'
  | 'optimization'
  | 'validation'
  | 'artifactLibrary'
  | 'aggregate'
  | 'options'
  | 'mlSignal'
  | 'campaign'
  | 'liveExecution'
  | 'settings'

export interface MockTauriScenario {
  name: string
  family: FixtureFamilyId
  description: string
  urlSearch?: string
  localStorage?: Record<string, string>
  sessionStorage?: Record<string, string>
  commandResults: Record<string, unknown>
  expectedInvokes?: string[]
  forbiddenInvokes?: string[]
  tags?: string[]
}
```

Rules:

- Keep `strictUnknownInvoke` as the default browser workflow behavior.
- Every scenario must declare `family`.
- Every scenario name must be unique.
- Every command result key must be a non-empty Tauri command name.
- `expectedInvokes` means the workflow must call the command at least once.
- `forbiddenInvokes` means the workflow must not call the command.
- Scenario names should use kebab-case.
- Fixture modules should export scenarios, not helper-only anonymous objects.

Compatibility:

- Keep `web/e2e/browser/fixtures/scenarios.ts` as a compatibility re-export during the first pass.
- Do not force all specs to migrate in one patch. Move workflows as they are touched.

### Phase 2: Fixture Family Split

Goal: break `scenarios.ts` into domain-owned fixture modules.

Target file layout:

```text
web/e2e/browser/fixtures/
  artifacts/
    saved-options-artifact-pack.json
  types.ts
  base.ts
  empty.ts
  dataset.ts
  optimization.ts
  validation.ts
  artifactLibrary.ts
  aggregate.ts
  options.ts
  mlSignal.ts
  campaign.ts
  liveExecution.ts
  settings.ts
  index.ts
  scenarios.ts
```

Family responsibilities:

- `empty.ts`: fresh app, empty shell, blocked actions, no latest artifact.
- `dataset.ts`: loaded OHLCV data, load failure, sample CSV behavior, provider/cache states.
- `optimization.ts`: DoubleMa success, failure, cancellation-ready, composite route summary, GPU-unavailable fallback display.
- `validation.ts`: dataset mismatch, blocked validation, accepted validation, review validation, rejected validation, overfit evidence states.
- `artifactLibrary.ts`: saved run list, hydrated artifact detail, repair dry run/write, delete/tag actions, many-run list.
- `aggregate.ts`: Meta/Stage surfaces ready, empty, failure, compare baseline.
- `options.ts`: options artifact library, parent artifact follow-up, validation feasibility, validation result.
- `mlSignal.ts`: missing model bundle, unavailable status, ready model, validation failure.
- `campaign.ts`: campaign unsupported strategy, failed campaign, resumable campaign summary.
- `liveExecution.ts`: dry-run ready, credential missing, broker failure, safety confirmation surfaces.
- `settings.ts`: diagnostics, storage config, artifact retention, updater state.

Migration rules:

- Move fixtures without changing serialized values unless the scenario needs new metadata.
- Keep large JSON packs under `fixtures/artifacts/`.
- Prefer small builder helpers only when they remove real duplication.
- Do not hide command result maps behind overly clever factory functions. A reviewer should be able to see what the frontend receives.

### Phase 3: Fixture Matrix Static Check

Goal: make fixture drift visible before running browser tests.

Add:

- `web/scripts/check-e2e-fixture-matrix.mjs`
- Package script: `check:e2e-fixture-matrix`
- Include this script in `check:qa-static`.

The script should emit:

- `output/qa/e2e-fixture-matrix.json`
- Optional human-readable summary line in stdout.

Checks:

- All scenario names are unique.
- All scenario names are kebab-case.
- All scenarios declare a valid `family`.
- Every family has at least one scenario, except explicitly deferred families.
- Every scenario has non-empty `description`.
- Every scenario has a command result object.
- Every command result key is non-empty.
- `expectedInvokes` and `forbiddenInvokes` do not overlap.
- Every workflow references an existing scenario object.
- Every scenario used by a workflow is reported with workflow IDs.
- No scenario should contain likely secret key names or credential-shaped values.
- No fixture JSON pack exceeds a documented size threshold without an explicit allowlist.

Suggested JSON output:

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-05-04T00:00:00.000Z",
  "status": "passed",
  "families": {
    "validation": {
      "scenarioCount": 4,
      "workflowCount": 3,
      "scenarios": ["validation-dataset-mismatch"]
    }
  },
  "issues": []
}
```

Regression watchpoint:

- The check must not require Playwright to run.
- The check must tolerate optional future families if they are explicitly listed as deferred with a reason.

### Phase 4: Workflow Runner Contract Hardening

Goal: make workflows enforce scenario command contracts consistently.

Update `web/e2e/browser/support/workflowRunner.ts`:

- Import scenario types from `fixtures/types.ts`.
- Enforce `expectedInvokes`.
- Enforce `forbiddenInvokes`.
- Include scenario family/name in assertion messages.
- Add helper accessors for invoke log assertions.
- Keep `installQaGuards(page)` default behavior.
- Add optional workflow metadata for `priority`, `surfaces`, and `fixtureFamilies`.

Recommended additions:

```ts
export interface WorkflowCase {
  id: string
  title: string
  tags: string[]
  scenario: MockTauriScenario
  priority?: 'critical' | 'important' | 'extended'
  surfaces?: string[]
  viewport?: { width: number; height: number }
  run: (ctx: WorkflowContext) => Promise<void>
}
```

Do not add:

- Global retry logic beyond Playwright config.
- Workflow-specific arbitrary waits as normal practice.
- Non-strict mocked Tauri behavior except for narrowly documented exploratory specs.

### Phase 5: Workflow Matrix Expansion

Goal: cover realistic user paths, not just shell reachability.

Add or extend workflows in `web/e2e/browser/qa/workflowMatrix.spec.ts`, `regressionRepros.spec.ts`, or new domain-specific specs when a file would become too large.

Required workflow backlog:

| Workflow ID | Fixture family | Required state |
| --- | --- | --- |
| `data.sample.load.success` | dataset | Sample data action loads a deterministic dataset and updates readiness. |
| `data.sample.load.failure` | dataset | Sample load failure surfaces a recoverable message without stuck busy state. |
| `optimize.loaded.success` | optimization | Loaded dataset can run optimization and display top result, metrics, and route summary. |
| `optimize.loaded.failure` | optimization | Optimization command failure leaves controls usable and surfaces a precise failure state. |
| `optimize.composite.routeSummary` | optimization | Composite profile route copy is visible and consistent with mocked route metadata. |
| `validation.blocked.datasetMismatch` | validation | Existing mismatch path stays blocked and actionable. |
| `validation.success.accepted` | validation | Accepted result displays verdict, evidence summary, and overfit evidence. |
| `validation.success.review` | validation | Review result displays warning tone without claiming acceptance. |
| `validation.success.rejected` | validation | Rejected result displays rejection tone and export/report affordance state. |
| `validation.export.actions` | validation | Shortlist/report export actions enable only when evidence exists. |
| `artifact.library.repair` | artifactLibrary | Repair dry-run and write states are reachable and summarized. |
| `artifact.library.detailHydration` | artifactLibrary | Saved run detail hydrates top metrics, route, validation, and tags. |
| `taskCenter.progress.lifecycle` | optimization | Long-running mocked task shows progress, completion, and clean close. |
| `commandPalette.workflowActions` | empty/dataset | Command palette actions navigate and respect readiness states. |
| `live.safety.confirmations` | liveExecution | Broker-sensitive actions require confirmation and do not run with missing credentials. |
| `settings.diagnostics.storage` | settings | Storage and artifact-retention diagnostics are visible and recoverable. |
| `options.artifact.followup` | options | Options artifact follow-up panel opens and validates mocked feasibility state. |
| `mlSignal.unavailable.failure` | mlSignal | Missing model bundle and failed ML action are understandable. |
| `responsive.mobile.primaryPath` | dataset/optimization | Compact viewport covers data -> optimize -> validation navigation without overflow. |
| `help.tooltips.visibleRole` | empty/validation/liveExecution | Every visible shared help trigger opens exactly one visible tooltip role. |

Workflow quality rules:

- Every workflow should assert at least one panel, one action, and one state where applicable.
- Every workflow should end with `expectNoStuckBusyState` for run-like paths.
- Use `expectNoHorizontalDocumentOverflow` for responsive and dense layout paths.
- Use action IDs for product actions, not display text, unless the text itself is the contract.
- Keep failure-path workflows as first-class coverage, not incidental assertions in happy paths.

### Phase 6: Action Coverage Manifest Completion

Goal: make action coverage meaningful enough to guide reviews.

Update `web/e2e/browser/qa/actionCoverageManifest.ts` and `check-action-coverage-manifest.ts`:

- Require every `coveredBy` workflow ID to exist.
- Require every required critical action to have at least one `coveredBy`.
- Require waivers to have owner, reason, and expiry.
- Fail expired waivers.
- Group dashboard output by action surface and priority.
- Add a `lastReviewed` field only if the check enforces a useful policy.

Waiver rules:

- Waivers are acceptable for actions that require external resources, native dialogs, active broker sessions, provider credentials, or large generated artifacts.
- Waivers are not acceptable for normal empty, blocked, success, failure, and recovery states that can be mocked.
- Waivers should expire in 30 to 90 days.

### Phase 7: QA Dashboard Expansion

Goal: make local QA evidence easy to inspect after a long run.

Update `web/scripts/write-frontend-qa-dashboard.mjs` to include:

- Fixture matrix status from `output/qa/e2e-fixture-matrix.json`.
- Fixture family summary.
- Workflow IDs grouped by family and surface.
- Required action coverage by priority.
- Expired or near-expiry waivers.
- Selector inventory counts.
- Playwright report/artifact directory metadata.
- Visual snapshot count.
- Tooltip regression spec status if available from Playwright output metadata.
- Desktop smoke summary if present.
- Git branch, revision, and dirty status.

Dashboard output remains:

- `output/qa/frontend-qa-dashboard.json`
- `output/qa/frontend-qa-dashboard.md`

Do not require previous E2E artifacts to exist. Missing optional artifacts should be reported as `not present`, not as a hard failure, unless the caller is a stricter gate.

### Phase 8: Desktop Smoke Expansion

Goal: add optional native-shell evidence without making desktop tests the main workflow gate.

Add:

- Optional WDIO suite: `smoke-expanded`
- Structured output: `output/wdio/desktop-smoke-expanded-summary.json`

Candidate checks:

- Real desktop shell launches.
- Titlebar controls are visible and invoke native handlers.
- Workspace navigation reaches workflow, library, aggregate, portfolio, live, settings.
- Command palette opens/closes.
- Task center opens/closes.
- Settings diagnostics panel renders native runtime facts.
- App logs are copied to `output/wdio/logs/` when available.

Rules:

- Keep existing `test:desktop:smoke` stable and small.
- Add a separate package script for expanded smoke.
- Do not promote expanded smoke into default gates until it has a stable local track record.

### Phase 9: Targeted Accessibility And Visual Coverage

Goal: catch UI regressions that action coverage cannot see.

Add targeted accessibility checks inside browser workflows for:

- Command palette.
- Task center.
- Validation evidence panel.
- Live safety confirmation dialog.
- Settings diagnostics.
- Compact/mobile navigation.

Add or update visual baselines for:

- Desktop workflow data state.
- Desktop validation evidence state.
- Desktop artifact library detail.
- Mobile advanced shell.
- Mobile validation state.

Rules:

- Visual snapshots should be updated only for intentional UI changes.
- Accessibility scans should focus on critical states to avoid noisy broad sweeps.
- Do not block implementation on non-critical color-contrast findings unless the changed surface introduced them.

### Phase 10: Scripts And Gate Wiring

Goal: make the new evidence easy to run locally.

Package script changes:

- Add `check:e2e-fixture-matrix`.
- Add it to `check:qa-static`.
- Keep `check:qa-report` as static checks plus dashboard generation.
- Add a focused browser workflow script if useful:
  - `test:e2e:workflow:fixtures`
  - or keep using existing `test:e2e:workflow:core` if the matrix remains in one spec.
- Add optional desktop script:
  - `test:desktop:smoke:expanded`

Suggested command ladder:

```powershell
npm --prefix web run typecheck
npm --prefix web run check:qa-static
npm --prefix web run check:qa-report
npm --prefix web run test:e2e:workflow:core
npm --prefix web run test:e2e:regressions
npm --prefix web run test:e2e:responsive
npm --prefix web run test:e2e:action-coverage
```

Optional local evidence:

```powershell
npm --prefix web run test:e2e:visual
npm --prefix web run a11y:smoke:wcag
npm --prefix web run typecheck:desktop
npm --prefix web run test:desktop:smoke
npm --prefix web run test:desktop:smoke:expanded
```

Do not activate GitHub workflows as part of this implementation. If disabled workflow metadata is touched, it must remain under `.github/workflows.disabled/`.

## Recommended Implementation Order

1. Add fixture types and compatibility re-exports.
2. Add fixture matrix static check and dashboard integration.
3. Split `empty`, `dataset`, `optimization`, and `validation` fixtures first.
4. Migrate existing workflows to imported fixture family modules.
5. Add validation and optimization success/failure workflows.
6. Tighten action manifest references and waiver expiry enforcement.
7. Add artifact library, options, ML, live, and settings fixtures.
8. Add the remaining workflow backlog.
9. Add optional desktop expanded smoke summary.
10. Run static, unit, browser, and optional desktop gates.

This order front-loads static safety before the browser workflow expansion. It also avoids touching every domain fixture before the checker and dashboard can prove whether the split is useful.

## Regression Watchpoints

- Fixture split must not make mocked Tauri less strict.
- Static checks must not silently waive missing coverage.
- Browser workflows must not depend on real local data, provider credentials, live network, GPU, or broker state.
- Generated artifacts must stay under `output/`.
- Dashboard generation must not fail just because optional E2E output is missing.
- Desktop smoke must stay optional until stable.
- Action IDs must remain stable across copy changes.
- Tests should use product state and selectors, not incidental CSS structure, except for shared component regressions like tooltip role checks.

## Validation Checklist

Minimum for fixture/checker-only changes:

- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run check:qa-static`
- [ ] `npm --prefix web run check:qa-report`

Minimum for browser workflow additions:

- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run test:e2e:workflow:core`
- [ ] `npm --prefix web run test:e2e:regressions`
- [ ] `npm --prefix web run test:e2e:action-coverage`

For responsive or layout-sensitive workflow additions:

- [ ] `npm --prefix web run test:e2e:responsive`
- [ ] `npm --prefix web run test:e2e:visual` if snapshots are touched

For accessibility-sensitive workflow additions:

- [ ] `npm --prefix web run a11y:smoke:wcag`

For desktop smoke additions:

- [ ] `npm --prefix web run typecheck:desktop`
- [ ] `npm --prefix web run test:desktop:smoke`
- [ ] `npm --prefix web run test:desktop:smoke:expanded` on a provisioned desktop test environment

Broad final handoff, when practical:

- [ ] `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

## Definition Of Done

This program is complete when:

- Fixture families are split into domain modules and re-exported through `fixtures/index.ts`.
- `scenarios.ts` remains only as compatibility glue or is removed after all imports migrate.
- `check:e2e-fixture-matrix` exists, emits JSON evidence, and is part of `check:qa-static`.
- The QA dashboard reports fixture family health, workflow coverage, action coverage, selector inventory, visual snapshot count, and optional desktop smoke status.
- Critical visible actions in `actionCoverageManifest.ts` are covered by real workflow IDs or have explicit, non-expired waivers.
- The required workflow backlog has coverage or documented deferrals.
- Browser workflows remain deterministic with strict mocked Tauri commands.
- Desktop expanded smoke is available as an optional lane with structured output.
- The validation checklist passes for the slices touched.

## First Implementation Slice

If this is started as the next long-running Codex task, the first slice should be:

1. Add `fixtures/types.ts`, `fixtures/base.ts`, `fixtures/index.ts`.
2. Add `web/scripts/check-e2e-fixture-matrix.mjs`.
3. Add `check:e2e-fixture-matrix` and wire it into `check:qa-static`.
4. Split only `empty.ts`, `dataset.ts`, `optimization.ts`, and `validation.ts`.
5. Keep `scenarios.ts` as compatibility re-export.
6. Update `write-frontend-qa-dashboard.mjs` to include fixture matrix evidence.
7. Run `typecheck`, `check:qa-static`, and `check:qa-report`.

That slice creates the foundation without forcing every workflow and every domain fixture to move at once.
