# Trade22 CUDA Exact Selection Implementation Guide

Created: 2026-04-26
Status: implementation planning guide

This document scopes the next CUDA and exact-selection performance
investigation around the Trade22 exact metric surface. The goal is to determine
whether safer pruning, smarter row-first selection, shortlist-level robustness,
and adaptive search can produce a measurable improvement without regressing the
current exact kernel route.

## Intent

Use the existing Trade22 matrix as the canonical rich exact metric surface, then
extract more decision value from that surface before host materialization. Do
not turn the next iteration into Trade30 or Trade40 unless the new fields are
fixed-state online path metrics and profiling proves they belong in the hot
bar loop.

The target shape is:

```text
exact candidate path walk
  -> bounded metric surface on device
  -> exact feasibility pruning where math permits
  -> device-side constraints
  -> device-side objective heads / score heads
  -> compact device-side shortlist composition where it pays
  -> shortlist/finalist statistical validation
  -> adaptive refinement only where exhaustive exact search is too expensive
  -> bounded D2H rows and diagnostics
```

The work is only successful if it has correctness evidence and benchmark
evidence. A faster micro-path is not enough if default routes slow down, route
fallback behavior changes, CPU-only builds break, or shortlist semantics drift.

## Placement Principles

Use placement as the first design filter:

- Online path functionals belong in the path walk only when they are bounded,
  fixed-state recurrences. The current Trade22 tail is justified by this rule.
- Completed-row functions belong after metric-row production. Top-k, score
  heads, scalar objective unions, and compact Pareto/epsilon-front passes should
  generally consume completed metric rows instead of adding per-bar state.
- Statistical confidence, bootstrap, walk-forward, CSCV/PBO, future
  CPCV-style diagnostics, and multiple-testing corrections belong on the
  shortlist or finalists, not the full grid.
- Adaptive search can propose candidates, but reported winners must still be
  exact evaluations unless the product explicitly labels an approximate result.
- Path compression and signature deduplication are separate experiments. Enable
  them only behind exactness gates that prove identical execution semantics.

## Current Evidence Snapshot

Trade22 production is already fused into the path walk:

- `crates/app_core/src/double_ma.rs` defines `DoubleMaMetricContract::Trade22`.
- `crates/app_core/src/gpu_backtest_kernel.rs` promotes exact rows to
  `TRADE_METRICS_COUNT = 22` when `metric_contract=Trade22` or
  `include_trade_metrics=true`, except that an explicit
  `metric_contract=Full17` wins over `include_trade_metrics=true`.
- `crates/app_core/src/kernels/double_crossover.cu` accumulates
  `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and
  `gross_loss` during the same candidate path walk.

The current CUDA path already does some post-row extraction on device:

- Primary top-k selection runs on device for the global top-k path.
- Metric-only constraints run on device when the v2 selector path is available.
  These are post-row selector constraints, not early path-pruning constraints.
- Extra shortlist objective heads are selected on device from the same tile
  metric rows and merged into compact per-head global buffers.

The current CUDA path still has host-side shortlist composition:

- Extra-head rows are materialized to host before final union mode reranking.
- `shortlist_union_mode` is applied in Rust through
  `rerank_shortlist_by_union_mode`.
- `shortlist_confidence_mode` is applied in Rust through
  `rerank_shortlist_by_confidence`.
- `score_mode` currently forces `device_include_all`, and advanced shortlist
  union/confidence requests reject active `score_mode`.
- `shortlist_slice_mode` performs bounded exact GPU replays over slice ranges.
- `shortlist_neighborhood_radius` currently calls local stability after
  optimization and recomputes the neighborhood surface.

The row-first path should remain the default baseline:

- The supplied performance notes report score-first/global-head experiments on
  ALMA exact surfaces that were materially slower than the row-first route.
  Treat those large-case notes as a warning sign: full17 score-first around
  `124 ms` versus row-first around `70 ms`, and trade22 score-first around
  `126 ms` versus row-first around `75 ms`.
- New scoring work should therefore consume completed rows or compact candidate
  buffers unless fresh profiling proves a different layout wins.
- Any score-first experiment must stay opt-in and must be compared against the
  current row-first route, not against CPU or a synthetic baseline.
- Row-first global top-k is the baseline only when the request does not force
  include-all, export, host filtering, or another incompatible route.

## Source Truth

Primary implementation files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - host CUDA orchestration, scratch buffers, route diagnostics, top-k merging,
    shortlist materialization, final reranks, and timing telemetry.
- `crates/app_core/src/kernels/double_crossover.cu`
  - metric production kernels, selector kernels, gather kernels, and any new
    device-side score/union kernels.
- `crates/app_core/src/double_ma.rs`
  - request contract, score-mode definitions, CPU score functions, route
    validation, follow-up cache, local stability, and high-level tests.
- `crates/app_core/src/double_ma_vdps.rs`
  - VDPS survivor/refill support and quality-check telemetry merging.
- `crates/app_core/src/validation_stats.rs`
  - PSR/DSR confidence helper entry points.
- `crates/app_core/src/sharpe_stats.rs`
  - probabilistic/deflated Sharpe and effective-sample helpers.
- `crates/app_core/src/validation_pipeline.rs`
  - walk-forward, SPA-style validation, stationary bootstrap, and validation
    reduction logic.
- `crates/app_core/src/cscv_pbo.rs`
  - CSCV/PBO validation support used by the validation pipeline.

Secondary files:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`

