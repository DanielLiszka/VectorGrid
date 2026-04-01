# OpenAI Codex Implementation Prompt — Bundle 14: Data Loading, Resampling, and Cache Latency

## Purpose

Use this prompt with an OpenAI Codex agent to implement the **Bundle 14** data-load / resample / cache-latency plan in the repo.

This prompt is intentionally implementation-oriented. It is designed to produce a working patch set, not another review.

---

## Copy/paste prompt for Codex

You are implementing a repo-grounded performance/correctness patch series for the **Bundle 14: data loading, resampling, and cache latency** surface.

Your job is to **fully implement the mandatory plan**, plus the **low-risk addendum corrections and strong optional improvements**, while preserving existing functionality and avoiding broad redesign.

### Authoritative documents to read first

Read these documents before editing code:

1. `bundle14_data_load_resample_cache_addendum.md`  
   - This contains corrections and low-risk additions.  
   - **If it conflicts with the original plan, the addendum wins.**
2. `bundle14_data_load_resample_cache_plan.md`  
   - This is the main implementation spec.
3. `gpt54_feature_review_map.md`  
   - Use only the **Bundle 14** section and the prompt rules / scope guard from that file.

If these docs are attached rather than committed in the repo, treat the attached copies as authoritative.

### Scope guard

Stay inside Bundle 14.

This patch series is about:

- real local CSV load latency
- warm binary-cache hit behavior
- warm in-memory base reuse behavior
- fresh vs reused timeframe resamples
- truthful load-source / cache-path reporting
- metadata-scan overhead that affects startup or dataset-switch latency
- startup dataset readiness and truthfulness
- clearing backend + frontend dataset state consistently
- low-risk Polygon cache metadata-scan cleanup that directly affects this surface

Do **not** drift into:

- broad UI redesign
- new product features
- moving controls to different product areas
- speculative architecture rewrites
- broad cache-format redesign
- large async/runtime refactors unless explicitly called out below as deferred

### Required outcome

After your patch set lands, the product should behave like this:

1. **Cold CSV load**
   - Parse CSV.
   - Attempt to write `.vta` cache.
   - Return truthful metadata saying whether it was:
     - `csv_parsed_and_cached`, or
     - `csv_parsed_cache_write_skipped`.

2. **Warm binary-cache load**
   - Reuse `.vta`.
   - Return `load_source = binary_cache`.
   - Return a real `cache_path` only if the path points to an actual `.vta` file used for this load.

3. **Warm in-memory identical base reload**
   - Reuse the already-loaded base dataset.
   - Preserve the same base dataset ID.
   - Preserve derived datasets hanging off that base.
   - Return `load_source = in_memory_base_reuse`.
   - Do **not** pretend the cache path is a `.vta` if no `.vta` was used for this response.

4. **Fresh derived timeframe build**
   - Resample from the loaded base dataset in memory.
   - Return `load_source = derived_fresh_resample`.
   - Return empty `cache_path`.

5. **Repeated same derived timeframe request**
   - Reuse the existing derived dataset in memory.
   - Preserve the same derived dataset ID.
   - Return `load_source = derived_in_memory_reuse`.
   - Return empty `cache_path`.

6. **Clear**
   - Clear frontend UI state.
   - Clear backend in-memory dataset state for the active base dataset and descendants.
   - Update startup-restore snapshot state so the cleared dataset is not restored on next launch.

7. **Startup restore**
   - If enabled, restore the last successfully loaded base dataset and selected timeframe.
   - Do not restore stale backend dataset IDs/metadata from a saved UI snapshot.
   - Do not eagerly perform unrelated Polygon cache scans on startup.
   - Be safe if startup hydration is triggered from both bootstrap and component mount.

8. **Market-data-root / Polygon metadata correctness**
   - Changing or resetting the market-data root must clear stale in-memory Polygon cache summaries in the store.
   - Opening Polygon should lazy-load cache metadata.
   - Avoid redundant metadata round trips when backend commands already return the needed info.

### Hard constraints

- Preserve existing features and visible workflows.
- Prefer small, parity-safe changes over large refactors.
- Keep `cache_hit` backward-compatible enough not to break existing consumers, but make all provided frontend code prefer `load_source` for user-visible truth.
- Do not make `settingsBootstrap.ts` responsible for dataset restore logic.
- Do not implement broad `data_cache.rs` fingerprint redesign in the mandatory patch.
- Do not move load/resample fully into `spawn_blocking` in this patch unless you are doing the explicitly described two-stage follow-up and it stays very small. By default, **defer async/offload work**.
- If you need to touch adjacent wiring files not listed below (for example Tauri command registration or storage-key helpers), keep those edits minimal and directly in service of this patch.

