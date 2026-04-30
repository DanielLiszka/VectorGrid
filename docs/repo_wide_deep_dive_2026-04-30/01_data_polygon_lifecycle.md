# Data, Polygon, Cache, And Timeframe Lifecycle Implementation Plan

## Surface

Primary files:

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/settings.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/metaService.ts`
- `web/src/services/optionsExpressionService.ts`

## Current Strengths

- CSV loading has source identity and binary `.vta` cache support.
- Derived timeframe datasets are tracked as `base@timeframe` and invalidated when parents change.
- Market-data root is configurable, absolute-path checked, and reflected in diagnostics.
- Polygon historical download covers stocks, fx, and crypto, with cache validation/repair for parse errors, duplicates, ordering, and same-day gaps.
- Polygon API keys use secure keyring first, with JSON fallback and migration.
- Live Polygon streaming is scoped to stocks and tracks active stream status.
- Options data explicitly treats Polygon as the supported provider and carries manifest/provenance metadata.

## Improvement Plan

### DPL-01: Packaged Synthetic Sample Data Contract

Risk: Low to medium.

Problem:
Simple Mode depends on a sample-data path for public first-run success. The sample already exists at `src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv` and is wired through `RESOURCE_SAMPLE_CSVS`, `list_sample_csvs`, frontend sample loading, and Windows Tauri resources. The remaining work is to keep that contract deterministic and release-proven.

Implementation:

- Keep `src-tauri/tauri.windows.conf.json` as the current packaged-resource mapping authority for Windows installer resources.
- Extend `list_sample_csvs` release coverage to prove packaged-resource availability in release builds.
- Expose sample metadata: synthetic/real, timeframe, rows, date range, license/provenance.
- Add a release-gate check that the bundled sample exists and can be loaded.

Files:

- `src-tauri/src/commands_data.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `scripts/generate_release_manifest.ps1`
- `scripts/write_clean_machine_smoke_checklist.ps1`
- `web/src/services/dataService.ts`
- `web/src/components/GuidedWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `npm --prefix web run test -- src/services/dataService.test.ts src/components/GuidedWorkspacePanel.test.ts`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta`

### DPL-02: Dataset Health Summary Everywhere

Risk: Low.

Problem:
Dataset quality metadata exists in pieces, but users need a consistent health summary after CSV load, Polygon download, repair, top-off, resample, and artifact replay.

Implementation:

- Define a compact frontend `DatasetHealthSummary` model from current metadata.
- Populate status badges for parse status, gap status, duplicate/out-of-order repair, source fingerprint, cache hit/miss, row count, and timeframe.
- Add the same summary to Simple Mode, Data workspace, History artifact detail, and validation preview.

Files:

- `web/src/utils/core/datasetMeta.ts`
- `web/src/services/dataService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/validationService.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`

Verification:

- `npm --prefix web run test -- src/utils/core/datasetMeta.test.ts src/services/dataService.test.ts src/services/historyService.test.ts src/services/validationService.test.ts`
- `npm --prefix web run test:e2e:workflow:core`

### DPL-03: Polygon Error Taxonomy And Recovery Actions

Risk: Low.

Problem:
Polygon errors should map to user-recoverable classes instead of raw provider/network messages.

Implementation:

- Normalize errors into classes: missing key, unauthorized, rate limited, symbol not found, provider unavailable, bad date range, cache corrupt, repair partial, unsupported market.
- Return structured error codes from Tauri where feasible while preserving string compatibility.
- Add frontend recovery actions per class: set API key, retry later, repair cache, change symbol/market, reveal cache folder.

Files:

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/components/DataWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `npm --prefix web run test -- src/services/dataService.test.ts src/components/DataWorkspacePanel.test.ts`

### DPL-04: Cache Inventory And Retention Dry Run

Risk: Low.

Problem:
Users need to understand disk growth and cleanup consequences before deleting cached data or artifacts.

Implementation:

- Extend cache info to include size by family: CSV binary cache, Polygon cache, options datasets, run artifacts, reports, model bundles.
- Add a dry-run retention preview before cleanup: what would be deleted, how much disk would be reclaimed, and what workflows would need refetch/rebuild.
- Add copy/export of cache inventory as Markdown or JSON.

Files:

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/settingsService.ts`
- `web/src/components/SettingsWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --test settings_tests`
- `npm --prefix web run test -- src/services/settingsService.test.ts src/components/SettingsWorkspacePanel.test.ts`

### DPL-05: Live Stream Dataset Mutation Evidence

Risk: Medium.

Problem:
Live stream bars mutate loaded datasets by `data_id` and invalidate derived resamples. This should be visible when users run validation or execution after a live stream session.

Implementation:

- Record live mutation lineage on dataset context: stream source, symbol, delayed/realtime mode, first/last live bar, invalidated descendants.
- Surface a warning when a workflow uses a dataset that has been live-mutated since initial load.
- Persist that lineage in run artifacts.

Files:

- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/validationService.ts`

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `npm --prefix web run test -- src/services/workflowArtifactsSync.test.ts src/services/validationService.test.ts`

### DPL-06: Identity-Backed Polygon Cache Manifest

Risk: Medium.

Problem:
A cached CSV can be externally modified, truncated, or replaced while the manifest still reports a clean full hit, append base, or top-off base.

Implementation:

- Extend Polygon dataset metadata with file size, modified time, and content fingerprint.
- Concrete fields: `file_size_bytes`, `modified_utc_ms`, and `content_sha256`.
- Verify identity before full cache hits, append/top-off, validation, and repair.
- Specifically re-check identity before `DownloadCachePlan::FullHit`, append-only planning, `top_off_cached_dataset`, `validate_cached_dataset`, and `repair_cached_dataset`.
- On mismatch, mark the cache as needs validation or force refresh instead of trusting the manifest.
- Surface cache identity mismatch as a user-recoverable data health issue.

Files:

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/components/DataWorkspacePanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib polygon`
- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `npm --prefix web run test -- src/services/dataService.test.ts src/components/DataWorkspacePanel.test.ts`

### DPL-07: Market-Data Root And Generic Open Policy

Risk: Medium.

Problem:
If the market-data root is configured too broadly, generic open/reveal permissions can become broader than intended.

Implementation:

- Reject drive roots, user home roots, system roots, and other over-broad directories as market-data roots.
- Apply rejection both before persisting the setting and before `local_path_policy` admits the configured market root for reveal/open operations.
- Store canonicalized roots and re-check on use.
- Prefer purpose-specific reveal/open commands for reports, market data, and artifacts.
- Add path-policy tests for symlinks, traversal, roots, and app-local directories.

Files:

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/commands_reports_api.rs`
- `web/src/services/settingsService.ts`

Verification:

- `cargo test -p ta_desktop_demo_app --lib local_path_policy`
- `cargo test -p ta_desktop_demo_app --lib commands_data`
- `cargo test -p ta_desktop_demo_app --lib commands_reports_api`

## Acceptance Criteria For This Guide

- Current Polygon manifest functions remain backward-compatible with old `polygon_manifest.json` files.
- Cache identity checks never delete user data automatically; they mark stale/corrupt state and require refresh, repair, or explicit cleanup.
- Frontend recovery actions map to structured backend error classes, while legacy string errors still display safely.
- Sample-data changes are verified in both dev and packaged-resource contexts.
- Retention cleanup always has dry-run output before destructive action.

## Deferred Higher-Risk Work

- Multi-provider market data abstraction beyond Polygon.
- Quote/tick lane expansion for non-options workflows.
- Multi-symbol dataset catalog with universe-level refresh policies.
- Background top-off scheduler.
