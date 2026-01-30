# ML Confidence Ranking — High‑ROI Improvements (v2 Roadmap + Research)

_Draft created: 2026-01-29_

## Context (what we have today)

We currently ship an **optional** ML “confidence ranking” for Meta (post‑validation):

- **Model:** CPU‑only L2‑regularized logistic regression (deterministic).
- **Training data:** saved `vectorbt_meta_selection` artifacts that include per‑group `holdout` aggregates.
- **Scope key:** `(strategy_kind, timeframe)`.
- **Features (train‑only):** `samples`, `pass_rate`, `robust_score_p10`, `robust_score_median`, `max_dd_p90`, `cross_score_median`, `meta_score`.
- **Label:** binary gate from holdout aggregates (min pass rate + min p10, optional dd cap, min holdout datasets).
- **UX:** Meta table has `ML` column + optional “Autopilot” (train/apply automatically when possible).

This is intentionally conservative: the ML layer is advisory-only, never required for the workflow.

## Goal (what “better” means)

Make ML confidence more **trustworthy, automatic, and cost‑saving**, without regressions:

1) **More evidence** per unit of real work (bigger labeled corpus, less noise).
2) **Better semantics** for “confidence” (calibration + health gating).
3) **Less manual effort** (autopilot stays current).
4) **Lower compute cost** (use ML to decide what to evaluate next).
5) **No CUDA/toolchain requirement** and minimal dependency surface.

## High‑ROI improvements (ranked)

### 1) Persist per‑holdout‑dataset labels (largest ROI)

**Problem:** v1 training uses ~`top_k` rows per meta run (one row per group), which is often too few for stable ranking and calibration.

**Upgrade:** When running “Meta holdout evaluation”, also persist **per‑holdout‑dataset** outcomes:

`(group_key, holdout_dataset_id) -> metrics + pass/fail`

This converts one meta run into **10–100× more labeled rows** (depending on holdout dataset count), without changing the backtesting core.

**Implementation notes**
- Keep it **opt‑in** (to manage file size): “Save per‑dataset holdout rows (for ML)”.
- Store only for **top_k** (already bounded), and only for the holdout set used in that run.
- Add schema/versioning:
  - `ml_labels_version` (so future label tweaks don’t silently mix with old data).
  - `dataset_fingerprint` (timeframe + candle source + candle range hash).

**Payoff**
- Stabilizes ML quickly even with only a few meta runs.
- Enables better split discipline (group by meta run, by dataset, etc.).
- Enables real uncertainty estimation and conformal methods later.

---

### 2) “Autopilot v2”: staleness detection + safe auto‑retrain (high UX ROI)

**Problem:** current autopilot only trains if the model is missing, so models silently go stale as new labeled artifacts appear.

**Upgrade:** persist model metadata and retrain automatically when “enough” new evidence exists.

**Model metadata to store**
- `trained_ts`, `trained_on_rows`
- `newest_meta_ts_used`, `meta_artifacts_used_count`
- `labels_version`, `scope` (already)
- snapshot metrics (AUC/Brier) + warnings

**Staleness policy (pragmatic defaults)**
- Retrain when either:
  - `new_rows >= 200` (or a percentage threshold), or
  - last training is “old” relative to newest meta, and
  - there are at least `min_rows` total.
- Cooldown to avoid retraining repeatedly (e.g., “at most once per X minutes”).

---

### 3) Probability calibration + “model health gate” (high trust ROI)

Even logistic regression can be miscalibrated (and calibration will degrade when labels are scarce).

**Upgrade**
- Fit a lightweight calibrator from **out‑of‑fold** predictions:
  - Platt scaling (sigmoid on logits) or temperature scaling.
- Compute/report:
  - ROC‑AUC (ranking usefulness)
  - Brier score (calibration + sharpness)
  - reliability bins (small)

**Health gate**
- If CV AUC ~ random (or Brier worse than baseline), the UI should:
  - show ML as “unhealthy / disabled” and avoid auto‑sorting by ML,
  - keep ML visible (optional) but clearly marked.

**Why this matters**
- This turns “confidence” into something closer to: “P(this group meets our holdout robustness criterion)”.
- Makes it safe to add automation later (“auto‑promote groups above 0.9 confidence”).

References:
- Temperature scaling: https://arxiv.org/abs/1706.04599
- Calibration overview: https://scikit-learn.org/stable/modules/calibration.html
- Platt scaling (reference hub): https://transferlab.ai/refs/platt_probabilistic_1999/

---

### 4) Evidence‑aware scheduling (active-learning style cost saver)

Once confidence is stable, the next step is making it **save time**:

**Idea:** Don’t run expensive holdout/heavy validation for every candidate. Use ML to pick what to evaluate next.

Practical scheduler (v1)
- Exploit: evaluate top `K_exploit` highest confidence.
- Explore: evaluate `K_explore` most uncertain (closest to 0.5).
- Diversity (optional): enforce parameter spread / clustering to avoid evaluating near‑duplicates only.

This is “active learning” in spirit: spend expensive evaluation where it is most informative.

Reference:
- Uncertainty sampling discussion: https://link.springer.com/article/10.1007/s10994-021-06003-9

---

### 5) Conformal “risk‑control” sets (optional, higher effort, strong upside)

If we want “automation with guardrails”, conformal prediction can produce **abstaining / safe‑set** behavior:

- “Show only candidates in the conformal ‘pass’ set at error ≤ ε.”
- “Abstain unless the confidence set is tight.”

This requires more labels and careful assumptions (exchangeability), but it can be a strong future step.

References:
- Tutorial (JMLR): https://jmlr.csail.mit.edu/papers/v9/shafer08a.html
- MAPIE docs (good overview + references): https://mapie.readthedocs.io/

---

## What not to chase (low ROI / high risk for this app right now)

- **RL trading agents**: large surface area (environment design, reward shaping, leakage, evaluation). High engineering cost and usually lower real-world reliability than “stats-first + robustness gates”. Consider only if the product pivots to *execution policy learning* rather than *strategy region robustness*.
- Heavy deep learning or LLMs inside the app: dependency/compute complexity with unclear measurable value for “will this pass our holdout gate?”

## Recommended rollout plan (minimal regressions)

### v2.1 (safe + mostly plumbing)
1) Autopilot v2 (staleness detection + retrain thresholds + model metadata)
2) Calibration + health gate (Platt/temperature from out‑of‑fold predictions)
3) UI: show “Model health” + last trained + rows + warnings

### v2.2 (biggest ROI)
1) Opt‑in per‑holdout‑dataset row persistence (schema + file size caps)
2) Train from per‑dataset rows (stronger splits: group by meta run and dataset)

### v2.3 (cost saver)
1) Add “Evaluate top‑N only” mode driven by ML confidence/uncertainty
2) Add optional diversity sampling

## Success metrics (objective)

1) **Calibration:** Brier improves vs baseline; reliability bins closer to diagonal.
2) **Ranking:** CV AUC meaningfully > 0.5 on real artifacts.
3) **Stability:** rankings don’t whipsaw across runs with similar evidence.
4) **Cost reduction:** “Evaluate top‑N only” reduces holdout runtime while keeping final selection quality (as measured by downstream post‑analysis).

## Bench / regression guardrails

- Keep a fast end‑to‑end benchmark including ML:
  - `crates/app_core/benches/e2e_workflow_ml.rs`
- Add a small “no‑ML regression” bench run and verify:
  - ML disabled is identical to baseline outputs.

