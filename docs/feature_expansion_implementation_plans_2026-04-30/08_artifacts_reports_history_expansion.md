# Artifacts, Reports, History, And Export Expansion Plan

Date: 2026-04-30

## Goal

Expand artifact/report/history behavior as a durable product contract. The priority is additive schema evolution, health/repair visibility, output parity, atomic writes, and richer exports that do not mutate source artifacts unexpectedly.

## Current Owner Files

Artifacts and history:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/utils/core/runArtifactReview.ts`
- `web/src/utils/core/artifactIdentity.ts`

Reports and exports:

- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `scripts/run_output_parity_check.mjs`
- `scripts/check_output_parity.mjs`
- `scripts/check_output_semantic_parity.mjs`

## Research Basis

- SQLite documents atomic commit and crash recovery behavior; app-local stores should use transactional or atomic write patterns where possible: <https://www.sqlite.org/atomiccommit.html>
- Serde supports additive schema evolution with defaults and skip behavior: <https://serde.rs/attributes.html>
- GitHub Actions artifact retention is configurable, reinforcing that evidence retention should be explicit rather than accidental: <https://docs.github.com/organizations/managing-organization-settings/configuring-the-retention-period-for-github-actions-artifacts-and-logs-in-your-organization>
- `rust_xlsxwriter` is the current XLSX writer crate and supports workbook/worksheet generation: <https://docs.rs/rust_xlsxwriter>
- Apache Arrow IPC and Perspective are useful future analytics formats, but should follow profiling rather than replace JSON by default: <https://arrow.apache.org/docs/format/Columnar.html> and <https://perspective.finos.org/guide/how_to/javascript/loading_data.html>

## Expansion Track A: Artifact Health Dashboard

### Problem

Users need to know whether saved artifacts and linked reports are available, missing, repairable, or blocked.

### Implementation

Current owners:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

Add derived health fields:

- artifact file exists;
- recognized artifact family;
- schema version;
- source data state;
- linked report state;
- replay class;
- repair availability;
- warning count;
- blocked reason.

Do not scan cache/report trees during routine refresh. Use fields already returned by list/detail/hydration commands unless the user asks for a health scan.

### Tests

- missing report path displays `missing_on_disk`;
- unknown report state remains unknown;
- artifact list does not hash every report on refresh;
- repair dry-run/write-target behavior remains path-scoped.

## Expansion Track B: Atomic Writes And Recovery

### Problem

Artifacts, settings, manifests, and reports should tolerate interruption.

### Implementation

Current owners:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/data_cache.rs`
- report writers.

Add helper if no existing one is suitable:

- new: `src-tauri/src/atomic_write.rs`

Behavior:

- write to temp file in same directory;
- flush;
- rename/replace;
- keep backup only where recovery flow is explicit;
- include path policy checks before write.

For SQLite-backed state, prefer transactions over ad hoc files.

### Tests

- interrupted write simulation where practical;
- invalid partial file ignored or recovered;
- settings/artifact legacy parse still works;
- no destructive migration.

## Expansion Track C: Report Mutation Contract Enforcement

### Problem

Report generation must not silently mutate source artifacts except where documented.

### Implementation

Use `docs/artifact_replay_report_evidence_contract.md` as authority.

Add tests around:

- `save_report_xlsx`;
- `save_report_xlsx_from_artifact`;
- `build_report_xlsx`;
- open/reveal operations;
- report-path warning propagation.

Frontend:

- immediate warning display after report save;
- History report state display;
- no durable operation log unless separately designed.

### Acceptance Criteria

- Direct report save does not mutate artifacts.
- Artifact-based report save only updates report path when documented.
- Warnings are operation-scoped and user-visible.

## Expansion Track D: Output Parity Expansion

### Problem

New report/export shapes need deterministic parity evidence.

### Implementation

Current owners:

- `scripts/run_output_parity_check.mjs`
- `scripts/check_output_parity.mjs`
- `scripts/check_output_semantic_parity.mjs`
- `scripts/output_parity_pairs.json`

Add pairs for:

- validation shortlist CSV if a headless output is added;
- options tuple/report outputs;
- portfolio feasibility exports;
- Simple Mode Markdown summary if release-critical.

Do not require byte-for-byte XLSX equality. Add sheet/cell invariants.

### Tests

- generated JSON semantic parity;
- XLSX workbook sheet names and key cell invariants;
- CSV header/order invariants;
- fixture artifact compatibility.