## External Math References

These references support the statistical validation terms used in this guide.
They do not override repo behavior; implementation still needs local parity
against the existing Rust code.

- Newey and West, heteroskedasticity and autocorrelation consistent covariance:
  https://ssrn.com/abstract=225071
- Bailey and Lopez de Prado, Deflated Sharpe Ratio:
  https://ssrn.com/abstract=2460551
- Bailey, Borwein, Lopez de Prado, and Zhu, Probability of Backtest Overfitting
  and CSCV:
  https://ssrn.com/abstract=2326253
- White, Reality Check for Data Snooping:
  https://www.jstor.org/stable/2999444
- Hansen, Superior Predictive Ability test:
  https://ssrn.com/abstract=264569

## Non-Goals

- Do not replace the Trade22 path-walk kernel.
- Do not add more fields to Trade22 unless the new field is a fixed-state
  online metric that is naturally accumulated during the bar walk.
- Do not make CUDA mandatory for normal builds or tests.
- Do not remove CPU fallback or host fallback paths.
- Do not move slice robustness into a fake metric-row calculation. Slice
  robustness needs slice-local path metrics and can continue to replay unless a
  measured alternative exists.
- Do not assume neighborhood robustness is free from current rows. The optimizer
  currently tiles the metric matrix and does not retain the full exact grid as a
  reusable resident matrix after final top-k selection.
- Do not run full-grid bootstrap, CSCV/PBO, CPCV-style diagnostics, White
  Reality Check, SPA, or multiple-testing correction as part of the main
  exhaustive candidate pass. These are shortlist/finalist validation tools.
- Do not run global Pareto over every metric and every candidate by default.
  Build scalar-head unions first, then apply Pareto or epsilon-front logic to
  the compact union if it is still useful.
- Do not enable signal-signature deduplication unless the strategy semantics
  prove that identical signatures imply identical realized paths.
- Do not start with block-transducer time parallelism. Keep that as a later
  research path for narrow, risk-off, overlay-off surfaces after simpler exact
  improvements plateau.

## Non-Regression Policy

Do not make a new CUDA decision path default until all of these are true:

- CPU-only builds still pass without CUDA tooling.
- Existing `Full17` and `Trade22` metric contract tests still pass.
- Existing route diagnostics remain accurate.
- Existing default GPU top-k results are unchanged, except for documented and
  explicitly requested ranking-mode changes.
- Default GPU perf gate cases do not regress beyond their configured budgets.
- Advanced shortlist cases have measurable improvement on at least one
  representative workload.
- D2H bytes and host-finalize time do not increase on default routes.
- Any new CUDA kernel compiles to fresh PTX locally before handoff.
- Early-reject, signature-dedup, event-path, and surrogate paths carry explicit
  exactness gates. A stale threshold may reduce pruning, but it must never
  reject a candidate that could still satisfy constraints or beat the current
  kth result under the documented objective.

Performance experiments must be serialized. Before running benchmarks,
profilers, Nsight, CUPTI, or Criterion, verify no other benchmark/profiling
process is active.

## Viability Threshold

Treat this as an experiment until the data proves otherwise.

