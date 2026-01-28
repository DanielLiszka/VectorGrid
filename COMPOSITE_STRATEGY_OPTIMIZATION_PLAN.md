# Composite Strategy Optimizer (HalfTrend + MACZ + OTT + MA)

_Last updated: 2026-01-24_

This document is the implementation plan for a **CPU-first, Optuna-inspired, multi-objective optimizer**
for the **composite strategy**:

> Double-MA signal **gated** by HalfTrend regime + MACZ histogram + OTT line.

The existing **double-MA** optimizer remains the “brute-force batch” showcase (GPU-kernel + CPU),
and is used as a fast **pre-scan** to narrow the MA search space for the composite strategy.

---

## Current Code State (Implemented)

As of 2026-01-24, the composite multi-objective optimizer is implemented and runnable end-to-end.
It supports:
- a **general search space** where any supported parameter can be optimized (MA types/lengths, HalfTrend,
  MACZ, OTT, and selected `StrategyConfig` fields)
- **two objectives** (maximize one metric, minimize another) with **constrained-domination** (feasible
  dominates infeasible; among infeasible compare total violation)
- **optional GPU pre-scan** (stage-0) to narrow MA period domains before CPU MO optimization
- selectable samplers:
  - **NSGA-II**
  - **Random**
  - **TPE (MO)**
  - **QMC (Halton)**

Relevant files:
- Strategy logic: `crates/strategies/src/composite_cpu/mod.rs`
- Composite MO runner + request/response: `crates/app_core/src/composite_mo.rs`
- Parameter parsing/helpers: `crates/app_core/src/composite_cpu.rs`, `crates/app_core/src/double_ma.rs`
- MO sampler implementations: `crates/optimizer/src/mo.rs`
- Tauri commands: `src-tauri/src/commands.rs` (`run_composite_mo_optimization`, drilldown)
- UI: `dist/index.html`, `dist/app.js`

---

## Goal

Build a **multi-objective** optimizer for the composite strategy where **all relevant parameters** can
be optimized:

- MA selection and parameters (types, lengths, MA-specific params)
- HalfTrend params
- MACZ params
- OTT params
- StrategyConfig knobs (commission, eps band, etc.)

The optimizer must:
- remain **CPU-only** for this composite strategy (CUDA speedups likely negligible vs complexity)
- support **two objectives**: `(maximize A, minimize B)` with sensible defaults (e.g., Sharpe vs MaxDD)
- scale to large search spaces using:
  - **batch evaluation** (ask → evaluate batch → tell)
  - **tiling/caching** to reuse MA computations
  - optional **two-tier pipeline** using GPU-kernel double-MA to narrow MA search ranges

---

## Next Development Steps (Plan)

These are the remaining v1 items to make the optimizer more Optuna-like and easier to use:

- [x] Add **Sobol QMC** option (Optuna-style QMCSampler parity) alongside Halton.
- [x] Improve **MO-TPE** fidelity:
  - [x] avoid duplicates in batch suggestions (“constant liar”-like behavior)
  - [x] weight “good” samples using **2D hypervolume contribution** (approx Optuna MOTPE weighting)
- [x] UI polish:
  - [x] show MO fields in Top table: `maximize_value`, `minimize_value`, `violation`
- [x] Add tests:
  - [x] optimizer-level smoke test for Sobol QMC
  - [x] app-core smoke tests for CompositeMo sampler wiring (TPE/QMC)

---

## Strategy Definition (What A “Trial” Evaluates)

The composite strategy is essentially:

1) Compute MA crossover sign `sign ∈ {-1,0,+1}` with epsilon band (`StrategyConfig.eps_rel`), long-only
and allow-flip logic.
2) Gate that sign using:
   - **HalfTrend regime** (`halftrend_trend[t]`)
     - if regime is “bull” → forbid shorts
     - if regime is “bear” → forbid longs
   - **MACZ histogram** (`macz_hist[t]`)
     - if histogram > 0 → forbid shorts
     - if histogram < 0 → forbid longs
     - if histogram == 0 → flat
   - **OTT** (`price[t]` vs `ott_line[t]`)
     - if price > ott → forbid shorts
     - if price < ott → forbid longs
     - else → flat
3) Apply commission, exposure accounting, and compute metrics.

This gating structure is already implemented in:
`crates/strategies/src/composite_cpu/mod.rs`.

---

## Parameter Space (v1)

### A) Double-MA parameters

- `fast_ma_type` (categorical)
- `slow_ma_type` (categorical)
- `ma_source` (categorical: close/open/high/low/hl2/hlc3/ohlc4/etc.)
- `fast_len` (int)
- `slow_len` (int), with constraint `fast_len < slow_len`
- MA-specific params (conditional on ma type), e.g.:
  - ALMA: `offset`, `sigma`
  - JMA, Gaussian, etc. (whatever `vector-ta` exposes in our MA registry)

