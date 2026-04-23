# Command Palette Store Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the command-palette store so open/close state, query resets, bounded selection, and keyboard-driven navigation remain stable across titlebar, shortcut, and overlay consumers.

## Scope Boundaries

In scope:

- `web/src/stores/commandPaletteStore.ts`
- command-palette open/query/selection state
- overlay/titlebar/global-shortcut consumers of that store
- selection-bound and reset behavior

Out of scope:

- the full palette command inventory itself
- unrelated shell dialogs
- options-specific command surfaces

## Current User-Facing Behavior

The command palette already behaves like a small focused state machine. Current behavior includes:

- `open` boolean state
- `query` text state
- `selectedIndex` state
- `openPalette()` resetting query and selection
- `closePalette()` resetting query and selection
- `setQuery()` resetting selection to the top
- `setSelection()` clamping to valid bounds
- `moveSelection()` shifting selection within valid bounds
- `clampSelection()` repairing invalid selection after result-count changes

Consumers already include the command-palette overlay, desktop titlebar entrypoints, and global shortcut handling.

## Current Code Surface

- `web/src/stores/commandPaletteStore.ts`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/CommandPaletteOverlay.test.ts`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/GlobalShortcutBridge.test.ts`

## Current Verification Surface

- `web/src/stores/commandPaletteStore.test.ts`
- `web/src/components/CommandPaletteOverlay.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/GlobalShortcutBridge.test.ts`

## Strengths Already Present

- the command palette already uses a dedicated store instead of component-local state
- reset behavior on open/close and query change is explicit
- selection is clamped rather than allowed to drift out of range
- store scope is intentionally small and understandable

## Weaknesses / Failure Modes

- the store contract is simple enough that future changes might bypass it casually
- selection semantics can drift if overlay behavior starts mutating raw indices directly
- query-reset behavior is easy to “optimize away” without realizing it changes UX
- the store does not yet document whether selection persistence across sessions is intentionally unsupported

## Regressions To Avoid

- persisting stale query or selection unexpectedly across open/close
- allowing negative or out-of-range selection indices
- bypassing the store from overlay/titlebar code
- making query changes preserve an invalid selection target
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because the palette store is a clean example of small, explicit shared state
- [Pinia Actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because all palette state transitions should stay action-driven
- [Modal Dialog Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the command palette is presented as an overlay/dialog shell
- [Listbox Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
  - relevant because keyboard-driven result selection should remain compatible with list-like interaction expectations

## Proposed Improvement Approach

1. Document the palette store as a three-field state machine:
   - open state
   - query state
   - bounded selection state
2. Make reset rules explicit:
   - open resets query/selection
   - close resets query/selection
   - query change resets selection
3. Treat bounded selection as a hard invariant, not a convenience helper.
4. Document which consumers may mutate palette state and which must stay read-only.
5. Add focused tests around result-count changes and keyboard-driven movement.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- added direct store-level coverage in `web/src/stores/commandPaletteStore.test.ts` for open/close resets, selection clamping, selection movement, and query-reset behavior
- verified the overlay consumer remains keyboard-safe through `CommandPaletteOverlay.test.ts`, with `DesktopTitlebar.test.ts` and `GlobalShortcutBridge.test.ts` covering the other mutation entrypoints
- left the palette state intentionally ephemeral and bounded; no persistence or feature expansion was introduced

## Checklist

- [x] Document the palette store state inventory and invariants.
- [x] Document open/close/query reset behavior explicitly.
- [x] Document selection clamping and movement rules.
- [x] Document which shell consumers are allowed to mutate the store.
- [x] Add tests for open/close reset behavior.
- [x] Add tests for selection clamping on result-count shrink.
- [x] Add tests for keyboard navigation through the overlay consumer.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/components/CommandPaletteOverlay.test.ts`
- run `npm test -- src/components/DesktopTitlebar.test.ts`
- run `npm test -- src/components/GlobalShortcutBridge.test.ts`
- manually verify open, search, arrow navigation, and close behavior in the shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current store shape unless there is a strong compatibility reason to change it
- keep palette state ephemeral unless a deliberate persistence policy is introduced later
- if the palette grows richer, preserve the current bounded-selection/reset rules first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/04_shell_command_palette_2026-04-09.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/11_browser_palette_overlays_2026-04-09.md`

