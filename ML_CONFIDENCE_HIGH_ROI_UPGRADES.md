# ML Confidence Ranking — High‑ROI Upgrades (Plan + Research)

_Draft created: 2026-01-29_

## Context (current v1 implementation)

We currently ship an **optional** ML “confidence ranking” for post‑validation (Meta):

- **Model:** L2‑regularized logistic regression (CPU‑only, deterministic).
- **Training data:** saved `vectorbt_meta_selection` artifacts that have per‑group `holdout` aggregates.
- **Scope:** model is keyed by `strategy_kind + timeframe`.
- **Features:** meta‑aggregate features only (no holdout fields): `samples`, `pass_rate`, `robust_score_p10/median`, `max_dd_p90`, `cross_score_median`, `meta_score`.
- **Label:** binary gate computed from holdout aggregates (pass‑rate + p10 threshold; optional dd cap; min holdout datasets).
- **UI:** Meta tab shows `ML` column; optional sorting; optional “Autopilot” trains/loads/applies automatically when possible.

Key code locations:
- Core ML: `crates/app_core/src/ml_confidence.rs`
- Tauri commands: `src-tauri/src/commands.rs` (train/apply/status/delete)
- UI integration: `dist/index.html` + `dist/app.js`

## Why improve it?

The v1 is intentionally conservative and low‑dependency, but it has predictable limitations:

1) **Small labeled dataset**: one meta run yields only `top_k` labeled rows; calibration and ranking stability suffer.
2) **Probability semantics**: “confidence” is not guaranteed calibrated (even if AUC is decent).
3) **Autopilot is only “train‑if‑missing”**: it does not automatically retrain when new labeled meta artifacts arrive.
4) **No evidence‑aware scheduling**: ML currently ranks results, but doesn’t reduce expensive evaluation cost.

The following upgrades focus on maximum ROI with minimal regression risk.

---

## High‑ROI upgrades (prioritized)

### 1) Persist richer labels (biggest ROI)

**Problem:** Today we train on *one row per meta group* using **holdout aggregates**. That’s low sample count and high noise.

**Upgrade:** Persist *per‑holdout‑dataset* outcomes for each meta group (or a compact approximation), so training rows become:

`(meta group, holdout dataset) → pass/fail or score`

This yields 10–100× more labeled rows from the same work the app already did.

**Implementation sketch (low risk):**
- Extend the meta artifact schema written by `run_meta_selection` to optionally include:
  - `res.holdout_rows[]`: rows like `{ group_key, dataset_label, score, sharpe, max_dd, pnl }`
  - or `res.groups[i].holdout_by_dataset[]` with the same per‑dataset metrics
- Store only for:
  - `top_k` groups (already small), and
  - the holdout dataset set used for that meta run
- Add a flag in Meta UI:
  - “Save per‑dataset holdout details (for ML)” (default OFF initially)

**Payoff:**
- ML training becomes stable with fewer meta runs.
- Enables better CV splits (group by dataset label and/or meta run).
- Unlocks uncertainty estimation and conformal calibration later.

**Key risks / mitigations:**
- File size growth: keep it opt‑in; store only top_k; store floats compactly.
- Leakage: per‑dataset labels must still be scoped to `(strategy_kind, timeframe, ma_source, etc.)`.

---

### 2) Calibrate the probabilities (high ROI, minimal compute)

**Problem:** Logistic regression outputs probabilities, but they can still be **miscalibrated** (too confident or too timid).

**Upgrade:** Add a lightweight calibrator trained on out‑of‑fold predictions:

- **Platt scaling** (sigmoid on logits) or **temperature scaling** (a special case).
- Store calibrator params in the model file and apply at inference time.

**Implementation sketch:**
- During training we already compute out‑of‑fold predictions (`cv_ps`, `cv_ys`).
- Fit `p_cal = sigmoid(a * logit(p) + b)` (Platt scaling).
- Report:
  - AUC (discrimination)
  - Brier score (calibration + sharpness)
  - Reliability bins
- In UI: show a “Model health” row and a warning if calibration is poor.

**Payoff:**
- “80% confidence” becomes closer to a meaningful operating statement.
- Better for auto‑decision rules (e.g., “only auto‑promote groups with confidence ≥ 0.9”).

**References:**
- Temperature scaling: https://arxiv.org/abs/1706.04599
- Calibration overview (Platt / isotonic): https://scikit-learn.org/stable/modules/calibration.html
- Reliability diagrams perspective: https://arxiv.org/abs/2305.16930
- SmoothECE (better-behaved ECE): https://arxiv.org/abs/2309.12236
- Platt scaling chapter reference: https://transferlab.ai/refs/platt_probabilistic_1999/
- Improved Platt algorithm note (Lin et al. 2007): https://link.springer.com/content/pdf/10.1007%2Fs10994-007-5018-6.pdf

