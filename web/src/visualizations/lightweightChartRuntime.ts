import {
  baseLightweightChartOptions,
  candlestickSeriesOptions,
  histogramPriceScaleOptions,
  histogramSeriesOptions,
  lineSeriesOptions,
  normalizeChartSize,
  type HistogramSeriesKind,
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
}

export interface DrillSeriesRuntime extends CandlestickRuntime {
  fastSeries: any
  slowSeries: any
  halftrendSeries: any
  ottSeries: any
  maczSeries: any
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

function removeChart(chart: any): void {
  if (chart && typeof chart.remove === 'function') chart.remove()
}

function addCandlestickSeries(chart: any, chartModule: LightweightChartModule): any {
  const options = candlestickSeriesOptions()
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
): CandlestickRuntime | null {
  if (typeof chartModule.createChart !== 'function') return null
  let chart: any
  try {
    chart = chartModule.createChart(
      host,
      baseLightweightChartOptions(normalizeChartSize(host.clientWidth, host.clientHeight)),
    )
  } catch {
    return null
  }

  const candleSeries = addCandlestickSeries(chart, chartModule)
  if (!candleSeries) {
    try {
      removeChart(chart)
    } catch {}
    return null
  }

  const markerApi =
    typeof chartModule.createSeriesMarkers === 'function'
      ? chartModule.createSeriesMarkers(candleSeries, [])
      : null

  return { chart, candleSeries, markerApi }
}

function addLineSeries(chart: any, chartModule: LightweightChartModule, kind: 'fast' | 'slow' | 'ott'): any {
  const options = lineSeriesOptions(kind)
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

function addHistogramSeries(chart: any, chartModule: LightweightChartModule, kind: HistogramSeriesKind): any {
  const options = histogramSeriesOptions(kind)
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
  const base = createCandlestickRuntime(host, chartModule)
  if (!base) return null
  const fastSeries = addLineSeries(base.chart, chartModule, 'fast')
  const slowSeries = addLineSeries(base.chart, chartModule, 'slow')
  const halftrendSeries = addHistogramSeries(base.chart, chartModule, 'halftrend')
  const ottSeries = addLineSeries(base.chart, chartModule, 'ott')
  const maczSeries = addHistogramSeries(base.chart, chartModule, 'macz')

  return {
    ...base,
    fastSeries,
    slowSeries,
    halftrendSeries,
    ottSeries,
    maczSeries,
  }
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
