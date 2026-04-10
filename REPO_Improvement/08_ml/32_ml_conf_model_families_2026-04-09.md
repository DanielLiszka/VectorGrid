# ML Confidence Model Families Plan

Last updated: 2026-04-09

## Purpose

Define the current ML-confidence model-family contract so backend family selection, auto-family fallback behavior, stored `model_kind`, and frontend status expectations remain explicit and do not drift into imaginary user configurability.

## Scope Boundaries

In scope:

- `MlConfidenceModelFamily`
- `MlConfidenceModelKind`
- auto-family resolution
- family-specific defaults inside `MlConfidenceTrainConfig`
- saved model metadata reflecting the chosen model kind
- frontend/status implications of backend-only family selection

Out of scope:

- scope identity
- row counting and label-threshold semantics except where they affect family selection
- Stage 5 ranking/blending after a model has already been trained
- options workflows

## Current User-Facing Behavior

There is currently no dedicated frontend selector for ML-confidence model family. Users can refresh, train/update, apply the model to current meta results, and run Stage 5, but the shipped UI trains through the backend default path rather than an explicit family picker. Current behavior means:

- the effective family decision is backend-owned
- the frontend trains with `cfg: null` in the normal path, so backend defaults decide the family
- status/report surfaces reveal the resolved `model_kind`, not an “Auto” runtime family
- saved models persist the actual trained model kind and supporting metadata
- users can infer the chosen family from status/model summaries, but not choose it directly from the shipped panel

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - ML-confidence actions and status surface, without a dedicated family selector
- `web/src/services/metaService.ts`
  - current train/update flow using default backend config and rendering returned model status
- `crates/app_core/src/ml_confidence.rs`
  - `MlConfidenceModelFamily`
  - `MlConfidenceModelKind`
  - `MlConfidenceTrainConfig`
  - auto-family resolution
  - logistic and GBDT training paths
  - saved model summary/meta fields
- `src-tauri/src/commands_meta.rs`
  - training command boundary and scoped model save/load behavior

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- tests embedded in `crates/app_core/src/ml_confidence.rs`
- tests embedded in `src-tauri/src/commands_meta.rs`

## Strengths Already Present

- family selection is centralized in the backend rather than fragmented across UI and backend layers
- `Auto`, `Logistic`, and `GbdtStump` are explicit types, not stringly typed conventions
- saved models persist the resolved `model_kind` separately from training intent
- auto-family fallback already degrades safely from GBDT to logistic when the richer path is not viable
- the frontend’s lack of a selector means current product behavior is stable and harder to misconfigure interactively

## Weaknesses / Failure Modes

- it is easy to assume “Auto” is a stable runtime model kind when it is really a training-time resolver
- because the UI does not expose family selection, changes in backend auto-family behavior can have large product impact with low visibility
- family choice depends on row count and diversity, so the same scope can resolve differently as more data arrives
- GBDT failure falls back to logistic with warnings, which is safe but easy to miss if docs only describe the happy path
- status surfaces expose the resolved kind only after training/status refresh, not as a proactive planning surface

## Regressions To Avoid

- documenting model family as user-configurable when the current shipped UI does not expose it
- changing auto-family heuristics without updating tests and saved-model expectations together
- conflating `model_family` with `model_kind`
- hiding GBDT-to-logistic fallback warnings
- weakening saved-model metadata so the resolved model kind becomes harder to inspect later
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn LogisticRegression](https://scikit-learn.org/stable/modules/generated/sklearn.linear_model.LogisticRegression.html)
  - relevant because the current linear baseline family is logistic and its regularized, probability-oriented behavior is the conceptual baseline for this repo’s linear path
- [scikit-learn HistGradientBoostingClassifier](https://scikit-learn.org/stable/modules/generated/sklearn.ensemble.HistGradientBoostingClassifier.html)
  - relevant because the repo’s `GbdtStump` path is the lightweight non-linear alternative and should be documented as a different model class with different data-shape expectations
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because both families ultimately feed calibrated probability-style confidence outputs
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because shipped model reporting should state which family actually trained and under what conditions
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document family selection as a backend-owned training contract, not a frontend feature.
2. Separate training intent (`model_family`) from persisted result (`model_kind`) explicitly.
3. Record current auto-family resolution thresholds and current fallback behavior in one place.
4. Make family-specific default knobs visible enough that later backend tuning is treated as a material behavior change.
5. Preserve the distinction between:
   - auto-family resolution
   - explicit forced family
   - runtime prediction against an already trained model
6. Keep saved-model status/report surfaces aligned with the resolved family and any fallback warnings.
7. Treat any future family selector UI as a new feature layered on top of, not a reinterpretation of, the current contract.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document all current `MlConfidenceModelFamily` variants.
- [x] Document all current `MlConfidenceModelKind` variants and how they differ from family intent.
- [x] Document the current default family (`Auto`) and current default model kind (`Logistic`) semantics.
- [x] Document current auto-family resolution thresholds and conditions.
- [x] Document current GBDT training-failure fallback to logistic.
- [x] Document family-specific train-config defaults that materially affect behavior.
- [x] Document where the resolved `model_kind` is surfaced to users and persisted in saved models.
- [x] Preserve tests for logistic training, GBDT training, and auto-family fallback behavior.
- [x] Keep frontend docs honest about the lack of a shipped model-family selector.
- [x] Treat future family-selection UI work as additive, not as a retroactive reinterpretation of the current contract.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted ML-confidence tests in `crates/app_core/src/ml_confidence.rs`
- run targeted ML-confidence command tests in `src-tauri/src/commands_meta.rs`
- inspect one saved model summary that resolved to logistic and one that resolved to GBDT if available
- manually confirm status surfaces still report the resolved `model_kind`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current auto-family behavior until there is an explicit review of downstream Stage 5 effects
- keep `model_kind` persisted even if future training surfaces expose `model_family` more directly
- prefer additive reporting fields over changing the meaning of `Auto`, `Logistic`, or `GbdtStump`
- if the UI later exposes family choice, preserve the current backend defaults for callers that still send `cfg: null`
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by surfacing the resolved `model_kind` in the formatted ML status and by making the panel explicit that family selection is backend-owned.
- `metaService.ts` now includes `model_kind=<...>` when the backend status payload provides it.
- `MetaWorkspacePanel.vue` now shows:
  - backend-owned `Auto` family semantics
  - the resolved model kind from the current status when available
  - an explicit note that the shipped UI does not expose a family picker
- `metaService.test.ts` now asserts `model_kind=logistic`, which keeps the resolved-family reporting under regression coverage.
- Stable fixture / replay payload:
  - `model_kind=logistic`
  - `target=holdout_dataset_pass`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts`
  - `cargo test -p ta_desktop_demo_app commands_meta --lib`
  - `cargo test -p ta_app_core ml_confidence --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/29_ml_conf_scope_identity_2026-04-09.md`
- `REPO_Improvement/08_ml/30_ml_conf_training_rows_2026-04-09.md`
- `REPO_Improvement/08_ml/31_ml_conf_label_config_2026-04-09.md`
- `REPO_Improvement/08_ml/33_ml_conf_health_staleness_2026-04-09.md`
- `REPO_Improvement/08_ml/34_ml_conf_apply_contract_2026-04-09.md`