---

## Primary files to modify

These are the core implementation targets:

- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/workflowBootstrap.ts`

### Files to inspect and change only if needed

- `src-tauri/src/data_cache.rs`
  - Only for small helper reuse, comments, or a narrowly scoped test placeholder / no-op cleanup.
  - Do **not** do broad cache-format churn in the mandatory patch.
- `web/src/services/settingsBootstrap.ts`
  - No functional dataset-restore logic should move here.
  - Optional comment only if useful.

### Likely adjacent wiring files you may need to update

These were not in the review bundle, but you should locate and update them if required by the repo structure:

- Tauri command registration file(s), likely one of:
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/main.rs`
  - or the file that contains `tauri::generate_handler!`
- Any typed invoke-command registry or frontend Tauri wrapper, if the repo enforces one.
- If the repo centralizes storage key constants, update that file instead of using a local inline key only if doing so is clearly the existing convention.
- If the repo already has web unit tests for this surface, update or add tests in those existing locations.

---

## Required implementation order

Implement in this order unless repo constraints force a tiny reordering:

1. Rust response truthfulness + base-reuse fix
2. Backend clear command
3. Service-layer normalization changes
4. Store startup snapshot / restore / truthful status changes
5. Lazy Polygon metadata changes and market-data-root runtime reset
6. Bootstrap wiring
7. Small Data panel correctness polish
8. Tests
9. Only then consider the low-risk optional improvements listed later

---

## Detailed implementation instructions

# 1. Rust: `src-tauri/src/state.rs`

## 1.1 Add a first-class load-source enum

Add a serialized load-source enum and extend `LoadPriceDataResponse`.

Use serde snake_case values.

Use these exact semantic variants unless the repo already has an equivalent existing type:

- `binary_cache`
- `csv_parsed_and_cached`
- `csv_parsed_cache_write_skipped`
- `in_memory_base_reuse`
- `derived_fresh_resample`
- `derived_in_memory_reuse`

Implementation requirements:

- Extend `LoadPriceDataResponse` with `load_source`.
- Keep existing `cache_hit` and `cache_path` fields for compatibility.
- `cache_path` must be a **real** cache path only when a real cache file was used for the load.
- In-memory reuse paths must not return a fake cache path.

## 1.2 Fix the broken base-reuse discriminator

The current base-reuse path is effectively blocked for normal inferred-timeframe datasets because it relies on `LoadedDataContext.timeframe.is_none()` as if “timeframe missing” meant “base dataset.” That is not a reliable discriminator.

Required fix:

- Stop using `ctx.timeframe.is_none()` to detect whether a dataset is a base dataset.
- Instead, determine base-vs-derived using the actual dataset kind from the main dataset map (`DatasetKind::Base` vs `DatasetKind::Derived`).

## 1.3 Strongly recommended low-risk improvement: normalize source identity matching

Implement the addendum’s `source_key` improvement unless the repo already has an equivalent helper.

Required behavior:

- Keep `csv_path` in `LoadedDataContext` for display.
- Add a normalized `source_key` for matching equivalent path spellings.
- Base matching should use:
  - normalized source key
  - source identity (size + mtime)
  - dataset kind = base

Recommended normalized source-key behavior:

- canonicalize path if possible
- keep original path fallback if canonicalization fails
- on Windows, compare case-insensitively by lowercasing the normalized key

If you implement `source_key`, derived datasets should inherit the base dataset’s `source_key` in their `LoadedDataContext`.

## 1.4 Set truthful `load_source` everywhere

Required behavior:

- In `load_price_data(...)`:
  - binary `.vta` hit => `LoadSource::BinaryCache`
  - CSV parsed + cache written => `LoadSource::CsvParsedAndCached`
  - CSV parsed but cache write skipped/failed => `LoadSource::CsvParsedCacheWriteSkipped`
  - in-memory base reuse => `LoadSource::InMemoryBaseReuse`

- In `resample_loaded_data(...)`:
  - reused derived dataset => `LoadSource::DerivedInMemoryReuse`
  - fresh resample => `LoadSource::DerivedFreshResample`

Compatibility requirement:

- Keep `cache_hit = true` on warm reuse paths if existing call sites expect that boolean for “warm path,” but do not let the UI interpret it as “binary cache.”

