# CSV Shortlist Export Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation shortlist CSV export so exported columns, escaping rules, shortlist selection policy, and download/save expectations remain explicit and stable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_api.rs`
- `web/src/services/validationService.ts`
- shortlist CSV export from validation results

Out of scope:

- workbook/XLSX export
- raw artifact JSON export
- options-related CSV exports

## Current User-Facing Behavior

The validation workspace already exposes `Export shortlist CSV`. The backend builds this CSV from the validation report model, preferring shortlist rows when present and falling back to all rows otherwise. The export includes approval label/reason, hard-gate state, legacy shortlist flags, profile kind, params summary, robust/probability metrics, cross metrics, warning flags, fail reasons, and JSON params.

That means the CSV is already more than a convenience dump; it is a structured analyst-facing export of the shortlist decision surface.

## Current Code Surface

- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `web/src/services/validationService.ts`

## Current Verification Surface

- `web/src/services/validationService.test.ts`
- report-model tests in `commands_reports_validation_common.rs`
- any CSV-specific backend tests added around export text generation

## Strengths Already Present

- CSV generation is already centralized in the backend
- escaping logic already exists
- export uses the same shortlist/report model as workbook output
- the CSV includes verdict and warning context, not just metrics
- the frontend already exposes export as an explicit validation action

## Weaknesses / Failure Modes

- CSV shape can drift from shortlist semantics if the export model changes silently
- there is no dedicated doc freezing which columns are contractually important
- fallback-to-all-rows behavior can surprise users if shortlist availability is not explicit
- CSV escaping and line formatting are easy to overlook during refactors

## Regressions To Avoid

- changing exported column order or meaning without updating users/tests/docs
- dropping approval, warning, or fail-reason fields that explain why a row is present
- breaking CSV escaping for commas, quotes, or newlines
- exporting a broader row set than intended when shortlist rows are available
- making CSV export semantics diverge from workbook shortlist semantics

## External Research Summary

- [RFC 4180 - Common Format and MIME Type for CSV Files](https://www.rfc-editor.org/rfc/rfc4180) is relevant because CSV escaping, delimiters, and row structure should follow a stable exchange convention.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because one exported field is JSON params text and its serialized meaning should remain stable.
- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because the shortlist export model already behaves like a structured schema even though the output format is CSV.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because shortlist CSV export is a command-mediated backend/frontend boundary.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because export affordances and status outcomes should remain observable at the UI layer.

## Proposed Improvement Approach

1. Freeze shortlist CSV export as a structured contract, not a casual text dump.
2. Document column order, meanings, and shortlist-row selection behavior.
3. Keep CSV semantics aligned with workbook shortlist semantics wherever possible.
4. Treat escaping and fallback-to-all-rows behavior as explicit policy.
5. Add or tighten tests around shape, escaping, and shortlist selection.

## Checklist

- [ ] Document the exported shortlist CSV columns and their meaning.
- [ ] Document when shortlist rows versus all rows are exported.
- [ ] Document CSV escaping behavior and JSON-field expectations.
- [ ] Keep CSV semantics aligned with workbook shortlist semantics.
- [ ] Add or tighten tests for escaping, column order, and shortlist fallback behavior.
- [ ] Add or tighten tests for approval/warning/fail-reason field preservation.
- [ ] Verify frontend status text remains aligned with backend export behavior.

## Verification Plan

- run `npm --prefix web test -- src/services/validationService.test.ts`
- run targeted backend tests around `build_validation_shortlist_csv_text`
- manually export shortlist CSV from one standard validation run and inspect the emitted text

## Rollout / Fallback / Compatibility Notes

- preserve current CSV column meaning unless a coordinated export contract change is made
- prefer additive columns over breaking renames/reorders
- keep fallback-to-all-rows behavior explicit if shortlist rows are unavailable
- if workbook shortlist semantics change, audit CSV parity at the same time

## Related Docs

- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
- `REPO_Improvement/05_validation/04_validate_report_export_2026-04-09.md`
- `REPO_Improvement/05_validation/03_validate_results_table_2026-04-09.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
