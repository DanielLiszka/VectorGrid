# Workflow Artifacts Sync Plan

Last updated: 2026-04-08

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

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because sync depends on Tauri commands for artifact discovery and hydration
- [RFC 8785: JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785)
  - relevant because the store’s meaningful-change logic maps naturally to stable structural identity
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)
  - relevant because the sync payload and store-normalized shapes already behave like schema-governed contracts
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because sync should ultimately be validated through visible workspace-state changes, not only internal flags

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

## Checklist

- [ ] Document the latest-artifact discovery flow and assumptions.
- [ ] Document identity-compare and skip-load rules.
- [ ] Document when source tags should update even on no-op sync.
- [ ] Document when analysis refresh should and should not run.
- [ ] Add tests for same-path/same-ts skip behavior with and without validation.
- [ ] Add tests for invoke-failure and malformed-payload best-effort behavior.
- [ ] Add tests proving meaningful-change detection stays aligned with store normalization.

## Verification Plan

- run `npm test -- src/services/workflowArtifactsSync.test.ts`
- run `npm test -- src/stores/workflowArtifactsStore.test.ts`
- run relevant history-service tests that cover artifact hydration
- manually verify latest-run sync from a saved artifact set in the workflow shell

## Rollout / Fallback / Compatibility Notes

- preserve best-effort sync behavior rather than turning latest-artifact sync into a hard blocker
- keep sync scoped to latest-run hydration unless a broader artifact-library contract is defined separately
- if sync logic is generalized later, preserve current latest-run behavior as a stable baseline

## Related Docs

- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`

