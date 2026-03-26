<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { applyRecommendedOptimizationDefaults } from '../services/recommendedOptimizationDefaults'
import { useAdvancedStore, type AdvancedOption } from '../stores/advancedStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useStrategyStore } from '../stores/strategyStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { describeStrategyOptimizationCapabilities } from '../utils/strategyOptimizationCapabilities'

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

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)
const adoptedNodes: { node: HTMLElement; parent: Node; next: ChildNode | null }[] = []
const compatCleanupFns: Array<() => void> = []
const compatObservers: MutationObserver[] = []

const maSourceDraft = ref('close')
const objectiveDraft = ref('Sharpe')
const scoreModeDraft = ref('')
const hurdleTauDraft = ref('')
const includeTradeMetricsDraft = ref(false)
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
const tradeMetricsKindSupported = computed(
  () => strategyCapabilities.value.supportsTradeMetrics,
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
const showIncludeTradeMetricsControl = computed(() =>
  tradeMetricsKindSupported.value || includeTradeMetricsDraft.value,
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
const includeTradeMetricsStateNote = computed(() => {
  const hasValue = includeTradeMetricsDraft.value
  if (!tradeMetricsKindSupported.value) {
    return hasValue
      ? 'The current route does not support trade metrics. Turn this off or switch to the exact GpuKernel route.'
      : 'Available on the exact Double MA GPU-kernel route only. DynamicDoubleMa can request this only on exact-compatible GpuKernel(dynamic_strategy_v1) shapes.'
  }
  if (String(strategyCapabilities.value.kind || '').trim() === 'DynamicDoubleMa') {
    return hasValue
      ? 'Adds trade-summary metrics to exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) requests. Unsupported dynamic shapes will reject or fall back cleanly.'
      : 'Optional trade-summary metrics for exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) requests.'
  }
  return hasValue
    ? 'Adds trade-summary metrics to the one-pass optimization request. This currently requires the exact GpuKernel route; hybrid and CPU fallback routes will reject it.'
    : 'Optional trade-summary metrics. Leaving this off preserves the current default request surface.'
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

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function resolvedOptions(options: AdvancedOption[], fallbackValue: string, fallbackLabel: string): AdvancedOption[] {
  if (options.length > 0) return options
  const value = String(fallbackValue || '').trim()
  if (!value) return []
  return [{ value, label: String(fallbackLabel || value).trim() || value, disabled: false }]
}

const maSourceOptionItems = computed(() => resolvedOptions(maSourceOptions.value, maSource.value, maSource.value || 'close'))
const objectiveOptionItems = computed(() => resolvedOptions(objectiveOptions.value, objective.value, objective.value || 'Sharpe'))
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

watch(
  () => [
    String(maSource.value || 'close'),
    String(objective.value || 'Sharpe'),
    String(scoreMode.value || ''),
    String(hurdleTau.value || ''),
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
    if (includeTradeMetricsDraft.value !== nextIncludeTradeMetrics) {
      includeTradeMetricsDraft.value = nextIncludeTradeMetrics
    }
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

function clearCompatListeners(): void {
  while (compatCleanupFns.length > 0) {
    const cleanup = compatCleanupFns.pop()
    if (!cleanup) continue
    cleanup()
  }
}

function clearCompatObservers(): void {
  while (compatObservers.length > 0) {
    const observer = compatObservers.pop()
    if (!observer) continue
    observer.disconnect()
  }
}

function queryLegacyById<T extends Element>(id: string): T | null {
  const selector = `[id="${id}"]`
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  if (panel) {
    const scoped = panel.querySelector(selector) as T | null
    if (scoped) return scoped
  }
  const byQuery = typeof document.querySelector === 'function'
    ? document.querySelector(selector)
    : null
  if (byQuery) return byQuery as T
  const getById = (document as unknown as { getElementById?: (nextId: string) => Element | null }).getElementById
  if (typeof getById !== 'function') return null
  return (getById.call(document, id) as T | null) ?? null
}

function legacyAdvancedControls(): Array<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement> {
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  if (!panel) return []
  return Array.from(panel.querySelectorAll('input[id], select[id], textarea[id]'))
    .filter((el) => !String((el as HTMLElement).id || '').startsWith('vue'))
    .map((el) => el as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement)
}

function optionsById(id: string): AdvancedOption[] {
  const el = queryLegacyById<HTMLSelectElement>(id)
  if (!el) return []
  return Array.from(el.options)
    .map((option) => {
      const value = String(option.value || '').trim()
      const label = String(option.textContent || option.value || '').trim()
      return {
        value: value || label,
        label: label || value,
        disabled: !!option.disabled,
      }
    })
    .filter((entry) => entry.value.length > 0 || entry.label.length > 0)
}

function optionsEqual(left: AdvancedOption[], right: AdvancedOption[]): boolean {
  if (left.length !== right.length) return false
  for (let i = 0; i < left.length; i += 1) {
    const l = left[i]
    const r = right[i]
    if (!l || !r) return false
    if (l.value !== r.value || l.label !== r.label || l.disabled !== r.disabled) return false
  }
  return true
}

function syncLegacyAdvancedSelectOptions(): void {
  const maOptions = optionsById('maSource')
  const objectiveOptionsNext = optionsById('objective')
  const modeOptionsNext = optionsById('mode')
  const includeAllOptionsNext = optionsById('includeAll')
  advancedStore.$patch((state) => {
    let changed = false
    if (maOptions.length > 0 && !optionsEqual(state.maSourceOptions, maOptions)) {
      state.maSourceOptions = maOptions
      changed = true
    }
    if (objectiveOptionsNext.length > 0 && !optionsEqual(state.objectiveOptions, objectiveOptionsNext)) {
      state.objectiveOptions = objectiveOptionsNext
      changed = true
    }
    if (modeOptionsNext.length > 0 && !optionsEqual(state.modeOptions, modeOptionsNext)) {
      state.modeOptions = modeOptionsNext
      changed = true
    }
    if (includeAllOptionsNext.length > 0 && !optionsEqual(state.includeAllOptions, includeAllOptionsNext)) {
      state.includeAllOptions = includeAllOptionsNext
      changed = true
    }
    if (changed) state.updatedAtUnixMs = Date.now()
  })
}

function syncLegacyAdvancedUiMeta(): void {
  const vdpsWrap = queryLegacyById<HTMLElement>('vdpsConfigWrap')
  const compositeWrap = queryLegacyById<HTMLElement>('compositeMoWrap')
  const telemetryNode = queryLegacyById<HTMLElement>('vdpsTelemetryHint')
  const statusNode = queryLegacyById<HTMLElement>('status')
  const exportInput = queryLegacyById<HTMLInputElement>('exportAllCsvPath')
  const canReset = String(document.documentElement.dataset.advCanResetMoSearchSpace || '').trim() === '1'
  const vdpsVisibleNext = !!vdpsWrap && window.getComputedStyle(vdpsWrap).display !== 'none'
  const compositeVisibleNext = !!compositeWrap && window.getComputedStyle(compositeWrap).display !== 'none'
  const telemetryText = String(telemetryNode?.textContent || '').trim() || 'VDPS telemetry appears after a run.'
  const statusText = String(statusNode?.textContent || '').trim() || 'Ready.'
  const canBrowseNext = exportInput ? !exportInput.disabled : true

  advancedStore.$patch((state) => {
    let changed = false
    if (state.vdpsVisible !== vdpsVisibleNext) {
      state.vdpsVisible = vdpsVisibleNext
      changed = true
    }
    if (state.compositeMoVisible !== compositeVisibleNext) {
      state.compositeMoVisible = compositeVisibleNext
      changed = true
    }
    if (state.vdpsTelemetryHint !== telemetryText) {
      state.vdpsTelemetryHint = telemetryText
      changed = true
    }
    if (state.statusLine !== statusText) {
      state.statusLine = statusText
      changed = true
    }
    if (state.canBrowseExportAll !== canBrowseNext) {
      state.canBrowseExportAll = canBrowseNext
      changed = true
    }
    if (state.canResetMoSearchSpace !== canReset) {
      state.canResetMoSearchSpace = canReset
      changed = true
    }
    if (changed) state.updatedAtUnixMs = Date.now()
  })
}

function seedLegacyAdvancedInputsToStore(): void {
  const storeMap = advancedStore as unknown as Record<string, unknown>
  for (const control of legacyAdvancedControls()) {
    const id = String(control.id || '').trim()
    if (!id || !(id in storeMap)) continue
    const current = storeMap[id]
    if (typeof current === 'boolean' && control instanceof HTMLInputElement && control.type === 'checkbox') {
      advancedStore.setCheckboxField(id, !!control.checked)
      continue
    }
    if (typeof current !== 'string') continue
    const nextValue = String(control.value || '')
    if (String(current) === nextValue) continue
    if (control instanceof HTMLSelectElement) {
      advancedStore.setSelectField(id, nextValue)
    } else {
      advancedStore.setInputField(id, nextValue)
    }
  }
}

function syncLegacyAdvancedInputsFromStore(): void {
  const storeMap = advancedStore as unknown as Record<string, unknown>
  for (const control of legacyAdvancedControls()) {
    const id = String(control.id || '').trim()
    if (!id || !(id in storeMap)) continue
    const nextValue = storeMap[id]
    if (typeof nextValue === 'boolean' && control instanceof HTMLInputElement && control.type === 'checkbox') {
      if (control.checked !== nextValue) control.checked = nextValue
      continue
    }
    if (typeof nextValue !== 'string') continue
    if (String(control.value || '') === nextValue) continue
    control.value = nextValue
  }
}

function bindLegacyAdvancedInputs(): void {
  clearCompatListeners()
  for (const control of legacyAdvancedControls()) {
    const id = String(control.id || '').trim()
    if (!id) continue
    const onChange = (event: Event) => {
      const target = event.target as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
      if (!target) return
      if (target instanceof HTMLInputElement && target.type === 'checkbox') {
        advancedStore.setCheckboxField(id, !!target.checked)
      } else if (target instanceof HTMLSelectElement) {
        advancedStore.setSelectField(id, String(target.value || ''))
      } else {
        advancedStore.setInputField(id, String(target.value || ''))
      }
      syncLegacyAdvancedUiMeta()
      syncLegacyAdvancedSelectOptions()
    }
    control.addEventListener('input', onChange)
    control.addEventListener('change', onChange)
    compatCleanupFns.push(() => {
      control.removeEventListener('input', onChange)
      control.removeEventListener('change', onChange)
    })
  }
}

function bindLegacyAdvancedObservers(): void {
  clearCompatObservers()

  const textObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  for (const id of ['vdpsTelemetryHint', 'status'] as const) {
    const node = queryLegacyById<HTMLElement>(id)
    if (!node) continue
    textObserver.observe(node, { childList: true, characterData: true, subtree: true })
  }
  compatObservers.push(textObserver)

  const rootObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  rootObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-controls-tab', 'data-running', 'data-adv-can-reset-mo-search-space'],
  })
  compatObservers.push(rootObserver)

  const visibilityObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  for (const id of ['vdpsConfigWrap', 'compositeMoWrap'] as const) {
    const node = queryLegacyById<HTMLElement>(id)
    if (!node) continue
    visibilityObserver.observe(node, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }
  compatObservers.push(visibilityObserver)

  const optionObserver = new MutationObserver(() => {
    syncLegacyAdvancedSelectOptions()
  })
  for (const id of ['maSource', 'objective', 'mode', 'includeAll'] as const) {
    const node = queryLegacyById<HTMLElement>(id)
    if (!node) continue
    optionObserver.observe(node, {
      attributes: true,
      childList: true,
      subtree: true,
      attributeFilter: ['disabled', 'selected', 'value', 'class'],
    })
  }
  compatObservers.push(optionObserver)

  const exportObserver = new MutationObserver(() => {
    syncLegacyAdvancedUiMeta()
  })
  const exportInput = queryLegacyById<HTMLElement>('exportAllCsvPath')
  if (exportInput) {
    exportObserver.observe(exportInput, {
      attributes: true,
      attributeFilter: ['disabled', 'class'],
    })
  }
  compatObservers.push(exportObserver)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null
  const mount = document.querySelector('#advancedVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueAdvanced = '1'
  mountReady.value = true
}

function adoptLegacyAdvancedBlocks(): void {
  const panel = panelRoot.value ?? (document.querySelector('[data-vue-panel="advanced"]') as HTMLElement | null)
  const host = document.querySelector('#advancedCompatHostVue') as HTMLElement | null
  if (!panel || !host) return
  const nodes = Array.from(panel.querySelectorAll('.advancedLegacyPassthrough')) as HTMLElement[]
  for (const node of nodes) {
    if (node.parentElement === host) continue
    const parent = node.parentNode
    if (!parent) continue
    adoptedNodes.push({ node, parent, next: node.nextSibling })
    host.appendChild(node)
  }
}

function restoreLegacyAdvancedBlocks(): void {
  while (adoptedNodes.length > 0) {
    const entry = adoptedNodes.pop()
    if (!entry) continue
    if (entry.next && entry.next.parentNode === entry.parent) {
      entry.parent.insertBefore(entry.node, entry.next)
    } else {
      entry.parent.appendChild(entry.node)
    }
  }
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  window.setTimeout(() => {
    adoptLegacyAdvancedBlocks()
    seedLegacyAdvancedInputsToStore()
    applyRecommendedOptimizationDefaults()
    syncLegacyAdvancedInputsFromStore()
    bindLegacyAdvancedInputs()
    syncLegacyAdvancedSelectOptions()
    syncLegacyAdvancedUiMeta()
    bindLegacyAdvancedObservers()
  }, 0)
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  clearCompatObservers()
  clearCompatListeners()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueAdvanced
    panelRoot.value = null
  }
  restoreLegacyAdvancedBlocks()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#advancedVueShellMount">
    <section class="advancedPanelVueRoot advancedVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Advanced Controls</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <h3>Optimization setup</h3>
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
        <div v-if="showScoreModeControl" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted" style="grid-column: span 2;">
            Composite score modes are additive opt-ins over the existing metric row. Leaving Score mode at the default preserves objective-only ranking.
            {{ ` ${hurdleTauStateNote}` }}
          </div>
        </div>
        <div v-if="showIncludeTradeMetricsControl" class="gridThree" style="margin-top: 0.75rem;">
          <div>
            <label for="vueIncludeTradeMetrics">Trade metrics</label>
            <label class="checkboxRow" for="vueIncludeTradeMetrics">
              <input
                id="vueIncludeTradeMetrics"
                type="checkbox"
                :checked="includeTradeMetricsDraft"
                @change="includeTradeMetricsDraft = ($event.target as HTMLInputElement).checked; advancedStore.setCheckboxField('includeTradeMetrics', includeTradeMetricsDraft)"
              />
              <span>Include trade summary metrics in the optimization result</span>
            </label>
          </div>
          <div class="muted" style="grid-column: span 2;">
            {{ includeTradeMetricsStateNote }}
          </div>
        </div>
        <div v-if="showExtraShortlistObjectivesControl" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted" style="grid-column: span 2;">
            {{ extraShortlistObjectivesStateNote }}
          </div>
        </div>
        <div v-if="showShortlistConfidenceModeControl" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted" style="grid-column: span 2;">
            {{ shortlistConfidenceModeStateNote }}
          </div>
        </div>
        <div v-if="showShortlistUnionModeControl" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted" style="grid-column: span 2;">
            {{ shortlistUnionModeStateNote }}
          </div>
        </div>
        <div v-if="showShortlistSliceModeControl" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted" style="grid-column: span 2;">
            {{ shortlistSliceModeStateNote }}
          </div>
        </div>
        <div v-if="showShortlistSliceBenchmarkControl" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted" style="grid-column: span 2;">
            {{ shortlistSliceBenchmarkStateNote }}
          </div>
        </div>
        <div v-if="showShortlistNeighborhoodControls" class="gridThree" style="margin-top: 0.75rem;">
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
          <div class="muted">
            {{ shortlistNeighborhoodStateNote }}
          </div>
        </div>
      </section>

      <section class="pane">
        <h3>Output shaping</h3>
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
        <h3>Strategy samplers</h3>
        <div class="metaLine mono">VDPS panel: {{ vdpsVisible ? 'visible' : 'hidden' }}</div>
        <div class="metaLine mono">Composite-MO panel: {{ compositeMoVisible ? 'visible' : 'hidden' }}</div>
        <div class="muted">{{ vdpsTelemetryHint }}</div>
        <div class="pathRow" style="margin-top: 0.55rem;">
          <div></div>
          <button type="button" :disabled="!canResetMoSearchSpace" @click="advancedStore.resetMoSearchSpace()">
            Reset MO search-space JSON
          </button>
        </div>
        <div id="advancedCompatHostVue"></div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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
  align-items: center;
  gap: 0.75rem;
}

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
}

.gridThree {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.65rem;
}

.pathRow {
  margin-top: 0.4rem;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.checkboxRow {
  margin-top: 0.4rem;
  display: flex;
  align-items: center;
  gap: 0.55rem;
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
}
</style>

<style>
[data-vue-panel='advanced'][data-vue-advanced='1'] > :not(#advancedVueShellMount) {
  display: none !important;
}
</style>


