import {
  baseLightweightChartOptions,
  candlestickSeriesOptions,
  histogramPriceScaleOptions,
  histogramSeriesOptions,
  lightweightChartSeriesPalette,
  lightweightChartThemeOptions,
  lineSeriesOptions,
  normalizeChartSize,
  resolveLightweightChartTheme,
  type HistogramSeriesKind,
  type LightweightChartSeriesTone,
  type LightweightChartTheme,
} from './chartTheme'

export interface LightweightChartModule {
  createChart?: (host: HTMLElement, options: Record<string, unknown>) => any
  CandlestickSeries?: unknown
  LineSeries?: unknown
  HistogramSeries?: unknown
  createSeriesMarkers?: (series: any, markers?: unknown[]) => { setMarkers?: (markers: unknown[]) => void } | null
}

export interface CandlestickRuntime {
  chart: any
  candleSeries: any
  markerApi: { setMarkers?: (markers: unknown[]) => void } | null
  theme: LightweightChartTheme
}

export interface DrillSeriesRuntime extends CandlestickRuntime {
  fastSeries: any
  slowSeries: any
  halftrendSeries: any
  ottSeries: any
  maczSeries: any
}

export interface LightweightChartSeriesThemeTargets {
  candleSeries?: unknown
  fastSeries?: unknown
  slowSeries?: unknown
  halftrendSeries?: unknown
  ottSeries?: unknown
  maczSeries?: unknown
}

export interface LightweightChartThemeObserverOptions {
  root?: HTMLElement | null
  series?: LightweightChartSeriesThemeTargets
  initialTheme?: LightweightChartTheme
  onThemeChange?: (theme: LightweightChartTheme) => void
}

export type NamedChartSeriesKind = 'line' | 'histogram'

export interface NamedChartSeriesDataPoint {
  time: unknown
  value?: number | null
  color?: string
}

export interface NamedChartReferenceLineInput {
  value: number
  label?: string
  colorIndex?: number
  lineStyle?: number
  lineWidth?: number
  axisLabelVisible?: boolean
}

/**
 * Low-level, model-agnostic input for a strategy-owned chart series.
 *
 * Callers should pass only the series that are currently visible. `visible`
 * remains available as a defensive convenience for consumers that retain a
 * complete bundle while toggling individual series.
 */
export interface NamedChartSeriesInput {
  id: string
  label: string
  kind: NamedChartSeriesKind
  paneId: string
  scaleId?: string
  unit?: string
  colorIndex?: number
  data: readonly NamedChartSeriesDataPoint[]
  referenceLines?: readonly NamedChartReferenceLineInput[]
  visible?: boolean
  /**
   * Keeps the series instantiated while it is hidden so visibility-only
   * changes can be applied without rebuilding the chart or its panes.
   */
  resident?: boolean
}

export type NamedChartSeriesFailureStage =
  | 'input'
  | 'series'
  | 'data'
  | 'price-scale'
  | 'reference-line'
  | 'pane-layout'
  | 'visibility'
  | 'cleanup'

export interface NamedChartSeriesFailure {
  seriesId: string
  stage: NamedChartSeriesFailureStage
  message: string
  referenceLineIndex?: number
}

export interface NamedChartReferenceLineRuntime {
  input: NamedChartReferenceLineInput
  api: any
}

export interface NamedChartSeriesRegistryEntry {
  input: NamedChartSeriesInput
  series: any
  paneIndex: number
  priceScaleId: string | null
  colorIndex: number
  referenceLines: NamedChartReferenceLineRuntime[]
}

export interface NamedChartSeriesRuntime {
  chart: any
  registry: Map<string, NamedChartSeriesRegistryEntry>
  createdCount: number
  failures: NamedChartSeriesFailure[]
  paneIndexById: ReadonlyMap<string, number>
  initialPaneCount: number
  theme: LightweightChartTheme
  disposed: boolean
  reapplyTheme: (theme: LightweightChartTheme) => void
  setVisibleSeriesIds: (visibleSeriesIds: ReadonlySet<string>) => boolean
  dispose: () => void
}

export function applyLightweightChartSize(
  chart: unknown,
  widthRaw: unknown,
  heightRaw: unknown,
): void {
  if (!chart || typeof (chart as { applyOptions?: unknown }).applyOptions !== 'function') return
  ;(chart as { applyOptions: (options: { width: number; height: number }) => void }).applyOptions(
    normalizeChartSize(widthRaw, heightRaw),
  )
}

