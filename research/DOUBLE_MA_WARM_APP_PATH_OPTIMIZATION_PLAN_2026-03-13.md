# Double MA Warm App-Path Optimization Plan

Date: 2026-03-13
Status: Executed; no accepted production optimization beyond instrumentation
Scope: iterative benchmarking, Nsight-guided optimization, and closure criteria for the warm app-path `DoubleMa` GPU benchmark

## Goal

Reduce the real warm app-path `DoubleMa` GPU optimization latency, not just the tighter Criterion benchmark latency.

This plan began from an older app-path reading around `494.757 ms`, but the fresh March 13, 2026 reruns established that the current warm app-path baseline is already much faster:

- fresh warm app-path faceoff benchmark:
  - `target/vectorbt_vs_vectorgrid_faceoff_58300_20260313/summary.json`
  - fresh median `VectorGrid GpuKernel`: `106.832 ms`
- fresh Criterion helper artifact:
  - `target/vectorgrid_double_ma_faceoff_20260313.json`
  - reported point estimate: `485.21 ms`
- historical Criterion anchor:
  - documented around `89 ms` for `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`

This plan is specifically about closing the gap between the real warm app-path timing and the best achievable product-facing runtime, while preserving parity and existing backend behavior.

## Final Finding

The optimization pass did not uncover a robust production code change that improved the already-fast warm app-path `DoubleMa` GPU benchmark.

The main conclusions are:

- the fresh warm app-path path is already running near `~100 ms`, not `~495 ms`
- the dominant cost is overwhelmingly the GPU backtest stage itself, not host prep, heatmap, or result shaping
- the existing Criterion helper is not benchmarking the same request shape as the faceoff
  - Criterion currently uses `top_k=50` and `heatmap_bins=64`
  - the warm app-path faceoff uses `top_k=10` and `heatmap_bins=0`
- the exact same warm app-path request shows bimodal process-level behavior:
  - common fast mode around `~101-107 ms`
  - intermittent slow mode around `~482-494 ms`
- dispatch logging showed both fast and slow modes still selecting the same `dispatch=m7` path, so the bimodality is not explained by obvious kernel-branch selection in the current source
- no toggle or code change produced a robust, parity-safe improvement over the fast `~100 ms` mode

The result is that the original optimization target was stale. The current app-path implementation is already close to the old `~89 ms` historical anchor, and the remaining major issue is unstable process-level runtime behavior rather than an obvious hot-path inefficiency in the current production code.

## Why This Is Separate

This is not the same task as:

- general GPU validation migration
- data residency implementation
- synthetic faceoff/demo work
- speculative kernel redesign

The current warm app-path benchmark already:

- loads the dataset once
- runs warmup trials before measured trials
- reuses resident market-series buffers on the GPU path

So the remaining `~495 ms` is now close enough to the actual production optimization path that it deserves its own optimization tracker.

## Source Surfaces

Primary execution surfaces:

- [../src-tauri/src/bin/ta_app_path_faceoff.rs](../src-tauri/src/bin/ta_app_path_faceoff.rs)
- [../crates/app_core/src/double_ma.rs](../crates/app_core/src/double_ma.rs)
- [../crates/app_core/src/gpu_backtest_kernel.rs](../crates/app_core/src/gpu_backtest_kernel.rs)
- [../crates/app_core/src/kernels/double_crossover.cu](../crates/app_core/src/kernels/double_crossover.cu)

Primary benchmark and artifact surfaces:

- [../scripts/vectorbt_double_ma_faceoff_scenario.json](../scripts/vectorbt_double_ma_faceoff_scenario.json)
- [../scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1](../scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1)
- [../scripts/run_vectorgrid_double_ma_faceoff.ps1](../scripts/run_vectorgrid_double_ma_faceoff.ps1)
- [../scripts/profile_gpu_backtest_ncu.ps1](../scripts/profile_gpu_backtest_ncu.ps1)
- [../target/vectorbt_vs_vectorgrid_faceoff_58300_rerun/summary.json](../target/vectorbt_vs_vectorgrid_faceoff_58300_rerun/summary.json)
- [../target/vectorgrid_double_ma_faceoff.json](../target/vectorgrid_double_ma_faceoff.json)

Related architecture and tracker context:

- [./OPEN_IMPLEMENTATION_COMPLETION_TRACKER_2026-03-12.md](./OPEN_IMPLEMENTATION_COMPLETION_TRACKER_2026-03-12.md)
- [./PRICE_DATA_RESIDENCY_AND_VRAM_CACHE_IMPLEMENTATION_PLAN_2026-03-12.md](./PRICE_DATA_RESIDENCY_AND_VRAM_CACHE_IMPLEMENTATION_PLAN_2026-03-12.md)
- [./VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_PLAN_2026-03-11.md](./VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_PLAN_2026-03-11.md)

