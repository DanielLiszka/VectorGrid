import { invokeCommand } from '../tauri/invoke'
import { getActivePinia } from 'pinia'
import { buildValidationRequest } from './validationRequestBuilder'
import { applyWorkflowArtifactsPayload, readLatestWorkflowRunSnapshot } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { computeConfigFingerprint } from '../utils/core/artifactIdentity'
import { buildDatasetMetaForArtifact } from '../utils/core/datasetMeta'

interface PolygonCrossDatasetResponse {
  csv_path?: unknown
  display_symbol?: unknown
  polygon_ticker?: unknown
  market?: unknown
  timeframe?: unknown
  min_ts?: unknown
  max_ts?: unknown
  last_updated_utc_ms?: unknown
  row_count?: unknown
  adjusted?: unknown
}

interface PolygonCacheInfoResponse {
  datasets?: unknown
}

export interface ValidationCrossDatasetSummary {
  path: string
  label: string
  meta: string
  market: string
  timeframe: string
  adjusted: boolean | null
}

export interface ValidationCrossRefreshSummary {
  datasets: ValidationCrossDatasetSummary[]
}

function normalizePath(value: string): string {
  return String(value || '').trim()
}

function normalizeText(value: unknown): string {
  return String(value || '').trim()
}

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  const kind = normalizeText(kindRaw || 'DoubleMa') || 'DoubleMa'
  if (kind === 'Pit') return 'Pit'
  if (kind === 'Set') return 'Set'
  if (kind === 'Mts') return 'Mts'
  if (kind === 'Orc') return 'Orc'
  if (kind === 'Marrt') return 'Marrt'
  if (kind === 'Vfct') return 'Vfct'
  if (kind === 'Rocb') return 'Rocb'
  if (kind === 'Asr') return 'Asr'
  if (kind === 'Vcc') return 'Vcc'
  if (kind === 'Vemr') return 'Vemr'
  if (kind === 'Mkcs') return 'Mkcs'
  if (kind === 'CompositeCpu') return 'CompositeCpu'
  if (kind === 'CompositeMo') return 'CompositeMo'
  if (kind === 'CompositeTrend') return 'CompositeTrend'
  if (kind === 'RegimeHybrid') return 'RegimeHybrid'
  if (kind === 'BreakoutMomentum') return 'BreakoutMomentum'
  return 'DoubleMa'
}

function isDoubleLikeKind(kindRaw: unknown): boolean {
  const kind = normalizeStrategyKindRaw(kindRaw)
  return kind === 'DoubleMa' || kind === 'Pit' || kind === 'Set' || kind === 'Mts' || kind === 'Orc' || kind === 'Marrt' || kind === 'Vfct' || kind === 'CompositeTrend' || kind === 'RegimeHybrid' || kind === 'BreakoutMomentum'
}

function resolveValidationCommand(runReq: unknown): 'run_double_ma_validation' | 'run_composite_validation' {
  const req = runReq && typeof runReq === 'object' ? (runReq as Record<string, unknown>) : null
  const kind = normalizeStrategyKindRaw(req?.strategy_kind ?? req?.strategyKind)
  return isDoubleLikeKind(kind) ? 'run_double_ma_validation' : 'run_composite_validation'
}

interface CrossSelectionEntry {
  csv_path?: unknown
  path?: unknown
  label?: unknown
  checked?: unknown
}

interface LegacyValidationBuildContext {
  currentRunReq: unknown
  currentRunRes: unknown
  topRows: Array<{ params?: unknown }>
  topRowsFiltered: Array<{ params?: unknown }>
}

interface RunSnapshot {
  req: unknown
  res: unknown
}

export interface ValidationRunOptions {
  useFiltered?: boolean
  switchTab?: boolean
}

interface SaveRunArtifactResponse {
  path?: unknown
}

interface SaveReportXlsxResponse {
  path?: unknown
}

interface WorkflowRunArtifactSnapshot {
  ts: string
  artifactPath: string
  reportXlsxPath: string
  csvPath: string
  datasetMeta: Record<string, unknown> | null
  tags: string[]
  req: unknown
  res: unknown
}

interface WorkflowValidationArtifactSnapshot {
  ts: string
  req: unknown
  res: unknown
}

interface ValidationTableColumn {
  label: string
  alignLeft: boolean
}

interface ValidationTableCell {
  text: string
  className: string
  title: string
}

interface ValidationTableRow {
  cells: ValidationTableCell[]
}

interface VueStoreFieldBinding {
  key: string
}

const VUE_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valCandidates: { key: 'candidates' },
  valSeed: { key: 'seed' },
  valMinPsr: { key: 'minPsr' },
  valMinDsr: { key: 'minDsr' },
  valMinCscvRank: { key: 'minCscvRank' },
  valMinCscvMetric: { key: 'minCscvMetric' },
  valMaxPbo: { key: 'maxPbo' },
  valSpaSlices: { key: 'spaSlices' },
  valSpaBootstrapSamples: { key: 'spaBootstrapSamples' },
  valSpaBlockLength: { key: 'spaBlockLength' },
  valReoptWfWindow: { key: 'reoptWfWindow' },
  valReoptWfStep: { key: 'reoptWfStep' },
  valReoptWfTopN: { key: 'reoptWfTopN' },
  valWfWindow: { key: 'wfWindow' },
  valWfStep: { key: 'wfStep' },
  valPerturbIters: { key: 'perturbIters' },
  valPerturbMin: { key: 'perturbMin' },
  valPerturbMax: { key: 'perturbMax' },
  valMcSims: { key: 'mcSims' },
  valCscvSlices: { key: 'cscvSlices' },
  valCrossMax: { key: 'crossMax' }
}

const VUE_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valWfEnable: { key: 'wfEnable' },
  valPerturbEnable: { key: 'perturbEnable' },
  valMcEnable: { key: 'mcEnable' },
  valCscvEnable: { key: 'cscvEnable' },
  valSpaEnable: { key: 'spaEnable' },
  valReoptWfEnable: { key: 'reoptWfEnable' },
  valCrossEnable: { key: 'crossEnable' }
}

function readPiniaStoreState(storeId: string): Record<string, unknown> | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  const root = pinia.state.value as Record<string, unknown>
  const store = root[storeId]
  if (!store || typeof store !== 'object') return null
  return store as Record<string, unknown>
}

function readValidationStoreState(): Record<string, unknown> | null {
  return readPiniaStoreState('validation')
}

function readVueTextFieldValue(fieldId: string): string | null {
  const binding = VUE_TEXT_FIELD_BINDINGS[fieldId]
  if (!binding) return null
  const store = readValidationStoreState()
  if (!store) return null
  if (!Object.prototype.hasOwnProperty.call(store, binding.key)) return null
  return normalizeText(store[binding.key])
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function readVueCheckboxFieldValue(fieldId: string): boolean | null {
  const binding = VUE_CHECKBOX_FIELD_BINDINGS[fieldId]
  if (!binding) return null
  const store = readValidationStoreState()
  if (!store) return null
  if (!Object.prototype.hasOwnProperty.call(store, binding.key)) return null
  return normalizeBoolean(store[binding.key])
}

function readVueCrossSelectionFromStore(): CrossSelectionEntry[] | null {
  const store = readValidationStoreState()
  if (!store) return null
  const rows = store.crossDatasets
  if (!Array.isArray(rows)) return null

  const selected: CrossSelectionEntry[] = []
  for (const entry of rows) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as CrossSelectionEntry
    if (!normalizeBoolean(row.checked)) continue
    const csv_path = normalizeText(row.csv_path ?? row.path)
    if (!csv_path) continue
    selected.push({
      csv_path,
      label: normalizeText(row.label)
    })
  }
  return selected
}

