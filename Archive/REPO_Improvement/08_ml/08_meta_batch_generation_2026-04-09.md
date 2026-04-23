# Meta Batch Generation Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because batch generation is a shared, long-running workflow with state carried across many controls.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because retry/resume and selection logic need stateful regression coverage.
- [Tauri event namespace](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because campaign progress and summary updates are long-running runtime events surfaced to the frontend.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because batch generation controls ultimately map into a substantial Rust campaign request.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because batch/campaign summaries and requests are structured operational artifacts, not opaque text blobs.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

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
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every batch-generation control and its request/summary effect.
- [x] Audit dataset selection, retry-failed, and resume-incomplete behavior for stale-state risk.
- [x] Add or tighten tests for execution mode, preload policy, and auto-meta/auto-stage5 combinations.
- [x] Add or tighten tests for summary reveal and resume flows.
- [x] Improve help text and status messaging where batch controls currently assume too much user context.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- run targeted campaign command tests in `commands_campaign.rs` if request semantics change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current batch capabilities and recovery flows
- prefer clearer copy, grouping, and tests over hiding controls
- if a UI cleanup makes retry/resume harder to trust, restore the more explicit current controls and refine incrementally
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_2026-04-09.md`
- `REPO_Improvement/08_ml/06_meta_panel_ia_2026-04-09.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`

## Execution Notes

- Closed by adding an explicit operating-mode summary to the batch tab instead of hiding or removing controls.
- `MetaWorkspacePanel.vue` now surfaces a compact batch summary grid for:
  - execution mode
  - preload policy
  - automation mode
  - selected dataset count
- The recovery controls (`Retry failed`, `Resume incomplete`, `Reveal summary`) remain visible and unchanged; the improvement is contextual clarity, not workflow simplification.
- `MetaWorkspacePanel.test.ts` now verifies the batch operating summary against an `optimize_only` + VRAM-staging scenario while preserving the recovery-action surface.
- `metaServiceCampaignResume.test.ts` and `metaServiceCampaignStrategy.test.ts` remain part of the acceptance evidence for batch resume/strategy semantics.
- `REPO_MAP.md` was updated so contributors can find the batch-mode summary behavior quickly.
- Stable fixture / replay payload:
  - batch state with `batchExecutionMode = optimize_only`, `batchPreloadPolicy = host_plus_doublema_vram`, `batchSelectedCount = 2`, `batchTotalCount = 5`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts src/services/metaServiceCampaignResume.test.ts src/services/metaServiceCampaignStrategy.test.ts src/stores/analysisStore.actions.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
