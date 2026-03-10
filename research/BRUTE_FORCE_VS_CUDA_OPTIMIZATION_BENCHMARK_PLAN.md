# Brute-Force vs CUDA Search Algorithm Benchmark Plan

Date: 2026-03-09

## Goal

Determine when the current brute-force GPU optimization path (`mode=Grid`) is still the best practical choice, and when CUDA search algorithms such as `GpuVdps` or `CoarseToFine` actually earn their overhead.

The key question is not just "which mode is faster?" It is:

1. When is brute-force exact grid still the best time-to-answer?
2. When does an algorithmic search become faster enough to justify approximation overhead?
3. What quality loss, if any, is introduced when the faster algorithm wins?

## Current State In This Repo

The existing infrastructure is a good base, but it does not yet answer the full question.

### What already exists

- Main `double_ma` optimization benchmark harness:
  - `crates/app_core/benches/double_ma_opt.rs`
- Existing backend comparison doc:
  - `benchmark_unification_matrix_alma_2026-02-28.md`
- Existing GPU perf gate:
  - `scripts/run_gpu_perf_gate.ps1`
- Existing Nsight Compute helper:
  - `scripts/profile_gpu_backtest_ncu.ps1`

### What the current benchmarks already cover well

- `Grid` runs for:
  - `CpuOnly`
  - `GpuOnly`
  - `GpuKernel`
  - `GpuHybridKernel`
- MA-pair coverage for:
  - `sma_sma`
  - `alma_alma`
  - many extra MA pairs under `gpu_kernel_extra`
- Strategy overlays on the GPU kernel path for:
  - `marrt`
  - `pit`
  - `set`
  - `mts`
  - `orc`
  - `vfct`
  - `trend_quality`
  - `regime_hybrid`
  - `breakout_momentum`
- Existing exact-grid vs algorithm comparison:
  - `double_ma/gpu_kernel_vdps/sma_sma/200k`
  - compares `Grid` vs `GpuVdps`

### Current gaps

- The main benchmark groups are still mostly "feature-on" runs:
  - `top_k=50`
  - `heatmap_bins=64`
- The current `Grid` vs `GpuVdps` comparison is narrow:
  - only `sma_sma`
  - only 200k candles
  - only a few large workloads
  - no `alma_alma`
  - no strategy overlays
- There is no benchmark coverage for `CoarseToFine`.
- The perf gate checks only one small `sma_sma` case, so it is not useful for answering the broader search-mode question.
- Nsight Compute support exists, but there is no repo-level Nsight Systems flow yet for timeline-level study of orchestration overhead.

## Core Research Principle

This should be treated as a **time-vs-quality frontier** study, not a simple speed contest.

Why:

- `Grid` is exact for the defined search space.
- `GpuVdps` and `CoarseToFine` trade search completeness for less work.
- If an algorithm is faster but misses materially better candidates, it is not actually better.

So every algorithmic run should be judged against a brute-force reference for the same scenario.

## Primary Hypotheses

1. Brute-force `Grid` will remain best for small and medium discrete spaces, especially when the GPU kernel path is highly specialized.
2. Algorithmic search will only win reliably once the candidate space or VRAM pressure grows enough that exact enumeration starts paying a large orchestration or batching cost.
3. `GpuVdps` is more likely to win on very large spaces than on ALMA/ALMA-class moderate spaces.
4. Strategy complexity matters:
   more expensive per-pair backtests make search overhead less dominant and improve the odds that algorithmic search pays off.
5. "Feature-on" requests can hide the true tradeoff:
   heatmap/top-k/export overhead must be separated from optimizer-core costs.

## Benchmark Design

## Study Structure

Run the research in three layers.

### Layer 1: Optimizer-core study

Purpose:
Measure the search-mode tradeoff with minimal extra feature noise.

Request rules:

- `include_all=false`
- `heatmap_bins=0`
- `export_all_csv_path=None`
- fixed objective
- fixed dataset
- same ranges and strategy per compared run

