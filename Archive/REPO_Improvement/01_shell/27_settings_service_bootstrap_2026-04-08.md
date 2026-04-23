# Settings Service Bootstrap Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the boot-time settings bootstrap so persisted preferences are hydrated deterministically, cross-platform value catalogs stay aligned, and native side effects do not fire from ambiguous or partially normalized storage state.

## Scope Boundaries

In scope:

- local-storage settings hydration
- preflight/store/bootstrap ownership
- boot ordering relative to workflow and data bootstrap
- native side effects triggered by boot-time settings application

Out of scope:

- redesigning the Settings workspace UI
- adding new settings categories
- options-related settings surfaces

## Current User-Facing Behavior

On startup, storage migrations run first and the frontend then rehydrates persisted settings from browser storage. Theme, density, effects, help mode, workflow advanced state, and several other fields are applied before workflow bootstrap completes. Some settings values are normalized or ignored silently when they do not match the current catalog.

## Current Code Surface

- `web/src/main.ts`
- `web/src/preflight.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/services/settingsService.ts`
- `src-tauri/src/commands_system.rs`
- `web/src/components/SettingsWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/settingsBootstrap.test.ts`
- `web/src/services/settingsService.test.ts`
- `web/src/main.test.ts`
- settings panel tests

## Strengths Already Present

- migrations already run before settings bootstrap
- bootstrap logic is mostly centralized instead of scattered through many components
- settings store already has normalization helpers and explicit actions
- native system-effect calls are already wrapped behind service/store boundaries

## Weaknesses / Failure Modes

- boot-time settings ownership is split across preflight defaults, store initial state, bootstrap hydration, and native system calls
- value catalogs already drift for window effects, especially across `blur` and `vibrancy`
- hydration currently replays through actions that can persist or invoke side effects during boot
- invalid or stale storage values can be silently dropped without diagnostics
- there is no explicit startup-status channel for settings bootstrap warnings

## Regressions To Avoid

- changing user-visible settings defaults unexpectedly
- triggering native effects repeatedly during startup because hydration and persistence are not separated
- making cross-platform effect behavior less predictable
- breaking workflow bootstrap ordering by moving settings hydration too late
- removing current safe fallback behavior for invalid stored values
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html) reinforces keeping initial state predictable and explicit.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) supports clear ownership boundaries between persistence, normalization, and reactive state.
- [MDN: `localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) clarifies browser storage behavior and persistence constraints.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces treating native calls as explicit side effects instead of implicit state hydration.
- [Tauri Window Customization](https://v2.tauri.app/learn/window-customization/) is relevant for effect/value catalogs that must remain platform-aware.
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Inventory every boot-time settings key, its persisted storage key, its current owner, and whether it is pure state or has native side effects.
2. Split settings bootstrap into three explicit phases:
   - read raw storage
   - normalize to the current catalog
   - apply to reactive state and then trigger any native side effects intentionally
3. Unify the value catalog for appearance/effects across preflight, store defaults, bootstrap logic, UI selectors, and native command boundaries.
4. Add a bootstrap report object or logging path so invalid values and native-application failures are visible during boot diagnostics instead of being silent.
5. Tighten tests around ordering, cross-platform normalization, and native-side-effect boundaries.
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- `web/src/services/settingsBootstrap.ts` now reads through the shared storage helpers and routes boot-time hydration through `settingsStore.applyBootstrapState(...)` instead of the ordinary mutating setters
- that keeps startup hydration additive and deterministic: persisted settings still normalize and apply, native window-effect calls still occur intentionally, but unchanged persisted values are no longer rewritten during bootstrap
- `web/src/services/settingsBootstrap.test.ts` now asserts the bootstrap path does not re-persist unchanged values and treats storage-access failures as an empty, safe bootstrap state
- `web/src/main.test.ts` remains the ordering proof that settings bootstrap still happens before desktop initialization completes and before the runtime is marked ready

## Checklist

- [x] Inventory every boot-time settings field and its owner.
- [x] Separate pure hydration from side-effectful native application.
- [x] Normalize effect/value catalogs across preflight, bootstrap, store, UI, and backend.
- [x] Define how invalid storage values are logged, ignored, or migrated.
- [x] Document ordering relative to workflow bootstrap and dataset readiness.
- [x] Verify that hydration does not unnecessarily re-persist unchanged values.
- [x] Add tests for theme/effect/help/density/bootstrap ordering and invalid-value handling.
- [x] Add at least one integration-style test for boot-time settings application.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/services/settingsBootstrap.test.ts`
- run `npm test -- src/main.test.ts`
- run targeted settings panel/store tests
- run desktop smoke if window-effect application or native command wiring changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current storage keys unless a documented migration replaces them
- keep unsupported platform-specific values on a safe fallback path instead of hard failing startup
- prefer additive diagnostics over aggressive normalization that discards user intent silently
- preserve current boot ordering unless a clearer and safer ordering is explicitly documented
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
- `REPO_Improvement/01_shell/12_settings_runtime_diagnostics_2026-04-08.md`
- `REPO_Improvement/00_meta/09_regression_safety_rules_2026-04-08.md`
