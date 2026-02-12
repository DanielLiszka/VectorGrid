# VectorTA Optimizer App Vision, Workflow, and Research Notes

Last updated: 2026-02-11

Execution checklist: `APP_OBJECTIVE_ADDITIONS_CHECKLIST.md`

This is a research + planning document to clarify the end-to-end premise of the app and propose high-ROI
direction changes (beyond UI/UX) to make it both:

1. A credible performance showcase for the `vector-ta` library (especially VRAM-resident CUDA paths), and
2. A useful quant workflow for finding robust parameterizations of a fixed strategy family.

This document intentionally focuses on correctness, robustness, and research methodology. No process can
guarantee profitability; what we can do is reduce selection bias and make it harder to fool ourselves.

---

## Executive summary (recommended direction)

Premise:
- Treat the app as a "fast search + strict validation + meta selection" pipeline that produces a small, robust
  shortlist of parameterizations for a curated strategy family.

Main improvements to pursue (highest ROI beyond UI/UX):
- Add execution realism as a first-class validation dimension (costs, slippage, intrabar assumptions).
- Make the pipeline explicitly two-engine:
  - vectorized / GPU for broad search
  - iterative simulation (optional) for realistic re-scoring of top-N candidates during validation/holdout
- Harden reproducibility:
  - dataset fingerprints + versioning + deterministic seeds
  - artifacts that can be re-run and compared exactly
- Strengthen data-snooping awareness:
  - keep holdout discipline non-negotiable
  - add PSR/DSR gates or, at minimum, report them prominently

If we do only one major thing: implement "iterative validation mode" and use it only for top-N/holdout so
the speed showcase remains intact.

---

## 1) Premise (What This App Is)

### What the app is meant to do

Given:
- An asset (or a universe of assets),
- A timeframe (e.g., 1h),
- A fixed strategy family (Double MA, Composite),

the app runs an end-to-end pipeline:

1. Optimize parameters quickly (grid or sampler).
2. Validate the best candidates with out-of-sample discipline and robustness checks.
3. Aggregate validated results across many assets (meta selection) to recommend parameter sets that
   are more likely to generalize (set-and-forget shortlist).
4. Optionally export reports (XLSX) and provide drilldowns (charts + trades).

### What the app is not

- It is not a general-purpose backtesting framework where users write arbitrary strategies at runtime.
- It is not a guarantee of profitability. A fixed strategy can still have no edge on a market/timeframe.

### North star

The app should demonstrate:
- Very fast optimization on real datasets (especially GPU kernel for Double MA).
- A workflow that defaults to anti-overfitting practices:
  - holdout discipline
  - walk-forward validation
  - perturbation / Monte Carlo resampling
  - cross-asset evaluation
  - multiple-testing / selection-bias awareness

---

## 2) Vocabulary / Artifacts

Terminology used throughout:

- Dataset: a CSV of OHLCV bars (Polygon-cached or local `data/` folder).
- Base timeframe vs optimization timeframe:
  - base timeframe is "as loaded" (often 1m)
  - optimization timeframe may be derived (resampled) for backtests (e.g., 1h)
- Run artifact: a persisted JSON record of optimization inputs/outputs and (optionally) validation outputs.
  Stored under `runs/` (app-local data dir).
- Validation: robustness checks applied to a candidate set from an optimization run.
- Meta selection: aggregates validated candidates across many run artifacts to recommend robust parameter groups.
- Candidate: a parameter set + metrics.

---

## 3) End-to-End Workflow (Detailed)

This section is written as if you run the app end-to-end "for real", not as a developer.

### Step 0: Data selection and normalization

Goal: ensure all comparisons are apples-to-apples.

1. Choose a dataset source:
   - Polygon cache (downloaded in-app)
   - Local CSVs under repo `data/` (batch-friendly)
2. Choose the optimization timeframe:
   - "As loaded" (e.g., 1m), OR
   - Derived timeframe (e.g., 1h) by resampling the base dataset
3. Decide on universe and holdout discipline:
   - For single-asset exploration: still reserve a final time holdout.
   - For multi-asset robustness: reserve a subset of symbols as holdout.

Practical note:
- A large portion of backtest "alpha" evaporates when you add realistic costs and remove leakage.
  The data stage must be strict: correct timestamps, stable resampling rules, consistent session handling.

### Step 1: Optimization (fast search)

