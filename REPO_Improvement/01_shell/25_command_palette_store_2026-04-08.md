# Command Palette Store Plan

Last updated: 2026-04-08

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

## Checklist

- [ ] Document the palette store state inventory and invariants.
- [ ] Document open/close/query reset behavior explicitly.
- [ ] Document selection clamping and movement rules.
- [ ] Document which shell consumers are allowed to mutate the store.
- [ ] Add tests for open/close reset behavior.
- [ ] Add tests for selection clamping on result-count shrink.
- [ ] Add tests for keyboard navigation through the overlay consumer.

## Verification Plan

- run `npm test -- src/components/CommandPaletteOverlay.test.ts`
- run `npm test -- src/components/DesktopTitlebar.test.ts`
- run `npm test -- src/components/GlobalShortcutBridge.test.ts`
- manually verify open, search, arrow navigation, and close behavior in the shell

## Rollout / Fallback / Compatibility Notes

- preserve the current store shape unless there is a strong compatibility reason to change it
- keep palette state ephemeral unless a deliberate persistence policy is introduced later
- if the palette grows richer, preserve the current bounded-selection/reset rules first

## Related Docs

- `REPO_Improvement/01_shell/04_shell_command_palette_DATE.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_DATE.md`
- `REPO_Improvement/10_tests_perf/11_browser_palette_overlays_DATE.md`

