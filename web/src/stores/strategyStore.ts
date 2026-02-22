import { defineStore } from 'pinia'

export interface StrategyOption {
  value: string
  label: string
  disabled: boolean
}

function normalizedOptions(values: unknown): StrategyOption[] {
  if (!Array.isArray(values)) return []
  const entries: StrategyOption[] = []
  for (const item of values) {
    if (!item || typeof item !== 'object') continue
    const typed = item as Partial<StrategyOption>
    const value = String(typed.value || '').trim()
    const label = String(typed.label || typed.value || '').trim()
    if (!value && !label) continue
    entries.push({
      value: value || label,
      label: label || value,
      disabled: Boolean(typed.disabled),
    })
  }
  return entries
}

function updateLegacyField(field: string, value: string, eventType: 'input' | 'change'): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el || el.disabled) return
  if (String(el.value ?? '') !== value) el.value = value
  el.dispatchEvent(new Event(eventType, { bubbles: true }))
}

interface StrategyState {
  activeControlsTab: string
  costPreset: string
  costPresetOptions: StrategyOption[]
  feeBps: string
  spreadBps: string
  slippageBps: string
  commission: string
  epsRel: string
  costHint: string
  startingEquity: string
  stopLossPct: string
  takeProfitPct: string
  cooldownBars: string
  signalPersistenceBars: string
  trailingStopPct: string
  dmConstraintMinTrades: string
  dmConstraintMaxDd: string
  dmConstraintsVisible: boolean
  exitMode: string
  exitModeOptions: StrategyOption[]
  kaufmanStopPeriod: string
  kaufmanStopMult: string
  kaufmanStopMaType: string
  kaufmanStopPeriodVisible: boolean
  kaufmanStopMultVisible: boolean
  kaufmanStopMaTypeVisible: boolean
  statusLine: string
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useStrategyStore = defineStore('strategy', {
  state: (): StrategyState => ({
    activeControlsTab: 'data',
    costPreset: 'custom',
    costPresetOptions: [],
    feeBps: '0',
    spreadBps: '0',
    slippageBps: '0',
    commission: '0',
    epsRel: '0',
    costHint: 'Total: —',
    startingEquity: '10000',
    stopLossPct: '',
    takeProfitPct: '',
    cooldownBars: '0',
    signalPersistenceBars: '1',
    trailingStopPct: '',
    dmConstraintMinTrades: '',
    dmConstraintMaxDd: '',
    dmConstraintsVisible: true,
    exitMode: 'cross',
    exitModeOptions: [],
    kaufmanStopPeriod: '22',
    kaufmanStopMult: '2.0',
    kaufmanStopMaType: 'sma',
    kaufmanStopPeriodVisible: true,
    kaufmanStopMultVisible: true,
    kaufmanStopMaTypeVisible: true,
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    markVueUpdate(): void {
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyLegacySnapshot(snapshot: Partial<StrategyState>): void {
      this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
      this.costPreset = String(snapshot.costPreset ?? this.costPreset ?? 'custom').trim() || 'custom'
      this.costPresetOptions = normalizedOptions(snapshot.costPresetOptions ?? this.costPresetOptions ?? [])
      this.feeBps = String(snapshot.feeBps ?? this.feeBps ?? '0').trim() || '0'
      this.spreadBps = String(snapshot.spreadBps ?? this.spreadBps ?? '0').trim() || '0'
      this.slippageBps = String(snapshot.slippageBps ?? this.slippageBps ?? '0').trim() || '0'
      this.commission = String(snapshot.commission ?? this.commission ?? '0').trim() || '0'
      this.epsRel = String(snapshot.epsRel ?? this.epsRel ?? '0').trim() || '0'
      this.costHint = String(snapshot.costHint ?? this.costHint ?? 'Total: —').trim() || 'Total: —'
      this.startingEquity = String(snapshot.startingEquity ?? this.startingEquity ?? '10000').trim() || '10000'
      this.stopLossPct = String(snapshot.stopLossPct ?? this.stopLossPct ?? '').trim()
      this.takeProfitPct = String(snapshot.takeProfitPct ?? this.takeProfitPct ?? '').trim()
      this.cooldownBars = String(snapshot.cooldownBars ?? this.cooldownBars ?? '0').trim() || '0'
      this.signalPersistenceBars = String(snapshot.signalPersistenceBars ?? this.signalPersistenceBars ?? '1').trim() || '1'
      this.trailingStopPct = String(snapshot.trailingStopPct ?? this.trailingStopPct ?? '').trim()
      this.dmConstraintMinTrades = String(snapshot.dmConstraintMinTrades ?? this.dmConstraintMinTrades ?? '').trim()
      this.dmConstraintMaxDd = String(snapshot.dmConstraintMaxDd ?? this.dmConstraintMaxDd ?? '').trim()
      this.dmConstraintsVisible = Boolean(snapshot.dmConstraintsVisible ?? this.dmConstraintsVisible)
      this.exitMode = String(snapshot.exitMode ?? this.exitMode ?? 'cross').trim() || 'cross'
      this.exitModeOptions = normalizedOptions(snapshot.exitModeOptions ?? this.exitModeOptions ?? [])
      this.kaufmanStopPeriod = String(snapshot.kaufmanStopPeriod ?? this.kaufmanStopPeriod ?? '22').trim() || '22'
      this.kaufmanStopMult = String(snapshot.kaufmanStopMult ?? this.kaufmanStopMult ?? '2.0').trim() || '2.0'
      this.kaufmanStopMaType = String(snapshot.kaufmanStopMaType ?? this.kaufmanStopMaType ?? 'sma').trim() || 'sma'
      this.kaufmanStopPeriodVisible = Boolean(snapshot.kaufmanStopPeriodVisible ?? this.kaufmanStopPeriodVisible)
      this.kaufmanStopMultVisible = Boolean(snapshot.kaufmanStopMultVisible ?? this.kaufmanStopMultVisible)
      this.kaufmanStopMaTypeVisible = Boolean(snapshot.kaufmanStopMaTypeVisible ?? this.kaufmanStopMaTypeVisible)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'feeBps') {
        this.feeBps = next
        this.markVueUpdate()
      } else if (key === 'spreadBps') {
        this.spreadBps = next
        this.markVueUpdate()
      } else if (key === 'slippageBps') {
        this.slippageBps = next
        this.markVueUpdate()
      } else if (key === 'epsRel') {
        this.epsRel = next
        this.markVueUpdate()
      } else if (key === 'startingEquity') {
        this.startingEquity = next
        this.markVueUpdate()
      } else if (key === 'stopLossPct') {
        this.stopLossPct = next
        this.markVueUpdate()
      } else if (key === 'takeProfitPct') {
        this.takeProfitPct = next
        this.markVueUpdate()
      } else if (key === 'cooldownBars') {
        this.cooldownBars = next
        this.markVueUpdate()
      } else if (key === 'signalPersistenceBars') {
        this.signalPersistenceBars = next
        this.markVueUpdate()
      } else if (key === 'trailingStopPct') {
        this.trailingStopPct = next
        this.markVueUpdate()
      } else if (key === 'dmConstraintMinTrades') {
        this.dmConstraintMinTrades = next
        this.markVueUpdate()
      } else if (key === 'dmConstraintMaxDd') {
        this.dmConstraintMaxDd = next
        this.markVueUpdate()
      } else if (key === 'kaufmanStopPeriod') {
        this.kaufmanStopPeriod = next
        this.markVueUpdate()
      } else if (key === 'kaufmanStopMult') {
        this.kaufmanStopMult = next
        this.markVueUpdate()
      } else if (key === 'kaufmanStopMaType') {
        this.kaufmanStopMaType = next
        this.markVueUpdate()
      }
      updateLegacyField(key, next, 'input')
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'costPreset') {
        this.costPreset = next
        this.markVueUpdate()
      } else if (key === 'exitMode') {
        this.exitMode = next
        this.markVueUpdate()
      }
      updateLegacyField(key, next, 'change')
    },
  },
})
