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

Most recent observed results after lockstep-time + adaptive `M=7` specialization:
- `small/1590pairs`: ~`86 ms` (unchanged reference)
- `medium/14150pairs`: ~`97 ms` (unchanged reference)
- `large/58300pairs`: ~`99.19-99.54 ms`
- `xlarge/370750pairs`: ~`395.67-401.13 ms`
- `xlarge_batched_1gb/370750pairs`: ~`946.80-957.33 ms`
- `xxlarge/1491500pairs`: ~`1.552-1.560 s`
- `xxlarge_batched_1gb/1491500pairs`: ~`3.621-3.662 s`

Nsight reports:
- `target/nsight/alma_xlarge_no_lockstep_no_dedup_pass2.ncu-rep`
- `target/nsight/alma_xlarge_fast_warp_no_lockstep_pass2.ncu-rep`
- `target/nsight/alma_xlarge_lockstep_pass2.ncu-rep`
- `target/nsight/alma_xlarge_lockstep_m7_pass4.ncu-rep`

Primary remaining bottleneck:
- Store-side memory layout (`metrics_out` AoS strided writes) and residual control-flow imbalance.

## Strategy Profile Baselines (ALMA/ALMA xlarge, 200k)

Benchmark family added:
- `double_ma/gpu_kernel_strategy/alma_alma/200k/*/xlarge/370750pairs`

Criterion snapshots:
- `trend_quality`: ~`487.44-496.02 ms`
- `regime_hybrid`: ~`492.59-498.95 ms`
- `breakout_momentum`: ~`451.24-460.02 ms`

Nsight reports:
- `target/nsight/trend_quality_alma_xlarge_pass1.ncu-rep`
- `target/nsight/regime_hybrid_alma_xlarge_pass1.ncu-rep`
- `target/nsight/breakout_momentum_alma_xlarge_pass1.ncu-rep`

Common observations (xlarge profiled backtest kernel = `double_cross_backtest_tm_flex_f32_lockstep_time_m7`):
- All three strategy profiles currently hit the same optimized backtest kernel path.
- `UncoalescedGlobalAccess` is ~`11%` excessive sectors (same remaining class of issue as baseline strategy).
- Scheduler eligibility remains healthy relative to pre-lockstep baselines (`Eligible warps/scheduler` ~`1.38-1.50`).
- Runtime differences across profiles mainly track extra strategy logic / executed instruction count, not a different core kernel bottleneck.

## Strategy Optimization Board

Legend:
- `Not Started`
- `In Progress`
- `Blocked`
- `Done`

| Strategy Profile | Status | Baseline Bench Captured | Nsight Captured | Latest Bottleneck | Next Step |
| --- | --- | --- | --- | --- | --- |
| `double_ma` baseline | In Progress | Yes | Yes | Load-side coalescing largely addressed; store-side AoS writes still inefficient | Prototype SoA/staged metric write path and re-profile |
| `trend_quality` | In Progress | Yes | Yes | Same backtest kernel bottleneck class as baseline (residual uncoalesced/global + metric/store overhead) | Reuse baseline kernel passes, then profile regime-filter-specific overhead |
| `regime_hybrid` | In Progress | Yes | Yes | Same backtest kernel bottleneck class; parity validation currently failing | Triage parity mismatch before strategy-specific perf tuning beyond shared kernel work |
| `breakout_momentum` | In Progress | Yes | Yes | Same backtest kernel bottleneck class in profiled backtest stage | Profile full pipeline split (regime build vs backtest kernel) and target dominant stage |
| `halftrend_filter` variant | Not Started (CPU-preferred policy) | No | No | TBD | Validate if GPU can beat CPU on representative workloads |

## Validation Notes (Current)

- Known failing parity test (CPU vs GPU):
  - Command:
    - `cargo test -p ta_app_core gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies --features cuda,cuda-backtest-kernel -- --nocapture`
  - Current failure:
    - `pnl delta too large for regime_hybrid: 2.8055098289775566`
    - panic site: `crates/app_core/src/double_ma.rs:7218`
- Passing regression check in current state:
  - `cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture`
- Policy for ongoing optimization passes:
  - Treat performance wins as provisional until this parity mismatch is triaged/resolved or explicitly baselined as pre-existing.
  - Re-run at least one correctness/regression test after each major kernel pass.

## Autonomous Iteration Checklist (Backtest Optimization)

Use this loop for each optimization target and each pass (GPU-kernel backtest path only; do not modify `vector-ta`).

1. Define one target and one hypothesis.
2. Capture baseline benchmark(s) for at least:
   - `alma_alma xlarge`
   - one small/medium control (or `large`)
   - one batched workload if target can affect batching behavior
3. Capture/compare Nsight (`ncu`) if the target concerns kernel execution behavior.
4. Implement one isolated code change (or one tightly-coupled change set).
5. Rebuild and run at least one regression/correctness test.
6. Re-run the same benchmark set sequentially (never parallel on one GPU).
7. Keep or revert using explicit criteria:
   - keep only if xlarge/xxlarge improves and large does not materially regress
   - revert if regression is clear on representative workloads
8. Record benchmark times, Nsight report names, and decision in this tracker.
9. Promote runtime default only after a stable keep decision.
10. Move to the next pass on the same target, or close the target and advance.

## Active Target Queue (Current)

### Target A - Load-side coalescing / lane synchronization

- Status: Partially complete (major gains captured with lockstep-time pass).
- Latest kept result:
  - `alma_alma xlarge`: ~`1.85 s -> ~0.45 s`
  - `UncoalescedGlobalAccess`: `82% -> 13%` excessive sectors (same-commit Nsight)
- Remaining note:
  - A residual global load still dominates most of the remaining excessive sectors.
- Recent sub-pass outcome:
  - `lockstep + fast-warp-dedup` combo remained regressive and was reverted (see Change Log).

### Target B - Metrics output stage (metric branching/finalization + store-side layout)

- Status: In Progress
- Why this is next:
  - Nsight still flags global store access inefficiency.
  - Source counters show end-of-kernel `metrics_out` stores as repeated strided `STG.E` instructions.
- Scope note:
  - A full physical stride change impacts multiple downstream kernels and host indexing.
  - Prefer staged, reversible experiments first before large refactors.
- Current staged result:
  - Pass 1 (`M=7` specialization) is kept with adaptive runtime gating.
  - Pass 2 (`M=7` padded AoS stride: physical `7 -> 8` floats) is implemented as an opt-in experiment (`VECTORBT_KERNEL_METRICS_PAD8=1`).
    - Strong ALMA workload wins were observed.
    - Small `sma_sma` xlarge regression was observed.
    - Decision: keep code path, leave default off pending broader coverage / adaptive gating.
  - Full store-layout refactor (physical stride / SoA) remains open.
- Baseline references for this target:
  - `target/nsight/alma_xlarge_lockstep_pass2.ncu-rep`
  - `double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs` ~`448-456 ms` (pre-pass-1 reference)

### Target C - Strategy-profile coverage and attribution (trend/regime/breakout)

- Status: Baseline/Nsight captured for xlarge (ALMA/ALMA)
- Goal:
  - Ensure all GPU-kernel strategy profiles are tracked with reproducible benchmark and Nsight baselines.
  - Separate shared backtest-kernel wins from strategy-specific precompute overhead.
- Current status:
  - `trend_quality`, `regime_hybrid`, `breakout_momentum` now have xlarge benchmarks and Nsight reports.
  - All three profile captures show the same optimized backtest kernel (`lockstep_time_m7`) and similar source-counter signature.
