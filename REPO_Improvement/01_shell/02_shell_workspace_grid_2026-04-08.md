# Shell Workspace Grid Plan

Last updated: 2026-04-12

## Purpose

Define the current `VueWorkspaceShell.vue` layout, navigation, and split-grid contract so shell structure can be improved without breaking posture-specific panel routing or keyboard navigation.

## Scope Boundaries

In scope:

- shell header and context strip
- workspace/mode/workflow-step navigation
- controls/results columns and split layout
- keyboard navigation and panel host routing

Out of scope:

- deep internals of mounted workspace panels
- titlebar behavior
- options-specific shell branches

## Current User-Facing Behavior

`VueWorkspaceShell.vue` already defines the current shell posture. Current behavior includes:

- top-level workspace tabs for `Workflow`, `Run Library`, `Aggregate Analysis`, `Pro Tools`, `Live`, and `Settings`
- mode tabs for `Conventional` and `ML Signal`
- workflow-step tabs when in the workflow workspace
- a context strip with default and overflow context cards
- a task-center launcher in the shell header
- controls and results columns that appear only when the current workspace/mode/step supports them
- split-layout behavior for selected conventional workflow postures, with keyboard-accessible and pointer-accessible resizing
- alternate dedicated layouts for `live` and `settings`, with execution promoted out of the settings shell

The component already acts as the main shell layout authority, not just visual chrome.

## Current Code Surface

- `web/src/components/VueWorkspaceShell.vue`
  - owns workspace/mode/step tabs, context strip, split layout, column visibility, resize behavior, and shell mount hosts
- `web/src/components/VueWorkspaceShell.test.ts`
  - verifies navigation posture, ML Signal shell differences, context cards, collapsed controls, keyboard navigation, and tab semantics
- `web/src/stores/workflowStore.ts`
  - drives workspace, mode, workflow-step, control-tab, result-tab, and split-fraction state
- `web/src/stores/settingsStore.ts`
  - drives `controlsCollapsed`
- `web/src/composables/useTaskCenterVisibility.ts`
  - drives task-center open state used by the shell launcher

## Current Verification Surface

- `web/src/components/VueWorkspaceShell.test.ts`
- related store tests for `workflowStore` and `settingsStore`
- panel-specific tests for mounted child surfaces

## Strengths Already Present

- shell posture is centralized in one component
- keyboard navigation already follows tablist semantics across workspace, mode, step, control, and result tabs
- the split divider is keyboard accessible and pointer resizable
- ML Signal posture is already treated as a real shell variant rather than a cosmetic tweak
- settings gets a separate layout path instead of being forced through the workflow grid

## Weaknesses / Failure Modes

- the shell component is large and encodes several different layout modes
- visible controls/results rules are valuable but implicit
- split-layout eligibility is specific and easy to regress accidentally
- context-card selection between default and overflow sets is not documented outside the component/tests

## Regressions To Avoid

- breaking tab keyboard navigation or tabpanel wiring
- showing controls/results columns in shell postures that should hide them
- regressing ML Signal’s different step labels and hidden conventional results chrome
- breaking split-layout resizing, clamping, or persisted split behavior
- turning the context strip into stale or contradictory state compared with the mounted panels
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
  - relevant because the shell uses tablist semantics extensively and tests already reflect that contract
- [MDN CSS Grid Layout](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout)
  - relevant because the shell grid relies on grid-template columns, stacked fallbacks, and responsive layout switching
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because shell layout is driven by centralized workflow/settings state
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because the shell should remain a consumer of store truth rather than growing its own competing state model

## Proposed Improvement Approach

1. Freeze the shell posture model:
   - workspace
   - mode
   - workflow step
   - visible controls/results columns
2. Document the layout branches:
   - conventional workflow
   - ML Signal workflow
   - library
   - aggregate
   - pro
   - live
   - settings
3. Preserve keyboard navigation and tab semantics as first-class contract items.
4. Document split-layout eligibility and resizing rules explicitly.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document workspace/mode/step routing and which layouts they imply.
- [x] Document controls/results visibility rules per shell posture.
- [x] Document context-strip default versus overflow behavior.
- [x] Document split-layout eligibility, clamping, keyboard control, and pointer resize behavior.
- [x] Preserve tablist/tab/tabpanel semantics and keyboard coverage.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- run related `workflowStore` and `settingsStore` tests
- manually verify conventional, ML Signal, library, settings, and pro shell postures
- manually verify split resize and collapsed-controls behavior
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep shell posture routing store-backed and declarative
- if the shell component is later split, preserve its current routing semantics before rearranging markup
- maintain current keyboard semantics even if the visuals evolve
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/01_shell_app_root_2026-04-08.md`
- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/11_settings_storage_shortcuts_2026-04-08.md`
- `REPO_Improvement/01_shell/08_shell_docks_shortcuts_2026-04-08.md`
