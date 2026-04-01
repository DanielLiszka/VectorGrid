# Bundle 8 addendum: missed items, refinements, and optional extensions

This addendum is intended to be read alongside:

- `bundle8_frontend_orchestration_implementation_plan.md`

It focuses on:

1. items that were missed or under-emphasized in the first plan
2. refinements to the earlier code edits so they are safer and lower-churn
3. optional changes that go slightly beyond bundle 8 while still staying aligned with performance, optimization, and correctness

This addendum does **not** recommend removing features or narrowing current functionality.

---

## 1. What I would add to the original plan

The original plan was directionally correct, but I would add the following items.

### A. Gate the legacy DOM bridge so Vue is the default runtime path

### Why this matters
`optimizeService.ts` does more than mutate Pinia state. It also performs direct DOM writes through `writeDomInputValue()`, `writeDomCheckboxValue()`, and `writeDomTextContent()`, and those helpers dispatch synthetic `input` and `change` events. That means the optimize service can do duplicate work even when Vue/Pinia is already the source of truth.

The test file strongly suggests that the service is expected to work **without** legacy optimize runtime wrappers. The test setup explicitly sets `window.__VTA_LEGACY_BOOTED__ = false`, and one of the first tests is named around applying presets without legacy optimize runtime wrappers. That is a strong hint that the DOM bridge should be treated as a compatibility path, not the default path.

### Concrete recommendation
Make the DOM bridge conditional. If the legacy runtime is not booted, skip all direct DOM writes and synthetic event dispatches.

### Code snippet

```ts
function legacyOptimizeDomBridgeEnabled(): boolean {
  if (typeof window === "undefined") return false;
  return !!(
    window as Window & { __VTA_LEGACY_BOOTED__?: boolean }
  ).__VTA_LEGACY_BOOTED__;
}

function writeDomInputValue(
  id: string,
  rawValue: string,
  scopeSelector?: string,
): void {
  if (!legacyOptimizeDomBridgeEnabled()) return;
  const el = queryDomById<
    HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
  >(id, scopeSelector);
  if (!el) return;
  const value = String(rawValue || "");
  if (el.value === value) return;
  el.value = value;
  dispatchInputLikeEvents(el);
}

function writeDomCheckboxValue(
  id: string,
  checked: boolean,
  scopeSelector?: string,
): void {
  if (!legacyOptimizeDomBridgeEnabled()) return;
  const el = queryDomById<HTMLInputElement>(id, scopeSelector);
  if (!el) return;
  const next = !!checked;
  if (el.checked === next) return;
  el.checked = next;
  dispatchInputLikeEvents(el);
}

function writeDomTextContent(
  id: string,
  value: string,
  scopeSelector?: string,
): void {
  if (!legacyOptimizeDomBridgeEnabled()) return;
  const el = queryDomById<HTMLElement>(id, scopeSelector);
  if (!el) return;
  el.textContent = value;
}
```

### Why this should be in phase 1 or 2
This is one of the cleanest ways to reduce redundant frontend orchestration work without removing any feature. It preserves compatibility while making the Vue path cheaper.

---

### B. Do **not** call the side-effecting recommended-default helper directly from `runOptimization()`

### Why this matters
The first plan recommended wiring `applyRecommendedOptimizationDefaults()` into the auto-build path. That needs refinement.

Right now, `recommendedOptimizationDefaults.ts` is not a pure helper. It mutates:

- `advancedStore.topK`
- `advancedStore.heatmapBins`
- `advancedStore.statusLine`
- `advancedStore.source`
- `advancedStore.updatedAtUnixMs`

If you call that helper inside `runOptimization()`, you introduce extra store churn and status mutation during the hot path. That is avoidable.

### Better recommendation
Split the helper into:

1. a **pure** resolver that computes fallback values
2. an **optional** mutating wrapper for UI-driven repair actions

Use the pure resolver in the optimize auto-build path.

### Code snippet

