import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { isReactive } from "vue";
import { TauriRuntimeError } from "../tauri/runtimeError";
import { useOptionsExpressionStore } from "./optionsExpressionStore";

const {
  analyzeOptionsExpressionFeasibilityMock,
  analyzeOptionsExpressionValidationFeasibilityMock,
  downloadPolygonOptionsDatasetMock,
  exportOptionsExpressionCsvMock,
  exportOptionsExpressionEventPathCsvMock,
  exportOptionsExpressionInspectionCsvMock,
  importOptionsDatasetMock,
  inspectOptionsExpressionEventPathMock,
  inspectOptionsExpressionTupleMock,
  listOptionsDatasetsMock,
  listOptionsExpressionArtifactsMock,
  loadOptionsExpressionArtifactMock,
  pickOptionsCsvPathMock,
  revealOptionsPathMock,
  runOptionsExpressionMock,
  validateOptionsExpressionArtifactMock,
} = vi.hoisted(() => ({
  analyzeOptionsExpressionFeasibilityMock: vi.fn(),
  analyzeOptionsExpressionValidationFeasibilityMock: vi.fn(),
  downloadPolygonOptionsDatasetMock: vi.fn(),
  exportOptionsExpressionCsvMock: vi.fn(),
  exportOptionsExpressionEventPathCsvMock: vi.fn(),
  exportOptionsExpressionInspectionCsvMock: vi.fn(),
  importOptionsDatasetMock: vi.fn(),
  inspectOptionsExpressionEventPathMock: vi.fn(),
  inspectOptionsExpressionTupleMock: vi.fn(),
  listOptionsDatasetsMock: vi.fn(),
  listOptionsExpressionArtifactsMock: vi.fn(),
  loadOptionsExpressionArtifactMock: vi.fn(),
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
  exportOptionsExpressionEventPathCsv: exportOptionsExpressionEventPathCsvMock,
  exportOptionsExpressionInspectionCsv:
    exportOptionsExpressionInspectionCsvMock,
  importOptionsDataset: importOptionsDatasetMock,
  inspectOptionsExpressionEventPath: inspectOptionsExpressionEventPathMock,
  inspectOptionsExpressionTuple: inspectOptionsExpressionTupleMock,
  listOptionsDatasets: listOptionsDatasetsMock,
  listOptionsExpressionArtifacts: listOptionsExpressionArtifactsMock,
  loadOptionsExpressionArtifact: loadOptionsExpressionArtifactMock,
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
    objectiveConfig: { profile: "balanced" },
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

function makeFeasibilitySummary(overrides: Record<string, unknown> = {}) {
  return {
    replayBuildMs: 18,
    totalPreviewMs: 41,
    replayBuildTiming: null,
    diagnostics: makeDiagnosticsSummary(),
    objectiveConfig: { profile: "balanced" },
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
    priceSemantics: "minute_aggregate",
    request: {
      parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
      parentCandidateIndex: null,
      optionsDatasetId: "spy_options",
      objectiveConfig: { profile: "balanced" },
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

function makeValidationSummary(overrides: Record<string, unknown> = {}) {
  return {
    artifactPath: "C:/runs/vectorgrid_options_validation_1.json",
    replayBuildMs: 3,
    validationMs: 4,
    replayBuildTiming: null,
    result: {
      version: 1,
      kind: "vectorgrid_options_expression_validation_v1",
      ts: "2026-04-05T12:30:00Z",
      validationArtifactPath: "C:/runs/vectorgrid_options_validation_1.json",
      sourceArtifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      sourceArtifactHash: "source-hash",
      parentArtifactId: "vectorgrid_run_1",
      parentArtifactHash: "parent-hash",
      signalManifestId: "manifest",
      rootSymbol: "SPY",
      optionsDatasetId: "spy_options",
      optionsDatasetHash: "hash-1",
      datasetPolicy: "require_exact_replay",
      replacementDataset: false,
      validationPlan: {
        mode: "original_embedded_splits",
        startTimestampMs: null,
        endTimestampMs: null,
        trainSessionCount: null,
        validationSessionCount: null,
        stepSessionCount: null,
      },
      tupleSelection: {
        includeSealedFinalists: true,
        includeRobustLeader: true,
        includeHoldoutLeader: true,
        tupleIndexes: [],
      },
      priceSemantics: "minute_aggregate",
      datasetProvenance: null,
      diagnostics: makeDiagnosticsSummary(),
      overlapWindow: makeOverlapWindow(),
      coverageAdequacy: makeCoverageAdequacy(),
      laneSummaries: [],
      acceptanceSummary: {
        verdict: "accepted",
        selectedTupleCount: 2,
        validatedTupleCount: 2,
        acceptedTupleCount: 1,
        supportDeficientTupleCount: 0,
        stressFragileTupleCount: 0,
        minValidTrades: null,
        bestValidatedExpectancyPerPremium: 0.24,
        worstStressExpectancyPerPremium: 0.12,
        notes: ["1 selected tuple passed support, validation expectancy, and stress checks."],
      },
      tupleResults: [],
    },
    ...overrides,
  };
}

describe("optionsExpressionStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("prefers matching-root datasets for the attached parent artifact", async () => {
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "qqq_options",
        rootSymbol: "QQQ",
        priceSemantics: "minute_aggregate",
        contractCount: 5,
        rowCount: 100,
      },
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    await store.bootstrap();

    expect(store.parentArtifactId).toBe("vectorgrid_run_1");
    expect(store.parentRootHint).toBe("SPY");
    expect(store.selectedDatasetId).toBe("spy_options");
  });

  it("defaults backend preference to auto for new options follow-up sessions", () => {
    const store = useOptionsExpressionStore();
    expect(store.backendPreference).toBe("auto");
  });

  it("coerces incompatible fill models back to aggregate_research for aggregate datasets", () => {
    const store = useOptionsExpressionStore();
    store.datasets = [
      {
        version: 1,
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        providerName: "polygon.io",
        providerSourceSurface: "minute_aggregate",
        providerEntitlementProfile: "",
        quoteResolution: "1m",
        priceSemantics: "minute_aggregate",
        calendarId: "market_hours_utc",
        timezone: "UTC",
        coverageStart: 0,
        coverageEnd: 0,
        normalizationVersion: 1,
        quoteSemanticsVersion: 1,
        corporateActionMappingVersion: 1,
        contractReferenceSource: "polygon_contract_reference",
        contractReferenceAsOfUtc: "",
        underlierReferenceSource: "",
        datasetPath: "",
        contractCount: 3,
        rowCount: 42,
        aggregatePriceField: "close",
      },
      {
        version: 1,
        optionsDatasetId: "spy_quotes",
        rootSymbol: "SPY",
        providerName: "polygon.io",
        providerSourceSurface: "top_of_book_quote",
        providerEntitlementProfile: "",
        quoteResolution: "1m",
        priceSemantics: "top_of_book_quote",
        calendarId: "market_hours_utc",
        timezone: "UTC",
        coverageStart: 0,
        coverageEnd: 0,
        normalizationVersion: 1,
        quoteSemanticsVersion: 1,
        corporateActionMappingVersion: 1,
        contractReferenceSource: "polygon_contract_reference",
        contractReferenceAsOfUtc: "",
        underlierReferenceSource: "",
        datasetPath: "",
        contractCount: 3,
        rowCount: 42,
        aggregatePriceField: "",
      },
    ];

    store.selectedDatasetId = "spy_options";
    store.fillModel = "quote_mid";
    store.ensureCompatibleFillModel();
    expect(store.fillModel).toBe("aggregate_research");

    store.selectedDatasetId = "spy_quotes";
    store.fillModel = "quote_mid";
    store.ensureCompatibleFillModel();
    expect(store.fillModel).toBe("quote_mid");
  });

  it("treats the current double-like parent families as supported follow-up inputs", () => {
    const store = useOptionsExpressionStore();

    store.attachParent(
      "C:/runs/vectorgrid_run_1.json",
      "CompositeTrend",
      "SPY.csv",
    );
    expect(store.supportedParent).toBe(true);
    expect(store.statusLine).toContain("Ready to import");

    store.attachParent("C:/runs/vectorgrid_run_2.json", "MlSignal", "SPY.csv");
    expect(store.supportedParent).toBe(false);
    expect(store.statusLine).toContain("CompositeTrend");
    expect(store.statusLine).toContain("BreakoutMomentum");
  });

  it("runs options expression with normalized request fields", async () => {
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
    runOptionsExpressionMock.mockResolvedValue({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      backendUsed: "cpu_reference",
      backendFallbackReason: "GPU planner probe failed; using CPU.",
      priceSemantics: "minute_aggregate",
      tupleCount: 2,
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

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    await store.bootstrap();
    store.delayMinutesInput = "0, 5, 1";
    store.selectedMoneynessBuckets = ["atm", "otm_1"];
    store.backendPreference = "gpu_compiled_tensor";
    store.fillModel = "quote_mid";
    store.splitAssignmentMode = "session_chronological";
    store.topKInput = "12";
    store.sealedFinalistCountInput = "7";
    await store.runCurrentExpression();

    expect(runOptionsExpressionMock).toHaveBeenCalledWith(
      expect.objectContaining({
        parent_artifact_path: "C:/runs/vectorgrid_run_1.json",
        options_dataset_id: "spy_options",
        backend_preference: "gpu_compiled_tensor",
        objective_config: { profile: "balanced" },
        fill_model: "aggregate_research",
        split_assignment_mode: "session_chronological",
        delay_minutes: [0, 1, 5],
        moneyness_buckets: ["atm", "otm_1"],
        top_k: 12,
        sealed_finalist_count: 7,
      }),
    );
    expect(store.latestRun?.artifactPath).toBe(
      "C:/runs/vectorgrid_options_expression_1.json",
    );
    expect(isReactive(store.latestRun)).toBe(false);
    expect(isReactive(store.latestRun?.rankedResults)).toBe(false);
    expect(store.activeStaging?.compiledCandidateCount).toBe(9);
    expect(store.activeBackendFallbackReason).toBe(
      "GPU planner probe failed; using CPU.",
    );
  });

  it("previews feasibility with normalized request fields", async () => {
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
    analyzeOptionsExpressionFeasibilityMock.mockResolvedValue(
      makeFeasibilitySummary({
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
          cpuRankingUs: 6_400,
          gpuRankingMs: 3,
          gpuRankingUs: 3_100,
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
        },
        invalidReasons: [{ reason: "no_expiration_in_bucket", count: 20 }],
        cacheStats: {
          structuralCacheHits: 1,
          structuralCacheMisses: 1,
          structuralCacheEntries: 1,
          summaryCacheHits: 1,
          summaryCacheMisses: 1,
          summaryCacheEntries: 1,
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
      }),
    );

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    await store.bootstrap();
    store.delayMinutesInput = "0, 5, 1";
    store.selectedMoneynessBuckets = ["atm", "otm_1"];
    store.backendPreference = "cpu_reference";
    store.splitAssignmentMode = "session_chronological";
    store.sealedFinalistCountInput = "4";
    await store.previewCurrentFeasibility();

    expect(analyzeOptionsExpressionFeasibilityMock).toHaveBeenCalledWith(
      expect.objectContaining({
        parent_artifact_path: "C:/runs/vectorgrid_run_1.json",
        options_dataset_id: "spy_options",
        backend_preference: "cpu_reference",
        objective_config: { profile: "balanced" },
        split_assignment_mode: "session_chronological",
        delay_minutes: [0, 1, 5],
        moneyness_buckets: ["atm", "otm_1"],
        sealed_finalist_count: 4,
      }),
    );
    expect(store.latestFeasibility?.compiledTupleCount).toBe(12);
    expect(store.statusLine).toContain("60.0% candidate density");
  });

  it("imports datasets from the CSV picker and reveals artifacts", async () => {
    pickOptionsCsvPathMock.mockResolvedValue("C:/imports/spy_options.csv");
    importOptionsDatasetMock.mockResolvedValue({
      optionsDatasetId: "spy_options",
      rootSymbol: "SPY",
      priceSemantics: "minute_aggregate",
      contractCount: 3,
      rowCount: 42,
    });
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        contractCount: 3,
        rowCount: 42,
      },
    ]);
    revealOptionsPathMock.mockResolvedValue({
      path: "C:/runs/vectorgrid_options_expression_1.json",
      openedWith: "reveal_path",
    });

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    await store.importDatasetFromPicker();
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
    });
    await store.revealActiveArtifact();

    expect(importOptionsDatasetMock).toHaveBeenCalledWith(
      expect.objectContaining({
        csv_path: "C:/imports/spy_options.csv",
        root_symbol: "SPY",
        provider_source_surface: "minute_aggregate",
        price_semantics: "minute_aggregate",
        aggregate_price_field: "close",
      }),
    );
    expect(revealOptionsPathMock).toHaveBeenCalledWith(
      "C:/runs/vectorgrid_options_expression_1.json",
    );
  });

  it("can import quote-aware Polygon.io datasets through the picker flow", async () => {
    pickOptionsCsvPathMock.mockResolvedValue("C:/imports/spy_quotes.csv");
    importOptionsDatasetMock.mockResolvedValue({
      optionsDatasetId: "spy_quotes",
      rootSymbol: "SPY",
      priceSemantics: "top_of_book_quote",
      contractCount: 2,
      rowCount: 24,
    });
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_quotes",
        rootSymbol: "SPY",
        priceSemantics: "top_of_book_quote",
        contractCount: 2,
        rowCount: 24,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const store = useOptionsExpressionStore();
    store.attachParent(
      "C:/runs/vectorgrid_run_1.json",
      "CompositeTrend",
      "SPY.csv",
    );
    store.setRealismLane("quote_aware");

    await store.importDatasetFromPicker();

    expect(importOptionsDatasetMock).toHaveBeenCalledWith(
      expect.objectContaining({
        csv_path: "C:/imports/spy_quotes.csv",
        provider_source_surface: "top_of_book_quote",
        price_semantics: "top_of_book_quote",
        aggregate_price_field: undefined,
      }),
    );
    expect(store.selectedDatasetId).toBe("spy_quotes");
  });

  it("can import derived-mid Polygon.io datasets through the picker flow", async () => {
    pickOptionsCsvPathMock.mockResolvedValue("C:/imports/spy_mid.csv");
    importOptionsDatasetMock.mockResolvedValue({
      optionsDatasetId: "spy_mid",
      rootSymbol: "SPY",
      priceSemantics: "derived_mid_snapshot",
      contractCount: 2,
      rowCount: 24,
    });
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_mid",
        rootSymbol: "SPY",
        priceSemantics: "derived_mid_snapshot",
        contractCount: 2,
        rowCount: 24,
      },
    ]);
    listOptionsExpressionArtifactsMock.mockResolvedValue([]);

    const store = useOptionsExpressionStore();
    store.attachParent(
      "C:/runs/vectorgrid_run_1.json",
      "CompositeTrend",
      "SPY.csv",
    );
    store.setRealismLane("derived_mid");

    await store.importDatasetFromPicker();

    expect(importOptionsDatasetMock).toHaveBeenCalledWith(
      expect.objectContaining({
        csv_path: "C:/imports/spy_mid.csv",
        provider_source_surface: "derived_mid_snapshot",
        price_semantics: "derived_mid_snapshot",
        aggregate_price_field: undefined,
      }),
    );
    expect(store.selectedDatasetId).toBe("spy_mid");
    expect(store.fillModel).toBe("aggregate_research");
  });

  it("formats browser-runtime options dataset failures without the runtime class prefix", async () => {
    listOptionsDatasetsMock.mockRejectedValueOnce(
      new TauriRuntimeError({
        code: "TAURI_INVOKE_FAILED",
        operation: "list_options_datasets",
        message:
          "Desktop runtime unavailable for list options datasets. Open this action in the Tauri desktop app.",
      }),
    );
    const store = useOptionsExpressionStore();

    await store.refreshDatasets();

    expect(store.statusLine).toContain(
      "Desktop runtime unavailable for list options datasets.",
    );
    expect(store.statusLine).not.toContain("TauriRuntimeError:");
  });

  it("blocks direct provider download outside the aggregate lane", async () => {
    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.setRealismLane("quote_aware");
    store.downloadFromDateInput = "2026-04-01";
    store.downloadToDateInput = "2026-04-02";

    await store.downloadDatasetFromPolygon();

    expect(downloadPolygonOptionsDatasetMock).not.toHaveBeenCalled();
    expect(store.statusLine).toContain("aggregate-lane only");
  });

  it("downloads Polygon.io datasets through the bounded options flow", async () => {
    downloadPolygonOptionsDatasetMock.mockResolvedValue({
      manifest: {
        optionsDatasetId: "spy_options_1m",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        providerName: "polygon.io",
        providerSourceSurface: "minute_aggregate",
        quoteResolution: "1m",
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
    listOptionsDatasetsMock.mockResolvedValue([
      {
        optionsDatasetId: "spy_options_1m",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
        datasetPath: "C:/datasets/spy_options_1m",
        contractCount: 8,
        rowCount: 200,
      },
    ]);
    revealOptionsPathMock.mockResolvedValue({
      path: "C:/cache/polygon_stocks_raw_SPY_1m.csv",
      openedWith: "reveal_path",
    });

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.downloadFromDateInput = "2026-04-01";
    store.downloadToDateInput = "2026-04-02";
    store.downloadContractType = "put";
    store.downloadExpirationFromDateInput = "2026-04-01";
    store.downloadExpirationToDateInput = "2026-04-03";
    store.downloadStrikeMinInput = "645";
    store.downloadStrikeMaxInput = "665";
    store.downloadMaxContractsInput = "30";

    await store.downloadDatasetFromPolygon();

    expect(downloadPolygonOptionsDatasetMock).toHaveBeenCalledWith(
      expect.objectContaining({
        rootSymbol: "SPY",
        from: "2026-04-01",
        to: "2026-04-02",
        contractType: "put",
        expirationDateFrom: "2026-04-01",
        expirationDateTo: "2026-04-03",
        strikeMin: 645,
        strikeMax: 665,
        maxContracts: 30,
      }),
    );
    expect(store.selectedDatasetId).toBe("spy_options_1m");
    expect(store.latestDatasetDownload?.underlyingCsvPath).toBe(
      "C:/cache/polygon_stocks_raw_SPY_1m.csv",
    );
    expect(store.latestDatasetDownload?.usedUnderlyingCacheOnly).toBe(true);
    expect(store.statusLine).toContain(
      "Downloaded Polygon.io options dataset",
    );

    await store.revealLatestDownloadUnderlyingPath();

    expect(revealOptionsPathMock).toHaveBeenCalledWith(
      "C:/cache/polygon_stocks_raw_SPY_1m.csv",
    );
  });

  it("loads saved options expression artifacts", async () => {
    loadOptionsExpressionArtifactMock.mockResolvedValue(
      makeArtifactRecord({
        version: 1,
        kind: "vectorgrid_options_expression_v1",
        ts: "2026-04-05T12:00:00Z",
        artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
        parentArtifactId: "vectorgrid_run_1",
        rootSymbol: "SPY",
        optionsDatasetId: "spy_options",
        optionsDatasetHash: "hash-1",
        backendUsed: "cpu_reference",
        backendFallbackReason: "GPU planner probe failed; using CPU.",
        priceSemantics: "minute_aggregate",
        staging: {
          candidateSlotCount: 12,
          compiledCandidateCount: 9,
          baselineExitSummaryCount: 24,
          conservativeExitSummaryCount: 24,
          harsherExitSummaryCount: 24,
        },
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
        selectionMetrics: [],
      }),
    );

    const store = useOptionsExpressionStore();
    await store.loadArtifact("C:/runs/vectorgrid_options_expression_1.json");

    expect(store.activeArtifact?.parentArtifactId).toBe("vectorgrid_run_1");
    expect(isReactive(store.activeArtifact)).toBe(false);
    expect(isReactive(store.activeArtifact?.selectionMetrics)).toBe(false);
    expect(store.objectiveProfile).toBe("balanced");
    expect(store.activeBackendFallbackReason).toBe(
      "GPU planner probe failed; using CPU.",
    );
    expect(store.latestRun).toBeNull();
  });

  it("exposes active dataset provenance from latest runs before saved artifacts", () => {
    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord({
      datasetProvenance: {
        providerName: "saved-provider",
        providerSourceSurface: "saved-surface",
        quoteResolution: "",
        aggregatePriceField: "",
        contractReferenceAsOfUtc: "",
        quoteQuality: null,
      },
    });
    store.latestRun = makeRunSummary({
      datasetProvenance: {
        providerName: "polygon.io",
        providerSourceSurface: "options_aggs",
        quoteResolution: "top_of_book",
        aggregatePriceField: "mid",
        contractReferenceAsOfUtc: "2026-04-05T00:00:00Z",
        quoteQuality: null,
      },
    });

    expect(store.activeDatasetProvenance?.providerName).toBe("polygon.io");
    expect(store.activeDatasetProvenance?.providerSourceSurface).toBe(
      "options_aggs",
    );
  });

  it("refreshes saved artifact summaries even when retained-leader summary fields are absent", async () => {
    listOptionsExpressionArtifactsMock.mockResolvedValue([
      {
        ts: "2026-04-05T12:00:00Z",
        path: "C:/runs/vectorgrid_options_expression_legacy.json",
        bytes: 2048,
        parentArtifactId: "vectorgrid_run_1",
        parentStrategyKind: "DoubleMa",
        rootSymbol: "SPY",
        optionsDatasetId: "spy_options",
        backendUsed: "cpu_reference",
        backendFallbackReason: "",
        priceSemantics: "minute_aggregate",
        requestBackendPreference: "auto",
        requestFillModel: "aggregate_research",
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
        tupleCount: 1,
        bestRobustScore: 0.18,
        bestSelectionExpectancyPerPremium: 0.2,
        bestHoldoutExpectancyPerPremium: null,
        bestSealedExpectancyPerPremium: null,
        bestStressExpectancyPerPremium: null,
        bestSealedStressExpectancyPerPremium: null,
        bestHarsherStressExpectancyPerPremium: null,
        bestSealedHarsherStressExpectancyPerPremium: null,
        bestExtremeStressExpectancyPerPremium: null,
        bestSealedExtremeStressExpectancyPerPremium: null,
        topValidationVerdict: "search_only",
        validatedTupleCount: 0,
        stressFragileTupleCount: 0,
        hasHoldoutMetrics: false,
        hasSealedEvaluationMetrics: false,
        hasStressMetrics: false,
        hasHarsherStressMetrics: false,
        hasExtremeStressMetrics: false,
      } as any,
    ]);

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");

    await store.refreshArtifacts();

    expect(store.artifacts).toHaveLength(1);
    expect(store.selectedArtifactPath).toBe(
      "C:/runs/vectorgrid_options_expression_legacy.json",
    );
    expect(store.statusLine).toContain(
      "Loaded 1 saved options expression artifact",
    );
  });

  it("restores the matching realism lane from reopened artifact semantics", async () => {
    loadOptionsExpressionArtifactMock.mockResolvedValue(
      makeArtifactRecord({
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
        },
        diagnostics: {
          parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
          optionsDatasetId: "spy_quotes",
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
    );

    const store = useOptionsExpressionStore();
    store.datasets = [
      {
        optionsDatasetId: "spy_quotes_alt",
        rootSymbol: "SPY",
        priceSemantics: "top_of_book_quote",
        contractCount: 2,
        rowCount: 24,
      },
      {
        optionsDatasetId: "spy_quotes",
        rootSymbol: "SPY",
        priceSemantics: "top_of_book_quote",
        contractCount: 2,
        rowCount: 24,
      },
    ] as any;
    store.selectedDatasetId = "spy_quotes_alt";
    await store.loadArtifact(
      "C:/runs/vectorgrid_options_expression_quotes.json",
    );

    expect(store.activeArtifact?.priceSemantics).toBe("top_of_book_quote");
    expect(store.realismLane).toBe("quote_aware");
    expect(store.importPriceSemantics).toBe("top_of_book_quote");
    expect(store.fillModel).toBe("quote_conservative");
    expect(store.selectedDatasetId).toBe("spy_quotes");
    expect(store.statusLine).toContain("pinned replay to dataset spy_quotes");
  });

  it("previews and runs subsequent validation for the active artifact", async () => {
    analyzeOptionsExpressionValidationFeasibilityMock.mockResolvedValue({
      replayBuildMs: 2,
      replayBuildTiming: null,
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      sourceArtifactHash: "source-hash",
      optionsDatasetId: "spy_options",
      optionsDatasetHash: "hash-1",
      datasetPolicy: "require_exact_replay",
      replacementDataset: false,
      validationPlan: {
        mode: "original_embedded_splits",
        startTimestampMs: null,
        endTimestampMs: null,
        trainSessionCount: null,
        validationSessionCount: null,
        stepSessionCount: null,
      },
      tupleSelection: {
        includeSealedFinalists: true,
        includeRobustLeader: true,
        includeHoldoutLeader: true,
        tupleIndexes: [3],
      },
      selectedTupleIndexes: [0, 3],
      selectedTupleCount: 2,
      priceSemantics: "minute_aggregate",
      datasetProvenance: null,
      notes: ["Validation is eligible for exact replay."],
    });
    validateOptionsExpressionArtifactMock.mockResolvedValue(
      makeValidationSummary(),
    );

    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord();
    store.selectedDatasetId = "spy_options";
    store.validationTupleIndexesInput = "3";

    await store.previewValidationFeasibility();
    await store.validateActiveArtifact();

    expect(analyzeOptionsExpressionValidationFeasibilityMock).toHaveBeenCalledWith(
      expect.objectContaining({
        artifact_path: "C:/runs/vectorgrid_options_expression_1.json",
        options_dataset_id: "spy_options",
        dataset_policy: "require_exact_replay",
        tuple_selection: expect.objectContaining({
          include_sealed_finalists: true,
          include_robust_leader: true,
          include_holdout_leader: true,
          tuple_indexes: [3],
        }),
      }),
    );
    expect(validateOptionsExpressionArtifactMock).toHaveBeenCalledTimes(1);
    expect(store.latestValidation?.result.acceptanceSummary.verdict).toBe(
      "accepted",
    );
    expect(store.statusLine).toContain("Validation accepted");
  });

  it("builds date-range holdout validation requests with UTC timestamps", () => {
    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord();
    store.selectedDatasetId = "spy_options";
    store.validationPlanMode = "date_range_holdout";
    store.validationStartDateInput = "2026-04-01";
    store.validationEndDateInput = "2026-04-03";

    const req = store.buildValidationRequest();

    expect(req.validation_plan).toEqual({
      mode: "date_range_holdout",
      start_timestamp_ms: Date.parse("2026-04-01T00:00:00.000Z"),
      end_timestamp_ms: Date.parse("2026-04-03T00:00:00.000Z"),
    });
  });

  it("rejects incomplete date-range validation requests before invoke", () => {
    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord();
    store.selectedDatasetId = "spy_options";
    store.validationPlanMode = "date_range_holdout";
    store.validationStartDateInput = "2026-04-03";
    store.validationEndDateInput = "2026-04-01";

    expect(() => store.buildValidationRequest()).toThrow(
      /start date must be before/i,
    );
  });

  it("builds walk-forward validation requests with session counts", () => {
    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord();
    store.selectedDatasetId = "spy_options";
    store.validationPlanMode = "rolling_session_walk_forward";
    store.validationTrainSessionCountInput = "8";
    store.validationSessionCountInput = "2";
    store.validationStepSessionCountInput = "2";

    const req = store.buildValidationRequest();

    expect(req.validation_plan).toEqual({
      mode: "rolling_session_walk_forward",
      train_session_count: 8,
      validation_session_count: 2,
      step_session_count: 2,
    });
  });

  it("rejects invalid walk-forward session counts before invoke", () => {
    const store = useOptionsExpressionStore();
    store.activeArtifact = makeArtifactRecord();
    store.selectedDatasetId = "spy_options";
    store.validationPlanMode = "anchored_walk_forward";
    store.validationTrainSessionCountInput = "0";
    store.validationSessionCountInput = "1";
    store.validationStepSessionCountInput = "1";

    expect(() => store.buildValidationRequest()).toThrow(
      /Train sessions must be a positive whole number/i,
    );
  });

  it("leaves dataset selection empty when a reopened artifact dataset is missing", async () => {
    loadOptionsExpressionArtifactMock.mockResolvedValue(
      makeArtifactRecord({
        artifactPath:
          "C:/runs/vectorgrid_options_expression_missing_dataset.json",
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
        },
        diagnostics: makeDiagnosticsSummary({
          optionsDatasetId: "spy_quotes_missing",
        }),
        selectionMetrics: [],
      }),
    );

    const store = useOptionsExpressionStore();
    store.datasets = [
      {
        optionsDatasetId: "spy_quotes_alt",
        rootSymbol: "SPY",
        priceSemantics: "top_of_book_quote",
        contractCount: 2,
        rowCount: 24,
      },
    ] as any;
    await store.loadArtifact(
      "C:/runs/vectorgrid_options_expression_missing_dataset.json",
    );

    expect(store.selectedDatasetId).toBe("");
    expect(store.statusLine).toContain(
      "dataset spy_quotes_missing is not currently imported",
    );
  });

  it("rejects invalid negative filter values before running options expressions", async () => {
    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.selectedDatasetId = "spy_options";
    store.datasets = [
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
      },
    ] as any;
    store.maxPremiumInput = "-1";

    await store.runCurrentExpression();

    expect(runOptionsExpressionMock).not.toHaveBeenCalled();
    expect(store.statusLine).toContain(
      "Max premium must be a non-negative number.",
    );
  });

  it("rejects inverted premium ranges before feasibility preview", async () => {
    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.selectedDatasetId = "spy_options";
    store.datasets = [
      {
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        priceSemantics: "minute_aggregate",
      },
    ] as any;
    store.minPremiumInput = "2";
    store.maxPremiumInput = "1";

    await store.previewCurrentFeasibility();

    expect(analyzeOptionsExpressionFeasibilityMock).not.toHaveBeenCalled();
    expect(store.statusLine).toContain(
      "Min premium must be less than or equal to max premium.",
    );
  });

  it("loads tuple inspection rows for the active artifact", async () => {
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
          exitReason: "target_hit",
        },
      ],
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

    await store.loadTupleInspection(1);

    expect(inspectOptionsExpressionTupleMock).toHaveBeenCalledWith({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      tupleIndex: 1,
    });
    expect(store.activeTupleInspection?.rows[0]?.contractId).toBe("SPY_C_1");
    expect(store.statusLine).toContain("Loaded 1 event row");
  });

  it("loads event-path inspection rows for the active artifact", async () => {
    inspectOptionsExpressionEventPathMock.mockResolvedValue({
      tupleIndex: 1,
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

    await store.loadEventPathInspection(1, 0);

    expect(inspectOptionsExpressionEventPathMock).toHaveBeenCalledWith({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      tupleIndex: 1,
      eventIndex: 0,
    });
    expect(store.activeEventPathInspection?.eventId).toBe("event-1");
    expect(store.activeEventPathInspection?.pointCount).toBe(2);
    expect(store.statusLine).toContain("Loaded 2 path points");
  });

  it("exports the active ranked tuples to CSV with a desktop save path", async () => {
    exportOptionsExpressionCsvMock.mockResolvedValue({
      mode: "saved",
      filename: "vectorgrid_run_1_options_expression.csv",
      path: "C:/exports/vectorgrid_run_1_options_expression.csv",
      bytes: 512,
      revealed: true,
    });
    const rankedResults = Array.from({ length: 12 }, (_, index) => ({
      delayMinutes: index % 3,
      dteDays: 1 + (index % 2),
      moneynessBucket: index % 2 === 0 ? "atm" : "otm_1",
      holdMinutes: 15 + index,
      targetReturn: 0.25,
      stopReturn: 0.25,
      exitTemplate: "fixed_hold_or_target_stop",
      fillModel: "aggregate_research",
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
    }));

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
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
      rankedResults,
    });

    await store.exportActiveCsv();

    const exportInput = exportOptionsExpressionCsvMock.mock.calls[0]?.[0];
    expect(exportOptionsExpressionCsvMock).toHaveBeenCalledWith(
      expect.objectContaining({
        filenameStem: "vectorgrid_run_1_options_expression",
        rootSymbol: "SPY",
        backendUsed: "cpu_reference",
        priceSemantics: "minute_aggregate",
      }),
    );
    expect(exportInput?.tuples).toHaveLength(12);
    expect(store.statusLine).toContain("Saved and revealed 12 ranked tuples");
    expect(store.statusLine).toContain(
      "C:/exports/vectorgrid_run_1_options_expression.csv",
    );
  });

  it("resets active selection when the parent artifact changes", () => {
    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.selectedArtifactPath =
      "C:/runs/vectorgrid_options_expression_old.json";
    store.activeArtifact = makeArtifactRecord({
      artifactPath: "C:/runs/vectorgrid_options_expression_old.json",
      staging: {
        candidateSlotCount: 12,
        compiledCandidateCount: 9,
        baselineExitSummaryCount: 24,
        conservativeExitSummaryCount: 24,
        harsherExitSummaryCount: 24,
      },
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
      selectionMetrics: [],
    });
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_latest.json",
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
      rankedResults: [],
    });

    store.attachParent("C:/runs/vectorgrid_run_2.json", "DoubleMa", "QQQ.csv");

    expect(store.parentArtifactId).toBe("vectorgrid_run_2");
    expect(store.parentRootHint).toBe("QQQ");
    expect(store.selectedArtifactPath).toBe("");
    expect(store.activeArtifact).toBeNull();
    expect(store.latestRun).toBeNull();
  });

  it("exports active tuple inspection rows to CSV with a desktop save path", async () => {
    exportOptionsExpressionInspectionCsvMock.mockResolvedValue({
      mode: "saved",
      filename: "vectorgrid_options_expression_1_tuple_1_inspection.csv",
      path: "C:/exports/vectorgrid_options_expression_1_tuple_1_inspection.csv",
      bytes: 384,
      revealed: true,
    });

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 1,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        totalEvents: 2,
        bullishEvents: 1,
        bearishEvents: 1,
        searchEvents: 1,
        holdoutEvents: 1,
        sealedTestEvents: 0,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults: [],
    });
    store.activeTupleInspection = {
      tupleIndex: 0,
      tupleCount: 1,
      totalEvents: 2,
      executedEvents: 1,
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
          validTrades: 1,
          invalidEvents: 1,
          invalidRate: 0.5,
          expectancyPerTrade: 12,
          expectancyPerPremium: 0.2,
          medianReturnOnPremium: 0.15,
          winRate: 1,
          totalPnlPerContract: 24,
        },
        holdoutMetrics: null,
        sealedEvaluationMetrics: null,
        stressMetrics: null,
      },
      rows: [
        {
          eventIndex: 0,
          eventId: "evt-1",
          split: "search",
          side: "bullish",
          decisionTimestamp: 100,
          earliestTradableTimestamp: 160,
          requestedEntryTimestamp: 160,
          rawUnderlierPrice: 500,
          baseExitTimestamp: null,
          status: "executed",
          candidateInvalidReason: "",
          contractId: "SPY_C_1",
          expirationDate: "2026-04-12",
          strike: 505,
          multiplier: 100,
          entryTimestamp: 160,
          entryOptionPrice: 1.2,
          entryUnderlierPrice: 500.1,
          exitTimestamp: 220,
          exitOptionPrice: 1.5,
          premiumPaid: 120,
          pnlPerContract: 30,
          returnOnPremium: 0.25,
          entryLagMinutes: 0,
          minutesToExpiryAtEntry: 1440,
          realizedHoldMinutes: 60,
          pathOptionHigh: 1.6,
          pathOptionLow: 1.1,
          pathUnderlierHigh: 501.5,
          pathUnderlierLow: 499.8,
          exitReason: "target_hit",
        },
      ],
    };

    await store.exportActiveInspectionCsv();

    expect(exportOptionsExpressionInspectionCsvMock).toHaveBeenCalledWith(
      expect.objectContaining({
        filenameStem: "vectorgrid_options_expression_1_tuple_1_inspection",
        artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
        tupleIndex: 0,
      }),
    );
    expect(store.statusLine).toContain("Saved and revealed 1 inspection row");
    expect(store.statusLine).toContain(
      "C:/exports/vectorgrid_options_expression_1_tuple_1_inspection.csv",
    );
  });

  it("treats a cancelled ranked-tuple export as a non-error", async () => {
    exportOptionsExpressionCsvMock.mockResolvedValue({
      mode: "cancelled",
      filename: "vectorgrid_run_1_options_expression.csv",
      path: null,
      bytes: 256,
      revealed: false,
    });
    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 1,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
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
            validTrades: 1,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 12,
            expectancyPerPremium: 0.2,
            medianReturnOnPremium: 0.15,
            winRate: 1,
            totalPnlPerContract: 24,
          },
          holdoutMetrics: null,
          sealedEvaluationMetrics: null,
          stressMetrics: null,
        },
      ],
    });

    await store.exportActiveCsv();

    expect(store.statusLine).toBe("CSV export cancelled.");
  });

  it("exports active event path rows to CSV with a desktop save path", async () => {
    exportOptionsExpressionEventPathCsvMock.mockResolvedValue({
      mode: "saved",
      filename: "vectorgrid_options_expression_1_tuple_1_evt-1_path.csv",
      path: "C:/exports/vectorgrid_options_expression_1_tuple_1_evt-1_path.csv",
      bytes: 320,
      revealed: false,
    });

    const store = useOptionsExpressionStore();
    store.attachParent("C:/runs/vectorgrid_run_1.json", "DoubleMa", "SPY.csv");
    store.latestRun = makeRunSummary({
      artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
      artifactTs: "",
      parentArtifactId: "",
      optionsDatasetHash: "",
      backendUsed: "cpu_reference",
      priceSemantics: "minute_aggregate",
      tupleCount: 1,
      request: null,
      staging: null,
      diagnostics: {
        parentArtifactPath: "C:/runs/vectorgrid_run_1.json",
        optionsDatasetId: "spy_options",
        rootSymbol: "SPY",
        totalEvents: 2,
        bullishEvents: 1,
        bearishEvents: 1,
        searchEvents: 1,
        holdoutEvents: 1,
        sealedTestEvents: 0,
        datasetContracts: 3,
        datasetRows: 42,
        coverageStart: null,
        coverageEnd: null,
      },
      rankedResults: [],
    });
    store.activeEventPathInspection = {
      tupleIndex: 0,
      eventIndex: 0,
      eventId: "evt-1",
      row: {
        eventIndex: 0,
        eventId: "evt-1",
        split: "search",
        side: "bullish",
        decisionTimestamp: 100,
        earliestTradableTimestamp: 160,
        requestedEntryTimestamp: 160,
        rawUnderlierPrice: 500,
        baseExitTimestamp: null,
        status: "executed",
        candidateInvalidReason: "",
        contractId: "SPY_C_1",
        companionContractId: "",
        expirationDate: "2026-04-12",
        strike: 505,
        companionStrike: null,
        multiplier: 100,
        entryTimestamp: 160,
        entryOptionPrice: 1.2,
        companionEntryOptionPrice: null,
        entryUnderlierPrice: 500.1,
        exitTimestamp: 220,
        exitOptionPrice: 1.5,
        companionExitOptionPrice: null,
        premiumPaid: 120,
        pnlPerContract: 30,
        returnOnPremium: 0.25,
        entryLagMinutes: 0,
        minutesToExpiryAtEntry: 1440,
        realizedHoldMinutes: 60,
        pathOptionHigh: 1.6,
        pathOptionLow: 1.1,
        pathUnderlierHigh: 501.5,
        pathUnderlierLow: 499.8,
        exitReason: "target_hit",
      },
      pointCount: 2,
      points: [
        {
          timestampMs: 1700000000000,
          optionOpen: 1.2,
          optionHigh: 1.3,
          optionLow: 1.1,
          optionClose: 1.25,
          underlyingPrice: 500.5,
        },
        {
          timestampMs: 1700000060000,
          optionOpen: 1.25,
          optionHigh: 1.5,
          optionLow: 1.2,
          optionClose: 1.45,
          underlyingPrice: 501.2,
        },
      ],
    };

    await store.exportActiveEventPathCsv();

    expect(exportOptionsExpressionEventPathCsvMock).toHaveBeenCalledWith(
      expect.objectContaining({
        filenameStem: "vectorgrid_options_expression_1_tuple_1_evt-1_path",
        artifactPath: "C:/runs/vectorgrid_options_expression_1.json",
        tupleIndex: 0,
        eventId: "evt-1",
      }),
    );
    expect(store.statusLine).toContain("Saved 2 replay points for event evt-1");
    expect(store.statusLine).toContain(
      "C:/exports/vectorgrid_options_expression_1_tuple_1_evt-1_path.csv",
    );
  });
});
