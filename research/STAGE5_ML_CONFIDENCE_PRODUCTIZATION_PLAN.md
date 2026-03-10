# Stage 5 ML Confidence Productization Plan

Last updated: 2026-03-09
Scope: `ta_desktop_demo` backend + frontend
Goal: graduate the current post-meta ML layer from "experimental tooling" into a reliable, user-facing Stage 5 workflow.

## Why This Stage Merits Completion

The current ML layer already has the right shape to be useful:

- It is a post-meta ranking layer, not a direct price-prediction system.
- It learns from holdout outcomes, which is the correct label source for this product.
- It already has health checks, calibration, scope filters, and UI entry points.

That gives it real merit as a final filter after:

`optimize -> validate -> meta-select -> ML confidence`

The right bar is not "AI that finds alpha by itself."
The right bar is:

- reduce false positives among top meta candidates
- improve shortlist quality at fixed review budget
- remain optional and auditable

## Current State

Implemented today:

- model training, scoring, and calibration in `crates/app_core/src/ml_confidence.rs`
- Stage 4 orchestration and artifact saving in `src-tauri/src/commands_meta.rs`
- UI actions for refresh/train/apply/run Stage 4 in `web/src/components/MetaWorkspacePanel.vue`
- saved Stage 4 artifact browsing in the meta workspace

Current limitations:

- command and artifact naming still center on `Stage 4 experimental`
- success criteria are not enforced by a dedicated offline evaluation workflow
- UX is workable but still feels like advanced tooling instead of a guided production step
- model-health gates are present, but operational policy is not fully codified
- rollout policy is not clearly split into advisory mode vs production mode

## Product Definition

Stage 5 should be a purpose-built finalization layer that:

1. consumes a completed meta artifact
2. loads or trains a scoped ML confidence model
3. scores each meta group
4. blends ML confidence with meta score
5. emits an auditable shortlist with decision buckets

Recommended product semantics:

- `accept`: high-confidence, low-uncertainty candidates worth immediate shortlist inclusion
- `review`: candidates that remain plausible but need human or heavy-validation review
- `reject`: candidates that score well in meta but look weak historically on holdout behavior

## Success Criteria

This stage should only be considered "real" when all of the following are true:

- Stage 5 improves `Precision@K` versus meta-only ranking on historical holdout evaluation.
- Model health gating blocks obviously weak models by default.
- A user can run the full flow from the UI without needing backend-only knowledge.
- Output artifacts are easy to inspect and explain.
- There is a clear fallback path to pure meta ranking.

## Backend Plan

### 1. Rename and stabilize the product surface

Replace "Stage 4 experimental" language with "Stage 5 ML Confidence" in the backend-facing API while preserving compatibility for existing artifacts and callers.

Desired outcomes:

- stable request/result naming
- stable artifact kind naming
- compatibility shim for older `vectorbt_meta_stage4_experimental` artifacts

### 2. Formalize model health policy

The current AUC/Brier checks are directionally correct, but they need explicit policy semantics.

Backend work:

- define `healthy`, `warning`, and `blocked` model states
- distinguish "no model", "stale model", "legacy model", and "unhealthy model"
- make fallback behavior explicit in result payloads
- add a backend summary of why a model was accepted or downgraded

### 3. Add offline policy evaluation

This is the highest-value backend gap.

Add a dedicated evaluator that can answer:

- does ML improve top-K holdout precision vs meta-only?
- for which strategy kinds and timeframes does it help?
- how sensitive is it to threshold and blend-weight choices?

Suggested command:

- `evaluate_ml_confidence_policy`

Suggested outputs:

- baseline meta-only metrics
- ML-only metrics
- blended Stage 5 metrics
- per-scope summary
- warnings for insufficient labeled data

### 4. Harden training data rules

Current training is reasonable, but production use needs stricter policy around data sufficiency and leakage.

Backend work:

- codify minimum labeled-row thresholds per scope
- enforce minimum class-balance thresholds
- surface training-set size and class-balance warnings in artifact metadata
- keep source-meta exclusion mandatory
- preserve group-aware CV and document it in result metadata

### 5. Improve artifact structure

The final artifact should be optimized for review and downstream use.

Backend work:

- add explicit Stage 5 artifact kind
- include source meta artifact identity
- persist model identity and health state
- persist blend weights and thresholds used
- persist fallback reason when Stage 5 falls back to meta-only ranking

