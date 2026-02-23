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

## 2026-02-22 Source-Counter Guided Pass (Fast-MA Warp Dedup)

### What changed

- Added a second backtest kernel entrypoint with warp-level fast-MA load dedup:
  - `double_cross_backtest_tm_flex_f32_fast_warp_dedup`
- Implementation detail:
  - Lanes with the same `pf_local` use `__match_any_sync` + `__shfl_sync` so only one lane performs the fast-MA load per subgroup.
- Rust launch policy now selects kernel variant by workload size:
  - enabled when `launch_pairs >= 150000` and kernel is available.
  - initially default enabled via `VECTORBT_KERNEL_FAST_WARP_DEDUP=1`.
  - tunable threshold via `VECTORBT_KERNEL_FAST_WARP_DEDUP_MIN_PAIRS`.
  - can be disabled with `VECTORBT_KERNEL_FAST_WARP_DEDUP=0`.
- Updated Nsight helper script default kernel filter to include both variants:
  - `regex:^double_cross_backtest_tm_flex_f32`.

### Benchmark snapshot (ALMA/ALMA 200k)

- `large/58300pairs` (default policy): ~`100 ms` (no meaningful regression).
- `xlarge/370750pairs`:
  - dedup enabled (default): ~`0.74 s`
  - dedup disabled: ~`1.85 s`
  - speedup: roughly `2.5x` on this workload.

### Nsight snapshot

- New report: `target/nsight/alma_xlarge_fast_warp_dedup_pass1.ncu-rep`
- Same-commit no-dedup comparison: `target/nsight/alma_xlarge_no_dedup_pass1.ncu-rep`

Key deltas (xlarge profiled kernel):
- Duration: `~1.96 s -> ~0.77 s`
- Eligible warps/scheduler: `~0.16 -> ~0.68`
- No Eligible: `~86% -> ~56%`
- Uncoalesced global access: `~82% -> ~76%` excessive sectors.

### Interpretation

- This pass materially improves latency/eligibility on larger launch spaces.
- Work remains on coalescing quality, but the main throughput bottleneck for xlarge workloads is significantly reduced.

## 2026-02-22 Source-Counter Guided Pass 2 (Lockstep Time Index)

### What changed

- Added a third backtest kernel entrypoint:
  - `double_cross_backtest_tm_flex_f32_lockstep_time`
- Kernel behavior:
  - keeps warp lanes on a common loop time index (`t`),
  - preserves per-pair warmup semantics by gating work with `if (t < start_t) continue`.
- Runtime selection now prefers lockstep kernel on larger workloads:
  - `VECTORBT_KERNEL_LOCKSTEP_TIME=1` (default on),
  - `VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS=150000` (default).
- Fast-dedup kernel remains available but is now opt-in by default:
  - `VECTORBT_KERNEL_FAST_WARP_DEDUP=0` (default off),
  - only considered when lockstep kernel is not selected.

### Benchmark snapshot (ALMA/ALMA 200k)

- `large/58300pairs`: ~`100.2-100.5 ms` (no regression vs prior ~`100 ms` range)
- `xlarge/370750pairs`:
  - no lockstep, no dedup: ~`1.85 s`
  - no lockstep, fast dedup: ~`0.715-0.719 s`
  - lockstep (default): ~`447-453 ms`
- `xlarge_batched_1gb/370750pairs`: ~`944-949 ms`
- `xxlarge/1491500pairs`: ~`1.666-1.674 s` (down from prior ~`2.26 s` fast-dedup path)
- `xxlarge_batched_1gb/1491500pairs`: ~`3.54-3.59 s` (within noise vs prior baseline)

### Nsight snapshot (same commit, xlarge)

Reports:
- `target/nsight/alma_xlarge_no_lockstep_no_dedup_pass2.ncu-rep`
- `target/nsight/alma_xlarge_fast_warp_no_lockstep_pass2.ncu-rep`
- `target/nsight/alma_xlarge_lockstep_pass2.ncu-rep`

Key deltas:
- Duration:
  - `1.95 s` (no lockstep/no dedup)
  - `765 ms` (fast dedup, no lockstep)
  - `451 ms` (lockstep)
- Scheduler:
  - `No Eligible`: `86.11% -> 56.33% -> 38.44%`
  - `Eligible warps/scheduler`: `0.16 -> 0.68 -> 1.26`
- SourceCounters `UncoalescedGlobalAccess`:
  - excessive sectors ratio: `82% -> 76% -> 13%`

### Interpretation

- The dominant coalescing issue was lane time-index desynchronization, not just pair-index ordering.
- Lockstep-time iteration substantially improves memory access locality and scheduler eligibility.
- Remaining notable inefficiency is store-side layout (`metrics_out` AoS writes are still strided).

## 2026-02-22 Follow-up Attempts On Residual Load-Side Access (Reverted)

### Attempts

