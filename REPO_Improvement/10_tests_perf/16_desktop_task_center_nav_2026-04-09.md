# Desktop Task Center Navigation Plan

Last updated: 2026-04-09

## Purpose

Define the real-shell desktop-smoke contract for opening, inspecting, and dismissing the task center in the native runtime.

## Scope Boundaries

In scope:

- the `desktop-task-center.spec.ts` smoke contract
- task-center open/close behavior in the real Tauri shell
- stable section-presence assertions for task-center content
- the `TaskCenterOverlay.vue` component and visibility composable as the primary surface under test

Out of scope:

- deep task lifecycle correctness owned by store and service tests
- broad runtime diagnostics content owned by settings diagnostics
- options-related overlays
- performance measurement of overlay rendering

## Current User-Facing Behavior

The current desktop task-center smoke spec launches the shell, opens the task center, verifies that its core sections are present, and closes it again in the real desktop runtime. The corresponding component persists launcher position, handles focus return, renders posture/timeline sections, and exposes a dialog-style overlay. This makes the task center a distinct real-shell navigation and overlay contract rather than a generic content surface.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-task-center.spec.ts` contains the current desktop smoke journey.
- `web/src/components/TaskCenterOverlay.vue` owns the overlay DOM, launcher behavior, and section layout.
- `web/src/composables/useTaskCenterVisibility.ts` owns open/close state and return-focus behavior.
- `web/e2e/desktop/support/waits.ts` and `selectors.ts` provide the desktop-harness open/close helpers and selectors.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke:spec -- .\e2e\desktop\specs\desktop-task-center.spec.ts`
- the default desktop smoke suite
- component tests for `TaskCenterOverlay.vue` when a failure points to internal rendering or visibility logic

## Strengths Already Present

- the desktop spec validates a real-shell overlay that browser smoke can only partially approximate
- the component already has explicit launcher, dialog, and visibility semantics rather than ad hoc DOM toggles
- the smoke assertions are section-oriented and therefore relatively stable
- the current spec proves both open and close behavior in the native shell

## Weaknesses / Failure Modes

- the overlay content is rich, so it is easy to over-assert on copy rather than contract-level sections
- desktop smoke cannot cheaply cover every task-center branch and still remain stable
- the task center mixes overlay behavior with live task-status presentation, which can complicate failure triage
- launcher positioning/persistence exists but is not yet part of this smoke contract

## Regressions To Avoid

- reducing task-center smoke to a single visibility check without confirming meaningful sections
- turning the spec into a full task-state integration matrix
- asserting on too much volatile copy
- forgetting focus-return and close semantics while only checking content presence

## External Research Summary

- [WAI-ARIA dialog (modal) pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) is relevant because the task center behaves like a focusable overlay dialog with Escape-close semantics.
- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because desktop overlay checks need resilient selectors and built-in waits.
- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant by analogy because this spec should stay user-visible and journey-oriented rather than implementation-driven.
- [Tauri state management guidance](https://v2.tauri.app/develop/state-management/) is relevant because runtime and task information are surfaced across the desktop shell boundary.

## Proposed Improvement Approach

1. Keep desktop task-center smoke centered on open, section presence, and close behavior in the real shell.
2. Use component/store tests for dense task-state permutations instead of growing the desktop spec.
3. Assert on stable section headings and dialog state rather than incidental copy.
4. Treat launcher persistence and richer task semantics as adjacent concerns unless they become a proven user-facing regression.

## Checklist

- [ ] Document the minimum task-center sections that must appear in desktop smoke.
- [ ] Document the required open and close behavior in the native shell.
- [ ] Keep the spec tied to stable dialog and launcher selectors.
- [ ] Avoid widening the spec into full task-data validation.
- [ ] Keep the task-center desktop smoke non-options only.
- [ ] Cross-check failures against the overlay component and visibility composable before broadening the test.

## Verification Plan

- run the targeted task-center spec after overlay or shell changes
- run the full desktop smoke suite after broader shell changes
- use component tests when a failure points to rendering logic instead of native runtime behavior
- review artifact screenshots when task-center visibility or section rendering breaks

## Rollout / Fallback / Compatibility Notes

- preserve the current open/close journey while clarifying the section contract
- add deeper task-center assertions only when a real user-facing regression requires them
- keep launcher drag/persistence separate unless it is promoted intentionally into smoke coverage

## Related Docs

- `REPO_Improvement/10_tests_perf/11_browser_palette_overlays_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/17_desktop_settings_diag_2026-04-09.md`
