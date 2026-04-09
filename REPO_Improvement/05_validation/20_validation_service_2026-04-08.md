# Validation Service Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the frontend validation service so request construction, preview generation, execution orchestration, artifact persistence, cross-dataset refresh, and export flows remain coordinated and truthful.

## Scope Boundaries

In scope:

- `web/src/services/validationService.ts`
- validation execution orchestration
- preview/admission comparison helpers
- export/report flows
- cross-dataset refresh and selection handoff

Out of scope:

- low-level request-shape rules owned by the validation request builder
- backend validation execution internals
- options flows

## Current User-Facing Behavior

The validation service already does much more than send one command. Current behavior includes:

- building and running validation
- previewing validation execution summaries from current workflow state
- previewing admission-policy comparisons
- exporting:
  - shortlist CSV
  - validation JSON
  - report JSON
  - report XLSX
- opening the reports directory
- cross-dataset refresh and selection synchronization
- artifact persistence and workflow-artifact application after successful validation

The service also normalizes user-facing status and validation-panel state during:

- startup of validation
- successful run
- failed run
- export/report follow-up
- cross-dataset refresh and selection updates

## Current Code Surface

- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/tauri/invoke.ts`
- `src-tauri/src/commands_validation.rs`

## Current Verification Surface

- `web/src/services/validationService.test.ts`
- `web/src/stores/validationStore.actions.test.ts`
- `web/src/components/ValidationWorkspacePanel.test.ts`
- backend validation command tests

## Strengths Already Present

- the service already centralizes validation orchestration instead of burying it in the component
- preview generation and actual execution live near each other, which helps keep them aligned
- export/report behavior is already part of the service contract
- cross-dataset refresh and selection flows are already explicit
- artifact persistence is already wired into post-validation flow

## Weaknesses / Failure Modes

- the service owns many adjacent responsibilities at once:
  - preview building
  - execution
  - exports
  - artifact persistence
  - cross-dataset refresh
- status updates can drift from actual service behavior if patch-store rules are not documented clearly
- preview logic and run logic can diverge subtly if only one side changes
- cross-dataset synchronization includes optimistic UI updates plus service/backend handoff, which is easy to destabilize

## Regressions To Avoid

- breaking validation preview accuracy relative to actual execution
- losing validation-artifact application after successful runs
- hiding export/report failures behind fake success status
- making cross-dataset refresh or selection less truthful than it is today
- weakening dataset/timeframe compatibility enforcement before the command runs

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the service is the frontend orchestration boundary before Tauri invocation
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because validation orchestration depends on chronological robustness modules and summaries
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because request/result/report shapes are already acting like explicit contracts
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because the most important service outcomes should remain visible and testable from the user-facing surface

## Proposed Improvement Approach

1. Split the service contract into explicit responsibility groups:
   - preview
   - execution
   - artifact persistence
   - export/report
   - cross-dataset utilities
2. Document the status-line and store-patch contract for success, failure, and partial-success flows.
3. Keep preview helpers and execution builders aligned by documenting shared prerequisites and expected divergences.
4. Treat cross-dataset refresh and selection synchronization as first-class orchestration behavior, not side logic.
5. Expand tests around preview parity, export failure, and artifact application.

## Checklist

- [ ] Document validation service responsibility groups and ownership boundaries.
- [ ] Document preview-generation and execution-orchestration flows separately.
- [ ] Document status-line/store-patch behavior for success, failure, and partial export/report outcomes.
- [ ] Document artifact persistence and workflow-artifact application behavior.
- [ ] Document cross-dataset refresh and selection synchronization behavior.
- [ ] Add tests for preview parity with actual request execution.
- [ ] Add tests for export/report failure status handling.
- [ ] Add tests for successful validation artifact application and cross-dataset refresh flows.

## Verification Plan

- run `npm test -- src/services/validationService.test.ts`
- run `npm test -- src/stores/validationStore.actions.test.ts`
- run `npm test -- src/components/ValidationWorkspacePanel.test.ts`
- run backend validation command tests
- manually verify a validation run plus at least one export/report path

## Rollout / Fallback / Compatibility Notes

- preserve current service-facing method names and broad behavior where possible
- keep preview and execution logic close unless a better shared contract layer is introduced
- if the service is later split, preserve current user-visible behavior and store-patch semantics first

## Related Docs

- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/21_validation_store_2026-04-08.md`
- `REPO_Improvement/06_artifacts/28_workflow_artifacts_sync_2026-04-08.md`

