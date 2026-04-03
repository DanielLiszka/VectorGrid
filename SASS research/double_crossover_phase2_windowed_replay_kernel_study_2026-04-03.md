# `double_crossover` Phase 2 Windowed Replay Kernel Study

Date: 2026-04-03

## Purpose

Record the targeted replay-kernel follow-on that came after the shortlist-slice attribution branch.

This note answers a narrower question than the earlier shortlist timing pass:

1. Are the retained `windowed_m17` / `windowed_m22` replay kernels large enough that more kernel-local tuning is still justified?
2. Or is the remaining shortlist cost now primarily a replay-shape / batching problem?

This study directly led to a retained same-day replay-batching follow-on. The retained implementation is recorded in `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`.

## Commands Run

Targeted Nsight Compute capture, `full17`, shortlist-heavy `xlarge`, `halves`:

```powershell
& .\scripts\profile_gpu_backtest_ncu.ps1 `
  -Target runner `
  -RunnerStrategy double_ma `
  -RunnerFastMa alma `
  -RunnerSlowMa alma `
  -RunnerAdxMode off `
  -RunnerWorkload xlarge `
  -RunnerCandleShape synth `
  -RunnerMetricContract full17 `
  -RunnerShortlistSliceMode halves `
  -RunnerIncludeTradeMetrics off `
  -RunnerWarmupIterations 2 `
  -RunnerIterations 1 `
  -RunnerHeatmapBins 0 `
  -RunnerDeviceId 0 `
  -DynamicStrategyMode off `
  -CudaGraphsMode off `
  -UseKernelNameFilter $true `
  -KernelName 'regex:^double_cross_backtest_tm_flex_f32_windowed_m17' `
  -LaunchCount 200 `
  -OutputPrefix 'target/nsight/double_crossover_phase2_windowed_replay_full17_halves_post_stagec'
```

Targeted Nsight Compute capture, `trade22`, shortlist-heavy `xlarge`, `halves`:

```powershell
& .\scripts\profile_gpu_backtest_ncu.ps1 `
  -Target runner `
  -RunnerStrategy double_ma `
  -RunnerFastMa alma `
  -RunnerSlowMa alma `
  -RunnerAdxMode off `
  -RunnerWorkload xlarge `
  -RunnerCandleShape synth `
  -RunnerMetricContract trade22 `
  -RunnerShortlistSliceMode halves `
  -RunnerIncludeTradeMetrics on `
  -RunnerWarmupIterations 2 `
  -RunnerIterations 1 `
  -RunnerHeatmapBins 0 `
  -RunnerDeviceId 0 `
  -DynamicStrategyMode off `
  -CudaGraphsMode off `
  -UseKernelNameFilter $true `
  -KernelName 'regex:^double_cross_backtest_tm_flex_f32_windowed_m22' `
  -LaunchCount 200 `
  -OutputPrefix 'target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_post_stagec'
```

Replay dispatch correlation, `full17`:

```powershell
$env:VECTORBT_KERNEL_LOG_DISPATCH='1'
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 1 `
  --warmup-iterations 0 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract full17 `
  --shortlist-slice-mode halves `
  --include-trade-metrics off `
  2> target/nsight/double_crossover_phase2_windowed_replay_full17_halves_dispatch.log `
  | Out-File -Encoding utf8 target/nsight/double_crossover_phase2_windowed_replay_full17_halves_dispatch.jsonl
Remove-Item Env:VECTORBT_KERNEL_LOG_DISPATCH
```

Replay dispatch correlation, `trade22`:

```powershell
$env:VECTORBT_KERNEL_LOG_DISPATCH='1'
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 1 `
  --warmup-iterations 0 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract trade22 `
  --shortlist-slice-mode halves `
  --include-trade-metrics on `
  2> target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_dispatch.log `
  | Out-File -Encoding utf8 target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_dispatch.jsonl
Remove-Item Env:VECTORBT_KERNEL_LOG_DISPATCH
```

## Artifacts

Primary replay artifacts:

- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_post_stagec.ncu-rep`
- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_post_stagec.provenance.json`
- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_post_stagec_raw.csv`
- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_post_stagec_details.csv`
- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_post_stagec_details.txt`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_post_stagec.ncu-rep`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_post_stagec.provenance.json`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_post_stagec_raw.csv`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_post_stagec_details.csv`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_post_stagec_details.txt`
- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_dispatch.log`
- `target/nsight/double_crossover_phase2_windowed_replay_full17_halves_dispatch.jsonl`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_dispatch.log`
- `target/nsight/double_crossover_phase2_windowed_replay_trade22_halves_dispatch.jsonl`

