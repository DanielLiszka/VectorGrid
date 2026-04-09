# ML Confidence Health/Staleness Plan

Last updated: 2026-04-09

## Purpose

Define the current ML-confidence health and staleness contract so model state classification, stale/retrain recommendations, legacy-model handling, and frontend action gating remain explicit and compatible.

## Scope Boundaries

In scope:

- `MlConfidenceModelState`
- `MlConfidenceModelStatus`
- stale detection
- retrain recommendation rules
- legacy-model handling
- frontend status formatting and button gating that depend on model state

Out of scope:

- scope identity except where scope affects stale/status computation
- label-threshold semantics except where they affect available labeled rows
- Stage 5 selection logic beyond its dependence on health-state outputs
- options workflows

## Current User-Facing Behavior

The Meta workspace exposes ML-confidence health/staleness through one compact status surface rather than a dedicated dashboard. Current shipped behavior means:

- users see a formatted status string driven by backend state classification
- the state can be `missing`, `healthy`, `warning`, `blocked`, or `legacy`
- stale and retrain-recommended are separate concepts from unhealthy
- a legacy model is a compatibility state meaning the saved model lacks training metadata
- frontend action availability depends on whether a model exists/loads and how the backend classified it
- autopilot behavior can retrain not only on missing/unhealthy models, but also on stale or explicitly retrain-recommended ones

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - ML-confidence controls and status display
- `web/src/services/metaService.ts`
  - status formatting, status application, autopilot behavior, and ML button enablement
- `src-tauri/src/commands_meta.rs`
  - `MlConfidenceModelState`
  - `MlConfidenceModelStatus`
  - stale/retrain computation
  - legacy-model handling
  - command response shaping
- `crates/app_core/src/ml_confidence.rs`
  - health signals emitted from training metrics and saved model meta

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- tests embedded in `src-tauri/src/commands_meta.rs`
- tests embedded in `crates/app_core/src/ml_confidence.rs`

## Strengths Already Present

- state classification is centralized in the backend rather than improvised in the frontend
- health, stale, retrain-recommended, and legacy are distinct signals instead of one overloaded boolean
- retrain recommendations already include a row-growth threshold and cooldown
- legacy models are handled explicitly rather than silently treated as healthy modern models
- frontend autopilot and action gating already reuse the same cached status payload

## Weaknesses / Failure Modes

- the user-facing status is compact relative to the richer backend classification logic, so nuance can be lost
- `stale` does not imply `blocked`, and that distinction is easy to miss from the panel alone
- row-growth-based retrain heuristics can change system behavior materially even though they are not large visible settings
- a model can exist but fail to load, which creates a different operational state than “missing”
- many actions mutate the same status surface:
  - refresh
  - train/update
  - apply
  - Stage 5 completion
  - delete model

## Regressions To Avoid

- collapsing `legacy`, `warning`, and `blocked` into a simpler but less informative state model
- weakening the stale/retrain threshold logic without a deliberate review
- treating stale as synonymous with unhealthy
- hiding model-load failures inside generic “missing model” messaging
- breaking autopilot expectations by changing health-state semantics without updating the frontend logic
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because a useful model status/report surface must distinguish freshness, trustworthiness, and known limitations
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because calibration and Brier/AUC quality are part of why the repo distinguishes healthy from unhealthy models
- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because persisted model metadata should stay interpretable and tied to compatibility/freshness expectations
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the health/staleness payload crosses the command boundary and drives frontend state directly
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document health/staleness as a classification contract, not just a formatted status string.
2. Record exactly how stale is computed:
   - newer meta timestamps
   - increased artifact counts
   - row-growth thresholds
   - cooldown behavior
3. Keep legacy-model behavior explicit because it is a compatibility obligation, not a transient warning.
4. Separate “model exists but is unusable” from “no model exists.”
5. Document which frontend actions and autopilot branches depend on the cached status payload.
6. Tie unhealthy status back to training metrics so the contract remains evidence-based.
7. Preserve the difference between:
   - stale
   - retrain recommended
   - unhealthy
   - blocked
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document all current `MlConfidenceModelState` values and their meanings.
- [ ] Document the backend fields in `MlConfidenceModelStatus`.
- [ ] Document current stale-detection rules based on timestamps, artifact counts, and row growth.
- [ ] Document current retrain-recommendation thresholds and cooldown behavior.
- [ ] Document legacy-model detection and current legacy-state semantics.
- [ ] Document the difference between missing, unreadable, stale, unhealthy, and blocked states.
- [ ] Document which frontend actions/autopilot paths depend on the status payload.
- [ ] Preserve tests for stale-with-new-rows, legacy-model handling, and unhealthy-model blocking.
- [ ] Preserve frontend tests for status formatting and action gating.
- [ ] Treat any state-model simplification as a compatibility-sensitive behavior change.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted ML-confidence status tests in `src-tauri/src/commands_meta.rs`
- run targeted ML-confidence metric tests in `crates/app_core/src/ml_confidence.rs`
- manually refresh ML-confidence status with:
  - no model
  - a healthy current model
  - a stale model
  - a legacy model if available
- manually confirm autopilot behavior still matches the documented states
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current state names and meanings unless there is a coordinated frontend/backend migration
- if stale heuristics change, update both the backend classifier and the frontend explanatory text together
- keep legacy-model support explicit until older saved models are deliberately migrated or retired
- prefer richer status metadata over flattening multiple operational states into one generic warning
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/29_ml_conf_scope_identity_2026-04-09.md`
- `REPO_Improvement/08_ml/30_ml_conf_training_rows_2026-04-09.md`
- `REPO_Improvement/08_ml/32_ml_conf_model_families_2026-04-09.md`
- `REPO_Improvement/08_ml/34_ml_conf_apply_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/40_meta_ml_status_surface_2026-04-09.md`
