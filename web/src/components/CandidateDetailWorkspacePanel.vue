<script setup lang="ts">
import { computed, markRaw, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import {
  useDrillPriceChartRuntime,
  type DrillPriceChartMarkerAvailability,
  type DrillPriceChartRenderState,
} from '../composables/useDrillPriceChartRuntime'
import { useShellMount } from '../composables/useShellMount'
import {
  disposeDrillSupportCharts,
  registerDrillChartController,
  type DrillChartController,
} from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import {
  buildDrillTradeMarkers,
  buildDrillTradeTableModel,
  type DrillChartModel,
  type DrillTradeTableModel,
} from '../visualizations/drillChartModel'
import type { LightweightChartLegendItem } from '../visualizations/chartTheme'
import type { StrategyIndicatorSeriesModel } from '../visualizations/priceSeriesModel'
import PriceChartPanel from './visualizations/PriceChartPanel.vue'
import AppMetricStrip, { type AppMetricItem } from './ui/AppMetricStrip.vue'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  statusLine,
  updatedAtUnixMs,
  drillChartModel,
  drillMeta,
  drillStatus,
  drillTradeTableModel,
  stabilityMeta,
  stabilityRejectMeta,
  stabilityStatus,
  stabilityTableRows,
  selectedTopDecisionNote,
  selectedTopProvenanceChips,
  selectedTopSummary,
  selectedTopTitle,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#drillVueShellMount', {
  paneSelector: "[data-vue-panel='drill']",
  paneDatasetKey: 'vueDrill',
})

const visible = computed(() => String(resultsTab.value || '').trim() === 'drill')
const selectedTradeIdx = ref(-1)
const tradeRangeStart = ref(0)
const chartMetaText = ref('Hover for OHLC/MA values; use the Trades table to focus a trade.')
const chartViewLabel = ref('')
const chartRuntimeState = ref<DrillPriceChartRenderState>('idle')
const chartMarkerAvailability = ref<DrillPriceChartMarkerAvailability>('unknown')
const visibleNativeSeriesIds = ref<string[]>([])

interface NativeIndicatorGroup {
  paneId: string
  label: string
  series: StrategyIndicatorSeriesModel[]
}

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))
const drillContentVisible = computed(() => analysisStore.drillWrapVisible || Boolean(drillChartModel.value))
const drillMetaText = computed(() => cleanText(drillMeta.value, '-'))
const drillStatusText = computed(() => cleanText(drillStatus.value, 'Click a Top row to view candidate detail curves.'))
const stabilityMetaText = computed(() => cleanText(stabilityMeta.value, '-'))
const stabilityRejectMetaText = computed(() => cleanText(stabilityRejectMeta.value, '-'))
const stabilityStatusText = computed(() => cleanText(stabilityStatus.value, '-'))
const stabilityStatusClass = computed(() => ({
  panelStatus: true,
  ok: stabilityStatusText.value.startsWith('Done'),
  badText: stabilityStatusText.value.startsWith('Stability failed'),
}))
const drillCompatibilityMessage = computed(() => {
  if (drillContentVisible.value) return ''
  const status = cleanText(drillStatus.value)
  return status.startsWith('Candidate detail unavailable:') || status.startsWith('Candidate detail stale/incompatible:')
    ? status
    : ''
})
const drillEmptyMessage = computed(() => {
  if (drillCompatibilityMessage.value) return drillCompatibilityMessage.value
  const status = cleanText(drillStatus.value)
  if (status && !status.startsWith('Click a Top row')) return status
  return 'Select a row in the Top tab to compute candidate detail curves.'
})
const tradeTableVisible = computed(() => Boolean(drillTradeTableModel.value?.visible))
const tradeStatsText = computed(() => drillTradeTableModel.value?.statsText || '-')
const tradeRows = computed(() => drillTradeTableModel.value?.rows ?? [])
const hasTradeEvidence = computed(() => Number(drillChartModel.value?.trades?.length ?? 0) > 0)
const chartModelReady = computed(() => {
  const model = drillChartModel.value
  return Boolean(model?.bars.length && model?.candleData.length)
})
const chartRenderReady = computed(() =>
  chartModelReady.value && chartRuntimeState.value === 'ready',
)
const markerOverlayAvailable = computed(() =>
  chartRenderReady.value &&
  (
    chartMarkerAvailability.value === 'display-only' ||
    chartMarkerAvailability.value === 'interactive'
  ) &&
  hasTradeEvidence.value,
)
const markerInteractionAvailable = computed(() =>
  markerOverlayAvailable.value && chartMarkerAvailability.value === 'interactive',
)

function cleanText(value: unknown, fallback = ''): string {
  const text = String(value ?? '').trim()
  if (!text || text === '-' || text === '—' || text === 'undefined' || text === 'NaN') return fallback
  return text
}