- Tried a combined kernel path: `lockstep + fast-warp-dedup`.
- Tried a cheaper fast-MA dedup helper variant (uniform-warp fast path before `__match_any_sync`).

### Benchmark outcome (ALMA/ALMA xlarge, clean sequential)

- Lockstep baseline (kept): ~`452-456 ms`
- Lockstep + dedup combo: ~`616-617 ms` (regression)
- No-lockstep fast-dedup with modified helper: ~`756-764 ms` (regression vs prior ~`714-719 ms`)

### Decision

- Reverted both follow-up attempts.
- Kept lockstep-only path as the load-side access optimization baseline.

## 2026-02-22 Metrics-Stage Pass 1 (Adaptive `M=7` Specialization)

### What changed

- Added specialized kernels for the common full-metrics path (`M=7`):
  - `double_cross_backtest_tm_flex_f32_m7`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
- Runtime selection policy:
  - `VECTORBT_KERNEL_M7_SPECIALIZE=1` (default on)
  - `VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS=300000` (default)
  - disabled automatically for multi-tile/batched workloads

### Benchmark snapshot (ALMA/ALMA 200k)

- `large/58300pairs`: ~`99.19-99.54 ms` (noise-level change)
- `xlarge/370750pairs`: ~`395.67-401.13 ms` (improved)
- `xlarge_batched_1gb/370750pairs`: ~`946.80-957.33 ms` (improved vs recent ~`0.99 s` runs)
- `xxlarge/1491500pairs`: ~`1.552-1.560 s` (improved)
- `xxlarge_batched_1gb/1491500pairs`: ~`3.621-3.662 s` (no material change)

Cross-check:
- `sma_sma xlarge/370750pairs`: ~`399.09-405.72 ms` (improved)

### Nsight snapshot

- New report: `target/nsight/alma_xlarge_lockstep_m7_pass4.ncu-rep`
- Compared against:
  - `target/nsight/alma_xlarge_lockstep_pass2.ncu-rep`

Key deltas (xlarge profiled kernel):
- Kernel duration: `450.94 ms -> 427.69 ms`
- Eligible warps/scheduler: `1.26 -> 1.37`
- `No Eligible`: `38.44% -> 37.79%`
- Executed instructions: `268.8B -> 249.3B`
- `UncoalescedGlobalAccess`: unchanged at ~`13%` excessive sectors (same source-counter diagnosis)

### Interpretation

- This pass improves performance primarily by reducing metric-stage/control-flow overhead (fewer executed instructions), not by changing memory coalescing.
- Next major target remains physical metrics output layout / store-side write efficiency.

## 2026-02-22 Metrics-Stage / Store-Side Pass 2 (Padded AoS Tile Stride, Opt-In)

### What changed

- Added explicit `metrics_stride` support to the backtest kernel and tile-buffer consumers that read `scratch.d_tile`:
  - `double_cross_backtest_tm_flex_f32*` (all variants)
  - `heatmap_update_scores_tm_f32`
  - `select_topk_from_pairs_tm_f32*`
  - `select_topk_from_candidates_tm_f32*`
  - `gather_topk_pairs_tm_f32`
- Rust host path now supports a physical tile stride different from logical `metrics_count`.
- New opt-in env flag:
  - `VECTORBT_KERNEL_METRICS_PAD8=1`
  - When enabled and `metrics_count == 7`, `scratch.d_tile` uses physical stride `8` floats.
- `top_metrics` / global compact metric rows remain tightly packed (`M` stride); only the large per-tile metrics buffer is padded.

### Why this pass

- Prior Nsight runs still showed residual store/read inefficiency around `metrics_out` AoS accesses.
- A full SoA refactor is broader in scope; padding `7 -> 8` floats is a reversible intermediate experiment targeting alignment/coalescing.

### Benchmark snapshot (same commit)

ALMA/ALMA 200k:
- `xlarge/370750pairs` A/B:
  - `pad8=0`: `417.13-423.38 ms`
  - `pad8=1`: `391.84-399.93 ms` (improved)
- `large/58300pairs` (`pad8=1`): `100.14-103.86 ms` (no material regression)
- `xlarge_batched_1gb/370750pairs` (`pad8=1`): `893.44-898.32 ms` (improved)
- `xxlarge/1491500pairs` (`pad8=1`): `1.5562-1.5721 s` (roughly neutral vs recent pass-4 range)

Cross-checks:
- `sma_sma xlarge/370750pairs` same-commit A/B:
  - `pad8=0`: `406.32-412.11 ms`
  - `pad8=1`: `410.26-416.60 ms` (small regression)
- `regime_hybrid` strategy profile (`alma_alma`, xlarge, `pad8=1`):
  - `497.32-504.82 ms` (no clear change)

Prebuilt PTX sanity:
- `VECTORBT_USE_PREBUILT_PTX=1`, `VECTORBT_KERNEL_METRICS_PAD8=1`, ALMA/ALMA xlarge:
  - `395.48-398.56 ms`

