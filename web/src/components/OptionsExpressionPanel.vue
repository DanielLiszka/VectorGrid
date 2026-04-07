<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import {
  useOptionsExpressionStore,
  OPTIONS_BACKEND_PREFERENCE_OPTIONS,
  OPTIONS_EXIT_TEMPLATE_OPTIONS,
  OPTIONS_FILL_MODEL_OPTIONS,
  OPTIONS_MONEYNESS_BUCKET_OPTIONS,
  OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS,
} from '../stores/optionsExpressionStore'

const props = defineProps<{
  parentArtifactPath: string
  parentStrategyKind: string
  parentDatasetLabel: string
}>()

const store = useOptionsExpressionStore()
const {
  activeArtifactPath,
  activeArtifactTimestamp,
  activeBackendUsed,
  activeBackendFallbackReason,
  activeEventPathInspection,
  activePriceSemantics,
  datasets,
  datasetsBusy,
  eventPathBusy,
  artifactsBusy,
  downloadBusy,
  feasibilityBusy,
  filteredArtifacts,
  importBusy,
  importRootSymbol,
  downloadDatasetIdInput,
  downloadFromDateInput,
  downloadToDateInput,
  downloadContractType,
  downloadExpirationFromDateInput,
  downloadExpirationToDateInput,
  downloadMaxDteDaysInput,
  downloadStrikeMinInput,
  downloadStrikeMaxInput,
  downloadMaxContractsInput,
  loadBusy,
  latestFeasibility,
  latestDatasetDownload,
  parentRootHint,
  runBusy,
  selectedArtifactPath,
  selectedDataset,
  selectedDatasetId,
  statusLine,
  supportedParent,
  activeDiagnostics,
  activeTupleInspection,
  activeOptionsDatasetHash,
  activeRequest,
  activeStaging,
  activeRankedResults,
  inspectionBusy,
  delayMinutesInput,
  dteBucketsInput,
  holdMinutesInput,
  targetReturnInput,
  stopReturnInput,
  selectedMoneynessBuckets,
  exitTemplate,
  fillModel,
  backendPreference,
  splitAssignmentMode,
  topKInput,
  sealedFinalistCountInput,
  minValidTradesInput,
  splitRatioSearchInput,
  splitRatioHoldoutInput,
  maxEntryLagMinutesInput,
  maxPremiumInput,
  minPremiumInput,
  minVolumeInput,
  minOpenInterestInput,
} = storeToRefs(store)

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function formatList(value: Array<number | string> | null | undefined): string {
  return Array.isArray(value) && value.length > 0 ? value.join(', ') : '—'
}

function formatTimestamp(value: number | null | undefined): string {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed <= 0) return '—'
  try {
    return new Date(parsed).toISOString()
  } catch {
    return '—'
  }
}

function formatOptionalDecimal(value: number | null | undefined, digits = 4): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : '—'
}

function formatPercent(value: number | null | undefined, digits = 1): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? `${(parsed * 100).toFixed(digits)}%` : '—'
}

function formatOptionalMinutes(value: number | null | undefined): string {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? `${parsed.toFixed(0)}m` : '—'
}

function formatLatency(ms: number | null | undefined, us: number | null | undefined): string {
  const micros = Number(us)
  if (Number.isFinite(micros) && micros > 0 && micros < 10_000) {
    return `${(micros / 1_000).toFixed(3)} ms`
  }
  const millis = Number(ms)
  return Number.isFinite(millis) ? `${millis.toFixed(0)} ms` : '—'
}

function formatSplitAssignmentMode(value: string | null | undefined): string {
  switch (normalizeText(value)) {
    case 'session_chronological':
      return 'session_chronological'
    case 'event_chronological':
    default:
      return 'event_chronological'
  }
}

function formatBytes(value: number | null | undefined): string {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed < 0) return '—'
  if (parsed >= 1024 * 1024) return `${(parsed / (1024 * 1024)).toFixed(2)} MiB`
  if (parsed >= 1024) return `${(parsed / 1024).toFixed(1)} KiB`
  return `${parsed.toFixed(0)} B`
}

function pushUniqueRecommendation(target: string[], message: string): void {
  if (!target.includes(message)) {
    target.push(message)
  }
}

function formatBucketInvalidReasons(
  reasons: Array<{ reason: string; count: number }> | null | undefined,
): string {
  if (!Array.isArray(reasons) || reasons.length === 0) return '—'
  return reasons
    .slice(0, 2)
    .map((entry) => `${normalizeText(entry.reason)} (${entry.count})`)
    .join(' | ')
}

function formatMoneynessBucketLabel(value: string | null | undefined): string {
  const normalized = normalizeText(value)
  return normalized.length > 0 ? normalized.split('_').join(' ') : '—'
}

watch(
  () => [props.parentArtifactPath, props.parentStrategyKind, props.parentDatasetLabel] as const,
  ([path, strategyKind, datasetLabel]) => {
    store.attachParent(path, strategyKind, datasetLabel)
    if (normalizeText(path) && normalizeText(strategyKind) === 'DoubleMa') {
      void store.bootstrap()
    }
  },
  { immediate: true },
)

const hasParentSelection = computed(() => normalizeText(props.parentArtifactPath).length > 0)

const datasetSummary = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return ''
  return [
    selected.rootSymbol || 'unknown root',
    `${selected.contractCount} contracts`,
    `${selected.rowCount} rows`,
    selected.priceSemantics || 'unknown semantics',
  ].join(' | ')
})

const datasetProvenanceSummary = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return ''
  const segments = [
    normalizeText(selected.providerName),
    normalizeText(selected.providerSourceSurface),
    normalizeText(selected.quoteResolution),
    normalizeText(selected.aggregatePriceField),
    normalizeText(selected.contractReferenceAsOfUtc),
  ].filter((entry) => entry.length > 0)
  return segments.join(' | ')
})

const selectedDatasetDetailRows = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return []
  return [
    { key: 'coverage', label: 'Coverage', value: `${formatTimestamp(selected.coverageStart)} → ${formatTimestamp(selected.coverageEnd)}` },
    { key: 'provider', label: 'Provider', value: `${normalizeText(selected.providerName)} · ${normalizeText(selected.providerSourceSurface)}` },
    { key: 'resolution', label: 'Resolution', value: `${normalizeText(selected.quoteResolution)} · ${normalizeText(selected.priceSemantics)} · ${normalizeText(selected.aggregatePriceField) || 'n/a'}` },
    { key: 'contracts', label: 'Contracts / Rows', value: `${selected.contractCount} / ${selected.rowCount}` },
    { key: 'calendar', label: 'Calendar / TZ', value: `${normalizeText(selected.calendarId)} · ${normalizeText(selected.timezone)}` },
    { key: 'contract-ref', label: 'Contract ref', value: `${normalizeText(selected.contractReferenceSource)} · ${normalizeText(selected.contractReferenceAsOfUtc) || 'n/a'}` },
    { key: 'underlier-ref', label: 'Underlier ref', value: normalizeText(selected.underlierReferenceSource) || '—' },
    { key: 'entitlement', label: 'Entitlement', value: normalizeText(selected.providerEntitlementProfile) || '—' },
    { key: 'dataset-path', label: 'Dataset path', value: normalizeText(selected.datasetPath) || '—' },
  ]
})

const latestDownloadAuditRows = computed(() => {
  const latestDownload = latestDatasetDownload.value
  if (!latestDownload) return []
  return [
    { key: 'download-contracts', label: 'Contracts discovered / selected / rows', value: `${latestDownload.contractsDiscovered} / ${latestDownload.contractsSelected} / ${latestDownload.contractsWithRows}` },
    { key: 'download-rows', label: 'Rows / skipped', value: `${latestDownload.downloadedOptionRows} / ${latestDownload.skippedContracts}` },
    { key: 'underlier-csv', label: 'Underlying CSV', value: normalizeText(latestDownload.underlyingCsvPath) || '—' },
    { key: 'underlier-cache', label: 'Underlying source', value: latestDownload.usedUnderlyingCacheOnly ? 'cache-only reuse' : 'download + cache reuse' },
  ]
})

const activeDatasetManifest = computed(() => {
  const requestDatasetId = normalizeText(activeRequest.value?.optionsDatasetId)
  const selectedId = normalizeText(selectedDataset.value?.optionsDatasetId)
  const targetId = requestDatasetId || selectedId
  if (!targetId) return selectedDataset.value
  return datasets.value.find((entry) => normalizeText(entry.optionsDatasetId) === targetId) || selectedDataset.value
})

const activeDatasetProvenanceSummary = computed(() => {
  const activeDataset = activeDatasetManifest.value
  if (!activeDataset) return ''
  return [
    normalizeText(activeDataset.providerName),
    normalizeText(activeDataset.providerSourceSurface),
    normalizeText(activeDataset.quoteResolution),
    normalizeText(activeDataset.aggregatePriceField),
    normalizeText(activeDataset.contractReferenceAsOfUtc),
  ]
    .filter((entry) => entry.length > 0)
    .join(' | ')
})

const rootMismatch = computed(() => {
  const selected = selectedDataset.value
  if (!selected) return false
  const selectedRoot = normalizeText(selected.rootSymbol).toUpperCase()
  const parentRoot = normalizeText(parentRootHint.value).toUpperCase()
  return !!selectedRoot && !!parentRoot && selectedRoot !== parentRoot
})

const activeRows = computed(() => activeRankedResults.value)
const hasHoldoutMetrics = computed(() => activeRows.value.some((row) => row.holdoutMetrics != null))
const hasSealedMetrics = computed(() => activeRows.value.some((row) => row.sealedEvaluationMetrics != null))
const hasStressMetrics = computed(() => activeRows.value.some((row) => row.stressMetrics != null))
const hasSealedStressMetrics = computed(() => activeRows.value.some((row) => row.sealedStressMetrics != null))
const hasHarsherStressMetrics = computed(() => activeRows.value.some((row) => row.harsherStressMetrics != null))
const hasSealedHarsherStressMetrics = computed(() => activeRows.value.some((row) => row.sealedHarsherStressMetrics != null))
const selectedTupleIndex = ref(0)
const inspectionSplitFilter = ref<'all' | 'search' | 'holdout' | 'sealed_test'>('all')
const showAllInspectionRows = ref(false)
const selectedInspectionEventId = ref('')

watch(
  activeRows,
  (rows) => {
    if (!rows.length) {
      selectedTupleIndex.value = 0
      store.clearTupleInspection()
      return
    }
    if (selectedTupleIndex.value >= rows.length) {
      selectedTupleIndex.value = 0
    }
  },
  { immediate: true },
)

