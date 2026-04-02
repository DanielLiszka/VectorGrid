# OpenAI Codex Implementation Prompt — Bundle 14: Data Loading, Resampling, and Cache Latency

## Purpose

Use this prompt for the validated `bundle14` implementation scope only.

The original plan and addendum are only partially current. This file is the repo-grounded tracker and implementation prompt that should be followed for Bundle 14 work in this repository.

## Authoritative inputs

Read these first, in this order:

1. `bundle14_data_load_resample_cache_addendum.md`
2. `bundle14_data_load_resample_cache_plan.md`
3. `REPO_MAP.md`
4. `AGENTS.md`

When the markdown plan disagrees with the current repo, trust the repo. This prompt records those repo-state overrides.

## Repo-state validation

- [x] The bundle is still directionally valid for the current repo.
- [x] The active backend ownership is still `src-tauri/src/state.rs` for dataset residency and resampling, plus `src-tauri/src/commands_data.rs` for the Tauri command boundary.
- [x] The active frontend ownership is still `web/src/services/dataService.ts`, `web/src/stores/dataStore.ts`, `web/src/services/workflowBootstrap.ts`, and a small amount of `web/src/components/DataWorkspacePanel.vue`.
- [x] Base in-memory reuse is still broken because `find_matching_loaded_base(...)` relies on `LoadedDataContext.timeframe.is_none()` instead of actual dataset kind.
- [x] Load-path truthfulness is still incomplete because `LoadPriceDataResponse` only exposes `cache_hit` and `cache_path`.
- [x] Reused derived resamples still report a fake `cache_path` by returning the source CSV path.
- [x] Frontend clear is still frontend-only; there is no `clear_loaded_data` Tauri command yet.
- [x] Startup restore is still not real; `hydrateStartupState()` currently fetches metadata only.
- [x] `applyLegacySnapshot(...)` still restores stale runtime dataset and Polygon-cache state that should not be trusted as live data.
- [x] Normal startup still eagerly scans Polygon cache metadata.
- [x] Polygon open/download/API-key flows still perform redundant cache rescans.
- [x] `dataService.ts` still does redundant `get_market_data_dir_info` round trips after `set_market_data_dir` and `clear_market_data_dir`, even though the backend commands already return `MarketDataDirInfo`.
- [x] The repo already centralizes storage helpers in `web/src/utils/storage.ts` and storage keys in `web/src/utils/core/storageKeys.ts`; do not add ad hoc `localStorage` access for the new startup snapshot.
- [x] The repo already has relevant tests in `src-tauri/src/state.rs`, `web/src/services/dataService.test.ts`, `web/src/stores/dataStore.actions.test.ts`, `web/src/stores/domainStores.test.ts`, `web/src/components/DataWorkspacePanel.test.ts`, `web/src/services/workflowBootstrap.test.ts`, and `web/src/tauri/commands.test.ts`.
- [x] The repo already has a relevant app-path measurement surface in `src-tauri/src/bin/ta_app_path_faceoff.rs`; a new benchmark harness is not required for this bundle.

## Active implementation scope

Only the unchecked items below are in active implementation scope.

### Backend and command contract

- [x] `src-tauri/src/state.rs`: add a serialized `LoadSource` enum and extend `LoadPriceDataResponse` with `load_source`.
- [x] `src-tauri/src/state.rs`: fix identical-base reuse so it uses dataset kind (`DatasetKind::Base`) rather than `LoadedDataContext.timeframe.is_none()`.
- [x] `src-tauri/src/state.rs`: keep `cache_hit` backward-compatible, but return truthful `load_source` values for base reuse, binary-cache hits, cold CSV parse paths, fresh derived resamples, and reused derived resamples.
- [x] `src-tauri/src/state.rs`: stop returning a fake cache path on in-memory reuse paths; repeated derived resamples must return empty `cache_path`.
- [x] `src-tauri/src/commands_data.rs`: add a real `clear_loaded_data` command that clears backend dataset state idempotently for the active base dataset and descendants.
- [x] `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs`: register the new `clear_loaded_data` command.
- [x] `web/src/tauri/commands.ts` and `web/src/tauri/commands.test.ts`: regenerate or update the typed command registry if the command surface changes.

