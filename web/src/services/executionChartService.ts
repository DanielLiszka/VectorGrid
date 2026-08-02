import { invokeCommand } from '../tauri/invoke'
import { formatUtcDateTimeFromUnixMs } from '../utils/uiFormat'
import {
  applyLightweightChartSize,
  createCandlestickRuntime,
  fitLightweightChartContent,
  observeLightweightChartTheme,
  panLightweightChart,
  type LightweightChartModule,
  visibleLogicalRangeLabel,
} from '../visualizations/lightweightChartRuntime'
import {
  lightweightChartSeriesColor,
  resolveLightweightChartTheme,
  type LightweightChartSeriesTone,
  type LightweightChartTheme,
} from '../visualizations/chartTheme'
import { normalizePriceBars, type PriceBar } from '../visualizations/priceSeriesModel'
import type {
  ExecutionOrderRow,
  ExecutionPositionReview,
} from './executionService'

export type ExecutionChartSurfaceState =
  | 'no_symbol'
  | 'no_dataset'
  | 'dataset_provenance_unknown'
  | 'symbol_mismatch'
  | 'provenance_suppressed'
  | 'stale_broker_review'
  | 'loaded_no_markers'
  | 'loaded_broker_markers'
  | 'loaded_marker_overlay_unavailable'
  | 'broker_review_loaded_chart_unavailable'

export interface ExecutionChartSurfaceInput {
  dataId: string
  datasetLabel: string
  datasetSymbol: string
  datasetRevision?: string | number | null
  symbol: string
  brokerReviewLoaded: boolean
  fetchedAtUnixMs: number | null
  staleThresholdMs: number
  orderRows: ExecutionOrderRow[]
  positionReview?: ExecutionPositionReview | null
}

export type ExecutionChartReferenceKind =
  | 'average_entry'
  | 'current_price'
  | 'stop'
  | 'take_profit'
  | 'limit'

export interface ExecutionChartReferenceLineSummary {
  id: string
  kind: ExecutionChartReferenceKind
  label: string
  price: number
}

export interface ExecutionChartMarkerBreakdown {
  filled: number
  partial: number
  working: number
  closed: number
  other: number
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
  barCount: number
  visibleRangeLabel: string
  referenceLines?: ExecutionChartReferenceLineSummary[]
  referenceLineIssue?: string
  referenceLineOmittedCount?: number
  markerBreakdown?: ExecutionChartMarkerBreakdown
}

type ExecutionPriceBar = PriceBar

interface ExecutionMarkerRow {
  id: string
  atUnixMs: number
  side: string
  status: string
  label: string
}

type ExecutionMarkerLifecycle =
  | 'filled'
  | 'partial'
  | 'working'
  | 'closed'
  | 'other'

interface ExecutionSeriesMarker {
  id: string
  time: number
  position: 'belowBar' | 'aboveBar' | 'inBar'
  shape: 'arrowUp' | 'arrowDown' | 'circle' | 'square'
  tone: LightweightChartSeriesTone
  text: string
  lifecycle: ExecutionMarkerLifecycle
}

interface ExecutionReferenceLineRow extends ExecutionChartReferenceLineSummary {
  tone: LightweightChartSeriesTone
  lineStyle: number
  lineWidth: number
}

interface ExecutionReferenceLineRuntime {
  row: ExecutionReferenceLineRow
  api: any
}

const PRICE_BARS_CACHE_MAX = 8
const EXECUTION_REFERENCE_LINE_LIMIT = 10

interface PriceBarsCacheEntry {
  dataId: string
  bars: ExecutionPriceBar[]
}

interface PriceBarsPendingEntry {
  dataId: string
  promise: Promise<ExecutionPriceBar[]>
}

const priceBarsCache = new Map<string, PriceBarsCacheEntry>()
const priceBarsPending = new Map<string, PriceBarsPendingEntry>()
let priceBarsCacheGeneration = 0

let chartModulePromise: Promise<LightweightChartModule | null> | null = null
let chartLifecycleGeneration = 0
let chartSyncGeneration = 0
let chartCreationHost: HTMLElement | null = null
let chartCreationPromise: Promise<boolean> | null = null

