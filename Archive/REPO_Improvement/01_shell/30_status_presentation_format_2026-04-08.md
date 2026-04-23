# Status Presentation Format Plan

Last updated: 2026-04-09

## Purpose

Define the current shell-wide status-presentation formatting layer so boot labels, task lifecycle labels, runtime descriptions, and status-oriented accessibility cues remain consistent across the non-options UI.

## Scope Boundaries

In scope:

- `web/src/utils/statusPresentation.ts`
- lifecycle-label formatting
- runtime-phase formatting
- task activity and task outcome text
- shell/status role usage where current components already depend on it

Out of scope:

- restyling the entire shell
- changing task lifecycle semantics themselves
- options-specific panels

## Current User-Facing Behavior

The app already normalizes status wording across multiple shell surfaces. Task-center and footer views use `taskLifecycleLabel`, `taskLifecycleTone`, and related helpers. The runtime banner and settings workspace use `runtimePhaseLabel` and `describeRuntimePhase`. Multiple panels also use `role="status"` and `aria-live="polite"` for changing status text. This means the repo already has a real status-presentation language, but it is only partly explicit.

## Current Code Surface

- `web/src/utils/statusPresentation.ts`
  - owns lifecycle tone/label formatting, runtime-phase formatting, task activity summaries, timeline/toast descriptions, and overall-progress label formatting
- `web/src/utils/statusPresentation.test.ts`
  - verifies label normalization and cross-surface formatting behavior
- `web/src/components/AppRuntimeBanner.vue`
  - renders formatted runtime status
- `web/src/components/TaskCenterOverlay.vue`
  - renders task status labels and task-center presentation tone
- `web/src/components/RunFooterPanel.vue`
  - consumes task lifecycle label helpers in the footer shell
- `web/src/components/SettingsWorkspacePanel.vue`
  - reuses runtime-phase formatting and task status summaries
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
  - current examples of status regions using `role="status"` / `aria-live="polite"`

## Current Verification Surface

- `web/src/utils/statusPresentation.test.ts`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`

## Strengths Already Present

- status wording is already normalized in one utility file instead of being duplicated everywhere
- task lifecycle values already map to consistent human-readable labels
- runtime-phase descriptions already include elapsed time and error formatting
- current components already use accessibility-oriented status regions in several places

## Weaknesses / Failure Modes

- it is easy for components to bypass `statusPresentation.ts` and hand-roll slightly different wording
- lifecycle labels and status tones are stable enough that silent wording drift can create UI inconsistency without obvious type errors
- `role="status"` usage exists across the repo, but the preferred pattern is not yet documented as a shell convention
- the line between terse status chips and longer descriptive status strings is implicit

## Regressions To Avoid

- introducing duplicate lifecycle-label logic in components or stores
- changing user-facing status wording inconsistently across shell surfaces
- weakening current `aria-live` / `role="status"` usage on existing status surfaces
- turning concise status helpers into component-specific ad hoc text formatting
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MDN: ARIA `status` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/status_role)
  - relevant because many shell surfaces already present evolving status text and should stay polite, non-disruptive live regions
- [MDN: ARIA `alert` role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/alert_role)
  - relevant because the repo should keep ordinary status updates distinct from genuinely urgent error announcements
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because formatted status output should consume shared state instead of duplicating it
- [Pinia: Getters](https://pinia.vuejs.org/core-concepts/getters.html)
  - relevant because status formatting often sits between raw store state and user-facing text
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Document the current status-formatting vocabulary:
   - lifecycle labels
   - lifecycle tones
   - runtime-phase labels
   - runtime-phase descriptions
   - task activity summaries
2. Distinguish raw state from presentation state so components know when they should call the formatter instead of inventing strings.
3. Record current accessibility expectations for status surfaces that already exist in the shell.
4. Treat wording consistency as a UX contract, not just cosmetic polish.
5. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- strengthened `web/src/utils/statusPresentation.test.ts` so lifecycle-tone and level-tone normalization are now covered directly alongside the existing wording-format tests
- verified the current status vocabulary remains aligned across `AppRuntimeBanner`, `TaskCenterOverlay`, `RunFooterPanel`, `SettingsWorkspacePanel`, and `VueWorkspaceShell`
- preserved the existing accessibility posture: normal shell status stays polite, while fatal runtime error presentation remains assertive in the banner surface

## Checklist

- [x] Enumerate the current format helpers and what each one is for.
- [x] Document the canonical lifecycle labels and tones.
- [x] Document runtime-phase labels and descriptive output behavior.
- [x] Document when to use terse labels versus full descriptive status strings.
- [x] Document current status-region accessibility expectations for existing shell surfaces.
- [x] Identify components that already rely on these helpers.
- [x] Add or tighten tests if wording, tone, or accessibility usage changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/utils/statusPresentation.test.ts`
- run `npm --prefix web run test -- src/components/TaskCenterOverlay.test.ts`
- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/components/VueWorkspaceShell.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep wording changes coordinated across all current consumers
- prefer central formatter updates over component-local string drift
- preserve existing accessible status-region behavior unless a better documented pattern replaces it
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/09_live/14_runtime_state_bridge_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/01_fe_test_matrix_2026-04-08.md`
