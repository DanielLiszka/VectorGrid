import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import {
  applySavedRun as applySavedRunAction,
  applySession as applySessionAction,
  clearSession as clearSessionAction,
  deleteSavedRun as deleteSavedRunAction,
  exportPreset as exportPresetAction,
  importPreset as importPresetAction,
  loadSavedRun as loadSavedRunAction,
  loadSavedRunDetail as loadSavedRunDetailAction,
  openRunsDir as openRunsDirAction,
  openSavedRunReport as openSavedRunReportAction,
  readSessionHistory as readSessionHistoryAction,
  refreshSavedRuns as refreshSavedRunsAction,
  rerunSavedRun as rerunSavedRunAction,
  revealSavedRun as revealSavedRunAction,
  saveSavedRunTags as saveSavedRunTagsAction,
  type HistorySavedRunDetail,
  type HistorySessionSummary,
} from '../services/historyService'

export interface HistoryOption {
  value: string
  label: string
  disabled: boolean
}

interface HistorySavedRunEntry {
  ts: string
  path: string
  bytes: number
  strategyKind: string
  hasValidation: boolean
  reportXlsxPath: string
  csvPath: string
  tags: string[]
}

function normalizeText(value: unknown): string {
  return String(value || '').trim()
}

function normalizePath(value: string): string {
  return normalizeText(value)
}

function normalizeTags(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  const out: string[] = []
  for (const item of value) {
    const next = normalizeText(item)
    if (!next) continue
    out.push(next)
  }
  return markRaw(out)
}

function rawArray<T>(value: T[]): T[] {
  return markRaw(value)
}

function filterTokens(value: string): string[] {
  const query = normalizeText(value).toLowerCase()
  if (!query) return []
  const out: string[] = []
  const tokens = query.split(/\s+/g)
  for (const token of tokens) {
    const next = normalizeText(token)
    if (!next) continue
    out.push(next)
  }
  return out
}

function fileBasename(path: string): string {
  const next = normalizePath(path)
  if (!next) return ''
  const parts = next.split(/[\\/]+/g)
  return normalizeText(parts[parts.length - 1] || '')
}

function formatBytesCompact(bytes: number): string {
  const value = Number(bytes || 0)
  if (!Number.isFinite(value) || value <= 0) return ''
  const gib = value / (1024 ** 3)
  if (gib >= 1) return `${gib.toFixed(1)} GiB`
  const mib = value / (1024 ** 2)
  if (mib >= 1) return `${mib.toFixed(0)} MiB`
  const kib = value / 1024
  if (kib >= 1) return `${kib.toFixed(0)} KiB`
  return `${Math.floor(value)} B`
}

function runtimeRunning(): boolean {
  if (typeof document === 'undefined') return false
  const root = document.documentElement as HTMLElement | undefined
  if (!root || !root.dataset) return false
  return String(root.dataset.running || '').trim() === '1'
}

function savedRunLabel(entry: HistorySavedRunEntry): string {
  const hasVal = entry.hasValidation ? 'val' : '-'
  const hasXlsx = entry.reportXlsxPath ? 'xlsx' : '-'
  const tagsHint = entry.tags.length ? ` | tags=${entry.tags.length}` : ''
  return `${entry.ts || 'unknown'} | ${entry.strategyKind || ''} | ${hasVal}/${hasXlsx}${tagsHint}`
}

function sessionRunLabel(entry: HistorySessionSummary): string {
  return `${entry.ts || 'unknown'} | ${entry.backend || ''} | ${Math.max(0, Math.floor(Number(entry.pairs || 0)))} pairs`
}

function filterSavedRunsEntries(entries: HistorySavedRunEntry[], search: string): HistorySavedRunEntry[] {
  const tokens = filterTokens(search)
  if (!tokens.length) return entries
  const out: HistorySavedRunEntry[] = []
  for (const entry of entries) {
    const hay = `${entry.ts} ${entry.strategyKind} ${entry.csvPath} ${entry.tags.join(' ')}`
      .trim()
      .toLowerCase()
    if (tokens.every((token) => hay.includes(token))) out.push(entry)
  }
  return out
}

interface HistoryState {
  sessionRunCount: number
  sessionOptions: HistoryOption[]
  sessionSelectedValue: string
  sessionSelectedLabel: string
  savedRunCount: number
  savedRunsRaw: HistorySavedRunEntry[]
  savedRunDetailCache: Record<string, HistorySavedRunDetail>
  savedRunDetailLoadingPath: string
  savedRunDetailSeq: number
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
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

function sanitizeOptionArray(value: unknown): HistoryOption[] {
  if (!Array.isArray(value)) return []
  const out: HistoryOption[] = []
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue
    const valueText = String((entry as { value?: unknown }).value ?? '').trim()
    const labelText = String((entry as { label?: unknown }).label ?? '').trim()
    const disabled = Boolean((entry as { disabled?: unknown }).disabled)
    out.push({
      value: valueText,
      label: labelText || valueText || '—',
      disabled,
    })
  }
  return rawArray(out)
}

