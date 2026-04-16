# ML Signal CatBoost Workflow Improvement Plan

Last updated: 2026-04-15  
Scope: `ta_desktop_demo` `MlSignal` research harness, score-cache workflow, and desktop-facing bundle/validation contracts  
Status: proposed implementation plan  
Primary goal: improve the current CatBoost-backed `MlSignal` workflow so it becomes methodologically trustworthy, operationally explicit, and easier to evolve without silently breaking desktop validation.

## 1. Executive Summary

The current `MlSignal` architecture is pointed in the right direction:

- Rust owns feature export, score-cache ingestion, validation, and policy sweep.
- Python owns CatBoost training, calibration, and research-side evaluation.
- The desktop consumes score caches rather than depending on a hard CatBoost runtime in the main app path.

That separation is good and should be preserved.

The main problem is not architectural confusion. The main problem is research validity and contract rigor:

- the current Python walk-forward evaluation is not a true no-future-leak walk-forward procedure
- the feature/export/bundle contracts are useful but still underspecified
- the current pattern-recognition feature block is likely throwing away useful polarity information
- model bundle health and score-cache integrity checks are good but still too shallow for long-lived iteration

This plan improves the existing workflow instead of replacing it.

The intended outcome is:

- a leakage-safe rolling evaluation path
- explicit dataset, feature, label, calibration, threshold, and bundle provenance
- stronger diagnostics and baseline comparisons
- desktop validation that can trust the score-cache inputs it receives

This plan does **not** assume native CatBoost inference inside the desktop app is required.

## 2. Current Repo State

### 2.1 What exists today

Relevant repo surfaces:

- Rust feature export and score-cache logic:
  - `crates/app_core/src/ml_signal.rs`
- Tauri command boundary:
  - `src-tauri/src/commands_ml_signal.rs`
- frontend orchestration:
  - `web/src/services/mlSignalService.ts`
- Python research harness:
  - `research/ml_signal/common.py`
  - `research/ml_signal/train_catboost.py`
  - `research/ml_signal/calibrate.py`
  - `research/ml_signal/evaluate_walkforward.py`
  - `research/ml_signal/gate_report.py`
- prior broad implementation framing:
  - `research/ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`

Current operational flow:

1. desktop exports an `MlSignal` dataset CSV plus sidecar schema/manifest
2. Python trains CatBoost long/short models
3. Python optionally calibrates scores
4. Python writes score CSV artifacts
5. Rust builds a score-cache JSON from the score CSV
6. desktop validation and policy sweep consume the score cache

This matches the current repo map, which explicitly treats `MlSignal` as a separate workflow with the stages:

- dataset export
- score-cache generation
- validation
- policy sweep

### 2.2 What is already good

- The workflow is separate from the normal optimizer analysis path.
- Score-cache ingestion is explicit about accepted columns, preferred calibrated probability columns, and coverage metadata.
- Dataset/timeframe compatibility checks already exist on the Tauri boundary.
- Bundle manifests and score-cache manifests already encode useful metadata.
- The current ML workflow keeps the latency-sensitive desktop path free of a mandatory Python/CatBoost dependency.

### 2.3 What is weak today

#### A. The current walk-forward evaluation is not a real rolling walk-forward

`research/ml_signal/common.py` currently resolves split masks from only the last split window when a split manifest exists. `train_catboost.py` then trains on those fixed masks, while `evaluate_walkforward.py` scores the entire dataset with one fitted pair of models and then evaluates multiple windows.

That is acceptable only as a rough proxy. It is not a reliable estimate of forward performance across all windows.

#### B. The feature contract is useful but not yet strong enough

The exporter writes:

- feature schema
- export manifest
- optional split manifest

But it does not yet fully freeze:

- schema hash
- indicator parameter block
- exact label/deadband policy
- training/calibration threshold provenance
- research bundle lineage across re-trains

#### C. Pattern-recognition features are likely oversimplified

The current exporter collapses non-zero pattern outputs to binary hits. That is a valid first pass, but it may discard bullish/bearish polarity or signed pattern semantics if the upstream indicator exposes them.

#### D. Sample weighting is scaffolded, not intentional

The dataset contains `sample_weight`, and Python will consume it, but the Rust exporter currently writes a constant `1.0` weight per row. That means the contract is ahead of the actual weighting policy.

#### E. Score-cache and bundle health checks are still mostly structural

The current checks catch:

- missing files
- schema version mismatch
- missing probability columns for certain desktop paths
- coverage and duplicate timestamps

But they do not yet fully enforce:

- schema hash compatibility
- score-scope provenance
- calibration provenance
- training-window provenance
- threshold provenance

## 3. External Research Findings That Should Shape The Plan

This section only includes sources that materially affect implementation choices.

### 3.1 Time-series validation must avoid future-to-past leakage

Scikit-learn documents `TimeSeriesSplit` as the appropriate family of cross-validation for ordered data because standard cross-validation methods can train on future data and evaluate on past data. It also supports:

- `gap`
- `test_size`
- `max_train_size`

Repo implication:

- the `MlSignal` research harness should move from "single fit + evaluate many windows" to true rolling training/calibration/testing
- the split plan should explicitly encode an embargo/gap large enough to prevent label leakage across adjacent windows
- the workflow should support both expanding-train and capped-train variants

### 3.2 CatBoost has explicit support for ordered data

CatBoost documents `has_time` as using the input order and not performing random permutations during the categorical-to-numeric and tree-structure stages.

Repo implication:

- the training harness should make a deliberate choice about `has_time`
- for time-ordered market data, `has_time=True` should be the default unless a measured counterexample justifies something else

### 3.3 Best-model selection requires a true validation set

CatBoost documents `use_best_model` as saving trees only up to the best iteration found on the validation dataset, and it requires a validation set.

Repo implication:

- the harness should keep a distinct calibration/validation slice for best-iteration selection
- `use_best_model` is appropriate only if the validation slice is strictly earlier than the test slice and disjoint from any final reported rows

### 3.4 Class imbalance handling should be explicit, not accidental

CatBoost exposes:

- `class_weights`
- `auto_class_weights`
- `scale_pos_weight`

Repo implication:

- the harness should choose one imbalance policy deliberately and record it in the bundle metadata
- it should not simultaneously rely on constant `sample_weight` and implicit class imbalance handling without documenting which one is actually active

### 3.5 Calibration requires disjoint calibration data and should be judged separately from ranking

Scikit-learn calibration guidance explicitly notes:

- the classifier-fit data and calibrator-fit data must be disjoint
- sigmoid and isotonic calibration have different tradeoffs
- calibration mainly improves probability quality, not rank metrics such as ROC AUC
- calibration quality should be examined with calibration curves and proper scoring rules such as Brier score and log loss

Repo implication:

- the current separation of train vs calibration rows is the right shape
- calibration should remain a first-class step, not an afterthought
- bundle reports should distinguish rank quality from probability quality
- default calibration should be sigmoid unless calibration-row counts are large enough to justify isotonic

### 3.6 Threshold selection can be treated as a controlled downstream policy decision

CatBoost exposes threshold utilities such as `get_probability_threshold` and `select_threshold`. The docs treat class-separation threshold as a model-adjacent but configurable decision surface.

Repo implication:

- threshold choice should remain outside the model weights and inside the bundle or desktop policy layer
- the desktop should keep owning operational thresholds for validation/policy sweep
- research reports should record the threshold policy used to produce each artifact

### 3.7 CatBoost already exposes model diagnostics worth using

CatBoost documents:

- feature importance
- SHAP values
- object importance
- model metadata storage

Repo implication:

- the research harness should write explainability artifacts for bundle review
- the model metadata storage can carry lineage and training-contract data
- object-importance diagnostics are useful when investigating suspicious rows or regime drift

### 3.8 Native inference is possible, but it is optional future scope

CatBoost documents a native evaluation library for C/C++ and explicitly presents it as the fastest way to evaluate a model, with shared-library artifacts available for Windows.

Repo implication:

- native inference is a viable future path if score-cache generation becomes operationally costly
- but it should remain optional until the research methodology and data contracts are solid

## 4. Guiding Decisions For This Repo

These decisions should be locked before implementation starts.

### 4.1 Keep the score-cache-first architecture

Do not reframe this effort as "embed CatBoost into the app."

The near-term product contract should stay:

- export dataset
- train/calibrate offline
- generate score CSV
- build score cache
- run desktop validation/policy sweep

### 4.2 Treat the current Python walk-forward as insufficient

Do not patch the reporting language around it and move on.

The methodology itself needs to change.

### 4.3 Strengthen the data and bundle contracts before adding more model complexity

The biggest risk is not model underfitting. The biggest risk is silently invalid or stale artifacts.

### 4.4 Keep native inference in future scope only

