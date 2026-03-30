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
- targeted lockstep `regime_hybrid` full-row specializations remain compiled and routed in the exact-kernel path.
- the direct exact benchmark runner now keeps `regime_hybrid` on the `GpuKernel` backend route, so fixed-contract exact profiling is available on that strategy surface.
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
- corrected replay-backed Nsight Compute on the hot `trade22` shortlist kernel now points more clearly at the next bounded coalescing target:
  - kernel: `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`
  - `62` registers per thread, `66.67%` theoretical occupancy, `53.83%` achieved occupancy
  - no local/shared spilling was reported
  - global load sector utilization is poor at about `17.4 / 32` bytes used per sector
  - global store sector utilization is very poor at about `4.0 / 32` bytes used per sector
  - the largest sampled stall remains L1TEX scoreboard dependency at about `5.4` cycles, about `52.4%` of the average `10.3` cycles between issued instructions
  - Nsight speedup guidance on this corrected hotspot was about `8.152%` for global-load efficiency and `15.62%` for global-store efficiency
- a bounded fixed-row metrics-write experiment was then run directly against that signal:
  - first pass: add paired `float2` stores for both `M17` and `M22` in `store_metrics_row_fixed_f32()`
  - controlled A/B on `alma/alma` + `xlarge/370750pairs` + `objective=sharpe` + `shortlist_slice_mode=halves`:
    - paired-store state:
      - `trade22`: `456 / 454 / 453 ms`
      - `full17`: `437 / 436 / 430 ms`
    - scalar-write reverted state:
      - `trade22`: `465 / 473 / 461 ms`
      - `full17`: `428 / 424 / 419 ms`
  - conclusion: reject the `M17` paired-store variant after the direct `full17` regression, but keep the `M22` paired-store specialization because the corrected-scope `trade22` A/B stayed favorable
- current kept code state for that experiment:
  - `store_metrics_row_fixed_f32<22>()` now uses paired `float2` stores for the fixed 22-row metrics layout
  - all other fixed-row contracts, including `M17`, remain on the original scalar write path
- fresh Nsight Systems captures taken after that experiment became inconsistent with the direct runner timings in the same session.
  - treat the corrected A/B runner timings and the imported corrected Nsight Compute counters as the acceptance evidence
  - do not treat those later Nsight Systems wall times as acceptance evidence for or against the kept `M22`-only store specialization
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
- Nsight Compute is now also captured on the direct windowed `regime_hybrid` exact surface:
  - reports captured at `target/nsight/regime_hybrid_windowed_full17_direct_profile.ncu-rep` and `target/nsight/regime_hybrid_windowed_trade22_direct_profile.ncu-rep`
  - xlarge `regime_hybrid` + `alma/alma` + `eval_range=[1000,199000)` hot-kernel summary:
    - `windowed_m17_regime_hybrid_norisk_eps0_nocomm`: `411.06 ms`, compute throughput `68.29%`, memory throughput `26.53%`, `53` registers/thread, theoretical occupancy `75.0%`, achieved occupancy `59.32%`
    - `windowed_m22_regime_hybrid_norisk_eps0_nocomm`: `430.34 ms`, compute throughput `67.66%`, memory throughput `23.61%`, `59` registers/thread, theoretical occupancy `66.67%`, achieved occupancy `54.38%`
  - both windowed reports show the same dominant hints:
    - no local or shared spilling
    - ALU is still the highest-utilized pipeline
    - the largest sampled stall remains L1TEX scoreboard dependency
    - global load/store sector utilization is poor enough that future work should treat coalescing and load/store locality as a more credible next target than generic launch-bounds tuning
- a first source-level access-pattern experiment is now implemented and kept on the windowed family:
  - change shape: walk the time-major `fast_ma_T`, `slow_ma_T`, `lr`, and `ret1` pointers directly inside the non-lockstep windowed body instead of rebuilding indexed `t * stride + i` addresses every iteration
  - scope: windowed kernels only; the generic non-windowed and lockstep families are unchanged
  - direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)`:
    - before pointer walking:
      - `full17`: `377 / 385 / 392 ms` min / median / max, `384.7 ms` mean
      - `trade22`: `386 / 392 / 403 ms` min / median / max, `393.7 ms` mean
    - after the retained windowed-only pointer walk:
      - `full17`: `373 / 376 / 376 ms` min / median / max, `375.0 ms` mean
      - `trade22`: `388 / 394 / 396 ms` min / median / max, `392.7 ms` mean
  - the first broader non-windowed version of the same idea was also measured and then narrowed back out:
    - it still helped the direct windowed hot surface
    - but it introduced mixed full-range behavior on non-windowed exact routes, which is why only the windowed family kept the change
  - verification:
    - `cargo test -p ta_app_core fixed_contract_kernel_flavor_covers_full_row_regime_variants --features cuda,cuda-backtest-kernel`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - conclusion: this is the first positive bounded response to the windowed L1TEX / address-dependency investigation; it stays on the windowed family because `full17` improves materially and `trade22` stays effectively neutral
- a follow-up regime-value hoist on top of that retained pointer-walk path was prototyped and rejected:
  - change shape: pre-walk `regime_filter` and feed the loaded regime value directly into fixed `RegimeOn` / `RegimeHybrid` helpers inside the same windowed path
  - workload: direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + objective `sharpe`
  - baseline retained pointer-walk state:
    - `full17`: `373 / 376 / 376 ms` min / median / max, `375.0 ms` mean
  - with the regime-value hoist enabled:
    - `full17`: `390 / 394 / 396 ms` min / median / max, `393.3 ms` mean
  - decision:
    - revert immediately
    - do not spend further time on direct regime-value helper inlining until profiling shows a stronger bottleneck than the retained generic pointer walk
- a second bounded write-path experiment is now implemented and kept on the same windowed family:
  - change shape:
    - add a fixed-`M` template path to the shared non-lockstep body for these wrappers
    - use `zero_metrics_row_fixed_f32<17/22>()` for invalid rows and empty eval windows
    - use `store_metrics_row_fixed_f32<17/22>()` for the final metrics row writeout
    - thread the fixed metric count through the dedicated windowed M17 / M22 wrappers instead of relying on wrapper-only constant propagation
  - workload: direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + objective `sharpe`
  - baseline retained pointer-walk state:
    - `full17`: `373 / 376 / 376 ms` min / median / max, `375.0 ms` mean
    - `trade22`: `388 / 394 / 396 ms` min / median / max, `392.7 ms` mean
  - with fixed-row write specialization enabled:
    - `full17`: `370 / 373 / 374 ms` min / median / max, `372.3 ms` mean
    - `trade22`: `383 / 393 / 393 ms` min / median / max, `389.7 ms` mean
  - verification:
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
      - passed with Criterion GPU mean about `82.0 ms`
  - conclusion:
    - wrapper-only constant propagation was not sufficient for this slice
    - a fixed-`M` body path is now justified and retained for the dedicated windowed M17 / M22 wrappers
    - the next credible write-path experiment is a larger layout change such as padded windowed `metrics_stride`, not more helper-level regime inlining
- the exact eval / replay path now truly honors the requested full-row contract instead of silently collapsing back to 17-row decode/routing:
  - `eval_double_ma_batch_gpu_kernel_impl()` now derives exact replay row count from `metric_contract` / `include_trade_metrics`
  - eval/replay symbol loading now includes the fixed-contract `windowed_m22`, `m22`, and `lockstep_m22` families alongside the existing M17 families
  - eval/replay dispatch now uses the same fixed-contract selection model as optimize, including `windowed_m22` and the dedicated no-risk regime flavors
  - the generic `double_cross_backtest_tm_flex_f32_windowed` wrapper is no longer accidentally hard-wired to the fixed 17-row write path
  - coverage:
    - `gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics`
    - `gpu_kernel_regime_hybrid_trade22_populates_trade_tail_metrics`
    - `exact_eval_metrics_count_contract_respects_trade22_request`
    - `finalist_recompute_eval_range_preserves_windowed_span`
  - live runner confirmation on the product-facing shortlist-slice surface:
    - `regime_hybrid` + `alma/alma` + `large/58300pairs` + `metric_contract=trade22` + `shortlist_slice_mode=halves`
    - main search still logs `dispatch=lockstep_m22`
    - replay ranges now log `VECTORBT_KERNEL_EVAL_DISPATCH ... dispatch=windowed_m22 ... metrics_count=22 ... fixed_metrics_contract=Some(22)`
    - one measured run completed at about `257 ms`
- verification after the replay fix:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
    - passed
    - Criterion `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: about `84.07 ms`
