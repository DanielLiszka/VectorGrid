# Bundle 13 Addendum: Additional fixes, missed items, and optional extensions

This addendum supplements the earlier Bundle 13 implementation plan.

It focuses on items that are either:

- newly identified and worth adding to the original plan,
- important correctness/performance refinements to the earlier code-edit guidance, or
- optional changes that go a bit beyond the strict prompt scope while still staying aligned with performance, optimization, and correctness.

The goal remains the same: improve responsiveness and correctness **without removing features or breaking functionality**.

---

# 1. What I would add to the original plan now

## 1.1 Confirmed missed issue: `score_mode` is not propagated into local-stability requests

### Why this matters

`buildLocalStabilityInvocation()` currently adds `objective`, `radius`, and `step`, but it does **not** propagate `score_mode` from the run request.

That means a run optimized with a score mode such as:

- `balanced_risk_adjusted_10`
- `adversarial_envelope_10`
- `support_shrunk_10`
- `active_sortino_10`
- `effective_sample_lcb_10`
- `omega_vs_sharpe_10`
- `robust_margin_10`

can still compute local stability using only the plain objective path.

This is primarily a **correctness / parity** problem, not just a display issue. The stability neighborhood can disagree with the ranking logic used to pick the candidate in the first place.

### Why this was easy to miss

The current code already computes score-mode-aware ranking and top-table score values elsewhere. The omission only shows up inside the local-stability request builder.

### Recommended edit

When building the local-stability request:

- use `objectiveOf(runReq)` instead of `text(runReq.objective) || "Sharpe"`
- propagate `score_mode` for the `DoubleMa` path
- for the `CompositeCpu` path, only propagate `score_mode` if that request type supports it in your local type definitions

### Suggested snippet

```ts
function buildLocalStabilityInvocation(
  paramsRaw: Record<string, unknown>,
  invocation: DrillInvocation,
): StabilityInvocation | null {
  const runReq = invocation.runReq;
  const objective = objectiveOf(runReq);
  const scoreMode = scoreModeOf(runReq);

  const base: Record<string, unknown> = {
    data_id: invocation.request.data_id,
    backend_hint: invocation.request.backend_hint,
    followup_cache_key: invocation.request.followup_cache_key,
    params: invocation.request.params,
    ma_source: invocation.request.ma_source,
    fast_ma_params: invocation.request.fast_ma_params,
    slow_ma_params: invocation.request.slow_ma_params,
    strategy: invocation.request.strategy,
    objective,
    radius: 1,
    step: 1,
  };

  if (invocation.command === "compute_double_ma_drilldown") {
    return {
      command: "compute_double_ma_local_stability",
      request: {
        ...base,
        ...(scoreMode ? { score_mode: scoreMode } : {}),
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
        ...base,
        // Only include score_mode here if your CompositeCpu request type supports it.
        // ...(scoreMode ? { score_mode: scoreMode } : {}),
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
      ...base,
      // Only include score_mode here if your CompositeCpu request type supports it.
      // ...(scoreMode ? { score_mode: scoreMode } : {}),
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

### Test to add

```ts
it("propagates score_mode into double-ma local-stability requests", () => {
  const invocation = {
    runReq: {
      data_id: "spy",
      objective: "Sharpe",
      score_mode: "balanced_risk_adjusted_10",
    },
    kind: "DoubleMa",
    command: "compute_double_ma_drilldown",
    request: {
      data_id: "spy",
      backend_hint: { backend: "CpuOnly" },
      params: { fast_len: 10, slow_len: 30, fast_ma_type: "sma", slow_ma_type: "sma" },
      ma_source: "close",
      fast_ma_params: null,
      slow_ma_params: null,
      strategy: {},
      followup_cache_key: "abc",
      halftrend_filter: null,
    },
  } as const;

  const built = buildLocalStabilityInvocation({}, invocation as unknown as DrillInvocation);
  expect(built?.command).toBe("compute_double_ma_local_stability");
  expect((built?.request as Record<string, unknown>).score_mode).toBe(
    "balanced_risk_adjusted_10",
  );
});
```

---

## 1.2 Confirmed missed issue: stale drill DOM/status updates can slip through after price-bar fetch

### Why this matters

`runVueDrillPipeline()` guards stale results immediately after the drilldown compute returns, and again after local stability returns.

However, there is **no stale-sequence guard after `fetchDrillPriceBars()`** and before:

- `renderDrillDomFallback(...)`
- drill status/meta writes
- other drill-side DOM/store updates

If the user selects another row while the old pipeline is still waiting for `get_price_bars`, the old request can still paint stale drill DOM and status text before the newer request finishes.

This is a correctness / responsiveness issue. It can produce stale flashes even when `drillSeq` is intended to suppress them.

### Recommended edit

Add a sequence guard immediately after price-bar fetch and before any drill-side DOM or store mutation.

### Suggested snippet

```ts
let bars: DrillPriceBar[] | null = null;
const dataId = text(runReq.data_id ?? runReq.dataId);
if (dataId) {
  try {
    bars = await fetchDrillPriceBars(dataId);
  } catch {
    bars = null;
  }
}

