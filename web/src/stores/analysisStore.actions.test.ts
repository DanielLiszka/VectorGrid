import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import * as analysisService from '../services/analysisService'
import * as dataService from '../services/dataService'
import * as metaService from '../services/metaService'
import * as optimizeService from '../services/optimizeService'
import { bindRuntimePinia } from './runtimePinia'
import { useAnalysisStore } from './analysisStore'
import { useHistoryStore } from './historyStore'
import { useWorkflowStore } from './workflowStore'

vi.mock('../services/analysisService', () => ({
  clearBaseline: vi.fn(async () => undefined),
  clearPinnedTopCandidates: vi.fn(async () => undefined),
  clearTopFilters: vi.fn(async () => undefined),
  copyBest: vi.fn(async () => undefined),
  drillClearTradeSelection: vi.fn(async () => undefined),
  drillNextTrade: vi.fn(async () => undefined),
  drillPrevTrade: vi.fn(async () => undefined),
  exportTopCsv: vi.fn(async () => undefined),
  exportTopJson: vi.fn(async () => undefined),
  heatmapExportPng: vi.fn(async () => undefined),
  pinSelectedTopRow: vi.fn(async () => undefined),
  pinTopRow: vi.fn(async () => undefined),
  refreshHeatmapFromWorkflowRun: vi.fn(() => true),
  refreshParetoFromWorkflowRun: vi.fn(() => true),
  rerenderDrill: vi.fn(async () => undefined),
  refreshCompareSummary: vi.fn(async () => undefined),
  refreshTopTableFromWorkflowRun: vi.fn(() => true),
  replacePinnedTopCandidate: vi.fn(async () => undefined),
  selectTopRow: vi.fn(async () => undefined),
  setBaseline: vi.fn(async () => undefined),
  sortTopTable: vi.fn(async () => undefined),
  unpinTopCandidate: vi.fn(async () => undefined),
}))

vi.mock('../services/metaService', () => ({
  batchRefresh: vi.fn(async () => undefined),
  batchRevealSummary: vi.fn(async () => undefined),
  batchRetryFailed: vi.fn(async () => undefined),
  batchSelectAll: vi.fn(async () => undefined),
  batchSelectNone: vi.fn(async () => undefined),
  batchStop: vi.fn(async () => undefined),
  deleteSaved: vi.fn(async () => undefined),
  deleteStage4Saved: vi.fn(async () => undefined),
  deleteStage5Saved: vi.fn(async () => undefined),
  loadSaved: vi.fn(async () => undefined),
  loadStage4Saved: vi.fn(async () => undefined),
  loadStage5Saved: vi.fn(async () => undefined),
  mlApply: vi.fn(async () => undefined),
  mlDelete: vi.fn(async () => undefined),
  mlRefresh: vi.fn(async () => undefined),
  mlTrain: vi.fn(async () => undefined),
  openMetaReport: vi.fn(async () => undefined),
  openSavedReport: vi.fn(async () => undefined),
  refreshSaved: vi.fn(async () => undefined),
  refreshStage4Saved: vi.fn(async () => undefined),
  refreshStage5Saved: vi.fn(async () => undefined),
  revealMetaFiles: vi.fn(async () => undefined),
  revealSaved: vi.fn(async () => undefined),
  revealStage4: vi.fn(async () => undefined),
  revealStage5: vi.fn(async () => undefined),
  revealStage4Saved: vi.fn(async () => undefined),
  revealStage5Saved: vi.fn(async () => undefined),
  runBatch: vi.fn(async () => undefined),
  runMeta: vi.fn(async () => undefined),
  runStage4: vi.fn(async () => undefined),
  runStage5: vi.fn(async () => undefined),
  refreshBatchDatasetView: vi.fn(() => undefined),
  saveSavedTags: vi.fn(async () => undefined),
  saveStage4SavedTags: vi.fn(async () => undefined),
  saveStage5SavedTags: vi.fn(async () => undefined),
  toggleBatchDataset: vi.fn(async () => undefined),
}))

