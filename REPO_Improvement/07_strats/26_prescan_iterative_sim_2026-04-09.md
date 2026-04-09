## Purpose

Define the improvement plan for the two accuracy-sensitive adjuncts around non-options search and validation: predictive prescan narrowing and the optional iterative simulation engine used to compare richer execution assumptions against the vectorized baseline.

## Scope Boundaries

In scope:

- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- fidelity-audit and optimizer-eval bins that measure decision preservation

Out of scope:

- replacing the main vectorized backtest path
- adding new stop/take/slippage models
- frontend redesign of validation controls
- options-related validation/search behavior

## Current User-Facing Behavior

These surfaces affect visible product behavior in two different ways:

- predictive prescan narrows or seeds candidate exploration in composite optimization paths
- iterative simulation provides an alternate validation engine with stop-loss, take-profit, intrabar, slippage, cooldown, and persistence effects

Users experience this through:

- which candidates survive prescan narrowing
- whether “rescue” pairs appear in composite search
- how iterative metrics differ from vectorized metrics in validation outputs
- whether richer execution assumptions change shortlist ordering or approval decisions

## Current Code Surface

- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- related evaluators:
  - `crates/app_core/src/bin/real_data_optimizer_eval.rs`
  - `crates/app_core/src/bin/real_data_fidelity_audit.rs`

## Current Verification Surface

- direct tests in `crates/app_core/src/iterative_sim.rs`
- validation integration tests in `double_ma_validate.rs`
- validation integration tests in `composite_validate.rs`
- prescan behavior exercised indirectly in `composite_cpu.rs` and `composite_mo.rs`
- decision-preservation tooling in `real_data_optimizer_eval.rs` and `real_data_fidelity_audit.rs`

## Strengths Already Present

- predictive prescan is isolated in its own file rather than buried inside composite search loops
- iterative simulation has a concrete config model and delta reporting instead of a hidden code path
- validation pipeline ownership is explicit about when iterative mode runs
- validation outputs already include iterative-versus-vectorized metric deltas instead of forcing manual comparison
- the repo now has a fidelity-audit harness that can evaluate decision preservation instead of relying on intuition

## Weaknesses / Failure Modes

- predictive prescan has little direct verification compared with its downstream influence on search-space narrowing
- prescan heuristics are currently hard-coded, which makes scope creep or silent behavior drift likely
- iterative simulation semantics are spread across the engine, pipeline, and validation adapters
- it is easy for vectorized and iterative paths to diverge in ways that change decisions without a clear audit trail
- richer simulation can look “more realistic” and still be wrong, under-tested, or inconsistently applied

## Regressions To Avoid

- changing prescan narrowing semantics without evidence that candidate quality or rescue behavior is preserved
- letting iterative metrics silently alter candidate ranking, gate decisions, or report wording without explicit policy
- treating iterative mode as authoritative when the repo still treats vectorized mode as the baseline path
- adding new simulation complexity without corresponding end-to-end fidelity checks

## External Research Summary

- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because both prescan and iterative validation live in time-ordered workflows where information leakage and ordering mistakes are costly.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because validation outputs and iterative deltas are structured contracts consumed by reports and artifacts.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because prescan narrowing and iterative simulation can affect both fidelity and runtime cost.
- [rayon iterator docs](https://docs.rs/rayon/latest/rayon/iter/index.html) are relevant because composite search and validation often combine heuristics with parallel evaluation, making deterministic aggregation important.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because iterative deltas and prescan outcomes ultimately flow into JSON-shaped artifacts and summary payloads.

## Proposed Improvement Approach

1. Freeze the conceptual roles of the two surfaces:
   - predictive prescan is a heuristic narrowing aid
   - iterative simulation is a richer validation comparison engine
2. Make those roles explicit in tests and reporting so neither surface quietly becomes “the truth” by accident.
3. Add direct prescan tests for:
   - empty/small period sets
   - stride/cap behavior
   - deterministic seed-pair ordering
   - top-k truncation
4. Add policy tests for iterative mode:
   - when deltas are attached
   - when deltas affect ranking or gating
   - how vectorized and iterative metrics are displayed side by side
5. Use the fidelity-audit harness as the final arbiter before any tightening or expansion of these paths.

## Checklist

- [ ] Document predictive prescan as a heuristic narrowing contract, not as an exact optimizer.
- [ ] Add or tighten direct unit tests in `prescan_predictive.rs` for stride, cap, ordering, and empty-edge behavior.
- [ ] Add or tighten composite integration tests that verify rescue-pair behavior stays bounded and explainable.
- [ ] Document when iterative mode is informational versus decision-affecting in validation outputs.
- [ ] Add or tighten validation tests for iterative delta attachment and verdict stability.
- [ ] Run the fidelity-audit harness or equivalent bounded checks before accepting behavioral changes to either path.
- [ ] Decide whether iterative-vs-vectorized ranking differences require stronger UI/report wording in follow-on docs.

## Verification Plan

- run direct tests in `iterative_sim.rs`
- add/run direct tests in `prescan_predictive.rs`
- run validation integration tests in `double_ma_validate.rs`
- run validation integration tests in `composite_validate.rs`
- run composite search tests in `composite_cpu.rs` and `composite_mo.rs`
- run `real_data_optimizer_eval.rs` or `real_data_fidelity_audit.rs` for decision-preservation checks when behavior changes

## Rollout / Fallback / Compatibility Notes

- keep vectorized validation as the baseline unless a separate policy doc changes that stance
- keep prescan heuristic behavior bounded and explainable
- prefer stronger visibility and tests over increasing heuristic aggressiveness
- if fidelity checks weaken after a change, revert to the simpler prior behavior rather than defending the heuristic

## Related Docs

- `REPO_Improvement/07_strats/07_strat_composite_mo_2026-04-09.md`
- `REPO_Improvement/07_strats/13_eng_composite_mo_prescan_search_2026-04-09.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
