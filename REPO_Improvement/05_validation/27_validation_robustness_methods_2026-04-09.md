# Validation Robustness Methods Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the validation robustness-method stack so walk-forward, re-opt walk-forward, perturbation, trade bootstrap, CSCV/PBO, SPA, and related gating semantics stay explicit, comparable, and truthful across frontend and backend surfaces.

## Scope Boundaries

In scope:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

Out of scope:

- basic candidate-table rendering
- artifact-library replay flows
- options-related validation

## Current User-Facing Behavior

The validation workspace exposes a large robustness stack: walk-forward, finalist re-opt walk-forward, perturbation, trade bootstrap, CSCV/PBO, SPA, cross-ticker reruns, and probability-style hard gates such as PSR, DSR, CSCV thresholds, and max PBO. The builder and service already warn that some of these methods are omitted or reduced under certain routes, especially during cross reruns or some composite-family validations.

The backend then folds these methods into candidate verdicts and run-level recommendations. That means the user-facing panel is already acting as a control surface for a complex method stack, not just a thin trigger UI.

## Current Code Surface

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`

## Current Verification Surface

- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`
- `crates/app_core/src/validation_pipeline.rs` tests
- strategy-specific validation tests in `double_ma_validate.rs` and `composite_validate.rs`

## Strengths Already Present

- the method stack is already broader and more explicit than a simple holdout rerun
- builder previews already surface several omitted-module and route-restriction warnings
- backend verdict logic already consumes CSCV, SPA, and track-record-style evidence explicitly
- deterministic tests already exist for some robustness primitives such as SPA determinism and minimum track-record behavior
- the validation panel already groups many of these controls into one analyst workflow

## Weaknesses / Failure Modes

- the method stack is large enough that users can confuse “enabled in UI” with “fully applied on every route”
- some method interactions are subtle:
  - cross reruns omit modules
  - GPU routes may support subsets differently
  - approval logic distinguishes warnings from failures
- current summaries are spread across preview text, warnings, shortlist meta, and recommendation text
- method defaults can silently reshape the product if they drift

## Regressions To Avoid

- letting unsupported methods appear fully active on routes that omit or degrade them
- changing default robustness posture without updating preview text and tests
- weakening CSCV, SPA, or track-record logic while keeping old recommendation language
- hiding omitted-module consequences during cross or composite special-case routes
- making frontend previews less specific than backend behavior

## External Research Summary

- [TimeSeriesSplit — scikit-learn](https://scikit-learn.org/1.0/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because chronological validation discipline is the baseline reference for split-based robustness evaluation.
- [The Probability of Backtest Overfitting — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253) is relevant because CSCV/PBO is a core part of the repo’s robustness-method framing.
- [A Test for Superior Predictive Ability — SSRN](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=264569) is relevant because Hansen SPA is explicitly represented in the repo’s validation pipeline.
- [The Sharp Razor / Minimum Track Record Length](https://www.researchgate.net/publication/256035273_The_Sharp_Razor_Deflating_the_Sharpe_Ratio_by_Asking_for_a_Minimum_Track_Record_Length) is relevant because track-record sufficiency already affects the final recommendation path.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the method stack’s visible contract is currently exercised through frontend builder/service tests as well as backend tests.

## Proposed Improvement Approach

1. Document the robustness stack as a layered method taxonomy instead of one undifferentiated control mass.
2. Distinguish:
   - primary split methods
   - finalist-focused methods
   - bootstrap/significance methods
   - hard-gate thresholds
   - cross-dataset extensions
3. Make route- and mode-specific omissions explicit in the contract.
4. Keep frontend previews, backend verdict logic, and report language aligned to the same method vocabulary.
5. Expand tests around route-specific method omission and recommendation parity.

## Checklist

- [ ] Document the current robustness method taxonomy and default posture.
- [ ] Document which methods are always available versus route-conditional.
- [ ] Document omitted/degraded method behavior for cross reruns and constrained composite routes.
- [ ] Document how hard gates differ from warning-only methods in the final recommendation.
- [ ] Add or tighten tests for route-specific omitted-method summaries.
- [ ] Add or tighten tests for method-default behavior and recommendation parity.
- [ ] Add or tighten tests for CSCV, SPA, and track-record interactions at the run-verdict layer.

## Verification Plan

- run `npm --prefix web test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- run targeted `validation_pipeline.rs` tests
- run representative `double_ma_validate.rs` and `composite_validate.rs` tests
- manually verify preview summaries for one standard run, one cross run, and one constrained composite route

## Rollout / Fallback / Compatibility Notes

- preserve the current method vocabulary unless a coordinated rename/update is made everywhere
- prefer additive clarification over silently changing the enabled-by-default posture
- if a route begins supporting more methods, update preview warnings and tests in the same change
- keep omitted-method messaging explicit even when the route still returns a successful validation result

## Related Docs

- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/26_cross_ticker_finalist_logic_2026-04-09.md`
