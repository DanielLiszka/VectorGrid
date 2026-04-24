import {
  baseLightweightChartOptions,
  candlestickSeriesOptions,
  histogramSeriesOptions,
  lineSeriesOptions,
  normalizeChartSize,
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

function addLineSeries(chart: any, chartModule: LightweightChartModule, kind: 'fast' | 'slow' | 'halftrend'): any {
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

function addHistogramSeries(chart: any, chartModule: LightweightChartModule): any {
  const options = histogramSeriesOptions()
  if (typeof chart.addSeries === 'function' && chartModule.HistogramSeries) {
    try {
      return chart.addSeries(chartModule.HistogramSeries, options)
    } catch {}
  }
  return null
}

export function createDrillSeriesRuntime(
  host: HTMLElement,
  chartModule: LightweightChartModule,
): DrillSeriesRuntime | null {
  const base = createCandlestickRuntime(host, chartModule)
  if (!base) return null
  const fastSeries = addLineSeries(base.chart, chartModule, 'fast')
  const slowSeries = addLineSeries(base.chart, chartModule, 'slow')
  const halftrendSeries = addLineSeries(base.chart, chartModule, 'halftrend')
  const maczSeries = addHistogramSeries(base.chart, chartModule)

  return {
    ...base,
    fastSeries,
    slowSeries,
    halftrendSeries,
    maczSeries,
  }
}
