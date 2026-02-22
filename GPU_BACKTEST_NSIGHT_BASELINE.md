# GPU Backtest Nsight Baseline

Date: 2026-02-21

## Scope

Backtest grid kernel path only (Rust/CUDA in this repo), excluding VectorTA internals.

## Capture command

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 `
  -BenchFilter "double_ma/gpu_kernel/sma_sma/200k/small" `
  -OutputPrefix target/nsight/test_gpu_kernel_filtered `
  -LaunchCount 10
```

Report:
- `target/nsight/test_gpu_kernel_filtered.ncu-rep`

## Primary kernel

- Kernel: `double_cross_backtest_tm_flex_f32`
- Launch config: block `(256,1,1)`, grid `(7,1,1)` for the `small` case

## Key Nsight metrics (current baseline)

- Duration: `106.90 ms`
- Compute (SM) Throughput: `1.14%`
- Memory Throughput: `0.77%`
- Achieved Occupancy: `16.15%`
- Scheduler `No Eligible`: `79.05%`
- Eligible warps per scheduler: `0.23`
- SourceCounters warning: large uncoalesced global accesses (reported `85%` excessive sectors)

## Interpretation

- The benchmark case is too small to saturate the GPU (low grid size, low eligible warps).
- Kernel is latency/issue-limited, not bandwidth-limited.
- Global memory access pattern is a strong candidate for optimization (coalescing/tiling/layout).

## Next optimization passes

1. Profile `medium` and `large` grid cases to remove tiny-grid artifacts from occupancy conclusions.
2. Inspect source locations flagged by `UncoalescedGlobalAccess` and improve access coalescing.
3. Re-check launch geometry and workload partitioning to increase eligible warps per scheduler.
4. Re-run `scripts/run_gpu_perf_gate.ps1 -Mode candidate` after each change for regression control.

## 2026-02-21 Launch Policy Update (current)

### Change summary

- Added adaptive backtest launch floor policy in Rust (`choose_backtest_launch_shape_1d`):
  - Use block `>= 256` when pair density is at least `192` pairs/SM.
  - Use block `>= 512` when pair density is at least `1024` pairs/SM.
  - Keep `VECTORBT_KERNEL_BLOCK_X` env override as the top priority.
- Kept a single CUDA backtest kernel and existing valid-pair map path.

### Benchmark snapshots (ALMA/ALMA 200k)

- `small/1590pairs`: `~86 ms` (no material regression)
- `medium/14150pairs`: `~97 ms` (no material change)
- `large/58300pairs`: `~100 ms` (improved from prior ~`140 ms`)
- `xlarge/370750pairs`: `~1.86 s` (improved from prior ~`2.03 s`)
- `xlarge_batched_1gb/370750pairs`: `~0.99 s` (improved)
- `xxlarge/1491500pairs`: `~7.3-7.4 s` (faster with block 512 than 256/64 on this GPU)
- `xxlarge_batched_1gb/1491500pairs`: `~3.52 s` (improved)

### Nsight captures

- Large case report: `target/nsight/alma_large_launch_policy_v2.ncu-rep`
  - Launch: `(228,1,1) x (256,1,1)`
  - Duration (profiled kernel): `108.05 ms`
  - Uncoalesced global access still high (`83%` excessive sectors)
- Xlarge case report: `target/nsight/alma_xlarge_launch_policy_v2.ncu-rep`
  - Launch: `(961,1,1) x (512,1,1)`
  - Duration (profiled kernel): `1.96 s`
  - Uncoalesced global access remains dominant (`82%` excessive sectors)

### Current interpretation

- Launch sizing is materially better for large/xlarge workloads.
- Main remaining bottleneck is still memory access pattern/coalescing inside `double_cross_backtest_tm_flex_f32` (L1TEX dependency stalls).

## 2026-02-22 Coalescing Pass Attempt (TM stride padding)

### What was attempted

- Added optional TM stride padding path in Rust/CUDA:
  - `transpose_row_to_tm` now supports an explicit output stride (`out_rows`).
  - `double_cross_backtest_tm_flex_f32` now accepts explicit TM strides for fast/slow MA loads.
  - Host path can pad TM row strides to warp-aligned widths via env flag:
    - `VECTORBT_KERNEL_TM_PAD=1`
- Default remains:
  - `VECTORBT_KERNEL_TM_PAD=0` (disabled) to avoid regressions.

### Benchmark results

- Controlled launch check (`VECTORBT_KERNEL_BLOCK_X=384`) was regressive:
  - `large/58300pairs`: ~`153.6 ms` vs ~`100.9 ms` default
  - `xlarge/370750pairs`: ~`1.93 s` vs ~`1.88 s` default
- TM padding enabled (`VECTORBT_KERNEL_TM_PAD=1`):
  - `large/58300pairs`: ~`109-110 ms` (regression)
  - `xlarge/370750pairs`: ~`1.88 s` (no material change)
- TM padding disabled (`VECTORBT_KERNEL_TM_PAD=0`, current default):
  - `large/58300pairs`: ~`101-103 ms` (within noise band of prior baseline)
  - `xlarge/370750pairs`: ~`1.84-1.86 s` (slightly improved)

### Nsight result

- Report: `target/nsight/alma_large_coalescing_pad_pass1.ncu-rep`
- `UncoalescedGlobalAccess` stayed at ~`83%` excessive sectors (no improvement vs prior baseline).
- Kernel duration worsened for the padding-on run (~`119.5 ms` profiled vs prior ~`108.1 ms`).

### Decision

- Keep the stride-padding path available only as an opt-in experiment.
- Leave it disabled by default to preserve current performance.
