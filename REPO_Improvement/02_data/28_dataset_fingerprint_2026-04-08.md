# Dataset Fingerprint Plan

Last updated: 2026-04-08

## Purpose

Define the current fingerprinting contract so config identity, source-content identity, and canonicalization can be improved without mixing distinct identity concepts or breaking artifact compatibility.

## Scope Boundaries

In scope:

- config fingerprint generation for run + validation requests
- source-content fingerprint fields carried in `dataset_meta`
- canonical JSON and strip rules used before hashing
- frontend/backend parity for config fingerprint semantics

Out of scope:

- inventing new cryptographic trust guarantees
- raw dataset deduplication engines
- options-specific artifact fingerprinting

## Current User-Facing Behavior

The current app already uses more than one notion of identity:

- `config_fingerprint`
  - identifies the logical run/validation configuration while intentionally ignoring volatile fields such as `data_id` and candidate payloads
- `source_content_fingerprint`
  - identifies the underlying dataset file content when available

That separation is already meaningful for artifact continuity, report reproduction, and cross-session comparisons. The shipped behavior is already telling us that “same request” and “same dataset bytes” are not the same claim.

## Current Code Surface

- `web/src/utils/core/artifactIdentity.ts`
  - canonicalizes JSON, strips volatile fields, and computes `cfg1-fnv1a64-*`
- `web/src/utils/core/artifactIdentity.test.ts`
  - verifies ignored fields and deterministic behavior
- `src-tauri/src/run_artifacts.rs`
  - computes/normalizes `config_fingerprint`, `config_fingerprint_version`, and `source_content_fingerprint`
- `web/src/utils/core/datasetMeta.ts`
  - carries `source_content_fingerprint` into artifact metadata when present
- `src-tauri/src/commands_reports.rs`
  - preserves source-content fingerprint in report data

## Current Verification Surface

- `web/src/utils/core/artifactIdentity.test.ts`
- `src-tauri/src/run_artifacts.rs` tests
  - especially `config_fingerprint_ignores_data_ids_candidates_and_cross_dataset_order`
- artifact-detail/report normalization tests in `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- config and dataset-content identity are already separated
- the config fingerprint has an explicit versioned prefix
- canonicalization and strip rules already remove volatile request noise
- cross-dataset order normalization already exists for cross-ticker validation inputs
- source-content fingerprint is already preserved inside the dataset metadata path

## Weaknesses / Failure Modes

- the difference between config identity and dataset-content identity is not documented in one place
- frontend and backend fingerprint logic can drift without a parity contract
- FNV-based hashing is deterministic and cheap, but contributors could misread it as a cryptographic guarantee
- there is no implementation doc that lists which request fields are intentionally stripped before hashing
- version-prefix semantics are partly implicit and could be changed casually

## Regressions To Avoid

- treating CSV path or transient `data_id` as stable dataset identity
- folding `source_content_fingerprint` into config identity
- changing strip rules without synchronized frontend/backend verification
- changing the fingerprint version prefix without a migration plan
- weakening deterministic cross-ticker ordering rules

## External Research Summary

- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because both frontend and backend fingerprints are computed from JSON-shaped payloads
- [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because deterministic canonicalization is the core requirement before hashing structured payloads
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because fingerprint stability depends on stable cross-layer request schemas
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because source-content identity and source file diagnostics are tied to file-origin context

## Proposed Improvement Approach

1. Document two separate identity contracts:
   - configuration identity
   - source-content identity
2. Record the exact strip rules for config fingerprinting:
   - `data_id` / `dataId`
   - validation candidates
   - admission provenance
   - cross-dataset ordering normalization
3. Define versioning expectations for the current `cfg1-fnv1a64-*` format.
4. Add or preserve parity tests across frontend and backend for the same logical input families.
5. Keep future changes additive:
   - new fingerprint versions should coexist with old ones during migration
   - dataset-content identity should remain separate from config identity

## Checklist

- [ ] Document the difference between `config_fingerprint` and `source_content_fingerprint`.
- [ ] Document the current canonicalization and strip rules before hashing.
- [ ] Document the current version prefix and when it may change.
- [ ] Document cross-ticker dataset ordering normalization.
- [ ] Add or tighten frontend/backend parity checks for representative request families.
- [ ] Clarify that the current hash is for stable identity, not for cryptographic trust.

## Verification Plan

- run `npm --prefix web test -- src/utils/core/artifactIdentity.test.ts`
- run `cargo test -p ta_desktop_demo_app run_artifacts`
- compare one frontend-computed config fingerprint with the backend-normalized artifact fingerprint for the same request family

## Rollout / Fallback / Compatibility Notes

- preserve the current fingerprint version prefix unless a new migration path is documented
- prefer dual-read / dual-write transitions if a future fingerprint version is introduced
- do not repurpose `source_content_fingerprint` as a substitute for request identity
- if strip rules expand, update tests and this document in the same change

## Related Docs

- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
- `REPO_Improvement/02_data/29_dataset_invalidation_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