function applySeriesOptions(series: unknown, options: Record<string, unknown>): void {
  if (!series || typeof (series as { applyOptions?: unknown }).applyOptions !== 'function') return
  try {
    ;(series as { applyOptions: (next: Record<string, unknown>) => void }).applyOptions(options)
  } catch {}
}

export function applyLightweightChartSeriesTheme(
  series: LightweightChartSeriesThemeTargets,
  theme: LightweightChartTheme,
): void {
  applySeriesOptions(series.candleSeries, candlestickSeriesOptions(theme))
  applySeriesOptions(series.fastSeries, lineSeriesOptions('fast', theme))
  applySeriesOptions(series.slowSeries, lineSeriesOptions('slow', theme))
  applySeriesOptions(series.halftrendSeries, histogramSeriesOptions('halftrend', theme))
  applySeriesOptions(series.ottSeries, lineSeriesOptions('ott', theme))
  applySeriesOptions(series.maczSeries, histogramSeriesOptions('macz', theme))
}

export function observeLightweightChartTheme(
  chart: unknown,
  options: LightweightChartThemeObserverOptions = {},
): () => void {
  const root = options.root === undefined
    ? (typeof document !== 'undefined' ? document.documentElement : null)
    : options.root
  if (
    !root ||
    typeof MutationObserver === 'undefined'
  ) {
    return () => undefined
  }

  let active = true
  let currentTheme = options.initialTheme ?? resolveLightweightChartTheme(root)
  const applyTheme = (theme: LightweightChartTheme): void => {
    if (!active) return
    if (chart && typeof (chart as { applyOptions?: unknown }).applyOptions === 'function') {
      try {
        ;(chart as { applyOptions: (next: Record<string, unknown>) => void })
          .applyOptions(lightweightChartThemeOptions(theme))
      } catch {}
    }
    applyLightweightChartSeriesTheme(options.series ?? {}, theme)
    try {
      options.onThemeChange?.(theme)
    } catch {}
  }
  let observer: MutationObserver
  try {
    observer = new MutationObserver(() => {
      if (!active) return
      const nextTheme = resolveLightweightChartTheme(root)
      if (nextTheme === currentTheme) return
      currentTheme = nextTheme
      applyTheme(nextTheme)
    })
    observer.observe(root, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })
  } catch {
    return () => undefined
  }
  const observedTheme = resolveLightweightChartTheme(root)
  if (observedTheme !== currentTheme) {
    currentTheme = observedTheme
    applyTheme(observedTheme)
  }
  return () => {
    if (!active) return
    active = false
    observer.disconnect()
  }
}

function removeChart(chart: any): void {
  if (chart && typeof chart.remove === 'function') chart.remove()
}

function addCandlestickSeries(
  chart: any,
  chartModule: LightweightChartModule,
  theme: LightweightChartTheme,
): any {
  const options = candlestickSeriesOptions(theme)
  if (typeof chart.addSeries === 'function' && chartModule.CandlestickSeries) {
    try {
      return chart.addSeries(chartModule.CandlestickSeries, options)
    } catch {}
  }
  if (typeof chart.addCandlestickSeries === 'function') {
    try {
      return chart.addCandlestickSeries(options)
    } catch {}
  }
  return null
}

export function createCandlestickRuntime(
  host: HTMLElement,
  chartModule: LightweightChartModule,
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): CandlestickRuntime | null {
  if (typeof chartModule.createChart !== 'function') return null
  let chart: any
  try {
    chart = chartModule.createChart(
      host,
      baseLightweightChartOptions(
        normalizeChartSize(host.clientWidth, host.clientHeight),
        theme,
      ),
    )
  } catch {
    return null
  }

  const candleSeries = addCandlestickSeries(chart, chartModule, theme)
  if (!candleSeries) {
    try {
      removeChart(chart)
    } catch {}
    return null
  }

  let markerApi: CandlestickRuntime['markerApi'] = null
  if (typeof chartModule.createSeriesMarkers === 'function') {
    try {
      const candidate = chartModule.createSeriesMarkers(candleSeries, [])
      markerApi =
        candidate && typeof candidate.setMarkers === 'function'
          ? candidate
          : null
    } catch {
      // Marker support is optional. Keep the usable candle runtime when the
      // plugin is unavailable or fails to initialize.
      markerApi = null
    }
  }

  return { chart, candleSeries, markerApi, theme }
}

