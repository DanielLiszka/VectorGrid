import { getActivePinia } from 'pinia'
import { markRaw } from 'vue'
import { invokeCommand } from '../tauri/invoke'
import { applyTimeframe as applyDataTimeframe, loadCsv as loadDataCsv } from './dataService'
import { buildOptimizationRequestFromVueStores, runOptimization } from './optimizeService'
import { runValidation } from './validationService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { applyWorkflowArtifactsPayload, useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { LAST_BATCH_SUMMARY_KEY } from '../utils/core/storageKeys'
import { computeConfigFingerprint, defaultBatchTag, dedupTags, splitHoldoutPaths } from '../utils/core/artifactIdentity'
import { readStorageString, removeStorageKey, writeStorageString } from '../utils/storage'
import { buildMetaBatchRequest } from './metaBatchRequestBuilder'
import { buildMetaMlScope } from './metaMlScopeBuilder'
import { buildMetaSelectionRequest } from './metaRequestBuilder'
import { buildMetaStage4Request } from './metaStage4RequestBuilder'

export interface MetaRunOptions {
  request?: unknown
  switchTab?: boolean
}

export interface MetaBatchRunOptions {
  request?: unknown
}

export interface MetaStage4RunOptions {
  request?: unknown
}

export interface MetaMlActionOptions {
  scope?: unknown
}

interface VueStoreFieldBinding {
  key: string
}

interface MetaTableColumn {
  label: string
  alignLeft: boolean
}

interface MetaTableCell {
  text: string
  className: string
  title: string
}

interface MetaTableRow {
  cells: MetaTableCell[]
}

interface BatchDatasetEntry {
  path: string
  label: string
  meta: string
  market: string
  timeframe: string
  adjusted: boolean | null
  source: string
  source_size_bytes: number | null
  source_mtime_utc_ms: number | null
  source_content_fingerprint: string
}

interface BatchSummaryEnvelope {
  path: string
  summary: Record<string, unknown>
}

type AnalysisStore = ReturnType<typeof useAnalysisStore>

const META_TABLE_COLUMNS: MetaTableColumn[] = markRaw([
  { label: '#', alignLeft: false },
  { label: 'Samples', alignLeft: false },
  { label: 'Pass', alignLeft: false },
  { label: 'Meta', alignLeft: false },
  { label: 'ML', alignLeft: false },
  { label: 'Robust p10', alignLeft: false },
  { label: 'Robust med', alignLeft: false },
  { label: 'DD p90', alignLeft: false },
  { label: 'PSR', alignLeft: false },
  { label: 'DSR', alignLeft: false },
  { label: 'H pass', alignLeft: false },
  { label: 'H p10', alignLeft: false },
  { label: 'H med', alignLeft: false },
  { label: 'H DD p90', alignLeft: false },
  { label: 'Fast', alignLeft: true },
  { label: 'Slow', alignLeft: true },
  { label: 'Params', alignLeft: true },
]) as MetaTableColumn[]

const ANALYSIS_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  metaStrategyKind: { key: 'metaStrategyKind' },
  metaTimeframe: { key: 'metaTimeframe' },
  metaRequiredTag: { key: 'metaRequiredTag' },
  metaMinSamples: { key: 'metaMinSamples' },
  metaTopK: { key: 'metaTopK' },
  metaHoldoutTag: { key: 'metaHoldoutTag' },
  metaHoldoutMode: { key: 'metaHoldoutMode' },
  metaHoldoutEvalTopN: { key: 'metaHoldoutEvalTopN' },
  metaHoldoutExploreN: { key: 'metaHoldoutExploreN' },
  metaHoldoutEvalRankBy: { key: 'metaHoldoutEvalRankBy' },
  metaHoldoutReps: { key: 'metaHoldoutReps' },
  metaGroupingMode: { key: 'metaGroupingMode' },
  metaClusterIntBucket: { key: 'metaClusterIntBucket' },
  metaClusterFloatBucket: { key: 'metaClusterFloatBucket' },
  batchHoldoutPct: { key: 'batchHoldoutPct' },
  batchHoldoutTag: { key: 'batchHoldoutTag' },
  batchTags: { key: 'batchTags' },
  metaStage4TopK: { key: 'metaStage4TopK' },
  metaStage4MinConf: { key: 'metaStage4MinConf' },
  metaStage4MetaWeight: { key: 'metaStage4MetaWeight' },
  metaStage4MlWeight: { key: 'metaStage4MlWeight' },
  metaStage4UncertaintyWeight: { key: 'metaStage4UncertaintyWeight' },
  metaStage4AcceptConf: { key: 'metaStage4AcceptConf' },
  metaStage4ReviewConf: { key: 'metaStage4ReviewConf' },
  metaStage4MaxAcceptUncertainty: { key: 'metaStage4MaxAcceptUncertainty' },
}

const ANALYSIS_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  metaEvalHoldout: { key: 'metaEvalHoldout' },
  metaSaveHoldoutRows: { key: 'metaSaveHoldoutRows' },
  batchAutoMeta: { key: 'batchAutoMeta' },
  batchSkipDone: { key: 'batchSkipDone' },
  metaStage4AutoTrain: { key: 'metaStage4AutoTrain' },
  metaStage4RequireHealthy: { key: 'metaStage4RequireHealthy' },
  metaStage4FallbackToMeta: { key: 'metaStage4FallbackToMeta' },
}

const VALIDATION_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valSeed: { key: 'seed' },
  valWfWindow: { key: 'wfWindow' },
  valWfStep: { key: 'wfStep' },
  valPerturbIters: { key: 'perturbIters' },
  valPerturbMin: { key: 'perturbMin' },
  valPerturbMax: { key: 'perturbMax' },
  valMcSims: { key: 'mcSims' },
}

const VALIDATION_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valPerturbEnable: { key: 'perturbEnable' },
  valMcEnable: { key: 'mcEnable' },
}

let LAST_META_RUN: Record<string, unknown> | null = null
let LAST_META_STAGE4_RUN: Record<string, unknown> | null = null
let LAST_META_ML_STATUS: Record<string, unknown> | null = null
let SAVED_META: Record<string, unknown>[] = []
let SAVED_STAGE4: Record<string, unknown>[] = []
let BATCH_RUNNING = false
let BATCH_STOP_REQUESTED = false
let BATCH_DATASETS_CACHE: BatchDatasetEntry[] = []
let BATCH_SELECTED_PATHS = new Set<string>()
let BATCH_SELECTED_PATHS_READY = false
let LAST_BATCH_SUMMARY: BatchSummaryEnvelope | null = null

function normalizePath(value: unknown): string {
  return String(value || '').trim()
}

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeRequestPayload(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') return null
  return value as Record<string, unknown>
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asRecordArray(value: unknown): Record<string, unknown>[] {
  if (!Array.isArray(value)) return []
  const rows: Record<string, unknown>[] = []
  for (const entry of value) {
    const row = asRecord(entry)
    if (!row) continue
    rows.push(row)
  }
  return rows
}

function fmtFixed(value: unknown, digits: number): string {
  const num = Number(value)
  return Number.isFinite(num) ? num.toFixed(digits) : '-'
}

function fmtPct(value: unknown, digits: number): string {
  const num = Number(value)
  return Number.isFinite(num) ? `${(num * 100).toFixed(digits)}%` : '-'
}

function fileBasename(path: string): string {
  const normalized = normalizePath(path).replace(/\\/g, '/')
  if (!normalized) return ''
  const index = normalized.lastIndexOf('/')
  return index >= 0 ? normalized.slice(index + 1) : normalized
}

function pretty(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2)
  } catch {
    return String(value ?? '')
  }
}

function parseTagsInput(value: string): string[] {
  const out: string[] = []
  const seen = new Set<string>()
  for (const part of normalizeText(value).split(',')) {
    const token = normalizeText(part)
    if (!token) continue
    const key = token.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    out.push(token)
  }
  return out
}

function formatParamsCompact(params: Record<string, unknown>): string {
  const parts: string[] = []
  for (const [key, value] of Object.entries(params)) {
    if (value == null) continue
    if (typeof value === 'number' && Number.isFinite(value)) {
      parts.push(`${key}=${Number.isInteger(value) ? value : value.toFixed(4)}`)
      continue
    }
    if (typeof value === 'boolean') {
      parts.push(`${key}=${value ? '1' : '0'}`)
      continue
    }
    const text = normalizeText(value)
    if (!text) continue
    parts.push(`${key}=${text}`)
  }
  return parts.join(' ')
}

function readStoreRecord(storeKey: 'analysis' | 'validation'): Record<string, unknown> | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  const root = pinia.state.value as Record<string, unknown>
  const store = root[storeKey]
  if (!store || typeof store !== 'object') return null
  return store as Record<string, unknown>
}

function readPiniaStoreRecord(storeKey: string): Record<string, unknown> | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  const root = pinia.state.value as Record<string, unknown>
  const store = root[storeKey]
  if (!store || typeof store !== 'object') return null
  return store as Record<string, unknown>
}

function getAnalysisStore(): AnalysisStore | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  return useAnalysisStore(pinia)
}

function getDataStore() {
  const pinia = getActivePinia()
  if (!pinia) return null
  return useDataStore(pinia)
}

function getOptimizeStore() {
  const pinia = getActivePinia()
  if (!pinia) return null
  return useOptimizeStore(pinia)
}

function getValidationStore() {
  const pinia = getActivePinia()
  if (!pinia) return null
  return useValidationStore(pinia)
}

function getWorkflowArtifactsState() {
  const pinia = getActivePinia()
  if (!pinia) return null
  return useWorkflowArtifactsStore(pinia)
}

function patchAnalysisStore(patch: Record<string, unknown>): void {
  const store = getAnalysisStore()
  if (!store) return
  const target = store as unknown as Record<string, unknown>
  for (const [key, value] of Object.entries(patch)) {
    target[key] = value
  }
  store.source = 'vue'
  store.updatedAtUnixMs = Date.now()
}

function setStatusLine(message: string): void {
  patchAnalysisStore({
    statusLine: normalizeText(message) || 'Ready.',
  })
}

function setErrorStatus(error: unknown): void {
  const message = normalizeText(error) || 'Operation failed.'
  patchAnalysisStore({ statusLine: message })
}

function selectMetaResultsTab(): void {
  const pinia = getActivePinia()
  if (!pinia) return
  const workflowStore = useWorkflowStore(pinia)
  workflowStore.selectResultsTab('meta')
}

function readVueTextFieldValue(id: string): string | null {
  const key = normalizeText(id)
  if (!key) return null

  const analysisBinding = ANALYSIS_TEXT_FIELD_BINDINGS[key]
  if (analysisBinding) {
    const store = readStoreRecord('analysis')
    if (store && Object.prototype.hasOwnProperty.call(store, analysisBinding.key)) {
      return normalizeText(store[analysisBinding.key])
    }
  }

  const validationBinding = VALIDATION_TEXT_FIELD_BINDINGS[key]
  if (validationBinding) {
    const store = readStoreRecord('validation')
    if (store && Object.prototype.hasOwnProperty.call(store, validationBinding.key)) {
      return normalizeText(store[validationBinding.key])
    }
  }

  return null
}

