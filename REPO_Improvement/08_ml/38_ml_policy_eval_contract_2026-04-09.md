# ML Policy Evaluation Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current ML policy-evaluation contract so scoped historical evaluation, request reuse, saved-artifact behavior, and comparison-arm semantics remain explicit and separate from both direct Stage 5 execution and in-memory ML apply.

## Scope Boundaries

In scope:

- `evaluate_ml_confidence_policy`
- policy-evaluation request defaults and normalization
- scoped historical artifact selection
- `meta_only`, `ml_only`, and `stage5_blended` comparison arms
- saved policy-evaluation artifacts and replay surfaces

Out of scope:

- current-meta in-memory ML apply
- live Stage 5 execution of one current source meta artifact
- Stage 3 holdout selection logic except where policy eval consumes saved Stage 3 artifacts
- options workflows

## Current User-Facing Behavior

The Meta workspace exposes policy evaluation as a separate offline analysis lane. Current shipped behavior means:

- policy evaluation is not run against only the current meta artifact
- it evaluates a time-ordered scoped history of saved meta artifacts
- it compares three policy arms:
  - `meta_only`
  - `ml_only`
  - `stage5_blended`
- it produces a saved JSON evaluation artifact with replay support
- it reuses the same Stage 5-style thresholds and weights for automatic request building, even though the workflow is analytically different

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - policy-eval controls, preview, and saved policy-eval lane
- `web/src/services/metaService.ts`
  - auto-building the policy-eval request
  - command invocation
  - preview rendering
  - saved-entry management
- `src-tauri/src/commands_meta.rs`
  - policy-eval request/result types
  - request normalization
  - scoped artifact ordering/filtering
  - historical prefix training/evaluation loop
  - saved artifact writing
- `src-tauri/src/run_artifacts.rs`
  - policy-eval artifact listing, summary extraction, and replay hydration
- `src-tauri/src/commands_artifacts.rs`
  - replay/listing commands used by the frontend saved policy-eval surface

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- tests embedded in `src-tauri/src/commands_meta.rs`
- tests embedded in `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- policy evaluation is already a distinct command and artifact family instead of being overloaded onto Stage 5
- the evaluation loop uses ordered scoped history rather than evaluating only one point-in-time artifact
- the three-arm comparison is explicit in the result shape
- saved policy-evaluation artifacts are already replayable and manageable from the UI
- backend tests already compare the prepared-row path to a raw reference implementation

## Weaknesses / Failure Modes

- the policy-eval request reuses Stage 5-style thresholds/weights, which makes the contract easy to misdescribe as “just Stage 5 again”
- the saved artifact kind and file-prefix naming are not perfectly aligned, creating a subtle naming mismatch
- many artifacts can be skipped for legitimate reasons:
  - no history prefix
  - training failure
  - unhealthy model
  - no labels
- without documentation, “best policy” can be misread as a production recommendation rather than an offline historical comparison result
- there is no workbook/report parallel to the Stage 3 meta workbook surface

## Regressions To Avoid

- collapsing policy evaluation into Stage 5 or current-meta apply in docs or code structure
- changing request defaults without updating auto-builder and backend normalization together
- weakening the historical prefix-training discipline
- hiding skip counts or warning categories that explain why some scoped artifacts were not comparable
- breaking policy-eval artifact replay while cleaning up naming drift

## External Research Summary

- [scikit-learn Model Selection and Evaluation](https://scikit-learn.org/stable/model_selection.html)
  - relevant because policy evaluation is fundamentally an offline comparative evaluation workflow
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because the repo’s policy-eval loop respects time order and prefix history rather than shuffled evaluation
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because policy-comparison artifacts should remain explainable about what was evaluated and how
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the policy-eval artifact is a persisted JSON contract with arm-level metrics and warnings

## Proposed Improvement Approach

1. Document policy evaluation as a distinct offline contract, not a synonym for Stage 5.
2. Record the historical prefix-training loop explicitly so future contributors do not accidentally turn it into same-artifact evaluation.
3. Separate request reuse from semantic reuse:
   - the auto-builder reuses Stage 5 controls
   - the evaluation workflow is still distinct
4. Preserve skip-category visibility because artifact comparability is a major part of understanding policy-eval results.
5. Document the current artifact-kind/prefix naming mismatch so cleanup work does not accidentally break saved-replay behavior.
6. Keep “best policy” framed as an offline comparison outcome, not a direct live-execution recommendation.
7. Tie the policy-eval doc closely to replay/saved-entry management because persistence is part of the contract.

## Checklist

- [ ] Document all major request fields and current default values for policy evaluation.
- [ ] Document that the auto-built request currently reuses Stage 5 controls.
- [ ] Document the scoped historical artifact ordering and prefix-training loop.
- [ ] Document the three comparison arms and their intended meaning.
- [ ] Document skip categories and warning surfaces.
- [ ] Document the current saved artifact kind and filename-prefix behavior.
- [ ] Document replay/list/delete behavior for saved policy-eval artifacts.
- [ ] Preserve tests for request normalization, historical evaluation, and replay summary extraction.
- [ ] Keep docs explicit that policy eval has no workbook/report surface today.
- [ ] Treat any future “best policy” productization as a new contract, not an implicit property of current offline evaluation.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted policy-eval tests in `src-tauri/src/commands_meta.rs`
- run targeted policy-eval artifact tests in `src-tauri/src/run_artifacts.rs`
- manually run one policy evaluation and replay the saved artifact from the UI

## Rollout / Fallback / Compatibility Notes

- preserve current replayable JSON artifact behavior while naming cleanup is still incomplete
- if policy evaluation later gains workbook/report output, add it explicitly without implying it already exists in current artifacts
- keep skip counters and warning families stable so historical comparisons remain interpretable
- prefer additive artifact metadata over changing the meaning of current policy-arm metrics

## Related Docs

- `REPO_Improvement/08_ml/34_ml_conf_apply_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/36_stage4_selection_logic_2026-04-09.md`
- `REPO_Improvement/08_ml/37_stage5_execution_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/07_policy_eval_artifact_replay_2026-04-09.md`
- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
