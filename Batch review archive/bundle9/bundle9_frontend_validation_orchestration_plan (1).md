
# Bundle 9 implementation plan: Frontend validation orchestration and artifact-heavy validation flows

## Scope alignment

This plan is intentionally scoped to the bundle-9 surface from `gpt54_feature_review_map.md`: validation execution cost, route-summary fidelity, shortlist shaping, artifact persistence, export-adjacent follow-up work, and saved-run replay drift.

It does **not** propose a new validation UI, new strategy features, or a broad frontend/backend redesign. The canonical validation surface remains `ValidationWorkspacePanel.vue`, and the recommended changes below are parity-safe, benchmark-aware, and focused on removing confirmed waste or correctness drift.

## Files in scope

Primary files covered in this plan:

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

Adjacent files that are **not** in the bundle but are worth touching if you choose the more durable options:

- `web/src/services/historyService.test.ts` *(new)*
- `web/src/services/validationRoutePolicy.ts` *(new shared TS helper, optional but recommended)*
- the Tauri command wrapper that exposes `build_report_xlsx` to the frontend *(not included in the uploaded bundle, but it must accept `max_rows` if you implement the fallback cap fix)*
- `src-tauri/src/commands_reports_validation_xlsx.rs` *(only if you later decide to surface new `actual_*` fields such as `actual_category_summary` in the workbook UI)*

## Executive summary

The highest-priority fixes are:

1. **Fix the validation timing boundary in `runValidation()`** so the UI stops labeling command-only time as “frontend”.
2. **Propagate `dataset_meta` end-to-end** so replay, rerun, and report export continue to use the artifact’s dataset context instead of silently falling back to the current runtime dataset state.
3. **Remove the duplicate run-artifact write from the auto-export success path** so “validation + auto-export” only persists the artifact once.
4. **Pass `max_rows` through the XLSX fallback path** so the degraded path does not produce a different workbook shape or do more work than the normal path.
5. **Unify route-policy text and category classification across preview, post-run, and export paths**, especially for `VCC`, `CARM`, `CDMC`, `VCET`, and `CompositeMo`.

If you only implement one pass, do them in exactly that order.

---

## 1. Surface understanding

### Canonical UI surface

`ValidationWorkspacePanel.vue` is still the canonical validation launch/report surface. It renders the pre-run execution preview via `previewValidationExecutionSummaryFromStores()` and shows the “Expected Route / Cross / Modules / Categories / Skips / Notes” block at `ValidationWorkspacePanel.vue:609-633`.

That means:
- preview fidelity matters,
- preview drift is user-visible before launch,
- and the fix should land in the service layer first so the panel can stay mostly unchanged.

### Frontend orchestration owner

`validationService.ts` is the actual orchestration center:

- artifact/report payload building: `buildCurrentReportPayloadFromStore()` (`validationService.ts:463-491`)
- dataset-match enforcement: `assertValidationRunMatchesCurrentDataset()` (`validationService.ts:613-649`)
- validation artifact persistence: `persistValidationArtifact()` (`validationService.ts:1514-1559`)
- live store update after validation: `applyVueValidationArtifacts()` (`validationService.ts:1561-1593`)
- validation launch flow: `runValidation()` (`validationService.ts:1610-1813`)
- preview summary flow: `previewValidationExecutionSummaryFromStores()` (`validationService.ts:1815-1905`)
- report export flow: `exportReportXlsx()` (`validationService.ts:1945-2013`)

### Preview-side request/expectation owner

`validationRequestBuilder.ts` owns preview-time request shaping and capability summaries:

- backend normalization: `normalizeValidationBackend()` (`validationRequestBuilder.ts:267-282`)
- route detail / auto-CPU notes: `compositeProfileGpuRouteNote()` and `compositeProfileAutoCpuPreferenceNote()` (`validationRequestBuilder.ts:351-400`)
- expected category summary builder: `buildValidationCategorySummaryForMode()` (`validationRequestBuilder.ts:455-494`)
- preview summary builder: `buildValidationCapabilitySummary()` (`validationRequestBuilder.ts:496-616`)
- validation context + candidate shaping: `buildValidationRequest()` (`validationRequestBuilder.ts:618-962`)

### Replay/store owner

`workflowArtifactsStore.ts` already supports `datasetMeta` at the normalized run-artifact level (`workflowArtifactsStore.ts:5-15`, `133-167`), and `historyService.ts` already reads `dataset_meta` for detail and rerun flows (`historyService.ts:914-915`, `935-944`, `1024-1048`).

That is important because the repo already has the right **artifact contract**; the main drift is that some hot paths do not re-apply that contract back into the live store.

### Backend/export context

`commands_validation.rs` already enforces validation context against the loaded dataset and timeframe through `validate_validation_context()` (`commands_validation.rs:131-182`). That guard is good and should **not** be loosened.

`commands_reports.rs` and `commands_reports_validation_common.rs` matter because:
- they are on the export path,
- they shape workbook expectations,
- and they currently duplicate route-summary and shortlist logic in ways that can drift from the frontend.

---

## 2. Findings

### Finding 1 — `runValidation()` labels command-only time as frontend/end-to-end time

- **Severity:** high  
- **Type:** benchmark-fidelity  
- **Location:** `validationService.ts` — `runValidation()`  
- **Timing boundary affected:** app-path  
- **Why it matters:** `runValidation()` starts timing immediately before `invokeCommand(...)` and stops immediately after the command returns (`validationService.ts:1731-1734`), but then it performs shortlist building, table shaping, artifact persistence, workflow-artifact application, status updates, DOM writes, and optional tab switching afterward (`validationService.ts:1735-1780`). The current status string, `Validation done in Xms (frontend)`, is therefore mislabeled. It is **command roundtrip time**, not full frontend/app-path time.  
- **Recommended change:** measure and label at least two timings:
  - `commandMs`
  - `validationEndToEndMs`

  Optionally add `exportFollowupMs` when auto-export is enabled.

#### Required code edits

##### `validationService.ts` — add explicit timing helpers

```ts
function nowMs(): number {
  return typeof performance !== 'undefined' ? performance.now() : Date.now()
}

function formatValidationStatus(commandMs: number, endToEndMs: number): string {
  return `Validation done in ${endToEndMs.toFixed(0)}ms end-to-end (command ${commandMs.toFixed(0)}ms).`
}

function formatValidationFollowupStatus(
  baseStatus: string,
  exportMs: number,
  reportPath: string,
  downloaded: boolean,
): string {
  if (reportPath) return `${baseStatus} XLSX follow-up ${exportMs.toFixed(0)}ms • ${reportPath}`
  if (downloaded) return `${baseStatus} XLSX follow-up ${exportMs.toFixed(0)}ms • download fallback`
  return `${baseStatus} XLSX follow-up ${exportMs.toFixed(0)}ms`
}
```

##### `validationService.ts` — replace the current timer block in `runValidation()`

```ts
const overallStartedAt = nowMs()
const commandStartedAt = nowMs()
const validationRes = await invokeCommand(command, { req: validationReq })
const commandFinishedAt = nowMs()

// ... shortlist/table/store work ...

const validationEndToEndMs = Math.max(0, nowMs() - overallStartedAt)
const commandMs = Math.max(0, commandFinishedAt - commandStartedAt)
let statusLine = formatValidationStatus(commandMs, validationEndToEndMs)
```

