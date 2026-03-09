import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import * as analysisService from '../services/analysisService'
import * as metaService from '../services/metaService'
import { useDataStore } from './dataStore'
import { useHistoryStore } from './historyStore'
import { useOptimizeStore } from './optimizeStore'
import { useWorkflowStore } from './workflowStore'

function rawArray<T>(value: unknown, fallback: T[]): T[] {
  return Array.isArray(value) ? markRaw(value as T[]) : fallback
}

function runOverviewEmptyAction(actionName: string): void {
  const action = String(actionName || '').trim()
  if (!action) return

  const workflowStore = useWorkflowStore()
  const dataStore = useDataStore()
  const optimizeStore = useOptimizeStore()

  if (action === 'browse_csv') {
    workflowStore.selectWorkspace('workflow', 'data')
    dataStore.browseCsv()
    return
  }

  if (action === 'polygon') {
    workflowStore.selectWorkspace('workflow', 'data')
    dataStore.openPolygon()
    return
  }

  if (action === 'go_opt') {
    workflowStore.selectWorkspace('workflow', 'optimize')
    workflowStore.selectControlsTab('opt')
    return
  }

  if (action === 'run_opt') {
    workflowStore.selectWorkspace('workflow', 'optimize')
    workflowStore.selectControlsTab('opt')
    optimizeStore.runOptimization()
    return
  }

  if (action === 'cancel_run') {
    optimizeStore.cancelOptimization()
  }
}

