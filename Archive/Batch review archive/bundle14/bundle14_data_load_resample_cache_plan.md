
# Bundle 14 Implementation Plan — Data Loading, Resampling, and Cache Latency

## Scope and prompt alignment

This document follows **Bundle 14: “data loading, resampling, and cache latency”** from `gpt54_feature_review_map.md` and stays inside that bundle’s hard scope:

- real local CSV load latency
- warm binary-cache hit behavior
- warm in-memory reuse behavior
- resample reuse vs fresh resample cost
- metadata-scan overhead that affects startup or dataset-switch latency
- startup dataset-readiness and truthfulness

It intentionally avoids broad UI redesign, product-surface expansion, or architecture rewrites unless directly needed to fix a confirmed latency/correctness issue.

## Surface under review

This surface covers the end-to-end app path for loading a dataset, reusing it from warm state, deriving higher timeframes, clearing it, and restoring it on startup. The path spans Tauri commands, Rust in-memory dataset state, the binary `.vta` cache, the Vue/Pinia data store, the Data workspace panel, and startup bootstrap helpers.

## Files reviewed

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/dataService.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/settingsBootstrap.ts`

## Extra context / intended behavior

After this plan lands, the product should behave like this:

1. **Cold CSV load**
   - Parse CSV once.
   - Optionally write `.vta`.
   - Return clear metadata saying this was a cold parse path.

2. **Warm binary-cache load**
   - Reuse `.vta`.
   - Return clear metadata saying this was a binary-cache hit.

3. **Warm in-memory identical base reload**
   - Reuse the already loaded base dataset.
   - Keep the same dataset ID.
   - Preserve any derived datasets that depend on that base.
   - Return clear metadata saying this was an in-memory base reuse, not a binary-cache hit.

4. **Fresh resample**
   - Build a derived timeframe from the base dataset in memory.
   - Return clear metadata saying this was a fresh resample.

5. **Repeated same resample**
   - Reuse the already derived dataset in memory.
   - Keep the same derived dataset ID.
   - Return clear metadata saying this was an in-memory derived reuse, not a binary-cache hit.

6. **Clear**
   - Clear both the Vue/UI state and the backend in-memory dataset state.
   - Remove startup-restore eligibility for the cleared dataset.

7. **Startup restore**
   - If enabled, actually reload the last base dataset and selected timeframe.
   - Avoid showing stale dataset IDs or metadata before backend state is ready.
   - Avoid unrelated metadata scans on startup when those scans are not needed.

---

# 1. Surface understanding

## Current execution path

### A. Base CSV load path

`DataWorkspacePanel.vue` triggers `dataStore.loadCsv()` / `browseCsv()` / `loadSampleCsv()`.

Those actions call `dataService.loadCsv()`, which invokes the Tauri `load_price_data` command.

`commands_data.rs::load_price_data()` resolves the app cache directory and calls `AppState::load_price_data()`.

`state.rs::load_price_data()`:

1. computes `SourceIdentity` from the CSV path
2. tries `find_matching_loaded_base(...)`
3. if no match, calls `data_cache::load_or_build_with_identity(...)`
4. inserts the dataset into the in-memory map
5. inserts a `LoadedDataContext`
6. returns `LoadPriceDataResponse`

### B. Derived timeframe path

`DataWorkspacePanel.vue` triggers `dataStore.applyTimeframe()`.

That calls `dataService.applyTimeframe()`, which invokes the Tauri `resample_price_data` command.

`commands_data.rs::resample_price_data()` calls `AppState::resample_loaded_data()`.

`state.rs::resample_loaded_data()`:

1. computes an output ID like `base-id@5m`
2. reuses the existing derived dataset if it is fresh
3. otherwise resamples from the base dataset in memory
4. inserts the derived dataset into the in-memory map
5. inserts a matching `LoadedDataContext`
6. returns `LoadPriceDataResponse`

### C. Startup path

`DataWorkspacePanel.vue` currently calls `dataStore.hydrateStartupState()` inside `onMounted()`.

`dataStore.hydrateStartupState()` currently:

- fetches market-data-dir info
- fetches sample CSVs
- fetches Polygon cache info

It **does not** actually reload the last dataset or reapply the selected timeframe.

`workflowBootstrap.ts` restores workspace/step/tab state.

`settingsBootstrap.ts` restores UI settings.

Neither file currently rehydrates dataset residency.

### D. Clear path

`DataWorkspacePanel.vue` triggers `dataStore.clearData()`.

`dataStore.clearData()` currently resets only Vue-side state and workflow artifacts.

`dataService.clearData()` is currently a no-op, so the backend `AppState` keeps its loaded base/derived datasets until some other path evicts them.

## Timing boundary

Every performance claim in this document belongs primarily to the **warm app-path runtime** boundary, not to a Criterion microbenchmark. I do **not** have timing output from the adjacent benchmark binaries in this file bundle, so all conclusions below are from code-path analysis.

---

# 2. Findings

## Confirmed issue 1

- **Severity:** high
- **Type:** correctness / performance
- **Location:** `src-tauri/src/state.rs` — `load_price_data`, `find_matching_loaded_base`
- **Timing boundary affected:** app-path
- **Why it matters:** base in-memory reuse is effectively blocked for normal inferred-timeframe datasets. `load_price_data()` stores `LoadedDataContext.timeframe = meta.timeframe.clone()`, while `find_matching_loaded_base()` only reuses a dataset when `ctx.timeframe.is_none()`. Standard minute-bar datasets infer a timeframe like `1m`, so they miss the base-reuse fast path. That defeats the warm identical-reload path and can evict derived datasets unnecessarily.
- **Recommended change:** stop using `LoadedDataContext.timeframe.is_none()` to determine whether a dataset is a base dataset. Reuse either:
  1. the dataset kind from the main `candles` map, or
  2. an explicit base/derived discriminator in `LoadedDataContext`.

## Confirmed issue 2

- **Severity:** high
- **Type:** correctness / maintainability
- **Location:** `src-tauri/src/state.rs` — `LoadPriceDataResponse`, `find_matching_loaded_base`, `resample_loaded_data`; `web/src/services/dataService.ts`; `web/src/stores/dataStore.ts`
- **Timing boundary affected:** app-path
- **Why it matters:** the current response contract conflates three very different load paths:
  - binary-cache hit
  - in-memory base reuse
  - in-memory derived reuse

  The UI then labels any `cacheHit=true` result as “Loaded (binary cache)” or “Using X (cached)”, which is misleading.

  There is also a stricter correctness bug: the cached-derived resample path currently returns `cache_path = ctx.csv_path`, which is **not** a cache path at all.
- **Recommended change:** add a first-class `load_source` field to `LoadPriceDataResponse` and use it for UI status text. Keep `cache_path` reserved for real `.vta` paths only.

## Confirmed issue 3

- **Severity:** medium
- **Type:** correctness / performance
- **Location:** `web/src/stores/dataStore.ts` — `clearData`; `web/src/services/dataService.ts` — `clearData`; `src-tauri/src/state.rs` — `invalidate_dataset`; `src-tauri/src/commands_data.rs`
- **Timing boundary affected:** app-path
- **Why it matters:** “Clear” currently clears only Vue-side state. The backend still keeps loaded base and derived datasets in memory. That makes later reload timing non-truthful because users can get hidden warm behavior after pressing Clear.
- **Recommended change:** add a real backend `clear_loaded_data` command and call `AppState::invalidate_dataset(base_data_id)` from the store’s clear path.

## Confirmed issue 4

- **Severity:** high
- **Type:** correctness / startup-readiness
- **Location:** `web/src/stores/dataStore.ts` — `hydrateStartupState`, `applyLegacySnapshot`; `web/src/components/DataWorkspacePanel.vue`; `web/src/services/workflowBootstrap.ts`
- **Timing boundary affected:** app-path
- **Why it matters:** the UI exposes “Restore last dataset and timeframe on startup,” but no real startup dataset restore happens. Worse, `applyLegacySnapshot()` restores dataset IDs and metadata into Vue state even though the Rust backend is not rehydrated, which can present stale “loaded” state.
- **Recommended change:** persist only the minimum startup-restore snapshot needed (base CSV path, selected timeframe, source kind, auto-restore flag), then actually replay that load on startup. Also sanitize `applyLegacySnapshot()` so it does **not** claim a live dataset before backend state is restored.

## Confirmed issue 5

- **Severity:** medium
- **Type:** performance
- **Location:** `web/src/stores/dataStore.ts` — `hydrateStartupState`, `openPolygon`, `downloadPolygonDataset`
- **Timing boundary affected:** app-path
- **Why it matters:** the store eagerly fetches Polygon cache info on startup, fetches it again when the Polygon pane opens, and fetches it again immediately after download/load. That is repeated metadata work on a path that should stay lean.
- **Recommended change:** make Polygon cache info lazy. Do not scan it on startup unless the Polygon section is already open or otherwise explicitly needed. After `downloadPolygonDataset()`, upsert the just-downloaded dataset into local store state instead of forcing a full cache rescan.

## Likely issue 6

- **Severity:** medium
- **Type:** startup-readiness / maintainability
- **Location:** `web/src/components/DataWorkspacePanel.vue` — `onMounted`; `web/src/services/workflowBootstrap.ts`
- **Timing boundary affected:** app-path
- **Why it matters:** startup data hydration currently depends on the Data panel component mounting. If bootstrap code elsewhere runs first or the panel mount timing changes, dataset restore/readiness can drift again.
- **Recommended change:** make `dataStore.hydrateStartupState()` idempotent and call it from `workflowBootstrap.ts` as a startup-level bootstrap action. Keep the component-level call as a harmless fallback.

## Open question 7

- **Severity:** low
- **Type:** correctness / benchmark-fidelity
- **Location:** `src-tauri/src/data_cache.rs` — `identity_for_source`, `try_load_cache`
- **Timing boundary affected:** app-path
- **Why it matters:** binary-cache validity is based only on source `size` and `mtime_utc_ms`. That is fast, but it can be wrong when content changes without changing those two values. I do not have real bug reports or app-path measurements proving this is currently happening.
- **Recommended change:** treat this as **optional hardening**. If you revisit it, prefer a versioned, opt-in strong-validation path instead of making every warm load hash the entire source file by default.

## Higher-risk opportunity 8

- **Severity:** low to medium
- **Type:** performance
- **Location:** `src-tauri/src/commands_data.rs` — `load_price_data`, `resample_price_data`
- **Timing boundary affected:** app-path
- **Why it matters:** cold load, `.vta` read, and fresh resample are synchronous at the Tauri command boundary. For large CSVs that can be visibly blocking.
- **Recommended change:** defer until after the safer fixes in this document. If still needed, convert the data commands to `spawn_blocking` **after** the app-state ownership model is adjusted so that the state can be safely moved into a blocking closure.

---

# 3. Safe optimization opportunities

1. **Fix warm base reuse first.**  
   This is the most important fix because it restores the intended fast path and prevents unnecessary derived-cache loss.

2. **Make the response contract truthful.**  
   Introduce `load_source` and stop reusing `cache_hit` as the only status input.

3. **Make Clear actually clear backend state.**  
   This improves both memory behavior and perf-truthfulness.

4. **Implement real startup restore.**  
   Persist a minimal snapshot and replay it on boot.

5. **Make Polygon metadata lazy.**  
   Remove startup cache scans that are unrelated to the user’s immediate task.

6. **Use the existing `datasetContext` and `statusLine` UI real estate more truthfully.**  
   You do not need a new panel or a larger redesign. Small copy/label changes are enough.

---

# 4. Higher-risk optimization opportunities

1. **Tauri `spawn_blocking` for load/resample**  
   Do this only after measuring app-path improvement need. It likely requires adjacent state-registration work not shown in this bundle.

2. **Strong cache identity / fingerprint validation**  
   Good hardening, but it must be opt-in or carefully bounded so that it does not erase the benefit of warm `.vta` loads.

---

# 5. Benchmark and perf-gate review

I do not have the adjacent benchmark binaries in this review bundle, so I cannot claim measured wall-clock improvements yet.

## What to measure after implementing this plan

Use the existing adjacent app-path/perf-gate posture from the repo review map and measure these exact scenarios:

1. cold CSV load, no `.vta`
2. warm `.vta` load
3. warm identical base reload of an already loaded dataset
4. first `1m -> 5m` resample
5. repeated `1m -> 5m` resample
6. startup with auto-restore **off**
7. startup with auto-restore **on**
8. startup with auto-restore **on** and Polygon pane **closed**
9. first open of Polygon pane after boot
10. repeated open/close of Polygon pane after metadata has been loaded once

## What should improve

- repeated identical base reload should become significantly cheaper because it will finally hit the intended in-memory reuse path
- repeated resample status text should become truthful
- startup with Polygon closed should avoid an unnecessary metadata scan
- startup restore should trade a small amount of real load work for truthful readiness instead of showing stale UI state

---

# 6. Test review

## Existing useful Rust tests already in `state.rs`

The file already contains tests for:

- replacing prior datasets
- invalidating derived datasets on live updates
- reusing derived resamples until the base changes
- reusing identical base loads
- access-sequence updates
- retention env overrides

Those are good foundations, but at least one of them is currently inconsistent with the code path and should become meaningful again after the base-reuse fix lands.

## Existing useful Rust tests already in `data_cache.rs`

The file already contains tests for:

- content fingerprint stability
- content fingerprint change detection
- source identity equality semantics
- default loader vs identity loader behavior

## Missing tests to add

### In `state.rs`

1. **base reuse when base timeframe is inferred**
2. **derived reuse reports `DerivedInMemoryReuse` and empty `cache_path`**
3. **base reuse reports `InMemoryBaseReuse`**
4. **fresh resample reports `DerivedFreshResample`**
5. **binary-cache hit reports `BinaryCache`**
6. **clear/invalidate removes both base and descendants**
7. **startup-facing truthfulness test at the store/service layer** (adjacent frontend test if/when test files are added)

### In `data_cache.rs`

1. optional ignored test for strong cache-identity validation if you later add that path
2. any test tied to a future header/version bump if you implement fingerprint hardening

---

# 7. Suggested improvement plan

## Phase 1 — Fix correctness and truthfulness in the Rust response contract

### Files
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

### Deliverables
- base in-memory reuse actually works
- load responses distinguish binary-cache vs in-memory reuse vs fresh resample
- Clear invalidates backend state

## Phase 2 — Make startup restore real and idempotent

### Files
- `web/src/stores/dataStore.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/components/DataWorkspacePanel.vue`

### Deliverables
- stored auto-restore preference
- stored base dataset path + selected timeframe
- real startup restore
- no stale “loaded” UI before backend is ready
- bootstrap call no longer depends purely on component mount order

## Phase 3 — Remove avoidable metadata scans

### Files
- `web/src/stores/dataStore.ts`

### Deliverables
- no eager Polygon cache fetch during startup when the pane is closed
- no repeated cache scan on every Polygon pane open
- no forced full cache rescan immediately after download/load

## Phase 4 — Tighten tests

### Files
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`