watch(selectedTupleIndex, () => {
  showAllInspectionRows.value = false
  inspectionSplitFilter.value = 'all'
  selectedInspectionEventId.value = ''
  store.clearTupleInspection()
})

const selectedTuple = computed(() => activeRows.value[selectedTupleIndex.value] || null)
const selectedInspection = computed(() => {
  const inspection = activeTupleInspection.value
  if (!inspection) return null
  return inspection.tupleIndex === selectedTupleIndex.value ? inspection : null
})
const inspectionRows = computed(() => {
  const inspection = selectedInspection.value
  if (!inspection) return []
  const filter = inspectionSplitFilter.value
  return inspection.rows.filter((row) => filter === 'all' || row.split === filter)
})
const displayedInspectionRows = computed(() => {
  if (showAllInspectionRows.value) return inspectionRows.value
  return inspectionRows.value.slice(0, 50)
})
const selectedInspectionRow = computed(() => {
  const rows = inspectionRows.value
  if (!rows.length) return null
  const match = rows.find((row) => row.eventId === selectedInspectionEventId.value)
  return match || rows[0] || null
})
const selectedArtifactSummary = computed(() => {
  const nextPath = normalizeText(selectedArtifactPath.value)
  return filteredArtifacts.value.find((entry) => normalizeText(entry.path) === nextPath) || null
})

const artifactComparisonRows = computed(() =>
  [...filteredArtifacts.value].sort((left, right) => {
    const byRobust = (right.bestRobustScore ?? 0) - (left.bestRobustScore ?? 0)
    if (Math.abs(byRobust) > 1e-9) return byRobust
    return normalizeText(right.ts).localeCompare(normalizeText(left.ts))
  }),
)
const resultSortKey = ref<'robust' | 'selection_ev' | 'holdout_ev' | 'stress_ev' | 'harsher_ev' | 'invalid_rate' | 'valid_trades' | 'sealed_rank'>('robust')
const resultSortDirection = ref<'desc' | 'asc'>('desc')
const finalistsOnly = ref(false)
const maxInvalidRateFilterInput = ref('')
const minRobustScoreFilterInput = ref('')
const maxStressDropFilterInput = ref('')
const selectedEventPathInspection = computed(() => {
  const inspection = activeEventPathInspection.value
  const selectedRow = selectedInspectionRow.value
  if (!inspection || !selectedRow) return null
  if (inspection.tupleIndex !== selectedTupleIndex.value) return null
  return inspection.eventIndex === selectedRow.eventIndex ? inspection : null
})
const eventPathSummaryRows = computed(() => {
  const inspection = selectedEventPathInspection.value
  if (!inspection || inspection.points.length === 0) return []
  const first = inspection.points[0]
  const last = inspection.points[inspection.points.length - 1]
  const optionLow = inspection.points.reduce((acc, point) => Math.min(acc, point.optionLow), Number.POSITIVE_INFINITY)
  const optionHigh = inspection.points.reduce((acc, point) => Math.max(acc, point.optionHigh), Number.NEGATIVE_INFINITY)
  const underlierLow = inspection.points.reduce((acc, point) => Math.min(acc, point.underlyingPrice), Number.POSITIVE_INFINITY)
  const underlierHigh = inspection.points.reduce((acc, point) => Math.max(acc, point.underlyingPrice), Number.NEGATIVE_INFINITY)
  return [
    {
      key: 'path-points',
      label: 'Path points',
      value: `${inspection.pointCount}`,
    },
    {
      key: 'path-window',
      label: 'Window',
      value: `${formatTimestamp(first?.timestampMs)} -> ${formatTimestamp(last?.timestampMs)}`,
    },
    {
      key: 'option-range',
      label: 'Option range',
      value: `${formatOptionalDecimal(optionLow)} -> ${formatOptionalDecimal(optionHigh)}`,
    },
    {
      key: 'underlier-range',
      label: 'Underlier range',
      value: `${formatOptionalDecimal(underlierLow, 2)} -> ${formatOptionalDecimal(underlierHigh, 2)}`,
    },
    {
      key: 'option-close-delta',
      label: 'Option open -> close',
      value: `${formatOptionalDecimal(first?.optionOpen)} -> ${formatOptionalDecimal(last?.optionClose)}`,
    },
    {
      key: 'underlier-delta',
      label: 'Underlier start -> end',
      value: `${formatOptionalDecimal(first?.underlyingPrice, 2)} -> ${formatOptionalDecimal(last?.underlyingPrice, 2)}`,
    },
  ]
})

const displayedRowsWithIndex = computed(() => {
  const rawInvalidRate = normalizeText(maxInvalidRateFilterInput.value).length
    ? Number(maxInvalidRateFilterInput.value)
    : Number.POSITIVE_INFINITY
  const rawMinRobustScore = normalizeText(minRobustScoreFilterInput.value).length
    ? Number(minRobustScoreFilterInput.value)
    : Number.NEGATIVE_INFINITY
  const rawMaxStressDrop = normalizeText(maxStressDropFilterInput.value).length
    ? Number(maxStressDropFilterInput.value)
    : Number.POSITIVE_INFINITY
  const maxInvalidRate = Number.isFinite(rawInvalidRate) ? rawInvalidRate : Number.POSITIVE_INFINITY
  const minRobustScore = Number.isFinite(rawMinRobustScore)
    ? rawMinRobustScore
    : Number.NEGATIVE_INFINITY
  const maxStressDrop = Number.isFinite(rawMaxStressDrop) ? rawMaxStressDrop : Number.POSITIVE_INFINITY

  const rows = activeRows.value
    .map((row, originalIndex) => ({ row, originalIndex }))
    .filter(({ row }) => !finalistsOnly.value || row.sealedFinalist)
    .filter(({ row }) => row.selectionMetrics.invalidRate <= maxInvalidRate)
    .filter(({ row }) => row.robustScore >= minRobustScore)
    .filter(({ row }) => {
      const stressed = row.harsherStressMetrics?.expectancyPerPremium ?? row.stressMetrics?.expectancyPerPremium
      if (!Number.isFinite(maxStressDrop) || stressed == null) return true
      return row.selectionMetrics.expectancyPerPremium - stressed <= maxStressDrop
    })

  const metricFor = (entry: (typeof rows)[number]): number => {
    switch (resultSortKey.value) {
      case 'selection_ev':
        return entry.row.selectionMetrics.expectancyPerPremium
      case 'holdout_ev':
        return entry.row.holdoutMetrics?.expectancyPerPremium ?? Number.NEGATIVE_INFINITY
      case 'stress_ev':
        return entry.row.stressMetrics?.expectancyPerPremium ?? Number.NEGATIVE_INFINITY
      case 'harsher_ev':
        return entry.row.harsherStressMetrics?.expectancyPerPremium ?? Number.NEGATIVE_INFINITY
      case 'invalid_rate':
        return entry.row.selectionMetrics.invalidRate
      case 'valid_trades':
        return entry.row.selectionMetrics.validTrades
      case 'sealed_rank':
        return entry.row.sealedFinalRank ?? Number.POSITIVE_INFINITY
      case 'robust':
      default:
        return entry.row.robustScore
    }
  }

  rows.sort((left, right) => {
    const leftMetric = metricFor(left)
    const rightMetric = metricFor(right)
    if (leftMetric === rightMetric) return left.originalIndex - right.originalIndex
    return resultSortDirection.value === 'asc' ? leftMetric - rightMetric : rightMetric - leftMetric
  })
  return rows
})

watch(displayedRowsWithIndex, (rows) => {
  if (!rows.length) return
  if (!rows.some((entry) => entry.originalIndex === selectedTupleIndex.value)) {
    selectedTupleIndex.value = rows[0]?.originalIndex ?? 0
  }
})

watch(
  inspectionRows,
  (rows) => {
    if (!rows.length) {
      selectedInspectionEventId.value = ''
      store.clearEventPathInspection()
      return
    }
    if (!rows.some((row) => row.eventId === selectedInspectionEventId.value)) {
      selectedInspectionEventId.value = rows[0]?.eventId || ''
    }
  },
  { immediate: true },
)

watch(
  selectedInspectionRow,
  (row) => {
    if (!row) {
      store.clearEventPathInspection()
      return
    }
    if (row.status !== 'executed') {
      store.clearEventPathInspection()
      return
    }
    const existing = activeEventPathInspection.value
    if (existing && existing.tupleIndex === selectedTupleIndex.value && existing.eventIndex === row.eventIndex) {
      return
    }
    void store.loadEventPathInspection(selectedTupleIndex.value, row.eventIndex)
  },
  { immediate: true },
)

const resultSummaryChips = computed(() => {
  const diagnostics = activeDiagnostics.value
  if (!diagnostics) return []
  const chips = [
    `${diagnostics.totalEvents} events`,
    `${activeRankedResults.value.length} tuples`,
  ]
  if (activeBackendUsed.value) chips.push(activeBackendUsed.value)
  if (activePriceSemantics.value) chips.push(activePriceSemantics.value)
  if (diagnostics.holdoutEvents > 0) chips.push(`${diagnostics.holdoutEvents} holdout`)
  if (diagnostics.sealedTestEvents > 0) chips.push(`${diagnostics.sealedTestEvents} sealed`)
  if (activeStaging.value) {
    chips.push(`${activeStaging.value.compiledCandidateCount}/${activeStaging.value.candidateSlotCount} candidates`)
    chips.push(`${activeStaging.value.baselineExitSummaryCount} baseline summaries`)
    chips.push(`${activeStaging.value.conservativeExitSummaryCount} conservative summaries`)
    chips.push(`${activeStaging.value.harsherExitSummaryCount} harsher summaries`)
  }
  return chips
})

const reportingMetricCards = computed(() => {
  const diagnostics = activeDiagnostics.value
  if (!diagnostics) return []
  const cards = [
    {
      key: 'bull-bear',
      label: 'Bull / Bear',
      value: `${diagnostics.bullishEvents} / ${diagnostics.bearishEvents}`,
    },
    {
      key: 'search-holdout-sealed',
      label: 'Search / Holdout / Sealed',
      value: `${diagnostics.searchEvents} / ${diagnostics.holdoutEvents} / ${diagnostics.sealedTestEvents}`,
    },
  ]
  if (activeStaging.value && activeStaging.value.candidateSlotCount > 0) {
    cards.push({
      key: 'candidate-coverage',
      label: 'Candidates',
      value: `${activeStaging.value.compiledCandidateCount} / ${activeStaging.value.candidateSlotCount}`,
    })
  }
  if (activeStaging.value) {
    cards.push({
      key: 'exit-summaries',
      label: 'Exit summaries',
      value: `${activeStaging.value.baselineExitSummaryCount} / ${activeStaging.value.conservativeExitSummaryCount} / ${activeStaging.value.harsherExitSummaryCount}`,
    })
  }
  return cards
})

