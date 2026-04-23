# Xlarge Cache Reuse Probe Findings

Date: 2026-04-04

## Purpose

This note records the execution result of `xlarge_cache_reuse_probe_implementation_plan_2026-04-04.md`.

The branch goal was narrow:

1. force smaller exact-search tiles on `double_ma`, `alma/alma`, `xlarge`
2. measure whether the hot lockstep kernel becomes more cache-friendly
3. keep the branch only if wall time also improves

## Scope

Surfaces measured:

- `double_ma`, `alma/alma`, `xlarge`, `synth`, `metric_contract=full17`, `shortlist_slice_mode=none`
- `double_ma`, `alma/alma`, `xlarge`, `synth`, `metric_contract=trade22`, `shortlist_slice_mode=none`
- `double_ma`, `alma/alma`, `large`, `synth`, `metric_contract=full17`, `shortlist_slice_mode=none` as regression control

No product code changed during this branch.

## Environment

- GPU: `NVIDIA GeForce RTX 4090`
- Driver: `595.71`
- Nsight Compute: `2026.1.0`
- Shell: elevated PowerShell
- Branch / commit at probe start:
  - branch: `main`
  - commit: `cdf6bcca1bb4df1ee69b6a4725d67cd05dc74ce0`

## Artifacts

Primary artifacts are under `target/cache_probe/`.

Important summaries:

- `target/cache_probe/direct_runner_tile_sweep_summary.csv`
- `target/cache_probe/ncu_cache_summary.csv`
- `target/cache_probe/xlarge_full17_budget_sweep_summary.json`
- `target/cache_probe/xlarge_trade22_budget_sweep_summary.json`
- `target/cache_probe/baseline_and_control_summary.json`

Raw captures are also preserved there:

- direct-runner stdout/stderr per surface and budget
- NCU raw CSV captures
- NCU logs

## Direct-Runner Results

Observed default full-fit baselines:

- `xlarge/full17`: `349 ms`, `pf_tile=496/496`, `ps_tile=991/991`
- `xlarge/trade22`: `385 ms`, `pf_tile=496/496`, `ps_tile=991/991`
- `large/full17`: `99 ms`, `pf_tile=196/196`, `ps_tile=391/391`

Smaller-budget sweep results:

| Surface | Budget | Effective debug budget | Tile choice | Wall time |
| --- | ---: | ---: | --- | ---: |
| `xlarge/full17` | default | `19114 MiB` | `pf 496/496`, `ps 991/991` | `349 ms` |
| `xlarge/full17` | `1200` | `688 MiB` | `pf 399/496`, `ps 399/991` | `555 ms` |
| `xlarge/full17` | `1000` | `488 MiB` | `pf 272/496`, `ps 272/991` | `587 ms` |
| `xlarge/full17` | `800` | `288 MiB` | `pf 144/496`, `ps 143/991` | `1359 ms` |
| `xlarge/full17` | `600` | `88 MiB` | `pf 496/496`, `ps 1/991` | `49427 ms` |
| `xlarge/full17` | `400` | `200 MiB` | `pf 87/496`, `ps 86/991` | `3013 ms` |
| `xlarge/trade22` | default | `19114 MiB` | `pf 496/496`, `ps 991/991` | `385 ms` |
| `xlarge/trade22` | `1200` | `688 MiB` | `pf 397/496`, `ps 397/991` | `552 ms` |
| `xlarge/trade22` | `1000` | `488 MiB` | `pf 271/496`, `ps 271/991` | `604 ms` |
| `xlarge/trade22` | `800` | `288 MiB` | `pf 144/496`, `ps 143/991` | `1308 ms` |
| `xlarge/trade22` | `600` | `88 MiB` | `pf 496/496`, `ps 1/991` | `47787 ms` |
| `xlarge/trade22` | `400` | `200 MiB` | `pf 87/496`, `ps 86/991` | `2914 ms` |
| `large/full17` | default | `19114 MiB` | `pf 196/196`, `ps 391/391` | `99 ms` |
| `large/full17` | `1200` | `688 MiB` | `pf 196/196`, `ps 391/391` | `96 ms` |

The best smaller-budget candidate on both `xlarge` surfaces was `1200 MiB`, and it was still materially slower than baseline.

## NCU Cache Results

`MemoryWorkloadAnalysis` was captured for the hot lockstep kernels:

- `regex:^double_cross_backtest_tm_flex_f32_lockstep_time_m17` for `full17`
- `regex:^double_cross_backtest_tm_flex_f32_lockstep_time_m22` for `trade22`

Summary:

| Surface | Budget | L1/TEX Hit Rate | L2 Hit Rate | Compute/Memory Access Throughput | SM Memory Throughput |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xlarge/full17` | default | `34.70%` | `76.92%` | `26.48%` | `18.73%` |
| `xlarge/full17` | `1200` | `66.42%` | `96.63%` | `11.67%` | `14.31%` |
| `xlarge/trade22` | default | `36.78%` | `81.76%` | `24.49%` | `17.82%` |
| `xlarge/trade22` | `1200` | `67.34%` | `97.25%` | `11.19%` | `13.98%` |
| `large/full17` | default | `63.39%` | `96.72%` | `11.25%` | `12.33%` |
| `large/full17` | `1200` | `63.39%` | `97.38%` | `11.24%` | `12.38%` |

## Interpretation

The probe produced a clear negative result for product implementation:

1. forcing smaller tiles does improve cache hit rates substantially on `xlarge`
2. the same runs are still much slower end to end
3. the `large/full17` control remains full-fit and effectively unchanged

So the measured problem is not:

- “cache hit rate is low, therefore smaller tiles will improve performance”

Instead, the evidence says:

- smaller tiles create enough extra tiling overhead, launch repetition, or related work inflation that the route becomes slower even though the lockstep kernel looks more cache-friendly

This means the cache-fit hypothesis does not clear the branch promotion gate.

It also means this branch does not justify:

- a new `choose_tiles()` cache-fit heuristic
- broad price-data persistence work
- new L2 persistence policy work

## Decision

Decision: `NO-GO`

Reason:

- the branch achieved the intended tile reduction and the intended cache-metric improvement
- it did not improve wall time on the primary target
- the contrast case behaved the same way
- the regression control did not indicate a hidden large-surface benefit

Therefore:

1. stop this lane without code changes
2. preserve the result as a closed measurement branch
3. do not promote a tile-choice heuristic from these measurements

## Functionality Impact

None.

This branch was measurement-only. No application functionality was removed or changed, and no product code was retained from this probe.

## Follow-On

If this area is revisited later, the next question should not be “can we force smaller tiles?”

The more appropriate follow-on question would be:

- why does improved cache residency still lose to the current full-fit route on `xlarge`?

That is a different architecture question than the one tested here.