A change is viable if it meets at least one of these criteria without default
regressions:

- Reduces advanced exact-kernel optimization wall time by at least 5% on a
  workload using `Trade22`, extra shortlist objectives, and union or confidence
  reranking.
- Reduces measured host finalize time for shortlist union/confidence by at
  least 25% on the same workload.
- Reduces D2H traffic for advanced shortlist extraction by at least 25% while
  preserving output parity.
- Reduces constrained large-grid exact-route wall time by at least 5% through
  exact feasibility pruning while preserving parity against the no-pruning path.
- Produces useful instrumentation that falsifies a proposed optimization. For
  example, a signature-dedup unique ratio near `1.0` is a valid stop condition.

If the improvement is below those thresholds, keep the path behind an opt-in
environment flag or do not ship it.

## Recommended Priority Order

Use this order unless baseline measurements clearly reorder the bottlenecks:

1. Exact feasibility pruning for monotone and bounded constraints.
2. Device-side row-only score heads from completed metric rows.
3. Compact device-side shortlist union and dedup.
4. VDPS holdout-regret completion and uncertainty-aware promotion/injection.
5. Shortlist-level robust validation: PSR/DSR, existing walk-forward and
   CSCV/PBO surfaces, SPA-style checks, stationary/bootstrap diagnostics, and
   multiple-testing-aware reporting. Treat purged/CPCV-style diagnostics as a
   future contract, not something already shipped in this repo.
6. Signal-signature dedup instrumentation.
7. Local quadratic trust-region refinement using existing surrogate admission
   diagnostics.
8. Event/run-length backtesting prototype for narrow simple surfaces.
9. Block-transducer time parallelism only after the simpler exact approaches
   plateau.

## Phase 0: Baseline And Instrumentation

Purpose: measure whether host-side union/confidence work is large enough to
justify CUDA implementation.

Tasks:

- [ ] Build a repeatable advanced exact-kernel workload with:
  - `metric_contract=Trade22`
  - `top_k > 0`
  - `include_all=false`
  - `extra_shortlist_objectives` with two or three heads
  - each `shortlist_union_mode`
  - `shortlist_confidence_mode=psr` and `dsr`
- [ ] Build at least one constrained large-grid workload where exact pruning
  could matter:
  - max drawdown limit
  - min/max exposure bounds
  - min trade count
  - large enough candidate count that rejected bar walks can move wall time
- [ ] Preserve an unconstrained row-first baseline so pruning overhead is
  measured when little or nothing can be rejected.
- [ ] Capture baseline timings with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`.
- [ ] Record:
  - total optimization time
  - `host_finalize_shortlist_union_ms`
  - `host_finalize_shortlist_confidence_ms`
  - `host_finalize_shortlist_slice_ms`
  - D2H copy time
  - host decode time
  - route diagnostics
  - top row identity and metrics
- [ ] Add instrumentation fields only where needed:
  - early-pruned candidate count by reason
  - bars skipped by early pruning
  - signature unique ratio, average bucket size, and max bucket size
  - VDPS quality-check holdout regret once the value is implemented
- [ ] Add missing telemetry only if current timing output cannot isolate the
  relevant cost.
- [ ] Save the baseline command lines and results under `output/` if the run is
  used to justify implementation.

Suggested baseline commands:

```powershell
powershell -File scripts/run_workflow_gate.ps1 -Mode candidate
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

For targeted local runs, keep environment overrides explicit:

```powershell
$env:VECTORBT_KERNEL_PROFILE_PIPELINE="1"
$env:VECTORBT_KERNEL_GLOBAL_TOPK="1"
$env:VECTORBT_KERNEL_FORCE_LEGACY_TOPK="0"
```

Acceptance:

- [ ] A clear baseline exists before kernel changes.
- [ ] The measured host-side shortlist cost is large enough to justify CUDA
  work, or the effort stops here.

## Phase 0A: Exact Feasibility Pruning

Purpose: skip work only when a candidate can be rejected without changing the
exact result.

This phase is proposal-only today. The current code has post-row device
constraints and host fallback filtering, but no early path-pruning
implementation or environment flag.

This is the cleanest first implementation candidate because it can reduce
wasted bar walks without changing metric definitions or shortlist semantics.
The expected win is workload-dependent: it helps most on large constrained
grids and may be neutral or slightly negative on unconstrained small grids.

