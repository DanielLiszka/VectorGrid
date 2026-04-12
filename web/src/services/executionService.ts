import { invokeCommand } from '../tauri/invoke'
import { listenEvent } from '../tauri/events'
import { readLatestWorkflowRunSnapshot } from '../stores/workflowArtifactsStore'
import { EXEC_KEYS_STORAGE_KEY } from '../utils/core/storageKeys'
import { formatUtcDateTimeFromUnixMs } from '../utils/uiFormat'
import { readStorageString, removeStorageKey, writeStorageString } from '../utils/storage'

let analysisServicePromise: Promise<typeof import('./analysisService')> | null = null
let optimizeServicePromise: Promise<typeof import('./optimizeService')> | null = null

function loadAnalysisService(): Promise<typeof import('./analysisService')> {
  if (!analysisServicePromise) analysisServicePromise = import('./analysisService')
  return analysisServicePromise
}

function loadOptimizeService(): Promise<typeof import('./optimizeService')> {
  if (!optimizeServicePromise) optimizeServicePromise = import('./optimizeService')
  return optimizeServicePromise
}

interface ExecutionStoredKeysPayload {
  api_key?: unknown
  api_secret?: unknown
}

interface ExecutionStoredCredentialStatusResponse {
  remember_available?: unknown
  has_api_key?: unknown
  has_api_secret?: unknown
  has_credentials?: unknown
  state?: unknown
  storage_backend?: unknown
  message?: unknown
}

interface ExecutionRiskPayload {
  stop_loss_pct: number | null
  take_profit_pct: number | null
}

interface ExecutionActionOptions {
  mode: string
  symbol: string
  qty: string | number
  paramSource: string
  riskStopLossPct: string
  riskTakeProfitPct: string
  manualFastLen: string
  manualSlowLen: string
  dryRun: boolean
  rememberKeys: boolean
  apiKey: string
  apiSecret: string
  liveUseDelayed: boolean
}

interface ExecutionRunnerStatusResponse {
  active?: unknown
  inflight?: unknown
  run_count?: unknown
  skip_count?: unknown
  error_count?: unknown
  config?: unknown
}

interface ExecutionRunnerConfigResponse {
  symbol?: unknown
  mode?: unknown
  dry_run?: unknown
}

interface ExecutionConnectionResponse {
  account_number?: unknown
  account_id?: unknown
  status?: unknown
}

interface ExecutionSnapshotOrderResponse {
  id?: unknown
  side?: unknown
  status?: unknown
  qty?: unknown
  filled_qty?: unknown
  filled_avg_price?: unknown
  submitted_at?: unknown
  filled_at?: unknown
  updated_at?: unknown
  canceled_at?: unknown
  created_at?: unknown
}

interface ExecutionSnapshotPositionResponse {
  qty?: unknown
  side?: unknown
  avg_entry_price?: unknown
  current_price?: unknown
  market_value?: unknown
  unrealized_pl?: unknown
  unrealized_plpc?: unknown
}

interface ExecutionSnapshotResponse {
  orders?: unknown
  symbol?: unknown
  position?: unknown
}

interface LiveCandleStreamInfoResponse {
  symbol?: unknown
  delayed?: unknown
  market?: unknown
  active?: unknown
}

interface LiveCandleStreamStatusResponse {
  active?: unknown
  stream?: unknown
}

interface LiveCandleStreamResponse {
  stopped?: unknown
  symbol?: unknown
  delayed?: unknown
  channel?: unknown
}

interface ExecuteSignalResponse {
  action?: unknown
  order_submitted?: unknown
  order_id?: unknown
  delta_qty?: unknown
  last_bar_ts?: unknown
  warnings?: unknown
}

interface PolygonLiveStatusEventPayload {
  status?: unknown
  message?: unknown
  market?: unknown
  symbol?: unknown
  delayed?: unknown
  channel?: unknown
  at_unix_ms?: unknown
}

interface LiveExecutionRunnerEventPayload {
  status?: unknown
  message?: unknown
  symbol?: unknown
  at_unix_ms?: unknown
  bar_open_unix_ms?: unknown
  execution?: unknown
}

const RUNNER_ACTIVE_EVENT_STATUSES = new Set([
  'started',
  'triggered',
  'queued_latest',
  'signal_processed',
  'order_submitted',
])

const RUNNER_INACTIVE_EVENT_STATUSES = new Set([
  'stopped',
  'cancelled',
])

