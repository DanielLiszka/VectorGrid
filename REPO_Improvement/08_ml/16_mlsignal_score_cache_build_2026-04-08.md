# ML Signal Score Cache Build Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for ML Signal score-cache generation so imported score CSVs, cache manifests, coverage summaries, and compatibility checks remain explicit and reproducible within the current non-options workflow.

## Scope Boundaries

In scope:

- score-cache build request/result contract
- input-score CSV to cache/manifest conversion
- coverage and duplicate summaries
- compatibility expectations for later validation stages

Out of scope:

- dataset export details
- policy validation or policy sweep execution details
- model-bundle training internals

## Current User-Facing Behavior

Users choose a model-score CSV input path and a score-cache output path, optionally choose a model bundle, and then generate an ML score cache. The backend matches input rows to loaded candles by timestamp, accepts timestamp plus at least one of `p_long` / `p_short` / `raw_long` / `raw_short`, infers score-space and score-scope metadata, writes a JSON cache plus manifest, and returns coverage-style summary fields. The frontend reports matched-row counts and score-coverage details, persists a run artifact, and carries the produced cache path forward into validation and sweep stages.

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- ML workspace component tests
- tests in `crates/app_core/src/ml_signal.rs`
- artifact/history tests that cover saved ML run payloads

## Strengths Already Present

- score-cache generation is already a dedicated command/stage
- cache build already returns matched-row and coverage-style summary information
- cache kind/version/schema checks already exist on the backend for later use
- artifact persistence already ties cache generation into the rest of the workflow

## Weaknesses / Failure Modes

- input CSV assumptions and score-cache manifest expectations are not yet owned by one explicit contract doc
- score space, score scope, duplicate timestamps, and coverage semantics are rich but easy to under-document
- cache compatibility with later validation depends on schema/version/data-id fields that users do not directly see
- raw-margin-only caches can be built successfully but later fail probability-threshold validation or sweep if required probability columns are absent
- frontend request payloads can include extra convenience fields that are not part of the strict Rust request shape
- stage boundaries between cache build and later validation are easy to blur

## Regressions To Avoid

- breaking existing score-cache generation from current input CSV layouts
- weakening cache kind/version/schema checks
- hiding coverage or duplicate-timestamp warnings
- losing artifact linkage between cache generation and later ML Signal stages
- turning score-cache generation into a generic “model import” feature without documenting the current contract
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CatBoost prediction docs](https://catboost.ai/docs/en/features/prediction) is relevant because upstream model tooling can output probabilities or raw margins that later imply different runtime restrictions.
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259) is relevant because the produced cache and manifest are JSON contracts.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for the cache JSON and manifest payload contracts.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because cache generation crosses a command boundary and exposes structured result fields to the UI.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the real Rust request contract, including required fields and which extra frontend fields are convenience-only.
2. Document accepted input-column aliases and the rules for inferring `score_space` and `score_scope`.
3. Document returned result fields and the meaning of coverage, matched rows, skipped rows, bars scored, and duplicate timestamps.
4. Make the produced cache and manifest contract explicit so later validation stages are not relying on hidden assumptions.
5. Document the runtime restriction that raw-margin-only caches can build successfully but are insufficient for probability-threshold validation/sweep.
6. Keep artifact persistence and UI stage summaries aligned with the backend result contract.
7. Expand tests around coverage, schema, malformed input, and runtime restriction behavior.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the real score-cache build request fields used by Rust.
- [ ] Note any extra frontend-only or convenience fields that are not part of the backend request type.
- [ ] Document returned score-cache result and manifest fields.
- [ ] Document accepted input-column aliases and score-scope inference rules.
- [ ] Document coverage, matched/skipped rows, and duplicate-timestamp semantics.
- [ ] Clarify model-bundle context and when it is optional vs meaningful.
- [ ] Document the raw-margin-only success path vs later validation/sweep restrictions.
- [ ] Keep artifact linkage from score-cache build into later ML stages.
- [ ] Add or maintain tests for malformed or partially compatible input score CSVs.
- [ ] Preserve backend kind/version/schema checks.
- [ ] Keep UI status and metadata summaries aligned with backend fields.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run ML workspace component tests for stage-2 behavior
- run targeted Rust score-cache tests in `ta_app_core`
- manually verify one score-cache build if manifest or summary fields change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current cache kind/version/schema expectations until a versioned migration exists
- keep score-cache generation bounded to the current ML Signal workflow
- if cache manifests expand, preserve current fields until downstream consumers are updated
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/07_strats/08_strat_ml_signal_2026-04-08.md`
- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
