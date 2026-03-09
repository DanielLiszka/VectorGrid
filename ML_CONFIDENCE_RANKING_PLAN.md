# Optional ML “Confidence Ranking” for Post‑Validation (Design + Research Plan)

_Draft created: 2026-01-29_

## Problem statement

We currently have a strong **post‑validation / meta** pipeline (optimize → validate → meta aggregate → optional holdout eval → meta report). The remaining pain point is that the **most expensive** robustness checks (especially holdout evaluation across many tickers and/or “heavy” validation) can’t be run for every possible candidate region when search spaces get large.

This document proposes an **optional** ML feature that learns from **accumulated expensive outcomes** and produces a **confidence ranking** for candidates during post‑validation/meta analysis.

The confidence ranking is **advisory-only**: it never replaces validation/holdout; it helps us **prioritize** what to evaluate next and how to rank candidates when we have partial expensive coverage.

---

## Goals / non‑goals

### Goals

1) **Optional and safe**: default OFF; clearly marked as experimental; no impact on existing workflows unless enabled.
2) **Confidence as calibrated probability**: output a probability like “P(this candidate meets a holdout robustness criterion)” rather than a magic score.
3) **Uses existing artifacts**: train from stored run/meta artifacts under app data `runs/` (no external pipelines required).
4) **No CUDA/toolchain requirement**: CPU‑only, deterministic, fast inference.
5) **Anti‑leakage**: do not mix timeframes/strategies; avoid training on information that would not be available at decision time.

### Non‑goals (v1)

- Predicting future profitability; the target is **robustness under our own evaluation protocol**, not market forecasting.
- Replacing “statistics-first” selection gates (PBO/CSCV/DSR, pass-rates, quantiles, etc.).
- GPU-accelerated ML training.

---

## Where this fits in the app workflow

Current conceptual workflow:

1) **Optimization** (very fast matrix/VRAM kernel where applicable)
2) **Validation** (WF / perturb / bootstrap / cross‑ticker, etc.)
3) **Post‑validation / Meta selection** (aggregate across many artifacts)
4) Optional: **Meta holdout evaluation** (expensive) on “never touched” holdout artifacts
5) Export / compare / iterate

Proposed ML addition:

- After (3) and/or (4), optionally compute **ML confidence** per candidate group:
  - If (4) exists historically: use it as the label to train the model.
  - If (4) is not run for the current meta selection: use ML confidence to rank which groups should get expensive evaluation next.

In UI terms: this should live in the **Meta / post‑validation** area as an optional panel + an extra sortable table column.

---

## What counts as “expensive results” (labels)

For v1, the cleanest and least leaky label source is **Meta holdout evaluation** results (currently supported for `CompositeCpu`/`CompositeMo` in `src-tauri/src/commands.rs`).

Why it’s a good label:
- It is explicitly out‑of‑sample by construction (holdout tag partition).
- It directly matches the product question: “will this candidate region generalize?”

### Candidate labels (v1)

We need a crisp target. Start with a **binary classification** label because it is stable and easy to calibrate.

Recommended default label (adjustable later):
- `y = 1` if a meta candidate group meets all of:
  - `holdout.pass_rate ≥ P_min` (e.g. `0.60`)
  - `holdout.score_p10 ≥ S_min` (e.g. `0.00`)
  - optionally: `holdout.max_dd_p90 ≤ DD_max` (risk gate)
- `y = 0` otherwise

Rationale:
- Pass rate + lower-tail score are harder to “game” than median alone.
- Binary labels avoid over-interpreting small metric differences.

Optional future targets:
- Regression target: predict `holdout.score_p10` (more information, but riskier).
- Multi-objective target: predict probability of being Pareto‑good on holdout.

---

## Model inputs (features)

The model must only use features that are available **before** expensive evaluation.

### Primary feature source (recommended): `MetaCandidateAggregate` (train-only stats)

For each meta candidate group, use features like:
- `samples`
- `pass_rate`
- `robust_score_p10`, `robust_score_median`
- `max_dd_p90`
- `cross_score_median`
- `meta_score` (as a convenience feature; or recompute from weights)

Strict rule: **never include** `holdout.*` fields as features.

### Additional/derived features (cheap + helpful)

Derived transforms that usually help simple linear models:
- log transforms: `log1p(samples)`, `log1p(abs(meta_score))`
- ranks/percentiles within the meta result (e.g. rank by `robust_score_p10`)
- dispersion proxies (if we have underlying distributions): IQR, stddev, worst‑case