Modes to compare:

- `Grid`
- `CoarseToFine`
- `GpuVdps`
- optionally `Auto` as a routing sanity check, but not as a primary research datapoint

### Layer 2: Feature-on study

Purpose:
Measure whether the answer changes once real product features are turned on.

Request rules:

- `top_k=50`
- `heatmap_bins=64`
- same compared modes where supported

Important:

- `CoarseToFine` and `GpuVdps` currently require `heatmap_bins=0`, so this layer mainly tells us how much "real request" overhead favors or disfavors brute-force in production-style runs.
- This layer should not replace the optimizer-core study.

### Layer 3: Quality frontier study

Purpose:
Measure whether faster algorithmic modes stay acceptably close to brute-force quality.

Compare each algorithmic result against the brute-force reference for the same scenario using:

- top-1 objective regret
- top-k overlap
- rank correlation for finalists
- whether the brute-force top-1 appears in the algorithmic top-k

## Scenario Matrix

The research should be broad enough to be credible, but not so broad that it becomes noise.

### Axis A: Strategy family

Start with double-like strategies, because they share the same optimization skeleton and already have GPU-kernel benchmark coverage.

Tier 1:

- `double_ma`
- `alma_alma`
- `sma_sma`

Tier 2:

- `trend_quality`
- `regime_hybrid`
- `breakout_momentum`
- `vfct`

Reason:

- This gives one pure baseline, one ALMA-heavy case, and several more realistic strategy overlays with different per-pair costs.

### Axis B: Search space size

Use at least four search-space tiers:

- small: ~1e3 to 1e4 valid pairs
- medium: ~1e4 to 1e5 valid pairs
- large: ~5e4 to 2e5 valid pairs
- huge: ~2e5 to 1e6+ valid pairs

For double-MA-style ranges, the exact ranges should be chosen so that the number of valid `fast < slow` pairs lands in those buckets.

### Axis C: Series length

Use at least:

- `50k`
- `200k`
- `1m`

Reason:

- Search overhead matters more at shorter series.
- Per-pair backtest cost matters more at longer series.

### Axis D: VRAM pressure

Use at least:

- unconstrained
- moderate cap
- near-capacity cap

Suggested implementation:

- default environment
- `VECTORBT_KERNEL_VRAM_BUDGET_MB=1024`
- one more tighter budget chosen from actual device limits

### Axis E: Determinism

Run algorithmic modes under:

- deterministic on
- deterministic off
- multiple seeds where applicable

Reason:

- If an algorithm only wins in a nondeterministic mode with unstable ranking quality, that matters.

## Dataset Plan

Use both synthetic and realistic datasets.

### Synthetic datasets

These already exist or can be built easily from the current benchmark harness:

- smooth trend
- breakout / volatility regime change
- low-noise synthetic drift

Synthetic data is useful for controlled reproducibility and targeted stress cases.

### Realistic datasets

Add a small frozen dataset pack of real candles and keep it versioned or artifacted for reproducibility.

Recommended minimum:

- 1 large-cap equity
- 1 index ETF
- 1 crypto pair
- 1 high-volatility growth asset
- 1 sideways / mean-reverting asset if available

Rules:

- fixed timeframes
- fixed file snapshots
- no live-downloaded data during the benchmark run

## Metrics

## Primary decision metrics

- wall-clock runtime
- throughput: candidate-pairs per second
- time-to-top-1
- time-to-top-k
- top-1 objective regret vs brute-force
- top-k overlap vs brute-force

## Secondary metrics

- full-fidelity evaluations performed
- batches or rungs executed
- peak VRAM
- H2D / D2H transfer bytes
- kernel occupancy
- achieved memory bandwidth
- deterministic repeatability

## Product-facing metrics

- did the mode return the brute-force top-1 exactly?
- if not, how far off was it?
- how often does the algorithmic mode change the selected candidate enough to matter?

## Measurement Methodology

