# OX-07 Implementation Guide

Source response: `ox07_fill_slippage_execution_review.md`

Decision: `Valid`, but only for bounded realism patches inside the current ownership surfaces.

## Valid Scope
- [ ] Make spread/staleness gating real and optional by extending `OptionsStaticFilterBundle` and consuming existing `quote_age_ms`/`SpreadTooWide` in `select_candidate`.
- [ ] Extract a shared barrier-resolution helper in `options_expression.rs` so scalar and compiled paths stay aligned.
- [ ] Add live safety checks in `commands_execution.rs` to block or reconcile when open orders are present before computing a fresh `delta_qty`.

## Non-Goals
- [ ] Do not reinterpret existing `DerivedMidSnapshot` defaults in place without an explicit compatibility step.
- [ ] Do not add a broad microstructure fill model that current inputs cannot support.
- [ ] Do not change same-bar exit policy before centralizing it in one helper.
- [ ] Do not force a deadband-to-probability conversion in `ml_signal.rs`.
- [ ] Do not replace the current one-shot live bracket flow with post-fill bracket placement as the first patch.

## Additive Guardrails
- [ ] Current options semantics must remain available unless a new opt-in gating field is enabled.
- [ ] Scalar and compiled paths must stay behaviorally aligned.
- [ ] Existing pending-bar “latest wins” runner behavior must remain unchanged.

## Primary Touchpoints
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/ml_signal.rs`

## Implementation Checklist
- [ ] Extend `OptionsStaticFilterBundle` with optional spread/staleness gating that is default-off or compatibility-preserving.
- [ ] Consume existing `quote_age_ms` and `SpreadTooWide` signals in `select_candidate` when the new gate is enabled.
- [ ] Extract one shared same-bar barrier-resolution helper and route both scalar and compiled logic through it.
- [ ] In `commands_execution.rs`, check open orders before recomputing a fresh market delta and block/reconcile if one is active.
- [ ] Keep live bracket anchoring and broader fill-model redesign out of the first patch unless explicitly separated.
- [ ] Keep any ML-signal change diagnostic-first, such as threshold brittleness or uncertainty margin reporting.

## Verification Checklist
- [ ] Add candidate rejection tests for spread and quote-age thresholds.
- [ ] Add scalar-vs-compiled barrier-resolution parity tests.
- [ ] Add live execution tests that refuse or reconcile submission with outstanding open orders.
- [ ] Add no-change regressions for the current pending-bar latest-wins behavior.

## Rollout Checklist
- [ ] Ship spread/staleness gating as optional/compatibility-preserving first.
- [ ] Land the barrier helper before changing any branch policy.
- [ ] Keep live-order safety checks additive and explicit in diagnostics.

## Kill Criteria
- [ ] Reject if scalar and compiled paths diverge after the barrier helper refactor.
- [ ] Reject if the patch silently changes current options semantics without an explicit compatibility path.

