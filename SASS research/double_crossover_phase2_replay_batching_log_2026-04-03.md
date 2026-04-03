# `double_crossover` Phase 2 Replay Batching Log

Date: 2026-04-03

## Purpose

Record the retained replay-batching implementation that followed the shortlist-slice attribution pass and the replay-kernel study.

This branch keeps:

1. exact finalist authority unchanged
2. `full17` / `trade22` row semantics unchanged
3. shortlist diagnostics semantics unchanged

## Scope

Touched implementation surfaces:

1. `crates/app_core/src/gpu_backtest_kernel.rs`
2. `crates/app_core/src/kernels/double_crossover.cu`

Supporting validation and instrumentation surfaces already in place:

1. `crates/app_core/src/double_ma.rs`
2. `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

## Hypothesis

The shortlist-heavy replay cost was no longer a host-side finalize problem and no longer a large throughput-heavy GPU kernel problem.

The retained evidence before this change showed:

1. replay dominated the surfaced shortlist-slice cost
2. `windowed_m17` / `windowed_m22` were tiny `grid=(1,1,1)` launches
3. the live `halves` route was still issuing `6` replay launches per request:
   - `2x50` shortlist rows
   - `4x8` quartiles diagnostics rows

So the bounded next question was:

"Can multi-range replay be packed into fewer launches without changing current row or diagnostics semantics?"

## Implementation

Retained implementation shape:

1. `gpu_backtest_kernel.rs`
   - adds batched replay-row preparation for exact `windowed` replay
   - uploads per-row `eval_start` / `eval_end` arrays only on the replay path
   - replaces one launch-per-range replay with one launch-per-replay-mode when ranges are present
2. `double_crossover.cu`
   - `windowed` wrappers now accept optional per-row replay-range pointers
   - launch-time dispatch selects the compact replay-range variant only when those pointers are present
   - non-windowed hot kernels stay on the old compile-time path

The retained dispatch shape under `shortlist_slice_mode=halves` is now:

1. one batched ranking replay launch:
   - `batched_windowed_ranges=true range_count=2 launch_pairs=100`
2. one batched diagnostics replay launch:
   - `batched_windowed_ranges=true range_count=4 launch_pairs=32`

## Commands Run

Targeted shortlist-heavy reruns:

```powershell
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 5 `
  --warmup-iterations 2 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract full17 `
  --shortlist-slice-mode none `
  --include-trade-metrics off `
  --emit-json > target/double_crossover_phase2_replay_batching_xlarge_full17_none_2026-04-03.jsonl
```

```powershell
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 5 `
  --warmup-iterations 2 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract full17 `
  --shortlist-slice-mode halves `
  --include-trade-metrics off `
  --emit-json > target/double_crossover_phase2_replay_batching_xlarge_full17_halves_2026-04-03.jsonl
```

```powershell
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 5 `
  --warmup-iterations 2 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract full17 `
  --shortlist-slice-mode quartiles `
  --include-trade-metrics off `
  --emit-json > target/double_crossover_phase2_replay_batching_xlarge_full17_quartiles_2026-04-03.jsonl
```

```powershell
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 5 `
  --warmup-iterations 2 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract trade22 `
  --shortlist-slice-mode none `
  --emit-json > target/double_crossover_phase2_replay_batching_xlarge_trade22_none_2026-04-03.jsonl
```

```powershell
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 5 `
  --warmup-iterations 2 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract trade22 `
  --shortlist-slice-mode halves `
  --emit-json > target/double_crossover_phase2_replay_batching_xlarge_trade22_halves_2026-04-03.jsonl
```

```powershell
.\target\release\profile_gpu_dynamic_runner.exe `
  --strategy double_ma `
  --fast-ma alma `
  --slow-ma alma `
  --adx-mode off `
  --workload xlarge `
  --candle-shape synth `
  --iterations 5 `
  --warmup-iterations 2 `
  --heatmap-bins 0 `
  --device-id 0 `
  --dynamic-mode off `
  --cuda-graphs off `
  --metric-contract trade22 `
  --shortlist-slice-mode quartiles `
  --emit-json > target/double_crossover_phase2_replay_batching_xlarge_trade22_quartiles_2026-04-03.jsonl
```