const feasibilitySummaryRows = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  const gpuRank = preview.timings.gpuRankingMs
  const gpuRankUs = preview.timings.gpuRankingUs
  const gpuTensor = preview.timings.gpuTensorScoreMs
  const gpuTensorUs = preview.timings.gpuTensorScoreUs
  const gpuExactify = preview.timings.gpuExactifyMs
  const gpuExactifyUs = preview.timings.gpuExactifyUs
  const gpuSealed = preview.timings.gpuSealedStageMs
  const gpuSealedUs = preview.timings.gpuSealedStageUs
  const gpuSpeedup = preview.timings.gpuSpeedupVsCpu
  const backendLine = gpuRank == null
    ? `${preview.bestAvailableBackend || 'cpu_reference'} · cpu ${formatLatency(preview.timings.cpuRankingMs, preview.timings.cpuRankingUs)}`
    : `${preview.bestAvailableBackend || 'gpu_compiled_tensor_hybrid'} · cpu ${formatLatency(preview.timings.cpuRankingMs, preview.timings.cpuRankingUs)} / gpu hybrid ${formatLatency(gpuRank, gpuRankUs)}${gpuSpeedup == null ? '' : ` (${gpuSpeedup.toFixed(2)}x)`}`
  const gpuBreakdownLine = gpuTensor == null
    ? '—'
    : `tensor ${formatLatency(gpuTensor, gpuTensorUs)} · exactify ${formatLatency(gpuExactify, gpuExactifyUs)} · sealed ${formatLatency(gpuSealed, gpuSealedUs)}`
  const gpuUploadReuseLine = `shared ${preview.cacheStats.gpuSharedInputCacheHits}/${preview.cacheStats.gpuSharedInputCacheMisses} · outcome ${preview.cacheStats.gpuOutcomeInputCacheHits}/${preview.cacheStats.gpuOutcomeInputCacheMisses} · out ${preview.cacheStats.gpuOutputBufferReuses}/${preview.cacheStats.gpuOutputBufferAllocations}`
  const gpuChunkingLine = preview.cacheStats.gpuTupleChunkCount > 0
    ? `chunks ${preview.cacheStats.gpuTupleChunkCount}×${preview.cacheStats.gpuTupleChunkSize} · launches ${preview.cacheStats.gpuKernelLaunchCount} · chunk ${formatBytes(preview.cacheStats.gpuTotalDeviceBytesPerChunk)} · outcome ${formatBytes(preview.cacheStats.gpuOutcomeTensorBytes)}`
    : '—'
  const gpuMemoryPlanLine = preview.cacheStats.gpuMemoryBudgetSource
    ? `${preview.cacheStats.gpuMemoryBudgetSource} · free ${formatBytes(preview.cacheStats.gpuDeviceFreeMemoryBytes)} · usable ${formatBytes(preview.cacheStats.gpuDeviceUsableMemoryBytes)} · budget ${formatBytes(preview.cacheStats.gpuTupleBudgetBytes)}`
    : '—'
  const gpuFitLine = preview.cacheStats.gpuMemoryBudgetSource
    ? `outcome ${preview.cacheStats.gpuOutcomeTensorFitsInMemory ? 'fits' : 'does not fit'} · chunk ${preview.cacheStats.gpuChunkPlanFitsInMemory ? 'fits' : 'does not fit'}`
    : '—'
  const autoBackendLine = preview.autoBackendDecision.recommendedBackend
    ? `${preview.autoBackendDecision.recommendedBackend}${preview.autoBackendDecision.gpuEligible ? ' · GPU-eligible' : ' · CPU-favored'}`
    : '—'
  return [
    {
      key: 'candidate-density',
      label: 'Candidate density',
      value: `${preview.compiledCandidateCount} / ${preview.candidateSlotCount} (${formatPercent(preview.candidateDensity)})`,
    },
    {
      key: 'invalid-slots',
      label: 'Invalid slots',
      value: `${preview.invalidSlotCount} (${formatPercent(preview.invalidSlotRate)})`,
    },
    {
      key: 'event-coverage',
      label: 'Event coverage',
      value: `${preview.eventCoverage.totalEventsWithCandidates} candidate-ready / ${preview.eventCoverage.totalEventsWithBaselineOutcomes} baseline / ${preview.eventCoverage.totalEventsWithConservativeOutcomes} conservative / ${preview.eventCoverage.totalEventsWithHarsherOutcomes} harsher`,
    },
    {
      key: 'tuple-grid',
      label: 'Tuple grid',
      value: `${preview.compiledTupleCount} tuples · ${preview.projectedEventTupleEvaluations} event-tuples`,
    },
    {
      key: 'preview-runtime',
      label: 'Replay / Total preview',
      value: `${preview.replayBuildMs} ms / ${preview.totalPreviewMs} ms`,
    },
    {
      key: 'compile-runtime',
      label: 'Compile / CPU rank',
      value: `${preview.timings.baselineCandidateCompileMs + preview.timings.baselineExitSummaryCompileMs + preview.timings.conservativeCandidateCompileMs + preview.timings.conservativeExitSummaryCompileMs + preview.timings.harsherCandidateCompileMs + preview.timings.harsherExitSummaryCompileMs} ms compile · ${formatLatency(preview.timings.cpuRankingMs, preview.timings.cpuRankingUs)} rank`,
    },
    {
      key: 'summary-density',
      label: 'Summary density',
      value: `${formatPercent(preview.baselineExitSummaryDensity)} / ${formatPercent(preview.conservativeExitSummaryDensity)} / ${formatPercent(preview.harsherExitSummaryDensity)}`,
    },
    {
      key: 'backend-preview',
      label: 'Backend / Hybrid rank timing',
      value: backendLine,
    },
    {
      key: 'auto-backend',
      label: 'Auto backend',
      value: autoBackendLine,
    },
    {
      key: 'gpu-breakdown',
      label: 'GPU breakdown',
      value: gpuBreakdownLine,
    },
    {
      key: 'cache-reuse',
      label: 'Cache reuse',
      value: `struct ${preview.cacheStats.structuralCacheHits}/${preview.cacheStats.structuralCacheMisses} · summary ${preview.cacheStats.summaryCacheHits}/${preview.cacheStats.summaryCacheMisses}`,
    },
    {
      key: 'gpu-upload-reuse',
      label: 'GPU upload reuse',
      value: gpuUploadReuseLine,
    },
    {
      key: 'gpu-chunking',
      label: 'GPU chunking',
      value: gpuChunkingLine,
    },
    {
      key: 'gpu-memory-plan',
      label: 'GPU memory plan',
      value: gpuMemoryPlanLine,
    },
    {
      key: 'gpu-fit',
      label: 'GPU fit',
      value: gpuFitLine,
    },
  ]
})

const feasibilityInvalidReasons = computed(() => {
  const preview = latestFeasibility.value
  if (!preview || preview.invalidReasons.length === 0) return ''
  return preview.invalidReasons
    .slice(0, 3)
    .map((entry) => `${entry.reason} (${entry.count})`)
    .join(' | ')
})

const feasibilityRecommendations = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  const out: string[] = []
  const axisCoverage = preview.axisCoverage ?? {
    delayMinutes: [],
    dteDays: [],
    moneynessBuckets: [],
  }
  if (preview.candidateDensity < 0.3 || preview.invalidSlotRate > 0.7) {
    pushUniqueRecommendation(
      out,
      `This grid is still quality-bound rather than scorer-bound: density is ${formatPercent(preview.candidateDensity)} and invalid slots are ${formatPercent(preview.invalidSlotRate)}. Narrow the representative search space before treating GPU as product-safe.`,
    )
  }
  const totalInvalid = Math.max(1, preview.invalidSlotCount)
  for (const entry of preview.invalidReasons) {
    const share = entry.count / totalInvalid
    if (share < 0.1) continue
    switch (entry.reason) {
      case 'no_expiration_in_bucket':
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Reduce the default DTE spread for this dataset or download a wider expiration window before widening the tuple grid.`,
        )
        break
      case 'no_strike_in_tolerance':
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Re-center the strike window closer to spot or relax the moneyness selector tolerance before widening the tuple grid.`,
        )
        break
      case 'entry_past_cutoff':
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Trim late delay buckets, lower max entry lag, or avoid late-session parent signals for this profile.`,
        )
        break
      case 'no_quote_after_entry_time':
      case 'missing_data':
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Check dataset coverage continuity before widening the search grid.`,
        )
        break
    }
  }
  const zeroCoverageDte = axisCoverage.dteDays
    .filter((entry) => entry.validSlotCount === 0)
    .map((entry) => `${entry.dteDays}d`)
  if (zeroCoverageDte.length > 0) {
    pushUniqueRecommendation(
      out,
      `These DTE buckets are producing zero valid slots on the current dataset: ${zeroCoverageDte.join(', ')}. Drop them from the request or widen the downloaded expiration window before treating this as a representative grid.`,
    )
  }
  const cutoffHeavyDelays = axisCoverage.delayMinutes
    .filter((entry) => {
      const cutoffMisses = entry.invalidReasons.find((reason) => reason.reason === 'entry_past_cutoff')?.count ?? 0
      return entry.invalidSlotCount > 0 && cutoffMisses / entry.invalidSlotCount >= 0.5
    })
    .map((entry) => `${entry.delayMinutes}m`)
  if (cutoffHeavyDelays.length > 0) {
    pushUniqueRecommendation(
      out,
      `Late-entry pressure is concentrated in these delay buckets: ${cutoffHeavyDelays.join(', ')}. Trim them, lower max entry lag, or keep the parent signal earlier in the session for this profile.`,
    )
  }
  const zeroCoverageMoneyness = axisCoverage.moneynessBuckets
    .filter((entry) => entry.validSlotCount === 0)
    .map((entry) => formatMoneynessBucketLabel(entry.moneynessBucket))
  if (zeroCoverageMoneyness.length > 0) {
    pushUniqueRecommendation(
      out,
      `These moneyness buckets never found a valid contract in the imported strike window: ${zeroCoverageMoneyness.join(', ')}. Re-center the strike range or prune those selectors before widening the grid.`,
    )
  }
  const gpuSpeedup = preview.timings.gpuSpeedupVsCpu
  if (gpuSpeedup != null && gpuSpeedup >= 3.0) {
    pushUniqueRecommendation(
      out,
      `GPU hybrid ranking already clears the current speed gate at ${gpuSpeedup.toFixed(2)}x vs CPU. The remaining blocker is candidate quality, not scorer throughput.`,
    )
  }
  if (preview.cacheStats.gpuPlanNotes.length > 0) {
    for (const note of preview.cacheStats.gpuPlanNotes) {
      pushUniqueRecommendation(out, note)
    }
  }
  for (const reason of preview.autoBackendDecision.reasons ?? []) {
    pushUniqueRecommendation(out, reason)
  }
  return out
})

