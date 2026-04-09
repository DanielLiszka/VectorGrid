# Run Artifact Schema Plan

Last updated: 2026-04-09

## Purpose

Define the canonical non-options run-artifact schema so save/list/load/hydrate/replay behavior stays stable across the backend and the frontend history/workflow layers.

## Scope Boundaries

In scope:

- top-level run-artifact JSON envelope
- summary/detail/hydration payload derivation
- required identity, schema, fingerprint, and dataset-meta fields
- legacy artifact-kind compatibility
- report-path and ML/meta attachments inside the saved artifact

Out of scope:

- options artifacts
- workbook sheet/chart content
- directory/path rules beyond the schema fields that carry paths

## Current User-Facing Behavior

Users can save runs, validations, meta outputs, and ML-related flows as artifacts, reopen them from history, hydrate workflow state from them, and replay details without rerunning the original workflow. Current behavior already depends on the saved artifact carrying:

- a recognized kind such as `vectorgrid_run_artifact`, with legacy `vectorbt_run_artifact` acceptance still preserved
- `config_fingerprint` and optional version information
- `report_xlsx_path` for later reopen flows
- normalized `dataset_meta` fields including timeframe and resample identity
- enough `run`, `validation`, `meta`, and `ml_signal` payload data for history/detail/hydration surfaces

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
  - owns artifact-kind recognition, dataset-meta normalization, config-fingerprint computation, save/list/load/detail/hydration logic, and legacy compatibility behavior
- `src-tauri/src/commands_artifacts.rs`
  - exposes the Tauri async wrappers for save/list/load/detail/hydration/delete/tag operations
- `web/src/services/historyService.ts`
  - consumes artifact summaries and detail payloads, normalizes saved fields, and drives reopen/delete flows
- `web/src/services/workflowArtifactsSync.ts`
  - hydrates current workflow state from the latest saved artifact
- `web/src/stores/workflowArtifactsStore.ts`
  - stores normalized artifact identity, report path, dataset metadata, and run/validation state in the frontend
- `web/src/services/validationService.ts`
  - persists current validation runs as `vectorgrid_run_artifact` values with `config_fingerprint`, `report_xlsx_path`, and `dataset_meta`

## Current Verification Surface

- unit tests in `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- validation-service tests that assert saved artifact contents

## Strengths Already Present

- the backend already centralizes artifact normalization instead of duplicating schema shaping in every command
- config-fingerprint computation already ignores some unstable fields and encodes a compatibility posture
- dataset metadata already receives normalization/backfill logic instead of being treated as opaque free-form JSON
- the frontend already distinguishes summary, detail, and hydration views rather than assuming one payload fits every screen

## Weaknesses / Failure Modes

- the saved artifact shape is rich enough that compatibility currently lives mostly in code and tests, not in one explicit schema contract
- legacy-kind support and current-kind support can drift if aliases are not written down
- optional attachments such as `report_xlsx_path`, `dataset_meta`, and ML/meta subtrees are easy to treat as incidental even though UI flows depend on them
- summary/detail/hydration derivation rules are subtle enough that a field can disappear from one view while still existing in the raw artifact

## Regressions To Avoid

- breaking old saved artifacts without an explicit dual-read or migration plan
- changing artifact kind names or top-level field names without updating both backend and frontend consumers
- dropping `config_fingerprint`, `report_xlsx_path`, or `dataset_meta` semantics relied on by replay/hydration flows
- making schema hardening so strict that partially older but still usable saved artifacts no longer load
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the artifact is effectively a versioned contract with required, optional, and nullable fields.
- [JSON Schema 2020-12 release notes](https://json-schema.org/draft/2020-12/release-notes) are relevant for thinking about additive schema evolution and compatibility rather than ad-hoc shape drift.
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259) is relevant because the artifact is persisted JSON and null-versus-absent behavior matters for compatibility.
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785) is relevant because the repo already computes config fingerprints from canonicalized artifact content, so schema evolution must respect deterministic identity.
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Write the artifact as a canonical envelope with explicit required top-level fields and clearly scoped optional attachments.
2. Document legacy-kind alias support and the current compatibility stance for older saved artifacts.
3. Document how summary, detail, and hydration payloads derive from the raw artifact and which fields each view must preserve.
4. Make `config_fingerprint`, `config_fingerprint_version`, `dataset_meta`, and `report_xlsx_path` first-class schema concepts rather than incidental fields.
5. Tie schema guarantees directly to the existing test surfaces that already enforce parts of the contract.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Enumerate the required top-level run-artifact fields and their meaning.
- [ ] Document recognized artifact kinds and legacy alias support.
- [ ] Document the `run`, `validation`, `meta`, and `ml_signal` subtrees currently expected by consumers.
- [ ] Document `config_fingerprint` and version behavior, including fallback computation.
- [ ] Document normalized `dataset_meta` expectations, especially timeframe and resample identity fields.
- [ ] Document `report_xlsx_path` semantics and its relationship to reopen flows.
- [ ] Document summary/detail/hydration derivation rules and minimum payload guarantees.
- [ ] Add or tighten tests for missing fields, legacy fields, and compatibility fallback behavior if the schema changes.
- [ ] Verify saved artifacts still load, hydrate, and replay after schema hardening.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted unit tests in `src-tauri/src/run_artifacts.rs`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/services/workflowArtifactsSync.test.ts`
- run targeted validation-service tests that assert persisted artifact contents
- manually save, reload, hydrate, and replay one representative non-options artifact
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve existing non-options artifact kinds and top-level field names first
- prefer additive schema expansion over breaking renames
- if a breaking schema change is unavoidable, add a dual-read path or explicit migration before deleting compatibility code
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/24_tauri_run_artifact_paths_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
