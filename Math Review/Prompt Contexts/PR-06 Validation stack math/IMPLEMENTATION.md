# PR-06 Implementation Guide

Source response: `pr06_validation_stack_math_all_passes.md`

Decision: `Valid`, but only for stage-indexed replay/provenance hardening and falsifier coverage.

## Valid Scope
- [ ] Document validation as a stage-indexed replay/provenance contract.
- [ ] Make score-channel provenance, `passed` versus `approved`, CSCV provenance, cross-mode provenance, and family-tagged regime semantics explicit.
- [ ] Treat regime summaries as descriptive unless separately promoted into a decision contract.
- [ ] Add falsifier coverage for order sensitivity, `passed=true` with `minTRL=None`, CSCV zero-selection handling, and success-only cross-summary limits.

## Non-Goals
- [ ] Do not collapse the full verdict stack into one scalar score.
- [ ] Do not present success-only cross summaries as full-panel robustness.
- [ ] Do not redesign unrelated validation families outside this stage/provenance contract.

## Additive Guardrails
- [ ] Existing validation outputs must remain available.
- [ ] Replay-backed exact paths must remain the reference behavior.
- [ ] New provenance fields must augment, not replace, current outputs.

## Primary Touchpoints
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

## Implementation Checklist
- [ ] Enumerate the validation stages explicitly: local path evidence, probability metrics, CSCV/PBO layer, cross-ticker aggregation, final verdict rebuild.
- [ ] For each stage, list the retained state required for exact replay versus descriptive-only summaries.
- [ ] Add explicit provenance fields for score channel, probability-metric source, CSCV mask/coverage state, cross-mode type, and regime summary family.
- [ ] Keep `passed` and `approved` separate in artifacts and reporting when they can diverge.
- [ ] Mark regime summaries as descriptive unless a later patch deliberately promotes them into gating.
- [ ] Prevent downstream report or UI layers from inferring stronger semantics than the retained state supports.

## Verification Checklist
- [ ] Add replay-order regression tests across the stage boundary.
- [ ] Add tests where `passed` and `approved` diverge.
- [ ] Add CSCV tests for zero-selection and asymmetric missingness cases.
- [ ] Add cross-ticker tests proving success-only summaries do not imply full-panel robustness.
- [ ] Add report/artifact tests proving provenance fields survive serialization.

## Rollout Checklist
- [ ] Update validation/report documentation to describe stage-indexed semantics.
- [ ] Keep descriptive summaries visible, but label them clearly as non-gating where appropriate.
- [ ] Avoid changing ranking or approval thresholds in the same patch unless separately justified.

## Kill Criteria
- [ ] Reject any patch that reintroduces one-scalar explanations for the verdict stack.
- [ ] Reject any artifact/report change that hides the difference between descriptive and decision-critical state.

