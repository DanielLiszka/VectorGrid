

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  disposeExecutionChart,
  invalidateExecutionPriceBarsCache,
  summarizeExecutionChartSurface,
  syncExecutionChart,
} from './executionChartService'
import { lightweightChartSeriesPalette } from '../visualizations/chartTheme'

const createChartMock = vi.fn()
const addSeriesMock = vi.fn()
const setDataMock = vi.fn()
const fitContentMock = vi.fn()
const removeMock = vi.fn()
const setMarkersMock = vi.fn()
const createSeriesMarkersMock = vi.fn()
const createPriceLineMock = vi.fn()
const removePriceLineMock = vi.fn()
const priceLineApplyOptionsMock = vi.fn()
const applyOptionsMock = vi.fn()
const seriesApplyOptionsMock = vi.fn()
const resizeObserveMock = vi.fn()
const resizeUnobserveMock = vi.fn()
const resizeDisconnectMock = vi.fn()
const chartModuleImportState = vi.hoisted(() => ({
  attempts: 0,
  failuresRemaining: 0,
}))

function input(overrides: Partial<Parameters<typeof syncExecutionChart>[1]> = {}) {
  return {
    dataId: 'AAPL',
    datasetLabel: 'AAPL',
    datasetSymbol: 'AAPL',
    symbol: 'AAPL',
    brokerReviewLoaded: true,
    fetchedAtUnixMs: Date.now(),
    staleThresholdMs: 90_000,
    orderRows: [],
    ...overrides,
  }
}

function bars(close = 180.8) {
  return [
    { time: 1_744_500_000, open: 180, high: 181, low: 179.5, close },
    { time: 1_744_500_060, open: close, high: 181.2, low: 180.4, close: 181.0 },
  ]
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => []),
}))

vi.mock('lightweight-charts', () => {
  chartModuleImportState.attempts += 1
  if (chartModuleImportState.failuresRemaining > 0) {
    chartModuleImportState.failuresRemaining -= 1
    throw new Error('transient chart module import failure')
  }
  return {
    CandlestickSeries: {},
    createChart: (...args: unknown[]) => createChartMock(...args),
    createSeriesMarkers: (...args: unknown[]) => createSeriesMarkersMock(...args),
  }
})

