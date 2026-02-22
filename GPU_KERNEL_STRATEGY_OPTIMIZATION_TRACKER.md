# GPU Kernel Strategy Optimization Tracker

Last updated: 2026-02-22

## Purpose

Track:
- Which strategy profiles are eligible for VRAM-resident GPU-kernel execution.
- Why a request falls back from `Backend::GpuKernel`.
- Optimization progress per GPU-kernel strategy profile.

This document is the single tracker for strategy-level GPU-kernel optimization work.

## Scope And Assumptions

- Scope is the double-MA GPU kernel backend path (`Backend::GpuKernel`) in `crates/app_core/src/gpu_backtest_kernel.rs`.
- Composite engines (`CompositeCpu`, `CompositeMo`) are not GPU-kernel main execution paths; they can use GPU only for optional prescan.
- MA selection policy assumption for performance work:
  - User selects MAs that are known CUDA winners from `gpu_speedups.csv`.
  - Additional MAs can be added later as VectorTA improves.
- HalfTrend can be technically enabled in GPU-kernel regime flow, but is currently treated as CPU-preferred for performance policy until proven otherwise.

## GPU-Kernel Eligibility Matrix

| Strategy Profile | Main Path Uses GPU Kernel | Status |
| --- | --- | --- |
| `double_ma` baseline crossover | Yes | Supported |
| `trend_quality` (`composite_trend` test label) | Yes | Supported |
| `regime_hybrid` (trend/range switching) | Yes | Supported |
| `breakout_momentum` | Yes | Supported |
| `halftrend_filter` variant | Yes (technical), CPU-preferred by perf policy | Conditional |
| `CompositeCpu` (`HardGate`, `EntryCoreExitOverlay`, `TrendPullbackReversion`) | No (prescan-only GPU) | CPU main path |
| `CompositeMo` | No (prescan-only GPU) | CPU main path |

## Kernel Fallback/Disqualifier Rules

GPU-kernel execution is disallowed or downgraded when any of the following is true:

- `exit_mode = KaufmanStop`.
- Execution overlays are enabled:
  - cooldown bars,
  - signal persistence > 1,
  - trailing stop.
- Selected MA pair is not supported by VRAM kernel MA path.
- Regime filters are requested but kernel regime filters are disabled via env.
- Invalid strategy combinations:
  - `regime_hybrid_enable` + `breakout_momentum_enable`,
  - `regime_hybrid_enable` + `halftrend_filter`.

## Current Optimization Snapshot (Baseline Strategy Bench)

Reference benchmark family:
- `double_ma/gpu_kernel/alma_alma/200k/*`

Most recent observed results after launch-policy update:
- `small/1590pairs`: ~`86 ms`
- `medium/14150pairs`: ~`97 ms`
- `large/58300pairs`: ~`100 ms`
- `xlarge/370750pairs`: ~`1.86 s`
- `xlarge_batched_1gb/370750pairs`: ~`0.99 s`
- `xxlarge/1491500pairs`: ~`7.3-7.4 s`
- `xxlarge_batched_1gb/1491500pairs`: ~`3.52 s`

Nsight reports:
- `target/nsight/alma_large_launch_policy_v2.ncu-rep`
- `target/nsight/alma_xlarge_launch_policy_v2.ncu-rep`

Primary remaining bottleneck:
- Uncoalesced global memory access in `double_cross_backtest_tm_flex_f32`.

## Strategy Optimization Board

Legend:
- `Not Started`
- `In Progress`
- `Blocked`
- `Done`

| Strategy Profile | Status | Baseline Bench Captured | Nsight Captured | Latest Bottleneck | Next Step |
| --- | --- | --- | --- | --- | --- |
| `double_ma` baseline | In Progress | Yes | Yes | Uncoalesced global memory access (~83% excessive sectors) | Source-counter-guided pass (target dominant load/store site) |
| `trend_quality` | Not Started | No | No | TBD | Add dedicated bench filter + capture baseline |
| `regime_hybrid` | Not Started | No | No | TBD | Add dedicated bench filter + capture baseline |
| `breakout_momentum` | Not Started | No | No | TBD | Add dedicated bench filter + capture baseline |
| `halftrend_filter` variant | Not Started (CPU-preferred policy) | No | No | TBD | Validate if GPU can beat CPU on representative workloads |

## Per-Strategy Checklist Template

Copy this block for each strategy pass.

```md
### <strategy_name> - Pass <N>
- Date:
- Owner:
- Goal:
- Workload:
  - Candles:
  - Fast/slow ranges:
  - MA pair:
  - Objective:
- Baseline:
  - Criterion:
  - Nsight report:
- Change implemented:
- Result:
  - Criterion delta:
  - Nsight deltas:
- Correctness checks:
  - Tests:
  - Parity checks:
- Decision:
  - Keep / Revert
- Follow-up:
```

## Standard Commands

Criterion benchmark example:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs"
```

Nsight capture example:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 `
  -BenchFilter "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs" `
  -OutputPrefix target/nsight/alma_large_passX `
  -LaunchCount 40
```

## Change Log

### 2026-02-22

- Created tracker document.
- Registered current supported GPU-kernel strategy profiles and fallback rules.
- Recorded latest baseline benchmark and Nsight snapshot from launch-policy optimization pass.
- Coalescing-focused pass attempt completed:
  - Tested forced launch geometry (`block_x=384`) and confirmed regression.
  - Implemented optional TM stride padding experiment (`VECTORBT_KERNEL_TM_PAD`), then benchmarked/profiled it.
  - Result: no reduction in uncoalesced global access, with regression when enabled.
  - Decision: keep TM padding off by default; continue with source-counter-targeted optimization.
