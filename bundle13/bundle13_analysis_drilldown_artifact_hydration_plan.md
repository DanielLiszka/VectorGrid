
# Bundle 13 Implementation Plan: Analysis / Drilldown / Artifact-Hydration Responsiveness

## Scope and intent

This document follows the `## 13. Analysis/drilldown and artifact-hydration responsiveness` bundle from `gpt54_feature_review_map.md`.

That bundle explicitly owns:

- drilldown and local-stability command execution
- analysis-table, heatmap, and comparison hydration from workflow artifacts
- legacy-runtime to Vue artifact sync and refresh behavior

It explicitly asks for:

- drilldown/local-stability latency review
- artifact-hydration and sync-boundary review
- cache and recompute review
- user-visible responsiveness review

This plan stays inside that scope. It does **not** propose a new chart stack, a new UI surface, a broad architecture rewrite, or generic feature expansion. The goal is parity-safe responsiveness improvement on the current product path.

## Files covered

Primary files:

- `src-tauri/src/commands_analysis.rs`
- `web/src/services/analysisService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`

Supporting benchmark / workflow files:

- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

---

# 1. Surface understanding

This surface is dominated by three categories of work:

1. **Artifact hydration and sync**
   - `workflowArtifactsSync.ts` lists run artifacts, loads the latest artifact, normalizes it, applies it to the Vue/Pinia runtime, and then triggers `refreshAnalysisFromWorkflowRun()`.
   - `workflowArtifactsStore.ts` normalizes `currentRun` / `currentValidation`, updates a latest snapshot, and updates the store timestamp/state.

2. **Analysis-service orchestration**
   - `analysisService.ts` owns overview hydration, Top-table hydration, Pareto canvas rendering, heatmap canvas rendering, compare/baseline hydration, drilldown execution, local-stability execution, multiple request caches, and chart/table DOM hydration.
   - This file is the main responsiveness hot path for bundle 13.

3. **Panel reveal behavior**
   - `DrillWorkspacePanel.vue` and `HeatmapWorkspacePanel.vue` are Teleport/mount-helper-driven shells that call imperative analysis-service functions during mount and when panels become visible.
   - User-visible latency is not just backend compute latency here. It is the combination of:
     - artifact load/parse
     - store mutation
     - analysis refresh
     - table rebuild
     - heatmap repaint
     - drill chart/trade-table/stability render
     - and panel reveal logic that may replay more of the stack than necessary

The overall performance posture for this bundle should therefore be:

- eliminate unnecessary artifact reloads
- make unchanged payload application a true no-op
- stop recomputing sorted/filterable row state multiple times per refresh
- prevent drill/local-stability cache misses caused by UI-only request fields
- split “panel became visible” from “rerun the expensive pipeline”
- keep existing benchmark evidence in the right lane instead of over-claiming it

---

# 2. Findings

## Confirmed issue 1: latest-artifact sync replays heavy work even when nothing materially changed

- **Severity:** high
- **Type:** performance
- **Location:** `web/src/services/workflowArtifactsSync.ts` → `syncWorkflowArtifactsFromLegacyRuntime`, `web/src/stores/workflowArtifactsStore.ts` → `applyWorkflowArtifactsPayload`, `web/src/services/analysisService.ts` → `refreshAnalysisFromWorkflowRun`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The sync path always does:

  1. `list_run_artifacts`
  2. `load_run_artifact` for the latest path
  3. normalize/apply the payload
  4. full `refreshAnalysisFromWorkflowRun()`

  `applyWorkflowArtifactsPayload()` currently returns `true` for any object payload, not “changed vs unchanged”. That means an unchanged latest artifact still updates store state and triggers a full analysis refresh.

  This is wasted work on exactly the bundle-13 boundary the review map called out: repeated artifact load/parse work, redundant recompute, and workflow-artifact sync replaying more work than needed.

- **Recommended change:**  
  Introduce a two-stage guard:

  1. **Pre-load identity guard** in `workflowArtifactsSync.ts` so an unchanged latest artifact does not call `load_run_artifact` at all when it is safe to skip.
  2. **Post-normalization change detection** in `workflowArtifactsStore.ts` so unchanged normalized payloads do not update the store timestamp or trigger analysis refresh.

### Why this is the highest-value fix

This removes wasted work at the earliest possible point in the pipeline. It saves:

- artifact file load
- artifact parse/normalization
- store mutation
- timestamp churn
- full overview/top/pareto/heatmap refresh

### Important edge case

A same-path/same-ts artifact may be rewritten in place to add validation after the original run artifact landed. Because the provided files do not show a backend-provided artifact digest or `file_modified_at`, phase 1 should be **conservative**:

- skip pre-load only when:
  - latest path and ts match the already-loaded artifact, **and**
  - local validation is already present

That avoids suppressing a possible “validation arrived later in the same artifact file” case.

### Code edits: `workflowArtifactsStore.ts`

Add a richer apply result, stable signatures, and a richer latest snapshot.

```ts
export interface WorkflowArtifactsApplyResult {
  applied: boolean;
  runChanged: boolean;
  validationChanged: boolean;
  sourceChanged: boolean;
}

interface WorkflowRunSnapshot {
  ts: string;
  artifactPath: string;
  req: unknown;
  res: unknown;
}

let latestWorkflowRunSnapshot: WorkflowRunSnapshot | null = null;
let latestWorkflowRunSignature = "";
let latestWorkflowValidationSignature = "";

function stableStringify(value: unknown): string {
  if (value == null) return "null";
  if (typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(",")}]`;
  }
  const row = value as Record<string, unknown>;
  const keys = Object.keys(row).sort();
  return `{${keys
    .map((key) => `${JSON.stringify(key)}:${stableStringify(row[key])}`)
    .join(",")}}`;
}

function workflowRunSignature(run: WorkflowRunArtifact | null): string {
  if (!run) return "null";
  return stableStringify({
    ts: run.ts,
    artifactPath: run.artifactPath,
    reportXlsxPath: run.reportXlsxPath,
    csvPath: run.csvPath,
    datasetMeta: run.datasetMeta,
    mlSignal: run.mlSignal,
    tags: run.tags,
    req: run.req,
    res: run.res,
  });
}

function workflowValidationSignature(
  validation: WorkflowValidationArtifact | null,
): string {
  if (!validation) return "null";
  return stableStringify({
    ts: validation.ts,
    req: validation.req,
    res: validation.res,
  });
}

export function readLatestWorkflowRunIdentitySnapshot(): {
  ts: string;
  artifactPath: string;
  hasValidation: boolean;
} | null {
  if (!latestWorkflowRunSnapshot) return null;
  return {
    ts: latestWorkflowRunSnapshot.ts,
    artifactPath: latestWorkflowRunSnapshot.artifactPath,
    hasValidation: latestWorkflowValidationSignature !== "null",
  };
}
```

Now change `applyNormalizedArtifacts()` so it only mutates the live store and updates `updatedAtUnixMs` when the normalized run or validation actually changed.

```ts
function applyNormalizedArtifacts(
  run: WorkflowRunArtifact | null,
  validation: WorkflowValidationArtifact | null,
  source: WorkflowArtifactsSource,
  targetStore?: WorkflowArtifactsState,
): WorkflowArtifactsApplyResult {
  const nextRunSignature = workflowRunSignature(run);
  const nextValidationSignature = workflowValidationSignature(validation);

  const runChanged = nextRunSignature !== latestWorkflowRunSignature;
  const validationChanged =
    nextValidationSignature !== latestWorkflowValidationSignature;

  if (runChanged || validationChanged) {
    latestWorkflowRunSignature = nextRunSignature;
    latestWorkflowValidationSignature = nextValidationSignature;
    latestWorkflowRunSnapshot = run
      ? {
          ts: run.ts,
          artifactPath: run.artifactPath,
          req: run.req ?? null,
          res: run.res ?? null,
        }
      : null;
  }

  const store =
    targetStore ??
    (() => {
      const pinia = resolveRuntimePinia();
      return pinia ? useWorkflowArtifactsStore(pinia) : null;
    })();

  if (!store) {
    return {
      applied: runChanged || validationChanged,
      runChanged,
      validationChanged,
      sourceChanged: false,
    };
  }

  const sourceChanged = store.source !== source;

  // Do not churn updatedAtUnixMs for source-only changes.
  if (!runChanged && !validationChanged) {
    if (sourceChanged) {
      store.source = source;
    }
    return {
      applied: sourceChanged,
      runChanged: false,
      validationChanged: false,
      sourceChanged,
    };
  }

  store.currentRun = run;
  store.currentValidation = validation;
  store.source = source;
  store.updatedAtUnixMs = Date.now();

  return {
    applied: true,
    runChanged,
    validationChanged,
    sourceChanged,
  };
}
```

Then change `applyWorkflowArtifactsPayload()` to return the richer result.

```ts
export function applyWorkflowArtifactsPayload(
  payload: unknown,
  source: WorkflowArtifactsSource,
  targetStore?: WorkflowArtifactsState,
): WorkflowArtifactsApplyResult {
  if (!payload || typeof payload !== "object") {
    return {
      applied: false,
      runChanged: false,
      validationChanged: false,
      sourceChanged: false,
    };
  }
  const row = payload as WorkflowArtifactsPayload;
  const run = normalizeRunArtifact(row.currentRun);
  const validation = normalizeValidationArtifact(row.currentValidation);
  return applyNormalizedArtifacts(run, validation, source, targetStore);
}
```

### Code edits: `workflowArtifactsSync.ts`

Use the identity snapshot to skip redundant artifact file loads, then only refresh analysis when run or validation actually changed.

```ts
import { invokeCommand } from '../tauri/invoke'
import {
  applyWorkflowArtifactsPayload,
  readLatestWorkflowRunIdentitySnapshot,
} from '../stores/workflowArtifactsStore'
import { refreshAnalysisFromWorkflowRun } from './analysisService'