### Deliverables
- regression coverage for the newly fixed fast paths
- explicit tests for truthful `load_source` / `cache_path` behavior

## Phase 5 — Optional higher-risk follow-up

### Files
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`

### Deliverables
- optional `spawn_blocking` conversion
- optional strong cache identity validation

---

# File-by-file implementation plan and code edits

## A. `src-tauri/src/state.rs`

## Goals

1. fix base reuse
2. add truthful load-source metadata
3. stop returning fake cache paths
4. add regression tests

## Recommended code edits

### 1. Add a first-class load source enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadSource {
    BinaryCache,
    CsvParsedAndCached,
    CsvParsedCacheWriteSkipped,
    InMemoryBaseReuse,
    DerivedFreshResample,
    DerivedInMemoryReuse,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadPriceDataResponse {
    pub id: String,
    pub meta: PriceDataMeta,
    pub cache_hit: bool,
    pub cache_path: String,
    pub load_source: LoadSource,
}
```

### 2. Fix `find_matching_loaded_base(...)`

Do **not** use `ctx.timeframe.is_none()` as the base/derived discriminator. Use the actual `DatasetKind` from the main dataset map.

```rust
fn find_matching_loaded_base(
    &self,
    path: &str,
    source_identity: data_cache::SourceIdentity,
) -> Result<Option<LoadPriceDataResponse>, String> {
    let access_seq = self.next_access_seq();

    let candidate_ids: Vec<String> = {
        let ctx_map = self
            .loaded_data_context
            .lock()
            .map_err(|_| "failed to lock loaded-data context map".to_string())?;

        ctx_map
            .iter()
            .filter_map(|(id, ctx)| {
                (ctx.csv_path == path && ctx.source_identity == Some(source_identity))
                    .then(|| id.clone())
            })
            .collect()
    };

    if candidate_ids.is_empty() {
        return Ok(None);
    }

    let mut map = self
        .candles
        .lock()
        .map_err(|_| "failed to lock candles map".to_string())?;

    let mut best_id: Option<String> = None;
    let mut best_access_seq = 0u64;

    for id in candidate_ids {
        let Some(entry) = map.get(&id) else {
            continue;
        };
        if !matches!(entry.version_meta.kind, DatasetKind::Base) {
            continue;
        }
        if best_id.is_none() || entry.last_access_seq >= best_access_seq {
            best_access_seq = entry.last_access_seq;
            best_id = Some(id);
        }
    }

    let Some(id) = best_id else {
        return Ok(None);
    };

    let Some(entry) = map.get_mut(&id) else {
        return Ok(None);
    };

    entry.last_access_seq = access_seq;
    let meta = build_price_data_meta(
        entry.candles.as_ref(),
        infer_timeframe_label(&entry.candles.timestamp),
        Some(source_identity.size),
        Some(source_identity.mtime_utc_ms),
    );

    Ok(Some(LoadPriceDataResponse {
        id,
        meta,
        cache_hit: true,
        cache_path: String::new(),
        load_source: LoadSource::InMemoryBaseReuse,
    }))
}
```

