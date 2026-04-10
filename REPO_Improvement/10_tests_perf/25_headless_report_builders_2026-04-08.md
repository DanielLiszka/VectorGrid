# Headless Report Builders Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [rust_xlsxwriter getting started](https://rustxlsxwriter.github.io/worksheet/getting_stared.html)
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2)
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document the active builder entrypoints and clearly separate them from legacy/reference code in the same module.
2. Describe the current caller matrix: direct headless bytes, Tauri base64 build, Tauri save, artifact rebuild, and CLI output.
3. Make builder parity and artifact-backed rebuild behavior explicit.
4. Keep the doc grounded in the current validation-first path rather than reopening older analysis-builder design questions.
5. Use the current regression suite as the implementation-plan baseline instead of inventing a new test harness first.
6. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current authoritative validation workbook builder and all public entrypoints that call it.
- [x] Document how the headless byte builder differs from the Tauri base64 build wrapper.
- [x] Document artifact-backed rebuild behavior and its dependency on persisted dataset/report fields.
- [x] Document the current `max_rows` normalization/parity contract.
- [x] Document the default `ta_headless_validate` XLSX output path and how it differs from app-local report saves.
- [x] Document compatibility-mode behavior for legacy validation artifacts.
- [x] Document which legacy builders remain reference-only and are not on the active product path.
- [x] Add or tighten parity/regression tests if any builder entrypoint changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run targeted unit tests in `src-tauri/src/commands_reports_validation_xlsx.rs`
- run targeted unit tests in `src-tauri/src/commands_reports.rs`
- run a representative `ta_headless_validate` invocation and confirm the XLSX writes successfully
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current validation-first builder path first
- keep the current base64 Tauri build wrapper and direct-byte headless helper in parity unless there is an explicit migration plan
- if legacy/reference builders are ever removed, do it only after the active headless and artifact-backed paths are fully documented and verified
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on current-path verification rather than a code rewrite; the active validation-first builder contract already holds.
- The authoritative workbook path is explicit and unchanged:
  - `commands_reports_validation_xlsx::build_validation_report_xlsx` is the only active workbook builder
  - `build_report_xlsx_bytes_headless` returns raw bytes from that builder
  - `build_report_xlsx` returns base64 around those same bytes for the Tauri invoke boundary
  - `save_report_xlsx` and `save_report_xlsx_from_artifact` write the same builder output to disk
  - `ta_headless_validate` calls the headless byte helper and writes the resulting XLSX directly
- Headless/desktop differences remain intentional and documented:
  - headless builder callers use raw bytes or repo-local `_write_test/headless_reports/...` output
  - desktop save flows write to app-local `reports/`
  - the different output roots do not imply different workbook builders
- `max_rows` parity remains explicit:
  - Tauri command wrappers accept `Option<u32>`
  - headless entrypoints accept `Option<usize>`
  - both normalize through the same positive-cap path before calling `build_validation_report_xlsx`
- Compatibility mode and artifact-backed rebuild behavior remain preserved:
  - legacy validation artifacts without full verdict fields still export through compatibility mode
  - artifact-backed rebuilds still depend on `build_validation_report_from_artifact` and preserve persisted dataset metadata plus optimization metric-surface context before calling the active workbook builder
- Legacy/reference builders in `commands_reports.rs` remain reference-only and are not on the active product path; this item closes on explicit documentation plus parity evidence, not on removing those reference builders.
- No `README.md` or `REPO_MAP.md` update was required because this item confirmed the existing builder-routing contract and added no new user-facing path.
- Acceptance evidence for this closure:
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
  - `cargo test -p ta_desktop_demo_app --lib commands_reports_validation_xlsx::tests::`
  - `cargo test -p ta_desktop_demo_app --lib commands_reports::tests::`
  - `cargo test -p ta_desktop_demo_app --bin ta_headless_validate`
  - `cargo run -p ta_desktop_demo_app --bin ta_headless_validate -- --csv data/AAPL.csv --backend cpu --fast-period 5:6:1 --slow-period 20:22:1 --optimize-top-k 4 --validate-max-candidates 4 --xlsx-max-rows 8 --out-xlsx _write_test/headless_reports/doc25_smoke_aapl.xlsx`
  - resulting smoke artifact: `_write_test/headless_reports/doc25_smoke_aapl.xlsx`

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