Initial exact rejects:

- `max_dd_so_far > max_drawdown_limit`
  - Max drawdown is monotone; once breached, the candidate cannot become valid.
  - The comparison must use the same raw/log drawdown representation as the
    active metric row and selector constraint path.
- `(abs_pos_sum_so_far + remaining_bars) / total_obs < min_exposure`
  - Even maximum future exposure cannot meet the lower bound.
- `abs_pos_sum_so_far / total_obs > max_exposure`
  - Exposure already exceeds the upper bound.
- `trades_so_far + max_possible_future_trade_increments < min_trades`
  - Even the safest upper bound on future trade increments cannot meet the
    lower bound. This bound must match the execution contract. In the current
    kernels, a direct flip can count as two trade increments, so assuming only
    one future trade increment per bar would be unsafe for those modes.

Ranking-bound pruning is allowed only for PnL-like objectives with a
conservative upper bound:

```text
candidate_upper_bound = current_log_eq + sum_remaining_abs_log_returns
```

For long-only surfaces, `sum_remaining_positive_log_returns` may be used if the
long-only contract is enforced. The current kth threshold may be stale; stale
thresholds are safe but less effective. Do not apply this PnL upper bound to
Sharpe, Omega, Calmar, robust score modes, or union modes unless an
objective-specific conservative bound has also been proven.

Tasks:

- [ ] Add prefix/suffix return-bound data only if the measured cost is small
  relative to candidate bar-walk savings.
- [ ] Start with monotone constraint pruning before kth-threshold ranking
  pruning.
- [ ] Add per-reason counters and skipped-bar counters to diagnostics.
- [ ] Keep pruning disabled by default behind an environment flag.
- [ ] Compare pruned and unpruned outputs on deterministic fixtures.
- [ ] Include constrained and unconstrained perf cases so overhead is visible.

Acceptance:

- [ ] Pruned and unpruned top-k outputs match for deterministic exact fixtures.
- [ ] Invalid candidates rejected early match host/device constraint semantics.
- [ ] Unconstrained workloads do not regress beyond perf budgets.
- [ ] Constrained large-grid workloads meet the viability threshold before
  making the path default.

## Phase 1: Preserve Current Device Multi-Head Extraction

Purpose: make sure future changes do not weaken what already works.

Tasks:

- [ ] Add or tighten tests proving extra shortlist heads still select on the
  exact global top-k kernel path.
- [ ] Assert that requests with `extra_shortlist_objectives` still reject
  incompatible host-filter, export, include-all, and active score-mode paths.
- [ ] Preserve per-head global buffers in `ExtraShortlistHeadState`.
- [ ] Preserve device-side constraints for the v2 selector path.

Acceptance:

- [ ] Existing row-first global top-k tests still pass.
- [ ] Extra-head output parity matches the current host-finalized result.
- [ ] Route diagnostics still identify `row_first_global_topk` when expected.

## Phase 2: Device Score Heads For Row-Only Score Modes

Purpose: avoid `device_include_all` and host scoring for score modes that are
pure functions of completed metric rows.

Initial scope:

- `BalancedRiskAdjusted10`
- `AdversarialEnvelope10`
- `SupportShrunk10`
- `ActiveSortino10`
- `OmegaVsSharpe10`
- `RobustMargin10`

Defer initially:

- `EffectiveSampleLcb10`, because it needs pair observation context in addition
  to the metric row.

Tasks:

- [ ] Add a CUDA score-mode enum that mirrors `DoubleMaScoreMode` for row-only
  modes.
- [ ] Add device helper functions in `double_crossover.cu` that match the Rust
  score functions in `double_ma.rs`.
- [ ] Add a selector path that scores metric rows by score mode without forcing
  `device_include_all`.
- [ ] Keep the new score-mode selector behind an opt-in flag until parity and
  performance are proven.
- [ ] Add direct CUDA tests comparing device scores to Rust score functions for
  representative metric rows.
- [ ] Add end-to-end tests proving top-k identity matches the host score-mode
  path for each row-only score mode.

Acceptance:

- [ ] Row-only score modes no longer require full host materialization on the
  experimental path.
- [ ] Top-k order matches the host implementation for deterministic fixtures.
- [ ] No default route changes until the GPU perf gate passes.

