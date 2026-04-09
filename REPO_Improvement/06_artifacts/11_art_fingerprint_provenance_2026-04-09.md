# Artifact Fingerprint And Provenance Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to artifact fingerprint and provenance handling so config fingerprints, dataset provenance, and summary/detail identity fields remain deterministic and comparable across saved-run flows.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- config fingerprint computation and versioning
- dataset/source provenance fields surfaced in artifact summaries
- frontend history/detail consumers of those fields

Out of scope:

- tag editing and retention policy except where they intersect identity/provenance
- options-related artifact fingerprints
- broader benchmark provenance outside saved artifacts

## Current User-Facing Behavior

Saved run summaries already expose `config_fingerprint`, optional `config_fingerprint_version`, timeframe/resample provenance, source size/mtime/content fingerprint, and related dataset metadata. Those fields support comparison, history browsing, and later reasoning about whether two artifacts are meaningfully the same strategy setup versus merely the same filename.

The backend also already normalizes fingerprint behavior so some unstable fields such as `data_id` or cross-dataset ordering do not cause unnecessary identity churn.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/utils/core/artifactIdentity.ts`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `web/src/services/historyService.test.ts`
- `web/src/stores/historyStore.search.test.ts`

## Strengths Already Present

- fingerprint computation is already centralized
- fingerprint versioning already exists
- dataset/source provenance fields are already included in saved-run summaries
- backend tests already cover important normalization behavior
- history/detail consumers already surface related identity information

## Weaknesses / Failure Modes

- fingerprint and provenance fields are adjacent enough to be conflated
- canonicalization rules can drift if not documented explicitly
- frontend and backend identity reasoning could diverge if one side hardcodes assumptions
- provenance fields are rich enough that partial omission can quietly reduce their value

## Regressions To Avoid

- changing config-fingerprint rules without updating version semantics and tests
- letting unstable fields re-enter fingerprint computation accidentally
- dropping provenance fields such as source size/mtime/content fingerprint from saved-run summaries
- making history/detail consumers assume a fingerprint field that older artifacts do not carry
- conflating config identity with dataset provenance identity

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because fingerprint/provenance fields are structured contract data, not incidental metadata.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because fingerprint inputs and artifact metadata are persisted JSON.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because stable canonical ordering is the backbone of deterministic config fingerprints.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because fingerprint/provenance fields become shared history/detail state once loaded into the frontend.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because identity/provenance behavior is exercised through history/search/detail flows.

## Proposed Improvement Approach

1. Separate config fingerprint from dataset/source provenance explicitly in the contract.
2. Document fingerprint inputs, ignored fields, and version semantics.
3. Document which provenance fields are summary-level versus detail-level expectations.
4. Keep frontend history/detail consumers aligned with backend identity semantics.
5. Expand tests around canonicalization, version fallback, and provenance-field preservation.

## Checklist

- [ ] Document config-fingerprint inputs, ignored fields, and version semantics.
- [ ] Document summary-level provenance fields and their meaning.
- [ ] Document fallback behavior when older artifacts lack fingerprint/version fields.
- [ ] Add or tighten tests for canonicalization and ignored-field behavior.
- [ ] Add or tighten tests for provenance-field preservation in summaries and details.
- [ ] Keep frontend history/detail identity logic aligned with backend semantics.

## Verification Plan

- run targeted `run_artifacts.rs` tests
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/stores/historyStore.search.test.ts`
- manually inspect two related saved runs with different unstable fields and confirm identity/provenance behavior still makes sense

## Rollout / Fallback / Compatibility Notes

- keep current fingerprint versioning additive
- prefer bumping the fingerprint version over silently redefining an existing version
- preserve provenance fields even when some values are unavailable
- continue treating dataset/source provenance as related to but distinct from config identity

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/02_artifact_dataset_meta_norm_2026-04-09.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/02_data/28_dataset_fingerprint_2026-04-08.md`
