# Workflow Artifacts Sync Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for workflow-artifact sync so the Vue runtime can safely discover the latest run artifact, decide whether hydration is necessary, apply normalized payloads, and refresh analysis only when the effective workflow meaning changed.

## Scope Boundaries

In scope:

- `web/src/services/workflowArtifactsSync.ts`
- list-latest-run discovery
- identity-based skip logic
- hydration invocation and normalized application
- post-sync analysis refresh behavior

Out of scope:

- artifact generation itself
- full history-library browsing
- options artifacts

## Current User-Facing Behavior

The app already supports syncing workflow artifacts from the legacy runtime path. Current behavior includes:

- listing run artifacts
- selecting the latest run summary
- loading hydration for the latest path when needed
- skipping hydration if the current identity already matches and validation is already present
- applying the payload into the workflow-artifacts store
- refreshing analysis only when the normalized payload changed meaningfully
- updating the artifact source tag

This means artifact sync already behaves like a small decision engine rather than a blind “always reload latest artifact” command.

## Current Code Surface

- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/tauri/invoke.ts`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- history-service tests that cover hydration behavior

## Strengths Already Present

- the sync decision flow is already centralized
- latest-identity skip logic already exists
- normalized store application already determines whether the change was meaningful
- analysis refresh already depends on meaningful application instead of unconditional churn
- source tagging already distinguishes `legacy-runtime` and `vue`

## Weaknesses / Failure Modes

- sync behavior spans list, identity compare, hydrate, normalize, and refresh layers
- same-path/same-ts skip rules are subtle because validation presence matters
- sync failures are intentionally best-effort, which can hide missing sync if the contract is not documented clearly
- store identity logic and sync service behavior can drift if they are treated separately

## Regressions To Avoid

- reloading every time even when identity is unchanged
- skipping hydration when validation is newly available but the run path/ts are unchanged
- refreshing analysis when nothing meaningful changed
- dropping source-tag updates during no-op syncs
- widening sync logic beyond the intended latest-run workflow without a new contract
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because sync depends on Tauri commands for artifact discovery and hydration
- [RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because the store’s meaningful-change logic maps naturally to stable structural identity
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)
  - relevant because the sync payload and store-normalized shapes already behave like schema-governed contracts
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because sync should ultimately be validated through visible workspace-state changes, not only internal flags
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Document workflow-artifact sync as a five-step contract:
   - discover latest run
   - compare identity
   - hydrate when needed
   - normalize/apply
   - refresh dependent analysis when meaning changed
2. Make the skip rule explicit, including the “same path and ts but missing validation must still reload” exception.
3. Treat source-tag updates and analysis refresh behavior as part of the contract, not side effects.
4. Expand tests around no-op sync, validation-newly-appears sync, and invoke-failure handling.
5. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the latest-artifact discovery flow and assumptions.
- [ ] Document identity-compare and skip-load rules.
- [ ] Document when source tags should update even on no-op sync.
- [ ] Document when analysis refresh should and should not run.
- [ ] Add tests for same-path/same-ts skip behavior with and without validation.
- [ ] Add tests for invoke-failure and malformed-payload best-effort behavior.
- [ ] Add tests proving meaningful-change detection stays aligned with store normalization.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/services/workflowArtifactsSync.test.ts`
- run `npm test -- src/stores/workflowArtifactsStore.test.ts`
- run relevant history-service tests that cover artifact hydration
- manually verify latest-run sync from a saved artifact set in the workflow shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve best-effort sync behavior rather than turning latest-artifact sync into a hard blocker
- keep sync scoped to latest-run hydration unless a broader artifact-library contract is defined separately
- if sync logic is generalized later, preserve current latest-run behavior as a stable baseline
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`

