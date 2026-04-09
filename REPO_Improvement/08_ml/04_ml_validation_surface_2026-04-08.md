# ML Validation Surface Plan

Last updated: 2026-04-08

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

## Checklist

- [ ] Document staged ML validation/sweep UX states.
- [ ] Document prerequisite failures and the exact stage they block.
- [ ] Document direct-validation and policy-sweep result summaries separately.
- [ ] Document user-facing score-summary fields and how they should be displayed.
- [ ] Document Stage 5 attached-validation summary states and wording.
- [ ] Document artifact reveal/open/replay expectations after successful runs.
- [ ] Add tests for missing score-cache paths and malformed/partial results.
- [ ] Add tests preserving stage-specific busy/success/failure wording.
- [ ] Add tests for artifact-path propagation and retained score-summary fields.

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

## Rollout / Fallback / Compatibility Notes

- preserve current stage-specific wording where it is already accurate
- prefer additive explanatory text over hiding existing summary fields
- if the surface later supports more ML stages, document them explicitly rather than overloading current messages

## Related Docs

- `REPO_Improvement/08_ml/19_mlsignal_validation_flow_2026-04-08.md`
- `REPO_Improvement/08_ml/03_ml_score_cache_bundles_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
