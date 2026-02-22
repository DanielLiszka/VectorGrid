import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import {
  invokeLegacyExecutionClearKeysRuntime,
  invokeLegacyExecutionLiveStartRuntime,
  invokeLegacyExecutionLiveStopRuntime,
  invokeLegacyExecutionRefreshRuntime,
  invokeLegacyExecutionRunOnceRuntime,
  invokeLegacyExecutionRunnerStartRuntime,
  invokeLegacyExecutionRunnerStopRuntime,
  invokeLegacyExecutionTestConnectionRuntime,
} from '../legacy/actions'

function updateLegacyTextField(field: string, value: string, eventType: 'input' | 'change'): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el || el.disabled) return
  if (String(el.value ?? '') !== value) el.value = value
  el.dispatchEvent(new Event(eventType, { bubbles: true }))
}

function updateLegacyCheckboxField(field: string, checked: boolean): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | null
  if (!el || el.disabled) return
  if (String(el.type || '').trim().toLowerCase() !== 'checkbox') return
  if (el.checked !== checked) el.checked = checked
  el.dispatchEvent(new Event('change', { bubbles: true }))
}

export interface ExecutionOrderRow {
  cells: string[]
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

interface ExecutionState {
  mode: string
  symbol: string
  qty: string
  paramSource: string
  riskStopLossPct: string
  riskTakeProfitPct: string
  manualFastLen: string
  manualSlowLen: string
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
  canTestConnection: boolean
  canRunOnce: boolean
  canRefreshOrders: boolean
  canStartLive: boolean
  canStopLive: boolean
  canStartRunner: boolean
  canStopRunner: boolean
  canClearKeys: boolean
  statusLine: string
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useExecutionStore = defineStore('execution', {
  state: (): ExecutionState => ({
    mode: 'paper',
    symbol: '',
    qty: '1',
    paramSource: 'selected',
    riskStopLossPct: '',
    riskTakeProfitPct: '',
    manualFastLen: '',
    manualSlowLen: '',
    dryRun: true,
    rememberKeys: false,
    liveUseDelayed: true,
    liveStatusHint: 'Live candles: off',
    runnerStatusHint: 'Auto-runner: off',
    statusHint: 'Requires a loaded dataset. Optimization run is optional when using Manual parameter source.',
    positionMeta: 'Position: -',
    chartMeta: 'Load a dataset and refresh orders to overlay broker events.',
    orderCount: 0,
    orderRows: [],
    outputPreview: '-',
    canTestConnection: true,
    canRunOnce: true,
    canRefreshOrders: true,
    canStartLive: true,
    canStopLive: true,
    canStartRunner: true,
    canStopRunner: true,
    canClearKeys: true,
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

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
      this.liveStatusHint = String(snapshot.liveStatusHint ?? this.liveStatusHint ?? 'Live candles: off').trim() || 'Live candles: off'
      this.runnerStatusHint = String(snapshot.runnerStatusHint ?? this.runnerStatusHint ?? 'Auto-runner: off').trim() || 'Auto-runner: off'
      this.statusHint = String(snapshot.statusHint ?? this.statusHint ?? '-').trim() || '-'
      this.positionMeta = String(snapshot.positionMeta ?? this.positionMeta ?? 'Position: -').trim() || 'Position: -'
      this.chartMeta = String(snapshot.chartMeta ?? this.chartMeta ?? '-').trim() || '-'
      this.orderCount = Math.max(0, Math.floor(Number(snapshot.orderCount ?? this.orderCount ?? 0)))
      this.orderRows = normalizedOrderRows(snapshot.orderRows ?? this.orderRows ?? [])
      this.outputPreview = String(snapshot.outputPreview ?? this.outputPreview ?? '-').trim() || '-'
      this.canTestConnection = Boolean(snapshot.canTestConnection ?? this.canTestConnection)
      this.canRunOnce = Boolean(snapshot.canRunOnce ?? this.canRunOnce)
      this.canRefreshOrders = Boolean(snapshot.canRefreshOrders ?? this.canRefreshOrders)
      this.canStartLive = Boolean(snapshot.canStartLive ?? this.canStartLive)
      this.canStopLive = Boolean(snapshot.canStopLive ?? this.canStopLive)
      this.canStartRunner = Boolean(snapshot.canStartRunner ?? this.canStartRunner)
      this.canStopRunner = Boolean(snapshot.canStopRunner ?? this.canStopRunner)
      this.canClearKeys = Boolean(snapshot.canClearKeys ?? this.canClearKeys)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'input')
    },

    setSelectField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'change')
    },

    setCheckboxField(field: string, checked: boolean): void {
      updateLegacyCheckboxField(field, !!checked)
    },

    testConnection(): void {
      void invokeLegacyExecutionTestConnectionRuntime()
    },

    runOnce(): void {
      void invokeLegacyExecutionRunOnceRuntime()
    },

    refreshOrders(): void {
      void invokeLegacyExecutionRefreshRuntime()
    },

    startLive(): void {
      void invokeLegacyExecutionLiveStartRuntime()
    },

    stopLive(): void {
      void invokeLegacyExecutionLiveStopRuntime()
    },

    startRunner(): void {
      void invokeLegacyExecutionRunnerStartRuntime()
    },

    stopRunner(): void {
      void invokeLegacyExecutionRunnerStopRuntime()
    },

    clearKeys(): void {
      void invokeLegacyExecutionClearKeysRuntime()
    },
  },
})