const chartRuntime: {
  host: HTMLElement | null
  chart: any | null
  candleSeries: any | null
  markerApi: { setMarkers?: (markers: unknown[]) => void } | null
  markerRows: ExecutionSeriesMarker[]
  referenceLines: ExecutionReferenceLineRuntime[]
  resizeObserver: ResizeObserver | null
  stopThemeObserver: (() => void) | null
} = {
  host: null,
  chart: null,
  candleSeries: null,
  markerApi: null,
  markerRows: [],
  referenceLines: [],
  resizeObserver: null,
  stopThemeObserver: null,
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

function basename(raw: string): string {
  const normalized = String(raw || '').trim().replace(/\\/g, '/')
  if (!normalized) return ''
  const parts = normalized.split('/')
  return String(parts[parts.length - 1] || '').trim()
}

function looksLikeTradingSymbol(raw: unknown): boolean {
  if (/\s/u.test(text(raw))) return false
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

const BROKER_WORKING_ORDER_STATUSES = new Set([
  'new',
  'accepted',
  'pending_new',
  'accepted_for_bidding',
  'pending_replace',
  'pending_cancel',
  'held',
  'stopped',
  'calculated',
  'suspended',
])

const BROKER_CLOSED_ORDER_STATUSES = new Set([
  'canceled',
  'cancelled',
  'done_for_day',
  'expired',
  'rejected',
  'replaced',
])

const BROKER_ACTIVE_ORDER_STATUSES = new Set([
  ...BROKER_WORKING_ORDER_STATUSES,
  'partially_filled',
])

function markerLifecycle(status: string): ExecutionMarkerLifecycle {
  const normalized = text(status).toLowerCase()
  if (normalized === 'filled') return 'filled'
  if (normalized === 'partially_filled') return 'partial'
  if (BROKER_WORKING_ORDER_STATUSES.has(normalized)) return 'working'
  if (BROKER_CLOSED_ORDER_STATUSES.has(normalized)) return 'closed'
  return 'other'
}

function readableStatus(status: string): string {
  return text(status).replace(/_/g, ' ').toUpperCase() || 'ORDER'
}

function markerLabel(
  side: string,
  status: string,
  bracket: string,
): string {
  const bracketRole = text(bracket)
    .split('·')[0]
    ?.trim()
    .toLowerCase()
    .replace(/\s+/gu, '_')
  const role = bracketRole === 'stop_loss'
    ? 'SL'
    : bracketRole === 'take_profit'
      ? 'TP'
      : text(side).toUpperCase() || 'ORDER'
  const lifecycle = text(status).toLowerCase() === 'partially_filled'
    ? 'PARTIAL'
    : readableStatus(status)
  return `${role} · ${lifecycle}`
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
    const bracket = text(row.cells?.[8])
    const dedupeKey = `${orderId}|${atUnixMs}|${status}`
    if (seen.has(dedupeKey)) continue
    seen.add(dedupeKey)
    out.push({
      id: orderId,
      atUnixMs,
      side,
      status,
      label: markerLabel(side, status, bracket),
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
      barCount: 0,
      visibleRangeLabel: '',
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
      barCount: 0,
      visibleRangeLabel: '',
    }
  }
  if (!datasetSymbol) {
    return {
      state: 'dataset_provenance_unknown',
      summary: `Execution chart unavailable: loaded dataset ${datasetLabel || datasetId || '-'} has no verified symbol provenance for ${symbol}.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol: '',
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: 0,
      barCount: 0,
      visibleRangeLabel: '',
    }
  }
  if (datasetSymbol !== symbol) {
    return {
      state: 'symbol_mismatch',
      summary: `Execution chart unavailable: loaded dataset ${datasetLabel || datasetId || '-'} does not match ${symbol}.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: 0,
      barCount: 0,
      visibleRangeLabel: '',
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
      barCount: 0,
      visibleRangeLabel: '',
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
      barCount: 0,
      visibleRangeLabel: '',
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
      barCount: 0,
      visibleRangeLabel: '',
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
    barCount: 0,
    visibleRangeLabel: '',
  }
}

function normalizeDatasetRevision(raw: unknown): string {
  if (typeof raw === 'number') return Number.isFinite(raw) ? String(raw) : ''
  return text(raw)
}

async function loadExecutionPriceBars(dataId: string): Promise<ExecutionPriceBar[]> {
  let raw: unknown
  try {
    raw = await invokeCommand('get_price_bars', { dataId, maxPoints: 8000 })
  } catch (firstError) {
    try {
      raw = await invokeCommand('get_price_bars', { data_id: dataId, max_points: 8000 })
    } catch {
      throw firstError
    }
  }
  return normalizePriceBars(raw)
}

async function fetchExecutionPriceBars(
  dataId: string,
  datasetRevision?: string | number | null,
): Promise<ExecutionPriceBar[]> {
  const id = text(dataId)
  if (!id) return []
  const revision = normalizeDatasetRevision(datasetRevision)
  const cacheable = !!revision
  const key = JSON.stringify([cacheable ? 'revision' : 'transient', id, revision])
  const cached = cacheable ? priceBarsCache.get(key) : null
  if (cached) {
    priceBarsCache.delete(key)
    priceBarsCache.set(key, cached)
    return cached.bars
  }

  const active = priceBarsPending.get(key)
  if (active) return active.promise

  const generation = priceBarsCacheGeneration
  const promise = loadExecutionPriceBars(id)
    .then((bars) => {
      if (cacheable && generation === priceBarsCacheGeneration) {
        priceBarsCache.set(key, { dataId: id, bars })
        while (priceBarsCache.size > PRICE_BARS_CACHE_MAX) {
          const oldest = priceBarsCache.keys().next()
          if (oldest.done) break
          priceBarsCache.delete(oldest.value)
        }
      }
      return bars
    })
    .finally(() => {
      if (priceBarsPending.get(key)?.promise === promise) {
        priceBarsPending.delete(key)
      }
    })
  priceBarsPending.set(key, { dataId: id, promise })
  return promise
}

export function invalidateExecutionPriceBarsCache(dataId?: string): void {
  const id = text(dataId)
  priceBarsCacheGeneration += 1
  if (!id) {
    priceBarsCache.clear()
    priceBarsPending.clear()
    return
  }
  for (const [key, entry] of priceBarsCache) {
    if (entry.dataId === id) priceBarsCache.delete(key)
  }
  for (const [key, entry] of priceBarsPending) {
    if (entry.dataId === id) priceBarsPending.delete(key)
  }
}

function mapMarkerTimeToBarTime(atUnixMs: number, bars: ExecutionPriceBar[]): number | null {
  if (!bars.length) return null
  const target = Math.floor(atUnixMs / 1000)
  const firstTime = bars[0]?.time
  const lastTime = bars[bars.length - 1]?.time
  if (
    !Number.isFinite(firstTime ?? Number.NaN) ||
    !Number.isFinite(lastTime ?? Number.NaN) ||
    target < (firstTime as number) ||
    target > (lastTime as number)
  ) {
    return null
  }
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
  if (hi < 0 || lo >= bars.length) return null
  const left = bars[hi]
  const right = bars[lo]
  if (!left || !right) return null
  return Math.abs(target - left.time) <= Math.abs(right.time - target) ? left.time : right.time
}

function buildSeriesMarkerRows(
  rows: ExecutionOrderRow[],
  bars: ExecutionPriceBar[],
): ExecutionSeriesMarker[] {
  const brokerMarkers = buildBrokerMarkerRows(rows)
  const out: ExecutionSeriesMarker[] = []
  for (const marker of brokerMarkers) {
    const time = mapMarkerTimeToBarTime(marker.atUnixMs, bars)
    if (!Number.isFinite(time ?? Number.NaN)) continue
    const lowerSide = marker.side.toLowerCase()
    const isBuy = lowerSide === 'buy'
    const isSell = lowerSide === 'sell'
    const lifecycle = markerLifecycle(marker.status)
    const isFilled = lifecycle === 'filled'
    const isPartial = lifecycle === 'partial'
    const isWorking = lifecycle === 'working'
    const tone: LightweightChartSeriesTone = isWorking
      ? 'highlight'
      : lifecycle === 'closed'
        ? (marker.status === 'rejected' ? 'negative' : 'neutral')
        : isBuy
          ? 'positive'
          : isSell
            ? 'negative'
            : 'neutral'
    out.push({
      id: marker.id,
      time: time as number,
      position: isBuy ? 'belowBar' : isSell ? 'aboveBar' : 'inBar',
      shape:
        isFilled || isPartial
          ? (isBuy ? 'arrowUp' : isSell ? 'arrowDown' : 'circle')
          : lifecycle === 'closed'
            ? 'square'
            : 'circle',
      tone,
      text: marker.label,
      lifecycle,
    })
  }
  return out
}

function emptyMarkerBreakdown(): ExecutionChartMarkerBreakdown {
  return {
    filled: 0,
    partial: 0,
    working: 0,
    closed: 0,
    other: 0,
  }
}

function markerBreakdown(
  markers: readonly ExecutionSeriesMarker[],
): ExecutionChartMarkerBreakdown {
  const breakdown = emptyMarkerBreakdown()
  for (const marker of markers) breakdown[marker.lifecycle] += 1
  return breakdown
}

function colorizeExecutionSeriesMarkers(
  markers: readonly ExecutionSeriesMarker[],
  theme: LightweightChartTheme,
): unknown[] {
  return markers.map(({ tone, lifecycle: _lifecycle, ...marker }) => ({
    ...marker,
    color: lightweightChartSeriesColor(theme, tone),
  }))
}

function finitePositivePrice(raw: unknown): number | null {
  const value = Number(raw)
  return Number.isFinite(value) && value > 0 ? value : null
}

function formattedReferencePrice(price: number): string {
  if (Math.abs(price) >= 1000) return price.toFixed(2)
  if (Math.abs(price) >= 1) return price.toFixed(4).replace(/0+$/u, '').replace(/\.$/u, '')
  return price.toFixed(6).replace(/0+$/u, '').replace(/\.$/u, '')
}

function parseTriggerPrice(triggerRaw: unknown): {
  type: 'stop' | 'limit'
  price: number
} | null {
  const trigger = text(triggerRaw).toLowerCase()
  const match = trigger.match(/\b(stop|limit)\s+([-+]?(?:\d+\.?\d*|\.\d+))/u)
  if (!match) return null
  const price = finitePositivePrice(match[2])
  if (!price) return null
  return {
    type: match[1] === 'stop' ? 'stop' : 'limit',
    price,
  }
}

function addReferenceLine(
  rows: ExecutionReferenceLineRow[],
  seen: Set<string>,
  row: ExecutionReferenceLineRow,
): void {
  const key = `${row.kind}|${row.price.toPrecision(12)}`
  if (seen.has(key)) return
  seen.add(key)
  rows.push(row)
}

function buildExecutionReferenceLines(
  input: ExecutionChartSurfaceInput,
): ExecutionReferenceLineRow[] {
  if (!input.brokerReviewLoaded) return []
  const rows: ExecutionReferenceLineRow[] = []
  const seen = new Set<string>()
  const position = input.positionReview
  const positionSymbol = canonicalSymbol(position?.symbol)
  const selectedSymbol = canonicalSymbol(input.symbol)
  if (
    position?.state === 'open' &&
    (!positionSymbol || positionSymbol === selectedSymbol)
  ) {
    const averageEntry = finitePositivePrice(position.avgEntryPrice)
    if (averageEntry) {
      addReferenceLine(rows, seen, {
        id: `average-entry-${averageEntry}`,
        kind: 'average_entry',
        label: `AVG ENTRY ${formattedReferencePrice(averageEntry)}`,
        price: averageEntry,
        tone: 'fast',
        lineStyle: 0,
        lineWidth: 2,
      })
    }
    const currentPrice = finitePositivePrice(position.currentPrice)
    if (currentPrice) {
      addReferenceLine(rows, seen, {
        id: `current-price-${currentPrice}`,
        kind: 'current_price',
        label: `BROKER MARK ${formattedReferencePrice(currentPrice)}`,
        price: currentPrice,
        tone: 'highlight',
        lineStyle: 1,
        lineWidth: 2,
      })
    }
  }

  for (const order of input.orderRows) {
    const status = text(order.cells?.[2]).toLowerCase()
    if (
      !BROKER_ACTIVE_ORDER_STATUSES.has(status) ||
      status.endsWith('*')
    ) {
      continue
    }
    const parsedTrigger = parseTriggerPrice(order.cells?.[7])
    if (!parsedTrigger) continue
    const bracket = text(order.cells?.[8]).toLowerCase()
    const isTakeProfit =
      parsedTrigger.type === 'limit' &&
      (bracket.includes('take_profit') || bracket.includes('take profit'))
    const isStopLoss =
      parsedTrigger.type === 'stop' &&
      (bracket.includes('stop_loss') || bracket.includes('stop loss'))
    const kind: ExecutionChartReferenceKind = isTakeProfit
      ? 'take_profit'
      : parsedTrigger.type
    const shortLabel = isTakeProfit
      ? 'TAKE PROFIT'
      : isStopLoss
        ? 'STOP LOSS'
        : parsedTrigger.type === 'stop'
          ? 'STOP ORDER'
          : 'LIMIT ORDER'
    addReferenceLine(rows, seen, {
      id: `${kind}-${parsedTrigger.price}`,
      kind,
      label: `${shortLabel} ${formattedReferencePrice(parsedTrigger.price)}`,
      price: parsedTrigger.price,
      tone:
        kind === 'stop'
          ? 'negative'
          : kind === 'take_profit'
            ? 'positive'
            : 'slow',
      lineStyle: kind === 'limit' ? 3 : 2,
      lineWidth: 2,
    })
  }
  return rows
}

function referenceLineOptions(
  row: ExecutionReferenceLineRow,
  theme: LightweightChartTheme,
): Record<string, unknown> {
  return {
    price: row.price,
    title: row.label,
    color: lightweightChartSeriesColor(theme, row.tone),
    lineStyle: row.lineStyle,
    lineWidth: row.lineWidth,
    axisLabelVisible: true,
    lineVisible: true,
  }
}

function clearExecutionReferenceLines(): void {
  const candleSeries = chartRuntime.candleSeries
  for (const reference of chartRuntime.referenceLines) {
    if (candleSeries && typeof candleSeries.removePriceLine === 'function') {
      try {
        candleSeries.removePriceLine(reference.api)
      } catch {}
    }
  }
  chartRuntime.referenceLines = []
}

function renderExecutionReferenceLines(
  rows: readonly ExecutionReferenceLineRow[],
  theme: LightweightChartTheme,
): {
  summaries: ExecutionChartReferenceLineSummary[]
  issue: string
  omittedCount: number
} {
  clearExecutionReferenceLines()
  const visibleRows = rows.slice(0, EXECUTION_REFERENCE_LINE_LIMIT)
  const omittedCount = Math.max(0, rows.length - visibleRows.length)
  const omittedIssue = omittedCount
    ? `${omittedCount} additional broker price level${omittedCount === 1 ? '' : 's'} ${omittedCount === 1 ? 'was' : 'were'} omitted because the chart displays at most ${EXECUTION_REFERENCE_LINE_LIMIT}.`
    : ''
  if (!visibleRows.length) return { summaries: [], issue: '', omittedCount: 0 }
  const candleSeries = chartRuntime.candleSeries
  if (!candleSeries || typeof candleSeries.createPriceLine !== 'function') {
    return {
      summaries: [],
      issue: [
        'Broker price-level overlay is unavailable because the chart runtime does not expose price lines.',
        omittedIssue,
      ].filter(Boolean).join(' '),
      omittedCount,
    }
  }
  const summaries: ExecutionChartReferenceLineSummary[] = []
  let failed = 0
  for (const row of visibleRows) {
    try {
      const api = candleSeries.createPriceLine(referenceLineOptions(row, theme))
      if (!api) {
        failed += 1
        continue
      }
      chartRuntime.referenceLines.push({ row, api })
      summaries.push({
        id: row.id,
        kind: row.kind,
        label: row.label,
        price: row.price,
      })
    } catch {
      failed += 1
    }
  }
  const failureIssue = failed
    ? `${failed} broker price level${failed === 1 ? '' : 's'} could not be rendered.`
    : ''
  return {
    summaries,
    issue: [failureIssue, omittedIssue].filter(Boolean).join(' '),
    omittedCount,
  }
}

function reapplyExecutionReferenceLineTheme(theme: LightweightChartTheme): void {
  for (const reference of chartRuntime.referenceLines) {
    if (reference.api && typeof reference.api.applyOptions === 'function') {
      try {
        reference.api.applyOptions(referenceLineOptions(reference.row, theme))
      } catch {}
    }
  }
}

async function loadChartModule(): Promise<LightweightChartModule | null> {
  if (chartModulePromise) return chartModulePromise
  const attempt = import('lightweight-charts')
    .then((module) => module as LightweightChartModule)
    .catch(() => null)
  chartModulePromise = attempt
  const chartModule = await attempt
  if (!chartModule && chartModulePromise === attempt) {
    chartModulePromise = null
  }
  return chartModule
}

function clearRuntimeData(): void {
  clearExecutionReferenceLines()
  if (chartRuntime.candleSeries && typeof chartRuntime.candleSeries.setData === 'function') {
    try {
      chartRuntime.candleSeries.setData([])
    } catch {

    }
  }
  if (chartRuntime.markerApi && typeof chartRuntime.markerApi.setMarkers === 'function') {
    try {
      chartRuntime.markerApi.setMarkers([])
    } catch {

    }
  }
  chartRuntime.markerRows = []
}

export function disposeExecutionChart(): void {
  disposeExecutionChartRuntime(true)
}

function disposeExecutionChartRuntime(invalidateSync: boolean): void {
  if (invalidateSync) chartSyncGeneration += 1
  chartLifecycleGeneration += 1
  chartCreationHost = null
  chartCreationPromise = null
  clearExecutionReferenceLines()
  chartRuntime.stopThemeObserver?.()
  chartRuntime.stopThemeObserver = null
  if (chartRuntime.resizeObserver && chartRuntime.host) {
    try {
      chartRuntime.resizeObserver.unobserve(chartRuntime.host)
    } catch {

    }
  }
  if (chartRuntime.resizeObserver) {
    try {
      chartRuntime.resizeObserver.disconnect()
    } catch {

    }
  }
  chartRuntime.resizeObserver = null
  if (chartRuntime.chart && typeof chartRuntime.chart.remove === 'function') {
    try {
      chartRuntime.chart.remove()
    } catch {

    }
  }
  chartRuntime.host = null
  chartRuntime.chart = null
  chartRuntime.candleSeries = null
  chartRuntime.markerApi = null
  chartRuntime.markerRows = []
  chartRuntime.referenceLines = []
}

function reconcileExecutionChartHost(host: HTMLElement): void {
  const activeHostChanged = !!chartRuntime.host && chartRuntime.host !== host
  const pendingHostChanged =
    !!chartCreationPromise &&
    !!chartCreationHost &&
    chartCreationHost !== host
  if (activeHostChanged || pendingHostChanged) {
    disposeExecutionChartRuntime(false)
  }
}

export function fitExecutionChartContent(): string {
  fitLightweightChartContent(chartRuntime.chart)
  return visibleLogicalRangeLabel(chartRuntime.chart)
}

export function panExecutionChart(direction: 'left' | 'right'): string {
  panLightweightChart(chartRuntime.chart, direction)
  return visibleLogicalRangeLabel(chartRuntime.chart)
}

async function ensureExecutionChart(host: HTMLElement): Promise<boolean> {
  if (chartRuntime.chart && chartRuntime.host === host && chartRuntime.candleSeries) return true
  if (chartRuntime.chart && chartRuntime.host !== host) {
    disposeExecutionChartRuntime(false)
  }
  if (chartCreationPromise && chartCreationHost === host) return chartCreationPromise
  if (chartCreationPromise && chartCreationHost !== host) {
    disposeExecutionChartRuntime(false)
  }

  const generation = chartLifecycleGeneration
  const pending = (async () => {
    const chartModule = await loadChartModule()
    if (
      !chartModule?.createChart ||
      generation !== chartLifecycleGeneration ||
      chartCreationHost !== host
    ) {
      return false
    }
    if (chartRuntime.chart && chartRuntime.host === host && chartRuntime.candleSeries) {
      return true
    }

    const runtime = createCandlestickRuntime(host, chartModule)
    if (!runtime) return false
    if (generation !== chartLifecycleGeneration || chartCreationHost !== host) {
      try {
        runtime.chart?.remove?.()
      } catch {

      }
      return false
    }

    const chartAny = runtime.chart
    chartRuntime.host = host
    chartRuntime.chart = chartAny
    chartRuntime.candleSeries = runtime.candleSeries
    chartRuntime.markerApi = runtime.markerApi
    chartRuntime.stopThemeObserver = observeLightweightChartTheme(chartAny, {
      series: runtime,
      initialTheme: runtime.theme,
      onThemeChange: (theme) => {
        if (
          chartRuntime.markerApi &&
          typeof chartRuntime.markerApi.setMarkers === 'function'
        ) {
          try {
            chartRuntime.markerApi.setMarkers(
              colorizeExecutionSeriesMarkers(chartRuntime.markerRows, theme),
            )
          } catch {}
        }
        reapplyExecutionReferenceLineTheme(theme)
      },
    })
    if (typeof ResizeObserver !== 'undefined') {
      try {
        const observer = new ResizeObserver((entries) => {
          const entry = entries[0]
          const chartRef = chartRuntime.chart
          if (
            generation !== chartLifecycleGeneration ||
            chartRuntime.host !== host ||
            chartRef !== chartAny ||
            !entry ||
            typeof chartRef?.applyOptions !== 'function'
          ) {
            return
          }
          try {
            applyLightweightChartSize(chartRef, entry.contentRect.width, entry.contentRect.height)
          } catch {

          }
        })
        chartRuntime.resizeObserver = observer
        observer.observe(host)
      } catch {
        chartRuntime.resizeObserver = null
      }
    }
    return true
  })().catch(() => false)

  chartCreationHost = host
  chartCreationPromise = pending
  try {
    return await pending
  } finally {
    if (chartCreationPromise === pending) {
      chartCreationPromise = null
      chartCreationHost = null
    }
  }
}

export async function syncExecutionChart(
  host: HTMLElement | null,
  input: ExecutionChartSurfaceInput,
): Promise<ExecutionChartSurfaceResult> {
  const syncToken = chartSyncGeneration + 1
  chartSyncGeneration = syncToken
  const summary = summarizeExecutionChartSurface(input)
  if (!host) {
    disposeExecutionChartRuntime(false)
    return summary
  }
  if (
    summary.state === 'no_symbol' ||
    summary.state === 'no_dataset' ||
    summary.state === 'dataset_provenance_unknown' ||
    summary.state === 'symbol_mismatch'
  ) {
    disposeExecutionChartRuntime(false)
    return summary
  }
  reconcileExecutionChartHost(host)
  let bars: ExecutionPriceBar[]
  try {
    bars = await fetchExecutionPriceBars(summary.datasetId, input.datasetRevision)
  } catch {
    if (syncToken !== chartSyncGeneration) return summary
    clearRuntimeData()
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: price bars could not be loaded for ${summary.datasetLabel || summary.datasetId || summary.symbol || '-'}.`,
      markerCount: 0,
      barCount: 0,
      visibleRangeLabel: '',
    }
  }
  if (syncToken !== chartSyncGeneration) return summary
  if (!bars.length) {
    clearRuntimeData()
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: no price bars loaded for ${summary.datasetLabel || summary.datasetId || summary.symbol}.`,
      markerCount: 0,
      barCount: 0,
      visibleRangeLabel: '',
    }
  }
  const ready = await ensureExecutionChart(host)
  if (syncToken !== chartSyncGeneration) return summary
  if (!ready || !chartRuntime.candleSeries) {
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: chart runtime could not be created for ${summary.symbol || summary.datasetLabel || summary.datasetId || '-'}.`,
      markerCount: 0,
      barCount: 0,
      visibleRangeLabel: '',
    }
  }
  const markerEligible =
    summary.state === 'loaded_broker_markers' ||
    summary.state === 'stale_broker_review'
  const markerRows = markerEligible
    ? buildSeriesMarkerRows(input.orderRows, bars)
    : []
  const seriesMarkers = colorizeExecutionSeriesMarkers(
    markerRows,
    resolveLightweightChartTheme(),
  )
  const markerApiReady =
    !!chartRuntime.markerApi &&
    typeof chartRuntime.markerApi.setMarkers === 'function'
  let markerOverlayFailure: 'runtime_unavailable' | 'render_failed' | null =
    seriesMarkers.length > 0 && !markerApiReady
      ? 'runtime_unavailable'
      : null
  chartRuntime.markerRows = markerApiReady ? markerRows : []
  const referenceEligible =
    markerEligible ||
    summary.state === 'loaded_no_markers'
  const referenceRows = referenceEligible
    ? buildExecutionReferenceLines(input)
    : []
  let renderedMarkerCount = 0
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
  } catch {
    clearRuntimeData()
    return {
      ...summary,
      state: 'broker_review_loaded_chart_unavailable',
      summary: `Execution chart unavailable: render failed for ${summary.symbol || '-'} on ${summary.datasetLabel || summary.datasetId || '-'}.`,
      markerCount: 0,
      barCount: 0,
      visibleRangeLabel: '',
    }
  }
  if (chartRuntime.markerApi && typeof chartRuntime.markerApi.setMarkers === 'function') {
    try {
      chartRuntime.markerApi.setMarkers(seriesMarkers)
      renderedMarkerCount = seriesMarkers.length
    } catch {
      chartRuntime.markerApi = null
      chartRuntime.markerRows = []
      if (seriesMarkers.length > 0) markerOverlayFailure = 'render_failed'
    }
  }
  const referenceLineResult = renderExecutionReferenceLines(
    referenceRows,
    resolveLightweightChartTheme(),
  )
  fitLightweightChartContent(chartRuntime.chart)
  let renderedState = summary.state
  let renderedSummary = summary.summary
  if (markerOverlayFailure) {
    renderedState = 'loaded_marker_overlay_unavailable'
    const reason = markerOverlayFailure === 'runtime_unavailable'
      ? 'the marker runtime did not initialize'
      : 'the marker runtime rejected the marker update'
    renderedSummary = `Execution chart loaded for ${summary.symbol || summary.datasetLabel || summary.datasetId || '-'} with ${bars.length} price bar${bars.length === 1 ? '' : 's'}, but the broker order marker overlay is unavailable because ${reason}. No broker markers are displayed.`
  } else if (summary.state === 'loaded_broker_markers') {
    if (!renderedMarkerCount) {
      renderedState = 'loaded_no_markers'
      renderedSummary = `Execution chart loaded for ${summary.symbol}. No broker-confirmed order markers fall within the loaded dataset range.`
    } else if (renderedMarkerCount !== summary.markerCount) {
      const omitted = summary.markerCount - renderedMarkerCount
      renderedSummary = `Execution chart loaded for ${summary.symbol} with ${renderedMarkerCount} broker-confirmed order marker${renderedMarkerCount === 1 ? '' : 's'}; ${omitted} outside the loaded dataset range ${omitted === 1 ? 'was' : 'were'} omitted.`
    }
  }
  if (referenceLineResult.summaries.length > 0) {
    renderedSummary = `${renderedSummary} ${referenceLineResult.summaries.length} broker-confirmed price level${referenceLineResult.summaries.length === 1 ? '' : 's'} displayed.`
  }
  return {
    ...summary,
    state: renderedState,
    summary: renderedSummary,
    markerCount: renderedMarkerCount,
    barCount: bars.length,
    visibleRangeLabel: visibleLogicalRangeLabel(chartRuntime.chart),
    referenceLines: referenceLineResult.summaries,
    ...(referenceLineResult.issue
      ? { referenceLineIssue: referenceLineResult.issue }
      : {}),
    ...(referenceLineResult.omittedCount
      ? { referenceLineOmittedCount: referenceLineResult.omittedCount }
      : {}),
    markerBreakdown: renderedMarkerCount
      ? markerBreakdown(markerRows)
      : emptyMarkerBreakdown(),
  }
}
