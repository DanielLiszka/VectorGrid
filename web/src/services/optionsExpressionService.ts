import { invokeCommand } from '../tauri/invoke'

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeNumber(value: unknown, fallback = 0): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function normalizeOptionalNumber(value: unknown): number | null {
  if (value == null) return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeStringList(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return value.map((entry) => normalizeText(entry)).filter((entry) => entry.length > 0)
}

export interface OptionsDatasetManifestSummary {
  version: number
  optionsDatasetId: string
  rootSymbol: string
  providerName: string
  providerSourceSurface: string
  providerEntitlementProfile: string
  quoteResolution: string
  priceSemantics: string
  calendarId: string
  timezone: string
  coverageStart: number
  coverageEnd: number
  normalizationVersion: number
  quoteSemanticsVersion: number
  corporateActionMappingVersion: number
  contractReferenceSource: string
  contractReferenceAsOfUtc: string
  underlierReferenceSource: string
  datasetPath: string
  rowCount: number
  contractCount: number
  aggregatePriceField: string
}

export interface PolygonOptionsDatasetDownloadRequestSummary {
  rootSymbol: string
  from: string
  to: string
  optionsDatasetId?: string
  contractType?: string
  expirationDateFrom?: string
  expirationDateTo?: string
  maxDteDays?: number | null
  strikeMin?: number | null
  strikeMax?: number | null
  maxContracts?: number | null
  providerEntitlementProfile?: string
  contractReferenceAsOfUtc?: string
}

export interface PolygonOptionsDatasetDownloadSummary {
  manifest: OptionsDatasetManifestSummary
  contractsDiscovered: number
  contractsSelected: number
  contractsWithRows: number
  downloadedOptionRows: number
  skippedContracts: number
  underlyingCsvPath: string
  usedUnderlyingCacheOnly: boolean
}

export interface OptionsExpressionMetricsSummary {
  validTrades: number
  invalidEvents: number
  invalidRate: number
  expectancyPerTrade: number
  expectancyPerPremium: number
  medianReturnOnPremium: number
  winRate: number
  totalPnlPerContract: number
}

export interface OptionsStaticFilterBundleSummary {
  maxEntryLagMinutes: number | null
  maxPremium: number | null
  minPremium: number | null
  minVolume: number | null
  minOpenInterest: number | null
}

export interface OptionsExpressionRequestSummary {
  parentArtifactPath: string
  parentCandidateIndex: number | null
  optionsDatasetId: string
  structureFamily?: string
  backendPreference?: string
  splitAssignmentMode: string
  delayMinutes: number[]
  dteBuckets: number[]
  moneynessBuckets: string[]
  holdMinutes: number[]
  targetReturnBuckets: number[]
  stopReturnBuckets: number[]
  exitTemplate: string
  fillModel: string
  filters: OptionsStaticFilterBundleSummary
  topK: number
  minValidTrades: number | null
  splitRatioSearch: number
  splitRatioHoldout: number
  sealedFinalistCount?: number
}

export type OptionsExpressionValidationVerdict =
  | 'search_only'
  | 'holdout_supported'
  | 'sealed_finalist'
  | 'stress_fragile'

export interface OptionsExpressionTupleSummary {
  structureFamily?: string
  delayMinutes: number
  dteDays: number
  moneynessBucket: string
  holdMinutes: number
  targetReturn: number | null
  stopReturn: number | null
  exitTemplate: string
  fillModel: string
  preSealedRank?: number | undefined
  sealedFinalist?: boolean | undefined
  sealedFinalRank?: number | undefined
  validationVerdict?: OptionsExpressionValidationVerdict | undefined
  robustScore: number
  selectionMetrics: OptionsExpressionMetricsSummary
  holdoutMetrics: OptionsExpressionMetricsSummary | null
  sealedEvaluationMetrics: OptionsExpressionMetricsSummary | null
  stressMetrics: OptionsExpressionMetricsSummary | null
  sealedStressMetrics?: OptionsExpressionMetricsSummary | null
  harsherStressMetrics?: OptionsExpressionMetricsSummary | null
  sealedHarsherStressMetrics?: OptionsExpressionMetricsSummary | null
  extremeStressMetrics?: OptionsExpressionMetricsSummary | null
  sealedExtremeStressMetrics?: OptionsExpressionMetricsSummary | null
}

export interface OptionsExpressionDiagnosticsSummary {
  parentArtifactPath: string
  optionsDatasetId: string
  rootSymbol: string
  totalEvents: number
  bullishEvents: number
  bearishEvents: number
  searchEvents: number
  holdoutEvents: number
  sealedTestEvents: number
  datasetContracts: number
  datasetRows: number
  coverageStart: number | null
  coverageEnd: number | null
}

export interface OptionsExpressionStagingSummary {
  candidateSlotCount: number
  compiledCandidateCount: number
  baselineExitSummaryCount: number
  conservativeExitSummaryCount: number
  harsherExitSummaryCount: number
}

export interface OptionsCandidateInvalidReasonCountSummary {
  reason: string
  count: number
}

export interface OptionsDelayBucketCoverageSummary {
  delayMinutes: number
  slotCount: number
  validSlotCount: number
  invalidSlotCount: number
  validRate: number
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[]
}

export interface OptionsDteBucketCoverageSummary {
  dteDays: number
  slotCount: number
  validSlotCount: number
  invalidSlotCount: number
  validRate: number
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[]
}

export interface OptionsMoneynessBucketCoverageSummary {
  moneynessBucket: string
  slotCount: number
  validSlotCount: number
  invalidSlotCount: number
  validRate: number
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[]
}

export interface OptionsCandidateAxisCoverageSummary {
  delayMinutes: OptionsDelayBucketCoverageSummary[]
  dteDays: OptionsDteBucketCoverageSummary[]
  moneynessBuckets: OptionsMoneynessBucketCoverageSummary[]
}

export interface OptionsExpressionFeasibilityTimingSummary {
  tupleCompileMs: number
  baselineCandidateCompileMs: number
  baselineExitSummaryCompileMs: number
  conservativeCandidateCompileMs: number
  conservativeExitSummaryCompileMs: number
  harsherCandidateCompileMs: number
  harsherExitSummaryCompileMs: number
  cpuRankingMs: number
  cpuRankingUs: number
  gpuRankingMs: number | null
  gpuRankingUs: number | null
  gpuTensorScoreMs: number | null
  gpuTensorScoreUs: number | null
  gpuExactifyMs: number | null
  gpuExactifyUs: number | null
  gpuSealedStageMs: number | null
  gpuSealedStageUs: number | null
  gpuSpeedupVsCpu: number | null
}

export interface OptionsExpressionFeasibilityEventCoverageSummary {
  totalEventsWithCandidates: number
  searchEventsWithCandidates: number
  holdoutEventsWithCandidates: number
  sealedTestEventsWithCandidates: number
  totalEventsWithBaselineOutcomes: number
  searchEventsWithBaselineOutcomes: number
  holdoutEventsWithBaselineOutcomes: number
  sealedTestEventsWithBaselineOutcomes: number
  totalEventsWithConservativeOutcomes: number
  searchEventsWithConservativeOutcomes: number
  holdoutEventsWithConservativeOutcomes: number
  sealedTestEventsWithConservativeOutcomes: number
  totalEventsWithHarsherOutcomes: number
  searchEventsWithHarsherOutcomes: number
  holdoutEventsWithHarsherOutcomes: number
  sealedTestEventsWithHarsherOutcomes: number
}

export interface OptionsExpressionOverlapWindowSummary {
  contractReferenceAsOfUtc: string | null
  totalEventsInDatasetWindow: number
  searchEventsInDatasetWindow: number
  holdoutEventsInDatasetWindow: number
  sealedTestEventsInDatasetWindow: number
  totalEventsBeforeDatasetWindow: number
  totalEventsAfterDatasetWindow: number
  searchEventsBeforeDatasetWindow: number
  searchEventsAfterDatasetWindow: number
  holdoutEventsBeforeDatasetWindow: number
  holdoutEventsAfterDatasetWindow: number
  sealedTestEventsBeforeDatasetWindow: number
  sealedTestEventsAfterDatasetWindow: number
  totalEventsBeforeContractReferenceAsOf: number
  searchEventsBeforeContractReferenceAsOf: number
  holdoutEventsBeforeContractReferenceAsOf: number
  sealedTestEventsBeforeContractReferenceAsOf: number
  totalOverlapRate: number
  searchOverlapRate: number
  holdoutOverlapRate: number
  sealedTestOverlapRate: number
}

export interface OptionsExpressionSplitInvalidReasonSummary {
  searchInvalidReasons: OptionsCandidateInvalidReasonCountSummary[]
  holdoutInvalidReasons: OptionsCandidateInvalidReasonCountSummary[]
  sealedTestInvalidReasons: OptionsCandidateInvalidReasonCountSummary[]
}

export type OptionsExpressionCoverageAdequacyVerdict =
  | 'unknown'
  | 'healthy'
  | 'weak_dataset_overlap'
  | 'search_overlap_missing'
  | 'search_coverage_weak'
  | 'holdout_coverage_weak'
  | 'sealed_coverage_weak'
  | 'contract_eligibility_fragile'
  | 'late_entry_fragile'
  | 'strike_window_fragile'
  | 'mixed_fragile'

export interface OptionsExpressionCoverageAdequacySummary {
  verdict: OptionsExpressionCoverageAdequacyVerdict
  nonPromotable: boolean
  reasons: string[]
  searchCandidateCoverageRate: number
  holdoutCandidateCoverageRate: number
  sealedTestCandidateCoverageRate: number
  searchBaselineOutcomeCoverageRate: number
  holdoutBaselineOutcomeCoverageRate: number
  sealedTestBaselineOutcomeCoverageRate: number
  dominantInvalidReason: string
  dominantInvalidReasonShare: number
}

export interface OptionsExpressionFeasibilityCacheStatsSummary {
  structuralCacheHits: number
  structuralCacheMisses: number
  structuralCacheEntries: number
  summaryCacheHits: number
  summaryCacheMisses: number
  summaryCacheEntries: number
  gpuSharedInputCacheHits: number
  gpuSharedInputCacheMisses: number
  gpuOutcomeInputCacheHits: number
  gpuOutcomeInputCacheMisses: number
  gpuOutputBufferReuses: number
  gpuOutputBufferAllocations: number
  gpuTupleChunkCount: number
  gpuTupleChunkSize: number
  gpuKernelLaunchCount: number
  gpuTupleBudgetBytes: number
  gpuOutcomeTensorBytes: number
  gpuSharedInputBytesPerChunk: number
  gpuOutputBytesPerChunk: number
  gpuTotalDeviceBytesPerChunk: number
  gpuDeviceTotalMemoryBytes: number
  gpuDeviceFreeMemoryBytes: number
  gpuDeviceUsableMemoryBytes: number
  gpuMemoryBudgetSource: string
  gpuOutcomeTensorFitsInMemory: boolean
  gpuChunkPlanFitsInMemory: boolean
  gpuPlanNotes: string[]
}

export interface OptionsExpressionAutoBackendDecisionSummary {
  recommendedBackend: string
  gpuEligible: boolean
  reasons: string[]
}

export interface AnalyzeOptionsExpressionFeasibilitySummary {
  replayBuildMs: number
  totalPreviewMs: number
  diagnostics: OptionsExpressionDiagnosticsSummary
  priceSemantics: string
  fillModel: string
  bestAvailableBackend: string
  autoBackendDecision: OptionsExpressionAutoBackendDecisionSummary
  compiledTupleCount: number
  perEventCandidateSlots: number
  perCandidateExitSummaries: number
  candidateSlotCount: number
  compiledCandidateCount: number
  candidateDensity: number
  invalidSlotCount: number
  invalidSlotRate: number
  baselineExitSummaryCount: number
  baselineCompiledExitSummaryCount: number
  baselineExitSummaryDensity: number
  conservativeExitSummaryCount: number
  conservativeCompiledExitSummaryCount: number
  conservativeExitSummaryDensity: number
  harsherExitSummaryCount: number
  harsherCompiledExitSummaryCount: number
  harsherExitSummaryDensity: number
  projectedEventTupleEvaluations: number
  structuralCacheBytesEstimate: number
  baselineSummaryBytesEstimate: number
  conservativeSummaryBytesEstimate: number
  harsherSummaryBytesEstimate: number
  timings: OptionsExpressionFeasibilityTimingSummary
  eventCoverage: OptionsExpressionFeasibilityEventCoverageSummary
  overlapWindow: OptionsExpressionOverlapWindowSummary
  splitInvalidReasons: OptionsExpressionSplitInvalidReasonSummary
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary
  cacheStats: OptionsExpressionFeasibilityCacheStatsSummary
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[]
  axisCoverage: OptionsCandidateAxisCoverageSummary
}

export interface RunOptionsExpressionSummary {
  artifactPath: string
  artifactTs: string
  parentArtifactId: string
  optionsDatasetHash: string
  backendUsed: string
  backendFallbackReason?: string
  priceSemantics: string
  tupleCount: number
  request: OptionsExpressionRequestSummary | null
  diagnostics: OptionsExpressionDiagnosticsSummary
  staging: OptionsExpressionStagingSummary | null
  overlapWindow: OptionsExpressionOverlapWindowSummary
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary
  splitInvalidReasons: OptionsExpressionSplitInvalidReasonSummary
  rankedResults: OptionsExpressionTupleSummary[]
}

export interface OptionsExpressionArtifactSummary {
  ts: string
  path: string
  bytes: number
  parentArtifactId: string
  parentStrategyKind?: string
  rootSymbol: string
  optionsDatasetId: string
  backendUsed: string
  backendFallbackReason?: string
  priceSemantics: string
  requestBackendPreference?: string
  requestFillModel?: string
  requestStructureFamily?: string
  requestSplitAssignmentMode?: string
  coverageAdequacyVerdict: OptionsExpressionCoverageAdequacyVerdict
  coverageNonPromotable: boolean
  searchOverlapRate: number
  holdoutOverlapRate: number
  sealedTestOverlapRate: number
  searchCandidateCoverageRate: number
  holdoutCandidateCoverageRate: number
  sealedTestCandidateCoverageRate: number
  searchBaselineOutcomeCoverageRate: number
  holdoutBaselineOutcomeCoverageRate: number
  sealedTestBaselineOutcomeCoverageRate: number
  tupleCount: number
  bestRobustScore: number
  bestSelectionExpectancyPerPremium: number
  bestHoldoutExpectancyPerPremium: number | null
  bestSealedExpectancyPerPremium: number | null
  bestStressExpectancyPerPremium: number | null
  bestSealedStressExpectancyPerPremium: number | null
  bestHarsherStressExpectancyPerPremium: number | null
  bestSealedHarsherStressExpectancyPerPremium: number | null
  bestExtremeStressExpectancyPerPremium: number | null
  bestSealedExtremeStressExpectancyPerPremium: number | null
  topValidationVerdict?: OptionsExpressionValidationVerdict | undefined
  sealedFinalistBudget?: number
  sealedEvaluatedTupleCount?: number
  validatedTupleCount?: number
  stressFragileTupleCount?: number
  hasHoldoutMetrics: boolean
  hasSealedEvaluationMetrics: boolean
  hasStressMetrics: boolean
  hasHarsherStressMetrics: boolean
  hasExtremeStressMetrics: boolean
}

export interface OptionsExpressionArtifactRecord {
  version: number
  kind: string
  ts: string
  artifactPath: string
  parentArtifactId: string
  rootSymbol: string
  optionsDatasetId: string
  optionsDatasetHash: string
  backendUsed: string
  backendFallbackReason?: string
  priceSemantics: string
  request: OptionsExpressionRequestSummary
  diagnostics: OptionsExpressionDiagnosticsSummary
  staging: OptionsExpressionStagingSummary | null
  overlapWindow: OptionsExpressionOverlapWindowSummary
  coverageAdequacy: OptionsExpressionCoverageAdequacySummary
  splitInvalidReasons: OptionsExpressionSplitInvalidReasonSummary
  selectionMetrics: OptionsExpressionTupleSummary[]
}

export interface OptionsExpressionEventInspectionRow {
  eventIndex: number
  eventId: string
  split: string
  side: string
  decisionTimestamp: number
  earliestTradableTimestamp: number
  requestedEntryTimestamp: number
  rawUnderlierPrice: number
  baseExitTimestamp: number | null
  status: string
  candidateInvalidReason: string
  contractId: string
  companionContractId?: string
  expirationDate: string
  strike: number | null
  companionStrike?: number | null
  multiplier: number | null
  entryTimestamp: number | null
  entryOptionPrice: number | null
  companionEntryOptionPrice?: number | null
  entryUnderlierPrice: number | null
  exitTimestamp: number | null
  exitOptionPrice: number | null
  companionExitOptionPrice?: number | null
  premiumPaid: number | null
  pnlPerContract: number | null
  returnOnPremium: number | null
  entryLagMinutes: number | null
  minutesToExpiryAtEntry: number | null
  realizedHoldMinutes: number | null
  pathOptionHigh: number | null
  pathOptionLow: number | null
  pathUnderlierHigh: number | null
  pathUnderlierLow: number | null
  exitReason: string
}

export interface OptionsExpressionTupleInspectionSummary {
  tupleIndex: number
  tupleCount: number
  tupleResult: OptionsExpressionTupleSummary
  totalEvents: number
  executedEvents: number
  candidateMissingEvents: number
  outcomeMissingEvents: number
  rows: OptionsExpressionEventInspectionRow[]
}

export interface OptionsExpressionEventPathPointSummary {
  timestampMs: number
  optionOpen: number
  optionHigh: number
  optionLow: number
  optionClose: number
  underlyingPrice: number
}

export interface OptionsExpressionEventPathInspectionSummary {
  tupleIndex: number
  eventIndex: number
  eventId: string
  row: OptionsExpressionEventInspectionRow
  pointCount: number
  points: OptionsExpressionEventPathPointSummary[]
}

export interface OpenOrRevealPathResult {
  path: string
  openedWith: 'open_path' | 'reveal_path'
}

export interface OptionsExpressionCsvExportInput {
  filenameStem?: string
  rootSymbol?: string
  backendUsed?: string
  priceSemantics?: string
  tuples: OptionsExpressionTupleSummary[]
}

export interface OptionsExpressionInspectionCsvExportInput {
  filenameStem?: string
  artifactPath?: string
  tupleIndex?: number
  rows: OptionsExpressionEventInspectionRow[]
}

export interface OptionsExpressionEventPathCsvExportInput {
  filenameStem?: string
  artifactPath?: string
  tupleIndex?: number
  eventId?: string
  points: OptionsExpressionEventPathPointSummary[]
}

export type OptionsExpressionCsvExportMode =
  | 'saved'
  | 'downloaded'
  | 'cancelled'
  | 'unavailable'

export interface OptionsExpressionCsvExportResult {
  mode: OptionsExpressionCsvExportMode
  filename: string
  path: string | null
  bytes: number
  revealed: boolean
}

function normalizeOptionsDatasetManifest(raw: unknown): OptionsDatasetManifestSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
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
    corporateActionMappingVersion: normalizeNumber(row.corporate_action_mapping_version),
    contractReferenceSource: normalizeText(row.contract_reference_source),
    contractReferenceAsOfUtc: normalizeText(row.contract_reference_as_of_utc),
    underlierReferenceSource: normalizeText(row.underlier_reference_source),
    datasetPath: normalizeText(row.dataset_path),
    rowCount: normalizeNumber(row.row_count),
    contractCount: normalizeNumber(row.contract_count),
    aggregatePriceField: normalizeText(row.aggregate_price_field),
  }
}

