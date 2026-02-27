import { defineStore } from 'pinia'

export interface StrategyOption {
  value: string
  label: string
  disabled: boolean
}

const COST_PRESET_OPTIONS_DEFAULT: StrategyOption[] = [
  { value: 'custom', label: 'Custom', disabled: false },
  { value: 'none', label: 'None (0 bps)', disabled: false },
  { value: 'stocks_liquid', label: 'Stocks (liquid, conservative)', disabled: false },
  { value: 'fx_majors', label: 'FX (majors, conservative)', disabled: false },
  { value: 'crypto_spot_maker', label: 'Crypto spot (maker-ish)', disabled: false },
  { value: 'crypto_spot_taker', label: 'Crypto spot (taker, conservative)', disabled: false },
]

const EXIT_MODE_OPTIONS_DEFAULT: StrategyOption[] = [
  { value: 'cross', label: 'Cross signal', disabled: false },
  { value: 'kaufman_stop', label: 'Kaufman-stop', disabled: false },
]

const COST_PRESETS_DEFAULT: Record<string, { feeBps: string | null; spreadBps: string | null; slippageBps: string | null }> = {
  custom: { feeBps: null, spreadBps: null, slippageBps: null },
  none: { feeBps: '0', spreadBps: '0', slippageBps: '0' },
  stocks_liquid: { feeBps: '0.10', spreadBps: '1.00', slippageBps: '0.50' },
  fx_majors: { feeBps: '0.00', spreadBps: '1.00', slippageBps: '0.50' },
  crypto_spot_maker: { feeBps: '2.00', spreadBps: '5.00', slippageBps: '1.50' },
  crypto_spot_taker: { feeBps: '7.50', spreadBps: '5.00', slippageBps: '2.50' },
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

function parseNonNegativeNumber(value: string): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed < 0) return 0
  return parsed
}

function syncDerivedCostFields(state: StrategyState): void {
  const feeBps = parseNonNegativeNumber(state.feeBps)
  const spreadBps = parseNonNegativeNumber(state.spreadBps)
  const slippageBps = parseNonNegativeNumber(state.slippageBps)
  const perSideBps = feeBps + slippageBps + (spreadBps / 2)
  const roundTripBps = perSideBps * 2
  const commissionFraction = Math.max(0, Math.min(0.999999, perSideBps / 10_000))

  state.commission = String(commissionFraction)
  state.costHint = `Total: ${perSideBps.toFixed(2)} bps/side (${roundTripBps.toFixed(2)} bps round-trip)`
}