if (seq !== drillSeq) return true;

const rendered = renderDrillDomFallback(curves, params, metrics, row, bars);
if (rendered) {
  void renderDrillTvChartDom(curves, bars, seq).catch(() => {});
}

if (seq !== drillSeq) return true;

if (store) {
  store.drillStatus = cachedDrill ? "Done (cached)." : "Done.";
  // existing drillMeta update...
  markAnalysisVue(store);
}
```

### Test / manual check

- Select row A
- quickly select row B before `get_price_bars` resolves
- verify that row A does not repaint equity/drawdown/trade-table/status after row B is the active request

---

## 1.3 Confirmed missed issue: identical in-flight requests are not deduplicated

### Why this matters

The current caches only store **completed** results.

That means identical calls can still duplicate backend work while the first request is still in flight.

This applies to:

- drilldown compute requests
- local-stability compute requests
- `get_price_bars`
- baseline artifact disk loads
- and possibly legacy artifact sync, depending on how often it is triggered

This is one of the biggest remaining “warm-path still not actually warm” problems on the frontend side.

### Concrete examples

- rapid row re-selection can issue the same drilldown twice before the first result lands
- drill restore / rerender + user selection can overlap and issue the same local-stability request twice
- two near-simultaneous drill requests for the same dataset can both call `get_price_bars`
- two compare refreshes targeting the same baseline path can both call `load_run_artifact`

### Recommended edit

Add a generic `cacheGetOrLoad()` helper with both:

- settled-result cache
- pending-promise cache

Then reuse it for:

- `drillCurvesCache`
- `stabilityCache`
- `drillPriceBarsCache`
- `baselineArtifactCache`

### Suggested snippet

```ts
const drillCurvesPending = new Map<string, Promise<unknown>>();
const stabilityPending = new Map<string, Promise<unknown>>();
const drillPriceBarsPending = new Map<string, Promise<DrillPriceBar[]>>();
const baselineArtifactPending = new Map<string, Promise<BaselineRun | null>>();

async function cacheGetOrLoad<T>(
  cache: Map<string, T>,
  pending: Map<string, Promise<T>>,
  key: string,
  load: () => Promise<T>,
  max = 96,
): Promise<{ value: T; cacheHit: boolean; sharedInFlight: boolean }> {
  const cached = cache.get(key);
  if (cached !== undefined) {
    return { value: cached, cacheHit: true, sharedInFlight: false };
  }

  const existing = pending.get(key);
  if (existing) {
    return {
      value: await existing,
      cacheHit: false,
      sharedInFlight: true,
    };
  }

  const promise = load()
    .then((value) => {
      cacheSet(cache, key, value, max);
      return value;
    })
    .finally(() => {
      pending.delete(key);
    });

  pending.set(key, promise);
  return {
    value: await promise,
    cacheHit: false,
    sharedInFlight: false,
  };
}
```

Use it in `runVueDrillPipeline()`:

```ts
const drillLoad = await cacheGetOrLoad(
  drillCurvesCache,
  drillCurvesPending,
  drillKey,
  () => invokeCommand(invocation.command, { req: invocation.request }),
  96,
);
const curves = drillLoad.value;
const cachedDrill = drillLoad.cacheHit || drillLoad.sharedInFlight;
```

And for stability:

```ts
const stabilityLoad = await cacheGetOrLoad(
  stabilityCache,
  stabilityPending,
  stabilityKey,
  () => invokeCommand(stability.command, { req: stability.request }),
  96,
);
const stabilityRes = stabilityLoad.value;
const cachedStability =
  stabilityLoad.cacheHit || stabilityLoad.sharedInFlight;