function normalizePolygonOptionsDatasetDownload(raw: unknown): PolygonOptionsDatasetDownloadSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    manifest: normalizeOptionsDatasetManifest(row.manifest),
    contractsDiscovered: normalizeNumber(row.contracts_discovered),
    contractsSelected: normalizeNumber(row.contracts_selected),
    contractsWithRows: normalizeNumber(row.contracts_with_rows),
    downloadedOptionRows: normalizeNumber(row.downloaded_option_rows),
    skippedContracts: normalizeNumber(row.skipped_contracts),
    underlyingCsvPath: normalizeText(row.underlying_csv_path),
    usedUnderlyingCacheOnly: Boolean(row.used_underlying_cache_only),
  }
}

function normalizeMetrics(raw: unknown): OptionsExpressionMetricsSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    validTrades: normalizeNumber(row.valid_trades),
    invalidEvents: normalizeNumber(row.invalid_events),
    invalidRate: normalizeNumber(row.invalid_rate),
    expectancyPerTrade: normalizeNumber(row.expectancy_per_trade),
    expectancyPerPremium: normalizeNumber(row.expectancy_per_premium),
    medianReturnOnPremium: normalizeNumber(row.median_return_on_premium),
    winRate: normalizeNumber(row.win_rate),
    totalPnlPerContract: normalizeNumber(row.total_pnl_per_contract),
  }
}

