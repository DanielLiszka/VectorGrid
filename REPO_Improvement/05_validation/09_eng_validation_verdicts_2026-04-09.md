# Validation Verdict Engine Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the backend validation-verdict logic so candidate-level gates, run-level approval, recommendation text, and robustness-method folding remain coherent, explainable, and stable across strategy families.

## Scope Boundaries

In scope:

- `crates/app_core/src/validation_pipeline.rs`
- candidate hard-gate application
- CSCV/PBO, SPA, track-record, and recommendation folding
- run-level verdict construction consumed by validation command surfaces

Out of scope:

- frontend rendering details
- request-builder control shaping
- options-related validation logic

## Current User-Facing Behavior

The backend does more than label candidates pass/fail. It computes probability-style signals, optional CSCV/PBO statistics, optional SPA statistics, track-record bars and minimum-track-record requirements, candidate hard-gate failures, robust-score folding, and then a run-level verdict with fields such as `passed`, `approved`, `approved_candidate_count`, `final_recommendation`, and track-record metadata for the top candidate.

The frontend already surfaces pieces of this engine through shortlist meta text, route/warning summaries, row-level gate markers, and recommendation/status language. That means changes in backend verdict logic can materially change what users infer from the validation tab, even if the UI itself is untouched.

## Current Code Surface

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationService.ts`

## Current Verification Surface

- `crates/app_core/src/validation_pipeline.rs` tests
- `src-tauri/src/commands_validation.rs` tests
- `web/src/services/validationService.test.ts`

## Strengths Already Present

- the repo already centralizes run-verdict construction instead of reimplementing it per strategy
- track-record and minimum-track-record signals are already explicit
- CSCV and SPA are already integrated as optional but first-class contributors to the final recommendation
- run-verdict fields already expose enough structure for the frontend to explain outcomes
- there is already backend coverage for important cases such as min-track-record failures and post-cross-filter recomputation

## Weaknesses / Failure Modes

- verdict construction is mathematically and operationally dense, making silent drift hard to detect
- candidate hard-gate failure reasons and run-level recommendation strings can diverge if they are not treated as one contract
- optional-method availability matters:
  - requested-but-unavailable CSCV
  - skipped SPA
  - track-record not met
- frontend summary text can imply stronger guarantees than the backend really provides if field meanings are not frozen explicitly

## Regressions To Avoid

- changing `final_recommendation` semantics without updating downstream summary logic
- making run-level approval inconsistent with candidate-level hard-gate outcomes
- weakening track-record or CSCV/PBO handling while still emitting the old recommendation language
- failing to recompute verdicts after cross-ticker finalist filtering
- turning optional-method absence into silent success

## External Research Summary

- [TimeSeriesSplit — scikit-learn](https://scikit-learn.org/1.0/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because chronological split discipline is the baseline external reference for time-ordered validation behavior.
- [The Probability of Backtest Overfitting — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253) is relevant because CSCV/PBO is the primary external framing for the repo’s overfitting-risk logic.
- [A Test for Superior Predictive Ability — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=264569) is relevant because SPA significance is part of the repo’s robustness-method stack and should remain grounded in a real test concept rather than generic warning text.
- [The Sharp Razor / Minimum Track Record Length](https://www.researchgate.net/publication/256035273_The_Sharp_Razor_Deflating_the_Sharpe_Ratio_by_Asking_for_a_Minimum_Track_Record_Length) is relevant because the repo already computes minimum track-record requirements and uses them in run-level recommendations.
- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because returned verdict/result objects are effectively a structured contract consumed across crates and the frontend.

## Proposed Improvement Approach

1. Document the verdict engine as a layered pipeline:
   - candidate metrics
   - optional robustness methods
   - hard gates
   - folded robust score
   - run-level approval/recommendation
2. Freeze the meaning of key run-verdict fields and recommendation strings.
3. Keep optional-method absence explicit so “requested but unavailable” and “skipped” remain observable outcomes.
4. Align frontend summary text to the documented backend meaning instead of treating it as free-form copy.
5. Expand tests around representative recommendation outcomes and post-filter recomputation.

## Checklist

- [ ] Document the ordered stages of verdict construction from candidate metrics to run recommendation.
- [ ] Document the meaning of `passed`, `approved`, `approved_candidate_count`, and `final_recommendation`.
- [ ] Document how track-record and minimum-track-record fields affect approval and recommendation.
- [ ] Document how CSCV/PBO and SPA influence warnings, failures, and recommendation language.
- [ ] Document how post-cross-filter verdict recomputation must behave.
- [ ] Add or tighten backend tests for representative recommendation outcomes.
- [ ] Add or tighten frontend summary tests for recommendation and warning parity.

## Verification Plan

- run targeted `validation_pipeline.rs` tests
- run targeted `commands_validation.rs` tests
- run `npm --prefix web test -- src/services/validationService.test.ts`
- manually inspect one validation run each for:
  - no-survivor outcome
  - single-supported outcome
  - min-track-record-not-met outcome
  - cross-filtered verdict recomputation

## Rollout / Fallback / Compatibility Notes

- preserve current verdict-field names unless a coordinated schema change is made
- prefer additive explanation over changing recommendation semantics casually
- if recommendation tokens change, update frontend summary tests and artifact consumers immediately
- keep optional robustness modules non-silent even when disabled or unavailable

## Related Docs

- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/26_cross_ticker_finalist_logic_2026-04-09.md`
- `REPO_Improvement/05_validation/27_validation_robustness_methods_2026-04-09.md`