function readVueCrossSelection(): CrossSelectionEntry[] {
  const selectedFromStore = readVueCrossSelectionFromStore()
  return selectedFromStore || []
}

function normalizeCrossSelectionEntries(entries: CrossSelectionEntry[]): Array<{ csv_path: string; label: string }> {
  const selected: Array<{ csv_path: string; label: string }> = []
  const seen = new Set<string>()
  const rows = Array.isArray(entries) ? entries : []
  for (const entry of rows) {
    const csv_path = normalizePath(normalizeText(entry?.csv_path ?? entry?.path))
    if (!csv_path || seen.has(csv_path)) continue
    seen.add(csv_path)
    selected.push({
      csv_path,
      label: normalizeText(entry?.label)
    })
  }
  return selected
}

function writeVueCrossSelection(entries: CrossSelectionEntry[]): void {
  const store = readValidationStoreState()
  if (!store) return
  const rows = Array.isArray(store.crossDatasets) ? store.crossDatasets : []
  if (!rows.length) return

  const selected = normalizeCrossSelectionEntries(entries)
  const selectedPaths = new Set(selected.map((entry) => entry.csv_path))
  let changed = false

  const nextRows = rows.map((entry) => {
    if (!entry || typeof entry !== 'object') return entry
    const row = entry as Record<string, unknown>
    const path = normalizePath(normalizeText(row.path))
    if (!path) return entry
    const nextChecked = selectedPaths.has(path)
    const prevChecked = normalizeBoolean(row.checked)
    if (prevChecked === nextChecked) return entry
    changed = true
    return {
      ...row,
      checked: nextChecked
    }
  })

  if (!changed) return
  store.crossDatasets = nextRows
  store.crossTotalCount = nextRows.length
  store.crossSelectedCount = nextRows.filter((entry) => !!entry && typeof entry === 'object' && normalizeBoolean((entry as Record<string, unknown>).checked)).length
  store.source = 'vue'
  store.updatedAtUnixMs = Date.now()
}

function normalizeMarketLabel(market: string, adjusted: boolean): string {
  if (!market) return 'market'
  if (market !== 'stocks') return market
  return adjusted ? 'stocks (adj)' : 'stocks (raw)'
}

function normalizeRowsHint(value: unknown): string {
  const rows = Number(value || 0)
  if (!Number.isFinite(rows) || rows <= 0) return '—'
  return rows.toLocaleString()
}

function formatYmdUtc(value: unknown): string {
  const ms = Number(value)
  if (!Number.isFinite(ms)) return '—'
  const date = new Date(ms)
  if (!Number.isFinite(date.getTime())) return '—'
  return date.toISOString().slice(0, 10)
}

function readStoreTextInputValue(id: string): string {
  const key = String(id || '').trim()
  if (!key) return ''
  const vueValue = readVueTextFieldValue(key)
  return vueValue != null ? vueValue : ''
}

function readStoreCheckboxValue(id: string): boolean {
  const key = String(id || '').trim()
  if (!key) return false
  const vueValue = readVueCheckboxFieldValue(key)
  return vueValue != null ? vueValue : false
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') return null
  return value as Record<string, unknown>
}

function extractTopRowsFromRunResult(runRes: unknown): Array<{ params?: unknown }> {
  const row = asRecord(runRes)
  if (!row) return []
  const topRowsRaw = Array.isArray(row.top) ? row.top : []
  const rows: Array<{ params?: unknown }> = []
  for (const entry of topRowsRaw) {
    const item = asRecord(entry)
    if (!item) continue
    if (!item.params) continue
    rows.push({ params: item.params })
  }

  if (rows.length > 0) return rows
  const bestParams = row.best_params
  if (!bestParams || typeof bestParams !== 'object') return []
  return [{ params: bestParams }]
}

function readRunSnapshotFromStore(): RunSnapshot | null {
  const snapshot = readLatestWorkflowRunSnapshot()
  if (!snapshot) return null
  return {
    req: snapshot.req ?? null,
    res: snapshot.res ?? null
  }
}

function normalizeStringList(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  const out: string[] = []
  for (const item of value) {
    const next = normalizeText(item)
    if (!next) continue
    out.push(next)
  }
  return out
}

function readWorkflowRunArtifactFromStore(): WorkflowRunArtifactSnapshot | null {
  const workflowArtifactsStore = readPiniaStoreState('workflowArtifacts')
  if (!workflowArtifactsStore) return null
  const run = workflowArtifactsStore.currentRun
  if (!run || typeof run !== 'object') return null
  const row = run as Record<string, unknown>
  return {
    ts: normalizeText(row.ts) || new Date().toISOString(),
    artifactPath: normalizeText(row.artifactPath ?? row.artifact_path),
    reportXlsxPath: normalizeText(row.reportXlsxPath ?? row.report_xlsx_path),
    csvPath: normalizeText(row.csvPath ?? row.csv_path),
    datasetMeta: row.datasetMeta && typeof row.datasetMeta === 'object'
      ? row.datasetMeta as Record<string, unknown>
      : (row.dataset_meta && typeof row.dataset_meta === 'object'
        ? row.dataset_meta as Record<string, unknown>
        : null),
    tags: normalizeStringList(row.tags),
    req: row.req ?? null,
    res: row.res ?? null
  }
}

function readWorkflowValidationArtifactFromStore(): WorkflowValidationArtifactSnapshot | null {
  const workflowArtifactsStore = readPiniaStoreState('workflowArtifacts')
  if (!workflowArtifactsStore) return null
  const validation = workflowArtifactsStore.currentValidation
  if (!validation || typeof validation !== 'object') return null
  const row = validation as Record<string, unknown>
  return {
    ts: normalizeText(row.ts) || new Date().toISOString(),
    req: row.req ?? null,
    res: row.res ?? null
  }
}

function readLatestMetaReportPath(): string {
  const commandActionsStore = readPiniaStoreState('commandActions')
  const fromStore = normalizeText(commandActionsStore?.latestMetaReportPath)
  if (fromStore) return fromStore
  if (typeof document === 'undefined') return ''
  return normalizeText(document.documentElement?.dataset?.latestMetaReportPath)
}

function readValidationExportPrefs(): {
  autoReveal: boolean
  maxRows: number | null
} {
  const store = readValidationStoreState()
  const autoReveal = normalizeBoolean(store?.autoRevealReport)
  const rawTopN = Math.floor(Number(store?.autoExportTopN))
  const maxRows = Number.isFinite(rawTopN) && rawTopN > 0 ? rawTopN : null
  return { autoReveal, maxRows }
}

function buildCurrentReportPayloadFromStore(): {
  version: number
  kind: string
  ts: string
  data: {
    meta: Record<string, unknown> | null
    ctx: Record<string, unknown>
  }
  run: { req: unknown; res: unknown }
  validation: { req: unknown; res: unknown } | null
} {
  const run = readWorkflowRunArtifactFromStore()
  const validation = readWorkflowValidationArtifactFromStore()
  const ts = normalizeText(run?.ts || validation?.ts) || new Date().toISOString()
  const datasetCtx = buildRunDatasetContextState()
  return {
    version: 1,
    kind: 'vectorbt_report',
    ts,
    data: {
      meta: datasetCtx.currentDataMeta,
      ctx: datasetCtx.currentDatasetCtx
    },
    run: {
      req: run?.req ?? null,
      res: run?.res ?? null
    },
    validation: validation ? { req: validation.req ?? null, res: validation.res ?? null } : null
  }
}