- a third bounded write-layout experiment was measured on the same windowed family and then rejected:
  - change shape:
    - raise the windowed full-row exact `metrics_stride` from `17/22` to `24`
    - keep host decode on the true metric row count so padded rows do not change full17 decode semantics on exact finalist reruns
    - keep fixed-contract dispatch strict off-windowed so padded full-row strides do not create a latent non-windowed misroute
  - workload: direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + objective `sharpe`
  - corrected-scope A/B on the same binary:
    - `full17` with padded `metrics_stride=24`: `372 / 377 / 377 ms` min / median / max, `375.3 ms` mean
    - `full17` with forced exact `metrics_stride=17`: `374 / 375 / 375 ms` min / median / max, `374.7 ms` mean
    - `trade22` with padded `metrics_stride=24`: `386 / 389 / 393 ms` min / median / max, `389.3 ms` mean
    - `trade22` with forced exact `metrics_stride=22`: `386 / 390 / 393 ms` min / median / max, `389.7 ms` mean
  - verification:
    - `cargo test -p ta_app_core fixed_metrics_contract_rows_require_matching_metric_stride --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core fixed_contract_dispatch_selects_specialized_routes_when_symbols_are_present --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core exact_gpu_kernel_config_preserves_metric_contract_request --features cuda,cuda-backtest-kernel`
  - conclusion:
    - the corrected-scope delta is too small and inconsistent to justify carrying the extra stride specialization
    - revert it and keep the tree on the stronger retained pointer-walk and fixed-row write-path changes only
    - the next credible step is either a larger row-write layout change or deferring tail trade metrics out of the all-pairs pass
- a targeted windowed fast-side dedup experiment was prototyped on the exact hot `regime_hybrid` surface, measured, and reverted in the same session:
  - workload: `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + objective `sharpe`
  - baseline current specialized windowed route:
    - `full17`: `383 / 388 / 389 ms` min / median / max, `386.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `390 / 393 / 393 ms` min / median / max, `392.0 ms` mean across 3 measured iterations after 1 warmup
  - with narrow windowed `FAST_WARP_DEDUP=true` M17 / M22 `eps0 + nocomm` kernels enabled through the existing dedup gate:
    - `full17`: `458 / 458 / 459 ms` min / median / max, `458.3 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `533 / 534 / 535 ms` min / median / max, `534.0 ms` mean across 3 measured iterations after 1 warmup
  - conclusion: do not expand the current fast-side dedup helper into dedicated windowed full-row kernels; the present helper shape is a clear regression on the benchmark-hot direct windowed surface
- a temporary alternate compact valid-pair order probe was measured on device-only exact routes and then removed from runtime:
  - the probe temporarily exposed `VECTORBT_KERNEL_GLOBAL_TOPK_VALID_PAIR_ORDER=slow_major`, but only when `global_topk_mode` was active, so it never changed the host decode / export path that still reconstructs dense pf-major compact rows on the CPU
  - direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)`:
    - baseline current order:
      - `full17`: `377 / 385 / 392 ms` min / median / max, `384.7 ms` mean
      - `trade22`: `386 / 392 / 403 ms` min / median / max, `393.7 ms` mean
    - with temporary `slow_major` probe:
      - `full17`: `379 / 379 / 388 ms` min / median / max, `382.0 ms` mean
      - `trade22`: `385 / 391 / 392 ms` min / median / max, `389.3 ms` mean
  - direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + full evaluation range:
    - baseline current order:
      - `full17`: one measured pass produced `457 / 418 / 418 ms`; the stable steady-state samples were `418 / 418 ms`
      - `trade22`: `418 / 420 / 422 ms` min / median / max, `420.0 ms` mean
    - with temporary `slow_major` probe:
      - `full17`: `420 / 422 / 423 ms` min / median / max, `421.7 ms` mean
      - `trade22`: `414 / 418 / 420 ms` min / median / max, `417.3 ms` mean
  - conclusion: alternate compact-order execution is a real probe, but the gain is too small and inconsistent to justify carrying a runtime knob for it yet
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
- the targeted windowed fast-side dedup experiment is also a regression, so the current helper design is no longer a credible next step for exact windowed M17 / M22 work
- the temporary `slow_major` compact-order probe produced only mixed low-single-digit movement, so it was not retained in the runtime
- the first source-level windowed pointer-walk experiment is the first kept access-shape win on this surface: it materially improves `full17`, keeps `trade22` roughly flat, and is still stronger evidence than the rejected occupancy and dedup experiments
- the first regime-value hoist on top of that pointer walk is now also measured as a regression, so remaining windowed work should bias toward larger layout or write-path changes rather than more helper-level regime inlining
- the second bounded write-path experiment is also now a kept win on top of the retained pointer walk:
  - the fixed-`M` windowed write path improves both direct exact contracts, with a clearer `full17` gain and a smaller `trade22` gain
  - the passing GPU perf gate is enough to keep it, but the higher small Criterion mean means it should still be treated as a targeted exact-windowed win rather than a universal throughput improvement
