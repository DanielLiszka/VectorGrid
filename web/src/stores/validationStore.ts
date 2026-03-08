import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import {
  crossRefresh as crossRefreshAction,
  exportReportJson as exportReportJsonAction,
  exportReportXlsx as exportReportXlsxAction,
  exportShortlistCsv as exportShortlistCsvAction,
  exportValidationJson as exportValidationJsonAction,
  openReportsDir as openReportsDirAction,
  runValidation as runValidationAction,
  toggleCrossDataset as toggleCrossDatasetAction,
} from '../services/validationService'
import { useDataStore } from './dataStore'

function rawArray<T>(value: unknown, fallback: T[]): T[] {
  return Array.isArray(value) ? markRaw(value as T[]) : fallback
}

interface CrossDatasetEntry {
  path: string
  label: string
  meta: string
  checked: boolean
  disabled: boolean
  market?: string
  timeframe?: string
  adjusted?: boolean | null
}

function normalizePath(value: string): string {
  return String(value || '').trim()
}

function crossFilterTokens(value: string): string[] {
  const query = String(value || '').trim().toLowerCase()
  if (!query) return []
  const out: string[] = []
  const tokens = query.split(/[\s,]+/g)
  for (const token of tokens) {
    const next = String(token || '').trim()
    if (!next) continue
    out.push(next)
  }
  return out
}

function crossSearchText(entry: CrossDatasetEntry): string {
  return `${String(entry.path || '')} ${String(entry.label || '')} ${String(entry.meta || '')}`
    .trim()
    .toLowerCase()
}

function crossVisible(entry: CrossDatasetEntry, tokens: string[]): boolean {
  if (!tokens.length) return true
  const haystack = crossSearchText(entry)
  return tokens.every((token) => haystack.includes(token))
}

function buildCrossMeta(entries: CrossDatasetEntry[], search: string): string {
  const tokens = crossFilterTokens(search)
  const total = entries.length
  let selected = 0
  let visible = 0
  const hasFilter = tokens.length > 0
  for (const entry of entries) {
    if (entry.checked) selected += 1
    if (!hasFilter || crossVisible(entry, tokens)) visible += 1
  }
  return buildCrossMetaFromCounts(total, selected, visible, tokens)
}

function buildCrossMetaFromCounts(total: number, selected: number, visible: number, tokens: string[]): string {
  const parts: string[] = []
  parts.push(`selected ${selected}/${total}${visible !== total ? ` (showing ${visible})` : ''}`)
  if (tokens.length) parts.push(`filter=${tokens.join(' ')}`)
  return parts.join(' • ')
}

interface CrossSelectionChange {
  path: string
  checked: boolean
}

function applyCrossSelection(
  rows: CrossDatasetEntry[],
  tokens: string[],
  resolveChecked: (entry: CrossDatasetEntry) => boolean,
): {
  nextRows: CrossDatasetEntry[]
  changes: CrossSelectionChange[]
  selectedCount: number
  visibleCount: number
} {
  const nextRows: CrossDatasetEntry[] = []
  const changes: CrossSelectionChange[] = []
  let selectedCount = 0
  let visibleCount = 0
  const hasFilter = tokens.length > 0
  for (const entry of rows) {
    const isVisible = !hasFilter || crossVisible(entry, tokens)
    if (isVisible) visibleCount += 1

    let nextEntry = entry
    if (!entry.disabled && isVisible) {
      const nextChecked = !!resolveChecked(entry)
      if (entry.checked !== nextChecked) {
        nextEntry = { ...entry, checked: nextChecked }
        changes.push({ path: nextEntry.path, checked: nextChecked })
      }
    }

    if (nextEntry.checked) selectedCount += 1
    nextRows.push(nextEntry)
  }

  return {
    nextRows: markRaw(nextRows),
    changes,
    selectedCount,
    visibleCount,
  }
}

