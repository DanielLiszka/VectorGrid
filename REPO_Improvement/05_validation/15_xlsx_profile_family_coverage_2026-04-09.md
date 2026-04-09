# XLSX Profile Family Coverage Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation XLSX profile-family coverage so every supported non-options profile family renders consistently in workbook sheets, dashboard counts, and exported candidate summaries.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- profile-kind detection, display labels, profile count aggregation, and workbook text coverage

Out of scope:

- strategy execution or validation correctness
- options-related profile families
- broad workbook styling already covered by other XLSX docs

## Current User-Facing Behavior

The validation workbook already exports a `Profile` column, profile-aware `Params Summary` text, and a dashboard chart titled `Profile counts`. The export path is strategy-agnostic enough to accept multiple advanced profile families even when the workbook is generated from minimal or compatibility-mode validation artifacts.

The current tests already prove that several profile families render without crashing and that profile text appears in workbook XML. This means the workbook is already treated as a cross-profile reporting surface, not a Double-MA-only export.

## Current Code Surface

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports.rs`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports_validation_common.rs` tests
- headless workbook builder tests via `build_report_xlsx_bytes_headless`

## Strengths Already Present

- active and auxiliary profile keys are already centralized in `commands_reports_validation_common.rs`
- `ExportCandidateRow` already carries `profile_kind`
- the workbook already surfaces profile counts and profile text in multiple places
- strategy-agnostic export already works for several advanced profile families
- there is already a dedicated test covering many profile families in one loop

## Weaknesses / Failure Modes

- the current explicit test list does not fully mirror the active-profile registry, so a supported key can drift out of coverage
- active and auxiliary profile handling is still list-driven, which makes silent omission possible when a new profile is added
- workbook display labels can diverge between candidate rows, dashboard counts, and params-summary rendering
- unknown or mixed-profile candidates can degrade into vague text instead of a stable fallback label
- compatibility-mode exports can still hide whether a profile was positively detected or merely inferred from params

## Regressions To Avoid

- dropping support for any currently exported non-options profile family
- changing existing uppercase/profile display labels without a compatibility decision
- making `Profile counts` disagree with candidate sheet `Profile` values
- allowing a new profile family to ship without workbook/export test coverage
- turning unknown profiles into hard export failures

## External Research Summary

- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because the workbook is a real user-facing document contract and should preserve stable, high-fidelity structure across profile families.
- [Chart examples - rust_xlsxwriter](https://rustxlsxwriter.github.io/examples/chart_secondary_axis.html) is relevant because the profile-family summary is already charted and needs stable category labeling to remain readable.
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because workbook export is a typed command boundary; schema drift should be handled at the Rust side, not hidden in the Vue caller.
- [What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because profile-kind and params-summary fields are part of a structured export contract that should be validated explicitly.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because profile coverage should ultimately remain observable through user-facing export flows, not only internal helper assertions.

## Proposed Improvement Approach

1. Freeze one canonical profile-family registry for workbook export.
2. Keep profile detection, display label selection, and count aggregation derived from the same source.
3. Separate:
   - active profile family detection
   - auxiliary profile decoration
   - fallback/unknown profile labeling
4. Expand tests so every non-options active profile key and every supported auxiliary profile path is exercised by the workbook builder.
5. Add explicit mixed-profile and unknown-profile coverage so export behavior remains additive instead of brittle.

## Checklist

- [ ] Enumerate every non-options active profile family currently supported by workbook export.
- [ ] Enumerate every auxiliary profile decoration currently surfaced in workbook text.
- [ ] Document the canonical display label for each supported profile family.
- [ ] Document fallback behavior for unknown or mixed-profile candidates.
- [ ] Add or tighten tests so the active-profile registry and workbook profile-family coverage cannot drift apart.
- [ ] Add or tighten tests proving dashboard `Profile counts` categories match exported candidate `Profile` values.
- [ ] Add or tighten tests for compatibility-mode artifacts and missing `run.req` strategy fallback.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests -- --nocapture`
- run targeted `commands_reports_validation_common` tests
- add or update one headless workbook test that iterates every active profile key from the canonical registry
- manually inspect one generated workbook with multiple profile families present in the same shortlist

## Rollout / Fallback / Compatibility Notes

- preserve current profile-family labels unless a coordinated export-contract update is made
- prefer additive registry extensions over ad hoc string matching in the workbook writer
- if a profile family is not recognized, export should degrade to a stable fallback label rather than fail
- keep compatibility-mode behavior explicit so legacy artifacts still export even when newer profile metadata is missing

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/12_xlsx_sheet_chart_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/05_validation/16_xlsx_route_note_status_2026-04-09.md`