### Nsight snapshot

- New report: `target/nsight/alma_xlarge_lockstep_m7_pad8_pass5.ncu-rep`
- Compare against: `target/nsight/alma_xlarge_lockstep_m7_pass4.ncu-rep`

Key deltas (xlarge profiled kernel):
- Kernel duration: `427.69 ms -> 408.99 ms`
- Executed instructions: effectively unchanged (~`249.3B`)
- Eligible warps/scheduler: ~`1.37` (flat)
- `No Eligible`: ~`37.8%` (flat)
- `UncoalescedGlobalAccess`: still reported at ~`13%` excessive sectors

### Interpretation

- This pass likely improves transaction alignment (same instruction count, lower runtime) rather than reducing instruction volume.
- Benefit is workload-dependent; ALMA workloads improved clearly, while `sma_sma xlarge` showed a small regression.

### Decision

- Keep the code path available for continued experimentation.
- Do **not** enable by default yet:
  - `VECTORBT_KERNEL_METRICS_PAD8=0` remains the default until broader coverage or adaptive gating is implemented.

### Notes

- One parallel benchmark attempt (`large` + `xxlarge`) was discarded due GPU contention. Only sequential reruns were used for decisions.

## 2026-02-22 Metrics-Stage / Store-Side Follow-ups (Policy + Reverted Vector-Store Attempt)

### Attempt A (reverted): vectorized final stores on `pad8` path

- Tried emitting two `float4` stores for `FULL_M7` when `metrics_stride == 8`.
- Goal: improve padded-tile write throughput further.
- Outcome (clean sequential A/B):
  - `alma_alma xlarge` with `pad8=1` regressed into the ~`427-446 ms` band.
  - This erased the prior `pad8` benefit (~`392-400 ms` band).
- Decision:
  - Reverted; scalar final stores performed better in this kernel.

### Attempt B (kept): selective default `pad8` policy (MA-pair-aware)

- `VECTORBT_KERNEL_METRICS_PAD8` remains an env override.
- New default behavior when env is unset:
  - enabled for `alma/alma`
  - disabled otherwise
- This preserves ALMA wins while avoiding the small `sma_sma xlarge` regression seen with global `pad8` default-on.

### Benchmark spot-checks (env unset, clean sequential)

- `alma_alma xlarge/370750pairs`: `401.48-409.86 ms`
- `alma_alma xlarge_batched_1gb/370750pairs`: `897.79-907.05 ms`
- `sma_sma xlarge/370750pairs`: `403.34-408.91 ms`

### Notes

- Additional parallel benchmark attempts were discarded due single-GPU contention (benchmark vs benchmark; benchmark vs test).

## 2026-02-22 Backtest-Kernel Follow-up (Regime-Mode Specialization Nsight A/B)

### Goal

- Validate the prior `lockstep+m7` regime-mode specialization with Nsight Compute and determine whether the win comes from instruction reduction or memory-access changes.

### Reports

- `target/nsight/alma_xlarge_lockstep_m7_regime_mode_off_pass6.ncu-rep`
- `target/nsight/alma_xlarge_lockstep_m7_regime_mode_on_pass6.ncu-rep`

Profiled kernels:
- off: `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
- on: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off`

### Nsight snapshot (ALMA/ALMA xlarge, replay mode)

- Kernel duration: `420.46 ms -> 416.83 ms`
- Executed instructions: `249.31B -> 242.00B`
- Eligible warps/scheduler: `1.38 -> 1.35`
- Derived `No Eligible` (`100 - smsp__issue_active.avg.pct_of_peak_sustained_active`): `37.43% -> 38.77%`
- L2 theoretical excessive sectors ratio (computed from `memory_l2_theoretical_sectors_global` vs `_ideal`): ~`12.62% -> 12.62%` (flat)

### Interpretation

- The specialization reduces instruction count while leaving the memory-sector/coalescing signature effectively unchanged.
- Replay-mode Nsight shows only a modest duration improvement, but Criterion A/B runs still show larger end-to-end wins for ALMA/ALMA and `regime_hybrid`.

## 2026-02-22 Backtest-Kernel Pass (No-Risk Specialization) - Benchmark Stage

### Status

- Added `lockstep+m7 regime_off no-risk` specialization (`double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`) and host dispatch support.
- Benchmarks show wins on ALMA/ALMA and `sma_sma` xlarge workloads; pass is currently kept.
- Nsight A/B captured (see follow-up below).

## 2026-02-22 Backtest-Kernel Follow-up (No-Risk Specialization Nsight A/B)

### Reports

- `target/nsight/alma_xlarge_lockstep_m7_norisk_off_pass7.ncu-rep`
- `target/nsight/alma_xlarge_lockstep_m7_norisk_on_pass7.ncu-rep`

Profiled kernels:
- off: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off`
- on: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`

### Nsight snapshot (ALMA/ALMA xlarge, replay mode)

