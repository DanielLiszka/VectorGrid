# Shell Command Palette Plan

Last updated: 2026-04-09

## Purpose

Define the current command-palette contract so global command discovery, ranking, keyboard control, and command routing can be improved without breaking the existing shell-wide action surface.

## Scope Boundaries

In scope:

- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/stores/commandPaletteStore.ts`
- command-palette search/ranking and keyboard behavior
- routing from palette commands into existing stores and actions

Out of scope:

- global shortcut registration itself
- deep implementation of the commands the palette invokes
- options-specific commands

## Current User-Facing Behavior

The app already ships a real command palette:

- it opens as a modal overlay from the shell
- it groups commands into `actions`, `data`, `exports`, `layout`, and `navigation`
- it resets query and selection state on open and close
- it focuses the query input when opened
- it supports fuzzy search while still preferring direct name-prefix matches for short queries
- it exposes combobox/listbox semantics and keyboard navigation
- it routes commands into existing store actions rather than duplicating business logic

Current command coverage already includes:

- run / cancel / validate
- browse CSV / open Polygon downloader
- export top CSV / JSON
- open latest report / open latest meta report
- toggle controls panel
- workflow-step and workspace navigation

So the palette is already a shell command surface, not a placeholder.

## Current Code Surface

- `web/src/components/CommandPaletteOverlay.vue`
  - dialog structure, grouping, Fuse-based search, prefix ranking, keyboard handling, and command execution
- `web/src/components/CommandPaletteOverlay.test.ts`
  - verifies focus, Escape close, combobox/listbox semantics, fuzzy ranking, prefix preference, and Home/End navigation
- `web/src/stores/commandPaletteStore.ts`
  - owns `open`, `query`, and `selectedIndex`
- `web/src/stores/commandActionStore.ts`
  - provides routed commands for data, run, export, report-open, and layout actions
- `web/src/stores/workflowStore.ts`
  - provides workflow-step and workspace navigation targets

## Current Verification Surface

- `web/src/components/CommandPaletteOverlay.test.ts`
- `web/src/stores/commandActionStore.test.ts`
- shell-level interaction through `web/src/components/DesktopTitlebar.test.ts`

## Strengths Already Present

- the palette is already wired into existing store actions instead of creating shadow logic
- ranking already balances prefix matches and typo-tolerant fuzzy search
- accessibility semantics are already tested directly
- the command inventory is grouped and intentionally shell-oriented
- query and selection reset behavior already prevents stale state across openings

## Weaknesses / Failure Modes

- command inventory and ranking logic live inside the component, which makes future expansion dense
- there is no separate implementation doc that freezes the command categories and ranking rules
- enabled/disabled command visibility rules are implicit
- command coverage can drift without an obvious inventory review
- routing spans multiple stores, so palette regressions can appear as “random action failures” if not documented

## Regressions To Avoid

- turning the palette into a second implementation surface for existing commands
- weakening current combobox/listbox semantics or keyboard behavior
- losing direct-prefix preference for short queries
- silently dropping major command groups or navigation targets
- retaining stale query/selection state across palette reopenings
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the palette is currently implemented as a modal overlay that must keep focus and escape behavior predictable
- [WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
  - relevant because command results are presented as a keyboard-navigable option list
- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because the palette should continue dispatching into existing action owners
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because shell command surfaces benefit from realistic end-to-end interaction checks beyond narrow unit tests

## Proposed Improvement Approach

1. Freeze the palette contract in four parts:
   - command inventory ownership
   - ranking/search behavior
   - keyboard and accessibility behavior
   - action routing
2. Consider extracting the command inventory and ranking helper into pure modules if the surface grows further.
3. Keep the palette as a router:
   - no duplicated run/export/navigation logic
   - no command-specific business logic in the overlay
4. Preserve visibility of disabled commands only if their disabled posture remains explainable and useful.
5. Add explicit inventory review whenever new shell commands are added.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current command groups and command inventory owners.
- [ ] Document prefix-priority and fuzzy-search behavior separately.
- [ ] Document combobox/listbox semantics and keyboard controls.
- [ ] Document how command routing delegates into existing stores.
- [ ] Add or tighten tests if the command inventory or ranking rules change.
- [ ] Keep open/close query reset behavior stable.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/CommandPaletteOverlay.test.ts`
- run `npm --prefix web run test -- src/stores/commandActionStore.test.ts`
- manually verify open, search, selection, Enter execute, and Escape close behavior from the desktop shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the palette as a shell-level action surface, not a new app mode
- if command inventory moves out of the component later, keep existing ranking and grouping behavior through regression fixtures
- if global shortcuts change, keep the palette inventory and shortcut hints synchronized
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/25_command_palette_store_2026-04-08.md`
- `REPO_Improvement/01_shell/08_shell_docks_shortcuts_2026-04-08.md`
- `REPO_Improvement/01_shell/15_state_artifacts_commands_2026-04-09.md`