function readVueCheckboxFieldValue(id: string): boolean | null {
  const key = normalizeText(id)
  if (!key) return null

  const analysisBinding = ANALYSIS_CHECKBOX_FIELD_BINDINGS[key]
  if (analysisBinding) {
    const store = readStoreRecord('analysis')
    if (store && Object.prototype.hasOwnProperty.call(store, analysisBinding.key)) {
      return normalizeBoolean(store[analysisBinding.key])
    }
  }

  const validationBinding = VALIDATION_CHECKBOX_FIELD_BINDINGS[key]
  if (validationBinding) {
    const store = readStoreRecord('validation')
    if (store && Object.prototype.hasOwnProperty.call(store, validationBinding.key)) {
      return normalizeBoolean(store[validationBinding.key])
    }
  }

  return null
}

function readVueBatchSelectedPaths(): string[] | null {
  const store = readStoreRecord('analysis')
  if (!store) return null
  const rows = store.batchDatasets
  if (!Array.isArray(rows)) return null

  const selected: string[] = []
  for (const entry of rows) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as Record<string, unknown>
    if (!normalizeBoolean(row.checked)) continue
    const path = normalizeText(row.path)
    if (!path) continue
    selected.push(path)
  }
  return selected
}

function readStoreTextInputValue(id: string): string {
  const key = normalizeText(id)
  if (!key) return ''
  const vueValue = readVueTextFieldValue(key)
  return vueValue != null ? vueValue : ''
}

function readStoreCheckboxValue(id: string): boolean {
  const key = normalizeText(id)
  if (!key) return false
  const vueValue = readVueCheckboxFieldValue(key)
  return vueValue != null ? vueValue : false
}

function tryBuildAutoMetaRequest(): Record<string, unknown> | null {
  return buildMetaSelectionRequest({
    strategyKind: readStoreTextInputValue('metaStrategyKind'),
    timeframe: readStoreTextInputValue('metaTimeframe'),
    requiredTag: readStoreTextInputValue('metaRequiredTag'),
    minSamples: readStoreTextInputValue('metaMinSamples'),
    topK: readStoreTextInputValue('metaTopK'),
    holdoutTag: readStoreTextInputValue('metaHoldoutTag'),
    evalHoldout: readStoreCheckboxValue('metaEvalHoldout'),
    holdoutMode: readStoreTextInputValue('metaHoldoutMode'),
    holdoutEvalTopN: readStoreTextInputValue('metaHoldoutEvalTopN'),
    holdoutExploreN: readStoreTextInputValue('metaHoldoutExploreN'),
    holdoutRankBy: readStoreTextInputValue('metaHoldoutEvalRankBy'),
    holdoutReps: readStoreTextInputValue('metaHoldoutReps'),
    groupingMode: readStoreTextInputValue('metaGroupingMode'),
    clusterIntBucket: readStoreTextInputValue('metaClusterIntBucket'),
    clusterFloatBucket: readStoreTextInputValue('metaClusterFloatBucket'),
    saveHoldoutRows: readStoreCheckboxValue('metaSaveHoldoutRows'),
    valSeed: readStoreTextInputValue('valSeed'),
    valWfWindow: readStoreTextInputValue('valWfWindow'),
    valWfStep: readStoreTextInputValue('valWfStep'),
    valPerturbEnable: readStoreCheckboxValue('valPerturbEnable'),
    valPerturbIters: readStoreTextInputValue('valPerturbIters'),
    valPerturbMin: readStoreTextInputValue('valPerturbMin'),
    valPerturbMax: readStoreTextInputValue('valPerturbMax'),
    valMcEnable: readStoreCheckboxValue('valMcEnable'),
    valMcSims: readStoreTextInputValue('valMcSims'),
  })
}

function readStoreBatchSelectedPaths(): string[] {
  return readBatchSelectedPaths()
}

function tryBuildAutoMetaBatchRequest(): Record<string, unknown> | null {
  return buildMetaBatchRequest({
    selectedPaths: readStoreBatchSelectedPaths(),
    holdoutPct: readStoreTextInputValue('batchHoldoutPct'),
    holdoutTag: readStoreTextInputValue('batchHoldoutTag'),
    autoMeta: readStoreCheckboxValue('batchAutoMeta'),
    skipDone: readStoreCheckboxValue('batchSkipDone'),
    tags: readStoreTextInputValue('batchTags'),
  })
}

function tryBuildAutoMetaStage4Request(): Record<string, unknown> | null {
  return buildMetaStage4Request({
    strategyKind: readStoreTextInputValue('metaStrategyKind'),
    timeframe: readStoreTextInputValue('metaTimeframe'),
    topK: readStoreTextInputValue('metaStage4TopK'),
    minMlConfidence: readStoreTextInputValue('metaStage4MinConf'),
    metaWeight: readStoreTextInputValue('metaStage4MetaWeight'),
    mlWeight: readStoreTextInputValue('metaStage4MlWeight'),
    uncertaintyWeight: readStoreTextInputValue('metaStage4UncertaintyWeight'),
    acceptConfidence: readStoreTextInputValue('metaStage4AcceptConf'),
    reviewConfidence: readStoreTextInputValue('metaStage4ReviewConf'),
    maxAcceptUncertainty: readStoreTextInputValue('metaStage4MaxAcceptUncertainty'),
    autoTrainModel: readStoreCheckboxValue('metaStage4AutoTrain'),
    requireHealthyModel: readStoreCheckboxValue('metaStage4RequireHealthy'),
    fallbackToMetaIfEmpty: readStoreCheckboxValue('metaStage4FallbackToMeta'),
  })
}

function tryBuildAutoMetaMlScope(): Record<string, unknown> | null {
  return buildMetaMlScope({
    strategyKind: readStoreTextInputValue('metaStrategyKind'),
    timeframe: readStoreTextInputValue('metaTimeframe'),
  })
}

function currentMetaScope(): { strategy_kind: string; timeframe: string } {
  const store = getAnalysisStore()
  const result = asRecord(LAST_META_RUN?.result)
  const strategy = normalizeText(result?.strategy_kind ?? store?.metaStrategyKind ?? 'CompositeMo') || 'CompositeMo'
  const timeframe = normalizeText(result?.timeframe ?? store?.metaTimeframe)
  return {
    strategy_kind: strategy,
    timeframe,
  }
}

function normalizeMlScope(scopeValue: unknown): { strategy_kind: string; timeframe: string } {
  const payload = normalizeRequestPayload(scopeValue)
  if (payload) {
    const strategyKind = normalizeText(payload.strategy_kind ?? payload.strategyKind) || currentMetaScope().strategy_kind
    const timeframe = normalizeText(payload.timeframe) || currentMetaScope().timeframe
    return { strategy_kind: strategyKind, timeframe }
  }

  let autoScope: Record<string, unknown> | null = null
  try {
    autoScope = tryBuildAutoMetaMlScope()
  } catch {
    autoScope = null
  }

  if (autoScope) {
    const strategyKind = normalizeText(autoScope.strategy_kind ?? autoScope.strategyKind) || currentMetaScope().strategy_kind
    const timeframe = normalizeText(autoScope.timeframe) || currentMetaScope().timeframe
    return { strategy_kind: strategyKind, timeframe }
  }

  return currentMetaScope()
}

function buildMetaRows(groups: Record<string, unknown>[]): MetaTableRow[] {
  const rows: MetaTableRow[] = []
  for (let index = 0; index < groups.length; index += 1) {
    const group = groups[index] ?? {}
    const params = asRecord(group.params) ?? {}
    const holdout = asRecord(group.holdout)

    const fast = `${normalizeText(params.fast_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.fast_len || 0)))}`.trim()
    const slow = `${normalizeText(params.slow_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.slow_len || 0)))}`.trim()

    const finalScore = Number(group.final_score)
    const rankScore = Number.isFinite(finalScore)
      ? finalScore
      : Number(group.meta_score || 0)

    const cells = [
      String(index + 1),
      String(Math.max(0, Math.floor(Number(group.samples || 0)))),
      fmtPct(group.pass_rate, 1),
      fmtFixed(rankScore, 6),
      Number.isFinite(Number(group.ml_confidence)) ? fmtPct(group.ml_confidence, 1) : '-',
      fmtFixed(group.robust_score_p10, 6),
      fmtFixed(group.robust_score_median, 6),
      fmtFixed(group.max_dd_p90, 6),
      fmtFixed(group.psr_median, 4),
      fmtFixed(group.dsr_median, 4),
      holdout ? fmtPct(holdout.pass_rate, 1) : '-',
      holdout ? fmtFixed(holdout.score_p10, 6) : '-',
      holdout ? fmtFixed(holdout.score_median, 6) : '-',
      holdout ? fmtFixed(holdout.max_dd_p90, 6) : '-',
      fast,
      slow,
      formatParamsCompact(params) || pretty(params),
    ]

    rows.push({
      cells: cells.map((text, cellIndex) => ({
        text,
        className: cellIndex >= 14 ? 'left' : '',
        title: '',
      })),
    })
  }
  return rows
}

function renderMetaSelection(resp: Record<string, unknown> | null): void {
  const previousMetaPath = normalizePath(LAST_META_RUN?.meta_json_path)
  LAST_META_RUN = resp

  const result = asRecord(resp?.result)
  const groups = asRecordArray(result?.groups)
  const holdout = asRecord(result?.holdout)
  const holdoutDatasets = Math.max(0, Math.floor(Number(holdout?.datasets || 0)))
  const holdoutSkipped = Math.max(0, Math.floor(Number(holdout?.skipped || 0)))
  const warnings = Array.isArray(result?.warnings) ? result.warnings : []
  const reportPath = normalizePath(resp?.report_xlsx_path)
  const metaPath = normalizePath(resp?.meta_json_path)

  const warnText = warnings.length ? ` · warn=${warnings.length}` : ''
  const holdoutText = holdoutDatasets
    ? ` · holdout=${holdoutDatasets}${holdoutSkipped ? ` (skipped ${holdoutSkipped})` : ''}`
    : ''
  const rankMode = groups.some((group) => Number.isFinite(Number(group.final_score)))
    ? ' · rank=holdout-adjusted'
    : ''

  const metaMeta = result
    ? `groups=${groups.length} · artifacts=${Math.max(0, Math.floor(Number(result.artifacts_used || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))} · candidates=${Math.max(0, Math.floor(Number(result.candidates_considered || 0)))} · runtime=${Math.max(0, Math.floor(Number(result.runtime_ms || 0)))}ms${holdoutText}${rankMode}${warnText}`
    : '—'

  const latestGroups = String(groups.length)
  const latestArtifacts = result
    ? `${Math.max(0, Math.floor(Number(result.artifacts_used || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))}`
    : '—'
  const latestHoldout = holdoutDatasets
    ? `${holdoutDatasets}${holdoutSkipped ? ` (skip ${holdoutSkipped})` : ''}`
    : (result ? '0' : '—')
  const runtimeMs = Math.max(0, Math.floor(Number(result?.runtime_ms || 0)))
  const latestRuntime = result
    ? (runtimeMs >= 1000 ? `${(runtimeMs / 1000).toFixed(2)}s` : `${runtimeMs}ms`)
    : '—'

  patchAnalysisStore({
    metaMeta,
    metaOutputPreview: result ? pretty(result) : '-',
    metaTableColumns: META_TABLE_COLUMNS,
    metaTableRows: markRaw(buildMetaRows(groups)),
    canOpenMetaReport: !!reportPath,
    canRevealMetaFiles: !!(reportPath || metaPath),
    aggregateSummaryLatestGroups: latestGroups,
    aggregateSummaryLatestArtifacts: latestArtifacts,
    aggregateSummaryLatestHoldout: latestHoldout,
    aggregateSummaryLatestRuntime: latestRuntime,
    statusLine: reportPath ? `Meta complete. Saved report: ${reportPath}` : (result ? 'Meta complete.' : 'Ready.'),
  })

  if (!resp || previousMetaPath !== metaPath) {
    LAST_META_STAGE4_RUN = null
    patchAnalysisStore({
      metaStage4Status: '—',
      metaStage4OutputPreview: '-',
      canRevealMetaStage4: false,
    })
  }

  updateMetaMlButtons()
  updateMetaStage4Buttons()
}

