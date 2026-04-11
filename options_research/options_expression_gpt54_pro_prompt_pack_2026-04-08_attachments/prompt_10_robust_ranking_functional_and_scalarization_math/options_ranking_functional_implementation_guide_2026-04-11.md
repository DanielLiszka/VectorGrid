# Options Ranking Functional - Detailed Implementation Guide

Date: 2026-04-11
Status: validated implementation guide created from the valid portions of Prompt 10 (`options_ranking_functional_direction_FINAL.md`)

## Purpose

This document converts the valid implementation residue of:

- `options_ranking_functional_direction_FINAL.md`

into one narrow, implementation-ready plan for the current repo state.

The key point is:

- Prompt 10 does justify a real ranking-math change
- Prompt 10 does justify a narrow revision of `robust_score(...)`
- Prompt 10 does not justify widening workflow stages, GPU output state, artifact schemas, or frontend payloads
- Prompt 10 does not justify pulling sealed metrics into broad ranking
- Prompt 10 does not justify immediately promoting Prompt 9's new mean/dispersion metrics into ranking

The valid adopted direction is:

- keep the current scalar ranking family
- keep the current lane weights
- keep the current fallback ladder
- keep sealed metrics out of broad pre-`top_k` ranking
- revise `robust_score(...)` so each resolved lane contributes its own coverage-adjusted expectancy term

This guide exists to make that change explicit, additive, testable, and easy to verify without reopening:

- Prompt 9 metric expansion scope
- Prompt 11 validation-math scope
- sealed-stage ordering policy
- GPU aggregate-state scope

## Source Validation Filter

The folder-local markdown set is:

- `options_ranking_functional_direction_FINAL.md`
- `options_ranking_functional_implementation_guide_2026-04-11.md`

For implementation authority, this guide treats:

- `options_ranking_functional_direction_FINAL.md` as the source to be validated and filtered
- this guide as the implementation tracker derived from only the valid portions of that source

The response document is not adopted wholesale. It must be filtered against the current repo state.

### Valid from the Prompt 10 response

- revise `robust_score(...)` rather than replacing the ranking system
- use lane-local coverage adjustment:
  - `lane_utility(m) = clamp(1 - m.invalid_rate, 0, 1) * m.expectancy_per_premium`
- keep the current lane weights:
  - search `0.45`
  - holdout `0.25`
  - stress `0.15`
  - harsher stress `0.10`
  - extreme stress `0.05`
- keep the current fallback ladder:
  - `S = search`
  - `H = holdout.unwrap_or(S)`
  - `C = stress.unwrap_or(S)`
  - `U = harsher_stress.unwrap_or(C)`
  - `X = extreme_stress.unwrap_or(U)`
- apply the same scalar family to:
  - broad pre-`top_k` ranking
  - exactified reranking on retained tuples
- keep sealed-finalist ordering separate
- keep validation-verdict logic separate
- avoid schema widening, GPU state widening, and extra stages

### Valid but only as cautionary rollout context

- saved artifacts produced before the change will carry old `robust_score` semantics
- newly produced artifacts will carry new semantics
- that old/new drift is real even if no schema shape changes

### Stale or partially invalid in the Prompt 10 response

The Prompt 10 response contains state-availability assumptions that are no longer current after Prompt 9 implementation.

Specifically, it is now stale to say that the repo does not yet have:

- `mean_return_on_premium`
- `return_volatility_on_premium`

Those now exist in live code and are computed on the CPU and aggregate/GPU paths.

That does **not** mean they belong in this implementation wave.

The correct conclusion is:

- the repo now has more state than the Prompt 10 response assumed
- this implementation guide still excludes those Prompt 9 metrics from ranking adoption
- any use of those fields in ranking should be treated as a separate follow-up, not bundled into the minimal Prompt 10 revision

## Additive Posture And Explicit Non-Goals

### Additive posture

This implementation is additive in one narrow sense:

- it improves the semantics of `robust_score(...)`
- it does not remove any current lane
- it does not remove any current ordering stage
- it does not remove any current CPU fallback behavior
- it does not remove any current artifact/report surface

### Explicit non-goals

This guide does **not** authorize:

- changing `OptionsExpressionMetrics`
- changing `OptionsGpuTupleAggregate`
- changing `TupleAggregateMetrics`
- changing `metrics_from_aggregate(...)`
- changing `compute_metrics(...)`
- changing `validation_verdict` semantics
- changing `inferred_validation_verdict()`
- changing `is_stress_fragile()`
- changing sealed-finalist ordering rules
- changing tie-break policy in `sort_ranked_results(...)`
- changing `best_robust_score` payload shape
- changing frontend tables or panel displays
- using `mean_return_on_premium` or `return_volatility_on_premium` inside ranking
- adding shrinkage, lower-confidence-bound, or dispersion-penalized ranking in this wave