export async function syncWorkflowArtifactsFromLegacyRuntime(
  source: WorkflowArtifactsSource = 'vue',
): Promise<boolean> {
  try {
    const rowsRaw = (await invokeCommand('list_run_artifacts')) as unknown
    const rows = Array.isArray(rowsRaw) ? rowsRaw : []
    if (rows.length === 0) return false

    const latest = asRecord(rows[0])
    const latestPath = normalizeText(latest?.path)
    const latestTs = normalizeText(latest?.ts)
    if (!latestPath) return false

    const current = readLatestWorkflowRunIdentitySnapshot()
    const sameLatestArtifact =
      !!current &&
      current.artifactPath === latestPath &&
      current.ts === latestTs

    // Conservative phase-1 skip: only skip if validation is already present locally.
    if (sameLatestArtifact && current?.hasValidation) {
      return false
    }

    const artifactRaw = (await invokeCommand('load_run_artifact', {
      path: latestPath,
    })) as unknown
    const artifact = asRecord(artifactRaw)
    if (!artifact) return false

    const validation = asRecord(artifact.validation)
    const payload: Record<string, unknown> = {
      currentRun: {
        ts: normalizeText(artifact.ts ?? latest?.ts),
        artifact_path: latestPath,
        report_xlsx_path: normalizeText(
          artifact.report_xlsx_path ?? latest?.report_xlsx_path,
        ),
        csv_path: normalizeText(artifact.csv_path ?? latest?.csv_path),
        dataset_meta: asRecord(artifact.dataset_meta),
        tags: normalizeTags(artifact.tags ?? latest?.tags),
        req: artifact.req ?? null,
        res: artifact.res ?? null,
      },
      currentValidation: validation
        ? {
            ts: normalizeText(artifact.ts ?? latest?.ts),
            req: validation.req ?? null,
            res: validation.res ?? null,
          }
        : null,
    }

    const applied = applyWorkflowArtifactsPayload(payload, source)

    if (applied.runChanged || applied.validationChanged) {
      refreshAnalysisFromWorkflowRun()
    }

    return applied.applied
  } catch {
    return false
  }
}
```

### Expected impact

- unchanged latest artifact sync becomes a real no-op
- `updatedAtUnixMs` stops churning on identical payloads
- analysis refresh stops replaying on identical payloads
- reduced unnecessary heatmap/Top-table rebuilds
- reduced user-visible “everything flickered/updated but nothing changed” feel

---

## Confirmed issue 2: Top-table refresh duplicates work and also loses score-context fidelity on one path

- **Severity:** high
- **Type:** performance / correctness
- **Location:** `web/src/services/analysisService.ts` → `refreshTopTableFromWorkflowRun`, `buildTopRowModelsForCurrentFilters`, `refreshAnalysisFromWorkflowRun`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  `refreshAnalysisFromWorkflowRun()` already calls `refreshOverviewFromWorkflowRunStore()`, but `refreshTopTableFromWorkflowRun()` calls it again. That duplicates overview work.

  `refreshTopTableFromWorkflowRun()` also:

  - collects rows
  - sorts rows
  - then calls `buildTopRowModelsForCurrentFilters()`, which collects and sorts again

  So one refresh is paying for multiple collection/sort passes.

  There is also a correctness issue: the explicit `sortRows()` call in `refreshTopTableFromWorkflowRun()` does **not** pass `scoreObservationContextOf(res)`, while `buildTopRowModelsForCurrentFilters()` does. That means score-based ordering can drift for score modes that depend on observation context (for example the effective-sample-LCB style path).

- **Recommended change:**  
  Refactor the top-table refresh so it computes `scoreContext`, `collected`, `sorted`, and `filtered` exactly once and uses those results everywhere.

### Code edits: `analysisService.ts`

#### Step 1: make `cacheSet()` generic while you are in this area

This is not the core fix, but it makes the later cache work cleaner.

```ts
function cacheSet<T>(
  cache: Map<string, T>,
  key: string,
  value: T,
  max = 96,
): void {
  if (!key) return;
  if (cache.has(key)) cache.delete(key);
  cache.set(key, value);
  while (cache.size > Math.max(8, Math.floor(Number(max || 96)))) {
    const oldest = cache.keys().next();
    if (oldest.done) break;
    cache.delete(oldest.value);
  }
}
```

#### Step 2: let `buildTopRowModelsForCurrentFilters()` accept pre-sorted rows

```ts
function buildTopRowModelsForCurrentFilters(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
  preSorted?: TopRowModel[],
): TopRowModel[] {
  const filters = readFilters(store);
  if (preSorted) {
    return applyTopFilters(preSorted, filters);
  }

  const objective = objectiveOf(req);
  const scoreMode = scoreModeOf(req);
  const scoreContext = scoreObservationContextOf(res);
  const sort = readSortState(store);
  const collected = collectTopRows(res, {
    allowBestFallback: requestedTopK(req) !== 0,
  });
  const sorted = sortRows(
    collected,
    sort.key,
    sort.dir,
    objective,
    scoreMode,
    scoreContext,
  );

  return applyTopFilters(sorted, filters);
}
```

#### Step 3: remove the nested overview refresh and use one sort/filter pass

Delete this line from `refreshTopTableFromWorkflowRun()`:

```ts
refreshOverviewFromWorkflowRunStore(store, req, res);
```

Then replace the top-table refresh body with a single-pass flow:

```ts
export function refreshTopTableFromWorkflowRun(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run) return false;

  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;

  const objective = objectiveOf(req);
  const scoreMode = scoreModeOf(req);
  const scoreContext = scoreObservationContextOf(res);
  const sort = readSortState(store);
  const topK = requestedTopK(req);

  const collected = collectTopRows(res, { allowBestFallback: topK !== 0 });
  const sorted = sortRows(
    collected,
    sort.key,
    sort.dir,
    objective,
    scoreMode,
    scoreContext,
  );

  const filters = readFilters(store);
  const filtered = applyTopFilters(sorted, filters);

  topRowsModelCache = filtered;
  topRowsAllModelCache = sorted;

  const selectedOldRows = readTopRows(store);
  const previousSelected = selectedOldRows.find((row) => !!row?.selected);
  const previousSelectedId = text(previousSelected?.rowId);
  const selectedOld = selectedOldRows.findIndex((row) => !!row?.selected);

  const selectedByIdIndex = previousSelectedId
    ? filtered.findIndex((row) => topRowIdFor(row) === previousSelectedId)
    : -1;

  const selectedIndex = filtered.length
    ? selectedByIdIndex >= 0
      ? selectedByIdIndex
      : selectedOld >= 0
      ? Math.min(selectedOld, filtered.length - 1)
      : 0
    : -1;

  // ... keep the existing row-building logic ...
}
```

#### Step 4: keep `refreshAnalysisFromWorkflowRun()` responsible for overview only once

```ts
export function refreshAnalysisFromWorkflowRun(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run) return false;

  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;

  if (kindFromReq(req) === "MlSignal") {
    // existing MlSignal branch unchanged
    // ...
    markAnalysisVue(store);
    return true;
  }

  refreshOverviewFromWorkflowRunStore(store, req, res);
  refreshTopTableFromWorkflowRun();
  refreshParetoFromWorkflowRun();
  refreshHeatmapFromWorkflowRun();
  markAnalysisVue(store);
  return true;
}
```

### Expected impact

- one overview refresh instead of two
- one row collection/sort pass instead of two
- consistent score-based sort order because the same `scoreContext` is used everywhere
- less JS work on every artifact hydration

---

## Confirmed issue 3: local-stability request keys are polluted by drilldown-only UI fields (`bins`)

- **Severity:** high
- **Type:** performance / correctness
- **Location:** `web/src/services/analysisService.ts` → `buildDrilldownInvocation`, `buildLocalStabilityInvocation`, `requestCacheKey`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The drilldown request intentionally includes `bins: desiredDrillBins()` because chart resolution is viewport-sensitive.

  `buildLocalStabilityInvocation()` currently starts from `...invocation.request`, which means the local-stability request inherits all drilldown request fields, including `bins`.

  But the local-stability request contract does not use `bins`. Because `requestCacheKey()` hashes the whole request object, a resize or different device-pixel-ratio can produce a different local-stability cache key for the **same logical stability request**.

  That means you can miss stability cache hits just because the UI width changed.

- **Recommended change:**  
  Build a local-stability request from an explicit allowlist, not from `...invocation.request`.

### Code edits: `analysisService.ts`

Add a shared identity helper first; it will also help the drill cache-restore work later.

```ts
function workflowRunIdentity(run: WorkflowRunState | null): string {
  if (!run) return "";
  const ts = text(run.ts);
  const artifactPath = text(run.artifactPath ?? run.artifact_path);
  if (ts || artifactPath) return `${ts}|${artifactPath}`;

  const req = asRecord(run.req);
  const res = asRecord(run.res);
  return stableStringify({
    req,
    res: {
      best_params: asRecord(res.best_params),
      followup_cache_key: text(
        res.followup_cache_key ?? res.followupCacheKey,
      ),
    },
  });
}
```

Then replace `buildLocalStabilityInvocation()` with an explicit request builder:

```ts
function buildLocalStabilityInvocation(
  paramsRaw: Record<string, unknown>,
  invocation: DrillInvocation,
): StabilityInvocation | null {
  const runReq = invocation.runReq;
  const objective = text(runReq.objective) || "Sharpe";

  const sharedBase: Record<string, unknown> = {
    data_id: invocation.request.data_id,
    backend_hint: invocation.request.backend_hint,
    followup_cache_key: invocation.request.followup_cache_key ?? null,
    params: invocation.request.params,
    ma_source: invocation.request.ma_source,
    fast_ma_params: invocation.request.fast_ma_params ?? null,
    slow_ma_params: invocation.request.slow_ma_params ?? null,
    objective,
    radius: 1,
    step: 1,
  };

  if (invocation.command === "compute_double_ma_drilldown") {
    return {
      command: "compute_double_ma_local_stability",
      request: {
        ...sharedBase,
        strategy: invocation.request.strategy ?? {},
        halftrend_filter: invocation.request.halftrend_filter ?? null,
      },
    };
  }

  const kind = normalizeStrategyKindRaw(
    runReq.strategy_kind ?? runReq.strategyKind,
  );
  const params = asRecord(paramsRaw);

  if (kind === "CompositeMo") {
    return {
      command: "compute_composite_cpu_local_stability",
      request: {
        ...sharedBase,
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        signal_mode:
          params.signal_mode ||
          params.signalMode ||
          runReq.signal_mode ||
          runReq.signalMode ||
          "hard_gate",
        strategy:
          params.strategy || runReq.strategy || runReq.base_strategy || {},
      },
    };
  }

  return {
    command: "compute_composite_cpu_local_stability",
    request: {
      ...sharedBase,
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
      asr: runReq.asr || null,
      vemr: runReq.vemr || null,
      signal_mode: runReq.signal_mode || runReq.signalMode || "hard_gate",
    },
  };
}
```

The key point is: **no `...invocation.request` spread**.

### Optional test-only shim

Because this helper is not exported, expose a minimal test shim if you want clean unit coverage.

```ts
export const __analysisServiceTestOnly = {
  buildDrilldownInvocation,
  buildLocalStabilityInvocation,
  requestCacheKey,
};
```

### Expected impact

- stability cache survives viewport-only drill chart width changes
- fewer redundant local-stability command invocations
- cleaner separation between drilldown DTO shape and local-stability DTO shape

---

## Confirmed issue 4: baseline artifact cache and price-bar cache are unbounded

- **Severity:** medium
- **Type:** performance / maintainability
- **Location:** `web/src/services/analysisService.ts` → `baselineArtifactCache`, `loadBaselineFromDisk`, `drillPriceBarsCache`, `fetchDrillPriceBars`
- **Timing boundary affected:** app-path / long-session memory
- **Why it matters:**  
  `drillCurvesCache` and `stabilityCache` are bounded through `cacheSet(..., 96)`, but:

  - `baselineArtifactCache` is an unbounded `Map<string, BaselineRun>`
  - `drillPriceBarsCache` is an unbounded `Map<string, DrillPriceBar[]>`

  Long sessions with multiple baseline comparisons or dataset switches can keep these structures growing without any eviction discipline.

- **Recommended change:**  
  Use the same bounded cache policy already present in the file.

### Code edits: `analysisService.ts`

Add explicit cache limits near the cache declarations.

```ts
const DRILL_CURVES_CACHE_MAX = 96;
const STABILITY_CACHE_MAX = 96;
const BASELINE_ARTIFACT_CACHE_MAX = 16;
const DRILL_PRICE_BARS_CACHE_MAX = 8;

