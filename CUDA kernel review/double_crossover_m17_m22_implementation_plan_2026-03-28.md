# Double Crossover M17 / M22 Implementation Plan

Date: 2026-03-28

Scope: current-state implementation plan for the still-valid `double_ma` CUDA work that directly affects the 17-metric-row and 22-metric-row paths in `double_crossover.cu`.

This plan is derived from the existing review in `CUDA kernel review/double_crossover_performance_review_final.md`, but only keeps the parts that are still supported by the current codebase.

Primary target workloads:

- full 17-row metric contract (`METRICS_COUNT = 17`)
- full 22-row trade-metric contract (`TRADE_METRICS_COUNT = 22`)
- exact / shortlist-compatible GPU kernel runs that require full-row materialization
- `include_trade_metrics=true` GPU kernel runs

This plan does not include:

- cubin-resource or occupancy claims that require fresh disassembly/profiling to prove
- `trade_on_next_bar=false` specialization work
  - the app already rejects same-bar execution in strategy validation
- compact valid-pair map work
  - compact valid-pair output is already enabled by default and the pair map preserves dense / `pf`-major order
- commission-range validation work
  - host-side strategy validation already rejects invalid effective commission

## Why This Exists

The current GPU kernel stack has already moved beyond the state assumed by some parts of the review:

- reduced 10-row product contracts exist and are used intentionally for parts of the optimizer
- compact valid-pair output is already the default
- host-side max-drawdown threshold conversion already accounts for reduced-row log-space search rows

But the core M17 / M22 performance gap is still real:

- there are still no true fixed-contract M17 wrappers
- there are still no true fixed-contract M22 wrappers
- the host dispatch matrix still lacks a combined `lockstep + fast_warp_dedup` route
- the lockstep full-row kernels still reload warp-uniform series values lane-by-lane
- the pipeline still recomputes objective / constraint work from metric rows after backtest

