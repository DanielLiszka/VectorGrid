import { invokeCommand } from '../tauri/invoke'
import { formatUtcDateTimeFromUnixMs } from '../utils/uiFormat'
import type { ExecutionOrderRow } from './executionService'

export type ExecutionChartSurfaceState =
  | 'no_symbol'
  | 'no_dataset'
  | 'symbol_mismatch'
  | 'provenance_suppressed'
  | 'stale_broker_review'
  | 'loaded_no_markers'
  | 'loaded_broker_markers'
  | 'broker_review_loaded_chart_unavailable'

export interface ExecutionChartSurfaceInput {
  dataId: string
  datasetLabel: string
  datasetSymbol: string
  symbol: string
  brokerReviewLoaded: boolean
  fetchedAtUnixMs: number | null
  staleThresholdMs: number
  orderRows: ExecutionOrderRow[]
}

export interface ExecutionChartSurfaceResult {
  state: ExecutionChartSurfaceState
  summary: string
  symbol: string
  datasetId: string
  datasetLabel: string
  datasetSymbol: string
  fetchedAtUnixMs: number | null
  markerCount: number
}

interface ExecutionPriceBar {
  time: number
  open: number
  high: number
  low: number
  close: number
}

interface ExecutionMarkerRow {
  id: string
  atUnixMs: number
  side: string
  status: string
  label: string
}

interface LightweightChartModule {
  createChart?: (host: HTMLElement, options: Record<string, unknown>) => any
  CandlestickSeries?: unknown
  createSeriesMarkers?: (series: any, markers?: unknown[]) => { setMarkers?: (markers: unknown[]) => void } | null
}

const PRICE_BARS_CACHE_MAX = 8

const priceBarsCache = new Map<string, ExecutionPriceBar[]>()
const priceBarsPending = new Map<string, Promise<ExecutionPriceBar[]>>()

let chartModulePromise: Promise<LightweightChartModule | null> | null = null

const chartRuntime: {
  host: HTMLElement | null
  chart: any | null
  candleSeries: any | null
  markerApi: { setMarkers?: (markers: unknown[]) => void } | null
  resizeObserver: ResizeObserver | null
} = {
  host: null,
  chart: null,
  candleSeries: null,
  markerApi: null,
  resizeObserver: null,
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function canonicalSymbol(raw: unknown): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, '')
    .toUpperCase()
}

function toNumber(value: unknown): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : Number.NaN
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value)
}

function basename(raw: string): string {
  const normalized = String(raw || '').trim().replace(/\\/g, '/')
  if (!normalized) return ''
  const parts = normalized.split('/')
  return String(parts[parts.length - 1] || '').trim()
}

function looksLikeTradingSymbol(raw: unknown): boolean {
  const symbol = canonicalSymbol(raw)
  return /^[A-Z][A-Z0-9./-]{0,23}$/.test(symbol)
}

function inferDatasetSymbol(dataId: string, datasetLabel: string): string {
  const candidates = [
    text(dataId),
    basename(text(dataId)),
    text(datasetLabel),
    basename(text(datasetLabel)),
  ]
  for (const candidate of candidates) {
    const withoutExt = candidate.replace(/\.[A-Za-z0-9]+$/, '')
    if (looksLikeTradingSymbol(withoutExt)) return canonicalSymbol(withoutExt)
  }
  return ''
}

function parseOrderRowUnixMs(row: ExecutionOrderRow): number | null {
  const raw = text(row.cells?.[0])
  if (!raw || raw === '-') return null
  const parsed = Date.parse(raw.replace(' ', 'T') + 'Z')
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null
}

