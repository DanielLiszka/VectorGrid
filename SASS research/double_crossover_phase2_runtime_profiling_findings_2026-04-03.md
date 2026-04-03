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

- mean imported-details duration: `16.071 ms` across `18` profiled invocations
- block/grid shape: block `(64, 1, 1)`, grid `(1, 1, 1)`, waves per SM `0`
- registers per thread: `56`
- theoretical occupancy: `75.00%`
- achieved occupancy mean: `2.78%`
- achieved active warps per SM mean: `1.33`
- average warp cycles per issued instruction: `4.69`
- average scoreboard stall cycles: `2.23`
- active threads per warp mean: `13.66`
- predicated-on threads per warp mean: `11.95`
- global-load bytes per sector: `8.83 / 32 B`
- global-store bytes per sector: `5.70 / 32 B`
- no local or shared spilling
- two replay populations are visible in the imported-details summary:
  - `12` shorter launches with `1` achieved active warp per SM and `7.1 / 32 B` global-load sectors
  - `6` longer launches with `2` achieved active warps per SM and `12.3 / 32 B` global-load sectors

`xlarge/trade22` windowed replay:

- mean imported-details duration: `15.592 ms` across `18` profiled invocations
- block/grid shape: block `(64, 1, 1)`, grid `(1, 1, 1)`, waves per SM `0`
- registers per thread: `62`
- theoretical occupancy: `66.67%`
- achieved occupancy mean: `2.78%`
- achieved active warps per SM mean: `1.33`
- average warp cycles per issued instruction: `4.41`
- average scoreboard stall cycles: `2.10`
- active threads per warp mean: `13.66`
- predicated-on threads per warp mean: `11.84`
- global-load bytes per sector: `8.83 / 32 B`
- global-store bytes per sector: `10.40 / 32 B`
- no local or shared spilling
- two replay populations are visible in the imported-details summary:
  - `12` shorter launches with `1` achieved active warp per SM and `7.1 / 32 B` global-load sectors
  - `6` longer launches with `2` achieved active warps per SM and `12.3 / 32 B` global-load sectors

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
- the replay family is structurally a repeated tiny-launch problem, not a large-grid throughput problem

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
3. The immediate follow-on after Stage C was the shortlist-slice replay / diagnostics attribution branch.

Completed shortlist-slice attribution branch:

1. `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, and `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs` now surface:
   - `host_shortlist_materialize_ms`
   - `host_finalize_shortlist_slice_replay_ms`
   - `host_finalize_shortlist_slice_record_assembly_ms`
   - `host_finalize_shortlist_slice_sort_ms`
   - `host_finalize_shortlist_slice_diagnostics_ms`
2. Shortlist-heavy `xlarge`, `alma/alma`, `synth` reruns were completed for:
   - `full17`, `shortlist_slice_mode=none`
   - `full17`, `shortlist_slice_mode=halves`
   - `full17`, `shortlist_slice_mode=quartiles`
   - `trade22`, `shortlist_slice_mode=none`
   - `trade22`, `shortlist_slice_mode=halves`
   - `trade22`, `shortlist_slice_mode=quartiles`
3. Measured direct-runner deltas:
   - `full17`: `297.40 ms` with `none` vs `379.20 ms` with `halves` vs `986.80 ms` with serialized `quartiles`
   - `trade22`: `319.60 ms` with `none` vs `396.20 ms` with `halves` vs `599.00 ms` with serialized `quartiles`
4. The runner-visible shortlist sub-buckets show that the surfaced shortlist cost is replay:
   - `full17` `halves`: `host_finalize_shortlist_slice_ms=60.40`, replay `=60.40`
   - `full17` `quartiles`: `host_finalize_shortlist_slice_ms=164.00`, replay `=163.80`
   - `trade22` `halves`: `host_finalize_shortlist_slice_ms=58.40`, replay `=58.40`
   - `trade22` `quartiles`: `host_finalize_shortlist_slice_ms=132.40`, replay `=132.40`
5. Corrected high-resolution pipeline logs with `VECTORBT_KERNEL_PROFILE_PIPELINE=1` show the same composition result even though the profiler perturbs absolute wall time:
   - `full17` `halves`: replay about `215.507` to `224.230 ms`, record assembly about `0.036` to `0.043 ms`, sort about `0.004` to `0.012 ms`, diagnostics about `0.003 ms`
   - `full17` `quartiles`: replay about `209.326` to `225.585 ms`, record assembly about `0.038` to `0.044 ms`, sort about `0.004` to `0.013 ms`, diagnostics about `0.002` to `0.003 ms`
6. Record assembly, sort, diagnostics, and shortlist-row materialization stayed negligible on this branch.
7. One focused post-instrumentation Nsight Systems rerun on `xlarge/full17/halves` tied the host-side slice bucket back to the GPU trace:
   - `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
   - `51.595 ms` total across `6` instances
   - `14.3%` of traced GPU kernel time in `target/nsight/double_crossover_phase2_shortlist_full17_halves_post_instrumentation_stats.txt`
