import { invokeCommand } from "../tauri/invoke";
import {
  parseBoundaryArray,
  parseBoundaryObject,
} from "./serviceBoundarySchemas";
import {
  buildOptionsExpressionValidationStageCompletion,
  normalizeStageCompletions,
  type StageCompletionPacketV1,
} from "../utils/core/stageCompletion";

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizeNumber(value: unknown, fallback = 0): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function normalizeOptionalNumber(value: unknown): number | null {
  if (value == null) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizeOptionalBoolean(value: unknown): boolean | null {
  if (value === true) return true;
  if (value === false) return false;
  if (value == null) return null;
  if (typeof value === "string") {
    const lowered = value.trim().toLowerCase();
    if (lowered === "true") return true;
    if (lowered === "false") return false;
  }
  return null;
}

function normalizeStringList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0);
}

export interface OptionsExpressionExactnessSemanticsSummary {
  exactnessClass: string;
  medianReplayContract: string;
  retainedFrontierCertification: string;
  exactPruningContract: string;
}

function normalizeExactnessSemantics(
  raw: unknown,
): OptionsExpressionExactnessSemanticsSummary | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as Record<string, unknown>;
  const exactnessClass = normalizeText(row.exactness_class);
  const medianReplayContract = normalizeText(row.median_replay_contract);
  const retainedFrontierCertification = normalizeText(
    row.retained_frontier_certification,
  );
  const exactPruningContract = normalizeText(row.exact_pruning_contract);
  if (
    !exactnessClass &&
    !medianReplayContract &&
    !retainedFrontierCertification &&
    !exactPruningContract
  ) {
    return null;
  }
  return {
    exactnessClass,
    medianReplayContract,
    retainedFrontierCertification,
    exactPruningContract,
  };
}

export interface OptionsDatasetManifestSummary {
  version: number;
  optionsDatasetId: string;
  rootSymbol: string;
  providerName: string;
  providerSourceSurface: string;
  providerEntitlementProfile: string;
  quoteResolution: string;
  priceSemantics: string;
  calendarId: string;
  timezone: string;
  coverageStart: number;
  coverageEnd: number;
  normalizationVersion: number;
  quoteSemanticsVersion: number;
  corporateActionMappingVersion: number;
  contractReferenceSource: string;
  contractReferenceAsOfUtc: string;
  underlierReferenceSource: string;
  datasetPath: string;
  rowCount: number;
  contractCount: number;
  aggregatePriceField: string;
  normalizedRowsSha256?: string;
  manifestSemanticSha256?: string;
}

export interface PolygonOptionsDatasetDownloadRequestSummary {
  rootSymbol: string;
  from: string;
  to: string;
  optionsDatasetId?: string;
  contractType?: string;
  expirationDateFrom?: string;
  expirationDateTo?: string;
  maxDteDays?: number | null;
  strikeMin?: number | null;
  strikeMax?: number | null;
  maxContracts?: number | null;
  providerEntitlementProfile?: string;
  contractReferenceAsOfUtc?: string;
}

export interface PolygonOptionsDatasetDownloadSummary {
  manifest: OptionsDatasetManifestSummary;
  contractsDiscovered: number;
  contractsSelected: number;
  contractsWithRows: number;
  downloadedOptionRows: number;
  skippedContracts: number;
  underlyingCsvPath: string;
  usedUnderlyingCacheOnly: boolean;
}

export interface OptionsExpressionMetricsSummary {
  validTrades: number;
  invalidEvents: number;
  invalidRate: number;
  expectancyPerTrade: number;
  expectancyPerPremium: number;
  meanReturnOnPremium?: number;
  returnVolatilityOnPremium?: number;
  medianReturnOnPremium: number;
  winRate: number;
  totalPnlPerContract: number;
}

export type OptionsObjectiveProfile =
  | "balanced"
  | "validation_first"
  | "stress_resistant"
  | "liquidity_aware"
  | string;

export interface OptionsObjectiveConfigSummary {
  profile: OptionsObjectiveProfile;
}

export interface OptionsObjectiveScoreComponentsSummary {
  profile: OptionsObjectiveProfile;
  searchUtility: number;
  searchWeight: number;
  searchContribution: number;
  holdoutUtility: number;
  holdoutWeight: number;
  holdoutContribution: number;
  stressUtility: number;
  stressWeight: number;
  stressContribution: number;
  harsherStressUtility: number;
  harsherStressWeight: number;
  harsherStressContribution: number;
  extremeStressUtility: number;
  extremeStressWeight: number;
  extremeStressContribution: number;
  totalScore: number;
}

export interface OptionsStaticFilterBundleSummary {
  maxEntryLagMinutes: number | null;
  maxPremium: number | null;
  minPremium: number | null;
  minVolume: number | null;
  minOpenInterest: number | null;
}

export interface OptionsExpressionRequestSummary {
  parentArtifactPath: string;
  parentCandidateIndex: number | null;
  optionsDatasetId: string;
  structureFamily?: string;
  backendPreference?: string;
  objectiveConfig?: OptionsObjectiveConfigSummary;
  splitAssignmentMode: string;
  delayMinutes: number[];
  dteBuckets: number[];
  moneynessBuckets: string[];
  holdMinutes: number[];
  targetReturnBuckets: number[];
  stopReturnBuckets: number[];
  exitTemplate: string;
  fillModel: string;
  filters: OptionsStaticFilterBundleSummary;
  topK: number;
  minValidTrades: number | null;
  splitRatioSearch: number;
  splitRatioHoldout: number;
  sealedFinalistCount?: number;
}

export type OptionsExpressionValidationVerdict =
  | "search_only"
  | "holdout_supported"
  | "sealed_finalist"
  | "stress_fragile";

export interface OptionsExpressionTupleSummary {
  structureFamily?: string;
  delayMinutes: number;
  dteDays: number;
  moneynessBucket: string;
  holdMinutes: number;
  targetReturn: number | null;
  stopReturn: number | null;
  exitTemplate: string;
  fillModel: string;
  preSealedRank?: number | undefined;
  sealedFinalist?: boolean | undefined;
  sealedFinalRank?: number | undefined;
  validationVerdict?: OptionsExpressionValidationVerdict | undefined;
  objectiveScoreComponents?: OptionsObjectiveScoreComponentsSummary | null;
  robustScore: number;
  selectionMetrics: OptionsExpressionMetricsSummary;
  holdoutMetrics: OptionsExpressionMetricsSummary | null;
  sealedEvaluationMetrics: OptionsExpressionMetricsSummary | null;
  stressMetrics: OptionsExpressionMetricsSummary | null;
  sealedStressMetrics?: OptionsExpressionMetricsSummary | null;
  harsherStressMetrics?: OptionsExpressionMetricsSummary | null;
  sealedHarsherStressMetrics?: OptionsExpressionMetricsSummary | null;
  extremeStressMetrics?: OptionsExpressionMetricsSummary | null;
  sealedExtremeStressMetrics?: OptionsExpressionMetricsSummary | null;
}

export interface OptionsExpressionDiagnosticsSummary {
  parentArtifactPath: string;
  optionsDatasetId: string;
  rootSymbol: string;
  totalEvents: number;
  bullishEvents: number;
  bearishEvents: number;
  searchEvents: number;
  holdoutEvents: number;
  sealedTestEvents: number;
  datasetContracts: number;
  datasetRows: number;
  coverageStart: number | null;
  coverageEnd: number | null;
}

export interface OptionsExpressionStagingSummary {
  candidateSlotCount: number;
  compiledCandidateCount: number;
  baselineExitSummaryCount: number;
  conservativeExitSummaryCount: number;
  harsherExitSummaryCount: number;
}

export interface OptionsCandidateInvalidReasonCountSummary {
  reason: string;
  count: number;
}

export interface OptionsDelayBucketCoverageSummary {
  delayMinutes: number;
  slotCount: number;
  validSlotCount: number;
  invalidSlotCount: number;
  validRate: number;
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[];
}

export interface OptionsDteBucketCoverageSummary {
  dteDays: number;
  slotCount: number;
  validSlotCount: number;
  invalidSlotCount: number;
  validRate: number;
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[];
}

export interface OptionsMoneynessBucketCoverageSummary {
  moneynessBucket: string;
  slotCount: number;
  validSlotCount: number;
  invalidSlotCount: number;
  validRate: number;
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[];
}

export interface OptionsCandidateAxisCoverageSummary {
  delayMinutes: OptionsDelayBucketCoverageSummary[];
  dteDays: OptionsDteBucketCoverageSummary[];
  moneynessBuckets: OptionsMoneynessBucketCoverageSummary[];
}

export interface OptionsExpressionFeasibilityTimingSummary {
  tupleCompileMs: number;
  baselineCandidateCompileMs: number;
  baselineExitSummaryCompileMs: number;
  conservativeCandidateCompileMs: number;
  conservativeExitSummaryCompileMs: number;
  harsherCandidateCompileMs: number;
  harsherExitSummaryCompileMs: number;
  cpuRankingMs: number;
  cpuRankingUs: number;
  gpuRankingMs: number | null;
  gpuRankingUs: number | null;
  gpuTensorScoreMs: number | null;
  gpuTensorScoreUs: number | null;
  gpuExactifyMs: number | null;
  gpuExactifyUs: number | null;
  gpuSealedStageMs: number | null;
  gpuSealedStageUs: number | null;
  gpuSpeedupVsCpu: number | null;
}

export interface OptionsExpressionFeasibilityEventCoverageSummary {
  totalEventsWithCandidates: number;
  searchEventsWithCandidates: number;
  holdoutEventsWithCandidates: number;
  sealedTestEventsWithCandidates: number;
  totalEventsWithBaselineOutcomes: number;
  searchEventsWithBaselineOutcomes: number;
  holdoutEventsWithBaselineOutcomes: number;
  sealedTestEventsWithBaselineOutcomes: number;
  totalEventsWithConservativeOutcomes: number;
  searchEventsWithConservativeOutcomes: number;
  holdoutEventsWithConservativeOutcomes: number;
  sealedTestEventsWithConservativeOutcomes: number;
  totalEventsWithHarsherOutcomes: number;
  searchEventsWithHarsherOutcomes: number;
  holdoutEventsWithHarsherOutcomes: number;
  sealedTestEventsWithHarsherOutcomes: number;
}

export interface OptionsExpressionOverlapWindowSummary {
  contractReferenceAsOfUtc: string | null;
  totalEventsInDatasetWindow: number;
  searchEventsInDatasetWindow: number;
  holdoutEventsInDatasetWindow: number;
  sealedTestEventsInDatasetWindow: number;
  totalEventsBeforeDatasetWindow: number;
  totalEventsAfterDatasetWindow: number;
  searchEventsBeforeDatasetWindow: number;
  searchEventsAfterDatasetWindow: number;
  holdoutEventsBeforeDatasetWindow: number;
  holdoutEventsAfterDatasetWindow: number;
  sealedTestEventsBeforeDatasetWindow: number;
  sealedTestEventsAfterDatasetWindow: number;
  totalEventsBeforeContractReferenceAsOf: number;
  searchEventsBeforeContractReferenceAsOf: number;
  holdoutEventsBeforeContractReferenceAsOf: number;
  sealedTestEventsBeforeContractReferenceAsOf: number;
  totalOverlapRate: number;
  searchOverlapRate: number;
  holdoutOverlapRate: number;
  sealedTestOverlapRate: number;
}

export interface OptionsExpressionSplitInvalidReasonSummary {
  searchInvalidReasons: OptionsCandidateInvalidReasonCountSummary[];
  holdoutInvalidReasons: OptionsCandidateInvalidReasonCountSummary[];
  sealedTestInvalidReasons: OptionsCandidateInvalidReasonCountSummary[];
}

export type OptionsExpressionCoverageAdequacyVerdict =
  | "unknown"
  | "healthy"
  | "weak_dataset_overlap"
  | "search_overlap_missing"
  | "search_coverage_weak"
  | "holdout_coverage_weak"
  | "sealed_coverage_weak"
  | "contract_eligibility_fragile"
  | "late_entry_fragile"
  | "strike_window_fragile"
  | "mixed_fragile";

export interface OptionsExpressionCoverageAdequacySummary {
  verdict: OptionsExpressionCoverageAdequacyVerdict;
  nonPromotable: boolean;
  reasons: string[];
  searchCandidateCoverageRate: number;
  holdoutCandidateCoverageRate: number;
  sealedTestCandidateCoverageRate: number;
  searchBaselineOutcomeCoverageRate: number;
  holdoutBaselineOutcomeCoverageRate: number;
  sealedTestBaselineOutcomeCoverageRate: number;
  dominantInvalidReason: string;
  dominantInvalidReasonShare: number;
}

export interface OptionsExpressionFeasibilityCacheStatsSummary {
  structuralCacheHits: number;
  structuralCacheMisses: number;
  structuralCacheEntries: number;
  summaryCacheHits: number;
  summaryCacheMisses: number;
  summaryCacheEntries: number;
  gpuSharedInputCacheHits: number;
  gpuSharedInputCacheMisses: number;
  gpuOutcomeInputCacheHits: number;
  gpuOutcomeInputCacheMisses: number;
  gpuOutputBufferReuses: number;
  gpuOutputBufferAllocations: number;
  gpuTupleChunkCount: number;
  gpuTupleChunkSize: number;
  gpuKernelLaunchCount: number;
  gpuTupleBudgetBytes: number;
  gpuOutcomeTensorBytes: number;
  gpuSharedInputBytesPerChunk: number;
  gpuOutputBytesPerChunk: number;
  gpuTotalDeviceBytesPerChunk: number;
  gpuDeviceTotalMemoryBytes: number;
  gpuDeviceFreeMemoryBytes: number;
  gpuDeviceUsableMemoryBytes: number;
  gpuMemoryBudgetSource: string;
  gpuOutcomeTensorFitsInMemory: boolean;
  gpuChunkPlanFitsInMemory: boolean;
  gpuPlanNotes: string[];
}

