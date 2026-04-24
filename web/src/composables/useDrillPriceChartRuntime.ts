import { markRaw } from 'vue'
import { useLightweightChart } from './useLightweightChart'
import {
  applyLightweightChartSize,
  createDrillSeriesRuntime,
  type LightweightChartModule,
} from '../visualizations/lightweightChartRuntime'
import {
  buildDrillTradeMarkers,
  type DrillChartModel,
  type DrillCandlePoint,
  type DrillHistogramPoint,
  type DrillLinePoint,
} from '../visualizations/drillChartModel'

interface DrillChartContext {
  chart: any
  candle: any
  fast: any
  slow: any
  halftrend: any
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
  halftrendData: DrillLinePoint[]
  maczData: DrillHistogramPoint[]
}

export interface DrillPriceChartRuntime {
  setHost: (host: HTMLElement | null) => void
  render: (model: DrillChartModel | null, selectedTradeIdx: number) => Promise<void>
  updateSelectedTrade: (model: DrillChartModel | null, selectedTradeIdx: number) => void
  clear: (message?: string) => void
  dispose: () => void
}

export interface DrillPriceChartRuntimeOptions {
  onTradeMarkerSelect?: (tradeIdx: number) => void
  onMetaChange?: (message: string) => void
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function markRawEntity<T>(value: T): T {
  if (value && typeof value === 'object') return markRaw(value as object) as T
  return value
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

function clearSeries(series: any): void {
  setSeriesData(series, [])
}

export function useDrillPriceChartRuntime(
  options: DrillPriceChartRuntimeOptions = {},
): DrillPriceChartRuntime {
  const chartRuntime = useLightweightChart<any>()
  let modulePromise: Promise<LightweightChartModule | null> | null = null
  let candleSeries: any = null
  let fastSeries: any = null
  let slowSeries: any = null
  let halftrendSeries: any = null
  let maczSeries: any = null
  let markerApi: any = null
  let clickHandler: ((param: unknown) => void) | null = null
  let renderFrame = 0
  let pendingPayload: DrillChartRenderPayload | null = null
  let renderToken = 0
  let currentModel: DrillChartModel | null = null
  let currentSelectedTradeIdx = -1

  function setMeta(message: string): void {
    options.onMetaChange?.(message)
  }

  async function loadModule(): Promise<LightweightChartModule | null> {
    if (!modulePromise) {
      modulePromise = import('lightweight-charts')
        .then((module) => module as LightweightChartModule)
        .catch(() => null)
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

  function dispose(): void {
    cancelRenderFrame()
    removeClickHandler()
    chartRuntime.dispose((chart) => {
      if (chart && typeof (chart as { remove?: unknown }).remove === 'function') {
        ;(chart as { remove: () => void }).remove()
      }
    })
    candleSeries = null
    fastSeries = null
    slowSeries = null
    halftrendSeries = null
    maczSeries = null
    markerApi = null
    currentModel = null
    currentSelectedTradeIdx = -1
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
  }

  function updateSelectedTrade(
    model: DrillChartModel | null = currentModel,
    selectedTradeIdx = currentSelectedTradeIdx,
  ): void {
    currentModel = model
    currentSelectedTradeIdx = selectedTradeIdx
    if (!markerApi || typeof markerApi.setMarkers !== 'function' || !model) return
    try {
      markerApi.setMarkers(buildDrillTradeMarkers(model, selectedTradeIdx))
    } catch {}
  }

  function flushRenderFrame(): void {
    renderFrame = 0
    const payload = pendingPayload
    pendingPayload = null
    if (!payload || payload.token !== renderToken) return
    const { ctx, model, selectedTradeIdx } = payload
    if (!setSeriesData(ctx.candle, payload.candleData)) {
      clear('Unable to render drill chart.')
      return
    }
    setSeriesData(ctx.fast, payload.fastData)
    setSeriesData(ctx.slow, payload.slowData)
    setSeriesData(ctx.halftrend, payload.halftrendData)
    setSeriesData(ctx.macz, payload.maczData)
    updateSelectedTrade(model, selectedTradeIdx)

    try {
      const timeScale = ctx.chart.timeScale && ctx.chart.timeScale()
      if (timeScale && typeof timeScale.fitContent === 'function') timeScale.fitContent()
    } catch {}

    setMeta(
      `candles=${payload.candleData.length} fast=${payload.fastData.length} slow=${payload.slowData.length} trades=${model.trades.length} markers=${buildDrillTradeMarkers(model, selectedTradeIdx).length}`,
    )
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

  async function ensureChart(): Promise<DrillChartContext | null> {
    const host = chartRuntime.hostRef.value
    if (!host) return null
    if (chartRuntime.chartRef.value && candleSeries) {
      return {
        chart: chartRuntime.chartRef.value,
        candle: candleSeries,
        fast: fastSeries,
        slow: slowSeries,
        halftrend: halftrendSeries,
        macz: maczSeries,
        markerApi,
      }
    }

    const chartModule = await loadModule()
    if (!chartModule || typeof chartModule.createChart !== 'function') return null
    const runtime = createDrillSeriesRuntime(host, chartModule)
    if (!runtime) return null
    const chartAny = runtime.chart

    candleSeries = markRawEntity(runtime.candleSeries)
    fastSeries = markRawEntity(runtime.fastSeries)
    slowSeries = markRawEntity(runtime.slowSeries)
    halftrendSeries = markRawEntity(runtime.halftrendSeries)
    maczSeries = markRawEntity(runtime.maczSeries)
    markerApi = markRawEntity(runtime.markerApi)
    chartRuntime.setChart(markRawEntity(chartAny))

    if (typeof chartAny.subscribeClick === 'function') {
      clickHandler = (param: unknown) => {
        const row = param && typeof param === 'object' ? (param as Record<string, unknown>) : {}
        const markerId = text(row.hoveredObjectId ?? row.hoveredMarkerId)
        const match = /^trade-(\d+)(?:-(?:entry|exit))?$/u.exec(markerId)
        if (!match) return
        options.onTradeMarkerSelect?.(Number(match[1]))
      }
      try {
        chartAny.subscribeClick(clickHandler)
      } catch {}
    }

    chartRuntime.observeHostResize((rect) => {
      const chartRef = chartRuntime.chartRef.value
      if (!chartRef || !rect.width || !rect.height) return
      try {
        applyLightweightChartSize(chartRef, rect.width, rect.height)
      } catch {}
    })

    return {
      chart: chartRuntime.chartRef.value,
      candle: candleSeries,
      fast: fastSeries,
      slow: slowSeries,
      halftrend: halftrendSeries,
      macz: maczSeries,
      markerApi,
    }
  }

  function clear(
    message = 'Hover for OHLC/MA values; click a marker to focus a trade.',
  ): void {
    renderToken += 1
    cancelRenderFrame()
    setMeta(message)
    clearSeries(candleSeries)
    clearSeries(fastSeries)
    clearSeries(slowSeries)
    clearSeries(halftrendSeries)
    clearSeries(maczSeries)
    if (markerApi && typeof markerApi.setMarkers === 'function') {
      try {
        markerApi.setMarkers([])
      } catch {}
    }
    currentModel = null
    currentSelectedTradeIdx = -1
  }

  async function render(
    model: DrillChartModel | null,
    selectedTradeIdx: number,
  ): Promise<void> {
    const token = renderToken + 1
    renderToken = token
    currentModel = model
    currentSelectedTradeIdx = selectedTradeIdx
    if (!model) {
      clear()
      return
    }
    if (!model.bars.length) {
      clear('No valid OHLC bars available for chart rendering.')
      return
    }
    if (!model.candleData.length) {
      clear('No candle points generated from loaded bars.')
      return
    }
    const ctx = await ensureChart()
    if (!ctx || token !== renderToken) return
    scheduleRenderFrame({
      token,
      ctx,
      model,
      selectedTradeIdx,
      candleData: model.candleData,
      fastData: model.fastData,
      slowData: model.slowData,
      halftrendData: model.halftrendData,
      maczData: model.maczData,
    })
  }

  return {
    setHost,
    render,
    updateSelectedTrade,
    clear,
    dispose,
  }
}
