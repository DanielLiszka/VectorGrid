# Browser Shell UX Smoke Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WAI-ARIA tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) is relevant because the shell workspace and workflow-step bars behave as tabbed navigation surfaces.
- [Playwright locators](https://playwright.dev/docs/locators) are relevant because shell smoke should keep using stable role- and attribute-oriented selectors.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because this spec should verify the user-observable navigation contract rather than internal implementation details.
- [Playwright assertions](https://playwright.dev/docs/test-assertions) are relevant because automatic waiting is critical for keeping shell-navigation smoke stable without sleeps.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep browser shell smoke narrowly focused on keyboard navigation, selection state, and orientation after shell routing changes.
2. Define which workspace and workflow surfaces are contract-level assertions versus optional contextual assertions.
3. Prefer semantic selectors and built-in waiting over manual timing or deep DOM traversal.
4. Treat any growth in spec scope as a deliberate decision that must still fit the shell-level boundary.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the required shell-navigation assertions for workspace tabs.
- [x] Document the required shell-navigation assertions for workflow-step tabs.
- [x] Keep focus, selection, and panel visibility as first-class assertions.
- [x] Audit selectors so they stay stable and accessibility-oriented.
- [x] Avoid adding dense optimize/validate/analyze detail checks to this shell smoke spec.
- [x] Keep the spec non-options only.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run test:browser:ux` after shell-routing or tab-behavior changes
- review failure traces when selection/focus behavior changes unexpectedly
- verify that any changed selectors remain shared-safe with the desktop harness
- confirm the shell-ready helper still guarantees the app has mounted before navigation begins
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current keyboard contract while refining assertion quality
- if shell IA changes, update the contract intentionally rather than weakening the test to “make it pass”
- split genuinely separate shell behaviors into additional docs/specs instead of bloating this one
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on current-state browser-shell evidence; no new Playwright spec change was required for this item.
- Required workspace-tab assertions are currently:
  - keyboard movement from workflow -> library -> aggregate -> pro -> settings
  - focus transfer to the newly selected tab
  - `aria-selected="true"` on the active workspace tab
  - orientation proof through the matching visible panel or shell surface (`#results_panel_history`, `#results_panel_meta`, settings grid)
  - `Home` returning focus and selection to workflow
- Required workflow-step assertions are currently:
  - initial `data` step selected
  - keyboard movement through `data -> optimize -> validate -> analyze`
  - focus follows the active step
  - optimize step still exposes the expected controls/results routing surfaces
  - validate step still exposes the validation results panel
  - analyze step still leaves a visible active results-tab state
- Selector audit outcome:
  - the spec stays on accessibility-oriented or state-oriented selectors (`aria-selected`, focus, `data-workspace`, `data-workflow-step`, shared shell selectors)
  - shared selectors remain reused from the desktop harness without introducing decorative style hooks
  - the lone explicit `pro` workspace selector remains bounded and readable without widening the selector surface elsewhere
- Scope remains correct:
  - no dense results-table assertions were added here
  - no form-detail behavior was pulled in from optimize/validate/analyze surfaces
  - non-options only
- Cross-doc sync:
  - no `README.md` or `REPO_MAP.md` update was required because the shell smoke contract already matched the current spec; this closure records fresh verification against the current selector/state posture
- Acceptance evidence:
  - `npm.cmd --prefix web run test:browser:ux`
  - the browser shell UX journey passed in the built mocked-Tauri harness during this tranche
  - supporting harness-level evidence from adjacent browser docs:
    - `npm.cmd --prefix web run a11y:smoke`
    - `npm.cmd --prefix web run baseline:ui`

## Related Docs

- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/11_browser_palette_overlays_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/14_desktop_shell_readiness_2026-04-09.md`
