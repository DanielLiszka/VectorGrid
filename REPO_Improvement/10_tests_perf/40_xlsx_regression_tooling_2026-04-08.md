# XLSX Regression Tooling Plan

Last updated: 2026-04-10

## Purpose

Define the regression-tooling strategy that keeps XLSX reports structurally and semantically stable.

## Scope Boundaries

In scope:

- workbook XML regression tests
- headless build helpers
- representative payload fixtures
- preview/sample generation for manual review

Out of scope:

- workbook dashboard content strategy
- options report testing

## Current User-Facing Behavior

Users can export validation workbooks, and the repo already has multiple layers of regression tooling around them: direct XML/ZIP inspection tests, workbook-builder tests, headless JSON + XLSX generation, and an output-parity script that validates deterministic structural parity. However, the broader tooling strategy around representative fixtures, semantic assertions, and manual analyst review is still implicit rather than documented.

## Current Code Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `scripts/run_output_parity_check.mjs`
- report helper code in `src-tauri/src/commands_reports_*`

## Current Verification Surface

- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- targeted workbook-builder tests in `src-tauri/src/commands_reports_validation_xlsx.rs`
- `node scripts/run_output_parity_check.mjs`
- manual workbook inspection

## Strengths Already Present

- the repo already does XML-level workbook inspection
- the test suite already uses representative report payloads
- the report build path can already run headlessly
- current tests already cover rich cases beyond “file opens,” including legacy compatibility, zero survivors, route notes, disabled modules, and plateau output

## Weaknesses / Failure Modes

- structural checks can miss semantic drift
- workbook samples and expected outputs are not yet organized as a documented toolchain
- manual workbook review is necessary but not yet formalized
- the tooling layers exist, but their ownership is not explicit: it is easy to forget when to use direct tests versus headless parity versus manual open-and-review
- current coverage is strong for workbook XML but less explicit about when a change requires new representative fixtures or updated parity manifests

## Regressions To Avoid

- relying only on “file opens” checks
- changing sheet/column/chart semantics without updating fixtures
- losing representative sparse/degraded-case coverage
- treating parity output generation as a replacement for targeted assertions
- dropping current degraded-case coverage such as compatibility mode and zero-survivor reports
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [rust_xlsxwriter](https://rustxlsxwriter.github.io/index.html) is relevant because workbook structure and formatting are code-driven and testable.
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) reinforces chart/data integrity as part of workbook usefulness.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant for the “user-visible output” mindset: the generated workbook must be reviewable, not merely structurally valid.
- [rust_xlsxwriter: Worksheet tabs](https://rustxlsxwriter.github.io/worksheet/tabs.html) is relevant because current workbook verification includes active-tab behavior, not just content presence.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat workbook regression tooling as a layered system: direct XML assertions, workbook-builder tests, headless artifact generation, parity scripts, and manual workbook spot checks.
2. Keep at least one rich report fixture and one degraded/sparse fixture, but also document the specific current edge classes already covered: compatibility mode, zero survivors, route-note surfaces, disabled modules, and plateau output.
3. Tie workbook contract changes to fixture/test updates and manual workbook review, not just one of those.
4. Use the tooling to protect both structure and analyst usefulness.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory current XLSX regression tests and what they actually assert.
- [x] Inventory the workbook-builder tests in `commands_reports_validation_xlsx.rs` and how they differ from the integration-style test file.
- [x] Define the representative payload set the tooling must keep, including rich, compatibility, zero-survivor, and route-detail cases.
- [x] Add semantic assertions where structural checks are too weak.
- [x] Define when manual workbook review is mandatory.
- [x] Define how sample workbooks are regenerated or refreshed through `ta_headless_validate`.
- [x] Keep path/save/open behavior in scope when reports change.
- [x] Tie workbook changes to explicit fixture/test/parity updates.
- [x] Verify sparse and zero-survivor cases remain covered.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run targeted workbook-builder tests in `src-tauri/src/commands_reports_validation_xlsx.rs`
- run `node scripts/run_output_parity_check.mjs`
- build and open at least one representative workbook manually after contract-level changes
- confirm XML assertions and manual review still cover the changed surface
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the existing structural test suite while expanding it
- preserve current headless/parity tooling as supplemental verification rather than replacing targeted tests
- prefer additive assertions over replacing current checks wholesale
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on fresh XLSX regression-tooling verification rather than a workbook code change.
- Fresh acceptance evidence for this closure:
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
  - `cargo test -p ta_desktop_demo_app validation_report_ --lib`
  - `node scripts/run_output_parity_check.mjs`
- The integration-style XLSX suite passed `15/15` tests and still covers the representative payload classes the doc calls out:
  - rich workbook/dashboard content and chart presence
  - legacy compatibility mode
  - run-level rejection with zero approved rows
  - zero-survivor fallback charts
  - route-note, skipped-module, and disabled-module dashboard surfaces
  - strategy-family coverage
  - plateau-analysis output
- The workbook-builder layer also passed when scoped correctly to the library surface:
  - `validation_report_workbook_smoke`
  - `validation_report_workbook_contains_expected_sheets_and_charts`
  - `validation_report_empties_approved_sheet_when_run_verdict_rejects_final_approval`
  - `validation_report_supports_legacy_compatibility_mode`
- The targeted builder command for this item is the `--lib` form above. A broader `cargo test -p ta_desktop_demo_app validation_report_` sweep pulled unrelated bins into the build and is not the correct verification surface for this XLSX-tooling item.
- The headless parity tool also passed and regenerated representative workbook artifacts through `ta_headless_validate`:
  - `_write_test/headless_reports/AAPL_headless_current.json`
  - `_write_test/headless_reports/AAPL_headless_current.xlsx`
  - `_write_test/headless_reports/AAPL_headless_current.parity_manifest.json`
  - parity status: `Output parity check passed for 2 pair(s).`
- Manual workbook open-and-review remains mandatory after workbook contract changes, but it was not required for this closure because the current work verified existing tooling coverage rather than changing workbook semantics or layout contracts.
- No `README.md` or `REPO_MAP.md` update was required because this item verified the current regression-tooling stack rather than changing report behavior or public documentation.

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
