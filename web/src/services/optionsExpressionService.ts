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
}

export interface OptionsExpressionTupleSummary {
  delayMinutes: number
  dteDays: number
  moneynessBucket: string
  holdMinutes: number
  targetReturn: number | null
  stopReturn: number | null
  exitTemplate: string
  fillModel: string
  robustScore: number
  selectionMetrics: OptionsExpressionMetricsSummary
  holdoutMetrics: OptionsExpressionMetricsSummary | null
  sealedEvaluationMetrics: OptionsExpressionMetricsSummary | null
  stressMetrics: OptionsExpressionMetricsSummary | null
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
}

export interface OptionsCandidateInvalidReasonCountSummary {
  reason: string
  count: number
}

export interface OptionsExpressionFeasibilityTimingSummary {
  tupleCompileMs: number
  baselineCandidateCompileMs: number
  baselineExitSummaryCompileMs: number
  conservativeCandidateCompileMs: number
  conservativeExitSummaryCompileMs: number
  cpuRankingMs: number
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
}

export interface AnalyzeOptionsExpressionFeasibilitySummary {
  replayBuildMs: number
  totalPreviewMs: number
  diagnostics: OptionsExpressionDiagnosticsSummary
  priceSemantics: string
  fillModel: string
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
  projectedEventTupleEvaluations: number
  structuralCacheBytesEstimate: number
  baselineSummaryBytesEstimate: number
  conservativeSummaryBytesEstimate: number
  timings: OptionsExpressionFeasibilityTimingSummary
  eventCoverage: OptionsExpressionFeasibilityEventCoverageSummary
  invalidReasons: OptionsCandidateInvalidReasonCountSummary[]
}

export interface RunOptionsExpressionSummary {
  artifactPath: string
  artifactTs: string
  parentArtifactId: string
  optionsDatasetHash: string
  backendUsed: string
  priceSemantics: string
  tupleCount: number
  request: OptionsExpressionRequestSummary | null
  diagnostics: OptionsExpressionDiagnosticsSummary
  staging: OptionsExpressionStagingSummary | null
  rankedResults: OptionsExpressionTupleSummary[]
}

