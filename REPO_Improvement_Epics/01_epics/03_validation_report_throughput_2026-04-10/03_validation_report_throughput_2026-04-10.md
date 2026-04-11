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
- broadening this epic into speculative route work unless a measured current supported route is the dominant bottleneck

## Current User-Facing Behavior

Validation is already rich in scope and report detail, but the current throughput story is likely a mix of:

- validation compute time
- requested-route versus resolved-route behavior on some validation paths
- workbook/export build time
- artifact/report persistence time
- optional export branch behavior such as save-from-artifact, save-from-report, build fallback, and open/reveal side effects

Those are not yet cleanly separated in the user-facing workflow, which makes the product harder to optimize and harder to explain when one validation run feels disproportionately slow.

## Current Code Surface

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/validationService.ts`

## Current Verification Surface

- `crates/app_core/src/double_ma_validate.rs`
  - validation backend/fallback and fixed-candidate tests
- `crates/app_core/src/composite_validate.rs`
  - current composite validation behavior if the epic touches standalone composite paths
- `src-tauri/src/commands_reports_validation_xlsx.rs`
  - workbook builder tests
- `cargo test -p ta_desktop_demo_app validation_report_ --lib`
- `cargo test -p ta_app_core`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `node scripts/run_output_parity_check.mjs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

## Validation And Report Truth Contract

- verdict semantics and current hard-gate behavior remain unchanged unless a narrower follow-up doc explicitly widens them
- route/fallback truth, compatibility wording, and omitted-module notes remain explicit when throughput work changes a current path
- workbook/report fidelity means preserving the current meaning of `Approved Candidates`, `Audit Data`, `Regime Audit`, dashboard links/summaries, and the existing approved-sheet suppression behavior under a rejecting run verdict
- artifact persistence remains a truth surface, not just a latency surface; validation req/res, `report_xlsx_path`, dataset metadata, route provenance, lineage, `config_fingerprint`, and optimization metric surface stay behaviorally intact unless explicitly widened

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

1. Establish representative validation scenarios and freeze their scenario contract before optimizing:
   - backend family touched
   - cross mode and finalist-cap behavior
   - omitted-module behavior on preserved-finalists or lightweight reruns
   - auto-export on/off
   - compatibility replay cases
2. Time the path as explicit branches rather than one blob:
   - validation command runtime
   - command-side cross/aux compute
   - save-from-artifact export
   - save-from-report export
   - `build_report_xlsx` fallback
   - open/reveal side effects
   - artifact/report persistence
3. Make the timing boundaries explicit in code and in acceptance evidence.
4. Prioritize the slowest dominant phase first instead of treating “validation” as one blob.
5. Keep GPU-validation maturity tied to measurable supported-route wins rather than broad speculative CUDA work.
6. Keep success criteria explicit:
   - lower end-to-end time on the representative scenarios
   - no report-parity regressions
   - no verdict or workbook-contract regressions

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the representative validation scenarios and report/export variants to measure, including route family, cross mode, finalist-cap behavior, omitted-module behavior, and auto-export posture.
- [ ] Record pre-change timings for command runtime, command-side cross/aux compute, report build/export branch, open/reveal, and save/persist phases separately.
- [ ] Identify the dominant throughput bottleneck on the selected scenarios before changing implementation.
- [ ] Keep GPU fallback reasons and backend-used reporting explicit if validation route behavior changes.
- [ ] Preserve workbook sheet, chart, route/fallback summary, and compatibility behavior unless a narrower follow-up doc widens scope.
- [ ] Preserve artifact/report truth for validation req/res, `report_xlsx_path`, dataset metadata, route provenance, lineage, `config_fingerprint`, and optimization metric surface.
- [ ] Add or strengthen tests around the chosen throughput bottleneck area.
- [ ] Reuse the existing parity, workbook builder, and route/fallback checks as non-negotiable regression gates.
- [ ] Define rollback conditions if a throughput optimization materially harms clarity, maintainability, or parity evidence.
- [ ] Record any required cross-doc sync in first-tier validation/report/perf docs if timing boundaries or operator expectations change.

## Verification Plan

- rerun the representative validation scenarios and compare pre/post timings for each phase
- rerun `cargo test -p ta_desktop_demo_app validation_report_ --lib`
- rerun targeted validation crate tests near any touched backend code, plus `cargo test -p ta_app_core` when core validation behavior changes
- rerun `src-tauri/src/bin/ta_headless_validate.rs` or `src-tauri/src/bin/ta_double_ma_validation_cost.rs` helpers when they are the clearest current evidence lane for the touched path
- rerun `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when end-to-end timing boundaries change
- rerun `node scripts/run_output_parity_check.mjs` when artifact/report output behavior changes
- rerun `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` when the touched path is performance-sensitive on CPU, and the matching GPU perf lane only when a supported GPU validation route is materially affected
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck` where frontend or desktop code is touched
- capture concrete acceptance evidence for closure, including the recorded phase timings and any parity artifacts needed to prove no output regression

## Rollout / Fallback / Compatibility Notes

- add timing separation first if current profiling is too coarse to identify the bottleneck honestly
- keep workbook/report parity as a hard stop, not an optional follow-up
- keep compute-only evidence and user-visible end-to-end evidence separate in the closure record
- prefer route-specific validation improvements over broad CUDA reopenings when only a subset of supported scenarios benefits
- do not widen this epic into artifact-library scale or general startup work except where report persistence directly dominates the measured path

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-09.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-09.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