export interface OptionsExpressionAutoBackendDecisionSummary {
  recommendedBackend: string;
  gpuEligible: boolean;
  reasons: string[];
}

export interface OptionsExpressionReplayBuildTimingSummary {
  totalUs: number;
  parentArtifactValidationUs: number;
  sourceResolutionUs: number;
  parentSignalProcessCacheLookupUs: number;
  parentSignalProcessCacheHit: boolean;
  persistedSignalManifestCacheLookupUs: number;
  persistedSignalManifestCacheHit: boolean;
  candleReplayLoadUs: number;
  parentReplayAndCandidateDetailUs: number;
  signalManifestMaterializationUs: number;
  optionsDatasetManifestValidationUs: number;
  optionsDatasetProcessCacheLookupUs: number;
  optionsDatasetProcessCacheHit: boolean;
  optionsDatasetPersistedCacheLookupUs: number;
  optionsDatasetPersistedCacheHit: boolean;
  optionsDatasetCsvLoadUs: number;
  requestNormalizationUs: number;
}

export interface AnalyzeOptionsExpressionFeasibilitySummary {
  replayBuildMs: number;
  totalPreviewMs: number;
  replayBuildTiming: OptionsExpressionReplayBuildTimingSummary | null;
  exactnessSemantics?: OptionsExpressionExactnessSemanticsSummary | null;
  objectiveConfig?: OptionsObjectiveConfigSummary;
  diagnostics: OptionsExpressionDiagnosticsSummary;
  priceSemantics: string;
  fillModel: string;
  bestAvailableBackend: string;
  autoBackendDecision: OptionsExpressionAutoBackendDecisionSummary;
  compiledTupleCount: number;
  perEventCandidateSlots: number;
  perCandidateExitSummaries: number;
  candidateSlotCount: number;
  compiledCandidateCount: number;
  candidateDensity: number;
  invalidSlotCount: number;
  invalidSlotRate: number;
  baselineExitSummaryCount: number;
  baselineCompiledExitSummaryCount: number;
  baselineExitSummaryDensity: number;
  conservativeExitSummaryCount: number;
  conservativeCompiledExitSummaryCount: number;
  conservativeExitSummaryDensity: number;
  harsherExitSummaryCount: number;
  harsherCompiledExitSummaryCount: number;
  harsherExitSummaryDensity: number;
  projectedEventTupleEvaluations: number;
  structuralCacheBytesEstimate: number;
  baselineSummaryBytesEstimate: number;
  conservativeSummaryBytesEstimate: number;
  harsherSummaryBytesEstimate: number;
  timings: OptionsExpressionFeasibilityTimingSummary;
  eventCoverage: OptionsExpressionFeasibilityEventCoverageSummary;
  overlapWindow: OptionsExpressionOverlapWindowSummary;
  splitInvalidReasons: OptionsExpressionSplitInvalidReasonSummary;
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary;
  cacheStats: OptionsExpressionFeasibilityCacheStatsSummary;
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[];
  axisCoverage: OptionsCandidateAxisCoverageSummary;
}

export interface RunOptionsExpressionSummary {
  artifactPath: string;
  artifactTs: string;
  parentArtifactId: string;
  optionsDatasetHash: string;
  optionsDatasetNormalizedRowsSha256?: string;
  optionsDatasetManifestSemanticSha256?: string;
  datasetProvenance: OptionsExpressionDatasetProvenanceSummary | null;
  exactnessSemantics?: OptionsExpressionExactnessSemanticsSummary | null;
  objectiveConfig?: OptionsObjectiveConfigSummary;
  backendUsed: string;
  backendFallbackReason?: string;
  priceSemantics: string;
  tupleCount: number;
  request: OptionsExpressionRequestSummary | null;
  diagnostics: OptionsExpressionDiagnosticsSummary;
  staging: OptionsExpressionStagingSummary | null;
  overlapWindow: OptionsExpressionOverlapWindowSummary;
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary;
  splitInvalidReasons: OptionsExpressionSplitInvalidReasonSummary;
  rankedResults: OptionsExpressionTupleSummary[];
}

export interface OptionsExpressionArtifactSummary {
  ts: string;
  path: string;
  bytes: number;
  parentArtifactId: string;
  parentStrategyKind?: string;
  rootSymbol: string;
  optionsDatasetId: string;
  exactnessSemantics?: OptionsExpressionExactnessSemanticsSummary | null;
  backendUsed: string;
  backendFallbackReason?: string;
  priceSemantics: string;
  requestBackendPreference?: string;
  requestObjectiveProfile?: string;
  requestFillModel?: string;
  requestStructureFamily?: string;
  requestSplitAssignmentMode?: string;
  coverageAdequacyVerdict: OptionsExpressionCoverageAdequacyVerdict;
  coverageNonPromotable: boolean;
  searchOverlapRate: number;
  holdoutOverlapRate: number;
  sealedTestOverlapRate: number;
  searchCandidateCoverageRate: number;
  holdoutCandidateCoverageRate: number;
  sealedTestCandidateCoverageRate: number;
  searchBaselineOutcomeCoverageRate: number;
  holdoutBaselineOutcomeCoverageRate: number;
  sealedTestBaselineOutcomeCoverageRate: number;
  tupleCount: number;
  bestRobustScore: number;
  bestSelectionExpectancyPerPremium: number;
  bestHoldoutExpectancyPerPremium: number | null;
  bestSealedExpectancyPerPremium: number | null;
  bestStressExpectancyPerPremium: number | null;
  bestSealedStressExpectancyPerPremium: number | null;
  bestHarsherStressExpectancyPerPremium: number | null;
  bestSealedHarsherStressExpectancyPerPremium: number | null;
  bestExtremeStressExpectancyPerPremium: number | null;
  bestSealedExtremeStressExpectancyPerPremium: number | null;
  leaderSummary: OptionsExpressionArtifactLeaderSummary;
  topValidationVerdict?: OptionsExpressionValidationVerdict | undefined;
  sealedFinalistBudget?: number;
  sealedEvaluatedTupleCount?: number;
  validatedTupleCount?: number;
  stressFragileTupleCount?: number;
  supportedHoldoutTupleCount: number;
  supportedSealedTupleCount: number;
  supportedStressTupleCount: number;
  hasHoldoutMetrics: boolean;
  hasSealedEvaluationMetrics: boolean;
  hasStressMetrics: boolean;
  hasHarsherStressMetrics: boolean;
  hasExtremeStressMetrics: boolean;
}

export interface OptionsExpressionArtifactLeaderSummary {
  searchLeaderTupleIndex: number | null;
  robustLeaderTupleIndex: number | null;
  holdoutLeaderTupleIndex: number | null;
  sealedLeaderTupleIndex: number | null;
  stressLeaderTupleIndex: number | null;
  distinctLeaderCount: number;
  searchVsRobustDisagree: boolean;
  robustVsHoldoutDisagree: boolean | null;
  robustVsSealedDisagree: boolean | null;
  robustLeaderReachedSealedFinalist: boolean | null;
  robustLeaderSealedFinalRank: number | null;
}

export interface OptionsExpressionArtifactRecord {
  version: number;
  kind: string;
  ts: string;
  artifactPath: string;
  parentArtifactId: string;
  rootSymbol: string;
  optionsDatasetId: string;
  optionsDatasetHash: string;
  optionsDatasetNormalizedRowsSha256?: string;
  optionsDatasetManifestSemanticSha256?: string;
  datasetProvenance: OptionsExpressionDatasetProvenanceSummary | null;
  exactnessSemantics?: OptionsExpressionExactnessSemanticsSummary | null;
  backendUsed: string;
  backendFallbackReason?: string;
  priceSemantics: string;
  request: OptionsExpressionRequestSummary;
  diagnostics: OptionsExpressionDiagnosticsSummary;
  staging: OptionsExpressionStagingSummary | null;
  overlapWindow: OptionsExpressionOverlapWindowSummary;
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary;
  splitInvalidReasons: OptionsExpressionSplitInvalidReasonSummary;
  selectionMetrics: OptionsExpressionTupleSummary[];
}

export interface OptionsExpressionDatasetProvenanceSummary {
  providerName: string;
  providerSourceSurface: string;
  quoteResolution: string;
  aggregatePriceField: string;
  contractReferenceAsOfUtc: string;
  normalizedRowsSha256?: string;
  manifestSemanticSha256?: string;
  quoteQuality: OptionsExpressionDatasetQuoteQualitySummary | null;
}

export interface OptionsExpressionContractQuoteQualitySummary {
  contractId: string;
  rowCount: number;
  anyQuoteRowCount: number;
  bidAskRowCount: number;
  midRowCount: number;
  quoteAgeRowCount: number;
  staleQuoteRowCount: number;
  zeroBidRowCount: number;
  lockedBidAskRowCount: number;
  crossedBidAskRowCount: number;
  invalidBidAskRowCount: number;
  anyQuoteCoverageRate: number;
  bidAskCoverageRate: number;
  midCoverageRate: number;
  staleQuoteRate: number;
  meanSpreadBps: number | null;
  maxSpreadBps: number | null;
  maxQuoteAgeMs: number | null;
  warnings: string[];
}

export interface OptionsExpressionDatasetQuoteQualitySummary {
  priceSemantics: string;
  rowCount: number;
  contractCount: number;
  anyQuoteRowCount: number;
  bidRowCount: number;
  askRowCount: number;
  bidAskRowCount: number;
  midRowCount: number;
  quoteAgeRowCount: number;
  staleQuoteThresholdMs: number | null;
  staleQuoteRowCount: number;
  zeroBidRowCount: number;
  lockedBidAskRowCount: number;
  crossedBidAskRowCount: number;
  invalidBidAskRowCount: number;
  anyQuoteCoverageRate: number;
  bidAskCoverageRate: number;
  midCoverageRate: number;
  staleQuoteRate: number;
  meanSpreadBps: number | null;
  medianSpreadBps: number | null;
  maxSpreadBps: number | null;
  maxQuoteAgeMs: number | null;
  perContract: OptionsExpressionContractQuoteQualitySummary[];
  warnings: string[];
}

export interface OptionsExpressionValidationTupleSelectionSummary {
  includeSealedFinalists: boolean;
  includeRobustLeader: boolean;
  includeHoldoutLeader: boolean;
  tupleIndexes: number[];
}

export interface OptionsExpressionValidationPlanSummary {
  mode: string;
  startTimestampMs: number | null;
  endTimestampMs: number | null;
  trainSessionCount: number | null;
  validationSessionCount: number | null;
  stepSessionCount: number | null;
}

export interface OptionsExpressionValidationTupleDriftSummary {
  selectionExpectancyDelta: number;
  holdoutExpectancyDelta: number | null;
  sealedExpectancyDelta: number | null;
  stressExpectancyDelta: number | null;
  extremeStressExpectancyDelta: number | null;
  robustScoreDelta: number;
}

export interface OptionsExpressionValidationTupleResultSummary {
  sourceTupleIndex: number;
  sourceValidationVerdict?: OptionsExpressionValidationVerdict | undefined;
  sourceRobustScore: number;
  validatedTuple: OptionsExpressionTupleSummary;
  supportDeficient: boolean;
  drift: OptionsExpressionValidationTupleDriftSummary;
}

export interface OptionsExpressionValidationLaneSummary {
  lane: string;
  tupleCount: number;
  supportedTupleCount: number;
  supportDeficientTupleCount: number;
  minValidTrades: number | null;
  bestExpectancyPerPremium: number | null;
  worstExpectancyPerPremium: number | null;
  medianExpectancyPerPremium: number | null;
}

export interface OptionsExpressionValidationAcceptanceSummary {
  verdict: string;
  selectedTupleCount: number;
  validatedTupleCount: number;
  acceptedTupleCount: number;
  supportDeficientTupleCount: number;
  stressFragileTupleCount: number;
  minValidTrades: number | null;
  bestValidatedExpectancyPerPremium: number | null;
  worstStressExpectancyPerPremium: number | null;
  notes: string[];
}

export interface OptionsExpressionValidationResultSummary {
  version: number;
  kind: string;
  ts: string;
  validationArtifactPath: string;
  sourceArtifactPath: string;
  sourceArtifactHash: string;
  parentArtifactId: string;
  parentArtifactHash: string;
  signalManifestId: string;
  rootSymbol: string;
  optionsDatasetId: string;
  optionsDatasetHash: string;
  optionsDatasetNormalizedRowsSha256?: string;
  optionsDatasetManifestSemanticSha256?: string;
  datasetPolicy: string;
  replacementDataset: boolean;
  validationPlan: OptionsExpressionValidationPlanSummary;
  tupleSelection: OptionsExpressionValidationTupleSelectionSummary;
  priceSemantics: string;
  datasetProvenance: OptionsExpressionDatasetProvenanceSummary | null;
  diagnostics: OptionsExpressionDiagnosticsSummary;
  overlapWindow: OptionsExpressionOverlapWindowSummary;
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary;
  laneSummaries: OptionsExpressionValidationLaneSummary[];
  acceptanceSummary: OptionsExpressionValidationAcceptanceSummary;
  tupleResults: OptionsExpressionValidationTupleResultSummary[];
  stageCompletions: StageCompletionPacketV1[];
}

export interface ValidateOptionsExpressionArtifactSummary {
  artifactPath: string;
  replayBuildMs: number;
  validationMs: number;
  replayBuildTiming: OptionsExpressionReplayBuildTimingSummary | null;
  result: OptionsExpressionValidationResultSummary;
}

