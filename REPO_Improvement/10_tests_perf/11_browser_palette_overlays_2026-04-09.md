# Browser Palette and Overlay Smoke Plan

Last updated: 2026-04-09

## Purpose

Define the browser-level smoke contract for command-palette and task-center overlay behavior in mocked-Tauri mode.

## Scope Boundaries

In scope:

- command palette open, search, active-descendant, command execution, and Escape-close behavior
- task center open/close and focus-return smoke behavior in the browser harness
- browser smoke around overlay focus restoration and `aria-expanded` state
- shared shell overlay helpers used by the browser harness

Out of scope:

- desktop-native overlay rendering or window focus behavior
- deep command-action correctness already covered by unit/service tests
- options-related palette commands or overlays
- broad visual or screenshot comparison work

## Current User-Facing Behavior

The current browser overlay spec exercises the command palette from the shell opener, confirms command search results and active-descendant movement, executes a settings-navigation command, then checks that focus returns to the opener. It also opens the task center, checks for its core sections, closes it with Escape, and verifies focus is restored. This is already a meaningful shell-level overlay contract, but the exact scope needs to stay disciplined so the spec does not devolve into a broad command-action integration suite.

## Current Code Surface

- `web/e2e/browser/command-palette-overlays.spec.ts` holds the current overlay browser smoke journeys.
- `web/e2e/browser/support/appHarness.ts` exports the open/close helpers used by the spec.
- `web/src/components/CommandPaletteOverlay.vue` owns the actual command palette DOM, search, and selection behavior.
- `web/src/components/TaskCenterOverlay.vue` and `web/src/composables/useTaskCenterVisibility.ts` own task-center dialog visibility and focus return.
- `web/src/stores/commandPaletteStore.ts` drives palette query, open state, and selected index behavior under the component layer.

## Current Verification Surface

- `npm --prefix web run test:browser:ux`
- Playwright retained traces and failure screenshots
- targeted component tests for `CommandPaletteOverlay` and `TaskCenterOverlay` when the smoke points to a narrower regression

## Strengths Already Present

- the spec already validates user-observable focus and dismissal behavior instead of just component presence
- the browser harness keeps overlay behavior deterministic enough to test keyboard semantics cleanly
- the current assertions center on stable aria/state signals such as `aria-expanded`, active descendant, and dialog visibility
- overlay smoke bridges the gap between isolated component tests and the real desktop shell

## Weaknesses / Failure Modes

- overlay behavior spans components, stores, and shell helpers, so responsibility can blur without a written contract
- browser smoke cannot prove native focus quirks that only appear in the real Tauri shell
- it is easy to keep adding commands to the palette scenario until the spec becomes an integration catch-all
- task-center content assertions can become noisy if they depend on incidental copy rather than stable sections

## Regressions To Avoid

- dropping focus-return checks, which are the main value of this overlay smoke layer
- testing command-action internals here instead of treating the overlay as the primary subject
- replacing semantic dialog/listbox assertions with brittle styling selectors
- using mock data or mock events that are broader than the current browser harness contract requires

## External Research Summary

- [WAI-ARIA dialog (modal) pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) is relevant because both overlay surfaces need predictable focus containment and Escape-close behavior.
- [WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/) is relevant because the command palette uses an active-descendant driven result list.
- [Playwright locators](https://playwright.dev/docs/locators) are relevant because overlay smoke should keep using stable role- and aria-oriented selectors.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because overlay smoke should stay focused on user-observable outcomes instead of component internals.

## Proposed Improvement Approach

1. Keep browser overlay smoke centered on open, search, select, dismiss, and focus-return behavior.
2. Prefer section-presence assertions for the task center over brittle full-copy snapshots.
3. Use component tests for dense overlay rendering rules and keep browser smoke at shell-journey level.
4. Document the small set of stable commands and overlay sections that are worth treating as contract-level smoke.

## Checklist

- [ ] Document the minimal stable command-palette journey owned by browser smoke.
- [ ] Document the minimal stable task-center overlay journey owned by browser smoke.
- [ ] Keep active-descendant and focus-return assertions explicit.
- [ ] Keep overlay selectors accessible and stable.
- [ ] Avoid turning overlay smoke into a generic command-action integration suite.
- [ ] Keep the browser overlay suite non-options only.

## Verification Plan

- run `npm --prefix web run test:browser:ux` after overlay, shell, or command-palette changes
- follow failures into component/store tests when the browser smoke points to a narrower regression
- review traces to confirm focus, dialog visibility, and command execution order remain sane
- verify that overlay tests continue to start from the mocked shell bootstrap path

## Rollout / Fallback / Compatibility Notes

- preserve the current overlay journeys while tightening the contract around what is intentionally asserted
- add new overlay journeys only when the user-visible behavior cannot be covered cleanly at component level
- if the task center or command palette IA changes, update the assertion hierarchy instead of just loosening expectations

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/12_browser_support_mocks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/16_desktop_task_center_nav_2026-04-09.md`
