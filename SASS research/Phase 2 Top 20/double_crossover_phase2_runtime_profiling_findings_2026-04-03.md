# `double_crossover` Phase 2 Runtime Profiling Findings

Date: 2026-04-03

## Purpose

This note records the first completed runtime-attribution bundle for SASS Phase 2. It is the evidence base to use before any new source changes or any GPT-5.4 Pro review pass.

It continues:

- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
- `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`

## Artifact Bundle

Direct-runner JSON baselines:

- `target/double_crossover_phase2_large_full17_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_large_trade22_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_xlarge_shortlist_full17_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_xlarge_shortlist_trade22_row_first_2026-04-03.jsonl`

Built-in dispatch and pipeline timing:

- `target/double_crossover_phase2_large_full17_pipeline_2026-04-03.log`
- `target/double_crossover_phase2_large_trade22_pipeline_2026-04-03.log`

Nsight Systems:

- `target/nsight/double_crossover_phase2_large_full17.nsys-rep`
- `target/nsight/double_crossover_phase2_large_full17.provenance.json`
- `target/nsight/double_crossover_phase2_large_full17_stats.txt`
- `target/nsight/double_crossover_phase2_large_trade22.nsys-rep`
- `target/nsight/double_crossover_phase2_large_trade22.provenance.json`
- `target/nsight/double_crossover_phase2_large_trade22_stats.txt`

Nsight Compute:

- `target/nsight/double_crossover_phase2_large_full17_backtest.ncu-rep`
- `target/nsight/double_crossover_phase2_large_full17_backtest.provenance.json`
- `target/nsight/double_crossover_phase2_large_full17_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_large_trade22_backtest.ncu-rep`
- `target/nsight/double_crossover_phase2_large_trade22_backtest.provenance.json`
- `target/nsight/double_crossover_phase2_large_trade22_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_reducers.ncu-rep`
- `target/nsight/double_crossover_phase2_xlarge_reducers.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_reducers_raw.csv`
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.json`
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`

## Measured Surfaces

Row-first direct-runner means:

- `large`, `full17`: `74.8 ms` mean end-to-end runner runtime
- `large`, `trade22`: `77.0 ms` mean end-to-end runner runtime
- `xlarge` shortlist, `full17`: `452.8 ms` mean end-to-end runner runtime
- `xlarge` shortlist, `trade22`: `441.8 ms` mean end-to-end runner runtime

Route state on all four surfaces:

- live route remains `row_first_global_topk`
- `compact_valid_pair_output=true`
- `global_topk_mode=true`
- score-first remains disabled by `env_disabled`

## Dispatch And Built-In Pipeline Timing

`large/full17`:

- dispatch resolves to `lockstep_m17`
- `fixed_metrics_contract=Some(17)`
- `decode_without_recompute=true`
- `tile_score` stays about `95.5%` to `96.8%` of total pipeline time
- `tile_prepare` stays about `2.3 ms` to `3.2 ms`
- `tile_finalize` stays about `0.09 ms` to `0.10 ms`
- `final_finalize` stays about `0.006 ms` to `0.010 ms`

`large/trade22`:

- dispatch resolves to `lockstep_m22`
- `fixed_metrics_contract=Some(22)`
- `decode_without_recompute=true`
- `tile_score` stays about `95.7%` to `96.6%` of total pipeline time
- `tile_prepare` stays about `2.4 ms` to `3.0 ms`
- `tile_finalize` stays about `0.07 ms` to `0.11 ms`
- `final_finalize` stays about `0.006 ms` to `0.010 ms`

Interpretation:

- built-in timing does not support a "finalize path dominates" story on the current large exact surfaces
- the live large exact route is still overwhelmingly backtest-body bound

## Nsight Systems Findings

`large/full17`:

- `cuStreamSynchronize`: `223,983,748 ns` total CUDA API time, `96.3%`
- dominant GPU kernel:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
  - `203,189,677 ns` total across `3` launches
  - `67.73 ms` average per launch
  - `95.5%` of traced GPU kernel time
- `gather_topk_pairs_tm_f32`: `5,088 ns` total
- `pack_metric_rows_f32`: `4,000 ns` total

`large/trade22`:

- `cuStreamSynchronize`: `231,323,215 ns` total CUDA API time, `96.6%`
- dominant GPU kernel:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`
  - `209,936,376 ns` total across `3` launches
  - `69.98 ms` average per launch
  - `95.3%` of traced GPU kernel time
- `gather_topk_pairs_tm_f32`: `6,016 ns` total
- `pack_metric_rows_f32`: `5,058 ns` total

Interpretation:

- the large exact route is still dominated by the backtest kernel, not by host orchestration and not by physical transfer bandwidth
- the heavy `cuStreamSynchronize` share is a host-wait symptom on the same hot kernels, not separate work
- current large-surface evidence does not justify treating pair gather or final row packing as the primary product bottleneck

## Nsight Compute Findings

### `large/full17` backtest kernel

Kernel:

- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`

