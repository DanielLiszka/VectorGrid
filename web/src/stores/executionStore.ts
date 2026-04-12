import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import {
  cancelOpenOrders as cancelOpenOrdersAction,
  canonicalSymbol,
  clearKeys as clearKeysAction,
  emergencyHalt as emergencyHaltAction,
  flattenPosition as flattenPositionAction,
  getStoredExecutionCredentialStatus,
  listSavedLiveSessions as listSavedLiveSessionsAction,
  loadSavedLiveSessionDetail as loadSavedLiveSessionDetailAction,
  migrateLegacyStoredExecutionKeysToSecureStorage,
  optimisticOrderRowFromExecution,
  refreshOrders as refreshOrdersAction,
  refreshStatuses as refreshStatusesAction,
  runOnce as runOnceAction,
  startLive as startLiveAction,
  startRunner as startRunnerAction,
  stopLive as stopLiveAction,
  stopRunner as stopRunnerAction,
  subscribeExecutionRuntimeEvents,
  testConnection as testConnectionAction,
  type ParsedLiveStatus,
  type ParsedRunnerEvent,
  type ExecutionActionResult,
  type ExecutionAccountReview,
  type ExecutionBrokerReview,
  type ExecutionChartReview,
  type ExecutionLiveLifecycleState,
  type ExecutionPositionReview,
  type ExecutionPreviewMeta,
  type ExecutionRunnerLifecycleState,
  type ExecutionSavedSessionDetail,
  type ExecutionSavedSessionSummary,
  type ExecutionStoredCredentialStatus,
} from '../services/executionService'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

function applyVueTextFieldUpdate(state: ExecutionState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false
  switch (key) {
    case 'execMode':
      state.mode = next || 'paper'
      return true
    case 'execSymbol':
      state.symbol = next
      return true
    case 'execQty':
      state.qty = next || '1'
      return true
    case 'execParamSource':
      state.paramSource = next || 'selected'
      return true
    case 'execRiskStopLossPct':
      state.riskStopLossPct = next
      return true
    case 'execRiskTakeProfitPct':
      state.riskTakeProfitPct = next
      return true
    case 'execManualFastLen':
      state.manualFastLen = next
      return true
    case 'execManualSlowLen':
      state.manualSlowLen = next
      return true
    case 'execApiKey':
      state.apiKey = next
      return true
    case 'execApiSecret':
      state.apiSecret = next
      return true
    default:
      return false
  }
}

function applyVueCheckboxFieldUpdate(state: ExecutionState, field: string, checked: boolean): boolean {
  const key = String(field || '').trim()
  const next = !!checked
  if (!key) return false
  switch (key) {
    case 'execDryRun':
      state.dryRun = next
      return true
    case 'execRememberKeys':
      state.rememberKeys = next
      return true
    case 'execLiveUseDelayed':
      state.liveUseDelayed = next
      return true
    default:
      return false
  }
}

export interface ExecutionOrderRow {
  cells: string[]
}

type ExecutionPendingKey =
  | 'refreshSavedSessions'
  | 'refreshStatuses'
  | 'testConnection'
  | 'runOnce'
  | 'refreshOrders'
  | 'cancelOpenOrders'
  | 'flattenPosition'
  | 'emergencyHalt'
  | 'startLive'
  | 'stopLive'
  | 'startRunner'
  | 'stopRunner'
  | 'clearKeys'

interface ExecutionPendingState {
  refreshSavedSessions: boolean
  refreshStatuses: boolean
  testConnection: boolean
  runOnce: boolean
  refreshOrders: boolean
  cancelOpenOrders: boolean
  flattenPosition: boolean
  emergencyHalt: boolean
  startLive: boolean
  stopLive: boolean
  startRunner: boolean
  stopRunner: boolean
  clearKeys: boolean
}

function createPendingState(): ExecutionPendingState {
  return {
    refreshSavedSessions: false,
    refreshStatuses: false,
    testConnection: false,
    runOnce: false,
    refreshOrders: false,
    cancelOpenOrders: false,
    flattenPosition: false,
    emergencyHalt: false,
    startLive: false,
    stopLive: false,
    startRunner: false,
    stopRunner: false,
    clearKeys: false,
  }
}

interface CredentialPresence {
  key: string
  secret: string
  any: boolean
  both: boolean
}

type ExecutionKeyStateKind =
  | 'missing'
  | 'incomplete'
  | 'session'
  | 'stored'
  | 'will-store'
  | 'will-update'
  | 'remember-unavailable'
  | 'clear-failed'

type ExecutionReconciliationStateKind =
  | 'disabled'
  | 'unknown'
  | 'broker_confirmed_flat'
  | 'broker_confirmed_open'
  | 'broker_stale'
  | 'optimistic_local_pending'

type ExecutionDatasetRecoveryState =
  | 'no_symbol'
  | 'unknown'
  | 'valid'
  | 'no_dataset'
  | 'symbol_mismatch'

interface ExecutionStoreRuntimeHost {
  _runtimeSubscriptionsPromise?: Promise<void> | null
  _runtimeSubscriptionsTeardown?: (() => void) | null
  _credentialHydrationPromise?: Promise<void> | null
}

function runtimeHost(store: unknown): ExecutionStoreRuntimeHost {
  return store as ExecutionStoreRuntimeHost
}

function redactExecutionCredentialText(state: ExecutionState, raw: unknown): string {
  let next = String(raw || '').trim()
  for (const secret of [String(state.apiKey || '').trim(), String(state.apiSecret || '').trim()]) {
    if (!secret) continue
    next = next.split(secret).join('[redacted]')
  }
  return next
}

function normalizedOrderRows(values: unknown): ExecutionOrderRow[] {
  if (!Array.isArray(values)) return []
  const rows: ExecutionOrderRow[] = []
  for (const row of values) {
    if (!row || typeof row !== 'object') continue
    const typed = row as Partial<ExecutionOrderRow>
    const sourceCells = Array.isArray(typed.cells) ? typed.cells : []
    const cells = sourceCells
      .map((cell) => String(cell || '').trim())
      .slice(0, 7)
    while (cells.length < 7) cells.push('—')
    const hasContent = cells.some((cell) => cell.length > 0 && cell !== '—')
    if (!hasContent) continue
    rows.push(markRaw({ cells: markRaw(cells) }))
  }
  return markRaw(rows)
}

function orderRowsEqual(left: ExecutionOrderRow[], right: ExecutionOrderRow[]): boolean {
  if (left === right) return true
  if (left.length !== right.length) return false
  for (let index = 0; index < left.length; index += 1) {
    const leftRow = left[index]
    const rightRow = right[index]
    const leftCells = Array.isArray(leftRow?.cells) ? leftRow.cells : []
    const rightCells = Array.isArray(rightRow?.cells) ? rightRow.cells : []
    if (leftCells.length !== rightCells.length) return false
    for (let cellIndex = 0; cellIndex < leftCells.length; cellIndex += 1) {
      if (leftCells[cellIndex] !== rightCells[cellIndex]) return false
    }
  }
  return true
}

function defaultAccountReview(mode: 'paper' | 'live' = 'paper'): ExecutionAccountReview {
  return {
    state: 'not_verified',
    mode,
    sourceCommand: 'none',
    fetchedAtUnixMs: null,
    accountId: '',
    accountNumber: '',
    status: '',
    currency: '',
    buyingPower: '',
    cash: '',
    portfolioValue: '',
  }
}

function defaultPreviewMeta(): ExecutionPreviewMeta {
  return {
    sourceCommand: 'none',
    scopeLabel: 'unscoped',
    generatedAtUnixMs: null,
  }
}

function defaultPositionReview(): ExecutionPositionReview {
  return {
    state: 'suppressed',
    symbol: '',
    qty: null,
    side: '',
    avgEntryPrice: null,
    currentPrice: null,
    marketValue: null,
    unrealizedPl: null,
    unrealizedPlpc: null,
  }
}

function defaultChartReview(): ExecutionChartReview {
  return {
    state: 'no_dataset',
    symbol: '',
    datasetId: '',
    datasetLabel: '-',
    datasetSymbol: '',
    orderCount: 0,
    sourceCommand: 'none',
    fetchedAtUnixMs: null,
  }
}

function optimisticOrderCount(rows: ExecutionOrderRow[]): number {
  let count = 0
  for (const row of rows) {
    const status = String(row?.cells?.[2] || '').trim().toLowerCase()
    if (status === 'submitted*') count += 1
  }
  return count
}