## Phase 3: Device-Side Compact Union Composition

Purpose: reduce host-side union reranking and D2H work after multiple objective
heads have already been extracted from the same metric surface.

Preferred implementation order:

1. Device dedup of compact per-head top-k rows.
2. Device lexicographic union ranking.
3. Device weighted-rank union ranking.
4. Device Pareto and epsilon-front ranking only if Phase 0 shows enough host
   cost to justify the more complex O(k^2) logic.

Tasks:

- [ ] Add a compact union scratch buffer sized from `top_k` and head count, not
  from full grid size.
- [ ] Add a device kernel that merges primary and extra-head compact rows and
  deduplicates by `(fast_len, slow_len)`.
- [ ] Add a device ranking kernel for lexicographic union mode.
- [ ] Add a device ranking kernel for weighted-rank union mode.
- [ ] Copy back only the final compact union rows needed by the result.
- [ ] Keep the existing Rust `rerank_shortlist_by_union_mode` as the oracle and
  fallback.
- [ ] Add parity tests for all supported union modes, including duplicate rows
  across heads and tie-breakers.

Acceptance:

- [ ] Device union output matches the host oracle on deterministic fixtures.
- [ ] D2H row count is no larger than the current compact materialization path.
- [ ] The path falls back to host union if required CUDA symbols are missing.
- [ ] The path remains opt-in until GPU perf evidence clears the viability
  threshold.

## Phase 4: Shortlist Confidence And Robust Validation

Purpose: improve ranking confidence on compact candidates without moving
expensive statistical validation into the full-grid hot loop.

This phase has two layers. First, evaluate whether PSR/DSR reranking can move
to device without changing semantics. Second, strengthen shortlist/finalist
validation with statistical methods that are too expensive for every candidate.
If confidence reranking is not a measured host-side cost, keep it on host and
focus on robust finalist validation.

Tasks:

- [ ] Decide whether peer mean/std should be computed on device or copied as a
  compact precomputed scalar pair from host.
- [ ] Match `validation_stats::compute_psr_dsr` semantics exactly or keep the
  host path.
- [ ] Add a compact confidence score buffer for materialized shortlist rows.
- [ ] Add a device confidence ranking path for `psr` and `dsr`.
- [ ] Keep the host confidence reranker as the oracle and fallback.
- [ ] Keep HAC/Newey-West style lower-confidence bounds as finalist replay or
  shortlist validation work initially. They require lagged return state and
  should not be added to the core m17/m22 hot kernel first.
- [ ] Extend the shortlist confidence contract only through existing surfaces
  where possible:
  - PSR / DSR
  - effective-sample penalties
  - stationary block bootstrap percentile
  - current walk-forward / reopt walk-forward diagnostics
  - current CSCV/PBO and SPA-style validation diagnostics
  - future purged/CPCV-style stability diagnostics only after adding an
    explicit local contract
  - multiple-testing-aware reporting such as deflated Sharpe or Reality
    Check / SPA-style p-values
- [ ] Rank or report robust scores as:

```text
score = point_estimate - uncertainty_penalty - multiple_testing_penalty
```

- [ ] Run current bootstrap and SPA-style diagnostics, plus any future
  CPCV/Reality-Check-style corrections, on the shortlist/finalists only.

Acceptance:

- [ ] PSR/DSR order matches host for deterministic fixtures.
- [ ] Numeric differences are bounded and cannot flip documented tie-breakers.
- [ ] The path improves measured advanced shortlist workload time.
- [ ] New robust validation fields do not affect default optimization ranking
  unless explicitly requested.

## Phase 5: Neighborhood Robustness From Existing Rows

Purpose: investigate whether neighborhood robustness can reuse already evaluated
rows in cases where the required local surface is available.

This is lower priority than score heads and union composition.

Current limitation:

- The optimizer computes tiled metric rows and retains compact top-k heads, not
  the full exact grid. For most normal runs, the neighborhood around the final
  best candidate is not resident after optimization finalization.
- Current request validation also ties neighborhood robustness to slice-mode
  shortlist support, so any new reuse path needs to preserve that contract or
  explicitly change it with tests.

Potential approaches:

- Retain a bounded device-side neighborhood buffer only after a candidate is
  known.