function renderMetaStage4(resp: Record<string, unknown> | null): void {
  LAST_META_STAGE4_RUN = resp
  const result = asRecord(resp?.result)
  const groups = asRecordArray(result?.selected_groups)
  const decisionCounts = asRecord(result?.decision_counts) ?? {}
  const path = normalizePath(resp?.stage4_json_path)

  if (!result || groups.length === 0) {
    patchAnalysisStore({
      metaStage4Status: '—',
      metaStage4OutputPreview: '-',
    })
    updateMetaStage4Buttons()
    return
  }

  const lines: string[] = []
  lines.push(
    `scope=${normalizeText(result.strategy_kind)}/${normalizeText(result.timeframe)} selected=${Math.max(0, Math.floor(Number(result.groups_selected || 0)))} scored=${Math.max(0, Math.floor(Number(result.groups_scored || 0)))}/${Math.max(0, Math.floor(Number(result.groups_considered || 0)))} min_ml=${fmtFixed(result.min_ml_confidence, 3)}`,
  )
  lines.push(
    `weights(meta/ml/unc)=${fmtFixed(result.meta_weight, 3)}/${fmtFixed(result.ml_weight, 3)}/${fmtFixed(result.uncertainty_weight, 3)} buckets(accept/review/reject)=${Math.max(0, Math.floor(Number(decisionCounts.accept || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.review || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.reject || 0)))}`,
  )
  lines.push(
    `policy accept>=${fmtFixed(result.accept_confidence, 3)} review>=${fmtFixed(result.review_confidence, 3)} max_accept_unc=${fmtFixed(result.max_accept_uncertainty, 3)}`,
  )

  for (let index = 0; index < groups.length; index += 1) {
    const row = groups[index] ?? {}
    const params = asRecord(row.params) ?? {}
    const rank = Math.max(1, Math.floor(Number(row.stage4_rank || index + 1)))
    const bucket = normalizeText(row.decision_bucket).toLowerCase() || 'reject'
    const confText = Number.isFinite(Number(row.ml_confidence)) ? `${fmtPct(row.ml_confidence, 1)}` : '-'
    const uncText = Number.isFinite(Number(row.ml_uncertainty)) ? fmtFixed(row.ml_uncertainty, 3) : '-'
    const fast = `${normalizeText(params.fast_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.fast_len || 0)))}`.trim()
    const slow = `${normalizeText(params.slow_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.slow_len || 0)))}`.trim()
    const finalRank = Number(row.final_rank_score)
    const rankScore = Number.isFinite(finalRank)
      ? finalRank
      : (Number.isFinite(Number(row.final_score)) ? Number(row.final_score) : Number(row.meta_score || 0))
    lines.push(`#${rank} ${bucket} ml=${confText} unc=${uncText} final=${fmtFixed(rankScore, 6)} ${fast} vs ${slow}`.trim())
  }

  const warnings = Array.isArray(result.warnings) ? result.warnings : []
  if (warnings.length > 0) {
    lines.push(`warnings=${warnings.length}: ${warnings.map((item) => normalizeText(item)).filter((item) => item.length > 0).join(' | ')}`)
  }

  patchAnalysisStore({
    metaStage4Status: `Stage 4 complete: selected=${Math.max(0, Math.floor(Number(result.groups_selected || 0)))} scored=${Math.max(0, Math.floor(Number(result.groups_scored || 0)))}/${Math.max(0, Math.floor(Number(result.groups_considered || 0)))} accept/review/reject=${Math.max(0, Math.floor(Number(decisionCounts.accept || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.review || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.reject || 0)))} · file=${path ? fileBasename(path) : '—'}`,
    metaStage4OutputPreview: lines.join('\n'),
  })

  updateMetaStage4Buttons()
}

function formatMlStatus(status: Record<string, unknown>, scope: { strategy_kind: string; timeframe: string }): string {
  const exists = normalizeBoolean(status.exists)
  const errorText = normalizeText(status.error)
  const availableMeta = Math.max(0, Math.floor(Number(status.available_meta_artifacts || 0)))
  const availableRows = Math.max(0, Math.floor(Number(status.available_labeled_rows || 0)))
  const newest = normalizeText(status.newest_meta_ts_available)
  const stale = normalizeBoolean(status.stale)
  const retrain = normalizeBoolean(status.retrain_recommended)
  const retrainReason = normalizeText(status.retrain_reason)

  if (!exists) {
    return `No ML model saved for ${scope.strategy_kind} / ${scope.timeframe}. · available: meta=${availableMeta}, rows=${availableRows}${newest ? ` · newest=${newest}` : ''}`
  }

  if (errorText) {
    return `ML model exists but failed to load: ${errorText} · available: meta=${availableMeta}, rows=${availableRows}${newest ? ` · newest=${newest}` : ''}`
  }

  const model = asRecord(status.model)
  const label = asRecord(model?.label)
  const modelMeta = asRecord(model?.meta)
  const modelPath = normalizePath(status.model_path)

  const healthy = modelMeta?.healthy === false ? 'UNHEALTHY' : (modelMeta?.healthy === true ? 'healthy' : 'unknown')
  const target = normalizeText(modelMeta?.target)
  const trainedRows = Number(modelMeta?.trained_rows)
  const auc = Number(modelMeta?.cv_auc)
  const brierRaw = Number(modelMeta?.cv_brier_calibrated)
  const brier = Number.isFinite(brierRaw) ? brierRaw : Number(modelMeta?.cv_brier)
  const baselineBrier = Number(modelMeta?.baseline_brier)

  const parts = [
    `Model: ${fileBasename(modelPath) || modelPath || 'model.json'}`,
    `bytes=${Math.max(0, Math.floor(Number(status.bytes || 0)))}`,
    healthy,
  ]

  if (target) parts.push(`target=${target}`)
  if (Number.isFinite(trainedRows)) parts.push(`trained_rows=${Math.max(0, Math.floor(trainedRows))}`)
  if (Number.isFinite(auc)) parts.push(`cv_auc=${auc.toFixed(3)}`)
  if (Number.isFinite(brier)) parts.push(`cv_brier=${brier.toFixed(4)}`)
  if (Number.isFinite(baselineBrier)) parts.push(`baseline=${baselineBrier.toFixed(4)}`)

  const passRate = Number(label?.min_holdout_pass_rate)
  const scoreP10 = Number(label?.min_holdout_score_p10)
  const minDatasets = Number(label?.min_holdout_datasets)
  const ddP90 = Number(label?.max_holdout_max_dd_p90)
  if (Number.isFinite(passRate) || Number.isFinite(scoreP10) || Number.isFinite(minDatasets) || Number.isFinite(ddP90)) {
    const ddSuffix = Number.isFinite(ddP90) ? ` dd_p90<=${ddP90.toFixed(3)}` : ''
    parts.push(`label: pass>=${Number.isFinite(passRate) ? passRate.toFixed(2) : '0.00'}, p10>=${Number.isFinite(scoreP10) ? scoreP10.toFixed(3) : '0.000'}, datasets>=${Number.isFinite(minDatasets) ? Math.max(0, Math.floor(minDatasets)) : 0}${ddSuffix}`)
  }

  parts.push(`available: meta=${availableMeta}, rows=${availableRows}`)
  if (newest) parts.push(`newest=${newest}`)
  parts.push(stale ? 'stale' : 'fresh')
  if (retrain) parts.push('retrain recommended')
  if (retrainReason) parts.push(retrainReason)

  return parts.join(' · ')
}

function updateMetaMlButtons(): void {
  const scope = currentMetaScope()
  const hasTf = !!normalizeText(scope.timeframe)
  const hasMeta = !!asRecord(LAST_META_RUN?.result)
  const hasModel = !!normalizeBoolean(LAST_META_ML_STATUS?.exists) && !normalizeText(LAST_META_ML_STATUS?.error)

  patchAnalysisStore({
    canMetaMlRefresh: hasTf,
    canMetaMlTrain: hasTf,
    canMetaMlApply: hasTf && hasMeta && hasModel,
    canMetaMlDelete: hasTf && hasModel,
  })
}

function updateMetaStage4Buttons(): void {
  const hasTf = !!normalizeText(currentMetaScope().timeframe)
  const result = asRecord(LAST_META_RUN?.result)
  const groups = asRecordArray(result?.groups)
  const stage4Path = normalizePath(LAST_META_STAGE4_RUN?.stage4_json_path)
  patchAnalysisStore({
    canRunMetaStage4: hasTf && groups.length > 0,
    canRevealMetaStage4: !!stage4Path,
  })
}

function filterMetaSaved(items: Record<string, unknown>[], query: string): Record<string, unknown>[] {
  const q = normalizeText(query).toLowerCase()
  if (!q) return items
  const terms = q.split(/\s+/).filter((token) => token.length > 0)
  if (terms.length === 0) return items

  return items.filter((item) => {
    const tags = Array.isArray(item.tags)
      ? item.tags.map((tag) => normalizeText(tag).toLowerCase()).join(' ')
      : ''
    const haystack = [
      normalizeText(item.ts).toLowerCase(),
      normalizeText(item.strategy_kind).toLowerCase(),
      normalizeText(item.timeframe).toLowerCase(),
      normalizeText(item.required_tag).toLowerCase(),
      tags,
    ].join(' ')
    return terms.every((term) => haystack.includes(term))
  })
}

function filterStage4Saved(items: Record<string, unknown>[], query: string): Record<string, unknown>[] {
  const q = normalizeText(query).toLowerCase()
  if (!q) return items
  const terms = q.split(/\s+/).filter((token) => token.length > 0)
  if (terms.length === 0) return items

  return items.filter((item) => {
    const tags = Array.isArray(item.tags)
      ? item.tags.map((tag) => normalizeText(tag).toLowerCase()).join(' ')
      : ''
    const haystack = [
      normalizeText(item.ts).toLowerCase(),
      normalizeText(item.strategy_kind).toLowerCase(),
      normalizeText(item.timeframe).toLowerCase(),
      normalizeText(item.source_meta_json_path).toLowerCase(),
      tags,
    ].join(' ')
    return terms.every((term) => haystack.includes(term))
  })
}