Do not spend the next implementation cycle on Rust CatBoost model application unless the workflow first proves trustworthy and repeatedly useful.

### 4.5 Additive rollout is mandatory

This cycle must be implemented as an additive hardening pass, not as a rewrite.

That means:

- keep the current desktop stage model intact
- keep current Tauri command names stable
- keep current score-cache-driven validation behavior working
- add new fields before making any of them hard requirements
- preserve current artifact reopen/replay behavior for older `MlSignal` runs

The target outcome is "better evidence with preserved workflow," not "new workflow with better evidence."

### 4.6 Non-regression rules

The following rules are hard constraints for implementation.

#### A. Older artifacts must continue to load

New readers must continue to load:

- existing exported ML datasets
- existing `feature_schema.json`
- existing bundle manifests
- existing score-cache manifests
- existing saved `MlSignal` run artifacts reopened through history/workflow views

If a new field becomes operationally important, it must still have:

- a safe default
- a compatibility fallback
- or an explicit warning path instead of immediate hard failure

#### B. The current proxy research flow cannot disappear until the rolling path is proven

The current single-fit proxy path is methodologically weaker, but it is still a working tool.

Implementation order must be:

1. add true rolling walk-forward
2. verify it
3. mark the old path as proxy/legacy
4. only then consider later removal

#### C. Desktop validation must not become Python-dependent

Validation and policy sweep must continue to operate from score caches without needing Python or CatBoost at runtime.

#### D. App startup must not gain a mandatory CatBoost-native dependency

Any native inference exploration must stay optional and must not alter the default app runtime path in this cycle.

#### E. Label, calibration, and threshold drift must never be silent

If any of the following change, those changes must be versioned and visible in manifests/reports:

- label logic
- deadband logic
- weighting policy
- calibration method
- threshold policy
- split policy

### 4.7 Compatibility plus better evidence is the definition of success

This repo should only consider the cycle successful if it delivers both:

- improved methodological trustworthiness
- preserved desktop workflow and replay safety

If model quality improves but saved artifacts no longer reopen or old score caches no longer validate, that is a regression.

## 5. Workstreams

## 5.1 Workstream A: Repair The Evaluation Methodology

### Objective

Replace the current "single fit + many-window evaluation" flow with a genuine rolling walk-forward procedure.

### Why this matters

This is the highest-priority issue because every downstream metric depends on it.

### Required changes

#### A1. Split resolution must expose all windows, not just the last window

Files:

- `research/ml_signal/common.py`

Tasks:

- replace `resolve_split_masks(...)` with a richer split utility that returns all valid windows, not a single last-window mask
- add helpers for:
  - expanding-train windows
  - capped-train windows via `max_train_size`
  - explicit `gap` / embargo handling
- validate that every returned window satisfies:
  - `train_end <= calibration_start`
  - `calibration_end <= test_start`
  - no overlap after `first_valid_bar` adjustment

#### A2. Train and calibrate per roll

Files:

- `research/ml_signal/train_catboost.py`
- `research/ml_signal/evaluate_walkforward.py`

Tasks:

- add a roll-aware training mode that fits:
  - one long model per roll
  - one short model per roll
  - optional per-roll calibrators
- score only each roll's test slice with that roll's prior-trained models
- persist per-roll summaries even if per-roll models are not kept permanently

#### A3. Keep a cheap research-only baseline mode

Files:

- `research/ml_signal/evaluate_walkforward.py`
- `research/ml_signal/README.md`

Tasks:

- preserve the current one-fit evaluation only as an explicitly named baseline such as `single_fit_proxy`
- ensure reports never present that baseline as true walk-forward evidence

### Acceptance criteria

- no scored test row is produced by a model fitted on later rows
- calibration rows are disjoint from training rows
- reported walk-forward metrics come only from true rolling evaluation

## 5.2 Workstream B: Harden The CatBoost Training Contract

### Objective

Make the training configuration explicit, reproducible, and visible in manifests.

### Required changes

#### B1. Add a typed training config

Files:

- `research/ml_signal/train_catboost.py`
- `research/ml_signal/README.md`

Tasks:

- introduce a structured training config block with:
  - `iterations`
  - `learning_rate`
  - `depth`
  - `loss_function`
  - `eval_metric`
  - `has_time`
  - `use_best_model`
  - optional overfitting detector / early-stopping config
  - imbalance policy
- persist the resolved config into:
  - `train_metadata.json`
  - `manifest.json`

