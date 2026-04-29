# Frontend QA Automation Expansion

Status: proposed
Risk: low to moderate
Primary value: reduce regression risk across large Vue workflow surfaces

## Summary

The frontend has many large workflow panels and services. Expanding fixture-backed browser QA, stable action selectors, and coverage manifests is mostly additive and directly supports the riskier feature tasks in this folder.

## Current Anchors

- Existing plan: `docs/frontend_end_to_end_qa_automation_plan.md`
- Browser fixtures: `web/e2e/browser/fixtures/`
- Browser specs: `web/e2e/browser/`
- Large panels: `web/src/components/OptionsExpressionPanel.vue`, `MetaWorkspacePanel.vue`, `OptimizeWorkspacePanel.vue`, `ValidationWorkspacePanel.vue`, `ExecutionWorkspacePanel.vue`
- Task/progress store: `web/src/stores/taskStore.ts`

## Additive Boundary

- Add selectors and fixtures without changing product behavior.
- Tests should mock Tauri strictly and fail on unexpected commands.
- Do not make slow exploratory sweeps mandatory in default gates until stable.
- Keep visual/a11y checks scoped and opt-in until flake rate is known.

## Implementation Checklist

### Phase 1: Selector Coverage

- [ ] Add `data-qa-action` to primary workflow actions that lack stable ids.
- [ ] Add `data-qa-panel` and `data-qa-state` for high-signal assertions.
- [ ] Update action coverage manifest.
- [ ] Add or update tests that inventory visible `[data-qa-action]` elements.
- [ ] Keep selectors stable and semantic, not tied to CSS or copy.
- [ ] Configure Playwright's test id attribute to the repo's chosen selector convention if `getByTestId` is used.
- [ ] Prefer role/text assertions for visible user behavior and reserve test ids for dense controls or workflow actions.

### Phase 2: Fixture Families

- [ ] Add fixture modules for data import, optimization, validation, analysis, artifact library, aggregate/meta, ML Signal, options, live execution, settings, and diagnostics.
- [ ] Move inline mocked command responses into named fixtures.
- [ ] Add failure fixtures for dataset mismatch, command failure, cancellation, stale artifacts, and unsupported routes.
- [ ] Make mocked Tauri command handling strict by default.

### Phase 3: Core Workflow Matrix

- [ ] Add browser workflow tests for data import to optimization to validation.
- [ ] Add analysis/result review workflows.
- [ ] Add saved artifact open/replay workflows.
- [ ] Add cancellation/recovery cases for long-running commands.
- [ ] Add task center progress assertions.

### Phase 4: Advanced Surface Matrix

- [ ] Add aggregate/meta/campaign workflows.
- [ ] Add ML Signal workflows.
- [ ] Add options expression workflows.
- [ ] Add live execution safety/readiness workflows.
- [ ] Add settings/diagnostics workflows.
- [ ] Add representative failure/recovery cases for each advanced surface.

### Phase 5: Reporting And CI

- [ ] Add local coverage report for action selectors and workflow scenarios.
- [ ] Keep flaky or slow suites out of default CI until stable.
- [ ] Document packaged smoke steps for release candidates.
- [ ] Review waivers before release.
- [ ] When CI integration is added, upload browser traces, screenshots, and coverage reports as workflow artifacts.
- [ ] Use matrix jobs for independent fixture families only after local flake rate is acceptable.

## Validation Checklist

- [ ] `npm --prefix web run test`
- [ ] `npm --prefix web run typecheck`
- [ ] Run targeted browser E2E specs added by this task.
- [ ] Run action coverage manifest check.
- [ ] Run repo quality gate for broad selector/test infrastructure changes.

## Regression Watchpoints

- Selector additions must not alter layout or copy.
- Strict fixtures can reveal existing unexpected calls; classify failures as product bug, fixture gap, or waiver.
- Avoid introducing brittle timing waits.
- Keep tests deterministic and fixture-backed.

## Definition Of Done

- High-value workflows have strict fixture-backed browser coverage.
- Primary actions have stable selector ids.
- Coverage gaps are visible in a local report.
- The harness can protect the higher-risk tasks in this folder.
