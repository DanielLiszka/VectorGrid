# Tauri Report Command API Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the non-options Tauri report-command API so XLSX build/save/open/reveal flows have a clearly documented contract, path handling stays safe, and async/frontend behavior remains well tested.

## Scope Boundaries

In scope:

- Tauri report commands and wrappers
- frontend command consumption for report save/open/reveal flows
- path and opener/reveal behavior
- failure and collision handling

Out of scope:

- options report flows
- workbook content/layout quality
- replacing the current report-generation pipeline

## Current User-Facing Behavior

When users export validation reports, the frontend prefers saving the workbook to disk, then opening or revealing the path. Artifact-backed export can rebuild from saved run artifacts. If saving fails, the frontend can fall back to an in-memory workbook build.

## Current Code Surface

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/settingsService.ts`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri` lib/command tests
- `web/src/services/validationService.test.ts`
- `web/src/tauri/commands.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/settingsService.test.ts`

## Strengths Already Present

- the command surface is relatively small and focused
- artifact-backed export already exists
- app-local report save paths are already centralized
- there is meaningful backend and frontend test coverage already
- frontend fallbacks already exist when direct save fails

## Weaknesses / Failure Modes

- `open_path` and `reveal_path` can operate on any existing path rather than a narrow report-only surface
- async Tauri wrapper behavior does not appear to have dedicated integration-style coverage
- filename collisions, write failures, and nonexistent-path failures are not comprehensively tested
- `build_report_xlsx` returns base64 over invoke, which is a tolerable fallback but not ideal for larger workbooks
- cross-platform open/reveal behavior depends on custom wrapper logic that needs an explicit contract

## Regressions To Avoid

- breaking current save/open/reveal flows that users rely on
- narrowing path behavior without documenting the compatibility impact
- removing the in-memory build fallback before save-path flows are fully reliable
- weakening app-local path containment checks
- creating frontend assumptions that do not match Tauri async command behavior

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is the primary contract reference for command invocation and payload shape.
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/) is relevant for documenting and tightening command/path authority.
- [Tauri opener plugin](https://v2.tauri.app/plugin/opener/) is relevant because the repo currently uses custom path-open helpers and needs to justify or align that choice.

## Proposed Improvement Approach

1. Document the six current report-facing commands, including inputs, outputs, error categories, and authority boundaries.
2. Make path rules explicit for app-local saves, artifact-backed exports, open/reveal behavior, and fallback in-memory builds.
3. Add tests for filename collisions, direct write failures, nonexistent-path errors, and async wrapper behavior.
4. Review whether open/reveal behavior should remain custom, be tightened further, or align more closely with Tauri capability/opener patterns.
5. Preserve the current frontend save-first flow while making fallback behavior more explicit and testable.

## Checklist

- [ ] Document each report-facing Tauri command and its payload contract.
- [ ] Document app-local path rules and artifact-backed export rules.
- [ ] Define the error taxonomy for save/open/reveal/build failures.
- [ ] Add tests for filename collision handling and direct write failures.
- [ ] Add tests for nonexistent-path open/reveal failures.
- [ ] Add dedicated coverage for the async API wrapper layer.
- [ ] Review and document the current open/reveal authority model.
- [ ] Preserve current in-memory fallback behavior unless a better replacement is ready.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run targeted `src-tauri` lib/report-command tests
- run `npm test -- src/services/validationService.test.ts`
- run `npm test -- src/services/historyService.test.ts`
- run `npm test -- src/services/settingsService.test.ts`
- run `npm test -- src/tauri/commands.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current save-first frontend behavior
- keep the base64 build fallback until direct-save reliability and coverage are strong enough to justify change
- if path authority is tightened, document the compatibility implications explicitly and keep report reopening viable for existing saved runs

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/24_tauri_run_artifact_paths_DATE.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
