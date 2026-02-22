import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyAnalysisClearBaselineRuntime,
  invokeLegacyAnalysisClearTopFiltersRuntime,
  invokeLegacyAnalysisCopyBestRuntime,
  invokeLegacyAnalysisExportTopCsvRuntime,
  invokeLegacyAnalysisExportTopJsonRuntime,
  invokeLegacyAnalysisHeatmapExportPngRuntime,
  invokeLegacyAnalysisRerenderDrillRuntime,
  invokeLegacyAnalysisSelectTopRowRuntime,
  invokeLegacyAnalysisSetBaselineRuntime,
  invokeLegacyAnalysisSortTopTableRuntime,
  invokeLegacyDataBrowseCsvRuntime,
  invokeLegacyDataOpenPolygonRuntime,
  invokeLegacyMetaBatchRefreshRuntime,
  invokeLegacyMetaBatchRevealSummaryRuntime,
  invokeLegacyMetaBatchRetryFailedRuntime,
  invokeLegacyMetaBatchSelectAllRuntime,
  invokeLegacyMetaBatchSelectNoneRuntime,
  invokeLegacyMetaBatchStopRuntime,
  invokeLegacyMetaDeleteSavedRuntime,
  invokeLegacyMetaDeleteStage4SavedRuntime,
  invokeLegacyMetaLoadSavedRuntime,
  invokeLegacyMetaLoadStage4SavedRuntime,
  invokeLegacyMetaMlApplyRuntime,
  invokeLegacyMetaMlDeleteRuntime,
  invokeLegacyMetaMlRefreshRuntime,
  invokeLegacyMetaMlTrainRuntime,
  invokeLegacyMetaOpenReportRuntime,
  invokeLegacyMetaOpenSavedReportRuntime,
  invokeLegacyMetaRefreshSavedRuntime,
  invokeLegacyMetaRefreshStage4SavedRuntime,
  invokeLegacyMetaRevealFilesRuntime,
  invokeLegacyMetaRevealSavedRuntime,
  invokeLegacyMetaRevealStage4Runtime,
  invokeLegacyMetaRevealStage4SavedRuntime,
  invokeLegacyMetaRunBatchRuntime,
  invokeLegacyMetaRunRuntime,
  invokeLegacyMetaRunStage4Runtime,
  invokeLegacyMetaSaveSavedTagsRuntime,
  invokeLegacyMetaSaveStage4SavedTagsRuntime,
  invokeLegacyMetaToggleBatchDatasetRuntime,
  invokeLegacyOptimizeCancelRuntime,
  invokeLegacyOptimizeRunRuntime,
} from '../legacy/actions'
import { useAnalysisStore } from './analysisStore'
import { useWorkflowStore } from './workflowStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyAnalysisClearBaselineRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisClearTopFiltersRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisCopyBestRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisExportTopCsvRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisExportTopJsonRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisHeatmapExportPngRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisRerenderDrillRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisSelectTopRowRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisSetBaselineRuntime: vi.fn(async () => true),
  invokeLegacyAnalysisSortTopTableRuntime: vi.fn(async () => true),
  invokeLegacyDataBrowseCsvRuntime: vi.fn(async () => true),
  invokeLegacyDataOpenPolygonRuntime: vi.fn(async () => true),
  invokeLegacyMetaBatchRefreshRuntime: vi.fn(async () => true),
  invokeLegacyMetaBatchRevealSummaryRuntime: vi.fn(async () => true),
  invokeLegacyMetaBatchRetryFailedRuntime: vi.fn(async () => true),
  invokeLegacyMetaBatchSelectAllRuntime: vi.fn(async () => true),
  invokeLegacyMetaBatchSelectNoneRuntime: vi.fn(async () => true),
  invokeLegacyMetaBatchStopRuntime: vi.fn(async () => true),
  invokeLegacyMetaDeleteSavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaDeleteStage4SavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaLoadSavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaLoadStage4SavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaMlApplyRuntime: vi.fn(async () => true),
  invokeLegacyMetaMlDeleteRuntime: vi.fn(async () => true),
  invokeLegacyMetaMlRefreshRuntime: vi.fn(async () => true),
  invokeLegacyMetaMlTrainRuntime: vi.fn(async () => true),
  invokeLegacyMetaOpenReportRuntime: vi.fn(async () => true),
  invokeLegacyMetaOpenSavedReportRuntime: vi.fn(async () => true),
  invokeLegacyMetaRefreshSavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaRefreshStage4SavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaRevealFilesRuntime: vi.fn(async () => true),
  invokeLegacyMetaRevealSavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaRevealStage4Runtime: vi.fn(async () => true),
  invokeLegacyMetaRevealStage4SavedRuntime: vi.fn(async () => true),
  invokeLegacyMetaRunBatchRuntime: vi.fn(async () => true),
  invokeLegacyMetaRunRuntime: vi.fn(async () => true),
  invokeLegacyMetaRunStage4Runtime: vi.fn(async () => true),
  invokeLegacyMetaSaveSavedTagsRuntime: vi.fn(async () => true),
  invokeLegacyMetaSaveStage4SavedTagsRuntime: vi.fn(async () => true),
  invokeLegacyMetaToggleBatchDatasetRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeRunRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeCancelRuntime: vi.fn(async () => true),
}))

