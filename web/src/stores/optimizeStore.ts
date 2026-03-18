import { defineStore } from 'pinia'
import {
  applyPreset as applyPresetAction,
  cancelOptimization as cancelOptimizationAction,
  presetKernel as presetKernelAction,
  refreshGpuStatus as refreshGpuStatusAction,
  runGpuParityCheck as runGpuParityCheckAction,
  runOptimization as runOptimizationAction,
  runValidation as runValidationAction,
} from '../services/optimizeService'

export interface OptimizeOption {
  value: string
  label: string
  disabled: boolean
}

const STRATEGY_OPTIONS_DEFAULT: OptimizeOption[] = [
  { value: 'DoubleMa', label: 'Double MA', disabled: false },
  { value: 'Pit', label: 'Pressure Impulse Trend (PIT)', disabled: false },
  { value: 'Set', label: 'Slope + Efficiency Trend (SET)', disabled: false },
  { value: 'Mts', label: 'Momentum Triple Stack (MTS)', disabled: false },
  { value: 'Orc', label: 'Oscillator Reversion Confluence (ORC)', disabled: false },
  { value: 'Marrt', label: 'MA Ribbon Regime Trend (MARRT)', disabled: false },
  { value: 'Vfct', label: 'Volume Flow Confirmation Trend (VFCT)', disabled: false },
  { value: 'Rocb', label: 'Risk Overlay Channel Breakout (ROCB)', disabled: false },
  { value: 'Vcet', label: 'VIDYA Chandelier Exit Trend (VCET)', disabled: false },
  { value: 'Kqsr', label: 'KDJ QQE Stochastic Reversion (KQSR)', disabled: false },
  { value: 'Carm', label: 'Coppock AO Relative Momentum (CARM)', disabled: false },
  { value: 'Kvdf', label: 'Keltner VWAP Directional Flow (KVDF)', disabled: false },
  { value: 'Mkcs', label: 'Market Kurtosis Compression Setup (MKCS)', disabled: false },
  { value: 'Cdmc', label: 'Cycle Decycler MAMA Composite (CDMC)', disabled: false },
  { value: 'Asr', label: 'Adaptive Squeeze Regime (ASR)', disabled: false },
  { value: 'Vcc', label: 'Volatility Compression Confirmation (VCC)', disabled: false },
  { value: 'Vemr', label: 'Volatility Expansion Mean Reversion (VEMR)', disabled: false },
  { value: 'CompositeCpu', label: 'Composite CPU', disabled: false },
  { value: 'CompositeTrend', label: 'Composite Trend', disabled: false },
  { value: 'RegimeHybrid', label: 'Regime Hybrid', disabled: false },
  { value: 'BreakoutMomentum', label: 'Breakout Momentum', disabled: false },
  { value: 'CompositeMo', label: 'Composite MO', disabled: false },
]

const BACKEND_OPTIONS_DEFAULT: OptimizeOption[] = [
  { value: 'Auto', label: 'Auto (recommended)', disabled: false },
  { value: 'CpuOnly', label: 'CPU only (indicators + backtest on CPU)', disabled: false },
  { value: 'GpuKernel', label: 'GPU unified (kernel + staged fallback)', disabled: false },
]

type MaParamMap = Record<string, number>
type MaParamSweepMap = Record<string, [number, number, number]>
interface MaParamStateInput {
  params?: unknown
  sweeps?: unknown
}