- the first Nsight Compute pass does not point to a DRAM-bound kernel:
  - both hot lockstep regime-on kernels are primarily compute / latency limited
  - both are register-limited on theoretical occupancy
  - that makes warp-uniform regime-value broadcast cleanup a lower-confidence next step than it looked before profiling
- the first bounded register-pressure experiment also failed to produce a stable cross-contract win
  - future occupancy tuning should stay narrowly targeted and be treated as opt-in experimentation, not presumed progress
- shortlist-slice reruns are now confirmed as a usable windowed exact/full-row benchmark surface:
  - `profile_gpu_dynamic_runner` already drives it through `shortlist_slice_mode`
  - this is not a direct request-level `eval_range` surface, but it does trigger exact subrange reruns inside the kernel executor
- a direct request-level windowed benchmark surface is now available for exact GPU routes:
  - public entrypoint: `run_double_ma_optimization_blocking_with_candles_and_eval_range(...)`
  - runner flags: `--eval-start` and `--eval-end`
  - direct smoke runs on `xlarge` `double_ma` + `alma/alma` + objective `sharpe`:
    - `full17` with `[1000, 199000)` logs `dispatch=windowed_m17` and ran at `335 ms`
    - `trade22` with `[1000, 199000)` logs `dispatch=windowed_m22` and ran at `359 ms`
  - when `eval_range` is set, the current runner stays on the exact non-dynamic kernel path even if dynamic preflight is available
- windowed exact full-row hotness is now measured and justified:
  - before specialization, `xlarge` `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves` ran at `507 ms` in a direct runner pass
  - Nsight Systems on that same surface showed the generic `double_cross_backtest_tm_flex_f32_windowed` kernel at `140.1 ms` total across 6 launches, `30.7%` of GPU kernel time
- the first narrow windowed specialization slice is now implemented:
  - new CUDA symbols:
    - `double_cross_backtest_tm_flex_f32_windowed_m17`
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
    - `double_cross_backtest_tm_flex_f32_windowed_m22`
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`
  - host dispatch now routes fixed-contract windowed requests to the new symbols when present
  - the exact shortlist-slice rerun helper now routes its 17-row windowed replay path to `windowed_m17`, preferring the hot `regime_off + no_risk + eps0 + nocomm` flavor when applicable
- the measured gain is material:
  - `xlarge` `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves` now runs at `433 / 429 / 425 ms` min / median / max, `429.0 ms` mean across 3 measured iterations after 1 warmup
  - `xlarge` `double_ma` + `alma/alma` + `trade22` + `shortlist_slice_mode=halves` now runs at `442 / 445 / 447 ms` min / median / max, `444.7 ms` mean across 3 measured iterations after 1 warmup
  - the older `trade22` shortlist-slice number above should now be treated as a stale pre-fix reference because the replay / eval path was still pinned to 17 rows when it was captured
  - Nsight Systems after specialization shows the 6 rerun launches now using `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm` at `95.4 ms` total, `22.5%` of GPU kernel time
- the direct windowed `trend_quality` exact surface is now measured and specialized:
  - before the new slice, direct `xlarge` `trend_quality` + `alma/alma` + `eval_range=[1000,199000)` smokes landed on:
    - `dispatch=windowed_m17` at about `454 ms`
    - `dispatch=windowed_m22` at about `488 ms`
  - Nsight Systems on that direct surface showed the generic windowed full-row kernels dominating:
    - `double_cross_backtest_tm_flex_f32_windowed_m17`: `395.0 ms`, `96.5%` of GPU kernel time
    - `double_cross_backtest_tm_flex_f32_windowed_m22`: `440.0 ms`, `96.9%` of GPU kernel time
  - the second narrow windowed specialization slice is now implemented:
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk`
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk_eps0_nocomm`
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk`
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk_eps0_nocomm`
  - host dispatch now routes windowed fixed-contract `RegimeOnNoRisk` and `RegimeOnNoRiskEps0NoComm` requests to those symbols when present
  - warmed direct runner timings on the same exact surface now land at:
    - `full17`: `417 / 418 / 423 ms` min / median / max, `419.3 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `437 / 437 / 439 ms` min / median / max, `437.7 ms` mean across 3 measured iterations after 1 warmup
  - Nsight Systems now shows the new regime-on windowed kernels as the dominant exact kernels:
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk_eps0_nocomm`: `413.5 ms`, `96.7%` of GPU kernel time
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk_eps0_nocomm`: `444.0 ms`, `96.9%` of GPU kernel time
- the direct windowed `regime_hybrid` exact surface is now measured and specialized:
  - before the new slice, direct `xlarge` `regime_hybrid` + `alma/alma` + `eval_range=[1000,199000)` smokes landed on:
    - `dispatch=windowed_m17` at about `475 ms`
    - `dispatch=windowed_m22` at about `506 ms`
  - Nsight Systems on that direct surface previously showed the generic windowed full-row kernels dominating:
    - `double_cross_backtest_tm_flex_f32_windowed_m17`: `426.9 ms`, `96.8%` of GPU kernel time
    - `double_cross_backtest_tm_flex_f32_windowed_m22`: `457.2 ms`, `97.2%` of GPU kernel time
  - the third narrow windowed specialization slice is now implemented:
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk`
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm`
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk`
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk_eps0_nocomm`
  - host dispatch now routes windowed fixed-contract `RegimeHybridNoRisk` and `RegimeHybridNoRiskEps0NoComm` requests to those symbols when present
  - warmed direct runner timings on the same exact surface now land at:
    - `full17`: `379 / 379 / 384 ms` min / median / max, `380.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `383 / 386 / 389 ms` min / median / max, `386.0 ms` mean across 3 measured iterations after 1 warmup
  - Nsight Systems now shows the new regime-hybrid windowed kernels as the dominant exact kernels:
    - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm`: `372.4 ms`, `96.2%` of GPU kernel time
    - `double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk_eps0_nocomm`: `376.1 ms`, `96.2%` of GPU kernel time
