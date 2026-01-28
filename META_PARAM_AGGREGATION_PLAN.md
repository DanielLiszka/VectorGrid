# Meta Parameter Aggregation / “Set-and-Forget” Selection (Design Plan)

_Draft created: 2026-01-26_

## Problem statement

We already support:
- per-run optimization (Double-MA, Composite MO/CPU)
- an optional (but hands-off by default) validation layer (walk-forward, perturbations, trade bootstrap, cross-ticker)
- report export (XLSX) + saved run artifacts (disk)

What we **do not** yet support is a **meta-selection** layer that:
- aggregates *validated* parameter candidates across many runs (e.g., 1,000 symbols for the same timeframe),
- filters down to a small pool of “most robust” parameter sets/regions,
- provides an evidence-based recommendation intended to be closer to “set-and-forget”.

This document plans that meta layer.

> Important: no process can guarantee profitability. The goal is to reduce the probability that we pick a parameter set
> that is only “lucky” in-sample, by designing an explicit selection pipeline with proper out-of-sample discipline.

---

## Why “statistics-first” (not ML-first)

An “AI model” trained on backtest outcomes is very easy to overfit, because it becomes yet another selection layer.
Most of the benefit can be captured with:
- holdout discipline,
- robust summary statistics (quantiles, pass-rates),
- multiple-testing / data-snooping controls (as feasible),
- clustering into stable regions rather than picking a single fragile point.

If ML is added later, it should be used as a *regularized shrinkage/forecast* component and evaluated with strict
grouped holdouts (e.g., by symbol), not as a “chooser” that sees everything.

---

## Methodology guardrails (avoid fooling ourselves)

### 1) Reserve a final holdout (“never touched”)

Meta-selection is itself selection. Without a final holdout set, confidence can be illusory due to data snooping.

Recommended:
- **Holdout symbols** (e.g., 10–20% of the universe) never used in optimization/validation/meta-selection until the end.
- Optionally also hold out **a time slice** (later period) for each symbol.

This is conceptually aligned with classic data-snooping warnings and bootstrap-based procedures in the literature.

### 2) Reduce “multiple testing” inflation

When scanning large parameter spaces (and then scanning many symbols), we effectively run many implicit tests.
We should treat “best-of-many” results as inflated and prefer conservative, distributional criteria.

Tools to consider (references in Appendix):
- White’s Reality Check (RC) style thinking about data snooping
- Hansen’s SPA test (a more powerful RC-style test)
- Deflated Sharpe Ratio / multiple-testing corrections
- Probability of Backtest Overfitting (PBO) / CSCV

We likely won’t implement all of these at once, but the design should not preclude them.

---

## Research-backed methods (what to add, and what it costs)

This section summarizes practical “anti-overfitting” techniques and what **additional data** we would need in order
to implement them inside the app.

### A) Probability of Backtest Overfitting (PBO) via CSCV

Bailey et al. propose **Combinatorially Symmetric Cross-Validation (CSCV)** to estimate the **Probability of Backtest
Overfitting (PBO)** when selecting the “best” strategy/parameter set from a large set.

How we can use it:
- **Per-symbol/per-run**: compute PBO on the optimization candidate pool to quantify how likely “best in-sample” is
  just selection luck.
- **Meta layer**: use PBO as a *penalty* (or hard filter) so artifacts generated under high overfitting risk contribute
  less to the global parameter recommendation.

Implementation notes (cost discipline):
- PBO/CSCV needs metrics computed on many train/test partitions. We can keep overhead low by running CSCV on only:
  - the **top-N** candidates after optimization, or
  - the **top-N** candidates after validation (preferred).
- We can support two modes:
  1) **Re-run backtests** on folds on-demand (slow, but no new artifact schema).
  2) Store **fold metrics** for top candidates during validation (fast later; slightly larger artifacts).

Data required (minimum):
- a `T × N` matrix of **per-period returns** (T samples, N candidates) and an evaluation function (e.g., Sharpe/Omega)
  to score each candidate on each fold.