## Expansion Track E: Export Schema Registry

### Problem

JSON/CSV/XLSX/Markdown outputs can drift without a small schema registry.

### Implementation

Add a docs-backed export matrix first:

- export id;
- source command;
- source artifact family;
- schema version;
- output format;
- compatibility promise;
- parity coverage;
- public/private release status.

Potential new file:

- `docs/export_schema_registry.md`

Only add machine-readable JSON after the Markdown stabilizes.

## Expansion Track F: Advanced Artifact Analytics

### Problem

Users may need to inspect large saved result sets interactively.

### Implementation

This ties to frontend plan Track D.

Use JSON row flattening first:

- top rows;
- validation rows;
- options tuples;
- artifact metadata;
- portfolio candidate evidence.

Defer Arrow IPC until profiling shows JSON parsing or transfer is material.

### Acceptance Criteria

- Production history tables stay unchanged by default.
- Analytics lab is feature-flagged.
- Bundle and memory impact are measured.

## Second-Pass Expansion: Retention, Redaction, And Failure Status

### Problem

Artifact and report expansion should not create indefinite local evidence growth or leak private data through exports and support workflows.

### Current Anchors

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/redaction.rs`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

### Implementation

Add retention metadata:

- artifact family;
- created time;
- last opened time, if available;
- linked report paths;
- public/private data classification;
- estimated size;
- retention recommendation;
- purge dry-run status.

Add export failure status:

- `not_started`;
- `running`;
- `completed`;
- `completed_with_warnings`;
- `blocked`;
- `failed`;
- `cancelled`.

Redaction:

- redact API keys, bearer tokens, signing/updater keys, and private absolute paths;
- keep dataset identity hashes and report schema versions;
- make support-bundle export a dry run until the user explicitly confirms.

### Acceptance Criteria

- retention/purge preview never deletes without explicit user action;
- export failure reason appears in History without mutating source artifacts;
- redaction tests cover report paths and serialized command payload snippets.

## Second-Pass Expansion: Schema-Version Resolver

### Problem

Artifacts and reports will accumulate workflow-local schema versions. The frontend needs a tolerant resolver instead of scattered integer checks.

### Current Anchors

- `web/src/utils/core/runArtifactReview.ts`
- `web/src/utils/core/artifactIdentity.ts`
- `web/src/utils/core/stageCompletion.ts`
- `web/src/services/historyService.ts`
- `src-tauri/src/run_artifacts.rs`

### Implementation

Add a small resolver:

- artifact family;
- artifact schema version;
- report schema version;
- command response version;
- legacy fallback status;
- unsupported future-version status;
- safe review-only state.

Rules:

- older optional fields default to unknown/not available;
- future major versions are review-only unless a compatibility adapter exists;
- unsupported versions cannot be replayed or promoted to validation/portfolio inputs.

### Tests

- legacy artifact with missing schema version;
- future version blocked for replay but visible in history;
- report schema missing while artifact schema is known;
- stage completion does not treat unknown artifact status as complete.

## Second-Pass Expansion: Stage Completion Merge And Backfill

### Problem

The workflow model should derive completion consistently from current stores, hydrated artifacts, and saved reports.

### Current Anchors

- `web/src/utils/core/stageCompletion.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`

### Implementation

Create a merge policy:

- live store state wins for active runs;
- hydrated artifact state fills gaps only when source identity matches;
- report state can improve review status but cannot mark computational stages complete;
- cancelled/incomplete artifacts never satisfy downstream readiness.

Backfill:

- add a one-shot in-memory compatibility pass for legacy artifacts;
- do not rewrite artifacts during normal history load;
- offer explicit repair dry-run before any write-back.

## Non-Goals

- No generic artifact super-schema replacing workflow-local payloads.
- No live broker-action replay.
- No campaign resume from summary artifacts alone.
- No report tree scan during normal history refresh.
- No raw XLSX byte equality.

## Verification

Targeted:

- `npm --prefix web run test -- src/services/historyService.test.ts src/services/workflowArtifactsSync.test.ts src/stores/workflowArtifactsStore.test.ts src/components/HistoryWorkspacePanel.test.ts src/utils/core/runArtifactReview.test.ts`
- Tauri artifact/report tests as added.

Output parity:

- `node scripts/run_output_parity_check.mjs`

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