```

And for price bars:

```ts
async function fetchDrillPriceBars(dataId: string): Promise<DrillPriceBar[]> {
  const id = text(dataId);
  if (!id) return [];

  const loaded = await cacheGetOrLoad(
    drillPriceBarsCache,
    drillPriceBarsPending,
    id,
    async () => {
      let raw: unknown;
      try {
        raw = await invokeCommand("get_price_bars", {
          dataId: id,
          maxPoints: 8000,
        });
      } catch (firstError) {
        try {
          raw = await invokeCommand("get_price_bars", {
            data_id: id,
            max_points: 8000,
          });
        } catch {
          throw firstError;
        }
      }
      return normalizeDrillPriceBars(raw);
    },
    24,
  );

  return loaded.value;
}
```

And for baseline loads:

```ts
async function loadBaselineFromDisk(path: string): Promise<BaselineRun | null> {
  const normalizedPath = text(path);
  if (!normalizedPath) return null;

  const loaded = await cacheGetOrLoad(
    baselineArtifactCache,
    baselineArtifactPending,
    normalizedPath,
    async () => {
      const raw = (await invokeCommand("load_run_artifact", {
        path: normalizedPath,
      })) as RunArtifactPayload;
      const row = asRecord(raw);
      const run = asRecord(row.run);
      const req = asRecord(run.req);
      const res = asRecord(run.res);
      if (!Object.keys(req).length || !Object.keys(res).length) return null;
      return {
        source: "disk",
        ts: text(row.ts) || text(run.ts),
        req,
        res,
      } satisfies BaselineRun;
    },
    24,
  );

  return loaded.value;
}
```

### Why this belongs near the top of the rollout order

This is a true responsiveness improvement that does not change the user-visible feature set and does not require backend math changes.

---

## 1.4 Likely missed issue: compute caches have no explicit invalidation hook for dataset changes

### Why this matters

`drillCurvesCache`, `stabilityCache`, and `drillPriceBarsCache` are all module-level caches.

There is no explicit cache invalidation function in `analysisService.ts`.

If a dataset can change while preserving the same `data_id`, the cache keys used today are vulnerable to staleness.

The current files do not include the data-loading call sites, so I am classifying this as **likely issue / correctness risk**, not fully confirmed. But it is important enough to plan for.

### Recommended edit

Add an exported invalidation hook now, even if you wire it up later from the data-load / resample surfaces.

### Suggested snippet

```ts
export function invalidateAnalysisExecutionCaches(): void {
  drillCurvesCache.clear();
  stabilityCache.clear();
  drillPriceBarsCache.clear();
  drillRenderState = null;
  heatmapHoverCell = null;
  drillSeq = (drillSeq + 1) >>> 0;
}
```

### Integration note

The actual call sites likely belong in the data-load / resample / bootstrap bundle, not strictly inside the current file set. Good trigger points would be:

- dataset reload
- timeframe resample
- explicit dataset switch
- any workflow bootstrap path that restores a different dataset into the same logical slot

---

## 1.5 Optional in-scope addition: treat Pareto like Heatmap for hidden-render suppression

### Why this matters

`refreshAnalysisFromWorkflowRun()` always calls `refreshParetoFromWorkflowRun()`, and `renderParetoCanvas()` always repaints the canvas and rebinds click handlers.

Compared with heatmap, Pareto is probably a smaller hotspot, but it is still hidden work when the user is not looking at that panel.

### Recommended edit

This does **not** need to be phase 1, but I would add it to the backlog:

- add a render signature for Pareto
- skip canvas repaint when identical
- preferably defer full Pareto canvas rendering while hidden, the same way you are now planning to be more careful on heatmap/drill reveal

### Why I classify this as optional

It is useful, but the bigger wins are still:

- no-op artifact sync
- local-stability request correctness
- stale-sequence suppression
- in-flight dedupe

---

# 2. Refinements I would make to the earlier plan

## 2.1 The local-stability fix should include both `bins` removal **and** score-mode propagation

The earlier plan correctly focused on removing drill-only fields like `bins` from the local-stability cache key.

I would now make the local-stability fix explicitly do **both**:

1. construct a typed request explicitly instead of spreading the drill request
2. propagate `objectiveOf(runReq)`
3. propagate `score_mode` where supported
4. keep drill-only fields out of the request entirely

That makes the fix both a cache-key correction and a scoring-parity correction.

## 2.2 The drill rerender / restore work should include stale-sequence guards before DOM mutation

The restore idea in the original plan is still good.

But before or alongside that change, I would add the smaller correctness guard first:

- after `fetchDrillPriceBars()`
- before `renderDrillDomFallback()`
- before drill-side status/meta writes

That fix is tiny and removes a real stale-render race.

## 2.3 The artifact-diff path should stay cheap on the hot path

The earlier plan’s “detect changed vs unchanged artifact payload” direction is right.

I would just add one refinement:

- keep the **pre-load identity guard** cheap and conservative
- only do deeper normalized comparison when a full artifact had to be loaded anyway
- if you later add a backend artifact digest/head command, move deeper equality there instead of doing more frontend JSON work

The direction is still right; this is just a practical implementation note.

---

# 3. Optional changes that go a bit beyond the strict bundle prompt

These are **not required** to get most of the bundle-13 value, but they are sensible next-step optimizations if you want to go further.

## 3.1 Add a combined follow-up command for drilldown + local stability

### Why it is attractive

Today the frontend does:

1. compute drilldown
2. fetch price bars
3. compute local stability

And the Tauri boundary exposes drilldown and stability as separate commands.

A combined backend follow-up command could:

- load candles once
- run both follow-up computations in one blocking section
- optionally return route/cache metadata together
- reduce command overhead and simplify status handling

### Compatibility-safe approach

Keep the current commands and add an **optional** combined command that the frontend can use when available.

### Sketch

```rust
#[derive(serde::Serialize)]
pub struct DoubleMaFollowupBundle {
    pub drilldown: DoubleMaCurves,
    pub stability: LocalStabilityResult,
}