const baselineArtifactCache = new Map<string, BaselineRun>();
const drillCurvesCache = new Map<string, unknown>();
const stabilityCache = new Map<string, unknown>();
const drillPriceBarsCache = new Map<string, DrillPriceBar[]>();
```

Then update the read-through sites.

```ts
async function loadBaselineFromDisk(path: string): Promise<BaselineRun | null> {
  const normalizedPath = text(path);
  if (!normalizedPath) return null;

  const cached = baselineArtifactCache.get(normalizedPath);
  if (cached) return cached;

  const raw = (await invokeCommand("load_run_artifact", {
    path: normalizedPath,
  })) as RunArtifactPayload;

  const row = asRecord(raw);
  const run = asRecord(row.run);
  const req = asRecord(run.req);
  const res = asRecord(run.res);

  if (!Object.keys(req).length || !Object.keys(res).length) return null;

  const baseline: BaselineRun = {
    source: "disk",
    ts: text(row.ts) || text(run.ts),
    req,
    res,
  };

  cacheSet(
    baselineArtifactCache,
    normalizedPath,
    baseline,
    BASELINE_ARTIFACT_CACHE_MAX,
  );

  return baseline;
}
```

```ts
async function fetchDrillPriceBars(dataId: string): Promise<DrillPriceBar[]> {
  const id = text(dataId);
  if (!id) return [];

  const cached = drillPriceBarsCache.get(id);
  if (cached) return cached;

  let raw: unknown;
  try {
    raw = await invokeCommand("get_price_bars", {
      dataId: id,
      maxPoints: 8000,
    });
  } catch (firstError) {
    raw = await invokeCommand("get_price_bars", {
      data_id: id,
      max_points: 8000,
    }).catch(() => {
      throw firstError;
    });
  }

  const bars = normalizeDrillPriceBars(raw);

  cacheSet(
    drillPriceBarsCache,
    id,
    bars,
    DRILL_PRICE_BARS_CACHE_MAX,
  );

  return bars;
}
```

Also update the existing drill/stability inserts for clarity:

```ts
if (!cachedDrill) {
  cacheSet(drillCurvesCache, drillKey, curves, DRILL_CURVES_CACHE_MAX);
}

if (!cachedStability) {
  cacheSet(stabilityCache, stabilityKey, stabilityRes, STABILITY_CACHE_MAX);
}
```

### Expected impact

- bounded long-session memory growth
- more predictable cache behavior
- consistent cache policy across the file

---

## Confirmed issue 5: heatmap reveal path repaints more than necessary

- **Severity:** medium
- **Type:** performance
- **Location:** `web/src/components/HeatmapWorkspacePanel.vue`, `web/src/services/analysisService.ts` → `refreshHeatmapFromWorkflowRun`, `renderHeatmapCanvas`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The heatmap panel currently does a full `refreshHeatmapFromWorkflowRun()` on mount and whenever the heatmap tab becomes visible. That full refresh resets options/status state and repaints the base canvas even when:

  - the run is unchanged
  - the heatmap data is unchanged
  - only panel visibility changed

  The expensive part is the base heatmap fill loop over every cell. Repeating that on every reveal is avoidable.

- **Recommended change:**  
  Split heatmap work into two layers:

  1. **full heatmap refresh** when the underlying run changed
  2. **cheap visible-panel restore** when the panel simply became visible again

### Code edits: `analysisService.ts`

Add a render-state cache for the heatmap.

```ts
interface HeatmapRenderState {
  runIdentity: string;
  baseSignature: string;
  model: HeatmapModel;
  range: { min: number; max: number; label: string };
  palette: { negative: string; zero: string; positive: string };
  markers: Array<{ col: number; drawRow: number; label: string }>;
  cellW: number;
  cellH: number;
}