function parseMarketLabel(value: string): { market: string; adjusted: boolean | null } {
  const text = String(value || '').trim().toLowerCase()
  if (!text) return { market: '', adjusted: null }
  if (text.startsWith('stocks')) {
    if (text.includes('(adj)')) return { market: 'stocks', adjusted: true }
    if (text.includes('(raw)')) return { market: 'stocks', adjusted: false }
    return { market: 'stocks', adjusted: null }
  }
  return { market: text, adjusted: null }
}

function parseCurrentMarket(datasetContext: string): { market: string; adjusted: boolean | null } {
  const text = String(datasetContext || '')
  const match = text.match(/\bmarket=([^•]+)/i)
  if (!match || !Array.isArray(match) || match.length < 2) return { market: '', adjusted: null }
  return parseMarketLabel(String(match[1] || ''))
}

function parseCurrentTimeframe(dataTimeframe: string, datasetRange: string): string {
  const tf = String(dataTimeframe || '').trim().toLowerCase()
  if (tf) return tf
  const head = String(datasetRange || '').split('•')[0]
  const rangeHead = String(head || '').trim().toLowerCase()
  if (!rangeHead || rangeHead.includes('→')) return ''
  return rangeHead
}

function parseEntryContext(entry: CrossDatasetEntry): { market: string; timeframe: string; adjusted: boolean | null } {
  const meta = String(entry.meta || '')
  const metaLead = String(meta.split('•')[0] || '').trim()
  const parsedMarket = parseMarketLabel(metaLead)
  const market = String(entry.market || parsedMarket.market || '').trim().toLowerCase()

  const inlineAdjusted = entry.adjusted === true ? true : (entry.adjusted === false ? false : null)
  const adjusted = inlineAdjusted == null ? parsedMarket.adjusted : inlineAdjusted

  let timeframe = String(entry.timeframe || '').trim().toLowerCase()
  if (!timeframe) {
    const match = meta.match(/\btf=([^\s•]+)/i)
    if (match && Array.isArray(match) && match.length >= 2) {
      timeframe = String(match[1] || '').trim().toLowerCase()
    }
  }

  return {
    market,
    timeframe,
    adjusted,
  }
}

function applyVueTextFieldUpdate(state: ValidationState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false
  switch (key) {
    case 'valCandidates':
      state.candidates = next
      return true
    case 'valSeed':
      state.seed = next
      return true
    case 'valMinPsr':
      state.minPsr = next
      return true
    case 'valMinDsr':
      state.minDsr = next
      return true
    case 'valCscvSlices':
      state.cscvSlices = next
      return true
    case 'valAutoExportTopN':
      state.autoExportTopN = next
      return true
    case 'valWfWindow':
      state.wfWindow = next
      return true
    case 'valWfStep':
      state.wfStep = next
      return true
    case 'valPerturbIters':
      state.perturbIters = next
      return true
    case 'valPerturbMin':
      state.perturbMin = next
      return true
    case 'valPerturbMax':
      state.perturbMax = next
      return true
    case 'valMcSims':
      state.mcSims = next
      return true
    case 'valCrossMax':
      state.crossMax = next
      return true
    case 'valCrossSearch':
      state.crossSearch = next
      state.crossMeta = buildCrossMeta(state.crossDatasets, next)
      return true
    default:
      return false
  }
}

