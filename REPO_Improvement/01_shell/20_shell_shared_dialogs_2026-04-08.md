# Shell Shared Dialogs Plan

Last updated: 2026-04-08

## Purpose

Define the current shared dialog contract so confirmation and shell-modal behavior can be improved without regressing focus management, escape handling, or current user safeguards.

## Scope Boundaries

In scope:

- shared confirmation dialog behavior
- focus trapping and focus return behavior in shell dialogs
- relationship between the shared confirmation primitive and shell-specific dialog surfaces

Out of scope:

- command-palette search logic
- task-center content model
- options-specific dialogs

## Current User-Facing Behavior

The shell already uses dialog semantics in multiple places:

- `ConfirmationDialog` as the shared alert/confirmation primitive
- titlebar close guard using that shared confirmation dialog
- task center as its own dialog surface
- command palette as its own modal command surface

The shared reusable piece is currently `ConfirmationDialog`. Its behavior already includes:

- `alertdialog` semantics
- escape-to-cancel
- focus trapping
- focus return to the invoking element
- configurable confirm/cancel labels
- danger-state confirm styling

## Current Code Surface

- `web/src/components/ConfirmationDialog.vue`
  - shared dialog primitive for confirm/cancel flows
- `web/src/components/DesktopTitlebar.vue`
  - uses the confirmation dialog for guarded close behavior
- `web/src/components/DesktopTitlebar.test.ts`
  - verifies close-guard behavior that depends on the dialog
- `web/src/components/TaskCenterOverlay.vue`
  - separate dialog surface with its own focus/escape logic
- `web/src/components/TaskCenterOverlay.test.ts`
  - verifies task-center dialog behavior
- `web/src/components/CommandPaletteOverlay.vue`
  - modal command surface implemented with dialog primitives

## Current Verification Surface

- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/CommandPaletteOverlay.test.ts`

## Strengths Already Present

- the shared confirmation primitive already has real focus and escape handling
- the close-guard flow already depends on it in a user-visible way
- shell dialogs already return focus appropriately in key flows
- dialog semantics are already present rather than deferred to future cleanup
- confirm/cancel copy and danger styling are already parameterized

## Weaknesses / Failure Modes

- shared versus non-shared dialog behavior is not documented in one place
- only the confirmation primitive is truly shared today; other shell dialogs have their own local patterns
- future contributors could clone dialog logic instead of strengthening the shared primitive
- focus-trap and focus-return behavior can regress through template changes without type errors
- there is no dedicated doc that defines when a shell modal should reuse the shared confirmation primitive

## Regressions To Avoid

- breaking focus trap or focus return in the shared confirmation dialog
- losing escape-to-cancel behavior
- duplicating new confirmation-style dialogs instead of reusing the shared primitive
- weakening the titlebar close-guard flow that currently protects active work
- confusing alert-dialog confirmation flows with richer modal workflows like the command palette

## External Research Summary

- [WAI-ARIA Authoring Practices: dialog (modal)](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the shared confirmation primitive already behaves as a modal dialog with focus management
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because dialog-triggering flows are store-backed and should remain easy to verify
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because dialog open/close state should remain explicit rather than hidden in DOM-only logic
- [WAI-ARIA Authoring Practices: combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
  - relevant because shell modal surfaces differ: confirmation dialogs should not be conflated with command-palette behavior

## Proposed Improvement Approach

1. Document the shared confirmation dialog as the reusable alert/confirm primitive.
2. Record which shell modals are shared versus shell-specific.
3. Preserve focus/escape/focus-return behavior as first-class contract items.
4. Prefer reuse of the shared confirmation primitive for new confirm/cancel flows instead of duplicating alert-dialog logic.
5. Keep shell-specific modal surfaces free to stay specialized when their interaction model differs from simple confirmation.

## Checklist

- [ ] Document the current shared confirmation-dialog contract.
- [ ] Document which shell dialog surfaces are shared versus specialized.
- [ ] Preserve focus trap, escape cancel, and focus return behavior.
- [ ] Add or tighten dialog regressions anywhere a new shared confirm flow is added.
- [ ] Reuse the shared confirmation primitive for simple alert/confirm flows instead of cloning logic.

## Verification Plan

- run `npm --prefix web test -- src/components/DesktopTitlebar.test.ts`
- run `npm --prefix web test -- src/components/TaskCenterOverlay.test.ts`
- run `npm --prefix web test -- src/components/CommandPaletteOverlay.test.ts`
- manually verify focus entry, escape, confirm, cancel, and focus return in the close-guard flow

## Rollout / Fallback / Compatibility Notes

- preserve the shared confirmation dialog API unless a migration is deliberate
- keep specialized modals specialized when they are not simple confirm/cancel flows
- if more shared dialog primitives are introduced later, document their scope boundaries explicitly
- update this doc and related tests whenever shell dialog semantics change materially

## Related Docs

- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/04_shell_command_palette_2026-04-08.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-08.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-08.md`