```ts
export interface RecommendedOptimizationDefaultsResolution {
  topK: string | null;
  heatmapBins: string | null;
  applied: string[];
}

function parseOptionalNonNegativeInt(value: unknown): number | null {
  const text = String(value ?? "").trim();
  if (!text) return null;
  const next = Number(text);
  if (!Number.isFinite(next) || next < 0) return null;
  return Math.floor(next);
}

export function resolveRecommendedOptimizationDefaults(input: {
  topK: unknown;
  heatmapBins: unknown;
  mode: unknown;
}): RecommendedOptimizationDefaultsResolution {
  const applied: string[] = [];
  let topK: string | null = null;
  let heatmapBins: string | null = null;

  const parsedTopK = parseOptionalNonNegativeInt(input.topK);
  if (parsedTopK == null || parsedTopK <= 0) {
    topK = RECOMMENDED_TOP_K;
    applied.push(`Top K ${topK}`);
  }

  const parsedHeatmapBins = parseOptionalNonNegativeInt(input.heatmapBins);
  if (
    parsedHeatmapBins == null &&
    modeSupportsRecommendedGridHeatmap(input.mode)
  ) {
    heatmapBins = RECOMMENDED_HEATMAP_BINS;
    applied.push(`Heatmap bins ${heatmapBins}`);
  }

  return { topK, heatmapBins, applied };
}

export function applyRecommendedOptimizationDefaults(): boolean {
  const pinia = resolveRuntimePinia();
  if (!pinia) return false;

  const advancedStore = useAdvancedStore(pinia);
  const resolved = resolveRecommendedOptimizationDefaults({
    topK: advancedStore.topK,
    heatmapBins: advancedStore.heatmapBins,
    mode: advancedStore.mode,
  });

  let changed = false;
  if (resolved.topK != null) {
    advancedStore.topK = resolved.topK;
    changed = true;
  }
  if (resolved.heatmapBins != null) {
    advancedStore.heatmapBins = resolved.heatmapBins;
    changed = true;
  }

  if (changed) {
    advancedStore.statusLine =
      `Applied recommended output defaults: ${resolved.applied.join(", ")}.`;
    advancedStore.source = "vue";
    advancedStore.updatedAtUnixMs = Date.now();
  }

  return changed;
}
```

Then, inside `optimizeService.ts`, use a quiet, non-status-mutating normalizer:

```ts
function applyQuietRecommendedDefaultsForRun(): void {
  const advancedStore = readPiniaStoreState("advanced");
  if (!advancedStore) return;

  const resolved = resolveRecommendedOptimizationDefaults({
    topK: advancedStore.topK,
    heatmapBins: advancedStore.heatmapBins,
    mode: advancedStore.mode,
  });

  const patch: Record<string, unknown> = {};
  if (resolved.topK != null) patch.topK = resolved.topK;
  if (resolved.heatmapBins != null) patch.heatmapBins = resolved.heatmapBins;
  if (Object.keys(patch).length > 0) {
    patchStoreState("advanced", patch);
  }
}
```

### Why this is better
It preserves the fallback-default feature while avoiding unnecessary status mutation in the optimize hot path.

---

### C. After splitting busy state, guards must stop checking only `canCancel`

### Why this matters
The first plan proposed splitting the lifecycle into:

- command-running
- post-run busy
- idle

That change is right, but it introduces a secondary requirement: code paths that currently early-return on `canCancel` need to use a real “busy” predicate instead.

Otherwise, after the command finishes and `canCancel` drops to `false`, actions like `applyPreset()`, `presetKernel()`, or `runGpuParityCheck()` could execute while post-run follow-up is still active.

### Code snippet

```ts
function isOptimizeBusy(): boolean {
  const optimizeStore = readPiniaStoreState("optimize");
  return (
    normalizeBoolean(optimizeStore?.runDisabled) ||
    normalizeBoolean(optimizeStore?.validateDisabled) ||
    normalizeBoolean(optimizeStore?.canCancel)
  );
}
```

Then update the early guards:

```ts
export async function applyPreset(): Promise<void> {
  if (isOptimizeBusy()) return;
  // existing body
}

export async function presetKernel(): Promise<void> {
  if (isOptimizeBusy()) return;
  // existing body
}

export async function runGpuParityCheck(): Promise<void> {
  if (isOptimizeBusy()) return;
  // existing body
}
```

### Why this should be in the main plan
This is a correctness dependency of the earlier state-splitting recommendation.

