import { defineStore } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { invokeLegacyAdvancedResetMoSearchSpaceRuntime } from '../legacy/actions'

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

function updateLegacyField(field: string, value: string, eventType: 'input' | 'change'): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el || el.disabled) return
  if (String(el.value ?? '') !== value) el.value = value
  el.dispatchEvent(new Event(eventType, { bubbles: true }))
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
  exportAllCsvPath: string
  canBrowseExportAll: boolean
  canResetMoSearchSpace: boolean
  statusLine: string
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
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
      this.mode = String(snapshot.mode ?? this.mode ?? 'Grid').trim() || 'Grid'
      this.modeOptions = normalizedOptions(snapshot.modeOptions ?? this.modeOptions ?? [])
      this.vdpsVisible = Boolean(snapshot.vdpsVisible ?? this.vdpsVisible)
      this.vdpsTelemetryHint = String(snapshot.vdpsTelemetryHint ?? this.vdpsTelemetryHint ?? 'VDPS telemetry appears after a run.').trim() || 'VDPS telemetry appears after a run.'
      this.compositeMoVisible = Boolean(snapshot.compositeMoVisible ?? this.compositeMoVisible)
      this.heatmapBins = String(snapshot.heatmapBins ?? this.heatmapBins ?? '128').trim() || '128'
      this.topK = String(snapshot.topK ?? this.topK ?? '20').trim() || '20'
      this.includeAll = String(snapshot.includeAll ?? this.includeAll ?? 'false').trim() || 'false'
      this.includeAllOptions = normalizedOptions(snapshot.includeAllOptions ?? this.includeAllOptions ?? [])
      this.exportAllCsvPath = String(snapshot.exportAllCsvPath ?? this.exportAllCsvPath ?? '').trim()
      this.canBrowseExportAll = Boolean(snapshot.canBrowseExportAll ?? this.canBrowseExportAll)
      this.canResetMoSearchSpace = Boolean(snapshot.canResetMoSearchSpace ?? this.canResetMoSearchSpace)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'heatmapBins') {
        this.heatmapBins = next
        this.markVueUpdate()
      } else if (key === 'topK') {
        this.topK = next
        this.markVueUpdate()
      } else if (key === 'exportAllCsvPath') {
        this.exportAllCsvPath = next
        this.markVueUpdate()
      }
      updateLegacyField(key, next, 'input')
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'maSource') {
        this.maSource = next
        this.markVueUpdate()
      } else if (key === 'objective') {
        this.objective = next
        this.markVueUpdate()
      } else if (key === 'mode') {
        this.mode = next
        this.markVueUpdate()
      } else if (key === 'includeAll') {
        this.includeAll = next
        this.markVueUpdate()
      }
      updateLegacyField(key, next, 'change')
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
          await invokeLegacyAdvancedResetMoSearchSpaceRuntime()
        } catch {}
      })()
    },
  },
})
