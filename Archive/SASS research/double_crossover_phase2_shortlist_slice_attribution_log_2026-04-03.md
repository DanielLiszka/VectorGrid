# `double_crossover` Phase 2 Shortlist-Slice Attribution Log

Date: 2026-04-03

## Purpose

This note records the execution, artifacts, and outcome for the post-Stage C shortlist-slice attribution branch.

The goal of this branch was:

1. instrument shortlist-slice replay/finalization more finely,
2. compare shortlist-heavy `xlarge` truth surfaces with `shortlist_slice_mode=none` vs `shortlist_slice_mode=halves`,
3. decide whether there was a safe feature-preserving local optimization to retain before any larger architecture work.

## Retained Code Changes

Files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Retained change set:

1. Add runner-visible shortlist timing fields:
   - `host_shortlist_materialize_ms`
   - `host_finalize_shortlist_slice_replay_ms`
   - `host_finalize_shortlist_slice_record_assembly_ms`
   - `host_finalize_shortlist_slice_sort_ms`
   - `host_finalize_shortlist_slice_diagnostics_ms`
2. Split shortlist-slice timing inside `gpu_backtest_kernel.rs` so replay, record assembly, sort, and diagnostics are tracked separately.
3. Extend the built-in pipeline profile output so shortlist-slice sub-buckets are visible in the stderr logs.

No behavior-changing shortlist-slice optimization was retained from this branch.

## Shared Runner Environment

Baseline JSONL reruns used:

- `VECTORBT_KERNEL_FAST_PATH=1`
- `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL=off`
- `VECTORBT_KERNEL_USE_CUDA_GRAPHS=off`
- `VECTORBT_KERNEL_PROFILE_PIPELINE` unset
- binary: `target/release/profile_gpu_dynamic_runner.exe`

High-resolution pipeline captures used:

- same runner binary and runtime flags as above
- `VECTORBT_KERNEL_PROFILE_PIPELINE=1`

Important measurement note:

- An initial `quartiles` attempt was accidentally launched in parallel and is not valid under repo benchmarking rules.
- Those contaminated outputs were discarded and replaced in-place by serialized reruns before any conclusions were recorded below.

## Build Command

```powershell
cargo build -p ta_app_core --release --features cuda,cuda-backtest-kernel --bin profile_gpu_dynamic_runner
```

## Direct-Runner Commands

```powershell
.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 5 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode none --include-trade-metrics off --emit-json

.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 5 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode halves --include-trade-metrics off --emit-json

.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 5 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract full17 --shortlist-slice-mode quartiles --include-trade-metrics off --emit-json

.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 5 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode none --include-trade-metrics on --emit-json

.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 5 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode halves --include-trade-metrics on --emit-json

.\target\release\profile_gpu_dynamic_runner.exe --strategy double_ma --fast-ma alma --slow-ma alma --adx-mode off --workload xlarge --candle-shape synth --iterations 5 --warmup-iterations 2 --heatmap-bins 0 --device-id 0 --dynamic-mode off --cuda-graphs off --metric-contract trade22 --shortlist-slice-mode quartiles --include-trade-metrics on --emit-json
```

Stdout was redirected to JSONL files for artifact preservation.

## Saved Artifacts

Runner JSONL outputs:

- `target/double_crossover_phase2_shortlist_attribution_xlarge_full17_none_2026-04-03.jsonl`
- `target/double_crossover_phase2_shortlist_attribution_xlarge_full17_halves_2026-04-03.jsonl`
- `target/double_crossover_phase2_shortlist_attribution_xlarge_full17_quartiles_2026-04-03.jsonl`
- `target/double_crossover_phase2_shortlist_attribution_xlarge_trade22_none_2026-04-03.jsonl`
- `target/double_crossover_phase2_shortlist_attribution_xlarge_trade22_halves_2026-04-03.jsonl`
- `target/double_crossover_phase2_shortlist_attribution_xlarge_trade22_quartiles_2026-04-03.jsonl`

High-resolution pipeline logs:

- `target/double_crossover_phase2_shortlist_pipeline_xlarge_full17_halves_2026-04-03.log`
- `target/double_crossover_phase2_shortlist_pipeline_xlarge_trade22_halves_2026-04-03.log`
- `target/double_crossover_phase2_shortlist_pipeline_xlarge_full17_quartiles_2026-04-03.log`

Focused post-instrumentation Nsight Systems artifacts:

- `target/nsight/double_crossover_phase2_shortlist_full17_halves_post_instrumentation.nsys-rep`
- `target/nsight/double_crossover_phase2_shortlist_full17_halves_post_instrumentation.provenance.json`
- `target/nsight/double_crossover_phase2_shortlist_full17_halves_post_instrumentation_stats.txt`

Gate outputs:

- `target/gpu_perf_summary.json`
- `target/gpu_alma_contract_rows_perf_summary.json`

## Measured Summary

Direct-runner measured-iteration means:

1. `full17`, `shortlist_slice_mode=none`
   - wall / route wall mean: `297.40 ms`
   - `explicit_sync_ms` mean: `280.00 ms`
   - `host_finalize_ms` mean: `0.00 ms`
2. `full17`, `shortlist_slice_mode=halves`
   - wall / route wall mean: `379.20 ms`
   - delta vs `none`: `+81.80 ms`
   - `explicit_sync_ms` mean: `277.00 ms`
   - `host_finalize_ms` mean: `60.40 ms`
   - `host_finalize_shortlist_slice_ms` mean: `60.40 ms`
   - `host_finalize_shortlist_slice_replay_ms` mean: `60.40 ms`
