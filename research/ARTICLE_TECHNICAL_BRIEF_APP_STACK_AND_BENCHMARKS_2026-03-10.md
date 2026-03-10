# Technical Brief For Article Drafting

Last updated: 2026-03-10

## Executive Summary

`ta_desktop_demo` is a desktop quant-research application built around one unusually strong idea: exact, brute-force technical-strategy optimization can still be the best answer when the system is engineered aggressively enough end to end.

The app is not a toy benchmark harness. It is a full workflow system with:

- a Vue 3 + Vite desktop UI
- a Tauri command layer
- Rust strategy, validation, and meta-selection engines
- CUDA-accelerated indicator and backtest execution
- artifact persistence, report generation, and dataset management
- live data / live execution plumbing
- an ML-assisted Stage 5 meta-ranking layer

The most important technical conclusion from current research is blunt:

> In this app's use case, exact brute-force grid search on the GPU has beaten the alternative optimization algorithms we tried often enough that brute-force is now the product-default and product-preferred optimization mode.

That is the story. The innovation is not "we invented a smarter search heuristic." The innovation is that the stack is efficient enough that exact search often wins anyway.

## What The App Actually Does

At a high level, the app supports the full research workflow:

1. Load or resample market data.
2. Optimize a strategy over a parameter search space.
3. Validate shortlisted candidates.
4. Run meta-selection and holdout analysis across saved runs.
5. Optionally apply ML confidence as a post-meta ranking/filter layer.
6. Save artifacts, export reports, and inspect results in the desktop UI.

The current product stance is intentionally conservative:

- optimization is exact `Grid`
- the UI no longer presents approximate optimization algorithms as normal choices
- CUDA is used where it clearly helps
- CPU-only builds still work

## Front-End Architecture

The frontend is a Vue 3 + Vite application living in `web/`. It is no longer a hybrid legacy shell; runtime ownership is now fully in the Vue layer.

### Front-End Stack

- Vue 3
- Vite
- Pinia state stores
- Tauri invoke/event bridge
- virtualized result tables for large datasets

### Front-End Organization

The app is organized as workspace panels instead of a monolithic page. Important panels include:

- Data workspace
- Optimize workspace
- Strategy workspace
- Validation workspace
- Overview / Top / Pareto / Drill / Heatmap result workspaces
- Meta workspace
- Settings and History
- Execution workspace for live execution state

This matters for the article because the product is not just "a fast kernel." It is a research workstation.

### Front-End Runtime Pattern

The front-end uses a service/store pattern:

- Pinia stores hold long-lived UI state
- services encapsulate Tauri command invocation and response shaping
- components are mostly render/control surfaces

Key properties of the current UI architecture:

- heavy panels are lazy-loaded
- task/progress updates are throttled to prevent UI churn
- large result tables are virtualized
- saved runs, meta artifacts, Stage 5 artifacts, and policy-evaluation artifacts are first-class UI objects

### Optimize UI

The Optimize workspace is intentionally simpler now than in earlier iterations.

It exposes:

- strategy selection
- backend selection
- objective selection
- MA range selection
- MA-type parameter controls and parameter sweeps
- GPU capability hints and parity helpers

The key product simplification is that optimization mode complexity has been removed from the normal workflow. The app normalizes product-facing requests to exact `Grid`.

### Meta UI And Stage 5

The Meta workspace is one of the strongest parts of the product from a workflow perspective. It supports:

- strategy/timeframe-scoped meta selection
- holdout evaluation
- grouped/meta candidate analysis
- saved-meta artifact browsing
- saved Stage 5 artifact browsing
- ML-confidence model status/train/apply/delete
- policy-evaluation previewing

This is notable because many strategy apps stop at optimization. This app continues into candidate adjudication and meta-ranking.

## Tauri Bridge And Desktop Command Surface

The desktop app uses Tauri as the boundary between the Vue UI and the Rust backend.

The command surface is intentionally split into focused modules instead of one huge gateway:

- `commands_data.rs`
- `commands_optimize.rs`
- `commands_validation.rs`
- `commands_meta.rs`
- `commands_analysis.rs`
- `commands_execution.rs`
- `commands_artifacts.rs`
- `commands_reports.rs`
- `commands_reports_api.rs`
- `commands_system.rs`