### B) Deflated Sharpe Ratio (DSR) (selection-bias corrected Sharpe)

Bailey & López de Prado propose the **Deflated Sharpe Ratio** to correct Sharpe for selection effects and non-normality
when we are picking “the best” among many trials.

How we can use it:
- Treat DSR (or the corresponding confidence / p-value threshold) as a **gate** in validation/meta-selection when
  Sharpe-like objectives dominate ranking.
- Prefer DSR over raw Sharpe for any “best-of-many” UI table.

Data required (can be computed cheaply if we persist enough summary stats):
- sample size (bars or trades, depending on definition),
- Sharpe estimate,
- skewness and kurtosis of returns (or a return series to compute them),
- an estimate of the **effective number of trials** (not necessarily the raw grid size).

### C) Data-snooping tests: White’s Reality Check / Hansen’s SPA

White’s Reality Check and Hansen’s SPA test are classic approaches to evaluate whether the “best of many” genuinely
beats a benchmark after accounting for data snooping.

How we can use it:
- As an **optional final certification** step on the meta-shortlist (top-K groups) before calling them “recommended”.

Data required (this is the key blocker):
- a per-period **loss/return differential series** for each candidate vs a benchmark (e.g., buy&hold, or a simple
  reference strategy), because the procedures rely on resampling/bootstrapping time-series performance differences.

### D) Multiple comparisons control: Romano–Wolf (stepdown)

Romano–Wolf stepdown procedures control family-wise error rate under dependence and are often paired with resampling.

How we can use it:
- Once we have per-period loss series, we can test many candidates at once while controlling false positives more
  rigorously than naive p-values.

Cost:
- Same as Reality Check / SPA: requires storing or re-running per-period loss/return series.

### E) Model Confidence Set (MCS): output a “statistically defensible set”

Hansen, Lunde, and Nason’s Model Confidence Set produces a subset of models that are not significantly inferior to the
best model under a chosen loss function.

How we can use it:
- Instead of outputting a single “winner”, output a small **set** of candidates/regions with statistical justification.
  This aligns with the product goal: “complicated app that appears simple” (one click → a short set to choose from).

Cost:
- Again requires per-period loss series for candidates (or re-run capability).

### F) Sharpe ratio estimation pitfalls (serial correlation)

Sharpe-like metrics can be materially biased by serial correlation and other time-series effects. At minimum, the app
should:
- avoid over-trusting small Sharpe differences,
- prefer robust percentiles/pass-rates over single-point Sharpe,
- and (optionally) add an “autocorrelation-aware” Sharpe adjustment in advanced mode.

---

## Pragmatic rollout (v1 → v3)

### v1 (fast, high ROI): robust aggregation + holdout

Uses only what we already compute:
- strict validation shortlist rule,
- robust distributional aggregation across symbols (median + p10 + pass-rate),
- optional clustering into stable parameter regions,
- final **holdout symbols/time** evaluation before recommendation.

### v2 (moderate): stability selection + DSR + lightweight CSCV

Add conservative “selection stability” signals without huge overhead:
- **symbol-fold stability**: split symbols into folds; the recommended param group must survive across folds.
- **DSR gate** for Sharpe-based ranking.
- **CSCV/PBO on top-N candidates** only (either re-run on folds or store fold metrics during validation).

Symbol-fold stability (sketch):
1) Partition symbols into `K` folds.
2) For each fold `k`:
   - meta-select on `symbols \ fold_k` → `TopM_k`
   - evaluate `TopM_k` on `fold_k` (no re-fitting) → holdout metrics
3) Final shortlist prioritizes:
   - high selection frequency across `k`,
   - strong lower-tail holdout metrics across `k` (p10/pass-rate),
   - and low dispersion (avoid “wins only in 1 fold”).

### v3 (heavy but principled): RC/SPA + Romano–Wolf + MCS

