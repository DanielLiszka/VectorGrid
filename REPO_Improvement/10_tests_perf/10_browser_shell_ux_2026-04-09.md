# Browser Shell UX Smoke Plan

Last updated: 2026-04-09

## Purpose

Pin the deterministic shell-navigation browser smoke contract around workspace and workflow-step navigation in mocked-Tauri mode.

## Scope Boundaries

In scope:

- keyboard workspace navigation in `web/e2e/browser/shell-ux.spec.ts`
- workflow-step navigation and orientation assertions in the browser harness
- shell-ready expectations owned by `gotoMockedShell()` and `waitForShellReady()`
- ARIA-selected and focus-return semantics for the top-level shell tabs

Out of scope:

- options-related workflow steps or workspaces
- dense per-control assertions better covered by component tests
- desktop-native window behavior
- visual baseline capture and screenshot review

## Current User-Facing Behavior

The current browser shell smoke spec opens the mocked shell, walks through workspace tabs with keyboard input, verifies `aria-selected` and focus transitions, and confirms that the expected panel regions and result surfaces appear as navigation moves from workflow to library, aggregate, pro, settings, and across workflow steps. It already serves as the shell-level keyboard contract for the non-options browser layer, but it needs a stronger implementation plan so future shell work does not dilute or overextend it.

## Current Code Surface

- `web/e2e/browser/shell-ux.spec.ts` contains the current keyboard shell-journey assertions.
- `web/e2e/browser/support/appHarness.ts` owns mocked-shell boot and shell-ready gating.
- `web/e2e/desktop/support/selectors.ts` provides shared selectors reused by both browser and desktop harnesses.
- `web/src/components/VueWorkspaceShell.vue` and related shell components provide the user-visible tab and panel behavior under test.

## Current Verification Surface

- `npm --prefix web run test:browser:ux -- shell-ux` via the configured Playwright suite
- the broader `npm --prefix web run test:browser:ux` command
- Playwright failure traces and screenshots retained by `playwright.config.ts`

## Strengths Already Present

- the spec already tests keyboard movement, focus, selected state, and visible-surface orientation instead of only click-through behavior
- the browser harness is deterministic enough to make these shell assertions reliable
- the selectors target accessible/tab state rather than brittle styling hooks
- the spec exercises the actual workspace and workflow routing shape exposed to users

## Weaknesses / Failure Modes

- it is still easy to add too much incidental layout detail to what should remain a shell-navigation contract
- the spec currently assumes a particular set of visible panels and may become noisy if shell IA evolves without a documented assertion hierarchy
- browser smoke cannot prove native focus or desktop event delivery
- the shared selectors come from the desktop-support layer, so careless changes can break both harnesses at once

## Regressions To Avoid

- replacing focus and `aria-selected` assertions with weaker text-only checks
- overfitting the test to transient copy or decorative markup
- using arbitrary pauses where explicit readiness or built-in assertions are available
- letting shell smoke absorb result-table or form-detail coverage that belongs in other tests

## External Research Summary

- [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because the shell workspace and workflow-step bars behave as tabbed navigation surfaces.
- [Playwright locators](https://playwright.dev/docs/locators) are relevant because shell smoke should keep using stable role- and attribute-oriented selectors.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because this spec should verify the user-observable navigation contract rather than internal implementation details.
- [Playwright assertions](https://playwright.dev/docs/test-assertions) are relevant because automatic waiting is critical for keeping shell-navigation smoke stable without sleeps.

## Proposed Improvement Approach

1. Keep browser shell smoke narrowly focused on keyboard navigation, selection state, and orientation after shell routing changes.
2. Define which workspace and workflow surfaces are contract-level assertions versus optional contextual assertions.
3. Prefer semantic selectors and built-in waiting over manual timing or deep DOM traversal.
4. Treat any growth in spec scope as a deliberate decision that must still fit the shell-level boundary.

## Checklist

- [ ] Document the required shell-navigation assertions for workspace tabs.
- [ ] Document the required shell-navigation assertions for workflow-step tabs.
- [ ] Keep focus, selection, and panel visibility as first-class assertions.
- [ ] Audit selectors so they stay stable and accessibility-oriented.
- [ ] Avoid adding dense optimize/validate/analyze detail checks to this shell smoke spec.
- [ ] Keep the spec non-options only.

## Verification Plan

- run `npm --prefix web run test:browser:ux` after shell-routing or tab-behavior changes
- review failure traces when selection/focus behavior changes unexpectedly
- verify that any changed selectors remain shared-safe with the desktop harness
- confirm the shell-ready helper still guarantees the app has mounted before navigation begins

## Rollout / Fallback / Compatibility Notes

- preserve the current keyboard contract while refining assertion quality
- if shell IA changes, update the contract intentionally rather than weakening the test to “make it pass”
- split genuinely separate shell behaviors into additional docs/specs instead of bloating this one

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/11_browser_palette_overlays_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/14_desktop_shell_readiness_2026-04-09.md`
