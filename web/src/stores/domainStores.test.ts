import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, getActivePinia, setActivePinia } from 'pinia'
import { useAdvancedStore } from './advancedStore'
import { useAnalysisStore } from './analysisStore'
import { useDataStore } from './dataStore'
import { useExecutionStore } from './executionStore'
import { useHistoryStore } from './historyStore'
import { useOptimizeStore } from './optimizeStore'
import { useRuntimeStore } from './runtimeStore'
import { useSettingsStore } from './settingsStore'
import { useStrategyStore } from './strategyStore'
import { useValidationStore } from './validationStore'
import { useWorkflowArtifactsStore } from './workflowArtifactsStore'
import { useWorkflowStore } from './workflowStore'

describe('domain stores', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('applies data snapshot from legacy bridge', () => {
    const store = useDataStore()
    store.applyLegacySnapshot({
      activeControlsTab: 'data',
      csvPath: 'C:/tmp/aapl.csv',
      recentCsvPaths: ['C:/tmp/aapl.csv', 'C:/tmp/msft.csv'],
      sampleCsvOptions: [{ label: 'AAPL sample', path: 'C:/samples/aapl.csv', disabled: false }],
      sampleCsvVisible: true,
      marketDataDir: 'C:/market-data',
      marketDataMoveExisting: false,
      marketDataDirHint: 'Current folder: C:/market-data',
      dataId: 'abc',
      datasetLabel: 'AAPL',
      datasetRange: '2024..2025',
      dataLen: '1234',
      dataTimeframe: '5m',
      dataTimeframeHint: 'Will resample to 5m',
      autoRestoreDataset: true,
      datasetContext: 'AAPL 1m from Polygon',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataFieldHint: 'Using canonical OHLCV',
      canLoadCsv: true,
      canApplyTimeframe: false,
      statusLine: 'Loaded.',
    })
    expect(store.activeControlsTab).toBe('data')
    expect(store.csvPath).toContain('aapl.csv')
    expect(store.recentCsvPaths.length).toBe(2)
    expect(store.sampleCsvOptions.length).toBe(1)
    expect(store.sampleCsvOptions[0]?.path).toContain('samples/aapl.csv')
    expect(store.sampleCsvVisible).toBe(true)
    expect(store.marketDataDir).toContain('market-data')
    expect(store.marketDataMoveExisting).toBe(false)
    expect(store.dataId).toBe('abc')
    expect(store.datasetLabel).toBe('AAPL')
    expect(store.dataTimeframe).toBe('5m')
    expect(store.dataFields).toContain('close')
    expect(store.canApplyTimeframe).toBe(true)
    expect(store.statusLine).toBe('Loaded.')
    expect(store.source).toBe('vue')
  })

  it('applies optimize snapshot and disabled states', () => {
    const store = useOptimizeStore()
    store.applyLegacySnapshot({
      activeControlsTab: 'opt',
      strategyKind: 'DoubleMa',
      strategyHint: 'Double MA crossover',
      presetProfile: 'Balanced',
      backend: 'CpuOnly',
      fastStep: '1',
      slowStep: '2',
      pairsPreview: 'Pairs: 100',
      gpuParityMeta: 'Parity: ok',
      runPlanMeta: 'Plan: medium grid',
      runReadinessHint: 'Readiness: green',
      runReadinessBadges: ['Data loaded', 'Params valid'],
      runDisabled: true,
      canCancel: true,
      validateDisabled: false,
      canParityCheck: false,
    })
    expect(store.activeControlsTab).toBe('opt')
    expect(store.strategyKind).toBe('DoubleMa')
    expect(store.strategyHint).toContain('Double')
    expect(store.presetProfile).toBe('Balanced')
    expect(store.backend).toBe('CpuOnly')
    expect(store.fastStep).toBe('1')
    expect(store.slowStep).toBe('2')
    expect(store.pairsPreview).toContain('100')
    expect(store.gpuParityMeta).toContain('ok')
    expect(store.runPlanMeta).toContain('medium grid')
    expect(store.runReadinessHint).toContain('green')
    expect(store.runReadinessBadges.length).toBe(2)
    expect(store.runDisabled).toBe(true)
    expect(store.canCancel).toBe(true)
    expect(store.validateDisabled).toBe(false)
    expect(store.canParityCheck).toBe(false)
  })

  it('applies validation snapshot and row count', () => {
    const store = useValidationStore()
    store.applyLegacySnapshot({
      activeResultsTab: 'validate',
      validationHint: 'Run validation after optimization',
      validationMeta: 'validated 10',
      validationShortlistMeta: 'shortlist 4 rows',
      validationColumns: [{ label: '#', alignLeft: true }],
      validationRows: [{ cells: [{ text: '1', className: 'mono left', title: '' }] }],
      tableRowCount: 10,
      candidates: '64',
      seed: '7',
      minPsr: '0.8',
      minDsr: '0.7',
      cscvSlices: '16',
      autoRun: true,
      autoExportXlsx: false,
      autoRevealReport: true,
      autoExportTopN: '20',
      wfEnable: true,
      wfWindow: '5000',
      wfStep: '1000',
      perturbEnable: true,
      perturbIters: '12',
      perturbMin: '0.02',
      perturbMax: '0.08',
      mcEnable: true,
      mcSims: '1000',
      cscvEnable: true,
      crossEnable: true,
      crossMax: '8',
      crossSearch: 'aapl',
      crossDatasets: [{ path: 'C:/data/aapl.csv', label: 'AAPL', meta: 'stocks', checked: true, disabled: false }],
      crossSelectedCount: 1,
      crossTotalCount: 1,
      tableVisible: true,
      canExportValidationCsv: true,
      canCrossRefresh: true,
      canCrossSelectAll: true,
      canCrossSelectNone: true,
      canCrossSelectMatch: false,
    })
    expect(store.activeResultsTab).toBe('validate')
    expect(store.validationHint).toContain('Run')
    expect(store.validationMeta).toContain('10')
    expect(store.validationShortlistMeta).toContain('shortlist')
    expect(store.validationColumns.length).toBe(1)
    expect(store.validationRows.length).toBe(1)
    expect(store.tableRowCount).toBe(10)
    expect(store.candidates).toBe('64')
    expect(store.seed).toBe('7')
    expect(store.minPsr).toBe('0.8')
    expect(store.minDsr).toBe('0.7')
    expect(store.cscvSlices).toBe('16')
    expect(store.autoRun).toBe(true)
    expect(store.autoRevealReport).toBe(true)
    expect(store.wfEnable).toBe(true)
    expect(store.wfWindow).toBe('5000')
    expect(store.perturbEnable).toBe(true)
    expect(store.perturbIters).toBe('12')
    expect(store.mcEnable).toBe(true)
    expect(store.cscvEnable).toBe(true)
    expect(store.crossEnable).toBe(true)
    expect(store.crossMax).toBe('8')
    expect(store.crossSearch).toBe('aapl')
    expect(store.crossDatasets.length).toBe(1)
    expect(store.crossSelectedCount).toBe(1)
    expect(store.canCrossSelectMatch).toBe(false)
    expect(store.tableVisible).toBe(true)
    expect(store.canExportValidationCsv).toBe(true)
  })

  it('tracks runtime boot phase transitions', () => {
    const store = useRuntimeStore()
    store.markBooting('boot')
    expect(store.bootPhase).toBe('booting')
    store.markReady()
    expect(store.bootPhase).toBe('ready')
    store.markError('oops')
    expect(store.bootPhase).toBe('error')
    expect(store.bootError).toContain('oops')
  })

  it('applies workflow artifact snapshot from legacy runtime bridge', () => {
    const pinia = getActivePinia()
    const store = pinia ? useWorkflowArtifactsStore(pinia) : useWorkflowArtifactsStore()
    store.applyLegacyRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        artifact_path: 'C:/runs/run.json',
        report_xlsx_path: 'C:/runs/report.xlsx',
        csv_path: 'C:/data/aapl.csv',
        tags: ['batch-a', 'holdout'],
        req: { strategy_kind: 'DoubleMa' },
        res: { rows: 10 },
      },
      currentValidation: {
        ts: '2026-02-25T12:00:00Z',
        req: { candidates: 32 },
        res: { rows: [] },
      },
    })

    expect(store.currentRun?.ts).toBe('2026-02-25T12:00:00Z')
    expect(store.currentRun?.artifactPath).toContain('run.json')
    expect(store.currentRun?.reportXlsxPath).toContain('report.xlsx')
    expect(store.currentRun?.csvPath).toContain('aapl.csv')
    expect(store.currentRun?.tags).toEqual(['batch-a', 'holdout'])
    expect(store.hasRun).toBe(true)
    expect(store.currentValidation?.ts).toBe('2026-02-25T12:00:00Z')
    expect(store.hasValidation).toBe(true)
    expect(store.source).toBe('legacy-runtime')
  })

  it('applies workflow snapshot from legacy bridge', () => {
    const store = useWorkflowStore()
    store.applyLegacySnapshot({
      workspace: 'pro',
      workflow: 'optimize',
      workflowStep: 'validate',
      workflowSubtitle: 'Testing subtitle',
    })
    expect(store.workspace).toBe('pro')
    expect(store.workflow).toBe('optimize')
    expect(store.workflowStep).toBe('validate')
    expect(store.workflowSubtitle).toContain('subtitle')
  })

  it('applies analysis snapshot from legacy bridge', () => {
    const store = useAnalysisStore()
    store.applyLegacySnapshot({
      topColumns: [{ key: 'rank', label: '#', sort: 'asc', alignLeft: true }],
      topTableRows: [{ selected: true, cells: [{ text: '1', className: 'mono left', title: '' }] }],
      canCopyBest: false,
      bestStats: [{ label: 'PnL', value: '1.23', tone: 'good' }],
      overviewEmptyVisible: true,
      overviewEmptyTitle: 'Load data to begin',
      overviewEmptyBody: 'Load CSV first',
      overviewEmptyHint: 'Tip text',
      overviewPrimaryLabel: 'Browse CSV…',
      overviewPrimaryAction: 'browse_csv',
      overviewPrimaryDisabled: false,
      overviewSecondaryVisible: true,
      overviewSecondaryLabel: 'Polygon…',
      overviewSecondaryAction: 'polygon',
      overviewSecondaryDisabled: false,
      bestSummary: 'Summary lines',
      filterMinPnl: '100',
      canClearTopFilters: false,
      paretoX: 'max_dd',
      paretoY: 'sharpe',
      paretoWrapVisible: true,
      drillWrapVisible: true,
      stabilityWrapVisible: true,
      tradeStatsWrapVisible: false,
      heatmapWrapVisible: true,
      heatmapColor: 'viridis',
      canHeatmapExportPng: false,
      metaStrategyKind: 'CompositeCpu',
      metaTimeframe: '1m',
      metaRequiredTag: 'forex',
      metaMinSamples: '40',
      metaTopK: '20',
      metaHoldoutTag: 'holdout',
      metaEvalHoldout: true,
      metaSaveHoldoutRows: true,
      metaHoldoutMode: 'heavy',
      metaHoldoutReps: '3',
      metaHoldoutEvalTopN: '12',
      metaHoldoutEvalRankBy: 'ml',
      metaHoldoutExploreN: '20',
      metaGroupingMode: 'exact',
      metaClusterIntBucket: '7',
      metaClusterFloatBucket: '0.5',
      metaHoldoutNote: 'Holdout runs are excluded',
      aggregateSummarySavedRuns: '12',
      metaMeta: 'Meta complete',
      metaTableColumns: [{ label: '#', alignLeft: false }],
      metaTableRows: [{ cells: [{ text: '1', className: 'mono', title: '' }] }],
      metaCompareOptions: [{ value: 'meta-a', label: 'Meta A', disabled: false }],
      metaCompareSelected: 'meta-a',
      metaCompareSummary: 'delta=0.12',
      batchDatasetSearch: 'aapl',
      batchTags: 'batch-1',
      batchHoldoutPct: '20',
      batchHoldoutTag: 'holdout',
      batchAutoMeta: false,
      batchSkipDone: false,
      batchDatasets: [{ path: 'C:/data/aapl.csv', label: 'AAPL', meta: 'stocks', checked: true, disabled: false }],
      batchSelectedCount: 1,
      batchTotalCount: 1,
      batchMeta: 'selected 1/1',
      batchOutputPreview: '{"batch":true}',
      canRunBatch: true,
      canBatchStop: true,
      canBatchRefresh: true,
      canBatchSelectAll: true,
      canBatchSelectNone: true,
      canBatchRetryFailed: false,
      canBatchRevealSummary: false,
      metaSavedSearch: 'core',
      metaSavedOptions: [{ value: 'meta-1', label: 'Meta #1', disabled: false }],
      metaSavedSelected: 'meta-1',
      metaSavedTagsDraft: 'holdout,1m',
      metaSavedMeta: 'saved meta = 3',
      canSaveMetaSavedTags: true,
      stage4SavedSearch: 'stage4',
      stage4SavedOptions: [{ value: 'stage4-1', label: 'Stage4 #1', disabled: false }],
      stage4SavedSelected: 'stage4-1',
      stage4SavedTagsDraft: 'final',
      stage4SavedMeta: 'saved stage4 = 2',
      canSaveStage4SavedTags: true,
      metaUseMlConfidence: true,
      metaMlAutopilot: false,
      canMetaMlRefresh: true,
      canMetaMlTrain: false,
      canMetaMlApply: true,
      canMetaMlDelete: true,
      metaMlStatus: 'healthy model',
      metaStage4TopK: '15',
      metaStage4MinConf: '0.65',
      metaStage4MetaWeight: '0.55',
      metaStage4MlWeight: '0.40',
      metaStage4UncertaintyWeight: '0.05',
      metaStage4AcceptConf: '0.72',
      metaStage4ReviewConf: '0.60',
      metaStage4MaxAcceptUncertainty: '0.35',
      metaStage4AutoTrain: false,
      metaStage4RequireHealthy: true,
      metaStage4FallbackToMeta: false,
      canRunMetaStage4: true,
      canRevealMetaStage4: true,
      metaStage4Status: 'ready',
      metaStage4OutputPreview: '{"stage4":true}',
      debugOutputPreview: '{"ok":true}',
      activeResultsTab: 'drill',
      bestTitle: 'Fast=10 Slow=30',
      topRows: 128,
      stabilityStatus: 'Done.',
    })
    expect(store.canCopyBest).toBe(false)
    expect(store.topColumns.length).toBe(1)
    expect(store.topColumns[0]?.key).toBe('rank')
    expect(store.topTableRows.length).toBe(1)
    expect(store.topTableRows[0]?.selected).toBe(true)
    expect(store.bestStats.length).toBe(1)
    expect(store.bestStats[0]?.label).toBe('PnL')
    expect(store.overviewEmptyVisible).toBe(true)
    expect(store.overviewEmptyTitle).toContain('Load data')
    expect(store.overviewPrimaryAction).toBe('browse_csv')
    expect(store.overviewSecondaryLabel).toContain('Polygon')
    expect(store.bestSummary).toContain('Summary')
    expect(store.filterMinPnl).toBe('100')
    expect(store.canClearTopFilters).toBe(false)
    expect(store.paretoX).toBe('max_dd')
    expect(store.paretoY).toBe('sharpe')
    expect(store.paretoWrapVisible).toBe(true)
    expect(store.drillWrapVisible).toBe(true)
    expect(store.stabilityWrapVisible).toBe(true)
    expect(store.tradeStatsWrapVisible).toBe(false)
    expect(store.heatmapWrapVisible).toBe(true)
    expect(store.heatmapColor).toBe('viridis')
    expect(store.canHeatmapExportPng).toBe(false)
    expect(store.metaStrategyKind).toBe('CompositeCpu')
    expect(store.metaTimeframe).toBe('1m')
    expect(store.metaRequiredTag).toBe('forex')
    expect(store.metaMinSamples).toBe('40')
    expect(store.metaTopK).toBe('20')
    expect(store.metaHoldoutTag).toBe('holdout')
    expect(store.metaEvalHoldout).toBe(true)
    expect(store.metaSaveHoldoutRows).toBe(true)
    expect(store.metaHoldoutMode).toBe('heavy')
    expect(store.metaHoldoutReps).toBe('3')
    expect(store.metaHoldoutEvalTopN).toBe('12')
    expect(store.metaHoldoutEvalRankBy).toBe('ml')
    expect(store.metaHoldoutExploreN).toBe('20')
    expect(store.metaGroupingMode).toBe('exact')
    expect(store.metaClusterIntBucket).toBe('7')
    expect(store.metaClusterFloatBucket).toBe('0.5')
    expect(store.metaHoldoutNote).toContain('excluded')
    expect(store.aggregateSummarySavedRuns).toBe('12')
    expect(store.metaMeta).toContain('Meta')
    expect(store.metaTableColumns.length).toBe(1)
    expect(store.metaTableRows.length).toBe(1)
    expect(store.metaCompareOptions.length).toBe(1)
    expect(store.metaCompareSelected).toBe('meta-a')
    expect(store.metaCompareSummary).toContain('delta')
    expect(store.batchDatasetSearch).toBe('aapl')
    expect(store.batchTags).toBe('batch-1')
    expect(store.batchHoldoutPct).toBe('20')
    expect(store.batchAutoMeta).toBe(false)
    expect(store.batchDatasets.length).toBe(1)
    expect(store.batchSelectedCount).toBe(1)
    expect(store.batchMeta).toContain('1/1')
    expect(store.batchOutputPreview).toContain('batch')
    expect(store.canBatchStop).toBe(true)
    expect(store.metaSavedSearch).toBe('core')
    expect(store.metaSavedOptions.length).toBe(1)
    expect(store.metaSavedSelected).toBe('meta-1')
    expect(store.metaSavedTagsDraft).toContain('holdout')
    expect(store.metaSavedMeta).toContain('saved meta')
    expect(store.canSaveMetaSavedTags).toBe(true)
    expect(store.stage4SavedSearch).toBe('stage4')
    expect(store.stage4SavedOptions.length).toBe(1)
    expect(store.stage4SavedSelected).toBe('stage4-1')
    expect(store.stage4SavedTagsDraft).toBe('final')
    expect(store.stage4SavedMeta).toContain('stage4')
    expect(store.canSaveStage4SavedTags).toBe(true)
    expect(store.metaUseMlConfidence).toBe(true)
    expect(store.metaMlAutopilot).toBe(false)
    expect(store.canMetaMlTrain).toBe(false)
    expect(store.metaMlStatus).toContain('healthy')
    expect(store.metaStage4TopK).toBe('15')
    expect(store.metaStage4MinConf).toBe('0.65')
    expect(store.metaStage4AutoTrain).toBe(false)
    expect(store.canRunMetaStage4).toBe(true)
    expect(store.metaStage4OutputPreview).toContain('stage4')
    expect(store.debugOutputPreview).toContain('"ok"')
    expect(store.activeResultsTab).toBe('drill')
    expect(store.bestTitle).toContain('Fast')
    expect(store.topRows).toBe(128)
    expect(store.stabilityStatus).toContain('Done')
  })

  it('applies advanced snapshot from legacy bridge', () => {
    const store = useAdvancedStore()
    store.applyLegacySnapshot({
      activeControlsTab: 'advanced',
      maSource: 'hlc3',
      objective: 'Sortino',
      mode: 'GpuVdps',
      modeOptions: [
        { value: 'Grid', label: 'Grid', disabled: false },
        { value: 'CoarseToFine', label: 'CoarseToFine', disabled: false },
        { value: 'GpuVdps', label: 'GpuVdps', disabled: false },
      ],
      vdpsVisible: true,
      compositeMoVisible: true,
      heatmapBins: '256',
      topK: '50',
      includeAll: 'true',
      exportAllCsvPath: 'C:/tmp/all_pairs.csv',
      canBrowseExportAll: false,
    })
    expect(store.activeControlsTab).toBe('advanced')
    expect(store.maSource).toBe('hlc3')
    expect(store.objective).toBe('Sortino')
    expect(store.mode).toBe('GpuVdps')
    expect(store.modeOptions.map((option) => option.value)).toEqual(['Grid', 'CoarseToFine', 'GpuVdps'])
    expect(store.vdpsVisible).toBe(true)
    expect(store.compositeMoVisible).toBe(true)
    expect(store.heatmapBins).toBe('256')
    expect(store.topK).toBe('50')
    expect(store.includeAll).toBe('true')
    expect(store.exportAllCsvPath).toContain('all_pairs.csv')
    expect(store.canBrowseExportAll).toBe(false)
  })

  it('applies history snapshot from legacy bridge', () => {
    const store = useHistoryStore()
    store.applyLegacySnapshot({
      sessionRunCount: 5,
      sessionOptions: [{ value: 'a', label: 'run-a', disabled: false }],
      sessionSelectedValue: 'a',
      savedRunCount: 12,
      savedOptions: [{ value: 'b', label: 'saved-b', disabled: false }],
      savedSelectedValue: 'b',
      savedSelectedLabel: 'AAPL 2026-02-19',
      canApplySavedRun: true,
    })
    expect(store.sessionRunCount).toBe(5)
    expect(store.sessionOptions.length).toBe(1)
    expect(store.sessionSelectedValue).toBe('a')
    expect(store.savedRunCount).toBe(12)
    expect(store.savedOptions.length).toBe(1)
    expect(store.savedSelectedValue).toBe('b')
    expect(store.savedSelectedLabel).toContain('AAPL')
    expect(store.canApplySavedRun).toBe(true)
    expect(store.source).toBe('vue')
  })

  it('applies settings snapshot from legacy bridge', () => {
    const store = useSettingsStore()
    store.applyLegacySnapshot({
      theme: 'dark',
      effect: 'mica',
      workflowAdvanced: true,
      controlsCollapsed: true,
      marketDataDir: 'C:/data',
      moveExisting: false,
      canOpenPolygonModal: false,
      canGotoLibrary: false,
    })
    expect(store.theme).toBe('dark')
    expect(store.effect).toBe('mica')
    expect(store.workflowAdvanced).toBe(true)
    expect(store.controlsCollapsed).toBe(true)
    expect(store.marketDataDir).toContain('C:/data')
    expect(store.moveExisting).toBe(false)
    expect(store.canOpenPolygonModal).toBe(false)
    expect(store.canGotoLibrary).toBe(false)
  })

  it('applies strategy snapshot from legacy bridge', () => {
    const store = useStrategyStore()
    store.applyLegacySnapshot({
      activeControlsTab: 'strategy',
      costPreset: 'stocks_liquid',
      feeBps: '1.5',
      spreadBps: '2',
      slippageBps: '0.5',
      commission: '0.0002',
      epsRel: '0.001',
      costHint: 'Total: 4 bps',
      exitMode: 'kaufman_stop',
      kaufmanStopMaType: 'ema',
      dmConstraintsVisible: false,
    })
    expect(store.activeControlsTab).toBe('strategy')
    expect(store.costPreset).toBe('stocks_liquid')
    expect(store.feeBps).toBe('1.5')
    expect(store.commission).toBe('0.0002')
    expect(store.costHint).toContain('4 bps')
    expect(store.exitMode).toBe('kaufman_stop')
    expect(store.kaufmanStopMaType).toBe('ema')
    expect(store.dmConstraintsVisible).toBe(false)
  })

  it('applies execution snapshot from legacy bridge', () => {
    const store = useExecutionStore()
    store.applyLegacySnapshot({
      mode: 'live',
      symbol: 'AAPL',
      qty: '2',
      riskStopLossPct: '0.02',
      manualFastLen: '10',
      orderCount: 9,
      orderRows: [{ cells: ['2026-01-01 12:00:00', 'buy', 'filled', '1', '1', '189.55', 'ord_123'] }],
      canRunOnce: false,
      canClearKeys: false,
      liveStatusHint: 'Live candles active (AAPL)',
    })
    expect(store.mode).toBe('live')
    expect(store.symbol).toBe('AAPL')
    expect(store.qty).toBe('2')
    expect(store.riskStopLossPct).toBe('0.02')
    expect(store.manualFastLen).toBe('10')
    expect(store.orderCount).toBe(9)
    expect(store.orderRows[0]?.cells?.[1]).toBe('buy')
    expect(store.canRunOnce).toBe(false)
    expect(store.canClearKeys).toBe(false)
    expect(store.liveStatusHint).toContain('AAPL')
  })
})

