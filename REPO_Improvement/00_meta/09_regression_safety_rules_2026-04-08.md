# Regression Safety Rules

Last updated: 2026-04-08

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

## External Research Summary

- [Playwright Best Practices](https://playwright.dev/docs/best-practices) recommends testing user-visible behavior and keeping tests isolated.
- [Vitest Mocking Guide](https://vitest.dev/guide/mocking.html) highlights mocking hazards such as hoisting and misuse of spies.
- [Pinia Testing Stores](https://pinia.vuejs.org/cookbook/testing.html) supports explicit store test setup instead of ad-hoc globals.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces that heavy work should remain async and command boundaries should stay explicit.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) emphasizes minimizing host/device transfers and batching work when GPU paths are performance-critical.

## Proposed Improvement Approach

Every implementation plan should include a regression section that answers:

- what exact current behaviors must not change
- which tests or scripts prove those behaviors now
- what new checks need to be added if the current coverage is insufficient
- what fallback or compatibility behavior must survive the change

## Checklist

- [ ] Identify current user-visible behavior that must remain valid.
- [ ] Identify request / response / artifact contracts that must remain compatible.
- [ ] Identify path, file, or report semantics that must not drift.
- [ ] Identify exact tests, harnesses, or scripts to run before handoff.
- [ ] Identify whether performance-sensitive code paths need perf-gate coverage.
- [ ] Identify whether GPU/CPU fallback behavior must be preserved.
- [ ] Identify stale-data, report-open, and history/replay behaviors if artifacts are touched.
- [ ] Refuse plans that improve one layer by silently weakening another.

## Verification Plan

- frontend-only plans must identify Vitest and browser/desktop checks
- Tauri/backend plans must identify Rust tests plus any command-wiring impacts
- report/artifact plans must identify workbook / parity / path persistence checks
- performance-sensitive plans must identify the relevant perf gates or benchmark harnesses

## Rollout / Fallback / Compatibility Notes

- when behavior changes are unavoidable, the plan must state the compatibility break explicitly
- when no test exists, the plan must include adding the test rather than hand-waving the risk
- the default assumption is backward-compatible improvement, not behavior reset

## Related Docs

- `REPO_Improvement/00_meta/02_pre_impl_research_gate_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
