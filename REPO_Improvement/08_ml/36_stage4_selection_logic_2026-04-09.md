# Stage 4 Selection Logic Plan

Last updated: 2026-04-09

## Purpose

Define the current Stage 4 selection-logic contract so source-meta choice, training-set exclusion, ML scoring, blended ranking, fallback behavior, and decision-bucket assignment remain explicit despite the current Stage 4/Stage 5 naming overlap.

## Scope Boundaries

In scope:

- source meta artifact resolution for the Stage 5 finalizer
- exclusion of the source artifact from auto-training history
- ML scoring of source groups
- blended rank construction
- `min_ml_confidence` filtering
- fallback-to-meta behavior
- decision-bucket assignment

Out of scope:

- Stage 4 request-field normalization details already covered by the request contract
- Stage 5 artifact replay and saved-artifact UX beyond what selection logic writes
- offline policy-evaluation comparisons
- options workflows

## Current User-Facing Behavior

The user-facing flow is labeled and invoked as Stage 5, but the actual selection logic still lives in the Stage-4-named backend path. Current shipped behavior is:

- the system resolves a source meta artifact for the current scope
- it loads or auto-trains an ML-confidence model for that scope
- it scores source groups with ML confidence and ML uncertainty
- it filters and re-ranks groups using a blended score
- it can fall back to pure meta-score ranking when the ML thresholds leave no selected groups
- it assigns `accept`, `review`, or `reject` buckets after ranking

This means the ranking contract and the bucketing contract are related but not identical.

## Current Code Surface

- `web/src/services/metaStage4RequestBuilder.ts`
  - frontend request shaping for the Stage 5 finalizer
- `web/src/services/metaService.ts`
  - Stage 5 invocation, preview rendering, and Stage 4/Stage 5 alias handling
- `web/src/components/MetaWorkspacePanel.vue`
  - user-facing Stage 5 controls and result preview
- `src-tauri/src/commands_meta.rs`
  - source-meta resolution
  - source-artifact exclusion from auto-train history
  - ML scoring
  - blended rank computation
  - fallback handling
  - decision-bucket labeling
- `crates/app_core/src/ml_confidence.rs`
  - feature extraction and probability prediction used by selection
- `src-tauri/src/run_artifacts.rs`
  - Stage 5 summary extraction from the saved selection result

## Current Verification Surface

- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- tests embedded in `src-tauri/src/commands_meta.rs`
- artifact summary tests in `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- source-artifact leakage is already guarded against by excluding the source meta artifact from auto-training history
- selection logic is centralized in one backend path rather than partially duplicated in the frontend
- weights and thresholds are normalized before use
- fallback-to-meta is explicit and warning-bearing instead of silently mixing behaviors
- decision buckets are computed after the selection/ranking pass, making the pipeline legible

## Weaknesses / Failure Modes

- the user-facing Stage 5 name and backend Stage 4 implementation name create persistent documentation risk
- source resolution, model loading/training, ranking, fallback, and bucketing all happen in one large flow, making partial summaries easy to oversimplify
- non-finite or weak ML outputs can degrade into fallback ranking and warnings that may be missed if docs only describe the happy path
- ranking weights are normalized in the backend, so UI-entered numbers are not interpreted literally
- bucket assignment can look like the selection criterion even though selected groups are already chosen before bucket labeling

## Regressions To Avoid

- removing source-artifact exclusion from the auto-training set
- changing the order of ranking, fallback, and bucket assignment without documenting the behavior change
- weakening current fallback warnings
- treating decision buckets as the same thing as shortlist ranking
- obscuring the current Stage 4/Stage 5 alias relationship in docs or status surfaces
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn Model Selection and Evaluation](https://scikit-learn.org/stable/model_selection.html)
  - relevant because the Stage 5 finalizer is a rank-and-threshold evaluation layer, not just a UI list transformer
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because `min_ml_confidence` filtering and bucket thresholds assume stable confidence semantics
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because selected outputs and fallback reasons should remain explainable in model-driven shortlist logic
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because all ranking and bucketing semantics ultimately cross a structured frontend/backend boundary
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the selection pipeline in strict order:
   - resolve source meta
   - resolve/load/train model
   - score groups
   - rank groups
   - apply fallback if needed
   - assign decision buckets
2. Make source-artifact exclusion explicit because it is a critical anti-leakage invariant.
3. Separate ranking semantics from decision-bucket semantics so later contributors do not conflate them.
4. Record current backend normalization behavior for weights and thresholds in the logic doc, not only in the request doc.
5. Preserve current fallback explanation behavior and ensure it remains visible in saved results and previews.
6. Keep Stage 4/Stage 5 aliasing explicit whenever describing the owner of this logic.
7. Tie every major stage of the flow to its current code owner and tests.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document source meta artifact resolution and current path restrictions.
- [ ] Document exclusion of the source artifact from auto-training history.
- [ ] Document ML scoring of source groups and dependence on `MlConfidenceGroupFeatures`.
- [ ] Document weight normalization and blended rank construction.
- [ ] Document `min_ml_confidence` filtering and empty-selection behavior.
- [ ] Document fallback-to-meta semantics and warning generation.
- [ ] Document decision-bucket assignment and its thresholds.
- [ ] Preserve tests for source exclusion, unhealthy-model gating, and auto-train fallback behavior.
- [ ] Preserve preview/status rendering tests that reflect fallback reasons and bucket counts.
- [ ] Keep Stage 4/Stage 5 naming drift explicit in the logic doc until the implementation is renamed.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaStage4RequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted Stage 4/Stage 5 tests in `src-tauri/src/commands_meta.rs`
- inspect one saved Stage 5 artifact with no fallback and one with fallback-to-meta triggered
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current ranking/fallback order unless there is a deliberate migration plan for saved semantics
- if the Stage 4 names are later retired, retain compatibility notes in docs until saved artifacts and API wrappers are fully migrated
- prefer additive result metadata over changing the meaning of existing ranking and bucket fields
- keep anti-leakage source exclusion as a non-negotiable invariant during any refactor
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/37_stage5_execution_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/38_ml_policy_eval_contract_2026-04-09.md`
- `REPO_Improvement/08_ml/34_ml_conf_apply_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