const invokeLegacyAnalysisClearBaselineRuntimeMock = vi.mocked(invokeLegacyAnalysisClearBaselineRuntime)
const invokeLegacyAnalysisClearTopFiltersRuntimeMock = vi.mocked(invokeLegacyAnalysisClearTopFiltersRuntime)
const invokeLegacyAnalysisCopyBestRuntimeMock = vi.mocked(invokeLegacyAnalysisCopyBestRuntime)
const invokeLegacyAnalysisExportTopCsvRuntimeMock = vi.mocked(invokeLegacyAnalysisExportTopCsvRuntime)
const invokeLegacyAnalysisExportTopJsonRuntimeMock = vi.mocked(invokeLegacyAnalysisExportTopJsonRuntime)
const invokeLegacyAnalysisHeatmapExportPngRuntimeMock = vi.mocked(invokeLegacyAnalysisHeatmapExportPngRuntime)
const invokeLegacyAnalysisRerenderDrillRuntimeMock = vi.mocked(invokeLegacyAnalysisRerenderDrillRuntime)
const invokeLegacyAnalysisSelectTopRowRuntimeMock = vi.mocked(invokeLegacyAnalysisSelectTopRowRuntime)
const invokeLegacyAnalysisSetBaselineRuntimeMock = vi.mocked(invokeLegacyAnalysisSetBaselineRuntime)
const invokeLegacyAnalysisSortTopTableRuntimeMock = vi.mocked(invokeLegacyAnalysisSortTopTableRuntime)
const invokeLegacyDataBrowseCsvRuntimeMock = vi.mocked(invokeLegacyDataBrowseCsvRuntime)
const invokeLegacyDataOpenPolygonRuntimeMock = vi.mocked(invokeLegacyDataOpenPolygonRuntime)
const invokeLegacyMetaBatchRefreshRuntimeMock = vi.mocked(invokeLegacyMetaBatchRefreshRuntime)
const invokeLegacyMetaBatchRevealSummaryRuntimeMock = vi.mocked(invokeLegacyMetaBatchRevealSummaryRuntime)
const invokeLegacyMetaBatchRetryFailedRuntimeMock = vi.mocked(invokeLegacyMetaBatchRetryFailedRuntime)
const invokeLegacyMetaBatchSelectAllRuntimeMock = vi.mocked(invokeLegacyMetaBatchSelectAllRuntime)
const invokeLegacyMetaBatchSelectNoneRuntimeMock = vi.mocked(invokeLegacyMetaBatchSelectNoneRuntime)
const invokeLegacyMetaBatchStopRuntimeMock = vi.mocked(invokeLegacyMetaBatchStopRuntime)
const invokeLegacyMetaDeleteSavedRuntimeMock = vi.mocked(invokeLegacyMetaDeleteSavedRuntime)
const invokeLegacyMetaDeleteStage4SavedRuntimeMock = vi.mocked(invokeLegacyMetaDeleteStage4SavedRuntime)
const invokeLegacyMetaLoadSavedRuntimeMock = vi.mocked(invokeLegacyMetaLoadSavedRuntime)
const invokeLegacyMetaLoadStage4SavedRuntimeMock = vi.mocked(invokeLegacyMetaLoadStage4SavedRuntime)
const invokeLegacyMetaMlApplyRuntimeMock = vi.mocked(invokeLegacyMetaMlApplyRuntime)
const invokeLegacyMetaMlDeleteRuntimeMock = vi.mocked(invokeLegacyMetaMlDeleteRuntime)
const invokeLegacyMetaMlRefreshRuntimeMock = vi.mocked(invokeLegacyMetaMlRefreshRuntime)
const invokeLegacyMetaMlTrainRuntimeMock = vi.mocked(invokeLegacyMetaMlTrainRuntime)
const invokeLegacyMetaOpenReportRuntimeMock = vi.mocked(invokeLegacyMetaOpenReportRuntime)
const invokeLegacyMetaOpenSavedReportRuntimeMock = vi.mocked(invokeLegacyMetaOpenSavedReportRuntime)
const invokeLegacyMetaRefreshSavedRuntimeMock = vi.mocked(invokeLegacyMetaRefreshSavedRuntime)
const invokeLegacyMetaRefreshStage4SavedRuntimeMock = vi.mocked(invokeLegacyMetaRefreshStage4SavedRuntime)
const invokeLegacyMetaRevealFilesRuntimeMock = vi.mocked(invokeLegacyMetaRevealFilesRuntime)
const invokeLegacyMetaRevealSavedRuntimeMock = vi.mocked(invokeLegacyMetaRevealSavedRuntime)
const invokeLegacyMetaRevealStage4RuntimeMock = vi.mocked(invokeLegacyMetaRevealStage4Runtime)
const invokeLegacyMetaRevealStage4SavedRuntimeMock = vi.mocked(invokeLegacyMetaRevealStage4SavedRuntime)
const invokeLegacyMetaRunBatchRuntimeMock = vi.mocked(invokeLegacyMetaRunBatchRuntime)
const invokeLegacyMetaRunRuntimeMock = vi.mocked(invokeLegacyMetaRunRuntime)
const invokeLegacyMetaRunStage4RuntimeMock = vi.mocked(invokeLegacyMetaRunStage4Runtime)
const invokeLegacyMetaSaveSavedTagsRuntimeMock = vi.mocked(invokeLegacyMetaSaveSavedTagsRuntime)
const invokeLegacyMetaSaveStage4SavedTagsRuntimeMock = vi.mocked(invokeLegacyMetaSaveStage4SavedTagsRuntime)
const invokeLegacyMetaToggleBatchDatasetRuntimeMock = vi.mocked(invokeLegacyMetaToggleBatchDatasetRuntime)
const invokeLegacyOptimizeRunRuntimeMock = vi.mocked(invokeLegacyOptimizeRunRuntime)
const invokeLegacyOptimizeCancelRuntimeMock = vi.mocked(invokeLegacyOptimizeCancelRuntime)

