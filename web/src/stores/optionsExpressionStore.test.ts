import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  useOptionsExpressionStore,
} from './optionsExpressionStore'

const {
  analyzeOptionsExpressionFeasibilityMock,
  exportOptionsExpressionCsvMock,
  importOptionsDatasetMock,
  inspectOptionsExpressionTupleMock,
  listOptionsDatasetsMock,
  listOptionsExpressionArtifactsMock,
  loadOptionsExpressionArtifactMock,
  openOrRevealOptionsPathMock,
  pickOptionsCsvPathMock,
  runOptionsExpressionMock,
} = vi.hoisted(() => ({
  analyzeOptionsExpressionFeasibilityMock: vi.fn(),
  exportOptionsExpressionCsvMock: vi.fn(),
  importOptionsDatasetMock: vi.fn(),
  inspectOptionsExpressionTupleMock: vi.fn(),
  listOptionsDatasetsMock: vi.fn(),
  listOptionsExpressionArtifactsMock: vi.fn(),
  loadOptionsExpressionArtifactMock: vi.fn(),
  openOrRevealOptionsPathMock: vi.fn(),
  pickOptionsCsvPathMock: vi.fn(),
  runOptionsExpressionMock: vi.fn(),
}))

vi.mock('../services/optionsExpressionService', () => ({
  analyzeOptionsExpressionFeasibility: analyzeOptionsExpressionFeasibilityMock,
  exportOptionsExpressionCsv: exportOptionsExpressionCsvMock,
  importOptionsDataset: importOptionsDatasetMock,
  inspectOptionsExpressionTuple: inspectOptionsExpressionTupleMock,
  listOptionsDatasets: listOptionsDatasetsMock,
  listOptionsExpressionArtifacts: listOptionsExpressionArtifactsMock,
  loadOptionsExpressionArtifact: loadOptionsExpressionArtifactMock,
  openOrRevealOptionsPath: openOrRevealOptionsPathMock,
  pickOptionsCsvPath: pickOptionsCsvPathMock,
  runOptionsExpression: runOptionsExpressionMock,
}))

