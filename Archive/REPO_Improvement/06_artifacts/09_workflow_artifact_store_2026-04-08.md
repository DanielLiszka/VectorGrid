# Workflow Artifact Store Plan

Last updated: 2026-04-09

## Purpose

Define the `workflowArtifactsStore` contract so current-run and current-validation hydration, normalization, identity tracking, and no-op detection remain stable across optimization, validation, ML, history, and workflow-sync flows.

## Scope Boundaries

In scope:

- `web/src/stores/workflowArtifactsStore.ts`
- `applyWorkflowArtifactsPayload`
- current-run/current-validation normalization
- latest identity snapshot and no-op semantics

Out of scope:

- Rust artifact building
- full history detail rendering
- options artifact state

## Current User-Facing Behavior

Users experience this store indirectly whenever a run is saved, hydrated, reopened from history, synced from the latest artifact, or reused by analysis/validation flows. The current store already normalizes hydrated payloads, preserves run identity, keeps source tags, avoids timestamp churn on identical normalized payloads, and treats validation appearing later for the same run as a meaningful state change. Several frontend flows already assume those semantics, especially latest-artifact sync and post-validation refresh.

## Current Code Surface

- `web/src/stores/workflowArtifactsStore.ts`
  - owns `currentRun`, `currentValidation`, source tagging, normalization helpers, signatures, and latest identity snapshot helpers
- `web/src/stores/workflowArtifactsStore.test.ts`
  - verifies no-op payload behavior, validation-arrives-later semantics, and metric-surface normalization
- `web/src/services/workflowArtifactsSync.ts`
  - consumes identity snapshot and apply result for latest-artifact sync
- `web/src/services/validationService.ts`
  - applies payloads after validation/save flows
- `web/src/services/mlSignalService.ts`
  - applies payloads for ML stages that also reuse the store
- `src-tauri/src/run_artifacts.rs`
  - upstream producer for hydrated payload shape

## Current Verification Surface

- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/mlSignalService.test.ts`

## Strengths Already Present

- current-run/current-validation payloads are already normalized into one shared store
- identity snapshot helpers already make artifact-sync skip logic safe and explicit
- no-op detection already avoids unnecessary timestamp churn
- metric-surface and ML subtrees are already normalized instead of being blindly accepted
- the store already acts as a bridge between optimization, validation, history, and ML flows

## Weaknesses / Failure Modes

- normalization logic is dense enough that field-level guarantees are easy to misread
- the store serves multiple domains, which increases the risk of accidental broad regressions
- source tagging is valuable but easy to treat as incidental rather than part of contract
- public hydration identity and deep payload equality are related but distinct concepts

## Regressions To Avoid

- losing no-op detection and re-stamping timestamps on identical payloads
- skipping validation change detection when validation arrives for the same run later
- silently dropping `optimizationMetricSurface`, `datasetMeta`, `mlSignal`, tags, or report path
- weakening or removing latest-identity snapshot helpers used by sync logic
- refreshing analysis on source-only or otherwise meaningless changes
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: What Is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because the store effectively enforces a normalized cross-layer payload contract
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because these payloads ultimately derive from persisted JSON artifacts
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because stable normalization/signature behavior benefits from canonicalized payload thinking
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because this store is shared workflow state, not local widget state

## Proposed Improvement Approach

1. Document the store contract by payload family:
   - current run
   - current validation
   - ML attachment subtree
   - optimization metric surface
2. Document apply-result semantics explicitly:
   - `applied`
   - `runChanged`
   - `validationChanged`
   - `sourceChanged`
3. Separate public hydration identity from deep payload-equality semantics in the documentation, because both are used today for different purposes.
4. Treat this store as shared infrastructure for multiple workflows, so changes must be synchronized across all producers.
5. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the normalized `currentRun` and `currentValidation` field families.
- [x] Document source-tag semantics and latest identity snapshot semantics.
- [x] Document no-op detection and timestamp-churn avoidance.
- [x] Document validation-arrives-later behavior for same-run hydration.
- [x] Document current producer flows that write this store.
- [x] Add or tighten tests if normalization or apply-result semantics change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm --prefix web run test -- src/services/workflowArtifactsSync.test.ts`
- run `npm --prefix web run test -- src/services/validationService.test.ts`
- run `npm --prefix web run test -- src/services/mlSignalService.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- keep normalized field names compatible across existing producers
- if signatures or identity helpers change, update sync/history/validation consumers in the same change
- preserve current source tags until all callers are migrated deliberately
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed as a normalization/identity hardening slice in `web/src/stores/workflowArtifactsStore.ts`. The store contract already centralizes snake_case to camelCase normalization, ML attachment shaping, metric-surface preservation, no-op detection, latest-identity tracking, and validation-arrives-later semantics.
- Added direct store tests for two previously under-pinned behaviors: snake_case hydrated run payload normalization without losing `reportXlsxPath`, `datasetMeta`, tags, or `mlSignal`, and source-only updates that should mark `sourceChanged` without timestamp churn or run/validation reshaping.
- Re-verified the main producer/consumer flows around the store: `workflowArtifactsSync`, `historyService`, and validation artifact persistence still behave correctly on top of the same store contract.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/stores/workflowArtifactsStore.test.ts src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo check -p ta_desktop_demo_app`.

## Related Docs

- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/04_results/17_analysis_store_service_2026-04-08.md`