function addLineSeries(
  chart: any,
  chartModule: LightweightChartModule,
  kind: 'fast' | 'slow' | 'ott',
  theme: LightweightChartTheme,
): any {
  const options = lineSeriesOptions(kind, theme)
  if (typeof chart.addSeries === 'function' && chartModule.LineSeries) {
    try {
      return chart.addSeries(chartModule.LineSeries, options)
    } catch {}
  }
  if (typeof chart.addLineSeries === 'function') {
    try {
      return chart.addLineSeries(options)
    } catch {}
  }
  return null
}

function applyHistogramPriceScale(series: any, kind: HistogramSeriesKind): void {
  if (!series || typeof series.priceScale !== 'function') return
  try {
    const priceScale = series.priceScale()
    if (priceScale && typeof priceScale.applyOptions === 'function') {
      priceScale.applyOptions(histogramPriceScaleOptions(kind))
    }
  } catch {}
}

function addHistogramSeries(
  chart: any,
  chartModule: LightweightChartModule,
  kind: HistogramSeriesKind,
  theme: LightweightChartTheme,
): any {
  const options = histogramSeriesOptions(kind, theme)
  let series: any = null
  if (typeof chart.addSeries === 'function' && chartModule.HistogramSeries) {
    try {
      series = chart.addSeries(chartModule.HistogramSeries, options)
    } catch {}
  }
  if (!series && typeof chart.addHistogramSeries === 'function') {
    try {
      series = chart.addHistogramSeries(options)
    } catch {}
  }
  applyHistogramPriceScale(series, kind)
  return series
}

export function createDrillSeriesRuntime(
  host: HTMLElement,
  chartModule: LightweightChartModule,
): DrillSeriesRuntime | null {
  const theme = resolveLightweightChartTheme()
  const base = createCandlestickRuntime(host, chartModule, theme)
  if (!base) return null
  const fastSeries = addLineSeries(base.chart, chartModule, 'fast', theme)
  const slowSeries = addLineSeries(base.chart, chartModule, 'slow', theme)
  const halftrendSeries = addHistogramSeries(base.chart, chartModule, 'halftrend', theme)
  const ottSeries = addLineSeries(base.chart, chartModule, 'ott', theme)
  const maczSeries = addHistogramSeries(base.chart, chartModule, 'macz', theme)

  return {
    ...base,
    fastSeries,
    slowSeries,
    halftrendSeries,
    ottSeries,
    maczSeries,
  }
}

const NAMED_SERIES_TONES: readonly LightweightChartSeriesTone[] = [
  'fast',
  'slow',
  'ott',
  'positive',
  'negative',
  'highlight',
  'neutral',
]

function normalizedColorIndex(value: unknown, fallback = 0): number {
  const candidate = Number(value)
  const integer = Number.isFinite(candidate) ? Math.trunc(candidate) : fallback
  return ((integer % NAMED_SERIES_TONES.length) + NAMED_SERIES_TONES.length)
    % NAMED_SERIES_TONES.length
}

function stableNamedSeriesColorIndex(id: string): number {
  let hash = 0
  for (let index = 0; index < id.length; index += 1) {
    hash = ((hash * 31) + id.charCodeAt(index)) | 0
  }
  return normalizedColorIndex(hash)
}

export function namedChartSeriesColor(
  theme: LightweightChartTheme,
  colorIndex: unknown,
): string {
  const tone = NAMED_SERIES_TONES[normalizedColorIndex(colorIndex)] ?? 'fast'
  return lightweightChartSeriesPalette(theme)[tone]
}

function normalizedUnit(unitRaw: unknown): string {
  return String(unitRaw ?? '').trim().toLowerCase()
}

