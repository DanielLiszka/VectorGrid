# PR-07 Implementation Guide

Source response: `PR-07_all_passes.md`

Decision: `Valid`, but only for compatibility-contract hardening and missingness preservation.

## Valid Scope
- [ ] Add a finer ML-confidence compatibility contract and key than normalized `(strategy_kind,timeframe)`.
- [ ] Distinguish pooled-history compatibility by train-score semantics, grouping mode, shortlist/fallback policy, and target family.
- [ ] Preserve missingness explicitly so `cross_score_median` missing is not aliased to `0.0`.

## Non-Goals
- [ ] Do not claim exact group-local train ranking from current aggregates alone.
- [ ] Do not present current `Auto` target semantics as exact or stationary.
- [ ] Do not redesign PR-06 validation semantics inside this patch.

## Additive Guardrails
- [ ] Existing compatible history pools must keep working.
- [ ] New compatibility fields must be additive and backward-compatible on load.
- [ ] Missingness preservation must not silently convert old zero values into missing.

## Primary Touchpoints
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

## Implementation Checklist
- [ ] Define an expanded compatibility key that includes at least target family, train-score semantics, grouping mode, and shortlist/fallback policy.
- [ ] Apply that key consistently wherever pooled ML-confidence history is loaded or merged.
- [ ] Preserve missingness distinctly in stored summaries and ranking inputs, especially for `cross_score_median`.
- [ ] Keep old artifacts loadable by treating absent new fields as “unknown/legacy,” not as compatible-by-default.
- [ ] Keep `Auto` target decisions labeled as heuristic/history-prefix-dependent where surfaced.

## Verification Checklist
- [ ] Add history-pooling tests that reject incompatible score semantics and target families.
- [ ] Add serialization tests for the expanded compatibility key.
- [ ] Add regression tests proving missing `cross_score_median` is not treated as numeric zero.
- [ ] Add backward-compatibility tests for older saved artifacts.

## Rollout Checklist
- [ ] Add docs/status text describing why a historical artifact is considered incompatible.
- [ ] Keep previous successful compatible pools unchanged when the new fields match.
- [ ] Defer any train-ranking formula changes to a separate patch.

## Kill Criteria
- [ ] Reject if incompatible historical artifacts can still pool into the same ML-confidence dataset.
- [ ] Reject if missingness continues to collapse to zero in any ranking-critical field.