Mean over three profiled launches:

- GPU time: `68.887 ms`
- SM throughput: `41.91%`
- compute-memory throughput: `13.02%`
- DRAM throughput: `1.41%`
- L1TEX throughput: `13.02%`
- registers per thread: `55`
- max-warp ceiling: `75.0%`
- active warps: `29.66%`
- average warp latency per issued instruction: `8.49`
- average long-scoreboard stall per issue-active cycle: `5.02`
- average short-scoreboard stall per issue-active cycle: `0.000235`
- average LG throttle per issue-active cycle: `0.001098`
- average math-pipe throttle per issue-active cycle: `0.226`
- branch-target uniformity: `99.99%`
- global-load bytes per sector: `17.60 / 32 B`
- global-store bytes per sector: `5.67 / 32 B`
- local spilling requests: `0`
- shared spilling requests: `0`

### `large/trade22` backtest kernel

Kernel:

- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`

Mean over three profiled launches:

- GPU time: `71.760 ms`
- SM throughput: `41.28%`
- compute-memory throughput: `12.82%`
- DRAM throughput: `1.48%`
- L1TEX throughput: `12.82%`
- registers per thread: `64`
- max-warp ceiling: `66.67%`
- active warps: `29.64%`
- average warp latency per issued instruction: `8.59`
- average long-scoreboard stall per issue-active cycle: `5.16`
- average short-scoreboard stall per issue-active cycle: `0.01497`
- average LG throttle per issue-active cycle: `0.000606`
- average math-pipe throttle per issue-active cycle: `0.174`
- branch-target uniformity: `99.99%`
- global-load bytes per sector: `17.60 / 32 B`
- global-store bytes per sector: `10.35 / 32 B`
- local spilling requests: `0`
- shared spilling requests: `0`

### `xlarge/full17` reducer capture

Captured reducer kernel:

- `gather_topk_pairs_tm_f32`

Mean over three profiled launches:

- GPU time: `3.029 ms`
- SM throughput: `0.0177%`
- compute-memory throughput: `1.48%`
- DRAM throughput: `1.31%`
- L1TEX throughput: `0.188%`
- registers per thread: `40`
- max-warp ceiling: `100%`
- active warps: `6.56%`
- average warp latency per issued instruction: `43.30`
- average long-scoreboard stall per issue-active cycle: `15.89`
- global-load bytes per sector: `4.18 / 32 B`
- global-store bytes per sector: `4.40 / 32 B`
- no spill story

Interpretation:

- the reducer-side gather is sparse and inefficient on a per-sector basis, but its absolute runtime is still small
- against the `xlarge/full17` row-first runtime mean of `452.8 ms`, this capture is about `0.7%` of end-to-end wall time
- this does not rule out compact finalist handling as a research topic, but it does weaken the claim that reducer/gather work is the first product bottleneck to attack

## What The Bundle Supports

This profiling bundle supports these claims:

1. the live exact row-first route is still dominated by the main backtest kernel
2. current large exact cost is not mainly Rust launch submission, D2H transfer volume, or final row packing
3. the hot kernels are not spill-driven
4. the hot kernels are not primarily a branch-divergence problem
5. long-scoreboard and dependency pressure remain the dominant in-kernel symptom
6. the `m22` store path is materially better than `full17` on store-width recovery, but it is still well below ideal sector utilization

## What The Bundle Does Not Yet Prove

This bundle does not yet prove:

1. that score-first is permanently uncompetitive rather than underdeveloped
2. that compact exact finalist handling can never matter on broader route surfaces
3. that the next best intervention is definitely backtest-body source/layout work rather than a route redesign
4. that fast-warp-dedup or distribution-aware specialization is unimportant on the right workload mix

## Recommended Immediate Next Step

Use this bundle for a stronger-model review before making more source changes.

The most defensible prompt to GPT-5.4 Pro is now:

- given this exact profiling evidence, what is the strongest next source/runtime intervention for real product performance
- what measurements would justify escalating from product optimization into the narrower research directions

The prompt should explicitly forbid generic CUDA advice and require the model to:

1. explain what the repo has already proved
2. explain what these new artifacts still leave unresolved
3. pick the best next one or two interventions
4. identify exact code symbols and measurement commands
5. define rejection criteria up front
