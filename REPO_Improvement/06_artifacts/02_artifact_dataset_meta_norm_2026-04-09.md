# Artifact Dataset Metadata Normalization Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to artifact dataset-metadata normalization so saved runs, validation replay, history detail, hydration, and report builders all consume one consistent non-options dataset metadata contract.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- `web/src/utils/core/datasetMeta.ts`
- report builders and history/detail consumers that depend on normalized `dataset_meta`
- campaign/meta producers that emit dataset metadata reused by artifacts

Out of scope:

- full artifact-library indexing
- options-specific dataset metadata
- raw CSV load/parsing rules

## Current User-Facing Behavior

Saved-run detail, hydration, and later report reuse already depend on normalized `dataset_meta`. History surfaces show dataset path, timeframe, and related provenance from artifact payloads even when the original dataset is not currently loaded. Report builders also reuse persisted dataset metadata fields such as `timeframe`, `base_timeframe`, `derived_timeframe`, `resample_rule`, `resample_anchor`, and related booleans instead of recomputing them from live state.

That means `dataset_meta` is already effectively part of the product contract, not just internal housekeeping.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `web/src/utils/core/datasetMeta.ts`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_campaign.rs`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `src-tauri/src/commands_reports.rs` tests
- `web/src/services/historyService.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`

## Strengths Already Present

- dataset metadata is already normalized on both backend and frontend paths
- schema versioning already exists
- resample identity fields are already treated as first-class data
- history, hydration, and reports already depend on the same broad metadata family
- there is already backend test coverage for normalization and fallback behavior

## Weaknesses / Failure Modes

- normalization and precedence rules are encoded across multiple files and producers
- `dataset_meta` can drift if save/detail/report consumers each infer slightly different defaults
- timeframe precedence across live state, artifact payload, and validation context is subtle
- campaign/meta producers can accidentally become shape outliers if not tied back to the same contract

## Regressions To Avoid

- dropping `csv_path` or changing its normalization rules
- changing timeframe precedence without updating detail, hydration, and report builders together
- breaking `resample_applied`, `resample_rule`, or `resample_anchor` semantics
- letting report builders consume a different `dataset_meta` shape than history or hydration
- widening the shape casually without documenting compatibility
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because `dataset_meta` already behaves like a structured schema shared across backend and frontend consumers.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because absent-versus-null metadata fields affect compatibility and fallback precedence.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because stable metadata structure supports repeatable artifact identity and diff behavior.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because normalized dataset metadata becomes shared workflow state once applied in the frontend.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because metadata normalization consequences are visible through store hydration and history/detail tests.

## Proposed Improvement Approach

1. Freeze one canonical non-options `dataset_meta` contract reused by save, detail, hydration, and reports.
2. Document fallback precedence for path, timeframe, and resample fields explicitly.
3. Keep frontend `buildDatasetMetaForArtifact()` aligned with backend normalization rather than evolving independently.
4. Tie campaign/meta-produced dataset metadata back to the same contract.
5. Expand tests around precedence, resample semantics, and cross-consumer consistency.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the canonical non-options `dataset_meta` fields and schema version.
- [ ] Document fallback precedence for `csv_path`, `timeframe`, `base_timeframe`, and `derived_timeframe`.
- [ ] Document `resample_applied`, `resample_rule`, and `resample_anchor` semantics.
- [ ] Document which producers are allowed to emit `dataset_meta` and how they stay aligned.
- [ ] Add or tighten tests for normalization precedence and report/history parity.
- [ ] Add or tighten tests for campaign/meta-produced dataset metadata compatibility.
- [ ] Add or tighten tests for hydration using persisted dataset metadata without live reload.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `run_artifacts.rs` tests
- run targeted `commands_reports.rs` tests
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/services/workflowArtifactsSync.test.ts`
- manually inspect one saved-run detail view and one reopened report path using persisted dataset metadata
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve the current schema version and additive-compatibility posture unless a migration is introduced
- prefer one normalized contract over multiple inferred shapes
- if a new metadata field is added, update backend normalizer, frontend builder, and report/detail consumers together
- keep options-specific metadata out of this non-options contract
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