export function namedChartSeriesPriceFormat(unitRaw: unknown): Record<string, unknown> {
  const unit = normalizedUnit(unitRaw)
  if (unit === '%' || unit.includes('percent')) {
    return { type: 'percent', precision: 2, minMove: 0.01 }
  }
  if (unit.includes('volume')) {
    return { type: 'volume' }
  }
  if (
    unit.includes('state') ||
    unit.includes('signal') ||
    unit.includes('count') ||
    unit.includes('binary') ||
    unit.includes('integer')
  ) {
    return { type: 'price', precision: 0, minMove: 1 }
  }
  if (unit.includes('degree') || unit.includes('angle')) {
    return {
      type: 'custom',
      minMove: 0.1,
      formatter: (value: number) => `${Number(value).toFixed(1)}°`,
    }
  }
  if (
    unit.includes('price') ||
    unit.includes('currency') ||
    unit === 'atr'
  ) {
    return { type: 'price', precision: 4, minMove: 0.0001 }
  }
  return { type: 'price', precision: 3, minMove: 0.001 }
}

function normalizeNamedPaneId(paneIdRaw: unknown): string {
  const paneId = String(paneIdRaw ?? '').trim().toLowerCase()
  return paneId || 'indicator'
}

function normalizeNamedScaleId(input: NamedChartSeriesInput): string {
  return (
    String(input.scaleId ?? '').trim().toLowerCase() ||
    normalizedUnit(input.unit) ||
    'default'
  )
}

function compareStableText(first: string, second: string): number {
  if (first < second) return -1
  if (first > second) return 1
  return 0
}

function buildNamedPaneIndex(
  inputs: readonly NamedChartSeriesInput[],
): ReadonlyMap<string, number> {
  const nonPricePaneIds = Array.from(
    new Set(
      inputs
        .map((input) => normalizeNamedPaneId(input.paneId))
        .filter((paneId) => paneId !== 'price'),
    ),
  ).sort(compareStableText)
  const paneIndexById = new Map<string, number>([['price', 0]])
  nonPricePaneIds.forEach((paneId, index) => {
    paneIndexById.set(paneId, index + 1)
  })
  return paneIndexById
}

function buildNamedScaleAssignments(
  inputs: readonly NamedChartSeriesInput[],
): ReadonlyMap<string, string | null> {
  const assignments = new Map<string, string | null>()
  const scalesByPane = new Map<string, Map<string, string>>()
  for (const input of inputs) {
    const id = String(input.id ?? '').trim()
    if (!id || assignments.has(id)) continue
    const paneId = normalizeNamedPaneId(input.paneId)
    if (paneId === 'price') {
      assignments.set(id, null)
      continue
    }
    const scaleId = normalizeNamedScaleId(input)
    let paneScales = scalesByPane.get(paneId)
    if (!paneScales) {
      paneScales = new Map<string, string>()
      scalesByPane.set(paneId, paneScales)
    }
    let mappedScaleId = paneScales.get(scaleId)
    if (!mappedScaleId) {
      mappedScaleId = paneScales.size === 0
        ? 'right'
        : `named:${paneId}:${scaleId}`
      paneScales.set(scaleId, mappedScaleId)
    }
    assignments.set(id, mappedScaleId)
  }
  return assignments
}

function namedLayoutInputs(
  inputs: readonly NamedChartSeriesInput[],
): NamedChartSeriesInput[] {
  const seenIds = new Set<string>()
  return inputs.filter((input) => {
    const id = String(input.id ?? '').trim()
    if (
      !id ||
      seenIds.has(id) ||
      (input.kind !== 'line' && input.kind !== 'histogram') ||
      !Array.isArray(input.data)
    ) {
      return false
    }
    seenIds.add(id)
    return true
  })
}

function currentChartPaneCount(chart: any): number {
  if (typeof chart?.panes !== 'function') return 1
  try {
    const panes = chart.panes()
    return Array.isArray(panes) ? Math.max(1, panes.length) : 1
  } catch {
    return 1
  }
}

function namedSeriesOptions(
  input: NamedChartSeriesInput,
  theme: LightweightChartTheme,
  colorIndex: number,
  priceScaleId: string | null,
  includeScaleAssignment: boolean,
): Record<string, unknown> {
  const options: Record<string, unknown> = {
    title: String(input.label ?? '').trim(),
    color: namedChartSeriesColor(theme, colorIndex),
    priceFormat: namedChartSeriesPriceFormat(input.unit),
    priceLineVisible: false,
    lastValueVisible: false,
    visible: input.visible !== false,
  }
  if (includeScaleAssignment && priceScaleId) {
    options.priceScaleId = priceScaleId
  }
  if (input.kind === 'histogram') {
    options.base = 0
  } else {
    options.lineWidth = 2
    options.crosshairMarkerVisible = false
  }
  return options
}