function downloadBlob(filename: string, blob: Blob): boolean {
  if (typeof document === 'undefined') return false
  if (typeof URL === 'undefined' || typeof URL.createObjectURL !== 'function') return false
  const href = URL.createObjectURL(blob)
  try {
    const link = document.createElement('a')
    link.href = href
    link.download = filename
    link.style.display = 'none'
    document.body?.appendChild(link)
    link.click()
    link.remove()
    return true
  } finally {
    URL.revokeObjectURL(href)
  }
}

function downloadText(filename: string, text: string, mime: string): boolean {
  try {
    return downloadBlob(filename, new Blob([text], { type: mime }))
  } catch {
    return false
  }
}

function decodeBase64(value: string): Uint8Array | null {
  const src = String(value || '').trim()
  if (!src) return null
  if (typeof atob === 'function') {
    const bin = atob(src)
    const out = new Uint8Array(bin.length)
    for (let i = 0; i < bin.length; i += 1) {
      out[i] = bin.charCodeAt(i) & 0xff
    }
    return out
  }
  return null
}

function downloadBase64(filename: string, base64: string, mime: string): boolean {
  const bytes = decodeBase64(base64)
  if (!bytes) return false
  const copy = new Uint8Array(bytes.length)
  copy.set(bytes)
  return downloadBlob(filename, new Blob([copy.buffer], { type: mime }))
}

async function openPathWithRevealFallback(path: string): Promise<void> {
  const nextPath = normalizeText(path)
  if (!nextPath) return
  try {
    await invokeCommand('open_path', { path: nextPath })
  } catch {
    await invokeCommand('reveal_path', { path: nextPath })
  }
}

function buildRunDatasetContextState(): {
  currentDataMeta: Record<string, unknown> | null
  currentBaseDataMeta: Record<string, unknown> | null
  currentBaseDatasetCtx: Record<string, unknown>
  currentDatasetCtx: Record<string, unknown>
} {
  const dataStore = readPiniaStoreState('data')
  const dataFields = Array.isArray(dataStore?.dataFields) ? dataStore.dataFields : []
  const fields = new Set<string>()
  for (const entry of dataFields) {
    const key = normalizeText(entry).toLowerCase()
    if (!key) continue
    fields.add(key)
  }
  const len = Number(dataStore?.dataLen)
  const baseMeta = asRecord(dataStore?.baseDataMeta)
  const timeframe = normalizeText(dataStore?.dataTimeframe || baseMeta?.timeframe)
  const dataMeta: Record<string, unknown> = {
    has_open: fields.has('open'),
    has_high: fields.has('high'),
    has_low: fields.has('low'),
    has_close: fields.has('close'),
    has_volume: fields.has('volume')
  }
  if (Number.isFinite(len) && len >= 0) dataMeta.len = Math.floor(len)
  if (timeframe) dataMeta.timeframe = timeframe

  const datasetCtx: Record<string, unknown> = {}
  const label = normalizeText(dataStore?.datasetLabel)
  const source = normalizeText(dataStore?.datasetContext)
  const symbol = normalizeText(dataStore?.polygonSymbol || label)
  if (label && label !== '—') datasetCtx.label = label
  if (symbol && symbol !== '—') datasetCtx.symbol = symbol
  if (source && source !== '—') datasetCtx.source = source
  if (timeframe) {
    datasetCtx.timeframe = timeframe
    datasetCtx.base_timeframe = timeframe
    datasetCtx.derived_timeframe = timeframe
    datasetCtx.resample_applied = false
  }

  return {
    currentDataMeta: dataMeta,
    currentBaseDataMeta: dataMeta,
    currentBaseDatasetCtx: datasetCtx,
    currentDatasetCtx: datasetCtx
  }
}

function canonicalLower(value: unknown): string {
  return normalizeText(value).toLowerCase()
}

function firstNonEmptyText(...values: unknown[]): string {
  for (const value of values) {
    const next = normalizeText(value)
    if (next) return next
  }
  return ''
}

function assertValidationRunMatchesCurrentDataset(
  run: WorkflowRunArtifactSnapshot,
  runReq: Record<string, unknown>,
): void {
  const datasetMeta = run.datasetMeta
  const expectedCsvPath = normalizePath(normalizeText(datasetMeta?.csv_path ?? run.csvPath))
  const expectedTimeframe = canonicalLower(
    firstNonEmptyText(datasetMeta?.timeframe, datasetMeta?.derived_timeframe, datasetMeta?.base_timeframe),
  )
  if (!expectedCsvPath && !expectedTimeframe) return

  const dataStore = readPiniaStoreState('data')
  const currentCsvPath = normalizePath(normalizeText(dataStore?.csvPath))
  if (expectedCsvPath && !currentCsvPath) {
    throw new Error('Load the same dataset used by the optimization artifact before running validation.')
  }

  if (expectedCsvPath && currentCsvPath !== expectedCsvPath) {
    const expectedLabel = firstNonEmptyText(datasetMeta?.symbol, datasetMeta?.label, expectedCsvPath)
    throw new Error(`Validation requires the optimization dataset ${expectedLabel} (${expectedCsvPath}).`)
  }

  const currentTimeframe = canonicalLower(
    firstNonEmptyText(dataStore?.dataTimeframe, asRecord(dataStore?.baseDataMeta)?.timeframe),
  )
  if (currentTimeframe && expectedTimeframe && currentTimeframe !== expectedTimeframe) {
    throw new Error(`Validation requires timeframe ${expectedTimeframe}; current dataset is ${currentTimeframe}.`)
  }

  const artifactStrategy = normalizeStrategyKindRaw(runReq.strategy_kind ?? runReq.strategyKind)
  const runtimeStrategy = normalizeStrategyKindRaw(run.req && typeof run.req === 'object'
    ? (run.req as Record<string, unknown>).strategy_kind ?? (run.req as Record<string, unknown>).strategyKind
    : '')
  if (runtimeStrategy && artifactStrategy !== runtimeStrategy) {
    throw new Error(`Validation requires strategy ${runtimeStrategy}; current run context is ${artifactStrategy}.`)
  }
}

function patchValidationStoreState(patch: Record<string, unknown>): void {
  const store = readValidationStoreState()
  if (!store) return
  Object.assign(store, patch)
  store.source = 'vue'
  store.updatedAtUnixMs = Date.now()
}

function safePrettyJson(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2)
  } catch {
    return String(value ?? '')
  }
}

function toOutputPreview(value: unknown): string {
  const text = safePrettyJson(value)
  return text.length > 200 ? `${text.slice(0, 200).trimEnd()}…` : text
}

function queryDomById<T extends Element>(id: string): T | null {
  if (typeof document === 'undefined') return null
  const byQuery = typeof document.querySelector === 'function' ? document.querySelector(`[id="${id}"]`) : null
  if (byQuery) return byQuery as T
  const getById = (
    document as unknown as {
      getElementById?: (nextId: string) => Element | null
    }
  ).getElementById
  if (typeof getById !== 'function') return null
  return (getById.call(document, id) as T | null) ?? null
}

