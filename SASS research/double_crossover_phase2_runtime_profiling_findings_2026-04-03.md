# `double_crossover` Phase 2 Runtime Profiling Findings

Date: 2026-04-03

## Purpose

This note records the completed Phase 2 runtime-attribution bundle after the shortlist-heavy `xlarge` follow-up pass.

It continues:

- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `SASS research/double_crossover_performance_findings_and_code_suggestions.md`
- `SASS research/double_crossover_sass_validated_implementation_and_investigation_plan_2026-04-02.md`
- `SASS research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md`

The goal of this file is narrower than the plan. It records what the profiler bundle now proves, what remains inference, and what Stage B branch is justified by repo evidence.

## Measurement Provenance

- Branch: `main`
- Commit: `d727607157cc224424a825b2d73c1160ee23060f`
- GPU: `NVIDIA GeForce RTX 4090`
- Driver: `595.71`
- Nsight Systems used for export and stats: `2026.2.1`
- Nsight Compute used for capture and import: `2026.1.0`
- Nsight profiling was run from elevated PowerShell

## Artifact Bundle

Direct-runner JSON baselines:

- `target/double_crossover_phase2_large_full17_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_large_trade22_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_xlarge_shortlist_full17_row_first_2026-04-03.jsonl`
- `target/double_crossover_phase2_xlarge_shortlist_trade22_row_first_2026-04-03.jsonl`

Built-in dispatch and pipeline timing:

- `target/double_crossover_phase2_large_full17_pipeline_2026-04-03.log`
- `target/double_crossover_phase2_large_trade22_pipeline_2026-04-03.log`
- `target/double_crossover_phase2_xlarge_full17_pipeline_2026-04-03.log`
- `target/double_crossover_phase2_xlarge_trade22_pipeline_2026-04-03.log`

Nsight Systems:

- `target/nsight/double_crossover_phase2_large_full17.nsys-rep`
- `target/nsight/double_crossover_phase2_large_full17.provenance.json`
- `target/nsight/double_crossover_phase2_large_full17_stats.txt`
- `target/nsight/double_crossover_phase2_large_trade22.nsys-rep`
- `target/nsight/double_crossover_phase2_large_trade22.provenance.json`
- `target/nsight/double_crossover_phase2_large_trade22_stats.txt`
- `target/nsight/double_crossover_phase2_xlarge_full17.nsys-rep`
- `target/nsight/double_crossover_phase2_xlarge_full17.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_full17_stats.txt`
- `target/nsight/double_crossover_phase2_xlarge_trade22.nsys-rep`
- `target/nsight/double_crossover_phase2_xlarge_trade22.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_trade22_stats.txt`

Nsight Compute:

- `target/nsight/double_crossover_phase2_large_full17_backtest.ncu-rep`
- `target/nsight/double_crossover_phase2_large_full17_backtest.provenance.json`
- `target/nsight/double_crossover_phase2_large_full17_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_large_trade22_backtest.ncu-rep`
- `target/nsight/double_crossover_phase2_large_trade22_backtest.provenance.json`
- `target/nsight/double_crossover_phase2_large_trade22_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_full17_backtest.ncu-rep`
- `target/nsight/double_crossover_phase2_xlarge_full17_backtest.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_full17_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_trade22_backtest.ncu-rep`
- `target/nsight/double_crossover_phase2_xlarge_trade22_backtest.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_trade22_backtest_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_full17_reducers.ncu-rep`
- `target/nsight/double_crossover_phase2_xlarge_full17_reducers.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_full17_reducers_raw.csv`
- `target/nsight/double_crossover_phase2_xlarge_trade22_reducers.ncu-rep`
- `target/nsight/double_crossover_phase2_xlarge_trade22_reducers.provenance.json`
- `target/nsight/double_crossover_phase2_xlarge_trade22_reducers_raw.csv`
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.json`
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`

## Measured Surfaces

Row-first direct-runner means:

- `large`, `full17`: `74.8 ms`
- `large`, `trade22`: `77.0 ms`
- `xlarge` shortlist, `full17`: `452.8 ms`
- `xlarge` shortlist, `trade22`: `441.8 ms`

Large exact route state:

- `gpu_kernel.path=row_first_global_topk`
- `gpu_kernel.compact_valid_pair_output=true`
- `gpu_kernel.global_topk_mode=true`
- `gpu_kernel.score_first_blockers=env_disabled`

Shortlist-heavy `xlarge` route state:

- `gpu_kernel.path=row_first_global_topk`
- `gpu_kernel.compact_valid_pair_output=true`
- `gpu_kernel.global_topk_mode=true`
- `gpu_kernel.score_first_blockers=env_disabled,shortlist_slice_enabled`
- `shortlist_slice_mode=halves`