Inputs:
- Strategy kind:
  - Double MA (CPU / CUDA MA sweep / CUDA kernel)
  - Composite CPU (single objective)
  - Composite MO (multi-objective)
- Parameter search space:
  - Double MA: fast/slow length ranges, MA types/params, plus execution knobs.
  - Composite: broader search space (MA + gating indicators + config).
- Objective(s):
  - Single objective for ranking (e.g., Sharpe/Sortino/Calmar/Omega).
  - For MO: maximize one metric and minimize another (current design).

Outputs:
- A best candidate + top-K candidates (and Pareto front for MO).
- A run artifact saved to disk for reproducibility and later aggregation.

What optimization should do (methodology)
- Be "fast and wide" and produce a candidate set, not a final answer.
- Avoid overfitting via basic guardrails:
  - minimum trades constraint
  - maximum drawdown constraint
  - exposure / turnover constraints (add if missing)
- Persist enough metadata to reproduce and to support validation/meta.

### Step 2: Validation (robustness gates)

Optimization is selection. Validation exists to punish selection bias and fragile candidates.

Validation consumes:
- the strategy + selected candidates
- the dataset (and optionally other datasets)

Validation methods (recommended set)
- Walk-forward:
  - score candidates across sequential windows and use out-of-sample performance as the main signal
- Perturbation testing:
  - randomly perturb candles (or features) and ensure the strategy degrades gracefully
- Trade bootstrap / Monte Carlo:
  - resample trades to estimate distribution of outcomes (uncertainty)
- Cross-ticker / cross-dataset evaluation:
  - run a candidate across multiple assets of the same timeframe to see if it generalizes at all
- Data-snooping controls:
  - CSCV / Probability of Backtest Overfitting (PBO) as a penalty or gate
  - selection-bias corrected Sharpe measures (PSR/DSR) where Sharpe-like objectives dominate

Outputs:
- A validated shortlist with robust metrics (quantiles, pass rate, worst-case DD, etc.)
- An updated run artifact including validation results
- Optional XLSX report export (ideally only for top-K / final candidates)

Validation should be:
- Strict by default.
- Fast enough to run routinely (but can scale up for final selection).
- Deterministic / reproducible with a seed.

### Step 3: Meta selection (aggregate many runs into a shortlist)

Why meta selection exists:
- Even validation on a single dataset is not enough.
- If the goal is "robust parameters", we must evaluate across many datasets and focus on distributions.

Meta selection inputs:
- A set of saved run artifacts (usually generated by batch runs).
- Filters: strategy kind, timeframe, tags.
- Grouping configuration:
  - exact grouping (same params) or clustered grouping (bucket similar params)
- Scoring weights and minimum sample thresholds (e.g., min 30 assets).

Meta selection outputs:
- A ranked list of parameter groups with:
  - robust score distributions across assets (p10/median)
  - max drawdown quantiles (p90)
  - pass rate across assets
  - cross-ticker consistency
- A meta artifact (saved JSON) and optional meta XLSX report.

Holdout inside meta selection
- Meta selection is another selection layer; it needs a final holdout evaluation.
- Recommended:
  - holdout symbols or holdout time slices are excluded from ranking and only used at the end
  - heavy mode can re-run stronger validation on holdout (walk-forward + perturb/MC)

### Step 4: Post-meta analysis (final decision + operationalization)

At this point, the output should be a small shortlist, not thousands of candidates.

Post-meta analysis should include:
- Re-run the top-N recommendations on a curated "representative set" of assets.
- Compare:
  - in-sample vs out-of-sample
  - train universe vs holdout universe
  - stability around the chosen parameter point (sensitivity)
- Export high-quality reports only now:
  - XLSX reports for top-N only (costly, but worth it here)
  - an audit trail of configs, seeds, and dataset fingerprints
- Optionally produce a "deployment profile":
  - expected turnover, capacity assumptions, risk limits, and failure modes

---

## 4) Current State of the App (As Observed)

What is already strong:
- Double MA optimization is a genuine speed showcase:
  - CPU and CUDA paths
  - VRAM-resident CUDA kernel mode exists (fast)
- Composite strategy exists and is optimized via MO samplers (NSGA-II / TPE / QMC / random).
- Validation exists and can compute robust metrics + cross-ticker + CSCV/PBO-like statistics.
- Meta selection exists:
  - aggregates across runs
  - grouping and scoring
  - optional holdout evaluation (light/heavy)
  - optional ML confidence model integration (as a ranker/shrinkage component)