function accountMetaFromReview(review: ExecutionAccountReview): string {
  if (review.state === 'not_verified') return 'Account review: not verified.'
  if (review.state === 'unavailable') return `Account review: ${review.mode} account unavailable.`
  const parts = [
    `Account review: ${review.mode}`,
    review.accountNumber || review.accountId ? `account=${review.accountNumber || review.accountId}` : '',
    review.status ? `status=${review.status}` : '',
    review.currency ? `currency=${review.currency}` : '',
    review.buyingPower ? `buying_power=${review.buyingPower}` : '',
    review.cash ? `cash=${review.cash}` : '',
    review.portfolioValue ? `portfolio=${review.portfolioValue}` : '',
  ].filter((part) => part.length > 0)
  return parts.join(' · ')
}

function positionMetaFromReview(review: ExecutionPositionReview): string {
  if (review.state === 'suppressed') return 'Position: -'
  if (review.state === 'flat') return 'Position: flat'
  if (review.state !== 'open' || !Number.isFinite(review.qty ?? Number.NaN)) return 'Position: unavailable.'
  const avg = Number.isFinite(review.avgEntryPrice ?? Number.NaN) ? Number(review.avgEntryPrice).toFixed(4).replace(/(?:\.0+|(\.\d*?)0+)$/, '$1') : '-'
  const now = Number.isFinite(review.currentPrice ?? Number.NaN) ? Number(review.currentPrice).toFixed(4).replace(/(?:\.0+|(\.\d*?)0+)$/, '$1') : '-'
  const upl = Number.isFinite(review.unrealizedPl ?? Number.NaN) ? Number(review.unrealizedPl).toFixed(4).replace(/(?:\.0+|(\.\d*?)0+)$/, '$1') : '-'
  const uplPct = Number.isFinite(review.unrealizedPlpc ?? Number.NaN)
    ? `${(Number(review.unrealizedPlpc) * 100).toFixed(2).replace(/(?:\.0+|(\.\d*?)0+)$/, '$1')}%`
    : '-'
  const qty = Number.isFinite(review.qty ?? Number.NaN)
    ? Number(review.qty).toFixed(6).replace(/(?:\.0+|(\.\d*?)0+)$/, '$1')
    : '-'
  return `Position: qty=${qty} side=${review.side || '-'} avg=${avg} now=${now} upl=${upl} (${uplPct})`
}

function chartMetaFromReview(review: ExecutionChartReview): string {
  if (review.state === 'no_symbol') return 'Execution chart unavailable: select a symbol first.'
  if (review.state === 'no_dataset') return 'Execution chart unavailable: load a workflow dataset first.'
  if (review.state === 'symbol_mismatch') {
    return `Execution chart unavailable: loaded dataset ${review.datasetLabel || review.datasetId || '-'} does not match ${review.symbol || '-'}.`
  }
  if (review.state === 'stale_broker_review') {
    return `Execution chart loaded for ${review.symbol || '-'}, but broker review is stale as of ${formatUiTimeFromUnixMs(review.fetchedAtUnixMs)}.`
  }
  if (review.state === 'provenance_suppressed') {
    return `Execution chart loaded for ${review.symbol || '-'}. Broker-confirmed markers stay suppressed until a symbol-scoped broker review is refreshed.`
  }
  if (review.state === 'loaded_no_markers') {
    return `Execution chart loaded for ${review.symbol || '-'}. No broker-confirmed order markers are present in the current review.`
  }
  if (review.state === 'loaded_broker_markers') {
    return `Execution chart loaded for ${review.symbol || '-'} with ${review.orderCount} broker-confirmed order marker${review.orderCount === 1 ? '' : 's'}.`
  }
  if (review.state === 'broker_review_loaded_chart_unavailable') {
    return `Execution chart unavailable after broker review. symbol=${review.symbol || '-'}`
  }
  return `Execution chart sync pending. symbol=${review.symbol || '-'} orders=${review.orderCount}`
}

function deriveReconciliationState(state: ExecutionState): {
  kind: ExecutionReconciliationStateKind
  fetchedAtUnixMs: number | null
  openOrderCount: number
  optimisticOrderCount: number
} {
  const symbol = canonicalSymbol(state.symbol)
  const optimisticCount = optimisticOrderCount(state.orderRows)
  if (!symbol) {
    return {
      kind: 'disabled',
      fetchedAtUnixMs: null,
      openOrderCount: 0,
      optimisticOrderCount: optimisticCount,
    }
  }
  if (state.recoveryReviewRequired) {
    return {
      kind: 'unknown',
      fetchedAtUnixMs: null,
      openOrderCount: 0,
      optimisticOrderCount: optimisticCount,
    }
  }
  const brokerReview =
    state.brokerReview && state.brokerReview.symbol === symbol && state.brokerReview.mode === (String(state.mode || 'paper').trim() === 'live' ? 'live' : 'paper')
      ? state.brokerReview
      : null
  if (!brokerReview) {
    return {
      kind: optimisticCount > 0 ? 'optimistic_local_pending' : 'unknown',
      fetchedAtUnixMs: null,
      openOrderCount: 0,
      optimisticOrderCount: optimisticCount,
    }
  }
  const fetchedAtUnixMs = brokerReview.fetchedAtUnixMs
  const stale =
    Number.isFinite(fetchedAtUnixMs ?? Number.NaN) &&
    fetchedAtUnixMs != null &&
    fetchedAtUnixMs > 0 &&
    Date.now() - fetchedAtUnixMs > brokerReview.staleThresholdMs
  if (stale) {
    return {
      kind: 'broker_stale',
      fetchedAtUnixMs,
      openOrderCount: brokerReview.orders.openCount,
      optimisticOrderCount: optimisticCount,
    }
  }
  const hasOpenPosition = brokerReview.position.state === 'open'
  return {
    kind: hasOpenPosition || brokerReview.orders.openCount > 0 ? 'broker_confirmed_open' : 'broker_confirmed_flat',
    fetchedAtUnixMs,
    openOrderCount: brokerReview.orders.openCount,
    optimisticOrderCount: optimisticCount,
  }
}

function reviewMetaFromState(state: ExecutionState): string {
  const symbol = canonicalSymbol(state.symbol)
  if (state.recoveryReviewRequired) {
    return `Broker review: ${String(state.mode || 'paper').trim()} symbol-scoped ${symbol || '-'} pending broker refresh after the last live interruption.`
  }
  const review = deriveReconciliationState(state)
  if (review.kind === 'disabled') {
    return 'Broker review: unavailable until the selected symbol is refreshed.'
  }
  if (review.kind === 'unknown') {
    return `Broker review: ${String(state.mode || 'paper').trim()} symbol-scoped ${symbol || '-'} pending broker refresh.`
  }
  if (review.kind === 'optimistic_local_pending') {
    return `Broker review: ${String(state.mode || 'paper').trim()} symbol-scoped ${symbol || '-'} waiting for broker confirmation (${review.optimisticOrderCount} optimistic local order${review.optimisticOrderCount === 1 ? '' : 's'}).`
  }
  const asOf = review.fetchedAtUnixMs ? formatUiTimeFromUnixMs(review.fetchedAtUnixMs) : 'unknown'
  if (review.kind === 'broker_stale') {
    return `Broker review: ${String(state.mode || 'paper').trim()} symbol-scoped ${symbol || '-'} stale as of ${asOf} · ${review.openOrderCount} open orders`
  }
  if (review.kind === 'broker_confirmed_open') {
    return `Broker review: ${String(state.mode || 'paper').trim()} symbol-scoped ${symbol || '-'} broker-confirmed open as of ${asOf} · ${review.openOrderCount} open orders`
  }
  return `Broker review: ${String(state.mode || 'paper').trim()} symbol-scoped ${symbol || '-'} broker-confirmed flat as of ${asOf} · ${review.openOrderCount} open orders`
}

function reconciliationAdvisories(state: ExecutionState): string[] {
  const review = deriveReconciliationState(state)
  if (review.kind === 'optimistic_local_pending') {
    return [
      `Broker review is waiting on ${review.optimisticOrderCount} optimistic local order${review.optimisticOrderCount === 1 ? '' : 's'}. Refresh orders before the next broker-armed action.`,
    ]
  }
  if (review.kind === 'broker_stale') {
    return [
      'Broker snapshot is stale. Refresh orders before the next broker-armed action.',
    ]
  }
  if (review.kind === 'broker_confirmed_open') {
    if (review.openOrderCount > 0) {
      return [
        `Broker still has ${review.openOrderCount} open order${review.openOrderCount === 1 ? '' : 's'}. Review fills before the next broker-armed action.`,
      ]
    }
    return [
      'Broker-confirmed position is already open. Review the current position before the next broker-armed action.',
    ]
  }
  return []
}

