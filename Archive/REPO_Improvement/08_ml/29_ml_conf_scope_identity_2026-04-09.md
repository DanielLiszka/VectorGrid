# ML Confidence Scope Identity Plan

Last updated: 2026-04-09

## Purpose

Define the current ML-confidence scope-identity contract so model files, status reporting, saved Stage 5 artifacts, and meta-artifact filtering all agree on what constitutes the active scope.

## Scope Boundaries

In scope:

- `MlConfidenceScope`
- frontend scope building and fallback behavior
- backend scope normalization/validation
- model-path identity derived from scope
- saved artifact/report surfaces that expose scope
- scope mismatch behavior when scanning saved meta artifacts

Out of scope:

- label-threshold semantics
- training-row counting and row-family selection
- Stage 5 blend weights and bucket configuration beyond their dependence on a scoped model
- options workflows

## Current User-Facing Behavior

The Meta workspace shows ML-confidence status for the active strategy/timeframe scope and lets users refresh, train, or apply a scoped model. In current shipped behavior:

- scope is exactly a strategy kind plus timeframe, not a larger dataset identity object
- the frontend can derive scope from the latest meta result before falling back to raw store inputs
- missing or empty timeframe blocks ML-confidence refresh/train/apply with explicit status messaging
- model availability, health, trained-row counts, and label thresholds are all reported relative to the current scope
- saved Stage 5 artifacts retain strategy and timeframe so later replay and status surfaces can stay scoped
- scope mismatches against saved meta artifacts are skipped with warnings rather than silently merged

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - user-facing ML-confidence controls and status display
- `web/src/services/metaMlScopeBuilder.ts`
  - frontend scope-object construction
- `web/src/services/metaService.ts`
  - current-scope resolution, model-status formatting, and scope-aware ML actions
- `crates/app_core/src/ml_confidence.rs`
  - canonical `MlConfidenceScope` type plus scoped filtering of saved meta artifacts
- `src-tauri/src/commands_meta.rs`
  - scope normalization, model-path derivation, model load/train/apply command handling, and scope-aware status building
- `src-tauri/src/run_artifacts.rs`
  - Stage 5 artifact summary fields carrying strategy and timeframe

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- tests embedded in `crates/app_core/src/ml_confidence.rs`
- tests embedded in `src-tauri/src/commands_meta.rs`
- artifact replay/detail tests touching `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- scope is currently minimal and explicit instead of being inferred from many loosely related fields
- the backend trims and normalizes scope before model-path use
- model identity on disk is already deterministic from scope
- artifact filtering honors scope instead of training across mixed strategy/timeframe artifacts
- frontend status text already makes missing-timeframe and no-matching-artifact failures visible

## Weaknesses / Failure Modes

- the frontend can derive scope from the latest meta result or from store inputs, so scope provenance is easy to under-document
- current scope is intentionally smaller than full dataset identity, which is useful but easy to over-assume
- Stage 4/Stage 5 naming drift can obscure that the same scoped model identity is serving the current Stage 5 surface
- scope mismatch errors can look like data scarcity problems unless the scope filter is documented clearly
- if model-path naming changes, saved expectations around where a scoped model lives can break silently

## Regressions To Avoid

- allowing empty strategy or timeframe values to pass scope normalization
- broadening scope implicitly without updating model-path identity, filtering, and status messaging together
- conflating current scope with dataset path or artifact family identity when the shipped contract is narrower
- breaking saved model lookup by changing scope-derived file naming without an explicit migration path
- hiding scope mismatch warnings in ways that make artifact scarcity look like model-training scarcity
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MLflow Model Signatures and Input Examples](https://mlflow.org/docs/3.0.1/model/signatures)
  - relevant because model scope and compatibility should behave like a documented contract, not an implicit convention
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because `MlConfidenceScope` crosses persisted JSON surfaces and should stay explicit and shape-stable
- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because scoped model reporting is only useful if the model identity and applicability boundaries are stated clearly
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because scope objects cross the command boundary and should remain stable as a frontend/backend contract
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document scope as the canonical identity key for ML-confidence rather than as a convenience parameter.
2. Record frontend scope provenance explicitly:
   - last loaded meta result first
   - store fallback second
3. Tie scope normalization, model-path identity, and artifact filtering together so future changes are treated as one contract edit.
4. Make current non-goals explicit:
   - scope is not yet full dataset identity
   - scope is not a generic report family selector
5. Clarify how saved Stage 5 artifacts expose scope and how that differs from model-file identity.
6. Keep Stage 4/Stage 5 naming drift visible so the documentation does not accidentally describe two separate scope systems.
7. Preserve explicit status messaging for missing timeframe and no-matching-artifact conditions.
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the exact current fields in `MlConfidenceScope`.
- [x] Document frontend scope-construction behavior and fallback order.
- [x] Document backend scope normalization rules and hard-failure conditions.
- [x] Document the current scope-derived model-path identity.
- [x] Document how scope filters saved meta artifacts during training/status scans.
- [x] Document how Stage 5 artifact summaries expose scope without becoming the model-identity source of truth.
- [x] Document scope mismatch warning behavior separately from “no data” behavior.
- [x] Preserve tests for missing timeframe, empty scope fields, and scope-filtered artifact scans.
- [x] Keep scope docs aligned with current Stage 4/Stage 5 naming reality.
- [x] Treat any future expansion of scope fields as a compatibility change, not a local refactor.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted ML-confidence tests in `crates/app_core/src/ml_confidence.rs`
- run targeted ML-confidence status tests in `src-tauri/src/commands_meta.rs`
- inspect one saved Stage 5 artifact summary and one saved model path for the same scope
- manually verify missing-timeframe status behavior in the Meta panel
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current scope fields and current file-naming rules unless there is an explicit migration plan for saved models
- if scope later grows beyond strategy/timeframe, add fields deliberately and version the affected persisted surfaces
- keep current status messaging conservative so scope mismatch does not masquerade as ordinary lack of data
- prefer additive artifact metadata over redefining what current scope means
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by surfacing scope identity directly in the ML panel rather than leaving it implicit in the service/backend contract.
- `MetaWorkspacePanel.vue` now exposes:
  - `strategy_kind / timeframe` as the current ML scope
  - the scope-derived model file naming pattern `ml_confidence_<strategy>_<timeframe>.json`
  - an explicit note that scope resolves from the latest meta result first and then falls back to current Meta inputs
- The panel copy also now states that only Stage 5 and Policy Eval persist durable artifacts, which keeps Stage 5 scope exposure distinct from model-file identity.
- `REPO_MAP.md` was updated so contributors can find the ML scope-identity contract quickly.
- Stable fixture / replay payload:
  - `strategy_kind = CompositeMo`
  - `timeframe = 1m`
  - model file `ml_confidence_CompositeMo_1m.json`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts`
  - `cargo test -p ta_desktop_demo_app commands_meta --lib`
  - `cargo test -p ta_app_core ml_confidence --lib`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/10_meta_ml_confidence_reports_2026-04-09.md`
- `REPO_Improvement/08_ml/30_ml_conf_training_rows_2026-04-09.md`
- `REPO_Improvement/08_ml/31_ml_conf_label_config_2026-04-09.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/06_stage5_artifact_replay_2026-04-09.md`
