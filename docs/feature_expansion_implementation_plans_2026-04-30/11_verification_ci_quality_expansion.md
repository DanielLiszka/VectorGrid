# Verification, CI, And Quality Expansion Plan

Date: 2026-04-30

## Goal

Expand verification from a collection of strong local gates into a clearer evidence system: targeted tests first, authority-aligned gates second, CI reactivation only after local contracts are stable, and artifacts that explain what was proven.

## Current Owner Files

Gate authority:

- `docs/verification_benchmark_release_authority.md`
- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workspace_tests_stable.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_output_parity_check.mjs`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_frontend_e2e_gate.ps1`

Frontend QA:

- `web/package.json`
- `web/e2e/browser/qa/*`
- `web/scripts/check-action-coverage-manifest.ts`
- `web/scripts/check-qa-selector-inventory.mjs`
- `web/scripts/check-e2e-artifact-fixtures.mjs`
- `web/scripts/write-frontend-qa-dashboard.mjs`
- `web/e2e/accessibility-review-matrix.md`

CI:

- `.github/workflows/cpu-quality.yml`
- `scripts/run_cpu_ci_gate.ps1`
- `docs/CI.md`
- `.github/workflows.disabled/ci.yml.disabled` (archived reference)
- `.github/workflows.disabled/desktop-smoke.yml.disabled` (archived reference)

## Research Basis

- Playwright recommends testing user-visible behavior, test isolation, locators, and web-first assertions: <https://playwright.dev/docs/best-practices>
- Playwright locators prioritize roles, text, labels, alt text, titles, and test IDs for resilient tests: <https://playwright.dev/docs/locators>
- Playwright accessibility snapshots can support stable structural assertions, but they should complement visible-behavior tests rather than replace them: <https://playwright.dev/docs/aria-snapshots>
- Vitest Browser Mode is a candidate for component tests where jsdom is too abstract and full E2E is too costly: <https://vitest.dev/guide/browser/>
- GitHub Actions artifact retention is configurable and should be intentional for CI evidence: <https://docs.github.com/organizations/managing-organization-settings/configuring-the-retention-period-for-github-actions-artifacts-and-logs-in-your-organization>
- GitHub Actions cache behavior and artifact uploads should be treated as evidence plumbing with retention and redaction policy, not as proof by themselves: <https://docs.github.com/actions/using-workflows/caching-dependencies-to-speed-up-workflows>
- Criterion is useful for statistics-driven Rust benchmarking, but repo authority depends on matching timing boundaries: <https://docs.rs/criterion/latest/criterion/>

Detailed frontend script mapping lives in `docs/feature_expansion_implementation_plans_2026-04-30/15_frontend_qa_gate_wiring_matrix.md`.

## Expansion Track A: Claim-To-Gate Dashboard

### Problem

The repo already has many gates, but developers need a quick way to choose the right one.

### Implementation

Generate a local dashboard summary from:

- `docs/verification_benchmark_release_authority.md`
- script metadata;
- last local output under `output/`.

Potential new script:

- `scripts/write_verification_dashboard.mjs`

Output:

- `output/verification_dashboard/summary.json`
- `output/verification_dashboard/summary.md`

Fields:

- claim type;
- primary gate;
- last run timestamp;
- status;
- artifact path;
- stale/missing warning;
- authority caveat.

### Acceptance Criteria

- Dashboard does not run gates.
- It never treats missing local evidence as pass.
- It distinguishes companion diagnostics from authority gates.

## Expansion Track B: Frontend Fixture Matrix

### Problem

Frontend E2E coverage should be driven by representative user workflows and stable fixture ownership.

### Implementation

Current owners:

- `web/e2e/browser/qa/workflowMatrix.spec.ts`
- `web/e2e/browser/qa/actionCoverage.spec.ts`
- `web/e2e/browser/qa/workbenchShell.spec.ts`
- fixture scripts under `web/scripts`.

Add fixture classes:

- empty app;
- loaded sample dataset;
- optimization result;
- validation result;
- artifact history with missing report;
- options artifact;
- portfolio feasibility result;
- execution dry-run snapshot.

### Tests

- fixture validator;
- action coverage report;
- E2E smoke for each representative workflow.

## Expansion Track C: Output Parity Promotion

### Problem

Report/export shape changes need a first-class gate.

### Implementation

Current owners:

- `scripts/run_output_parity_check.mjs`
- `scripts/output_parity_pairs.json`
- report command modules.

Add:

- CSV parity pairs where headless outputs exist;
- options output parity;
- portfolio export parity;
- public release integration only after runtime and artifact cost are known.

### Acceptance Criteria

- Output parity remains separate from perf authority.
- XLSX invariants are semantic/sheet-level, not byte equality.
- Generated baseline files remain intentionally local or explicitly reviewed if promoted.

## Expansion Track D: CI Reactivation Plan