Recommended defaults:

- `loss_function=Logloss`
- `eval_metric=Logloss`
- `has_time=True`
- `use_best_model=True`

#### B2. Make imbalance policy explicit

Files:

- `research/ml_signal/train_catboost.py`
- `research/ml_signal/common.py`
- `crates/app_core/src/ml_signal.rs`

Tasks:

- choose one of:
  - explicit row weights
  - `auto_class_weights`
  - `scale_pos_weight`
- do not leave constant `sample_weight=1.0` and implicit class handling as an undocumented hybrid
- write the chosen policy into bundle metadata

Recommended default:

- keep row-level `sample_weight` support
- add an optional class-balance layer
- default to no extra class weighting until measured label imbalance justifies it

### Acceptance criteria

- the bundle records enough information to reproduce training
- changing training hyperparameters without changing bundle metadata becomes impossible

## 5.3 Workstream C: Strengthen The Calibration Pipeline

### Objective

Make calibration a deliberate, data-aware step with clearly separated evidence.

### Required changes

#### C1. Support calibration policy selection

Files:

- `research/ml_signal/calibrate.py`
- `research/ml_signal/train_catboost.py`
- `research/ml_signal/gate_report.py`

Tasks:

- support:
  - sigmoid calibration
  - isotonic calibration
- choose sigmoid as the default
- allow isotonic only when calibration sample counts exceed a configured floor
- record the chosen method and row counts in `calibration.json`

#### C2. Report calibration quality separately from ranking quality

Files:

- `research/ml_signal/calibration_slice_report.py`
- `research/ml_signal/gate_report.py`

Tasks:

- report:
  - calibration curve bins
  - Brier score
  - log loss
  - positive rate by probability bin
- separate these from:
  - ROC AUC
  - average precision / PR summary

#### C3. Keep desktop score-cache ingestion strict

Files:

- `crates/app_core/src/ml_signal.rs`

Tasks:

- keep calibrated probability columns preferred
- keep raw margins as diagnostic-only fallback
- extend manifests to record whether a score file is:
  - raw
  - uncalibrated probability
  - calibrated probability

### Acceptance criteria

- a bundle reviewer can tell whether a model ranks well but calibrates poorly
- desktop validation clearly knows whether it is consuming calibrated probabilities

## 5.4 Workstream D: Strengthen The Feature And Label Contracts

### Objective

Freeze feature, label, and warmup semantics so research outputs stay reproducible across revisions.

### Required changes

#### D1. Add schema hash and indicator-parameter provenance

Files:

- `crates/app_core/src/ml_signal.rs`
- `web/src/services/mlSignalService.ts`

Tasks:

- extend the export manifest and feature schema with:
  - schema hash
  - exporter version
  - indicator parameter block
  - warmup logic
  - label/deadband policy
  - horizon definition

#### D2. Improve pattern-recognition feature fidelity

Files:

- `crates/app_core/src/ml_signal.rs`

Tasks:

- investigate whether `pattern_recognition` output semantics include sign or directional information
- if yes, preserve it instead of collapsing all non-zero values to `1`
- if directional semantics are not stable across pattern rows, emit both:
  - binary hit features
  - signed/polarity features when available

Recommended constraint:

- do not broaden the feature family dramatically in this pass
- improve fidelity of the current features before adding many new ones

#### D3. Make sample weighting intentional

Files:

- `crates/app_core/src/ml_signal.rs`
- `research/ml_signal/common.py`

Tasks:

- either:
  - implement a real weighting policy, or
  - remove `sample_weight` from the contract for now
- candidate weighting policies:
  - class-balance weight
  - recency decay
  - volatility-normalized weight

Recommended default:

- keep the column
- still write `1.0` only if the manifest explicitly declares `sample_weight_policy=constant_one`

### Acceptance criteria

- schema changes are deliberate and versioned
- label and deadband semantics can be recovered from the bundle without reading source code

## 5.5 Workstream E: Harden Bundle And Score-Cache Integrity

### Objective

Catch stale or incompatible artifacts before the desktop consumes them.

### Required changes

#### E1. Expand bundle classification

