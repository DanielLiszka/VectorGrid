import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const resizeObserverState = vi.hoisted(() => ({
  callback: null as null | ((entries: Array<{ contentRect: DOMRectReadOnly }>) => void),
  stop: vi.fn(),
}))

const drillChartState = vi.hoisted(() => ({
  createChart: vi.fn(),
  createSeriesMarkers: vi.fn(),
  crosshairHandler: null as null | ((param: unknown) => void),
  charts: [] as Array<{
    addSeries: ReturnType<typeof vi.fn>
    applyOptions: ReturnType<typeof vi.fn>
    remove: ReturnType<typeof vi.fn>
    removeSeries: ReturnType<typeof vi.fn>
    removePane: ReturnType<typeof vi.fn>
    panes: () => Array<{
      getSeries: () => unknown[]
      setStretchFactor: ReturnType<typeof vi.fn>
    }>
    series: Array<{
      applyOptions: ReturnType<typeof vi.fn>
      setData: ReturnType<typeof vi.fn>
      priceScale: () => { applyOptions: ReturnType<typeof vi.fn> }
      createPriceLine: ReturnType<typeof vi.fn>
      removePriceLine: ReturnType<typeof vi.fn>
      referenceLines: Array<{ applyOptions: ReturnType<typeof vi.fn> }>
      paneIndex: number
      removed: boolean
    }>
    timeScaleApi: {
      fitContent: ReturnType<typeof vi.fn>
      getVisibleLogicalRange: ReturnType<typeof vi.fn>
      setVisibleLogicalRange: ReturnType<typeof vi.fn>
      subscribeVisibleLogicalRangeChange: ReturnType<typeof vi.fn>
      unsubscribeVisibleLogicalRangeChange: ReturnType<typeof vi.fn>
    }
    setCrosshairPosition: ReturnType<typeof vi.fn>
    clearCrosshairPosition: ReturnType<typeof vi.fn>
    unsubscribeCrosshairMove: ReturnType<typeof vi.fn>
  }>,
}))

vi.mock('@vueuse/core', () => ({
  useResizeObserver: (_target: unknown, callback: (entries: Array<{ contentRect: DOMRectReadOnly }>) => void) => {
    resizeObserverState.callback = callback
    resizeObserverState.stop = vi.fn()
    return {
      stop: resizeObserverState.stop,
    }
  },
}))

vi.mock('lightweight-charts', () => ({
  CandlestickSeries: { type: 'candle' },
  LineSeries: { type: 'line' },
  HistogramSeries: { type: 'histogram' },
  createChart: (...args: unknown[]) => drillChartState.createChart(...args),
  createSeriesMarkers: (...args: unknown[]) => drillChartState.createSeriesMarkers(...args),
}))

import { useLightweightChart } from './useLightweightChart'
import { useDrillPriceChartRuntime } from './useDrillPriceChartRuntime'
import type { DrillChartModel } from '../visualizations/drillChartModel'
import { lightweightChartSeriesPalette } from '../visualizations/chartTheme'
import type { LightweightChartModule } from '../visualizations/lightweightChartRuntime'

function drillModel(): DrillChartModel {
  return {
    bars: [{
      time: 1_744_500_000,
      open: 100,
      high: 101,
      low: 99,
      close: 100.5,
      fromIndex: 1,
      toIndex: 1,
      volume: 0,
    }],
    candleData: [{
      time: 1_744_500_000,
      open: 100,
      high: 101,
      low: 99,
      close: 100.5,
    }],
    fastData: [{ time: 1_744_500_000, value: 100.2 }],
    slowData: [{ time: 1_744_500_000, value: 100.1 }],
    halftrendData: [],
    ottData: [],
    maczData: [],
    trades: [],
    tradeStats: {},
  }
}

function namedDrillModel(): DrillChartModel {
  const model = drillModel()
  model.strategyIndicators = {
    strategyKind: 'Asr',
    aggregation: 'last_finite',
    bars: [{ time: 1_744_500_000, fromIndex: 1, toIndex: 1 }],
    series: [
      {
        id: 'kaufman-short',
        label: 'Kaufman short',
        kind: 'line',
        paneId: 'price',
        scaleId: 'price',
        unit: 'price',
        defaultVisible: true,
        data: [{ time: 1_744_500_000, value: 100.25 }],
        finitePointCount: 1,
        referenceLines: [],
      },
      {
        id: 'angle',
        label: 'Angle',
        kind: 'histogram',
        paneId: 'angle',
        scaleId: 'degrees',
        unit: 'degrees',
        defaultVisible: true,
        data: [{ time: 1_744_500_000, value: 12.5 }],
        finitePointCount: 1,
        referenceLines: [{ value: 0, label: 'Neutral' }],
      },
      {
        id: 'z-score',
        label: 'Z-score',
        kind: 'line',
        paneId: 'z-score',
        scaleId: 'z-score',
        unit: 'z_score',
        defaultVisible: false,
        data: [{ time: 1_744_500_000, value: -0.75 }],
        finitePointCount: 1,
        referenceLines: [
          { value: 2, label: 'Upper' },
          { value: -2, label: 'Lower' },
        ],
      },
      {
        id: 'empty',
        label: 'Unavailable',
        kind: 'line',
        paneId: 'empty',
        scaleId: 'number',
        unit: 'number',
        defaultVisible: false,
        data: [{ time: 1_744_500_000 }],
        finitePointCount: 0,
        referenceLines: [],
      },
    ],
  }
  return model
}