Correctness requirement:

- Reused derived resample path must return empty `cache_path`, not `ctx.csv_path`.

## 1.5 Preserve existing retention / invalidation semantics

Do not break:

- derived dataset invalidation when the base changes
- retention-limit behavior
- derived reuse by `base_id@timeframe`
- `invalidate_dataset(...)`

## 1.6 Add / update Rust tests in `state.rs`

At minimum add targeted tests covering:

1. **Base reuse works even when timeframe is inferred**
   - load base CSV once
   - load same CSV again
   - assert same base ID is reused
   - assert warm path is `in_memory_base_reuse`
   - assert derived cache remains intact

2. **Derived reuse is truthful**
   - first resample => `derived_fresh_resample`
   - repeated same resample => `derived_in_memory_reuse`
   - reused derived path returns empty `cache_path`

3. **Clear/invalidate behavior still removes descendants**
   - if you add a new targeted helper or change behavior around invalidation, cover it explicitly

4. **If you implement normalized `source_key`**
   - add a reuse test that exercises equivalent path spellings or, if cross-platform path-shape testing is awkward, add a helper-level test or the closest realistic equivalent supported by the repo/test environment

---

# 2. Rust: `src-tauri/src/commands_data.rs`

## 2.1 Add a real backend clear command

Add a Tauri command for clearing loaded data from backend state.

Preferred shape:

- command name: `clear_loaded_data`
- input: optional `base_data_id`
- behavior:
  - empty/missing ID => no-op success response
  - valid base ID => call `AppState::invalidate_dataset(...)`
  - unknown ID => no-op success response instead of hard-failing the UI
  - other errors => real error

Preferred response shape:

- `cleared: bool`
- `removed_ids: Vec<String>`

This command should invalidate the base dataset and all descendants.

## 2.2 Preserve existing load/resample commands

Do not redesign `load_price_data(...)` or `resample_price_data(...)` here beyond carrying through the updated `LoadPriceDataResponse` contract.

## 2.3 Update Tauri command registration

Locate the repo’s actual command-registration site and wire `clear_loaded_data` into the generated handler list.

---

# 3. Service layer: `web/src/services/dataService.ts`

## 3.1 Add a normalized frontend load-source type

Add a `DataLoadSource` union aligned with the Rust enum.

Preferred values:

- `'binary_cache'`
- `'csv_parsed_and_cached'`
- `'csv_parsed_cache_write_skipped'`
- `'in_memory_base_reuse'`
- `'derived_fresh_resample'`
- `'derived_in_memory_reuse'`
- `'unknown'`

Add a normalizer for `load_source` from raw backend payload.

## 3.2 Extend normalized load result shape

Add `loadSource` to `DataLoadResult` and normalize it from the backend response.

Preserve existing normalized fields.

## 3.3 Make `clearData()` real

Update `clearData(...)` so it invokes the new backend command rather than being a no-op.

Recommended behavior:

- accept an optional base-data ID
- call `invokeCommand('clear_loaded_data', { baseDataId: ... })`
- keep the function safe to call when no base ID is present

## 3.4 Remove redundant market-data-dir round trips

The backend `set_market_data_dir` and `clear_market_data_dir` commands already return `MarketDataDirInfo`.

Required change:

- do not immediately call `getMarketDataDirInfo()` after those commands
- normalize the returned payload directly with `normalizeMarketDataDirInfo(...)`

This applies to:

- `setMarketDataDir(...)`
- `resetMarketDataDir(...)`

## 3.5 Optional low-risk improvement if easy

If safe and small, avoid a full `getPolygonCacheInfo()` scan after `polygon_set_api_key` / `polygon_clear_api_key` by updating store state locally instead.

This is optional for this run. Only do it if the change remains tiny and does not introduce ambiguity.

---

# 4. Store: `web/src/stores/dataStore.ts`

This is the largest implementation surface. Keep it disciplined and local.

## 4.1 Add durable loaded-base fields

**Important correction from the addendum:** do not rely on `csvPath` for startup restore.

Add fields dedicated to the **actually loaded base dataset**, distinct from the editable input field.

Required additions:

- `baseCsvPath: string`
- `baseSourceKind: 'csv' | 'polygon'`
- `baseSourceSizeBytes: number | null`
- `baseSourceMtimeUtcMs: number | null`

Set them **only on successful base loads**.

Do not overwrite them merely because the user edits the CSV input field.

## 4.2 Add a minimal startup snapshot

Add a small durable snapshot for startup dataset restore.

