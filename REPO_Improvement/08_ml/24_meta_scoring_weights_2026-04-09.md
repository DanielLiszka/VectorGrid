# Meta Scoring Weights Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 3 meta scoring-weight contract so `MetaScoreWeights`, `meta_score`, holdout-adjusted `final_score`, Stage 5 blend-weight separation, and downstream artifact/report impacts remain explicit and auditable.

## Scope Boundaries

In scope:

- Stage 3 `MetaScoreWeights`
- Stage 3 `meta_score` construction
- holdout-adjusted `final_score` relationship to Stage 3 weights
- downstream consumers of Stage 3 scores
- distinction between Stage 3 weights and later Stage 5 blend weights

Out of scope:

- Stage 3 grouping/clustering controls except where they affect score inputs
- Stage 5 ML blend math except where naming overlap creates drift risk
- generic strategy metrics unrelated to Stage 3 scoring
- options workflows

## Current User-Facing Behavior

Stage 3 scoring weights are mostly an implementation contract today, not a broad first-class UI editing surface. Users see the resulting `meta_score`, later `final_score`, and downstream rankings, but they do not currently edit the Stage 3 weight vector the same way they can edit the Stage 5 blend weights. Current behavior therefore has two distinct scoring layers:

- Stage 3 computes `meta_score` using `MetaScoreWeights`
- holdout can later influence a Stage 3 `final_score`
- Stage 5 performs a different ML/meta blend with separate weights and a different request builder

This makes the current system usable, but it also makes the naming overlap easy to misunderstand.

## Current Code Surface

- `crates/app_core/src/meta_select.rs`
  - `MetaScoreWeights`
  - `MetaSelectionRequest.weights`
  - `meta_score` calculation
  - `MetaCandidateAggregate.meta_score`
  - `MetaCandidateAggregate.final_score`
- `src-tauri/src/commands_meta.rs`
  - holdout-adjusted `final_score` assignment and later-stage sorting
- `crates/app_core/src/ml_confidence.rs`
  - ML feature vectors that already consume `meta_score`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
  - workbook/report output for `meta_score` and `final_score`
- `src-tauri/src/bin/ta_workflow_regression.rs`
  - workflow regression reference path using `MetaScoreWeights`
- `src-tauri/src/bin/ta_refactor_snapshots.rs`
  - snapshot parity path using `MetaScoreWeights`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/components/MetaWorkspacePanel.vue`
  - separate Stage 5 weighting surface that must not be confused with Stage 3 weights

## Current Verification Surface

- tests in `crates/app_core/src/meta_select.rs`
- Stage 4 / Stage 5 tests in `src-tauri/src/commands_meta.rs`
- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/bin/ta_refactor_snapshots.rs`
- report tests in `src-tauri/src/commands_reports_meta_xlsx.rs`

## Strengths Already Present

- Stage 3 weights are already centralized in one backend type
- `meta_score` is computed from a concrete formula instead of ad hoc ranking branches
- downstream reports already expose `meta_score` and `final_score`
- workflow-regression and refactor-snapshot paths already exist to catch semantic drift
- Stage 5 weights are separated into a different builder rather than silently reusing Stage 3 fields

## Weaknesses / Failure Modes

- the Stage 3 weight vector is mostly hidden from the user-facing surface, so changes can have large effects with little visibility
- `final_score` can mean different things in different stages if docs do not distinguish them carefully
- Stage 3 weights are not automatically normalized, so extreme values can dominate unexpectedly
- Stage 3 score changes cascade into:
  - grouped ranking
  - holdout-adjusted ranking
  - ML confidence features
  - saved reports
- reports currently show resulting scores more clearly than they show the weight configuration that produced them

## Regressions To Avoid

- confusing Stage 3 weight changes with Stage 5 blend-weight changes
- changing Stage 3 defaults without snapshot/regression updates
- renaming score fields in ways that obscure Stage 3 versus Stage 5 ownership
- weakening artifact/report traceability for `meta_score` and `final_score`
- introducing silent normalization or reweighting without documenting the behavior change
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn Model Evaluation Guide](https://scikit-learn.org/stable/modules/model_evaluation.html)
  - relevant because Stage 3 score construction is a metric-composition problem and should stay explicit about what each component contributes
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because Stage 3 scores already feed later ML-confidence logic, so score semantics and comparability matter downstream
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because these weight-bearing requests and artifacts behave like versioned contracts
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because stage naming and score ownership cross a frontend/backend boundary and need explicit contract discipline
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document Stage 3 weighting as its own contract, separate from Stage 5 blend weights.
2. Record every current default weight and the exact Stage 3 formula in one implementation doc.
3. Distinguish clearly between:
   - `meta_score`
   - holdout-adjusted Stage 3 `final_score`
   - Stage 5 ranking/blend scores
4. Document downstream consumers so future weight edits are treated as behavior changes, not local refactors.
5. Improve artifact/report traceability by documenting where the current system exposes scores and where it does not yet expose the originating weight vector strongly enough.
6. Keep regression and snapshot tooling in scope for any future weight change.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document all current `MetaScoreWeights` fields and defaults.
- [ ] Document the exact Stage 3 `meta_score` formula.
- [ ] Document how holdout later contributes to Stage 3 `final_score`.
- [ ] Document the difference between Stage 3 weights and Stage 5 blend weights.
- [ ] Document downstream consumers of `meta_score`, including ML confidence features.
- [ ] Document report/artifact surfaces that expose `meta_score` and `final_score`.
- [ ] Identify where current artifact/report outputs do not expose the originating weight vector clearly enough.
- [ ] Add or maintain regression coverage for weight-sensitive behavior.
- [ ] Add or maintain snapshot coverage for score-field compatibility.
- [ ] Preserve naming clarity between Stage 3 and Stage 5 score fields.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run targeted tests in `crates/app_core/src/meta_select.rs`
- run targeted Stage 4 / Stage 5 tests in `src-tauri/src/commands_meta.rs`
- run `npm --prefix web run test -- src/services/metaStage4RequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run the regression/snapshot bins that currently use `MetaScoreWeights`
- inspect one Stage 3 artifact and one meta workbook to confirm `meta_score` and `final_score` still round-trip clearly
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current default weights until there is a deliberate scoring-policy change
- if Stage 3 weights ever become user-editable, add that surface without breaking current artifact and replay shapes
- keep Stage 3 and Stage 5 score naming distinct even if UX copy is later simplified
- prefer additive report metadata over changing the meaning of existing score fields
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/23_meta_grouping_clustering_2026-04-09.md`
- `REPO_Improvement/08_ml/25_meta_evidence_flags_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
