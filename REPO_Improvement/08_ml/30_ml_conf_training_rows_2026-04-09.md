# ML Confidence Training Rows Plan

Last updated: 2026-04-09

## Purpose

Define the current ML-confidence training-row contract so `available_labeled_rows`, `trained_rows`, dataset-row versus aggregate-row targets, and retrain-readiness logic remain explicit and comparable across status surfaces, training code, and saved model metadata.

## Scope Boundaries

In scope:

- preparation of labeled training rows from saved meta artifacts
- aggregate-holdout versus per-dataset-row label families
- `available_labeled_rows` status semantics
- `trained_rows` saved-model metadata semantics
- retrain-readiness comparison logic
- warnings and failure modes caused by row scarcity or row-family mismatch

Out of scope:

- scope identity itself beyond the rows being filtered by scope
- label-threshold configuration details beyond what affects row presence
- Stage 5 scoring/blending after a model already exists
- options workflows

## Current User-Facing Behavior

The Meta workspace status line exposes ML-confidence data availability and model freshness in row-oriented terms. Current shipped behavior means:

- the status surface reports available meta artifacts and available labeled rows for the current scope
- if a model exists, the status surface also reports `trained_rows`
- users do not directly choose between aggregate-holdout and per-dataset-row training paths; `target=auto` resolves that internally
- dataset-row training is preferred when `holdout_rows` are available, otherwise aggregate holdout is used
- training hard-fails when there are too few usable labeled rows
- retrain recommendations compare current row availability to the last trained-row count rather than only checking for the existence of any new artifact

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - user-facing ML-confidence status surface
- `web/src/services/metaService.ts`
  - status formatting and ML-confidence result rendering
- `crates/app_core/src/ml_confidence.rs`
  - row preparation, target resolution, training, metrics, warnings, and saved-model metadata
- `src-tauri/src/commands_meta.rs`
  - status aggregation, row-availability reporting, and retrain-readiness logic
- `src-tauri/src/commands_reports_meta_xlsx.rs`
  - current workbook surfaces that expose train-row-derived tables without being a dedicated ML-confidence workbook

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- tests embedded in `crates/app_core/src/ml_confidence.rs`
- tests embedded in `src-tauri/src/commands_meta.rs`
- report tests touching `src-tauri/src/commands_reports_meta_xlsx.rs`

## Strengths Already Present

- row preparation is centralized in the backend instead of being duplicated in the frontend or Tauri layer
- current status distinguishes `available_labeled_rows` from `trained_rows`
- `target=auto` already prefers richer per-dataset labels when they exist
- model metadata persists row counts, target family, and health/warnings
- retrain logic uses a concrete delta threshold instead of retraining on every new artifact

## Weaknesses / Failure Modes

- “rows” can mean either aggregate-holdout rows or expanded per-dataset rows, which is easy to oversimplify
- `available_labeled_rows` and `trained_rows` are related but not equivalent; they come from different times and different sources
- status text can look deceptively simple compared with the richer backend warning logic that produced it
- row scarcity can come from many causes:
  - scope mismatch
  - insufficient holdout datasets
  - missing holdout aggregates
  - missing `holdout_rows`
  - invalid metric values
- the current workbook surfaces use training rows indirectly, but they are not a dedicated ML-confidence provenance report

## Regressions To Avoid

- conflating `available_labeled_rows` with `trained_rows`
- changing `target=auto` resolution without updating status, metadata, and docs together
- weakening the minimum-row guard without deliberate evaluation
- hiding warnings that explain why artifacts were skipped during row preparation
- describing row counts as simple “number of groups” when dataset-row expansion can produce many rows per group

## External Research Summary

- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because row preparation is still a time-aware evaluation problem and should not drift toward shuffled assumptions
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because row counts and target quality affect calibration usefulness, not just raw classifier fit
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because trained-row counts, label provenance, and health warnings are part of the minimum useful reporting surface for a shipped model
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because row availability and model metadata cross the command boundary and must remain stable in meaning

## Proposed Improvement Approach

1. Document row semantics around three distinct concepts:
   - raw scoped meta artifacts
   - available labeled rows
   - saved model `trained_rows`
2. Make `target=auto` resolution explicit so it is clear when the system switches from aggregate labels to per-dataset labels.
3. Record the current minimum-row threshold and the current retrain-delta threshold in one place.
4. Keep row-preparation warnings first-class because missing rows are often caused by compatibility or evidence issues rather than true absence of data.
5. Describe status-line fields and saved-model metadata as different views over the same training-row pipeline, not interchangeable numbers.
6. Note the current report gap: workbook surfaces reflect row-derived outputs indirectly but do not yet provide a dedicated ML-confidence provenance sheet.
7. Tie every row-family-sensitive behavior to its current owner and test surface.

## Checklist

- [ ] Document how scoped meta artifacts become candidate training rows.
- [ ] Document the difference between aggregate-holdout rows and per-dataset `holdout_rows`.
- [ ] Document `target=auto` resolution order and how it changes the row family used for training.
- [ ] Document the current minimum labeled-row threshold required to train.
- [ ] Document how `available_labeled_rows` is computed for status surfaces.
- [ ] Document how `trained_rows` is stored in model metadata.
- [ ] Document the current retrain-readiness delta threshold logic.
- [ ] Document warning/failure causes for row scarcity and invalid rows.
- [ ] Preserve tests for dataset-row preference, aggregate fallback, and insufficient-row rejection.
- [ ] Preserve status rendering tests that distinguish row availability from trained-model metadata.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run targeted ML-confidence tests in `crates/app_core/src/ml_confidence.rs`
- run targeted ML-confidence status tests in `src-tauri/src/commands_meta.rs`
- inspect one saved ML-confidence model summary for `trained_rows`
- manually compare status before and after adding new scoped meta artifacts

## Rollout / Fallback / Compatibility Notes

- keep current row-count semantics additive and explicit if new row families are introduced later
- preserve saved `trained_rows` metadata so existing model summaries remain interpretable
- if the retrain heuristic changes, document the new threshold logic alongside the status-copy changes
- prefer extra warning detail over collapsing row-scarcity causes into a generic “not enough data” message

## Related Docs

- `REPO_Improvement/08_ml/29_ml_conf_scope_identity_2026-04-09.md`
- `REPO_Improvement/08_ml/31_ml_conf_label_config_2026-04-09.md`
- `REPO_Improvement/08_ml/10_meta_ml_confidence_reports_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-09.md`
