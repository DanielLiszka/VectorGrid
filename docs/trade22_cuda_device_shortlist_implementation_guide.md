# Trade22 CUDA Exact Selection Implementation Guide

Created: 2026-04-26
Status: implementation guide with measured Phase 0 evidence, opt-in pruning,
row-score head paths, telemetry-only later phases, and deferred device
union/confidence work

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

## Measured Evidence Snapshot: 2026-04-26

The first full pass measured the proposed "post-row device extraction" work
before adding more CUDA kernels. Results are local GPU runs on an RTX 4090 with
Nsight Compute 2026.1 available. Raw run logs and profiler reports are preserved
under `output/`.

Phase 0 advanced shortlist measurements:

- Large Trade22 baseline and union/confidence runs used
  `VECTORBT_KERNEL_PROFILE_PIPELINE=1`, `VECTORBT_KERNEL_GLOBAL_TOPK=1`, and
  `VECTORBT_KERNEL_FORCE_LEGACY_TOPK=0`.
- `large`, `120000` candles, `top_k=64`, Trade22, choppy candles:
  baseline wall/route averaged about `50.5 ms`.
- Lexicographic and weighted-rank union runs averaged about `45.5-46 ms`.
  Pipeline telemetry showed host union below JSON millisecond resolution.
- PSR and DSR confidence runs averaged about `44.5-45 ms`. Pipeline telemetry
  showed host confidence around `0.011-0.024 ms`.
- The stress case (`xlarge`, `200000` candles, `top_k=256`, extra objectives
  `pnl,omega,calmar`, weighted-rank union) averaged about `330.5 ms` route time,
  with host union around `0.134-0.207 ms`.

Phase 0A pruning measurements:

- With `VECTORBT_KERNEL_EXACT_PRUNING=0`, the constrained xlarge Trade22 case
  using `--max-drawdown 0.004` averaged about `328.5 ms`.
- With `VECTORBT_KERNEL_EXACT_PRUNING=1`, the same constrained case averaged
  about `77.5 ms` and kept the same best `(fast_len=296, slow_len=297)`.
- Diagnostics reported
  `gpu_kernel.exact_pruning_pruned_candidates=370749` and
  `gpu_kernel.exact_pruning_skipped_bars=73907205292`.
- A medium constrained probe also activated
  `row_first_global_topk_exact_pruning`, pruning `14148` candidates and
  skipping `846412378` bars.

Nsight Compute evidence:

- Baseline report:
  `output/trade22_ncu_baseline_medium.ncu-rep` and
  `output/trade22_ncu_baseline_medium_raw.csv`.
- Pruning report:
  `output/trade22_ncu_pruning_medium.ncu-rep` and
  `output/trade22_ncu_pruning_medium_raw.csv`.
- Baseline Trade22 kernel symbol:
  `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0_nocomm`.
  The sampled kernel used `62` registers/thread.
- Pruning kernel symbol:
  `double_cross_backtest_tm_flex_f32_m22_prune_maxdd`. The sampled kernel used
  `72` registers/thread.
- Nsight replay times are not directly comparable to normal route wall time,
  but the reports confirm the intended symbols and show that the pruning path
  changes kernel behavior rather than host-only filtering.

Phase decisions from this evidence:

- Exact max-drawdown pruning is viable as an opt-in constrained-workload path.
  It should stay disabled by default until broader constraint coverage and perf
  gates justify changing that.
- Device compact union does not currently meet the viability threshold. Host
  union was below `0.1%` of the stressed route, so implementing a device compact
  union now would add ABI and fallback risk without a measured payoff.
- Device PSR/DSR confidence reranking does not currently meet the viability
  threshold. The host confidence cost was below `0.03 ms`, so it stays host-side
  and shortlist/finalist-focused.
- Signature deduplication is not worth a full implementation on the measured
  workload. A `1024`-candidate sample produced `1024` unique signatures
  (`unique_ratio=1.0`, `max_bucket_size=1`).

Remaining-phase measurements added later on 2026-04-26:

- Signature instrumentation now also reports duplicate candidate count,
  average evaluated bars, signal-change density, estimated duplicate bar walks
  avoided, and host hashing wall time. The current `1024`-candidate Trade22
  sample again produced `unique_ratio=1.0`, `duplicate_candidate_count=0`,
  `avg_signal_changes=4238.76`, `avg_bars_per_signal_change=28.27`,
  `estimated_duplicate_bar_walks_avoided=0`, and `hash_wall_ms=369.76`.
  This strengthens the stop decision for full signature dedup/scatter on the
  measured workload.
- Neighborhood robustness is now measurable from
  `profile_gpu_dynamic_runner` using explicit `--shortlist-neighborhood-radius`
  and `--shortlist-neighborhood-step` flags. On large Trade22, `120000`
  candles, `top_k=64`, `shortlist_slice_mode=halves`, `radius=1`, `step=1`
  averaged `82.67 ms` outer wall time versus `77 ms` for slice-only baseline.
  The route/runtime timing still excludes part of the post-optimization local
  stability recompute, so outer `wall_ms` is the right comparison.
- VDPS quality-check telemetry now reports
  `quality_check_holdout_regret_unavailable_reason="no_holdout_evaluation_context"`
  when same-dataset exact quality-check fields are populated but no holdout
  scorer exists. The numeric holdout-regret field remains empty by design until
  both exact-full-grid and VDPS holdout scores are available.

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

- [x] Build a repeatable advanced exact-kernel workload with:
  - `metric_contract=Trade22`
  - `top_k > 0`
  - `include_all=false`
  - `extra_shortlist_objectives` with two or three heads
  - each `shortlist_union_mode`
  - `shortlist_confidence_mode=psr` and `dsr`
- [x] Build at least one constrained large-grid workload where exact pruning
  could matter:
  - max drawdown limit
  - min/max exposure bounds
  - min trade count
  - large enough candidate count that rejected bar walks can move wall time
- [x] Preserve an unconstrained row-first baseline so pruning overhead is
  measured when little or nothing can be rejected.
- [x] Capture baseline timings with `VECTORBT_KERNEL_PROFILE_PIPELINE=1`.
- [x] Record:
  - total optimization time
  - `host_finalize_shortlist_union_ms`
  - `host_finalize_shortlist_confidence_ms`
  - `host_finalize_shortlist_slice_ms`
  - D2H copy time
  - host decode time
  - route diagnostics
  - top row identity and metrics
- [x] Add instrumentation fields only where needed:
  - early-pruned candidate count by reason
  - bars skipped by early pruning
  - signature unique ratio, average bucket size, and max bucket size
  - VDPS quality-check holdout regret once the value is implemented
- [x] Add missing telemetry only if current timing output cannot isolate the
  relevant cost.
- [x] Save the baseline command lines and results under `output/` if the run is
  used to justify implementation.

Measured outputs:

- `output/trade22_phase0_baseline.jsonl`
- `output/trade22_phase0_union_lexicographic.jsonl`
- `output/trade22_phase0_union_weighted_rank.jsonl`
- `output/trade22_phase0_confidence_psr.jsonl`
- `output/trade22_phase0_confidence_dsr.jsonl`
- `output/trade22_phase0_signature_stats.jsonl`
- `output/trade22_phase0_stress_union_weighted_rank.log`
- `output/trade22_phase0_pruning_off.log`
- `output/trade22_phase0_pruning_on.log`
- `output/trade22_pruning_medium_probe.log`
- `output/trade22_phase5_neighborhood_baseline_current.jsonl`
- `output/trade22_phase5_neighborhood_radius1_current.jsonl`
- `output/trade22_phase5_neighborhood_radius1_current.stderr.txt`
- `output/trade22_phase7_signature_stats_current.jsonl`

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

- [x] A clear baseline exists before kernel changes.
- [x] The measured host-side shortlist cost is large enough to justify CUDA
  work, or the effort stops here. For device compact union and device PSR/DSR
  reranking, the effort stops here because host-side costs were too small.

## Phase 0A: Exact Feasibility Pruning

Purpose: skip work only when a candidate can be rejected without changing the
exact result.

Initial implementation status: max-drawdown early pruning is implemented as an
opt-in row-first CUDA optimization behind `VECTORBT_KERNEL_EXACT_PRUNING=1`.
The default route is unchanged.

The current implementation is deliberately narrow:

- It only applies to the optimization row-first global top-k path.
- It requires device-side constraints, `include_all=false`, no export, no
  host-filter mode, no eval range, no precomputed fast/slow inputs, and
  `heatmap_bins=0`.
