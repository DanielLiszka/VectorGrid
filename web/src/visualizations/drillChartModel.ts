import {
  normalizeCompositeIndicatorBars,
  normalizeMovingAverageIndicatorBars,
  type IndexedPriceBar,
} from './priceSeriesModel'

export type DrillChartPriceBar = IndexedPriceBar

export interface DrillCandlePoint {
  time: number
  open: number
  high: number
  low: number
  close: number
}

export interface DrillLinePoint {
  time: number
  value: number
}

export interface DrillHistogramPoint extends DrillLinePoint {
  color?: string
}

export interface DrillTradeMarker {
  id: string
  time: number
  position: 'belowBar' | 'aboveBar' | 'inBar'
  shape: 'arrowUp' | 'arrowDown' | 'circle'
  color: string
  size?: number
  text?: string
}

export interface DrillTradeTableCell {
  text: string
  className: string
  title: string
}

export interface DrillTradeTableRow {
  tradeIdx: number
  selected: boolean
  cells: DrillTradeTableCell[]
}

export interface DrillTradeTableModel {
  visible: boolean
  statsText: string
  rows: DrillTradeTableRow[]
  start: number
  total: number
}

export interface DrillChartModel {
  bars: DrillChartPriceBar[]
  candleData: DrillCandlePoint[]
  fastData: DrillLinePoint[]
  slowData: DrillLinePoint[]
  halftrendData: DrillLinePoint[]
  maczData: DrillHistogramPoint[]
  trades: Record<string, unknown>[]
  tradeStats: Record<string, unknown>
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : {}
}

function num(value: unknown, fallback = Number.NaN): number {
  const next = Number(value)
  return Number.isFinite(next) ? next : fallback
}

export function buildDrillChartModel(
  curvesRaw: unknown,
  barsRaw: DrillChartPriceBar[] | null | undefined,
): DrillChartModel {
  const bars = Array.isArray(barsRaw)
    ? barsRaw.filter(
        (bar) =>
          Number.isFinite(bar.time) &&
          Number.isFinite(bar.open) &&
          Number.isFinite(bar.high) &&
          Number.isFinite(bar.low) &&
          Number.isFinite(bar.close),
      )
    : []
  const curves = asRecord(curvesRaw)
  const indicatorBars = normalizeMovingAverageIndicatorBars(curves.indicator_bars)
  const fastData: DrillLinePoint[] = []
  const slowData: DrillLinePoint[] = []
  for (const row of indicatorBars) {
    if (Number.isFinite(row.fast ?? Number.NaN)) {
      fastData.push({ time: row.time, value: Number(row.fast) })
    }
    if (Number.isFinite(row.slow ?? Number.NaN)) {
      slowData.push({ time: row.time, value: Number(row.slow) })
    }
  }

  const compositeBars = normalizeCompositeIndicatorBars(curves.composite_indicator_bars)
  const halftrendData: DrillLinePoint[] = []
  const maczData: DrillHistogramPoint[] = []
  for (const row of compositeBars) {
    if (Number.isFinite(row.halftrend ?? Number.NaN)) {
      halftrendData.push({ time: row.time, value: Number(row.halftrend) })
    }
    if (Number.isFinite(row.macz ?? Number.NaN)) {
      const value = Number(row.macz)
      maczData.push({
        time: row.time,
        value,
        color: value >= 0 ? 'rgba(34, 197, 94, 0.75)' : 'rgba(244, 63, 94, 0.75)',
      })
    }
  }

  return {
    bars,
    candleData: bars.map((bar) => ({
      time: bar.time,
      open: bar.open,
      high: bar.high,
      low: bar.low,
      close: bar.close,
    })),
    fastData,
    slowData,
    halftrendData,
    maczData,
    trades: Array.isArray(curves.trades) ? curves.trades.map((trade) => asRecord(trade)) : [],
    tradeStats: asRecord(curves.trade_stats),
  }
}

export function mapTradeIndexToChartTime(
  indexRaw: unknown,
  barsRaw: DrillChartPriceBar[] | null | undefined,
): number | null {
  const bars = Array.isArray(barsRaw) ? barsRaw : []
  if (!bars.length) return null
  const x = Math.max(0, Math.floor(num(indexRaw, 0)))
  let monotonic = true
  let prevFrom = Number.NEGATIVE_INFINITY
  for (const bar of bars) {
    if (bar.fromIndex < prevFrom || bar.toIndex < bar.fromIndex) {
      monotonic = false
      break
    }
    prevFrom = bar.fromIndex
  }
  if (!monotonic) {
    let bestTime = bars[0]?.time ?? null
    let bestDist = Number.POSITIVE_INFINITY
    for (const bar of bars) {
      if (x >= bar.fromIndex && x <= bar.toIndex) return bar.time
      const distance = x < bar.fromIndex ? bar.fromIndex - x : x - bar.toIndex
      if (distance < bestDist) {
        bestDist = distance
        bestTime = bar.time
      }
    }
    return bestTime
  }

  let lo = 0
  let hi = bars.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const bar = bars[mid]
    if (!bar) break
    if (x < bar.fromIndex) hi = mid - 1
    else if (x > bar.toIndex) lo = mid + 1
    else return bar.time
  }
  if (hi < 0) return bars[0]?.time ?? null
  if (lo >= bars.length) return bars[bars.length - 1]?.time ?? null
  const left = bars[hi]
  const right = bars[lo]
  if (!left || !right) return null
  return Math.abs(x - left.toIndex) <= Math.abs(right.fromIndex - x) ? left.time : right.time
}