- Next steps:
  - Add pipeline-stage attribution pass (time regime-filter build vs backtest kernel launch path).
  - For `regime_hybrid`, pair any perf changes with parity triage due existing CPU/GPU mismatch.

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
- Source-counter-guided pass completed:
  - Added fast-MA warp dedup kernel variant (`double_cross_backtest_tm_flex_f32_fast_warp_dedup`).
  - Runtime selection uses dedup on larger workloads (`launch_pairs >= 150000`) as an optional path.
  - Result:
    - `large/58300pairs`: remains ~`100 ms` range.
    - `xlarge/370750pairs`: ~`1.85 s -> ~0.74 s` on benchmark runs.
  - Nsight (`target/nsight/alma_xlarge_fast_warp_dedup_pass1.ncu-rep`) shows:
    - `No Eligible` dropped substantially, `Eligible warps/scheduler` increased.
    - Uncoalesced access ratio improved (still non-trivial).
- Source-counter-guided pass 2 completed (lockstep time-index kernel):
  - Added `double_cross_backtest_tm_flex_f32_lockstep_time`.
  - Large-workload default now prefers lockstep kernel:
    - `VECTORBT_KERNEL_LOCKSTEP_TIME=1`
    - `VECTORBT_KERNEL_LOCKSTEP_TIME_MIN_PAIRS=150000`
  - Fast dedup remains available but default-off:
    - `VECTORBT_KERNEL_FAST_WARP_DEDUP=0`
  - Result:
    - `xlarge/370750pairs`: ~`1.85 s -> ~0.45 s` (default path)
    - `xxlarge/1491500pairs`: ~`2.27 s -> ~1.67 s`
  - Same-commit Nsight (`*_pass2.ncu-rep`) shows:
    - `No Eligible`: `86.11% -> 38.44%`
    - `Eligible warps/scheduler`: `0.16 -> 1.26`
    - `UncoalescedGlobalAccess`: `82% -> 13%` excessive sectors.
  - Validation status:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes.
    - CPU/GPU parity test for `regime_hybrid` still fails (see Validation Notes); not yet confirmed as introduced by this pass.
- Source-counter-guided pass 3 attempts (same target: residual load-side access):
  - Attempt A: `lockstep + fast-warp-dedup` combined kernel variant (opt-in runtime path).
    - Clean sequential benchmark (`alma_alma xlarge`):
      - baseline lockstep: ~`451.9-455.9 ms`
      - combo enabled: ~`615.7-617.0 ms` (regression)
  - Attempt B: optimize dedup helper with uniform-warp fast path (`__all_sync` + broadcast) before `__match_any_sync`.
    - Clean sequential benchmark (`alma_alma xlarge`, no lockstep, dedup on):
      - prior fast-dedup reference: ~`714-719 ms`
      - new helper: ~`755.7-764.1 ms` (regression)
    - Combo path after helper change also regressed further (~`640.6-641.8 ms`).
  - Decision:
    - Reverted pass-3 experimental code (combo kernel selector + helper fast path).
    - Keep lockstep-only default path from pass 2.
  - Note:
    - Discarded one set of parallel benchmark runs due GPU contention; only sequential runs above were used for decisions.
- Metrics-stage pass 1 completed (adaptive `M=7` specialization):
  - Added specialized kernels for the common full-metrics path:
    - `double_cross_backtest_tm_flex_f32_m7`
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
  - Runtime selection policy (default-on):
    - `VECTORBT_KERNEL_M7_SPECIALIZE=1`
    - `VECTORBT_KERNEL_M7_SPECIALIZE_MIN_PAIRS=300000`
    - disabled automatically for multi-tile/batched workloads
  - Benchmark results (clean sequential):
    - `alma_alma large/58300pairs`: ~`99.19-99.54 ms` (noise-level change)
    - `alma_alma xlarge/370750pairs`: ~`395.67-401.13 ms` (improved vs ~`448-456 ms`)
    - `alma_alma xlarge_batched_1gb/370750pairs`: ~`946.80-957.33 ms` (improved vs recent lockstep-only ~`~0.99 s` runs)
    - `alma_alma xxlarge/1491500pairs`: ~`1.552-1.560 s` (improved vs ~`1.666-1.674 s`)
    - `alma_alma xxlarge_batched_1gb/1491500pairs`: ~`3.621-3.662 s` (no material change)
    - `sma_sma xlarge/370750pairs`: ~`399.09-405.72 ms` (improved)
  - Nsight (`target/nsight/alma_xlarge_lockstep_m7_pass4.ncu-rep`) vs lockstep pass2:
    - Kernel duration: `450.94 ms -> 427.69 ms`
    - Eligible warps/scheduler: `1.26 -> 1.37`
    - `No Eligible`: `38.44% -> 37.79%`
    - Executed instructions: `268.8B -> 249.3B`
    - Uncoalesced access ratio remained ~`13%` (same source-counter diagnosis)
  - Prebuilt PTX status:
    - Synced `double_crossover.prebuilt.ptx` with `m7` kernels present.
    - `VECTORBT_USE_PREBUILT_PTX=1` works; `alma_alma xlarge` observed ~`417.82-424.50 ms`.
- Validation status:
  - `gpu_kernel_topk_v2_matches_forced_legacy` passes.
  - Known parity failure for `regime_hybrid` CPU vs GPU remains unchanged (see Validation Notes).
- Valid-pair-map threshold probe for xlarge (policy attempt, not promoted):
  - Hypothesis:
    - Raising `VECTORBT_KERNEL_VALID_PAIR_MAP_MAX_PAIRS` to include xlarge (`370750` pairs) may reduce work enough to improve overall runtime.
  - Results (same commit, sequential):
    - `alma_alma xlarge`, `MAX_PAIRS=400000`: small improvement in one A/B sequence (~`1-3%`)
    - `sma_sma xlarge`, `MAX_PAIRS=400000`: regression (~`1-3%`)
    - `alma_alma xlarge_batched_1gb`, `MAX_PAIRS=400000`: regression (~`4-5%`)
  - Decision:
    - Do not promote a larger global valid-pair-map threshold.
    - Keep existing threshold policy.
- Strategy pipeline attribution pass 1 (coarse stage timing, opt-in) - kept instrumentation:
  - Added `VECTORBT_KERNEL_PROFILE_PIPELINE=1` (GPU-kernel optimize path only).
  - Profiling mode behavior:
    - prints per-run coarse stage timings
    - disables CUDA graph replay for readability
    - adds stream sync points for stage attribution (profiling-only overhead)
  - Current stage buckets:
    - `host_prep`
    - `tile_prepare`
    - `tile_score` (includes backtest + heatmap launches)
    - `tile_finalize`
    - `final_finalize`
  - `regime_hybrid` strategy profile (`alma_alma`, xlarge, `topk=50`, `heatmap_bins=64`) findings:
    - total ~`500-530 ms`
    - `tile_score`: ~`94%`
    - `tile_prepare`: ~`6%`
    - all other tracked buckets: negligible in this profile
  - Implication:
    - Next optimization effort should continue targeting `tile_score` (backtest kernel and/or heatmap kernel), not host-side assembly.
