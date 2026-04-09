# ML Signal Frontend Service Plan

Last updated: 2026-04-09

## Purpose

Define the current ML Signal frontend-service contract so export, score-cache, validation, sweep, bundle status, artifact persistence, and UI status ownership remain explicit and separate from the Meta service family.

## Scope Boundaries

In scope:

- `mlSignalService.ts`
- ML Signal export/build/validate/sweep orchestration
- bundle status and optional-bundle behavior
- ML Signal artifact persistence and replay-adjacent state updates

Out of scope:

- Meta workspace service behavior
- backend ML Signal algorithm details except where the frontend contract depends on them
- options workflows

## Current User-Facing Behavior

The ML Signal workflow is orchestrated by its own frontend service. Current behavior means:

- the service is responsible for dataset export, score-cache build, validation, sweep, and bundle inspection
- bundle selection is optional for score-cache-backed validation
- the service sets busy/status text, persists artifacts, and updates workflow state
- ML Signal is not part of the current batch/meta campaign surface

## Current Code Surface

- `web/src/services/mlSignalService.ts`
  - core frontend orchestration for ML Signal
- `web/src/components/MlSignalWorkspacePanel.vue`
  - main ML Signal UI
- `web/src/components/MlSignalShellPanel.vue`
  - shell-level ML Signal surface
- `web/src/stores/optimizeStore.ts`
  - ML Signal state/defaults
- `web/src/stores/workflowArtifactsStore.ts`
  - artifact persistence and hydration
- `web/src/services/historyService.ts`
  - replay/inspection of persisted ML Signal artifacts
- `src-tauri/src/commands_ml_signal.rs`
  - backend command boundary for ML Signal flows

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/services/historyService.test.ts`
- tests embedded in `src-tauri/src/commands_ml_signal.rs`

## Strengths Already Present

- ML Signal has its own service boundary instead of being buried inside the Meta service
- artifact persistence is already part of the service contract, not an afterthought
- bundle-optional behavior is explicit rather than hidden behind failing validation calls
- store defaults for ML Signal are already centralized

## Weaknesses / Failure Modes

- the service does more than transport, which makes it easy to under-document its artifact and status responsibilities
- ML Signal status/meta shaping is spread across busy-state helpers, optimize store patches, and artifact persistence
- because it lives beside the larger optimization workspace, it can be misdescribed as just another optimization mode instead of its own service family
- there is no campaign integration, so attempts to generalize Meta campaign docs onto ML Signal will be wrong

## Regressions To Avoid

- collapsing ML Signal service docs into Meta service docs
- documenting bundle selection as mandatory when the current service makes it optional in some flows
- weakening artifact persistence or replay-related state updates while refactoring the UI
- hiding the service’s ownership of busy/status presentation

## External Research Summary

- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because ML Signal validation and sweep semantics rely on probability-style thresholds and score interpretation
- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because bundle/score-cache surfaces benefit from explicit model/input/output contracts
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because ML Signal artifact payloads, manifests, and summaries are structured JSON contracts
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because ML Signal is command-driven from the frontend and not purely local state manipulation

## Proposed Improvement Approach

1. Document ML Signal as its own frontend service family with its own artifact/status contract.
2. Make bundle-optional behavior explicit in the service doc.
3. Record the service’s responsibilities beyond invoke calls:
   - status/busy handling
   - artifact persistence
   - workflow-state updates
4. Keep ML Signal clearly outside the Meta campaign/service story.
5. Tie service changes to artifact/history/store tests because that is where its real contract shows up.

## Checklist

- [ ] Document all major ML Signal frontend-service responsibilities.
- [ ] Document current bundle-optional behavior and where it applies.
- [ ] Document the service’s artifact persistence and workflow-state updates.
- [ ] Document busy/status ownership in the service layer.
- [ ] Preserve tests for export, score-cache build, validation, sweep, and artifact persistence.
- [ ] Preserve tests for history/replay surfaces that depend on persisted ML Signal artifacts.
- [ ] Keep ML Signal docs separate from the Meta service/campaign family.

## Verification Plan

- run `npm --prefix web test -- src/services/mlSignalService.test.ts`
- run `npm --prefix web test -- src/components/MlSignalWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run targeted ML Signal tests in `src-tauri/src/commands_ml_signal.rs`

## Rollout / Fallback / Compatibility Notes

- preserve ML Signal’s separate service boundary even if adjacent workflows are reorganized
- keep artifact persistence compatible with current history/replay behavior
- prefer additive bundle/status metadata over reinterpreting existing fields

## Related Docs

- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/17_mlsignal_score_cache_validation_2026-04-09.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_DATE.md`
