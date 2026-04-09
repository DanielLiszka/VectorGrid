# Report Artifact Path Persistence Plan

Last updated: 2026-04-08

## Purpose

Define how report paths are produced, normalized, persisted, reopened, and surfaced across artifacts and UI state.

## Scope Boundaries

In scope:

- `report_xlsx_path`
- save/open/reveal flows
- artifact persistence and hydration
- latest-report resolution in the UI

Out of scope:

- workbook content quality
- options report flows

## Current User-Facing Behavior

Users can save reports, open or reveal them from the active workflow, and later reopen persisted report paths from history, artifact-backed hydration, and meta/ML-related follow-on flows. `report_xlsx_path` is currently written into saved artifacts, exposed through history summary/detail rows, and reopened through generic `open_path` / `reveal_path` helpers. When direct open fails, the history layer already falls back to revealing the file path instead of simply dropping the action.

## Current Code Surface

- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/commandActionStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- history/command-action/workflow artifact tests
- XLSX report tests
- manual open/reveal validation

## Strengths Already Present

- there is already explicit report save/open/reveal command support
- run artifacts already persist report paths
- history and workflow artifact layers already surface report paths in multiple places
- the frontend already normalizes saved paths before invoking generic open/reveal commands
- run artifacts, meta artifacts, and ML-related flows already share the same general reopen vocabulary instead of each inventing a different field name
- the current history flow already distinguishes artifact JSON paths from report workbook paths

## Weaknesses / Failure Modes

- multiple layers can disagree about the “latest” report path
- missing-path vs intentionally-blank-path semantics can be unclear
- path normalization across saved artifacts and current workflow state is easy to drift
- generic `open_path` / `reveal_path` authority is broader than the run-artifact domain, so later tightening could break report reopen flows if the dependency is undocumented
- report files live under a different app-local directory than artifact JSON files, but the UX exposes them together
- report path producers span validation, artifact rebuild, and history hydration, which makes null/empty/stale semantics easy to misstate

## Regressions To Avoid

- losing report reopen capability after saving artifacts
- silently changing path fields or null/empty semantics
- making open/reveal behavior inconsistent across validation/history/settings
- dropping saved report path information during hydration/replay
- conflating missing report generation with stale or deleted report files
- breaking fallback-to-reveal behavior when direct OS open fails
- changing which layer is allowed to overwrite the latest report path without a documented priority rule

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because save/open/reveal flows are mediated through commands.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because persisted path fields should be treated as contract data.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant for validating what a user can actually reopen from the UI rather than only checking state mutations.
- [Tauri Opener Plugin Reference](https://v2.tauri.app/plugin/opener/) is relevant because report reopening depends on OS-integrated open behavior, not just local path bookkeeping.

## Proposed Improvement Approach

1. Document the full lifecycle of `report_xlsx_path`:
   - generated from validation/report save
   - persisted into artifact JSON
   - surfaced through history/detail/hydration models
   - reopened through generic path helpers
2. Separate the current path states clearly:
   - no report built yet
   - intentionally empty path
   - stale/deleted file path
   - valid existing workbook path
3. Define precedence rules between active workflow state, saved artifact state, and history selection so “latest report” does not depend on incidental ordering.
4. Document the difference between artifact JSON location, runs directory location, and report workbook location so users and future code do not conflate them.
5. Keep direct-open plus reveal fallback behavior explicit in the contract and in verification.

## Checklist

- [ ] Document all sources of report paths in the app.
- [ ] Document when `report_xlsx_path` must be written.
- [ ] Document when an empty path is legitimate vs a failure signal.
- [ ] Document how latest-report resolution chooses between runtime and artifact state.
- [ ] Document open/reveal fallback behavior.
- [ ] Document stale/missing path messaging.
- [ ] Document the distinction between artifact JSON path and workbook path.
- [ ] Document which flows are allowed to clear or overwrite a prior report path.
- [ ] Add or tighten history/command-action tests if path behavior changes.
- [ ] Manually verify reopen flows from validation and history.

## Verification Plan

- run history and workflow-artifact frontend tests
- run relevant XLSX/report tests
- manually save a report, save an artifact, restart, and reopen from history

## Rollout / Fallback / Compatibility Notes

- preserve current path fields and empty-string semantics unless explicitly changed
- prefer additive path metadata over changing existing path storage contracts

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/05_validation/17_xlsx_open_reveal_flows_DATE.md`
- `REPO_Improvement/04_results/16_report_path_reopen_flow_DATE.md`