### B) HalfTrend parameters

Already modeled as:
- `amplitude: u16`
- `channel_deviation: f64`
- `atr_period: u16`

### C) MACZ parameters

Already modeled as `MaczGateParams` in `crates/app_core/src/composite_cpu.rs`:
- lengths: `fast_length`, `slow_length`, `signal_length`, `lengthz`, `length_stdev`
- scalars: `a`, `b`, `gamma`
- booleans: `use_lag`, `use_volume`

Constraints already enforced in code:
- `fast_length < slow_length`
- `a, b` finite and in `[-2, 2]`
- `gamma ∈ [0, 1)`
- if `use_volume=true`, candles must include volume

### D) OTT parameters

Already modeled as `OttGateParams`:
- `period: u16`
- `percent: f64`
- `ma_type: String` (categorical)

### E) StrategyConfig parameters

Already in `ta_strategies::double_ma::StrategyConfig`:
- `commission` (float)
- `eps_rel` (float)
- `long_only` (bool)
- `allow_flip` (bool)
- `trade_on_next_bar` (bool)

For v1, keep this set. Later: add stop-loss/TP/trailing, etc.

---

## Objectives, Directions, Constraints

### Objectives

We support two user-selected objectives:
- Objective 1: **maximize**
- Objective 2: **minimize**

Candidate metric choices (already in `Metrics`):
- `pnl` (maximize)
- `sharpe` (maximize)
- `max_dd` (minimize)
- `trades` (usually a constraint, not an objective)
- `exposure`, `net_exposure` (usually constraints or reporting)

Implementation detail:
- normalize to “maximize both” internally by using `obj2 = -metric_to_minimize`.

### Constraints (recommended)

In practice, users usually want constraints like:
- `trades >= min_trades`
- `max_dd <= dd_limit`
- `exposure <= exposure_limit`

Optuna’s NSGA-II supports constraints via a `constraints_func` and “constrained domination”
(feasible dominates infeasible; among infeasible compare total violation).
We should replicate that shape in our optimizer.

---

## Optimization Architecture (Optuna-Inspired, But Batch-Friendly)

Optuna’s **ask-and-tell** interface explicitly supports **batch optimization**:
ask for many trials → evaluate them in a batch → tell results back (and optionally use
techniques like “constant liar” to avoid duplicated suggestions in parallel runs).

We should model our optimizer similarly:

### Core interfaces

- `SearchSpace`: typed parameter definitions (float/int/categorical + log/step)
- `Sampler`: algorithm that produces candidate parameter sets (“asks”)
- `Evaluator`: CPU backtest that takes a batch of trials and returns metrics
- `Archive/Aggregator`: maintains Pareto front, top-K, best-by-weighted-score, etc.

Pseudo-shape:

```text
for gen in 0..:
  trials = sampler.ask(batch_size, archive_state)
  results = evaluator.eval_batch(trials)   # parallel
  archive.update(results)
  sampler.tell(results, archive_state)
```

**Key point:** update the sampler **per batch** (or per generation), not per backtest call in the inner loop.

---

## Samplers to Implement (Priority Order)

### P0 (must-have)

1) **RandomSampler**
   - Baseline, robust, easy to test.
2) **NSGA-II** (multi-objective genetic algorithm)
   - This is the most direct match to Optuna’s `NSGAIISampler`:
     - `population_size`
     - `crossover` (uniform/sbx/etc.)
     - `mutation_prob`
     - constrained domination (constraints support)

### P1 (nice-to-have)

3) **TPE-like sampler** for mixed spaces
   - Optuna’s TPE supports batch optimization via `constant_liar`.
   - Multi-objective TPE exists (MOTPE), but implementing it well is more work than NSGA-II.
4) **QMC / Sobol sampler**
   - Useful for early “space-filling” stages.

### Not in v1 (unless strongly desired)

- Hyperband/Successive Halving “pruning”
  - Optuna pruners are primarily designed for **single-objective** and require intermediate reporting.
  - For trading, “partial training curves” map to “partial backtest windows”, which can be misleading.

---

## Performance Plan (CPU)

The expensive part is computing indicators across `n` candles for each trial.
To keep throughput high:

### 1) Reuse MA computations aggressively

We should avoid recomputing MAs for every trial.

Options:
- **Cache per (ma_type, ma_source, ma_params_hash, period)** → `Vec<f64>`
  - Great reuse, but can grow large; needs eviction strategy.
- **Batch period computation** (preferred)
  - Collect unique fast/slow periods required by a batch and compute them together.
  - For contiguous ranges, use the existing `ma_matrix_cpu_f64` approach.

### 2) Gate indicators are per-trial

