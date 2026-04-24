# Meta Validation and AI/ML Validation Improvement Plan

## Intent

Improve the decision quality, provenance, and deployability of the conventional backtest optimization workflow after a strategy has been optimized and validated.

This plan focuses on these current product surfaces:

1. Conventional fixed-candidate validation after optimization.
2. Meta selection over saved optimization and validation artifacts.
3. Stage 5 ML-confidence reranking over saved Meta artifacts.
4. Standalone `MlSignal` validation and policy sweep, where it shares validation-governance concerns with the conventional workflow.

Last updated: 2026-04-24.

## Status

This is an implementation plan. No items in this document should be treated as already implemented unless they are explicitly marked complete.

## Current State Snapshot

The current workflow already has a strong validation spine:

- Frontend validation request shaping lives in `web/src/services/validationRequestBuilder.ts`.
- Frontend validation execution, artifact persistence, report export, and status summaries live in `web/src/services/validationService.ts`.
- Tauri validation command boundaries live in `src-tauri/src/commands_validation.rs`.
- Shared validation orchestration lives in `crates/app_core/src/validation_core.rs`.
- Shared validation post-processing, probability metrics, hard gates, CSCV/PBO, run verdicts, and semantics provenance live in `crates/app_core/src/validation_pipeline.rs`.
- Strategy-local validation execution lives primarily in `crates/app_core/src/double_ma_validate.rs` and `crates/app_core/src/composite_validate.rs`.
- Meta request shaping lives in `web/src/services/metaRequestBuilder.ts`.
- Stage 5 request shaping lives in `web/src/services/metaStage4RequestBuilder.ts`.
- Meta/Stage 5 frontend orchestration lives in `web/src/services/metaService.ts` and `web/src/components/MetaWorkspacePanel.vue`.
- Meta/Stage 5 backend orchestration lives in `src-tauri/src/commands_meta.rs`.
- Meta grouping and scoring live in `crates/app_core/src/meta_select.rs`.
- ML confidence training, compatibility keys, scoring, calibration, and health checks live in `crates/app_core/src/ml_confidence.rs`.
- Standalone `MlSignal` orchestration lives in `web/src/services/mlSignalService.ts`, `src-tauri/src/commands_ml_signal.rs`, and `crates/app_core/src/ml_signal.rs`.

Key current strengths:

- Normal validation enforces dataset, timeframe, and strategy context at the Tauri boundary for conventional validation commands.
- Validation request building records admission provenance, candidate dedupe, and selected admission policy.
- Validation supports walk-forward, reopt walk-forward, SPA, perturbation, trade bootstrap, CSCV/PBO, hard gates, cross-ticker checks, and report/artifact persistence.
- Meta selection aggregates saved artifacts by strategy/timeframe/tag, groups candidates, applies robust gates, and optionally runs holdout evaluation.
- Stage 5 ML confidence excludes the active source meta artifact from auto-training to avoid direct self-scoring leakage.
- ML confidence supports aggregate holdout labels, per-dataset holdout-row labels, compatibility keys, logistic/GBDT-stump models, calibration, and AUC/Brier health.
- `MlSignal` score-cache validation rejects raw-only score caches for probability-threshold validation, rejects duplicate timestamps, tracks score scope, and persists score-cache provenance.

Key current limitations this plan targets:

- Cross-ticker validation has lightweight and preserved-finalist modes that intentionally strip parts of the full validation stack.
- Meta holdout train/holdout separation is tag/artifact driven, not a sealed cohort manifest with hashes and explicit disjointness evidence.
- Partial Meta holdout coverage can be hard to reason about because evaluated and unevaluated groups are mixed in the same artifact family.
- Stage 5 training uses scoped history except the source artifact, but it does not yet offer a strict chronological training mode.
- Stage 5 policy evaluation currently centers on precision@k, which is too narrow for deciding whether to trust ML-assisted reranking.
- `MlSignal` validation has useful scope warnings, but there is no explicit deployable-validation gate that distinguishes research evidence from production-ready evidence.

## Goals

1. Make validation evidence explicit enough that users and future automation can tell what was actually proven.
2. Preserve the current fast workflow while adding stronger, opt-in evidence modes.
3. Add sealed cohort provenance for Meta and Stage 5 ML-confidence workflows.
4. Make partial holdout evaluation visible, auditable, and safe for downstream ML training.
5. Add chronological Stage 5 training and evaluation modes to reduce historical leakage.
6. Expand ML policy evaluation from precision-only comparison to decision-grade model governance.
7. Add deployable-readiness gates for `MlSignal` validation without weakening its research workflow.
8. Keep CPU-only builds, existing artifacts, and existing advanced controls working.

## Non-Goals

- Do not remove current lightweight validation modes.
- Do not make CUDA mandatory for validation, Meta, Stage 5, or `MlSignal`.
- Do not force options-expression validation into the conventional validation contract.
- Do not make Stage 5 ML confidence a hidden part of normal validation. It remains a separate advanced analysis stage.
- Do not treat `MlSignal` as part of the conventional optimizer shortlist. It remains a separate workflow with its own dataset export and score-cache stages.
- Do not make old saved artifacts unreadable. Add schema versions and compatibility fallback where needed.

