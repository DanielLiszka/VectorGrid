# Validation And Report Throughput Plan

Last updated: 2026-04-10

## Purpose

Define a measured epic to improve validation throughput and report/export generation time without weakening correctness, verdict semantics, workbook fidelity, or compatibility with retained artifact/report surfaces.

## Scope Boundaries

In scope:

- validation compute latency
- report/export generation latency
- validation artifact/report save latency where it materially affects the end-to-end flow
- separating timing boundaries between compute, workbook generation, and persistence
- GPU-validation maturity only where it directly changes validation throughput on supported current routes

Out of scope:

- changing validation verdict policy or hard-gate semantics
- changing report schema or workbook meaning without a narrower follow-up plan
- options-related validation/report flows
- general artifact/history scalability except where report generation directly depends on it

## Current User-Facing Behavior

Validation is already rich in scope and report detail, but the current throughput story is likely a mix of:

- validation compute time
- GPU fallback behavior on some requested routes
- workbook/export build time
- artifact/report persistence time

Those are not yet cleanly separated in the user-facing workflow, which makes the product harder to optimize and harder to explain when one validation run feels disproportionately slow.

## Current Code Surface

- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/validationService.ts`

## Current Verification Surface

- `crates/app_core/src/double_ma_validate.rs`
  - validation backend/fallback and fixed-candidate tests
- `src-tauri/src/commands_reports_validation_xlsx.rs`
  - workbook builder tests
- `cargo test -p ta_desktop_demo_app validation_report_ --lib`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `node scripts/run_output_parity_check.mjs`

## Strengths Already Present

- validation and workbook/report surfaces already have centralized owners
- workbook generation already has direct tests
- the repo already distinguishes optimization, validation, and report artifact surfaces

## Weaknesses / Failure Modes

- “validation is slow” can currently hide different bottlenecks that should be measured separately
- CPU fallback on some GPU-intended validation paths can distort user expectations and route-policy clarity
- workbook generation can remain expensive even after compute improves
- report/export work can accidentally dominate the end-to-end experience while being treated as secondary

## Regressions To Avoid

- improving throughput by weakening verdict correctness or workbook/report fidelity
- changing validation semantics while claiming a pure performance improvement
- hiding fallback-to-CPU behavior instead of measuring and explaining it
- optimizing only synthetic cases while ignoring the representative robustness/export flows users actually run
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [rust_xlsxwriter - Rust](https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/)
  - relevant because workbook generation is a first-class throughput surface in this repo, not a peripheral convenience path
- [The Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/)
  - relevant because this epic should separate compute and export timing with defensible measurement boundaries
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/develop/calling-rust/)
  - relevant because report/export throughput is observed through Tauri command boundaries, not just pure Rust internals

## Proposed Improvement Approach

1. Establish representative validation scenarios and time them as separate phases:
   - validation compute
   - workbook/report build
   - artifact/report save
2. Make the timing boundaries explicit in code and in acceptance evidence.
3. Prioritize the slowest dominant phase first instead of treating “validation” as one blob.
4. Keep GPU-validation maturity tied to measurable supported-route wins rather than broad speculative CUDA work.
5. Keep success criteria explicit:
   - lower end-to-end time on the representative scenarios
   - no report-parity regressions
   - no verdict or workbook-contract regressions

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the representative validation scenarios and report/export variants to measure.
- [ ] Record pre-change timings for compute, report build, and save/persist phases separately.
- [ ] Identify the dominant throughput bottleneck on the selected scenarios before changing implementation.
- [ ] Keep GPU fallback reasons and backend-used reporting explicit if validation route behavior changes.
- [ ] Preserve workbook sheet, chart, and compatibility behavior unless a narrower follow-up doc widens scope.
- [ ] Add or strengthen tests around the chosen throughput bottleneck area.
- [ ] Reuse the existing parity and workbook builder checks as non-negotiable regression gates.
- [ ] Define rollback conditions if a throughput optimization materially harms clarity, maintainability, or parity evidence.
- [ ] Record any required cross-doc sync in first-tier validation/report/perf docs if timing boundaries or operator expectations change.

## Verification Plan

- rerun the representative validation scenarios and compare pre/post timings for each phase
- rerun `cargo test -p ta_desktop_demo_app validation_report_ --lib`
- rerun targeted validation crate tests near any touched backend code
- rerun `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when end-to-end timing boundaries change
- rerun `node scripts/run_output_parity_check.mjs` when artifact/report output behavior changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck` where frontend or desktop code is touched
- capture concrete acceptance evidence for closure, including the recorded phase timings and any parity artifacts needed to prove no output regression

## Rollout / Fallback / Compatibility Notes

- add timing separation first if current profiling is too coarse to identify the bottleneck honestly
- keep workbook/report parity as a hard stop, not an optional follow-up
- prefer route-specific validation improvements over broad CUDA reopenings when only a subset of supported scenarios benefits
- do not widen this epic into artifact-library scale or general startup work except where report persistence directly dominates the measured path

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-09.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-09.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