function buildBrokerMarkerRows(orderRows: ExecutionOrderRow[]): ExecutionMarkerRow[] {
  const out: ExecutionMarkerRow[] = []
  const seen = new Set<string>()
  for (const row of orderRows) {
    const status = text(row.cells?.[2]).toLowerCase()
    if (!status || status.endsWith('*')) continue
    const atUnixMs = parseOrderRowUnixMs(row)
    if (!atUnixMs) continue
    const side = text(row.cells?.[1]).toLowerCase()
    const orderId = text(row.cells?.[6]) || `${side}-${status}-${atUnixMs}`
    const dedupeKey = `${orderId}|${atUnixMs}|${status}`
    if (seen.has(dedupeKey)) continue
    seen.add(dedupeKey)
    out.push({
      id: orderId,
      atUnixMs,
      side,
      status,
      label: `${side || 'order'} ${status}`,
    })
  }
  out.sort((a, b) => a.atUnixMs - b.atUnixMs)
  return out
}

export function summarizeExecutionChartSurface(
  input: ExecutionChartSurfaceInput,
): ExecutionChartSurfaceResult {
  const symbol = canonicalSymbol(input.symbol)
  const datasetId = text(input.dataId)
  const datasetLabel = text(input.datasetLabel) || datasetId || '-'
  const datasetSymbol = canonicalSymbol(input.datasetSymbol || inferDatasetSymbol(datasetId, datasetLabel))
  const brokerMarkers = buildBrokerMarkerRows(input.orderRows)
  if (!symbol) {
    return {
      state: 'no_symbol',
      summary: 'Execution chart unavailable: select a symbol first.',
      symbol: '',
      datasetId,
      datasetLabel,
      datasetSymbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: 0,
    }
  }
  if (!datasetId) {
    return {
      state: 'no_dataset',
      summary: 'Execution chart unavailable: load a workflow dataset first.',
      symbol,
      datasetId: '',
      datasetLabel: '-',
      datasetSymbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: 0,
    }
  }
  if (datasetSymbol && datasetSymbol !== symbol) {
    return {
      state: 'symbol_mismatch',
      summary: `Execution chart unavailable: loaded dataset ${datasetLabel || datasetId || '-'} does not match ${symbol}.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: 0,
    }
  }
  if (!input.brokerReviewLoaded) {
    return {
      state: 'provenance_suppressed',
      summary: `Execution chart loaded for ${symbol}. Broker-confirmed markers stay suppressed until a symbol-scoped broker review is refreshed.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol,
      fetchedAtUnixMs: null,
      markerCount: 0,
    }
  }
  const stale =
    Number.isFinite(input.fetchedAtUnixMs ?? Number.NaN) &&
    input.fetchedAtUnixMs != null &&
    Date.now() - input.fetchedAtUnixMs > input.staleThresholdMs
  if (stale) {
    return {
      state: 'stale_broker_review',
      summary: `Execution chart loaded for ${symbol}, but broker review is stale as of ${formatUtcDateTimeFromUnixMs(input.fetchedAtUnixMs)}.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: brokerMarkers.length,
    }
  }
  if (!brokerMarkers.length) {
    return {
      state: 'loaded_no_markers',
      summary: `Execution chart loaded for ${symbol}. No broker-confirmed order markers are present in the current review.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: 0,
    }
  }
  return {
    state: 'loaded_broker_markers',
    summary: `Execution chart loaded for ${symbol} with ${brokerMarkers.length} broker-confirmed order marker${brokerMarkers.length === 1 ? '' : 's'}.`,
    symbol,
    datasetId,
    datasetLabel,
    datasetSymbol,
    fetchedAtUnixMs: input.fetchedAtUnixMs,
    markerCount: brokerMarkers.length,
  }
}

function normalizeChartTimeSec(raw: unknown): number {
  const value = toNumber(raw)
  if (!Number.isFinite(value)) return Number.NaN
  const abs = Math.abs(value)
  if (abs >= 1e17) return Math.floor(value / 1e9)
  if (abs >= 1e14) return Math.floor(value / 1e6)
  if (abs >= 1e11) return Math.floor(value / 1e3)
  return Math.floor(value)
}