#### Test updates

Add a non-brittle assertion in `validationService.test.ts`:

```ts
expect(String(validationState.statusLine || '')).toContain('end-to-end')
expect(String(validationState.statusLine || '')).toContain('command')
```

Do **not** assert exact numbers.

---

### Finding 2 — `dataset_meta` is saved, but then dropped on apply/load/report paths

- **Severity:** high  
- **Type:** correctness  
- **Location:**  
  - `validationService.ts` — `persistValidationArtifact()` / `applyVueValidationArtifacts()` / `buildCurrentReportPayloadFromStore()`  
  - `historyService.ts` — `loadSavedRun()`  
  - `workflowArtifactsStore.ts` — normalization contract already supports it  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  - `persistValidationArtifact()` writes `dataset_meta` into the artifact (`validationService.ts:1533-1545`)
  - `workflowArtifactsStore.ts` already knows how to hydrate it (`workflowArtifactsStore.ts:133-167`)
  - but `applyVueValidationArtifacts()` does **not** pass it back into `currentRun` (`validationService.ts:1574-1593`)
  - and `loadSavedRun()` in `historyService.ts` also omits it (`historyService.ts:870-890`)
  - `buildCurrentReportPayloadFromStore()` then rebuilds report dataset context from the **current runtime data store** instead of the artifact dataset metadata (`validationService.ts:463-491`)

  This creates replay drift:
  - validation/rerun dataset checks become less reliable after reload,
  - report export can silently reflect the current loaded dataset rather than the artifact’s dataset context,
  - and the repo loses information it already saved correctly.
- **Recommended change:** carry `datasetMeta` through:
  1. persistence return values,
  2. live store apply,
  3. saved-run reload,
  4. report payload construction.

#### Required code edits

##### `validationService.ts` — return `datasetMeta` from persistence

```ts
interface PersistValidationArtifactResult {
  path: string
  ts: string
  csvPath: string
  reportXlsxPath: string
  tags: string[]
  datasetMeta: Record<string, unknown> | null
  saved: boolean
  persistWarning: string
}
```

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
  const datasetMeta = buildDatasetMetaForArtifact(csvPath, buildRunDatasetContextState())
  const configFingerprint = computeConfigFingerprint(runReq, validationReq)

  const artifact: Record<string, unknown> = {
    version: 1,
    kind: 'vectorgrid_run_artifact',
    ts,
    config_fingerprint: configFingerprint,
    run: { req: runReq, res: runRes },
    validation: { req: validationReq, res: validationRes },
    report_xlsx_path: reportXlsxPath,
    data: { csv_path: csvPath },
    dataset_meta: datasetMeta,
    tags,
  }

  let path = normalizeText(currentRun?.artifactPath)
  let saved = false
  let persistWarning = ''

  if (options.saveArtifact !== false) {
    try {
      const savedArtifact = (await invokeCommand('save_run_artifact', {
        artifact,
      })) as SaveRunArtifactResponse
      const next = normalizeText(savedArtifact?.path)
      if (next) path = next
      saved = true
    } catch (error) {
      persistWarning = String(error || '').trim() || 'artifact persistence failed'
    }
  }

  return { path, ts, csvPath, reportXlsxPath, tags, datasetMeta, saved, persistWarning }
}
```

##### `validationService.ts` — apply `dataset_meta` back into the workflow store

```ts
function applyVueValidationArtifacts(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  persisted: PersistValidationArtifactResult,
): void {
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: persisted.ts,
        artifact_path: persisted.path,
        report_xlsx_path: persisted.reportXlsxPath,
        csv_path: persisted.csvPath,
        dataset_meta: persisted.datasetMeta,
        tags: persisted.tags,
        req: runReq,
        res: runRes,
      },
      currentValidation: {
        ts: persisted.ts,
        req: validationReq,
        res: validationRes,
      },
    },
    'vue',
  )
}
```

##### `historyService.ts` — preserve `dataset_meta` when loading a saved run

```ts
export async function loadSavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifact(path)
  const nextPath = loaded.path
  const record = loaded.record

  const datasetMeta =
    record.dataset_meta && typeof record.dataset_meta === "object"
      ? (record.dataset_meta as Record<string, unknown>)
      : null

  // ... existing req/res extraction ...

  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: nextPath,
        report_xlsx_path: reportXlsxPath,
        csv_path: csvPath,
        dataset_meta: datasetMeta,
        tags,
        req: runReq,
        res: runRes,
      },
      currentValidation: validationRes
        ? { ts, req: validationReq, res: validationRes }
        : null,
    },
    "vue",
  )
}
```

##### `validationService.ts` — prefer artifact dataset metadata for report export

```ts
function buildReportDataFromRunArtifact(run: WorkflowRunArtifactSnapshot | null): {
  meta: Record<string, unknown> | null
  ctx: Record<string, unknown>
} {
  const fallback = buildRunDatasetContextState()
  const datasetMeta = run?.datasetMeta
  if (!datasetMeta) {
    return {
      meta: fallback.currentDataMeta,
      ctx: fallback.currentDatasetCtx,
    }
  }

  const ctx: Record<string, unknown> = { ...fallback.currentDatasetCtx }
  const timeframe = firstNonEmptyText(
    datasetMeta.timeframe,
    datasetMeta.derived_timeframe,
    datasetMeta.base_timeframe,
    ctx.timeframe,
  )
  const label = firstNonEmptyText(datasetMeta.label, datasetMeta.symbol, ctx.label)
  const symbol = firstNonEmptyText(datasetMeta.symbol, datasetMeta.label, ctx.symbol)

  if (label) ctx.label = label
  if (symbol) ctx.symbol = symbol
  if (timeframe) {
    ctx.timeframe = timeframe
    ctx.base_timeframe = timeframe
    ctx.derived_timeframe = timeframe
    ctx.resample_applied = false
  }

  return {
    meta: datasetMeta,
    ctx,
  }
}
```

```ts
function buildCurrentReportPayloadFromStore() {
  const run = readWorkflowRunArtifactFromStore()
  const validation = readWorkflowValidationArtifactFromStore()
  const ts = normalizeText(run?.ts || validation?.ts) || new Date().toISOString()
  const reportData = buildReportDataFromRunArtifact(run)

  return {
    version: 1,
    kind: 'vectorgrid_report',
    ts,
    data: {
      meta: reportData.meta,
      ctx: reportData.ctx,
    },
    run: { req: run?.req ?? null, res: run?.res ?? null },
    validation: validation ? { req: validation.req ?? null, res: validation.res ?? null } : null,
  }
}
```

#### Test updates

##### `validationService.test.ts` — assert dataset metadata survives the validation run

```ts
;(pinia.state.value as Record<string, unknown>).data = {
  csvPath: 'C:/data/AAPL.csv',
  dataTimeframe: '1h',
  baseDataMeta: { timeframe: '1h' },
  datasetLabel: 'AAPL',
  datasetContext: 'source=csv',
  dataFields: ['open', 'high', 'low', 'close', 'volume'],
  dataLen: '1000',
}

await runValidation({ switchTab: false })

