# Review Packet: `crates/app_core/src/ml_confidence.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ml_confidence.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and locally well tested, but not fully public-review ready because model health can currently be inferred from in-sample metrics when cross-validation is unavailable, the reported count of 'meta artifacts used' can overstate the artifacts that actually contributed labeled rows, and group-based CV depends on metadata assumptions that are not documented at the public contract boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `ML-support`, `Semantics-sensitive`, `Workflow-critical`, `Low-runtime-risk`

## What This File Owns

`ml_confidence.rs` owns ML-confidence dataset preparation, lightweight model training, calibration, prediction, and model-summary metadata for meta-selection artifacts.

It owns:

- public scope/config/model DTOs:
  - `MlConfidenceScope`: `crates/app_core/src/ml_confidence.rs:29-32`
  - `MlConfidenceLabelConfig`: `35-55`
  - `MlConfidenceTarget`: `99-109`
  - `MlConfidenceModelFamily`: `118-127`
  - `MlConfidenceModelKind`: `136-139`
  - `MlConfidenceTrainConfig`: `148-189`
  - training/calibration/model metadata structs:
    - `192-291`
- public feature-row and prepared-row DTOs:
  - `MlConfidenceGroupFeatures`: `307-335`
  - `MlConfidenceDatasetRow`: `338-342`
  - `MlConfidencePreparedArtifactRows`: `345-349`
  - `MlConfidencePreparedRows`: `352-357`
  - GBDT model structs:
    - `360-372`
- feature extraction and label derivation:
  - `extract_ml_confidence_group_features`: `443-456`
  - holdout-label helpers:
    - `462-505`
- meta-artifact preparation into labeled rows:
  - `prepare_ml_confidence_rows_from_meta_artifacts`: `542-695`
  - prepared-row dataset build:
    - `698-771`
- scaler, logistic regression, GBDT-stump training, calibration, and metrics:
  - scaler helpers: `796-840`
  - `fit_logistic_regression`: `842-905`
  - threshold/stump selection and GBDT training: `907-1280`
  - `brier_score`: `1294-1304`
  - `fit_platt_scaling_from_probs`: `1314-1339`
  - `apply_calibration`: `1342-1346`
  - `auc_roc`: `1348-1390`
  - `calibration_bins`: `1392-1419`
  - `train_ml_confidence_model_from_rows_inner`: `1421-1637`
- public training/prediction APIs:
  - `train_ml_confidence_model_from_rows`: `1639-1671`
  - `train_ml_confidence_model_from_prepared_rows`: `1673-1686`
  - `train_ml_confidence_model_from_meta_artifacts`: `1688-1707`
  - `predict_ml_confidence_for_features`: `1709-1740`
  - `predict_ml_confidence_for_group`: `1742-1744`
- file-local tests:
  - `1748-2205`

This file does not own:

- meta-selection artifact generation and holdout augmentation:
  - `crates/app_core/src/meta_select.rs`
  - `src-tauri/src/commands_meta.rs`
- desktop command orchestration and model-status presentation:
  - `src-tauri/src/commands_meta.rs`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:46`
  - its public surface is re-exported from `crates/app_core/src/lib.rs:190`
- Hot path: no, not in the same sense as optimization/runtime kernels
  - training and preparation are batch/offline workflow paths rather than per-bar execution hot paths
- CUDA/runtime-critical: no
- Strategy-local: no
  - this is cross-cutting meta/ML support infrastructure
- Low-risk support code: no
  - it is support code, but it produces user-facing model-health and prediction outputs

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:46`
  - `190`
- meta-selection result shape dependency:
  - `crates/app_core/src/meta_select.rs:259-324`
  - `327-338`
- Tauri command boundary and status/policy consumers:
  - `src-tauri/src/commands_meta.rs:14-15`
  - `433`
  - `962-967`
  - `1020`
  - `1380-1384`
  - `3282`
  - `3310-3315`
  - `3692-3738`
- direct command-side imports of model meta:
  - `src-tauri/src/commands_meta.rs:3126`

### Downstream verification impact if changed

- changing public model/train DTOs affects:
  - `commands_meta.rs`
  - any persisted model bundles or summaries
- changing prepared-row semantics affects:
  - Tauri-side status generation
  - policy evaluation/training workflows
