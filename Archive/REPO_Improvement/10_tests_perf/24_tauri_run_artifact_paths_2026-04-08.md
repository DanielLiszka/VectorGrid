# Tauri Run Artifact Paths Plan

Last updated: 2026-04-10

## Purpose

Document the current Tauri-side run artifact path rules so save/list/load/delete/tag/open flows stay compatible with current app-local storage behavior and current frontend history workflows.

## Scope Boundaries

In scope:

- app-local run artifact directory selection
- run artifact filename/path generation
- path containment checks for load/delete/tag operations
- history-facing list/detail/hydration path payloads
- runs-directory open/reveal behavior

Out of scope:

- artifact JSON schema beyond the path fields it exposes
- report workbook content
- options artifact flows

## Current User-Facing Behavior

Saved run artifacts live under the app-local `runs` directory. Users can list saved runs, load hydration/detail payloads from a saved path, reveal the saved JSON file, open the containing runs directory, delete artifacts, and update tags. The history layer also consumes each artifact's persisted `report_xlsx_path` so users can reopen the last exported workbook for a saved run.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
  - owns `runs_dir`, run-artifact filename generation, artifact-library scanning, path containment checks, load/delete/tag operations, and summary/detail/hydration payload builders
- `src-tauri/src/commands_artifacts.rs`
  - exposes the Tauri async wrappers for save/list/load/delete/tag/open-runs-dir commands
- `src-tauri/src/commands_reports_shared.rs`
  - owns filename sanitization plus the shared `open_path_blocking` and `reveal_path_blocking` helpers used by history/report reopen flows
- `src-tauri/src/commands_reports_api.rs`
  - exposes the generic `open_path` and `reveal_path` commands that the frontend uses when reopening report files from saved runs
- `src-tauri/src/lib.rs`
  - registers the artifact and path-related Tauri commands
- `web/src/services/historyService.ts`
  - lists saved runs, normalizes saved paths, loads detail/hydration payloads, reopens report paths, reveals run JSON paths, and deletes saved runs
- `web/src/stores/workflowArtifactsStore.ts`
  - stores normalized artifact/report path fields in the frontend workflow-artifact model
- `web/src/services/historyService.test.ts`
  - covers direct command use for list/detail/reveal/delete/open-report flows
- unit tests in `src-tauri/src/run_artifacts.rs`
  - cover artifact-kind aliases, hydration/detail shaping, and related path-bearing payload fields

## Current User-Facing Contract

- run artifacts are stored under `app_local_data_dir()/runs`
- `save_run_artifact` writes a JSON artifact using the current `ts` with the `vectorgrid_run_<ts>.json` naming pattern
- `list_run_artifacts` returns summary rows with `path`, `ts`, `bytes`, `strategy_kind`, `has_validation`, `report_xlsx_path`, `csv_path`, timeframe metadata, and tags
- `load_run_artifact_hydration` and `load_run_artifact_detail` require a path inside the app runs directory and reject unsupported artifact kinds
- `delete_run_artifact` and `update_artifact_tags` require a path inside the app runs directory
- `open_runs_dir` reveals the runs directory itself
- history-layer report reopen uses `open_path(report_xlsx_path)` and falls back to `reveal_path(report_xlsx_path)` if direct open fails
- legacy `vectorbt_run_artifact` kinds and `vectorbt_run_*.json` filename prefixes remain accepted by the artifact layer

## Current Verification Surface

- `web/src/services/historyService.test.ts`
- unit tests in `src-tauri/src/run_artifacts.rs`
- manual artifact save/list/reopen/delete checks

## Strengths Already Present

- path-sensitive load/delete/tag operations already enforce containment within the runs directory
- the artifact layer already centralizes summary/detail/hydration shaping instead of duplicating it across commands
- legacy run-artifact naming compatibility is already preserved in the backend
- the frontend history service already normalizes path strings before invoking path commands

## Weaknesses / Failure Modes

- `save_run_artifact` does not currently use a collision-avoiding suffix helper, so repeated saves with the same timestamp can overwrite the same run-artifact filename
- containment checks rely on canonicalization of an existing path, so nonexistent-path and out-of-dir failures currently collapse into filesystem-error behavior unless explicitly tested
- `open_path` and `reveal_path` are generic existing-path helpers, not runs-dir-only commands, so tightening their authority later would be a compatibility change
- artifact scanning reads every JSON file under the runs directory and silently skips unreadable/non-JSON files, which can mask drift when the directory accumulates extra JSON outputs
- path behavior is split between `run_artifacts.rs`, shared report-path helpers, and frontend history normalization, so wording and failure handling can diverge without a dedicated path contract doc
- the history layer depends on `report_xlsx_path` persistence even though report files themselves live under a different app-local directory

