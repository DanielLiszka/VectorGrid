# OX-06 Implementation Guide

Source response: `ox06_tail_risk_review.md`

Decision: `Valid`, but only for producer-side retention fixes, additive tail summaries, and bounded exactify buffering.

## Valid Scope
- [ ] Stop zeroing already-observable downside and persistence fields in `iterative_sim.rs`.
- [ ] Extend `validation_pipeline.rs` with ES/tail-mean summaries from existing bootstrap vectors before considering any score changes.
- [ ] In `options_expression.rs`, retain a bounded pre-final-truncation buffer, exactify it, and rerank before the final `top_k` cut so tail-aware exact replay can affect selection.

## Non-Goals
- [ ] Do not redesign `composite_gpu_metric_decode.rs`; it already accepts optional tail fields.
- [ ] Do not immediately widen `options_expression_gpu.rs` if CPU-side exactify buffering solves the ranking alias first.
- [ ] Do not add a new replay subsystem.

## Additive Guardrails
- [ ] Existing simulation semantics must remain unchanged.
- [ ] New fields and summaries must be additive before any score widening.
- [ ] Current GPU compact output must remain supported unless a later patch deliberately widens it.

## Primary Touchpoints
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`

## Implementation Checklist
- [ ] In `iterative_sim.rs`, export persistence and downside-frequency/severity fields already visible in the current bar/path walk.
- [ ] In `validation_pipeline.rs`, add additive ES/tail-mean summaries from bootstrap/walk-forward vectors already materialized.
- [ ] Promote warning flags first if helpful, before widening robust-score formulas.
- [ ] In `options_expression.rs`, keep a bounded surplus shortlist before final truncation, exactify it, then apply the final `top_k` cut.
- [ ] Keep `options_expression_gpu.rs` stable for the first patch unless CPU-side exactify buffering proves insufficient.
- [ ] Leave `composite_gpu_metric_decode.rs` unchanged unless an upstream producer later emits new tail fields.

## Verification Checklist
- [ ] Add tests proving non-zero exported persistence/downside fields from iterative replay.
- [ ] Add deterministic bootstrap ES/tail summary tests alongside existing quantiles.
- [ ] Add pre/post exactify rank-flip fixtures for options selection.
- [ ] Add decoder coverage for both current compact and widened optional tail-row shapes if any producer changes later.

## Rollout Checklist
- [ ] Start producer-side in `iterative_sim.rs` before any downstream scoring work.
- [ ] Keep tail summaries descriptive-first if score widening is deferred.
- [ ] Treat options exactify buffering as a bounded shortlist-quality fix, not a new search mode.

## Kill Criteria
- [ ] Reject if downstream tail-aware changes are added while producer-side fields are still zeroed.
- [ ] Reject if exactify buffering is introduced in a way that weakens current exact replay behavior.

