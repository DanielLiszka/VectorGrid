# Price Data Residency And VRAM Cache Implementation Plan

Date: 2026-03-12
Status: Implemented core residency/cache path; benchmark-harness follow-up aligned
Scope: host-memory residency, bounded dataset retention, lazy raw-field loading, and resident GPU market-series caching

## Goal

Reduce repeated price-data overhead in real app usage by:

- replacing per-command `Candles` cloning with shared host-memory ownership
- bounding system-memory retention to the active dataset footprint
- keeping one active dataset lazily resident in VRAM for repeated GPU optimize/validate calls
- preserving current optimize/validate API compatibility and CPU fallback behavior

This plan is intentionally about price-data residency and upload reuse. It is not a generic strategy-execution rewrite.

## Current Repo Truth

- The app now keeps loaded data in shared host memory in [state.rs](../src-tauri/src/state.rs).
  - `AppState` stores `Arc<Candles>` inside versioned dataset entries.
  - `get_candles` now returns shared `Arc<Candles>` handles.
- Optimize and validation commands already fetch by `data_id` from app state in [commands_optimize.rs](../src-tauri/src/commands_optimize.rs) and [commands_validation.rs](../src-tauri/src/commands_validation.rs).
- Resampled datasets are cached under derived ids in [state.rs](../src-tauri/src/state.rs), and stale derived datasets are invalidated when the source dataset version changes.
- `VTA` is the current binary-cache load path in [data_cache.rs](../src-tauri/src/data_cache.rs), and the frontend already surfaces binary-cache hits in [dataStore.ts](../web/src/stores/dataStore.ts).
- CUDA runtime reuse exists per thread/device in [gpu_backtest_kernel.rs](../crates/app_core/src/gpu_backtest_kernel.rs), and the current branch now also contains a resident market-series cache for the double-MA kernel path.
- Shared CUDA indicator prep in [dynamic_indicator_adapter.rs](../crates/app_core/src/dynamic_indicator_adapter.rs) now reuses cached host/device OHLCV inputs per device and dataset identity.
- `double_ma` already supports user-selected `ma_source` through backend and frontend plumbing:
  - request field in [double_ma.rs](../crates/app_core/src/double_ma.rs)
  - UI control in [AdvancedWorkspacePanel.vue](../web/src/components/AdvancedWorkspacePanel.vue)
  - request shaping in [optimizeRequestBuilder.ts](../web/src/services/optimizeRequestBuilder.ts) and [validationRequestBuilder.ts](../web/src/services/validationRequestBuilder.ts)
  - raw-field requirement checks in [candleFields.ts](../web/src/utils/core/candleFields.ts)
  - GPU-side source validation in [gpu_backtest_kernel.rs](../crates/app_core/src/gpu_backtest_kernel.rs)

## Design Decisions

### 1. Host ownership model

Adopt `Arc<Candles>` in Tauri app state instead of returning owned clones.

Why:

- preserves current `ta_app_core` signatures because call sites can pass `candles.as_ref()`
- removes repeated command-boundary candle cloning
- gives a clean immutability boundary for later dataset-version invalidation

### 2. Bounded retention policy

Use a bounded active-dataset policy rather than keeping an unbounded `data_id -> Candles` cache alive forever.

Target policy:

- one active base dataset
- zero or one active derived timeframe dataset
- one resident raw-device dataset per GPU device

This should still preserve current UX semantics for “load once, run many” and “resample once, run many”.

### 3. Versioned invalidation

Every resident dataset should carry a monotonic version. Version must bump on:

- new dataset load
- timeframe derivation replacement
- live-bar mutation or append
- explicit data replacement from any import/fetch path

### 4. Raw-field cache, not strategy cache

The resident device cache should be keyed by raw field availability, not by strategy name and not by derived `ma_source` series identity.

Recommended device-cache identity:

- `device_id`
- `dataset_id`
- `dataset_version`
- `timeframe_key`
- `field_mask`

`ma_source` should not be part of the resident raw-dataset key if the cache stores raw fields only.

### 5. Field-mask policy

The host/device residency layer should request only the raw fields actually needed by the strategy route.

Initial source mapping for single-series strategies:

- `close` -> `close`
- `open` -> `open`
- `high` -> `high`
- `low` -> `low`
- `hl2` -> `high`, `low`
- `hlc3` -> `high`, `low`, `close`
- `hlcc` -> `high`, `low`, `close`
- `ohlc4` -> `open`, `high`, `low`, `close`

Practical implication:

- `double_ma` with `close` should not require full OHLCV residency
- `double_ma` with `hlc3` should not require `open` or `volume`
- full-candle strategies can request wider field masks

### 6. Lazy first-use VRAM upload

Do not eagerly upload on load. Upload on first GPU optimize/validate call for the active dataset and retain until invalidated.

Why:

- not every loaded dataset is used on GPU
- not every route needs the same fields
- invalidation remains simple and explicit

## Non-Goals

- no VTA format redesign
- no change to user-visible optimize/validate request schemas
- no forced full-strategy rewrite around a new generic execution layer
- no requirement that every strategy use the exact same field mask
- no optimization-to-validation chained GPU state dependency; validation remains independently executable

## Implementation Phases

## Phase 1: State Ownership Refactor

- [x] Change `AppState` candle storage from `Candles` to `Arc<Candles>` in [state.rs](../src-tauri/src/state.rs).
- [x] Change `get_candles` to return `Arc<Candles>`.
- [x] Update optimize command entry points in [commands_optimize.rs](../src-tauri/src/commands_optimize.rs) to move `Arc<Candles>` into workers and pass `candles.as_ref()` into core calls.
- [x] Update validation command entry points in [commands_validation.rs](../src-tauri/src/commands_validation.rs) the same way.
- [x] Audit remaining command/helper paths that assumed owned `Candles`, including live execution in [commands_execution.rs](../src-tauri/src/commands_execution.rs).
- [x] Keep CPU-only builds working with no CUDA toolchain.

## Phase 2: Bounded Active-Dataset Policy

- [x] Replace unbounded “many loaded datasets” retention with explicit active-base and active-derived tracking in [state.rs](../src-tauri/src/state.rs).
- [x] Keep current `data_id` semantics intact for the active dataset.
- [x] Preserve derived timeframe behavior, but store only the current active derived dataset.
- [x] Make derived timeframe replacement invalidate the previous derived dataset rather than accumulating cached entries.
- [x] Ensure live-data update paths replace the active dataset atomically instead of mutating shared state in place.

## Phase 3: Dataset Identity And Versioning

- [x] Introduce a small host-side dataset metadata struct: dataset id, timeframe key, version, field availability.
- [x] Add version bumping on load/import/fetch.
- [x] Add version bumping on resample replacement.
- [x] Add version bumping on live-bar append/update.
- [x] Ensure command paths can obtain the active dataset identity together with `Arc<Candles>`.

## Phase 4: Raw-Field Requirement Model

- [ ] Add a shared backend helper for converting route requirements into a raw field mask.
- [x] Keep `ma_source` support aligned with the currently supported values in the frontend and GPU upload path.
- [x] Start with a small field-mask enum/bitset that covers `open`, `high`, `low`, `close`, `volume` where resident-cache paths use it.
- [ ] Add route-level helpers for:
  - [ ] `double_ma` and double-like families
  - [ ] composite GPU routes that already rely on `PreparedCudaIndicatorInputs`
  - [ ] validation-stage rerun routes
- [ ] Keep strategy-local indicator staging free to request a superset when needed.

## Phase 5: Resident Device Raw-Series Cache

- [x] Add a resident device-dataset cache near [gpu_backtest_kernel.rs](../crates/app_core/src/gpu_backtest_kernel.rs), not in Tauri app state.
- [x] Key it by `device_id + dataset identity + field requirements`.
- [x] Start with one resident dataset per device for the active dataset.
- [x] Support lazy field-mask widening:
  - if `close` is resident and a later request needs `high+low+close`, promote to the wider field mask
- [x] Reuse the existing thread/device runtime boundary rather than inventing a separate ownership model.
- [x] Add explicit invalidation on dataset identity/version mismatch.

## Phase 6: Shared GPU Upload Integration

- [x] Refactor the current raw-series upload path in [gpu_backtest_kernel.rs](../crates/app_core/src/gpu_backtest_kernel.rs) so optimize/validate routes can reuse resident device buffers when the cache hits.
- [x] Keep current CPU fallback behavior unchanged on cache miss or GPU error.
- [x] Preserve current source validation and required-field checks.
- [x] Keep compact result-return behavior unchanged.