export function buildDrillTradeMarkers(
  chartModel: DrillChartModel,
  selectedTradeIdx = -1,
): DrillTradeMarker[] {
  if (!chartModel.trades.length) return []
  const markers: DrillTradeMarker[] = []
  for (let tradeIdx = 0; tradeIdx < chartModel.trades.length; tradeIdx += 1) {
    const trade = chartModel.trades[tradeIdx] || {}
    const direction = num(trade.direction, 0)
    const isLong = direction > 0
    const isShort = direction < 0
    if (!isLong && !isShort) continue
    const selected = tradeIdx === selectedTradeIdx
    const entryTime = mapTradeIndexToChartTime(trade.entry_t, chartModel.bars)
    if (Number.isFinite(entryTime ?? Number.NaN)) {
      markers.push({
        id: `trade-${tradeIdx}-entry`,
        time: entryTime as number,
        position: isLong ? 'belowBar' : 'aboveBar',
        shape: isLong ? 'arrowUp' : 'arrowDown',
        color: selected
          ? isLong
            ? 'rgba(34, 197, 94, 1)'
            : 'rgba(248, 113, 113, 1)'
          : isLong
            ? 'rgba(22, 163, 74, 0.95)'
            : 'rgba(225, 29, 72, 0.95)',
        size: selected ? 1.4 : 1,
        text: `${isLong ? 'L' : 'S'}${tradeIdx + 1}`,
      })
    }

    const open = !!trade.open
    const exitT = num(trade.exit_t, Number.NaN)
    const exitTime =
      !open && Number.isFinite(exitT)
        ? mapTradeIndexToChartTime(exitT, chartModel.bars)
        : null
    if (Number.isFinite(exitTime ?? Number.NaN)) {
      markers.push({
        id: `trade-${tradeIdx}-exit`,
        time: exitTime as number,
        position: 'inBar',
        shape: 'circle',
        color: selected ? 'rgba(250, 204, 21, 0.98)' : 'rgba(148, 163, 184, 0.9)',
        size: selected ? 1.2 : 0.8,
      })
    }
  }
  return markers
}

export function formatUtcTimeFromSec(secRaw: unknown): string {
  const sec = num(secRaw, Number.NaN)
  if (!Number.isFinite(sec)) return '-'
  const date = new Date(sec * 1000)
  const pad = (value: number): string => String(Math.max(0, Math.floor(value))).padStart(2, '0')
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())}:${pad(date.getUTCSeconds())} UTC`
}

export function formatUtcTimeShortFromSec(secRaw: unknown): string {
  const sec = num(secRaw, Number.NaN)
  if (!Number.isFinite(sec)) return '-'
  const date = new Date(sec * 1000)
  const pad = (value: number): string => String(Math.max(0, Math.floor(value))).padStart(2, '0')
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())}`
}

export function computeTradeTableStart(
  totalRaw: unknown,
  selectedRaw: unknown,
  maxRowsRaw: unknown,
): number {
  const total = Math.max(0, Math.floor(num(totalRaw, 0)))
  const maxRows = Math.max(1, Math.floor(num(maxRowsRaw, 200)))
  if (!Number.isFinite(total) || total <= maxRows) return 0
  const maxStart = Math.max(0, total - maxRows)
  const selected = Math.floor(num(selectedRaw, Number.NaN))
  if (Number.isFinite(selected) && selected >= 0 && selected < total) {
    const half = Math.floor(maxRows / 2)
    return Math.max(0, Math.min(maxStart, selected - half))
  }
  return maxStart
}