function writeDomTextContent(id: string, value: string): void {
  const el = queryDomById<HTMLElement>(id)
  if (!el) return
  el.textContent = value
}

function selectWorkflowResultsTab(tab: 'validate', persist: boolean): void {
  const pinia = getActivePinia()
  if (!pinia) return
  const workflowStore = useWorkflowStore(pinia)
  workflowStore.selectResultsTab(tab, { persist })
}

function buildValidationMetaText(result: unknown): string {
  const row = asRecord(result)
  const candidates = Array.isArray(row?.candidates) ? row.candidates.length : 0
  const runtimeMs = Math.max(0, Math.floor(Number(row?.runtime_ms ?? 0)))
  const backendUsed = normalizeText(row?.backend_used)
  const backendRoute = normalizeText(row?.backend_route)
  const backendFallbackReason = normalizeText(row?.backend_fallback_reason)
  const backendParts: string[] = []
  if (backendUsed) backendParts.push(`backend=${backendUsed}`)
  if (backendRoute) backendParts.push(`route=${backendRoute}`)
  if (backendFallbackReason) backendParts.push(`fallback=${backendFallbackReason}`)
  const backendSummary = backendParts.length > 0 ? backendParts.join(' • ') : 'backend'
  const cross = asRecord(row?.cross_ticker)
  const crossDatasets = Array.isArray(cross?.datasets) ? cross.datasets.length : 0
  const crossSuffix = crossDatasets > 0 ? ` • cross=${crossDatasets}` : ''

  const cscv = asRecord(row?.cscv_pbo)
  const cscvSkipped = normalizeText(cscv?.skipped_reason)
  const cscvCombos = Math.max(0, Math.floor(Number(cscv?.combinations ?? 0)))
  const cscvPbo = Number(cscv?.pbo ?? Number.NaN)
  let cscvSuffix = ''
  if (cscvSkipped) cscvSuffix = ' • cscv skipped'
  else if (cscv && cscvCombos > 0 && Number.isFinite(cscvPbo)) cscvSuffix = ` • pbo=${cscvPbo.toFixed(3)} (cs=${cscvCombos})`

  const reopt = asRecord(row?.reopt_walk_forward)
  const reoptWindows = Math.max(0, Math.floor(Number(reopt?.windows ?? 0)))
  const reoptSharpe = Number(reopt?.forward_sharpe_median ?? Number.NaN)
  const reoptSuffix = reoptWindows > 0
    ? ` • reopt_wf=${reoptWindows}w${Number.isFinite(reoptSharpe) ? ` sh_med=${reoptSharpe.toFixed(3)}` : ''}`
    : ''
  const spa = asRecord(row?.spa)
  const spaP = Number(spa?.p_value ?? Number.NaN)
  const spaSuffix = spa
    ? ` • spa=${Number.isFinite(spaP) ? spaP.toFixed(3) : '-'}${spa?.passed === false ? ' fail' : ' pass'}`
    : ''

  const runVerdict = asRecord(row?.run_verdict)
  const survived = Math.max(0, Math.floor(Number(runVerdict?.candidates_survived_hard_gates ?? 0)))
  const tested = Math.max(0, Math.floor(Number(runVerdict?.candidates_tested ?? candidates)))
  const approved = Math.max(0, Math.floor(Number(runVerdict?.approved_candidate_count ?? 0)))
  const recommendation = normalizeText(runVerdict?.final_recommendation)
  const topObs = Math.max(0, Math.floor(Number(runVerdict?.top_candidate_track_record_bars ?? 0)))
  const topMinTrl = Math.max(0, Math.floor(Number(runVerdict?.top_candidate_min_track_record_length_bars ?? 0)))
  const approvalSuffix = runVerdict
    ? ` • approved=${approved}/${survived}${recommendation ? ` • rec=${recommendation}` : ''}`
    : ''
  const minTrlSuffix = topObs > 0 && topMinTrl > 0
    ? ` • top_mintrl=${topObs}/${topMinTrl}`
    : ''
  const hasRunVerdict = !!runVerdict
  const runState = hasRunVerdict ? ` • hard_gates=${survived}/${tested}${runVerdict?.passed === false ? ' fail' : ''}` : ''

  return `validated ${candidates} candidates in ${runtimeMs}ms (${backendSummary})${crossSuffix}${cscvSuffix}${reoptSuffix}${spaSuffix}${runState}${approvalSuffix}${minTrlSuffix}`
}

function compactWinnerLabel(value: unknown): string {
  const raw = normalizeText(value)
  if (!raw) return '—'
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>
    const fastMa = normalizeText(parsed.fast_ma_type || '?')
    const slowMa = normalizeText(parsed.slow_ma_type || '?')
    const fastLen = Math.max(0, Math.floor(Number(parsed.fast_len || 0)))
    const slowLen = Math.max(0, Math.floor(Number(parsed.slow_len || 0)))
    const base = `${fastMa || '?'}(${fastLen || '?'})/${slowMa || '?'}(${slowLen || '?'})`
    const overlays: string[] = []
    for (const key of ['asr', 'vcc', 'vemr', 'rocb', 'mkcs']) {
      const entry = asRecord(parsed[key])
      if (entry) overlays.push(key)
    }
    if (parsed.halftrend || parsed.halftrend_filter) overlays.push('halftrend')
    return overlays.length > 0 ? `${base}+${overlays.join('+')}` : base
  } catch {
    return raw.length > 40 ? `${raw.slice(0, 37)}...` : raw
  }
}

function buildValidationReoptMetaText(result: unknown): string {
  const row = asRecord(result)
  const reopt = asRecord(row?.reopt_walk_forward)
  if (!reopt) return '-'
  const windows = Math.max(0, Math.floor(Number(reopt.windows ?? 0)))
  if (windows <= 0) return '-'

  const topN = Math.max(1, Math.floor(Number(reopt.top_n_candidates ?? 1)))
  const uniqueWinners = Math.max(0, Math.floor(Number(reopt.unique_winners ?? 0)))
  const winnerChanges = Math.max(0, Math.floor(Number(reopt.winner_changes ?? 0)))
  const driftMean = Number(reopt.winner_param_drift_mean ?? Number.NaN)
  const driftMax = Number(reopt.winner_param_drift_max ?? Number.NaN)
  const hitRate = Number(reopt.forward_positive_pnl_frac ?? Number.NaN)
  const sharpeMedian = Number(reopt.forward_sharpe_median ?? Number.NaN)
  const ddWorst = Number(reopt.forward_max_dd_worst ?? Number.NaN)
  const sequenceRaw = Array.isArray(reopt.winner_sequence) ? reopt.winner_sequence : []
  const sequence = sequenceRaw
    .slice(0, 4)
    .map((entry) => compactWinnerLabel(entry))
    .filter((entry) => entry.length > 0)
  const sequenceSuffix = sequence.length > 0
    ? ` • winners=${sequence.join(' -> ')}${sequenceRaw.length > sequence.length ? ' -> ...' : ''}`
    : ''

  return `reopt_wf: ${windows} windows • topN=${topN} • unique=${uniqueWinners} • changes=${winnerChanges}` +
    `${Number.isFinite(hitRate) ? ` • oos_hit=${fmtPct(hitRate, 1)}` : ''}` +
    `${Number.isFinite(sharpeMedian) ? ` • sh_med=${sharpeMedian.toFixed(3)}` : ''}` +
    `${Number.isFinite(ddWorst) ? ` • dd_worst=${ddWorst.toFixed(3)}` : ''}` +
    `${Number.isFinite(driftMean) ? ` • drift_mean=${driftMean.toFixed(3)}` : ''}` +
    `${Number.isFinite(driftMax) ? ` • drift_max=${driftMax.toFixed(3)}` : ''}` +
    sequenceSuffix
}