- Perform a second device pass over only the requested neighborhood.
- Use existing local stability recompute when the grid surface was not retained.

Tasks:

- [ ] Measure current local-stability recompute cost before changing it.
- [ ] Do not add full-grid retention just for neighborhood diagnostics unless
  memory and timing evidence supports it.
- [ ] If implemented, keep the feature bounded by radius and step and preserve
  the existing local-stability summary as the oracle.

Acceptance:

- [ ] No additional full-grid memory residency is introduced on default routes.
- [ ] Neighborhood diagnostics match existing local-stability summaries.
- [ ] Any new path is skipped when the local surface is not already available or
  cheap to recompute.

## Phase 6: VDPS Uncertainty And Regret

Purpose: make adaptive search more trustworthy before expanding it.

The current VDPS telemetry already has fields for quality-check overlap, score
regret, exact winner rank, holdout regret, and speedup. The holdout-regret
field is present in the result shape, but current quality-check code leaves it
empty.

Core VDPS optimization does not currently carry a holdout evaluation context.
Filling holdout regret therefore requires either a new holdout scoring input or
a bounded quality-check path that can compute both exact and VDPS holdout scores.

Tasks:

- [ ] Fill `quality_check_holdout_regret` when a holdout objective is available:

```text
quality_check_holdout_regret =
  exact_holdout_score(best_exact_full_grid) - exact_holdout_score(best_vdps)
```

- [ ] Add tests proving the field is populated only when both holdout scores
  are available.
- [ ] Add optional uncertainty-aware VDPS promotion:

```text
promotion_score = estimated_score - lambda * uncertainty
exploration_score = estimated_score + beta * uncertainty
```

- [ ] Promote high-LCB candidates, inject high-UCB candidates, and mutate around
  stable local basins.
- [ ] Keep VDPS winners exact-evaluated before reporting.
- [ ] Treat uncertainty-aware promotion as new behavior. Current VDPS refill
  supports deterministic seeds plus random, elite mutation, and differential
  injection; it does not already rank promotions by LCB/UCB.

Acceptance:

- [ ] VDPS quality-check telemetry reports real holdout regret when possible.
- [ ] Uncertainty-aware promotion is opt-in until overlap, regret, and speedup
  evidence justify enabling it.
- [ ] Adaptive search diagnostics make missed exact winners visible rather than
  hiding them behind speedup claims.

## Phase 7: Signal-Signature Dedup Instrumentation

Purpose: measure whether exact signal deduplication is worth implementing.

For plain crossover semantics, neighboring MA pairs can produce identical
desired-position sequences. If two candidates have the same signal sequence,
same `start_t`, same regime inputs, and same execution overlay contract, their
realized path is identical and one exact path walk can be scattered to all
equivalent candidate ids.

This is not the same as the existing `fast_warp_dedup` CUDA path, which reduces
duplicated fast-MA loads within a warp. Signal-signature dedup would deduplicate
candidate execution paths and scatter completed metric rows.

Do not implement full dedup first. Instrument the opportunity first.

Enable signature instrumentation only when all gates are true:

- no `regime_hybrid`
- no signal mode that depends on raw spread beyond sign
- same `start_t`
- same execution overlay contract
- no stop/take/cooldown/persistence mode that changes equivalence without being
  represented in the signature

Metrics:

- `unique_signature_count / candidate_count`
- average candidates per signature
- max bucket size
- time spent hashing/signature packing
- estimated bar walks avoided if full dedup existed

Acceptance:

- [ ] If the unique ratio is near `1.0` on real workloads, stop the effort.
- [ ] If the unique ratio is materially below `1.0`, add a separate design for
  bit-packed signatures, device/host hash placement, and metric-row scatter.
- [ ] Any later dedup implementation proves candidate metrics match the
  no-dedup exact path byte-for-byte or within existing numeric tolerances.

## Phase 8: Local Trust-Region Refinement

Purpose: use the existing local surrogate admission machinery to decide where
to spend additional exact evaluations, not to report approximate winners.

The current local robustness surface already tracks neighborhood roughness,
curvature, anisotropy, cliff flags, path-change proxies, and
`LocalSurrogateAdmissionSummary`. Treat that as the basis for a bounded
trust-region optimizer.

