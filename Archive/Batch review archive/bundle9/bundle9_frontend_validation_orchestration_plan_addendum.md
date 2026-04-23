# Bundle 9 plan addendum: extra performance, correctness, and low-risk durability improvements

## What this addendum is

This addendum supplements the original `bundle9_frontend_validation_orchestration_plan.md`.

The original plan already covered the main bundle-9 issues:
- mislabeled validation timing,
- dropped `dataset_meta`,
- duplicate artifact persistence on auto-export,
- fallback XLSX `max_rows` drift,
- preview/post-run/export route-summary drift,
- duplicated shortlist semantics,
- and the dead `topRowsFiltered` branch.

What follows are the **extra items I would now add or strengthen** if your top priorities remain performance, optimization, and correctness **without removing features or breaking functionality**.

---

## 1. Add an explicit export follow-up action result so you do not double-reveal on fallback

### Why this matters

`exportReportXlsx()` always calls `openPathWithRevealFallback(path)`, and then, when `autoReveal` is enabled, it calls `reveal_path` again. The same pattern exists on the Stage 3 fallback/open branch.

That means:
- when `open_path` fails and `openPathWithRevealFallback()` already called `reveal_path`, the later `autoReveal` block can call `reveal_path` a second time,
- and when `open_path` succeeds and `autoReveal=true`, you are intentionally doing both actions, but the code cannot currently distinguish that from the failure path.

This is not the biggest latency issue in the bundle, but it is real redundant OS follow-up work on the export path.

### Recommendation

Keep the current product behavior available, but make the helper return what it actually did.

### Code change

```ts
interface OpenPathOutcome {
  mode: 'opened' | 'revealed'
}

async function openPathWithRevealFallback(path: string): Promise<OpenPathOutcome> {
  const nextPath = normalizeText(path)
  if (!nextPath) return { mode: 'revealed' }
  try {
    await invokeCommand('open_path', { path: nextPath })
    return { mode: 'opened' }
  } catch {
    await invokeCommand('reveal_path', { path: nextPath })
    return { mode: 'revealed' }
  }
}
```

Then in `exportReportXlsx()`:

```ts
const openOutcome = await openPathWithRevealFallback(path)
if (autoReveal && openOutcome.mode !== 'revealed') {
  await invokeCommand('reveal_path', { path }).catch(() => undefined)
}
```

And on the Stage 3 branch:

```ts
const openOutcome = await openPathWithRevealFallback(stage3Path)
if (autoReveal && openOutcome.mode !== 'revealed') {
  await invokeCommand('reveal_path', { path: stage3Path }).catch(() => undefined)
}
```

### Tests to add

```ts
it('does not call reveal_path twice when open_path already failed and fallback reveal succeeded', async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'open_path') throw new Error('open failed')
    if (command === 'reveal_path') return true
    if (command === 'save_report_xlsx') return { path: 'runs/report.xlsx' }
    return {}
  })

  ;(pinia.state.value as Record<string, unknown>).validation = {
    autoRevealReport: true,
  }

  useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
    currentRun: {
      ts: '2026-04-01T00:00:00Z',
      req: { strategy_kind: 'DoubleMa', data_id: 'x' },
      res: { top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'sma', fast_len: 10, slow_len: 30 } }] },
    },
    currentValidation: {
      ts: '2026-04-01T00:00:00Z',
      req: { validation_context: { strategy_kind: 'DoubleMa' } },
      res: { candidates: [] },
    },
  })

  await exportReportXlsx()

  const revealCalls = invokeCommandMock.mock.calls.filter(([command]) => command === 'reveal_path')
  expect(revealCalls).toHaveLength(1)
})
```

### Priority

Low-medium. Safe and worthwhile, but after the main timing / metadata / single-save fixes.

---

## 2. Reuse the existing artifact `datasetMeta` when persisting validation artifacts

### Why this matters

`persistValidationArtifact()` currently rebuilds `dataset_meta` from `csvPath` plus `buildRunDatasetContextState()` every time it saves. That is better than dropping metadata completely, but it is still not the same as reusing the already-normalized artifact metadata from `workflowArtifactsStore.currentRun.datasetMeta` when it exists.

If the original run artifact carried richer dataset context than the currently loaded store state, re-deriving can silently flatten or alter the saved metadata.

### Recommendation

Prefer the existing artifact metadata first, and only rebuild when it is missing.

### Code change