- a later host-route audit found that the exact `GpuHybridKernel` phase-1 path was silently dropping `include_trade_metrics` and `metric_contract`, forcing `trade22` requests back onto the 17-row contract on the direct `regime_hybrid` surface:
  - evidence before the fix:
    - direct dispatch logging on `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + `metric_contract=trade22` showed `dispatch=windowed_m17`, `metrics_count=17`, and `fixed_metrics_contract=Some(17)`
    - repeated direct measurements then collapsed to wrong output on the same `(39,138)` pair, for example `462 / 463 / 465 ms` with `sharpe=0.026430`
  - action taken:
    - rejected the attempted `trade22 -> 17-row search + finalist refill` contraction because the current refill path still replays only 17 rows and cannot restore the 22-row trade tail safely
    - kept the separate `eval_range` finalist-recompute fix so replay-backed exact routes no longer hardcode `[0, candles.len())`
    - threaded `include_trade_metrics` and `metric_contract` through `optimize_double_ma_gpu_hybrid_kernel_phase1()`
    - added a focused GPU test that exercises the `regime_hybrid` exact trade22 surface and asserts the trade-tail fields are populated
  - corrected direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` timings after the host-route fix:
    - `full17`: `378 / 380 / 380 ms` min / median / max, `379.3 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `388 / 391 / 396 ms` min / median / max, `391.7 ms` mean across 3 measured iterations after 1 warmup
  - corrected dispatch evidence after the host-route fix:
    - direct dispatch logging on the same `trade22` surface now shows `dispatch=windowed_m22`, `metrics_count=22`, and `fixed_metrics_contract=Some(22)`
  - implication:
    - the earlier direct `regime_hybrid` `trade22` timings recorded above should be treated as invalid historical data because they were still running on the 17-row contract
- a later host-side revisit tried to re-enable `trade22 -> 17-row search + 22-row refill` now that the exact eval / replay path was repaired, then rejected it again and reverted it:
  - change shape under test:
    - allow the existing `trade22 -> 17-row` search contraction only when finalist recompute is available
    - move the 22-row refill later in the host finalize / rerank flow so the explicit contract is restored after ranking
  - measured outcome:
    - direct `large/58300pairs` + `alma/alma` + `trade22` regressed from the stable mid-`80 ms` range to about `184 ms` on a one-iteration logged run and about `140-145 ms` on repeated measured iterations
    - `xlarge` shortlist-slice reruns briefly hit `CUDA_ERROR_INVALID_VALUE` because the slice reranker was already issuing exact `m22` refreshes and the extra post-rank refill became redundant
  - decision:
    - revert the host-side refill revisit completely
    - keep the earlier rejection in force: this contraction is still not a credible performance win on the current runtime architecture even after the eval-path repairs
- a separate exact-eval audit then found that the replay / eval implementation itself was also still structurally pinned to 17 rows:
  - evidence before the fix:
    - `eval_double_ma_batch_gpu_kernel_impl()` still chose tiles, kernel `M`, decode width, and stride from `METRICS_COUNT`
    - the eval path only loaded / selected `m17` and `windowed_m17` fixed-contract symbols, not the matching 22-row family
  - action taken:
    - added an exact-eval contract helper so replay-backed exact paths honor `metric_contract` and `include_trade_metrics`
    - loaded `windowed_m22`, `m22`, and `lockstep_m22` families in the eval path and switched eval dispatch onto the same fixed-contract selection logic used by the main optimize path
    - added a focused GPU test that exercises windowed exact eval with `metric_contract=trade22` and asserts the trade-tail fields are populated
    - added a second focused GPU test that drives shortlist-slice reruns with `include_trade_metrics=false` and `metric_contract=trade22`, proving the replay helper honors the contract instead of relying on the trade-metrics boolean
    - corrected the older multi-range exact-eval regression test so it only requires non-zero trade-tail fields on ranges that actually close trades
  - corrected dispatch evidence after the eval-path fix:
    - dispatch-logged GPU test now shows `VECTORBT_KERNEL_EVAL_DISPATCH: ... dispatch=windowed_m22 ... metrics_count=22 ... tile_metrics_stride=22 ... fixed_metrics_contract=Some(22)`
  - verification:
    - `cargo test -p ta_app_core exact_eval_metrics_count_prefers_metric_contract_over_trade_flag --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core gpu_kernel_exact_eval_trade22_metric_contract_populates_trade_tail_metrics --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core gpu_kernel_regime_hybrid_trade22_populates_trade_tail_metrics --features cuda,cuda-backtest-kernel`
  - corrected replay-backed shortlist-slice measurements on `double_ma` + `alma/alma` + `xlarge/370750pairs` + `shortlist_slice_mode=halves` after the eval-path fix:
    - `full17`: `432 / 434 / 442 ms` min / median / max, `436.0 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `465 / 468 / 468 ms` min / median / max, `467.0 ms` mean across 3 measured iterations after 1 warmup
    - dispatch logging on the same rerank surface now shows the all-pairs pass on `lockstep_m22` and the replay slices on `windowed_m22`
  - Nsight Systems on the corrected replay-backed `trade22` shortlist-slice surface (`target/nsight/double_ma_shortlist_trade22_postfix.nsys-rep`) now shows:
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`: `321.2 ms`, `73.6%` of GPU kernel time
    - 6 replay launches on `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`: `100.1 ms` total, `22.9%` of GPU kernel time
    - implication: the corrected replay-backed `trade22` slowdown is still mostly dominated by the main all-pairs `lockstep_m22` pass, with replay slices as a secondary cost rather than the primary bottleneck
  - current GPU perf gate summary on the updated state:
    - `target/gpu_perf_summary.json` now records `criterion_mean_ms=85.398143` for `double_ma/gpu_kernel/sma_sma/200k/small`
    - dynamic preflight ratio is now recorded as `72.75 / 80.75 = 0.900929`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` returned a passing state on the current tree
    - both checked values remain comfortably inside the budgets in `scripts/baselines/gpu_perf_baseline.json`
  - implication:
    - replay-backed exact `trade22` surfaces are no longer structurally forced through 17-row decode / dispatch
    - the older `444.7 ms` shortlist-slice `trade22` figure above is now a stale pre-fix reference, not the current corrected number
- a later repaired exact-eval / finalist-refill revisit kept only the exact-eval dedupe cleanup and rejected the contraction again:
  - kept code shape:
    - `exact_eval_metrics_for_params()` now dedupes exact replay params by `(fast_len, slow_len)` before the GPU batch eval and expands results back to caller order
    - `recompute_optimization_result_finalists_with_full_gpu_pass()` now uses that helper
  - rejected code shape:
    - a second `trade22 -> full17 search + 22-row restore` attempt was measured after the eval-path repairs and then reverted completely
    - it still regressed the replay-backed shortlist surface and briefly reintroduced unstable first-run behavior on the hot `trade22` path
  - corrected serial post-revert baselines on `double_ma` + `alma/alma` + `xlarge/370750pairs` + `shortlist_slice_mode=halves`:
    - `full17`: `391 / 391 / 392 ms` min / median / max, `391.3 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `395 / 398 / 401 ms` min / median / max, `398.0 ms` mean across 3 measured iterations after 1 warmup
    - best output is stable again on the same `(5,1000)` pair after the revert
  - current post-revert GPU perf gate summary:
    - Criterion `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `[82.498 ms 82.765 ms 83.054 ms]`
    - dynamic preflight: `71.5 ms` (on) vs `77.5 ms` (off)
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed on the reverted tree
  - implication:
    - keep the contraction rejected
    - treat the reverted shortlist baselines above as the current pre-experiment anchor for any next compact-pair hotspot work
