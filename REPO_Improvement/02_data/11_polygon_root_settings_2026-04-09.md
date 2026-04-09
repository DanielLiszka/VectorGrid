# Polygon Root Settings Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to market-data root and Polygon-cache root handling so root changes, cache-path hints, and downstream dataset rewrites remain explicit and non-destructive.

## Scope Boundaries

In scope:

- configured market-data root directory settings
- effective root and Polygon cache directory derivation
- browse/set/reset/open folder behavior
- path rewriting when cached Polygon roots move

Out of scope:

- new storage backends
- remote object storage
- options-specific data roots

## Current User-Facing Behavior

The current app already supports configurable market-data roots. `src-tauri/src/settings.rs` stores an optional `market_data_root_dir`, resolves an effective root, and supports moving data when requested. `src-tauri/src/commands_data.rs` exposes `get_market_data_dir_info`, `set_market_data_dir`, and `clear_market_data_dir`, and `web/src/services/dataService.ts` plus `web/src/stores/dataStore.ts` normalize that into root hints, cache-dir text, and folder actions. The store also rewrites persisted Polygon startup-state paths when the cache root changes so previously selected datasets remain addressable.

Current shipped behavior includes:

- default root versus configured root distinction
- explicit `polygonCacheDir` in the response contract
- open/reveal folder affordances from the frontend
- optional move-existing behavior when switching market-data roots

## Current Code Surface

- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/stores/settingsStore.ts`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`

## Strengths Already Present

- root resolution is already centralized in backend settings code
- frontend code already receives normalized root info instead of reconstructing paths itself
- the app already exposes set/reset/browse/open root actions
- Polygon startup-state rewriting already acknowledges that cache roots can move

## Weaknesses / Failure Modes

- root semantics span backend settings, data service normalization, and store rewrite logic
- move-existing behavior and path-rewrite behavior are useful but not yet frozen in one doc
- settings-store and data-store surfaces both touch root-related behavior, which increases drift risk
- the current policy for when a root change should invalidate or rewrite selected datasets is implicit

## Regressions To Avoid

- losing access to cached Polygon datasets after a legitimate root move
- silently changing the effective root without updating UI hints
- breaking open/reveal folder behavior by changing which directory is treated as canonical
- moving data destructively without explicit user intent
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Dialog Plugin](https://v2.tauri.app/plugin/dialog/) is relevant because root configuration already depends on explicit user-picked folders.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because root-info responses and startup snapshots behave like persisted contracts.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant because moved-root handling intersects with persisted startup restore state on the frontend.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the difference between default root, configured root, effective root, and Polygon cache root.
2. Freeze move-existing semantics and current open/reveal behavior.
3. Document how root changes interact with selected Polygon dataset paths and startup restore snapshots.
4. Keep root-derived hints and settings-store synchronization explicit.
5. Add or tighten tests wherever root changes can rewrite stored paths or visible hints.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every root-related field returned to the frontend.
- [x] Document set/reset/browse/open behaviors and current fallback posture.
- [x] Document move-existing semantics and current safety expectations.
- [x] Document path rewriting for moved Polygon roots.
- [x] Add or tighten tests for root moves and settings-store synchronization if behavior changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- perform one manual root-change flow with and without move-existing after root-related edits
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current non-destructive defaults when changing roots
- prefer explicit rewrite behavior over silent invalidation where the app already supports safe rewrites
- avoid introducing a new canonical root meaning without updating both data and settings surfaces together
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added backend regression coverage in `src-tauri/tests/settings_tests.rs` so clearing a configured market-data root now explicitly proves the app falls back to the default app-local root instead of leaving stale configured state behind.
- Expanded frontend contract coverage in `web/src/services/dataService.test.ts` and `web/src/components/SettingsWorkspacePanel.test.ts` so the current root-field model stays explicit: `openMarketDataDir()` falls back to `effectiveRootDir` when no Polygon cache path is available, and the settings panel visibly preserves the market-data root hint plus the `Move existing Polygon cache` affordance.
- Kept the existing moved-root rewrite behavior intact in `web/src/stores/dataStore.actions.test.ts`; this slice tightened coverage around current semantics instead of moving rewrite logic or changing the canonical root/open-folder policy.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app --test settings_tests`, `npm.cmd --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/components/SettingsWorkspacePanel.test.ts src/stores/settingsStore.actions.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, `cargo check -p ta_desktop_demo_app`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/09_startup_dataset_restore_2026-04-09.md`
- `REPO_Improvement/02_data/12_polygon_api_key_2026-04-09.md`
- `REPO_Improvement/01_shell/11_settings_storage_shortcuts_2026-04-08.md`