## Target Workflow

The improved conventional workflow should become:

1. Optimize a strategy.
2. Run validation with explicit evidence depth:
   - basic fixed-candidate validation,
   - full local validation stack,
   - cross-dataset lightweight evidence,
   - cross-dataset preserved-finalist evidence,
   - cross-dataset full-stack evidence.
3. Persist validation artifacts with a normalized evidence profile.
4. Run Meta selection over a sealed train cohort.
5. Optionally run Meta holdout over a sealed holdout cohort.
6. Persist Meta artifacts with cohort manifests, group-level holdout coverage, and train/holdout disjointness evidence.
7. Train/apply Stage 5 ML confidence using an explicit history policy:
   - current default history policy,
   - source-excluded policy,
   - chronological-before-source policy.
8. Persist Stage 5 artifacts with the exact training artifact list, model health evidence, scoring coverage, ranking weights, and fallback reason if used.
9. Optionally run policy evaluation with richer governance metrics.
10. Promote a result only if its validation evidence profile and ML governance status meet the chosen deployment policy.

## Evidence Vocabulary

Add a shared evidence vocabulary that the backend, artifacts, reports, and UI can reuse.

### Evidence Depth

Recommended values:

- `local_basic`: fixed-candidate validation on the active dataset.
- `local_full_stack`: active dataset validation with all selected local modules.
- `cross_lightweight`: cross-dataset reruns with reduced module coverage.
- `cross_preserved_finalists`: cross-dataset reruns over bounded finalists with verdict-relevant gates preserved.
- `cross_full_stack`: cross-dataset reruns preserving the full requested validation stack.
- `meta_train_only`: Meta selection without holdout evaluation.
- `meta_holdout_partial`: Meta selection with holdout for a subset of groups.
- `meta_holdout_complete`: Meta selection with holdout for all retained groups.
- `stage5_source_excluded`: Stage 5 source artifact excluded from training.
- `stage5_chronological`: Stage 5 trained only on artifacts older than the source artifact.
- `ml_signal_research`: `MlSignal` validation that is useful for research but not deployable by policy.
- `ml_signal_deployable`: `MlSignal` validation that satisfies deployable-readiness gates.

### Evidence Warnings

Recommended warning flags:

- `cross_modules_stripped`
- `cross_full_stack_unavailable`
- `holdout_cohort_manifest_missing`
- `holdout_partial_group_coverage`
- `holdout_dataset_hash_missing`
- `train_holdout_overlap_detected`
- `stage5_training_manifest_missing`
- `stage5_training_uses_future_artifacts`
- `stage5_fallback_to_meta`
- `ml_policy_eval_precision_only`
- `ml_policy_eval_low_sample_count`
- `ml_model_unhealthy`
- `ml_model_stale`
- `ml_signal_score_scope_research_only`
- `ml_signal_score_coverage_low`
- `ml_signal_context_missing`

### Promotion States

Recommended promotion states:

- `research_only`: useful for exploration but not sufficient for deployment.
- `needs_review`: enough evidence to inspect, but one or more warnings remain.
- `validated`: validation evidence supports the candidate under the selected policy.
- `deployable_candidate`: validation and ML governance gates pass.
- `blocked`: hard evidence problem, such as cohort overlap or invalid score cache.

## Workstream 1: Sealed Meta Cohort Manifest

### Problem

Meta selection currently separates train and holdout artifacts mainly through request filters and artifact tags. This is flexible, but it does not produce a durable manifest proving exactly which artifacts were used, what their dataset identities were, and whether train and holdout cohorts were disjoint.

### Target

Every saved Meta artifact should include an optional `cohort_manifest` with train and holdout artifact identities, dataset identities, source fingerprints, request filters, and disjointness checks.

### Proposed Data Contract

Add a serializable structure in `crates/app_core` or `src-tauri` depending on ownership:

```rust
pub struct MetaCohortManifest {
    pub version: u32,
    pub strategy_kind: String,
    pub timeframe: Option<String>,
    pub required_tag: Option<String>,
    pub holdout_tag: Option<String>,
    pub train_artifacts: Vec<MetaCohortArtifactRef>,
    pub holdout_artifacts: Vec<MetaCohortArtifactRef>,
    pub disjointness: MetaCohortDisjointness,
    pub warnings: Vec<String>,
}

pub struct MetaCohortArtifactRef {
    pub path: String,
    pub artifact_hash: Option<String>,
    pub ts: Option<String>,
    pub run_id: Option<String>,
    pub strategy_kind: Option<String>,
    pub timeframe: Option<String>,
    pub csv_path: Option<String>,
    pub dataset_id: Option<String>,
    pub dataset_hash: Option<String>,
    pub source_content_fingerprint: Option<String>,
    pub tags: Vec<String>,
}

pub struct MetaCohortDisjointness {
    pub checked: bool,
    pub train_count: usize,
    pub holdout_count: usize,
    pub overlapping_dataset_keys: Vec<String>,
    pub overlapping_artifact_hashes: Vec<String>,
    pub passed: bool,
}
```