- the next compact-pair hotspot slice is now implemented and kept in the shared backtest body:
  - change shape:
    - the backtest ABI now accepts optional compact valid-pair metadata for:
      - compact fast/slow period rows
      - compact fast/slow local row indices
    - the hot compact-output path now uses those local-row arrays directly instead of recomputing `pair_index_map -> dense_pair -> pf_local / ps_local` and then reloading period values lane-by-lane
    - optimize-path cached valid-pair maps now cache the extra local-row arrays alongside the existing compact period rows
    - exact eval / replay scratch now uploads the same compact period/local-row metadata for replay-backed full-row routes
  - added host coverage:
    - `build_valid_pair_local_rows_match_compact_pf_major_order`
  - measured serial `double_ma` + `alma/alma` results on the kept tree:
    - `xlarge/370750pairs` + `shortlist_slice_mode=halves`:
      - `full17`: `386 / 389 / 403 ms` min / median / max, `392.7 ms` mean across 3 measured iterations after 1 warmup
      - `trade22`: `398 / 401 / 401 ms` min / median / max, `400.0 ms` mean across 3 measured iterations after 1 warmup
    - compared with the corrected post-revert anchor above:
      - `full17` stayed roughly flat inside normal shortlist-surface noise
      - `trade22` stayed effectively flat inside normal shortlist-surface noise
    - direct `eval_range=[1000,199000)` runner smoke on the kept tree:
      - `full17`: `342 / 342 / 344 ms` min / median / max, `342.7 ms` mean across 3 measured iterations after 1 warmup
      - `trade22`: `355 / 359 / 360 ms` min / median / max, `358.0 ms` mean across 3 measured iterations after 1 warmup
    - compared with the corrected direct exact anchor above:
      - `full17` improved from `379.3 ms` to `342.7 ms`
      - `trade22` improved from `391.7 ms` to `358.0 ms`
  - Nsight Systems on the kept direct exact tree now shows:
    - artifact: `target/nsight/regime_hybrid_windowed_full17_compact_rows.nsys-rep`
    - hot kernel: `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm`
    - kernel time: `325.5 ms`, `95.8%` of GPU kernel time
  - verification:
    - `cargo build -p ta_app_core --bin profile_gpu_dynamic_runner --release --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core build_valid_pair_local_rows_match_compact_pf_major_order --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core build_valid_pair_index_map_preserves_dense_pf_major_order --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_exact_eval_trade22_metric_contract_populates_trade_tail_metrics --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core compact_valid_pair_output --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core fixed_contract_kernel_flavor_covers_full_row_regime_variants --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
      - passed
      - Criterion `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `[81.359 ms 81.602 ms 81.845 ms]`
      - dynamic preflight: `72.25 ms` (on) vs `76.75 ms` (off)
- implication:
  - keep the compact local-row metadata path
  - it is a targeted direct exact-route win, safe on the current gate, and a credible partial response to the compact-pair setup cost in the hot backtest body
  - it does not yet move the replay-heavy shortlist surface materially, so the next work should come from post-compact profiling or a larger write/layout experiment rather than revisiting the rejected 17-row contraction
- fresh recovered profiling now pins the current product-facing shortlist hotspot more cleanly:
  - Nsight Systems artifact: `target/nsight/double_ma_shortlist_full17_postrevert.nsys-rep`
  - xlarge `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves`:
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`: `285.5 ms`, `70.2%` of GPU kernel time
    - 6 replay launches on `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`: `105.5 ms` total, `26.0%` of GPU kernel time
  - implication:
    - the current shortlist surface is still dominated by the main all-pairs lockstep `m17` regime-off kernel
    - replay-windowed reruns are still material, but they are secondary on this surface after the retained direct exact improvements
- light Nsight Compute now also exists for the current recovered shortlist lockstep hotspots:
  - artifacts:
    - `target/nsight/double_ma_shortlist_full17_lockstep_ncu_light.ncu-rep`
    - `target/nsight/double_ma_shortlist_trade22_lockstep_ncu_light.ncu-rep`
  - `lockstep_time_m17_regime_off_norisk_eps0_nocomm`:
    - `275.27 ms`, compute throughput `67.90%`, memory throughput `23.21%`
    - `56` registers/thread, theoretical occupancy `75.0%`, achieved occupancy `66.42%`
    - no local/shared spilling
    - global loads still use only about `17.7 / 32` bytes per L1TEX sector
    - global stores still use only about `4.0 / 32` bytes per L1TEX sector
    - L1TEX scoreboard stalls are still about `5.4` cycles, about `48.9%` of the average `11.0` cycles between issued instructions
    - Nsight still calls out a tail-wave risk of up to `33.3%`
  - `lockstep_time_m22_regime_off_norisk_eps0_nocomm`:
    - `311.64 ms`, compute throughput `67.55%`, memory throughput `20.90%`
    - `64` registers/thread, theoretical occupancy `66.67%`, achieved occupancy `61.31%`
    - no local/shared spilling
    - global loads still use only about `17.7 / 32` bytes per L1TEX sector
    - global stores still use only about `4.0 / 32` bytes per L1TEX sector
    - L1TEX scoreboard stalls are still about `4.8` cycles, about `47.0%` of the average `10.3` cycles between issued instructions
    - Nsight still calls out a tail-wave risk of up to `33.3%`
- three additional bounded probes are now measured and rejected on the recovered tree:
  - lockstep fixed-contract pointer-walk addressing in the shared backtest body:
    - change shape: extend the retained pointer-walk idea into `LOCKSTEP_TIME` fixed `m17` / `m22` routes by prewalking `fast_ma_T`, `slow_ma_T`, `lr`, and `ret1`
    - workload: xlarge `double_ma` + `alma/alma` + `shortlist_slice_mode=halves`
    - measured result:
      - `full17`: `435 / 437 / 438 ms` min / median / max, `436.7 ms` mean
      - `trade22`: `468 / 470 / 472 ms` min / median / max, `470.0 ms` mean
    - decision: revert immediately; do not treat lockstep pointer walking as the next route-matrix win
  - padded `full17` lockstep/write-layout probe via `VECTORBT_KERNEL_METRICS_FULL17_STRIDE=18`:
    - workload: xlarge `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves`
    - measured result: `621 / 641 / 655 ms` min / median / max, `639.0 ms` mean
    - decision: reject; simple row padding alone is not a viable fix for the current shortlist hotspot
  - block-size override probes via `VECTORBT_KERNEL_BLOCK_X`:
    - workload: xlarge `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves`
    - `block_x=256`: `454 / 481 / 485 ms` min / median / max, `473.3 ms` mean
    - `block_x=96`: `461 / 472 / 506 ms` min / median / max, `479.7 ms` mean
    - decision: reject both overrides; the current chooser was not missing an easy launch-shape fix on this surface
- a later exact-only compact column-major output probe was also measured and rejected immediately:
  - change shape:
    - keep the existing exact windowed backtest kernels and host decode flow
    - switch only the exact replay compact scratch for fixed `windowed_m17` / `windowed_m22` routes to a column-major metrics layout so per-metric stores become warp-contiguous
    - add matching host-side column-major decode helpers and coverage tests
  - workload: direct `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + objective `sharpe`
  - measured result with the probe enabled:
    - `full17`: `392 / 400 / 384 ms` min / median / max
    - `trade22`: `1055 / 1079 / 1088 ms` min / median / max
  - decision:
    - revert immediately
    - do not revisit this exact column-major scratch idea without a much narrower theory for why the current `trade22` path explodes
