# `double_crossover` Phase 2 Stage C Bounded Lockstep Pass Log

Date: 2026-04-03

## Purpose

This note records the exact commands, retained artifacts, and outcome for the Stage C bounded lockstep-body pass selected from the Phase 2 runtime profiling bundle.

## Retained Code Change

File:

- `crates/app_core/src/kernels/double_crossover.cu`

Retained change:

1. Add `ld_tm_row_base(...)`.
2. Hoist `fast_row_base` / `slow_row_base` inside `double_cross_backtest_tm_flex_f32_body`.
3. Use the row-base load path only on the retained `LOCKSTEP_TIME` exact-body branch.

## Shared Runner Environment

All direct-runner reruns used:

- `VECTORBT_KERNEL_FAST_PATH=1`
- `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL=off`
- `VECTORBT_KERNEL_USE_CUDA_GRAPHS=off`
- binary: `target/release/profile_gpu_dynamic_runner.exe`

## Build Commands

Patched or reverted local A/B builds:

```powershell
cargo build -p ta_app_core --release --features cuda-backtest-kernel --bin profile_gpu_dynamic_runner
```

## Direct-Runner Commands

Initial four-surface patched runs:

```powershell
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload large --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode none --include-trade-metrics off --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload large --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode none --include-trade-metrics on --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode halves --include-trade-metrics off --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode halves --include-trade-metrics on --emit-json
```

Matching reverted same-tree runs:

```powershell
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload large --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode none --include-trade-metrics off --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload large --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode none --include-trade-metrics on --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode halves --include-trade-metrics off --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 3 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode halves --include-trade-metrics on --emit-json
```

Targeted 7-iteration trade22 recheck:

```powershell
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload large --iterations 7 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode none --include-trade-metrics on --emit-json
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 7 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode halves --include-trade-metrics on --emit-json
```

## Gate Commands

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate
```

## Narrow SASS Refresh Commands

Current cubin:

- `target/release/build/ta_app_core-143e881f0fcbd810/out/double_crossover.sm_89.cubin`

CUDA toolchain used for the refresh:

- `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0\bin\cuobjdump.exe`
- `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.0\bin\nvdisasm.exe`

Commands:

```powershell
cuobjdump --list-text target/release/build/ta_app_core-143e881f0fcbd810/out/double_crossover.sm_89.cubin
cuobjdump --dump-sass --function double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm,double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm target/release/build/ta_app_core-143e881f0fcbd810/out/double_crossover.sm_89.cubin
```

## Saved Artifacts

Direct-runner JSONL outputs:

- `target/double_crossover_stagec_large_full17_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_full17_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_full17_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_full17_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_reverted_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_patchcheck_2026-04-03.jsonl`
- `target/double_crossover_stagec_large_trade22_rowbase_revertcheck_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_patchcheck_2026-04-03.jsonl`
- `target/double_crossover_stagec_xlarge_trade22_rowbase_revertcheck_2026-04-03.jsonl`

Gate outputs:

- `target/gpu_perf_summary.json`
- `target/gpu_alma_contract_rows_perf_summary.json`
- `target/gpu_alma_contract_rows_perf/auto.json`
- `target/gpu_alma_contract_rows_perf/full17.json`
- `target/gpu_alma_contract_rows_perf/trade22.json`

Narrow SASS refresh:

- `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_2026-04-03.sass.txt`
- `SASS research/double_crossover_sm89_cuobjdump_stagec_lockstep_rowbase_symbols_2026-04-03.txt`

## Outcome

1. The bounded lockstep row-base change is retained.
2. Same-tree direct-runner evidence is clearly positive on `full17` and mixed but not disqualifying on shortlist-heavy `trade22`.
3. The authoritative GPU perf gates stayed green.
4. The next product-facing follow-on, if another pass is pursued, should move to shortlist-slice replay and diagnostics efficiency rather than reducer-first work or a broad SASS reopening.