## Final Adopted Functional

### Current shipped functional

Today the repo effectively computes:

```text
coverage = clamp(1 - search.invalid_rate, 0, 1)

robust_score =
    coverage * (
        0.45 * search.expectancy_per_premium
      + 0.25 * holdout_or_search.expectancy_per_premium
      + 0.15 * stress_or_search.expectancy_per_premium
      + 0.10 * harsher_or_stress.expectancy_per_premium
      + 0.05 * extreme_or_harsher.expectancy_per_premium
    )
```

The asymmetry is:

- expectancies are lane-specific
- coverage is search-only

### Adopted replacement

Implement:

```text
lane_utility(m) = clamp(1 - m.invalid_rate, 0, 1) * m.expectancy_per_premium

S = search
H = holdout.unwrap_or(S)
C = stress.unwrap_or(S)
U = harsher_stress.unwrap_or(C)
X = extreme_stress.unwrap_or(U)

robust_score_v2 =
    0.45 * lane_utility(S)
  + 0.25 * lane_utility(H)
  + 0.15 * lane_utility(C)
  + 0.10 * lane_utility(U)
  + 0.05 * lane_utility(X)
```

### Required invariants

The implementation must preserve all of the following:

- weights remain exactly `0.45 / 0.25 / 0.15 / 0.10 / 0.05`
- fallback ladder remains exactly `S / H / C / U / X`
- `_sealed` remains ignored inside `robust_score(...)`
- broad ranking remains scalar
- exactified reranking remains scalar
- sealed-finalist ordering remains sealed-metrics-first and separate

## Current Code Touch Points

Start in these locations before broader repo search:

- `crates/app_core/src/options_expression.rs`
  - `robust_score(...)`
  - `exactify_ranked_results(...)`
  - `gpu_scoring::rank_compiled_tuples_gpu(...)`
  - `rank_compiled_tuples_cpu(...)`
  - `assign_sealed_final_ranks(...)`
  - `apply_sealed_finalist_stage(...)`
  - tests:
    - `robust_score_excludes_sealed_metrics_from_tuple_ranking()`
    - `sealed_final_rank_uses_sealed_metrics_ordering()`
- `src-tauri/src/commands_options_expression.rs`
  - no core implementation change expected
  - artifact-summary smoke only
- `web/src/components/OptionsExpressionPanel.vue`
  - no required code change expected

## Route-Specific Ranking Surface

The guide must remain honest about which lanes are present at which ranking point.

### Broad CPU ranking

Current broad CPU ranking resolves:

- search
- holdout when present
- conservative stress when present

It currently passes `None` for:

- harsher stress
- extreme stress

That should remain unchanged in this wave.

### Broad GPU ranking

Current broad GPU ranking resolves:

- search
- holdout when present
- conservative stress when present
- harsher stress when present

It currently passes `None` for:

- extreme stress

That should remain unchanged in this wave.

### Exactified reranking on retained tuples

Current exactified reranking resolves:

- search
- holdout
- conservative stress
- harsher stress
- extreme stress

That should remain unchanged in this wave.

### Sealed-finalist ordering

Current sealed-finalist ordering already uses sealed-validation surfaces explicitly.

That ordering should remain unchanged in this wave.

## Detailed Implementation Plan

## 1. Core Scalar Change

### 1.1 `robust_score(...)`

Primary implementation file:

- `crates/app_core/src/options_expression.rs`

Required work:

- add a small local helper, for example:
  - `fn lane_utility(metrics: &OptionsExpressionMetrics) -> f64`
- compute lane utility from:
  - `invalid_rate`
  - `expectancy_per_premium`
- resolve lanes using the current fallback ladder
- replace the current search-coverage-times-global-sum formula with the lane-local version
- keep the existing function signature unchanged

Important:

- do not rename `robust_score(...)` in this wave
- do not change the `_sealed` parameter contract in this wave
- do not widen the function into a more generic ranking-policy framework

### 1.2 Call-site behavior

No structural call-site rewrite is required.

The new semantics should flow automatically through existing calls in:

- broad GPU ranking
- broad CPU ranking
- exactified reranking

That means the implementation should prefer:

- a narrow helper change in `robust_score(...)`
- no changes to tuple result shapes
- no changes to ranking data plumbing

## 2. Required Tests

All new tests should live near the ranking code in:

- `crates/app_core/src/options_expression.rs`

### 2.1 Preserve existing invariants

Keep and re-run:

- `robust_score_excludes_sealed_metrics_from_tuple_ranking()`
- `sealed_final_rank_uses_sealed_metrics_ordering()`

Those are invariant tests, not optional nice-to-haves.

### 2.2 Add new scalar-behavior tests

Add explicit targeted tests for:

1. identical lane expectancies, worse holdout `invalid_rate` lowers score
2. identical lane expectancies, worse stress `invalid_rate` lowers score
3. identical lane expectancies, worse harsher-stress `invalid_rate` lowers score when that lane is present
4. fallback equality:
   - missing holdout should match explicit holdout identical to search
5. fallback equality:
   - missing harsher stress should match explicit harsher stress identical to conservative stress
6. all lane `invalid_rate` equal:
   - new scalar collapses to the same ordering as the old weighted expectancy form up to the common coverage factor

### 2.3 Recommended narrow regression tests

If the current test harness makes it cheap, also add:

1. broad ranking smoke where two tuples differ only in holdout invalid rate
2. exactified reranking smoke where the extreme-stress lane changes the revised scalar as expected

These are useful but secondary to the core unit-math tests above.

## 3. Compatibility And Freeze Surface

### 3.1 Must stay frozen

These files or symbols should remain code-frozen for this wave unless verification proves a direct need:

- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`

### 3.2 Why they stay frozen

This change only needs:

- `invalid_rate`
- `expectancy_per_premium`

Those already exist at every current ranking point.

Therefore:

- no GPU aggregate widening is justified
- no artifact payload widening is justified
- no frontend contract change is justified

## 4. Rollout And Artifact-Semantics Risk

The ranking change is semantic, not structural.

That means:

- old artifacts keep old `robust_score`
- new artifacts get new `robust_score`
- artifact summary shape does not change

This guide does **not** require immediate schema versioning.

But the implementation should explicitly acknowledge:

- compare surfaces spanning old and new artifacts may show score drift that is semantic, not a bug

That is acceptable in this wave unless the team decides to introduce an explicit score-semantics label later.

## 5. Verification Plan

Code-level verification is required before any benchmark or canonical-slice follow-up.

### 5.1 Local unit and integration verification

Run:

- `cargo test -q -p ta_app_core robust_score --lib`
- `cargo test -q -p ta_app_core sealed_final_rank_uses_sealed_metrics_ordering --lib -- --exact`
- `cargo test -q -p ta_app_core options_ --lib`
- `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`

Expected outcome:

- all ranking tests pass
- no sealed-ordering invariant regresses
- no artifact-summary smoke regresses

### 5.2 CPU/GPU parity sanity

Because the scalar is shared across CPU and GPU ranking paths, confirm:

- no CPU/GPU parity regression emerges beyond expected float noise
- retained tuple ordering changes only because of the intended semantic revision, not because of path divergence

Use existing broad ranking coverage where possible. No new GPU kernel tests should be required.

### 5.3 Canonical-slice semantic validation

Before calling the change signed off, re-run the current canonical provider-backed proof slices used for options-expression validation/ranking sanity.

Minimum expectation:

- the revised scalar should improve or at least not clearly worsen retained/finalist quality on the current SPY and QQQ bounded DTE7 slices
- any ranking churn should be explainable in terms of lane-local invalid-rate penalties

This is not a throughput benchmark question first. It is a semantic-quality check first.

### 5.4 Performance sanity

This change should be effectively free.

Reject any implementation that:

- widens runtime state
- adds replay work
- adds new kernels
- adds noticeable runtime overhead in ranking

## 6. Stop Conditions

Stop implementation or stop sign-off if any of the following happen.

### 6.1 No decision-quality win

If canonical slice reruns show:

- no meaningful improvement in retained/finalist quality
- or a clear worsening of finalist quality

then stop and keep the current `robust_score`.

### 6.2 Coverage gaming without better ranking

If the revised scalar merely demotes tuples with higher invalid rates without improving:

- holdout quality
- stress resilience
- or sealed-stage quality

then stop.

### 6.3 Invariant pressure

If the change pressures any of the following, stop and defer:

- sealed metrics entering broad ranking
- sealed-finalist ordering redesign
- GPU aggregate widening
- Prompt 9 metric promotion into ranking
- tie-break redesign as part of the same patch

### 6.4 Artifact rollout friction is unacceptable

If the team decides that old/new `robust_score` semantic drift across saved artifacts is unacceptable without explicit labeling or versioning, stop and make that a separate rollout decision first.

## 7. Implementation Checklist

## 7.1 Baseline invariants

- [x] Keep broad ranking scalar rather than moving to Pareto or dominance ranking.
- [x] Keep sealed metrics out of broad pre-`top_k` ranking.
- [x] Keep sealed-finalist ordering separate.
- [x] Keep validation-verdict logic separate from ranking scalar changes.
- [x] Keep CPU fallback intact.
- [x] Keep artifact and frontend payload shapes unchanged in this wave.

## 7.2 Core backend implementation

- [ ] Add a small `lane_utility(...)` helper near `robust_score(...)` in `crates/app_core/src/options_expression.rs`.
- [ ] Update `robust_score(...)` to apply lane-local `invalid_rate` to each resolved lane utility instead of using only search-lane coverage.
- [ ] Preserve the current weights `0.45 / 0.25 / 0.15 / 0.10 / 0.05`.
- [ ] Preserve the current fallback ladder `S / H / C / U / X`.
- [ ] Keep the `robust_score(...)` function signature unchanged.
- [ ] Keep `_sealed` ignored in `robust_score(...)`.
- [ ] Leave the broad CPU ranking call structure unchanged.
- [ ] Leave the broad GPU ranking call structure unchanged.
- [ ] Leave the exactified reranking call structure unchanged.

## 7.3 Freeze-surface enforcement

- [ ] Do not modify `OptionsExpressionMetrics` for this prompt.
- [ ] Do not modify `OptionsGpuTupleAggregate` for this prompt.
- [ ] Do not modify `TupleAggregateMetrics` for this prompt.
- [ ] Do not modify `metrics_from_aggregate(...)` for this prompt.
- [ ] Do not modify `compute_metrics(...)` for this prompt.
- [ ] Do not modify `assign_sealed_final_ranks(...)`.
- [ ] Do not modify `apply_sealed_finalist_stage(...)`.
- [ ] Do not modify `validation_verdict` or `inferred_validation_verdict()` semantics.
- [ ] Do not pull Prompt 9's `mean_return_on_premium` or `return_volatility_on_premium` into ranking in this wave.
- [ ] Do not redesign tie-break behavior in `sort_ranked_results(...)` in this wave.

## 7.4 Test implementation

- [ ] Keep `robust_score_excludes_sealed_metrics_from_tuple_ranking()` passing.
- [ ] Keep `sealed_final_rank_uses_sealed_metrics_ordering()` passing.
- [ ] Add a test proving worse holdout `invalid_rate` lowers score when expectancies are otherwise equal.
- [ ] Add a test proving worse stress `invalid_rate` lowers score when expectancies are otherwise equal.
- [ ] Add a test proving worse harsher-stress `invalid_rate` lowers score when that lane is present.
- [ ] Add a fallback-equivalence test for missing holdout vs explicit holdout equal to search.
- [ ] Add a fallback-equivalence test for missing harsher stress vs explicit harsher stress equal to conservative stress.
- [ ] Add a test proving equal lane invalid rates collapse to the expected weighted-expectancy ordering.
- [ ] Add a narrow reranking smoke test only if needed to clarify intended exactified behavior.

## 7.5 Verification

- [ ] Run `cargo test -q -p ta_app_core robust_score --lib`.
- [ ] Run `cargo test -q -p ta_app_core sealed_final_rank_uses_sealed_metrics_ordering --lib -- --exact`.
- [ ] Run `cargo test -q -p ta_app_core options_ --lib`.
- [ ] Run `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`.
- [ ] Confirm no GPU aggregate or schema changes were introduced accidentally.
- [ ] Confirm broad CPU ranking and broad GPU ranking remain semantically aligned on a representative prepared workload.
- [ ] Re-run the canonical SPY bounded DTE7 ranking proof slice.
- [ ] Re-run the canonical QQQ bounded DTE7 ranking proof slice.
- [ ] Compare retained tuple churn and finalist quality before vs after the ranking revision.
- [ ] Smoke reopen at least one newly produced artifact and confirm summary/compare surfaces remain interpretable after the score semantic change.
- [ ] Reject the change if the churn is not clearly defensible by the lane-local coverage correction.

## 7.6 Rollout awareness

- [ ] Document internally that old and new artifacts will carry different `robust_score` semantics.
- [ ] Decide whether score-semantics labeling is deferred or needed immediately.

## Final Implementation Rule

If the patch starts pulling in:

- Prompt 9 metric usage
- Prompt 11 confidence math
- sealed-stage redesign
- GPU aggregate widening
- or frontend/UI expansion

then the patch is no longer Prompt 10 implementation and should stop.
