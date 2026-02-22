import { defineStore } from 'pinia'
import {
  invokeLegacyOptimizeApplyPresetRuntime,
  invokeLegacyOptimizeCancelRuntime,
  invokeLegacyOptimizeGpuParityCheckRuntime,
  invokeLegacyOptimizePresetKernelRuntime,
  invokeLegacyOptimizeRefreshGpuStatusRuntime,
  invokeLegacyOptimizeRunRuntime,
  invokeLegacyOptimizeValidateRuntime,
} from '../legacy/actions'

export interface OptimizeOption {
  value: string
  label: string
  disabled: boolean
}

function updateLegacyTextField(field: string, value: string, eventType: 'input' | 'change'): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el || el.disabled) return
  if (String(el.value ?? '') !== value) el.value = value
  el.dispatchEvent(new Event(eventType, { bubbles: true }))
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

interface OptimizeState {
  activeControlsTab: string
  strategyKind: string
  strategyHint: string
  strategyOptions: OptimizeOption[]
  presetProfile: string
  presetOptions: OptimizeOption[]
  backend: string
  backendOptions: OptimizeOption[]
  deviceId: string
  deviceOptions: OptimizeOption[]
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
  pairsPreview: string
  gpuStatus: string
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
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useOptimizeStore = defineStore('optimize', {
  state: (): OptimizeState => ({
    activeControlsTab: 'data',
    strategyKind: '',
    strategyHint: '—',
    strategyOptions: [],
    presetProfile: 'Balanced',
    presetOptions: [],
    backend: '',
    backendOptions: [],
    deviceId: '0',
    deviceOptions: [],
    objective: '',
    mode: '',
    fastStart: '',
    fastEnd: '',
    fastStep: '',
    slowStart: '',
    slowEnd: '',
    slowStep: '',
    fastMaType: '',
    fastMaTypeOptions: [],
    slowMaType: '',
    slowMaTypeOptions: [],
    pairsPreview: 'Pairs: —',
    gpuStatus: 'GPU: —',
    gpuParityMeta: 'Parity: —',
    progressMeta: '',
    runPlanMeta: 'Plan: —',
    runReadinessHint: 'Readiness: —',
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
      this.strategyOptions = normalizedOptions(snapshot.strategyOptions ?? this.strategyOptions ?? [])
      this.presetProfile = String(snapshot.presetProfile ?? this.presetProfile ?? 'Balanced').trim() || 'Balanced'
      this.presetOptions = normalizedOptions(snapshot.presetOptions ?? this.presetOptions ?? [])
      this.backend = String(snapshot.backend ?? this.backend ?? '').trim()
      this.backendOptions = normalizedOptions(snapshot.backendOptions ?? this.backendOptions ?? [])
      this.deviceId = String(snapshot.deviceId ?? this.deviceId ?? '0').trim() || '0'
      this.deviceOptions = normalizedOptions(snapshot.deviceOptions ?? this.deviceOptions ?? [])
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
      this.pairsPreview = String(snapshot.pairsPreview ?? this.pairsPreview ?? 'Pairs: —').trim() || 'Pairs: —'
      this.gpuStatus = String(snapshot.gpuStatus ?? this.gpuStatus ?? 'GPU: —').trim() || 'GPU: —'
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
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'input')
    },

    setSelectField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'change')
    },

    applyPreset(): void {
      void invokeLegacyOptimizeApplyPresetRuntime()
    },

    presetKernel(): void {
      void invokeLegacyOptimizePresetKernelRuntime()
    },

    refreshGpuStatus(): void {
      void invokeLegacyOptimizeRefreshGpuStatusRuntime()
    },

    runGpuParityCheck(): void {
      void invokeLegacyOptimizeGpuParityCheckRuntime()
    },

    runOptimization(): void {
      void invokeLegacyOptimizeRunRuntime()
    },

    cancelOptimization(): void {
      void invokeLegacyOptimizeCancelRuntime()
    },

    runValidation(): void {
      void invokeLegacyOptimizeValidateRuntime()
    },
  },
})