function recoveryDatasetState(state: ExecutionState): ExecutionDatasetRecoveryState {
  if (!hasRequestedSymbol(state)) return 'no_symbol'
  const chartState = state.brokerReview?.chart.state
  if (chartState === 'no_dataset') return 'no_dataset'
  if (chartState === 'symbol_mismatch') return 'symbol_mismatch'
  if (
    chartState === 'sync_pending' ||
    chartState === 'broker_review_loaded_chart_unavailable' ||
    chartState === 'stale_broker_review' ||
    chartState === 'loaded_no_markers' ||
    chartState === 'loaded_broker_markers' ||
    chartState === 'provenance_suppressed'
  ) {
    return 'valid'
  }
  return 'unknown'
}

function recoveryBlockedReason(state: ExecutionState): string | null {
  if (!state.recoveryReviewRequired) return null
  const requestedSymbol = canonicalSymbol(state.symbol) || 'the selected symbol'
  switch (recoveryDatasetState(state)) {
    case 'no_dataset':
      return 'Load the execution dataset and refresh broker review before re-arming the auto-runner.'
    case 'symbol_mismatch':
      return `The loaded dataset does not match ${requestedSymbol}. Refresh the matching dataset and broker review before re-arming the auto-runner.`
    default:
      return 'Refresh broker review after the last live interruption before re-arming the auto-runner.'
  }
}

function recoveryStatusLabel(state: ExecutionState): string {
  if (state.recoveryReviewRequired) {
    return recoveryBlockedReason(state) || 'Broker review refresh required before re-arm.'
  }
  if (recoveryDatasetState(state) === 'no_dataset') {
    return 'Execution dataset unavailable.'
  }
  if (recoveryDatasetState(state) === 'symbol_mismatch') {
    return 'Execution dataset does not match the selected symbol.'
  }
  if (hasRequestedSymbol(state)) {
    return 'No recovery review pending.'
  }
  return 'Select one symbol to enable recovery review.'
}

function liveLifecycleStateLabel(state: ExecutionState): string {
  switch (state.liveLifecycleState) {
    case 'connecting':
      return 'Connecting'
    case 'subscribed':
      return 'Subscribed'
    case 'started':
      return 'Started'
    case 'closed':
      return 'Closed'
    case 'error':
      return 'Error'
    case 'stopped':
      return 'Stopped'
    case 'active':
      return 'Active'
    default:
      return 'Off'
  }
}

function runnerLifecycleStateLabel(state: ExecutionState): string {
  switch (state.runnerLifecycleState) {
    case 'started':
      return 'Started'
    case 'triggered':
      return 'Triggered'
    case 'queued_latest':
      return 'Queued latest'
    case 'signal_processed':
      return 'Signal processed'
    case 'order_submitted':
      return 'Order submitted'
    case 'stopped':
      return 'Stopped'
    case 'cancelled':
      return 'Cancelled'
    case 'error':
      return 'Error'
    case 'active':
      return 'Active'
    default:
      return 'Off'
  }
}

function syncReviewPresentation(state: ExecutionState): void {
  state.accountMeta = accountMetaFromReview(state.accountReview)
  state.positionMeta = positionMetaFromReview(state.brokerReview?.position ?? defaultPositionReview())
  state.chartMeta = chartMetaFromReview(state.brokerReview?.chart ?? defaultChartReview())
  state.reviewMeta = reviewMetaFromState(state)
}

function prependOptimisticOrderRows(state: ExecutionState, rows: ExecutionOrderRow[]): boolean {
  const optimistic = normalizedOrderRows(rows)
  if (!optimistic.length) return false
  const seenOrderIds = new Set<string>()
  const merged: ExecutionOrderRow[] = []
  for (const row of [...optimistic, ...state.orderRows]) {
    const orderId = String(row.cells?.[6] || '').trim()
    if (orderId && seenOrderIds.has(orderId)) continue
    if (orderId) seenOrderIds.add(orderId)
    merged.push(row)
  }
  const nextRows = markRaw(normalizedOrderRows(merged).slice(0, 50))
  const nextCount = nextRows.length
  if (orderRowsEqual(state.orderRows, nextRows) && state.orderCount === nextCount) {
    return false
  }
  state.orderRows = nextRows
  state.orderCount = nextCount
  return true
}

function markVue(state: ExecutionState): void {
  state.source = 'vue'
  state.updatedAtUnixMs = Date.now()
}

function currentCredentialPresence(state: ExecutionState): CredentialPresence {
  const key = String(state.apiKey || '').trim()
  const secret = String(state.apiSecret || '').trim()
  return {
    key,
    secret,
    any: !!key || !!secret,
    both: !!key && !!secret,
  }
}

function storedCredentialPresence(state: ExecutionState): CredentialPresence {
  const key = state.storedHasApiKey ? 'stored' : ''
  const secret = state.storedHasApiSecret ? 'stored' : ''
  return {
    key,
    secret,
    any: !!key || !!secret,
    both: !!key && !!secret,
  }
}

function hasRequestedSymbol(state: ExecutionState): boolean {
  return canonicalSymbol(state.symbol).length > 0
}

function hasValidQty(state: ExecutionState): boolean {
  const qty = Number(state.qty)
  return Number.isFinite(qty) && qty > 0
}

function hasEffectiveCredentials(state: ExecutionState): boolean {
  const current = currentCredentialPresence(state)
  if (current.any) return current.both
  return state.storedHasCredentials
}

function applyStoredCredentialStatus(
  state: ExecutionState,
  status: ExecutionStoredCredentialStatus,
): void {
  state.storedRememberAvailable = status.rememberAvailable
  state.storedHasApiKey = status.hasApiKey
  state.storedHasApiSecret = status.hasApiSecret
  state.storedHasCredentials = status.hasCredentials
  state.storedCredentialState = status.state
  state.storedCredentialBackend = status.storageBackend
  state.storedCredentialMessage = status.message
  state.storedCredentialClearFailed = false
}

function applyStoredCredentialFailure(state: ExecutionState, message: string): void {
  state.storedRememberAvailable = false
  state.storedHasApiKey = false
  state.storedHasApiSecret = false
  state.storedHasCredentials = false
  state.storedCredentialState = 'unavailable'
  state.storedCredentialBackend = ''
  state.storedCredentialMessage = String(message || '').trim() || 'Stored credential status unavailable.'
  state.storedCredentialClearFailed = false
}

function syncTypedCredentialPersistenceState(state: ExecutionState): void {
  const current = currentCredentialPresence(state)
  state.typedCredentialStorePending = !!(
    state.rememberKeys &&
    current.both &&
    state.storedRememberAvailable
  )
}

function executionKeyStateKind(state: ExecutionState): ExecutionKeyStateKind {
  const current = currentCredentialPresence(state)
  if (state.storedCredentialClearFailed) return 'clear-failed'
  if (current.any && !current.both) return 'incomplete'
  if (current.both && !state.rememberKeys) return 'session'
  if (current.both && state.rememberKeys) {
    if (!state.storedRememberAvailable) return 'remember-unavailable'
    if (state.typedCredentialStorePending) {
      return state.storedHasCredentials ? 'will-update' : 'will-store'
    }
    return state.storedHasCredentials ? 'stored' : 'will-store'
  }
  if (state.storedHasCredentials) return 'stored'
  if (!state.storedRememberAvailable) return 'remember-unavailable'
  if (state.storedCredentialState === 'incomplete') return 'incomplete'
  return 'missing'
}

function keyStateLabel(state: ExecutionState): string {
  const kind = executionKeyStateKind(state)
  if (kind === 'clear-failed') return 'Clear failed'
  if (kind === 'incomplete') return 'Incomplete'
  if (kind === 'session') return 'Session only'
  if (kind === 'stored') return 'Stored securely'
  if (kind === 'will-update') return 'Will update stored credentials'
  if (kind === 'will-store') return 'Will store on next successful broker action'
  if (kind === 'remember-unavailable') return 'Remember unavailable'
  return 'Missing'
}