This split is important to mention because the app is not just "GPU code plus a thin shell." The desktop contract is a substantial part of the system design.

### What The Command Surface Exposes

The app can currently do all of the following from the UI:

- load data, pick CSVs, and resample data
- manage Polygon dataset caching and downloads
- run optimization
- cancel runs
- run validation
- apply cross-ticker validation
- compute drilldown and local-stability analysis
- run meta selection
- run Stage 5 ML-confidence ranking
- evaluate Stage 5 policy variants
- save, load, list, tag, reveal, and delete artifacts
- build and save reports
- inspect GPU status
- drive live execution runner state

For an article, this is an important point: the product is an end-to-end research platform, not just a benchmark screenshot.

## Rust Workspace And Backend Structure

The Rust workspace is split into meaningful crates:

- `src-tauri/` -> desktop application crate
- `crates/app_core/` -> optimization, validation, meta, reporting logic
- `crates/strategies/` -> strategy execution primitives
- `crates/optimizer/` -> shared optimization abstractions and legacy research surfaces

### Backend Design Priorities

The current backend has been shaped around a few invariants:

- exact optimization quality matters
- CUDA must be optional at build time
- CPU-only builds must remain valid
- refactors should preserve request/response behavior
- hot paths should not be abstracted into slow generic layers

## Optimization Engine

The center of gravity for the app is the `double_ma` optimization engine in `crates/app_core/src/double_ma.rs` and its extracted helper modules.

### Supported Backend Modes Internally

Internally the backend still knows about several execution/optimization concepts:

- `CpuOnly`
- `GpuOnly`
- `GpuKernel`
- `GpuHybridKernel`
- legacy/research approximate modes like `CoarseToFine` and `GpuVdps`

### Product Reality

Product-facing optimization has been intentionally normalized to:

- exact `Grid`
- backend chosen as `Auto`, `CpuOnly`, or unified GPU routing

That was not an accident. It was an evidence-driven product decision.

### Why The GPU Path Matters

The GPU kernel route keeps as much of the Double MA workflow as possible on the GPU:

- MA generation
- tile/transposed preparation
- crossover/backtest evaluation
- ranking/top-k-oriented result handling

For supported request shapes, this avoids the classic trap where the GPU computes indicators but the host still pays most of the orchestration cost.

## Validation Engine

Optimization is only Stage 1. The app has a real validation layer, not a token "rerun the best row" step.

The validation pipeline is shared structurally, with strategy modules acting as adapters:

- `validation_core.rs`
- `validation_pipeline.rs`
- `validation_stats.rs`
- `stats_common.rs`

Strategy-specific validation modules:

- `double_ma_validate.rs`
- `composite_validate.rs`

### What Validation Adds

Validation adds:

- post-optimization candidate filtering
- strategy-specific candidate execution
- probability-style gates and robustness scoring
- CSCV/fold-aware logic
- final ranking and post-processing
- cross-ticker application

For article purposes, this is important because the system does not assume that an in-sample optimizer winner is enough.

## Meta Selection, Holdout, And Stage 5 ML Confidence

The app goes beyond optimization and validation into cross-run intelligence.

### Meta Selection

Meta selection groups and ranks prior validated artifacts, with optional holdout evaluation. It supports:

- grouped candidate analysis
- holdout-tag-aware comparisons
- final score adjustment using holdout statistics
- saved meta artifacts

### Holdout

Holdout support is not cosmetic. It is used in:

- validation scoring
- meta-selection scoring
- ML labeling for Stage 5

The app can also store per-dataset holdout rows so the ML layer learns from richer labels rather than only aggregate pass/fail summaries.

### Stage 5 ML Confidence

The ML layer is best described as a post-meta filter/reranker rather than an autonomous alpha engine.

It currently uses:

- scope-aware models by strategy kind + timeframe
- features derived from meta artifacts and holdout behavior
- grouped CV
- calibration
- AUC/Brier-based health checks
- policy evaluation across meta-only, ML-only, and blended ranking arms