export interface ExecutionOrderRow {
  cells: string[]
}

export interface ExecutionActionResult {
  statusLine: string
  statusHint?: string
  liveStatusHint?: string
  runnerStatusHint?: string
  prependOrderRows?: ExecutionOrderRow[]
  positionMeta?: string
  chartMeta?: string
  outputPreview?: string
  orderRows?: ExecutionOrderRow[]
  orderCount?: number
  liveActive?: boolean
  runnerActive?: boolean
  liveSymbol?: string
  liveDelayed?: boolean
}

export interface ExecutionStoredCredentialStatus {
  rememberAvailable: boolean
  hasApiKey: boolean
  hasApiSecret: boolean
  hasCredentials: boolean
  state: 'missing' | 'stored' | 'incomplete' | 'unavailable'
  storageBackend: string
  message: string
}

export interface ParsedLiveStatus {
  active: boolean
  symbol: string
  delayed: boolean
  hint: string
  status: string
  message: string
}

export interface ParsedRunnerEvent {
  status: string
  symbol: string
  message: string
  hint: string
  active: boolean | null
  execution: ExecuteSignalResponse | null
}

export interface ExecutionRuntimeEventHandlers {
  onPolygonLiveStatus?: (event: ParsedLiveStatus) => void
  onRunnerEvent?: (event: ParsedRunnerEvent) => void
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

export function canonicalSymbol(raw: unknown): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, '')
    .toUpperCase()
}

function toNumber(value: unknown): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return Number.NaN
  return parsed
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value)
}

function modeValue(raw: unknown): 'paper' | 'live' {
  return text(raw).toLowerCase() === 'live' ? 'live' : 'paper'
}

function sourceValue(raw: unknown): 'selected' | 'best' | 'manual' {
  const normalized = text(raw).toLowerCase()
  if (normalized === 'best') return 'best'
  if (normalized === 'manual') return 'manual'
  return 'selected'
}