- Kernel duration: `421.23 ms -> 398.70 ms`
- Executed instructions: `242.00B -> 239.65B`
- Eligible warps/scheduler: `1.35 -> 1.42`
- Derived `No Eligible` (`100 - smsp__issue_active.avg.pct_of_peak_sustained_active`): `38.35% -> 35.34%`
- L2 theoretical excessive sectors ratio (computed from `memory_l2_theoretical_sectors_global` vs `_ideal`): ~`12.62% -> 12.62%` (flat)

### Interpretation

- This pass improves scheduler readiness and reduces instruction volume while leaving the memory-sector/coalescing signature unchanged.
- This matches the stronger Criterion A/B gains observed on no-risk workloads.

## 2026-02-22 Backtest-Kernel Pass (Regime No-Risk Specialization Extension) - Benchmark Stage

### Status

- Added `lockstep+m7` no-risk wrappers for regime-filtered variants:
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk`
  - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`
- Host dispatch now selects matching no-risk wrappers (`off/on/hybrid`) when stop-loss/take-profit are disabled.
- `regime_hybrid` benchmark (ALMA/ALMA xlarge) improved in same-commit A/B:
  - `norisk=0`: `497.27-505.45 ms`
  - `norisk=1`: `477.46-485.46 ms`
- Nsight A/B for the regime-hybrid no-risk wrapper is captured below.

## 2026-02-22 Backtest-Kernel Follow-up (Regime-Hybrid No-Risk Specialization Nsight A/B)

### Reports

- `target/nsight/regime_hybrid_alma_xlarge_norisk_off_pass8.ncu-rep`
- `target/nsight/regime_hybrid_alma_xlarge_norisk_on_pass8.ncu-rep`

Profiled kernels:
- off: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid`
- on: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`

### Nsight snapshot (ALMA/ALMA xlarge, replay mode)

- Kernel duration: `536.83 ms -> 498.37 ms`
- Executed instructions: `329.32B -> 317.19B`
- Eligible warps/scheduler: `1.46 -> 1.45`
- Derived `No Eligible` (`100 - smsp__issue_active.avg.pct_of_peak_sustained_active`): `35.95% -> 34.91%`
- L2 theoretical excessive sectors ratio (computed from `memory_l2_theoretical_sectors_global` vs `_ideal`): ~`10.98% -> 10.98%` (flat)

### Interpretation

- This follows the same pattern as the prior no-risk Nsight pass: instruction-count reduction is the primary win, while memory-sector/coalescing behavior is effectively unchanged.
- Replay-mode Nsight shows a stronger duration win here than the `regime_off` no-risk specialization.

## 2026-02-22 Backtest-Kernel Pass Attempt (No-Commission Specialization) - Reverted

### Goal

- Test whether compile-time specialization for `commission == 0.0f` improves hot-loop performance on `lockstep+m7` no-risk workloads (a common benchmark configuration).

### Attempted change (reverted)

- Added a compile-time commission-mode parameter and `*_norisk_nocomm` wrapper kernels for `regime_off`, `regime_on`, and `regime_hybrid`.
- Added host dispatch for exact-zero commission config values.

### Benchmark outcome (same-commit, sequential A/B)

- ALMA/ALMA xlarge (`regime_mode=1`, `norisk=1`):
  - `nocomm=0`: `368.16-373.45 ms`
  - `nocomm=1`: `375.17-380.35 ms` (no win; slight regression/noise)
- `regime_hybrid` ALMA/ALMA xlarge (`regime_mode=1`, `norisk=1`):
  - `nocomm=0`: `480.86-487.68 ms`
  - `nocomm=1`: `481.37-488.29 ms` (no material change)

### Decision

- Reverted the no-commission specialization.
- The additional kernel variants and dispatch complexity were not justified without a repeatable performance gain.

## 2026-02-22 Backtest-Kernel Pass Attempt (Trade-On-Next-Bar Specialization) - Reverted

### Goal

- Test whether compile-time specialization for `trade_on_next_bar=true` improves the hot loop on `lockstep+m7` no-risk workloads (common default strategy behavior).

### Attempted change (reverted)

- Added a compile-time trade-on-next-bar mode parameter to the shared CUDA backtest body.
- Added `*_norisk_tnb_on` wrapper kernels for `regime_off`, `regime_on`, and `regime_hybrid`.
- Added host dispatch behind `VECTORBT_KERNEL_LOCKSTEP_M7_TNB_SPECIALIZE`.

### Benchmark outcome (same-commit, sequential A/B)

- ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`):
  - `tnb=0`: `392.69-398.99 ms`
  - `tnb=1`: `406.23-411.29 ms` (clear regression)

### Decision

- Reverted the trade-on-next-bar specialization.
- Extra kernel variants and dispatch complexity are not justified for a regressing primary workload.

## 2026-02-22 Backtest-Kernel Pass Attempt (Template Branch-Tightening Cleanup) - Split and Reverted

### Goal

- Reduce hot-loop instruction overhead without adding new kernel variants by using compile-time branch selection (`if constexpr`) inside the shared CUDA kernel body.

### Attempted changes (split)

- Sub-pass A: switched `desired_sign_mode_f32<REGIME_MODE>` dispatch from runtime `if (REGIME_MODE == ...)` checks to `if constexpr`.
- Sub-pass B: rewrote in-loop metric updates for `FULL_M7` to explicit `if constexpr (FULL_M7)` fast paths.

### Benchmark outcome (sequential reruns only; parallel runs discarded)

- Sub-pass A (`desired_sign_mode_f32 if constexpr`) was neutral:
  - ALMA/ALMA xlarge: `373.40-377.99 ms` (no significant change vs nearby baseline)
  - `regime_hybrid` ALMA/ALMA xlarge: `483.89-493.20 ms` (no significant change)
- Sub-pass B (`FULL_M7` in-loop `if constexpr`) regressed ALMA/ALMA xlarge:
  - `396.98-402.45 ms` (clear regression, ~`5-7%`)

### Decision

- Reverted sub-pass B due regression on the primary ALMA/ALMA xlarge workload.
- Reverted sub-pass A as well (neutral / no measurable performance win).

## 2026-02-22 Backtest-Kernel Pass (Regime-Hybrid Helper Dead-Branch Cleanup) - Kept

### Goal

- Reduce instruction overhead in the `regime_hybrid` sign-selection helper path without adding new kernel variants.

### Change

- Removed a dead `fabsf(spread_rel)` branch sequence from the hybrid range-mode logic in:
  - `desired_sign_regime_hybrid_f32`
  - `desired_sign_with_regime_f32` (runtime-dispatch fallback)
- Semantics are unchanged because the removed branch and fallback branch both returned `0`.

### Benchmark outcome (sequential reruns)

- `regime_hybrid` ALMA/ALMA xlarge:
  - post-pass: `467.25-477.60 ms`
  - nearby pre-pass comparison band: `478.46-488.42 ms`
- ALMA/ALMA xlarge baseline: `371.16-377.06 ms` (no regression)
- `trend_quality` ALMA/ALMA xlarge: `492.79-501.69 ms` (no material change)
- `sma_sma` xlarge baseline: `379.25-383.79 ms` (no material change)

Notes:
- Multiple accidental parallel GPU benchmark runs were discarded (single-GPU contention / Cargo lock contention). Only sequential reruns were used for decisions.

### Nsight follow-up (`regime_hybrid_norisk`, xlarge ALMA/ALMA)

Reports:
- baseline: `target/nsight/regime_hybrid_alma_xlarge_norisk_on_pass8.ncu-rep`
- current: `target/nsight/regime_hybrid_alma_xlarge_deadfabs_pass9.ncu-rep`

Profiled kernel:
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`

Nsight snapshot (replay mode):
- Kernel duration: `498.37 ms -> 492.39 ms`
- Executed instructions: `317.19B -> 305.75B`
- Eligible warps/scheduler: `1.45 -> 1.46` (flat/slight improvement)
- Derived `No Eligible`: `34.91% -> 35.01%` (flat)
- L2 theoretical excessive sectors ratio: effectively unchanged

### Interpretation

- This is an instruction-count win in the `regime_hybrid` helper path.
- Memory/coalescing behavior remains unchanged, which matches the nature of the code change.

### PTX sync / deployment note

- Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` with CUDA 13.0 (`compute_89`) and explicit MSVC `-ccbin`.
- `VECTORBT_USE_PREBUILT_PTX=1` sanity on `regime_hybrid` ALMA/ALMA xlarge: `468.83-474.01 ms`.

## 2026-02-22 Backtest-Kernel Pass (Regime-Hybrid Helper Branch+Math Tightening) - Kept

### Goal

- Further reduce `regime_hybrid` helper overhead by:
  - skipping `sgn_eps(...)` work when the helper exits through hybrid range-mode
  - replacing a division-based threshold compare with multiply-threshold compares

### Change

- Updated `desired_sign_regime_hybrid_f32` and `desired_sign_with_regime_f32` in `crates/app_core/src/kernels/double_crossover.cu`:
  - compute `long_only` once
  - delay `sgn_eps` computation until after the hybrid range-mode early-return path
  - replace:
    - `spread_rel = (f - s) / denom`
    - compare `spread_rel` against `±regime_range_entry_rel`
  - with:
    - `delta = f - s`
    - `entry = regime_range_entry_rel * denom`
    - `range_sign = (delta <= -entry) - (delta >= entry)`

### Benchmark outcome (sequential reruns only)

- `regime_hybrid` ALMA/ALMA xlarge:
  - post-pass: `424.85-431.84 ms`
  - nearby pre-pass comparison band: `467.25-477.60 ms` (material improvement)
- ALMA/ALMA baseline xlarge: `373.89-381.91 ms` (no material regression)
- `sma_sma` xlarge: `381.12-387.06 ms` (no material regression)

Notes:
- Multiple accidental parallel benchmark runs (benchmark-vs-benchmark and benchmark-vs-Nsight) were discarded due single-GPU contention.

### Nsight follow-up (`regime_hybrid_norisk`, xlarge ALMA/ALMA)

Reports:
- baseline: `target/nsight/regime_hybrid_alma_xlarge_deadfabs_pass9.ncu-rep`
- current: `target/nsight/regime_hybrid_alma_xlarge_hybridhelper_pass10.ncu-rep`

Profiled kernel:
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`