const workflowStore = useWorkflowArtifactsStore(pinia)
expect(workflowStore.currentRun?.datasetMeta).toEqual(
  expect.objectContaining({
    csv_path: 'C:/data/AAPL.csv',
    timeframe: '1h',
  }),
)
```

##### New adjacent file recommended: `historyService.test.ts`

```ts
it('preserves dataset_meta when loading a saved run artifact', async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'load_run_artifact') {
      return {
        ts: '2026-03-08T18:00:00Z',
        csv_path: 'C:/data/AAPL.csv',
        dataset_meta: { csv_path: 'C:/data/AAPL.csv', timeframe: '15m', symbol: 'AAPL' },
        run: { req: { strategy_kind: 'DoubleMa' }, res: { top: [] } },
        validation: null,
        tags: [],
      }
    }
    return {}
  })

  await loadSavedRun('runs/example.json')

  const workflowStore = useWorkflowArtifactsStore(pinia)
  expect(workflowStore.currentRun?.datasetMeta).toEqual(
    expect.objectContaining({
      csv_path: 'C:/data/AAPL.csv',
      timeframe: '15m',
      symbol: 'AAPL',
    }),
  )
})
```

---

### Finding 3 — auto-export currently writes the run artifact twice on the success path

- **Severity:** high  
- **Type:** performance  
- **Location:** `validationService.ts` — `runValidation()` and `exportReportXlsx()`  
- **Timing boundary affected:** app-path  
- **Why it matters:**  
  - `runValidation()` always persists the artifact once before applying validation state (`validationService.ts:1747-1750`)
  - if auto-export is enabled, `exportReportXlsx()` may immediately persist the artifact again after it learns `reportXlsxPath` (`validationService.ts:2001-2004`)

  That means the “validation + auto-export” critical path includes:
  - validation result shaping,
  - artifact write #1,
  - workbook save,
  - artifact write #2,
  - open/reveal follow-up.
- **Recommended change:** on the auto-export path:
  1. apply validation state to the live store without saving the artifact first,
  2. run export,
  3. persist the artifact exactly once afterward with `reportXlsxPath` if a path exists,
  4. if export falls back to download or fails, persist once afterward without a report path.

#### Required code edits

##### `validationService.ts` — let `exportReportXlsx()` optionally skip artifact persistence and return a result

```ts
interface ExportReportXlsxResult {
  path: string
  downloaded: boolean
}