### Service layer

- [x] `web/src/services/dataService.ts`: add a normalized `DataLoadSource` type aligned with the Rust enum.
- [x] `web/src/services/dataService.ts`: extend normalized load results to carry `loadSource`.
- [x] `web/src/services/dataService.ts`: make `clearData()` invoke the real backend clear command instead of remaining a no-op.
- [x] `web/src/services/dataService.ts`: stop doing redundant `get_market_data_dir_info` reads after `set_market_data_dir` and `clear_market_data_dir`; normalize the returned payload directly.
- [x] `web/src/services/dataService.test.ts`: update expectations for `load_source`, the real clear command, and the reduced market-data-dir round trips.

### Store, startup restore, and Polygon metadata latency

- [x] `web/src/utils/core/storageKeys.ts`: add a dedicated storage key for the startup dataset snapshot if one does not already exist.
- [x] `web/src/stores/dataStore.ts`: add durable loaded-base identity fields, including `baseCsvPath`, `baseSourceKind`, `baseSourceSizeBytes`, and `baseSourceMtimeUtcMs`.
- [x] `web/src/stores/dataStore.ts`: persist startup restore state from the loaded base dataset, not from the editable `csvPath` input.
- [x] `web/src/stores/dataStore.ts`: add truthful helper text derived from `loadSource` instead of inferring "binary cache" from `cacheHit`.
- [x] `web/src/stores/dataStore.ts`: update `datasetContextText(...)` and load status text so source kind, load source, and real cache basename are reported truthfully.
- [x] `web/src/stores/dataStore.ts`: make `applyLoadResult(...)` preserve base identity correctly across derived loads and persist the startup snapshot after successful base-load and timeframe-apply paths.
- [x] `web/src/stores/dataStore.ts`: make `applyBaseDatasetState()` persist the startup snapshot when returning to base timeframe.
- [x] `web/src/stores/dataStore.ts`: sanitize `applyLegacySnapshot(...)` so it no longer trusts stale runtime dataset ids, runtime summaries, transient `can*` flags, or persisted Polygon cache/runtime summaries as live state.
- [x] `web/src/stores/dataStore.ts`: make `hydrateStartupState()` async, idempotent, promise-deduped, and capable of real startup restore using the saved base path and selected timeframe.
- [x] `web/src/stores/dataStore.ts`: keep startup restore truthful on failure by clearing stale runtime state and surfacing a real restore-failed status.
- [x] `web/src/stores/dataStore.ts`: remove the eager Polygon cache scan from default startup hydration.
- [x] `web/src/stores/dataStore.ts`: add a small `resetPolygonCacheRuntimeState(...)` helper and use it when market-data-root changes invalidate in-memory Polygon cache summaries.
- [x] `web/src/stores/dataStore.ts`: make Polygon cache loading lazy and promise-deduped for open/refresh overlap cases.
- [x] `web/src/stores/dataStore.ts`: remove `setControlsBusy(state, false)` from `applyPolygonCacheInfo(...)`; the helper should only apply payload to state.
- [x] `web/src/stores/dataStore.ts`: avoid a full `getPolygonCacheInfo()` rescan after successful `downloadPolygonDataset()` by locally upserting the downloaded dataset summary and reselecting it.
- [x] `web/src/stores/dataStore.ts`: keep market-data-root update flows efficient by using the normalized returned payload directly and resetting Polygon runtime state only when the effective cache dir changes.
- [x] `web/src/stores/dataStore.ts`: make `clearData()` clear backend state when a base dataset exists, then clear frontend dataset/artifact state and persist an empty startup snapshot.
- [x] `web/src/stores/dataStore.actions.test.ts`: update and extend tests for real startup restore, lazy Polygon metadata loading, clear behavior, load-source truthfulness, and market-data-root reset behavior.
- [x] `web/src/stores/domainStores.test.ts`: update legacy snapshot expectations so stale runtime dataset/cache state is no longer treated as valid restored state.

