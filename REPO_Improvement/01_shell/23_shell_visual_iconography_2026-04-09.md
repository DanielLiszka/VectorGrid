# Shell Visual Iconography Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the shared non-options shell iconography system so asset-backed visuals, Lucide fallbacks, and illustration usage stay coherent without regressing current workflows or accessibility.

## Scope Boundaries

In scope:

- non-options SVG asset inventory in `appSvgAssets.ts`
- fallback icon mapping in `iconography.ts`
- `AppVisualIcon.vue` rendering behavior and alt/decorative rules
- current shell/result/task/artifact icon usage in shared panels

Out of scope:

- options-specific icon assets and options workspace illustrations
- full visual redesign of the application
- adding brand-new workflow surfaces

## Current User-Facing Behavior

The app already ships a mixed iconography system rather than raw ad hoc icons. `web/src/utils/appSvgAssets.ts` defines generated SVG asset specs for shell posture, task state, artifacts, compare actions, runtime readiness, provenance cues, and empty-state illustrations. `web/src/utils/iconography.ts` maps task levels, task statuses, artifact actions, and compare actions onto either asset icons or Lucide fallbacks. `web/src/components/AppVisualIcon.vue` is the shared renderer: it emits an `<img>` for asset specs and a component render path for Lucide icons, while preserving a decorative default and allowing alt text for non-decorative cases.

Current usage is spread across real surfaces:

- `VueWorkspaceShell.vue` shows the shell posture icon
- `TaskCenterOverlay.vue` and `SettingsWorkspacePanel.vue` show task and level icons
- `TopWorkspacePanel.vue`, `OverviewWorkspacePanel.vue`, and `MetaWorkspacePanel.vue` use compare/artifact icons and empty-state illustrations
- `AppRuntimeBanner.vue` uses the runtime-ready versus error posture icon

That means the icon layer is already part of product behavior, not just styling.

## Current Code Surface

- `web/src/utils/appSvgAssets.ts`
- `web/src/utils/iconography.ts`
- `web/src/components/AppVisualIcon.vue`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/OverviewWorkspacePanel.vue`

## Current Verification Surface

- `web/src/components/AppVisualIcon.test.ts`
- `web/src/components/VueWorkspaceShell.test.ts`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`

## Strengths Already Present

- icon definitions are already centralized instead of duplicated inline across templates
- `AppVisualIcon.vue` already gives the app one rendering boundary for asset versus component icons
- important shell states already have dedicated visuals instead of generic defaults
- asset-backed empty states and provenance cues already improve scanability in review surfaces

## Weaknesses / Failure Modes

- non-options and options icon assets currently live in the same registry file, which makes scope discipline easy to lose
- there is no direct implementation doc that freezes which surfaces should use asset icons versus Lucide fallbacks
- shared icon accessibility can still drift if contributors bypass `AppVisualIcon.vue` semantics in consumers
- icon copy, asset labels, and usage semantics can drift if contributors treat visuals as purely decorative

## Regressions To Avoid

- dropping existing task/artifact/shell posture icons back to generic placeholders
- breaking decorative versus non-decorative alt behavior in `AppVisualIcon.vue`
- mixing options icon work into the non-options improvement lane
- changing visual meaning for task or artifact actions without updating tests and usage guidance together
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because shared UI semantics such as status and action meaning are easiest to keep stable when the icon layer is centralized.
- [MDN prefers-color-scheme](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-color-scheme) is relevant because icon assets and fallbacks need to remain legible across the app theme modes already exposed in settings.
- [MDN prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion) is relevant because decorative shell visuals should not quietly reintroduce motion-heavy behavior when the app already respects reduced-motion settings elsewhere.
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Write a non-options icon inventory that separates shell posture, runtime, task center, artifact, compare, provenance, and empty-state visuals from the out-of-scope options lane.
2. Freeze the current rendering contract for `AppVisualIcon.vue`, including asset-spec detection, decorative defaults, and non-decorative alt usage.
3. Define when a surface should use a dedicated asset icon, a Lucide fallback, or no icon at all.
4. Add direct verification for shared icon rendering and keep one small usage matrix for the major shell panels.
5. Document any places where current asset labels or icon meanings are overloaded and should be normalized later without removing functionality.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- added direct component coverage in `web/src/components/AppVisualIcon.test.ts` for asset-backed and Lucide/component-backed icons
- froze the shared renderer contract in `web/src/components/AppVisualIcon.vue` so non-decorative component-path icons carry an accessible name and `role="img"` while decorative icons remain hidden from assistive technology
- verified representative shell consumers through `SettingsWorkspacePanel.test.ts`, `DesktopTitlebar.test.ts`, and the shared command/task shell test bundle
- left the shared asset registry intact to avoid widening scope into options ownership refactors during the non-options lane

## Checklist

- [x] Inventory all non-options asset icons and the shell/result surfaces that use them.
- [x] Document the current asset-versus-Lucene/Lucide fallback rule set.
- [x] Add a direct test plan for `AppVisualIcon.vue` asset and component rendering.
- [x] Separate non-options icon ownership from out-of-scope options icons at the documentation level.
- [x] Document alt-text and decorative-image expectations for shell icons and empty-state illustrations.
- [x] Record any places where current icon meanings or labels are inconsistent.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted frontend tests for shell and task-center surfaces that currently assert icon presence
- add direct component coverage for `AppVisualIcon.vue` before changing render semantics
- manually inspect the shell, task center, overview, top, and meta panels after any iconography refactor
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- treat icon changes as user-facing behavior, not cosmetic cleanup
- prefer additive documentation and test coverage before moving assets or renaming labels
- keep current visuals in place unless a replacement is clearly safer and already covered
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/30_status_presentation_format_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