- current measurement caveat after that revert:
  - the tree is back on the previously retained code path
  - however, later post-revert measurements in this same session became invalid because the workstation GPU was already saturated by external load
  - observed machine state:
    - `nvidia-smi`: about `98-100%` GPU utilization, about `47-48%` memory-controller utilization, about `304-323 W`
    - active external graphics workload: `DS2.exe`
  - implication:
    - do not treat the subsequent multi-second exact-run timings from this session as acceptance evidence for or against the reverted code
    - the next CUDA timing pass must start from an idle GPU
- the next idle-GPU recovery pass on the same tree found and fixed two real regressions on the shared small product surface:
  - first issue:
    - the bench build was no longer trustworthy because `use_windowed_col_major_exact_output` was referenced outside its defining scope in the exact replay helper
    - that scope bug is now fixed before taking any new acceptance timings
  - second issue:
    - the generic product valid-pair cache was still constructing and uploading compact fast/slow local-row metadata even though the non-windowed product path had already stopped using those pointers
    - the cache now keeps only compact pair indexes plus compact fast/slow period rows on the product path; exact replay scratch still owns its own local-row uploads where they were actually measured as useful
  - recovered measurements on an idle GPU:
    - direct `profile_gpu_dynamic_runner` sanity on `double_ma` + `sma/sma` + `small/1590pairs` + `heatmap_bins=64`: about `84-96 ms` wall/runtime across measured iterations on the restored default path
    - fresh Criterion rerun on `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `[92.990 ms 109.87 ms 140.57 ms]`
    - authoritative gate rerun on the same recovered tree:
      - Criterion `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `[91.906 ms 92.152 ms 92.392 ms]`
      - dynamic preflight: `79 ms` (on) vs `86.25 ms` (off)
      - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed
  - verification:
    - `cargo test -p ta_app_core build_valid_pair_local_rows_match_compact_pf_major_order --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_exact_eval_trade22_metric_contract_populates_trade_tail_metrics --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- the first explicit lockstep warp-uniform `lr` / `ret1` load experiment is now measured and rejected:
  - change shape:
    - add warp-uniform broadcast loads for `lr[t]` and `ret1[t]` inside the shared `LOCKSTEP_TIME` loop body
    - do not change regime loads, pair scheduling, or pointer-walk behavior in the same probe
  - workload: xlarge `double_ma` + `alma/alma` + `shortlist_slice_mode=halves`
  - pre-change anchor on the recovered tree:
    - `full17`: `455 / 440 / 440 ms` min / median / max, `441.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `458 / 456 / 461 ms` min / median / max, `458.3 ms` mean across 3 measured iterations after 1 warmup
  - measured result with the probe enabled:
    - `full17`: `469 / 460 / 460 ms` min / median / max, `463.0 ms` mean
    - `trade22`: `489 / 481 / 492 ms` min / median / max, `487.0 ms` mean
  - decision:
    - revert immediately
    - do not treat warp-uniform `lr` / `ret1` broadcast alone as a viable next lockstep win on the shortlist surface
- a later post-crash shortlist finalize pass first confirmed that the previously invoked release runner had been stale:
  - after rebuilding `profile_gpu_dynamic_runner`, the truthful current-source shortlist anchor briefly widened to:
    - `xlarge` `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves`: `487 / 489 / 487 ms` min / median / max, with `host_finalize` about `160-162 ms`
    - `xlarge` `double_ma` + `alma/alma` + `trade22` + `shortlist_slice_mode=halves`: `507 / 564 / 513 ms` min / median / max, with `host_finalize` about `170-174 ms`
  - focused profiling on that rebuilt state showed the widened shortlist tail was replay-heavy:
    - shortlist `full17` pipeline profile: `final_finalize` about `161.7 ms`, about `35.1%` of total pipeline time
    - replay profile on the same `halves` surface exposed three exact replay batches:
      - `50 finalists x 2 ranges`: about `61.8 ms`
      - another `50 finalists x 2 ranges`: about `50.7 ms`
      - `50 finalists x 4 ranges`: about `48.5 ms`
    - the replay timing was overwhelmingly landing in `d2h_decode`, which on this path also carries the replay-launch synchronization cost
