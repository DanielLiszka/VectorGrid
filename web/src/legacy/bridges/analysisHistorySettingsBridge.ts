import { useAnalysisStore } from '../../stores/analysisStore'
import { useHistoryStore, type HistoryOption } from '../../stores/historyStore'
import { useSettingsStore } from '../../stores/settingsStore'
import { createCoalescedSync } from './coalescedSync'

const ANALYSIS_TEXT_IDS = [
  'paretoMetaVue',
  'overviewEmptyTitle',
  'overviewEmptyBody',
  'overviewEmptyHint',
  'bestKicker',
  'bestTitle',
  'bestMeta',
  'bestSummary',
  'compareSummary',
  'topFilterMeta',
  'topSelectionGuardrail',
  'paretoMeta',
  'drillMetaVue',
  'drillMeta',
  'drillStatusVue',
  'drillStatus',
  'stabilityStatusVue',
  'stabilityStatus',
  'stabilityMetaVue',
  'stabilityMeta',
  'heatmapMetaVue',
  'heatmapMeta',
  'aggregateSummarySavedRuns',
  'aggregateSummarySavedMeta',
  'aggregateSummaryLatestGroups',
  'aggregateSummaryLatestArtifacts',
  'aggregateSummaryLatestHoldout',
  'aggregateSummaryLatestRuntime',
  'batchMeta',
  'batchOutput',
  'metaMeta',
  'metaSavedMeta',
  'stage4SavedMeta',
  'metaMlStatus',
  'metaStage4Status',
  'metaStage4Output',
  'metaHoldoutNote',
  'metaCompareSummary',
  'metaOutput',
  'output',
  'status',
] as const
const ANALYSIS_VALUE_IDS = [
  'filterMinPnl',
  'filterMinSharpe',
  'filterMaxDd',
  'filterMinTrades',
  'filterMinExposure',
  'metaStrategyKind',
  'metaTimeframe',
  'metaRequiredTag',
  'metaMinSamples',
  'metaTopK',
  'metaHoldoutTag',
  'metaEvalHoldout',
  'metaSaveHoldoutRows',
  'metaHoldoutMode',
  'metaHoldoutReps',
  'metaHoldoutEvalTopN',
  'metaHoldoutEvalRankBy',
  'metaHoldoutExploreN',
  'metaGroupingMode',
  'metaClusterIntBucket',
  'metaClusterFloatBucket',
  'metaCompareSelect',
  'batchDatasetSearch',
  'batchTags',
  'batchHoldoutPct',
  'batchHoldoutTag',
  'batchAutoMeta',
  'batchSkipDone',
  'metaSavedSearch',
  'metaSavedSelect',
  'metaSavedTags',
  'stage4SavedSearch',
  'stage4SavedSelect',
  'stage4SavedTags',
  'metaUseMlConfidence',
  'metaMlAutopilot',
  'metaStage4TopK',
  'metaStage4MinConf',
  'metaStage4MetaWeight',
  'metaStage4MlWeight',
  'metaStage4UncertaintyWeight',
  'metaStage4AcceptConf',
  'metaStage4ReviewConf',
  'metaStage4MaxAcceptUncertainty',
  'metaStage4AutoTrain',
  'metaStage4RequireHealthy',
  'metaStage4FallbackToMeta',
] as const

const HISTORY_TEXT_IDS = [
  'savedRunsMeta',
  'savedRunDetailTitle',
  'savedRunDetailMeta',
  'savedRunDetailStrategy',
  'savedRunDetailDataset',
  'savedRunDetailTimeframe',
  'savedRunDetailValidation',
  'savedRunDetailReport',
  'savedRunDetailSize',
  'savedRunDetailTags',
  'savedRunDetailParams',
  'status',
] as const
const SETTINGS_TEXT_IDS = ['marketDataDirHint', 'status'] as const
const HISTORY_SELECT_IDS = ['historySelect', 'savedRunsSelect', 'compareSelect'] as const
const HISTORY_INPUT_IDS = ['savedRunsSearch', 'savedRunsTags'] as const
const ANALYSIS_SNAPSHOT_EVENT = 'vta:analysis-snapshot'
const HISTORY_SNAPSHOT_EVENT = 'vta:history-snapshot'
const SETTINGS_INPUT_IDS = [
  'marketDataDir',
  'marketDataMoveExisting',
] as const
const SETTINGS_BUTTON_IDS: readonly string[] = []

function textById(id: string): string {
  const el = document.getElementById(id)
  return String(el?.textContent || '').trim()
}

function valueById(id: string): string {
  const el = document.getElementById(id) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  return String(el?.value || '').trim()
}

function checkedById(id: string): boolean {
  const el = document.getElementById(id) as HTMLInputElement | null
  return !!el?.checked
}

function runtimeRunning(): boolean {
  return String(document.documentElement.dataset.running || '').trim() === '1'
}

function runtimeFlag(flag: string, fallback = false): boolean {
  const raw = document.documentElement.dataset[flag]
  if (raw == null) return fallback
  return String(raw).trim() === '1'
}