### Implementation Notes

- Build the manifest in `src-tauri/src/commands_meta.rs` after artifact scan and filtering, before `meta_select_from_artifacts`.
- Reuse existing artifact reading and filtering logic where possible.
- Prefer existing dataset metadata fields from saved run artifacts:
  - `dataset_meta.csv_path`
  - `dataset_meta.timeframe`
  - `dataset_meta.source_content_fingerprint`
  - `dataset_meta.symbol` or label fields
  - run artifact path and timestamp
- If a hash is expensive, compute it only for artifact JSON first. Dataset file hashes can be optional in the first phase if a source fingerprint exists.
- Treat overlap as a hard warning first; make it a hard error only after UI and migration behavior are stable.

### UI and Report Changes

- Add a Meta summary row for:
  - train artifacts,
  - holdout artifacts,
  - disjointness passed/failed,
  - missing fingerprint count.
- Add warning copy when disjointness cannot be fully checked.
- Add XLSX report columns or a manifest worksheet in the Meta report.

### Tests

- Unit test manifest extraction from saved artifact JSON.
- Unit test train/holdout overlap detection by CSV path, dataset id, artifact hash, and source fingerprint.
- Frontend test that Meta panel surfaces manifest/disjointness status.
- Output parity test for Meta report shape if XLSX changes.

## Workstream 2: Full Cross-Dataset Validation Mode

### Problem

Cross-ticker validation currently has modes that intentionally omit expensive modules. This is valid for fast evidence, but it leaves a gap between "quick cross evidence" and "same full validation stack across datasets."

### Target

Add an opt-in `cross_full_stack` mode that reruns all requested validation modules on each cross dataset for the retained candidates.

### Proposed Frontend Contract

Extend `ValidationRequestPayload.cross_ticker.mode`:

```ts
mode: "lightweight" | "preserved_finalists" | "full_stack"
```

Add request-builder summaries:

- `lightweight`: quick cross-dataset evidence; many modules omitted.
- `preserved_finalists`: bounded finalists; CSCV/hard gates preserved; selected auxiliary modules omitted.
- `full_stack`: bounded finalists or configured cap; all requested modules rerun on each cross dataset.

### Proposed Backend Contract

Extend `CrossTickerMode` in app-core if it already owns the enum. Then update:

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- report builders that display cross mode.

For full-stack mode:

- Preserve `walk_forward`.
- Preserve `reopt_walk_forward`.
- Preserve `spa`.
- Preserve `perturb`.
- Preserve `trade_bootstrap`.
- Preserve `cscv_pbo`.
- Preserve hard gates.
- Preserve validation engine and iterative config.
- Preserve backend choice and route metadata.

### Runtime Controls

Full-stack cross validation can be expensive. Add safety controls:

- Default cap lower than lightweight mode, such as `2` or `3` datasets.
- Explicit warning when user requests high dataset count.
- Progress phase labels that distinguish local validation from cross full-stack validation.
- Cancellation checks before every dataset and before expensive module groups.

### Artifact Changes

Add fields under validation provenance:

```json
{
  "cross_evidence_depth": "cross_full_stack",
  "cross_requested_modules": ["walk_forward", "reopt_walk_forward", "spa", "perturb", "trade_bootstrap", "cscv_pbo"],
  "cross_executed_modules": ["walk_forward", "reopt_walk_forward", "spa", "perturb", "trade_bootstrap", "cscv_pbo"],
  "cross_omitted_modules": [],
  "cross_dataset_cap": 3
}
```

### Tests

- Request-builder test for `full_stack` payload and summary copy.
- Backend unit tests that full-stack cross request preserves each module.
- Regression test that lightweight and preserved-finalist modes keep current stripping behavior.
- Validation artifact/report tests for new provenance fields.
- Performance smoke using a small cross dataset set.

## Workstream 3: First-Class Partial Holdout Coverage

### Problem

Meta holdout can evaluate all groups or only a subset of groups. Partial evaluation is useful for cost control, but downstream ranking and ML training need a stronger indication of which groups have holdout labels and which do not.

### Target

Persist group-level holdout coverage and make partial coverage visible in Meta, Stage 5, and policy evaluation.

### Proposed Data Contract

Add to `MetaSelectionResult`:

```rust
pub struct MetaHoldoutCoverageSummary {
    pub groups_total: usize,
    pub groups_evaluated: usize,
    pub groups_unevaluated: usize,
    pub datasets_total: usize,
    pub datasets_skipped: usize,
    pub coverage_mode: String, // none, complete, partial
    pub selection_policy: Option<String>,
}
```

Add to each `MetaCandidateAggregate`:

```rust
pub struct MetaGroupHoldoutCoverage {
    pub evaluated: bool,
    pub dataset_rows: usize,
    pub missing_reason: Option<String>,
}
```

If adding a nested struct is too disruptive initially, use artifact JSON fields:

- `holdout_coverage.evaluated`
- `holdout_coverage.dataset_rows`
- `holdout_coverage.missing_reason`

### Ranking Policy

Keep current ranking behavior initially, but make it explicit:

- Evaluated groups rank ahead of unevaluated groups only when holdout mode is active.
- Unevaluated groups must carry `holdout_coverage.evaluated=false`.
- Stage 5 and policy evaluation must not treat unevaluated groups as negative holdout labels.
- ML confidence training should count and report skipped unevaluated groups.

### UI and Report Changes

- Add Meta summary chip: `holdout partial: X/Y groups`.
- Add Meta table column: `H coverage`.
- Add warning when policy evaluation or Stage 5 sees partial holdout coverage.
- Add report worksheet or columns for holdout coverage state.

### Tests

- Unit test complete holdout coverage.
- Unit test partial holdout coverage.
- Unit test unevaluated groups do not become negative ML labels.
- Frontend tests for summary and warning text.
- Meta XLSX output parity if report fields are added.

## Workstream 4: Chronological Stage 5 Training

### Problem

Stage 5 already excludes the active source artifact from auto-training, which prevents direct source leakage. However, for historical replay or offline policy evaluation, a stricter mode should train only from artifacts older than the source artifact.

### Target

Add an explicit training history policy:

```rust
pub enum Stage5TrainingHistoryPolicy {
    SourceExcluded,
    ChronologicalBeforeSource,
}
```

The default can remain `SourceExcluded` for backward compatibility, but the UI should expose `Chronological` as the safer audit mode.

### Request Contract

Extend `MetaStage5Request`:

```rust
pub training_history_policy: Option<Stage5TrainingHistoryPolicy>,
```

Frontend builder field:

```ts
training_history_policy: "source_excluded" | "chronological_before_source"
```

### Artifact Contract

Persist:

```json
{
  "training_history_policy": "chronological_before_source",
  "training_artifacts": [
    {
      "path": "...",
      "artifact_hash": "...",
      "ts": "..."
    }
  ],
  "excluded_artifacts": [
    {
      "path": "...",
      "reason": "source_or_future"
    }
  ],
  "training_artifact_count": 12,
  "future_artifact_count": 3
}
```

### Model Metadata

Add compatible metadata to `MlConfidenceModelMeta`:

- `training_history_policy`
- `training_artifact_count`
- `training_artifact_hashes` or a manifest hash
- `latest_training_artifact_ts`
- `source_artifact_ts` when trained for Stage 5

If model files are intended to stay reusable globally, keep per-run training history in the Stage 5 artifact rather than mutating the stable model contract too aggressively.

### UI Changes

- Add Stage 5 control: `Training history` with options:
  - `Source excluded`
  - `Chronological`
- Surface warnings:
  - no older artifacts available,
  - insufficient class balance,
  - future artifacts excluded,
  - fallback to existing model.

### Tests

- Unit test source artifact excluded.
- Unit test chronological mode excludes source and newer artifacts.
- Unit test chronological mode errors or falls back clearly when no training rows remain.
- Frontend request-builder test for training policy.
- Saved Stage 5 replay test for manifest fields.

## Workstream 5: Expanded Stage 5 Policy Evaluation

### Problem

Policy evaluation currently compares `meta_only`, `ml_only`, and `stage5_blended` using precision@k. Precision@k is useful, but it is not enough to decide whether ML-assisted ranking is better, stable, calibrated, or safe.

### Target

Expand policy evaluation into a governance report while preserving the current compact summary.

### Additional Metrics

Add to `MlConfidencePolicyArmMetrics`:

- `recall_at_k`
- `selected_rate`
- `lift_vs_base_rate`
- `mean_label`
- `positive_rate`
- `utility_at_k`
- `max_dd_guard_pass_rate`
- `coverage`
- `evaluated_groups`
- `confidence_interval_low`
- `confidence_interval_high`

Add to result-level comparison:

- `base_positive_rate`
- `paired_artifacts_evaluated`
- `stage5_vs_meta_lift`
- `stage5_vs_meta_lift_ci`
- `stage5_vs_ml_lift`
- `stage5_vs_ml_lift_ci`
- `calibration_brier`
- `calibration_ece`
- `regime_breakdown`
- `timeframe_breakdown`
- `sample_size_warning`

### Statistical Method

Start pragmatic:

- Use Wilson interval for precision@k or bootstrap over evaluated artifacts.
- Keep calculations deterministic with request seed if bootstrapped.
- Persist enough counts that future reports can recompute summaries.

### Utility Function

Default utility:

```text
utility = label_positive
          + alpha * holdout_score_p10
          - beta * holdout_max_dd_p90
```

Initial constants can be conservative and documented in the request:

- `alpha = 1.0`
- `beta = 1.0`

Make utility optional in the first pass if it risks too much scope.

### UI Changes

- Keep existing compact `best_policy` line.
- Add expandable policy detail:
  - precision@k with interval,
  - lift vs meta,
  - calibration status,
  - sample size,
  - warnings.
- Show a clear "policy evaluation is insufficient" state when sample count is too low.

### Tests

- Unit test metric calculations on fixed labels.
- Unit test interval determinism.
- Unit test low-sample warning.
- Unit test policy comparison still preserves existing `best_policy` behavior for old fields.
- Frontend test for expanded preview.