function normalizeExecutionPriceBars(raw: unknown): ExecutionPriceBar[] {
  const rows = Array.isArray(raw) ? raw : []
  const out: ExecutionPriceBar[] = []
  for (const rowRaw of rows) {
    const row = isRecord(rowRaw) ? rowRaw : null
    if (!row) continue
    const time = normalizeChartTimeSec(row.time ?? row.timestamp ?? row.ts ?? row.t)
    const open = toNumber(row.open ?? row.o)
    const highRaw = toNumber(row.high ?? row.h)
    const lowRaw = toNumber(row.low ?? row.l)
    const close = toNumber(row.close ?? row.c)
    if (!Number.isFinite(time) || !Number.isFinite(open) || !Number.isFinite(highRaw) || !Number.isFinite(lowRaw) || !Number.isFinite(close)) {
      continue
    }
    out.push({
      time,
      open,
      high: Math.max(highRaw, open, close),
      low: Math.min(lowRaw, open, close),
      close,
    })
  }
  out.sort((a, b) => a.time - b.time)
  const deduped: ExecutionPriceBar[] = []
  for (const row of out) {
    const last = deduped[deduped.length - 1]
    if (last && last.time === row.time) {
      deduped[deduped.length - 1] = row
    } else {
      deduped.push(row)
    }
  }
  return deduped
}

async function cacheGetOrLoad<T>(
  cache: Map<string, T>,
  pending: Map<string, Promise<T>>,
  key: string,
  loader: () => Promise<T>,
  maxEntries: number,
): Promise<T> {
  if (cache.has(key)) {
    const value = cache.get(key) as T
    cache.delete(key)
    cache.set(key, value)
    return value
  }
  const active = pending.get(key)
  if (active) return active
  const promise = loader()
    .then((value) => {
      pending.delete(key)
      cache.set(key, value)
      while (cache.size > maxEntries) {
        const oldest = cache.keys().next()
        if (oldest.done) break
        cache.delete(oldest.value)
      }
      return value
    })
    .catch((error) => {
      pending.delete(key)
      throw error
    })
  pending.set(key, promise)
  return promise
}

async function fetchExecutionPriceBars(dataId: string): Promise<ExecutionPriceBar[]> {
  const id = text(dataId)
  if (!id) return []
  return cacheGetOrLoad(
    priceBarsCache,
    priceBarsPending,
    id,
    async () => {
      let raw: unknown
      try {
        raw = await invokeCommand('get_price_bars', { dataId: id, maxPoints: 8000 })
      } catch (firstError) {
        try {
          raw = await invokeCommand('get_price_bars', { data_id: id, max_points: 8000 })
        } catch {
          throw firstError
        }
      }
      return normalizeExecutionPriceBars(raw)
    },
    PRICE_BARS_CACHE_MAX,
  )
}

function mapMarkerTimeToBarTime(atUnixMs: number, bars: ExecutionPriceBar[]): number | null {
  if (!bars.length) return null
  const target = Math.floor(atUnixMs / 1000)
  let lo = 0
  let hi = bars.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const bar = bars[mid]
    if (!bar) break
    if (target < bar.time) hi = mid - 1
    else if (target > bar.time) lo = mid + 1
    else return bar.time
  }
  if (hi < 0) return bars[0]?.time ?? null
  if (lo >= bars.length) return bars[bars.length - 1]?.time ?? null
  const left = bars[hi]
  const right = bars[lo]
  if (!left || !right) return null
  return Math.abs(target - left.time) <= Math.abs(right.time - target) ? left.time : right.time
}

