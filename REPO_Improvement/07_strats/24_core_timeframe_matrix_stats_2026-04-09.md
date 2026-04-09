## Purpose

Define the improvement plan for the small but correctness-critical core utilities that normalize timeframe handling, matrix indexing, and statistical primitives used across non-options optimization and validation paths.

## Scope Boundaries

In scope:

- `crates/app_core/src/timeframe.rs`
- `crates/app_core/src/matrix_index.rs`
- `crates/app_core/src/stats_common.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_stats.rs`

Out of scope:

- frontend dataset pickers or timeframe selectors
- changing the product’s supported timeframe set
- rewriting downstream optimization/validation workflows
- options-related timeframe or stats logic

## Current User-Facing Behavior

These utilities are not primary UI features, but they directly influence user-visible outputs:

- timeframe parsing and resampling from CSV or downloaded candles
- deterministic matrix indexing for fast/slow parameter tables
- PSR/DSR/probability-gate calculations in validation outputs
- quantile, median, and worst-case summaries used in validation and report surfaces

If these utilities drift, the visible symptoms are often misleading:

- resampled datasets that do not match user expectations
- heatmaps or matrices with wrong row/column mapping
- validation gates that flip verdicts unexpectedly
- statistical summaries that appear plausible but are numerically inconsistent

## Current Code Surface

- `crates/app_core/src/timeframe.rs`
- `crates/app_core/src/matrix_index.rs`
- `crates/app_core/src/stats_common.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_stats.rs`
- major consumers in:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/bin/real_data_fidelity_audit.rs`
  - `src-tauri/src/bin/ta_profile_route_matrix.rs`

## Current Verification Surface

- unit tests embedded in `timeframe.rs`
- unit tests embedded in `matrix_index.rs`
- unit tests embedded in `stats_common.rs`
- unit tests embedded in `sharpe_stats.rs`
- unit tests embedded in `validation_stats.rs`
- consumer coverage through `double_ma.rs`, `validation_pipeline.rs`, and headless bins

## Strengths Already Present

- each concern has a fairly clear home instead of being duplicated everywhere
- timeframe parsing already enforces non-empty labels, positive intervals, and sorted timestamps
- resampling already anchors buckets to the first bar of each trading session
- matrix indexing is explicitly centralized instead of rederived in every consumer
- PSR/DSR and gate sanitization are already separated from UI code
- these files are small enough to harden thoroughly without a major rewrite

## Weaknesses / Failure Modes

- the utilities are foundational, so small mistakes have wide downstream blast radius
- consumers may partially reimplement assumptions instead of treating these modules as single sources of truth
- resampling correctness is easy to regress with edge cases such as missing fields, non-finite data, or session transitions
- statistical helpers can become inconsistent if downstream code interprets them differently than their authors intended
- matrix-index ordering bugs can quietly scramble result surfaces while leaving totals or counts looking valid

## Regressions To Avoid

- changing resample bucket boundaries or session anchoring without deliberate compatibility review
- changing matrix ordering or fast<slow indexing semantics
- changing probability-gate sanitization or PSR/DSR behavior without corresponding report/test updates
- duplicating quantile or worst-case logic outside the shared core

## External Research Summary

- [Polygon aggregates/custom bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) are relevant because upstream bar semantics and interval expectations influence how local resampling should be validated and explained.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because downstream validation and holdout workflows depend on preserving time order; timeframe and stats helpers should support that temporal discipline rather than eroding it.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because these small utilities influence structured output fields and should keep those contracts stable.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because foundational utility changes can cause broad performance regressions even when they are behavior-preserving.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because these core numeric/timeframe helpers often feed artifact/report JSON and should remain predictable for serialization consumers.

## Proposed Improvement Approach

1. Treat these five modules as shared correctness primitives with explicit downstream contracts.
2. Make each module the documented single source of truth for its concern:
   - timeframe parsing/resampling
   - matrix indexing/order
   - simple quantile/worst-case helpers
   - Sharpe-derived probability metrics
   - validation probability gates
3. Add a consumer-focused audit that identifies any duplicated logic elsewhere in the repo.
4. Expand edge-case tests for:
   - session-boundary resampling
   - invalid timeframe labels
   - non-finite data handling
   - matrix-order/index mapping
   - PSR/DSR gate thresholds
5. If cleanup is needed, prefer small helper extraction or clearer naming over semantic redesign.

## Checklist

- [ ] Document the canonical supported timeframe-label rules and back-compat cases.
- [ ] Add or tighten resample tests for session transitions, missing optional fields, and non-finite close rejection.
- [ ] Add or tighten matrix-index tests for ordering, bounds, and fast<slow assumptions.
- [ ] Add or tighten tests for quantile/median helpers on empty, singleton, even, and non-finite edge cases.
- [ ] Add or tighten PSR/DSR tests around threshold and probability-gate boundaries.
- [ ] Audit consumers for duplicated timeframe, matrix-index, or summary-stat logic.
- [ ] Decide whether a lightweight benchmark is warranted for hot utility paths used in repeated scans.

## Verification Plan

- run targeted tests in `timeframe.rs`
- run targeted tests in `matrix_index.rs`
- run targeted tests in `stats_common.rs`
- run targeted tests in `sharpe_stats.rs`
- run targeted tests in `validation_stats.rs`
- run representative consumer tests in `double_ma.rs` and `validation_pipeline.rs`
- run the fidelity-audit or route-matrix bins if changes affect time bucket or stat semantics

## Rollout / Fallback / Compatibility Notes

- preserve current semantics first; improve documentation and edge-case coverage before attempting cleanup
- if any consumer relies on behavior that seems odd, capture it in tests before deciding whether it should change
- keep these modules small and authoritative; do not let downstream convenience code become the real contract
- treat any resample or stats change as cross-cutting and worthy of artifact/report rechecks

## Related Docs

- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/05_validation/10_eng_validation_stats_core_2026-04-08.md`
- `REPO_Improvement/07_strats/16_fidelity_audit_harness_2026-04-08.md`
- `REPO_Improvement/04_results/20_matrix_table_state_2026-04-08.md`