### Bootstrap and panel behavior

- [x] `web/src/services/workflowBootstrap.ts`: call `dataStore.hydrateStartupState()` after Pinia is available even when there is no saved workflow bootstrap state.
- [x] `web/src/services/workflowBootstrap.test.ts`: cover the new data-store bootstrap call and preserve existing workflow bootstrap behavior.
- [x] `web/src/components/DataWorkspacePanel.vue`: keep the existing component-mount hydration call only as an idempotent fallback once the store-level hydrate guard exists.
- [x] `web/src/components/DataWorkspacePanel.vue`: if needed, make only small truthful panel updates, such as showing the effective/base timeframe chip from real store state.
- [x] `web/src/components/DataWorkspacePanel.test.ts`: update tests only as needed for the small panel/store contract changes.

## Explicit exclusions and deferred items

These items were mentioned in the original plan/addendum but are not part of the active mandatory implementation scope for the current repo state.

- [x] Do not move dataset restore logic into `web/src/services/settingsBootstrap.ts`.
- [x] Do not make normalized Rust-side `source_key` matching mandatory in this bundle. It is a valid later improvement, but it is not required to fix the current broken base-reuse discriminator.
- [x] Do not add a `source_path` field to `LoadPriceDataResponse` in the mandatory patch.
- [x] Do not do broad `src-tauri/src/data_cache.rs` header, fingerprint, or cache-format redesign in the mandatory patch.
- [x] Do not turn `load_price_data` or `resample_price_data` into a broad `spawn_blocking` refactor in this bundle.
- [x] Do not require direct edits in `src-tauri/src/polygon.rs` or `src-tauri/src/settings.rs` unless a small contract fix becomes strictly necessary while implementing the active scope.
- [x] Do not add a new benchmark harness just for this bundle. Use existing tests plus `src-tauri/src/bin/ta_app_path_faceoff.rs` if extra latency evidence is needed.
- [x] Do not treat API-key save/clear local-state optimization as mandatory. If it stays tiny and obvious it may be folded in, otherwise leave it alone.
- [x] Do not make startup snapshot identity-comparison or path-move reconciliation mandatory for this bundle.

## Implementation guardrails

- Preserve existing visible workflows.
- Keep `cache_hit` compatible for existing consumers, but make the frontend use `loadSource` for user-visible truth.
- Use the repo's existing storage helpers and storage-key conventions instead of raw `localStorage`.
- Keep Polygon open/close service hooks non-throwing.
- Keep changes local and parity-safe; avoid opportunistic refactors.
- If command registration changes, keep the generated command list and command-count test consistent.
- Prefer updating existing tests over creating new parallel test files.

## Verification checklist

- [x] `cargo test -p ta_desktop_demo_app state`
- [x] `cargo test -p ta_desktop_demo_app commands_data`
- [x] `npm --prefix web run check:commands`
- [x] `npm --prefix web run test -- --run src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/stores/domainStores.test.ts src/services/workflowBootstrap.test.ts src/components/DataWorkspacePanel.test.ts src/stores/commandActionStore.test.ts src/services/metaService.test.ts src/tauri/commands.test.ts`
- [x] `npm --prefix web run typecheck`
- [x] Automated coverage now exercises truthful load-source normalization, stable base/derived reuse IDs with empty reuse `cache_path`, backend-aware clear behavior, startup restore success paths, lazy Polygon metadata loading, and market-data-root reset behavior via the updated Rust and Vitest surfaces.
- [x] `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

## Final response requirements for the implementation run

When implementing this bundle, the final response should:

- say whether Bundle 14 is fully complete for the validated scope
- list the actual files changed
- state which verification commands were run and which passed
- call out any intentionally deferred optional items that were left untouched
- mention any unrelated pre-existing warnings or failures that are not bundle blockers
