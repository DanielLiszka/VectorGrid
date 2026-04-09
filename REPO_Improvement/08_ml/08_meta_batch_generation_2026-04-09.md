## Purpose

Define the improvement plan for the Meta Batch Generation surface so campaign-style batch selection, retry/resume, preload policy, and auto-meta/stage5 behavior stay understandable and operationally safe.

## Scope Boundaries

In scope:

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `src-tauri/src/commands_campaign.rs`
- batch dataset selection, execution-mode/preload-policy controls, retry/resume flows, and summary reveal behavior

Out of scope:

- full campaign execution internals outside the surfaced controls and outputs
- legacy batch-summary compatibility plumbing outside how the UI consumes it
- options-related batch workflows

## Current User-Facing Behavior

The Meta workspace already provides a real batch-generation console:

- dataset search and selection
- execution-mode control
- preload-policy control
- auto-meta and auto-stage5 toggles
- skip-done, retry-failed, resume-incomplete, and reveal-summary flows
- batch summary preview and counts

This is already more than a simple “run batch” button. The challenge is making it feel deterministic and trustworthy rather than dense and partially magical.

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `src-tauri/src/commands_campaign.rs`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`

## Current Verification Surface

- `MetaWorkspacePanel.test.ts`
- `metaService.test.ts`
- `metaServiceCampaignResume.test.ts`
- `metaServiceCampaignStrategy.test.ts`
- backend command tests in `commands_campaign.rs`

## Strengths Already Present

- batch generation is already a first-class tab with meaningful controls
- retry/resume and reveal-summary flows already exist
- dataset selection and summary state are already modeled explicitly in the service
- the frontend already understands current campaign summary kinds
- tests already cover important campaign/batch UI-service behavior

## Weaknesses / Failure Modes

- the control set is dense and can be hard to interpret without strong labeling
- auto-meta and auto-stage5 imply chained downstream behavior that users may not fully understand
- retry/resume behavior relies heavily on summary state and selected paths
- execution mode, preload policy, and skip-done semantics are operationally important but easy to gloss over

## Regressions To Avoid

- changing batch control meaning without corresponding summary/status updates
- letting retry/resume operate on stale or ambiguous dataset selection
- weakening reveal-summary or resume affordances while reorganizing the tab
- making advanced controls less transparent in the name of simplification

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because batch generation is a shared, long-running workflow with state carried across many controls.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because retry/resume and selection logic need stateful regression coverage.
- [Tauri event namespace](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because campaign progress and summary updates are long-running runtime events surfaced to the frontend.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because batch generation controls ultimately map into a substantial Rust campaign request.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because batch/campaign summaries and requests are structured operational artifacts, not opaque text blobs.

## Proposed Improvement Approach

1. Treat batch generation as an operational console with explicit workflow stages.
2. Document each control by its effect on the campaign request and summary behavior.
3. Tighten the UI explanation for:
   - execution mode
   - preload policy
   - auto-meta
   - auto-stage5
   - skip-done
   - retry/resume
4. Ensure batch summary preview and action buttons always reflect the current campaign state truthfully.
5. Add tests for important control combinations and state transitions rather than only single actions.

## Checklist

- [ ] Document every batch-generation control and its request/summary effect.
- [ ] Audit dataset selection, retry-failed, and resume-incomplete behavior for stale-state risk.
- [ ] Add or tighten tests for execution mode, preload policy, and auto-meta/auto-stage5 combinations.
- [ ] Add or tighten tests for summary reveal and resume flows.
- [ ] Improve help text and status messaging where batch controls currently assume too much user context.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- run targeted campaign command tests in `commands_campaign.rs` if request semantics change

## Rollout / Fallback / Compatibility Notes

- preserve current batch capabilities and recovery flows
- prefer clearer copy, grouping, and tests over hiding controls
- if a UI cleanup makes retry/resume harder to trust, restore the more explicit current controls and refine incrementally

## Related Docs

- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
- `REPO_Improvement/08_ml/06_meta_panel_ia_2026-04-09.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