- Batch runs exist:
  - can run optimize -> validate across many datasets
  - can automatically run meta selection afterward

---

## 5) What Can Be Improved (Outside Observer)

This section focuses on improvements that directly impact research integrity or usefulness (not only UI).

### A) Make the pipeline explicitly "two-engine"

Right now the mental model is:
- optimization is fast search
- validation is robustness gates
- meta is selection across many assets

We should explicitly design around a two-engine approach:

1. Fast vectorized engine for broad search (GPU kernel / CPU grid / sampler).
2. Slower iterative engine for realism and path-dependent execution details, used only on shortlisted candidates.

This lets us keep the VectorTA speed showcase while still improving realism where it matters.

### B) Make execution realism a first-class validation dimension

If the strategy can only be profitable under unrealistic fills, the pipeline should kill it.

Add or harden:
- slippage + spread models (per market preset)
- realistic fee schedules
- funding/borrow costs if futures/shorting are modeled
- limit order fill assumptions (if we support them)
- intrabar ambiguity handling (OHLC ordering assumptions) OR resimulate using higher resolution data

### C) Stronger multiple-testing / selection bias handling

When scanning large parameter spaces, raw "best Sharpe" is a classic trap.

We already have robust scoring and CSCV/PBO hooks. Consider:
- Probabilistic Sharpe Ratio (PSR): confidence that Sharpe exceeds a threshold.
- Deflated Sharpe Ratio (DSR): correct Sharpe for selection bias (best-of-many) and non-normality.
- White's Reality Check / Hansen's SPA thinking:
  - meta selection should be treated as data mining unless holdout proves otherwise.

### D) Improve dataset identity and reproducibility

For large-scale batch workflows:
- store dataset fingerprints (hash, row count, min/max ts, timeframe)
- store the exact resampling configuration
- store the strategy + library version
- ensure deterministic seeds for all stochastic components (QMC, TPE, MC bootstrap)

The goal is: "given an artifact, it is trivially reproducible and comparable."

### E) Define success criteria explicitly

Without an explicit definition of "robust", the app will always drift into "best in-sample score wins".

A practical definition could be:
- Meta shortlist candidates must meet:
  - robust_score p10 > 0
  - pass rate > 0.6 across assets
  - max_dd p90 < threshold
  - min trades > threshold
  - stable neighborhood (local stability)
- And then holdout must confirm:
  - median robust_score remains positive
  - drawdown does not explode

---

## 6) Trade-Dependent Events vs Grid Optimization (Do We Need Iterative Simulation?)

Short answer: yes, it can be valuable, but only if used in a targeted way.

### Why vectorized/grid backtests struggle with trade-dependent events

Vectorized engines are excellent when:
- entry/exit is driven by signals that can be expressed as arrays
- position state is simple and rules are not heavily path dependent

They struggle (or become approximate) when you introduce:
- stop loss / take profit / trailing stops with intrabar ambiguity
- partial fills and limit orders
- dynamic position sizing based on evolving equity or volatility
- pyramiding, scaling in/out
- complex order precedence rules (what triggers first within an OHLC bar)

Even if a strategy is "static" (compiled and fixed), it can still have path-dependent execution logic.

### When an iterative engine is worth it

It is worth adding an iterative/event-driven simulator if we want at least one of:
- realistic cost/slippage models that depend on volatility/liquidity
- order models beyond "enter/exit at close/next open"
- intrabar stop/take-profit behavior
- trade-level features (MAE/MFE, heatmaps, hold duration stats) computed from simulated fills

### Recommended compromise: iterative mode only for validation and holdout

To preserve the speed showcase:
- keep the current vectorized + GPU kernel search as the default optimizer path
- add an optional iterative simulation mode in validation:
  - re-simulate only top-N candidates
  - on holdout folds / representative datasets
  - with strict execution assumptions

This gives the best of both worlds:
- huge speed on exploration
- realism when it matters for "is this actually tradable?"

### If we do NOT add iterative simulation

Then we should still strengthen realism by:
- adding conservative execution assumptions to the vectorized model (penalize by costs)
- forcing the user to validate on higher-resolution data (e.g., optimize on 1h, validate using 1m bars)
- making intrabar ambiguity explicit and pessimistic