### 1. Separate macrobenchmarks from profiler runs

Use Criterion for stable timing comparisons.

Use Nsight only on a reduced subset of scenarios for diagnosis, not for the headline speed table.

Reason:

- Nsight Compute adds replay overhead.
- NVIDIA documents that profiling overhead depends on the number of sections, metrics, kernels, and replay passes.

### 2. Always keep one exact brute-force reference

For every scenario used to judge an algorithmic mode:

- run exact `Grid`
- save the top result
- save the top-k set
- compare the algorithmic result against that reference

This is non-negotiable. Otherwise we cannot tell whether the algorithm is "faster because it did less useful work" or "faster with acceptable quality."

### 3. Benchmark both feature-off and feature-on shapes

Use two standard request shapes:

Optimizer-core shape:

- `heatmap_bins=0`
- `top_k=50`
- no export

Feature-on shape:

- `heatmap_bins=64`
- `top_k=50`

Reason:

- This separates search overhead from post-processing overhead.

### 4. Use repeated runs with confidence intervals

For timing:

- Criterion sample size at least `10`
- longer measurement windows for large GPU runs
- discard obvious unstable runs caused by machine contamination only if documented

For algorithmic quality:

- repeated seeds when applicable
- repeated deterministic runs to verify rank stability

### 5. Add timeline instrumentation for phase attribution

Use NVTX ranges around:

- candidate generation
- MA bank prep
- transpose / layout prep
- backtest kernel phase
- top-k / ranking
- heatmap build
- algorithm rung transitions

This allows Nsight Systems to answer:

- where time is actually going
- whether algorithmic search reduces kernel work enough to offset orchestration
- whether launch/host overhead dominates at smaller spaces

### 6. Limit Nsight Compute scope

Profile only a few representative kernels and launches:

- brute-force winning case
- algorithmic winning case
- ambiguous crossover case

Avoid full-set NCU captures for the entire matrix.

Reason:

- NVIDIA documents that collecting more sections and more kernels increases profiling overhead.

## Recommended Benchmark Matrix

Start with this minimum matrix.

### Phase 1: Answer the core question quickly

Strategies:

- `sma_sma`
- `alma_alma`

Modes:

- `Grid`
- `CoarseToFine`
- `GpuVdps`

Series lengths:

- `50k`
- `200k`

Search sizes:

- medium
- large
- huge

VRAM:

- default
- 1 GB cap

This phase is the shortest path to finding the rough crossover point.

### Phase 2: Check realism

Strategies:

- `trend_quality`
- `regime_hybrid`
- `breakout_momentum`

Modes:

- `Grid`
- best algorithmic challenger from Phase 1

Series lengths:

- `200k`
- `1m`

Search sizes:

- medium
- huge

This phase tests whether the same conclusion holds under more realistic strategy cost.

### Phase 3: Quality stress test

Run only algorithmic modes on scenarios where they look competitive, then compare against brute-force.

Extra checks:

- 10 seeds
- deterministic on/off
- top-1 regret
- top-10 overlap

## What We Should Not Do

- Do not compare `Grid` feature-on against `GpuVdps` feature-off and call it an algorithm comparison.
- Do not use `Auto` routing as the primary research subject.
- Do not use only one MA pair such as `alma_alma` and generalize to the entire backend.
- Do not judge algorithmic search only by raw runtime without quality comparison.
- Do not run full Nsight Compute across the entire matrix; that will distort the work and waste time.

## Recommended Implementation In This Repo

### Bench harness additions

Extend `crates/app_core/benches/double_ma_opt.rs` with:

- a dedicated `search_mode_core` group
  - compares `Grid`, `CoarseToFine`, `GpuVdps`
  - `heatmap_bins=0`
  - same exact ranges and datasets
- an `alma_alma` version of the existing `gpu_kernel_vdps` comparison
- a strategy-overlay search-mode group
  - compare `Grid` vs algorithmic modes on a smaller scenario matrix