## Workstream 6: ML Confidence Training Governance

### Problem

ML confidence already has health gates and compatibility keys, but training history and label quality need to be more visible to users and downstream automation.

### Target

Add a model governance surface that explains:

- what labels were used,
- whether labels came from aggregate holdout or per-dataset holdout rows,
- whether the history contract is compatible,
- how many artifacts and rows were skipped,
- whether calibration helped,
- whether health gates are based on CV or train fallback.

### Backend Changes

Extend training metrics or status response:

- `label_target_resolved`
- `labeled_artifacts`
- `labeled_rows`
- `skipped_incompatible_artifacts`
- `skipped_missing_holdout`
- `skipped_partial_holdout`
- `cv_group_count`
- `cv_available`
- `health_evidence_source` (`cv`, `train_fallback`, `mixed`)
- `calibration_retained`
- `calibration_brier_delta`

### UI Changes

Add a model governance block to the ML Confidence panel:

- target: aggregate vs per-dataset,
- rows and class balance,
- CV evidence source,
- compatibility status,
- health verdict,
- stale/retrain reason.

### Tests

- Unit test status distinguishes stale, unhealthy, and incompatible.
- Unit test status reports per-dataset target when `holdout_rows` are available.
- Frontend test for governance summaries.

## Workstream 7: `MlSignal` Deployable Validation Gate

### Problem

`MlSignal` is a separate workflow, but users need to know whether a validation result is only research evidence or is strong enough to consider for deployment.

### Target

Add a deployable validation gate for `MlSignal` that is stricter than the current validation run verdict.

### Proposed Gate Inputs

Required for `deployable_candidate`:

- `validation_context` present and matches loaded dataset when the frontend initiates validation.
- `score_space = probability`.
- `duplicate_timestamps = 0`.
- score cache scope is `walkforward_test` or another explicitly deployable scope.
- coverage ratio above threshold, for example `>= 0.95` for full-dataset style validation.
- no calibration-slice-only score scope for deployment.
- feature schema version/hash present.
- source bundle manifest hash present for native bundle scoring.
- min PSR/DSR gates pass where configured.
- no boundary warning above configured threshold.

### Proposed Data Contract

Add to `MlSignalValidationResult`:

```rust
pub struct MlSignalDeployability {
    pub state: String, // research_only, needs_review, deployable_candidate, blocked
    pub gates_passed: Vec<String>,
    pub gates_failed: Vec<String>,
    pub warnings: Vec<String>,
}
```

If this is too large for the first pass, add a generic `deployability` JSON object and stabilize it later.

### UI Changes

- Add a deployability line to ML validation and policy sweep summaries.
- Distinguish:
  - `Research validation`
  - `Needs review`
  - `Deployable candidate`
  - `Blocked`
- Do not block research runs from completing.

### Tests

- Unit test deployable gate passes with walk-forward test scope and full contract.
- Unit test calibration-slice scope produces research-only state.
- Unit test missing validation context produces needs-review or blocked depending on policy.
- Frontend test for status text.

## Workstream 8: Artifact and Report Compatibility

### Artifact Versioning

Do not silently mutate old artifact meaning. Add versioned optional fields:

- Meta artifacts:
  - `cohort_manifest`
  - `holdout_coverage`
  - `evidence_depth`
- Stage 5 artifacts:
  - `training_history_policy`
  - `training_manifest`
  - `policy_governance`
- Validation artifacts:
  - `cross_evidence_depth`
  - `cross_requested_modules`
  - `cross_executed_modules`
  - `cross_omitted_modules`
- `MlSignal` artifacts:
  - `deployability`
  - `score_scope_policy`

Old artifacts should load with:

- `manifest_status = legacy_missing`
- `evidence_depth = unknown_legacy`
- warnings in UI, not load failure.

### Report Changes

Recommended report additions:

- Validation workbook:
  - cross evidence depth,
  - module coverage,
  - omitted module list,
  - deployability state if `MlSignal`.
- Meta workbook:
  - cohort manifest summary,
  - train/holdout disjointness,
  - holdout group coverage,
  - ML label availability.
- Stage 5 JSON replay:
  - source artifact,
  - training artifact count,
  - training policy,
  - model health,
  - fallback reason.

## Workstream 9: Frontend Product Clarity

### Current Risk

The UI already explains many contracts, but advanced users still need stronger evidence labels to avoid overreading lightweight validation as full validation.

### Target UI Copy Principles

- Use "evidence" for partial or lightweight modes.
- Use "validated" only when the selected validation policy actually passed.
- Always show whether cross-dataset reruns were lightweight, preserved-finalist, or full-stack.
- Always show when Stage 5 fell back to Meta ranking.
- Always show when ML confidence used an unhealthy, stale, or train-fallback model.
- Always show when `MlSignal` validation is research-only.

### Frontend Files