export interface AnalyzeOptionsExpressionValidationFeasibilitySummary {
  replayBuildMs: number;
  replayBuildTiming: OptionsExpressionReplayBuildTimingSummary | null;
  artifactPath: string;
  sourceArtifactHash: string;
  optionsDatasetId: string;
  optionsDatasetHash: string;
  optionsDatasetNormalizedRowsSha256?: string;
  optionsDatasetManifestSemanticSha256?: string;
  datasetPolicy: string;
  replacementDataset: boolean;
  validationPlan: OptionsExpressionValidationPlanSummary;
  tupleSelection: OptionsExpressionValidationTupleSelectionSummary;
  selectedTupleIndexes: number[];
  selectedTupleCount: number;
  priceSemantics: string;
  datasetProvenance: OptionsExpressionDatasetProvenanceSummary | null;
  notes: string[];
}

export interface OptionsExpressionEventInspectionRow {
  eventIndex: number;
  eventId: string;
  split: string;
  side: string;
  decisionTimestamp: number;
  earliestTradableTimestamp: number;
  requestedEntryTimestamp: number;
  rawUnderlierPrice: number;
  baseExitTimestamp: number | null;
  status: string;
  candidateInvalidReason: string;
  contractId: string;
  companionContractId?: string;
  expirationDate: string;
  strike: number | null;
  companionStrike?: number | null;
  multiplier: number | null;
  entryTimestamp: number | null;
  entryOptionPrice: number | null;
  companionEntryOptionPrice?: number | null;
  entryUnderlierPrice: number | null;
  exitTimestamp: number | null;
  exitOptionPrice: number | null;
  companionExitOptionPrice?: number | null;
  premiumPaid: number | null;
  pnlPerContract: number | null;
  returnOnPremium: number | null;
  entryLagMinutes: number | null;
  minutesToExpiryAtEntry: number | null;
  realizedHoldMinutes: number | null;
  pathOptionHigh: number | null;
  pathOptionLow: number | null;
  pathUnderlierHigh: number | null;
  pathUnderlierLow: number | null;
  exitReason: string;
}

export interface OptionsExpressionTupleInspectionSummary {
  tupleIndex: number;
  tupleCount: number;
  tupleResult: OptionsExpressionTupleSummary;
  totalEvents: number;
  executedEvents: number;
  candidateMissingEvents: number;
  outcomeMissingEvents: number;
  rows: OptionsExpressionEventInspectionRow[];
}

export interface OptionsExpressionEventPathPointSummary {
  timestampMs: number;
  optionOpen: number;
  optionHigh: number;
  optionLow: number;
  optionClose: number;
  underlyingPrice: number;
}

export interface OptionsExpressionEventPathInspectionSummary {
  tupleIndex: number;
  eventIndex: number;
  eventId: string;
  row: OptionsExpressionEventInspectionRow;
  pointCount: number;
  points: OptionsExpressionEventPathPointSummary[];
}

export interface OpenOrRevealPathResult {
  path: string;
  openedWith: "open_path" | "reveal_path";
}

export interface OptionsExpressionCsvExportInput {
  filenameStem?: string;
  rootSymbol?: string;
  backendUsed?: string;
  priceSemantics?: string;
  tuples: OptionsExpressionTupleSummary[];
}

export interface OptionsExpressionInspectionCsvExportInput {
  filenameStem?: string;
  artifactPath?: string;
  tupleIndex?: number;
  rows: OptionsExpressionEventInspectionRow[];
}

export interface OptionsExpressionEventPathCsvExportInput {
  filenameStem?: string;
  artifactPath?: string;
  tupleIndex?: number;
  eventId?: string;
  points: OptionsExpressionEventPathPointSummary[];
}

export type OptionsExpressionCsvExportMode =
  | "saved"
  | "downloaded"
  | "cancelled"
  | "unavailable";

export interface OptionsExpressionCsvExportResult {
  mode: OptionsExpressionCsvExportMode;
  filename: string;
  path: string | null;
  bytes: number;
  revealed: boolean;
}

function normalizeOptionsDatasetManifest(
  raw: unknown,
): OptionsDatasetManifestSummary {
  const row = parseBoundaryObject(raw);
  return {
    version: normalizeNumber(row.version),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    rootSymbol: normalizeText(row.root_symbol),
    providerName: normalizeText(row.provider_name),
    providerSourceSurface: normalizeText(row.provider_source_surface),
    providerEntitlementProfile: normalizeText(row.provider_entitlement_profile),
    quoteResolution: normalizeText(row.quote_resolution),
    priceSemantics: normalizeText(row.price_semantics),
    calendarId: normalizeText(row.calendar_id),
    timezone: normalizeText(row.timezone),
    coverageStart: normalizeNumber(row.coverage_start),
    coverageEnd: normalizeNumber(row.coverage_end),
    normalizationVersion: normalizeNumber(row.normalization_version),
    quoteSemanticsVersion: normalizeNumber(row.quote_semantics_version),
    corporateActionMappingVersion: normalizeNumber(
      row.corporate_action_mapping_version,
    ),
    contractReferenceSource: normalizeText(row.contract_reference_source),
    contractReferenceAsOfUtc: normalizeText(row.contract_reference_as_of_utc),
    underlierReferenceSource: normalizeText(row.underlier_reference_source),
    datasetPath: normalizeText(row.dataset_path),
    rowCount: normalizeNumber(row.row_count),
    contractCount: normalizeNumber(row.contract_count),
    aggregatePriceField: normalizeText(row.aggregate_price_field),
    normalizedRowsSha256: normalizeText(row.normalized_rows_sha256),
    manifestSemanticSha256: normalizeText(row.manifest_semantic_sha256),
  };
}

function normalizePolygonOptionsDatasetDownload(
  raw: unknown,
): PolygonOptionsDatasetDownloadSummary {
  const row = parseBoundaryObject(raw);
  return {
    manifest: normalizeOptionsDatasetManifest(row.manifest),
    contractsDiscovered: normalizeNumber(row.contracts_discovered),
    contractsSelected: normalizeNumber(row.contracts_selected),
    contractsWithRows: normalizeNumber(row.contracts_with_rows),
    downloadedOptionRows: normalizeNumber(row.downloaded_option_rows),
    skippedContracts: normalizeNumber(row.skipped_contracts),
    underlyingCsvPath: normalizeText(row.underlying_csv_path),
    usedUnderlyingCacheOnly: Boolean(row.used_underlying_cache_only),
  };
}

function normalizeMetrics(raw: unknown): OptionsExpressionMetricsSummary {
  const row = parseBoundaryObject(raw);
  return {
    validTrades: normalizeNumber(row.valid_trades),
    invalidEvents: normalizeNumber(row.invalid_events),
    invalidRate: normalizeNumber(row.invalid_rate),
    expectancyPerTrade: normalizeNumber(row.expectancy_per_trade),
    expectancyPerPremium: normalizeNumber(row.expectancy_per_premium),
    meanReturnOnPremium: normalizeNumber(row.mean_return_on_premium),
    returnVolatilityOnPremium: normalizeNumber(
      row.return_volatility_on_premium,
    ),
    medianReturnOnPremium: normalizeNumber(row.median_return_on_premium),
    winRate: normalizeNumber(row.win_rate),
    totalPnlPerContract: normalizeNumber(row.total_pnl_per_contract),
  };
}

function normalizeObjectiveProfile(raw: unknown): OptionsObjectiveProfile {
  return normalizeText(raw) || "balanced";
}

function normalizeObjectiveConfig(raw: unknown): OptionsObjectiveConfigSummary {
  const row = parseBoundaryObject(raw);
  return {
    profile: normalizeObjectiveProfile(row.profile),
  };
}

function normalizeObjectiveScoreComponents(
  raw: unknown,
): OptionsObjectiveScoreComponentsSummary | null {
  if (!raw) return null;
  const row = parseBoundaryObject(raw);
  if (Object.keys(row).length === 0) return null;
  return {
    profile: normalizeObjectiveProfile(row.profile),
    searchUtility: normalizeNumber(row.search_utility),
    searchWeight: normalizeNumber(row.search_weight),
    searchContribution: normalizeNumber(row.search_contribution),
    holdoutUtility: normalizeNumber(row.holdout_utility),
    holdoutWeight: normalizeNumber(row.holdout_weight),
    holdoutContribution: normalizeNumber(row.holdout_contribution),
    stressUtility: normalizeNumber(row.stress_utility),
    stressWeight: normalizeNumber(row.stress_weight),
    stressContribution: normalizeNumber(row.stress_contribution),
    harsherStressUtility: normalizeNumber(row.harsher_stress_utility),
    harsherStressWeight: normalizeNumber(row.harsher_stress_weight),
    harsherStressContribution: normalizeNumber(
      row.harsher_stress_contribution,
    ),
    extremeStressUtility: normalizeNumber(row.extreme_stress_utility),
    extremeStressWeight: normalizeNumber(row.extreme_stress_weight),
    extremeStressContribution: normalizeNumber(
      row.extreme_stress_contribution,
    ),
    totalScore: normalizeNumber(row.total_score),
  };
}

function normalizeValidationVerdict(
  raw: unknown,
): OptionsExpressionValidationVerdict | undefined {
  const value = normalizeText(raw);
  switch (value) {
    case "holdout_supported":
    case "sealed_finalist":
    case "stress_fragile":
      return value;
    case "search_only":
      return "search_only";
    default:
      return undefined;
  }
}

function normalizeTuple(raw: unknown): OptionsExpressionTupleSummary {
  const row = parseBoundaryObject(raw);
  const tuple = parseBoundaryObject(row.tuple);
  return {
    structureFamily: normalizeText(tuple.structure_family),
    delayMinutes: normalizeNumber(tuple.delay_minutes),
    dteDays: normalizeNumber(tuple.dte_days),
    moneynessBucket: normalizeText(tuple.moneyness_bucket),
    holdMinutes: normalizeNumber(tuple.hold_minutes),
    targetReturn: normalizeOptionalNumber(tuple.target_return),
    stopReturn: normalizeOptionalNumber(tuple.stop_return),
    exitTemplate: normalizeText(tuple.exit_template),
    fillModel: normalizeText(tuple.fill_model),
    preSealedRank:
      row.pre_sealed_rank == null
        ? undefined
        : normalizeNumber(row.pre_sealed_rank),
    sealedFinalist: Boolean(row.sealed_finalist),
    sealedFinalRank:
      row.sealed_final_rank == null
        ? undefined
        : normalizeNumber(row.sealed_final_rank),
    validationVerdict: normalizeValidationVerdict(row.validation_verdict),
    objectiveScoreComponents: normalizeObjectiveScoreComponents(
      row.objective_score_components,
    ),
    robustScore: normalizeNumber(row.robust_score),
    selectionMetrics: normalizeMetrics(row.selection_metrics),
    holdoutMetrics: row.holdout_metrics
      ? normalizeMetrics(row.holdout_metrics)
      : null,
    sealedEvaluationMetrics: row.sealed_evaluation_metrics
      ? normalizeMetrics(row.sealed_evaluation_metrics)
      : null,
    stressMetrics: row.stress_metrics
      ? normalizeMetrics(row.stress_metrics)
      : null,
    sealedStressMetrics: row.sealed_stress_metrics
      ? normalizeMetrics(row.sealed_stress_metrics)
      : null,
    harsherStressMetrics: row.harsher_stress_metrics
      ? normalizeMetrics(row.harsher_stress_metrics)
      : null,
    sealedHarsherStressMetrics: row.sealed_harsher_stress_metrics
      ? normalizeMetrics(row.sealed_harsher_stress_metrics)
      : null,
    extremeStressMetrics: row.extreme_stress_metrics
      ? normalizeMetrics(row.extreme_stress_metrics)
      : null,
    sealedExtremeStressMetrics: row.sealed_extreme_stress_metrics
      ? normalizeMetrics(row.sealed_extreme_stress_metrics)
      : null,
  };
}

function normalizeContractQuoteQuality(
  raw: unknown,
): OptionsExpressionContractQuoteQualitySummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    contractId: normalizeText(row.contract_id),
    rowCount: normalizeNumber(row.row_count),
    anyQuoteRowCount: normalizeNumber(row.any_quote_row_count),
    bidAskRowCount: normalizeNumber(row.bid_ask_row_count),
    midRowCount: normalizeNumber(row.mid_row_count),
    quoteAgeRowCount: normalizeNumber(row.quote_age_row_count),
    staleQuoteRowCount: normalizeNumber(row.stale_quote_row_count),
    zeroBidRowCount: normalizeNumber(row.zero_bid_row_count),
    lockedBidAskRowCount: normalizeNumber(row.locked_bid_ask_row_count),
    crossedBidAskRowCount: normalizeNumber(row.crossed_bid_ask_row_count),
    invalidBidAskRowCount: normalizeNumber(row.invalid_bid_ask_row_count),
    anyQuoteCoverageRate: normalizeNumber(row.any_quote_coverage_rate),
    bidAskCoverageRate: normalizeNumber(row.bid_ask_coverage_rate),
    midCoverageRate: normalizeNumber(row.mid_coverage_rate),
    staleQuoteRate: normalizeNumber(row.stale_quote_rate),
    meanSpreadBps: normalizeOptionalNumber(row.mean_spread_bps),
    maxSpreadBps: normalizeOptionalNumber(row.max_spread_bps),
    maxQuoteAgeMs: normalizeOptionalNumber(row.max_quote_age_ms),
    warnings: normalizeStringList(row.warnings),
  };
}

