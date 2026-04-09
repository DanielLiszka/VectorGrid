# ML Signal Score Cache Validation Plan

Last updated: 2026-04-09

## Purpose

Define the current score-cache validation contract for ML Signal so probability requirements, dataset/timeframe compatibility, artifact metadata, and validation/sweep prerequisites remain explicit and regression-resistant.

## Scope Boundaries

In scope:

- score-cache loading and compatibility checks for ML validation
- validation-context matching against the currently loaded dataset
- probability-score versus raw-margin requirements
- metadata propagation into UI summaries and saved artifacts
- direct validation and policy-sweep dependence on the same cache contract

Out of scope:

- score-cache build input CSV rules
- model-bundle inventory and bundle health semantics
- policy-parameter defaults beyond what the cache contract constrains
- Stage 3 / Stage 4 meta-analysis logic

## Current User-Facing Behavior

Users build or select a score cache in the ML Signal workspace, then run direct validation or a policy sweep against the currently loaded dataset. The frontend requires a score-cache path before invoking either flow, builds a `validation_context` from the loaded dataset path and timeframe, and then surfaces the returned score-cache summary back into the status line, stage metadata, and saved run artifact. Current UX behavior means:

- the cache is treated as a required stage boundary, not a hidden intermediate
- validation and sweep both rely on the same cache compatibility rules
- score coverage, score scope, score space, and duplicate timestamps are already surfaced in `mlSignalScoreMeta`, `mlSignalValidationMeta`, `mlSignalSweepMeta`, and artifact details
- artifact replay can restore score-cache-backed ML runs even when there is no workbook/report path

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
  - stage controls, summary display, artifact detail view, and validation/sweep affordances
- `web/src/services/mlSignalService.ts`
  - request shaping, `validation_context` construction, busy/status handling, artifact persistence, and score-summary formatting
- `web/src/stores/optimizeStore.ts`
  - persisted score-cache path and current validation defaults
- `web/src/stores/workflowArtifactsStore.ts`
  - normalized ML artifact hydration and replay state
- `src-tauri/src/commands_ml_signal.rs`
  - `validate_ml_signal_context()` and command-boundary enforcement
- `crates/app_core/src/ml_signal.rs`
  - `load_score_cache()`, `validate_score_cache_contract()`, and the actual validation/sweep entry points
- `src-tauri/src/run_artifacts.rs`
  - saved artifact detail summary and ML-signal payload round-tripping
- `web/src/services/historyService.ts`
  - replay of score-cache-backed ML validation and sweep artifacts

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- tests embedded in `crates/app_core/src/ml_signal.rs`
  - `score_cache_build_persists_probability_metadata`
  - `validation_rejects_raw_only_score_cache`
  - `score_cache_build_and_validation_smoke`
  - `score_cache_prefers_calibrated_columns_and_persists_metadata`
- artifact/history tests touching `run_artifacts.rs` and `historyService.ts`
- ML workflow benches in `crates/app_core/benches/ml_signal_workflow.rs` and `crates/app_core/benches/ml_signal_validation.rs`

## Strengths Already Present

- score-cache validation is already centralized instead of being reimplemented in multiple layers
- the command boundary enforces current dataset compatibility before expensive work starts
- the backend distinguishes probability-space caches from raw-margin-only caches explicitly
- the same contract protects both direct validation and policy sweep
- saved artifacts already persist score-cache path, manifest path, schema version, and score-summary metadata

## Weaknesses / Failure Modes

- the real contract spans frontend request shaping, Tauri context checks, Rust cache loading, and artifact replay, so it is easy to document only one layer and miss the others
- raw-margin-only caches can be built successfully but later fail validation with probability-threshold-specific errors
- two-sided validation requires short-side probability coverage; that nuance is easy to miss from the UI alone
- metadata is reshaped more than once:
  - backend score summary
  - service-layer status/meta strings
  - workflow artifact payload
  - history replay
- ML runs often have no workbook path, so any UX that assumes a report file exists can look broken even when the artifact path is valid

## Regressions To Avoid

- weakening `data_id`, score-cache version, kind, or feature-schema checks for convenience
- permitting validation against a loaded dataset whose CSV path or timeframe no longer matches the saved validation context
- silently accepting raw-margin-only caches where probability-threshold semantics are required
- hiding duplicate-timestamp or partial-coverage metadata
- changing artifact payload fields in a way that breaks replay or history detail views

## External Research Summary

- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because the repo already distinguishes calibrated probability-style scores from raw margins, and validation semantics depend on that distinction remaining explicit
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because validation context and dataset compatibility should remain time-aware rather than drift toward generic shuffled-evaluation assumptions
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the cache file and saved artifact payload behave like versioned JSON contracts
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the frontend-visible request/result contract crosses a structured command boundary and should stay serializable and shape-stable

## Proposed Improvement Approach

1. Document the score-cache validation contract as a staged gate:
   - path presence
   - loaded dataset compatibility
   - cache file compatibility
   - probability-column sufficiency
   - candidate-shape-specific requirements
2. Separate user-facing prerequisites from backend rejection rules so the UI can stay helpful without weakening the Rust contract.
3. Make explicit which metadata is authoritative in the cache file and which metadata is merely derived for display.
4. Document the exact failure split between:
   - score-cache build success
   - later validation/sweep rejection
5. Tie every sensitive invariant to its owner:
   - service
   - command boundary
   - app-core
   - artifact hydration
6. Tighten documentation around missing-report-path behavior so ML artifacts are not mistaken for incomplete runs when the artifact itself is valid.
7. Keep replay/history behavior aligned with the same cache contract instead of creating a more permissive restore path.

## Checklist

- [ ] Document `load_score_cache()` checks for version, kind, feature schema, and `data_id`.
- [ ] Document `validate_ml_signal_context()` rules for strategy kind, CSV path, and timeframe.
- [ ] Document the probability-space requirement for ML validation and sweep.
- [ ] Document long-only versus two-sided short-probability requirements.
- [ ] Document `rows_with_probability`, missing-probability-row, and duplicate-timestamp semantics.
- [ ] Document which score-summary fields are surfaced into `mlSignalScoreMeta`, `mlSignalValidationMeta`, and `mlSignalSweepMeta`.
- [ ] Document how score-cache metadata is persisted into workflow/history artifacts.
- [ ] Document that artifact replay must tolerate empty workbook/report paths for ML runs.
- [ ] Add or maintain tests for raw-only cache rejection, schema/version mismatch, and context mismatch.
- [ ] Preserve the same cache-contract behavior for both direct validation and policy sweep.

## Verification Plan

- run `npm --prefix web test -- src/services/mlSignalService.test.ts`
- run `npm --prefix web test -- src/components/MlSignalWorkspacePanel.test.ts`
- run targeted ML-signal tests in `crates/app_core/src/ml_signal.rs`
- manually validate one score cache against the matching loaded dataset
- manually replay one saved ML validation artifact through history/detail surfaces

## Rollout / Fallback / Compatibility Notes

- preserve current score-cache version and feature-schema expectations until there is a documented migration plan
- prefer clearer frontend error messages over relaxing backend compatibility checks
- keep artifact fields additive so older ML run artifacts remain inspectable
- if workbook/report export is later added for ML validation, do not make current artifact-only replay paths dependent on that report

## Related Docs

- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/08_ml/18_mlsignal_policy_params_2026-04-09.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
