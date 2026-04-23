# Tauri Report Command API Plan

Last updated: 2026-04-10

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
- `web/src/utils/openPath.ts`

## Current Verification Surface

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `web/src/services/validationService.test.ts`
- `web/src/tauri/commands.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/settingsService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/dataService.test.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/utils/openPath.test.ts`

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is the primary contract reference for command invocation and payload shape.
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/) is relevant for documenting and tightening command/path authority.
- [Tauri opener plugin](https://v2.tauri.app/plugin/opener/) is relevant because the repo currently uses custom path-open helpers and needs to justify or align that choice.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document the six current report-facing commands, including inputs, outputs, error categories, and authority boundaries.
2. Make path rules explicit for app-local saves, artifact-backed exports, open/reveal behavior, and fallback in-memory builds.
3. Add tests for filename collisions, direct write failures, nonexistent-path errors, and async wrapper behavior.
4. Review whether open/reveal behavior should remain custom, be tightened further, or align more closely with Tauri capability/opener patterns.
5. Preserve the current frontend save-first flow while making fallback behavior more explicit and testable.
6. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document each report-facing Tauri command and its payload contract.
- [x] Document app-local path rules and artifact-backed export rules.
- [x] Define the error taxonomy for save/open/reveal/build failures.
- [x] Add tests for filename collision handling and direct write failures.
- [x] Add tests for nonexistent-path open/reveal failures.
- [x] Add dedicated coverage for the async API wrapper layer.
- [x] Review and document the current open/reveal authority model.
- [x] Preserve current in-memory fallback behavior unless a better replacement is ready.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- run targeted `src-tauri` lib/report-command tests
- run `npm test -- src/services/validationService.test.ts`
- run `npm test -- src/services/historyService.test.ts`
- run `npm test -- src/services/settingsService.test.ts`
- run `npm test -- src/tauri/commands.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve current save-first frontend behavior
- keep the base64 build fallback until direct-save reliability and coverage are strong enough to justify change
- if path authority is tightened, document the compatibility implications explicitly and keep report reopening viable for existing saved runs
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on targeted contract hardening and verification; the report-command surface did not need a broad rewrite.
- The current report command contract is now explicit and covered:
  - `build_report_xlsx`: builds an in-memory workbook and returns base64 over invoke as the existing fallback path
  - `build_validation_shortlist_csv`: builds CSV text from validation results
  - `save_report_xlsx`: writes validation workbooks into the app-local `reports/` directory
  - `save_report_xlsx_from_artifact`: reloads a saved run artifact from the app-local `runs/` directory and then writes the workbook into app-local `reports/`
  - `open_reports_dir`: reveals the app-local `reports/` directory
  - `open_path` / `reveal_path`: remain shared existing-path helpers used by report flows and other non-options flows
- The authority model was reviewed and intentionally not narrowed for `open_path` / `reveal_path`:
  - report save and artifact reload paths remain bounded by app-local `reports/` and `runs/`
  - `open_path` / `reveal_path` still accept generic existing paths because the same helpers are reused by non-options history, data, meta, and ML flows
  - this item closes on explicit documentation plus regression coverage rather than a late compatibility-breaking authority reduction
- Failure coverage was tightened at the backend contract layer:
  - `src-tauri/src/commands_reports_shared.rs` now explicitly covers missing-path failures, collision suffix allocation, collision-slot exhaustion, and blocked `reports/` directory creation
  - `src-tauri/src/commands_reports.rs` now exposes base-path save seams and an explicit workbook-write helper so collision handling and direct write failures are covered without synthetic shell-process tests
  - collision exhaustion now stays visible as an error instead of silently reusing an existing filename slot
- Async wrapper coverage is now dedicated instead of being inferred:
  - `src-tauri/src/commands_reports_api.rs` tests now exercise async build/save/open wrappers against the same blocking logic through mock-runtime-compatible helpers
  - the wrapper layer is now directly verified for `build_report_xlsx`, `build_validation_shortlist_csv`, `save_report_xlsx`, `save_report_xlsx_from_artifact`, `open_reports_dir`, `open_path`, and `reveal_path`
- Frontend behavior remains unchanged by design:
  - validation export still prefers direct save first
  - saved-run report reopening still prefers `open_path` and falls back to `reveal_path`
  - the in-memory base64 fallback remains in place
- No `README.md` or `REPO_MAP.md` update was required because this item changed contract tests and helper structure only; visible workflow routing did not change.
- Acceptance evidence for this closure:
  - `cargo check -p ta_desktop_demo_app --lib`
  - `cargo test -p ta_desktop_demo_app --lib commands_reports_shared::tests::`
  - `cargo test -p ta_desktop_demo_app --lib commands_reports::tests::`
  - `cargo test -p ta_desktop_demo_app --lib commands_reports_api::tests::`
  - `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
  - `npm.cmd --prefix web run test -- src/services/validationService.test.ts src/services/historyService.test.ts src/services/settingsService.test.ts src/tauri/commands.test.ts src/utils/openPath.test.ts`

## Related Docs

- `REPO_Improvement/05_validation/11_xlsx_workbook_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/13_xlsx_dashboard_quality_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/24_tauri_run_artifact_paths_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/40_xlsx_regression_tooling_2026-04-08.md`