Current surrogate admission already gates on metric-only score modes, neighbor
availability, elite margin, plateau, roughness, and curvature. The path-change
gate below is a proposed stricter trust-region requirement, not current
admission behavior.

Tasks:

- [ ] Fit a local quadratic around exact elites:

```text
s(f, s) = a + bf*x + bs*y + cf*x^2 + cs*y^2 + cfs*x*y
```

- [ ] Compute a residual bound from exact evaluated neighbors.
- [ ] Admit the surrogate only when:
  - elite margin is greater than `2 * residual_bound`
  - roughness and curvature gates pass
  - path-change proxy is quiet
  - score mode is metric-only or explicitly supported
- [ ] Use admitted surrogates only to propose the next exact evaluations.

Acceptance:

- [ ] Final reported winners are exact.
- [ ] Surrogate admissions and rejections are visible in diagnostics.
- [ ] The path improves evaluation allocation on expensive surfaces without
  changing exact result semantics.

## Phase 9: Event/Run-Length Backtesting Prototype

Purpose: test candidate path compression only on narrow surfaces where exactness
is provable.

The most advanced exact direction is:

```text
MA pair -> signal/change events -> compressed strategy path
        -> prefix/block metric algebra
```

This can reduce work from `O(candidates * bars)` toward
`O(candidates * signal_changes)` when signal changes are sparse. It should not
be mixed into the main Trade22 kernel until a prototype proves both exactness
and end-to-end value.

Tiered scope:

- Tier A: exact event path for PnL, Sharpe, Sortino, Omega, exposure, side means,
  and other prefix-sum-compatible metrics.
- Tier B: exact max-drawdown block composition using max-prefix/min-prefix
  summaries.
- Tier C: ulcer and underwater-duration exactness only if the block transducer
  remains manageable.

Initial gates:

- regime off
- risk overlays off
- stop/take/cooldown/persistence off
- commission off or trivially composable
- eps0/static signal semantics
- Full17/Trade22 lockstep kernels only

Acceptance:

- [ ] Prototype metrics match bar-walk metrics on deterministic fixtures.
- [ ] The event path reports its supported contract and refuses unsupported
  overlays.
- [ ] End-to-end timing beats row-first bar-walk on at least one realistic
  sparse-signal workload before integration work continues.

## Phase 10: Block Transducers And Conditional Contracts

Purpose: keep high-risk mathematical extensions visible without making them the
next default implementation.

Block transducers can parallelize time by composing block summaries:

```text
block_summary(input_state) -> output_state + metric_delta
B3 o B2 o B1
```

This is mathematically appealing because composition is associative, but the
state grows quickly with position, log equity, log peak, trade-open state,
cooldown, pending signal, and overlay state. Treat it as later research for
narrow, risk-off, overlay-off surfaces.

Regime-conditional metrics are a more product-facing extension than Trade30:

- score in trend regime
- score in range regime
- score in high-vol regime
- score in low-vol regime
- long-side score
- short-side score
- worst conditional score

Finalist validation already has descriptive regime-slice robustness summaries
for market-state buckets. This phase is about whether to add an optimization
metric contract or finalist replay mode for conditional ranking, not about
creating the first regime diagnostics in the product.

Rank with:

```text
robust_score = mean_score - lambda * regime_dispersion
robust_score = min_regime_score
```

Implementation guidance:

- Prefer a separate `RobustConditional` contract or finalist replay mode over
  bloating Trade22.
- If the regime series is already device-resident, test streamable conditional
  accumulators behind an opt-in metric contract.
- Keep multiple-testing-aware selection in the validator/report layer unless a
  compact shortlist device helper has a measured payoff.

## Testing Plan

Add tests close to the changed surface.

Rust unit and integration coverage:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - device score helpers versus Rust score helpers
  - compact union dedup and ranking parity
  - CUDA symbol fallback behavior
  - exact pruning parity against unpruned path
  - pruning diagnostics and skipped-bar counters
  - signature instrumentation gates and telemetry, if added here
- `crates/app_core/src/double_ma.rs`
  - end-to-end request behavior
  - top-k identity parity for `Trade22`
  - union/confidence result parity
  - route diagnostic expectations
  - VDPS holdout-regret telemetry
  - local surrogate admission and trust-region proposal behavior
- `crates/app_core/src/double_ma_vdps.rs`
  - uncertainty-aware promotion/refill scoring
  - quality-check telemetry merge behavior