If we store per-period loss/return series for top candidates (or can re-run efficiently):
- certify the shortlist against a benchmark (RC/SPA),
- control multiple comparisons (Romano–Wolf),
- output an MCS set instead of a single winner.

---

## Inputs (what the meta layer consumes)

Primary input: saved run artifacts under app data `runs/` (already implemented).

From each run artifact we need:
- run identity: `ts`, `strategy_kind`, app/version
- dataset identity: symbol/pair, market, timeframe, date range, adjusted/raw (if relevant)
- optimization results (Pareto/top)
- validation results (candidates with robust/cross/WF/MC/perturb scores)
- stability outputs (optional; currently per-drilldown)

### Gap: dataset metadata is not guaranteed

For Polygon datasets we can infer most fields.
For user-provided CSVs we should store minimally:
- user label (symbol) + timeframe (dropdown or inferred if known)
- date range (min/max timestamps at load time)

Plan: extend the run artifact schema to include a `dataset_meta` object, and persist it at run-save time.

---

## Outputs (what the user gets)

### 1) A small recommended shortlist

Goal: shrink ~1,000 validated candidates down to:
- **Top-K parameter sets** (e.g., 5–25), and/or
- **Top-M parameter regions** (clusters), with a representative “center” point.

### 2) Evidence report (XLSX)

Add a new XLSX report type (or new sheets in the existing report) with:
- “Meta Dashboard” (final picks + confidence summary)
- “Meta Candidates” (ranked table with robust statistics across runs)
- “Per-Symbol Heat” (optional: sparse table of candidate × symbol metrics)
- “Stability/Region” summary (if clustering enabled)

---

## Proposed scoring: robust, distributional, interpretable

Meta-selection should prefer:
- high **median** performance (not just best),
- strong **lower-tail** performance (e.g., p10 / worst),
- high **pass rate** (fraction of symbols where metrics exceed thresholds),
- low **dispersion** (IQR/std) and low fragility (peak vs plateau).

### Candidate identity

Two options:
1) **Exact param set key** (strict equality of all optimized knobs).
2) **Clustered region key** (bucket continuous params; group near-equal MA lengths/types).

Recommendation: support both:
- exact keys for “repeatability”
- optional clustering to find stable regions

### Suggested per-symbol score (already mostly available)

For each run’s validated candidate:
- `robust_score` (already)
- `cross_ticker.score` (if enabled)
- walk-forward worst/median (if enabled)
- MC tails: `pnl_p05`, `max_dd_p95`, `prob_pnl_positive`
- perturb worst-case (if enabled)
- local stability `peakΔ` (if available; can be computed cheaply for top picks)

### Suggested meta score (v1)

For each candidate key/cluster across symbols:
- `S_med = median(robust_score)`
- `S_p10 = p10(robust_score)`
- `DD_p90 = p90(max_dd_worst)` (or p90 of in-sample max_dd if WF not enabled)
- `PR = pass_rate(robust_pareto && (cross_pareto if available))`
- `MC = median(prob_pnl_positive)` (if MC enabled)
- `Peak = median(peak_delta)` (if stability enabled)

MetaScore example (tunable weights):

`MetaScore = S_p10 + 0.25*S_med + 0.25*MC + 0.10*PR - 0.75*DD_p90 - 0.25*PeakPenalty`

Where `PeakPenalty` is positive when the base point is meaningfully above the neighborhood median.

Notes:
- prefer `p10`/`worst` as the main term (robustness), not `max`.
- keep this formula **transparent** and user-editable later.

---

## Meta pipeline (hands-off)

### Phase A: Build the pool
- collect run artifacts matching:
  - same strategy kind (e.g., CompositeMo)
  - same timeframe (user-selected)
  - same validation configuration (optional strictness)
  - optional tags (e.g., “forex majors”, “crypto top50”)

### Phase B: Normalize + group
- parse candidate params into canonical representation
- optionally bin/cluster into regions

