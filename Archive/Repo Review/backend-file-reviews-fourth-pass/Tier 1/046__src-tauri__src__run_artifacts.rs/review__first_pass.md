# Review Packet: `src-tauri/src/run_artifacts.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/run_artifacts.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-value artifact persistence hub with strong coverage, but not yet fully hardened because overwrite semantics can silently fork artifacts, scan failures are swallowed, and read-time normalization lags save-time enrichment`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Persistence-sensitive`, `Artifact-schema-sensitive`, `History-hydration-sensitive`, `Path-safety-sensitive`

## What This File Owns

`run_artifacts.rs` owns the desktop crate's saved-artifact persistence, scanning, hydration, and replay payload surfaces.

This file owns:

- run-artifact save/load/delete/update flows:
  - `save_run_artifact_blocking`: `src-tauri/src/run_artifacts.rs:1001-1176`
  - load/detail/hydration helpers: `1739-1863`, `2435-2572`
  - delete/tag update/open runs dir helpers in the later public blocking API surface
- persistence-time enrichment and normalization:
  - config fingerprint version backfill: `1034-1052`
  - dataset-meta normalization and source identity backfill: `1055-1108`
  - route provenance and lineage backfill: `1111-1127`
  - optimization metric-surface backfill: `1131-1134`
- artifact-library scanning and "latest artifact" resolution:
  - `read_json_value`: `1273-1275`
  - `scan_artifact_library`: `1278-1310`
  - `load_latest_run_artifact_entry_blocking`: `1324-1375`
- workflow/history payload building:
  - hydration payload: `1739-1793`
  - detail payload: `1796-1863`
- stage4/stage5 artifact summary compatibility and replay helpers:
  - stage5 summary builder and list adapters around `2207`, `2811-2820`
- a large, valuable test matrix covering fingerprint stability, dataset-meta normalization, metric-surface threading, overwrite behavior, latest-artifact selection, path safety, and tag updates:
  - `2893-4870`

This file does not own:

- the Tauri command wrappers
  - those live in `src-tauri/src/commands_artifacts.rs`
- validation report generation itself
  - that lives in `src-tauri/src/commands_reports.rs`
- meta/stage execution logic
  - that lives in `src-tauri/src/commands_meta.rs` and `src-tauri/src/commands_campaign.rs`

## Classification

- Public surface: indirect but high
  - internal module, but its payloads and file contracts are consumed broadly by the desktop frontend and multiple backend commands
- Hot path: no
  - artifact persistence/history flows rather than numeric runtime loops
- Runtime-critical: high
  - saved-run history, report backfill, batch reuse, and replay workflows all depend on it
- CUDA/runtime-critical: indirect only
  - it persists metadata about route/runtime behavior rather than running compute itself
- Strategy-local: no
  - cross-workflow artifact infrastructure
- Low-risk support code: no
  - persistence and compatibility hub

## Direct Interactions

### Backend interaction map

- Tauri artifact command boundary:
  - `src-tauri/src/commands_artifacts.rs:12-199`
- campaign reuse and fingerprint comparison:
  - `src-tauri/src/commands_campaign.rs:2366`
  - save paths through `save_run_artifact_blocking`: `3065`, `3504`
- report export mutates artifacts through overwrite-save semantics:
  - `src-tauri/src/commands_reports.rs:224-240`
- workflow latest-run rehydration:
  - `web/src/services/workflowArtifactsSync.ts:70-123`
- history/detail consumers:
  - `web/src/services/historyService.ts:971-987`
  - `1178-1187`
  - `1358-1386`
- meta and options artifact-library consumers:
  - `web/src/services/metaService.ts:1260-1267`
  - `web/src/services/optionsExpressionService.ts:1368-1369`
- validation persistence expects overwrite behavior when `existing_path` is provided:
  - `web/src/services/validationService.ts:1936-1940`

## Standards And Research Log

This packet was written after targeted external review focused on file-replacement semantics, canonical path handling, JSON adaptation, and command-boundary behavior.

### Sources consulted

1. `std::fs::write`
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why it was consulted:
     - this file uses direct whole-file writes for artifact persistence and mutation
   - Standard informed:
     - direct writes are simple but do not themselves provide replacement durability

2. `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why it was consulted:
     - this file is a persistence hub and replacement semantics matter
   - Standard informed:
     - replacement strategy should be deliberate when avoiding partial or missing files matters

3. `std::fs::canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why it was consulted:
     - this file enforces run-directory path boundaries using canonical paths
   - Standard informed:
     - canonical-path checks are the right tool for path-safety validation, but callers must decide whether invalid paths are fatal or best-effort

4. Serde overview
   - Source: <https://serde.rs/>
   - Why it was consulted:
     - this file is heavily JSON-oriented and performs version-tolerant adaptation/backfill
   - Standard informed:
     - schema evolution and adaptation should be explicit when JSON blobs are long-lived product artifacts

5. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file sits behind multiple Tauri command surfaces and its errors propagate directly to the desktop client
   - Standard informed:
     - command-adjacent persistence helpers need predictable error semantics and compatibility behavior

## Findings

## Correctness And Safety

### 1. `existing_path` is treated as best-effort instead of as an overwrite contract

- Evidence:
  - when `existing_path` is provided, target-path validation errors are downgraded with `.ok()`:
    - `src-tauri/src/run_artifacts.rs:1006-1009`
  - the fallback is then a fresh timestamped path:
    - `1009`
- Why it matters:
  - callers that intend "overwrite this artifact" can silently end up creating a sibling artifact instead
- Cross-file impact:
  - validation persistence passes `existing_path` for the current run artifact:
    - `web/src/services/validationService.ts:1936-1940`
  - report export uses overwrite-save to backfill `report_xlsx_path` into the source artifact:
    - `src-tauri/src/commands_reports.rs:229-240`
- Smallest justified fix:
  - if `existing_path` is present, validate it strictly and return the error instead of silently falling back to a new artifact path

### 2. Core artifact rewrites are whole-file and non-atomic

- Evidence:
  - helper `write_json_pretty` uses `std::fs::write`: `744-746`
  - live-session mutation writes through that helper: `844-860`
  - save-time artifact persistence also uses `std::fs::write`: `1137-1138`
  - later update paths also write directly:
    - `1158`
    - `2888`
- Why it matters:
  - this file is the persistence authority for long-lived artifacts
  - direct overwrite leaves a crash window for truncation or partially rewritten files
- Smallest justified fix:
  - introduce a temp-file-plus-replace helper and use it across save/mutation/tag-update paths

## API And Naming

### 1. Stage4 and Stage5 intentionally share one summary/replay lane, but the contract is underspecified

- Evidence:
  - shared summary builder:
    - `build_stage5_artifact_summary`: `2207`
  - both list APIs use the same stage5 lane:
    - `2811-2820`
  - the frontend compensates by reading `library.stage5` into its saved-stage4 store:
    - `web/src/services/metaService.ts:1260-1267`
  - there is a dedicated alias-contract test:
    - `src-tauri/src/run_artifacts.rs:4421-4447`
- Why it matters:
  - the current behavior is intentional, but difficult to reason about without reading both backend and frontend code
- Smallest justified fix:
  - add an explicit contract comment near the shared summary/list helpers, or rename the shared bucket to a neutral compatibility term if API churn is acceptable later

## Error Handling

### 1. Library scans silently hide unreadable or malformed artifacts

- Evidence:
  - JSON read and parse failures become `None`:
    - `read_json_value`: `1273-1275`
  - scan entry creation is `Option`-based:
    - `1313-1321`
  - scan loop simply skips failed entries:
    - `1303-1307`
  - latest-run resolution does the same on newest candidates:
    - `1350-1352`
- Why it matters:
  - a corrupt artifact can quietly disappear from:
    - artifact library listings
    - latest-run resolution
    - fallback full scans
  - the user receives omission rather than a corruption signal
- Smallest justified fix:
  - preserve scan diagnostics for skipped files and surface them in logs or lightweight warning payloads

## Module Boundaries And Technical Debt

### 1. Save-time enrichment is stronger than read-time normalization

- Evidence:
  - save-time backfill adds:
    - `config_fingerprint_version`: `1034-1052`
    - normalized `dataset_meta` and source identity: `1055-1108`
    - route provenance and lineage: `1111-1127`
    - optimization metric surface: `1131-1134`
  - hydration payload mostly clones persisted fields:
    - `1761-1775`
  - detail payload mostly clones persisted fields:
    - `1800-1838`
- Why it matters:
  - older artifacts can remain partially normalized in history/detail consumers even though this module already knows how to derive richer metadata
  - this creates a compatibility gap between:
    - newly saved artifacts
    - previously saved artifacts being rehydrated today
- Smallest justified fix:
  - add a read-time normalization pass for `dataset_meta`, `config_fingerprint_version`, and `optimization_metric_surface` before building hydration/detail payloads

### 2. Path safety is strong, but the overwrite caller contract around path validation is weaker than the path-safety implementation itself

- Evidence:
  - canonical path checks are present:
    - `540-570`
  - tests cover rejecting paths outside the runs directory:
    - `4290-4362`
  - save-time overwrite path still downgrades validation failure to a fallback new path:
    - `1006-1009`
- Why it matters:
  - the module has already done the hard work of path-safety validation
  - the remaining problem is contract clarity, not missing safety machinery
- Review judgment:
  - keep the current path-safety checks
  - tighten caller-facing overwrite semantics

## Docs And Examples

### 1. The file needs a short persistence-policy note near the main save helpers

