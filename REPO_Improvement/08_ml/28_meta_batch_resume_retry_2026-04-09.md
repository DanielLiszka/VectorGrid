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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [AWS Well-Architected: Make All Responses Idempotent](https://docs.aws.amazon.com/wellarchitected/latest/framework/rel_prevent_interaction_failure_idempotent.html)
  - relevant because retry safety in this repo depends on repeat requests not duplicating completed work
- [Background Tasks with Hosted Services in ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/host/hosted-services)
  - relevant because long-running campaign recovery should keep cancellation and shutdown/restart behavior explicit instead of implicit in UI state
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the saved batch summary acts as a versioned replay contract, not just a log blob
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because replayed campaigns still cross the same command boundary and must preserve request-shape compatibility
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

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
8. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

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
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/stores/taskStoreCampaignProgress.test.ts`
- run targeted campaign tests in `src-tauri/src/commands_campaign.rs`
- save one batch summary, then manually exercise `Retry failed` and `Resume incomplete`
- manually verify that a recovery run still forces skip-completed behavior
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- treat saved batch summaries as compatibility artifacts until there is an explicit migration mechanism
- if recovery semantics expand later, add summary fields rather than reinterpreting existing ones
- preserve the distinction between operational summary recovery and general history/artifact replay
- prefer failing clearly on incomplete summaries over issuing partially reconstructed campaigns
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/27_meta_batch_registry_modes_2026-04-09.md`
- `REPO_Improvement/08_ml/08_meta_batch_generation_2026-04-09.md`
- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
- `REPO_Improvement/06_artifacts/25_history_detail_actions_2026-04-09.md`
