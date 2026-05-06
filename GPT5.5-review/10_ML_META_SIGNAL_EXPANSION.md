# Meta, Stage 5, And ML Signal Expansion Plan

Date: 2026-04-30

## Goal

Expand ML-adjacent workflows by improving evidence, compatibility, score-cache rigor, model-bundle provenance, and validation review. The goal is not to make ML claims stronger than the evidence supports.

## Current Owner Files

Meta and Stage 5:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaMlScopeBuilder.ts`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

ML Signal:

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalShellPanel.vue`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `scripts/train_ml_signal_catboost.py`
- `scripts/run_ml_signal_quality_gate.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`

## Research Basis

- Backtest overfitting and false discovery research applies strongly to ML and meta-selection workflows that search candidate/model spaces: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
- NIST AI RMF is a useful framework for documenting model validity, transparency, and monitoring limits without overstating model assurance: <https://www.nist.gov/itl/ai-risk-management-framework>
- Model Cards and Datasheets for Datasets provide practical documentation patterns for model/data provenance and intended-use boundaries: <https://arxiv.org/abs/1810.03993> and <https://arxiv.org/abs/1803.09010>
- scikit-learn's model-persistence guidance highlights the security and compatibility risks of loading serialized model artifacts, which applies to any future model-registry workflow: <https://scikit-learn.org/stable/model_persistence.html>
- NIST SSDF supports treating model/native-bundle provenance, dependency hygiene, and release evidence as part of secure software development: <https://csrc.nist.gov/publications/detail/sp/800-218/final>
- Serde additive schema tools support optional evidence fields for older artifacts: <https://serde.rs/attributes.html>

## Expansion Track A: Stage 5 Evidence Tiering

### Problem

Stage 5 ML confidence can look authoritative unless users can see training scope, compatibility, and apply-time checks.

### Implementation

Current owners:

- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Add evidence tier:

- `review_only`;
- `train_validated`;
- `apply_compatible`;
- `apply_incompatible`;
- `native_unavailable`;
- `blocked`.

Add metadata:

- training artifact references;
- feature schema version;
- candidate universe identity;
- train-only health fields separated from apply-time fields;
- model/native backend status;
- fallback reason.

### Tests

- legacy Stage 5 artifact without new fields remains readable;
- apply-time compatibility rejects mismatched feature schema;
- native unavailable path produces explicit evidence tier.

## Expansion Track B: Meta Selection Cohort And Holdout Clarity

### Problem

Meta selection can combine candidates across strategy/context boundaries. Expansion should make cohort identity explicit.

### Implementation

Current owners:

- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Add:

- cohort id;
- strategy family;
- feature context;
- run context;
- holdout mode;
- score weights;
- candidate source artifacts;
- rejected-candidate reason counts.

### Acceptance Criteria

- Meta summary does not depend only on MA labels when richer context exists.
- Legacy MA fallback remains compatible.
- Reports and artifacts can explain why a candidate was selected.

## Expansion Track C: ML Signal Score Cache Strict Mode

### Problem

ML Signal score-cache reuse is valuable but can become a source of stale evidence unless contract mode is visible.

### Implementation

Current owners:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

Add strict mode fields:

- feature schema hash;
- model bundle hash;
- dataset split plan;
- score scope;
- retained/full score coverage;
- native backend status;
- compatibility verdict.

Expose strict score-cache mode in Advanced ML Signal only. Do not make it the default until existing artifacts and workflows are proven compatible.

### Tests

- cache mismatch blocks reuse under strict mode;
- fallback mode labels score-cache reuse as compatible or best-effort;
- full-vs-retained dual verdict displays without changing ranking defaults.

## Expansion Track D: Model Bundle And Native Library Provenance

### Problem

ML Signal has native CatBoost-related release resources. Expansion needs stronger manifest evidence and missing-native behavior.

### Implementation

Current owners:

- `crates/app_core/src/ml_signal.rs`
- `scripts/train_ml_signal_catboost.py`
- `scripts/run_ml_signal_native_smoke.ps1`
- `scripts/generate_release_manifest.ps1`
- `docs/PUBLIC_RELEASE.md`

Add:

- model bundle manifest schema version;
- native library identity and hash;
- training script version/provenance;
- inference backend label;
- missing-native explicit unsupported/fallback result;
- release manifest check for native resources by variant.

### Tests

- native smoke when configured;
- CPU-only missing-native path remains user-readable;
- release manifest includes or explicitly allows missing native resource depending on variant.

## Expansion Track E: Policy Sweep Reporting

### Problem

Policy sweeps can produce many configurations. Users need concise outcome summaries.

### Implementation

Current owners:

- `crates/app_core/src/ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

Add:

- sweep axis summary;
- best retained score vs full score;
- policy stability indicator;
- rejected policy count;
- low-sample warning;
- exportable sweep table.

### Acceptance Criteria

- No policy default changes.
- Sweep output is deterministic and testable.
- Report/export shape changes use output parity.

## Second-Pass Expansion: Calibration And Threshold Stability

### Problem

ML confidence and policy-sweep output can be misleading if users see a single score without calibration and threshold stability context.

### Current Anchors

- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.vue`

### Implementation

Add optional calibration fields:

- calibration bucket count;
- per-bucket predicted score range;
- observed retained/full performance summary;
- sample count per bucket;
- empty/low-sample bucket warnings;
- threshold sweep range;
- stable-threshold interval;
- chosen threshold provenance.

UI:

- show a compact calibration table only in Advanced ML views;
- label unavailable calibration as `not computed`;
- keep Stage 5 and ML Signal calibration separate unless the feature schemas match exactly.

Acceptance criteria:

- no score or policy default changes in this phase;
- calibration fields are optional for legacy artifacts;
- threshold stability is evidence text, not an automatic approval.

## Second-Pass Expansion: Stage 3 Evidence Policy And Policy-Eval Details

### Problem

The prioritized program omitted ML/meta from the first-pass execution order. Stage 3 and policy-evaluation surfaces should become explicit so ML expansion does not lag behind artifact and validation contracts.

### Implementation

Add a Stage 3 evidence-policy summary:

- source candidate universe;
- training/validation split scope;
- policy-eval row count;
- rejected policy reasons;
- retained-vs-full score delta;
- feature-schema hash;
- model bundle hash where applicable;
- native backend availability.

Add policy-eval detail tables:

- policy id;
- threshold or rule parameters;
- retained count;
- full count;
- primary score;
- stability label;
- rejection reason;
- compatibility verdict.

Verification:

- policy-eval tables sort deterministically;
- missing feature schema blocks strict reuse;
- native unavailable state is explicit and non-fatal for CPU-only workflows.

## Second-Pass Expansion: ML Signal Browser Workflow Gate

### Problem

ML Signal has CLI/native gates, but the browser workflow also needs proof that the user can load, inspect, and understand ML results.

### Implementation

Add a focused frontend QA lane:

- fixture ML Signal artifact with no native dependency;
- score-cache compatible artifact;
- score-cache incompatible artifact;
- missing-native state;
- policy sweep summary table;
- calibration empty/low-sample states.

Candidate scripts:

- add a browser QA spec under `web/e2e/browser/qa/`;
- wire it into `docs/feature_expansion_implementation_plans_2026-04-30/15_frontend_qa_gate_wiring_matrix.md`;
- keep it out of default release gate until fixture cost and runtime are stable.

## Non-Goals

- No production trading claims.
- No auto-training on user data without explicit command and artifact.
- No hidden network dependency for model inference.
- No changing optimization ranking based on ML confidence until a separate evidence-backed plan approves it.

## Verification

Targeted:

- `npm --prefix web run test -- src/services/metaService.test.ts src/services/metaRequestBuilder.test.ts src/services/metaStage4RequestBuilder.test.ts src/services/mlSignalService.test.ts src/components/MetaWorkspacePanel.test.ts src/components/MlSignalWorkspacePanel.test.ts`
- `cargo test -p ta_app_core ml_confidence`
- `cargo test -p ta_app_core ml_signal`

ML-specific:

- `powershell -File scripts/run_ml_signal_quality_gate.ps1`
- `powershell -File scripts/run_ml_signal_native_smoke.ps1` with required local args/resources.

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
