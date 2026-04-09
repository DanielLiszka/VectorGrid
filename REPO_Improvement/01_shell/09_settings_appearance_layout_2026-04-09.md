# Settings Appearance Layout Plan

Last updated: 2026-04-09

## Purpose

Define the current appearance and shell-layout settings contract so theme, window effect, help mode, density, and layout defaults can be improved without breaking the current preference/bootstrap model.

## Scope Boundaries

In scope:

- appearance and shell-layout controls in `SettingsWorkspacePanel.vue`
- theme, window effect, help mode, density mode, workflow advanced, and controls-collapsed preferences
- preflight and bootstrap behavior for shell appearance/layout state

Out of scope:

- saved perspectives
- desktop-notification settings
- Tauri app-settings persistence for market-data directories

## Current User-Facing Behavior

Settings already exposes a coherent appearance/layout surface:

- theme: `light`, `dark`, `system`
- window effect: `none`, `mica`, `acrylic`, `vibrancy`
- help mode: `full`, `concise`, `off`
- density mode: `comfortable`, `standard`, `dense`
- layout defaults for advanced controls and collapsed controls panel

Current behavior already spans multiple phases of startup:

- `preflight.ts` sets root dataset values before the app mounts
- settings bootstrap replays persisted values into the store
- `settingsStore` applies root dataset attributes and native window-effect commands
- Settings UI updates the same store-backed preferences interactively

So this surface is already a real shell-preference system, not just form state.

## Current Code Surface

- `web/src/components/SettingsWorkspacePanel.vue`
  - appearance and layout controls plus explanatory copy
- `web/src/components/SettingsWorkspacePanel.test.ts`
  - verifies visible settings controls and operator-facing copy
- `web/src/stores/settingsStore.ts`
  - normalizes, persists, and applies theme/effect/help/density/layout flags
- `web/src/services/settingsBootstrap.ts`
  - applies persisted settings via typed store actions
- `web/src/preflight.ts`
  - pre-applies theme/effect/workflow/density/help shell attributes before mount
- `web/src/services/settingsService.ts`
  - routes native window-effect changes and directory-opening actions
- `web/src/services/settingsService.test.ts`
  - verifies native settings service command mapping

## Current Verification Surface

- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/services/settingsBootstrap.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/services/settingsService.test.ts`

## Strengths Already Present

- appearance/layout state already has a preflight path to reduce post-mount flicker
- settings changes are normalized and store-backed
- root dataset attributes already provide a consistent shell-wide contract
- native window effects are routed through a dedicated service instead of ad hoc calls
- defaults are intentionally removed from storage where possible, which keeps persisted state leaner

## Weaknesses / Failure Modes

- preflight, bootstrap, and live store application are all part of the same preference system, but that three-stage contract is not documented in one place
- theme/effect/help/density interactions are easy to change without updating the operator-facing copy
- native window-effect availability is platform-sensitive, but the settings contract does not yet spell that out explicitly
- layout flags and appearance flags live together in Settings, which makes ownership easy to blur

## Regressions To Avoid

- increasing theme/effect flicker by moving changes out of preflight/bootstrap
- mixing frontend shell-preference persistence with Tauri app-settings persistence
- breaking root dataset attributes that existing shell styling depends on
- weakening default-removal semantics in storage and creating noisy persisted state
- changing help or density behavior without updating the current copy and tests
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MDN `localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because appearance/layout state is persisted locally and restored at boot
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because appearance/layout settings are already centralized in store-backed state
- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because native window-effect changes cross the frontend-to-backend boundary
- [Pinia core concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because current settings behavior is already modeled as normalized store state rather than component-local state
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Freeze the appearance/layout contract as a three-stage system:
   - preflight
   - bootstrap
   - live settings updates
2. Separate preference families explicitly:
   - appearance
   - help/guidance
   - density
   - layout defaults
3. Preserve the current root-dataset contract and default-removal storage behavior.
4. Keep native window-effect behavior service-backed and platform-aware.
5. If layout settings expand later, keep them inside the current shell-preference model instead of creating a second settings path.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document theme, effect, help, density, and layout-default settings separately.
- [ ] Document preflight, bootstrap, and live-update responsibilities.
- [ ] Document root dataset attributes used by the shell.
- [ ] Document native window-effect behavior and its service boundary.
- [ ] Add or tighten tests if appearance or layout copy/behavior changes.
- [ ] Preserve the separation from Tauri app-settings persistence.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/components/SettingsWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/settingsBootstrap.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- run `npm --prefix web run test -- src/services/settingsService.test.ts`
- manually verify theme/effect/help/density changes and reload persistence
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep default values removable from storage unless a future migration requires explicit persistence
- preserve preflight behavior if appearance/layout state is reorganized later
- if platform-specific effect handling changes, keep the Settings UX explicit about what still applies locally
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/11_settings_storage_shortcuts_2026-04-08.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/22_tauri_settings_persistence_2026-04-08.md`
