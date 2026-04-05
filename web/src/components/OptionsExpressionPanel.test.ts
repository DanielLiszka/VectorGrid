// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import OptionsExpressionPanel from './OptionsExpressionPanel.vue'
import { useOptionsExpressionStore } from '../stores/optionsExpressionStore'

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

describe('OptionsExpressionPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('shows an unsupported message for non-DoubleMa parents', () => {
    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_ml_signal.json',
        parentStrategyKind: 'MlSignal',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    expect(wrapper.text()).toContain('supports saved DoubleMa runs only')
    expect(listOptionsDatasetsMock).not.toHaveBeenCalled()
  })

  it('bootstraps datasets and renders the options follow-up controls for DoubleMa parents', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        providerName: 'massive',
        providerSourceSurface: 'minute_aggregate',
        quoteResolution: '1m',
        priceSemantics: 'minute_aggregate',
        contractReferenceAsOfUtc: '2026-04-05T00:00:00Z',
        contractCount: 3,
        rowCount: 42,
        aggregatePriceField: 'close',
      },
    ])
    listOptionsExpressionArtifactsMock.mockResolvedValue([
      {
        ts: '2026-04-05T12:00:00Z',
        path: 'C:/runs/vectorgrid_options_expression_1.json',
        bytes: 2048,
        parentArtifactId: 'vectorgrid_run_1',
        rootSymbol: 'SPY',
        optionsDatasetId: 'spy_options',
        backendUsed: 'cpu_reference',
        priceSemantics: 'minute_aggregate',
        tupleCount: 2,
        bestRobustScore: 0.18,
        bestSelectionExpectancyPerPremium: 0.2,
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
      },
    ])

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: 'DoubleMa',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
    })

    expect(wrapper.text()).toContain('Options Expression')
    expect(wrapper.text()).toContain('Optimize expression')
    expect(wrapper.text()).toContain('spy_options')
    expect(wrapper.text()).toContain('massive | minute_aggregate | 1m | close | 2026-04-05T00:00:00Z')
    expect(wrapper.text()).toContain('robust 0.180')
    expect(wrapper.text()).toContain('sealed')
    expect(wrapper.text()).toContain('stress')
  })

  it('boots once the parent strategy becomes DoubleMa after mount', async () => {
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

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: '',
        parentDatasetLabel: '',
      },
    })

    expect(wrapper.text()).toContain('Saved run not supported yet.')
    expect(listOptionsDatasetsMock).not.toHaveBeenCalled()

    await wrapper.setProps({
      parentStrategyKind: 'DoubleMa',
      parentDatasetLabel: 'SPY.csv',
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
    })

    expect(wrapper.text()).toContain('Options Expression')
    expect(wrapper.text()).toContain('spy_options')
  })

  it('exports active ranked tuples from the current result card', async () => {
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
    exportOptionsExpressionCsvMock.mockReturnValue(true)

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: 'DoubleMa',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
    })

    const store = useOptionsExpressionStore()
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
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.1667,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.15,
            winRate: 0.6,
            totalPnlPerContract: 60,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: null,
        },
      ],
    }

    await wrapper.vm.$nextTick()

    const exportButton = wrapper.findAll('button').find((entry) => entry.text() === 'Export CSV')
    expect(exportButton).toBeTruthy()
    await exportButton!.trigger('click')

    expect(exportOptionsExpressionCsvMock).toHaveBeenCalledWith(
      expect.objectContaining({
        filenameStem: 'vectorgrid_run_1_options_expression',
        rootSymbol: 'SPY',
      }),
    )
  })

  it('renders the feasibility preview summary when available', async () => {
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

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: 'DoubleMa',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
    })

    const store = useOptionsExpressionStore()
    store.latestFeasibility = {
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
        { reason: 'premium_too_high', count: 16 },
      ],
    }

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Phase 0.5 preview')
    expect(wrapper.text()).toContain('54 / 90 (60.0%)')
    expect(wrapper.text()).toContain('18 ms / 41 ms')
    expect(wrapper.text()).toContain('no_expiration_in_bucket (20)')
  })

  it('shows holdout, sealed, stress, and invalid-rate reporting when those metrics are present', async () => {
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

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: 'DoubleMa',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
    })

    const store = useOptionsExpressionStore()
    store.latestRun = {
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      artifactTs: '2026-04-05T12:00:00Z',
      parentArtifactId: 'vectorgrid_run_1',
      optionsDatasetHash: 'hash-1',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 1,
      request: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentCandidateIndex: null,
        optionsDatasetId: 'spy_options',
        delayMinutes: [0, 1],
        dteBuckets: [1, 3],
        moneynessBuckets: ['atm', 'otm_1'],
        holdMinutes: [15, 30],
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
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.1667,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.15,
            winRate: 0.6,
            totalPnlPerContract: 60,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.07,
            medianReturnOnPremium: 0.05,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 3,
            expectancyPerPremium: 0.04,
            medianReturnOnPremium: 0.03,
            winRate: 0.5,
            totalPnlPerContract: 6,
          },
        },
      ],
    }

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Invalid %')
    expect(wrapper.text()).toContain('Holdout EV / Premium')
    expect(wrapper.text()).toContain('Sealed EV / Premium')
    expect(wrapper.text()).toContain('Stress EV / Premium')
    expect(wrapper.text()).toContain('Bull / Bear')
    expect(wrapper.text()).toContain('6 / 4')
    expect(wrapper.text()).toContain('Search / Holdout / Sealed')
    expect(wrapper.text()).toContain('6 / 2 / 2')
    expect(wrapper.text()).toContain('Candidates')
    expect(wrapper.text()).toContain('9 / 12')
    expect(wrapper.text()).toContain('Exit summaries')
    expect(wrapper.text()).toContain('24 / 24')
    expect(wrapper.text()).toContain('Run provenance')
    expect(wrapper.text()).toContain('2026-04-05T12:00:00Z')
    expect(wrapper.text()).toContain('hash-1')
    expect(wrapper.text()).toContain('0, 1 | 1, 3 | 15, 30')
    expect(wrapper.text()).toContain('0.60 / 0.20')
    expect(wrapper.text()).toContain('0.167')
    expect(wrapper.text()).toContain('0.1000')
    expect(wrapper.text()).toContain('0.0700')
    expect(wrapper.text()).toContain('0.0400')
  })

  it('shows a selected tuple inspector and updates when a different row is clicked', async () => {
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

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: 'DoubleMa',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
    })

    const store = useOptionsExpressionStore()
    store.latestRun = {
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      artifactTs: '',
      parentArtifactId: '',
      optionsDatasetHash: '',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 2,
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
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.1667,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.15,
            winRate: 0.6,
            totalPnlPerContract: 60,
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
          exitTemplate: 'base_exit_or_target_stop',
          fillModel: 'quote_mid',
          robustScore: 0.33,
          selectionMetrics: {
            validTrades: 7,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 18,
            expectancyPerPremium: 0.31,
            medianReturnOnPremium: 0.22,
            winRate: 0.71,
            totalPnlPerContract: 126,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: null,
        },
      ],
    }

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Selected tuple')
    expect(wrapper.text()).toContain('fixed_hold_or_target_stop')
    expect(wrapper.text()).toContain('aggregate_research')

    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(2)
    expect(rows[0]?.classes()).toContain('isSelected')
    await rows[1]!.trigger('click')

    expect(rows[1]?.classes()).toContain('isSelected')
    expect(wrapper.text()).toContain('base_exit_or_target_stop')
    expect(wrapper.text()).toContain('quote_mid')
    expect(wrapper.text()).toContain('126.0000')
  })

  it('loads lazy event drilldown for the selected tuple', async () => {
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
    inspectOptionsExpressionTupleMock.mockResolvedValue({
      tupleIndex: 0,
      tupleCount: 2,
      totalEvents: 3,
      executedEvents: 2,
      candidateMissingEvents: 1,
      outcomeMissingEvents: 0,
      tupleResult: {
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
        {
          eventIndex: 1,
          eventId: 'event-2',
          split: 'holdout',
          side: 'bearish',
          decisionTimestamp: 200,
          earliestTradableTimestamp: 260,
          requestedEntryTimestamp: 560,
          rawUnderlierPrice: 498.75,
          baseExitTimestamp: null,
          status: 'candidate_missing',
          contractId: '',
          expirationDate: '',
          strike: null,
          multiplier: null,
          entryTimestamp: null,
          entryOptionPrice: null,
          entryUnderlierPrice: null,
          exitTimestamp: null,
          exitOptionPrice: null,
          premiumPaid: null,
          pnlPerContract: null,
          returnOnPremium: null,
          exitReason: '',
        },
      ],
    })

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentStrategyKind: 'DoubleMa',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1)
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1)
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

    await wrapper.vm.$nextTick()

    const loadButton = wrapper
      .findAll('button')
      .find((entry) => entry.text() === 'Load event drilldown')
    expect(loadButton).toBeTruthy()
    await loadButton!.trigger('click')

    await vi.waitFor(() => {
      expect(inspectOptionsExpressionTupleMock).toHaveBeenCalledWith({
        artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
        tupleIndex: 0,
      })
    })

    expect(wrapper.text()).toContain('Tuple event drilldown')
    expect(wrapper.text()).toContain('SPY_C_1')
    expect(wrapper.text()).toContain('target_hit')
    expect(wrapper.text()).toContain('candidate_missing')
    expect(wrapper.text()).toContain('Showing 2 of 2 event rows')
  })
})