function keyStateDetail(state: ExecutionState): string {
  const kind = executionKeyStateKind(state)
  if (kind === 'clear-failed') {
    return state.storedCredentialMessage || 'Stored credential clear failed. Review secure storage diagnostics and retry.'
  }
  if (kind === 'incomplete') {
    if (currentCredentialPresence(state).any) {
      return 'Complete both API key and secret before testing the account or enabling broker-armed actions.'
    }
    return state.storedCredentialMessage || 'Stored execution credentials are incomplete. Clear them and save a complete key pair again.'
  }
  if (kind === 'session') {
    if (state.storedHasCredentials) {
      return 'Entered keys are a session-only override. Stored secure credentials remain available until you explicitly clear them.'
    }
    return 'Entered keys are available for this session only. Turn on Remember keys to store them after the next successful broker-facing action.'
  }
  if (kind === 'stored') {
    return state.storedCredentialMessage || 'Remembered execution credentials are stored securely in the backend. Inputs stay blank until you type a transient override.'
  }
  if (kind === 'will-update') {
    return 'Remember keys is enabled. The typed key pair will replace the stored secure credentials after the next successful broker-facing action.'
  }
  if (kind === 'will-store') {
    return 'Remember keys is enabled. The current key pair will store securely after the next successful broker-facing action.'
  }
  if (kind === 'remember-unavailable') {
    return state.storedCredentialMessage || 'Secure storage is unavailable, so remember-mode cannot store execution credentials right now.'
  }
  return 'No stored secure credentials or session overrides are available.'
}

function primaryBusyReason(state: ExecutionState): string | null {
  if (state.pending.refreshSavedSessions) return 'Saved session evidence refresh already in progress.'
  if (state.pending.testConnection) return 'Account test already in progress.'
  if (state.pending.runOnce) return 'Manual execution already in progress.'
  if (state.pending.refreshOrders) return 'Order refresh already in progress.'
  if (state.pending.cancelOpenOrders) return 'Open-order cancellation already in progress.'
  if (state.pending.flattenPosition) return 'Position flatten already in progress.'
  if (state.pending.emergencyHalt) return 'Emergency halt already in progress.'
  if (state.pending.startLive) return 'Live feed start already in progress.'
  if (state.pending.stopLive) return 'Live feed stop already in progress.'
  if (state.pending.startRunner) return 'Auto-runner start already in progress.'
  if (state.pending.stopRunner) return 'Auto-runner stop already in progress.'
  if (state.pending.clearKeys) return 'Credential clearing already in progress.'
  return null
}

function dedupeReasons(reasons: string[]): string[] {
  return Array.from(new Set(reasons.map((value) => String(value || '').trim()).filter((value) => value.length > 0)))
}

function runOnceBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (!hasRequestedSymbol(state)) reasons.push('Enter a symbol before manual execution.')
  if (!hasValidQty(state)) reasons.push('Enter a quantity greater than zero before manual execution.')
  if (!state.dryRun && !hasEffectiveCredentials(state)) {
    reasons.push('API key + secret are required for broker-armed execution.')
  }
  return dedupeReasons(reasons)
}

function refreshOrdersBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (!hasRequestedSymbol(state)) reasons.push('Enter a symbol before broker order review.')
  if (!hasEffectiveCredentials(state)) {
    reasons.push('API key + secret are required for broker-backed order review.')
  }
  return dedupeReasons(reasons)
}

function startLiveBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (state.liveActive) {
    const liveSymbol = canonicalSymbol(state.liveSymbol)
    reasons.push(liveSymbol ? `Live candles are already active for ${liveSymbol}.` : 'Live candles are already active.')
  }
  if (!hasRequestedSymbol(state)) reasons.push('Enter a symbol before starting live candles.')
  return dedupeReasons(reasons)
}

function startRunnerBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (!hasRequestedSymbol(state)) reasons.push('Enter a symbol before starting the auto-runner.')
  const recoveryReason = recoveryBlockedReason(state)
  if (recoveryReason) reasons.push(recoveryReason)
  if (!state.liveActive) {
    reasons.push('Start live candles first. The auto-runner only reacts to active live bars.')
  }
  const requestedSymbol = canonicalSymbol(state.symbol)
  const liveSymbol = canonicalSymbol(state.liveSymbol)
  if (state.liveActive && requestedSymbol && liveSymbol && liveSymbol !== requestedSymbol) {
    reasons.push(`Live candles are active for ${liveSymbol}, not ${requestedSymbol}.`)
  }
  if (state.runnerActive) reasons.push('The auto-runner is already active.')
  if (!state.dryRun && !hasEffectiveCredentials(state)) {
    reasons.push('API key + secret are required for broker-armed auto-runner start.')
  }
  return dedupeReasons(reasons)
}

function cancelOpenOrdersBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (!hasRequestedSymbol(state)) reasons.push('Enter a symbol before cancelling open broker orders.')
  if (!hasEffectiveCredentials(state)) {
    reasons.push('API key + secret are required for broker-backed safety controls.')
  }
  if (state.runnerActive) {
    reasons.push('Stop the auto-runner before cancelling broker open orders.')
  }
  return dedupeReasons(reasons)
}

function flattenPositionBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (!hasRequestedSymbol(state)) reasons.push('Enter a symbol before flattening the broker position.')
  if (!hasEffectiveCredentials(state)) {
    reasons.push('API key + secret are required for broker-backed safety controls.')
  }
  if (state.runnerActive) {
    reasons.push('Stop the auto-runner before flattening the broker position.')
  }
  return dedupeReasons(reasons)
}

function emergencyHaltBlockedReasons(state: ExecutionState): string[] {
  const reasons: string[] = []
  const busy = primaryBusyReason(state)
  if (busy) reasons.push(busy)
  if (!state.liveActive && !state.runnerActive) {
    reasons.push('Live automation is already disarmed.')
  }
  return dedupeReasons(reasons)
}

function setAvailability(state: ExecutionState): void {
  state.isBusy = Object.values(state.pending).some(Boolean)
  const liveSymbol = canonicalSymbol(state.liveSymbol)
  const requestedSymbol = canonicalSymbol(state.symbol)
  const liveMatchesRequested = !!liveSymbol && liveSymbol === requestedSymbol
  const currentCredentials = currentCredentialPresence(state)
  const storedCredentials = storedCredentialPresence(state)
  const effectiveCredentials = hasEffectiveCredentials(state)
  const requestedSymbolReady = !!requestedSymbol
  const qtyReady = hasValidQty(state)
  const primaryBusy =
    state.pending.refreshSavedSessions ||
    state.pending.testConnection ||
    state.pending.runOnce ||
    state.pending.refreshOrders ||
    state.pending.cancelOpenOrders ||
    state.pending.flattenPosition ||
    state.pending.emergencyHalt ||
    state.pending.startLive ||
    state.pending.startRunner ||
    state.pending.clearKeys

  state.canTestConnection = !primaryBusy && effectiveCredentials
  state.canRunOnce = !primaryBusy && requestedSymbolReady && qtyReady && (state.dryRun || effectiveCredentials)
  state.canRefreshOrders = !primaryBusy && requestedSymbolReady && effectiveCredentials
  state.canCancelOpenOrders = !primaryBusy && requestedSymbolReady && effectiveCredentials && !state.runnerActive
  state.canFlattenPosition = !primaryBusy && requestedSymbolReady && effectiveCredentials && !state.runnerActive
  state.canEmergencyHalt = !primaryBusy && (state.liveActive || state.runnerActive)
  state.canStartLive = !primaryBusy && !state.liveActive && !state.pending.stopLive && requestedSymbolReady
  state.canStopLive = state.liveActive && !state.pending.stopLive
  state.canStartRunner =
    !primaryBusy &&
    !state.pending.startRunner &&
    !state.pending.stopRunner &&
    !state.runnerActive &&
    !state.recoveryReviewRequired &&
    state.liveActive &&
    liveMatchesRequested &&
    requestedSymbolReady &&
    (state.dryRun || effectiveCredentials)
  state.canStopRunner = state.runnerActive && !state.pending.stopRunner
  state.canClearKeys = !primaryBusy && (currentCredentials.any || storedCredentials.any)
}

