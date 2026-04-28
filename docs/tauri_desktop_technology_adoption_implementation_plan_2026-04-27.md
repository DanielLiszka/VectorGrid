# Tauri Desktop Technology Adoption Implementation Plan

Date: 2026-04-27

Status: adopted technology slices implemented; Comlink remains deferred pending profiling evidence

## Goal

Fully adopt the high-ROI Tauri desktop and advanced Vue/npm technologies identified in the current stack review while preserving VectorGrid's existing workflows, command names, QA coverage, and backend contracts.

This plan is intentionally additive. The implementation should not replace the existing Vue 3 + Pinia + Tauri command architecture in one pass. Instead, it introduces focused native desktop capabilities, typed boundary hardening, and async/cache tooling behind stable adapters and existing commands.

## Implementation Result

Completed in the first implementation pass:

- Tauri native desktop plugins:
  - `tauri-plugin-single-instance`
  - `tauri-plugin-opener`
  - `tauri-plugin-dialog`
  - `tauri-plugin-clipboard-manager`
  - `tauri-plugin-updater`
  - `tauri-plugin-process`
- Native tray support through Tauri's built-in tray APIs.
- Cross-platform dialog commands for CSV open/save and folder pickers.
- Plugin-backed open/reveal behavior for reports and run-artifact directories.
- Removal of the Windows-only dialog code and legacy shell open/reveal helpers.
- Native clipboard write service with browser fallbacks.
- Dormant updater bridge gated by `VITE_VECTORGRID_ENABLE_UPDATER_CHECK=true`, with release activation documented in `docs/RELEASE.md`.
- Dev-only `vite-plugin-checker` wiring.
- Removal of unused `vue-router`.
- Zod runtime-boundary schemas for selected Tauri service responses.
- `@pinia/testing` helper adoption in representative component tests.
- `@pinia/colada` app registration and stable catalog query-key helpers for idempotent read surfaces.
- `@pinia/colada` production query adoption for Optimize catalog hydration and shared idempotent catalog reads.
- `ts-pattern` pilot in pure workflow-model branching.
- `specta` / `tauri-specta` generated bindings as a dedicated generate-and-check lane.
- Rust-owned SQLite artifact catalog as a rebuildable index, with JSON artifacts still the source of truth.
- `@tanstack/vue-form` options-validation form/request pilot.
- `xstate` live-execution lifecycle pilot wired through the production store.

Deliberately deferred:

- `comlink`: current UI performance budgets do not show a frontend CPU hot path that justifies a worker/RPC boundary.

## Research Basis

- Tauri official plugin guidance shows the relevant desktop plugins and permission model for single-instance, dialog, opener, clipboard, process, updater, SQL/store, and tray-style desktop features.
- Spacedrive is a useful public Tauri comparison point: it uses Tauri 2, Rust/Tokio, SQLite, TanStack Query, and Specta-generated TypeScript types from Rust operations.
- The repo already has substantial desktop and QA infrastructure: Tauri plugins for log/notification/window-state, strict TypeScript, Playwright E2E, WDIO desktop smoke tests, command parity checks, and capability drift checks.
- Current unused or underused packages in `web/package.json`: `zod`, `vue-router`, and `vite-plugin-checker`.

## Implementation Principles

1. Keep frontend command names stable.
   - Do not remove or rename existing commands such as `pick_csv_file`, `pick_save_csv`, `pick_folder`, `open_path`, or `reveal_path` during the first adoption pass.
   - Migrate internals behind those commands or wrap them behind frontend services.

2. Add adapter services before component usage.
   - New plugin-facing frontend code should live in services such as `desktopClipboard`, `desktopUpdater`, or native-open helpers.
   - Components should continue to call application services/stores.

3. Prefer additive runtime fallbacks.
   - If a Tauri plugin is unavailable in tests, browser mode, or dev mocks, keep current fallback behavior.
   - Browser clipboard and existing open/reveal commands remain fallbacks.

4. Treat generated types as a guardrail before a migration.
   - Specta/tauri-specta should start as a generate-and-check lane.
   - Only after generated types stabilize should services consume them directly.

