# Shell Debug Workspace Plan

Last updated: 2026-04-09

## Purpose

Define the current debug-workspace contract so raw troubleshooting output can be improved without contaminating the main analysis flow or breaking the existing debug shell posture.

## Scope Boundaries

In scope:

- `DebugWorkspacePanel.vue`
- debug-shell posture in `VueWorkspaceShell.vue` and `workflowStore.ts`
- analysis-store debug-output preview and presentation

Out of scope:

- full developer tooling redesign
- backend debug logging systems
- options-specific debug panels

## Current User-Facing Behavior

The shell already has a dedicated debug surface:

- `resultsTab = debug` maps into the debug workspace posture
- `App.vue` only mounts the panel when `debugPanelActive` is true
- the debug panel teleports into a dedicated shell mount host
- it presents raw analysis JSON for troubleshooting
- it is intentionally separated from the main review flow
- it retains status and timestamp context

The current product meaning is explicit: this is an operator/developer troubleshooting surface, not the main analysis workflow.

## Current Code Surface

- `web/src/components/DebugWorkspacePanel.vue`
  - debug panel layout, raw JSON preview, timestamp, and status output
- `web/src/App.vue`
  - mounts the debug panel only when the debug shell state is active
- `web/src/stores/analysisStore.ts`
  - owns `debugOutputPreview`, status, and update timestamp
- `web/src/stores/workflowStore.ts`
  - treats debug as part of the shell results/workspace grammar
- `web/src/components/VueWorkspaceShell.vue`
  - defines the debug mount host and `pro` workspace posture
- `web/src/composables/useShellMount.ts`
  - mount-host readiness for the teleported debug panel

## Current Verification Surface

- `web/src/components/DebugWorkspacePanel.test.ts`
- indirect coverage through `web/src/components/VueWorkspaceShell.test.ts`
- analysis-store tests that protect debug-output state indirectly

## Strengths Already Present

- the debug surface is already isolated from the main review flow
- the panel is simple and clearly scoped
- raw payload visibility is already available for troubleshooting without overloading normal result surfaces
- debug workspace posture is already modeled in the shell grammar

## Weaknesses / Failure Modes

- there is no dedicated component test for the debug panel
- raw JSON is useful, but the contract for when it should exist is not documented in one place
- debug workspace semantics depend on workflow-store and shell posture rules that are easy to miss if the panel is read in isolation
- because the panel is intentionally minimal, it can be accidentally neglected in refactors

## Regressions To Avoid

- leaking debug content into the main analysis flow
- removing the dedicated debug workspace posture from the shell grammar silently
- breaking debug panel mount-host readiness
- losing raw payload visibility when troubleshooting is needed most
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the debug panel is a consumer of centralized analysis state, not a source of separate truth
- [MDN ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because the panel already exposes status-like troubleshooting updates
- [Vue performance best practices](https://vuejs.org/guide/best-practices/performance.html)
  - relevant because the debug panel should stay lightweight and isolated
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because shell-posture regressions affecting the debug workspace are easier to catch through higher-level interaction tests
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Freeze the debug workspace as a separate troubleshooting surface.
2. Document the current ownership split:
   - workflow store owns debug posture
   - shell owns mount host
   - analysis store owns raw payload content
3. Add a dedicated `DebugWorkspacePanel` test.
4. Keep the panel intentionally simple and separate from the main review experience.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current debug workspace posture and mount path.
- [x] Document the source of `debugOutputPreview`.
- [x] Add direct component coverage for `DebugWorkspacePanel.vue`.
- [x] Preserve separation between debug troubleshooting and the main analysis flow.
- [x] Keep debug shell posture synchronized with workflow-store grammar.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/DebugWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- run relevant analysis-store tests if debug-output behavior changes
- manually verify the debug workspace still mounts and shows raw output in the `pro` posture
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the debug workspace available even if most users never use it; it is a troubleshooting tool
- if richer debug tooling is added later, preserve the current raw JSON escape hatch until parity exists
- keep debug output separate from user-facing result narratives
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/02_shell_workspace_grid_2026-04-08.md`
- `REPO_Improvement/01_shell/16_ux_shell_mount_readiness_2026-04-09.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-09.md`
- `REPO_Improvement/04_results/17_analysis_store_service_2026-04-08.md`
