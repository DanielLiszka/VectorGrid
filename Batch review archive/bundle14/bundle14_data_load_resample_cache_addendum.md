# Bundle 14 Addendum — Extra Corrections and Optional Performance/Correctness Improvements

This addendum supplements `bundle14_data_load_resample_cache_plan.md`.

It has three goals:

1. correct a few implementation details that deserve tightening before you code the main plan
2. add several small, parity-safe improvements that stay close to Bundle 14’s scope
3. capture optional follow-up changes that go slightly beyond the prompt but still align with your stated priority: **performance, optimization, and correctness without removing features or breaking functionality**

---

## 1. What I would correct in the original MD plan

### Correction A — do not use `csvPath` as the persisted startup-restore source path

This is the most important correction.

In the current store, `csvPath` is both:

- the editable input field value
- the last successfully loaded path

That means it can drift away from the actually loaded base dataset if the user edits the input after loading but does not load again. If startup snapshot persistence uses `csvPath`, auto-restore can later restore the wrong file.

### Why this matters

A safe startup snapshot must persist the **actual loaded base dataset path**, not the current input draft.

### Recommended change

Add a dedicated field like `baseCsvPath` or `loadedBasePath` to `DataState` and set it only on successful base loads. Use that field for:

- startup snapshot persistence
- clear semantics
- restore replay
- “current loaded source” truthfulness

### Recommended code edits

#### Extend `DataState`

```ts
interface DataState {
  // existing fields...
  baseCsvPath: string
  baseSourceKind: 'csv' | 'polygon'
  baseSourceSizeBytes: number | null
  baseSourceMtimeUtcMs: number | null
}
```

Default values:

```ts
baseCsvPath: '',
baseSourceKind: 'csv',
baseSourceSizeBytes: null,
baseSourceMtimeUtcMs: null,
```

#### Set it only on successful base load

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
    this.baseCsvPath = path
    this.baseSourceKind = sourceKind
    this.baseSourceSizeBytes = meta?.sourceSizeBytes ?? null
    this.baseSourceMtimeUtcMs = meta?.sourceMtimeUtcMs ?? null
    this.baseDatasetLabel = fileBasename(path) || this.baseDatasetLabel || this.datasetLabel || '—'
    this.baseDatasetContext = datasetContextText(path, sourceKind)
  }

  // rest of existing logic...
}
```

#### Persist the startup snapshot from `baseCsvPath`, not `csvPath`

```ts
interface DataStartupSnapshot {
  autoRestoreDataset: boolean
  baseCsvPath: string
  selectedTimeframe: string
  source: 'csv' | 'polygon'
  baseSourceSizeBytes: number | null
  baseSourceMtimeUtcMs: number | null
  savedAtUnixMs: number
}