function fmt(value: unknown, digits = 6): string {
  const n = Number(value)
  if (!Number.isFinite(n)) return '-'
  return n.toFixed(digits)
}

function fmtFixed(value: number, digits = 2): string {
  if (!Number.isFinite(value)) return '-'
  return value.toFixed(digits)
}

function fmtPct(value: unknown, digits = 1): string {
  const n = Number(value)
  if (!Number.isFinite(n)) return '-'
  return `${(n * 100).toFixed(digits)}%`
}

function parseProbabilityGateValue(value: unknown): number | null {
  const raw = normalizeText(value)
  if (!raw) return null
  const n = Number(raw)
  if (!Number.isFinite(n)) return null
  return Math.max(0, Math.min(1, n))
}

function parseFiniteNumberValue(value: unknown): number | null {
  const raw = normalizeText(value)
  if (!raw) return null
  const n = Number(raw)
  return Number.isFinite(n) ? n : null
}

function validationCandidates(result: unknown): Array<Record<string, unknown>> {
  const row = asRecord(result)
  const xs = Array.isArray(row?.candidates) ? row.candidates : []
  const out: Array<Record<string, unknown>> = []
  for (const entry of xs) {
    const candidate = asRecord(entry)
    if (!candidate) continue
    out.push(candidate)
  }
  return out
}

function computeValidationShortlist(
  result: unknown,
  minPsr: number | null,
  minDsr: number | null,
  minCscvRank: number | null,
  minCscvMetric: number | null
): {
  rows: Array<Record<string, unknown>>
  candidates: Array<Record<string, unknown>>
  strictBase: Array<Record<string, unknown>>
  hasCross: boolean
  usedFallback: boolean
  hasCscv: boolean
  cscvCombos: number
  cscvThreshold: number
  cscvRankMin: number
  usedCscvFallback: boolean
  hasProbGate: boolean
  probPassN: number
  hasBackendVerdicts: boolean
  backendPassN: number
  runVerdict: Record<string, unknown> | null
} {
  const CSCV_RANK_FRAC_MIN = 0.55
  const candidates = validationCandidates(result)
  const hasProbGate = minPsr != null || minDsr != null
  const cscvRankThreshold = minCscvRank ?? CSCV_RANK_FRAC_MIN
  const runVerdict = asRecord(asRecord(result)?.run_verdict)
  const hasBackendVerdicts = candidates.some((candidate) => {
    const verdict = asRecord(candidate.validation_verdict)
    return !!verdict && typeof verdict.passed === 'boolean'
  })
  const passProbabilityGates = (candidate: Record<string, unknown>): boolean => {
    if (!hasProbGate) return true
    const psr = Number(candidate.psr)
    const dsr = Number(candidate.dsr)
    if (minPsr != null && (!Number.isFinite(psr) || psr < minPsr)) return false
    if (minDsr != null && (!Number.isFinite(dsr) || dsr < minDsr)) return false
    return true
  }

  const hasCross = candidates.some((candidate) => {
    const cross = candidate.cross_ticker
    return !!(cross && typeof cross === 'object')
  })

  const strict = candidates.filter((candidate) => {
    if (!candidate.robust_pareto) return false
    if (hasCross) return !!candidate.cross_ticker_pareto
    return true
  })

  const resultRecord = asRecord(result)
  const cscv = asRecord(resultRecord?.cscv_pbo)
  const cscvCombos = Math.max(0, Math.floor(Number(cscv?.combinations ?? 0)))
  const cscvThreshold = Number(cscv?.threshold ?? 0)
  const hasCscv =
    cscvCombos > 0 &&
    candidates.some((candidate) => {
      const stats = candidate.cscv_pbo
      return !!(stats && typeof stats === 'object')
    })

  const baseRows = strict.length > 0 ? strict : candidates
  const usedFallback = strict.length === 0

  const passCscv = (candidate: Record<string, unknown>): boolean => {
    const verdict = asRecord(candidate.validation_verdict)
    if (verdict && typeof verdict.passed === 'boolean') return verdict.passed === true
    const stats = asRecord(candidate.cscv_pbo)
    if (!stats) return false
    const selectedFrac = Number(stats.is_selected_frac ?? 0)
    const rankFrac = Number(stats.oos_rank_frac_p50 ?? 0)
    const metric = Number(stats.oos_metric_p50 ?? Number.NaN)
    if (!Number.isFinite(selectedFrac) || selectedFrac <= 0) return false
    if (!Number.isFinite(rankFrac) || rankFrac < cscvRankThreshold) return false
    const requiredMetric = minCscvMetric != null ? minCscvMetric : cscvThreshold
    if (!Number.isFinite(metric) || metric < requiredMetric) return false
    return true
  }

  const probFiltered = hasProbGate ? baseRows.filter(passProbabilityGates) : baseRows
  const cscvFiltered = hasCscv || hasBackendVerdicts ? probFiltered.filter(passCscv) : probFiltered
  const usedCscvFallback = !hasBackendVerdicts && hasCscv && baseRows.length > 0 && cscvFiltered.length === 0
  const pickedRows = hasProbGate ? cscvFiltered : usedCscvFallback ? baseRows : cscvFiltered
  const backendPassN = hasBackendVerdicts
    ? candidates.filter((candidate) => asRecord(candidate.validation_verdict)?.passed === true).length
    : 0

  const sortByFinal = (left: Record<string, unknown>, right: Record<string, unknown>): number => {
    const robustL = Number(left.robust_score ?? 0)
    const robustR = Number(right.robust_score ?? 0)
    if (robustR !== robustL) return robustR - robustL

    const cscvL = asRecord(left.cscv_pbo)
    const cscvR = asRecord(right.cscv_pbo)
    const rankL = Number(cscvL?.oos_rank_frac_p50 ?? 0)
    const rankR = Number(cscvR?.oos_rank_frac_p50 ?? 0)
    if (rankR !== rankL) return rankR - rankL

    const selectedL = Number(cscvL?.is_selected_frac ?? 0)
    const selectedR = Number(cscvR?.is_selected_frac ?? 0)
    if (selectedR !== selectedL) return selectedR - selectedL

    const metricL = Number(cscvL?.oos_metric_p50 ?? 0)
    const metricR = Number(cscvR?.oos_metric_p50 ?? 0)
    if (metricR !== metricL) return metricR - metricL

    return 0
  }

  return {
    rows: [...pickedRows].sort(sortByFinal),
    candidates,
    strictBase: strict,
    hasCross,
    usedFallback,
    hasCscv,
    cscvCombos,
    cscvThreshold,
    cscvRankMin: cscvRankThreshold,
    usedCscvFallback,
    hasProbGate,
    probPassN: probFiltered.length,
    hasBackendVerdicts,
    backendPassN,
    runVerdict
  }
}

