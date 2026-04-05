<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { applyRecommendedOptimizationDefaults } from '../services/recommendedOptimizationDefaults'
import { useAdvancedStore, type AdvancedOption } from '../stores/advancedStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useStrategyStore } from '../stores/strategyStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { describeStrategyOptimizationCapabilities } from '../utils/strategyOptimizationCapabilities'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

interface GuidanceItem {
  key: string
  title: string
  detail: string
}

const advancedStore = useAdvancedStore()
const optimizeStore = useOptimizeStore()
const strategyStore = useStrategyStore()
const workflowStore = useWorkflowStore()
const {
  maSource,
  maSourceOptions,
  objective,
  objectiveOptions,
  scoreMode,
  hurdleTau,
  metricContract,
  includeTradeMetrics,
  extraShortlistObjectives,
  shortlistConfidenceMode,
  shortlistUnionMode,
  shortlistSliceMode,
  shortlistSliceBenchmark,
  shortlistNeighborhoodRadius,
  shortlistNeighborhoodStep,
  mode,
  vdpsVisible,
  vdpsTelemetryHint,
  compositeMoVisible,
  heatmapBins,
  topK,
  includeAll,
  includeAllOptions,
  exportAllCsvPath,
  canBrowseExportAll,
  canResetMoSearchSpace,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(advancedStore)
const { controlsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#advancedVueShellMount', {
  paneSelector: '[data-vue-panel="advanced"]',
  paneDatasetKey: 'vueAdvanced',
})

const maSourceDraft = ref('close')
const objectiveDraft = ref('Sharpe')
const scoreModeDraft = ref('')
const hurdleTauDraft = ref('')
const metricContractDraft = ref('auto')
const extraShortlistObjectivesDraft = ref('')
const shortlistConfidenceModeDraft = ref('')
const shortlistUnionModeDraft = ref('')
const shortlistSliceModeDraft = ref('')
const shortlistSliceBenchmarkDraft = ref('')
const shortlistNeighborhoodRadiusDraft = ref('')
const shortlistNeighborhoodStepDraft = ref('')
const modeDraft = ref('Grid')
const heatmapBinsDraft = ref('128')
const topKDraft = ref('20')
const includeAllDraft = ref('false')
const exportPathDraft = ref('')