function normalizeValidationVerdict(raw: unknown): OptionsExpressionValidationVerdict {
  const value = normalizeText(raw)
  switch (value) {
    case 'holdout_supported':
    case 'sealed_finalist':
    case 'stress_fragile':
      return value
    case 'search_only':
    default:
      return 'search_only'
  }
}

function normalizeTuple(raw: unknown): OptionsExpressionTupleSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const tuple = (row.tuple && typeof row.tuple === 'object' ? row.tuple : {}) as Record<string, unknown>
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
    preSealedRank: row.pre_sealed_rank == null ? undefined : normalizeNumber(row.pre_sealed_rank),
    sealedFinalist: Boolean(row.sealed_finalist),
    sealedFinalRank: row.sealed_final_rank == null ? undefined : normalizeNumber(row.sealed_final_rank),
    validationVerdict: normalizeValidationVerdict(row.validation_verdict),
    robustScore: normalizeNumber(row.robust_score),
    selectionMetrics: normalizeMetrics(row.selection_metrics),
    holdoutMetrics: row.holdout_metrics ? normalizeMetrics(row.holdout_metrics) : null,
    sealedEvaluationMetrics: row.sealed_evaluation_metrics
      ? normalizeMetrics(row.sealed_evaluation_metrics)
      : null,
    stressMetrics: row.stress_metrics ? normalizeMetrics(row.stress_metrics) : null,
    sealedStressMetrics: row.sealed_stress_metrics ? normalizeMetrics(row.sealed_stress_metrics) : null,
    harsherStressMetrics: row.harsher_stress_metrics ? normalizeMetrics(row.harsher_stress_metrics) : null,
    sealedHarsherStressMetrics: row.sealed_harsher_stress_metrics
      ? normalizeMetrics(row.sealed_harsher_stress_metrics)
      : null,
    extremeStressMetrics: row.extreme_stress_metrics
      ? normalizeMetrics(row.extreme_stress_metrics)
      : null,
    sealedExtremeStressMetrics: row.sealed_extreme_stress_metrics
      ? normalizeMetrics(row.sealed_extreme_stress_metrics)
      : null,
  }
}

function normalizeDiagnostics(raw: unknown): OptionsExpressionDiagnosticsSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
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
  }
}

function normalizeStaging(raw: unknown): OptionsExpressionStagingSummary | null {
  if (!raw || typeof raw !== 'object') return null
  const row = raw as Record<string, unknown>
  return {
    candidateSlotCount: normalizeNumber(row.candidate_slot_count),
    compiledCandidateCount: normalizeNumber(row.compiled_candidate_count),
    baselineExitSummaryCount: normalizeNumber(row.baseline_exit_summary_count),
    conservativeExitSummaryCount: normalizeNumber(row.conservative_exit_summary_count),
    harsherExitSummaryCount: normalizeNumber(row.harsher_exit_summary_count),
  }
}

function normalizeFeasibilityTiming(raw: unknown): OptionsExpressionFeasibilityTimingSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    tupleCompileMs: normalizeNumber(row.tuple_compile_ms),
    baselineCandidateCompileMs: normalizeNumber(row.baseline_candidate_compile_ms),
    baselineExitSummaryCompileMs: normalizeNumber(row.baseline_exit_summary_compile_ms),
    conservativeCandidateCompileMs: normalizeNumber(row.conservative_candidate_compile_ms),
    conservativeExitSummaryCompileMs: normalizeNumber(row.conservative_exit_summary_compile_ms),
    harsherCandidateCompileMs: normalizeNumber(row.harsher_candidate_compile_ms),
    harsherExitSummaryCompileMs: normalizeNumber(row.harsher_exit_summary_compile_ms),
    cpuRankingMs: normalizeNumber(row.cpu_ranking_ms),
    cpuRankingUs: normalizeNumber(row.cpu_ranking_us),
    gpuRankingMs: row.gpu_ranking_ms == null ? null : normalizeNumber(row.gpu_ranking_ms),
    gpuRankingUs: row.gpu_ranking_us == null ? null : normalizeNumber(row.gpu_ranking_us),
    gpuTensorScoreMs: row.gpu_tensor_score_ms == null ? null : normalizeNumber(row.gpu_tensor_score_ms),
    gpuTensorScoreUs: row.gpu_tensor_score_us == null ? null : normalizeNumber(row.gpu_tensor_score_us),
    gpuExactifyMs: row.gpu_exactify_ms == null ? null : normalizeNumber(row.gpu_exactify_ms),
    gpuExactifyUs: row.gpu_exactify_us == null ? null : normalizeNumber(row.gpu_exactify_us),
    gpuSealedStageMs: row.gpu_sealed_stage_ms == null ? null : normalizeNumber(row.gpu_sealed_stage_ms),
    gpuSealedStageUs: row.gpu_sealed_stage_us == null ? null : normalizeNumber(row.gpu_sealed_stage_us),
    gpuSpeedupVsCpu: normalizeOptionalNumber(row.gpu_speedup_vs_cpu),
  }
}

