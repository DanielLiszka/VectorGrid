# Frontend QA Fixture Matrix, CI Artifacts, And Desktop Smoke Expansion

Status: dashboard, manifest cleanup, and disabled-CI metadata implemented; fixture-family split and desktop expansion remain planned
Risk: low to moderate, mostly test-infrastructure flake risk
Primary value: make large workflow UI regressions easier to catch and diagnose without making slow suites default

## Summary

The frontend QA surface now has action coverage and browser workflow checks, but fixture ownership and evidence artifacts are still fragmented. The next safe slice is to split fixture families, clean manifest drift, emit a local QA dashboard, and keep desktop smoke expansion optional until flake rate is known.

GitHub Actions supports matrix jobs and workflow artifacts; Playwright supports screenshots and visual comparisons; OpenTelemetry defines CI/CD resource attributes that can be mirrored in dashboard metadata. Tauri documents a WebdriverIO desktop testing path, which matches the repo's current WDIO setup.

Sources:

- GitHub Actions matrix jobs: https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations
- GitHub Actions workflow artifacts: https://docs.github.com/en/actions/tutorials/store-and-share-data
- Playwright visual comparisons: https://playwright.dev/docs/test-snapshots
- OpenTelemetry CI/CD resource semantic conventions: https://opentelemetry.io/docs/specs/semconv/resource/cicd/
- Tauri WebdriverIO testing: https://tauri.app/develop/tests/webdriver/example/webdriverio/

## Current Anchors

- Browser fixtures: `web/e2e/browser/fixtures/`
- Browser QA specs: `web/e2e/browser/qa/`
- Action coverage manifest: `web/e2e/browser/qa/actionCoverageManifest.ts`
- QA summary script: `web/scripts/write-local-qa-summary.mjs`
- Desktop WDIO config/specs: `web/wdio.desktop.conf.ts`, `web/e2e/desktop/`
- Disabled workflows: `.github/workflows.disabled/`
- Existing parent plan: `08_frontend_qa_automation_expansion.md`

## Existing Behavior

- Browser fixtures are largely concentrated in `scenarios.ts`.
- Workflow matrix covers core shell and selected workflows.
- Action coverage manifest reports required/waived actions.
- Local QA summary exists.
- GitHub workflows are intentionally disabled.
- Desktop smoke proves launch/navigation/titlebar/settings basics; geometry remains opt-in.

## Additive Boundary

- Add or reorganize test fixtures without changing product behavior.
- Keep strict mocked Tauri invokes.
- Keep slow, visual, exploratory, desktop, and packaged smoke suites outside default gates until stable.
- Do not reactivate workflows under `.github/workflows/` in this slice.
- Disabled workflow metadata may be documented, but local authority remains repo scripts.

## Non-Goals

- No new active CI requirement.
- No GPU/CUDA matrix jobs.
- No browser tests that depend on live network/broker/provider data.
- No arbitrary sleeps or timing-based assertions.
- No desktop geometry suite promotion.

## Implementation Checklist

### Phase 1: Fixture Ownership

- [ ] Add fixture family modules for empty app, dataset loaded, optimization, validation, artifact library, options, live execution, ML Signal, and campaign.
- [ ] Re-export through `fixtures/index.ts` and keep compatibility from `scenarios.ts`.
- [ ] Add `check:e2e-fixture-matrix` to validate names, command maps, expected invokes, and family ownership.
- [ ] Move inline mocked command responses into named fixtures as workflows are touched.

### Phase 2: Workflow Matrix Expansion

- [ ] Add first-class workflows for optimize success/failure, composite route summary, aggregate ready, ML empty/failure, campaign failure, and task-center progress.
- [x] Fix manifest drift for missing `coveredBy` references.
- [ ] Keep expanded families in full/regression lanes until stable.

### Phase 3: Local QA Dashboard

- [x] Add `qa:dashboard` script that emits `output/qa/frontend-qa-dashboard.json`.
- [x] Emit `output/qa/frontend-qa-dashboard.md` for human review.
- [x] Include Playwright report status, action coverage, fixture checks, visual snapshot count, WDIO summary if present, git SHA if available, and CI-like metadata.
- [x] Use stable keys compatible with OpenTelemetry CI/CD naming concepts where useful.

### Phase 4: Disabled CI Artifact Metadata

- [x] Document future artifact names and matrix dimensions in `.github/workflows.disabled/README.md`.
- [ ] If disabled workflow YAML is edited, keep it disabled and use unique artifact names per matrix row.
- [ ] Upload artifacts with `if: always()` only when workflows are intentionally reactivated later.

### Phase 5: Desktop Smoke Expansion

- [ ] Add an optional `smoke-expanded` WDIO suite for shell parity and native diagnostics.
- [ ] Add structured desktop smoke summary JSON in WDIO `onComplete`.
- [ ] Keep packaged smoke and geometry opt-in.
- [ ] Copy app logs for managed debug smoke where available.

## Validation Checklist

- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run check:qa-static`
- [ ] `npm --prefix web run check:qa-report`
- [ ] `npm --prefix web run test:e2e:qa`
- [ ] `npm --prefix web run test:e2e:regressions`
- [ ] `npm --prefix web run test:e2e:responsive`
- [ ] `npm --prefix web run typecheck:desktop`
- [ ] `npm --prefix web run test:desktop:smoke` only on a provisioned desktop test environment.

## Regression Watchpoints

- Fixture split must not weaken strict command handling.
- Manifest cleanup must not hide untested actions with broad waivers.
- Dashboard generation must not require prior E2E output to exist.
- Artifact paths must remain under `output/` and avoid secrets.
- Disabled workflows must stay non-authoritative.

## Definition Of Done

- Fixture ownership is visible and checkable.
- QA dashboard provides local evidence without CI dependency.
- Desktop smoke expansion is available but not required by default.
- Future CI artifact shape is documented without reactivating CI.
