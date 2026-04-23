# Artifact Identity Workflow Model Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to artifact identity and workflow hydration semantics so non-options saved runs, report paths, dataset provenance, and replay flows use a clearly separated identity model instead of several partially overlapping ones.

## Scope Boundaries

In scope:

- config identity
- dataset identity
- hydration/sync identity
- workflow artifact store skip/reload rules

Out of scope:

- options artifact families
- redesigning report contents
- replacing saved-run persistence wholesale

## Current User-Facing Behavior

Users can save, reload, replay, and validate artifacts across optimization, validation, and ML-adjacent flows. The app usually succeeds because multiple identity clues are carried through, but those clues are not clearly separated into one model, which makes sync, replay, and update behavior harder to reason about.

## Current Code Surface

- `web/src/utils/core/artifactIdentity.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/validationService.ts`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `web/src/utils/core/artifactIdentity.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/services/historyService.test.ts`
- `src-tauri/src/run_artifacts.rs` tests

## Strengths Already Present

- config fingerprints already exist
- dataset metadata already carries size, modified-time, and content-oriented fields
- artifact save/detail/hydration flows already centralize a large part of the contract in Rust
- workflow artifact store already normalizes and deduplicates payload updates

## Weaknesses / Failure Modes

- identity is split across config fingerprint, dataset provenance, artifact path/timestamp, and workflow-store signatures
- TypeScript and Rust both participate in canonicalization/fingerprint logic
- sync skip logic can rely too heavily on path/timestamp heuristics
- validation/report reuse still leans on `csv_path + timeframe` in some paths
- field fallback behavior in Rust can hide which identity field was actually authoritative

## Regressions To Avoid

- breaking hydration of existing saved runs
- making tag/report-path updates invisible because sync skip logic is too coarse
- changing fingerprint behavior without cross-layer parity tests
- removing current dataset provenance fields before their replacements are proven
- making replay identity stricter in ways that strand existing artifacts
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant for reasoning about stable canonical JSON inputs to fingerprints.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting distinct artifact identity layers and payload authority.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because workflow hydration depends on a clear boundary between persisted backend data and frontend reactive state.
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Define three explicit identity layers:
   - config identity
   - dataset identity
   - hydration/sync identity
2. Document which fields are authoritative at save, list, detail, hydration, replay, and validation/report reuse time.
3. Decide whether the current custom canonical JSON approach is sufficient or whether it should be aligned more explicitly with a formal canonicalization model.
4. Tighten workflow-artifact sync rules so same-path/same-timestamp artifacts do not accidentally suppress meaningful content updates.
5. Add cross-layer parity tests between TypeScript and Rust fingerprint/canonicalization outputs.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define config identity, dataset identity, and hydration identity separately.
- [x] Document authoritative fields for save/list/detail/hydration/replay paths.
- [x] Audit where TypeScript and Rust duplicate canonicalization or fingerprint logic.
- [x] Define skip/reload rules for same-path and same-timestamp artifacts.
- [x] Clarify how validation/report reuse should key dataset identity.
- [x] Add parity tests for TypeScript vs Rust identity outputs.
- [x] Add tests for artifact updates that change tags or report paths without changing core timestamps.
- [x] Preserve backward compatibility for existing saved artifacts.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/utils/core/artifactIdentity.test.ts`
- run `npm test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm test -- src/services/workflowArtifactsSync.test.ts`
- run targeted history-service tests
- run targeted Rust run-artifact tests
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- keep existing artifact fields until new identity rules are proven and migration-safe
- if sync identity changes, provide an explicit compatibility path for older artifacts instead of hard invalidation
- treat stricter identity separation as additive clarity, not as a reason to reject existing saved work casually
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`

## Execution Notes

- Closed as an identity-model hardening slice after resolving the open sync-policy question in favor of reloading same-path artifacts when report-path, dataset-path, or tag metadata changed.
- The identity model now stays split between config identity (`artifactIdentity.ts` and Rust canonicalization/fingerprint helpers), dataset identity (`dataset_meta` and current CSV/timeframe hints), and hydration/sync identity (`workflowArtifactsStore` plus `workflowArtifactsSync`).
- Added store/service regressions in `web/src/stores/workflowArtifactsStore.test.ts` and `web/src/services/workflowArtifactsSync.test.ts` proving tag/report-path changes are treated as meaningful run updates instead of being hidden behind a coarse path+timestamp skip rule.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/stores/workflowArtifactsStore.test.ts src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