HalfTrend/MACZ/OTT parameters vary per trial, so we likely compute these per trial.
Mitigations:
- reuse allocations using scratch buffers (`Vec<f64>` reused per worker)
- compute only what we need (start index/warmup handling)
- keep everything single-pass and branch-light

### 3) Batch size and threading

Tune batch size based on:
- candle length
- number of threads
- memory bandwidth

Reasonable starting points:
- `batch_size`: 64–512
- `population_size` (NSGA-II): 50–200

---

## Two-Tier Pipeline: GPU Pre-Scan → CPU Composite Optimization

This is the recommended way to keep MA search under control while still allowing “all params” optimization.

### Stage 0: GPU-kernel Double-MA Pre-Scan (fast)

Run the existing GPU-kernel double-MA optimizer to identify promising MA length regions:
- choose a **compatible MA-type pair** (GPU kernel supports a subset)
- run once for objective1, once for objective2 (or a proxy), then union results:
  - objective2 (minimize) can be run as maximize `-objective2`

Outputs:
- top-K MA pairs
- optional heatmap to identify good regions

Convert outputs into candidate sets:
- expand each top pair into a window: `(fast_len ± W, slow_len ± W)`
- dedupe/sort to produce `fast_periods_candidate`, `slow_periods_candidate`

### Stage 1: CPU Multiobjective Composite Optimization

Run NSGA-II / Random / (later TPE) on the full parameter space, **but restrict**
`fast_len`/`slow_len` to the candidate sets derived above.

This keeps the optimizer focused while still allowing HalfTrend/MACZ/OTT/MA-params to vary.

### Stage 2: Optional Refinement

For Pareto-front solutions, optionally refine:
- widen MA windows locally around each Pareto point
- re-evaluate on full data, and/or multiple date splits

**Caveat:** double-MA “best” regions won’t always remain best under gating.
That’s why we use the GPU scan as a *broad candidate generator* (top-K + window), not a single best pair.

---

## Data Model / API Changes (Backend ↔ UI)

We need a new request/response pair for the composite optimizer v2:

### Request

- `data_id`
- `objectives`: `{ maximize: MetricKind, minimize: MetricKind }`
- `constraints` (optional): min trades, max dd, etc.
- `sampler` config:
  - kind: random | nsgaii | (future: tpe)
  - seed
  - budget: trials or generations
  - batch_size / population_size
- `search_space` config:
  - ranges for ints/floats, categorical sets
  - conditional params for MA types and OTT MA type
- `gpu_prescan` config (optional):
  - enable/disable
  - top_k
  - window size W
  - which MA type pairs to scan

### Response

Return both a “human friendly” summary and full data for UI plots:
- Pareto front: `Vec<(params, metrics, obj1, obj2, constraint_violation)>`
- best-by-objective extremes (max obj1, min obj2)
- optional “pick one” selection via user-chosen scalarization (weights)
- runtime, #trials, etc.

---

## Work Breakdown (Suggested Milestones)

### M1: Multiobjective core (no UI yet)

- Add multiobjective objective/constraint representation in `ta_optimizer`
- Implement:
  - Pareto archive (non-dominated set + bounded size + crowding distance)
  - NSGA-II sampler (mixed discrete/continuous/categorical)
  - Random sampler baseline
- Unit tests for:
  - dominance + constrained dominance
  - Pareto extraction
  - determinism with fixed seed (single-thread)

### M2: Composite evaluator for arbitrary params

- Build an evaluator that can consume “trial param sets” and:
  - compute HalfTrend/MACZ/OTT for each trial
  - reuse MA computations for the batch
  - return `Metrics` + derived objective values + constraint violations

### M3: GPU pre-scan integration

- Add a helper that runs GPU-kernel double-MA to generate candidate MA sets.
- Plumb this into composite optimizer v2 as an optional pre-step.

### M4: Tauri + UI integration

- Add Tauri commands for composite optimizer v2 (keep old one for now).
- UI:
  - objective pickers (maximize/minimize)
  - sampler selection + budget controls
  - Pareto scatter + table
  - drilldown for selected Pareto point

### M5: Benchmarks + polish

- Criterion bench for composite evaluator throughput.
- Add “safe defaults” to avoid gigantic runs without user intent.

---

## Open Questions / Decisions Needed

1) **Parameter bounds** for v1:
   - HalfTrend amplitude range? ATR period range? channel deviation range?
   - MACZ/OTT ranges that are meaningful and not absurdly large?
2) **MA types to include** in composite optimization:
   - All CPU-supported MAs, or only a curated subset?
3) **Default objectives + constraints**
   - Default: maximize Sharpe, minimize MaxDD, constrain trades >= N?
4) **Reproducibility**
   - How do we want determinism under Rayon parallelism? (Optuna itself notes reproducibility issues when parallel.)
5) **Overfitting controls**
   - Optional walk-forward or train/validation split integration?