interface LegacyTextField {
  value: string
  disabled?: boolean
  type?: string
  dispatchEvent: (event: Event) => boolean
}

interface LegacyCheckboxField {
  checked: boolean
  disabled?: boolean
  type: 'checkbox'
  dispatchEvent: (event: Event) => boolean
}

interface DocumentHarness {
  fields: Record<string, LegacyTextField | LegacyCheckboxField>
  eventsById: Record<string, string[]>
}

function installDocumentHarness(): DocumentHarness {
  const eventsById: Record<string, string[]> = {}
  const trackEvent = (id: string, event: Event): boolean => {
    if (!eventsById[id]) eventsById[id] = []
    eventsById[id].push(String(event.type || '').trim())
    return true
  }
  const fields: Record<string, LegacyTextField | LegacyCheckboxField> = {
    filterMinPnl: {
      value: '',
      dispatchEvent: (event: Event) => trackEvent('filterMinPnl', event),
    },
    compareSelect: {
      value: '',
      dispatchEvent: (event: Event) => trackEvent('compareSelect', event),
    },
    metaTopK: {
      value: '25',
      dispatchEvent: (event: Event) => trackEvent('metaTopK', event),
    },
    metaStrategyKind: {
      value: 'DoubleMa',
      dispatchEvent: (event: Event) => trackEvent('metaStrategyKind', event),
    },
    metaEvalHoldout: {
      checked: false,
      type: 'checkbox',
      dispatchEvent: (event: Event) => trackEvent('metaEvalHoldout', event),
    },
  }
  vi.stubGlobal('document', {
    documentElement: { dataset: {} },
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('analysis store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyAnalysisClearBaselineRuntimeMock.mockClear()
    invokeLegacyAnalysisClearBaselineRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisClearTopFiltersRuntimeMock.mockClear()
    invokeLegacyAnalysisClearTopFiltersRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisCopyBestRuntimeMock.mockClear()
    invokeLegacyAnalysisCopyBestRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisExportTopCsvRuntimeMock.mockClear()
    invokeLegacyAnalysisExportTopCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisExportTopJsonRuntimeMock.mockClear()
    invokeLegacyAnalysisExportTopJsonRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisHeatmapExportPngRuntimeMock.mockClear()
    invokeLegacyAnalysisHeatmapExportPngRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisRerenderDrillRuntimeMock.mockClear()
    invokeLegacyAnalysisRerenderDrillRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisSelectTopRowRuntimeMock.mockClear()
    invokeLegacyAnalysisSelectTopRowRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisSetBaselineRuntimeMock.mockClear()
    invokeLegacyAnalysisSetBaselineRuntimeMock.mockResolvedValue(true)
    invokeLegacyAnalysisSortTopTableRuntimeMock.mockClear()
    invokeLegacyAnalysisSortTopTableRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataBrowseCsvRuntimeMock.mockClear()
    invokeLegacyDataBrowseCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataOpenPolygonRuntimeMock.mockClear()
    invokeLegacyDataOpenPolygonRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaBatchRefreshRuntimeMock.mockClear()
    invokeLegacyMetaBatchRefreshRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaBatchRevealSummaryRuntimeMock.mockClear()
    invokeLegacyMetaBatchRevealSummaryRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaBatchRetryFailedRuntimeMock.mockClear()
    invokeLegacyMetaBatchRetryFailedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaBatchSelectAllRuntimeMock.mockClear()
    invokeLegacyMetaBatchSelectAllRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaBatchSelectNoneRuntimeMock.mockClear()
    invokeLegacyMetaBatchSelectNoneRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaBatchStopRuntimeMock.mockClear()
    invokeLegacyMetaBatchStopRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaDeleteSavedRuntimeMock.mockClear()
    invokeLegacyMetaDeleteSavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaDeleteStage4SavedRuntimeMock.mockClear()
    invokeLegacyMetaDeleteStage4SavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaLoadSavedRuntimeMock.mockClear()
    invokeLegacyMetaLoadSavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaLoadStage4SavedRuntimeMock.mockClear()
    invokeLegacyMetaLoadStage4SavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaMlApplyRuntimeMock.mockClear()
    invokeLegacyMetaMlApplyRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaMlDeleteRuntimeMock.mockClear()
    invokeLegacyMetaMlDeleteRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaMlRefreshRuntimeMock.mockClear()
    invokeLegacyMetaMlRefreshRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaMlTrainRuntimeMock.mockClear()
    invokeLegacyMetaMlTrainRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaOpenReportRuntimeMock.mockClear()
    invokeLegacyMetaOpenReportRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaOpenSavedReportRuntimeMock.mockClear()
    invokeLegacyMetaOpenSavedReportRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRefreshSavedRuntimeMock.mockClear()
    invokeLegacyMetaRefreshSavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRefreshStage4SavedRuntimeMock.mockClear()
    invokeLegacyMetaRefreshStage4SavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRevealFilesRuntimeMock.mockClear()
    invokeLegacyMetaRevealFilesRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRevealSavedRuntimeMock.mockClear()
    invokeLegacyMetaRevealSavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRevealStage4RuntimeMock.mockClear()
    invokeLegacyMetaRevealStage4RuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRevealStage4SavedRuntimeMock.mockClear()
    invokeLegacyMetaRevealStage4SavedRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRunBatchRuntimeMock.mockClear()
    invokeLegacyMetaRunBatchRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRunRuntimeMock.mockClear()
    invokeLegacyMetaRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaRunStage4RuntimeMock.mockClear()
    invokeLegacyMetaRunStage4RuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaSaveSavedTagsRuntimeMock.mockClear()
    invokeLegacyMetaSaveSavedTagsRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaSaveStage4SavedTagsRuntimeMock.mockClear()
    invokeLegacyMetaSaveStage4SavedTagsRuntimeMock.mockResolvedValue(true)
    invokeLegacyMetaToggleBatchDatasetRuntimeMock.mockClear()
    invokeLegacyMetaToggleBatchDatasetRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeRunRuntimeMock.mockClear()
    invokeLegacyOptimizeRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeCancelRuntimeMock.mockClear()
    invokeLegacyOptimizeCancelRuntimeMock.mockResolvedValue(true)
  })

  it('dispatches top-table field updates via legacy DOM events', () => {
    const store = useAnalysisStore()
    store.setInputField('filterMinPnl', '42')
    store.setSelectField('compareSelect', 'run_01')

    expect((harness.fields.filterMinPnl as LegacyTextField).value).toBe('42')
    expect((harness.fields.compareSelect as LegacyTextField).value).toBe('run_01')
    expect(harness.eventsById.filterMinPnl).toEqual(['input'])
    expect(harness.eventsById.compareSelect).toEqual(['change'])
  })

  it('dispatches top-table selection actions', () => {
    const store = useAnalysisStore()
    store.clearTopFilters()
    store.sortTopTable('pnl')
    store.selectTopRow(3)

    expect(invokeLegacyAnalysisClearTopFiltersRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyAnalysisSortTopTableRuntimeMock).toHaveBeenCalledWith({ key: 'pnl' })
    expect(invokeLegacyAnalysisSelectTopRowRuntimeMock).toHaveBeenCalledWith({ index: 3 })
  })

  it('dispatches overview export and baseline actions', () => {
    const store = useAnalysisStore()
    store.copyBest()
    store.exportTopCsv()
    store.exportTopJson()
    store.setBaseline()
    store.clearBaseline()

    expect(invokeLegacyAnalysisCopyBestRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyAnalysisExportTopCsvRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyAnalysisExportTopJsonRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyAnalysisSetBaselineRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyAnalysisClearBaselineRuntimeMock).toHaveBeenCalledTimes(1)
  })

  it('routes overview browse and polygon actions through workflow + data stores', () => {
    const workflowStore = useWorkflowStore()
    const workspaceSpy = vi.spyOn(workflowStore, 'selectWorkspace')
    const controlsSpy = vi.spyOn(workflowStore, 'selectControlsTab')
    const store = useAnalysisStore()
    store.overviewPrimaryAction = 'browse_csv'
    store.overviewSecondaryAction = 'polygon'

    store.overviewEmptyPrimary()
    store.overviewEmptySecondary()

    expect(workspaceSpy).toHaveBeenCalledWith('workflow', 'data')
    expect(controlsSpy).not.toHaveBeenCalled()
    expect(invokeLegacyDataBrowseCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataOpenPolygonRuntimeMock).toHaveBeenCalled()
  })

  it('routes overview optimize actions through workflow + optimize stores', () => {
    const workflowStore = useWorkflowStore()
    const workspaceSpy = vi.spyOn(workflowStore, 'selectWorkspace')
    const controlsSpy = vi.spyOn(workflowStore, 'selectControlsTab')
    const store = useAnalysisStore()

    store.overviewPrimaryAction = 'go_opt'
    store.overviewEmptyPrimary()
    expect(workspaceSpy).toHaveBeenCalledWith('workflow', 'optimize')
    expect(controlsSpy).toHaveBeenCalledWith('opt')

    store.overviewSecondaryAction = 'run_opt'
    store.overviewEmptySecondary()
    expect(invokeLegacyOptimizeRunRuntimeMock).toHaveBeenCalled()

    store.overviewSecondaryAction = 'cancel_run'
    store.overviewEmptySecondary()
    expect(invokeLegacyOptimizeCancelRuntimeMock).toHaveBeenCalled()
  })

  it('dispatches heatmap export action', () => {
    const store = useAnalysisStore()
    store.heatmapExportPng()
    expect(invokeLegacyAnalysisHeatmapExportPngRuntimeMock).toHaveBeenCalledTimes(1)
  })

  it('dispatches drill rerender action', () => {
    const store = useAnalysisStore()
    store.rerenderDrill()
    expect(invokeLegacyAnalysisRerenderDrillRuntimeMock).toHaveBeenCalledTimes(1)
  })

  it('dispatches meta field updates via legacy DOM events', () => {
    const store = useAnalysisStore()
    store.setMetaInputField('metaTopK', '20')
    store.setMetaSelectField('metaStrategyKind', 'CompositeMo')
    store.setMetaCheckboxField('metaEvalHoldout', true)

    const metaEvalHoldout = harness.fields.metaEvalHoldout as LegacyCheckboxField
    expect((harness.fields.metaTopK as LegacyTextField).value).toBe('20')
    expect((harness.fields.metaStrategyKind as LegacyTextField).value).toBe('CompositeMo')
    expect(metaEvalHoldout.checked).toBe(true)
    expect(harness.eventsById.metaTopK).toEqual(['input'])
    expect(harness.eventsById.metaStrategyKind).toEqual(['change'])
    expect(harness.eventsById.metaEvalHoldout).toEqual(['change'])
  })

  it('dispatches meta action groups', () => {
    const store = useAnalysisStore()
    store.runBatch()
    store.batchStop()
    store.batchRefresh()
    store.batchSelectAll()
    store.batchSelectNone()
    store.batchRetryFailed()
    store.batchRevealSummary()
    store.saveSavedTags()
    store.refreshSaved()
    store.loadSaved()
    store.openSavedReport()
    store.revealSaved()
    store.deleteSaved()
    store.saveStage4SavedTags()
    store.refreshStage4Saved()
    store.loadStage4Saved()
    store.revealStage4Saved()
    store.deleteStage4Saved()
    store.mlRefresh()
    store.mlTrain()
    store.mlApply()
    store.mlDelete()
    store.runStage4()
    store.revealStage4()
    store.runMeta()
    store.openMetaReport()
    store.revealMetaFiles()
    store.toggleBatchDataset(' C:/data.csv ', true)

    expect(invokeLegacyMetaRunBatchRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaBatchStopRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaBatchRefreshRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaBatchSelectAllRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaBatchSelectNoneRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaBatchRetryFailedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaBatchRevealSummaryRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaSaveSavedTagsRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRefreshSavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaLoadSavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaOpenSavedReportRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRevealSavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaDeleteSavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaSaveStage4SavedTagsRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRefreshStage4SavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaLoadStage4SavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRevealStage4SavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaDeleteStage4SavedRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaMlRefreshRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaMlTrainRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaMlApplyRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaMlDeleteRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRunStage4RuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRevealStage4RuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRunRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaOpenReportRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaRevealFilesRuntimeMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyMetaToggleBatchDatasetRuntimeMock).toHaveBeenCalledWith({
      path: 'C:/data.csv',
      checked: true,
    })
  })
})