describe('executionChartService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    invalidateExecutionPriceBarsCache()
    setDataMock.mockReset()
    fitContentMock.mockReset()
    removeMock.mockReset()
    setMarkersMock.mockReset()
    createPriceLineMock.mockReset()
    removePriceLineMock.mockReset()
    priceLineApplyOptionsMock.mockReset()
    applyOptionsMock.mockReset()
    seriesApplyOptionsMock.mockReset()
    createPriceLineMock.mockImplementation(() => ({
      applyOptions: priceLineApplyOptionsMock,
    }))
    addSeriesMock.mockImplementation(() => ({
      applyOptions: seriesApplyOptionsMock,
      setData: setDataMock,
      createPriceLine: createPriceLineMock,
      removePriceLine: removePriceLineMock,
    }))
    createChartMock.mockImplementation(() => ({
      addSeries: addSeriesMock,
      timeScale: () => ({ fitContent: fitContentMock }),
      applyOptions: applyOptionsMock,
      remove: removeMock,
    }))
    createSeriesMarkersMock.mockImplementation(() => ({
      setMarkers: setMarkersMock,
    }))
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(target: Element): void {
          resizeObserveMock(target)
        }

        unobserve(target: Element): void {
          resizeUnobserveMock(target)
        }

        disconnect(): void {
          resizeDisconnectMock()
        }
      },
    )
    document.documentElement.dataset.theme = 'dark'
  })

  afterEach(() => {
    disposeExecutionChart()
    invalidateExecutionPriceBarsCache()
    delete document.documentElement.dataset.theme
    document.body.innerHTML = ''
  })

  it('suppresses the chart when no symbol is selected', () => {
    const result = summarizeExecutionChartSurface({
      dataId: 'AAPL',
      datasetLabel: 'AAPL',
      datasetSymbol: 'AAPL',
      symbol: '',
      brokerReviewLoaded: false,
      fetchedAtUnixMs: null,
      staleThresholdMs: 90_000,
      orderRows: [],
    })

    expect(result.state).toBe('no_symbol')
    expect(result.summary).toContain('select a symbol first')
  })

  it('counts only broker-confirmed rows as chart markers', () => {
    const result = summarizeExecutionChartSurface({
      dataId: 'AAPL',
      datasetLabel: 'AAPL',
      datasetSymbol: 'AAPL',
      symbol: 'AAPL',
      brokerReviewLoaded: true,
      fetchedAtUnixMs: Date.now(),
      staleThresholdMs: 90_000,
      orderRows: [
        { cells: ['2026-04-12 06:00:00', 'BUY', 'filled', '1', '1', '182.10', 'ord-1'] },
        { cells: ['2026-04-12 06:01:00', 'SELL', 'submitted*', '1', '-', '-', 'ord-local'] },
      ],
    })

    expect(result.state).toBe('loaded_broker_markers')
    expect(result.markerCount).toBe(1)
    expect(result.summary).toContain('1 broker-confirmed order marker')
  })

  it('flags stale broker review separately from dataset readiness', () => {
    const result = summarizeExecutionChartSurface({
      dataId: 'AAPL',
      datasetLabel: 'AAPL',
      datasetSymbol: 'AAPL',
      symbol: 'AAPL',
      brokerReviewLoaded: true,
      fetchedAtUnixMs: Date.now() - 180_000,
      staleThresholdMs: 90_000,
      orderRows: [],
    })

    expect(result.state).toBe('stale_broker_review')
    expect(result.summary).toContain('broker review is stale')
  })

  it('rejects datasets whose symbol provenance cannot be verified', () => {
    const result = summarizeExecutionChartSurface(input({
      dataId: 'import_batch_42',
      datasetLabel: 'Imported prices',
      datasetSymbol: '',
    }))

    expect(result.state).toBe('dataset_provenance_unknown')
    expect(result.summary).toContain('no verified symbol provenance')
    expect(result.markerCount).toBe(0)
  })

  it('retries a transient chart-module import failure on the next sync', async () => {
    chartModuleImportState.attempts = 0
    chartModuleImportState.failuresRemaining = 1
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    const first = await syncExecutionChart(host, input())
    const second = await syncExecutionChart(host, input())

    expect(first.state).toBe('broker_review_loaded_chart_unavailable')
    expect(first.summary).toContain('chart runtime could not be created')
    expect(second.state).toBe('loaded_no_markers')
    expect(second.barCount).toBe(2)
    expect(chartModuleImportState.attempts).toBe(2)
    expect(createChartMock).toHaveBeenCalledTimes(1)
  })

  it('caches price bars only when an explicit dataset revision is supplied', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_price_bars') {
        return bars() as never
      }
      return [] as never
    })
    const host = document.createElement('div')
    document.body.appendChild(host)

    const chartInput = input({
      datasetRevision: 'rev-1',
      orderRows: [{ cells: ['2025-04-12 23:20:30', 'BUY', 'filled', '1', '1', '182.10', 'ord-1'] }],
    })

    const first = await syncExecutionChart(host, chartInput)
    const second = await syncExecutionChart(host, chartInput)

    expect(first.state).toBe('loaded_broker_markers')
    expect(second.state).toBe('loaded_broker_markers')
    expect(invokeCommandMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenCalled()
    expect(setMarkersMock).toHaveBeenCalled()
  })

  it('refetches the same data id when its revision changes', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock
      .mockResolvedValueOnce(bars(180.5) as never)
      .mockResolvedValueOnce(bars(190.5) as never)
    const host = document.createElement('div')

    await syncExecutionChart(host, input({ datasetRevision: 'rev-1' }))
    await syncExecutionChart(host, input({ datasetRevision: 'rev-2' }))

    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
    expect(setDataMock).toHaveBeenLastCalledWith(
      expect.arrayContaining([expect.objectContaining({ close: 190.5 })]),
    )
  })

  it('does not persistently cache unrevisioned datasets', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock.mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    await syncExecutionChart(host, input())
    await syncExecutionChart(host, input())

    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
  })

  it('invalidates a revisioned cache entry explicitly', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock.mockResolvedValue(bars() as never)
    const host = document.createElement('div')
    const chartInput = input({ datasetRevision: 'rev-1' })

    await syncExecutionChart(host, chartInput)
    invalidateExecutionPriceBarsCache('AAPL')
    await syncExecutionChart(host, chartInput)

    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
  })

  it('contains both price-bar command failures as an unavailable chart result', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock.mockRejectedValue(new Error('backend unavailable'))
    const host = document.createElement('div')

    const result = await syncExecutionChart(host, input())

    expect(result.state).toBe('broker_review_loaded_chart_unavailable')
    expect(result.summary).toContain('price bars could not be loaded')
    expect(result.barCount).toBe(0)
    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
    expect(createChartMock).not.toHaveBeenCalled()
  })

  it('tears down the prior host before a replacement-host fetch failure returns', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock
      .mockResolvedValueOnce(bars() as never)
      .mockRejectedValueOnce(new Error('replacement unavailable'))
      .mockRejectedValueOnce(new Error('replacement unavailable'))
    const firstHost = document.createElement('div')
    const replacementHost = document.createElement('div')

    await syncExecutionChart(firstHost, input())
    const result = await syncExecutionChart(replacementHost, input())

    expect(result.state).toBe('broker_review_loaded_chart_unavailable')
    expect(result.summary).toContain('price bars could not be loaded')
    expect(createChartMock).toHaveBeenCalledTimes(1)
    expect(removeMock).toHaveBeenCalledTimes(1)
    expect(resizeObserveMock).toHaveBeenCalledWith(firstHost)
    expect(resizeUnobserveMock).toHaveBeenCalledWith(firstHost)
    expect(resizeDisconnectMock).toHaveBeenCalledTimes(1)

    const themeCallsAfterTeardown = applyOptionsMock.mock.calls.length
    document.documentElement.dataset.theme = 'light'
    await Promise.resolve()
    expect(applyOptionsMock).toHaveBeenCalledTimes(themeCallsAfterTeardown)
  })

  it('tears down the prior host before an empty replacement dataset returns', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock
      .mockResolvedValueOnce(bars() as never)
      .mockResolvedValueOnce([] as never)
    const firstHost = document.createElement('div')
    const replacementHost = document.createElement('div')

    await syncExecutionChart(firstHost, input())
    const result = await syncExecutionChart(replacementHost, input())

    expect(result.state).toBe('broker_review_loaded_chart_unavailable')
    expect(result.summary).toContain('no price bars loaded')
    expect(createChartMock).toHaveBeenCalledTimes(1)
    expect(removeMock).toHaveBeenCalledTimes(1)
    expect(resizeUnobserveMock).toHaveBeenCalledWith(firstHost)
    expect(resizeDisconnectMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenCalledTimes(1)
  })

  it('preserves candles but reports the marker overlay as unavailable when the plugin fails', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    createSeriesMarkersMock.mockImplementationOnce(() => {
      throw new Error('marker plugin unavailable')
    })
    const host = document.createElement('div')

    const result = await syncExecutionChart(host, input({
      orderRows: [
        { cells: ['2025-04-12 23:20:30', 'BUY', 'filled', '1', '1', '180.00', 'inside'] },
      ],
    }))

    expect(result.state).toBe('loaded_marker_overlay_unavailable')
    expect(result.summary).toContain('marker runtime did not initialize')
    expect(result.markerCount).toBe(0)
    expect(result.barCount).toBe(2)
    expect(setMarkersMock).not.toHaveBeenCalled()
    expect(setDataMock).toHaveBeenCalledTimes(1)
    expect(fitContentMock).toHaveBeenCalledTimes(1)
    expect(removeMock).not.toHaveBeenCalled()
    expect(resizeObserveMock).toHaveBeenCalledWith(host)
    expect(resizeUnobserveMock).not.toHaveBeenCalled()
    expect(resizeDisconnectMock).not.toHaveBeenCalled()
  })

  it('preserves rendered candles when the marker runtime rejects an update', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    setMarkersMock.mockImplementationOnce(() => {
      throw new Error('marker render failed')
    })
    const host = document.createElement('div')

    const result = await syncExecutionChart(host, input({
      orderRows: [
        { cells: ['2025-04-12 23:20:30', 'BUY', 'filled', '1', '1', '180.00', 'inside'] },
      ],
    }))

    expect(result.state).toBe('loaded_marker_overlay_unavailable')
    expect(result.summary).toContain('marker runtime rejected the marker update')
    expect(result.markerCount).toBe(0)
    expect(result.barCount).toBe(2)
    expect(setMarkersMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenLastCalledWith([
      expect.objectContaining({ time: 1_744_500_000, close: 180.8 }),
      expect.objectContaining({ time: 1_744_500_060, close: 181.0 }),
    ])
    expect(fitContentMock).toHaveBeenCalledTimes(1)
    expect(removeMock).not.toHaveBeenCalled()
    expect(resizeObserveMock).toHaveBeenCalledWith(host)
    expect(resizeUnobserveMock).not.toHaveBeenCalled()
    expect(resizeDisconnectMock).not.toHaveBeenCalled()

    const markerCallsAfterFailure = setMarkersMock.mock.calls.length
    document.documentElement.dataset.theme = 'light'
    await Promise.resolve()
    expect(setMarkersMock).toHaveBeenCalledTimes(markerCallsAfterFailure)
  })

  it('omits markers outside the loaded dataset instead of clamping them', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    const result = await syncExecutionChart(host, input({
      orderRows: [
        { cells: ['2025-04-12 23:19:00', 'BUY', 'filled', '1', '1', '179.00', 'before'] },
        { cells: ['2025-04-12 23:20:30', 'BUY', 'filled', '1', '1', '180.00', 'inside'] },
        { cells: ['2025-04-12 23:22:00', 'SELL', 'filled', '1', '1', '181.00', 'after'] },
      ],
    }))

    expect(result.state).toBe('loaded_broker_markers')
    expect(result.markerCount).toBe(1)
    expect(result.summary).toContain('2 outside the loaded dataset range')
    expect(setMarkersMock).toHaveBeenLastCalledWith([
      expect.objectContaining({ id: 'inside', time: 1_744_500_000 }),
    ])
  })

  it('renders broker-confirmed position, stop, target, and limit price levels', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    const result = await syncExecutionChart(host, input({
      positionReview: {
        state: 'open',
        symbol: 'AAPL',
        qty: 2,
        side: 'long',
        avgEntryPrice: 180.25,
        currentPrice: 181,
        marketValue: 362,
        unrealizedPl: 1.5,
        unrealizedPlpc: 0.00416,
      },
      orderRows: [
        {
          cells: [
            '2025-04-12 23:20:30',
            'SELL',
            'accepted',
            '2',
            '0',
            '-',
            'stop-1',
            'stop 176.5',
            'stop_loss · stop_loss_first',
            'parent-1',
          ],
        },
        {
          cells: [
            '2025-04-12 23:20:30',
            'SELL',
            'new',
            '2',
            '0',
            '-',
            'target-1',
            'limit 188',
            'take_profit · stop_loss_first',
            'parent-1',
          ],
        },
        {
          cells: [
            '2025-04-12 23:20:30',
            'BUY',
            'held',
            '1',
            '0',
            '-',
            'limit-1',
            'limit 179',
            '-',
            '-',
          ],
        },
      ],
    }))

    expect(result.referenceLines).toEqual([
      expect.objectContaining({ kind: 'average_entry', price: 180.25 }),
      expect.objectContaining({ kind: 'current_price', price: 181 }),
      expect.objectContaining({ kind: 'stop', price: 176.5 }),
      expect.objectContaining({ kind: 'take_profit', price: 188 }),
      expect.objectContaining({ kind: 'limit', price: 179 }),
    ])
    expect(result.summary).toContain('5 broker-confirmed price levels displayed')
    expect(result.referenceLineIssue).toBeUndefined()
    expect(createPriceLineMock).toHaveBeenCalledTimes(5)
    expect(createPriceLineMock).toHaveBeenCalledWith(expect.objectContaining({
      price: 180.25,
      title: 'AVG ENTRY 180.25',
      lineStyle: 0,
      lineWidth: 2,
      axisLabelVisible: true,
    }))
    expect(createPriceLineMock).toHaveBeenCalledWith(expect.objectContaining({
      price: 176.5,
      title: 'STOP LOSS 176.5',
      lineStyle: 2,
    }))
    expect(createPriceLineMock).toHaveBeenCalledWith(expect.objectContaining({
      price: 188,
      title: 'TAKE PROFIT 188',
      lineStyle: 2,
    }))
  })

  it('caps broker price levels at ten while reporting the exact omitted count', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')
    const orderRows = Array.from({ length: 12 }, (_, index) => ({
      cells: [
        '2025-04-12 23:20:30',
        'SELL',
        'accepted',
        '1',
        '0',
        '-',
        `limit-${index + 1}`,
        `limit ${170 + index}`,
        '-',
        '-',
      ],
    }))

    const result = await syncExecutionChart(host, input({ orderRows }))

    expect(createPriceLineMock).toHaveBeenCalledTimes(10)
    expect(result.referenceLines).toHaveLength(10)
    expect(result.referenceLineOmittedCount).toBe(2)
    expect(result.referenceLineIssue).toBe(
      '2 additional broker price levels were omitted because the chart displays at most 10.',
    )
    expect(result.summary).toContain('10 broker-confirmed price levels displayed')
  })

  it('differentiates known order lifecycles and preserves an explicit other fallback', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')
    const palette = lightweightChartSeriesPalette('dark')

    const result = await syncExecutionChart(host, input({
      orderRows: [
        { cells: ['2025-04-12 23:20:30', 'BUY', 'filled', '1', '1', '180', 'filled-1', '-', '-', '-'] },
        { cells: ['2025-04-12 23:20:30', 'SELL', 'partially_filled', '2', '1', '181', 'partial-1', '-', '-', '-'] },
        { cells: ['2025-04-12 23:20:30', 'BUY', 'accepted', '1', '0', '-', 'working-1', '-', '-', '-'] },
        { cells: ['2025-04-12 23:20:30', 'SELL', 'canceled', '1', '0', '-', 'closed-1', '-', '-', '-'] },
        { cells: ['2025-04-12 23:20:30', 'SELL', 'done_for_day', '1', '0', '-', 'closed-2', '-', '-', '-'] },
        { cells: ['2025-04-12 23:20:30', 'BUY', 'broker_specific', '1', '0', '-', 'other-1', '-', '-', '-'] },
      ],
    }))

    expect(result.markerBreakdown).toEqual({
      filled: 1,
      partial: 1,
      working: 1,
      closed: 2,
      other: 1,
    })
    expect(setMarkersMock).toHaveBeenLastCalledWith([
      expect.objectContaining({
        id: 'filled-1',
        shape: 'arrowUp',
        position: 'belowBar',
        text: 'BUY · FILLED',
        color: palette.positive,
      }),
      expect.objectContaining({
        id: 'partial-1',
        shape: 'arrowDown',
        position: 'aboveBar',
        text: 'SELL · PARTIAL',
        color: palette.negative,
      }),
      expect.objectContaining({
        id: 'working-1',
        shape: 'circle',
        text: 'BUY · ACCEPTED',
        color: palette.highlight,
      }),
      expect.objectContaining({
        id: 'closed-1',
        shape: 'square',
        text: 'SELL · CANCELED',
        color: palette.neutral,
      }),
      expect.objectContaining({
        id: 'closed-2',
        shape: 'square',
        text: 'SELL · DONE FOR DAY',
        color: palette.neutral,
      }),
      expect.objectContaining({
        id: 'other-1',
        shape: 'circle',
        text: 'BUY · BROKER SPECIFIC',
        color: palette.positive,
      }),
    ])
  })

  it('keeps dense protective-order marker labels compact and role-specific', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    await syncExecutionChart(host, input({
      orderRows: [
        { cells: ['2025-04-12 23:20:30', 'SELL', 'accepted', '1', '0', '-', 'stop-1', 'stop 176.5', 'stop_loss · stop_loss_first', 'parent-1'] },
        { cells: ['2025-04-12 23:20:30', 'SELL', 'new', '1', '0', '-', 'target-1', 'limit 188', 'take_profit · stop_loss_first', 'parent-1'] },
      ],
    }))

    expect(setMarkersMock).toHaveBeenLastCalledWith([
      expect.objectContaining({ id: 'stop-1', text: 'SL · ACCEPTED' }),
      expect.objectContaining({ id: 'target-1', text: 'TP · NEW' }),
    ])
  })

  it('rethemes and removes broker price levels with the chart lifecycle', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    await syncExecutionChart(host, input({
      positionReview: {
        state: 'open',
        symbol: 'AAPL',
        qty: 1,
        side: 'long',
        avgEntryPrice: 180.25,
        currentPrice: 181,
        marketValue: 181,
        unrealizedPl: 0.75,
        unrealizedPlpc: 0.00416,
      },
    }))
    document.documentElement.dataset.theme = 'light'

    await vi.waitFor(() => {
      expect(priceLineApplyOptionsMock).toHaveBeenCalledWith(expect.objectContaining({
        price: 180.25,
        color: lightweightChartSeriesPalette('light').fast,
      }))
      expect(priceLineApplyOptionsMock).toHaveBeenCalledWith(expect.objectContaining({
        price: 181,
        color: lightweightChartSeriesPalette('light').highlight,
      }))
    })

    disposeExecutionChart()
    expect(removePriceLineMock).toHaveBeenCalledTimes(2)
  })

  it('lets only the latest overlapping dataset sync render', async () => {
    const a = deferred<unknown>()
    const b = deferred<unknown>()
    vi.mocked(invokeCommand).mockImplementation(async (_command, args) => {
      const record = args as Record<string, unknown>
      const dataId = String(record.dataId ?? record.data_id ?? '')
      return await (dataId === 'AAPL' ? a.promise : b.promise) as never
    })
    const host = document.createElement('div')

    const first = syncExecutionChart(host, input({
      dataId: 'AAPL',
      datasetLabel: 'AAPL',
      datasetSymbol: 'AAPL',
      symbol: 'AAPL',
      datasetRevision: 'a-1',
    }))
    const second = syncExecutionChart(host, input({
      dataId: 'MSFT',
      datasetLabel: 'MSFT',
      datasetSymbol: 'MSFT',
      symbol: 'MSFT',
      datasetRevision: 'm-1',
    }))
    b.resolve(bars(320.5))
    const secondResult = await second
    a.resolve(bars(180.5))
    await first

    expect(secondResult.symbol).toBe('MSFT')
    expect(createChartMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenLastCalledWith(
      expect.arrayContaining([expect.objectContaining({ close: 320.5 })]),
    )
  })

  it('invalidates an in-flight sync when the chart is disposed', async () => {
    const pendingBars = deferred<unknown>()
    vi.mocked(invokeCommand).mockImplementation(async () => await pendingBars.promise as never)
    const host = document.createElement('div')

    const syncing = syncExecutionChart(host, input({ datasetRevision: 'rev-1' }))
    disposeExecutionChart()
    pendingBars.resolve(bars())
    await syncing

    expect(createChartMock).not.toHaveBeenCalled()
    expect(setDataMock).not.toHaveBeenCalled()
  })

  it('reacts to root theme changes and stops observing after disposal', async () => {
    vi.mocked(invokeCommand).mockResolvedValue(bars() as never)
    const host = document.createElement('div')

    await syncExecutionChart(host, input({
      orderRows: [
        { cells: ['2025-04-12 23:20:30', 'BUY', 'filled', '1', '1', '180.00', 'inside'] },
      ],
    }))
    document.documentElement.dataset.theme = 'light'
    const light = lightweightChartSeriesPalette('light')

    await vi.waitFor(() => {
      expect(applyOptionsMock).toHaveBeenCalledWith(expect.objectContaining({
        layout: expect.objectContaining({
          background: { type: 'solid', color: 'rgb(255, 253, 250)' },
          attributionLogo: true,
        }),
      }))
      expect(seriesApplyOptionsMock).toHaveBeenCalledWith(expect.objectContaining({
        upColor: light.positive,
        downColor: light.negative,
      }))
      expect(setMarkersMock).toHaveBeenLastCalledWith([
        expect.objectContaining({
          id: 'inside',
          time: 1_744_500_000,
          color: light.positive,
        }),
      ])
    })

    disposeExecutionChart()
    const callsAfterDispose = applyOptionsMock.mock.calls.length
    const markerCallsAfterDispose = setMarkersMock.mock.calls.length
    document.documentElement.dataset.theme = 'dark'
    await Promise.resolve()
    expect(applyOptionsMock).toHaveBeenCalledTimes(callsAfterDispose)
    expect(setMarkersMock).toHaveBeenCalledTimes(markerCallsAfterDispose)
  })
})