Likely files:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`
- relevant Pinia stores:
  - `web/src/stores/validationStore.ts`
  - `web/src/stores/analysisStore.ts`
  - `web/src/stores/optimizeStore.ts`

## Workstream 10: Verification and Gates

### Unit and Integration Tests

Backend:

- `cargo test -p ta_app_core meta_select`
- `cargo test -p ta_app_core ml_confidence`
- `cargo test -p ta_app_core ml_signal`
- `cargo test -p ta_app_core double_ma_validate`
- targeted `src-tauri` tests for validation and report contracts.

Frontend:

- `npm --prefix web run test -- validationRequestBuilder`
- `npm --prefix web run test -- validationService`
- `npm --prefix web run test -- metaRequestBuilder`
- `npm --prefix web run test -- metaStage4RequestBuilder`
- `npm --prefix web run test -- metaService`
- `npm --prefix web run test -- mlSignalService`
- `npm --prefix web run typecheck`

Repo gates:

- `powershell -File scripts/run_repo_quality_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests` for broad backend changes.
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when changing validation or Meta runtime behavior.
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` for expensive validation changes.
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` for CUDA validation route changes when GPU is available.

### New Suggested Verification Assets

Add small fixture artifacts for:

- Meta train/holdout manifest disjointness.
- Meta train/holdout overlap.
- Partial holdout coverage.
- Stage 5 chronological training.
- Policy evaluation low sample count.
- `MlSignal` deployable vs research-only score scopes.

## Phased Implementation Plan

### Phase 0: Contract Audit and Fixtures

Purpose: prepare test fixtures and freeze expected current behavior before changing contracts.

Tasks:

- Document current artifact shapes for validation, Meta, Stage 5, and `MlSignal`.
- Add or refresh fixture artifacts with representative current payloads.
- Add regression tests proving current lightweight and preserved-finalist cross behavior before adding full-stack mode.
- Add tests proving old Meta and Stage 5 artifacts still load.

Exit criteria:

- Fixture tests pass.
- No production behavior changes yet.

### Phase 1: Evidence Vocabulary and Artifact Fields

Purpose: add optional fields that make evidence explicit without changing runtime behavior.

Tasks:

- Add validation evidence-depth fields.
- Add Meta holdout coverage fields.
- Add Stage 5 training manifest placeholder fields.
- Add `MlSignal` deployability placeholder fields.
- Surface legacy/unknown evidence states in UI summaries.

Exit criteria:

- Existing artifacts load.
- New artifacts contain optional evidence fields.
- UI displays evidence states without changing validation outcomes.

### Phase 2: Sealed Meta Cohort Manifest

Purpose: make Meta train/holdout data provenance auditable.

Tasks:

- Build `MetaCohortManifest`.
- Persist manifest in Meta JSON.
- Add disjointness checks.
- Add Meta UI/report summaries.
- Add warnings for missing fingerprints.

Exit criteria:

- Meta artifacts identify train/holdout inputs.
- Overlap is detected and surfaced.
- Missing metadata is visible.

### Phase 3: Partial Holdout Coverage Hardening

Purpose: make partial holdout safe for ranking, Stage 5, and ML training.

Tasks:

- Persist group-level coverage.
- Add Meta summary coverage.
- Update ML confidence row prep to count and report partial/unevaluated groups.
- Add policy evaluation warnings for partial coverage.

Exit criteria:

- Partial holdout cannot be mistaken for complete holdout.
- Unevaluated groups are not treated as failed labels.

### Phase 4: Chronological Stage 5 Training

Purpose: reduce historical leakage in Stage 5 workflows.

Tasks:

- Add training history policy to request builder and Rust request.
- Filter training artifacts by source timestamp in chronological mode.
- Persist training manifest and excluded artifact reasons.
- Add UI control and warnings.

Exit criteria:

- Stage 5 chronological mode trains only on older artifacts.
- Stage 5 artifact records exactly what was used.

### Phase 5: Full Cross-Dataset Validation

Purpose: add a stronger evidence mode for cross-dataset validation.

Tasks:

- Add `full_stack` cross mode to frontend and backend contracts.
- Preserve all selected modules in cross reruns.
- Add caps and warnings.
- Add artifact/report module coverage.

Exit criteria:

- Full-stack cross mode preserves requested modules.
- Existing cross modes retain current behavior.
- UI clearly distinguishes all three cross modes.

### Phase 6: Expanded Policy Evaluation

Purpose: make Stage 5 evaluation decision-grade.

Tasks:

- Expand policy arm metrics.
- Add intervals/lift/sample-size warnings.
- Add calibration and utility summaries where feasible.
- Update UI preview and saved policy eval replay.

Exit criteria:

- Policy eval reports more than precision@k.
- Low sample or weak lift is visible.
- Old policy eval artifacts still load.

### Phase 7: `MlSignal` Deployability

Purpose: distinguish research validation from deployable validation.

Tasks:

- Add deployability state to `MlSignal` validation result.
- Implement gates for score scope, context, coverage, duplicate timestamps, schema, and bundle provenance.
- Surface state in ML panel and artifacts.

Exit criteria:

- Research runs still work.
- Deployable state requires stronger score-cache and context evidence.

### Phase 8: Report and Workflow Gate Updates

Purpose: make the new evidence visible in durable outputs and regression gates.

Tasks:

- Update validation XLSX and Meta XLSX where needed.
- Update output parity fixtures.
- Extend workflow gate or add targeted headless checks for cohort and Stage 5 policies.
- Add performance smoke for full-stack cross mode.

Exit criteria:

- Reports explain evidence depth.
- Gates catch contract regressions.

## Implementation Checklist

### Phase 0: Contract Audit and Fixtures

- [ ] Inventory current validation artifact JSON fields.
- [ ] Inventory current Meta artifact JSON fields.
- [ ] Inventory current Stage 5 artifact JSON fields.
- [ ] Inventory current `MlSignal` artifact JSON fields.
- [ ] Add fixture for conventional validation with no cross dataset.
- [ ] Add fixture for preserved-finalist cross validation.
- [ ] Add fixture for lightweight cross validation.
- [ ] Add fixture for Meta train-only selection.
- [ ] Add fixture for Meta holdout selection.
- [ ] Add fixture for Stage 5 source-excluded artifact.
- [ ] Add fixture for `MlSignal` score-cache validation.
- [ ] Add load/replay tests for all fixture families.

### Phase 1: Evidence Vocabulary and Optional Fields

- [ ] Define evidence-depth string constants or enums.
- [ ] Add validation evidence-depth serialization.
- [ ] Add validation module coverage serialization.
- [ ] Add Meta evidence-depth serialization.
- [ ] Add Meta holdout coverage placeholder.
- [ ] Add Stage 5 training manifest placeholder.
- [ ] Add Stage 5 fallback reason normalization.
- [ ] Add `MlSignal` deployability placeholder.
- [ ] Update frontend renderers to tolerate missing legacy fields.
- [ ] Add frontend tests for legacy missing evidence fields.

### Phase 2: Sealed Meta Cohort Manifest

- [ ] Implement `MetaCohortArtifactRef`.
- [ ] Implement `MetaCohortManifest`.
- [ ] Implement artifact hash helper for Meta JSON artifacts.
- [ ] Extract dataset identity fields from saved run artifacts.
- [ ] Build train artifact refs during Meta scan.
- [ ] Build holdout artifact refs during Meta scan.
- [ ] Implement disjointness check by artifact hash.
- [ ] Implement disjointness check by dataset id.
- [ ] Implement disjointness check by CSV path.
- [ ] Implement disjointness check by source fingerprint.
- [ ] Persist `cohort_manifest` in Meta JSON.
- [ ] Surface manifest summary in Meta UI.
- [ ] Surface manifest warnings in Meta UI.
- [ ] Add Meta report manifest summary.
- [ ] Add tests for clean disjoint cohorts.
- [ ] Add tests for overlapping train/holdout cohorts.
- [ ] Add tests for missing fingerprints.

### Phase 3: Partial Holdout Coverage

- [ ] Add result-level holdout coverage summary.
- [ ] Add group-level holdout coverage fields.
- [ ] Mark evaluated holdout groups.
- [ ] Mark unevaluated holdout groups.
- [ ] Preserve current full-holdout ranking behavior.
- [ ] Preserve current partial-holdout ranking behavior, but make it explicit.
- [ ] Add warning when partial holdout is active.
- [ ] Update ML confidence row prep to skip unevaluated groups.
- [ ] Report skipped partial/unevaluated group counts.
- [ ] Add Meta table/report coverage columns.
- [ ] Add tests for complete holdout coverage.
- [ ] Add tests for partial holdout coverage.
- [ ] Add tests that unevaluated groups are not negative labels.

### Phase 4: Chronological Stage 5 Training

- [ ] Add `training_history_policy` to Stage 5 frontend builder.
- [ ] Add `training_history_policy` to Rust request.
- [ ] Add source artifact timestamp extraction.
- [ ] Filter training artifacts older than source in chronological mode.
- [ ] Exclude source artifact in all Stage 5 training modes.
- [ ] Persist training artifact refs.
- [ ] Persist excluded artifact refs and reasons.
- [ ] Warn when chronological mode leaves too little training data.
- [ ] Add UI control for training history policy.
- [ ] Add Stage 5 saved replay display for training policy.
- [ ] Add tests for source-excluded mode.
- [ ] Add tests for chronological mode.
- [ ] Add tests for future artifact exclusion.
- [ ] Add tests for no-history failure/fallback.

### Phase 5: Full Cross-Dataset Validation

- [ ] Add `full_stack` mode to frontend type union.
- [ ] Add `full_stack` mode to backend enum.
- [ ] Update request builder normalization.
- [ ] Add UI option and warning text.
- [ ] Preserve walk-forward in full-stack cross requests.
- [ ] Preserve reopt walk-forward in full-stack cross requests.
- [ ] Preserve SPA in full-stack cross requests.
- [ ] Preserve perturbation in full-stack cross requests.
- [ ] Preserve trade bootstrap in full-stack cross requests.
- [ ] Preserve CSCV/PBO in full-stack cross requests.
- [ ] Preserve hard gates in full-stack cross requests.
- [ ] Add cross module coverage provenance.
- [ ] Add cross dataset cap and warning.
- [ ] Add tests proving current lightweight stripping behavior remains.
- [ ] Add tests proving current preserved-finalist behavior remains.
- [ ] Add tests proving full-stack preserves modules.
- [ ] Add report output coverage for cross evidence depth.

### Phase 6: Expanded Policy Evaluation

- [ ] Add policy arm selected/evaluated counts.
- [ ] Add base positive rate.
- [ ] Add recall@k.
- [ ] Add lift vs base.
- [ ] Add lift vs Meta.
- [ ] Add confidence intervals.
- [ ] Add low-sample warning.
- [ ] Add calibration summary fields.
- [ ] Add optional utility score.
- [ ] Add regime/timeframe breakdown if existing metadata supports it.
- [ ] Update saved policy evaluation JSON.
- [ ] Update policy evaluation preview.
- [ ] Add tests for metric calculations.
- [ ] Add tests for interval determinism.
- [ ] Add tests for low-sample warnings.
- [ ] Add tests for old artifact replay.

### Phase 7: ML Confidence Governance

- [ ] Add labeled artifact count to model status.
- [ ] Add labeled row count to model status.
- [ ] Add target resolved to model status.
- [ ] Add skipped incompatible count to model status.
- [ ] Add skipped missing holdout count to model status.
- [ ] Add skipped partial holdout count to model status.
- [ ] Add CV evidence source to model status.
- [ ] Add calibration retained and Brier delta to model status.
- [ ] Surface model governance in Meta panel.
- [ ] Add tests for model governance status.

### Phase 8: `MlSignal` Deployability

- [ ] Define `MlSignalDeployability`.
- [ ] Add deployability to validation result.
- [ ] Add deployability to policy sweep result via nested validation result.
- [ ] Gate on validation context presence.
- [ ] Gate on probability score space.
- [ ] Gate on duplicate timestamps.
- [ ] Gate on score scope.
- [ ] Gate on coverage ratio.
- [ ] Gate on feature schema hash/version.
- [ ] Gate on source bundle manifest hash when native bundle scoring is used.
- [ ] Gate on configured PSR/DSR survival.
- [ ] Add research-only warning for calibration-slice score scope.
- [ ] Surface deployability in ML panel.
- [ ] Add tests for deployable pass.
- [ ] Add tests for research-only scope.
- [ ] Add tests for blocked score-cache states.

### Phase 9: Reports, Gates, and Documentation

- [ ] Update validation XLSX report fields.
- [ ] Update Meta XLSX report fields.
- [ ] Update saved replay displays.
- [ ] Update output parity fixtures.
- [ ] Update `REPO_MAP.md` routing notes after implementation.
- [ ] Update `ARCHITECTURE_NOTES.md` if new contracts are broad.
- [ ] Add workflow gate coverage for cohort manifest.
- [ ] Add workflow gate coverage for Stage 5 chronological mode.
- [ ] Add workflow gate coverage for `MlSignal` deployability.
- [ ] Run `npm --prefix web run test`.
- [ ] Run `npm --prefix web run typecheck`.
- [ ] Run targeted Rust tests.
- [ ] Run `powershell -File scripts/run_repo_quality_gate.ps1`.
- [ ] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`.

