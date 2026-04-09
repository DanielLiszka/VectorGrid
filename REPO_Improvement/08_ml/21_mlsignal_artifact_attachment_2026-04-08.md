# ML Signal Artifact Attachment Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the ML Signal artifact-attachment contract so non-options ML artifacts remain stage-aware, replayable, and truthfully attached across export, score-cache, validation, and policy-sweep flows.

## Scope Boundaries

In scope:

- `ml_signal` artifact sub-payload structure
- stage ownership and carry-forward behavior
- attachment persistence across workflow/history/replay flows
- compatibility checks for dataset, timeframe, and score-cache context

Out of scope:

- options ML surfaces
- redesigning model training logic itself
- inventing new ML artifact families unrelated to the current workflow

## Current User-Facing Behavior

ML Signal flows can persist artifacts during dataset export, score-cache creation, validation, and policy sweep work. The saved attachment helps replay and continue work later, but the payload is built incrementally and can inherit prior fields, which means stage semantics are not always explicit.

## Current Code Surface

- `web/src/services/mlSignalService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/validationService.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `web/src/services/analysisService.ts`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- ML workspace component tests

## Strengths Already Present

- ML artifact persistence already exists across multiple stages
- workflow artifact normalization already understands ML-related fields
- validation/report flows already carry ML attachment data forward when available
- backend commands already enforce some compatibility checks for loaded data and score caches

## Weaknesses / Failure Modes

- stage is inferred partly by heuristics and partial prior state
- carry-forward merging can preserve stale fields unintentionally
- Vue state can advance even when artifact save fails
- replay dispatch depends on stage-shape assumptions instead of one explicit attachment contract
- compatibility checks lean heavily on path/timeframe or `data_id` matches rather than a richer dataset identity model

## Regressions To Avoid

- breaking replay of existing ML Signal artifacts
- dropping stage-specific fields that current flows still rely on
- over-tightening compatibility in a way that strands valid saved work
- hiding persistence failures that users need to understand
- making standard non-ML artifact flows depend on ML-specific assumptions

## External Research Summary

- [scikit-learn cross-validation guidance](https://scikit-learn.org/1.6/modules/cross_validation.html) is relevant because ML artifact stages must preserve the temporal/data-split assumptions used to generate them.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces keeping backend compatibility and save semantics explicit where frontend workflow state can drift.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for formalizing the `ml_signal` sub-payload by stage.

## Proposed Improvement Approach

1. Define the `ml_signal` sub-payload by stage:
   - dataset export
   - score-cache build
   - validation
   - policy sweep
2. Mark each field as one of:
   - stage-owned
   - carry-forward
   - optional diagnostic metadata
3. Document every consumer of the attachment in workflow state, history replay, validation/report export, and analysis surfaces.
4. Tighten compatibility rules for dataset, timeframe, score cache, and bundle/schema assumptions.
5. Add end-to-end tests that prove attachment continuity across the full stage sequence.

## Checklist

- [ ] Define the stage-specific `ml_signal` payload contract.
- [ ] Mark carry-forward vs stage-owned vs optional fields.
- [ ] Document all replay and hydration consumers.
- [ ] Define dataset/timeframe/score-cache compatibility rules.
- [ ] Surface or log artifact-save failures more clearly.
- [ ] Add end-to-end tests for export -> score cache -> validation -> policy sweep continuity.
- [ ] Add tests for stale-field clearing when stage transitions invalidate prior data.
- [ ] Preserve replay compatibility for existing ML Signal artifacts.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run `npm test -- src/services/historyService.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run `npm test -- src/stores/workflowArtifactsStore.test.ts`
- run targeted ML workspace/service tests if payload semantics change

## Rollout / Fallback / Compatibility Notes

- keep current ML artifact replay working while the attachment contract is tightened
- prefer additive stage metadata over removing current fields abruptly
- if attachment failures remain non-fatal, make that degradation visible rather than silent

## Related Docs

- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_DATE.md`
- `REPO_Improvement/08_ml/04_ml_validation_surface_DATE.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/06_artifacts/05_meta_artifact_replay_DATE.md`