#[tauri::command]
pub async fn compute_double_ma_followup_bundle(
    drill_req: DoubleMaDrilldownRequest,
    stability_req: DoubleMaLocalStabilityRequest,
    state: tauri::State<'_, AppState>,
) -> Result<DoubleMaFollowupBundle, String> {
    let candles = state.get_candles(&drill_req.data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let drilldown =
            ta_app_core::compute_double_ma_drilldown_blocking_with_candles(
                drill_req,
                &candles,
            )?;
        let stability =
            ta_app_core::compute_double_ma_local_stability_blocking_with_candles(
                stability_req,
                &candles,
            )?;
        Ok(DoubleMaFollowupBundle { drilldown, stability })
    })
    .await
    .map_err(|e| e.to_string())?
}
```

### Why I would keep it optional

This touches contracts and frontend orchestration. It is a valid optimization, but it is larger than the safer frontend-side fixes.

---

## 3.2 Add a cheap backend “latest artifact head” / digest path

### Why it is attractive

The cleanest no-op sync is not “load the whole artifact and compare it.”

The cleanest no-op sync is:

- ask for a cheap latest-artifact head/digest
- compare head/digest to the last applied one
- load the full artifact only when the head/digest changed

### Example shape

A future backend-side metadata command could expose something like:

```ts
interface LatestArtifactHead {
  ts: string;
  path: string;
  hasValidation: boolean;
  digest: string;
}
```

### Why it is beyond current scope

The files needed to implement the artifact-head command are not part of the current review bundle.

Still, this is a very reasonable follow-on if artifact sync remains hot after the safer guards land.

---

## 3.3 Add dev-only app-path timing instrumentation in `analysisService.ts`

### Why it is useful

The current supporting benchmark evidence is not a true analysis-hydration app-path benchmark.

A small amount of dev-only phase timing inside `analysisService.ts` can make regressions obvious during development without changing user-visible behavior.

### Suggested pattern

```ts
const ANALYSIS_DIAG =
  typeof window !== "undefined" &&
  Boolean((window as unknown as { __TA_ANALYSIS_DIAG__?: boolean }).__TA_ANALYSIS_DIAG__);

