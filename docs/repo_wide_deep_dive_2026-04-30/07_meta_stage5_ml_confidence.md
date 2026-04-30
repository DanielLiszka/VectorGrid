# Meta Selection And Stage 5 ML Confidence Implementation Plan

## Surface

Primary files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaMlScopeBuilder.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Strengths

- Meta selection has meaningful holdout/cohort semantics and artifact integration.
- Stage 5 ML confidence has recent hardening around lineage, warnings, quality gates, and frontend normalization.
- The workflow is positioned as evidence generation rather than a hidden optimizer shortcut.
- Tests already cover a number of stale model and compatibility cases.

## Improvement Plan

### META-01: Stage 5 Training Artifact Reference Contract

Risk: Low/medium.

Problem:
Saved Stage 5 output already has a `Stage5TrainingManifest`, `Stage5TrainingArtifactRef`, `MlConfidenceModelMeta`, and `MlConfidenceCompatibilityKey` path. The improvement is to finish the contract so a loaded model can be classified against a current candidate set without inference from loose strings.

Implementation:

- Extend the existing Stage 5 training/model metadata with:
  - source artifact ID
  - source artifact hash
  - candidate fingerprint
  - data content fingerprint
  - feature schema hash
  - objective/profile version
  - train/validation/holdout split descriptor
- Add stale/equal/future/schema-mismatch compatibility classification to loaded models.
- Carry the compatibility classification through Tauri responses as typed fields, not warning text only.
- Surface classification in frontend summary and artifact detail.

Files:

- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib ml_confidence`
- `cargo test -p ta_desktop_demo_app --lib commands_meta`
- `npm --prefix web run test -- src/services/metaService.test.ts`

### META-02: Strict Health Must Distinguish Train-Only Evidence

Risk: Low.

Problem:
The current flow warns when it has to fall back to train metrics. That is useful, but strict health and saved artifacts should carry an explicit evidence tier so train-only evidence can never look equivalent to cross-validation, holdout, or external evidence.

Implementation:

- Add explicit evidence levels:
  - train only
  - cross-validation
  - holdout
  - external/cross-ticker
- Make strict health fail or warn separately for train-only evidence.
- Store the evidence tier in model metadata, policy evaluation output, artifact summaries, and frontend normalized result shapes.
- Add UI copy that explains the evidence tier without encouraging false confidence.

Files:

- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib ml_confidence`
- `cargo test -p ta_desktop_demo_app --lib commands_meta`
- `npm --prefix web run test -- src/services/metaService.test.ts`

### META-03: Apply-Time Compatibility Recheck

Risk: Low/medium.

Problem:
Model compatibility can be checked during load/training, but applying a model to a new candidate set should repeat the important compatibility checks.

Implementation:

- In `apply_ml_confidence_model`, re-check:
  - feature schema hash
  - candidate schema
  - objective profile
  - artifact lineage
  - stale/future/equal status
- Add exact request/result fields that report these compatibility facts for the apply operation.
- Return a structured rejection or warning rather than silently applying.
- Preserve existing permissive behavior only behind an explicit research flag.

Files:

- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/ml_confidence.rs`
- `web/src/services/metaService.ts`

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_meta`
- `cargo test -p ta_app_core --lib ml_confidence`
- `npm --prefix web run test -- src/services/metaService.test.ts`

### META-04: Policy Evaluation Detail Table

Risk: Low.

Problem:
Summary-level policy evaluation can hide whether a model is improving the right rows or only filtering easy cases.

Implementation:

- Extend the existing policy evaluation structs and `commands_meta.rs` retained/rejected summaries with:
  - retained vs rejected counts
  - retained mean/median outcome
  - rejected mean/median outcome
  - calibration buckets
  - confidence intervals where cheap
  - evidence tier
- Include separate rows for important confidence-threshold arms so the table does not collapse threshold behavior into one headline result.
- Keep the current summary fields for compatibility.

Files:

- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib ml_confidence`
- `cargo test -p ta_desktop_demo_app --lib commands_meta`
- `npm --prefix web run test -- src/services/metaService.test.ts`

### META-05: Stage 5 Saved Summary Expansion

Risk: Low.

Problem:
The saved artifact summary should show the evidence state without requiring users to open raw JSON.

Implementation:

- Extend the existing `Stage5ArtifactSummary` in `src-tauri/src/run_artifacts.rs` with compact fields:
  - model ID
  - training artifact ID
  - evidence tier
  - strict health verdict
  - stale status
  - warning count
  - top rejection reason
- Keep the full detail in the artifact payload.

Files:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `cargo test -p ta_desktop_demo_app --lib commands_meta`
- `npm --prefix web run test -- src/services/historyService.test.ts`

### META-06: Stage 3 Evidence Policy Summary

Risk: Low.

Problem:
Stage 5 already has an `evidence_policy_summary`, and several frontend stores can normalize it. Stage 3 `MetaSelectionResult` does not yet expose the same explicit evidence-policy language, so Meta selection can look less audited than newer ML surfaces.

Implementation:

- Add `evidence_policy_summary` to Stage 3 results.
- Include holdout availability, cohort coverage, stale data warnings, and pass/fail reasons.
- Hydrate the new field through `commands_meta.rs`, normalize it in `metaService.ts`, and surface it in Meta and History panels.

Files:

- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Verification:

- `cargo test -p ta_app_core --lib meta_select`
- `cargo test -p ta_desktop_demo_app --lib commands_meta`
- `npm --prefix web run test -- src/services/metaService.test.ts`

## Acceptance Criteria For This Guide

- Stage 5 artifacts identify the training artifact, candidate fingerprint, data fingerprint, feature schema, and split descriptor.
- Strict health distinguishes train-only, cross-validation, holdout, and external/cross-ticker evidence tiers.
- Apply-time compatibility checks can reject stale/future/schema-mismatched models before scoring.
- Existing Stage 3 and Stage 5 response fields remain backward-compatible.
- UI summaries expose evidence tier and warning counts without implying live-trading approval.

## Deferred Higher-Risk Work

- Changing model selection defaults or thresholds.
- Treating ML confidence as a trade execution signal.
- Expanding feature engineering without new leakage tests.
