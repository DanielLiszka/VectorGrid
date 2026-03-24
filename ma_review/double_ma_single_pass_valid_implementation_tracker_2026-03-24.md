# Double MA Single-Pass Valid Implementation Tracker

Date: 2026-03-24

## Purpose

This document consolidates the valid, additive implementation work from the following review documents:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `ma_review/double_ma_single_pass_validation_pullforward_2026-03-24.md`

This tracker is grounded against the current repo state, not the older replay-backed product path. The current Double MA GPU product route is the 10-row single-pass path with exact `omega`, and the review items below are filtered against that reality.

## Current Repo State

- Default Sharpe/Volatility product route: 10-row single-pass GPU path.
- Current benchmark surface remains the contract-split `double_ma_opt` benchmark.
- The current optimization path already exposes the main product metrics from the 10-row contract and should remain the default path.
- Existing validation modules remain valid and in scope:
  - walk-forward
  - reopt walk-forward
  - perturb
  - trade bootstrap
  - SPA
  - CSCV / PBO
  - cross-ticker
- Optimization-stage additions must supplement those downstream validation modules, not replace them.

## Validity Filter

Only include items that satisfy all of the following:

1. Additive-only. No removal, narrowing, or weakening of the current 10-row product path, existing metrics, current benchmark variants, or CPU fallback behavior.
2. Compatible with the current shared GPU backend:
   - `crates/app_core/src/gpu_backtest_kernel.rs`
   - `crates/app_core/src/kernels/double_crossover.cu`
   - `crates/app_core/src/double_ma.rs`
3. Prefer exact row-derived or low-state additions before wider-state or approximate additions.
4. No replay / second-pass reintroduction for this workstream.
5. New optimization-stage robustness summaries must not be framed as replacements for the existing validation pipeline.
6. Keep the default exported 10-row contract fixed unless a later item explicitly justifies an opt-in auxiliary buffer.
7. Every implementation phase must preserve or improve current performance on the serialized `double_ma_opt` benchmark surface.

## Consolidated Valid Work

### Phase 1: Immediate Exact Additions On Top Of The Existing Row / Reduction Path

These items are the strongest first-wave implementation targets because they mainly reuse the current row contract, current scoring path, or current device reduction path.

#### 1. Native GPU `Sortino` and `Omega` objective support

Why valid:

- The current backend already computes and decodes the necessary state for these objectives.
- The remaining gap is routing / scorer enablement, not a missing replay pass.

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

#### 2. Expanded device-side hard constraints

Why valid:

- Current device constraints are narrow.
- A broader device-side constraint surface reduces host filtering and keeps shortlist formation device-first.

Tier A first:

- `min_sharpe`
- `min_calmar`
- `max_volatility`
- `min_exposure`
- `max_exposure`
- `max_abs_net_exposure`

Tier B after Tier A:

- `min_omega`
- `max_ulcer`
- `max_downside_deviation`

Primary touchpoints:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

#### 3. Row-derived score heads and composite score families

Why valid:

- The bar walk is already paid for.
- Adding more score heads over the existing row is much cheaper than another backtest pass.

Recommended first-wave row-derived heads:

- `active_mean`
- `active_sortino`
- drawdown chronicity: `ulcer_index / max_drawdown`
- downside-share / downside-asymmetry proxy

Recommended first-wave composite families:

- constrained normalized additive utility
- adversarial / envelope-weight composite
- lower-confidence / shrinkage flavored score head, if kept additive and opt-in

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 4. Exact multi-head top-k and union-shortlist support

Why valid:

- The reduction machinery already exists.
- Multiple exact scalar heads can share the same optimization pass.

Scope:

- maintain exact top-k per scalar head
- build an opt-in union shortlist
- allow an optional host rerank only on the tiny deduplicated union shortlist
- keep host rerank explicitly separate from exact per-head top-k semantics

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/benches/double_ma_opt.rs`

### Phase 2: Exact Low-State Kernel Additions

These items add useful diagnostics or stress proxies while keeping state growth controlled.

#### 5. Underwater duration diagnostics

Why valid:

- The kernel already tracks equity and drawdown state.
- Duration-style drawdown diagnostics are a natural additive extension.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 6. Entry-margin fragility metrics

Why valid:

- The kernel already has the crossover decision context needed to accumulate margin-style fragility proxies.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 7. Exact fee-stress / stressed final-PnL transform

Why valid:

- Useful and additive if it is made exact.

Required qualification:

- Do not assume exported `trades` is identical to commission-event count.
- First prove equivalence across all entry / exit / flip / stop / take-profit branches, or add a dedicated internal `cost_event_count`.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_validate.rs`

### Phase 3: Exact Robustness Pull-Forward Using Optional Auxiliary State

These items are valid, but they should be implemented only after the row/reduction work above is stable. They need optional auxiliary state or post-pass logic, not a permanent widening of the default 10-row export contract.

#### 8. PSR / LCB-style shortlist features aligned with current validation semantics

Why valid:

- Useful as additive shortlist or ranking signals.

Required qualification:

- Current downstream PSR / DSR uses simplified skew / kurtosis assumptions.
- First-wave parity should target the current `validation_stats.rs` semantics.
- Treat richer `m3 / m4` probability metrics as a separate advanced mode, not implicit parity.

Primary touchpoints:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 9. Checkpoint slice robustness

Why valid:

- Half-split / quartile summaries can be accumulated from one pass if checkpoints are used.

Recommended first-wave slice summaries:

- worst-slice metric
- positive-slice fraction
- jackknife-like omission summary

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_validate.rs`

#### 10. Volatility-bucket robustness

Why valid:

- Existing validation already computes volatility-slice style robustness downstream.
- Optimization can accumulate low / mid / high volatility bucket summaries with one precomputed dataset state array.

Primary touchpoints:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

#### 11. Matrix-neighborhood robustness / basin / plateau metrics

Why valid:

- Additive optimization-stage surrogate for downstream perturb robustness.

Required qualification:

- This is not "free row algebra."
- It needs either shortlist-surface retention or a dedicated small surface buffer.
- It supplements the current perturb validation module; it does not replace it.

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/validation_pipeline.rs`

### Phase 4: Medium-Risk Exact Extensions

These items remain valid, but they should stay behind the earlier phases because they add more state, more semantics, or more exactness caveats.

#### 12. Hurdle partial moments

Candidate scope:

- `Omega_tau`
- `Kappa_tau`

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 13. Downside frequency / severity decomposition

Candidate scope:

- `count_neg`
- `sum_neg_abs`
- derived severity / frequency diagnostics

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 14. Side-conditional long / short summaries

Why valid:

- Exact and useful, but more stateful than the first-wave row algebra additions.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 15. Richer heatmap aggregates

Candidate scope:

- constrained pass rate
- mean score
- best constrained score
- robustness aggregate per cell

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/benches/double_ma_opt.rs`

#### 16. Pareto / lexicographic / epsilon-front exploration on a union shortlist

Why valid:

- Additive ranking mode.

Required qualification:

- Keep the exact scalar-head top-k path intact.
- Introduce this only as an opt-in shortlist or rerank mode.

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

## Explicit Non-Goals For This Tracker

These ideas may still be useful research directions, but they should not be implemented as part of this optimization-stage tracker:

- full optimization-stage `reopt_walk_forward`
- full optimization-stage trade bootstrap
- full optimization-stage cross-ticker validation
- replacing downstream SPA with an optimization-stage proxy
- replacing downstream CSCV / PBO with an optimization-stage proxy
- reintroducing a generic replay pass for finalist completion
- shrinking or redefining the current default 10-row export contract

## Implementation Order

1. Phase 1: objectives, constraints, row-derived heads, composite scoring, multi-head top-k / union shortlist.
2. Phase 2: underwater duration, entry-margin fragility, cost-event semantics, exact stressed final-PnL.
3. Phase 3: PSR / LCB alignment, checkpoint slice summaries, volatility buckets, basin / plateau metrics.
4. Phase 4: hurdle partial moments, downside frequency / severity, side splits, richer heatmaps, Pareto / lexicographic shortlist modes.

## Detailed Checklist

### Review Hygiene

- [ ] Confirm the current benchmark baseline in `crates/app_core/benches/double_ma_opt.rs` before Phase 1 starts.
- [ ] Capture the current default-path timings for:
  - `search_only`
  - `search_plus_heatmap`
  - `search_plus_finalist_recompute`
  - `end_to_end_product`
- [ ] Keep all benchmark runs serialized and record the command / environment used for each captured number.

### Phase 1A: Native Objective Support

- [ ] Remove the remaining GPU route blockers for `Sortino`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`
  Validation: CPU/GPU parity tests for objective selection and ranking order.

