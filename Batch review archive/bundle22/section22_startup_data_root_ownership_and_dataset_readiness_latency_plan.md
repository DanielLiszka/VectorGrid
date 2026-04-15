# Section 22 implementation plan — startup data-root ownership and dataset-readiness latency

## Goal

Improve startup truthfulness and time-to-usable-dataset without turning this section into a broad settings or data-cache redesign.

This first wave should stay focused on:

- making the app's startup-ready boundary truthful relative to critical data bootstrap work
- keeping critical startup data-root and dataset-restore work separate from deferred affordance work
- preserving data-root ownership when Polygon datasets are moved to a new configured root
- tightening the verification around startup restore and data-root changes

This is not a generic settings UX pass and not a broad `polygon.rs` / `data_cache.rs` rewrite.

## Repo-grounded current state

### What is already implemented

- `src-tauri/src/settings.rs` already owns settings-backed market-data root resolution through:
  - `AppSettings.market_data_root_dir`
  - `configured_market_data_root_dir(...)`
  - `resolve_market_data_root_dir(...)`
  - `set_market_data_root_dir(...)`
- `src-tauri/src/commands_data.rs` already owns the Tauri command surface for:
  - `get_market_data_dir_info(...)`
  - `set_market_data_dir(...)`
  - `clear_market_data_dir(...)`
  - `polygon_get_cache_info(...)`
  - `polygon_validate_dataset(...)`
  - `polygon_repair_dataset(...)`
  - `polygon_download_1m_and_load(...)`
- `src-tauri/src/state.rs` already has truthful base/derived load-source and in-memory reuse semantics. This bundle should not reopen earlier fake `cache_path` / `load_source` fixes.
- `web/src/stores/dataStore.ts` already owns startup dataset restoration through `hydrateStartupState()`, startup snapshot persistence, Polygon cache lazy loading, and data-root hint state.
- `web/src/services/dataService.ts` already normalizes the backend’s truthful load-root / cache-root responses and exposes the command boundary cleanly.
- `web/src/stores/settingsStore.ts` already mirrors market-data-root state from `dataStore`; the Settings surface is already a thin forwarder, not the true owner.
- `web/src/main.ts` already mounts the app before running the boot sequence, so startup changes do not need to block initial shell mount.
- `src-tauri/src/data_cache.rs` is not a startup owner. It is the explicit CSV load path, not the shell bootstrap path.

### Where startup ownership or latency is still split

- `web/src/services/workflowBootstrap.ts` triggers `useDataStore(...).hydrateStartupState()` but does not return or await that async startup work.
- `web/src/main.ts` calls `applyWorkflowBootstrapFromStorage()` synchronously and then marks the runtime ready after desktop-service setup, not after critical data-root / dataset-restore work completes.
- `web/src/stores/dataStore.ts` mixes critical startup work with optional affordance work in `hydrateStartupState()`:
  - `getMarketDataDirInfoAction()`
  - startup snapshot restore
  - sample CSV listing
  - optional Polygon cache scan when `polygonOpen` is restored
- `web/src/stores/dataStore.ts` persists an absolute startup restore path. When `setMarketDataDir(..., moveExisting=true)` moves a cached Polygon dataset to a new root, the current startup snapshot path is not rewritten to the new cache root.
- `web/src/stores/dataStore.ts::applyLegacySnapshot(...)` can restore `polygonOpen`, and `hydrateStartupState()` will then eagerly scan Polygon cache on startup even though the shell only needs dataset-readiness, not modal cache contents, to become usable.

## Current repo-valid problems

1. The runtime-ready boundary is not yet truthful for startup dataset readiness.
   - `web/src/main.ts` marks the runtime ready before `hydrateStartupState()` necessarily finishes.
   - The current startup banner / overlay can therefore hand off before critical dataset restore work is done.

2. Critical startup work is still mixed with deferred affordance work.
   - `dataStore.hydrateStartupState()` always fetches market-data-dir info and sample CSVs up front.
   - It may also scan Polygon cache immediately when stale UI state restores `polygonOpen`.
   - Dataset-readiness and modal affordance readiness are not the same boundary.

3. Polygon data-root moves can leave stale startup restore paths behind.
   - A cached Polygon dataset may be moved from the old cache root to the new cache root.
   - The persisted startup snapshot still points at the old absolute CSV path.
   - Next startup then pays a failed restore path before the user can become dataset-ready again.

4. Verification exists for pieces but not for the startup ownership boundary as a whole.
   - There is good coverage for `settings.rs`, `workflowBootstrap.ts`, and `dataStore.ts` in isolation.
   - There is not yet a tight first-wave checklist around truthful startup-ready sequencing and moved-root snapshot behavior.

## First-wave implementation scope

### Phase 1 — truthful startup-ready sequencing