let heatmapRenderState: HeatmapRenderState | null = null;
```

Add a stable base-signature helper:

```ts
function heatmapBaseSignature(
  req: Record<string, unknown>,
  model: HeatmapModel,
  canvas: HTMLCanvasElement,
): string {
  return stableStringify({
    runIdentity: workflowRunIdentity(readWorkflowRun()),
    objective: objectiveOf(req),
    scoreMode: scoreModeOf(req),
    binsFast: model.binsFast,
    binsSlow: model.binsSlow,
    fastMin: model.fastMin,
    fastMax: model.fastMax,
    slowMin: model.slowMin,
    slowMax: model.slowMax,
    canvasWidth: canvas.width,
    canvasHeight: canvas.height,
  });
}
```

Extract the overlay painter so it can be reused without repainting the base heatmap.

```ts
function paintHeatmapOverlay(
  overlayCtx: CanvasRenderingContext2D,
  width: number,
  height: number,
  state: HeatmapRenderState,
  showMarkers: boolean,
  showCrosshair: boolean,
): void {
  overlayCtx.clearRect(0, 0, width, height);

  if (showMarkers) {
    for (const marker of state.markers) {
      const cx = (marker.col + 0.5) * state.cellW;
      const cy = (marker.drawRow + 0.5) * state.cellH;

      overlayCtx.fillStyle = "rgba(248, 250, 252, 0.95)";
      overlayCtx.beginPath();
      overlayCtx.arc(
        cx,
        cy,
        Math.max(3, Math.min(state.cellW, state.cellH) * 0.18),
        0,
        Math.PI * 2,
      );
      overlayCtx.fill();

      overlayCtx.strokeStyle = "rgba(15, 23, 42, 0.95)";
      overlayCtx.lineWidth = 1;
      overlayCtx.stroke();

      overlayCtx.fillStyle = "rgba(15, 23, 42, 0.95)";
      overlayCtx.font = "11px ui-monospace, monospace";
      overlayCtx.fillText(marker.label, cx + 5, cy - 5);
    }
  }

  if (!heatmapHoverCell) return;

  const hoverX = heatmapHoverCell.col * state.cellW;
  const hoverY = heatmapHoverCell.row * state.cellH;

  overlayCtx.strokeStyle = "rgba(248, 250, 252, 0.95)";
  overlayCtx.lineWidth = 1.5;
  overlayCtx.strokeRect(
    hoverX + 0.5,
    hoverY + 0.5,
    Math.max(0, state.cellW - 1),
    Math.max(0, state.cellH - 1),
  );

  if (!showCrosshair) return;

  overlayCtx.beginPath();
  overlayCtx.moveTo(hoverX + state.cellW / 2, 0);
  overlayCtx.lineTo(hoverX + state.cellW / 2, height);
  overlayCtx.moveTo(0, hoverY + state.cellH / 2);
  overlayCtx.lineTo(width, hoverY + state.cellH / 2);
  overlayCtx.strokeStyle = "rgba(248, 250, 252, 0.28)";
  overlayCtx.stroke();
}
```

Now modify `renderHeatmapCanvas()` to skip the base repaint when the signature is unchanged.

```ts
function renderHeatmapCanvas(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): boolean {
  const model = readHeatmapModel(res);
  const wrap = byIds<HTMLElement>(["heatmapWrapVue"]);
  const hint = byIds<HTMLElement>(["heatmapEmptyHintVue"]);
  const meta = byIds<HTMLElement>(["heatmapMetaVue"]);
  const canvas = byIds<HTMLCanvasElement>(["heatmapCanvasVue"]);
  const overlay = byIds<HTMLCanvasElement>(["heatmapOverlayVue"]);
  const tooltip = byIds<HTMLElement>(["heatmapTooltipVue"]);
  if (!wrap || !hint || !meta || !canvas || !overlay || !tooltip) return false;

  // ... existing empty-state branches unchanged ...

  const finite = finiteHeatmapValues(model);
  if (!finite.length) {
    // existing no-finite-values branch unchanged
    return true;
  }

  const width = canvas.width;
  const height = canvas.height;
  const ctx = canvas.getContext("2d");
  const overlayCtx = overlay.getContext("2d");
  if (!ctx || !overlayCtx) return false;

  const range = resolveHeatmapRange(finite);
  const palette = heatmapPalette();
  const markers = heatmapMarkerRows(res, model);
  const cellW = width / Math.max(1, model.binsFast);
  const cellH = height / Math.max(1, model.binsSlow);
  const showMarkers = bool(store.heatmapShowTop, true);
  const showCrosshair = bool(store.heatmapShowCrosshair, true);

  const baseSignature = heatmapBaseSignature(req, model, canvas);
  const canReuseBase =
    !!heatmapRenderState &&
    heatmapRenderState.baseSignature === baseSignature &&
    heatmapRenderState.runIdentity === workflowRunIdentity(readWorkflowRun());

  if (!canReuseBase) {
    ctx.clearRect(0, 0, width, height);

    for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
      const drawRow = model.binsSlow - 1 - logicalRow;
      for (let col = 0; col < model.binsFast; col += 1) {
        const value = model.values[heatmapIndex(model, logicalRow, col)];
        const x = col * cellW;
        const y = drawRow * cellH;
        if (typeof value !== "number" || !Number.isFinite(value)) {
          ctx.fillStyle = "rgba(82, 96, 118, 0.12)";
          ctx.fillRect(x, y, cellW, cellH);
          continue;
        }
        ctx.fillStyle = sampleHeatmapColor(
          value,
          Math.max(Math.abs(range.min), Math.abs(range.max)),
          palette,
        );
        ctx.fillRect(x, y, cellW, cellH);
      }
    }

    renderHeatmapLegend(range.min, range.max, palette);
  }

  heatmapRenderState = {
    runIdentity: workflowRunIdentity(readWorkflowRun()),
    baseSignature,
    model,
    range,
    palette,
    markers,
    cellW,
    cellH,
  };

  paintHeatmapOverlay(
    overlayCtx,
    width,
    height,
    heatmapRenderState,
    showMarkers,
    showCrosshair,
  );

  // keep the existing tooltip / onmousemove / onclick logic,
  // but make it read from `heatmapRenderState`
  // instead of recomputing marker/cell geometry each time.
  // ...
}
```

Add a cheap visible-panel restore helper:

```ts
export function ensureHeatmapVisibleRender(): boolean {
  const store = readAnalysisStore();
  const overlay = byIds<HTMLCanvasElement>(["heatmapOverlayVue"]);
  const base = byIds<HTMLCanvasElement>(["heatmapCanvasVue"]);
  if (!store || !overlay || !base) return false;

  const overlayCtx = overlay.getContext("2d");
  if (!overlayCtx) return false;

  const state = heatmapRenderState;
  const currentRunIdentity = workflowRunIdentity(readWorkflowRun());

  heatmapHoverCell = null;
  hideHeatmapTooltip();

  if (
    state &&
    state.runIdentity === currentRunIdentity &&
    base.width > 0 &&
    base.height > 0
  ) {
    paintHeatmapOverlay(
      overlayCtx,
      overlay.width,
      overlay.height,
      state,
      bool(store.heatmapShowTop, true),
      bool(store.heatmapShowCrosshair, true),
    );
    return true;
  }

  const run = readWorkflowRun();
  if (!run) return false;

  return renderHeatmapCanvas(store, asRecord(run.req), asRecord(run.res));
}
```

### Code edits: `HeatmapWorkspacePanel.vue`

Replace the full-refresh-on-reveal behavior with the cheaper visible-render helper.

```vue
<script setup lang="ts">
import { computed, nextTick, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { ensureHeatmapVisibleRender } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  heatmapMeta,
  canHeatmapExportPng,
  heatmapStatusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#heatmapVueShellMount')
const visible = computed(() => String(resultsTab.value || '').trim() === 'heatmap')
const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

async function hydrateVisibleHeatmap(): Promise<void> {
  await nextTick()
  await nextTick()
  ensureHeatmapVisibleRender()
}

watch(
  () => [mountReady.value, visible.value] as const,
  async ([ready, isVisible]) => {
    if (!ready || !isVisible) return
    await hydrateVisibleHeatmap()
  },
  { immediate: true },
)

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}
</script>
```

### Expected impact

- panel reveal stops forcing a full state reset/rebuild
- unchanged heatmap run can reuse the already-painted base canvas
- reveal cost becomes overlay-only in the warm case

---

## Confirmed issue 6: drill panel rerender path re-enters selection/pipeline work instead of restoring the current rendered view

- **Severity:** high
- **Type:** performance / responsiveness
- **Location:** `web/src/components/DrillWorkspacePanel.vue`, `web/src/services/analysisService.ts` → `rerenderDrill`, `selectTopRow`, `runVueDrillPipeline`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  `DrillWorkspacePanel.vue` currently nudges `analysisStore.rerenderDrill()` from `onMounted()`. `rerenderDrill()` then routes back through `selectTopRow()`, which routes back through `runVueDrillPipeline()`.

  Even if command-level drill/stability results are cached, that path still:

  - touches selection state
  - can fetch bars
  - rebuilds chart/table/stability DOM
  - and can trigger more work than a simple “make the current rendered drill view visible again”

- **Recommended change:**  
  Add a drill render-state cache and split the service into:

  1. **full pipeline execution** when run/selection actually changed
  2. **cheap restore** when the panel just became visible again for the same selected row and same run

### Code edits: `analysisService.ts`

Add a render-state object:

```ts
interface DrillRenderState {
  runIdentity: string;
  rowId: string;
  row: TopRowModel;
  params: Record<string, unknown>;
  metrics: Record<string, unknown>;
  curves: unknown;
  stability: unknown | null;
  bars: DrillPriceBar[] | null;
}