Dispatch confirmation:

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
  2> target/double_crossover_phase2_replay_batching_dispatch_full17_halves_2026-04-03.log `
  | Out-File -Encoding utf8 target/double_crossover_phase2_replay_batching_dispatch_full17_halves_2026-04-03.jsonl
Remove-Item Env:VECTORBT_KERNEL_LOG_DISPATCH
```

Verification:

```powershell
cargo test -p ta_app_core prepare_batched_windowed_exact_replay_rows --lib --release --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics --lib --release --features cuda,cuda-backtest-kernel -- --nocapture
cargo test -p ta_app_core gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics --lib --release --features cuda,cuda-backtest-kernel -- --nocapture
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate
```

## Artifacts

Primary retained artifacts:

1. `target/double_crossover_phase2_replay_batching_xlarge_full17_none_2026-04-03.jsonl`
2. `target/double_crossover_phase2_replay_batching_xlarge_full17_halves_2026-04-03.jsonl`
3. `target/double_crossover_phase2_replay_batching_xlarge_full17_quartiles_2026-04-03.jsonl`
4. `target/double_crossover_phase2_replay_batching_xlarge_trade22_none_2026-04-03.jsonl`
5. `target/double_crossover_phase2_replay_batching_xlarge_trade22_halves_2026-04-03.jsonl`
6. `target/double_crossover_phase2_replay_batching_xlarge_trade22_quartiles_2026-04-03.jsonl`
7. `target/double_crossover_phase2_replay_batching_dispatch_full17_halves_2026-04-03.log`
8. `target/double_crossover_phase2_replay_batching_dispatch_full17_halves_2026-04-03.jsonl`
9. `target/gpu_perf_summary.json`
10. `target/gpu_alma_contract_rows_perf_summary.json`

## Results

Direct-runner means after the retained prototype:

1. `full17`
   - `none`: `296.00 ms`
   - `halves`: `342.80 ms`
   - `quartiles`: `349.00 ms`
2. `trade22`
   - `none`: `308.20 ms`
   - `halves`: `359.00 ms`
   - `quartiles`: `365.00 ms`

Runner-visible shortlist buckets after the retained prototype:

1. `full17`
   - `halves`: shortlist slice `27.80 ms`, replay `27.80 ms`
   - `quartiles`: shortlist slice `28.00 ms`, replay `27.80 ms`
2. `trade22`
   - `halves`: shortlist slice `28.20 ms`, replay `28.20 ms`
   - `quartiles`: shortlist slice `27.20 ms`, replay `27.20 ms`

Interpretation:

1. The retained prototype collapses the replay bucket from the earlier `~58` to `60 ms` class down to about `28 ms`.
2. `quartiles` no longer explodes runtime because replay is now batched once per replay mode rather than once per range.
3. The replay cost is still real, but the first meaningful replay-shape reduction has now been captured without changing product semantics.

## Verification

Targeted tests passed:

1. `prepare_batched_windowed_exact_replay_rows_preserves_range_major_order`
2. `gpu_kernel_shortlist_slice_trade22_metric_contract_preserves_trade_tail_metrics`
3. `gpu_kernel_regime_hybrid_trade22_multi_range_eval_preserves_trade_tail_metrics`

General GPU perf gate passed:

1. `gpu_sma_small`: `69.396292 ms`
2. `gpu_alma_large`: `71.055036 ms`
3. dynamic strategy ratio: `0.973881`
4. static-off latency: `67 ms`

ALMA contract-row GPU perf gate passed:

1. `auto`: `67.329105 ms`
2. `full17`: `69.388547 ms`
3. `trade22`: `67.941178 ms`

## Decision

1. Keep the retained replay-batching implementation.
2. Keep the shortlist instrumentation and diagnostics semantics unchanged.
3. Do not reopen local finalize cleanup or another kernel-local `windowed` micro-pass from this branch.
4. If more performance work is still desired, treat the next step as a larger replay/dataflow architecture study beyond this first retained batching step.