vi.mock('../services/dataService', () => ({
  browseCsv: vi.fn(async () => null),
  getPolygonCacheInfo: vi.fn(async () => ({ cacheDir: 'C:/polygon', hasApiKey: false, datasets: [] })),
  openPolygon: vi.fn(async () => undefined),
}))

vi.mock('../services/optimizeService', () => ({
  cancelOptimization: vi.fn(async () => undefined),
  runOptimization: vi.fn(async () => undefined),
}))

const clearBaselineMock = vi.mocked(analysisService.clearBaseline)
const clearPinnedTopCandidatesMock = vi.mocked(analysisService.clearPinnedTopCandidates)
const clearTopFiltersMock = vi.mocked(analysisService.clearTopFilters)
const copyBestMock = vi.mocked(analysisService.copyBest)
const drillClearTradeSelectionMock = vi.mocked(analysisService.drillClearTradeSelection)
const drillNextTradeMock = vi.mocked(analysisService.drillNextTrade)
const drillPrevTradeMock = vi.mocked(analysisService.drillPrevTrade)
const exportTopCsvMock = vi.mocked(analysisService.exportTopCsv)
const exportTopJsonMock = vi.mocked(analysisService.exportTopJson)
const heatmapExportPngMock = vi.mocked(analysisService.heatmapExportPng)
const pinSelectedTopRowMock = vi.mocked(analysisService.pinSelectedTopRow)
const pinTopRowMock = vi.mocked(analysisService.pinTopRow)
const refreshHeatmapFromWorkflowRunMock = vi.mocked(analysisService.refreshHeatmapFromWorkflowRun)
const refreshParetoFromWorkflowRunMock = vi.mocked(analysisService.refreshParetoFromWorkflowRun)
const rerenderDrillMock = vi.mocked(analysisService.rerenderDrill)
const refreshCompareSummaryMock = vi.mocked(analysisService.refreshCompareSummary)
const refreshTopTableFromWorkflowRunMock = vi.mocked(analysisService.refreshTopTableFromWorkflowRun)
const replacePinnedTopCandidateMock = vi.mocked(analysisService.replacePinnedTopCandidate)
const selectTopRowMock = vi.mocked(analysisService.selectTopRow)
const setBaselineMock = vi.mocked(analysisService.setBaseline)
const sortTopTableMock = vi.mocked(analysisService.sortTopTable)
const unpinTopCandidateMock = vi.mocked(analysisService.unpinTopCandidate)

const batchRefreshMock = vi.mocked(metaService.batchRefresh)
const batchRevealSummaryMock = vi.mocked(metaService.batchRevealSummary)
const batchRetryFailedMock = vi.mocked(metaService.batchRetryFailed)
const batchSelectAllMock = vi.mocked(metaService.batchSelectAll)
const batchSelectNoneMock = vi.mocked(metaService.batchSelectNone)
const batchStopMock = vi.mocked(metaService.batchStop)
const deleteSavedMock = vi.mocked(metaService.deleteSaved)
const deleteStage4SavedMock = vi.mocked(metaService.deleteStage4Saved)
const deleteStage5SavedMock = vi.mocked(metaService.deleteStage5Saved)
const loadSavedMock = vi.mocked(metaService.loadSaved)
const loadStage4SavedMock = vi.mocked(metaService.loadStage4Saved)
const loadStage5SavedMock = vi.mocked(metaService.loadStage5Saved)
const mlApplyMock = vi.mocked(metaService.mlApply)
const mlDeleteMock = vi.mocked(metaService.mlDelete)
const mlRefreshMock = vi.mocked(metaService.mlRefresh)
const mlTrainMock = vi.mocked(metaService.mlTrain)
const openMetaReportMock = vi.mocked(metaService.openMetaReport)
const openSavedReportMock = vi.mocked(metaService.openSavedReport)
const refreshSavedMock = vi.mocked(metaService.refreshSaved)
const refreshStage4SavedMock = vi.mocked(metaService.refreshStage4Saved)
const refreshStage5SavedMock = vi.mocked(metaService.refreshStage5Saved)
const revealMetaFilesMock = vi.mocked(metaService.revealMetaFiles)
const revealSavedMock = vi.mocked(metaService.revealSaved)
const revealStage4Mock = vi.mocked(metaService.revealStage4)
const revealStage5Mock = vi.mocked(metaService.revealStage5)
const revealStage4SavedMock = vi.mocked(metaService.revealStage4Saved)
const revealStage5SavedMock = vi.mocked(metaService.revealStage5Saved)
const runBatchMock = vi.mocked(metaService.runBatch)
const runMetaMock = vi.mocked(metaService.runMeta)
const runStage4Mock = vi.mocked(metaService.runStage4)
const runStage5Mock = vi.mocked(metaService.runStage5)
const saveSavedTagsMock = vi.mocked(metaService.saveSavedTags)
const saveStage4SavedTagsMock = vi.mocked(metaService.saveStage4SavedTags)
const saveStage5SavedTagsMock = vi.mocked(metaService.saveStage5SavedTags)
const toggleBatchDatasetMock = vi.mocked(metaService.toggleBatchDataset)