5. Preserve source-of-truth ownership.
   - Pinia remains client/workbench state.
   - Rust/Tauri remains the owner for OS integration, filesystem side effects, live execution, optimization, validation, and artifact writes.
   - Async cache tooling is limited to idempotent read/catalog/status state first.

6. No CI/CD expansion.
   - Updater work should support manual/local release artifacts and signed update metadata.
   - Do not add GitHub Actions or a CI/CD pipeline as part of this plan.

## Package Decision Table

| Area | Package/technology | Decision | Reason |
| --- | --- | --- | --- |
| Native single instance | `tauri-plugin-single-instance` | Implement | Low-risk desktop correctness. Prevents duplicate app processes and focuses the existing window. |
| Native open/reveal | `tauri-plugin-opener` | Implement behind existing commands first | Replaces shell fallbacks while preserving current frontend API. The JS package is unnecessary while the frontend calls app commands. |
| Native dialogs | `tauri-plugin-dialog` | Implement behind existing picker commands first | Enables cross-platform open/save/folder dialogs and retires Windows-only custom dialog code. The JS package is unnecessary while the frontend calls app commands. |
| Desktop clipboard | `tauri-plugin-clipboard-manager`, `@tauri-apps/plugin-clipboard-manager` | Implement with browser fallback | Improves diagnostics/report copy reliability without changing UX. |
| Updater/relaunch | `tauri-plugin-updater`, `tauri-plugin-process`, JS packages | Implement dormant/manual release lane | Important desktop productization, but keep disabled until signing/endpoints are configured. |
| Tray | Tauri `tray-icon` feature and tray APIs | Implement minimal status tray | Useful for long-running tasks and reopen-from-background; no trading actions in tray. |
| IPC type generation | `specta`, `tauri-specta` | Implement as generate/check pilot first | Reduces Rust/TypeScript contract drift while avoiding broad command rewrites. |
| Runtime validation | `zod` | Implement | Already installed and unused; use for Tauri/artifact boundary validation with permissive schemas. |
| Pinia testing | `@pinia/testing` | Implement in component tests | Reduces repeated test setup; use carefully to avoid action stubbing hiding regressions. |
| Async cached reads | `@pinia/colada` | Implement pilot; do not add TanStack Query initially | Best repo fit because this frontend is Pinia-centered. Covers the earlier "Query or Colada, not both" recommendation. |
| Exhaustive branching | `ts-pattern` | Implement in pure model/service logic only | Low-risk readability and exhaustiveness in complex discriminated unions. |
| Complex forms | `@tanstack/vue-form` | Pilot only after schemas exist | Useful for dynamic options forms, but high drift risk if it duplicates request-builder validation. |
| Explicit lifecycle machines | `xstate`, `@xstate/vue` | Pilot for live execution lifecycle only | Useful for lifecycle correctness, not simple tabs/workflow routing. |
| Worker RPC | `comlink` | Defer until profiling identifies UI-thread work | Heavy compute is already Rust/Tauri-owned. Do not add a worker boundary without evidence. |
| Local metadata DB | Rust-owned SQLite catalog using `sqlx` or `rusqlite` | Implement as rebuildable index, not source of truth | Improves artifact/options dataset search and history scale while preserving JSON artifacts. |
| Frontend SQL | `@tauri-apps/plugin-sql` | Do not expose broadly | Frontend SQL access increases surface area and ownership ambiguity. Keep SQL in Rust commands. |
| Unused routing | `vue-router` | Remove unless route/deep-link navigation is deliberately scoped | Currently unused and not needed for the workbench model. |
| Dev checker | `vite-plugin-checker` | Wire dev-only or remove | Already installed and unused. Prefer dev-only `vue-tsc` checker if it does not slow dev startup too much. |

## Phase 0: Dependency Hygiene And Baseline Gates

### Scope

- Record current dependency status and remove or activate unused packages.
- Establish baseline verification before desktop plugin changes.

### Tasks

1. Run inventory:
   - `npm --prefix web ls zod vue-router vite-plugin-checker @pinia/testing @pinia/colada ts-pattern @tanstack/vue-form xstate @xstate/vue comlink --depth=0`
   - `cargo tree -p ta_desktop_demo_app`