function normalizeFeasibilityEventCoverage(raw: unknown): OptionsExpressionFeasibilityEventCoverageSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    totalEventsWithCandidates: normalizeNumber(row.total_events_with_candidates),
    searchEventsWithCandidates: normalizeNumber(row.search_events_with_candidates),
    holdoutEventsWithCandidates: normalizeNumber(row.holdout_events_with_candidates),
    sealedTestEventsWithCandidates: normalizeNumber(row.sealed_test_events_with_candidates),
    totalEventsWithBaselineOutcomes: normalizeNumber(row.total_events_with_baseline_outcomes),
    searchEventsWithBaselineOutcomes: normalizeNumber(row.search_events_with_baseline_outcomes),
    holdoutEventsWithBaselineOutcomes: normalizeNumber(row.holdout_events_with_baseline_outcomes),
    sealedTestEventsWithBaselineOutcomes: normalizeNumber(row.sealed_test_events_with_baseline_outcomes),
    totalEventsWithConservativeOutcomes: normalizeNumber(row.total_events_with_conservative_outcomes),
    searchEventsWithConservativeOutcomes: normalizeNumber(row.search_events_with_conservative_outcomes),
    holdoutEventsWithConservativeOutcomes: normalizeNumber(row.holdout_events_with_conservative_outcomes),
    sealedTestEventsWithConservativeOutcomes: normalizeNumber(row.sealed_test_events_with_conservative_outcomes),
    totalEventsWithHarsherOutcomes: normalizeNumber(row.total_events_with_harsher_outcomes),
    searchEventsWithHarsherOutcomes: normalizeNumber(row.search_events_with_harsher_outcomes),
    holdoutEventsWithHarsherOutcomes: normalizeNumber(row.holdout_events_with_harsher_outcomes),
    sealedTestEventsWithHarsherOutcomes: normalizeNumber(row.sealed_test_events_with_harsher_outcomes),
  }
}

function normalizeOverlapWindow(raw: unknown): OptionsExpressionOverlapWindowSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    contractReferenceAsOfUtc: normalizeText(row.contract_reference_as_of_utc) || null,
    totalEventsInDatasetWindow: normalizeNumber(row.total_events_in_dataset_window),
    searchEventsInDatasetWindow: normalizeNumber(row.search_events_in_dataset_window),
    holdoutEventsInDatasetWindow: normalizeNumber(row.holdout_events_in_dataset_window),
    sealedTestEventsInDatasetWindow: normalizeNumber(row.sealed_test_events_in_dataset_window),
    totalEventsBeforeDatasetWindow: normalizeNumber(row.total_events_before_dataset_window),
    totalEventsAfterDatasetWindow: normalizeNumber(row.total_events_after_dataset_window),
    searchEventsBeforeDatasetWindow: normalizeNumber(row.search_events_before_dataset_window),
    searchEventsAfterDatasetWindow: normalizeNumber(row.search_events_after_dataset_window),
    holdoutEventsBeforeDatasetWindow: normalizeNumber(row.holdout_events_before_dataset_window),
    holdoutEventsAfterDatasetWindow: normalizeNumber(row.holdout_events_after_dataset_window),
    sealedTestEventsBeforeDatasetWindow: normalizeNumber(row.sealed_test_events_before_dataset_window),
    sealedTestEventsAfterDatasetWindow: normalizeNumber(row.sealed_test_events_after_dataset_window),
    totalEventsBeforeContractReferenceAsOf: normalizeNumber(row.total_events_before_contract_reference_as_of),
    searchEventsBeforeContractReferenceAsOf: normalizeNumber(row.search_events_before_contract_reference_as_of),
    holdoutEventsBeforeContractReferenceAsOf: normalizeNumber(row.holdout_events_before_contract_reference_as_of),
    sealedTestEventsBeforeContractReferenceAsOf: normalizeNumber(row.sealed_test_events_before_contract_reference_as_of),
    totalOverlapRate: normalizeNumber(row.total_overlap_rate),
    searchOverlapRate: normalizeNumber(row.search_overlap_rate),
    holdoutOverlapRate: normalizeNumber(row.holdout_overlap_rate),
    sealedTestOverlapRate: normalizeNumber(row.sealed_test_overlap_rate),
  }
}

function normalizeSplitInvalidReasons(raw: unknown): OptionsExpressionSplitInvalidReasonSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    searchInvalidReasons: normalizeInvalidReasonCounts(row.search_invalid_reasons),
    holdoutInvalidReasons: normalizeInvalidReasonCounts(row.holdout_invalid_reasons),
    sealedTestInvalidReasons: normalizeInvalidReasonCounts(row.sealed_test_invalid_reasons),
  }
}

function normalizeCoverageAdequacy(raw: unknown): OptionsExpressionCoverageAdequacySummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    verdict: normalizeText(row.verdict) as OptionsExpressionCoverageAdequacyVerdict || 'unknown',
    nonPromotable: Boolean(row.non_promotable),
    reasons: normalizeStringList(row.reasons),
    searchCandidateCoverageRate: normalizeNumber(row.search_candidate_coverage_rate),
    holdoutCandidateCoverageRate: normalizeNumber(row.holdout_candidate_coverage_rate),
    sealedTestCandidateCoverageRate: normalizeNumber(row.sealed_test_candidate_coverage_rate),
    searchBaselineOutcomeCoverageRate: normalizeNumber(row.search_baseline_outcome_coverage_rate),
    holdoutBaselineOutcomeCoverageRate: normalizeNumber(row.holdout_baseline_outcome_coverage_rate),
    sealedTestBaselineOutcomeCoverageRate: normalizeNumber(row.sealed_test_baseline_outcome_coverage_rate),
    dominantInvalidReason: normalizeText(row.dominant_invalid_reason),
    dominantInvalidReasonShare: normalizeNumber(row.dominant_invalid_reason_share),
  }
}

function normalizeFeasibilityCacheStats(raw: unknown): OptionsExpressionFeasibilityCacheStatsSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    structuralCacheHits: normalizeNumber(row.structural_cache_hits),
    structuralCacheMisses: normalizeNumber(row.structural_cache_misses),
    structuralCacheEntries: normalizeNumber(row.structural_cache_entries),
    summaryCacheHits: normalizeNumber(row.summary_cache_hits),
    summaryCacheMisses: normalizeNumber(row.summary_cache_misses),
    summaryCacheEntries: normalizeNumber(row.summary_cache_entries),
    gpuSharedInputCacheHits: normalizeNumber(row.gpu_shared_input_cache_hits),
    gpuSharedInputCacheMisses: normalizeNumber(row.gpu_shared_input_cache_misses),
    gpuOutcomeInputCacheHits: normalizeNumber(row.gpu_outcome_input_cache_hits),
    gpuOutcomeInputCacheMisses: normalizeNumber(row.gpu_outcome_input_cache_misses),
    gpuOutputBufferReuses: normalizeNumber(row.gpu_output_buffer_reuses),
    gpuOutputBufferAllocations: normalizeNumber(row.gpu_output_buffer_allocations),
    gpuTupleChunkCount: normalizeNumber(row.gpu_tuple_chunk_count),
    gpuTupleChunkSize: normalizeNumber(row.gpu_tuple_chunk_size),
    gpuKernelLaunchCount: normalizeNumber(row.gpu_kernel_launch_count),
    gpuTupleBudgetBytes: normalizeNumber(row.gpu_tuple_budget_bytes),
    gpuOutcomeTensorBytes: normalizeNumber(row.gpu_outcome_tensor_bytes),
    gpuSharedInputBytesPerChunk: normalizeNumber(row.gpu_shared_input_bytes_per_chunk),
    gpuOutputBytesPerChunk: normalizeNumber(row.gpu_output_bytes_per_chunk),
    gpuTotalDeviceBytesPerChunk: normalizeNumber(row.gpu_total_device_bytes_per_chunk),
    gpuDeviceTotalMemoryBytes: normalizeNumber(row.gpu_device_total_memory_bytes),
    gpuDeviceFreeMemoryBytes: normalizeNumber(row.gpu_device_free_memory_bytes),
    gpuDeviceUsableMemoryBytes: normalizeNumber(row.gpu_device_usable_memory_bytes),
    gpuMemoryBudgetSource: normalizeText(row.gpu_memory_budget_source),
    gpuOutcomeTensorFitsInMemory: !!row.gpu_outcome_tensor_fits_in_memory,
    gpuChunkPlanFitsInMemory: !!row.gpu_chunk_plan_fits_in_memory,
    gpuPlanNotes: normalizeStringList(row.gpu_plan_notes),
  }
}

