# Run Artifact Schema Plan

Last updated: 2026-04-08

## Purpose

Define the canonical non-options run-artifact schema so save/list/load/hydrate/replay behavior stays stable across the backend and the frontend history/workflow layers.

## Scope Boundaries

In scope:

- top-level run-artifact JSON envelope
- summary/detail/hydration payload derivation
- required identity, schema, fingerprint, and dataset-meta fields
- legacy artifact-kind compatibility
- report-path and ML/meta attachments inside the saved artifact

Out of scope:

- options artifacts
- workbook sheet/chart content
- directory/path rules beyond the schema fields that carry paths

## Current User-Facing Behavior

Users can save runs, validations, meta outputs, and ML-related flows as artifacts, reopen them from history, hydrate workflow state from them, and replay details without rerunning the original workflow. Current behavior already depends on the saved artifact carrying:

- a recognized kind such as `vectorgrid_run_artifact`, with legacy `vectorbt_run_artifact` acceptance still preserved
- `config_fingerprint` and optional version information
- `report_xlsx_path` for later reopen flows
- normalized `dataset_meta` fields including timeframe and resample identity
- enough `run`, `validation`, `meta`, and `ml_signal` payload data for history/detail/hydration surfaces

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
  - owns artifact-kind recognition, dataset-meta normalization, config-fingerprint computation, save/list/load/detail/hydration logic, and legacy compatibility behavior
- `src-tauri/src/commands_artifacts.rs`
  - exposes the Tauri async wrappers for save/list/load/detail/hydration/delete/tag operations
- `web/src/services/historyService.ts`
  - consumes artifact summaries and detail payloads, normalizes saved fields, and drives reopen/delete flows
- `web/src/services/workflowArtifactsSync.ts`
  - hydrates current workflow state from the latest saved artifact
- `web/src/stores/workflowArtifactsStore.ts`
  - stores normalized artifact identity, report path, dataset metadata, and run/validation state in the frontend
- `web/src/services/validationService.ts`
  - persists current validation runs as `vectorgrid_run_artifact` values with `config_fingerprint`, `report_xlsx_path`, and `dataset_meta`

## Current Verification Surface

- unit tests in `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- validation-service tests that assert saved artifact contents

## Strengths Already Present

- the backend already centralizes artifact normalization instead of duplicating schema shaping in every command
- config-fingerprint computation already ignores some unstable fields and encodes a compatibility posture
- dataset metadata already receives normalization/backfill logic instead of being treated as opaque free-form JSON
- the frontend already distinguishes summary, detail, and hydration views rather than assuming one payload fits every screen

## Weaknesses / Failure Modes

- the saved artifact shape is rich enough that compatibility currently lives mostly in code and tests, not in one explicit schema contract
- legacy-kind support and current-kind support can drift if aliases are not written down
- optional attachments such as `report_xlsx_path`, `dataset_meta`, and ML/meta subtrees are easy to treat as incidental even though UI flows depend on them
- summary/detail/hydration derivation rules are subtle enough that a field can disappear from one view while still existing in the raw artifact

## Regressions To Avoid

- breaking old saved artifacts without an explicit dual-read or migration plan
- changing artifact kind names or top-level field names without updating both backend and frontend consumers
- dropping `config_fingerprint`, `report_xlsx_path`, or `dataset_meta` semantics relied on by replay/hydration flows
- making schema hardening so strict that partially older but still usable saved artifacts no longer load

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the artifact is effectively a versioned contract with required, optional, and nullable fields.
- [JSON Schema 2020-12 release notes](https://json-schema.org/draft/2020-12/release-notes) are relevant for thinking about additive schema evolution and compatibility rather than ad-hoc shape drift.
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259) is relevant because the artifact is persisted JSON and null-versus-absent behavior matters for compatibility.
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785) is relevant because the repo already computes config fingerprints from canonicalized artifact content, so schema evolution must respect deterministic identity.

## Proposed Improvement Approach

1. Write the artifact as a canonical envelope with explicit required top-level fields and clearly scoped optional attachments.
2. Document legacy-kind alias support and the current compatibility stance for older saved artifacts.
3. Document how summary, detail, and hydration payloads derive from the raw artifact and which fields each view must preserve.
4. Make `config_fingerprint`, `config_fingerprint_version`, `dataset_meta`, and `report_xlsx_path` first-class schema concepts rather than incidental fields.
5. Tie schema guarantees directly to the existing test surfaces that already enforce parts of the contract.

## Checklist

- [ ] Enumerate the required top-level run-artifact fields and their meaning.
- [ ] Document recognized artifact kinds and legacy alias support.
- [ ] Document the `run`, `validation`, `meta`, and `ml_signal` subtrees currently expected by consumers.
- [ ] Document `config_fingerprint` and version behavior, including fallback computation.
- [ ] Document normalized `dataset_meta` expectations, especially timeframe and resample identity fields.
- [ ] Document `report_xlsx_path` semantics and its relationship to reopen flows.
- [ ] Document summary/detail/hydration derivation rules and minimum payload guarantees.
- [ ] Add or tighten tests for missing fields, legacy fields, and compatibility fallback behavior if the schema changes.
- [ ] Verify saved artifacts still load, hydrate, and replay after schema hardening.

## Verification Plan

- run targeted unit tests in `src-tauri/src/run_artifacts.rs`
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/services/workflowArtifactsSync.test.ts`
- run targeted validation-service tests that assert persisted artifact contents
- manually save, reload, hydrate, and replay one representative non-options artifact

## Rollout / Fallback / Compatibility Notes

- preserve existing non-options artifact kinds and top-level field names first
- prefer additive schema expansion over breaking renames
- if a breaking schema change is unavoidable, add a dual-read path or explicit migration before deleting compatibility code

## Related Docs

- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/24_tauri_run_artifact_paths_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
