# Artifacts, History, Reports, And Exports Implementation Plan

## Surface

Primary files:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/validationService.ts`
- `web/src/services/metaService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

## Current Strengths

- Saved run artifacts are treated as first-class workflow evidence.
- Reports and exports cover validation and workflow history rather than only raw result dumps.
- There is existing output parity tooling.
- History synchronization already connects backend artifacts with frontend state.

## Improvement Plan

### ART-01: Artifact Health And Repair Dashboard

Risk: Low/medium.

Problem:
Artifacts can become stale, partially missing linked files, or incompatible after schema changes. Users need a clear inspection and repair path.

Implementation:

- Extend the existing artifact catalog commands (`list_artifact_library` and `rebuild_artifact_catalog`) with a health-oriented response that returns:
  - total artifacts
  - schema versions
  - missing linked files
  - invalid JSON
  - replay classes
  - recoverable vs unrecoverable issues
- Add a History panel health section.
- Add safe repair actions:
  - rebuild index
  - remove orphaned index entries
  - relink known report files
  - export diagnostic summary

Files:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --test artifact_catalog_tests`
- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `npm --prefix web run test -- src/services/historyService.test.ts src/components/HistoryWorkspacePanel.test.ts`

### ART-02: Replay Class Contract

Risk: Low.

Problem:
Users and release checks need to know whether a run is exact replayable, compatible replayable, inspect-only, or broken.
Use the current replay/report evidence contract labels rather than inventing a parallel class vocabulary.

Implementation:

- Add or normalize a replay class field:
  - `exact_evidence_review`
  - `compatible_recompute`
  - `replacement_scoped_validation`
  - `review_only_saved_result`
  - `blocked`
- Include reasons:
  - missing source data fingerprint
  - missing model bundle
  - legacy artifact
  - version mismatch
  - provider data no longer available
- Surface replay class in History, reports, and exported JSON.

Files:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `npm --prefix web run test -- src/services/historyService.test.ts`

### ART-03: Atomic Writes For Durable Local State

Risk: Medium.

Problem:
Settings, manifests, artifacts, and reports can be corrupted by interrupted writes, antivirus locks, or crashes.
Current direct-write sites include `settings.rs`, `run_artifacts.rs`, `commands_reports.rs`, and `commands_simple_summary.rs`. `data_cache.rs` and parts of `polygon.rs` already use temp/rename patterns and should be treated as local precedent.

Implementation:

- Add a shared atomic write helper:
  - write temp file in same directory
  - flush file
  - sync file and parent directory where the platform supports it
  - rename into place
  - optionally keep `.bak` for critical JSON files
- Use it for settings, Polygon manifest, artifact JSON, and report metadata.
- Recover from `.bak` when primary JSON is corrupt and the backup is valid.

Files:

- `src-tauri/src/settings.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_simple_summary.rs`
- `src-tauri/src/file_io.rs` (new shared utility module) or an equivalent shared helper near current write sites

Verification:

- `cargo test -p ta_desktop_demo_app --lib settings`
- `cargo test -p ta_desktop_demo_app --lib polygon`
- `cargo test -p ta_desktop_demo_app --lib run_artifacts`

### ART-04: Output Parity As Release Evidence

Risk: Low.

Problem:
Output parity currently exists as a tool, but public release confidence is stronger when parity evidence is required or explicitly waived.

Implementation:

- Make output parity evidence part of the release candidate summary.
- Add a release switch for strict public release mode requiring parity output.
- Suggested script contract: `-RequireOutputParity` plus an explicit waiver field for internal builds.
- Persist `output_parity_status`, command, output path, manifest path, and waiver reason in release candidate and public release summaries.
- Keep a documented bypass only for non-public/internal builds.

Files:

- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_public_release_gate.ps1`
- `scripts/run_output_parity_check.mjs`
- `docs/RELEASE.md`
- `docs/PUBLIC_RELEASE.md`

Verification:

- `node scripts/run_output_parity_check.mjs`
- `powershell -File scripts/run_release_candidate_gate.ps1 -Variant cpu -SkipPackagedSmoke -SkipWorkspaceTests`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta -SkipWorkspaceTests`

### ART-05: Export Redaction And Failure Status

Risk: Low.

Problem:
Exports should never leak secrets, local keys, or over-broad local paths, and failed exports should leave a clear status.

Implementation:

- Route all export payloads through shared redaction before writing.
- Add export status metadata:
  - started
  - completed
  - failed
  - cancelled
  - path
  - redaction version
- Show last export status in UI.

Files:

- `src-tauri/src/redaction.rs`
- `src-tauri/src/commands_reports*.rs`
- `web/src/services/validationService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib redaction`
- `cargo test -p ta_desktop_demo_app --lib commands_reports`
- `npm --prefix web run test -- src/services/validationService.test.ts src/services/metaService.test.ts src/services/historyService.test.ts`

### ART-06: Retention And Disk-Growth Controls

Risk: Medium.

Problem:
Artifacts, reports, cache sidecars, and linked XLSX outputs can grow indefinitely.

Implementation:

- Add a local data inventory view showing:
  - artifact count and size
  - report count and size
  - Polygon cache size
  - CSV cache sidecar size
  - model bundle/cache size
- Add dry-run prune policies by age, count, and size.
- Do not delete source user data by default.

Files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `web/src/services/settingsService.ts`
- `web/src/components/SettingsWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib data_cache`
- `cargo test -p ta_desktop_demo_app --test artifact_catalog_tests`
- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `npm --prefix web run test -- src/services/settingsService.test.ts`

## Acceptance Criteria For This Guide

- Artifact health checks classify issues using the current replay/report evidence contract labels.
- Atomic-write helpers preserve existing JSON schemas, use same-directory temp files, flush/sync before rename, and provide recovery for corrupt primary files where backups exist.
- Output parity evidence is either present in public release summaries or explicitly waived for non-public/internal builds.
- Export redaction is shared with existing `src-tauri/src/redaction.rs` behavior.
- Retention actions never delete source user datasets by default.

## Deferred Higher-Risk Work

- Deleting user-selected source datasets automatically.
- Changing saved artifact schemas without legacy migration tests.
- Treating provider-backed data as exact replayable without local content fingerprints.