## Measurement Boundaries

Do not mix these timing boundaries in analysis or reporting.

### 1. Warm app-path benchmark

This is the primary optimization target for this plan.

What it includes:

- app-path runner setup
- request shaping
- `AppState` candle lookup
- real optimization call through `run_double_ma_optimization_blocking_with_candles(...)`
- result shaping and serialization inside the benchmark harness

What it intentionally avoids:

- repeated disk load per measured trial
- repeated cold process startup per measured trial
- repeated market-series upload after warmup when the resident cache remains valid

### 2. Criterion benchmark

This remains a tighter engine benchmark.

What it is good for:

- stable engine-level regression tracking
- highly comparable microbenchmark numbers
- lower orchestration noise

What it is not:

- a full product-facing app-path benchmark

### 3. Nsight captures

Nsight is diagnostic evidence, not the final benchmark number.

Use it to answer:

- which stage dominates
- whether the dominant stage changed after an optimization
- whether the optimization improved utilization, memory behavior, or launch efficiency

## Baseline Freeze

Before optimization work starts, treat the following as the current baseline set.

- [x] Fresh warm app-path `58,300`-pair artifact exists:
  - `target/vectorbt_vs_vectorgrid_faceoff_58300_20260313/summary.json`
- [x] Fresh warm app-path GPU median recorded: `106.832 ms`
- [x] Fresh warm app-path CPU median recorded: `3422.248 ms`
- [x] Fresh warm app-path VectorBT CPU median recorded: `92941.103 ms`
- [x] Fresh Criterion helper artifact exists:
  - `target/vectorgrid_double_ma_faceoff_20260313.json`
- [x] Fresh Criterion helper point estimate recorded: `485.21 ms`
- [x] Nsight artifact for the exact `alma_alma/200k/large/58300pairs` path is captured:
  - `target/nsight/double_ma_alma_58300_20260313.ncu-rep`
  - raw CSV export: `target/nsight/double_ma_alma_58300_20260313_raw.csv`
- [x] Pipeline profile output for the exact warm app-path path is captured:
  - `target/vectorbt_vs_vectorgrid_faceoff_58300_pipeline_20260313/pipeline.log`

## Optimization Rules

Do not optimize blindly.

- [x] Every optimization attempt starts from a measured baseline on the exact same scenario.
- [x] Every optimization attempt states a specific hypothesis before code changes.
- [x] Only one primary optimization hypothesis is tested at a time.
- [x] Parity and route behavior were preserved.
- [x] Changes or toggles that did not help the exact warm app-path case were rejected.
- [x] Changes or toggles with unstable/bimodal outcomes were rejected.
- [x] Apparent benchmark wins that depended on a different timing boundary or request shape were rejected.

## Closure Rules

Do not claim the path is exhausted until all of the following are true.

- [x] The warm app-path benchmark has been profiled with pipeline timing and Nsight on the exact frozen scenario.
- [ ] Each dominant stage identified by profiling has either:
  - [x] been optimized and remeasured, or
  - [x] been explicitly rejected with a written reason
- [x] No optimization was accepted without reruns.
- [x] Instrumentation changes were verified with relevant Rust checks and GPU perf gate.
- [x] The final document includes:
  - [x] before/after timings
  - [x] artifact paths
  - [x] parity and route notes
  - [x] the final conclusion on whether more work is justified

## Phase 1: Instrument The Real Path

Goal: obtain a stage-by-stage view of the warm app-path path before touching kernels.