## Current Repo Baseline

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_strategy.rs`

Current kernel contracts:

- `BASE_METRICS_COUNT = 7`
- `PRODUCT_REDUCED_METRICS_COUNT = 10`
- `METRICS_COUNT = 17`
- `TRADE_METRICS_COUNT = 22`

Current dispatch families:

- baseline generic backtest
- generic `windowed`
- generic `fast_warp_dedup`
- generic `lockstep_time`
- lockstep `m7` specializations
- lockstep `m10 regime_off norisk` specializations

Current gaps relevant to 17 / 22:

- no fixed M17 specialization family
- no fixed M22 specialization family
- no generic `lockstep_time_fast_warp_dedup` kernel
- no dedicated 17 / 22 windowed specialization family
- no direct `score_out` / `feasible_out` outputs from the backtest finalizer

Current correctness debt that still matters:

- calmar decode still uses `M < 15` while the writer uses `M < 17`
- `regime_range_exit_rel` is still threaded through the ABI but unused in the regime helpers
- the raw kernel contract still assumes slot 6 net exposure semantics match the signed-exposure flag, even though the current host path always enables `STRAT_SIGNED_EXPOSURE`

Current host-side safeguards already present:

- `trade_on_next_bar=false` is rejected by `validate_strategy_cfg`
- invalid effective commission is rejected by `validate_strategy_cfg`
- reduced-row max-drawdown thresholds are converted into search-space form before device launch
- compact valid-pair output is always enabled

## Current Contract Notes

`trade_on_next_bar`:

- the current app contract requires `trade_on_next_bar=true`
- the kernel still carries `trade_on_next_bar` as a runtime flag today
- if this plan ever promotes that flag to a compile-time contract, it should hard-wire the `true` case
- this plan should not create a `TRADE_ON_NEXT_BAR=false` specialization family

`STRAT_SIGNED_EXPOSURE`:

- the raw kernel API still makes slot 6 depend on the signed-exposure flag
- the current GPU host path always enables `STRAT_SIGNED_EXPOSURE`
- the plan should treat this as a host invariant that either remains explicit and tested, or gets hardened before future flag-matrix expansion

`metrics_out_stride`:

- the raw kernel ABI still allows a caller to pass a stride that differs from `M`
- the current host path already passes a safe effective stride
- the plan only needs lightweight hardening here, not a large redesign

## Per-File Edit Map

`crates/app_core/src/kernels/double_crossover.cu`:

- add new M17 / M22 wrapper symbols
- add any fixed-`M` body refactor if wrapper-only constant propagation is not sufficient
- add the generic combined `lockstep_time + fast_warp_dedup` route
- add warp-uniform helpers for lockstep full-row loads
- centralize search-row log-space decode behavior
- resolve or remove the active `regime_range_exit_rel` contract

`crates/app_core/src/gpu_backtest_kernel.rs`:

- extend both symbol-loading blocks
- extend both dispatch blocks
- keep reduced-row M7 / M10 route precedence intact
- add or update dispatch logging so M17 / M22 specializations are visible in logs
- add tests that lock dispatch choice and current host invariants

`crates/app_core/src/double_ma.rs`:

- keep exact-route expectations aligned with the new host dispatch
- expand GPU exact-route tests where the optimizer result shape depends on the full-row contract

`crates/app_core/src/double_ma_strategy.rs`:

- keep `trade_on_next_bar=true` and effective-commission validation explicit
- add comments or tests only if the new tracking work needs the contract to be more visible

## Goals

1. Improve wall-clock performance for full 17-row exact GPU kernel runs.
2. Improve wall-clock performance for 22-row trade-metric GPU kernel runs.
3. Preserve CPU fallback and current reduced-row behavior.
4. Avoid widening the correctness surface while optimizing the hot path.
5. Keep routing and specialization logic explicit enough that future kernel work stays maintainable.

## Non-Goals

- redesigning reduced-row 10-metric product policy
- broad CUDA runtime refactors unrelated to `double_ma`
- replacing the current exact / shortlist pipeline with a different optimizer architecture
- speculative tuning that is not first grounded in current dispatch and correctness gaps

## Success Criteria

This plan is successful when:

- common 17-row and 22-row exact GPU runs route to dedicated kernels instead of only generic wrappers
- the dispatch matrix covers the important non-windowed full-row combinations used by the current app
- the kernel no longer carries known latent correctness mismatches in the full-row path
- lockstep full-row kernels stop paying avoidable warp-uniform load overhead
- profiling can clearly show whether the backtest stage or the downstream rerank stage remains dominant

## Status: 2026-03-29

Completed so far:

- Phase 1 calmar/log-space cleanup is in place.
  - device-side search-row log-space behavior is now centralized under the `M < 17` rule
  - the metric writer and calmar scoring now use the same device helper
- True fixed-contract base wrappers for `m17`, `m22`, `lockstep_time_m17`, and `lockstep_time_m22` are in place.
- First-wave lockstep `regime_off + norisk` families are now implemented for both full-row contracts:
  - `lockstep_time_m17_regime_off_norisk`
  - `lockstep_time_m17_regime_off_norisk_nocomm`
  - `lockstep_time_m17_regime_off_norisk_eps0`
  - `lockstep_time_m17_regime_off_norisk_eps0_nocomm`
  - same family for `m22`
- First-wave non-lockstep `regime_off + norisk` families are now implemented for both full-row contracts:
  - `m17_regime_off_norisk`
  - `m17_regime_off_norisk_nocomm`
  - `m17_regime_off_norisk_eps0`
  - `m17_regime_off_norisk_eps0_nocomm`
  - same family for `m22`
- Second-wave non-lockstep regime-bearing families are now implemented for both full-row contracts:
  - `m17_regime_on_norisk`
  - `m17_regime_on_norisk_eps0_nocomm`
  - `m17_regime_hybrid_norisk`
  - `m17_regime_hybrid_norisk_eps0_nocomm`
  - same family for `m22`
- Host dispatch now reaches the dedicated M17 / M22 lockstep families instead of falling back to generic full-row lockstep when the request is:
  - fixed-contract 17-row or 22-row
  - non-windowed
  - `regime_off`
  - `no_risk`
  - optionally `eps0` and/or zero commission
- Host dispatch now also reaches the dedicated non-lockstep M17 / M22 families for the same `regime_off + no_risk (+ eps0/+ nocomm)` slice when the workload does not meet the lockstep heuristic.
- Host dispatch now also reaches the dedicated non-lockstep M17 / M22 families for the `regime_on + no_risk` and `regime_hybrid + no_risk` slices when the workload does not meet the lockstep heuristic.

Measured smoke results:

- earlier baseline `lockstep_time_m17`: about `342.3 ms` dominant GPU kernel time
- current `lockstep_time_m17_regime_off_norisk_eps0_nocomm`: about `295.0 ms`
- earlier baseline `lockstep_time_m22`: about `368.0 ms` dominant GPU kernel time
- current `lockstep_time_m22_regime_off_norisk_eps0_nocomm`: about `323.6 ms`
- small-workload non-lockstep smoke now lands on:
  - `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0_nocomm`
  - `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0_nocomm`
- current `large/58300pairs` fixed-contract runner timings:
  - `full17`: `77 / 79 / 86 ms` min / median / max, `81.0 ms` mean runtime across 5 measured iterations after 1 warmup
  - `trade22`: `76 / 76 / 77 ms` min / median / max, `76.4 ms` mean runtime across 5 measured iterations after 1 warmup
- latest `alma/alma large/58300pairs` fixed-contract runner timings:
  - `full17`: `75 / 79 / 81 ms` min / median / max, `78.0 ms` mean runtime across 5 measured iterations after 1 warmup
  - `trade22`: `73 / 75 / 80 ms` min / median / max, `76.4 ms` mean runtime across 5 measured iterations after 1 warmup
- combined generic `lockstep_time + fast_warp_dedup` was prototyped and measured, then reverted:
  - `large` PnL generic lockstep case regressed from about `150.6 ms` mean to about `221.6 ms`
  - `xlarge` PnL generic lockstep case regressed from about `404.4 ms` mean to about `615.8 ms`
  - conclusion: do not treat the generic combined route as the next win without a more targeted design
- targeted lockstep `regime_on` full-row specializations are now implemented for the benchmark-hot `no_risk` slice, including the default `eps0 + zero_commission` specialization:
  - verified active via Nsight on `trend_quality` xlarge ALMA runs:
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk_eps0_nocomm`: about `420.0 ms`
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_on_norisk_eps0_nocomm`: about `444.4 ms`
- targeted lockstep `regime_hybrid` full-row specializations are compiled and routed in the exact-kernel path, but the current `profile_gpu_dynamic_runner` cannot benchmark them with fixed metric contracts because the `regime_hybrid` strategy does not currently stay on the exact `GpuKernel` backend route.
- targeted non-lockstep `regime_on` full-row routing is now verified active on the small exact-route benchmark surface:
  - `trend_quality` + `alma/alma` + `small/1590pairs` + `full17` logs `dispatch=m17`
  - `trend_quality` + `alma/alma` + `small/1590pairs` + `trade22` logs `dispatch=m22`
  - with `trend_quality_strategy_for_kernel()` enabling the regime filter and `StrategyConfig::default()` keeping `stop_loss_pct`, `take_profit_pct`, `commission`, and `eps_rel` at zero or `None`, those dispatches land on the `regime_on_norisk_eps0_nocomm` selectors for the non-lockstep M17 / M22 families
- optimize-path full-row selection is now centralized behind pure fixed-contract dispatch / flavor helpers:
  - route selection with symbols present is unit-tested
  - fallback when the matching M17 / M22 symbol is unavailable is unit-tested
  - the host-side `STRAT_SIGNED_EXPOSURE` invariant is now explicit in both code comments and unit tests
- the first `__shfl_sync`-based warp-uniform lockstep load attempt for `lr[t]` and `ret1[t]` was measured and reverted in the same session:
  - workload: `trend_quality` + `alma/alma` + `xlarge/370750pairs` + `objective=sharpe`
  - with the helper enabled:
    - `full17`: `496 / 489 / 500 ms` min / median / max, `495.0 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `505 / 507 / 509 ms` min / median / max, `507.0 ms` mean across 3 measured iterations after 1 warmup
  - after reverting the helper:
    - `full17`: `423 / 427 / 433 ms` min / median / max, `427.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `436 / 437 / 438 ms` min / median / max, `437.0 ms` mean across 3 measured iterations after 1 warmup
  - conclusion: do not treat the current broadcast-helper form as the Phase 4 answer for warp-uniform lockstep loads
- forced generic `fast_warp_dedup` is now benchmarked as a proxy for possible M17 / M22 dedicated dedup variants on the same hot xlarge regime-on slice:
  - workload: `trend_quality` + `alma/alma` + `xlarge/370750pairs` + `objective=sharpe`
  - current specialized lockstep routes:
    - `dispatch=lockstep_m17`: `412 / 412 / 431 ms` min / median / max, `418.3 ms` mean across 3 measured iterations after 1 warmup
    - `dispatch=lockstep_m22`: `437 / 449 / 455 ms` min / median / max, `447.0 ms` mean across 3 measured iterations after 1 warmup
  - forced generic dedup-only route:
    - `dispatch=fast_warp_dedup` + `full17`: `945 / 963 / 968 ms` min / median / max, `958.7 ms` mean across 3 measured iterations after 1 warmup
    - `dispatch=fast_warp_dedup` + `trade22`: `958 / 962 / 974 ms` min / median / max, `964.7 ms` mean across 3 measured iterations after 1 warmup
  - conclusion: do not add dedicated M17 / M22 dedup variants based on the current dedup design; the existing dedup route is more than 2x slower than the specialized lockstep full-row routes on this benchmark-hot surface
- reduced-row and full-row max-drawdown constraint packing is now covered directly in unit tests:
  - `build_device_constraint_args()` converts reduced rows into log-space
  - `build_device_constraint_args()` keeps raw `max_drawdown` thresholds for both `full17` and `trade22`
- compact valid-pair ordering is now covered directly at the builder level:
  - `build_valid_pair_index_map_i32()` is unit-tested to preserve dense `pf`-major order while skipping invalid `fast >= slow` pairs
- `regime_hybrid_reversion_exit_rel` is now implemented as a real hysteresis threshold instead of dead contract surface:
  - CPU range-mode helpers now keep the current side until the spread crosses the exit band or an opposite entry threshold
  - CUDA range-mode helpers now use the same current-position-aware rule
  - covered by the `desired_sign_with_filters_*` unit tests and the existing GPU hybrid parity test
- Nsight Compute is now usable on this machine when launched with workspace-local `HOME`, `TEMP`, and `TMP`:
  - reports captured at `target/nsight/m17_regime_on_xlarge_profile.ncu-rep` and `target/nsight/m22_regime_on_xlarge_profile.ncu-rep`
  - xlarge `trend_quality` + `alma/alma` lockstep hot-kernel summary:
    - `lockstep_time_m17_regime_on_norisk_eps0_nocomm`: `474.24 ms`, compute throughput `61.62%`, memory throughput `25.05%`, `56` registers/thread, theoretical occupancy `75.0%`, achieved occupancy `59.81%`
    - `lockstep_time_m22_regime_on_norisk_eps0_nocomm`: `484.96 ms`, compute throughput `59.76%`, memory throughput `20.97%`, `64` registers/thread, theoretical occupancy `66.67%`, achieved occupancy `53.42%`
- a selective `__launch_bounds__(128, 2)` experiment on just the two benchmark-hot lockstep regime-on kernels was measured and reverted:
  - workload: xlarge `trend_quality` + `alma/alma` + objective `sharpe`
  - baseline before the experiment:
    - `full17`: `423 / 427 / 433 ms` min / median / max, `427.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `436 / 437 / 438 ms` min / median / max, `437.0 ms` mean across 3 measured iterations after 1 warmup
  - with `__launch_bounds__(128, 2)` on `lockstep_time_m17_regime_on_norisk_eps0_nocomm` and `lockstep_time_m22_regime_on_norisk_eps0_nocomm`:
    - `full17`: `422 / 426 / 429 ms` min / median / max, `425.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `443 / 447 / 450 ms` min / median / max, `446.7 ms` mean across 3 measured iterations after 1 warmup
  - conclusion: reject the experiment; the small 17-row gain does not justify the larger 22-row regression
- Nsight Systems capture for this new non-lockstep regime-on slice is still pending in the recovered post-crash session:
  - older lockstep M17 / M22 captures remain valid
  - current `nsys` attach attempts time out before session creation even after repairing the temp directory

Current interpretation:

- the first full-row win is real and the specialized kernels are active
- the `large` fixed-contract slice is already materially faster than the generic full-row path and `trade22` currently runs slightly tighter than `full17`
- the `alma/alma` large fixed-contract slice is now running below the old mid-80 ms expectation in direct exact-route runner measurements
- the generic combined lockstep/dedup idea is now measured as a regression, not an open presumed win
- the existing generic dedup path is also a regression proxy for M17 / M22 full-row work, so dedicated dedup variants are now lower priority than lockstep load cleanup
- the first Nsight Compute pass does not point to a DRAM-bound kernel:
  - both hot lockstep regime-on kernels are primarily compute / latency limited
  - both are register-limited on theoretical occupancy
  - that makes warp-uniform regime-value broadcast cleanup a lower-confidence next step than it looked before profiling
- the first bounded register-pressure experiment also failed to produce a stable cross-contract win
  - future occupancy tuning should stay narrowly targeted and be treated as opt-in experimentation, not presumed progress
- windowed exact full-row hotness is still unmeasured, but it is now clearer why:
  - `profile_gpu_dynamic_runner` currently constructs only a plain grid `DoubleMaRequest`
  - `DoubleMaRequest` does not currently expose an explicit windowed optimization surface for this benchmark harness to drive
- remaining open work is now centered on regime-hybrid verification once there is an exact-kernel benchmark surface for it, any bounded register-pressure / occupancy experiments that can justify themselves against the new Nsight Compute data, and any windowed full-row families that profiling can justify

## Implementation Order

1. Lock correctness and measurement for current 17 / 22 behavior.
2. Add true fixed-contract M17 / M22 kernels for the highest-frequency contracts.
3. Expand host dispatch so those kernels are actually reachable.
4. Close the highest-value remaining lockstep overhead without reviving the rejected generic combined lockstep / dedup route.
5. Re-profile and decide whether direct `score_out` / `feasible_out` is the next bottleneck win.

## Phase 1: Baseline And Correctness Lock

Goal: make full-row optimization work measurable and safe before adding more specialized kernels.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`

