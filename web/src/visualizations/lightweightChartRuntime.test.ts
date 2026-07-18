
import { describe, expect, it, vi } from 'vitest'
import {
  applyLightweightChartSize,
  createCandlestickRuntime,
  createDrillSeriesRuntime,
  fitLightweightChartContent,
  panLightweightChart,
  visibleLogicalRangeLabel,
  type LightweightChartModule,
} from './lightweightChartRuntime'

function host(width = 640, height = 320): HTMLElement {
  const el = document.createElement('div')
  Object.defineProperty(el, 'clientWidth', { value: width })
  Object.defineProperty(el, 'clientHeight', { value: height })
  return el
}

describe('lightweightChartRuntime', () => {
  it('creates candlestick runtimes through the v5 series API', () => {
    const addSeries = vi.fn(() => ({ setData: vi.fn() }))
    const createSeriesMarkers = vi.fn(() => ({ setMarkers: vi.fn() }))
    const chart = { addSeries, remove: vi.fn() }
    const module: LightweightChartModule = {
      CandlestickSeries: { type: 'candle' },
      createChart: vi.fn(() => chart),
      createSeriesMarkers,
    }

    const runtime = createCandlestickRuntime(host(), module)

    expect(runtime?.chart).toBe(chart)
    expect(addSeries).toHaveBeenCalledWith(module.CandlestickSeries, expect.objectContaining({
      borderVisible: true,
    }))
    expect(createSeriesMarkers).toHaveBeenCalledWith(runtime?.candleSeries, [])
  })

  it('falls back to legacy candlestick and line APIs when addSeries is unavailable', () => {
    const candle = { setData: vi.fn() }
    const chart = {
      addCandlestickSeries: vi.fn(() => candle),
      addLineSeries: vi.fn(() => ({ setData: vi.fn() })),
      remove: vi.fn(),
    }
    const module: LightweightChartModule = {
      createChart: vi.fn(() => chart),
    }

    const runtime = createDrillSeriesRuntime(host(), module)

    expect(runtime?.candleSeries).toBe(candle)
    expect(chart.addLineSeries).toHaveBeenCalledTimes(3)
    expect(runtime?.halftrendSeries).toBeNull()
    expect(runtime?.maczSeries).toBeNull()
  })

  it('creates HalfTrend and MAC-Z on independent histogram price scales', () => {
    const applyOptions = vi.fn()
    const makeSeries = () => ({ setData: vi.fn(), priceScale: () => ({ applyOptions }) })
    const addSeries = vi.fn(() => makeSeries())
    const chart = { addSeries, remove: vi.fn() }
    const module: LightweightChartModule = {
      CandlestickSeries: { type: 'candle' },
      HistogramSeries: { type: 'histogram' },
      LineSeries: { type: 'line' },
      createChart: vi.fn(() => chart),
    }

    const runtime = createDrillSeriesRuntime(host(), module)

    expect(runtime?.halftrendSeries).toBeTruthy()
    expect(runtime?.maczSeries).toBeTruthy()
    expect(addSeries).toHaveBeenCalledWith(module.HistogramSeries, expect.objectContaining({
      priceScaleId: 'halftrend',
    }))
    expect(addSeries).toHaveBeenCalledWith(module.HistogramSeries, expect.objectContaining({
      priceScaleId: 'macz',
    }))
    expect(applyOptions).toHaveBeenCalledWith(expect.objectContaining({
      scaleMargins: { top: 0.18, bottom: 0.70 },
    }))
    expect(applyOptions).toHaveBeenCalledWith(expect.objectContaining({
      scaleMargins: { top: 0.78, bottom: 0.03 },
    }))
  })

  it('removes the chart when no candle series can be created', () => {
    const chart = { remove: vi.fn() }
    const module: LightweightChartModule = {
      createChart: vi.fn(() => chart),
    }

    expect(createCandlestickRuntime(host(), module)).toBeNull()
    expect(chart.remove).toHaveBeenCalledTimes(1)
  })

  it('normalizes resize options before applying them', () => {
    const applyOptions = vi.fn()

    applyLightweightChartSize({ applyOptions }, 10, 20)

    expect(applyOptions).toHaveBeenCalledWith({ width: 240, height: 220 })
  })

  it('fits, pans, and labels visible logical ranges defensively', () => {
    const range = { from: 10, to: 50 }
    const fitContent = vi.fn()
    const setVisibleLogicalRange = vi.fn((nextRange) => {
      range.from = nextRange.from
      range.to = nextRange.to
    })
    const chart = {
      timeScale: () => ({
        fitContent,
        getVisibleLogicalRange: () => ({ ...range }),
        setVisibleLogicalRange,
      }),
    }

    expect(visibleLogicalRangeLabel(chart)).toBe('10.0..50.0')
    expect(panLightweightChart(chart, 'left')).toBe(true)
    expect(setVisibleLogicalRange).toHaveBeenCalled()
    expect(range.from).toBeLessThan(10)
    expect(fitLightweightChartContent(chart)).toBe(true)
    expect(fitContent).toHaveBeenCalled()
  })
})