## Regressions To Avoid

- allowing load/delete/tag operations outside the runs directory
- breaking current history-service assumptions about `path` and `report_xlsx_path`
- changing canonicalization or containment semantics without targeted tests
- silently dropping legacy run-artifact kind/filename compatibility
- tightening generic `open_path` / `reveal_path` behavior in a way that breaks saved-run report reopen flows
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
- [Tauri `PathResolver` docs](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html#method.app_local_data_dir)
- [Rust `std::fs::canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html)
- [Tauri opener plugin reference](https://v2.tauri.app/plugin/opener/)
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document the current runs-directory path contract separately from report-directory behavior.
2. Make path generation, containment, and legacy alias handling explicit.
3. Document which operations are runs-dir constrained and which use the broader generic open/reveal helpers.
4. Tie history-service behavior back to the backend path rules it relies on.
5. Preserve current compatibility first, especially around legacy artifact names and report reopen behavior.
6. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current app-local runs directory contract.
- [x] Document how `save_run_artifact` currently derives filenames.
- [x] Document current collision behavior and whether same-`ts` overwrites are acceptable.
- [x] Document containment checks for load/delete/tag operations.
- [x] Document legacy kind and filename alias support.
- [x] Document summary/detail/hydration path fields consumed by the frontend.
- [x] Document how history uses `report_xlsx_path` versus the artifact JSON path itself.
- [x] Add or tighten tests for out-of-dir, nonexistent, and same-timestamp path behavior if the contract changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run targeted unit tests in `src-tauri/src/run_artifacts.rs`
- manually save a run artifact, list it, reveal it, reopen its report path, and delete it
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve existing path fields and legacy filename/kind aliases first
- if filename collision behavior changes, document the compatibility effect on existing automation and saved-path expectations
- if generic `open_path` or `reveal_path` authority is narrowed later, keep a migration path for report reopen flows
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on bounded path-contract hardening and regression coverage; the history/artifact surface did not need a workflow rewrite.
- The app-local runs contract remains the same:
  - run artifacts still live under `app_local_data_dir()/runs`
  - history-facing list/detail/hydration payloads still expose the saved run JSON `path` separately from any persisted `report_xlsx_path`
  - `open_runs_dir` still reveals the runs directory itself
- Filename behavior is now explicit and safer:
  - `save_run_artifact_blocking` now derives run JSON paths through the same timestamp-suffix allocator used by other artifact families
  - same-timestamp saves now preserve the first `vectorgrid_run_<ts>.json` and allocate `vectorgrid_run_<ts>_2.json`, `..._3.json`, and so on instead of silently overwriting
  - suffix-slot exhaustion now returns an explicit error instead of silently reusing an existing filename
- Path containment failures are now explicit instead of leaking raw canonicalize behavior:
  - missing artifact paths under `runs/` now return `artifact path does not exist: ...`
  - out-of-dir paths still return `path is outside app runs dir`
  - load/detail, delete, and tag-update flows all continue to share that containment boundary
- Legacy compatibility remains preserved:
  - `vectorbt_run_artifact` kind aliases still deserialize and summarize correctly
  - legacy filename prefixes remain accepted by the artifact scan and prune logic
- Frontend/history behavior remains unchanged by design:
  - the history service still normalizes the saved run JSON `path` before load/detail/delete/reveal operations
  - saved report reopening still uses the persisted `report_xlsx_path`, prefers `open_path`, and falls back to `reveal_path`
  - the workflow artifact store still keeps `artifactPath` and `reportXlsxPath` as distinct fields
- No `README.md` or `REPO_MAP.md` update was required because this item changed backend helper behavior and colocated tests only; user-facing routing and workspace navigation did not change.
- Acceptance evidence for this closure:
  - `cargo check -p ta_desktop_demo_app --lib`
  - `cargo test -p ta_desktop_demo_app --lib run_artifacts::tests::`
  - `npm.cmd --prefix web run test -- src/services/historyService.test.ts src/stores/workflowArtifactsStore.test.ts`

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