Deliverables:

- centralize the search-row log-space rule in device code so writer and scorer use the same rule
  - target rule: rows use log-space when `M < 17`
- remove the current calmar decode mismatch
- document and resolve the `regime_range_exit_rel` contract
  - either implement it properly in the regime helper flow or remove it from the active contract
- keep the current host-side max-drawdown reduced-row conversion behavior covered by tests
- keep the current host-side `STRAT_SIGNED_EXPOSURE` expectation explicit
  - either preserve the invariant and test it, or harden the raw kernel contract before future flag specialization

Acceptance:

- 17-row and 22-row decode semantics are consistent across writer, scorer, and host decode
- exact GPU tests covering calmar and max-drawdown constraints pass
- there is no dead-but-user-visible `regime_range_exit_rel` contract left undocumented

## Phase 2: True Fixed-Contract M17 / M22 Kernels

Goal: extend the current M10 specialization strategy to the full-row contracts that matter.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Deliverables:

- add dedicated fixed-contract M17 wrappers
- add dedicated fixed-contract M22 wrappers
- start with the high-value non-windowed contracts:
  - non-lockstep, regime off, no risk
  - lockstep, regime off, no risk
  - `eps0`
  - `nocomm`
  - `eps0 + nocomm`
- keep the initial specialization matrix bounded
  - do not explode immediately into every possible flag combination
