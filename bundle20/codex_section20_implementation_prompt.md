# Codex implementation prompt — Section 20 artifact persistence, replay, and hydration backend ownership

## Mission

Implement only the remaining repo-valid Section 20 work for artifact persistence, replay, and hydration backend ownership.

This is not a storage-engine rewrite and not a broad UI cleanup pass. Keep the first wave focused on:

1. backend artifact-owner consolidation
2. shared artifact-library scan/list plumbing
3. backend-owned replay/hydration/detail DTOs
4. artifact-backed validation report export reuse
5. narrow frontend fallout only

---

## Read first

Before editing code, read:

1. `bundle20/section20_artifact_persistence_replay_and_hydration_backend_ownership_plan.md`
2. `gpt54_feature_review_map.md` section `## 20. Artifact persistence, replay, and hydration backend ownership`
3. `REPO_MAP.md`
4. `src-tauri/src/commands_artifacts.rs`
5. `src-tauri/src/run_artifacts.rs`

Trust the current code and the repo-state corrections below over older review wording.

---

## Repo-state validation and scope corrections

### Already true in this repo

- [x] `src-tauri/src/run_artifacts.rs` already centralizes the app runs-dir root, generic run-artifact save/load/delete/tag operations, summary extraction for run/meta/stage4/stage5/policy-eval families, run/meta/stage4 retention, legacy `vectorbt_*` alias acceptance, dataset-meta enrichment, and config-fingerprint backfill.
- [x] `src-tauri/src/commands_artifacts.rs` is already the thin Tauri facade for the generic artifact CRUD/list operations.
- [x] `web/src/stores/workflowArtifactsStore.ts` already provides the canonical active artifact handoff state and no-op apply suppression for identical normalized payloads.
- [x] `web/src/services/workflowArtifactsSync.ts` already skips one important class of redundant reload when the latest artifact identity is already present with validation.
- [x] `web/src/services/historyService.ts` already owns saved-run list/load/apply/rerun/delete/tag flows, including `MlSignal` replay branches.
- [x] `web/src/services/metaService.ts` already owns saved meta / Stage 5 / policy-eval list, load, reveal, delete, and tag flows.
- [x] `web/src/services/validationService.ts` already persists validation back into the run-artifact channel and already reuses artifact-backed dataset metadata for export decisions.
- [x] `src-tauri/src/commands_reports.rs` already routes product and headless validation workbook generation through the same builder path.
- [x] `src-tauri/src/commands_reports_api.rs` already exposes the generic `open_path` / `reveal_path` surface; this section does not need a new path-opening command family.

### Still-open repo-valid work

- [x] backend artifact-owner consolidation now routes the remaining Stage 5 / ML-policy-eval artifact pathing, prune, and shared meta-artifact scan/load work through `src-tauri/src/run_artifacts.rs`, leaving `src-tauri/src/commands_meta.rs` focused on result generation
- [x] artifact-family listing now flows through the shared backend artifact-library scan path instead of repeated independent runs-dir scans
- [x] replay/hydration normalization now comes from backend-owned DTOs instead of duplicated frontend reconstruction in:
  - `web/src/services/workflowArtifactsSync.ts`
  - `web/src/services/historyService.ts`
  - `web/src/services/metaService.ts`
- [x] lightweight saved-run detail/report actions now use backend detail DTOs and summary data instead of hydrating full raw artifacts for every action
- [x] validation report export now reuses persisted artifact context when a matching validated run artifact already exists and falls back to direct payload export otherwise
- [x] `src-tauri/src/run_artifacts.rs` now owns the dedicated ML-policy-eval prune helper

### Explicit defer / exclude items

- [x] Do not replace JSON artifacts with SQLite, a database layer, or a binary store in this section.
- [x] Do not redesign history/meta UI tables or general result-table presentation in this section.
- [x] Do not move all optimization / validation / `MlSignal` artifact composition fully into backend commands in the first wave.
- [x] Do not remove the raw `load_run_artifact` command in the first wave.
- [x] Do not rewrite workbook/report semantics in `src-tauri/src/commands_reports_validation_xlsx.rs` unless a narrow compatibility fix is proven necessary.
- [x] Do not add a filesystem watcher, background artifact index daemon, or process-wide cache service in the first wave.
- [x] Do not fold session-history local storage into backend artifact persistence.
- [x] Do not use this bundle to split Stage 4 and Stage 5 filename prefixes or artifact families.
- [x] Keep `web/src/services/mlSignalService.ts` and `web/src/services/optimizeService.ts` inspect-only unless the new replay/export DTOs naturally require compatibility glue.
- [x] Keep `web/src/services/analysisService.ts` inspect-only unless a shared typed artifact detail/load path naturally improves the current raw baseline-compare load.

---

## File scope

### Primary implementation files

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/validationService.ts`
- `web/src/tauri/commands.ts`
- `web/src/tauri/commands.test.ts`

### Tests / verification surfaces that already exist

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/services/analysisService.test.ts`

### Inspect-only / conditional surfaces

- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_analysis.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/analysisService.ts`
- `web/src/services/optimizeService.ts`

### Keep unchanged unless evidence forces it

- current artifact file formats and legacy alias acceptance
- current report/workbook payload contracts
- the generic `open_path` / `reveal_path` command behavior

---

## Hard constraints

- Keep the first wave file-based and additive.
- Preserve current artifact compatibility unless a helper move forces narrow glue.
- Prefer typed backend DTOs over new frontend parsing helpers.
- Do not turn this into a general frontend state-management refactor.
- Keep CPU-only builds working.

### Ownership guard

Treat the backend as the owner of:

- artifact file-layout policy
- artifact family classification
- artifact summary extraction
- safe load/delete/tag operations
- typed replay/detail DTO construction for the common saved-run and saved-meta paths
- artifact-backed validation report export reuse when a persisted artifact already exists

Treat the frontend as the owner of:

- in-memory handoff state in `workflowArtifactsStore.ts`
- workflow/tab selection and view-model updates
- user-facing messaging and optimistic UI state

---

## Implementation checklist

### Phase 1 — backend owner consolidation

- [x] Move Stage 5 artifact path helper ownership into `src-tauri/src/run_artifacts.rs`.
- [x] Move ML-policy-eval artifact path helper ownership into `src-tauri/src/run_artifacts.rs`.
- [x] Add the missing ML-policy-eval prune helper in `src-tauri/src/run_artifacts.rs`.
- [x] Replace `src-tauri/src/commands_meta.rs` duplicate meta-artifact scan/load helpers with shared `run_artifacts.rs` helpers.
- [x] Keep `src-tauri/src/commands_meta.rs` focused on result generation, not on duplicating runs-dir policy.

### Phase 2 — artifact-library fanout reduction

- [x] Add a shared backend scan/index path that can feed run/meta/Stage 5/policy-eval summaries from one pass.
- [x] Rewire family summary commands to consume that shared scan path instead of independent full-directory walks.
- [x] Preserve current summary fields already consumed by the web.

### Phase 3 — typed replay / detail DTOs

- [x] Add a backend-owned saved-run detail DTO for `web/src/services/historyService.ts`.
- [x] Add a backend-owned saved-run hydration DTO for `web/src/services/workflowArtifactsSync.ts` and saved-run load/apply flows.
- [x] Add backend-owned typed replay/detail DTOs for saved meta / Stage 5 / policy-eval loads used by `web/src/services/metaService.ts`.
- [x] Keep the raw `load_run_artifact` command available for compatibility/debug use.

### Phase 4 — frontend consumer cleanup

- [x] Repoint `web/src/services/workflowArtifactsSync.ts` to backend-owned hydration payloads.
- [x] Repoint `web/src/services/historyService.ts` to backend-owned saved-run detail / hydrate responses.
- [x] Repoint `web/src/services/metaService.ts` to backend-owned replay/detail responses.
- [x] Remove redundant frontend artifact reconstruction helpers once those DTOs exist.
- [x] Update `web/src/stores/workflowArtifactsStore.ts` only as needed for typed DTO compatibility, without reopening the no-op apply logic.

### Phase 5 — report/export boundary cleanup

- [x] Add an artifact-backed validation report export/save path in the backend.
- [x] Use that artifact-backed path in `web/src/services/validationService.ts` when a persisted run artifact with validation is available.
- [x] Keep the existing direct report-payload export path as a fallback for unsaved/headless/manual scenarios.
- [x] Stop hydrating full raw artifacts for lightweight detail/report-path actions where a summary/detail DTO is sufficient.
- [x] Touch `src-tauri/src/commands_reports_validation_xlsx.rs` only for the narrow test-alignment fallout required by the already-landed report wording; workbook semantics remain unchanged.

### Phase 6 — command fallout

- [x] Update Tauri command registration/export surfaces if new artifact DTO or export commands are added.
- [x] Update web command inventory/tests if the command surface changes.

## Verification checklist

- [x] `cargo test -p ta_desktop_demo_app run_artifacts`
- [x] `cargo test -p ta_desktop_demo_app commands_meta`
- [x] `cargo test -p ta_desktop_demo_app commands_reports`
- [x] `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- [x] `npm --prefix web run test -- --run src/stores/workflowArtifactsStore.test.ts src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts src/services/metaService.test.ts`
- [x] `npm --prefix web run test -- --run src/services/validationService.test.ts src/services/mlSignalService.test.ts src/services/analysisService.test.ts`
- [x] `npm --prefix web run check:commands`
- [x] `npm --prefix web run typecheck`
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

---

## Acceptance criteria

- [x] `run_artifacts.rs` is the clear backend owner of the artifact families touched by this section, including the remaining Stage 5 / policy-eval helper responsibilities.
- [x] main replay/hydration consumers use backend-owned typed DTOs instead of rebuilding them from raw artifact JSON in multiple services.
- [x] artifact-library refresh no longer relies on repeated independent runs-dir scans for the main first-wave list paths.
- [x] validation report export can reuse persisted artifact context when available instead of always resending a full reconstructed report payload.
- [x] lightweight detail/report actions avoid full raw-artifact hydration where a summary or typed detail DTO is sufficient.
- [x] current artifact compatibility and report/workbook consumers remain intact.
