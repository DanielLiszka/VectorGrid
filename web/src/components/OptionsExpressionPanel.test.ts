
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import OptionsExpressionPanel from "./OptionsExpressionPanel.vue";
import { useOptionsExpressionStore } from "../stores/optionsExpressionStore";

const {
  analyzeOptionsExpressionFeasibilityMock,
  analyzeOptionsExpressionValidationFeasibilityMock,
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
  revealOptionsPathMock,
  runOptionsExpressionMock,
  validateOptionsExpressionArtifactMock,
} = vi.hoisted(() => ({
  analyzeOptionsExpressionFeasibilityMock: vi.fn(),
  analyzeOptionsExpressionValidationFeasibilityMock: vi.fn(),
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
  revealOptionsPathMock: vi.fn(),
  runOptionsExpressionMock: vi.fn(),
  validateOptionsExpressionArtifactMock: vi.fn(),
}));

vi.mock("../services/optionsExpressionService", () => ({
  analyzeOptionsExpressionFeasibility: analyzeOptionsExpressionFeasibilityMock,
  analyzeOptionsExpressionValidationFeasibility:
    analyzeOptionsExpressionValidationFeasibilityMock,
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
  revealOptionsPath: revealOptionsPathMock,
  runOptionsExpression: runOptionsExpressionMock,
  validateOptionsExpressionArtifact: validateOptionsExpressionArtifactMock,
}));

function makeDiagnosticsSummary(overrides: Record<string, unknown> = {}) {
  return {
    parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
    optionsDatasetId: "spy_options",
    rootSymbol: "SPY",
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
    ...overrides,
  };
}

function makeOverlapWindow(overrides: Record<string, unknown> = {}) {
  return {
    contractReferenceAsOfUtc: null,
    totalEventsInDatasetWindow: 0,
    searchEventsInDatasetWindow: 0,
    holdoutEventsInDatasetWindow: 0,
    sealedTestEventsInDatasetWindow: 0,
    totalEventsBeforeDatasetWindow: 0,
    totalEventsAfterDatasetWindow: 0,
    searchEventsBeforeDatasetWindow: 0,
    searchEventsAfterDatasetWindow: 0,
    holdoutEventsBeforeDatasetWindow: 0,
    holdoutEventsAfterDatasetWindow: 0,
    sealedTestEventsBeforeDatasetWindow: 0,
    sealedTestEventsAfterDatasetWindow: 0,
    totalEventsBeforeContractReferenceAsOf: 0,
    searchEventsBeforeContractReferenceAsOf: 0,
    holdoutEventsBeforeContractReferenceAsOf: 0,
    sealedTestEventsBeforeContractReferenceAsOf: 0,
    totalOverlapRate: 0,
    searchOverlapRate: 0,
    holdoutOverlapRate: 0,
    sealedTestOverlapRate: 0,
    ...overrides,
  };
}

function makeCoverageAdequacy(overrides: Record<string, unknown> = {}) {
  return {
    verdict: "unknown" as const,
    nonPromotable: false,
    reasons: [],
    searchCandidateCoverageRate: 0,
    holdoutCandidateCoverageRate: 0,
    sealedTestCandidateCoverageRate: 0,
    searchBaselineOutcomeCoverageRate: 0,
    holdoutBaselineOutcomeCoverageRate: 0,
    sealedTestBaselineOutcomeCoverageRate: 0,
    dominantInvalidReason: "",
    dominantInvalidReasonShare: 0,
    ...overrides,
  };
}

function makeSplitInvalidReasons(overrides: Record<string, unknown> = {}) {
  return {
    searchInvalidReasons: [],
    holdoutInvalidReasons: [],
    sealedTestInvalidReasons: [],
    ...overrides,
  };
}

function makeRunSummary(overrides: Record<string, unknown> = {}) {
  return {
    artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
    artifactTs: "2026-04-05T12:00:00Z",
    parentArtifactId: "vectorgrid_run_1",
    optionsDatasetHash: "hash-1",
    datasetProvenance: null,
    backendUsed: "cpu_reference",
    priceSemantics: "minute_aggregate",
    tupleCount: 0,
    request: null,
    diagnostics: makeDiagnosticsSummary(),
    staging: null,
    overlapWindow: makeOverlapWindow(),
    coverageAdequacy: makeCoverageAdequacy(),
    splitInvalidReasons: makeSplitInvalidReasons(),
    rankedResults: [],
    ...overrides,
  };
}

function makeMetrics(overrides: Record<string, unknown> = {}) {
  return {
    validTrades: 2,
    invalidEvents: 0,
    invalidRate: 0,
    expectancyPerTrade: 10,
    expectancyPerPremium: 0.1,
    medianReturnOnPremium: 0.05,
    winRate: 0.5,
    totalPnlPerContract: 20,
    ...overrides,
  };
}

function makeTuple(overrides: Record<string, unknown> = {}) {
  return {
    delayMinutes: 0,
    dteDays: 1,
    moneynessBucket: "atm",
    holdMinutes: 15,
    targetReturn: 0.25,
    stopReturn: 0.25,
    exitTemplate: "fixed_hold_or_target_stop",
    fillModel: "aggregate_research",
    robustScore: 0.1,
    sealedFinalist: false,
    selectionMetrics: makeMetrics({
      expectancyPerPremium: 0.2,
      expectancyPerTrade: 12,
      totalPnlPerContract: 24,
    }),
    holdoutMetrics: null,
    sealedEvaluationMetrics: null,
    stressMetrics: null,
    sealedStressMetrics: null,
    harsherStressMetrics: null,
    sealedHarsherStressMetrics: null,
    extremeStressMetrics: null,
    sealedExtremeStressMetrics: null,
    ...overrides,
  };
}

function makeArtifactRecord(overrides: Record<string, unknown> = {}) {
  return {
    version: 1,
    kind: "vectorgrid_options_expression_v1",
    ts: "2026-04-05T12:00:00Z",
    artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
    parentArtifactId: "vectorgrid_run_1",
    rootSymbol: "SPY",
    optionsDatasetId: "spy_options",
    optionsDatasetHash: "hash-1",
    datasetProvenance: null,
    backendUsed: "cpu_reference",
    backendFallbackReason: "",
    priceSemantics: "minute_aggregate",
    request: {
      parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
      parentCandidateIndex: null,
      optionsDatasetId: "spy_options",
      splitAssignmentMode: "event_chronological",
      delayMinutes: [0],
      dteBuckets: [1],
      moneynessBuckets: ["atm"],
      holdMinutes: [15],
      targetReturnBuckets: [0.25],
      stopReturnBuckets: [0.25],
      exitTemplate: "fixed_hold_or_target_stop",
      fillModel: "aggregate_research",
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
      sealedFinalistCount: 5,
      backendPreference: "auto",
      structureFamily: "long_single",
    },
    diagnostics: makeDiagnosticsSummary(),
    staging: null,
    overlapWindow: makeOverlapWindow(),
    coverageAdequacy: makeCoverageAdequacy(),
    splitInvalidReasons: makeSplitInvalidReasons(),
    selectionMetrics: [],
    ...overrides,
  };
}

function makeFeasibilitySummary(overrides: Record<string, unknown> = {}) {
  return {
    replayBuildMs: 18,
    totalPreviewMs: 41,
    replayBuildTiming: null,
    diagnostics: makeDiagnosticsSummary(),
    priceSemantics: "minute_aggregate",
    fillModel: "aggregate_research",
    bestAvailableBackend: "cpu_reference",
    autoBackendDecision: {
      recommendedBackend: "cpu_reference",
      gpuEligible: false,
      reasons: [],
    },
    compiledTupleCount: 0,
    perEventCandidateSlots: 0,
    perCandidateExitSummaries: 0,
    candidateSlotCount: 0,
    compiledCandidateCount: 0,
    candidateDensity: 0,
    invalidSlotCount: 0,
    invalidSlotRate: 0,
    baselineExitSummaryCount: 0,
    baselineCompiledExitSummaryCount: 0,
    baselineExitSummaryDensity: 0,
    conservativeExitSummaryCount: 0,
    conservativeCompiledExitSummaryCount: 0,
    conservativeExitSummaryDensity: 0,
    harsherExitSummaryCount: 0,
    harsherCompiledExitSummaryCount: 0,
    harsherExitSummaryDensity: 0,
    projectedEventTupleEvaluations: 0,
    structuralCacheBytesEstimate: 0,
    baselineSummaryBytesEstimate: 0,
    conservativeSummaryBytesEstimate: 0,
    harsherSummaryBytesEstimate: 0,
    timings: {
      tupleCompileMs: 0,
      baselineCandidateCompileMs: 0,
      baselineExitSummaryCompileMs: 0,
      conservativeCandidateCompileMs: 0,
      conservativeExitSummaryCompileMs: 0,
      harsherCandidateCompileMs: 0,
      harsherExitSummaryCompileMs: 0,
      cpuRankingMs: 0,
      cpuRankingUs: 0,
      gpuRankingMs: null,
      gpuRankingUs: null,
      gpuTensorScoreMs: null,
      gpuTensorScoreUs: null,
      gpuExactifyMs: null,
      gpuExactifyUs: null,
      gpuSealedStageMs: null,
      gpuSealedStageUs: null,
      gpuSpeedupVsCpu: null,
    },
    eventCoverage: {
      totalEventsWithCandidates: 0,
      searchEventsWithCandidates: 0,
      holdoutEventsWithCandidates: 0,
      sealedTestEventsWithCandidates: 0,
      totalEventsWithBaselineOutcomes: 0,
      searchEventsWithBaselineOutcomes: 0,
      holdoutEventsWithBaselineOutcomes: 0,
      sealedTestEventsWithBaselineOutcomes: 0,
      totalEventsWithConservativeOutcomes: 0,
      searchEventsWithConservativeOutcomes: 0,
      holdoutEventsWithConservativeOutcomes: 0,
      sealedTestEventsWithConservativeOutcomes: 0,
      totalEventsWithHarsherOutcomes: 0,
      searchEventsWithHarsherOutcomes: 0,
      holdoutEventsWithHarsherOutcomes: 0,
      sealedTestEventsWithHarsherOutcomes: 0,
    },
    overlapWindow: makeOverlapWindow(),
    splitInvalidReasons: makeSplitInvalidReasons(),
    coverageAdequacy: makeCoverageAdequacy(),
    cacheStats: {
      structuralCacheHits: 0,
      structuralCacheMisses: 0,
      structuralCacheEntries: 0,
      summaryCacheHits: 0,
      summaryCacheMisses: 0,
      summaryCacheEntries: 0,
      gpuSharedInputCacheHits: 0,
      gpuSharedInputCacheMisses: 0,
      gpuOutcomeInputCacheHits: 0,
      gpuOutcomeInputCacheMisses: 0,
      gpuOutputBufferReuses: 0,
      gpuOutputBufferAllocations: 0,
      gpuTupleChunkCount: 0,
      gpuTupleChunkSize: 0,
      gpuKernelLaunchCount: 0,
      gpuTupleBudgetBytes: 0,
      gpuOutcomeTensorBytes: 0,
      gpuSharedInputBytesPerChunk: 0,
      gpuOutputBytesPerChunk: 0,
      gpuTotalDeviceBytesPerChunk: 0,
      gpuDeviceTotalMemoryBytes: 0,
      gpuDeviceFreeMemoryBytes: 0,
      gpuDeviceUsableMemoryBytes: 0,
      gpuMemoryBudgetSource: "",
      gpuOutcomeTensorFitsInMemory: false,
      gpuChunkPlanFitsInMemory: false,
      gpuPlanNotes: [],
    },
    invalidReasons: [],
    axisCoverage: {
      delayMinutes: [],
      dteDays: [],
      moneynessBuckets: [],
    },
    ...overrides,
  };
}