Measured `xlarge` route timing on the retained measure iterations:

- `full17`: `explicit_sync_ms=278`, `host_finalize_ms=95`, `route_wall_ms=433`
- `trade22`: `explicit_sync_ms=291`, `host_finalize_ms=96`, `route_wall_ms=472`

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

`xlarge/full17`:

- dispatch resolves to `lockstep_m17`
- `fixed_metrics_contract=Some(17)`
- `decode_without_recompute=true`
- `tile_prepare` stays about `12.2 ms` to `13.6 ms`
- `tile_score` stays about `72.5%` to `80.8%` of total pipeline time
- `tile_finalize` stays about `0.11 ms` to `0.12 ms`
- `final_finalize` stays about `61.0 ms` to `93.3 ms`, or about `15.7%` to `24.3%`

`xlarge/trade22`:

- dispatch resolves to `lockstep_m22`
- `fixed_metrics_contract=Some(22)`
- `decode_without_recompute=true`
- `tile_prepare` stays about `13.5 ms` to `13.8 ms`
- `tile_score` stays about `73.3%` to `81.3%` of total pipeline time
- `tile_finalize` stays about `0.12 ms` to `0.13 ms`
- `final_finalize` stays about `57.0 ms` to `92.7 ms`, or about `15.0%` to `23.3%`

Interpretation:

- the `large` exact surfaces are still almost entirely backtest-body bound
- the shortlist-heavy `xlarge` surfaces are still backtest-family dominated, but unlike `large` they now show a real `final_finalize` slice
- the current code in `crates/app_core/src/gpu_backtest_kernel.rs` shows that `final_finalize` includes shortlist-slice rerank and replay work, not only host-side bookkeeping

## Nsight Systems Findings

`large/full17`:

- `cuStreamSynchronize`: `223,983,748 ns`, `96.3%` of traced CUDA API time
- dominant GPU kernel:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
  - `203,189,677 ns` total across `3` launches
  - `67.73 ms` average
  - `95.5%` of traced GPU kernel time
- `gather_topk_pairs_tm_f32`: `5,088 ns`
- `pack_metric_rows_f32`: `4,000 ns`

`large/trade22`:

- `cuStreamSynchronize`: `231,323,215 ns`, `96.6%` of traced CUDA API time
- dominant GPU kernel:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`
  - `209,936,376 ns` total across `3` launches
  - `69.98 ms` average
  - `95.3%` of traced GPU kernel time
- `gather_topk_pairs_tm_f32`: `6,016 ns`
- `pack_metric_rows_f32`: `5,058 ns`

`xlarge/full17`:

- `cuStreamSynchronize`: `360,892,803 ns`, `91.6%` of traced CUDA API time
- `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`:
  - `286,286,496 ns` total across `1` launch
  - `80.8%` of traced GPU kernel time
- `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`:
  - `53,393,307 ns` total across `6` launches
  - `8.899 ms` average
  - `15.1%` of traced GPU kernel time
- `alma_batch_tiled_f32_2x_tile256_tm`:
  - `14,500,037 ns` total across `6` launches
  - `4.1%` of traced GPU kernel time
- `gather_topk_pairs_tm_f32`: `1,696 ns`
- `pack_metric_rows_f32`: `1,344 ns`

`xlarge/trade22`:

- `cuStreamSynchronize`: `382,699,983 ns`, `92.0%` of traced CUDA API time
- `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`:
  - `311,003,298 ns` total across `1` launch
  - `82.5%` of traced GPU kernel time
- `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`:
  - `51,683,529 ns` total across `6` launches
  - `8.614 ms` average
  - `13.7%` of traced GPU kernel time
- `alma_batch_tiled_f32_2x_tile256_tm`:
  - `14,078,078 ns` total across `6` launches
  - `3.7%` of traced GPU kernel time
- `gather_topk_pairs_tm_f32`: `2,240 ns`
- `pack_metric_rows_f32`: `1,568 ns`

Interpretation:

- the `xlarge` GPU time story is still overwhelmingly a backtest-family story, not a reducer or pair-gather story
- the surfaced secondary GPU cost on shortlist-heavy `xlarge` is now the `windowed_m17` / `windowed_m22` replay family, not reducer kernels
- the heavy `cuStreamSynchronize` share remains a host wait on the same GPU work, not separate substantial host-side bandwidth work

## Nsight Compute Findings

Important caveat:

- the `xlarge` Nsight Compute captures used regex-filtered kernel profiling with many launches
- their `mean_gpu_time_ms` values are profiler-perturbed and should not be used as whole-route wall-time substitutes
- use Nsight Systems for absolute share and Nsight Compute for stall, occupancy, and access-shape signatures

### `large` lockstep kernels

`large/full17` lockstep:

- GPU time: `68.887 ms`
- SM throughput: `41.91%`
- compute-memory throughput: `13.02%`
- DRAM throughput: `1.41%`
- registers per thread: `55`
- max-warp ceiling: `75.0%`
- active warps: `29.66%`
- average warp latency per issued instruction: `8.49`
- average long-scoreboard stall per issue-active cycle: `5.02`
- global-load bytes per sector: `17.60 / 32 B`
- global-store bytes per sector: `5.67 / 32 B`
- no local or shared spilling

`large/trade22` lockstep:

- GPU time: `71.760 ms`
- SM throughput: `41.28%`
- compute-memory throughput: `12.82%`
- DRAM throughput: `1.48%`
- registers per thread: `64`
- max-warp ceiling: `66.67%`
- active warps: `29.64%`
- average warp latency per issued instruction: `8.59`
- average long-scoreboard stall per issue-active cycle: `5.16`
- global-load bytes per sector: `17.60 / 32 B`
- global-store bytes per sector: `10.35 / 32 B`
- no local or shared spilling

### `xlarge` lockstep and replay kernels

`xlarge/full17` lockstep:

- GPU time: `324.629 ms`
- SM throughput: `73.20%`
- compute-memory throughput: `29.00%`
- DRAM throughput: `29.00%`
- registers per thread: `55`
- max-warp ceiling: `75.0%`
- active warps: `66.19%`
- average warp latency per issued instruction: `10.40`
- average long-scoreboard stall per issue-active cycle: `4.31`
- global-load bytes per sector: `17.70 / 32 B`
- global-store bytes per sector: `5.67 / 32 B`
- no local or shared spilling

`xlarge/trade22` lockstep:

- GPU time: `337.047 ms`
- SM throughput: `71.44%`
- compute-memory throughput: `23.37%`
- DRAM throughput: `18.36%`
- registers per thread: `64`
- max-warp ceiling: `66.67%`
- active warps: `58.06%`
- average warp latency per issued instruction: `9.26`
- average long-scoreboard stall per issue-active cycle: `3.91`
- global-load bytes per sector: `17.70 / 32 B`
- global-store bytes per sector: `10.35 / 32 B`
- no local or shared spilling

`xlarge/full17` windowed replay:

- mean profiled launch time: `22.353 ms`
- registers per thread: `56`
- max-warp ceiling: `75.0%`
- active warps: `4.17%`
- average warp latency per issued instruction: `6.22`
- average long-scoreboard stall per issue-active cycle: `3.20`
- global-load bytes per sector: `12.31 / 32 B`
- global-store bytes per sector: `5.69 / 32 B`
- no spill story

`xlarge/trade22` windowed replay:

- mean profiled launch time: `22.305 ms`
- registers per thread: `62`
- max-warp ceiling: `66.67%`
- active warps: `4.17%`
- average warp latency per issued instruction: `5.98`
- average long-scoreboard stall per issue-active cycle: `3.10`
- global-load bytes per sector: `12.31 / 32 B`
- global-store bytes per sector: `10.35 / 32 B`
- no spill story

### `xlarge` reducer and packing kernels

`xlarge/full17`:

- `gather_topk_pairs_tm_f32`: `2.709 ms` mean profiled launch time
- `pack_metric_rows_f32`: `2.272 ms` mean profiled launch time
- both remain sparse, inefficient, and spill-free
- both stay tiny in whole-process Nsight Systems on the measured truth surface

`xlarge/trade22`:

- `gather_topk_pairs_tm_f32`: `3.509 ms` mean profiled launch time
- `pack_metric_rows_f32`: `2.827 ms` mean profiled launch time
- both remain sparse, inefficient, and spill-free
- both stay tiny in whole-process Nsight Systems on the measured truth surface

Interpretation:

- the retained hot kernels still do not show a spill-driven story
- the retained hot kernels still do not show a branch-divergence-first story
- `trade22` continues to preserve materially better store width than `full17`
- the current shortlist-heavy `xlarge` evidence does not justify reducer-first work as the next product move

## What The Bundle Now Supports

This bundle now supports these claims:

1. The live exact row-first route remains the canonical path on both `large` and shortlist-heavy `xlarge`.
2. The `large` exact surfaces are overwhelmingly dominated by the lockstep backtest kernels.
3. The shortlist-heavy `xlarge` GPU kernel mix is still dominated by the same lockstep families, with the shortlist-slice `windowed` replay family as a material but secondary cost.
4. The surfaced reducer and pack kernels are not materially large in absolute whole-process time on the measured truth surfaces.
5. The retained hot kernels remain spill-free and high-branch-uniformity.
6. The retained hot-kernel symptom remains source/access-shape and dependency pressure rather than a simple occupancy-only or branch-only story.
7. The shortlist-heavy `xlarge` route has a real `final_finalize` / `host_finalize` slice, and current source shows that this includes shortlist-slice rerank and replay work rather than only host bookkeeping.

## What The Bundle Does Not Yet Prove

This bundle still does not prove:

1. that score-first is permanently uncompetitive rather than still underdeveloped
2. that shortlist-slice replay/diagnostics should be rewritten before one more bounded lockstep-body pass is tried
3. that compact exact search-stage redesign is the best immediate product move on the measured truth surfaces
4. that fast-warp-dedup or other distribution-aware specialization is unimportant on a different workload mix
5. that any renewed PTX or SASS-level codegen intervention is justified on the retained hot kernels

## Stage B Decision

Stage B branch label: `bounded_lockstep_pass`

Reason:

1. the largest absolute hotspot on both `large` and `xlarge` remains the lockstep backtest family
2. the surfaced shortlist-heavy `xlarge` secondary cost is `windowed` shortlist-slice replay, not reducer/search-stage kernels
3. the reducer/search-stage kernels remain too small in whole-process time to justify promoting `compact_exact_search_stage` first
4. current score-first blockers and prior measured losses still justify keeping score-first deferred

Immediate implication:

- the next bounded product-facing implementation lane should remain concentrated in `crates/app_core/src/kernels/double_crossover.cu` around the retained lockstep body
- if that pass stalls, the next architecture candidate should be shortlist-slice replay and diagnostics efficiency in `crates/app_core/src/gpu_backtest_kernel.rs`, not a generic reducer-first rewrite

## Stage C Bounded Lockstep Pass

Retained implementation:

1. Keep a bounded source change in `crates/app_core/src/kernels/double_crossover.cu` that:
   - adds `ld_tm_row_base(...)`
   - hoists `fast_row_base` / `slow_row_base`
   - uses that row-base load path only on the retained `LOCKSTEP_TIME` exact-body branch inside `double_cross_backtest_tm_flex_f32_body`
2. No route-policy or score-first behavior change was required.

Same-tree direct-runner A/B:

1. Initial four-surface pass against a reverted same-tree build:
   - `large/full17`: `69.33 ms` patched vs `71.67 ms` reverted
   - shortlist-heavy `xlarge/full17`: `381.33 ms` patched vs `409.00 ms` reverted
   - `large/trade22`: first 3-iteration pass was noisy
   - shortlist-heavy `xlarge/trade22`: `409.00 ms` patched vs `414.00 ms` reverted
2. Targeted 7-iteration recheck on the ambiguous `trade22` surfaces:
   - `large/trade22`: `70.00 ms` patched vs `71.86 ms` reverted
   - shortlist-heavy `xlarge/trade22`: `390.29 ms` patched vs `387.29 ms` reverted
3. Interpretation:
   - `full17` improvement is clear
   - shortlist-heavy `trade22` remains mixed, but the observed swing is small relative to the full17 gain and did not justify dropping the change before running the authoritative gate surfaces

Authoritative gate result:

1. `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed and wrote `target/gpu_perf_summary.json`.
2. `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` passed and wrote `target/gpu_alma_contract_rows_perf_summary.json`.
3. The specialized ALMA contract-row gate stayed green on:
   - `auto`
   - `full17`
   - `trade22`

Post-change static refresh:

1. Narrow cuobjdump refresh captured:
   - `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
   - `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm`
2. The refreshed artifacts are:
   - `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt`
   - `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_symbols_2026-04-03.txt`

Stage C decision:

1. Keep the bounded lockstep row-base addressing change.
2. Treat the retained lockstep-body pass as complete for this branch.
3. The next product-facing follow-on, if another pass is needed, should move to shortlist-slice replay and diagnostics efficiency rather than reducer-first work or a broad static SASS reopening.

Planned next measurement-and-intervention branch:

1. Instrument `crates/app_core/src/gpu_backtest_kernel.rs` so `final_finalize` is split into shortlist-row materialization, replay evaluation, slice-record computation, rerank/sort, and diagnostics assembly.
2. Re-run shortlist-heavy `xlarge` `full17` and `trade22` with `shortlist_slice_mode=none` vs `halves` before changing route behavior.
3. Use that finer breakdown to decide whether the next retained change should target:
   - row materialization
   - replay setup / replay evaluation
   - rerank / sort logic
   - diagnostics-only work
4. Keep `compact_exact_search_stage` deferred unless the shortlist-slice branch proves that the local replay/finalization path is exhausted.
