import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import {
  canonicalSymbol,
  clearKeys as clearKeysAction,
  loadPersistedExecutionKeys,
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
} from '../services/executionService'

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
  | 'refreshStatuses'
  | 'testConnection'
  | 'runOnce'
  | 'refreshOrders'
  | 'startLive'
  | 'stopLive'
  | 'startRunner'
  | 'stopRunner'
  | 'clearKeys'

interface ExecutionPendingState {
  refreshStatuses: boolean
  testConnection: boolean
  runOnce: boolean
  refreshOrders: boolean
  startLive: boolean
  stopLive: boolean
  startRunner: boolean
  stopRunner: boolean
  clearKeys: boolean
}

function createPendingState(): ExecutionPendingState {
  return {
    refreshStatuses: false,
    testConnection: false,
    runOnce: false,
    refreshOrders: false,
    startLive: false,
    stopLive: false,
    startRunner: false,
    stopRunner: false,
    clearKeys: false,
  }
}

interface ExecutionStoreRuntimeHost {
  _runtimeSubscriptionsPromise?: Promise<void> | null
  _runtimeSubscriptionsTeardown?: (() => void) | null
}

function runtimeHost(store: unknown): ExecutionStoreRuntimeHost {
  return store as ExecutionStoreRuntimeHost
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

function setAvailability(state: ExecutionState): void {
  state.isBusy = Object.values(state.pending).some(Boolean)
  const liveSymbol = canonicalSymbol(state.liveSymbol)
  const requestedSymbol = canonicalSymbol(state.symbol)
  const liveMatchesRequested = !!liveSymbol && liveSymbol === requestedSymbol
  const primaryBusy =
    state.pending.testConnection ||
    state.pending.runOnce ||
    state.pending.refreshOrders ||
    state.pending.startLive ||
    state.pending.startRunner ||
    state.pending.clearKeys

  state.canTestConnection = !primaryBusy
  state.canRunOnce = !primaryBusy
  state.canRefreshOrders = !primaryBusy
  state.canStartLive = !primaryBusy && !state.liveActive && !state.pending.stopLive
  state.canStopLive = state.liveActive && !state.pending.stopLive
  state.canStartRunner =
    !primaryBusy &&
    !state.pending.startRunner &&
    !state.pending.stopRunner &&
    !state.runnerActive &&
    state.liveActive &&
    liveMatchesRequested
  state.canStopRunner = state.runnerActive && !state.pending.stopRunner
  state.canClearKeys = !primaryBusy
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
  if (typeof result.positionMeta === 'string' && result.positionMeta.trim()) {
    state.positionMeta = result.positionMeta.trim()
  }
  if (typeof result.chartMeta === 'string' && result.chartMeta.trim()) {
    state.chartMeta = result.chartMeta.trim()
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
  liveUseDelayed: boolean
  liveStatusHint: string
  runnerStatusHint: string
  statusHint: string
  positionMeta: string
  chartMeta: string
  orderCount: number
  orderRows: ExecutionOrderRow[]
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
    const persisted = loadPersistedExecutionKeys()
    const hasPersistedKeys = !!(persisted.apiKey || persisted.apiSecret)
    return {
      mode: 'paper',
      symbol: '',
      qty: '1',
      paramSource: 'selected',
      riskStopLossPct: '',
      riskTakeProfitPct: '',
      manualFastLen: '',
      manualSlowLen: '',
      apiKey: persisted.apiKey,
      apiSecret: persisted.apiSecret,
      dryRun: true,
      rememberKeys: hasPersistedKeys,
      liveUseDelayed: true,
      liveStatusHint: 'Live candles: off',
      runnerStatusHint: 'Auto-runner: off',
      statusHint: 'Requires a loaded dataset. Optimization run is optional when using Manual parameter source.',
      positionMeta: 'Position: -',
      chartMeta: 'Load a dataset and refresh orders to overlay broker events.',
      orderCount: 0,
      orderRows: [],
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
      canStartLive: true,
      canStopLive: false,
      canStartRunner: false,
      canStopRunner: false,
      canClearKeys: true,
      statusLine: 'Ready.',
      source: 'unknown',
      updatedAtUnixMs: 0,
    }
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
      this.apiKey = String(snapshot.apiKey ?? this.apiKey ?? '').trim()
      this.apiSecret = String(snapshot.apiSecret ?? this.apiSecret ?? '').trim()
      this.dryRun = Boolean(snapshot.dryRun ?? this.dryRun)
      this.rememberKeys = Boolean(snapshot.rememberKeys ?? this.rememberKeys)
      this.liveUseDelayed = Boolean(snapshot.liveUseDelayed ?? this.liveUseDelayed)
      this.liveStatusHint = String(snapshot.liveStatusHint ?? this.liveStatusHint ?? 'Live candles: off').trim() || 'Live candles: off'
      this.runnerStatusHint = String(snapshot.runnerStatusHint ?? this.runnerStatusHint ?? 'Auto-runner: off').trim() || 'Auto-runner: off'
      this.statusHint = String(snapshot.statusHint ?? this.statusHint ?? '-').trim() || '-'
      this.positionMeta = String(snapshot.positionMeta ?? this.positionMeta ?? 'Position: -').trim() || 'Position: -'
      this.chartMeta = String(snapshot.chartMeta ?? this.chartMeta ?? '-').trim() || '-'
      this.orderCount = Math.max(0, Math.floor(Number(snapshot.orderCount ?? this.orderCount ?? 0)))
      this.orderRows = normalizedOrderRows(snapshot.orderRows ?? this.orderRows ?? [])
      this.outputPreview = String(snapshot.outputPreview ?? this.outputPreview ?? '-').trim() || '-'
      this.liveActive = Boolean(snapshot.liveActive ?? (snapshot.canStartLive != null ? !snapshot.canStartLive : this.liveActive))
      this.runnerActive = Boolean(snapshot.runnerActive ?? (snapshot.canStartRunner != null ? !snapshot.canStartRunner : this.runnerActive))
      this.liveSymbol = String((snapshot as Partial<ExecutionState>).liveSymbol ?? this.liveSymbol ?? '').trim()
      this.liveDelayed = Boolean((snapshot as Partial<ExecutionState>).liveDelayed ?? this.liveDelayed)
      this.pending = createPendingState()
      this.canTestConnection = Boolean(snapshot.canTestConnection ?? this.canTestConnection)
      this.canRunOnce = Boolean(snapshot.canRunOnce ?? this.canRunOnce)
      this.canRefreshOrders = Boolean(snapshot.canRefreshOrders ?? this.canRefreshOrders)
      this.canStartLive = Boolean(snapshot.canStartLive ?? this.canStartLive)
      this.canStopLive = Boolean(snapshot.canStopLive ?? this.canStopLive)
      this.canStartRunner = Boolean(snapshot.canStartRunner ?? this.canStartRunner)
      this.canStopRunner = Boolean(snapshot.canStopRunner ?? this.canStopRunner)
      this.canClearKeys = Boolean(snapshot.canClearKeys ?? this.canClearKeys)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        setAvailability(this)
        markVue(this)
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        setAvailability(this)
        markVue(this)
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
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
      const msg = String(error || '').trim() || 'Unknown execution error.'
      this.statusLine = `${prefix}: ${msg}`
      this.statusHint = this.statusLine
      this.outputPreview = msg
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
      if (!changed) return
      setAvailability(this)
      markVue(this)
    },

    applyLiveExecutionRunnerEvent(event: ParsedRunnerEvent): void {
      let changed = false
      if (typeof event.active === 'boolean') {
        if (this.runnerActive !== event.active) {
          this.runnerActive = event.active
          changed = true
        }
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
      if (!changed) return
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
      this.beginAction('testConnection', 'Testing account...')
      void testConnectionAction(this.buildActionOptions())
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Connection test failed', error))
        .finally(() => this.finishAction('testConnection'))
    },

    runOnce(): void {
      if (!this.canRunOnce) return
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('runOnce', 'Executing signal...')
      void runOnceAction(this.buildActionOptions())
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Execution failed', error))
        .finally(() => this.finishAction('runOnce'))
    },

    refreshOrders(): void {
      if (!this.canRefreshOrders) return
      this.beginAction('refreshOrders', 'Refreshing broker orders...')
      void refreshOrdersAction(this.buildActionOptions())
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Order refresh failed', error))
        .finally(() => this.finishAction('refreshOrders'))
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
      void this.ensureRuntimeSubscriptions().catch(() => {})
      this.beginAction('startRunner', 'Starting auto-runner...')
      void startRunnerAction(this.buildActionOptions())
        .then((result) => this.applyActionResult(result))
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
      void clearKeysAction()
        .then((result) => this.applyActionResult(result))
        .catch((error) => this.failAction('Clear keys failed', error))
        .finally(() => this.finishAction('clearKeys'))
    },
  },
})

