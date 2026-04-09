# Shell Task Center Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the task-center UI surface so the floating launcher, expanded overlay, timeline/event/toast summaries, cancellation affordances, and keyboard/focus behavior remain clear, bounded, and non-disruptive.

## Scope Boundaries

In scope:

- `web/src/components/TaskCenterOverlay.vue`
- `useTaskCenterVisibility`
- launcher position persistence
- overlay interaction and focus management
- task/event/timeline/toast presentation in the shell

Out of scope:

- underlying task-tracking state semantics owned by `taskStore`
- feature-specific command execution logic
- options-related overlays

## Current User-Facing Behavior

The app already exposes a floating task-center launcher and expandable overlay. Current user-facing behavior includes:

- a floating launcher with persisted position
- quiet-mode behavior when there are no active tasks, no active progress, and no recent toasts
- an expanded panel with:
  - task posture summary
  - recent events
  - recent timeline entries
  - recent toasts
  - task cancellation actions where command mapping exists
- keyboard handling for:
  - `Escape`
  - `Tab` focus trapping
  - toggle open/close
- bounded visible slices:
  - recent events
  - recent timeline entries
  - recent toasts

The overlay also provides quick navigation actions into other surfaces, such as opening top results from relevant task-center UI affordances.

## Current Code Surface

- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/composables/useTaskCenterVisibility.ts`
- `web/src/composables/useAnchoredOverlay.ts`
- `web/src/stores/taskStore.ts`
- `web/src/services/runControlService.ts`
- `web/src/App.vue`

## Current Verification Surface

- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/stores/taskStore.lifecycle.test.ts`
- `web/src/stores/taskStoreEventThrottle.test.ts`
- `web/src/services/runControlService.test.ts`
- `web/src/tauri/errorPathQa.test.ts`

## Strengths Already Present

- task-center UI already exists as a dedicated overlay instead of being buried in settings/debug-only surfaces
- launcher position is persisted rather than hard-resetting every session
- focus-trap and escape-close behavior are already implemented
- cancellation is wired through mapped command authority rather than ad hoc button logic
- the panel already shows multiple levels of recency:
  - active posture
  - recent timeline
  - recent events
  - recent toasts

## Weaknesses / Failure Modes

- the UI contract for quiet mode versus expanded mode is still implicit
- launcher persistence can become stale or invalid if the viewport or overlay size changes
- truncation limits are practical but under-documented
- task-center copy and task-store state can drift if the presentation rules are not treated as a contract
- focus return and toggle-ignore behavior are subtle and easy to regress

## Regressions To Avoid

- breaking escape-close or tab trapping
- losing persisted launcher position or allowing it to drift off-screen
- showing stale cancellation state or miswired cancel affordances
- letting the overlay grow into an unbounded event dump
- bypassing the task center with duplicate shell alert surfaces that disagree with it
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Modal Dialog Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the expanded task-center overlay behaves like an interactive shell dialog with focus and close expectations
- [Listbox Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
  - relevant because task/event selection and keyboard movement patterns should stay consistent with accessible list-style interaction where applicable
- [ARIA: status role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because task-center summaries and progress text are status information, not just decoration
- [Window.localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because launcher position and related shell preferences are persisted in browser storage

## Proposed Improvement Approach

1. Document the task-center UI contract in layers:
   - launcher
   - overlay shell
   - active-task posture
   - recent events/timeline/toasts
   - cancellation affordances
2. Make quiet-mode rules explicit so the launcher behaves predictably when the app is idle.
3. Lock the focus/keyboard contract:
   - toggle open/close
   - escape close
   - focus trap
   - return focus on close
4. Define the persisted-position contract, including clamping and first-run initialization.
5. Treat recency truncation and category counts as deliberate design decisions rather than incidental slicing.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document launcher visibility, quiet-mode, and expanded-mode rules.
- [x] Document persisted launcher-position behavior and viewport clamping.
- [x] Document focus management, escape handling, and return-focus behavior.
- [x] Document recent event/timeline/toast truncation policy.
- [x] Document cancellation affordance rules and when they should appear.
- [x] Add tests for persisted-position recovery and off-screen clamping.
- [x] Add tests for focus trap and return-focus behavior.
- [x] Add tests for quiet-mode display and cancellation-path UI behavior.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/components/TaskCenterOverlay.test.ts`
- run `npm test -- src/stores/taskStore.lifecycle.test.ts`
- run `npm test -- src/stores/taskStoreEventThrottle.test.ts`
- run `npm test -- src/services/runControlService.test.ts`
- manually verify launcher drag/reposition, overlay open/close, and cancellation UI in the shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current launcher/overlay entrypoint rather than introducing competing task-center shells
- keep task-center improvements additive and contract-preserving first
- if deeper filtering or richer history is added later, keep current recent-summary behavior available as the default quick view
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/26_runtime_task_store_2026-04-08.md`
- `REPO_Improvement/01_shell/30_status_presentation_format_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`

