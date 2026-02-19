# Stage 5 Post-Meta AI/ML Layer: Research and Implementation Blueprint

Last updated: 2026-02-16
Scope: `ta_desktop_demo` only. This document defines a final AI/ML layer that runs after meta-analysis.

## 1) Executive Summary

The right high-level goal is:

- Predict `P(generalizes out-of-sample)` for each meta-selected parameter group.
- Use that probability to improve final ranking and reduce false positives.
- Keep this layer as a strict add-on to meta-analysis, not a replacement for validation discipline.

This stage is best treated as:

- Stage 5 (`Meta -> ML confidence -> final shortlist`), initially in shadow mode.
- A systems research problem in reliable decision-making under finite validation budget.

## 2) Why Stage 5 Should Be Post-Meta

Post-meta is the best first integration point because:

- Labels are strongest there (holdout outcomes, cross-dataset aggregation).
- Leakage risk is lower than training directly on optimization-only outputs.
- The app already has the core training path and model lifecycle for this layer.

Current code already supports this direction:

- Model train/apply path in `crates/app_core/src/ml_confidence.rs`.
- Tauri integration in `src-tauri/src/commands_meta.rs`.
- Meta artifact persistence and scope filters in `src-tauri/src/commands_meta.rs` and `src-tauri/src/run_artifacts.rs`.

## 3) Research Thesis (Falsifiable)

Thesis:

- Adding a calibrated ML confidence layer after meta-analysis increases holdout pass precision at top-K without increasing false discovery rate, relative to meta-score-only ranking.

Primary falsifiable claims:

1. `Precision@K` on holdout improves vs meta-only baseline.
2. `Brier score` and calibration error improve after calibration.
3. For equal or better precision, fewer candidates need heavy follow-up validation.

If these fail on out-of-time/out-of-dataset tests, the ML layer should remain advisory only.

## 4) Target Definition

The target should remain robust-generalization oriented, not PnL-maximization.

Recommended target:

- Binary target: pass/fail based on holdout quality gates.

Candidate gate definition:

- `holdout_pass_rate >= threshold`
- `holdout_score_p10 >= threshold`
- Optional `holdout_max_dd_p90 <= threshold`

This matches the existing model intent and avoids in-sample leakage.

## 5) Data Unit, Features, and Labels

Training unit:

- One meta group row (or one holdout row variant when available).

Feature set (current, stable baseline):

- `samples`
- `pass_rate`
- `robust_score_p10`
- `robust_score_median`
- `max_dd_p90`
- `cross_score_median`
- `meta_score`

These are already implemented in `ml_confidence.rs` and should be kept as the baseline feature contract.

Label sources:

- Preferred: per-dataset holdout row labels (`holdout_rows`) when available.
- Fallback: group-level holdout aggregate label.

## 6) Leakage and Split Protocol

Use strict split rules:

1. Train only on artifacts earlier than evaluation timestamp.
2. Keep symbol-level separation where possible (no same dataset family in train and test slices).
3. Use rolling-window evaluation (walk-forward by artifact time).
4. Do not derive labels from any metric computed on the candidate being scored in the same evaluation fold.

Minimum data quality gates:

- Minimum labeled rows before training (keep existing threshold floor).
- Positive/negative class balance sanity checks.
- Reject or downgrade unhealthy models.

## 7) Recommended Model Stack

Default production model:

- Regularized logistic regression with standardized features.
- Probability calibration (Platt scaling).
- Health checks for training stability and calibration quality.

Why this baseline:

- Fast, deterministic, interpretable, and already compatible with your code path.
- Easy to audit and less fragile than high-capacity models on sparse labels.

Optional research ablations:

1. Gradient-boosted tree classifier for non-linear effects.
2. Isotonic calibration vs Platt.
3. Hierarchical model by strategy/timeframe scope.

Only promote an ablation if out-of-time calibration and precision are better.

## 8) Stage 5 Decision Policy

Stage 5 output per group:

- `ml_confidence` in `[0, 1]`
- `ml_uncertainty` (from calibration residuals or bin-count reliability)
- `final_rank_score`
- `decision_bucket`: `accept`, `review`, `reject`

Recommended ranking formula:

```text
final_rank_score =
    w_meta * z(meta_score)
  + w_ml   * z(logit(clamp(ml_confidence)))
  - w_unc  * z(ml_uncertainty)
```