---

### 3) “Autopilot v2”: retrain when new labels arrive (high UX ROI)

**Problem:** Current autopilot only trains when no model exists, so models silently go stale as you accumulate more meta+holdout evidence.

**Upgrade:** Add model staleness detection and safe auto‑retrain:

- Model file stores:
  - `trained_ts`
  - `labeled_rows`
  - `artifact_fingerprint` (e.g., newest meta ts used + count + last modified times hash)
  - training metrics snapshot (AUC/Brier)
- On Meta open / run / load:
  - If new labeled artifacts exist beyond the fingerprint and user has Autopilot ON → retrain.
  - Add a cooldown to avoid retraining multiple times in a row.

**Payoff:**
- “Set and forget” ML: the model improves automatically as you do more real work.

**Key risk / mitigation:**
- Surprise compute: only retrain when “enough new labels” (e.g., +100 rows) or when last training is “old”.

---

### 4) Evidence-aware “what to evaluate next” (active-learning style scheduling)

**Problem:** ML helps ranking but doesn’t yet reduce expensive evaluation.

**Upgrade:** Add a mode that uses ML to decide which meta groups deserve expensive holdout (or heavy validation) next.

Pragmatic v1 scheduler:
- Run meta selection (cheap) → get candidate groups (unlabeled).
- Score with ML:
  - **Exploit:** evaluate top `K_exploit` by confidence.
  - **Explore:** evaluate `K_explore` with highest uncertainty (closest to 0.5 probability).
  - Optional: ensure diversity by sampling across parameter clusters.
- Run holdout evaluation only for the selected set (not all groups).

This is “active learning” in spirit: spend expensive evaluation where it improves the model/decision most.

**References (uncertainty sampling + measurement):**
- “How to measure uncertainty in uncertainty sampling for active learning” (Nguyen et al., 2022): https://link.springer.com/article/10.1007/s10994-021-06003-9

---

### 5) Conformal prediction / risk control (optional but powerful)

**Problem:** Even calibrated probabilities don’t give **finite-sample guarantees**.

**Upgrade (v2+):** Use conformal prediction / risk control to produce “safe sets” like:

- “These groups pass the holdout gate with error ≤ ε” (under exchangeability assumptions).

This can be exposed as:
- “Show only groups in the conformal ‘pass set’”,
- or “Abstain unless confidence set is tight”.

**References:**
- Tutorial (JMLR): https://jmlr.csail.mit.edu/papers/v9/shafer08a.html
- arXiv preprint: https://arxiv.org/abs/0706.3188
- MAPIE docs (good reference list for risk control): https://mapie.readthedocs.io/

---

### 6) Make ML genuinely strategy-agnostic (data plumbing)

Today ML can only learn where we can generate labels (holdout results). To expand:

- Add holdout evaluation support for more strategies (e.g., Double‑MA meta).
- Ensure artifact schema stores enough info to rerun candidate evaluation consistently.

This is more engineering than modeling, but it increases the labeled corpus dramatically.

---

## Recommended rollout (maximize ROI, minimize risk)

### v1.1 (small / safe)
1) Store model metadata + staleness detection (Autopilot v2).
2) Add Platt scaling calibration from out‑of‑fold predictions.
3) Add “model health gate” in UI (hide/gray ML if CV AUC < threshold or Brier worse than baseline).

### v1.2 (medium)
1) Add optional “save holdout per‑dataset rows” to meta artifacts.
2) Retrain using per‑dataset rows (much larger labeled dataset).
3) Add simple “evaluate top‑N groups only” option to reduce holdout compute.

### v1.3 (heavier, research-y)
1) Conformal risk control sets for “pass/fail with guarantees”.
2) Active-learning style scheduling (exploit+explore, diversity).

---

## Benchmarks / regression guardrails

We now have an end‑to‑end benchmark including ML:

- `crates/app_core/benches/e2e_workflow_ml.rs`

Use it to measure:
- baseline runtime (no ML),
- ML training overhead,
- ML apply overhead,
- scaling with candle count via `VTA_E2E_BENCH_LENS`.

---

## Open questions (need your preference)

1) Should “confidence” be interpreted as:
   - P(pass holdout gate), or
   - P(being top‑X% on holdout), or
   - something multi-objective (sharpe vs dd)?
2) Do you want ML to:
   - only rank, or
   - actually drive auto‑evaluation scheduling (cost saving)?
3) How large can meta artifacts be before it becomes annoying (MB per run)?

