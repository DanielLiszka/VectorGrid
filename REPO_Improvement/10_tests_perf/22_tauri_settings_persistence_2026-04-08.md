# Tauri Settings Persistence Plan

Last updated: 2026-04-08

## Purpose

Define the current Tauri-side settings persistence contract so Rust settings commands, filesystem validation, and frontend bootstrap boundaries can be improved without mixing browser preferences with app-settings persistence.

## Scope Boundaries

In scope:

- Rust-side `app_settings.json` behavior
- `market_data_root_dir` get/set/clear and resolution rules
- validation of absolute writable directories and cache-move safety
- test coverage for Tauri-side settings persistence

Out of scope:

- browser `localStorage` settings such as theme/help/layout
- options-specific settings
- new cross-device settings sync

## Current User-Facing Behavior

The current Tauri settings surface is intentionally narrow. Rust-side persisted settings currently cover:

- `market_data_root_dir` in `app_settings.json`

Current behavior already includes important safety rules:

- unset settings resolve to the app-local data directory
- configured roots must be absolute paths
- root-directory changes may move Polygon cache data
- frontend appearance/help/layout preferences stay in browser storage and bootstrap through a separate path

So the current product already has a split-brain settings model by design:

- filesystem/data-root settings live in Tauri app settings
- shell/UI preferences live in frontend local state

## Current Code Surface

- `src-tauri/src/settings.rs`
  - loads, saves, resolves, and updates `app_settings.json`
- `src-tauri/src/commands_data.rs`
  - exposes market-data-root command behavior and related validation/move flows
- `src-tauri/tests/settings_tests.rs`
  - verifies default resolution, persistence, absolute-path rejection, and directory moves
- `web/src/services/settingsBootstrap.ts`
  - frontend-only bootstrap for local preference state
- `web/src/stores/settingsStore.ts`
  - frontend-only preference persistence
- `web/src/services/settingsBootstrap.test.ts`
  - verifies frontend bootstrap behavior
- `web/src/stores/settingsStore.actions.test.ts`
  - verifies frontend settings persistence behavior

## Current Verification Surface

- `src-tauri/tests/settings_tests.rs`
- `web/src/services/settingsBootstrap.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- command-level tests in `src-tauri/src/commands_data.rs`

## Strengths Already Present

- Rust-side settings scope is narrow and understandable
- app-settings persistence already validates absolute paths
- default-versus-configured resolution is already explicit
- directory-move support already exists
- frontend and Tauri settings persistence are already separate code paths

## Weaknesses / Failure Modes

- the separation between browser preference storage and Tauri app settings is not documented in one place
- malformed JSON/write-failure coverage is still thinner than the happy-path tests
- command-level behavior around get/set/clear is more important than the current doc coverage suggests
- future contributors could accidentally route a frontend preference into Tauri settings or vice versa
- cache-move safety and failure semantics are easy to weaken if refactored casually

## Regressions To Avoid

- mixing browser-pref persistence with Tauri `app_settings.json` persistence
- weakening absolute-path or writability checks
- breaking default-vs-configured root resolution
- making cache moves unsafe or partially silent on failure
- expanding Rust settings scope casually without documenting the boundary change

## External Research Summary

- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the current settings surface is command-driven from the frontend into Rust
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
  - relevant because Rust settings should remain a clear backend-owned persistence boundary
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because path validation and writable-root behavior depend on filesystem semantics
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because frontend-only settings persistence should keep its own verification boundary separate from Tauri settings

## Proposed Improvement Approach

1. Freeze the split between:
   - Tauri app settings
   - frontend local preferences
2. Document the Rust settings contract explicitly:
   - file location
   - supported keys
   - validation rules
   - default resolution behavior
3. Add command-level tests for:
   - get market data dir
   - set market data dir
   - clear market data dir
   - malformed JSON and write failures
4. Preserve path-validation and cache-move safety as first-class contract items.
5. Keep future settings additions explicit and bounded rather than silently expanding `app_settings.json`.

## Checklist

- [ ] Document the current `app_settings.json` schema and supported key set.
- [ ] Document the frontend-localStorage versus Tauri-settings split explicitly.
- [ ] Document absolute-path validation and default resolution behavior.
- [ ] Add or tighten command-level tests for get/set/clear flows.
- [ ] Add malformed-JSON and write-failure coverage where still missing.
- [ ] Preserve cache-move safety and failure visibility.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test settings_tests`
- run relevant `cargo test -p ta_desktop_demo_app commands_data`
- run `npm --prefix web test -- src/services/settingsBootstrap.test.ts`
- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`
- manually verify that frontend preference changes do not mutate Rust app settings and vice versa

## Rollout / Fallback / Compatibility Notes

- preserve the narrow Tauri settings scope unless a deliberate expansion is approved
- prefer adding new Rust settings keys explicitly over overloading existing ones
- keep malformed settings recoverable where possible instead of turning startup into a brittle failure
- if more Tauri-side settings are added later, document them separately from browser UI preferences

## Related Docs

- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
- `REPO_Improvement/02_data/17_polygon_download_load_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