export async function exportReportXlsx(
  options: { persistArtifactWithReportPath?: boolean } = {},
): Promise<ExportReportXlsxResult> {
  const report = buildCurrentReportPayloadFromStore()
  const { autoReveal, maxRows } = readValidationExportPrefs()
  const ts = normalizeText(report.ts) || new Date().toISOString()
  const runReq = asRecord(report.run?.req)
  const runRes = report.run?.res ?? null
  const validationReq = asRecord(report.validation?.req ?? null)
  const validationRes = report.validation?.res ?? null

  if (!validationReq || validationRes == null) {
    const stage3Path = readLatestMetaReportPath()
    if (!stage3Path) {
      patchValidationStoreState({
        statusLine: 'Run validation first to export report XLSX (or run Meta Analysis to open Stage 3 XLSX).',
      })
      return { path: '', downloaded: false }
    }

    await openPathWithRevealFallback(stage3Path)
    if (autoReveal) {
      await invokeCommand('reveal_path', { path: stage3Path }).catch(() => undefined)
    }
    patchValidationStoreState({ statusLine: `Opened Stage 3 XLSX report: ${stage3Path}` })
    return { path: stage3Path, downloaded: false }
  }

  let path = ''
  try {
    const saved = (await invokeCommand('save_report_xlsx', {
      report,
      max_rows: maxRows,
    })) as SaveReportXlsxResponse
    path = normalizeText(saved?.path)
  } catch (saveError) {
    try {
      const base64 = await invokeCommand('build_report_xlsx', {
        report,
        max_rows: maxRows,
      })
      const downloaded = downloadBase64(
        `vectorgrid_report_${ts}.xlsx`,
        String(base64 || ''),
        'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      )
      patchValidationStoreState({
        statusLine: downloaded
          ? 'Exported XLSX report as a file download.'
          : 'Built XLSX report (download unavailable in this environment).',
      })
      return { path: '', downloaded }
    } catch (fallbackError) {
      const message = `Failed to export XLSX report: ${String(saveError)} | fallback failed: ${String(fallbackError)}`
      patchValidationStoreState({ statusLine: message })
      throw new Error(message)
    }
  }

  if (!path) {
    patchValidationStoreState({ statusLine: 'Saved XLSX report, but no path was returned.' })
    return { path: '', downloaded: false }
  }

  if (runReq && runRes != null && options.persistArtifactWithReportPath !== false) {
    const persisted = await persistValidationArtifact(
      runReq,
      runRes,
      validationReq,
      validationRes,
      { reportXlsxPath: path, saveArtifact: true },
    )
    applyVueValidationArtifacts(runReq, runRes, validationReq, validationRes, persisted)
  }

  patchValidationStoreState({ statusLine: `Saved XLSX report: ${path}` })
  await openPathWithRevealFallback(path)
  if (autoReveal) {
    await invokeCommand('reveal_path', { path }).catch(() => undefined)
  }

  return { path, downloaded: false }
}
```

##### `validationService.ts` — change `runValidation()` auto-export flow to persist once

```ts
const { autoExport } = readValidationExportPrefs()

// Build the live artifact state immediately, but skip disk persistence when auto-export is on.
const initialPersisted = await persistValidationArtifact(
  runReq,
  runRes,
  validationReq,
  validationRes,
  { saveArtifact: !autoExport },
)

applyVueValidationArtifacts(runReq, runRes, validationReq, validationRes, initialPersisted)

const commandMs = Math.max(0, commandFinishedAt - commandStartedAt)
const validationEndToEndMs = Math.max(0, nowMs() - overallStartedAt)
let statusLine = formatValidationStatus(commandMs, validationEndToEndMs)

if (initialPersisted.persistWarning) {
  statusLine += ` • artifact save warning: ${initialPersisted.persistWarning}`
}

patchValidationStoreState({
  // ... existing fields ...
  statusLine,
})

if (switchTab) selectWorkflowResultsTab('validate', true)

if (autoExport) {
  try {
    const exportStartedAt = nowMs()
    const exportResult = await exportReportXlsx({ persistArtifactWithReportPath: false })

    // Persist exactly once, after export, with the final report path if one exists.
    const finalPersisted = await persistValidationArtifact(
      runReq,
      runRes,
      validationReq,
      validationRes,
      {
        reportXlsxPath: exportResult.path || undefined,
        saveArtifact: true,
      },
    )
    applyVueValidationArtifacts(runReq, runRes, validationReq, validationRes, finalPersisted)

    const followupStatus = formatValidationFollowupStatus(
      statusLine,
      Math.max(0, nowMs() - exportStartedAt),
      exportResult.path,
      exportResult.downloaded,
    )
    patchValidationStoreState({ statusLine: followupStatus })
    writeDomTextContent('status', followupStatus, VALIDATION_PANEL_SELECTOR)
  } catch (error) {
    // Export failed; ensure the validation artifact still lands on disk once.
    const finalPersisted = await persistValidationArtifact(
      runReq,
      runRes,
      validationReq,
      validationRes,
      { saveArtifact: true },
    )
    applyVueValidationArtifacts(runReq, runRes, validationReq, validationRes, finalPersisted)

    const reason = String(error || '').trim() || 'unknown error'
    const exportStatus = `Validation complete, but auto-export XLSX failed: ${reason}`
    patchValidationStoreState({ statusLine: exportStatus })
    writeDomTextContent('status', exportStatus, VALIDATION_PANEL_SELECTOR)
  }
}
```

#### Test updates

##### `validationService.test.ts` — assert the auto-export success path only saves the run artifact once

```ts
await runValidation({ switchTab: false })

const artifactSaves = invokeCommandMock.mock.calls.filter(
  ([commandName]) => commandName === 'save_run_artifact',
)
expect(artifactSaves).toHaveLength(1)
```

##### `validationService.test.ts` — keep auto-export failure non-fatal, but still ensure the artifact is saved once

```ts
await expect(runValidation({ switchTab: false })).resolves.toBeUndefined()

const artifactSaves = invokeCommandMock.mock.calls.filter(
  ([commandName]) => commandName === 'save_run_artifact',
)
expect(artifactSaves.length).toBeGreaterThanOrEqual(1)
```

---

### Finding 4 — XLSX fallback export ignores `max_rows`

- **Severity:** medium  
- **Type:** performance / parity  
- **Location:**  
  - `validationService.ts` — `exportReportXlsx()`  
  - `commands_reports.rs` — `build_report_xlsx_blocking()`  
- **Timing boundary affected:** app-path  
- **Why it matters:** the primary save path passes `max_rows` to `save_report_xlsx` (`validationService.ts:1974-1977`), but the fallback path calls `build_report_xlsx` without it (`validationService.ts:1980-1986`). On the Rust side, `save_report_xlsx_blocking()` honors the cap (`commands_reports.rs:64-76`), while `build_report_xlsx_blocking()` hardcodes `None` (`commands_reports.rs:46-49`). The degraded path can therefore do more work and produce a different workbook shape than the primary path.  
- **Recommended change:** thread `max_rows` through the fallback path too.

#### Required code edits

##### `validationService.ts` — pass `max_rows` to `build_report_xlsx`

```ts
const base64 = await invokeCommand('build_report_xlsx', {
  report,
  max_rows: maxRows,
})
```

##### `commands_reports.rs` — accept `max_rows` in the base64 builder path

```rust
pub(crate) fn build_report_xlsx_blocking(
    report: Value,
    max_rows: Option<u32>,
) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let max_rows = max_rows.map(|n| n as usize).filter(|n| *n > 0);
    let bytes = commands_reports_validation_xlsx::build_validation_report_xlsx(&report, max_rows)?;
    Ok(general_purpose::STANDARD.encode(bytes))
}
```

##### Adjacent note

The Tauri command wrapper that exposes `build_report_xlsx` to the web layer must also accept `max_rows`. That wrapper is not in the uploaded bundle, but it must be updated for the frontend argument to take effect.

#### Test updates

##### `validationService.test.ts` — add a fallback-cap test

```ts
it('passes max_rows through the XLSX fallback path', async () => {
  ;(pinia.state.value as Record<string, unknown>).validation = {
    autoExportTopN: '7',
    autoRevealReport: false,
  }

  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'save_report_xlsx') throw new Error('save failed')
    if (command === 'build_report_xlsx') return 'AA=='
    if (command === 'open_path' || command === 'reveal_path') return true
    return {}
  })

  useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
    currentRun: {
      ts: '2026-02-25T12:00:00Z',
      req: { strategy_kind: 'DoubleMa', data_id: 'data-xlsx', ma_source: 'close' },
      res: { top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 } }] },
    },
    currentValidation: {
      ts: '2026-02-25T12:00:00Z',
      req: {
        data_id: 'data-xlsx',
        validation_context: { strategy_kind: 'DoubleMa' },
        candidates: [{ fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 }],
      },
      res: { candidates: [{ score: 1 }] },
    },
  })

  await exportReportXlsx()

  expect(invokeCommandMock).toHaveBeenCalledWith(
    'build_report_xlsx',
    expect.objectContaining({
      report: expect.any(Object),
      max_rows: 7,
    }),
  )
})
```

---

### Finding 5 — preview route-policy messaging still drifts in `validationRequestBuilder.ts`

- **Severity:** high  
- **Type:** parity  
- **Location:** `validationRequestBuilder.ts` — `compositeProfileGpuRouteNote()` / `compositeProfileAutoCpuPreferenceNote()` / `buildValidationCapabilitySummary()`  
- **Timing boundary affected:** route-matrix / route-summary evidence  
- **Why it matters:** the preview path still uses older language for several composite profiles:
  - `VCC` still says Auto prefers CPU and that GPU falls back on fp32 stability (`validationRequestBuilder.ts:357-385`)
  - `CARM` / `CDMC` still say “GPU metric evaluation” rather than “exact CPU-series validation metrics” (`validationRequestBuilder.ts:371-374`)

  But the post-run path in `validationService.ts` already uses the newer wording (`validationService.ts:970-1017`), and the Rust export path partially reflects the newer wording as well (`commands_reports_validation_common.rs:1648-1739`).
- **Recommended change:** update the preview builder immediately, then optionally extract a shared TS route-policy helper to keep preview and post-run summaries in sync.

#### Minimum required patch

##### `validationRequestBuilder.ts` — update the route-note strings

```ts
function compositeProfileGpuRouteNote(profile: CompositeProfileKey): string {
  switch (profile) {
    case 'vcc':
      return 'VCC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. This removes the old fp32 momentum roundtrip fallback while keeping strict validation parity.'
    case 'carm':
      return 'CARM GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.'
    case 'cdmc':
      return 'CDMC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads.'
    // leave the other cases as-is
  }
}

function compositeProfileAutoCpuPreferenceNote(profile: CompositeProfileKey): string {
  switch (profile) {
    case 'vcc':
      return ''
    // leave the other cases as-is
  }
}
```

That single change will also cause `buildValidationCapabilitySummary()` to stop previewing VCC as “expected CPU-preferred composite route” when `backend=Auto`, which is the correct direction.

#### Durable patch (recommended)

Create a new shared TS helper:

##### New file: `web/src/services/validationRoutePolicy.ts`

```ts
export interface CompositeValidationRoutePolicy {
  gpuRouteDetail: string
  autoCpuPreferenceNote: string
  cpuReferenceHybrid: boolean
  vcetMixed: boolean
}

const DEFAULT_POLICY: CompositeValidationRoutePolicy = {
  gpuRouteDetail: '',
  autoCpuPreferenceNote: '',
  cpuReferenceHybrid: false,
  vcetMixed: false,
}

const COMPOSITE_POLICIES: Record<string, CompositeValidationRoutePolicy> = {
  Vcc: {
    gpuRouteDetail:
      'VCC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. This removes the old fp32 momentum roundtrip fallback while keeping strict validation parity.',
    autoCpuPreferenceNote: '',
    cpuReferenceHybrid: false,
    vcetMixed: false,
  },
  Carm: {
    gpuRouteDetail:
      'CARM GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.',
    autoCpuPreferenceNote:
      'Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.',
    cpuReferenceHybrid: true,
    vcetMixed: false,
  },
  Cdmc: {
    gpuRouteDetail:
      'CDMC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads.',
    autoCpuPreferenceNote: '',
    cpuReferenceHybrid: true,
    vcetMixed: false,
  },
  Vcet: {
    gpuRouteDetail:
      'VCET GPU validation now supports the fixed-candidate GPU route for the main profile. Walk-forward and reopt walk-forward currently use the generic validation path, and perturb/trade bootstrap remain auxiliary CPU work.',
    autoCpuPreferenceNote: '',
    cpuReferenceHybrid: false,
    vcetMixed: true,
  },
}