function updateMetaSavedState(preferredPath = ''): void {
  const store = getAnalysisStore()
  if (!store) return

  const filtered = filterMetaSaved(SAVED_META, store.metaSavedSearch)
  let selectedPath = normalizePath(preferredPath || store.metaSavedSelected)
  if (selectedPath && !filtered.some((entry) => normalizePath(entry.path) === selectedPath)) {
    selectedPath = ''
  }

  const selectedEntry = SAVED_META.find((entry) => normalizePath(entry.path) === selectedPath) ?? null
  const selectedTags = selectedEntry && Array.isArray(selectedEntry.tags)
    ? selectedEntry.tags.map((tag) => normalizeText(tag)).filter((tag) => tag.length > 0).join(', ')
    : ''

  const metaSavedMeta = selectedEntry
    ? [
      `ts=${normalizeText(selectedEntry.ts)}`,
      `strategy=${normalizeText(selectedEntry.strategy_kind) || '—'}`,
      normalizeText(selectedEntry.timeframe) ? `tf=${normalizeText(selectedEntry.timeframe)}` : '',
      normalizeText(selectedEntry.required_tag) ? `tag=${normalizeText(selectedEntry.required_tag)}` : '',
      `groups=${Math.max(0, Math.floor(Number(selectedEntry.groups || 0)))}`,
      `artifacts=${Math.max(0, Math.floor(Number(selectedEntry.artifacts_used || 0)))}`,
      Number(selectedEntry.holdout_datasets || 0) > 0 ? `holdout=${Math.max(0, Math.floor(Number(selectedEntry.holdout_datasets || 0)))}` : '',
      Array.isArray(selectedEntry.tags) && selectedEntry.tags.length > 0
        ? `tags=${selectedEntry.tags.map((tag) => normalizeText(tag)).filter((tag) => tag.length > 0).join(',')}`
        : '',
      selectedEntry.bytes != null ? `bytes=${Math.max(0, Math.floor(Number(selectedEntry.bytes || 0)))}` : '',
    ].filter((part) => part.length > 0).join(' • ')
    : `saved meta: ${filtered.length}/${SAVED_META.length}`

  const reportPath = normalizePath(selectedEntry?.report_xlsx_path)

  const options = [
    { value: '', label: filtered.length > 0 ? 'Select a saved meta run...' : 'No meta runs yet', disabled: false },
    ...filtered.map((entry) => {
      const ts = normalizeText(entry.ts) || 'unknown'
      const kind = normalizeText(entry.strategy_kind)
      const tf = normalizeText(entry.timeframe)
      const groups = Math.max(0, Math.floor(Number(entry.groups || 0)))
      const artifacts = Math.max(0, Math.floor(Number(entry.artifacts_used || 0)))
      const holdout = Math.max(0, Math.floor(Number(entry.holdout_datasets || 0)))
      const tagsCount = Array.isArray(entry.tags) ? entry.tags.length : 0
      const tfHint = tf ? ` | ${tf}` : ''
      const holdoutHint = holdout ? ` | holdout=${holdout}` : ''
      const tagsHint = tagsCount ? ` | tags=${tagsCount}` : ''
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | groups=${groups} | arts=${artifacts}${holdoutHint}${tagsHint}`,
        disabled: false,
      }
    }),
  ]

  let compareSelected = normalizePath(store.metaCompareSelected)
  if (compareSelected && !SAVED_META.some((entry) => normalizePath(entry.path) === compareSelected)) {
    compareSelected = ''
  }
  const compareOptions = [
    { value: '', label: SAVED_META.length > 0 ? 'Compare against...' : 'No meta runs yet', disabled: false },
    ...SAVED_META.map((entry) => {
      const ts = normalizeText(entry.ts) || 'unknown'
      const kind = normalizeText(entry.strategy_kind)
      const tf = normalizeText(entry.timeframe)
      const groups = Math.max(0, Math.floor(Number(entry.groups || 0)))
      const tfHint = tf ? ` | ${tf}` : ''
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | groups=${groups}`,
        disabled: false,
      }
    }),
  ]

  patchAnalysisStore({
    metaSavedOptions: markRaw(options),
    metaSavedSelected: selectedPath,
    metaSavedTagsDraft: selectedTags,
    metaSavedMeta,
    canLoadMetaSaved: !!selectedEntry,
    canOpenMetaSavedReport: !!selectedEntry && !!reportPath,
    canRevealMetaSaved: !!selectedEntry,
    canDeleteMetaSaved: !!selectedEntry,
    canSaveMetaSavedTags: !!selectedEntry,
    canRefreshMetaSaved: true,
    metaCompareOptions: markRaw(compareOptions),
    metaCompareSelected: compareSelected,
    aggregateSummarySavedMeta: String(SAVED_META.length),
  })
}

