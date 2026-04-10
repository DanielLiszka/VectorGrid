# Meta Service Campaigns Plan

Last updated: 2026-04-09

## Purpose

Define the current frontend meta campaign contract so dataset selection, request shaping, progress handling, summary persistence, and resume/retry boundaries remain explicit across the service/store/runtime layers.

## Scope Boundaries

In scope:

- frontend campaign orchestration in `metaService.ts`
- dataset selection and compatibility checks
- campaign progress/listener integration
- summary persistence and recovery hooks
- interaction with the backend campaign command

Out of scope:

- general batch request-builder details already covered elsewhere
- ML Signal campaign behavior, which is not part of the shipped campaign path
- options workflows

## Current User-Facing Behavior

The Meta workspace batch-generation surface is orchestrated from the frontend service. Current behavior means:

- users select datasets and batch settings in the Meta workspace
- the service builds the campaign request, invokes the backend, and tracks progress events
- the service persists the latest summary path and uses it for retry/resume flows
- campaigns currently reject unsupported strategy families like `MlSignal` and `DynamicDoubleMa`
- `optimize_only`, `validate_only`, and `full` materially change follow-up behavior

## Current Code Surface

- `web/src/services/metaService.ts`
  - campaign request assembly, dataset compatibility checks, progress handling, summary persistence, retry/resume, and reveal-summary behavior
- `web/src/components/MetaWorkspacePanel.vue`
  - batch-generation controls and recovery buttons
- `web/src/stores/analysisStore.ts`
  - batch state, defaults, and entrypoint actions
- `web/src/stores/taskStore.ts`
  - progress/task integration adjacent to campaign flow
- `src-tauri/src/commands_campaign.rs`
  - backend campaign execution contract
- `src-tauri/src/run_artifacts.rs`
  - batch/campaign summary path helpers and persistence

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- tests embedded in `src-tauri/src/commands_campaign.rs`

## Strengths Already Present

- campaign orchestration is centralized in one frontend service instead of being spread across the component and store
- unsupported strategy and mixed-timeframe checks happen early in the frontend
- summary persistence already exists, which keeps recovery possible
- progress events are already integrated into the service/task/store flow

## Weaknesses / Failure Modes

- campaign orchestration, progress handling, summary persistence, and recovery all live in one large service surface
- retry/resume depend on last-summary ownership rather than a richer library-style summary picker
- service behavior depends on several adjacent stores and runtime listeners, which makes ownership easy to misdescribe
- auto-meta and auto-Stage-5 behavior are easy to oversimplify if execution mode is not documented carefully

## Regressions To Avoid

- describing campaigns as meta-only or Stage-5-only when they are broader orchestration flows
- forgetting that `MlSignal` and `DynamicDoubleMa` are excluded from the shipped campaign strategy surface
- breaking summary persistence while refactoring batch UI or task handling
- weakening mixed-timeframe checks or execution-mode constraints
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [AWS Well-Architected: Make All Responses Idempotent](https://docs.aws.amazon.com/wellarchitected/latest/framework/rel_prevent_interaction_failure_idempotent.html)
  - relevant because campaign resume/retry depends on replay-safe behavior and `skip_done` posture
- [Background Tasks with Hosted Services in ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/host/hosted-services)
  - relevant because campaign work is long-running and needs clear progress, cancellation, and restart boundaries
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the campaign path is a structured frontend/backend orchestration workflow
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because summary envelopes and batch requests are persisted structured contracts
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the frontend service as the orchestration owner and the backend as the execution owner.
2. Separate campaign responsibilities into:
   - dataset eligibility
   - request shaping
   - progress/listener handling
   - summary persistence
   - recovery
3. Preserve explicit notes about unsupported strategy families and timeframe compatibility.
4. Keep execution-mode consequences explicit in the campaign doc, not only in the builder doc.
5. Record the summary-path lifecycle clearly because recovery depends on it.
6. Tie campaign docs to progress/task tests and recovery tests instead of only request-builder tests.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current frontend campaign orchestration responsibilities in `metaService.ts`.
- [x] Document dataset compatibility and unsupported-strategy checks.
- [x] Document progress-listener and task-store interaction.
- [x] Document current summary persistence and last-summary ownership.
- [x] Document retry/resume hooks and their dependence on the saved summary.
- [x] Preserve tests for campaign strategy filtering, resume/retry, and progress behavior.
- [x] Preserve backend campaign tests in `commands_campaign.rs`.
- [x] Treat major campaign-service refactors as compatibility-sensitive because they affect UI, tasking, and recovery at once.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/taskStoreCampaignProgress.test.ts`
- run targeted campaign tests in `src-tauri/src/commands_campaign.rs`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current summary-recovery compatibility while campaign architecture is still service-centric
- if the repo later gains richer summary browsing, add it without breaking current last-summary recovery flows
- keep unsupported-strategy exclusions explicit until there is deliberate campaign support for those families
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by pinning the campaign service’s recovery contract to real storage-backed behavior instead of only in-memory state. `metaServiceCampaignResume.test.ts` now verifies that `batchRevealSummary()` hydrates the last summary path from `LAST_BATCH_SUMMARY_KEY`, reloads the saved summary artifact, and reveals that same persisted path.
- The existing campaign strategy/resume suites already cover unsupported strategy filtering, execution-mode semantics, skip-done behavior, and resumable request reuse. This pass filled the missing summary-ownership gap rather than refactoring the service.
- `REPO_MAP.md` now makes it easier to find the batch/campaign orchestration notes alongside the Meta service ownership description.
- Stable fixture / replay payload:
  - `LAST_BATCH_SUMMARY_KEY -> C:/runs/completed_campaign.json`
  - `kind=vectorgrid_campaign_summary`
  - `strategy_kind=DoubleMa`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts src/services/metaServiceCampaignResume.test.ts src/services/metaService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`

## Related Docs

- `REPO_Improvement/08_ml/27_meta_batch_registry_modes_2026-04-09.md`
- `REPO_Improvement/08_ml/28_meta_batch_resume_retry_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