function normalizeDatasetQuoteQuality(
  raw: unknown,
): OptionsExpressionDatasetQuoteQualitySummary | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as Record<string, unknown>;
  const perContract = Array.isArray(row.per_contract)
    ? row.per_contract
    : [];
  return {
    priceSemantics: normalizeText(row.price_semantics),
    rowCount: normalizeNumber(row.row_count),
    contractCount: normalizeNumber(row.contract_count),
    anyQuoteRowCount: normalizeNumber(row.any_quote_row_count),
    bidRowCount: normalizeNumber(row.bid_row_count),
    askRowCount: normalizeNumber(row.ask_row_count),
    bidAskRowCount: normalizeNumber(row.bid_ask_row_count),
    midRowCount: normalizeNumber(row.mid_row_count),
    quoteAgeRowCount: normalizeNumber(row.quote_age_row_count),
    staleQuoteThresholdMs: normalizeOptionalNumber(
      row.stale_quote_threshold_ms,
    ),
    staleQuoteRowCount: normalizeNumber(row.stale_quote_row_count),
    zeroBidRowCount: normalizeNumber(row.zero_bid_row_count),
    lockedBidAskRowCount: normalizeNumber(row.locked_bid_ask_row_count),
    crossedBidAskRowCount: normalizeNumber(row.crossed_bid_ask_row_count),
    invalidBidAskRowCount: normalizeNumber(row.invalid_bid_ask_row_count),
    anyQuoteCoverageRate: normalizeNumber(row.any_quote_coverage_rate),
    bidAskCoverageRate: normalizeNumber(row.bid_ask_coverage_rate),
    midCoverageRate: normalizeNumber(row.mid_coverage_rate),
    staleQuoteRate: normalizeNumber(row.stale_quote_rate),
    meanSpreadBps: normalizeOptionalNumber(row.mean_spread_bps),
    medianSpreadBps: normalizeOptionalNumber(row.median_spread_bps),
    maxSpreadBps: normalizeOptionalNumber(row.max_spread_bps),
    maxQuoteAgeMs: normalizeOptionalNumber(row.max_quote_age_ms),
    perContract: perContract.map((entry) =>
      normalizeContractQuoteQuality(entry),
    ),
    warnings: normalizeStringList(row.warnings),
  };
}

function normalizeDatasetProvenance(
  raw: unknown,
): OptionsExpressionDatasetProvenanceSummary | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as Record<string, unknown>;
  return {
    providerName: normalizeText(row.provider_name),
    providerSourceSurface: normalizeText(row.provider_source_surface),
    quoteResolution: normalizeText(row.quote_resolution),
    aggregatePriceField: normalizeText(row.aggregate_price_field),
    contractReferenceAsOfUtc: normalizeText(row.contract_reference_as_of_utc),
    normalizedRowsSha256: normalizeText(row.normalized_rows_sha256),
    manifestSemanticSha256: normalizeText(row.manifest_semantic_sha256),
    quoteQuality: normalizeDatasetQuoteQuality(row.quote_quality),
  };
}

function normalizeValidationTupleSelection(
  raw: unknown,
): OptionsExpressionValidationTupleSelectionSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    includeSealedFinalists: row.include_sealed_finalists !== false,
    includeRobustLeader: row.include_robust_leader !== false,
    includeHoldoutLeader: row.include_holdout_leader !== false,
    tupleIndexes: Array.isArray(row.tuple_indexes)
      ? row.tuple_indexes.map((entry) => normalizeNumber(entry))
      : [],
  };
}

function normalizeValidationPlan(
  raw: unknown,
): OptionsExpressionValidationPlanSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    mode: normalizeText(row.mode) || "original_embedded_splits",
    startTimestampMs: normalizeOptionalNumber(row.start_timestamp_ms),
    endTimestampMs: normalizeOptionalNumber(row.end_timestamp_ms),
    trainSessionCount: normalizeOptionalNumber(row.train_session_count),
    validationSessionCount: normalizeOptionalNumber(row.validation_session_count),
    stepSessionCount: normalizeOptionalNumber(row.step_session_count),
  };
}

function normalizeValidationTupleDrift(
  raw: unknown,
): OptionsExpressionValidationTupleDriftSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    selectionExpectancyDelta: normalizeNumber(row.selection_expectancy_delta),
    holdoutExpectancyDelta: normalizeOptionalNumber(
      row.holdout_expectancy_delta,
    ),
    sealedExpectancyDelta: normalizeOptionalNumber(
      row.sealed_expectancy_delta,
    ),
    stressExpectancyDelta: normalizeOptionalNumber(
      row.stress_expectancy_delta,
    ),
    extremeStressExpectancyDelta: normalizeOptionalNumber(
      row.extreme_stress_expectancy_delta,
    ),
    robustScoreDelta: normalizeNumber(row.robust_score_delta),
  };
}

function normalizeValidationTupleResult(
  raw: unknown,
): OptionsExpressionValidationTupleResultSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    sourceTupleIndex: normalizeNumber(row.source_tuple_index),
    sourceValidationVerdict: normalizeValidationVerdict(
      row.source_validation_verdict,
    ),
    sourceRobustScore: normalizeNumber(row.source_robust_score),
    validatedTuple: normalizeTuple(row.validated_tuple),
    supportDeficient: Boolean(row.support_deficient),
    drift: normalizeValidationTupleDrift(row.drift),
  };
}

function normalizeValidationLaneSummary(
  raw: unknown,
): OptionsExpressionValidationLaneSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    lane: normalizeText(row.lane),
    tupleCount: normalizeNumber(row.tuple_count),
    supportedTupleCount: normalizeNumber(row.supported_tuple_count),
    supportDeficientTupleCount: normalizeNumber(
      row.support_deficient_tuple_count,
    ),
    minValidTrades: normalizeOptionalNumber(row.min_valid_trades),
    bestExpectancyPerPremium: normalizeOptionalNumber(
      row.best_expectancy_per_premium,
    ),
    worstExpectancyPerPremium: normalizeOptionalNumber(
      row.worst_expectancy_per_premium,
    ),
    medianExpectancyPerPremium: normalizeOptionalNumber(
      row.median_expectancy_per_premium,
    ),
  };
}

function normalizeValidationAcceptanceSummary(
  raw: unknown,
): OptionsExpressionValidationAcceptanceSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    verdict: normalizeText(row.verdict) || "support_deficient",
    selectedTupleCount: normalizeNumber(row.selected_tuple_count),
    validatedTupleCount: normalizeNumber(row.validated_tuple_count),
    acceptedTupleCount: normalizeNumber(row.accepted_tuple_count),
    supportDeficientTupleCount: normalizeNumber(
      row.support_deficient_tuple_count,
    ),
    stressFragileTupleCount: normalizeNumber(row.stress_fragile_tuple_count),
    minValidTrades: normalizeOptionalNumber(row.min_valid_trades),
    bestValidatedExpectancyPerPremium: normalizeOptionalNumber(
      row.best_validated_expectancy_per_premium,
    ),
    worstStressExpectancyPerPremium: normalizeOptionalNumber(
      row.worst_stress_expectancy_per_premium,
    ),
    notes: normalizeStringList(row.notes),
  };
}

function normalizeDiagnostics(
  raw: unknown,
): OptionsExpressionDiagnosticsSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    parentArtifactPath: normalizeText(row.parent_artifact_path),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    rootSymbol: normalizeText(row.root_symbol),
    totalEvents: normalizeNumber(row.total_events),
    bullishEvents: normalizeNumber(row.bullish_events),
    bearishEvents: normalizeNumber(row.bearish_events),
    searchEvents: normalizeNumber(row.search_events),
    holdoutEvents: normalizeNumber(row.holdout_events),
    sealedTestEvents: normalizeNumber(row.sealed_test_events),
    datasetContracts: normalizeNumber(row.dataset_contracts),
    datasetRows: normalizeNumber(row.dataset_rows),
    coverageStart: normalizeOptionalNumber(row.coverage_start),
    coverageEnd: normalizeOptionalNumber(row.coverage_end),
  };
}

function normalizeStaging(
  raw: unknown,
): OptionsExpressionStagingSummary | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as Record<string, unknown>;
  return {
    candidateSlotCount: normalizeNumber(row.candidate_slot_count),
    compiledCandidateCount: normalizeNumber(row.compiled_candidate_count),
    baselineExitSummaryCount: normalizeNumber(row.baseline_exit_summary_count),
    conservativeExitSummaryCount: normalizeNumber(
      row.conservative_exit_summary_count,
    ),
    harsherExitSummaryCount: normalizeNumber(row.harsher_exit_summary_count),
  };
}

function normalizeFeasibilityTiming(
  raw: unknown,
): OptionsExpressionFeasibilityTimingSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    tupleCompileMs: normalizeNumber(row.tuple_compile_ms),
    baselineCandidateCompileMs: normalizeNumber(
      row.baseline_candidate_compile_ms,
    ),
    baselineExitSummaryCompileMs: normalizeNumber(
      row.baseline_exit_summary_compile_ms,
    ),
    conservativeCandidateCompileMs: normalizeNumber(
      row.conservative_candidate_compile_ms,
    ),
    conservativeExitSummaryCompileMs: normalizeNumber(
      row.conservative_exit_summary_compile_ms,
    ),
    harsherCandidateCompileMs: normalizeNumber(
      row.harsher_candidate_compile_ms,
    ),
    harsherExitSummaryCompileMs: normalizeNumber(
      row.harsher_exit_summary_compile_ms,
    ),
    cpuRankingMs: normalizeNumber(row.cpu_ranking_ms),
    cpuRankingUs: normalizeNumber(row.cpu_ranking_us),
    gpuRankingMs:
      row.gpu_ranking_ms == null ? null : normalizeNumber(row.gpu_ranking_ms),
    gpuRankingUs:
      row.gpu_ranking_us == null ? null : normalizeNumber(row.gpu_ranking_us),
    gpuTensorScoreMs:
      row.gpu_tensor_score_ms == null
        ? null
        : normalizeNumber(row.gpu_tensor_score_ms),
    gpuTensorScoreUs:
      row.gpu_tensor_score_us == null
        ? null
        : normalizeNumber(row.gpu_tensor_score_us),
    gpuExactifyMs:
      row.gpu_exactify_ms == null ? null : normalizeNumber(row.gpu_exactify_ms),
    gpuExactifyUs:
      row.gpu_exactify_us == null ? null : normalizeNumber(row.gpu_exactify_us),
    gpuSealedStageMs:
      row.gpu_sealed_stage_ms == null
        ? null
        : normalizeNumber(row.gpu_sealed_stage_ms),
    gpuSealedStageUs:
      row.gpu_sealed_stage_us == null
        ? null
        : normalizeNumber(row.gpu_sealed_stage_us),
    gpuSpeedupVsCpu: normalizeOptionalNumber(row.gpu_speedup_vs_cpu),
  };
}

function normalizeFeasibilityEventCoverage(
  raw: unknown,
): OptionsExpressionFeasibilityEventCoverageSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    totalEventsWithCandidates: normalizeNumber(
      row.total_events_with_candidates,
    ),
    searchEventsWithCandidates: normalizeNumber(
      row.search_events_with_candidates,
    ),
    holdoutEventsWithCandidates: normalizeNumber(
      row.holdout_events_with_candidates,
    ),
    sealedTestEventsWithCandidates: normalizeNumber(
      row.sealed_test_events_with_candidates,
    ),
    totalEventsWithBaselineOutcomes: normalizeNumber(
      row.total_events_with_baseline_outcomes,
    ),
    searchEventsWithBaselineOutcomes: normalizeNumber(
      row.search_events_with_baseline_outcomes,
    ),
    holdoutEventsWithBaselineOutcomes: normalizeNumber(
      row.holdout_events_with_baseline_outcomes,
    ),
    sealedTestEventsWithBaselineOutcomes: normalizeNumber(
      row.sealed_test_events_with_baseline_outcomes,
    ),
    totalEventsWithConservativeOutcomes: normalizeNumber(
      row.total_events_with_conservative_outcomes,
    ),
    searchEventsWithConservativeOutcomes: normalizeNumber(
      row.search_events_with_conservative_outcomes,
    ),
    holdoutEventsWithConservativeOutcomes: normalizeNumber(
      row.holdout_events_with_conservative_outcomes,
    ),
    sealedTestEventsWithConservativeOutcomes: normalizeNumber(
      row.sealed_test_events_with_conservative_outcomes,
    ),
    totalEventsWithHarsherOutcomes: normalizeNumber(
      row.total_events_with_harsher_outcomes,
    ),
    searchEventsWithHarsherOutcomes: normalizeNumber(
      row.search_events_with_harsher_outcomes,
    ),
    holdoutEventsWithHarsherOutcomes: normalizeNumber(
      row.holdout_events_with_harsher_outcomes,
    ),
    sealedTestEventsWithHarsherOutcomes: normalizeNumber(
      row.sealed_test_events_with_harsher_outcomes,
    ),
  };
}