- Strategy pipeline attribution pass 2 (tile_score split into backtest vs heatmap) - kept instrumentation:
  - Change:
    - Refined `VECTORBT_KERNEL_PROFILE_PIPELINE=1` timing buckets in `crates/app_core/src/gpu_backtest_kernel.rs` to split `tile_score` into:
      - `backtest`
      - `heatmap`
      - `score_other` (residual `tile_score - backtest - heatmap`)
  - Profiling notes:
    - Profiling mode intentionally adds synchronization and disables CUDA graph replay for readability.
    - Do not compare profiling-mode benchmark totals against normal benchmark runs.
  - `regime_hybrid` strategy profile (`alma_alma`, xlarge, `topk=50`, `heatmap_bins=64`) findings (sequential runs; rechecked after later kernel passes):
    - total: ~`461-489 ms`
    - `tile_prepare`: ~`11-14 ms` (~`2.3-2.8%`)
    - `tile_score`: ~`448-477 ms` (~`97%`)
    - `tile_score` split:
      - `backtest`: effectively ~`100%` of `tile_score` (for example ~`450.86 ms` vs `heatmap ~0.08 ms`)
      - `heatmap`: ~`0.07-0.16 ms` (negligible)
      - `score_other`: ~`0.001-0.002 ms` (negligible)
    - `tile_finalize` / `final_finalize`: negligible in this profile
  - Implication:
    - Heatmap/postprocessing is not a worthwhile target for this workload.
    - Continue optimization passes on the backtest kernel itself.
- Backtest-kernel micro-pass attempt (flag-hoist cleanup) - reverted:
  - Change attempted:
    - Hoisted repeated constant checks (`trade_on_next`, `no_flip`, `use_commission`, signed-exposure flag) out of the inner loops in `double_crossover.cu`.
  - Result:
    - `alma_alma xlarge`, `sma_sma xlarge`, and `regime_hybrid` xlarge strategy-profile benchmarks all landed within noise / no clear improvement.
  - Decision:
    - Reverted to keep the kernel baseline clean before the next pass.
- Strategy-profile baseline coverage pass completed:
  - Added Criterion benchmark family in `crates/app_core/benches/double_ma_opt.rs`:
    - `double_ma/gpu_kernel_strategy/<ma_pair>/200k/<strategy>/<workload>/<pairs>pairs`
  - Added bench input generators for strategy-profile-friendly candles:
    - `make_trending_candles`, `make_breakout_candles`
  - ALMA/ALMA xlarge benchmark baselines captured:
    - `trend_quality`: ~`487.44-496.02 ms`
    - `regime_hybrid`: ~`492.59-498.95 ms`
    - `breakout_momentum`: ~`451.24-460.02 ms`
  - Nsight xlarge baselines captured:
    - `target/nsight/trend_quality_alma_xlarge_pass1.ncu-rep`
    - `target/nsight/regime_hybrid_alma_xlarge_pass1.ncu-rep`
    - `target/nsight/breakout_momentum_alma_xlarge_pass1.ncu-rep`
  - Common Nsight observation:
    - All three currently profile the same optimized backtest kernel (`double_cross_backtest_tm_flex_f32_lockstep_time_m7`)
    - Residual `UncoalescedGlobalAccess` remains ~`11%` excessive sectors
  - Decision:
    - Mark strategy profiles as `In Progress` (baseline + Nsight captured)
    - Continue with shared-kernel and pipeline-attribution optimization passes.
- Metrics-stage/store-layout pass 2 completed (padded AoS stride experiment, opt-in):
  - Goal:
    - Improve store/read alignment for the common full-metrics tile path without a full SoA refactor.
  - Change:
    - Added explicit `metrics_stride` plumbing across the backtest kernel and tile consumers (`heatmap_update`, pair top-k selectors, `gather_topk_pairs_tm_f32`).
    - Added opt-in runtime knob:
      - `VECTORBT_KERNEL_METRICS_PAD8=1` pads full `M=7` tile rows to physical stride `8` floats.
    - Logical metrics count remains unchanged; only `scratch.d_tile` physical layout changes.
  - Benchmark results (same commit, sequential unless noted):
    - ALMA/ALMA `xlarge/370750pairs` A/B:
      - `pad8=0`: `417.13-423.38 ms`
      - `pad8=1`: `391.84-399.93 ms` (improved, ~`5-7%`)
    - ALMA/ALMA `large/58300pairs` (`pad8=1`): `100.14-103.86 ms` (no material regression vs prior ~`100 ms`)
    - ALMA/ALMA `xlarge_batched_1gb/370750pairs` (`pad8=1`): `893.44-898.32 ms` (improved vs recent ~`947-957 ms`)
    - ALMA/ALMA `xxlarge/1491500pairs` (`pad8=1`): `1.5562-1.5721 s` (roughly in line with recent ~`1.552-1.560 s`, slight noise)
    - `sma_sma xlarge/370750pairs` same-commit A/B:
      - `pad8=0`: `406.32-412.11 ms`
      - `pad8=1`: `410.26-416.60 ms` (small regression, ~`~1%`)
    - Strategy profile check (`regime_hybrid`, ALMA/ALMA xlarge, `pad8=1`):
      - `497.32-504.82 ms` (no clear change vs prior baseline band)
    - Prebuilt PTX sanity (`VECTORBT_USE_PREBUILT_PTX=1`, `pad8=1`, ALMA xlarge):
      - `395.48-398.56 ms`
  - Nsight (`target/nsight/alma_xlarge_lockstep_m7_pad8_pass5.ncu-rep`) vs `target/nsight/alma_xlarge_lockstep_m7_pass4.ncu-rep`:
    - Kernel duration: `427.69 ms -> 408.99 ms`
    - Executed instructions: effectively unchanged (~`249.3B`)
    - Eligible warps/scheduler: ~`1.37` (flat)
    - `No Eligible`: ~`37.8%` (flat)
    - `UncoalescedGlobalAccess`: still reported at ~`13%` excessive sectors
  - Interpretation:
    - The win is consistent with improved memory transaction alignment (same instruction count, lower runtime).
    - Benefit is workload-dependent; not a universal improvement.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after stride-plumbing changes.
    - Known `regime_hybrid` parity mismatch remains tracked separately (not addressed here).
  - Decision:
    - Keep code path and PTX updates.
    - Leave `VECTORBT_KERNEL_METRICS_PAD8` default **off** pending adaptive gating or broader workload validation.
  - Benchmark hygiene note:
    - Discarded one parallel `large`/`xxlarge` run pair due GPU contention; only sequential reruns above were used.
- Metrics-stage/store-layout pass 3 attempt (vectorized final stores on padded path) - reverted:
  - Change attempted:
    - `FULL_M7` backtest kernel path used a helper that emitted two `float4` stores when `metrics_stride == 8`.
  - Result (clean sequential A/B, same commit):
    - `alma_alma xlarge` with `pad8=1`: regressed into ~`427-446 ms` band (previous `pad8=1` reference ~`392-400 ms`)
    - `sma_sma xlarge` also showed no improvement
  - Decision:
    - Reverted vectorized-store attempt; keep scalar stores in kernel.
  - Note:
    - Discarded one benchmark run where a GPU test and benchmark were executed concurrently.
- Metrics-stage/store-layout pass 4 (selective `pad8` runtime policy) - kept:
  - Change:
    - `VECTORBT_KERNEL_METRICS_PAD8` remains an env override, but default behavior is now selective:
      - default-on for `alma/alma`
      - default-off for other MA pairs
  - Implementation notes:
    - Added env override parsing helper and MA-pair-aware `pad8` policy in `crates/app_core/src/gpu_backtest_kernel.rs`.
    - Tile-byte estimation now uses the same MA-aware policy to preserve VRAM budgeting correctness.
  - Validation/benchmarks (env unset, clean sequential):
    - `alma_alma xlarge/370750pairs`: `401.48-409.86 ms` (keeps ALMA fast path in the `~400 ms` band)
    - `alma_alma xlarge_batched_1gb/370750pairs`: `897.79-907.05 ms` (keeps batched ALMA improvement)
    - `sma_sma xlarge/370750pairs`: `403.34-408.91 ms` (avoids prior `pad8=1` regression)
  - Regression check:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes.
  - Decision:
    - Keep selective default policy.
  - Benchmark hygiene note:
    - Discarded two parallel benchmark runs (single GPU contention) before sequential reruns.
