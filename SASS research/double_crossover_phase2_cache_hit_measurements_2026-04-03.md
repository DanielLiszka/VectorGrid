# Double Crossover Phase 2 Cache Hit Measurements

Date: 2026-04-03

## Purpose

Measure the main `double_ma` `alma/alma` exact backtest kernels for `large` and `xlarge` truth surfaces across the current row-contract variants (`auto`, `full17`, `trade22`) and record the L1/TEX and L2 hit rates exposed by Nsight Compute.

This pass answers a narrower question than the broader replay/dataflow work: does the current profiler evidence show obviously poor cache behavior on the main lockstep backtest kernels, and does it justify a cache-policy follow-on such as explicit persisting-L2 experiments?

## Method

- Tool: Nsight Compute 2026.1 CLI
- Section: `MemoryWorkloadAnalysis`
- Target binary: `target/release/profile_gpu_dynamic_runner.exe`
- Route shape:
  - `--strategy double_ma`
  - `--fast-ma alma`
  - `--slow-ma alma`
  - `--adx-mode off`
  - `--candle-shape synth`
  - `--dynamic-mode off`
  - `--cuda-graphs off`
  - `--heatmap-bins 0`
  - `--shortlist-slice-mode none`
- Kernel filter: `regex:^double_cross_backtest_tm_flex_f32`
- Profiling scope: one filtered backtest-kernel launch per surface

Artifacts:

- JSON summary: [cache_l1l2_alma_double_ma_summary_2026-04-03.json](C:/Rust%20Projects/ta_desktop_demo/target/nsight/cache_l1l2_alma_double_ma_summary_2026-04-03.json)
- CSV summary: [cache_l1l2_alma_double_ma_summary_2026-04-03.csv](C:/Rust%20Projects/ta_desktop_demo/target/nsight/cache_l1l2_alma_double_ma_summary_2026-04-03.csv)
- Per-surface reports and extracted section text under [target/nsight](C:/Rust%20Projects/ta_desktop_demo/target/nsight)

## Results

| Workload | Contract | Kernel | L1/TEX Hit Rate | L2 Hit Rate | L2 Persisting Size | Memory Throughput | Mem Busy |
|---|---|---|---:|---:|---:|---:|---:|
| `large` | `auto` | `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0_nocomm` | `63.40%` | `97.01%` | `14.16 MB` | `15.18 GB/s` | `11.98%` |
| `large` | `full17` | `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm` | `63.38%` | `96.92%` | `14.16 MB` | `15.37 GB/s` | `11.51%` |
| `large` | `trade22` | `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm` | `63.38%` | `96.41%` | `14.16 MB` | `15.39 GB/s` | `11.20%` |
| `xlarge` | `auto` | `double_cross_backtest_tm_flex_f32_lockstep_time_m10_regime_off_norisk_eps0_nocomm` | `42.41%` | `87.33%` | `14.16 MB` | `224.56 GB/s` | `28.88%` |
| `xlarge` | `full17` | `double_cross_backtest_tm_flex_f32_lockstep_time_m17_regime_off_norisk_eps0_nocomm` | `36.44%` | `70.21%` | `14.16 MB` | `318.78 GB/s` | `28.12%` |
| `xlarge` | `trade22` | `double_cross_backtest_tm_flex_f32_lockstep_time_m22_regime_off_norisk_eps0_nocomm` | `38.84%` | `84.54%` | `14.16 MB` | `176.19 GB/s` | `23.06%` |

## Interpretation

### What the current evidence supports

- The `large` exact lockstep kernels already have very high L2 hit rates, roughly `96.4%` to `97.0%`.
- The `large` surfaces also keep stable L1/TEX hit rates near `63.4%` across `m10`, `m17`, and `m22`.
- The `xlarge` surfaces are materially more cache-stressed than `large`.
- The worst `xlarge` cache behavior in this set is `full17`, with `36.44%` L1/TEX hit rate and `70.21%` L2 hit rate.
- `xlarge trade22` is noticeably better than `xlarge full17` on both L1/TEX and L2 hit rate.

### What the current evidence does not prove

- These measurements do not isolate price arrays from other read-mostly inputs or derived series.
- The reported `L2 Persisting Size = 14.16 MB` is not proof that the important price working set is already explicitly pinned in a useful way; it is only the section-reported active persisting-L2 size for these launches.
- The measurements do not prove that an explicit L2 persistence policy would improve end-to-end runtime.
- The measurements do not identify whether the remaining `xlarge` cache stress is due to:
  - price-array reuse limits,
  - derived-indicator footprint,
  - row-contract width,
  - or simple working-set growth at `xlarge`.

## Readout

The current cache evidence weakens the idea that the main `large` `alma/alma` route is leaving obvious performance on the table because price data is not staying resident in cache. On `large`, the main lockstep kernels already hit L2 at roughly `96%` to `97%`.

The more credible cache-oriented follow-on, if one is still desired for research purposes, is an `xlarge`-focused feasibility pass rather than a broad product change. The clean next branch would be:

1. Use `MemoryWorkloadAnalysis_Tables` or raw hit/miss counters to separate global-load and global-store cache behavior.
2. Keep the scope on `xlarge` `alma/alma`, especially `full17`.
3. Only then test an explicit persisting-L2 or relayout intervention if the more granular counters show a cache-policy story rather than a broader dataflow or working-set story.

## Bottom Line

- `large`: current cache behavior already looks strong.
- `xlarge`: cache stress is real, but the evidence is not yet specific enough to justify a product change aimed at price-data persistence.
- The current measurements support cache as a targeted `xlarge` research hypothesis, not as an already-proven next optimization.
