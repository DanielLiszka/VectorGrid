# Meta Batch Resume/Retry Plan

Last updated: 2026-04-09

## Purpose

Define the current meta batch resume/retry contract so summary-path ownership, failed-path recovery, `skip_existing` posture, and resumable request restoration remain explicit and safe against duplicate work or broken campaign state.

## Scope Boundaries

In scope:

- `Retry failed` and `Resume incomplete` behavior
- saved batch summary path ownership and lifecycle
- summary-derived path selection and request restoration
- `skip_existing` semantics for replayed batch work
- current frontend orchestration of resume/retry flows
- compatibility requirements for the persisted batch summary

Out of scope:

- baseline batch request creation rules outside what resume/retry consumes
- Stage 3 or Stage 5 algorithmic behavior inside each per-dataset run
- artifact-library browsing for families other than the saved summary files that power these flows
- options workflows

## Current User-Facing Behavior

The Meta workspace exposes `Retry failed`, `Resume incomplete`, and `Reveal summary` after or during batch activity. These are currently frontend-orchestrated recovery workflows driven by the last saved batch summary rather than a generic artifact replay system. Current behavior includes:

- both resume and retry require a known summary path
- retry targets failed datasets first, but can prefer currently selected paths where appropriate
- resume requires an embedded resumable request in the summary and restores batch UI state before re-running
- both flows force `skip_existing=true` to remain conservative around duplicated work
- summary reveal is treated as an operational debugging/inspection surface, not only a file-save side effect
- batch recovery is asymmetrical with normal artifact-library flows because batch summaries are persisted but not yet browsed as a first-class artifact family

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - batch recovery buttons, button-state wiring, and summary reveal affordances
- `web/src/stores/analysisStore.ts`
  - batch recovery actions, enable/disable state, and current batch-summary-related store fields
- `web/src/services/metaService.ts`
  - summary-path tracking, resume/retry orchestration, restored request shaping, failed-path handling, and reveal-summary behavior
- `src-tauri/src/commands_campaign.rs`
  - batch execution command boundary reused by fresh and replayed campaigns
- `src-tauri/src/run_artifacts.rs`
  - batch-summary path suggestion and persistence helpers
- `src-tauri/src/commands_meta.rs`
  - artifact-kind recognition for saved batch and campaign summaries
- `web/src/stores/taskStoreCampaignProgress.test.ts`
  - progress plumbing adjacent to summary-path lifecycle

## Current Verification Surface

- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- targeted campaign tests in `src-tauri/src/commands_campaign.rs`
- artifact-save tests touching `src-tauri/src/run_artifacts.rs`

## Strengths Already Present

- retry and resume are already separated instead of overloaded into one ambiguous “rerun” button
- saved summaries already contain enough information to support conservative recovery
- current flows force `skip_existing=true`, which reduces duplicate-work risk
- the service layer already restores UI state from the saved request instead of asking the user to rebuild context manually
- failure cases such as missing summary, missing resumable request, or empty failed-path set already abort cleanly

## Weaknesses / Failure Modes

- recovery semantics live primarily in `metaService.ts`, so they are easy to mis-document as backend-native when they are currently frontend orchestration
- summary-path ownership is partly event-driven, which means stale or missing summary metadata can strand otherwise resumable work
- retry semantics depend on correct failed-path extraction and currently selected-path reconciliation
- batch summaries are operationally important but not yet first-class in the artifact library, which makes discoverability and mental models weaker
- if summary schema fields drift, resume/retry can fail much later than the code change that introduced the incompatibility

## Regressions To Avoid

- allowing retry or resume to run without a valid summary path
- removing `skip_existing=true` from recovery flows without a deliberate idempotency review
- changing summary field names or semantics without updating recovery logic and compatibility expectations
- conflating retry and resume into one flow that loses the distinction between failed-path-only and resumable-request restoration
- treating batch summaries as disposable files when they currently anchor operational recovery

## External Research Summary

- [AWS Well-Architected: Make All Responses Idempotent](https://docs.aws.amazon.com/wellarchitected/latest/framework/rel_prevent_interaction_failure_idempotent.html)
  - relevant because retry safety in this repo depends on repeat requests not duplicating completed work
- [Background Tasks with Hosted Services in ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/host/hosted-services)
  - relevant because long-running campaign recovery should keep cancellation and shutdown/restart behavior explicit instead of implicit in UI state
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the saved batch summary acts as a versioned replay contract, not just a log blob
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because replayed campaigns still cross the same command boundary and must preserve request-shape compatibility

## Proposed Improvement Approach

1. Document retry and resume as separate recovery contracts with different prerequisites and different scope of replay.
2. Make summary-path ownership explicit:
   - where it is produced
   - where it is cached
   - where it is consumed
3. Record the exact fields a saved summary must preserve for retry and for resume.
4. Keep `skip_existing=true` documented as a deliberate duplicate-work guard, not an incidental implementation detail.
5. Describe failure handling explicitly so the UI remains conservative when summaries are stale or incomplete.
6. Tie recovery semantics to current service-layer ownership so refactors do not accidentally move or weaken behavior without updating tests.
7. Keep “Reveal summary” in scope because recovery usability depends on inspectable summary files, not just hidden internal state.

## Checklist

- [ ] Document the prerequisite summary-path requirement for retry and resume.
- [ ] Document how the last batch summary path is produced, stored, and reused.
- [ ] Document retry path selection rules, including failed-path extraction and selected-path interaction.
- [ ] Document resume prerequisites, including the need for an embedded resumable request.
- [ ] Document forced `skip_existing=true` behavior for both flows.
- [ ] Document current clean-abort behavior for missing summary, missing failed paths, or missing resumable request.
- [ ] Document the current asymmetry between batch-summary recovery and normal artifact-library replay.
- [ ] Document the minimum summary fields required to keep recovery compatible.
- [ ] Add or preserve tests for retry-only, resume-only, and missing-summary failure cases.
- [ ] Keep summary-schema edits synchronized with recovery logic and tests.

## Verification Plan

- run `npm --prefix web test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/stores/taskStoreCampaignProgress.test.ts`
- run targeted campaign tests in `src-tauri/src/commands_campaign.rs`
- save one batch summary, then manually exercise `Retry failed` and `Resume incomplete`
- manually verify that a recovery run still forces skip-completed behavior

## Rollout / Fallback / Compatibility Notes

- treat saved batch summaries as compatibility artifacts until there is an explicit migration mechanism
- if recovery semantics expand later, add summary fields rather than reinterpreting existing ones
- preserve the distinction between operational summary recovery and general history/artifact replay
- prefer failing clearly on incomplete summaries over issuing partially reconstructed campaigns

## Related Docs

- `REPO_Improvement/08_ml/27_meta_batch_registry_modes_2026-04-09.md`
- `REPO_Improvement/08_ml/08_meta_batch_generation_2026-04-09.md`
- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
- `REPO_Improvement/06_artifacts/25_history_detail_actions_2026-04-09.md`