---

## 7) Suggested Roadmap (High ROI)

### Phase 1: Methodology hardening (no new strategy complexity)

1. Execution presets and costs:
   - slippage/spread presets and exposure constraints
2. Artifact reproducibility:
   - dataset fingerprints + versioning in artifacts
3. "Full pipeline" automation:
   - one-click: batch -> validation -> meta -> export shortlist
4. Statistical reporting:
   - PSR/DSR as optional columns and gates

### Phase 2: Iterative simulation (validation-only)

1. Implement an iterative simulator for a restricted order model:
   - market orders (close/next open) + slippage
   - optional stop/take-profit with intrabar rules
2. Use it in validation:
   - re-score top-N candidates with iterative simulation
   - compare vectorized vs iterative deltas (sanity check)

### Phase 3: Strategy family expansion (curated, not arbitrary user code)

If we expand strategies, keep them curated and compiled:
- a small set of strategy families that share a common execution model
- optional indicator toggles via parameters (compile the superset, optimize selection)
- GPU kernel expansion only where it meaningfully improves the demo

---

## 8) Direction options (and what I would pick)

Option A: Showcase-first (keep it small)
- Focus on Double MA GPU-kernel as the headline demo.
- Keep Composite as the "research-grade CPU-only" example.
- Invest mostly in validation/meta methodology and reproducibility.
- Avoid becoming a general-purpose backtester.

Option B: "Legit quant tool" (curated, but realism-focused)
- Add iterative validation mode and a conservative execution model.
- Add costs/slippage presets and make them part of scoring/selection.
- Add portfolio-ish evaluation (even if only equal-weight across assets) as a stress test.
- Keep strategy families curated and compiled (no runtime strategy scripting).

Option C: Platform (plugins / user strategies)
- Add a plugin system (dynamic library / WASM / scripting).
- This is a large scope increase and risks undermining the core message ("fast and correct by default"),
  unless it is extremely disciplined.

Recommendation:
- Pursue Option B, but only for validation/holdout paths. Keep optimization fast and constrained so the
  app remains a VectorTA performance showcase.

---

## 9) If we stop feature development, what can be objectively improved?

Even without adding new strategy families or an iterative engine, we can materially improve research quality:

- Cost realism:
  - add explicit slippage/spread/fee presets and bake them into every run artifact
- Reproducibility and auditability:
  - dataset fingerprints and versioned artifacts
  - deterministic seeds everywhere
  - "rerun exactly" buttons that truly reproduce
- Default workflow guardrails:
  - require/encourage holdout use in batch/meta
  - refuse to show "best-of-many" as a final answer without validation/holdout context
- More conservative reporting:
  - emphasize lower-tail quantiles (p10) and pass-rates over maxima
  - report selection bias warnings when scanning huge spaces

These changes improve correctness and trust even if the surface UI stays similar.

---

## 10) References (Web Research Starting Points)

Backtest overfitting and selection bias:
- Bailey et al. "The Probability of Backtest Overfitting" (PBO) / CSCV (SSRN)
  - https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253
- Bailey & Lopez de Prado: "The Deflated Sharpe Ratio" (SSRN + Journal of Portfolio Management)
  - https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2460551
  - https://www.pm-research.com/content/iijpormgmt/42/4/94
- Probabilistic Sharpe Ratio (PSR) definition (industry reference)
  - https://www.risk.net/definition/probabilistic-sharpe-ratio
- White (2000): "A Reality Check for Data Snooping" (Econometrica)
  - https://www.econometricsociety.org/publications/econometrica/2000/03/01/reality-check-data-snooping

Vectorized vs event-driven backtesting tradeoffs:
- Interactive Brokers: discussion of vectorized vs event-driven backtesting approaches
  - https://www.interactivebrokers.com/campus/ibkr-quant-news/vectorized-vs-event-driven-backtesting-speed-vs-flexibility/

Time-series cross-validation (leakage control):
- "Purged k-fold cross-validation" and embargo concepts (Lopez de Prado / AFML)
  - https://en.wikipedia.org/wiki/Purged_k-fold_cross-validation

Note: these references are intended to inform design choices and defaults, not to turn the app into
a statistics paper. The goal is to encode best practices into the workflow so the default path is
"hard to fool yourself".
