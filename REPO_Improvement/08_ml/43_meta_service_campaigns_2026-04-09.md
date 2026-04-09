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

## External Research Summary

- [AWS Well-Architected: Make All Responses Idempotent](https://docs.aws.amazon.com/wellarchitected/latest/framework/rel_prevent_interaction_failure_idempotent.html)
  - relevant because campaign resume/retry depends on replay-safe behavior and `skip_done` posture
- [Background Tasks with Hosted Services in ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/host/hosted-services)
  - relevant because campaign work is long-running and needs clear progress, cancellation, and restart boundaries
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the campaign path is a structured frontend/backend orchestration workflow
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because summary envelopes and batch requests are persisted structured contracts

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

## Checklist

- [ ] Document the current frontend campaign orchestration responsibilities in `metaService.ts`.
- [ ] Document dataset compatibility and unsupported-strategy checks.
- [ ] Document progress-listener and task-store interaction.
- [ ] Document current summary persistence and last-summary ownership.
- [ ] Document retry/resume hooks and their dependence on the saved summary.
- [ ] Preserve tests for campaign strategy filtering, resume/retry, and progress behavior.
- [ ] Preserve backend campaign tests in `commands_campaign.rs`.
- [ ] Treat major campaign-service refactors as compatibility-sensitive because they affect UI, tasking, and recovery at once.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/taskStoreCampaignProgress.test.ts`
- run targeted campaign tests in `src-tauri/src/commands_campaign.rs`

## Rollout / Fallback / Compatibility Notes

- preserve current summary-recovery compatibility while campaign architecture is still service-centric
- if the repo later gains richer summary browsing, add it without breaking current last-summary recovery flows
- keep unsupported-strategy exclusions explicit until there is deliberate campaign support for those families

## Related Docs

- `REPO_Improvement/08_ml/27_meta_batch_registry_modes_2026-04-09.md`
- `REPO_Improvement/08_ml/28_meta_batch_resume_retry_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
