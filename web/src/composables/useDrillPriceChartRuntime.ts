import { markRaw } from 'vue'
import { useLightweightChart } from './useLightweightChart'
import {
  applyLightweightChartSize,
  createDrillSeriesRuntime,
  createNamedSeriesRuntime,
  fitLightweightChartContent,
  observeLightweightChartTheme,
  panLightweightChart,
  type LightweightChartModule,
  type NamedChartSeriesInput,
  type NamedChartSeriesRuntime,
  visibleLogicalRangeLabel,
} from '../visualizations/lightweightChartRuntime'
import {
  buildDrillTradeMarkers,
  type DrillChartModel,
  type DrillCandlePoint,
  type DrillHistogramPoint,
  type DrillLinePoint,
} from '../visualizations/drillChartModel'
import {
  colorizeHistogramSeriesData,
  resolveLightweightChartTheme,
  type LightweightChartTheme,
} from '../visualizations/chartTheme'
import { formatMarketTimestamp } from '../utils/marketTime'

interface DrillChartContext {
  chart: any
  chartModule: LightweightChartModule
  candle: any
  fast: any
  slow: any
  halftrend: any
  ott: any
  macz: any
  markerApi: any
}

interface DrillChartRenderPayload {
  token: number
  ctx: DrillChartContext
  model: DrillChartModel
  selectedTradeIdx: number
  candleData: DrillCandlePoint[]
  fastData: DrillLinePoint[]
  slowData: DrillLinePoint[]
  halftrendData: DrillHistogramPoint[]
  ottData: DrillLinePoint[]
  maczData: DrillHistogramPoint[]
  namedMode: boolean
  restoredLogicalRange: { from: number; to: number } | null
}

type DrillOverlayKey = 'fast' | 'slow' | 'halftrend' | 'ott' | 'macz'
type DrillOverlayRenderState = Record<DrillOverlayKey, boolean>

const DEFAULT_DRILL_META = 'Hover for OHLC/indicator values; use the Trades table to focus a trade.'
const MAX_NAMED_CROSSHAIR_VALUES = 6
const INSPECTOR_META_PREFIX = ' · inspect'
const chartValueFormatter = new Intl.NumberFormat('en-US', {
  maximumFractionDigits: 6,
})

export type DrillPriceChartVisibleSeriesIds =
  | ReadonlySet<string>
  | readonly string[]

interface DrillNamedSeriesPlan {
  active: boolean
  selectedCount: number
  renderableCount: number
  inputs: NamedChartSeriesInput[]
  selectedIds: ReadonlySet<string>
  structuralSignature: string
}

interface DrillNamedRenderState {
  active: boolean
  selectedCount: number
  renderableCount: number
  renderedCount: number
  failedCount: number
}

export type DrillPriceChartRenderState = 'idle' | 'loading' | 'ready' | 'error'
export type DrillPriceChartMarkerAvailability =
  | 'unknown'
  | 'unavailable'
  | 'display-only'
  | 'interactive'

export interface DrillPriceChartRuntime {
  setHost: (host: HTMLElement | null) => void
  render: (
    model: DrillChartModel | null,
    selectedTradeIdx: number,
    visibleSeriesIds?: DrillPriceChartVisibleSeriesIds,
  ) => Promise<void>
  updateSelectedTrade: (model: DrillChartModel | null, selectedTradeIdx: number) => void
  fitContent: () => boolean
  pan: (direction: 'left' | 'right') => boolean
  clear: (message?: string) => void
  dispose: () => void
}