- a `1m` candles group for long-series crossover behavior

### Artifact/reporting additions

Each benchmark scenario should write or expose:

- scenario id
- mode
- backend
- candles length
- search-space size
- deterministic flag
- seed
- runtime summary
- best candidate
- top-k candidates
- brute-force comparison fields for algorithmic modes

### Profiling additions

Add a new Nsight Systems helper script and annotate code with NVTX ranges around major optimization stages.

Suggested new script:

- `scripts/profile_gpu_backtest_nsys.ps1`

Purpose:

- capture timeline-level overhead
- correlate host orchestration with GPU work
- restrict capture to marked ranges instead of whole-run noise

## Decision Rule

At the end of the study, each scenario should be classified into one of three buckets.

### Bucket A: Brute-force wins

Use brute-force as default when:

- `Grid` is faster or within a small margin of the challenger
- and it remains exact

### Bucket B: Algorithmic mode earns it

Use an algorithmic mode when:

- it is materially faster
- and quality loss is within an agreed tolerance

Suggested tolerance:

- top-1 regret near zero
- top-10 overlap high
- selected candidate stable enough for downstream validation

### Bucket C: Needs routing heuristic

Use heuristic routing when:

- winner depends strongly on search size, series length, or VRAM pressure

This is the likely long-term outcome.

## Concrete Success Criteria

The research should be considered conclusive enough to influence defaults only if:

1. We identify at least one repeatable crossover boundary where algorithmic search clearly beats brute-force.
2. We identify at least one repeatable region where brute-force remains the better default.
3. The conclusion holds across more than one strategy shape.
4. The quality loss of the algorithmic winner is measured, not assumed.
5. The main cause of wins/losses is explained by profiler evidence, not just timing tables.

## Checklist

- [ ] Add a dedicated benchmark group for search-mode comparisons with `heatmap_bins=0`.
- [ ] Add `CoarseToFine` benchmark coverage.
- [ ] Extend the existing `Grid` vs `GpuVdps` comparison to `alma_alma`.
- [ ] Add a long-series (`1m`) benchmark tier.
- [ ] Add a small frozen set of real candle datasets for benchmark realism.
- [ ] Standardize scenario ids so results can be aggregated automatically.
- [ ] Persist brute-force top-1 and top-k references for comparison.
- [ ] Add regret and top-k-overlap reporting for algorithmic modes.
- [ ] Run the minimum Phase 1 matrix.
- [ ] Identify provisional crossover points by search size and series length.
- [ ] Run the Phase 2 realism matrix on strategy overlays.
- [ ] Run repeated-seed and deterministic stability tests for algorithmic modes.
- [ ] Add NVTX ranges around major optimization stages.
- [ ] Add a repo helper for Nsight Systems capture.
- [ ] Profile one brute-force winner, one algorithmic winner, and one crossover case with Nsight Systems.
- [ ] Profile representative hot kernels with Nsight Compute using limited sections.
- [ ] Produce a final comparison table with runtime, regret, overlap, and profiler notes.
- [ ] Convert the research result into routing heuristics only after the matrix is complete.

## Useful Existing Files

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/src/double_ma.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `benchmark_unification_matrix_alma_2026-02-28.md`
- `research/STAGE6_VRAM_AWARE_GPU_SEARCH_VARIANT.md`

## External Guidance

These were useful in shaping the methodology:

- NVIDIA CUDA C++ Best Practices Guide
  - https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/
- NVIDIA Nsight Systems User Guide
  - https://docs.nvidia.com/nsight-systems/UserGuide/
- NVIDIA Nsight Compute Profiling Guide
  - https://docs.nvidia.com/nsight-compute/ProfilingGuide/

Key takeaways applied here:

- use realistic workloads
- separate timing from intrusive profiling
- use capture ranges / NVTX for targeted timeline study
- limit Nsight Compute scope because replay and metric collection add overhead
- judge performance with bandwidth/occupancy/timeline evidence, not wall time alone