function applyCostPresetToState(state: StrategyState, presetId: string): void {
  const presetKey = String(presetId || 'custom').trim() || 'custom'
  const preset = COST_PRESETS_DEFAULT[presetKey] ?? COST_PRESETS_DEFAULT.custom ?? { feeBps: null, spreadBps: null, slippageBps: null }
  if (preset.feeBps != null) state.feeBps = preset.feeBps
  if (preset.spreadBps != null) state.spreadBps = preset.spreadBps
  if (preset.slippageBps != null) state.slippageBps = preset.slippageBps
  syncDerivedCostFields(state)
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
  compositeSignalMode: string
  halfTrendAmplitude: string
  halfTrendAtrPeriod: string
  halfTrendChannelDev: string
  maczFastLength: string
  maczSlowLength: string
  maczSignalLength: string
  maczLengthZ: string
  maczLengthStdev: string
  maczGamma: string
  maczA: string
  maczB: string
  ottPeriod: string
  ottPercent: string
  ottMaType: string
  trendLrPeriod: string
  trendLrMinDeg: string
  trendBbwPeriod: string
  trendBbwMult: string
  trendBbwMin: string
  trendRocPeriod: string
  trendAdxPeriod: string
  trendAdxMin: string
  regimeAdxPeriod: string
  regimeAdxEnter: string
  regimeAdxExit: string
  regimeRevEntryRel: string
  regimeRevExitRel: string
  breakoutBbPeriod: string
  breakoutBbMult: string
  breakoutConfirmBars: string
  breakoutMacdFast: string
  breakoutMacdSlow: string
  breakoutMacdSignal: string
  breakoutCmoPeriod: string
  breakoutCmoMinAbs: string
  breakoutAdoscShort: string
  breakoutAdoscLong: string
  breakoutAdxPeriod: string
  breakoutAdxMin: string
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
  longOnly: boolean
  allowFlip: boolean
  tradeOnNext: boolean
  trendUseRoc: boolean
  trendUseAdx: boolean
  breakoutUseAdx: boolean
  maczUseVolume: boolean
  maczUseLag: boolean
  statusLine: string
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

const STRATEGY_TEXT_FIELDS = [
  'costPreset',
  'feeBps',
  'spreadBps',
  'slippageBps',
  'commission',
  'epsRel',
  'startingEquity',
  'stopLossPct',
  'takeProfitPct',
  'cooldownBars',
  'signalPersistenceBars',
  'trailingStopPct',
  'compositeSignalMode',
  'halfTrendAmplitude',
  'halfTrendAtrPeriod',
  'halfTrendChannelDev',
  'maczFastLength',
  'maczSlowLength',
  'maczSignalLength',
  'maczLengthZ',
  'maczLengthStdev',
  'maczGamma',
  'maczA',
  'maczB',
  'ottPeriod',
  'ottPercent',
  'ottMaType',
  'trendLrPeriod',
  'trendLrMinDeg',
  'trendBbwPeriod',
  'trendBbwMult',
  'trendBbwMin',
  'trendRocPeriod',
  'trendAdxPeriod',
  'trendAdxMin',
  'regimeAdxPeriod',
  'regimeAdxEnter',
  'regimeAdxExit',
  'regimeRevEntryRel',
  'regimeRevExitRel',
  'breakoutBbPeriod',
  'breakoutBbMult',
  'breakoutConfirmBars',
  'breakoutMacdFast',
  'breakoutMacdSlow',
  'breakoutMacdSignal',
  'breakoutCmoPeriod',
  'breakoutCmoMinAbs',
  'breakoutAdoscShort',
  'breakoutAdoscLong',
  'breakoutAdxPeriod',
  'breakoutAdxMin',
  'dmConstraintMinTrades',
  'dmConstraintMaxDd',
  'exitMode',
  'kaufmanStopPeriod',
  'kaufmanStopMult',
  'kaufmanStopMaType',
] as const

type StrategyTextField = (typeof STRATEGY_TEXT_FIELDS)[number]
const STRATEGY_TEXT_FIELD_SET: ReadonlySet<string> = new Set(STRATEGY_TEXT_FIELDS)

const STRATEGY_CHECKBOX_FIELDS = [
  'dmConstraintsVisible',
  'longOnly',
  'allowFlip',
  'tradeOnNext',
  'trendUseRoc',
  'trendUseAdx',
  'breakoutUseAdx',
  'maczUseVolume',
  'maczUseLag',
] as const

type StrategyCheckboxField = (typeof STRATEGY_CHECKBOX_FIELDS)[number]
const STRATEGY_CHECKBOX_FIELD_SET: ReadonlySet<string> = new Set(STRATEGY_CHECKBOX_FIELDS)

function applyVueTextFieldUpdate(state: StrategyState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  if (!STRATEGY_TEXT_FIELD_SET.has(key)) return false
  const typedKey = key as StrategyTextField
  ;(state as unknown as Record<StrategyTextField, string>)[typedKey] = String(value || '')
  return true
}

function applyVueCheckboxFieldUpdate(state: StrategyState, field: string, checked: boolean): boolean {
  const key = String(field || '').trim()
  if (!STRATEGY_CHECKBOX_FIELD_SET.has(key)) return false
  const typedKey = key as StrategyCheckboxField
  ;(state as unknown as Record<StrategyCheckboxField, boolean>)[typedKey] = !!checked
  return true
}

export const useStrategyStore = defineStore('strategy', {
  state: (): StrategyState => ({
    activeControlsTab: 'data',
    costPreset: 'custom',
    costPresetOptions: [...COST_PRESET_OPTIONS_DEFAULT],
    feeBps: '0',
    spreadBps: '0',
    slippageBps: '0',
    commission: '0',
    epsRel: '0',
    costHint: 'Total: 0.00 bps/side (0.00 bps round-trip)',
    startingEquity: '10000',
    stopLossPct: '',
    takeProfitPct: '',
    cooldownBars: '0',
    signalPersistenceBars: '1',
    trailingStopPct: '',
    compositeSignalMode: '',
    halfTrendAmplitude: '',
    halfTrendAtrPeriod: '',
    halfTrendChannelDev: '',
    maczFastLength: '',
    maczSlowLength: '',
    maczSignalLength: '',
    maczLengthZ: '',
    maczLengthStdev: '',
    maczGamma: '',
    maczA: '',
    maczB: '',
    ottPeriod: '',
    ottPercent: '',
    ottMaType: '',
    trendLrPeriod: '',
    trendLrMinDeg: '',
    trendBbwPeriod: '',
    trendBbwMult: '',
    trendBbwMin: '',
    trendRocPeriod: '',
    trendAdxPeriod: '',
    trendAdxMin: '',
    regimeAdxPeriod: '',
    regimeAdxEnter: '',
    regimeAdxExit: '',
    regimeRevEntryRel: '',
    regimeRevExitRel: '',
    breakoutBbPeriod: '',
    breakoutBbMult: '',
    breakoutConfirmBars: '',
    breakoutMacdFast: '',
    breakoutMacdSlow: '',
    breakoutMacdSignal: '',
    breakoutCmoPeriod: '',
    breakoutCmoMinAbs: '',
    breakoutAdoscShort: '',
    breakoutAdoscLong: '',
    breakoutAdxPeriod: '',
    breakoutAdxMin: '',
    dmConstraintMinTrades: '',
    dmConstraintMaxDd: '',
    dmConstraintsVisible: true,
    exitMode: 'cross',
    exitModeOptions: [...EXIT_MODE_OPTIONS_DEFAULT],
    kaufmanStopPeriod: '22',
    kaufmanStopMult: '2.0',
    kaufmanStopMaType: 'sma',
    kaufmanStopPeriodVisible: true,
    kaufmanStopMultVisible: true,
    kaufmanStopMaTypeVisible: true,
    longOnly: true,
    allowFlip: true,
    tradeOnNext: true,
    trendUseRoc: true,
    trendUseAdx: false,
    breakoutUseAdx: false,
    maczUseVolume: false,
    maczUseLag: false,
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
      this.compositeSignalMode = String(snapshot.compositeSignalMode ?? this.compositeSignalMode ?? '').trim()
      this.halfTrendAmplitude = String(snapshot.halfTrendAmplitude ?? this.halfTrendAmplitude ?? '').trim()
      this.halfTrendAtrPeriod = String(snapshot.halfTrendAtrPeriod ?? this.halfTrendAtrPeriod ?? '').trim()
      this.halfTrendChannelDev = String(snapshot.halfTrendChannelDev ?? this.halfTrendChannelDev ?? '').trim()
      this.maczFastLength = String(snapshot.maczFastLength ?? this.maczFastLength ?? '').trim()
      this.maczSlowLength = String(snapshot.maczSlowLength ?? this.maczSlowLength ?? '').trim()
      this.maczSignalLength = String(snapshot.maczSignalLength ?? this.maczSignalLength ?? '').trim()
      this.maczLengthZ = String(snapshot.maczLengthZ ?? this.maczLengthZ ?? '').trim()
      this.maczLengthStdev = String(snapshot.maczLengthStdev ?? this.maczLengthStdev ?? '').trim()
      this.maczGamma = String(snapshot.maczGamma ?? this.maczGamma ?? '').trim()
      this.maczA = String(snapshot.maczA ?? this.maczA ?? '').trim()
      this.maczB = String(snapshot.maczB ?? this.maczB ?? '').trim()
      this.ottPeriod = String(snapshot.ottPeriod ?? this.ottPeriod ?? '').trim()
      this.ottPercent = String(snapshot.ottPercent ?? this.ottPercent ?? '').trim()
      this.ottMaType = String(snapshot.ottMaType ?? this.ottMaType ?? '').trim()
      this.trendLrPeriod = String(snapshot.trendLrPeriod ?? this.trendLrPeriod ?? '').trim()
      this.trendLrMinDeg = String(snapshot.trendLrMinDeg ?? this.trendLrMinDeg ?? '').trim()
      this.trendBbwPeriod = String(snapshot.trendBbwPeriod ?? this.trendBbwPeriod ?? '').trim()
      this.trendBbwMult = String(snapshot.trendBbwMult ?? this.trendBbwMult ?? '').trim()
      this.trendBbwMin = String(snapshot.trendBbwMin ?? this.trendBbwMin ?? '').trim()
      this.trendRocPeriod = String(snapshot.trendRocPeriod ?? this.trendRocPeriod ?? '').trim()
      this.trendAdxPeriod = String(snapshot.trendAdxPeriod ?? this.trendAdxPeriod ?? '').trim()
      this.trendAdxMin = String(snapshot.trendAdxMin ?? this.trendAdxMin ?? '').trim()
      this.regimeAdxPeriod = String(snapshot.regimeAdxPeriod ?? this.regimeAdxPeriod ?? '').trim()
      this.regimeAdxEnter = String(snapshot.regimeAdxEnter ?? this.regimeAdxEnter ?? '').trim()
      this.regimeAdxExit = String(snapshot.regimeAdxExit ?? this.regimeAdxExit ?? '').trim()
      this.regimeRevEntryRel = String(snapshot.regimeRevEntryRel ?? this.regimeRevEntryRel ?? '').trim()
      this.regimeRevExitRel = String(snapshot.regimeRevExitRel ?? this.regimeRevExitRel ?? '').trim()
      this.breakoutBbPeriod = String(snapshot.breakoutBbPeriod ?? this.breakoutBbPeriod ?? '').trim()
      this.breakoutBbMult = String(snapshot.breakoutBbMult ?? this.breakoutBbMult ?? '').trim()
      this.breakoutConfirmBars = String(snapshot.breakoutConfirmBars ?? this.breakoutConfirmBars ?? '').trim()
      this.breakoutMacdFast = String(snapshot.breakoutMacdFast ?? this.breakoutMacdFast ?? '').trim()
      this.breakoutMacdSlow = String(snapshot.breakoutMacdSlow ?? this.breakoutMacdSlow ?? '').trim()
      this.breakoutMacdSignal = String(snapshot.breakoutMacdSignal ?? this.breakoutMacdSignal ?? '').trim()
      this.breakoutCmoPeriod = String(snapshot.breakoutCmoPeriod ?? this.breakoutCmoPeriod ?? '').trim()
      this.breakoutCmoMinAbs = String(snapshot.breakoutCmoMinAbs ?? this.breakoutCmoMinAbs ?? '').trim()
      this.breakoutAdoscShort = String(snapshot.breakoutAdoscShort ?? this.breakoutAdoscShort ?? '').trim()
      this.breakoutAdoscLong = String(snapshot.breakoutAdoscLong ?? this.breakoutAdoscLong ?? '').trim()
      this.breakoutAdxPeriod = String(snapshot.breakoutAdxPeriod ?? this.breakoutAdxPeriod ?? '').trim()
      this.breakoutAdxMin = String(snapshot.breakoutAdxMin ?? this.breakoutAdxMin ?? '').trim()
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
      this.longOnly = Boolean(snapshot.longOnly ?? this.longOnly)
      this.allowFlip = Boolean(snapshot.allowFlip ?? this.allowFlip)
      this.tradeOnNext = Boolean(snapshot.tradeOnNext ?? this.tradeOnNext)
      this.trendUseRoc = Boolean(snapshot.trendUseRoc ?? this.trendUseRoc)
      this.trendUseAdx = Boolean(snapshot.trendUseAdx ?? this.trendUseAdx)
      this.breakoutUseAdx = Boolean(snapshot.breakoutUseAdx ?? this.breakoutUseAdx)
      this.maczUseVolume = Boolean(snapshot.maczUseVolume ?? this.maczUseVolume)
      this.maczUseLag = Boolean(snapshot.maczUseLag ?? this.maczUseLag)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (!applyVueTextFieldUpdate(this, key, next)) return
      if (key === 'feeBps' || key === 'spreadBps' || key === 'slippageBps') {
        syncDerivedCostFields(this)
      }
      this.markVueUpdate()
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (!applyVueTextFieldUpdate(this, key, next)) return
      if (key === 'costPreset') {
        applyCostPresetToState(this, next)
      }
      this.markVueUpdate()
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) this.markVueUpdate()
    },
  },
})