function nativeIndicatorPaneLabel(paneIdRaw: string, strategyKindRaw: string): string {
  const paneId = cleanText(paneIdRaw, 'indicator')
  if (paneId.toLowerCase() === 'price') return 'Price pane'
  const strategyPrefix = cleanText(strategyKindRaw)
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, '_')
    .replace(/^_+|_+$/gu, '')
  const withoutStrategy = paneId
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, '_')
    .replace(new RegExp(`^${strategyPrefix}_?`, 'u'), '')
    .replace(/^_+|_+$/gu, '')
  const label = (withoutStrategy || paneId)
    .split(/[_\s-]+/u)
    .filter(Boolean)
    .map((part) => {
      if (part === 'adx') return 'ADX'
      if (part === 'atr') return 'ATR'
      if (part === 'dpo') return 'DPO'
      if (part === 'ift') return 'IFT'
      if (part === 'roc') return 'ROC'
      if (part === 'zscore') return 'Z-score'
      return `${part.charAt(0).toUpperCase()}${part.slice(1)}`
    })
    .join(' ')
  return `${label || 'Indicator'} pane`
}

function nativeIndicatorUnitLabel(unit: StrategyIndicatorSeriesModel['unit']): string {
  return unit.replace(/_/gu, ' ')
}

function nativeIndicatorDomId(value: string): string {
  return cleanText(value, 'series')
    .replace(/[^a-zA-Z0-9_-]+/gu, '-')
    .replace(/^-+|-+$/gu, '') || 'series'
}

function nativeIndicatorControlId(seriesId: string): string {
  return `drillIndicator_${nativeIndicatorDomId(seriesId)}`
}

function namedSeriesLegendColorToken(
  index: number,
  kind: StrategyIndicatorSeriesModel['kind'],
): string {
  return kind === 'histogram'
    ? `named-histogram:${index}`
    : `named-series:${index}`
}

function nativeIndicatorIsVisible(seriesId: string): boolean {
  return visibleNativeSeriesIds.value.includes(seriesId)
}

function defaultVisibleNativeSeriesIds(model: DrillChartModel | null): string[] {
  return model?.strategyIndicators?.series
    .filter((series) => series.defaultVisible && series.finitePointCount > 0)
    .map((series) => series.id) ?? []
}

function onNativeIndicatorChange(seriesId: string, event: Event): void {
  const input = event.currentTarget
  if (!(input instanceof HTMLInputElement)) return
  const bundle = nativeStrategyIndicators.value
  if (!bundle) return
  const series = bundle.series.find((candidate) => candidate.id === seriesId)
  if (!series || series.finitePointCount <= 0) return
  const selected = new Set(visibleNativeSeriesIds.value)
  if (input.checked) selected.add(seriesId)
  else selected.delete(seriesId)
  visibleNativeSeriesIds.value = bundle.series
    .filter((candidate) => candidate.finitePointCount > 0 && selected.has(candidate.id))
    .map((candidate) => candidate.id)
  void renderCurrentChart()
}