export interface OptionsExpressionArtifactSummary {
  ts: string
  path: string
  bytes: number
  parentArtifactId: string
  rootSymbol: string
  optionsDatasetId: string
  backendUsed: string
  priceSemantics: string
  tupleCount: number
  bestRobustScore: number
  bestSelectionExpectancyPerPremium: number
  hasHoldoutMetrics: boolean
  hasSealedEvaluationMetrics: boolean
  hasStressMetrics: boolean
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
  priceSemantics: string
  request: OptionsExpressionRequestSummary
  diagnostics: OptionsExpressionDiagnosticsSummary
  staging: OptionsExpressionStagingSummary | null
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
  contractId: string
  expirationDate: string
  strike: number | null
  multiplier: number | null
  entryTimestamp: number | null
  entryOptionPrice: number | null
  entryUnderlierPrice: number | null
  exitTimestamp: number | null
  exitOptionPrice: number | null
  premiumPaid: number | null
  pnlPerContract: number | null
  returnOnPremium: number | null
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

function normalizeTuple(raw: unknown): OptionsExpressionTupleSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const tuple = (row.tuple && typeof row.tuple === 'object' ? row.tuple : {}) as Record<string, unknown>
  return {
    delayMinutes: normalizeNumber(tuple.delay_minutes),
    dteDays: normalizeNumber(tuple.dte_days),
    moneynessBucket: normalizeText(tuple.moneyness_bucket),
    holdMinutes: normalizeNumber(tuple.hold_minutes),
    targetReturn: normalizeOptionalNumber(tuple.target_return),
    stopReturn: normalizeOptionalNumber(tuple.stop_return),
    exitTemplate: normalizeText(tuple.exit_template),
    fillModel: normalizeText(tuple.fill_model),
    robustScore: normalizeNumber(row.robust_score),
    selectionMetrics: normalizeMetrics(row.selection_metrics),
    holdoutMetrics: row.holdout_metrics ? normalizeMetrics(row.holdout_metrics) : null,
    sealedEvaluationMetrics: row.sealed_evaluation_metrics
      ? normalizeMetrics(row.sealed_evaluation_metrics)
      : null,
    stressMetrics: row.stress_metrics ? normalizeMetrics(row.stress_metrics) : null,
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
    cpuRankingMs: normalizeNumber(row.cpu_ranking_ms),
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

function normalizeFeasibilityResponse(raw: unknown): AnalyzeOptionsExpressionFeasibilitySummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const report = (row.report && typeof row.report === 'object' ? row.report : {}) as Record<string, unknown>
  return {
    replayBuildMs: normalizeNumber(row.replay_build_ms),
    totalPreviewMs: normalizeNumber(row.total_preview_ms),
    diagnostics: normalizeDiagnostics(report.diagnostics),
    priceSemantics: normalizeText(report.price_semantics),
    fillModel: normalizeText(report.fill_model),
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
    projectedEventTupleEvaluations: normalizeNumber(report.projected_event_tuple_evaluations),
    structuralCacheBytesEstimate: normalizeNumber(report.structural_cache_bytes_estimate),
    baselineSummaryBytesEstimate: normalizeNumber(report.baseline_summary_bytes_estimate),
    conservativeSummaryBytesEstimate: normalizeNumber(report.conservative_summary_bytes_estimate),
    timings: normalizeFeasibilityTiming(report.timings),
    eventCoverage: normalizeFeasibilityEventCoverage(report.event_coverage),
    invalidReasons: normalizeInvalidReasonCounts(report.invalid_reasons),
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
    priceSemantics: normalizeText(result.price_semantics),
    tupleCount: normalizeNumber(result.tuple_count),
    request: Object.keys(artifact).length > 0 ? normalizeRequest(artifact.request) : null,
    diagnostics: normalizeDiagnostics(result.diagnostics),
    staging: normalizeStaging(result.staging),
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
  }
}

function normalizeOptionsExpressionArtifactSummary(raw: unknown): OptionsExpressionArtifactSummary {
  const row = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    ts: normalizeText(row.ts),
    path: normalizeText(row.path),
    bytes: normalizeNumber(row.bytes),
    parentArtifactId: normalizeText(row.parent_artifact_id),
    rootSymbol: normalizeText(row.root_symbol),
    optionsDatasetId: normalizeText(row.options_dataset_id),
    backendUsed: normalizeText(row.backend_used),
    priceSemantics: normalizeText(row.price_semantics),
    tupleCount: normalizeNumber(row.tuple_count),
    bestRobustScore: normalizeNumber(row.best_robust_score),
    bestSelectionExpectancyPerPremium: normalizeNumber(row.best_selection_expectancy_per_premium),
    hasHoldoutMetrics: Boolean(row.has_holdout_metrics),
    hasSealedEvaluationMetrics: Boolean(row.has_sealed_evaluation_metrics),
    hasStressMetrics: Boolean(row.has_stress_metrics),
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
    priceSemantics: normalizeText(row.price_semantics),
    request: normalizeRequest(row.request),
    diagnostics: normalizeDiagnostics(row.diagnostics),
    staging: normalizeStaging(row.staging),
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
    contractId: normalizeText(row.contract_id),
    expirationDate: normalizeText(row.expiration_date),
    strike: normalizeOptionalNumber(row.strike),
    multiplier: normalizeOptionalNumber(row.multiplier),
    entryTimestamp: normalizeOptionalNumber(row.entry_timestamp),
    entryOptionPrice: normalizeOptionalNumber(row.entry_option_price),
    entryUnderlierPrice: normalizeOptionalNumber(row.entry_underlier_price),
    exitTimestamp: normalizeOptionalNumber(row.exit_timestamp),
    exitOptionPrice: normalizeOptionalNumber(row.exit_option_price),
    premiumPaid: normalizeOptionalNumber(row.premium_paid),
    pnlPerContract: normalizeOptionalNumber(row.pnl_per_contract),
    returnOnPremium: normalizeOptionalNumber(row.return_on_premium),
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

export async function importOptionsDataset(req: Record<string, unknown>): Promise<OptionsDatasetManifestSummary> {
  const raw = await invokeCommand('import_options_dataset', { req })
  return normalizeOptionsDatasetManifest(raw)
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
  const raw = await invokeCommand('list_options_expression_artifacts')
  return Array.isArray(raw) ? raw.map((entry) => normalizeOptionsExpressionArtifactSummary(entry)) : []
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

export function exportOptionsExpressionCsv(input: OptionsExpressionCsvExportInput): boolean {
  const tuples = Array.isArray(input.tuples) ? input.tuples : []
  if (tuples.length === 0) return false
  const stem = normalizeText(input.filenameStem)
    || `vectorgrid_options_expression_${normalizeText(input.rootSymbol).toLowerCase() || 'results'}`
  const header = [
    'delay_minutes',
    'dte_days',
    'moneyness_bucket',
    'hold_minutes',
    'target_return',
    'stop_return',
    'exit_template',
    'fill_model',
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
    'backend_used',
    'price_semantics',
  ]
  const lines = [
    header.join(','),
    ...tuples.map((row) => [
      row.delayMinutes,
      row.dteDays,
      row.moneynessBucket,
      row.holdMinutes,
      row.targetReturn ?? '',
      row.stopReturn ?? '',
      row.exitTemplate,
      row.fillModel,
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
      normalizeText(input.backendUsed),
      normalizeText(input.priceSemantics),
    ].map((value) => csvCell(value)).join(',')),
  ]
  return downloadText(`${stem}.csv`, lines.join('\r\n'), 'text/csv;charset=utf-8')
}
