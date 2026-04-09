# Shell Titlebar Plan

Last updated: 2026-04-09

## Purpose

Define the current `DesktopTitlebar.vue` contract so desktop chrome, command access, primary action routing, and close-guard behavior can be improved without breaking the current native-window posture.

## Scope Boundaries

In scope:

- desktop titlebar identity and subtitle
- palette/settings/primary action buttons
- native window controls
- close-guard logic and close-confirm dialog

Out of scope:

- deeper workflow shell layout
- task-center overlay internals
- tray behavior or hide-to-tray changes

## Current User-Facing Behavior

The titlebar is already a meaningful part of the product:

- it shows `VectorGrid`, current workspace, and workflow-step labeling
- it shows a subtitle derived from the current shell posture
- it offers three primary tools: command palette, settings, and run/stop
- it exposes native minimize/maximize/restore/close window controls when the Tauri runtime is available
- it uses drag surfaces and double-click maximize behavior
- closing the window may be guarded when the live runner or protected long-running tasks are active
- the close confirmation explicitly states that close exits the app instead of minimizing it

So the titlebar already defines part of the desktop product behavior, not just decoration.

## Current Code Surface

- `web/src/components/DesktopTitlebar.vue`
  - titlebar identity, command/tool buttons, native window bindings, close guards, drag behavior
- `web/src/components/DesktopTitlebar.test.ts`
  - verifies workspace labeling, ML Signal labels, store action routing, native controls, close guarding, and drag behavior
- `web/src/stores/workflowStore.ts`
  - provides workspace and workflow-step state
- `web/src/stores/optimizeStore.ts`
  - provides run/stop readiness state
- `web/src/stores/taskStore.ts`
  - provides guarded active-task information
- `web/src/stores/executionStore.ts`
  - provides live-runner guard state
- `web/src/tauri/invoke.ts`
  - exposes runtime-availability checks

## Current Verification Surface

- `web/src/components/DesktopTitlebar.test.ts`
- related store tests for task, optimize, execution, and workflow state
- shell-level smoke through the root app and shell tests

## Strengths Already Present

- titlebar behavior is already tied to real app state rather than static labels
- native controls are safely disabled when the desktop runtime is unavailable
- close guarding already distinguishes guarded long-running work from short non-guarded tasks
- the primary action already flips cleanly between run and stop
- titlebar drag/maximize behavior is implemented without hiding the current app-exit semantics

## Weaknesses / Failure Modes

- the close-guard contract is rich but not documented outside the component/tests
- titlebar identity and primary-action logic live alongside native window bindings, making the surface dense
- desktop runtime availability is critical but easy to misunderstand if someone reads only the template
- the titlebar has responsive behavior that is not yet documented as contract

## Regressions To Avoid

- losing the current workspace/workflow-step identity labeling
- breaking the palette/settings/run-stop store routing
- weakening close guarding for live runner or protected tasks
- changing close semantics toward hidden minimize/tray behavior accidentally
- breaking native drag, maximize, or window-control disable behavior
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri window JavaScript API](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
  - relevant because the titlebar directly uses current-window minimize, maximize, focus, drag, and close APIs
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the titlebar is part of the frontend/desktop integration boundary and must degrade safely when runtime access is absent
- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because the titlebar’s shell labels and actions sit above tab-driven shell posture and should not contradict it
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the titlebar should remain a store-backed consumer of shell/runtime state

## Proposed Improvement Approach

1. Freeze the titlebar contract into four layers:
   - identity and subtitle
   - shell actions
   - native window controls
   - close guard behavior
2. Preserve the current exit posture:
   - close exits the app
   - minimize is a separate action
   - guarded work requires explicit confirmation
3. Keep runtime availability and guarded-close logic explicit and test-backed.
4. If the titlebar is refactored later, keep the store-driven action routing intact.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document titlebar identity and subtitle sources.
- [ ] Document palette/settings/run-stop action routing.
- [ ] Document native window-control availability and behavior.
- [ ] Document close-guard triggers and confirmation copy expectations.
- [ ] Preserve drag and double-click maximize behavior.
- [ ] Preserve responsive titlebar behavior at smaller widths.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/DesktopTitlebar.test.ts`
- manually verify titlebar actions, drag surfaces, native controls, and guarded close in desktop runtime
- manually verify responsive behavior under narrower widths
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current “close exits” product posture unless a separate product decision changes it
- keep native controls gracefully disabled outside desktop runtime
- if close guarding rules change, update the tests and this doc in the same change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
