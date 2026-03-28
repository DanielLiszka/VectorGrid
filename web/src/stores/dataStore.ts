import { defineStore, type Pinia } from 'pinia'
import {
  applyTimeframe as applyTimeframeAction,
  browseCsv as browseCsvAction,
  browseMarketDataDir as browseMarketDataDirAction,
  clearPolygonApiKey as clearPolygonApiKeyAction,
  clearData as clearDataAction,
  closePolygon as closePolygonAction,
  downloadPolygonDataset as downloadPolygonDatasetAction,
  getPolygonCacheInfo as getPolygonCacheInfoAction,
  type DataLoadResult,
  getMarketDataDirInfo as getMarketDataDirInfoAction,
  listSampleCsvs as listSampleCsvsAction,
  loadCsv as loadCsvAction,
  loadSampleCsv as loadSampleCsvAction,
  openMarketDataDir as openMarketDataDirAction,
  openPolygon as openPolygonAction,
  type PolygonCacheInfoSummary,
  type PolygonDatasetSummary,
  repairPolygonDataset as repairPolygonDatasetAction,
  resetMarketDataDir as resetMarketDataDirAction,
  setPolygonApiKey as setPolygonApiKeyAction,
  setMarketDataDir as setMarketDataDirAction,
  validatePolygonDataset as validatePolygonDatasetAction,
} from '../services/dataService'
import { formatUtcDateFromUnixMs } from '../utils/uiFormat'
import { resolveRuntimePinia } from './runtimePinia'
import { useWorkflowArtifactsStore } from './workflowArtifactsStore'
import { useWorkflowStore } from './workflowStore'

export interface DataSampleOption {
  label: string
  path: string
  disabled: boolean
}

