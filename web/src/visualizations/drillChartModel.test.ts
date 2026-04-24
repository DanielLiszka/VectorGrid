import { describe, expect, it } from 'vitest'
import {
  buildDrillChartModel,
  buildDrillTradeMarkers,
  buildDrillTradeTableModel,
  mapTradeIndexToChartTime,
  type DrillChartPriceBar,
} from './drillChartModel'

const bars: DrillChartPriceBar[] = [
  { time: 1_744_500_000, open: 100, high: 101, low: 99.8, close: 100.6, fromIndex: 1, toIndex: 1, volume: 0 },
  { time: 1_744_500_060, open: 100.6, high: 101.2, low: 100.3, close: 101.0, fromIndex: 2, toIndex: 2, volume: 0 },
  { time: 1_744_500_120, open: 101.0, high: 101.1, low: 100.2, close: 100.4, fromIndex: 3, toIndex: 3, volume: 0 },
  { time: 1_744_500_180, open: 100.4, high: 100.8, low: 99.9, close: 100.0, fromIndex: 4, toIndex: 4, volume: 0 },
]

const curves = {
  trades: [
    { entry_t: 1, exit_t: 2, direction: 1, pnl: 0.03, bars: 1, open: false },
    { entry_t: 3, exit_t: 4, direction: -1, pnl: -0.01, bars: 1, open: false },
  ],
  trade_stats: {
    closed_trades: 2,
    open_trades: 0,
    wins: 1,
    losses: 1,
    win_rate: 0.5,
    avg_win: 0.03,
    avg_loss: -0.01,
    profit_factor: 3,
    expectancy: 0.01,
  },
  indicator_bars: [
    { time: 1_744_500_000, from_index: 1, to_index: 1, fast: 100.2, slow: 100.0 },
    { time: 1_744_500_060, from_index: 2, to_index: 2, fast: 100.5, slow: 100.1 },
  ],
}

describe('drillChartModel', () => {
  it('maps trade indexes to chart times for monotonic and sparse bars', () => {
    expect(mapTradeIndexToChartTime(3, bars)).toBe(1_744_500_120)
    expect(mapTradeIndexToChartTime(99, bars)).toBe(1_744_500_180)
    expect(mapTradeIndexToChartTime(0, bars)).toBe(1_744_500_000)
  })

  it('builds chart series and selected trade markers from drill curves', () => {
    const model = buildDrillChartModel(curves, bars)
    expect(model.candleData).toHaveLength(4)
    expect(model.fastData).toHaveLength(2)

    const markers = buildDrillTradeMarkers(model, 1)
    expect(markers.some((marker) => marker.id === 'trade-0-entry' && marker.text === 'L1')).toBe(true)
    expect(markers.some((marker) => marker.id === 'trade-0-exit')).toBe(true)
    expect(markers.find((marker) => marker.id === 'trade-1-entry')?.size).toBe(1.4)
  })

  it('builds the trade table model without touching the DOM', () => {
    const model = buildDrillChartModel(curves, bars)
    const table = buildDrillTradeTableModel({
      chartModel: model,
      selectedTradeIdx: 1,
      rangeStart: 0,
      maxRows: 200,
    })

    expect(table.visible).toBe(true)
    expect(table.statsText).toContain('closed=2')
    expect(table.rows).toHaveLength(2)
    expect(table.rows[1]?.selected).toBe(true)
    expect(table.rows[0]?.cells[3]?.text).toBe('2025-04-12 23:20')
  })
})