This is an unusually sane use of ML in a trading-research app. It is not trying to predict raw price directly. It is trying to improve triage on already-vetted candidate groups.

## Data And Execution Features

The app also contains meaningful data and execution plumbing:

- CSV loading and resampling
- repo sample-data discovery
- Polygon cache/download helpers
- live candle stream controls
- dataset repair / validation utilities
- live execution runner status
- Alpaca connectivity test flow
- live signal execution request/response surface

This broadens the article narrative. The app is not just an offline backtester.

## Artifact And Report System

One underappreciated strength of the app is that it treats outputs as durable artifacts, not transient UI state.

It supports:

- saved run artifacts
- saved meta artifacts
- saved Stage 5 artifacts
- saved policy-evaluation artifacts
- tag updates
- reveal/open/delete flows
- report and workbook export
- CSV shortlist export

This matters in practice because serious research tooling needs reproducibility and auditability, not only speed.

## Performance Engineering Philosophy

The app's performance story is not one trick. It comes from many layered decisions:

- GPU-resident kernels where constraints allow it
- careful backend routing
- exact-grid execution rather than speculative shortcuts
- feature-aware benchmark separation
- virtualized UI result tables
- throttled task/progress updates
- optional pinned H2D behavior
- benchmark gates and regression baselines
- Nsight-guided CUDA investigation

The interesting part is that performance engineering did not end in "therefore use a fancy heuristic search." It ended in "exact search is fast enough to dominate more often than expected."

## Benchmarks Executed For This Brief

The benchmark results below were rerun serially on 2026-03-10 after resetting the GPU to a standard clock profile. The numbers used here are the confirmed direct Criterion runs for the cherry-picked benchmarks.

### Hardware / Environment

- GPU: NVIDIA GeForce RTX 4090
- Driver: 595.71
- CUDA-enabled bench path: `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel`

### 1. Small Exact-Grid GPU Kernel Reference Run

Command:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/sma_sma/200k/small/1590pairs" --sample-size 20 --measurement-time 3 --warm-up-time 3
```

Result:

- benchmark: `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`
- time: `85.863 ms` to `86.806 ms`
- point estimate: `86.326 ms`

Interpretation:

- this remains a compact reference benchmark for the unified GPU kernel path
- after the clock normalization, it settled in the mid-`86 ms` range on this machine

### 2. Large ALMA x ALMA Exact Brute-Force Benchmark

Command:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large" --sample-size 10 --measurement-time 3 --warm-up-time 1
```

Result:

- benchmark: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- time: `88.666 ms` to `90.120 ms`
- point estimate: `89.383 ms`

Derived throughput:

- `58,300` complete backtests in about `89.383 ms`
- about `652,255` backtests/sec
- about `1.534 us` per backtest
- about `130.5 billion pair-bars/sec` on a `58,300 x 200,000` workload

Why this is a "wow" result:

This is not a tiny microbenchmark. It is a full exact-grid search over `58,300` valid ALMA/ALMA parameter pairs on `200,000` bars. The app is not approximating the answer here. It is brute-forcing the exact space.

### 3. Same ALMA Benchmark Under A 1 GB VRAM Budget

Same benchmark family, but with the batched `1 GB` budget variant:

- benchmark: `double_ma/gpu_kernel/alma_alma/200k/large_batched_1gb/58300pairs`
- time: `177.65 ms` to `179.25 ms`
- point estimate: `178.48 ms`

Derived throughput:

- about `326,647` backtests/sec
- about `3.061 us` per backtest

Why this matters:

The app still remains very fast even when the search has to respect a tighter VRAM envelope. That is an important product-strength story because real deployment conditions are not always "infinite memory, single perfect benchmark."

### 4. Strategy-Overlay Benchmark, Not Just Plain Crossovers