describe('optionsExpressionStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('prefers matching-root datasets for the attached parent artifact', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'qqq_options',
        rootSymbol: 'QQQ',
        priceSemantics: 'minute_aggregate',
        contractCount: 5,
        rowCount: 100,
      },
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        priceSemantics: 'minute_aggregate',
        contractCount: 3,
        rowCount: 42,
      },
    ])
    listOptionsExpressionArtifactsMock.mockResolvedValue([])

    const store = useOptionsExpressionStore()
    store.attachParent('C:/runs/vectorgrid_run_1.json', 'DoubleMa', 'SPY.csv')
    await store.bootstrap()

    expect(store.parentArtifactId).toBe('vectorgrid_run_1')
    expect(store.parentRootHint).toBe('SPY')
    expect(store.selectedDatasetId).toBe('spy_options')
  })

  it('runs options expression with normalized request fields', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        priceSemantics: 'minute_aggregate',
        contractCount: 3,
        rowCount: 42,
      },
    ])
    listOptionsExpressionArtifactsMock.mockResolvedValue([])
    runOptionsExpressionMock.mockResolvedValue({
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 2,
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
      },
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 10,
        bullishEvents: 6,
        bearishEvents: 4,
        searchEvents: 6,
        holdoutEvents: 2,
        sealedTestEvents: 2,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults: [],
    })

    const store = useOptionsExpressionStore()
    store.attachParent('C:/runs/vectorgrid_run_1.json', 'DoubleMa', 'SPY.csv')
    await store.bootstrap()
    store.delayMinutesInput = '0, 5, 1'
    store.selectedMoneynessBuckets = ['atm', 'otm_1']
    store.topKInput = '12'
    await store.runCurrentExpression()

    expect(runOptionsExpressionMock).toHaveBeenCalledWith(
      expect.objectContaining({
        parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
        options_dataset_id: 'spy_options',
        delay_minutes: [0, 1, 5],
        moneyness_buckets: ['atm', 'otm_1'],
        top_k: 12,
      }),
    )
    expect(store.latestRun?.artifactPath).toBe('C:/runs/vectorgrid_options_expression_1.json')
    expect(store.activeStaging?.compiledCandidateCount).toBe(9)
  })

  it('previews feasibility with normalized request fields', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        priceSemantics: 'minute_aggregate',
        contractCount: 3,
        rowCount: 42,
      },
    ])
    listOptionsExpressionArtifactsMock.mockResolvedValue([])
    analyzeOptionsExpressionFeasibilityMock.mockResolvedValue({
      replayBuildMs: 18,
      totalPreviewMs: 41,
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 10,
        bullishEvents: 6,
        bearishEvents: 4,
        searchEvents: 6,
        holdoutEvents: 2,
        sealedTestEvents: 2,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      priceSemantics: 'minute_aggregate',
      fillModel: 'aggregate_research',
      compiledTupleCount: 12,
      perEventCandidateSlots: 9,
      perCandidateExitSummaries: 6,
      candidateSlotCount: 90,
      compiledCandidateCount: 54,
      candidateDensity: 0.6,
      invalidSlotCount: 36,
      invalidSlotRate: 0.4,
      baselineExitSummaryCount: 324,
      baselineCompiledExitSummaryCount: 240,
      baselineExitSummaryDensity: 0.7407,
      conservativeExitSummaryCount: 324,
      conservativeCompiledExitSummaryCount: 228,
      conservativeExitSummaryDensity: 0.7037,
      projectedEventTupleEvaluations: 120,
      structuralCacheBytesEstimate: 4096,
      baselineSummaryBytesEstimate: 8192,
      conservativeSummaryBytesEstimate: 8192,
      timings: {
        tupleCompileMs: 1,
        baselineCandidateCompileMs: 2,
        baselineExitSummaryCompileMs: 3,
        conservativeCandidateCompileMs: 4,
        conservativeExitSummaryCompileMs: 5,
        cpuRankingMs: 6,
      },
      eventCoverage: {
        totalEventsWithCandidates: 9,
        searchEventsWithCandidates: 5,
        holdoutEventsWithCandidates: 2,
        sealedTestEventsWithCandidates: 2,
        totalEventsWithBaselineOutcomes: 8,
        searchEventsWithBaselineOutcomes: 4,
        holdoutEventsWithBaselineOutcomes: 2,
        sealedTestEventsWithBaselineOutcomes: 2,
        totalEventsWithConservativeOutcomes: 7,
        searchEventsWithConservativeOutcomes: 4,
        holdoutEventsWithConservativeOutcomes: 2,
        sealedTestEventsWithConservativeOutcomes: 1,
      },
      invalidReasons: [
        { reason: 'no_expiration_in_bucket', count: 20 },
      ],
    })

    const store = useOptionsExpressionStore()
    store.attachParent('C:/runs/vectorgrid_run_1.json', 'DoubleMa', 'SPY.csv')
    await store.bootstrap()
    store.delayMinutesInput = '0, 5, 1'
    store.selectedMoneynessBuckets = ['atm', 'otm_1']
    await store.previewCurrentFeasibility()

    expect(analyzeOptionsExpressionFeasibilityMock).toHaveBeenCalledWith(
      expect.objectContaining({
        parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
        options_dataset_id: 'spy_options',
        delay_minutes: [0, 1, 5],
        moneyness_buckets: ['atm', 'otm_1'],
      }),
    )
    expect(store.latestFeasibility?.compiledTupleCount).toBe(12)
    expect(store.statusLine).toContain('60.0% candidate density')
  })

  it('imports datasets from the CSV picker and reveals artifacts', async () => {
    pickOptionsCsvPathMock.mockResolvedValue('C:/imports/spy_options.csv')
    importOptionsDatasetMock.mockResolvedValue({
      optionsDatasetId: 'spy_options',
      rootSymbol: 'SPY',
      priceSemantics: 'minute_aggregate',
      contractCount: 3,
      rowCount: 42,
    })
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        priceSemantics: 'minute_aggregate',
        contractCount: 3,
        rowCount: 42,
      },
    ])
    openOrRevealOptionsPathMock.mockResolvedValue({
      path: 'C:/runs/vectorgrid_options_expression_1.json',
      openedWith: 'reveal_path',
    })

    const store = useOptionsExpressionStore()
    store.attachParent('C:/runs/vectorgrid_run_1.json', 'DoubleMa', 'SPY.csv')
    await store.importDatasetFromPicker()
    store.latestRun = {
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      artifactTs: '',
      parentArtifactId: '',
      optionsDatasetHash: '',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 1,
      request: null,
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
      },
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 1,
        bullishEvents: 1,
        bearishEvents: 0,
        searchEvents: 1,
        holdoutEvents: 0,
        sealedTestEvents: 0,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults: [],
    }
    await store.revealActiveArtifact()

    expect(importOptionsDatasetMock).toHaveBeenCalledWith(
      expect.objectContaining({
        csv_path: 'C:/imports/spy_options.csv',
        root_symbol: 'SPY',
      }),
    )
    expect(openOrRevealOptionsPathMock).toHaveBeenCalledWith('C:/runs/vectorgrid_options_expression_1.json')
  })

  it('loads saved options expression artifacts', async () => {
    loadOptionsExpressionArtifactMock.mockResolvedValue({
      version: 1,
      kind: 'vectorgrid_options_expression_v1',
      ts: '2026-04-05T12:00:00Z',
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      parentArtifactId: 'vectorgrid_run_1',
      rootSymbol: 'SPY',
      optionsDatasetId: 'spy_options',
      optionsDatasetHash: 'hash-1',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
      },
      request: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentCandidateIndex: null,
        optionsDatasetId: 'spy_options',
        delayMinutes: [0],
        dteBuckets: [1],
        moneynessBuckets: ['atm'],
        holdMinutes: [15],
        targetReturnBuckets: [0.25],
        stopReturnBuckets: [0.25],
        exitTemplate: 'fixed_hold_or_target_stop',
        fillModel: 'aggregate_research',
        filters: {
          maxEntryLagMinutes: 5,
          maxPremium: null,
          minPremium: null,
          minVolume: null,
          minOpenInterest: null,
        },
        topK: 25,
        minValidTrades: null,
        splitRatioSearch: 0.6,
        splitRatioHoldout: 0.2,
      },
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 10,
        bullishEvents: 6,
        bearishEvents: 4,
        searchEvents: 6,
        holdoutEvents: 2,
        sealedTestEvents: 2,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      selectionMetrics: [],
    })

    const store = useOptionsExpressionStore()
    await store.loadArtifact('C:/runs/vectorgrid_options_expression_1.json')

    expect(store.activeArtifact?.parentArtifactId).toBe('vectorgrid_run_1')
    expect(store.latestRun).toBeNull()
  })

  it('loads tuple inspection rows for the active artifact', async () => {
    inspectOptionsExpressionTupleMock.mockResolvedValue({
      tupleIndex: 1,
      tupleCount: 2,
      totalEvents: 3,
      executedEvents: 2,
      candidateMissingEvents: 1,
      outcomeMissingEvents: 0,
      tupleResult: {
        delayMinutes: 5,
        dteDays: 3,
        moneynessBucket: 'otm_1',
        holdMinutes: 30,
        targetReturn: 0.5,
        stopReturn: 0.25,
        exitTemplate: 'fixed_hold_or_target_stop',
        fillModel: 'aggregate_research',
        robustScore: 0.12,
        selectionMetrics: {
          validTrades: 2,
          invalidEvents: 1,
          invalidRate: 0.3333,
          expectancyPerTrade: 10,
          expectancyPerPremium: 0.11,
          medianReturnOnPremium: 0.08,
          winRate: 0.5,
          totalPnlPerContract: 20,
        },
        holdoutMetrics: null,
        sealedEvaluationMetrics: null,
        stressMetrics: null,
      },
      rows: [
        {
          eventIndex: 0,
          eventId: 'event-1',
          split: 'search',
          side: 'bullish',
          decisionTimestamp: 100,
          earliestTradableTimestamp: 160,
          requestedEntryTimestamp: 460,
          rawUnderlierPrice: 501.25,
          baseExitTimestamp: 700,
          status: 'executed',
          contractId: 'SPY_C_1',
          expirationDate: '2026-04-12',
          strike: 505,
          multiplier: 100,
          entryTimestamp: 460,
          entryOptionPrice: 1.4,
          entryUnderlierPrice: 501.5,
          exitTimestamp: 520,
          exitOptionPrice: 1.8,
          premiumPaid: 140,
          pnlPerContract: 40,
          returnOnPremium: 0.2857,
          exitReason: 'target_hit',
        },
      ],
    })

    const store = useOptionsExpressionStore()
    store.latestRun = {
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      artifactTs: '2026-04-05T12:00:00Z',
      parentArtifactId: 'vectorgrid_run_1',
      optionsDatasetHash: 'hash-1',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 2,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 3,
        bullishEvents: 2,
        bearishEvents: 1,
        searchEvents: 2,
        holdoutEvents: 1,
        sealedTestEvents: 0,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: 'atm',
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: 'fixed_hold_or_target_stop',
          fillModel: 'aggregate_research',
          robustScore: 0.18,
          selectionMetrics: {
            validTrades: 2,
            invalidEvents: 1,
            invalidRate: 0.3333,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.15,
            winRate: 0.6,
            totalPnlPerContract: 24,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: null,
        },
        {
          delayMinutes: 5,
          dteDays: 3,
          moneynessBucket: 'otm_1',
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: 'fixed_hold_or_target_stop',
          fillModel: 'aggregate_research',
          robustScore: 0.12,
          selectionMetrics: {
            validTrades: 2,
            invalidEvents: 1,
            invalidRate: 0.3333,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.11,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 20,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: null,
        },
      ],
    }

    await store.loadTupleInspection(1)

    expect(inspectOptionsExpressionTupleMock).toHaveBeenCalledWith({
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      tupleIndex: 1,
    })
    expect(store.activeTupleInspection?.rows[0]?.contractId).toBe('SPY_C_1')
    expect(store.statusLine).toContain('Loaded 1 event row')
  })

  it('exports the active ranked tuples to CSV', () => {
    exportOptionsExpressionCsvMock.mockReturnValue(true)
    const rankedResults = Array.from({ length: 12 }, (_, index) => ({
      delayMinutes: index % 3,
      dteDays: 1 + (index % 2),
      moneynessBucket: index % 2 === 0 ? 'atm' : 'otm_1',
      holdMinutes: 15 + index,
      targetReturn: 0.25,
      stopReturn: 0.25,
      exitTemplate: 'fixed_hold_or_target_stop',
      fillModel: 'aggregate_research',
      robustScore: 0.18 + index * 0.01,
      selectionMetrics: {
        validTrades: 5 + index,
        invalidEvents: 1,
        invalidRate: 0.1667,
        expectancyPerTrade: 12,
        expectancyPerPremium: 0.2,
        medianReturnOnPremium: 0.15,
        winRate: 0.6,
        totalPnlPerContract: 60 + index,
      },
      holdoutMetrics: null,
      sealedEvaluationMetrics: null,
      stressMetrics: null,
    }))

    const store = useOptionsExpressionStore()
    store.attachParent('C:/runs/vectorgrid_run_1.json', 'DoubleMa', 'SPY.csv')
    store.latestRun = {
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      artifactTs: '',
      parentArtifactId: '',
      optionsDatasetHash: '',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 1,
      request: null,
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
      },
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 10,
        bullishEvents: 6,
        bearishEvents: 4,
        searchEvents: 6,
        holdoutEvents: 2,
        sealedTestEvents: 2,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults,
    }

    store.exportActiveCsv()

    const exportInput = exportOptionsExpressionCsvMock.mock.calls[0]?.[0]
    expect(exportOptionsExpressionCsvMock).toHaveBeenCalledWith(
      expect.objectContaining({
        filenameStem: 'vectorgrid_run_1_options_expression',
        rootSymbol: 'SPY',
        backendUsed: 'cpu_reference',
        priceSemantics: 'minute_aggregate',
      }),
    )
    expect(exportInput?.tuples).toHaveLength(12)
    expect(store.statusLine).toContain('Exported 12 ranked tuples')
  })

  it('resets active selection when the parent artifact changes', () => {
    const store = useOptionsExpressionStore()
    store.attachParent('C:/runs/vectorgrid_run_1.json', 'DoubleMa', 'SPY.csv')
    store.selectedArtifactPath = 'C:/runs/vectorgrid_options_expression_old.json'
    store.activeArtifact = {
      version: 1,
      kind: 'vectorgrid_options_expression_v1',
      ts: '2026-04-05T12:00:00Z',
      artifactPath: 'C:/runs/vectorgrid_options_expression_old.json',
      parentArtifactId: 'vectorgrid_run_1',
      rootSymbol: 'SPY',
      optionsDatasetId: 'spy_options',
      optionsDatasetHash: 'hash-1',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
      },
      request: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentCandidateIndex: null,
        optionsDatasetId: 'spy_options',
        delayMinutes: [0],
        dteBuckets: [1],
        moneynessBuckets: ['atm'],
        holdMinutes: [15],
        targetReturnBuckets: [0.25],
        stopReturnBuckets: [0.25],
        exitTemplate: 'fixed_hold_or_target_stop',
        fillModel: 'aggregate_research',
        filters: {
          maxEntryLagMinutes: 5,
          maxPremium: null,
          minPremium: null,
          minVolume: null,
          minOpenInterest: null,
        },
        topK: 25,
        minValidTrades: null,
        splitRatioSearch: 0.6,
        splitRatioHoldout: 0.2,
      },
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 10,
        bullishEvents: 6,
        bearishEvents: 4,
        searchEvents: 6,
        holdoutEvents: 2,
        sealedTestEvents: 2,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      selectionMetrics: [],
    }
    store.latestRun = {
      artifactPath: 'C:/runs/vectorgrid_options_expression_latest.json',
      artifactTs: '',
      parentArtifactId: '',
      optionsDatasetHash: '',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 1,
      request: null,
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
      },
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 10,
        bullishEvents: 6,
        bearishEvents: 4,
        searchEvents: 6,
        holdoutEvents: 2,
        sealedTestEvents: 2,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults: [],
    }

    store.attachParent('C:/runs/vectorgrid_run_2.json', 'DoubleMa', 'QQQ.csv')

    expect(store.parentArtifactId).toBe('vectorgrid_run_2')
    expect(store.parentRootHint).toBe('QQQ')
    expect(store.selectedArtifactPath).toBe('')
    expect(store.activeArtifact).toBeNull()
    expect(store.latestRun).toBeNull()
  })
})