## Phase 7: Strategy Integration

- [x] Integrate `double_ma` optimize routes first.
- [x] Integrate `double_ma` validation routes next.
- [x] Integrate composite GPU routes that already use shared CUDA prep/upload surfaces.
- [x] Treat strategy-local work as adaptation to shared raw-field residency, not as separate strategy-specific VRAM caches.
- [x] Keep routes that truly require broader OHLCV or volume access able to request it explicitly.

## Phase 8: Faceoff And Benchmark Alignment

- [x] Rework the VectorBT faceoff methodology so the app side reflects real product usage:
  - load once
  - optional timeframe selection once
  - repeated optimize/validate runs against the already-loaded dataset
- [x] Keep the existing CLI-style wall-clock workflow as a separate cold-start measurement if needed.
- [x] Add a warm-path benchmark or scripted app-path measurement for resident-data runs.

## Execution Notes

- Host-side `Arc<Candles>` residency, bounded retention, dataset versioning, and derived invalidation are already implemented in the current `state.rs` branch shape.
- The live execution command path was aligned with shared candle ownership so it no longer forces an owned `Candles` copy when loading from app state.
- The CUDA double-MA kernel file already contains a resident market-series cache in the current branch; that path should be treated as existing implementation, not new scope introduced by this pass.
- A dedicated warm-path faceoff runner now exists in [ta_app_path_faceoff.rs](../src-tauri/src/bin/ta_app_path_faceoff.rs), and [run_vectorbt_vs_vectorgrid_faceoff.ps1](../scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1) now defaults the app side to `app-path` mode while still preserving `cli-cold`.

## Verification Checklist

- [x] `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- [x] `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- [ ] `cargo test -p ta_app_core double_ma --features cuda,cuda-backtest-kernel`
- [x] `cargo test -p ta_app_core validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel`
- [x] `cargo test -p ta_app_core advanced_routes_smoke --features cuda,cuda-backtest-kernel`
- [x] `cargo test -p ta_desktop_demo_app cross_ticker_ --features cuda,cuda-backtest-kernel`
- [ ] `npm --prefix ./web run typecheck`
- [ ] `npm --prefix ./web run test -- optimizeRequestBuilder.test.ts`
- [ ] `npm --prefix ./web run test -- validationRequestBuilder.test.ts`
- [x] `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- [ ] Add a targeted before/after measurement for:
  - [x] repeated `double_ma` optimize on already-loaded data
  - [ ] repeated validation on already-loaded data
  - [x] first-run cold upload vs warm resident upload

Verification notes:

- The GPU perf gate now passes again after caching successful dynamic GPU preflight compilation for repeated identical requests in [double_ma.rs](../crates/app_core/src/double_ma.rs).
- The faceoff wrapper now auto-selects the dedicated repo venv at `.venv-vectorbt-faceoff\Scripts\python.exe` when `-PythonExe` is not overridden.
- A smoke run of the warm app-path faceoff wrapper completed end to end and wrote artifacts under `target/vectorbt_vs_vectorgrid_faceoff_smoke/`.

## Risks And Guardrails

- Shared ownership means live-update paths must switch from in-place mutation to replace-on-write semantics.
- A resident device cache must remain device-local and version-aware to avoid stale-buffer reuse.
- The first implementation should prefer correctness and explicit invalidation over multi-dataset cache complexity.
- Field-mask widening must not silently under-provision required data for strategy-local indicators.

## Success Criteria

- Repeated optimize/validate runs on the same loaded dataset avoid host `Candles` cloning.
- Repeated GPU optimize/validate runs avoid redundant market-series upload when dataset identity and field requirements are unchanged.
- Existing parity tests and validation checks continue to pass.
- The app-path faceoff methodology becomes closer to real user behavior than the current fresh-process CLI loop.

## Immediate Execution Order

- [x] Implement Phase 1 first.
- [x] Implement Phase 3 alongside Phase 2 so invalidation has a real identity model from the start.
- [x] Implement Phase 5 and Phase 6 for `double_ma` before broadening to composite routes.
- [x] Re-run parity/perf checks before expanding strategy coverage.