### 3. Set `load_source` in `load_price_data(...)`

```rust
pub fn load_price_data(
    &self,
    path: &str,
    app_cache_dir: &Path,
) -> Result<LoadPriceDataResponse, String> {
    let pathbuf = Path::new(path);
    let source_identity = data_cache::identity_for_source(pathbuf)?;

    if let Some(existing) = self.find_matching_loaded_base(path, source_identity)? {
        return Ok(existing);
    }

    let outcome =
        data_cache::load_or_build_with_identity(pathbuf, app_cache_dir, source_identity)?;
    let candles = outcome.candles;
    let meta = build_price_data_meta(
        &candles,
        infer_timeframe_label(&candles.timestamp),
        Some(outcome.source_identity.size),
        Some(outcome.source_identity.mtime_utc_ms),
    );
    let id = self.next_dataset_id();
    let access_seq = self.next_access_seq();

    let evicted_ids = {
        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        map.insert(
            id.clone(),
            DatasetEntry {
                candles: Arc::new(candles),
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Base,
                    source_version: None,
                },
                last_access_seq: access_seq,
            },
        );
        let protect_ids = HashSet::from([id.clone()]);
        enforce_dataset_retention_locked(&mut map, &protect_ids)
    };
    self.remove_loaded_context_entries(&evicted_ids)?;

    let mut ctx_map = self
        .loaded_data_context
        .lock()
        .map_err(|_| "failed to lock loaded-data context map".to_string())?;
    ctx_map.insert(
        id.clone(),
        LoadedDataContext {
            csv_path: path.to_string(),
            label: dataset_label_from_path(path),
            timeframe: meta.timeframe.clone(),
            source_identity: Some(outcome.source_identity),
        },
    );

    let load_source = if outcome.cache_hit {
        LoadSource::BinaryCache
    } else if outcome.cache_path.is_some() {
        LoadSource::CsvParsedAndCached
    } else {
        LoadSource::CsvParsedCacheWriteSkipped
    };

    Ok(LoadPriceDataResponse {
        id,
        meta,
        cache_hit: outcome.cache_hit,
        cache_path: outcome
            .cache_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        load_source,
    })
}
```

### 4. Set `load_source` in `resample_loaded_data(...)`

#### Cached derived reuse path

Note the important correction: `cache_path` should be empty here.

```rust
if let Some(meta) = cached_meta {
    return Ok(LoadPriceDataResponse {
        id: out_id,
        meta,
        cache_hit: true,
        cache_path: String::new(),
        load_source: LoadSource::DerivedInMemoryReuse,
    });
}
```

#### Fresh derived build path

```rust
Ok(LoadPriceDataResponse {
    id: out_id,
    meta,
    cache_hit: false,
    cache_path: String::new(),
    load_source: LoadSource::DerivedFreshResample,
})
```

### 5. Add regression tests

#### Base reuse with inferred timeframe

```rust
#[test]
fn identical_base_load_reuses_existing_id_even_when_timeframe_is_inferred() {
    let temp = make_temp_dir("state_base_reuse_inferred_tf");
    let cache_dir = temp.join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");

    let csv = temp.join("base.csv");
    write_candles_csv(&csv, &make_trending_candles(36, 0.2));

    let state = AppState::default();
    let first = state
        .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
        .expect("first load");
    assert_eq!(first.meta.timeframe.as_deref(), Some("1m"));

    let second = state
        .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
        .expect("second load");

    assert_eq!(first.id, second.id);
    assert_eq!(second.load_source, LoadSource::InMemoryBaseReuse);
    assert!(second.cache_hit);
    assert!(second.cache_path.is_empty());
}
```

#### Derived reuse returns truthful load source and no fake cache path

```rust
#[test]
fn repeated_resample_reports_in_memory_reuse_and_no_cache_path() {
    let temp = make_temp_dir("state_resample_load_source");
    let cache_dir = temp.join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");

    let csv = temp.join("base.csv");
    write_candles_csv(&csv, &make_trending_candles(40, 0.25));

    let state = AppState::default();
    let load = state
        .load_price_data(csv.to_str().unwrap_or_default(), &cache_dir)
        .expect("load base");

    let fresh = state
        .resample_loaded_data(&load.id, "5m")
        .expect("fresh resample");
    assert_eq!(fresh.load_source, LoadSource::DerivedFreshResample);
    assert!(fresh.cache_path.is_empty());

    let reused = state
        .resample_loaded_data(&load.id, "5m")
        .expect("reused resample");
    assert_eq!(reused.load_source, LoadSource::DerivedInMemoryReuse);
    assert!(reused.cache_path.is_empty());
}
```

## Why this file is priority #1

This file contains the warm-path bug and the main truthfulness contract. Fixing it first unlocks every other improvement.

---

## B. `src-tauri/src/commands_data.rs`

## Goals

1. expose a real backend clear command
2. preserve the new `load_source` contract through existing commands
3. leave higher-risk async/offload work deferred

## Recommended code edits

### 1. Add a clear command

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ClearLoadedDataResponse {
    pub cleared: bool,
    #[serde(default)]
    pub removed_ids: Vec<String>,
}

#[tauri::command]
pub fn clear_loaded_data(
    base_data_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<ClearLoadedDataResponse, String> {
    let id = base_data_id.unwrap_or_default();
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Ok(ClearLoadedDataResponse {
            cleared: false,
            removed_ids: Vec::new(),
        });
    }

    match state.invalidate_dataset(trimmed) {
        Ok(removed_ids) => Ok(ClearLoadedDataResponse {
            cleared: !removed_ids.is_empty(),
            removed_ids,
        }),
        Err(err) if err.starts_with("unknown data id:") => Ok(ClearLoadedDataResponse {
            cleared: false,
            removed_ids: Vec::new(),
        }),
        Err(err) => Err(err),
    }
}
```

### 2. Keep `load_price_data(...)` and `resample_price_data(...)` synchronous for now

I do **not** recommend changing these command signatures in the same patch series as the correctness fixes above. Get the semantics right first, then measure.

## Optional deferred note

If you later convert load/resample to `spawn_blocking`, do it as a separate follow-up after app-path measurements show it is still necessary.

---

## C. `src-tauri/src/data_cache.rs`

## Goals

1. keep the current warm `.vta` path fast
2. do not destabilize cache behavior while fixing the higher-value bugs above
3. optionally prepare a deferred strong-validation path

## Immediate recommendation

**No required phase-1 code edit.**  
The most important latency/correctness problems in this bundle are elsewhere.

## Optional hardening follow-up (deferred)

The header currently has spare bytes, so if you later want stronger cache validation, you can add a versioned source-content hash without redesigning the file format.

### Example design

- bump `VERSION`
- use bytes `12..16` for header flags
- use bytes `56..64` for a sampled/full FNV64 content hash
- compute/compare the hash only when a guard says it is worth doing

### Sketch

```rust
const VERSION: u32 = 2;
const HEADER_FLAG_SOURCE_HASH_PRESENT: u32 = 1;

fn write_header(
    out: &mut [u8; HEADER_LEN],
    header_flags: u32,
    fields_mask: u32,
    len: u64,
    csv_size: u64,
    csv_mtime_ms: i64,
    min_ts: i64,
    max_ts: i64,
    source_hash: u64,
) {
    out.fill(0);
    out[0..4].copy_from_slice(MAGIC);
    out[4..8].copy_from_slice(&VERSION.to_le_bytes());
    out[8..12].copy_from_slice(&fields_mask.to_le_bytes());
    out[12..16].copy_from_slice(&header_flags.to_le_bytes());
    out[16..24].copy_from_slice(&len.to_le_bytes());
    out[24..32].copy_from_slice(&csv_size.to_le_bytes());
    out[32..40].copy_from_slice(&csv_mtime_ms.to_le_bytes());
    out[40..48].copy_from_slice(&min_ts.to_le_bytes());
    out[48..56].copy_from_slice(&max_ts.to_le_bytes());
    out[56..64].copy_from_slice(&source_hash.to_le_bytes());
}
```

### Recommended guardrail

Do **not** enable strong source hashing for all warm loads by default unless app-path measurement proves the cost is negligible for your real datasets.

## Optional test placeholder

If you later add strong validation, add an ignored regression test now or a normal test when the feature lands:

```rust
#[test]
#[ignore = "enable once strong cache validation lands"]
fn cache_miss_when_source_content_changes_without_size_or_mtime_change() {
    // prepare source A
    // build cache
    // rewrite source content while preserving size + mtime
    // expect cache miss or rebuild
}
```

---

## D. `web/src/services/dataService.ts`

## Goals

1. normalize the new `load_source` field
2. stop making `clearData()` a no-op
3. keep the service layer thin and explicit

## Recommended code edits

### 1. Add `DataLoadSource`

```ts
export type DataLoadSource =
  | 'binary_cache'
  | 'csv_parsed_and_cached'
  | 'csv_parsed_cache_write_skipped'
  | 'in_memory_base_reuse'
  | 'derived_fresh_resample'
  | 'derived_in_memory_reuse'
  | 'unknown'