function runtimeText(key: string, fallback: string): string {
  const raw = document.documentElement.dataset[key]
  const text = String(raw ?? '').trim()
  return text || fallback
}

function selectedLabelById(id: string, fallback: string): string {
  const el = document.getElementById(id) as HTMLSelectElement | null
  const text = String(el?.selectedOptions?.[0]?.textContent || '').trim()
  return text || fallback
}

function selectedValueById(id: string): string {
  const el = document.getElementById(id) as HTMLSelectElement | null
  return String(el?.value || '').trim()
}

function optionsById(id: string): HistoryOption[] {
  const el = document.getElementById(id) as HTMLSelectElement | null
  if (!el) return []
  return Array.from(el.options).map((option) => ({
    value: String(option.value || '').trim(),
    label: String(option.textContent || '').trim() || String(option.value || '').trim() || '—',
    disabled: !!option.disabled,
  }))
}

function topColumns(): { key: string; label: string; sort: '' | 'asc' | 'desc'; alignLeft: boolean }[] {
  const nodes = Array.from(document.querySelectorAll('#topTable thead th')) as HTMLTableCellElement[]
  return nodes.map((th, index) => {
    const key = String(th.dataset.key || '').trim() || `col_${index + 1}`
    const label = String(th.textContent || '').trim() || key
    const sortRaw = String(th.getAttribute('data-sort') || '').trim().toLowerCase()
    const sort = sortRaw === 'asc' || sortRaw === 'desc' ? sortRaw : ''
    const alignLeft = th.classList.contains('left')
    return { key, label, sort, alignLeft }
  })
}

function topRowsSnapshot(): { selected: boolean; cells: { text: string; className: string; title: string }[] }[] {
  const body = document.getElementById('topTableBody')
  if (!body) return []
  const rows = Array.from(body.querySelectorAll('tr'))
  return rows.map((row) => {
    const cells = Array.from(row.querySelectorAll('td')).map((cell) => ({
      text: String(cell.textContent || '').trim(),
      className: String(cell.className || '').trim(),
      title: String(cell.getAttribute('title') || '').trim(),
    }))
    return {
      selected: row.classList.contains('selected'),
      cells,
    }
  })
}

function bestStatsSnapshot(): { label: string; value: string; tone: string }[] {
  const nodes = Array.from(document.querySelectorAll('#bestGrid .bestStat')) as HTMLElement[]
  return nodes
    .map((node) => {
      const label = String((node.querySelector('.bestStatK') as HTMLElement | null)?.textContent || '').trim()
      const value = String((node.querySelector('.bestStatV') as HTMLElement | null)?.textContent || '').trim()
      const tone = node.classList.contains('good')
        ? 'good'
        : (node.classList.contains('bad') ? 'bad' : '')
      return { label, value, tone }
    })
    .filter((entry) => entry.label.length > 0)
}

function metaColumns(): { label: string; alignLeft: boolean }[] {
  const nodes = Array.from(document.querySelectorAll('#metaTable thead th')) as HTMLTableCellElement[]
  return nodes.map((th) => ({
    label: String(th.textContent || '').trim() || '—',
    alignLeft: th.classList.contains('left'),
  }))
}

function metaRowsSnapshot(): { cells: { text: string; className: string; title: string }[] }[] {
  const body = document.getElementById('metaTableBody')
  if (!body) return []
  const rows = Array.from(body.querySelectorAll('tr'))
  return rows.map((row) => ({
    cells: Array.from(row.querySelectorAll('td')).map((cell) => ({
      text: String(cell.textContent || '').trim(),
      className: String(cell.className || '').trim(),
      title: String(cell.getAttribute('title') || '').trim(),
    })),
  }))
}

function batchDatasetsSnapshot(): { path: string; label: string; meta: string; checked: boolean; disabled: boolean }[] {
  const nodes = Array.from(document.querySelectorAll('#batchDatasets .listItem')) as HTMLElement[]
  return nodes.map((node) => {
    const checkbox = node.querySelector('input[type="checkbox"][data-csv-path]') as HTMLInputElement | null
    const title = String((node.querySelector('.mono') as HTMLElement | null)?.textContent || '').trim()
    const meta = String((node.querySelector('.muted') as HTMLElement | null)?.textContent || '').trim()
    const path = String(checkbox?.dataset?.csvPath || '').trim()
    return {
      path,
      label: title || path || '—',
      meta: meta || '—',
      checked: !!checkbox?.checked,
      disabled: !!checkbox?.disabled,
    }
  })
}

function countOptions(id: string): number {
  const el = document.getElementById(id) as HTMLSelectElement | null
  if (!el) return 0
  const options = Array.from(el.options)
  return options.filter((opt, index) => {
    const value = String(opt.value || '').trim()
    const label = String(opt.textContent || '').trim().toLowerCase()
    if (!value) return false
    if (index === 0 && (label.includes('select') || label.includes('compare against') || label.includes('no runs'))) {
      return false
    }
    return true
  }).length
}

