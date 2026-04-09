# Dataset Meta Schema Plan

Last updated: 2026-04-08

## Purpose

Define the current `dataset_meta` contract so artifacts, reports, history detail, and dataset-context UX can be improved without breaking the schema already moving across frontend and Tauri layers.

## Scope Boundaries

In scope:

- `dataset_meta` creation for run artifacts
- normalization/backfill of `dataset_meta` in Tauri artifact handling
- report-builder consumption of `dataset_meta`
- history/detail and validation persistence paths that carry `dataset_meta`

Out of scope:

- raw candle payload storage
- options-specific dataset flows
- inventing new dataset fields that are not justified by current repo behavior

## Current User-Facing Behavior

The app already persists and reuses dataset metadata rather than treating the CSV path as the only data context. The current schema supports user-facing context such as:

- label and CSV path
- timeframe, base timeframe, and derived timeframe
- resample-applied state plus rule/anchor
- source, market, symbol, and adjusted flag
- row-count/time-range metadata
- source file size, source mtime, and optional source content fingerprint

This data already influences report generation, artifact detail, and workflow continuity. It is not theoretical metadata; it is part of the current product behavior.

## Current Code Surface

- `web/src/utils/core/datasetMeta.ts`
  - builds artifact-facing dataset metadata with `schema_version = 2`
- `web/src/services/validationService.ts`
  - persists `dataset_meta` with validation artifacts
- `src-tauri/src/run_artifacts.rs`
  - normalizes, backfills, and persists artifact dataset metadata
- `src-tauri/src/commands_reports.rs`
  - converts persisted `dataset_meta` into report `data.meta` / `data.ctx`
- `web/src/stores/dataStore.ts`
  - exposes current/base dataset context that feeds metadata construction

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` unit tests
  - especially `dataset_meta_normalization_backfills_resample_identity`
  - and `dataset_meta_normalization_marks_resampled_when_base_and_derived_differ`
- validation-artifact persistence coverage through `web/src/services/validationService.ts`
- artifact-backed report-builder tests in `src-tauri/src/commands_reports.rs`

## Strengths Already Present

- the schema already has an explicit version
- the frontend builder and backend normalizer already agree on key families
- derived-timeframe and resample metadata are already modeled separately from base data
- report generation already consumes a filtered subset instead of re-deriving all dataset context
- the schema includes both UX-oriented fields and reproducibility-oriented fields

## Weaknesses / Failure Modes

- there is no single document that defines required vs optional fields
- frontend and backend normalization can still drift if the schema changes silently
- the report builder uses a subset view of `dataset_meta`, but that subset is not documented as contract
- null/empty behavior for fields like `source`, `market`, or `adjusted` is only implicit in code
- contributors could mistakenly treat `timeframe` and `derived_timeframe` as interchangeable without documenting the rule

## Regressions To Avoid

- removing or renaming schema keys that artifacts and reports already rely on
- losing `base_timeframe` / `derived_timeframe` distinction
- degrading resample identity into a path-only guess
- stripping provenance fields like `source_content_fingerprint` or `source_mtime_utc_ms`
- changing schema version or normalization rules without synchronized frontend/backend updates

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because `dataset_meta` behaves like a cross-layer JSON schema even though it is not yet formalized as one
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because artifact/report data moves as JSON and the schema should remain stable and unambiguous
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because source size and mtime fields ultimately derive from file metadata semantics
- [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because stable structured metadata and stable hashing depend on deterministic JSON treatment

## Proposed Improvement Approach

1. Freeze the current `dataset_meta` field inventory into an explicit contract.
2. Separate fields into tiers:
   - core identity/provenance
   - timeframe/resample semantics
   - descriptive labels
   - source-file diagnostics
3. Document frontend creation and backend normalization separately.
4. Document the report-builder subset explicitly so report consumers know which `dataset_meta` fields survive into report payloads.
5. Add parity checks for any schema change that touches:
   - key names
   - nullability
   - versioning
   - resample semantics

## Checklist

- [ ] Enumerate the current `dataset_meta` field set and schema version.
- [ ] Document required, optional, nullable, and derived fields separately.
- [ ] Document the relationship between `timeframe`, `base_timeframe`, and `derived_timeframe`.
- [ ] Document resample fields and when `resample_applied` becomes true.
- [ ] Document provenance fields such as size, mtime, and source-content fingerprint.
- [ ] Document the subset that `commands_reports.rs` preserves in `report.data`.
- [ ] Add or tighten frontend/backend parity tests when the schema changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app run_artifacts`
- run `cargo test -p ta_desktop_demo_app commands_reports`
- manually inspect one saved artifact and one artifact-backed report payload to confirm field carry-through

## Rollout / Fallback / Compatibility Notes

- preserve `schema_version = 2` behavior unless there is a real migration
- prefer additive fields over repurposing existing keys
- if the report subset expands, keep old fields stable rather than reinterpreting them
- do not fall back to CSV path alone when richer dataset metadata is already available

## Related Docs

- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/02_data/26_derived_dataset_identity_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
