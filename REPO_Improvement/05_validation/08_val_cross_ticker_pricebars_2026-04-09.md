# Validation Cross-Ticker Price Bars Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation-related price-bar and cross-ticker alignment utilities so chart/downsample output, base-horizon overlap behavior, and cross-review supporting data remain explicit and trustworthy.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_validation.rs`
- `get_price_bars` and `build_price_bars`
- cross-ticker candle alignment to the base horizon
- supporting data behavior used for review/inspection around validation runs

Out of scope:

- drilldown trade/equity rendering
- validation candidate-table semantics
- options-related chart or validation flows

## Current User-Facing Behavior

The validation command surface currently includes a `get_price_bars` helper that converts loaded candle data into a bounded downsampled `PriceBar` series. The helper preserves open/high/low/close/volume semantics over each aggregated segment, retains source index ranges, and clamps the requested point cap into a practical range.

The same command module also owns cross-ticker alignment logic that trims or borrows cross datasets to match the base horizon before reruns. Together, these utilities shape the supporting time-series evidence a user can trust when inspecting validation or cross-ticker behavior.

## Current Code Surface

- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationService.ts`
- `web/src/services/analysisService.ts`
- any consumer of `get_price_bars` and validation-adjacent chart inspection flows

## Current Verification Surface

- `src-tauri/src/commands_validation.rs` tests
- validation/analysis service tests where price-bar-related flows are exercised indirectly

## Strengths Already Present

- price-bar downsampling already preserves OHLCV semantics instead of using a naive close-only decimation
- returned bars carry source index ranges, which supports traceability
- point caps are already bounded
- cross-ticker datasets are already aligned to the base horizon before shared reruns
- overlap-alignment tests already exist in the validation command module

## Weaknesses / Failure Modes

- price-bar output is easy to treat as a generic helper even though it carries evidence implications
- supporting chart data and cross-horizon alignment live in the same large command file, which obscures ownership
- the relationship between aligned cross reruns and any charted/downsampled review data is not written down explicitly
- downsample semantics could drift from user expectations if cap and aggregation rules change silently

## Regressions To Avoid

- changing OHLC aggregation into a less faithful downsample
- losing `from_index` / `to_index` traceability
- breaking overlap alignment for cross-ticker datasets
- allowing non-monotonic timestamps in emitted price bars
- silently changing cap behavior or bar-construction rules without updating tests and docs

## External Research Summary

- [TimeSeriesSplit — scikit-learn](https://scikit-learn.org/1.0/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because cross-ticker evidence should remain anchored to comparable chronological windows.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because `get_price_bars` is a typed command boundary for frontend review surfaces.
- [canvas element accessibility | MDN](https://developer.mozilla.org/es/docs/Web/HTML/Reference/Elements/canvas) is relevant because any chart surface built from price bars needs explicit accessibility and supporting text rather than assuming the bitmap alone is self-explanatory.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because supporting chart evidence should stay observable and testable from the user-facing surface.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because chart inputs should stay derived from shared command/service state rather than panel-local ad hoc logic.

## Proposed Improvement Approach

1. Document price-bar output as an evidence-supporting contract, not a generic helper.
2. Freeze downsample semantics around bounded OHLCV aggregation plus source-index traceability.
3. Document the relationship between cross-ticker base-horizon alignment and any price-bar review that accompanies validation.
4. Keep command ownership explicit so later chart changes do not silently alter validation-supporting evidence.
5. Expand tests around cap behavior, overlap alignment, and monotonic output.

## Checklist

- [ ] Document the `PriceBar` contract and emitted fields.
- [ ] Document cap clamping and aggregation rules for downsampled bars.
- [ ] Document base-horizon alignment behavior for cross-ticker datasets.
- [ ] Document which review surfaces rely on these helpers.
- [ ] Add or tighten tests for monotonic timestamps and index-range traceability.
- [ ] Add or tighten tests for overlap alignment and cap behavior.
- [ ] Keep chart-supporting evidence distinct from run-verdict semantics.

## Verification Plan

- run targeted `commands_validation.rs` tests
- run any service tests that exercise price-bar fetch or review flows
- manually inspect one loaded dataset with `get_price_bars` and one cross-ticker-aligned validation path

## Rollout / Fallback / Compatibility Notes

- preserve current OHLCV aggregation semantics unless a deliberate visualization contract change is made
- keep index-range fields stable for traceability
- if cross alignment changes, update both validation docs and tests at the same time
- prefer additive explanation over changing supporting-data behavior casually

## Related Docs

- `REPO_Improvement/05_validation/26_cross_ticker_finalist_logic_2026-04-09.md`
- `REPO_Improvement/05_validation/27_validation_robustness_methods_2026-04-09.md`
- `REPO_Improvement/04_results/05_results_drilldown_2026-04-09.md`
- `REPO_Improvement/04_results/19_chart_rendering_composables_2026-04-09.md`