const visible = computed(() => String(controlsTab.value || '').trim() === 'advanced')
const strategyCapabilities = computed(() => describeStrategyOptimizationCapabilities(
  optimizeStore.strategyKind,
  {
    requestedBackend: optimizeStore.backend,
    trendUseAdx: strategyStore.trendUseAdx,
    breakoutUseAdx: strategyStore.breakoutUseAdx,
  },
))
const standaloneUnifiedGpuShortlistKind = computed(() => [
  'Asr',
  'Vcc',
  'Vemr',
  'Rocb',
  'Vcet',
  'Kqsr',
  'Carm',
  'Kvdf',
  'Mkcs',
  'Cdmc',
].includes(String(strategyCapabilities.value.kind || '').trim()))
const showModeControl = computed(() => strategyCapabilities.value.usesGlobalMode)
const showHeatmapControl = computed(() => strategyCapabilities.value.supportsHeatmap)
const showTopKControl = computed(() => strategyCapabilities.value.supportsTopK)
const showIncludeAllControl = computed(() => strategyCapabilities.value.supportsIncludeAll)
const showExportControl = computed(() => strategyCapabilities.value.supportsExportAll)
const showScoreModeControl = computed(() => [
  'DoubleMa',
  'DynamicDoubleMa',
  'Pit',
  'Set',
  'Mts',
  'Orc',
  'Marrt',
  'Vfct',
  'CompositeTrend',
  'RegimeHybrid',
  'BreakoutMomentum',
].includes(String(strategyCapabilities.value.kind || '').trim()))
const extraShortlistKindSupported = computed(
  () => strategyCapabilities.value.supportsExtraShortlistObjectives,
)
const metricContractKindSupported = computed(
  () => strategyCapabilities.value.supportsMetricContract,
)
const shortlistConfidenceKindSupported = computed(
  () => strategyCapabilities.value.supportsShortlistConfidenceMode,
)
const shortlistUnionKindSupported = computed(
  () => strategyCapabilities.value.supportsShortlistUnionMode,
)
const shortlistSliceKindSupported = computed(
  () => strategyCapabilities.value.supportsAdvancedExactKernelControls,
)
const showExtraShortlistObjectivesControl = computed(() =>
  extraShortlistKindSupported.value || extraShortlistObjectivesDraft.value.trim().length > 0,
)
const showMetricContractControl = computed(() =>
  metricContractKindSupported.value || metricContractDraft.value !== 'auto',
)
const showShortlistConfidenceModeControl = computed(() =>
  shortlistConfidenceKindSupported.value || shortlistConfidenceModeDraft.value.trim().length > 0,
)
const showShortlistUnionModeControl = computed(() =>
  shortlistUnionKindSupported.value || shortlistUnionModeDraft.value.trim().length > 0,
)
const showShortlistSliceModeControl = computed(() =>
  shortlistSliceKindSupported.value || shortlistSliceModeDraft.value.trim().length > 0,
)
const showShortlistSliceBenchmarkControl = computed(() =>
  shortlistSliceKindSupported.value || shortlistSliceBenchmarkDraft.value.trim().length > 0,
)
const showShortlistNeighborhoodControls = computed(() =>
  shortlistSliceKindSupported.value
  || shortlistNeighborhoodRadiusDraft.value.trim().length > 0
  || shortlistNeighborhoodStepDraft.value.trim().length > 0,
)
const extraShortlistObjectivesStateNote = computed(() => {
  const hasValue = extraShortlistObjectivesDraft.value.trim().length > 0
  if (!extraShortlistKindSupported.value) {
    return hasValue
      ? 'The current route does not support extra shortlist objectives. Clear this field or switch to the exact GpuKernel route.'
      : 'Available on the exact Double MA GPU-kernel route only. DynamicDoubleMa can request this only on exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  if (standaloneUnifiedGpuShortlistKind.value) {
    if (String(includeAllDraft.value || '').trim() !== 'true') {
      return 'Requires include all = true so the full standalone candidate set is available on the host.'
    }
    if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
      return 'Requires top K greater than 0.'
    }
    return hasValue
      ? 'Builds additional host-side shortlist heads for the standalone unified-GPU route before any optional union or confidence rerank.'
      : 'Optional host-side extra shortlist heads for the standalone unified-GPU route. This currently requires include all = true.'
  }
  if (scoreModeDraft.value.trim().length > 0) {
    return 'Requires objective-only ranking. Clear Score mode to enable an exact union shortlist.'
  }
  if (String(includeAllDraft.value || '').trim() === 'true') {
    return 'Requires include all = false.'
  }
  if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
    return 'Requires top K greater than 0.'
  }
  if (exportPathDraft.value.trim().length > 0) {
    return 'Requires export-all CSV path to be empty.'
  }
  if (String(strategyCapabilities.value.kind || '').trim() === 'DynamicDoubleMa') {
    return 'Additional exact shortlist heads for exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) shapes. Unsupported dynamic shapes will reject or fall back cleanly.'
  }
  return 'Additional exact shortlist heads. Comma-separated objectives such as Pnl, Omega. Current primary objective is always included automatically.'
})
const metricContractStateNote = computed(() => {
  const current = String(metricContractDraft.value || 'auto').trim() || 'auto'
  const hasExplicitValue = current !== 'auto'
  if (!metricContractKindSupported.value) {
    return hasExplicitValue
      ? 'The current route does not support explicit metric contracts. Clear this or switch to a route that advertises metric-contract support.'
      : 'Available only on routes that advertise metric-contract support. DynamicDoubleMa still limits this to exact-compatible GpuKernel(dynamic_strategy_v1) shapes or audited plain CPU fallback surfaces.'
  }
  if (String(strategyCapabilities.value.kind || '').trim() === 'DynamicDoubleMa') {
    if (current === 'full_17') {
      return 'Forces the full 17-metric contract on exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) requests.'
    }
    if (current === 'trade_22') {
      return 'Forces the 22-metric trade-summary contract on exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) requests.'
    }
    return 'Auto preserves the current default contract. Use 17 or 22 only when you want an explicit exact-kernel metric width.'
  }
  if (current === 'full_17') {
    return 'Forces the full 17-metric contract without trade-summary rows.'
  }
  if (current === 'trade_22') {
    return 'Forces the 22-metric trade-summary contract on the current audited route.'
  }
  return 'Auto preserves the current default contract. Use 17 or 22 only when you want an explicit canonical metric surface.'
})
const shortlistConfidenceModeStateNote = computed(() => {
  const hasValue = shortlistConfidenceModeDraft.value.trim().length > 0
  if (!shortlistConfidenceKindSupported.value) {
    return hasValue
      ? 'The current route does not support shortlist confidence mode. Clear this field or switch to the exact GpuKernel route.'
      : 'Available on the exact Double MA GPU-kernel route only. DynamicDoubleMa can request this only on exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  if (standaloneUnifiedGpuShortlistKind.value) {
    if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
      return 'Requires top K greater than 0.'
    }
    return hasValue
      ? 'Reranks the returned standalone unified-GPU shortlist by PSR or DSR confidence without adding slice or neighborhood diagnostics.'
      : 'Optional PSR / DSR rerank over the returned standalone unified-GPU shortlist.'
  }
  if (scoreModeDraft.value.trim().length > 0) {
    return 'Requires objective-only ranking. Clear Score mode to rerank the exact shortlist by PSR or DSR.'
  }
  if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
    return 'Requires top K greater than 0.'
  }
  if (exportPathDraft.value.trim().length > 0) {
    return 'Requires export-all CSV path to be empty.'
  }
  if (String(strategyCapabilities.value.kind || '').trim() === 'DynamicDoubleMa') {
    return hasValue
      ? 'Reranks the exact DynamicDoubleMa shortlist by PSR or DSR for exact-compatible GpuKernel(dynamic_strategy_v1) shapes. Unsupported dynamic shapes will reject or fall back cleanly.'
      : 'Optional PSR / DSR rerank over the DynamicDoubleMa shortlist for exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  return hasValue
    ? 'Reranks the exact returned shortlist using validation-aligned PSR or DSR confidence. Works with the primary top-K or the union shortlist.'
    : 'Optional PSR / DSR rerank over the exact returned shortlist. Leaving this off preserves the current ranking order.'
})
const shortlistUnionModeStateNote = computed(() => {
  const hasValue = shortlistUnionModeDraft.value.trim().length > 0
  if (!shortlistUnionKindSupported.value) {
    return hasValue
      ? 'The current route does not support shortlist union mode. Clear this field or switch to the exact GpuKernel route.'
      : 'Available on the exact Double MA GPU-kernel route only. DynamicDoubleMa can request this only on exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  if (extraShortlistObjectivesDraft.value.trim().length === 0) {
    return 'Requires Extra shortlist objectives to define additional exact shortlist heads.'
  }
  if (standaloneUnifiedGpuShortlistKind.value) {
    if (String(includeAllDraft.value || '').trim() !== 'true') {
      return 'Requires include all = true so the standalone union shortlist can be built from the full candidate set.'
    }
    if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
      return 'Requires top K greater than 0.'
    }
    return hasValue
      ? 'Reranks the standalone host-side union shortlist using lexicographic, weighted-rank, Pareto-front, or epsilon-front semantics.'
      : 'Optional rerank over the standalone host-side union shortlist.'
  }
  if (scoreModeDraft.value.trim().length > 0) {
    return 'Requires objective-only ranking. Clear Score mode to enable exact union reranking.'
  }
  if (String(includeAllDraft.value || '').trim() === 'true') {
    return 'Requires include all = false.'
  }
  if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
    return 'Requires top K greater than 0.'
  }
  if (exportPathDraft.value.trim().length > 0) {
    return 'Requires export-all CSV path to be empty.'
  }
  if (String(strategyCapabilities.value.kind || '').trim() === 'DynamicDoubleMa') {
    return hasValue
      ? 'Reranks the exact DynamicDoubleMa union shortlist for exact-compatible GpuKernel(dynamic_strategy_v1) shapes. Unsupported dynamic shapes will reject or fall back cleanly.'
      : 'Optional exact rerank over the DynamicDoubleMa union shortlist for exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  return hasValue
    ? 'Reranks the exact union shortlist using lexicographic, weighted-rank, Pareto-front, or epsilon-front semantics before any optional confidence or slice reranks.'
    : 'Optional exact rerank over the union shortlist. Leaving this off preserves primary-objective ordering.'
})
const shortlistSliceModeStateNote = computed(() => {
  const hasValue = shortlistSliceModeDraft.value.trim().length > 0
  if (!shortlistSliceKindSupported.value) {
    return hasValue
      ? 'The current route does not support shortlist slice mode. Clear this field or switch to the exact GpuKernel route.'
      : 'Available on the exact Double MA GPU-kernel route only.'
  }
  if (String(includeAllDraft.value || '').trim() === 'true') {
    return 'Requires include all = false.'
  }
  if (Number.parseInt(String(topKDraft.value || '0'), 10) <= 0) {
    return 'Requires top K greater than 0.'
  }
  if (exportPathDraft.value.trim().length > 0) {
    return 'Requires export-all CSV path to be empty.'
  }
  if (String(strategyCapabilities.value.kind || '').trim() === 'DynamicDoubleMa') {
    return hasValue
      ? 'Adds exact slice diagnostics to DynamicDoubleMa for exact-compatible GpuKernel(dynamic_strategy_v1) shapes. Unsupported dynamic shapes will reject or fall back cleanly.'
      : 'Optional exact slice diagnostics over the DynamicDoubleMa shortlist for exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  return hasValue
    ? 'Adds exact half-split or quartile shortlist diagnostics and optional benchmark-relative summaries for the returned shortlist.'
    : 'Optional exact slice diagnostics over the returned shortlist. Leaving this off preserves the current result surface.'
})
const shortlistSliceBenchmarkStateNote = computed(() => {
  if (shortlistSliceModeDraft.value.trim().length === 0) {
    return 'Requires shortlist slice mode.'
  }
  return shortlistSliceBenchmarkDraft.value.trim().length > 0
    ? 'Compares each slice against the chosen benchmark without replacing downstream SPA.'
    : 'Optional benchmark-relative slice summaries such as beat rate and worst-slice excess return.'
})
const shortlistNeighborhoodStateNote = computed(() => {
  if (shortlistSliceModeDraft.value.trim().length === 0) {
    return 'Requires shortlist slice mode.'
  }
  if (shortlistNeighborhoodRadiusDraft.value.trim().length === 0) {
    return 'Optional local plateau / neighborhood robustness summary around the best shortlist candidate.'
  }
  return 'Exact local neighborhood robustness summary. Radius is required; step defaults to 1 when left blank.'
})
const hurdleTauStateNote = computed(() =>
  hurdleTauDraft.value.trim().length > 0
    ? 'Optional hurdle threshold applied to Sortino, Omega, and downside-family metrics. Leaving it blank preserves the current zero-threshold semantics.'
    : 'Optional hurdle threshold. Blank or 0 preserves the current metric semantics.',
)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

function resolvedOptions(options: AdvancedOption[], fallbackValue: string, fallbackLabel: string): AdvancedOption[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const maSourceOptionItems = computed(() => resolvedOptions(maSourceOptions.value, maSource.value, maSource.value || 'close'))
const objectiveOptionItems = computed(() => resolvedOptions(objectiveOptions.value, objective.value, objective.value || 'Sharpe'))
function optionLabel(options: AdvancedOption[], nextValue: string, fallback: string): string {
  const match = options.find((option) => String(option.value || '').trim() === String(nextValue || '').trim())
  return String(match?.label || fallback || nextValue || '').trim() || fallback
}
const scoreModeOptionItems = computed(() => [
  { value: '', label: 'Objective only (default)', disabled: false },
  { value: 'balanced_risk_adjusted_10', label: 'Balanced Risk-Adjusted 10', disabled: false },
  { value: 'adversarial_envelope_10', label: 'Adversarial Envelope 10', disabled: false },
  { value: 'support_shrunk_10', label: 'Support-Shrunk 10', disabled: false },
  { value: 'active_sortino_10', label: 'Active Sortino 10', disabled: false },
  { value: 'effective_sample_lcb_10', label: 'Effective-Sample LCB 10', disabled: false },
  { value: 'omega_vs_sharpe_10', label: 'Omega vs Sharpe 10', disabled: false },
  { value: 'robust_margin_10', label: 'Robust Margin 10', disabled: false },
])
const metricContractOptionItems = computed(() => [
  { value: 'auto', label: 'Auto (default contract)', disabled: false },
  { value: 'full_17', label: 'Full 17-row metrics', disabled: false },
  { value: 'trade_22', label: 'Trade-summary 22-row metrics', disabled: false },
])
const shortlistConfidenceModeOptionItems = computed(() => [
  { value: '', label: 'Off (default)', disabled: false },
  { value: 'psr', label: 'PSR rerank', disabled: false },
  { value: 'dsr', label: 'DSR rerank', disabled: false },
])
const shortlistUnionModeOptionItems = computed(() => [
  { value: '', label: 'Primary objective order (default)', disabled: false },
  { value: 'lexicographic', label: 'Lexicographic union', disabled: false },
  { value: 'weighted_rank', label: 'Weighted-rank union', disabled: false },
  { value: 'pareto', label: 'Pareto-front union', disabled: false },
  { value: 'epsilon_front', label: 'Epsilon-front union', disabled: false },
])
const shortlistSliceModeOptionItems = computed(() => [
  { value: '', label: 'Off (default)', disabled: false },
  { value: 'halves', label: 'Halves', disabled: false },
  { value: 'quartiles', label: 'Quartiles', disabled: false },
])
const shortlistSliceBenchmarkOptionItems = computed(() => [
  { value: '', label: 'Off (default)', disabled: false },
  { value: 'flat', label: 'Flat benchmark', disabled: false },
  { value: 'buy_and_hold', label: 'Buy and Hold', disabled: false },
])
const modeOptionItems = computed(() => [{ value: 'Grid', label: 'Grid', disabled: false }])
const includeAllOptionItems = computed(() => resolvedOptions(includeAllOptions.value, includeAll.value, includeAll.value || 'false'))
const advancedSummaryChips = computed(() => {
  const modeLabel = showModeControl.value
    ? optionLabel(modeOptionItems.value, modeDraft.value, 'Grid')
    : String(strategyCapabilities.value.modeSummary || 'Route-managed')
  const chips = [
    `Route ${String(strategyCapabilities.value.kind || 'unknown').trim() || 'unknown'}`,
    `Objective ${optionLabel(objectiveOptionItems.value, objectiveDraft.value, 'Sharpe')}`,
    `Mode ${modeLabel}`,
    showTopKControl.value ? `Top K ${String(topKDraft.value || '0').trim() || '0'}` : 'Top K locked',
    showHeatmapControl.value ? `Heatmap ${String(heatmapBinsDraft.value || '0').trim() || '0'}` : 'Heatmap locked',
    exportPathDraft.value.trim().length > 0 ? 'Export-all CSV on' : 'Export-all CSV off',
  ]
  return chips
})
const activeAdvancedStateSummary = computed(() => {
  const states: string[] = []
  if (scoreModeDraft.value.trim().length > 0) {
    states.push(`Score ${optionLabel(scoreModeOptionItems.value, scoreModeDraft.value, scoreModeDraft.value)}`)
  }
  if (metricContractDraft.value !== 'auto') {
    states.push(`Metrics ${optionLabel(metricContractOptionItems.value, metricContractDraft.value, metricContractDraft.value)}`)
  }
  if (extraShortlistObjectivesDraft.value.trim().length > 0) {
    states.push(`Extra heads ${extraShortlistObjectivesDraft.value.trim()}`)
  }
  if (shortlistConfidenceModeDraft.value.trim().length > 0) {
    states.push(`Confidence ${optionLabel(shortlistConfidenceModeOptionItems.value, shortlistConfidenceModeDraft.value, shortlistConfidenceModeDraft.value)}`)
  }
  if (shortlistUnionModeDraft.value.trim().length > 0) {
    states.push(`Union ${optionLabel(shortlistUnionModeOptionItems.value, shortlistUnionModeDraft.value, shortlistUnionModeDraft.value)}`)
  }
  if (shortlistSliceModeDraft.value.trim().length > 0) {
    states.push(`Slices ${optionLabel(shortlistSliceModeOptionItems.value, shortlistSliceModeDraft.value, shortlistSliceModeDraft.value)}`)
  }
  if (shortlistNeighborhoodRadiusDraft.value.trim().length > 0) {
    states.push(`Neighborhood radius ${shortlistNeighborhoodRadiusDraft.value.trim()}`)
  }
  return states.join(' · ')
})
const rankingGuidanceItems = computed<GuidanceItem[]>(() => {
  const items: GuidanceItem[] = []
  if (showScoreModeControl.value) {
    items.push({
      key: 'score',
      title: 'Score mode',
      detail: `${shortlistConfidenceModeDraft.value.trim().length > 0 ? 'Confidence rerank remains independent of score mode. ' : ''}${hurdleTauStateNote.value}`.trim().length > 0
        ? `Composite score modes are additive opt-ins over the existing metric row. Leaving Score mode at the default preserves objective-only ranking. ${hurdleTauStateNote.value}`.trim()
        : 'Composite score modes are additive opt-ins over the existing metric row.',
    })
  }
  if (showMetricContractControl.value) {
    items.push({
      key: 'metric-contract',
      title: 'Metric contract',
      detail: metricContractStateNote.value,
    })
  }
  if (showExtraShortlistObjectivesControl.value) {
    items.push({
      key: 'extra-shortlist',
      title: 'Extra shortlist objectives',
      detail: extraShortlistObjectivesStateNote.value,
    })
  }
  if (showShortlistConfidenceModeControl.value) {
    items.push({
      key: 'confidence',
      title: 'Confidence rerank',
      detail: shortlistConfidenceModeStateNote.value,
    })
  }
  if (showShortlistUnionModeControl.value) {
    items.push({
      key: 'union',
      title: 'Union rerank',
      detail: shortlistUnionModeStateNote.value,
    })
  }
  if (showShortlistSliceModeControl.value) {
    items.push({
      key: 'slice',
      title: 'Slice diagnostics',
      detail: shortlistSliceModeStateNote.value,
    })
  }
  if (showShortlistSliceBenchmarkControl.value) {
    items.push({
      key: 'slice-benchmark',
      title: 'Slice benchmark',
      detail: shortlistSliceBenchmarkStateNote.value,
    })
  }
  if (showShortlistNeighborhoodControls.value) {
    items.push({
      key: 'neighborhood',
      title: 'Neighborhood diagnostics',
      detail: shortlistNeighborhoodStateNote.value,
    })
  }
  return items
})

watch(
  () => [
    String(maSource.value || 'close'),
    String(objective.value || 'Sharpe'),
    String(scoreMode.value || ''),
    String(hurdleTau.value || ''),
    String(metricContract.value || ''),
    Boolean(includeTradeMetrics.value),
    String(extraShortlistObjectives.value || ''),
    String(shortlistConfidenceMode.value || ''),
    String(shortlistUnionMode.value || ''),
    String(shortlistSliceMode.value || ''),
    String(shortlistSliceBenchmark.value || ''),
    String(shortlistNeighborhoodRadius.value || ''),
    String(shortlistNeighborhoodStep.value || ''),
    String(mode.value || 'Grid'),
    String(heatmapBins.value || '128'),
    String(topK.value || '20'),
    String(includeAll.value || 'false'),
    String(exportAllCsvPath.value || ''),
  ] as const,
  ([
    nextMaSource,
    nextObjective,
    nextScoreMode,
    nextHurdleTau,
    nextMetricContractRaw,
    nextIncludeTradeMetrics,
    nextExtraShortlistObjectives,
    nextShortlistConfidenceMode,
    nextShortlistUnionMode,
    nextShortlistSliceMode,
    nextShortlistSliceBenchmark,
    nextShortlistNeighborhoodRadius,
    nextShortlistNeighborhoodStep,
    nextMode,
    nextHeatmapBins,
    nextTopK,
    nextIncludeAll,
    nextExportPath,
  ]) => {
    if (maSourceDraft.value !== nextMaSource) maSourceDraft.value = nextMaSource
    if (objectiveDraft.value !== nextObjective) objectiveDraft.value = nextObjective
    if (scoreModeDraft.value !== nextScoreMode) scoreModeDraft.value = nextScoreMode
    if (hurdleTauDraft.value !== nextHurdleTau) hurdleTauDraft.value = nextHurdleTau
    const nextMetricContract =
      (String(nextMetricContractRaw || '').trim() || '')
      || (nextIncludeTradeMetrics ? 'trade_22' : 'auto')
    if (metricContractDraft.value !== nextMetricContract) metricContractDraft.value = nextMetricContract
    if (extraShortlistObjectivesDraft.value !== nextExtraShortlistObjectives) {
      extraShortlistObjectivesDraft.value = nextExtraShortlistObjectives
    }
    if (shortlistConfidenceModeDraft.value !== nextShortlistConfidenceMode) {
      shortlistConfidenceModeDraft.value = nextShortlistConfidenceMode
    }
    if (shortlistUnionModeDraft.value !== nextShortlistUnionMode) {
      shortlistUnionModeDraft.value = nextShortlistUnionMode
    }
    if (shortlistSliceModeDraft.value !== nextShortlistSliceMode) {
      shortlistSliceModeDraft.value = nextShortlistSliceMode
    }
    if (shortlistSliceBenchmarkDraft.value !== nextShortlistSliceBenchmark) {
      shortlistSliceBenchmarkDraft.value = nextShortlistSliceBenchmark
    }
    if (shortlistNeighborhoodRadiusDraft.value !== nextShortlistNeighborhoodRadius) {
      shortlistNeighborhoodRadiusDraft.value = nextShortlistNeighborhoodRadius
    }
    if (shortlistNeighborhoodStepDraft.value !== nextShortlistNeighborhoodStep) {
      shortlistNeighborhoodStepDraft.value = nextShortlistNeighborhoodStep
    }
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
    if (heatmapBinsDraft.value !== nextHeatmapBins) heatmapBinsDraft.value = nextHeatmapBins
    if (topKDraft.value !== nextTopK) topKDraft.value = nextTopK
    if (includeAllDraft.value !== nextIncludeAll) includeAllDraft.value = nextIncludeAll
    if (exportPathDraft.value !== nextExportPath) exportPathDraft.value = nextExportPath
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  advancedStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  advancedStore.setSelectField(id, nextValue)
}

onMounted(() => {
  applyRecommendedOptimizationDefaults()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#advancedVueShellMount">
    <section class="advancedPanelVueRoot advancedVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Diagnostics</div>
          <h2 class="panelTitle">Advanced Controls</h2>
          <div class="panelSubtitle">
            Shape ranking, shortlist diagnostics, and output surfaces without changing the underlying strategy module.
          </div>
        </div>
        <div class="meta mono">{{ stamp }}</div>
      </header>
      <div class="panelSummaryChips" aria-label="Advanced summary">
        <span v-for="chip in advancedSummaryChips" :key="`advanced_summary_${chip}`" class="panelSummaryChip mono">
          {{ chip }}
        </span>
      </div>
      <div v-if="activeAdvancedStateSummary" class="panelMuted mono">
        Active advanced state: {{ activeAdvancedStateSummary }}
      </div>

      <section class="pane panelProminentPane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Ranking</div>
            <h3 class="panelPaneTitle">Optimization setup</h3>
            <div class="panelSectionLead">
              Set the objective, route mode, and exact-kernel diagnostics that should influence shortlist ordering.
            </div>
          </div>
        </div>
        <div class="gridThree">
          <div>
            <label for="vueMaSource">MA price source</label>
            <select
              id="vueMaSource"
              :value="maSourceDraft"
              @change="maSourceDraft = String(($event.target as HTMLSelectElement).value || 'close'); setSelectValue('maSource', maSourceDraft)"
            >
              <option
                v-for="option in maSourceOptionItems"
                :key="`ma_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div>
            <label for="vueObjective">Objective</label>
            <select
              id="vueObjective"
              :value="objectiveDraft"
              @change="objectiveDraft = String(($event.target as HTMLSelectElement).value || 'Sharpe'); setSelectValue('objective', objectiveDraft)"
            >
              <option
                v-for="option in objectiveOptionItems"
                :key="`objective_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div>
            <template v-if="showModeControl">
              <label for="vueMode">Mode</label>
              <select
                id="vueMode"
                :value="modeDraft"
                @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'Grid'); setSelectValue('mode', modeDraft)"
              >
                <option
                  v-for="option in modeOptionItems"
                  :key="`mode_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </template>
            <template v-else>
              <label>Mode</label>
              <div class="muted mono">{{ strategyCapabilities.modeSummary }}</div>
            </template>
          </div>
        </div>
        <div v-if="showScoreModeControl" class="gridThree stackBlock">
          <div>
            <label for="vueScoreMode">Score mode</label>
            <select
              id="vueScoreMode"
              :value="scoreModeDraft"
              @change="scoreModeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('scoreMode', scoreModeDraft)"
            >
              <option
                v-for="option in scoreModeOptionItems"
                :key="`score_mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueHurdleTau">Hurdle tau</label>
            <input
              id="vueHurdleTau"
              type="text"
              :value="hurdleTauDraft"
              placeholder="0.0"
              @input="hurdleTauDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('hurdleTau', hurdleTauDraft)"
              @change="commitInputValue('hurdleTau', hurdleTauDraft)"
            />
          </div>
        </div>
        <div v-if="showMetricContractControl" class="gridThree stackBlock">
          <div>
            <label for="vueMetricContract">Metric contract</label>
            <select
              id="vueMetricContract"
              :value="metricContractDraft"
              @change="
                metricContractDraft = String(($event.target as HTMLSelectElement).value || 'auto');
                setSelectValue('metricContract', metricContractDraft);
                advancedStore.setCheckboxField('includeTradeMetrics', metricContractDraft === 'trade_22');
              "
            >
              <option
                v-for="option in metricContractOptionItems"
                :key="`metric_contract_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div v-if="showExtraShortlistObjectivesControl" class="gridThree stackBlock">
          <div>
            <label for="vueExtraShortlistObjectives">Extra shortlist objectives</label>
            <input
              id="vueExtraShortlistObjectives"
              type="text"
              :value="extraShortlistObjectivesDraft"
              placeholder="Pnl, Omega"
              @input="extraShortlistObjectivesDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('extraShortlistObjectives', extraShortlistObjectivesDraft)"
              @change="commitInputValue('extraShortlistObjectives', extraShortlistObjectivesDraft)"
            />
          </div>
        </div>
        <div v-if="showShortlistConfidenceModeControl" class="gridThree stackBlock">
          <div>
            <label for="vueShortlistConfidenceMode">Shortlist confidence mode</label>
            <select
              id="vueShortlistConfidenceMode"
              :value="shortlistConfidenceModeDraft"
              @change="shortlistConfidenceModeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('shortlistConfidenceMode', shortlistConfidenceModeDraft)"
            >
              <option
                v-for="option in shortlistConfidenceModeOptionItems"
                :key="`shortlist_confidence_mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div v-if="showShortlistUnionModeControl" class="gridThree stackBlock">
          <div>
            <label for="vueShortlistUnionMode">Shortlist union mode</label>
            <select
              id="vueShortlistUnionMode"
              :value="shortlistUnionModeDraft"
              @change="shortlistUnionModeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('shortlistUnionMode', shortlistUnionModeDraft)"
            >
              <option
                v-for="option in shortlistUnionModeOptionItems"
                :key="`shortlist_union_mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div v-if="showShortlistSliceModeControl" class="gridThree stackBlock">
          <div>
            <label for="vueShortlistSliceMode">Shortlist slice mode</label>
            <select
              id="vueShortlistSliceMode"
              :value="shortlistSliceModeDraft"
              @change="shortlistSliceModeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('shortlistSliceMode', shortlistSliceModeDraft)"
            >
              <option
                v-for="option in shortlistSliceModeOptionItems"
                :key="`shortlist_slice_mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div v-if="showShortlistSliceBenchmarkControl" class="gridThree stackBlock">
          <div>
            <label for="vueShortlistSliceBenchmark">Shortlist slice benchmark</label>
            <select
              id="vueShortlistSliceBenchmark"
              :value="shortlistSliceBenchmarkDraft"
              @change="shortlistSliceBenchmarkDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('shortlistSliceBenchmark', shortlistSliceBenchmarkDraft)"
            >
              <option
                v-for="option in shortlistSliceBenchmarkOptionItems"
                :key="`shortlist_slice_benchmark_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
        <div v-if="showShortlistNeighborhoodControls" class="gridThree stackBlock">
          <div>
            <label for="vueShortlistNeighborhoodRadius">Neighborhood radius</label>
            <input
              id="vueShortlistNeighborhoodRadius"
              type="text"
              :value="shortlistNeighborhoodRadiusDraft"
              placeholder="1"
              @input="shortlistNeighborhoodRadiusDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('shortlistNeighborhoodRadius', shortlistNeighborhoodRadiusDraft)"
              @change="commitInputValue('shortlistNeighborhoodRadius', shortlistNeighborhoodRadiusDraft)"
            />
          </div>
          <div>
            <label for="vueShortlistNeighborhoodStep">Neighborhood step</label>
            <input
              id="vueShortlistNeighborhoodStep"
              type="text"
              :value="shortlistNeighborhoodStepDraft"
              placeholder="1"
              @input="shortlistNeighborhoodStepDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('shortlistNeighborhoodStep', shortlistNeighborhoodStepDraft)"
              @change="commitInputValue('shortlistNeighborhoodStep', shortlistNeighborhoodStepDraft)"
            />
          </div>
        </div>
        <details v-if="rankingGuidanceItems.length" class="panelDetails advancedGuidanceDetails">
          <summary>Ranking guidance</summary>
          <div class="panelDetailsBody guidanceList">
            <div v-for="item in rankingGuidanceItems" :key="item.key" class="guidanceItem">
              <div class="guidanceTitle">{{ item.title }}</div>
              <div class="panelMuted">{{ item.detail }}</div>
            </div>
          </div>
        </details>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Output</div>
            <h3 class="panelPaneTitle">Output shaping</h3>
            <div class="panelSectionLead">
              Control how much of the shortlist comes back, whether a heatmap is emitted, and when a full export should be written.
            </div>
          </div>
        </div>
        <div class="gridThree">
          <div v-if="showHeatmapControl">
            <label for="vueHeatmapBins">Heatmap bins (0 = off)</label>
            <input
              id="vueHeatmapBins"
              type="number"
              min="0"
              step="1"
              :value="heatmapBinsDraft"
              @input="heatmapBinsDraft = String(($event.target as HTMLInputElement).value || '128'); queueInputValue('heatmapBins', heatmapBinsDraft)"
              @change="commitInputValue('heatmapBins', heatmapBinsDraft)"
            />
          </div>
          <div v-if="showTopKControl">
            <label for="vueTopK">Return top K (0 = best only)</label>
            <input
              id="vueTopK"
              type="number"
              min="0"
              step="1"
              :value="topKDraft"
              @input="topKDraft = String(($event.target as HTMLInputElement).value || '20'); queueInputValue('topK', topKDraft)"
              @change="commitInputValue('topK', topKDraft)"
            />
          </div>
          <div v-if="showIncludeAllControl">
            <label for="vueIncludeAll">Return all results</label>
            <select
              id="vueIncludeAll"
              :value="includeAllDraft"
              @change="includeAllDraft = String(($event.target as HTMLSelectElement).value || 'false'); setSelectValue('includeAll', includeAllDraft)"
            >
              <option
                v-for="option in includeAllOptionItems"
                :key="`include_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <template v-if="showExportControl">
          <label for="vueExportAllCsvPath">Export per-pair metrics CSV path (optional)</label>
          <div class="pathRow">
            <input
              id="vueExportAllCsvPath"
              type="text"
              :value="exportPathDraft"
              placeholder="C:\\path\\to\\all_pairs.csv"
              @input="exportPathDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('exportAllCsvPath', exportPathDraft)"
              @change="commitInputValue('exportAllCsvPath', exportPathDraft)"
            />
            <button type="button" :disabled="!canBrowseExportAll" @click="advancedStore.browseExportAll()">Browse</button>
          </div>
        </template>
        <div v-if="!showHeatmapControl || !showTopKControl || !showIncludeAllControl || !showExportControl" class="muted mono">
          {{ strategyCapabilities.outputSummary }}
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Sampler state</div>
            <h3 class="panelPaneTitle">Strategy samplers</h3>
            <div class="panelSectionLead">
              Review sampler visibility and reset any strategy-specific search-space overrides that should not carry into the next run.
            </div>
          </div>
        </div>
        <div class="metaLine mono">VDPS panel: {{ vdpsVisible ? 'visible' : 'hidden' }}</div>
        <div class="metaLine mono">Composite-MO panel: {{ compositeMoVisible ? 'visible' : 'hidden' }}</div>
        <div class="muted">{{ vdpsTelemetryHint }}</div>
        <div class="pathRow samplerActionRow">
          <div></div>
          <button type="button" :disabled="!canResetMoSearchSpace" @click="advancedStore.resetMoSearchSpace()">
            Reset MO search-space JSON
          </button>
        </div>
      </section>

      <div class="status mono" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.advancedPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane :is(h3, h4) {
  margin: 0.1rem 0 0;
}

.gridThree {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.65rem;
}

.stackBlock {
  margin-top: 0.75rem;
}

.panelInlineDetails {
  margin-top: 0.15rem;
}

.advancedGuidanceDetails {
  margin-top: 0.75rem;
}

.guidanceList {
  display: grid;
  gap: 0.65rem;
}

.guidanceItem {
  display: grid;
  gap: 0.2rem;
}

.guidanceTitle {
  font-size: 0.76rem;
  font-weight: 650;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  color: var(--muted-strong);
}

.pathRow {
  margin-top: 0.4rem;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.samplerActionRow {
  margin-top: 0.55rem;
}

.checkboxRow {
  margin-top: 0.4rem;
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.spanTwo {
  grid-column: span 2;
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  margin-top: 0.4rem;
  color: var(--muted);
  font-size: 0.8rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .gridThree,
  .pathRow {
    grid-template-columns: 1fr;
  }

  .spanTwo {
    grid-column: auto;
  }
}
</style>