- [ ] Remove the remaining GPU route blockers for `Omega`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`
  Validation: CPU/GPU parity tests for objective selection and ranking order.

- [ ] Extend the device objective scorer so `Sortino` and `Omega` use the current single-pass row semantics directly.
  Files: `crates/app_core/src/kernels/double_crossover.cu`
  Validation: targeted scorer tests and end-to-end optimization parity on a fixed fixture.

### Phase 1B: Expanded Device Constraints

- [ ] Extend `DoubleMaConstraints` with first-wave exact constraints:
  - `min_sharpe`
  - `min_calmar`
  - `max_volatility`
  - `min_exposure`
  - `max_exposure`
  - `max_abs_net_exposure`
  Files: `crates/app_core/src/double_ma.rs`
  Validation: request decode / defaulting tests and CPU-only compile safety.

- [ ] Extend device-side constraint evaluation for the first-wave exact constraints.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: device-filter vs host-filter parity on fixed fixtures.

- [ ] Add second-wave 10-row risk constraints:
  - `min_omega`
  - `max_ulcer`
  - `max_downside_deviation`
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: constrained top-k parity and no-regression benchmark rerun.

- [ ] Ensure unsupported constraint combinations no longer force unnecessary host filtering when a device implementation exists.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: route-policy tests and log / instrumentation checks.

### Phase 1C: Row-Derived Heads And Composite Scoring

- [ ] Add row-derived helpers for `active_mean`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: compare against CPU-derived reference from the same candidate result set.

- [ ] Add row-derived helpers for `active_sortino`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: compare against CPU-derived reference with explicit exposure semantics checks.

- [ ] Add row-derived helper for drawdown chronicity (`ulcer / max_drawdown`).
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic decode / derived-metric tests.

- [ ] Add downside-share / downside-asymmetry style row-derived diagnostics.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic decode / derived-metric tests.

- [ ] Add one normalized additive composite score mode over existing row fields.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: CPU/GPU parity on composite ranking order for a fixed fixture.

- [ ] Add one adversarial / envelope-weight composite score mode over existing row fields.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: deterministic score-order tests and benchmark check.

- [ ] Add one effective-sample / lower-confidence opt-in score mode that penalizes low effective support without changing the default ranking modes.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: deterministic score-order tests, explicit opt-in behavior tests, and benchmark check.

### Phase 1D: Multi-Head Reduction / Shortlist Union

- [ ] Add exact per-head top-k support for multiple scalar heads in one pass.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: each head exactly matches standalone per-head ranking on the same fixture.

- [ ] Add opt-in union-shortlist materialization and deduplication.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: union membership tests and deterministic deduplication tests.

- [ ] Add optional host rerank over the tiny union shortlist only.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: explicit tests that host rerank is opt-in and does not alter exact per-head top-k semantics.

- [ ] Preserve current benchmark meaning after multi-head support:
  - `search_only` remains a first-pass benchmark
  - `end_to_end_product` remains the actual default product route
  Files: `crates/app_core/benches/double_ma_opt.rs`
  Validation: benchmark harness review and rerun.

### Phase 2: Low-State Exact Diagnostics

- [ ] Add underwater-duration accumulators and export / decode path.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: CPU/GPU parity on representative fixtures and no-regression benchmark rerun.

- [ ] Add entry-margin fragility accumulators and export / decode path.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic fixture tests and benchmark rerun.

- [ ] Audit whether exported `trades` is identical to commission-event count for every trade path.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: targeted path tests covering entry, exit, flip, stop-loss, and take-profit branches.

- [ ] If `trades != commission_event_count`, add a dedicated internal `cost_event_count`.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: path-level exactness tests and decode tests.

- [ ] Add exact stressed final-PnL derivation using the proven event-count semantics.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma_validate.rs`
  Validation: compare against downstream friction-stress reference behavior where semantics overlap.