function parseRiskPct(raw: unknown, label: string): number | null {
  const value = text(raw)
  if (!value) return null
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${label} must be finite and > 0.`)
  }
  const fraction = parsed > 1 ? parsed / 100 : parsed
  if (!Number.isFinite(fraction) || fraction <= 0 || fraction >= 1) {
    throw new Error(`${label} must be in (0, 1) or a percent in (0, 100).`)
  }
  return fraction
}

function parseQty(raw: unknown): number {
  const value = Number(raw)
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error('Execution quantity must be finite and > 0.')
  }
  const rounded = Math.round(value * 10_000) / 10_000
  if (!Number.isFinite(rounded) || rounded <= 0) {
    throw new Error('Execution quantity is too small.')
  }
  return rounded
}

function requireSymbol(raw: unknown): string {
  const symbol = canonicalSymbol(raw)
  if (!symbol) throw new Error('Execution symbol is required (e.g. AAPL).')
  return symbol
}

function parseFallbackLen(value: unknown): number | null {
  if (Array.isArray(value)) {
    const next = Math.floor(toNumber(value[0]))
    return Number.isFinite(next) && next >= 1 && next <= 65_535 ? next : null
  }
  if (isRecord(value)) {
    const next = Math.floor(toNumber(value.start))
    return Number.isFinite(next) && next >= 1 && next <= 65_535 ? next : null
  }
  const next = Math.floor(toNumber(value))
  return Number.isFinite(next) && next >= 1 && next <= 65_535 ? next : null
}

function parseManualLen(raw: unknown, fallbackRaw: unknown, label: string): number {
  const value = text(raw)
  const parsedRaw = value ? Math.floor(toNumber(value)) : parseFallbackLen(fallbackRaw)
  if (typeof parsedRaw !== 'number' || !Number.isFinite(parsedRaw) || parsedRaw < 1 || parsedRaw > 65_535) {
    throw new Error(`${label} must be an integer in [1, 65535].`)
  }
  return parsedRaw
}

function safePrettyJson(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2)
  } catch {
    return String(value ?? '')
  }
}

function trimTrailingZeros(num: number, maxDecimals = 6): string {
  if (!Number.isFinite(num)) return '-'
  const fixed = num.toFixed(Math.max(0, Math.floor(maxDecimals)))
  return fixed.replace(/(?:\.0+|(\.\d*?)0+)$/, '$1')
}

function toUtcDateTime(rawMs: unknown): string {
  return formatUtcDateTimeFromUnixMs(rawMs)
}

function orderTimeMs(order: ExecutionSnapshotOrderResponse): number {
  const candidates: unknown[] = [
    order.filled_at,
    order.submitted_at,
    order.updated_at,
    order.canceled_at,
    order.created_at,
  ]
  for (const raw of candidates) {
    if (typeof raw === 'number' && Number.isFinite(raw) && raw > 0) {
      const abs = Math.abs(raw)
      return abs >= 1e12 ? Math.floor(raw) : Math.floor(raw * 1000)
    }
    const txt = text(raw)
    if (!txt) continue
    const parsed = Date.parse(txt)
    if (Number.isFinite(parsed) && parsed > 0) return parsed
  }
  return Number.NaN
}

function buildOrderRows(ordersRaw: unknown): ExecutionOrderRow[] {
  if (!Array.isArray(ordersRaw)) return []
  const rows: ExecutionOrderRow[] = []
  for (const entry of ordersRaw) {
    const order = isRecord(entry) ? (entry as ExecutionSnapshotOrderResponse) : null
    if (!order) continue
    const timeText = toUtcDateTime(orderTimeMs(order))
    const side = text(order.side) || '-'
    const status = text(order.status) || '-'
    const qty = trimTrailingZeros(toNumber(order.qty))
    const filled = trimTrailingZeros(toNumber(order.filled_qty))
    const avgPx = trimTrailingZeros(toNumber(order.filled_avg_price))
    const orderId = text(order.id) || '-'
    rows.push({
      cells: [timeText, side, status, qty, filled, avgPx, orderId],
    })
  }
  return rows
}

function positionMeta(raw: unknown): string {
  const position = isRecord(raw) ? (raw as ExecutionSnapshotPositionResponse) : null
  if (!position) return 'Position: flat'
  const qty = toNumber(position.qty)
  if (!Number.isFinite(qty) || Math.abs(qty) < 1e-12) return 'Position: flat'
  const side = text(position.side) || (qty > 0 ? 'long' : 'short')
  const avg = trimTrailingZeros(toNumber(position.avg_entry_price), 4)
  const now = trimTrailingZeros(toNumber(position.current_price), 4)
  const upl = trimTrailingZeros(toNumber(position.unrealized_pl), 4)
  const uplPct = toNumber(position.unrealized_plpc)
  const uplPctText = Number.isFinite(uplPct) ? `${trimTrailingZeros(uplPct * 100, 2)}%` : '-'
  return `Position: qty=${trimTrailingZeros(qty, 6)} side=${side} avg=${avg} now=${now} upl=${upl} (${uplPctText})`
}

function chartMeta(symbolRaw: unknown, orderCount: number): string {
  const symbol = canonicalSymbol(symbolRaw) || '-'
  return `Execution chart sync pending. symbol=${symbol} orders=${orderCount}`
}

function summarizeRunnerStatus(raw: unknown): { text: string; active: boolean } {
  const status = isRecord(raw) ? (raw as ExecutionRunnerStatusResponse) : {}
  const active = !!status.active
  const cfg = isRecord(status.config) ? (status.config as ExecutionRunnerConfigResponse) : null
  if (!active || !cfg) return { text: 'Auto-runner: off', active: false }
  const mode = text(cfg.mode).toLowerCase() || 'paper'
  const dry = !!cfg.dry_run ? 'dry-run' : 'broker'
  const sym = canonicalSymbol(cfg.symbol) || text(cfg.symbol) || '-'
  const runs = Math.max(0, Math.floor(Number(status.run_count || 0)))
  const errs = Math.max(0, Math.floor(Number(status.error_count || 0)))
  const skips = Math.max(0, Math.floor(Number(status.skip_count || 0)))
  const inflight = !!status.inflight
  return {
    text: `Auto-runner: on (${sym}, ${mode}, ${dry}) runs=${runs} skips=${skips} errs=${errs}${inflight ? ' [in-flight]' : ''}`,
    active,
  }
}

function liveStatusHint(active: boolean, symbolRaw: unknown, delayed: boolean, statusRaw?: unknown): string {
  const symbol = canonicalSymbol(symbolRaw)
  const suffix = symbol ? ` for ${symbol}` : ''
  const delayText = delayed ? ' [delayed]' : ''
  const status = text(statusRaw).toLowerCase()
  if (active && status === 'connecting') {
    return `Live candles connecting${suffix}${delayText}.`.trim()
  }
  if (active && status === 'subscribed') {
    return `Live candles subscribed${suffix}${delayText}.`.trim()
  }
  if (active && status === 'started') {
    return `Live candles started${suffix}${delayText}.`.trim()
  }
  if (!active) {
    if (status === 'stopped') return 'Live candles stopped.'
    if (status === 'closed') return `Live candles closed${suffix}${delayText}.`.trim()
    if (status === 'error') return `Live candles error${suffix}${delayText}.`.trim()
    return 'Live candles: off'
  }
  return `Live candles active${suffix}${delayText}.`.trim()
}

function parseLiveStreamStatus(raw: unknown): ParsedLiveStatus {
  const row = isRecord(raw) ? (raw as LiveCandleStreamStatusResponse) : {}
  const stream = isRecord(row.stream) ? (row.stream as LiveCandleStreamInfoResponse) : null
  const active = !!row.active
  const symbol = canonicalSymbol(stream?.symbol)
  const delayed = !!stream?.delayed
  return {
    active,
    symbol,
    delayed,
    hint: liveStatusHint(active, symbol, delayed),
    status: active ? 'active' : 'off',
    message: '',
  }
}

export function parsePolygonLiveStatusEvent(raw: unknown): ParsedLiveStatus {
  const row = isRecord(raw) ? (raw as PolygonLiveStatusEventPayload) : {}
  const status = text(row.status).toLowerCase()
  const symbol = canonicalSymbol(row.symbol)
  const delayed = !!row.delayed
  const active = status === 'started' || status === 'connecting' || status === 'subscribed'
  return {
    active,
    symbol,
    delayed,
    hint: liveStatusHint(active, symbol, delayed, status),
    status,
    message: text(row.message),
  }
}

export function optimisticOrderRowFromExecution(
  executionRaw: unknown,
): ExecutionOrderRow | null {
  const execution = isRecord(executionRaw) ? (executionRaw as ExecuteSignalResponse) : null
  if (!execution?.order_submitted) return null
  const lastBarTs = toNumber(execution.last_bar_ts)
  const atUnixMs = Number.isFinite(lastBarTs) && lastBarTs > 0 ? Math.floor(lastBarTs) : Date.now()
  const side = text(execution.action).toLowerCase() || '-'
  const qty = trimTrailingZeros(Math.abs(toNumber(execution.delta_qty)), 6)
  const orderId = text(execution.order_id) || '-'
  return {
    cells: [
      toUtcDateTime(atUnixMs),
      side,
      'submitted*',
      qty,
      '-',
      '-',
      orderId,
    ],
  }
}

export function parseRunnerEvent(raw: unknown): ParsedRunnerEvent {
  const row = isRecord(raw) ? (raw as LiveExecutionRunnerEventPayload) : {}
  const status = text(row.status).toLowerCase()
  const symbol = canonicalSymbol(row.symbol)
  const message = text(row.message)
  let active: boolean | null = null
  if (RUNNER_INACTIVE_EVENT_STATUSES.has(status)) {
    active = false
  }
  if (RUNNER_ACTIVE_EVENT_STATUSES.has(status)) {
    active = true
  }
  return {
    status,
    symbol,
    message,
    hint: message ? `Auto-runner: ${message}` : 'Auto-runner status updated.',
    active,
    execution: isRecord(row.execution) ? (row.execution as ExecuteSignalResponse) : null,
  }
}

export async function subscribeExecutionRuntimeEvents(
  handlers: ExecutionRuntimeEventHandlers,
): Promise<() => void> {
  const unsubs = await Promise.all([
    listenEvent<PolygonLiveStatusEventPayload>('polygon_live_status', (payload) => {
      handlers.onPolygonLiveStatus?.(parsePolygonLiveStatusEvent(payload))
    }),
    listenEvent<LiveExecutionRunnerEventPayload>('live_execution_runner_event', (payload) => {
      handlers.onRunnerEvent?.(parseRunnerEvent(payload))
    }),
  ])
  return () => {
    for (const off of unsubs) {
      try {
        off?.()
      } catch {
        // best-effort teardown
      }
    }
  }
}

function parseStoredExecutionKeys(raw: unknown): { apiKey: string; apiSecret: string } {
  const payload = isRecord(raw) ? (raw as ExecutionStoredKeysPayload) : null
  if (!payload) return { apiKey: '', apiSecret: '' }
  return {
    apiKey: text(payload.api_key),
    apiSecret: text(payload.api_secret),
  }
}

function parseStoredExecutionCredentialStatus(raw: unknown): ExecutionStoredCredentialStatus {
  const row = isRecord(raw) ? (raw as ExecutionStoredCredentialStatusResponse) : {}
  const stateRaw = text(row.state).toLowerCase()
  const state =
    stateRaw === 'stored' || stateRaw === 'incomplete' || stateRaw === 'unavailable'
      ? (stateRaw as ExecutionStoredCredentialStatus['state'])
      : 'missing'
  return {
    rememberAvailable: !!row.remember_available,
    hasApiKey: !!row.has_api_key,
    hasApiSecret: !!row.has_api_secret,
    hasCredentials: !!row.has_credentials,
    state,
    storageBackend: text(row.storage_backend),
    message: text(row.message),
  }
}

export function loadPersistedExecutionKeys(): { apiKey: string; apiSecret: string } {
  try {
    const parsed = JSON.parse(readStorageString(EXEC_KEYS_STORAGE_KEY, 'null'))
    return parseStoredExecutionKeys(parsed)
  } catch {
    return { apiKey: '', apiSecret: '' }
  }
}

function syncPersistedExecutionKeys(options: ExecutionActionOptions): void {
  try {
    if (options.rememberKeys) {
      const apiKey = text(options.apiKey)
      const apiSecret = text(options.apiSecret)
      if (apiKey && apiSecret) {
        writeStorageString(
          EXEC_KEYS_STORAGE_KEY,
          JSON.stringify({ api_key: apiKey, api_secret: apiSecret }),
        )
        return
      }
    }
    removeStorageKey(EXEC_KEYS_STORAGE_KEY)
  } catch {
    // best effort only
  }
}

function resolveApiKeys(options: ExecutionActionOptions): { apiKey: string; apiSecret: string } {
  const providedKey = text(options.apiKey)
  const providedSecret = text(options.apiSecret)
  if (providedKey || providedSecret) {
    return {
      apiKey: providedKey,
      apiSecret: providedSecret,
    }
  }
  return loadPersistedExecutionKeys()
}

async function resolveManualExecutionStrategy(
  options: ExecutionActionOptions,
): Promise<{ strategyKind: string; strategy: { kind: 'double_ma' | 'composite_cpu'; req: Record<string, unknown> }; source: string }> {
  let runReq = isRecord(readLatestWorkflowRunSnapshot()?.req)
    ? (readLatestWorkflowRunSnapshot()?.req as Record<string, unknown>)
    : null
  if (!runReq) {
    const { buildOptimizationRequestFromVueStores } = await loadOptimizeService()
    runReq = await buildOptimizationRequestFromVueStores()
  }

  const fastMaType = text(Array.isArray(runReq.fast_ma_types) ? runReq.fast_ma_types[0] : runReq.fast_ma_type)
  const slowMaType = text(Array.isArray(runReq.slow_ma_types) ? runReq.slow_ma_types[0] : runReq.slow_ma_type)
  if (!fastMaType || !slowMaType) {
    throw new Error('Select fast/slow MA types before using Manual parameter source.')
  }

  const runRes = isRecord(readLatestWorkflowRunSnapshot()?.res)
    ? (readLatestWorkflowRunSnapshot()?.res as Record<string, unknown>)
    : null
  const fallbackBest = isRecord(runRes?.best_params) ? runRes?.best_params as Record<string, unknown> : null
  const fastLen = parseManualLen(
    options.manualFastLen,
    runReq.fast_range ?? fallbackBest?.fast_len ?? 5,
    'Manual fast length',
  )
  const slowLen = parseManualLen(
    options.manualSlowLen,
    runReq.slow_range ?? fallbackBest?.slow_len ?? 20,
    'Manual slow length',
  )
  if (fastLen >= slowLen) {
    throw new Error('Manual parameters require fast length < slow length.')
  }

  const { buildExecutionStrategyFromRunContext } = await loadAnalysisService()
  const strategySnapshot = buildExecutionStrategyFromRunContext(
    {
      fast_len: fastLen,
      slow_len: slowLen,
      fast_ma_type: fastMaType,
      slow_ma_type: slowMaType,
    },
    runReq,
  )
  if (!strategySnapshot) {
    throw new Error('Unable to build manual execution strategy from current settings.')
  }
  return {
    ...strategySnapshot,
    source: 'manual_ui',
  }
}

async function buildExecutionInvokePayload(
  options: ExecutionActionOptions,
): Promise<{
    req: Record<string, unknown>
    strategyKind: string
    source: string
  }> {
  const mode = modeValue(options.mode)
  const dry_run = !!options.dryRun
  const qty = parseQty(options.qty)
  const symbol = requireSymbol(options.symbol)
  const risk: ExecutionRiskPayload = {
    stop_loss_pct: parseRiskPct(options.riskStopLossPct, 'Risk override SL'),
    take_profit_pct: parseRiskPct(options.riskTakeProfitPct, 'Risk override TP'),
  }
  const api = resolveApiKeys(options)

  const paramSource = sourceValue(options.paramSource)
  if (paramSource === 'manual') {
    const manual = await resolveManualExecutionStrategy(options)
    return {
      req: {
        symbol,
        qty,
        mode,
        dry_run,
        risk,
        alpaca_api_key: api.apiKey,
        alpaca_api_secret: api.apiSecret,
        strategy: manual.strategy,
      },
      strategyKind: manual.strategyKind,
      source: manual.source,
    }
  }

  const snapshot = readLatestWorkflowRunSnapshot()
  const runReq = isRecord(snapshot?.req) ? (snapshot.req as Record<string, unknown>) : null
  if (!runReq) {
    throw new Error('No optimization run loaded. Use Manual parameter source or run an optimization.')
  }
  const { buildExecutionStrategyFromRunContext, resolveExecutionTopRowSnapshot } =
    await loadAnalysisService()
  const row = resolveExecutionTopRowSnapshot(paramSource)
  const strategy = buildExecutionStrategyFromRunContext(row.params, runReq)
  if (!strategy) {
    throw new Error('Unable to build strategy request from selected parameters.')
  }
  return {
    req: {
      symbol,
      qty,
      mode,
      dry_run,
      risk,
      alpaca_api_key: api.apiKey,
      alpaca_api_secret: api.apiSecret,
      strategy: strategy.strategy,
    },
    strategyKind: strategy.strategyKind,
    source: row.source,
  }
}

async function refreshRunnerStatusHint(): Promise<{ hint: string; active: boolean }> {
  const raw = await invokeCommand('live_execution_runner_status')
  const summary = summarizeRunnerStatus(raw)
  return { hint: summary.text, active: summary.active }
}

async function refreshLiveStatus(): Promise<ParsedLiveStatus> {
  return parseLiveStreamStatus(
    (await invokeCommand('polygon_live_candle_stream_status')) as LiveCandleStreamStatusResponse,
  )
}

export async function testConnection(options: ExecutionActionOptions): Promise<ExecutionActionResult> {
  const mode = modeValue(options.mode)
  const api = resolveApiKeys(options)
  if (!api.apiKey || !api.apiSecret) {
    throw new Error('API key + secret are required for account test.')
  }

  const resp = (await invokeCommand('test_alpaca_connection', {
    req: { mode, alpaca_api_key: api.apiKey, alpaca_api_secret: api.apiSecret },
  })) as ExecutionConnectionResponse
  syncPersistedExecutionKeys(options)
  const account = text(resp?.account_number || resp?.account_id)
  const status = text(resp?.status)
  const suffix = `${account ? ` account=${account}` : ''}${status ? ` status=${status}` : ''}`
  return {
    statusLine: `Alpaca ${mode} connection OK.`,
    statusHint: `Alpaca ${mode} connection OK.${suffix}`.trim(),
    outputPreview: safePrettyJson(resp),
  }
}

export async function refreshOrders(options: ExecutionActionOptions): Promise<ExecutionActionResult> {
  const mode = modeValue(options.mode)
  const api = resolveApiKeys(options)
  if (!api.apiKey || !api.apiSecret) {
    throw new Error('API key + secret are required to refresh broker orders.')
  }
  const symbol = canonicalSymbol(options.symbol)
  const resp = (await invokeCommand('get_live_execution_snapshot', {
    req: {
      mode,
      alpaca_api_key: api.apiKey,
      alpaca_api_secret: api.apiSecret,
      symbol,
      limit: 200,
      include_account: false,
      include_orders: true,
      include_position: true,
    },
  })) as ExecutionSnapshotResponse
  syncPersistedExecutionKeys(options)
  const rows = buildOrderRows(resp?.orders)
  const count = rows.length
  const scopeLabel = symbol
    ? `recent order${count === 1 ? '' : 's'} for ${symbol}`
    : `order${count === 1 ? '' : 's'} for all symbols`
  return {
    statusLine: `Loaded ${count} ${scopeLabel}.`,
    statusHint: `Loaded ${count} ${scopeLabel}.`,
    outputPreview: safePrettyJson(resp),
    orderRows: rows,
    orderCount: count,
    positionMeta: positionMeta(resp?.position),
    chartMeta: chartMeta(resp?.symbol || symbol, count),
  }
}

export async function startLive(options: ExecutionActionOptions): Promise<ExecutionActionResult> {
  const symbol = requireSymbol(options.symbol)
  const delayed = !!options.liveUseDelayed
  const resp = (await invokeCommand('polygon_start_live_candle_stream', {
    req: { symbol, market: 'stocks', delayed },
  })) as LiveCandleStreamResponse
  return {
    statusLine: `Live candles started for ${text(resp?.symbol) || symbol}.`,
    liveStatusHint: `Live candles started for ${text(resp?.symbol) || symbol}${text(resp?.channel) ? ` (${text(resp?.channel)})` : ''}${resp?.delayed ? ' [delayed]' : ''}.`,
    liveActive: true,
    liveSymbol: canonicalSymbol(resp?.symbol || symbol),
    liveDelayed: !!resp?.delayed,
  }
}

export async function stopLive(): Promise<ExecutionActionResult> {
  const resp = (await invokeCommand('polygon_stop_live_candle_stream')) as LiveCandleStreamResponse
  const stopped = !!resp?.stopped
  return {
    statusLine: stopped ? 'Live candles stopped.' : 'Live candles were already stopped.',
    liveStatusHint: stopped ? 'Live candles stopped.' : 'Live candles were already stopped.',
    liveActive: false,
    liveSymbol: '',
    liveDelayed: false,
  }
}

export async function startRunner(options: ExecutionActionOptions): Promise<ExecutionActionResult> {
  const payload = await buildExecutionInvokePayload(options)
  const req = payload.req
  if (!req.dry_run && (!text(req.alpaca_api_key) || !text(req.alpaca_api_secret))) {
    throw new Error('API key + secret are required unless dry-run is enabled.')
  }

  const live = parseLiveStreamStatus(
    (await invokeCommand('polygon_live_candle_stream_status')) as LiveCandleStreamStatusResponse,
  )
  if (!live.active) {
    throw new Error('Start live candles first. Auto-runner triggers on Polygon live bars.')
  }
  const requestedSymbol = canonicalSymbol(req.symbol)
  if (live.symbol && live.symbol !== requestedSymbol) {
    throw new Error(`Live candles are active for ${live.symbol}, not ${requestedSymbol}.`)
  }

  const statusRaw = await invokeCommand('start_live_execution_runner', { req })
  syncPersistedExecutionKeys(options)
  const summary = summarizeRunnerStatus(statusRaw)
  return {
    statusLine: `Auto-runner started for ${text(req.symbol) || '-'}.`,
    runnerStatusHint: summary.text,
    runnerActive: summary.active,
    liveSymbol: live.symbol,
    liveDelayed: live.delayed,
  }
}

export async function stopRunner(): Promise<ExecutionActionResult> {
  const resp = await invokeCommand('stop_live_execution_runner')
  const row = isRecord(resp) ? resp : {}
  const stopped = !!row.stopped
  const summary = summarizeRunnerStatus(row.status)
  return {
    statusLine: stopped ? 'Auto-runner stopped.' : 'Auto-runner was already off.',
    runnerStatusHint: summary.text,
    runnerActive: summary.active,
  }
}

export async function runOnce(options: ExecutionActionOptions): Promise<ExecutionActionResult> {
  const payload = await buildExecutionInvokePayload(options)
  const req = payload.req
  if (!req.dry_run && (!text(req.alpaca_api_key) || !text(req.alpaca_api_secret))) {
    throw new Error('API key + secret are required unless dry-run is enabled.')
  }
  const resp = (await invokeCommand('execute_strategy_signal_once', { req })) as ExecuteSignalResponse
  syncPersistedExecutionKeys(options)

  const action = text(resp?.action || 'hold') || 'hold'
  const submitted = !!resp?.order_submitted
  const orderId = text(resp?.order_id)
  const warningCount = Array.isArray(resp?.warnings) ? resp.warnings.length : 0
  let statusHint = submitted
    ? `Submitted ${action.toUpperCase()} order${orderId ? ` (id=${orderId})` : ''}.`
    : `Signal processed: action=${action}.`
  if (req.dry_run) statusHint += ' Dry-run only.'
  if (warningCount) statusHint += ` Warnings=${warningCount}.`
  if (submitted && !req.dry_run) {
    statusHint += ' Use Refresh orders for broker-confirmed state.'
  }

  const result: ExecutionActionResult = {
    statusLine: statusHint,
    statusHint,
    outputPreview: safePrettyJson(resp),
  }

  if (submitted && !req.dry_run) {
    const optimisticRow = optimisticOrderRowFromExecution(resp)
    if (optimisticRow) {
      result.prependOrderRows = [optimisticRow]
    }
  }

  return result
}

export async function refreshStatuses(): Promise<ExecutionActionResult> {
  const [runner, live] = await Promise.allSettled([
    refreshRunnerStatusHint(),
    refreshLiveStatus(),
  ])
  const runnerHint = runner.status === 'fulfilled'
    ? runner.value.hint
    : 'Auto-runner status unavailable.'
  const runnerActive = runner.status === 'fulfilled' ? runner.value.active : false
  const liveSummary = live.status === 'fulfilled'
    ? live.value
    : null
  const liveHint = liveSummary
    ? liveSummary.hint
    : 'Live status unavailable.'
  const liveActive = liveSummary ? liveSummary.active : false
  return {
    statusLine: 'Execution status refreshed.',
    runnerStatusHint: runnerHint,
    runnerActive,
    liveStatusHint: liveHint,
    liveActive,
    liveSymbol: liveSummary?.symbol || '',
    liveDelayed: !!liveSummary?.delayed,
  }
}

export async function getStoredExecutionCredentialStatus(): Promise<ExecutionStoredCredentialStatus> {
  return parseStoredExecutionCredentialStatus(await invokeCommand('alpaca_get_credential_status'))
}

export async function storeExecutionCredentialsSecurely(
  apiKey: string,
  apiSecret: string,
): Promise<ExecutionStoredCredentialStatus> {
  return parseStoredExecutionCredentialStatus(
    await invokeCommand('alpaca_set_api_credentials', {
      req: { api_key: text(apiKey), api_secret: text(apiSecret) },
    }),
  )
}

export async function clearStoredExecutionCredentialsSecurely(): Promise<ExecutionStoredCredentialStatus> {
  return parseStoredExecutionCredentialStatus(await invokeCommand('alpaca_clear_api_credentials'))
}

export async function migrateLegacyStoredExecutionKeysToSecureStorage(): Promise<ExecutionStoredCredentialStatus> {
  const legacy = loadPersistedExecutionKeys()
  const legacyKey = text(legacy.apiKey)
  const legacySecret = text(legacy.apiSecret)
  if (!legacyKey && !legacySecret) {
    return getStoredExecutionCredentialStatus()
  }
  if (!legacyKey || !legacySecret) {
    return {
      rememberAvailable: false,
      hasApiKey: !!legacyKey,
      hasApiSecret: !!legacySecret,
      hasCredentials: false,
      state: 'incomplete',
      storageBackend: '',
      message: 'Legacy stored execution credentials are incomplete.',
    }
  }
  const status = await storeExecutionCredentialsSecurely(legacyKey, legacySecret)
  if (status.hasCredentials) {
    try {
      removeStorageKey(EXEC_KEYS_STORAGE_KEY)
    } catch {
      // best-effort cleanup only after verified secure write
    }
  }
  return status
}

export async function clearKeys(): Promise<ExecutionActionResult> {
  try {
    await clearStoredExecutionCredentialsSecurely()
  } catch {
    // best-effort secure-store cleanup during the transition to backend-owned secrets
  }
  try {
    removeStorageKey(EXEC_KEYS_STORAGE_KEY)
  } catch {
    // best-effort local key cleanup only
  }
  return {
    statusLine: 'Cleared execution API keys.',
    statusHint: 'Cleared execution API keys.',
    outputPreview: '-',
  }
}
