import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { bindRuntimePinia } from './runtimePinia'
import * as analysisService from '../services/analysisService'
import * as dataService from '../services/dataService'
import * as optimizeService from '../services/optimizeService'
import * as validationService from '../services/validationService'
import { invokeCommand } from '../tauri/invoke'
import { useCommandActionStore } from './commandActionStore'
import { useDataStore } from './dataStore'
import { useSettingsStore } from './settingsStore'
import { useWorkflowArtifactsStore } from './workflowArtifactsStore'

vi.mock('../services/analysisService', () => ({
  drillClearTradeSelection: vi.fn(async () => undefined),
  drillNextTrade: vi.fn(async () => undefined),
  drillPrevTrade: vi.fn(async () => undefined),
  exportTopCsv: vi.fn(async () => undefined),
  exportTopJson: vi.fn(async () => undefined),
}))

vi.mock('../services/dataService', () => ({
  browseCsv: vi.fn(async () => null),
  getPolygonCacheInfo: vi.fn(async () => ({ cacheDir: 'C:/polygon', hasApiKey: false, datasets: [] })),
  loadCsv: vi.fn(async () => ({
    path: 'C:/tmp/aapl.csv',
    dataId: 'data-1',
    cacheHit: false,
    cachePath: '',
    loadSource: 'csv_parsed_cache_write_skipped',
    meta: {
      len: 1,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1,
      maxTs: 1,
      timeframe: '1m',
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
  })),
  openPolygon: vi.fn(async () => undefined),
}))

vi.mock('../services/optimizeService', () => ({
  cancelOptimization: vi.fn(async () => undefined),
  runOptimization: vi.fn(async () => undefined),
  runValidation: vi.fn(async () => undefined),
}))

vi.mock('../services/validationService', () => ({
  exportShortlistCsv: vi.fn(async () => undefined),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const drillClearTradeSelectionMock = vi.mocked(analysisService.drillClearTradeSelection)
const drillNextTradeMock = vi.mocked(analysisService.drillNextTrade)
const drillPrevTradeMock = vi.mocked(analysisService.drillPrevTrade)
const exportTopCsvMock = vi.mocked(analysisService.exportTopCsv)
const exportTopJsonMock = vi.mocked(analysisService.exportTopJson)
const browseCsvMock = vi.mocked(dataService.browseCsv)
const loadCsvMock = vi.mocked(dataService.loadCsv)
const openPolygonMock = vi.mocked(dataService.openPolygon)
const cancelOptimizationMock = vi.mocked(optimizeService.cancelOptimization)
const runOptimizationMock = vi.mocked(optimizeService.runOptimization)
const runValidationMock = vi.mocked(optimizeService.runValidation)
const exportShortlistCsvMock = vi.mocked(validationService.exportShortlistCsv)
const invokeCommandMock = vi.mocked(invokeCommand)

function ensureDocumentDataset(): Record<string, string> {
  const existing = (globalThis as { document?: { documentElement?: { dataset?: Record<string, string> } } }).document
  if (existing?.documentElement?.dataset) return existing.documentElement.dataset
  const dataset: Record<string, string> = {}
  vi.stubGlobal('document', { documentElement: { dataset } })
  return dataset
}

describe('command action store', () => {
  let pinia: ReturnType<typeof createPinia>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    vi.clearAllMocks()
    const dataset = ensureDocumentDataset()
    for (const key of Object.keys(dataset)) delete dataset[key]
  })

  it('dispatches run/cancel/validate actions', async () => {
    const store = useCommandActionStore()
    store.runOptimization()
    store.cancelOptimization()
    store.validateOptimization()

    await vi.dynamicImportSettled()
    await vi.waitFor(() => {
      expect(runOptimizationMock).toHaveBeenCalled()
      expect(cancelOptimizationMock).toHaveBeenCalled()
      expect(runValidationMock).toHaveBeenCalled()
    })
  })

  it('dispatches shared data/export/report actions', async () => {
    const dataset = ensureDocumentDataset()
    const store = useCommandActionStore()
    const dataStore = useDataStore()
    dataStore.csvPath = 'C:/tmp/aapl.csv'
    store.applyLegacyQuickActionsSnapshot({
      latestReportPath: 'C:/reports/latest.xlsx',
      latestMetaReportPath: 'C:/reports/meta_latest.xlsx',
    })
    const settingsStore = useSettingsStore()
    store.browseCsv()
    store.loadData()
    store.openPolygon()
    store.exportTopCsv()
    store.exportValidationShortlistCsv()
    store.exportTopJson()
    store.openLatestReport()
    store.openLatestMetaReport()
    store.toggleControls()

    await vi.dynamicImportSettled()
    await vi.waitFor(() => {
      expect(browseCsvMock).toHaveBeenCalled()
      expect(loadCsvMock).toHaveBeenCalled()
      expect(openPolygonMock).toHaveBeenCalled()
      expect(exportTopCsvMock).toHaveBeenCalled()
      expect(exportShortlistCsvMock).toHaveBeenCalled()
      expect(exportTopJsonMock).toHaveBeenCalled()
      expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/latest.xlsx' })
      expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/meta_latest.xlsx' })
      expect(settingsStore.controlsCollapsed).toBe(true)
      expect(dataset.controlsCollapsed).toBe('1')
      expect(store.canOpenLatestReport).toBe(true)
      expect(store.canOpenLatestMetaReport).toBe(true)
    })
  })

  it('falls back to workflow artifacts store for latest report path', () => {
    const store = useCommandActionStore()
    const artifactsStore = useWorkflowArtifactsStore()
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        report_xlsx_path: 'C:/reports/from_artifacts.xlsx',
        req: {},
        res: {},
      },
    })

    store.openLatestReport()

    expect(store.canOpenLatestReport).toBe(true)
    expect(store.source).toBe('vue-artifacts')
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/from_artifacts.xlsx' })
  })

  it('falls back to runtime dataset for latest meta report path when no legacy snapshot exists', () => {
    const dataset = ensureDocumentDataset()
    dataset.latestMetaReportPath = 'C:/reports/runtime_meta.xlsx'
    const store = useCommandActionStore()

    store.openLatestMetaReport()

    expect(store.canOpenLatestMetaReport).toBe(true)
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/runtime_meta.xlsx' })
  })

  it('prefers the runtime dataset latest meta report path over a stale legacy snapshot', () => {
    const dataset = ensureDocumentDataset()
    dataset.latestMetaReportPath = 'C:/reports/runtime_meta.xlsx'
    const store = useCommandActionStore()
    store.applyLegacyQuickActionsSnapshot({
      latestMetaReportPath: 'C:/reports/stale_meta.xlsx',
    })

    store.openLatestMetaReport()

    expect(store.canOpenLatestMetaReport).toBe(true)
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/runtime_meta.xlsx' })
  })

  it('prefers the local store report path over artifact and runtime fallback paths', () => {
    const dataset = ensureDocumentDataset()
    dataset.latestReportPath = 'C:/reports/runtime.xlsx'
    const artifactsStore = useWorkflowArtifactsStore()
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        report_xlsx_path: 'C:/reports/from_artifacts.xlsx',
        req: {},
        res: {},
      },
    })
    const store = useCommandActionStore()
    store.applyLegacyQuickActionsSnapshot({
      latestReportPath: 'C:/reports/from_store.xlsx',
    })

    store.openLatestReport()

    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/from_store.xlsx' })
    expect(store.source).toBe('legacy-event')
  })

  it('blocks latest report actions while runtime is marked as running', () => {
    const dataset = ensureDocumentDataset()
    dataset.running = '1'
    const store = useCommandActionStore()
    store.applyLegacyQuickActionsSnapshot({
      latestReportPath: 'C:/reports/latest.xlsx',
      latestMetaReportPath: 'C:/reports/meta_latest.xlsx',
    })

    store.openLatestReport()
    store.openLatestMetaReport()

    expect(invokeCommandMock).not.toHaveBeenCalled()
    expect(store.source).toBe('legacy-event')
  })

  it('falls back to reveal_path when opening the latest report path fails', async () => {
    const store = useCommandActionStore()
    invokeCommandMock.mockRejectedValueOnce(new Error('open failed'))
    store.applyLegacyQuickActionsSnapshot({
      latestReportPath: 'C:/reports/latest.xlsx',
    })

    store.openLatestReport()

    await vi.waitFor(() => {
      expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'open_path', { path: 'C:/reports/latest.xlsx' })
      expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'reveal_path', { path: 'C:/reports/latest.xlsx' })
    })
  })

  it('falls back to reveal_path when opening the latest meta report path fails', async () => {
    const store = useCommandActionStore()
    invokeCommandMock.mockRejectedValueOnce(new Error('open failed'))
    store.applyLegacyQuickActionsSnapshot({
      latestMetaReportPath: 'C:/reports/meta_latest.xlsx',
    })

    store.openLatestMetaReport()

    await vi.waitFor(() => {
      expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'open_path', { path: 'C:/reports/meta_latest.xlsx' })
      expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'reveal_path', { path: 'C:/reports/meta_latest.xlsx' })
    })
  })

  it('dispatches drill-trade navigation and clear actions', async () => {
    const store = useCommandActionStore()
    store.clearDrillTradeSelection()
    store.drillPrevTrade()
    store.drillNextTrade()

    await vi.dynamicImportSettled()
    await vi.waitFor(() => {
      expect(drillClearTradeSelectionMock).toHaveBeenCalled()
      expect(drillPrevTradeMock).toHaveBeenCalled()
      expect(drillNextTradeMock).toHaveBeenCalled()
    })
  })
})
