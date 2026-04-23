# OX-01 Implementation Guide

Source response: `ox01_sensitivity_review_answers.md`

Decision: `Valid`, but only for additive enrichment of existing local-neighborhood summaries.

## Valid Scope
- [ ] Extend existing `double_ma` local-stability and neighborhood-robustness outputs with axiswise finite-difference fields computed from the already materialized neighborhood.
- [ ] Add exact-on-the-shared-neighborhood fields such as one-step `fast/slow` deltas, centered curvatures, anisotropy, dominant fragile axis, and optional axis half-widths.
- [ ] Add only cheap proxy telemetry derived from already available metrics.
- [ ] If mirrored elsewhere, limit that to already-supported composite local-stability paths and keep the semantics labeled as common-start local geometry.

## Non-Goals
- [ ] Do not add a new cross-repo sensitivity trait/framework.
- [ ] Do not claim exact raw-lattice derivatives on non-smooth staircase/gated surfaces.
- [ ] Do not require full trace materialization for every neighbor just to detect path equality.

## Additive Guardrails
- [ ] Existing winner selection and existing local replay behavior must remain unchanged.
- [ ] New fields must be descriptive only unless a later patch deliberately promotes them into decision logic.
- [ ] Unsupported composite local-stability paths must remain unsupported rather than widened opportunistically.

## Primary Touchpoints
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/options_expression.rs`

## Implementation Checklist
- [ ] Extend `ShortlistNeighborhoodRobustnessSummary` in `double_ma.rs` with exact common-start neighborhood geometry fields.
- [ ] Compute those fields only from the already replayed neighborhood; do not trigger extra backtests.
- [ ] Add an explicit label or flag for ridge/cliff or axis-dominant fragility where the current scalar blend can hide anisotropy.
- [ ] Keep any path-change telemetry proxy-like and descriptive, using existing metrics instead of new full-path retention.
- [ ] If composite mirroring is later desired, restrict it to already-supported composite local-stability routes.

## Verification Checklist
- [ ] Add synthetic neighborhood tests with known ridge/cliff behavior.
- [ ] Add regression tests proving summary enrichment does not change winner selection.
- [ ] Add serialization/backward-compat tests for enriched neighborhood summaries.
- [ ] Add telemetry tests proving any prescan/local-coverage fields remain descriptive only.

## Rollout Checklist
- [ ] Surface the new fields as “local neighborhood geometry” rather than generic sensitivity derivatives.
- [ ] Keep current scalar plateau score and current labels intact for compatibility.
- [ ] Defer any UI or ranking use of these fields until the descriptive semantics are stable.

## Kill Criteria
- [ ] Reject if the patch widens ownership into a cross-repo sensitivity framework.
- [ ] Reject if the new fields are described as smooth derivatives or full-grid geometry.

