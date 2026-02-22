import { defineStore } from 'pinia'
import {
  invokeLegacyDataApplyTimeframeRuntime,
  invokeLegacyDataBrowseCsvRuntime,
  invokeLegacyDataClosePolygonRuntime,
  invokeLegacyDataBrowseMarketDataDirRuntime,
  invokeLegacyDataClearRuntime,
  invokeLegacyDataLoadRuntime,
  invokeLegacyDataLoadSampleCsvRuntime,
  invokeLegacyDataOpenMarketDataDirRuntime,
  invokeLegacyDataOpenPolygonRuntime,
  invokeLegacyDataResetMarketDataDirRuntime,
  invokeLegacyDataSetMarketDataDirRuntime,
} from '../legacy/actions'

export interface DataSampleOption {
  label: string
  path: string
  disabled: boolean
}

function updateLegacyTextField(field: string, value: string, eventType: 'input' | 'change'): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el || el.disabled) return
  if (String(el.value ?? '') !== value) el.value = value
  el.dispatchEvent(new Event(eventType, { bubbles: true }))
}

function updateLegacyCheckboxField(field: string, checked: boolean): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | null
  if (!el || el.disabled) return
  if (String(el.type || '').trim().toLowerCase() !== 'checkbox') return
  if (el.checked !== checked) el.checked = checked
  el.dispatchEvent(new Event('change', { bubbles: true }))
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
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
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
    canApplyTimeframe: true,
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
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

      this.dataId = String(dataId).trim() || '—'
      this.datasetLabel = String(datasetLabel).trim() || '—'
      this.datasetRange = String(datasetRange).trim() || '—'
      this.dataLen = String(dataLen).trim() || '—'
      this.statusLine = String(statusLine).trim() || 'Ready.'
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'input')
    },

    setSelectField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'change')
    },

    setCheckboxField(field: string, checked: boolean): void {
      updateLegacyCheckboxField(field, !!checked)
    },

    browseCsv(): void {
      void invokeLegacyDataBrowseCsvRuntime()
    },

    openPolygon(): void {
      void invokeLegacyDataOpenPolygonRuntime()
    },

    closePolygon(): void {
      void invokeLegacyDataClosePolygonRuntime()
    },

    loadCsv(path?: string): void {
      const nextPath = String(path || '').trim()
      if (nextPath) {
        void invokeLegacyDataLoadRuntime({ path: nextPath })
        return
      }
      void invokeLegacyDataLoadRuntime()
    },

    clearData(): void {
      void invokeLegacyDataClearRuntime()
    },

    applyTimeframe(timeframe: string): void {
      void invokeLegacyDataApplyTimeframeRuntime({ timeframe: String(timeframe || '').trim() })
    },

    browseMarketDataDir(): void {
      void invokeLegacyDataBrowseMarketDataDirRuntime()
    },

    setMarketDataDir(dir: string, moveExisting: boolean): void {
      void invokeLegacyDataSetMarketDataDirRuntime({ dir: String(dir || '').trim(), moveExisting: !!moveExisting })
    },

    resetMarketDataDir(): void {
      void invokeLegacyDataResetMarketDataDirRuntime()
    },

    openMarketDataDir(): void {
      void invokeLegacyDataOpenMarketDataDirRuntime()
    },

    loadSampleCsv(index: number, path?: string): void {
      const nextPath = String(path || '').trim()
      if (nextPath) {
        this.loadCsv(nextPath)
        return
      }
      if (!Number.isFinite(index) || index < 0) return
      void invokeLegacyDataLoadSampleCsvRuntime({ index: Math.floor(index) })
    },
  },
})