function formatMetricNumber(value: unknown, digits = 2): string {
  const next = Number(value)
  if (!Number.isFinite(next)) return ''
  if (Number.isInteger(next)) return String(next)
  return next.toFixed(Math.max(0, digits)).replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

function formatMetricPercent(value: unknown): string {
  const next = Number(value)
  if (!Number.isFinite(next)) return ''
  const pct = Math.abs(next) <= 1 ? next * 100 : next
  return `${formatMetricNumber(pct, 1)}%`
}

const selectedCandidateTitle = computed(() => {
  const explicit = cleanText(selectedTopTitle.value)
  if (explicit && explicit !== 'No ranked candidate selected.') return explicit
  if (drillContentVisible.value) return 'Selected candidate evidence'
  return 'No ranked candidate selected.'
})

const selectedCandidateSummary = computed(() => {
  if (drillCompatibilityMessage.value) return drillCompatibilityMessage.value
  const explicit = cleanText(selectedTopSummary.value)
  if (
    explicit &&
    explicit !== 'No ranked candidate selected.' &&
    explicit !== 'No ranked candidates are available.'
  ) {
    return explicit
  }
  if (drillContentVisible.value) {
    if (chartRuntimeState.value === 'error') {
      return 'Candidate evidence loaded, but the price chart is unavailable. Trade, drawdown, and stability evidence remain available.'
    }
    if (chartModelReady.value && !chartRenderReady.value) {
      return 'Candidate evidence loaded; the price chart is rendering while trade, drawdown, and stability evidence remain available.'
    }
    if (chartRenderReady.value) {
      return 'Chart, trade, drawdown, and stability evidence are ready for the current drilldown target.'
    }
    return 'Candidate detail evidence is available for the current review target.'
  }
  return 'Select a Top Results row to compute candidate detail curves.'
})

const selectedCandidateDecisionNote = computed(() => {
  if (drillCompatibilityMessage.value) {
    return 'Historical candidate rows remain available in Top Results. Load the matching dataset before recomputing detail evidence.'
  }
  const explicit = cleanText(selectedTopDecisionNote.value)
  if (
    explicit &&
    explicit !== 'Select a ranked candidate to lock a review target before moving into Candidate Detail or Validation.'
  ) {
    return explicit
  }
  if (!drillContentVisible.value) {
    return 'Candidate Detail stays empty until a ranked row has been selected from Top Results.'
  }
  return markerInteractionAvailable.value
    ? 'Use trade markers and stability evidence to confirm whether this candidate deserves validation.'
    : 'Use the Trades table and stability evidence to confirm whether this candidate deserves validation.'
})

const nativeStrategyIndicators = computed(() =>
  drillChartModel.value?.strategyIndicators ?? null,
)
const availableNativeIndicatorSeries = computed(() =>
  nativeStrategyIndicators.value?.series.filter((series) => series.finitePointCount > 0) ?? [],
)
const unavailableNativeIndicatorSeries = computed(() =>
  nativeStrategyIndicators.value?.series.filter((series) => series.finitePointCount === 0) ?? [],
)
const visibleNativeIndicatorSeries = computed(() => {
  const visibleIds = new Set(visibleNativeSeriesIds.value)
  return availableNativeIndicatorSeries.value.filter((series) => visibleIds.has(series.id))
})
const nativeIndicatorGroups = computed<NativeIndicatorGroup[]>(() => {
  const bundle = nativeStrategyIndicators.value
  if (!bundle) return []
  const groups = new Map<string, NativeIndicatorGroup>()
  for (const series of bundle.series) {
    let group = groups.get(series.paneId)
    if (!group) {
      group = {
        paneId: series.paneId,
        label: nativeIndicatorPaneLabel(series.paneId, bundle.strategyKind),
        series: [],
      }
      groups.set(series.paneId, group)
    }
    group.series.push(series)
  }
  return [...groups.values()]
})
const nativeIndicatorSummaryText = computed(() => {
  const bundle = nativeStrategyIndicators.value
  if (!bundle) return ''
  return `${bundle.strategyKind} strategy indicators · ${visibleNativeIndicatorSeries.value.length} shown · ${availableNativeIndicatorSeries.value.length} available · ${bundle.series.length} total`
})
const nativeIndicatorUnavailableText = computed(() => {
  const count = unavailableNativeIndicatorSeries.value.length
  if (!count) return ''
  return `${count} ${count === 1 ? 'series is' : 'series are'} unavailable because this candidate returned no finite points.`
})
const nativeIndicatorPaneCount = computed(() => {
  if (!nativeStrategyIndicators.value) return 1
  const panes = new Set<string>(['price'])
  for (const series of visibleNativeIndicatorSeries.value) panes.add(series.paneId)
  return Math.max(1, panes.size)
})
const chartTitle = computed(() =>
  nativeStrategyIndicators.value ? 'Price + strategy indicators + trades' : 'Price + trades',
)
const chartFrameAriaLabel = computed(() =>
  nativeStrategyIndicators.value
    ? 'Backtest price, strategy indicator, and trade chart for the selected candidate. Use the Top results tab to select a different candidate without clicking inside the chart.'
    : 'Backtest price and trade chart for the selected candidate. Use the Top results tab to select a different candidate without clicking inside the chart.',
)
const chartHostAriaLabel = computed(() =>
  nativeStrategyIndicators.value
    ? 'Backtest price and strategy indicator chart'
    : 'Backtest price chart',
)

const chartEvidenceChips = computed(() => {
  const model = drillChartModel.value
  if (!model) return ['Awaiting Top Results selection']
  if (model.strategyIndicators) {
    return [
      model.bars.length > 0 ? `${model.bars.length} price bars` : '',
      model.trades.length > 0 ? `${model.trades.length} trades` : 'No trades',
      ...visibleNativeIndicatorSeries.value.map((series) => series.label),
      visibleNativeIndicatorSeries.value.length === 0 ? 'No strategy indicators shown' : '',
    ].filter((chip) => chip.length > 0)
  }
  const chips = [
    model.bars.length > 0 ? `${model.bars.length} price bars` : '',
    model.trades.length > 0 ? `${model.trades.length} trades` : 'No trades',
    model.fastData.length || model.slowData.length ? 'MA overlays' : '',
    model.halftrendData.length ? 'HalfTrend regime' : '',
    model.ottData.length ? 'OTT' : '',
    model.maczData.length ? 'MAC-Z' : '',
  ]
  return chips.filter((chip) => chip.length > 0)
})

const chartStateLabel = computed(() => {
  if (chartRuntimeState.value === 'error') return 'Chart unavailable'
  if (!chartModelReady.value) return 'Awaiting chart'
  if (!chartRenderReady.value) return 'Rendering chart'
  return selectedTradeIdx.value >= 0 ? `Trade ${selectedTradeIdx.value + 1} selected` : 'Chart ready'
})

const chartStateKind = computed<'idle' | 'loading' | 'ready' | 'warning' | 'error'>(() => {
  if (chartRuntimeState.value === 'error') return 'error'
  if (!chartModelReady.value) return 'idle'
  if (!chartRenderReady.value) return 'loading'
  if (
    chartMarkerAvailability.value !== 'interactive' &&
    hasTradeEvidence.value
  ) {
    return 'warning'
  }
  return 'ready'
})

const chartAdvisory = computed(() => {
  if (!chartRenderReady.value || !hasTradeEvidence.value) return ''
  if (chartMarkerAvailability.value === 'unavailable') {
    return 'Trade markers are unavailable in this chart runtime. Use the Trades table and Previous/Next trade controls to review individual trades.'
  }
  if (chartMarkerAvailability.value === 'display-only') {
    return 'Trade markers are visible, but direct marker selection is unavailable. Use the Trades table and Previous/Next trade controls to focus individual trades.'
  }
  return ''
})

const drillKeyboardHintText = computed(() =>
  markerInteractionAvailable.value
    ? 'Use the Top results table to select another candidate, or click chart markers if you prefer direct chart interaction.'
    : 'Use the Top results table to select another candidate, and use the Trades table to focus an individual trade.',
)

const tradeTableHintText = computed(() =>
  markerInteractionAvailable.value
    ? 'Showing up to 200 trades. Select a Top result or click a marker to focus a specific trade.'
    : 'Showing up to 200 trades. Select a Top result, then use this table or the Previous/Next trade controls to focus a specific trade.',
)

const chartLegendItems = computed<LightweightChartLegendItem[]>(() => {
  const model = drillChartModel.value
  if (!model) return []
  const items: LightweightChartLegendItem[] = [
    { label: 'Candles', paletteKey: 'candles', value: String(model.candleData.length) },
  ]
  if (model.strategyIndicators) {
    const visibleIds = new Set(visibleNativeSeriesIds.value)
    model.strategyIndicators.series.forEach((series, index) => {
      if (series.finitePointCount <= 0 || !visibleIds.has(series.id)) return
      items.push({
        label: series.label,
        color: namedSeriesLegendColorToken(index, series.kind),
        value: String(series.finitePointCount),
      })
    })
  } else {
    if (model.fastData.length) items.push({ label: 'Fast MA', paletteKey: 'fast', value: String(model.fastData.length) })
    if (model.slowData.length) items.push({ label: 'Slow MA', paletteKey: 'slow', value: String(model.slowData.length) })
    if (model.halftrendData.length) items.push({ label: 'HalfTrend', paletteKey: 'halftrend', value: String(model.halftrendData.length) })
    if (model.ottData.length) items.push({ label: 'OTT', paletteKey: 'ott', value: String(model.ottData.length) })
    if (model.maczData.length) items.push({ label: 'MAC-Z', paletteKey: 'macz', value: String(model.maczData.length) })
  }
  if (model.trades.length && markerOverlayAvailable.value) {
    const markers = buildDrillTradeMarkers(model, -1)
    const entries = markers.filter((marker) => marker.id.endsWith('-entry')).length
    const exits = markers.filter((marker) => marker.id.endsWith('-exit')).length
    if (entries) items.push({ label: 'Entries', paletteKey: 'entries', value: String(entries) })
    if (exits) items.push({ label: 'Exits', paletteKey: 'exits', value: String(exits) })
  }
  return items
})

const candidateProvenanceChips = computed(() => {
  const explicit = Array.isArray(selectedTopProvenanceChips.value)
    ? selectedTopProvenanceChips.value.map((chip) => cleanText(chip)).filter((chip) => chip.length > 0)
    : []
  const seen = new Set<string>()
  return [...explicit, ...chartEvidenceChips.value]
    .filter((chip) => {
      if (seen.has(chip)) return false
      seen.add(chip)
      return true
    })
    .slice(0, 6)
})

const drillEvidenceStats = computed<AppMetricItem[]>(() => {
  const model = drillChartModel.value
  const stats = model?.tradeStats ?? {}
  const closedTrades = formatMetricNumber(stats.closed_trades)
  const winRate = formatMetricPercent(stats.win_rate)
  const profitFactor = formatMetricNumber(stats.profit_factor, 2)
  const expectancy = formatMetricNumber(stats.expectancy, 4)
  const rows = [
    { key: 'bars', label: 'Bars', value: formatMetricNumber(model?.bars.length ?? Number.NaN) },
    { key: 'trades', label: 'Trades', value: formatMetricNumber(model?.trades.length ?? Number.NaN) },
    { key: 'closed', label: 'Closed', value: closedTrades },
    { key: 'win-rate', label: 'Win rate', value: winRate },
    { key: 'profit-factor', label: 'Profit factor', value: profitFactor },
    { key: 'expectancy', label: 'Expectancy', value: expectancy },
  ]
  return rows.filter((row) => cleanText(row.value).length > 0).slice(0, 6)
})

const chartRuntime = useDrillPriceChartRuntime({
  onTradeMarkerSelect: (tradeIdx) => {
    if (selectTradeByIndex(tradeIdx)) {
      setPanelStatus(`Selected trade ${tradeIdx + 1}.`)
    }
  },
  onMetaChange: (message) => {
    chartMetaText.value = message
  },
  onViewChange: (label) => {
    chartViewLabel.value = label
  },
  onRenderStateChange: (state) => {
    chartRuntimeState.value = state
  },
  onMarkerAvailabilityChange: (availability) => {
    chartMarkerAvailability.value = availability
  },
})

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

function setDrillChartHost(element: HTMLElement | null): void {
  chartRuntime.setHost(element)
  if (!element || !visible.value || !mountReady.value) return
  void renderCurrentChart()
}

function fitDrillChartContent(): void {
  chartRuntime.fitContent()
}

function panDrillChartLeft(): void {
  chartRuntime.pan('left')
}

function panDrillChartRight(): void {
  chartRuntime.pan('right')
}

function setPanelStatus(message: string): void {
  analysisStore.statusLine = message
  analysisStore.source = 'vue'
  analysisStore.updatedAtUnixMs = Date.now()
}

function publishTradeTable(model: DrillChartModel | null = drillChartModel.value): DrillTradeTableModel | null {
  if (!model) {
    analysisStore.drillTradeTableModel = null
    analysisStore.tradeStatsWrapVisible = false
    return null
  }
  const tableModel = buildDrillTradeTableModel({
    chartModel: model,
    selectedTradeIdx: selectedTradeIdx.value,
    rangeStart: tradeRangeStart.value,
    maxRows: 200,
  })
  tradeRangeStart.value = tableModel.start
  analysisStore.drillTradeTableModel = markRaw(tableModel)
  analysisStore.tradeStatsWrapVisible = tableModel.visible
  return tableModel
}

function selectTradeByIndex(tradeIdxRaw: unknown): boolean {
  const model = drillChartModel.value
  const tradeIdx = Math.floor(Number(tradeIdxRaw))
  if (!model || !Number.isFinite(tradeIdx) || tradeIdx < 0 || tradeIdx >= model.trades.length) {
    return false
  }
  selectedTradeIdx.value = tradeIdx
  publishTradeTable(model)
  chartRuntime.updateSelectedTrade(model, tradeIdx)
  return true
}

function selectTradeFromRow(tradeIdx: number): void {
  if (selectTradeByIndex(tradeIdx)) {
    setPanelStatus(`Selected trade ${tradeIdx + 1}.`)
  }
}

function onTradeRowKeydown(event: KeyboardEvent, tradeIdx: number): void {
  if (event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
  selectTradeFromRow(tradeIdx)
}

function tradeRowLabel(tradeIdx: number): string {
  return `Select trade ${tradeIdx + 1}`
}

function clearTradeSelection(): boolean {
  if (selectedTradeIdx.value < 0) return false
  selectedTradeIdx.value = -1
  const model = drillChartModel.value
  publishTradeTable(model)
  chartRuntime.updateSelectedTrade(model, -1)
  return true
}

function clearTradeSelectionFromPanel(): void {
  if (clearTradeSelection()) {
    setPanelStatus('Cleared selected trade.')
    return
  }
  setPanelStatus('No trade is currently selected.')
}

function selectNextTrade(): string | null {
  const model = drillChartModel.value
  if (!model?.trades.length) return null
  const next = selectedTradeIdx.value < 0
    ? 0
    : selectedTradeIdx.value >= model.trades.length - 1
      ? -1
      : selectedTradeIdx.value + 1
  if (next < 0 || !selectTradeByIndex(next)) return null
  return String(next + 1)
}

function selectNextTradeFromPanel(): void {
  const next = selectNextTrade()
  setPanelStatus(next ? `Selected trade ${next}.` : 'No next trade to select.')
}

function selectPrevTrade(): string | null {
  const model = drillChartModel.value
  if (!model?.trades.length) return null
  const prev = selectedTradeIdx.value < 0
    ? model.trades.length - 1
    : selectedTradeIdx.value <= 0
      ? -1
      : selectedTradeIdx.value - 1
  if (prev < 0 || !selectTradeByIndex(prev)) return null
  return String(prev + 1)
}

function selectPrevTradeFromPanel(): void {
  const prev = selectPrevTrade()
  setPanelStatus(prev ? `Selected trade ${prev}.` : 'No previous trade to select.')
}

const drillChartController: DrillChartController = {
  clearEvidence: () => {
    selectedTradeIdx.value = -1
    tradeRangeStart.value = 0
    visibleNativeSeriesIds.value = []
    chartRuntime.clear()
  },
  clearTradeSelection,
  selectPrevTrade,
  selectNextTrade,
  selectTradeByIndex,
}

registerDrillChartController(drillChartController)

async function renderCurrentChart(): Promise<void> {
  await nextTick()
  if (!visible.value || !mountReady.value) return
  const model = drillChartModel.value
  await chartRuntime.render(
    model,
    selectedTradeIdx.value,
    model?.strategyIndicators ? [...visibleNativeSeriesIds.value] : undefined,
  )
}

async function ensureVisibleDrill(): Promise<void> {
  await nextTick()
  await analysisStore.rerenderDrill()
  await renderCurrentChart()
}

watch([visible, mountReady], async ([nextVisible, nextReady]) => {
  if (!nextVisible || !nextReady) return
  await ensureVisibleDrill()
}, { immediate: true })

watch(drillChartModel, async (model) => {
  selectedTradeIdx.value = -1
  tradeRangeStart.value = 0
  visibleNativeSeriesIds.value = defaultVisibleNativeSeriesIds(model)
  chartMetaText.value = model?.strategyIndicators
    ? 'Hover for OHLC and strategy indicator values; use the Trades table to focus a trade.'
    : 'Hover for OHLC/MA values; use the Trades table to focus a trade.'
  publishTradeTable(model)
  await renderCurrentChart()
}, { immediate: true })

onBeforeUnmount(() => {
  disposeDrillSupportCharts()
  registerDrillChartController(null)
  chartRuntime.dispose()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#drillVueShellMount">
    <section class="drillPanelVueRoot drillVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Candidate evidence</div>
          <h2 class="panelTitle">Candidate Detail</h2>
          <div class="panelSubtitle">{{ selectedCandidateSummary }}</div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section class="pane candidateDecisionPane">
        <div class="candidateDecisionHeader">
          <div class="candidateDecisionCopy">
            <div class="panelEyebrow">Current review target</div>
            <h3 class="candidateDecisionTitle">{{ selectedCandidateTitle }}</h3>
            <div class="panelMuted">{{ selectedCandidateDecisionNote }}</div>
          </div>
          <div class="actions candidateDecisionActions">
            <button type="button" data-qa-action="candidateDetail.reviewTop" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
        <div class="panelSummaryChips" aria-label="Candidate detail provenance">
          <span
            v-for="chip in candidateProvenanceChips"
            :key="`drill_chip_${chip}`"
            class="panelSummaryChip mono"
          >
            {{ chip }}
          </span>
        </div>
        <AppMetricStrip
          v-if="drillEvidenceStats.length > 0"
          class="candidateMetricStrip"
          :items="drillEvidenceStats"
          aria-label="Candidate detail metrics"
          min-item-width="5.8rem"
          size="compact"
          surface="sunken"
        />
      </section>

      <section class="pane">
        <div
          id="drillEmptyHintVue"
          v-show="!drillContentVisible"
          class="panelStatus"
          role="status"
          aria-live="polite"
        >
          {{ drillEmptyMessage }}
        </div>
        <div class="chartAssist">
          <div id="drillKeyboardHint" class="panelMuted">{{ drillKeyboardHintText }}</div>
          <div class="actions">
            <button type="button" data-qa-action="candidateDetail.reviewTop" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
          <div v-if="!drillContentVisible" class="panelAdvisory">
            {{ drillCompatibilityMessage
              ? 'Load the matching dataset before recomputing detail evidence. The historical shortlist remains available in Top Results.'
              : 'If the charts stay empty, confirm that a Top result is selected and that the latest optimization run finished writing analysis results.' }}
          </div>
          <div
            id="drillWrapVue"
            :style="{ marginTop: '0.75rem', display: drillContentVisible ? 'block' : 'none' }"
          >
            <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillMetaVue">{{ drillMetaText }}</div>
            <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillStatusVue" role="status" aria-live="polite">{{ drillStatusText }}</div>
          <div class="chartEvidenceGrid">
            <div class="priceEvidencePane">
              <details
                v-if="nativeStrategyIndicators"
                class="nativeIndicatorControls"
                data-testid="native-indicator-controls"
                :data-native-strategy="nativeStrategyIndicators.strategyKind"
                aria-label="Strategy indicators"
              >
                <summary class="nativeIndicatorSummary">
                  <span>Strategy indicators</span>
                  <span class="nativeIndicatorSummaryCount mono">{{ nativeIndicatorSummaryText }}</span>
                </summary>
                <div class="nativeIndicatorControlBody">
                  <p class="panelMuted nativeIndicatorHelp">
                    Choose which backend-provided series are visible. Price overlays share the candle pane; other groups use aligned indicator panes.
                  </p>
                  <fieldset
                    v-for="group in nativeIndicatorGroups"
                    :key="group.paneId"
                    class="nativeIndicatorFieldset"
                    :data-indicator-pane="group.paneId"
                  >
                    <legend>{{ group.label }}</legend>
                    <div class="nativeIndicatorOptionGrid">
                      <label
                        v-for="series in group.series"
                        :key="series.id"
                        class="nativeIndicatorOption"
                        :class="{ unavailable: series.finitePointCount === 0 }"
                        :for="nativeIndicatorControlId(series.id)"
                      >
                        <input
                          :id="nativeIndicatorControlId(series.id)"
                          type="checkbox"
                          :checked="nativeIndicatorIsVisible(series.id)"
                          :disabled="series.finitePointCount === 0"
                          :aria-label="series.label"
                          aria-controls="drillTvChartVue"
                          :aria-describedby="series.finitePointCount === 0 ? 'drillIndicatorUnavailableSummary' : undefined"
                          :data-indicator-series-id="series.id"
                          @change="onNativeIndicatorChange(series.id, $event)"
                        />
                        <span class="nativeIndicatorOptionCopy">
                          <strong>{{ series.label }}</strong>
                          <small class="mono">{{ series.kind }} · {{ nativeIndicatorUnitLabel(series.unit) }} · {{ series.finitePointCount }} points</small>
                        </span>
                        <span
                          v-if="series.finitePointCount === 0"
                          class="nativeIndicatorUnavailableTag"
                        >
                          Unavailable
                        </span>
                      </label>
                    </div>
                  </fieldset>
                  <p
                    v-if="nativeIndicatorUnavailableText"
                    id="drillIndicatorUnavailableSummary"
                    class="panelAdvisory nativeIndicatorUnavailableCopy"
                  >
                    {{ nativeIndicatorUnavailableText }}
                  </p>
                </div>
              </details>
              <PriceChartPanel
                chart-id="drillTvChartVue"
                meta-id="drillTvChartMetaVue"
                :title="chartTitle"
                :meta-text="chartMetaText"
                :summary-items="chartEvidenceChips"
                :legend-items="chartLegendItems"
                :state-label="chartStateLabel"
                :state-kind="chartStateKind"
                :render-ready="chartRenderReady"
                :view-label="chartViewLabel"
                :navigation="true"
                :point-inspection="true"
                :advisory="chartAdvisory"
                :pane-count="nativeIndicatorPaneCount"
                :native-strategy="nativeStrategyIndicators?.strategyKind ?? ''"
                :native-series-visible="nativeStrategyIndicators ? visibleNativeIndicatorSeries.length : 0"
                :native-series-total="nativeStrategyIndicators?.series.length ?? 0"
                qa-prefix="candidateChart"
                :frame-aria-label="chartFrameAriaLabel"
                :host-aria-label="chartHostAriaLabel"
                described-by="drillKeyboardHint"
                @host-ready="setDrillChartHost"
                @fit-content="fitDrillChartContent"
                @pan-left="panDrillChartLeft"
                @pan-right="panDrillChartRight"
              />
            </div>
            <div class="supportChartGrid">
              <div class="supportChartPane">
                <div class="panelMuted">Equity (min/max per bin)</div>
                <canvas id="equityCanvasVue" class="chart supportChart" width="1024" height="160" role="img" aria-label="Equity envelope chart for the selected candidate"></canvas>
                <div id="equityChartStatusVue" class="supportChartStatus panelMuted" role="status" hidden></div>
              </div>
              <div class="supportChartPane">
                <div class="panelMuted">Drawdown (min/max per bin)</div>
                <canvas id="drawdownCanvasVue" class="chart supportChart" width="1024" height="160" role="img" aria-label="Drawdown envelope chart for the selected candidate"></canvas>
                <div id="drawdownChartStatusVue" class="supportChartStatus panelMuted" role="status" hidden></div>
              </div>
            </div>
          </div>

          <div
            id="stabilityWrapVue"
            :style="{ marginTop: '1.1rem', display: analysisStore.stabilityWrapVisible ? 'block' : 'none' }"
          >
            <div class="panelSectionHeader compactSectionHeader">
              <div>
                <div class="panelEyebrow">Neighborhood</div>
                <h3 class="panelPaneTitle">Stability</h3>
              </div>
            </div>
            <div :class="stabilityStatusClass" style="margin-bottom: 0.35rem;" id="stabilityStatusVue" role="status" aria-live="polite">{{ stabilityStatusText }}</div>
            <div class="panelStatus" style="margin-bottom: 0.6rem;" id="stabilityMetaVue">{{ stabilityMetaText }}</div>
            <div class="panelStatus" style="margin-bottom: 0.6rem;" id="rejectMetaVue">{{ stabilityRejectMetaText }}</div>
            <div
              class="table-wrap"
              id="stabilityTableWrapVue"
              :style="{ display: stabilityTableRows.length ? 'block' : 'none', marginTop: '0.45rem' }"
              tabindex="0"
              aria-label="Stability neighborhood table"
            >
              <table>
                <caption class="srOnly">Stability neighborhood metrics around the selected fast and slow parameter pair.</caption>
                <thead>
                  <tr>
                    <th class="left">Fast</th>
                    <th class="left">Slow</th>
                    <th>Score</th>
                    <th>PnL</th>
                    <th>Sharpe</th>
                    <th>MaxDD</th>
                    <th>Trades</th>
                  </tr>
                </thead>
                <tbody id="stabilityTableBodyVue">
                  <tr v-for="row in stabilityTableRows" :key="row.key">
                    <td
                      v-for="(cell, cellIndex) in row.cells"
                      :key="cellIndex"
                      :class="cell.className"
                    >
                      {{ cell.text }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div class="panelMuted" style="margin-top: 0.5rem;">Evaluates a small neighborhood around the selected MA lengths to detect sharp peaks.</div>
          </div>

          <div
            id="tradeStatsWrapVue"
            :style="{ marginTop: '1.1rem', display: tradeTableVisible ? 'block' : 'none' }"
          >
            <div class="tradeEvidenceHeader">
              <div>
                <div class="panelEyebrow">Trade evidence</div>
                <h3 class="panelPaneTitle">Trades</h3>
              </div>
              <div class="actions tradeEvidenceActions">
                <button type="button" :disabled="!hasTradeEvidence" data-qa-action="candidateDetail.trade.prev" @click="selectPrevTradeFromPanel()">Previous trade</button>
                <button type="button" :disabled="!hasTradeEvidence" data-qa-action="candidateDetail.trade.next" @click="selectNextTradeFromPanel()">Next trade</button>
                <button type="button" :disabled="!hasTradeEvidence" data-qa-action="candidateDetail.trade.clear" @click="clearTradeSelectionFromPanel()">Clear selection</button>
              </div>
            </div>
            <div class="mono" id="tradeStatsVue">{{ tradeStatsText }}</div>
            <div class="table-wrap" style="margin-top: 0.6rem;" tabindex="0" aria-label="Trade list table">
              <table>
                <caption class="srOnly">Trade-level results for the selected candidate, including entry, exit, duration, profit and loss, and current state.</caption>
                <thead>
                  <tr>
                    <th class="left">#</th>
                    <th class="left">Dir</th>
                    <th class="left">Entry</th>
                    <th class="left">Entry time</th>
                    <th class="left">Exit</th>
                    <th class="left">Exit time</th>
                    <th>Bars</th>
                    <th>PnL</th>
                    <th class="left">State</th>
                  </tr>
                </thead>
                <tbody id="tradeTableBodyVue">
                  <tr
                    v-for="row in tradeRows"
                    :key="row.tradeIdx"
                    :data-trade-idx="String(row.tradeIdx)"
                    :class="{ selected: row.selected }"
                    tabindex="0"
                    :aria-selected="row.selected ? 'true' : 'false'"
                    :aria-label="tradeRowLabel(row.tradeIdx)"
                    @click="selectTradeFromRow(row.tradeIdx)"
                    @keydown="onTradeRowKeydown($event, row.tradeIdx)"
                  >
                    <td
                      v-for="(cell, cellIndex) in row.cells"
                      :key="cellIndex"
                      :class="cell.className"
                      :title="cell.title || undefined"
                    >
                      {{ cell.text }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div class="panelMuted" style="margin-top: 0.5rem;">{{ tradeTableHintText }}</div>
          </div>
        </div>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.drillPanelVueRoot {
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
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 95%, var(--panel-footer));
  padding: var(--density-pane-y) var(--density-pane-x);
  min-width: 0;
}

.candidateDecisionPane {
  display: grid;
  gap: 0.65rem;
}

.candidateDecisionHeader,
.tradeEvidenceHeader {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  min-width: 0;
}

.candidateDecisionCopy {
  min-width: 0;
  display: grid;
  gap: 0.18rem;
}

.candidateDecisionTitle {
  margin: 0;
  font-size: 1rem;
  line-height: var(--leading-tight);
  font-weight: 700;
  overflow-wrap: anywhere;
}

.chartEvidenceGrid {
  display: grid;
  gap: 0.85rem;
}

.priceEvidencePane,
.supportChartPane {
  min-width: 0;
}

.priceEvidencePane {
  display: grid;
  gap: 0.55rem;
}

.nativeIndicatorControls {
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-inline) 64%, var(--surface-working));
  overflow: clip;
}

.nativeIndicatorControls[open] {
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
}

.nativeIndicatorSummary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  padding: 0.5rem 0.62rem;
  cursor: pointer;
  color: var(--text);
  font-size: 0.76rem;
  font-weight: 720;
}

.nativeIndicatorSummary:hover {
  background: color-mix(in oklab, var(--accent-soft) 42%, transparent);
}

.nativeIndicatorSummary:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: -2px;
}

.nativeIndicatorSummaryCount {
  min-width: 0;
  color: var(--muted-strong);
  font-size: 0.68rem;
  font-weight: 560;
  text-align: right;
  overflow-wrap: anywhere;
}

.nativeIndicatorControlBody {
  display: grid;
  gap: 0.55rem;
  padding: 0 0.62rem 0.62rem;
}

.nativeIndicatorHelp,
.nativeIndicatorUnavailableCopy {
  margin: 0;
}

.nativeIndicatorFieldset {
  min-width: 0;
  margin: 0;
  padding: 0.45rem;
  border: 1px solid color-mix(in oklab, var(--border) 76%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 78%, transparent);
}

.nativeIndicatorFieldset legend {
  padding: 0 0.28rem;
  color: var(--muted-strong);
  font-size: 0.7rem;
  font-weight: 720;
}

.nativeIndicatorOptionGrid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 13.5rem), 1fr));
  gap: 0.38rem;
}