function normalizeOverlapWindow(
  raw: unknown,
): OptionsExpressionOverlapWindowSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    contractReferenceAsOfUtc:
      normalizeText(row.contract_reference_as_of_utc) || null,
    totalEventsInDatasetWindow: normalizeNumber(
      row.total_events_in_dataset_window,
    ),
    searchEventsInDatasetWindow: normalizeNumber(
      row.search_events_in_dataset_window,
    ),
    holdoutEventsInDatasetWindow: normalizeNumber(
      row.holdout_events_in_dataset_window,
    ),
    sealedTestEventsInDatasetWindow: normalizeNumber(
      row.sealed_test_events_in_dataset_window,
    ),
    totalEventsBeforeDatasetWindow: normalizeNumber(
      row.total_events_before_dataset_window,
    ),
    totalEventsAfterDatasetWindow: normalizeNumber(
      row.total_events_after_dataset_window,
    ),
    searchEventsBeforeDatasetWindow: normalizeNumber(
      row.search_events_before_dataset_window,
    ),
    searchEventsAfterDatasetWindow: normalizeNumber(
      row.search_events_after_dataset_window,
    ),
    holdoutEventsBeforeDatasetWindow: normalizeNumber(
      row.holdout_events_before_dataset_window,
    ),
    holdoutEventsAfterDatasetWindow: normalizeNumber(
      row.holdout_events_after_dataset_window,
    ),
    sealedTestEventsBeforeDatasetWindow: normalizeNumber(
      row.sealed_test_events_before_dataset_window,
    ),
    sealedTestEventsAfterDatasetWindow: normalizeNumber(
      row.sealed_test_events_after_dataset_window,
    ),
    totalEventsBeforeContractReferenceAsOf: normalizeNumber(
      row.total_events_before_contract_reference_as_of,
    ),
    searchEventsBeforeContractReferenceAsOf: normalizeNumber(
      row.search_events_before_contract_reference_as_of,
    ),
    holdoutEventsBeforeContractReferenceAsOf: normalizeNumber(
      row.holdout_events_before_contract_reference_as_of,
    ),
    sealedTestEventsBeforeContractReferenceAsOf: normalizeNumber(
      row.sealed_test_events_before_contract_reference_as_of,
    ),
    totalOverlapRate: normalizeNumber(row.total_overlap_rate),
    searchOverlapRate: normalizeNumber(row.search_overlap_rate),
    holdoutOverlapRate: normalizeNumber(row.holdout_overlap_rate),
    sealedTestOverlapRate: normalizeNumber(row.sealed_test_overlap_rate),
  };
}

function normalizeSplitInvalidReasons(
  raw: unknown,
): OptionsExpressionSplitInvalidReasonSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    searchInvalidReasons: normalizeInvalidReasonCounts(
      row.search_invalid_reasons,
    ),
    holdoutInvalidReasons: normalizeInvalidReasonCounts(
      row.holdout_invalid_reasons,
    ),
    sealedTestInvalidReasons: normalizeInvalidReasonCounts(
      row.sealed_test_invalid_reasons,
    ),
  };
}

function normalizeCoverageAdequacy(
  raw: unknown,
): OptionsExpressionCoverageAdequacySummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    verdict:
      (normalizeText(
        row.verdict,
      ) as OptionsExpressionCoverageAdequacyVerdict) || "unknown",
    nonPromotable: Boolean(row.non_promotable),
    reasons: normalizeStringList(row.reasons),
    searchCandidateCoverageRate: normalizeNumber(
      row.search_candidate_coverage_rate,
    ),
    holdoutCandidateCoverageRate: normalizeNumber(
      row.holdout_candidate_coverage_rate,
    ),
    sealedTestCandidateCoverageRate: normalizeNumber(
      row.sealed_test_candidate_coverage_rate,
    ),
    searchBaselineOutcomeCoverageRate: normalizeNumber(
      row.search_baseline_outcome_coverage_rate,
    ),
    holdoutBaselineOutcomeCoverageRate: normalizeNumber(
      row.holdout_baseline_outcome_coverage_rate,
    ),
    sealedTestBaselineOutcomeCoverageRate: normalizeNumber(
      row.sealed_test_baseline_outcome_coverage_rate,
    ),
    dominantInvalidReason: normalizeText(row.dominant_invalid_reason),
    dominantInvalidReasonShare: normalizeNumber(
      row.dominant_invalid_reason_share,
    ),
  };
}

function normalizeFeasibilityCacheStats(
  raw: unknown,
): OptionsExpressionFeasibilityCacheStatsSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    structuralCacheHits: normalizeNumber(row.structural_cache_hits),
    structuralCacheMisses: normalizeNumber(row.structural_cache_misses),
    structuralCacheEntries: normalizeNumber(row.structural_cache_entries),
    summaryCacheHits: normalizeNumber(row.summary_cache_hits),
    summaryCacheMisses: normalizeNumber(row.summary_cache_misses),
    summaryCacheEntries: normalizeNumber(row.summary_cache_entries),
    gpuSharedInputCacheHits: normalizeNumber(row.gpu_shared_input_cache_hits),
    gpuSharedInputCacheMisses: normalizeNumber(
      row.gpu_shared_input_cache_misses,
    ),
    gpuOutcomeInputCacheHits: normalizeNumber(row.gpu_outcome_input_cache_hits),
    gpuOutcomeInputCacheMisses: normalizeNumber(
      row.gpu_outcome_input_cache_misses,
    ),
    gpuOutputBufferReuses: normalizeNumber(row.gpu_output_buffer_reuses),
    gpuOutputBufferAllocations: normalizeNumber(
      row.gpu_output_buffer_allocations,
    ),
    gpuTupleChunkCount: normalizeNumber(row.gpu_tuple_chunk_count),
    gpuTupleChunkSize: normalizeNumber(row.gpu_tuple_chunk_size),
    gpuKernelLaunchCount: normalizeNumber(row.gpu_kernel_launch_count),
    gpuTupleBudgetBytes: normalizeNumber(row.gpu_tuple_budget_bytes),
    gpuOutcomeTensorBytes: normalizeNumber(row.gpu_outcome_tensor_bytes),
    gpuSharedInputBytesPerChunk: normalizeNumber(
      row.gpu_shared_input_bytes_per_chunk,
    ),
    gpuOutputBytesPerChunk: normalizeNumber(row.gpu_output_bytes_per_chunk),
    gpuTotalDeviceBytesPerChunk: normalizeNumber(
      row.gpu_total_device_bytes_per_chunk,
    ),
    gpuDeviceTotalMemoryBytes: normalizeNumber(
      row.gpu_device_total_memory_bytes,
    ),
    gpuDeviceFreeMemoryBytes: normalizeNumber(row.gpu_device_free_memory_bytes),
    gpuDeviceUsableMemoryBytes: normalizeNumber(
      row.gpu_device_usable_memory_bytes,
    ),
    gpuMemoryBudgetSource: normalizeText(row.gpu_memory_budget_source),
    gpuOutcomeTensorFitsInMemory: !!row.gpu_outcome_tensor_fits_in_memory,
    gpuChunkPlanFitsInMemory: !!row.gpu_chunk_plan_fits_in_memory,
    gpuPlanNotes: normalizeStringList(row.gpu_plan_notes),
  };
}

function normalizeInvalidReasonCounts(
  value: unknown,
): OptionsCandidateInvalidReasonCountSummary[] {
  return parseBoundaryArray(value).map((entry) => {
    const row = parseBoundaryObject(entry);
    return {
      reason: normalizeText(row.reason),
      count: normalizeNumber(row.count),
    };
  });
}

function normalizeDelayBucketCoverage(
  value: unknown,
): OptionsDelayBucketCoverageSummary[] {
  return parseBoundaryArray(value).map((entry) => {
    const row = parseBoundaryObject(entry);
    return {
      delayMinutes: normalizeNumber(row.delay_minutes),
      slotCount: normalizeNumber(row.slot_count),
      validSlotCount: normalizeNumber(row.valid_slot_count),
      invalidSlotCount: normalizeNumber(row.invalid_slot_count),
      validRate: normalizeNumber(row.valid_rate),
      invalidReasons: normalizeInvalidReasonCounts(row.invalid_reasons),
    };
  });
}

function normalizeDteBucketCoverage(
  value: unknown,
): OptionsDteBucketCoverageSummary[] {
  return parseBoundaryArray(value).map((entry) => {
    const row = parseBoundaryObject(entry);
    return {
      dteDays: normalizeNumber(row.dte_days),
      slotCount: normalizeNumber(row.slot_count),
      validSlotCount: normalizeNumber(row.valid_slot_count),
      invalidSlotCount: normalizeNumber(row.invalid_slot_count),
      validRate: normalizeNumber(row.valid_rate),
      invalidReasons: normalizeInvalidReasonCounts(row.invalid_reasons),
    };
  });
}

function normalizeMoneynessBucketCoverage(
  value: unknown,
): OptionsMoneynessBucketCoverageSummary[] {
  return parseBoundaryArray(value).map((entry) => {
    const row = parseBoundaryObject(entry);
    return {
      moneynessBucket: normalizeText(row.moneyness_bucket),
      slotCount: normalizeNumber(row.slot_count),
      validSlotCount: normalizeNumber(row.valid_slot_count),
      invalidSlotCount: normalizeNumber(row.invalid_slot_count),
      validRate: normalizeNumber(row.valid_rate),
      invalidReasons: normalizeInvalidReasonCounts(row.invalid_reasons),
    };
  });
}

function normalizeAxisCoverage(
  raw: unknown,
): OptionsCandidateAxisCoverageSummary {
  const row = parseBoundaryObject(raw);
  return {
    delayMinutes: normalizeDelayBucketCoverage(row.delay_minutes),
    dteDays: normalizeDteBucketCoverage(row.dte_days),
    moneynessBuckets: normalizeMoneynessBucketCoverage(row.moneyness_buckets),
  };
}

function normalizeAutoBackendDecision(
  raw: unknown,
): OptionsExpressionAutoBackendDecisionSummary {
  const row = parseBoundaryObject(raw);
  return {
    recommendedBackend: normalizeText(row.recommended_backend),
    gpuEligible: Boolean(row.gpu_eligible),
    reasons: normalizeStringList(row.reasons),
  };
}

function normalizeReplayBuildTiming(
  raw: unknown,
): OptionsExpressionReplayBuildTimingSummary | null {
  const row = parseBoundaryObject(raw);
  if (Object.keys(row).length === 0) return null;
  return {
    totalUs: normalizeNumber(row.total_us),
    parentArtifactValidationUs: normalizeNumber(
      row.parent_artifact_validation_us,
    ),
    sourceResolutionUs: normalizeNumber(row.source_resolution_us),
    parentSignalProcessCacheLookupUs: normalizeNumber(
      row.parent_signal_process_cache_lookup_us,
    ),
    parentSignalProcessCacheHit: Boolean(row.parent_signal_process_cache_hit),
    persistedSignalManifestCacheLookupUs: normalizeNumber(
      row.persisted_signal_manifest_cache_lookup_us,
    ),
    persistedSignalManifestCacheHit: Boolean(
      row.persisted_signal_manifest_cache_hit,
    ),
    candleReplayLoadUs: normalizeNumber(row.candle_replay_load_us),
    parentReplayAndCandidateDetailUs: normalizeNumber(
      row.parent_replay_and_drilldown_us,
    ),
    signalManifestMaterializationUs: normalizeNumber(
      row.signal_manifest_materialization_us,
    ),
    optionsDatasetManifestValidationUs: normalizeNumber(
      row.options_dataset_manifest_validation_us,
    ),
    optionsDatasetProcessCacheLookupUs: normalizeNumber(
      row.options_dataset_process_cache_lookup_us,
    ),
    optionsDatasetProcessCacheHit: Boolean(
      row.options_dataset_process_cache_hit,
    ),
    optionsDatasetPersistedCacheLookupUs: normalizeNumber(
      row.options_dataset_persisted_cache_lookup_us,
    ),
    optionsDatasetPersistedCacheHit: Boolean(
      row.options_dataset_persisted_cache_hit,
    ),
    optionsDatasetCsvLoadUs: normalizeNumber(row.options_dataset_csv_load_us),
    requestNormalizationUs: normalizeNumber(row.request_normalization_us),
  };
}

function normalizeFeasibilityResponse(
  raw: unknown,
): AnalyzeOptionsExpressionFeasibilitySummary {
  const row = parseBoundaryObject(raw);
  const report = parseBoundaryObject(row.report);
  return {
    replayBuildMs: normalizeNumber(row.replay_build_ms),
    totalPreviewMs: normalizeNumber(row.total_preview_ms),
    replayBuildTiming: normalizeReplayBuildTiming(row.replay_build_timing),
    exactnessSemantics: normalizeExactnessSemantics(
      row.exactness_semantics ??
        row.exactnessSemantics ??
        report.exactness_semantics ??
        report.exactnessSemantics,
    ),
    objectiveConfig: normalizeObjectiveConfig(report.objective_config),
    diagnostics: normalizeDiagnostics(report.diagnostics),
    priceSemantics: normalizeText(report.price_semantics),
    fillModel: normalizeText(report.fill_model),
    bestAvailableBackend: normalizeText(report.best_available_backend),
    autoBackendDecision: normalizeAutoBackendDecision(
      report.auto_backend_decision,
    ),
    compiledTupleCount: normalizeNumber(report.compiled_tuple_count),
    perEventCandidateSlots: normalizeNumber(report.per_event_candidate_slots),
    perCandidateExitSummaries: normalizeNumber(
      report.per_candidate_exit_summaries,
    ),
    candidateSlotCount: normalizeNumber(report.candidate_slot_count),
    compiledCandidateCount: normalizeNumber(report.compiled_candidate_count),
    candidateDensity: normalizeNumber(report.candidate_density),
    invalidSlotCount: normalizeNumber(report.invalid_slot_count),
    invalidSlotRate: normalizeNumber(report.invalid_slot_rate),
    baselineExitSummaryCount: normalizeNumber(
      report.baseline_exit_summary_count,
    ),
    baselineCompiledExitSummaryCount: normalizeNumber(
      report.baseline_compiled_exit_summary_count,
    ),
    baselineExitSummaryDensity: normalizeNumber(
      report.baseline_exit_summary_density,
    ),
    conservativeExitSummaryCount: normalizeNumber(
      report.conservative_exit_summary_count,
    ),
    conservativeCompiledExitSummaryCount: normalizeNumber(
      report.conservative_compiled_exit_summary_count,
    ),
    conservativeExitSummaryDensity: normalizeNumber(
      report.conservative_exit_summary_density,
    ),
    harsherExitSummaryCount: normalizeNumber(report.harsher_exit_summary_count),
    harsherCompiledExitSummaryCount: normalizeNumber(
      report.harsher_compiled_exit_summary_count,
    ),
    harsherExitSummaryDensity: normalizeNumber(
      report.harsher_exit_summary_density,
    ),
    projectedEventTupleEvaluations: normalizeNumber(
      report.projected_event_tuple_evaluations,
    ),
    structuralCacheBytesEstimate: normalizeNumber(
      report.structural_cache_bytes_estimate,
    ),
    baselineSummaryBytesEstimate: normalizeNumber(
      report.baseline_summary_bytes_estimate,
    ),
    conservativeSummaryBytesEstimate: normalizeNumber(
      report.conservative_summary_bytes_estimate,
    ),
    harsherSummaryBytesEstimate: normalizeNumber(
      report.harsher_summary_bytes_estimate,
    ),
    timings: normalizeFeasibilityTiming(report.timings),
    eventCoverage: normalizeFeasibilityEventCoverage(report.event_coverage),
    overlapWindow: normalizeOverlapWindow(report.overlap_window),
    splitInvalidReasons: normalizeSplitInvalidReasons(
      report.split_invalid_reasons,
    ),
    coverageAdequacy: normalizeCoverageAdequacy(report.coverage_adequacy),
    cacheStats: normalizeFeasibilityCacheStats(report.cache_stats),
    invalidReasons: normalizeInvalidReasonCounts(report.invalid_reasons),
    axisCoverage: normalizeAxisCoverage(report.axis_coverage),
  };
}