describe("OptionsExpressionPanel", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("shows an unsupported message for non-double-like parents", () => {
    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_ml_signal.json",
        parentStrategyKind: "MlSignal",
        parentDatasetLabel: "SPY.csv",
      },
    });

    expect(wrapper.text()).toContain("supports saved DoubleMa");
    expect(wrapper.text()).toContain("BreakoutMomentum");
    expect(wrapper.find("img.emptyStateIllustration").exists()).toBe(true);
    expect(listOptionsDatasetsMock).not.toHaveBeenCalled();
  });

  it("bootstraps datasets and renders the options follow-up controls for DoubleMa parents", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        providerName: "polygon.io",
        providerSourceSurface: "minute_aggregate",
        providerEntitlementProfile: "options_business",
        quoteResolution: "1m",
        priceSemantics: "minute_aggregate",
        calendarId: "market_hours_utc",
        timezone: "UTC",
        coverageStart: 1_700_000_000_000,
        coverageEnd: 1_700_000_120_000,
        contractReferenceSource: "polygon_contract_reference",
        underlierReferenceSource: "underlying_dataset",
        datasetPath: "C:/datasets/spy_options",
        contractReferenceAsOfUtc: "2026-04-05T00:00:00Z",
        contractCount: 3,
        rowCount: 42,
        aggregatePriceField: "close",
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([
      {
        ts: "2026-04-05T12:00:00Z",
        path: "C:/runs/vectorgrid_options_expression_1.json",
        bytes: 2048,
        parentArtifactId: "vectorgrid_run_1",
        parentStrategyKind: "DoubleMa",
        rootSymbol: "SPY",
        optionsDatasetId: "spy_options",
        backendUsed: "cpu_reference",
        backendFallbackReason: "GPU planner probe failed; using CPU.",
        priceSemantics: "minute_aggregate",
        requestBackendPreference: "auto",
        requestFillModel: "aggregate_research",
        requestStructureFamily: "long_single",
        requestSplitAssignmentMode: "event_chronological",
        coverageAdequacyVerdict: "mixed_fragile",
        coverageNonPromotable: true,
        searchOverlapRate: 0.8333,
        holdoutOverlapRate: 1,
        sealedTestOverlapRate: 1,
        searchCandidateCoverageRate: 1,
        holdoutCandidateCoverageRate: 1,
        sealedTestCandidateCoverageRate: 1,
        searchBaselineOutcomeCoverageRate: 0.8,
        holdoutBaselineOutcomeCoverageRate: 1,
        sealedTestBaselineOutcomeCoverageRate: 1,
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
        leaderSummary: {
          searchLeaderTupleIndex: 1,
          robustLeaderTupleIndex: 0,
          holdoutLeaderTupleIndex: 1,
          sealedLeaderTupleIndex: 1,
          stressLeaderTupleIndex: 0,
          distinctLeaderCount: 2,
          searchVsRobustDisagree: true,
          robustVsHoldoutDisagree: true,
          robustVsSealedDisagree: true,
          robustLeaderReachedSealedFinalist: false,
          robustLeaderSealedFinalRank: null,
        },
        topValidationVerdict: "stress_fragile",
        supportedHoldoutTupleCount: 2,
        supportedSealedTupleCount: 1,
        supportedStressTupleCount: 2,
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
        hasHarsherStressMetrics: true,
        hasExtremeStressMetrics: true,
        validatedTupleCount: 2,
        stressFragileTupleCount: 1,
      },
      {
        ts: "2026-04-05T12:30:00Z",
        path: "C:/runs/vectorgrid_options_expression_1_quotes.json",
        bytes: 2048,
        parentArtifactId: "vectorgrid_run_3",
        parentStrategyKind: "DoubleMa",
        rootSymbol: "SPY",
        optionsDatasetId: "spy_quotes",
        backendUsed: "gpu_compiled_tensor",
        priceSemantics: "top_of_book_quote",
        requestBackendPreference: "auto",
        requestFillModel: "quote_conservative",
        requestStructureFamily: "long_single",
        requestSplitAssignmentMode: "event_chronological",
        coverageAdequacyVerdict: "healthy",
        coverageNonPromotable: false,
        searchOverlapRate: 1,
        holdoutOverlapRate: 1,
        sealedTestOverlapRate: 1,
        searchCandidateCoverageRate: 1,
        holdoutCandidateCoverageRate: 1,
        sealedTestCandidateCoverageRate: 1,
        searchBaselineOutcomeCoverageRate: 1,
        holdoutBaselineOutcomeCoverageRate: 1,
        sealedTestBaselineOutcomeCoverageRate: 1,
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
        leaderSummary: {
          searchLeaderTupleIndex: 0,
          robustLeaderTupleIndex: 0,
          holdoutLeaderTupleIndex: 0,
          sealedLeaderTupleIndex: 0,
          stressLeaderTupleIndex: 0,
          distinctLeaderCount: 1,
          searchVsRobustDisagree: false,
          robustVsHoldoutDisagree: false,
          robustVsSealedDisagree: false,
          robustLeaderReachedSealedFinalist: true,
          robustLeaderSealedFinalRank: 1,
        },
        topValidationVerdict: "sealed_finalist",
        supportedHoldoutTupleCount: 2,
        supportedSealedTupleCount: 2,
        supportedStressTupleCount: 2,
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
        hasHarsherStressMetrics: true,
        hasExtremeStressMetrics: true,
        validatedTupleCount: 2,
        stressFragileTupleCount: 0,
      },
      {
        ts: "2026-04-05T13:00:00Z",
        path: "C:/runs/vectorgrid_options_expression_2.json",
        bytes: 2048,
        parentArtifactId: "vectorgrid_run_2",
        parentStrategyKind: "CompositeTrend",
        rootSymbol: "QQQ",
        optionsDatasetId: "qqq_quotes",
        backendUsed: "gpu_compiled_tensor",
        priceSemantics: "top_of_book_quote",
        requestBackendPreference: "gpu_compiled_tensor",
        requestFillModel: "quote_conservative",
        requestStructureFamily: "debit_vertical",
        requestSplitAssignmentMode: "session_chronological",
        coverageAdequacyVerdict: "weak_dataset_overlap",
        coverageNonPromotable: true,
        searchOverlapRate: 0.6667,
        holdoutOverlapRate: 1,
        sealedTestOverlapRate: 1,
        searchCandidateCoverageRate: 1,
        holdoutCandidateCoverageRate: 1,
        sealedTestCandidateCoverageRate: 1,
        searchBaselineOutcomeCoverageRate: 0.75,
        holdoutBaselineOutcomeCoverageRate: 1,
        sealedTestBaselineOutcomeCoverageRate: 1,
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
        leaderSummary: {
          searchLeaderTupleIndex: 2,
          robustLeaderTupleIndex: 0,
          holdoutLeaderTupleIndex: 1,
          sealedLeaderTupleIndex: 0,
          stressLeaderTupleIndex: 0,
          distinctLeaderCount: 3,
          searchVsRobustDisagree: true,
          robustVsHoldoutDisagree: true,
          robustVsSealedDisagree: false,
          robustLeaderReachedSealedFinalist: true,
          robustLeaderSealedFinalRank: 2,
        },
        topValidationVerdict: "sealed_finalist",
        supportedHoldoutTupleCount: 3,
        supportedSealedTupleCount: 2,
        supportedStressTupleCount: 3,
        hasHoldoutMetrics: true,
        hasSealedEvaluationMetrics: true,
        hasStressMetrics: true,
        hasHarsherStressMetrics: true,
        hasExtremeStressMetrics: true,
        validatedTupleCount: 3,
        stressFragileTupleCount: 0,
      },
    ]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    expect(wrapper.text()).toContain("Options Expression");
    expect(wrapper.text()).toContain("This is an options follow-up surface.");
    expect(wrapper.find("img.optionsPaneGlyph").exists()).toBe(true);
    expect(
      wrapper.findAll("img.optionsActionIcon").length,
    ).toBeGreaterThanOrEqual(4);
    expect(wrapper.text()).toContain("Optimize expression");
    expect(wrapper.text()).toContain("Realism lane");
    expect(wrapper.text()).toContain("Split assignment");
    expect(wrapper.find('option[value="session_chronological"]').exists()).toBe(
      true,
    );
    expect(wrapper.text()).toContain("Polygon.io 1m options download");
    expect(wrapper.text()).toContain("Subsequent validation");
    expect(wrapper.text()).toContain("Validate artifact");
    expect(wrapper.find('option[value="date_range_holdout"]').exists()).toBe(
      true,
    );
    expect(wrapper.find('option[value="anchored_walk_forward"]').exists()).toBe(
      true,
    );
    expect(
      wrapper.find('option[value="rolling_session_walk_forward"]').exists(),
    ).toBe(true);
    expect(wrapper.text()).toContain("Aggregate lane");
    expect(wrapper.text()).toContain(
      "only support aggregate_research in the backend",
    );
    expect(wrapper.find("#optionsRealismLane").exists()).toBe(true);
    expect(wrapper.find("#optionsImportPriceSemantics").exists()).toBe(true);
    expect(wrapper.text()).toContain(
      "The selected realism lane controls the expected CSV semantics for this import.",
    );
    expect(
      wrapper.find('option[value="quote_conservative"]').attributes("disabled"),
    ).toBeDefined();
    expect(
      wrapper.find('option[value="quote_mid"]').attributes("disabled"),
    ).toBeDefined();
    expect(wrapper.text()).toContain("Lane product status");
    expect(wrapper.text()).toContain("Saved datasets");
    expect(wrapper.text()).toContain("Saved artifacts");
    expect(wrapper.text()).toContain("Avg validation EV");
    expect(wrapper.text()).toContain("Avg stressed EV");
    expect(wrapper.text()).toContain(
      "This download is options-only and stays separate from the normal stock data workflow.",
    );
    expect(wrapper.text()).toContain("Dataset manifest");
    expect(wrapper.text()).toContain("options_business");
    expect(wrapper.text()).toContain("C:/datasets/spy_options");
    expect(wrapper.text()).toContain("spy_options");
    expect(wrapper.text()).toContain(
      "polygon.io | minute_aggregate | 1m | close | 2026-04-05T00:00:00Z",
    );
    expect(wrapper.text()).toContain("Saved artifact comparison");
    expect(wrapper.text()).toContain("Artifact frontier leaders");
    expect(wrapper.text()).toContain("Options artifact meta-analysis");
    expect(wrapper.text()).toContain("Reopened artifact validation frontier");
    expect(wrapper.text()).toContain("By parent family");
    expect(wrapper.text()).toContain("CompositeTrend");
    expect(wrapper.text()).toContain("By root");
    expect(wrapper.text()).toContain("QQQ");
    expect(wrapper.text()).toContain("By structure family");
    expect(wrapper.text()).toContain("debit_vertical");
    expect(wrapper.text()).toContain("By realism lane");
    expect(wrapper.text()).toContain("Quote-aware · quote_conservative");
    expect(wrapper.text()).toContain("Realism lane disagreement");
    expect(wrapper.text()).toContain("SPY · DoubleMa · long_single");
    expect(wrapper.text()).toContain("Aggregate -> Quote-aware");
    expect(wrapper.text()).toContain("Lane");
    expect(wrapper.text()).toContain("Validation");
    expect(wrapper.text()).toContain("Adequacy");
    expect(wrapper.text()).toContain("Overlap");
    expect(wrapper.text()).toContain("Validated %");
    expect(wrapper.text()).toContain("Stress %");
    expect(wrapper.text()).toContain("Val Δ");
    expect(wrapper.text()).toContain("Stress Δ");
    expect(wrapper.text()).toContain("Diff vs selected");
    expect(wrapper.text()).toContain("auto -> cpu_reference");
    expect(wrapper.text()).toContain("gpu_compiled_tensor");
    expect(wrapper.text()).toContain("Aggregate lane");
    expect(wrapper.text()).toContain("aggregate_research");
    expect(wrapper.text()).toContain("validation Stress-fragile");
    expect(wrapper.text()).toContain("2 retained leaders");
    expect(wrapper.text()).toContain(
      "Sel #2 · Robust #1 · Hold #2 · Sealed #2 · Stress #1",
    );
    expect(wrapper.text()).toContain(
      "Sel≠Robust · Robust≠Hold · Robust≠Sealed",
    );
    expect(wrapper.text()).toContain(
      "Robust leader did not reach the sealed finalist set",
    );
    expect(wrapper.text()).toContain("quote_conservative");
    expect(wrapper.text()).toContain("Mixed fragile · no-promote");
    expect(wrapper.text()).toContain("83.3% / 100.0% / 100.0%");
    expect(wrapper.text()).toContain("robust 0.180");
    expect(wrapper.text()).toContain("hold 0.100");
    expect(wrapper.text()).toContain("sealed 0.070");
    expect(wrapper.text()).toContain("stress 0.040");
    expect(wrapper.text()).toContain("harsher 0.020");
    expect(wrapper.text()).toContain("sealed harsher 0.010");
    expect(wrapper.text()).toContain("extreme -0.010");
    expect(wrapper.text()).toContain("sealed extreme -0.020");
    expect(wrapper.text()).toContain("Validated ratio");
    expect(wrapper.text()).toContain("Fragility ratio");
    expect(wrapper.text()).toContain(
      "Hold 2 / 2 (100.0%) · Sealed 1 / 2 (50.0%) · Stress 2 / 2 (100.0%)",
    );
    expect(wrapper.text()).toContain("Selection");
    expect(wrapper.text()).toContain("Holdout");
    expect(wrapper.text()).toContain("Best validated artifact");
    expect(wrapper.text()).toContain("Leadership");
    expect(wrapper.text()).toContain("Disagree");
    expect(useOptionsExpressionStore().splitAssignmentMode).toBe(
      "event_chronological",
    );
  });

  it("shows validation plan controls for date-range and walk-forward modes", async () => {
    listOptionsDatasetsMock.mockResolvedValue([]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });
    const store = useOptionsExpressionStore();

    store.validationPlanMode = "date_range_holdout";
    await wrapper.vm.$nextTick();
    expect(
      wrapper.find('[data-qa-control="options.validation.startDate"]').exists(),
    ).toBe(true);
    expect(
      wrapper.find('[data-qa-control="options.validation.endDate"]').exists(),
    ).toBe(true);

    store.validationPlanMode = "rolling_session_walk_forward";
    await wrapper.vm.$nextTick();
    expect(
      wrapper.find('[data-qa-control="options.validation.startDate"]').exists(),
    ).toBe(false);
    expect(
      wrapper
        .find('[data-qa-control="options.validation.trainSessions"]')
        .exists(),
    ).toBe(true);
    expect(
      wrapper
        .find('[data-qa-control="options.validation.validationSessions"]')
        .exists(),
    ).toBe(true);
    expect(
      wrapper.find('[data-qa-control="options.validation.stepSessions"]').exists(),
    ).toBe(true);
  });

  it("switches the workflow into the quote-aware lane and disables direct provider download", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
      {
        optionsDatasetId: "spy_quotes",
        rootSymbol: "SPY",
        priceSemantics: "top_of_book_quote",
        contractCount: 2,
        rowCount: 24,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
    });

    await wrapper.find("#optionsRealismLane").setValue("quote_aware");

    const downloadButton = wrapper
      .findAll("button")
      .find((entry) => entry.text() === "Download Polygon.io 1m data");
    expect(downloadButton).toBeTruthy();
    expect(downloadButton!.attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain(
      "Quote-aware realism is a first-class lane with CSV import for new dataset acquisition today.",
    );
    expect(wrapper.text()).toContain(
      "Direct provider download in this panel is currently available only for the Aggregate lane.",
    );
    expect(wrapper.text()).toContain(
      "Select Quote-aware Polygon.io dataset…",
    );
    expect(wrapper.text()).toContain("Lane product status");
    expect(wrapper.text()).toContain("Saved datasets");
    expect(wrapper.text()).toContain(
      "No saved artifacts currently match the selected Quote-aware lane for this parent run.",
    );
  });

  it("switches the workflow into the derived-mid lane and keeps import-first acquisition guidance", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
      {
        optionsDatasetId: "spy_mid",
        rootSymbol: "SPY",
        priceSemantics: "derived_mid_snapshot",
        contractCount: 2,
        rowCount: 24,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
    });

    await wrapper.find("#optionsRealismLane").setValue("derived_mid");

    const downloadButton = wrapper
      .findAll("button")
      .find((entry) => entry.text() === "Download Polygon.io 1m data");
    expect(downloadButton).toBeTruthy();
    expect(downloadButton!.attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain(
      "Derived-mid realism is a first-class lane with CSV import for new dataset acquisition today.",
    );
    expect(wrapper.text()).toContain(
      "Direct provider download in this panel is currently available only for the Aggregate lane.",
    );
    expect(wrapper.text()).toContain(
      "Select Derived-mid Polygon.io dataset…",
    );
    expect(wrapper.text()).toContain(
      "only support aggregate_research in the backend",
    );
    expect(wrapper.text()).toContain("Lane product status");
  });

  it("does not borrow selected dataset provenance when a reopened artifact dataset is missing locally", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        providerName: "polygon.io",
        providerSourceSurface: "minute_aggregate",
        providerEntitlementProfile: "options_business",
        quoteResolution: "1m",
        priceSemantics: "minute_aggregate",
        calendarId: "market_hours_utc",
        timezone: "UTC",
        coverageStart: 1_700_000_000_000,
        coverageEnd: 1_700_000_120_000,
        contractReferenceSource: "polygon_contract_reference",
        contractReferenceAsOfUtc: "2026-04-05T00:00:00Z",
        underlierReferenceSource: "underlying_dataset",
        datasetPath: "C:/datasets/spy_options",
        contractCount: 3,
        rowCount: 42,
        aggregatePriceField: "close",
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.activeArtifact = {
      ...makeArtifactRecord({
        artifactPath:
          "C:/runs/vectorgrid_options_expression_missing_quotes.json",
        optionsDatasetId: "spy_quotes_missing",
        priceSemantics: "top_of_book_quote",
        request: {
          parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
          parentCandidateIndex: null,
          optionsDatasetId: "spy_quotes_missing",
          splitAssignmentMode: "event_chronological",
          delayMinutes: [0],
          dteBuckets: [1],
          moneynessBuckets: ["atm"],
          holdMinutes: [15],
          targetReturnBuckets: [0.25],
          stopReturnBuckets: [0.25],
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "quote_conservative",
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
          sealedFinalistCount: 5,
          backendPreference: "auto",
          structureFamily: "long_single",
        },
        diagnostics: {
          parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
          optionsDatasetId: "spy_quotes_missing",
          rootSymbol: "SPY",
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
      }),
    };

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("manifest unavailable locally");
  });

  it("states explicit runtime semantics for reopened quote-aware and derived-mid artifacts", async () => {
    listOptionsDatasetsMock.mockResolvedValue([]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord({
      artifactPath: "C:/runs/vectorgrid_options_expression_quotes.json",
      optionsDatasetId: "spy_quotes",
      priceSemantics: "top_of_book_quote",
      request: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentCandidateIndex: null,
        optionsDatasetId: "spy_quotes",
        splitAssignmentMode: "event_chronological",
        delayMinutes: [0],
        dteBuckets: [1],
        moneynessBuckets: ["atm"],
        holdMinutes: [15],
        targetReturnBuckets: [0.25],
        stopReturnBuckets: [0.25],
        exitTemplate: "fixed_hold_or_target_stop",
        fillModel: "quote_conservative",
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
        sealedFinalistCount: 5,
        backendPreference: "auto",
        structureFamily: "long_single",
      },
      selectionMetrics: [],
    });
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain(
      "only quote-side execution lane currently surfaced in the options workflow",
    );
    expect(wrapper.text()).toContain(
      "new dataset acquisition remains CSV import-first today",
    );

    store.activeArtifact = makeArtifactRecord({
      artifactPath: "C:/runs/vectorgrid_options_expression_mid.json",
      optionsDatasetId: "spy_mid",
      priceSemantics: "derived_mid_snapshot",
      request: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentCandidateIndex: null,
        optionsDatasetId: "spy_mid",
        splitAssignmentMode: "event_chronological",
        delayMinutes: [0],
        dteBuckets: [1],
        moneynessBuckets: ["atm"],
        holdMinutes: [15],
        targetReturnBuckets: [0.25],
        stopReturnBuckets: [0.25],
        exitTemplate: "fixed_hold_or_target_stop",
        fillModel: "aggregate_research",
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
        sealedFinalistCount: 5,
        backendPreference: "auto",
        structureFamily: "long_single",
      },
      selectionMetrics: [],
    });
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain(
      "runtime execution remains aggregate-style and non-quote-side",
    );
  });

  it("downloads a bounded Polygon.io dataset from the options panel", async () => {
    listOptionsDatasetsMock.mockResolvedValue([]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);
    downloadPolygonOptionsDatasetMock.mockResolvedValue({
      manifest: {
        optionsDatasetId: "spy_options_1m",
        rootSymbol: "SPY",
        providerName: "polygon.io",
        providerSourceSurface: "minute_aggregate",
        quoteResolution: "1m",
        priceSemantics: "minute_aggregate",
        contractReferenceAsOfUtc: "2026-04-06T23:59:00Z",
        contractCount: 8,
        rowCount: 200,
        aggregatePriceField: "close",
      },
      contractsDiscovered: 22,
      contractsSelected: 8,
      contractsWithRows: 6,
      downloadedOptionRows: 200,
      skippedContracts: 2,
      underlyingCsvPath: "C:/cache/polygon_stocks_raw_SPY_1m.csv",
      usedUnderlyingCacheOnly: true,
    });

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const inputs = wrapper.findAll('input[type="date"]');
    expect(inputs.length).toBeGreaterThanOrEqual(4);
    await inputs[0]!.setValue("2026-04-01");
    await inputs[1]!.setValue("2026-04-02");

    await wrapper.find("#optionsDownloadContractType").setValue("put");

    const downloadButton = wrapper
      .findAll("button")
      .find((entry) => entry.text() === "Download Polygon.io 1m data");
    expect(downloadButton).toBeTruthy();
    await downloadButton!.trigger("click");

    await vi.waitFor(() => {
      expect(downloadPolygonOptionsDatasetMock).toHaveBeenCalledWith(
        expect.objectContaining({
          rootSymbol: "SPY",
          from: "2026-04-01",
          to: "2026-04-02",
          contractType: "put",
        }),
      );
    });

    const store = useOptionsExpressionStore();
    await vi.waitFor(() => {
      expect(store.statusLine).toContain(
        "Downloaded Polygon.io options dataset",
      );
    });
    expect(store.latestDatasetDownload?.underlyingCsvPath).toBe(
      "C:/cache/polygon_stocks_raw_SPY_1m.csv",
    );
    expect(wrapper.text()).toContain("Latest download audit");
    expect(wrapper.text()).toContain("cache-only reuse");
    expect(wrapper.text()).toContain("C:/cache/polygon_stocks_raw_SPY_1m.csv");
  });

  it("boots once the parent strategy becomes a supported double-like family after mount", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "",
        parentDatasetLabel: "",
      },
    });

    expect(wrapper.text()).toContain("Saved run not supported yet.");
    expect(listOptionsDatasetsMock).not.toHaveBeenCalled();

    await wrapper.setProps({
      parentStrategyKind: "CompositeTrend",
      parentDatasetLabel: "SPY.csv",
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    expect(wrapper.text()).toContain("Options Expression");
    expect(wrapper.text()).toContain("spy_options");
  });

  it("exports active ranked tuples from the current result card", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);
    exportOptionsExpressionCsvMock.mockResolvedValue({
      mode: "saved",
      filename: "vectorgrid_run_1_options_expression.csv",
      path: "C:/exports/vectorgrid_run_1_options_expression.csv",
      bytes: 512,
      revealed: true,
    });

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
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
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
    });

    await wrapper.vm.$nextTick();

    const exportButton = wrapper
      .findAll("button")
      .find((entry) => entry.text() === "Export CSV");
    expect(exportButton).toBeTruthy();
    await exportButton!.trigger("click");

    await vi.waitFor(() => {
      expect(exportOptionsExpressionCsvMock).toHaveBeenCalledWith(
        expect.objectContaining({
          filenameStem: "vectorgrid_run_1_options_expression",
          rootSymbol: "SPY",
        }),
      );
    });
  });

  it("shows backend fallback details in the request summary when available", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "2026-04-07T14:00:00Z",
      parentArtifactId: "vectorgrid_run_1",
      optionsDatasetHash: "hash-1",
      datasetProvenance: {
        providerName: "polygon.io",
        providerSourceSurface: "options_aggs",
        quoteResolution: "top_of_book",
        aggregatePriceField: "mid",
        contractReferenceAsOfUtc: "2026-04-05T00:00:00Z",
        quoteQuality: {
          priceSemantics: "top_of_book_quote",
          rowCount: 100,
          contractCount: 2,
          anyQuoteRowCount: 80,
          bidRowCount: 80,
          askRowCount: 80,
          bidAskRowCount: 75,
          midRowCount: 80,
          quoteAgeRowCount: 70,
          staleQuoteThresholdMs: 60000,
          staleQuoteRowCount: 5,
          zeroBidRowCount: 1,
          lockedBidAskRowCount: 2,
          crossedBidAskRowCount: 1,
          invalidBidAskRowCount: 3,
          anyQuoteCoverageRate: 0.8,
          bidAskCoverageRate: 0.75,
          midCoverageRate: 0.8,
          staleQuoteRate: 0.05,
          meanSpreadBps: 12.5,
          medianSpreadBps: 10.5,
          maxSpreadBps: 40,
          maxQuoteAgeMs: 120000,
          perContract: [],
          warnings: ["stale_quote_rows_present"],
        },
      },
      backendUsed: "cpu_reference",
      backendFallbackReason: "GPU planner probe failed; using CPU.",
      priceSemantics: "minute_aggregate",
      tupleCount: 1,
      request: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentCandidateIndex: null,
        optionsDatasetId: "spy_options",
        delayMinutes: [0, 1],
        dteBuckets: [1, 3],
        moneynessBuckets: ["atm"],
        holdMinutes: [15],
        targetReturnBuckets: [0.25],
        stopReturnBuckets: [0.25],
        exitTemplate: "fixed_hold_or_target_stop",
        fillModel: "aggregate_research",
        filters: {
          maxEntryLagMinutes: 5,
          maxPremium: null,
          minPremium: null,
          minVolume: null,
          minOpenInterest: null,
        },
        backendPreference: "auto",
        splitAssignmentMode: "event_chronological",
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
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Backend detail");
    expect(wrapper.text()).toContain("GPU planner probe failed; using CPU.");
    expect(wrapper.text()).toContain("polygon.io");
    expect(wrapper.text()).toContain("Bid/ask coverage");
    expect(wrapper.text()).toContain("75.0%");
    expect(wrapper.text()).toContain("stale_quote_rows_present");
  });

  it("renders the feasibility preview summary when available", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestFeasibility = makeFeasibilitySummary({
      replayBuildMs: 18,
      totalPreviewMs: 41,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
      priceSemantics: "minute_aggregate",
      fillModel: "aggregate_research",
      bestAvailableBackend: "gpu_compiled_tensor_hybrid",
      autoBackendDecision: {
        recommendedBackend: "cpu_reference",
        gpuEligible: false,
        reasons: [
          "Measured GPU hybrid speedup 2.00x is below the current promotion gate of 3.0x.",
        ],
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
      overlapWindow: makeOverlapWindow({
        contractReferenceAsOfUtc: "2026-04-05T00:00:00Z",
        totalEventsInDatasetWindow: 8,
        searchEventsInDatasetWindow: 4,
        holdoutEventsInDatasetWindow: 2,
        sealedTestEventsInDatasetWindow: 2,
        totalEventsBeforeDatasetWindow: 2,
        searchEventsBeforeDatasetWindow: 2,
        totalOverlapRate: 0.8,
        searchOverlapRate: 0.6667,
        holdoutOverlapRate: 1,
        sealedTestOverlapRate: 1,
        totalEventsBeforeContractReferenceAsOf: 1,
        searchEventsBeforeContractReferenceAsOf: 1,
      }),
      splitInvalidReasons: makeSplitInvalidReasons({
        searchInvalidReasons: [
          { reason: "entry_past_cutoff", count: 10 },
          { reason: "premium_too_high", count: 8 },
        ],
        holdoutInvalidReasons: [{ reason: "premium_too_high", count: 4 }],
        sealedTestInvalidReasons: [
          { reason: "no_expiration_in_bucket", count: 3 },
        ],
      }),
      coverageAdequacy: makeCoverageAdequacy({
        verdict: "weak_dataset_overlap",
        nonPromotable: true,
        reasons: [
          "Only 66.7% of search events and 80.0% of total events overlap the current dataset window.",
        ],
        searchCandidateCoverageRate: 1,
        holdoutCandidateCoverageRate: 1,
        sealedTestCandidateCoverageRate: 1,
        searchBaselineOutcomeCoverageRate: 0.8,
        holdoutBaselineOutcomeCoverageRate: 1,
        sealedTestBaselineOutcomeCoverageRate: 1,
        dominantInvalidReason: "entry_past_cutoff",
        dominantInvalidReasonShare: 0.5,
      }),
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
        gpuMemoryBudgetSource: "free_memory_headroom",
        gpuOutcomeTensorFitsInMemory: true,
        gpuChunkPlanFitsInMemory: true,
        gpuPlanNotes: [
          "GPU planner probe failed before ranking: synthetic test note",
        ],
      },
      invalidReasons: [
        { reason: "no_expiration_in_bucket", count: 20 },
        { reason: "premium_too_high", count: 16 },
      ],
      axisCoverage: {
        delayMinutes: [
          {
            delayMinutes: 0,
            slotCount: 30,
            validSlotCount: 24,
            invalidSlotCount: 6,
            validRate: 0.8,
            invalidReasons: [{ reason: "premium_too_high", count: 6 }],
          },
          {
            delayMinutes: 5,
            slotCount: 30,
            validSlotCount: 18,
            invalidSlotCount: 12,
            validRate: 0.6,
            invalidReasons: [{ reason: "entry_past_cutoff", count: 8 }],
          },
          {
            delayMinutes: 15,
            slotCount: 30,
            validSlotCount: 0,
            invalidSlotCount: 30,
            validRate: 0,
            invalidReasons: [{ reason: "entry_past_cutoff", count: 22 }],
          },
        ],
        dteDays: [
          {
            dteDays: 1,
            slotCount: 30,
            validSlotCount: 24,
            invalidSlotCount: 6,
            validRate: 0.8,
            invalidReasons: [{ reason: "premium_too_high", count: 6 }],
          },
          {
            dteDays: 3,
            slotCount: 30,
            validSlotCount: 18,
            invalidSlotCount: 12,
            validRate: 0.6,
            invalidReasons: [{ reason: "premium_too_high", count: 10 }],
          },
          {
            dteDays: 5,
            slotCount: 30,
            validSlotCount: 0,
            invalidSlotCount: 30,
            validRate: 0,
            invalidReasons: [{ reason: "no_expiration_in_bucket", count: 20 }],
          },
        ],
        moneynessBuckets: [
          {
            moneynessBucket: "atm",
            slotCount: 30,
            validSlotCount: 24,
            invalidSlotCount: 6,
            validRate: 0.8,
            invalidReasons: [{ reason: "premium_too_high", count: 6 }],
          },
          {
            moneynessBucket: "otm_1",
            slotCount: 30,
            validSlotCount: 18,
            invalidSlotCount: 12,
            validRate: 0.6,
            invalidReasons: [{ reason: "premium_too_high", count: 10 }],
          },
          {
            moneynessBucket: "itm_1",
            slotCount: 30,
            validSlotCount: 12,
            invalidSlotCount: 18,
            validRate: 0.4,
            invalidReasons: [{ reason: "premium_too_high", count: 8 }],
          },
        ],
      },
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Phase 0.5 preview");
    expect(wrapper.text()).toContain("54 / 90 (60.0%)");
    expect(wrapper.text()).toContain("18 ms / 41 ms");
    expect(wrapper.text()).toContain("gpu_compiled_tensor_hybrid");
    expect(wrapper.text()).toContain(
      "cpu 6.400 ms / gpu hybrid 3.100 ms (2.00x)",
    );
    expect(wrapper.text()).toContain(
      "tensor 1.200 ms · exactify 1.100 ms · sealed 0.800 ms",
    );
    expect(wrapper.text()).toContain("GPU viability");
    expect(wrapper.text()).toContain(
      "Coverage Weak dataset overlap · non-promotable",
    );
    expect(wrapper.text()).toContain(
      "Only 66.7% of search events and 80.0% of total events overlap the current dataset window.",
    );
    expect(wrapper.text()).toContain("Candidate density >= 30%");
    expect(wrapper.text()).toContain("GPU hybrid speedup >= 3.0x");
    expect(wrapper.text()).toContain("Auto backend");
    expect(wrapper.text()).toContain("cpu_reference · CPU-favored");
    expect(wrapper.text()).toContain("74.1% / 70.4% / 66.7%");
    expect(wrapper.text()).toContain("struct 2/2 · summary 2/2");
    expect(wrapper.text()).toContain("shared 1/1 · outcome 1/1 · out 2/1");
    expect(wrapper.text()).toContain("chunks 3×4096 · launches 6");
    expect(wrapper.text()).toContain(
      "free_memory_headroom · free 512.00 MiB · usable 256.00 MiB · budget 4.00 MiB",
    );
    expect(wrapper.text()).toContain("outcome fits · chunk fits");
    expect(wrapper.text()).toContain("Overlap adequacy");
    expect(wrapper.text()).toContain("Split invalid reasons");
    expect(wrapper.text()).toContain("Events in dataset window");
    expect(wrapper.text()).toContain("8 / 10 (80.0%)");
    expect(wrapper.text()).toContain("Search invalid reasons");
    expect(wrapper.text()).toContain("entry_past_cutoff");
    expect(wrapper.text()).toContain("no_expiration_in_bucket (20)");
    expect(wrapper.text()).toContain("Bucket coverage");
    expect(wrapper.text()).toContain("Delay buckets");
    expect(wrapper.text()).toContain("DTE buckets");
    expect(wrapper.text()).toContain("15m");
    expect(wrapper.text()).toContain("5d");
    expect(wrapper.text()).toContain("0 / 30");
    expect(wrapper.text()).toContain("Recommendations");
    expect(wrapper.text()).toContain(
      "GPU planner probe failed before ranking: synthetic test note",
    );
    expect(wrapper.text()).toContain(
      "Reduce the default DTE spread for this dataset",
    );
    expect(wrapper.text()).toContain(
      "These DTE buckets are producing zero valid slots on the current dataset: 5d",
    );
    expect(wrapper.text()).toContain(
      "Late-entry pressure is concentrated in these delay buckets: 5m, 15m",
    );
  });

  it("shows holdout, sealed, stress, and invalid-rate reporting when those metrics are present", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        providerName: "polygon.io",
        providerSourceSurface: "minute_aggregate",
        quoteResolution: "1m",
        priceSemantics: "minute_aggregate",
        contractReferenceAsOfUtc: "2026-04-05T00:00:00Z",
        aggregatePriceField: "close",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "2026-04-05T12:00:00Z",
      parentArtifactId: "vectorgrid_run_1",
      optionsDatasetHash: "hash-1",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 1,
      request: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentCandidateIndex: null,
        optionsDatasetId: "spy_options",
        splitAssignmentMode: "session_chronological",
        delayMinutes: [0, 1],
        dteBuckets: [1, 3],
        moneynessBuckets: ["atm", "otm_1"],
        holdMinutes: [15, 30],
        targetReturnBuckets: [0.25],
        stopReturnBuckets: [0.25],
        exitTemplate: "fixed_hold_or_target_stop",
        fillModel: "aggregate_research",
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
        backendPreference: "auto",
      },
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
        harsherExitSummaryCount: 24,
      },
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
    });

    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("session_chronological | 0.60 / 0.20");

    expect(wrapper.text()).toContain("Invalid %");
    expect(wrapper.text()).toContain("Holdout EV / Premium");
    expect(wrapper.text()).toContain("Sealed EV / Premium");
    expect(wrapper.text()).toContain("Stress EV / Premium");
    expect(wrapper.text()).toContain("Sealed stress EV / Premium");
    expect(wrapper.text()).toContain("Harsher EV / Premium");
    expect(wrapper.text()).toContain("Sealed harsher EV / Premium");
    expect(wrapper.text()).toContain("Extreme EV / Premium");
    expect(wrapper.text()).toContain("Sealed extreme EV / Premium");
    expect(wrapper.text()).toContain("Bull / Bear");
    expect(wrapper.text()).toContain("6 / 4");
    expect(wrapper.text()).toContain("Search / Holdout / Sealed");
    expect(wrapper.text()).toContain("6 / 2 / 2");
    expect(wrapper.text()).toContain("Candidates");
    expect(wrapper.text()).toContain("9 / 12");
    expect(wrapper.text()).toContain("Exit summaries");
    expect(wrapper.text()).toContain("24 / 24 / 24");
    expect(wrapper.text()).toContain("Run provenance");
    expect(wrapper.text()).toContain("Backend preference");
    expect(wrapper.text()).toContain("auto");
    expect(wrapper.text()).toContain("Dataset provenance");
    expect(wrapper.text()).toContain(
      "polygon.io | minute_aggregate | 1m | close | 2026-04-05T00:00:00Z",
    );
    expect(wrapper.text()).toContain("Filters");
    expect(wrapper.text()).toContain(
      "entry lag 5 | max prem — | min prem — | min vol — | min OI —",
    );
    expect(wrapper.text()).toContain("Sealed finalists");
    expect(wrapper.text()).toContain("Sealed final rank");
    expect(wrapper.text()).toContain("2026-04-05T12:00:00Z");
    expect(wrapper.text()).toContain("hash-1");
    expect(wrapper.text()).toContain("0, 1 | 1, 3 | 15, 30");
    expect(wrapper.text()).toContain("0.60 / 0.20");
    expect(wrapper.text()).toContain("yes");
    expect(wrapper.text()).toContain("0.167");
    expect(wrapper.text()).toContain("0.1000");
    expect(wrapper.text()).toContain("0.0700");
    expect(wrapper.text()).toContain("0.0400");
    expect(wrapper.text()).toContain("0.0300");
    expect(wrapper.text()).toContain("0.0200");
    expect(wrapper.text()).toContain("0.0100");
    expect(wrapper.text()).toContain("-0.0100");
    expect(wrapper.text()).toContain("-0.0200");
  });

  it("supports client-side finalists and robustness filtering in the result table", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "vectorgrid_run_1",
      optionsDatasetHash: "hash-1",
      backendUsed: "gpu_compiled_tensor_hybrid",
      priceSemantics: "minute_aggregate",
      tupleCount: 3,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          preSealedRank: 2,
          sealedFinalist: false,
          sealedFinalRank: undefined,
          robustScore: 0.18,
          selectionMetrics: {
            validTrades: 5,
            invalidEvents: 1,
            invalidRate: 0.2,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.2,
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
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
          moneynessBucket: "itm_1",
          holdMinutes: 60,
          targetReturn: 1,
          stopReturn: 0.5,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
            expectancyPerPremium: -0.1,
            medianReturnOnPremium: -0.08,
            winRate: 0.0,
            totalPnlPerContract: -16,
          },
        },
      ],
    });

    await wrapper.vm.$nextTick();

    const finalistsOnly = wrapper.find('.toggleField input[type="checkbox"]');
    await finalistsOnly.setValue(true);
    await wrapper.vm.$nextTick();

    let rows = wrapper.findAll(
      ".resultCard > .tableWrap .resultTable tbody tr",
    );
    expect(rows).toHaveLength(1);
    expect(wrapper.text()).toContain("#1");
    expect(wrapper.text()).toContain("0.320");

    await finalistsOnly.setValue(false);
    const maxInvalidInput = wrapper.find('input[placeholder="0.35"]');
    await maxInvalidInput.setValue("0.15");
    await wrapper.vm.$nextTick();

    rows = wrapper.findAll(".resultCard > .tableWrap .resultTable tbody tr");
    expect(rows).toHaveLength(1);
    expect(wrapper.text()).toContain("0.3100");

    await maxInvalidInput.setValue("");
    const maxStressDropInput = wrapper.find('input[placeholder="0.10"]');
    await maxStressDropInput.setValue("0.08");
    await wrapper.vm.$nextTick();

    rows = wrapper.findAll(".resultCard > .tableWrap .resultTable tbody tr");
    expect(rows).toHaveLength(1);
    expect(wrapper.text()).toContain("0.2200");
  });

  it("shows a selected tuple inspector and updates when a different row is clicked", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
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
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "base_exit_or_target_stop",
          fillModel: "quote_mid",
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
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Selected tuple");
    expect(wrapper.text()).toContain("fixed_hold_or_target_stop");
    expect(wrapper.text()).toContain("aggregate_research");

    const rows = wrapper.findAll(
      ".resultCard > .tableWrap .resultTable tbody tr",
    );
    expect(rows).toHaveLength(2);
    expect(rows[0]?.attributes("tabindex")).toBe("0");
    expect(rows.some((entry) => entry.attributes("aria-selected") === "true")).toBe(
      true,
    );
    const promotedRow = rows.find((entry) => entry.text().includes("otm_1"));
    expect(promotedRow).toBeTruthy();
    expect(promotedRow?.attributes("aria-label")).toContain("Select tuple #2");
    await promotedRow!.trigger("click");
    await wrapper.vm.$nextTick();

    await vi.waitFor(() => {
      expect(wrapper.text()).toContain("base_exit_or_target_stop");
      expect(wrapper.text()).toContain("quote_mid");
      expect(wrapper.text()).toContain("126.0000");
    });

    const originalRow = rows.find((entry) => entry.text().includes("atm"));
    expect(originalRow).toBeTruthy();
    await originalRow!.trigger("keydown", { key: "Enter" });
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("fixed_hold_or_target_stop");
    expect(wrapper.text()).toContain("aggregate_research");
  });

  it("surfaces explicit validation verdicts for selected tuples", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 4,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          validationVerdict: "search_only",
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
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          validationVerdict: "holdout_supported",
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
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: "itm_1",
          holdMinutes: 45,
          targetReturn: 0.75,
          stopReturn: 0.5,
          exitTemplate: "base_exit_or_target_stop",
          fillModel: "quote_mid",
          validationVerdict: "sealed_finalist",
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
            expectancyPerPremium: 0.1,
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
          moneynessBucket: "otm_2",
          holdMinutes: 60,
          targetReturn: 1,
          stopReturn: 0.5,
          exitTemplate: "base_exit_or_target_stop",
          fillModel: "quote_mid",
          validationVerdict: "stress_fragile",
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
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Selected tuple validation · Search only");
    expect(wrapper.text()).toContain("Validation winners");
    expect(wrapper.text()).toContain("Best sealed EV");
    expect(wrapper.text()).toContain("Best sealed extreme EV");
    expect(wrapper.text()).toContain(
      "Ranked tuples remain search-stage candidates",
    );

    const rows = wrapper.findAll(
      ".resultCard > .tableWrap .resultTable tbody tr",
    );
    expect(rows).toHaveLength(4);

    const holdoutRow = rows.find((entry) => entry.text().includes("otm_1"));
    expect(holdoutRow).toBeTruthy();
    await holdoutRow!.trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(
      "Selected tuple validation · Holdout-supported",
    );
    expect(wrapper.text()).toContain("Holdout support 2 / 2 (100.0%)");

    const sealedRow = rows.find((entry) => entry.text().includes("itm_1"));
    expect(sealedRow).toBeTruthy();
    await sealedRow!.trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(
      "Selected tuple validation · Sealed finalist",
    );
    expect(wrapper.text()).toContain(
      "Promoted into the sealed finalist set (#1)",
    );
    expect(wrapper.text()).toContain("Sealed support 2 / 2 (100.0%)");
    expect(wrapper.text()).toContain("Validation frontier");
    expect(wrapper.text()).toContain("Sealed finalist");

    const fragileRow = rows.find((entry) => entry.text().includes("otm_2"));
    expect(fragileRow).toBeTruthy();
    await fragileRow!.trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(
      "Selected tuple validation · Stress-fragile",
    );
    expect(wrapper.text()).toContain("Support");
    expect(wrapper.text()).toContain(
      "EV / premium falls from 0.0900 to -0.0200",
    );
  });

  it("falls back to support-aware tuple validation semantics when verdicts are absent", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.18,
          selectionMetrics: {
            validTrades: 8,
            invalidEvents: 1,
            invalidRate: 0.1111,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.22,
            medianReturnOnPremium: 0.16,
            winRate: 0.62,
            totalPnlPerContract: 80,
          },
          holdoutMetrics: {
            validTrades: 0,
            invalidEvents: 2,
            invalidRate: 1,
            expectancyPerTrade: 0,
            expectancyPerPremium: 0.09,
            medianReturnOnPremium: 0,
            winRate: 0,
            totalPnlPerContract: 0,
          },
          sealedEvaluationMetrics: {
            validTrades: 0,
            invalidEvents: 1,
            invalidRate: 1,
            expectancyPerTrade: 0,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0,
            winRate: 0,
            totalPnlPerContract: 0,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0.05,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
      ],
    });

    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("Selected tuple validation · Search only");
    expect(wrapper.text()).toContain(
      "No supported holdout or sealed trades are present yet",
    );

    store.latestRun = makeRunSummary({
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          robustScore: 0.18,
          selectionMetrics: {
            validTrades: 8,
            invalidEvents: 1,
            invalidRate: 0.1111,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.22,
            medianReturnOnPremium: 0.16,
            winRate: 0.62,
            totalPnlPerContract: 80,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.09,
            medianReturnOnPremium: 0.06,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          sealedEvaluationMetrics: {
            validTrades: 0,
            invalidEvents: 1,
            invalidRate: 1,
            expectancyPerTrade: 0,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0,
            winRate: 0,
            totalPnlPerContract: 0,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0.05,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
      ],
    });

    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(
      "Selected tuple validation · Holdout-supported",
    );
    expect(wrapper.text()).toContain("Supported holdout trades are present");
  });

  it("adds validation interpretation drift, monotonicity, regret, and scope labels for the selected tuple", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.3,
          selectionMetrics: {
            validTrades: 4,
            invalidEvents: 1,
            invalidRate: 0.2,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.14,
            winRate: 0.5,
            totalPnlPerContract: 40,
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
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.07,
            medianReturnOnPremium: 0.05,
            winRate: 0.5,
            totalPnlPerContract: 10,
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
        {
          delayMinutes: 5,
          dteDays: 3,
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 2,
          robustScore: 0.29,
          selectionMetrics: {
            validTrades: 4,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 9,
            expectancyPerPremium: 0.18,
            medianReturnOnPremium: 0.12,
            winRate: 0.5,
            totalPnlPerContract: 36,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 7,
            expectancyPerPremium: 0.12,
            medianReturnOnPremium: 0.09,
            winRate: 0.5,
            totalPnlPerContract: 14,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.07,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.05,
            medianReturnOnPremium: 0.03,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 3,
            expectancyPerPremium: 0.04,
            medianReturnOnPremium: 0.03,
            winRate: 0.5,
            totalPnlPerContract: 6,
          },
          harsherStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 2,
            expectancyPerPremium: 0.01,
            medianReturnOnPremium: 0.01,
            winRate: 0.5,
            totalPnlPerContract: 2,
          },
          sealedHarsherStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 1,
            expectancyPerPremium: 0.0,
            medianReturnOnPremium: 0.0,
            winRate: 0.5,
            totalPnlPerContract: 0,
          },
          extremeStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: -1,
            expectancyPerPremium: -0.03,
            medianReturnOnPremium: -0.02,
            winRate: 0.5,
            totalPnlPerContract: -2,
          },
          sealedExtremeStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: -2,
            expectancyPerPremium: -0.04,
            medianReturnOnPremium: -0.03,
            winRate: 0.5,
            totalPnlPerContract: -4,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: "otm_2",
          holdMinutes: 45,
          targetReturn: 0.75,
          stopReturn: 0.5,
          exitTemplate: "base_exit_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: false,
          robustScore: 0.12,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.14,
            medianReturnOnPremium: 0.09,
            winRate: 0.5,
            totalPnlPerContract: 18,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 7,
            expectancyPerPremium: 0.16,
            medianReturnOnPremium: 0.11,
            winRate: 0.5,
            totalPnlPerContract: 14,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 15,
            expectancyPerPremium: 0.9,
            medianReturnOnPremium: 0.8,
            winRate: 0.5,
            totalPnlPerContract: 30,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 14,
            expectancyPerPremium: 0.8,
            medianReturnOnPremium: 0.7,
            winRate: 0.5,
            totalPnlPerContract: 28,
          },
        },
      ],
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Validation interpretation");
    expect(wrapper.text()).toContain("Selection -> Holdout");
    expect(wrapper.text()).toContain("-0.1000");
    expect(wrapper.text()).toContain("Holdout -> Sealed");
    expect(wrapper.text()).toContain("-0.0300");
    expect(wrapper.text()).toContain("Holdout -> Stress");
    expect(wrapper.text()).toContain("Sealed stress -> Sealed harsher");
    expect(wrapper.text()).toContain("Holdout family");
    expect(wrapper.text()).toContain("Sealed family");
    expect(wrapper.text()).toContain("Monotone");
    expect(wrapper.text()).toContain("Selected tuple only");
    expect(wrapper.text()).toContain("Retained holdout regret");
    expect(wrapper.text()).toContain("0.0200");
    expect(wrapper.text()).toContain("Sealed regret");
    expect(wrapper.text()).toContain("0.0300");
    expect(wrapper.text()).toContain("Within retained top_k only");
    expect(wrapper.text()).toContain("Within sealed finalists only");
    expect(wrapper.text()).toContain(
      "Support rows above reflect valid trades over total events",
    );
  });

  it("reports non-monotone and insufficient validation ladders separately", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.2,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 8,
            expectancyPerPremium: 0.16,
            medianReturnOnPremium: 0.11,
            winRate: 0.5,
            totalPnlPerContract: 24,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.12,
            medianReturnOnPremium: 0.09,
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
        },
      ],
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Validation interpretation");
    expect(wrapper.text()).toContain("Non-monotone");
    expect(wrapper.text()).toContain("Holdout 0.1000 -> Stress 0.1200");
    expect(wrapper.text()).toContain("Insufficient lanes");
  });

  it("computes retained regret from raw retained rows rather than the filtered display table", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.18,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 7,
            expectancyPerPremium: 0.16,
            medianReturnOnPremium: 0.1,
            winRate: 0.5,
            totalPnlPerContract: 21,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.07,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 3,
            expectancyPerPremium: 0.06,
            medianReturnOnPremium: 0.04,
            winRate: 0.5,
            totalPnlPerContract: 6,
          },
        },
        {
          delayMinutes: 5,
          dteDays: 3,
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: false,
          robustScore: 0.12,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.12,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 18,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 9,
            expectancyPerPremium: 0.25,
            medianReturnOnPremium: 0.18,
            winRate: 0.5,
            totalPnlPerContract: 18,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.08,
            medianReturnOnPremium: 0.05,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
        },
      ],
    });

    await wrapper.vm.$nextTick();

    const finalistsToggle = wrapper
      .findAll("label")
      .find((entry) => entry.text().includes("Finalists only"));
    expect(finalistsToggle).toBeTruthy();
    await finalistsToggle!.find("input").setValue(true);
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Retained holdout regret");
    expect(wrapper.text()).toContain("0.1500");
    expect(wrapper.text()).toContain("Within retained top_k only");
  });

  it("computes sealed-scope regret from sealed finalists only", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        {
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 1,
          robustScore: 0.2,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 8,
            expectancyPerPremium: 0.16,
            medianReturnOnPremium: 0.1,
            winRate: 0.5,
            totalPnlPerContract: 24,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 10,
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
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.04,
            medianReturnOnPremium: 0.03,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 3,
            expectancyPerPremium: 0.03,
            medianReturnOnPremium: 0.02,
            winRate: 0.5,
            totalPnlPerContract: 6,
          },
        },
        {
          delayMinutes: 5,
          dteDays: 3,
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: true,
          sealedFinalRank: 2,
          robustScore: 0.19,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 7,
            expectancyPerPremium: 0.14,
            medianReturnOnPremium: 0.09,
            winRate: 0.5,
            totalPnlPerContract: 21,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.11,
            medianReturnOnPremium: 0.08,
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
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.05,
            medianReturnOnPremium: 0.03,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 4,
            expectancyPerPremium: 0.04,
            medianReturnOnPremium: 0.03,
            winRate: 0.5,
            totalPnlPerContract: 8,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: "itm_1",
          holdMinutes: 45,
          targetReturn: 0.75,
          stopReturn: 0.5,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          sealedFinalist: false,
          robustScore: 0.1,
          selectionMetrics: {
            validTrades: 3,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 6,
            expectancyPerPremium: 0.12,
            medianReturnOnPremium: 0.07,
            winRate: 0.5,
            totalPnlPerContract: 18,
          },
          holdoutMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.1,
            medianReturnOnPremium: 0.07,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          sealedEvaluationMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 10,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.14,
            winRate: 0.5,
            totalPnlPerContract: 20,
          },
          stressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5,
            expectancyPerPremium: 0.06,
            medianReturnOnPremium: 0.04,
            winRate: 0.5,
            totalPnlPerContract: 10,
          },
          sealedStressMetrics: {
            validTrades: 2,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 8,
            expectancyPerPremium: 0.15,
            medianReturnOnPremium: 0.1,
            winRate: 0.5,
            totalPnlPerContract: 16,
          },
        },
      ],
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Sealed regret");
    expect(wrapper.text()).toContain("0.0200");
    expect(wrapper.text()).toContain("Sealed stress regret");
    expect(wrapper.text()).toContain("0.0100");
    expect(wrapper.text()).toContain("Within sealed finalists only");
  });

  it("surfaces search-winner vs validation-winner disagreement in the current frontier", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_disagreement.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 3,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          validationVerdict: "search_only",
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
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
          validationVerdict: "holdout_supported",
          robustScore: 0.38,
          selectionMetrics: {
            validTrades: 6,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 16,
            expectancyPerPremium: 0.31,
            medianReturnOnPremium: 0.2,
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
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.11,
            winRate: 0.5,
            totalPnlPerContract: 12,
          },
        },
        {
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: "itm_1",
          holdMinutes: 45,
          targetReturn: 0.75,
          stopReturn: 0.5,
          exitTemplate: "base_exit_or_target_stop",
          fillModel: "quote_mid",
          validationVerdict: "sealed_finalist",
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
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Winner alignment · Winner disagreement");
    expect(wrapper.text()).toContain(
      "Selection winner #1 does not fully survive validation.",
    );
    expect(wrapper.text()).toContain("holdout winner #2");
    expect(wrapper.text()).toContain("sealed winner #3");
    expect(wrapper.text()).toContain("Search baseline");
    expect(wrapper.text()).toContain("Same as robust");
    expect(wrapper.text()).toContain("Different tuple");
  });

  it("loads lazy event detail for the selected tuple", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);
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
        moneynessBucket: "atm",
        holdMinutes: 15,
        targetReturn: 0.25,
        stopReturn: 0.25,
        exitTemplate: "fixed_hold_or_target_stop",
        fillModel: "aggregate_research",
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
          eventId: "event-1",
          split: "search",
          side: "bullish",
          decisionTimestamp: 100,
          earliestTradableTimestamp: 160,
          requestedEntryTimestamp: 460,
          rawUnderlierPrice: 501.25,
          baseExitTimestamp: 700,
          status: "executed",
          contractId: "SPY_C_1",
          expirationDate: "2026-04-12",
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
          exitReason: "target_hit",
        },
        {
          eventIndex: 1,
          eventId: "event-2",
          split: "holdout",
          side: "bearish",
          decisionTimestamp: 200,
          earliestTradableTimestamp: 260,
          requestedEntryTimestamp: 560,
          rawUnderlierPrice: 498.75,
          baseExitTimestamp: null,
          status: "candidate_missing",
          candidateInvalidReason: "no_expiration_in_bucket",
          contractId: "",
          expirationDate: "",
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
          exitReason: "",
        },
      ],
    });
    inspectOptionsExpressionEventPathMock.mockResolvedValue({
      tupleIndex: 0,
      eventIndex: 0,
      eventId: "event-1",
      row: {
        eventIndex: 0,
        eventId: "event-1",
        split: "search",
        side: "bullish",
        decisionTimestamp: 100,
        earliestTradableTimestamp: 160,
        requestedEntryTimestamp: 460,
        rawUnderlierPrice: 501.25,
        baseExitTimestamp: 700,
        status: "executed",
        candidateInvalidReason: "",
        contractId: "SPY_C_1",
        expirationDate: "2026-04-12",
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
        exitReason: "target_hit",
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
    });

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "2026-04-05T12:00:00Z",
      parentArtifactId: "vectorgrid_run_1",
      optionsDatasetHash: "hash-1",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 2,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
          moneynessBucket: "atm",
          holdMinutes: 15,
          targetReturn: 0.25,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
          moneynessBucket: "otm_1",
          holdMinutes: 30,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: "fixed_hold_or_target_stop",
          fillModel: "aggregate_research",
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
    });

    await wrapper.vm.$nextTick();

    const loadButton = wrapper
      .findAll("button")
      .find((entry) => entry.text() === "Load event detail");
    expect(loadButton).toBeTruthy();
    await loadButton!.trigger("click");

    await vi.waitFor(() => {
      expect(inspectOptionsExpressionTupleMock).toHaveBeenCalledWith({
        artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
        tupleIndex: 0,
      });
    });
    await vi.waitFor(() => {
      expect(inspectOptionsExpressionEventPathMock).toHaveBeenCalledWith({
        artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
        tupleIndex: 0,
        eventIndex: 0,
      });
    });

    expect(wrapper.text()).toContain("Tuple event detail");
    expect(wrapper.text()).toContain("SPY_C_1");
    expect(wrapper.text()).toContain("target_hit");
    expect(wrapper.text()).toContain("5m / 1440m");
    expect(wrapper.text()).toContain("hold 60m");
    expect(wrapper.text()).toContain("opt 1.3500 -> 1.9500");
    expect(wrapper.text()).toContain("und 500.80 -> 503.10");
    expect(wrapper.text()).toContain("Export inspection CSV");
    expect(wrapper.text()).toContain("candidate_missing");
    expect(wrapper.text()).toContain("no_expiration_in_bucket");
    expect(wrapper.text()).toContain("Showing 2 of 2 event rows");
    expect(wrapper.text()).toContain("Selected event");
    expect(wrapper.text()).toContain("Event path replay");
    expect(wrapper.findAll("img.sectionGlyph").length).toBeGreaterThanOrEqual(
      3,
    );
    expect(
      wrapper.findAll("img.optionsActionIcon").length,
    ).toBeGreaterThanOrEqual(6);
    expect(wrapper.text()).toContain("Option range");
    expect(wrapper.text()).toContain("1.4000 -> 1.8000");
    expect(wrapper.text()).toContain("501.50 -> 503.10");

    const inspectionRows = wrapper.findAll(".inspectionTable tbody tr");
    expect(inspectionRows).toHaveLength(2);
    expect(inspectionRows[0]?.attributes("tabindex")).toBe("0");
    expect(inspectionRows[0]?.attributes("aria-selected")).toBe("true");
    expect(inspectionRows[1]?.attributes("aria-label")).toBe(
      "Select inspection event event-2",
    );
    await inspectionRows[1]!.trigger("click");

    expect(wrapper.text()).toContain("event-2 / holdout / bearish");
    expect(wrapper.text()).toContain(
      "candidate_missing / no_expiration_in_bucket",
    );
    expect(wrapper.text()).not.toContain("Event path replay");

    await inspectionRows[0]!.trigger("keydown", { key: " " });
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("event-1 / search / bullish");
  });

  it("adds validation interpretation drift rows, monotonicity cues, and scope labels for the selected tuple", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        makeTuple({
          sealedFinalist: true,
          sealedFinalRank: 1,
          validationVerdict: "sealed_finalist",
          selectionMetrics: makeMetrics({
            expectancyPerPremium: 0.22,
            expectancyPerTrade: 12,
          }),
          holdoutMetrics: makeMetrics({
            expectancyPerPremium: 0.18,
            expectancyPerTrade: 8,
          }),
          sealedEvaluationMetrics: makeMetrics({
            expectancyPerPremium: 0.16,
            expectancyPerTrade: 7,
          }),
          stressMetrics: makeMetrics({
            expectancyPerPremium: 0.12,
            expectancyPerTrade: 6,
          }),
          sealedStressMetrics: makeMetrics({
            expectancyPerPremium: 0.15,
            expectancyPerTrade: 6,
          }),
          harsherStressMetrics: makeMetrics({
            expectancyPerPremium: 0.1,
            expectancyPerTrade: 5,
          }),
          sealedHarsherStressMetrics: makeMetrics({
            expectancyPerPremium: 0.12,
            expectancyPerTrade: 5,
          }),
          extremeStressMetrics: makeMetrics({
            expectancyPerPremium: 0.11,
            expectancyPerTrade: 4,
          }),
          sealedExtremeStressMetrics: makeMetrics({
            expectancyPerPremium: 0.08,
            expectancyPerTrade: 3,
          }),
        }),
      ],
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Validation interpretation");
    expect(wrapper.text()).toContain("Retained frontier scope");
    expect(wrapper.text()).toContain("Within retained top_k only");
    expect(wrapper.text()).toContain("Sealed frontier scope");
    expect(wrapper.text()).toContain("Within sealed finalists only");
    expect(wrapper.text()).toContain(
      "Support rows above reflect valid trades over total events",
    );
    expect(wrapper.text()).toContain("Selection -> Holdout");
    expect(wrapper.text()).toContain("Holdout -> Sealed");
    expect(wrapper.text()).toContain("Holdout -> Stress");
    expect(wrapper.text()).toContain("Stress -> Harsher");
    expect(wrapper.text()).toContain("Harsher -> Extreme");
    expect(wrapper.text()).toContain("Sealed -> Sealed stress");
    expect(wrapper.text()).toContain("Sealed stress -> Sealed harsher");
    expect(wrapper.text()).toContain("Sealed harsher -> Sealed extreme");
    expect(wrapper.text()).toContain("-0.0400");
    expect(wrapper.text()).toContain("+0.0100");
    expect(wrapper.text()).toContain("Holdout family");
    expect(wrapper.text()).toContain("Sealed family");
    expect(wrapper.text()).toContain("Non-monotone");
    expect(wrapper.text()).toContain("Monotone");
  });

  it("marks sparse validation ladders as insufficient instead of treating missing lanes as failure", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      rankedResults: [
        makeTuple({
          sealedFinalist: true,
          sealedFinalRank: 1,
          selectionMetrics: makeMetrics({ expectancyPerPremium: 0.2 }),
          holdoutMetrics: makeMetrics({ expectancyPerPremium: 0.14 }),
          sealedEvaluationMetrics: makeMetrics({ expectancyPerPremium: 0.13 }),
          stressMetrics: null,
          sealedStressMetrics: null,
          harsherStressMetrics: null,
          sealedHarsherStressMetrics: null,
          extremeStressMetrics: null,
          sealedExtremeStressMetrics: null,
        }),
      ],
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Validation interpretation");
    expect(wrapper.text()).toContain("Holdout family");
    expect(wrapper.text()).toContain("Sealed family");
    expect(
      wrapper.text().match(/Insufficient lanes/g)?.length ?? 0,
    ).toBeGreaterThanOrEqual(2);
  });

  it("computes retained-scope regret from raw retained rows rather than the filtered display table", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const wrapper = mount(OptionsExpressionPanel, {
      props: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        parentStrategyKind: "DoubleMa",
        parentDatasetLabel: "SPY.csv",
      },
    });

    await vi.waitFor(() => {
      expect(listOptionsDatasetsMock).toHaveBeenCalledTimes(1);
      expect(listOptionsExpressionArtifactsMock).toHaveBeenCalledTimes(1);
    });

    const store = useOptionsExpressionStore();
    store.latestRun = makeRunSummary({
      tupleCount: 3,
      rankedResults: [
        makeTuple({
          delayMinutes: 0,
          dteDays: 1,
          moneynessBucket: "atm",
          robustScore: 0.36,
          selectionMetrics: makeMetrics({ expectancyPerPremium: 0.34 }),
          holdoutMetrics: makeMetrics({ expectancyPerPremium: 0.3 }),
          stressMetrics: makeMetrics({ expectancyPerPremium: 0.28 }),
          sealedFinalist: false,
          sealedEvaluationMetrics: null,
          sealedStressMetrics: null,
        }),
        makeTuple({
          delayMinutes: 5,
          dteDays: 3,
          moneynessBucket: "otm_1",
          robustScore: 0.32,
          sealedFinalist: true,
          sealedFinalRank: 1,
          selectionMetrics: makeMetrics({ expectancyPerPremium: 0.29 }),
          holdoutMetrics: makeMetrics({ expectancyPerPremium: 0.24 }),
          stressMetrics: makeMetrics({ expectancyPerPremium: 0.2 }),
          sealedEvaluationMetrics: makeMetrics({ expectancyPerPremium: 0.22 }),
          sealedStressMetrics: makeMetrics({ expectancyPerPremium: 0.18 }),
        }),
        makeTuple({
          delayMinutes: 10,
          dteDays: 5,
          moneynessBucket: "itm_1",
          robustScore: 0.28,
          sealedFinalist: true,
          sealedFinalRank: 2,
          selectionMetrics: makeMetrics({ expectancyPerPremium: 0.27 }),
          holdoutMetrics: makeMetrics({ expectancyPerPremium: 0.26 }),
          stressMetrics: makeMetrics({ expectancyPerPremium: 0.22 }),
          sealedEvaluationMetrics: makeMetrics({ expectancyPerPremium: 0.25 }),
          sealedStressMetrics: makeMetrics({ expectancyPerPremium: 0.21 }),
        }),
      ],
    });

    await wrapper.vm.$nextTick();

    const rows = wrapper.findAll(
      ".resultCard > .tableWrap .resultTable tbody tr",
    );
    const selectedRow = rows.find((entry) => entry.text().includes("otm_1"));
    expect(selectedRow).toBeTruthy();
    await selectedRow!.trigger("click");
    await wrapper.vm.$nextTick();

    const finalistsOnly = wrapper.find('.toggleField input[type="checkbox"]');
    await finalistsOnly.setValue(true);
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Validation interpretation");
    expect(wrapper.text()).toContain("Retained holdout regret");
    expect(wrapper.text()).toContain("Retained stress regret");
    expect(wrapper.text()).toContain("Sealed regret");
    expect(wrapper.text()).toContain("Sealed stress regret");
    expect(wrapper.text()).toContain("0.0600");
    expect(wrapper.text()).toContain("0.0800");
    expect(wrapper.text()).toContain("0.0300");
  });
});