const feasibilityGateRows = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  const structuralAttempts = preview.cacheStats.structuralCacheHits + preview.cacheStats.structuralCacheMisses
  const summaryAttempts = preview.cacheStats.summaryCacheHits + preview.cacheStats.summaryCacheMisses
  const structuralReuse =
    structuralAttempts > 0 ? preview.cacheStats.structuralCacheHits / structuralAttempts : null
  const summaryReuse = summaryAttempts > 0 ? preview.cacheStats.summaryCacheHits / summaryAttempts : null
  const gpuSpeedup = preview.timings.gpuSpeedupVsCpu
  return [
    {
      key: 'density',
      label: 'Candidate density >= 30%',
      value: formatPercent(preview.candidateDensity),
      passed: preview.candidateDensity >= 0.30,
      pending: false,
    },
    {
      key: 'invalid-rate',
      label: 'Invalid rate <= 70%',
      value: formatPercent(preview.invalidSlotRate),
      passed: preview.invalidSlotRate <= 0.70,
      pending: false,
    },
    {
      key: 'struct-reuse',
      label: 'Structural reuse >= 95%',
      value: structuralReuse == null ? '—' : formatPercent(structuralReuse),
      passed: structuralReuse != null && structuralReuse >= 0.95,
      pending: structuralReuse == null,
    },
    {
      key: 'summary-reuse',
      label: 'Summary reuse >= 95%',
      value: summaryReuse == null ? '—' : formatPercent(summaryReuse),
      passed: summaryReuse != null && summaryReuse >= 0.95,
      pending: summaryReuse == null,
    },
    {
      key: 'gpu-speedup',
      label: 'GPU hybrid speedup >= 3.0x',
      value: gpuSpeedup == null ? 'pending' : `${gpuSpeedup.toFixed(2)}x`,
      passed: gpuSpeedup != null && gpuSpeedup >= 3.0,
      pending: gpuSpeedup == null,
    },
  ]
})

const feasibilityOverallStatus = computed<'pass' | 'mixed' | 'pending'>(() => {
  const rows = feasibilityGateRows.value
  if (!rows.length) return 'pending'
  if (rows.every((row) => row.passed || row.pending)) {
    return rows.every((row) => row.passed) ? 'pass' : 'pending'
  }
  return 'mixed'
})

const feasibilityLaneRows = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  return [
    {
      key: 'baseline',
      label: 'Baseline',
      summaries: `${preview.baselineCompiledExitSummaryCount} / ${preview.baselineExitSummaryCount}`,
      density: formatPercent(preview.baselineExitSummaryDensity),
      compileMs: `${preview.timings.baselineCandidateCompileMs + preview.timings.baselineExitSummaryCompileMs} ms`,
      bytes: formatBytes(preview.baselineSummaryBytesEstimate),
    },
    {
      key: 'conservative',
      label: 'Conservative',
      summaries: `${preview.conservativeCompiledExitSummaryCount} / ${preview.conservativeExitSummaryCount}`,
      density: formatPercent(preview.conservativeExitSummaryDensity),
      compileMs: `${preview.timings.conservativeCandidateCompileMs + preview.timings.conservativeExitSummaryCompileMs} ms`,
      bytes: formatBytes(preview.conservativeSummaryBytesEstimate),
    },
    {
      key: 'harsher',
      label: 'Harsher',
      summaries: `${preview.harsherCompiledExitSummaryCount} / ${preview.harsherExitSummaryCount}`,
      density: formatPercent(preview.harsherExitSummaryDensity),
      compileMs: `${preview.timings.harsherCandidateCompileMs + preview.timings.harsherExitSummaryCompileMs} ms`,
      bytes: formatBytes(preview.harsherSummaryBytesEstimate),
    },
  ]
})

const feasibilityInvalidReasonRows = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  const totalInvalid = preview.invalidSlotCount || 1
  return preview.invalidReasons.slice(0, 6).map((entry) => ({
    ...entry,
    share: entry.count / totalInvalid,
  }))
})

const feasibilityAxisCoverageSections = computed(() => {
  const preview = latestFeasibility.value
  if (!preview) return []
  const axisCoverage = preview.axisCoverage ?? {
    delayMinutes: [],
    dteDays: [],
    moneynessBuckets: [],
  }
  return [
    {
      key: 'delay',
      label: 'Delay buckets',
      bucketLabel: 'Delay',
      rows: axisCoverage.delayMinutes.map((entry) => ({
        key: `delay-${entry.delayMinutes}`,
        bucket: `${entry.delayMinutes}m`,
        validSlots: `${entry.validSlotCount} / ${entry.slotCount}`,
        validRate: formatPercent(entry.validRate),
        invalidReasons: formatBucketInvalidReasons(entry.invalidReasons),
      })),
    },
    {
      key: 'dte',
      label: 'DTE buckets',
      bucketLabel: 'DTE',
      rows: axisCoverage.dteDays.map((entry) => ({
        key: `dte-${entry.dteDays}`,
        bucket: `${entry.dteDays}d`,
        validSlots: `${entry.validSlotCount} / ${entry.slotCount}`,
        validRate: formatPercent(entry.validRate),
        invalidReasons: formatBucketInvalidReasons(entry.invalidReasons),
      })),
    },
    {
      key: 'moneyness',
      label: 'Moneyness buckets',
      bucketLabel: 'Bucket',
      rows: axisCoverage.moneynessBuckets.map((entry) => ({
        key: `moneyness-${entry.moneynessBucket}`,
        bucket: formatMoneynessBucketLabel(entry.moneynessBucket),
        validSlots: `${entry.validSlotCount} / ${entry.slotCount}`,
        validRate: formatPercent(entry.validRate),
        invalidReasons: formatBucketInvalidReasons(entry.invalidReasons),
      })),
    },
  ].filter((section) => section.rows.length > 0)
})

const requestSummaryRows = computed(() => {
  const request = activeRequest.value
  if (!request) return []
  const filters = request.filters
  const filterSegments = [
    `entry lag ${filters.maxEntryLagMinutes ?? '—'}`,
    `max prem ${filters.maxPremium ?? '—'}`,
    `min prem ${filters.minPremium ?? '—'}`,
    `min vol ${filters.minVolume ?? '—'}`,
    `min OI ${filters.minOpenInterest ?? '—'}`,
  ]
  return [
    {
      key: 'artifact-ts',
      label: 'Artifact timestamp',
      value: normalizeText(activeArtifactTimestamp.value) || '—',
    },
    {
      key: 'dataset-hash',
      label: 'Dataset hash',
      value: normalizeText(activeOptionsDatasetHash.value) || '—',
    },
    {
      key: 'delays-dte-holds',
      label: 'Delays / DTE / Holds',
      value: `${formatList(request.delayMinutes)} | ${formatList(request.dteBuckets)} | ${formatList(request.holdMinutes)}`,
    },
    {
      key: 'targets-stops',
      label: 'Targets / Stops',
      value: `${formatList(request.targetReturnBuckets)} | ${formatList(request.stopReturnBuckets)}`,
    },
    {
      key: 'exit-fill',
      label: 'Exit / Fill',
      value: `${normalizeText(request.exitTemplate) || '—'} | ${normalizeText(request.fillModel) || '—'}`,
    },
    {
      key: 'backend-preference',
      label: 'Backend preference',
      value: normalizeText(request.backendPreference) || 'auto',
    },
    {
      key: 'backend-fallback',
      label: 'Backend fallback',
      value: normalizeText(activeBackendFallbackReason.value) || '—',
    },
    {
      key: 'dataset-provenance',
      label: 'Dataset provenance',
      value: activeDatasetProvenanceSummary.value || '—',
    },
    {
      key: 'splits',
      label: 'Split assignment / ratios',
      value:
        `${formatSplitAssignmentMode(request.splitAssignmentMode)} | ` +
        `${request.splitRatioSearch.toFixed(2)} / ${request.splitRatioHoldout.toFixed(2)}`,
    },
    {
      key: 'sealed-finalists',
      label: 'Sealed finalists',
      value: `${request.sealedFinalistCount ?? 0}`,
    },
    {
      key: 'filters',
      label: 'Filters',
      value: filterSegments.join(' | '),
    },
  ]
})

function toggleMoneyness(bucket: string, checked: boolean): void {
  const next = new Set(selectedMoneynessBuckets.value)
  if (checked) next.add(bucket)
  else next.delete(bucket)
  selectedMoneynessBuckets.value = Array.from(next)
}

function onMoneynessToggle(bucket: string, event: Event): void {
  const target = event.target
  const checked = target instanceof HTMLInputElement ? target.checked : false
  toggleMoneyness(bucket, checked)
}

function selectTuple(index: number): void {
  selectedTupleIndex.value = index
}

function loadSelectedTupleInspection(): void {
  if (!selectedTuple.value) return
  void store.loadTupleInspection(selectedTupleIndex.value)
}

function loadSelectedEventPath(): void {
  const row = selectedInspectionRow.value
  if (!row || row.status !== 'executed') return
  void store.loadEventPathInspection(selectedTupleIndex.value, row.eventIndex)
}

function selectInspectionRow(eventId: string): void {
  selectedInspectionEventId.value = normalizeText(eventId)
  store.clearEventPathInspection()
}

