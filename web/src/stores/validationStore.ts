import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import {
  invokeLegacyOptimizeValidateRuntime,
  invokeLegacyValidationCrossRefreshRuntime,
  invokeLegacyValidationCrossSelectAllRuntime,
  invokeLegacyValidationCrossSelectMatchRuntime,
  invokeLegacyValidationCrossSelectNoneRuntime,
  invokeLegacyValidationExportReportJsonRuntime,
  invokeLegacyValidationExportReportXlsxRuntime,
  invokeLegacyValidationExportShortlistCsvRuntime,
  invokeLegacyValidationExportValidationJsonRuntime,
  invokeLegacyValidationToggleCrossDatasetRuntime,
} from '../legacy/actions'
import { invokeCommand } from '../tauri/invoke'

function rawArray<T>(value: unknown, fallback: T[]): T[] {
  return Array.isArray(value) ? markRaw(value as T[]) : fallback
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

interface ValidationState {
  activeResultsTab: string
  validationHint: string
  validationMeta: string
  validationShortlistMeta: string
  validationColumns: { label: string; alignLeft: boolean }[]
  validationRows: { cells: { text: string; className: string; title: string }[] }[]
  validationOutputPreview: string
  crossMeta: string
  tableRowCount: number
  candidates: string
  seed: string
  minPsr: string
  minDsr: string
  cscvSlices: string
  autoRun: boolean
  autoExportXlsx: boolean
  autoRevealReport: boolean
  autoExportTopN: string
  wfEnable: boolean
  wfWindow: string
  wfStep: string
  perturbEnable: boolean
  perturbIters: string
  perturbMin: string
  perturbMax: string
  mcEnable: boolean
  mcSims: string
  cscvEnable: boolean
  crossEnable: boolean
  crossMax: string
  crossSearch: string
  crossDatasets: { path: string; label: string; meta: string; checked: boolean; disabled: boolean }[]
  crossSelectedCount: number
  crossTotalCount: number
  tableVisible: boolean
  canOpenReportsDir: boolean
  canExportValidationCsv: boolean
  canExportValidationJson: boolean
  canExportReportJson: boolean
  canExportReportXlsx: boolean
  canCrossRefresh: boolean
  canCrossSelectAll: boolean
  canCrossSelectNone: boolean
  canCrossSelectMatch: boolean
  statusLine: string
  validateDisabled: boolean
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useValidationStore = defineStore('validation', {
  state: (): ValidationState => ({
    activeResultsTab: 'overview',
    validationHint: '—',
    validationMeta: '-',
    validationShortlistMeta: '-',
    validationColumns: [],
    validationRows: [],
    validationOutputPreview: '-',
    crossMeta: '—',
    tableRowCount: 0,
    candidates: '32',
    seed: '1',
    minPsr: '',
    minDsr: '',
    cscvSlices: '',
    autoRun: true,
    autoExportXlsx: false,
    autoRevealReport: false,
    autoExportTopN: '20',
    wfEnable: false,
    wfWindow: '20000',
    wfStep: '20000',
    perturbEnable: false,
    perturbIters: '8',
    perturbMin: '0.015',
    perturbMax: '0.06',
    mcEnable: false,
    mcSims: '500',
    cscvEnable: false,
    crossEnable: false,
    crossMax: '6',
    crossSearch: '',
    crossDatasets: [],
    crossSelectedCount: 0,
    crossTotalCount: 0,
    tableVisible: false,
    canOpenReportsDir: true,
    canExportValidationCsv: false,
    canExportValidationJson: false,
    canExportReportJson: false,
    canExportReportXlsx: false,
    canCrossRefresh: true,
    canCrossSelectAll: true,
    canCrossSelectNone: true,
    canCrossSelectMatch: true,
    statusLine: 'Ready.',
    validateDisabled: false,
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<ValidationState>): void {
      this.activeResultsTab = String(snapshot.activeResultsTab ?? this.activeResultsTab ?? 'overview').trim() || 'overview'
      this.validationHint = String(snapshot.validationHint ?? this.validationHint ?? '—').trim() || '—'
      this.validationMeta = String(snapshot.validationMeta ?? this.validationMeta ?? '-').trim() || '-'
      this.validationShortlistMeta = String(snapshot.validationShortlistMeta ?? this.validationShortlistMeta ?? '-').trim() || '-'
      this.validationColumns = rawArray(snapshot.validationColumns, this.validationColumns)
      this.validationRows = rawArray(snapshot.validationRows, this.validationRows)
      this.validationOutputPreview = String(snapshot.validationOutputPreview ?? this.validationOutputPreview ?? '-').trim() || '-'
      this.crossMeta = String(snapshot.crossMeta ?? this.crossMeta ?? '—').trim() || '—'
      this.tableRowCount = Math.max(0, Math.floor(Number(snapshot.tableRowCount ?? this.tableRowCount ?? 0)))
      this.candidates = String(snapshot.candidates ?? this.candidates ?? '32').trim() || '32'
      this.seed = String(snapshot.seed ?? this.seed ?? '1').trim() || '1'
      this.minPsr = String(snapshot.minPsr ?? this.minPsr ?? '').trim()
      this.minDsr = String(snapshot.minDsr ?? this.minDsr ?? '').trim()
      this.cscvSlices = String(snapshot.cscvSlices ?? this.cscvSlices ?? '').trim()
      this.autoRun = Boolean(snapshot.autoRun ?? this.autoRun)
      this.autoExportXlsx = Boolean(snapshot.autoExportXlsx ?? this.autoExportXlsx)
      this.autoRevealReport = Boolean(snapshot.autoRevealReport ?? this.autoRevealReport)
      this.autoExportTopN = String(snapshot.autoExportTopN ?? this.autoExportTopN ?? '20').trim() || '20'
      this.wfEnable = Boolean(snapshot.wfEnable ?? this.wfEnable)
      this.wfWindow = String(snapshot.wfWindow ?? this.wfWindow ?? '20000').trim() || '20000'
      this.wfStep = String(snapshot.wfStep ?? this.wfStep ?? '20000').trim() || '20000'
      this.perturbEnable = Boolean(snapshot.perturbEnable ?? this.perturbEnable)
      this.perturbIters = String(snapshot.perturbIters ?? this.perturbIters ?? '8').trim() || '8'
      this.perturbMin = String(snapshot.perturbMin ?? this.perturbMin ?? '0.015').trim() || '0.015'
      this.perturbMax = String(snapshot.perturbMax ?? this.perturbMax ?? '0.06').trim() || '0.06'
      this.mcEnable = Boolean(snapshot.mcEnable ?? this.mcEnable)
      this.mcSims = String(snapshot.mcSims ?? this.mcSims ?? '500').trim() || '500'
      this.cscvEnable = Boolean(snapshot.cscvEnable ?? this.cscvEnable)
      this.crossEnable = Boolean(snapshot.crossEnable ?? this.crossEnable)
      this.crossMax = String(snapshot.crossMax ?? this.crossMax ?? '6').trim() || '6'
      this.crossSearch = String(snapshot.crossSearch ?? this.crossSearch ?? '').trim()
      this.crossDatasets = rawArray(snapshot.crossDatasets, this.crossDatasets)
      this.crossSelectedCount = Math.max(0, Math.floor(Number(snapshot.crossSelectedCount ?? this.crossSelectedCount ?? 0)))
      this.crossTotalCount = Math.max(0, Math.floor(Number(snapshot.crossTotalCount ?? this.crossTotalCount ?? 0)))
      this.tableVisible = Boolean(snapshot.tableVisible ?? this.tableVisible)
      this.canOpenReportsDir = Boolean(snapshot.canOpenReportsDir ?? this.canOpenReportsDir)
      this.canExportValidationCsv = Boolean(snapshot.canExportValidationCsv ?? this.canExportValidationCsv)
      this.canExportValidationJson = Boolean(snapshot.canExportValidationJson ?? this.canExportValidationJson)
      this.canExportReportJson = Boolean(snapshot.canExportReportJson ?? this.canExportReportJson)
      this.canExportReportXlsx = Boolean(snapshot.canExportReportXlsx ?? this.canExportReportXlsx)
      this.canCrossRefresh = Boolean(snapshot.canCrossRefresh ?? this.canCrossRefresh)
      this.canCrossSelectAll = Boolean(snapshot.canCrossSelectAll ?? this.canCrossSelectAll)
      this.canCrossSelectNone = Boolean(snapshot.canCrossSelectNone ?? this.canCrossSelectNone)
      this.canCrossSelectMatch = Boolean(snapshot.canCrossSelectMatch ?? this.canCrossSelectMatch)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.validateDisabled = Boolean(snapshot.validateDisabled ?? this.validateDisabled)
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      updateLegacyTextField(field, String(value || ''), 'input')
    },

    setCheckboxField(field: string, checked: boolean): void {
      updateLegacyCheckboxField(field, !!checked)
    },

    runValidation(): void {
      void invokeLegacyOptimizeValidateRuntime()
    },

    exportShortlistCsv(): void {
      void invokeLegacyValidationExportShortlistCsvRuntime()
    },

    exportValidationJson(): void {
      void invokeLegacyValidationExportValidationJsonRuntime()
    },

    exportReportJson(): void {
      void invokeLegacyValidationExportReportJsonRuntime()
    },

    exportReportXlsx(): void {
      void invokeLegacyValidationExportReportXlsxRuntime()
    },

    openReportsDir(): void {
      if (!this.canOpenReportsDir) return
      void invokeCommand('open_reports_dir').catch(() => undefined)
    },

    crossRefresh(): void {
      void invokeLegacyValidationCrossRefreshRuntime()
    },

    crossSelectAll(): void {
      void invokeLegacyValidationCrossSelectAllRuntime()
    },

    crossSelectNone(): void {
      void invokeLegacyValidationCrossSelectNoneRuntime()
    },

    crossSelectMatch(): void {
      void invokeLegacyValidationCrossSelectMatchRuntime()
    },

    toggleCrossDataset(path: string, checked: boolean): void {
      const nextPath = String(path || '').trim()
      if (!nextPath) return
      void invokeLegacyValidationToggleCrossDatasetRuntime({ path: nextPath, checked: !!checked })
    },
  },
})