- It requires a finite `constraints.max_drawdown < 1.0`.
- It launches additive CUDA symbols with a `_prune_maxdd` suffix instead of
  changing the ABI of existing Trade22, Full17, or baseline kernels.
- It writes NaN metric rows for candidates rejected by monotone drawdown
  breach so the existing device selector constraints naturally exclude them.

This remains an experimental path until perf evidence proves it should move
closer to default. The expected win is workload-dependent: it helps most on
large constrained grids and may be neutral or slightly negative on
unconstrained small grids.

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

- [x] Keep pruning disabled by default behind an environment flag.
- [x] Start with monotone max-drawdown constraint pruning before kth-threshold
  ranking pruning.
- [x] Compare pruned and unpruned outputs on deterministic Trade22 fixtures.
- [x] Preserve existing Trade22 fixed-row kernels and add `_prune_maxdd`
  variants rather than replacing them.
- [x] Add route diagnostics for env state, symbol availability, active mode,
  blockers, and log-space drawdown threshold.
- [ ] Add prefix/suffix return-bound data only if the measured cost is small
  relative to candidate bar-walk savings.
- [x] Add per-reason counters and skipped-bar counters to diagnostics.
- [x] Include constrained and unconstrained perf cases so overhead is visible.

Acceptance:

- [x] Pruned and unpruned top-k outputs match for a deterministic exact Trade22
  fixture.
- [x] Invalid max-drawdown candidates rejected early match device constraint
  semantics.
- [x] Default/unconstrained workloads do not regress beyond perf budgets with
  pruning disabled by default. Verified with CPU and GPU perf gates on
  2026-04-26.
- [x] Constrained large-grid workloads meet the viability threshold before
  making the path default. The measured constrained xlarge route improved from
  about `328.5 ms` to about `77.5 ms`, but the path remains opt-in while
  broader constraint coverage and perf gates are accumulated.

## Phase 1: Preserve Current Device Multi-Head Extraction

Purpose: make sure future changes do not weaken what already works.

Implementation status: deterministic Trade22 parity coverage now verifies that
extra shortlist objective heads are selected on the row-first global top-k path
and that Rust union-mode reranking still matches an exhaustive include-all
oracle for lexicographic, Pareto, epsilon-front, and weighted-rank modes.

Tasks:

- [x] Add or tighten tests proving extra shortlist heads still select on the
  exact global top-k kernel path.
- [x] Assert that requests with `extra_shortlist_objectives` still reject
  incompatible host-filter, export, include-all, and active score-mode paths.
- [x] Preserve per-head global buffers in `ExtraShortlistHeadState`.
- [x] Preserve device-side constraints for the v2 selector path.

Acceptance:

- [x] Existing row-first global top-k tests still pass.
- [x] Extra-head output parity matches the current host-finalized result.
- [x] Route diagnostics still identify `row_first_global_topk` when expected.

## Phase 2: Device Score Heads For Row-Only Score Modes

Purpose: avoid `device_include_all` and host scoring for score modes that are
pure functions of completed metric rows.

Implementation status: an opt-in row-score head path is implemented behind
`VECTORBT_KERNEL_ROW_SCORE_HEADS=1`. It keeps the current include-all score-mode
route as the default, reuses the existing row-first global top-k selector ABI
with encoded objective ids, and preserves the Trade22 metric surface. The
initial implementation deliberately blocks include-all, export, extra shortlist
heads, union/confidence/slice modes, constraints, precomputed MA matrices,
eval ranges, heatmaps, and `top_k=0`; those requests fall back to the current
host scoring path unless the experimental route was already preselected.
The checked-in `double_crossover.prebuilt.ptx` has been regenerated, and
`build.rs` now requires both the new pruning kernel symbols and an exported
row-score ABI marker so stale prebuilt PTX fails fast.

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

- [x] Add a CUDA score-mode enum that mirrors `DoubleMaScoreMode` for row-only
  modes.
- [x] Add device helper functions in `double_crossover.cu` that match the Rust
  score functions in `double_ma.rs`.
- [x] Add a selector path that scores metric rows by score mode without forcing
  `device_include_all`.
- [x] Keep the new score-mode selector behind an opt-in flag until parity and
  performance are proven.