- [x] Capture fresh warm app-path benchmark artifact for `scripts/vectorbt_double_ma_faceoff_scenario.json`
- [x] Capture fresh Criterion artifact for `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- [x] Run the warm app-path scenario with pipeline profiling enabled
- [x] Record the pipeline breakdown in this document
- [x] Capture Nsight Compute for the exact production `alma_alma/200k/large/58300pairs` path
- [x] Record the top kernels, key durations, and utilization observations in this document

Execution note:

- fresh warm app-path faceoff:
  - `target/vectorbt_vs_vectorgrid_faceoff_58300_20260313/summary.json`
  - `VectorGrid GpuKernel`: `106.832 ms`
  - `VectorGrid CPU`: `3422.248 ms`
  - `VectorBT CPU`: `92941.103 ms`
- fresh Criterion helper:
  - `target/vectorgrid_double_ma_faceoff_20260313.json`
  - reported point estimate: `485.21 ms`
- warm pipeline profile:
  - `target/vectorbt_vs_vectorgrid_faceoff_58300_pipeline_20260313/pipeline.log`
  - representative breakdown:
    - `host_prep`: about `0.002 ms`
    - `tile_prepare`: about `4.9-6.2 ms`
    - `tile_score`: about `477-479 ms`
    - `backtest`: essentially all of `tile_score`
    - `tile_finalize + final_finalize`: under `0.4 ms`
- Nsight artifact:
  - `target/nsight/double_ma_alma_58300_20260313.ncu-rep`
  - raw CSV export:
    - `target/nsight/double_ma_alma_58300_20260313_raw.csv`
  - first exact benchmark kernel group from the raw CSV shows:
    - `double_cross_backtest_tm_flex_f32_m7`: about `99.064 ms`
    - `alma_batch_tiled_f32_2x_tile256_tm`: about `1.198 ms` + `2.524 ms`
    - top-k and gather kernels: all in the `~0.003-0.012 ms` range

Suggested commands:

```powershell
powershell -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 `
  -ScenarioPath scripts/vectorbt_double_ma_faceoff_scenario.json `
  -OutDir target/vectorbt_vs_vectorgrid_faceoff_58300_profile

powershell -File scripts/run_vectorgrid_double_ma_faceoff.ps1

$env:VECTORBT_KERNEL_PIPELINE_PROFILE="1"
powershell -File scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1 `
  -ScenarioPath scripts/vectorbt_double_ma_faceoff_scenario.json `
  -OutDir target/vectorbt_vs_vectorgrid_faceoff_58300_pipeline
Remove-Item Env:VECTORBT_KERNEL_PIPELINE_PROFILE -ErrorAction SilentlyContinue

powershell -File scripts/profile_gpu_backtest_ncu.ps1 `
  -BenchFilter "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs"
```

## Phase 2: Decompose The Gap

Goal: explain the difference between the warm app-path number and the Criterion number before attempting optimization.

Questions to answer:

- [x] How much of the warm app-path timing is inside `result.runtime_ms` versus external harness overhead?
- [ ] How much of `result.runtime_ms` is:
  - [x] MA preparation
  - [x] device score kernel time
  - [x] reduction / top-k
  - [x] heatmap work
  - [x] tile orchestration
  - [x] host result shaping
- [x] Does the current exact `58,300` scenario fit in one tile?
- [x] Are ALMA prep costs disproportionately large relative to the score kernel?
- [x] Is the current dominant stage compute-bound, memory-bound, or launch/overhead-bound?

Completion evidence:

- [x] written decomposition of the warm-vs-Criterion gap
- [x] dominant stage ranking recorded
- [x] one explicit “do not optimize this” list for non-dominant stages

Findings:

- In the fresh warm app-path artifact, the measured GPU wall-clock median is `106.832 ms`.
- In direct GPU-only warm app-path runs, typical process medians were also around `~101-107 ms`, with intermittent slow processes around `~482-494 ms`.
- For the fast mode, the warm app-path number is already close to the per-kernel Nsight evidence (`~99 ms` backtest kernel plus low single-digit ALMA prep and negligible top-k overhead).
- The fresh Criterion helper number (`485.21 ms`) is not an app-path lower bound for this faceoff shape because the benchmark definition in `crates/app_core/benches/double_ma_opt.rs` uses:
  - `top_k=50`
  - `heatmap_bins=64`
  while the faceoff uses:
  - `top_k=10`
  - `heatmap_bins=0`
- The current exact `58,300` faceoff scenario fits in one tile.
- ALMA preparation is not the dominant cost for the fast mode.
- Reduction, top-k, and host finalization are not dominant.
- The dominant stage is the backtest score kernel itself.

Dominant stage ranking:

1. GPU backtest score kernel
2. ALMA preparation / transpose
3. everything else

Explicit non-dominant rejection list:

- heatmap work for the faceoff shape (`heatmap_bins=0`)
- host prep
- tile finalize / final finalize
- top-k gather / metric row gather
- app-path wrapper overhead as the primary explanation for the fast `~100 ms` mode

## Phase 3: Ranked Hypothesis Backlog

Goal: maintain a strict order of likely optimizations rather than jumping between ideas.

Initial candidate backlog:

### A. ALMA preparation and staging

- [x] inspect whether ALMA matrix generation or transpose/staging is dominating
- [x] test row/column layout assumptions against the current kernel access pattern
- [x] inspect whether repeated temporary allocations remain on the hot path
- [x] inspect whether graph replay or reusable launch setup can apply cleanly
  Conclusion:
  - rejected as the primary optimization target for the current fast mode
  - ALMA staging is low single-digit milliseconds in the exact Nsight target and about `1%` of the profiled pipeline split