### Problem

The legacy workflows remain disabled archives. A reviewed CPU-only workflow is
prepared in the working tree at `.github/workflows/cpu-quality.yml`; GPU, native
desktop, credentialed-provider, signing, and clean-machine evidence still
require separate lanes.

### Current implementation status (2026-07-24)

- `Required CPU quality` is configured for pull requests, pushes, merge queues,
  and manual dispatches once the workflow is committed and enabled remotely.
- `scripts/run_cpu_ci_gate.ps1` fail-fast delegates to the current repo quality,
  serialized Rust/frontend tests, full headless Chromium QA, and output parity.
- QA evidence has a seven-day retention window and remains diagnostic rather
  than release authority.
- After the first clean GitHub run, repository branch protection must require
  `Required CPU quality`.
- See `docs/CI.md` for the exact contract and exclusions.

### Implementation

Review disabled workflows first:

- `.github/workflows.disabled/ci.yml.disabled`
- `.github/workflows.disabled/desktop-smoke.yml.disabled`

Phase 1 CI:

- formatting;
- cargo check CPU-only;
- frontend typecheck/lint;
- command parity/bindings checks;
- artifact fixture static checks.

Phase 2 CI:

- targeted Rust tests;
- frontend unit tests;
- output parity smoke with small fixtures.

Phase 3 CI:

- desktop smoke on suitable Windows runner;
- browser E2E slices;
- release doctor dry run.

Do not put GPU benchmarks into general PR CI unless dedicated hardware and serialization policy exist.

### Acceptance Criteria

- CI artifact retention is configured.
- CI artifacts redact sensitive paths/secrets.
- CI does not become release authority without matching local release evidence.

## Expansion Track E: Benchmark Baseline Governance

### Problem

Benchmark baselines are powerful but can be contaminated by concurrency or boundary mismatch.

### Implementation

Extend benchmark scripts to report:

- case registry version;
- timing boundary;
- hardware identity;
- build profile;
- feature flags;
- baseline path;
- candidate path;
- benchmark lock status;
- competing process scan result where available.

Add local helper:

- baseline refresh checklist in Markdown;
- candidate summary comparison.

### Acceptance Criteria

- Candidate mode never updates baseline.
- Baseline refresh is explicit.
- Near-threshold failures require serialized rerun before conclusion.

## Second-Pass Expansion: Exact Frontend Gate Wiring

### Problem

Generic "run frontend tests" language is not enough for a large Vue/Tauri workbench. Each feature type should map to the smallest meaningful script set first, then broader gates.

### Script Matrix

Use `web/package.json` as the source of truth and keep `15_frontend_qa_gate_wiring_matrix.md` synchronized. The core lanes are:

- static/type: `npm --prefix web run typecheck`, `npm --prefix web run lint`;
- unit/component: `npm --prefix web run test`;
- production build: `npm --prefix web run build`;
- bundle/perf: `npm --prefix web run check:bundle-budget`, `npm --prefix web run perf:ui:budget`;
- QA static: `npm --prefix web run check:qa-static`;
- browser QA: `npm --prefix web run test:e2e:qa:built`;
- accessibility: `npm --prefix web run a11y:smoke:built`, `npm --prefix web run a11y:smoke:wcag:built`;
- desktop boundary: `npm --prefix web run security:desktop-boundary`.

Feature-to-gate rule:

- changed request builders or services: unit tests plus typecheck;
- changed command bindings/capabilities: desktop boundary plus Tauri parity checks;
- changed tables/charts/workbench UI: unit/component tests plus browser QA slice;
- changed accessibility-sensitive controls: accessibility smoke plus focused component tests;
- changed release resources: release doctor plus manifest/config sync checks.

### Acceptance Criteria

- each new feature plan names a focused gate before broad repo gates;
- browser QA fixtures are owned and validated before specs depend on them;
- CI reactivation starts with stable local scripts only.

## Second-Pass Expansion: Visual Snapshot And Accessibility Policy

### Problem

Screenshots and snapshots can become brittle unless their purpose is narrow.

### Policy

- use visual snapshots for stable, fixed-format surfaces such as Simple Mode summary, report preview, and deterministic chart fixtures;
- avoid visual snapshots for live timestamps, randomized samples, or OS/WebView-specific chrome;
- use ARIA/accessibility snapshots only for structure that should remain stable;
- keep explicit text/role assertions for the user-visible behavior under test;
- store baseline update instructions with the owning spec.

## Non-Goals

- No broad GPU perf in default CI.
- No disabled workflows treated as active authority.
- No output parity as semantic trading correctness.
- No single "run everything" command for routine work.

## Verification

Targeted:

- run changed script tests/checks as applicable;
- `npm --prefix web run check:qa-static`;
- `npm --prefix web run qa:dashboard`.

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests` for cross-layer backend changes.
