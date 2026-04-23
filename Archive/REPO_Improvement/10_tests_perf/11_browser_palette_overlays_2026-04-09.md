# Browser Palette and Overlay Smoke Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA dialog (modal) pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) is relevant because both overlay surfaces need predictable focus containment and Escape-close behavior.
- [WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/) is relevant because the command palette uses an active-descendant driven result list.
- [Playwright locators](https://playwright.dev/docs/locators) are relevant because overlay smoke should keep using stable role- and aria-oriented selectors.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because overlay smoke should stay focused on user-observable outcomes instead of component internals.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep browser overlay smoke centered on open, search, select, dismiss, and focus-return behavior.
2. Prefer section-presence assertions for the task center over brittle full-copy snapshots.
3. Use component tests for dense overlay rendering rules and keep browser smoke at shell-journey level.
4. Document the small set of stable commands and overlay sections that are worth treating as contract-level smoke.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the minimal stable command-palette journey owned by browser smoke.
- [x] Document the minimal stable task-center overlay journey owned by browser smoke.
- [x] Keep active-descendant and focus-return assertions explicit.
- [x] Keep overlay selectors accessible and stable.
- [x] Avoid turning overlay smoke into a generic command-action integration suite.
- [x] Keep the browser overlay suite non-options only.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run test:browser:ux` after overlay, shell, or command-palette changes
- follow failures into component/store tests when the browser smoke points to a narrower regression
- review traces to confirm focus, dialog visibility, and command execution order remain sane
- verify that overlay tests continue to start from the mocked shell bootstrap path
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current overlay journeys while tightening the contract around what is intentionally asserted
- add new overlay journeys only when the user-visible behavior cannot be covered cleanly at component level
- if the task center or command palette IA changes, update the assertion hierarchy instead of just loosening expectations
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on current-state browser overlay evidence; no new spec or helper change was required for this item.
- Minimal stable command-palette journey currently owned by browser smoke:
  - open from the shell launcher
  - type a broad query and confirm expected result visibility
  - move active descendant with arrow keys
  - execute the stable `Go to Settings` path with Enter
  - confirm dialog dismissal, workspace selection, and focus return to the opener
  - separate Escape-close path keeps focus restoration explicit without coupling it to command execution
- Minimal stable task-center journey currently owned by browser smoke:
  - open from the launcher
  - confirm stable section presence (`Active posture`, `Recent outcomes`)
  - close with Escape
  - confirm focus return to the launcher and `aria-expanded="false"`
- Selector/state contract remains healthy:
  - dialog, listbox, active descendant, `aria-expanded`, focus, and shared shell selectors stay first-class
  - no decorative CSS selectors were introduced
  - the spec still avoids deep command-action correctness or broad overlay rendering detail
- Scope remains disciplined:
  - non-options only
  - browser smoke continues to treat the overlays as the subject, not their downstream service/store internals
  - denser overlay rendering rules remain owned by component tests for `CommandPaletteOverlay` and `TaskCenterOverlay`
- Cross-doc sync:
  - no `README.md` or `REPO_MAP.md` update was required because the overlay browser contract already matched the current spec and helper posture; this closure records fresh verification against the current journeys
- Acceptance evidence:
  - `npm.cmd --prefix web run test:browser:ux`
  - supporting adjacent overlay/component evidence from the same tranche:
    - `npm.cmd --prefix web run test -- src/components/CommandPaletteOverlay.test.ts src/components/StartupAnimationOverlay.test.ts`
    - `npm.cmd --prefix web run test -- src/components/TaskCenterOverlay.test.ts src/components/VueWorkspaceShell.test.ts src/App.test.ts`

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/10_browser_shell_ux_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/12_browser_support_mocks_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/16_desktop_task_center_nav_2026-04-09.md`