interface DataState {
  activeControlsTab: string
  csvPath: string
  recentCsvPaths: string[]
  sampleCsvOptions: DataSampleOption[]
  sampleCsvVisible: boolean
  marketDataDir: string
  marketDataMoveExisting: boolean
  marketDataDirHint: string
  dataId: string
  datasetLabel: string
  datasetRange: string
  dataLen: string
  dataTimeframe: string
  dataTimeframeHint: string
  autoRestoreDataset: boolean
  datasetContext: string
  dataFields: string[]
  dataFieldHint: string
  canBrowseCsv: boolean
  canPolygonOpen: boolean
  canLoadCsv: boolean
  canClearData: boolean
  canBrowseMarketDataDir: boolean
  canSetMarketDataDir: boolean
  canResetMarketDataDir: boolean
  canOpenMarketDataDir: boolean
  canApplyTimeframe: boolean
  polygonOpen: boolean
  polygonApiKey: string
  polygonMarket: string
  polygonSymbol: string
  polygonFrom: string
  polygonTo: string
  polygonTimeframe: string
  polygonAdjusted: boolean
  polygonCacheDir: string
  polygonHasApiKey: boolean
  polygonDatasets: PolygonDatasetSummary[]
  polygonSelectedPath: string
  polygonSelectedMeta: string
  polygonCacheMeta: string
  canPolygonRefresh: boolean
  canPolygonSaveApiKey: boolean
  canPolygonClearApiKey: boolean
  canPolygonDownload: boolean
  canPolygonLoadSelected: boolean
  canPolygonValidateSelected: boolean
  canPolygonRepairSelected: boolean
  statusLine: string
  baseDataId: string
  baseDataMeta: DataLoadResult['meta'] | null
  baseDatasetLabel: string
  baseDatasetContext: string
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

function fileBasename(path: string): string {
  const normalized = String(path || '').trim().replace(/\\/g, '/')
  if (!normalized) return ''
  const parts = normalized.split('/')
  return String(parts[parts.length - 1] || '').trim()
}

function normalizeUnixToDateText(value: number): string {
  const ts = Number(value)
  if (!Number.isFinite(ts) || ts <= 0) return '—'
  const ms = ts >= 1_000_000_000_000 ? ts : ts * 1000
  return formatUtcDateFromUnixMs(ms)
}

function datasetRangeText(meta: DataLoadResult['meta'] | null, timeframe: string): string {
  const row = meta && typeof meta === 'object' ? meta : null
  if (!row) return '—'
  const tf = String(timeframe || row.timeframe || '').trim()
  const minTs = normalizeUnixToDateText(Number(row.minTs))
  const maxTs = normalizeUnixToDateText(Number(row.maxTs))
  const parts: string[] = []
  if (tf) parts.push(tf)
  if (minTs !== '—' || maxTs !== '—') parts.push(`${minTs} → ${maxTs}`)
  return parts.length ? parts.join(' • ') : '—'
}

function datasetContextText(path: string, source: string): string {
  const parts: string[] = []
  const src = String(source || '').trim()
  if (src) parts.push(`source=${src}`)
  const name = fileBasename(path)
  if (name) parts.push(`file=${name}`)
  return parts.length ? parts.join(' • ') : '—'
}

function dataFieldList(meta: DataLoadResult['meta'] | null): string[] {
  const row = meta && typeof meta === 'object' ? meta : null
  if (!row) return []
  const fields: string[] = []
  if (row.hasOpen) fields.push('open')
  if (row.hasHigh) fields.push('high')
  if (row.hasLow) fields.push('low')
  if (row.hasClose) fields.push('close')
  if (row.hasVolume) fields.push('volume')
  return fields
}

function dataFieldHint(meta: DataLoadResult['meta'] | null): string {
  const row = meta && typeof meta === 'object' ? meta : null
  if (!row) return ''
  const msgs: string[] = []
  if (!row.hasClose) msgs.push('Missing close (required).')
  if (!row.hasVolume) msgs.push('No volume -> VWMA/VPWMA disabled.')
  if (!row.hasHigh || !row.hasLow) msgs.push('No high/low -> FRAMA + some sources disabled.')
  if (!row.hasOpen) msgs.push('No open -> open/ohlc4 disabled.')
  return msgs.join(' · ')
}

function hasLoadedDataset(dataId: string): boolean {
  const value = String(dataId || '').trim()
  return value.length > 0 && value !== '—' && value !== '-'
}

function timeframeHintText(baseTimeframe: string, currentTimeframe: string, hasBase: boolean): string {
  if (!hasBase) return 'Load a dataset to enable timeframe resampling.'
  const baseLabel = String(baseTimeframe || '').trim() || 'unknown'
  const effective = String(currentTimeframe || '').trim() || baseLabel
  const derived = !!baseTimeframe && !!effective && baseLabel !== effective
  if (derived) {
    return `Using ${effective} derived from base ${baseLabel} (no extra downloads). Periods are in candles.`
  }
  return `Base timeframe: ${baseLabel}. Choose a higher timeframe to derive candles (no extra downloads). Periods are in candles.`
}

function markVueUpdate(state: DataState): void {
  state.source = 'vue'
  state.updatedAtUnixMs = Date.now()
}

function setControlsBusy(state: DataState, busy: boolean): void {
  const canMutate = !busy
  state.canBrowseCsv = canMutate
  state.canPolygonOpen = canMutate
  state.canLoadCsv = canMutate
  state.canClearData = canMutate
  state.canBrowseMarketDataDir = canMutate
  state.canSetMarketDataDir = canMutate
  state.canResetMarketDataDir = canMutate
  state.canOpenMarketDataDir = canMutate
  state.canApplyTimeframe = canMutate && hasLoadedDataset(state.baseDataId || state.dataId)
  state.canPolygonRefresh = canMutate
  state.canPolygonSaveApiKey = canMutate && !!String(state.polygonApiKey || '').trim()
  state.canPolygonClearApiKey = canMutate && state.polygonHasApiKey
  state.canPolygonDownload = canMutate && state.polygonHasApiKey && !!String(state.polygonSymbol || '').trim() && !!String(state.polygonFrom || '').trim() && !!String(state.polygonTo || '').trim()
  const hasSelectedPolygon = !!String(state.polygonSelectedPath || '').trim()
  state.canPolygonLoadSelected = canMutate && hasSelectedPolygon
  state.canPolygonValidateSelected = canMutate && hasSelectedPolygon
  state.canPolygonRepairSelected = canMutate && hasSelectedPolygon
}

function pushRecentCsvPath(recent: string[], path: string): string[] {
  const nextPath = String(path || '').trim()
  if (!nextPath) return recent
  const out = [nextPath]
  for (const item of recent) {
    const normalized = String(item || '').trim()
    if (!normalized || normalized === nextPath) continue
    out.push(normalized)
    if (out.length >= 8) break
  }
  return out
}

function normalizedList(values: unknown): string[] {
  if (!Array.isArray(values)) return []
  return values
    .map((value) => String(value || '').trim())
    .filter((value) => value.length > 0)
}

function normalizedSamples(values: unknown): DataSampleOption[] {
  if (!Array.isArray(values)) return []
  const entries: DataSampleOption[] = []
  for (const item of values) {
    if (!item || typeof item !== 'object') continue
    const typed = item as Partial<DataSampleOption>
    const label = String(typed.label || '').trim()
    if (!label) continue
    entries.push({
      label,
      path: String(typed.path || '').trim(),
      disabled: Boolean(typed.disabled),
    })
  }
  return entries
}

function normalizeDatasetDateInput(value: string): string {
  const next = String(value || '').trim()
  if (!next) return ''
  return /^\d{4}-\d{2}-\d{2}$/.test(next) ? next : ''
}

function polygonIntegrityText(dataset: PolygonDatasetSummary): string {
  const integrity = dataset.integrity
  if (!integrity) return 'Integrity: unchecked'
  return `Integrity: parse=${integrity.parseErrors} dup=${integrity.duplicates} order=${integrity.outOfOrder} gaps=${integrity.sameDayGapsGt2Min}`
}

function polygonDatasetMetaText(dataset: PolygonDatasetSummary): string {
  const label = String(dataset.displaySymbol || dataset.polygonTicker || fileBasename(dataset.csvPath) || dataset.csvPath).trim()
  const parts = [
    label,
    dataset.timeframe || '1m',
    dataset.adjusted ? 'adjusted' : 'raw',
    dataset.rowCount > 0 ? `${dataset.rowCount} rows` : '',
    polygonIntegrityText(dataset),
  ].filter((item) => item.length > 0)
  return parts.join(' · ')
}

function polygonCacheMetaText(info: PolygonCacheInfoSummary): string {
  const count = info.datasets.length
  const parts = [
    info.hasApiKey ? 'API key saved' : 'API key missing',
    info.cacheDir ? `Cache: ${info.cacheDir}` : '',
    `${count} cached dataset${count === 1 ? '' : 's'}`,
  ].filter((item) => item.length > 0)
  return parts.join(' · ') || 'Polygon cache unavailable.'
}

function selectPolygonDatasetPath(state: DataState, preferredPath = ''): void {
  const nextPath = String(preferredPath || '').trim()
  if (nextPath && state.polygonDatasets.some((row) => row.csvPath === nextPath)) {
    state.polygonSelectedPath = nextPath
  } else if (state.polygonSelectedPath && state.polygonDatasets.some((row) => row.csvPath === state.polygonSelectedPath)) {
    // keep current selection
  } else {
    state.polygonSelectedPath = state.polygonDatasets[0]?.csvPath || ''
  }
  const selected = state.polygonDatasets.find((row) => row.csvPath === state.polygonSelectedPath) || null
  state.polygonSelectedMeta = selected ? polygonDatasetMetaText(selected) : 'Select a cached Polygon dataset to inspect, validate, repair, or load.'
}

function applyPolygonCacheInfo(state: DataState, info: PolygonCacheInfoSummary, preferredPath = ''): void {
  state.polygonCacheDir = String(info.cacheDir || '').trim()
  state.polygonHasApiKey = !!info.hasApiKey
  state.polygonDatasets = Array.isArray(info.datasets) ? [...info.datasets] : []
  state.polygonCacheMeta = polygonCacheMetaText(info)
  selectPolygonDatasetPath(state, preferredPath)
  setControlsBusy(state, false)
}

function applyVueTextFieldUpdate(state: DataState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false
  switch (key) {
    case 'csvPath':
      state.csvPath = next
      return true
    case 'marketDataDir':
      state.marketDataDir = next
      return true
    case 'dataTimeframe':
      state.dataTimeframe = next
      return true
    case 'polygonApiKey':
      state.polygonApiKey = next
      return true
    case 'polygonMarket':
      state.polygonMarket = next || 'stocks'
      return true
    case 'polygonSymbol':
      state.polygonSymbol = next.toUpperCase()
      return true
    case 'polygonFrom':
      state.polygonFrom = normalizeDatasetDateInput(next) || next
      return true
    case 'polygonTo':
      state.polygonTo = normalizeDatasetDateInput(next) || next
      return true
    case 'polygonTimeframe':
      state.polygonTimeframe = next || '1m'
      return true
    case 'polygonSelectedPath':
      state.polygonSelectedPath = next
      selectPolygonDatasetPath(state, next)
      return true
    default:
      return false
  }
}

function applyVueCheckboxFieldUpdate(state: DataState, field: string, checked: boolean): boolean {
  const key = String(field || '').trim()
  const next = !!checked
  if (!key) return false
  switch (key) {
    case 'marketDataMoveExisting':
      state.marketDataMoveExisting = next
      return true
    case 'autoRestoreDataset':
      state.autoRestoreDataset = next
      return true
    case 'polygonAdjusted':
      state.polygonAdjusted = next
      return true
    default:
      return false
  }
}

export const useDataStore = defineStore('data', {
  state: (): DataState => ({
    activeControlsTab: 'data',
    csvPath: '',
    recentCsvPaths: [],
    sampleCsvOptions: [],
    sampleCsvVisible: false,
    marketDataDir: '',
    marketDataMoveExisting: true,
    marketDataDirHint: '—',
    dataId: '—',
    datasetLabel: '—',
    datasetRange: '—',
    dataLen: '—',
    dataTimeframe: '',
    dataTimeframeHint: '—',
    autoRestoreDataset: true,
    datasetContext: '—',
    dataFields: [],
    dataFieldHint: '',
    canBrowseCsv: true,
    canPolygonOpen: true,
    canLoadCsv: true,
    canClearData: true,
    canBrowseMarketDataDir: true,
    canSetMarketDataDir: true,
    canResetMarketDataDir: true,
    canOpenMarketDataDir: true,
    canApplyTimeframe: false,
    polygonOpen: false,
    polygonApiKey: '',
    polygonMarket: 'stocks',
    polygonSymbol: '',
    polygonFrom: '',
    polygonTo: '',
    polygonTimeframe: '1m',
    polygonAdjusted: true,
    polygonCacheDir: '',
    polygonHasApiKey: false,
    polygonDatasets: [],
    polygonSelectedPath: '',
    polygonSelectedMeta: 'Select a cached Polygon dataset to inspect, validate, repair, or load.',
    polygonCacheMeta: 'Polygon cache unavailable.',
    canPolygonRefresh: true,
    canPolygonSaveApiKey: false,
    canPolygonClearApiKey: false,
    canPolygonDownload: false,
    canPolygonLoadSelected: false,
    canPolygonValidateSelected: false,
    canPolygonRepairSelected: false,
    statusLine: 'Ready.',
    baseDataId: '',
    baseDataMeta: null,
    baseDatasetLabel: '',
    baseDatasetContext: '',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    hydrateStartupState(): void {
      void getMarketDataDirInfoAction()
        .then((info) => {
          const configured = String(info?.configuredRootDir || '').trim()
          const hint = String(info?.hint || this.marketDataDirHint || '—').trim() || '—'
          this.marketDataDir = configured
          this.marketDataDirHint = hint
          markVueUpdate(this)
        })
        .catch(() => undefined)

      void listSampleCsvsAction()
        .then((samples) => {
          this.sampleCsvOptions = Array.isArray(samples) ? samples : []
          this.sampleCsvVisible = this.sampleCsvOptions.length > 0
          markVueUpdate(this)
        })
        .catch(() => undefined)

      void getPolygonCacheInfoAction()
        .then((info) => {
          applyPolygonCacheInfo(this, info)
          markVueUpdate(this)
        })
        .catch(() => undefined)
    },

    applyLegacySnapshot(snapshot: Partial<DataState>): void {
      this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
      this.csvPath = String(snapshot.csvPath ?? this.csvPath ?? '').trim()
      this.recentCsvPaths = normalizedList(snapshot.recentCsvPaths ?? this.recentCsvPaths ?? [])
      this.sampleCsvOptions = normalizedSamples(snapshot.sampleCsvOptions ?? this.sampleCsvOptions ?? [])
      this.sampleCsvVisible = Boolean(snapshot.sampleCsvVisible ?? this.sampleCsvVisible)
      this.marketDataDir = String(snapshot.marketDataDir ?? this.marketDataDir ?? '').trim()
      this.marketDataMoveExisting = Boolean(snapshot.marketDataMoveExisting ?? this.marketDataMoveExisting)
      this.marketDataDirHint = String(snapshot.marketDataDirHint ?? this.marketDataDirHint ?? '—').trim() || '—'
      const dataId = snapshot.dataId ?? this.dataId ?? '—'
      const datasetLabel = snapshot.datasetLabel ?? this.datasetLabel ?? '—'
      const datasetRange = snapshot.datasetRange ?? this.datasetRange ?? '—'
      const dataLen = snapshot.dataLen ?? this.dataLen ?? '—'
      this.dataTimeframe = String(snapshot.dataTimeframe ?? this.dataTimeframe ?? '').trim()
      this.dataTimeframeHint = String(snapshot.dataTimeframeHint ?? this.dataTimeframeHint ?? '—').trim() || '—'
      this.autoRestoreDataset = Boolean(snapshot.autoRestoreDataset ?? this.autoRestoreDataset)
      this.datasetContext = String(snapshot.datasetContext ?? this.datasetContext ?? '—').trim() || '—'
      this.dataFields = normalizedList(snapshot.dataFields ?? this.dataFields ?? [])
      this.dataFieldHint = String(snapshot.dataFieldHint ?? this.dataFieldHint ?? '').trim()
      this.canBrowseCsv = Boolean(snapshot.canBrowseCsv ?? this.canBrowseCsv)
      this.canPolygonOpen = Boolean(snapshot.canPolygonOpen ?? this.canPolygonOpen)
      this.canLoadCsv = Boolean(snapshot.canLoadCsv ?? this.canLoadCsv)
      this.canClearData = Boolean(snapshot.canClearData ?? this.canClearData)
      this.canBrowseMarketDataDir = Boolean(snapshot.canBrowseMarketDataDir ?? this.canBrowseMarketDataDir)
      this.canSetMarketDataDir = Boolean(snapshot.canSetMarketDataDir ?? this.canSetMarketDataDir)
      this.canResetMarketDataDir = Boolean(snapshot.canResetMarketDataDir ?? this.canResetMarketDataDir)
      this.canOpenMarketDataDir = Boolean(snapshot.canOpenMarketDataDir ?? this.canOpenMarketDataDir)
      this.canApplyTimeframe = Boolean(snapshot.canApplyTimeframe ?? this.canApplyTimeframe)
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
      this.canPolygonRefresh = Boolean(snapshot.canPolygonRefresh ?? this.canPolygonRefresh)
      this.canPolygonSaveApiKey = Boolean(snapshot.canPolygonSaveApiKey ?? this.canPolygonSaveApiKey)
      this.canPolygonClearApiKey = Boolean(snapshot.canPolygonClearApiKey ?? this.canPolygonClearApiKey)
      this.canPolygonDownload = Boolean(snapshot.canPolygonDownload ?? this.canPolygonDownload)
      this.canPolygonLoadSelected = Boolean(snapshot.canPolygonLoadSelected ?? this.canPolygonLoadSelected)
      this.canPolygonValidateSelected = Boolean(snapshot.canPolygonValidateSelected ?? this.canPolygonValidateSelected)
      this.canPolygonRepairSelected = Boolean(snapshot.canPolygonRepairSelected ?? this.canPolygonRepairSelected)
      const statusLine = snapshot.statusLine ?? this.statusLine ?? 'Ready.'
      const baseDataId = String(snapshot.baseDataId ?? this.baseDataId ?? '').trim()
      const baseDataMeta = snapshot.baseDataMeta ?? this.baseDataMeta ?? null
      const baseDatasetLabel = snapshot.baseDatasetLabel ?? this.baseDatasetLabel ?? ''
      const baseDatasetContext = snapshot.baseDatasetContext ?? this.baseDatasetContext ?? ''

      this.dataId = String(dataId).trim() || '—'
      this.datasetLabel = String(datasetLabel).trim() || '—'
      this.datasetRange = String(datasetRange).trim() || '—'
      this.dataLen = String(dataLen).trim() || '—'
      this.statusLine = String(statusLine).trim() || 'Ready.'
      selectPolygonDatasetPath(this, this.polygonSelectedPath)
      setControlsBusy(this, false)
      this.baseDataId = hasLoadedDataset(baseDataId) ? baseDataId : ''
      this.baseDataMeta = (baseDataMeta && typeof baseDataMeta === 'object') ? (baseDataMeta as DataLoadResult['meta']) : null
      this.baseDatasetLabel = String(baseDatasetLabel || '').trim()
      this.baseDatasetContext = String(baseDatasetContext || '').trim()
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyLoadResult(result: DataLoadResult, options?: { treatAsBase?: boolean; statusLine?: string; source?: 'csv' | 'polygon' }): void {
      const row = result && typeof result === 'object' ? result : null
      if (!row) return
      const dataId = String(row.dataId || '').trim()
      const meta = row.meta && typeof row.meta === 'object' ? row.meta : null
      const path = String(row.path || '').trim()
      const treatAsBase = options?.treatAsBase !== false

      if (path) {
        this.csvPath = path
        this.recentCsvPaths = pushRecentCsvPath(this.recentCsvPaths, path)
      }

      const source = options?.source || 'csv'

      if (treatAsBase) {
        this.baseDataId = dataId
        this.baseDataMeta = meta
        this.baseDatasetLabel = fileBasename(path) || this.baseDatasetLabel || this.datasetLabel || '—'
        this.baseDatasetContext = datasetContextText(path, source)
      }

      const baseMeta = this.baseDataMeta && typeof this.baseDataMeta === 'object' ? this.baseDataMeta : meta
      const baseTf = String(baseMeta?.timeframe || '').trim()
      const effectiveTf = String(meta?.timeframe || '').trim() || baseTf
      const derived = !!baseTf && !!effectiveTf && baseTf !== effectiveTf

      this.dataId = dataId || this.dataId || '—'
      this.dataLen = meta ? String(meta.len) : (this.dataLen || '—')
      this.datasetLabel = treatAsBase
        ? (fileBasename(path) || this.datasetLabel || '—')
        : (this.baseDatasetLabel || this.datasetLabel || '—')
      this.datasetContext = treatAsBase
        ? datasetContextText(path, source)
        : (this.baseDatasetContext || this.datasetContext || '—')
      this.datasetRange = datasetRangeText(meta, effectiveTf)
      this.dataFields = dataFieldList(meta)
      this.dataFieldHint = dataFieldHint(meta)
      this.dataTimeframe = derived ? effectiveTf : ''
      this.dataTimeframeHint = timeframeHintText(baseTf, effectiveTf, hasLoadedDataset(this.baseDataId || this.dataId))
      this.canApplyTimeframe = hasLoadedDataset(this.baseDataId || this.dataId)
      this.statusLine = String(options?.statusLine || this.statusLine || 'Ready.').trim() || 'Ready.'
      markVueUpdate(this)
    },

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
      markVueUpdate(this)
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        setControlsBusy(this, false)
        markVueUpdate(this)
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        setControlsBusy(this, false)
        markVueUpdate(this)
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
        setControlsBusy(this, false)
        markVueUpdate(this)
      }
    },

    browseCsv(): void {
      setControlsBusy(this, true)
      this.statusLine = 'Loading data...'
      markVueUpdate(this)

      void browseCsvAction()
        .then((result) => {
          if (!result) return
          const status = result.cacheHit
            ? 'Loaded (binary cache).'
            : (result.cachePath ? 'Loaded (CSV parsed + cached).' : 'Loaded.')
          this.applyLoadResult(result, { treatAsBase: true, statusLine: status })
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Data load failed.'
          markVueUpdate(this)
        })
        .finally(() => {
          setControlsBusy(this, false)
          markVueUpdate(this)
        })
    },

    openPolygon(): void {
      const pinia = resolveRuntimePinia()
      if (pinia) {
        useWorkflowStore(pinia).selectWorkspace('workflow', 'data')
      }
      this.polygonOpen = true
      markVueUpdate(this)
      void openPolygonAction().catch(() => undefined)
      void getPolygonCacheInfoAction()
        .then((info) => {
          applyPolygonCacheInfo(this, info, this.polygonSelectedPath)
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to refresh Polygon cache.'
          markVueUpdate(this)
        })
    },

    closePolygon(): void {
      this.polygonOpen = false
      markVueUpdate(this)
      void closePolygonAction().catch(() => undefined)
    },

    refreshPolygon(): void {
      setControlsBusy(this, true)
      this.statusLine = 'Refreshing Polygon cache...'
      markVueUpdate(this)
      void getPolygonCacheInfoAction()
        .then((info) => {
          applyPolygonCacheInfo(this, info, this.polygonSelectedPath)
          this.statusLine = 'Refreshed Polygon cache.'
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false)
          this.statusLine = String(error || '').trim() || 'Failed to refresh Polygon cache.'
          markVueUpdate(this)
        })
    },

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
        .then((info) => {
          applyPolygonCacheInfo(this, info, this.polygonSelectedPath)
          this.statusLine = 'Saved Polygon API key.'
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false)
          this.statusLine = String(error || '').trim() || 'Failed to save Polygon API key.'
          markVueUpdate(this)
        })
    },

