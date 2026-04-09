# Settings Storage And Shortcuts Plan

Last updated: 2026-04-08

## Purpose

Define the current settings-workspace contract for storage-related settings, utility shortcuts, and navigation shortcuts so the Settings surface can be improved without breaking persisted preferences, market-data directory behavior, or existing quick-link actions.

## Scope Boundaries

In scope:

- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/stores/settingsStore.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/services/settingsService.ts`
- `web/src/utils/storage.ts`
- `web/src/preflight.ts`
- settings actions for data-folder, runs-folder, reports-folder, Polygon modal, library, and aggregate navigation

Out of scope:

- runtime diagnostics details
- desktop-notification delivery policy
- options-specific settings or options-library navigation

## Current User-Facing Behavior

The settings workspace already does more than cosmetic preferences. It persists theme, window effect, help mode, density mode, advanced-mode toggle, controls-collapsed toggle, desktop-notification preference, and custom workstation perspectives. It also acts as a practical shortcut hub for storage and workflow actions: the user can browse, set, reset, and open the market-data directory; open the workflow data step; open the runs directory; open the reports directory; open the Polygon modal; and jump to the run library or aggregate workspace. The Settings screen also shows a shortcut summary, even though that summary is still hard-coded rather than data-driven.

## Current Code Surface

- `web/src/components/SettingsWorkspacePanel.vue`
  - renders settings sections, shortcut summaries, perspective actions, and storage utility actions
- `web/src/stores/settingsStore.ts`
  - owns persisted shell prefs, perspective CRUD/apply flows, and storage/navigation action routing
- `web/src/stores/settingsStore.actions.test.ts`
  - verifies current store action routing, perspective behavior, and mutability gating
- `web/src/services/settingsBootstrap.ts`
  - hydrates persisted local settings back into the settings store during boot
- `web/src/services/settingsService.ts`
  - wraps Tauri-facing `set_window_effect`, `open_runs_dir`, and `open_reports_dir`
- `web/src/utils/storage.ts`
  - local storage helpers used by settings persistence
- `web/src/preflight.ts`
  - also participates in early theme/window-effect normalization
- `web/src/stores/dataStore.ts`
  - remains the source of truth for market-data directory state mirrored into Settings

## Current Verification Surface

- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/services/settingsBootstrap.test.ts`
- `web/src/services/settingsService.test.ts`
- `web/src/components/SettingsWorkspacePanel.test.ts`

## Strengths Already Present

- persisted settings are already normalized before storage and before bootstrap restore
- settings actions already route through stores/services instead of component-specific side effects
- perspective snapshots already capture workflow layout state instead of only visual preferences
- market-data shortcuts already respect runtime mutability and data-store truth
- built-in perspective protections already prevent destructive edits to shipped defaults

## Weaknesses / Failure Modes

- the settings workspace mixes durable preferences, shell navigation, and filesystem shortcuts in one large surface
- market-data directory state is mirrored from `dataStore`, which makes ownership easy to misread
- local-storage keys are stable in practice but not yet treated as a documented schema
- current shortcut summary is presentational and not derived from one source of truth
- window-effect normalization currently spans `preflight`, bootstrap, store, and backend-facing service code

## Regressions To Avoid

- breaking persisted theme/effect/help/density/notification restore on boot
- letting settings mutate market-data state without keeping `dataStore` as the real owner
- removing or silently changing current runs/reports/data/Polygon shortcut actions
- changing perspective apply/duplicate/rename/delete behavior without test updates
- breaking queued market-data input flush or the `data-running` mutability lock for storage utilities

## External Research Summary

- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because settings are shared shell state, not component-local values
- [Pinia: Core Concepts](https://pinia.vuejs.org/core-concepts/)
  - relevant because the settings store already acts as a central action/state boundary
- [Pinia: Testing](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because most current behavior is already protected through store-level action tests
- [MDN: `Window.localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because settings bootstrap and persistence depend on stable local-storage behavior
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because several settings shortcuts are command-mediated desktop actions

## Proposed Improvement Approach

1. Separate the settings contract into three documented families:
   - persisted appearance/help/layout settings
   - perspective save/apply management
   - storage and navigation shortcuts
2. Document which values are:
   - persisted locally
   - mirrored from `dataStore`
   - forwarded through Tauri command wrappers
3. Make the shortcut summary data-driven so the panel, store, and any future command-palette/help surface do not drift.
4. Align window-effect normalization rules across `preflight`, bootstrap, store, and backend bridge behavior before expanding effects further.

## Checklist

- [ ] Document the current persisted settings keys and their normalization rules.
- [ ] Document market-data directory ownership between `dataStore` and `settingsStore`.
- [ ] Document current shortcut actions available from Settings.
- [ ] Document perspective CRUD/apply behavior and persistence.
- [ ] Document runtime mutability rules that disable storage actions during active runs.
- [ ] Add or tighten tests if settings persistence or shortcut routing changes.

## Verification Plan

- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`
- run `npm --prefix web test -- src/services/settingsBootstrap.test.ts`
- run `npm --prefix web test -- src/services/settingsService.test.ts`
- run `npm --prefix web test -- src/components/SettingsWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current storage keys unless a deliberate migration path is added
- preserve current utility shortcuts in Settings even if the layout is reorganized
- if market-data actions move, keep `dataStore` as the authoritative state owner

## Related Docs

- `REPO_Improvement/01_shell/10_settings_perspectives_2026-04-08.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/02_data/31_data_service_store_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/22_tauri_settings_persistence_2026-04-08.md`