function persistStartupSnapshot(state: DataState): void {
  writeStartupSnapshot({
    autoRestoreDataset: !!state.autoRestoreDataset,
    baseCsvPath: hasLoadedDataset(state.baseDataId)
      ? String(state.baseCsvPath || '').trim()
      : '',
    selectedTimeframe: hasLoadedDataset(state.baseDataId)
      ? effectiveSelectedTimeframe(state)
      : '',
    source: state.baseSourceKind,
    baseSourceSizeBytes: hasLoadedDataset(state.baseDataId)
      ? state.baseSourceSizeBytes
      : null,
    baseSourceMtimeUtcMs: hasLoadedDataset(state.baseDataId)
      ? state.baseSourceMtimeUtcMs
      : null,
    savedAtUnixMs: Date.now(),
  })
}
```

#### Clear it explicitly

```ts
this.baseCsvPath = ''
this.baseSourceKind = 'csv'
this.baseSourceSizeBytes = null
this.baseSourceMtimeUtcMs = null
```

### Extra test to add

```ts
it('persists startup restore from the loaded base path, not the edited csv input draft', async () => {
  // load A successfully
  // edit csvPath input to B without loading B
  // persist snapshot
  // expect snapshot.baseCsvPath to remain A
})
```

---

### Correction B — sanitize `applyLegacySnapshot(...)` more aggressively

The original plan correctly stopped trusting persisted dataset IDs and live backend-residency fields, but I would go further.

The store should also avoid trusting stale runtime/cache summary fields from a snapshot:

- `polygonCacheDir`
- `polygonHasApiKey`
- `polygonDatasets`
- `polygonSelectedMeta`
- `polygonCacheMeta`
- `can*` control flags
- `statusLine`
- `source`
- `updatedAtUnixMs`

These are all transient runtime outputs, not durable user intent.

### Why this matters

Once Polygon cache loading becomes lazy, restoring old `polygonDatasets` or old cache/auth summaries from snapshot makes the UI look “warm” before any real refresh has happened. That undermines the same startup truthfulness goal the main plan is trying to protect.

### Recommended change

Treat only these as durable snapshot inputs:

- visible tab/open state if you really want it
- typed CSV path input
- market-data-dir input text
- Polygon market/symbol/date/timeframe/adjusted input values
- auto-restore preference

Everything else should be recomputed from fresh runtime state.

### Tightened `applyLegacySnapshot(...)` sketch

```ts
applyLegacySnapshot(snapshot: Partial<DataState>): void {
  this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
  this.csvPath = String(snapshot.csvPath ?? this.csvPath ?? '').trim()
  this.recentCsvPaths = normalizedList(snapshot.recentCsvPaths ?? this.recentCsvPaths ?? [])
  this.marketDataDir = String(snapshot.marketDataDir ?? this.marketDataDir ?? '').trim()
  this.marketDataMoveExisting = Boolean(snapshot.marketDataMoveExisting ?? this.marketDataMoveExisting)
  this.marketDataDirHint = '—'

  this.dataTimeframe = String(snapshot.dataTimeframe ?? this.dataTimeframe ?? '').trim()
  this.dataTimeframeHint = 'Load a dataset to enable timeframe resampling.'
  this.autoRestoreDataset = Boolean(snapshot.autoRestoreDataset ?? this.autoRestoreDataset)

  this.polygonOpen = Boolean(snapshot.polygonOpen ?? this.polygonOpen)
  this.polygonApiKey = String(snapshot.polygonApiKey ?? this.polygonApiKey ?? '').trim()
  this.polygonMarket = String(snapshot.polygonMarket ?? this.polygonMarket ?? 'stocks').trim() || 'stocks'
  this.polygonSymbol = String(snapshot.polygonSymbol ?? this.polygonSymbol ?? '').trim().toUpperCase()
  this.polygonFrom = normalizeDatasetDateInput(String(snapshot.polygonFrom ?? this.polygonFrom ?? '').trim())
  this.polygonTo = normalizeDatasetDateInput(String(snapshot.polygonTo ?? this.polygonTo ?? '').trim())
  this.polygonTimeframe = String(snapshot.polygonTimeframe ?? this.polygonTimeframe ?? '1m').trim() || '1m'
  this.polygonAdjusted = Boolean(snapshot.polygonAdjusted ?? this.polygonAdjusted)

  // runtime/cache summaries are not trusted from snapshot
  this.polygonCacheDir = ''
  this.polygonHasApiKey = false
  this.polygonDatasets = []
  this.polygonSelectedPath = ''
  this.polygonSelectedMeta = 'Select a cached Polygon dataset to inspect, validate, repair, or load.'
  this.polygonCacheMeta = 'Polygon cache unavailable.'

  this.dataId = '—'
  this.datasetLabel = '—'
  this.datasetRange = '—'
  this.dataLen = '—'
  this.datasetContext = '—'
  this.dataFields = []
  this.dataFieldHint = ''
  this.baseDataId = ''
  this.baseDataMeta = null
  this.baseCsvPath = ''
  this.baseDatasetLabel = ''
  this.baseDatasetContext = ''
  this.baseSourceKind = 'csv'
  this.baseSourceSizeBytes = null
  this.baseSourceMtimeUtcMs = null

  this.statusLine = this.autoRestoreDataset && this.csvPath
    ? 'Startup restore pending.'
    : 'Ready.'

  setControlsBusy(this, false)
  this.source = 'vue'
  this.updatedAtUnixMs = Date.now()
}
```

---

## 2. Extra near-scope improvements I would add to the plan

### Addition 1 — normalize source paths for in-memory base reuse

The current reuse path in `state.rs` matches by exact string path plus `SourceIdentity`. That misses reuse when the same file is referenced via different but equivalent spellings:

- relative vs absolute path
- different slash style
- canonical vs non-canonical path
- Windows case differences

`data_cache.rs` already canonicalizes for fallback-cache file naming, so reusing a normalized source key for in-memory dataset matching is consistent with the existing direction.

### Recommended change

Store both:

- the original `csv_path` for display
- a normalized `source_key` for matching

### Suggested Rust snippet

```rust
fn normalized_source_key(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        text.to_ascii_lowercase()
    }

    #[cfg(not(target_os = "windows"))]
    {
        text
    }
}
```

Extend context:

```rust
#[derive(Debug, Clone)]
pub struct LoadedDataContext {
    pub csv_path: String,
    pub source_key: String,
    pub label: String,
    pub timeframe: Option<String>,
    pub source_identity: Option<data_cache::SourceIdentity>,
}
```

Use it when inserting base or derived contexts:

```rust
let source_key = normalized_source_key(pathbuf);