function normalizeInvalidReasonCounts(value: unknown): OptionsCandidateInvalidReasonCountSummary[] {
  if (!Array.isArray(value)) return []
  return value.map((entry) => {
    const row = (entry && typeof entry === 'object' ? entry : {}) as Record<string, unknown>
    return {
      reason: normalizeText(row.reason),
      count: normalizeNumber(row.count),
    }
  })
}

function normalizeDelayBucketCoverage(value: unknown): OptionsDelayBucketCoverageSummary[] {
  if (!Array.isArray(value)) return []
  return value.map((entry) => {
    const row = (entry && typeof entry === 'object' ? entry : {}) as Record<string, unknown>
    return {
      delayMinutes: normalizeNumber(row.delay_minutes),
      slotCount: normalizeNumber(row.slot_count),
      validSlotCount: normalizeNumber(row.valid_slot_count),
      invalidSlotCount: normalizeNumber(row.invalid_slot_count),
      validRate: normalizeNumber(row.valid_rate),
      invalidReasons: normalizeInvalidReasonCounts(row.invalid_reasons),
    }
  })
}

function normalizeDteBucketCoverage(value: unknown): OptionsDteBucketCoverageSummary[] {
  if (!Array.isArray(value)) return []
  return value.map((entry) => {
    const row = (entry && typeof entry === 'object' ? entry : {}) as Record<string, unknown>
    return {
      dteDays: normalizeNumber(row.dte_days),
      slotCount: normalizeNumber(row.slot_count),
      validSlotCount: normalizeNumber(row.valid_slot_count),
      invalidSlotCount: normalizeNumber(row.invalid_slot_count),
      validRate: normalizeNumber(row.valid_rate),
      invalidReasons: normalizeInvalidReasonCounts(row.invalid_reasons),
    }
  })
}

function normalizeMoneynessBucketCoverage(value: unknown): OptionsMoneynessBucketCoverageSummary[] {
  if (!Array.isArray(value)) return []
  return value.map((entry) => {
    const row = (entry && typeof entry === 'object' ? entry : {}) as Record<string, unknown>
    return {
      moneynessBucket: normalizeText(row.moneyness_bucket),
      slotCount: normalizeNumber(row.slot_count),
      validSlotCount: normalizeNumber(row.valid_slot_count),
      invalidSlotCount: normalizeNumber(row.invalid_slot_count),
      validRate: normalizeNumber(row.valid_rate),
      invalidReasons: normalizeInvalidReasonCounts(row.invalid_reasons),
    }
  })
}

function normalizeAxisCoverage(raw: unknown): OptionsCandidateAxisCoverageSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    delayMinutes: normalizeDelayBucketCoverage(row.delay_minutes),
    dteDays: normalizeDteBucketCoverage(row.dte_days),
    moneynessBuckets: normalizeMoneynessBucketCoverage(row.moneyness_buckets),
  }
}

function normalizeAutoBackendDecision(raw: unknown): OptionsExpressionAutoBackendDecisionSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    recommendedBackend: normalizeText(row.recommended_backend),
    gpuEligible: Boolean(row.gpu_eligible),
    reasons: normalizeStringList(row.reasons),
  }
}

function normalizeFeasibilityResponse(raw: unknown): AnalyzeOptionsExpressionFeasibilitySummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const report = (row.report && typeof row.report === 'object' ? row.report : {}) as Record<string, unknown>
  return {
    replayBuildMs: normalizeNumber(row.replay_build_ms),
    totalPreviewMs: normalizeNumber(row.total_preview_ms),
    diagnostics: normalizeDiagnostics(report.diagnostics),
    priceSemantics: normalizeText(report.price_semantics),
    fillModel: normalizeText(report.fill_model),
    bestAvailableBackend: normalizeText(report.best_available_backend),
    autoBackendDecision: normalizeAutoBackendDecision(report.auto_backend_decision),
    compiledTupleCount: normalizeNumber(report.compiled_tuple_count),
    perEventCandidateSlots: normalizeNumber(report.per_event_candidate_slots),
    perCandidateExitSummaries: normalizeNumber(report.per_candidate_exit_summaries),
    candidateSlotCount: normalizeNumber(report.candidate_slot_count),
    compiledCandidateCount: normalizeNumber(report.compiled_candidate_count),
    candidateDensity: normalizeNumber(report.candidate_density),
    invalidSlotCount: normalizeNumber(report.invalid_slot_count),
    invalidSlotRate: normalizeNumber(report.invalid_slot_rate),
    baselineExitSummaryCount: normalizeNumber(report.baseline_exit_summary_count),
    baselineCompiledExitSummaryCount: normalizeNumber(report.baseline_compiled_exit_summary_count),
    baselineExitSummaryDensity: normalizeNumber(report.baseline_exit_summary_density),
    conservativeExitSummaryCount: normalizeNumber(report.conservative_exit_summary_count),
    conservativeCompiledExitSummaryCount: normalizeNumber(report.conservative_compiled_exit_summary_count),
    conservativeExitSummaryDensity: normalizeNumber(report.conservative_exit_summary_density),
    harsherExitSummaryCount: normalizeNumber(report.harsher_exit_summary_count),
    harsherCompiledExitSummaryCount: normalizeNumber(report.harsher_compiled_exit_summary_count),
    harsherExitSummaryDensity: normalizeNumber(report.harsher_exit_summary_density),
    projectedEventTupleEvaluations: normalizeNumber(report.projected_event_tuple_evaluations),
    structuralCacheBytesEstimate: normalizeNumber(report.structural_cache_bytes_estimate),
    baselineSummaryBytesEstimate: normalizeNumber(report.baseline_summary_bytes_estimate),
    conservativeSummaryBytesEstimate: normalizeNumber(report.conservative_summary_bytes_estimate),
    harsherSummaryBytesEstimate: normalizeNumber(report.harsher_summary_bytes_estimate),
    timings: normalizeFeasibilityTiming(report.timings),
    eventCoverage: normalizeFeasibilityEventCoverage(report.event_coverage),
    overlapWindow: normalizeOverlapWindow(report.overlap_window),
    splitInvalidReasons: normalizeSplitInvalidReasons(report.split_invalid_reasons),
    coverageAdequacy: normalizeCoverageAdequacy(report.coverage_adequacy),
    cacheStats: normalizeFeasibilityCacheStats(report.cache_stats),
    invalidReasons: normalizeInvalidReasonCounts(report.invalid_reasons),
    axisCoverage: normalizeAxisCoverage(report.axis_coverage),
  }
}

function normalizeRunOptionsExpression(raw: unknown): RunOptionsExpressionSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const result = (row.result && typeof row.result === 'object' ? row.result : {}) as Record<string, unknown>
  const artifact = (row.artifact && typeof row.artifact === 'object' ? row.artifact : {}) as Record<string, unknown>
  const ranked = Array.isArray(result.ranked_results) ? result.ranked_results : []
  return {
    artifactPath: normalizeText(row.artifact_path),
    artifactTs: normalizeText(artifact.ts),
    parentArtifactId: normalizeText(artifact.parent_artifact_id),
    optionsDatasetHash: normalizeText(artifact.options_dataset_hash),
    backendUsed: normalizeText(result.backend_used),
    backendFallbackReason: normalizeText(result.backend_fallback_reason),
    priceSemantics: normalizeText(result.price_semantics),
    tupleCount: normalizeNumber(result.tuple_count),
    request: Object.keys(artifact).length > 0 ? normalizeRequest(artifact.request) : null,
    diagnostics: normalizeDiagnostics(result.diagnostics),
    staging: normalizeStaging(result.staging),
    overlapWindow: normalizeOverlapWindow(result.overlap_window || artifact.overlap_window),
    coverageAdequacy: normalizeCoverageAdequacy(result.coverage_adequacy || artifact.coverage_adequacy),
    splitInvalidReasons: normalizeSplitInvalidReasons(result.split_invalid_reasons || artifact.split_invalid_reasons),
    rankedResults: ranked.map((entry) => normalizeTuple(entry)),
  }
}

