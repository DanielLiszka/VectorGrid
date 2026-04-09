# File Picker Path Boundary Plan

Last updated: 2026-04-09

## Purpose

Define the current boundary between native file/folder pickers, browser pickers, constrained artifact paths, and generic OS open/reveal helpers so path-related improvements do not accidentally broaden authority or break existing reopen flows.

## Scope Boundaries

In scope:

- native CSV/save/folder picker commands
- market-data-root selection and persistence
- artifact path containment for load/delete/tag flows
- generic `open_path` and `reveal_path` helpers
- history/report reopen and path-handling UX

Out of scope:

- report workbook content
- dataset parsing or validation semantics beyond the path contract
- options-related file handling

## Current User-Facing Behavior

The repo currently exposes several distinct path behaviors:

- Windows-only native dialogs can pick CSV files, save CSV paths, and choose folders
- browser-side preset import uses browser-native file selection rather than the Tauri dialog surface
- changing the market-data root requires an absolute writable directory and can optionally move the Polygon cache
- run-artifact load/delete/tag operations are constrained to the app-local `runs` directory
- generic `open_path` and `reveal_path` helpers can target any existing path and are used by history/report reopen flows

These are not one unified “file picker” feature. The authority boundary is intentionally split between trusted artifact-path operations and broader helper commands used only after the app already knows a path.

## Current Code Surface

- `src-tauri/src/commands_data.rs`
  - owns `pick_csv_file`, `pick_save_csv`, `pick_folder`, market-data-dir commands, and Polygon dataset path flows
- `src-tauri/src/settings.rs`
  - owns persisted market-data-root path semantics
- `src-tauri/src/run_artifacts.rs`
  - owns containment checks for artifact load/delete/tag flows under the runs directory
- `src-tauri/src/commands_reports_shared.rs`
  - owns generic `open_path_blocking` and `reveal_path_blocking`
- `src-tauri/src/commands_reports_api.rs`
  - exposes `open_path` and `reveal_path` to the frontend
- `src-tauri/src/commands_meta.rs`
  - participates in saved-artifact/report reopen behavior
- `web/src/services/dataService.ts`
  - wraps native data pickers and market-data directory controls
- `web/src/services/historyService.ts`
  - reopens report paths, reveals saved artifact files, and normalizes history-side path behavior
- `web/src/services/mlSignalService.ts`
  - participates in artifact/report reopen behavior for ML-related flows
- `web/src/stores/workflowArtifactsStore.ts`
  - persists reopened artifact/report path references in the workflow model

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/mlSignalService.test.ts`
- `src-tauri/tests/settings_tests.rs`
- unit tests in `src-tauri/src/run_artifacts.rs`
- targeted command/report path tests in `src-tauri/tests/validation_xlsx_report_tests.rs`

## Strengths Already Present

- constrained artifact operations already enforce canonical-path containment
- market-data-root changes already reject relative and non-writable destinations
- generic open/reveal behavior is centralized rather than duplicated across services
- picker commands and path-sensitive operations are already separated at the backend layer
- browser fallback behavior exists where native dialogs are not the active mechanism

## Weaknesses / Failure Modes

- “path handling” sounds like one surface even though picker authority, artifact authority, and generic open/reveal authority are different
- `browseCsv()` intentionally swallows picker failures, which can hide environment-specific dialog issues
- generic `open_path` and `reveal_path` are broader than artifact-only operations, so tightening them later would be a compatibility change
- path behavior is split across data, reports, artifacts, and history layers, making wording and failure UX easy to drift
- Windows-only native picker behavior is easy to forget during browser-first or cross-platform verification

## Regressions To Avoid

- weakening runs-directory containment for artifact load/delete/tag commands
- broadening market-data-root acceptance beyond absolute writable directories
- silently changing generic `open_path` / `reveal_path` authority without auditing history/report reopen flows
- removing browser-native import fallbacks while claiming picker support is unchanged
- conflating “user selected a path” with “the app is authorized to mutate that path”

## External Research Summary

- [Tauri dialog plugin](https://v2.tauri.app/plugin/dialog/)
  - relevant because dialog-driven path selection should stay distinct from arbitrary filesystem authority
- [Tauri opener plugin](https://v2.tauri.app/plugin/opener/)
  - relevant because generic open/reveal behavior is a separate capability surface from picker selection
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
  - relevant because file-selection and open/reveal flows should stay scoped by explicit permissions
- [Rust `std::fs::canonicalize`](https://doc.rust-lang.org/std/fs/fn.canonicalize.html)
  - relevant because the repo’s artifact containment checks depend on canonical path resolution

## Proposed Improvement Approach

1. Document the current path model as four distinct contracts:
   - native pickers
   - browser-side import pickers
   - constrained artifact paths
   - generic open/reveal helpers
2. Make the authority split explicit in user-facing and developer-facing wording.
3. Keep market-data-root mutation rules strict and documented.
4. Preserve current artifact containment first; only broaden host-path behavior with an explicit security/compatibility review.
5. Tie reopen-path behavior back to the artifact/report models that currently supply those paths.

## Checklist

- [ ] Document which pickers are Tauri-native and Windows-only today.
- [ ] Document which file-selection flows are browser-native instead of Tauri-native.
- [ ] Document current market-data-root path requirements and optional cache-move behavior.
- [ ] Document runs-directory containment rules for artifact load/delete/tag.
- [ ] Document generic `open_path` / `reveal_path` behavior separately from contained artifact operations.
- [ ] Document current history/report reopen behavior and the path fields it relies on.
- [ ] Add or tighten tests if picker authority or path-containment behavior changes.
- [ ] Review any future path broadening against Tauri capabilities and current reopen compatibility.

## Verification Plan

- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/services/mlSignalService.test.ts`
- run `cargo test -p ta_desktop_demo_app --test settings_tests`
- run targeted unit tests in `src-tauri/src/run_artifacts.rs`

## Rollout / Fallback / Compatibility Notes

- preserve current artifact containment and legacy reopen flows first
- keep browser-native import fallback behavior intact unless a native replacement is fully verified
- if open/reveal authority is narrowed later, provide a migration path for saved report and history actions
- avoid platform-promising picker behavior beyond the current Windows-native implementation

## Related Docs

- `REPO_Improvement/02_data/11_polygon_root_settings_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/25_history_detail_actions_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/24_tauri_run_artifact_paths_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