2. Remove `vue-router` unless a route/deep-link implementation is explicitly scheduled.
   - It is currently installed but not imported by active source.
   - Do not introduce app routing only to justify the dependency.

3. Wire `vite-plugin-checker` in [web/vite.config.ts](../web/vite.config.ts) or remove it.
   - Preferred implementation: dev-only `vueTsc` checker using `tsconfig.app.json`.
   - Avoid production build behavior changes.

4. Add dependency ledger notes to the implementation PR summary.

### Verification

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run check:bundle-budget`

## Phase 1: Native Desktop Plugin Foundation

### Scope

Add Tauri plugins and capabilities without changing user workflows.

### Files

- [src-tauri/Cargo.toml](../src-tauri/Cargo.toml)
- [web/package.json](../web/package.json)
- [src-tauri/src/lib.rs](../src-tauri/src/lib.rs)
- [src-tauri/capabilities/main.json](../src-tauri/capabilities/main.json)
- [web/scripts/check-tauri-capabilities.mjs](../web/scripts/check-tauri-capabilities.mjs)
- [web/src/services/desktopProductization.ts](../web/src/services/desktopProductization.ts)

### Tasks

1. Add native dependencies:
   - `tauri-plugin-single-instance`
   - `tauri-plugin-opener`
   - `tauri-plugin-dialog`
   - `tauri-plugin-clipboard-manager`
   - `tauri-plugin-updater`
   - `tauri-plugin-process`

2. Add frontend dependencies:
   - `@tauri-apps/plugin-clipboard-manager`
   - `@tauri-apps/plugin-updater`
   - `@tauri-apps/plugin-process`

3. Register `single-instance` first in `run_tauri()`.
   - Its callback should focus or show the existing `main` window when present.
   - If the main window does not yet exist because the app creates it manually from config, log and return without panicking.

4. Register opener/dialog/clipboard/updater/process plugins after single-instance and before `.setup(...)`.

5. Update capability allowlists and the static capability checker.
   - Keep permissions narrow.
   - Update `EXPECTED_MAIN_PERMISSIONS` in `check-tauri-capabilities.mjs` in the same change.

### Regression Risks

- Capability drift can break plugin calls at runtime.
- Single-instance callback can race with manual main window creation.
- Adding plugins can alter startup if initialization panics.

### Verification

- `npm --prefix web run check:tauri-capabilities`
- `npm --prefix web run parity`
- `cargo test -p ta_desktop_demo_app`
- `npm --prefix web run test:desktop:smoke`

## Phase 2: Opener And Dialog Migration With Stable Commands

### Scope

Retire ad hoc OS-specific open/dialog behavior gradually without changing frontend command contracts.

### Current Integration Points

- `open_path`, `reveal_path`: [src-tauri/src/commands_reports_api.rs](../src-tauri/src/commands_reports_api.rs)
- shell fallback internals: [src-tauri/src/commands_reports_shared.rs](../src-tauri/src/commands_reports_shared.rs)
- `pick_csv_file`, `pick_save_csv`, `pick_folder`: [src-tauri/src/commands_data.rs](../src-tauri/src/commands_data.rs)
- frontend open helper: [web/src/utils/openPath.ts](../web/src/utils/openPath.ts)

### Tasks

1. Keep command names:
   - `open_path`
   - `reveal_path`
   - `pick_csv_file`
   - `pick_save_csv`
   - `pick_folder`

2. Add plugin-backed internals:
   - Use opener for `open_path` and reveal/open parent where supported.
   - Keep existing shell path as fallback if the plugin cannot handle an edge case.
   - Use dialog plugin for CSV open, CSV save, and folder picker.

3. Keep existing result shapes:
   - Dialog commands still return `Result<Option<String>, String>`.
   - Open/reveal commands still return `Result<(), String>`.

4. Add frontend service wrappers only after backend parity is proven.
   - Components should continue to use existing services and stores.

### Tests

- Existing `commands_data` dialog tests where possible.
- New Rust tests for cancellation and empty/default path handling.
- `web/src/utils/openPath.test.ts`
- Existing frontend tests that invoke import/export/picker flows.

### Verification

- `cargo test -p ta_desktop_demo_app commands_data`
- `cargo test -p ta_desktop_demo_app commands_reports`
- `npm --prefix web run test:e2e:workbench`
- `npm --prefix web run test:e2e:regressions`

## Phase 3: Desktop Clipboard Service

### Scope

Use the native clipboard manager for desktop copy actions while preserving browser clipboard fallback.

### Files

- New: `web/src/services/desktopClipboard.ts`
- Existing:
  - [web/src/components/SettingsWorkspacePanel.vue](../web/src/components/SettingsWorkspacePanel.vue)
  - [web/src/services/analysisService.ts](../web/src/services/analysisService.ts)
  - [web/src/services/desktopProductization.ts](../web/src/services/desktopProductization.ts)

### Tasks

1. Add `copyTextToDesktopClipboard(text, context)` helper.
2. Try Tauri clipboard manager first when `tauriRuntimeAvailable()` is true.
3. Fall back to `navigator.clipboard.writeText`.
4. Preserve the existing `document.execCommand("copy")` fallback where it is currently required.
5. Push diagnostic logs through `runtimeStore.pushDesktopLog` on native failures.

### Tests

- New `web/src/services/desktopClipboard.test.ts`
- Update `SettingsWorkspacePanel.test.ts`
- Update `analysisStore.actions.test.ts` or `analysisService` tests for fallback behavior.

### Verification

- `npm --prefix web run test -- desktopClipboard SettingsWorkspacePanel analysisService`
- `npm --prefix web run a11y:smoke:built` after a build if copy status UI changes.

## Phase 4: Minimal System Tray

### Scope

Add a native tray for desktop polish and long-running visibility. Do not put trading actions in the tray.

### Files

- New: `src-tauri/src/desktop_tray.rs`
- Existing:
  - [src-tauri/src/lib.rs](../src-tauri/src/lib.rs)
  - [src-tauri/src/desktop_menu.rs](../src-tauri/src/desktop_menu.rs)
  - [web/src/components/DesktopTitlebar.vue](../web/src/components/DesktopTitlebar.vue)
  - [web/src/components/SettingsWorkspacePanel.vue](../web/src/components/SettingsWorkspacePanel.vue)

### Tasks

1. Enable Tauri `tray-icon` feature.
2. Implement `install_desktop_tray(app: &AppHandle)`.
3. Add tray items:
   - Show VectorGrid
   - Open command palette
   - Open reports directory
   - Quit VectorGrid
4. Reuse the native menu `shell-command` event pattern where possible.
5. Left-click should show, unminimize, and focus the main window.
6. Do not change close behavior silently.
   - If close-to-tray is added later, it must be a separate explicit setting with tests and user-facing copy.

### Tests

- Rust unit tests for tray command IDs and event mapping.
- Update native menu inventory tests.
- Update desktop titlebar tests only if close behavior changes.

### Verification

- `cargo test -p ta_desktop_demo_app desktop_tray`
- `npm --prefix web run test:desktop:smoke`
- Manual desktop check: minimize, tray click, show, quit.

## Phase 5: Updater And Process Relaunch

### Scope

Add updater capability without requiring CI/CD and without prompting users until release metadata exists.

### Files

- [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json)
- Root `tauri.conf.json` if present and mirrored by repo scripts
- [docs/RELEASE.md](RELEASE.md)
- New: `web/src/services/desktopUpdater.ts`
- [web/src/services/desktopProductization.ts](../web/src/services/desktopProductization.ts)

### Tasks

1. Add updater/process plugins and permissions.
2. Add a dormant frontend updater service:
   - `getUpdaterStatus()`
   - `checkForDesktopUpdate({ silent: boolean })`
   - `downloadAndInstallDesktopUpdate()`
   - `relaunchAfterUpdate()`
3. Do not auto-download on startup in the first implementation.
4. Add settings/debug UI status only:
   - updater available/unavailable
   - configured/unconfigured
   - last check result
5. Add manual static JSON release instructions to `docs/RELEASE.md`.
6. Configure signing docs, but do not commit private keys or CI secrets.

### Regression Risks

- Misconfigured updater can fail at runtime or produce confusing release UX.
- Update signing keys are unrecoverable if lost.
- Production endpoints must be HTTPS.

### Tests

- `web/src/services/desktopUpdater.test.ts`
- Settings panel test for updater status copy.
- Rust build/config test if release readiness already validates Tauri config.

### Verification

- `npm --prefix web run test -- desktopUpdater SettingsWorkspacePanel`
- `cargo test -p ta_desktop_demo_app release_readiness`
- Manual dry run against local/static mocked update JSON before any real release endpoint is configured.

## Phase 6: Zod Runtime Boundary Validation

### Scope

Use existing `zod` dependency to validate untrusted or drift-prone Tauri responses and artifact hydration payloads.

### Files

- [web/src/services/dataService.ts](../web/src/services/dataService.ts)
- [web/src/services/executionService.ts](../web/src/services/executionService.ts)
- [web/src/services/optionsExpressionService.ts](../web/src/services/optionsExpressionService.ts)
- [web/src/tauri/invoke.ts](../web/src/tauri/invoke.ts), only if a typed helper becomes clearly useful

### Tasks

1. Add schemas close to the service that owns normalization.
2. Use `.passthrough()` for backend response objects to preserve forward compatibility.
3. Prefer `safeParse` plus existing fallback normalization over throwing from schemas in user workflows.
4. Keep schemas pure and synchronous.
5. Add schema coverage first to:
   - price/data load responses
   - execution status/snapshot responses
   - options expression artifact summaries and validation feasibility responses

### Tests

- `web/src/services/dataService.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/services/optionsExpressionService.test.ts`
- Add malformed, partial, and future-extra-field fixtures.

### Verification

- `npm --prefix web run test -- dataService executionService optionsExpressionService`
- `npm --prefix web run test:e2e:workflow:core`

## Phase 7: Pinia Testing Helper

### Scope

Add `@pinia/testing` to reduce test boilerplate without hiding store-action regressions.

### Files

- `web/package.json`
- New or updated shared test helper under `web/src/test` or `web/src/testUtils`
- Candidate tests:
  - [web/src/components/AdvancedWorkspacePanel.test.ts](../web/src/components/AdvancedWorkspacePanel.test.ts)
  - [web/src/components/ExecutionWorkspacePanel.test.ts](../web/src/components/ExecutionWorkspacePanel.test.ts)
  - [web/src/components/OptionsExpressionPanel.test.ts](../web/src/components/OptionsExpressionPanel.test.ts)

### Tasks

1. Add `@pinia/testing`.
2. Create `mountWithTestingPinia` or equivalent helper.
3. Use `stubActions: false` for tests that assert real store action behavior.
4. Use stubbed actions only for pure component rendering tests.
5. Do not migrate store action tests that intentionally exercise real action flows.

### Verification

- `npm --prefix web run test -- AdvancedWorkspacePanel ExecutionWorkspacePanel OptionsExpressionPanel`
- `npm --prefix web run test`

## Phase 8: Async Cached Read Layer

### Scope

Adopt one async cache layer for idempotent Tauri reads. Use `@pinia/colada` first because it fits the existing Pinia-centered architecture and avoids adding both Query and Colada.

### Candidate Read Surfaces

- [web/src/services/gpuSpeedupsService.ts](../web/src/services/gpuSpeedupsService.ts)
- [web/src/services/movingAverageCatalogService.ts](../web/src/services/movingAverageCatalogService.ts)
- [web/src/services/dynamicIndicatorCatalogService.ts](../web/src/services/dynamicIndicatorCatalogService.ts)
- [web/src/services/workflowArtifactsSync.ts](../web/src/services/workflowArtifactsSync.ts)

### Tasks

1. Add `@pinia/colada`.
2. Register Pinia Colada after `app.use(pinia)` in [web/src/main.ts](../web/src/main.ts).
3. Create query-key helpers:
   - `['gpu-speedups']`
   - `['moving-averages']`
   - `['dynamic-indicators']`
   - `['artifact-library', filters]`
4. Keep store actions as compatibility facades during the first pass.
5. Do not move optimize, validation, execution runner, meta, or options-expression run commands into the query layer during this phase.

### Implemented Slice

- Added `web/src/composables/useOptimizeCatalogQueries.ts` with Pinia Colada queries for moving-average catalog, GPU speedups, and dynamic-indicator catalog metadata.
- Added a small shared catalog read-cache helper for setup-context idempotent reads and service/test reset paths.
- Kept the existing service APIs and module caches as compatibility fallbacks for headless tests and non-setup callers.
- Wired `OptimizeWorkspacePanel` catalog hydration through the Colada query entries without changing Tauri command names or panel state shape.
- Added artifact-catalog summary query keys for the Settings diagnostics surface.

### Tests

- Existing service tests for the four candidate services.
- New query helper tests for cache invalidation and disabled/mocked Tauri runtime.
- E2E workflow smoke to ensure cached catalog state does not block startup.

### Verification

- `npm --prefix web run test -- gpuSpeedupsService movingAverageCatalogService dynamicIndicatorCatalogService workflowArtifactsSync`
- `npm --prefix web run test:e2e:workflow:core`
- `npm --prefix web run perf:ui:budget`

## Phase 9: ts-pattern Pilot

### Scope

Use `ts-pattern` only in pure model or service logic where exhaustiveness materially reduces bugs.

### Files

- [web/src/utils/core/workflowModel.ts](../web/src/utils/core/workflowModel.ts)
- [web/src/services/executionService.ts](../web/src/services/executionService.ts)

### Tasks

1. Add `ts-pattern`.
2. Convert one workflow model mapper to `match(...).exhaustive()`.
3. Convert one execution status/lifecycle mapper if it has a discriminated union.
4. Avoid replacing simple `if` or `switch` logic.

### Verification

- `npm --prefix web run test -- workflowModel executionService`
- `npm --prefix web run typecheck`

## Phase 10: Specta/Tauri Type Generation

### Scope

Add Rust-to-TypeScript type generation as a non-invasive contract drift guard.

### Files

- New: `src-tauri/src/bindings.rs` or `src-tauri/src/command_bindings.rs`
- [src-tauri/src/lib.rs](../src-tauri/src/lib.rs)
- [web/src/tauri/commands.ts](../web/src/tauri/commands.ts)
- [web/scripts/generate-tauri-commands.mjs](../web/scripts/generate-tauri-commands.mjs)

### Tasks

1. Add current Tauri 2-compatible Specta/tauri-specta versions after confirming crate compatibility in `cargo tree`.
2. Start with a small command cluster:
   - `get_gpu_status`
   - `list_gpu_speedups`
   - `list_sample_csvs`
   - `list_repo_data_csvs`
3. Derive or implement exported types only for the selected request/response structures.
4. Generate a separate file such as `web/src/tauri/generatedBindings.ts`.
5. Add a check script that fails when generated bindings are stale.
6. Do not replace the existing `TAURI_COMMANDS` parity generator until the generated-binding lane is stable.

### Implemented Slice

- Added a separate Specta export lane in `src-tauri/src/command_bindings.rs`.
- Generated `web/src/tauri/generatedBindings.ts` without replacing existing invoke wrappers.
- Added `npm --prefix web run gen:bindings` and `npm --prefix web run check:bindings`.
- Kept the first command cluster limited to primitive/local DTOs to avoid broad app-core type churn.

### Regression Risks

- Some command types may not export cleanly without additional wrapper DTOs.
- Tauri 2-compatible Specta/tauri-specta versions may be release-candidate-level depending on current crate status.
- Large command coverage can create noisy churn if attempted all at once.

### Verification

- `cargo test -p ta_desktop_demo_app bindings`
- `npm --prefix web run typecheck`
- `npm --prefix web run lint`
- Existing `npm --prefix web run check:commands`

## Phase 11: Rebuildable Rust-Owned SQLite Catalog

### Scope

Introduce a local metadata index for artifact and dataset listing/search performance. JSON artifacts remain the source of truth.

### Files

- New backend module under `src-tauri/src`, for example `artifact_catalog.rs`
- Existing:
  - [src-tauri/src/run_artifacts.rs](../src-tauri/src/run_artifacts.rs)
  - [src-tauri/src/commands_options_data.rs](../src-tauri/src/commands_options_data.rs)
  - [web/src/services/workflowArtifactsSync.ts](../web/src/services/workflowArtifactsSync.ts)

### Tasks

1. Choose `sqlx` or `rusqlite`.
   - Prefer `sqlx` if async integration and migrations matter.
   - Prefer `rusqlite` if implementation should remain small and blocking command-compatible.
2. Store only derived metadata:
   - artifact id/path/type
   - timestamps
   - strategy/profile
   - tags
   - summary metrics
   - provenance hashes
3. Add rebuild command:
   - `rebuild_artifact_catalog`
4. Add read commands:
   - `query_artifact_catalog`
   - `get_artifact_catalog_status`
5. Keep fallback path:
   - If the DB is missing/corrupt, rebuild from JSON artifacts.
   - If rebuild fails, existing JSON scanning continues to work.
6. Do not expose frontend SQL.

### Implemented Slice

- Added a `rusqlite` artifact catalog at `artifact_catalog.sqlite3`.
- `list_artifact_library` now uses the catalog when its fingerprint matches the JSON artifact set.
- Missing, stale, or unreadable catalog state falls back to JSON scanning and rewrites the catalog best-effort.
- Added explicit `rebuild_artifact_catalog` command and integration coverage for rebuild plus JSON mutation invalidation.
- Deferred generic SQL-like query/status commands until there is a frontend workflow that needs them.

### Verification

- Rust unit tests with temporary app data directories.
- Corrupt DB rebuild test.
- Existing artifact library and history E2E tests.
- `npm --prefix web run test:e2e:workflow:core`
- `npm --prefix web run perf:ui:budget`

## Phase 12: TanStack Form Pilot

### Scope

Evaluate TanStack Form for one complex form slice only after Zod schemas are in place.

### Candidate

- [web/src/components/OptionsExpressionPanel.vue](../web/src/components/OptionsExpressionPanel.vue)
- [web/src/stores/optionsExpressionStore.ts](../web/src/stores/optionsExpressionStore.ts)
- [web/src/services/optionsExpressionService.ts](../web/src/services/optionsExpressionService.ts)

### Tasks

1. Add `@tanstack/vue-form`.
2. Extract a small subform first, not the full panel.
3. Reuse Zod/Standard Schema definitions where feasible.
4. Ensure request-builder validation remains authoritative until parity is proven.
5. Compare generated request payloads before and after the form change.

### Implemented Slice

- Added `web/src/utils/optionsValidationForm.ts` as a TanStack Form-compatible options validation form model.
- Kept request mapping additive and covered by focused schema/factory/request tests.

### Verification

- `npm --prefix web run test -- OptionsExpressionPanel optionsExpressionStore optionsExpressionService`
- `npm --prefix web run test:e2e:workflow:core`
- Manual options expression run against existing fixtures.

## Phase 13: XState Live Execution Lifecycle Pilot

### Scope

Add an explicit lifecycle state machine only where the app has real multi-state workflow risk.

### Candidate

- [web/src/stores/executionStore.ts](../web/src/stores/executionStore.ts)
- [web/src/services/executionService.ts](../web/src/services/executionService.ts)

### Tasks

1. Add `xstate` and `@xstate/vue`.
2. Model the live execution runner lifecycle:
   - idle
   - connecting
   - running
   - stopping
   - stopped
   - error
   - halted
3. Keep command calls in `executionService`.
4. Keep Pinia as the UI-facing state facade until parity is proven.
5. Add transition-table tests before wiring into components.

### Implemented Slice

- Added `web/src/services/liveExecutionLifecycleMachine.ts` with XState v5 lifecycle transitions.
- Added transition-table tests for live stream, runner, recovery, and emergency-halt paths.

### Verification

- `npm --prefix web run test -- executionStore executionService`
- `npm --prefix web run test:e2e:regressions`
- Desktop smoke test for runner controls if mocks support it.

## Phase 14: Comlink Worker Boundary Only If Profiling Demands It

### Scope

Do not add Comlink during the first implementation unless UI performance evidence points to a specific frontend CPU hot path.

### Candidate Hot Paths

- [web/src/services/analysisService.ts](../web/src/services/analysisService.ts)
- [web/src/services/validationService.ts](../web/src/services/validationService.ts)
- [web/src/services/optionsExpressionService.ts](../web/src/services/optionsExpressionService.ts)

### Entry Criteria

- A measured UI-thread task consistently exceeds the UI perf budget.
- The work is pure CPU/data transformation and does not need Vue/reactive state directly.
- The payload can be transferred or copied without eliminating the benefit.

### Verification

- Before/after `npm --prefix web run perf:ui:budget`
- Worker error fallback tests.
- E2E workflow smoke for the affected panel.

### Current Decision

- Not implemented. The latest `npm --prefix web run perf:ui:budget` pass remains comfortably within budget, so adding Comlink would introduce a new worker boundary without measured need.

## Global QA Gate

After each phase:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run check:qa-static`