function buildSeriesMarkers(rows: ExecutionOrderRow[], bars: ExecutionPriceBar[]): unknown[] {
  const brokerMarkers = buildBrokerMarkerRows(rows)
  const out: Array<Record<string, unknown>> = []
  for (const marker of brokerMarkers) {
    const time = mapMarkerTimeToBarTime(marker.atUnixMs, bars)
    if (!Number.isFinite(time ?? Number.NaN)) continue
    const lowerSide = marker.side.toLowerCase()
    const isBuy = lowerSide === 'buy'
    const isSell = lowerSide === 'sell'
    out.push({
      id: marker.id,
      time,
      position: isBuy ? 'belowBar' : isSell ? 'aboveBar' : 'inBar',
      shape: isBuy ? 'arrowUp' : isSell ? 'arrowDown' : 'circle',
      color: isBuy ? 'rgba(22, 163, 74, 0.95)' : isSell ? 'rgba(225, 29, 72, 0.95)' : 'rgba(148, 163, 184, 0.9)',
      text: marker.label,
    })
  }
  return out
}

async function loadChartModule(): Promise<LightweightChartModule | null> {
  if (!chartModulePromise) {
    chartModulePromise = import('lightweight-charts')
      .then((module) => module as LightweightChartModule)
      .catch(() => null)
  }
  return chartModulePromise
}

function clearRuntimeData(): void {
  if (chartRuntime.candleSeries && typeof chartRuntime.candleSeries.setData === 'function') {
    try {
      chartRuntime.candleSeries.setData([])
    } catch {
      // chart already disposed
    }
  }
  if (chartRuntime.markerApi && typeof chartRuntime.markerApi.setMarkers === 'function') {
    try {
      chartRuntime.markerApi.setMarkers([])
    } catch {
      // chart already disposed
    }
  }
}

export function disposeExecutionChart(): void {
  if (chartRuntime.resizeObserver && chartRuntime.host) {
    try {
      chartRuntime.resizeObserver.unobserve(chartRuntime.host)
    } catch {
      // host already detached
    }
  }
  chartRuntime.resizeObserver = null
  if (chartRuntime.chart && typeof chartRuntime.chart.remove === 'function') {
    try {
      chartRuntime.chart.remove()
    } catch {
      // chart already removed
    }
  }
  chartRuntime.host = null
  chartRuntime.chart = null
  chartRuntime.candleSeries = null
  chartRuntime.markerApi = null
}