export function getCompositeValidationRoutePolicy(strategyKind: string): CompositeValidationRoutePolicy {
  return COMPOSITE_POLICIES[strategyKind] || DEFAULT_POLICY
}
```

Then both `validationRequestBuilder.ts` and `validationService.ts` can import this helper instead of duplicating string tables.

#### Test updates

##### `validationRequestBuilder.test.ts`

Update the VCC expectations:

```ts
const summary = buildValidationCapabilitySummary(req)
expect(summary.routeSummary).toContain('expected GPU composite route (VCC)')
expect(summary.warningSummary).toContain(
  'VCC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics',
)
```

Update the CDMC wording:

```ts
expect(summary.warningSummary).toContain(
  'CDMC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics',
)
```

Add a CARM expectation if you want explicit preview coverage for the newer wording.

---

### Finding 6 — export-side expected and actual route summaries still drift from the frontend

- **Severity:** high  
- **Type:** parity  
- **Location:** `commands_reports_validation_common.rs`  
- **Timing boundary affected:** route-matrix / route-summary evidence  
- **Why it matters:** the Rust export model currently diverges from the frontend in two important ways:

  1. **Expected execution category summary is too coarse.**  
     `build_expected_execution_category_summary()` treats composite GPU routes as fully GPU-native unless they are CPU-only / CPU-preferred / fallback (`commands_reports_validation_common.rs:1595-1645`). That misses the mixed `GPU main + CPU tail` behavior already encoded in the frontend for `CARM`, `CDMC`, and `VCET`.

  2. **Actual route-note summary is incomplete.**  
     `build_model_from_report()` currently derives `actual_route_note_summary` only from `backend_fallback_reason` via `split_actual_route_reason()` (`commands_reports_validation_common.rs:2295-2303`). That means it can miss:
     - the `CompositeMo` CPU-route note,
     - the `CARM` Auto CPU-preference note,
     - the GPU route detail note for `VCC` / `CARM` / `CDMC` / `VCET`.

- **Recommended change:** add a Rust-side route-policy helper and use it for both:
  - `expected_*` summaries
  - `actual_route_note_summary`

#### Required code edits

##### `commands_reports_validation_common.rs` — add a small Rust route-policy helper

```rust
#[derive(Clone, Copy, Debug, Default)]
struct CompositeRoutePolicy {
    gpu_route_detail: Option<&'static str>,
    auto_cpu_preference: Option<&'static str>,
    cpu_reference_hybrid: bool,
    vcet_mixed: bool,
    cpu_fallback: bool,
}

fn composite_route_policy(strategy_kind: &str) -> CompositeRoutePolicy {
    match strategy_kind {
        "Vcc" => CompositeRoutePolicy {
            gpu_route_detail: Some(
                "VCC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. This removes the old fp32 momentum roundtrip fallback while keeping strict validation parity.",
            ),
            auto_cpu_preference: None,
            cpu_reference_hybrid: false,
            vcet_mixed: false,
            cpu_fallback: false,
        },
        "Carm" => CompositeRoutePolicy {
            gpu_route_detail: Some(
                "CARM GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.",
            ),
            auto_cpu_preference: Some(
                "Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.",
            ),
            cpu_reference_hybrid: true,
            vcet_mixed: false,
            cpu_fallback: false,
        },
        "Cdmc" => CompositeRoutePolicy {
            gpu_route_detail: Some(
                "CDMC GPU validation currently uses CPU-reference signal prep with exact CPU-series validation metrics on the current mixed route. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads.",
            ),
            auto_cpu_preference: None,
            cpu_reference_hybrid: true,
            vcet_mixed: false,
            cpu_fallback: false,
        },
        "Vcet" => CompositeRoutePolicy {
            gpu_route_detail: Some(
                "VCET GPU validation now supports the fixed-candidate GPU route for the main profile. Walk-forward currently uses the generic validation path, and perturb/trade bootstrap remain auxiliary CPU work.",
            ),
            auto_cpu_preference: None,
            cpu_reference_hybrid: false,
            vcet_mixed: true,
            cpu_fallback: false,
        },
        "CompositeMo" => CompositeRoutePolicy {
            gpu_route_detail: Some(
                "CompositeMo GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only.",
            ),
            auto_cpu_preference: None,
            cpu_reference_hybrid: false,
            vcet_mixed: false,
            cpu_fallback: true,
        },
        _ => CompositeRoutePolicy::default(),
    }
}
```

##### `commands_reports_validation_common.rs` — update expected category classification

```rust
fn build_expected_execution_category_summary(
    strategy_kind: &str,
    validation_req: &Value,
    backend: &str,
) -> String {
    let modules = enabled_execution_category_modules(validation_req);

    if is_double_like_strategy(strategy_kind) {
        // keep the existing double-like logic
    }

    let policy = composite_route_policy(strategy_kind);
    if backend == "CpuOnly" || (backend == "Auto" && policy.auto_cpu_preference.is_some()) || policy.cpu_fallback {
        return format_execution_category_summary(vec![("CPU-only", modules)]);
    }

    let mut gpu_native = Vec::new();
    let mut gpu_tail = Vec::new();

    let base = if policy.cpu_reference_hybrid { &mut gpu_tail } else { &mut gpu_native };
    base.push("base reruns");
    base.push("hard gates");

    if validation_req.get("walk_forward").is_some_and(|v| !v.is_null()) {
        if policy.cpu_reference_hybrid || policy.vcet_mixed {
            gpu_tail.push("walk_forward");
        } else {
            gpu_native.push("walk_forward");
        }
    }

    if validation_req.get("reopt_walk_forward").is_some_and(|v| !v.is_null()) {
        if policy.cpu_reference_hybrid || policy.vcet_mixed {
            gpu_tail.push("reopt_walk_forward");
        } else {
            gpu_native.push("reopt_walk_forward");
        }
    }

    if validation_req.get("cscv_pbo").is_some_and(|v| !v.is_null()) {
        if policy.cpu_reference_hybrid {
            gpu_tail.push("cscv_pbo");
        } else {
            gpu_native.push("cscv_pbo");
        }
    }

    if validation_req.get("spa").is_some_and(|v| !v.is_null()) {
        if policy.cpu_reference_hybrid {
            gpu_tail.push("spa");
        } else {
            gpu_native.push("spa");
        }
    }

    if validation_req.get("perturb").is_some_and(|v| !v.is_null()) {
        gpu_tail.push("perturb");
    }
    if validation_req.get("trade_bootstrap").is_some_and(|v| !v.is_null()) {
        gpu_tail.push("trade_bootstrap");
    }

    format_execution_category_summary(vec![
        ("GPU-native", gpu_native),
        ("GPU main + CPU tail", gpu_tail),
    ])
}
```

##### `commands_reports_validation_common.rs` — compute actual route-note summary from policy, not just raw fallback text

```rust
fn build_actual_route_annotations(
    strategy_kind: &str,
    requested_backend: &str,
    backend_used: &str,
    backend_route: &str,
    backend_fallback_reason: &str,
) -> (String, String) {
    let policy = composite_route_policy(strategy_kind);
    let mut fallback_parts: Vec<String> = Vec::new();
    let mut route_note_parts: Vec<String> = Vec::new();

    if !backend_fallback_reason.trim().is_empty() {
        if backend_route.contains("->CpuOnly") {
            fallback_parts.push(backend_fallback_reason.trim().to_string());
        } else {
            route_note_parts.push(backend_fallback_reason.trim().to_string());
        }
    }

    if strategy_kind == "CompositeMo"
        && backend_used.eq_ignore_ascii_case("cpu")
        && backend_route.to_ascii_lowercase().contains("cpuonly")
    {
        route_note_parts.push(
            "CompositeMo uses the CPU validation route; GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only."
                .to_string(),
        );
    }

    if strategy_kind == "Carm"
        && requested_backend == "Auto"
        && backend_used.eq_ignore_ascii_case("cpu")
        && backend_route.to_ascii_lowercase().contains("cpuonly")
    {
        if let Some(note) = policy.auto_cpu_preference {
            route_note_parts.push(note.to_string());
        }
    }

    if backend_used.eq_ignore_ascii_case("gpu") {
        if let Some(note) = policy.gpu_route_detail {
            route_note_parts.push(note.to_string());
        }
    }

    (
        summary_or_dash(fallback_parts, " | "),
        summary_or_dash(route_note_parts, " | "),
    )
}
```

Then replace the current `split_actual_route_reason(...)` call in `build_model_from_report()`:

```rust
let requested_backend = validation_req
    .pointer("/backend/backend")
    .and_then(Value::as_str)
    .unwrap_or("CpuOnly")
    .trim();