function namedSeriesData(
  input: NamedChartSeriesInput,
  theme: LightweightChartTheme,
): Array<Record<string, unknown>> {
  const palette = lightweightChartSeriesPalette(theme)
  return input.data.map((point) => {
    const value = point.value
    if (value === null || value === undefined || !Number.isFinite(Number(value))) {
      return { time: point.time }
    }
    const normalizedPoint: Record<string, unknown> = {
      time: point.time,
      value: Number(value),
    }
    if (input.kind === 'histogram') {
      normalizedPoint.color = Number(value) >= 0 ? palette.positive : palette.negative
    } else if (point.color) {
      normalizedPoint.color = point.color
    }
    return normalizedPoint
  })
}

function failureMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim()) return error.message
  const message = String(error ?? '').trim()
  return message || fallback
}

function pushNamedFailure(
  failures: NamedChartSeriesFailure[],
  seriesId: string,
  stage: NamedChartSeriesFailureStage,
  error: unknown,
  fallback: string,
  referenceLineIndex?: number,
): void {
  failures.push({
    seriesId,
    stage,
    message: failureMessage(error, fallback),
    ...(referenceLineIndex === undefined ? {} : { referenceLineIndex }),
  })
}

function addNamedSeries(
  chart: any,
  chartModule: LightweightChartModule,
  input: NamedChartSeriesInput,
  options: Record<string, unknown>,
  paneIndex: number,
): any {
  if (typeof chart?.addSeries !== 'function') {
    throw new Error('Lightweight Charts v5 addSeries API is unavailable')
  }
  const definition = input.kind === 'histogram'
    ? chartModule.HistogramSeries
    : chartModule.LineSeries
  if (!definition) {
    throw new Error(`${input.kind} series definition is unavailable`)
  }
  return chart.addSeries(definition, options, paneIndex)
}

function applyNamedPriceScaleOptions(
  chart: any,
  entry: NamedChartSeriesRegistryEntry,
  failures: NamedChartSeriesFailure[],
): void {
  if (!entry.priceScaleId) return
  const scaleOptions = entry.priceScaleId === 'right'
    ? {
        visible: true,
        borderVisible: true,
        autoScale: true,
        scaleMargins: { top: 0.12, bottom: 0.12 },
      }
    : {
        visible: false,
        borderVisible: false,
        autoScale: true,
        scaleMargins: { top: 0.12, bottom: 0.12 },
      }
  let priceScale: any = null
  try {
    if (typeof chart?.priceScale === 'function') {
      priceScale = chart.priceScale(entry.priceScaleId, entry.paneIndex)
    } else if (typeof entry.series?.priceScale === 'function') {
      priceScale = entry.series.priceScale()
    }
    if (priceScale && typeof priceScale.applyOptions === 'function') {
      priceScale.applyOptions(scaleOptions)
    }
  } catch (error) {
    pushNamedFailure(
      failures,
      entry.input.id,
      'price-scale',
      error,
      'Price-scale options could not be applied',
    )
  }
}

function normalizedReferenceLineWidth(value: unknown): number {
  const width = Math.trunc(Number(value))
  if (!Number.isFinite(width)) return 1
  return Math.max(1, Math.min(4, width))
}

function normalizedReferenceLineStyle(value: unknown): number {
  const style = Math.trunc(Number(value))
  if (!Number.isFinite(style)) return 2
  return Math.max(0, Math.min(4, style))
}

function referenceLineOptions(
  input: NamedChartReferenceLineInput,
  theme: LightweightChartTheme,
  fallbackColorIndex: number,
  seriesVisible = true,
): Record<string, unknown> {
  const label = String(input.label ?? '').trim()
  return {
    price: Number(input.value),
    color: namedChartSeriesColor(
      theme,
      input.colorIndex === undefined ? fallbackColorIndex : input.colorIndex,
    ),
    lineWidth: normalizedReferenceLineWidth(input.lineWidth),
    lineStyle: normalizedReferenceLineStyle(input.lineStyle),
    lineVisible: seriesVisible,
    axisLabelVisible: seriesVisible && (input.axisLabelVisible ?? true),
    title: label,
  }
}