- changing prediction semantics affects:
  - model-confidence scoring inside `commands_meta.rs`
  - any logic that blocks or blends policy stages based on ML confidence

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API contracts, Serde configuration compatibility, probability calibration guidance, and standard evaluation metrics used by the file’s model-health logic.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exports many public config, model, metrics, and prediction DTOs
   - Standard informed:
     - public APIs should be intentional, documented, and not over-promise semantics the implementation cannot support clearly

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - the file’s public config/model types rely on defaults and optional serialized fields
   - Standard informed:
     - defaulted/optional fields are part of the compatibility contract and should be documented when they shape persisted model state

3. scikit-learn probability calibration guide
   - Source: <https://scikit-learn.org/stable/modules/calibration.html>
   - Why it was consulted:
     - this file fits and applies Platt-style calibration from predicted probabilities
   - Standard informed:
     - calibration should be based on data independent from the fitting data when possible, and CV-derived predictions are a meaningful boundary for deciding whether calibration is trustworthy

4. scikit-learn `roc_auc_score` documentation
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.metrics.roc_auc_score.html>
   - Why it was consulted:
     - the file uses AUC as part of the model-health decision
   - Standard informed:
     - AUC is an evaluation metric, but the reliability of the health signal depends on whether the evaluation is out-of-sample or in-sample

5. scikit-learn `brier_score_loss` documentation
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.metrics.brier_score_loss.html>
   - Why it was consulted:
     - the file uses Brier score and baseline Brier as a health gate and calibration signal
   - Standard informed:
     - Brier score is useful, but interpretation depends on how the evaluation set is constructed and whether calibration was fit on unbiased predictions

6. Rust Book, Recoverable Errors With `Result`
   - Source: <https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html>
   - Why it was consulted:
     - public training APIs return `Result<..., String>` and flatten some warning/error context
   - Standard informed:
     - error returns should preserve meaningful diagnostics when a caller needs to distinguish recoverable conditions

## Findings

## Correctness And Safety

### 1. Model `healthy` can currently be inferred from training metrics when cross-validation is unavailable

- Evidence:
  - CV warnings are emitted when there are not enough unique groups:
    - `crates/app_core/src/ml_confidence.rs:1181`
    - `1267`
  - health evaluation falls back to training metrics when CV metrics are absent:
    - `eval_auc = cv_auc.unwrap_or(train_auc)`: `1566`
    - `eval_brier = cv_brier_cal.or(cv_brier).unwrap_or(train_brier)`: `1567`
  - `healthy` is then derived from those fallback metrics:
    - `1568-1578`
- Why it matters:
  - a model can be marked healthy even when there is no out-of-sample validation signal
  - that makes the model-health bit stronger than the evidence actually supports
- Smallest justified fix:
  - keep training possible when CV is unavailable
  - but downgrade health when CV was skipped, or add a distinct “trained but unvalidated” state in the metadata/warnings
  - do not silently treat train metrics as equivalent to CV metrics
- Review judgment:
  - medium-high correctness / trust-signal issue

## API And Naming

### 1. `meta_artifacts_used` overstates actual labeled contributors because it counts scanned prefix artifacts, not artifacts that produced training rows

- Evidence:
  - `used_meta` is incremented before checking whether the artifact contributes any rows:
    - `crates/app_core/src/ml_confidence.rs:709-710`
  - artifacts with empty source rows are then skipped:
    - `725-737`
  - the resulting `used_meta` is written into public metrics/model metadata:
    - `1605`
    - `1630`
- Why it matters:
  - the name reads as “artifacts used for training,” but the implementation counts artifacts examined within the prefix, including ones that contributed zero labels
  - that can mislead downstream status and retraining logic
- Smallest justified fix:
  - either rename the field semantics clearly
  - or increment `used_meta` only when the artifact actually contributes rows to the chosen target dataset
- Review judgment:
  - medium API/metrics-reporting issue

### 2. The public surface is broad relative to apparent in-repo consumers

- Evidence:
  - the module exports many public structs and functions:
    - examples:
      - `MlConfidenceModel`: `264`
      - `MlConfidenceModelSummary`: `284`
      - `MlConfidencePreparedRows`: `352`
      - `prepare_ml_confidence_rows_from_meta_artifacts`: `542`
      - public training/prediction APIs: `1639-1744`
  - apparent primary consumers are `commands_meta.rs` and related tests/status builders:
    - `src-tauri/src/commands_meta.rs:962-967`
    - `1020`
    - `3282`
    - `3310-3315`
    - `3692-3738`
- Why it matters:
  - every public DTO and helper becomes a compatibility promise, including persistence-oriented internal plumbing types