ctx_map.insert(
    id.clone(),
    LoadedDataContext {
        csv_path: path.to_string(),
        source_key: source_key.clone(),
        label: dataset_label_from_path(path),
        timeframe: meta.timeframe.clone(),
        source_identity: Some(outcome.source_identity),
    },
);
```

And when matching:

```rust
let pathbuf = Path::new(path);
let source_key = normalized_source_key(pathbuf);

let candidate_ids: Vec<String> = {
    let ctx_map = self
        .loaded_data_context
        .lock()
        .map_err(|_| "failed to lock loaded-data context map".to_string())?;

    ctx_map
        .iter()
        .filter_map(|(id, ctx)| {
            (ctx.source_key == source_key && ctx.source_identity == Some(source_identity))
                .then(|| id.clone())
        })
        .collect()
};
```

### Why I would do this

It improves the same hot reuse path the main plan already prioritizes, and it does so with a small change instead of a redesign.

---

### Addition 2 — avoid redundant `get_market_data_dir_info` round trips in `dataService.ts`

This is a small but clean win that the original plan did not call out.

`commands_data.rs` already returns `MarketDataDirInfo` from:

- `set_market_data_dir`
- `clear_market_data_dir`

But `dataService.ts` currently discards that returned payload and makes a second command call via `getMarketDataDirInfo()`.

### Recommended change

Normalize the returned payload directly.

```ts
export async function setMarketDataDir(dir: string, moveExisting: boolean): Promise<MarketDataDirInfoSummary> {
  const normalizedDir = normalizeText(dir)
  if (!normalizedDir) {
    throw new Error('Enter a market data folder path first.')
  }

  const raw = await invokeCommand('set_market_data_dir', {
    dir: normalizedDir,
    moveExisting: !!moveExisting,
  })
  return normalizeMarketDataDirInfo(raw)
}