- [ ] Add direct CUDA tests comparing device scores to Rust score functions for
  representative metric rows if a callable device-score test harness is added.
- [x] Add end-to-end tests proving top-k identity matches the host score-mode
  path for each row-only score mode.

Acceptance:

- [x] Row-only score modes no longer require full host materialization on the
  experimental path.
- [x] Top-k order matches the host implementation for deterministic fixtures.
- [x] No default route changes until the GPU perf gate passes. Verified with
  the GPU perf gate on 2026-04-26 and prebuilt-PTX targeted CUDA tests.

## Phase 3: Device-Side Compact Union Composition

Purpose: reduce host-side union reranking and D2H work after multiple objective
heads have already been extracted from the same metric surface.

Implementation decision: deferred. The measured host-side union cost is too
small to justify a new CUDA compact-union ABI in this pass. On the stressed
xlarge Trade22 workload with `top_k=256` and three extra heads, host union was
about `0.134-0.207 ms` while route time was about `330 ms`. That misses both
the 5% wall-time threshold and the 25% meaningful host-finalize threshold.

Keep the design below as the next opt-in path only if a future workload shows
host union or compact-row D2H as a real bottleneck.

Preferred implementation order:

1. Device dedup of compact per-head top-k rows.
2. Device lexicographic union ranking.
3. Device weighted-rank union ranking.
4. Device Pareto and epsilon-front ranking only if Phase 0 shows enough host
   cost to justify the more complex O(k^2) logic.

Future tasks if new measurements justify reopening this phase:

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

Future acceptance:

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

Implementation decision: keep PSR/DSR confidence reranking host-side for now.
The measured host cost was about `0.011-0.024 ms`, below the threshold where a
device implementation would be worth the ABI, fallback, and parity risk.
Shortlist/finalist validation remains the right placement for HAC, bootstrap,
walk-forward, CSCV/PBO, SPA-style diagnostics, and multiple-testing reporting.

Future tasks if confidence becomes a measured bottleneck:

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

Future acceptance:

- [ ] PSR/DSR order matches host for deterministic fixtures.
- [ ] Numeric differences are bounded and cannot flip documented tie-breakers.
- [ ] The path improves measured advanced shortlist workload time.
- [ ] New robust validation fields do not affect default optimization ranking
  unless explicitly requested.

## Phase 5: Neighborhood Robustness From Existing Rows

Purpose: investigate whether neighborhood robustness can reuse already evaluated
rows in cases where the required local surface is available.

This is lower priority than score heads and union composition.

Implementation decision: no production optimizer change in this pass. The exact
grid is not retained on the normal row-first path, and adding full-grid
residency for neighborhood diagnostics would be a memory and D2H risk without
a current measurement showing local-stability recompute as a bottleneck.

Measurement support: `profile_gpu_dynamic_runner` now accepts
`--shortlist-neighborhood-radius` and `--shortlist-neighborhood-step`, emits
the requested radius/step in JSON, and includes the resulting
`shortlist_neighborhood_robustness` summary. The flags require
`--shortlist-slice-mode`, matching the current backend contract, and `step`
requires `radius`.

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

Future tasks:

- [x] Measure current local-stability recompute cost before changing it.
- [ ] Do not add full-grid retention just for neighborhood diagnostics unless
  memory and timing evidence supports it.
- [x] Keep runner measurement bounded by radius and step and preserve the
  existing local-stability summary as the oracle.
- [ ] If a production device reuse path is implemented, keep the feature
  bounded by radius and step and preserve
  the existing local-stability summary as the oracle.

Future acceptance:

- [ ] No additional full-grid memory residency is introduced on default routes.
- [x] The measurement path emits the existing local-stability summary without a
  new diagnostic schema.
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

Implementation decision: blocked until a real holdout input contract exists.
The result shape has a holdout-regret slot, but populating it without both
exact-full-grid and VDPS holdout scores would be misleading. VDPS selection
behavior was not changed in this pass.

Telemetry change: when the same-dataset quality check runs, the telemetry now
sets `quality_check_holdout_regret_unavailable_reason` to
`no_holdout_evaluation_context` and leaves `quality_check_holdout_regret=None`.
Merged multi-mode VDPS telemetry clears the reason along with the other
single-run quality-check fields because a merged result no longer has one
unambiguous quality-check context.