function buildValidationShortlistMetaText(shortlist: ReturnType<typeof computeValidationShortlist>, minPsr: number | null, minDsr: number | null): string {
  const robustParetoN = shortlist.candidates.filter((candidate) => !!candidate.robust_pareto).length
  const crossParetoN = shortlist.hasCross ? shortlist.candidates.filter((candidate) => !!candidate.cross_ticker_pareto).length : 0
  const baseRule = shortlist.hasCross ? 'robust_pareto && cross_ticker_pareto' : 'robust_pareto'
  const probRule = shortlist.hasProbGate ? `prob(${minPsr != null ? `psr>=${fmtFixed(minPsr, 2)}` : ''}${minPsr != null && minDsr != null ? ',' : ''}${minDsr != null ? `dsr>=${fmtFixed(minDsr, 2)}` : ''})` : ''
  const cscvRule = shortlist.hasCscv ? `cscv(oos_rank_p50>=${shortlist.cscvRankMin}, oos_sharpe_p50>=${shortlist.cscvThreshold})` : ''
  const backendRule = shortlist.hasBackendVerdicts ? 'backend_hard_gates' : ''
  const rule = [baseRule, probRule, cscvRule, backendRule].filter((part) => part.length > 0).join(' && ')
  const baseFallbackNote = shortlist.usedFallback ? ' • (fallback: top robust score)' : ''
  const cscvFallbackNote = shortlist.usedCscvFallback && !shortlist.hasProbGate ? ' • (cscv fallback)' : ''
  const crossNote = shortlist.hasCross ? ` • cross_pareto=${crossParetoN}/${shortlist.candidates.length}` : ''
  const strictCount = shortlist.strictBase.length > 0 ? shortlist.strictBase.length : shortlist.candidates.length
  const probNote = shortlist.hasProbGate ? ` • prob_pass=${shortlist.probPassN}/${strictCount}` : ''
  const cscvNote = shortlist.hasCscv ? ` • cscv=${shortlist.cscvCombos} combos` : ''
  const cscvPassNote = shortlist.hasCscv ? ` • cscv_pass=${shortlist.rows.length}/${strictCount}` : ''
  const backendNote = shortlist.hasBackendVerdicts ? ` • backend_pass=${shortlist.backendPassN}/${shortlist.candidates.length}` : ''
  const approvedCount = Math.max(0, Math.floor(Number(shortlist.runVerdict?.approved_candidate_count ?? 0)))
  const firstApprovedRank = Math.max(0, Math.floor(Number(shortlist.runVerdict?.first_approved_rank ?? 0)))
  const runVerdictRecommendation = normalizeText(shortlist.runVerdict?.final_recommendation)
  const runVerdictNote = shortlist.runVerdict
    ? ` • run=${shortlist.runVerdict.passed === false ? 'fail' : 'pass'} • approved=${approvedCount}${firstApprovedRank > 0 ? `@#${firstApprovedRank}` : ''}${runVerdictRecommendation ? ` • rec=${runVerdictRecommendation}` : ''}`
    : ''
  return `shortlist=${shortlist.rows.length}/${shortlist.candidates.length} • robust_pareto=${robustParetoN}/${shortlist.candidates.length}${crossNote}${probNote}${cscvNote}${cscvPassNote}${backendNote}${runVerdictNote} • rule=${rule}${baseFallbackNote}${cscvFallbackNote}`
}

function buildValidationTableColumns(): ValidationTableColumn[] {
  return [
    { label: '#', alignLeft: true },
    { label: 'Fast', alignLeft: true },
    { label: 'Slow', alignLeft: true },
    { label: 'Robust', alignLeft: false },
    { label: 'PSR', alignLeft: false },
    { label: 'DSR', alignLeft: false },
    { label: 'CSCV Rank', alignLeft: false },
    { label: 'Gate', alignLeft: true },
    { label: 'Robust Pareto', alignLeft: true },
    { label: 'Cross', alignLeft: false },
    { label: 'Cross Pareto', alignLeft: true },
    { label: 'PnL', alignLeft: false },
    { label: 'Sharpe', alignLeft: false },
    { label: 'Max DD', alignLeft: false },
    { label: 'Trades', alignLeft: false }
  ]
}

function buildValidationTableRows(shortlistRows: Array<Record<string, unknown>>, hasCross: boolean): ValidationTableRow[] {
  const rows: ValidationTableRow[] = []
  for (let i = 0; i < shortlistRows.length; i += 1) {
    const candidate = shortlistRows[i] || {}
    const params = asRecord(candidate.params) ?? {}
    const inSample = asRecord(candidate.in_sample) ?? {}
    const cross = asRecord(candidate.cross_ticker)
    const cscv = asRecord(candidate.cscv_pbo)
    const verdict = asRecord(candidate.validation_verdict)
    const failReasons = Array.isArray(verdict?.fail_reasons) ? verdict.fail_reasons.map((entry) => normalizeText(entry)).filter((entry) => entry.length > 0) : []
    const warningFlags = Array.isArray(verdict?.warning_flags) ? verdict.warning_flags.map((entry) => normalizeText(entry)).filter((entry) => entry.length > 0) : []
    const gateText = verdict
      ? verdict.passed === false
        ? failReasons[0] || 'fail'
        : warningFlags.length
          ? `pass*`
          : 'pass'
      : '-'
    const gateTitle = verdict
      ? [...failReasons, ...warningFlags].filter((entry) => entry.length > 0).join(' • ')
      : ''

    rows.push({
      cells: [
        { text: String(i + 1), className: 'left mono', title: '' },
        {
          text: `${normalizeText(params.fast_ma_type || '?')}(${Math.max(0, Math.floor(Number(params.fast_len || 0))) || '?'})`,
          className: 'left mono',
          title: ''
        },
        {
          text: `${normalizeText(params.slow_ma_type || '?')}(${Math.max(0, Math.floor(Number(params.slow_len || 0))) || '?'})`,
          className: 'left mono',
          title: ''
        },
        { text: fmt(candidate.robust_score, 6), className: 'mono', title: '' },
        {
          text: candidate.psr != null ? fmt(candidate.psr, 4) : '-',
          className: 'mono',
          title: ''
        },
        {
          text: candidate.dsr != null ? fmt(candidate.dsr, 4) : '-',
          className: 'mono',
          title: ''
        },
        {
          text: cscv ? fmtPct(cscv.oos_rank_frac_p50, 1) : '-',
          className: 'mono',
          title: ''
        },
        {
          text: gateText,
          className: 'left mono',
          title: gateTitle
        },
        {
          text: candidate.robust_pareto ? 'robust' : '—',
          className: 'left',
          title: ''
        },
        {
          text: cross ? fmt(cross.score, 6) : '-',
          className: 'mono',
          title: ''
        },
        {
          text: hasCross ? (candidate.cross_ticker_pareto ? 'cross' : '—') : '—',
          className: 'left',
          title: ''
        },
        { text: fmt(inSample.pnl, 6), className: 'mono', title: '' },
        { text: fmt(inSample.sharpe, 6), className: 'mono', title: '' },
        { text: fmt(inSample.max_dd, 6), className: 'mono', title: '' },
        {
          text: String(Math.max(0, Math.floor(Number(inSample.trades || 0)))),
          className: 'mono',
          title: ''
        }
      ]
    })
  }
  return rows
}

