# Regression Safety Rules

Last updated: 2026-04-09

## Purpose

Define the non-regression rules that every future implementation plan must preserve.

## Scope Boundaries

In scope:

- behavior regressions
- contract regressions
- report / artifact regressions
- verification and perf-gate expectations

Out of scope:

- adding new product scope
- options-related workflows

## Current User-Facing Behavior

Current user-visible behavior is protected by a mix of frontend tests, Tauri tests, desktop/browser harnesses, parity scripts, report tests, and perf gates, but the rules are spread across the repo rather than stated in one place.

## Current Code Surface

- `web/package.json`
- `web/e2e/browser/README.md`
- `web/e2e/desktop/README.md`
- `src-tauri/tests/command_runtime_wiring.rs`
- `src-tauri/tests/mock_runtime_smoke.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/check_tauri_config_sync.mjs`
- `scripts/run_output_parity_check.mjs`

## Current Verification Surface

- Vitest component/store/service tests
- Playwright browser UX tests
- WDIO desktop smoke tests
- Rust unit/integration tests
- XLSX regression tests
- parity / config-sync scripts
- perf gates and benchmark harnesses

## Strengths Already Present

- there are already multiple verification layers
- report generation already has structural tests
- Tauri command wiring is already checked
- perf and parity tooling already exists

## Weaknesses / Failure Modes

- different layers can drift without a shared regression checklist
- path persistence, artifact hydration, and export semantics are easy to break quietly
- UI changes can degrade evidence density while still passing type checks
- approximate routes can drift away from exact-anchor semantics if not explicitly guarded

## Regressions To Avoid

- removing or narrowing existing functionality without explicit approval
- changing request defaults or route semantics without updating verification and docs
- silently changing artifact/report schema or path fields
- weakening frontend coverage by over-mocking or by testing implementation details instead of visible behavior
- increasing host/device churn on GPU paths without a clear justification
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Playwright Best Practices](https://playwright.dev/docs/best-practices) recommends testing user-visible behavior and keeping tests isolated.
- [Vitest Mocking Guide](https://vitest.dev/guide/mocking.html) highlights mocking hazards such as hoisting and misuse of spies.
- [Pinia Testing Stores](https://pinia.vuejs.org/cookbook/testing.html) supports explicit store test setup instead of ad-hoc globals.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that heavy work should remain async and command boundaries should stay explicit.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) emphasizes minimizing host/device transfers and batching work when GPU paths are performance-critical.
- [GitHub tasklists](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/about-tasklists)
  - relevant because the planning layer needs executable checklists, explicit acceptance criteria, and visible progress semantics rather than vague prose

## Proposed Improvement Approach

Every implementation plan should include a regression section that answers:

- what exact current behaviors must not change
- which tests or scripts prove those behaviors now
- what new checks need to be added if the current coverage is insufficient
- what fallback or compatibility behavior must survive the change
1. Treat planning-layer quality as part of the implementation scope: keep the program auditable, explicit, refreshable, and easy for future contributors to execute without guesswork.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Required Non-Regression Matrix

- shell and UX changes
  - preserve mount/readiness behavior, navigation continuity, keyboard flow, and visible status messaging
  - verify with targeted frontend tests plus browser or desktop checks when the behavior is visible
- data changes
  - preserve dataset identity, provenance, restore behavior, and source/repair semantics
  - verify with request/response or artifact checks plus any affected workflow smoke tests
- optimization changes
  - preserve request defaults, route selection, cancellation, CPU/GPU fallback, and result-contract stability
  - verify with targeted Rust and frontend request-shaping tests, then run quality gates
- results or validation changes
  - preserve user-facing metrics, workbook/report path semantics, and display adequacy
  - verify with report tests, workbook checks, and visual/manual evidence where needed
- artifact and history changes
  - preserve hydration, replay, reopen, tags, and path persistence semantics
  - verify with artifact/library tests, file-shape evidence, and reopen flows
- performance-sensitive changes
  - preserve or justify perf-sensitive behavior with the relevant gate or benchmark evidence
  - use `scripts/run_repo_quality_gate.ps1` as the broad local baseline before narrower perf work
- public-repo docs or quality changes
  - preserve contributor clarity, runnable local verification, and public-facing hygiene
  - update adjacent docs in the same change when contributor flow would otherwise drift

## Checklist

- [x] Identify current user-visible behavior that must remain valid.
- [x] Identify request / response / artifact contracts that must remain compatible.
- [x] Identify path, file, or report semantics that must not drift.
- [x] Identify exact tests, harnesses, or scripts to run before handoff.
- [x] Identify whether performance-sensitive code paths need perf-gate coverage.
- [x] Identify whether GPU/CPU fallback behavior must be preserved.
- [x] Identify stale-data, report-open, and history/replay behaviors if artifacts are touched.
- [x] Refuse plans that improve one layer by silently weakening another.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- frontend-only plans must identify Vitest and browser/desktop checks
- Tauri/backend plans must identify Rust tests plus any command-wiring impacts
- report/artifact plans must identify workbook / parity / path persistence checks
- performance-sensitive plans must identify the relevant perf gates or benchmark harnesses
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- verify the definition of done, refresh triggers, and any superseded/historical pointers are explicit enough that another contributor could continue the program without guesswork

## Rollout / Fallback / Compatibility Notes

- when behavior changes are unavoidable, the plan must state the compatibility break explicitly
- when no test exists, the plan must include adding the test rather than hand-waving the risk
- the default assumption is backward-compatible improvement, not behavior reset
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