let drillRenderState: DrillRenderState | null = null;
```

Add a restore helper:

```ts
async function restoreDrillViewFromCache(selectedRowId: string): Promise<boolean> {
  const state = drillRenderState;
  if (!state || !selectedRowId) return false;

  const currentRunIdentity = workflowRunIdentity(readWorkflowRun());
  if (state.runIdentity !== currentRunIdentity) return false;
  if (state.rowId !== selectedRowId) return false;

  const store = readAnalysisStore();
  const seq = (drillSeq + 1) >>> 0;
  drillSeq = seq;

  renderDrillDomFallback(
    state.curves,
    state.params,
    state.metrics,
    state.row,
    state.bars,
  );
  void renderDrillTvChartDom(state.curves, state.bars, seq).catch(() => {});

  if (state.stability) {
    renderStabilityDom(state.stability, store);
  }

  if (store) {
    store.drillStatus = "Done (view cached).";
    store.stabilityStatus = state.stability ? "Done (view cached)." : "-";
    markAnalysisVue(store);
  }

  return true;
}
```

At the end of a successful `runVueDrillPipeline()`, capture the rendered state:

```ts
drillRenderState = {
  runIdentity: workflowRunIdentity(run),
  rowId: topRowIdFor(row),
  row,
  params,
  metrics,
  curves,
  stability: stabilityRes ?? null,
  bars,
};
```

Also clear it on hard failures if you do not want stale restore attempts:

```ts
} catch (error) {
  if (seq !== drillSeq) return true;
  drillRenderState = null;
  const message = `Drilldown failed: ${String(error)}`;
  // existing error handling...
}
```

Now change `rerenderDrill()` so it tries the restore path first.

```ts
export async function rerenderDrill(): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;

  const rows = readTopRows(store);
  if (!rows.length) return;

  const selectedIndex = rows.findIndex((row) => !!row?.selected);
  const boundedIndex = selectedIndex >= 0 ? selectedIndex : 0;
  const selectedRowId = text(rows[boundedIndex]?.rowId);

  if (await restoreDrillViewFromCache(selectedRowId)) {
    setStatus(store, "Drilldown ready.");
    return;
  }

  await selectTopRow(boundedIndex);
}
```

### Code edits: `DrillWorkspacePanel.vue`

Remove the `setTimeout(...rerenderDrill...)` mount nudge and switch to a visibility-aware watcher. That lets you restore on first visible mount and on later re-entry without ad hoc timing.

```vue
<script setup lang="ts">
import { computed, nextTick, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#drillVueShellMount')
const visible = computed(() => String(resultsTab.value || '').trim() === 'drill')
const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

async function hydrateVisibleDrill(): Promise<void> {
  await nextTick()
  await analysisStore.rerenderDrill()
}

watch(
  () => [mountReady.value, visible.value] as const,
  async ([ready, isVisible]) => {
    if (!ready || !isVisible) return
    await hydrateVisibleDrill()
  },
  { immediate: true },
)

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}
</script>
```

### Expected impact

- warm drill-panel reveal stops re-entering the full pipeline for the same selected row
- chart/trade/stability DOM can be restored from already available render state
- drill responsiveness feels much more stable on repeated tab switches

---

## Confirmed issue 7: the current bundle-13 perf evidence does not measure the real app-path hotspot

- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `crates/app_core/benches/double_ma_followup_workflow.rs`, `src-tauri/src/bin/ta_workflow_regression.rs`
- **Timing boundary affected:** Criterion / workflow-regression
- **Why it matters:**  
  The current evidence is useful, but it is easy to over-interpret:

  - `double_ma_followup_workflow.rs` is a **Criterion engine benchmark** for follow-up compute reuse.
  - `ta_workflow_regression.rs` is a **workflow-regression harness** for optimize → validate → meta → holdout.

  Neither one measures:

  - `load_run_artifact`
  - workflow-artifact apply and sync
  - Top-table rebuild
  - heatmap canvas repaint
  - drill-panel restore/recompute behavior
  - user-visible tab reveal latency

- **Recommended change:**  
  Keep both files, but make their timing-boundary limits explicit so future readers do not misuse them as proof of bundle-13 UI-path improvements.

### Code edits: `double_ma_followup_workflow.rs`

The benchmark already prints metadata. Extend that metadata so the benchmark advertises its scope more clearly.

```rust
print_bench_metadata(
    &format!("double_ma_followup/{label}/50k"),
    &[
        ("backend", backend_label),
        ("candles_len", candles.close.len().to_string()),
        ("surface", "followup_workflow".to_string()),
        ("phases", "drilldown_then_local_stability".to_string()),
        (
            "timing_boundary_note",
            "engine_only_excludes_tauri_and_web_hydration".to_string(),
        ),
        (
            "bundle_13_role",
            "backend_companion_not_app_path_proof".to_string(),
        ),
    ],
);
```

This does not change the benchmark shape. It makes the benchmark harder to misread.

### Code edits: `ta_workflow_regression.rs`

Add an explicit exclusions field to the serialized summary.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct WorkflowSummary {
    schema_version: u32,
    timing_boundary: String,
    timing_boundary_note: String,
    timing_boundary_exclusions: String,
    build_profile: String,
    environment: EnvironmentFingerprint,
    candles_len: usize,
    train_assets: usize,
    holdout_assets: usize,
    artifacts_used: usize,
    groups: usize,
    top_meta_score: f64,
    robust_score_p10: f64,
    max_dd_p90: f64,
    holdout_score_p10: f64,
    holdout_max_dd_p90: f64,
    holdout_pass_rate: f64,
    runtime_ms: u64,
    stage: WorkflowStageBreakdown,
}

const fn workflow_timing_boundary_exclusions() -> &'static str {
    "excludes tauri analysis commands, workflow artifact sync, top-table hydration, heatmap rendering, and drill/local-stability panel hydration"
}
```

Set it in `Default` and in the final summary:

```rust
impl Default for WorkflowSummary {
    fn default() -> Self {
        Self {
            schema_version: workflow_schema_version(),
            timing_boundary: workflow_timing_boundary().to_string(),
            timing_boundary_note: workflow_timing_boundary_note().to_string(),
            timing_boundary_exclusions: workflow_timing_boundary_exclusions().to_string(),
            build_profile: build_profile().to_string(),
            environment: environment_fingerprint(),
            // ...
        }
    }
}
```

```rust
Ok(WorkflowSummary {
    schema_version: workflow_schema_version(),
    timing_boundary: workflow_timing_boundary().to_string(),
    timing_boundary_note: workflow_timing_boundary_note().to_string(),
    timing_boundary_exclusions: workflow_timing_boundary_exclusions().to_string(),
    build_profile: build_profile().to_string(),
    environment: environment_fingerprint(),
    // ...
})
```

Add one assertion in the legacy-deserialization test so the new field stays default-safe:

```rust
assert_eq!(
    parsed.timing_boundary_exclusions,
    super::workflow_timing_boundary_exclusions()
);
```

### Expected impact

- better perf-evidence hygiene
- reduced risk of claiming “bundle 13 is fixed” based only on engine or workflow harnesses
- clearer documentation in serialized perf artifacts

---

## Likely issue 1: heatmap panel mount logic can duplicate reveal work

- **Severity:** medium
- **Type:** performance / maintainability
- **Location:** `web/src/components/HeatmapWorkspacePanel.vue`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The current component uses `onMounted()` plus a `watch(visible)` reveal path. Depending on tab initialization order, mount readiness, and whether the panel is visible on first render, this can be more work than necessary.

- **Recommended change:**  
  Use one immediate watcher on `[mountReady, visible]` and route that watcher to the lighter `ensureHeatmapVisibleRender()` path described above.

This is already covered in the proposed component snippet under confirmed issue 5.

---

## Likely issue 2: drill chart layout restoration may still depend on hidden→visible resize semantics

- **Severity:** medium
- **Type:** responsiveness
- **Location:** `web/src/services/analysisService.ts` → `ensureDrillTvChartDom`, `renderDrillTvChartDom`; `web/src/components/DrillWorkspacePanel.vue`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  The lightweight-charts host uses resize observation, but hidden-to-visible layout changes are not always the cleanest trigger path in every shell composition. In practice, the cached drill-restore path will already improve things a lot. If the host still occasionally wakes up at the wrong size, you may need one explicit layout apply on reveal.

- **Recommended change:**  
  Only if you still see stale chart size after the cache-restore change, add a helper like this:

```ts
function applyCurrentDrillChartSize(): void {
  const chart = drillTvRuntime.chartRef.value;
  const host = drillTvRuntime.hostRef.value;
  if (!chart || !host || typeof chart.applyOptions !== "function") return;

  const width = Math.max(240, Math.floor(host.clientWidth || 820));
  const height = Math.max(220, Math.floor(host.clientHeight || 360));

  try {
    chart.applyOptions({ width, height });
  } catch {
    // no-op
  }
}
```

Then call it inside `restoreDrillViewFromCache()` before `renderDrillTvChartDom()`.

This is likely, not confirmed, because the provided files do not include the full shell composition around the Teleport host.

---

## Open question 1: can the latest artifact file be rewritten in place without `path`/`ts` changing?

- **Severity:** medium
- **Type:** correctness / benchmark-fidelity
- **Location:** `workflowArtifactsSync.ts` boundary; adjacent artifact-list command not provided
- **Timing boundary affected:** app-path
- **Why it matters:**  
  If validation or other artifact content can be appended in place while `path` and `ts` stay unchanged, then a pure same-path/same-ts sync skip could miss a meaningful update.

- **Recommended change:**  
  Phase 1 already avoids the riskiest version of this by skipping only when local validation is already present. If this rewrite-in-place pattern is common, the longer-term fix is to expose one more identity field from the artifact-list command, such as:

  - `file_modified_at`
  - `artifact_digest`
  - `has_validation`

  That adjacent command file was not provided, so this stays an open question rather than a confirmed code change in this document.

---

## Open question 2: should drilldown bins be bucketed to improve drilldown cache reuse across minor resizes?

- **Severity:** low
- **Type:** optimization opportunity
- **Location:** `analysisService.ts` → `desiredDrillBins`, `buildDrilldownInvocation`
- **Timing boundary affected:** app-path
- **Why it matters:**  
  Even after fixing the local-stability request shape, drilldown itself still uses viewport-derived `bins`. Small width changes can create distinct drilldown cache entries for nearly identical charts.

- **Recommended change:**  
  This is optional and slightly higher-risk because it changes the exact curve bin count. If you want it, bucket bins instead of using exact pixel width.

```ts
function bucketDrillBins(raw: number): number {
  const buckets = [256, 384, 512, 768, 1024, 1536, 2048, 3072, 4096];
  for (const bucket of buckets) {
    if (raw <= bucket) return bucket;
  }
  return 4096;
}

function desiredDrillBins(): number {
  if (typeof document === "undefined") return 1024;
  const canvas = byIds<HTMLCanvasElement>(["equityCanvasVue", "equityCanvas"]);
  if (!canvas) return 1024;

  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const w = Math.floor((rect.width || 0) * dpr);

  if (w < 64) return 1024;
  return bucketDrillBins(Math.max(64, Math.min(4096, w)));
}
```

Keep this behind a second pass. The first pass should land the no-op sync, cache-key cleanup, and render-state reuse.

---

# 3. Safe optimization opportunities