describe('useLightweightChart', () => {
  beforeEach(() => {
    resizeObserverState.callback = null
    resizeObserverState.stop = vi.fn()
    class ResizeObserverMock {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
    ;(globalThis as typeof globalThis & { ResizeObserver?: typeof ResizeObserver }).ResizeObserver =
      ResizeObserverMock as unknown as typeof ResizeObserver
    drillChartState.charts.length = 0
    drillChartState.crosshairHandler = null
    drillChartState.createChart.mockReset()
    drillChartState.createSeriesMarkers.mockReset()
    drillChartState.createSeriesMarkers.mockImplementation(() => ({
      setMarkers: vi.fn(),
    }))
    drillChartState.createChart.mockImplementation(() => {
      let visibleLogicalRange = { from: 0, to: 1 }
      const series: Array<{
        applyOptions: ReturnType<typeof vi.fn>
        setData: ReturnType<typeof vi.fn>
        priceScale: () => { applyOptions: ReturnType<typeof vi.fn> }
        createPriceLine: ReturnType<typeof vi.fn>
        removePriceLine: ReturnType<typeof vi.fn>
        referenceLines: Array<{ applyOptions: ReturnType<typeof vi.fn> }>
        paneIndex: number
        removed: boolean
      }> = []
      const panes: Array<{
        getSeries: () => unknown[]
        setStretchFactor: ReturnType<typeof vi.fn>
      }> = []
      const ensurePane = (paneIndex: number): void => {
        while (panes.length <= paneIndex) {
          const nextPaneIndex = panes.length
          panes.push({
            getSeries: () => series.filter(
              (item) => !item.removed && item.paneIndex === nextPaneIndex,
            ),
            setStretchFactor: vi.fn(),
          })
        }
      }
      ensurePane(0)
      const timeScaleApi = {
        fitContent: vi.fn(),
        getVisibleLogicalRange: vi.fn(() => visibleLogicalRange),
        setVisibleLogicalRange: vi.fn((range: { from: number; to: number }) => {
          visibleLogicalRange = range
        }),
        subscribeVisibleLogicalRangeChange: vi.fn(),
        unsubscribeVisibleLogicalRangeChange: vi.fn(),
      }
      const chart = {
        addSeries: vi.fn((_definition: unknown, _options: unknown, paneIndexRaw?: number) => {
          const paneIndex = Number.isFinite(Number(paneIndexRaw))
            ? Math.max(0, Math.trunc(Number(paneIndexRaw)))
            : 0
          ensurePane(paneIndex)
          const referenceLines: Array<{ applyOptions: ReturnType<typeof vi.fn> }> = []
          const nextSeries = {
            applyOptions: vi.fn(),
            setData: vi.fn(),
            priceScale: () => ({ applyOptions: vi.fn() }),
            createPriceLine: vi.fn(() => {
              const referenceLine = { applyOptions: vi.fn() }
              referenceLines.push(referenceLine)
              return referenceLine
            }),
            removePriceLine: vi.fn(),
            referenceLines,
            paneIndex,
            removed: false,
          }
          series.push(nextSeries)
          return nextSeries
        }),
        series,
        panes: () => panes,
        removeSeries: vi.fn((target: { removed?: boolean }) => {
          target.removed = true
        }),
        removePane: vi.fn((paneIndex: number) => {
          panes.splice(paneIndex, 1)
        }),
        remove: vi.fn(),
        applyOptions: vi.fn(),
        subscribeClick: vi.fn(),
        unsubscribeClick: vi.fn(),
        subscribeCrosshairMove: vi.fn((handler: (param: unknown) => void) => {
          drillChartState.crosshairHandler = handler
        }),
        unsubscribeCrosshairMove: vi.fn(),
        setCrosshairPosition: vi.fn(),
        clearCrosshairPosition: vi.fn(),
        timeScaleApi,
        timeScale: () => timeScaleApi,
      }
      drillChartState.charts.push(chart)
      return chart
    })
    document.documentElement.dataset.theme = 'dark'
    document.body.innerHTML = ''
  })

  afterEach(() => {
    delete document.documentElement.dataset.theme
    document.body.innerHTML = ''
  })

  it('observes host resizes and disposes the shared chart runtime cleanly', () => {
    const runtime = useLightweightChart<{ id: string }>()
    const host = document.createElement('div')
    const onResize = vi.fn()
    const removeChart = vi.fn()
    const chart = { id: 'chart_a' }

    runtime.setHost(host)
    runtime.setChart(chart)
    runtime.observeHostResize(onResize)

    const rect = { width: 640, height: 240 } as DOMRectReadOnly
    resizeObserverState.callback?.([{ contentRect: rect }])

    expect(onResize).toHaveBeenCalledWith(rect)

    runtime.dispose(removeChart)

    expect(resizeObserverState.stop).toHaveBeenCalledTimes(1)
    expect(removeChart).toHaveBeenCalledWith(chart)
    expect(runtime.chartRef.value).toBeNull()
    expect(runtime.hostRef.value).toBeNull()
  })

  it('stops the prior resize observer before registering a new one', () => {
    const runtime = useLightweightChart<{ id: string }>()
    runtime.setHost(document.createElement('div'))

    runtime.observeHostResize(() => undefined)
    const firstStop = resizeObserverState.stop

    runtime.observeHostResize(() => undefined)

    expect(firstStop).toHaveBeenCalledTimes(1)
  })

  describe('useDrillPriceChartRuntime lifecycle', () => {
  it('coalesces concurrent renders into one chart creation', async () => {
    const runtime = useDrillPriceChartRuntime()
    const host = document.createElement('div')
    document.body.appendChild(host)
    runtime.setHost(host)

    await Promise.all([
      runtime.render(drillModel(), -1),
      runtime.render(drillModel(), -1),
    ])

    expect(drillChartState.createChart).toHaveBeenCalledTimes(1)
    expect(drillChartState.createChart).toHaveBeenCalledWith(host, expect.any(Object))

    runtime.dispose()
    expect(drillChartState.charts[0]?.remove).toHaveBeenCalledTimes(1)
  })

  it('does not create a detached chart after disposal during module loading', async () => {
    const onRenderStateChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onRenderStateChange })
    const host = document.createElement('div')
    runtime.setHost(host)

    const rendering = runtime.render(drillModel(), -1)
    runtime.dispose()
    await rendering

    expect(drillChartState.createChart).not.toHaveBeenCalled()
    expect(onRenderStateChange).toHaveBeenLastCalledWith('idle')
    expect(onRenderStateChange).not.toHaveBeenCalledWith('ready')
  })

  it('does not attach a chart after clear invalidates module loading', async () => {
    const onRenderStateChange = vi.fn()
    const onMarkerAvailabilityChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onRenderStateChange,
      onMarkerAvailabilityChange,
    })
    runtime.setHost(document.createElement('div'))

    const rendering = runtime.render(drillModel(), -1)
    runtime.clear()
    await rendering

    expect(drillChartState.createChart).not.toHaveBeenCalled()
    expect(onRenderStateChange).toHaveBeenLastCalledWith('idle')
    expect(onRenderStateChange).not.toHaveBeenCalledWith('ready')
    expect(onMarkerAvailabilityChange).not.toHaveBeenCalled()
    runtime.dispose()
  })

  it('invalidates pending creation when the chart host changes', async () => {
    const runtime = useDrillPriceChartRuntime()
    const firstHost = document.createElement('div')
    const secondHost = document.createElement('div')
    runtime.setHost(firstHost)

    const firstRender = runtime.render(drillModel(), -1)
    runtime.setHost(secondHost)
    const secondRender = runtime.render(drillModel(), -1)
    await Promise.all([firstRender, secondRender])

    expect(drillChartState.createChart).toHaveBeenCalledTimes(1)
    expect(drillChartState.createChart).toHaveBeenCalledWith(secondHost, expect.any(Object))
    runtime.dispose()
  })

  it('publishes loading and ready only after the primary candle series renders', async () => {
    const onRenderStateChange = vi.fn()
    const onMarkerAvailabilityChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onRenderStateChange,
      onMarkerAvailabilityChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(drillModel(), -1)

    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })
    expect(onRenderStateChange.mock.calls.map(([state]) => state)).toEqual([
      'loading',
      'ready',
    ])
    expect(onMarkerAvailabilityChange).toHaveBeenCalledWith('interactive')
    expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    const readyCallIndex = onRenderStateChange.mock.calls.findIndex(
      ([state]) => state === 'ready',
    )
    const candleSetDataCallOrder =
      drillChartState.charts[0]?.series[0]?.setData.mock.invocationCallOrder ?? []
    expect(
      candleSetDataCallOrder[candleSetDataCallOrder.length - 1],
    ).toBeLessThan(onRenderStateChange.mock.invocationCallOrder[readyCallIndex] ?? 0)
    runtime.dispose()
  })

  it('reports chart creation failure without publishing a false ready state', async () => {
    drillChartState.createChart.mockImplementationOnce(() => {
      throw new Error('chart creation unavailable')
    })
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(drillModel(), -1)

    expect(onRenderStateChange).toHaveBeenLastCalledWith('error')
    expect(onRenderStateChange).not.toHaveBeenCalledWith('ready')
    expect(onMetaChange).toHaveBeenLastCalledWith('Unable to create drill chart runtime.')
    const model = drillModel()
    model.trades = [{ entry_t: 1, direction: 1, open: true }]
    runtime.updateSelectedTrade(model, 0)
    expect(onMetaChange).toHaveBeenLastCalledWith('Unable to create drill chart runtime.')
    runtime.dispose()
  })

  it('retries a failed chart module load on the next render', async () => {
    const chartModule =
      await import('lightweight-charts') as unknown as LightweightChartModule
    let attempts = 0
    const loadChartModule = vi.fn(async (): Promise<LightweightChartModule> => {
      attempts += 1
      if (attempts === 1) throw new Error('temporary module load failure')
      return chartModule
    })
    const onRenderStateChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      loadChartModule,
      onRenderStateChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(drillModel(), -1)
    expect(onRenderStateChange).toHaveBeenLastCalledWith('error')
    expect(drillChartState.createChart).not.toHaveBeenCalled()

    await runtime.render(drillModel(), -1)
    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })

    expect(loadChartModule).toHaveBeenCalledTimes(2)
    expect(drillChartState.createChart).toHaveBeenCalledTimes(1)
    runtime.dispose()
  })

  it('reports primary candle rendering failure without publishing ready', async () => {
    const defaultCreateChart = drillChartState.createChart.getMockImplementation()
    drillChartState.createChart.mockImplementationOnce((...args: unknown[]) => {
      const chart = defaultCreateChart?.(...args)
      if (!chart) throw new Error('missing default chart mock')
      const addSeries = chart.addSeries
      let seriesIndex = 0
      chart.addSeries = vi.fn((...seriesArgs: unknown[]) => {
        const series = addSeries(...seriesArgs)
        if (seriesIndex === 0) {
          series.setData.mockImplementationOnce(() => {
            throw new Error('candle rendering unavailable')
          })
        }
        seriesIndex += 1
        return series
      })
      return chart
    })
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(drillModel(), -1)

    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('error')
    })
    expect(onRenderStateChange).not.toHaveBeenCalledWith('ready')
    expect(onMetaChange).toHaveBeenLastCalledWith('Unable to render drill chart.')
    const failedModel = drillModel()
    failedModel.trades = [{ entry_t: 1, direction: 1, open: true }]
    runtime.updateSelectedTrade(failedModel, 0)
    document.documentElement.dataset.theme = 'light'
    await Promise.resolve()
    expect(onMetaChange).toHaveBeenLastCalledWith('Unable to render drill chart.')
    runtime.dispose()
  })

  it('keeps candles ready and reports optional overlay creation and rendering failures', async () => {
    const defaultCreateChart = drillChartState.createChart.getMockImplementation()
    drillChartState.createChart.mockImplementationOnce((...args: unknown[]) => {
      const chart = defaultCreateChart?.(...args)
      if (!chart) throw new Error('missing default chart mock')
      const addSeries = chart.addSeries
      let addSeriesCall = 0
      chart.addSeries = vi.fn((...seriesArgs: unknown[]) => {
        addSeriesCall += 1
        if (addSeriesCall === 2) {
          throw new Error('fast overlay creation unavailable')
        }
        const series = addSeries(...seriesArgs)
        if (addSeriesCall === 6) {
          series.setData.mockImplementationOnce(() => {
            throw new Error('macz overlay rendering unavailable')
          })
        }
        return series
      })
      return chart
    })
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const model = drillModel()
    model.halftrendData = [{ time: 1_744_500_000, value: 1 }]
    model.ottData = [{ time: 1_744_500_000, value: 99.8 }]
    model.maczData = [{ time: 1_744_500_000, value: -0.12 }]
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(model, -1)
    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })

    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('fast=unavailable'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('slow=1'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('halftrend=1'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('ott=1'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('macz=unavailable'),
    )
    expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    runtime.dispose()
  })

  it('keeps candles ready while reporting an unavailable marker overlay honestly', async () => {
    drillChartState.createSeriesMarkers.mockImplementationOnce(() => null)
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const onMarkerAvailabilityChange = vi.fn()
    const model = drillModel()
    model.trades = [{ entry_t: 1, direction: 1, open: true }]
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
      onMarkerAvailabilityChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(model, -1)

    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })
    expect(onMarkerAvailabilityChange).toHaveBeenCalledWith('unavailable')
    expect(onMetaChange).toHaveBeenLastCalledWith(expect.stringContaining('markers=unavailable'))
    expect(onMetaChange).toHaveBeenLastCalledWith(expect.not.stringContaining('entries='))
    expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    runtime.dispose()
  })

  it('reports display-only markers when click subscription fails', async () => {
    const defaultCreateChart = drillChartState.createChart.getMockImplementation()
    drillChartState.createChart.mockImplementationOnce((...args: unknown[]) => {
      const chart = defaultCreateChart?.(...args)
      if (!chart) throw new Error('missing default chart mock')
      chart.subscribeClick = vi.fn(() => {
        throw new Error('click subscription unavailable')
      })
      return chart
    })
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const onMarkerAvailabilityChange = vi.fn()
    const model = drillModel()
    model.trades = [{ entry_t: 1, direction: 1, open: true }]
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
      onMarkerAvailabilityChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(model, -1)

    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })
    expect(onMarkerAvailabilityChange).toHaveBeenCalledWith('display-only')
    expect(onMetaChange).toHaveBeenLastCalledWith(expect.stringContaining('entries=1'))
    expect(onMetaChange).toHaveBeenLastCalledWith(expect.stringContaining('markers=1'))
    runtime.dispose()
  })

  it('focuses a selected trade entry and exit window when logical range control is available', async () => {
    const model = drillModel()
    const startTime = 1_744_500_000
    model.bars = Array.from({ length: 40 }, (_, index) => ({
      time: startTime + index * 60,
      open: 100 + index,
      high: 101 + index,
      low: 99 + index,
      close: 100.5 + index,
      fromIndex: index,
      toIndex: index,
      volume: 0,
    }))
    model.candleData = model.bars.map((bar) => ({
      time: bar.time,
      open: bar.open,
      high: bar.high,
      low: bar.low,
      close: bar.close,
    }))
    model.trades = [{
      entry_t: 10,
      exit_t: 15,
      direction: 1,
      open: false,
      pnl: 0.05,
    }]
    const onViewChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onViewChange })
    runtime.setHost(document.createElement('div'))

    await runtime.render(model, -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })
    const timeScale = drillChartState.charts[0]?.timeScaleApi
    timeScale?.setVisibleLogicalRange.mockClear()

    runtime.updateSelectedTrade(model, 0)

    expect(timeScale?.setVisibleLogicalRange).toHaveBeenCalledTimes(1)
    const range = timeScale?.setVisibleLogicalRange.mock.calls[0]?.[0] as
      | { from: number; to: number }
      | undefined
    expect(range?.from).toBeLessThanOrEqual(10)
    expect(range?.to).toBeGreaterThanOrEqual(15)
    expect((range?.to ?? 40) - (range?.from ?? 0)).toBeLessThan(40)
    expect(onViewChange).toHaveBeenLastCalledWith(
      `${range?.from.toFixed(1)}..${range?.to.toFixed(1)}`,
    )
    runtime.dispose()
  })

  it('publishes finite crosshair values, restores resting metadata, and unsubscribes', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    runtime.setHost(document.createElement('div'))
    await runtime.render(drillModel(), -1)
    const chart = drillChartState.charts[0]
    const handler = drillChartState.crosshairHandler

    expect(handler).toBeTypeOf('function')
    await vi.waitFor(() => {
      expect(onMetaChange).toHaveBeenCalledWith(expect.stringContaining('candles=1'))
    })
    handler?.({
      time: 1_744_500_000,
      seriesData: new Map([
        [chart?.series[0], {
          time: 1_744_500_000,
          open: 100,
          high: 101,
          low: 99,
          close: 100.5,
        }],
        [chart?.series[1], { time: 1_744_500_000, value: 100.2 }],
        [chart?.series[2], { time: 1_744_500_000, value: 100.1 }],
        [chart?.series[3], { time: 1_744_500_000, value: 1 }],
        [chart?.series[4], { time: 1_744_500_000, value: 99.8 }],
        [chart?.series[5], { time: 1_744_500_000, value: -0.12 }],
      ]),
    })

    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringMatching(
        /candles=1.*inspect\[pointer\]: .* ET \(UTC-04:00\) · O 100 · H 101 · L 99 · C 100\.5 · Fast MA 100\.2 · Slow MA 100\.1 · HalfTrend 1 · OTT 99\.8 · MAC-Z -0\.12/u,
      ),
    )

    handler?.({ seriesData: new Map() })
    expect(onMetaChange).toHaveBeenLastCalledWith(expect.stringContaining('candles=1'))

    runtime.dispose()
    expect(chart?.unsubscribeCrosshairMove).toHaveBeenCalledWith(handler)
  })

  it('supports ET-formatted keyboard point inspection and removes host shortcuts on disposal', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    const host = document.createElement('div')
    const model = drillModel()
    model.bars = [0, 1, 2].map((offset) => ({
      time: 1_744_500_000 + offset * 60,
      open: 100 + offset,
      high: 101 + offset,
      low: 99 + offset,
      close: 100.5 + offset,
      fromIndex: offset,
      toIndex: offset,
      volume: 0,
    }))
    model.candleData = model.bars.map((bar) => ({
      time: bar.time,
      open: bar.open,
      high: bar.high,
      low: bar.low,
      close: bar.close,
    }))
    model.fastData = model.bars.map((bar) => ({
      time: bar.time,
      value: bar.close - 0.1,
    }))
    document.body.appendChild(host)
    runtime.setHost(host)
    await runtime.render(model, -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })
    const chart = drillChartState.charts[0]

    host.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'End',
      bubbles: true,
      cancelable: true,
    }))

    expect(chart?.setCrosshairPosition).toHaveBeenLastCalledWith(
      102.5,
      1_744_500_120,
      chart?.series[0],
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringMatching(
        /inspect\[keyboard\]: .* ET \(UTC-04:00\) · O 102 · H 103 · L 101 · C 102\.5 · Fast MA 102\.4/u,
      ),
    )

    host.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'ArrowLeft',
      bubbles: true,
      cancelable: true,
    }))
    expect(chart?.setCrosshairPosition).toHaveBeenLastCalledWith(
      101.5,
      1_744_500_060,
      chart?.series[0],
    )

    host.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'Escape',
      bubbles: true,
      cancelable: true,
    }))
    expect(chart?.clearCrosshairPosition).toHaveBeenCalledTimes(1)
    expect(onMetaChange).toHaveBeenLastCalledWith(expect.stringContaining('candles=3'))
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.not.stringContaining('inspect['),
    )

    runtime.dispose()
    const crosshairCallsAfterDispose = chart?.setCrosshairPosition.mock.calls.length ?? 0
    host.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'Home',
      bubbles: true,
      cancelable: true,
    }))
    expect(chart?.setCrosshairPosition).toHaveBeenCalledTimes(crosshairCallsAfterDispose)
  })

  it('reacts to theme changes and stops observing after disposal', async () => {
    const runtime = useDrillPriceChartRuntime()
    const model = drillModel()
    model.halftrendData = [{ time: 1_744_500_000, value: 1 }]
    model.maczData = [{ time: 1_744_500_000, value: -0.12 }]
    model.trades = [{ entry_t: 1, direction: 1, open: true }]
    runtime.setHost(document.createElement('div'))
    await runtime.render(model, -1)
    const chart = drillChartState.charts[0]
    const markerApi = drillChartState.createSeriesMarkers.mock.results[0]?.value as {
      setMarkers: ReturnType<typeof vi.fn>
    }

    document.documentElement.dataset.theme = 'light'
    const light = lightweightChartSeriesPalette('light')
    await vi.waitFor(() => {
      expect(chart?.applyOptions).toHaveBeenCalledWith(expect.objectContaining({
        layout: expect.objectContaining({
          background: { type: 'solid', color: 'rgb(255, 253, 250)' },
          attributionLogo: true,
        }),
      }))
      expect(chart?.series[3]?.setData).toHaveBeenLastCalledWith([
        { time: 1_744_500_000, value: 1, color: light.positive },
      ])
      expect(chart?.series[5]?.setData).toHaveBeenLastCalledWith([
        { time: 1_744_500_000, value: -0.12, color: light.negative },
      ])
      expect(markerApi.setMarkers).toHaveBeenLastCalledWith([
        expect.objectContaining({
          id: 'trade-0-entry',
          time: 1_744_500_000,
          color: light.positive,
        }),
      ])
    })
    expect(chart?.series[0]?.applyOptions).toHaveBeenCalledWith(expect.objectContaining({
      upColor: light.positive,
      downColor: light.negative,
    }))
    expect(chart?.series[1]?.applyOptions).toHaveBeenCalledWith(expect.objectContaining({
      color: light.fast,
    }))

    runtime.dispose()
    const callsAfterDispose = chart?.applyOptions.mock.calls.length ?? 0
    const histogramCallsAfterDispose = chart?.series[3]?.setData.mock.calls.length ?? 0
    const markerCallsAfterDispose = markerApi.setMarkers.mock.calls.length
    document.documentElement.dataset.theme = 'dark'
    await Promise.resolve()
    expect(chart?.applyOptions).toHaveBeenCalledTimes(callsAfterDispose)
    expect(chart?.series[3]?.setData).toHaveBeenCalledTimes(histogramCallsAfterDispose)
    expect(markerApi.setMarkers).toHaveBeenCalledTimes(markerCallsAfterDispose)
  })

  it('renders only native default indicators and omits legacy overlay metadata', async () => {
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(namedDrillModel(), -1)
    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })

    const chart = drillChartState.charts[0]
    expect(chart?.series).toHaveLength(8)
    expect(chart?.series[6]?.setData).toHaveBeenCalledWith([
      { time: 1_744_500_000, value: 100.25 },
    ])
    expect(chart?.series[6]?.paneIndex).toBe(0)
    expect(chart?.series[7]?.setData).toHaveBeenCalledWith([
      expect.objectContaining({ time: 1_744_500_000, value: 12.5 }),
    ])
    expect(chart?.series[7]?.paneIndex).toBe(1)
    expect(chart?.panes()).toHaveLength(2)
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=2/2'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.not.stringContaining('fast='),
    )
    runtime.dispose()
  })

  it('rebuilds on native visibility structure changes, removes stale panes, and restores the view', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    runtime.setHost(document.createElement('div'))
    const model = namedDrillModel()

    await runtime.render(model, -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })
    const firstChart = drillChartState.charts[0]
    const setFirstVisibleRange = firstChart?.timeScaleApi
      .setVisibleLogicalRange as unknown as
        | ((range: { from: number; to: number }) => void)
        | undefined
    setFirstVisibleRange?.({ from: 4, to: 9 })

    await runtime.render(model, -1, new Set(['z-score']))
    await vi.waitFor(() => {
      expect(drillChartState.charts).toHaveLength(2)
      expect(drillChartState.charts[1]?.series[0]?.setData).toHaveBeenCalled()
    })

    const secondChart = drillChartState.charts[1]
    expect(firstChart?.removeSeries).toHaveBeenCalledTimes(2)
    expect(firstChart?.removePane).toHaveBeenCalledWith(1)
    expect(firstChart?.remove).toHaveBeenCalledTimes(1)
    expect(secondChart?.series).toHaveLength(7)
    expect(secondChart?.addSeries.mock.calls[6]?.[1]).toEqual(
      expect.objectContaining({ title: 'Z-score' }),
    )
    expect(secondChart?.series[6]?.paneIndex).toBe(1)
    expect(secondChart?.timeScaleApi.setVisibleLogicalRange).toHaveBeenLastCalledWith({
      from: 4,
      to: 9,
    })
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=1/1'),
    )

    await runtime.render(model, -1, new Set())
    await vi.waitFor(() => {
      expect(drillChartState.charts).toHaveLength(3)
      expect(drillChartState.charts[2]?.series[0]?.setData).toHaveBeenCalled()
    })
    expect(drillChartState.charts[2]?.panes()).toHaveLength(1)
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=hidden'),
    )
    runtime.dispose()
  })

  it('updates named-series visibility in place when the active pane layout is unchanged', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    const model = namedDrillModel()
    const angle = model.strategyIndicators?.series.find((series) => series.id === 'angle')
    if (!model.strategyIndicators || !angle) {
      throw new Error('angle fixture unavailable')
    }
    model.strategyIndicators.series.push({
      ...angle,
      id: 'angle-peer',
      label: 'Angle peer',
      defaultVisible: false,
      data: [{ time: 1_744_500_000, value: 18.25 }],
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(model, -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })
    const chart = drillChartState.charts[0]
    const candleSetDataCalls = chart?.series[0]?.setData.mock.calls.length ?? 0

    await runtime.render(
      model,
      -1,
      new Set(['kaufman-short', 'angle-peer']),
    )

    expect(drillChartState.createChart).toHaveBeenCalledTimes(1)
    expect(chart?.remove).not.toHaveBeenCalled()
    expect(chart?.series).toHaveLength(9)
    expect(chart?.series[7]?.applyOptions).toHaveBeenLastCalledWith({
      visible: false,
    })
    expect(chart?.series[8]?.applyOptions).toHaveBeenLastCalledWith({
      visible: true,
    })
    expect(chart?.series[0]?.setData).toHaveBeenCalledTimes(candleSetDataCalls)
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=2/2'),
    )
    runtime.dispose()
  })

  it('keeps candles ready without empty panes when selected indicators are all-null or fail', async () => {
    const onMetaChange = vi.fn()
    const onRenderStateChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      onRenderStateChange,
    })
    runtime.setHost(document.createElement('div'))

    await runtime.render(namedDrillModel(), -1, new Set(['empty']))
    await vi.waitFor(() => {
      expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    })
    expect(drillChartState.charts[0]?.series).toHaveLength(6)
    expect(drillChartState.charts[0]?.panes()).toHaveLength(1)
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=unavailable'),
    )

    const defaultCreateChart = drillChartState.createChart.getMockImplementation()
    drillChartState.createChart.mockImplementationOnce((...args: unknown[]) => {
      const chart = defaultCreateChart?.(...args)
      if (!chart) throw new Error('missing default chart mock')
      const addSeries = chart.addSeries
      let addSeriesCall = 0
      chart.addSeries = vi.fn((...seriesArgs: unknown[]) => {
        addSeriesCall += 1
        if (addSeriesCall === 7) {
          throw new Error('named series unavailable')
        }
        return addSeries(...seriesArgs)
      })
      return chart
    })

    await runtime.render(namedDrillModel(), -1, new Set(['z-score']))
    await vi.waitFor(() => {
      expect(drillChartState.charts).toHaveLength(2)
      expect(drillChartState.charts[1]?.series[0]?.setData).toHaveBeenCalled()
    })
    expect(onRenderStateChange).toHaveBeenLastCalledWith('ready')
    expect(drillChartState.charts[1]?.series).toHaveLength(6)
    expect(drillChartState.charts[1]?.panes()).toHaveLength(1)
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=0/1'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicator_failures=1'),
    )
    runtime.dispose()
  })

  it('lets the latest visibility request win while chart creation is pending', async () => {
    const chartModule =
      await import('lightweight-charts') as unknown as LightweightChartModule
    let resolveModule: (module: LightweightChartModule) => void = () => undefined
    const pendingModule = new Promise<LightweightChartModule>((resolve) => {
      resolveModule = resolve
    })
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({
      onMetaChange,
      loadChartModule: () => pendingModule,
    })
    runtime.setHost(document.createElement('div'))
    const model = namedDrillModel()

    const firstRender = runtime.render(model, -1)
    const secondRender = runtime.render(model, -1, new Set(['z-score']))
    resolveModule(chartModule)
    await Promise.all([firstRender, secondRender])
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })

    expect(drillChartState.createChart).toHaveBeenCalledTimes(1)
    expect(drillChartState.charts[0]?.series).toHaveLength(7)
    expect(drillChartState.charts[0]?.addSeries.mock.calls[6]?.[1]).toEqual(
      expect.objectContaining({ title: 'Z-score' }),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('indicators=1/1'),
    )
    runtime.dispose()
  })

  it('reapplies native series and reference-line themes with stable bundle colors', async () => {
    const runtime = useDrillPriceChartRuntime()
    runtime.setHost(document.createElement('div'))
    await runtime.render(namedDrillModel(), -1, new Set(['z-score']))
    const chart = drillChartState.charts[0]
    const namedSeries = chart?.series[6]
    const referenceLines = namedSeries?.referenceLines ?? []
    const dark = lightweightChartSeriesPalette('dark')

    expect(chart?.addSeries.mock.calls[6]?.[1]).toEqual(
      expect.objectContaining({ color: dark.ott, title: 'Z-score' }),
    )
    expect(referenceLines).toHaveLength(2)

    document.documentElement.dataset.theme = 'light'
    const light = lightweightChartSeriesPalette('light')
    await vi.waitFor(() => {
      expect(namedSeries?.applyOptions).toHaveBeenCalledWith(
        expect.objectContaining({ color: light.ott, title: 'Z-score' }),
      )
      expect(referenceLines[0]?.applyOptions).toHaveBeenCalledWith(
        expect.objectContaining({ color: light.ott, price: 2, title: 'Upper' }),
      )
      expect(referenceLines[1]?.applyOptions).toHaveBeenCalledWith(
        expect.objectContaining({ color: light.ott, price: -2, title: 'Lower' }),
      )
    })
    runtime.dispose()
  })

  it('supports indicator-only crosshair metadata and caps named values', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    const model = namedDrillModel()
    const baseSeries = model.strategyIndicators?.series[0]
    if (!model.strategyIndicators || !baseSeries) {
      throw new Error('named strategy fixture unavailable')
    }
    model.strategyIndicators.series = Array.from({ length: 8 }, (_, index) => ({
      ...baseSeries,
      id: `indicator-${index}`,
      label: `Indicator ${index}`,
      defaultVisible: true,
      data: [{ time: 1_744_500_000, value: index + 0.5 }],
    }))
    runtime.setHost(document.createElement('div'))

    await runtime.render(model, -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })
    const chart = drillChartState.charts[0]
    const handler = drillChartState.crosshairHandler
    const points = new Map<unknown, unknown>()
    for (let index = 0; index < 8; index += 1) {
      points.set(chart?.series[index + 6], {
        time: 1_744_500_000,
        value: index + 0.5,
      })
    }
    handler?.({ time: 1_744_500_000, seriesData: points })

    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringMatching(
        /inspect\[pointer\]: .* ET \(UTC-04:00\).*Indicator 0 0\.5.*Indicator 5 5\.5.*\+2 more/u,
      ),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.not.stringContaining('Indicator 6 '),
    )
    runtime.dispose()
  })

  it('clears and replaces hosts without retaining named registries, panes, or stale callbacks', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    const firstHost = document.createElement('div')
    const secondHost = document.createElement('div')
    runtime.setHost(firstHost)
    await runtime.render(namedDrillModel(), -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })
    const firstChart = drillChartState.charts[0]
    const staleCrosshairHandler = drillChartState.crosshairHandler

    runtime.clear()
    expect(firstChart?.removeSeries).toHaveBeenCalledTimes(2)
    expect(firstChart?.removePane).toHaveBeenCalledWith(1)
    expect(firstChart?.panes()).toHaveLength(1)

    runtime.setHost(secondHost)
    await runtime.render(namedDrillModel(), -1, new Set(['z-score']))
    await vi.waitFor(() => {
      expect(drillChartState.charts[1]?.series[0]?.setData).toHaveBeenCalled()
    })
    const callsBeforeStaleHover = onMetaChange.mock.calls.length
    staleCrosshairHandler?.({
      seriesData: new Map([
        [firstChart?.series[6], { time: 1_744_500_000, value: 123 }],
      ]),
    })
    expect(onMetaChange).toHaveBeenCalledTimes(callsBeforeStaleHover)
    expect(firstChart?.remove).toHaveBeenCalledTimes(1)

    const secondChart = drillChartState.charts[1]
    runtime.dispose()
    expect(secondChart?.removeSeries).toHaveBeenCalledTimes(1)
    expect(secondChart?.removePane).toHaveBeenCalledWith(1)
    expect(secondChart?.remove).toHaveBeenCalledTimes(1)
  })

  it('returns to the unchanged legacy overlay path after a named model', async () => {
    const onMetaChange = vi.fn()
    const runtime = useDrillPriceChartRuntime({ onMetaChange })
    runtime.setHost(document.createElement('div'))
    await runtime.render(namedDrillModel(), -1, new Set(['z-score']))
    await vi.waitFor(() => {
      expect(drillChartState.charts[0]?.series[0]?.setData).toHaveBeenCalled()
    })

    await runtime.render(drillModel(), -1)
    await vi.waitFor(() => {
      expect(drillChartState.charts[1]?.series[0]?.setData).toHaveBeenCalled()
    })
    const legacyChart = drillChartState.charts[1]
    expect(legacyChart?.series).toHaveLength(6)
    expect(legacyChart?.series[1]?.setData).toHaveBeenCalledWith([
      { time: 1_744_500_000, value: 100.2 },
    ])
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.stringContaining('fast=1'),
    )
    expect(onMetaChange).toHaveBeenLastCalledWith(
      expect.not.stringContaining('indicators='),
    )
    runtime.dispose()
  })
})

})