Nsight snapshot (replay mode):
- Kernel duration: `492.39 ms -> 464.06 ms`
- Executed instructions: `305.75B -> 264.34B`
- Eligible warps/scheduler: `1.46 -> 1.23` (regressed)
- Derived `No Eligible`: `35.01% -> 39.08%` (regressed)
- L2 theoretical excessive sectors ratio: effectively unchanged

### Interpretation

- This is a substantial instruction-count reduction in the `regime_hybrid` helper path.
- Despite weaker scheduler-readiness metrics, total kernel duration improves materially because the kernel executes significantly fewer instructions.
- Memory/coalescing behavior remains unchanged.

### PTX sync / deployment note

- Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` with CUDA 13.0 (`compute_89`) and explicit MSVC `-ccbin`.
- `VECTORBT_USE_PREBUILT_PTX=1` sanity on `regime_hybrid` ALMA/ALMA xlarge:
  - first run `448.35-455.97 ms` (warm-up outlier; not used for decision)
  - repeat run `427.87-434.58 ms` (matches the post-pass fast band)

## 2026-02-23 Backtest-Kernel Pass Attempt (Helper-Signature Cleanup `flags -> long_only`) - Reverted

### Goal

- Reduce repeated `STRAT_LONG_ONLY` bit tests in sign helpers by passing a precomputed `long_only` flag through `desired_sign_*` helper signatures.

### Outcome

- Validation (`gpu_kernel_topk_v2_matches_forced_legacy`) passed, but the pass did not produce a recoverable performance win.
- Sequential `sma_sma` xlarge rerun after reverting helper signatures back to `flags` remained in a slower band (`437.81-444.18 ms`), so the attempted signature cleanup was reverted and not kept.

### Source-note / PTX sync

- During the helper-signature revert, helper-local `STRAT_LONG_ONLY` references were briefly reintroduced before the macro definition in `double_crossover.cu`, which broke direct `nvcc` PTX regeneration.
- Fixed by restoring helper-local checks to `(1u << 0)` before regenerating `double_crossover.prebuilt.ptx`.

## 2026-02-23 Backtest-Kernel Pass Attempt (Flag/Commission Hoisting) - Reverted

### Goal

- Hoist repeated runtime flag checks and commission comparisons in `double_cross_backtest_tm_flex_f32_body` without adding kernel variants.

### Benchmark outcome (sequential)

- ALMA/ALMA xlarge: `395.28-397.67 ms` (regressed vs current fast band)
- `regime_hybrid` ALMA/ALMA xlarge: `426.46-431.51 ms` (roughly neutral)

### Decision

- Reverted due regression on the primary ALMA/ALMA xlarge workload.

## 2026-02-23 Backtest-Kernel Pass (`sgn_eps` Zero-Epsilon Fast Path) - Kept

### Goal

- Reduce sign-helper overhead for the common `eps == 0` case without adding kernel variants.

### Change

- Updated `sgn_eps(float f, float s, float eps)` in `crates/app_core/src/kernels/double_crossover.cu`:
  - if `eps <= 0.0f`, return `(f > s) - (f < s)`
  - otherwise keep the epsilon-threshold compare path

### Validation

- `gpu_kernel_topk_v2_matches_forced_legacy` passes after the change.

### Benchmark outcome (sequential reruns)

- ALMA/ALMA xlarge: `369.76-376.24 ms` (improved vs nearby `~373.89-381.91 ms` band)
- `regime_hybrid` ALMA/ALMA xlarge: `428.24-434.11 ms` (no material change)
- `sma_sma` xlarge: rerun `379.96-384.04 ms` (fast band; no regression)

Notes:
- Discarded one anomalous `sma_sma` run at ~`2.83 s` (single-run outlier); sequential rerun returned to the normal band and was used for the decision.

### PTX sync / deployment note

- Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` with CUDA 13.0 (`compute_89`) using explicit MSVC `-ccbin`.

### Decision

- Kept the `sgn_eps` zero-epsilon fast path.

### Follow-up (prebuilt PTX performance)

- `VECTORBT_USE_PREBUILT_PTX=1` ALMA/ALMA xlarge sanity after PTX regen measured:
  - first run: `402.78-410.95 ms`
  - repeat: `406.04-412.60 ms`
