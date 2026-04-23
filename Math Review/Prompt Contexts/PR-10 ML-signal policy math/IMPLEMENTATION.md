# PR-10 Implementation Guide

Source response: `pr10_ml_signal_policy_math_all_passes.md`

Decision: `Valid`, but only for request-word contract hardening and explicit exhaustive-sweep semantics.

## Valid Scope
- [ ] Document the `ML_SIGNAL_NO_REQUEST` versus `0` or flat contract because they produce different downstream behavior in `fill_requested_targets` and `fill_desired_positions`.
- [ ] Document current ML-signal sweep semantics as: evaluate all candidates, sort by robust score, truncate to `top_k`, then rebuild the verdict on the retained shortlist.
- [ ] Keep the overlay-neutralization contract explicit between `ml_signal.rs` and `iterative_sim.rs`.

## Non-Goals
- [ ] Do not claim any pruning guarantee on the current sweep surface.
- [ ] Do not claim approval is a pure function of desired-position path alone.
- [ ] Do not redesign the search into a non-exhaustive policy layer here.

## Additive Guardrails
- [ ] Current exhaustive-sweep behavior must remain unchanged.
- [ ] Request-word semantics must be clarified without changing existing defaults.
- [ ] Overlay-neutralization must stay explicit and test-backed.

## Primary Touchpoints
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`

## Implementation Checklist
- [ ] Name the request-word states explicitly in code comments, docs, and artifacts where they surface.
- [ ] Make the sentinel/no-request path distinct from zero or flat target requests in every ranking-critical path.
- [ ] Document the exact ownership of the exhaustive sweep, robust-score ordering, `top_k` truncation, and verdict rebuild.
- [ ] Keep overlay-neutralization explicit at the interface boundary and avoid implicit duplication of execution semantics.
- [ ] Ensure downstream readers do not infer pruning or full-sweep approval semantics that the current pipeline does not provide.

## Verification Checklist
- [ ] Add tests covering `ML_SIGNAL_NO_REQUEST` versus zero or flat request behavior.
- [ ] Add regression tests for exhaustive sweep followed by `top_k` truncation semantics.
- [ ] Add interface tests confirming overlay-neutralized execution assumptions stay aligned.
- [ ] Add artifact/report tests that preserve request-word distinctions.

## Rollout Checklist
- [ ] Update docs/status text first, then artifact fields if needed.
- [ ] Keep current search behavior unchanged unless a later patch explicitly introduces a new mode.
- [ ] Prefer additive diagnostics over any behavior change in this guide.

## Kill Criteria
- [ ] Reject if any doc or code path starts describing the current sweep as pruned.
- [ ] Reject if approval semantics ignore gate-layer state such as `actual_score_obs`.