### 6. Define rollout modes

Stage 5 should have explicit modes:

- `shadow`: compute and display only
- `advisory`: reorder shortlist but preserve original meta score and fallback markers
- `strict`: require healthy model and enforce ML confidence thresholds

## Frontend Plan

### 1. Reframe the UI around a guided Stage 5 workflow

The current UI exposes the parts, but the workflow still feels fragmented.

Frontend work:

- rename the panel from ad hoc Stage 4 wording to a deliberate Stage 5 section
- present the flow in order:
  - holdout-ready meta
  - model status
  - training/update
  - run Stage 5
  - inspect shortlist
- make prerequisites obvious before the user clicks run

### 2. Improve model-status visibility

Users need to know whether the ML model is usable without reading raw JSON.

Frontend work:

- display model health state prominently
- show rows used, positives/negatives, CV AUC, calibrated Brier, and staleness
- show why training is recommended
- show when Stage 5 is running in fallback mode

### 3. Make holdout-row collection a first-class prerequisite

This is the practical dependency that determines model quality.

Frontend work:

- explain that "Save per-dataset holdout rows" improves training quality
- warn when the current meta artifact lacks row-level labels
- guide the user to rerun meta with holdout rows enabled when needed

### 4. Improve Stage 5 output review

The result table should make the ML layer explainable.

Frontend work:

- show `ml_confidence`, `ml_uncertainty`, `final_rank_score`, and `decision_bucket`
- keep original meta rank visible for comparison
- add filters for `accept`, `review`, `reject`
- make fallback-to-meta cases visually obvious

### 5. Add evaluation/reporting UX

If we add backend policy evaluation, the UI should expose it directly.

Frontend work:

- add a "Evaluate Stage 5 policy" action
- render comparison metrics against meta-only baseline
- surface per-scope recommendations for weights/thresholds

## Recommended Implementation Order

1. Backend policy evaluation command and metrics output
2. Backend model-health/state formalization
3. Stable Stage 5 artifact and request/result schema
4. Frontend model-status and prerequisite UX
5. Frontend Stage 5 workflow and output review improvements
6. Rename/deprecate "experimental Stage 4" terminology after compatibility shims are in place

## Risks

- The model may be mildly predictive but not strong enough to justify hard gating.
- Sparse scopes may produce unstable training and misleading confidence.
- Users may overtrust a probability score unless uncertainty and fallback state are visible.
- Renaming the stage too early could overstate maturity before evaluation proves merit.

## Non-Goals

- replacing validation with ML
- replacing meta-analysis with ML
- adding high-complexity deep-learning infrastructure
- using ML to score raw optimization outputs directly

## Checklist

- [ ] Define final naming: Stage 5 ML Confidence vs legacy Stage 4 experimental compatibility.
- [ ] Add a dedicated backend policy-evaluation command for ML-vs-meta comparison.
- [ ] Define explicit model states: healthy, warning, blocked, stale, missing, legacy.
- [ ] Enforce clearer training-data sufficiency and class-balance rules.
- [ ] Persist stronger model metadata in saved model and Stage 5 artifacts.
- [ ] Add a new stable Stage 5 artifact kind and compatibility handling for old artifacts.
- [ ] Add backend result fields for fallback reason and policy mode used.
- [ ] Decide and encode rollout modes: shadow, advisory, strict.
- [ ] Redesign the frontend section into a guided Stage 5 workflow.
- [ ] Surface model health and training-quality metrics prominently in the UI.
- [ ] Warn when the source meta artifact lacks `holdout_rows`.
- [ ] Show Stage 5 outputs with original meta rank, ML confidence, uncertainty, and decision bucket.
- [ ] Add filtering and review affordances for `accept`, `review`, and `reject`.
- [ ] Expose backend policy evaluation in the frontend.
- [ ] Add targeted tests for compatibility, fallback behavior, and policy evaluation.
- [ ] Write a short operator guide for when Stage 5 should be trusted vs advisory only.

## Exit Condition

Stage 5 should be declared production-ready only when:

- offline evaluation shows repeatable improvement over meta-only ranking in at least some supported scopes
- unhealthy or stale models are clearly handled
- the UI makes prerequisites and fallback state obvious
- a user can complete the workflow end to end without backend intervention