---

### D. Treat a blank artifact path as a failed save, not a success

### Why this matters
The earlier plan handled thrown `save_run_artifact` failures, but a second case also matters: a non-throw response with a blank or invalid path.

Today, `persistRunArtifact()` trusts any non-throw response. That means a payload like `{ path: "" }` still looks “successful enough” to the current flow.

### Code snippet

```ts
async function persistRunArtifact(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
  context: RunArtifactContext,
): Promise<PersistRunArtifactResult> {
  const artifact = buildRunArtifactPayload(req, res, ts, context);

  try {
    const saved = (await invokeCommand("save_run_artifact", {
      artifact,
    })) as SaveRunArtifactResponse;

    const path = normalizeText(saved?.path);
    if (!path) {
      return {
        path: "",
        csvPath: context.csvPath,
        datasetMeta: context.datasetMeta,
        saved: false,
        warning: "artifact_save_invalid=empty_path",
      };
    }

    return {
      path,
      csvPath: context.csvPath,
      datasetMeta: context.datasetMeta,
      saved: true,
      warning: "",
    };
  } catch (error) {
    return {
      path: "",
      csvPath: context.csvPath,
      datasetMeta: context.datasetMeta,
      saved: false,
      warning: `artifact_save_failed=${String(error || "")}`.trim(),
    };
  }
}
```

### Test addition
Add a frontend test where `save_run_artifact` returns `{}` or `{ path: "" }` and ensure the UI marks the run unsaved.

---

### E. Avoid the double tab flip when auto-validation is enabled

### Why this matters
The current optimize path can do this when `switchTab` is true and auto-validation is enabled:

1. switch to `top`
2. run validation
3. switch to `overview`

That is unnecessary UI churn and may cause avoidable re-render/hydration work.

### Better recommendation
Only switch once to the final target tab.

### Code snippet

```ts
const autoValidationEnabled = readValidationAutoRunEnabled();
let validationSucceeded = false;

if (autoValidationEnabled) {
  const validationStartedAt = nowMs();
  try {
    await runValidationAction({ useFiltered: false, switchTab: false });
    timings.validationMs = elapsedMs(validationStartedAt);
    validationSucceeded = true;
  } catch (error) {
    timings.validationMs = elapsedMs(validationStartedAt);
    progressNotes.push(
      isCancellationError(error)
        ? "validation=cancelled"
        : `validation_failed=${normalizeText(error)}`,
    );
  }
}

if (switchTab) {
  if (autoValidationEnabled && validationSucceeded) {
    selectWorkflowResultsTab("overview", true);
  } else {
    selectWorkflowResultsTab("top", false);
  }
}
```

### Why this is worth adding
It is a low-risk improvement that reduces UI churn without removing any feature.

---

### F. Cache GPU status briefly or skip redundant refreshes

### Why this matters
`refreshGpuStatus()` is a direct Tauri roundtrip. The current file calls it in several places:

- `presetKernel()`
- `runGpuParityCheck()` before the parity run
- `runGpuParityCheck()` again after the parity run
- `runOptimization()` after every successful run

GPU build/runtime status is typically much more static than optimization results, so a short-lived cache or more selective refresh policy is appropriate.

### Low-risk option
Add a short TTL cache.

### Code snippet

```ts
const GPU_STATUS_CACHE_TTL_MS = 1_500;

let gpuStatusCache:
  | { value: GpuStatusSummary; expiresAt: number }
  | null = null;

function invalidateGpuStatusCache(): void {
  gpuStatusCache = null;
}

export async function refreshGpuStatus(
  options: { force?: boolean } = {},
): Promise<GpuStatusSummary> {
  const now = nowMs();
  if (
    !options.force &&
    gpuStatusCache &&
    gpuStatusCache.expiresAt > now
  ) {
    return gpuStatusCache.value;
  }

  const status = await invokeCommand("get_gpu_status");
  const summary = normalizeStatusText(status);
  gpuStatusCache = {
    value: summary,
    expiresAt: now + GPU_STATUS_CACHE_TTL_MS,
  };
  return summary;
}
```

### Policy recommendation
Use a forced refresh only for:

- explicit manual refresh actions
- startup/bootstrap if you want a fresh first read
- paths where a device/runtime failure specifically suggests the cache might be stale

For normal post-run optimize completion, the cached value is usually sufficient.

---

### G. Fix the unsupported-strategy parity message

### Why this matters
`runGpuParityCheck()` currently supports:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`

But the unsupported message only says to switch to “Double MA, MARRT, or VFCT.” That is incomplete and slightly misleading.

### Code snippet

```ts
const PARITY_SUPPORTED_STRATEGIES: ReadonlySet<OptimizeStrategyKind> = new Set([
  "DoubleMa",
  "Pit",
  "Set",
  "Mts",
  "Orc",
  "Marrt",
  "Vfct",
]);
```

Then:

```ts
if (!PARITY_SUPPORTED_STRATEGIES.has(strategyKind)) {
  updateParityStatus(
    "Parity: supported for DoubleMa/Pit/Set/Mts/Orc/Marrt/Vfct only.",
    "Parity unavailable: unsupported strategy.",
  );
  return;
}
```

### Why this is worth including
This is a small correctness/documentation fix with no downside.

---

### H. Add a low-risk validation step for injected requests

### Why this matters
Injected requests bypass the store-based build path, which is fine, but `runOptimization()` still resolves the command from `strategy_kind` / `strategyKind`. If that field is missing on an injected payload, the code can silently fall back to `DoubleMa` routing.

### Code snippet

```ts
function assertInjectedOptimizationRequestShape(
  req: Record<string, unknown>,
): void {
  const strategyKind = normalizeText(req.strategy_kind ?? req.strategyKind);
  const dataId = normalizeDataId(req.data_id ?? req.dataId);

  if (!strategyKind) {
    throw new Error("Injected optimization request is missing strategy_kind.");
  }
  if (!dataId) {
    throw new Error("Injected optimization request is missing data_id.");
  }
}
```

Then in the request resolver:

```ts
const injected = normalizeRequestPayload(options.request);
if (injected) {
  assertInjectedOptimizationRequestShape(injected);
  return {
    req: injected,
    requestBuildMs: 0,
  };
}
```

### Why this is a good optional guard
It is small, safe, and improves correctness for service callers.

---

### I. Add an explicit note about cancellation domains

### Why this matters
The Rust command layer exposes multiple cancellation entry points:

- `cancel_current_run`
- `cancel_double_ma_optimization`
- `cancel_background_task`

That means the correct cancellation path may depend on whether the system is currently inside:

- the optimize command itself
- a background validation/export/report phase

This was acknowledged as an open question in the first plan, but I would make it more explicit in the document so it becomes a tracked follow-up rather than a footnote.

### Recommended note to add to the plan
Add a dedicated “adjacent scope follow-up” item:

> Audit `runControlService.ts` and `validationService.ts` to ensure cancel requests are routed to `cancel_current_run` during command execution and to `cancel_background_task` during background validation/report phases.

I would **not** try to implement that in the bundle-8 files alone, but I would track it explicitly.

---

## 2. Refinements to the earlier `runOptimization()` rewrite

The earlier replacement for `runOptimization()` is still the right direction, but I would refine it with these additional points.

### 1. Do quiet default repair before build
Use the pure default resolver or a quiet apply helper, not the noisy UI helper.

### 2. Validate injected requests early
This prevents silent command misrouting.

### 3. Build `artifactContext` only after request resolution succeeds
That keeps the sequence clean and makes timing easier to interpret.

### 4. Treat blank artifact paths as unsaved
This should feed the same progress-note path as thrown save errors.

### 5. Switch tabs only once
Choose the final tab after validation outcome is known.

### 6. Avoid unconditional post-run GPU refresh when a cached summary is acceptable
This is optional, but it is an easy follow-on if you want to squeeze extra responsiveness from the warm path.

---

## 3. Optional changes that go slightly beyond bundle 8, but still align with your goals

These are not necessary for the bundle-8 implementation itself, but they are strong follow-ups if your top priority remains performance, optimization, and correctness.

### Optional extension 1: bring in bundle 7 evidence before changing numeric defaults

The review map makes it clear that benchmark harnesses, perf gates, and timing boundaries are the correct evidence path before claiming a given `top_k` or `heatmap_bins` default is measured-best. If you want to revisit the actual numbers in `Quick`, `Balanced`, `Deep`, or the recommended fallback helper, the next correct step is a bundle-7 review, not guesswork.

### Recommended next action
Do a narrow follow-up against the perf-gate and app-path harness surfaces before changing preset numbers.

---

### Optional extension 2: audit validation orchestration as bundle 9 immediately after bundle 8

Once optimize orchestration is cleaned up, the next likely latency/correctness surface is the validation follow-up path:

- validation launch cost
- validation route-summary preservation
- artifact-heavy follow-up
- save/replay drift
- background cancellation domain correctness

That is the most natural adjacent review after bundle 8.

---

### Optional extension 3: defer or lazily hydrate heavier analysis follow-up

If `refreshAnalysisFromWorkflowRun()` is still expensive after bundle 8 cleanup, the next place to look is bundle 13, not more changes inside `optimizeService.ts` alone. That bundle owns analysis/drilldown and workflow-artifact hydration responsiveness.

### Why I am not recommending that as part of bundle 8
It crosses into adjacent-file behavior that was not part of the original file set.

---

## 4. Additional checklist items to append to the original document

Use these as incremental checklist items to add to the bottom of the first plan.

### Additional checklist: missed items / refinements
- [ ] Add a `legacyOptimizeDomBridgeEnabled()` guard and bypass direct DOM writes when legacy wrappers are not booted.
- [ ] Add tests proving preset application and optimize orchestration still work with `window.__VTA_LEGACY_BOOTED__ = false`.
- [ ] Split recommended-default logic into a pure resolver plus an optional mutating UI helper.
- [ ] Use the pure or quiet default repair path inside optimize request resolution.
- [ ] Add `isOptimizeBusy()` and convert early guards from `canCancel` checks to busy-state checks.
- [ ] Treat blank `save_run_artifact` paths as unsaved runs.
- [ ] Add a test where `save_run_artifact` returns an empty path.
- [ ] Switch workflow tabs only once per optimize run when auto-validation is enabled.
- [ ] Add a short TTL cache or selective-refresh policy for GPU status.
- [ ] Fix the parity unsupported-strategy message so it reflects all supported kinds.
- [ ] Add a low-risk guard for injected requests missing `strategy_kind` or `data_id`.
- [ ] Add an explicit follow-up item to audit cancel-domain routing in `runControlService.ts` / `validationService.ts` against `cancel_current_run` vs `cancel_background_task`.

---

## 5. Recommended revised implementation order

If I were revising the original execution order, I would use this order instead.

### Phase 0: tests for the newly missed cases
Add tests for:
- legacy DOM bridge disabled behavior
- blank artifact path treated as unsaved
- single-tab-switch behavior with auto-validation enabled
- injected request missing `strategy_kind`

### Phase 1: correctness-critical fixes
Implement:
- pure/quiet default resolver
- explicit build-error transparency
- busy guard helper (`isOptimizeBusy()`)
- blank artifact path handling
- parity message correction

### Phase 2: orchestration overhead reduction
Implement:
- DOM bridge gating
- single final tab switch
- compact output preview
- artifact-context reuse

### Phase 3: post-run status cleanup
Implement:
- stable command-completion `statusLine`
- phase timing in `progressMeta`
- unsaved warnings and validation warnings appended to `progressMeta`

### Phase 4: optional warm-path polish
Implement only if desired after the earlier steps are green:
- GPU status TTL cache
- injected-request validation guard
- additional profiling polish in Rust

---

## 6. Bottom line

I do think the first plan was missing a few real items.

The most important additions are:

1. **legacy DOM bridge gating**
2. **pure/quiet recommended-default repair instead of calling the side-effecting helper from the run path**
3. **busy guards based on real busy state, not just `canCancel`**
4. **blank artifact path treated as save failure**
5. **single final tab switch instead of `top -> overview` churn**
6. **short-lived GPU status caching or at least fewer redundant refreshes**

If your goal is maximum performance and correctness without removing functionality, those are the main improvements I would add on top of the original document.
