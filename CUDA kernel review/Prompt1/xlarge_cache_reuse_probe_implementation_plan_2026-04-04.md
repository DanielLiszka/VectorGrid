# Xlarge Cache Reuse Probe Implementation Plan

Date: 2026-04-04

## Execution Outcome

Completed on `2026-04-04`.

Outcome:

- measurement branch executed fully
- decision gate failed
- no code change retained

See:

- `CUDA kernel review/Prompt1/xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `target/cache_probe/`

## Purpose

This document extracts the valid, action-driving parts of `xlarge_cache_reuse_direction_FINAL.md` and turns them into a repo-owned implementation checklist.

This is a **measurement-first** plan. It does not assume that cache-policy or layout changes are already justified.

The current repo evidence supports a narrower question:

1. `large` already looks cache-friendly.
2. `xlarge`, especially `full17`, is more cache-stressed.
3. The repo already has a retained replay-batching win, so this plan should not reopen replay/dataflow work.
4. The next bounded cache-oriented step, if any, is to test whether smaller exact-search tiles improve the hot lockstep kernel on `xlarge/full17`.

Primary evidence baseline:

- `CUDA kernel review/Prompt1/xlarge_cache_reuse_direction_FINAL.md`
- `SASS research/double_crossover_phase2_cache_hit_measurements_2026-04-03.md`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`

## Scope

This plan covers only:

- `double_ma`
- `alma/alma`
- exact `GpuKernel` route
- `xlarge` primary probe surface
- `full17` primary target
- `trade22` contrast case
- `large/full17` regression control

This plan does not authorize:

- broad cache-policy code changes
- broad price-data persistence work
- another kernel-body micro-pass
- replay/dataflow redesign
- PTX or SASS intervention

## Current Evidence To Preserve

Treat these as current repo facts:

1. `large` lockstep kernels already show strong cache behavior.
2. `xlarge/full17` is the worst cache case currently surfaced.
3. The runtime already has reuse surfaces:
   - resident market cache
   - cached `d_lr` / `d_ret1`
   - valid-pair map caching
   - persistent scratch
4. The tile chooser is already explicit and externally steerable through:
   - `VECTORBT_KERNEL_VRAM_BUDGET_MB`
   - `VECTORBT_KERNEL_TILE_DEBUG`
   - `estimate_tile_bytes(...)`
   - `choose_tiles(...)`

## Working Hypothesis

The working hypothesis for this plan is narrow:

- on `xlarge/full17`, the current chosen exact-search tile may be too large for good cache fit
- forcing smaller tiles may improve both hot-kernel cache behavior and route wall time

This is still only a hypothesis.

The probe is not meant to prove:

- that price arrays specifically are the problem
- that L2 persistence is the right fix
- that TM-state alone explains the remaining slowdown

## Code Touch Surfaces

Primary touch surface if a code change is later justified:

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - `vram_budget_bytes()`
  - `estimate_tile_bytes(...)`
  - `choose_tiles(...)`

Diagnostics-only support if needed:

- `crates/app_core/src/double_ma.rs`

Out of scope for the first branch:

- `crates/app_core/src/kernels/double_crossover.cu`

## Truth Surfaces

Primary target:

1. `double_ma`, `alma/alma`, `xlarge`, `synth`, `metric_contract=full17`, `shortlist_slice_mode=none`

Contrast:

2. `double_ma`, `alma/alma`, `xlarge`, `synth`, `metric_contract=trade22`, `shortlist_slice_mode=none`

Regression control:

3. `double_ma`, `alma/alma`, `large`, `synth`, `metric_contract=full17`, `shortlist_slice_mode=none`

## Phase 0: Preflight

- [ ] Confirm no benchmark or profiler process is active.
- [ ] Build `profile_gpu_dynamic_runner.exe` with:
  - `cargo build -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel`
- [ ] Record current branch and commit.
- [ ] Record GPU, driver, and Nsight Compute version.
- [ ] Keep the current retained code state fixed while the probe is running.

## Phase 1: Baseline Tile-Sensitivity Probe

Goal: determine whether shrinking the chosen exact-search tile changes both cache behavior and route wall time on `xlarge/full17`.

### 1A. Baseline tile debug

- [ ] Run the primary `xlarge/full17` truth surface with:
  - `VECTORBT_KERNEL_TILE_DEBUG=1`
  - default `VECTORBT_KERNEL_VRAM_BUDGET_MB` behavior
- [ ] Save:
  - direct-runner output
  - tile-debug stderr
  - route diagnostics
- [ ] Record:
  - reported full-fit size
  - chosen `pf_tile`
  - chosen `ps_tile`

### 1B. Budget sweep

- [ ] Choose `3` to `5` smaller `VECTORBT_KERNEL_VRAM_BUDGET_MB` settings below the default full-fit threshold from step `1A`.
- [ ] Re-run the same `xlarge/full17` surface for each budget.
- [ ] Keep:
  - `--iterations 1`
  - `--warmup-iterations 0`
  - `--heatmap-bins 0`
  - `--dynamic-mode off`
  - `--cuda-graphs off`
  - `--shortlist-slice-mode none`
