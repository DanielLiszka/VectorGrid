## Purpose

Define the improvement plan for the core dataset-meta and path contract that moves through run artifacts, workflow hydration, validation reuse, and campaign-prepared datasets.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_campaign.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/validationService.ts`
- dataset-meta/path fields such as `csv_path`, timeframe/base/derived timeframe, resample metadata, and source identity/fingerprint

Out of scope:

- frontend dataset picker UX
- new dataset provenance fields not already implied by current artifacts
- options-related dataset/artifact paths

## Current User-Facing Behavior

Dataset provenance is already visible across several surfaces:

- saved-run summaries and detail show `csv_path`, timeframe, and resample-related fields
- workflow hydration can restore dataset-meta into the active run state
- validation can reuse dataset-meta from current workflow artifacts
- campaigns build parallel dataset-meta objects before optimization/validation

The user-visible promise is that a reopened run still points to the same data context. The risk is that dataset-meta construction is not yet singular.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_campaign.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/validationService.ts`

Key current ownership:

- `run_artifacts.rs` normalizes/backfills artifact `dataset_meta` and exposes it through summary, detail, and hydration payloads
- `commands_campaign.rs` prepares campaign dataset metadata before optimization/validation
- `workflowArtifactsStore.ts` keeps current run dataset metadata available to the Vue runtime
- `validationService.ts` consumes artifact-backed dataset metadata for downstream reuse and path resolution

## Current Verification Surface

- dataset-meta normalization tests in `src-tauri/src/run_artifacts.rs`
- hydration/detail payload tests in `src-tauri/src/run_artifacts.rs`
- artifact-backed dataset-meta tests in `web/src/services/validationService.test.ts`
- campaign/meta artifact flow tests in `web/src/services/metaService.test.ts`

## Strengths Already Present

- dataset meta already includes more than just `csv_path`
- the Rust artifact layer already backfills schema version, timeframe, resample rule/anchor, and source identity fields
- workflow hydration already preserves dataset-meta instead of flattening everything into loose strings
- campaign preparation already attempts to carry parallel dataset metadata

## Weaknesses / Failure Modes

- dataset-meta construction is duplicated between run-artifact normalization and campaign dataset prep
- there is no shared schema validator or canonical serializer for dataset meta
- TypeScript consumers often treat `datasetMeta` as an untyped raw object
- fallback resolution across current run, validation state, and campaign state makes it easy for the effective source of truth to drift

## Regressions To Avoid

- changing dataset-meta field meaning without coordinated artifact/detail/hydration updates
- silently dropping source identity or resample fields during artifact replay
- letting campaign dataset-meta diverge from saved-run dataset-meta for the same underlying dataset
- widening the path contract without test coverage for hydration and validation reuse

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because dataset meta already behaves like a schema-bearing artifact contract, not an incidental blob.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because the Rust layer currently transports dataset meta as generic JSON values that frontend code then reuses.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because dataset-meta payloads cross the Rust/TypeScript boundary in multiple command flows.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because `csv_path` remains the anchor to the original dataset source and should stay clearly tied to CSV-backed provenance.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because hydrated dataset-meta state is shared across multiple runtime stores and services.

## Proposed Improvement Approach

1. Freeze one canonical dataset-meta contract for non-options artifacts.
2. Extract or document a single construction path so run artifacts and campaign prep do not encode subtly different dataset-meta shapes.
3. Add explicit schema expectations for:
   - `csv_path`
   - timeframe/base/derived timeframe
   - resample flags/rules/anchor
   - source-size, mtime, and fingerprint fields
4. Tighten TypeScript consumption by centralizing normalization instead of treating dataset meta as raw everywhere.
5. Use hydration and validation reuse tests as the compatibility gate for any dataset-meta change.

## Checklist

- [ ] Document the canonical dataset-meta field set and meaning.
- [ ] Audit duplication between `run_artifacts.rs` and `commands_campaign.rs`.
- [ ] Add or tighten tests for dataset-meta normalization and backfill behavior.
- [ ] Add or tighten workflow-hydration tests for dataset-meta path preservation.
- [ ] Add or tighten validation tests that reuse artifact-backed dataset metadata.
- [ ] Decide whether a shared Rust helper should own dataset-meta construction across artifacts and campaigns.

## Verification Plan

- run dataset-meta normalization tests in `src-tauri/src/run_artifacts.rs`
- run artifact detail/hydration tests in `src-tauri/src/run_artifacts.rs`
- run `validationService.test.ts` cases that exercise artifact-backed dataset-meta reuse
- run `metaService.test.ts` cases that exercise campaign dataset-meta flow into workflow artifacts

## Rollout / Fallback / Compatibility Notes

- preserve current field names and meanings unless there is a strong compatibility reason to change them
- prefer shared normalization/helpers over proliferating more raw `Value` handling
- if a cleanup risks breaking saved-run reopening or validation reuse, keep the current shape and document it more clearly first

## Related Docs

- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
- `REPO_Improvement/02_data/28_dataset_fingerprint_2026-04-08.md`
- `REPO_Improvement/06_artifacts/02_artifact_dataset_meta_norm_2026-04-09.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