These are the safest changes in priority order.

## 3.1 Make unchanged artifact sync a true no-op

This is the biggest return per line of code because it cuts waste before downstream hydration even starts.

**Files:**

- `workflowArtifactsStore.ts`
- `workflowArtifactsSync.ts`

**Risk:** low  
**Value:** very high

## 3.2 Remove `bins` from the local-stability request shape

This is a clean cache-key fix with almost no product-risk because local stability does not need viewport bins.

**Files:**

- `analysisService.ts`

**Risk:** low  
**Value:** high

## 3.3 Collapse duplicate top-table collection/sort/overview work

This reduces repeated JS work and also fixes score-context drift on one sort path.

**Files:**

- `analysisService.ts`

**Risk:** low  
**Value:** high

## 3.4 Bound baseline and price-bar caches

This prevents long-session memory growth and aligns cache policy across the file.

**Files:**

- `analysisService.ts`

**Risk:** low  
**Value:** medium

## 3.5 Split “panel reveal” from “expensive recompute”

This improves repeat-tab responsiveness and makes the panels feel steadier.

**Files:**

- `analysisService.ts`
- `DrillWorkspacePanel.vue`
- `HeatmapWorkspacePanel.vue`

**Risk:** low-to-medium  
**Value:** high

## 3.6 Add timing-boundary clarity to supporting perf artifacts

This is evidence hygiene, not runtime speed, but it is still useful and safe.

**Files:**

- `double_ma_followup_workflow.rs`
- `ta_workflow_regression.rs`

**Risk:** low  
**Value:** medium

---

# 4. Higher-risk optimization opportunities

These are reasonable follow-ons, but they should land after the safe changes above.

## 4.1 Add backend-used / fallback / followup-cache-used metadata to analysis command responses

This would improve user-visible and perf-debug transparency for drilldown/local-stability, but it likely needs adjacent app-core response support that was not in the provided files.

**Files in current scope:** `commands_analysis.rs`  
**Likely adjacent files:** `crates/app_core/src/double_ma.rs` and/or related follow-up result types  
**Risk:** medium  
**Why it is higher-risk:** response-shape change

## 4.2 Bucket drilldown bins to improve drill cache reuse

This can reduce cache fragmentation across small width changes, but it changes the exact resolution of drilldown curves.

**Files:** `analysisService.ts`  
**Risk:** medium  
**Why it is higher-risk:** output shape changes slightly

## 4.3 Add a dedicated app-path analysis hydration harness

This is probably the correct future perf gate for bundle 13, but it is a new benchmark/test surface, not a simple local change.

**Candidate surfaces:**

- Tauri smoke binary for artifact load + analysis refresh
- frontend integration harness for heatmap/drill reveal latency
- Playwright or component-test timing around warm panel reveal

**Risk:** medium  
**Why it is higher-risk:** new infrastructure, not just local code cleanup

---

# 5. Benchmark and perf-gate review

## What the current benchmarks are good at

### `crates/app_core/benches/double_ma_followup_workflow.rs`

This is good evidence for:

- engine-level follow-up compute behavior
- cold vs cached follow-up compute
- `followup_cache_key` payoff on the core drilldown/local-stability functions

It is a **Criterion** boundary, not an app-path boundary.

### `src-tauri/src/bin/ta_workflow_regression.rs`

This is good evidence for:

- deterministic workflow wall time
- optimize / validate / meta-select / holdout stage budgets

It is a **workflow-regression** boundary, not an analysis hydration boundary.

## What the current benchmarks do not prove

They do **not** prove improvements to:

- artifact hydration
- workflow artifact sync no-op behavior
- Top-table row rebuild cost
- heatmap base repaint cost
- drill panel reveal cost
- price-bar baseline cache behavior
- panel switch responsiveness

## Practical guidance

After you land the phase-1 changes in this document:

1. Keep using the existing engine/workflow perf artifacts for their current scopes.
2. Do **not** cite them as proof that bundle-13 responsiveness is fixed.
3. Add one app-path-specific measurement later if you want bundle-13 regression protection.

A good minimal target is:

- “load already-known artifact → apply if changed → refresh analysis → reveal heatmap/drill warm”

That is the real bundle-13 user path.

---

# 6. Test review

There were no bundle-local tests provided for:

- `analysisService.ts`
- `workflowArtifactsSync.ts`
- `workflowArtifactsStore.ts`
- the drill/heatmap panel reveal logic

That is the main test gap for this bundle.

## Highest-value new tests

### 6.1 Store apply no-op test

**Target:** `workflowArtifactsStore.ts`

Verify:

- identical normalized payload returns:
  - `applied=false` or `applied` without `runChanged/validationChanged` depending on your final convention
  - `runChanged=false`
  - `validationChanged=false`
- `updatedAtUnixMs` does not change on identical data
- source-only changes do not force full “data changed” semantics

### 6.2 Validation-arrives-later test

**Target:** `workflowArtifactsStore.ts`

Verify:

- same run content with validation added later yields:
  - `runChanged=false`
  - `validationChanged=true`

### 6.3 Sync skip test

**Target:** `workflowArtifactsSync.ts`

Mock `invokeCommand()` and verify:

- when latest path/ts matches the loaded artifact and local validation already exists, `load_run_artifact` is not called
- when local validation is still absent, `load_run_artifact` is still attempted

### 6.4 Local-stability cache-key test

**Target:** `analysisService.ts`

Verify:

- drilldown requests with different `bins` generate different drill cache keys
- local-stability requests built from those drill invocations generate the **same** stability cache key

### 6.5 Top-table score-context consistency test

**Target:** `analysisService.ts`

Verify:

- `refreshTopTableFromWorkflowRun()` and `buildTopRowModelsForCurrentFilters()` use the same sort order when `scoreMode` depends on observation context

### 6.6 Drill restore test

**Target:** `analysisService.ts`

Verify:

- with same run identity and same selected row id, `rerenderDrill()` restores from cached render state instead of re-entering `runVueDrillPipeline()`

### 6.7 Heatmap visible-render test

**Target:** `analysisService.ts`

Verify:

- `ensureHeatmapVisibleRender()` repaints overlay only when the base signature is unchanged
- `renderHeatmapCanvas()` repaints the base canvas when the run identity or signature changes

## Suggested test skeletons

### Example: local-stability request-shape test

```ts
import { __analysisServiceTestOnly } from '../services/analysisService'

describe('buildLocalStabilityInvocation', () => {
  it('omits drill-only bins from the stability request and preserves cache identity', () => {
    const params = {
      fast_len: 10,
      slow_len: 30,
      fast_ma_type: 'sma',
      slow_ma_type: 'sma',
    }

    const runReq = {
      data_id: 'spy',
      strategy_kind: 'DoubleMa',
      objective: 'Sharpe',
      backend: { backend: 'CpuOnly' },
    }

    const runRes = {
      followup_cache_key: 'followup-key-1',
    }

    const drillA = __analysisServiceTestOnly.buildDrilldownInvocation(
      params,
      runReq,
      runRes,
    )!
    drillA.request.bins = 512

    const drillB = __analysisServiceTestOnly.buildDrilldownInvocation(
      params,
      runReq,
      runRes,
    )!
    drillB.request.bins = 2048

    const stabilityA = __analysisServiceTestOnly.buildLocalStabilityInvocation(
      params,
      drillA,
    )!
    const stabilityB = __analysisServiceTestOnly.buildLocalStabilityInvocation(
      params,
      drillB,
    )!

    expect(stabilityA.request).not.toHaveProperty('bins')
    expect(stabilityB.request).not.toHaveProperty('bins')

    const keyA = __analysisServiceTestOnly.requestCacheKey(
      stabilityA.command,
      stabilityA.request,
    )
    const keyB = __analysisServiceTestOnly.requestCacheKey(
      stabilityB.command,
      stabilityB.request,
    )

    expect(keyA).toBe(keyB)
  })
})
```

### Example: workflow artifact apply test

```ts
import {
  applyWorkflowArtifactsPayload,
} from '../stores/workflowArtifactsStore'

describe('applyWorkflowArtifactsPayload', () => {
  it('returns no material change for identical payloads', () => {
    const payload = {
      currentRun: {
        ts: '2026-04-01T00:00:00Z',
        artifact_path: '/tmp/run.json',
        req: { strategy_kind: 'DoubleMa' },
        res: { best_params: { fast_len: 10, slow_len: 30 } },
      },
      currentValidation: null,
    }

    const first = applyWorkflowArtifactsPayload(payload, 'vue')
    const second = applyWorkflowArtifactsPayload(payload, 'vue')

    expect(first.runChanged).toBe(true)
    expect(second.runChanged).toBe(false)
    expect(second.validationChanged).toBe(false)
  })
})
```

---

# 7. Suggested improvement plan

## Phase 1: land the highest-value low-risk fixes first

### 7.1 `workflowArtifactsStore.ts`
- add change-detection signatures
- enrich latest snapshot with `ts` + `artifactPath`
- return a structured apply result
- stop updating `updatedAtUnixMs` for identical payloads

### 7.2 `workflowArtifactsSync.ts`
- add conservative pre-load skip
- only trigger `refreshAnalysisFromWorkflowRun()` when run or validation changed

### 7.3 `analysisService.ts`
- fix local-stability request shape
- fix top-table duplicate sort/refresh work
- pass score-context consistently
- bound baseline and price-bar caches

These three items alone should produce the largest “felt” improvement for relatively little risk.

## Phase 2: fix panel reveal behavior