Derived summaries:

- `target/nsight/double_crossover_phase2_windowed_replay_post_stagec_details_summary.json`
- `target/nsight/double_crossover_phase2_windowed_replay_post_stagec_details_summary.txt`
- `target/nsight/double_crossover_phase2_windowed_replay_post_stagec_summary.json`
- `target/nsight/double_crossover_phase2_windowed_replay_post_stagec_summary.txt`
- `target/nsight/double_crossover_phase2_windowed_replay_summary_2026-04-03.json`
- `target/nsight/double_crossover_phase2_windowed_replay_summary_2026-04-03.txt`

## Code-Path Mapping

The shortlist replay path remains:

1. `rerank_shortlist_by_slice_robustness(...)` in `crates/app_core/src/gpu_backtest_kernel.rs`
2. `compute_shortlist_slice_mode_records(...)` in `crates/app_core/src/gpu_backtest_kernel.rs`
3. `eval_double_ma_batch_gpu_kernel_impl(...)` with `ranges` populated for replay
4. `use_windowed_eval = ...` inside the exact GPU batch path
5. `windowed_m17` / `windowed_m22` dispatch selection when the fixed metric contracts are active

The replay-shape detail that matters is that `compute_shortlist_slice_mode_records(...)` already batches candidate params for a requested mode into one call, but `eval_double_ma_batch_gpu_kernel_impl(...)` still executes one `windowed` launch per replay range inside the `for (range_idx, &(eval_start_t, eval_end_t_raw)) in ranges.iter().enumerate()` loop.

For `halves`, `rerank_shortlist_by_slice_robustness(...)` also computes a quartiles diagnostic view for the top diagnostic rows even though the ranking mode is still `halves`. That shows up as:

- `compute_shortlist_slice_mode_records(..., ShortlistSliceMode::Halves, ...)`
- then `compute_shortlist_slice_mode_records(..., ShortlistSliceMode::Quartiles, ...)` for the diagnostic subset

So the live replay family under `halves` is not only the ranking replay. It is the ranking replay plus diagnostic quartile replay.

## Measured Replay Summary

`full17`, `windowed_m17`, `xlarge`, `halves`:

- kernel: `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
- profiled invocations: `18`
- mean imported-details duration: `16.071 ms`
- block size: `(64, 1, 1)`
- grid size: `(1, 1, 1)`
- waves per SM: `0`
- registers per thread: `56`
- theoretical occupancy: `75.00%`
- achieved occupancy mean: `2.78%`
- achieved active warps per SM mean: `1.33`
- warp cycles per issued instruction mean: `4.69`
- scoreboard cycles mean: `2.23`
- active threads per warp mean: `13.66`
- predicated-on threads per warp mean: `11.95`
- global-load bytes per sector mean: `8.83 / 32 B`
- global-store bytes per sector mean: `5.70 / 32 B`
- no local or shared spilling
- launch populations:
  - `12` shorter launches with `1` achieved active warp per SM, scoreboard `1.7`, and global-load sectors `7.1 / 32 B`
  - `6` longer launches with `2` achieved active warps per SM, scoreboard `3.3`, and global-load sectors `12.3 / 32 B`

`trade22`, `windowed_m22`, `xlarge`, `halves`:

- kernel: `double_cross_backtest_tm_flex_f32_windowed_m22_regime_off_norisk_eps0_nocomm`
- profiled invocations: `18`
- mean imported-details duration: `15.592 ms`
- block size: `(64, 1, 1)`
- grid size: `(1, 1, 1)`
- waves per SM: `0`
- registers per thread: `62`
- theoretical occupancy: `66.67%`
- achieved occupancy mean: `2.78%`
- achieved active warps per SM mean: `1.33`
- warp cycles per issued instruction mean: `4.41`
- scoreboard cycles mean: `2.10`
- active threads per warp mean: `13.66`
- predicated-on threads per warp mean: `11.84`
- global-load bytes per sector mean: `8.83 / 32 B`
- global-store bytes per sector mean: `10.40 / 32 B`
- no local or shared spilling
- launch populations:
  - `12` shorter launches with `1` achieved active warp per SM, scoreboard `1.6`, and global-load sectors `7.1 / 32 B`
  - `6` longer launches with `2` achieved active warps per SM, scoreboard `3.1`, and global-load sectors `12.3 / 32 B`

## Dispatch Correlation

The dispatch logs show the same replay shape on both contracts:

- main exact path:
  - `launch_pairs=370750`
  - `dispatch=lockstep_m17` or `dispatch=lockstep_m22`
- shortlist replay path under `halves`:
  - `2` `windowed` launches with `launch_pairs=50`
  - `4` `windowed` launches with `launch_pairs=8`

That replay shape matches the current source:

- the `2x50` launches are the active `halves` slices for the current shortlist rows
- the `4x8` launches are the quartiles diagnostic reruns for the diagnostic-row subset

The targeted Nsight Compute imports show `18` profiled invocations because multiple replay pass groups are represented in the imported report. The dispatch logs show the original request shape more clearly: `6` `windowed` replay launches per measured `halves` request.

## Valid Interpretation

What this study supports:

1. The retained shortlist replay cost is not a big, throughput-heavy GPU kernel.
2. It is a repeated tiny-launch problem with `grid=(1,1,1)` and only `1` to `2` achieved active warps per SM.
3. Another kernel-local micro-pass on `windowed_m17` / `windowed_m22` is unlikely to produce a meaningful product win by itself.
4. The next meaningful replay-side gain, if pursued, is a replay-shape / batching / dataflow change.
5. The current `halves` path includes extra replay work for quartiles diagnostics, so any future architecture change must account for both the product rerank and the retained diagnostic payload.

What this study does not support:

1. Removing shortlist diagnostics to save time.
2. Dropping quartiles diagnostics semantics silently.
3. Reopening blanket SASS work on the replay kernels.
4. Promoting `compact_exact_search_stage` ahead of a narrower replay-side architecture study.

## Next Bounded Architecture Candidate

This study's next bounded candidate was implemented later the same day. The retained prototype:

1. keeps exact finalist authority unchanged
2. keeps `full17` / `trade22` row semantics unchanged
3. keeps shortlist diagnostics semantics unchanged
4. batches multi-range replay so the old `2x50` plus `4x8` `windowed` launches collapse to one launch per replay mode on the retained path

Implementation outcome:

1. `crates/app_core/src/gpu_backtest_kernel.rs` now batches shortlist replay rows before the `windowed` launch and uploads per-row eval-start / eval-end arrays only on that replay path.
2. `crates/app_core/src/kernels/double_crossover.cu` now accepts optional per-row replay ranges on the `windowed` wrappers through a launch-time dispatch helper that keeps non-windowed hot kernels on the old compile-time path.
3. Direct-runner reruns on shortlist-heavy `xlarge`, `alma/alma`, `synth` show the retained replay reduction:
   - `full17`: `296.00 ms` with `none`, `342.80 ms` with `halves`, `349.00 ms` with `quartiles`
   - `trade22`: `308.20 ms` with `none`, `359.00 ms` with `halves`, `365.00 ms` with `quartiles`
4. The runner-visible shortlist bucket collapsed to about `28 ms` replay on both retained contracts and both replay modes.
5. The saved `xlarge/full17/halves` dispatch log confirms the retained launch shape:
   - `batched_windowed_ranges=true range_count=2 launch_pairs=100`
   - `batched_windowed_ranges=true range_count=4 launch_pairs=32`
6. Verification after retaining the prototype passed:
   - targeted replay-path tests for range-major ordering and `trade22` tail-metric preservation
   - `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
   - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`

If more performance work is still desired after the retained prototype, the next bounded candidate should:

1. keep exact finalist authority unchanged
2. keep `full17` / `trade22` row semantics unchanged
3. keep shortlist diagnostics semantics unchanged
4. move beyond the first retained multi-range replay batching step and only then consider broader replay/dataflow changes

That likely means a focused architecture study around:

- `compute_shortlist_slice_mode_records(...)`
- `eval_double_ma_batch_gpu_kernel_impl(...)`
- the `for ... in ranges.iter().enumerate()` replay launch loop in `crates/app_core/src/gpu_backtest_kernel.rs`
- and, only if needed, the `windowed` CUDA kernels in `crates/app_core/src/kernels/double_crossover.cu`

The intended question is not "can the current tiny windowed kernel be hand-tuned further?"

The intended question is:

"Can replay work across slice ranges and shortlist rows be packed into fewer, denser launches while preserving current result and diagnostics semantics?"

## Decision

1. Keep all retained instrumentation and shortlist behavior unchanged.
2. Keep the retained replay-batching implementation recorded in `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`.
3. Do not pursue another replay-kernel micro-optimization from this study.
4. Treat any further replay work, if reopened, as a larger replay/dataflow architecture study beyond the first retained batching step.
