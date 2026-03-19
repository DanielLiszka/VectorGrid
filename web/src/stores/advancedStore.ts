import { defineStore } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { resetMoSearchSpace as resetMoSearchSpaceAction } from '../services/advancedService'

export interface AdvancedOption {
  value: string
  label: string
  disabled: boolean
}

function normalizedOptions(values: unknown): AdvancedOption[] {
  if (!Array.isArray(values)) return []
  const entries: AdvancedOption[] = []
  for (const item of values) {
    if (!item || typeof item !== 'object') continue
    const typed = item as Partial<AdvancedOption>
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

function normalizeOptimizationMode(value: unknown): string {
  const mode = String(value || '').trim()
  if (
    mode === 'Grid'
    || mode === 'CoarseToFine'
    || mode === 'GpuVdps'
    || mode === 'Auto'
  ) {
    return mode
  }
  if (
    mode === 'GpuProgressive'
    || mode === 'GpuProgressiveSearch'
  ) {
    return 'Grid'
  }
  return mode || 'Grid'
}

function normalizeModeOptions(values: unknown): AdvancedOption[] {
  const entries = normalizedOptions(values)
    .map((option) => ({
      value: normalizeOptimizationMode(option.value),
      label: normalizeOptimizationMode(option.value),
      disabled: Boolean(option.disabled),
    }))
    .filter((option, index, arr) => arr.findIndex((row) => row.value === option.value) === index)
  if (entries.length > 0) return entries
  return [{ value: 'Grid', label: 'Grid', disabled: false }]
}

interface AdvancedState {
  activeControlsTab: string
  maSource: string
  maSourceOptions: AdvancedOption[]
  objective: string
  objectiveOptions: AdvancedOption[]
  mode: string
  modeOptions: AdvancedOption[]
  vdpsVisible: boolean
  vdpsTelemetryHint: string
  compositeMoVisible: boolean
  heatmapBins: string
  topK: string
  includeAll: string
  includeAllOptions: AdvancedOption[]
  vdpsEta: string
  vdpsRungs: string
  vdpsMinInitialPairs: string
  vdpsMaxSurvivorSeeds: string
  vdpsMaxPeriodCandidates: string
  vdpsRandomInjectionRatio: string
  vdpsEliteMutationRatio: string
  vdpsDifferentialRatio: string
  vdpsSeed: string
  vdpsQualityCheckMaxPairs: string
  vdpsQualityCheckTopK: string
  moObjectiveMin: string
  moSampler: string
  moRandomSeed: string
  moRandomTrials: string
  moTpeSeed: string
  moTpeTrials: string
  moTpeStartupTrials: string
  moTpeCandidates: string
  moTpeGamma: string
  moTpeBatchSize: string
  moTpeMaxModelPoints: string
  moQmcSeed: string
  moQmcTrials: string
  moQmcSequence: string
  moNsga2Seed: string
  moNsga2Population: string
  moNsga2Generations: string
  moNsga2CrossoverProb: string
  moNsga2SwappingProb: string
  moNsga2MutationProb: string
  moConstraintMinTrades: string
  moConstraintMaxDd: string
  moConstraintMaxAbsExposure: string
  moPrescanTopK: string
  moPrescanWindow: string
  moPrescanPredictiveTopK: string
  moPrescanPredictiveStride: string
  moPrescanObjectives: string
  moPrescanMode: string
  moPrescanMaPairs: string
  moSearchSpace: string
  moHtAmpLow: string
  moHtAmpHigh: string
  moHtAmpStep: string
  moHtDevLow: string
  moHtDevHigh: string
  moHtDevStep: string
  moHtAtrLow: string
  moHtAtrHigh: string
  moHtAtrStep: string
  moMaczFastLow: string
  moMaczFastHigh: string
  moMaczFastStep: string
  moMaczSlowLow: string
  moMaczSlowHigh: string
  moMaczSlowStep: string
  moMaczSignalLow: string
  moMaczSignalHigh: string
  moMaczSignalStep: string
  moMaczLengthZLow: string
  moMaczLengthZHigh: string
  moMaczLengthZStep: string
  moMaczLengthStdevLow: string
  moMaczLengthStdevHigh: string
  moMaczLengthStdevStep: string
  moMaczGammaLow: string
  moMaczGammaHigh: string
  moMaczGammaStep: string
  moMaczALow: string
  moMaczAHigh: string
  moMaczAStep: string
  moMaczBLow: string
  moMaczBHigh: string
  moMaczBStep: string
  moMaczUseLagMode: string
  moMaczUseVolumeMode: string
  moOttPeriodLow: string
  moOttPeriodHigh: string
  moOttPeriodStep: string
  moOttPercentLow: string
  moOttPercentHigh: string
  moOttPercentStep: string
  moOttMaTypes: string
  vdpsDeterministic: boolean
  moPrescanEnable: boolean
  moPrescanRequireGpu: boolean
  moUseRangeBuilder: boolean
  exportAllCsvPath: string
  canBrowseExportAll: boolean
  canResetMoSearchSpace: boolean
  statusLine: string
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

const ADVANCED_STRING_FIELD_IDS = new Set<string>([
  'maSource',
  'objective',
  'mode',
  'heatmapBins',
  'topK',
  'includeAll',
  'vdpsEta',
  'vdpsRungs',
  'vdpsMinInitialPairs',
  'vdpsMaxSurvivorSeeds',
  'vdpsMaxPeriodCandidates',
  'vdpsRandomInjectionRatio',
  'vdpsEliteMutationRatio',
  'vdpsDifferentialRatio',
  'vdpsSeed',
  'vdpsQualityCheckMaxPairs',
  'vdpsQualityCheckTopK',
  'moObjectiveMin',
  'moSampler',
  'moRandomSeed',
  'moRandomTrials',
  'moTpeSeed',
  'moTpeTrials',
  'moTpeStartupTrials',
  'moTpeCandidates',
  'moTpeGamma',
  'moTpeBatchSize',
  'moTpeMaxModelPoints',
  'moQmcSeed',
  'moQmcTrials',
  'moQmcSequence',
  'moNsga2Seed',
  'moNsga2Population',
  'moNsga2Generations',
  'moNsga2CrossoverProb',
  'moNsga2SwappingProb',
  'moNsga2MutationProb',
  'moConstraintMinTrades',
  'moConstraintMaxDd',
  'moConstraintMaxAbsExposure',
  'moPrescanTopK',
  'moPrescanWindow',
  'moPrescanPredictiveTopK',
  'moPrescanPredictiveStride',
  'moPrescanObjectives',
  'moPrescanMode',
  'moPrescanMaPairs',
  'moSearchSpace',
  'moHtAmpLow',
  'moHtAmpHigh',
  'moHtAmpStep',
  'moHtDevLow',
  'moHtDevHigh',
  'moHtDevStep',
  'moHtAtrLow',
  'moHtAtrHigh',
  'moHtAtrStep',
  'moMaczFastLow',
  'moMaczFastHigh',
  'moMaczFastStep',
  'moMaczSlowLow',
  'moMaczSlowHigh',
  'moMaczSlowStep',
  'moMaczSignalLow',
  'moMaczSignalHigh',
  'moMaczSignalStep',
  'moMaczLengthZLow',
  'moMaczLengthZHigh',
  'moMaczLengthZStep',
  'moMaczLengthStdevLow',
  'moMaczLengthStdevHigh',
  'moMaczLengthStdevStep',
  'moMaczGammaLow',
  'moMaczGammaHigh',
  'moMaczGammaStep',
  'moMaczALow',
  'moMaczAHigh',
  'moMaczAStep',
  'moMaczBLow',
  'moMaczBHigh',
  'moMaczBStep',
  'moMaczUseLagMode',
  'moMaczUseVolumeMode',
  'moOttPeriodLow',
  'moOttPeriodHigh',
  'moOttPeriodStep',
  'moOttPercentLow',
  'moOttPercentHigh',
  'moOttPercentStep',
  'moOttMaTypes',
  'exportAllCsvPath',
])

const ADVANCED_BOOLEAN_FIELD_IDS = new Set<string>([
  'vdpsDeterministic',
  'moPrescanEnable',
  'moPrescanRequireGpu',
  'moUseRangeBuilder',
])

function applyVueStringFieldUpdate(state: AdvancedState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  if (!key || !ADVANCED_STRING_FIELD_IDS.has(key)) return false
  ;(state as unknown as Record<string, string>)[key] = String(value || '').trim()
  return true
}

function applyVueCheckboxFieldUpdate(state: AdvancedState, field: string, checked: boolean): boolean {
  const key = String(field || '').trim()
  if (!key || !ADVANCED_BOOLEAN_FIELD_IDS.has(key)) return false
  ;(state as unknown as Record<string, boolean>)[key] = !!checked
  return true
}

export const useAdvancedStore = defineStore('advanced', {
  state: (): AdvancedState => ({
    activeControlsTab: 'data',
    maSource: 'close',
    maSourceOptions: [],
    objective: 'Sharpe',
    objectiveOptions: [],
    mode: 'Grid',
    modeOptions: [],
    vdpsVisible: false,
    vdpsTelemetryHint: 'VDPS telemetry appears after a run.',
    compositeMoVisible: false,
    heatmapBins: '128',
    topK: '20',
    includeAll: 'false',
    includeAllOptions: [],
    vdpsEta: '',
    vdpsRungs: '',
    vdpsMinInitialPairs: '',
    vdpsMaxSurvivorSeeds: '',
    vdpsMaxPeriodCandidates: '',
    vdpsRandomInjectionRatio: '',
    vdpsEliteMutationRatio: '',
    vdpsDifferentialRatio: '',
    vdpsSeed: '',
    vdpsQualityCheckMaxPairs: '',
    vdpsQualityCheckTopK: '',
    moObjectiveMin: '',
    moSampler: '',
    moRandomSeed: '',
    moRandomTrials: '',
    moTpeSeed: '',
    moTpeTrials: '',
    moTpeStartupTrials: '',
    moTpeCandidates: '',
    moTpeGamma: '',
    moTpeBatchSize: '',
    moTpeMaxModelPoints: '',
    moQmcSeed: '',
    moQmcTrials: '',
    moQmcSequence: '',
    moNsga2Seed: '',
    moNsga2Population: '',
    moNsga2Generations: '',
    moNsga2CrossoverProb: '',
    moNsga2SwappingProb: '',
    moNsga2MutationProb: '',
    moConstraintMinTrades: '',
    moConstraintMaxDd: '',
    moConstraintMaxAbsExposure: '',
    moPrescanTopK: '',
    moPrescanWindow: '',
    moPrescanPredictiveTopK: '',
    moPrescanPredictiveStride: '',
    moPrescanObjectives: '',
    moPrescanMode: '',
    moPrescanMaPairs: '',
    moSearchSpace: '',
    moHtAmpLow: '',
    moHtAmpHigh: '',
    moHtAmpStep: '',
    moHtDevLow: '',
    moHtDevHigh: '',
    moHtDevStep: '',
    moHtAtrLow: '',
    moHtAtrHigh: '',
    moHtAtrStep: '',
    moMaczFastLow: '',
    moMaczFastHigh: '',
    moMaczFastStep: '',
    moMaczSlowLow: '',
    moMaczSlowHigh: '',
    moMaczSlowStep: '',
    moMaczSignalLow: '',
    moMaczSignalHigh: '',
    moMaczSignalStep: '',
    moMaczLengthZLow: '',
    moMaczLengthZHigh: '',
    moMaczLengthZStep: '',
    moMaczLengthStdevLow: '',
    moMaczLengthStdevHigh: '',
    moMaczLengthStdevStep: '',
    moMaczGammaLow: '',
    moMaczGammaHigh: '',
    moMaczGammaStep: '',
    moMaczALow: '',
    moMaczAHigh: '',
    moMaczAStep: '',
    moMaczBLow: '',
    moMaczBHigh: '',
    moMaczBStep: '',
    moMaczUseLagMode: '',
    moMaczUseVolumeMode: '',
    moOttPeriodLow: '',
    moOttPeriodHigh: '',
    moOttPeriodStep: '',
    moOttPercentLow: '',
    moOttPercentHigh: '',
    moOttPercentStep: '',
    moOttMaTypes: '',
    vdpsDeterministic: true,
    moPrescanEnable: false,
    moPrescanRequireGpu: false,
    moUseRangeBuilder: true,
    exportAllCsvPath: '',
    canBrowseExportAll: true,
    canResetMoSearchSpace: false,
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    markVueUpdate(): void {
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyLegacySnapshot(snapshot: Partial<AdvancedState>): void {
      this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
      this.maSource = String(snapshot.maSource ?? this.maSource ?? 'close').trim() || 'close'
      this.maSourceOptions = normalizedOptions(snapshot.maSourceOptions ?? this.maSourceOptions ?? [])
      this.objective = String(snapshot.objective ?? this.objective ?? 'Sharpe').trim() || 'Sharpe'
      this.objectiveOptions = normalizedOptions(snapshot.objectiveOptions ?? this.objectiveOptions ?? [])
      this.mode = normalizeOptimizationMode(snapshot.mode ?? this.mode ?? 'Grid')
      this.modeOptions = normalizeModeOptions(snapshot.modeOptions ?? this.modeOptions ?? [])
      this.vdpsVisible = Boolean(snapshot.vdpsVisible ?? this.vdpsVisible)
      this.vdpsTelemetryHint = String(snapshot.vdpsTelemetryHint ?? this.vdpsTelemetryHint ?? 'VDPS telemetry appears after a run.').trim() || 'VDPS telemetry appears after a run.'
      this.compositeMoVisible = Boolean(snapshot.compositeMoVisible ?? this.compositeMoVisible)
      this.heatmapBins = String(snapshot.heatmapBins ?? this.heatmapBins ?? '128').trim() || '128'
      this.topK = String(snapshot.topK ?? this.topK ?? '20').trim() || '20'
      this.includeAll = String(snapshot.includeAll ?? this.includeAll ?? 'false').trim() || 'false'
      this.includeAllOptions = normalizedOptions(snapshot.includeAllOptions ?? this.includeAllOptions ?? [])
      this.vdpsEta = String(snapshot.vdpsEta ?? this.vdpsEta ?? '').trim()
      this.vdpsRungs = String(snapshot.vdpsRungs ?? this.vdpsRungs ?? '').trim()
      this.vdpsMinInitialPairs = String(snapshot.vdpsMinInitialPairs ?? this.vdpsMinInitialPairs ?? '').trim()
      this.vdpsMaxSurvivorSeeds = String(snapshot.vdpsMaxSurvivorSeeds ?? this.vdpsMaxSurvivorSeeds ?? '').trim()
      this.vdpsMaxPeriodCandidates = String(snapshot.vdpsMaxPeriodCandidates ?? this.vdpsMaxPeriodCandidates ?? '').trim()
      this.vdpsRandomInjectionRatio = String(snapshot.vdpsRandomInjectionRatio ?? this.vdpsRandomInjectionRatio ?? '').trim()
      this.vdpsEliteMutationRatio = String(snapshot.vdpsEliteMutationRatio ?? this.vdpsEliteMutationRatio ?? '').trim()
      this.vdpsDifferentialRatio = String(snapshot.vdpsDifferentialRatio ?? this.vdpsDifferentialRatio ?? '').trim()
      this.vdpsSeed = String(snapshot.vdpsSeed ?? this.vdpsSeed ?? '').trim()
      this.vdpsQualityCheckMaxPairs = String(snapshot.vdpsQualityCheckMaxPairs ?? this.vdpsQualityCheckMaxPairs ?? '').trim()
      this.vdpsQualityCheckTopK = String(snapshot.vdpsQualityCheckTopK ?? this.vdpsQualityCheckTopK ?? '').trim()
      this.moObjectiveMin = String(snapshot.moObjectiveMin ?? this.moObjectiveMin ?? '').trim()
      this.moSampler = String(snapshot.moSampler ?? this.moSampler ?? '').trim()
      this.moRandomSeed = String(snapshot.moRandomSeed ?? this.moRandomSeed ?? '').trim()
      this.moRandomTrials = String(snapshot.moRandomTrials ?? this.moRandomTrials ?? '').trim()
      this.moTpeSeed = String(snapshot.moTpeSeed ?? this.moTpeSeed ?? '').trim()
      this.moTpeTrials = String(snapshot.moTpeTrials ?? this.moTpeTrials ?? '').trim()
      this.moTpeStartupTrials = String(snapshot.moTpeStartupTrials ?? this.moTpeStartupTrials ?? '').trim()
      this.moTpeCandidates = String(snapshot.moTpeCandidates ?? this.moTpeCandidates ?? '').trim()
      this.moTpeGamma = String(snapshot.moTpeGamma ?? this.moTpeGamma ?? '').trim()
      this.moTpeBatchSize = String(snapshot.moTpeBatchSize ?? this.moTpeBatchSize ?? '').trim()
      this.moTpeMaxModelPoints = String(snapshot.moTpeMaxModelPoints ?? this.moTpeMaxModelPoints ?? '').trim()
      this.moQmcSeed = String(snapshot.moQmcSeed ?? this.moQmcSeed ?? '').trim()
      this.moQmcTrials = String(snapshot.moQmcTrials ?? this.moQmcTrials ?? '').trim()
      this.moQmcSequence = String(snapshot.moQmcSequence ?? this.moQmcSequence ?? '').trim()
      this.moNsga2Seed = String(snapshot.moNsga2Seed ?? this.moNsga2Seed ?? '').trim()
      this.moNsga2Population = String(snapshot.moNsga2Population ?? this.moNsga2Population ?? '').trim()
      this.moNsga2Generations = String(snapshot.moNsga2Generations ?? this.moNsga2Generations ?? '').trim()
      this.moNsga2CrossoverProb = String(snapshot.moNsga2CrossoverProb ?? this.moNsga2CrossoverProb ?? '').trim()
      this.moNsga2SwappingProb = String(snapshot.moNsga2SwappingProb ?? this.moNsga2SwappingProb ?? '').trim()
      this.moNsga2MutationProb = String(snapshot.moNsga2MutationProb ?? this.moNsga2MutationProb ?? '').trim()
      this.moConstraintMinTrades = String(snapshot.moConstraintMinTrades ?? this.moConstraintMinTrades ?? '').trim()
      this.moConstraintMaxDd = String(snapshot.moConstraintMaxDd ?? this.moConstraintMaxDd ?? '').trim()
      this.moConstraintMaxAbsExposure = String(snapshot.moConstraintMaxAbsExposure ?? this.moConstraintMaxAbsExposure ?? '').trim()
      this.moPrescanTopK = String(snapshot.moPrescanTopK ?? this.moPrescanTopK ?? '').trim()
      this.moPrescanWindow = String(snapshot.moPrescanWindow ?? this.moPrescanWindow ?? '').trim()
      this.moPrescanPredictiveTopK = String(snapshot.moPrescanPredictiveTopK ?? this.moPrescanPredictiveTopK ?? '').trim()
      this.moPrescanPredictiveStride = String(snapshot.moPrescanPredictiveStride ?? this.moPrescanPredictiveStride ?? '').trim()
      this.moPrescanObjectives = String(snapshot.moPrescanObjectives ?? this.moPrescanObjectives ?? '').trim()
      this.moPrescanMode = String(snapshot.moPrescanMode ?? this.moPrescanMode ?? '').trim()
      this.moPrescanMaPairs = String(snapshot.moPrescanMaPairs ?? this.moPrescanMaPairs ?? '').trim()
      this.moSearchSpace = String(snapshot.moSearchSpace ?? this.moSearchSpace ?? '').trim()
      this.moHtAmpLow = String(snapshot.moHtAmpLow ?? this.moHtAmpLow ?? '').trim()
      this.moHtAmpHigh = String(snapshot.moHtAmpHigh ?? this.moHtAmpHigh ?? '').trim()
      this.moHtAmpStep = String(snapshot.moHtAmpStep ?? this.moHtAmpStep ?? '').trim()
      this.moHtDevLow = String(snapshot.moHtDevLow ?? this.moHtDevLow ?? '').trim()
      this.moHtDevHigh = String(snapshot.moHtDevHigh ?? this.moHtDevHigh ?? '').trim()
      this.moHtDevStep = String(snapshot.moHtDevStep ?? this.moHtDevStep ?? '').trim()
      this.moHtAtrLow = String(snapshot.moHtAtrLow ?? this.moHtAtrLow ?? '').trim()
      this.moHtAtrHigh = String(snapshot.moHtAtrHigh ?? this.moHtAtrHigh ?? '').trim()
      this.moHtAtrStep = String(snapshot.moHtAtrStep ?? this.moHtAtrStep ?? '').trim()
      this.moMaczFastLow = String(snapshot.moMaczFastLow ?? this.moMaczFastLow ?? '').trim()
      this.moMaczFastHigh = String(snapshot.moMaczFastHigh ?? this.moMaczFastHigh ?? '').trim()
      this.moMaczFastStep = String(snapshot.moMaczFastStep ?? this.moMaczFastStep ?? '').trim()
      this.moMaczSlowLow = String(snapshot.moMaczSlowLow ?? this.moMaczSlowLow ?? '').trim()
      this.moMaczSlowHigh = String(snapshot.moMaczSlowHigh ?? this.moMaczSlowHigh ?? '').trim()
      this.moMaczSlowStep = String(snapshot.moMaczSlowStep ?? this.moMaczSlowStep ?? '').trim()
      this.moMaczSignalLow = String(snapshot.moMaczSignalLow ?? this.moMaczSignalLow ?? '').trim()
      this.moMaczSignalHigh = String(snapshot.moMaczSignalHigh ?? this.moMaczSignalHigh ?? '').trim()
      this.moMaczSignalStep = String(snapshot.moMaczSignalStep ?? this.moMaczSignalStep ?? '').trim()
      this.moMaczLengthZLow = String(snapshot.moMaczLengthZLow ?? this.moMaczLengthZLow ?? '').trim()
      this.moMaczLengthZHigh = String(snapshot.moMaczLengthZHigh ?? this.moMaczLengthZHigh ?? '').trim()
      this.moMaczLengthZStep = String(snapshot.moMaczLengthZStep ?? this.moMaczLengthZStep ?? '').trim()
      this.moMaczLengthStdevLow = String(snapshot.moMaczLengthStdevLow ?? this.moMaczLengthStdevLow ?? '').trim()
      this.moMaczLengthStdevHigh = String(snapshot.moMaczLengthStdevHigh ?? this.moMaczLengthStdevHigh ?? '').trim()
      this.moMaczLengthStdevStep = String(snapshot.moMaczLengthStdevStep ?? this.moMaczLengthStdevStep ?? '').trim()
      this.moMaczGammaLow = String(snapshot.moMaczGammaLow ?? this.moMaczGammaLow ?? '').trim()
      this.moMaczGammaHigh = String(snapshot.moMaczGammaHigh ?? this.moMaczGammaHigh ?? '').trim()
      this.moMaczGammaStep = String(snapshot.moMaczGammaStep ?? this.moMaczGammaStep ?? '').trim()
      this.moMaczALow = String(snapshot.moMaczALow ?? this.moMaczALow ?? '').trim()
      this.moMaczAHigh = String(snapshot.moMaczAHigh ?? this.moMaczAHigh ?? '').trim()
      this.moMaczAStep = String(snapshot.moMaczAStep ?? this.moMaczAStep ?? '').trim()
      this.moMaczBLow = String(snapshot.moMaczBLow ?? this.moMaczBLow ?? '').trim()
      this.moMaczBHigh = String(snapshot.moMaczBHigh ?? this.moMaczBHigh ?? '').trim()
      this.moMaczBStep = String(snapshot.moMaczBStep ?? this.moMaczBStep ?? '').trim()
      this.moMaczUseLagMode = String(snapshot.moMaczUseLagMode ?? this.moMaczUseLagMode ?? '').trim()
      this.moMaczUseVolumeMode = String(snapshot.moMaczUseVolumeMode ?? this.moMaczUseVolumeMode ?? '').trim()
      this.moOttPeriodLow = String(snapshot.moOttPeriodLow ?? this.moOttPeriodLow ?? '').trim()
      this.moOttPeriodHigh = String(snapshot.moOttPeriodHigh ?? this.moOttPeriodHigh ?? '').trim()
      this.moOttPeriodStep = String(snapshot.moOttPeriodStep ?? this.moOttPeriodStep ?? '').trim()
      this.moOttPercentLow = String(snapshot.moOttPercentLow ?? this.moOttPercentLow ?? '').trim()
      this.moOttPercentHigh = String(snapshot.moOttPercentHigh ?? this.moOttPercentHigh ?? '').trim()
      this.moOttPercentStep = String(snapshot.moOttPercentStep ?? this.moOttPercentStep ?? '').trim()
      this.moOttMaTypes = String(snapshot.moOttMaTypes ?? this.moOttMaTypes ?? '').trim()
      this.vdpsDeterministic = Boolean(snapshot.vdpsDeterministic ?? this.vdpsDeterministic)
      this.moPrescanEnable = Boolean(snapshot.moPrescanEnable ?? this.moPrescanEnable)
      this.moPrescanRequireGpu = Boolean(snapshot.moPrescanRequireGpu ?? this.moPrescanRequireGpu)
      this.moUseRangeBuilder = Boolean(snapshot.moUseRangeBuilder ?? this.moUseRangeBuilder)
      this.exportAllCsvPath = String(snapshot.exportAllCsvPath ?? this.exportAllCsvPath ?? '').trim()
      this.canBrowseExportAll = Boolean(snapshot.canBrowseExportAll ?? this.canBrowseExportAll)
      this.canResetMoSearchSpace = Boolean(snapshot.canResetMoSearchSpace ?? this.canResetMoSearchSpace)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueStringFieldUpdate(this, key, next)) {
        this.markVueUpdate()
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueStringFieldUpdate(this, key, next)) {
        this.markVueUpdate()
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
        this.markVueUpdate()
      }
    },

    browseExportAll(): void {
      void (async () => {
        try {
          const picked = await invokeCommand('pick_save_csv', { defaultName: 'all_pairs.csv' })
          const path = String(picked || '').trim()
          if (!path) return
          this.setInputField('exportAllCsvPath', path)
        } catch {}
      })()
    },

    resetMoSearchSpace(): void {
      void (async () => {
        try {
          if (!this.canResetMoSearchSpace) return
          const result = await resetMoSearchSpaceAction()
          const json = String(result?.json || '').trim()
          if (!json) return
          this.setInputField('moSearchSpace', json)
          this.statusLine = 'Reset Composite MO search space.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        } catch (error) {
          this.statusLine = `Reset search space failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        }
      })()
    },
  },
})