export async function resetMarketDataDir(): Promise<MarketDataDirInfoSummary> {
  const raw = await invokeCommand('clear_market_data_dir')
  return normalizeMarketDataDirInfo(raw)
}
```

### Why I would add this

It removes avoidable command churn and keeps the service layer honest about what the backend already returned.

---

### Addition 3 — reset Polygon cache state when market-data root changes

Changing or resetting the market-data root changes the effective Polygon cache directory. The current store-level market-data-dir actions do not clear the old Polygon dataset list or force a fresh cache read.

### Why this matters

Without a reset, the UI can temporarily show a dataset list and cache-dir metadata from the old root.

### Recommended change

Add a helper that clears all runtime Polygon cache state and the lazy-load guards.

```ts
let polygonCacheLoadedOnce = false
let polygonCacheLoadPromise: Promise<void> | null = null

function resetPolygonCacheRuntimeState(state: DataState): void {
  polygonCacheLoadedOnce = false
  polygonCacheLoadPromise = null
  state.polygonCacheDir = ''
  state.polygonHasApiKey = false
  state.polygonDatasets = []
  state.polygonSelectedPath = ''
  state.polygonSelectedMeta = 'Select a cached Polygon dataset to inspect, validate, repair, or load.'
  state.polygonCacheMeta = 'Polygon cache unavailable.'
}
```

Use it after market-data-dir updates:

```ts
setMarketDataDir(dir: string, moveExisting: boolean): void {
  const nextDir = String(dir || '').trim()
  const nextMoveExisting = !!moveExisting
  if (nextDir) this.marketDataDir = nextDir
  this.marketDataMoveExisting = nextMoveExisting
  markVueUpdate(this)

  void setMarketDataDirAction(nextDir, nextMoveExisting)
    .then((info) => {
      const previousPolygonCacheDir = String(this.polygonCacheDir || '').trim()
      const nextPolygonCacheDir = String(info?.polygonCacheDir || '').trim()

      this.marketDataDir = String(info?.configuredRootDir || '').trim()
      this.marketDataDirHint = String(info?.hint || this.marketDataDirHint || '—').trim() || '—'

      if (!previousPolygonCacheDir || previousPolygonCacheDir !== nextPolygonCacheDir) {
        resetPolygonCacheRuntimeState(this)
      }

      this.statusLine = 'Updated market data folder.'
      markVueUpdate(this)
    })
    .catch((error: unknown) => {
      this.statusLine = String(error || '').trim() || 'Failed to update market data folder.'
      markVueUpdate(this)
    })
}
```

Do the same for `resetMarketDataDir()`.

---

### Addition 4 — make Polygon cache loading promise-deduped, not just “loaded once” guarded

The main plan added `polygonCacheLoadedOnce`, which is useful, but I would tighten it with an in-flight promise the same way startup hydration already uses `startupHydratePromise`.

### Why this matters

It prevents duplicate scans when:

- the pane is opened twice quickly
- refresh is clicked during an in-flight load
- startup hydration and pane-open logic overlap

### Recommended helper

```ts
async function ensurePolygonCacheLoaded(state: DataState, force = false): Promise<void> {
  if (polygonCacheLoadPromise && !force) {
    await polygonCacheLoadPromise
    return
  }

  polygonCacheLoadPromise = (async () => {
    const info = await getPolygonCacheInfoAction()
    applyPolygonCacheInfo(state, info, state.polygonSelectedPath)
  })()

  try {
    await polygonCacheLoadPromise
  } finally {
    polygonCacheLoadPromise = null
  }
}
```

Then use:

```ts
openPolygon(): void {
  const pinia = resolveRuntimePinia()
  if (pinia) {
    useWorkflowStore(pinia).selectWorkspace('workflow', 'data')
  }

  this.polygonOpen = true
  markVueUpdate(this)
  void openPolygonAction().catch(() => undefined)

  if (!polygonCacheLoadedOnce) {
    setControlsBusy(this, true)
    this.statusLine = 'Loading Polygon cache...'
    markVueUpdate(this)

    void ensurePolygonCacheLoaded(this)
      .then(() => {
        this.statusLine = 'Loaded Polygon cache.'
        markVueUpdate(this)
      })
      .catch((error: unknown) => {
        this.statusLine = String(error || '').trim() || 'Failed to refresh Polygon cache.'
        markVueUpdate(this)
      })
      .finally(() => {
        setControlsBusy(this, false)
        markVueUpdate(this)
      })
  }
}
```

---

### Addition 5 — remove `setControlsBusy(state, false)` from `applyPolygonCacheInfo(...)`

This helper currently acts like a view-state mutator instead of a pure “apply payload” helper.

### Why this matters

It can prematurely re-enable controls in the middle of larger workflows, especially if reused by:

- refresh
- API key save/clear
- download + load
- startup hydration

### Recommended change

Keep the helper focused on applying data only.

```ts
function applyPolygonCacheInfo(state: DataState, info: PolygonCacheInfoSummary, preferredPath = ''): void {
  polygonCacheLoadedOnce = true
  state.polygonCacheDir = String(info.cacheDir || '').trim()
  state.polygonHasApiKey = !!info.hasApiKey
  state.polygonDatasets = Array.isArray(info.datasets) ? [...info.datasets] : []
  state.polygonCacheMeta = polygonCacheMetaText(info)
  selectPolygonDatasetPath(state, preferredPath)
}
```

Let each caller own busy-state transitions.

---

## 3. Optional changes that go beyond the prompt but still fit your goals

### Optional 1 — make Polygon API-key save/clear paths avoid a full cache rescan

Today the service layer does this pattern:

- invoke `polygon_set_api_key` or `polygon_clear_api_key`
- immediately call `getPolygonCacheInfo()`

That second step can be avoided because saving or clearing the key does not change the dataset list.

### Lowest-risk version

Keep commands returning `()` and update the store locally.

#### Service layer

```ts
export async function setPolygonApiKey(apiKey: string): Promise<void> {
  const normalizedKey = normalizeText(apiKey)
  if (!normalizedKey) throw new Error('Enter a Polygon API key.')
  await invokeCommand('polygon_set_api_key', { apiKey: normalizedKey })
}