### B. GPU backtest score kernel

- [x] confirm whether the current production backtest kernel is still the dominant cost
- [x] inspect launch shape, occupancy, warp efficiency, and DRAM throughput
- [x] inspect whether any current branch specialization can be widened for the exact faceoff configuration
- [x] inspect whether the `alma/alma`, regime-off, no-risk path has avoidable per-tile setup
  Conclusion:
  - this is the dominant stage
  - the exact fast mode already dispatches the `m7` path with `fast_path=true`, single tile, no regime, no fast-warp path
  - no robust win was found from existing runtime toggles

### C. Reduction and top-k path

- [x] confirm whether `top_k=10`, `include_all=false`, `heatmap_bins=0` is already on the cheapest practical path
- [x] inspect whether global top-k or host compaction still costs material time
- [x] inspect whether result shaping can avoid extra conversions or copies
  Conclusion:
  - rejected as a meaningful optimization target for this exact faceoff shape
  - `VECTORBT_KERNEL_GLOBAL_TOPK=0` did not produce a robust win

### D. App-path wrapper overhead

- [x] inspect whether the faceoff runner still does avoidable work per measured trial
- [x] inspect whether any JSON serialization or artifact shaping is inside the measured boundary unnecessarily
- [x] inspect whether repeated `get_candles(...)` and request construction are materially visible
  Conclusion:
  - not the primary cause of the fast-mode runtime
  - the warm app-path GPU median is already close to the kernel lower bound

### E. Explicit rejection list

- [x] reject any optimization idea that only improves a discarded or non-dominant stage
- [x] reject speculative kernel rewrites without a profiling-backed dominant-stage case

## Phase 4: One-Change Iteration Loop

Use this loop for every accepted hypothesis.

### Per-iteration checklist

- [ ] state the hypothesis
- [ ] record the exact files expected to change
- [ ] implement the narrowest viable change
- [ ] rerun the exact warm app-path benchmark
- [ ] rerun the exact Criterion anchor when relevant
- [ ] rerun Nsight or pipeline profiling if the changed stage was dominant
- [ ] rerun parity/smoke coverage
- [ ] write the result in this document
- [ ] either keep the change, revert it, or mark it rejected

### Minimum verification set per accepted change

- [ ] `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- [ ] `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel --bin ta_app_path_faceoff`
- [ ] relevant targeted `cargo test` for changed `double_ma` / kernel behavior
- [ ] `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- [ ] exact warm app-path rerun on `scripts/vectorbt_double_ma_faceoff_scenario.json`

## Iteration Log

### Iteration 1

Date: 2026-03-13
Hypothesis: the real warm app-path baseline is materially slower than the historical benchmark anchor and still needs major optimization.
Files changed: none
Expected dominant stage: mixed app-path overhead plus kernel work
Warm app-path before: stale prior note `494.757 ms`
Warm app-path after: fresh rerun `106.832 ms`
Criterion before: stale prior note `373.01 ms`
Criterion after: fresh helper artifact `485.21 ms`
Parity status: unchanged best params, same faceoff scenario
Nsight / pipeline artifact paths:
- `target/vectorbt_vs_vectorgrid_faceoff_58300_20260313/summary.json`
- `target/vectorgrid_double_ma_faceoff_20260313.json`
Conclusion:
- the stale target was wrong
- the current warm app-path path is already near `~100 ms`
Keep / Reject:
- keep the new baseline
- reject the old `~495 ms` target as the current baseline

### Iteration 2

Date: 2026-03-13
Hypothesis: non-kernel stages meaningfully contribute to the current warm app-path runtime.
Files changed: none
Expected dominant stage: mixed ALMA prep / top-k / wrapper overhead
Warm app-path before: `106.832 ms`
Warm app-path after: unchanged baseline target
Criterion before: `485.21 ms`
Criterion after: unchanged
Parity status: unchanged
Nsight / pipeline artifact paths:
- `target/vectorbt_vs_vectorgrid_faceoff_58300_pipeline_20260313/pipeline.log`
- `target/nsight/double_ma_alma_58300_20260313.ncu-rep`
- `target/nsight/double_ma_alma_58300_20260313_raw.csv`
Conclusion:
- profiled pipeline shows `tile_score` / backtest overwhelmingly dominant
- ALMA prep, top-k, heatmap, and finalization are not the main optimization target
Keep / Reject:
- keep the conclusion
- reject non-dominant stages as primary work items

### Iteration 3