function sanitizeSavedRunsArray(value: unknown): HistorySavedRunEntry[] {
  if (!Array.isArray(value)) return []
  const out: HistorySavedRunEntry[] = []
  for (const item of value) {
    if (!item || typeof item !== 'object') continue
    const typed = item as Partial<HistorySavedRunEntry>
    const path = normalizePath(String(typed.path || ''))
    if (!path) continue
    const bytes = Number(typed.bytes)
    out.push({
      ts: normalizeText(typed.ts),
      path,
      bytes: Number.isFinite(bytes) && bytes > 0 ? Math.floor(bytes) : 0,
      strategyKind: normalizeText(typed.strategyKind),
      hasValidation: !!typed.hasValidation,
      reportXlsxPath: normalizePath(String(typed.reportXlsxPath || '')),
      csvPath: normalizePath(String(typed.csvPath || '')),
      tags: normalizeTags(typed.tags),
    })
  }
  return rawArray(out)
}

function pruneSavedRunDetailCache(
  cache: Record<string, HistorySavedRunDetail>,
  savedRuns: HistorySavedRunEntry[],
): Record<string, HistorySavedRunDetail> {
  const allowed = new Set<string>()
  for (const entry of savedRuns) {
    const path = normalizePath(entry.path)
    if (!path) continue
    allowed.add(path)
  }

  let removedAny = false
  const nextCache: Record<string, HistorySavedRunDetail> = {}
  for (const [path, detail] of Object.entries(cache)) {
    if (!allowed.has(path)) {
      removedAny = true
      continue
    }
    nextCache[path] = detail
  }
  return removedAny ? nextCache : cache
}

function applyVueTextFieldUpdate(state: HistoryState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '')
  if (!key) return false
  switch (key) {
    case 'savedRunsSearch':
      state.savedRunsSearch = next
      return true
    case 'savedRunsTags':
      state.savedRunsTagsDraft = next
      return true
    default:
      return false
  }
}

function applyVueSelectFieldUpdate(state: HistoryState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false
  switch (key) {
    case 'historySelect':
      state.sessionSelectedValue = next
      return true
    case 'savedRunsSelect':
      state.savedSelectedValue = next
      return true
    case 'compareSelect':
      state.compareSelectedValue = next
      return true
    default:
      return false
  }
}