- This is materially slower than the JIT/source path (`369.76-376.24 ms`) even though the regenerated PTX compiles and loads correctly.
- Keep as a tracked follow-up; not part of the kept `sgn_eps` pass decision for the default JIT/source path.

## 2026-02-23 Backtest-Kernel Pass Attempt (Branchless Exposure Abs Accumulation) - Reverted

### Goal

- Reduce stats-path instruction cost by replacing `llabs((long long)pos)` / `llabs((long long)pos_bar)` with boolean increments (`pos != 0`) because position magnitude is limited to `{-1,0,1}`.

### Outcome

- Validation passed (`gpu_kernel_topk_v2_matches_forced_legacy`), but sequential benchmarks showed regressions on primary workloads.

### Benchmark outcome (sequential reruns)

- ALMA/ALMA xlarge: `391.42-396.13 ms` (regressed vs current ~`375-381 ms` band)
- `regime_hybrid` ALMA/ALMA xlarge: `421.01-425.24 ms` (looked improved / neutral)
- `sma_sma` xlarge: `436.76-440.94 ms` (regressed)

Notes:
- Discarded accidental parallel benchmark runs for `regime_hybrid` and `sma_sma` due single-GPU/Cargo lock contention.

### Decision

- Reverted.

## 2026-02-23 Backtest-Kernel Pass Attempt (Regime Helper Non-Null Specialization) - Reverted

### Goal

- Remove per-iteration `regime_filter == nullptr` checks from compile-time regime-specialized helper paths (`REGIME_MODE=1/2`).

### Change attempted

- Added non-null helper variants for regime-only and regime-hybrid paths and routed `desired_sign_mode_f32<1/2>` to them.

### Benchmark outcome (sequential)

- `regime_hybrid` ALMA/ALMA xlarge: `548.92-549.48 ms` (clear regression, ~`+29-30%`)

### Decision

- Reverted (likely code-size/register-pressure side effect).

## 2026-02-23 Nsight Follow-up (Kept `sgn_eps` Zero-Epsilon Fast Path)

### Reports

- baseline: `target/nsight/alma_xlarge_lockstep_m7_norisk_on_pass7.ncu-rep`
- current: `target/nsight/alma_xlarge_sgneps_pass11.ncu-rep`

Profiled kernel:
- `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`

### Nsight snapshot (replay mode)

- Kernel duration: `398.70 ms -> 396.15 ms` (~`0.64%` faster)
- Executed instructions: `239.65B -> 244.52B` (~`+2.03%`)
- Eligible warps/scheduler: `1.420 -> 1.420` (flat)
- `derived__memory_l2_theoretical_sectors_global_excessive`: unchanged (`2.05215`)
- L2 theoretical excessive-sector ratio: `12.62% -> 12.62%` (unchanged)

### Interpretation

- Replay-mode kernel delta is small and not driven by memory/coalescing changes.
- The `sgn_eps` zero-epsilon fast path remains a low-risk kept pass, but its measured benefit appears modest and workload-dependent.

## 2026-02-23 Backtest-Kernel Pass (`eps_rel == 0` Specialization) - Kept (Selective)

### Goal

- Reduce hot-loop overhead in the `lockstep+m7+no-risk` backtest kernel family for the common `eps_rel == 0` case by removing per-iteration epsilon computation/branching via compile-time specialization.

### CUDA / dispatch changes

- Added a templated epsilon helper (`compute_eps_f32<EPS_MODE>`) and extended the shared backtest kernel body template with `EPS_MODE`.
- Added `*_eps0` wrapper kernels for no-risk lockstep+m7 regime-mode variants.
- Host dispatch now uses `*_eps0` wrappers only when:
  - `use_lockstep_m7`
  - no-risk mode
  - `eps_rel <= 0`
  - `VECTORBT_KERNEL_LOCKSTEP_M7_EPS0_SPECIALIZE` is enabled
- Final default policy is **ALMA/ALMA only** (env override still wins).
- The `regime_on` eps0 wrapper was removed after benchmarking showed regression.

### Benchmark outcome (sequential, current-build checks)

- ALMA/ALMA xlarge A/B:
  - `eps0=0`: `400.64-405.86 ms`
  - `eps0=1`: `387.21-395.20 ms` (win)
- `regime_hybrid` ALMA/ALMA xlarge A/B:
  - `eps0=0`: `449.85-456.61 ms`
  - `eps0=1`: `411.26-415.50 ms` (win)
- `breakout_momentum` ALMA/ALMA xlarge A/B:
  - `eps0=0`: `454.92-464.56 ms`
  - `eps0=1`: `453.38-459.72 ms` (neutral)
- `sma_sma` xlarge A/B:
  - `eps0=0`: `384.15-390.82 ms`
  - `eps0=1`: `433.02-437.25 ms` (regression; drove selective ALMA-only default)

### Final default-policy sanity (sequential reruns)