export async function clearPolygonApiKey(): Promise<void> {
  await invokeCommand('polygon_clear_api_key')
}
```

#### Store layer

```ts
savePolygonApiKey(): void {
  const apiKey = String(this.polygonApiKey || '').trim()
  if (!apiKey) {
    this.statusLine = 'Enter a Polygon API key.'
    markVueUpdate(this)
    return
  }

  setControlsBusy(this, true)
  this.statusLine = 'Saving Polygon API key...'
  markVueUpdate(this)

  void setPolygonApiKeyAction(apiKey)
    .then(() => {
      this.polygonHasApiKey = true
      this.polygonCacheMeta = polygonCacheMetaText({
        cacheDir: this.polygonCacheDir,
        hasApiKey: true,
        datasets: this.polygonDatasets,
      })
      this.statusLine = 'Saved Polygon API key.'
      markVueUpdate(this)
    })
    .catch((error: unknown) => {
      this.statusLine = String(error || '').trim() || 'Failed to save Polygon API key.'
      markVueUpdate(this)
    })
    .finally(() => {
      setControlsBusy(this, false)
      markVueUpdate(this)
    })
}
```

Do the equivalent for clear.

### Why I like this change

It is small, parity-safe, and removes another avoidable metadata scan.

---

### Optional 2 — add a `source_path` field to `LoadPriceDataResponse`

This is not strictly required if you add `baseCsvPath` to the store, but it makes the response contract cleaner and more self-describing.

### Why this helps

Right now the frontend injects `path` into the normalized response from the caller context. That is fine for base loads, but derived responses naturally have no direct path in the command call, so the store relies on separately remembered base state.

### Recommended shape

```rust
#[derive(Debug, Clone, Serialize)]
pub struct LoadPriceDataResponse {
    pub id: String,
    pub meta: PriceDataMeta,
    pub cache_hit: bool,
    pub cache_path: String,
    pub load_source: LoadSource,
    pub source_path: String,
}
```

Set it in base load and derived reuse/build paths from `LoadedDataContext`.

### When I would do this

If you want the load contract to be easier to reason about for debugging, startup restore, or future tests.

---

### Optional 3 — split `spawn_blocking` follow-up into a safer two-stage design

The original plan correctly deferred command-level async/offload work. If you later pursue it, I would narrow the design.

### Better shape than “move everything into a blocking closure”

#### For CSV/base load

1. command becomes async
2. compute source identity on the cheap
3. `spawn_blocking` only for `data_cache::load_or_build_with_identity(...)`
4. after `.await`, register the returned `Candles` into `AppState` on the normal command path

That implies a new helper like:

```rust
pub fn register_loaded_price_data(
    &self,
    path: &str,
    outcome: data_cache::LoadOutcome,
) -> Result<LoadPriceDataResponse, String>
```

#### For resample

1. get `Arc<Candles>` and base context on the normal path
2. `spawn_blocking` only for `ta_app_core::resample_candles(...)`
3. register the derived result afterward in state

That implies a helper like:

```rust
pub fn register_resampled_data(
    &self,
    base_data_id: &str,
    timeframe: &str,
    derived: Candles,
) -> Result<LoadPriceDataResponse, String>
```

### Why this is better

It keeps state mutation structured and localized while still offloading the heavy I/O/CPU portion.

---

### Optional 4 — compare startup snapshot identity to the restored source identity

If you add `baseSourceSizeBytes` and `baseSourceMtimeUtcMs` to the snapshot, you can surface a truthful note when the “same path” now points to changed contents.

### Example

```ts
const restoredSize = base.meta?.sourceSizeBytes ?? null
const restoredMtime = base.meta?.sourceMtimeUtcMs ?? null
const identityChanged = (
  startup.baseSourceSizeBytes != null
  && restoredSize != null
  && startup.baseSourceSizeBytes !== restoredSize
) || (
  startup.baseSourceMtimeUtcMs != null
  && restoredMtime != null
  && startup.baseSourceMtimeUtcMs !== restoredMtime
)

