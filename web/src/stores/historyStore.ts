import { defineStore } from 'pinia'
import {
  invokeLegacyHistoryApplySavedRunRuntime,
  invokeLegacyHistoryApplySessionRuntime,
  invokeLegacyHistoryClearSessionRuntime,
  invokeLegacyHistoryDeleteSavedRunRuntime,
  invokeLegacyHistoryExportPresetRuntime,
  invokeLegacyHistoryImportPresetRuntime,
  invokeLegacyHistoryLoadSavedRunRuntime,
  invokeLegacyHistoryOpenSavedRunReportRuntime,
  invokeLegacyHistoryRefreshSavedRunsRuntime,
  invokeLegacyHistoryRerunSavedRunRuntime,
  invokeLegacyHistoryRevealSavedRunRuntime,
  invokeLegacyHistorySaveSavedRunTagsRuntime,
} from '../legacy/actions'
import { invokeCommand } from '../tauri/invoke'

export interface HistoryOption {
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

interface HistoryState {
  sessionRunCount: number
  sessionOptions: HistoryOption[]
  sessionSelectedValue: string
  sessionSelectedLabel: string
  savedRunCount: number
  savedOptions: HistoryOption[]
  savedSelectedValue: string
  savedSelectedLabel: string
  compareRunCount: number
  compareOptions: HistoryOption[]
  compareSelectedValue: string
  savedRunsSearch: string
  savedRunsTagsDraft: string
  savedRunsMeta: string
  savedRunDetailTitle: string
  savedRunDetailMeta: string
  savedRunDetailStrategy: string
  savedRunDetailDataset: string
  savedRunDetailTimeframe: string
  savedRunDetailValidation: string
  savedRunDetailReport: string
  savedRunDetailSize: string
  savedRunDetailTags: string
  savedRunDetailParams: string
  canApplyHistory: boolean
  canExportPreset: boolean
  canImportPreset: boolean
  canClearHistory: boolean
  canSaveSavedRunTags: boolean
  canRefreshSavedRuns: boolean
  canLoadSavedRun: boolean
  canApplySavedRun: boolean
  canRerunSavedRun: boolean
  canOpenSavedRunReport: boolean
  canRevealSavedRun: boolean
  canDeleteSavedRun: boolean
  canOpenRunsDir: boolean
  statusLine: string
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

function sanitizeOptionArray(value: unknown): HistoryOption[] {
  if (!Array.isArray(value)) return []
  return value
    .map((entry) => {
      if (!entry || typeof entry !== 'object') return null
      const valueText = String((entry as { value?: unknown }).value ?? '').trim()
      const labelText = String((entry as { label?: unknown }).label ?? '').trim()
      const disabled = Boolean((entry as { disabled?: unknown }).disabled)
      return {
        value: valueText,
        label: labelText || valueText || '—',
        disabled,
      }
    })
    .filter((entry): entry is HistoryOption => entry !== null)
}

export const useHistoryStore = defineStore('history', {
  state: (): HistoryState => ({
    sessionRunCount: 0,
    sessionOptions: [],
    sessionSelectedValue: '',
    sessionSelectedLabel: 'No runs yet',
    savedRunCount: 0,
    savedOptions: [],
    savedSelectedValue: '',
    savedSelectedLabel: 'No saved runs',
    compareRunCount: 0,
    compareOptions: [],
    compareSelectedValue: '',
    savedRunsSearch: '',
    savedRunsTagsDraft: '',
    savedRunsMeta: '—',
    savedRunDetailTitle: 'No saved run selected',
    savedRunDetailMeta: 'Select a run to inspect metadata and settings.',
    savedRunDetailStrategy: '—',
    savedRunDetailDataset: '—',
    savedRunDetailTimeframe: '—',
    savedRunDetailValidation: '—',
    savedRunDetailReport: '—',
    savedRunDetailSize: '—',
    savedRunDetailTags: '—',
    savedRunDetailParams: 'Select a saved run to inspect parameters.',
    canApplyHistory: false,
    canExportPreset: true,
    canImportPreset: true,
    canClearHistory: false,
    canSaveSavedRunTags: false,
    canRefreshSavedRuns: true,
    canLoadSavedRun: false,
    canApplySavedRun: false,
    canRerunSavedRun: false,
    canOpenSavedRunReport: false,
    canRevealSavedRun: false,
    canDeleteSavedRun: false,
    canOpenRunsDir: true,
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<HistoryState>): void {
      this.sessionRunCount = Math.max(0, Math.floor(Number(snapshot.sessionRunCount ?? this.sessionRunCount ?? 0)))
      this.sessionOptions = sanitizeOptionArray(snapshot.sessionOptions ?? this.sessionOptions)
      this.sessionSelectedValue = String(snapshot.sessionSelectedValue ?? this.sessionSelectedValue ?? '').trim()
      this.sessionSelectedLabel = String(snapshot.sessionSelectedLabel ?? this.sessionSelectedLabel ?? 'No runs yet').trim() || 'No runs yet'
      this.savedRunCount = Math.max(0, Math.floor(Number(snapshot.savedRunCount ?? this.savedRunCount ?? 0)))
      this.savedOptions = sanitizeOptionArray(snapshot.savedOptions ?? this.savedOptions)
      this.savedSelectedValue = String(snapshot.savedSelectedValue ?? this.savedSelectedValue ?? '').trim()
      this.savedSelectedLabel = String(snapshot.savedSelectedLabel ?? this.savedSelectedLabel ?? 'No saved runs').trim() || 'No saved runs'
      this.compareRunCount = Math.max(0, Math.floor(Number(snapshot.compareRunCount ?? this.compareRunCount ?? 0)))
      this.compareOptions = sanitizeOptionArray(snapshot.compareOptions ?? this.compareOptions)
      this.compareSelectedValue = String(snapshot.compareSelectedValue ?? this.compareSelectedValue ?? '').trim()
      this.savedRunsSearch = String(snapshot.savedRunsSearch ?? this.savedRunsSearch ?? '').trim()
      this.savedRunsTagsDraft = String(snapshot.savedRunsTagsDraft ?? this.savedRunsTagsDraft ?? '').trim()
      this.savedRunsMeta = String(snapshot.savedRunsMeta ?? this.savedRunsMeta ?? '—').trim() || '—'
      this.savedRunDetailTitle = String(snapshot.savedRunDetailTitle ?? this.savedRunDetailTitle ?? 'No saved run selected').trim() || 'No saved run selected'
      this.savedRunDetailMeta = String(snapshot.savedRunDetailMeta ?? this.savedRunDetailMeta ?? 'Select a run to inspect metadata and settings.').trim() || 'Select a run to inspect metadata and settings.'
      this.savedRunDetailStrategy = String(snapshot.savedRunDetailStrategy ?? this.savedRunDetailStrategy ?? '—').trim() || '—'
      this.savedRunDetailDataset = String(snapshot.savedRunDetailDataset ?? this.savedRunDetailDataset ?? '—').trim() || '—'
      this.savedRunDetailTimeframe = String(snapshot.savedRunDetailTimeframe ?? this.savedRunDetailTimeframe ?? '—').trim() || '—'
      this.savedRunDetailValidation = String(snapshot.savedRunDetailValidation ?? this.savedRunDetailValidation ?? '—').trim() || '—'
      this.savedRunDetailReport = String(snapshot.savedRunDetailReport ?? this.savedRunDetailReport ?? '—').trim() || '—'
      this.savedRunDetailSize = String(snapshot.savedRunDetailSize ?? this.savedRunDetailSize ?? '—').trim() || '—'
      this.savedRunDetailTags = String(snapshot.savedRunDetailTags ?? this.savedRunDetailTags ?? '—').trim() || '—'
      this.savedRunDetailParams = String(snapshot.savedRunDetailParams ?? this.savedRunDetailParams ?? 'Select a saved run to inspect parameters.').trim() || 'Select a saved run to inspect parameters.'
      this.canApplyHistory = Boolean(snapshot.canApplyHistory ?? this.canApplyHistory)
      this.canExportPreset = Boolean(snapshot.canExportPreset ?? this.canExportPreset)
      this.canImportPreset = Boolean(snapshot.canImportPreset ?? this.canImportPreset)
      this.canClearHistory = Boolean(snapshot.canClearHistory ?? this.canClearHistory)
      this.canSaveSavedRunTags = Boolean(snapshot.canSaveSavedRunTags ?? this.canSaveSavedRunTags)
      this.canRefreshSavedRuns = Boolean(snapshot.canRefreshSavedRuns ?? this.canRefreshSavedRuns)
      this.canLoadSavedRun = Boolean(snapshot.canLoadSavedRun ?? this.canLoadSavedRun)
      this.canApplySavedRun = Boolean(snapshot.canApplySavedRun ?? this.canApplySavedRun)
      this.canRerunSavedRun = Boolean(snapshot.canRerunSavedRun ?? this.canRerunSavedRun)
      this.canOpenSavedRunReport = Boolean(snapshot.canOpenSavedRunReport ?? this.canOpenSavedRunReport)
      this.canRevealSavedRun = Boolean(snapshot.canRevealSavedRun ?? this.canRevealSavedRun)
      this.canDeleteSavedRun = Boolean(snapshot.canDeleteSavedRun ?? this.canDeleteSavedRun)
      this.canOpenRunsDir = Boolean(snapshot.canOpenRunsDir ?? this.canOpenRunsDir)
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

    applySession(): void {
      void invokeLegacyHistoryApplySessionRuntime()
    },

    exportPreset(): void {
      void invokeLegacyHistoryExportPresetRuntime()
    },

    importPreset(): void {
      void invokeLegacyHistoryImportPresetRuntime()
    },

    clearSession(): void {
      void invokeLegacyHistoryClearSessionRuntime()
    },

    saveSavedRunTags(path: string, tags: string): void {
      void invokeLegacyHistorySaveSavedRunTagsRuntime({
        path: String(path || '').trim(),
        tags: String(tags || ''),
      })
    },

    refreshSavedRuns(): void {
      void invokeLegacyHistoryRefreshSavedRunsRuntime()
    },

    openRunsDir(): void {
      if (!this.canOpenRunsDir) return
      void invokeCommand('open_runs_dir').catch(() => undefined)
    },

    loadSavedRun(path: string): void {
      void invokeLegacyHistoryLoadSavedRunRuntime({ path: String(path || '').trim() })
    },

    applySavedRun(path: string): void {
      void invokeLegacyHistoryApplySavedRunRuntime({ path: String(path || '').trim() })
    },

    rerunSavedRun(path: string): void {
      void invokeLegacyHistoryRerunSavedRunRuntime({ path: String(path || '').trim() })
    },

    openSavedRunReport(path: string): void {
      void invokeLegacyHistoryOpenSavedRunReportRuntime({ path: String(path || '').trim() })
    },

    revealSavedRun(path: string): void {
      void invokeLegacyHistoryRevealSavedRunRuntime({ path: String(path || '').trim() })
    },

    deleteSavedRun(path: string): void {
      void invokeLegacyHistoryDeleteSavedRunRuntime({ path: String(path || '').trim() })
    },
  },
})