Command:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_strategy/alma_alma/200k/trend_quality/large" --sample-size 10 --measurement-time 3 --warm-up-time 1
```

Result:

- benchmark: `double_ma/gpu_kernel_strategy/alma_alma/200k/trend_quality/large/58300pairs`
- time: `125.81 ms` to `129.28 ms`
- point estimate: `127.74 ms`

Derived throughput:

- about `456,396` backtests/sec
- about `2.191 us` per backtest

Why this matters:

The GPU path is not only fast for the plain Double MA case. It still handles a more opinionated strategy overlay at high throughput.

## The Most Impressive Results To Emphasize

If the article needs a short list of headline numbers, these are the ones to lead with:

1. `58,300` exact ALMA/ALMA backtests on `200,000` bars in about `89.38 ms`.
2. Roughly `652k` exact backtests/sec on that workload.
3. Roughly `130.5 billion pair-bars/sec` on the same exact-grid run.
4. Even under a `1 GB` VRAM budget, the same workload still lands around `178 ms`.
5. A strategy-overlay ALMA/ALMA run still lands around `127.74 ms`, showing the speed is not limited to the simplest profile.

If the goal is to "wow" the reader, the strongest framing is:

> The app does not win by cheating with approximation first. It wins by making exact search shockingly cheap.

## Brute-Force Versus Alternative Optimization Algorithms

This is one of the most article-worthy findings in the entire project.

The app did not simply assume brute-force was best. Alternative optimization algorithms were attempted and evaluated, including GPU-progressive / `GpuVdps`-style approaches intended to stop early and inspect promising regions of the space.

### What We Found

Using saved real-data evaluator artifacts on AAPL 1-minute history since 2012, resampled into higher timeframes, the conclusion was:

- approximate search could often recover the exact winner
- but the overhead frequently erased the theoretical benefit
- and in several realistic workloads, exact brute-force grid search was still faster anyway

### Real-Data Evaluator Evidence

The saved evaluator artifacts compared exact `Grid` against the progressive search approach on real AAPL data across:

- `plain`
- `trend_quality`
- `breakout_momentum`

and across:

- `5m`
- `15m`

with very large search spaces.

### AAPL 15m, XLarge Space

Artifact: `target/real_data_optimizer_eval_aapl_15m_xlarge_strategies.json`

Summary:

- progressive was faster in `6/6` cases
- progressive still matched the exact-grid winner in `6/6` cases

This is the best-case scenario for the alternative optimizer, and it is worth mentioning because it proves the research was fair.

### AAPL 15m, XXLarge Space

Artifact: `target/real_data_optimizer_eval_aapl_15m_xxlarge_strategies.json`

Summary:

- exact grid was faster in `5/6` cases
- progressive was faster in only `1/6`
- progressive still matched the exact best in `6/6`

Representative examples:

- `plain/sma_sma`: grid `2282 ms`, progressive `1629 ms`
- `plain/alma_alma`: grid `1555 ms`, progressive `1604 ms`
- `trend_quality/sma_sma`: grid `1223 ms`, progressive `1695 ms`
- `trend_quality/alma_alma`: grid `1601 ms`, progressive `1723 ms`

The important point is that once the workload changed shape, the fancy search no longer dominated.

### AAPL 5m, XLarge Space

Artifact: `target/real_data_optimizer_eval_aapl_5m_xlarge_strategies.json`

Summary:

- exact grid was faster in `5/6` cases
- progressive was faster in only `1/6`
- progressive still matched the exact best in `6/6`

### AAPL 5m, XXLarge Space

Artifact: `target/real_data_optimizer_eval_aapl_5m_xxlarge_strategies.json`

Summary:

- exact grid was faster in `6/6` cases
- progressive matched the exact winner in only `5/6`
- one miss landed at exact rank `10` with non-zero regret

Representative miss:

- `breakout_momentum/sma_sma`
- progressive best exact rank: `10`
- regret: `0.000268783767675601`

### Holdout-Recovery Findings

The real-data evaluator also tracked how many holdout-positive top candidates the approximate optimizer recovered from the exact-grid shortlist.

Representative ratios:

- AAPL `15m xxlarge plain/sma_sma`: `80%`
- AAPL `15m xxlarge trend_quality/sma_sma`: `95%`
- AAPL `15m xxlarge trend_quality/alma_alma`: `50%`
- AAPL `5m xxlarge plain/alma_alma`: `90%`
- AAPL `5m xxlarge trend_quality/alma_alma`: `40%`

This matters because even when the approximate method found the best single row, it could still distort the rest of the high-quality frontier.

## The Product Decision That Followed

Because of those findings, the product direction changed:

- exact `Grid` became the explicit product default
- approximate optimization modes were removed from the normal frontend workflow
- product-facing optimization requests are normalized to exact `Grid`
- approximate modes remain, at most, as internal research leftovers rather than first-class product features

This is a good story for the article because it is evidence-driven rather than ideology-driven.

The team did not fall in love with "smart optimization." It kept the method that actually won on speed, determinism, and answer quality.

## Why This Is Genuinely Interesting

There are several article-worthy angles here.

### 1. Exact Search As A Competitive Advantage

Many research tools assume brute-force is naive and approximation is inherently more sophisticated. This app shows that once GPU execution is engineered aggressively enough, exact search can become the practical high-end answer.

### 2. End-To-End Engineering Beats Isolated Kernel Tricks

The system's performance comes from full-stack discipline:

- UI virtualization
- command-boundary cleanup
- artifact workflows
- GPU backtest routing
- validation/meta integration
- reproducible benchmarking

The app is strong because the whole workflow is coherent.

### 3. ML Is Used In A Credible Place

The ML layer is not pretending to discover alpha from raw candles. It is used after optimization and validation, where it actually has a defensible role: post-meta ranking and confidence gating.

### 4. Research Honesty Improved The Product

The app tried approximate optimizers. The benchmarks and real-data evaluation did not justify them as the default. That honesty made the product simpler and better.

## Suggested "Wow" Narrative For The Article

If you want a strong through-line, this is the one I would use:

> This app started like many quant tools do: by assuming smarter search algorithms would eventually beat brute force. But once the stack was engineered from the Vue desktop shell down to VRAM-resident CUDA kernels, the opposite kept happening. Exact search stopped being the slow, naive baseline and became the winning product strategy.

Supporting beats:

- It is a desktop quant workstation, not just a benchmark harness.
- It performs exact grid search at GPU speeds that most people would assume require approximation.
- It has a real Stage 2 validation layer and a real meta-selection layer.
- It even adds ML, but in the one place where ML is actually credible here: post-meta candidate triage.
- It removed product complexity when the evidence said the complexity was not worth it.

## Suggested Claims To Make Carefully

These are safe, strong claims:

- the app is heavily optimized for exact Double MA search
- the GPU kernel path can brute-force very large search spaces at unusually high speed
- exact brute-force grid search outperformed the attempted alternative optimization algorithms often enough to become the product default
- the system includes validation, meta-selection, holdout, artifacts, reporting, and Stage 5 ML confidence

These should be framed more carefully:

- do not claim approximate search was useless in every case; on some real-data workloads it did win
- do not claim Stage 5 ML is a standalone alpha engine; it is a post-meta ranking/filter layer
- do not claim the app "proves" brute-force is universally optimal; it shows that brute-force is the best product choice in this stack and workload family

## Reproducibility Commands

GPU perf gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

Large ALMA exact-grid GPU benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel/alma_alma/200k/large" --sample-size 10 --measurement-time 3 --warm-up-time 1
```

Strategy-overlay GPU benchmark:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_strategy/alma_alma/200k/trend_quality/large" --sample-size 10 --measurement-time 3 --warm-up-time 1
```

Real-data evaluator artifacts referenced in this brief:

- `target/real_data_optimizer_eval_aapl_15m_xlarge_strategies.json`
- `target/real_data_optimizer_eval_aapl_15m_xxlarge_strategies.json`
- `target/real_data_optimizer_eval_aapl_5m_xlarge_strategies.json`
- `target/real_data_optimizer_eval_aapl_5m_xxlarge_strategies.json`

## Bottom Line

The biggest article takeaway is not merely that the app is fast.

It is that this app arrived at a counterintuitive result through actual engineering and actual measurement:

> after building GPU-accelerated exact search, validation, meta-analysis, artifact workflows, and an ML-assisted Stage 5 layer, the winning optimization strategy was still brute-force.

That is a much stronger story than "we made another optimizer." It says the system became so efficient that the supposedly naive method won.
