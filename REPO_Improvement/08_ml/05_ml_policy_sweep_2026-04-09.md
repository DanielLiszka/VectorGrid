## Purpose

Define the improvement plan for the ML Signal policy-sweep surface so parameter-sweep execution, result attachment, and saved-run interpretation remain understandable and regression-resistant.

## Scope Boundaries

In scope:

- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `src-tauri/src/commands_ml_signal.rs`
- ML Signal policy-sweep request building, execution, saved-artifact attachment, and result summary behavior

Out of scope:

- general meta policy evaluation in the meta workspace
- training/model-family internals
- options-related policy surfaces

## Current User-Facing Behavior

The ML Signal workspace already supports a dedicated policy-sweep workflow:

- users can configure sweep axes and validation gates
- the frontend runs a dedicated policy-sweep command
- saved artifacts can carry policy-sweep summaries
- the workspace can present “policy sweep attached” status and reuse artifact context during reopen flows

This is valuable, but it is easy for the sweep to become “just another validation result” instead of a clearly distinct artifact family.

## Current Code Surface

- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/historyService.ts`

Key current ownership:

- `mlSignalService.ts` builds optional sweep axes, dispatches `run_ml_signal_policy_sweep`, and extracts sweep summaries into artifact payloads
- `MlSignalWorkspacePanel.vue` owns the policy-sweep controls and user-facing execution flow
- `commands_ml_signal.rs` exposes the dedicated sweep command

## Current Verification Surface

- `web/src/components/MlSignalWorkspacePanel.test.ts`
- ML reopen/replay behavior in `web/src/services/historyService.test.ts`
- any command-level tests for `run_ml_signal_policy_sweep`

## Strengths Already Present

- policy sweep is already a first-class named action, not hidden behind generic validation
- sweep axes are structured rather than free-form strings
- sweep summary extraction is explicit in the frontend service
- saved runs can already surface policy-sweep attachment state

## Weaknesses / Failure Modes

- sweep behavior is assembled inside a large service that already owns many ML tasks
- the distinction between ordinary ML validation and policy sweep can blur in saved-run interpretation
- sweep summary attachment is compact, which helps usability but may hide important contract assumptions
- sweep parameter defaults and gating rules are easy to change without a centralized policy doc

## Regressions To Avoid

- conflating policy-sweep artifacts with ordinary validation artifacts
- changing sweep axis semantics or gate defaults without corresponding test updates
- attaching sweep summaries inconsistently across fresh runs and reopened runs
- letting UI controls drift from the actual command payload shape

## External Research Summary

- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because policy sweeps are still evaluated on time-ordered validation surfaces where leakage and split-discipline matter.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because sweep requests and saved summaries should behave like structured contracts.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the sweep is a dedicated command boundary with a distinct request/response shape.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because sweep configuration is assembled from shared optimize/runtime state.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because sweep behavior crosses component, store, and service boundaries.

## Proposed Improvement Approach

1. Treat ML policy sweep as its own artifact-producing workflow.
2. Document the request contract:
   - sweep axes
   - gate thresholds
   - candidate extraction
   - saved summary fields
3. Tighten the distinction between:
   - ordinary ML validation
   - ML policy sweep
   - meta-level policy evaluation
4. Add tests that pin sweep attachment and reopen semantics.
5. Keep sweep controls additive and explicit rather than hiding them behind overloaded validation controls.

## Checklist

- [ ] Document the ML policy-sweep request and response contract.
- [ ] Add or tighten tests for sweep-axis parsing and optional-axis omission behavior.
- [ ] Add or tighten tests for saved-run sweep-summary attachment and reopen interpretation.
- [ ] Audit UI copy so users can distinguish sweep output from normal ML validation.
- [ ] Confirm frontend controls match `run_ml_signal_policy_sweep` payload semantics exactly.

## Verification Plan

- run `MlSignalWorkspacePanel.test.ts`
- run relevant ML reopen tests in `historyService.test.ts`
- run targeted command/request tests for `run_ml_signal_policy_sweep` if available
- perform a bounded manual sweep run if payload or summary semantics change materially

## Rollout / Fallback / Compatibility Notes

- preserve the dedicated sweep command instead of folding it into generic validation by default
- keep saved-run sweep summaries backward-compatible while tightening interpretation
- if a cleanup makes policy sweep harder to distinguish, prefer clearer labels and separate summaries over consolidation

## Related Docs

- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/08_ml/04_ml_validation_surface_2026-04-08.md`
- `REPO_Improvement/06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md`