function preTextById(id: string, maxLength = 1600): string {
  const text = String(document.getElementById(id)?.textContent || '').trim()
  if (!text) return '—'
  if (text.length <= maxLength) return text
  return `${text.slice(0, maxLength).trimEnd()}…`
}

function activeResultsTab(): string {
  const rootTab = String(document.documentElement.dataset.resultsTab || '').trim()
  return rootTab || 'overview'
}

function activeAggregateTab(): string {
  const rootTab = String(document.documentElement.dataset.aggregateTab || '').trim()
  return rootTab || 'meta'
}

function observeTextNodes(ids: readonly string[], onChange: () => void): MutationObserver {
  const observer = new MutationObserver(() => onChange())
  for (const id of ids) {
    const node = document.getElementById(id)
    if (!node) continue
    observer.observe(node, {
      childList: true,
      characterData: true,
      subtree: true,
    })
  }
  return observer
}

export function attachAnalysisContextBridge(): () => void {
  const store = useAnalysisStore()

  const syncNow = () => {
    const currentResultsTab = activeResultsTab()
    const currentAggregateTab = activeAggregateTab()
    const topTableActive = currentResultsTab === 'top'
    const metaTableActive = currentResultsTab === 'meta' && currentAggregateTab === 'meta'
    const batchTabActive = currentResultsTab === 'meta' && currentAggregateTab === 'batch'

    const topTableBody = topTableActive ? document.getElementById('topTableBody') : null
    const topRows = topTableBody ? topTableBody.querySelectorAll('tr').length : store.topRows
    const batchDatasets = batchTabActive ? batchDatasetsSnapshot() : store.batchDatasets
    const batchSelectedCount = batchTabActive
      ? batchDatasets.filter((entry) => entry.checked).length
      : store.batchSelectedCount
    const batchTotalCount = batchTabActive ? batchDatasets.length : store.batchTotalCount
    const running = runtimeRunning()
    const paretoWrap =
      (document.getElementById('paretoWrapVue') as HTMLElement | null) ??
      (document.getElementById('paretoWrap') as HTMLElement | null)
    const paretoWrapVisible = !!paretoWrap && window.getComputedStyle(paretoWrap).display !== 'none'
    const drillWrap =
      (document.getElementById('drillWrapVue') as HTMLElement | null) ??
      (document.getElementById('drillWrap') as HTMLElement | null)
    const drillWrapVisible = !!drillWrap && window.getComputedStyle(drillWrap).display !== 'none'
    const stabilityWrap =
      (document.getElementById('stabilityWrapVue') as HTMLElement | null) ??
      (document.getElementById('stabilityWrap') as HTMLElement | null)
    const stabilityWrapVisible = !!stabilityWrap && window.getComputedStyle(stabilityWrap).display !== 'none'
    const tradeStatsWrap =
      (document.getElementById('tradeStatsWrapVue') as HTMLElement | null) ??
      (document.getElementById('tradeStatsWrap') as HTMLElement | null)
    const tradeStatsWrapVisible = !!tradeStatsWrap && window.getComputedStyle(tradeStatsWrap).display !== 'none'
    const heatmapWrap =
      (document.getElementById('heatmapWrapVue') as HTMLElement | null) ??
      (document.getElementById('heatmapWrap') as HTMLElement | null)
    const heatmapWrapVisible = !!heatmapWrap && window.getComputedStyle(heatmapWrap).display !== 'none'
    const overviewEmpty = document.getElementById('overviewEmpty') as HTMLElement | null
    const overviewEmptyVisible = !!overviewEmpty && window.getComputedStyle(overviewEmpty).display !== 'none'
    store.applyLegacySnapshot({
      activeResultsTab: currentResultsTab,
      topColumns: topTableActive ? topColumns() : store.topColumns,
      topTableRows: topTableActive ? topRowsSnapshot() : store.topTableRows,
      bestKicker: textById('bestKicker') || '—',
      bestTitle: textById('bestTitle') || '—',
      bestMeta: textById('bestMeta') || '—',
      bestSummary: textById('bestSummary') || '-',
      bestStats: bestStatsSnapshot(),
      overviewEmptyVisible,
      overviewEmptyTitle: textById('overviewEmptyTitle') || 'Load data to begin',
      overviewEmptyBody: textById('overviewEmptyBody') || 'Load a CSV dataset or download candles from Polygon, then run an optimization to view results here.',
      overviewEmptyHint: textById('overviewEmptyHint') || 'Tip: start with a small dataset to validate your strategy settings.',
      overviewPrimaryLabel: runtimeText('overviewPrimaryLabel', 'Browse CSV…'),
      overviewPrimaryAction: runtimeText('overviewPrimaryAction', 'browse_csv'),
      overviewPrimaryDisabled: runtimeFlag('overviewPrimaryDisabled', false),
      overviewSecondaryVisible: runtimeFlag('overviewSecondaryVisible', true),
      overviewSecondaryLabel: runtimeText('overviewSecondaryLabel', 'Polygon…'),
      overviewSecondaryAction: runtimeText('overviewSecondaryAction', 'polygon'),
      overviewSecondaryDisabled: runtimeFlag('overviewSecondaryDisabled', false),
      compareSummary: textById('compareSummary') || '-',
      canCopyBest: runtimeFlag('anaCanCopyBest', true),
      canExportCsv: runtimeFlag('anaCanExportCsv', true),
      canExportJson: runtimeFlag('anaCanExportJson', true),
      canSetBaseline: runtimeFlag('anaCanSetBaseline', true),
      canClearBaseline: runtimeFlag('anaCanClearBaseline', true),
      filterMinPnl: valueById('filterMinPnl'),
      filterMinSharpe: valueById('filterMinSharpe'),
      filterMaxDd: valueById('filterMaxDd'),
      filterMinTrades: valueById('filterMinTrades'),
      filterMinExposure: valueById('filterMinExposure'),
      canClearTopFilters: runtimeFlag('anaCanClearTopFilters', true),
      paretoX: valueById('paretoX') || 'max_dd',
      paretoY: valueById('paretoY') || 'pnl',
      paretoXOptions: optionsById('paretoX'),
      paretoYOptions: optionsById('paretoY'),
      paretoWrapVisible,
      drillWrapVisible,
      stabilityWrapVisible,
      tradeStatsWrapVisible,
      heatmapWrapVisible,
      heatmapColor: valueById('heatmapColor') || 'cividis',
      heatmapColorOptions: optionsById('heatmapColor'),
      heatmapRange: valueById('heatmapRange') || 'q2_98',
      heatmapRangeOptions: optionsById('heatmapRange'),
      heatmapShowTop: valueById('heatmapShowTop') || 'true',
      heatmapShowTopOptions: optionsById('heatmapShowTop'),
      heatmapShowCrosshair: valueById('heatmapShowCrosshair') || 'true',
      heatmapShowCrosshairOptions: optionsById('heatmapShowCrosshair'),
      canHeatmapExportPng: runtimeFlag('anaCanHeatmapExportPng', true),
      metaStrategyKind: valueById('metaStrategyKind') || 'CompositeMo',
      metaStrategyOptions: optionsById('metaStrategyKind'),
      metaTimeframe: valueById('metaTimeframe'),
      metaRequiredTag: valueById('metaRequiredTag'),
      metaMinSamples: valueById('metaMinSamples') || '30',
      metaTopK: valueById('metaTopK') || '25',
      metaHoldoutTag: valueById('metaHoldoutTag') || 'holdout',
      metaEvalHoldout: checkedById('metaEvalHoldout'),
      metaSaveHoldoutRows: checkedById('metaSaveHoldoutRows'),
      metaHoldoutMode: valueById('metaHoldoutMode') || 'light',
      metaHoldoutReps: valueById('metaHoldoutReps') || '2',
      metaHoldoutEvalTopN: valueById('metaHoldoutEvalTopN') || '0',
      metaHoldoutEvalRankBy: valueById('metaHoldoutEvalRankBy') || 'auto',
      metaHoldoutExploreN: valueById('metaHoldoutExploreN') || '0',
      metaGroupingMode: valueById('metaGroupingMode') || 'clustered',
      metaClusterIntBucket: valueById('metaClusterIntBucket') || '5',
      metaClusterFloatBucket: valueById('metaClusterFloatBucket') || '0.25',
      metaHoldoutNote: textById('metaHoldoutNote') || '—',
      aggregateSummarySavedRuns: textById('aggregateSummarySavedRuns') || '0',
      aggregateSummarySavedMeta: textById('aggregateSummarySavedMeta') || '0',
      aggregateSummaryLatestGroups: textById('aggregateSummaryLatestGroups') || '—',
      aggregateSummaryLatestArtifacts: textById('aggregateSummaryLatestArtifacts') || '—',
      aggregateSummaryLatestHoldout: textById('aggregateSummaryLatestHoldout') || '—',
      aggregateSummaryLatestRuntime: textById('aggregateSummaryLatestRuntime') || '—',
      metaMeta: textById('metaMeta') || '—',
      metaOutputPreview: preTextById('metaOutput'),
      metaTableColumns: metaTableActive ? metaColumns() : store.metaTableColumns,
      metaTableRows: metaTableActive ? metaRowsSnapshot() : store.metaTableRows,
      metaCompareOptions: optionsById('metaCompareSelect'),
      metaCompareSelected: selectedValueById('metaCompareSelect'),
      metaCompareSummary: textById('metaCompareSummary') || '-',
      batchDatasetSearch: valueById('batchDatasetSearch'),
      batchTags: valueById('batchTags'),
      batchHoldoutPct: valueById('batchHoldoutPct') || '15',
      batchHoldoutTag: valueById('batchHoldoutTag') || 'holdout',
      batchAutoMeta: checkedById('batchAutoMeta'),
      batchSkipDone: checkedById('batchSkipDone'),
      batchDatasets,
      batchSelectedCount,
      batchTotalCount,
      batchMeta: textById('batchMeta') || '—',
      batchOutputPreview: preTextById('batchOutput'),
      canRunBatch: runtimeFlag('metaCanRunBatch', !running),
      canBatchStop: runtimeFlag('metaCanBatchStop', false),
      canBatchRefresh: runtimeFlag('metaCanBatchRefresh', !running),
      canBatchSelectAll: runtimeFlag('metaCanBatchSelectAll', !running),
      canBatchSelectNone: runtimeFlag('metaCanBatchSelectNone', !running),
      canBatchRetryFailed: runtimeFlag('metaCanBatchRetryFailed', false),
      canBatchRevealSummary: runtimeFlag('metaCanBatchRevealSummary', false),
      metaSavedSearch: valueById('metaSavedSearch'),
      metaSavedOptions: optionsById('metaSavedSelect'),
      metaSavedSelected: selectedValueById('metaSavedSelect'),
      metaSavedTagsDraft: valueById('metaSavedTags'),
      metaSavedMeta: textById('metaSavedMeta') || '—',
      canSaveMetaSavedTags: runtimeFlag('metaCanSaveSavedTags', false),
      canRefreshMetaSaved: runtimeFlag('metaCanRefreshSaved', !running),
      canLoadMetaSaved: runtimeFlag('metaCanLoadSaved', false),
      canOpenMetaSavedReport: runtimeFlag('metaCanOpenSavedReport', false),
      canRevealMetaSaved: runtimeFlag('metaCanRevealSaved', false),
      canDeleteMetaSaved: runtimeFlag('metaCanDeleteSaved', false),
      stage4SavedSearch: valueById('stage4SavedSearch'),
      stage4SavedOptions: optionsById('stage4SavedSelect'),
      stage4SavedSelected: selectedValueById('stage4SavedSelect'),
      stage4SavedTagsDraft: valueById('stage4SavedTags'),
      stage4SavedMeta: textById('stage4SavedMeta') || '—',
      canSaveStage4SavedTags: runtimeFlag('metaCanSaveStage4SavedTags', false),
      canRefreshStage4Saved: runtimeFlag('metaCanRefreshStage4Saved', !running),
      canLoadStage4Saved: runtimeFlag('metaCanLoadStage4Saved', false),
      canRevealStage4Saved: runtimeFlag('metaCanRevealStage4Saved', false),
      canDeleteStage4Saved: runtimeFlag('metaCanDeleteStage4Saved', false),
      metaUseMlConfidence: checkedById('metaUseMlConfidence'),
      metaMlAutopilot: checkedById('metaMlAutopilot'),
      canMetaMlRefresh: runtimeFlag('metaCanMlRefresh', !running),
      canMetaMlTrain: runtimeFlag('metaCanMlTrain', !running),
      canMetaMlApply: runtimeFlag('metaCanMlApply', false),
      canMetaMlDelete: runtimeFlag('metaCanMlDelete', !running),
      metaMlStatus: textById('metaMlStatus') || '—',
      metaStage4TopK: valueById('metaStage4TopK') || '10',
      metaStage4MinConf: valueById('metaStage4MinConf') || '0.55',
      metaStage4MetaWeight: valueById('metaStage4MetaWeight') || '0.60',
      metaStage4MlWeight: valueById('metaStage4MlWeight') || '0.35',
      metaStage4UncertaintyWeight: valueById('metaStage4UncertaintyWeight') || '0.05',
      metaStage4AcceptConf: valueById('metaStage4AcceptConf') || '0.70',
      metaStage4ReviewConf: valueById('metaStage4ReviewConf') || '0.55',
      metaStage4MaxAcceptUncertainty: valueById('metaStage4MaxAcceptUncertainty') || '0.45',
      metaStage4AutoTrain: checkedById('metaStage4AutoTrain'),
      metaStage4RequireHealthy: checkedById('metaStage4RequireHealthy'),
      metaStage4FallbackToMeta: checkedById('metaStage4FallbackToMeta'),
      canRunMetaStage4: runtimeFlag('metaCanRunStage4', false),
      canRevealMetaStage4: runtimeFlag('metaCanRevealStage4', false),
      metaStage4Status: textById('metaStage4Status') || '—',
      metaStage4OutputPreview: preTextById('metaStage4Output'),
      canRunMeta: runtimeFlag('metaCanRun', !running),
      canOpenMetaReport: runtimeFlag('metaCanOpenReport', false),
      canRevealMetaFiles: runtimeFlag('metaCanRevealFiles', false),
      debugOutputPreview: preTextById('output'),
      topRows,
      topFilterMeta: textById('topFilterMeta') || '-',
      topSelectionGuardrail: textById('topSelectionGuardrail') || '—',
      paretoMeta: textById('paretoMetaVue') || textById('paretoMeta') || '-',
      drillMeta: textById('drillMetaVue') || textById('drillMeta') || '-',
      drillStatus: textById('drillStatusVue') || textById('drillStatus') || 'Click a Top row to view curves.',
      stabilityStatus: textById('stabilityStatusVue') || textById('stabilityStatus') || '-',
      stabilityMeta: textById('stabilityMetaVue') || textById('stabilityMeta') || '-',
      heatmapMeta: textById('heatmapMetaVue') || textById('heatmapMeta') || '-',
      statusLine: textById('status') || 'Ready.',
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const textObserver = observeTextNodes(ANALYSIS_TEXT_IDS, scheduleSync)
  const tabObserver = new MutationObserver(() => scheduleSync())
  const rootResultsObserver = new MutationObserver(() => scheduleSync())
  rootResultsObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-results-tab'],
  })
  tabObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-aggregate-tab'],
  })
  const rootRuntimeObserver = new MutationObserver(() => scheduleSync())
  rootRuntimeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: [
      'data-running',
      'data-overview-primary-label',
      'data-overview-primary-action',
      'data-overview-primary-disabled',
      'data-overview-secondary-visible',
      'data-overview-secondary-label',
      'data-overview-secondary-action',
      'data-overview-secondary-disabled',
      'data-ana-can-copy-best',
      'data-ana-can-export-csv',
      'data-ana-can-export-json',
      'data-ana-can-set-baseline',
      'data-ana-can-clear-baseline',
      'data-ana-can-clear-top-filters',
      'data-ana-can-heatmap-export-png',
      'data-meta-can-run-batch',
      'data-meta-can-batch-stop',
      'data-meta-can-batch-refresh',
      'data-meta-can-batch-select-all',
      'data-meta-can-batch-select-none',
      'data-meta-can-batch-retry-failed',
      'data-meta-can-batch-reveal-summary',
      'data-meta-can-run',
      'data-meta-can-open-report',
      'data-meta-can-reveal-files',
      'data-meta-can-ml-refresh',
      'data-meta-can-ml-train',
      'data-meta-can-ml-apply',
      'data-meta-can-ml-delete',
      'data-meta-can-run-stage4',
      'data-meta-can-reveal-stage4',
      'data-meta-can-save-saved-tags',
      'data-meta-can-refresh-saved',
      'data-meta-can-load-saved',
      'data-meta-can-open-saved-report',
      'data-meta-can-reveal-saved',
      'data-meta-can-delete-saved',
      'data-meta-can-save-stage4-saved-tags',
      'data-meta-can-refresh-stage4-saved',
      'data-meta-can-load-stage4-saved',
      'data-meta-can-reveal-stage4-saved',
      'data-meta-can-delete-stage4-saved',
    ],
  })
  const topBodyObserver = new MutationObserver(() => scheduleSync())
  const topTableBody = document.getElementById('topTableBody')
  if (topTableBody) {
    topBodyObserver.observe(topTableBody, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['class'],
    })
  }
  const topHeadObserver = new MutationObserver(() => scheduleSync())
  const topTableHead = document.querySelector('#topTable thead')
  if (topTableHead) {
    topHeadObserver.observe(topTableHead, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['class', 'data-sort', 'aria-sort'],
    })
  }
  const metaTableObserver = new MutationObserver(() => scheduleSync())
  const metaTableHead = document.querySelector('#metaTable thead')
  if (metaTableHead) {
    metaTableObserver.observe(metaTableHead, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class'],
    })
  }
  const metaTableBody = document.getElementById('metaTableBody')
  if (metaTableBody) {
    metaTableObserver.observe(metaTableBody, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class'],
    })
  }
  const batchObserver = new MutationObserver(() => scheduleSync())
  const batchDatasets = document.getElementById('batchDatasets')
  if (batchDatasets) {
    batchObserver.observe(batchDatasets, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class', 'style', 'disabled', 'checked', 'value'],
    })
  }
  const batchOutput = document.getElementById('batchOutput')
  if (batchOutput) {
    batchObserver.observe(batchOutput, {
      childList: true,
      subtree: true,
      characterData: true,
    })
  }
  const onInputLike = () => scheduleSync()
  const attachedTopInputs: (HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement)[] = []
  for (const id of ANALYSIS_VALUE_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
    if (!el) continue
    attachedTopInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }
  const paretoX = document.getElementById('paretoX') as HTMLSelectElement | null
  const paretoY = document.getElementById('paretoY') as HTMLSelectElement | null
  const heatmapColor = document.getElementById('heatmapColor') as HTMLSelectElement | null
  const heatmapRange = document.getElementById('heatmapRange') as HTMLSelectElement | null
  const heatmapShowTop = document.getElementById('heatmapShowTop') as HTMLSelectElement | null
  const heatmapShowCrosshair = document.getElementById('heatmapShowCrosshair') as HTMLSelectElement | null
  paretoX?.addEventListener('change', onInputLike)
  paretoY?.addEventListener('change', onInputLike)
  heatmapColor?.addEventListener('change', onInputLike)
  heatmapRange?.addEventListener('change', onInputLike)
  heatmapShowTop?.addEventListener('change', onInputLike)
  heatmapShowCrosshair?.addEventListener('change', onInputLike)
  const paretoObserver = new MutationObserver(() => scheduleSync())
  const paretoWrapLegacy = document.getElementById('paretoWrap')
  if (paretoWrapLegacy) {
    paretoObserver.observe(paretoWrapLegacy, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }
  const paretoWrapVue = document.getElementById('paretoWrapVue')
  if (paretoWrapVue) {
    paretoObserver.observe(paretoWrapVue, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }
  for (const id of ['drillWrap', 'drillWrapVue', 'stabilityWrap', 'stabilityWrapVue', 'tradeStatsWrap', 'tradeStatsWrapVue'] as const) {
    const node = document.getElementById(id)
    if (!node) continue
    paretoObserver.observe(node, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }
  for (const id of ['heatmapWrap', 'heatmapWrapVue', 'heatmapColor', 'heatmapRange', 'heatmapShowTop', 'heatmapShowCrosshair'] as const) {
    const node = document.getElementById(id)
    if (!node) continue
    paretoObserver.observe(node, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['style', 'class', 'value', 'selected', 'disabled'],
    })
  }
  if (paretoX) {
    paretoObserver.observe(paretoX, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['value', 'selected', 'disabled', 'class'],
    })
  }
  if (paretoY) {
    paretoObserver.observe(paretoY, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['value', 'selected', 'disabled', 'class'],
    })
  }

  const overviewObserver = new MutationObserver(() => scheduleSync())
  for (const id of ['overviewEmpty', 'overviewContent', 'bestGrid'] as const) {
    const node = document.getElementById(id)
    if (!node) continue
    overviewObserver.observe(node, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }

  const onAnalysisSnapshot = (): void => scheduleSync()
  window.addEventListener(ANALYSIS_SNAPSHOT_EVENT, onAnalysisSnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    tabObserver.disconnect()
    rootResultsObserver.disconnect()
    rootRuntimeObserver.disconnect()
    topBodyObserver.disconnect()
    topHeadObserver.disconnect()
    metaTableObserver.disconnect()
    batchObserver.disconnect()
    paretoObserver.disconnect()
    overviewObserver.disconnect()
    window.removeEventListener(ANALYSIS_SNAPSHOT_EVENT, onAnalysisSnapshot)
    for (const el of attachedTopInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
    paretoX?.removeEventListener('change', onInputLike)
    paretoY?.removeEventListener('change', onInputLike)
    heatmapColor?.removeEventListener('change', onInputLike)
    heatmapRange?.removeEventListener('change', onInputLike)
    heatmapShowTop?.removeEventListener('change', onInputLike)
    heatmapShowCrosshair?.removeEventListener('change', onInputLike)
  }
}

export function attachHistoryContextBridge(): () => void {
  const store = useHistoryStore()

  const syncNow = () => {
    const running = runtimeRunning()
    const canMutate = !running
    store.applyLegacySnapshot({
      sessionRunCount: countOptions('historySelect'),
      sessionOptions: optionsById('historySelect'),
      sessionSelectedValue: selectedValueById('historySelect'),
      sessionSelectedLabel: selectedLabelById('historySelect', 'No runs yet'),
      savedRunCount: countOptions('savedRunsSelect'),
      savedOptions: optionsById('savedRunsSelect'),
      savedSelectedValue: selectedValueById('savedRunsSelect'),
      savedSelectedLabel: selectedLabelById('savedRunsSelect', 'No saved runs'),
      compareRunCount: countOptions('compareSelect'),
      compareOptions: optionsById('compareSelect'),
      compareSelectedValue: selectedValueById('compareSelect'),
      savedRunsSearch: valueById('savedRunsSearch'),
      savedRunsTagsDraft: valueById('savedRunsTags'),
      savedRunsMeta: textById('savedRunsMeta') || '—',
      savedRunDetailTitle: textById('savedRunDetailTitle') || 'No saved run selected',
      savedRunDetailMeta: textById('savedRunDetailMeta') || 'Select a run to inspect metadata and settings.',
      savedRunDetailStrategy: textById('savedRunDetailStrategy') || '—',
      savedRunDetailDataset: textById('savedRunDetailDataset') || '—',
      savedRunDetailTimeframe: textById('savedRunDetailTimeframe') || '—',
      savedRunDetailValidation: textById('savedRunDetailValidation') || '—',
      savedRunDetailReport: textById('savedRunDetailReport') || '—',
      savedRunDetailSize: textById('savedRunDetailSize') || '—',
      savedRunDetailTags: textById('savedRunDetailTags') || '—',
      savedRunDetailParams: preTextById('savedRunDetailParams'),
      canApplyHistory: runtimeFlag('histCanApplyHistory', true),
      canExportPreset: runtimeFlag('histCanExportPreset', true),
      canImportPreset: runtimeFlag('histCanImportPreset', true),
      canClearHistory: runtimeFlag('histCanClearHistory', true),
      canSaveSavedRunTags: runtimeFlag('histCanSaveSavedRunTags', false),
      canRefreshSavedRuns: runtimeFlag('histCanRefreshSavedRuns', true),
      canLoadSavedRun: runtimeFlag('histCanLoadSavedRun', false),
      canApplySavedRun: runtimeFlag('histCanApplySavedRun', false),
      canRerunSavedRun: runtimeFlag('histCanRerunSavedRun', false),
      canOpenSavedRunReport: runtimeFlag('histCanOpenSavedRunReport', false),
      canRevealSavedRun: runtimeFlag('histCanRevealSavedRun', false),
      canDeleteSavedRun: runtimeFlag('histCanDeleteSavedRun', false),
      canOpenRunsDir: canMutate,
      statusLine: textById('status') || 'Ready.',
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedSelects: HTMLSelectElement[] = []
  const attachedInputs: (HTMLInputElement | HTMLTextAreaElement)[] = []
  for (const id of HISTORY_SELECT_IDS) {
    const el = document.getElementById(id) as HTMLSelectElement | null
    if (!el) continue
    attachedSelects.push(el)
    el.addEventListener('change', onInputLike)
    el.addEventListener('input', onInputLike)
  }
  for (const id of HISTORY_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(HISTORY_TEXT_IDS, scheduleSync)
  const selectObserver = new MutationObserver(() => scheduleSync())
  for (const id of HISTORY_SELECT_IDS) {
    const el = document.getElementById(id)
    if (!el) continue
    selectObserver.observe(el, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['value', 'selected'],
    })
  }
  const rootRunningObserver = new MutationObserver(() => scheduleSync())
  rootRunningObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: [
      'data-running',
      'data-hist-can-apply-history',
      'data-hist-can-export-preset',
      'data-hist-can-import-preset',
      'data-hist-can-clear-history',
      'data-hist-can-save-saved-run-tags',
      'data-hist-can-refresh-saved-runs',
      'data-hist-can-load-saved-run',
      'data-hist-can-apply-saved-run',
      'data-hist-can-rerun-saved-run',
      'data-hist-can-open-saved-run-report',
      'data-hist-can-reveal-saved-run',
      'data-hist-can-delete-saved-run',
    ],
  })

  const onHistorySnapshot = (): void => scheduleSync()
  window.addEventListener(HISTORY_SNAPSHOT_EVENT, onHistorySnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    selectObserver.disconnect()
    rootRunningObserver.disconnect()
    window.removeEventListener(HISTORY_SNAPSHOT_EVENT, onHistorySnapshot)
    for (const el of attachedSelects) {
      el.removeEventListener('change', onInputLike)
      el.removeEventListener('input', onInputLike)
    }
    for (const el of attachedInputs) {
      el.removeEventListener('change', onInputLike)
      el.removeEventListener('input', onInputLike)
    }
  }
}

export function attachSettingsContextBridge(): () => void {
  const store = useSettingsStore()
  const root = document.documentElement

  const syncNow = () => {
    const running = String(root.dataset.running || '').trim() === '1'
    const canMutate = !running
    const theme = String(root.dataset.themePref || '').trim()
      || String(root.dataset.theme || '').trim()
      || 'system'
    const effect = String(root.dataset.windowEffect || '').trim()
      || 'none'
    const workflowAdvanced = String(root.dataset.workflowAdvanced || '').trim() === '1'
    const controlsCollapsed = String(root.dataset.controlsCollapsed || '').trim() === '1'
    const marketDataDir = valueById('marketDataDir')
    const moveExisting = checkedById('marketDataMoveExisting')
    const marketDataDirHint = textById('marketDataDirHint') || '—'

    store.applyLegacySnapshot({
      theme,
      effect,
      workflowAdvanced,
      controlsCollapsed,
      marketDataDir,
      moveExisting,
      marketDataDirHint,
      canBrowseMarketDataDir: canMutate,
      canSetMarketDataDir: canMutate,
      canResetMarketDataDir: canMutate,
      canOpenMarketDataDir: canMutate,
      canOpenWorkflowData: canMutate,
      canOpenRunsDir: canMutate,
      canOpenReportsDir: canMutate,
      canOpenPolygonModal: canMutate,
      canGotoLibrary: true,
      canGotoAggregate: true,
      statusLine: textById('status') || 'Ready.',
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement)[] = []
  for (const id of SETTINGS_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(SETTINGS_TEXT_IDS, scheduleSync)
  const rootObserver = new MutationObserver(() => scheduleSync())
  rootObserver.observe(root, {
    attributes: true,
    attributeFilter: ['data-theme', 'data-theme-pref', 'data-window-effect', 'data-workflow-advanced', 'data-controls-collapsed', 'data-running'],
  })
  const buttonObserver = new MutationObserver(() => scheduleSync())
  for (const id of SETTINGS_BUTTON_IDS) {
    const el = document.getElementById(id)
    if (!el) continue
    buttonObserver.observe(el, {
      attributes: true,
      attributeFilter: ['disabled', 'class'],
    })
  }

  syncNow()

  return () => {
    textObserver.disconnect()
    rootObserver.disconnect()
    buttonObserver.disconnect()
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}
