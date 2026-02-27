import { defineStore, type Pinia } from 'pinia'
import {
  applyTimeframe as applyTimeframeAction,
  browseCsv as browseCsvAction,
  browseMarketDataDir as browseMarketDataDirAction,
  clearData as clearDataAction,
  closePolygon as closePolygonAction,
  type DataLoadResult,
  getMarketDataDirInfo as getMarketDataDirInfoAction,
  listSampleCsvs as listSampleCsvsAction,
  loadCsv as loadCsvAction,
  loadSampleCsv as loadSampleCsvAction,
  openMarketDataDir as openMarketDataDirAction,
  openPolygon as openPolygonAction,
  resetMarketDataDir as resetMarketDataDirAction,
  setMarketDataDir as setMarketDataDirAction,
} from '../services/dataService'
import { useWorkflowArtifactsStore } from './workflowArtifactsStore'

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
  const date = new Date(ms)
  if (!Number.isFinite(date.getTime())) return '—'
  return date.toISOString().slice(0, 10)
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
      this.baseDataId = hasLoadedDataset(baseDataId) ? baseDataId : ''
      this.baseDataMeta = (baseDataMeta && typeof baseDataMeta === 'object') ? (baseDataMeta as DataLoadResult['meta']) : null
      this.baseDatasetLabel = String(baseDatasetLabel || '').trim()
      this.baseDatasetContext = String(baseDatasetContext || '').trim()
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyLoadResult(result: DataLoadResult, options?: { treatAsBase?: boolean; statusLine?: string }): void {
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

      if (treatAsBase) {
        this.baseDataId = dataId
        this.baseDataMeta = meta
        this.baseDatasetLabel = fileBasename(path) || this.baseDatasetLabel || this.datasetLabel || '—'
        this.baseDatasetContext = datasetContextText(path, 'csv')
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
        ? datasetContextText(path, 'csv')
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
        markVueUpdate(this)
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        markVueUpdate(this)
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
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
      void openPolygonAction()
    },

    closePolygon(): void {
      void closePolygonAction()
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