- Backtest-kernel pass (lockstep+m7 regime-mode specialization) - kept with selective default:
  - Goal:
    - Reduce per-iteration branch overhead in the dominant `lockstep_time_m7` backtest kernel by specializing regime-mode dispatch (`none`, `regime`, `regime_hybrid`) while preserving kernel semantics.
  - Change:
    - Added regime-mode-specialized `lockstep+m7` kernel variants in `crates/app_core/src/kernels/double_crossover.cu`:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off`
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on`
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid`
    - Added host-side runtime selection (with fallback to existing `lockstep_time_m7`) in `crates/app_core/src/gpu_backtest_kernel.rs`.
    - Added env override + selective default policy:
      - `VECTORBT_KERNEL_LOCKSTEP_M7_REGIME_MODE_SPECIALIZE`
      - default-on for `alma/alma`
      - default-off for other MA pairs (to avoid measured `sma/sma` regression)
  - Benchmark results (same commit, sequential A/B unless noted):
    - ALMA/ALMA baseline (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `specialize=0`: `413.06-419.63 ms`
      - `specialize=1`: `389.99-394.23 ms` (improved, ~`5-7%`)
    - Strategy profile `regime_hybrid` (ALMA/ALMA xlarge)
      - `specialize=0`: `516.77-522.31 ms`
      - `specialize=1`: `495.49-504.08 ms` (improved, ~`3-5%`)
    - Strategy profile `trend_quality` (ALMA/ALMA xlarge)
      - `specialize=0`: `488.59-495.18 ms`
      - `specialize=1`: `493.39-500.14 ms` (no clear change; noise/slight regression band)
    - Strategy profile `breakout_momentum` (ALMA/ALMA xlarge)
      - `specialize=0`: `477.78-483.95 ms`
      - `specialize=1`: `474.36-480.25 ms` (no clear change; noise/slight improvement band)
    - `sma_sma xlarge` (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - `specialize=0`: `401.87-405.33 ms`
      - `specialize=1`: `456.79-462.15 ms` (regression, ~`13-15%`)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after specialization + host dispatch changes.
    - Re-ran `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`; known `regime_hybrid` mismatch remains unchanged (`pnl delta too large ... 2.8055098289775566`).
  - Decision:
    - Keep specialization code path and new kernels.
    - Keep default policy selective (`alma/alma` only) with env override for A/B and future tuning.
    - Prebuilt PTX synced so specialized symbols are available when `VECTORBT_USE_PREBUILT_PTX=1`.
    - Prebuilt PTX sanity:
      - `VECTORBT_USE_PREBUILT_PTX=1`, `specialize=1`, ALMA/ALMA xlarge observed `387.02-392.07 ms` (specialized symbols active and fast).
  - Benchmark hygiene note:
    - Discarded several attempted parallel benchmark runs (single-GPU contention and Cargo build lock contention) before using the sequential A/B results above.
- Nsight follow-up (regime-mode specialization A/B, xlarge ALMA/ALMA):
  - Reports:
    - `target/nsight/alma_xlarge_lockstep_m7_regime_mode_off_pass6.ncu-rep`
    - `target/nsight/alma_xlarge_lockstep_m7_regime_mode_on_pass6.ncu-rep`
  - Profiled kernels:
    - off: `double_cross_backtest_tm_flex_f32_lockstep_time_m7`
    - on: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off`
  - Nsight raw-counter summary (same workload, replay mode):
    - Kernel duration: `420.46 ms -> 416.83 ms` (small win in replay mode)
    - Executed instructions: `249.31B -> 242.00B` (instruction reduction)
    - Eligible warps/scheduler: `1.38 -> 1.35` (slight regression)
    - Derived `No Eligible` (`100 - smsp__issue_active.avg.pct_of_peak_sustained_active`): `37.43% -> 38.77%` (slight regression)
    - L2 theoretical excessive sectors ratio (computed from `memory_l2_theoretical_sectors_global` / `_ideal`): ~`12.62% -> 12.62%` (flat)
  - Interpretation:
    - This pass primarily reduces instruction volume while leaving memory-sector behavior unchanged.
    - Replay-mode Nsight shows only a modest duration win; larger wins still show up in Criterion benchmark runs.