```ts
async function persistValidationArtifact(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  options: { reportXlsxPath?: string; saveArtifact?: boolean } = {},
): Promise<PersistValidationArtifactResult> {
  const currentRun = readWorkflowRunArtifactFromStore()
  const dataStore = readPiniaStoreState('data')
  const ts = normalizeText(currentRun?.ts) || new Date().toISOString()
  const csvPath = normalizeText(currentRun?.csvPath || dataStore?.csvPath)
  const reportXlsxPath = normalizeText(options.reportXlsxPath || currentRun?.reportXlsxPath)
  const tags = currentRun?.tags ?? []

  const datasetMeta =
    currentRun?.datasetMeta && typeof currentRun.datasetMeta === 'object'
      ? currentRun.datasetMeta
      : buildDatasetMetaForArtifact(csvPath, buildRunDatasetContextState())

  // ... rest unchanged ...
}
```

### Test to add

```ts
it('reuses existing workflow artifact datasetMeta when persisting validation artifacts', async () => {
  useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
    currentRun: {
      ts: '2026-04-01T00:00:00Z',
      csvPath: 'C:/data/AAPL.csv',
      datasetMeta: {
        csv_path: 'C:/data/AAPL.csv',
        timeframe: '1h',
        symbol: 'AAPL',
        source_identity: 'artifact-rich-meta',
      },
      req: {
        strategy_kind: 'DoubleMa',
        data_id: 'aapl_1h',
      },
      res: {
        top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'sma', fast_len: 10, slow_len: 30 } }],
      },
    },
    currentValidation: null,
  })

  await runValidation({ switchTab: false })

  const saveCall = invokeCommandMock.mock.calls.find(([command]) => command === 'save_run_artifact')
  const artifact = (saveCall?.[1] as { artifact?: Record<string, unknown> } | undefined)?.artifact
  expect(artifact?.dataset_meta).toMatchObject({
    source_identity: 'artifact-rich-meta',
  })
})
```

### Priority

Medium. Strong correctness hardening with very low regression risk.

---

## 3. Add preview-summary memoization or staged recomputation

### Why this matters

`ValidationWorkspacePanel.vue` computes `expectedExecutionSummary` directly from `previewValidationExecutionSummaryFromStores()`. That preview builder re-reads store state, rebuilds validation inputs, calls `buildValidationRequest()`, and then calls `buildValidationCapabilitySummary()` each time it recomputes.

That is correct, but it also means every relevant reactive change on the validation form re-runs the full preview pipeline.

This is probably not your dominant latency, but it is a real frontend recompute surface and a good optional polish target once the run path is fixed.

### Recommendation

Memoize by a small deterministic signature made from:
- current run artifact timestamp/path,
- strategy kind,
- validation form fields that affect the preview,
- selected cross datasets.

### Code pattern

```ts
interface PreviewValidationSummaryCacheEntry {
  key: string
  value: ReturnType<typeof buildValidationCapabilitySummary>
}

let previewValidationSummaryCache: PreviewValidationSummaryCacheEntry | null = null

function buildPreviewValidationSummaryKey(
  workflowRun: WorkflowRunArtifactSnapshot | null,
  runReq: Record<string, unknown> | null,
): string {
  return JSON.stringify({
    runTs: workflowRun?.ts || '',
    artifactPath: workflowRun?.artifactPath || '',
    strategyKind: normalizeStrategyKindRaw(runReq?.strategy_kind ?? runReq?.strategyKind),
    candidates: readStoreTextInputValue('valCandidates'),
    seed: readStoreTextInputValue('valSeed'),
    minPsr: readStoreTextInputValue('valMinPsr'),
    minDsr: readStoreTextInputValue('valMinDsr'),
    minCscvRank: readStoreTextInputValue('valMinCscvRank'),
    minCscvMetric: readStoreTextInputValue('valMinCscvMetric'),
    maxPbo: readStoreTextInputValue('valMaxPbo'),
    spaEnable: readStoreCheckboxValue('valSpaEnable'),
    reoptWfEnable: readStoreCheckboxValue('valReoptWfEnable'),
    wfEnable: readStoreCheckboxValue('valWfEnable'),
    perturbEnable: readStoreCheckboxValue('valPerturbEnable'),
    mcEnable: readStoreCheckboxValue('valMcEnable'),
    cscvEnable: readStoreCheckboxValue('valCscvEnable'),
    crossEnable: readStoreCheckboxValue('valCrossEnable'),
    crossMode: readStoreTextInputValue('valCrossMode'),
    crossMax: readStoreTextInputValue('valCrossMax'),
    crossDatasets: readVueCrossSelection(),
  })
}
```

Then inside `previewValidationExecutionSummaryFromStores()`:

