# ML Score Cache Bundles Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for ML Signal score-cache bundles so the cache JSON, manifest, schema/version fields, bundle identifiers, and downstream reuse rules remain explicit and replayable across build, validation, sweep, and artifact surfaces.

## Scope Boundaries

In scope:

- score-cache JSON contract
- score-cache manifest contract
- bundle-related metadata exposed through artifacts
- schema/version/data-id compatibility
- score-space and score-scope metadata

Out of scope:

- dataset export internals
- direct validation/policy-sweep execution details
- model training internals

## Current User-Facing Behavior

The app already treats ML score caches as more than one opaque file. Current behavior includes:

- Stage 2 bundle/status controls in the ML Signal workspace
- bundle refresh and bundle-status refresh actions
- building a score-cache JSON file
- writing a score-cache manifest
- carrying score-cache metadata through artifact details
- validating the cache against:
  - version
  - kind
  - strategy kind
  - feature schema version
  - `data_id`

Current user-visible and artifact-visible summary fields already include:

- `model_bundle_id`
- `score_cache_path`
- `score_manifest_path`
- `feature_schema_path`
- `feature_schema_version`
- `bundle_id`
- `matched_rows`
- `duplicate_timestamps`
- `coverage_ratio`
- `score_space`
- `score_scope`

Bundle discovery already uses an app-local model directory rooted under `models/ml_signal/{bundle_id}/manifest.json`, and missing-manifest cases already surface as warning-style bundle state rather than crashing the workflow. That means the repo already has a practical bundle contract even if the terminology is not fully formalized yet.

## Current Code Surface

- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- Rust tests in `crates/app_core/src/ml_signal.rs`
- artifact/history tests that consume ML artifact detail fields

## Strengths Already Present

- the backend already enforces kind/version/schema/data-id checks
- a manifest is already written instead of relying on an undocumented JSON blob alone
- score-space and score-scope are already preserved as explicit metadata
- duplicate-timestamp and coverage summaries already survive into the UI
- artifact plumbing already carries bundle-related metadata forward
- bundle status is already classified into health-style states based on manifest/file checks

## Weaknesses / Failure Modes

- bundle identity is spread across several files and payload fields rather than described as one contract
- the relationship between cache JSON, manifest, and artifact attachment is still implicit
- score-space and score-scope semantics are rich enough to drift if not documented clearly
- it is easy to weaken compatibility checks for convenience if the bundle contract stays under-specified
- feature-schema version and bundle ID can become “just strings” unless their meaning is locked explicitly
- bundle status can go stale if the UI, manifest checks, and artifact summaries drift apart

## Regressions To Avoid

- weakening version, kind, strategy-kind, schema-version, or `data_id` checks
- dropping manifest generation or manifest linkage
- hiding duplicate-timestamp or low-coverage signals
- losing bundle-related fields in run artifacts or history hydration
- treating raw-margin-only and probability-space caches as interchangeable

## External Research Summary

- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because score caches and manifests are JSON contracts
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)
  - relevant because the cache/manifest pair already behaves like a versioned schema contract
- [scikit-learn calibration guide](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because probability-space versus raw-score semantics materially affect what later validation can do
- [scikit-learn model persistence](https://scikit-learn.org/stable/model_persistence.html)
  - relevant because the repo already behaves like a model-adjacent artifact bundle inventory instead of a single-file cache loader
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because cache generation and reuse cross a Tauri command boundary with structured results

## Proposed Improvement Approach

1. Define the score-cache bundle inventory explicitly:
   - bundle manifest under `models/ml_signal/{bundle_id}/manifest.json`
   - score-cache JSON
   - manifest JSON
   - feature-schema metadata
   - optional bundle identifier carried into artifacts
2. Document required identity and compatibility fields:
   - `version`
   - `kind`
   - `strategy_kind`
   - `feature_schema_version`
   - `data_id`
3. Define quality and coverage fields as first-class bundle metadata:
   - `matched_rows`
   - `duplicate_timestamps`
   - `coverage_ratio`
   - `score_space`
   - `score_scope`
4. Document the difference between “bundle built successfully” and “bundle is valid for a specific downstream validation request”.
5. Expand tests around mismatched schema/data IDs, raw-only caches, and artifact round-tripping.
6. Document bundle-status states and when the UI should surface `healthy` versus `warning` conditions.

## Checklist

- [ ] Document the full score-cache bundle inventory and file/path relationships.
- [ ] Document required identity and compatibility fields.
- [ ] Document `score_space` and `score_scope` semantics.
- [ ] Document coverage, matched-row, and duplicate-timestamp fields as quality signals.
- [ ] Document bundle-status health/warning classification and missing-manifest behavior.
- [ ] Document how bundle metadata is attached to run artifacts and history views.
- [ ] Add tests for mismatched version/kind/schema/data-id cases.
- [ ] Add tests for raw-score versus probability-space bundle restrictions.
- [ ] Add tests for manifest and artifact round-trip preservation.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run targeted Rust tests in `crates/app_core/src/ml_signal.rs`
- inspect one generated score-cache JSON and manifest pair
- inspect one discovered bundle manifest and one missing-manifest bundle-status response
- inspect one saved artifact carrying score-cache bundle metadata

## Rollout / Fallback / Compatibility Notes

- preserve current bundle checks unless a versioned migration is written
- prefer additive bundle metadata over renaming existing keys
- keep manifest generation mandatory if downstream consumers rely on it

## Related Docs

- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/08_ml/04_ml_validation_surface_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