function applyActionResult(state: ExecutionState, result: ExecutionActionResult): void {
  if (typeof result.statusLine === 'string' && result.statusLine.trim()) {
    state.statusLine = result.statusLine.trim()
  }
  if (typeof result.statusHint === 'string' && result.statusHint.trim()) {
    state.statusHint = result.statusHint.trim()
  }
  if (typeof result.liveStatusHint === 'string' && result.liveStatusHint.trim()) {
    state.liveStatusHint = result.liveStatusHint.trim()
  }
  if (typeof result.runnerStatusHint === 'string' && result.runnerStatusHint.trim()) {
    state.runnerStatusHint = result.runnerStatusHint.trim()
  }
  if (typeof result.liveLifecycleState === 'string' && result.liveLifecycleState.trim()) {
    state.liveLifecycleState = result.liveLifecycleState
  }
  if (typeof result.runnerLifecycleState === 'string' && result.runnerLifecycleState.trim()) {
    state.runnerLifecycleState = result.runnerLifecycleState
  }
  if (result.accountReview) {
    state.accountReview = result.accountReview
  }
  if (result.brokerReview) {
    state.brokerReview = result.brokerReview
    state.recoveryReviewRequired = false
  }
  if (typeof result.recoveryReviewRequired === 'boolean') {
    state.recoveryReviewRequired = result.recoveryReviewRequired
  }
  if (result.outputPreviewMeta) {
    state.outputPreviewMeta = result.outputPreviewMeta
  }
  if (typeof result.outputPreview === 'string') {
    state.outputPreview = result.outputPreview.trim() || '-'
  }
  if (Array.isArray(result.orderRows)) {
    state.orderRows = normalizedOrderRows(result.orderRows)
  }
  if (Array.isArray(result.prependOrderRows)) {
    prependOptimisticOrderRows(state, result.prependOrderRows)
  }
  if (typeof result.orderCount === 'number' && Number.isFinite(result.orderCount)) {
    state.orderCount = Math.max(0, Math.floor(result.orderCount))
  } else if (Array.isArray(result.orderRows)) {
    state.orderCount = state.orderRows.length
  } else if (Array.isArray(result.prependOrderRows)) {
    state.orderCount = state.orderRows.length
  }
  if (typeof result.liveActive === 'boolean') {
    state.liveActive = result.liveActive
  }
  if (typeof result.runnerActive === 'boolean') {
    state.runnerActive = result.runnerActive
  }
  if (typeof result.liveSymbol === 'string') {
    state.liveSymbol = result.liveSymbol
  }
  if (typeof result.liveDelayed === 'boolean') {
    state.liveDelayed = result.liveDelayed
  }
  const reconciliation = deriveReconciliationState(state)
  state.reconciliationState = reconciliation.kind
  state.reconciliationFetchedAtUnixMs = reconciliation.fetchedAtUnixMs
  state.reconciliationOpenOrderCount = reconciliation.openOrderCount
  state.reconciliationOptimisticOrderCount = reconciliation.optimisticOrderCount
  syncReviewPresentation(state)
  if (typeof result.positionMeta === 'string' && result.positionMeta.trim()) {
    state.positionMeta = result.positionMeta.trim()
  }
  if (typeof result.chartMeta === 'string' && result.chartMeta.trim()) {
    state.chartMeta = result.chartMeta.trim()
  }
  if (typeof result.accountMeta === 'string' && result.accountMeta.trim()) {
    state.accountMeta = result.accountMeta.trim()
  }
  if (typeof result.reviewMeta === 'string' && result.reviewMeta.trim()) {
    state.reviewMeta = result.reviewMeta.trim()
  }
}