Files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`

Tasks:

- extend `classify_ml_signal_manifest(...)` to warn on:
  - schema hash mismatch
  - missing calibration declared by manifest
  - unsupported score-contract version
  - stale training date relative to export date
  - missing training-config block

#### E2. Expand score-cache manifest requirements

Files:

- `crates/app_core/src/ml_signal.rs`

Tasks:

- persist:
  - `score_scope`
  - `score_space`
  - `score_contract_version`
  - `source_bundle_id`
  - `source_bundle_manifest_hash`
  - `calibration_state`
  - `threshold_policy`
- validate them when loading caches for validation/policy sweep

#### E3. Improve dataset/timeframe guardrails

Files:

- `src-tauri/src/commands_ml_signal.rs`

Tasks:

- keep existing dataset/timeframe checks
- add clearer errors when:
  - dataset path differs
  - timeframe differs
  - score cache came from calibration-only rows but the user expects full-dataset validation

### Acceptance criteria

- users cannot easily run validation on a mismatched cache without seeing a hard error or explicit warning

## 5.6 Workstream F: Add Baselines, Diagnostics, And Research Gates

### Objective

Make model quality easy to judge and easy to reject.

### Required changes

#### F1. Add simple baselines

Files:

- `research/ml_signal/compare_baseline.py`
- `research/ml_signal/gate_report.py`

Tasks:

- compare CatBoost against at least:
  - a logistic baseline on the same features
  - a simple threshold/rule baseline using a small handpicked subset of features

#### F2. Expand reported metrics

Files:

- `research/ml_signal/gate_report.py`
- `research/ml_signal/calibration_slice_report.py`

Tasks:

- report:
  - ROC AUC
  - average precision / PR summary
  - Brier score
  - log loss
  - calibration bins
  - positive-roll rate
  - positive-ticker rate
  - active-share metrics
- explicitly separate:
  - classifier metrics
  - calibration metrics
  - desktop policy metrics

#### F3. Add explainability artifacts

Files:

- `research/ml_signal/train_catboost.py`
- optional new helper:
  - `research/ml_signal/explain_bundle.py`

Tasks:

- write:
  - global feature importance
  - optional SHAP summaries on a capped sample
  - top suspicious rows / regime examples

### Acceptance criteria

- a reviewer can reject a weak bundle from the report package alone

## 5.7 Workstream G: Improve Desktop-Facing Workflow Clarity

### Objective

Make the UI and persisted artifacts match the true workflow.

### Required changes

Files:

- `web/src/services/mlSignalService.ts`
- relevant ML workspace components and tests
- `REPO_MAP.md`

Tasks:

- update copy so the app is described as:
  - bundle-aware
  - score-cache-driven
  - validation/policy-sweep capable
- avoid implying native CatBoost inference exists if it does not
- surface:
  - score scope
  - calibration state
  - schema hash/version
  - bundle lineage

### Acceptance criteria

- the user can tell, from the UI alone, whether a cache is:
  - calibration-slice only
  - walk-forward research output
  - full-dataset desktop-ready probability cache

## 5.8 Workstream H: Optional Native Inference Exploration

### Objective

Evaluate future native inference paths without committing them to the current improvement cycle.

### Required changes

Files:

- no production file changes required in this phase
- optional research note under `research/`

Tasks:

- compare future options:
  - CatBoost evaluation library via C/C++ API
  - model export targets supported by CatBoost where relevant
  - continued score-cache-first workflow
- estimate:
  - implementation complexity
  - Windows packaging burden
  - CPU/GPU inference cost
  - whether native inference actually improves operator workflow

Decision rule:

- do not begin native inference productization unless repeated usage shows that external score generation is the actual bottleneck

## 5.9 Workstream I: Additive Compatibility, Replay Safety, And Rollback

### Objective

Guarantee that this improvement cycle does not regress current `MlSignal` desktop usage.

### Required changes

#### I1. Keep manifest parsing backward compatible

Files:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`

Tasks:

- treat newly added manifest fields as optional on read
- preserve current required fields until a later deliberate major schema revision
- emit warnings where richer metadata is absent instead of failing immediately
- keep older bundle and score-cache manifests loadable

#### I2. Keep saved-run replay stable

Files:

- `web/src/services/historyService.ts`
- `web/src/services/mlSignalService.ts`
- relevant ML workflow stores/tests

Tasks:

- ensure older ML artifacts still reopen into ML mode
- ensure restored workflow-step behavior stays stable
- ensure new metadata fields do not overwrite restored state with incompatible defaults

#### I3. Use side-by-side artifacts when semantics change materially

Files:

- `research/ml_signal/train_catboost.py`
- `research/ml_signal/evaluate_walkforward.py`
- `research/ml_signal/gate_report.py`