function normalizeIntegerList(value: unknown): number[] {
  if (!Array.isArray(value)) return []
  return value
    .map((entry) => Math.floor(normalizeNumber(entry)))
    .filter((entry) => Number.isFinite(entry))
}

function normalizeFloatList(value: unknown): number[] {
  if (!Array.isArray(value)) return []
  return value
    .map((entry) => normalizeNumber(entry))
    .filter((entry) => Number.isFinite(entry))
}

function normalizeFilters(raw: unknown): OptionsStaticFilterBundleSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    maxEntryLagMinutes: normalizeOptionalNumber(row.max_entry_lag_minutes),
    maxPremium: normalizeOptionalNumber(row.max_premium),
    minPremium: normalizeOptionalNumber(row.min_premium),
    minVolume: normalizeOptionalNumber(row.min_volume),
    minOpenInterest: normalizeOptionalNumber(row.min_open_interest),
  }
}

function normalizeRequest(raw: unknown): OptionsExpressionRequestSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    parentArtifactPath: normalizeText(row.parent_artifact_path),
    parentCandidateIndex: normalizeOptionalNumber(row.parent_candidate_index),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    structureFamily: normalizeText(row.structure_family),
    backendPreference: normalizeText(row.backend_preference),
    splitAssignmentMode: normalizeText(row.split_assignment_mode) || 'event_chronological',
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
  }
}

function normalizeOptionsExpressionArtifactSummary(raw: unknown): OptionsExpressionArtifactSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    ts: normalizeText(row.ts),
    path: normalizeText(row.path),
    bytes: normalizeNumber(row.bytes),
    parentArtifactId: normalizeText(row.parent_artifact_id),
    parentStrategyKind: normalizeText(row.parent_strategy_kind),
    rootSymbol: normalizeText(row.root_symbol),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    backendUsed: normalizeText(row.backend_used),
    backendFallbackReason: normalizeText(row.backend_fallback_reason),
    priceSemantics: normalizeText(row.price_semantics),
    requestBackendPreference: normalizeText(row.request_backend_preference),
    requestFillModel: normalizeText(row.request_fill_model),
    requestStructureFamily: normalizeText(row.request_structure_family),
    requestSplitAssignmentMode: normalizeText(row.request_split_assignment_mode),
    coverageAdequacyVerdict: (normalizeText(row.coverage_adequacy_verdict) as OptionsExpressionCoverageAdequacyVerdict) || 'unknown',
    coverageNonPromotable: Boolean(row.coverage_non_promotable),
    searchOverlapRate: normalizeNumber(row.search_overlap_rate),
    holdoutOverlapRate: normalizeNumber(row.holdout_overlap_rate),
    sealedTestOverlapRate: normalizeNumber(row.sealed_test_overlap_rate),
    searchCandidateCoverageRate: normalizeNumber(row.search_candidate_coverage_rate),
    holdoutCandidateCoverageRate: normalizeNumber(row.holdout_candidate_coverage_rate),
    sealedTestCandidateCoverageRate: normalizeNumber(row.sealed_test_candidate_coverage_rate),
    searchBaselineOutcomeCoverageRate: normalizeNumber(row.search_baseline_outcome_coverage_rate),
    holdoutBaselineOutcomeCoverageRate: normalizeNumber(row.holdout_baseline_outcome_coverage_rate),
    sealedTestBaselineOutcomeCoverageRate: normalizeNumber(row.sealed_test_baseline_outcome_coverage_rate),
    tupleCount: normalizeNumber(row.tuple_count),
    bestRobustScore: normalizeNumber(row.best_robust_score),
    bestSelectionExpectancyPerPremium: normalizeNumber(row.best_selection_expectancy_per_premium),
    bestHoldoutExpectancyPerPremium: normalizeOptionalNumber(row.best_holdout_expectancy_per_premium),
    bestSealedExpectancyPerPremium: normalizeOptionalNumber(row.best_sealed_expectancy_per_premium),
    bestStressExpectancyPerPremium: normalizeOptionalNumber(row.best_stress_expectancy_per_premium),
    bestSealedStressExpectancyPerPremium: normalizeOptionalNumber(row.best_sealed_stress_expectancy_per_premium),
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
    topValidationVerdict: normalizeValidationVerdict(row.top_validation_verdict),
    sealedFinalistBudget: normalizeNumber(row.sealed_finalist_budget),
    sealedEvaluatedTupleCount: normalizeNumber(row.sealed_evaluated_tuple_count),
    validatedTupleCount: normalizeNumber(row.validated_tuple_count),
    stressFragileTupleCount: normalizeNumber(row.stress_fragile_tuple_count),
    hasHoldoutMetrics: Boolean(row.has_holdout_metrics),
    hasSealedEvaluationMetrics: Boolean(row.has_sealed_evaluation_metrics),
    hasStressMetrics: Boolean(row.has_stress_metrics),
    hasHarsherStressMetrics: Boolean(row.has_harsher_stress_metrics),
    hasExtremeStressMetrics: Boolean(row.has_extreme_stress_metrics),
  }
}

function normalizeOptionsExpressionArtifact(raw: unknown): OptionsExpressionArtifactRecord {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const metrics = Array.isArray(row.selection_metrics) ? row.selection_metrics : []
  return {
    version: normalizeNumber(row.version),
    kind: normalizeText(row.kind),
    ts: normalizeText(row.ts),
    artifactPath: normalizeText(row.artifact_path),
    parentArtifactId: normalizeText(row.parent_artifact_id),
    rootSymbol: normalizeText(row.root_symbol),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    optionsDatasetHash: normalizeText(row.options_dataset_hash),
    backendUsed: normalizeText(row.backend_used),
    backendFallbackReason: normalizeText(row.backend_fallback_reason),
    priceSemantics: normalizeText(row.price_semantics),
    request: normalizeRequest(row.request),
    diagnostics: normalizeDiagnostics(row.diagnostics),
    staging: normalizeStaging(row.staging),
    overlapWindow: normalizeOverlapWindow(row.overlap_window),
    coverageAdequacy: normalizeCoverageAdequacy(row.coverage_adequacy),
    splitInvalidReasons: normalizeSplitInvalidReasons(row.split_invalid_reasons),
    selectionMetrics: metrics.map((entry) => normalizeTuple(entry)),
  }
}

function normalizeInspectionRow(raw: unknown): OptionsExpressionEventInspectionRow {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
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
    companionEntryOptionPrice: normalizeOptionalNumber(row.companion_entry_option_price),
    entryUnderlierPrice: normalizeOptionalNumber(row.entry_underlier_price),
    exitTimestamp: normalizeOptionalNumber(row.exit_timestamp),
    exitOptionPrice: normalizeOptionalNumber(row.exit_option_price),
    companionExitOptionPrice: normalizeOptionalNumber(row.companion_exit_option_price),
    premiumPaid: normalizeOptionalNumber(row.premium_paid),
    pnlPerContract: normalizeOptionalNumber(row.pnl_per_contract),
    returnOnPremium: normalizeOptionalNumber(row.return_on_premium),
    entryLagMinutes: normalizeOptionalNumber(row.entry_lag_minutes),
    minutesToExpiryAtEntry: normalizeOptionalNumber(row.minutes_to_expiry_at_entry),
    realizedHoldMinutes: normalizeOptionalNumber(row.realized_hold_minutes),
    pathOptionHigh: normalizeOptionalNumber(row.path_option_high),
    pathOptionLow: normalizeOptionalNumber(row.path_option_low),
    pathUnderlierHigh: normalizeOptionalNumber(row.path_underlier_high),
    pathUnderlierLow: normalizeOptionalNumber(row.path_underlier_low),
    exitReason: normalizeText(row.exit_reason),
  }
}

function normalizeTupleInspection(raw: unknown): OptionsExpressionTupleInspectionSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const rows = Array.isArray(row.rows) ? row.rows : []
  return {
    tupleIndex: normalizeNumber(row.tuple_index),
    tupleCount: normalizeNumber(row.tuple_count),
    tupleResult: normalizeTuple(row.tuple_result),
    totalEvents: normalizeNumber(row.total_events),
    executedEvents: normalizeNumber(row.executed_events),
    candidateMissingEvents: normalizeNumber(row.candidate_missing_events),
    outcomeMissingEvents: normalizeNumber(row.outcome_missing_events),
    rows: rows.map((entry) => normalizeInspectionRow(entry)),
  }
}