function normalizedOptions(values: unknown): OptimizeOption[] {
  if (!Array.isArray(values)) return []
  const entries: OptimizeOption[] = []
  for (const item of values) {
    if (!item || typeof item !== 'object') continue
    const typed = item as Partial<OptimizeOption>
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

function normalizeStrategyOptions(values: unknown): OptimizeOption[] {
  const entries = normalizedOptions(values)
  const out: OptimizeOption[] = []
  const seen = new Set<string>()
  for (const option of entries) {
    const value = String(option.value || '').trim()
    if (!value || seen.has(value)) continue
    seen.add(value)
    out.push(option)
  }
  for (const fallback of STRATEGY_OPTIONS_DEFAULT) {
    if (seen.has(fallback.value)) continue
    out.push({ ...fallback })
  }
  return out.length > 0 ? out : STRATEGY_OPTIONS_DEFAULT.map((option) => ({ ...option }))
}

function normalizeBackendOptions(values: unknown): OptimizeOption[] {
  const entries = normalizedOptions(values)
  const out: OptimizeOption[] = []
  const seen = new Set<string>()
  for (const option of entries) {
    const value = normalizeBackendValue(option.value)
    if (!value || seen.has(value)) continue
    seen.add(value)
    const fallbackLabel = BACKEND_OPTIONS_DEFAULT.find((row) => row.value === value)?.label
    out.push({
      value,
      label: String(fallbackLabel || option.label || value).trim() || value,
      disabled: Boolean(option.disabled),
    })
  }
  for (const fallback of BACKEND_OPTIONS_DEFAULT) {
    if (seen.has(fallback.value)) continue
    out.push({ ...fallback })
  }
  return out.length > 0 ? out : BACKEND_OPTIONS_DEFAULT.map((option) => ({ ...option }))
}

function normalizeBackendValue(value: unknown): string {
  const backend = String(value || '').trim()
  if (backend === 'Auto') return 'Auto'
  if (backend === 'CpuOnly') return 'CpuOnly'
  if (backend === 'GpuKernel') return 'GpuKernel'
  // Legacy backends are normalized to unified GPU backend.
  if (backend === 'GpuOnly' || backend === 'GpuHybridKernel') return 'GpuKernel'
  return 'CpuOnly'
}

function normalizeMaParams(value: unknown, fallback: MaParamMap | null = null): MaParamMap | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fallback
  const out: MaParamMap = {}
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = String(key || '').trim()
    if (!name) continue
    const parsed = Number(raw)
    if (!Number.isFinite(parsed)) continue
    out[name] = parsed
  }
  return Object.keys(out).length > 0 ? out : null
}

function normalizeMaParamSweeps(value: unknown, fallback: MaParamSweepMap | null = null): MaParamSweepMap | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fallback
  const out: MaParamSweepMap = {}
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = String(key || '').trim()
    if (!name) continue
    const row = raw as unknown
    let start = Number.NaN
    let end = Number.NaN
    let step = Number.NaN
    if (Array.isArray(row)) {
      start = Number(row[0])
      end = Number(row[1])
      step = Number(row[2])
    } else if (row && typeof row === 'object') {
      const typed = row as Record<string, unknown>
      start = Number(typed.start)
      end = Number(typed.end)
      step = Number(typed.step)
    }
    if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step) || step <= 0) continue
    out[name] = [start, end, step]
  }
  return Object.keys(out).length > 0 ? out : null
}

function applyVueFieldUpdate(state: OptimizeState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false
  switch (key) {
    case 'strategyKind':
      state.strategyKind = next
      return true
    case 'presetProfile':
      state.presetProfile = next || 'Balanced'
      return true
    case 'backend':
      state.backend = next
      return true
    case 'objective':
      state.objective = next
      return true
    case 'mode':
      state.mode = next
      return true
    case 'fastStart':
      state.fastStart = next
      return true
    case 'fastEnd':
      state.fastEnd = next
      return true
    case 'fastStep':
      state.fastStep = next
      return true
    case 'slowStart':
      state.slowStart = next
      return true
    case 'slowEnd':
      state.slowEnd = next
      return true
    case 'slowStep':
      state.slowStep = next
      return true
    case 'fastMaType':
      state.fastMaType = next
      return true
    case 'slowMaType':
      state.slowMaType = next
      return true
    default:
      return false
  }
}