interface ExecutionState {
  mode: string
  symbol: string
  qty: string
  paramSource: string
  riskStopLossPct: string
  riskTakeProfitPct: string
  manualFastLen: string
  manualSlowLen: string
  apiKey: string
  apiSecret: string
  dryRun: boolean
  rememberKeys: boolean
  storedRememberAvailable: boolean
  storedHasApiKey: boolean
  storedHasApiSecret: boolean
  storedHasCredentials: boolean
  storedCredentialState: ExecutionStoredCredentialStatus['state']
  storedCredentialBackend: string
  storedCredentialMessage: string
  storedCredentialHydrated: boolean
  storedCredentialClearFailed: boolean
  typedCredentialStorePending: boolean
  liveUseDelayed: boolean
  liveLifecycleState: ExecutionLiveLifecycleState
  runnerLifecycleState: ExecutionRunnerLifecycleState
  recoveryReviewRequired: boolean
  liveStatusHint: string
  runnerStatusHint: string
  statusHint: string
  accountReview: ExecutionAccountReview
  brokerReview: ExecutionBrokerReview | null
  outputPreviewMeta: ExecutionPreviewMeta
  reconciliationState: ExecutionReconciliationStateKind
  reconciliationFetchedAtUnixMs: number | null
  reconciliationOpenOrderCount: number
  reconciliationOptimisticOrderCount: number
  positionMeta: string
  chartMeta: string
  accountMeta: string
  reviewMeta: string
  orderCount: number
  orderRows: ExecutionOrderRow[]
  savedSessions: ExecutionSavedSessionSummary[]
  selectedSavedSessionPath: string
  savedSessionDetail: ExecutionSavedSessionDetail | null
  savedSessionStatusHint: string
  outputPreview: string
  liveActive: boolean
  runnerActive: boolean
  liveSymbol: string
  liveDelayed: boolean
  pending: ExecutionPendingState
  isBusy: boolean
  canTestConnection: boolean
  canRunOnce: boolean
  canRefreshOrders: boolean
  canCancelOpenOrders: boolean
  canFlattenPosition: boolean
  canEmergencyHalt: boolean
  canStartLive: boolean
  canStopLive: boolean
  canStartRunner: boolean
  canStopRunner: boolean
  canClearKeys: boolean
  statusLine: string
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useExecutionStore = defineStore('execution', {
  state: (): ExecutionState => {
    const initialState: ExecutionState = {
      mode: 'paper',
      symbol: '',
      qty: '1',
      paramSource: 'selected',
      riskStopLossPct: '',
      riskTakeProfitPct: '',
      manualFastLen: '',
      manualSlowLen: '',
      apiKey: '',
      apiSecret: '',
      dryRun: true,
      rememberKeys: false,
      storedRememberAvailable: true,
      storedHasApiKey: false,
      storedHasApiSecret: false,
      storedHasCredentials: false,
      storedCredentialState: 'missing',
      storedCredentialBackend: '',
      storedCredentialMessage: '',
      storedCredentialHydrated: false,
      storedCredentialClearFailed: false,
      typedCredentialStorePending: false,
      liveUseDelayed: true,
      liveLifecycleState: 'off',
      runnerLifecycleState: 'off',
      recoveryReviewRequired: false,
      liveStatusHint: 'Live candles: off',
      runnerStatusHint: 'Auto-runner: off',
      statusHint: 'Requires a loaded dataset. Optimization run is optional when using Manual parameter source.',
      accountReview: defaultAccountReview('paper'),
      brokerReview: null,
      outputPreviewMeta: defaultPreviewMeta(),
      reconciliationState: 'disabled',
      reconciliationFetchedAtUnixMs: null,
      reconciliationOpenOrderCount: 0,
      reconciliationOptimisticOrderCount: 0,
      positionMeta: 'Position: -',
      chartMeta: 'Load a dataset and refresh orders to overlay broker events.',
      accountMeta: 'Account review: not verified.',
      reviewMeta: 'Broker review: unavailable until the selected symbol is refreshed.',
      orderCount: 0,
      orderRows: [],
      savedSessions: [],
      selectedSavedSessionPath: '',
      savedSessionDetail: null,
      savedSessionStatusHint: 'No saved live session evidence loaded.',
      outputPreview: '-',
      liveActive: false,
      runnerActive: false,
      liveSymbol: '',
      liveDelayed: true,
      pending: createPendingState(),
      isBusy: false,
      canTestConnection: true,
      canRunOnce: true,
      canRefreshOrders: true,
      canCancelOpenOrders: false,
      canFlattenPosition: false,
      canEmergencyHalt: false,
      canStartLive: true,
      canStopLive: false,
      canStartRunner: false,
      canStopRunner: false,
      canClearKeys: false,
      statusLine: 'Ready.',
      source: 'unknown',
      updatedAtUnixMs: 0,
    }
    setAvailability(initialState)
    return initialState
  },

  getters: {
    brokerModeLabel: (state): string => (String(state.mode || 'paper').trim() === 'live' ? 'Live' : 'Paper'),
    submissionModeLabel: (state): string => {
      if (state.dryRun) return 'Simulation'
      return String(state.mode || 'paper').trim() === 'live' ? 'Live order' : 'Paper order'
    },
    feedStateLabel: (state): string => {
      if (!state.liveActive) return 'Off'
      const liveSymbol = canonicalSymbol(state.liveSymbol) || canonicalSymbol(state.symbol) || '-'
      const feedMode = state.liveDelayed ? 'Delayed' : 'Realtime'
      const requestedSymbol = canonicalSymbol(state.symbol)
      const mismatchSuffix =
        requestedSymbol && liveSymbol && requestedSymbol !== liveSymbol ? ` (mismatch: ${requestedSymbol})` : ''
      return `${feedMode} · ${liveSymbol}${mismatchSuffix}`
    },
    runnerStateLabel: (state): string => (state.runnerActive ? 'Active' : 'Off'),
    requestedSymbolLabel: (state): string => canonicalSymbol(state.symbol) || 'No symbol',
    liveSymbolMatchesRequested: (state): boolean => {
      const requestedSymbol = canonicalSymbol(state.symbol)
      const liveSymbol = canonicalSymbol(state.liveSymbol)
      return !!requestedSymbol && !!liveSymbol && requestedSymbol === liveSymbol
    },
    hasExecutionCredentials: (state): boolean => hasEffectiveCredentials(state),
    keyStateLabel: (state): string => keyStateLabel(state),
    keyStateDetail: (state): string => keyStateDetail(state),
    liveLifecycleLabel: (state): string => liveLifecycleStateLabel(state),
    runnerLifecycleLabel: (state): string => runnerLifecycleStateLabel(state),
    recoveryStatusLabel: (state): string => recoveryStatusLabel(state),
    refreshOrdersBlockedReasons: (state): string[] => refreshOrdersBlockedReasons(state),
    runOnceActionLabel: (state): string => {
      if (state.dryRun) return 'Simulate once'
      return String(state.mode || 'paper').trim() === 'live' ? 'Submit live order once' : 'Submit paper order once'
    },
    runOnceActionDetail: (state): string => {
      const symbol = canonicalSymbol(state.symbol) || 'the selected symbol'
      if (state.dryRun) {
        return `Runs the current strategy against ${symbol} without sending a broker order.`
      }
      return String(state.mode || 'paper').trim() === 'live'
        ? `Can submit a live broker order for ${symbol}.`
        : `Can submit a paper broker order for ${symbol}.`
    },
    runOnceBlockedReasons: (state): string[] => runOnceBlockedReasons(state),
    startLiveBlockedReasons: (state): string[] => startLiveBlockedReasons(state),
    startRunnerBlockedReasons: (state): string[] => startRunnerBlockedReasons(state),
    cancelOpenOrdersBlockedReasons: (state): string[] => cancelOpenOrdersBlockedReasons(state),
    flattenPositionBlockedReasons: (state): string[] => flattenPositionBlockedReasons(state),
    emergencyHaltBlockedReasons: (state): string[] => emergencyHaltBlockedReasons(state),
    reconciliationAdvisories: (state): string[] => reconciliationAdvisories(state),
  },

  actions: {
    applyLegacySnapshot(snapshot: Partial<ExecutionState>): void {
      this.mode = String(snapshot.mode ?? this.mode ?? 'paper').trim() || 'paper'
      this.symbol = String(snapshot.symbol ?? this.symbol ?? '').trim()
      this.qty = String(snapshot.qty ?? this.qty ?? '1').trim() || '1'
      this.paramSource = String(snapshot.paramSource ?? this.paramSource ?? 'selected').trim() || 'selected'
      this.riskStopLossPct = String(snapshot.riskStopLossPct ?? this.riskStopLossPct ?? '').trim()
      this.riskTakeProfitPct = String(snapshot.riskTakeProfitPct ?? this.riskTakeProfitPct ?? '').trim()
      this.manualFastLen = String(snapshot.manualFastLen ?? this.manualFastLen ?? '').trim()
      this.manualSlowLen = String(snapshot.manualSlowLen ?? this.manualSlowLen ?? '').trim()
      this.dryRun = Boolean(snapshot.dryRun ?? this.dryRun)
      this.rememberKeys = Boolean(snapshot.rememberKeys ?? this.rememberKeys)
      this.liveUseDelayed = Boolean(snapshot.liveUseDelayed ?? this.liveUseDelayed)
      this.liveLifecycleState = String((snapshot as Partial<ExecutionState>).liveLifecycleState ?? this.liveLifecycleState ?? 'off').trim() as ExecutionLiveLifecycleState || 'off'
      this.runnerLifecycleState = String((snapshot as Partial<ExecutionState>).runnerLifecycleState ?? this.runnerLifecycleState ?? 'off').trim() as ExecutionRunnerLifecycleState || 'off'
      this.recoveryReviewRequired = Boolean((snapshot as Partial<ExecutionState>).recoveryReviewRequired ?? false)
      this.liveStatusHint = String(snapshot.liveStatusHint ?? this.liveStatusHint ?? 'Live candles: off').trim() || 'Live candles: off'
      this.runnerStatusHint = String(snapshot.runnerStatusHint ?? this.runnerStatusHint ?? 'Auto-runner: off').trim() || 'Auto-runner: off'
      this.statusHint = String(snapshot.statusHint ?? this.statusHint ?? '-').trim() || '-'
      this.accountReview = defaultAccountReview(String(snapshot.mode ?? this.mode ?? 'paper').trim() === 'live' ? 'live' : 'paper')
      this.brokerReview = null
      this.outputPreviewMeta = defaultPreviewMeta()
      this.reconciliationState = 'disabled'
      this.reconciliationFetchedAtUnixMs = null
      this.reconciliationOpenOrderCount = 0
      this.reconciliationOptimisticOrderCount = 0
      this.positionMeta = String(snapshot.positionMeta ?? this.positionMeta ?? 'Position: -').trim() || 'Position: -'
      this.chartMeta = String(snapshot.chartMeta ?? this.chartMeta ?? '-').trim() || '-'
      this.accountMeta = String(snapshot.accountMeta ?? this.accountMeta ?? 'Account review: not verified.').trim() || 'Account review: not verified.'
      this.reviewMeta = String(snapshot.reviewMeta ?? this.reviewMeta ?? 'Broker review: unavailable until the selected symbol is refreshed.').trim() || 'Broker review: unavailable until the selected symbol is refreshed.'
      this.orderCount = Math.max(0, Math.floor(Number(snapshot.orderCount ?? this.orderCount ?? 0)))
      this.orderRows = normalizedOrderRows(snapshot.orderRows ?? this.orderRows ?? [])
      this.reconciliationOptimisticOrderCount = optimisticOrderCount(this.orderRows)
      this.outputPreview = String(snapshot.outputPreview ?? this.outputPreview ?? '-').trim() || '-'
      this.liveActive = Boolean(snapshot.liveActive ?? (snapshot.canStartLive != null ? !snapshot.canStartLive : this.liveActive))
      this.runnerActive = Boolean(snapshot.runnerActive ?? (snapshot.canStartRunner != null ? !snapshot.canStartRunner : this.runnerActive))
      this.liveSymbol = String((snapshot as Partial<ExecutionState>).liveSymbol ?? this.liveSymbol ?? '').trim()
      this.liveDelayed = Boolean((snapshot as Partial<ExecutionState>).liveDelayed ?? this.liveDelayed)
      this.pending = createPendingState()
      this.canTestConnection = Boolean(snapshot.canTestConnection ?? this.canTestConnection)
      this.canRunOnce = Boolean(snapshot.canRunOnce ?? this.canRunOnce)
      this.canRefreshOrders = Boolean(snapshot.canRefreshOrders ?? this.canRefreshOrders)
      this.canCancelOpenOrders = Boolean((snapshot as Partial<ExecutionState>).canCancelOpenOrders ?? this.canCancelOpenOrders)
      this.canFlattenPosition = Boolean((snapshot as Partial<ExecutionState>).canFlattenPosition ?? this.canFlattenPosition)
      this.canEmergencyHalt = Boolean((snapshot as Partial<ExecutionState>).canEmergencyHalt ?? this.canEmergencyHalt)
      this.canStartLive = Boolean(snapshot.canStartLive ?? this.canStartLive)
      this.canStopLive = Boolean(snapshot.canStopLive ?? this.canStopLive)
      this.canStartRunner = Boolean(snapshot.canStartRunner ?? this.canStartRunner)
      this.canStopRunner = Boolean(snapshot.canStopRunner ?? this.canStopRunner)
      this.canClearKeys = Boolean(snapshot.canClearKeys ?? this.canClearKeys)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      syncTypedCredentialPersistenceState(this)
      setAvailability(this)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        if (key === 'execSymbol') {
          this.brokerReview = null
          this.recoveryReviewRequired = false
          this.reconciliationState = 'disabled'
          this.reconciliationFetchedAtUnixMs = null
          this.reconciliationOpenOrderCount = 0
          this.orderRows = []
          this.orderCount = 0
          this.reconciliationOptimisticOrderCount = 0
          syncReviewPresentation(this)
        }
        if (key === 'execApiKey' || key === 'execApiSecret') {
          this.storedCredentialClearFailed = false
          syncTypedCredentialPersistenceState(this)
        }
        setAvailability(this)
        markVue(this)
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        if (key === 'execMode') {
          this.accountReview = defaultAccountReview(String(this.mode || 'paper').trim() === 'live' ? 'live' : 'paper')
          this.brokerReview = null
          this.outputPreviewMeta = defaultPreviewMeta()
          this.recoveryReviewRequired = false
          this.reconciliationState = hasRequestedSymbol(this) ? 'unknown' : 'disabled'
          this.reconciliationFetchedAtUnixMs = null
          this.reconciliationOpenOrderCount = 0
          this.orderRows = []
          this.orderCount = 0
          this.reconciliationOptimisticOrderCount = 0
          syncReviewPresentation(this)
        }
        setAvailability(this)
        markVue(this)
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
        if (key === 'execRememberKeys') {
          if (next && !this.storedRememberAvailable) {
            this.rememberKeys = false
          }
          this.storedCredentialClearFailed = false
          syncTypedCredentialPersistenceState(this)
        }
        setAvailability(this)
        markVue(this)
      }
    },

    buildActionOptions() {
      return {
        mode: this.mode,
        symbol: this.symbol,
        qty: this.qty,
        paramSource: this.paramSource,
        riskStopLossPct: this.riskStopLossPct,
        riskTakeProfitPct: this.riskTakeProfitPct,
        manualFastLen: this.manualFastLen,
        manualSlowLen: this.manualSlowLen,
        dryRun: this.dryRun,
        rememberKeys: this.rememberKeys,
        apiKey: this.apiKey,
        apiSecret: this.apiSecret,
        liveUseDelayed: this.liveUseDelayed,
      }
    },

    async hydrateStoredCredentialStatus(): Promise<void> {
      const host = runtimeHost(this)
      if (this.storedCredentialHydrated) return
      if (!host._credentialHydrationPromise) {
        host._credentialHydrationPromise = (async () => {
          try {
            const status = await migrateLegacyStoredExecutionKeysToSecureStorage()
            applyStoredCredentialStatus(this, status)
            if (!currentCredentialPresence(this).any && status.rememberAvailable && status.hasCredentials) {
              this.rememberKeys = true
            }
          } catch (error) {
            applyStoredCredentialFailure(this, String(error || '').trim())
          } finally {
            this.storedCredentialHydrated = true
            syncTypedCredentialPersistenceState(this)
            setAvailability(this)
            markVue(this)
          }
        })().finally(() => {
          host._credentialHydrationPromise = null
        })
      }
      await host._credentialHydrationPromise
    },

    async refreshStoredCredentialStatus(): Promise<void> {
      try {
        const status = await getStoredExecutionCredentialStatus()
        applyStoredCredentialStatus(this, status)
        this.storedCredentialHydrated = true
      } catch (error) {
        applyStoredCredentialFailure(this, String(error || '').trim())
        this.storedCredentialHydrated = true
        throw error
      } finally {
        syncTypedCredentialPersistenceState(this)
        setAvailability(this)
        markVue(this)
      }
    },

    async refreshSavedSessions(): Promise<void> {
      if (this.pending.refreshSavedSessions) return
      this.pending.refreshSavedSessions = true
      this.savedSessionStatusHint = 'Refreshing saved live session evidence...'
      setAvailability(this)
      markVue(this)
      try {
        const sessions = await listSavedLiveSessionsAction()
        this.savedSessions = markRaw(sessions.slice(0, 20))
        if (!sessions.length) {
          this.selectedSavedSessionPath = ''
          this.savedSessionDetail = null
          this.savedSessionStatusHint = 'No saved live session evidence found.'
          return
        }
        const selectedPath = sessions.some((session) => session.path === this.selectedSavedSessionPath)
          ? this.selectedSavedSessionPath
          : (sessions[0]?.path || '')
        this.selectedSavedSessionPath = selectedPath
        this.savedSessionDetail = await loadSavedLiveSessionDetailAction(selectedPath)
        this.savedSessionStatusHint = `Loaded ${sessions.length} saved live session artifact${sessions.length === 1 ? '' : 's'}.`
      } catch (error) {
        this.savedSessionStatusHint = String(error || '').trim() || 'Saved live session evidence refresh failed.'
      } finally {
        this.pending.refreshSavedSessions = false
        setAvailability(this)
        markVue(this)
      }
    },

    async selectSavedSession(path: string): Promise<void> {
      const nextPath = String(path || '').trim()
      if (!nextPath) {
        this.selectedSavedSessionPath = ''
        this.savedSessionDetail = null
        this.savedSessionStatusHint = 'No saved live session selected.'
        markVue(this)
        return
      }
      if (this.pending.refreshSavedSessions) return
      this.pending.refreshSavedSessions = true
      this.selectedSavedSessionPath = nextPath
      this.savedSessionStatusHint = 'Loading saved live session evidence...'
      setAvailability(this)
      markVue(this)
      try {
        this.savedSessionDetail = await loadSavedLiveSessionDetailAction(nextPath)
        this.savedSessionStatusHint = 'Saved live session evidence loaded.'
      } catch (error) {
        this.savedSessionStatusHint = String(error || '').trim() || 'Saved live session load failed.'
      } finally {
        this.pending.refreshSavedSessions = false
        setAvailability(this)
        markVue(this)
      }
    },

    async ensureRuntimeSubscriptions(): Promise<void> {
      const host = runtimeHost(this)
      if (host._runtimeSubscriptionsTeardown) return
      if (!host._runtimeSubscriptionsPromise) {
        host._runtimeSubscriptionsPromise = subscribeExecutionRuntimeEvents({
          onPolygonLiveStatus: (event: ParsedLiveStatus) => this.applyPolygonLiveStatusEvent(event),
          onRunnerEvent: (event: ParsedRunnerEvent) => this.applyLiveExecutionRunnerEvent(event),
        })
          .then((teardown) => {
            host._runtimeSubscriptionsTeardown = teardown
          })
          .finally(() => {
            host._runtimeSubscriptionsPromise = null
          })
      }
      await host._runtimeSubscriptionsPromise
    },

    beginAction(key: ExecutionPendingKey, statusLine: string): void {
      if (statusLine.trim()) this.statusLine = statusLine
      this.pending[key] = true
      setAvailability(this)
      markVue(this)
    },

    finishAction(key: ExecutionPendingKey): void {
      this.pending[key] = false
      setAvailability(this)
      markVue(this)
    },

    failAction(prefix: string, error: unknown): void {
      const msg = redactExecutionCredentialText(this, error) || 'Unknown execution error.'
      this.statusLine = `${prefix}: ${msg}`
      this.statusHint = this.statusLine
      this.outputPreview = msg
      this.outputPreviewMeta = {
        sourceCommand: 'none',
        scopeLabel: `${prefix} error`,
        generatedAtUnixMs: Date.now(),
      }
      markVue(this)
    },

    applyActionResult(result: ExecutionActionResult): void {
      applyActionResult(this, result)
      setAvailability(this)
      markVue(this)
    },

    applyPolygonLiveStatusEvent(event: ParsedLiveStatus): void {
      const nextLiveSymbol = event.active ? event.symbol : ''
      const nextLiveDelayed = event.active ? event.delayed : false
      const nextLiveLifecycleState = (event.status || (event.active ? 'active' : 'off')) as ExecutionLiveLifecycleState
      let changed = false
      if (this.liveActive !== event.active) {
        this.liveActive = event.active
        changed = true
      }
      if (this.liveSymbol !== nextLiveSymbol) {
        this.liveSymbol = nextLiveSymbol
        changed = true
      }
      if (this.liveDelayed !== nextLiveDelayed) {
        this.liveDelayed = nextLiveDelayed
        changed = true
      }
      if (this.liveLifecycleState !== nextLiveLifecycleState) {
        this.liveLifecycleState = nextLiveLifecycleState
        changed = true
      }
      if (this.liveStatusHint !== event.hint) {
        this.liveStatusHint = event.hint
        changed = true
      }
      if (event.message) {
        if (this.statusLine !== event.message) {
          this.statusLine = event.message
          changed = true
        }
        if (this.statusHint !== event.message) {
          this.statusHint = event.message
          changed = true
        }
      }
      if ((event.status === 'closed' || event.status === 'error') && !this.dryRun) {
        if (!this.recoveryReviewRequired) {
          this.recoveryReviewRequired = true
          changed = true
        }
      } else if (event.status === 'stopped' && this.recoveryReviewRequired) {
        this.recoveryReviewRequired = false
        changed = true
      }
      if (!changed) return
      const reconciliation = deriveReconciliationState(this)
      this.reconciliationState = reconciliation.kind
      this.reconciliationFetchedAtUnixMs = reconciliation.fetchedAtUnixMs
      this.reconciliationOpenOrderCount = reconciliation.openOrderCount
      this.reconciliationOptimisticOrderCount = reconciliation.optimisticOrderCount
      syncReviewPresentation(this)
      setAvailability(this)
      markVue(this)
    },

    applyLiveExecutionRunnerEvent(event: ParsedRunnerEvent): void {
      const nextRunnerLifecycleState = (event.status || (event.active ? 'active' : 'off')) as ExecutionRunnerLifecycleState
      let changed = false
      if (typeof event.active === 'boolean') {
        if (this.runnerActive !== event.active) {
          this.runnerActive = event.active
          changed = true
        }
      }
      if (this.runnerLifecycleState !== nextRunnerLifecycleState) {
        this.runnerLifecycleState = nextRunnerLifecycleState
        changed = true
      }
      if (this.runnerStatusHint !== event.hint) {
        this.runnerStatusHint = event.hint
        changed = true
      }
      if (event.message) {
        if (this.statusLine !== event.message) {
          this.statusLine = event.message
          changed = true
        }
        if (this.statusHint !== event.message) {
          this.statusHint = event.message
          changed = true
        }
      }
      if (event.execution) {
        const optimisticRow = optimisticOrderRowFromExecution(event.execution)
        if (optimisticRow) {
          changed = prependOptimisticOrderRows(this, [optimisticRow]) || changed
        }
      }
      if (
        !this.dryRun &&
        (event.status === 'stopped' || event.status === 'cancelled') &&
        (this.liveLifecycleState === 'closed' || this.liveLifecycleState === 'error') &&
        !this.recoveryReviewRequired
      ) {
        this.recoveryReviewRequired = true
        changed = true
      }
      if (!changed) return
      const reconciliation = deriveReconciliationState(this)
      this.reconciliationState = reconciliation.kind
      this.reconciliationFetchedAtUnixMs = reconciliation.fetchedAtUnixMs
      this.reconciliationOpenOrderCount = reconciliation.openOrderCount
      this.reconciliationOptimisticOrderCount = reconciliation.optimisticOrderCount
      syncReviewPresentation(this)
      setAvailability(this)
      markVue(this)
    },

    async refreshStatuses(): Promise<void> {
      if (this.pending.refreshStatuses) return
      await this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('refreshStatuses', 'Refreshing execution status...')
      try {
        const result = await refreshStatusesAction()
        this.applyActionResult(result)
      } catch (error) {
        this.failAction('Execution status refresh failed', error)
      } finally {
        this.finishAction('refreshStatuses')
      }
    },

    testConnection(): void {
      if (!this.canTestConnection) return
      const persistedTypedCredentials = this.rememberKeys && currentCredentialPresence(this).both
      this.beginAction('testConnection', 'Testing account...')
      void testConnectionAction(this.buildActionOptions())
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch(() => {})
          if (persistedTypedCredentials) this.typedCredentialStorePending = false
        })
        .catch((error) => this.failAction('Connection test failed', error))
        .finally(() => this.finishAction('testConnection'))
    },