- the kept fix for that rebuilt shortlist regression removes the redundant same-mode replay in `rerank_shortlist_by_slice_robustness()`:
  - kept changes:
    - keep the shortlist-slice rerank call on a borrowed slice instead of cloning the shortlisted row vector at the call site
    - stop rebuilding a second `ShortlistSliceDiagnostics` object after rerank when the helper has already limited diagnostics to `top_limit`
    - stop cloning the full sorted rerank list on the optimize path and return only `top_limit` rows there, while preserving the full-row host wrapper for tests
    - align top `halves` / `quartiles` diagnostic records by order instead of temporary `(fast_len, slow_len)` hash maps, with explicit mismatch checks
    - reuse the already-ranked diagnostics for the active slice mode instead of replaying that same mode a second time
  - corrected profiling after the kept replay-reuse change:
    - shortlist `full17` pipeline profile: `final_finalize` back down to about `115.9 ms`
    - replay profile on the same `halves` surface now shows only two batches:
      - `50 finalists x 2 ranges`: about `64.7 ms`
      - `50 finalists x 4 ranges`: about `50.6 ms`
  - fresh serial reruns on the kept tree:
    - `xlarge` `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves`: `442 / 435 / 436 ms` min / median / max, with `host_finalize` about `109-113 ms`
    - `xlarge` `double_ma` + `alma/alma` + `trade22` + `shortlist_slice_mode=halves`: `458 / 456 / 451 ms` min / median / max, with `host_finalize` about `117-120 ms`
  - conclusion:
    - keep the bounded host-side trims and the same-mode replay reuse
    - the rebuilt current-source shortlist surface is now back in line with the earlier recovered anchor
    - the remaining shortlist tail is still replay-heavy, but the obvious duplicate rerun is now gone
  - verification:
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result --features cuda,cuda-backtest-kernel -- --test-threads=1`
- the next kept shortlist trim removes duplicate shortlist-row materialization on the exact slice-only path when there are no extra shortlist heads and no union/confidence rerank:
  - kept change:
    - only build `materialized_shortlist_rows` on the optimize path when later stages actually need a separate shortlist buffer
    - plain `shortlist_slice_mode` reranks without extra shortlist heads now fall back to `result.top.as_slice()` instead of cloning the same rows during decode
  - fresh serial reruns on the kept tree:
    - `xlarge` `double_ma` + `alma/alma` + `full17` + `shortlist_slice_mode=halves`: `436 / 431 / 435 ms` min / median / max, `434.0 ms` mean, with `host_finalize` about `108-114 ms`
    - `xlarge` `double_ma` + `alma/alma` + `trade22` + `shortlist_slice_mode=halves`: `454 / 453 / 453 ms` min / median / max, `453.3 ms` mean, with `host_finalize` about `116-118 ms`
  - gate / verification:
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_request_matches_exhaustive_host_union --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
      - Criterion `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `[91.963 ms 92.231 ms 92.481 ms]`
      - dynamic preflight: `79 ms` (on) vs `87 ms` (off)
      - pass
- the next retained windowed replay experiment is a software-pipelined load path layered on top of the kept pointer-walk body:
  - change shape:
    - keep the scope limited to `WALK_SERIES_PTRS && !LOCKSTEP_TIME && !FAST_WARP_DEDUP`
    - preload the current `fast_ma_T`, `slow_ma_T`, `lr`, and `ret1` values once per pair
    - fetch the next-step values before consuming the current step so the long per-pair loop can overlap more load latency with the current iteration's arithmetic
    - leave lockstep kernels, generic non-windowed kernels, pair ordering, and host dispatch unchanged
  - target benchmark surface:
    - `xlarge` `double_ma` + `alma/alma` + `shortlist_slice_mode=halves`
    - fixed full-row reruns that hit `windowed_m17_regime_off_norisk_eps0_nocomm` / `windowed_m22_regime_off_norisk_eps0_nocomm`
  - fresh serial reruns on the kept state:
    - `full17`: `397 / 396 / 398 ms` min / median / max, `397.0 ms` mean, with `host_finalize` about `77-79 ms`
    - `trade22`: `424 / 420 / 425 ms` min / median / max, `423.0 ms` mean, with `host_finalize` about `83-86 ms`
  - comparison against the prior recovered shortlist anchor:
    - `full17`: from `434.0 ms` mean down to `397.0 ms`
    - `trade22`: from `453.3 ms` mean down to `423.0 ms`
    - `host_finalize` dropped by about `31-35 ms` on both contracts, which matches the replay-heavy tail this pass was targeting
  - post-change Nsight Systems smoke on the `full17` shortlist surface:
    - report: `target/nsight/double_ma_shortlist_full17_postpipeline.nsys-rep`
    - hot-kernel summary:
      - `lockstep_time_m17_regime_off_norisk_eps0_nocomm`: `282.8 ms`, `76.6%` of GPU kernel time
      - `windowed_m17_regime_off_norisk_eps0_nocomm`: `71.7 ms` across 6 launches, `19.4%` of GPU kernel time
      - `alma_batch_tiled_f32_2x_tile256_tm`: `14.8 ms`, `4.0%`
    - compared with the earlier post-trim smoke on the same surface, the windowed rerun kernel dropped from about `103.4 ms` total to about `71.7 ms`
  - decision:
    - keep the software-pipelined windowed load path
    - treat it as the first retained follow-on win after the earlier pointer-walk cleanup, because it moves the remaining replay kernel cost itself rather than just shaving host-side duplication
  - verification:
    - `cargo build -p ta_app_core --bin profile_gpu_dynamic_runner --release --features cuda,cuda-backtest-kernel`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `cargo test -p ta_app_core gpu_kernel_shortlist_slice_request_matches_exhaustive_host_union --features cuda,cuda-backtest-kernel -- --test-threads=1`
    - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
      - Criterion `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `[85.308 ms 85.601 ms 85.876 ms]`
      - dynamic preflight: `77.75 ms` (on) vs `79.75 ms` (off)
      - pass
- a fresh post-crash serial rerun on the current tree tightened the shortlist anchor again:
  - workload: `double_ma` + `alma/alma` + `xlarge/370750pairs` + objective `sharpe` + `shortlist_slice_mode=halves`
  - `full17`: `396 / 398 / 400 ms` min / median / max, `398.0 ms` mean across 3 measured iterations after 1 warmup, with `host_finalize` about `75-77 ms`
  - `trade22`: `419 / 432 / 434 ms` min / median / max, `428.3 ms` mean across 3 measured iterations after 1 warmup, with `host_finalize` about `83-85 ms`
  - note:
    - single cold runs on this surface can still produce misleading first-iteration outliers well above the warmed steady state, so only the warmed serial measurements above should be treated as the current acceptance anchor
- a bounded lockstep regime-value broadcast experiment was measured and rejected:
  - change shape:
    - add warp-uniform `regime_filter[t_sig]` loads only for the compile-time `REGIME_MODE=1` / `REGIME_MODE=2` lockstep routes
    - leave generic runtime-regime paths and non-lockstep routes unchanged
  - workload: `trend_quality` + `alma/alma` + `xlarge/370750pairs` + objective `sharpe`
  - measured result:
    - `full17`: `425 / 429 / 430 ms` min / median / max, `428.0 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `436 / 438 / 440 ms` min / median / max, `438.0 ms` mean across 3 measured iterations after 1 warmup
  - decision:
    - revert immediately
    - do not carry extra lockstep regime-broadcast complexity without a clear win; the measured result was effectively flat against the prior regime-on anchor
