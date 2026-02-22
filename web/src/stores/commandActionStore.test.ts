import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyAnalysisDrillClearTradeSelectionRuntime,
  invokeLegacyAnalysisDrillNextTradeRuntime,
  invokeLegacyAnalysisDrillPrevTradeRuntime,
  invokeLegacyAnalysisExportTopCsvRuntime,
  invokeLegacyAnalysisExportTopJsonRuntime,
  invokeLegacyDataBrowseCsvRuntime,
  invokeLegacyDataLoadRuntime,
  invokeLegacyDataOpenPolygonRuntime,
  invokeLegacyOptimizeCancelRuntime,
  invokeLegacyOptimizeRunRuntime,
  invokeLegacyOptimizeValidateRuntime,
  invokeLegacyValidationExportShortlistCsvRuntime,
} from '../legacy/actions'
import { invokeCommand } from '../tauri/invoke'
import { useCommandActionStore } from './commandActionStore'
import { useSettingsStore } from './settingsStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyAnalysisDrillClearTradeSelectionRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisDrillNextTradeRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisDrillPrevTradeRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisExportTopCsvRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisExportTopJsonRuntime: vi.fn(async () => true),
  invokeLegacyDataBrowseCsvRuntime: vi.fn(async () => true),
  invokeLegacyDataLoadRuntime: vi.fn(async () => true),
  invokeLegacyDataOpenPolygonRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeRunRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeCancelRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeValidateRuntime: vi.fn(async () => true),
  invokeLegacyValidationExportShortlistCsvRuntime: vi.fn(async () => true),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeLegacyAnalysisDrillClearTradeSelectionRuntimeMock = vi.mocked(invokeLegacyAnalysisDrillClearTradeSelectionRuntime)
const invokeLegacyAnalysisDrillNextTradeRuntimeMock = vi.mocked(invokeLegacyAnalysisDrillNextTradeRuntime)
const invokeLegacyAnalysisDrillPrevTradeRuntimeMock = vi.mocked(invokeLegacyAnalysisDrillPrevTradeRuntime)
const invokeLegacyAnalysisExportTopCsvRuntimeMock = vi.mocked(invokeLegacyAnalysisExportTopCsvRuntime)
const invokeLegacyAnalysisExportTopJsonRuntimeMock = vi.mocked(invokeLegacyAnalysisExportTopJsonRuntime)
const invokeLegacyDataBrowseCsvRuntimeMock = vi.mocked(invokeLegacyDataBrowseCsvRuntime)
const invokeLegacyDataLoadRuntimeMock = vi.mocked(invokeLegacyDataLoadRuntime)
const invokeLegacyDataOpenPolygonRuntimeMock = vi.mocked(invokeLegacyDataOpenPolygonRuntime)
const invokeLegacyOptimizeRunRuntimeMock = vi.mocked(invokeLegacyOptimizeRunRuntime)
const invokeLegacyOptimizeCancelRuntimeMock = vi.mocked(invokeLegacyOptimizeCancelRuntime)
const invokeLegacyOptimizeValidateRuntimeMock = vi.mocked(invokeLegacyOptimizeValidateRuntime)
const invokeLegacyValidationExportShortlistCsvRuntimeMock = vi.mocked(invokeLegacyValidationExportShortlistCsvRuntime)
const invokeCommandMock = vi.mocked(invokeCommand)

function ensureDocumentDataset(): Record<string, string> {
  const existing = (globalThis as { document?: { documentElement?: { dataset?: Record<string, string> } } }).document
  if (existing?.documentElement?.dataset) return existing.documentElement.dataset
  const dataset: Record<string, string> = {}
  vi.stubGlobal('document', { documentElement: { dataset } })
  return dataset
}

describe('command action store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    invokeLegacyAnalysisDrillClearTradeSelectionRuntimeMock.mockClear()
    invokeLegacyAnalysisDrillClearTradeSelectionRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisDrillNextTradeRuntimeMock.mockClear()
    invokeLegacyAnalysisDrillNextTradeRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisDrillPrevTradeRuntimeMock.mockClear()
    invokeLegacyAnalysisDrillPrevTradeRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisExportTopCsvRuntimeMock.mockClear()
    invokeLegacyAnalysisExportTopCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisExportTopJsonRuntimeMock.mockClear()
    invokeLegacyAnalysisExportTopJsonRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataBrowseCsvRuntimeMock.mockClear()
    invokeLegacyDataBrowseCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataLoadRuntimeMock.mockClear()
    invokeLegacyDataLoadRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataOpenPolygonRuntimeMock.mockClear()
    invokeLegacyDataOpenPolygonRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeRunRuntimeMock.mockClear()
    invokeLegacyOptimizeRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeCancelRuntimeMock.mockClear()
    invokeLegacyOptimizeCancelRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeValidateRuntimeMock.mockClear()
    invokeLegacyOptimizeValidateRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationExportShortlistCsvRuntimeMock.mockClear()
    invokeLegacyValidationExportShortlistCsvRuntimeMock.mockResolvedValue(true)
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
    const dataset = ensureDocumentDataset()
    for (const key of Object.keys(dataset)) delete dataset[key]
  })

  it('dispatches run/cancel/validate actions', () => {
    const store = useCommandActionStore()
    store.runOptimization()
    store.cancelOptimization()
    store.validateOptimization()

    expect(invokeLegacyOptimizeRunRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizeCancelRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizeValidateRuntimeMock).toHaveBeenCalled()
  })

  it('dispatches shared data/export/report actions', () => {
    const dataset = ensureDocumentDataset()
    const store = useCommandActionStore()
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

    expect(invokeLegacyDataBrowseCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataLoadRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataOpenPolygonRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyAnalysisExportTopCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationExportShortlistCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyAnalysisExportTopJsonRuntimeMock).toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/latest.xlsx' })
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', { path: 'C:/reports/meta_latest.xlsx' })
    expect(settingsStore.controlsCollapsed).toBe(true)
    expect(dataset.controlsCollapsed).toBe('1')
    expect(store.canOpenLatestReport).toBe(true)
    expect(store.canOpenLatestMetaReport).toBe(true)
  })

  it('dispatches drill-trade navigation and clear actions', () => {
    const store = useCommandActionStore()
    store.clearDrillTradeSelection()
    store.drillPrevTrade()
    store.drillNextTrade()

    expect(invokeLegacyAnalysisDrillClearTradeSelectionRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyAnalysisDrillPrevTradeRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyAnalysisDrillNextTradeRuntimeMock).toHaveBeenCalled()
  })

})