let (actual_fallback_summary, actual_route_note_summary) = build_actual_route_annotations(
    &model.strategy_kind,
    requested_backend,
    &model.backend_used,
    &model.backend_route,
    &model.backend_fallback_reason,
);
```

#### Test updates

##### New tests inside `commands_reports_validation_common.rs`

Add a `#[cfg(test)] mod tests` block and cover at least:

1. **expected VCC preview/export no longer says CPU-preferred**
2. **expected CDMC category summary says `GPU main + CPU tail`**
3. **actual CompositeMo CPU route note appears in the export model**
4. **actual CARM Auto CPU-preference note appears in the export model**

Example starter:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn expected_vcc_summary_uses_gpu_route_and_exact_cpu_series_note() {
        let req = json!({
            "backend": { "backend": "Auto" },
            "validation_context": { "strategy_kind": "Vcc" },
            "candidates": [{ "vcc": {}, "fast_ma_type": "ema", "slow_ma_type": "ema", "fast_len": 10, "slow_len": 40 }]
        });

        let summary = build_expected_execution_summary("Vcc", Some(&req));
        assert!(summary.route_summary.contains("expected GPU composite route"));
        assert!(summary.route_note_summary.contains("exact CPU-series validation metrics"));
        assert_eq!(summary.fallback_summary, "-");
    }
}
```

---

### Finding 7 — shortlist logic is duplicated across frontend and Rust export paths

- **Severity:** medium  
- **Type:** parity / maintainability  
- **Location:**  
  - `validationService.ts` — `computeValidationShortlist()` (`1254-1382`)  
  - `commands_reports_validation_common.rs` — `legacy_shortlist_flags()` (`554-633`) and `shortlist_rows()` (`2249-2279`)  
- **Timing boundary affected:** app-path / export path  
- **Why it matters:** the shortlist contract exists in two languages and two shapes:
  - frontend shortlist drives on-screen rows,
  - Rust shortlist flags drive export compatibility.

  They are similar but not identical:
  - the frontend considers backend verdicts and probability-gate fallbacks,
  - the Rust `legacy_shortlist_flags()` is intentionally simpler and compatibility-oriented.

  That means the UI can show one shortlist while the workbook selects another.
- **Recommended change:** for the safe pass, keep the dual implementation but add parity tests using the same candidate fixtures. For the durable pass, serialize shortlist membership into the validation artifact and consume it in both places.

#### Safe-pass recommendation

Do **not** rewrite this immediately unless you are already changing workbook behavior. Instead:

1. keep `computeValidationShortlist()` as the live UI authority,
2. keep Rust legacy shortlist logic for compatibility,
3. add tests that compare both against a small agreed fixture matrix,
4. document any intentional differences.

#### Higher-confidence durable option

After the timing + dataset + route fixes land, add an explicit shortlist contract to the persisted validation artifact or report model:

```ts
interface PersistedValidationShortlist {
  shortlistRanks: number[]
  hasCross: boolean
  usedFallback: boolean
  usedCscvFallback: boolean
}
```

Then let:
- the UI use that when present,
- the workbook export use that when present,
- and keep the old logic only as a fallback for older artifacts.

Because this changes an artifact contract, I recommend treating it as **phase 2**, not phase 1.

#### Test recommendations

- TS test fixture for `computeValidationShortlist()`
- Rust test fixture for `legacy_shortlist_flags()`
- if you later persist shortlist membership: add backward-compat tests for “old artifact without shortlist payload”

---

### Finding 8 — `topRowsFiltered` is still a dead branch on the hot validation path

- **Severity:** low  
- **Type:** maintainability / open question  
- **Location:**  
  - `validationService.ts` — `resolveValidationBuildContextFromWorkflowArtifacts()` (`1595-1607`)  
  - `validationRequestBuilder.ts` — `buildValidationRequest()` (`641-645`)  
- **Timing boundary affected:** unknown  
- **Why it matters:** `buildValidationRequest()` supports `topRowsFiltered` and `useFiltered`, but `resolveValidationBuildContextFromWorkflowArtifacts()` hardcodes `topRowsFiltered: []`. That means:
  - either filtered finalists are no longer a real product path,
  - or the validation flow is quietly ignoring a work-reduction mechanism.
- **Recommended change:** choose one:
  - wire filtered finalists through the workflow artifact context, or
  - remove the dormant branch and simplify the request builder.

This is not a blocker for bundle-9 fixes, but it is worth resolving so the code matches real behavior.

---

## 3. Safe optimization opportunities

These are the low-risk changes I would make first.

### A. Fix timing labels without changing the backend contract
This is purely a frontend orchestration fix and immediately improves perf-fidelity.

### B. Propagate `dataset_meta` everywhere the run artifact is re-applied
This removes real replay/export drift and uses an artifact field the repo already saves correctly.

### C. Eliminate the duplicate run-artifact write from the auto-export success path
This directly reduces validation critical-path work.

### D. Pass `max_rows` through the fallback XLSX path
This removes a correctness + work-shape difference between primary and fallback export flows.

### E. Align preview and export route text with the current post-run wording
This is a parity-safe update that mostly changes strings and category classification, not execution.

### F. Add parity tests before doing any larger route-policy extraction
These tests will keep you from reintroducing drift later.

---

## 4. Higher-risk optimization opportunities

These are valuable, but I would land them **after** the safe fixes above.

### A. Make the backend own route-note/category summary strings
Instead of duplicating route-policy tables in:
- `validationService.ts`
- `validationRequestBuilder.ts`
- `commands_reports_validation_common.rs`

you could have the validation backend return:
- `route_note_summary`
- `category_summary`
- `aux_cpu_summary`

That would be the cleanest parity solution, but it requires changing the backend result contract and updating all consumers.

### B. Add `actual_category_summary` to the export model
Right now `ValidationExportModel` contains `expected_category_summary` but not `actual_category_summary`. If you want workbook parity with the post-run panel, you will need:
- a new model field in `commands_reports_validation_common.rs`
- and corresponding workbook rendering changes in the adjacent XLSX builder.

That is useful, but it reaches beyond the current bundle files.

### C. Persist shortlist membership in the artifact/report
This is the durable fix for shortlist drift, but it changes an artifact contract and should be versioned carefully.

### D. Remove direct DOM writes once the legacy shell no longer depends on them
`validationService.ts` still calls both:
- `patchValidationStoreState(...)`
- `writeDomTextContent(...)`

around the same status/meta fields.

That may still be needed for the legacy shell. If the legacy dependency is gone, delete the direct DOM writes and rely on Vue state only. Do **not** do this blindly without confirming the shell/runtime contract.

---

## 5. Benchmark and perf-gate review

I cannot quantify gains from the uploaded files alone, but the correct evidence path is clear.

### Timing boundaries to measure

Measure these separately:

1. **validation command roundtrip**  
   start immediately before `invokeCommand(run_*_validation)`  
   stop immediately after it resolves

2. **validation end-to-end app-path**  
   start before command invoke  
   stop after shortlist, table shaping, store apply, and status updates complete

3. **validation + XLSX follow-up**  
   start before export  
   stop after save/download + artifact persistence + open/reveal follow-up

### Recommended harness use

Use the existing repo perf evidence path before inventing new instrumentation:

- validation-cost harness:
  - `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- workflow benches:
  - `crates/app_core/benches/double_ma_validation_workflow.rs`
  - `crates/app_core/benches/composite_validation_workflow.rs`
