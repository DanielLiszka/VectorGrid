# ML Signal Strategy Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for `MlSignal` as a strategy-local non-options surface with its own staged dataset-export, score-cache, validation, and policy-sweep workflow that should not be confused with the standard optimization lane.

## Scope Boundaries

In scope:

- ML Signal stage ownership
- dataset export, score cache, validation, and policy sweep as one strategy workflow
- model-bundle and artifact expectations
- frontend/backend contract boundaries for ML Signal

Out of scope:

- Stage 4 / Stage 5 meta-selection ranking beyond direct ML Signal dependencies
- options ML surfaces
- generic “AI” expansion beyond the current shipped workflow

## Current User-Facing Behavior

ML Signal is exposed through its own shell/workflow mode, not as a normal optimization strategy. Users work through four shipped stages:

- dataset export
- score-cache build
- validation
- Stage 4 policy sweep

Those stages are driven from the dedicated ML workspace, persist through the normal run-artifact channel with an `ml_signal` payload, reroute the normal Run/Validate controls while `ml-signal` mode is active, and intentionally do not populate standard optimizer analysis views such as Top/Heatmap/Pareto.

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- ML workspace component tests
- tests in `crates/app_core/src/ml_signal.rs`
- artifact/history tests that cover ML attachment paths

## Strengths Already Present

- ML Signal already has a dedicated workflow instead of hiding inside generic optimize forms
- stage boundaries already exist in both the UI and backend command layer
- artifact persistence is already integrated with the rest of the app
- model-bundle status and compatibility checks already exist

## Weaknesses / Failure Modes

- strategy ownership is split across an ML-specific workspace, optimize-store fields, artifacts, and backend commands
- the workflow is staged, but some contracts are implicit rather than explicitly documented by stage
- users can mistake ML Signal for “just another optimization result” even though its dataflow is materially different
- artifact and status semantics are rich but spread across multiple layers
- history rerun and replay behavior is stage-shaped, which makes stale docs easy if the stage contract is not written down clearly

## Regressions To Avoid

- collapsing ML Signal into the standard optimize workflow in a way that hides stage prerequisites
- weakening dataset/timeframe compatibility checks
- dropping artifact linkage or model-bundle visibility
- making stage transitions less clear while trying to simplify the surface
- introducing feature creep that is really a new ML product lane, not an improvement

## External Research Summary

- [scikit-learn cross-validation guidance](https://scikit-learn.org/1.6/modules/cross_validation.html) is relevant because ML Signal is a staged, time-aware validation workflow rather than a generic prediction demo.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because dataset export and validation need explicit temporal-split assumptions.
- [scikit-learn probability calibration](https://scikit-learn.org/stable/modules/calibration.html) is relevant because downstream validation and sweep semantics assume score/probability meaning rather than arbitrary model outputs.
- [CatBoost prediction docs](https://catboost.ai/docs/en/features/prediction) is relevant because imported model-score semantics and probability/raw-margin expectations can differ depending on upstream model tooling.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for keeping stage payloads and artifacts contractual.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because each ML stage crosses an explicit command boundary.

## Proposed Improvement Approach

1. Document ML Signal as one strategy-local workflow with named stages, not as a loose collection of helper commands.
2. Keep stage ownership explicit:
   - dataset export
   - score cache build
   - validation
   - policy sweep
3. Document why standard optimizer analysis views intentionally stay empty for ML Signal artifacts so that “missing analysis” is not mistaken for a bug.
4. Keep artifact attachment, model-bundle status, and stage-specific status messaging tied to this strategy-local contract.
5. Preserve separation from the standard optimize/validate lane while improving cross-workflow artifact reuse and replay clarity.

## Checklist

- [ ] Document the full ML Signal stage sequence and required prerequisites.
- [ ] Document which state belongs to the strategy-local workflow vs shared artifact infrastructure.
- [ ] Document why optimizer analysis views intentionally exclude ML artifacts.
- [ ] Clarify model-bundle, score-cache, and artifact expectations by stage.
- [ ] Document history rerun/replay rules by ML stage.
- [ ] Keep dataset/timeframe compatibility and stage guards explicit.
- [ ] Ensure ML-specific status and artifact surfaces remain distinct from standard optimization copy.
- [ ] Add or tighten stage-specific tests if workflow semantics change.
- [ ] Preserve strategy-local discoverability in UI, services, and backend commands.
- [ ] Keep true new-feature ideas labeled out of improvement scope.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run ML workspace component tests
- run targeted Rust ML-signal tests in `ta_app_core`
- manually review one end-to-end ML Signal run if stage semantics change

## Rollout / Fallback / Compatibility Notes

- preserve the current staged ML Signal workflow
- improve naming and stage clarity without removing existing artifact/report hooks
- if shared infrastructure is reused more aggressively later, keep ML Signal’s distinct workflow contract visible

## Related Docs

- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_DATE.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_DATE.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
