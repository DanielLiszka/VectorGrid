# Report Shortlist CSV Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation shortlist CSV export so column order, escaping, shortlist-source semantics, and parity with workbook shortlist behavior remain explicit and stable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `web/src/services/validationService.ts`
- shortlist CSV contract and frontend download behavior

Out of scope:

- workbook candidate sheets already covered elsewhere
- general report JSON export
- options-related CSV/report behavior

## Current User-Facing Behavior

The validation workspace already supports `Export shortlist CSV`. The backend builds CSV from the validation export model, preferring `shortlist_rows` and falling back to `all_rows` when shortlist rows are empty. The frontend downloads the CSV as text and reports status through the validation store.

The CSV already contains a fixed header and a reproducibility field (`params_json`), so it is already being treated as a structured downstream artifact.

## Current Code Surface

- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_api.rs`
- `web/src/services/validationService.ts`
- `src-tauri/src/commands_reports.rs` tests

## Current Verification Surface

- `src-tauri/src/commands_reports.rs` CSV tests
- `web/src/services/validationService.test.ts`
- any downstream manual shortlist CSV inspection

## Strengths Already Present

- header order is already explicit in one place
- quoting/escaping is handled centrally in a dedicated helper
- shortlist-first then all-rows fallback is already deliberate
- frontend export remains command-driven and environment-aware
- tests already assert field count and `params_json` presence

## Weaknesses / Failure Modes

- CSV schema is implicit rather than formally documented
- shortlist fallback to all rows can surprise users if not clearly explained
- manual string formatting can drift from workbook field semantics
- current tests emphasize smoke behavior more than full header/ordering compatibility
- CSV and workbook shortlist exports can diverge subtly if one path changes labels or field naming

## Regressions To Avoid

- changing CSV column order casually
- weakening escaping for commas, quotes, or newlines
- dropping `params_json` or approval/fail-reason evidence
- silently changing shortlist fallback semantics
- making CSV disagree with workbook shortlist labels and row-source policy

## External Research Summary

- [RFC 4180 - Common Format and MIME Type for CSV Files](https://www.rfc-editor.org/rfc/inline-errata/rfc4180.html) is relevant because shortlist CSV export should preserve predictable escaping and row formatting semantics.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because the CSV intentionally embeds `params_json` and must keep it faithful to the source data.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because the CSV is a tabular projection of a structured validation export model.
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because CSV export is a typed backend command and should keep a stable serialized contract.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because CSV export should remain verifiable through user-facing export behavior and downloaded content.

## Proposed Improvement Approach

1. Freeze the shortlist CSV column contract and row-source policy.
2. Document exactly when the export uses `shortlist_rows` and when it falls back to `all_rows`.
3. Keep CSV escaping and header order centralized and test-protected.
4. Align CSV field naming and meaning with workbook shortlist terminology.
5. Expand tests for escaping, header order, and shortlist fallback behavior.

## Checklist

- [ ] Document the full shortlist CSV header and field meanings.
- [ ] Document shortlist-first and all-rows fallback behavior.
- [ ] Add or tighten tests for RFC-style escaping of commas, quotes, and newlines.
- [ ] Add or tighten tests for column order compatibility.
- [ ] Add or tighten tests for `params_json`, warnings, and fail-reason field preservation.
- [ ] Confirm CSV labels stay aligned with workbook shortlist semantics.
- [ ] Manually inspect one exported CSV containing quoted text and one zero-shortlist fallback case.

## Verification Plan

- run targeted `commands_reports.rs` tests
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- add or run a focused CSV escaping and header-order test
- manually export a shortlist CSV and open it in a spreadsheet app plus a plain-text editor

## Rollout / Fallback / Compatibility Notes

- preserve the current header unless a coordinated CSV contract change is made
- keep fallback-to-all-rows behavior explicit rather than silently changing it
- avoid adding presentation-only workbook concepts to the CSV unless they are genuinely useful downstream
- if the CSV grows, document the added columns rather than reordering existing ones

## Related Docs

- `REPO_Improvement/05_validation/18_csv_shortlist_export_2026-04-09.md`
- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
- `REPO_Improvement/06_artifacts/15_report_validation_candidate_sheets_2026-04-09.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-09.md`