- prefer a refactor path that allows `FIXED_M` to become a template contract if wrapper-only constant propagation is not sufficient

Recommended first wave:

- `m17_regime_off_norisk`
- `m17_regime_off_norisk_eps0`
- `m17_regime_off_norisk_nocomm`
- `m17_regime_off_norisk_eps0_nocomm`
- `lockstep_time_m17_regime_off_norisk`
- `lockstep_time_m17_regime_off_norisk_eps0`
- `lockstep_time_m17_regime_off_norisk_nocomm`
- `lockstep_time_m17_regime_off_norisk_eps0_nocomm`
- same family for `m22`

### Proposed Symbol Matrix

Phase-1 / required symbols:

- `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_nocomm`
- `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0`
- `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0_nocomm`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_nocomm`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
- `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_nocomm`
- `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0`
- `double_cross_backtest_tm_flex_f32_m22_regime_off_norisk_eps0_nocomm`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_nocomm`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`

Phase-2 / optional later symbols:

- `double_cross_backtest_tm_flex_f32_lockstep_time_fast_warp_dedup`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_fast_warp_dedup`
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_fast_warp_dedup`

Reserved only if profiling proves a need:

- `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk`
- `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk`
- matching `eps0` / `nocomm` / `eps0_nocomm` windowed variants

Acceptance:

- host code can load the new symbols successfully
- full-row exact requests can route to M17 / M22 specializations
- no regression in reduced-row M7 / M10 dispatch

## Phase 3: Host Dispatch Expansion

Goal: make the new full-row kernels first-class in the runtime rather than dead code.

Primary files:

- `crates/app_core/src/gpu_backtest_kernel.rs`

Deliverables:

- add host symbol loading for the new M17 / M22 kernels
- extend dispatch selection so 17-row and 22-row exact paths choose the new kernels when the runtime contract matches
- keep the dispatch rules explicit and product-oriented:
  - metrics count
  - lockstep eligibility
  - risk enabled vs no-risk
  - regime mode
  - `eps0`
  - zero vs non-zero commission
- keep current M10 product-route preference intact for reduced-row product runs

Acceptance:

- dispatch logs clearly show when the runtime chooses M17 / M22 specializations
- exact 17 / 22 runs do not silently fall back to baseline when a matching specialized kernel exists

### Minimum Dispatch Decision Variables

The full-row route chooser should only need these inputs:

- `metrics_count`
- `use_windowed_eval`
- `use_lockstep_time`
- `use_fast_warp_dedup`
- `use_regime_i`
- `use_regime_hybrid_i`
- `no_risk_mode`
- `eps_zero_mode`
- `zero_commission_mode`
- `launch_pairs`
- symbol availability for the chosen family

### Dispatch Precedence For Full-Row Routes

Recommended precedence for `metrics_count == 17 || metrics_count == 22`:

1. keep existing reduced-row M7 / M10 branches untouched when `metrics_count` is not 17 / 22
2. if `use_windowed_eval`, stay on the current generic windowed route unless dedicated windowed full-row kernels have been added
3. if full-row specialized lockstep kernels exist and the contract matches, choose them before generic lockstep
4. if full-row specialized non-lockstep kernels exist and the contract matches, choose them before generic baseline
5. if no specialization matches but combined generic `lockstep + dedup` exists and both gates are true, choose it
6. otherwise fall back to the existing generic `lockstep_time`, `fast_warp_dedup`, or baseline route

Fallback rule:

- never silently change a 17 / 22 request into a reduced-row M7 / M10 contract just to hit a specialization

## Phase 4: Route-Matrix And Lockstep Overhead Follow-Through

Goal: close the most important route and hot-loop gaps that still affect the full-row path.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Deliverables:

- add a combined generic `lockstep_time + fast_warp_dedup` route
- benchmark whether M17 / M22 specializations should also have a combined lockstep / dedup form
  - do not assume the M10 choice generalizes automatically
- add warp-uniform load helpers for lockstep full-row kernels:
  - `lr[t]`
  - `ret1[t]`
  - regime value when regime filters are active
- keep compact valid-pair ordering invariant explicit in tests / comments
  - dense / `pf`-major ordering is already present and should be preserved

Windowed policy:

- do not implement a full dedicated windowed M17 / M22 family by default
- add it only if profiling shows that exact full-row workloads spend meaningful time in `windowed` dispatch

Acceptance:

- non-windowed full-row dispatch no longer has a hard split between lockstep and dedup-only routes
- lockstep kernels no longer issue lane-by-lane loads for warp-uniform series values

## Phase 5: Optional Pipeline Fusion

Goal: reduce post-backtest reread / rerank overhead if backtest specialization makes downstream work more visible.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Deliverables:

- prototype optional `score_out`
- prototype optional `feasible_out`
- wire the outputs only for the exact shortlist / rerank flows that actually benefit
- keep full-row metrics output intact
  - this is additive, not a contract replacement

Decision gate:

- only do this after profiling shows that row reread / objective recompute is a material share of exact 17 / 22 runtime

Acceptance:

- fused scoring path can be benchmarked against the existing metric-row reread path
- the optimization result contract remains unchanged for callers

## Specialization Policy

These guide ideas are still valid enough to keep in scope:

- specialize around `M`
- specialize around lockstep vs non-lockstep
- specialize around regime-off and no-risk first
- specialize around `eps0` and zero-commission when those are common

These guide ideas should not be carried forward as written:

- `TRADE_ON_NEXT_BAR=false` specialization
  - invalid for the current app contract
- compact-pair-map conversion as a new project
  - already implemented
- host commission-range validation as a new task
  - already implemented

These ideas are conditional rather than default:

- compile-time specialization for `long_only` / `no_flip`
  - only if profiling shows the extra matrix complexity is justified
- dedicated windowed M17 / M22 kernels
  - only if windowed exact flows are materially hot

## Verification Plan

Behavioral verification:

- `cargo test -p ta_app_core`
- targeted GPU-kernel tests in:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/double_ma.rs`
- add or expand tests for:
  - 17-row calmar decode
  - 17-row max-drawdown constraint behavior
  - 22-row trade-metric decode
  - dispatch selection for M17 / M22 specializations
  - route fallback when a specialization is unavailable