function formatArtifactOptionLabel(entry: {
  ts: string
  path: string
  optionsDatasetId: string
  backendFallbackReason?: string
  tupleCount: number
  bestRobustScore: number
  bestSelectionExpectancyPerPremium: number
  bestHoldoutExpectancyPerPremium: number | null
  bestSealedExpectancyPerPremium: number | null
  bestStressExpectancyPerPremium: number | null
  bestSealedStressExpectancyPerPremium: number | null
  bestHarsherStressExpectancyPerPremium: number | null
  bestSealedHarsherStressExpectancyPerPremium: number | null
  sealedEvaluatedTupleCount?: number
  sealedFinalistBudget?: number
  hasHoldoutMetrics: boolean
  hasSealedEvaluationMetrics: boolean
  hasStressMetrics: boolean
  hasHarsherStressMetrics: boolean
}): string {
  const parts = [
    normalizeText(entry.ts) || normalizeText(entry.path),
    normalizeText(entry.optionsDatasetId) || 'unknown dataset',
    `${entry.tupleCount} tuples`,
    `robust ${entry.bestRobustScore.toFixed(3)}`,
    `sel ${entry.bestSelectionExpectancyPerPremium.toFixed(3)}`,
  ]
  if (entry.bestHoldoutExpectancyPerPremium != null) {
    parts.push(`hold ${entry.bestHoldoutExpectancyPerPremium.toFixed(3)}`)
  } else if (entry.hasHoldoutMetrics) {
    parts.push('holdout')
  }
  if (entry.bestSealedExpectancyPerPremium != null) {
    parts.push(`sealed ${entry.bestSealedExpectancyPerPremium.toFixed(3)}`)
  } else if (entry.hasSealedEvaluationMetrics) {
    parts.push('sealed')
  }
  if (entry.bestStressExpectancyPerPremium != null) {
    parts.push(`stress ${entry.bestStressExpectancyPerPremium.toFixed(3)}`)
  } else if (entry.hasStressMetrics) {
    parts.push('stress')
  }
  if (entry.bestSealedStressExpectancyPerPremium != null) {
    parts.push(`sealed stress ${entry.bestSealedStressExpectancyPerPremium.toFixed(3)}`)
  }
  if (entry.bestHarsherStressExpectancyPerPremium != null) {
    parts.push(`harsher ${entry.bestHarsherStressExpectancyPerPremium.toFixed(3)}`)
  } else if (entry.hasHarsherStressMetrics) {
    parts.push('harsher')
  }
  if (entry.bestSealedHarsherStressExpectancyPerPremium != null) {
    parts.push(`sealed harsher ${entry.bestSealedHarsherStressExpectancyPerPremium.toFixed(3)}`)
  }
  if ((entry.sealedFinalistBudget || 0) > 0) {
    parts.push(`finalists ${entry.sealedEvaluatedTupleCount || 0}/${entry.sealedFinalistBudget || 0}`)
  }
  if (normalizeText(entry.backendFallbackReason)) {
    parts.push('fallback')
  }
  return parts.join(' · ')
}

function loadArtifactFromComparison(path: string): void {
  selectedArtifactPath.value = path
  void store.loadArtifact()
}
</script>