- [ ] Save per-budget:
  - route wall time
  - tile-debug output
  - route diagnostics

### 1C. Contrast and regression controls

- [ ] Run the same sweep shape on `xlarge/trade22`.
- [ ] Run at least:
  - baseline
  - best-smaller-budget candidate
  on `large/full17` as regression control.

## Phase 2: Cache Confirmation With Nsight Compute

Goal: confirm whether the best-smaller-budget point materially changes lockstep-kernel cache behavior.

- [ ] Capture `MemoryWorkloadAnalysis` for baseline `xlarge/full17`.
- [ ] Capture `MemoryWorkloadAnalysis` for the best-smaller-budget `xlarge/full17` point.
- [ ] Use the hot-kernel filter:
  - `regex:^double_cross_backtest_tm_flex_f32_lockstep_time_m17`
- [ ] Extract at minimum:
  - `L1/TEX Hit Rate`
  - `L2 Hit Rate`
  - `Memory Throughput`
  - `Mem Busy`
- [ ] Repeat the same baseline vs best-point comparison on `xlarge/trade22`.
- [ ] Confirm that `large/full17` does not regress into a meaningfully worse cache regime.

## Phase 3: Decision Gate

Only promote this branch beyond measurement if all of the following are true:

- [ ] The reduced-budget run actually produced smaller `pf_tile` or `ps_tile`.
- [ ] `xlarge/full17` cache metrics improve meaningfully at the better point.
- [ ] Route wall time also improves at the better point.
- [ ] The effect is at least directionally consistent on `xlarge/trade22`, or the `full17` specificity is strong enough to justify a contract-specific follow-up.
- [ ] `large/full17` does not regress materially.

If the gate is not met:

- [ ] Stop the branch without code changes.
- [ ] Write a short no-go note.

## Phase 4: Conditional Narrow Code Change

Only start this phase if Phase 3 is clearly positive.

Candidate retained change:

- [ ] Add a narrow cache-fit cap or budget policy adjustment inside `choose_tiles()` for the exact `xlarge` route.
- [ ] Keep `estimate_tile_bytes(...)` as the footprint authority.
- [ ] Keep the change isolated to tile choice rather than changing kernel semantics.
- [ ] Do not broaden into:
  - replay logic
  - reducer logic
  - kernel-body changes
  - new cache-policy APIs

## Phase 5: Verification After Any Code Change

- [ ] Re-run:
  - `large/full17`
  - `large/trade22`
  - `xlarge/full17`
  - `xlarge/trade22`
- [ ] Re-run shortlist-heavy `xlarge` truth surfaces only if the tile change affects those routes in practice.
- [ ] Run:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- [ ] Save all outputs and provenance.

## Artifact Checklist

Suggested artifact stem:

- `target/cache_probe/`

Suggested files:

- [ ] baseline runner log for `xlarge/full17`
- [ ] per-budget runner logs for `xlarge/full17`
- [ ] per-budget tile-debug logs
- [ ] baseline and best-point NCU summaries for `xlarge/full17`
- [ ] baseline and best-point NCU summaries for `xlarge/trade22`
- [ ] regression-control logs for `large/full17`
- [ ] one summary markdown note comparing:
  - baseline
  - best point
  - contrast case
  - regression control

## Example Command Shape

Primary direct-runner shape:

```powershell
$env:VECTORBT_KERNEL_TILE_DEBUG='1'
$env:VECTORBT_KERNEL_VRAM_BUDGET_MB='<budget_mb>'
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
  --shortlist-slice-mode none `
  --include-trade-metrics off
Remove-Item Env:VECTORBT_KERNEL_TILE_DEBUG
Remove-Item Env:VECTORBT_KERNEL_VRAM_BUDGET_MB
```

Baseline NCU shape:

```powershell
& 'C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\target\windows-desktop-win7-x64\ncu.exe' `
  --target-processes all `
  --section MemoryWorkloadAnalysis `
  --kernel-name-base demangled `
  --kernel-name 'regex:^double_cross_backtest_tm_flex_f32_lockstep_time_m17' `
  --launch-count 1 `
  --force-overwrite `
  --export target/nsight/cache_probe_xlarge_full17_baseline `
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
  --shortlist-slice-mode none `
  --include-trade-metrics off
```

## Rejection Criteria

Reject this branch before implementation if any of the following happen:

- [ ] The budget sweep never changes `pf_tile` or `ps_tile`.
- [ ] Cache metrics stay essentially flat as tiles shrink.
- [ ] Cache metrics improve but route wall time does not.
- [ ] The effect is too fragile to repeat.
- [ ] `large` regresses materially.
- [ ] The evidence continues to point more strongly at `full17` store/access shape than at tile-fit or reuse distance.

## Decision Rule

Treat this plan as successful if it produces one of these outcomes:

1. a clear positive result that justifies one narrow tile-choice code change, or
2. a clean no-go result that closes this cache-fit lane without more speculation

Anything weaker should be treated as a deferral, not a partial win.
