# Shell Shared Dialogs Plan

Last updated: 2026-04-09

## Purpose

Define the current shared confirmation-dialog contract so modal confirmations can be improved without breaking focus management, destructive-action copy, or cross-shell reuse.

## Scope Boundaries

In scope:

- `web/src/components/ConfirmationDialog.vue`
- shell and workspace surfaces that reuse the shared confirmation dialog
- focus, keyboard, and destructive-action behavior for shared confirmations

Out of scope:

- custom non-confirmation modals
- command palette dialog behavior
- options-specific confirmation surfaces

## Current User-Facing Behavior

The app already uses one shared confirmation dialog across multiple surfaces:

- desktop close guard in `DesktopTitlebar.vue`
- execution actions in `ExecutionWorkspacePanel.vue`
- history destructive actions in `HistoryWorkspacePanel.vue`
- meta-workspace confirmations in `MetaWorkspacePanel.vue`

Current behavior already includes:

- modal backdrop and open-state gating
- `alertdialog` semantics
- focus capture into the dialog
- Tab trapping inside the dialog
- Escape to cancel
- confirm/cancel labels
- optional danger styling
- focus restoration to the previously focused element on close

So the shell already has a reusable confirmation surface, not a collection of unrelated modal implementations.

## Current Code Surface

- `web/src/components/ConfirmationDialog.vue`
  - shared dialog structure, focus behavior, Escape handling, and confirm/cancel actions
- consumers:
  - `web/src/components/DesktopTitlebar.vue`
  - `web/src/components/ExecutionWorkspacePanel.vue`
  - `web/src/components/HistoryWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`
- component and consumer tests that indirectly verify dialog behavior through those surfaces

## Current Verification Surface

- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- there is currently no dedicated `ConfirmationDialog` unit test

## Strengths Already Present

- one shared dialog component already exists
- focus management is already built into the shared component
- Escape and Tab behavior are already implemented centrally
- cross-surface reuse reduces divergence in destructive-action UX
- danger styling is already standardized

## Weaknesses / Failure Modes

- dialog behavior is verified mostly through consumers instead of a direct component test
- destructive copy expectations are spread across consumer components
- there is no one doc that defines what all shared confirmations must guarantee
- future consumers could bypass the shared component and introduce drift

## Regressions To Avoid

- breaking focus trap or focus restore behavior
- weakening Escape-to-cancel behavior
- diverging destructive-action copy or button posture across shell surfaces
- adding new confirmation flows without using the shared component
- changing `alertdialog` semantics accidentally
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  - relevant because the current shared dialog already behaves like a modal focus-managed confirmation
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because dialog consumers are store-backed and still need direct regression coverage
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because dialog open/close behavior should remain driven by stateful surfaces rather than ad hoc DOM manipulation
- [Playwright best practices](https://playwright.dev/docs/best-practices)
  - relevant because confirmation flows benefit from realistic interaction verification

## Proposed Improvement Approach

1. Freeze the shared confirmation-dialog guarantees:
   - semantics
   - focus behavior
   - keyboard behavior
   - action labeling
2. Add a dedicated `ConfirmationDialog` component test so consumers are not the only regression layer.
3. Keep destructive confirmation wording consumer-specific but structurally standardized through the shared component.
4. Require new shell confirmations to use this shared surface unless there is a documented reason not to.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the shared confirmation-dialog guarantees.
- [ ] Add direct unit coverage for `ConfirmationDialog.vue`.
- [ ] Document current consumer surfaces and their use of the shared component.
- [ ] Preserve focus trap, focus restore, and Escape behavior.
- [ ] Keep new shell confirmations on the shared component path by default.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- add and run a dedicated `ConfirmationDialog` component test
- run `npm --prefix web run test -- src/components/DesktopTitlebar.test.ts`
- run `npm --prefix web run test -- src/components/ExecutionWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/HistoryWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the shared component the default path for confirmation UX
- if a new modal type diverges later, document why it is not a shared confirmation
- preserve current keyboard and focus guarantees even if styling changes
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/03_shell_titlebar_2026-04-08.md`
- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-09.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
