# OX-08 Implementation Guide

Source response: `ox08_all_passes.md`

Decision: `Valid`, but only for local calibration/boundary fixes and diagnostics-first geometry improvements.

## Valid Scope
- [ ] Fix `ml_confidence.rs` by separating public row provenance from CV grouping, so folds key off meta-run identity rather than duplicated dataset rows.
- [ ] Add a retention buffer before exactify/final truncation in `options_expression.rs` and optionally record provisional-vs-final robust-score drift.
- [ ] Add an explicit missing-value policy wrapper in `validation_stats.rs` while preserving current default behavior.
- [ ] Add boundary-mass diagnostics in `ml_signal.rs` before any policy-geometry rewrite.

## Non-Goals
- [ ] Do not attempt a repo-wide calibration redesign.
- [ ] Do not immediately replace the current `score_target` wedge in `ml_signal.rs`.
- [ ] Do not add learned backend/stress models in `options_expression.rs` as the first slice.
- [ ] Do not silently change existing `passes_probability_gates` semantics.

## Additive Guardrails
- [ ] Existing CV, exactify, and gate behavior must remain the default unless the new diagnostics or wrappers are explicitly used.
- [ ] Missing-value handling must become more explicit without changing current defaults silently.
- [ ] Boundary-mass diagnostics must be descriptive first.

## Primary Touchpoints
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/ml_signal.rs`

## Implementation Checklist
- [ ] In `ml_confidence.rs`, separate training/CV grouping identity from duplicated public dataset-row identity.
- [ ] Keep current “need at least 2 unique meta runs” behavior, but key it on true meta-run grouping.
- [ ] In `options_expression.rs`, keep a retention buffer before exactify/final truncation and expose provisional-vs-final drift telemetry if helpful.
- [ ] In `validation_stats.rs`, add a wrapper or explicit policy seam for missing-value handling while preserving current defaults.
- [ ] In `ml_signal.rs`, add diagnostics for near-boundary mass or boundary distance before considering any threshold-geometry rewrite.

## Verification Checklist
- [ ] Add CV-grouping tests proving one artifact with many dataset rows no longer fakes multi-group CV.
- [ ] Add retained-buffer tests that preserve post-exactify winners and capture drift telemetry.
- [ ] Add wrapper parity tests proving existing default gate behavior is unchanged.
- [ ] Add decision-boundary or near-boundary-mass diagnostics tests for ML candidates.

## Rollout Checklist
- [ ] Ship CV grouping and exactify buffering before any broader policy-geometry change.
- [ ] Keep missing-value policy wrappers compatibility-preserving first.
- [ ] Treat ML boundary diagnostics as observability until a later patch justifies using them in decisions.

## Kill Criteria
- [ ] Reject if CV grouping still treats duplicated dataset rows as distinct validation groups.
- [ ] Reject if the patch silently changes existing probability-gate semantics or threshold defaults.
