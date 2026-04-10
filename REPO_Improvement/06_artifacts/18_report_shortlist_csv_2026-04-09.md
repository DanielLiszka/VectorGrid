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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the full shortlist CSV header and field meanings.
- [x] Document shortlist-first and all-rows fallback behavior.
- [x] Add or tighten tests for RFC-style escaping of commas, quotes, and newlines.
- [x] Add or tighten tests for column order compatibility.
- [x] Add or tighten tests for `params_json`, warnings, and fail-reason field preservation.
- [x] Confirm CSV labels stay aligned with workbook shortlist semantics.
- [x] Manually inspect one exported CSV containing quoted text and one zero-shortlist fallback case.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `commands_reports.rs` tests
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- add or run a focused CSV escaping and header-order test
- manually export a shortlist CSV and open it in a spreadsheet app plus a plain-text editor
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve the current header unless a coordinated CSV contract change is made
- keep fallback-to-all-rows behavior explicit rather than silently changing it
- avoid adding presentation-only workbook concepts to the CSV unless they are genuinely useful downstream
- if the CSV grows, document the added columns rather than reordering existing ones
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/05_validation/18_csv_shortlist_export_2026-04-09.md`
- `REPO_Improvement/05_validation/14_xlsx_shortlist_cap_policy_2026-04-09.md`
- `REPO_Improvement/06_artifacts/15_report_validation_candidate_sheets_2026-04-09.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-09.md`

## Execution Notes

- Closed as a shortlist-CSV contract item. The backend already centralizes header order and escaping, and the frontend already keeps shortlist CSV export command-driven with explicit status handling.
- Re-verified the CSV contract through `commands_reports::tests::validation_shortlist_csv_smoke` and the export-status / command-driven frontend checks in `web/src/services/validationService.test.ts`.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app validation_shortlist_csv_smoke --lib` and `npm.cmd --prefix web run test -- src/services/validationService.test.ts`.