    runOnce(): void {
      if (!this.canRunOnce) return
      const persistedTypedCredentials = this.rememberKeys && currentCredentialPresence(this).both
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('runOnce', 'Executing signal...')
      void runOnceAction(this.buildActionOptions())
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch(() => {})
          if (persistedTypedCredentials) this.typedCredentialStorePending = false
        })
        .catch((error) => this.failAction('Execution failed', error))
        .finally(() => this.finishAction('runOnce'))
    },

    refreshOrders(): void {
      if (!this.canRefreshOrders) return
      const persistedTypedCredentials = this.rememberKeys && currentCredentialPresence(this).both
      this.beginAction('refreshOrders', 'Refreshing broker orders...')
      void refreshOrdersAction(this.buildActionOptions())
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch(() => {})
          if (persistedTypedCredentials) this.typedCredentialStorePending = false
        })
        .catch((error) => this.failAction('Order refresh failed', error))
        .finally(() => this.finishAction('refreshOrders'))
    },

    cancelOpenOrders(): void {
      if (!this.canCancelOpenOrders) return
      const persistedTypedCredentials = this.rememberKeys && currentCredentialPresence(this).both
      this.beginAction('cancelOpenOrders', 'Cancelling open broker orders...')
      void cancelOpenOrdersAction(this.buildActionOptions())
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch(() => {})
          if (persistedTypedCredentials) this.typedCredentialStorePending = false
        })
        .catch((error) => this.failAction('Cancel open orders failed', error))
        .finally(() => this.finishAction('cancelOpenOrders'))
    },

    flattenPosition(): void {
      if (!this.canFlattenPosition) return
      const persistedTypedCredentials = this.rememberKeys && currentCredentialPresence(this).both
      this.beginAction('flattenPosition', 'Flattening broker position...')
      void flattenPositionAction(this.buildActionOptions())
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch(() => {})
          if (persistedTypedCredentials) this.typedCredentialStorePending = false
        })
        .catch((error) => this.failAction('Flatten position failed', error))
        .finally(() => this.finishAction('flattenPosition'))
    },

    emergencyHalt(): void {
      if (!this.canEmergencyHalt) return
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('emergencyHalt', 'Applying emergency halt...')
      void emergencyHaltAction()
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Emergency halt failed', error))
        .finally(() => this.finishAction('emergencyHalt'))
    },

    startLive(): void {
      if (!this.canStartLive) return
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('startLive', 'Starting live candles...')
      void startLiveAction(this.buildActionOptions())
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Live start failed', error))
        .finally(() => this.finishAction('startLive'))
    },

    stopLive(): void {
      if (!this.canStopLive) return
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('stopLive', 'Stopping live candles...')
      void stopLiveAction()
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Live stop failed', error))
        .finally(() => this.finishAction('stopLive'))
    },

    startRunner(): void {
      if (!this.canStartRunner) return
      const persistedTypedCredentials = this.rememberKeys && currentCredentialPresence(this).both
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('startRunner', 'Starting auto-runner...')
      void startRunnerAction(this.buildActionOptions())
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch(() => {})
          if (persistedTypedCredentials) this.typedCredentialStorePending = false
        })
        .catch((error) => this.failAction('Auto-runner start failed', error))
        .finally(() => this.finishAction('startRunner'))
    },

    stopRunner(): void {
      if (!this.canStopRunner) return
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('stopRunner', 'Stopping auto-runner...')
      void stopRunnerAction()
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Auto-runner stop failed', error))
        .finally(() => this.finishAction('stopRunner'))
    },

    clearKeys(): void {
      if (!this.canClearKeys) return
      this.beginAction('clearKeys', 'Clearing execution keys...')
      this.apiKey = ''
      this.apiSecret = ''
      this.typedCredentialStorePending = false
      this.storedCredentialClearFailed = false
      void clearKeysAction()
        .then(async (result) => {
          this.applyActionResult(result)
          await this.refreshStoredCredentialStatus().catch((error) => {
            this.storedCredentialClearFailed = true
            this.storedCredentialMessage =
              String(error || '').trim() || 'Stored credential clear failed.'
            setAvailability(this)
            markVue(this)
          })
        })
        .catch((error) => {
          this.storedCredentialClearFailed = true
          this.storedCredentialMessage =
            String(error || '').trim() || 'Stored credential clear failed.'
          this.failAction('Clear keys failed', error)
        })
        .finally(() => this.finishAction('clearKeys'))
    },
  },
})