It should include at least:

- `autoRestoreDataset`
- `baseCsvPath`
- `selectedTimeframe`
- `source` (`'csv' | 'polygon'`)
- `baseSourceSizeBytes`
- `baseSourceMtimeUtcMs`
- `savedAtUnixMs`

Implementation requirements:

- Add read/write helpers.
- Use localStorage unless the repo already has a clearly preferred storage helper for this exact purpose.
- If the repo centralizes storage keys, follow that convention; otherwise a local constant in `dataStore.ts` is fine.

## 4.3 Persist the snapshot from the loaded base state, not from `csvPath`

This is mandatory.

Snapshot persistence must use `baseCsvPath`, **not** the current editable `csvPath` field.

Persist the snapshot after:

- successful base load
- successful derived resample apply
- returning to base timeframe
- toggling the auto-restore preference
- clearing data
- successful startup restore completion

## 4.4 Add truthful load-source helpers for UI copy

Add small helper functions that convert `loadSource` into user-visible truth.

At minimum add helpers equivalent to:

- `loadSourceText(...)`
- `baseLoadStatusText(...)`
- `derivedLoadStatusText(...)`

The store should stop inferring “binary cache” from `cacheHit` alone.

## 4.5 Update `datasetContextText(...)`

Make `datasetContext` carry truthful context from:

- source kind (`csv` / `polygon`)
- file basename
- load source text
- real cache basename only when a real cache path exists

## 4.6 Update `applyLoadResult(...)`

Required behavior:

- on successful base load:
  - set `baseDataId`
  - set `baseDataMeta`
  - set `baseCsvPath`
  - set `baseSourceKind`
  - set `baseSourceSizeBytes`
  - set `baseSourceMtimeUtcMs`
  - set truthful `baseDatasetContext`
- on successful derived load:
  - keep the base identity fields intact
  - update displayed dataset/timeframe truthfully
- in all successful load cases:
  - use `loadSource` for `statusLine`
  - persist the startup snapshot

## 4.7 Make `applyBaseDatasetState()` persist snapshot truthfully

When the user goes back to the base timeframe, update displayed state and persist the current startup snapshot from base-loaded state.

## 4.8 Sanitize `applyLegacySnapshot(...)` aggressively

Do **not** restore stale runtime dataset or cache state from a UI snapshot.

Required: do **not** trust persisted values for any of the following runtime fields:

- `dataId`
- `baseDataId`
- `baseDataMeta`
- `datasetLabel` if it implies a live loaded dataset
- `datasetRange`
- `dataLen`
- `datasetContext`
- `dataFields`
- `dataFieldHint`
- `statusLine`
- `source`
- `updatedAtUnixMs`
- transient `can*` flags
- `polygonCacheDir`
- `polygonHasApiKey`
- `polygonDatasets`
- `polygonSelectedMeta`
- `polygonCacheMeta`
- other runtime-only summaries

What you may still restore from legacy snapshot state:

- user-entered CSV path text
- recent CSV paths if that is already the existing behavior
- tab/open state if already supported
- Polygon input fields (market, symbol, dates, timeframe, adjusted)
- market-data-dir text field state if appropriate
- auto-restore preference

After sanitation, the store must not claim that a live dataset is loaded until backend restore has actually run.

## 4.9 Make `hydrateStartupState()` real, idempotent, and safe

This must become a real startup hydrate path, not just a metadata fetch.

Required behavior:

- make it async
- dedupe concurrent calls with an in-flight promise guard
- safe if called from both bootstrap and component mount
- fetch market-data-dir info and sample CSVs
- do **not** eagerly fetch Polygon cache info during normal startup
- if startup snapshot says auto-restore and has `baseCsvPath`, then:
  - set a truthful “restoring…” status
  - load the base CSV/path
  - apply the selected timeframe if it differs from the base timeframe
  - surface truthful success/failure status
- leave the app in a truthful empty state if restore fails
- persist the snapshot after the hydrate completes

If you choose to compare snapshot identity to restored source identity using size/mtime and surface an extra status note, that is acceptable but not mandatory for this run.

## 4.10 Make clear actually clear backend state

Convert `clearData()` to an async action if needed.

Required behavior:

- call the real backend clear command when `baseDataId` exists
- clear the full frontend dataset state
- clear workflow artifacts as before
- reset the loaded-base fields:
  - `baseDataId`
  - `baseDataMeta`
  - `baseCsvPath`
  - `baseSourceKind`
  - `baseSourceSizeBytes`
  - `baseSourceMtimeUtcMs`