Tasks:

- do not silently repurpose an old filename to mean a materially different artifact
- if semantics change substantially:
  - add a version field, or
  - emit a sibling file with a more precise name

#### I4. Document rollback posture

Files:

- `research/ml_signal/README.md`
- this plan

Tasks:

- describe how to fall back to:
  - prior bundle manifests
  - prior score caches
  - prior proxy research outputs
- make sure the desktop can keep validating from legacy score caches even when richer metadata is absent

### Acceptance criteria

- older `MlSignal` artifacts still load
- newly added metadata is not an accidental hard requirement in the desktop path
- users can continue using the existing score-cache-first workflow while the richer rolling harness lands incrementally

## 6. Recommended Execution Order

### Phase 1: Methodology repair

- Workstream A
- the walk-forward parts of Workstream C

### Phase 2: Contract hardening

- Workstream B
- Workstream D
- Workstream E
- Workstream I

### Phase 3: Reporting and diagnostics

- Workstream F
- Workstream G

### Phase 4: Optional future-runtime exploration

- Workstream H

## 7. File-Level Task Map

### Python research harness

- `research/ml_signal/common.py`
  - return full split windows
  - add embargo/gap helpers
  - add schema-hash helpers
- `research/ml_signal/train_catboost.py`
  - roll-aware training mode
  - explicit training-config block
  - explicit imbalance policy
  - explainability export
- `research/ml_signal/calibrate.py`
  - method selection
  - calibration provenance
- `research/ml_signal/evaluate_walkforward.py`
  - true rolling evaluation
  - strict split-disjointness
  - bundle-level policy metrics
- `research/ml_signal/gate_report.py`
  - separated classifier/calibration/policy gates
- `research/ml_signal/README.md`
  - rewrite workflow language to distinguish proxy vs true walk-forward

### Rust app-core and Tauri

- `crates/app_core/src/ml_signal.rs`
  - richer schema/export manifest
  - improved pattern features
  - explicit sample-weight policy
  - stronger score-cache manifest
  - stronger manifest classification
- `src-tauri/src/commands_ml_signal.rs`
  - clearer context mismatch errors
  - stronger bundle-status response fields
- `src-tauri/src/run_artifacts.rs`
  - keep old `MlSignal` artifact payloads replayable
  - tolerate additive metadata without breaking reopen/apply flows

### Frontend

- `web/src/services/mlSignalService.ts`
  - carry through new manifest fields
  - surface calibration/scope/lineage details
- `web/src/services/historyService.ts`
  - keep legacy ML artifacts reopenable
  - preserve saved-run replay semantics across additive metadata changes
- relevant ML workspace views/tests
  - keep copy aligned with actual runtime scope

## 7.1 Additive Rollout Rules

### Rule A: new fields are additive first

For this cycle:

- add fields before renaming fields
- add fields before removing fields
- do not reuse an existing field name for a substantially different meaning

If semantics change materially, introduce a new field or version the contract explicitly.

### Rule B: readers should accept older artifacts unless safety is at risk

Hard errors are appropriate for:

- dataset mismatch
- timeframe mismatch
- unsupported schema version
- impossible score-cache contract for the requested desktop path

Warnings are appropriate for:

- absent newer provenance fields
- older proxy-only research outputs
- missing-but-recommended explainability or calibration metadata

### Rule C: stage and shell behavior stay stable

The current ML workflow stages should remain recognizable:

- export
- score-cache generation
- validation
- policy sweep

This cycle should enrich those stages, not replace the user's mental model of them.

### Rule D: hidden contract drift is not allowed

If any of the following change, the change must be encoded and surfaced:

- label logic
- deadband logic
- weighting policy
- calibration method
- threshold policy
- split policy

## 7.2 Backward-Compatibility Matrix

### Dataset export artifacts

Current surfaces:

- dataset CSV
- `feature_schema.json`
- `export_manifest.json`
- optional `split_manifest.json`

Compatibility rule:

- new export-manifest fields remain optional for readers
- current path naming keeps working

### Bundle manifests

Current surfaces:

- bundle manifest with model paths, optional calibration path, and score-contract hints

Compatibility rule:

- new bundle metadata may be added
- old bundle manifests must still classify and load, with warnings where richer provenance is absent

### Score-cache manifests

Current surfaces:

- cache JSON plus manifest containing scope/space and coverage metadata

Compatibility rule:

- new lineage/calibration/threshold metadata must be optional on read
- old caches stay valid for the desktop paths they already support

### Saved run artifacts and history replay

Current surfaces:

- workflow artifacts reopened through the ML shell/history path

Compatibility rule:

- older saved runs still reopen into ML mode
- additive metadata must not break workflow-step restoration

## 7.3 Additive Artifact Schema Extensions

This section defines the intended categories of new metadata for this cycle.

### Export manifest additions

- `feature_schema_hash`
- `exporter_version`
- `indicator_config`
- `warmup_policy`
- `label_policy`
- `deadband_policy`
- `sample_weight_policy`
- `data_timeframe`
- `created_at`

### Feature schema additions

- `schema_hash`
- `feature_block_versions`
- `indicator_params`
- `label_horizon_bars`
- `first_valid_reasoning`

### Bundle manifest additions

- `source_export_manifest_hash`
- `source_feature_schema_hash`
- `training_config`
- `training_windows_summary`
- `calibration_summary`
- `class_balance_policy`
- `threshold_policy_default`
- `explainability_artifacts`
- `bundle_parent_id`

### Score-cache manifest additions

- `source_bundle_id`
- `source_bundle_manifest_hash`
- `calibration_state`
- `threshold_policy`
- `score_contract_version`
- `producer_kind`

### Report additions

- explicit `evaluation_mode`
  - `single_fit_proxy`
  - `rolling_walkforward`
- split summary
- calibration method
- threshold policy
- feature schema hash

## 7.4 Implementation Slicing And PR Order

This work should not land as one large patch.

### PR 1: Python methodology repair only

Scope:

- Workstream A
- proxy vs rolling report-language fixes

Constraint:

- no desktop runtime change

### PR 2: Additive manifest and score-cache provenance

Scope:

- Workstreams B, C, D, and E on additive metadata only

Constraint:

- readers remain backward compatible

### PR 3: Frontend and replay hardening

Scope:

- Workstream G
- Workstream I replay/history changes

Constraint:

- preserve current stage model

### PR 4: Diagnostics and explainability

Scope:

- Workstream F
- optional explainability helper(s)

Constraint:

- keep heavy diagnostics optional and sampled

### PR 5: Native inference research note only

Scope:

- Workstream H as research-only follow-up unless separately approved

## 8. Verification Plan

## 8.1 Non-regression verification is mandatory

For any PR that touches desktop-facing code, the minimum repo-local confidence bar should include the standard repo quality gate.

Recommended minimum:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1
```

Increase confidence when the touched scope justifies it:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests
npm --prefix web run test
cargo test --workspace
```

Python-only research PRs do not need the full desktop gate if they do not change desktop code, but they still need artifact-contract compatibility tests.

## 8.2 Python verification

Add a small `research/ml_signal/tests/` suite with coverage for:

- split-window normalization
- no-future-leak roll training
- calibration disjointness
- score-contract compatibility
- schema-hash stability
- legacy proxy mode still working when explicitly requested

Recommended commands:

```powershell
cd research/ml_signal
python -m py_compile train_catboost.py calibrate.py evaluate_walkforward.py gate_report.py
pytest
```

## 8.3 Rust verification

Extend `ml_signal.rs` tests for:

- schema hash/version mismatch
- score-cache scope mismatch
- calibration-state mismatch
- pattern feature extraction semantics
- manifest classification warnings
- legacy manifest loading
- legacy score-cache loading
- additive field absence defaults

Recommended commands:

```powershell
cargo test -p ta_app_core ml_signal
```

## 8.4 Frontend and replay verification

Add targeted tests for:

- `mlSignalService.ts` carrying through new metadata fields
- `historyService.ts` reopening old and new ML artifacts
- UI copy surfacing scope/calibration/lineage without changing the stage model

Recommended commands:

```powershell
npm --prefix web run test
npm --prefix web run typecheck
```

## 8.5 End-to-end verification

Run one full artifact loop:

1. export desktop dataset
2. train bundle
3. calibrate bundle
4. generate walk-forward and calibration reports
5. build score cache
6. run desktop validation
7. run desktop policy sweep

Acceptance checks:

- schema version/hash match at every stage
- score cache coverage and scope are explicit
- desktop artifact reload preserves bundle lineage and score metadata
- old saved runs still reopen successfully
- legacy score caches still validate when richer optional metadata is absent

## 8.6 Performance and operator-safety verification