this.statusLine = identityChanged
  ? 'Restored last dataset, but the source file changed since the prior session.'
  : `Restored last dataset (${loadSourceText(base.loadSource)}).`
```

### Why this is useful

It does not block restore, but it keeps the restore path honest.

---

### Optional 5 — clear or rewrite the startup snapshot when market-data-root changes affect a loaded Polygon dataset

This is the furthest beyond the core prompt, but it is relevant to correctness.

If the loaded base dataset is a Polygon dataset under the old cache root and the user changes the market-data root with `moveExisting=true`, the currently saved `baseCsvPath` may no longer point to the new location.

### Safe first step

If the loaded base dataset came from Polygon and its `baseCsvPath` is under the old Polygon cache dir, clear the startup snapshot and surface a one-line status note:

```ts
if (
  this.baseSourceKind === 'polygon'
  && previousPolygonCacheDir
  && this.baseCsvPath.startsWith(previousPolygonCacheDir)
) {
  this.baseDataId = ''
  this.baseCsvPath = ''
  persistStartupSnapshot(this)
  this.statusLine = 'Market data folder changed. Reload the Polygon dataset to refresh startup restore state.'
}
```

### Why I would keep it optional

It is correct, but it touches the edge case where storage-path moves and active loaded dataset state intersect.

---

## 4. Additional tests I would add

### Rust-side

1. same file loaded via normalized-equivalent path still reuses the same base dataset id
2. `load_source` stays truthful when path spellings differ but source identity is the same
3. `source_path` is preserved on derived responses if you adopt that contract addition

### Store/service-side

1. startup snapshot uses `baseCsvPath`, not the editable `csvPath`
2. `applyLegacySnapshot(...)` does not restore stale runtime cache summaries
3. `setMarketDataDirAction()` and `resetMarketDataDirAction()` do not issue a redundant second info request
4. market-data-root changes reset Polygon cache runtime state
5. repeated `openPolygon()` calls while a cache load is in flight dedupe to one request
6. API-key save/clear paths do not force a full cache rescan if you adopt the local-update optimization

---

## 5. Revised priority order after these add-ons

### Must add to the main MD plan

1. **`baseCsvPath` / loaded-base-path field in the store**
2. **do not trust stale Polygon cache/runtime fields in `applyLegacySnapshot(...)`**
3. **normalize returned market-data-dir info directly in `dataService.ts`**
4. **reset Polygon cache runtime state when market-data root changes**
5. **remove `setControlsBusy(..., false)` from `applyPolygonCacheInfo(...)`**

### Strong optional additions

6. **normalized `source_key` for in-memory base reuse**
7. **promise-deduped `ensurePolygonCacheLoaded(...)` helper**
8. **local update on Polygon API-key save/clear instead of rescanning cache**

### Optional later follow-up

9. **`source_path` in `LoadPriceDataResponse`**
10. **two-stage `spawn_blocking` split for load/resample**
11. **startup snapshot identity comparison**
12. **market-data-root move handling for loaded Polygon snapshot paths**

---

## 6. Bottom-line assessment

I do **not** think the original MD missed another equally large Bundle-14 issue on the level of:

- broken warm base reuse
- fake cache-hit/status semantics
- frontend-only clear
- fake startup restore
- eager Polygon startup scans

Those are still the right primary fixes.

But I **did** miss one meaningful implementation detail in the original plan:

- the startup snapshot should not be persisted from `csvPath`; it needs a dedicated loaded-base path field

And I would also tighten the plan with the additional store/service/runtime-state cleanups above, because they are small, low-risk, and very aligned with your performance/correctness goals.

---

## 7. Addendum checklist

- [ ] Add `baseCsvPath` to `DataState`.
- [ ] Add `baseSourceSizeBytes` and `baseSourceMtimeUtcMs` to `DataState`.
- [ ] Set those fields only on successful base loads.
- [ ] Persist startup snapshot from `baseCsvPath`, not `csvPath`.
- [ ] Add a store test covering “loaded path vs edited draft path” snapshot behavior.
- [ ] Tighten `applyLegacySnapshot(...)` so it does not trust stale Polygon cache/runtime summary fields.
- [ ] Stop restoring transient `can*` flags from snapshot state.
- [ ] Normalize returned `set_market_data_dir` payload directly in `dataService.ts`.
- [ ] Normalize returned `clear_market_data_dir` payload directly in `dataService.ts`.
- [ ] Add `resetPolygonCacheRuntimeState(...)` helper.
- [ ] Call that helper when market-data-root changes or resets.
- [ ] Remove implicit busy-state mutation from `applyPolygonCacheInfo(...)`.
- [ ] Add `polygonCacheLoadPromise` and `ensurePolygonCacheLoaded(...)`.
- [ ] Use the deduped helper in `openPolygon()` and explicit refresh paths.
- [ ] Decide whether to make API-key save/clear update local state instead of rescanning the full Polygon cache.
- [ ] If adopted, simplify `setPolygonApiKey()` and `clearPolygonApiKey()` service functions to return `Promise<void>`.
- [ ] Decide whether to add a normalized Rust-side `source_key` for in-memory base reuse.
- [ ] If adopted, extend `LoadedDataContext` with `source_key`.
- [ ] If adopted, add reuse tests for path spelling differences.
- [ ] Decide whether to add `source_path` to `LoadPriceDataResponse`.
- [ ] Decide whether to add startup snapshot identity comparison.
- [ ] Decide whether to clear startup snapshot when a loaded Polygon source moves with a market-data-root change.
- [ ] Keep these add-ons behind the same parity-safe rollout discipline as the original plan: correctness first, then truthfulness, then metadata-scan trimming, then heavier async/offload work.