```ts
const cacheKey = buildPreviewValidationSummaryKey(workflowRun, runReq)
if (previewValidationSummaryCache?.key === cacheKey) {
  return previewValidationSummaryCache.value
}

const summary = buildValidationCapabilitySummary(validationReq)
previewValidationSummaryCache = { key: cacheKey, value: summary }
return summary
```

### Caution

Do **not** do this until the route-policy helper extraction is settled, otherwise you will memoize drift more efficiently.

### Priority

Optional. Good polish, not a phase-1 fix.

---

## 4. Widen the shared helper extraction beyond route strings only

### Why this matters

The current duplication is not just route-detail prose. `validationService.ts` and `validationRequestBuilder.ts` both carry their own copies of:
- `normalizeStrategyKindRaw()`
- `isDoubleLikeKind()`
- composite GPU route detail strings
- auto-CPU preference strings
- execution-category classification
- bootstrap summary helpers

The export layer in `commands_reports_validation_common.rs` duplicates a similar policy set again.

If you only extract the prose strings, drift can still survive in:
- strategy-kind normalization,
- category summary classification,
- aux CPU summary logic,
- and CPU-preferred/fallback classification.

### Recommendation

Promote the optional helper into a slightly broader shared policy module, not just a string table.

### Suggested frontend helper shape

```ts
export interface ValidationRoutePolicySummary {
  strategyKind: string
  isDoubleLike: boolean
  routeDetail: string
  autoCpuPreference: string
  categorySummaryForPreview: (req: ValidationRequestPayload) => string
  categorySummaryForActual: (validationReq: Record<string, unknown>, result: Record<string, unknown>) => string
  bootstrapSummary: (tradeBootstrap: unknown) => string
}

export function normalizeValidationStrategyKind(kindRaw: unknown): string {
  // single canonical implementation
}

export function isDoubleLikeValidationStrategy(kindRaw: unknown): boolean {
  // single canonical implementation
}

export function validationRoutePolicyForStrategy(strategyKindRaw: unknown): ValidationRoutePolicySummary {
  // one place for notes and category rules
}
```

### Suggested scope rule

Do this in TypeScript first.

Only move it to a backend-owned contract later if you explicitly want the backend to be the single source of truth for route metadata.

### Priority

Medium. This is the best durability improvement after the correctness fixes.

---

## 5. Add phase-level timing buckets, not just end-to-end and command time

### Why this matters

The original plan already fixes the incorrect "frontend" timing label. The next improvement is to make regressions diagnosable.

Today, if validation gets slower, you still will not know whether the regression came from:
- request build,
- command execution,
- shortlist computation,
- table row shaping,
- artifact persistence,
- or export/open/reveal follow-up.

### Recommendation

Add lightweight internal timing buckets and keep them out of the visible UI by default.

### Suggested shape

```ts
interface ValidationPhaseTimings {
  requestBuildMs: number
  commandMs: number
  shortlistMs: number
  tableMs: number
  persistMs: number
  applyArtifactsMs: number
  exportMs?: number
  endToEndMs: number
}
```

### Storage recommendation

Either:
- stash under a dev-only `validationLastTiming` store field, or
- attach as a hidden debug field in tests only.

### Test recommendation

Do **not** assert concrete durations. Only assert that:
- the keys exist,
- values are finite,
- and `endToEndMs >= commandMs`.

### Priority

Optional but very valuable if you expect more performance tuning after this bundle.

---

## 6. Strengthen the auto-export regression tests beyond the current plan

### Why this matters

The original plan covered persistence dedupe and fallback `max_rows`, but the tests should also nail the surrounding orchestration contract.

### Add these tests

#### A. Successful validation with `autoExport=false`

Verify:
- one `save_run_artifact`
- zero `save_report_xlsx`
- no open/reveal calls

#### B. Successful validation with `autoExport=true`

Verify:
- exactly one artifact persistence after the refactor
- one report save
- expected open/reveal behavior
- workflow artifact store contains the final `reportXlsxPath`

#### C. Report save fails, fallback download succeeds

Verify:
- no second artifact persistence
- status line explains that XLSX was downloaded rather than saved to a path
- `reportXlsxPath` remains empty rather than stale

#### D. Artifact persistence fails but validation succeeds

Verify:
- validation rows and summaries still populate
- a non-fatal persistence warning is surfaced
- workflow artifact store still updates using the best available in-memory data

#### E. `loadSavedRun()` preserves `datasetMeta`

Verify:
- `workflowArtifactsStore.currentRun.datasetMeta` survives the load path
- preview summary and rerun use it afterward