Optional artifact-level signals (only if available in artifacts without leakage):
- CSCV/PBO metrics (if stored in validation artifacts)
- Strategy config flags that materially change behavior

---

## Leakage prevention and split discipline

This is critical: ML becomes yet another selection layer, and can silently overfit if we don’t enforce discipline.

### Hard scoping rules

Train separate models by:
- `strategy_kind` (e.g., `CompositeMo`)
- `timeframe` (e.g., `1m`, `15m`, `2h`)
- (optional) `ma_source` if it changes indicator behavior materially

Do not mix:
- multiple timeframes
- different datasets/time ranges with incompatible semantics

### Train/test splitting

We want the model to generalize to **new tickers and new time periods**, not memorize repeated artifacts.

Recommended v1 split rule:
- Group by **meta selection run id** (or meta artifact filename) so that groups from the same meta run don’t land in both train and test.
- If we later store per‑symbol holdout results, also group by `symbol`.

If we ever use time-sliced labels (same symbols, different time periods), consider financial‑ML leakage protections:
- Purging + embargo around label horizons (“Purged K‑Fold” / “CPCV”). See references below.

---

## Model choice (min regression risk)

### Recommended v1 model: Regularized Logistic Regression

Why:
- Fast and deterministic.
- Interpretable (weights can be shown as “feature contributions”).
- Outputs a probability naturally (then we calibrate it).
- Works well with small to medium datasets.

Alternatives (v2+):
- Gradient boosted trees / random forest (better non‑linear fit, higher overfit risk).
- Conformal prediction on top of a classifier to produce coverage‑guaranteed sets (great for “uncertainty”, more plumbing).

### Rust implementation options

We should strongly prefer minimal-dependency, pure Rust solutions.

Options:
1) **Implement logistic regression ourselves** (IRLS / L‑BFGS + L2):
   - Pros: smallest dependency surface; full control/determinism.
   - Cons: more engineering + testing effort.
2) Use `smartcore`:
   - Pros: pure Rust ML toolkit; includes many models.
   - Cons: ensure model stability + serialization format.
   - Docs: https://docs.rs/smartcore/latest/smartcore/
3) Use `linfa` / `linfa-logistic`:
   - Pros: ML ecosystem with `ndarray`; logistic regression available.
   - Cons: dependency weight; ensure no BLAS requirement sneaks in.
   - Docs: https://docs.rs/linfa-logistic/latest/linfa_logistic/

For v1 “objective improvement without regressions”, #1 or #2 is likely best.

---

## Calibration and quality metrics (make “confidence” mean something)

Even simple models can be miscalibrated. We should measure and (optionally) calibrate.

### Metrics to report (in-app)

Discrimination:
- ROC‑AUC (is the ranking useful?)

Calibration:
- Brier score (proper scoring rule; lower is better)
- Reliability diagram (visual)
- Optional: Expected Calibration Error (ECE) (with caveats)

Calibration methods:
- Platt scaling / temperature scaling (simple, robust)
- Isotonic regression (flexible; overfits when data is small)

Useful references:
- scikit-learn calibration overview (Platt / isotonic): https://scikit-learn.org/stable/modules/calibration.html
- Brier score definition: https://en.wikipedia.org/wiki/Brier_score
- Reliability diagrams survey/perspective: https://arxiv.org/abs/2305.16930
- SmoothECE (better-behaved ECE / reliability diagrams): https://arxiv.org/abs/2309.12236
- “On Calibration of Modern Neural Networks” (temperature scaling): https://arxiv.org/abs/1706.04599

---

## Output semantics (“confidence ranking”)

In UI and reports, define confidence clearly:

> **ML Confidence** = calibrated estimate of `P(y=1)`  
> where `y=1` means “this candidate meets the chosen holdout robustness criterion”.

This is not:
- a probability of profit
- a guarantee
- comparable across different timeframes/strategies unless we explicitly normalize and validate that

Optional future enhancement (v2+): a “coverage‑guaranteed” uncertainty wrapper using conformal prediction:
- Intro: https://arxiv.org/abs/2107.07511

---

## UX proposal (Meta / post‑validation tab)

Add a new “ML Confidence (experimental)” panel:

- Toggle: **Enable ML confidence ranking**
- Button: **Train / Update model** (from stored artifacts)
- Status box:
  - model id (`strategy_kind`, `timeframe`)
  - training sample count + positive rate
  - cross‑val AUC, Brier
  - last trained timestamp
  - warnings (e.g., “insufficient labels”, “class imbalance”, “AUC near random”)

Table changes:
- Add column: `ML confidence`
- Sorting option: sort by confidence (desc)
- If the model is not available or insufficient data:
  - hide column or show `—` and a tooltip “Train model first”

Report changes:
- Include confidence column in **meta XLSX report** when enabled.

---

## Persistence / versioning plan

Store trained models under app local data dir, e.g.:
- `models/confidence/{strategy_kind}/{timeframe}/model.json`

Model snapshot contents (JSON, versioned):
- `schema_version`
- `model_id`
- `label_definition` (thresholds + fields used)
- `feature_list` + transforms
- normalization stats (mean/std per feature)
- model weights + bias (and calibration params if used)
- training metrics (CV AUC, Brier, calibration curve bins)
- dataset fingerprint:
  - app version
  - list/hash of input artifact paths + modified times (to detect staleness)

Determinism:
- fixed RNG seed where randomness exists
- stable ordering when building datasets (sort by path)

---

## Implementation plan (phased)

### Phase 1 — Data extraction + “no-model” plumbing (low risk)

1) Add a core module (e.g. `crates/app_core/src/ml_confidence.rs`) that:
   - scans `vectorbt_meta_selection` artifacts
   - builds a dataset of `(x, y)` rows for the given `(strategy_kind, timeframe)`
   - validates minimum sample sizes and label balance
2) Add a Tauri command to return:
   - “how many labeled rows exist for this scope”
   - example feature summary (min/max)

### Phase 2 — Train + infer (MVP)

1) Implement logistic regression (or integrate `smartcore`) + inference.
2) Persist model snapshot + metrics.
3) Add a Tauri command:
   - `train_ml_confidence_model(scope, label_config) -> metrics`
4) Add a Tauri command:
   - `apply_ml_confidence(scope, meta_result) -> confidences[]` (or returns updated meta result with `ml_confidence`)

### Phase 3 — UI integration + report export

1) Add the “ML Confidence (experimental)” panel in Meta tab.
2) Add table column + sorting.
3) Add XLSX meta report column if enabled.

### Phase 4 — Calibration + diagnostics (upgrade)

1) Add optional temperature scaling / isotonic calibration.
2) Show reliability diagram (simple binned plot or table).
3) Add “model health gate” (if AUC < threshold, auto-disable with warning).

### Phase 5 — Expand label sources (future)

- Add holdout evaluation support for more strategies (e.g., Double‑MA) so ML ranking can generalize beyond Composite.
- Add richer labels (per‑symbol holdout rows) to enable better group splits and reduce variance.

---

## Acceptance criteria (objective)

1) Feature is OFF by default; turning it on doesn’t change existing outputs unless it adds the optional column.
2) CPU-only builds remain clean (no CUDA requirement, no native BLAS requirement).
3) Model training is deterministic and fast (seconds, not minutes) on typical datasets.
4) Confidence is scoped correctly: never mixes timeframes/strategies.
5) Model quality is measurable:
   - Reports AUC and Brier at minimum.
   - If quality is poor, UI warns and recommends not using the ranking.

---

## References (research links)

Calibration / reliability:
- https://scikit-learn.org/stable/modules/calibration.html
- https://arxiv.org/abs/1706.04599
- https://arxiv.org/abs/2305.16930
- https://arxiv.org/abs/2309.12236
- https://en.wikipedia.org/wiki/Brier_score

Distribution-free uncertainty (optional v2+):
- https://arxiv.org/abs/2107.07511

Finance leakage / CV discipline (background):
- Purging + embargo (book chapter index): https://www.oreilly.com/library/view/advances-in-financial/9781119482086/c07.xhtml
- CPCV / purged CV overview: https://random-docs.readthedocs.io/en/latest/implementations/cross_validation.html
- Backtest overfitting discussion (Lopez de Prado, Significance 2021): https://academic.oup.com/jrssig/article/18/6/22/7038278

Rust ML crates (implementation options):
- `smartcore`: https://docs.rs/smartcore/latest/smartcore/
- `linfa-logistic`: https://docs.rs/linfa-logistic/latest/linfa_logistic/
