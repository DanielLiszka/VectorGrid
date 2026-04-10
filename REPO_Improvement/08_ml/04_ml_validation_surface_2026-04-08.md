# ML Validation Surface Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the user-facing ML Signal validation surface so prerequisite checks, busy-state copy, result summaries, score-cache diagnostics, and policy-sweep display remain clear and consistent without weakening the existing backend contract.

## Scope Boundaries

In scope:

- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- user-facing status, preview, and result-summary behavior
- validation and policy-sweep shell messaging
- surfaced score-summary fields

Out of scope:

- low-level score-cache file contract
- dataset export internals
- stage-4/stage-5 meta flows

## Current User-Facing Behavior

The ML Signal workspace already exposes a staged validation surface rather than one generic run button. Current user-visible stages include:

- dataset export
- score-cache generation
- direct ML validation
- ML policy sweep

The service already uses stage-specific busy/status text such as:

- `ML score cache • ...`
- `ML validation • ...`
- `ML policy sweep • ...`

The current panel also distinguishes attached validation outcomes in the Stage 5 summary, including:

- no ML validation artifact attached yet
- single validation attached
- policy sweep attached

Current result/summary material can include:

- candidate counts
- `matched_rows`
- `coverage_ratio`
- `duplicate_timestamps`
- `score_space`
- `score_scope`
- artifact paths and reveal/open actions

The service also blocks obvious prerequisite failures up front, such as missing score-cache paths.

## Current Code Surface

- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/mlSignalService.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.test.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/run_artifacts.rs`

## Current Verification Surface

- `web/src/services/mlSignalService.test.ts`
- ML Signal workspace component tests
- Rust ML validation/sweep tests in `crates/app_core/src/ml_signal.rs`

## Strengths Already Present

- the ML surface already distinguishes major stages instead of hiding them inside a generic optimize view
- stage-specific status text already exists
- score-summary quality signals already survive into the frontend
- the service already threads artifact paths and summaries back into the workflow
- direct validation and policy sweep already present distinct user outcomes
- validation and sweep both already enforce `validation_context` compatibility before work proceeds

## Weaknesses / Failure Modes

- user-facing stage messaging can drift from backend truth if service copy and result parsing diverge
- score-summary fields are rich but could be underexplained or inconsistently shown
- direct validation and policy sweep share machinery but should not feel identical to the user
- prerequisite failures can become confusing if the UI hides which stage is actually blocked
- artifact reveal/open affordances are easy to break when result parsing changes
- artifact/store state can drift from user-facing success copy if service updates are not treated as part of the same contract

## Regressions To Avoid

- weakening prerequisite checks in the frontend
- hiding coverage/duplicate/score-space diagnostics that currently help interpret results
- making policy sweep look like direct validation or vice versa
- losing artifact path/report-path visibility after successful ML runs
- replacing stage-specific status copy with generic “running” or “done” text
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn calibration guide](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because the UI surfaces probability-space results that are not interchangeable with raw scores
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because ML validation still operates on time-ordered data and should be described accordingly
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
  - relevant because this surface should be verified through visible status/results, not implementation details
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because stage-specific service behavior and store-facing updates are best locked down with focused frontend tests
- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because stage-progress and completion messaging is status information that should stay advisory and truthful

## Proposed Improvement Approach

1. Document the ML validation surface as a staged UX contract:
   - prerequisite state
   - busy state
   - success state
   - failure state
   - artifact state
2. Separate direct-validation and policy-sweep display rules:
   - candidate counts
   - summary wording
   - artifact expectations
   - attached validation summary wording in Stage 5
3. Define the user-facing meaning of score-summary fields:
   - coverage
   - duplicates
   - score space
   - score scope
4. Keep artifact-path reveal/open behavior tied to normalized result data rather than loose ad hoc fields.
5. Add tests for missing prerequisites, partial result payloads, and stage-specific success/failure copy.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document staged ML validation/sweep UX states.
- [x] Document prerequisite failures and the exact stage they block.
- [x] Document direct-validation and policy-sweep result summaries separately.
- [x] Document user-facing score-summary fields and how they should be displayed.
- [x] Document Stage 5 attached-validation summary states and wording.
- [x] Document artifact reveal/open/replay expectations after successful runs.
- [x] Add tests for missing score-cache paths and malformed/partial results.
- [x] Add tests preserving stage-specific busy/success/failure wording.
- [x] Add tests for artifact-path propagation and retained score-summary fields.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm test -- src/services/mlSignalService.test.ts`
- run ML workspace component tests
- manually exercise:
  - missing score-cache path
  - successful direct validation
  - successful policy sweep
  - failed validation
- manually verify Stage 5 attached-validation summary text after both single validation and sweep
- inspect the saved artifact and reopened workspace state for one ML validation run
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current stage-specific wording where it is already accurate
- prefer additive explanatory text over hiding existing summary fields
- if the surface later supports more ML stages, document them explicitly rather than overloading current messages
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`

## Execution Notes

- Closed by hardening the current validation/sweep UX contract instead of changing the ML workflow.
- `mlSignalService.test.ts` now verifies early failure on missing score-cache paths for both direct validation and policy sweep.
- `MlSignalWorkspacePanel.test.ts` now pins the disabled-state behavior of the stage buttons until dataset and path prerequisites exist, and keeps the Stage 5 empty-state copy explicit.
- Existing validation and sweep tests now also retain artifact-path and score-summary assertions through the persisted run-artifact payload.
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/mlSignalService.test.ts src/components/MlSignalWorkspacePanel.test.ts src/services/historyService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
