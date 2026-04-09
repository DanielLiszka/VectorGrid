# XLSX Shortlist Cap Policy Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to XLSX shortlist-cap behavior so workbook row limits, legacy-compatibility shortlist handling, and user-visible truncation cues remain explicit and consistent.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- shortlist visible-cap policy and related workbook/dashboard messaging

Out of scope:

- overall workbook styling already covered by broader XLSX docs
- CSV shortlist export
- options-related workbook behavior

## Current User-Facing Behavior

The validation workbook already distinguishes between all candidate rows and the visible shortlist exported into workbook sheets. The backend applies a visible-cap policy, carries compatibility-mode information for legacy shortlist behavior, and emits explicit workbook text when the visible shortlist is capped. The dashboard also reports `Shortlist Count` and related approval context.

This means shortlist-cap behavior is already part of the workbook’s explanatory contract, not just a performance implementation detail.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports_api.rs`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports.rs` tests
- any headless workbook-builder tests that exercise max-row behavior

## Strengths Already Present

- visible-cap behavior is already centralized in the report model builder
- workbook tests already assert shortlist-cap messaging
- compatibility-mode shortlist handling already exists for legacy artifacts
- dashboard and shortlist sheets already surface shortlist counts and related context
- cap behavior is already bounded rather than unstructured

## Weaknesses / Failure Modes

- visible-cap semantics can drift between workbook sheets and dashboard copy
- compatibility-mode shortlist rows and final-approved rows can be confused if the policy is not documented
- users may not realize the workbook is showing a capped shortlist unless the explanatory text remains prominent
- cap behavior spans both row selection and workbook messaging, so partial changes can mislead

## Regressions To Avoid

- silently changing the visible-cap default or bounds
- dropping the explanatory text that tells users a shortlist was capped
- breaking legacy compatibility mode labels and row inclusion behavior
- making dashboard shortlist counts disagree with exported shortlist sheets
- conflating capped visibility with actual approval count or total candidate count

## External Research Summary

- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because workbook row/sheet behavior and readability are first-class output concerns, not just serialization details.
- [Chart: Using a secondary axis - rust_xlsxwriter example](https://rustxlsxwriter.github.io/examples/chart_secondary_axis.html) is relevant because workbook explanation surfaces benefit from explicit, readable supporting cues when data presentation is bounded or transformed.
- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because the shortlist/report model is already a structured export contract.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because shortlist-cap consequences should remain visible to end users through observable export behavior and status text.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because frontend export initiation and workbook expectations should stay aligned to backend cap semantics.

## Proposed Improvement Approach

1. Freeze shortlist-cap behavior as an explicit workbook policy with user-visible messaging.
2. Document the distinction between:
   - total candidates
   - approved candidates
   - shortlist candidates
   - visible exported shortlist rows
3. Keep compatibility-mode and capped-shortlist semantics explicit in both sheets and dashboard copy.
4. Align row-cap behavior across headless builders, saved exports, and workbook tests.
5. Add or tighten tests around cap messaging and count parity.

## Checklist

- [ ] Document visible-cap policy and default bounds.
- [ ] Document how shortlist rows are selected when caps apply.
- [ ] Document compatibility-mode shortlist behavior for legacy artifacts.
- [ ] Document which workbook surfaces must display cap/count context.
- [ ] Add or tighten tests for capped-shortlist explanatory text.
- [ ] Add or tighten tests for dashboard/sheet shortlist count parity.
- [ ] Add or tighten tests for compatibility-mode shortlist inclusion.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- run targeted report-builder/headless workbook tests
- manually inspect one workbook with a capped shortlist and one workbook in legacy compatibility mode

## Rollout / Fallback / Compatibility Notes

- preserve current visible-cap messaging unless a coordinated workbook contract change is made
- keep compatibility-mode behavior explicit for older artifacts
- prefer additive explanation over raising row caps casually
- if cap defaults change, update workbook tests and status text together

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/05_validation/18_csv_shortlist_export_2026-04-09.md`