function createNamedReferenceLines(
  entry: NamedChartSeriesRegistryEntry,
  theme: LightweightChartTheme,
  failures: NamedChartSeriesFailure[],
): void {
  const inputs = entry.input.referenceLines ?? []
  inputs.forEach((input, referenceLineIndex) => {
    if (!Number.isFinite(Number(input.value))) {
      pushNamedFailure(
        failures,
        entry.input.id,
        'reference-line',
        new Error('Reference-line value must be finite'),
        'Reference-line value must be finite',
        referenceLineIndex,
      )
      return
    }
    if (typeof entry.series?.createPriceLine !== 'function') {
      pushNamedFailure(
        failures,
        entry.input.id,
        'reference-line',
        new Error('Reference-line API is unavailable'),
        'Reference-line API is unavailable',
        referenceLineIndex,
      )
      return
    }
    try {
      const api = entry.series.createPriceLine(
        referenceLineOptions(
          input,
          theme,
          entry.colorIndex,
          entry.input.visible !== false,
        ),
      )
      entry.referenceLines.push({ input, api })
    } catch (error) {
      pushNamedFailure(
        failures,
        entry.input.id,
        'reference-line',
        error,
        'Reference line could not be created',
        referenceLineIndex,
      )
    }
  })
}

function applyNamedPaneLayout(
  chart: any,
  paneIndexById: ReadonlyMap<string, number>,
  failures: NamedChartSeriesFailure[],
): void {
  if (typeof chart?.panes !== 'function') return
  let panes: any[]
  try {
    panes = chart.panes()
  } catch (error) {
    pushNamedFailure(
      failures,
      '__layout__',
      'pane-layout',
      error,
      'Chart panes could not be inspected',
    )
    return
  }
  const expectedIndices = Array.from(new Set(paneIndexById.values()))
  for (const paneIndex of expectedIndices) {
    const pane = panes[paneIndex]
    if (!pane || typeof pane.setStretchFactor !== 'function') continue
    try {
      pane.setStretchFactor(paneIndex === 0 ? 3 : 1)
    } catch (error) {
      pushNamedFailure(
        failures,
        '__layout__',
        'pane-layout',
        error,
        `Pane ${paneIndex} stretch factor could not be applied`,
      )
    }
  }
}

function reapplyNamedSeriesTheme(
  runtime: NamedChartSeriesRuntime,
  theme: LightweightChartTheme,
): void {
  if (runtime.disposed) return
  runtime.theme = theme
  for (const entry of runtime.registry.values()) {
    try {
      if (typeof entry.series?.applyOptions === 'function') {
        entry.series.applyOptions(
          namedSeriesOptions(
            entry.input,
            theme,
            entry.colorIndex,
            entry.priceScaleId,
            false,
          ),
        )
      }
    } catch (error) {
      pushNamedFailure(
        runtime.failures,
        entry.input.id,
        'series',
        error,
        'Series theme could not be applied',
      )
    }
    if (entry.input.kind === 'histogram') {
      try {
        if (typeof entry.series?.setData === 'function') {
          entry.series.setData(namedSeriesData(entry.input, theme))
        }
      } catch (error) {
        pushNamedFailure(
          runtime.failures,
          entry.input.id,
          'data',
          error,
          'Histogram data could not be recolored',
        )
      }
    }
    for (const referenceLine of entry.referenceLines) {
      if (typeof referenceLine.api?.applyOptions !== 'function') continue
      try {
        referenceLine.api.applyOptions(
          referenceLineOptions(
            referenceLine.input,
            theme,
            entry.colorIndex,
            entry.input.visible !== false,
          ),
        )
      } catch (error) {
        pushNamedFailure(
          runtime.failures,
          entry.input.id,
          'reference-line',
          error,
          'Reference-line theme could not be applied',
        )
      }
    }
  }
}

