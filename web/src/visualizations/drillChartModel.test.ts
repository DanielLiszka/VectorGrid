import { describe, expect, it } from 'vitest'
import {
  buildDrillChartModel,
  buildDrillTradeMarkers,
  buildDrillTradeTableModel,
  mapTradeIndexToChartTime,
  type DrillChartPriceBar,
} from './drillChartModel'
import { lightweightChartSeriesPalette } from './chartTheme'

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
  composite_indicator_bars: [
    { time: 1_744_500_000, halftrend: 0, macz_hist: 0.2, ott_line: 99.8 },
    { time: 1_744_500_060, halftrend: 1, macz_hist: -0.1, ott_line: 100.0 },
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
    expect(model.halftrendData).toEqual([
      { time: 1_744_500_000, value: 1 },
      { time: 1_744_500_060, value: -1 },
    ])
    expect(model.ottData).toHaveLength(2)
    expect(model.maczData).toHaveLength(2)

    const markers = buildDrillTradeMarkers(model, 1, 'light')
    const palette = lightweightChartSeriesPalette('light')
    expect(markers.some((marker) => marker.id === 'trade-0-entry' && marker.text === 'L1')).toBe(true)
    expect(markers.some((marker) => marker.id === 'trade-0-exit' && marker.text === 'X1')).toBe(true)
    expect(markers.find((marker) => marker.id === 'trade-1-entry')?.size).toBe(1.4)
    expect(markers.find((marker) => marker.id === 'trade-1-entry')?.color).toBe(palette.highlight)
    expect(markers.find((marker) => marker.id === 'trade-0-entry')?.color).toBe(palette.positive)
  })

  it('attaches native strategy indicators using their own aligned timestamps', () => {
    const model = buildDrillChartModel({
      ...curves,
      strategy_indicator_bundle: {
        strategy_kind: 'Vemr',
        aggregation: 'last_finite',
        bars: [
          { time: 1_744_400_000, from_index: 0, to_index: 3 },
          { time: 1_744_400_240, from_index: 4, to_index: 7 },
        ],
        series: [
          {
            id: 'vemr_bb_upper',
            label: 'Bollinger upper',
            kind: 'line',
            pane_id: 'price',
            scale_id: 'price',
            unit: 'price',
            default_visible: true,
            values: [102.5, null],
          },
          {
            id: 'vemr_atr',
            label: 'Average true range',
            kind: 'line',
            pane_id: 'vemr_volatility',
            scale_id: 'vemr_atr',
            unit: 'price_delta',
            default_visible: false,
            values: [null, null],
          },
        ],
      },
    }, bars)

    expect(model.strategyIndicators).toMatchObject({
      strategyKind: 'Vemr',
      aggregation: 'last_finite',
      bars: [
        { time: 1_744_400_000, fromIndex: 0, toIndex: 3 },
        { time: 1_744_400_240, fromIndex: 4, toIndex: 7 },
      ],
    })
    expect(model.strategyIndicators?.series[0]).toMatchObject({
      id: 'vemr_bb_upper',
      data: [
        { time: 1_744_400_000, value: 102.5 },
        { time: 1_744_400_240 },
      ],
      finitePointCount: 1,
    })
    expect(model.strategyIndicators?.series[1]).toMatchObject({
      id: 'vemr_atr',
      defaultVisible: false,
      finitePointCount: 0,
      data: [{ time: 1_744_400_000 }, { time: 1_744_400_240 }],
    })
    expect(model.candleData).toHaveLength(4)
    expect(model.fastData).toHaveLength(2)
    expect(model.halftrendData).toHaveLength(2)
    expect(model.trades).toHaveLength(2)
  })

  it('isolates malformed native indicators from the legacy chart model', () => {
    const model = buildDrillChartModel({
      ...curves,
      strategyIndicatorBundle: {
        strategyKind: 'Vemr',
        aggregation: 'last_finite',
        bars: [{ time: 1_744_400_000, fromIndex: 0, toIndex: 3 }],
        series: [
          {
            id: 'broken',
            label: 'Broken',
            kind: 'unsupported',
            paneId: 'price',
            scaleId: 'price',
            unit: 'price',
            defaultVisible: true,
            values: [102.5],
          },
        ],
      },
    }, bars)

    expect(model.strategyIndicators).toBeNull()
    expect(model.candleData).toHaveLength(4)
    expect(model.fastData).toHaveLength(2)
    expect(model.slowData).toHaveLength(2)
    expect(model.ottData).toHaveLength(2)
    expect(model.maczData).toHaveLength(2)
    expect(model.trades).toHaveLength(2)
    expect(model.tradeStats.closed_trades).toBe(2)
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