## Acceptance Criteria

The improvement project is complete when:

- Meta artifacts include cohort provenance sufficient to audit train/holdout separation.
- Partial holdout coverage is explicit at result and group level.
- Stage 5 artifacts identify source artifact, training history policy, training artifacts, excluded artifacts, model health, and fallback reason.
- Stage 5 can run in chronological training mode.
- Policy evaluation reports lift, uncertainty, sample-size warnings, and calibration/governance status beyond precision@k.
- Cross-dataset validation offers an opt-in full-stack mode while preserving current fast modes.
- Validation reports distinguish lightweight evidence from full-stack evidence.
- `MlSignal` validation reports a deployability state without blocking research workflows.
- Old artifacts continue to load with clear legacy evidence warnings.
- Tests cover the new contracts and old compatibility behavior.

## Suggested Implementation Order

Recommended order:

1. Evidence vocabulary and optional artifact fields.
2. Meta cohort manifest.
3. Partial holdout coverage.
4. Chronological Stage 5.
5. Expanded policy evaluation.
6. `MlSignal` deployability.
7. Full cross-dataset validation mode.
8. Report/gate hardening.

Rationale:

- Metadata-only fields reduce risk and unblock UI/report copy.
- Cohort and coverage improvements make Stage 5 and policy evaluation more reliable.
- Full-stack cross validation is likely the most runtime-sensitive change, so it should come after the contract work is stable.

## Open Design Questions

- Should train/holdout overlap become a hard error immediately, or first ship as a prominent warning?
- Should chronological Stage 5 become the default after one compatibility release?
- Should policy evaluation use bootstrap intervals by default, or simpler Wilson intervals first?
- Should `MlSignal` deployability require `walkforward_test` score scope only, or allow `full_dataset` when generated from an approved holdout bundle?
- Should full-stack cross validation reuse the same candidate cap as preserved-finalist mode, or expose a separate cap?
- Should Meta report XLSX get a dedicated manifest worksheet, or should manifest details stay JSON-only with summary columns in XLSX?