function normalizeEventPathPoint(raw: unknown): OptionsExpressionEventPathPointSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    timestampMs: normalizeNumber(row.timestamp_ms),
    optionOpen: normalizeNumber(row.option_open),
    optionHigh: normalizeNumber(row.option_high),
    optionLow: normalizeNumber(row.option_low),
    optionClose: normalizeNumber(row.option_close),
    underlyingPrice: normalizeNumber(row.underlying_price),
  }
}

function normalizeEventPathInspection(raw: unknown): OptionsExpressionEventPathInspectionSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const points = Array.isArray(row.points) ? row.points : []
  return {
    tupleIndex: normalizeNumber(row.tuple_index),
    eventIndex: normalizeNumber(row.event_index),
    eventId: normalizeText(row.event_id),
    row: normalizeInspectionRow(row.row),
    pointCount: normalizeNumber(row.point_count),
    points: points.map((entry) => normalizeEventPathPoint(entry)),
  }
}

function csvCell(value: unknown): string {
  const text = String(value ?? '')
  if (/[",\r\n]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`
  }
  return text
}

function downloadBlob(filename: string, blob: Blob): boolean {
  if (typeof document === 'undefined') return false
  if (typeof URL === 'undefined' || typeof URL.createObjectURL !== 'function') return false
  const href = URL.createObjectURL(blob)
  try {
    const link = document.createElement('a')
    link.href = href
    link.download = filename
    link.style.display = 'none'
    document.body?.appendChild(link)
    link.click()
    link.remove()
    return true
  } finally {
    URL.revokeObjectURL(href)
  }
}

function downloadText(filename: string, text: string, mime: string): boolean {
  try {
    return downloadBlob(filename, new Blob([text], { type: mime }))
  } catch {
    return false
  }
}

function csvByteLength(text: string): number {
  if (typeof TextEncoder !== 'undefined') {
    return new TextEncoder().encode(text).length
  }
  return text.length
}

export async function importOptionsDataset(req: Record<string, unknown>): Promise<OptionsDatasetManifestSummary> {
  const raw = await invokeCommand('import_options_dataset', { req })
  return normalizeOptionsDatasetManifest(raw)
}

export async function downloadPolygonOptionsDataset(
  req: PolygonOptionsDatasetDownloadRequestSummary,
): Promise<PolygonOptionsDatasetDownloadSummary> {
  const commandReq: Record<string, unknown> = {
    root_symbol: normalizeText(req.rootSymbol).toUpperCase(),
    from: normalizeText(req.from),
    to: normalizeText(req.to),
  }
  const optionsDatasetId = normalizeText(req.optionsDatasetId)
  if (optionsDatasetId) commandReq.options_dataset_id = optionsDatasetId
  const contractType = normalizeText(req.contractType)
  if (contractType) commandReq.contract_type = contractType
  const expirationDateFrom = normalizeText(req.expirationDateFrom)
  if (expirationDateFrom) commandReq.expiration_date_from = expirationDateFrom
  const expirationDateTo = normalizeText(req.expirationDateTo)
  if (expirationDateTo) commandReq.expiration_date_to = expirationDateTo
  if (req.maxDteDays != null) commandReq.max_dte_days = Math.max(0, Math.floor(normalizeNumber(req.maxDteDays)))
  if (req.strikeMin != null) commandReq.strike_min = normalizeNumber(req.strikeMin)
  if (req.strikeMax != null) commandReq.strike_max = normalizeNumber(req.strikeMax)
  if (req.maxContracts != null) commandReq.max_contracts = Math.max(1, Math.floor(normalizeNumber(req.maxContracts)))
  const providerEntitlementProfile = normalizeText(req.providerEntitlementProfile)
  if (providerEntitlementProfile) commandReq.provider_entitlement_profile = providerEntitlementProfile
  const contractReferenceAsOfUtc = normalizeText(req.contractReferenceAsOfUtc)
  if (contractReferenceAsOfUtc) commandReq.contract_reference_as_of_utc = contractReferenceAsOfUtc

  const raw = await invokeCommand('download_polygon_options_dataset', { req: commandReq })
  return normalizePolygonOptionsDatasetDownload(raw)
}

export async function listOptionsDatasets(): Promise<OptionsDatasetManifestSummary[]> {
  const raw = await invokeCommand('list_options_datasets')
  return Array.isArray(raw) ? raw.map((entry) => normalizeOptionsDatasetManifest(entry)) : []
}

export async function loadOptionsDatasetManifest(optionsDatasetId: string): Promise<OptionsDatasetManifestSummary> {
  const raw = await invokeCommand('load_options_dataset_manifest', { optionsDatasetId: normalizeText(optionsDatasetId) })
  return normalizeOptionsDatasetManifest(raw)
}

export async function runOptionsExpression(req: Record<string, unknown>): Promise<RunOptionsExpressionSummary> {
  const raw = await invokeCommand('run_options_expression', { req })
  return normalizeRunOptionsExpression(raw)
}

export async function analyzeOptionsExpressionFeasibility(
  req: Record<string, unknown>,
): Promise<AnalyzeOptionsExpressionFeasibilitySummary> {
  const raw = await invokeCommand('analyze_options_expression_feasibility', { req })
  return normalizeFeasibilityResponse(raw)
}

export async function listOptionsExpressionArtifacts(): Promise<OptionsExpressionArtifactSummary[]> {
  const raw = await invokeCommand('list_artifact_library')
  const library = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const rows = Array.isArray(library.optionsExpression)
    ? library.optionsExpression
    : Array.isArray(library.options_expression)
      ? library.options_expression
      : []
  return rows.map((entry) => normalizeOptionsExpressionArtifactSummary(entry))
}

export async function loadOptionsExpressionArtifact(path: string): Promise<OptionsExpressionArtifactRecord> {
  const raw = await invokeCommand('load_options_expression_artifact', { path: normalizeText(path) })
  return normalizeOptionsExpressionArtifact(raw)
}

export async function inspectOptionsExpressionTuple(input: {
  artifactPath: string
  tupleIndex: number
}): Promise<OptionsExpressionTupleInspectionSummary> {
  const raw = await invokeCommand('inspect_options_expression_tuple', {
    req: {
      artifact_path: normalizeText(input.artifactPath),
      tuple_index: Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))),
    },
  })
  return normalizeTupleInspection(raw)
}

export async function inspectOptionsExpressionEventPath(input: {
  artifactPath: string
  tupleIndex: number
  eventIndex: number
}): Promise<OptionsExpressionEventPathInspectionSummary> {
  const raw = await invokeCommand('inspect_options_expression_event_path', {
    req: {
      artifact_path: normalizeText(input.artifactPath),
      tuple_index: Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))),
      event_index: Math.max(0, Math.floor(normalizeNumber(input.eventIndex))),
    },
  })
  return normalizeEventPathInspection(raw)
}

export async function pickOptionsCsvPath(): Promise<string | null> {
  try {
    const raw = await invokeCommand<string | null>('pick_csv_file')
    const path = normalizeText(raw)
    return path.length > 0 ? path : null
  } catch {
    return null
  }
}

export async function openOrRevealOptionsPath(path: string): Promise<OpenOrRevealPathResult> {
  const normalized = normalizeText(path)
  if (!normalized) throw new Error('Path is required.')
  try {
    await invokeCommand('open_path', { path: normalized })
    return { path: normalized, openedWith: 'open_path' }
  } catch {
    await invokeCommand('reveal_path', { path: normalized })
    return { path: normalized, openedWith: 'reveal_path' }
  }
}

export function normalizeOptionsDatasetIds(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return normalizeStringList(value)
}