### 7.4 `analysisService.ts`
- add `heatmapRenderState`
- add `ensureHeatmapVisibleRender()`
- add `drillRenderState`
- add `restoreDrillViewFromCache()`
- make `rerenderDrill()` prefer restore over recompute

### 7.5 `HeatmapWorkspacePanel.vue`
- swap the full refresh call for visible-render restore

### 7.6 `DrillWorkspacePanel.vue`
- replace `setTimeout` mount nudge with one `[mountReady, visible]` watcher

This phase improves repeated panel entry without changing the underlying product behavior.

## Phase 3: evidence hygiene and follow-on tests

### 7.7 `double_ma_followup_workflow.rs`
- add metadata clarifying it is engine-only evidence

### 7.8 `ta_workflow_regression.rs`
- add serialized timing-boundary exclusions

### 7.9 tests
- add store apply no-op tests
- add sync skip tests
- add local-stability cache-key test
- add drill/heatmap restore tests

## Phase 4: optional second-pass items

Only after phases 1–3 are stable:

- bucket drill bins to improve drilldown cache reuse
- add explicit drill chart layout apply on reveal if still needed
- add command-level drill/local-stability metadata envelope if adjacent core files support it
- add a dedicated bundle-13 app-path perf harness

---

# 8. Per-file implementation guide and code snippets

This section is organized as a practical edit checklist by file.

## 8.1 `src-tauri/src/commands_analysis.rs`

### Goal
Do **not** change the product contract in phase 1. The mandatory bundle-13 fixes are elsewhere.

### Recommended edit
Add optional debug-only timing logs so you can observe candle-load vs compute time at the Tauri command boundary while validating the frontend changes.

### Suggested snippet

```rust
use std::time::Instant;

fn log_analysis_command_timing(
    command: &str,
    data_id: &str,
    candle_load_ms: u128,
    compute_ms: u128,
) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[analysis] command={} data_id={} candle_load_ms={} compute_ms={}",
        command, data_id, candle_load_ms, compute_ms
    );
}
```

Apply it to one command like this:

```rust
#[tauri::command]
pub async fn compute_double_ma_drilldown(
    req: DoubleMaDrilldownRequest,
    state: tauri::State<'_, AppState>,
) -> Result<DoubleMaCurves, String> {
    let data_id = req.data_id.clone();

    let load_started = Instant::now();
    let candles = state.get_candles(&data_id)?;
    let candle_load_ms = load_started.elapsed().as_millis();

    let compute_started = Instant::now();
    let result = tauri::async_runtime::spawn_blocking(move || {
        ta_app_core::compute_double_ma_drilldown_blocking_with_candles(req, &candles)
    })
    .await
    .map_err(|e| e.to_string())??;
    let compute_ms = compute_started.elapsed().as_millis();

    log_analysis_command_timing(
        "compute_double_ma_drilldown",
        &data_id,
        candle_load_ms,
        compute_ms,
    );

    Ok(result)
}
```

Repeat that pattern for the other three commands if you want consistent dev logging.

### Why this stays optional
It is diagnostics only. The real performance gains for this bundle come from the frontend/service/store work.

---

## 8.2 `web/src/stores/workflowArtifactsStore.ts`

### Goal
Turn artifact application into a change-aware operation instead of “any object payload means refresh”.

### Required edits
- add stable signatures
- add richer snapshot metadata
- return `WorkflowArtifactsApplyResult`
- avoid timestamp churn on no-op applies

### Mandatory snippet
Use the full code from **Confirmed issue 1**.

### Additional note
Because `readWorkflowRun()` in `analysisService.ts` already accepts partial `WorkflowRunState`, extending the latest snapshot to include `ts` and `artifactPath` is forward-compatible.

---

## 8.3 `web/src/services/workflowArtifactsSync.ts`

### Goal
Avoid file load + refresh when latest artifact identity is already loaded and validated locally.

### Required edits
- import `readLatestWorkflowRunIdentitySnapshot()`
- conservative same-artifact skip
- refresh only when run/validation changed

### Mandatory snippet
Use the full code from **Confirmed issue 1**.

### Additional note
If you later expose `file_modified_at` or `artifact_digest` from `list_run_artifacts`, replace the conservative skip heuristic with that stronger identity.

---

## 8.4 `web/src/services/analysisService.ts`

This file carries most of the bundle-13 work.

### 8.4.1 Cache policy cleanup

#### Goal
Use a consistent bounded-cache policy.

#### Required edits
- make `cacheSet()` generic
- add constants for all cache sizes
- bound baseline and price-bar caches

#### Snippet
Use the code from **Confirmed issue 4**.

---

### 8.4.2 Top-table dedupe and score-context correctness

#### Goal
Compute collected/sorted/filtered rows once per refresh.

#### Required edits
- remove nested overview refresh
- pass `scoreContext` in the explicit sort path
- use one sort/filter result for:
  - `topRowsModelCache`
  - `topRowsAllModelCache`
  - selected-row reconciliation
  - pinned candidate sync
  - row rendering

#### Snippet
Use the code from **Confirmed issue 2**.

---

### 8.4.3 Local-stability request shape fix

#### Goal
Prevent local-stability cache churn from UI-only drill fields.

#### Required edits
- stop spreading `...invocation.request`
- build stability requests from an explicit allowlist
- do not include `bins`

#### Snippet
Use the code from **Confirmed issue 3**.

---

### 8.4.4 Drill render-state restore

#### Goal
Make repeated drill-panel reveal cheap for the current selected row.

#### Required edits
- add `workflowRunIdentity()`
- add `DrillRenderState`
- add `restoreDrillViewFromCache()`
- capture state after successful pipeline completion
- make `rerenderDrill()` prefer restore

#### Snippet
Use the code from **Confirmed issue 6**.

#### Implementation note
This is the one place where I recommend being deliberate about invalidation. The restore path should only succeed when both match:

- current run identity
- current selected row id

That protects against stale drill views after a new artifact load or selection change.

---

### 8.4.5 Heatmap visible-render restore

#### Goal
Skip base heatmap repaint on pure panel reveal.

#### Required edits
- add `HeatmapRenderState`
- add `heatmapBaseSignature()`
- extract `paintHeatmapOverlay()`
- reuse base canvas when signature is unchanged
- add `ensureHeatmapVisibleRender()`

#### Snippet
Use the code from **Confirmed issue 5**.

#### Implementation note
The template uses fixed-size 512×512 canvases, so a base-signature keyed on run identity + model shape + canvas size is enough for phase 1.

---

### 8.4.6 Optional test shim exports

#### Goal
Make critical internal helpers testable without a large file split.

#### Suggested snippet

```ts
export const __analysisServiceTestOnly = {
  buildDrilldownInvocation,
  buildLocalStabilityInvocation,
  requestCacheKey,
  buildTopRowModelsForCurrentFilters,
}
```

Keep this clearly marked as test-only.

---

## 8.5 `web/src/components/DrillWorkspacePanel.vue`

### Goal
Replace the timeout-based mount nudge with a visibility-aware warm restore path.

### Required edits
- remove `onMounted`
- remove `window.setTimeout`
- add a watcher on `[mountReady, visible]`
- call `analysisStore.rerenderDrill()` from the watcher

### Mandatory snippet
Use the code from **Confirmed issue 6**.

### Why this is better
It is deterministic, visibility-aware, and matches the new service-level “restore before recompute” logic.

---

## 8.6 `web/src/components/HeatmapWorkspacePanel.vue`

### Goal
Stop full refresh on pure tab reveal.

### Required edits
- replace direct `refreshHeatmapFromWorkflowRun()` usage
- use `ensureHeatmapVisibleRender()`
- replace the separate mount + visible watchers with one `[mountReady, visible]` watcher

### Mandatory snippet
Use the code from **Confirmed issue 5**.

### Why this is better
It aligns the component with the new two-layer service model:

- full refresh when the run changes
- cheap visible restore when the panel simply reappears

---

## 8.7 `crates/app_core/benches/double_ma_followup_workflow.rs`

### Goal
Clarify timing-boundary scope without changing the benchmark’s job.

### Required edits
- extend printed metadata with timing-boundary note / bundle-13 note

### Mandatory snippet
Use the code from **Confirmed issue 7**.

### Why not rewrite this benchmark?
Because it is already useful at the engine boundary. The problem is interpretation, not the benchmark itself.

---

## 8.8 `src-tauri/src/bin/ta_workflow_regression.rs`

### Goal
Clarify that workflow regression is not a UI hydration benchmark.

### Required edits
- add `timing_boundary_exclusions`
- set it in defaults and generated summaries
- keep legacy deserialization default-safe

### Mandatory snippet
Use the code from **Confirmed issue 7**.

### Why not force bundle-13 into this harness?
Because that would overload the purpose of the harness. Bundle-13 deserves its own app-path measurement later.

---

# 9. Rollout order I recommend

## Step 1
Land:

- `workflowArtifactsStore.ts`
- `workflowArtifactsSync.ts`

Run manual checks:

- repeated sync with no new run does nothing
- validation added later still appears
- UI timestamp does not churn on no-op sync

## Step 2
Land:

- `analysisService.ts` top-table dedupe
- local-stability request shape fix
- bounded caches

Run checks:

- sorting still matches expected score/objective behavior
- local stability caches survive resize
- compare/baseline flow still works
- long-session memory growth stops being unbounded

## Step 3
Land:

- `analysisService.ts` drill + heatmap render-state restore
- `DrillWorkspacePanel.vue`
- `HeatmapWorkspacePanel.vue`