- ALMA/ALMA xlarge: `362.37-367.33 ms` (fast band)
- `regime_hybrid` ALMA/ALMA xlarge: `411.26-415.50 ms`
- `trend_quality` ALMA/ALMA xlarge: `492.03-498.48 ms` (regime-on path left unspecialized; no regression)
- `sma_sma` xlarge: `383.11-388.23 ms` (ALMA-only gate avoids regression)
- ALMA/ALMA large (`58300pairs`): `100.93-101.51 ms` (no material change)
- ALMA/ALMA `xlarge_batched_1gb`: `935.37-938.72 ms` (no material regression)

Notes:
- Discarded multiple obvious `sma_sma` outliers (~`2.79 s`, earlier ~`2.83 s`) and accidental parallel-run artifacts elsewhere in this cycle.

### Nsight status

- No new Nsight A/B was captured for this pass yet.
- This pass is currently kept on benchmark evidence and selective gating; Nsight follow-up remains optional.

### PTX sync / deployment note

- Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` (CUDA 13.0, `compute_89`, explicit MSVC `-ccbin`).
- `VECTORBT_USE_PREBUILT_PTX=1` ALMA/ALMA xlarge sanity: `386.06-392.80 ms`.
- Prebuilt PTX remains slower than JIT/source (`362.37-367.33 ms`), but is improved versus the earlier ~`406-413 ms` prebuilt band.

### Decision

- Kept with selective default (ALMA/ALMA only), and pruned the regressive `regime_on` eps0 specialization.

## 2026-02-23 Follow-up (Prebuilt PTX Divergence Investigation) - Closed

### Goal

- Verify whether the tracked prebuilt PTX performance gap was caused by different PTX artifacts between:
  - normal `nvcc` generation in `build.rs`
  - `VECTORBT_USE_PREBUILT_PTX=1` copy path

### Result

- `crates/app_core/build.rs` writes the same runtime-consumed artifact path in both modes:
  - `OUT_DIR/double_crossover.ptx`
- Controlled rebuild/hash comparison showed byte-identical PTX:
  - `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` SHA256:
    - `AC54F05A8BD699A07E472FB736B85E70C63D070FB8856A6650E831AEF01060E0`
  - `target/ptx_compare/generated_outdir.ptx` SHA256:
    - `AC54F05A8BD699A07E472FB736B85E70C63D070FB8856A6650E831AEF01060E0`
  - `target/ptx_compare/prebuilt_outdir.ptx` SHA256:
    - `AC54F05A8BD699A07E472FB736B85E70C63D070FB8856A6650E831AEF01060E0`

### Artifacts

- Build logs:
  - `target/ptx_compare/build_generated.log` (shows `nvcc` invocation)
  - `target/ptx_compare/build_prebuilt.log` (shows prebuilt copy to `OUT_DIR`)

### Conclusion

- The previously observed prebuilt-vs-non-prebuilt runtime gap is **not** explained by a PTX content mismatch.
- No Nsight profiling was needed for this follow-up because the PTX bytes are identical.

## 2026-02-23 Metrics Layout Follow-up (Full M=7 Row Stride Sweep) - Default Promotion Reverted

### Goal

- Re-test the remaining metrics-layout/store-side hypothesis by sweeping a wider physical stride for full `M=7` metrics rows using existing stride plumbing.

### Change (kept as tooling only)

- Added `VECTORBT_KERNEL_METRICS_FULL_STRIDE` in `crates/app_core/src/gpu_backtest_kernel.rs`:
  - when set to `>= 7`, overrides the physical stride used for full `M=7` tile metrics rows
  - default behavior remains unchanged (`pad8` path still governs normal execution)

### Validation

- `gpu_kernel_topk_v2_matches_forced_legacy` passes after adding the override hook and after reverting the default-promotion attempt.

### Benchmark outcome (decisive same-commit A/B, sequential)

- ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
  - `stride=8`: `394.82-401.11 ms`
  - `stride=16`: `394.03-398.69 ms`
  - Interpretation: neutral on current code state (no repeatable win)
- ALMA/ALMA `xlarge_batched_1gb` (`double_ma/gpu_kernel/alma_alma/200k/xlarge_batched_1gb/370750pairs`)
  - `stride=8`: `958.41-974.16 ms`
  - `stride=16`: `1.0219-1.0397 s`
  - Interpretation: clear regression
- `trend_quality` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/trend_quality/xlarge/370750pairs`)
  - `stride=8`: `497.51-506.69 ms`
  - `stride=16`: `500.70-508.94 ms`
  - Interpretation: neutral / slight regression

### Attempted default policy (reverted)

- Tried adaptive default selection for stride `16` (including workload-shape and VRAM-budget-aware variants).
- Follow-up benchmark matrices were inconsistent/noisy and did not justify a default change.
- Reverted the default-promotion logic and kept only the explicit env override for future experiments.

### Conclusion

- No new default metrics-layout optimization was kept from this pass.
- The deeper metrics-layout opportunity remains the larger SoA/hybrid refactor (still open).