function diagPhase(label: string, startedAt: number, extra: Record<string, unknown> = {}): void {
  if (!ANALYSIS_DIAG || typeof performance === "undefined") return;
  const elapsedMs = performance.now() - startedAt;
  console.debug(`[analysis-diag] ${label}`, { elapsedMs, ...extra });
}
```

Use it around:

- `syncWorkflowArtifactsFromLegacyRuntime()`
- `refreshAnalysisFromWorkflowRun()`
- `runVueDrillPipeline()`
- `refreshHeatmapFromWorkflowRun()`
- `refreshCompareSummary()`

### Why I like this

It is cheap, reversible, and helps confirm whether the frontend changes are working before you build new benchmark harnesses.

---

## 3.4 Consider off-main-thread canvas work only **after** the smaller fixes land

### Why this matters

Worker/OffscreenCanvas rendering sounds attractive, but it is a larger engineering jump.

The current files still contain simpler wins that should land first:

- no-op sync
- top-table dedupe
- local-stability request correction
- hidden-render suppression
- in-flight dedupe
- stale-sequence suppression

If heatmap or Pareto remain visibly expensive after those fixes, then off-main-thread canvas work becomes more compelling.

---

# 4. Revised priority order

## P0 — correctness / parity

1. propagate `score_mode` into local-stability requests where supported
2. use `objectiveOf(runReq)` rather than only `text(runReq.objective)`
3. add stale-sequence guards after price-bar fetch and before drill DOM/status writes

## P1 — warm-path performance

4. add in-flight dedupe for drilldown, stability, price bars, and baseline loads
5. keep the existing no-op artifact sync work from the original plan
6. keep the existing top-table dedupe work from the original plan

## P2 — defensive cache correctness

7. add `invalidateAnalysisExecutionCaches()` now
8. wire it into data-load/resample paths later

## P3 — optional extensions

9. hidden Pareto render suppression
10. combined follow-up backend command
11. backend latest-artifact head/digest path
12. dev-only app-path timing instrumentation

---

# 5. Additional implementation checklist

## 5.1 Must-add items

- [ ] Update `buildLocalStabilityInvocation()` to propagate `objectiveOf(runReq)`
- [ ] Propagate `score_mode` for Double-MA local-stability requests
- [ ] Verify whether `CompositeCpuLocalStabilityRequest` also supports `score_mode`
- [ ] Add a unit test covering local-stability `score_mode` propagation
- [ ] Add `if (seq !== drillSeq) return true;` after `fetchDrillPriceBars()`
- [ ] Add a second stale-sequence guard before drill-side status/meta mutation
- [ ] Manually verify that quick row changes do not flash stale drill content

## 5.2 Strongly recommended performance items

- [ ] Add a generic pending-promise dedupe helper for async cached loads
- [ ] Apply it to drilldown compute requests
- [ ] Apply it to local-stability compute requests
- [ ] Apply it to `fetchDrillPriceBars()`
- [ ] Apply it to `loadBaselineFromDisk()`
- [ ] Decide whether `syncWorkflowArtifactsFromLegacyRuntime()` also needs serialization / in-flight dedupe
- [ ] Add at least one test or manual diagnostic proving duplicate in-flight requests are coalesced

## 5.3 Defensive cache hygiene

- [ ] Export `invalidateAnalysisExecutionCaches()` from `analysisService.ts`
- [ ] Identify the right data-load / resample call sites for that invalidation hook
- [ ] Decide whether baseline-artifact cache should remain independent from active-dataset invalidation

## 5.4 Optional next-step work

- [ ] Add Pareto render signature / hidden-render suppression backlog item
- [ ] Decide whether to add a combined follow-up command in Tauri
- [ ] Decide whether to add a backend “artifact head / digest” command for cheap no-op sync
- [ ] Add dev-only timing logs for analysis hydration and drill follow-ups
- [ ] Re-evaluate worker/offscreen canvas only after the smaller fixes land and are measured

---

# 6. Bottom line

The original Bundle 13 plan is still directionally correct.

But I would now explicitly add these three items near the top of the queue:

1. **propagate `score_mode` into local-stability requests**
2. **add stale-sequence guards after price-bar fetch in `runVueDrillPipeline()`**
3. **deduplicate identical in-flight async work**

Those three additions improve correctness and responsiveness without changing product behavior or removing features.