- `crates/app_core/src/validation_stats.rs` and
  `crates/app_core/src/sharpe_stats.rs`
  - PSR/DSR and effective-sample confidence math
  - any HAC/Newey-West helper added for shortlist validation
- `crates/app_core/src/validation_pipeline.rs`
  - shortlist/finalist bootstrap, SPA-style, and walk-forward diagnostics
- `crates/app_core/src/cscv_pbo.rs`
  - CSCV/PBO validation fixtures and edge cases
- `web/src/services/optimizeRequestBuilder.test.ts`
  - request shaping remains unchanged unless a new explicit knob is added
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
  - capability text reflects opt-in or default status accurately

Correctness commands:

```powershell
cargo test -p ta_app_core
powershell -File scripts/run_workspace_tests_stable.ps1
```

CUDA correctness commands when a GPU is available:

```powershell
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

For low-level CUDA edits, compile fresh PTX before handoff. If Nsight or CUDA
profiling is needed and attach permissions fail, rerun from elevated
PowerShell.

## Performance Plan

Run measurements in this order:

1. Baseline current main branch behavior.
2. Enable the experimental path through an environment flag.
3. Run the same workload with identical inputs and no overlapping benchmarks.
4. Compare route diagnostics, top rows, D2H timing, host finalize timing, and
   wall time.
5. Compare constrained and unconstrained cases for pruning overhead.
6. Compare VDPS overlap, score regret, holdout regret, and speedup when adaptive
   changes are involved.
7. Run the default GPU perf gate before making the path default.

Minimum evidence to promote a path:

- Baseline and candidate outputs for the same workload.
- Route diagnostics showing the intended device path.
- Top-k parity or documented requested ranking-mode change.
- Timing evidence for the advanced workload.
- Default GPU perf gate pass.
- CPU-only safety pass for non-CUDA build behavior.
- For pruning and compression paths, parity against an explicit no-pruning or
  no-compression oracle.

## Rollback And Feature Gating

Every experimental path should have a narrow rollback point.

- Use an opt-in environment flag while evaluating performance.
- Keep host implementations as the semantic oracle.
- Fall back to host when CUDA symbols are unavailable.
- Fall back to host when unsupported score modes, union modes, or metric strides
  are requested.
- Do not remove host-finalize timing telemetry after moving logic to device.

Suggested experimental flags:

- `VECTORBT_KERNEL_EXACT_PRUNING`
- `VECTORBT_KERNEL_ROW_SCORE_HEADS`
- `VECTORBT_KERNEL_DEVICE_SHORTLIST_UNION`
- `VECTORBT_KERNEL_DEVICE_SHORTLIST_CONFIDENCE`
- `VECTORBT_KERNEL_SIGNATURE_DEDUP_INSTRUMENT`
- `VECTORBT_VDPS_UNCERTAINTY`
- `VECTORBT_KERNEL_EVENT_PATH_PROTOTYPE`

Avoid `VECTORBT_KERNEL_DEVICE_SCORE_MODE` for row-only score-head work because
it is too easy to confuse with the existing `VECTORBT_KERNEL_SCORE_FIRST_TOPK`
experiment. Use the `VECTORBT_VDPS_*` namespace for VDPS behavior unless the
feature is truly kernel-local.

Do not make these default until the viability threshold is met and the relevant
perf gates pass.

## Completion Checklist

- [ ] Phase 0 baseline proves this work is worth doing.
- [ ] Exact pruning either ships with parity and constrained-workload wins or is
  left disabled with diagnostic evidence.
- [ ] Device score heads match Rust score functions.
- [ ] Device compact union matches host union reranking.
- [ ] Confidence reranking is either implemented with parity or explicitly
  deferred based on measurements.
- [ ] VDPS holdout regret is populated where the inputs exist.
- [ ] Signature dedup has instrumentation results before any full dedup path is
  designed.
- [ ] Event/run-length and block-transducer ideas remain prototype-only unless
  exactness and timing evidence are both strong.
- [ ] Neighborhood robustness is either improved with bounded evidence or
  explicitly deferred.
- [ ] Default GPU perf gates pass.
- [ ] CPU-only safety remains intact.
- [ ] Documentation and capability copy match the final shipped behavior.