function normalizeRunOptionsExpression(
  raw: unknown,
): RunOptionsExpressionSummary {
  const row = parseBoundaryObject(raw);
  const result = parseBoundaryObject(row.result);
  const artifact = parseBoundaryObject(row.artifact);
  const artifactRequest = parseBoundaryObject(artifact.request);
  const ranked = parseBoundaryArray(result.ranked_results);
  return {
    artifactPath: normalizeText(row.artifact_path),
    artifactTs: normalizeText(artifact.ts),
    parentArtifactId: normalizeText(artifact.parent_artifact_id),
    optionsDatasetHash: normalizeText(artifact.options_dataset_hash),
    optionsDatasetNormalizedRowsSha256: normalizeText(
      artifact.options_dataset_normalized_rows_sha256,
    ),
    optionsDatasetManifestSemanticSha256: normalizeText(
      artifact.options_dataset_manifest_semantic_sha256,
    ),
    datasetProvenance: normalizeDatasetProvenance(artifact.dataset_provenance),
    exactnessSemantics: normalizeExactnessSemantics(
      row.exactness_semantics ??
        row.exactnessSemantics ??
        result.exactness_semantics ??
        result.exactnessSemantics ??
        artifact.exactness_semantics ??
        artifact.exactnessSemantics,
    ),
    objectiveConfig: normalizeObjectiveConfig(
      result.objective_config ?? artifactRequest.objective_config,
    ),
    backendUsed: normalizeText(result.backend_used),
    backendFallbackReason: normalizeText(result.backend_fallback_reason),
    priceSemantics: normalizeText(result.price_semantics),
    tupleCount: normalizeNumber(result.tuple_count),
    request:
      Object.keys(artifact).length > 0
        ? normalizeRequest(artifactRequest)
        : null,
    diagnostics: normalizeDiagnostics(result.diagnostics),
    staging: normalizeStaging(result.staging),
    overlapWindow: normalizeOverlapWindow(
      result.overlap_window || artifact.overlap_window,
    ),
    coverageAdequacy: normalizeCoverageAdequacy(
      result.coverage_adequacy || artifact.coverage_adequacy,
    ),
    splitInvalidReasons: normalizeSplitInvalidReasons(
      result.split_invalid_reasons || artifact.split_invalid_reasons,
    ),
    rankedResults: ranked.map((entry) => normalizeTuple(entry)),
  };
}

function normalizeIntegerList(value: unknown): number[] {
  return parseBoundaryArray(value)
    .map((entry) => Math.floor(normalizeNumber(entry)))
    .filter((entry) => Number.isFinite(entry));
}

function normalizeFloatList(value: unknown): number[] {
  return parseBoundaryArray(value)
    .map((entry) => normalizeNumber(entry))
    .filter((entry) => Number.isFinite(entry));
}

function normalizeFilters(raw: unknown): OptionsStaticFilterBundleSummary {
  const row = parseBoundaryObject(raw);
  return {
    maxEntryLagMinutes: normalizeOptionalNumber(row.max_entry_lag_minutes),
    maxPremium: normalizeOptionalNumber(row.max_premium),
    minPremium: normalizeOptionalNumber(row.min_premium),
    minVolume: normalizeOptionalNumber(row.min_volume),
    minOpenInterest: normalizeOptionalNumber(row.min_open_interest),
  };
}

function normalizeRequest(raw: unknown): OptionsExpressionRequestSummary {
  const row = parseBoundaryObject(raw);
  return {
    parentArtifactPath: normalizeText(row.parent_artifact_path),
    parentCandidateIndex: normalizeOptionalNumber(row.parent_candidate_index),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    structureFamily: normalizeText(row.structure_family),
    backendPreference: normalizeText(row.backend_preference),
    objectiveConfig: normalizeObjectiveConfig(row.objective_config),
    splitAssignmentMode:
      normalizeText(row.split_assignment_mode) || "event_chronological",
    delayMinutes: normalizeIntegerList(row.delay_minutes),
    dteBuckets: normalizeIntegerList(row.dte_buckets),
    moneynessBuckets: normalizeStringList(row.moneyness_buckets),
    holdMinutes: normalizeIntegerList(row.hold_minutes),
    targetReturnBuckets: normalizeFloatList(row.target_return_buckets),
    stopReturnBuckets: normalizeFloatList(row.stop_return_buckets),
    exitTemplate: normalizeText(row.exit_template),
    fillModel: normalizeText(row.fill_model),
    filters: normalizeFilters(row.filters),
    topK: normalizeNumber(row.top_k),
    minValidTrades: normalizeOptionalNumber(row.min_valid_trades),
    splitRatioSearch: normalizeNumber(row.split_ratio_search),
    splitRatioHoldout: normalizeNumber(row.split_ratio_holdout),
    sealedFinalistCount: normalizeNumber(row.sealed_finalist_count, 5),
  };
}

function normalizeOptionsExpressionArtifactSummary(
  raw: unknown,
): OptionsExpressionArtifactSummary {
  const row = parseBoundaryObject(raw);
  const leaderSummaryRaw = parseBoundaryObject(row.leader_summary);
  return {
    ts: normalizeText(row.ts),
    path: normalizeText(row.path),
    bytes: normalizeNumber(row.bytes),
    parentArtifactId: normalizeText(row.parent_artifact_id),
    parentStrategyKind: normalizeText(row.parent_strategy_kind),
    rootSymbol: normalizeText(row.root_symbol),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    exactnessSemantics: normalizeExactnessSemantics(
      row.exactness_semantics ?? row.exactnessSemantics,
    ),
    backendUsed: normalizeText(row.backend_used),
    backendFallbackReason: normalizeText(row.backend_fallback_reason),
    priceSemantics: normalizeText(row.price_semantics),
    requestBackendPreference: normalizeText(row.request_backend_preference),
    requestObjectiveProfile:
      normalizeText(row.request_objective_profile) || "balanced",
    requestFillModel: normalizeText(row.request_fill_model),
    requestStructureFamily: normalizeText(row.request_structure_family),
    requestSplitAssignmentMode: normalizeText(
      row.request_split_assignment_mode,
    ),
    coverageAdequacyVerdict:
      (normalizeText(
        row.coverage_adequacy_verdict,
      ) as OptionsExpressionCoverageAdequacyVerdict) || "unknown",
    coverageNonPromotable: Boolean(row.coverage_non_promotable),
    searchOverlapRate: normalizeNumber(row.search_overlap_rate),
    holdoutOverlapRate: normalizeNumber(row.holdout_overlap_rate),
    sealedTestOverlapRate: normalizeNumber(row.sealed_test_overlap_rate),
    searchCandidateCoverageRate: normalizeNumber(
      row.search_candidate_coverage_rate,
    ),
    holdoutCandidateCoverageRate: normalizeNumber(
      row.holdout_candidate_coverage_rate,
    ),
    sealedTestCandidateCoverageRate: normalizeNumber(
      row.sealed_test_candidate_coverage_rate,
    ),
    searchBaselineOutcomeCoverageRate: normalizeNumber(
      row.search_baseline_outcome_coverage_rate,
    ),
    holdoutBaselineOutcomeCoverageRate: normalizeNumber(
      row.holdout_baseline_outcome_coverage_rate,
    ),
    sealedTestBaselineOutcomeCoverageRate: normalizeNumber(
      row.sealed_test_baseline_outcome_coverage_rate,
    ),
    tupleCount: normalizeNumber(row.tuple_count),
    bestRobustScore: normalizeNumber(row.best_robust_score),
    bestSelectionExpectancyPerPremium: normalizeNumber(
      row.best_selection_expectancy_per_premium,
    ),
    bestHoldoutExpectancyPerPremium: normalizeOptionalNumber(
      row.best_holdout_expectancy_per_premium,
    ),
    bestSealedExpectancyPerPremium: normalizeOptionalNumber(
      row.best_sealed_expectancy_per_premium,
    ),
    bestStressExpectancyPerPremium: normalizeOptionalNumber(
      row.best_stress_expectancy_per_premium,
    ),
    bestSealedStressExpectancyPerPremium: normalizeOptionalNumber(
      row.best_sealed_stress_expectancy_per_premium,
    ),
    bestHarsherStressExpectancyPerPremium: normalizeOptionalNumber(
      row.best_harsher_stress_expectancy_per_premium,
    ),
    bestSealedHarsherStressExpectancyPerPremium: normalizeOptionalNumber(
      row.best_sealed_harsher_stress_expectancy_per_premium,
    ),
    bestExtremeStressExpectancyPerPremium: normalizeOptionalNumber(
      row.best_extreme_stress_expectancy_per_premium,
    ),
    bestSealedExtremeStressExpectancyPerPremium: normalizeOptionalNumber(
      row.best_sealed_extreme_stress_expectancy_per_premium,
    ),
    leaderSummary: {
      searchLeaderTupleIndex: normalizeOptionalNumber(
        leaderSummaryRaw.search_leader_tuple_index,
      ),
      robustLeaderTupleIndex: normalizeOptionalNumber(
        leaderSummaryRaw.robust_leader_tuple_index,
      ),
      holdoutLeaderTupleIndex: normalizeOptionalNumber(
        leaderSummaryRaw.holdout_leader_tuple_index,
      ),
      sealedLeaderTupleIndex: normalizeOptionalNumber(
        leaderSummaryRaw.sealed_leader_tuple_index,
      ),
      stressLeaderTupleIndex: normalizeOptionalNumber(
        leaderSummaryRaw.stress_leader_tuple_index,
      ),
      distinctLeaderCount: normalizeNumber(
        leaderSummaryRaw.distinct_leader_count,
      ),
      searchVsRobustDisagree: Boolean(
        leaderSummaryRaw.search_vs_robust_disagree,
      ),
      robustVsHoldoutDisagree: normalizeOptionalBoolean(
        leaderSummaryRaw.robust_vs_holdout_disagree,
      ),
      robustVsSealedDisagree: normalizeOptionalBoolean(
        leaderSummaryRaw.robust_vs_sealed_disagree,
      ),
      robustLeaderReachedSealedFinalist: normalizeOptionalBoolean(
        leaderSummaryRaw.robust_leader_reached_sealed_finalist,
      ),
      robustLeaderSealedFinalRank: normalizeOptionalNumber(
        leaderSummaryRaw.robust_leader_sealed_final_rank,
      ),
    },
    topValidationVerdict: normalizeValidationVerdict(
      row.top_validation_verdict,
    ),
    sealedFinalistBudget: normalizeNumber(row.sealed_finalist_budget),
    sealedEvaluatedTupleCount: normalizeNumber(
      row.sealed_evaluated_tuple_count,
    ),
    validatedTupleCount: normalizeNumber(row.validated_tuple_count),
    stressFragileTupleCount: normalizeNumber(row.stress_fragile_tuple_count),
    supportedHoldoutTupleCount: normalizeNumber(
      row.supported_holdout_tuple_count,
    ),
    supportedSealedTupleCount: normalizeNumber(
      row.supported_sealed_tuple_count,
    ),
    supportedStressTupleCount: normalizeNumber(
      row.supported_stress_tuple_count,
    ),
    hasHoldoutMetrics: Boolean(row.has_holdout_metrics),
    hasSealedEvaluationMetrics: Boolean(row.has_sealed_evaluation_metrics),
    hasStressMetrics: Boolean(row.has_stress_metrics),
    hasHarsherStressMetrics: Boolean(row.has_harsher_stress_metrics),
    hasExtremeStressMetrics: Boolean(row.has_extreme_stress_metrics),
  };
}

function normalizeOptionsExpressionArtifact(
  raw: unknown,
): OptionsExpressionArtifactRecord {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  const metrics = Array.isArray(row.selection_metrics)
    ? row.selection_metrics
    : [];
  return {
    version: normalizeNumber(row.version),
    kind: normalizeText(row.kind),
    ts: normalizeText(row.ts),
    artifactPath: normalizeText(row.artifact_path),
    parentArtifactId: normalizeText(row.parent_artifact_id),
    rootSymbol: normalizeText(row.root_symbol),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    optionsDatasetHash: normalizeText(row.options_dataset_hash),
    optionsDatasetNormalizedRowsSha256: normalizeText(
      row.options_dataset_normalized_rows_sha256,
    ),
    optionsDatasetManifestSemanticSha256: normalizeText(
      row.options_dataset_manifest_semantic_sha256,
    ),
    datasetProvenance: normalizeDatasetProvenance(row.dataset_provenance),
    exactnessSemantics: normalizeExactnessSemantics(
      row.exactness_semantics ?? row.exactnessSemantics,
    ),
    backendUsed: normalizeText(row.backend_used),
    backendFallbackReason: normalizeText(row.backend_fallback_reason),
    priceSemantics: normalizeText(row.price_semantics),
    request: normalizeRequest(row.request),
    diagnostics: normalizeDiagnostics(row.diagnostics),
    staging: normalizeStaging(row.staging),
    overlapWindow: normalizeOverlapWindow(row.overlap_window),
    coverageAdequacy: normalizeCoverageAdequacy(row.coverage_adequacy),
    splitInvalidReasons: normalizeSplitInvalidReasons(
      row.split_invalid_reasons,
    ),
    selectionMetrics: metrics.map((entry) => normalizeTuple(entry)),
  };
}

