# Tauri Settings Persistence Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri calling Rust from the frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the current settings surface is command-driven from the frontend into Rust
- [Tauri state management](https://v2.tauri.app/develop/state-management/)
  - relevant because Rust settings should remain a clear backend-owned persistence boundary
- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because path validation and writable-root behavior depend on filesystem semantics
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because frontend-only settings persistence should keep its own verification boundary separate from Tauri settings
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

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
6. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current `app_settings.json` schema and supported key set.
- [x] Document the frontend-localStorage versus Tauri-settings split explicitly.
- [x] Document absolute-path validation and default resolution behavior.
- [x] Add or tighten command-level tests for get/set/clear flows.
- [x] Add malformed-JSON and write-failure coverage where still missing.
- [x] Preserve cache-move safety and failure visibility.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test settings_tests`
- run relevant `cargo test -p ta_desktop_demo_app commands_data`
- run `npm --prefix web run test -- src/services/settingsBootstrap.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- manually verify that frontend preference changes do not mutate Rust app settings and vice versa
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the narrow Tauri settings scope unless a deliberate expansion is approved
- prefer adding new Rust settings keys explicitly over overloading existing ones
- keep malformed settings recoverable where possible instead of turning startup into a brittle failure
- if more Tauri-side settings are added later, document them separately from browser UI preferences
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed on the existing narrow Tauri settings contract with targeted test hardening; no user-facing settings-scope expansion was required.
- The current Rust-side schema remains explicitly narrow:
  - `app_settings.json` currently persists only `market_data_root_dir`
  - unset state still resolves to the app-local data directory
  - configured paths must remain absolute
- The frontend/localStorage versus Tauri-settings split remains explicit and unchanged:
  - filesystem/data-root settings stay on the Rust/Tauri side
  - browser appearance/help/layout preferences remain in frontend local state and bootstrap through the existing web services/stores
- Rust-side verification was tightened in two places:
  - `src-tauri/tests/settings_tests.rs` now covers malformed JSON parse failure and deterministic write failure when the provided settings base path is a file
  - `src-tauri/src/commands_data.rs` now routes `get/set/clear_market_data_dir` behavior through base-path helpers, and the local unit tests cover default resolution, set/clear flows, invalid inputs, cache moves, and destination-conflict failure visibility
- Cache-move safety remains explicitly pinned:
  - move requests still refuse self-nesting cache moves
  - non-empty destination cache directories still fail loudly instead of silently overwriting data
- No `README.md` or `REPO_MAP.md` update was required because this work only hardened tests and command-local helper structure inside the existing settings boundary.
- Acceptance evidence for this closure:
  - `cargo test -p ta_desktop_demo_app --test settings_tests`
  - `cargo test -p ta_desktop_demo_app --lib commands_data::tests::`
  - `npm.cmd --prefix web run test -- src/services/settingsBootstrap.test.ts src/stores/settingsStore.actions.test.ts`
- Verification note:
  - the broader `cargo test -p ta_desktop_demo_app commands_data` filter was not a trustworthy verifier on this machine in the current repo state because it pulled in unrelated bin/test compilation and failed with paging-file / unrelated crate errors outside the touched settings surface
  - the narrower `--lib commands_data::tests::` target was used intentionally because it owns the command-layer tests added for this item

## Related Docs

- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
- `REPO_Improvement/02_data/17_polygon_download_load_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/12_program_governance_2026-04-08.md`