- Backtest-kernel pass (lockstep+m7 regime-off no-risk specialization) - kept, default-on:
  - Goal:
    - Further reduce hot-loop instruction volume for the common no-risk `lockstep_time_m7` path by compile-time specializing away runtime stop/take logic in the already-specialized `regime_off` variant.
  - Change:
    - Extended kernel body template with compile-time risk-mode constant propagation (`RISK_MODE`) in `crates/app_core/src/kernels/double_crossover.cu`.
    - Added kernel variant:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`
    - Added host runtime selection in both GPU-kernel launch paths (`lockstep+m7`, `no_regime`, `no_risk`).
    - Added env override:
      - `VECTORBT_KERNEL_LOCKSTEP_M7_NORISK_SPECIALIZE`
      - default **on** (env override still available to disable).
  - Benchmark results (same commit, sequential A/B unless noted):
    - ALMA/ALMA `xlarge/370750pairs` (with `VECTORBT_KERNEL_LOCKSTEP_M7_REGIME_MODE_SPECIALIZE=1`)
      - `norisk=0`: `389.38-397.54 ms`
      - `norisk=1`: `374.24-378.78 ms` (improved, ~`3-5%`)
      - repeat `norisk=1` check: `371.38-377.07 ms` (same fast band)
    - ALMA/ALMA strategy profile `trend_quality` xlarge (with regime-mode specialization on)
      - `norisk=0`: `511.83-519.85 ms`
      - `norisk=1`: `494.23-503.13 ms` (improved, ~`2-5%`)
    - `sma_sma xlarge/370750pairs` (with `VECTORBT_KERNEL_LOCKSTEP_M7_REGIME_MODE_SPECIALIZE=0`)
      - `norisk=0`: `460.09-465.55 ms`
      - `norisk=1`: `389.57-393.60 ms` (improved, ~`14-16%`)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after no-risk specialization changes.
    - Re-ran `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`; known `regime_hybrid` mismatch remains unchanged (`pnl delta too large ... 2.8055098289775566`).
  - PTX / deployment notes:
    - Synced `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`; verified `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk` symbol is present.
    - Prebuilt PTX ALMA/ALMA xlarge sanity (`VECTORBT_USE_PREBUILT_PTX=1`, `regime_mode=1`, `norisk=1`) observed ~`395.05-400.01 ms` in one sequential run (not used for keep/revert decision).
  - Decision:
    - Keep code path and default it on (env-overridable).
    - Continue using explicit same-commit A/B runs for decisions; late env-unset spot checks showed noisy drift and were not used as decision inputs.
  - Benchmark hygiene note:
    - Discarded one prebuilt benchmark attempt run concurrently with a test due Cargo lock contention.
- Nsight follow-up (no-risk specialization A/B, xlarge ALMA/ALMA):
  - Reports:
    - `target/nsight/alma_xlarge_lockstep_m7_norisk_off_pass7.ncu-rep`
    - `target/nsight/alma_xlarge_lockstep_m7_norisk_on_pass7.ncu-rep`
  - Profiled kernels:
    - off: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off`
    - on: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`
  - Nsight raw-counter summary (same workload, replay mode):
    - Kernel duration: `421.23 ms -> 398.70 ms` (improved, ~`5.35%`)
    - Executed instructions: `242.00B -> 239.65B` (reduced)
    - Eligible warps/scheduler: `1.35 -> 1.42` (improved)
    - Derived `No Eligible` (`100 - smsp__issue_active.avg.pct_of_peak_sustained_active`): `38.35% -> 35.34%` (improved)
    - L2 theoretical excessive sectors ratio (computed from `memory_l2_theoretical_sectors_global` / `_ideal`): ~`12.62% -> 12.62%` (flat)
  - Interpretation:
    - This pass reduces instruction volume and improves scheduler readiness without changing the memory-sector/coalescing signature.
    - This aligns with the larger Criterion A/B wins on no-risk workloads.
- Backtest-kernel pass (regime-mode no-risk specialization extension) - kept:
  - Goal:
    - Extend the proven no-risk specialization pattern to the regime-filtered `lockstep+m7` variants (`regime_on`, `regime_hybrid`) to reduce hot-loop overhead on regime strategies.
  - Change:
    - Added CUDA kernel variants in `crates/app_core/src/kernels/double_crossover.cu`:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk`
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`
    - Updated host dispatch in both GPU-kernel paths to choose the matching no-risk wrapper (`off/on/hybrid`) when:
      - `use_lockstep_m7`
      - no stop-loss / take-profit flags
      - `VECTORBT_KERNEL_LOCKSTEP_M7_NORISK_SPECIALIZE` enabled
    - Fallback order remains intact (`*_norisk -> specialized -> base lockstep_m7`).
  - Benchmark results (same commit, sequential A/B):
    - Strategy profile `regime_hybrid` (ALMA/ALMA xlarge, `VECTORBT_KERNEL_LOCKSTEP_M7_REGIME_MODE_SPECIALIZE=1`)
      - `norisk=0`: `497.27-505.45 ms`
      - `norisk=1`: `477.46-485.46 ms` (improved, ~`3-5%`)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after dispatch + new kernel symbols.
    - Re-ran `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`; known `regime_hybrid` mismatch remains unchanged (`pnl delta too large ... 2.8055098289775566`).
  - PTX / deployment notes:
    - Synced `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`; verified:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk`
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`
  - Decision:
    - Keep the new regime no-risk variants and dispatch path.
- Nsight follow-up (regime_hybrid no-risk specialization A/B, xlarge ALMA/ALMA):
  - Reports:
    - `target/nsight/regime_hybrid_alma_xlarge_norisk_off_pass8.ncu-rep`
    - `target/nsight/regime_hybrid_alma_xlarge_norisk_on_pass8.ncu-rep`
  - Profiled kernels:
    - off: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid`
    - on: `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`
  - Nsight raw-counter summary (same workload, replay mode):
    - Kernel duration: `536.83 ms -> 498.37 ms` (improved, ~`7.16%`)
    - Executed instructions: `329.32B -> 317.19B` (reduced)
    - Eligible warps/scheduler: `1.46 -> 1.45` (roughly flat)
    - Derived `No Eligible` (`100 - smsp__issue_active.avg.pct_of_peak_sustained_active`): `35.95% -> 34.91%` (slight improvement)
    - L2 theoretical excessive sectors ratio (computed from `memory_l2_theoretical_sectors_global` / `_ideal`): ~`10.98% -> 10.98%` (flat)
  - Interpretation:
    - This mirrors the earlier no-risk pass: lower instruction volume with essentially unchanged memory-sector behavior.
    - Replay-mode Nsight shows a stronger duration win here than the `regime_off` no-risk pass.
- Backtest-kernel pass attempt (no-commission specialization) - reverted:
  - Goal:
    - Test whether compile-time specializing the common `commission == 0.0f` path reduces hot-loop instruction count on `lockstep+m7` no-risk variants.
  - Change (attempted, then reverted):
    - Added a compile-time commission mode parameter and `*_norisk_nocomm` wrappers for `regime_off`, `regime_on`, and `regime_hybrid`.
    - Added host dispatch for exact-zero commission configs.
  - Benchmark results (same commit, sequential A/B):
    - ALMA/ALMA baseline xlarge (`regime_mode=1`, `norisk=1`)
      - `nocomm=0`: `368.16-373.45 ms`
      - `nocomm=1`: `375.17-380.35 ms` (regression / noise; no win)
    - Strategy profile `regime_hybrid` (ALMA/ALMA xlarge, `regime_mode=1`, `norisk=1`)
      - `nocomm=0`: `480.86-487.68 ms`
      - `nocomm=1`: `481.37-488.29 ms` (no material change)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passed before and after the revert.
  - Decision:
    - Revert the no-commission specialization attempt (no repeatable win; extra variants not justified).
- Backtest-kernel pass attempt (trade_on_next_bar specialization) - reverted:
  - Goal:
    - Test whether compile-time specializing the common `trade_on_next_bar=true` path reduces hot-loop overhead on `lockstep+m7` no-risk wrappers.
  - Change (attempted, then reverted):
    - Added a compile-time trade-on-next-bar mode parameter to the shared CUDA kernel body.
    - Added `*_norisk_tnb_on` wrapper variants for `regime_off`, `regime_on`, and `regime_hybrid`, plus host dispatch behind `VECTORBT_KERNEL_LOCKSTEP_M7_TNB_SPECIALIZE`.
  - Benchmark results (same commit, sequential A/B):
    - ALMA/ALMA baseline xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `tnb=0`: `392.69-398.99 ms`
      - `tnb=1`: `406.23-411.29 ms` (regression, ~`3-4%`)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passed before and after the revert.
  - Decision:
    - Revert the trade-on-next-bar specialization attempt (clear regression on primary ALMA xlarge workload).
- Backtest-kernel pass attempt (template branch-tightening cleanup) - split and reverted:
  - Goal:
    - Reduce hot-loop instruction overhead without adding kernel variants by using compile-time branch selection (`if constexpr`) inside the shared CUDA kernel body.
  - Attempted changes (split into sub-passes):
    - Sub-pass A: `desired_sign_mode_f32<REGIME_MODE>` switched from runtime `if (REGIME_MODE == ...)` checks to `if constexpr`.
    - Sub-pass B: `FULL_M7` in-loop metric/update blocks rewritten to `if constexpr (FULL_M7)` fast path + non-`FULL_M7` fallback blocks.
  - Benchmark notes:
    - Discarded multiple benchmark runs caused by accidental parallel GPU benchmark execution (single-GPU contention / Cargo lock contention).
  - Sub-pass A results (sequential):
    - ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - baseline (pre sub-pass): `370.19-377.21 ms`
      - sub-pass A: `373.40-377.99 ms` (no significant change)
    - `regime_hybrid` strategy profile xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - sub-pass A: `483.89-493.20 ms` (no significant change)
  - Sub-pass B results (applied on top of sub-pass A, sequential):
    - ALMA/ALMA xlarge:
      - `396.98-402.45 ms` (clear regression, ~`5-7%`)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after revert.
  - Decision:
    - Revert sub-pass B (`FULL_M7` in-loop `if constexpr` rewrite) due ALMA regression.
    - Revert sub-pass A as well (neutral / no measurable win; keep kernel baseline clean).