### Test Matrix

Required correctness cases:

- 17-row exact run, regime off, no risk, non-lockstep
- 17-row exact run, regime off, no risk, lockstep
- 17-row exact run with calmar objective
- 17-row exact run with max-drawdown constraint
- 22-row exact run with `include_trade_metrics=true`
- 22-row exact run with trade summary fields decoded from GPU output
- exact run with `eps_rel = 0`
- exact run with zero commission
- exact run with non-zero commission
- exact run where the specialization symbol is intentionally absent and the route falls back cleanly

Required routing cases:

- dispatch chooses M17 specialization when `metrics_count == 17`
- dispatch chooses M22 specialization when `metrics_count == 22`
- dispatch keeps current M10 reduced route when `metrics_count == 10`
- dispatch keeps current M7 route when `metrics_count == 7`
- dispatch does not route windowed full-row requests to non-windowed specialized symbols

Required contract cases:

- `trade_on_next_bar=false` remains rejected by strategy validation
- current host path continues to set `STRAT_SIGNED_EXPOSURE`
- reduced-row max-drawdown threshold conversion stays intact

### Profiling Matrix

Minimum benchmark matrix for this plan:

- baseline generic 17-row exact route
- specialized 17-row non-lockstep route
- specialized 17-row lockstep route
- baseline generic 22-row exact route
- specialized 22-row non-lockstep route
- specialized 22-row lockstep route
- generic lockstep route vs combined lockstep+dedup route when the latter exists