### Priority

High for test coverage, even though the production code change is optional.

---

## 7. Add a richer artifact dataset-context snapshot as an optional future step

### Why this matters

The current artifact writes `dataset_meta`, which is necessary and already helpful. But the report payload also distinguishes:
- `meta`
- `ctx`
- base/current dataset context forms

If you want full replay/export fidelity without depending on the current runtime data store, the next contract step is to persist a richer dataset context snapshot alongside `dataset_meta`.

### Optional artifact extension

```ts
artifact.data = {
  csv_path: csvPath,
  dataset_ctx: buildRunDatasetContextState().currentDatasetCtx,
  data_meta: buildRunDatasetContextState().currentDataMeta,
  base_dataset_ctx: buildRunDatasetContextState().currentBaseDatasetCtx,
  base_data_meta: buildRunDatasetContextState().currentBaseDataMeta,
}
```

Then `buildCurrentReportPayloadFromStore()` can prefer:
1. persisted artifact dataset context,
2. persisted artifact `datasetMeta`,
3. current runtime store fallback.

### Caution

This is an artifact-contract change. Version it carefully.

### Priority

Optional / future. Strong correctness value, but larger surface area.

---

## 8. Beyond the prompt scope, but still aligned with your goals

These are not things I would fold into the first bundle-9 patch series, but they are good next-step reviews if performance remains the main goal.

### A. Tie the frontend changes into the existing perf harnesses

The review map explicitly says validation and workflow performance should stay benchmark- and gate-aware. After the bundle-9 fixes land, validate them against the existing workflow and app-path harnesses rather than inventing new bespoke evidence first.

Recommended evidence path after implementation:
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### B. Only if validation still feels slow: review bundle 14 for cross-refresh/data load coupling

If the user-visible delay is actually coming from cross-dataset refresh, dataset metadata loading, or CSV/cache interaction, the next review should move into the data load/resample/cache bundle rather than over-optimizing validation orchestration code.

### C. Only if route drift keeps reappearing: move route metadata ownership closer to the backend

This is the bigger optional move:
- have the backend return canonical route/category/note metadata,
- keep the frontend as a pure renderer,
- and keep export reading that same contract.

That is cleaner, but it is a larger contract change than the first-pass fixes.

---

## Revised implementation order with these additions

### Phase 1
- fix timing boundary labels
- propagate `datasetMeta`
- prefer existing artifact `datasetMeta` when re-persisting
- remove duplicate artifact save on auto-export
- pass `max_rows` through the fallback XLSX path

### Phase 2
- unify preview/post-run/export route policy
- widen the shared helper extraction beyond prose strings
- update VCC/CARM/CDMC/VCET/CompositeMo tests

### Phase 3
- add auto-export regression tests for open/reveal, persistence warnings, and fallback behavior
- avoid duplicate reveal calls when fallback already revealed the path

### Phase 4
- optional preview memoization
- optional phase timing buckets
- optional richer artifact dataset context snapshot

---

## Extra checklist items to append to the original plan

### Add to “Must do” if you want the stronger correctness pass

- [ ] Prefer `workflowArtifactsStore.currentRun.datasetMeta` over recomputing fresh `dataset_meta` during validation artifact persistence.
- [ ] Preserve current behavior while avoiding duplicate `reveal_path` calls when `openPathWithRevealFallback()` already revealed the file.
- [ ] Add `loadSavedRun()` regression coverage that proves `datasetMeta` survives reload.
- [ ] Add auto-export success/failure orchestration tests, not just request-shape tests.

### Add to “Strongly recommended”

- [ ] Extract shared validation strategy/route/category helpers, not only shared route-note strings.
- [ ] Add internal validation phase timing buckets for request build / command / shortlist / table / persistence / export.

### Add to “Optional / phase 2+”

- [ ] Memoize `previewValidationExecutionSummaryFromStores()` by a deterministic request signature.
- [ ] Persist a richer dataset context snapshot in the artifact for replay/export fidelity.
- [ ] Re-run bundle-7 workflow/app-path harnesses after landing the bundle-9 changes.

---

## Bottom line

I do **not** think the original plan missed any of the major bundle-9 defects.

What I would say it **underweighted** were:
1. **reusing existing artifact metadata instead of re-deriving it**,
2. **the open/reveal follow-up contract on export paths**,
3. **broader shared-helper extraction beyond route prose**,
4. and **phase-level timing visibility** for future regression diagnosis.

Those are the best extra additions if your goal stays: faster validation flows, lower orchestration waste, and stronger replay/export correctness without removing functionality.
