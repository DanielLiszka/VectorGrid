## Purpose

Define the improvement plan for the ML Signal feature-schema contract so exported datasets, score caches, saved artifacts, and reopened ML runs all agree on what features exist and which schema version is being used.

## Scope Boundaries

In scope:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `src-tauri/src/run_artifacts.rs`
- feature schema file creation, schema versioning, artifact attachment, and reopen/display semantics

Out of scope:

- changing the ML feature set itself for modeling purposes
- score-threshold policy semantics unrelated to schema identity
- options-related schemas

## Current User-Facing Behavior

The ML Signal workflow already exposes feature-schema awareness:

- dataset export and score-cache flows produce feature-schema paths
- saved run detail can show feature schema path and version
- the ML workspace summary displays feature-schema information
- score-cache validation checks schema version compatibility

This is a strong foundation. The risk is that schema identity is spread across export, cache generation, saved artifacts, and reopen flows rather than being treated as one explicit contract.

## Current Code Surface

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- ML Signal unit tests in `crates/app_core/src/ml_signal.rs`
- `MlSignalWorkspacePanel.test.ts`
- `mlSignalService.test.ts`
- `run_artifacts.rs` detail tests for feature-schema fields

## Strengths Already Present

- feature schema is already a named artifact, not an implicit side effect
- schema version is already persisted and checked
- saved run detail already surfaces feature schema path/version
- score-cache loading already rejects incompatible schema versions

## Weaknesses / Failure Modes

- schema creation, persistence, validation, and presentation live across multiple layers
- it is easy for frontend wording and backend schema/version checks to drift apart
- schema path and schema version can look informative while still lacking a clearly documented contract
- users may not know whether the schema path is merely informative or operationally required for reopen/validation

## Regressions To Avoid

- changing schema version or meaning without synchronized export/cache/reopen updates
- allowing score-cache reopen flows to hide schema incompatibility
- presenting feature-schema details inconsistently across workspace, saved runs, and artifacts
- turning schema identity into a best-effort hint instead of a true contract

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the ML feature schema should be treated as a deliberate versioned contract.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because schema and artifact fields are serialized and moved through JSON-heavy layers.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because feature-schema identity travels across Rust command responses into the Vue runtime.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the feature schema exists to describe exported CSV-backed ML datasets.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because schema state is surfaced across the ML workspace and saved-run reopen flows.

## Proposed Improvement Approach

1. Treat the feature schema as a versioned contract artifact, not just a helpful side file.
2. Document the schema lifecycle:
   - dataset export
   - score-cache generation
   - score-cache validation
   - saved artifact attachment
   - saved-run reopen
3. Tighten tests around version mismatch and missing-schema behavior.
4. Clarify UI wording so users know whether the schema is informative, required, or both.
5. Keep schema identity centralized and explicit before expanding the feature set.

## Checklist

- [ ] Document the feature-schema artifact contract and version meaning.
- [ ] Add or tighten tests for schema creation and version mismatch rejection.
- [ ] Add or tighten saved-artifact tests for feature schema path/version attachment.
- [ ] Audit workspace and saved-run wording for schema significance and fallback behavior.
- [ ] Confirm score-cache validation continues to enforce schema compatibility explicitly.

## Verification Plan

- run `crates/app_core/src/ml_signal.rs` tests covering schema generation and score-cache loading
- run `mlSignalService.test.ts`
- run `MlSignalWorkspacePanel.test.ts`
- run relevant `run_artifacts.rs` detail tests for schema fields

## Rollout / Fallback / Compatibility Notes

- preserve the current schema-version contract unless a deliberate migration exists
- prefer clearer validation and messaging over schema-field expansion
- if a change weakens reopen or cache-validation safety, revert to the stricter prior behavior

## Related Docs

- `REPO_Improvement/08_ml/02_ml_config_paths_2026-04-09.md`
- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md`