- app-path / workflow regression:
  - `src-tauri/src/bin/ta_app_path_faceoff.rs`
  - `src-tauri/src/bin/ta_workflow_regression.rs`

### Suggested measurement matrix

Record before/after for these scenarios:

1. validation only, no export  
2. validation with auto-export success to saved path  
3. validation with auto-export download fallback  
4. validation with auto-export full failure  
5. saved-run load → validation preview → export report XLSX  

### What success should look like

- “Validation done in …” now reports **true end-to-end** validation time.
- Auto-export success path issues **one** `save_run_artifact` call, not two.
- Fallback export honors the same `max_rows` cap as the primary save path.
- Saved-run reload keeps dataset context stable enough that rerun/report behavior matches the originating artifact.

---

## 6. Test review

## Existing strengths

The current uploaded tests already cover a lot of the right semantics:

- dataset mismatch rejection in `validationService.test.ts`
- route-detail text for CARM/CDMC/CompositeMo post-run summaries
- auto-export happy-path and failure-path behavior
- preview capability summaries in `validationRequestBuilder.test.ts`

## Gaps that should be closed now

### Must-add tests

1. **end-to-end vs command timing label**
2. **single artifact save on auto-export success**
3. **artifact still persisted when auto-export fails or downloads instead of saving**
4. **fallback `build_report_xlsx` receives `max_rows`**
5. **`datasetMeta` survives:**
   - validation run apply
   - saved-run load
   - report payload build
6. **preview VCC/CARM/CDMC wording matches current post-run wording**
7. **export model expected/actual route summaries match the updated policy**

### Nice-to-have tests

1. TS/Rust shortlist parity fixture
2. dead `topRowsFiltered` branch decision test (either wired or removed)
3. workbook-model test for actual CompositeMo/CARM route-note summary

---

## 7. Suggested improvement plan

## Phase 1 — correctness and timing (land first)

1. Fix timing labels in `runValidation()`.
2. Add `datasetMeta` to persistence return values.
3. Pass `dataset_meta` back into the workflow store in `applyVueValidationArtifacts()`.
4. Preserve `dataset_meta` in `historyService.loadSavedRun()`.
5. Make report payload building prefer artifact dataset metadata over runtime dataset state.
6. Add tests for timing and dataset-meta retention.

## Phase 2 — app-path overhead (land second)

1. Refactor auto-export so validation success path persists the artifact only once.
2. Make `exportReportXlsx()` return a structured result.
3. Add `max_rows` to the fallback `build_report_xlsx` path.
4. Add tests for single-save auto-export and capped fallback export.

## Phase 3 — route-policy parity (land third)

1. Update `validationRequestBuilder.ts` preview wording for VCC/CARM/CDMC.
2. Update `validationRequestBuilder.test.ts`.
3. Add Rust-side route-policy helper in `commands_reports_validation_common.rs`.
4. Update export expected/actual route summaries and mixed category classification.
5. Add Rust tests for export parity.

## Phase 4 — cleanup and durability (optional)

1. Decide whether to keep or remove `topRowsFiltered`.
2. Decide whether to extract a shared TS `validationRoutePolicy.ts`.
3. Decide whether to persist shortlist membership in the artifact.
4. Decide whether to move actual route-note/category ownership into the backend response contract.

---

# File-by-file implementation plan

## `ValidationWorkspacePanel.vue`

### Recommendation
**No required production logic change** for phase 1–3.

### Why
The panel already consumes:
- preview summary via `expectedExecutionSummary`
- live status via the validation store

The important fixes belong in the service layer. The panel will automatically display the improved summaries once the service/store data is corrected.

### Optional cosmetic follow-up
If you later want to expose both timings more explicitly without adding a new panel, reuse an existing `metaLine` or status text rather than introducing a new surface.

### Action
- keep as-is for now
- verify after service-layer changes that the “Expected …” block shows the corrected VCC/CARM/CDMC/VCET behavior

---

## `validationService.ts`

### Required changes
- fix timing boundary labeling
- propagate `datasetMeta`
- prefer artifact dataset metadata for report payloads
- refactor auto-export to avoid duplicate artifact writes
- pass `max_rows` through fallback export
- keep artifact persistence non-fatal, but visible

### Key implementation notes
- do **not** remove dataset-match enforcement in `assertValidationRunMatchesCurrentDataset()`
- do **not** let auto-export failure prevent validation completion
- do **not** let download-fallback success skip artifact persistence altogether

### Core patch checklist
- [ ] add `nowMs()`
- [ ] add status-format helpers
- [ ] expand persistence result type
- [ ] add `saveArtifact?: boolean`
- [ ] carry `dataset_meta` into `applyVueValidationArtifacts()`
- [ ] add `buildReportDataFromRunArtifact()`
- [ ] refactor `runValidation()` auto-export path
- [ ] change `exportReportXlsx()` return type and fallback `max_rows` pass-through

---

## `validationService.test.ts`

### Required changes
- update status expectations from “frontend” to “end-to-end / command”
- assert single `save_run_artifact` call on auto-export success
- add fallback `max_rows` test
- add datasetMeta retention test
- keep the existing non-fatal auto-export failure assertions

### Core patch checklist
- [ ] update old timing text assertions
- [ ] add single-save auto-export assertion
- [ ] add fallback builder cap assertion
- [ ] add datasetMeta retention assertion
- [ ] keep Stage 3 fallback behavior coverage unchanged

---

## `validationRequestBuilder.ts`

### Required changes
- update route-detail strings for VCC/CARM/CDMC
- stop previewing VCC as Auto CPU-preferred
- keep current mixed category classification logic for CARM/CDMC/VCET

### Recommended durable change
Extract route-note data into a tiny shared helper so `validationService.ts` and `validationRequestBuilder.ts` stop hand-maintaining separate string tables.