- persist an empty startup snapshot / no-loaded-dataset state
- keep failure handling truthful (for example, clear UI state but mention backend clear warning if backend clear fails)

## 4.11 Remove eager Polygon cache scan from normal startup

The current startup hydrate path should no longer fetch Polygon cache info unconditionally.

Required behavior:

- lazy-load Polygon cache info only when:
  - the user opens Polygon, or
  - the user explicitly refreshes Polygon cache info, or
  - there is some already-open Polygon state that truly requires it

## 4.12 Add runtime reset for Polygon cache state when market-data root changes

This is mandatory from the addendum.

Add a helper similar to `resetPolygonCacheRuntimeState(...)` that clears:

- cached Polygon directory text
- cached dataset list
- selected Polygon path
- selected summary text
- cache meta text
- any lazy-load flags / in-flight promises associated with Polygon cache loading

Call that helper when market-data root changes or resets **and** the effective Polygon cache directory changed.

## 4.13 Promise-dedupe Polygon cache loading

This is a strong optional addition and recommended if it stays small.

Preferred shape:

- `polygonCacheLoadedOnce: boolean`
- `polygonCacheLoadPromise: Promise<void> | null`
- `ensurePolygonCacheLoaded(state, force = false)` helper

Use it for:

- first open of Polygon
- refresh
- any startup overlap case where the pane is already open

## 4.14 Remove busy-state side effects from `applyPolygonCacheInfo(...)`

This is mandatory from the addendum.

Make `applyPolygonCacheInfo(...)` a pure “apply payload to state” helper.

Do **not** call `setControlsBusy(state, false)` inside it.

Each caller should own its own busy-state lifecycle.

## 4.15 Avoid full cache refresh after successful Polygon download/load

Do not immediately call `getPolygonCacheInfoAction()` after `downloadPolygonDataset()` just to refresh the list.

Instead:

- locally upsert the downloaded dataset summary into `polygonDatasets`
- refresh `polygonCacheMeta`
- select the downloaded path
- then apply the returned base load / derived load results

Keep the existing feature behavior:

- base load is always the 1m cached dataset
- requested higher timeframe is derived in memory from the loaded base

## 4.16 Keep market-data-dir updates efficient

When `setMarketDataDir(...)` or `resetMarketDataDir()` completes:

- use the normalized returned payload directly from `dataService.ts`
- update `marketDataDir` and `marketDataDirHint`
- reset Polygon runtime state if the effective cache dir changed
- do not perform unnecessary extra metadata reads

---

# 5. Component: `web/src/components/DataWorkspacePanel.vue`

Keep component changes small.

Required changes:

1. Include `baseDataMeta` in the `storeToRefs(...)` selection if needed.
2. Compute an effective timeframe for display using:
   - derived timeframe if one is active, otherwise
   - base timeframe from `baseDataMeta`
3. Use that effective timeframe in the summary chips so the panel remains truthful even when the base timeframe is active and `dataTimeframe` is blank.
4. Keep the existing startup-hydration call in `onMounted(...)`; rely on the store-side idempotence rather than redesigning the component.

Do not add a new panel or expand the UI.

---

# 6. Bootstrap: `web/src/services/workflowBootstrap.ts`

Required change:

- Ensure `useDataStore(...).hydrateStartupState()` is called after Pinia is available, even if there is no saved workflow-tab state to restore.
- Do not hide this behind the existing early-return branch for missing workflow bootstrap state.

Goal:

- dataset startup hydration becomes a bootstrap concern, not an accidental side effect of component mount timing
- still safe if component mount also calls it

---

# 7. `web/src/services/settingsBootstrap.ts`

Do **not** move dataset restore logic here.

Optional only:

- add a small clarifying comment if it helps future maintainers understand that dataset startup hydration belongs to the data store + workflow bootstrap path

No functional change is required here.

---

# 8. `src-tauri/src/data_cache.rs`

Mandatory scope for this file is intentionally minimal.

Do **not** redesign cache validation or switch to content-fingerprint validation in the main patch.

Only touch this file if needed for one of the following:

- helper reuse that supports the source-key or identity work
- a narrowly scoped comment / cleanup
- a targeted ignored test placeholder for future stronger cache validation

If you do add a placeholder/test, keep it clearly deferred and do not change current cache semantics in the mandatory patch.

---

## Tests to add or update

### Rust tests

