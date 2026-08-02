
import { describe, expect, it, vi } from 'vitest'
import {
  applyLightweightChartSize,
  applyLightweightChartSeriesTheme,
  createCandlestickRuntime,
  createDrillSeriesRuntime,
  createNamedSeriesRuntime,
  fitLightweightChartContent,
  namedChartSeriesPriceFormat,
  observeLightweightChartTheme,
  panLightweightChart,
  visibleLogicalRangeLabel,
  type LightweightChartModule,
} from './lightweightChartRuntime'
import { lightweightChartSeriesPalette } from './chartTheme'

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

  it('creates the chart with the resolved root theme and attribution enabled', () => {
    document.documentElement.dataset.theme = 'light'
    const chart = {
      addSeries: vi.fn(() => ({ setData: vi.fn() })),
      remove: vi.fn(),
    }
    const createChart = vi.fn(() => chart)
    const module: LightweightChartModule = {
      CandlestickSeries: { type: 'candle' },
      createChart,
    }

    createCandlestickRuntime(host(), module)

    expect(createChart).toHaveBeenCalledWith(
      expect.any(HTMLElement),
      expect.objectContaining({
        layout: expect.objectContaining({
          background: { type: 'solid', color: 'rgb(255, 253, 250)' },
          attributionLogo: true,
        }),
      }),
    )
    delete document.documentElement.dataset.theme
  })

  it('reacts to root theme changes and disconnects cleanly', async () => {
    document.documentElement.dataset.theme = 'dark'
    const applyOptions = vi.fn()
    const stop = observeLightweightChartTheme({ applyOptions })

    document.documentElement.dataset.themePref = 'system'
    document.documentElement.dataset.theme = 'dark'
    await Promise.resolve()
    expect(applyOptions).not.toHaveBeenCalled()

    document.documentElement.dataset.theme = 'light'
    await vi.waitFor(() => {
      expect(applyOptions).toHaveBeenCalledWith(expect.objectContaining({
        layout: expect.objectContaining({
          background: { type: 'solid', color: 'rgb(255, 253, 250)' },
          attributionLogo: true,
        }),
      }))
    })

    stop()
    stop()
    const callsAfterStop = applyOptions.mock.calls.length
    document.documentElement.dataset.theme = 'dark'
    await Promise.resolve()
    expect(applyOptions).toHaveBeenCalledTimes(callsAfterStop)
    delete document.documentElement.dataset.themePref
    delete document.documentElement.dataset.theme
  })

  it('recolors every live series from the shared palette and isolates optional failures', async () => {
    document.documentElement.dataset.theme = 'dark'
    const chartApplyOptions = vi.fn()
    const candleApplyOptions = vi.fn()
    const fastApplyOptions = vi.fn()
    const slowApplyOptions = vi.fn(() => {
      throw new Error('detached optional series')
    })
    const halftrendApplyOptions = vi.fn()
    const ottApplyOptions = vi.fn()
    const maczApplyOptions = vi.fn()
    const series = {
      candleSeries: { applyOptions: candleApplyOptions },
      fastSeries: { applyOptions: fastApplyOptions },
      slowSeries: { applyOptions: slowApplyOptions },
      halftrendSeries: { applyOptions: halftrendApplyOptions },
      ottSeries: { applyOptions: ottApplyOptions },
      maczSeries: { applyOptions: maczApplyOptions },
    }
    const stop = observeLightweightChartTheme(
      { applyOptions: chartApplyOptions },
      { series, initialTheme: 'dark' },
    )

    document.documentElement.dataset.theme = 'light'
    const palette = lightweightChartSeriesPalette('light')
    await vi.waitFor(() => {
      expect(candleApplyOptions).toHaveBeenCalledWith(expect.objectContaining({
        upColor: palette.positive,
        downColor: palette.negative,
      }))
    })
    expect(fastApplyOptions).toHaveBeenCalledWith(expect.objectContaining({ color: palette.fast }))
    expect(slowApplyOptions).toHaveBeenCalledWith(expect.objectContaining({ color: palette.slow }))
    expect(halftrendApplyOptions).toHaveBeenCalledWith(expect.objectContaining({ color: palette.positive }))
    expect(ottApplyOptions).toHaveBeenCalledWith(expect.objectContaining({ color: palette.ott }))
    expect(maczApplyOptions).toHaveBeenCalledWith(expect.objectContaining({ color: palette.neutral }))
    expect(chartApplyOptions).toHaveBeenCalled()

    stop()
    const callsAfterStop = fastApplyOptions.mock.calls.length
    document.documentElement.dataset.theme = 'dark'
    await Promise.resolve()
    expect(fastApplyOptions).toHaveBeenCalledTimes(callsAfterStop)
    delete document.documentElement.dataset.theme
  })

  it('applies a series palette defensively without requiring every optional series', () => {
    const applyOptions = vi.fn()

    expect(() => applyLightweightChartSeriesTheme(
      { candleSeries: { applyOptions }, fastSeries: null },
      'dark',
    )).not.toThrow()
    expect(applyOptions).toHaveBeenCalledWith(expect.objectContaining({
      upColor: lightweightChartSeriesPalette('dark').positive,
    }))
  })

  it('treats theme observation as optional when MutationObserver setup fails', () => {
    const NativeMutationObserver = globalThis.MutationObserver
    class ThrowingMutationObserver {
      constructor() {
        throw new Error('observer unavailable')
      }
    }
    vi.stubGlobal('MutationObserver', ThrowingMutationObserver)

    expect(() => observeLightweightChartTheme({ applyOptions: vi.fn() })).not.toThrow()

    vi.stubGlobal('MutationObserver', NativeMutationObserver)
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

  it('places named series on deterministic v5 panes and maps oscillator scales safely', () => {
    const paneStretch = [vi.fn(), vi.fn(), vi.fn()]
    const panes = paneStretch.map((setStretchFactor) => ({
      setStretchFactor,
      getSeries: vi.fn(() => []),
    }))
    const addSeries = vi.fn((..._args: unknown[]) => ({
      setData: vi.fn(),
      createPriceLine: vi.fn(),
    }))
    const priceScaleApplyOptions = vi.fn()
    const chart = {
      addSeries,
      priceScale: vi.fn(() => ({ applyOptions: priceScaleApplyOptions })),
      panes: vi.fn(() => panes),
    }
    const module: LightweightChartModule = {
      LineSeries: { type: 'line' },
      HistogramSeries: { type: 'histogram' },
    }

    const runtime = createNamedSeriesRuntime(chart, module, [
      {
        id: 'momentum',
        label: 'Momentum',
        kind: 'histogram',
        paneId: 'momentum',
        scaleId: 'momentum',
        unit: 'score',
        data: [{ time: 1, value: 1 }],
      },
      {
        id: 'price-overlay',
        label: 'Price overlay',
        kind: 'line',
        paneId: 'price',
        scaleId: 'price',
        unit: 'price',
        data: [{ time: 1, value: 100 }],
      },
      {
        id: 'oscillator',
        label: 'Oscillator',
        kind: 'line',
        paneId: 'angle',
        scaleId: 'oscillator',
        unit: 'degrees',
        data: [{ time: 1, value: 20 }],
      },
      {
        id: 'oscillator-peer',
        label: 'Oscillator peer',
        kind: 'line',
        paneId: 'angle',
        scaleId: 'oscillator',
        unit: 'degrees',
        data: [{ time: 1, value: 25 }],
      },
      {
        id: 'angle-state',
        label: 'Angle state',
        kind: 'histogram',
        paneId: 'angle',
        scaleId: 'state',
        unit: 'state',
        data: [{ time: 1, value: 1 }],
      },
      {
        id: 'hidden',
        label: 'Hidden',
        kind: 'line',
        paneId: 'zzz',
        data: [],
        visible: false,
      },
    ], 'dark')

    expect(runtime.createdCount).toBe(5)
    expect(runtime.paneIndexById).toEqual(new Map([
      ['price', 0],
      ['angle', 1],
      ['momentum', 2],
    ]))
    expect(addSeries).toHaveBeenCalledTimes(5)
    expect(addSeries.mock.calls[0]?.[2]).toBe(2)
    expect(addSeries.mock.calls[0]?.[1]).toMatchObject({ priceScaleId: 'right' })
    expect(addSeries.mock.calls[1]?.[2]).toBe(0)
    expect(addSeries.mock.calls[1]?.[1]).not.toHaveProperty('priceScaleId')
    expect(addSeries.mock.calls[2]?.[2]).toBe(1)
    expect(addSeries.mock.calls[2]?.[1]).toMatchObject({ priceScaleId: 'right' })
    expect(addSeries.mock.calls[3]?.[1]).toMatchObject({ priceScaleId: 'right' })
    expect(addSeries.mock.calls[4]?.[1]).toMatchObject({
      priceScaleId: 'named:angle:state',
    })
    expect(paneStretch[0]).toHaveBeenCalledWith(3)
    expect(paneStretch[1]).toHaveBeenCalledWith(1)
    expect(paneStretch[2]).toHaveBeenCalledWith(1)
    expect(priceScaleApplyOptions).toHaveBeenCalledWith(expect.objectContaining({
      visible: true,
    }))
    expect(priceScaleApplyOptions).toHaveBeenCalledWith(expect.objectContaining({
      visible: false,
    }))
  })

  it('keeps explicitly resident series hidden and reconciles visibility without recreation', () => {
    const applyOptions = vi.fn()
    const referenceApplyOptions = vi.fn()
    const series = {
      setData: vi.fn(),
      applyOptions,
      createPriceLine: vi.fn(() => ({ applyOptions: referenceApplyOptions })),
    }
    const addSeries = vi.fn(() => series)
    const runtime = createNamedSeriesRuntime(
      { addSeries },
      { LineSeries: { type: 'line' } },
      [{
        id: 'resident-overlay',
        label: 'Resident overlay',
        kind: 'line',
        paneId: 'price',
        data: [{ time: 1, value: 100 }],
        visible: false,
        resident: true,
        referenceLines: [{ value: 100, label: 'Anchor' }],
      }],
      'dark',
    )

    expect(runtime.createdCount).toBe(1)
    expect(addSeries).toHaveBeenCalledWith(
      { type: 'line' },
      expect.objectContaining({ visible: false }),
      0,
    )
    expect(runtime.setVisibleSeriesIds(new Set(['resident-overlay']))).toBe(true)
    expect(applyOptions).toHaveBeenLastCalledWith({ visible: true })
    expect(referenceApplyOptions).toHaveBeenLastCalledWith({
      lineVisible: true,
      axisLabelVisible: true,
    })
    expect(runtime.registry.get('resident-overlay')?.input.visible).toBe(true)

    expect(runtime.setVisibleSeriesIds(new Set())).toBe(true)
    expect(applyOptions).toHaveBeenLastCalledWith({ visible: false })
    expect(referenceApplyOptions).toHaveBeenLastCalledWith({
      lineVisible: false,
      axisLabelVisible: false,
    })
  })

  it('keeps whitespace points and uses unit-aware formats and themed histogram colors', () => {
    const setLineData = vi.fn()
    const setHistogramData = vi.fn()
    const addSeries = vi
      .fn()
      .mockReturnValueOnce({ setData: setLineData })
      .mockReturnValueOnce({ setData: setHistogramData })
    const chart = { addSeries }
    const module: LightweightChartModule = {
      LineSeries: { type: 'line' },
      HistogramSeries: { type: 'histogram' },
    }

    createNamedSeriesRuntime(chart, module, [
      {
        id: 'price',
        label: 'Price',
        kind: 'line',
        paneId: 'price',
        unit: 'price',
        colorIndex: 2,
        data: [
          { time: 1, value: null },
          { time: 2, value: 101.25 },
        ],
      },
      {
        id: 'signal',
        label: 'Signal',
        kind: 'histogram',
        paneId: 'signal',
        unit: 'state',
        data: [
          { time: 1 },
          { time: 2, value: -1 },
          { time: 3, value: 1 },
        ],
      },
    ], 'light')

    const palette = lightweightChartSeriesPalette('light')
    expect(setLineData).toHaveBeenCalledWith([
      { time: 1 },
      { time: 2, value: 101.25 },
    ])
    expect(setHistogramData).toHaveBeenCalledWith([
      { time: 1 },
      { time: 2, value: -1, color: palette.negative },
      { time: 3, value: 1, color: palette.positive },
    ])
    expect(addSeries.mock.calls[0]?.[1]).toMatchObject({
      priceFormat: { type: 'price', precision: 4, minMove: 0.0001 },
    })
    expect(addSeries.mock.calls[1]?.[1]).toMatchObject({
      priceFormat: { type: 'price', precision: 0, minMove: 1 },
    })
    expect(namedChartSeriesPriceFormat('percent')).toMatchObject({ type: 'percent' })
    const angleFormat = namedChartSeriesPriceFormat('degrees')
    expect(angleFormat.type).toBe('custom')
    expect((angleFormat.formatter as (value: number) => string)(12.25)).toBe('12.3°')
  })

  it('keeps fallback colors stable when visible siblings change', () => {
    const firstAddSeries = vi.fn((..._args: unknown[]) => ({ setData: vi.fn() }))
    const secondAddSeries = vi.fn((..._args: unknown[]) => ({ setData: vi.fn() }))
    const module: LightweightChartModule = { LineSeries: { type: 'line' } }
    const target = {
      id: 'stable-target',
      label: 'Stable target',
      kind: 'line' as const,
      paneId: 'price',
      data: [{ time: 1, value: 1 }],
    }

    createNamedSeriesRuntime({ addSeries: firstAddSeries }, module, [
      {
        id: 'earlier-sibling',
        label: 'Earlier sibling',
        kind: 'line',
        paneId: 'price',
        data: [{ time: 1, value: 2 }],
      },
      target,
    ], 'dark')
    createNamedSeriesRuntime({ addSeries: secondAddSeries }, module, [target], 'dark')

    const firstOptions = firstAddSeries.mock.calls[1]?.[1] as Record<string, unknown>
    const secondOptions = secondAddSeries.mock.calls[0]?.[1] as Record<string, unknown>
    expect(firstOptions.color).toBe(secondOptions.color)
  })

  it('isolates named-series, data, and reference-line failures', () => {
    const goodReference = { applyOptions: vi.fn() }
    const dataFailureSeries = {
      setData: vi.fn(() => {
        throw new Error('data detached')
      }),
      createPriceLine: vi
        .fn()
        .mockReturnValueOnce(goodReference)
        .mockImplementationOnce(() => {
          throw new Error('reference rejected')
        }),
    }
    const goodSeries = { setData: vi.fn(), createPriceLine: vi.fn() }
    const addSeries = vi.fn((_definition, options: Record<string, unknown>) => {
      if (options.title === 'Broken create') throw new Error('definition rejected')
      return options.title === 'Data failure' ? dataFailureSeries : goodSeries
    })
    const chart = { addSeries }
    const module: LightweightChartModule = {
      LineSeries: { type: 'line' },
    }

    const runtime = createNamedSeriesRuntime(chart, module, [
      {
        id: 'broken',
        label: 'Broken create',
        kind: 'line',
        paneId: 'price',
        data: [],
      },
      {
        id: 'partial',
        label: 'Data failure',
        kind: 'line',
        paneId: 'oscillator',
        data: [{ time: 1, value: 1 }],
        referenceLines: [
          { value: 0, label: 'Zero' },
          { value: 1, label: 'One' },
          { value: Number.NaN, label: 'Invalid' },
        ],
      },
      {
        id: 'good',
        label: 'Good',
        kind: 'line',
        paneId: 'price',
        data: [{ time: 1, value: 2 }],
      },
    ])

    expect(runtime.createdCount).toBe(2)
    expect(runtime.registry.has('partial')).toBe(true)
    expect(runtime.registry.has('good')).toBe(true)
    expect(runtime.failures).toEqual(expect.arrayContaining([
      expect.objectContaining({ seriesId: 'broken', stage: 'series' }),
      expect.objectContaining({ seriesId: 'partial', stage: 'data' }),
      expect.objectContaining({
        seriesId: 'partial',
        stage: 'reference-line',
        referenceLineIndex: 1,
      }),
      expect.objectContaining({
        seriesId: 'partial',
        stage: 'reference-line',
        referenceLineIndex: 2,
      }),
    ]))
    expect(runtime.registry.get('partial')?.referenceLines).toHaveLength(1)
  })

  it('rethemes and disposes named series with per-resource failure isolation', () => {
    const referenceApplyOptions = vi.fn()
    const referenceApi = { applyOptions: referenceApplyOptions }
    const setData = vi.fn()
    const applyOptions = vi.fn()
    const removePriceLine = vi.fn()
    const series = {
      setData,
      applyOptions,
      createPriceLine: vi.fn(() => referenceApi),
      removePriceLine,
    }
    const paneZero = { setStretchFactor: vi.fn(), getSeries: vi.fn(() => [{}]) }
    const paneOne = { setStretchFactor: vi.fn(), getSeries: vi.fn(() => []) }
    const removeSeries = vi.fn()
    const removePane = vi.fn()
    const panes = vi
      .fn()
      .mockReturnValueOnce([paneZero])
      .mockReturnValue([paneZero, paneOne])
    const chart = {
      addSeries: vi.fn(() => series),
      removeSeries,
      panes,
      removePane,
    }
    const module: LightweightChartModule = {
      HistogramSeries: { type: 'histogram' },
    }
    const runtime = createNamedSeriesRuntime(chart, module, [{
      id: 'state',
      label: 'State',
      kind: 'histogram',
      paneId: 'state',
      colorIndex: 3,
      data: [{ time: 1, value: -1 }, { time: 2, value: 1 }],
      referenceLines: [{ value: 0, label: 'Neutral', colorIndex: 6 }],
    }], 'dark')

    runtime.reapplyTheme('light')

    const lightPalette = lightweightChartSeriesPalette('light')
    expect(runtime.theme).toBe('light')
    expect(applyOptions).toHaveBeenCalledWith(expect.objectContaining({
      color: lightPalette.positive,
    }))
    expect(setData).toHaveBeenLastCalledWith([
      { time: 1, value: -1, color: lightPalette.negative },
      { time: 2, value: 1, color: lightPalette.positive },
    ])
    expect(referenceApplyOptions).toHaveBeenCalledWith(expect.objectContaining({
      color: lightPalette.neutral,
      price: 0,
    }))

    runtime.dispose()
    runtime.dispose()

    expect(removePriceLine).toHaveBeenCalledTimes(1)
    expect(removeSeries).toHaveBeenCalledWith(series)
    expect(removePane).toHaveBeenCalledWith(1)
    expect(runtime.registry.size).toBe(0)
    expect(runtime.disposed).toBe(true)
  })

  it('removes the chart when no candle series can be created', () => {
    const chart = { remove: vi.fn() }
    const module: LightweightChartModule = {
      createChart: vi.fn(() => chart),
    }

    expect(createCandlestickRuntime(host(), module)).toBeNull()
    expect(chart.remove).toHaveBeenCalledTimes(1)
  })

  it('keeps the candle runtime usable when marker plugin initialization fails', () => {
    const candleSeries = { setData: vi.fn() }
    const chart = {
      addSeries: vi.fn(() => candleSeries),
      remove: vi.fn(),
    }
    const module: LightweightChartModule = {
      CandlestickSeries: { type: 'candle' },
      createChart: vi.fn(() => chart),
      createSeriesMarkers: vi.fn(() => {
        throw new Error('marker plugin failed')
      }),
    }

    const runtime = createCandlestickRuntime(host(), module)

    expect(runtime?.candleSeries).toBe(candleSeries)
    expect(runtime?.markerApi).toBeNull()
    expect(chart.remove).not.toHaveBeenCalled()
  })

  it('exposes an unusable marker plugin result as unavailable to callers', () => {
    const candleSeries = { setData: vi.fn() }
    const chart = {
      addSeries: vi.fn(() => candleSeries),
      remove: vi.fn(),
    }
    const module: LightweightChartModule = {
      CandlestickSeries: { type: 'candle' },
      createChart: vi.fn(() => chart),
      createSeriesMarkers: vi.fn(() => ({})),
    }

    const runtime = createCandlestickRuntime(host(), module)

    expect(runtime?.candleSeries).toBe(candleSeries)
    expect(runtime?.markerApi).toBeNull()
    expect(chart.remove).not.toHaveBeenCalled()
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