This cycle is not primarily a performance project, but it still must not make the operator workflow materially worse.

Check:

- bundle classification remains quick enough for model-list refresh
- score-cache loading remains effectively linear in cache size
- new explainability/report files are optional and do not slow the default desktop path
- older artifacts lacking new metadata still render sensibly in the UI

## 9. Risks And Guardrails

### Risk: over-engineering the model before the methodology is fixed

Guardrail:

- do not add major feature families or native inference before Workstream A lands

### Risk: expanding the desktop contract too fast

Guardrail:

- keep the score-cache-first workflow as the stable desktop contract

### Risk: stale artifacts silently passing validation

Guardrail:

- strengthen manifest and score-cache checks before broadening model bundle usage

### Risk: heavy explainability artifacts making bundle generation too slow

Guardrail:

- keep SHAP/object-importance outputs optional and sample-based

### Risk: breaking current saved-run reopen behavior

Guardrail:

- treat history/replay compatibility as a first-class acceptance criterion
- add explicit regression tests for legacy `MlSignal` artifact loading

### Risk: introducing mandatory metadata too early

Guardrail:

- add metadata as optional fields first
- only hard-require new fields later and only when safety demands it

## 9.1 Rollback Strategy

If a new implementation slice causes regressions, the fallback posture should be:

- keep existing desktop score-cache validation behavior
- keep older manifests and caches loadable
- hide or disable only the newly added richer metadata surfaces if necessary
- keep the prior proxy research path available until the rolling path is trusted

Rollback should not require rewriting or deleting existing bundles or saved run artifacts.

## 10. Go/No-Go Criteria

This improvement cycle is successful when all of the following are true:

- walk-forward evaluation is genuinely leakage-safe
- bundle manifests carry enough provenance to reproduce training conditions
- score caches are explicit about scope, calibration state, and bundle lineage
- the desktop UI no longer implies native CatBoost inference if it is not present
- reports cleanly separate ranking quality, calibration quality, and policy performance
- older artifacts still load and replay correctly
- the desktop still works without native CatBoost inference or Python at runtime

This improvement cycle should be considered incomplete if:

- the current proxy walk-forward logic remains the main reported performance artifact
- calibration and threshold provenance are still implicit
- stale bundle/score-cache mismatches can still slip through quietly
- older `MlSignal` artifacts break or reopen incorrectly

## 11. External Sources

### Primary references used for this plan

- [Scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - ordered-data split guidance
  - `gap`, `test_size`, and `max_train_size`
- [Scikit-learn probability calibration user guide](https://scikit-learn.org/stable/modules/calibration.html)
  - disjoint classifier/calibrator guidance
  - sigmoid vs isotonic calibration tradeoffs
- [Scikit-learn calibration example](https://scikit-learn.org/1.2/auto_examples/calibration/plot_calibration_curve.html)
  - practical calibration interpretation
  - Brier/log-loss improvement vs rank stability
- [CatBoost common training parameters](https://catboost.ai/docs/en/references/training-parameters/common)
  - `has_time`
  - `use_best_model`
  - `class_weights`
  - `auto_class_weights`
  - `scale_pos_weight`
- [CatBoostClassifier reference](https://catboost.ai/docs/en/concepts/python-reference_catboostclassifier)
  - estimator behavior
  - metadata support
  - feature-importance and object-importance methods
- [CatBoost metadata access](https://catboost.ai/docs/en/concepts/python-reference_catboostclassifier_metadata)
  - model metadata storage and retrieval
- [CatBoost threshold utilities](https://catboost.ai/docs/en/concepts/python-reference_utils_select_threshold)
  - explicit threshold selection support
- [CatBoost feature importance](https://catboost.ai/docs/en/concepts/output-data_feature-analysis_feature-importance)
  - supported feature-importance outputs
- [CatBoost SHAP values](https://catboost.ai/docs/en/concepts/shap-values)
  - per-object attribution support
- [CatBoost evaluation library](https://catboost.ai/docs/en/concepts/c-plus-plus-api_dynamic-c-pluplus-wrapper)
  - future native inference option

## 12. Final Recommendation

The correct next step for this repo is not to chase native CatBoost runtime integration.

The correct next step is:

1. fix the walk-forward methodology
2. harden the feature/bundle/score-cache contracts
3. improve reporting and diagnostics
4. keep native inference as optional future work only if repeated usage proves it necessary

That path preserves the strongest parts of the current design while addressing the real weaknesses.
