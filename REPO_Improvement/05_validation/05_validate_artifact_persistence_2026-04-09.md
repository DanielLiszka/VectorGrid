# Validation Artifact Persistence Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to validation artifact persistence so successful validation runs reliably update saved-run artifacts, workflow-artifact state, and later replay/reopen surfaces without hiding partial-failure conditions.

## Scope Boundaries

In scope:

- `web/src/services/validationService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`

Out of scope:

- artifact-library indexing beyond the persisted validation payload
- workbook-generation internals
- options-related artifact flows

## Current User-Facing Behavior

After a successful validation run, the frontend persists the combined run/validation state as a `vectorgrid_run_artifact`, optionally includes `report_xlsx_path`, normalizes `dataset_meta`, and then reapplies that payload to the workflow-artifacts store. This lets the validation tab, history surfaces, and later reopen/replay flows work off a stable saved artifact instead of an ephemeral in-memory result.

Current behavior is intentionally forgiving: if validation artifact persistence fails, the run itself is not rethrown as a user-facing validation failure. The system tries to preserve the analyst result while keeping persistence and report follow-up as adjacent but non-identical concerns.

## Current Code Surface

- `web/src/services/validationService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`

## Current Verification Surface

- `web/src/services/validationService.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/workflowArtifactsSync.test.ts`
- `src-tauri/src/run_artifacts.rs` tests

## Strengths Already Present

- validation persistence is already centralized in the service instead of spread across component actions
- workflow-artifact reapplication already happens immediately after successful persistence
- run artifacts already carry `dataset_meta`, `config_fingerprint`, and report-path data needed by later consumers
- persistence failure is already kept logically separate from validation execution success
- sync/hydration infrastructure already exists for later reopen flows

## Weaknesses / Failure Modes

- successful validation and successful persistence are adjacent enough to be confused by users and future maintainers
- partial-success cases can become ambiguous if status-line language is not kept explicit
- persistence depends on several fields staying aligned:
  - run request/result
  - validation request/result
  - dataset metadata
  - report path
- workflow-artifact update behavior can drift from history/reopen expectations if the save payload changes silently

## Regressions To Avoid

- making validation success depend on artifact persistence success without an explicit product decision
- dropping `report_xlsx_path`, `dataset_meta`, or `config_fingerprint` from persisted artifacts
- updating workflow-artifact state with a shape that later hydration/detail flows cannot consume
- breaking the distinction between current run payload and current validation payload inside the workflow-artifacts store
- making saved validation artifacts non-replayable from history or sync surfaces

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because persisted validation artifacts act as a contract shared by backend save/load/detail logic and frontend hydration/replay logic.
- [RFC 8259 - The JavaScript Object Notation (JSON) Data Interchange Format](https://www.rfc-editor.org/rfc/rfc8259) is relevant because absent-vs-null field behavior matters for artifact compatibility.
- [RFC 8785 - JSON Canonicalization Scheme (JCS)](https://www.rfc-editor.org/rfc/rfc8785) is relevant because config-fingerprint and artifact identity depend on stable canonical JSON behavior.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because workflow artifacts are shared application state, not panel-local data.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because persistence and hydration correctness should remain testable through deterministic store/service scenarios.

## Proposed Improvement Approach

1. Write one explicit validation-persistence contract that separates execution success from persistence success.
2. Document the minimum artifact payload required for later detail, hydration, report reopen, and validation replay flows.
3. Keep `applyWorkflowArtifactsPayload` aligned with the saved artifact shape rather than allowing divergent frontend-only normalization.
4. Treat persistence status and validation status as separate but adjacent user-facing outcomes.
5. Add targeted tests around partial-success cases so later refactors cannot quietly blur these contracts.

## Checklist

- [ ] Document the minimum persisted validation-artifact fields and their consumers.
- [ ] Document the sequence from run result to saved artifact to workflow-artifact reapplication.
- [ ] Document how `report_xlsx_path` and `dataset_meta` are sourced and preserved.
- [ ] Document partial-success behavior when validation succeeds but artifact persistence fails.
- [ ] Document how current-run and current-validation payloads must remain separable in the workflow-artifacts store.
- [ ] Add or tighten tests for successful persistence, partial persistence failure, and later hydration compatibility.
- [ ] Add or tighten tests for persisted report-path propagation after export.

## Verification Plan

- run `npm --prefix web test -- src/services/validationService.test.ts`
- run `npm --prefix web test -- src/stores/workflowArtifactsStore.test.ts`
- run `npm --prefix web test -- src/services/workflowArtifactsSync.test.ts`
- run targeted `run_artifacts.rs` tests
- manually run validation, export XLSX, then reload/reopen the saved artifact path through history

## Rollout / Fallback / Compatibility Notes

- keep persistence non-fatal to validation execution unless product requirements change explicitly
- prefer additive payload hardening over schema-breaking changes
- if persistence semantics tighten, update history/detail/hydration docs and tests at the same time
- keep frontend normalization compatible with existing saved artifacts first

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/21_report_artifact_path_persistence_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/05_validation/04_validate_report_export_2026-04-09.md`
