# Headless Report Builders Plan

Last updated: 2026-04-08

## Purpose

Document the current headless validation-report builder path so tests, artifact rebuilds, and CLI report generation all stay aligned with the same workbook builder that the desktop app currently uses.

## Scope Boundaries

In scope:

- direct in-process XLSX byte builders
- validation-report model normalization for headless output
- artifact-backed report rebuilds
- the `ta_headless_validate` CLI's report-write path
- parity between headless and Tauri command wrappers

Out of scope:

- workbook visual-quality decisions already covered elsewhere
- options report tooling
- replacing the current validation-first workbook path

## Current User-Facing Behavior

The active report path is validation-first. Desktop save flows, artifact-backed rebuilds, XML regression tests, and the `ta_headless_validate` binary all converge on the same validation workbook builder. The Tauri `build_report_xlsx` command returns a base64 wrapper around the same workbook bytes, while the direct headless helper returns raw bytes and is used by tests and headless tooling. The headless CLI writes to `_write_test/headless_reports/...` by default unless the caller overrides the output path.

## Current Code Surface

- `src-tauri/src/commands_reports.rs`
  - exposes `build_report_xlsx_bytes_headless`, `build_report_xlsx_blocking`, `save_report_xlsx_blocking`, and `save_report_xlsx_from_artifact_blocking`; also retains older analysis builders as legacy/reference code only
- `src-tauri/src/commands_reports_validation_common.rs`
  - builds the normalized validation export model, including compatibility mode, visible row cap, run verdict handling, expected execution notes, and candidate row shaping
- `src-tauri/src/commands_reports_validation_xlsx.rs`
  - assembles the actual workbook sheets, charts, notes, and `save_to_buffer` output for the validation workbook
- `src-tauri/src/commands_reports_api.rs`
  - exposes the Tauri command wrappers that turn the same builder into invoke-facing save/build commands
- `src-tauri/src/bin/ta_headless_validate.rs`
  - runs headless optimize -> validate -> report generation and writes XLSX output to a repo-local default path when requested
- `src-tauri/src/commands.rs`
  - re-exports `build_report_xlsx_bytes_headless` for tests and binaries
- `src-tauri/tests/validation_xlsx_report_tests.rs`
  - uses the headless builder as the main regression entrypoint for workbook XML/content checks
- tests in `src-tauri/src/commands_reports.rs`
  - verify parity between the headless byte builder and the base64-wrapped Tauri build path, plus artifact-backed report shaping

## Current User-Facing Contract

- the active workbook builder is `build_validation_report_xlsx` in `commands_reports_validation_xlsx.rs`
- `build_report_xlsx_bytes_headless(report, max_rows)` returns raw XLSX bytes directly from that active builder
- `build_report_xlsx(report, max_rows)` returns base64-encoded bytes from the same active builder
- `save_report_xlsx` and `save_report_xlsx_from_artifact` write the same builder output to disk
- `ta_headless_validate` uses the headless byte builder and defaults output to `_write_test/headless_reports/<csv_stem>_report_<ts>.xlsx`
- `max_rows` is normalized to a positive visible-cap path before workbook generation
- validation artifacts without full verdict fields still export through compatibility mode rather than failing outright
- older analysis workbook builders remain in the file only as legacy/reference code and are not part of the active validation export path

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- unit tests in `src-tauri/src/commands_reports_validation_xlsx.rs`
- unit tests in `src-tauri/src/commands_reports.rs`
- manual headless workbook generation through `ta_headless_validate`

## Strengths Already Present

- the repo already has one active workbook builder reused by desktop save flows, headless tests, and the CLI
- headless and Tauri build paths already have an explicit parity test
- artifact-backed report reconstruction already preserves persisted dataset metadata and optimization metric surface data
- XML-level regression coverage already exercises rich, sparse, compatibility, and zero-survivor report shapes

## Weaknesses / Failure Modes

- `commands_reports.rs` still contains large legacy/reference builders next to the active validation-first builder, so it is easy for readers to mistake non-authoritative code for the current path
- the headless helper returns raw bytes while the Tauri invoke path returns base64, so wrapper parity must stay documented and tested
- headless CLI output goes to a repo-local `_write_test/headless_reports` path, while desktop saves go to `app_local_data_dir/reports`; those are intentionally different and easy to conflate
- artifact-backed workbook reconstruction depends on `build_validation_report_from_artifact`, so changes to artifact field handling can drift from direct in-memory report builds
- the regression suite is strong on structure and visible text, but semantic drift is still possible if the model layer changes without targeted assertions
- caller types differ on `max_rows` shape (`u32` at the Tauri boundary, `usize` internally), so parity can drift if normalization changes in only one entrypoint

## Regressions To Avoid

- breaking deterministic parity between direct-byte and base64-wrapped builder entrypoints
- changing shared builder behavior without updating headless CLI, tests, and Tauri wrappers together
- weakening compatibility-mode export of legacy validation artifacts
- conflating repo-local headless output paths with app-local desktop report-save paths
- treating structural parity tooling as a performance benchmark surface

## External Research Summary

- [rust_xlsxwriter getting started](https://rustxlsxwriter.github.io/worksheet/getting_stared.html)
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2)

## Proposed Improvement Approach

1. Document the active builder entrypoints and clearly separate them from legacy/reference code in the same module.
2. Describe the current caller matrix: direct headless bytes, Tauri base64 build, Tauri save, artifact rebuild, and CLI output.
3. Make builder parity and artifact-backed rebuild behavior explicit.
4. Keep the doc grounded in the current validation-first path rather than reopening older analysis-builder design questions.
5. Use the current regression suite as the implementation-plan baseline instead of inventing a new test harness first.

## Checklist

- [ ] Document the current authoritative validation workbook builder and all public entrypoints that call it.
- [ ] Document how the headless byte builder differs from the Tauri base64 build wrapper.
- [ ] Document artifact-backed rebuild behavior and its dependency on persisted dataset/report fields.
- [ ] Document the current `max_rows` normalization/parity contract.
- [ ] Document the default `ta_headless_validate` XLSX output path and how it differs from app-local report saves.
- [ ] Document compatibility-mode behavior for legacy validation artifacts.
- [ ] Document which legacy builders remain reference-only and are not on the active product path.
- [ ] Add or tighten parity/regression tests if any builder entrypoint changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run targeted unit tests in `src-tauri/src/commands_reports_validation_xlsx.rs`
- run targeted unit tests in `src-tauri/src/commands_reports.rs`
- run a representative `ta_headless_validate` invocation and confirm the XLSX writes successfully

## Rollout / Fallback / Compatibility Notes

- preserve the current validation-first builder path first
- keep the current base64 Tauri build wrapper and direct-byte headless helper in parity unless there is an explicit migration plan
- if legacy/reference builders are ever removed, do it only after the active headless and artifact-backed paths are fully documented and verified

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