export function buildDrillTradeTableModel(options: {
  chartModel: DrillChartModel
  selectedTradeIdx: number
  rangeStart: number
  maxRows?: number
}): DrillTradeTableModel {
  const { chartModel, selectedTradeIdx } = options
  const trades = chartModel.trades
  const stats = chartModel.tradeStats
  if (!trades.length && !Object.keys(stats).length) {
    return { visible: false, statsText: '-', rows: [], start: 0, total: 0 }
  }

  const maxRows = Math.max(1, Math.floor(num(options.maxRows, 200)))
  const total = trades.length
  let start = Math.max(0, Math.floor(num(options.rangeStart, 0)))
  if (total <= maxRows) {
    start = 0
  } else {
    const maxStart = Math.max(0, total - maxRows)
    start = Math.max(0, Math.min(maxStart, start))
    if (selectedTradeIdx >= 0 && selectedTradeIdx < total) {
      const endExcl = start + maxRows
      if (selectedTradeIdx < start || selectedTradeIdx >= endExcl) {
        start = computeTradeTableStart(total, selectedTradeIdx, maxRows)
      }
    } else if (start === 0) {
      start = maxStart
    }
  }

  let statsText = statsTextForModel(stats, trades.length)
  const shown = total ? trades.slice(start, Math.min(total, start + maxRows)) : []
  if (total) {
    statsText = `${statsText} | view=${start + 1}..${start + shown.length}/${total}`
  }

  return {
    visible: true,
    statsText,
    rows: shown.map((trade, i) => tradeTableRow(trade, start + i, selectedTradeIdx, chartModel.bars)),
    start,
    total,
  }
}

function statsTextForModel(stats: Record<string, unknown>, tradeCount: number): string {
  if (!Object.keys(stats).length) return `trades=${tradeCount}`
  const closed = Math.floor(num(stats.closed_trades, 0))
  const open = Math.floor(num(stats.open_trades, 0))
  const wins = Math.floor(num(stats.wins, 0))
  const losses = Math.floor(num(stats.losses, 0))
  const winRate = num(stats.win_rate, Number.NaN)
  const avgWin = num(stats.avg_win, Number.NaN)
  const avgLoss = num(stats.avg_loss, Number.NaN)
  const pf = num(stats.profit_factor, Number.NaN)
  const exp = num(stats.expectancy, Number.NaN)
  const pfStr = Number.isFinite(pf) ? (pf === Infinity ? 'inf' : pf.toFixed(3)) : '-'
  const wrStr = Number.isFinite(winRate) ? `${(winRate * 100).toFixed(1)}%` : '-'
  const avgWinStr = Number.isFinite(avgWin) ? `${(avgWin * 100).toFixed(2)}%` : '-'
  const avgLossStr = Number.isFinite(avgLoss) ? `${(avgLoss * 100).toFixed(2)}%` : '-'
  const expStr = Number.isFinite(exp) ? `${(exp * 100).toFixed(2)}%` : '-'
  return `closed=${closed} open=${open} wins=${wins} losses=${losses} win_rate=${wrStr} avg_win=${avgWinStr} avg_loss=${avgLossStr} pf=${pfStr} expectancy=${expStr}`
}

function tradeTableRow(
  trade: Record<string, unknown>,
  tradeIdx: number,
  selectedTradeIdx: number,
  bars: DrillChartPriceBar[],
): DrillTradeTableRow {
  const idxLabel = tradeIdx + 1
  const dirVal = num(trade.direction, 0)
  const dir = dirVal > 0 ? 'Long' : dirVal < 0 ? 'Short' : 'Flat'
  const entry = Math.floor(num(trade.entry_t, 0))
  const tradeBars = Math.floor(num(trade.bars, 0))
  const pnl = num(trade.pnl, Number.NaN)
  const open = !!trade.open
  const exitRaw = num(trade.exit_t, Number.NaN)
  const exit = open && (!Number.isFinite(exitRaw) || exitRaw <= 0) ? Number.NaN : exitRaw
  const entryTime = mapTradeIndexToChartTime(entry, bars)
  const exitTime = Number.isFinite(exit) ? mapTradeIndexToChartTime(exit, bars) : null

  return {
    tradeIdx,
    selected: tradeIdx === selectedTradeIdx,
    cells: [
      { text: String(idxLabel), className: 'left mono', title: '' },
      { text: dir, className: 'left mono', title: '' },
      {
        text: String(entry),
        className: 'left mono',
        title: Number.isFinite(entryTime ?? Number.NaN) ? formatUtcTimeFromSec(entryTime) : '',
      },
      {
        text: Number.isFinite(entryTime ?? Number.NaN) ? formatUtcTimeShortFromSec(entryTime) : '-',
        className: 'left mono',
        title: Number.isFinite(entryTime ?? Number.NaN) ? formatUtcTimeFromSec(entryTime) : '',
      },
      {
        text: Number.isFinite(exit) ? String(Math.floor(exit)) : '-',
        className: 'left mono',
        title: Number.isFinite(exitTime ?? Number.NaN) ? formatUtcTimeFromSec(exitTime) : '',
      },
      {
        text: Number.isFinite(exitTime ?? Number.NaN) ? formatUtcTimeShortFromSec(exitTime) : '-',
        className: 'left mono',
        title: Number.isFinite(exitTime ?? Number.NaN) ? formatUtcTimeFromSec(exitTime) : '',
      },
      { text: String(tradeBars), className: 'mono', title: '' },
      {
        text: Number.isFinite(pnl) ? `${(pnl * 100).toFixed(2)}%` : '-',
        className: 'mono',
        title: '',
      },
      { text: open ? 'open' : 'closed', className: 'left mono', title: '' },
    ],
  }
}
