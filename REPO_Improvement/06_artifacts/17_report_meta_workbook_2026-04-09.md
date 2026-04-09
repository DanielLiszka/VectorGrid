# Report Meta Workbook Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Stage 3 meta-analysis workbook so saved meta reports remain a stable, high-signal artifact with clear sheet structure, summary semantics, and saved-report reopen behavior.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- Stage 3 workbook sheet set, summary fields, charts, and saved-report path behavior

Out of scope:

- Stage 4 / Stage 5 result semantics beyond their saved-artifact linkage
- validation workbook structure
- options-related report generation

## Current User-Facing Behavior

Meta selection already writes a Stage 3 XLSX report to the app `reports/` directory, persists the saved path into the meta artifact JSON, and exposes reopen/load/reveal behavior from Meta Analysis and validation fallback flows. The workbook already includes at least a `Meta Analysis` sheet plus additional asset/detail sheets and charts.

This means the Stage 3 workbook is already part of the product workflow, not a one-off debug export.

## Current Code Surface

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/commandActionStore.ts`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`
- any existing Rust tests in `commands_reports_meta_xlsx.rs` or `commands_meta.rs`
- manual Stage 3 export/open flows

## Strengths Already Present

- the Stage 3 workbook is already saved automatically during meta selection
- saved meta artifacts already persist `report_xlsx_path`
- the workbook already includes summary metrics, grouped rows, and charts
- Meta Analysis already supports reopen and reveal of saved reports
- validation export already preserves Stage 3 fallback rather than discarding the latest meta report path

## Weaknesses / Failure Modes

- the meta workbook contract is less explicitly documented than the validation workbook contract
- saved-report path behavior spans backend generation, artifact persistence, and frontend reopen logic
- Stage 3 workbook chart/sheet minimums are not yet frozen as clearly as validation workbook minimums
- Stage naming and compatibility language can drift between backend, services, and UI copy
- lacking a clear workbook contract makes future cleanup more likely to remove useful summary surfaces

## Regressions To Avoid

- breaking automatic save of the Stage 3 workbook during meta selection
- losing `report_xlsx_path` persistence in saved meta artifacts
- weakening reopen/reveal behavior for saved Stage 3 reports
- reducing the Stage 3 workbook to a raw dump without summary sheets and charts
- making validation fallback unable to reuse the latest Stage 3 report

## External Research Summary

- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because the Stage 3 workbook is a structured user artifact that should preserve layout fidelity and reproducible sheet structure.
- [Chart examples - rust_xlsxwriter](https://rustxlsxwriter.github.io/examples/chart_secondary_axis.html) is relevant because the Stage 3 workbook already uses charts to summarize grouped meta results.
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because Stage 3 save/open/replay behavior crosses typed backend/frontend boundaries.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because saved Stage 3 report paths are coordinated through Meta services and command-action state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because saved-report open/reveal behavior is already validated through store/service tests and should remain so.

## Proposed Improvement Approach

1. Freeze a Stage 3 meta workbook contract:
   - required sheet set
   - required summary fields
   - required chart set
   - required saved-report path persistence
2. Document saved-report path behavior from generation through reopen.
3. Expand tests around saved-report persistence and Stage 3 reopen/reveal behavior.
4. Keep Stage 3 workbook improvements additive and summary-preserving.
5. Keep compatibility aliases and fallback flows explicit where older naming still matters.

## Checklist

- [ ] Document the required Stage 3 workbook sheets.
- [ ] Document the required summary fields and chart surfaces.
- [ ] Document how `report_xlsx_path` is generated and persisted.
- [ ] Add or tighten tests for saved meta report reopen/reveal behavior.
- [ ] Add or tighten tests or manual checks for workbook content minimums.
- [ ] Document naming/compatibility expectations around Stage 3 versus later meta stages.
- [ ] Confirm validation fallback still honors the latest Stage 3 report path.

## Verification Plan

- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/metaService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/stores/analysisStore.actions.test.ts"` from `web/`
- run targeted Rust tests for `commands_meta.rs` and `commands_reports_meta_xlsx.rs` if present
- manually run one meta selection, then open and reveal the saved Stage 3 workbook

## Rollout / Fallback / Compatibility Notes

- preserve automatic Stage 3 workbook generation
- keep saved path persistence backward-compatible with current meta artifact files
- if workbook sheet names or chart titles change, update reopen expectations and docs together
- preserve Stage 3 fallback visibility in other parts of the app

## Related Docs

- `REPO_Improvement/06_artifacts/05_meta_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/19_report_save_open_reveal_2026-04-09.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`