function buildOptionsExpressionCsvText(input: OptionsExpressionCsvExportInput): { filename: string, text: string } | null {
  const tuples = Array.isArray(input.tuples) ? input.tuples : []
  if (tuples.length === 0) return null
  const stem = normalizeText(input.filenameStem)
    || `vectorgrid_options_expression_${normalizeText(input.rootSymbol).toLowerCase() || 'results'}`
  const header = [
    'delay_minutes',
    'dte_days',
    'structure_family',
    'moneyness_bucket',
    'hold_minutes',
    'target_return',
    'stop_return',
    'exit_template',
    'fill_model',
    'pre_sealed_rank',
    'sealed_finalist',
    'sealed_final_rank',
    'validation_verdict',
    'robust_score',
    'selection_valid_trades',
    'selection_invalid_events',
    'selection_invalid_rate',
    'selection_expectancy_per_trade',
    'selection_expectancy_per_premium',
    'selection_median_return_on_premium',
    'selection_win_rate',
    'selection_total_pnl_per_contract',
    'holdout_expectancy_per_premium',
    'sealed_expectancy_per_premium',
    'stress_expectancy_per_premium',
    'sealed_stress_expectancy_per_premium',
    'harsher_stress_expectancy_per_premium',
    'sealed_harsher_stress_expectancy_per_premium',
    'extreme_stress_expectancy_per_premium',
    'sealed_extreme_stress_expectancy_per_premium',
    'backend_used',
    'price_semantics',
  ]
  const lines = [
    header.join(','),
    ...tuples.map((row) => [
      row.delayMinutes,
      row.dteDays,
      row.structureFamily,
      row.moneynessBucket,
      row.holdMinutes,
      row.targetReturn ?? '',
      row.stopReturn ?? '',
      row.exitTemplate,
      row.fillModel,
      row.preSealedRank,
      row.sealedFinalist ? 'yes' : '',
      row.sealedFinalRank ?? '',
      row.validationVerdict ?? 'search_only',
      row.robustScore,
      row.selectionMetrics.validTrades,
      row.selectionMetrics.invalidEvents,
      row.selectionMetrics.invalidRate,
      row.selectionMetrics.expectancyPerTrade,
      row.selectionMetrics.expectancyPerPremium,
      row.selectionMetrics.medianReturnOnPremium,
      row.selectionMetrics.winRate,
      row.selectionMetrics.totalPnlPerContract,
      row.holdoutMetrics?.expectancyPerPremium ?? '',
      row.sealedEvaluationMetrics?.expectancyPerPremium ?? '',
      row.stressMetrics?.expectancyPerPremium ?? '',
      row.sealedStressMetrics?.expectancyPerPremium ?? '',
      row.harsherStressMetrics?.expectancyPerPremium ?? '',
      row.sealedHarsherStressMetrics?.expectancyPerPremium ?? '',
      row.extremeStressMetrics?.expectancyPerPremium ?? '',
      row.sealedExtremeStressMetrics?.expectancyPerPremium ?? '',
      normalizeText(input.backendUsed),
      normalizeText(input.priceSemantics),
    ].map((value) => csvCell(value)).join(',')),
  ]
  return {
    filename: `${stem}.csv`,
    text: lines.join('\r\n'),
  }
}

function buildOptionsExpressionInspectionCsvText(
  input: OptionsExpressionInspectionCsvExportInput,
): { filename: string, text: string } | null {
  const rows = Array.isArray(input.rows) ? input.rows : []
  if (rows.length === 0) return null
  const stem = normalizeText(input.filenameStem)
    || `vectorgrid_options_expression_tuple_${Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))) + 1}`
  const header = [
    'event_index',
    'event_id',
    'split',
    'side',
    'status',
    'candidate_invalid_reason',
    'decision_timestamp',
    'earliest_tradable_timestamp',
    'requested_entry_timestamp',
    'raw_underlier_price',
    'base_exit_timestamp',
    'entry_timestamp',
    'entry_lag_minutes',
    'minutes_to_expiry_at_entry',
    'realized_hold_minutes',
    'contract_id',
    'companion_contract_id',
    'expiration_date',
    'strike',
    'companion_strike',
    'multiplier',
    'entry_option_price',
    'companion_entry_option_price',
    'entry_underlier_price',
    'exit_timestamp',
    'exit_option_price',
    'companion_exit_option_price',
    'exit_reason',
    'premium_paid',
    'pnl_per_contract',
    'return_on_premium',
    'path_option_low',
    'path_option_high',
    'path_underlier_low',
    'path_underlier_high',
    'artifact_path',
  ]
  const lines = [
    header.join(','),
    ...rows.map((row) => [
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
      row.baseExitTimestamp ?? '',
      row.entryTimestamp ?? '',
      row.entryLagMinutes ?? '',
      row.minutesToExpiryAtEntry ?? '',
      row.realizedHoldMinutes ?? '',
      row.contractId,
      row.companionContractId,
      row.expirationDate,
      row.strike ?? '',
      row.companionStrike ?? '',
      row.multiplier ?? '',
      row.entryOptionPrice ?? '',
      row.companionEntryOptionPrice ?? '',
      row.entryUnderlierPrice ?? '',
      row.exitTimestamp ?? '',
      row.exitOptionPrice ?? '',
      row.companionExitOptionPrice ?? '',
      row.exitReason,
      row.premiumPaid ?? '',
      row.pnlPerContract ?? '',
      row.returnOnPremium ?? '',
      row.pathOptionLow ?? '',
      row.pathOptionHigh ?? '',
      row.pathUnderlierLow ?? '',
      row.pathUnderlierHigh ?? '',
      normalizeText(input.artifactPath),
    ].map((value) => csvCell(value)).join(',')),
  ]
  return {
    filename: `${stem}.csv`,
    text: lines.join('\r\n'),
  }
}

function buildOptionsExpressionEventPathCsvText(
  input: OptionsExpressionEventPathCsvExportInput,
): { filename: string, text: string } | null {
  const points = Array.isArray(input.points) ? input.points : []
  if (points.length === 0) return null
  const stem = normalizeText(input.filenameStem)
    || `vectorgrid_options_expression_tuple_${Math.max(0, Math.floor(normalizeNumber(input.tupleIndex))) + 1}_event_path`
  const header = [
    'timestamp_ms',
    'option_open',
    'option_high',
    'option_low',
    'option_close',
    'underlying_price',
    'artifact_path',
    'tuple_index',
    'event_id',
  ]
  const lines = [
    header.join(','),
    ...points.map((point) => [
      point.timestampMs,
      point.optionOpen,
      point.optionHigh,
      point.optionLow,
      point.optionClose,
      point.underlyingPrice,
      normalizeText(input.artifactPath),
      input.tupleIndex == null ? '' : input.tupleIndex,
      normalizeText(input.eventId),
    ].map((value) => csvCell(value)).join(',')),
  ]
  return {
    filename: `${stem}.csv`,
    text: lines.join('\r\n'),
  }
}

async function saveOptionsCsvTextDesktop(
  filename: string,
  text: string,
): Promise<OptionsExpressionCsvExportResult> {
  const bytes = csvByteLength(text)
  let pickedPath: string | null
  try {
    const raw = await invokeCommand<string | null>('pick_save_csv', { defaultName: filename })
    pickedPath = normalizeText(raw) || null
  } catch {
    const downloaded = downloadText(filename, text, 'text/csv;charset=utf-8')
    return {
      mode: downloaded ? 'downloaded' : 'unavailable',
      filename,
      path: null,
      bytes,
      revealed: false,
    }
  }

  if (!pickedPath) {
    return {
      mode: 'cancelled',
      filename,
      path: null,
      bytes,
      revealed: false,
    }
  }

  const raw = await invokeCommand<{ path?: string, bytes?: number }>('save_options_csv', {
    req: {
      path: pickedPath,
      csv_text: text,
    },
  })
  const savedPath = normalizeText(raw?.path) || pickedPath
  let revealed = false
  try {
    await invokeCommand('reveal_path', { path: savedPath })
    revealed = true
  } catch {
    revealed = false
  }
  return {
    mode: 'saved',
    filename,
    path: savedPath,
    bytes: normalizeNumber(raw?.bytes, bytes),
    revealed,
  }
}

export async function exportOptionsExpressionCsv(
  input: OptionsExpressionCsvExportInput,
): Promise<OptionsExpressionCsvExportResult> {
  const built = buildOptionsExpressionCsvText(input)
  if (!built) {
    return {
      mode: 'unavailable',
      filename: '',
      path: null,
      bytes: 0,
      revealed: false,
    }
  }
  return saveOptionsCsvTextDesktop(built.filename, built.text)
}

export async function exportOptionsExpressionInspectionCsv(
  input: OptionsExpressionInspectionCsvExportInput,
): Promise<OptionsExpressionCsvExportResult> {
  const built = buildOptionsExpressionInspectionCsvText(input)
  if (!built) {
    return {
      mode: 'unavailable',
      filename: '',
      path: null,
      bytes: 0,
      revealed: false,
    }
  }
  return saveOptionsCsvTextDesktop(built.filename, built.text)
}

export async function exportOptionsExpressionEventPathCsv(
  input: OptionsExpressionEventPathCsvExportInput,
): Promise<OptionsExpressionCsvExportResult> {
  const built = buildOptionsExpressionEventPathCsvText(input)
  if (!built) {
    return {
      mode: 'unavailable',
      filename: '',
      path: null,
      bytes: 0,
      revealed: false,
    }
  }
  return saveOptionsCsvTextDesktop(built.filename, built.text)
}