```

### 2. Extend the response interfaces

```ts
interface LoadPriceDataResponse {
  id?: unknown
  meta?: unknown
  cache_hit?: unknown
  cache_path?: unknown
  load_source?: unknown
}

interface PolygonDownloadAndLoadResponse {
  csv_path?: unknown
  downloaded_bars?: unknown
  used_cache_only?: unknown
  base_load?: unknown
  id?: unknown
  meta?: unknown
  cache_hit?: unknown
  cache_path?: unknown
  load_source?: unknown
}
```

### 3. Normalize `load_source`

```ts
function normalizeLoadSource(value: unknown): DataLoadSource {
  const next = normalizeText(value)
  switch (next) {
    case 'binary_cache':
    case 'csv_parsed_and_cached':
    case 'csv_parsed_cache_write_skipped':
    case 'in_memory_base_reuse':
    case 'derived_fresh_resample':
    case 'derived_in_memory_reuse':
      return next
    default:
      return 'unknown'
  }
}
```

### 4. Add `loadSource` to `DataLoadResult`

```ts
export interface DataLoadResult {
  path: string
  dataId: string
  meta: DataMetaSummary
  cacheHit: boolean
  cachePath: string
  loadSource: DataLoadSource
}

function normalizeLoadPriceDataResponse(raw: unknown, path: string): DataLoadResult {
  const row = (raw && typeof raw === 'object' ? raw : {}) as LoadPriceDataResponse
  return {
    path: normalizeText(path),
    dataId: normalizeText(row.id),
    meta: normalizeDataMeta(row.meta),
    cacheHit: Boolean(row.cache_hit),
    cachePath: normalizeText(row.cache_path),
    loadSource: normalizeLoadSource(row.load_source),
  }
}
```

### 5. Make `clearData()` real

```ts
export async function clearData(baseDataId?: string): Promise<void> {
  await invokeCommand('clear_loaded_data', {
    baseDataId: normalizeText(baseDataId) || null,
  })
}
```

## Notes

- Keep `loadCsv(...)` and `applyTimeframe(...)` signatures stable.
- The store should become the main consumer of `loadSource`.
- This file should not contain UI wording beyond normalization.

---

## E. `web/src/stores/dataStore.ts`

## Goals

1. make load status truthful
2. persist and replay startup restore
3. clear backend state for real
4. remove avoidable Polygon metadata scans
5. sanitize stale restored UI snapshot state

## Recommended code edits

### 1. Add a minimal startup snapshot

Use local storage inside this file so the behavior is self-contained and does not depend on extra files.

```ts
const DATA_STARTUP_SNAPSHOT_KEY = 'vectorbt.data.startup.v1'

interface DataStartupSnapshot {
  autoRestoreDataset: boolean
  baseCsvPath: string
  selectedTimeframe: string
  source: 'csv' | 'polygon'
  savedAtUnixMs: number
}

let startupHydratePromise: Promise<void> | null = null
let polygonCacheLoadedOnce = false

function readStartupSnapshot(): DataStartupSnapshot | null {
  try {
    const raw = String(localStorage.getItem(DATA_STARTUP_SNAPSHOT_KEY) || '').trim()
    if (!raw) return null
    const parsed = JSON.parse(raw) as Partial<DataStartupSnapshot>
    const source = String(parsed.source || '').trim() === 'polygon' ? 'polygon' : 'csv'
    return {
      autoRestoreDataset: Boolean(parsed.autoRestoreDataset),
      baseCsvPath: String(parsed.baseCsvPath || '').trim(),
      selectedTimeframe: String(parsed.selectedTimeframe || '').trim(),
      source,
      savedAtUnixMs: Number.isFinite(Number(parsed.savedAtUnixMs))
        ? Math.floor(Number(parsed.savedAtUnixMs))
        : Date.now(),
    }
  } catch {
    return null
  }
}

function writeStartupSnapshot(snapshot: DataStartupSnapshot): void {
  try {
    localStorage.setItem(DATA_STARTUP_SNAPSHOT_KEY, JSON.stringify(snapshot))
  } catch {
    // best effort only
  }
}

function effectiveSelectedTimeframe(
  state: Pick<DataState, 'dataTimeframe' | 'baseDataMeta'>,
): string {
  const derived = String(state.dataTimeframe || '').trim()
  if (derived) return derived
  return String(state.baseDataMeta?.timeframe || '').trim()
}

function persistStartupSnapshot(state: DataState): void {
  writeStartupSnapshot({
    autoRestoreDataset: !!state.autoRestoreDataset,
    baseCsvPath: hasLoadedDataset(state.baseDataId)
      ? String(state.csvPath || '').trim()
      : '',
    selectedTimeframe: hasLoadedDataset(state.baseDataId)
      ? effectiveSelectedTimeframe(state)
      : '',
    source: state.baseSourceKind,
    savedAtUnixMs: Date.now(),
  })
}
```

### 2. Extend `DataState`

Add one new field to remember whether the loaded base came from CSV or Polygon.

```ts
interface DataState {
  // existing fields...
  baseSourceKind: 'csv' | 'polygon'
}
```

Default:

```ts
baseSourceKind: 'csv',
```

### 3. Add load-source helpers

```ts
function loadSourceText(source: DataLoadResult['loadSource']): string {
  switch (source) {
    case 'binary_cache':
      return 'binary-cache hit'
    case 'csv_parsed_and_cached':
      return 'CSV parsed + cache written'
    case 'csv_parsed_cache_write_skipped':
      return 'CSV parsed (cache write skipped)'
    case 'in_memory_base_reuse':
      return 'in-memory base reuse'
    case 'derived_fresh_resample':
      return 'fresh in-memory resample'
    case 'derived_in_memory_reuse':
      return 'in-memory derived reuse'
    default:
      return 'unknown path'
  }
}

function datasetContextText(
  path: string,
  source: string,
  loadSource?: DataLoadResult['loadSource'],
  cachePath = '',
): string {
  const parts: string[] = []
  const src = String(source || '').trim()
  if (src) parts.push(`source=${src}`)

  const name = fileBasename(path)
  if (name) parts.push(`file=${name}`)

  const load = loadSourceText(loadSource || 'unknown')
  if (load) parts.push(`load=${load}`)

  const cacheName = fileBasename(cachePath)
  if (cacheName) parts.push(`cache=${cacheName}`)

  return parts.length ? parts.join(' • ') : '—'
}

function baseLoadStatusText(result: DataLoadResult, subject = 'Loaded'): string {
  switch (result.loadSource) {
    case 'binary_cache':
      return `${subject} (binary cache hit).`
    case 'csv_parsed_and_cached':
      return `${subject} (CSV parsed + cache written).`
    case 'csv_parsed_cache_write_skipped':
      return `${subject} (CSV parsed; cache write skipped).`
    case 'in_memory_base_reuse':
      return `${subject} (in-memory reuse).`
    default:
      return `${subject}.`
  }
}