function setNamedSeriesVisibility(
  runtime: NamedChartSeriesRuntime,
  visibleSeriesIds: ReadonlySet<string>,
): boolean {
  if (runtime.disposed) return false
  let succeeded = true
  for (const entry of runtime.registry.values()) {
    const visible = visibleSeriesIds.has(entry.input.id)
    if (entry.input.visible === visible) continue
    try {
      if (typeof entry.series?.applyOptions !== 'function') {
        throw new Error('Series visibility API is unavailable')
      }
      entry.series.applyOptions({ visible })
    } catch (error) {
      succeeded = false
      pushNamedFailure(
        runtime.failures,
        entry.input.id,
        'visibility',
        error,
        'Series visibility could not be updated',
      )
      continue
    }
    entry.input = { ...entry.input, visible }
    for (const referenceLine of entry.referenceLines) {
      if (typeof referenceLine.api?.applyOptions !== 'function') continue
      try {
        referenceLine.api.applyOptions({
          lineVisible: visible,
          axisLabelVisible: visible && (referenceLine.input.axisLabelVisible ?? true),
        })
      } catch (error) {
        succeeded = false
        pushNamedFailure(
          runtime.failures,
          entry.input.id,
          'visibility',
          error,
          'Reference-line visibility could not be updated',
        )
      }
    }
  }
  return succeeded
}

function disposeNamedSeriesRuntime(runtime: NamedChartSeriesRuntime): void {
  if (runtime.disposed) return
  runtime.disposed = true
  const entries = Array.from(runtime.registry.values()).reverse()
  for (const entry of entries) {
    for (const referenceLine of entry.referenceLines) {
      if (typeof entry.series?.removePriceLine !== 'function') continue
      try {
        entry.series.removePriceLine(referenceLine.api)
      } catch (error) {
        pushNamedFailure(
          runtime.failures,
          entry.input.id,
          'cleanup',
          error,
          'Reference line could not be removed',
        )
      }
    }
    if (typeof runtime.chart?.removeSeries !== 'function') continue
    try {
      runtime.chart.removeSeries(entry.series)
    } catch (error) {
      pushNamedFailure(
        runtime.failures,
        entry.input.id,
        'cleanup',
        error,
        'Series could not be removed',
      )
    }
  }
  runtime.registry.clear()

  if (typeof runtime.chart?.panes !== 'function' || typeof runtime.chart?.removePane !== 'function') {
    return
  }
  try {
    const panes = runtime.chart.panes()
    for (
      let paneIndex = panes.length - 1;
      paneIndex >= runtime.initialPaneCount;
      paneIndex -= 1
    ) {
      const pane = panes[paneIndex]
      if (typeof pane?.getSeries === 'function' && pane.getSeries().length > 0) continue
      runtime.chart.removePane(paneIndex)
    }
  } catch (error) {
    pushNamedFailure(
      runtime.failures,
      '__cleanup__',
      'cleanup',
      error,
      'Empty named-series panes could not be removed',
    )
  }
}

/**
 * Creates strategy-named series on an existing Lightweight Charts v5 chart.
 *
 * Pane IDs are normalized and sorted so non-price pane indices remain stable
 * for a given visible set. The `price` pane is always pane 0 and deliberately
 * omits `priceScaleId`, sharing the candlestick series' default right scale.
 */