Optional benchmark matrix:

- zero-commission vs non-zero-commission full-row routes
- `eps0` vs non-zero-`eps_rel` full-row routes
- windowed exact full-row route if it appears hot in the first profile pass

Performance verification:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/profile_gpu_backtest_ncu.ps1`
- `powershell -File scripts/profile_gpu_backtest_nsys.ps1`

Measurement protocol:

- never run multiple benchmarks at the same time
- verify no other benchmark or profiling process is active before each run
- launch Nsight profiling from an elevated PowerShell terminal
- compare at least:
  - baseline 17-row exact route
  - specialized 17-row route
  - baseline 22-row exact route
  - specialized 22-row route
  - lockstep vs lockstep+dedup when both exist

## Risks And Guardrails

Main risks:

- specialization-matrix explosion
- silent dispatch fallback that hides dead code
- correctness drift between reduced-row and full-row decode behavior
- overfitting to a path that is not actually dominant in current workloads

Guardrails:

- implement specializations in bounded waves
- add dispatch logging and route expectation tests
- preserve CPU fallback and reduced-row product behavior
- do not make profiling-driven decisions from synthetic assumptions alone

## Implementation Checklist

### Phase 1 Checklist: Correctness Lock

- [x] Add a device helper that centralizes the current search-row log-space rule for `M < 17`.
- [x] Use that helper in the metric writer.
- [x] Use that helper in calmar scoring.
- [x] Confirm that current reduced-row max-drawdown threshold conversion still matches the device-side comparison contract.
- [x] Decide the fate of `regime_range_exit_rel`: implement meaningfully or remove/deprecate the active contract.
- [x] Add or update tests for 17-row calmar decode behavior.
- [x] Add or update tests for 17-row max-drawdown constraint behavior.
- [x] Keep `trade_on_next_bar=true` validation explicit and unchanged.
- [x] Keep the current host-side `STRAT_SIGNED_EXPOSURE` invariant explicit in tests or comments.

### Phase 2 Checklist: New Kernel Symbols

- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_off_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_on_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_on_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_hybrid_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_m17_regime_hybrid_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_on_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_hybrid_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_hybrid_norisk_eps0_nocomm`.
- [x] Add the same first-wave and second-wave family for `m22`.
- [ ] Decide whether wrapper-only constant propagation is sufficient or a fixed-`M` template body is required.