    clearPolygonApiKey(): void {
      setControlsBusy(this, true)
      this.statusLine = 'Clearing Polygon API key...'
      markVueUpdate(this)
      void clearPolygonApiKeyAction()
        .then((info) => {
          this.polygonApiKey = ''
          applyPolygonCacheInfo(this, info, this.polygonSelectedPath)
          this.statusLine = 'Cleared Polygon API key.'
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false)
          this.statusLine = String(error || '').trim() || 'Failed to clear Polygon API key.'
          markVueUpdate(this)
        })
    },

    selectPolygonDataset(path: string): void {
      this.polygonSelectedPath = String(path || '').trim()
      selectPolygonDatasetPath(this, this.polygonSelectedPath)
      markVueUpdate(this)
    },

    loadPolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || '').trim()
      if (!nextPath) {
        this.statusLine = 'Select a cached Polygon dataset first.'
        markVueUpdate(this)
        return
      }
      this.polygonSelectedPath = nextPath
      setControlsBusy(this, true)
      this.statusLine = 'Loading Polygon dataset...'
      markVueUpdate(this)
      void loadCsvAction(nextPath)
        .then((result) => {
          const status = result.cacheHit
            ? 'Loaded Polygon dataset (binary cache).'
            : (result.cachePath ? 'Loaded Polygon dataset (CSV parsed + cached).' : 'Loaded Polygon dataset.')
          this.applyLoadResult(result, { treatAsBase: true, statusLine: status, source: 'polygon' })
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Polygon data load failed.'
          markVueUpdate(this)
        })
        .finally(() => {
          setControlsBusy(this, false)
          markVueUpdate(this)
        })
    },

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
        .then(async (result) => {
          const info = await getPolygonCacheInfoAction()
          applyPolygonCacheInfo(this, info, result.csvPath)
          const sourcePath = String(result.csvPath || '').trim()
          const symbol = String(this.polygonSymbol || '').trim().toUpperCase()
          const cacheNote = result.usedCacheOnly ? 'cache reused' : `${result.downloadedBars} bars downloaded`
          const baseTimeframe = String(result.baseLoad?.meta?.timeframe || '').trim()
          const effectiveTimeframe = String(result.load?.meta?.timeframe || '').trim()
          this.applyLoadResult(result.baseLoad, {
            treatAsBase: true,
            statusLine: `Loaded Polygon ${symbol || 'dataset'} (${cacheNote}).`,
            source: 'polygon',
          })
          if (effectiveTimeframe && baseTimeframe && effectiveTimeframe !== baseTimeframe) {
            this.applyLoadResult(result.load, {
              treatAsBase: false,
              statusLine: `Loaded Polygon ${symbol || 'dataset'} (${cacheNote}).`,
              source: 'polygon',
            })
          }
          this.csvPath = sourcePath || this.csvPath
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false)
          this.statusLine = String(error || '').trim() || 'Polygon download failed.'
          markVueUpdate(this)
        })
    },

    validatePolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || '').trim()
      if (!nextPath) {
        this.statusLine = 'Select a cached Polygon dataset first.'
        markVueUpdate(this)
        return
      }
      setControlsBusy(this, true)
      this.statusLine = 'Validating Polygon dataset...'
      markVueUpdate(this)
      void validatePolygonDatasetAction(nextPath)
        .then((dataset) => {
          const updated = this.polygonDatasets.map((row) => row.csvPath === dataset.csvPath ? dataset : row)
          this.polygonDatasets = updated
          selectPolygonDatasetPath(this, dataset.csvPath)
          setControlsBusy(this, false)
          this.statusLine = `Validated Polygon dataset: ${fileBasename(dataset.csvPath) || dataset.displaySymbol || dataset.polygonTicker}.`
          this.polygonCacheMeta = polygonCacheMetaText({
            cacheDir: this.polygonCacheDir,
            hasApiKey: this.polygonHasApiKey,
            datasets: this.polygonDatasets,
          })
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false)
          this.statusLine = String(error || '').trim() || 'Polygon validation failed.'
          markVueUpdate(this)
        })
    },

    repairPolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || '').trim()
      if (!nextPath) {
        this.statusLine = 'Select a cached Polygon dataset first.'
        markVueUpdate(this)
        return
      }
      setControlsBusy(this, true)
      this.statusLine = 'Repairing Polygon dataset...'
      markVueUpdate(this)
      void repairPolygonDatasetAction(nextPath)
        .then((dataset) => {
          const updated = this.polygonDatasets.map((row) => row.csvPath === dataset.csvPath ? dataset : row)
          this.polygonDatasets = updated
          selectPolygonDatasetPath(this, dataset.csvPath)
          setControlsBusy(this, false)
          this.statusLine = `Repaired Polygon dataset: ${fileBasename(dataset.csvPath) || dataset.displaySymbol || dataset.polygonTicker}.`
          this.polygonCacheMeta = polygonCacheMetaText({
            cacheDir: this.polygonCacheDir,
            hasApiKey: this.polygonHasApiKey,
            datasets: this.polygonDatasets,
          })
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false)
          this.statusLine = String(error || '').trim() || 'Polygon repair failed.'
          markVueUpdate(this)
        })
    },

    loadCsv(path?: string): void {
      const nextPath = String(path ?? this.csvPath ?? '').trim()
      if (!nextPath) {
        this.statusLine = 'Enter a CSV path.'
        markVueUpdate(this)
        return
      }

      this.csvPath = nextPath
      setControlsBusy(this, true)
      this.statusLine = 'Loading data...'
      markVueUpdate(this)

      void loadCsvAction(nextPath)
        .then((result) => {
          const status = result.cacheHit
            ? 'Loaded (binary cache).'
            : (result.cachePath ? 'Loaded (CSV parsed + cached).' : 'Loaded.')
          this.applyLoadResult(result, { treatAsBase: true, statusLine: status })
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Data load failed.'
          markVueUpdate(this)
        })
        .finally(() => {
          setControlsBusy(this, false)
          markVueUpdate(this)
        })
    },

    clearData(): void {
      void clearDataAction().catch(() => undefined)
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
      this.canApplyTimeframe = false
      this.statusLine = 'Ready.'
      const pinia = (this as unknown as { $pinia?: Pinia }).$pinia
      if (pinia) useWorkflowArtifactsStore(pinia).clearArtifacts()
      markVueUpdate(this)
    },

    applyTimeframe(timeframe: string): void {
      const baseId = String(this.baseDataId || '').trim()
      if (!hasLoadedDataset(baseId)) {
        this.statusLine = 'Load a dataset first (missing base data id).'
        markVueUpdate(this)
        return
      }

      const target = String(timeframe || '').trim()
      this.dataTimeframe = target
      markVueUpdate(this)

      const baseTf = String(this.baseDataMeta?.timeframe || '').trim()
      if (!target || (baseTf && target === baseTf)) {
        this.applyBaseDatasetState()
        return
      }

      setControlsBusy(this, true)
      this.statusLine = `Resampling to ${target}...`
      markVueUpdate(this)

      void applyTimeframeAction(target, baseId)
        .then((result) => {
          const effective = String(result.meta?.timeframe || target).trim() || target
          const status = result.cacheHit ? `Using ${effective} (cached).` : `Using ${effective}.`
          this.applyLoadResult(result, { treatAsBase: false, statusLine: status })
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || `Resample to ${target} failed.`
          this.dataTimeframe = ''
          markVueUpdate(this)
        })
        .finally(() => {
          setControlsBusy(this, false)
          markVueUpdate(this)
        })
    },

    browseMarketDataDir(): void {
      void browseMarketDataDirAction()
        .then((pickedPath) => {
          const path = String(pickedPath || '').trim()
          if (!path) return
          this.marketDataDir = path
          this.statusLine = 'Selected market data folder.'
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          this.statusLine = `Market data browse failed: ${String(error || '')}`.trim()
          markVueUpdate(this)
        })
    },

    setMarketDataDir(dir: string, moveExisting: boolean): void {
      const nextDir = String(dir || '').trim()
      const nextMoveExisting = !!moveExisting
      if (nextDir) {
        this.marketDataDir = nextDir
      }
      this.marketDataMoveExisting = nextMoveExisting
      markVueUpdate(this)

      void setMarketDataDirAction(nextDir, nextMoveExisting)
        .then((info) => {
          this.marketDataDir = String(info?.configuredRootDir || '').trim()
          this.marketDataDirHint = String(info?.hint || this.marketDataDirHint || '—').trim() || '—'
          this.statusLine = 'Updated market data folder.'
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to update market data folder.'
          markVueUpdate(this)
        })
    },

    resetMarketDataDir(): void {
      void resetMarketDataDirAction()
        .then((info) => {
          this.marketDataDir = String(info?.configuredRootDir || '').trim()
          this.marketDataDirHint = String(info?.hint || this.marketDataDirHint || '—').trim() || '—'
          this.statusLine = 'Reset market data folder to default.'
          markVueUpdate(this)
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to reset market data folder.'
          markVueUpdate(this)
        })
    },

    openMarketDataDir(): void {
      void openMarketDataDirAction()
        .then((result) => {
          const path = String(result?.path || '').trim()
          if (!path) return
          this.statusLine = `${result.openedWith === 'reveal_path' ? 'Revealed' : 'Opened'}: ${path}`
          markVueUpdate(this)
          if (!this.marketDataDirHint || this.marketDataDirHint === '—') {
            void getMarketDataDirInfoAction()
              .then((info) => {
                this.marketDataDirHint = String(info?.hint || this.marketDataDirHint || '—').trim() || '—'
                markVueUpdate(this)
              })
              .catch(() => undefined)
          }
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to open market data folder.'
          markVueUpdate(this)
        })
    },

    loadSampleCsv(index: number, path?: string): void {
      setControlsBusy(this, true)
      this.statusLine = 'Loading data...'
      markVueUpdate(this)

      void loadSampleCsvAction(index, path)
        .then((result) => {
          if (!result) return
          const status = result.cacheHit
            ? 'Loaded (binary cache).'
            : (result.cachePath ? 'Loaded (CSV parsed + cached).' : 'Loaded.')
          this.applyLoadResult(result, { treatAsBase: true, statusLine: status })
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Data load failed.'
          markVueUpdate(this)
        })
        .finally(() => {
          setControlsBusy(this, false)
          markVueUpdate(this)
        })
    },
  },
})