### Phase 3A: Probability / Confidence Features

- [ ] Implement PSR / LCB-style shortlist features using semantics aligned with current `validation_stats.rs`.
  Files: `crates/app_core/src/validation_stats.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: parity against the current validation formula on fixed inputs.

- [ ] Keep richer `m3 / m4` probability metrics behind a separate research or advanced-mode gate.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/validation_stats.rs`
  Validation: explicit mode-separation tests.

### Phase 3B: Slice / Bucket Robustness

- [ ] Add optional half-split checkpoint snapshots.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: compare against CPU slice recomputation on a fixed fixture.

- [ ] Add optional quartile checkpoint snapshots.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: compare against CPU slice recomputation on a fixed fixture.

- [ ] Derive worst-slice, positive-slice fraction, and jackknife-like omission summaries from checkpoint state.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic summary tests and benchmark rerun.

- [ ] Add dataset-wide volatility bucket precompute and accumulate low / mid / high volatility candidate stats.
  Files: `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: compare against CPU bucket recomputation on fixed fixtures.

- [ ] Add opt-in benchmark-relative slice summaries such as beat-rate and worst-slice excess return, explicitly as additive optimization-stage summaries and not as replacements for downstream SPA.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/validation_pipeline.rs`
  Validation: compare against deterministic CPU reference calculations on fixed slices.

### Phase 3C: Surface Robustness

- [ ] Define a first-wave basin / plateau robustness metric that can be computed from a retained shortlist surface or small auxiliary buffer.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic neighborhood tests.

- [ ] Implement the first-wave basin / plateau metric as an additive optimization-stage robustness feature.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: fixed-surface tests and explicit separation from downstream `perturb` validation.

### Phase 4: Medium-Risk Exact Extensions

- [ ] Add hurdle partial moments:
  - `Omega_tau`
  - `Kappa_tau`
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: CPU/GPU parity and benchmark rerun.

- [ ] Add downside frequency / severity state:
  - `count_neg`
  - `sum_neg_abs`
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: CPU/GPU parity and benchmark rerun.

- [ ] Add side-conditional long / short summary state.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: CPU/GPU parity and benchmark rerun.

- [ ] Add richer heatmap aggregates only after the earlier phases are stable.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/benches/double_ma_opt.rs`
  Validation: heatmap correctness tests and no-regression benchmark rerun.

- [ ] Add opt-in trade-level metrics only after the earlier phases are stable and only if the benchmark budget remains intact.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: CPU/GPU parity and no-regression benchmark rerun.

- [ ] Add opt-in Pareto / lexicographic / epsilon-front union-shortlist modes.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: deterministic shortlist tests and explicit preservation of exact scalar-head ranking paths.

### Cross-Cutting Safety / Verification

- [ ] After every completed phase, rerun the serialized `double_ma_opt` benchmark and record results in this file or a linked tracker.
- [ ] After every kernel-state change, rerun targeted CUDA parity tests for the affected metrics or ranking modes.
- [ ] Keep CPU-only compilation / tests green after every phase.
- [ ] Preserve the current default 10-row product path behavior when the new capability is not explicitly enabled.
- [ ] Preserve the current validation stage APIs and semantics unless a later tracked change explicitly updates them.

## Notes On Stale Or Qualified Review Items

- The earlier reviews sometimes speak as if a full candidate matrix is already retained by default. The current runtime is still tile-based, so matrix-neighborhood work must account for that.
- Active-time algebra is useful, but any claim of "exact active Sharpe" is conditional on observation-count reconstruction.
- Exact fee-stress transforms are valid only if commission-event semantics are proven or carried explicitly.
- Optimization-stage robustness proxies are additive. They do not replace walk-forward, bootstrap, SPA, CSCV/PBO, or cross-ticker validation.