export const useHistoryStore = defineStore('history', {
  state: (): HistoryState => ({
    sessionRunCount: 0,
    sessionOptions: [],
    sessionSelectedValue: '',
    sessionSelectedLabel: 'No runs yet',
    savedRunCount: 0,
    savedRunsRaw: [],
    savedRunDetailCache: {},
    savedRunDetailLoadingPath: '',
    savedRunDetailSeq: 0,
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
    refreshSessionHistory(preferredTs?: string): void {
      const runs = readSessionHistoryAction()
      const selectedTs = normalizeText(preferredTs ?? this.sessionSelectedValue)
      const options: HistoryOption[] = [{
        value: '',
        label: runs.length ? 'Select a run...' : 'No runs yet',
        disabled: false,
      }]
      for (const entry of runs) {
        options.push({
          value: normalizeText(entry.ts),
          label: sessionRunLabel(entry),
          disabled: false,
        })
      }
      const hasSelection = selectedTs && runs.some((entry) => normalizeText(entry.ts) === selectedTs)
      this.sessionRunCount = runs.length
      this.sessionOptions = rawArray(options)
      this.sessionSelectedValue = hasSelection ? selectedTs : ''
      this.sessionSelectedLabel = hasSelection
        ? (options.find((entry) => entry.value === selectedTs)?.label || 'No runs yet')
        : 'No runs yet'
      this.canApplyHistory = !!hasSelection
      this.canClearHistory = runs.length > 0
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyLegacySnapshot(snapshot: Partial<HistoryState>): void {
      this.sessionRunCount = Math.max(0, Math.floor(Number(snapshot.sessionRunCount ?? this.sessionRunCount ?? 0)))
      this.sessionOptions = sanitizeOptionArray(snapshot.sessionOptions ?? this.sessionOptions)
      this.sessionSelectedValue = String(snapshot.sessionSelectedValue ?? this.sessionSelectedValue ?? '').trim()
      this.sessionSelectedLabel = String(snapshot.sessionSelectedLabel ?? this.sessionSelectedLabel ?? 'No runs yet').trim() || 'No runs yet'
      this.savedRunCount = Math.max(0, Math.floor(Number(snapshot.savedRunCount ?? this.savedRunCount ?? 0)))
      this.savedRunsRaw = sanitizeSavedRunsArray(snapshot.savedRunsRaw ?? this.savedRunsRaw)
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

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueSelectFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    syncSavedRunsView(preferredPath?: string): void {
      const filtered = filterSavedRunsEntries(this.savedRunsRaw, this.savedRunsSearch)
      const options: HistoryOption[] = [{
        value: '',
        label: filtered.length ? 'Select a saved run...' : 'No saved runs yet',
        disabled: false,
      }]
      for (const entry of filtered) {
        options.push({
          value: entry.path,
          label: savedRunLabel(entry),
          disabled: false,
        })
      }

      const requestedPath = normalizePath(preferredPath ?? this.savedSelectedValue)
      const selectedPath = requestedPath && filtered.some((entry) => entry.path === requestedPath)
        ? requestedPath
        : ''
      const selectedEntry = filtered.find((entry) => entry.path === selectedPath) || null

      this.savedOptions = rawArray(options)
      this.savedRunCount = this.savedRunsRaw.length
      this.savedSelectedValue = selectedPath
      this.savedSelectedLabel = selectedEntry ? savedRunLabel(selectedEntry) : 'No saved runs'
      this.savedRunsTagsDraft = selectedEntry ? selectedEntry.tags.join(', ') : ''

      const selectedDetail = selectedEntry ? this.savedRunDetailCache[selectedEntry.path] : undefined

      if (!selectedEntry) {
        this.savedRunsMeta = `saved runs: ${filtered.length}/${this.savedRunsRaw.length}`
        this.savedRunDetailTitle = 'No saved run selected'
        this.savedRunDetailMeta = 'Select a run to inspect metadata and settings.'
        this.savedRunDetailStrategy = '—'
        this.savedRunDetailDataset = '—'
        this.savedRunDetailTimeframe = '—'
        this.savedRunDetailValidation = '—'
        this.savedRunDetailReport = '—'
        this.savedRunDetailSize = '—'
        this.savedRunDetailTags = '—'
        this.savedRunDetailParams = 'Select a saved run to inspect parameters.'
        this.savedRunDetailLoadingPath = ''
      } else {
        const strategyKind = normalizeText(selectedDetail?.strategyKind || selectedEntry.strategyKind)
        const csvPath = normalizePath(selectedDetail?.csvPath || selectedEntry.csvPath)
        const timeframe = normalizeText(selectedDetail?.timeframe)
        const reportPath = normalizePath(selectedDetail?.reportPath || selectedEntry.reportXlsxPath)
        const hasValidation = selectedDetail ? !!selectedDetail.hasValidation : selectedEntry.hasValidation
        const tags = selectedDetail ? normalizeTags(selectedDetail.tags) : normalizeTags(selectedEntry.tags)

        const parts: string[] = []
        if (selectedEntry.ts) parts.push(`ts=${selectedEntry.ts}`)
        if (strategyKind) parts.push(`strategy=${strategyKind}`)
        if (hasValidation) parts.push('validation=yes')
        if (reportPath) parts.push('xlsx=yes')
        if (csvPath) parts.push(`csv=${csvPath}`)
        if (tags.length) parts.push(`tags=${tags.join(',')}`)
        if (selectedEntry.bytes > 0) parts.push(`bytes=${selectedEntry.bytes}`)
        this.savedRunsMeta = parts.join(' • ')
        this.savedRunDetailTitle = selectedEntry.ts || 'Selected run'
        this.savedRunDetailMeta = `artifact: ${fileBasename(selectedEntry.path) || selectedEntry.path || 'unknown'}`
        this.savedRunDetailStrategy = strategyKind || '—'
        this.savedRunDetailDataset = fileBasename(csvPath) || csvPath || '—'
        this.savedRunDetailTimeframe = timeframe || '—'
        this.savedRunDetailValidation = hasValidation ? 'Available' : 'Not saved'
        this.savedRunDetailReport = reportPath ? 'XLSX available' : 'Not exported'
        this.savedRunDetailSize = formatBytesCompact(selectedEntry.bytes) || '—'
        this.savedRunDetailTags = tags.length ? tags.join(', ') : '—'
        this.savedRunDetailParams = normalizeText(selectedDetail?.paramsText) || 'Loading parameter snapshot...'
      }

      const canMutate = !runtimeRunning()
      const hasSelection = !!selectedEntry
      this.canSaveSavedRunTags = canMutate && hasSelection
      this.canRefreshSavedRuns = canMutate
      this.canLoadSavedRun = canMutate && hasSelection
      this.canApplySavedRun = canMutate && hasSelection
      this.canRerunSavedRun = canMutate && hasSelection
      this.canOpenSavedRunReport = canMutate && !!selectedEntry?.reportXlsxPath
      this.canRevealSavedRun = canMutate && hasSelection
      this.canDeleteSavedRun = canMutate && hasSelection
      this.canOpenRunsDir = canMutate

      if (selectedEntry && !selectedDetail) {
        this.loadSavedRunDetail(selectedEntry.path)
      }
    },

    loadSavedRunDetail(path: string): void {
      const nextPath = normalizePath(path)
      if (!nextPath) return
      if (this.savedRunDetailLoadingPath === nextPath) return

      const seq = Math.max(0, Math.floor(Number(this.savedRunDetailSeq || 0))) + 1
      this.savedRunDetailSeq = seq
      this.savedRunDetailLoadingPath = nextPath

      void loadSavedRunDetailAction(nextPath)
        .then((detail) => {
          this.savedRunDetailCache[nextPath] = {
            strategyKind: normalizeText(detail?.strategyKind),
            timeframe: normalizeText(detail?.timeframe),
            csvPath: normalizePath(detail?.csvPath || ''),
            reportPath: normalizePath(detail?.reportPath || ''),
            hasValidation: !!detail?.hasValidation,
            tags: normalizeTags(detail?.tags),
            paramsText: normalizeText(detail?.paramsText) || 'Request unavailable.',
          }
          if (this.savedRunDetailSeq === seq && this.savedSelectedValue === nextPath) {
            this.syncSavedRunsView(nextPath)
            this.source = 'vue'
            this.updatedAtUnixMs = Date.now()
          }
        })
        .catch((error: unknown) => {
          if (this.savedRunDetailSeq !== seq || this.savedSelectedValue !== nextPath) return
          this.savedRunDetailParams = `Failed to load artifact:\n${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .finally(() => {
          if (this.savedRunDetailLoadingPath === nextPath) {
            this.savedRunDetailLoadingPath = ''
          }
        })
    },

    setSavedRunsSearch(value: string): void {
      this.savedRunsSearch = String(value || '')
      this.syncSavedRunsView(this.savedSelectedValue)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setSavedRunsTagsDraft(value: string): void {
      this.savedRunsTagsDraft = String(value || '')
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    selectSavedRun(path: string): void {
      this.syncSavedRunsView(path)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applySession(): void {
      const selectedTs = normalizePath(this.sessionSelectedValue)
      void applySessionAction(selectedTs)
        .then(() => {
          this.statusLine = 'Applied history run settings to the form.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to apply history run.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    exportPreset(): void {
      void exportPresetAction()
        .then(() => {
          this.statusLine = 'Exported preset.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to export preset.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    importPreset(): void {
      void importPresetAction()
        .then(() => {
          this.statusLine = 'Imported preset.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to import preset.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    clearSession(): void {
      void clearSessionAction()
        .then(() => {
          this.refreshSessionHistory('')
          this.compareSelectedValue = ''
          this.statusLine = 'History cleared.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to clear history.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    saveSavedRunTags(path: string, tags: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return
      const nextTags = String(tags || this.savedRunsTagsDraft || '')
      this.savedRunsTagsDraft = nextTags
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()

      void saveSavedRunTagsAction(nextPath, nextTags)
        .then(() => refreshSavedRunsAction())
        .then((summary) => {
          this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs)
          this.savedRunDetailCache = pruneSavedRunDetailCache(this.savedRunDetailCache, this.savedRunsRaw)
          this.syncSavedRunsView(nextPath)
          this.statusLine = 'Saved tags.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to save tags.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    refreshSavedRuns(): void {
      if (!this.canRefreshSavedRuns) return
      const preferredPath = normalizePath(this.savedSelectedValue)

      void refreshSavedRunsAction()
        .then((summary) => {
          this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs)
          this.savedRunDetailCache = pruneSavedRunDetailCache(this.savedRunDetailCache, this.savedRunsRaw)
          this.syncSavedRunsView(preferredPath)
          this.statusLine = 'Refreshed saved runs.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to refresh saved runs.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    openRunsDir(): void {
      if (!this.canOpenRunsDir) return
      void openRunsDirAction().catch(() => undefined)
    },

    loadSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return
      void loadSavedRunAction(nextPath)
    },

    applySavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return
      void applySavedRunAction(nextPath)
    },

    rerunSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return
      void rerunSavedRunAction(nextPath)
    },

    openSavedRunReport(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return
      void openSavedRunReportAction(nextPath)
    },

    revealSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return
      void revealSavedRunAction(nextPath)
    },

    deleteSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue)
      if (!nextPath) return

      void deleteSavedRunAction(nextPath)
        .then(() => refreshSavedRunsAction())
        .then((summary) => {
          this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs)
          this.savedRunDetailCache = pruneSavedRunDetailCache(this.savedRunDetailCache, this.savedRunsRaw)
          const preferred = this.savedSelectedValue === nextPath ? '' : this.savedSelectedValue
          this.syncSavedRunsView(preferred)
          this.statusLine = 'Deleted saved run.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Failed to delete saved run.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },
  },
})

