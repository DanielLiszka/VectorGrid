# ML Signal Split Plan Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the ML Signal split-plan contract so train/calibration/test windows, gap/step settings, and optional split-manifest generation remain explicit and reproducible.

## Scope Boundaries

In scope:

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- optional split-plan UI fields and generated split-manifest behavior

Out of scope:

- changing the downstream modeling approach
- generic validation split logic outside the ML dataset export flow
- options-related split workflows

## Current User-Facing Behavior

ML dataset export already supports an optional split plan:

- users can leave split fields blank and export a plain dataset
- or provide train/calibration/test bars plus gap and step bars
- the backend validates positive window and step sizes
- a split manifest sidecar can be written when split planning is active

This is powerful because it turns the export step into a reproducible dataset-partitioning stage, not just a flat CSV dump.

## Current Code Surface

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.test.ts`

## Current Verification Surface

- unit tests in `crates/app_core/src/ml_signal.rs`
- `MlSignalWorkspacePanel.test.ts`
- `mlSignalService.test.ts`

## Strengths Already Present

- split-plan parameters are already explicit in the UI
- the backend validates obviously invalid split settings
- split-manifest generation already exists as a concrete artifact
- the frontend already treats split planning as optional rather than always-on

## Weaknesses / Failure Modes

- split semantics are powerful enough that vague wording can lead to misuse
- the split plan exists across UI, service request assembly, backend validation, and manifest generation
- it is easy for users to think split settings are “just metadata” when they materially define the exported dataset partitions
- gap/step semantics are harder to infer than train/calibration/test lengths

## Regressions To Avoid

- changing split-plan field meaning without synchronizing UI, service, and manifest behavior
- writing split manifests inconsistently across export flows
- silently ignoring invalid split settings
- making split-plan usage more implicit or magical
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because the split plan is fundamentally a time-ordered partitioning problem and should preserve temporal discipline.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because the split manifest should be treated as a structured artifact contract.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because split-manifest and export payload details are serialized and moved through JSON-heavy layers.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because split settings cross the frontend/backend boundary as part of the export request.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because the split plan ultimately partitions a CSV-backed dataset export.

## Proposed Improvement Approach

1. Treat the split plan as a reproducible dataset-partition contract.
2. Document every split field and its effect:
   - train bars
   - calibration bars
   - test bars
   - gap bars
   - step bars
3. Tighten the distinction between “no split plan” and “split plan with manifest.”
4. Add tests for invalid combinations, manifest generation, and reopen/display behavior where applicable.
5. Keep split planning explicit and optional rather than auto-derived.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the split-plan field contract and optional-manifest behavior.
- [ ] Add or tighten tests for invalid split settings and manifest generation.
- [ ] Audit UI copy for gap/step semantics so they are understandable without code knowledge.
- [ ] Confirm the service and backend agree on when a split plan is considered present.
- [ ] Keep split-plan information visible in saved ML artifact summaries where useful.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `crates/app_core/src/ml_signal.rs` tests covering split-plan validation and export
- run `MlSignalWorkspacePanel.test.ts`
- run `mlSignalService.test.ts`
- manually verify a bounded export with and without split settings if contract wording or request assembly changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve the current optional split-plan model
- prefer clearer validation and manifest semantics over more automation
- if a split-plan cleanup weakens reproducibility, keep the stricter more explicit current behavior
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/11_mlsignal_dataset_export_2026-04-08.md`
- `REPO_Improvement/08_ml/12_mlsignal_feature_schema_2026-04-09.md`
- `REPO_Improvement/08_ml/14_mlsignal_export_manifests_2026-04-09.md`
- `REPO_Improvement/08_ml/02_ml_config_paths_2026-04-09.md`