- Missing documentation topics:
  - which fields are best-effort backfilled on save
  - whether read paths intentionally normalize old artifacts or only trust persisted blobs
  - whether overwrite callers should expect strict replacement or best-effort fallback

### 2. The artifact-library omission policy should be documented if silent skipping remains intentional

- Missing documentation topics:
  - whether malformed artifacts are deliberately hidden from UI consumers
  - whether latest-run helpers are allowed to fall back to older artifacts silently

## Tests And Missing Verification

### 1. Existing coverage is strong for the core artifact surface and currently green

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app save_run_artifact_blocking_overwrites_existing_path_when_provided -- --nocapture`
    - passed `1`
  - `cargo test -p ta_desktop_demo_app load_latest_run_artifact_summary_prefers_latest_run_file_without_scanning_meta_history -- --nocapture`
    - passed `1`
  - `cargo test -p ta_desktop_demo_app build_stage5_artifact_summary_preserves_stage4_stage5_alias_contract -- --nocapture`
    - passed `1`
- Existing strengths already present in the file:
  - fingerprint stability and dataset-meta normalization:
    - `2970-3156`
  - metric-surface threading into hydration/detail:
    - `3328-3463`
    - `3838-3932`
  - same-timestamp suffixes and overwrite behavior:
    - `3994-4098`
  - latest-run fast-path and fallback behavior:
    - `4106-4285`
  - path sandboxing and tag updates:
    - `4290-4415`

### 2. Important persistence and compatibility gaps remain

- Missing tests found in this review pass:
  - invalid `existing_path` should return an error instead of silently forking persistence
  - corrupt newest artifact should surface scan/load diagnostics instead of being silently skipped
  - read-time backfill for older artifacts missing `optimization_metric_surface` or normalized `dataset_meta`
  - explicit contract test for stage4/stage5 shared-bucket payload naming if that alias remains long-term

### 3. If this file changes, verification should span save, scan, and frontend-facing payloads

- Exact local verification required if changed:
  - `cargo test -p ta_desktop_demo_app save_run_artifact_blocking_overwrites_existing_path_when_provided -- --nocapture`
  - `cargo test -p ta_desktop_demo_app load_latest_run_artifact_summary_prefers_latest_run_file_without_scanning_meta_history -- --nocapture`
  - `cargo test -p ta_desktop_demo_app build_stage5_artifact_summary_preserves_stage4_stage5_alias_contract -- --nocapture`
  - `cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture`
  - `npm --prefix ./web run test -- src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts src/services/validationService.test.ts`

## Performance, Allocation, And CUDA Concerns

### 1. Performance is secondary here; persistence correctness and compatibility dominate

- Evidence:
  - the file is mostly JSON adaptation, filesystem I/O, and list scanning
- Review judgment:
  - do not prioritize clone/allocation cleanup over write semantics or compatibility normalization

### 2. The only notable performance-sensitive behavior is library scanning fallback

- Evidence:
  - latest-run resolution first tries filename heuristics, then falls back to full scan:
    - `1324-1375`
- Measure-first guidance:
  - keep the fast-path
  - if scan diagnostics are added, ensure they do not turn ordinary library listing into a much heavier operation without evidence that the added visibility is worth it

## Code-Quality Improvement Register

- Safe now:
  - make `existing_path` strict
  - add atomic replacement helper for artifact writes
  - add read-time normalization for stale artifacts
  - preserve scan diagnostics for skipped/corrupt files
  - document the stage4/stage5 alias contract
- Defer unless artifact format policy changes:
  - larger artifact schema redesign
  - broad unification of every summary lane into a new library model

## Technical-Debt Reduction Register

- Safe to reduce now:
  - silent overwrite fallback
  - silent omission of corrupt scan entries
  - asymmetry between save-time and read-time enrichment
- Defer:
  - renaming stage4/stage5 lanes at the frontend/backend contract level
  - redesigning the artifact library to carry rich corruption diagnostics everywhere
- Useful debt that should not be "cleaned up" casually:
  - filename-based latest-run fast path
  - permissive compatibility support for older artifacts

## Performance Opportunity Register

- Safe structural improvement:
  - read-time normalization for older artifacts, because it improves compatibility without touching hot compute paths
- Benchmark-sensitive or semantics-sensitive:
  - changing artifact-library scan order or latest-run selection heuristics
  - renaming stage4/stage5 compatibility lanes without a frontend migration plan

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve backward compatibility for already-saved artifacts
  - preserve path sandboxing and overwrite support for legitimate in-runs-dir artifacts
  - preserve current latest-run selection semantics unless a product decision changes them
- Regression risk if changed carelessly:
  - report backfill can stop updating the source artifact
  - validation persistence can fork artifacts unexpectedly
  - history/latest-run surfaces can hide or misread saved artifacts
- Safe change posture:
  - prefer additive hardening and clearer persistence contracts over library redesign
