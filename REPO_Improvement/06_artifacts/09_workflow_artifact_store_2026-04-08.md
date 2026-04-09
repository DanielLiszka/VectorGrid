# Workflow Artifact Store Plan

Last updated: 2026-04-08

## Purpose

Define the `workflowArtifactsStore` contract so current-run and current-validation hydration, normalization, identity tracking, and no-op detection remain stable across optimization, validation, ML, history, and workflow-sync flows.

## Scope Boundaries

In scope:

- `web/src/stores/workflowArtifactsStore.ts`
- `applyWorkflowArtifactsPayload`
- current-run/current-validation normalization
- latest identity snapshot and no-op semantics

Out of scope:

- Rust artifact building
- full history detail rendering
- options artifact state

## Current User-Facing Behavior

Users experience this store indirectly whenever a run is saved, hydrated, reopened from history, synced from the latest artifact, or reused by analysis/validation flows. The current store already normalizes hydrated payloads, preserves run identity, keeps source tags, avoids timestamp churn on identical normalized payloads, and treats validation appearing later for the same run as a meaningful state change. Several frontend flows already assume those semantics, especially latest-artifact sync and post-validation refresh.

## Current Code Surface

- `web/src/stores/workflowArtifactsStore.ts`
  - owns `currentRun`, `currentValidation`, source tagging, normalization helpers, signatures, and latest identity snapshot helpers
- `web/src/stores/workflowArtifactsStore.test.ts`
  - verifies no-op payload behavior, validation-arrives-later semantics, and metric-surface normalization
- `web/src/services/workflowArtifactsSync.ts`
  - consumes identity snapshot and apply result for latest-artifact sync
- `web/src/services/validationService.ts`
  - applies payloads after validation/save flows
- `web/src/services/mlSignalService.ts`
  - applies payloads for ML stages that also reuse the store
- `src-tauri/src/run_artifacts.rs`
  - upstream producer for hydrated payload shape

## Current Verification Surface

- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/mlSignalService.test.ts`

## Strengths Already Present

- current-run/current-validation payloads are already normalized into one shared store
- identity snapshot helpers already make artifact-sync skip logic safe and explicit
- no-op detection already avoids unnecessary timestamp churn
- metric-surface and ML subtrees are already normalized instead of being blindly accepted
- the store already acts as a bridge between optimization, validation, history, and ML flows

## Weaknesses / Failure Modes

- normalization logic is dense enough that field-level guarantees are easy to misread
- the store serves multiple domains, which increases the risk of accidental broad regressions
- source tagging is valuable but easy to treat as incidental rather than part of contract
- public hydration identity and deep payload equality are related but distinct concepts

## Regressions To Avoid

- losing no-op detection and re-stamping timestamps on identical payloads
- skipping validation change detection when validation arrives for the same run later
- silently dropping `optimizationMetricSurface`, `datasetMeta`, `mlSignal`, tags, or report path
- weakening or removing latest-identity snapshot helpers used by sync logic
- refreshing analysis on source-only or otherwise meaningless changes

## External Research Summary

- [JSON Schema: What Is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because the store effectively enforces a normalized cross-layer payload contract
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because these payloads ultimately derive from persisted JSON artifacts
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because stable normalization/signature behavior benefits from canonicalized payload thinking
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because this store is shared workflow state, not local widget state

## Proposed Improvement Approach

1. Document the store contract by payload family:
   - current run
   - current validation
   - ML attachment subtree
   - optimization metric surface
2. Document apply-result semantics explicitly:
   - `applied`
   - `runChanged`
   - `validationChanged`
   - `sourceChanged`
3. Separate public hydration identity from deep payload-equality semantics in the documentation, because both are used today for different purposes.
4. Treat this store as shared infrastructure for multiple workflows, so changes must be synchronized across all producers.

## Checklist

- [ ] Document the normalized `currentRun` and `currentValidation` field families.
- [ ] Document source-tag semantics and latest identity snapshot semantics.
- [ ] Document no-op detection and timestamp-churn avoidance.
- [ ] Document validation-arrives-later behavior for same-run hydration.
- [ ] Document current producer flows that write this store.
- [ ] Add or tighten tests if normalization or apply-result semantics change.

## Verification Plan

- run `npm --prefix web test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm --prefix web test -- src/services/workflowArtifactsSync.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- run `npm --prefix web test -- src/services/mlSignalService.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep normalized field names compatible across existing producers
- if signatures or identity helpers change, update sync/history/validation consumers in the same change
- preserve current source tags until all callers are migrated deliberately

## Related Docs

- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/04_results/17_analysis_store_service_2026-04-08.md`
