# Section 20 implementation plan — artifact persistence, replay, and hydration backend ownership

## Goal

Make artifact persistence, replay, and hydration ownership more backend-centered and less duplicated across frontend services.

This is not a storage-format rewrite and not a general history/meta UI cleanup pass. The first wave should:

- consolidate the remaining real artifact-owner logic in the backend
- reduce repeated runs-dir scan and full-JSON parse fanout across artifact families
- move replay and hydration normalization away from ad hoc frontend parsing where the backend can own it cleanly
- tighten report/export boundaries so saved artifacts can be reused without reopening workbook semantics

## Repo-grounded current state

### What is already centralized

- `src-tauri/src/run_artifacts.rs` already owns the generic runs-dir location, run-artifact save/load/delete/tag safety, summary extraction for run/meta/stage4/stage5/policy-eval families, retention for run/meta/stage4 artifacts, dataset-meta enrichment, and config-fingerprint backfill.
- `src-tauri/src/commands_artifacts.rs` is already the thin Tauri facade over those generic artifact commands.
- `web/src/stores/workflowArtifactsStore.ts` is already the canonical in-memory handoff store for current run/validation state.
- `web/src/services/workflowArtifactsSync.ts` already suppresses one important class of redundant hydration when the latest saved artifact identity with validation is already present.
- `web/src/services/historyService.ts` already supports saved-run list/load/apply/rerun/delete/tag flows, including `MlSignal` replay branches.
- `web/src/services/metaService.ts` already supports saved meta / Stage 5 / policy-eval list, load, reveal, delete, and tag flows.
- `web/src/services/validationService.ts` already persists validation back into the run-artifact channel and already prefers artifact-backed dataset metadata for report export decisions.
- `src-tauri/src/commands_reports.rs` already routes product and headless validation workbook generation through the same builder path.

### Where ownership is still split

- `src-tauri/src/commands_meta.rs` still owns Stage 5 / policy-eval artifact pathing and direct JSON writes, and still carries its own meta-artifact scan/load helpers instead of delegating fully to `run_artifacts.rs`.
- `web/src/services/workflowArtifactsSync.ts`, `web/src/services/historyService.ts`, and `web/src/services/metaService.ts` still load raw artifact JSON and rebuild normalized replay/detail payloads in the frontend.
- artifact-family list commands still rescan and fully parse the runs directory independently, even when the frontend asks for several families back-to-back.
- `web/src/services/validationService.ts` still rebuilds and ships a full report payload through `save_report_xlsx` even when the persisted run artifact already contains the same run/validation context.
- lightweight saved-run detail/report actions still load full artifacts where a typed backend detail DTO or existing summary row would be sufficient.

## Current repo-valid problems

1. Backend artifact persistence ownership is incomplete: `run_artifacts.rs` owns generic CRUD and family summaries, but `commands_meta.rs` still owns Stage 5 / policy-eval artifact save-path policy and duplicate meta scan/load helpers.
2. Artifact listing cost is fanout-heavy: run, meta, Stage 5, and policy-eval summaries all rescan and parse the same directory separately.
3. Replay and hydration normalization is duplicated in the web layer:
   - `workflowArtifactsSync.ts` rebuilds workflow-hydration payloads from raw artifacts
   - `historyService.ts` rebuilds saved-run hydrate/detail DTOs from raw artifacts
   - `metaService.ts` rebuilds meta / Stage 5 / policy-eval replay DTOs from raw artifacts
4. Validation report export can duplicate serialization work by rebuilding and sending a full report payload even when the saved artifact already contains the required run/validation context.
5. Current code already has the right persistence center of gravity; the missing piece is typed replay/detail/export ownership, not a new storage substrate.

## First-wave implementation scope

### Phase 1 — complete backend artifact-owner consolidation

- Move the remaining meta-family save-path and retention ownership into `run_artifacts.rs`:
  - shared path suggestion helpers for Stage 5 and ML-policy-eval artifacts
  - shared prune helper for ML-policy-eval artifacts
  - shared meta-artifact scan/load helpers needed by `commands_meta.rs`
- Keep `commands_meta.rs` focused on producing domain results, not on owning file-layout policy or duplicate runs-dir walkers.
- Preserve current filenames, family kinds, and legacy alias acceptance unless a backend helper move naturally requires tiny compatibility glue.

### Phase 2 — reduce artifact-library fanout

- Add a shared backend scan path in `run_artifacts.rs` so the repo does not reread and reparsed the runs directory independently for each artifact-family list.
- Prefer one shared typed scan pass that can feed:
  - run summaries
  - meta summaries
  - Stage 5 summaries
  - ML-policy-eval summaries
- Keep the first wave file-based. Do not introduce a database, background daemon, or persistent index format.

### Phase 3 — backend-owned replay and detail DTOs

- Add backend-owned typed detail and hydration helpers for the replay paths the frontend currently reconstructs manually.
- First-wave targets:
  - saved run hydration payload for `workflowArtifactsSync.ts`
  - saved run detail/apply/rerun DTOs for `historyService.ts`
  - saved meta / Stage 5 / policy-eval replay DTOs for `metaService.ts`
- Keep raw `load_run_artifact` for compatibility/debug use, but stop treating it as the preferred UI replay API where a typed DTO is better.

### Phase 4 — frontend consumer collapse

- Repoint the primary replay/hydration consumers to backend-owned DTOs instead of ad hoc frontend parsing.
- Keep `workflowArtifactsStore.ts` as the canonical in-memory handoff sink.
- Delete frontend-only artifact-response reconstruction helpers once the backend projections exist.