async function persistValidationArtifact(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  options: { reportXlsxPath?: string } = {}
): Promise<{
  path: string
  ts: string
  csvPath: string
  reportXlsxPath: string
  tags: string[]
}> {
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
    kind: 'vectorbt_run_artifact',
    ts,
    config_fingerprint: configFingerprint,
    run: { req: runReq, res: runRes },
    validation: { req: validationReq, res: validationRes },
    report_xlsx_path: reportXlsxPath,
    data: { csv_path: csvPath },
    dataset_meta: datasetMeta,
    tags
  }
  let path = normalizeText(currentRun?.artifactPath)
  try {
    const saved = (await invokeCommand('save_run_artifact', {
      artifact
    })) as SaveRunArtifactResponse
    const next = normalizeText(saved?.path)
    if (next) path = next
  } catch {
    // keep validation non-throwing if artifact persistence fails
  }

  return { path, ts, csvPath, reportXlsxPath, tags }
}

function applyVueValidationArtifacts(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  persisted: {
    path: string
    ts: string
    csvPath: string
    reportXlsxPath: string
    tags: string[]
  }
): void {
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: persisted.ts,
        artifact_path: persisted.path,
        report_xlsx_path: persisted.reportXlsxPath,
        csv_path: persisted.csvPath,
        tags: persisted.tags,
        req: runReq,
        res: runRes
      },
      currentValidation: {
        ts: persisted.ts,
        req: validationReq,
        res: validationRes
      }
    },
    'vue'
  )
}

function resolveValidationBuildContextFromWorkflowArtifacts(): LegacyValidationBuildContext | null {
  const runSnapshot = readRunSnapshotFromStore()
  const runArtifact = readWorkflowRunArtifactFromStore()
  const runReq = runSnapshot?.req ?? runArtifact?.req ?? null
  const runRes = runSnapshot?.res ?? runArtifact?.res ?? null
  const snapshotTopRows = extractTopRowsFromRunResult(runRes)
  if (!runReq || !runRes || snapshotTopRows.length === 0) return null
  return {
    currentRunReq: runReq,
    currentRunRes: runRes,
    topRows: snapshotTopRows,
    topRowsFiltered: []
  }
}

export async function runValidation(options: ValidationRunOptions = {}): Promise<void> {
  const buildContext = resolveValidationBuildContextFromWorkflowArtifacts()
  if (!buildContext) {
    const message = 'Run an optimization first.'
    patchValidationStoreState({
      statusLine: message,
      validationHint: message
    })
    writeDomTextContent('status', message)
    throw new Error(message)
  }
  const runSnapshot = readRunSnapshotFromStore()
  const workflowRun = readWorkflowRunArtifactFromStore()
  const runReq = asRecord(runSnapshot?.req ?? buildContext.currentRunReq)
  const runRes = runSnapshot?.res ?? buildContext.currentRunRes
  if (!runReq || !runRes) {
    const message = 'Run artifact is unavailable. Re-run optimization first.'
    patchValidationStoreState({
      statusLine: message,
      validationHint: message
    })
    writeDomTextContent('status', message)
    throw new Error(message)
  }
  if (!workflowRun) {
    const message = 'Run artifact is unavailable. Re-run optimization first.'
    patchValidationStoreState({
      statusLine: message,
      validationHint: message
    })
    writeDomTextContent('status', message)
    throw new Error(message)
  }
  assertValidationRunMatchesCurrentDataset(workflowRun, runReq)

  const buildInputs: Parameters<typeof buildValidationRequest>[0] = {
    currentRunReq: runReq,
    currentRunRes: runRes,
    runDatasetMeta: workflowRun.datasetMeta,
    runCsvPath: workflowRun.csvPath,
    topRows: buildContext.topRows,
    topRowsFiltered: buildContext.topRowsFiltered,
    candidates: readStoreTextInputValue('valCandidates'),
    seed: readStoreTextInputValue('valSeed'),
    minPsr: readStoreTextInputValue('valMinPsr'),
    minDsr: readStoreTextInputValue('valMinDsr'),
    minCscvOosRankFrac: readStoreTextInputValue('valMinCscvRank'),
    minCscvOosMetric: readStoreTextInputValue('valMinCscvMetric'),
    maxPbo: readStoreTextInputValue('valMaxPbo'),
    spaEnable: readStoreCheckboxValue('valSpaEnable'),
    spaSlices: readStoreTextInputValue('valSpaSlices'),
    spaBootstrapSamples: readStoreTextInputValue('valSpaBootstrapSamples'),
    spaBlockLength: readStoreTextInputValue('valSpaBlockLength'),
    reoptWfEnable: readStoreCheckboxValue('valReoptWfEnable'),
    reoptWfWindow: readStoreTextInputValue('valReoptWfWindow'),
    reoptWfStep: readStoreTextInputValue('valReoptWfStep'),
    reoptWfTopN: readStoreTextInputValue('valReoptWfTopN'),
    wfEnable: readStoreCheckboxValue('valWfEnable'),
    wfWindow: readStoreTextInputValue('valWfWindow'),
    wfStep: readStoreTextInputValue('valWfStep'),
    perturbEnable: readStoreCheckboxValue('valPerturbEnable'),
    perturbIters: readStoreTextInputValue('valPerturbIters'),
    perturbMin: readStoreTextInputValue('valPerturbMin'),
    perturbMax: readStoreTextInputValue('valPerturbMax'),
    mcEnable: readStoreCheckboxValue('valMcEnable'),
    mcSims: readStoreTextInputValue('valMcSims'),
    cscvEnable: readStoreCheckboxValue('valCscvEnable'),
    cscvSlices: readStoreTextInputValue('valCscvSlices'),
    crossEnable: readStoreCheckboxValue('valCrossEnable'),
    crossMax: readStoreTextInputValue('valCrossMax'),
    crossDatasets: readVueCrossSelection()
  }
  if (typeof options.useFiltered === 'boolean') buildInputs.useFiltered = options.useFiltered

  const validationReq = buildValidationRequest(buildInputs) as unknown as Record<string, unknown>
  const command = resolveValidationCommand(runReq)
  const switchTab = options.switchTab !== false

  patchValidationStoreState({
    validateDisabled: true,
    statusLine: 'Running validation...',
    validationHint: 'Running validation...',
    validationReoptMeta: '-'
  })
  writeDomTextContent('status', 'Running validation...')

  try {
    const startedAt = typeof performance !== 'undefined' ? performance.now() : Date.now()
    const validationRes = await invokeCommand(command, { req: validationReq })
    const finishedAt = typeof performance !== 'undefined' ? performance.now() : Date.now()
    const elapsedMs = Math.max(0, finishedAt - startedAt)
    const validationMeta = buildValidationMetaText(validationRes)
    const validationReoptMeta = buildValidationReoptMetaText(validationRes)
    const minPsrGate = parseProbabilityGateValue(validationReq.min_psr)
    const minDsrGate = parseProbabilityGateValue(validationReq.min_dsr)
    const minCscvRankGate = parseProbabilityGateValue(validationReq.min_cscv_oos_rank_frac)
    const minCscvMetricGate = parseFiniteNumberValue(validationReq.min_cscv_oos_metric)
    const shortlist = computeValidationShortlist(validationRes, minPsrGate, minDsrGate, minCscvRankGate, minCscvMetricGate)
    const validationColumns = buildValidationTableColumns()
    const validationRows = buildValidationTableRows(shortlist.rows, shortlist.hasCross)
    const validationShortlistMeta = validationRows.length > 0 ? buildValidationShortlistMetaText(shortlist, minPsrGate, minDsrGate) : 'No validation result yet.'
    const statusLine = `Validation done in ${elapsedMs.toFixed(0)}ms (frontend)`
    const persisted = await persistValidationArtifact(runReq, runRes, validationReq, validationRes)

    applyVueValidationArtifacts(runReq, runRes, validationReq, validationRes, persisted)
    patchValidationStoreState({
      validationHint: 'Validation complete.',
      validationMeta,
      validationReoptMeta,
      validationColumns,
      validationRows,
      validationOutputPreview: toOutputPreview(validationRes),
      tableRowCount: validationRows.length,
      tableVisible: validationRows.length > 0,
      canExportValidationCsv: true,
      canExportValidationJson: true,
      canExportReportJson: true,
      canExportReportXlsx: true,
      validationShortlistMeta,
      statusLine
    })
    writeDomTextContent('validationMeta', validationMeta)
    writeDomTextContent('status', statusLine)

    if (switchTab) selectWorkflowResultsTab('validate', true)
  } catch (error) {
    const message = String(error || '').trim() || 'Validation failed.'
    patchValidationStoreState({
      statusLine: message,
      validationHint: message,
      validationReoptMeta: '-'
    })
    writeDomTextContent('status', message)
    throw error
  } finally {
    patchValidationStoreState({ validateDisabled: false })
  }
}