<template>
  <section class="optionsPanel surfaceFocus" aria-label="Options expression follow-up">
    <div class="panelSectionHeader">
      <div>
        <div class="panelEyebrow">Follow-up workflow</div>
        <h3 class="panelPaneTitle" style="margin: 0;">Options Expression</h3>
        <div class="panelSectionLead">
          Express a saved validated directional artifact through a constrained Polygon/Massive options search without mutating the main workflow state.
        </div>
        <div class="panelMuted" style="margin-top: 6px;">
          This is an options follow-up surface. Conventional stock optimization and validation remain in the main workflow.
        </div>
      </div>
    </div>

    <section v-if="!hasParentSelection" class="emptyState">
      <div class="emptyTitle">Select a saved run first.</div>
      <div class="emptyBody">
        Options expression optimization is launched from a saved artifact in the History workspace.
      </div>
    </section>

    <section v-else-if="!supportedParent" class="emptyState">
      <div class="emptyTitle">Saved run not supported yet.</div>
      <div class="emptyBody">
        The current vertical slice supports saved <span class="mono">DoubleMa</span> artifacts only. Other strategy families stay read-only until their manifest projector is implemented.
      </div>
    </section>

    <template v-else>
      <div class="panelSummaryChips" aria-label="Options expression summary">
        <span v-for="chip in resultSummaryChips" :key="chip" class="panelSummaryChip mono">{{ chip }}</span>
      </div>

      <div class="optionsGrid">
        <div class="controlBlock">
          <label for="optionsDatasetSelect">Options dataset</label>
          <div class="row inlineControls">
            <select id="optionsDatasetSelect" v-model="selectedDatasetId">
              <option value="">Select Polygon/Massive dataset…</option>
              <option
                v-for="entry in datasets"
                :key="entry.optionsDatasetId"
                :value="entry.optionsDatasetId"
              >
                {{ entry.optionsDatasetId }} · {{ entry.rootSymbol || 'unknown root' }} · {{ entry.priceSemantics }}
              </option>
            </select>
            <button type="button" :disabled="datasetsBusy" @click="store.refreshDatasets()">
              Refresh
            </button>
          </div>
          <div class="panelMuted">{{ datasetSummary || 'Import or select a Polygon/Massive options dataset.' }}</div>
          <div v-if="datasetProvenanceSummary" class="panelMuted mono">{{ datasetProvenanceSummary }}</div>
          <div v-if="rootMismatch" class="warningText">
            Selected dataset root does not match the current saved run root hint ({{ parentRootHint || 'unknown' }}). Continue only if that mismatch is intentional.
          </div>
        </div>

        <div class="controlBlock">
          <label for="optionsImportRoot">Polygon/Massive root symbol</label>
          <div class="row inlineControls">
            <input
              id="optionsImportRoot"
              v-model="importRootSymbol"
              type="text"
              placeholder="SPY"
              spellcheck="false"
            />
            <button type="button" :disabled="importBusy" @click="store.importDatasetFromPicker()">
              Import Polygon/Massive CSV
            </button>
          </div>
          <div class="panelMuted">
            Uses the existing native CSV picker and stores a normalized Polygon/Massive options dataset under the app market-data root.
          </div>
        </div>

        <div class="controlBlock">
          <label for="optionsDownloadFrom">Polygon/Massive 1m options download</label>
          <div class="downloadFieldGrid">
            <label>
              From
              <input id="optionsDownloadFrom" v-model="downloadFromDateInput" type="date" />
            </label>
            <label>
              To
              <input v-model="downloadToDateInput" type="date" />
            </label>
            <label>
              Contract type
              <select v-model="downloadContractType">
                <option value="call">Calls</option>
                <option value="put">Puts</option>
                <option value="both">Calls + puts</option>
              </select>
            </label>
            <label>
              Dataset id override
              <input v-model="downloadDatasetIdInput" type="text" placeholder="optional_dataset_id" spellcheck="false" />
            </label>
            <label>
              Expiration from
              <input v-model="downloadExpirationFromDateInput" type="date" />
            </label>
            <label>
              Expiration to
              <input v-model="downloadExpirationToDateInput" type="date" />
            </label>
            <label>
              Max DTE days
              <input v-model="downloadMaxDteDaysInput" type="text" inputmode="numeric" placeholder="3" spellcheck="false" />
            </label>
            <label>
              Strike min
              <input v-model="downloadStrikeMinInput" type="text" inputmode="decimal" placeholder="645" spellcheck="false" />
            </label>
            <label>
              Strike max
              <input v-model="downloadStrikeMaxInput" type="text" inputmode="decimal" placeholder="665" spellcheck="false" />
            </label>
            <label>
              Max contracts
              <input v-model="downloadMaxContractsInput" type="text" inputmode="numeric" placeholder="30" spellcheck="false" />
            </label>
          </div>
          <div class="row inlineControls">
            <button type="button" :disabled="downloadBusy" @click="store.downloadDatasetFromPolygon()">
              Download Polygon/Massive 1m data
            </button>
          </div>
          <div class="panelMuted">
            This download is options-only and stays separate from the normal stock data workflow. Keep the date, expiry, strike, and contract windows bounded so the resulting 1m dataset remains usable for options-expression optimization.
          </div>
        </div>
      </div>

      <details v-if="selectedDataset" class="panelDetails" open>
        <summary>Dataset manifest</summary>
        <div class="panelDetailsBody">
          <div class="reportingGrid">
            <div v-for="row in selectedDatasetDetailRows" :key="row.key" class="metricCard">
              <div class="panelMetricKey">{{ row.label }}</div>
              <div class="metricValue mono">{{ row.value }}</div>
            </div>
          </div>
          <div class="actions">
            <button type="button" @click="store.revealSelectedDatasetPath()">
              Reveal dataset path
            </button>
          </div>
        </div>
      </details>

      <details v-if="latestDatasetDownload" class="panelDetails" open>
        <summary>Latest download audit</summary>
        <div class="panelDetailsBody">
          <div class="reportingGrid">
            <div v-for="row in latestDownloadAuditRows" :key="row.key" class="metricCard">
              <div class="panelMetricKey">{{ row.label }}</div>
              <div class="metricValue mono">{{ row.value }}</div>
            </div>
          </div>
          <div class="actions">
            <button type="button" @click="store.revealLatestDownloadUnderlyingPath()">
              Reveal underlying CSV
            </button>
          </div>
        </div>
      </details>

      <details class="panelDetails" open>
        <summary>Search grid and execution settings</summary>
        <div class="panelDetailsBody optionsFields">
          <label>
            Delay minutes
            <input v-model="delayMinutesInput" type="text" placeholder="0,1,5" spellcheck="false" />
          </label>
          <label>
            DTE buckets
            <input v-model="dteBucketsInput" type="text" placeholder="1,3,5" spellcheck="false" />
          </label>
          <label>
            Hold minutes
            <input v-model="holdMinutesInput" type="text" placeholder="5,15,30,60" spellcheck="false" />
          </label>
          <label>
            Target buckets
            <input v-model="targetReturnInput" type="text" placeholder="0.25,0.5,1" spellcheck="false" />
          </label>
          <label>
            Stop buckets
            <input v-model="stopReturnInput" type="text" placeholder="0.25,0.5" spellcheck="false" />
          </label>
          <label>
            Exit template
            <select v-model="exitTemplate">
              <option v-for="entry in OPTIONS_EXIT_TEMPLATE_OPTIONS" :key="entry" :value="entry">
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Fill model
            <select v-model="fillModel">
              <option v-for="entry in OPTIONS_FILL_MODEL_OPTIONS" :key="entry" :value="entry">
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Backend preference
            <select v-model="backendPreference">
              <option v-for="entry in OPTIONS_BACKEND_PREFERENCE_OPTIONS" :key="entry" :value="entry">
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Top K
            <input v-model="topKInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Sealed finalists
            <input v-model="sealedFinalistCountInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Min valid trades
            <input v-model="minValidTradesInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Search split
            <input v-model="splitRatioSearchInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Holdout split
            <input v-model="splitRatioHoldoutInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Split assignment
            <select v-model="splitAssignmentMode">
              <option v-for="entry in OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS" :key="entry" :value="entry">
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Max entry lag
            <input v-model="maxEntryLagMinutesInput" type="text" inputmode="numeric" spellcheck="false" />
          </label>
          <label>
            Max premium
            <input v-model="maxPremiumInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Min premium
            <input v-model="minPremiumInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Min volume
            <input v-model="minVolumeInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <label>
            Min open interest
            <input v-model="minOpenInterestInput" type="text" inputmode="decimal" spellcheck="false" />
          </label>
          <div class="full">
            <div class="panelMuted">
              <span class="mono">event_chronological</span> preserves the current default. Use
              <span class="mono">session_chronological</span> for multi-session feasibility work when
              holdout events are clustering late in the day.
            </div>
          </div>
          <div class="full">
            <div class="panelMetricKey">Moneyness buckets</div>
            <div class="moneynessList">
              <label v-for="entry in OPTIONS_MONEYNESS_BUCKET_OPTIONS" :key="entry" class="bucketToggle">
                <input
                  :checked="selectedMoneynessBuckets.includes(entry)"
                  type="checkbox"
                  @change="onMoneynessToggle(entry, $event)"
                />
                <span class="mono">{{ entry }}</span>
              </label>
            </div>
          </div>
        </div>
      </details>

      <div class="actions">
        <button
          type="button"
          :disabled="feasibilityBusy || !selectedDatasetId"
          @click="store.previewCurrentFeasibility()"
        >
          {{ feasibilityBusy ? 'Previewing…' : 'Preview feasibility' }}
        </button>
        <button
          type="button"
          :disabled="runBusy || !selectedDatasetId"
          @click="store.runCurrentExpression()"
        >
          {{ runBusy ? 'Running…' : 'Optimize expression' }}
        </button>
        <button type="button" :disabled="artifactsBusy" @click="store.refreshArtifacts()">
          Refresh artifacts
        </button>
        <button
          type="button"
          :disabled="loadBusy || !selectedArtifactPath"
          @click="store.loadArtifact()"
        >
          Load selected artifact
        </button>
        <button
          type="button"
          :disabled="!activeArtifactPath"
          @click="store.revealActiveArtifact()"
        >
          Reveal artifact
        </button>
        <button
          type="button"
          :disabled="activeRankedResults.length === 0"
          @click="store.exportActiveCsv()"
        >
          Export CSV
        </button>
      </div>

      <div class="controlBlock">
        <label for="optionsArtifactSelect">Saved options expression artifacts</label>
        <select id="optionsArtifactSelect" v-model="selectedArtifactPath">
          <option value="">Select saved options expression artifact…</option>
          <option
            v-for="entry in filteredArtifacts"
            :key="entry.path"
            :value="entry.path"
          >
            {{ formatArtifactOptionLabel(entry) }}
          </option>
        </select>
        <div class="panelMuted">
          Filtered to the currently selected saved run.
        </div>
        <div v-if="selectedArtifactSummary" class="panelMuted mono">
          {{ selectedArtifactSummary.backendUsed }}<template v-if="normalizeText(selectedArtifactSummary.backendFallbackReason)"> · fallback {{ selectedArtifactSummary.backendFallbackReason }}</template> · {{ selectedArtifactSummary.priceSemantics }} · sel {{ selectedArtifactSummary.bestSelectionExpectancyPerPremium.toFixed(3) }}<template v-if="selectedArtifactSummary.bestHoldoutExpectancyPerPremium != null"> · hold {{ selectedArtifactSummary.bestHoldoutExpectancyPerPremium.toFixed(3) }}</template><template v-if="selectedArtifactSummary.bestSealedExpectancyPerPremium != null"> · sealed {{ selectedArtifactSummary.bestSealedExpectancyPerPremium.toFixed(3) }}</template><template v-if="selectedArtifactSummary.bestStressExpectancyPerPremium != null"> · stress {{ selectedArtifactSummary.bestStressExpectancyPerPremium.toFixed(3) }}</template><template v-if="selectedArtifactSummary.bestSealedStressExpectancyPerPremium != null"> · sealed stress {{ selectedArtifactSummary.bestSealedStressExpectancyPerPremium.toFixed(3) }}</template><template v-if="selectedArtifactSummary.bestHarsherStressExpectancyPerPremium != null"> · harsher {{ selectedArtifactSummary.bestHarsherStressExpectancyPerPremium.toFixed(3) }}</template><template v-if="selectedArtifactSummary.bestSealedHarsherStressExpectancyPerPremium != null"> · sealed harsher {{ selectedArtifactSummary.bestSealedHarsherStressExpectancyPerPremium.toFixed(3) }}</template>
        </div>
      </div>

      <details v-if="artifactComparisonRows.length > 0" class="panelDetails" open>
        <summary>Saved artifact comparison</summary>
        <div class="panelDetailsBody">
          <div class="matrixTableWrap">
            <table class="matrixTable compactTable">
              <thead>
                <tr>
                  <th>Artifact</th>
                  <th>Dataset</th>
                  <th>Backend</th>
                  <th>Robust</th>
                  <th>Sel EV</th>
                  <th>Hold EV</th>
                  <th>Sealed EV</th>
                  <th>Stress</th>
                  <th>Harsher</th>
                  <th>Sealed</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="entry in artifactComparisonRows"
                  :key="entry.path"
                  :class="{ selectedTupleRow: normalizeText(entry.path) === normalizeText(selectedArtifactPath) }"
                >
                  <td class="mono">{{ normalizeText(entry.ts) || '—' }}</td>
                  <td class="mono">{{ entry.optionsDatasetId }}</td>
                  <td class="mono">
                    {{ entry.backendUsed }}<template v-if="normalizeText(entry.backendFallbackReason)"> · fallback</template>
                  </td>
                  <td class="mono">{{ entry.bestRobustScore.toFixed(3) }}</td>
                  <td class="mono">{{ entry.bestSelectionExpectancyPerPremium.toFixed(3) }}</td>
                  <td class="mono">{{ entry.bestHoldoutExpectancyPerPremium != null ? entry.bestHoldoutExpectancyPerPremium.toFixed(3) : '—' }}</td>
                  <td class="mono">{{ entry.bestSealedExpectancyPerPremium != null ? entry.bestSealedExpectancyPerPremium.toFixed(3) : '—' }}</td>
                  <td class="mono">{{ entry.bestStressExpectancyPerPremium != null ? entry.bestStressExpectancyPerPremium.toFixed(3) : '—' }}</td>
                  <td class="mono">{{ entry.bestHarsherStressExpectancyPerPremium != null ? entry.bestHarsherStressExpectancyPerPremium.toFixed(3) : '—' }}</td>
                  <td class="mono">{{ entry.sealedEvaluatedTupleCount ?? 0 }}/{{ entry.sealedFinalistBudget ?? 0 }}</td>
                  <td>
                    <button type="button" @click="loadArtifactFromComparison(entry.path)">
                      Load
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </details>

      <section v-if="latestFeasibility" class="resultCard">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Phase 0.5 preview</div>
            <div class="panelSectionLead">
              Review candidate density, event coverage, and replay/compile timing before committing to a full optimization run or GPU work.
            </div>
          </div>
        </div>
        <div class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: feasibilityOverallStatus === 'pass',
              isPending: feasibilityOverallStatus === 'pending',
              isMixed: feasibilityOverallStatus === 'mixed',
            }"
          >
            GPU viability {{ feasibilityOverallStatus === 'pass' ? 'pass' : feasibilityOverallStatus === 'pending' ? 'pending' : 'mixed' }}
          </span>
          <span
            v-for="row in feasibilityGateRows"
            :key="row.key"
            class="gateBadge"
            :class="{
              isPass: row.passed,
              isPending: row.pending && !row.passed,
              isMixed: !row.passed && !row.pending,
            }"
          >
            {{ row.label }} · {{ row.value }}
          </span>
        </div>
        <div class="detailGrid detailGridTight">
          <div v-for="row in feasibilitySummaryRows" :key="row.key">
            <span class="panelMetricKey">{{ row.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
          </div>
        </div>
        <details class="panelDetails">
          <summary>Lane diagnostics</summary>
          <div class="panelDetailsBody">
            <div class="detailGrid detailGridTight">
              <div>
                <span class="panelMetricKey">Structural cache</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{ formatBytes(latestFeasibility.structuralCacheBytesEstimate) }}
                </span>
              </div>
            </div>
            <div class="tableWrap">
              <table class="resultTable">
                <thead>
                  <tr>
                    <th>Lane</th>
                    <th>Compiled / Total summaries</th>
                    <th>Density</th>
                    <th>Compile time</th>
                    <th>Bytes</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in feasibilityLaneRows" :key="row.key">
                    <td class="mono">{{ row.label }}</td>
                    <td class="mono">{{ row.summaries }}</td>
                    <td class="mono">{{ row.density }}</td>
                    <td class="mono">{{ row.compileMs }}</td>
                    <td class="mono">{{ row.bytes }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </details>
        <details v-if="feasibilityInvalidReasonRows.length > 0" class="panelDetails">
          <summary>Invalid reasons</summary>
          <div class="panelDetailsBody tableWrap">
            <table class="resultTable">
              <thead>
                <tr>
                  <th>Reason</th>
                  <th>Count</th>
                  <th>Share</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in feasibilityInvalidReasonRows" :key="row.reason">
                  <td class="mono">{{ row.reason }}</td>
                  <td class="mono">{{ row.count }}</td>
                  <td class="mono">{{ formatPercent(row.share) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </details>
        <details v-if="feasibilityAxisCoverageSections.length > 0" class="panelDetails">
          <summary>Bucket coverage</summary>
          <div class="panelDetailsBody">
            <div
              v-for="section in feasibilityAxisCoverageSections"
              :key="section.key"
              class="coverageSection"
            >
              <div class="panelMuted">{{ section.label }}</div>
              <div class="tableWrap">
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>{{ section.bucketLabel }}</th>
                      <th>Valid / Total slots</th>
                      <th>Valid rate</th>
                      <th>Top invalid reasons</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="row in section.rows" :key="row.key">
                      <td class="mono">{{ row.bucket }}</td>
                      <td class="mono">{{ row.validSlots }}</td>
                      <td class="mono">{{ row.validRate }}</td>
                      <td class="mono">{{ row.invalidReasons }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </details>
        <div v-if="feasibilityInvalidReasons" class="panelMuted mono">
          Invalid reasons: {{ feasibilityInvalidReasons }}
        </div>
        <details v-if="feasibilityRecommendations.length > 0" class="panelDetails">
          <summary>Recommendations</summary>
          <div class="panelDetailsBody">
            <ul class="recommendationList">
              <li v-for="recommendation in feasibilityRecommendations" :key="recommendation">
                {{ recommendation }}
              </li>
            </ul>
          </div>
        </details>
      </section>

      <section v-if="activeDiagnostics" class="resultCard">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Current result</div>
            <div class="panelSectionLead">
              Review the current top tuples before deciding whether this expression family is worth promoting into a wider options workflow.
            </div>
          </div>
        </div>
        <div class="detailGrid">
          <div>
            <span class="panelMetricKey">Root</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.rootSymbol }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Dataset</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.optionsDatasetId }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Events</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.totalEvents }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Search / Holdout / Sealed</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ activeDiagnostics.searchEvents }} / {{ activeDiagnostics.holdoutEvents }} / {{ activeDiagnostics.sealedTestEvents }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Contracts</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.datasetContracts }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Rows</span>
            <span class="panelMetricValue mono panelNumeric">{{ activeDiagnostics.datasetRows }}</span>
          </div>
        </div>

        <div class="detailGrid detailGridTight">
          <div v-for="card in reportingMetricCards" :key="card.key">
            <span class="panelMetricKey">{{ card.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{ card.value }}</span>
          </div>
        </div>

        <details v-if="requestSummaryRows.length > 0" class="panelDetails">
          <summary>Run provenance</summary>
          <div class="panelDetailsBody detailGrid detailGridTight">
            <div v-for="row in requestSummaryRows" :key="row.key">
              <span class="panelMetricKey">{{ row.label }}</span>
              <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
            </div>
          </div>
        </details>

        <div v-if="activeRows.length > 0" class="resultTableControls">
          <label>
            Sort by
            <select v-model="resultSortKey">
              <option value="robust">robust</option>
              <option value="selection_ev">selection EV</option>
              <option value="holdout_ev">holdout EV</option>
              <option value="stress_ev">stress EV</option>
              <option value="harsher_ev">harsher EV</option>
              <option value="invalid_rate">invalid rate</option>
              <option value="valid_trades">valid trades</option>
              <option value="sealed_rank">sealed rank</option>
            </select>
          </label>
          <label>
            Direction
            <select v-model="resultSortDirection">
              <option value="desc">desc</option>
              <option value="asc">asc</option>
            </select>
          </label>
          <label class="toggleField">
            <input v-model="finalistsOnly" type="checkbox" />
            <span>Finalists only</span>
          </label>
          <label>
            Max invalid rate
            <input v-model="maxInvalidRateFilterInput" type="text" inputmode="decimal" spellcheck="false" placeholder="0.35" />
          </label>
          <label>
            Min robust
            <input v-model="minRobustScoreFilterInput" type="text" inputmode="decimal" spellcheck="false" placeholder="0.20" />
          </label>
          <label>
            Max stress drop
            <input v-model="maxStressDropFilterInput" type="text" inputmode="decimal" spellcheck="false" placeholder="0.10" />
          </label>
        </div>

        <div v-if="activeRows.length > 0 && displayedRowsWithIndex.length > 0" class="tableWrap">
          <table class="resultTable">
            <thead>
              <tr>
                <th>DTE</th>
                <th>Moneyness</th>
                <th>Delay</th>
                <th>Hold</th>
                <th>Target / Stop</th>
                <th>Robust</th>
                <th>EV / Premium</th>
                <th>Invalid %</th>
                <th v-if="hasHoldoutMetrics">Holdout EV / Premium</th>
                <th v-if="hasSealedMetrics">Finalist</th>
                <th v-if="hasSealedMetrics">Sealed EV / Premium</th>
                <th v-if="hasStressMetrics">Stress EV / Premium</th>
                <th v-if="hasSealedStressMetrics">Sealed stress EV / Premium</th>
                <th v-if="hasHarsherStressMetrics">Harsher EV / Premium</th>
                <th v-if="hasSealedHarsherStressMetrics">Sealed harsher EV / Premium</th>
                <th>Win</th>
                <th>Trades</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="{ row, originalIndex } in displayedRowsWithIndex"
                :key="`${originalIndex}-${row.dteDays}-${row.moneynessBucket}-${row.delayMinutes}-${row.holdMinutes}-${row.targetReturn}-${row.stopReturn}`"
                :class="{ isSelected: originalIndex === selectedTupleIndex }"
                @click="selectTuple(originalIndex)"
              >
                <td class="mono">{{ row.dteDays }}</td>
                <td class="mono">{{ row.moneynessBucket }}</td>
                <td class="mono">{{ row.delayMinutes }}</td>
                <td class="mono">{{ row.holdMinutes }}</td>
                <td class="mono">
                  {{ row.targetReturn == null ? '—' : row.targetReturn }} /
                  {{ row.stopReturn == null ? '—' : row.stopReturn }}
                </td>
                <td class="mono">{{ row.robustScore.toFixed(3) }}</td>
                <td class="mono">{{ row.selectionMetrics.expectancyPerPremium.toFixed(4) }}</td>
                <td class="mono">{{ row.selectionMetrics.invalidRate.toFixed(3) }}</td>
                <td v-if="hasHoldoutMetrics" class="mono">
                  {{ row.holdoutMetrics ? row.holdoutMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasSealedMetrics" class="mono">
                  {{ row.sealedFinalist ? `#${row.sealedFinalRank ?? row.preSealedRank ?? 0}` : '—' }}
                </td>
                <td v-if="hasSealedMetrics" class="mono">
                  {{ row.sealedEvaluationMetrics ? row.sealedEvaluationMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasStressMetrics" class="mono">
                  {{ row.stressMetrics ? row.stressMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasSealedStressMetrics" class="mono">
                  {{ row.sealedStressMetrics ? row.sealedStressMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasHarsherStressMetrics" class="mono">
                  {{ row.harsherStressMetrics ? row.harsherStressMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td v-if="hasSealedHarsherStressMetrics" class="mono">
                  {{ row.sealedHarsherStressMetrics ? row.sealedHarsherStressMetrics.expectancyPerPremium.toFixed(4) : '—' }}
                </td>
                <td class="mono">{{ row.selectionMetrics.winRate.toFixed(3) }}</td>
                <td class="mono">{{ row.selectionMetrics.validTrades }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else-if="activeRows.length > 0" class="panelMuted">
          No tuples match the current client-side filters.
        </div>
        <div v-if="selectedTuple" class="detailInspector">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Selected tuple</div>
              <div class="panelSectionLead">
                Inspect the current top-row configuration and robustness metrics before promoting it into a wider options workflow.
              </div>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              :disabled="inspectionBusy || !activeArtifactPath"
              @click="loadSelectedTupleInspection()"
            >
              {{ inspectionBusy ? 'Loading events…' : 'Load event drilldown' }}
            </button>
          </div>
          <div class="detailGrid detailGridTight">
            <div>
              <span class="panelMetricKey">Delay / DTE / Hold</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.delayMinutes }} / {{ selectedTuple.dteDays }} / {{ selectedTuple.holdMinutes }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Moneyness</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.moneynessBucket }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Target / Stop</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.targetReturn == null ? '—' : selectedTuple.targetReturn }} /
                {{ selectedTuple.stopReturn == null ? '—' : selectedTuple.stopReturn }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Exit template</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.exitTemplate }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Fill model</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.fillModel }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Robust score</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.robustScore.toFixed(3) }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Pre-sealed rank</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.preSealedRank ?? '—' }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Sealed final rank</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.sealedFinalRank ?? '—' }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Sealed finalist</span>
              <span class="panelMetricValue mono panelNumeric">{{ selectedTuple.sealedFinalist ? 'yes' : 'no' }}</span>
            </div>
            <div>
              <span class="panelMetricKey">EV / Trade</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.expectancyPerTrade.toFixed(4) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">EV / Premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Median return / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.medianReturnOnPremium.toFixed(4) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Win rate</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.winRate.toFixed(3) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Valid / Invalid</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.validTrades }} / {{ selectedTuple.selectionMetrics.invalidEvents }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">PnL / contract</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.totalPnlPerContract.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.holdoutMetrics">
              <span class="panelMetricKey">Holdout EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.holdoutMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedEvaluationMetrics">
              <span class="panelMetricKey">Sealed EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.sealedEvaluationMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.stressMetrics">
              <span class="panelMetricKey">Stress EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.stressMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedStressMetrics">
              <span class="panelMetricKey">Sealed stress EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.sealedStressMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.harsherStressMetrics">
              <span class="panelMetricKey">Harsher EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.harsherStressMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedHarsherStressMetrics">
              <span class="panelMetricKey">Sealed harsher EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.sealedHarsherStressMetrics.expectancyPerPremium.toFixed(4) }}
              </span>
            </div>
          </div>
          <details v-if="selectedInspection" class="panelDetails" open>
            <summary>Tuple event drilldown</summary>
            <div class="panelDetailsBody">
              <div class="detailGrid detailGridTight">
                <div>
                  <span class="panelMetricKey">Events</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.totalEvents }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Executed</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.executedEvents }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Candidate missing</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.candidateMissingEvents }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Outcome missing</span>
                  <span class="panelMetricValue mono panelNumeric">{{ selectedInspection.outcomeMissingEvents }}</span>
                </div>
              </div>

              <div class="row inlineControls">
                <label>
                  Split filter
                  <select v-model="inspectionSplitFilter">
                    <option value="all">all</option>
                    <option value="search">search</option>
                    <option value="holdout">holdout</option>
                    <option value="sealed_test">sealed_test</option>
                  </select>
                </label>
                <button type="button" @click="store.exportActiveInspectionCsv()">
                  Export inspection CSV
                </button>
                <button
                  v-if="inspectionRows.length > 50 && !showAllInspectionRows"
                  type="button"
                  @click="showAllInspectionRows = true"
                >
                  Show all {{ inspectionRows.length }}
                </button>
              </div>

              <div class="panelMuted">
                Showing {{ displayedInspectionRows.length }} of {{ inspectionRows.length }} event rows for tuple #{{ selectedInspection.tupleIndex + 1 }}.
              </div>

              <div class="tableWrap">
                <table class="resultTable inspectionTable">
                  <thead>
                    <tr>
                      <th>Event</th>
                      <th>Split</th>
                      <th>Side</th>
                      <th>Status / reason</th>
                      <th>Contract</th>
                      <th>Requested entry</th>
                      <th>Entry</th>
                      <th>Lag / expiry</th>
                      <th>Exit</th>
                      <th>Exit reason</th>
                      <th>Path range</th>
                      <th>Return / premium</th>
                      <th>PnL / contract</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in displayedInspectionRows"
                      :key="row.eventId"
                      :class="{ isSelected: selectedInspectionRow?.eventId === row.eventId }"
                      @click="selectInspectionRow(row.eventId)"
                    >
                      <td class="mono">{{ row.eventId }}</td>
                      <td class="mono">{{ row.split }}</td>
                      <td class="mono">{{ row.side }}</td>
                      <td class="mono">
                        {{ row.status }}
                        <template v-if="row.candidateInvalidReason">
                          <br>
                          <span class="inspectionReason">{{ row.candidateInvalidReason }}</span>
                        </template>
                      </td>
                      <td class="mono">
                        <template v-if="row.contractId">
                          {{ row.contractId }}
                          <span v-if="row.strike != null"> @ {{ formatOptionalDecimal(row.strike, 2) }}</span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">{{ formatTimestamp(row.requestedEntryTimestamp) }}</td>
                      <td class="mono">
                        <template v-if="row.entryTimestamp != null">
                          {{ formatTimestamp(row.entryTimestamp) }} · {{ formatOptionalDecimal(row.entryOptionPrice) }}
                          <br>
                          <span class="inspectionReason">
                            underlier {{ formatOptionalDecimal(row.entryUnderlierPrice, 2) }}
                          </span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">
                        {{ formatOptionalMinutes(row.entryLagMinutes) }} / {{ formatOptionalMinutes(row.minutesToExpiryAtEntry) }}
                      </td>
                      <td class="mono">
                        <template v-if="row.exitTimestamp != null">
                          {{ formatTimestamp(row.exitTimestamp) }} · {{ formatOptionalDecimal(row.exitOptionPrice) }}
                          <br>
                          <span class="inspectionReason">
                            hold {{ formatOptionalMinutes(row.realizedHoldMinutes) }}
                          </span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">{{ row.exitReason || '—' }}</td>
                      <td class="mono">
                        <template v-if="row.pathOptionHigh != null || row.pathUnderlierHigh != null">
                          opt {{ formatOptionalDecimal(row.pathOptionLow) }} -> {{ formatOptionalDecimal(row.pathOptionHigh) }}
                          <br>
                          <span class="inspectionReason">
                            und {{ formatOptionalDecimal(row.pathUnderlierLow, 2) }} -> {{ formatOptionalDecimal(row.pathUnderlierHigh, 2) }}
                          </span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">{{ formatOptionalDecimal(row.returnOnPremium) }}</td>
                      <td class="mono">{{ formatOptionalDecimal(row.pnlPerContract) }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div v-if="selectedInspectionRow" class="inspectionDetailCard">
                <div class="panelSectionHeader">
                  <div>
                    <div class="panelEyebrow">Selected event</div>
                    <div class="panelSectionLead">
                      Inspect the chosen event row without leaving the current tuple drilldown.
                    </div>
                  </div>
                </div>
                <div class="actions">
                  <button
                    v-if="selectedInspectionRow.status === 'executed'"
                    type="button"
                    :disabled="eventPathBusy"
                    @click="loadSelectedEventPath()"
                  >
                    {{ eventPathBusy ? 'Loading path…' : 'Reload path replay' }}
                  </button>
                </div>
                <div class="detailGrid detailGridTight">
                  <div>
                    <span class="panelMetricKey">Event / split / side</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.eventId }} / {{ selectedInspectionRow.split }} / {{ selectedInspectionRow.side }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Status / reason</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.status }}{{ selectedInspectionRow.candidateInvalidReason ? ` / ${selectedInspectionRow.candidateInvalidReason}` : '' }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Contract</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.contractId || '—' }}{{ selectedInspectionRow.strike != null ? ` @ ${formatOptionalDecimal(selectedInspectionRow.strike, 2)}` : '' }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Expiry / multiplier</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.expirationDate || '—' }} / {{ selectedInspectionRow.multiplier == null ? '—' : selectedInspectionRow.multiplier }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Decision / tradable</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatTimestamp(selectedInspectionRow.decisionTimestamp) }} / {{ formatTimestamp(selectedInspectionRow.earliestTradableTimestamp) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Requested / base exit</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatTimestamp(selectedInspectionRow.requestedEntryTimestamp) }} / {{ formatTimestamp(selectedInspectionRow.baseExitTimestamp) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Entry / exit</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatTimestamp(selectedInspectionRow.entryTimestamp) }} / {{ formatTimestamp(selectedInspectionRow.exitTimestamp) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Lag / expiry / hold</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatOptionalMinutes(selectedInspectionRow.entryLagMinutes) }} / {{ formatOptionalMinutes(selectedInspectionRow.minutesToExpiryAtEntry) }} / {{ formatOptionalMinutes(selectedInspectionRow.realizedHoldMinutes) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Entry underlier / option</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatOptionalDecimal(selectedInspectionRow.entryUnderlierPrice, 2) }} / {{ formatOptionalDecimal(selectedInspectionRow.entryOptionPrice) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Exit option / reason</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatOptionalDecimal(selectedInspectionRow.exitOptionPrice) }} / {{ selectedInspectionRow.exitReason || '—' }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Path range</span>
                    <span class="panelMetricValue mono panelNumeric">
                      opt {{ formatOptionalDecimal(selectedInspectionRow.pathOptionLow) }} -> {{ formatOptionalDecimal(selectedInspectionRow.pathOptionHigh) }} / und {{ formatOptionalDecimal(selectedInspectionRow.pathUnderlierLow, 2) }} -> {{ formatOptionalDecimal(selectedInspectionRow.pathUnderlierHigh, 2) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Premium / PnL / return</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ formatOptionalDecimal(selectedInspectionRow.premiumPaid) }} / {{ formatOptionalDecimal(selectedInspectionRow.pnlPerContract) }} / {{ formatOptionalDecimal(selectedInspectionRow.returnOnPremium) }}
                    </span>
                  </div>
                </div>
                <div v-if="selectedEventPathInspection" class="detailInspector">
                  <div class="panelSectionHeader">
                    <div>
                      <div class="panelEyebrow">Event path replay</div>
                      <div class="panelSectionLead">
                        Replayed option OHLC and underlier path for the selected executed event.
                      </div>
                    </div>
                  </div>
                  <div class="detailGrid detailGridTight">
                    <div v-for="row in eventPathSummaryRows" :key="row.key">
                      <span class="panelMetricKey">{{ row.label }}</span>
                      <span class="panelMetricValue mono panelNumeric">{{ row.value }}</span>
                    </div>
                  </div>
                  <div class="tableWrap">
                    <table class="resultTable">
                      <thead>
                        <tr>
                          <th>Timestamp</th>
                          <th>Option O/H/L/C</th>
                          <th>Underlier</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr
                          v-for="point in selectedEventPathInspection.points"
                          :key="`${selectedEventPathInspection.eventId}-${point.timestampMs}`"
                        >
                          <td class="mono">{{ formatTimestamp(point.timestampMs) }}</td>
                          <td class="mono">
                            {{ formatOptionalDecimal(point.optionOpen) }} /
                            {{ formatOptionalDecimal(point.optionHigh) }} /
                            {{ formatOptionalDecimal(point.optionLow) }} /
                            {{ formatOptionalDecimal(point.optionClose) }}
                          </td>
                          <td class="mono">{{ formatOptionalDecimal(point.underlyingPrice, 2) }}</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            </div>
          </details>
        </div>
        <div v-else class="emptyBody">
          No ranked tuples available yet. Run an options expression job or load a saved artifact.
        </div>
      </section>
    </template>

    <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
  </section>
</template>

<style scoped>
.optionsPanel {
  display: grid;
  gap: 0.85rem;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--panel-footer);
  padding: 0.9rem;
}

.optionsGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.controlBlock {
  display: grid;
  gap: 0.35rem;
}

.inlineControls {
  display: flex;
  gap: 0.55rem;
  align-items: center;
}

.resultTableControls {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  align-items: end;
}

.resultTableControls label {
  display: grid;
  gap: 0.3rem;
  font-size: 0.82rem;
}

.toggleField {
  display: inline-flex !important;
  grid-auto-flow: column;
  align-items: center;
  gap: 0.45rem;
  align-self: center;
  padding-top: 1.2rem;
}

.toggleField input {
  width: auto;
}

.inlineControls > * {
  min-width: 0;
}

.downloadFieldGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.7rem;
}

.optionsFields {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.optionsFields label,
.controlBlock label {
  display: grid;
  gap: 0.3rem;
  font-size: 0.82rem;
}

.optionsFields .full {
  grid-column: 1 / -1;
}

.moneynessList {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.bucketToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.45rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--panel);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
}

.gateBadgeRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.gateBadge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  padding: 0.3rem 0.55rem;
  font-size: 0.76rem;
  background: color-mix(in srgb, var(--panel) 82%, transparent);
}

.gateBadge.isPass {
  border-color: color-mix(in srgb, var(--success, #4caf50) 45%, var(--border));
  color: color-mix(in srgb, var(--success, #4caf50) 78%, white);
}

.gateBadge.isPending {
  border-color: color-mix(in srgb, var(--accent, #4da3ff) 35%, var(--border));
  color: color-mix(in srgb, var(--accent, #4da3ff) 80%, white);
}

.gateBadge.isMixed {
  border-color: color-mix(in srgb, var(--warning, #f0b429) 45%, var(--border));
  color: color-mix(in srgb, var(--warning, #f0b429) 85%, white);
}

.coverageSection {
  display: grid;
  gap: 0.55rem;
}

.warningText {
  color: var(--warning, #f0b429);
  font-size: 0.78rem;
}

.resultCard {
  display: grid;
  gap: 0.75rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.detailGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.detailGridTight {
  gap: 0.55rem;
}

.tableWrap {
  overflow: auto;
}

.resultTable {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.78rem;
}

.resultTable th,
.resultTable td {
  border-bottom: 1px solid var(--border);
  padding: 0.45rem 0.4rem;
  text-align: left;
}

.resultTable tbody tr {
  cursor: pointer;
}

.resultTable tbody tr.isSelected {
  background: color-mix(in srgb, var(--accent, #4da3ff) 12%, transparent);
}

.inspectionReason {
  color: color-mix(in oklab, var(--text-muted) 86%, var(--warning, #f0b429));
}

.emptyState {
  display: grid;
  gap: 0.35rem;
  padding: 0.7rem 0.2rem 0.1rem;
}

.emptyTitle {
  font-weight: 600;
}

.emptyBody {
  color: var(--muted);
  font-size: 0.84rem;
}

.detailInspector {
  display: grid;
  gap: 0.7rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.recommendationList {
  display: grid;
  gap: 0.45rem;
  margin: 0;
  padding-left: 1.1rem;
}

.inspectionDetailCard {
  display: grid;
  gap: 0.7rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

input,
select {
  width: 100%;
}

@media (max-width: 980px) {
  .optionsGrid,
  .downloadFieldGrid,
  .optionsFields,
  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>
