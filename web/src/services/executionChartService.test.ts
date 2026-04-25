

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  disposeExecutionChart,
  summarizeExecutionChartSurface,
  syncExecutionChart,
} from './executionChartService'

const createChartMock = vi.fn()
const addSeriesMock = vi.fn()
const setDataMock = vi.fn()
const fitContentMock = vi.fn()
const removeMock = vi.fn()
const setMarkersMock = vi.fn()
const createSeriesMarkersMock = vi.fn()

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => []),
}))

vi.mock('lightweight-charts', () => ({
  CandlestickSeries: {},
  createChart: (...args: unknown[]) => createChartMock(...args),
  createSeriesMarkers: (...args: unknown[]) => createSeriesMarkersMock(...args),
}))

describe('executionChartService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    setDataMock.mockReset()
    fitContentMock.mockReset()
    removeMock.mockReset()
    setMarkersMock.mockReset()
    addSeriesMock.mockImplementation(() => ({
      setData: setDataMock,
    }))
    createChartMock.mockImplementation(() => ({
      addSeries: addSeriesMock,
      timeScale: () => ({ fitContent: fitContentMock }),
      applyOptions: vi.fn(),
      remove: removeMock,
    }))
    createSeriesMarkersMock.mockImplementation(() => ({
      setMarkers: setMarkersMock,
    }))
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
      },
    )
  })

  afterEach(() => {
    disposeExecutionChart()
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

  it('caches price bars by data id across repeated chart syncs', async () => {
    const invokeCommandMock = vi.mocked(invokeCommand)
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_price_bars') {
        return [
          { time: 1_744_500_000, open: 180, high: 181, low: 179.5, close: 180.8 },
          { time: 1_744_500_060, open: 180.8, high: 181.2, low: 180.4, close: 181.0 },
        ] as never
      }
      return [] as never
    })
    const host = document.createElement('div')
    document.body.appendChild(host)

    const input = {
      dataId: 'AAPL',
      datasetLabel: 'AAPL',
      datasetSymbol: 'AAPL',
      symbol: 'AAPL',
      brokerReviewLoaded: true,
      fetchedAtUnixMs: Date.now(),
      staleThresholdMs: 90_000,
      orderRows: [{ cells: ['2026-04-12 06:00:00', 'BUY', 'filled', '1', '1', '182.10', 'ord-1'] }],
    }

    const first = await syncExecutionChart(host, input)
    const second = await syncExecutionChart(host, input)

    expect(first.state).toBe('loaded_broker_markers')
    expect(second.state).toBe('loaded_broker_markers')
    expect(invokeCommandMock).toHaveBeenCalledTimes(1)
    expect(setDataMock).toHaveBeenCalled()
    expect(setMarkersMock).toHaveBeenCalled()
  })
})