interface AnalysisState {
  activeResultsTab: string
  topColumns: { key: string; label: string; sort: '' | 'asc' | 'desc'; alignLeft: boolean }[]
  topTableRows: { selected: boolean; cells: { text: string; className: string; title: string }[] }[]
  bestKicker: string
  bestTitle: string
  bestMeta: string
  bestSummary: string
  bestStats: { label: string; value: string; tone: string }[]
  overviewEmptyVisible: boolean
  overviewEmptyTitle: string
  overviewEmptyBody: string
  overviewEmptyHint: string
  overviewPrimaryLabel: string
  overviewPrimaryAction: string
  overviewPrimaryDisabled: boolean
  overviewSecondaryVisible: boolean
  overviewSecondaryLabel: string
  overviewSecondaryAction: string
  overviewSecondaryDisabled: boolean
  compareSummary: string
  canCopyBest: boolean
  canExportCsv: boolean
  canExportJson: boolean
  canSetBaseline: boolean
  canClearBaseline: boolean
  filterMinPnl: string
  filterMinSharpe: string
  filterMaxDd: string
  filterMinTrades: string
  filterMinExposure: string
  canClearTopFilters: boolean
  paretoX: string
  paretoY: string
  paretoXOptions: { value: string; label: string; disabled: boolean }[]
  paretoYOptions: { value: string; label: string; disabled: boolean }[]
  paretoWrapVisible: boolean
  drillWrapVisible: boolean
  stabilityWrapVisible: boolean
  tradeStatsWrapVisible: boolean
  heatmapWrapVisible: boolean
  heatmapColor: string
  heatmapColorOptions: { value: string; label: string; disabled: boolean }[]
  heatmapRange: string
  heatmapRangeOptions: { value: string; label: string; disabled: boolean }[]
  heatmapShowTop: string
  heatmapShowTopOptions: { value: string; label: string; disabled: boolean }[]
  heatmapShowCrosshair: string
  heatmapShowCrosshairOptions: { value: string; label: string; disabled: boolean }[]
  canHeatmapExportPng: boolean
  metaStrategyKind: string
  metaStrategyOptions: { value: string; label: string; disabled: boolean }[]
  metaTimeframe: string
  metaRequiredTag: string
  metaMinSamples: string
  metaTopK: string
  metaHoldoutTag: string
  metaEvalHoldout: boolean
  metaSaveHoldoutRows: boolean
  metaHoldoutMode: string
  metaHoldoutReps: string
  metaHoldoutEvalTopN: string
  metaHoldoutEvalRankBy: string
  metaHoldoutExploreN: string
  metaGroupingMode: string
  metaClusterIntBucket: string
  metaClusterFloatBucket: string
  metaHoldoutNote: string
  aggregateSummarySavedRuns: string
  aggregateSummarySavedMeta: string
  aggregateSummaryLatestGroups: string
  aggregateSummaryLatestArtifacts: string
  aggregateSummaryLatestHoldout: string
  aggregateSummaryLatestRuntime: string
  metaMeta: string
  metaOutputPreview: string
  metaTableColumns: { label: string; alignLeft: boolean }[]
  metaTableRows: { cells: { text: string; className: string; title: string }[] }[]
  metaCompareOptions: { value: string; label: string; disabled: boolean }[]
  metaCompareSelected: string
  metaCompareSummary: string
  batchDatasetSearch: string
  batchTags: string
  batchHoldoutPct: string
  batchHoldoutTag: string
  batchAutoMeta: boolean
  batchSkipDone: boolean
  batchDatasets: { path: string; label: string; meta: string; checked: boolean; disabled: boolean }[]
  batchSelectedCount: number
  batchTotalCount: number
  batchMeta: string
  batchOutputPreview: string
  canRunBatch: boolean
  canBatchStop: boolean
  canBatchRefresh: boolean
  canBatchSelectAll: boolean
  canBatchSelectNone: boolean
  canBatchRetryFailed: boolean
  canBatchRevealSummary: boolean
  metaSavedSearch: string
  metaSavedOptions: { value: string; label: string; disabled: boolean }[]
  metaSavedSelected: string
  metaSavedTagsDraft: string
  metaSavedMeta: string
  canSaveMetaSavedTags: boolean
  canRefreshMetaSaved: boolean
  canLoadMetaSaved: boolean
  canOpenMetaSavedReport: boolean
  canRevealMetaSaved: boolean
  canDeleteMetaSaved: boolean
  stage4SavedSearch: string
  stage4SavedOptions: { value: string; label: string; disabled: boolean }[]
  stage4SavedSelected: string
  stage4SavedTagsDraft: string
  stage4SavedMeta: string
  canSaveStage4SavedTags: boolean
  canRefreshStage4Saved: boolean
  canLoadStage4Saved: boolean
  canRevealStage4Saved: boolean
  canDeleteStage4Saved: boolean
  metaUseMlConfidence: boolean
  metaMlAutopilot: boolean
  canMetaMlRefresh: boolean
  canMetaMlTrain: boolean
  canMetaMlApply: boolean
  canMetaMlDelete: boolean
  metaMlStatus: string
  metaStage4TopK: string
  metaStage4MinConf: string
  metaStage4MetaWeight: string
  metaStage4MlWeight: string
  metaStage4UncertaintyWeight: string
  metaStage4AcceptConf: string
  metaStage4ReviewConf: string
  metaStage4MaxAcceptUncertainty: string
  metaStage4AutoTrain: boolean
  metaStage4RequireHealthy: boolean
  metaStage4FallbackToMeta: boolean
  canRunMetaStage4: boolean
  canRevealMetaStage4: boolean
  metaStage4Status: string
  metaStage4OutputPreview: string
  canRunMeta: boolean
  canOpenMetaReport: boolean
  canRevealMetaFiles: boolean
  debugOutputPreview: string
  topRows: number
  topFilterMeta: string
  topSelectionGuardrail: string
  paretoMeta: string
  drillMeta: string
  drillStatus: string
  stabilityStatus: string
  stabilityMeta: string
  heatmapMeta: string
  heatmapStatusLine: string
  statusLine: string
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

function applyVueAnalysisFieldUpdate(state: AnalysisState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false

  switch (key) {
    case 'filterMinPnl':
      state.filterMinPnl = next
      return true
    case 'filterMinSharpe':
      state.filterMinSharpe = next
      return true
    case 'filterMaxDd':
      state.filterMaxDd = next
      return true
    case 'filterMinTrades':
      state.filterMinTrades = next
      return true
    case 'filterMinExposure':
      state.filterMinExposure = next
      return true
    case 'paretoX':
      state.paretoX = next || 'max_dd'
      return true
    case 'paretoY':
      state.paretoY = next || 'pnl'
      return true
    case 'heatmapColor':
      state.heatmapColor = next || 'cividis'
      return true
    case 'heatmapRange':
      state.heatmapRange = next || 'q2_98'
      return true
    case 'heatmapShowTop':
      state.heatmapShowTop = next || 'true'
      return true
    case 'heatmapShowCrosshair':
      state.heatmapShowCrosshair = next || 'true'
      return true
    default:
      return false
  }
}

function applyVueMetaTextFieldUpdate(state: AnalysisState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false

  switch (key) {
    case 'metaStrategyKind':
      state.metaStrategyKind = next
      return true
    case 'metaTimeframe':
      state.metaTimeframe = next
      return true
    case 'metaRequiredTag':
      state.metaRequiredTag = next
      return true
    case 'metaMinSamples':
      state.metaMinSamples = next
      return true
    case 'metaTopK':
      state.metaTopK = next
      return true
    case 'metaHoldoutTag':
      state.metaHoldoutTag = next
      return true
    case 'metaHoldoutMode':
      state.metaHoldoutMode = next
      return true
    case 'metaHoldoutReps':
      state.metaHoldoutReps = next
      return true
    case 'metaHoldoutEvalTopN':
      state.metaHoldoutEvalTopN = next
      return true
    case 'metaHoldoutEvalRankBy':
      state.metaHoldoutEvalRankBy = next
      return true
    case 'metaHoldoutExploreN':
      state.metaHoldoutExploreN = next
      return true
    case 'metaGroupingMode':
      state.metaGroupingMode = next
      return true
    case 'metaClusterIntBucket':
      state.metaClusterIntBucket = next
      return true
    case 'metaClusterFloatBucket':
      state.metaClusterFloatBucket = next
      return true
    case 'metaCompareSelect':
      state.metaCompareSelected = next
      return true
    case 'batchDatasetSearch':
      state.batchDatasetSearch = next
      return true
    case 'batchHoldoutPct':
      state.batchHoldoutPct = next
      return true
    case 'batchHoldoutTag':
      state.batchHoldoutTag = next
      return true
    case 'batchTags':
      state.batchTags = next
      return true
    case 'metaSavedSearch':
      state.metaSavedSearch = next
      return true
    case 'metaSavedSelect':
      state.metaSavedSelected = next
      return true
    case 'metaSavedTags':
      state.metaSavedTagsDraft = next
      return true
    case 'stage4SavedSearch':
      state.stage4SavedSearch = next
      return true
    case 'stage4SavedSelect':
      state.stage4SavedSelected = next
      return true
    case 'stage4SavedTags':
      state.stage4SavedTagsDraft = next
      return true
    case 'metaStage4TopK':
      state.metaStage4TopK = next
      return true
    case 'metaStage4MinConf':
      state.metaStage4MinConf = next
      return true
    case 'metaStage4MetaWeight':
      state.metaStage4MetaWeight = next
      return true
    case 'metaStage4MlWeight':
      state.metaStage4MlWeight = next
      return true
    case 'metaStage4UncertaintyWeight':
      state.metaStage4UncertaintyWeight = next
      return true
    case 'metaStage4AcceptConf':
      state.metaStage4AcceptConf = next
      return true
    case 'metaStage4ReviewConf':
      state.metaStage4ReviewConf = next
      return true
    case 'metaStage4MaxAcceptUncertainty':
      state.metaStage4MaxAcceptUncertainty = next
      return true
    default:
      return false
  }
}

function applyVueMetaCheckboxFieldUpdate(state: AnalysisState, field: string, checked: boolean): boolean {
  const key = String(field || '').trim()
  const next = !!checked
  if (!key) return false

  switch (key) {
    case 'metaEvalHoldout':
      state.metaEvalHoldout = next
      return true
    case 'metaSaveHoldoutRows':
      state.metaSaveHoldoutRows = next
      return true
    case 'batchAutoMeta':
      state.batchAutoMeta = next
      return true
    case 'batchSkipDone':
      state.batchSkipDone = next
      return true
    case 'metaStage4AutoTrain':
      state.metaStage4AutoTrain = next
      return true
    case 'metaStage4RequireHealthy':
      state.metaStage4RequireHealthy = next
      return true
    case 'metaStage4FallbackToMeta':
      state.metaStage4FallbackToMeta = next
      return true
    case 'metaUseMlConfidence':
      state.metaUseMlConfidence = next
      return true
    case 'metaMlAutopilot':
      state.metaMlAutopilot = next
      return true
    default:
      return false
  }
}

export const useAnalysisStore = defineStore('analysis', {
  state: (): AnalysisState => ({
    activeResultsTab: 'overview',
    topColumns: [],
    topTableRows: [],
    bestKicker: '—',
    bestTitle: '—',
    bestMeta: '—',
    bestSummary: '-',
    bestStats: [],
    overviewEmptyVisible: false,
    overviewEmptyTitle: 'Load data to begin',
    overviewEmptyBody: 'Load a CSV dataset or download candles from Polygon, then run an optimization to view results here.',
    overviewEmptyHint: 'Tip: start with a small dataset to validate your strategy settings.',
    overviewPrimaryLabel: 'Browse CSV…',
    overviewPrimaryAction: 'browse_csv',
    overviewPrimaryDisabled: false,
    overviewSecondaryVisible: true,
    overviewSecondaryLabel: 'Polygon…',
    overviewSecondaryAction: 'polygon',
    overviewSecondaryDisabled: false,
    compareSummary: '-',
    canCopyBest: true,
    canExportCsv: true,
    canExportJson: true,
    canSetBaseline: true,
    canClearBaseline: true,
    filterMinPnl: '',
    filterMinSharpe: '',
    filterMaxDd: '',
    filterMinTrades: '',
    filterMinExposure: '',
    canClearTopFilters: true,
    paretoX: 'max_dd',
    paretoY: 'pnl',
    paretoXOptions: [],
    paretoYOptions: [],
    paretoWrapVisible: false,
    drillWrapVisible: false,
    stabilityWrapVisible: false,
    tradeStatsWrapVisible: false,
    heatmapWrapVisible: false,
    heatmapColor: 'cividis',
    heatmapColorOptions: [],
    heatmapRange: 'q2_98',
    heatmapRangeOptions: [],
    heatmapShowTop: 'true',
    heatmapShowTopOptions: [],
    heatmapShowCrosshair: 'true',
    heatmapShowCrosshairOptions: [],
    canHeatmapExportPng: false,
    metaStrategyKind: 'CompositeMo',
    metaStrategyOptions: [],
    metaTimeframe: '',
    metaRequiredTag: '',
    metaMinSamples: '30',
    metaTopK: '25',
    metaHoldoutTag: 'holdout',
    metaEvalHoldout: true,
    metaSaveHoldoutRows: false,
    metaHoldoutMode: 'light',
    metaHoldoutReps: '2',
    metaHoldoutEvalTopN: '0',
    metaHoldoutEvalRankBy: 'auto',
    metaHoldoutExploreN: '0',
    metaGroupingMode: 'clustered',
    metaClusterIntBucket: '5',
    metaClusterFloatBucket: '0.25',
    metaHoldoutNote: '—',
    aggregateSummarySavedRuns: '0',
    aggregateSummarySavedMeta: '0',
    aggregateSummaryLatestGroups: '—',
    aggregateSummaryLatestArtifacts: '—',
    aggregateSummaryLatestHoldout: '—',
    aggregateSummaryLatestRuntime: '—',
    metaMeta: '—',
    metaOutputPreview: '-',
    metaTableColumns: [],
    metaTableRows: [],
    metaCompareOptions: [],
    metaCompareSelected: '',
    metaCompareSummary: '-',
    batchDatasetSearch: '',
    batchTags: '',
    batchHoldoutPct: '15',
    batchHoldoutTag: 'holdout',
    batchAutoMeta: true,
    batchSkipDone: true,
    batchDatasets: [],
    batchSelectedCount: 0,
    batchTotalCount: 0,
    batchMeta: '—',
    batchOutputPreview: '-',
    canRunBatch: true,
    canBatchStop: false,
    canBatchRefresh: true,
    canBatchSelectAll: true,
    canBatchSelectNone: true,
    canBatchRetryFailed: false,
    canBatchRevealSummary: false,
    metaSavedSearch: '',
    metaSavedOptions: [],
    metaSavedSelected: '',
    metaSavedTagsDraft: '',
    metaSavedMeta: '—',
    canSaveMetaSavedTags: false,
    canRefreshMetaSaved: true,
    canLoadMetaSaved: false,
    canOpenMetaSavedReport: false,
    canRevealMetaSaved: false,
    canDeleteMetaSaved: false,
    stage4SavedSearch: '',
    stage4SavedOptions: [],
    stage4SavedSelected: '',
    stage4SavedTagsDraft: '',
    stage4SavedMeta: '—',
    canSaveStage4SavedTags: false,
    canRefreshStage4Saved: true,
    canLoadStage4Saved: false,
    canRevealStage4Saved: false,
    canDeleteStage4Saved: false,
    metaUseMlConfidence: false,
    metaMlAutopilot: true,
    canMetaMlRefresh: true,
    canMetaMlTrain: true,
    canMetaMlApply: false,
    canMetaMlDelete: true,
    metaMlStatus: '—',
    metaStage4TopK: '10',
    metaStage4MinConf: '0.55',
    metaStage4MetaWeight: '0.60',
    metaStage4MlWeight: '0.35',
    metaStage4UncertaintyWeight: '0.05',
    metaStage4AcceptConf: '0.70',
    metaStage4ReviewConf: '0.55',
    metaStage4MaxAcceptUncertainty: '0.45',
    metaStage4AutoTrain: true,
    metaStage4RequireHealthy: true,
    metaStage4FallbackToMeta: true,
    canRunMetaStage4: false,
    canRevealMetaStage4: false,
    metaStage4Status: '—',
    metaStage4OutputPreview: '-',
    canRunMeta: true,
    canOpenMetaReport: true,
    canRevealMetaFiles: true,
    debugOutputPreview: '{}',
    topRows: 0,
    topFilterMeta: '-',
    topSelectionGuardrail: '—',
    paretoMeta: '-',
    drillMeta: '-',
    drillStatus: 'Click a Top row to view curves.',
    stabilityStatus: '-',
    stabilityMeta: '-',
    heatmapMeta: '-',
    heatmapStatusLine: 'Heatmap unavailable.',
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<AnalysisState>): void {
      this.activeResultsTab = String(snapshot.activeResultsTab ?? this.activeResultsTab ?? 'overview').trim() || 'overview'
      this.topColumns = rawArray(snapshot.topColumns, this.topColumns)
      this.topTableRows = rawArray(snapshot.topTableRows, this.topTableRows)
      this.bestKicker = String(snapshot.bestKicker ?? this.bestKicker ?? '—').trim() || '—'
      this.bestTitle = String(snapshot.bestTitle ?? this.bestTitle ?? '—').trim() || '—'
      this.bestMeta = String(snapshot.bestMeta ?? this.bestMeta ?? '—').trim() || '—'
      this.bestSummary = String(snapshot.bestSummary ?? this.bestSummary ?? '-').trim() || '-'
      this.bestStats = rawArray(snapshot.bestStats, this.bestStats)
      this.overviewEmptyVisible = Boolean(snapshot.overviewEmptyVisible ?? this.overviewEmptyVisible)
      this.overviewEmptyTitle = String(snapshot.overviewEmptyTitle ?? this.overviewEmptyTitle ?? 'Load data to begin').trim() || 'Load data to begin'
      this.overviewEmptyBody = String(snapshot.overviewEmptyBody ?? this.overviewEmptyBody ?? 'Load a CSV dataset or download candles from Polygon, then run an optimization to view results here.').trim() || 'Load a CSV dataset or download candles from Polygon, then run an optimization to view results here.'
      this.overviewEmptyHint = String(snapshot.overviewEmptyHint ?? this.overviewEmptyHint ?? 'Tip: start with a small dataset to validate your strategy settings.').trim() || 'Tip: start with a small dataset to validate your strategy settings.'
      this.overviewPrimaryLabel = String(snapshot.overviewPrimaryLabel ?? this.overviewPrimaryLabel ?? 'Browse CSV…').trim() || 'Browse CSV…'
      this.overviewPrimaryAction = String(snapshot.overviewPrimaryAction ?? this.overviewPrimaryAction ?? 'browse_csv').trim() || 'browse_csv'
      this.overviewPrimaryDisabled = Boolean(snapshot.overviewPrimaryDisabled ?? this.overviewPrimaryDisabled)
      this.overviewSecondaryVisible = Boolean(snapshot.overviewSecondaryVisible ?? this.overviewSecondaryVisible)
      this.overviewSecondaryLabel = String(snapshot.overviewSecondaryLabel ?? this.overviewSecondaryLabel ?? 'Polygon…').trim() || 'Polygon…'
      this.overviewSecondaryAction = String(snapshot.overviewSecondaryAction ?? this.overviewSecondaryAction ?? 'polygon').trim() || 'polygon'
      this.overviewSecondaryDisabled = Boolean(snapshot.overviewSecondaryDisabled ?? this.overviewSecondaryDisabled)
      this.compareSummary = String(snapshot.compareSummary ?? this.compareSummary ?? '-').trim() || '-'
      this.canCopyBest = Boolean(snapshot.canCopyBest ?? this.canCopyBest)
      this.canExportCsv = Boolean(snapshot.canExportCsv ?? this.canExportCsv)
      this.canExportJson = Boolean(snapshot.canExportJson ?? this.canExportJson)
      this.canSetBaseline = Boolean(snapshot.canSetBaseline ?? this.canSetBaseline)
      this.canClearBaseline = Boolean(snapshot.canClearBaseline ?? this.canClearBaseline)
      this.filterMinPnl = String(snapshot.filterMinPnl ?? this.filterMinPnl ?? '').trim()
      this.filterMinSharpe = String(snapshot.filterMinSharpe ?? this.filterMinSharpe ?? '').trim()
      this.filterMaxDd = String(snapshot.filterMaxDd ?? this.filterMaxDd ?? '').trim()
      this.filterMinTrades = String(snapshot.filterMinTrades ?? this.filterMinTrades ?? '').trim()
      this.filterMinExposure = String(snapshot.filterMinExposure ?? this.filterMinExposure ?? '').trim()
      this.canClearTopFilters = Boolean(snapshot.canClearTopFilters ?? this.canClearTopFilters)
      this.paretoX = String(snapshot.paretoX ?? this.paretoX ?? 'max_dd').trim() || 'max_dd'
      this.paretoY = String(snapshot.paretoY ?? this.paretoY ?? 'pnl').trim() || 'pnl'
      this.paretoXOptions = rawArray(snapshot.paretoXOptions, this.paretoXOptions)
      this.paretoYOptions = rawArray(snapshot.paretoYOptions, this.paretoYOptions)
      this.paretoWrapVisible = Boolean(snapshot.paretoWrapVisible ?? this.paretoWrapVisible)
      this.drillWrapVisible = Boolean(snapshot.drillWrapVisible ?? this.drillWrapVisible)
      this.stabilityWrapVisible = Boolean(snapshot.stabilityWrapVisible ?? this.stabilityWrapVisible)
      this.tradeStatsWrapVisible = Boolean(snapshot.tradeStatsWrapVisible ?? this.tradeStatsWrapVisible)
      this.heatmapWrapVisible = Boolean(snapshot.heatmapWrapVisible ?? this.heatmapWrapVisible)
      this.heatmapColor = String(snapshot.heatmapColor ?? this.heatmapColor ?? 'cividis').trim() || 'cividis'
      this.heatmapColorOptions = rawArray(snapshot.heatmapColorOptions, this.heatmapColorOptions)
      this.heatmapRange = String(snapshot.heatmapRange ?? this.heatmapRange ?? 'q2_98').trim() || 'q2_98'
      this.heatmapRangeOptions = rawArray(snapshot.heatmapRangeOptions, this.heatmapRangeOptions)
      this.heatmapShowTop = String(snapshot.heatmapShowTop ?? this.heatmapShowTop ?? 'true').trim() || 'true'
      this.heatmapShowTopOptions = rawArray(snapshot.heatmapShowTopOptions, this.heatmapShowTopOptions)
      this.heatmapShowCrosshair = String(snapshot.heatmapShowCrosshair ?? this.heatmapShowCrosshair ?? 'true').trim() || 'true'
      this.heatmapShowCrosshairOptions = rawArray(snapshot.heatmapShowCrosshairOptions, this.heatmapShowCrosshairOptions)
      this.canHeatmapExportPng = Boolean(snapshot.canHeatmapExportPng ?? this.canHeatmapExportPng)
      this.metaStrategyKind = String(snapshot.metaStrategyKind ?? this.metaStrategyKind ?? 'CompositeMo').trim() || 'CompositeMo'
      this.metaStrategyOptions = rawArray(snapshot.metaStrategyOptions, this.metaStrategyOptions)
      this.metaTimeframe = String(snapshot.metaTimeframe ?? this.metaTimeframe ?? '').trim()
      this.metaRequiredTag = String(snapshot.metaRequiredTag ?? this.metaRequiredTag ?? '').trim()
      this.metaMinSamples = String(snapshot.metaMinSamples ?? this.metaMinSamples ?? '30').trim() || '30'
      this.metaTopK = String(snapshot.metaTopK ?? this.metaTopK ?? '25').trim() || '25'
      this.metaHoldoutTag = String(snapshot.metaHoldoutTag ?? this.metaHoldoutTag ?? 'holdout').trim() || 'holdout'
      this.metaEvalHoldout = Boolean(snapshot.metaEvalHoldout ?? this.metaEvalHoldout)
      this.metaSaveHoldoutRows = Boolean(snapshot.metaSaveHoldoutRows ?? this.metaSaveHoldoutRows)
      this.metaHoldoutMode = String(snapshot.metaHoldoutMode ?? this.metaHoldoutMode ?? 'light').trim() || 'light'
      this.metaHoldoutReps = String(snapshot.metaHoldoutReps ?? this.metaHoldoutReps ?? '2').trim() || '2'
      this.metaHoldoutEvalTopN = String(snapshot.metaHoldoutEvalTopN ?? this.metaHoldoutEvalTopN ?? '0').trim() || '0'
      this.metaHoldoutEvalRankBy = String(snapshot.metaHoldoutEvalRankBy ?? this.metaHoldoutEvalRankBy ?? 'auto').trim() || 'auto'
      this.metaHoldoutExploreN = String(snapshot.metaHoldoutExploreN ?? this.metaHoldoutExploreN ?? '0').trim() || '0'
      this.metaGroupingMode = String(snapshot.metaGroupingMode ?? this.metaGroupingMode ?? 'clustered').trim() || 'clustered'
      this.metaClusterIntBucket = String(snapshot.metaClusterIntBucket ?? this.metaClusterIntBucket ?? '5').trim() || '5'
      this.metaClusterFloatBucket = String(snapshot.metaClusterFloatBucket ?? this.metaClusterFloatBucket ?? '0.25').trim() || '0.25'
      this.metaHoldoutNote = String(snapshot.metaHoldoutNote ?? this.metaHoldoutNote ?? '—').trim() || '—'
      this.aggregateSummarySavedRuns = String(snapshot.aggregateSummarySavedRuns ?? this.aggregateSummarySavedRuns ?? '0').trim() || '0'
      this.aggregateSummarySavedMeta = String(snapshot.aggregateSummarySavedMeta ?? this.aggregateSummarySavedMeta ?? '0').trim() || '0'
      this.aggregateSummaryLatestGroups = String(snapshot.aggregateSummaryLatestGroups ?? this.aggregateSummaryLatestGroups ?? '—').trim() || '—'
      this.aggregateSummaryLatestArtifacts = String(snapshot.aggregateSummaryLatestArtifacts ?? this.aggregateSummaryLatestArtifacts ?? '—').trim() || '—'
      this.aggregateSummaryLatestHoldout = String(snapshot.aggregateSummaryLatestHoldout ?? this.aggregateSummaryLatestHoldout ?? '—').trim() || '—'
      this.aggregateSummaryLatestRuntime = String(snapshot.aggregateSummaryLatestRuntime ?? this.aggregateSummaryLatestRuntime ?? '—').trim() || '—'
      this.metaMeta = String(snapshot.metaMeta ?? this.metaMeta ?? '—').trim() || '—'
      this.metaOutputPreview = String(snapshot.metaOutputPreview ?? this.metaOutputPreview ?? '-').trim() || '-'
      this.metaTableColumns = rawArray(snapshot.metaTableColumns, this.metaTableColumns)
      this.metaTableRows = rawArray(snapshot.metaTableRows, this.metaTableRows)
      this.metaCompareOptions = rawArray(snapshot.metaCompareOptions, this.metaCompareOptions)
      this.metaCompareSelected = String(snapshot.metaCompareSelected ?? this.metaCompareSelected ?? '').trim()
      this.metaCompareSummary = String(snapshot.metaCompareSummary ?? this.metaCompareSummary ?? '-').trim() || '-'
      this.batchDatasetSearch = String(snapshot.batchDatasetSearch ?? this.batchDatasetSearch ?? '').trim()
      this.batchTags = String(snapshot.batchTags ?? this.batchTags ?? '').trim()
      this.batchHoldoutPct = String(snapshot.batchHoldoutPct ?? this.batchHoldoutPct ?? '15').trim() || '15'
      this.batchHoldoutTag = String(snapshot.batchHoldoutTag ?? this.batchHoldoutTag ?? 'holdout').trim() || 'holdout'
      this.batchAutoMeta = Boolean(snapshot.batchAutoMeta ?? this.batchAutoMeta)
      this.batchSkipDone = Boolean(snapshot.batchSkipDone ?? this.batchSkipDone)
      this.batchDatasets = rawArray(snapshot.batchDatasets, this.batchDatasets)
      this.batchSelectedCount = Math.max(0, Math.floor(Number(snapshot.batchSelectedCount ?? this.batchSelectedCount ?? 0)))
      this.batchTotalCount = Math.max(0, Math.floor(Number(snapshot.batchTotalCount ?? this.batchTotalCount ?? 0)))
      this.batchMeta = String(snapshot.batchMeta ?? this.batchMeta ?? '—').trim() || '—'
      this.batchOutputPreview = String(snapshot.batchOutputPreview ?? this.batchOutputPreview ?? '-').trim() || '-'
      this.canRunBatch = Boolean(snapshot.canRunBatch ?? this.canRunBatch)
      this.canBatchStop = Boolean(snapshot.canBatchStop ?? this.canBatchStop)
      this.canBatchRefresh = Boolean(snapshot.canBatchRefresh ?? this.canBatchRefresh)
      this.canBatchSelectAll = Boolean(snapshot.canBatchSelectAll ?? this.canBatchSelectAll)
      this.canBatchSelectNone = Boolean(snapshot.canBatchSelectNone ?? this.canBatchSelectNone)
      this.canBatchRetryFailed = Boolean(snapshot.canBatchRetryFailed ?? this.canBatchRetryFailed)
      this.canBatchRevealSummary = Boolean(snapshot.canBatchRevealSummary ?? this.canBatchRevealSummary)
      this.metaSavedSearch = String(snapshot.metaSavedSearch ?? this.metaSavedSearch ?? '').trim()
      this.metaSavedOptions = rawArray(snapshot.metaSavedOptions, this.metaSavedOptions)
      this.metaSavedSelected = String(snapshot.metaSavedSelected ?? this.metaSavedSelected ?? '').trim()
      this.metaSavedTagsDraft = String(snapshot.metaSavedTagsDraft ?? this.metaSavedTagsDraft ?? '').trim()
      this.metaSavedMeta = String(snapshot.metaSavedMeta ?? this.metaSavedMeta ?? '—').trim() || '—'
      this.canSaveMetaSavedTags = Boolean(snapshot.canSaveMetaSavedTags ?? this.canSaveMetaSavedTags)
      this.canRefreshMetaSaved = Boolean(snapshot.canRefreshMetaSaved ?? this.canRefreshMetaSaved)
      this.canLoadMetaSaved = Boolean(snapshot.canLoadMetaSaved ?? this.canLoadMetaSaved)
      this.canOpenMetaSavedReport = Boolean(snapshot.canOpenMetaSavedReport ?? this.canOpenMetaSavedReport)
      this.canRevealMetaSaved = Boolean(snapshot.canRevealMetaSaved ?? this.canRevealMetaSaved)
      this.canDeleteMetaSaved = Boolean(snapshot.canDeleteMetaSaved ?? this.canDeleteMetaSaved)
      this.stage4SavedSearch = String(snapshot.stage4SavedSearch ?? this.stage4SavedSearch ?? '').trim()
      this.stage4SavedOptions = rawArray(snapshot.stage4SavedOptions, this.stage4SavedOptions)
      this.stage4SavedSelected = String(snapshot.stage4SavedSelected ?? this.stage4SavedSelected ?? '').trim()
      this.stage4SavedTagsDraft = String(snapshot.stage4SavedTagsDraft ?? this.stage4SavedTagsDraft ?? '').trim()
      this.stage4SavedMeta = String(snapshot.stage4SavedMeta ?? this.stage4SavedMeta ?? '—').trim() || '—'
      this.canSaveStage4SavedTags = Boolean(snapshot.canSaveStage4SavedTags ?? this.canSaveStage4SavedTags)
      this.canRefreshStage4Saved = Boolean(snapshot.canRefreshStage4Saved ?? this.canRefreshStage4Saved)
      this.canLoadStage4Saved = Boolean(snapshot.canLoadStage4Saved ?? this.canLoadStage4Saved)
      this.canRevealStage4Saved = Boolean(snapshot.canRevealStage4Saved ?? this.canRevealStage4Saved)
      this.canDeleteStage4Saved = Boolean(snapshot.canDeleteStage4Saved ?? this.canDeleteStage4Saved)
      this.metaUseMlConfidence = Boolean(snapshot.metaUseMlConfidence ?? this.metaUseMlConfidence)
      this.metaMlAutopilot = Boolean(snapshot.metaMlAutopilot ?? this.metaMlAutopilot)
      this.canMetaMlRefresh = Boolean(snapshot.canMetaMlRefresh ?? this.canMetaMlRefresh)
      this.canMetaMlTrain = Boolean(snapshot.canMetaMlTrain ?? this.canMetaMlTrain)
      this.canMetaMlApply = Boolean(snapshot.canMetaMlApply ?? this.canMetaMlApply)
      this.canMetaMlDelete = Boolean(snapshot.canMetaMlDelete ?? this.canMetaMlDelete)
      this.metaMlStatus = String(snapshot.metaMlStatus ?? this.metaMlStatus ?? '—').trim() || '—'
      this.metaStage4TopK = String(snapshot.metaStage4TopK ?? this.metaStage4TopK ?? '10').trim() || '10'
      this.metaStage4MinConf = String(snapshot.metaStage4MinConf ?? this.metaStage4MinConf ?? '0.55').trim() || '0.55'
      this.metaStage4MetaWeight = String(snapshot.metaStage4MetaWeight ?? this.metaStage4MetaWeight ?? '0.60').trim() || '0.60'
      this.metaStage4MlWeight = String(snapshot.metaStage4MlWeight ?? this.metaStage4MlWeight ?? '0.35').trim() || '0.35'
      this.metaStage4UncertaintyWeight = String(snapshot.metaStage4UncertaintyWeight ?? this.metaStage4UncertaintyWeight ?? '0.05').trim() || '0.05'
      this.metaStage4AcceptConf = String(snapshot.metaStage4AcceptConf ?? this.metaStage4AcceptConf ?? '0.70').trim() || '0.70'
      this.metaStage4ReviewConf = String(snapshot.metaStage4ReviewConf ?? this.metaStage4ReviewConf ?? '0.55').trim() || '0.55'
      this.metaStage4MaxAcceptUncertainty = String(snapshot.metaStage4MaxAcceptUncertainty ?? this.metaStage4MaxAcceptUncertainty ?? '0.45').trim() || '0.45'
      this.metaStage4AutoTrain = Boolean(snapshot.metaStage4AutoTrain ?? this.metaStage4AutoTrain)
      this.metaStage4RequireHealthy = Boolean(snapshot.metaStage4RequireHealthy ?? this.metaStage4RequireHealthy)
      this.metaStage4FallbackToMeta = Boolean(snapshot.metaStage4FallbackToMeta ?? this.metaStage4FallbackToMeta)
      this.canRunMetaStage4 = Boolean(snapshot.canRunMetaStage4 ?? this.canRunMetaStage4)
      this.canRevealMetaStage4 = Boolean(snapshot.canRevealMetaStage4 ?? this.canRevealMetaStage4)
      this.metaStage4Status = String(snapshot.metaStage4Status ?? this.metaStage4Status ?? '—').trim() || '—'
      this.metaStage4OutputPreview = String(snapshot.metaStage4OutputPreview ?? this.metaStage4OutputPreview ?? '-').trim() || '-'
      this.canRunMeta = Boolean(snapshot.canRunMeta ?? this.canRunMeta)
      this.canOpenMetaReport = Boolean(snapshot.canOpenMetaReport ?? this.canOpenMetaReport)
      this.canRevealMetaFiles = Boolean(snapshot.canRevealMetaFiles ?? this.canRevealMetaFiles)
      this.debugOutputPreview = String(snapshot.debugOutputPreview ?? this.debugOutputPreview ?? '{}').trim() || '{}'
      this.topRows = Math.max(0, Math.floor(Number(snapshot.topRows ?? this.topRows ?? 0)))
      this.topFilterMeta = String(snapshot.topFilterMeta ?? this.topFilterMeta ?? '-').trim() || '-'
      this.topSelectionGuardrail = String(snapshot.topSelectionGuardrail ?? this.topSelectionGuardrail ?? '—').trim() || '—'
      this.paretoMeta = String(snapshot.paretoMeta ?? this.paretoMeta ?? '-').trim() || '-'
      this.drillMeta = String(snapshot.drillMeta ?? this.drillMeta ?? '-').trim() || '-'
      this.drillStatus = String(snapshot.drillStatus ?? this.drillStatus ?? 'Click a Top row to view curves.').trim() || 'Click a Top row to view curves.'
      this.stabilityStatus = String(snapshot.stabilityStatus ?? this.stabilityStatus ?? '-').trim() || '-'
      this.stabilityMeta = String(snapshot.stabilityMeta ?? this.stabilityMeta ?? '-').trim() || '-'
      this.heatmapMeta = String(snapshot.heatmapMeta ?? this.heatmapMeta ?? '-').trim() || '-'
      this.heatmapStatusLine = String(snapshot.heatmapStatusLine ?? this.heatmapStatusLine ?? 'Heatmap unavailable.').trim() || 'Heatmap unavailable.'
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      const isTopFilterKey = key === 'filterMinPnl'
        || key === 'filterMinSharpe'
        || key === 'filterMaxDd'
        || key === 'filterMinTrades'
        || key === 'filterMinExposure'
      if (applyVueAnalysisFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
        if (isTopFilterKey) {
          analysisService.refreshTopTableFromWorkflowRun()
        }
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'compareSelect') {
        useHistoryStore().setSelectField(key, next)
        this.compareSummary = next.trim() ? `Baseline selected: ${next.trim()}` : '-'
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
        void analysisService.refreshCompareSummary()
        return
      }
      if (applyVueAnalysisFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
        if (key === 'paretoX' || key === 'paretoY') {
          analysisService.refreshParetoFromWorkflowRun()
        }
        if (key === 'heatmapColor' || key === 'heatmapRange' || key === 'heatmapShowTop' || key === 'heatmapShowCrosshair') {
          analysisService.refreshHeatmapFromWorkflowRun()
        }
      }
    },

    clearTopFilters(): void {
      void analysisService.clearTopFilters()
    },

    sortTopTable(key: string): void {
      void analysisService.sortTopTable(String(key || '').trim())
    },

    selectTopRow(index: number): void {
      void analysisService.selectTopRow(Math.max(0, Math.floor(Number(index || 0))))
    },

    copyBest(): void {
      void analysisService.copyBest()
    },

    exportTopCsv(): void {
      void analysisService.exportTopCsv()
    },

    exportTopJson(): void {
      void analysisService.exportTopJson()
    },

    setBaseline(): void {
      void analysisService.setBaseline()
    },

    clearBaseline(): void {
      void analysisService.clearBaseline()
    },

    heatmapExportPng(): void {
      void analysisService.heatmapExportPng()
    },

    overviewEmptyPrimary(): void {
      runOverviewEmptyAction(this.overviewPrimaryAction)
    },

    overviewEmptySecondary(): void {
      runOverviewEmptyAction(this.overviewSecondaryAction)
    },

    rerenderDrill(): void {
      void analysisService.rerenderDrill()
    },

    drillClearTradeSelection(): void {
      void analysisService.drillClearTradeSelection()
    },

    drillPrevTrade(): void {
      void analysisService.drillPrevTrade()
    },

    drillNextTrade(): void {
      void analysisService.drillNextTrade()
    },

    setMetaInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueMetaTextFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
        if (key === 'batchDatasetSearch') {
          metaService.refreshBatchDatasetView()
        }
      }
    },

    setMetaSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueMetaTextFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    setMetaCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (applyVueMetaCheckboxFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    runBatch(): void {
      void metaService.runBatch()
    },

    batchStop(): void {
      void metaService.batchStop()
    },

    batchRefresh(): void {
      void metaService.batchRefresh()
    },

    batchSelectAll(): void {
      void metaService.batchSelectAll()
    },

    batchSelectNone(): void {
      void metaService.batchSelectNone()
    },

    batchRetryFailed(): void {
      void metaService.batchRetryFailed()
    },

    batchRevealSummary(): void {
      void metaService.batchRevealSummary()
    },

    saveSavedTags(): void {
      void metaService.saveSavedTags()
    },

    refreshSaved(): void {
      void metaService.refreshSaved()
    },

    loadSaved(): void {
      void metaService.loadSaved()
    },

    openSavedReport(): void {
      void metaService.openSavedReport()
    },

    revealSaved(): void {
      void metaService.revealSaved()
    },

    deleteSaved(): void {
      void metaService.deleteSaved()
    },

    saveStage4SavedTags(): void {
      void metaService.saveStage4SavedTags()
    },

    refreshStage4Saved(): void {
      void metaService.refreshStage4Saved()
    },

    loadStage4Saved(): void {
      void metaService.loadStage4Saved()
    },

    revealStage4Saved(): void {
      void metaService.revealStage4Saved()
    },

    deleteStage4Saved(): void {
      void metaService.deleteStage4Saved()
    },

    mlRefresh(): void {
      void metaService.mlRefresh()
    },

    mlTrain(): void {
      void metaService.mlTrain()
    },

    mlApply(): void {
      void metaService.mlApply()
    },

    mlDelete(): void {
      void metaService.mlDelete()
    },

    runStage4(): void {
      void metaService.runStage4()
    },

    revealStage4(): void {
      void metaService.revealStage4()
    },

    runMeta(): void {
      void metaService.runMeta()
    },

    openMetaReport(): void {
      void metaService.openMetaReport()
    },

    revealMetaFiles(): void {
      void metaService.revealMetaFiles()
    },

    toggleBatchDataset(path: string, checked: boolean): void {
      const normalizedPath = String(path || '').trim()
      if (!normalizedPath) return
      const nextChecked = !!checked
      const rows = this.batchDatasets
      const targetIndex = rows.findIndex((entry) => String(entry.path || '').trim() === normalizedPath)
      if (targetIndex >= 0) {
        const current = rows[targetIndex]
        if (current && current.checked !== nextChecked) {
          const nextRows = rows.slice()
          nextRows[targetIndex] = {
            ...current,
            checked: nextChecked,
          }
          this.batchDatasets = markRaw(nextRows)
          let nextSelectedCount = Number(this.batchSelectedCount || 0) + (nextChecked ? 1 : -1)
          if (!Number.isFinite(nextSelectedCount) || nextSelectedCount < 0 || nextSelectedCount > nextRows.length) {
            nextSelectedCount = nextRows.reduce((count, entry) => count + (entry.checked ? 1 : 0), 0)
          }
          this.batchSelectedCount = nextSelectedCount
          this.batchTotalCount = nextRows.length
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        }
      }

      void metaService.toggleBatchDataset(normalizedPath, nextChecked)
    },
  },
})