Suggested starting weights:

- `w_meta = 0.60`
- `w_ml = 0.35`
- `w_unc = 0.05`

Decision buckets:

- `accept`: confidence high and uncertainty low.
- `review`: confidence moderate or uncertainty high.
- `reject`: confidence low.

This keeps ranking robust while preserving a human-auditable review lane.

## 9) Artifact and API Design for Stage 5

Add a new artifact kind:

- `vectorbt_meta_ml_selection`

Suggested artifact payload:

- `kind`, `ts`, `scope`
- model metadata (`model_version`, health flags, training window)
- top groups with:
  - original meta metrics
  - `ml_confidence`
  - `ml_uncertainty`
  - `final_rank_score`
  - `decision_bucket`
- evaluation summary (if holdout labels available at run time)

Recommended commands:

1. `run_meta_ml_selection(req)`:
   - Loads latest meta artifact(s) in scope.
   - Loads ML model.
   - Scores and re-ranks groups.
   - Saves Stage 5 artifact and optional XLSX summary.
2. `evaluate_meta_ml_policy(req)`:
   - Offline policy evaluation over historical artifacts.
   - Returns metrics and confidence intervals.

## 10) UI and Workflow Integration

UI placement:

- New panel after Meta: "Stage 5 - ML Confidence Finalizer".

Inputs:

- Strategy/timeframe scope.
- Model source (latest or fixed version).
- Decision thresholds and blend weights.
- Shadow mode toggle.

Outputs:

- Confidence-ranked shortlist.
- Reliability/calibration panel.
- Actionable buckets (`accept`, `review`, `reject`).
- Export path to Stage 5 report.

Initial release mode:

- Shadow mode default: show recommendations, do not auto-rewrite meta ranking unless explicitly enabled.

## 11) Evaluation Protocol

Baselines:

1. Meta-only ranking.
2. Random re-ranking sanity baseline.
3. Stage 5 blended ranking.

Primary metrics:

- `Precision@K` on holdout pass labels.
- `Recall@K`.
- `ROC-AUC` and `PR-AUC`.
- `Brier score`.
- Expected calibration error (ECE).
- Top-K median holdout score and drawdown profile.

Operational metrics:

- Number of candidates sent for heavy validation to hit a target precision.
- Runtime overhead of Stage 5.

Statistical procedure:

- Rolling out-of-time evaluation.
- Bootstrap confidence intervals for key deltas.

Promotion criteria (example):

1. Positive `Precision@K` lift with non-overlapping confidence interval vs meta-only.
2. Better or equal calibration (`Brier` and ECE).
3. No degradation in holdout risk profile at selected K.

## 12) Phased Implementation Plan

Phase 0: Research harness (offline only)

- Build `evaluate_meta_ml_policy`.
- Freeze benchmark splits and baselines.
- Produce first reproducible report.

Phase 1: Stage 5 shadow mode in app

- Add `run_meta_ml_selection` command.
- Add Stage 5 artifact writing and list/load UI support.
- Show confidence and bucketing without auto-enforcement.

Phase 2: Advisory gating

- Enable optional "use Stage 5 rank for final shortlist" switch.
- Keep one-click revert to meta-only ranking.

Phase 3: Closed-loop budget optimization (optional)

- Feed Stage 5 confidence back into candidate triage budgeting.
- Use this only after Phase 1/2 pass criteria are met.

## 13) Risks and Mitigations

Risk: label sparsity.

- Mitigation: scope aggregation, stricter minimum rows, remain shadow-only until enough data.

Risk: non-stationarity/drift.

- Mitigation: rolling retrain windows, staleness checks, model health flags.

Risk: leakage.

- Mitigation: strict time-aware split and symbol-aware segregation.

Risk: overfitting by model complexity.

- Mitigation: start with linear baseline, require out-of-time lift before complexity increases.

Risk: false confidence from uncalibrated probabilities.

- Mitigation: mandatory calibration diagnostics and hard "unhealthy model" fallback.

## 14) Definition of Success

Stage 5 is successful if:

1. It improves final shortlist quality on unseen holdout data.
2. It remains interpretable and operationally stable.
3. It can be disabled without disrupting the baseline optimize/validate/meta workflow.

If these are not met, keep Stage 5 as an offline research tool rather than production ranking logic.