Run checks:

- repeated tab reveal is visibly faster
- drill panel restores same selected row without recompute
- heatmap base image does not repaint on pure reveal
- clicking heatmap cells and Top rows still launches drill correctly

## Step 4
Land:

- perf-artifact boundary clarifications in the benchmark / workflow files

Run checks:

- benchmark output still parses
- regression summary still serializes/deserializes

## Step 5
Add tests

This can happen earlier if your repo workflow requires it, but in practical terms the highest-risk regressions are easiest to lock down once the implementation shape is stable.

---

# 10. What I would not change in phase 1

To keep this bundle parity-safe, I would **not** do the following in the first pass:

- migrate charts to another library
- redesign panel ownership or shell architecture
- move analysis hydration into a worker
- change the drill/local-stability backend result contracts
- add new optimizer-native analysis features
- merge drilldown and local-stability into one command response
- overfit the current benchmark suite to the UI path by mutating unrelated harnesses

The phase-1 work in this document is already enough to remove the major confirmed waste.

---

# 11. Detailed implementation checklist

Use this section as the work tracker while implementing.

## 11.1 `workflowArtifactsStore.ts`

- [ ] Add `WorkflowArtifactsApplyResult`
- [ ] Extend `WorkflowRunSnapshot` to include `ts`
- [ ] Extend `WorkflowRunSnapshot` to include `artifactPath`
- [ ] Add `latestWorkflowRunSignature`
- [ ] Add `latestWorkflowValidationSignature`
- [ ] Add local `stableStringify()`
- [ ] Add `workflowRunSignature()`
- [ ] Add `workflowValidationSignature()`
- [ ] Add `readLatestWorkflowRunIdentitySnapshot()`
- [ ] Change `applyNormalizedArtifacts()` to return a structured result
- [ ] Avoid updating `updatedAtUnixMs` when run/validation are unchanged
- [ ] Preserve `store.source` updates without treating source-only changes as data changes
- [ ] Change `applyWorkflowArtifactsPayload()` to return the structured result
- [ ] Confirm existing store actions still compile after the return-type change

## 11.2 `workflowArtifactsSync.ts`

- [ ] Import `readLatestWorkflowRunIdentitySnapshot()`
- [ ] Read latest artifact `path`
- [ ] Read latest artifact `ts`
- [ ] Compare latest identity vs current identity
- [ ] Only skip pre-load when local validation already exists
- [ ] Keep the conservative “load if validation is still absent” behavior
- [ ] Use the richer apply result from `applyWorkflowArtifactsPayload()`
- [ ] Only call `refreshAnalysisFromWorkflowRun()` when `runChanged || validationChanged`
- [ ] Verify no-op sync returns without refreshing analysis
- [ ] Verify validation arriving later still propagates

## 11.3 `analysisService.ts` — cache policy

- [ ] Convert `cacheSet()` to a generic helper
- [ ] Add explicit constants for cache limits
- [ ] Bound `baselineArtifactCache`
- [ ] Bound `drillPriceBarsCache`
- [ ] Use the constants for drill/stability cache insertions
- [ ] Verify TypeScript still infers correct map value types

## 11.4 `analysisService.ts` — top-table dedupe

- [ ] Remove nested `refreshOverviewFromWorkflowRunStore()` inside `refreshTopTableFromWorkflowRun()`
- [ ] Compute `scoreContext` once in `refreshTopTableFromWorkflowRun()`
- [ ] Pass `scoreContext` into `sortRows()` in the explicit sort path
- [ ] Stop recollecting/resorting inside `buildTopRowModelsForCurrentFilters()` when pre-sorted rows are already available
- [ ] Update `topRowsModelCache` from `filtered`
- [ ] Update `topRowsAllModelCache` from `sorted`
- [ ] Keep row selection reconciliation behavior intact
- [ ] Keep pinned candidate reconciliation behavior intact
- [ ] Verify score-based ordering matches expected output for context-sensitive score modes

## 11.5 `analysisService.ts` — local-stability request shape

- [ ] Add `workflowRunIdentity()`
- [ ] Rewrite `buildLocalStabilityInvocation()` with an explicit allowlist
- [ ] Ensure local-stability requests no longer spread `...invocation.request`
- [ ] Ensure local-stability requests no longer include `bins`
- [ ] Keep `followup_cache_key`
- [ ] Keep `backend_hint`
- [ ] Keep `objective`, `radius`, and `step`
- [ ] Keep strategy-specific fields for Composite CPU modes
- [ ] Verify local-stability cache keys remain stable across UI resize changes

## 11.6 `analysisService.ts` — drill restore

- [ ] Add `DrillRenderState`
- [ ] Add `drillRenderState` module-level cache
- [ ] Add `restoreDrillViewFromCache()`
- [ ] Capture the drill render state after successful pipeline completion
- [ ] Include `runIdentity` in the render state
- [ ] Include `rowId` in the render state
- [ ] Include `curves`, `stability`, and `bars` in the render state
- [ ] Make `rerenderDrill()` prefer restore for same run + same selected row
- [ ] Optionally clear `drillRenderState` on hard failure
- [ ] Verify repeated drill-tab reveals do not re-enter the full pipeline
- [ ] Verify switching Top selection still computes the new drill path

## 11.7 `analysisService.ts` — heatmap restore

- [ ] Add `HeatmapRenderState`
- [ ] Add `heatmapBaseSignature()`
- [ ] Extract `paintHeatmapOverlay()`
- [ ] Cache `markers`, `cellW`, and `cellH`
- [ ] Make `renderHeatmapCanvas()` skip base repaint when the signature matches
- [ ] Keep legend rendering tied to base repaint only
- [ ] Add `ensureHeatmapVisibleRender()`
- [ ] Clear `heatmapHoverCell` on visible restore
- [ ] Hide tooltip on visible restore
- [ ] Verify repeated heatmap-tab reveals do not repaint the base grid
- [ ] Verify heatmap click-to-drill still works
- [ ] Verify marker overlay still renders correctly

## 11.8 `DrillWorkspacePanel.vue`

- [ ] Remove `onMounted`
- [ ] Remove `window.setTimeout(...)`
- [ ] Import `watch`
- [ ] Add `hydrateVisibleDrill()`
- [ ] Add immediate watcher on `[mountReady, visible]`
- [ ] Call `analysisStore.rerenderDrill()` from that watcher
- [ ] Verify first visible mount still populates correctly
- [ ] Verify later reveal is warm and stable

## 11.9 `HeatmapWorkspacePanel.vue`

- [ ] Replace `refreshHeatmapFromWorkflowRun` import with `ensureHeatmapVisibleRender`
- [ ] Remove the old `onMounted()` full-refresh hook
- [ ] Remove the old `watch(visible)` full-refresh hook
- [ ] Add `hydrateVisibleHeatmap()`
- [ ] Add immediate watcher on `[mountReady, visible]`
- [ ] Call `ensureHeatmapVisibleRender()` from that watcher
- [ ] Verify first visible mount still paints the heatmap
- [ ] Verify later reveals are warm and stable

## 11.10 `commands_analysis.rs`

- [ ] Decide whether to add debug-only timing logs in this phase
- [ ] If yes, add `Instant`
- [ ] Add a small debug-only log helper
- [ ] Capture candle-load time
- [ ] Capture blocking-compute time
- [ ] Log both times per analysis command in debug builds only
- [ ] Verify no product contract changed

## 11.11 `double_ma_followup_workflow.rs`

- [ ] Add `timing_boundary_note` metadata field
- [ ] Add `bundle_13_role` metadata field
- [ ] Verify benchmark output format still looks correct
- [ ] Verify no Criterion logic changed

## 11.12 `ta_workflow_regression.rs`

- [ ] Add `timing_boundary_exclusions` to `WorkflowSummary`
- [ ] Add `workflow_timing_boundary_exclusions()`
- [ ] Populate the field in defaults
- [ ] Populate the field in emitted summaries
- [ ] Update the legacy-deserialization test
- [ ] Verify existing baseline summary loading still works due to `#[serde(default)]`

## 11.13 Tests

- [ ] Add store no-op apply test
- [ ] Add validation-arrives-later test
- [ ] Add sync skip test
- [ ] Add local-stability cache-key test
- [ ] Add top-table score-context consistency test
- [ ] Add drill restore test
- [ ] Add heatmap visible-render restore test

## 11.14 Manual verification scenarios

- [ ] Repeatedly trigger legacy-runtime sync with no new artifact and confirm no visible analysis refresh occurs
- [ ] Load a run, then later load validation for that same run and confirm validation still appears
- [ ] Resize the app and confirm local-stability warm hits still occur for the same selection
- [ ] Switch between Top / Drill / Heatmap repeatedly and confirm warm reveals feel faster
- [ ] Compare multiple baselines across multiple datasets and confirm memory does not grow without bound
- [ ] Confirm heatmap markers/crosshair/tooltip still behave correctly after restore
- [ ] Confirm drill trade table, stability table, and TV chart still populate correctly after restore
- [ ] Confirm `MlSignal` branch behavior is unchanged
- [ ] Confirm benchmark summaries remain parseable and clearly scoped

---

# 12. Bottom line

If you only implement three things from this document, implement these first:

1. **true no-op artifact apply + sync skip**
2. **explicit local-stability request shape without `bins`**
3. **restore-before-recompute for drill and heatmap panel reveal**

Those three changes directly target the confirmed bundle-13 waste in the provided files and should give you the biggest responsiveness improvement with the least regression risk.
