# Stage Completion Backend Backfill And Report Schema Versioning

Status: merge/backfill and explicit schema emission implemented; summary expansion/tolerant resolver remain planned
Risk: moderate due saved artifact compatibility and output parity
Primary value: make backend-owned artifact summaries and reports carry stable, tolerant stage/report metadata

## Summary

Frontend stage-completion packets now cover more workflow surfaces. Backend run-artifact backfill exists, but it currently treats any existing `stage_completions` array as complete. That means a partial legacy packet array can suppress missing known packets. The next safe backend slice is merge/upsert backfill by `(stage, substage)` while preserving unknown objects.

Report schema versioning should also become explicit but tolerant. Existing report payloads use `kind` and `version`; some headless artifacts differ. Strict validation would break legacy paths, so the first slice should emit a `report_schema_version` alias while continuing to accept missing schema as legacy.

Relevant local contract:

- Artifact/replay/report evidence contract: `docs/artifact_replay_report_evidence_contract.md`

## Current Anchors

- Frontend packet utilities: `web/src/utils/core/stageCompletion.ts`
- Backend run artifact normalization: `src-tauri/src/run_artifacts.rs`
- Report command boundary: `src-tauri/src/commands_reports.rs`
- Report workbook model: `src-tauri/src/commands_reports_validation_common.rs`
- Frontend report payload builder: `web/src/services/validationService.ts`
- Existing parent plan: `05_stage_completion_reporting_expansion.md`

## Existing Behavior

- Backend synthesizes run-artifact packets for optimization, validation, and ML Signal.
- Existing arrays are returned as-is, even if incomplete.
- Run artifact hydration/detail responses include derived packets.
- Meta, Stage 5, and policy-eval summaries do not carry backend stage completions.
- Report export does not enforce kind/version compatibility.

## Additive Boundary

- Preserve unknown future packet objects.
- Do not mutate artifacts during list/load/report export.
- Materialize run-artifact backfill only through existing save/repair paths.
- Keep report schema checks tolerant.
- Preserve `version: 1` for compatibility while optionally adding `report_schema_version: 1`.
- Do not change CSV shortlist row shape in the first slice.

## Non-Goals

- No strict schema rejection for legacy artifacts.
- No report export as hidden artifact migration.
- No removal of frontend fallback packet builders.
- No broad output-baseline refresh unless semantic parity scripts require an intentional update.

## Implementation Checklist

### Phase 1: Merge/Backfill

- [x] Replace existing-array early return with merge/upsert by `(stage, substage)`.
- [x] Preserve original packet objects when a matching packet exists.
- [x] Append synthesized missing known packets with `backfilled: true`.
- [x] Preserve unknown stage/substage packets unchanged.
- [ ] Add tests for partial arrays, unknown packets, and duplicate known packets.

### Phase 2: Backend Summary Packet Expansion

- [ ] Add optional `stage_completions` to meta, Stage 5, and policy-eval summaries.
- [ ] Build packets from backend-owned summary fields where available.
- [ ] Keep fields optional and skip empty arrays.
- [ ] Keep frontend normalization fallback in place.

### Phase 3: Report Schema Metadata

- [ ] Define a single report schema version constant.
- [x] Add `report_schema_version: 1` to direct frontend report payloads.
- [x] Add `report_schema_version: 1` to artifact-derived and headless report payloads.
- [ ] Add tolerant resolver for missing/current/future schema versions where report commands inspect payloads.
- [ ] Update output parity ignore/update rules only if the new field changes parity evidence intentionally.

### Phase 4: Documentation And Compatibility

- [ ] Document legacy behavior in tests.
- [ ] Confirm report export only mutates `report_xlsx_path` where already allowed.
- [ ] Add notes to evidence contract if schema metadata behavior changes.

## Validation Checklist

- [ ] `cargo test -p ta_desktop_demo_app run_artifacts`
- [ ] `cargo test -p ta_desktop_demo_app commands_reports`
- [ ] `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`
- [ ] `npm --prefix web run test -- stageCompletion validationService metaService optionsExpressionService historyService workflowArtifactsStore`
- [ ] `node scripts/run_output_parity_check.mjs`
- [ ] `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

## Regression Watchpoints

- Do not drop unknown future stage packets.
- Do not convert legacy report absence into an error.
- Do not widen report mutation behavior.
- Do not add schema fields to CSV rows unless CSV consumers/tests are updated.
- Adding optional fields can still affect snapshot/parity tests; treat parity diffs as evidence to review.

## Definition Of Done

- Partial legacy stage-completion arrays are completed without losing original packets.
- Backend summaries can carry optional stage packets where safe.
- Report schema metadata is explicit for new payloads and tolerant for old payloads.