.nativeIndicatorOption {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.42rem;
  padding: 0.38rem 0.44rem;
  border: 1px solid color-mix(in oklab, var(--border) 66%, transparent);
  border-radius: 7px;
  background: color-mix(in oklab, var(--surface-inline) 72%, transparent);
  cursor: pointer;
}

.nativeIndicatorOption:hover:not(.unavailable) {
  border-color: color-mix(in oklab, var(--accent) 42%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 42%, var(--surface-inline));
}

.nativeIndicatorOption.unavailable {
  cursor: not-allowed;
  background: color-mix(in oklab, var(--surface-inline) 52%, transparent);
}

.nativeIndicatorOption input {
  width: 0.9rem;
  height: 0.9rem;
  margin: 0;
  accent-color: var(--accent);
}

.nativeIndicatorOptionCopy {
  min-width: 0;
  display: grid;
  gap: 0.1rem;
}

.nativeIndicatorOptionCopy strong {
  color: var(--text);
  font-size: 0.72rem;
  line-height: 1.2;
  overflow-wrap: anywhere;
}

.nativeIndicatorOptionCopy small {
  color: var(--muted);
  font-size: 0.62rem;
  line-height: 1.25;
  overflow-wrap: anywhere;
}

.nativeIndicatorOption.unavailable .nativeIndicatorOptionCopy {
  opacity: 0.68;
}

