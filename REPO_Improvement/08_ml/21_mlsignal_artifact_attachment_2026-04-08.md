# ML Signal Artifact Attachment Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the ML Signal artifact-attachment contract so non-options ML artifacts remain stage-aware, replayable, and truthfully attached across export, score-cache, validation, and policy-sweep flows.

## Scope Boundaries

In scope:

- `ml_signal` artifact sub-payload structure
- stage ownership and carry-forward behavior
- attachment persistence across workflow/history/replay flows
- compatibility checks for dataset, timeframe, and score-cache context

Out of scope:

- options ML surfaces
- redesigning model training logic itself
- inventing new ML artifact families unrelated to the current workflow

## Current User-Facing Behavior

ML Signal flows can persist artifacts during dataset export, score-cache creation, validation, and policy sweep work. The saved attachment helps replay and continue work later, but the payload is built incrementally and can inherit prior fields, which means stage semantics are not always explicit.

## Current Code Surface

- `web/src/services/mlSignalService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/validationService.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `web/src/services/analysisService.ts`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- ML workspace component tests

## Strengths Already Present

- ML artifact persistence already exists across multiple stages
- workflow artifact normalization already understands ML-related fields
- validation/report flows already carry ML attachment data forward when available
- backend commands already enforce some compatibility checks for loaded data and score caches

## Weaknesses / Failure Modes

- stage is inferred partly by heuristics and partial prior state
- carry-forward merging can preserve stale fields unintentionally
- Vue state can advance even when artifact save fails
- replay dispatch depends on stage-shape assumptions instead of one explicit attachment contract
- compatibility checks lean heavily on path/timeframe or `data_id` matches rather than a richer dataset identity model

## Regressions To Avoid

- breaking replay of existing ML Signal artifacts
- dropping stage-specific fields that current flows still rely on
- over-tightening compatibility in a way that strands valid saved work
- hiding persistence failures that users need to understand
- making standard non-ML artifact flows depend on ML-specific assumptions
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn cross-validation guidance](https://scikit-learn.org/1.6/modules/cross_validation.html) is relevant because ML artifact stages must preserve the temporal/data-split assumptions used to generate them.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces keeping backend compatibility and save semantics explicit where frontend workflow state can drift.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for formalizing the `ml_signal` sub-payload by stage.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Define the `ml_signal` sub-payload by stage:
   - dataset export
   - score-cache build
   - validation
   - policy sweep
2. Mark each field as one of:
   - stage-owned
   - carry-forward
   - optional diagnostic metadata
3. Document every consumer of the attachment in workflow state, history replay, validation/report export, and analysis surfaces.
4. Tighten compatibility rules for dataset, timeframe, score cache, and bundle/schema assumptions.
5. Add end-to-end tests that prove attachment continuity across the full stage sequence.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the stage-specific `ml_signal` payload contract.
- [ ] Mark carry-forward vs stage-owned vs optional fields.
- [ ] Document all replay and hydration consumers.
- [ ] Define dataset/timeframe/score-cache compatibility rules.
- [ ] Surface or log artifact-save failures more clearly.
- [ ] Add end-to-end tests for export -> score cache -> validation -> policy sweep continuity.
- [ ] Add tests for stale-field clearing when stage transitions invalidate prior data.
- [ ] Preserve replay compatibility for existing ML Signal artifacts.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run `npm test -- src/services/historyService.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run `npm test -- src/stores/workflowArtifactsStore.test.ts`
- run targeted ML workspace/service tests if payload semantics change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- keep current ML artifact replay working while the attachment contract is tightened
- prefer additive stage metadata over removing current fields abruptly
- if attachment failures remain non-fatal, make that degradation visible rather than silent
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/04_ml_validation_surface_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/06_artifacts/05_meta_artifact_replay_2026-04-09.md`
