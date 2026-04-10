# ML Policy Sweep Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

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
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the ML policy-sweep request and response contract.
- [x] Add or tighten tests for sweep-axis parsing and optional-axis omission behavior.
- [x] Add or tighten tests for saved-run sweep-summary attachment and reopen interpretation.
- [x] Audit UI copy so users can distinguish sweep output from normal ML validation.
- [x] Confirm frontend controls match `run_ml_signal_policy_sweep` payload semantics exactly.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MlSignalWorkspacePanel.test.ts`
- run relevant ML reopen tests in `historyService.test.ts`
- run targeted command/request tests for `run_ml_signal_policy_sweep` if available
- perform a bounded manual sweep run if payload or summary semantics change materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve the dedicated sweep command instead of folding it into generic validation by default
- keep saved-run sweep summaries backward-compatible while tightening interpretation
- if a cleanup makes policy sweep harder to distinguish, prefer clearer labels and separate summaries over consolidation
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/08_ml/04_ml_validation_surface_2026-04-08.md`
- `REPO_Improvement/06_artifacts/23_mlsignal_report_path_semantics_2026-04-09.md`

## Execution Notes

- Closed by tightening the existing sweep request/summary contract rather than redesigning the feature.
- `mlSignalService.test.ts` now verifies optional sweep-axis omission behavior (`delta` and `max_hold_bars` remain null when unset) while keeping the dedicated `run_ml_signal_policy_sweep` payload shape explicit.
- Sweep-summary persistence and reopen interpretation remain covered through the saved-artifact assertions in `mlSignalService.test.ts` and `historyService.test.ts`.
- The workspace panel continues to distinguish policy sweep from direct validation through dedicated stage labels, button text, and Stage 5 attached-summary wording.
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/mlSignalService.test.ts src/components/MlSignalWorkspacePanel.test.ts src/services/historyService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
