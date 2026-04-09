# ML Signal Score CSV Ingest Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for ML Signal score-CSV ingest so score-cache generation has a clear, validated, and user-explainable input contract.

## Scope Boundaries

In scope:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- score CSV path selection, score CSV parsing rules, score-cache generation preconditions, and related error/report semantics

Out of scope:

- model training output generation outside the CSV ingest contract
- score-cache consumers after the cache is already built
- options-related ingest paths

## Current User-Facing Behavior

The ML Signal workspace already lets users choose a model-score CSV input and build a score cache from it. Under the hood, the ingest contract is stricter than the UI might imply:

- a timestamp column is required
- at least one of `p_long`, `p_short`, `raw_long`, or `raw_short` must exist
- row-level parse errors are explicit
- score-cache generation has schema/version/data-id checks downstream

This is good, but it needs to be documented and surfaced more clearly because ingest failures are likely to feel opaque otherwise.

## Current Code Surface

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `web/src/services/mlSignalService.test.ts`

## Current Verification Surface

- unit tests in `crates/app_core/src/ml_signal.rs` around score CSV parsing and score-cache generation
- `mlSignalService.test.ts`
- `MlSignalWorkspacePanel.test.ts`

## Strengths Already Present

- ingest rules are already enforced in the backend instead of being loosely assumed
- row-level error messages include column-specific context
- the UI already separates score input path from score cache output path
- score-cache generation is already treated as its own step rather than a hidden follow-on

## Weaknesses / Failure Modes

- the ingest contract is richer than the UI currently explains
- CSV field requirements and downstream cache-compatibility rules live in different layers
- parse errors can still feel technical if the user does not know the accepted schema
- it is easy to treat “choose score CSV” as a generic file-picker step when it is really a schema-bound ingest contract

## Regressions To Avoid

- loosening ingest validation until malformed CSVs reach later stages
- changing accepted columns without synchronized UI/help/test updates
- weakening row-level error clarity
- allowing ingest success while silently degrading the downstream cache contract
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the input is a CSV contract and field/row expectations should remain explicit.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the accepted score-row structure effectively behaves like a tabular schema contract.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because the resulting score-cache metadata is serialized and attached to artifacts downstream.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because score CSV paths and cache-generation requests cross the frontend/backend boundary.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because input/output paths and resulting artifact status are shared runtime state.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Treat score-CSV ingest as a schema-bound import contract.
2. Document accepted columns, required timestamp handling, and downstream implications.
3. Tighten UI copy so users know what kind of CSV is expected before they run score-cache generation.
4. Add tests that keep parse errors, required-column behavior, and downstream cache compatibility explicit.
5. Preserve strict backend validation even if the UI gets friendlier.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the accepted score CSV column contract and required timestamp behavior.
- [ ] Add or tighten tests for missing columns, bad timestamps, and mixed probability/raw inputs.
- [ ] Audit workspace copy around the score CSV input field and error messages.
- [ ] Confirm cache-generation success still implies downstream cache compatibility checks remain meaningful.
- [ ] Keep row-level error messages specific and user-actionable.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `crates/app_core/src/ml_signal.rs` tests covering score CSV parsing and score-cache generation
- run `mlSignalService.test.ts`
- run `MlSignalWorkspacePanel.test.ts`
- perform a bounded manual invalid-CSV and valid-CSV ingest check if request wording or path handling changes materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- keep strict ingest validation in the backend
- prefer clearer UI/help text over weakening the accepted schema
- if a cleanup reduces parse specificity, restore the more explicit validation behavior
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/02_ml_config_paths_2026-04-09.md`
- `REPO_Improvement/08_ml/05_ml_policy_sweep_2026-04-09.md`
- `REPO_Improvement/08_ml/16_mlsignal_score_cache_build_2026-04-08.md`
- `REPO_Improvement/08_ml/12_mlsignal_feature_schema_2026-04-09.md`
