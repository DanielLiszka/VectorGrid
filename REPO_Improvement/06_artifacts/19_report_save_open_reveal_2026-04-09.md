# Report Save Open Reveal Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to report save/open/reveal behavior across validation, history, settings, command actions, and meta surfaces so path handling, fallback ordering, and user-visible status remain consistent.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/settingsService.ts`
- `web/src/stores/commandActionStore.ts`

Out of scope:

- workbook content generation
- artifact indexing semantics except where they determine saved report paths
- options-related report handling

## Current User-Facing Behavior

The repo already supports:

- saving validation workbooks to `reports/`
- opening the reports folder
- opening saved report files directly
- revealing report files or artifact files in the OS file manager
- opening latest report and latest meta report from command actions
- using `open_path` first with `reveal_path` fallback in several UI flows

These behaviors exist in multiple places because the app supports report access from active workflow state, saved history, and meta-analysis artifacts.

## Current Code Surface

- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/settingsService.ts`
- `web/src/stores/commandActionStore.ts`
- `web/src/stores/historyStore.ts`

## Current Verification Surface

- `web/src/services/validationService.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/stores/commandActionStore.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`

## Strengths Already Present

- backend path actions are already centralized in `commands_reports_shared.rs`
- OS-specific open/reveal behavior is already implemented for Windows, macOS, and Linux
- reports-folder open is already a first-class command
- frontend services already use graceful open->reveal fallback in several important places
- saved-report actions and latest-report quick actions already exist

## Weaknesses / Failure Modes

- several frontend services still implement their own nearly identical open/reveal policy
- status-line wording differs slightly by entry point
- command-action, validation, history, and meta flows are not documented as one shared report-action contract
- saved report paths can be valid in one surface and stale in another if refresh behavior is unclear
- file-open and folder-open semantics are close enough to be conflated in future cleanup work

## Regressions To Avoid

- breaking existing OS-specific open/reveal behavior
- removing saved-report access from any current surface
- changing fallback ordering from open-first to reveal-only without a deliberate decision
- making latest-report actions diverge from saved-report actions
- dropping reports-folder open from settings or validation surfaces

## External Research Summary

- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/es/develop/calling-rust/) is relevant because file/folder actions are exposed as typed commands that should remain small, clear, and failure-aware.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because report-path state is coordinated across several stores and service layers.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because these behaviors are already best protected through store/service tests that mock command calls.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because open/reveal/report-folder behavior should stay observable as user actions with stable fallback, not hidden in internal helpers.
- [Introduction - rust_xlsxwriter](https://rustxlsxwriter.github.io/worksheet/introduction.html) is relevant because the entire point of these flows is to preserve access to a generated workbook artifact.

## Proposed Improvement Approach

1. Freeze one shared report path-action policy for:
   - save
   - open file
   - reveal file
   - open folder
   - latest-report quick actions
2. Centralize duplicated frontend fallback helpers where practical.
3. Document which surfaces may open a report directly versus reveal it.
4. Keep report actions additive across validation, history, meta, settings, and command palette/quick-action flows.
5. Expand tests that assert command/fallback parity across surfaces.

## Checklist

- [ ] Document the canonical open/reveal/folder-open policy.
- [ ] Document which services and stores are allowed to initiate report actions.
- [ ] Centralize duplicated frontend path-action helpers where feasible.
- [ ] Add or tighten tests for saved-run report open/reveal parity.
- [ ] Add or tighten tests for latest-report and latest-meta-report quick actions.
- [ ] Add or tighten tests for reports-folder open from validation/settings surfaces.
- [ ] Manually verify report open/reveal behavior on a desktop build.

## Verification Plan

- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/validationService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/historyService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/services/metaService.test.ts"` from `web/`
- run `C:\Program Files\PowerShell\7\pwsh.exe -Command "npm run test -- src/stores/commandActionStore.test.ts"` from `web/`
- manually verify open/reveal on Windows for one validation workbook and one Stage 3 workbook

## Rollout / Fallback / Compatibility Notes

- preserve open-first then reveal-fallback behavior where it already exists
- keep folder-open as a separate action rather than overloading file-open helpers
- avoid silent behavior changes in quick-action surfaces
- if helpers are centralized, update all status-line expectations and tests together

## Related Docs

- `REPO_Improvement/05_validation/17_xlsx_open_reveal_flows_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/01_shell/34_frontend_tauri_bridge_2026-04-08.md`