function applyVueCheckboxFieldUpdate(state: ValidationState, field: string, checked: boolean): boolean {
  const key = String(field || '').trim()
  const next = !!checked
  if (!key) return false
  switch (key) {
    case 'valAutoRun':
      state.autoRun = next
      return true
    case 'valAutoExportXlsx':
      state.autoExportXlsx = next
      return true
    case 'valAutoRevealReport':
      state.autoRevealReport = next
      return true
    case 'valWfEnable':
      state.wfEnable = next
      return true
    case 'valPerturbEnable':
      state.perturbEnable = next
      return true
    case 'valMcEnable':
      state.mcEnable = next
      return true
    case 'valCscvEnable':
      state.cscvEnable = next
      return true
    case 'valCrossEnable':
      state.crossEnable = next
      return true
    default:
      return false
  }
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
  crossDatasets: CrossDatasetEntry[]
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
  source: 'vue' | 'unknown'
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
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueTextFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    runValidation(): void {
      void runValidationAction()
    },

    exportShortlistCsv(): void {
      void exportShortlistCsvAction()
    },

    exportValidationJson(): void {
      void exportValidationJsonAction()
    },

    exportReportJson(): void {
      void exportReportJsonAction()
    },

    exportReportXlsx(): void {
      void exportReportXlsxAction()
    },

    openReportsDir(): void {
      if (!this.canOpenReportsDir) return
      void openReportsDirAction().catch(() => undefined)
    },

    crossRefresh(): void {
      if (!this.canCrossRefresh) return
      const previousSelected = new Set<string>()
      for (const entry of this.crossDatasets) {
        if (!entry.checked) continue
        const path = normalizePath(entry.path)
        if (!path) continue
        previousSelected.add(path)
      }

      void crossRefreshAction()
        .then((summary) => {
          const rows = Array.isArray(summary?.datasets) ? summary.datasets : []
          const activePaths = new Set<string>()
          const next: CrossDatasetEntry[] = []
          const tokens = crossFilterTokens(this.crossSearch)
          const hasFilter = tokens.length > 0
          let selectedCount = 0
          let visibleCount = 0
          for (const row of rows) {
            const path = normalizePath(row.path)
            if (!path || activePaths.has(path)) continue
            activePaths.add(path)
            const label = String(row.label || path).trim() || path
            const meta = String(row.meta || '—').trim() || '—'
            const nextEntry: CrossDatasetEntry = {
              path,
              label,
              meta,
              checked: previousSelected.has(path),
              disabled: this.validateDisabled,
              market: String(row.market || '').trim().toLowerCase(),
              timeframe: String(row.timeframe || '').trim().toLowerCase(),
              adjusted: row.adjusted === true ? true : (row.adjusted === false ? false : null),
            }
            if (nextEntry.checked) selectedCount += 1
            if (!hasFilter || crossVisible(nextEntry, tokens)) visibleCount += 1
            next.push(nextEntry)
          }

          const staleSelections: string[] = []
          for (const path of previousSelected) {
            if (!activePaths.has(path)) staleSelections.push(path)
          }
          this.crossDatasets = markRaw(next)
          this.crossSelectedCount = selectedCount
          this.crossTotalCount = next.length
          this.crossMeta = buildCrossMetaFromCounts(next.length, selectedCount, visibleCount, tokens)
          this.statusLine = 'Refreshed cross-ticker list.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()

          if (!staleSelections.length) return
          void Promise.all(
            staleSelections.map((path) => toggleCrossDatasetAction(path, false)),
          ).catch((error: unknown) => {
            this.statusLine = `Cross-ticker refresh sync failed: ${String(error || '')}`.trim()
            this.source = 'vue'
            this.updatedAtUnixMs = Date.now()
          })
        })
        .catch((error: unknown) => {
          this.statusLine = `Cross-ticker refresh failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    crossSelectAll(): void {
      const tokens = crossFilterTokens(this.crossSearch)
      const { nextRows, changes, selectedCount, visibleCount } = applyCrossSelection(
        this.crossDatasets,
        tokens,
        () => true,
      )

      this.crossDatasets = nextRows
      this.crossSelectedCount = selectedCount
      this.crossTotalCount = nextRows.length
      this.crossMeta = buildCrossMetaFromCounts(nextRows.length, selectedCount, visibleCount, tokens)
      this.statusLine = 'Cross-ticker: selected all (filtered view).'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()

      void Promise.all(
        changes.map((entry) => toggleCrossDatasetAction(entry.path, true)),
      ).catch((error: unknown) => {
        this.statusLine = `Cross-ticker selection failed: ${String(error || '')}`.trim()
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      })
    },

    crossSelectNone(): void {
      const tokens = crossFilterTokens(this.crossSearch)
      const { nextRows, changes, selectedCount, visibleCount } = applyCrossSelection(
        this.crossDatasets,
        tokens,
        () => false,
      )

      this.crossDatasets = nextRows
      this.crossSelectedCount = selectedCount
      this.crossTotalCount = nextRows.length
      this.crossMeta = buildCrossMetaFromCounts(nextRows.length, selectedCount, visibleCount, tokens)
      this.statusLine = 'Cross-ticker: cleared selection (filtered view).'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()

      void Promise.all(
        changes.map((entry) => toggleCrossDatasetAction(entry.path, false)),
      ).catch((error: unknown) => {
        this.statusLine = `Cross-ticker selection failed: ${String(error || '')}`.trim()
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      })
    },

    crossSelectMatch(): void {
      if (!this.canCrossSelectMatch) return

      const dataStore = useDataStore()
      const currentPath = normalizePath(dataStore.csvPath)
      const currentMarket = parseCurrentMarket(dataStore.datasetContext)
      const timeframe = parseCurrentTimeframe(
        dataStore.dataTimeframe,
        dataStore.datasetRange,
      )

      if (!currentMarket.market || !timeframe) {
        this.statusLine = 'Cross-ticker: unable to match current dataset (load data + timeframe first).'
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
        return
      }

      const tokens = crossFilterTokens(this.crossSearch)
      const { nextRows, changes, selectedCount, visibleCount } = applyCrossSelection(
        this.crossDatasets,
        tokens,
        (entry) => {
          const path = normalizePath(entry.path)
          if (currentPath && path === currentPath) return false

          const ctx = parseEntryContext(entry)
          let checked = ctx.market === currentMarket.market && ctx.timeframe === timeframe
          if (checked && currentMarket.market === 'stocks' && currentMarket.adjusted != null) {
            checked = ctx.adjusted === currentMarket.adjusted
          }
          return checked
        },
      )

      this.crossDatasets = nextRows
      this.crossSelectedCount = selectedCount
      this.crossTotalCount = nextRows.length
      this.crossMeta = buildCrossMetaFromCounts(nextRows.length, selectedCount, visibleCount, tokens)
      this.statusLine = 'Cross-ticker: selected datasets matching current dataset.'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()

      void Promise.all(
        changes.map((entry) => toggleCrossDatasetAction(entry.path, entry.checked)),
      ).catch((error: unknown) => {
        this.statusLine = `Cross-ticker match selection failed: ${String(error || '')}`.trim()
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      })
    },

    toggleCrossDataset(path: string, checked: boolean): void {
      const nextPath = String(path || '').trim()
      if (!nextPath) return
      const nextChecked = !!checked
      const tokens = crossFilterTokens(this.crossSearch)
      const next: CrossDatasetEntry[] = []
      const hasFilter = tokens.length > 0
      let selectedCount = 0
      let visibleCount = 0
      for (const entry of this.crossDatasets) {
        let nextEntry = entry
        if (String(entry.path || '').trim() === nextPath && !entry.disabled) {
          nextEntry = {
            ...entry,
            checked: nextChecked,
          }
        }
        if (nextEntry.checked) selectedCount += 1
        if (!hasFilter || crossVisible(nextEntry, tokens)) visibleCount += 1
        next.push(nextEntry)
      }
      this.crossDatasets = markRaw(next)
      this.crossSelectedCount = selectedCount
      this.crossTotalCount = next.length
      this.crossMeta = buildCrossMetaFromCounts(next.length, selectedCount, visibleCount, tokens)
      this.statusLine = `Cross-ticker: ${nextChecked ? 'selected' : 'cleared'} dataset.`
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()

      void toggleCrossDatasetAction(nextPath, nextChecked).catch((error: unknown) => {
        this.statusLine = `Cross-ticker update failed: ${String(error || '')}`.trim()
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      })
    },
  },
})