const browseCsvMock = vi.mocked(dataService.browseCsv)
const openPolygonMock = vi.mocked(dataService.openPolygon)
const runOptimizationMock = vi.mocked(optimizeService.runOptimization)
const cancelOptimizationMock = vi.mocked(optimizeService.cancelOptimization)

describe('analysis store actions', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    vi.clearAllMocks()
  })

  it('updates analysis/history controls and keeps Vue store state in sync', async () => {
    const store = useAnalysisStore()
    const historyStore = useHistoryStore()

    store.setInputField('filterMinPnl', '1.5')
    store.setInputField('filterMinSharpe', ' 0.9 ')
    store.setSelectField('paretoX', 'sharpe')
    store.setSelectField('heatmapRange', 'q1_99')
    store.setSelectField('compareSelect', 'latest')
    store.setMetaInputField('metaTopK', ' 40 ')
    store.setMetaSelectField('metaStrategyKind', 'MeanReversion')
    store.setMetaSelectField('metaCompareSelect', 'baseline-1')
    store.setMetaInputField('batchDatasetSearch', 'btc')
    store.setMetaInputField('metaSavedSearch', 'saved-foo')
    store.setMetaSelectField('metaSavedSelect', 'meta-saved-1')
    store.setMetaInputField('metaSavedTags', 'alpha,beta')
    store.setMetaInputField('stage4SavedSearch', 'stage4-foo')
    store.setMetaSelectField('stage4SavedSelect', 'stage4-saved-1')
    store.setMetaInputField('stage4SavedTags', 'gamma,delta')
    store.setMetaCheckboxField('metaEvalHoldout', true)
    store.setMetaCheckboxField('metaUseMlConfidence', true)
    store.setMetaCheckboxField('metaMlAutopilot', false)

    await vi.waitFor(() => {
      expect(refreshTopTableFromWorkflowRunMock).toHaveBeenCalledTimes(2)
      expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalledTimes(1)
      expect(refreshParetoFromWorkflowRunMock).toHaveBeenCalledTimes(1)
      expect(refreshCompareSummaryMock).toHaveBeenCalledTimes(1)
    })

    expect(store.filterMinPnl).toBe('1.5')
    expect(store.filterMinSharpe).toBe('0.9')
    expect(store.paretoX).toBe('sharpe')
    expect(store.heatmapRange).toBe('q1_99')
    expect(historyStore.compareSelectedValue).toBe('latest')
    expect(store.compareSummary).toBe('Baseline selected: latest')
    expect(store.metaTopK).toBe('40')
    expect(store.metaStrategyKind).toBe('MeanReversion')
    expect(store.metaCompareSelected).toBe('baseline-1')
    expect(store.batchDatasetSearch).toBe('btc')
    expect(store.metaSavedSearch).toBe('saved-foo')
    expect(store.metaSavedSelected).toBe('meta-saved-1')
    expect(store.metaSavedTagsDraft).toBe('alpha,beta')
    expect(store.stage4SavedSearch).toBe('stage4-foo')
    expect(store.stage4SavedSelected).toBe('stage4-saved-1')
    expect(store.stage4SavedTagsDraft).toBe('gamma,delta')
    expect(store.metaEvalHoldout).toBe(true)
    expect(store.metaUseMlConfidence).toBe(true)
    expect(store.metaMlAutopilot).toBe(false)
    expect(store.source).toBe('vue')
    expect(store.updatedAtUnixMs).toBeGreaterThan(0)
  })

  it('routes core analysis actions through analysisService', async () => {
    const store = useAnalysisStore()

    store.clearTopFilters()
    store.sortTopTable(' sharpe ')
    store.selectTopRow(3.9)
    store.pinTopRow(1.2)
    store.pinSelectedTopRow()
    store.replacePinnedTopCandidate(' pinned_a ')
    store.unpinTopCandidate(' pinned_b ')
    store.clearPinnedTopCandidates()
    store.copyBest()
    store.exportTopCsv()
    store.exportTopJson()
    store.setBaseline()
    store.clearBaseline()
    store.heatmapExportPng()
    store.rerenderDrill()
    store.drillClearTradeSelection()
    store.drillPrevTrade()
    store.drillNextTrade()

    await vi.waitFor(() => {
      expect(clearTopFiltersMock).toHaveBeenCalled()
      expect(sortTopTableMock).toHaveBeenCalledWith('sharpe')
      expect(selectTopRowMock).toHaveBeenCalledWith(3)
      expect(pinTopRowMock).toHaveBeenCalledWith(1)
      expect(pinSelectedTopRowMock).toHaveBeenCalled()
      expect(replacePinnedTopCandidateMock).toHaveBeenCalledWith('pinned_a')
      expect(unpinTopCandidateMock).toHaveBeenCalledWith('pinned_b')
      expect(clearPinnedTopCandidatesMock).toHaveBeenCalled()
      expect(copyBestMock).toHaveBeenCalled()
      expect(exportTopCsvMock).toHaveBeenCalled()
      expect(exportTopJsonMock).toHaveBeenCalled()
      expect(setBaselineMock).toHaveBeenCalled()
      expect(clearBaselineMock).toHaveBeenCalled()
      expect(heatmapExportPngMock).toHaveBeenCalled()
      expect(rerenderDrillMock).toHaveBeenCalled()
      expect(drillClearTradeSelectionMock).toHaveBeenCalled()
      expect(drillPrevTradeMock).toHaveBeenCalled()
      expect(drillNextTradeMock).toHaveBeenCalled()
    })
  })

  it('routes meta/stage5/ml actions through metaService', async () => {
    const store = useAnalysisStore()
    store.batchDatasets = [
      { path: 'C:/dataset.csv', label: 'dataset', meta: '—', checked: false, disabled: false },
    ]
    store.batchSelectedCount = 0
    store.batchTotalCount = 1

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
    store.saveStage5SavedTags()
    store.refreshStage5Saved()
    store.loadStage5Saved()
    store.revealStage5Saved()
    store.deleteStage5Saved()
    store.mlRefresh()
    store.mlTrain()
    store.mlApply()
    store.mlDelete()
    store.runStage4()
    store.revealStage4()
    store.runStage5()
    store.revealStage5()
    store.runMeta()
    store.openMetaReport()
    store.revealMetaFiles()
    store.toggleBatchDataset(' C:/dataset.csv ', true)

    await vi.waitFor(() => {
      expect(runBatchMock).toHaveBeenCalled()
      expect(batchStopMock).toHaveBeenCalled()
      expect(batchRefreshMock).toHaveBeenCalled()
      expect(batchSelectAllMock).toHaveBeenCalled()
      expect(batchSelectNoneMock).toHaveBeenCalled()
      expect(batchRetryFailedMock).toHaveBeenCalled()
      expect(batchRevealSummaryMock).toHaveBeenCalled()
      expect(saveSavedTagsMock).toHaveBeenCalled()
      expect(refreshSavedMock).toHaveBeenCalled()
      expect(loadSavedMock).toHaveBeenCalled()
      expect(openSavedReportMock).toHaveBeenCalled()
      expect(revealSavedMock).toHaveBeenCalled()
      expect(deleteSavedMock).toHaveBeenCalled()
      expect(saveStage4SavedTagsMock).not.toHaveBeenCalled()
      expect(refreshStage4SavedMock).not.toHaveBeenCalled()
      expect(loadStage4SavedMock).not.toHaveBeenCalled()
      expect(revealStage4SavedMock).not.toHaveBeenCalled()
      expect(deleteStage4SavedMock).not.toHaveBeenCalled()
      expect(saveStage5SavedTagsMock).toHaveBeenCalledTimes(2)
      expect(refreshStage5SavedMock).toHaveBeenCalledTimes(2)
      expect(loadStage5SavedMock).toHaveBeenCalledTimes(2)
      expect(revealStage5SavedMock).toHaveBeenCalledTimes(2)
      expect(deleteStage5SavedMock).toHaveBeenCalledTimes(2)
      expect(mlRefreshMock).toHaveBeenCalled()
      expect(mlTrainMock).toHaveBeenCalled()
      expect(mlApplyMock).toHaveBeenCalled()
      expect(mlDeleteMock).toHaveBeenCalled()
      expect(runStage4Mock).not.toHaveBeenCalled()
      expect(revealStage4Mock).not.toHaveBeenCalled()
      expect(runStage5Mock).toHaveBeenCalledTimes(2)
      expect(revealStage5Mock).toHaveBeenCalledTimes(2)
      expect(runMetaMock).toHaveBeenCalled()
      expect(openMetaReportMock).toHaveBeenCalled()
      expect(revealMetaFilesMock).toHaveBeenCalled()
      expect(toggleBatchDatasetMock).toHaveBeenCalledWith('C:/dataset.csv', true)
    })
    expect(store.batchDatasets[0]?.checked).toBe(true)
    expect(store.batchSelectedCount).toBe(1)
    expect(store.batchTotalCount).toBe(1)
  })

  it('routes overview empty actions through workflow/data/optimize stores', async () => {
    const store = useAnalysisStore()
    const workflowStore = useWorkflowStore()

    store.overviewPrimaryAction = 'browse_csv'
    store.overviewEmptyPrimary()
    expect(workflowStore.workspace).toBe('workflow')
    expect(workflowStore.workflow).toBe('data')
    expect(browseCsvMock).toHaveBeenCalled()

    store.overviewPrimaryAction = 'polygon'
    store.overviewEmptyPrimary()
    expect(openPolygonMock).toHaveBeenCalled()

    store.overviewPrimaryAction = 'go_opt'
    store.overviewEmptyPrimary()
    expect(workflowStore.workspace).toBe('workflow')
    expect(workflowStore.workflow).toBe('optimize')

    store.overviewPrimaryAction = 'run_opt'
    store.overviewEmptyPrimary()
    await vi.dynamicImportSettled()
    await vi.waitFor(() => {
      expect(runOptimizationMock).toHaveBeenCalled()
    })

    store.overviewPrimaryAction = 'cancel_run'
    store.overviewEmptyPrimary()
    await vi.dynamicImportSettled()
    await vi.waitFor(() => {
      expect(cancelOptimizationMock).toHaveBeenCalled()
    })
  })
})