Add/update narrow tests in `src-tauri/src/state.rs` and other adjacent Rust test locations as needed.

Required coverage:

- base reuse with inferred timeframe now works
- reused base path returns `in_memory_base_reuse`
- reused derived resample returns `derived_in_memory_reuse`
- fresh resample returns `derived_fresh_resample`
- reused derived path returns empty `cache_path`
- clearing a base dataset removes descendants / preserves truthfulness
- if `source_key` is implemented, add path-normalization coverage

### Web/store/service tests

If the repo already has TS/Vitest tests for this surface, add targeted tests. If it does not, add the smallest reasonable tests in the existing test style.

Strongly recommended coverage:

1. startup snapshot uses `baseCsvPath`, not an edited `csvPath` draft
2. `applyLegacySnapshot(...)` does not restore stale live dataset/runtime cache state
3. `hydrateStartupState()` restores base dataset + selected timeframe truthfully
4. market-data-root changes reset Polygon runtime cache state
5. if you implement `ensurePolygonCacheLoaded(...)`, duplicate opens/refreshes are deduped
6. load-source status mapping does not mislabel in-memory reuse as binary-cache load

---

## Manual verification checklist to run after coding

Perform these checks manually if possible and include the results in your final summary:

1. **Cold CSV load**
   - load a CSV with no `.vta`
   - verify truthful status text
   - verify `.vta` appears if cache write succeeds

2. **Warm binary-cache load**
   - load same CSV again after `.vta` exists and backend is cold
   - verify status says binary cache hit

3. **Warm in-memory base reuse**
   - load same CSV twice in one app session
   - verify same base ID is reused
   - verify status says in-memory reuse, not binary cache

4. **Fresh resample vs repeated resample**
   - apply a higher timeframe once
   - apply it again
   - verify first is fresh resample and second is in-memory reuse

5. **Clear**
   - clear current dataset
   - verify UI resets
   - verify a subsequent reload is not secretly reusing stale backend state after clear

6. **Startup restore**
   - enable auto-restore
   - load a base dataset, optionally apply derived timeframe
   - reload app / rerun startup bootstrap path
   - verify the dataset is truly restored and status is truthful

7. **Edited input draft does not poison snapshot**
   - load dataset A
   - change `csvPath` input to B without loading B
   - restart
   - verify auto-restore still restores A

8. **Polygon lazy cache**
   - start app with Polygon closed
   - verify no eager Polygon cache scan is required for initial readiness
   - open Polygon and verify cache metadata loads then

9. **Market-data-root change**
   - switch the market-data root
   - verify old Polygon cache list is cleared/reset and the new root’s state is not mixed with the old one

---

## Strong optional improvements to implement if they remain small and safe

These are recommended if they fit naturally into the patch set without broadening scope:

1. **Normalized `source_key` in Rust for base reuse**  
   Implement unless it becomes awkward for platform-specific reasons.

2. **Promise-deduped `ensurePolygonCacheLoaded(...)`**  
   Recommended if you already touch Polygon lazy-load logic.

3. **Local store update on Polygon API-key save/clear instead of rescanning full cache**  
   Only if this stays tiny and does not risk drift.

---

## Explicitly deferred / optional later follow-up items

Do not implement these in the main patch unless they become truly tiny and clearly necessary:

1. Add `source_path` to `LoadPriceDataResponse`
2. Switch base load / resample to the two-stage `spawn_blocking` split
3. Compare startup snapshot identity to restored file identity and surface a changed-file note
4. Handle Polygon snapshot rewrite/clearing when market-data-root moves rewrite the on-disk path of the loaded Polygon dataset
5. Broader cache validation redesign using stronger content fingerprint checks

If you do not implement these, mention them as deferred in your final summary rather than silently ignoring them.

---

## Output requirements for your final response

When you finish, provide:

1. a concise summary of what you changed
2. a file-by-file list of modified files
3. a list of tests added/updated
4. the exact test commands you ran
5. the results of those test commands
6. any manual verification you completed
7. any deferred items you intentionally did not implement
8. any repo-specific ambiguity you resolved while implementing (for example, where you found the Tauri handler registration)

---

## Implementation style notes

- Keep diffs local and readable.
- Preserve existing naming/style conventions where reasonable.
- Prefer adapting the current code shape instead of introducing a new abstraction layer.
- Make startup hydration and lazy Polygon loading idempotent rather than fragile.
- Avoid changing unrelated behavior.
- If you must choose between a slightly less “clever” design and a smaller safer patch, choose the smaller safer patch.

