# Dataset Fingerprint Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because both frontend and backend fingerprints are computed from JSON-shaped payloads
- [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because deterministic canonicalization is the core requirement before hashing structured payloads
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because fingerprint stability depends on stable cross-layer request schemas
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because source-content identity and source file diagnostics are tied to file-origin context
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

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
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the difference between `config_fingerprint` and `source_content_fingerprint`.
- [x] Document the current canonicalization and strip rules before hashing.
- [x] Document the current version prefix and when it may change.
- [x] Document cross-ticker dataset ordering normalization.
- [x] Add or tighten frontend/backend parity checks for representative request families.
- [x] Clarify that the current hash is for stable identity, not for cryptographic trust.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/utils/core/artifactIdentity.test.ts`
- run `cargo test -p ta_desktop_demo_app run_artifacts`
- compare one frontend-computed config fingerprint with the backend-normalized artifact fingerprint for the same request family
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target

## Rollout / Fallback / Compatibility Notes

- preserve the current fingerprint version prefix unless a new migration path is documented
- prefer dual-read / dual-write transitions if a future fingerprint version is introduced
- do not repurpose `source_content_fingerprint` as a substitute for request identity
- if strip rules expand, update tests and this document in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added a second focused frontend fingerprint test in `web/src/utils/core/artifactIdentity.test.ts` to pin two distinct properties that matter in a public repo: key-order noise must not change `config_fingerprint`, and a real request change must still produce a different fingerprint.
- Re-verified the backend half of the contract instead of guessing from frontend behavior. `src-tauri/src/run_artifacts.rs` remains the authority for `config_fingerprint` generation, and `src-tauri/src/data_cache.rs` remains the authority for content-fingerprint stability/change detection.
- Kept the identity split explicit: request/config identity and dataset-content identity stay separate, and this slice improved the parity evidence without repurposing FNV hashes as cryptographic guarantees.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app run_artifacts --lib`, `cargo test -p ta_desktop_demo_app data_cache --lib`, `npm.cmd --prefix web run test -- src/utils/core/artifactIdentity.test.ts src/services/validationService.test.ts src/stores/dataStore.actions.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
- `REPO_Improvement/02_data/29_dataset_invalidation_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