### Core patch checklist
- [ ] change VCC GPU route note
- [ ] change CARM GPU route note
- [ ] change CDMC GPU route note
- [ ] change VCC Auto CPU preference to empty
- [ ] optionally extract shared TS route-policy helper

---

## `validationRequestBuilder.test.ts`

### Required changes
- update VCC expectations from CPU-preferred to GPU-route preview
- update CDMC wording from “GPU metric evaluation” to “exact CPU-series validation metrics”
- add or tighten CARM/VCC route-note assertions if needed

### Core patch checklist
- [ ] update VCC routeSummary expectation
- [ ] update VCC warningSummary expectation
- [ ] update CDMC warningSummary expectation
- [ ] optionally add CARM wording test

---

## `historyService.ts`

### Required changes
- preserve `dataset_meta` in `loadSavedRun()`

### Optional adjacent change
Create `historyService.test.ts` so this cannot regress silently.

### Core patch checklist
- [ ] extract `datasetMeta` from `record.dataset_meta`
- [ ] pass `dataset_meta` into `currentRun` payload
- [ ] add/load adjacent test if you create one

---

## `workflowArtifactsStore.ts`

### Recommendation
**No required production change.**

### Why
This file already does the right normalization work:
- `dataset_meta` / `datasetMeta`
- `report_xlsx_path` / `reportXlsxPath`
- `ml_signal` / `mlSignal`

It is the contract target the other files should conform to.

### Optional follow-up
If you maintain store tests elsewhere, add one that proves `dataset_meta` survives `applyWorkflowArtifactsPayload()`.

### Core patch checklist
- [ ] no production edit required
- [ ] optional store-contract test if you have a store test suite

---

## `commands_validation.rs`

### Recommendation
**No required production change for the bundle-9 safe pass.**

### Why
The backend validation context guard is already correct and strict:
- strategy-kind compatibility
- csv-path match
- timeframe match

This file is useful context, but not the source of the current bundle-9 regressions.

### Optional future change
If you later decide to make the backend the single owner of route-note strings, this file (or the underlying app-core validation result model) is where you would add backend-generated route metadata fields.

### Core patch checklist
- [ ] keep `validate_validation_context()` as-is
- [ ] no phase-1 production edit required
- [ ] optional future backend-owned route-summary work only

---

## `commands_reports.rs`

### Required changes
- accept `max_rows` in `build_report_xlsx_blocking()`

### Core patch checklist
- [ ] add `max_rows: Option<u32>` parameter
- [ ] convert to `Option<usize>`
- [ ] pass it into `build_validation_report_xlsx(...)`

---

## `commands_reports_validation_common.rs`

### Required changes
- update route-note strings for VCC/CARM/CDMC
- stop treating VCC as Auto CPU-preferred
- classify composite mixed GPU/CPU categories like the frontend does
- compute `actual_route_note_summary` from route policy, not only from raw fallback text
- add Rust tests

### Optional but valuable follow-up
If you later want full workbook parity with the post-run panel, add `actual_category_summary` and render it in the adjacent XLSX builder.

### Core patch checklist
- [ ] add small Rust route-policy helper
- [ ] update expected category summary builder
- [ ] update actual route-note annotation builder
- [ ] update VCC/CARM/CDMC strings
- [ ] add `#[cfg(test)]` unit tests

---

# Recommended implementation order with commit boundaries

## Commit 1 — timing + dataset-meta propagation
Files:
- `validationService.ts`
- `historyService.ts`
- `validationService.test.ts`
- `historyService.test.ts` *(new, optional but strongly recommended)*

Goal:
- correct timing labels
- stable artifact dataset context

## Commit 2 — auto-export single-save refactor + fallback cap fix
Files:
- `validationService.ts`
- `validationService.test.ts`
- `commands_reports.rs`
- adjacent Tauri wrapper for `build_report_xlsx` *(not in bundle)*

Goal:
- remove duplicate artifact persistence on auto-export success
- keep failure/download fallback non-fatal
- honor `max_rows` everywhere

## Commit 3 — preview/export route-policy parity
Files:
- `validationRequestBuilder.ts`
- `validationRequestBuilder.test.ts`
- `commands_reports_validation_common.rs`

Goal:
- VCC/CARM/CDMC/VCET/CompositeMo parity across preview/export
- Rust tests to prevent regression

## Commit 4 — optional durability cleanup
Files:
- optional `validationRoutePolicy.ts`
- optional shortlist contract changes
- optional backend-owned route metadata

Goal:
- reduce future drift, not required for immediate bundle-9 correctness

---

# Final rollout checklist

## Must do

- [ ] Fix `runValidation()` timing so the UI stops calling command-only time “frontend”.
- [ ] Preserve `dataset_meta` in `applyVueValidationArtifacts()`.
- [ ] Preserve `dataset_meta` in `historyService.loadSavedRun()`.
- [ ] Make `buildCurrentReportPayloadFromStore()` prefer artifact dataset context.
- [ ] Refactor auto-export so the success path persists the run artifact only once.
- [ ] Ensure export failure/download fallback still leaves a saved validation artifact behind.
- [ ] Pass `max_rows` to `build_report_xlsx` fallback.
- [ ] Update preview VCC/CARM/CDMC wording in `validationRequestBuilder.ts`.
- [ ] Update export VCC/CARM/CDMC wording in `commands_reports_validation_common.rs`.
- [ ] Update export expected category classification for CARM/CDMC/VCET mixed routes.
- [ ] Add/adjust tests in `validationService.test.ts`.
- [ ] Add Rust tests in `commands_reports_validation_common.rs`.
- [ ] Verify `ValidationWorkspacePanel.vue` preview text now reflects the fixed service-layer output.

## Strongly recommended

- [ ] Add `historyService.test.ts` for `dataset_meta` replay coverage.
- [ ] Extract a shared TS route-policy helper used by both preview and post-run code.
- [ ] Add fixture-based shortlist parity tests across TS and Rust.
- [ ] Decide whether `topRowsFiltered` is real and wire it, or delete it.

## Optional / phase 2+

- [ ] Add backend-owned route-summary fields to validation results.
- [ ] Add `actual_category_summary` to the export model and workbook.
- [ ] Persist shortlist membership into the artifact/report contract.
- [ ] Remove direct DOM writes once you confirm the legacy shell no longer depends on them.

## Verification pass before merge

- [ ] Run frontend unit tests.
- [ ] Run Rust unit tests.
- [ ] Manually verify:
  - [ ] validation with no export
  - [ ] validation + auto-export save path
  - [ ] validation + auto-export download fallback
  - [ ] validation + auto-export full failure
  - [ ] load saved run → preview validation → export report
- [ ] Record before/after timings for:
  - [ ] command roundtrip
  - [ ] validation end-to-end
  - [ ] XLSX follow-up
- [ ] Confirm no user-visible drift in:
  - [ ] VCC
  - [ ] CARM
  - [ ] CDMC
  - [ ] VCET
  - [ ] CompositeMo

---

## Bottom line

If you implement only the must-do list, you will remove the main bundle-9 problems without redesigning the validation surface:

- timing becomes truthful,
- replay/export dataset context becomes stable,
- auto-export stops doing unnecessary artifact I/O on the hot success path,
- fallback export behavior becomes shape-consistent,
- preview/export route messaging stops disagreeing with post-run behavior.

That is the right bundle-9 outcome.
