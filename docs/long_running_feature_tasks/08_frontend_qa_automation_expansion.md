# Frontend QA Automation Expansion

Status: implemented for targeted workspace/action coverage expansion
Risk: low to moderate
Primary value: reduce regression risk across large Vue workflow surfaces

## Summary

The frontend has many large workflow panels and services. Expanding fixture-backed browser QA, stable action selectors, and coverage manifests is mostly additive and directly supports the riskier feature tasks in this folder.

Implemented slice:

- Added action/panel coverage entries for the new portfolio feasibility workspace and updated workbench navigation coverage.
- Expanded browser QA manifests/specs for action coverage, exploratory sweep, and workflow matrix coverage.
- Preserved fixture-backed/strict-command direction without making slow sweeps mandatory in default gates.
- Verified the action coverage manifest after selector updates.

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

- [x] Add `data-qa-action` to primary workflow actions that lack stable ids in touched surfaces.
- [x] Add `data-qa-panel` and `data-qa-state` for high-signal assertions in touched surfaces.
- [x] Update action coverage manifest.
- [x] Add or update tests that inventory visible `[data-qa-action]` elements.
- [x] Keep selectors stable and semantic, not tied to CSS or copy.
- [x] Keep the repo's existing selector convention.
- [x] Prefer role/text assertions for visible user behavior and reserve test ids for dense controls or workflow actions.

### Phase 2: Fixture Families

- [x] Reuse existing fixture-backed browser harness for touched workflow surfaces.
- [ ] Add fixture modules for every remaining workflow family.
- [ ] Move all inline mocked command responses into named fixtures.
- [ ] Add failure fixtures for dataset mismatch, command failure, cancellation, stale artifacts, and unsupported routes.
- [x] Preserve strict mocked Tauri command handling where existing specs use it.

### Phase 3: Core Workflow Matrix

- [x] Update browser workflow matrix coverage for touched workspace navigation/action surfaces.
- [x] Add analysis/result review coverage entries for touched surfaces.
- [x] Add saved artifact/portfolio workspace coverage entries.
- [ ] Add cancellation/recovery cases for all long-running commands.
- [ ] Add task center progress assertions for every workflow family.

### Phase 4: Advanced Surface Matrix

- [x] Preserve existing aggregate/meta/options/live execution coverage entries.
- [ ] Add full campaign workflows.
- [ ] Add full ML Signal workflows.
- [x] Add options expression coverage for touched finalist dashboard state through component tests.
- [x] Add live execution safety/readiness coverage for bracket table presentation through component/service tests.
- [ ] Add representative failure/recovery cases for each advanced surface.

### Phase 5: Reporting And CI

- [x] Preserve local coverage report/check for action selectors and workflow scenarios.
- [x] Keep flaky or slow suites out of default CI until stable.
- [ ] Document packaged smoke steps for release candidates.
- [ ] Review waivers before release.
- [ ] When CI integration is added, upload browser traces, screenshots, and coverage reports as workflow artifacts.
- [ ] Use matrix jobs for independent fixture families only after local flake rate is acceptable.

## Validation Checklist

- [x] `npm --prefix web run test -- portfolioWorkspaceModel portfolioService workflowModel workbenchNavigationModel workflowStore CommandPaletteOverlay WorkbenchShell`
- [x] `npm --prefix web run typecheck`
- [x] Updated targeted browser E2E specs/manifests added by this task.
- [x] `npm --prefix web run check:qa-actions`
- [x] Run repo quality gate for broad selector/test infrastructure changes.

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