8. Verification after retaining the instrumentation passed:
   - `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
   - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
9. The branch log is `SASS research/double_crossover_phase2_shortlist_slice_attribution_log_2026-04-03.md`.
10. A focused replay-kernel follow-on is now complete and is recorded in `SASS research/double_crossover_phase2_windowed_replay_kernel_study_2026-04-03.md`.
11. Targeted Nsight Compute on the retained replay family shows that both exact contracts are currently executing tiny `windowed` kernels rather than large, throughput-heavy replay launches:
   - `full17`: `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
     - average duration about `16.071 ms`
     - block `(64, 1, 1)`, grid `(1, 1, 1)`
     - `56` registers per thread
     - theoretical occupancy `75.00%`, achieved occupancy `2.78%`
   - `trade22`: `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`
     - average duration about `15.592 ms`
     - block `(64, 1, 1)`, grid `(1, 1, 1)`
     - `62` registers per thread
     - theoretical occupancy `66.67%`, achieved occupancy `2.78%`
12. Live dispatch logs tie that profiler result back to the shortlist rerank path:
   - both `full17` and `trade22` currently execute `6` original `windowed` launches under `shortlist_slice_mode=halves`
   - launch shape is `2x50` shortlist rows plus `4x8` diagnostic rows
13. Current source explains the `4x8` tail:
   - `rerank_shortlist_by_slice_robustness(...)` calls `compute_shortlist_slice_mode_records(..., Halves, ...)`
   - then still calls `compute_shortlist_slice_mode_records(..., Quartiles, ...)` for the diagnostic subset even when the active ranking mode is only `halves`
14. A bounded replay-batching follow-on was then implemented and retained around the replay `ranges.iter().enumerate()` loop in `eval_double_ma_batch_gpu_kernel_impl(...)`:
   - `crates/app_core/src/gpu_backtest_kernel.rs` now batches multi-range shortlist replay rows before the `windowed` launch and uploads per-row eval-start / eval-end arrays only on the shortlist replay path
   - `crates/app_core/src/kernels/double_crossover.cu` now accepts optional per-row replay ranges on the `windowed` wrappers through a launch-time dispatch helper that keeps non-windowed hot kernels on the old compile-time path
   - exact `full17` / `trade22` row authority and current shortlist diagnostics semantics were preserved
15. Post-implementation shortlist-heavy `xlarge`, `alma/alma`, `synth` reruns show that the retained replay fan-out reduction materially lowered shortlist replay cost:
   - `full17`: `296.00 ms` with `none` vs `342.80 ms` with `halves` vs `349.00 ms` with `quartiles`
   - `trade22`: `308.20 ms` with `none` vs `359.00 ms` with `halves` vs `365.00 ms` with `quartiles`
16. The runner-visible shortlist sub-buckets collapsed to the same replay-scale on both ranking and diagnostics modes:
   - `full17` `halves`: shortlist slice `27.80 ms`, replay `27.80 ms`
   - `full17` `quartiles`: shortlist slice `28.00 ms`, replay `27.80 ms`
   - `trade22` `halves`: shortlist slice `28.20 ms`, replay `28.20 ms`
   - `trade22` `quartiles`: shortlist slice `27.20 ms`, replay `27.20 ms`
17. Live dispatch logs confirm that the retained implementation replaced the old `6`-launch replay fan-out with one launch per replay mode:
   - `batched_windowed_ranges=true range_count=2 launch_pairs=100 dispatch=windowed_m17`
   - `batched_windowed_ranges=true range_count=4 launch_pairs=32 dispatch=windowed_m17`
18. Verification after retaining the replay-batching prototype passed:
   - targeted tests:
     - `prepare_batched_windowed_exact_replay_rows_preserves_range_major_order`
     - `gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics`
     - `gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics`
   - `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
   - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
19. The implementation log is `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`.

Branch conclusion:

1. Keep the new shortlist-slice timing instrumentation.
2. Keep the bounded replay-batching prototype. It preserves exact row authority and shortlist diagnostics semantics while materially reducing shortlist replay fan-out.
3. The retained replay cost is no longer a local host-side cleanup problem, and the obvious next gain is no longer another tiny-`windowed` micro-pass.
4. `quartiles` remains diagnostic-only replay expansion, but after batching it no longer produces the prior runtime blow-up.
5. Keep `compact_exact_search_stage` deferred for product work unless broader replay/dataflow architecture work is explicitly reopened.
6. If more work is still desired after this retained prototype, it is a larger replay/dataflow architecture study rather than another local finalize cleanup loop.
