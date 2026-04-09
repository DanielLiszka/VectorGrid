# Shell Run Footer Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the run-footer shell surface so execution readiness, run/cancel controls, insights, and task summaries remain synchronized with the active workflow state without duplicating or contradicting panel-level status.

## Scope Boundaries

In scope:

- `web/src/components/RunFooterPanel.vue`
- footer visibility and expansion state
- workflow-mode-specific primary actions
- readiness, plan, insights, and tasks summary cards

Out of scope:

- optimize-service execution internals
- ML Signal execution internals beyond footer-facing mode differences
- options surfaces

## Current User-Facing Behavior

The run footer is mounted as a shell-level footer surface and is shown for the workflow workspace. Current behavior includes:

- persisted footer expansion state
- a compact mode and a detailed mode
- primary actions:
  - `Run optimization`
  - `Build score cache` in ML Signal mode
- cancel action bound to current run posture
- a live summary pill derived from:
  - active run progress
  - readiness badges
  - readiness hint
- detail cards for:
  - plan
  - readiness
  - insights
  - tasks

The footer already summarizes analysis and task state rather than only exposing buttons.

## Current Code Surface

- `web/src/components/RunFooterPanel.vue`
- `web/src/components/RunFooterPanel.test.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/stores/commandActionStore.ts`

## Current Verification Surface

- `web/src/components/RunFooterPanel.test.ts`
- optimize/task store tests that affect footer-facing state
- shell tests that verify workflow-only mounting and run-state posture

## Strengths Already Present

- the footer already consolidates run posture into one shell surface
- compact versus expanded behavior already exists
- ML Signal mode already changes the primary action truthfully
- task-center posture and analysis summaries already flow into the footer
- readiness badges and hints are already visible without opening a deeper panel

## Weaknesses / Failure Modes

- the footer consumes several stores, so cross-store drift is easy to introduce
- summary-pill and detailed-card rules are under-documented
- compact mode can become misleading if it hides too much state during active work
- the footer could duplicate or contradict optimize-panel/task-center status if ownership is not explicit
- the workflow-only visibility rule is easy to break in shell changes

## Regressions To Avoid

- showing incorrect primary action labels for ML Signal mode
- desynchronizing run/cancel disabled posture from the real workflow state
- contradicting optimize-panel status or task-center summary
- losing persisted expand/collapse state
- surfacing stale analysis or task summaries as if they were current

## External Research Summary

- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because footer summary/status text is live status information
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the footer is a composite consumer of multiple shared stores
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because the footer relies on stable store ownership rather than local duplicated state
- [Window.localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because footer expansion state is persisted across sessions

## Proposed Improvement Approach

1. Define footer behavior by workflow mode:
   - regular optimization mode
   - ML Signal mode
2. Document compact and expanded contracts separately:
   - what must remain visible in compact mode
   - what moves into detail cards
3. Make store ownership explicit:
   - optimize store owns run posture and readiness
   - analysis store owns insight summaries
   - task store owns task posture and overall progress
4. Treat the summary pill as a first-class contract item, not just leftover copy.
5. Expand tests around mode switches, compact/expanded persistence, and run/cancel posture.

## Checklist

- [ ] Document workflow-only footer visibility.
- [ ] Document compact versus expanded footer behavior.
- [ ] Document primary-action differences for standard versus ML Signal mode.
- [ ] Document summary-pill priority rules.
- [ ] Document plan/readiness/insights/tasks card ownership.
- [ ] Add tests for expansion persistence and shell visibility.
- [ ] Add tests for ML Signal primary-action labeling.
- [ ] Add tests for run/cancel posture and summary consistency during active runs.

## Verification Plan

- run `npm test -- src/components/RunFooterPanel.test.ts`
- run relevant optimize/task store tests that change footer-facing state
- manually verify compact/expanded behavior, ML Signal mode, and active-run state in the desktop shell

## Rollout / Fallback / Compatibility Notes

- preserve the current footer mount location and shell role
- keep compact mode available even if the footer becomes richer
- if more actions are added later, keep the current primary-action truth contract intact first

## Related Docs

- `REPO_Improvement/03_opt/12_opt_service_progress_2026-04-08.md`
- `REPO_Improvement/01_shell/05_shell_task_center_2026-04-08.md`
- `REPO_Improvement/01_shell/30_status_presentation_format_2026-04-08.md`