### Phase 3 Checklist: Host Dispatch

- [x] Load all new M17 symbols in both host symbol-loading blocks.
- [x] Load all new M22 symbols in both host symbol-loading blocks.
- [x] Extend both full-row dispatch blocks to route 17-row requests to M17 kernels.
- [x] Extend both full-row dispatch blocks to route 22-row requests to M22 kernels.
- [x] Preserve current reduced-row M7 route precedence.
- [x] Preserve current reduced-row M10 product-route precedence.
- [x] Add or update dispatch logging so M17 / M22 routes are visible in logs.
- [x] Add tests for specialized-route selection when symbols are present.
- [x] Add tests for clean fallback when a matching symbol is unavailable.

### Phase 4 Checklist: Route Matrix And Lockstep Overhead

- [x] Benchmark a generic combined `double_cross_backtest_tm_flex_f32_lockstep_time_fast_warp_dedup` route.
- [x] Revert the generic combined route after measured regression.
- [x] Reject the first `__shfl_sync`-based warp-uniform `lr` / `ret1` lockstep helper after measured regression.
- [x] Capture Nsight Compute on the hot lockstep `m17` / `m22` regime-on kernels and record whether they are memory- or register-limited.
- [x] Reject the first selective `__launch_bounds__(128, 2)` experiment on the hot lockstep regime-on kernels after mixed results and a 22-row regression.
- [ ] Add warp-uniform lockstep loads for `lr`.
- [ ] Add warp-uniform lockstep loads for `ret1`.
- [ ] Add warp-uniform regime-value loads for lockstep regime paths.
- [x] Benchmark whether M17 lockstep full-row routes should also get dedicated dedup variants.
- [x] Benchmark whether M22 lockstep full-row routes should also get dedicated dedup variants.
- [x] Keep dense / `pf`-major compact valid-pair ordering covered by tests or comments.
- [ ] Re-check whether windowed exact full-row runs are hot enough to justify a dedicated family.

### Phase 5 Checklist: Optional Pipeline Fusion

- [ ] Re-profile after M17 / M22 specialization and route-matrix work.
- [ ] Confirm whether downstream objective / constraint reread is now a material bottleneck.
- [ ] Only if profiling justifies it, prototype `score_out`.
- [ ] Only if profiling justifies it, prototype `feasible_out`.
- [ ] Keep the existing metrics-row result contract unchanged while testing fused outputs.

### Verification Checklist

- [ ] Run `cargo test -p ta_app_core` after each specialization wave.
- [ ] Run targeted exact-route GPU tests for 17-row paths.
- [ ] Run targeted exact-route GPU tests for 22-row paths.
- [x] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` before handoff.
- [ ] Run Nsight Compute from an elevated PowerShell session for the benchmark matrix above.
- [ ] Run Nsight Systems from an elevated PowerShell session if stage dominance changes after specialization.

## Definition Of Done

The plan is complete when:

- full-row 17 and 22 metric routes have dedicated reachable kernels
- the current guide-backed correctness debt that still affects those routes is closed
- the dispatch matrix covers the important full-row exact combinations used by the app
- profiling has been rerun and recorded against the new full-row paths
- any remaining optimization ideas are either measured backlog items or intentionally deferred
