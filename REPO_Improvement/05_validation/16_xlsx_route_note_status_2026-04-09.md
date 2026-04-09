# XLSX Route Note Status Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation XLSX route-note and execution-status reporting so actual route behavior, fallback reasons, skipped modules, expected execution summaries, and warning text remain consistent and reviewable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- actual-versus-expected execution summaries on the dashboard and related workbook text

Out of scope:

- optimization route selection logic itself
- strategy runtime implementation details outside exported validation metadata
- options-related route reporting

## Current User-Facing Behavior

The workbook already surfaces route and status context in a detailed way. Current tests assert the presence of `Route Fallback`, `Route Note`, `Execution Notes`, `Disabled Modules`, `Cross Omitted Modules`, `Top Warning`, `Expected Route`, `Expected Cross`, `Expected Modules`, `Expected Categories`, `Expected Fallback`, `Expected Route Notes`, `Expected Disabled`, and `Expected Cross Omitted`.

That means the workbook already acts as an execution-explanation surface, not just a metrics export.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_validation.rs`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports_validation_common.rs` tests
- route-related validation tests in `src-tauri/src/commands_validation.rs`
- route-matrix and validation-cost bins used elsewhere for manual route inspection

## Strengths Already Present

- the export model already separates actual and expected execution summaries
- route-note, fallback, disabled-module, and cross-omission fields already exist explicitly in the model
- workbook tests already cover zero-survivor, route-note, skipped-module, and composite-route scenarios
- warning and skipped-reason text is already surfaced rather than discarded
- the dashboard already reserves dedicated space for execution explanation

## Weaknesses / Failure Modes

- status text is currently summarized into strings early, which can make ordering and wording drift hard to detect
- actual and expected summaries can diverge semantically even when both remain non-empty
- `disabled`, `skipped`, `omitted by cross mode`, and `not run` statuses can become inconsistent if formatting helpers fork
- composite-route explanatory notes and fallback reasons are especially vulnerable to copy drift
- workbook tests currently assert many strings, but they do not yet clearly freeze a canonical ordering policy for summary text

## Regressions To Avoid

- hiding actual fallback reasons when they exist
- collapsing expected execution summaries into vague generic copy
- changing status vocabulary in one workbook area but not the others
- silently removing top warning / skipped reason visibility for zero-survivor or degraded runs
- making route-note behavior depend on a specific strategy family only

## External Research Summary

- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because execution-status text crosses a typed command boundary and should remain serialized in a stable shape.
- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because workbook structure and sheet fidelity matter when status fields are part of the exported explanation contract.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because absent-versus-present route, fallback, and skipped-module fields must stay unambiguous in structured data before they become workbook text.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because stable ordering and deterministic serialization principles are useful when freezing human-readable execution summaries from structured fields.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the validation workspace already previews expected execution summaries and must stay aligned with the workbook export path.

## Proposed Improvement Approach

1. Freeze a canonical execution-status vocabulary for:
   - fallback
   - route notes
   - disabled modules
   - skipped modules
   - cross-omitted modules
   - top warning / top reason
2. Keep actual and expected execution summaries generated from structured fields through one shared formatter path.
3. Document summary ordering rules so workbook tests can assert more than mere string presence.
4. Add explicit scenario tests for:
   - hybrid route disabled -> kernel fallback
   - composite mixed-route expected notes
   - zero-survivor runs with skipped modules
   - disabled-module and cross-omission combinations
5. Keep the workbook additive: explain degraded routes without removing current execution detail.

## Checklist

- [ ] Document the canonical vocabulary for execution-status fields.
- [ ] Document the canonical ordering of summary text fragments.
- [ ] Document when the workbook should show `not run`, `disabled`, `skipped`, or `omitted by cross mode`.
- [ ] Document the difference between actual and expected execution summaries.
- [ ] Add or tighten tests for actual/expected parity on representative route scenarios.
- [ ] Add or tighten tests for zero-survivor and skipped-module dashboard text.
- [ ] Add or tighten tests for composite route-note explanations.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- run targeted `commands_validation` tests for route/fallback behavior
- add one focused headless workbook test for summary ordering and status vocabulary
- manually inspect one workbook from an exact GPU route and one from a degraded/fallback route

## Rollout / Fallback / Compatibility Notes

- preserve current route-note visibility while tightening the formatter contract
- keep richer execution explanation even when a route degrades or all candidates fail
- if wording changes are necessary, update tests and dashboard preview surfaces in the same change
- avoid introducing strategy-specific one-off wording where a category-level explanation would suffice

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/05_validation/17_xlsx_open_reveal_flows_2026-04-09.md`
- `REPO_Improvement/06_artifacts/16_report_route_fallback_2026-04-09.md`