function normalizeOptionsExpressionValidationResult(
  raw: unknown,
): OptionsExpressionValidationResultSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  const stageCompletions = normalizeStageCompletions(row.stage_completions);
  const hasOptionsValidationPacket = stageCompletions.some(
    (packet) => packet.stage === "options_expression_validation",
  );
  if (!hasOptionsValidationPacket) {
    stageCompletions.push(
      buildOptionsExpressionValidationStageCompletion({
        ts: row.ts,
        artifactKind: row.kind || "vectorgrid_options_expression_validation",
        artifactPath: row.validation_artifact_path,
        configFingerprint: row.source_artifact_hash,
        res: row,
        backfilled: true,
      }),
    );
  }
  const laneSummaries = Array.isArray(row.lane_summaries)
    ? row.lane_summaries
    : [];
  const tupleResults = Array.isArray(row.tuple_results)
    ? row.tuple_results
    : [];
  return {
    version: normalizeNumber(row.version),
    kind: normalizeText(row.kind),
    ts: normalizeText(row.ts),
    validationArtifactPath: normalizeText(row.validation_artifact_path),
    sourceArtifactPath: normalizeText(row.source_artifact_path),
    sourceArtifactHash: normalizeText(row.source_artifact_hash),
    parentArtifactId: normalizeText(row.parent_artifact_id),
    parentArtifactHash: normalizeText(row.parent_artifact_hash),
    signalManifestId: normalizeText(row.signal_manifest_id),
    rootSymbol: normalizeText(row.root_symbol),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    optionsDatasetHash: normalizeText(row.options_dataset_hash),
    optionsDatasetNormalizedRowsSha256: normalizeText(
      row.options_dataset_normalized_rows_sha256,
    ),
    optionsDatasetManifestSemanticSha256: normalizeText(
      row.options_dataset_manifest_semantic_sha256,
    ),
    datasetPolicy: normalizeText(row.dataset_policy),
    replacementDataset: Boolean(row.replacement_dataset),
    validationPlan: normalizeValidationPlan(row.validation_plan),
    tupleSelection: normalizeValidationTupleSelection(row.tuple_selection),
    priceSemantics: normalizeText(row.price_semantics),
    datasetProvenance: normalizeDatasetProvenance(row.dataset_provenance),
    diagnostics: normalizeDiagnostics(row.diagnostics),
    overlapWindow: normalizeOverlapWindow(row.overlap_window),
    coverageAdequacy: normalizeCoverageAdequacy(row.coverage_adequacy),
    laneSummaries: laneSummaries.map((entry) =>
      normalizeValidationLaneSummary(entry),
    ),
    acceptanceSummary: normalizeValidationAcceptanceSummary(
      row.acceptance_summary,
    ),
    tupleResults: tupleResults.map((entry) =>
      normalizeValidationTupleResult(entry),
    ),
    stageCompletions,
  };
}

function normalizeValidateOptionsExpressionArtifact(
  raw: unknown,
): ValidateOptionsExpressionArtifactSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    artifactPath: normalizeText(row.artifact_path),
    replayBuildMs: normalizeNumber(row.replay_build_ms),
    validationMs: normalizeNumber(row.validation_ms),
    replayBuildTiming: row.replay_build_timing
      ? normalizeReplayBuildTiming(row.replay_build_timing)
      : null,
    result: normalizeOptionsExpressionValidationResult(row.result),
  };
}

function normalizeValidationFeasibilityResponse(
  raw: unknown,
): AnalyzeOptionsExpressionValidationFeasibilitySummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    replayBuildMs: normalizeNumber(row.replay_build_ms),
    replayBuildTiming: row.replay_build_timing
      ? normalizeReplayBuildTiming(row.replay_build_timing)
      : null,
    artifactPath: normalizeText(row.artifact_path),
    sourceArtifactHash: normalizeText(row.source_artifact_hash),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    optionsDatasetHash: normalizeText(row.options_dataset_hash),
    optionsDatasetNormalizedRowsSha256: normalizeText(
      row.options_dataset_normalized_rows_sha256,
    ),
    optionsDatasetManifestSemanticSha256: normalizeText(
      row.options_dataset_manifest_semantic_sha256,
    ),
    datasetPolicy: normalizeText(row.dataset_policy),
    replacementDataset: Boolean(row.replacement_dataset),
    validationPlan: normalizeValidationPlan(row.validation_plan),
    tupleSelection: normalizeValidationTupleSelection(row.tuple_selection),
    selectedTupleIndexes: Array.isArray(row.selected_tuple_indexes)
      ? row.selected_tuple_indexes.map((entry) => normalizeNumber(entry))
      : [],
    selectedTupleCount: normalizeNumber(row.selected_tuple_count),
    priceSemantics: normalizeText(row.price_semantics),
    datasetProvenance: normalizeDatasetProvenance(row.dataset_provenance),
    notes: normalizeStringList(row.notes),
  };
}

function normalizeInspectionRow(
  raw: unknown,
): OptionsExpressionEventInspectionRow {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    eventIndex: normalizeNumber(row.event_index),
    eventId: normalizeText(row.event_id),
    split: normalizeText(row.split),
    side: normalizeText(row.side),
    decisionTimestamp: normalizeNumber(row.decision_timestamp),
    earliestTradableTimestamp: normalizeNumber(row.earliest_tradable_timestamp),
    requestedEntryTimestamp: normalizeNumber(row.requested_entry_timestamp),
    rawUnderlierPrice: normalizeNumber(row.raw_underlier_price),
    baseExitTimestamp: normalizeOptionalNumber(row.base_exit_timestamp),
    status: normalizeText(row.status),
    candidateInvalidReason: normalizeText(row.candidate_invalid_reason),
    contractId: normalizeText(row.contract_id),
    companionContractId: normalizeText(row.companion_contract_id),
    expirationDate: normalizeText(row.expiration_date),
    strike: normalizeOptionalNumber(row.strike),
    companionStrike: normalizeOptionalNumber(row.companion_strike),
    multiplier: normalizeOptionalNumber(row.multiplier),
    entryTimestamp: normalizeOptionalNumber(row.entry_timestamp),
    entryOptionPrice: normalizeOptionalNumber(row.entry_option_price),
    companionEntryOptionPrice: normalizeOptionalNumber(
      row.companion_entry_option_price,
    ),
    entryUnderlierPrice: normalizeOptionalNumber(row.entry_underlier_price),
    exitTimestamp: normalizeOptionalNumber(row.exit_timestamp),
    exitOptionPrice: normalizeOptionalNumber(row.exit_option_price),
    companionExitOptionPrice: normalizeOptionalNumber(
      row.companion_exit_option_price,
    ),
    premiumPaid: normalizeOptionalNumber(row.premium_paid),
    pnlPerContract: normalizeOptionalNumber(row.pnl_per_contract),
    returnOnPremium: normalizeOptionalNumber(row.return_on_premium),
    entryLagMinutes: normalizeOptionalNumber(row.entry_lag_minutes),
    minutesToExpiryAtEntry: normalizeOptionalNumber(
      row.minutes_to_expiry_at_entry,
    ),
    realizedHoldMinutes: normalizeOptionalNumber(row.realized_hold_minutes),
    pathOptionHigh: normalizeOptionalNumber(row.path_option_high),
    pathOptionLow: normalizeOptionalNumber(row.path_option_low),
    pathUnderlierHigh: normalizeOptionalNumber(row.path_underlier_high),
    pathUnderlierLow: normalizeOptionalNumber(row.path_underlier_low),
    exitReason: normalizeText(row.exit_reason),
  };
}

function normalizeTupleInspection(
  raw: unknown,
): OptionsExpressionTupleInspectionSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  const rows = Array.isArray(row.rows) ? row.rows : [];
  return {
    tupleIndex: normalizeNumber(row.tuple_index),
    tupleCount: normalizeNumber(row.tuple_count),
    tupleResult: normalizeTuple(row.tuple_result),
    totalEvents: normalizeNumber(row.total_events),
    executedEvents: normalizeNumber(row.executed_events),
    candidateMissingEvents: normalizeNumber(row.candidate_missing_events),
    outcomeMissingEvents: normalizeNumber(row.outcome_missing_events),
    rows: rows.map((entry) => normalizeInspectionRow(entry)),
  };
}

function normalizeEventPathPoint(
  raw: unknown,
): OptionsExpressionEventPathPointSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  return {
    timestampMs: normalizeNumber(row.timestamp_ms),
    optionOpen: normalizeNumber(row.option_open),
    optionHigh: normalizeNumber(row.option_high),
    optionLow: normalizeNumber(row.option_low),
    optionClose: normalizeNumber(row.option_close),
    underlyingPrice: normalizeNumber(row.underlying_price),
  };
}

function normalizeEventPathInspection(
  raw: unknown,
): OptionsExpressionEventPathInspectionSummary {
  const row = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  const points = Array.isArray(row.points) ? row.points : [];
  return {
    tupleIndex: normalizeNumber(row.tuple_index),
    eventIndex: normalizeNumber(row.event_index),
    eventId: normalizeText(row.event_id),
    row: normalizeInspectionRow(row.row),
    pointCount: normalizeNumber(row.point_count),
    points: points.map((entry) => normalizeEventPathPoint(entry)),
  };
}

function csvCell(value: unknown): string {
  const text = String(value ?? "");
  if (/[",\r\n]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }
  return text;
}

function downloadBlob(filename: string, blob: Blob): boolean {
  if (typeof document === "undefined") return false;
  if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function")
    return false;
  const href = URL.createObjectURL(blob);
  try {
    const link = document.createElement("a");
    link.href = href;
    link.download = filename;
    link.style.display = "none";
    document.body?.appendChild(link);
    link.click();
    link.remove();
    return true;
  } finally {
    URL.revokeObjectURL(href);
  }
}

function downloadText(filename: string, text: string, mime: string): boolean {
  try {
    return downloadBlob(filename, new Blob([text], { type: mime }));
  } catch {
    return false;
  }
}

function csvByteLength(text: string): number {
  if (typeof TextEncoder !== "undefined") {
    return new TextEncoder().encode(text).length;
  }
  return text.length;
}

export async function importOptionsDataset(
  req: Record<string, unknown>,
): Promise<OptionsDatasetManifestSummary> {
  const raw = await invokeCommand("import_options_dataset", { req });
  return normalizeOptionsDatasetManifest(raw);
}

export async function downloadPolygonOptionsDataset(
  req: PolygonOptionsDatasetDownloadRequestSummary,
): Promise<PolygonOptionsDatasetDownloadSummary> {
  const commandReq: Record<string, unknown> = {
    root_symbol: normalizeText(req.rootSymbol).toUpperCase(),
    from: normalizeText(req.from),
    to: normalizeText(req.to),
  };
  const optionsDatasetId = normalizeText(req.optionsDatasetId);
  if (optionsDatasetId) commandReq.options_dataset_id = optionsDatasetId;
  const contractType = normalizeText(req.contractType);
  if (contractType) commandReq.contract_type = contractType;
  const expirationDateFrom = normalizeText(req.expirationDateFrom);
  if (expirationDateFrom) commandReq.expiration_date_from = expirationDateFrom;
  const expirationDateTo = normalizeText(req.expirationDateTo);
  if (expirationDateTo) commandReq.expiration_date_to = expirationDateTo;
  if (req.maxDteDays != null)
    commandReq.max_dte_days = Math.max(
      0,
      Math.floor(normalizeNumber(req.maxDteDays)),
    );
  if (req.strikeMin != null)
    commandReq.strike_min = normalizeNumber(req.strikeMin);
  if (req.strikeMax != null)
    commandReq.strike_max = normalizeNumber(req.strikeMax);
  if (req.maxContracts != null)
    commandReq.max_contracts = Math.max(
      1,
      Math.floor(normalizeNumber(req.maxContracts)),
    );
  const providerEntitlementProfile = normalizeText(
    req.providerEntitlementProfile,
  );
  if (providerEntitlementProfile)
    commandReq.provider_entitlement_profile = providerEntitlementProfile;
  const contractReferenceAsOfUtc = normalizeText(req.contractReferenceAsOfUtc);
  if (contractReferenceAsOfUtc)
    commandReq.contract_reference_as_of_utc = contractReferenceAsOfUtc;

  const raw = await invokeCommand("download_polygon_options_dataset", {
    req: commandReq,
  });
  return normalizePolygonOptionsDatasetDownload(raw);
}

export async function listOptionsDatasets(): Promise<
  OptionsDatasetManifestSummary[]
> {
  const raw = await invokeCommand("list_options_datasets");
  return parseBoundaryArray(raw).map((entry) =>
    normalizeOptionsDatasetManifest(entry),
  );
}

export async function loadOptionsDatasetManifest(
  optionsDatasetId: string,
): Promise<OptionsDatasetManifestSummary> {
  const raw = await invokeCommand("load_options_dataset_manifest", {
    optionsDatasetId: normalizeText(optionsDatasetId),
  });
  return normalizeOptionsDatasetManifest(raw);
}

export async function runOptionsExpression(
  req: Record<string, unknown>,
): Promise<RunOptionsExpressionSummary> {
  const raw = await invokeCommand("run_options_expression", { req });
  return normalizeRunOptionsExpression(raw);
}

export async function analyzeOptionsExpressionFeasibility(
  req: Record<string, unknown>,
): Promise<AnalyzeOptionsExpressionFeasibilitySummary> {
  const raw = await invokeCommand("analyze_options_expression_feasibility", {
    req,
  });
  return normalizeFeasibilityResponse(raw);
}

export async function validateOptionsExpressionArtifact(
  req: Record<string, unknown>,
): Promise<ValidateOptionsExpressionArtifactSummary> {
  const raw = await invokeCommand("validate_options_expression_artifact", {
    req,
  });
  return normalizeValidateOptionsExpressionArtifact(raw);
}

export async function analyzeOptionsExpressionValidationFeasibility(
  req: Record<string, unknown>,
): Promise<AnalyzeOptionsExpressionValidationFeasibilitySummary> {
  const raw = await invokeCommand(
    "analyze_options_expression_validation_feasibility",
    { req },
  );
  return normalizeValidationFeasibilityResponse(raw);
}

export async function listOptionsExpressionArtifacts(): Promise<
  OptionsExpressionArtifactSummary[]
> {
  const raw = await invokeCommand("list_artifact_library");
  const library = parseBoundaryObject(raw);
  const rows =
    parseBoundaryArray(library.optionsExpression).length > 0
      ? parseBoundaryArray(library.optionsExpression)
      : parseBoundaryArray(library.options_expression);
  return rows.map((entry) => normalizeOptionsExpressionArtifactSummary(entry));
}

export async function loadOptionsExpressionArtifact(
  path: string,
): Promise<OptionsExpressionArtifactRecord> {
  const raw = await invokeCommand("load_options_expression_artifact", {
    path: normalizeText(path),
  });
  return normalizeOptionsExpressionArtifact(raw);
}

export async function inspectOptionsExpressionTuple(input: {
  artifactPath: string;
  tupleIndex: number;
}): Promise<OptionsExpressionTupleInspectionSummary> {
  const raw = await invokeCommand("inspect_options_expression_tuple", {
    req: {
      artifact_path: normalizeText(input.artifactPath),
      tuple_index: Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))),
    },
  });
  return normalizeTupleInspection(raw);
}