Future tasks:

- [ ] Fill `quality_check_holdout_regret` when a holdout objective is available:

```text
quality_check_holdout_regret =
  exact_holdout_score(best_exact_full_grid) - exact_holdout_score(best_vdps)
```

- [ ] Add tests proving the field is populated only when both holdout scores
  are available.
- [x] Add a test proving current quality-check telemetry marks holdout regret
  unavailable rather than inventing a numeric regret without holdout scores.
- [ ] Add optional uncertainty-aware VDPS promotion:

```text
promotion_score = estimated_score - lambda * uncertainty
exploration_score = estimated_score + beta * uncertainty
```

- [ ] Promote high-LCB candidates, inject high-UCB candidates, and mutate around
  stable local basins.
- [x] Keep VDPS winners exact-evaluated before reporting.
- [ ] Treat uncertainty-aware promotion as new behavior. Current VDPS refill
  supports deterministic seeds plus random, elite mutation, and differential
  injection; it does not already rank promotions by LCB/UCB.

Future acceptance:

- [x] VDPS quality-check telemetry reports why holdout regret is unavailable
  when no holdout scoring context exists.
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

Implementation status: telemetry-only instrumentation has been added to
`profile_gpu_dynamic_runner` via `--signature-dedup-stats on` and
`--signature-sample-limit`. It hashes sampled plain crossover sign streams with
`start_t` and `eval_end` included in the signature, records signal-change
density, estimates duplicate bar walks avoided, and measures host hashing time.
It is deliberately not part of the production optimizer path and it never
skips candidate walks.

Measured decision: the first Trade22 sample produced `1024` unique signatures
for `1024` sampled candidates (`unique_ratio=1.0`, `max_bucket_size=1`), so a
full dedup/scatter implementation is stopped for now. The follow-up sample
kept the same unique ratio and reported zero estimated duplicate bar walks
avoided; the host hashing probe took `369.76 ms` for `1024` candidates, which
is far larger than the measured GPU route wall time for the same optimization.

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
- average evaluated bars
- average signal changes and bars per signal change
- time spent hashing/signature packing
- estimated bar walks avoided if full dedup existed

Acceptance:

- [x] If the unique ratio is near `1.0` on real workloads, stop the effort.
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

Implementation decision: no production trust-region allocator was added in this
pass. The existing local surrogate admission remains diagnostic. A future
allocator should use surrogate output only to choose additional exact
evaluations, never to report approximate winners.

Future tasks:

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

Future acceptance:

- [x] Final reported winners are exact.
- [ ] Surrogate admissions and rejections are visible in diagnostics.
- [ ] The path improves evaluation allocation on expensive surfaces without
  changing exact result semantics.

## Phase 9: Event/Run-Length Backtesting Prototype

Purpose: test candidate path compression only on narrow surfaces where exactness
is provable.

Implementation decision: prototype-only and not implemented in this pass. The
measured signature sample did not show path duplication, the added signal-change
telemetry shows the sampled choppy workload has dense event streams, and the
current row-first Trade22 route plus exact pruning already produced a large
constrained workload win with lower integration risk.

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

Implementation decision: research-only in this pass. No block-transducer or
conditional metric contract was added, and Trade22 remains the canonical rich
exact contract.

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

## Active Development Status: 2026-04-26

There are no remaining active implementation phases in this guide after the
current Trade22 pass. The unchecked items above are intentionally future-gated,
not incomplete work in the current CUDA lane:

- Prefix/suffix return-bound pruning remains gated because the proven win came
  from monotone max-drawdown rejection, while return-bound pruning would need a
  separate objective-specific proof and cost measurement.
- Direct CUDA score-helper tests remain gated on adding a callable device-score
  harness. Current parity is covered end-to-end against the host score-mode
  route, which is the product behavior that can regress users.
- Device compact union and device PSR/DSR confidence remain deferred because
  measured host costs were below the viability threshold.
- Full-grid neighborhood retention remains deferred because the current
  optimizer does not retain the exact grid and the measured bounded recompute
  cost does not justify changing memory residency.
- VDPS numeric holdout regret and LCB/UCB promotion remain gated on a real
  holdout scoring contract and opt-in adaptive-search telemetry.