### Phase C: Aggregate + rank
- compute meta statistics per group
- apply hard filters:
  - minimum sample size (#symbols)
  - minimum pass rate
  - minimum median trades/exposure, etc.
- rank by MetaScore

### Phase D: Final holdout evaluation (recommended)
- evaluate the shortlisted groups on holdout symbols/time ranges
- report holdout deltas and only then recommend “set-and-forget” picks

### Phase E: Export report + persist meta-run artifact
- save meta analysis result JSON and XLSX
- allow compare between meta runs

---

## UI integration plan

Goal: keep the UI simple.

Add a new “Meta” tab:
- selector for:
  - strategy kind
  - timeframe
  - tag filter (optional)
  - “use holdout” toggle + holdout selector
- “Run meta analysis” button
- progress bar + ETA (reuse current progress system)
- results:
  - Top-K table
  - optional charts: distribution of robust_score across symbols, pass-rate bars
  - “Export meta report XLSX”

Defaults:
- on: use strict validated shortlist rule (robust pareto && cross pareto when available)
- on: enforce minimum sample size (e.g., >= 30 symbols)
- on: prioritize lower tail (p10)

---

## Implementation plan (Rust-first; test-first)

### 1) Data model + parsing
- Add `dataset_meta` to run artifacts.
- Add a “meta request” struct:
  - run selection criteria
  - grouping mode (exact vs clustered)
  - scoring weights
  - holdout config

### 2) Core aggregation engine (app_core)
- `ta_app_core::meta_select` module:
  - read artifacts (JSON) via Tauri command (paths provided)
  - extract candidates + metrics
  - group + aggregate + rank
  - return `MetaSelectionResult`

### 3) Tauri commands
- `run_meta_selection` command:
  - takes request + list of artifact paths OR auto-scans `runs/`
  - emits progress phases:
    - scan → aggregate → (optional) holdout-eval → export

### 4) Report export (XLSX)
- reuse the existing `rust_xlsxwriter` patterns
- add “Meta Dashboard / Meta Candidates” sheets

### 5) Tests (critical)

Add unit tests with synthetic mini-artifacts:
- grouping:
  - exact-key and clustering produce expected counts
- aggregation:
  - quantiles monotonic, pass-rates correct
- scoring:
  - deterministic ordering under controlled inputs
- holdout:
  - holdout set excluded from selection and only used for final evaluation

---

## Practical limitations / expectations

- “Set-and-forget” is a spectrum. The best we can do is:
  - pick params that are less sensitive,
  - perform well across many instruments and/or regimes,
  - and degrade gracefully rather than catastrophically.
- The selection pipeline must make overfitting harder, not easier.
- Any improvement should be judged by holdout performance and by forward monitoring, not by in-sample score.

---

## Appendix: research references (starting points)

- David H. Bailey, Jonathan Borwein, Marcos López de Prado, Qiji Jim Zhu — “The Probability of Backtest Overfitting” (PBO) / CSCV. (SSRN 2326253)
- David H. Bailey, Marcos López de Prado — “The Deflated Sharpe Ratio: Correcting for Selection Bias, Backtest Overfitting and Non-Normality.” (SSRN 2460551)
- Halbert White — “A Reality Check for Data Snooping” (Econometrica, 2000).
- Peter R. Hansen — “A Test for Superior Predictive Ability” (Journal of Business & Economic Statistics, 2005).
- Romano & Wolf — “Stepwise multiple testing as formalized data snooping” (Econometrics Journal, 2005) and related stepdown resampling work.
- Peter R. Hansen, Asger Lunde, James M. Nason — “The Model Confidence Set” (Econometrica, 2011).
- Andrew W. Lo — “The Statistics of Sharpe Ratios” (Financial Analysts Journal, 2002).
- Yoav Benjamini, Yosef Hochberg — “Controlling the False Discovery Rate: A Practical and Powerful Approach to Multiple Testing” (1995).