async function ensureExecutionChart(host: HTMLElement): Promise<boolean> {
  if (chartRuntime.chart && chartRuntime.host === host && chartRuntime.candleSeries) return true
  if (chartRuntime.chart && chartRuntime.host !== host) disposeExecutionChart()
  const chartModule = await loadChartModule()
  if (!chartModule?.createChart) return false
  const width = Math.max(240, Math.floor(host.clientWidth || 820))
  const height = Math.max(220, Math.floor(host.clientHeight || 360))
  let chart: any = null
  try {
    chart = chartModule.createChart(host, {
      width,
      height,
      layout: {
        textColor: 'rgba(203, 213, 225, 0.92)',
        background: { type: 'solid', color: 'rgba(2, 6, 23, 0)' },
      },
      grid: {
        vertLines: { color: 'rgba(51, 65, 85, 0.35)' },
        horzLines: { color: 'rgba(51, 65, 85, 0.35)' },
      },
      rightPriceScale: { borderColor: 'rgba(100, 116, 139, 0.45)' },
      timeScale: {
        borderColor: 'rgba(100, 116, 139, 0.45)',
        fixLeftEdge: false,
        fixRightEdge: false,
      },
      crosshair: {
        vertLine: { color: 'rgba(148, 163, 184, 0.45)' },
        horzLine: { color: 'rgba(148, 163, 184, 0.45)' },
      },
    })
  } catch {
    return false
  }
  const chartAny = chart as any
  let candleSeries: any = null
  if (typeof chartAny.addSeries === 'function' && chartModule.CandlestickSeries) {
    try {
      candleSeries = chartAny.addSeries(chartModule.CandlestickSeries, {
        upColor: 'rgba(22, 163, 74, 0.9)',
        downColor: 'rgba(225, 29, 72, 0.9)',
        wickUpColor: 'rgba(22, 163, 74, 0.9)',
        wickDownColor: 'rgba(225, 29, 72, 0.9)',
        borderVisible: true,
        borderUpColor: 'rgba(22, 163, 74, 1)',
        borderDownColor: 'rgba(225, 29, 72, 1)',
      })
    } catch {
      candleSeries = null
    }
  }
  if (!candleSeries && typeof chartAny.addCandlestickSeries === 'function') {
    try {
      candleSeries = chartAny.addCandlestickSeries({
        upColor: 'rgba(22, 163, 74, 0.9)',
        downColor: 'rgba(225, 29, 72, 0.9)',
        wickUpColor: 'rgba(22, 163, 74, 0.9)',
        wickDownColor: 'rgba(225, 29, 72, 0.9)',
        borderVisible: true,
        borderUpColor: 'rgba(22, 163, 74, 1)',
        borderDownColor: 'rgba(225, 29, 72, 1)',
      })
    } catch {
      candleSeries = null
    }
  }
  if (!candleSeries) {
    if (typeof chartAny.remove === 'function') chartAny.remove()
    return false
  }
  const markerApi =
    chartModule.createSeriesMarkers && candleSeries
      ? chartModule.createSeriesMarkers(candleSeries, [])
      : null

  chartRuntime.host = host
  chartRuntime.chart = chartAny
  chartRuntime.candleSeries = candleSeries
  chartRuntime.markerApi = markerApi
  if (typeof ResizeObserver !== 'undefined') {
    chartRuntime.resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0]
      const chartRef = chartRuntime.chart
      if (!entry || !chartRef || typeof chartRef.applyOptions !== 'function') return
      const width = Math.max(240, Math.floor(entry.contentRect.width))
      const height = Math.max(220, Math.floor(entry.contentRect.height))
      try {
        chartRef.applyOptions({ width, height })
      } catch {
        // chart already disposed
      }
    })
    chartRuntime.resizeObserver.observe(host)
  }
  return true
}

export async function syncExecutionChart(
  host: HTMLElement | null,
  input: ExecutionChartSurfaceInput,
): Promise<ExecutionChartSurfaceResult> {
  const summary = summarizeExecutionChartSurface(input)
  if (!host) return summary
  if (summary.state === 'no_symbol' || summary.state === 'no_dataset' || summary.state === 'symbol_mismatch') {
    clearRuntimeData()
    return summary
  }
  const bars = await fetchExecutionPriceBars(summary.datasetId)
  if (!bars.length) {
    clearRuntimeData()
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: no price bars loaded for ${summary.datasetLabel || summary.datasetId || summary.symbol}.`,
      markerCount: 0,
    }
  }
  const ready = await ensureExecutionChart(host)
  if (!ready || !chartRuntime.candleSeries) {
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: chart runtime could not be created for ${summary.symbol || summary.datasetLabel || summary.datasetId || '-'}.`,
      markerCount: 0,
    }
  }
  try {
    chartRuntime.candleSeries.setData(
      bars.map((bar) => ({
        time: bar.time,
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
      })),
    )
    if (chartRuntime.markerApi && typeof chartRuntime.markerApi.setMarkers === 'function') {
      chartRuntime.markerApi.setMarkers(buildSeriesMarkers(input.orderRows, bars))
    }
    if (chartRuntime.chart?.timeScale && typeof chartRuntime.chart.timeScale === 'function') {
      const timeScale = chartRuntime.chart.timeScale()
      if (timeScale && typeof timeScale.fitContent === 'function') {
        timeScale.fitContent()
      }
    }
  } catch {
    clearRuntimeData()
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: render failed for ${summary.symbol || '-'} on ${summary.datasetLabel || summary.datasetId || '-'}.`,
      markerCount: 0,
    }
  }
  return summary
}