- Signature dedup, event/run-length backtesting, block transducers, and
  conditional metric contracts remain research/prototype work until a workload
  proves enough duplication, sparsity, or robustness value to offset complexity.

The next implementation should reopen one of those items only with fresh
evidence that violates the stop condition recorded in its phase.

## Verification Results: 2026-04-26

Commands completed after the latest guide and profiling-runner update:

```powershell
cargo fmt --check
cargo check -p ta_app_core --features cuda,cuda-backtest-kernel --bin profile_gpu_dynamic_runner
cargo check -p ta_app_core --bin profile_gpu_dynamic_runner
cargo build -p ta_app_core --features cuda,cuda-backtest-kernel --release --bin profile_gpu_dynamic_runner
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_vdps_quality_check_emits_overlap_regret_and_winner_rank -- --nocapture
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel exact_pruning -- --nocapture
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel row_score -- --nocapture
cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_trade22_shortlist_union_modes_match_exhaustive_host_union_modes -- --nocapture
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

Targeted runner probes:

```powershell
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --candles-len 120000 --metric-contract trade22 --include-trade-metrics on --heatmap-bins 0 --top-k 64 --warmup-iterations 0 --iterations 1 --emit-json --candle-shape choppy --signature-dedup-stats on --signature-sample-limit 1024
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --candles-len 120000 --metric-contract trade22 --include-trade-metrics on --heatmap-bins 0 --top-k 64 --warmup-iterations 0 --iterations 3 --emit-json --candle-shape choppy --shortlist-slice-mode halves
target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --workload large --candles-len 120000 --metric-contract trade22 --include-trade-metrics on --heatmap-bins 0 --top-k 64 --warmup-iterations 0 --iterations 3 --emit-json --candle-shape choppy --shortlist-slice-mode halves --shortlist-neighborhood-radius 1 --shortlist-neighborhood-step 1
```

GPU perf gate results:

- `gpu_criterion_case_gpu_sma_small_not_regressed_10pct`: pass,
  `67.869193 ms` versus `70.640588 ms` baseline.
- `gpu_criterion_case_gpu_alma_large_not_regressed_10pct`: pass,
  `69.572497 ms` versus `85.373831 ms` baseline.
- `gpu_dynamic_strategy_ratio_not_regressed_20pct`: pass,
  `0.980695` versus `0.966667` baseline.
- `gpu_dynamic_strategy_static_off_latency_not_regressed_20pct`: pass,
  `64.75 ms` versus `67.5 ms` baseline.

Runner probe results:

- Signature dedup instrumentation:
  `unique_ratio=1.0`, `duplicate_candidate_count=0`,
  `estimated_duplicate_bar_walks_avoided=0`, `hash_wall_ms=369.76`.
- Slice-only neighborhood baseline:
  average `wall_ms=77`, `runtime_ms=77`, `route_wall_ms=77`,
  `host_finalize_shortlist_slice_ms=18.67`.
- Slice plus `radius=1`, `step=1` neighborhood:
  average `wall_ms=82.67`, `runtime_ms=75`, `route_wall_ms=75`,
  `host_finalize_shortlist_slice_ms=17.33`, with neighborhood summaries present
  on all three measured iterations.

## Completion Checklist

- [x] Phase 0 baseline proves exact pruning is worth continuing and device
  union/confidence should stop for now.
- [x] Exact pruning either ships with parity and constrained-workload wins or is
  left disabled with diagnostic evidence.
- [x] Device score heads have end-to-end parity against host score-mode
  selection on deterministic fixtures.
- [x] Device compact union is explicitly deferred based on measurements.
- [x] Confidence reranking is either implemented with parity or explicitly
  deferred based on measurements.
- [x] VDPS holdout regret is documented as blocked until holdout inputs exist.
- [x] Signature dedup has instrumentation results before any full dedup path is
  designed.
- [x] Event/run-length and block-transducer ideas remain prototype-only unless
  exactness and timing evidence are both strong.
- [x] Neighborhood robustness is either improved with bounded evidence or
  explicitly deferred.
- [x] Default GPU perf gates pass after the latest doc/runner update.
- [x] CPU-only safety remains intact after the latest doc/runner update.
- [x] Documentation and capability copy match the final shipped behavior.