3. `full17`, `shortlist_slice_mode=quartiles`
   - wall / route wall mean: `986.80 ms`
   - delta vs `halves`: `+607.60 ms`
   - `explicit_sync_ms` mean: `739.60 ms`
   - `host_finalize_ms` mean: `164.00 ms`
   - `host_finalize_shortlist_slice_ms` mean: `164.00 ms`
   - `host_finalize_shortlist_slice_replay_ms` mean: `163.80 ms`
4. `trade22`, `shortlist_slice_mode=none`
   - wall / route wall mean: `319.60 ms`
   - `explicit_sync_ms` mean: `292.20 ms`
   - `host_finalize_ms` mean: `0.00 ms`
5. `trade22`, `shortlist_slice_mode=halves`
   - wall / route wall mean: `396.20 ms`
   - delta vs `none`: `+76.60 ms`
   - `explicit_sync_ms` mean: `292.60 ms`
   - `host_finalize_ms` mean: `58.40 ms`
   - `host_finalize_shortlist_slice_ms` mean: `58.40 ms`
   - `host_finalize_shortlist_slice_replay_ms` mean: `58.40 ms`
6. `trade22`, `shortlist_slice_mode=quartiles`
   - wall / route wall mean: `599.00 ms`
   - delta vs `halves`: `+202.80 ms`
   - `explicit_sync_ms` mean: `412.80 ms`
   - `host_finalize_ms` mean: `132.40 ms`
   - `host_finalize_shortlist_slice_ms` mean: `132.40 ms`
   - `host_finalize_shortlist_slice_replay_ms` mean: `132.40 ms`

Best-row result on these `synth` truth surfaces:

- `none` and `halves` both kept `best=(5, 10)` for `full17`
- `none` and `halves` both kept `best=(5, 10)` for `trade22`

That stability does not justify skipping shortlist-slice in product code. It only shows that this particular surface did not change top-1.

## Pipeline-Profile Interpretation

The pipeline logs with `VECTORBT_KERNEL_PROFILE_PIPELINE=1` are the authoritative high-resolution source for the shortlist-slice sub-buckets because the JSON timing fields are millisecond-rounded.

Important interpretation rule:

- pipeline profiling is useful for internal bucket composition, not for absolute wall-time comparison against the baseline JSONL reruns, because enabling it perturbs the timing surface

Observed pattern from the corrected high-resolution logs:

1. `full17`
   - `halves`: `shortlist_slice` was about `215.568 ms` to `224.310 ms`
   - `quartiles`: `shortlist_slice` was about `209.415 ms` to `225.649 ms`
   - in both cases, replay accounted for effectively all of `shortlist_slice`
   - record assembly stayed at about `0.036 ms` to `0.044 ms`
   - sort stayed at about `0.004 ms` to `0.013 ms`
   - diagnostics stayed at about `0.002 ms` to `0.003 ms`
2. `trade22`
   - `halves`: `shortlist_slice` was about `57 ms` in the coarse JSON timing surface
   - the runner-visible sub-buckets again pinned essentially all surfaced shortlist work on replay

The qualitative result is stable even though the pipeline-profiler mode inflates absolute runtime: host-side bookkeeping remains trace-level, while replay dominates the retained shortlist bucket.

## Focused Post-Instrumentation Nsight Systems Correlation

One focused whole-process rerun was captured on:

- `double_ma`
- `alma/alma`
- `xlarge`
- `candle_shape=synth`
- `metric_contract=full17`
- `shortlist_slice_mode=halves`

The saved stats report in `target/nsight/double_crossover_phase2_shortlist_full17_halves_post_instrumentation_stats.txt` showed:

1. `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm`
   - `294.053 ms`
   - `81.7%` of traced GPU kernel time
2. `double_cross_backtest_tm_flex_f32_windowed_m17_regime_off_norisk_eps0_nocomm`
   - `51.595 ms`
   - `14.3%` of traced GPU kernel time
   - `6` instances

Interpretation:

1. The new host-side shortlist timing bucket is consistent with a real secondary GPU replay family, not just host bookkeeping.
2. The next meaningful shortlist-facing optimization, if pursued, should target replay reduction or replay dataflow rather than sort/diagnostics cleanup.

Repo interpretation:

1. shortlist-row materialization is not the active issue on this branch
2. clone/sort/diagnostics cleanup is not where the meaningful time is
3. the active cost is replay itself
4. `quartiles` amplifies the replay cost sharply enough that it looks like diagnostic-only replay expansion rather than a local host-side cleanup problem
5. any meaningful further win from this branch would require reducing or reshaping the replay work, not trimming host bookkeeping

## Verification Commands

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate
```

## Verification Outcome

1. `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed.
2. `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` passed.
3. The specialized contract-row gate passed for `auto`, `full17`, and `trade22`.

## Branch Decision

1. Keep the shortlist-slice timing instrumentation.
2. Do not retain a behavior-changing shortlist-slice optimization from this branch.
3. Treat this branch as evidence that the next shortlist-slice gain, if pursued, is an architecture-level replay reduction question rather than a local host-side cleanup question.
4. Keep `compact_exact_search_stage` deferred for product work unless broader architecture research is explicitly reopened.