function derivedLoadStatusText(result: DataLoadResult): string {
  const tf = String(result.meta?.timeframe || '').trim() || 'derived timeframe'
  switch (result.loadSource) {
    case 'derived_in_memory_reuse':
      return `Using ${tf} (in-memory reuse).`
    case 'derived_fresh_resample':
      return `Using ${tf} (fresh resample).`
    default:
      return `Using ${tf}.`
  }
}
```

### 4. Make `hydrateStartupState()` idempotent and real

Replace the current fire-and-forget metadata-only implementation with an async, idempotent startup bootstrap.

```ts
async hydrateStartupState(force = false): Promise<void> {
  if (startupHydratePromise && !force) {
    await startupHydratePromise
    return
  }

  startupHydratePromise = (async () => {
    const startup = readStartupSnapshot()
    if (startup) {
      this.autoRestoreDataset = !!startup.autoRestoreDataset
      markVueUpdate(this)
    }

    const [dirInfo, samples] = await Promise.allSettled([
      getMarketDataDirInfoAction(),
      listSampleCsvsAction(),
    ])

    if (dirInfo.status === 'fulfilled') {
      const configured = String(dirInfo.value?.configuredRootDir || '').trim()
      const hint = String(dirInfo.value?.hint || this.marketDataDirHint || '—').trim() || '—'
      this.marketDataDir = configured
      this.marketDataDirHint = hint
    }

    if (samples.status === 'fulfilled') {
      this.sampleCsvOptions = Array.isArray(samples.value) ? samples.value : []
      this.sampleCsvVisible = this.sampleCsvOptions.length > 0
    }

    if (startup?.autoRestoreDataset && startup.baseCsvPath) {
      setControlsBusy(this, true)
      this.statusLine = 'Restoring last dataset...'
      markVueUpdate(this)

      try {
        const base = await loadCsvAction(startup.baseCsvPath)
        this.applyLoadResult(base, {
          treatAsBase: true,
          statusLine: `Restored last dataset (${loadSourceText(base.loadSource)}).`,
          source: startup.source,
        })

        const baseTf = String(base.meta?.timeframe || '').trim()
        const selectedTf = String(startup.selectedTimeframe || '').trim()

        if (selectedTf && selectedTf !== baseTf) {
          const derived = await applyTimeframeAction(selectedTf, base.dataId)
          this.applyLoadResult(derived, {
            treatAsBase: false,
            statusLine: `Restored last dataset (${selectedTf}; ${loadSourceText(derived.loadSource)}).`,
            source: startup.source,
          })
        }
      } catch (error: unknown) {
        this.statusLine = `Startup restore failed: ${String(error || '').trim() || 'unknown error'}`
      } finally {
        setControlsBusy(this, false)
        markVueUpdate(this)
      }
    }

    if (this.polygonOpen && !polygonCacheLoadedOnce) {
      try {
        const info = await getPolygonCacheInfoAction()
        applyPolygonCacheInfo(this, info, this.polygonSelectedPath)
      } catch (error: unknown) {
        this.statusLine = String(error || '').trim() || 'Failed to refresh Polygon cache.'
      }
    }

    persistStartupSnapshot(this)
    markVueUpdate(this)
  })()

  await startupHydratePromise
}
```

### 5. Sanitize `applyLegacySnapshot(...)`

Do **not** restore live dataset IDs/metadata from a snapshot unless the backend has actually restored them.

```ts
applyLegacySnapshot(snapshot: Partial<DataState>): void {
  this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
  this.csvPath = String(snapshot.csvPath ?? this.csvPath ?? '').trim()
  this.recentCsvPaths = normalizedList(snapshot.recentCsvPaths ?? this.recentCsvPaths ?? [])
  this.sampleCsvOptions = normalizedSamples(snapshot.sampleCsvOptions ?? this.sampleCsvOptions ?? [])
  this.sampleCsvVisible = Boolean(snapshot.sampleCsvVisible ?? this.sampleCsvVisible)
  this.marketDataDir = String(snapshot.marketDataDir ?? this.marketDataDir ?? '').trim()
  this.marketDataMoveExisting = Boolean(snapshot.marketDataMoveExisting ?? this.marketDataMoveExisting)
  this.marketDataDirHint = String(snapshot.marketDataDirHint ?? this.marketDataDirHint ?? '—').trim() || '—'

  // restore user-facing inputs/preferences only
  this.dataTimeframe = String(snapshot.dataTimeframe ?? this.dataTimeframe ?? '').trim()
  this.dataTimeframeHint = 'Load a dataset to enable timeframe resampling.'
  this.autoRestoreDataset = Boolean(snapshot.autoRestoreDataset ?? this.autoRestoreDataset)
  this.datasetContext = '—'
  this.dataFields = []
  this.dataFieldHint = ''
  this.polygonOpen = Boolean(snapshot.polygonOpen ?? this.polygonOpen)
  this.polygonApiKey = String(snapshot.polygonApiKey ?? this.polygonApiKey ?? '').trim()
  this.polygonMarket = String(snapshot.polygonMarket ?? this.polygonMarket ?? 'stocks').trim() || 'stocks'
  this.polygonSymbol = String(snapshot.polygonSymbol ?? this.polygonSymbol ?? '').trim().toUpperCase()
  this.polygonFrom = normalizeDatasetDateInput(String(snapshot.polygonFrom ?? this.polygonFrom ?? '').trim())
  this.polygonTo = normalizeDatasetDateInput(String(snapshot.polygonTo ?? this.polygonTo ?? '').trim())
  this.polygonTimeframe = String(snapshot.polygonTimeframe ?? this.polygonTimeframe ?? '1m').trim() || '1m'
  this.polygonAdjusted = Boolean(snapshot.polygonAdjusted ?? this.polygonAdjusted)
  this.polygonCacheDir = String(snapshot.polygonCacheDir ?? this.polygonCacheDir ?? '').trim()
  this.polygonHasApiKey = Boolean(snapshot.polygonHasApiKey ?? this.polygonHasApiKey)
  this.polygonDatasets = Array.isArray(snapshot.polygonDatasets) ? snapshot.polygonDatasets as PolygonDatasetSummary[] : this.polygonDatasets
  this.polygonSelectedPath = String(snapshot.polygonSelectedPath ?? this.polygonSelectedPath ?? '').trim()
  this.polygonSelectedMeta = String(snapshot.polygonSelectedMeta ?? this.polygonSelectedMeta ?? 'Select a cached Polygon dataset to inspect, validate, repair, or load.').trim() || 'Select a cached Polygon dataset to inspect, validate, repair, or load.'
  this.polygonCacheMeta = String(snapshot.polygonCacheMeta ?? this.polygonCacheMeta ?? 'Polygon cache unavailable.').trim() || 'Polygon cache unavailable.'

  // never trust persisted live dataset residency
  this.dataId = '—'
  this.datasetLabel = '—'
  this.datasetRange = '—'
  this.dataLen = '—'
  this.baseDataId = ''
  this.baseDataMeta = null
  this.baseDatasetLabel = ''
  this.baseDatasetContext = ''
  this.baseSourceKind = 'csv'
  this.canApplyTimeframe = false

  this.statusLine = this.autoRestoreDataset && this.csvPath
    ? 'Startup restore pending.'
    : 'Ready.'

  selectPolygonDatasetPath(this, this.polygonSelectedPath)
  setControlsBusy(this, false)
  this.source = 'vue'
  this.updatedAtUnixMs = Date.now()
}
```

### 6. Update `applyLoadResult(...)`

Use the new `loadSource` information and persist the startup snapshot after every real load/resample.

```ts
applyLoadResult(
  result: DataLoadResult,
  options?: { treatAsBase?: boolean; statusLine?: string; source?: 'csv' | 'polygon' },
): void {
  const row = result && typeof result === 'object' ? result : null
  if (!row) return

  const dataId = String(row.dataId || '').trim()
  const meta = row.meta && typeof row.meta === 'object' ? row.meta : null
  const path = String(row.path || '').trim()
  const treatAsBase = options?.treatAsBase !== false
  const sourceKind = options?.source || 'csv'

  if (path) {
    this.csvPath = path
    this.recentCsvPaths = pushRecentCsvPath(this.recentCsvPaths, path)
  }

  if (treatAsBase) {
    this.baseDataId = dataId
    this.baseDataMeta = meta
    this.baseDatasetLabel = fileBasename(path) || this.baseDatasetLabel || this.datasetLabel || '—'
    this.baseDatasetContext = datasetContextText(path, sourceKind, row.loadSource, row.cachePath)
    this.baseSourceKind = sourceKind
  }

  const baseMeta = this.baseDataMeta && typeof this.baseDataMeta === 'object' ? this.baseDataMeta : meta
  const baseTf = String(baseMeta?.timeframe || '').trim()
  const effectiveTf = String(meta?.timeframe || '').trim() || baseTf
  const derived = !!baseTf && !!effectiveTf && baseTf !== effectiveTf
  const contextPath = String(path || this.csvPath || '').trim()

  this.dataId = dataId || this.dataId || '—'
  this.dataLen = meta ? String(meta.len) : (this.dataLen || '—')
  this.datasetLabel = treatAsBase
    ? (fileBasename(path) || this.datasetLabel || '—')
    : (this.baseDatasetLabel || this.datasetLabel || '—')
  this.datasetContext = datasetContextText(
    contextPath,
    treatAsBase ? sourceKind : this.baseSourceKind,
    row.loadSource,
    row.cachePath,
  )
  this.datasetRange = datasetRangeText(meta, effectiveTf)
  this.dataFields = dataFieldList(meta)
  this.dataFieldHint = dataFieldHint(meta)
  this.dataTimeframe = derived ? effectiveTf : ''
  this.dataTimeframeHint = timeframeHintText(baseTf, effectiveTf, hasLoadedDataset(this.baseDataId || this.dataId))
  this.canApplyTimeframe = hasLoadedDataset(this.baseDataId || this.dataId)
  this.statusLine = String(options?.statusLine || this.statusLine || 'Ready.').trim() || 'Ready.'

  persistStartupSnapshot(this)
  markVueUpdate(this)
}
```

### 7. Persist startup snapshot when returning to base timeframe

```ts
applyBaseDatasetState(): void {
  const baseId = String(this.baseDataId || '').trim()
  const baseMeta = this.baseDataMeta && typeof this.baseDataMeta === 'object' ? this.baseDataMeta : null
  if (!hasLoadedDataset(baseId) || !baseMeta) return

  const baseTf = String(baseMeta.timeframe || '').trim()

  this.dataId = baseId
  this.dataLen = String(baseMeta.len)
  this.datasetLabel = this.baseDatasetLabel || this.datasetLabel || '—'
  this.datasetContext = this.baseDatasetContext || this.datasetContext || '—'
  this.datasetRange = datasetRangeText(baseMeta, baseTf)
  this.dataFields = dataFieldList(baseMeta)
  this.dataFieldHint = dataFieldHint(baseMeta)
  this.dataTimeframe = ''
  this.dataTimeframeHint = timeframeHintText(baseTf, baseTf, true)
  this.canApplyTimeframe = true
  this.statusLine = baseTf ? `Using base timeframe (${baseTf}).` : 'Using base timeframe.'

  persistStartupSnapshot(this)
  markVueUpdate(this)
}
```

### 8. Persist startup snapshot when the restore checkbox changes

```ts
setCheckboxField(field: string, checked: boolean): void {
  const key = String(field || '').trim()
  const next = !!checked
  if (applyVueCheckboxFieldUpdate(this, key, next)) {
    if (key === 'autoRestoreDataset') {
      persistStartupSnapshot(this)
    }
    setControlsBusy(this, false)
    markVueUpdate(this)
  }
}
```

### 9. Make `clearData()` clear the backend too

Convert the action to async.

```ts
async clearData(): Promise<void> {
  const baseId = String(this.baseDataId || '').trim()

  setControlsBusy(this, true)
  this.statusLine = 'Clearing dataset...'
  markVueUpdate(this)

  let warning = ''
  if (baseId) {
    try {
      await clearDataAction(baseId)
    } catch (error: unknown) {
      warning = String(error || '').trim()
    }
  }

  this.dataId = '—'
  this.datasetLabel = '—'
  this.datasetRange = '—'
  this.dataLen = '—'
  this.dataTimeframe = ''
  this.dataTimeframeHint = 'Load a dataset to enable timeframe resampling.'
  this.datasetContext = '—'
  this.dataFields = []
  this.dataFieldHint = ''
  this.baseDataId = ''
  this.baseDataMeta = null
  this.baseDatasetLabel = ''
  this.baseDatasetContext = ''
  this.baseSourceKind = 'csv'
  this.canApplyTimeframe = false

  const pinia = (this as unknown as { $pinia?: Pinia }).$pinia
  if (pinia) useWorkflowArtifactsStore(pinia).clearArtifacts()

  persistStartupSnapshot(this)
  setControlsBusy(this, false)
  this.statusLine = warning
    ? `Cleared UI state. Backend clear warning: ${warning}`
    : 'Ready.'
  markVueUpdate(this)
}
```

### 10. Use truthful status text in load actions

#### CSV load / browse / sample load

```ts
const status = baseLoadStatusText(result)
this.applyLoadResult(result, { treatAsBase: true, statusLine: status })
```

#### Timeframe apply

```ts
const status = derivedLoadStatusText(result)
this.applyLoadResult(result, { treatAsBase: false, statusLine: status })
```

### 11. Make Polygon cache hydration lazy

#### Mark cache info as loaded when applied

```ts
function applyPolygonCacheInfo(state: DataState, info: PolygonCacheInfoSummary, preferredPath = ''): void {
  polygonCacheLoadedOnce = true
  state.polygonCacheDir = String(info.cacheDir || '').trim()
  state.polygonHasApiKey = !!info.hasApiKey
  state.polygonDatasets = Array.isArray(info.datasets) ? [...info.datasets] : []
  state.polygonCacheMeta = polygonCacheMetaText(info)
  selectPolygonDatasetPath(state, preferredPath)
  setControlsBusy(state, false)
}
```

#### Stop fetching Polygon cache info during normal startup

Remove the current unconditional `getPolygonCacheInfoAction()` call from `hydrateStartupState()`.

#### Fetch Polygon cache info only on first open (or explicit refresh)

```ts
openPolygon(): void {
  const pinia = resolveRuntimePinia()
  if (pinia) {
    useWorkflowStore(pinia).selectWorkspace('workflow', 'data')
  }

  this.polygonOpen = true
  markVueUpdate(this)
  void openPolygonAction().catch(() => undefined)

  if (polygonCacheLoadedOnce) {
    return
  }

  void getPolygonCacheInfoAction()
    .then((info) => {
      applyPolygonCacheInfo(this, info, this.polygonSelectedPath)
      markVueUpdate(this)
    })
    .catch((error: unknown) => {
      this.statusLine = String(error || '').trim() || 'Failed to refresh Polygon cache.'
      markVueUpdate(this)
    })
}
```

### 12. Stop forcing a full Polygon cache refresh after download/load

Add a local upsert helper.

```ts
function upsertPolygonDatasetFromDownload(
  state: DataState,
  result: PolygonDownloadAndLoadSummary,
): void {
  const csvPath = String(result.csvPath || '').trim()
  if (!csvPath) return

  const baseMeta = result.baseLoad?.meta
  const timeframe = String(baseMeta?.timeframe || '1m').trim() || '1m'
  const symbol = String(state.polygonSymbol || '').trim().toUpperCase()
  const market = String(state.polygonMarket || 'stocks').trim() || 'stocks'
  const now = Date.now()

  const existing = state.polygonDatasets.find((row) => row.csvPath === csvPath) || null

  const nextRow: PolygonDatasetSummary = {
    market,
    displaySymbol: symbol || existing?.displaySymbol || '',
    polygonTicker: symbol || existing?.polygonTicker || '',
    timeframe,
    csvPath,
    minTs: Number(baseMeta?.minTs || existing?.minTs || 0),
    maxTs: Number(baseMeta?.maxTs || existing?.maxTs || 0),
    lastUpdatedUtcMs: now,
    rowCount: Number(baseMeta?.len || existing?.rowCount || 0),
    adjusted: !!state.polygonAdjusted,
    integrity: existing?.integrity || null,
  }

  const idx = state.polygonDatasets.findIndex((row) => row.csvPath === csvPath)
  if (idx >= 0) {
    state.polygonDatasets.splice(idx, 1, nextRow)
  } else {
    state.polygonDatasets = [nextRow, ...state.polygonDatasets]
  }

  state.polygonCacheMeta = polygonCacheMetaText({
    cacheDir: state.polygonCacheDir,
    hasApiKey: state.polygonHasApiKey,
    datasets: state.polygonDatasets,
  })

  selectPolygonDatasetPath(state, csvPath)
}
```

Then update `downloadPolygonDataset()`:

```ts
downloadPolygonDataset(): void {
  setControlsBusy(this, true)
  this.statusLine = 'Downloading Polygon dataset...'
  markVueUpdate(this)

  void downloadPolygonDatasetAction({
    market: this.polygonMarket,
    symbol: this.polygonSymbol,
    from: this.polygonFrom,
    to: this.polygonTo,
    adjusted: this.polygonAdjusted,
    timeframe: this.polygonTimeframe,
  })
    .then((result) => {
      upsertPolygonDatasetFromDownload(this, result)

      const sourcePath = String(result.csvPath || '').trim()
      const symbol = String(this.polygonSymbol || '').trim().toUpperCase()
      const cacheNote = result.usedCacheOnly
        ? 'cache reused'
        : `${result.downloadedBars} bars downloaded`

      this.applyLoadResult(result.baseLoad, {
        treatAsBase: true,
        statusLine: `Loaded Polygon ${symbol || 'dataset'} (${cacheNote}; ${loadSourceText(result.baseLoad.loadSource)}).`,
        source: 'polygon',
      })

      const baseTimeframe = String(result.baseLoad?.meta?.timeframe || '').trim()
      const effectiveTimeframe = String(result.load?.meta?.timeframe || '').trim()
      if (effectiveTimeframe && baseTimeframe && effectiveTimeframe !== baseTimeframe) {
        this.applyLoadResult(result.load, {
          treatAsBase: false,
          statusLine: `Loaded Polygon ${symbol || 'dataset'} (${cacheNote}; ${loadSourceText(result.load.loadSource)}).`,
          source: 'polygon',
        })
      }

      this.csvPath = sourcePath || this.csvPath
      setControlsBusy(this, false)
      markVueUpdate(this)
    })
    .catch((error: unknown) => {
      setControlsBusy(this, false)
      this.statusLine = String(error || '').trim() || 'Polygon download failed.'
      markVueUpdate(this)
    })
}
```

## Why this file is priority #2

Most user-visible truthfulness issues are concentrated here, and this file determines whether the Rust-side fixes are actually understandable in the UI.

---

## F. `web/src/components/DataWorkspacePanel.vue`

## Goals

1. keep startup hydration safe after it becomes idempotent
2. make the current timeframe chip truthful even for base loads
3. rely on the existing `datasetContext` / `statusLine` UI surface instead of adding a new panel

## Recommended code edits

### 1. Include `baseDataMeta` in `storeToRefs`

```ts
const {
  csvPath,
  recentCsvPaths,
  sampleCsvOptions,
  sampleCsvVisible,
  marketDataDir,
  marketDataDirHint,
  dataId,
  datasetLabel,
  datasetRange,
  dataLen,
  dataTimeframe,
  dataTimeframeHint,
  autoRestoreDataset,
  datasetContext,
  dataFields,
  dataFieldHint,
  canBrowseCsv,
  canPolygonOpen,
  canLoadCsv,
  canClearData,
  canOpenMarketDataDir,
  canApplyTimeframe,
  polygonOpen,
  polygonApiKey,
  polygonMarket,
  polygonSymbol,
  polygonFrom,
  polygonTo,
  polygonTimeframe,
  polygonAdjusted,
  polygonCacheDir,
  polygonHasApiKey,
  polygonDatasets,
  polygonSelectedPath,
  polygonSelectedMeta,
  polygonCacheMeta,
  canPolygonRefresh,
  canPolygonSaveApiKey,
  canPolygonClearApiKey,
  canPolygonDownload,
  canPolygonLoadSelected,
  canPolygonValidateSelected,
  canPolygonRepairSelected,
  statusLine,
  updatedAtUnixMs,
  baseDataMeta,
} = storeToRefs(dataStore)
```

### 2. Compute an effective timeframe chip

```ts
const effectiveTimeframe = computed(() =>
  String(dataTimeframe.value || baseDataMeta.value?.timeframe || '').trim(),
)
```

### 3. Use it in `dataSummaryChips`

```ts
const dataSummaryChips = computed(() => {
  const chips: string[] = []
  const dataset = String(datasetLabel.value || '').trim()
  const timeframe = String(effectiveTimeframe.value || '').trim()
  const loadedId = String(dataId.value || '').trim()
  const fieldCount = dataFields.value.length

  if (dataset) chips.push(`Dataset: ${dataset}`)
  chips.push(`Rows: ${rowLabel.value}`)
  if (timeframe) chips.push(`Timeframe: ${timeframe}`)
  if (loadedId) chips.push(`Loaded ID: ${loadedId}`)
  if (fieldCount > 0) chips.push(`Fields: ${fieldCount}`)
  return chips
})
```

### 4. Keep the existing startup hydration call

No larger template rewrite is needed. Once `dataStore.hydrateStartupState()` becomes idempotent, this call remains safe:

```ts
onMounted(async () => {
  await nextTick()
  void dataStore.hydrateStartupState()
})
```

## Why template changes stay small

This bundle is about responsiveness and truthfulness, not UI expansion. The existing `datasetContext`, `statusLine`, and summary chips are already enough once the store starts feeding them correct data.

---

## G. `web/src/services/workflowBootstrap.ts`

## Goals

1. make startup dataset hydration a bootstrap concern, not only a component-mount concern
2. keep the call harmless if the component also calls it

## Recommended code edits

### 1. Import the data store

```ts
import { useDataStore } from '../stores/dataStore'
```

### 2. Call `hydrateStartupState()` even when there is no workflow-tab state to restore

Important: do **not** put this behind the existing early-return branch.

```ts
export function applyWorkflowBootstrapFromStorage(): void {
  const state = readWorkflowBootstrapState()

  const pinia = resolveRuntimePinia()
  if (!pinia) return

  // data startup hydration should not depend on data-panel mount timing
  const dataStore = useDataStore(pinia)
  void dataStore.hydrateStartupState()

  if (
    !state.workspace
    && !state.workflow
    && !state.workflowStep
    && !state.controlsTab
    && !state.resultsTab
    && !state.aggregateTab
    && !state.splitFraction
  ) {
    return
  }

  const store = useWorkflowStore(pinia)

  if (state.workspace) {
    if (state.workspace === 'workflow') {
      const candidateStep = state.workflowStep || (WORKFLOW_STEP_KEYS.has(state.workflow) ? state.workflow : '')
      store.selectWorkspace(state.workspace, candidateStep || undefined)
    } else {
      store.selectWorkspace(state.workspace)
    }
  } else if (state.workflow) {
    store.selectWorkflow(state.workflow)
  }

  if ((state.workspace === 'workflow' || (!state.workspace && store.workspace === 'workflow')) && state.workflowStep) {
    store.selectWorkflowStep(state.workflowStep)
  }

  if (state.controlsTab) {
    store.selectControlsTab(state.controlsTab)
  }

  if (state.resultsTab) {
    store.selectResultsTab(state.resultsTab)
  }

  if (state.aggregateTab) {
    const activeWorkspace = state.workspace || String(store.workspace || '').trim()
    if (activeWorkspace === 'aggregate') {
      store.selectAggregateTab(state.aggregateTab)
    } else {
      store.primeAggregateTab(state.aggregateTab)
    }
  }

  if (state.splitFraction) {
    store.setSplitFraction(Number(state.splitFraction), { persist: false })
  }
}
```

## Why this file matters

It is the safest place in the provided startup helpers to guarantee that dataset readiness is no longer an accidental by-product of panel mount order.

---

## H. `web/src/services/settingsBootstrap.ts`

## Recommendation

**No functional change required for this bundle.**

This file restores UI preference state, but it does not control backend market-data-root resolution or dataset residency. Do **not** move dataset restore logic into this file.

## Optional note-only change

If you want future readers to avoid repeating the same mistake, add a short comment:

```ts
// Data/dataset startup hydration is handled by the data store + workflow bootstrap,
// not by settings bootstrap.
```

That said, I would treat even this as optional.

---

# Rollout order

## Patch set 1 — Rust truthfulness + clear command

- `state.rs`
- `commands_data.rs`
- `dataService.ts`

Ship this first and run Rust tests.

## Patch set 2 — Store truthfulness + startup snapshot

- `dataStore.ts`

Ship next and manually verify:
- load CSV
- reload same CSV
- apply timeframe twice
- clear
- restart app with auto-restore on/off

## Patch set 3 — Bootstrap wiring + light panel tweak

- `workflowBootstrap.ts`
- `DataWorkspacePanel.vue`

Ship last in the safe series.

## Deferred patch set 4 — optional hardening / async follow-up

- `data_cache.rs`
- `commands_data.rs`

Only after app-path measurements justify it.

---

# Manual verification script

Run this exact sequence after implementation:

1. Launch the app fresh.
2. Confirm startup with auto-restore **off** shows no stale loaded dataset.
3. Load a CSV for the first time.
4. Confirm status mentions cold parse or binary-cache write, not generic “cached”.
5. Reload the exact same CSV without clearing.
6. Confirm:
   - same dataset ID
   - status says in-memory reuse
   - derived dataset remains valid
7. Apply `5m`.
8. Re-apply `5m`.
9. Confirm:
   - same derived ID
   - status says in-memory reuse
   - no fake cache path is displayed
10. Press Clear.
11. Confirm:
    - data disappears from UI
    - reloading the same file after Clear no longer benefits from hidden backend residency
12. Enable startup restore.
13. Load a CSV and apply a non-base timeframe.
14. Restart.
15. Confirm:
    - base dataset reloads
    - selected timeframe is reapplied
    - status is truthful during restore
16. Restart with Polygon section closed.
17. Confirm boot is not blocked by an eager Polygon cache scan.
18. Open Polygon once.
19. Confirm cache metadata loads.
20. Close and reopen Polygon.
21. Confirm it does not rescan unnecessarily unless Refresh is pressed.
22. Download/load a Polygon dataset.
23. Confirm the cached-dataset list is updated without a full forced rescan.

---

# Risks and mitigations

## Risk 1 — frontend expectations about `cache_hit`

**Risk:** other UI code outside the provided bundle may still treat `cache_hit=true` as “binary cache”.

**Mitigation:** keep `cache_hit` backward compatible for now, but switch the provided files to prefer `load_source` everywhere.

## Risk 2 — snapshot drift

**Risk:** startup snapshot gets stale if the file no longer exists.

**Mitigation:** startup restore catch path should leave the app in a truthful “restore failed” state and overwrite the snapshot on the next user action.

## Risk 3 — component mount vs bootstrap duplication

**Risk:** both `workflowBootstrap.ts` and `DataWorkspacePanel.vue` call `hydrateStartupState()`.

**Mitigation:** the store implementation in this plan is explicitly idempotent.

## Risk 4 — deferred cache hardening can hurt warm load

**Risk:** stronger cache validation hashes source files and makes `.vta` warm loads slower.

**Mitigation:** keep it deferred and opt-in unless app-path measurement proves it is safe.

---

# Final recommendation

If you do only three things from this document, do these first:

1. **Fix base reuse in `state.rs`.**
2. **Add `load_source` and stop returning fake `cache_path` values.**
3. **Implement a real startup restore snapshot in `dataStore.ts` and call it from `workflowBootstrap.ts`.**

That combination addresses the biggest correctness and latency-truthfulness problems without requiring a redesign.

---

# Comprehensive implementation checklist

## Phase 1 — Rust response contract and base-reuse fix

### `src-tauri/src/state.rs`
- [ ] Add `LoadSource` enum with `Serialize`, `PartialEq`, and `Eq`.
- [ ] Extend `LoadPriceDataResponse` with `load_source`.
- [ ] Remove the `ctx.timeframe.is_none()` gate from `find_matching_loaded_base(...)`.
- [ ] Use `DatasetKind::Base` to identify reusable base datasets.
- [ ] Ensure matching base reuse chooses a valid loaded base dataset.
- [ ] Return `LoadSource::InMemoryBaseReuse` for identical base reuse.
- [ ] Return `LoadSource::BinaryCache` for `.vta` hits.
- [ ] Return `LoadSource::CsvParsedAndCached` for cold parse + successful cache write.
- [ ] Return `LoadSource::CsvParsedCacheWriteSkipped` for cold parse + failed/skipped cache write.
- [ ] Return `LoadSource::DerivedFreshResample` for a newly built derived timeframe.
- [ ] Return `LoadSource::DerivedInMemoryReuse` for a repeated fresh derived reuse.
- [ ] Stop returning `ctx.csv_path` as `cache_path` in `resample_loaded_data(...)`.
- [ ] Keep `cache_path` empty for in-memory reuse / fresh resample paths.

### `src-tauri/src/commands_data.rs`
- [ ] Add `ClearLoadedDataResponse`.
- [ ] Add `clear_loaded_data` Tauri command.
- [ ] Treat empty dataset ID as a no-op clear.
- [ ] Treat unknown dataset ID as an idempotent clear rather than a hard user-visible failure.
- [ ] Keep existing load/resample commands stable for this patch set.

### Rust tests
- [ ] Add test: identical base load reuses existing ID when timeframe is inferred.
- [ ] Add test: identical base load reports `InMemoryBaseReuse`.
- [ ] Add test: repeated resample reports `DerivedInMemoryReuse`.
- [ ] Add test: fresh resample reports `DerivedFreshResample`.
- [ ] Add test: repeated resample returns empty `cache_path`.
- [ ] Re-run existing residency tests and confirm they now exercise the intended fast path.

## Phase 2 — Service/store contract and truthful UI status

### `web/src/services/dataService.ts`
- [ ] Add `DataLoadSource` union type.
- [ ] Extend `LoadPriceDataResponse` and `PolygonDownloadAndLoadResponse` to accept `load_source`.
- [ ] Add `normalizeLoadSource(...)`.
- [ ] Add `loadSource` to `DataLoadResult`.
- [ ] Update `normalizeLoadPriceDataResponse(...)` to populate `loadSource`.
- [ ] Replace no-op `clearData()` with backend command invocation.

### `web/src/stores/dataStore.ts`
- [ ] Add `baseSourceKind` to `DataState`.
- [ ] Add startup snapshot key and helper types.
- [ ] Add `readStartupSnapshot(...)`.
- [ ] Add `writeStartupSnapshot(...)`.
- [ ] Add `persistStartupSnapshot(...)`.
- [ ] Add `loadSourceText(...)`.
- [ ] Update `datasetContextText(...)` to include `load=` and optional `cache=`.
- [ ] Add `baseLoadStatusText(...)`.
- [ ] Add `derivedLoadStatusText(...)`.
- [ ] Update `applyLoadResult(...)` to use `loadSource`.
- [ ] Update `applyLoadResult(...)` to persist the startup snapshot.
- [ ] Update `applyLoadResult(...)` to preserve base-vs-derived context correctly.
- [ ] Update `applyBaseDatasetState()` to persist the startup snapshot.
- [ ] Update `setCheckboxField(...)` so `autoRestoreDataset` changes are persisted.
- [ ] Convert `clearData()` to async.
- [ ] In `clearData()`, call the backend clear command when a base dataset exists.
- [ ] In `clearData()`, reset base dataset state, derived state, and artifacts.
- [ ] In `clearData()`, persist a “no loaded dataset” startup snapshot.
- [ ] Replace all hard-coded `cacheHit ? "binary cache"` UI wording with `loadSource`-driven wording.
- [ ] Update CSV browse/load/sample load actions to use `baseLoadStatusText(...)`.
- [ ] Update timeframe apply action to use `derivedLoadStatusText(...)`.

## Phase 3 — Startup dataset restore and bootstrap truthfulness

### `web/src/stores/dataStore.ts`
- [ ] Convert `hydrateStartupState()` to an async, idempotent bootstrap action.
- [ ] Read startup snapshot at the beginning of hydration.
- [ ] Apply saved `autoRestoreDataset` preference immediately.
- [ ] Keep market-data-dir info fetch.
- [ ] Keep sample CSV fetch.
- [ ] Remove unconditional Polygon cache fetch from startup.
- [ ] If startup restore is enabled and a base path exists, actually load it.
- [ ] If a saved derived timeframe exists and differs from the base timeframe, reapply it.
- [ ] Ensure startup restore updates the same UI state path as normal load actions.
- [ ] Surface startup restore failures truthfully in `statusLine`.
- [ ] Avoid leaving stale IDs or metadata after a failed restore.

### `web/src/stores/dataStore.ts` — legacy snapshot sanitation
- [ ] Stop trusting persisted `dataId` from snapshot.
- [ ] Stop trusting persisted `baseDataId` from snapshot.
- [ ] Stop trusting persisted `baseDataMeta` from snapshot.
- [ ] Stop trusting persisted `datasetRange`, `dataLen`, and field hints from snapshot.
- [ ] Keep user-entered inputs and preference-like fields from snapshot.
- [ ] Set `statusLine` to “Startup restore pending.” when appropriate.
- [ ] Reset `canApplyTimeframe` until a real backend restore completes.

### `web/src/services/workflowBootstrap.ts`
- [ ] Import `useDataStore`.
- [ ] Resolve Pinia before early return.
- [ ] Call `void dataStore.hydrateStartupState()` during workflow bootstrap.
- [ ] Keep existing workspace/tab bootstrap logic unchanged.

### `web/src/components/DataWorkspacePanel.vue`
- [ ] Keep the existing `onMounted()` bootstrap call as an idempotent fallback.
- [ ] Confirm no duplicate-restore side effects occur once the store guard is in place.

## Phase 4 — Polygon metadata scan cleanup

### `web/src/stores/dataStore.ts`
- [ ] Add a module-level `polygonCacheLoadedOnce` guard.
- [ ] Set the guard inside `applyPolygonCacheInfo(...)`.
- [ ] Remove Polygon cache info fetch from default startup hydration.
- [ ] In `openPolygon()`, fetch cache info only on first open (or when explicitly refreshed).
- [ ] Keep `refreshPolygon()` as the manual explicit rescan path.
- [ ] Add `upsertPolygonDatasetFromDownload(...)`.
- [ ] Use download result metadata to update the local Polygon dataset list without a full rescan.
- [ ] Recompute `polygonCacheMeta` after local upsert.
- [ ] Re-select the just-downloaded dataset after local upsert.

## Phase 5 — Small Data panel polish

### `web/src/components/DataWorkspacePanel.vue`
- [ ] Include `baseDataMeta` in `storeToRefs`.
- [ ] Add computed `effectiveTimeframe`.
- [ ] Show base timeframe in summary chips even when `dataTimeframe` is empty.
- [ ] Confirm `datasetContext` now shows truthful load-path details coming from the store.

## Phase 6 — Deferred optional hardening

### `src-tauri/src/data_cache.rs`
- [ ] Decide whether strong cache validation is actually needed.
- [ ] If yes, use the spare header bytes for flags + optional source hash.
- [ ] Gate stronger validation so it does not regress normal warm loads by default.
- [ ] Add cache-version compatibility tests if you bump header version.

### `src-tauri/src/commands_data.rs`
- [ ] Re-measure app-path load/resample latency after phases 1–4.
- [ ] Only then decide whether `spawn_blocking` is still worth the complexity.
- [ ] If proceeding, separate that change into its own patch series and benchmark it independently.

## Verification checklist

### Functional
- [ ] Cold CSV load works.
- [ ] Warm `.vta` load works.
- [ ] Warm identical base reload keeps the same dataset ID.
- [ ] Repeated same resample keeps the same derived dataset ID.
- [ ] Clear removes the currently loaded dataset from backend state.
- [ ] Startup restore replays both dataset and timeframe when enabled.
- [ ] Startup restore does not claim a loaded dataset when restore is disabled or fails.

### Truthfulness
- [ ] Binary-cache hit is labeled as binary-cache hit.
- [ ] In-memory base reuse is labeled as in-memory base reuse.
- [ ] Fresh resample is labeled as fresh resample.
- [ ] Reused derived resample is labeled as in-memory derived reuse.
- [ ] `cache_path` is never populated with a CSV path.
- [ ] No stale dataset ID appears immediately after startup before a real restore finishes.

### Responsiveness
- [ ] Startup with Polygon closed no longer performs a default Polygon cache scan.
- [ ] First Polygon open loads metadata once.
- [ ] Reopening Polygon does not rescan unless Refresh is pressed.
- [ ] Downloading/loading a Polygon dataset does not force a full cache rescan just to update the list.

### Regression safety
- [ ] Existing Rust residency tests pass.
- [ ] New Rust fast-path tests pass.
- [ ] Manual startup restore scenarios pass with auto-restore both on and off.
- [ ] Manual Clear + reload scenario shows no hidden stale residency.

