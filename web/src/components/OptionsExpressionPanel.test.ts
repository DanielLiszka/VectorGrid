// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import OptionsExpressionPanel from './OptionsExpressionPanel.vue'
import { useOptionsExpressionStore } from '../stores/optionsExpressionStore'

const {
  analyzeOptionsExpressionFeasibilityMock,
  downloadPolygonOptionsDatasetMock,
  exportOptionsExpressionCsvMock,
  importOptionsDatasetMock,
  inspectOptionsExpressionEventPathMock,
  inspectOptionsExpressionTupleMock,
  listOptionsDatasetsMock,
  listOptionsExpressionArtifactsMock,
  loadOptionsExpressionArtifactMock,
  openOrRevealOptionsPathMock,
  pickOptionsCsvPathMock,
  runOptionsExpressionMock,
} = vi.hoisted(() => ({
  analyzeOptionsExpressionFeasibilityMock: vi.fn(),
  downloadPolygonOptionsDatasetMock: vi.fn(),
  exportOptionsExpressionCsvMock: vi.fn(),
  importOptionsDatasetMock: vi.fn(),
  inspectOptionsExpressionEventPathMock: vi.fn(),
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
  downloadPolygonOptionsDataset: downloadPolygonOptionsDatasetMock,
  exportOptionsExpressionCsv: exportOptionsExpressionCsvMock,
  importOptionsDataset: importOptionsDatasetMock,
  inspectOptionsExpressionEventPath: inspectOptionsExpressionEventPathMock,
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

  it('shows an unsupported message for non-double-like parents', () => {
    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_ml_signal.json',
        parentStrategyKind: 'MlSignal',
        parentDatasetLabel: 'SPY.csv',
      },
    })

    expect(wrapper.text()).toContain('supports saved DoubleMa')
    expect(wrapper.text()).toContain('BreakoutMomentum')
    expect(listOptionsDatasetsMock).not.toHaveBeenCalled()
  })

  it('bootstraps datasets and renders the options follow-up controls for DoubleMa parents', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        providerName: 'massive',
        providerSourceSurface: 'minute_aggregate',
        providerEntitlementProfile: 'options_business',
        quoteResolution: '1m',
        priceSemantics: 'minute_aggregate',
        calendarId: 'market_hours_utc',
        timezone: 'UTC',
        coverageStart: 1_700_000_000_000,
        coverageEnd: 1_700_000_120_000,
        contractReferenceSource: 'massive_contract_reference',
        underlierReferenceSource: 'underlying_dataset',
        datasetPath: 'C:/datasets/spy_options',
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
        parentStrategyKind: 'DoubleMa',
        rootSymbol: 'SPY',
        optionsDatasetId: 'spy_options',
        backendUsed: 'cpu_reference',
        backendFallbackReason: 'GPU planner probe failed; using CPU.',
        priceSemantics: 'minute_aggregate',
        requestBackendPreference: 'auto',
        requestFillModel: 'aggregate_research',
        requestStructureFamily: 'long_single',
        requestSplitAssignmentMode: 'event_chronological',
        tupleCount: 2,
        bestRobustScore: 0.18,
        bestSelectionExpectancyPerPremium: 0.2,
        bestHoldoutExpectancyPerPremium: 0.1,
        bestSealedExpectancyPerPremium: 0.07,
        bestStressExpectancyPerPremium: 0.04,
        bestSealedStressExpectancyPerPremium: 0.03,
        bestHarsherStressExpectancyPerPremium: 0.02,
        bestSealedHarsherStressExpectancyPerPremium: 0.01,
        bestExtremeStressExpectancyPerPremium: -0.01,
        bestSealedExtremeStressExpectancyPerPremium: -0.02,
        topValidationVerdict: 'stress_fragile',
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
        hasHarsherStressMetrics: true,
        hasExtremeStressMetrics: true,
        validatedTupleCount: 2,
        stressFragileTupleCount: 1,
      },
      {
        ts: '2026-04-05T12:30:00Z',
        path: 'C:/runs/vectorgrid_options_expression_1_quotes.json',
        bytes: 2048,
        parentArtifactId: 'vectorgrid_run_3',
        parentStrategyKind: 'DoubleMa',
        rootSymbol: 'SPY',
        optionsDatasetId: 'spy_quotes',
        backendUsed: 'gpu_compiled_tensor',
        priceSemantics: 'top_of_book_quote',
        requestBackendPreference: 'auto',
        requestFillModel: 'quote_conservative',
        requestStructureFamily: 'long_single',
        requestSplitAssignmentMode: 'event_chronological',
        tupleCount: 2,
        bestRobustScore: 0.19,
        bestSelectionExpectancyPerPremium: 0.18,
        bestHoldoutExpectancyPerPremium: 0.14,
        bestSealedExpectancyPerPremium: 0.15,
        bestStressExpectancyPerPremium: 0.12,
        bestSealedStressExpectancyPerPremium: 0.1,
        bestHarsherStressExpectancyPerPremium: 0.09,
        bestSealedHarsherStressExpectancyPerPremium: 0.08,
        bestExtremeStressExpectancyPerPremium: 0.07,
        bestSealedExtremeStressExpectancyPerPremium: 0.06,
        topValidationVerdict: 'sealed_finalist',
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
        hasHarsherStressMetrics: true,
        hasExtremeStressMetrics: true,
        validatedTupleCount: 2,
        stressFragileTupleCount: 0,
      },
      {
        ts: '2026-04-05T13:00:00Z',
        path: 'C:/runs/vectorgrid_options_expression_2.json',
        bytes: 2048,
        parentArtifactId: 'vectorgrid_run_2',
        parentStrategyKind: 'CompositeTrend',
        rootSymbol: 'QQQ',
        optionsDatasetId: 'qqq_quotes',
        backendUsed: 'gpu_compiled_tensor',
        priceSemantics: 'top_of_book_quote',
        requestBackendPreference: 'gpu_compiled_tensor',
        requestFillModel: 'quote_conservative',
        requestStructureFamily: 'debit_vertical',
        requestSplitAssignmentMode: 'session_chronological',
        tupleCount: 3,
        bestRobustScore: 0.24,
        bestSelectionExpectancyPerPremium: 0.22,
        bestHoldoutExpectancyPerPremium: 0.15,
        bestSealedExpectancyPerPremium: 0.13,
        bestStressExpectancyPerPremium: 0.11,
        bestSealedStressExpectancyPerPremium: 0.1,
        bestHarsherStressExpectancyPerPremium: 0.08,
        bestSealedHarsherStressExpectancyPerPremium: 0.07,
        bestExtremeStressExpectancyPerPremium: 0.05,
        bestSealedExtremeStressExpectancyPerPremium: 0.04,
        topValidationVerdict: 'sealed_finalist',
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
        hasHarsherStressMetrics: true,
        hasExtremeStressMetrics: true,
        validatedTupleCount: 3,
        stressFragileTupleCount: 0,
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
    expect(wrapper.text()).toContain('This is an options follow-up surface.')
    expect(wrapper.text()).toContain('Optimize expression')
    expect(wrapper.text()).toContain('Realism lane')
    expect(wrapper.text()).toContain('Split assignment')
    expect(wrapper.find('option[value="session_chronological"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Polygon/Massive 1m options download')
    expect(wrapper.text()).toContain('Aggregate lane')
    expect(wrapper.text()).toContain('only support aggregate_research in the backend')
    expect(wrapper.find('#optionsRealismLane').exists()).toBe(true)
    expect(wrapper.find('#optionsImportPriceSemantics').exists()).toBe(true)
    expect(wrapper.text()).toContain('The selected realism lane controls the expected CSV semantics for this import.')
    expect(wrapper.find('option[value="quote_conservative"]').attributes('disabled')).toBeDefined()
    expect(wrapper.find('option[value="quote_mid"]').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('Lane product status')
    expect(wrapper.text()).toContain('Saved datasets')
    expect(wrapper.text()).toContain('Saved artifacts')
    expect(wrapper.text()).toContain('Avg validation EV')
    expect(wrapper.text()).toContain('Avg stressed EV')
    expect(wrapper.text()).toContain('This download is options-only and stays separate from the normal stock data workflow.')
    expect(wrapper.text()).toContain('Dataset manifest')
    expect(wrapper.text()).toContain('options_business')
    expect(wrapper.text()).toContain('C:/datasets/spy_options')
    expect(wrapper.text()).toContain('spy_options')
    expect(wrapper.text()).toContain('massive | minute_aggregate | 1m | close | 2026-04-05T00:00:00Z')
    expect(wrapper.text()).toContain('Saved artifact comparison')
    expect(wrapper.text()).toContain('Artifact frontier leaders')
    expect(wrapper.text()).toContain('Options artifact meta-analysis')
    expect(wrapper.text()).toContain('Reopened artifact validation frontier')
    expect(wrapper.text()).toContain('By parent family')
    expect(wrapper.text()).toContain('CompositeTrend')
    expect(wrapper.text()).toContain('By root')
    expect(wrapper.text()).toContain('QQQ')
    expect(wrapper.text()).toContain('By structure family')
    expect(wrapper.text()).toContain('debit_vertical')
    expect(wrapper.text()).toContain('By realism lane')
    expect(wrapper.text()).toContain('Quote-aware · quote_conservative')
    expect(wrapper.text()).toContain('Realism lane disagreement')
    expect(wrapper.text()).toContain('SPY · DoubleMa · long_single')
    expect(wrapper.text()).toContain('Aggregate -> Quote-aware')
    expect(wrapper.text()).toContain('Lane')
    expect(wrapper.text()).toContain('Validation')
    expect(wrapper.text()).toContain('Validated %')
    expect(wrapper.text()).toContain('Stress %')
    expect(wrapper.text()).toContain('Val Δ')
    expect(wrapper.text()).toContain('Stress Δ')
    expect(wrapper.text()).toContain('Diff vs selected')
    expect(wrapper.text()).toContain('auto -> cpu_reference')
    expect(wrapper.text()).toContain('gpu_compiled_tensor')
    expect(wrapper.text()).toContain('Aggregate lane')
    expect(wrapper.text()).toContain('aggregate_research')
    expect(wrapper.text()).toContain('validation Stress-fragile')
    expect(wrapper.text()).toContain('quote_conservative')
    expect(wrapper.text()).toContain('robust 0.180')
    expect(wrapper.text()).toContain('hold 0.100')
    expect(wrapper.text()).toContain('sealed 0.070')
    expect(wrapper.text()).toContain('stress 0.040')
    expect(wrapper.text()).toContain('harsher 0.020')
    expect(wrapper.text()).toContain('sealed harsher 0.010')
    expect(wrapper.text()).toContain('extreme -0.010')
    expect(wrapper.text()).toContain('sealed extreme -0.020')
    expect(wrapper.text()).toContain('Validated ratio')
    expect(wrapper.text()).toContain('Fragility ratio')
    expect(wrapper.text()).toContain('Selection')
    expect(wrapper.text()).toContain('Holdout')
    expect(wrapper.text()).toContain('Best validated artifact')
    expect(useOptionsExpressionStore().splitAssignmentMode).toBe('event_chronological')
  })

  it('switches the workflow into the quote-aware lane and disables direct provider download', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        priceSemantics: 'minute_aggregate',
        contractCount: 3,
        rowCount: 42,
      },
      {
        optionsDatasetId: 'spy_quotes',
        rootSymbol: 'SPY',
        priceSemantics: 'top_of_book_quote',
        contractCount: 2,
        rowCount: 24,
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
    })

    await wrapper.find('#optionsRealismLane').setValue('quote_aware')

    const downloadButton = wrapper.findAll('button').find((entry) => entry.text() === 'Download Polygon/Massive 1m data')
    expect(downloadButton).toBeTruthy()
    expect(downloadButton!.attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('Quote-aware realism is currently a CSV-import lane.')
    expect(wrapper.text()).toContain('Direct provider download in this panel is currently available only for the Aggregate lane.')
    expect(wrapper.text()).toContain('Select Quote-aware Polygon/Massive dataset…')
    expect(wrapper.text()).toContain('Lane product status')
    expect(wrapper.text()).toContain('Saved datasets')
    expect(wrapper.text()).toContain('No saved artifacts currently match the selected Quote-aware lane for this parent run.')
  })

  it('downloads a bounded Polygon/Massive dataset from the options panel', async () => {
    listOptionsDatasetsMock.mockResolvedValue([])
    listOptionsExpressionArtifactsMock.mockResolvedValue([])
    downloadPolygonOptionsDatasetMock.mockResolvedValue({
      manifest: {
        optionsDatasetId: 'spy_options_1m',
        rootSymbol: 'SPY',
        providerName: 'massive',
        providerSourceSurface: 'minute_aggregate',
        quoteResolution: '1m',
        priceSemantics: 'minute_aggregate',
        contractReferenceAsOfUtc: '2026-04-06T23:59:00Z',
        contractCount: 8,
        rowCount: 200,
        aggregatePriceField: 'close',
      },
      contractsDiscovered: 22,
      contractsSelected: 8,
      contractsWithRows: 6,
      downloadedOptionRows: 200,
      skippedContracts: 2,
      underlyingCsvPath: 'C:/cache/polygon_stocks_raw_SPY_1m.csv',
      usedUnderlyingCacheOnly: true,
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

    const inputs = wrapper.findAll('input[type="date"]')
    expect(inputs.length).toBeGreaterThanOrEqual(4)
    await inputs[0]!.setValue('2026-04-01')
    await inputs[1]!.setValue('2026-04-02')

    await wrapper.find('#optionsDownloadContractType').setValue('put')

    const downloadButton = wrapper.findAll('button').find((entry) => entry.text() === 'Download Polygon/Massive 1m data')
    expect(downloadButton).toBeTruthy()
    await downloadButton!.trigger('click')

    await vi.waitFor(() => {
      expect(downloadPolygonOptionsDatasetMock).toHaveBeenCalledWith(
        expect.objectContaining({
          rootSymbol: 'SPY',
          from: '2026-04-01',
          to: '2026-04-02',
          contractType: 'put',
        }),
      )
    })

    const store = useOptionsExpressionStore()
    await vi.waitFor(() => {
      expect(store.statusLine).toContain('Downloaded Polygon/Massive options dataset')
    })
    expect(store.latestDatasetDownload?.underlyingCsvPath).toBe('C:/cache/polygon_stocks_raw_SPY_1m.csv')
    expect(wrapper.text()).toContain('Latest download audit')
    expect(wrapper.text()).toContain('cache-only reuse')
    expect(wrapper.text()).toContain('C:/cache/polygon_stocks_raw_SPY_1m.csv')
  })

  it('boots once the parent strategy becomes a supported double-like family after mount', async () => {
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
      parentStrategyKind: 'CompositeTrend',
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
    exportOptionsExpressionCsvMock.mockResolvedValue({
      mode: 'saved',
      filename: 'vectorgrid_run_1_options_expression.csv',
      path: 'C:/exports/vectorgrid_run_1_options_expression.csv',
      bytes: 512,
      revealed: true,
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
        harsherExitSummaryCount: 24,
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

    await vi.waitFor(() => {
      expect(exportOptionsExpressionCsvMock).toHaveBeenCalledWith(
        expect.objectContaining({
          filenameStem: 'vectorgrid_run_1_options_expression',
          rootSymbol: 'SPY',
        }),
      )
    })
  })

  it('shows backend fallback details in the request summary when available', async () => {
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
      artifactTs: '2026-04-07T14:00:00Z',
      parentArtifactId: 'vectorgrid_run_1',
      optionsDatasetHash: 'hash-1',
      backendUsed: 'cpu_reference',
      backendFallbackReason: 'GPU planner probe failed; using CPU.',
      priceSemantics: 'minute_aggregate',
      tupleCount: 1,
      request: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        parentCandidateIndex: null,
        optionsDatasetId: 'spy_options',
        delayMinutes: [0, 1],
        dteBuckets: [1, 3],
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
        backendPreference: 'auto',
        splitAssignmentMode: 'event_chronological',
        topK: 25,
        sealedFinalistCount: 5,
        minValidTrades: 0,
        splitRatioSearch: 0.6,
        splitRatioHoldout: 0.2,
      },
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
        harsherExitSummaryCount: 24,
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

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Backend detail')
    expect(wrapper.text()).toContain('GPU planner probe failed; using CPU.')
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
      bestAvailableBackend: 'gpu_compiled_tensor_hybrid',
      autoBackendDecision: {
        recommendedBackend: 'cpu_reference',
        gpuEligible: false,
        reasons: ['Measured GPU hybrid speedup 2.00x is below the current promotion gate of 3.0x.'],
      },
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
      harsherExitSummaryCount: 324,
      harsherCompiledExitSummaryCount: 216,
      harsherExitSummaryDensity: 0.6667,
      projectedEventTupleEvaluations: 120,
      structuralCacheBytesEstimate: 4096,
      baselineSummaryBytesEstimate: 8192,
      conservativeSummaryBytesEstimate: 8192,
      harsherSummaryBytesEstimate: 8192,
      timings: {
        tupleCompileMs: 1,
        baselineCandidateCompileMs: 2,
        baselineExitSummaryCompileMs: 3,
        conservativeCandidateCompileMs: 4,
        conservativeExitSummaryCompileMs: 5,
        harsherCandidateCompileMs: 6,
        harsherExitSummaryCompileMs: 7,
        cpuRankingMs: 6,
        cpuRankingUs: 6_400,
        gpuRankingMs: 3,
        gpuRankingUs: 3_100,
        gpuTensorScoreMs: 1,
        gpuTensorScoreUs: 1_200,
        gpuExactifyMs: 1,
        gpuExactifyUs: 1_100,
        gpuSealedStageMs: 0,
        gpuSealedStageUs: 800,
        gpuSpeedupVsCpu: 2,
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
        totalEventsWithHarsherOutcomes: 6,
        searchEventsWithHarsherOutcomes: 3,
        holdoutEventsWithHarsherOutcomes: 2,
        sealedTestEventsWithHarsherOutcomes: 1,
      },
      cacheStats: {
        structuralCacheHits: 2,
        structuralCacheMisses: 2,
        structuralCacheEntries: 2,
        summaryCacheHits: 2,
        summaryCacheMisses: 2,
        summaryCacheEntries: 2,
        gpuSharedInputCacheHits: 1,
        gpuSharedInputCacheMisses: 1,
        gpuOutcomeInputCacheHits: 1,
        gpuOutcomeInputCacheMisses: 1,
        gpuOutputBufferReuses: 2,
        gpuOutputBufferAllocations: 1,
        gpuTupleChunkCount: 3,
        gpuTupleChunkSize: 4096,
        gpuKernelLaunchCount: 6,
        gpuTupleBudgetBytes: 4 * 1024 * 1024,
        gpuOutcomeTensorBytes: 2 * 1024 * 1024,
        gpuSharedInputBytesPerChunk: 64 * 1024,
        gpuOutputBytesPerChunk: 128 * 1024,
        gpuTotalDeviceBytesPerChunk: 3 * 1024 * 1024,
        gpuDeviceTotalMemoryBytes: 8 * 1024 * 1024 * 1024,
        gpuDeviceFreeMemoryBytes: 512 * 1024 * 1024,
        gpuDeviceUsableMemoryBytes: 256 * 1024 * 1024,
        gpuMemoryBudgetSource: 'free_memory_headroom',
        gpuOutcomeTensorFitsInMemory: true,
        gpuChunkPlanFitsInMemory: true,
        gpuPlanNotes: ['GPU planner probe failed before ranking: synthetic test note'],
      },
      invalidReasons: [
        { reason: 'no_expiration_in_bucket', count: 20 },
        { reason: 'premium_too_high', count: 16 },
      ],
      axisCoverage: {
        delayMinutes: [
          {
            delayMinutes: 0,
            slotCount: 30,
            validSlotCount: 24,
            invalidSlotCount: 6,
            validRate: 0.8,
            invalidReasons: [{ reason: 'premium_too_high', count: 6 }],
          },
          {
            delayMinutes: 5,
            slotCount: 30,
            validSlotCount: 18,
            invalidSlotCount: 12,
            validRate: 0.6,
            invalidReasons: [{ reason: 'entry_past_cutoff', count: 8 }],
          },
          {
            delayMinutes: 15,
            slotCount: 30,
            validSlotCount: 0,
            invalidSlotCount: 30,
            validRate: 0,
            invalidReasons: [{ reason: 'entry_past_cutoff', count: 22 }],
          },
        ],
        dteDays: [
          {
            dteDays: 1,
            slotCount: 30,
            validSlotCount: 24,
            invalidSlotCount: 6,
            validRate: 0.8,
            invalidReasons: [{ reason: 'premium_too_high', count: 6 }],
          },
          {
            dteDays: 3,
            slotCount: 30,
            validSlotCount: 18,
            invalidSlotCount: 12,
            validRate: 0.6,
            invalidReasons: [{ reason: 'premium_too_high', count: 10 }],
          },
          {
            dteDays: 5,
            slotCount: 30,
            validSlotCount: 0,
            invalidSlotCount: 30,
            validRate: 0,
            invalidReasons: [{ reason: 'no_expiration_in_bucket', count: 20 }],
          },
        ],
        moneynessBuckets: [
          {
            moneynessBucket: 'atm',
            slotCount: 30,
            validSlotCount: 24,
            invalidSlotCount: 6,
            validRate: 0.8,
            invalidReasons: [{ reason: 'premium_too_high', count: 6 }],
          },
          {
            moneynessBucket: 'otm_1',
            slotCount: 30,
            validSlotCount: 18,
            invalidSlotCount: 12,
            validRate: 0.6,
            invalidReasons: [{ reason: 'premium_too_high', count: 10 }],
          },
          {
            moneynessBucket: 'itm_1',
            slotCount: 30,
            validSlotCount: 12,
            invalidSlotCount: 18,
            validRate: 0.4,
            invalidReasons: [{ reason: 'premium_too_high', count: 8 }],
          },
        ],
      },
    }

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Phase 0.5 preview')
    expect(wrapper.text()).toContain('54 / 90 (60.0%)')
    expect(wrapper.text()).toContain('18 ms / 41 ms')
    expect(wrapper.text()).toContain('gpu_compiled_tensor_hybrid')
    expect(wrapper.text()).toContain('cpu 6.400 ms / gpu hybrid 3.100 ms (2.00x)')
    expect(wrapper.text()).toContain('tensor 1.200 ms · exactify 1.100 ms · sealed 0.800 ms')
    expect(wrapper.text()).toContain('GPU viability')
    expect(wrapper.text()).toContain('Candidate density >= 30%')
    expect(wrapper.text()).toContain('GPU hybrid speedup >= 3.0x')
    expect(wrapper.text()).toContain('Auto backend')
    expect(wrapper.text()).toContain('cpu_reference · CPU-favored')
    expect(wrapper.text()).toContain('74.1% / 70.4% / 66.7%')
    expect(wrapper.text()).toContain('struct 2/2 · summary 2/2')
    expect(wrapper.text()).toContain('shared 1/1 · outcome 1/1 · out 2/1')
    expect(wrapper.text()).toContain('chunks 3×4096 · launches 6')
    expect(wrapper.text()).toContain('free_memory_headroom · free 512.00 MiB · usable 256.00 MiB · budget 4.00 MiB')
    expect(wrapper.text()).toContain('outcome fits · chunk fits')
    expect(wrapper.text()).toContain('no_expiration_in_bucket (20)')
    expect(wrapper.text()).toContain('Bucket coverage')
    expect(wrapper.text()).toContain('Delay buckets')
    expect(wrapper.text()).toContain('DTE buckets')
    expect(wrapper.text()).toContain('15m')
    expect(wrapper.text()).toContain('5d')
    expect(wrapper.text()).toContain('0 / 30')
    expect(wrapper.text()).toContain('Recommendations')
    expect(wrapper.text()).toContain('GPU planner probe failed before ranking: synthetic test note')
    expect(wrapper.text()).toContain('Reduce the default DTE spread for this dataset')
    expect(wrapper.text()).toContain('These DTE buckets are producing zero valid slots on the current dataset: 5d')
    expect(wrapper.text()).toContain('Late-entry pressure is concentrated in these delay buckets: 5m, 15m')
  })

  it('shows holdout, sealed, stress, and invalid-rate reporting when those metrics are present', async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        providerName: 'massive',
        providerSourceSurface: 'minute_aggregate',
        quoteResolution: '1m',
        priceSemantics: 'minute_aggregate',
        contractReferenceAsOfUtc: '2026-04-05T00:00:00Z',
        aggregatePriceField: 'close',
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
        splitAssignmentMode: 'session_chronological',
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
        sealedFinalistCount: 1,
        backendPreference: 'auto',
      },
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
        harsherExitSummaryCount: 24,
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
          preSealedRank: 1,
          sealedFinalist: true,
          sealedFinalRank: 1,
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
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 2,
            expectancyPerPremium: 0.03,
            medianReturnOnPremium: 0.02,
            winRate: 0.5,
            totalPnlPerContract: 4,
          },
          harsherStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 2,
            expectancyPerPremium: 0.02,
            medianReturnOnPremium: 0.01,
            winRate: 0.5,
            totalPnlPerContract: 3,
          },
          sealedHarsherStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 1,
            expectancyPerPremium: 0.01,
            medianReturnOnPremium: 0.01,
            winRate: 0.5,
            totalPnlPerContract: 2,
          },
          extremeStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 0,
            expectancyPerPremium: -0.01,
            medianReturnOnPremium: -0.01,
            winRate: 0.5,
            totalPnlPerContract: 0,
          },
          sealedExtremeStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: -1,
            expectancyPerPremium: -0.02,
            medianReturnOnPremium: -0.02,
            winRate: 0.5,
            totalPnlPerContract: -2,
          },
        },
      ],
    }

    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('session_chronological | 0.60 / 0.20')

    expect(wrapper.text()).toContain('Invalid %')
    expect(wrapper.text()).toContain('Holdout EV / Premium')
    expect(wrapper.text()).toContain('Sealed EV / Premium')
    expect(wrapper.text()).toContain('Stress EV / Premium')
    expect(wrapper.text()).toContain('Sealed stress EV / Premium')
    expect(wrapper.text()).toContain('Harsher EV / Premium')
    expect(wrapper.text()).toContain('Sealed harsher EV / Premium')
    expect(wrapper.text()).toContain('Extreme EV / Premium')
    expect(wrapper.text()).toContain('Sealed extreme EV / Premium')
    expect(wrapper.text()).toContain('Bull / Bear')
    expect(wrapper.text()).toContain('6 / 4')
    expect(wrapper.text()).toContain('Search / Holdout / Sealed')
    expect(wrapper.text()).toContain('6 / 2 / 2')
    expect(wrapper.text()).toContain('Candidates')
    expect(wrapper.text()).toContain('9 / 12')
    expect(wrapper.text()).toContain('Exit summaries')
    expect(wrapper.text()).toContain('24 / 24 / 24')
    expect(wrapper.text()).toContain('Run provenance')
    expect(wrapper.text()).toContain('Backend preference')
    expect(wrapper.text()).toContain('auto')
    expect(wrapper.text()).toContain('Dataset provenance')
    expect(wrapper.text()).toContain('massive | minute_aggregate | 1m | close | 2026-04-05T00:00:00Z')
    expect(wrapper.text()).toContain('Filters')
    expect(wrapper.text()).toContain('entry lag 5 | max prem — | min prem — | min vol — | min OI —')
    expect(wrapper.text()).toContain('Sealed finalists')
    expect(wrapper.text()).toContain('Sealed final rank')
    expect(wrapper.text()).toContain('2026-04-05T12:00:00Z')
    expect(wrapper.text()).toContain('hash-1')
    expect(wrapper.text()).toContain('0, 1 | 1, 3 | 15, 30')
    expect(wrapper.text()).toContain('0.60 / 0.20')
    expect(wrapper.text()).toContain('yes')
    expect(wrapper.text()).toContain('0.167')
    expect(wrapper.text()).toContain('0.1000')
    expect(wrapper.text()).toContain('0.0700')
    expect(wrapper.text()).toContain('0.0400')
    expect(wrapper.text()).toContain('0.0300')
    expect(wrapper.text()).toContain('0.0200')
    expect(wrapper.text()).toContain('0.0100')
    expect(wrapper.text()).toContain('-0.0100')
    expect(wrapper.text()).toContain('-0.0200')
  })

  it('supports client-side finalists and robustness filtering in the result table', async () => {
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
      parentArtifactId: 'vectorgrid_run_1',
      optionsDatasetHash: 'hash-1',
      backendUsed: 'gpu_compiled_tensor_hybrid',
      priceSemantics: 'minute_aggregate',
      tupleCount: 3,
      request: null,
      staging: null,
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
          preSealedRank: 2,
          sealedFinalist: false,
          sealedFinalRank: undefined,
          robustScore: 0.18,
          selectionMetrics: {
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.2,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.20,
            medianReturnOnPremium: 0.15,
            winRate: 0.6,
            totalPnlPerContract: 60,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: {
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.2,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.12,
            medianReturnOnPremium: 0.1,
            winRate: 0.6,
            totalPnlPerContract: 50,
          },
          extremeStressMetrics: {
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.2,
            expectancyPerTrade: 2,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0.06,
            winRate: 0.4,
            totalPnlPerContract: 10,
          },
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
          preSealedRank: 1,
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.32,
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
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.2,
            winRate: 0.5,
            totalPnlPerContract: 20,
          },
          stressMetrics: {
            validTrades: 7,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 15,
            expectancyPerPremium: 0.24,
            medianReturnOnPremium: 0.19,
            winRate: 0.71,
            totalPnlPerContract: 105,
          },
          extremeStressMetrics: {
            validTrades: 7,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 8,
            expectancyPerPremium: 0.23,
            medianReturnOnPremium: 0.16,
            winRate: 0.57,
            totalPnlPerContract: 56,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: 'itm_1',
          holdMinutes: 60,
          targetReturn: 1,
          stopReturn: 0.5,
          exitTemplate: 'fixed_hold_or_target_stop',
          fillModel: 'aggregate_research',
          preSealedRank: 3,
          sealedFinalist: false,
          sealedFinalRank: undefined,
          robustScore: 0.12,
          selectionMetrics: {
            validTrades: 4,
            invalidEvents: 2,
            invalidRate: 0.5,
            expectancyPerTrade: 8,
            expectancyPerPremium: 0.09,
            medianReturnOnPremium: 0.05,
            winRate: 0.5,
            totalPnlPerContract: 32,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: {
            validTrades: 4,
            invalidEvents: 2,
            invalidRate: 0.5,
            expectancyPerTrade: 2,
            expectancyPerPremium: -0.02,
            medianReturnOnPremium: -0.01,
            winRate: 0.25,
            totalPnlPerContract: 8,
          },
          extremeStressMetrics: {
            validTrades: 4,
            invalidEvents: 2,
            invalidRate: 0.5,
            expectancyPerTrade: -4,
            expectancyPerPremium: -0.10,
            medianReturnOnPremium: -0.08,
            winRate: 0.0,
            totalPnlPerContract: -16,
          },
        },
      ],
    }

    await wrapper.vm.$nextTick()

    const finalistsOnly = wrapper.find('.toggleField input[type="checkbox"]')
    await finalistsOnly.setValue(true)
    await wrapper.vm.$nextTick()

    let rows = wrapper.findAll('.resultCard > .tableWrap .resultTable tbody tr')
    expect(rows).toHaveLength(1)
    expect(wrapper.text()).toContain('#1')
    expect(wrapper.text()).toContain('0.320')

    await finalistsOnly.setValue(false)
    const maxInvalidInput = wrapper.find('input[placeholder="0.35"]')
    await maxInvalidInput.setValue('0.15')
    await wrapper.vm.$nextTick()

    rows = wrapper.findAll('.resultCard > .tableWrap .resultTable tbody tr')
    expect(rows).toHaveLength(1)
    expect(wrapper.text()).toContain('0.3100')

    await maxInvalidInput.setValue('')
    const maxStressDropInput = wrapper.find('input[placeholder="0.10"]')
    await maxStressDropInput.setValue('0.08')
    await wrapper.vm.$nextTick()

    rows = wrapper.findAll('.resultCard > .tableWrap .resultTable tbody tr')
    expect(rows).toHaveLength(1)
    expect(wrapper.text()).toContain('0.2200')
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
        harsherExitSummaryCount: 24,
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

    const rows = wrapper.findAll('.resultCard > .tableWrap .resultTable tbody tr')
    expect(rows).toHaveLength(2)
    const promotedRow = rows.find((entry) => entry.text().includes('otm_1'))
    expect(promotedRow).toBeTruthy()
    await promotedRow!.trigger('click')
    await wrapper.vm.$nextTick()

    await vi.waitFor(() => {
      expect(wrapper.text()).toContain('base_exit_or_target_stop')
      expect(wrapper.text()).toContain('quote_mid')
      expect(wrapper.text()).toContain('126.0000')
    })
  })

  it('surfaces explicit validation verdicts for selected tuples', async () => {
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
      tupleCount: 4,
      request: null,
      staging: null,
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
          validationVerdict: 'search_only',
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
          exitTemplate: 'fixed_hold_or_target_stop',
          fillModel: 'aggregate_research',
          validationVerdict: 'holdout_supported',
          robustScore: 0.28,
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
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.12,
            medianReturnOnPremium: 0.09,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          sealedEvaluationMetrics: null,
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.10,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: 'itm_1',
          holdMinutes: 45,
          targetReturn: 0.75,
          stopReturn: 0.5,
          exitTemplate: 'base_exit_or_target_stop',
          fillModel: 'quote_mid',
          validationVerdict: 'sealed_finalist',
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.35,
          selectionMetrics: {
            validTrades: 9,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 20,
            expectancyPerPremium: 0.34,
            medianReturnOnPremium: 0.25,
            winRate: 0.73,
            totalPnlPerContract: 180,
          },
          holdoutMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 7,
            expectancyPerPremium: 0.15,
            medianReturnOnPremium: 0.12,
            winRate: 0.67,
            totalPnlPerContract: 21,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.11,
            medianReturnOnPremium: 0.09,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.10,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.09,
            medianReturnOnPremium: 0.07,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          sealedExtremeStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0.06,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
        {
          delayMinutes: 15,
          dteDays: 7,
          moneynessBucket: 'otm_2',
          holdMinutes: 60,
          targetReturn: 1,
          stopReturn: 0.5,
          exitTemplate: 'base_exit_or_target_stop',
          fillModel: 'quote_mid',
          validationVerdict: 'stress_fragile',
          sealedFinalist: true,
          sealedFinalRank: 2,
          robustScore: 0.14,
          selectionMetrics: {
            validTrades: 8,
            invalidEvents: 1,
            invalidRate: 0.1111,
            expectancyPerTrade: 16,
            expectancyPerPremium: 0.28,
            medianReturnOnPremium: 0.19,
            winRate: 0.62,
            totalPnlPerContract: 128,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.09,
            medianReturnOnPremium: 0.06,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.09,
            medianReturnOnPremium: 0.06,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 3,
            expectancyPerPremium: 0.04,
            medianReturnOnPremium: 0.02,
            winRate: 0.5,
            totalPnlPerContract: 6,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 2,
            expectancyPerPremium: 0.03,
            medianReturnOnPremium: 0.02,
            winRate: 0.5,
            totalPnlPerContract: 4,
          },
          sealedExtremeStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: -1,
            expectancyPerPremium: -0.02,
            medianReturnOnPremium: -0.01,
            winRate: 0.5,
            totalPnlPerContract: -2,
          },
        },
      ],
    }

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Selected tuple validation · Search only')
    expect(wrapper.text()).toContain('Validation winners')
    expect(wrapper.text()).toContain('Best sealed EV')
    expect(wrapper.text()).toContain('Best sealed extreme EV')
    expect(wrapper.text()).toContain('Ranked tuples remain search-stage candidates')

    const rows = wrapper.findAll('.resultCard > .tableWrap .resultTable tbody tr')
    expect(rows).toHaveLength(4)

    const holdoutRow = rows.find((entry) => entry.text().includes('otm_1'))
    expect(holdoutRow).toBeTruthy()
    await holdoutRow!.trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('Selected tuple validation · Holdout-supported')

    const sealedRow = rows.find((entry) => entry.text().includes('itm_1'))
    expect(sealedRow).toBeTruthy()
    await sealedRow!.trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('Selected tuple validation · Sealed finalist')
    expect(wrapper.text()).toContain('Promoted into the sealed finalist set (#1)')
    expect(wrapper.text()).toContain('Validation frontier')
    expect(wrapper.text()).toContain('Sealed finalist')

    const fragileRow = rows.find((entry) => entry.text().includes('otm_2'))
    expect(fragileRow).toBeTruthy()
    await fragileRow!.trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('Selected tuple validation · Stress-fragile')
    expect(wrapper.text()).toContain('EV / premium falls from 0.0900 to -0.0200')
  })

  it('surfaces search-winner vs validation-winner disagreement in the current frontier', async () => {
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
      artifactPath: 'C:/runs/vectorgrid_options_expression_disagreement.json',
      artifactTs: '',
      parentArtifactId: '',
      optionsDatasetHash: '',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tupleCount: 3,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: 'C:/runs/vectorgrid_run_1.json',
        optionsDatasetId: 'spy_options',
        rootSymbol: 'SPY',
        totalEvents: 8,
        bullishEvents: 5,
        bearishEvents: 3,
        searchEvents: 4,
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
          validationVerdict: 'search_only',
          robustScore: 0.14,
          selectionMetrics: {
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.1667,
            expectancyPerTrade: 14,
            expectancyPerPremium: 0.42,
            medianReturnOnPremium: 0.18,
            winRate: 0.6,
            totalPnlPerContract: 70,
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
          validationVerdict: 'holdout_supported',
          robustScore: 0.38,
          selectionMetrics: {
            validTrades: 6,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 16,
            expectancyPerPremium: 0.31,
            medianReturnOnPremium: 0.20,
            winRate: 0.66,
            totalPnlPerContract: 96,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 7,
            expectancyPerPremium: 0.24,
            medianReturnOnPremium: 0.12,
            winRate: 0.5,
            totalPnlPerContract: 14,
          },
          sealedEvaluationMetrics: null,
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.20,
            medianReturnOnPremium: 0.11,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: 'itm_1',
          holdMinutes: 45,
          targetReturn: 0.75,
          stopReturn: 0.5,
          exitTemplate: 'base_exit_or_target_stop',
          fillModel: 'quote_mid',
          validationVerdict: 'sealed_finalist',
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.34,
          selectionMetrics: {
            validTrades: 7,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 15,
            expectancyPerPremium: 0.29,
            medianReturnOnPremium: 0.21,
            winRate: 0.71,
            totalPnlPerContract: 105,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.21,
            medianReturnOnPremium: 0.11,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.27,
            medianReturnOnPremium: 0.13,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.16,
            medianReturnOnPremium: 0.09,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
      ],
    }

    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Winner alignment · Winner disagreement')
    expect(wrapper.text()).toContain('Selection winner #1 does not fully survive validation.')
    expect(wrapper.text()).toContain('holdout winner #2')
    expect(wrapper.text()).toContain('sealed winner #3')
    expect(wrapper.text()).toContain('Search baseline')
    expect(wrapper.text()).toContain('Same as robust')
    expect(wrapper.text()).toContain('Different tuple')
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
          entryLagMinutes: 5,
          minutesToExpiryAtEntry: 1440,
          realizedHoldMinutes: 60,
          pathOptionHigh: 1.95,
          pathOptionLow: 1.35,
          pathUnderlierHigh: 503.1,
          pathUnderlierLow: 500.8,
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
          candidateInvalidReason: 'no_expiration_in_bucket',
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
          entryLagMinutes: null,
          minutesToExpiryAtEntry: null,
          realizedHoldMinutes: null,
          pathOptionHigh: null,
          pathOptionLow: null,
          pathUnderlierHigh: null,
          pathUnderlierLow: null,
          exitReason: '',
        },
      ],
    })
    inspectOptionsExpressionEventPathMock.mockResolvedValue({
      tupleIndex: 0,
      eventIndex: 0,
      eventId: 'event-1',
      row: {
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
        candidateInvalidReason: '',
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
        entryLagMinutes: 5,
        minutesToExpiryAtEntry: 1440,
        realizedHoldMinutes: 60,
        pathOptionHigh: 1.95,
        pathOptionLow: 1.35,
        pathUnderlierHigh: 503.1,
        pathUnderlierLow: 500.8,
        exitReason: 'target_hit',
      },
      pointCount: 2,
      points: [
        {
          timestampMs: 460,
          optionOpen: 1.4,
          optionHigh: 1.55,
          optionLow: 1.35,
          optionClose: 1.5,
          underlyingPrice: 501.5,
        },
        {
          timestampMs: 520,
          optionOpen: 1.5,
          optionHigh: 1.95,
          optionLow: 1.48,
          optionClose: 1.8,
          underlyingPrice: 503.1,
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
    await vi.waitFor(() => {
      expect(inspectOptionsExpressionEventPathMock).toHaveBeenCalledWith({
        artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
        tupleIndex: 0,
        eventIndex: 0,
      })
    })

    expect(wrapper.text()).toContain('Tuple event drilldown')
    expect(wrapper.text()).toContain('SPY_C_1')
    expect(wrapper.text()).toContain('target_hit')
    expect(wrapper.text()).toContain('5m / 1440m')
    expect(wrapper.text()).toContain('hold 60m')
    expect(wrapper.text()).toContain('opt 1.3500 -> 1.9500')
    expect(wrapper.text()).toContain('und 500.80 -> 503.10')
    expect(wrapper.text()).toContain('Export inspection CSV')
    expect(wrapper.text()).toContain('candidate_missing')
    expect(wrapper.text()).toContain('no_expiration_in_bucket')
    expect(wrapper.text()).toContain('Showing 2 of 2 event rows')
    expect(wrapper.text()).toContain('Selected event')
    expect(wrapper.text()).toContain('Event path replay')
    expect(wrapper.text()).toContain('Option range')
    expect(wrapper.text()).toContain('1.4000 -> 1.8000')
    expect(wrapper.text()).toContain('501.50 -> 503.10')

    const inspectionRows = wrapper.findAll('.inspectionTable tbody tr')
    expect(inspectionRows).toHaveLength(2)
    await inspectionRows[1]!.trigger('click')

    expect(wrapper.text()).toContain('event-2 / holdout / bearish')
    expect(wrapper.text()).toContain('candidate_missing / no_expiration_in_bucket')
    expect(wrapper.text()).not.toContain('Event path replay')
  })
})