- a later raw-pointer windowed-load probe was also measured and rejected:
  - change shape:
    - replace the retained windowed pointer-walk `__ldg` loads with plain dereferences on the `WALK_SERIES_PTRS && !LOCKSTEP_TIME && !FAST_WARP_DEDUP` path
    - leave the rest of the shared body unchanged
  - apparent shortlist result on `double_ma` + `alma/alma` + `xlarge/370750pairs` + `shortlist_slice_mode=halves`:
    - `full17`: `391 / 393 / 415 ms` min / median / max, `399.7 ms` mean across 3 measured iterations after 1 warmup
    - `trade22`: `425 / 427 / 429 ms` min / median / max, `427.0 ms` mean across 3 measured iterations after 1 warmup
  - failing cross-check on the direct exact windowed surface:
    - `regime_hybrid` + `alma/alma` + `xlarge/370750pairs` + `eval_range=[1000,199000)` + `full17` regressed to `1102 / 1110 / 1119 ms` min / median / max after a `590 ms` warmup
  - decision:
    - revert immediately
    - do not treat shortlist-only improvement as valid acceptance evidence when the same load change catastrophically regresses the direct exact windowed route
- remaining open work is now centered on source-level investigation of the windowed L1TEX dependency / coalescing hints, deciding whether the windowed family should expand beyond the now-proven `regime_off`, `regime_on`, and `regime_hybrid` hot slices, and only then reconsidering downstream fusion work if the backtest kernel stops dominating

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
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m17`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m17_regime_on_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m17_regime_hybrid_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m22`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m22_regime_on_norisk_eps0_nocomm`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk`.
- [x] Add `double_cross_backtest_tm_flex_f32_windowed_m22_regime_hybrid_norisk_eps0_nocomm`.
- [x] Decide whether wrapper-only constant propagation is sufficient or a fixed-`M` template body is required.

### Phase 3 Checklist: Host Dispatch

- [x] Load all new M17 symbols in both host symbol-loading blocks.
- [x] Load all new M22 symbols in both host symbol-loading blocks.
- [x] Extend both full-row dispatch blocks to route 17-row requests to M17 kernels.
- [x] Extend both full-row dispatch blocks to route 22-row requests to M22 kernels.
- [x] Route windowed fixed-contract requests to dedicated windowed M17 / M22 kernels when present.
- [x] Route windowed `RegimeOnNoRisk` fixed-contract requests to dedicated M17 / M22 kernels when present.
- [x] Route windowed `RegimeHybridNoRisk` fixed-contract requests to dedicated M17 / M22 kernels when present.
- [x] Route shortlist-slice 17-row reruns to `windowed_m17` when present.
- [x] Expose a direct `eval_start` / `eval_end` benchmark surface for exact GPU routes.
- [x] Thread `include_trade_metrics` and `metric_contract` through the exact `GpuHybridKernel` phase-1 route instead of silently dropping them.
- [x] Make the exact eval / replay path honor `metric_contract` and `include_trade_metrics` instead of forcing 17-row decode and dispatch.
- [x] Load and route fixed-contract `windowed_m22` / `m22` / `lockstep_m22` families on the eval / replay path.
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
- [x] Benchmark and reject the first warp-uniform lockstep `lr` / `ret1` load experiment after shortlist regressions on both 17-row and 22-row surfaces.
- [x] Reuse the already-ranked shortlist diagnostics for the active slice mode instead of replaying that same mode a second time on the current top-k shortlist surface.
- [x] Skip duplicate shortlist-row materialization on the exact slice-only path when there are no extra shortlist heads and no union/confidence rerank stages.
- [ ] Add warp-uniform regime-value loads for lockstep regime paths.
- [ ] Investigate windowed L1TEX scoreboard stalls and poor global load/store sector utilization before any further launch-bounds or occupancy experiments.
- [x] Profile the corrected replay-backed `trade22` hotspot with Nsight Compute and extract the dominant coalescing / L1TEX findings into the tracker.
- [x] Keep the first positive windowed pointer-walk load experiment after a direct `full17` win, `trade22` neutrality, and a passing GPU perf gate.
- [x] Reject the first windowed regime-value hoist on top of the retained pointer walk after a direct `full17` regression.
- [x] Keep the fixed-row windowed M17 / M22 metrics write specialization after direct `full17` and `trade22` wins plus a passing GPU perf gate.
- [x] Reject the broad M17 paired-store final-row write experiment after a same-surface `full17` regression.
- [x] Keep the narrowed M22-only paired-store final-row write specialization after the corrected serial `trade22` shortlist A/B stays favorable.
- [x] Keep compact valid-pair local-row metadata only on the exact / replay surfaces that still use it, and remove the generic product cache/build cost after the recovered small-benchmark regression.
- [x] Measure the first padded windowed full-row `metrics_stride=24` experiment and reject keeping it after corrected-scope A/B results land inside noise.
- [x] Reject the first dedicated windowed fast-side dedup experiment after measured regression on the hot `regime_hybrid` direct exact surface.
- [x] Reject the attempted `trade22 -> 17-row search` contraction after confirming the current finalist refill path is not a safe 22-row restore surface.
- [x] Measure a temporary `slow_major` compact-order probe on direct exact surfaces and reject keeping a runtime knob for it for now.
- [x] Benchmark whether M17 lockstep full-row routes should also get dedicated dedup variants.
- [x] Benchmark whether M22 lockstep full-row routes should also get dedicated dedup variants.
- [x] Keep dense / `pf`-major compact valid-pair ordering covered by tests or comments.
- [x] Re-check whether windowed exact full-row runs are hot enough to justify a dedicated family.

### Phase 5 Checklist: Optional Pipeline Fusion

- [x] Re-profile after M17 / M22 specialization and route-matrix work.
- [x] Confirm whether downstream objective / constraint reread is not yet a material bottleneck.
- [ ] Only if profiling justifies it, prototype `score_out`.
- [ ] Only if profiling justifies it, prototype `feasible_out`.
- [ ] Keep the existing metrics-row result contract unchanged while testing fused outputs.

### Verification Checklist

- [ ] Run `cargo test -p ta_app_core` after each specialization wave.
- [ ] Run targeted exact-route GPU tests for 17-row paths.
- [x] Run targeted exact-route GPU tests for 22-row paths.
- [x] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` before handoff.
- [ ] Run Nsight Compute from an elevated PowerShell session for the benchmark matrix above.
- [x] Run Nsight Systems from an elevated PowerShell session if stage dominance changes after specialization.

## Definition Of Done

The plan is complete when:

- full-row 17 and 22 metric routes have dedicated reachable kernels
- the current guide-backed correctness debt that still affects those routes is closed
- the dispatch matrix covers the important full-row exact combinations used by the app
- profiling has been rerun and recorded against the new full-row paths
- any remaining optimization ideas are either measured backlog items or intentionally deferred