export async function exportShortlistCsv(): Promise<void> {
  const run = readWorkflowRunArtifactFromStore()
  const validation = readWorkflowValidationArtifactFromStore()
  const validationRes = validation?.res ?? null
  if (!validationRes) return
  const ts = normalizeText(run?.ts || validation?.ts) || new Date().toISOString()
  const csv = await invokeCommand('build_validation_shortlist_csv', {
    validation_res: validationRes
  })
  const downloaded = downloadText(`vectorbt_validation_shortlist_${ts}.csv`, String(csv || ''), 'text/csv;charset=utf-8')
  patchValidationStoreState({
    statusLine: downloaded ? 'Exported validation shortlist CSV.' : 'Built validation shortlist CSV (download unavailable in this environment).'
  })
}

export async function exportValidationJson(): Promise<void> {
  const run = readWorkflowRunArtifactFromStore()
  const validation = readWorkflowValidationArtifactFromStore()
  const validationRes = validation?.res ?? null
  if (!validationRes) return
  const ts = normalizeText(run?.ts || validation?.ts) || new Date().toISOString()
  const downloaded = downloadText(`vectorbt_validation_${ts}.json`, safePrettyJson(validationRes), 'application/json;charset=utf-8')
  patchValidationStoreState({
    statusLine: downloaded ? 'Exported validation JSON.' : 'Built validation JSON (download unavailable in this environment).'
  })
}

export async function exportReportJson(): Promise<void> {
  const report = buildCurrentReportPayloadFromStore()
  const hasRun = !!(report.run && report.run.res)
  if (!hasRun) return
  const ts = normalizeText(report.ts) || new Date().toISOString()
  const downloaded = downloadText(`vectorbt_report_${ts}.json`, safePrettyJson(report), 'application/json;charset=utf-8')
  patchValidationStoreState({
    statusLine: downloaded ? 'Exported report JSON.' : 'Built report JSON (download unavailable in this environment).'
  })
}

export async function exportReportXlsx(): Promise<void> {
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
        statusLine: 'Run validation first to export report XLSX (or run Meta Analysis to open Stage 3 XLSX).'
      })
      return
    }
    await openPathWithRevealFallback(stage3Path)
    if (autoReveal) {
      await invokeCommand('reveal_path', { path: stage3Path }).catch(() => undefined)
    }
    patchValidationStoreState({
      statusLine: `Opened Stage 3 XLSX report: ${stage3Path}`
    })
    return
  }

  let path = ''
  try {
    const saved = (await invokeCommand('save_report_xlsx', {
      report,
      max_rows: maxRows
    })) as SaveReportXlsxResponse
    path = normalizeText(saved?.path)
  } catch (saveError) {
    try {
      const base64 = await invokeCommand('build_report_xlsx', { report })
      const downloaded = downloadBase64(`vectorbt_report_${ts}.xlsx`, String(base64 || ''), 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet')
      patchValidationStoreState({
        statusLine: downloaded ? 'Exported XLSX report as a file download.' : 'Built XLSX report (download unavailable in this environment).'
      })
      return
    } catch (fallbackError) {
      const message = `Failed to export XLSX report: ${String(saveError)} | fallback failed: ${String(fallbackError)}`
      patchValidationStoreState({ statusLine: message })
      throw new Error(message)
    }
  }

  if (!path) {
    patchValidationStoreState({
      statusLine: 'Saved XLSX report, but no path was returned.'
    })
    return
  }

  if (runReq && runRes != null) {
    const persisted = await persistValidationArtifact(runReq, runRes, validationReq, validationRes, { reportXlsxPath: path })
    applyVueValidationArtifacts(runReq, runRes, validationReq, validationRes, persisted)
    patchValidationStoreState({ statusLine: `Saved XLSX report: ${path}` })
  } else {
    patchValidationStoreState({ statusLine: `Saved XLSX report: ${path}` })
  }

  await openPathWithRevealFallback(path)
  if (autoReveal) {
    await invokeCommand('reveal_path', { path }).catch(() => undefined)
  }
}

export async function openReportsDir(): Promise<void> {
  await invokeCommand('open_reports_dir')
}

export async function crossRefresh(): Promise<ValidationCrossRefreshSummary> {
  const raw = (await invokeCommand('polygon_get_cache_info')) as PolygonCacheInfoResponse
  const datasets = Array.isArray(raw?.datasets) ? raw.datasets : []

  const entries: ValidationCrossDatasetSummary[] = []
  for (const row of datasets) {
    const dataset = (row && typeof row === 'object' ? row : {}) as PolygonCrossDatasetResponse
    const path = normalizeText(dataset.csv_path)
    if (!path) continue

    const label = normalizeText(dataset.display_symbol || dataset.polygon_ticker || path) || path
    const market = normalizeText(dataset.market)
    const timeframe = normalizeText(dataset.timeframe) || '1m'
    const adjusted = market === 'stocks' ? !!dataset.adjusted : null
    const minTs = formatYmdUtc(dataset.min_ts)
    const maxTs = formatYmdUtc(dataset.max_ts)
    const updated = formatYmdUtc(dataset.last_updated_utc_ms)
    const rowsHint = normalizeRowsHint(dataset.row_count)
    const marketLabel = normalizeMarketLabel(market, adjusted === true)

    entries.push({
      path,
      label,
      meta: `${marketLabel} • tf=${timeframe} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}`,
      market,
      timeframe: timeframe.toLowerCase(),
      adjusted
    })
  }

  entries.sort((a, b) => `${a.market}${a.label}`.localeCompare(`${b.market}${b.label}`))
  return { datasets: entries }
}

export async function toggleCrossDataset(path: string, checked: boolean): Promise<void> {
  const nextPath = normalizePath(path)
  if (!nextPath) return
  const selected = readVueCrossSelection()
  const nextSelected: CrossSelectionEntry[] = []
  let present = false
  for (const item of selected) {
    const csvPath = normalizeText(item?.csv_path ?? item?.path)
    if (!csvPath) continue
    if (csvPath === nextPath) {
      present = true
      if (checked) {
        nextSelected.push({
          csv_path: csvPath,
          label: normalizeText(item?.label)
        })
      }
      continue
    }
    nextSelected.push({
      csv_path: csvPath,
      label: normalizeText(item?.label)
    })
  }
  if (checked && !present) {
    nextSelected.push({ csv_path: nextPath, label: '' })
  }
  writeVueCrossSelection(nextSelected)
}
