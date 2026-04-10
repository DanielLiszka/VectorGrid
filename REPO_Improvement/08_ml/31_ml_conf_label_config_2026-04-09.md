# ML Confidence Label Config Plan

Last updated: 2026-04-09

## Purpose

Define the current ML-confidence label-configuration contract so aggregate-holdout labels, per-dataset-row labels, default thresholds, and override behavior remain explicit and do not drift into misleading or incompatible semantics.

## Scope Boundaries

In scope:

- `MlConfidenceLabelConfig`
- default threshold values
- aggregate-holdout label semantics
- per-dataset-row label semantics
- status/report surfaces that show label thresholds
- override paths that carry label config into evaluation or saved model metadata

Out of scope:

- scope identity
- training-row counting except where label config determines label presence
- general Stage 5 blend-weight behavior
- options workflows

## Current User-Facing Behavior

Users currently observe label configuration indirectly through ML-confidence status output, saved model summaries, and policy-evaluation flows rather than editing it as a large primary UI surface. Current shipped behavior means:

- the status surface shows the active label thresholds in a compact summary form
- saved models retain the label config used when they were trained
- policy-evaluation requests can carry label-config overrides
- aggregate-holdout training and per-dataset-row training do not use identical threshold semantics even though they share the same config type
- default label thresholds are opinionated and materially affect both row availability and what “positive” means for ML-confidence

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - user-facing ML-confidence status surface
- `web/src/services/metaService.ts`
  - label-threshold status formatting from returned model summaries
- `crates/app_core/src/ml_confidence.rs`
  - canonical `MlConfidenceLabelConfig`, default values, aggregate-holdout label helper, per-dataset-row label helper, and saved model metadata embedding
- `src-tauri/src/commands_meta.rs`
  - policy-eval request handling and status/report payloads that carry label config

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- tests embedded in `crates/app_core/src/ml_confidence.rs`
- tests embedded in `src-tauri/src/commands_meta.rs`

## Strengths Already Present

- label config is centralized in one backend type instead of being copied into multiple partial structs
- the default thresholds are explicit and serializable
- saved model summaries already preserve label config for later inspection
- policy-evaluation flows can already carry label-config overrides rather than silently assuming one hard-coded threshold set
- the code comments already acknowledge that aggregate and per-dataset labeling are intentionally not identical

## Weaknesses / Failure Modes

- the same field names mean slightly different things depending on whether aggregate-holdout or per-dataset-row labels are being generated
- users mostly see compact status text, not the full semantic differences behind those thresholds
- default threshold changes would affect both row availability and target meaning, making them easy to underestimate
- tests often lower `min_holdout_datasets`, which can confuse readers into thinking production defaults are looser than they are
- if overrides are introduced more broadly later, the repo will need clearer provenance reporting for which label config produced which model or policy result

## Regressions To Avoid

- describing aggregate and per-dataset label semantics as identical when they are not
- changing defaults without updating saved-model expectations, status text, and tests together
- weakening `min_holdout_datasets` or other thresholds without a deliberate evidence-quality review
- hiding label-config provenance from saved model summaries or policy-eval outputs
- letting override behavior become more permissive than the saved metadata makes visible
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because label quality and threshold design directly affect the meaning of confidence outputs and calibration quality
- [scikit-learn Model Selection and Evaluation](https://scikit-learn.org/stable/model_selection.html)
  - relevant because label thresholds are part of the evaluation target definition, not just a UI preference
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because a usable model report should state what positive labels mean and under which thresholds the model was trained
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because label config is persisted in model JSON and policy-eval request payloads as a structured contract
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document `MlConfidenceLabelConfig` as the shared threshold container but not as a promise of identical semantics across all label paths.
2. Record the exact default values and treat them as behavior-defining defaults, not incidental constants.
3. Separate aggregate-holdout labeling rules from per-dataset-row labeling rules in the plan and in any later implementation docs.
4. Keep saved-model provenance explicit so later reviewers can tell which thresholds produced a given model.
5. Make current override paths explicit and narrow so future UI expansion does not accidentally create hidden threshold drift.
6. Preserve the distinction between “fewer labeled rows because thresholds are strict” and “fewer rows because artifacts are missing or incompatible.”
7. Tie any future threshold changes to calibration/quality review rather than treating them as harmless parameter tweaks.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every field in `MlConfidenceLabelConfig` and its current default value.
- [x] Document aggregate-holdout label semantics for pass rate, score threshold, dataset count, and optional drawdown cap.
- [x] Document per-dataset-row label semantics and how they differ from aggregate-holdout labels.
- [x] Document how label config is surfaced in status text and saved model summaries.
- [x] Document current policy-eval override behavior and where it enters the backend.
- [x] Document that tests often use looser thresholds than production defaults.
- [x] Preserve tests for aggregate-label behavior, dataset-row label behavior, and label-threshold status rendering.
- [x] Preserve saved-model metadata carrying the effective label config.
- [x] Treat any default-threshold change as a compatibility-sensitive model-behavior change.
- [x] Keep docs explicit that threshold semantics are part of target definition, not mere cosmetic tuning.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run targeted ML-confidence tests in `crates/app_core/src/ml_confidence.rs`
- run targeted ML-confidence/policy-eval tests in `src-tauri/src/commands_meta.rs`
- inspect one saved model summary for label-threshold persistence
- manually compare status text for a model trained under the current default label config
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current defaults until there is an explicit label-policy review and compatibility plan
- if label editing becomes a bigger UI surface later, keep saved-model provenance and status copy explicit
- prefer additive override/report fields over redefining current threshold semantics
- if aggregate and dataset-row label paths diverge further, make that divergence more visible rather than hiding it behind one compact status string
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making label-threshold provenance visible in the ML contract surface and in the formatted status regression tests.
- The ML panel now shows the current label summary directly, while `metaService.test.ts` keeps the formatted status pinned to the effective threshold values returned by the backend.
- Current UI behavior remains honest about provenance:
  - label thresholds come from saved model metadata/status output
  - they are not yet a large interactive editor surface
  - they remain part of target definition, not cosmetic tuning
- Stable fixture / replay payload:
  - `label: pass>=0.60, p10>=0.100, datasets>=5`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts`
  - `cargo test -p ta_desktop_demo_app commands_meta --lib`
  - `cargo test -p ta_app_core ml_confidence --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/29_ml_conf_scope_identity_2026-04-09.md`
- `REPO_Improvement/08_ml/30_ml_conf_training_rows_2026-04-09.md`
- `REPO_Improvement/08_ml/10_meta_ml_confidence_reports_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-09.md`