### Phase 5 — report/export boundary cleanup

- Add an artifact-backed validation report export/save path that can reuse persisted run/validation context when a real artifact path is available.
- Keep the existing direct report-object path as fallback for:
  - unsaved ephemeral state
  - browser-download fallback
  - headless/manual callers that legitimately build reports without a saved artifact path
- Stop loading full raw artifacts merely to recover report paths or lightweight detail metadata when a summary row or typed detail DTO can provide the same information.
- Keep workbook semantics stable in `commands_reports_validation_xlsx.rs`.

## Explicit defers

- Do not replace JSON artifact files with SQLite, a custom database, or a binary store in this section.
- Do not move all optimization / validation / `MlSignal` artifact composition fully into backend commands in the first wave.
- Do not redesign history, meta, or saved-run UI tables unless an artifact-boundary change strictly requires a small compatibility tweak.
- Do not remove the raw `load_run_artifact` command in the first wave.
- Do not rewrite workbook presentation/layout logic in `commands_reports_validation_xlsx.rs` unless a narrow compatibility fix is required.
- Do not add a filesystem watcher, background indexer, or process-wide artifact daemon in the first wave.
- Do not use this bundle to split Stage 4 and Stage 5 filename prefixes or artifact families.
- Do not broaden the work into session/preset persistence, task throughput, or shell-state restoration.

## Concrete implementation checklist

### Backend ownership

- [ ] Move Stage 5 and ML-policy-eval artifact path/prune helper ownership into `src-tauri/src/run_artifacts.rs`.
- [ ] Replace duplicated meta-artifact scan/load helpers in `src-tauri/src/commands_meta.rs` with shared `run_artifacts.rs` helpers.
- [ ] Keep `src-tauri/src/commands_artifacts.rs` as the thin command facade, adding only the typed artifact DTO commands actually needed by the web consumers.

### Artifact library fanout

- [ ] Add a shared backend scan/index path so artifact-family summaries do not each reread the runs directory independently.
- [ ] Keep summary extraction typed by family, but sourced from one shared scan pass.
- [ ] Preserve current summary compatibility fields already consumed by the web.

### Replay and hydration DTOs

- [ ] Add a backend-owned typed saved-run detail DTO for `web/src/services/historyService.ts`.
- [ ] Add a backend-owned typed saved-run hydration payload for `web/src/services/workflowArtifactsSync.ts` and saved-run load/apply flows.
- [ ] Add backend-owned typed replay DTOs for meta / Stage 5 / policy-eval saved loads so `web/src/services/metaService.ts` no longer reconstructs them from raw JSON.
- [ ] Keep raw artifact loading available for compatibility/debug paths that genuinely need the full envelope.

### Frontend cleanup

- [ ] Repoint `web/src/services/workflowArtifactsSync.ts` to backend-owned hydration payloads.
- [ ] Repoint `web/src/services/historyService.ts` to backend-owned saved-run detail / hydrate responses.
- [ ] Repoint `web/src/services/metaService.ts` to backend-owned replay/detail responses for saved meta / Stage 5 / policy-eval artifacts.
- [ ] Remove redundant frontend artifact-response builders once the typed backend DTOs exist.
- [ ] Touch `web/src/services/mlSignalService.ts`, `web/src/services/validationService.ts`, or `web/src/services/analysisService.ts` only where the new DTOs or artifact-backed export path naturally require compatibility glue.

### Report/export boundary cleanup

- [ ] Add an artifact-backed validation report export/save path in the backend.
- [ ] Use that artifact-backed path in `web/src/services/validationService.ts` when a persisted run artifact with validation is available.
- [ ] Keep the existing direct report-payload export path as fallback for unsaved/headless/manual scenarios.
- [ ] Keep validation workbook compatibility behavior stable for legacy artifacts.
- [ ] Stop hydrating full raw artifacts just to obtain lightweight report/detail metadata where a summary or typed detail DTO is sufficient.

### Command registration / fallout

- [ ] Update Tauri command registration/export surfaces if new artifact DTO or export commands are added.
- [ ] Update web command inventory / command tests if the Tauri command surface changes.

## Verification plan

- `cargo test -p ta_desktop_demo_app run_artifacts`
- `cargo test -p ta_desktop_demo_app commands_meta`
- `cargo test -p ta_desktop_demo_app commands_reports`
- `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests` if report/export wiring changes
- `npm --prefix web run test -- --run src/stores/workflowArtifactsStore.test.ts src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts src/services/metaService.test.ts`
- `npm --prefix web run test -- --run src/services/validationService.test.ts src/services/mlSignalService.test.ts src/services/analysisService.test.ts` if those surfaces change
- `npm --prefix web run check:commands` if Tauri commands change
- `npm --prefix web run typecheck`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` if the artifact-owner cleanup changes workflow replay/hydration timing boundaries materially

## Acceptance criteria

- `run_artifacts.rs` is the clear backend owner of artifact save-path policy, family scan/load helpers, and prune rules for the artifact families touched by this section.
- frontend replay/hydration services consume backend-owned typed DTOs for the common saved-run and saved-meta load paths instead of rebuilding those DTOs manually from raw JSON.
- artifact-library refreshes no longer rely on repeated independent full-directory scans for each family in the main first-wave paths.
- validation report export can reuse persisted artifact context when available instead of always resending a full reconstructed report payload.
- lightweight detail/report actions avoid full raw-artifact hydration when a summary or typed detail DTO is sufficient.
- current artifact file formats, legacy compatibility, and report/workbook consumers remain intact.