.nativeIndicatorUnavailableTag {
  color: var(--warning-ink);
  font-size: 0.62rem;
  font-weight: 720;
}

.nativeIndicatorUnavailableCopy {
  font-size: 0.7rem;
}

.supportChartGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.supportChartPane {
  display: grid;
  gap: 0.35rem;
}

.supportChart {
  min-height: 118px;
}

.compactSectionHeader {
  margin-bottom: 0.45rem;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.candidateDecisionActions,
.tradeEvidenceActions {
  margin-top: 0;
  flex: 0 0 auto;
  justify-content: flex-end;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.chartAssist {
  margin-top: 0.75rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.chartAssist .panelMuted {
  flex: 1 1 22rem;
  margin: 0;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .supportChartGrid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 680px) {
  .head,
  .candidateDecisionHeader,
  .tradeEvidenceHeader {
    display: grid;
  }

  .supportChartGrid {
    grid-template-columns: 1fr;
  }

  .candidateDecisionActions,
  .tradeEvidenceActions {
    justify-content: flex-start;
  }

  .nativeIndicatorSummary {
    align-items: flex-start;
    flex-direction: column;
    gap: 0.18rem;
  }

  .nativeIndicatorSummaryCount {
    text-align: left;
  }
}
</style>

<style>
[data-vue-panel='drill'][data-vue-drill='1'] > :not(#drillVueShellMount) {
  display: none !important;
}
</style>