- Make the workflow bootstrap path capable of returning the critical startup data hydration promise.
- Keep initial app mount early.
- Do not block on non-critical affordance work to declare the runtime ready.
- Ensure the boot stage / ready handoff reflects:
  - storage bootstrap
  - desktop-service setup
  - critical data-root info + dataset auto-restore work

### Phase 2 — split critical startup data work from deferred affordance work

- Treat these as critical startup work:
  - market-data-root info needed for truthful root ownership state
  - startup dataset snapshot restore
  - derived timeframe restore when a startup snapshot requests one
- Treat these as deferred affordance work:
  - sample CSV listing
  - Polygon cache dataset scan when the modal is not actually needed for startup readiness
- Prevent stale restored `polygonOpen` state from forcing eager cache scans during startup.

### Phase 3 — preserve startup ownership when the Polygon root moves

- On successful `setMarketDataDir(..., moveExisting=true)`, detect when the currently loaded base dataset is a Polygon cached dataset under the old cache root.
- Rewrite the in-memory path-bearing frontend state to the new cache root when that move actually happened:
  - `baseCsvPath`
  - `csvPath` when applicable
  - `polygonSelectedPath` when applicable
  - the persisted startup snapshot
- Keep non-Polygon datasets and non-move root changes unchanged.
- Do not introduce a new persisted artifact format for this first wave.

### Phase 4 — focused verification and fallout

- Extend the startup unit tests to cover:
  - truthful ready-boundary sequencing
  - deferred Polygon cache loading
  - moved-root startup snapshot rewrite
- Touch Rust command tests only if the backend command surface changes.
- Keep desktop smoke and a11y/parity scripts as optional only unless the startup flow change actually needs them.

## Explicit defers

- Do not use this section for a generic settings-panel redesign.
- Do not use this section for broad `polygon.rs` or `data_cache.rs` refactors.
- Do not add a new startup command just to bundle unrelated data surfaces together unless later measurement shows the frontend still needs extra startup fanout after the sequencing cleanup.
- Do not redesign sample-dataset discovery or broaden it into repo-data indexing work.
- Do not make Polygon cache validation/repair/download part of the critical startup path.
- Do not replace the existing startup snapshot format with a broader persistence schema in the first wave.
- Do not reopen earlier data-load truthfulness fixes from Section 14.

## Concrete implementation checklist

### Startup sequencing

- [ ] Make `web/src/services/workflowBootstrap.ts` capable of returning/awaiting the critical data bootstrap work it starts.
- [ ] Update `web/src/main.ts` so runtime `markReady()` happens after critical startup data hydration, not before it.
- [ ] Keep initial app mount early and preserve existing boot error handling.
- [ ] Keep the boot stage text truthful for the new sequencing boundary.

### Critical vs deferred startup work

- [ ] Split critical startup data-root / dataset-restore work from deferred affordance work inside `web/src/stores/dataStore.ts`.
- [ ] Keep market-data-root info and startup dataset restore on the critical path.
- [ ] Keep sample CSV listing off the critical ready boundary if it is not required for dataset-readiness.
- [ ] Prevent stale restored `polygonOpen` state from forcing a startup Polygon cache scan when the shell only needs dataset readiness.
- [ ] Preserve explicit lazy Polygon cache loading on modal open / explicit refresh.

### Data-root ownership correctness

- [ ] Detect successful Polygon cache-root moves in `web/src/stores/dataStore.ts` when `setMarketDataDir(..., moveExisting=true)` changes the effective cache directory.
- [ ] Rewrite current Polygon path-bearing state and the startup snapshot to the new cache root when the loaded dataset was moved.
- [ ] Keep manual CSV datasets and non-move root changes unchanged.
- [ ] Add targeted tests for moved-root snapshot correctness.

## Verification plan

- `npm --prefix web run test -- --run src/services/workflowBootstrap.test.ts src/stores/dataStore.actions.test.ts`
- `npm --prefix web run test -- --run src/stores/settingsStore.actions.test.ts` if market-data-root sync behavior changes
- `npm --prefix web run test -- --run src/components/StartupAnimationOverlay.test.ts` if runtime-ready timing or boot-stage wording changes
- `npm --prefix web run typecheck`
- `cargo test -p ta_desktop_demo_app --test settings_tests`
- `cargo test -p ta_desktop_demo_app commands_data` only if command-layer behavior changes
- `npm --prefix web run check:commands` only if the Tauri command surface changes
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` if startup sequencing changes the user-visible readiness path

## Acceptance criteria

- the runtime-ready boundary no longer claims readiness before critical startup data-root / dataset-restore work completes
- startup no longer eagerly scans Polygon cache just because stale UI state restored the modal-open flag
- moving a Polygon cache root with `moveExisting=true` no longer leaves a stale startup restore path behind
- current market-data-root ownership remains centralized where it already lives today
- sample lists, cache scans, and other affordance work stay available without dominating first usable dataset state
- the bundle stays on startup ownership and latency, not on broad settings or cache redesign