Date: 2026-03-13
Hypothesis: an existing runtime toggle may unlock a robust win on the exact warm app-path shape.
Files changed: none
Expected dominant stage: same kernel path, but possibly cheaper runtime config
Warm app-path before: `~101-119 ms` fast mode
Warm app-path after: no accepted improvement
Criterion before: unchanged
Criterion after: unchanged
Parity status: unchanged
Artifacts:
- `target/double_ma_warm_opt_matrix_20260313_clean/matrix.json`
- `target/double_ma_pinned_verify_20260313/*.json`
- `target/double_ma_graphs_verify_20260313/*.json`
Conclusion:
- `VECTORBT_KERNEL_ASYNC_ALLOC=0` is clearly bad
- `VECTORBT_KERNEL_PINNED_H2D=0` and `VECTORBT_KERNEL_USE_CUDA_GRAPHS=1` showed unstable/non-robust outcomes
- no runtime toggle produced a trustworthy accepted win
Keep / Reject:
- reject all runtime-toggle changes as production defaults

### Iteration 4

Date: 2026-03-13
Hypothesis: the intermittent `~482-494 ms` slow mode is caused by kernel-branch selection instability.
Files changed:
- `crates/app_core/src/gpu_backtest_kernel.rs`
Expected dominant stage: branch-selection issue between fast and slow backtest kernels
Warm app-path before: bimodal `~101-107 ms` vs `~482-494 ms`
Warm app-path after: diagnosis only
Criterion before: unchanged
Criterion after: unchanged
Parity status: unchanged
Artifacts:
- `target/double_ma_dispatch_probe_20260313/run_*.json`
- `target/double_ma_dispatch_probe_20260313/run_*.log`
Conclusion:
- added env-gated dispatch logging via `VECTORBT_KERNEL_LOG_DISPATCH`
- both fast and slow processes still logged the same dispatch:
  - `dispatch=m7`
  - `fast_path=true`
  - `multi_tile=false`
  - `use_lockstep_time=false`
- the intermittent slow mode is therefore not explained by obvious backtest branch selection in the current source
Keep / Reject:
- keep the env-gated dispatch logger as a diagnostic aid
- reject “wrong dispatch path” as the current explanation

### Iteration Template

```text
Iteration:
Date:
Hypothesis:
Files changed:
Expected dominant stage:
Warm app-path before:
Warm app-path after:
Criterion before:
Criterion after:
Parity status:
Nsight / pipeline artifact paths:
Conclusion:
Keep / Reject:
```

## Known Starting Hypotheses

These are starting points, not accepted truths.

- [x] warm app-path is dominated by the exact production GPU pipeline rather than dataset overhead
- [x] the current app-path-vs-Criterion mismatch is largely explained by different benchmark request shapes, not just wrapper overhead
- [x] `alma/alma` prep is not a large enough fraction to justify targeting first
- [x] `top_k=10` with `heatmap_bins=0` is already near-minimal for this faceoff shape

## Stop Conditions

Declare the path exhausted only if one of these becomes true and the evidence is written down.

### Stop A: No materially dominant untested stage remains

- [x] all major stages have been profiled
- [x] no untested dominant stage remains

### Stop B: Additional optimizations are below practical value

- [x] the last serious toggle/config attempts did not produce a robust accepted improvement
- [x] no profiling evidence suggests a larger remaining win in the current product code path

### Stop C: Tradeoff becomes unjustified

- [x] further gains would require disproportionate complexity or a different investigation target
- [x] further speculative work would not be justified without first explaining the intermittent slow-mode behavior
- [x] there is no current evidence-backed product code change that would safely move the fast mode materially lower

## Final Closure Summary

Complete this only at the end.

- [x] Best warm app-path median achieved:
  - `101.639 ms` observed in the repeated March 13 fast-mode app-path probes
- [x] Best fresh faceoff median artifact:
  - `106.832 ms` in `target/vectorbt_vs_vectorgrid_faceoff_58300_20260313/summary.json`
- [x] Best Criterion helper point estimate achieved during this pass:
  - `485.21 ms` from the current helper artifact
- [x] Net warm app-path improvement versus the stale starting point:
  - the pass established that the true current warm baseline is already about `4.6x` faster than the stale `494.757 ms` figure
- [x] Net production-code improvement during this pass:
  - none accepted
- [x] Final dominant stage:
  - GPU backtest score kernel
- [x] Remaining meaningful backlog:
  - explain the intermittent slow-mode `~482-494 ms` behavior that still occurs on the same `dispatch=m7` path
  - if that instability is solved, re-evaluate whether any new product-code optimization work remains
- [x] Final recommendation:
  - [x] stop and preserve the current fast-path design
  - [ ] continue optimization
  - [ ] defer to a larger architectural change
