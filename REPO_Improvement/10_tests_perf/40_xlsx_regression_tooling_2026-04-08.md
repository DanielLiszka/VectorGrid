# XLSX Regression Tooling Plan

Last updated: 2026-04-08

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

## External Research Summary

- [rust_xlsxwriter](https://rustxlsxwriter.github.io/index.html) is relevant because workbook structure and formatting are code-driven and testable.
- [Microsoft Support: Create a chart from start to finish](https://support.microsoft.com/en-us/office/create-a-chart-from-start-to-finish-0baf399e-dd61-4e18-8a73-b3fd5d5680c2) reinforces chart/data integrity as part of workbook usefulness.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant for the “user-visible output” mindset: the generated workbook must be reviewable, not merely structurally valid.
- [rust_xlsxwriter: Worksheet tabs](https://rustxlsxwriter.github.io/worksheet/tabs.html) is relevant because current workbook verification includes active-tab behavior, not just content presence.

## Proposed Improvement Approach

1. Treat workbook regression tooling as a layered system: direct XML assertions, workbook-builder tests, headless artifact generation, parity scripts, and manual workbook spot checks.
2. Keep at least one rich report fixture and one degraded/sparse fixture, but also document the specific current edge classes already covered: compatibility mode, zero survivors, route-note surfaces, disabled modules, and plateau output.
3. Tie workbook contract changes to fixture/test updates and manual workbook review, not just one of those.
4. Use the tooling to protect both structure and analyst usefulness.

## Checklist

- [ ] Inventory current XLSX regression tests and what they actually assert.
- [ ] Inventory the workbook-builder tests in `commands_reports_validation_xlsx.rs` and how they differ from the integration-style test file.
- [ ] Define the representative payload set the tooling must keep, including rich, compatibility, zero-survivor, and route-detail cases.
- [ ] Add semantic assertions where structural checks are too weak.
- [ ] Define when manual workbook review is mandatory.
- [ ] Define how sample workbooks are regenerated or refreshed through `ta_headless_validate`.
- [ ] Keep path/save/open behavior in scope when reports change.
- [ ] Tie workbook changes to explicit fixture/test/parity updates.
- [ ] Verify sparse and zero-survivor cases remain covered.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run targeted workbook-builder tests in `src-tauri/src/commands_reports_validation_xlsx.rs`
- run `node scripts/run_output_parity_check.mjs`
- build and open at least one representative workbook manually after contract-level changes
- confirm XML assertions and manual review still cover the changed surface

## Rollout / Fallback / Compatibility Notes

- preserve the existing structural test suite while expanding it
- preserve current headless/parity tooling as supplemental verification rather than replacing targeted tests
- prefer additive assertions over replacing current checks wholesale

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_DATE.md`