export async function inspectOptionsExpressionEventPath(input: {
  artifactPath: string;
  tupleIndex: number;
  eventIndex: number;
}): Promise<OptionsExpressionEventPathInspectionSummary> {
  const raw = await invokeCommand("inspect_options_expression_event_path", {
    req: {
      artifact_path: normalizeText(input.artifactPath),
      tuple_index: Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))),
      event_index: Math.max(0, Math.floor(normalizeNumber(input.eventIndex))),
    },
  });
  return normalizeEventPathInspection(raw);
}

export async function pickOptionsCsvPath(): Promise<string | null> {
  try {
    const raw = await invokeCommand<string | null>("pick_csv_file");
    const path = normalizeText(raw);
    return path.length > 0 ? path : null;
  } catch {
    return null;
  }
}

export async function openOptionsPath(
  path: string,
): Promise<OpenOrRevealPathResult> {
  const normalized = normalizeText(path);
  if (!normalized) throw new Error("Path is required.");
  await invokeCommand("open_path", { path: normalized });
  return { path: normalized, openedWith: "open_path" };
}

export async function revealOptionsPath(
  path: string,
): Promise<OpenOrRevealPathResult> {
  const normalized = normalizeText(path);
  if (!normalized) throw new Error("Path is required.");
  await invokeCommand("reveal_path", { path: normalized });
  return { path: normalized, openedWith: "reveal_path" };
}

export async function openOrRevealOptionsPath(
  path: string,
): Promise<OpenOrRevealPathResult> {
  try {
    return await openOptionsPath(path);
  } catch {
    return await revealOptionsPath(path);
  }
}

export function normalizeOptionsDatasetIds(value: unknown): string[] {
  return normalizeStringList(value);
}

function buildOptionsExpressionCsvText(
  input: OptionsExpressionCsvExportInput,
): { filename: string; text: string } | null {
  const tuples = Array.isArray(input.tuples) ? input.tuples : [];
  if (tuples.length === 0) return null;
  const stem =
    normalizeText(input.filenameStem) ||
    `vectorgrid_options_expression_${normalizeText(input.rootSymbol).toLowerCase() || "results"}`;
  const header = [
    "delay_minutes",
    "dte_days",
    "structure_family",
    "moneyness_bucket",
    "hold_minutes",
    "target_return",
    "stop_return",
    "exit_template",
    "fill_model",
    "pre_sealed_rank",
    "sealed_finalist",
    "sealed_final_rank",
    "validation_verdict",
    "robust_score",
    "selection_valid_trades",
    "selection_invalid_events",
    "selection_invalid_rate",
    "selection_expectancy_per_trade",
    "selection_expectancy_per_premium",
    "selection_median_return_on_premium",
    "selection_win_rate",
    "selection_total_pnl_per_contract",
    "holdout_expectancy_per_premium",
    "sealed_expectancy_per_premium",
    "stress_expectancy_per_premium",
    "sealed_stress_expectancy_per_premium",
    "harsher_stress_expectancy_per_premium",
    "sealed_harsher_stress_expectancy_per_premium",
    "extreme_stress_expectancy_per_premium",
    "sealed_extreme_stress_expectancy_per_premium",
    "backend_used",
    "price_semantics",
  ];
  const lines = [
    header.join(","),
    ...tuples.map((row) =>
      [
        row.delayMinutes,
        row.dteDays,
        row.structureFamily,
        row.moneynessBucket,
        row.holdMinutes,
        row.targetReturn ?? "",
        row.stopReturn ?? "",
        row.exitTemplate,
        row.fillModel,
        row.preSealedRank,
        row.sealedFinalist ? "yes" : "",
        row.sealedFinalRank ?? "",
        row.validationVerdict ?? "search_only",
        row.robustScore,
        row.selectionMetrics.validTrades,
        row.selectionMetrics.invalidEvents,
        row.selectionMetrics.invalidRate,
        row.selectionMetrics.expectancyPerTrade,
        row.selectionMetrics.expectancyPerPremium,
        row.selectionMetrics.medianReturnOnPremium,
        row.selectionMetrics.winRate,
        row.selectionMetrics.totalPnlPerContract,
        row.holdoutMetrics?.expectancyPerPremium ?? "",
        row.sealedEvaluationMetrics?.expectancyPerPremium ?? "",
        row.stressMetrics?.expectancyPerPremium ?? "",
        row.sealedStressMetrics?.expectancyPerPremium ?? "",
        row.harsherStressMetrics?.expectancyPerPremium ?? "",
        row.sealedHarsherStressMetrics?.expectancyPerPremium ?? "",
        row.extremeStressMetrics?.expectancyPerPremium ?? "",
        row.sealedExtremeStressMetrics?.expectancyPerPremium ?? "",
        normalizeText(input.backendUsed),
        normalizeText(input.priceSemantics),
      ]
        .map((value) => csvCell(value))
        .join(","),
    ),
  ];
  return {
    filename: `${stem}.csv`,
    text: lines.join("\r\n"),
  };
}

function buildOptionsExpressionInspectionCsvText(
  input: OptionsExpressionInspectionCsvExportInput,
): { filename: string; text: string } | null {
  const rows = Array.isArray(input.rows) ? input.rows : [];
  if (rows.length === 0) return null;
  const stem =
    normalizeText(input.filenameStem) ||
    `vectorgrid_options_expression_tuple_${Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))) + 1}`;
  const header = [
    "event_index",
    "event_id",
    "split",
    "side",
    "status",
    "candidate_invalid_reason",
    "decision_timestamp",
    "earliest_tradable_timestamp",
    "requested_entry_timestamp",
    "raw_underlier_price",
    "base_exit_timestamp",
    "entry_timestamp",
    "entry_lag_minutes",
    "minutes_to_expiry_at_entry",
    "realized_hold_minutes",
    "contract_id",
    "companion_contract_id",
    "expiration_date",
    "strike",
    "companion_strike",
    "multiplier",
    "entry_option_price",
    "companion_entry_option_price",
    "entry_underlier_price",
    "exit_timestamp",
    "exit_option_price",
    "companion_exit_option_price",
    "exit_reason",
    "premium_paid",
    "pnl_per_contract",
    "return_on_premium",
    "path_option_low",
    "path_option_high",
    "path_underlier_low",
    "path_underlier_high",
    "artifact_path",
  ];
  const lines = [
    header.join(","),
    ...rows.map((row) =>
      [
        row.eventIndex,
        row.eventId,
        row.split,
        row.side,
        row.status,
        row.candidateInvalidReason,
        row.decisionTimestamp,
        row.earliestTradableTimestamp,
        row.requestedEntryTimestamp,
        row.rawUnderlierPrice,
        row.baseExitTimestamp ?? "",
        row.entryTimestamp ?? "",
        row.entryLagMinutes ?? "",
        row.minutesToExpiryAtEntry ?? "",
        row.realizedHoldMinutes ?? "",
        row.contractId,
        row.companionContractId,
        row.expirationDate,
        row.strike ?? "",
        row.companionStrike ?? "",
        row.multiplier ?? "",
        row.entryOptionPrice ?? "",
        row.companionEntryOptionPrice ?? "",
        row.entryUnderlierPrice ?? "",
        row.exitTimestamp ?? "",
        row.exitOptionPrice ?? "",
        row.companionExitOptionPrice ?? "",
        row.exitReason,
        row.premiumPaid ?? "",
        row.pnlPerContract ?? "",
        row.returnOnPremium ?? "",
        row.pathOptionLow ?? "",
        row.pathOptionHigh ?? "",
        row.pathUnderlierLow ?? "",
        row.pathUnderlierHigh ?? "",
        normalizeText(input.artifactPath),
      ]
        .map((value) => csvCell(value))
        .join(","),
    ),
  ];
  return {
    filename: `${stem}.csv`,
    text: lines.join("\r\n"),
  };
}

function buildOptionsExpressionEventPathCsvText(
  input: OptionsExpressionEventPathCsvExportInput,
): { filename: string; text: string } | null {
  const points = Array.isArray(input.points) ? input.points : [];
  if (points.length === 0) return null;
  const stem =
    normalizeText(input.filenameStem) ||
    `vectorgrid_options_expression_tuple_${Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))) + 1}_event_path`;
  const header = [
    "timestamp_ms",
    "option_open",
    "option_high",
    "option_low",
    "option_close",
    "underlying_price",
    "artifact_path",
    "tuple_index",
    "event_id",
  ];
  const lines = [
    header.join(","),
    ...points.map((point) =>
      [
        point.timestampMs,
        point.optionOpen,
        point.optionHigh,
        point.optionLow,
        point.optionClose,
        point.underlyingPrice,
        normalizeText(input.artifactPath),
        input.tupleIndex == null ? "" : input.tupleIndex,
        normalizeText(input.eventId),
      ]
        .map((value) => csvCell(value))
        .join(","),
    ),
  ];
  return {
    filename: `${stem}.csv`,
    text: lines.join("\r\n"),
  };
}

async function saveOptionsCsvTextDesktop(
  filename: string,
  text: string,
): Promise<OptionsExpressionCsvExportResult> {
  const bytes = csvByteLength(text);
  let pickedPath: string | null;
  try {
    const raw = await invokeCommand<string | null>("pick_save_csv", {
      defaultName: filename,
    });
    pickedPath = normalizeText(raw) || null;
  } catch {
    const downloaded = downloadText(filename, text, "text/csv;charset=utf-8");
    return {
      mode: downloaded ? "downloaded" : "unavailable",
      filename,
      path: null,
      bytes,
      revealed: false,
    };
  }

  if (!pickedPath) {
    return {
      mode: "cancelled",
      filename,
      path: null,
      bytes,
      revealed: false,
    };
  }

  const raw = await invokeCommand<{ path?: string; bytes?: number }>(
    "save_options_csv",
    {
      req: {
        path: pickedPath,
        csv_text: text,
      },
    },
  );
  const savedPath = normalizeText(raw?.path) || pickedPath;
  let revealed = false;
  try {
    await invokeCommand("reveal_path", { path: savedPath });
    revealed = true;
  } catch {
    revealed = false;
  }
  return {
    mode: "saved",
    filename,
    path: savedPath,
    bytes: normalizeNumber(raw?.bytes, bytes),
    revealed,
  };
}

export async function exportOptionsExpressionCsv(
  input: OptionsExpressionCsvExportInput,
): Promise<OptionsExpressionCsvExportResult> {
  const built = buildOptionsExpressionCsvText(input);
  if (!built) {
    return {
      mode: "unavailable",
      filename: "",
      path: null,
      bytes: 0,
      revealed: false,
    };
  }
  return saveOptionsCsvTextDesktop(built.filename, built.text);
}

export async function exportOptionsExpressionInspectionCsv(
  input: OptionsExpressionInspectionCsvExportInput,
): Promise<OptionsExpressionCsvExportResult> {
  const built = buildOptionsExpressionInspectionCsvText(input);
  if (!built) {
    return {
      mode: "unavailable",
      filename: "",
      path: null,
      bytes: 0,
      revealed: false,
    };
  }
  return saveOptionsCsvTextDesktop(built.filename, built.text);
}

export async function exportOptionsExpressionEventPathCsv(
  input: OptionsExpressionEventPathCsvExportInput,
): Promise<OptionsExpressionCsvExportResult> {
  const built = buildOptionsExpressionEventPathCsvText(input);
  if (!built) {
    return {
      mode: "unavailable",
      filename: "",
      path: null,
      bytes: 0,
      revealed: false,
    };
  }
  return saveOptionsCsvTextDesktop(built.filename, built.text);
}
