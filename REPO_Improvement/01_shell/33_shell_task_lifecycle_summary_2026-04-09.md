# Shell Task Lifecycle Summary Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the shared task-summary composable and its status-presentation dependencies so shell headlines, run-footer status, and settings diagnostics stay aligned with the task store without regressing current live feedback.

## Scope Boundaries

In scope:

- `useTaskLifecycleSummary.ts`
- status aggregation drawn from `taskStore.ts` and `statusPresentation.ts`
- shell consumers such as Run Footer, Settings, and Vue workspace shell
- task-summary wording and high-level posture presentation

Out of scope:

- rewriting the full Task Center store
- new task categories not already tracked
- options-specific status surfaces

## Current User-Facing Behavior

`useTaskLifecycleSummary.ts` already provides a shared high-level task summary for the shell. It derives active tasks, active-task count, center-status label, latest task, overall progress label, latest task outcome, and a headline string from `taskStore.ts` plus `statusPresentation.ts`. `RunFooterPanel.vue`, `SettingsWorkspacePanel.vue`, and `VueWorkspaceShell.vue` already consume that derived view instead of manually reconstructing task posture.

Current visible behavior therefore includes:

- a shared shell-level notion of active-task count and posture
- run-footer messaging driven by the same aggregate task picture used elsewhere
- settings diagnostics that reference the latest task outcome and task center posture
- task wording normalized through `statusPresentation.ts` rather than duplicated inline

## Current Code Surface

- `web/src/composables/useTaskLifecycleSummary.ts`
- `web/src/stores/taskStore.ts`
- `web/src/utils/statusPresentation.ts`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/TaskCenterOverlay.vue`

## Current Verification Surface

- `web/src/stores/taskStore.lifecycle.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `web/src/stores/taskStoreEventThrottle.test.ts`
- `web/src/components/RunFooterPanel.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`

## Strengths Already Present

- task summary derivation is already centralized in one composable
- the task store already carries enough information to drive queue/running/cancelled/success/failure posture
- status wording already flows through shared presentation helpers instead of hard-coded text in each panel
- multiple shell consumers already depend on the same summary path, which is the right direction

## Weaknesses / Failure Modes

- there is no dedicated implementation doc that freezes which summary signals belong in shell-wide posture versus Task Center detail
- status wording, tone mapping, and summary-headline behavior are spread across the composable and `statusPresentation.ts`
- the current shell summary is correct enough to ship but still easy to drift if contributors add consumer-specific formatting
- there is no direct test for the composable itself as a boundary independent of its consumers

## Regressions To Avoid

- letting the shell summary and Task Center drift into contradictory states
- breaking run-footer or settings posture by changing summary wording casually
- dropping active-task count or overall-progress semantics from shared shell surfaces
- creating new component-local task summaries instead of reusing the shared one
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because task summary correctness depends on deriving shared shell posture from one store rather than duplicating logic per component.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the repo already uses a store-plus-derived-consumer pattern for long-lived task state.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant indirectly because the task center already persists parts of its history and timeline, so summary semantics should remain compatible with persisted state.
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Document which pieces of task posture belong in the shell summary versus Task Center detail.
2. Freeze the contract between `taskStore.ts`, `statusPresentation.ts`, and `useTaskLifecycleSummary.ts`.
3. Add direct verification for the composable boundary or strengthen consumer tests so summary drift is easier to catch.
4. Keep summary text concise and shared so Run Footer, Settings, and shell header posture stay aligned.
5. Document how background task types and progress-bearing commands are expected to influence the summary.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document every value returned by `useTaskLifecycleSummary.ts` and where it is consumed.
- [ ] Document which `taskStore.ts` fields the summary depends on.
- [ ] Document how `statusPresentation.ts` participates in tone and wording normalization.
- [ ] Add or tighten tests for active-task count, latest outcome, and overall-progress label behavior.
- [ ] Record the rule that shell consumers should not fork their own summary logic.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted task-store lifecycle and progress tests
- run `npm --prefix web run test -- src/components/RunFooterPanel.test.ts`
- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- prefer documentation and test hardening before refactoring summary wording or return shape
- keep the Task Center as the detailed source of truth even if the shell summary becomes clearer
- maintain backward compatibility for existing consumers until all summary paths are migrated together
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/06_shell_run_footer_2026-04-08.md`
- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
- `REPO_Improvement/01_shell/30_status_presentation_format_2026-04-08.md`