- Smallest justified fix:
  - audit which items truly need to remain public
  - keep stable public model/config/result shapes public, but consider narrowing preparation-only helpers if no external consumer needs them
- Review judgment:
  - medium public-surface issue

## Error Handling

### 1. Training failures flatten accumulated warning context into a single joined `String`

- Evidence:
  - insufficient-row failure:
    - `crates/app_core/src/ml_confidence.rs:1433-1436`
  - insufficient-class-balance failure:
    - `1438-1444`
  - both return `Err(warnings.join(" "))`
- Why it matters:
  - the caller gets human-readable context, but loses structure that could distinguish data sufficiency, scope mismatch, and model-family fallback conditions programmatically
- Smallest justified fix:
  - keep string errors if broader crate consistency matters
  - but consider preserving the warning vector in the error path when a later pass needs more structured diagnostics
- Review judgment:
  - low-medium error-handling issue

## Module Boundaries And Technical Debt

### 1. Group-based CV depends on metadata assumptions that are not enforced strongly enough

- Evidence:
  - gate-target row grouping uses the artifact `ts` field as `group_id`:
    - `crates/app_core/src/ml_confidence.rs:574-578`
    - `618-623`
  - if `ts` is missing, the fallback is `""`:
    - `574-578`
  - CV fold assignment depends on `group_id` uniqueness:
    - `1125-1143`
    - unique group count also informs model-family selection:
      - `1449-1467`
- Why it matters:
  - multiple artifacts missing `ts` collapse into the same group bucket, which can reduce effective CV diversity and alter both CV availability and auto-family selection
- Smallest justified fix:
  - when `ts` is missing, derive a stable fallback group id from another artifact-identity source instead of the empty string
  - at minimum, emit an explicit warning when CV grouping is degraded by missing timestamps
- Review judgment:
  - medium module-boundary / data-contract issue

### 2. The file concentrates DTOs, feature extraction, dataset preparation, training, calibration, and prediction in one module

- Evidence:
  - public DTOs: `29-372`
  - preparation: `542-771`
  - training/calibration internals: `796-1637`
  - public train/predict APIs: `1639-1744`
- Why it matters:
  - this is a lot of responsibility in one file, which raises review cost and makes later surgical changes harder
  - the concentration is objective even if the current implementation is functional
- Smallest justified fix:
  - do not refactor broadly right now
  - document internal seams and consider a future split into `schema`, `prepare`, `train`, and `predict` only if repeated maintenance pressure justifies it
- Review judgment:
  - medium technical-debt issue

## Docs And Examples

### 1. The public model/train/predict surface lacks rustdoc about persistence and trust semantics

- Evidence:
  - public items include:
    - `MlConfidenceScope`: `crates/app_core/src/ml_confidence.rs:29`
    - `MlConfidenceLabelConfig`: `35`
    - `MlConfidenceTrainConfig`: `148`
    - `MlConfidenceTrainMetrics`: `200`
    - `MlConfidenceModelMeta`: `238`
    - `MlConfidenceModel`: `264`
    - `MlConfidenceModelSummary`: `284`
    - `prepare_ml_confidence_rows_from_meta_artifacts`: `542`
    - `train_ml_confidence_model_from_rows`: `1639`
    - `train_ml_confidence_model_from_prepared_rows`: `1673`
    - `train_ml_confidence_model_from_meta_artifacts`: `1688`
    - `predict_ml_confidence_for_features`: `1709`
    - `predict_ml_confidence_for_group`: `1742`
  - the persisted model currently carries `version: 4`:
    - `1582-1594`
  - there is no rustdoc explaining:
    - versioning compatibility expectations
    - what `healthy` really guarantees
    - whether `meta_artifacts_used` means scanned versus contributing artifacts
    - when CV-backed calibration is or is not available
- Smallest justified fix:
  - add rustdoc to the public model/train/predict APIs and model/meta structs
  - document the semantics of `healthy`, `meta_artifacts_used`, and persisted `version`

## Tests And Missing Verification