interface OptimizeState {
  activeControlsTab: string
  strategyKind: string
  strategyHint: string
  strategyOptions: OptimizeOption[]
  presetProfile: string
  presetOptions: OptimizeOption[]
  backend: string
  backendOptions: OptimizeOption[]
  objective: string
  mode: string
  fastStart: string
  fastEnd: string
  fastStep: string
  slowStart: string
  slowEnd: string
  slowStep: string
  fastMaType: string
  fastMaTypeOptions: OptimizeOption[]
  slowMaType: string
  slowMaTypeOptions: OptimizeOption[]
  fastMaParams: MaParamMap | null
  slowMaParams: MaParamMap | null
  fastMaParamSweeps: MaParamSweepMap | null
  slowMaParamSweeps: MaParamSweepMap | null
  pairsPreview: string
  gpuStatus: string
  gpuDeviceMeta: string
  gpuParityMeta: string
  progressMeta: string
  runPlanMeta: string
  runReadinessHint: string
  runReadinessBadges: string[]
  statusLine: string
  runDisabled: boolean
  canCancel: boolean
  validateDisabled: boolean
  canApplyPreset: boolean
  canPresetKernel: boolean
  canRefreshGpuStatus: boolean
  canParityCheck: boolean
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useOptimizeStore = defineStore('optimize', {
  state: (): OptimizeState => ({
    activeControlsTab: 'data',
    strategyKind: 'DoubleMa',
    strategyHint: 'Double MA crossover strategy.',
    strategyOptions: STRATEGY_OPTIONS_DEFAULT.map((option) => ({ ...option })),
    presetProfile: 'Balanced',
    presetOptions: [
      { value: 'Quick', label: 'Quick', disabled: false },
      { value: 'Balanced', label: 'Balanced', disabled: false },
      { value: 'Deep', label: 'Deep', disabled: false },
    ],
    backend: 'Auto',
    backendOptions: BACKEND_OPTIONS_DEFAULT.map((option) => ({ ...option })),
    objective: 'Sharpe',
    mode: 'Grid',
    fastStart: '5',
    fastEnd: '20',
    fastStep: '1',
    slowStart: '20',
    slowEnd: '80',
    slowStep: '1',
    fastMaType: 'sma',
    fastMaTypeOptions: [
      { value: 'sma', label: 'sma', disabled: false },
      { value: 'alma', label: 'alma', disabled: false },
    ],
    slowMaType: 'alma',
    slowMaTypeOptions: [
      { value: 'sma', label: 'sma', disabled: false },
      { value: 'alma', label: 'alma', disabled: false },
    ],
    fastMaParams: null,
    slowMaParams: null,
    fastMaParamSweeps: null,
    slowMaParamSweeps: null,
    pairsPreview: 'Pairs: —',
    gpuStatus: 'GPU: —',
    gpuDeviceMeta: 'CUDA device selection is automatic.',
    gpuParityMeta: 'Parity: —',
    progressMeta: '',
    runPlanMeta: 'Plan: —',
    runReadinessHint: 'Readiness: Load data first.',
    runReadinessBadges: [],
    statusLine: 'Ready.',
    runDisabled: false,
    canCancel: false,
    validateDisabled: false,
    canApplyPreset: true,
    canPresetKernel: true,
    canRefreshGpuStatus: true,
    canParityCheck: true,
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<OptimizeState>): void {
      this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
      this.strategyKind = String(snapshot.strategyKind ?? this.strategyKind ?? '').trim()
      this.strategyHint = String(snapshot.strategyHint ?? this.strategyHint ?? '—').trim() || '—'
      this.strategyOptions = normalizeStrategyOptions(snapshot.strategyOptions ?? this.strategyOptions ?? [])
      this.presetProfile = String(snapshot.presetProfile ?? this.presetProfile ?? 'Balanced').trim() || 'Balanced'
      this.presetOptions = normalizedOptions(snapshot.presetOptions ?? this.presetOptions ?? [])
      this.backend = normalizeBackendValue(snapshot.backend ?? this.backend ?? '')
      this.backendOptions = normalizeBackendOptions(snapshot.backendOptions ?? this.backendOptions ?? [])
      this.objective = String(snapshot.objective ?? this.objective ?? '').trim()
      this.mode = String(snapshot.mode ?? this.mode ?? '').trim()
      this.fastStart = String(snapshot.fastStart ?? this.fastStart ?? '').trim()
      this.fastEnd = String(snapshot.fastEnd ?? this.fastEnd ?? '').trim()
      this.fastStep = String(snapshot.fastStep ?? this.fastStep ?? '').trim()
      this.slowStart = String(snapshot.slowStart ?? this.slowStart ?? '').trim()
      this.slowEnd = String(snapshot.slowEnd ?? this.slowEnd ?? '').trim()
      this.slowStep = String(snapshot.slowStep ?? this.slowStep ?? '').trim()
      this.fastMaType = String(snapshot.fastMaType ?? this.fastMaType ?? '').trim()
      this.fastMaTypeOptions = normalizedOptions(snapshot.fastMaTypeOptions ?? this.fastMaTypeOptions ?? [])
      this.slowMaType = String(snapshot.slowMaType ?? this.slowMaType ?? '').trim()
      this.slowMaTypeOptions = normalizedOptions(snapshot.slowMaTypeOptions ?? this.slowMaTypeOptions ?? [])
      this.fastMaParams = normalizeMaParams(snapshot.fastMaParams, this.fastMaParams)
      this.slowMaParams = normalizeMaParams(snapshot.slowMaParams, this.slowMaParams)
      this.fastMaParamSweeps = normalizeMaParamSweeps(snapshot.fastMaParamSweeps, this.fastMaParamSweeps)
      this.slowMaParamSweeps = normalizeMaParamSweeps(snapshot.slowMaParamSweeps, this.slowMaParamSweeps)
      this.pairsPreview = String(snapshot.pairsPreview ?? this.pairsPreview ?? 'Pairs: —').trim() || 'Pairs: —'
      this.gpuStatus = String(snapshot.gpuStatus ?? this.gpuStatus ?? 'GPU: —').trim() || 'GPU: —'
      this.gpuDeviceMeta = String(snapshot.gpuDeviceMeta ?? this.gpuDeviceMeta ?? 'CUDA device selection is automatic.').trim() || 'CUDA device selection is automatic.'
      this.gpuParityMeta = String(snapshot.gpuParityMeta ?? this.gpuParityMeta ?? 'Parity: —').trim() || 'Parity: —'
      this.progressMeta = String(snapshot.progressMeta ?? this.progressMeta ?? '').trim()
      this.runPlanMeta = String(snapshot.runPlanMeta ?? this.runPlanMeta ?? 'Plan: —').trim() || 'Plan: —'
      this.runReadinessHint = String(snapshot.runReadinessHint ?? this.runReadinessHint ?? 'Readiness: —').trim() || 'Readiness: —'
      this.runReadinessBadges = Array.isArray(snapshot.runReadinessBadges)
        ? snapshot.runReadinessBadges
            .map((badge) => String(badge || '').trim())
            .filter((badge) => badge.length > 0)
        : this.runReadinessBadges
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.runDisabled = Boolean(snapshot.runDisabled ?? this.runDisabled)
      this.canCancel = Boolean(snapshot.canCancel ?? this.canCancel)
      this.validateDisabled = Boolean(snapshot.validateDisabled ?? this.validateDisabled)
      this.canApplyPreset = Boolean(snapshot.canApplyPreset ?? this.canApplyPreset)
      this.canPresetKernel = Boolean(snapshot.canPresetKernel ?? this.canPresetKernel)
      this.canRefreshGpuStatus = Boolean(snapshot.canRefreshGpuStatus ?? this.canRefreshGpuStatus)
      this.canParityCheck = Boolean(snapshot.canParityCheck ?? this.canParityCheck)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    syncMaParamState(state: { fast?: MaParamStateInput | null; slow?: MaParamStateInput | null }): void {
      const fastState = state?.fast ?? null
      const slowState = state?.slow ?? null
      this.fastMaParams = normalizeMaParams(fastState?.params, null)
      this.slowMaParams = normalizeMaParams(slowState?.params, null)
      this.fastMaParamSweeps = normalizeMaParamSweeps(fastState?.sweeps, null)
      this.slowMaParamSweeps = normalizeMaParamSweeps(slowState?.sweeps, null)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyPreset(): void {
      void applyPresetAction()
    },

    presetKernel(): void {
      void presetKernelAction()
    },

    refreshGpuStatus(quiet = false): void {
      void refreshGpuStatusAction()
        .then((summary) => {
          this.gpuStatus = String(summary?.text || this.gpuStatus || 'GPU: —').trim() || 'GPU: —'
          this.gpuDeviceMeta = String(summary?.deviceMeta || this.gpuDeviceMeta || 'CUDA device selection is automatic.').trim() || 'CUDA device selection is automatic.'
          if (!quiet) {
            this.statusLine = 'GPU status refreshed.'
          }
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          if (!quiet) {
            this.statusLine = `GPU status refresh failed: ${String(error || '')}`.trim()
          }
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    runGpuParityCheck(): void {
      void runGpuParityCheckAction()
    },

    runOptimization(): void {
      void runOptimizationAction()
    },

    cancelOptimization(): void {
      this.statusLine = 'Cancel requested...'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
      void cancelOptimizationAction().catch((error: unknown) => {
        this.statusLine = `Cancel failed: ${String(error || '')}`.trim()
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      })
    },

    runValidation(): void {
      void runValidationAction()
    },
  },
})