export interface DrillPriceChartRuntimeOptions {
  onTradeMarkerSelect?: (tradeIdx: number) => void
  onMetaChange?: (message: string) => void
  onViewChange?: (label: string) => void
  onRenderStateChange?: (state: DrillPriceChartRenderState) => void
  onMarkerAvailabilityChange?: (availability: DrillPriceChartMarkerAvailability) => void
  loadChartModule?: () => Promise<LightweightChartModule>
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function markRawEntity<T>(value: T): T {
  if (value && typeof value === 'object') return markRaw(value as object) as T
  return value
}

function normalizedVisibleSeriesIds(
  visibleSeriesIds: DrillPriceChartVisibleSeriesIds | undefined,
): ReadonlySet<string> | null {
  if (visibleSeriesIds === undefined) return null
  const ids = new Set<string>()
  for (const idRaw of visibleSeriesIds) {
    const id = text(idRaw).toLowerCase()
    if (id) ids.add(id)
  }
  return ids
}

function buildNamedSeriesPlan(
  model: DrillChartModel,
  visibleSeriesIds: DrillPriceChartVisibleSeriesIds | undefined,
): DrillNamedSeriesPlan {
  const bundle = model.strategyIndicators
  if (!bundle) {
    return {
      active: false,
      selectedCount: 0,
      renderableCount: 0,
      inputs: [],
      selectedIds: new Set(),
      structuralSignature: 'legacy',
    }
  }

  const explicitIds = normalizedVisibleSeriesIds(visibleSeriesIds)
  const selectedSeries = bundle.series
    .map((series, colorIndex) => ({ series, colorIndex }))
    .filter(({ series }) => (
      explicitIds
        ? explicitIds.has(series.id)
        : series.defaultVisible
    ))
  const selectedIds = new Set(selectedSeries.map(({ series }) => series.id))
  const selectedRenderableSeries = selectedSeries
    .filter(({ series }) => series.finitePointCount > 0)
  const activePaneIds = new Set(
    selectedRenderableSeries.map(({ series }) =>
      String(series.paneId ?? '').trim().toLowerCase() || 'indicator',
    ),
  )
  const residentSeries = bundle.series
    .map((series, colorIndex) => ({ series, colorIndex }))
    .filter(({ series }) =>
      series.finitePointCount > 0 &&
      activePaneIds.has(
        String(series.paneId ?? '').trim().toLowerCase() || 'indicator',
      ),
    )
  const inputs: NamedChartSeriesInput[] = residentSeries
    .map(({ series, colorIndex }) => ({
      id: series.id,
      label: series.label,
      kind: series.kind,
      paneId: series.paneId,
      scaleId: series.scaleId,
      unit: series.unit,
      colorIndex,
      data: series.data,
      referenceLines: series.referenceLines,
      visible: selectedIds.has(series.id),
      resident: true,
    }))
  const structuralSignature = JSON.stringify({
    mode: 'named',
    strategyKind: bundle.strategyKind,
    series: residentSeries.map(({ series, colorIndex }) => ({
      id: series.id,
      label: series.label,
      kind: series.kind,
      paneId: series.paneId,
      scaleId: series.scaleId,
      unit: series.unit,
      colorIndex,
      referenceLines: series.referenceLines.map((line) => ({
        value: line.value,
        label: line.label,
      })),
    })),
  })
  return {
    active: true,
    selectedCount: selectedSeries.length,
    renderableCount: selectedRenderableSeries.length,
    inputs,
    selectedIds,
    structuralSignature,
  }
}

function setSeriesData(series: any, data: unknown[]): boolean {
  if (!series || typeof series.setData !== 'function') return true
  try {
    series.setData(data)
    return true
  } catch {
    return false
  }
}

function setOptionalSeriesData(series: any, data: unknown[]): boolean {
  if (!series || typeof series.setData !== 'function') return false
  return setSeriesData(series, data)
}

function clearSeries(series: any): void {
  setSeriesData(series, [])
}

function unavailableOverlayRenderState(): DrillOverlayRenderState {
  return {
    fast: false,
    slow: false,
    halftrend: false,
    ott: false,
    macz: false,
  }
}

function unavailableNamedRenderState(): DrillNamedRenderState {
  return {
    active: false,
    selectedCount: 0,
    renderableCount: 0,
    renderedCount: 0,
    failedCount: 0,
  }
}

function finiteNumber(value: unknown): number | null {
  if (value == null || (typeof value === 'string' && !value.trim())) return null
  const next = Number(value)
  return Number.isFinite(next) ? next : null
}

function seriesPoint(
  seriesData: unknown,
  series: unknown,
): Record<string, unknown> | null {
  if (
    !seriesData ||
    typeof (seriesData as { get?: unknown }).get !== 'function' ||
    !series
  ) {
    return null
  }
  try {
    const point = (seriesData as { get: (key: unknown) => unknown }).get(series)
    return point && typeof point === 'object'
      ? point as Record<string, unknown>
      : null
  } catch {
    return null
  }
}

function formatChartValue(value: number): string {
  return chartValueFormatter.format(value)
}

function formatChartMarketTime(time: number): string {
  return formatMarketTimestamp(time * 1_000, `Unix ${time}`)
}

function buildInspectorMeta(
  source: 'pointer' | 'keyboard',
  time: number | null,
  ohlc: {
    open: number | null
    high: number | null
    low: number | null
    close: number | null
  },
  values: readonly [label: string, value: number][],
  omittedValueCount = 0,
): string | null {
  const hasOhlc =
    ohlc.open != null &&
    ohlc.high != null &&
    ohlc.low != null &&
    ohlc.close != null
  if (!hasOhlc && values.length === 0) return null
  const parts = [
    ...(time == null ? [] : [formatChartMarketTime(time)]),
    ...(hasOhlc
      ? [
          `O ${formatChartValue(ohlc.open!)}`,
          `H ${formatChartValue(ohlc.high!)}`,
          `L ${formatChartValue(ohlc.low!)}`,
          `C ${formatChartValue(ohlc.close!)}`,
        ]
      : []),
    ...values.map(([label, value]) => `${label} ${formatChartValue(value)}`),
    ...(omittedValueCount > 0 ? [`+${omittedValueCount} more`] : []),
  ]
  return `${INSPECTOR_META_PREFIX}[${source}]: ${parts.join(' · ')}`
}

function buildCrosshairMeta(
  param: unknown,
  ctx: DrillChartContext,
  namedRuntime: NamedChartSeriesRuntime | null,
): string | null {
  const row = param && typeof param === 'object'
    ? param as Record<string, unknown>
    : {}
  const candlePoint = seriesPoint(row.seriesData, ctx.candle)
  const open = finiteNumber(candlePoint?.open)
  const high = finiteNumber(candlePoint?.high)
  const low = finiteNumber(candlePoint?.low)
  const close = finiteNumber(candlePoint?.close)
  let firstNamedPoint: Record<string, unknown> | null = null
  if (namedRuntime) {
    for (const entry of namedRuntime.registry.values()) {
      if (entry.input.visible === false) continue
      firstNamedPoint = seriesPoint(row.seriesData, entry.series)
      if (firstNamedPoint) break
    }
  }
  const time = finiteNumber(candlePoint?.time ?? firstNamedPoint?.time ?? row.time)
  const values: Array<[string, number]> = []
  const overlays: Array<[string, unknown]> = [
    ['Fast MA', ctx.fast],
    ['Slow MA', ctx.slow],
    ['HalfTrend', ctx.halftrend],
    ['OTT', ctx.ott],
    ['MAC-Z', ctx.macz],
  ]
  for (const [label, series] of overlays) {
    const value = finiteNumber(seriesPoint(row.seriesData, series)?.value)
    if (value != null) values.push([label, value])
  }
  let namedValueCount = 0
  let omittedNamedValueCount = 0
  if (namedRuntime) {
    for (const entry of namedRuntime.registry.values()) {
      if (entry.input.visible === false) continue
      const value = finiteNumber(seriesPoint(row.seriesData, entry.series)?.value)
      if (value == null) continue
      if (namedValueCount < MAX_NAMED_CROSSHAIR_VALUES) {
        values.push([entry.input.label, value])
      } else {
        omittedNamedValueCount += 1
      }
      namedValueCount += 1
    }
  }
  return buildInspectorMeta(
    'pointer',
    time,
    { open, high, low, close },
    values,
    omittedNamedValueCount,
  )
}

function dataValueAtTime(
  data: readonly { time: unknown; value?: number | null }[],
  time: number,
): number | null {
  for (const point of data) {
    if (finiteNumber(point.time) !== time) continue
    return finiteNumber(point.value)
  }
  return null
}

function buildKeyboardInspectorMeta(
  model: DrillChartModel,
  index: number,
  namedRuntime: NamedChartSeriesRuntime | null,
): string | null {
  const candle = model.candleData[index]
  if (!candle) return null
  const values: Array<[string, number]> = []
  if (namedRuntime) {
    let namedValueCount = 0
    let omittedNamedValueCount = 0
    for (const entry of namedRuntime.registry.values()) {
      if (entry.input.visible === false) continue
      const value = dataValueAtTime(entry.input.data, candle.time)
      if (value == null) continue
      if (namedValueCount < MAX_NAMED_CROSSHAIR_VALUES) {
        values.push([entry.input.label, value])
      } else {
        omittedNamedValueCount += 1
      }
      namedValueCount += 1
    }
    return buildInspectorMeta(
      'keyboard',
      candle.time,
      candle,
      values,
      omittedNamedValueCount,
    )
  }
  const overlays: Array<[string, readonly DrillLinePoint[]]> = [
    ['Fast MA', model.fastData],
    ['Slow MA', model.slowData],
    ['HalfTrend', model.halftrendData],
    ['OTT', model.ottData],
    ['MAC-Z', model.maczData],
  ]
  for (const [label, data] of overlays) {
    const value = dataValueAtTime(data, candle.time)
    if (value != null) values.push([label, value])
  }
  return buildInspectorMeta(
    'keyboard',
    candle.time,
    candle,
    values,
  )
}

function buildRenderMeta(
  model: DrillChartModel,
  selectedTradeIdx: number,
  markersAvailable: boolean,
  overlays: DrillOverlayRenderState,
  named: DrillNamedRenderState,
): string {
  const tradeMarkers = buildDrillTradeMarkers(model, selectedTradeIdx)
  const markers = tradeMarkers.length
  const entries = tradeMarkers.filter((marker) => marker.id.endsWith('-entry')).length
  const exits = tradeMarkers.filter((marker) => marker.id.endsWith('-exit')).length
  const selected =
    selectedTradeIdx >= 0 && selectedTradeIdx < model.trades.length
      ? `trade-${selectedTradeIdx + 1}`
      : 'none'
  const markerMeta = markersAvailable
    ? [
        `entries=${entries}`,
        `exits=${exits}`,
        `markers=${markers}`,
      ]
    : ['markers=unavailable']
  const indicatorMeta = !named.active
    ? []
    : named.selectedCount === 0
      ? ['indicators=hidden']
      : named.renderableCount === 0
        ? ['indicators=unavailable']
        : [
            `indicators=${named.renderedCount}/${named.selectedCount}`,
            ...(named.failedCount > 0 ? [`indicator_failures=${named.failedCount}`] : []),
          ]
  const legacyOverlayMeta = named.active
    ? []
    : [
        `fast=${overlays.fast ? model.fastData.length : 'unavailable'}`,
        `slow=${overlays.slow ? model.slowData.length : 'unavailable'}`,
        `halftrend=${overlays.halftrend ? model.halftrendData.length : 'unavailable'}`,
        `ott=${overlays.ott ? model.ottData.length : 'unavailable'}`,
        `macz=${overlays.macz ? model.maczData.length : 'unavailable'}`,
      ]
  return [
    `candles=${model.candleData.length}`,
    ...legacyOverlayMeta,
    `trades=${model.trades.length}`,
    ...markerMeta,
    ...indicatorMeta,
    `selected=${selected}`,
  ].join(' ')
}

function chartBarIndexForTradeIndex(
  model: DrillChartModel,
  indexRaw: unknown,
): number | null {
  const indexValue = finiteNumber(indexRaw)
  if (indexValue == null || !model.bars.length) return null
  const index = Math.max(0, Math.floor(indexValue))
  let nearestIndex = 0
  let nearestDistance = Number.POSITIVE_INFINITY
  for (let barIndex = 0; barIndex < model.bars.length; barIndex += 1) {
    const bar = model.bars[barIndex]
    if (!bar) continue
    const fromIndex = Math.floor(Number(bar.fromIndex))
    const toIndex = Math.floor(Number(bar.toIndex))
    if (!Number.isFinite(fromIndex) || !Number.isFinite(toIndex)) continue
    if (index >= fromIndex && index <= toIndex) return barIndex
    const distance = index < fromIndex ? fromIndex - index : index - toIndex
    if (distance < nearestDistance) {
      nearestDistance = distance
      nearestIndex = barIndex
    }
  }
  return Number.isFinite(nearestDistance) ? nearestIndex : null
}

function selectedTradeLogicalRange(
  model: DrillChartModel,
  selectedTradeIdx: number,
): { from: number; to: number } | null {
  const trade =
    selectedTradeIdx >= 0 && selectedTradeIdx < model.trades.length
      ? model.trades[selectedTradeIdx]
      : null
  if (!trade || !model.bars.length) return null
  const entryIndex = chartBarIndexForTradeIndex(model, trade.entry_t)
  if (entryIndex == null) return null
  const exitIndex = trade.open
    ? null
    : chartBarIndexForTradeIndex(model, trade.exit_t)
  const tradeStart = Math.min(entryIndex, exitIndex ?? entryIndex)
  const tradeEnd = Math.max(entryIndex, exitIndex ?? entryIndex)
  const tradeSpan = Math.max(1, tradeEnd - tradeStart)
  const padding = Math.max(2, Math.ceil(tradeSpan * 0.2))
  let from = tradeStart - padding
  let to = tradeEnd + padding
  const minimumWidth = Math.min(12, Math.max(1, model.bars.length))
  if (to - from < minimumWidth) {
    const center = (tradeStart + tradeEnd) / 2
    from = center - minimumWidth / 2
    to = center + minimumWidth / 2
  }
  from = Math.max(-0.5, from)
  to = Math.min(model.bars.length - 0.5, to)
  return to > from ? { from, to } : null
}

function currentVisibleLogicalRange(
  chart: unknown,
): { from: number; to: number } | null {
  if (!chart || typeof (chart as { timeScale?: unknown }).timeScale !== 'function') {
    return null
  }
  try {
    const timeScale = (chart as { timeScale: () => any }).timeScale()
    if (!timeScale || typeof timeScale.getVisibleLogicalRange !== 'function') return null
    const range = timeScale.getVisibleLogicalRange()
    const from = finiteNumber(range?.from)
    const to = finiteNumber(range?.to)
    return from != null && to != null && to > from ? { from, to } : null
  } catch {
    return null
  }
}

function restoreVisibleLogicalRange(
  chart: unknown,
  range: { from: number; to: number } | null,
): boolean {
  if (
    !range ||
    !chart ||
    typeof (chart as { timeScale?: unknown }).timeScale !== 'function'
  ) {
    return false
  }
  try {
    const timeScale = (chart as { timeScale: () => any }).timeScale()
    if (!timeScale || typeof timeScale.setVisibleLogicalRange !== 'function') return false
    timeScale.setVisibleLogicalRange(range)
    return true
  } catch {
    return false
  }
}

export function useDrillPriceChartRuntime(
  options: DrillPriceChartRuntimeOptions = {},
): DrillPriceChartRuntime {
  const chartRuntime = useLightweightChart<any>()
  let modulePromise: Promise<LightweightChartModule | null> | null = null
  let activeChartModule: LightweightChartModule | null = null
  let candleSeries: any = null
  let fastSeries: any = null
  let slowSeries: any = null
  let halftrendSeries: any = null
  let ottSeries: any = null
  let maczSeries: any = null
  let markerApi: any = null
  let clickHandler: ((param: unknown) => void) | null = null
  let crosshairHandler: ((param: unknown) => void) | null = null
  let rangeHandler: (() => void) | null = null
  let hostKeydownHandler: ((event: KeyboardEvent) => void) | null = null
  let keyboardInspectionIndex: number | null = null
  let stopThemeObserver: (() => void) | null = null
  let namedSeriesRuntime: NamedChartSeriesRuntime | null = null
  let activeStructureSignature: string | null = null
  let renderFrame = 0
  let pendingPayload: DrillChartRenderPayload | null = null
  let renderToken = 0
  let chartGeneration = 0
  let chartCreationEpoch = 0
  let chartCreationHost: HTMLElement | null = null
  let chartCreationPromise: Promise<DrillChartContext | null> | null = null
  let currentModel: DrillChartModel | null = null
  let currentSelectedTradeIdx = -1
  let restingMeta = DEFAULT_DRILL_META
  let renderState: DrillPriceChartRenderState = 'idle'
  let markerAvailability: DrillPriceChartMarkerAvailability = 'unknown'
  let markerInteractionReady = false
  let overlayRenderState = unavailableOverlayRenderState()
  let namedRenderState = unavailableNamedRenderState()

  function setMeta(message: string): void {
    options.onMetaChange?.(message)
  }

  function setRestingMeta(message: string): void {
    restingMeta = message
    setMeta(message)
  }

  function setRenderState(state: DrillPriceChartRenderState): void {
    if (renderState === state) return
    renderState = state
    options.onRenderStateChange?.(state)
  }

  function setMarkerAvailability(
    availability: DrillPriceChartMarkerAvailability,
  ): void {
    if (markerAvailability === availability) return
    markerAvailability = availability
    options.onMarkerAvailabilityChange?.(availability)
  }

  function markersAreAvailable(): boolean {
    return markerAvailability === 'display-only' || markerAvailability === 'interactive'
  }

  function resolvedMarkerAvailability(): DrillPriceChartMarkerAvailability {
    if (!markerApi || typeof markerApi.setMarkers !== 'function') return 'unavailable'
    return markerInteractionReady ? 'interactive' : 'display-only'
  }

  function setTradeMarkers(markers: unknown[]): boolean {
    if (!markerApi || typeof markerApi.setMarkers !== 'function') {
      markerInteractionReady = false
      setMarkerAvailability('unavailable')
      return false
    }
    try {
      markerApi.setMarkers(markers)
      setMarkerAvailability(resolvedMarkerAvailability())
      return true
    } catch {
      markerApi = null
      markerInteractionReady = false
      removeClickHandler()
      setMarkerAvailability('unavailable')
      return false
    }
  }

  function publishViewLabel(): void {
    options.onViewChange?.(visibleLogicalRangeLabel(chartRuntime.chartRef.value))
  }

  function clearProgrammaticCrosshair(): void {
    const chart = chartRuntime.chartRef.value
    if (!chart || typeof chart.clearCrosshairPosition !== 'function') return
    try {
      chart.clearCrosshairPosition()
    } catch {}
  }

  function ensureInspectionIndexVisible(index: number): void {
    const chart = chartRuntime.chartRef.value
    const range = currentVisibleLogicalRange(chart)
    if (!chart || !range || (index >= range.from && index <= range.to)) return
    const width = Math.max(1, range.to - range.from)
    const from = Math.max(-0.5, index - width / 2)
    const to = from + width
    try {
      const timeScale = chart.timeScale?.()
      if (typeof timeScale?.setVisibleLogicalRange === 'function') {
        timeScale.setVisibleLogicalRange({ from, to })
        publishViewLabel()
      }
    } catch {}
  }

  function inspectChartPoint(indexRaw: number): boolean {
    const model = currentModel
    const chart = chartRuntime.chartRef.value
    if (!model?.candleData.length || !chart || !candleSeries) return false
    const index = Math.max(
      0,
      Math.min(model.candleData.length - 1, Math.trunc(indexRaw)),
    )
    const candle = model.candleData[index]
    const inspectionMeta = buildKeyboardInspectorMeta(
      model,
      index,
      namedSeriesRuntime,
    )
    if (!candle || !inspectionMeta) return false
    keyboardInspectionIndex = index
    ensureInspectionIndexVisible(index)
    if (typeof chart.setCrosshairPosition === 'function') {
      try {
        chart.setCrosshairPosition(candle.close, candle.time, candleSeries)
      } catch {}
    }
    setMeta(`${restingMeta}${inspectionMeta}`)
    return true
  }

  function initialKeyboardInspectionIndex(
    direction: 'backward' | 'forward',
  ): number {
    const model = currentModel
    if (!model?.candleData.length) return 0
    const range = currentVisibleLogicalRange(chartRuntime.chartRef.value)
    if (!range) {
      return direction === 'backward' ? model.candleData.length - 1 : 0
    }
    return direction === 'backward'
      ? Math.min(model.candleData.length - 1, Math.max(0, Math.floor(range.to)))
      : Math.min(model.candleData.length - 1, Math.max(0, Math.ceil(range.from)))
  }

  function handleHostKeydown(event: KeyboardEvent): void {
    const model = currentModel
    if (!model?.candleData.length || renderState !== 'ready') return
    let nextIndex: number | null = null
    if (event.key === 'Home') {
      nextIndex = 0
    } else if (event.key === 'End') {
      nextIndex = model.candleData.length - 1
    } else if (event.key === 'ArrowLeft') {
      nextIndex = keyboardInspectionIndex == null
        ? initialKeyboardInspectionIndex('backward')
        : keyboardInspectionIndex - 1
    } else if (event.key === 'ArrowRight') {
      nextIndex = keyboardInspectionIndex == null
        ? initialKeyboardInspectionIndex('forward')
        : keyboardInspectionIndex + 1
    } else if (event.key === 'Escape') {
      event.preventDefault()
      keyboardInspectionIndex = null
      clearProgrammaticCrosshair()
      setMeta(restingMeta)
      return
    } else {
      return
    }
    event.preventDefault()
    inspectChartPoint(nextIndex)
  }

  function attachHostKeyboardHandler(host: HTMLElement): void {
    if (hostKeydownHandler) return
    hostKeydownHandler = handleHostKeydown
    host.addEventListener('keydown', hostKeydownHandler)
  }

  function removeHostKeyboardHandler(): void {
    const host = chartRuntime.hostRef.value
    if (host && hostKeydownHandler) {
      host.removeEventListener('keydown', hostKeydownHandler)
    }
    hostKeydownHandler = null
    keyboardInspectionIndex = null
  }

  async function loadModule(): Promise<LightweightChartModule | null> {
    if (!modulePromise) {
      const load = options.loadChartModule
        ? (): Promise<unknown> => options.loadChartModule!()
        : (): Promise<unknown> => import('lightweight-charts')
      const pending = Promise.resolve()
        .then(load)
        .then((module) => module as LightweightChartModule)
        .catch(() => null)
      modulePromise = pending
      const loaded = await pending
      if (!loaded && modulePromise === pending) modulePromise = null
      return loaded
    }
    return modulePromise
  }

  function cancelRenderFrame(): void {
    if (
      renderFrame &&
      typeof window !== 'undefined' &&
      typeof window.cancelAnimationFrame === 'function'
    ) {
      window.cancelAnimationFrame(renderFrame)
    }
    renderFrame = 0
    pendingPayload = null
  }

  function removeClickHandler(): void {
    const chart = chartRuntime.chartRef.value
    if (
      chart &&
      clickHandler &&
      typeof (chart as { unsubscribeClick?: unknown }).unsubscribeClick === 'function'
    ) {
      try {
        ;(chart as { unsubscribeClick: (handler: (param: unknown) => void) => void })
          .unsubscribeClick(clickHandler)
      } catch {}
    }
    clickHandler = null
  }

  function removeCrosshairHandler(): void {
    const chart = chartRuntime.chartRef.value
    if (
      chart &&
      crosshairHandler &&
      typeof (chart as { unsubscribeCrosshairMove?: unknown }).unsubscribeCrosshairMove === 'function'
    ) {
      try {
        ;(chart as { unsubscribeCrosshairMove: (handler: (param: unknown) => void) => void })
          .unsubscribeCrosshairMove(crosshairHandler)
      } catch {}
    }
    crosshairHandler = null
  }

  function removeRangeHandler(): void {
    const chart = chartRuntime.chartRef.value
    if (
      chart &&
      rangeHandler &&
      typeof (chart as { timeScale?: unknown }).timeScale === 'function'
    ) {
      try {
        const timeScale = (chart as { timeScale: () => unknown }).timeScale()
        if (timeScale && typeof (timeScale as { unsubscribeVisibleLogicalRangeChange?: unknown }).unsubscribeVisibleLogicalRangeChange === 'function') {
          ;(timeScale as { unsubscribeVisibleLogicalRangeChange: (handler: () => void) => void })
            .unsubscribeVisibleLogicalRangeChange(rangeHandler)
        }
      } catch {}
    }
    rangeHandler = null
  }

  function currentChartContext(): DrillChartContext | null {
    const chart = chartRuntime.chartRef.value
    if (!chart || !candleSeries || !activeChartModule) return null
    return {
      chart,
      chartModule: activeChartModule,
      candle: candleSeries,
      fast: fastSeries,
      slow: slowSeries,
      halftrend: halftrendSeries,
      ott: ottSeries,
      macz: maczSeries,
      markerApi,
    }
  }

  function removeDetachedChart(chart: unknown): void {
    if (!chart || typeof (chart as { remove?: unknown }).remove !== 'function') return
    try {
      ;(chart as { remove: () => void }).remove()
    } catch {}
  }

  function teardownActiveChart(clearHost: boolean): void {
    chartGeneration += 1
    chartCreationEpoch += 1
    chartCreationHost = null
    chartCreationPromise = null
    cancelRenderFrame()
    removeClickHandler()
    removeCrosshairHandler()
    removeRangeHandler()
    stopThemeObserver?.()
    stopThemeObserver = null
    namedSeriesRuntime?.dispose()
    namedSeriesRuntime = null
    chartRuntime.stopHostResize()
    removeDetachedChart(chartRuntime.chartRef.value)
    chartRuntime.setChart(null)
    if (clearHost) {
      removeHostKeyboardHandler()
      chartRuntime.setHost(null)
    }
    activeChartModule = null
    candleSeries = null
    fastSeries = null
    slowSeries = null
    halftrendSeries = null
    ottSeries = null
    maczSeries = null
    markerApi = null
    markerInteractionReady = false
    overlayRenderState = unavailableOverlayRenderState()
    namedRenderState = unavailableNamedRenderState()
    activeStructureSignature = null
  }

  function dispose(): void {
    renderToken += 1
    teardownActiveChart(true)
    currentModel = null
    currentSelectedTradeIdx = -1
    restingMeta = DEFAULT_DRILL_META
    setRenderState('idle')
    setMarkerAvailability('unknown')
    options.onViewChange?.('')
  }

  function setHost(host: HTMLElement | null): void {
    const existingHost = chartRuntime.hostRef.value
    if (!host) {
      if (existingHost) dispose()
      chartRuntime.setHost(null)
      return
    }
    if (existingHost && existingHost !== host) dispose()
    chartRuntime.setHost(host)
    attachHostKeyboardHandler(host)
  }

  function updateSelectedTrade(
    model: DrillChartModel | null = currentModel,
    selectedTradeIdx = currentSelectedTradeIdx,
  ): void {
    if (renderState === 'error') return
    currentModel = model
    currentSelectedTradeIdx = selectedTradeIdx
    if (!model) return
    setTradeMarkers(
      buildDrillTradeMarkers(model, selectedTradeIdx, resolveLightweightChartTheme()),
    )
    const range = selectedTradeLogicalRange(model, selectedTradeIdx)
    const chart = chartRuntime.chartRef.value
    if (
      range &&
      chart &&
      typeof chart.timeScale === 'function'
    ) {
      try {
        const timeScale = chart.timeScale()
        if (timeScale && typeof timeScale.setVisibleLogicalRange === 'function') {
          timeScale.setVisibleLogicalRange(range)
          publishViewLabel()
        }
      } catch {}
    }
    setRestingMeta(
      buildRenderMeta(
        model,
        selectedTradeIdx,
        markersAreAvailable(),
        overlayRenderState,
        namedRenderState,
      ),
    )
  }

  function applyThemeBoundData(theme: LightweightChartTheme): void {
    if (renderState === 'error') return
    const model = currentModel
    if (!model) return
    namedSeriesRuntime?.reapplyTheme(theme)
    if (!namedRenderState.active) {
      const halftrendRendered = setOptionalSeriesData(
        halftrendSeries,
        colorizeHistogramSeriesData(model.halftrendData, theme),
      )
      const maczRendered = setOptionalSeriesData(
        maczSeries,
        colorizeHistogramSeriesData(model.maczData, theme),
      )
      overlayRenderState = {
        ...overlayRenderState,
        halftrend: halftrendRendered,
        macz: maczRendered,
      }
    }
    setTradeMarkers(
      buildDrillTradeMarkers(model, currentSelectedTradeIdx, theme),
    )
    setRestingMeta(
      buildRenderMeta(
        model,
        currentSelectedTradeIdx,
        markersAreAvailable(),
        overlayRenderState,
        namedRenderState,
      ),
    )
  }

  function flushRenderFrame(): void {
    renderFrame = 0
    const payload = pendingPayload
    pendingPayload = null
    if (!payload || payload.token !== renderToken) return
    const { ctx, model, selectedTradeIdx } = payload
    if (!setSeriesData(ctx.candle, payload.candleData)) {
      clear('Unable to render drill chart.', 'error')
      return
    }
    const theme = resolveLightweightChartTheme()
    if (payload.namedMode) {
      clearSeries(ctx.fast)
      clearSeries(ctx.slow)
      clearSeries(ctx.halftrend)
      clearSeries(ctx.ott)
      clearSeries(ctx.macz)
      overlayRenderState = unavailableOverlayRenderState()
    } else {
      overlayRenderState = {
        fast: setOptionalSeriesData(ctx.fast, payload.fastData),
        slow: setOptionalSeriesData(ctx.slow, payload.slowData),
        halftrend: setOptionalSeriesData(
          ctx.halftrend,
          colorizeHistogramSeriesData(payload.halftrendData, theme),
        ),
        ott: setOptionalSeriesData(ctx.ott, payload.ottData),
        macz: setOptionalSeriesData(
          ctx.macz,
          colorizeHistogramSeriesData(payload.maczData, theme),
        ),
      }
    }

    fitLightweightChartContent(ctx.chart)
    updateSelectedTrade(model, selectedTradeIdx)
    restoreVisibleLogicalRange(ctx.chart, payload.restoredLogicalRange)
    publishViewLabel()

    setRenderState('ready')
    if (keyboardInspectionIndex != null) {
      inspectChartPoint(keyboardInspectionIndex)
    }
  }

  function scheduleRenderFrame(payload: DrillChartRenderPayload): void {
    pendingPayload = payload
    if (renderFrame) return
    if (
      typeof window === 'undefined' ||
      typeof window.requestAnimationFrame !== 'function'
    ) {
      flushRenderFrame()
      return
    }
    renderFrame = window.requestAnimationFrame(() => {
      flushRenderFrame()
    })
  }

  async function createChartForHost(
    host: HTMLElement,
    generation: number,
    creationEpoch: number,
  ): Promise<DrillChartContext | null> {
    const chartModule = await loadModule()
    if (!chartModule || typeof chartModule.createChart !== 'function') return null
    if (
      generation !== chartGeneration ||
      creationEpoch !== chartCreationEpoch ||
      chartRuntime.hostRef.value !== host ||
      chartCreationHost !== host
    ) {
      return null
    }

    const existing = currentChartContext()
    if (existing) return existing

    const runtime = createDrillSeriesRuntime(host, chartModule)
    if (!runtime) return null
    const chartAny = runtime.chart

    if (
      generation !== chartGeneration ||
      creationEpoch !== chartCreationEpoch ||
      chartRuntime.hostRef.value !== host ||
      chartCreationHost !== host
    ) {
      removeDetachedChart(chartAny)
      return null
    }

    candleSeries = markRawEntity(runtime.candleSeries)
    fastSeries = markRawEntity(runtime.fastSeries)
    slowSeries = markRawEntity(runtime.slowSeries)
    halftrendSeries = markRawEntity(runtime.halftrendSeries)
    ottSeries = markRawEntity(runtime.ottSeries)
    maczSeries = markRawEntity(runtime.maczSeries)
    markerApi = markRawEntity(runtime.markerApi)
    activeChartModule = chartModule
    activeStructureSignature = 'legacy'
    markerInteractionReady = false
    chartRuntime.setChart(markRawEntity(chartAny))
    stopThemeObserver = observeLightweightChartTheme(chartAny, {
      series: runtime,
      initialTheme: runtime.theme,
      onThemeChange: (theme) => {
        if (
          generation !== chartGeneration ||
          chartRuntime.chartRef.value !== chartAny
        ) {
          return
        }
        applyThemeBoundData(theme)
      },
    })

    if (
      markerApi &&
      typeof markerApi.setMarkers === 'function' &&
      typeof chartAny.subscribeClick === 'function'
    ) {
      clickHandler = (param: unknown) => {
        if (
          generation !== chartGeneration ||
          chartRuntime.chartRef.value !== chartAny
        ) {
          return
        }
        const row = param && typeof param === 'object' ? (param as Record<string, unknown>) : {}
        const markerId = text(row.hoveredObjectId ?? row.hoveredMarkerId)
        const match = /^trade-(\d+)(?:-(?:entry|exit))?$/u.exec(markerId)
        if (!match) return
        options.onTradeMarkerSelect?.(Number(match[1]))
      }
      try {
        chartAny.subscribeClick(clickHandler)
        markerInteractionReady = true
      } catch {
        clickHandler = null
      }
    }
    setMarkerAvailability(resolvedMarkerAvailability())

    if (typeof chartAny.subscribeCrosshairMove === 'function') {
      const crosshairContext = currentChartContext()
      crosshairHandler = (param: unknown) => {
        if (
          generation !== chartGeneration ||
          chartRuntime.chartRef.value !== chartAny
        ) {
          return
        }
        const hoverMeta = crosshairContext
          ? buildCrosshairMeta(param, crosshairContext, namedSeriesRuntime)
          : null
        if (hoverMeta && currentModel) {
          const row = param && typeof param === 'object'
            ? param as Record<string, unknown>
            : {}
          const hoverTime = finiteNumber(row.time)
          if (hoverTime != null) {
            const nextIndex = currentModel.candleData.findIndex(
              (point) => point.time === hoverTime,
            )
            if (nextIndex >= 0) keyboardInspectionIndex = nextIndex
          }
        }
        setMeta(hoverMeta ? `${restingMeta}${hoverMeta}` : restingMeta)
      }
      try {
        chartAny.subscribeCrosshairMove(crosshairHandler)
      } catch {
        crosshairHandler = null
      }
    }

    try {
      const timeScale = chartAny.timeScale && chartAny.timeScale()
      if (timeScale && typeof timeScale.subscribeVisibleLogicalRangeChange === 'function') {
        rangeHandler = () => {
          if (
            generation === chartGeneration &&
            chartRuntime.chartRef.value === chartAny
          ) {
            publishViewLabel()
          }
        }
        timeScale.subscribeVisibleLogicalRangeChange(rangeHandler)
      }
    } catch {}

    try {
      chartRuntime.observeHostResize((rect) => {
        const chartRef = chartRuntime.chartRef.value
        if (
          generation !== chartGeneration ||
          chartRuntime.hostRef.value !== host ||
          chartRef !== chartAny ||
          !rect.width ||
          !rect.height
        ) {
          return
        }
        try {
          applyLightweightChartSize(chartRef, rect.width, rect.height)
        } catch {}
      })
    } catch {
      chartRuntime.stopHostResize()
    }

    return currentChartContext()
  }

  async function ensureChart(): Promise<DrillChartContext | null> {
    const host = chartRuntime.hostRef.value
    if (!host) return null

    const existing = currentChartContext()
    if (existing) return existing
    if (chartCreationPromise && chartCreationHost === host) return chartCreationPromise

    const generation = chartGeneration
    const creationEpoch = chartCreationEpoch + 1
    chartCreationEpoch = creationEpoch
    const pending = createChartForHost(host, generation, creationEpoch).catch(() => null)
    chartCreationHost = host
    chartCreationPromise = pending
    try {
      return await pending
    } finally {
      if (chartCreationPromise === pending) {
        chartCreationPromise = null
        chartCreationHost = null
      }
    }
  }

  function replaceNamedSeriesRuntime(
    ctx: DrillChartContext,
    plan: DrillNamedSeriesPlan,
  ): void {
    namedSeriesRuntime?.dispose()
    namedSeriesRuntime = null
    namedRenderState = {
      active: plan.active,
      selectedCount: plan.selectedCount,
      renderableCount: plan.renderableCount,
      renderedCount: 0,
      failedCount: 0,
    }
    activeStructureSignature = plan.structuralSignature
    if (!plan.active || plan.inputs.length === 0) return

    try {
      const runtime = createNamedSeriesRuntime(
        ctx.chart,
        ctx.chartModule,
        plan.inputs,
        resolveLightweightChartTheme(),
      )
      namedSeriesRuntime = runtime
      updateNamedRenderState(plan, runtime)
    } catch {
      namedRenderState = {
        ...namedRenderState,
        failedCount: Math.max(1, plan.inputs.length),
      }
    }
  }

  function updateNamedRenderState(
    plan: DrillNamedSeriesPlan,
    runtime: NamedChartSeriesRuntime | null,
  ): void {
    const blockingFailureIds = new Set(
      (runtime?.failures ?? [])
        .filter((failure) => (
          failure.stage === 'series' ||
          failure.stage === 'data' ||
          failure.stage === 'visibility'
        ))
        .map((failure) => failure.seriesId),
    )
    namedRenderState = {
      active: plan.active,
      selectedCount: plan.selectedCount,
      renderableCount: plan.renderableCount,
      renderedCount: runtime
        ? Array.from(plan.selectedIds)
            .filter((id) => runtime.registry.has(id) && !blockingFailureIds.has(id))
            .length
        : 0,
      failedCount: runtime?.failures.length ?? 0,
    }
  }

  function applyIncrementalNamedVisibility(
    model: DrillChartModel,
    selectedTradeIdx: number,
    plan: DrillNamedSeriesPlan,
  ): boolean {
    if (
      !plan.active ||
      !namedRenderState.active ||
      activeStructureSignature !== plan.structuralSignature ||
      !chartRuntime.chartRef.value
    ) {
      return false
    }
    if (plan.inputs.length > 0) {
      if (!namedSeriesRuntime) return false
      if (!namedSeriesRuntime.setVisibleSeriesIds(plan.selectedIds)) return false
    } else if (namedSeriesRuntime) {
      return false
    }
    updateNamedRenderState(plan, namedSeriesRuntime)
    currentModel = model
    const selectionChanged = currentSelectedTradeIdx !== selectedTradeIdx
    currentSelectedTradeIdx = selectedTradeIdx
    if (selectionChanged) {
      updateSelectedTrade(model, selectedTradeIdx)
    } else {
      setRestingMeta(
        buildRenderMeta(
          model,
          selectedTradeIdx,
          markersAreAvailable(),
          overlayRenderState,
          namedRenderState,
        ),
      )
    }
    if (keyboardInspectionIndex != null) {
      inspectChartPoint(keyboardInspectionIndex)
    }
    setRenderState('ready')
    return true
  }

  function clear(
    message = DEFAULT_DRILL_META,
    nextState: DrillPriceChartRenderState = 'idle',
  ): void {
    renderToken += 1
    chartCreationEpoch += 1
    chartCreationHost = null
    chartCreationPromise = null
    cancelRenderFrame()
    keyboardInspectionIndex = null
    clearProgrammaticCrosshair()
    namedSeriesRuntime?.dispose()
    namedSeriesRuntime = null
    activeStructureSignature = chartRuntime.chartRef.value ? 'legacy' : null
    setRestingMeta(message)
    clearSeries(candleSeries)
    clearSeries(fastSeries)
    clearSeries(slowSeries)
    clearSeries(halftrendSeries)
    clearSeries(ottSeries)
    clearSeries(maczSeries)
    if (markerApi && typeof markerApi.setMarkers === 'function') {
      setTradeMarkers([])
    }
    currentModel = null
    currentSelectedTradeIdx = -1
    overlayRenderState = unavailableOverlayRenderState()
    namedRenderState = unavailableNamedRenderState()
    setRenderState(nextState)
    options.onViewChange?.('')
  }

  function fitContent(): boolean {
    const ok = fitLightweightChartContent(chartRuntime.chartRef.value)
    publishViewLabel()
    return ok
  }

  function pan(direction: 'left' | 'right'): boolean {
    const ok = panLightweightChart(chartRuntime.chartRef.value, direction)
    publishViewLabel()
    return ok
  }

  async function render(
    model: DrillChartModel | null,
    selectedTradeIdx: number,
    visibleSeriesIds?: DrillPriceChartVisibleSeriesIds,
  ): Promise<void> {
    const token = renderToken + 1
    renderToken = token
    if (!model) {
      clear()
      return
    }
    const previousModel = currentModel
    const previousSelectedTradeIdx = currentSelectedTradeIdx
    const plan = buildNamedSeriesPlan(model, visibleSeriesIds)
    cancelRenderFrame()
    if (
      previousModel === model &&
      applyIncrementalNamedVisibility(model, selectedTradeIdx, plan)
    ) {
      return
    }
    let restoredLogicalRange: { from: number; to: number } | null = null
    if (
      chartRuntime.chartRef.value &&
      activeStructureSignature !== null &&
      activeStructureSignature !== plan.structuralSignature
    ) {
      if (previousSelectedTradeIdx === selectedTradeIdx) {
        restoredLogicalRange = currentVisibleLogicalRange(chartRuntime.chartRef.value)
      }
      teardownActiveChart(false)
    }
    currentModel = model
    if (previousModel !== model) keyboardInspectionIndex = null
    currentSelectedTradeIdx = selectedTradeIdx
    setRenderState('loading')
    if (!model.bars.length) {
      clear('No valid OHLC bars available for chart rendering.', 'error')
      return
    }
    if (!model.candleData.length) {
      clear('No candle points generated from loaded bars.', 'error')
      return
    }
    const ctx = await ensureChart()
    if (!ctx || token !== renderToken) {
      if (token === renderToken) {
        setRestingMeta('Unable to create drill chart runtime.')
        setRenderState('error')
      }
      return
    }
    replaceNamedSeriesRuntime(ctx, plan)
    if (token !== renderToken) return
    scheduleRenderFrame({
      token,
      ctx,
      model,
      selectedTradeIdx,
      candleData: model.candleData,
      fastData: model.fastData,
      slowData: model.slowData,
      halftrendData: model.halftrendData,
      ottData: model.ottData,
      maczData: model.maczData,
      namedMode: plan.active,
      restoredLogicalRange,
    })
  }

  return {
    setHost,
    render,
    updateSelectedTrade,
    fitContent,
    pan,
    clear,
    dispose,
  }
}