export function createNamedSeriesRuntime(
  chart: any,
  chartModule: LightweightChartModule,
  rawInputs: readonly NamedChartSeriesInput[],
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): NamedChartSeriesRuntime {
  const inputs = rawInputs.filter(
    (input) => input.visible !== false || input.resident === true,
  )
  const failures: NamedChartSeriesFailure[] = []
  const registry = new Map<string, NamedChartSeriesRegistryEntry>()
  const layoutInputs = namedLayoutInputs(inputs)
  const paneIndexById = buildNamedPaneIndex(layoutInputs)
  const scaleAssignments = buildNamedScaleAssignments(layoutInputs)
  const initialPaneCount = currentChartPaneCount(chart)
  const runtime: NamedChartSeriesRuntime = {
    chart,
    registry,
    createdCount: 0,
    failures,
    paneIndexById,
    initialPaneCount,
    theme,
    disposed: false,
    reapplyTheme: (nextTheme) => reapplyNamedSeriesTheme(runtime, nextTheme),
    setVisibleSeriesIds: (visibleSeriesIds) =>
      setNamedSeriesVisibility(runtime, visibleSeriesIds),
    dispose: () => disposeNamedSeriesRuntime(runtime),
  }

  inputs.forEach((input) => {
    const id = String(input.id ?? '').trim()
    if (!id) {
      pushNamedFailure(
        failures,
        '',
        'input',
        new Error('Named series id is required'),
        'Named series id is required',
      )
      return
    }
    if (registry.has(id)) {
      pushNamedFailure(
        failures,
        id,
        'input',
        new Error(`Duplicate named series id: ${id}`),
        `Duplicate named series id: ${id}`,
      )
      return
    }
    if (input.kind !== 'line' && input.kind !== 'histogram') {
      pushNamedFailure(
        failures,
        id,
        'input',
        new Error(`Unsupported named series kind: ${String(input.kind)}`),
        'Unsupported named series kind',
      )
      return
    }
    if (!Array.isArray(input.data)) {
      pushNamedFailure(
        failures,
        id,
        'input',
        new Error('Named series data must be an array'),
        'Named series data must be an array',
      )
      return
    }

    const paneId = normalizeNamedPaneId(input.paneId)
    const paneIndex = paneIndexById.get(paneId) ?? 0
    const priceScaleId = scaleAssignments.get(id) ?? null
    const colorIndex = normalizedColorIndex(
      input.colorIndex,
      stableNamedSeriesColorIndex(id),
    )
    let series: any
    try {
      series = addNamedSeries(
        chart,
        chartModule,
        input,
        namedSeriesOptions(input, theme, colorIndex, priceScaleId, true),
        paneIndex,
      )
      if (!series) throw new Error('Lightweight Charts returned no series')
    } catch (error) {
      pushNamedFailure(
        failures,
        id,
        'series',
        error,
        'Named series could not be created',
      )
      return
    }

    const entry: NamedChartSeriesRegistryEntry = {
      input,
      series,
      paneIndex,
      priceScaleId,
      colorIndex,
      referenceLines: [],
    }
    registry.set(id, entry)
    try {
      if (typeof series.setData !== 'function') {
        throw new Error('Series data API is unavailable')
      }
      series.setData(namedSeriesData(input, theme))
    } catch (error) {
      pushNamedFailure(
        failures,
        id,
        'data',
        error,
        'Named series data could not be set',
      )
    }
    applyNamedPriceScaleOptions(chart, entry, failures)
    createNamedReferenceLines(entry, theme, failures)
  })

  runtime.createdCount = registry.size
  applyNamedPaneLayout(chart, paneIndexById, failures)
  return runtime
}

function chartTimeScale(chart: unknown): any | null {
  if (!chart || typeof (chart as { timeScale?: unknown }).timeScale !== 'function') return null
  try {
    return (chart as { timeScale: () => unknown }).timeScale()
  } catch {
    return null
  }
}

export function fitLightweightChartContent(chart: unknown): boolean {
  const timeScale = chartTimeScale(chart)
  if (!timeScale || typeof timeScale.fitContent !== 'function') return false
  try {
    timeScale.fitContent()
    return true
  } catch {
    return false
  }
}

export function visibleLogicalRangeLabel(chart: unknown): string {
  const timeScale = chartTimeScale(chart)
  if (!timeScale || typeof timeScale.getVisibleLogicalRange !== 'function') return ''
  try {
    const range = timeScale.getVisibleLogicalRange()
    if (!range || !Number.isFinite(range.from) || !Number.isFinite(range.to)) return ''
    return `${Number(range.from).toFixed(1)}..${Number(range.to).toFixed(1)}`
  } catch {
    return ''
  }
}

export function panLightweightChart(
  chart: unknown,
  direction: 'left' | 'right',
  fraction = 0.55,
): boolean {
  const timeScale = chartTimeScale(chart)
  if (!timeScale || typeof timeScale.getVisibleLogicalRange !== 'function' || typeof timeScale.setVisibleLogicalRange !== 'function') {
    return false
  }
  try {
    const range = timeScale.getVisibleLogicalRange()
    if (!range || !Number.isFinite(range.from) || !Number.isFinite(range.to)) return false
    const width = Math.max(1, Number(range.to) - Number(range.from))
    const delta = width * Math.max(0.1, Math.min(1, fraction)) * (direction === 'left' ? -1 : 1)
    timeScale.setVisibleLogicalRange({ from: Number(range.from) + delta, to: Number(range.to) + delta })
    return true
  } catch {
    return false
  }
}