### 1. File-local training coverage is good, and a Tauri-side consumer test passes, but key trust/metadata edge cases are not pinned directly

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core ml_confidence::tests:: -- --nocapture`
      - passed
      - included:
        - `trains_and_predicts_confidence_from_meta_artifacts`
        - `trains_gbdt_when_requested`
        - `prepared_row_prefix_auto_target_preserves_meta_usage`
        - `trains_from_holdout_rows_dataset_labels`
        - `prepared_rows_match_raw_training_and_feature_prediction`
  - Tauri-side consumer verification run:
    - `cargo test -p ta_desktop_demo_app build_ml_confidence_status_prefers_dataset_rows_and_keeps_scan_error -- --nocapture`
      - passed
  - current tests do not directly pin:
    - health behavior when CV is skipped
    - `meta_artifacts_used` semantics for zero-contribution artifacts
    - missing-`ts` group-id fallback behavior
- Missing tests I would want before changing this file:
  - a test that a CV-skipped model is not marked healthy by train metrics alone, if that policy is adopted
  - a test that `meta_artifacts_used` reflects the intended semantic meaning
  - a test that missing `ts` values do not collapse CV grouping unexpectedly

## Performance, Allocation, And CUDA Concerns

### 1. No urgent performance defect stands out; correctness and trust semantics matter more here than micro-optimization

- Evidence:
  - the heavy work is batch-oriented data prep and training rather than a runtime execution hot loop:
    - `crates/app_core/src/ml_confidence.rs:542-771`
    - `796-1637`
- Review judgment:
  - this is not the right file to chase small allocation wins before the health/metadata semantics are clarified

### 2. The current CV/training implementation does clone data per fold, but that is acceptable for the current workflow role

- Evidence:
  - logistic CV builds per-fold train/test vectors:
    - `crates/app_core/src/ml_confidence.rs:1135-1177`
  - GBDT CV does the same:
    - `1216-1260`
- Review judgment:
  - benchmark-driven optimization can wait
  - any future optimization should preserve fold semantics and calibration correctness first

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for public types and trust semantics
  - fix `meta_artifacts_used` semantics or naming
  - add explicit warnings/tests for missing-`ts` CV grouping
  - tighten health semantics when CV is unavailable
- Coordination required:
  - narrowing public preparation/model-helper surfaces
  - changing persisted metadata semantics if UI or saved models already depend on them
- Defer until later pass:
  - larger module splitting work

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing rustdoc
  - ambiguous `meta_artifacts_used` reporting
  - missing-`ts` grouping fallback ambiguity
- Coordination required:
  - persisted-model compatibility changes
  - public-surface narrowing
- Useful debt that should not be “cleaned up” casually:
  - CV-based calibration flow, which is doing real contract work
  - the dual raw/prepared-row training entrypoints, which support useful workflow reuse

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent
- Benchmark-sensitive candidates:
  - reducing per-fold cloning
  - changing row-grouping strategy
- Not the right place to chase performance without evidence:
  - low-signal allocation changes that would complicate calibration and CV logic

## Observability And Diagnostics Gaps

- `healthy` does not currently distinguish “validated out-of-sample” from “looked good in-sample”
- `meta_artifacts_used` does not make its semantics obvious
- missing timestamp metadata can weaken CV grouping without a first-class diagnostic

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive tests
  - metadata/health semantics cleanup
- Coordination required:
  - persisted-model field semantics
  - public-surface narrowing
- Defer until later pass:
  - broader module splitting

## Change Blast Radius

- Local only:
  - docs
  - additive tests
- Same subsystem:
  - metadata/health changes affect `commands_meta.rs` status and policy evaluation flows
- Cross-backend / frontend-backend contract:
  - persisted model/summary field changes affect Tauri consumers and stored artifacts

## Open Questions Or Assumptions

- assumption:
  - `healthy` is intended to communicate trustworthiness to downstream command/UI flows, not just raw train success
- open question:
  - whether any external consumer already interprets `meta_artifacts_used` as “contributing artifacts” rather than “prefix artifacts scanned”
- open question:
  - whether missing `ts` on meta artifacts is expected to be rare enough that the current fallback has not mattered in practice

## Recommended Action

`Small surgical refactor`

The right next pass is:

- tighten the health and metadata semantics
- document the persisted/public model contract
- add targeted tests for the missing edge cases
- leave larger module-splitting work for a later pass

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core ml_confidence::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app build_ml_confidence_status_prefers_dataset_rows_and_keeps_scan_error -- --nocapture`
- if health semantics or metadata fields change:
  - `cargo test -p ta_desktop_demo_app commands::commands_meta::tests:: -- --nocapture`
  - `cargo test --workspace --no-run`

## Regression Risk And Feature-Preservation Constraints

- do not silently reinterpret persisted model metadata without checking Tauri-side status consumers
- do not tighten health semantics without deciding how downstream policy logic should react
- do not collapse or remove calibration/CV flows in the name of simplification