function updateStage4SavedState(preferredPath = ''): void {
  const store = getAnalysisStore()
  if (!store) return

  const filtered = filterStage4Saved(SAVED_STAGE4, store.stage4SavedSearch)
  let selectedPath = normalizePath(preferredPath || store.stage4SavedSelected)
  if (selectedPath && !filtered.some((entry) => normalizePath(entry.path) === selectedPath)) {
    selectedPath = ''
  }

  const selectedEntry = SAVED_STAGE4.find((entry) => normalizePath(entry.path) === selectedPath) ?? null
  const selectedTags = selectedEntry && Array.isArray(selectedEntry.tags)
    ? selectedEntry.tags.map((tag) => normalizeText(tag)).filter((tag) => tag.length > 0).join(', ')
    : ''

  const stage4SavedMeta = selectedEntry
    ? [
      `ts=${normalizeText(selectedEntry.ts)}`,
      `strategy=${normalizeText(selectedEntry.strategy_kind) || '—'}`,
      normalizeText(selectedEntry.timeframe) ? `tf=${normalizeText(selectedEntry.timeframe)}` : '',
      `selected=${Math.max(0, Math.floor(Number(selectedEntry.groups_selected || 0)))}/${Math.max(0, Math.floor(Number(selectedEntry.groups_scored || 0)))}/${Math.max(0, Math.floor(Number(selectedEntry.groups_considered || 0)))}`,
      Number.isFinite(Number(selectedEntry.min_ml_confidence)) ? `min_ml=${fmtFixed(selectedEntry.min_ml_confidence, 3)}` : '',
      Number.isFinite(Number(selectedEntry.top_k)) ? `top_k=${Math.max(0, Math.floor(Number(selectedEntry.top_k || 0)))}` : '',
      `model=${normalizeBoolean(selectedEntry.model_healthy) ? 'healthy' : 'unhealthy'}`,
      normalizeText(selectedEntry.source_meta_json_path) ? `source=${fileBasename(normalizeText(selectedEntry.source_meta_json_path))}` : '',
      Array.isArray(selectedEntry.tags) && selectedEntry.tags.length > 0
        ? `tags=${selectedEntry.tags.map((tag) => normalizeText(tag)).filter((tag) => tag.length > 0).join(',')}`
        : '',
      selectedEntry.bytes != null ? `bytes=${Math.max(0, Math.floor(Number(selectedEntry.bytes || 0)))}` : '',
    ].filter((part) => part.length > 0).join(' • ')
    : `saved Stage 4: ${filtered.length}/${SAVED_STAGE4.length}`

  const options = [
    { value: '', label: filtered.length > 0 ? 'Select a saved Stage 4 run...' : 'No Stage 4 runs yet', disabled: false },
    ...filtered.map((entry) => {
      const ts = normalizeText(entry.ts) || 'unknown'
      const kind = normalizeText(entry.strategy_kind)
      const tf = normalizeText(entry.timeframe)
      const selected = Math.max(0, Math.floor(Number(entry.groups_selected || 0)))
      const scored = Math.max(0, Math.floor(Number(entry.groups_scored || 0)))
      const considered = Math.max(0, Math.floor(Number(entry.groups_considered || 0)))
      const minConf = Number(entry.min_ml_confidence)
      const tagsCount = Array.isArray(entry.tags) ? entry.tags.length : 0
      const tfHint = tf ? ` | ${tf}` : ''
      const confHint = Number.isFinite(minConf) ? ` | min_ml=${minConf.toFixed(2)}` : ''
      const tagsHint = tagsCount ? ` | tags=${tagsCount}` : ''
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | selected=${selected}/${scored}/${considered}${confHint}${tagsHint}`,
        disabled: false,
      }
    }),
  ]

  patchAnalysisStore({
    stage4SavedOptions: markRaw(options),
    stage4SavedSelected: selectedPath,
    stage4SavedTagsDraft: selectedTags,
    stage4SavedMeta,
    canRefreshStage4Saved: true,
    canLoadStage4Saved: !!selectedEntry,
    canRevealStage4Saved: !!selectedEntry,
    canDeleteStage4Saved: !!selectedEntry,
    canSaveStage4SavedTags: !!selectedEntry,
  })
}

async function refreshSavedInternal(preferredPath = ''): Promise<void> {
  const rows = (await invokeCommand('list_meta_artifacts')) as unknown
  SAVED_META = asRecordArray(rows)
  updateMetaSavedState(preferredPath)
}

async function refreshStage4SavedInternal(preferredPath = ''): Promise<void> {
  const rows = (await invokeCommand('list_stage4_artifacts')) as unknown
  SAVED_STAGE4 = asRecordArray(rows)
  updateStage4SavedState(preferredPath)
}

function buildMetaSelectionResponseFromArtifact(artifact: Record<string, unknown>, path: string): Record<string, unknown> {
  return {
    ts: normalizeText(artifact.ts) || new Date().toISOString(),
    report_xlsx_path: normalizePath(artifact.report_xlsx_path),
    report_xlsx_bytes: 0,
    meta_json_path: normalizePath(path),
    meta_json_bytes: 0,
    result: asRecord(artifact.res),
  }
}

function buildStage4ResponseFromArtifact(artifact: Record<string, unknown>, path: string): Record<string, unknown> {
  return {
    ts: normalizeText(artifact.ts) || new Date().toISOString(),
    stage4_json_path: normalizePath(path),
    stage4_json_bytes: 0,
    result: asRecord(artifact.res),
  }
}

function selectedMetaSavedEntry(): Record<string, unknown> | null {
  const store = getAnalysisStore()
  if (!store) return null
  const path = normalizePath(store.metaSavedSelected)
  if (!path) return null
  return SAVED_META.find((entry) => normalizePath(entry.path) === path) ?? null
}

function selectedStage4SavedEntry(): Record<string, unknown> | null {
  const store = getAnalysisStore()
  if (!store) return null
  const path = normalizePath(store.stage4SavedSelected)
  if (!path) return null
  return SAVED_STAGE4.find((entry) => normalizePath(entry.path) === path) ?? null
}

function formatYmdUtc(value: unknown): string {
  const ms = Number(value)
  if (!Number.isFinite(ms)) return '—'
  const date = new Date(ms)
  if (!Number.isFinite(date.getTime())) return '—'
  return date.toISOString().slice(0, 10)
}

function normalizeMaybeInteger(value: unknown): number | null {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return null
  return Math.floor(parsed)
}

function normalizeProbability(value: unknown): number | null {
  const text = normalizeText(value)
  if (!text) return null
  const parsed = Number(text)
  if (!Number.isFinite(parsed)) return null
  return Math.max(0, Math.min(1, parsed))
}

function isCancellationError(error: unknown): boolean {
  const message = normalizeText(error).toLowerCase()
  return message.includes('cancelled') || message.includes('canceled')
}

function normalizeStrategyKind(kindRaw: unknown): string {
  const kind = normalizeText(kindRaw || 'DoubleMa') || 'DoubleMa'
  if (kind === 'CompositeCpu') return 'CompositeCpu'
  if (kind === 'CompositeMo') return 'CompositeMo'
  if (kind === 'CompositeTrend') return 'CompositeTrend'
  if (kind === 'RegimeHybrid') return 'RegimeHybrid'
  if (kind === 'BreakoutMomentum') return 'BreakoutMomentum'
  return 'DoubleMa'
}

function readBatchSelectedPaths(): string[] {
  if (BATCH_SELECTED_PATHS_READY) return Array.from(BATCH_SELECTED_PATHS)
  const paths = readVueBatchSelectedPaths()
  if (!paths) return []
  BATCH_SELECTED_PATHS = new Set(paths.map((path) => normalizePath(path)).filter((path) => path.length > 0))
  BATCH_SELECTED_PATHS_READY = true
  return Array.from(BATCH_SELECTED_PATHS)
}

function setBatchSelectedPaths(paths: unknown): void {
  const next = Array.isArray(paths)
    ? paths.map((entry) => normalizePath(entry)).filter((entry) => entry.length > 0)
    : []
  BATCH_SELECTED_PATHS = new Set(next)
  BATCH_SELECTED_PATHS_READY = true
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

function batchFailedCount(): number {
  const summary = LAST_BATCH_SUMMARY?.summary
  if (!summary) return 0
  const counts = asRecord(summary.counts)
  const failed = Number(counts?.failed)
  if (Number.isFinite(failed) && failed > 0) return Math.max(0, Math.floor(failed))
  const results = asRecord(summary.results)
  return Array.isArray(results?.failed) ? results.failed.length : 0
}

function syncBatchButtons(): void {
  const hasSummaryPath = !!normalizePath(LAST_BATCH_SUMMARY?.path)
  patchAnalysisStore({
    canRunBatch: !BATCH_RUNNING,
    canBatchStop: BATCH_RUNNING,
    canBatchRefresh: !BATCH_RUNNING,
    canBatchSelectAll: !BATCH_RUNNING,
    canBatchSelectNone: !BATCH_RUNNING,
    canBatchRetryFailed: !BATCH_RUNNING && batchFailedCount() > 0,
    canBatchRevealSummary: !BATCH_RUNNING && hasSummaryPath,
  })
}

function updateBatchMeta(extra = ''): void {
  const total = Array.isArray(BATCH_DATASETS_CACHE) ? BATCH_DATASETS_CACHE.length : 0
  const selected = readBatchSelectedPaths().length
  const parts = [`selected ${selected}/${total}`]
  const note = normalizeText(extra)
  if (note) parts.push(note)
  const summaryPath = normalizePath(LAST_BATCH_SUMMARY?.path)
  if (summaryPath) parts.push(`summary=${fileBasename(summaryPath)}`)
  patchAnalysisStore({ batchMeta: parts.join(' • ') })
}

function filterBatchDatasets(items: BatchDatasetEntry[], query: string): BatchDatasetEntry[] {
  const q = normalizeText(query).toLowerCase()
  if (!q) return items
  const terms = q.split(/\s+/g).filter((term) => term.length > 0)
  if (terms.length === 0) return items
  return items.filter((item) => {
    const haystack = `${item.market} ${item.label} ${item.path} tf=${item.timeframe}`.toLowerCase()
    return terms.every((term) => haystack.includes(term))
  })
}

function renderBatchDatasetList(extra = ''): void {
  const store = getAnalysisStore()
  if (!store) return
  const filtered = filterBatchDatasets(BATCH_DATASETS_CACHE, normalizeText(store.batchDatasetSearch))
    .sort((left, right) => `${left.market}${left.label}`.localeCompare(`${right.market}${right.label}`))

  const maxRender = filtered.length > 1000 ? 500 : filtered.length
  const visible = filtered.slice(0, maxRender)
  const selected = new Set(readBatchSelectedPaths())
  const rows = visible.map((entry) => ({
    path: entry.path,
    label: entry.label,
    meta: entry.meta,
    checked: selected.has(entry.path),
    disabled: BATCH_RUNNING,
  }))
  patchAnalysisStore({
    batchDatasets: markRaw(rows),
    batchSelectedCount: selected.size,
    batchTotalCount: BATCH_DATASETS_CACHE.length,
  })
  updateBatchMeta(extra)
  syncBatchButtons()
}

function normalizeBatchDatasetEntry(
  value: unknown,
  fallback: Partial<BatchDatasetEntry> = {},
): BatchDatasetEntry | null {
  const row = asRecord(value)
  if (!row) return null
  const path = normalizePath(row.csv_path ?? row.path ?? fallback.path)
  if (!path) return null
  const market = normalizeText(row.market ?? fallback.market).toLowerCase()
  const adjusted = market === 'stocks'
    ? normalizeBoolean(row.adjusted ?? fallback.adjusted)
    : null
  const timeframe = normalizeText(row.timeframe ?? fallback.timeframe)
  const label = normalizeText(
    (row.display_symbol ?? row.polygon_ticker ?? row.label ?? fallback.label ?? fileBasename(path)) || path,
  )
  const source = normalizeText(row.source ?? fallback.source) || (market === 'local' ? 'repo_data' : 'polygon')
  const sourceSize = normalizeMaybeInteger(row.source_size_bytes ?? fallback.source_size_bytes)
  const sourceMtime = normalizeMaybeInteger(row.source_mtime_utc_ms ?? fallback.source_mtime_utc_ms)
  const sourceFp = normalizeText(row.source_content_fingerprint ?? fallback.source_content_fingerprint)
  const marketLabel = normalizeMarketLabel(market, adjusted === true)
  const minTs = formatYmdUtc(row.min_ts)
  const maxTs = formatYmdUtc(row.max_ts)
  const updated = formatYmdUtc(row.last_updated_utc_ms)
  const rowsHint = normalizeRowsHint(row.row_count)

  return {
    path,
    label: label || path,
    meta: `${marketLabel} • tf=${timeframe || '?'} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}`,
    market,
    timeframe,
    adjusted,
    source,
    source_size_bytes: sourceSize,
    source_mtime_utc_ms: sourceMtime,
    source_content_fingerprint: sourceFp,
  }
}

async function refreshBatchDatasetsInternal(): Promise<void> {
  const byPath = new Map<string, BatchDatasetEntry>()
  let polygonError = ''
  let localError = ''
  let localDir = ''
  let localTruncated = false

  try {
    const raw = (await invokeCommand('polygon_get_cache_info')) as unknown
    const record = asRecord(raw)
    const datasets = Array.isArray(record?.datasets) ? (record?.datasets as unknown[]) : []
    for (const entry of datasets) {
      const row = normalizeBatchDatasetEntry(entry, { source: 'polygon' })
      if (!row) continue
      byPath.set(row.path, row)
    }
  } catch (error) {
    polygonError = normalizeText(error)
  }

  try {
    const raw = (await invokeCommand('list_repo_data_csvs', {
      maxFiles: 20000,
      recursive: true,
      includeIdentity: true,
    })) as unknown
    const record = asRecord(raw)
    localDir = normalizeText(record?.dir)
    localTruncated = normalizeBoolean(record?.truncated)
    const entries = Array.isArray(record?.csv_entries)
      ? (record?.csv_entries as unknown[])
      : (Array.isArray(record?.csv_paths) ? (record?.csv_paths as unknown[]) : [])
    for (const entry of entries) {
      const row = normalizeBatchDatasetEntry(entry, { market: 'local', source: 'repo_data' })
      if (!row) continue
      if (byPath.has(row.path)) continue
      byPath.set(row.path, row)
    }
  } catch (error) {
    localError = normalizeText(error)
  }

  BATCH_DATASETS_CACHE = Array.from(byPath.values())
  const validPaths = new Set(BATCH_DATASETS_CACHE.map((entry) => entry.path))
  setBatchSelectedPaths(readBatchSelectedPaths().filter((path) => validPaths.has(path)))

  const extras: string[] = []
  if (localDir) extras.push(`local=${fileBasename(localDir) || 'data'}`)
  if (localTruncated) extras.push('local truncated')
  if (polygonError && BATCH_DATASETS_CACHE.length === 0) extras.push('polygon error')
  if (localError && BATCH_DATASETS_CACHE.length === 0) extras.push('local data error')
  renderBatchDatasetList(extras.join(' • '))
}

async function ensureBatchDatasetsLoaded(): Promise<void> {
  if (Array.isArray(BATCH_DATASETS_CACHE) && BATCH_DATASETS_CACHE.length > 0) return
  await refreshBatchDatasetsInternal()
}

function setBatchSummaryState(summary: Record<string, unknown> | null, path: string): void {
  const summaryPath = normalizePath(path)
  const payload = asRecord(summary)
  LAST_BATCH_SUMMARY = payload ? { path: summaryPath, summary: payload } : null

  patchAnalysisStore({
    batchOutputPreview: payload ? pretty({ path: summaryPath || null, summary: payload }) : '-',
  })
  if (summaryPath) writeStorageString(LAST_BATCH_SUMMARY_KEY, summaryPath)
  else removeStorageKey(LAST_BATCH_SUMMARY_KEY)
  updateBatchMeta('')
  syncBatchButtons()
}

async function loadLastBatchSummaryFromDisk(): Promise<void> {
  if (LAST_BATCH_SUMMARY) return
  const path = normalizePath(readStorageString(LAST_BATCH_SUMMARY_KEY, ''))
  if (!path) return
  try {
    const artifact = (await invokeCommand('load_run_artifact', { path })) as unknown
    const row = asRecord(artifact)
    if (!row || normalizeText(row.kind) !== 'vectorbt_batch_summary') return
    setBatchSummaryState(row, path)
  } catch {
    // Best-effort hydration only.
  }
}

function buildValidationFingerprintReq(runReq: Record<string, unknown>): Record<string, unknown> {
  const validation = readPiniaStoreRecord('validation')
  const candidates = Math.max(1, Math.floor(Number(validation?.candidates || 32)))
  const seed = Math.max(0, Math.floor(Number(validation?.seed || 1)))
  const minPsr = normalizeProbability(validation?.minPsr)
  const minDsr = normalizeProbability(validation?.minDsr)

  const wfEnable = normalizeBoolean(validation?.wfEnable)
  const wfWindow = Math.max(64, Math.floor(Number(validation?.wfWindow || 20000)))
  const wfStep = Math.max(1, Math.floor(Number(validation?.wfStep || 20000)))

  const perturbEnable = normalizeBoolean(validation?.perturbEnable)
  const perturbIters = Math.max(0, Math.floor(Number(validation?.perturbIters || 8)))
  const perturbMin = Math.max(0, Number(validation?.perturbMin || 0.015))
  const perturbMax = Math.max(0, Number(validation?.perturbMax || 0.06))

  const mcEnable = normalizeBoolean(validation?.mcEnable)
  const mcSims = Math.max(0, Math.floor(Number(validation?.mcSims || 500)))

  const cscvEnable = normalizeBoolean(validation?.cscvEnable)
  let cscvSlices = Math.max(4, Math.floor(Number(validation?.cscvSlices || 16)))
  if ((cscvSlices & 1) === 1) cscvSlices = Math.max(4, cscvSlices - 1)

  const crossEnable = normalizeBoolean(validation?.crossEnable)
  const crossMax = Math.max(0, Math.floor(Number(validation?.crossMax || 6)))
  const crossRows = Array.isArray(validation?.crossDatasets) ? (validation?.crossDatasets as unknown[]) : []
  const crossDatasets = crossRows
    .map((entry) => asRecord(entry))
    .filter((entry) => !!entry && normalizeBoolean(entry.checked))
    .map((entry) => ({
      csv_path: normalizePath(entry?.csv_path ?? entry?.path),
      label: normalizeText(entry?.label),
    }))
    .filter((entry) => entry.csv_path.length > 0)
    .slice(0, crossMax)

  return {
    data_id: normalizeText(runReq.data_id || runReq.dataId || '__cfg__') || '__cfg__',
    ma_source: normalizeText(runReq.ma_source || runReq.maSource || 'close') || 'close',
    candidates: [],
    walk_forward: wfEnable ? { window_bars: wfWindow, step_bars: wfStep } : null,
    perturb: perturbEnable
      ? {
          iterations: perturbIters,
          pct_min: perturbMin,
          pct_max: perturbMax,
          flip_bool_prob: 0.02,
        }
      : null,
    trade_bootstrap: mcEnable ? { simulations: mcSims } : null,
    cross_ticker: crossEnable && crossMax > 0 && crossDatasets.length > 0
      ? { max_datasets: crossMax, datasets: crossDatasets }
      : null,
    cscv_pbo: cscvEnable
      ? {
          slices: cscvSlices,
          max_combinations: 50000,
          threshold: 0,
        }
      : null,
    seed,
    max_candidates: candidates,
    min_psr: minPsr,
    min_dsr: minDsr,
  }
}

async function tryBuildBatchConfigFingerprint(): Promise<string> {
  try {
    const runReq = await buildOptimizationRequestFromVueStores()
    return computeConfigFingerprint(runReq, buildValidationFingerprintReq(runReq))
  } catch {
    return ''
  }
}

async function applyRunTags(tags: string[]): Promise<void> {
  const workflowArtifacts = getWorkflowArtifactsState()
  const run = workflowArtifacts?.currentRun
  if (!workflowArtifacts || !run) return

  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: run.ts,
        artifact_path: run.artifactPath,
        report_xlsx_path: run.reportXlsxPath,
        csv_path: run.csvPath,
        tags,
        req: run.req,
        res: run.res,
      },
      currentValidation: workflowArtifacts.currentValidation
        ? {
            ts: workflowArtifacts.currentValidation.ts,
            req: workflowArtifacts.currentValidation.req,
            res: workflowArtifacts.currentValidation.res,
          }
        : null,
    },
    'vue',
  )

  const artifactPath = normalizePath(run.artifactPath)
  if (artifactPath) {
    await invokeCommand('update_artifact_tags', { path: artifactPath, tags })
  }
}

async function refreshSavedRunCount(): Promise<void> {
  try {
    const rows = (await invokeCommand('list_run_artifacts')) as unknown
    patchAnalysisStore({ aggregateSummarySavedRuns: String(Array.isArray(rows) ? rows.length : 0) })
  } catch {
    // keep batch flow non-throwing if run-list query fails
  }
}

async function loadBatchDataset(path: string): Promise<{ timeframe: string }> {
  const dataStore = getDataStore()
  const loaded = await loadDataCsv(path)
  dataStore?.applyLoadResult(loaded, {
    treatAsBase: true,
    statusLine: loaded.cacheHit
      ? 'Loaded (binary cache).'
      : (loaded.cachePath ? 'Loaded (CSV parsed + cached).' : 'Loaded.'),
  })
  return {
    timeframe: normalizeText(loaded.meta?.timeframe),
  }
}

async function maybeApplyBatchTimeframe(timeframe: string): Promise<{ timeframe: string }> {
  const target = normalizeText(timeframe)
  if (!target) return { timeframe: '' }
  const dataStore = getDataStore()
  const baseId = normalizeText(dataStore?.baseDataId || dataStore?.dataId)
  if (!baseId) return { timeframe: '' }
  const loaded = await applyDataTimeframe(target, baseId)
  dataStore?.applyLoadResult(loaded, {
    treatAsBase: false,
    statusLine: loaded.cacheHit ? `Using ${target} (cached).` : `Using ${target}.`,
  })
  return {
    timeframe: normalizeText(loaded.meta?.timeframe || target),
  }
}

export function refreshBatchDatasetView(): void {
  renderBatchDatasetList('')
}

export async function runBatch(options: MetaBatchRunOptions = {}): Promise<void> {
  const validationStore = getValidationStore()
  const prevAutoRun = !!validationStore?.autoRun
  const prevAutoExportXlsx = !!validationStore?.autoExportXlsx
  const prevAutoRevealReport = !!validationStore?.autoRevealReport

  try {
    if (BATCH_RUNNING) return

    let req = normalizeRequestPayload(options.request)
    if (!req) {
      try {
        req = tryBuildAutoMetaBatchRequest()
      } catch {
        req = null
      }
    }

    const optimizeStore = getOptimizeStore()
    const strategyKind = normalizeStrategyKind(optimizeStore?.strategyKind)
    if (strategyKind !== 'CompositeCpu' && strategyKind !== 'CompositeMo') {
      setStatusLine('Batch runs currently support Composite strategies only (Composite [CPU] or Composite [CPU + MO]).')
      return
    }

    await ensureBatchDatasetsLoaded()

    const selectedPaths = Array.isArray(req?.selected_paths)
      ? req!.selected_paths as unknown[]
      : readBatchSelectedPaths()
    const normalizedSelected = selectedPaths.map((entry) => normalizePath(entry)).filter((entry) => entry.length > 0)
    setBatchSelectedPaths(normalizedSelected)
    renderBatchDatasetList('')

    if (normalizedSelected.length === 0) {
      setStatusLine('Select at least 1 dataset in Meta → Batch runs first.')
      return
    }

    const holdoutPctRaw = Math.floor(Number((req?.holdout_pct ?? readStoreTextInputValue('batchHoldoutPct')) || 0))
    const holdoutPct = Number.isFinite(holdoutPctRaw) ? Math.max(0, Math.min(50, holdoutPctRaw)) : 0
    const holdoutTag = normalizeText(req?.holdout_tag ?? readStoreTextInputValue('batchHoldoutTag')) || 'holdout'
    const autoMeta = normalizeBoolean(req?.auto_meta ?? readStoreCheckboxValue('batchAutoMeta'))
    const skipDone = normalizeBoolean(req?.skip_done ?? readStoreCheckboxValue('batchSkipDone'))
    let baseTags = dedupTags(Array.isArray(req?.tags) ? req?.tags : parseTagsInput(readStoreTextInputValue('batchTags')))
    if (baseTags.length === 0) baseTags = [defaultBatchTag()]
    const primaryTag = normalizeText(baseTags[0])

    const analysisStore = getAnalysisStore()
    if (analysisStore) {
      analysisStore.batchTags = baseTags.join(', ')
      analysisStore.batchHoldoutPct = String(holdoutPct)
      analysisStore.batchHoldoutTag = holdoutTag
      analysisStore.batchAutoMeta = autoMeta
      analysisStore.batchSkipDone = skipDone
      if (!normalizeText(analysisStore.metaRequiredTag)) analysisStore.metaRequiredTag = primaryTag
      analysisStore.updatedAtUnixMs = Date.now()
    }

    const byPath = new Map<string, BatchDatasetEntry>()
    for (const entry of BATCH_DATASETS_CACHE) {
      byPath.set(entry.path, entry)
    }

    const dataStore = getDataStore()
    const tfOverride = normalizeText(dataStore?.dataTimeframe)
    const hasTfOverride = tfOverride.length > 0
    const selectedTimeframes = new Set<string>()
    for (const selectedPath of normalizedSelected) {
      const entry = byPath.get(selectedPath)
      const tf = normalizeText(entry?.timeframe)
      if (tf) selectedTimeframes.add(tf)
    }

    if (!hasTfOverride && selectedTimeframes.size > 1) {
      const found = Array.from(selectedTimeframes).sort().join(', ')
      setStatusLine(`Batch requires a single timeframe across selected datasets (found: ${found}). Filter by timeframe or set Data → Timeframe to force resampling.`)
      return
    }

    let expectedTimeframe = hasTfOverride
      ? tfOverride
      : (selectedTimeframes.size === 1 ? Array.from(selectedTimeframes)[0] : '')

    BATCH_RUNNING = true
    BATCH_STOP_REQUESTED = false
    syncBatchButtons()
    setBatchSummaryState(null, '')

    if (validationStore) {
      validationStore.autoRun = false
      validationStore.autoExportXlsx = false
      validationStore.autoRevealReport = false
    }

    const batchTs = new Date().toISOString()
    const startedAt = typeof performance !== 'undefined' ? performance.now() : Date.now()
    const n = normalizedSelected.length
    const split = splitHoldoutPaths(normalizedSelected, holdoutPct, primaryTag || '0')
    const holdoutSet = new Set(Array.isArray(split.holdout) ? split.holdout : [])
    const batchTimeframes = new Set<string>()
    if (expectedTimeframe) batchTimeframes.add(expectedTimeframe)

    let expectedFingerprint = await tryBuildBatchConfigFingerprint()
    if (!expectedFingerprint && skipDone) {
      try {
        const firstPath = normalizePath(normalizedSelected[0])
        if (firstPath) {
          await loadBatchDataset(firstPath)
          if (hasTfOverride) await maybeApplyBatchTimeframe(tfOverride)
          expectedFingerprint = await tryBuildBatchConfigFingerprint()
        }
      } catch {
        expectedFingerprint = ''
      }
    }

    const doneLoose = new Set<string>()
    const doneStrict = new Set<string>()
    const doneKeyLoose = (csvPath: string, tf: string): string => `${csvPath}@@tf=${tf}`
    const doneKeyStrictFp = (csvPath: string, tf: string, fp: string): string => `${csvPath}@@tf=${tf}@@fp=${fp}`
    const doneKeyStrictMeta = (csvPath: string, tf: string, size: number, mtime: number): string => `${csvPath}@@tf=${tf}@@id=${size}@${mtime}`

    if (skipDone && primaryTag && expectedFingerprint) {
      try {
        const rows = (await invokeCommand('list_run_artifacts')) as unknown
        const artifacts = Array.isArray(rows) ? rows : []
        for (const item of artifacts) {
          const row = asRecord(item)
          if (!row) continue
          if (normalizeStrategyKind(row.strategy_kind) !== strategyKind) continue
          if (normalizeText(row.config_fingerprint) !== expectedFingerprint) continue
          const rowTags = Array.isArray(row.tags)
            ? row.tags.map((tag) => normalizeText(tag).toLowerCase()).filter((tag) => tag.length > 0)
            : []
          if (!rowTags.includes(primaryTag.toLowerCase())) continue
          const csvPath = normalizePath(row.csv_path)
          if (!csvPath) continue
          const rowTf = normalizeText(row.timeframe)
          if (expectedTimeframe && rowTf && rowTf !== expectedTimeframe) continue
          if (!normalizeBoolean(row.has_validation)) continue

          const tfKey = expectedTimeframe || rowTf || ''
          doneLoose.add(doneKeyLoose(csvPath, tfKey))

          const fp = normalizeText(row.source_content_fingerprint)
          if (fp) {
            doneStrict.add(doneKeyStrictFp(csvPath, tfKey, fp))
            continue
          }
          const size = normalizeMaybeInteger(row.source_size_bytes)
          const mtime = normalizeMaybeInteger(row.source_mtime_utc_ms)
          if (size != null && size >= 0 && mtime != null && mtime > 0) {
            doneStrict.add(doneKeyStrictMeta(csvPath, tfKey, size, mtime))
          }
        }
      } catch {
        // skip-done is best-effort
      }
    } else if (skipDone && !expectedFingerprint) {
      setStatusLine('Batch skip-done is enabled but config fingerprint could not be resolved; proceeding without skip cache.')
    }

    let ok = 0
    let skipped = 0
    let cancelled = false
    const failures: Array<{ label: string; error: string }> = []
    const okRows: Array<Record<string, unknown>> = []
    const skippedRows: Array<Record<string, unknown>> = []
    const failedRows: Array<Record<string, unknown>> = []

    for (let i = 0; i < n; i += 1) {
      if (BATCH_STOP_REQUESTED) break
      const path = normalizePath(normalizedSelected[i])
      if (!path) continue

      const ds = byPath.get(path)
      const isHoldout = holdoutSet.has(path)
      const label = normalizeText(ds?.label || fileBasename(path) || path)
      const holdoutHint = holdoutSet.size ? ` • holdout=${holdoutSet.size}/${n}` : ''
      updateBatchMeta(`running ${i + 1}/${n}: ${label || 'dataset'}${isHoldout ? ' [holdout]' : ''}${holdoutHint}`)

      const tfKey = expectedTimeframe || ''
      const looseKey = doneKeyLoose(path, tfKey)
      const curFp = normalizeText(ds?.source_content_fingerprint)
      const curSize = normalizeMaybeInteger(ds?.source_size_bytes)
      const curMtime = normalizeMaybeInteger(ds?.source_mtime_utc_ms)
      const strictKey = curFp
        ? doneKeyStrictFp(path, tfKey, curFp)
        : (curSize != null && curMtime != null && curSize >= 0 && curMtime > 0
          ? doneKeyStrictMeta(path, tfKey, curSize, curMtime)
          : '')
      const isDone = skipDone && expectedFingerprint
        ? (strictKey ? doneStrict.has(strictKey) : doneLoose.has(looseKey))
        : false
      if (isDone) {
        skipped += 1
        updateBatchMeta(`skipping ${label || 'dataset'} (already completed for ${primaryTag})`)
        skippedRows.push({ csv_path: path, label, is_holdout: isHoldout })
        continue
      }

      try {
        const loaded = await loadBatchDataset(path)
        let effectiveTf = normalizeText(loaded.timeframe)
        if (hasTfOverride) {
          const resampled = await maybeApplyBatchTimeframe(tfOverride)
          effectiveTf = normalizeText(resampled.timeframe || effectiveTf || tfOverride)
        }
        if (!expectedTimeframe && effectiveTf) {
          expectedTimeframe = effectiveTf
          batchTimeframes.add(effectiveTf)
        }
        if (expectedTimeframe && effectiveTf && effectiveTf !== expectedTimeframe) {
          throw new Error(`timeframe mismatch: got ${effectiveTf}, expected ${expectedTimeframe}`)
        }
        if (effectiveTf) batchTimeframes.add(effectiveTf)
      } catch (error) {
        const err = `load: ${normalizeText(error)}`
        failures.push({ label, error: err })
        failedRows.push({ csv_path: path, label, is_holdout: isHoldout, error: err })
        setStatusLine(`Batch: failed to load ${label}: ${normalizeText(error)}`)
        continue
      }

      const extraTags = isHoldout && holdoutPct > 0 ? [holdoutTag] : []
      const perRunTags = dedupTags([...baseTags, label, ...extraTags].filter((tag) => normalizeText(tag).length > 0))

      try {
        await runOptimization({ switchTab: false })
        await applyRunTags(perRunTags)
        await runValidation({ useFiltered: false, switchTab: false })

        ok += 1
        const workflowArtifacts = getWorkflowArtifactsState()
        const run = workflowArtifacts?.currentRun
        okRows.push({
          csv_path: path,
          label,
          is_holdout: isHoldout,
          ts: normalizeText(run?.ts) || null,
          artifact_path: normalizePath(run?.artifactPath) || null,
          report_xlsx_path: normalizePath(run?.reportXlsxPath) || null,
        })

        if (skipDone && expectedFingerprint) {
          doneLoose.add(looseKey)
          if (strictKey) doneStrict.add(strictKey)
        }
      } catch (error) {
        if (isCancellationError(error)) {
          cancelled = true
          setStatusLine('Batch cancelled.')
          break
        }
        const err = `run: ${normalizeText(error)}`
        failures.push({ label, error: err })
        failedRows.push({ csv_path: path, label, is_holdout: isHoldout, error: err })
        setStatusLine(`Batch: failed on ${label}: ${normalizeText(error)}`)
      }
    }

    await refreshSavedRunCount()
    const failed = failures.length
    const skipHint = skipped ? ` • skipped=${skipped}` : ''
    const msg = `Batch done: ok=${ok}/${n}${skipHint}${failed ? ` • failed=${failed}` : ''}`
    setStatusLine(msg)
    updateBatchMeta(failed ? `${msg} • first=${failures[0]?.label || ''}` : msg)

    const summary: Record<string, unknown> = {
      version: 1,
      kind: 'vectorbt_batch_summary',
      ts: batchTs,
      strategy_kind: strategyKind,
      timeframe: batchTimeframes.size === 1 ? Array.from(batchTimeframes)[0] : null,
      batch_tag: primaryTag || null,
      config_fingerprint: expectedFingerprint || null,
      tags: baseTags,
      holdout_pct: holdoutPct,
      holdout_tag: holdoutTag,
      selected_paths: normalizedSelected,
      holdout_paths: Array.from(holdoutSet),
      counts: { total: n, ok, skipped, failed },
      results: {
        ok: okRows,
        skipped: skippedRows,
        failed: failedRows,
        cancelled,
      },
      runtime_ms: Math.max(0, Math.round((typeof performance !== 'undefined' ? performance.now() : Date.now()) - startedAt)),
    }

    try {
      const saved = (await invokeCommand('save_batch_summary', { summary })) as unknown
      const path = normalizePath(asRecord(saved)?.path)
      setBatchSummaryState(summary, path)
    } catch (error) {
      setBatchSummaryState(summary, '')
      setStatusLine(`Saved batch runs, but failed to write summary JSON: ${normalizeText(error)}`)
    }

    BATCH_RUNNING = false
    BATCH_STOP_REQUESTED = false
    syncBatchButtons()

    if (autoMeta && !cancelled) {
      if (analysisStore) {
        analysisStore.metaStrategyKind = strategyKind === 'CompositeCpu' ? 'CompositeCpu' : 'CompositeMo'
        if (batchTimeframes.size === 1 && !normalizeText(analysisStore.metaTimeframe)) {
          analysisStore.metaTimeframe = normalizeText(Array.from(batchTimeframes)[0])
        }
        analysisStore.metaRequiredTag = primaryTag || analysisStore.metaRequiredTag
        analysisStore.metaHoldoutTag = holdoutTag
        analysisStore.metaEvalHoldout = holdoutSet.size > 0
      }
      await runMeta()
    }
  } catch (error) {
    setErrorStatus(error)
  } finally {
    if (validationStore) {
      validationStore.autoRun = prevAutoRun
      validationStore.autoExportXlsx = prevAutoExportXlsx
      validationStore.autoRevealReport = prevAutoRevealReport
      validationStore.updatedAtUnixMs = Date.now()
    }
    BATCH_RUNNING = false
    BATCH_STOP_REQUESTED = false
    syncBatchButtons()
    renderBatchDatasetList('')
  }
}

export async function batchStop(): Promise<void> {
  try {
    if (!BATCH_RUNNING) return
    BATCH_STOP_REQUESTED = true
    updateBatchMeta('stop after current')
    setStatusLine('Batch will stop after the current dataset.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function batchRefresh(): Promise<void> {
  try {
    if (BATCH_RUNNING) return
    await refreshBatchDatasetsInternal()
    await loadLastBatchSummaryFromDisk()
    setStatusLine('Refreshed batch dataset list.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function batchSelectAll(): Promise<void> {
  try {
    if (BATCH_RUNNING) return
    await ensureBatchDatasetsLoaded()
    const query = normalizeText(getAnalysisStore()?.batchDatasetSearch)
    const filtered = filterBatchDatasets(BATCH_DATASETS_CACHE, query)
    const selected = new Set(readBatchSelectedPaths())
    for (const entry of filtered) {
      selected.add(entry.path)
    }
    setBatchSelectedPaths(Array.from(selected))
    renderBatchDatasetList('')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function batchSelectNone(): Promise<void> {
  try {
    if (BATCH_RUNNING) return
    setBatchSelectedPaths([])
    renderBatchDatasetList('')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function batchRetryFailed(): Promise<void> {
  try {
    if (BATCH_RUNNING) return
    await loadLastBatchSummaryFromDisk()
    const summary = LAST_BATCH_SUMMARY?.summary
    if (!summary) {
      setStatusLine('No batch summary loaded.')
      return
    }
    if (batchFailedCount() <= 0) {
      setStatusLine('No failures to retry.')
      return
    }

    const tags = Array.isArray(summary.tags) ? dedupTags(summary.tags) : []
    const selectedPaths = Array.isArray(summary.selected_paths) ? summary.selected_paths : []
    const analysisStore = getAnalysisStore()
    if (analysisStore) {
      analysisStore.batchTags = tags.join(', ')
      analysisStore.batchHoldoutPct = String(Math.max(0, Math.floor(Number(summary.holdout_pct || 0))))
      analysisStore.batchHoldoutTag = normalizeText(summary.holdout_tag) || 'holdout'
      analysisStore.batchSkipDone = true
      analysisStore.updatedAtUnixMs = Date.now()
    }

    setBatchSelectedPaths(selectedPaths)
    renderBatchDatasetList(`retrying failed (tag=${normalizeText(summary.batch_tag) || 'batch'})`)
    await runBatch()
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function batchRevealSummary(): Promise<void> {
  try {
    if (BATCH_RUNNING) return
    await loadLastBatchSummaryFromDisk()
    const path = normalizePath(LAST_BATCH_SUMMARY?.path)
    if (!path) {
      setStatusLine('No batch summary saved yet.')
      return
    }
    await invokeCommand('reveal_path', { path })
    setStatusLine('Revealed batch summary.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function saveSavedTags(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved meta run first.')
      return
    }

    const store = getAnalysisStore()
    const tags = parseTagsInput(normalizeText(store?.metaSavedTagsDraft))
    await invokeCommand('update_artifact_tags', { path, tags })
    await refreshSavedInternal(path)
    setStatusLine('Saved meta tags.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function refreshSaved(): Promise<void> {
  try {
    const store = getAnalysisStore()
    await refreshSavedInternal(normalizePath(store?.metaSavedSelected))
    setStatusLine('Refreshed saved meta runs.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function loadSaved(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved meta run first.')
      return
    }

    const artifactRaw = (await invokeCommand('load_run_artifact', { path })) as unknown
    const artifact = asRecord(artifactRaw)
    const kind = normalizeText(artifact?.kind)
    if (kind !== 'vectorbt_meta_selection') {
      setStatusLine(`Not a meta artifact: kind='${kind}'`)
      return
    }

    const response = buildMetaSelectionResponseFromArtifact(artifact ?? {}, path)
    renderMetaSelection(response)
    await refreshSavedInternal(path)
    selectMetaResultsTab()
    setStatusLine(`Loaded meta run ${normalizeText(response.ts)}`)
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function openSavedReport(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry()
    const reportPath = normalizePath(entry?.report_xlsx_path)
    if (!reportPath) {
      setStatusLine('No report path for this meta run.')
      return
    }

    try {
      await invokeCommand('open_path', { path: reportPath })
      setStatusLine('Opened report.')
    } catch {
      await invokeCommand('reveal_path', { path: reportPath })
      setStatusLine('Revealed report.')
    }
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function revealSaved(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved meta run first.')
      return
    }
    await invokeCommand('reveal_path', { path })
    setStatusLine('Revealed meta run.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function deleteSaved(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved meta run first.')
      return
    }

    await invokeCommand('delete_run_artifact', { path })
    if (normalizePath(LAST_META_RUN?.meta_json_path) === path) {
      LAST_META_RUN = null
      renderMetaSelection(null)
    }
    await refreshSavedInternal('')
    setStatusLine('Deleted meta run.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function saveStage4SavedTags(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved Stage 4 run first.')
      return
    }

    const store = getAnalysisStore()
    const tags = parseTagsInput(normalizeText(store?.stage4SavedTagsDraft))
    await invokeCommand('update_artifact_tags', { path, tags })
    await refreshStage4SavedInternal(path)
    setStatusLine('Saved Stage 4 tags.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function refreshStage4Saved(): Promise<void> {
  try {
    const store = getAnalysisStore()
    await refreshStage4SavedInternal(normalizePath(store?.stage4SavedSelected))
    setStatusLine('Refreshed saved Stage 4 runs.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function loadStage4Saved(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved Stage 4 run first.')
      return
    }

    const artifactRaw = (await invokeCommand('load_run_artifact', { path })) as unknown
    const artifact = asRecord(artifactRaw)
    const kind = normalizeText(artifact?.kind)
    if (kind !== 'vectorbt_meta_stage4_experimental') {
      setStatusLine(`Not a Stage 4 artifact: kind='${kind}'`)
      return
    }

    const response = buildStage4ResponseFromArtifact(artifact ?? {}, path)
    renderMetaStage4(response)
    await refreshStage4SavedInternal(path)
    selectMetaResultsTab()
    setStatusLine(`Loaded Stage 4 run ${normalizeText(response.ts)}`)
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function revealStage4Saved(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved Stage 4 run first.')
      return
    }
    await invokeCommand('reveal_path', { path })
    setStatusLine('Revealed Stage 4 run.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function deleteStage4Saved(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry()
    const path = normalizePath(entry?.path)
    if (!path) {
      setStatusLine('Select a saved Stage 4 run first.')
      return
    }

    await invokeCommand('delete_run_artifact', { path })
    if (normalizePath(LAST_META_STAGE4_RUN?.stage4_json_path) === path) {
      LAST_META_STAGE4_RUN = null
      renderMetaStage4(null)
    }
    await refreshStage4SavedInternal('')
    setStatusLine('Deleted Stage 4 run.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function mlRefresh(options: MetaMlActionOptions = {}): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope)
    if (!normalizeText(scope.timeframe)) {
      LAST_META_ML_STATUS = null
      patchAnalysisStore({
        metaMlStatus: 'Timeframe required (set Meta timeframe filter or run meta once).',
      })
      updateMetaMlButtons()
      return
    }

    const statusRaw = (await invokeCommand('ml_confidence_model_status', { scope })) as unknown
    LAST_META_ML_STATUS = asRecord(statusRaw) ?? {}
    patchAnalysisStore({
      metaMlStatus: formatMlStatus(LAST_META_ML_STATUS, scope),
    })
    updateMetaMlButtons()
    setStatusLine('Refreshed ML confidence status.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function mlTrain(options: MetaMlActionOptions = {}): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope)
    if (!normalizeText(scope.timeframe)) {
      setStatusLine('Timeframe required for ML model training.')
      return
    }

    await invokeCommand('train_ml_confidence_model', { scope, cfg: null })
    await mlRefresh({ scope })
    const store = getAnalysisStore()
    if (store?.metaUseMlConfidence && asRecordArray(asRecord(LAST_META_RUN?.result)?.groups).length > 0) {
      await mlApply({ scope })
      return
    }
    setStatusLine('ML model trained.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function mlApply(options: MetaMlActionOptions = {}): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope)
    if (!normalizeText(scope.timeframe)) {
      setStatusLine('Timeframe required for ML apply.')
      return
    }

    const run = asRecord(LAST_META_RUN)
    const result = asRecord(run?.result)
    const groups = asRecordArray(result?.groups)
    if (groups.length === 0) {
      setStatusLine('Run or load a meta analysis first.')
      return
    }

    const raw = (await invokeCommand('apply_ml_confidence_model', { scope, groups })) as unknown
    const payload = asRecord(raw) ?? {}
    const confidences = Array.isArray(payload.confidences) ? payload.confidences : []

    for (let i = 0; i < groups.length; i += 1) {
      const group = groups[i]
      if (!group) continue
      const conf = Number(confidences[i])
      group.ml_confidence = Number.isFinite(conf) ? conf : null
    }

    const healthy = asRecord(LAST_META_ML_STATUS?.model)?.meta
    const modelHealthy = asRecord(healthy)?.healthy
    const useMlRank = !!getAnalysisStore()?.metaUseMlConfidence
    if (useMlRank && modelHealthy !== false) {
      groups.sort((a, b) => {
        const lhs = Number(a.ml_confidence)
        const rhs = Number(b.ml_confidence)
        const left = Number.isFinite(lhs) ? lhs : -1
        const right = Number.isFinite(rhs) ? rhs : -1
        return right - left
      })
    }

    if (result) {
      result.groups = groups
      ;(LAST_META_RUN as Record<string, unknown>).result = result
    }

    renderMetaSelection(LAST_META_RUN)

    const warnings = Array.isArray(payload.warnings)
      ? payload.warnings.map((warning) => normalizeText(warning)).filter((warning) => warning.length > 0)
      : []
    if (warnings.length > 0) {
      setStatusLine(`Applied ML confidence (warnings: ${warnings.length}).`)
    } else {
      setStatusLine('Applied ML confidence.')
    }
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function mlDelete(options: MetaMlActionOptions = {}): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope)
    if (!normalizeText(scope.timeframe)) {
      setStatusLine('Timeframe required for ML delete.')
      return
    }

    await invokeCommand('delete_ml_confidence_model', { scope })
    LAST_META_ML_STATUS = null
    patchAnalysisStore({ metaMlStatus: 'Deleted model.' })
    updateMetaMlButtons()
    setStatusLine('Deleted ML confidence model.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function runStage4(options: MetaStage4RunOptions = {}): Promise<void> {
  try {
    let req = normalizeRequestPayload(options.request)
    if (!req) {
      try {
        req = tryBuildAutoMetaStage4Request()
      } catch {
        req = null
      }
    }

    if (!req) {
      setStatusLine('Missing Stage 4 request payload.')
      return
    }

    const payload = (await invokeCommand('run_meta_stage4_experimental', { req })) as unknown
    const response = asRecord(payload)
    if (!response) {
      setStatusLine('Stage 4 returned an invalid response.')
      return
    }

    renderMetaStage4(response)
    await refreshStage4SavedInternal(normalizePath(response.stage4_json_path))
    selectMetaResultsTab()
    setStatusLine(normalizePath(response.stage4_json_path)
      ? `Stage 4 complete. Saved: ${normalizePath(response.stage4_json_path)}`
      : 'Stage 4 complete.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function revealStage4(): Promise<void> {
  try {
    const path = normalizePath(LAST_META_STAGE4_RUN?.stage4_json_path)
    if (!path) {
      setStatusLine('No Stage 4 artifact saved yet.')
      return
    }
    await invokeCommand('reveal_path', { path })
    setStatusLine('Revealed Stage 4 artifact.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function runMeta(options: MetaRunOptions = {}): Promise<void> {
  try {
    let req = normalizeRequestPayload(options.request)
    if (!req) {
      try {
        req = tryBuildAutoMetaRequest()
      } catch {
        req = null
      }
    }

    if (!req) {
      setStatusLine('Missing meta request payload.')
      return
    }

    const payload = (await invokeCommand('run_meta_selection', { req })) as unknown
    const response = asRecord(payload)
    if (!response) {
      setStatusLine('Meta run returned an invalid response.')
      return
    }

    renderMetaSelection(response)
    await refreshSavedInternal(normalizePath(response.meta_json_path))
    if (options.switchTab !== false) {
      selectMetaResultsTab()
    }
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function openMetaReport(): Promise<void> {
  try {
    const reportPath = normalizePath(LAST_META_RUN?.report_xlsx_path)
    if (!reportPath) {
      setStatusLine('No meta report path yet. Run meta analysis first.')
      return
    }

    try {
      await invokeCommand('open_path', { path: reportPath })
      setStatusLine('Opened meta report.')
    } catch {
      await invokeCommand('reveal_path', { path: reportPath })
      setStatusLine('Revealed meta report.')
    }
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function revealMetaFiles(): Promise<void> {
  try {
    const reportPath = normalizePath(LAST_META_RUN?.report_xlsx_path)
    const metaPath = normalizePath(LAST_META_RUN?.meta_json_path)
    const path = reportPath || metaPath
    if (!path) {
      setStatusLine('No meta files yet. Run meta analysis first.')
      return
    }

    await invokeCommand('reveal_path', { path })
    setStatusLine('Revealed meta files.')
  } catch (error) {
    setErrorStatus(error)
  }
}

export async function toggleBatchDataset(path: string, checked: boolean): Promise<void> {
  try {
    const normalizedPath = normalizePath(path)
    if (!normalizedPath) return
    const selected = new Set(readBatchSelectedPaths())
    if (checked) selected.add(normalizedPath)
    else selected.delete(normalizedPath)
    setBatchSelectedPaths(Array.from(selected))
    renderBatchDatasetList('')
  } catch (error) {
    setErrorStatus(error)
  }
}