- Backtest-kernel pass (regime-hybrid helper dead-branch cleanup) - kept:
  - Goal:
    - Reduce instruction overhead in the `regime_hybrid` sign-selection helper path without adding kernel variants by removing a dead `fabsf`/branch sequence that always returned `0`.
  - Change:
    - In `crates/app_core/src/kernels/double_crossover.cu`, removed the dead `else if (fabsf(spread_rel) <= regime_range_exit_rel)` branch (and duplicate `else`) from:
      - `desired_sign_regime_hybrid_f32`
      - `desired_sign_with_regime_f32` (runtime-dispatch fallback path)
    - Semantics preserved: both removed branches returned `0` in all cases.
  - Benchmark notes:
    - Discarded multiple accidental parallel GPU benchmark runs (single-GPU contention / Cargo lock contention); keep/revert decisions below use sequential reruns only.
  - Benchmark results (sequential):
    - `regime_hybrid` strategy profile (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - post-pass: `467.25-477.60 ms`
      - nearby pre-pass comparison band: `478.46-488.42 ms` (improved)
    - ALMA/ALMA baseline xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `371.16-377.06 ms` (no regression; same fast band)
    - `trend_quality` strategy profile xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/trend_quality/xlarge/370750pairs`)
      - `492.79-501.69 ms` (no material change)
    - `sma_sma` baseline xlarge (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - `379.25-383.79 ms` (no material change)
  - Nsight follow-up (`regime_hybrid_norisk`, xlarge ALMA/ALMA):
    - Reports:
      - baseline: `target/nsight/regime_hybrid_alma_xlarge_norisk_on_pass8.ncu-rep`
      - current: `target/nsight/regime_hybrid_alma_xlarge_deadfabs_pass9.ncu-rep`
    - Profiled kernel:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`
    - Nsight raw-counter summary (replay mode):
      - Kernel duration: `498.37 ms -> 492.39 ms` (improved)
      - Executed instructions: `317.19B -> 305.75B` (reduced)
      - Eligible warps/scheduler: `1.45 -> 1.46` (flat/slight improvement)
      - Derived `No Eligible`: `34.91% -> 35.01%` (flat)
      - L2 theoretical excessive sectors ratio: flat (no meaningful change)
    - Interpretation:
      - This pass primarily reduces instruction count in the regime-hybrid helper path.
      - Memory-sector/coalescing behavior is unchanged, as expected.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes.
  - PTX / deployment notes:
    - Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` with CUDA 13.0 (`compute_89`) using explicit `-ccbin` (MSVC) because `cl.exe` was not on `PATH`.
    - Prebuilt PTX sanity (`VECTORBT_USE_PREBUILT_PTX=1`) on `regime_hybrid` xlarge: `468.83-474.01 ms`.
  - Decision:
    - Keep the helper cleanup.
- Backtest-kernel pass (regime-hybrid helper branch+math tightening) - kept:
  - Goal:
    - Further reduce `regime_hybrid` helper overhead in the hot loop by:
      - avoiding unnecessary `sgn_eps(...)` work when the helper exits through the hybrid range-mode path
      - replacing `(f - s) / denom` + compare with multiply-threshold compares (remove division)
  - Change:
    - Updated `desired_sign_regime_hybrid_f32` and `desired_sign_with_regime_f32` in `crates/app_core/src/kernels/double_crossover.cu`:
      - compute `long_only` once
      - delay `sgn_eps` until after the hybrid range-mode early-return path
      - replace `spread_rel = (f - s) / denom` comparisons with:
        - `delta = f - s`
        - `entry = regime_range_entry_rel * denom`
        - `range_sign = (delta <= -entry) - (delta >= entry)`
  - Benchmark notes:
    - Discarded multiple accidental parallel GPU benchmark runs (benchmark-vs-benchmark and benchmark-vs-Nsight contention).
    - Keep/revert decisions below use sequential reruns only.
  - Benchmark results (sequential):
    - `regime_hybrid` strategy profile (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - post-pass: `424.85-431.84 ms`
      - nearby pre-pass comparison band: `467.25-477.60 ms` (material improvement)
    - ALMA/ALMA baseline xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `373.89-381.91 ms` (no material regression)
    - `sma_sma` baseline xlarge (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - `381.12-387.06 ms` (no material regression)
  - Nsight follow-up (`regime_hybrid_norisk`, xlarge ALMA/ALMA):
    - Reports:
      - baseline: `target/nsight/regime_hybrid_alma_xlarge_deadfabs_pass9.ncu-rep`
      - current: `target/nsight/regime_hybrid_alma_xlarge_hybridhelper_pass10.ncu-rep`
    - Profiled kernel:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk`
    - Nsight raw-counter summary (replay mode):
      - Kernel duration: `492.39 ms -> 464.06 ms` (improved)
      - Executed instructions: `305.75B -> 264.34B` (reduced)
      - Eligible warps/scheduler: `1.46 -> 1.23` (regressed)
      - Derived `No Eligible`: `35.01% -> 39.08%` (regressed)
      - L2 theoretical excessive sectors ratio: flat (no meaningful change)
    - Interpretation:
      - This pass is another instruction-count win and materially improves replay-mode duration.
      - Scheduler readiness metrics regressed while total work dropped more substantially; net performance still improved.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes.
  - PTX / deployment notes:
    - Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` with CUDA 13.0 (`compute_89`) using explicit `-ccbin`.
    - Prebuilt PTX sanity (`VECTORBT_USE_PREBUILT_PTX=1`) on `regime_hybrid` xlarge:
      - first run `448.35-455.97 ms` (warm-up outlier; not used for decision)
      - repeat run `427.87-434.58 ms` (matches post-pass fast band)
  - Decision:
    - Keep the helper tightening pass.
- Backtest-kernel pass attempt (helper-signature cleanup: `flags` -> `long_only`) - reverted:
  - Goal:
    - Reduce repeated `STRAT_LONG_ONLY` bit tests in sign helpers by passing a precomputed `long_only` flag through `desired_sign_*` helper signatures.
  - Change attempted:
    - Changed `desired_sign_with_regime_f32`, `desired_sign_plain_f32`, `desired_sign_regime_only_f32`, `desired_sign_regime_hybrid_f32`, and `desired_sign_mode_f32` to take `long_only` instead of `flags`.
    - Added per-pair `const int long_only = (flags & STRAT_LONG_ONLY) ? 1 : 0;` in the hot loop and passed it to helpers.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passed.
  - Benchmark results (sequential rerun after revert to helper-flag signatures):
    - `sma_sma` baseline xlarge (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - `437.81-444.18 ms`
  - Decision:
    - Revert the helper-signature cleanup attempt (regression persisted / no clear win).
  - Follow-up note:
    - The helper-signature revert initially reintroduced `STRAT_LONG_ONLY` usage before the macro definition in `double_crossover.cu`; helper-local checks were corrected back to `(1u << 0)` before PTX sync.
- Backtest-kernel pass attempt (flag/commission hoisting in shared kernel body) - reverted:
  - Goal:
    - Hoist repeated runtime flag checks and commission comparisons (`trade_on_next_bar`, `no_flip`, `commission > 0`, etc.) into locals without adding kernel variants.
  - Change attempted:
    - Added local cached booleans/ints in `double_cross_backtest_tm_flex_f32_body` and replaced repeated hot-loop checks.
  - Benchmark results (sequential):
    - ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `395.28-397.67 ms` (regressed vs current fast band)
    - `regime_hybrid` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - `426.46-431.51 ms` (roughly neutral)
  - Decision:
    - Revert the hoisting pass (primary ALMA xlarge regression).
- Backtest-kernel pass (`sgn_eps` zero-epsilon fast path) - kept:
  - Goal:
    - Reduce hot helper instruction cost for the common `eps == 0` case without adding kernel variants.
  - Change:
    - Updated `sgn_eps(float f, float s, float eps)` in `crates/app_core/src/kernels/double_crossover.cu` to fast-path `eps <= 0.0f`:
      - return `(f > s) - (f < s)`
      - otherwise use the existing epsilon-threshold compare path.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after the change.
  - Benchmark notes:
    - Discarded one anomalous `sma_sma` run at ~`2.83 s` (single-run outlier; repeated sequential rerun returned to normal fast band).
  - Benchmark results (sequential reruns):
    - ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `369.76-376.24 ms` (improved vs nearby `~373.89-381.91 ms` band)
    - `regime_hybrid` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - `428.24-434.11 ms` (no material change vs current fast band)
    - `sma_sma` baseline xlarge (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - rerun: `379.96-384.04 ms` (fast band; no regression)
  - PTX / deployment notes:
    - Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` with CUDA 13.0 (`compute_89`) and explicit MSVC `-ccbin` after fixing the helper-local `STRAT_LONG_ONLY` macro-order issue.
  - Decision:
    - Keep the `sgn_eps` zero-epsilon fast path.
- Follow-up note (prebuilt PTX path):
  - `VECTORBT_USE_PREBUILT_PTX=1` ALMA/ALMA xlarge sanity after PTX regen measured `402.78-410.95 ms` and repeat `406.04-412.60 ms`, slower than the JIT/source path (`369.76-376.24 ms`).
  - PTX compiles and loads, but prebuilt-vs-JIT performance divergence is now a tracked follow-up item.
- Backtest-kernel pass attempt (branchless exposure abs accumulation via `pos != 0`) - reverted:
  - Goal:
    - Reduce stats-path instruction cost by replacing `llabs((long long)pos)` / `llabs((long long)pos_bar)` with branchless boolean increments (`(pos != 0)`) since position magnitude is constrained to `{-1,0,1}`.
  - Change attempted:
    - In `double_cross_backtest_tm_flex_f32_body`, replaced:
      - `sum_abs_pos += llabs((long long)pos);`
      - `sum_abs_pos += llabs((long long)pos_bar);`
    - with boolean increments cast to `long long`.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passed.
  - Benchmark notes:
    - Discarded accidental parallel benchmark runs for `regime_hybrid` / `sma_sma` (Cargo/GPU contention visible in output).
    - Keep/revert decision uses sequential reruns only.
  - Benchmark results (sequential):
    - ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `391.42-396.13 ms` (regressed vs current `~375-381 ms` band)
    - `regime_hybrid` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - `421.01-425.24 ms` (looked improved / neutral)
    - `sma_sma` xlarge (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - `436.76-440.94 ms` (regressed vs current `~380-387 ms`/`~437-441 ms` workload band depending path state)
  - Decision:
    - Revert due clear regression on primary ALMA xlarge and `sma_sma` xlarge.
- Backtest-kernel pass attempt (regime helper non-null specialization for `REGIME_MODE=1/2`) - reverted:
  - Goal:
    - Remove per-iteration `regime_filter == nullptr` checks from compile-time regime-specialized sign helpers (`REGIME_MODE=1/2`) by introducing non-null helper variants.
  - Change attempted:
    - Added `desired_sign_regime_only_nonnull_f32` and `desired_sign_regime_hybrid_nonnull_f32` and routed `desired_sign_mode_f32<1/2>` to them.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passed.
  - Benchmark results (sequential):
    - `regime_hybrid` ALMA/ALMA xlarge:
      - `548.92-549.48 ms` (clear regression, ~`+29-30%`)
  - Decision:
    - Revert (likely code-size/register-pressure regression outweighed removed null-check branch).
- Nsight follow-up (ALMA/ALMA xlarge) for kept `sgn_eps` zero-epsilon fast path:
  - Reports:
    - baseline: `target/nsight/alma_xlarge_lockstep_m7_norisk_on_pass7.ncu-rep`
    - current: `target/nsight/alma_xlarge_sgneps_pass11.ncu-rep`
  - Profiled kernel:
    - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk`
  - Nsight raw-counter summary (replay mode):
    - Kernel duration: `398.70 ms -> 396.15 ms` (~`0.64%` faster)
    - Executed instructions: `239.65B -> 244.52B` (~`+2.03%`)
    - Eligible warps/scheduler: `1.420 -> 1.420` (flat)
    - `derived__memory_l2_theoretical_sectors_global_excessive`: unchanged (`2.05215`)
    - L2 theoretical excessive-sector ratio: unchanged (`12.62% -> 12.62%`)
  - Interpretation:
    - Replay-mode kernel improvement is small and not explained by memory/coalescing gains.
    - The kept `sgn_eps` pass remains low-risk, but its benefit currently appears modest / workload-variant rather than a large universal win.
- Backtest-kernel pass (`eps_rel == 0` specialization for `lockstep+m7+no-risk`) - kept with selective dispatch:
  - Goal:
    - Remove per-iteration epsilon-threshold computation/branching in the hot no-risk lockstep+m7 path for the common `eps_rel == 0` case by adding compile-time `EPS_MODE=0` wrappers.
  - CUDA changes (`crates/app_core/src/kernels/double_crossover.cu`):
    - Added templated epsilon helper: `compute_eps_f32<EPS_MODE>`.
    - Extended shared kernel body template with `EPS_MODE` parameter (default runtime path preserved).
    - Added wrappers:
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_off_norisk_eps0` (kept)
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_hybrid_norisk_eps0` (kept)
      - `double_cross_backtest_tm_flex_f32_lockstep_time_m7_regime_on_norisk_eps0` (attempted, later pruned due regression)
  - Rust dispatch changes (`crates/app_core/src/gpu_backtest_kernel.rs`):
    - Added env override gate `VECTORBT_KERNEL_LOCKSTEP_M7_EPS0_SPECIALIZE`.
    - Dispatches to `*_eps0` wrappers only when:
      - `use_lockstep_m7`
      - `no_risk_mode`
      - `eps_rel <= 0`
      - gate enabled
    - Final policy:
      - default enabled only for `alma/alma`
      - explicit env override still wins
      - `regime_on` path intentionally does **not** use `eps0` specialization (pruned due regression)
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after implementation and after regime-on pruning.
  - Benchmark notes:
    - Discarded multiple obvious outliers (`sma_sma` ~`2.79 s`, previous ~`2.83 s`) and accidental parallel runs in this cycle; keep/revert decisions below use sequential reruns.
  - Same-commit A/B (representative current-build checks):
    - ALMA/ALMA baseline xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `eps0=0`: `400.64-405.86 ms`
      - `eps0=1`: `387.21-395.20 ms` (win, ~`3-4%`)
    - `regime_hybrid` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/regime_hybrid/xlarge/370750pairs`)
      - `eps0=0`: `449.85-456.61 ms`
      - `eps0=1`: `411.26-415.50 ms` (win, ~`8-10%`)
    - `breakout_momentum` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/breakout_momentum/xlarge/370750pairs`)
      - `eps0=0`: `454.92-464.56 ms`
      - `eps0=1`: `453.38-459.72 ms` (neutral)
    - `sma_sma` xlarge (`double_ma/gpu_kernel/sma_sma/200k/xlarge/370750pairs`)
      - `eps0=0`: `384.15-390.82 ms`
      - `eps0=1`: `433.02-437.25 ms` (regression)
      - drove ALMA-only default gating
  - Final default-policy sanity (sequential reruns):
    - ALMA/ALMA xlarge: `362.37-367.33 ms` (fast band)
    - `regime_hybrid` ALMA/ALMA xlarge: `411.26-415.50 ms` (with `eps0=1`; same as A/B on current build)
    - `trend_quality` ALMA/ALMA xlarge: `492.03-498.48 ms` (no regression; regime-on path left unspecialized)
    - `sma_sma` xlarge: `383.11-388.23 ms` (no regression; ALMA-only gate avoids `eps0`)
    - ALMA/ALMA large (`58300pairs`): `100.93-101.51 ms` (no material change)
    - ALMA/ALMA `xlarge_batched_1gb`: `935.37-938.72 ms` (no material regression; slight improvement/noise)
  - PTX / deployment notes:
    - Regenerated `crates/app_core/src/kernels/double_crossover.prebuilt.ptx` (CUDA 13.0, `compute_89`, explicit MSVC `-ccbin`).
    - Prebuilt PTX sanity (`VECTORBT_USE_PREBUILT_PTX=1`) ALMA/ALMA xlarge: `386.06-392.80 ms`.
    - Prebuilt PTX path still trails the current JIT/source path (`362.37-367.33 ms`), but is improved vs earlier ~`406-413 ms` prebuilt band.
  - Decision:
    - Keep the `eps0` specialization pass with selective dispatch (ALMA-only default; `regime_on` unspecialized).
- Follow-up note (prebuilt PTX path):
  - After `eps0` pass + PTX regen, ALMA/ALMA xlarge prebuilt PTX (`VECTORBT_USE_PREBUILT_PTX=1`) improved to `386.06-392.80 ms` but remains slower than JIT/source (`362.37-367.33 ms`).

- Prebuilt PTX divergence investigation - closed (artifact equality confirmed):
  - Goal:
    - Determine whether the tracked prebuilt PTX performance gap was caused by different PTX content/arch/flags between normal builds and `VECTORBT_USE_PREBUILT_PTX=1`.
  - Build-path findings:
    - `crates/app_core/build.rs` always emits `OUT_DIR/double_crossover.ptx`.
    - Normal path runs `nvcc -ptx -O3 -arch=compute_89` (plus Windows `-ccbin` wiring) into `OUT_DIR`.
    - `VECTORBT_USE_PREBUILT_PTX=1` path simply copies `src/kernels/double_crossover.prebuilt.ptx` into the same `OUT_DIR` path.
  - Controlled rebuild/hash comparison (same commit):
    - Source prebuilt PTX SHA256:
      - `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
      - `AC54F05A8BD699A07E472FB736B85E70C63D070FB8856A6650E831AEF01060E0`
    - Generated `OUT_DIR` PTX SHA256 (normal `nvcc` build):
      - `target/ptx_compare/generated_outdir.ptx`
      - `AC54F05A8BD699A07E472FB736B85E70C63D070FB8856A6650E831AEF01060E0`
    - Prebuilt-copy `OUT_DIR` PTX SHA256 (`VECTORBT_USE_PREBUILT_PTX=1`):
      - `target/ptx_compare/prebuilt_outdir.ptx`
      - `AC54F05A8BD699A07E472FB736B85E70C63D070FB8856A6650E831AEF01060E0`
    - Result:
      - `OUT_DIR` PTX artifacts are byte-identical between modes.
  - Build logs captured:
    - generated path: `target/ptx_compare/build_generated.log` (shows exact `nvcc` invocation)
    - prebuilt path: `target/ptx_compare/build_prebuilt.log` (shows copy into `OUT_DIR`)
  - Decision:
    - Remove "prebuilt PTX divergence" as an active optimization target for kernel codegen/layout.
    - Any remaining prebuilt-vs-non-prebuilt runtime differences are not explained by PTX content mismatch.

- Target B metrics/store-layout follow-up pass (full M=7 row-stride sweep) - default promotion reverted; manual override kept:
  - Goal:
    - Re-test the remaining metrics layout hypothesis by sweeping wider physical row strides for full `M=7` tile metrics (`metrics_out`) using the existing stride plumbing.
  - Code change kept (experimental/tuning hook only):
    - Added `VECTORBT_KERNEL_METRICS_FULL_STRIDE` env override in `crates/app_core/src/gpu_backtest_kernel.rs`.
    - Behavior:
      - when set to `>= 7`, full `M=7` tile rows use that physical stride;
      - otherwise existing policy applies (`pad8` for current ALMA/ALMA default path).
    - No default behavior change was kept.
  - Validation:
    - `gpu_kernel_topk_v2_matches_forced_legacy` passes after the hook and after reverting the default-promotion attempt.
  - Decisive same-commit A/B (forced stride sweep, sequential):
    - ALMA/ALMA xlarge (`double_ma/gpu_kernel/alma_alma/200k/xlarge/370750pairs`)
      - `stride=8`: `394.82-401.11 ms`
      - `stride=16`: `394.03-398.69 ms`
      - Interpretation: effectively neutral on current code state (not a repeatable win)
    - ALMA/ALMA xlarge batched 1GB (`double_ma/gpu_kernel/alma_alma/200k/xlarge_batched_1gb/370750pairs`)
      - `stride=8`: `958.41-974.16 ms`
      - `stride=16`: `1.0219-1.0397 s`
      - Interpretation: clear regression (~`+6-7%`)
    - `trend_quality` ALMA/ALMA xlarge (`double_ma/gpu_kernel_strategy/alma_alma/200k/trend_quality/xlarge/370750pairs`)
      - `stride=8`: `497.51-506.69 ms`
      - `stride=16`: `500.70-508.94 ms`
      - Interpretation: neutral / slight regression
  - Attempted default-promotion policy (reverted):
    - Tried adaptive default gating for stride `16` (pair-based and later workload-shape / VRAM-budget-aware).
    - Follow-up benchmark matrices were noisy/inconsistent and did not justify a default change.
    - Final decision was to revert the default policy and keep only the explicit env override.
  - Notes:
    - This pass does **not** close Target B.
    - The deeper SoA/hybrid metrics-tile refactor remains the remaining high-effort metrics-layout phase.

- Regime-hybrid CPU/GPU parity mismatch (`gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies`) - resolved:
  - Prior symptom:
    - Test failed on `regime_hybrid` with `pnl delta too large ... 2.8055098289775566` at `crates/app_core/src/double_ma.rs:7218`.
  - Root cause (not CUDA fp32 indicator precision):
    - CPU `eval_metrics_*_with_exit_mode` routed no-overlay `halftrend`/regime-filter cases through `ta_strategies::*with_halftrend_filter*`.
    - That path only supports a directional filter (`NaN` / long / short) and treats hybrid range-state (`2.0`) as short.
    - GPU kernel path correctly implements explicit `regime_hybrid` range-state mean-reversion behavior.
    - Result: CPU reference semantics diverged from GPU semantics specifically for `regime_hybrid`.
  - Fix:
    - In `crates/app_core/src/double_ma.rs`, route `regime_hybrid` metric/curve evaluation through the app-core cross evaluator (`desired_sign_with_filters(...)`) even when execution overlays are disabled.
    - Applied to:
      - `eval_metrics_f64_with_exit_mode`
      - `eval_metrics_f32_with_exit_mode`
      - `eval_curves_f64_with_exit_mode`
  - Validation:
    - `cargo test -p ta_app_core gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies --features cuda,cuda-backtest-kernel -- --nocapture` ✅
    - `cargo test -p ta_app_core gpu_kernel_topk_v2_matches_forced_legacy --features cuda,cuda-backtest-kernel -- --nocapture` ✅