After native/Tauri phases:

- `npm --prefix web run parity`
- `npm --prefix web run check:tauri-capabilities`
- `cargo test -p ta_desktop_demo_app`
- `npm --prefix web run test:desktop:smoke`
- `npm --prefix web run test:desktop:geometry`

Before final handoff for broad implementation:

- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests`
- `npm --prefix web run test:e2e:qa`
- `npm --prefix web run test:e2e:workflow:full`
- `npm --prefix web run a11y:smoke:built`
- `npm --prefix web run perf:ui:budget`

## Acceptance Criteria

- No existing user workflow changes behavior unless explicitly called out in release notes.
- Existing Tauri command names remain stable through the plugin adoption phases.
- Capabilities are explicit and checked by the static capability script.
- Browser/mock test mode keeps fallbacks for desktop-only plugin APIs.
- Zod schemas validate malformed and future-extra-field payloads without breaking current tolerant normalizers.
- Async cache adoption is limited to idempotent reads until a second review.
- Generated Rust/TypeScript bindings are checked for staleness before they are used as production service contracts.
- Updater support exists as a manual/dormant release lane without adding CI/CD.
- SQLite catalog is rebuildable and never the only copy of artifact metadata.

## Reference Sources

- Tauri plugin list and permissions model: https://v2.tauri.app/plugin/
- Tauri single-instance plugin: https://tauri.ubitools.com/plugin/single-instance/
- Tauri opener plugin: https://v2.tauri.app/plugin/opener/
- Tauri dialog plugin: https://v2.tauri.app/plugin/dialog/
- Tauri clipboard manager API: https://v2.tauri.app/reference/javascript/clipboard-manager/
- Tauri updater plugin: https://v2.tauri.app/plugin/updater/
- Tauri system tray guide: https://v2.tauri.app/learn/system-tray/
- Specta and Tauri Specta: https://specta.dev/ and https://docs.rs/tauri-specta/latest/tauri_specta/
- Spacedrive architecture comparison: https://github.com/spacedriveapp/spacedrive
- Spacedrive typed hooks/cache comparison: https://v2.spacedrive.com/react/ui/hooks and https://v2.spacedrive.com/react/ui/normalized-cache
- Pinia testing: https://pinia.vuejs.org/cookbook/testing.html
- Pinia Colada quick start: https://pinia-colada.esm.dev/quick-start.html
- TanStack Query server-state guidance: https://tanstack.com/query/v4/docs/vue/guides/does-this-replace-client-state
- Zod: https://zod.dev/
- ts-pattern: https://github.com/gvergnaud/ts-pattern
- TanStack Form Vue validation: https://tanstack.com/form/latest/docs/framework/vue/guides/validation
- XState Vue: https://stately.ai/docs/xstate-vue
- Comlink: https://github.com/GoogleChromeLabs/comlink
