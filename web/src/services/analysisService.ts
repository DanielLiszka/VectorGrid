import { getActivePinia } from 'pinia'
import { markRaw } from 'vue'
import { useLightweightChart } from '../composables/useLightweightChart'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { readLatestWorkflowRunSnapshot } from '../stores/workflowArtifactsStore'
import { invokeCommand } from '../tauri/invoke'

type TopColumn = {
  key?: unknown
  label?: unknown
  sort?: unknown
  alignLeft?: unknown
}
type TopRowCell = { text?: unknown; className?: unknown; title?: unknown }
type TopRow = { selected?: unknown; cells?: unknown }
type SortDir = 'asc' | 'desc'
type TopFilterValues = {
  minPnl?: number
  minSharpe?: number
  maxDd?: number
  minTrades?: number
  minExposure?: number
}

interface AnalysisStoreState {
  bestKicker?: unknown
  bestTitle?: unknown
  bestMeta?: unknown
  paretoX?: unknown
  paretoY?: unknown
  paretoXOptions?: unknown
  paretoYOptions?: unknown
  paretoWrapVisible?: unknown
  paretoMeta?: unknown
  topColumns?: unknown
  topTableRows?: unknown
  topRows?: unknown
  topFilterMeta?: unknown
  drillMeta?: unknown
  drillStatus?: unknown
  stabilityStatus?: unknown
  stabilityMeta?: unknown
  heatmapWrapVisible?: unknown
  heatmapColor?: unknown
  heatmapColorOptions?: unknown
  heatmapRange?: unknown
  heatmapRangeOptions?: unknown
  heatmapShowTop?: unknown
  heatmapShowTopOptions?: unknown
  heatmapShowCrosshair?: unknown
  heatmapShowCrosshairOptions?: unknown
  canHeatmapExportPng?: unknown
  heatmapMeta?: unknown
  heatmapStatusLine?: unknown
  filterMinPnl?: unknown
  filterMinSharpe?: unknown
  filterMaxDd?: unknown
  filterMinTrades?: unknown
  filterMinExposure?: unknown
  canClearTopFilters?: unknown
  topSelectionGuardrail?: unknown
  compareSummary?: unknown
  bestSummary?: unknown
  bestStats?: unknown
  statusLine?: unknown
  source?: unknown
  updatedAtUnixMs?: unknown
}

interface HistoryStoreState {
  compareSelectedValue?: unknown
  savedRunsRaw?: unknown
  source?: unknown
  updatedAtUnixMs?: unknown
}

interface WorkflowRunState {
  ts?: unknown
  artifactPath?: unknown
  artifact_path?: unknown
  req?: unknown
  res?: unknown
}

interface WorkflowValidationState {
  res?: unknown
}

interface TopRowModel {
  rank: number
  params: Record<string, unknown>
  metrics: Record<string, unknown>
  mo?: { maximize_value?: number; minimize_value?: number; violation?: number }
}

interface HeatmapModel {
  binsFast: number
  binsSlow: number
  fastMin: number
  fastMax: number
  slowMin: number
  slowMax: number
  values: Array<number | null>
}

export interface AnalysisExecutionTopRowSnapshot {
  params: Record<string, unknown>
  metrics: Record<string, unknown>
  source: string
}

export interface AnalysisExecutionStrategySnapshot {
  strategyKind: string
  strategy: {
    kind: 'double_ma' | 'composite_cpu'
    req: Record<string, unknown>
  }
}

interface RunArtifactPayload {
  ts?: unknown
  run?: unknown
  validation?: unknown
}

interface BaselineRun {
  source: 'disk' | 'hist'
  ts: string
  req: Record<string, unknown>
  res: Record<string, unknown>
}

interface DrillPriceBar {
  time: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  fromIndex: number
  toIndex: number
}

interface DrillInvocation {
  runReq: Record<string, unknown>
  kind: string
  command: 'compute_double_ma_drilldown' | 'compute_composite_cpu_drilldown'
  request: Record<string, unknown>
}

interface StabilityInvocation {
  command: 'compute_double_ma_local_stability' | 'compute_composite_cpu_local_stability'
  request: Record<string, unknown>
}

interface DrillTvChartContext {
  chart: any
  candle: any
  fast: any
  slow: any
  halftrend: any
  macz: any
}

interface DrillTvRenderPayload {
  seq: number
  ctx: DrillTvChartContext
  candleData: Array<{
    time: number
    open: number
    high: number
    low: number
    close: number
  }>
  fastData: Array<{ time: number; value: number }>
  slowData: Array<{ time: number; value: number }>
  halftrendData: Array<{ time: number; value: number }>
  maczData: Array<{ time: number; value: number; color?: string }>
  trades: number
}

let compareSeq = 0
let drillSeq = 0
let drillSelectedTradeIdx = -1
let drillTradesRangeStart = 0
let topRowsModelCache: TopRowModel[] = []
const baselineArtifactCache = new Map<string, BaselineRun>()
const drillCurvesCache = new Map<string, unknown>()
const stabilityCache = new Map<string, unknown>()
const drillPriceBarsCache = new Map<string, DrillPriceBar[]>()
let drillTvModulePromise: Promise<any> | null = null
const drillTvRuntime = useLightweightChart<any>()
let drillTvCandleSeries: any = null
let drillTvFastSeries: any = null
let drillTvSlowSeries: any = null
let drillTvHalftrendSeries: any = null
let drillTvMaczSeries: any = null
let drillTvRenderFrame = 0
let drillTvPendingPayload: DrillTvRenderPayload | null = null
let heatmapHoverCell: { col: number; row: number } | null = null

function text(v: unknown): string {
  return String(v ?? '').trim()
}

function num(v: unknown, fallback = NaN): number {
  const n = Number(v)
  return Number.isFinite(n) ? n : fallback
}

function asRecord(v: unknown): Record<string, unknown> {
  return v && typeof v === 'object' ? (v as Record<string, unknown>) : {}
}

function markRawEntity<T>(value: T): T {
  if (value && typeof value === 'object') {
    return markRaw(value as object) as T
  }
  return value
}

function asNumberMap(v: unknown): Record<string, number> | null {
  if (!v || typeof v !== 'object') return null
  const src = v as Record<string, unknown>
  const out: Record<string, number> = {}
  for (const key of Object.keys(src)) {
    const next = Number(src[key])
    if (!Number.isFinite(next)) continue
    out[key] = next
  }
  return Object.keys(out).length ? out : null
}

function fmt(v: unknown, digits = 6): string {
  const n = Number(v)
  if (!Number.isFinite(n)) return '-'
  return n.toFixed(Math.max(0, Math.floor(digits)))
}

function fmtPct(v: unknown, digits = 2): string {
  const n = Number(v)
  if (!Number.isFinite(n)) return '-'
  return `${(n * 100).toFixed(Math.max(0, Math.floor(digits)))}%`
}

function trimFixed(src: string): string {
  return src.replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

function fmtDisplay(v: unknown, digits = 4): string {
  const n = Number(v)
  if (!Number.isFinite(n)) return '-'
  const abs = Math.abs(n)
  if (abs === 0) return '0'
  const precision = Math.max(0, Math.floor(digits))
  const zeroThreshold = 1 / (10 ** Math.max(1, precision))
  if (abs < zeroThreshold) return '0'
  if (abs >= 1_000_000) {
    return n.toLocaleString(undefined, {
      maximumFractionDigits: Math.min(2, precision),
      minimumFractionDigits: 0,
    })
  }
  return trimFixed(n.toFixed(precision))
}

function fmtPctDisplay(v: unknown, digits = 2): string {
  const n = Number(v)
  if (!Number.isFinite(n)) return '-'
  return `${fmtDisplay(n * 100, digits)}%`
}

function requestedTopK(req: Record<string, unknown>): number {
  return Math.max(0, Math.floor(num(req.top_k ?? req.topK, 0)))
}

function requestedHeatmapBins(req: Record<string, unknown>): number {
  return Math.max(0, Math.floor(num(req.heatmap_bins ?? req.heatmapBins, 0)))
}

function objectiveScoreKey(objective: string): string {
  const obj = text(objective || 'Sharpe')
  if (obj === 'Pnl') return 'pnl'
  if (obj === 'Sharpe') return 'sharpe'
  if (obj === 'Sortino') return 'sortino'
  if (obj === 'Calmar') return 'calmar'
  if (obj === 'Omega') return 'omega'
  if (obj === 'Volatility') return 'volatility'
  if (obj === 'MaxDrawdown') return 'max_dd'
  return 'sharpe'
}

function formatDrillMetricsSummary(metricsRaw: Record<string, unknown>): string {
  const metrics = asRecord(metricsRaw)
  if (!Object.keys(metrics).length) return ''
  return `pnl=${fmt(metrics.pnl, 6)} ret=${fmtPct(metrics.pnl, 2)} sharpe=${fmt(metrics.sharpe, 6)} dd=${fmt(metrics.max_dd, 6)} trades=${num(metrics.trades, 0)} expo=${fmt(metrics.exposure, 4)}`
}

function formatDrillMoSummary(moRaw: Record<string, unknown> | null | undefined): string {
  const mo = asRecord(moRaw)
  if (!Object.keys(mo).length) return ''
  return `max=${fmt(mo.maximize_value, 6)} min=${fmt(mo.minimize_value, 6)} viol=${fmt(mo.violation, 6)}`
}

function parseIndex(v: unknown): number {
  const n = Number(v)
  if (!Number.isFinite(n)) return 0
  return Math.max(0, Math.floor(n))
}

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  const kind = text(kindRaw || 'DoubleMa') || 'DoubleMa'
  if (kind === 'Pit') return 'Pit'
  if (kind === 'Set') return 'Set'
  if (kind === 'Mts') return 'Mts'
  if (kind === 'Marrt') return 'Marrt'
  if (kind === 'Vfct') return 'Vfct'
  if (kind === 'Rocb') return 'Rocb'
  if (kind === 'Kqsr') return 'Kqsr'
  if (kind === 'Carm') return 'Carm'
  if (kind === 'Kvdf') return 'Kvdf'
  if (kind === 'Mkcs') return 'Mkcs'
  if (kind === 'Cdmc') return 'Cdmc'
  if (kind === 'Asr') return 'Asr'
  if (kind === 'Vcc') return 'Vcc'
  if (kind === 'Vemr') return 'Vemr'
  if (kind === 'Vcet') return 'Vcet'
  if (kind === 'CompositeCpu') return 'CompositeCpu'
  if (kind === 'CompositeMo') return 'CompositeMo'
  if (kind === 'CompositeTrend') return 'CompositeTrend'
  if (kind === 'RegimeHybrid') return 'RegimeHybrid'
  if (kind === 'BreakoutMomentum') return 'BreakoutMomentum'
  return 'DoubleMa'
}

function isDoubleLikeKind(kindRaw: unknown): boolean {
  const kind = normalizeStrategyKindRaw(kindRaw)
  return kind === 'DoubleMa' || kind === 'Pit' || kind === 'Set' || kind === 'Mts' || kind === 'Orc' || kind === 'Marrt' || kind === 'Vfct' || kind === 'CompositeTrend' || kind === 'RegimeHybrid' || kind === 'BreakoutMomentum'
}

function markAnalysisVue(store: AnalysisStoreState): void {
  store.source = 'vue'
  store.updatedAtUnixMs = Date.now()
}

function markHistoryVue(store: HistoryStoreState): void {
  store.source = 'vue'
  store.updatedAtUnixMs = Date.now()
}

function readAnalysisStore(): AnalysisStoreState | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  const root = pinia.state.value as Record<string, unknown>
  const state = root.analysis
  if (!state || typeof state !== 'object') return null
  return state as AnalysisStoreState
}

function readHistoryStore(): HistoryStoreState | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  const root = pinia.state.value as Record<string, unknown>
  const state = root.history
  if (!state || typeof state !== 'object') return null
  return state as HistoryStoreState
}

function readWorkflowRun(): WorkflowRunState | null {
  const pinia = getActivePinia()
  if (pinia) {
    const root = pinia.state.value as Record<string, unknown>
    const wf = asRecord(root.workflowArtifacts)
    const run = wf.currentRun
    if (run && typeof run === 'object') return run as WorkflowRunState
  }
  const snapshot = readLatestWorkflowRunSnapshot()
  if (!snapshot) return null
  return { req: snapshot.req ?? null, res: snapshot.res ?? null }
}

function hasValidationResult(): boolean {
  const pinia = getActivePinia()
  if (!pinia) return false
  const root = pinia.state.value as Record<string, unknown>
  const wf = asRecord(root.workflowArtifacts)
  const val = wf.currentValidation
  if (!val || typeof val !== 'object') return false
  return !!(val as WorkflowValidationState).res
}

function readTopColumns(store: AnalysisStoreState): TopColumn[] {
  return Array.isArray(store.topColumns) ? (store.topColumns as TopColumn[]) : []
}

function readTopRows(store: AnalysisStoreState): TopRow[] {
  return Array.isArray(store.topTableRows) ? (store.topTableRows as TopRow[]) : []
}

function readSortState(store: AnalysisStoreState): {
  key: string
  dir: SortDir
} {
  const columns = readTopColumns(store)
  for (const col of columns) {
    const key = text(col?.key)
    const dir = text(col?.sort).toLowerCase()
    if (!key) continue
    if (dir === 'asc' || dir === 'desc') return { key, dir }
  }
  return { key: 'rank', dir: 'asc' }
}

const ASC_KEYS = new Set(['rank', 'minimize_value', 'violation'])
function defaultSortDir(key: string): SortDir {
  return ASC_KEYS.has(text(key)) ? 'asc' : 'desc'
}

function scoreOf(metrics: Record<string, unknown>, objective: string): number {
  const obj = String(objective || 'Sharpe')
  if (obj === 'Pnl') return num(metrics.pnl, 0)
  if (obj === 'Sharpe') return num(metrics.sharpe, 0)
  if (obj === 'Sortino') return num(metrics.sortino, 0)
  if (obj === 'Calmar') return num(metrics.calmar, 0)
  if (obj === 'Omega') return num(metrics.omega, 0)
  if (obj === 'Volatility') return -num(metrics.volatility, 0)
  if (obj === 'MaxDrawdown') return -num(metrics.max_dd, 0)
  return num(metrics.sharpe, 0)
}

function objectiveOf(req: Record<string, unknown>): string {
  const direct = text(req.objective)
  if (direct) return direct
  const objectives = asRecord(req.objectives)
  return text(objectives.maximize) || 'Sharpe'
}

function isMo(req: Record<string, unknown>, res: Record<string, unknown>): boolean {
  const kind = text(req.strategy_kind ?? req.strategyKind ?? 'DoubleMa')
  return kind === 'CompositeMo' && Array.isArray(res.pareto)
}

function resolveRunMaParams(runReq: Record<string, unknown>, side: 'fast' | 'slow', maId: string): Record<string, number> | null {
  const direct = side === 'fast' ? asNumberMap(runReq.fast_ma_params ?? runReq.fastMaParams) : asNumberMap(runReq.slow_ma_params ?? runReq.slowMaParams)
  if (direct) return direct
  const id = text(maId).toLowerCase()
  if (id !== 'alma') return null
  const hasLegacy = runReq.alma_offset != null || runReq.almaOffset != null || runReq.alma_sigma != null || runReq.almaSigma != null
  if (!hasLegacy) return null
  return {
    offset: num(runReq.alma_offset ?? runReq.almaOffset, 0.85),
    sigma: num(runReq.alma_sigma ?? runReq.almaSigma, 6.0)
  }
}

function desiredDrillBins(): number {
  if (typeof document === 'undefined') return 1024
  const canvas = byIds<HTMLCanvasElement>(['equityCanvasVue', 'equityCanvas'])
  if (!canvas) return 1024
  const rect = canvas.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  const w = Math.floor((rect.width || 0) * dpr)
  if (w < 64) return 1024
  return Math.max(64, Math.min(4096, w))
}

function requestCacheKey(command: string, req: Record<string, unknown>): string {
  try {
    return `${command}:${stableStringify(req)}`
  } catch {
    return `${command}:${Date.now()}`
  }
}

function collectTopRows(res: Record<string, unknown>, options: { allowBestFallback?: boolean } = {}): TopRowModel[] {
  const out: TopRowModel[] = []
  const allowBestFallback = options.allowBestFallback !== false
  const pareto = Array.isArray(res.pareto) ? res.pareto : null
  if (pareto) {
    const pts = pareto.map((entry) => {
      const row = asRecord(entry)
      return {
        params: asRecord(row.params),
        metrics: asRecord(row.metrics),
        maximize_value: num(row.maximize_value, Number.NEGATIVE_INFINITY),
        minimize_value: num(row.minimize_value, Number.POSITIVE_INFINITY),
        violation: num(row.violation, 0)
      }
    })
    pts.sort((a, b) => {
      const af = a.violation <= 0
      const bf = b.violation <= 0
      if (af !== bf) return af ? -1 : 1
      if (a.maximize_value !== b.maximize_value) return b.maximize_value - a.maximize_value
      if (a.minimize_value !== b.minimize_value) return a.minimize_value - b.minimize_value
      return 0
    })
    for (let i = 0; i < pts.length; i += 1) {
      const p = pts[i]
      if (!p) continue
      out.push({
        rank: i + 1,
        params: p.params,
        metrics: p.metrics,
        mo: {
          maximize_value: p.maximize_value,
          minimize_value: p.minimize_value,
          violation: p.violation
        }
      })
    }
    return out
  }

  const top = Array.isArray(res.top) ? res.top : []
  for (let i = 0; i < top.length; i += 1) {
    const row = Array.isArray(top[i]) ? (top[i] as unknown[]) : []
    out.push({
      rank: i + 1,
      params: asRecord(row[0]),
      metrics: asRecord(row[1])
    })
  }
  if (!out.length && allowBestFallback) {
    const p = asRecord(res.best_params)
    const m = asRecord(res.best_metrics)
    if (Object.keys(p).length && Object.keys(m).length) out.push({ rank: 1, params: p, metrics: m })
  }
  return out
}

function readFilters(store: AnalysisStoreState): TopFilterValues {
  const out: TopFilterValues = {}
  const parse = (value: unknown): number | null => {
    const raw = text(value)
    if (!raw) return null
    const n = Number(raw)
    return Number.isFinite(n) ? n : null
  }
  const minPnl = parse(store.filterMinPnl)
  const minSharpe = parse(store.filterMinSharpe)
  const maxDd = parse(store.filterMaxDd)
  const minTrades = parse(store.filterMinTrades)
  const minExposure = parse(store.filterMinExposure)
  if (minPnl != null) out.minPnl = minPnl
  if (minSharpe != null) out.minSharpe = minSharpe
  if (maxDd != null) out.maxDd = maxDd
  if (minTrades != null) out.minTrades = minTrades
  if (minExposure != null) out.minExposure = minExposure
  return out
}

function applyTopFilters(rows: TopRowModel[], filters: TopFilterValues): TopRowModel[] {
  return rows.filter((row) => {
    const m = row.metrics || {}
    const pnl = num(m.pnl, 0)
    const sharpe = num(m.sharpe, 0)
    const dd = num(m.max_dd, 0)
    const trades = num(m.trades, 0)
    const expo = num(m.exposure, 0)
    if (filters.minPnl != null && pnl < filters.minPnl) return false
    if (filters.minSharpe != null && sharpe < filters.minSharpe) return false
    if (filters.maxDd != null && dd > filters.maxDd) return false
    if (filters.minTrades != null && trades < filters.minTrades) return false
    if (filters.minExposure != null && expo < filters.minExposure) return false
    return true
  })
}

function sortRows(rows: TopRowModel[], key: string, dir: SortDir, objective: string): TopRowModel[] {
  const d = dir === 'desc' ? -1 : 1
  const cmp = (a: number | string, b: number | string): number => (a === b ? 0 : a < b ? -1 : 1)
  const k = text(key || 'rank')
  return [...rows].sort((ra, rb) => {
    const pa = ra.params || {}
    const pb = rb.params || {}
    const ma = ra.metrics || {}
    const mb = rb.metrics || {}
    const moa = ra.mo || {}
    const mob = rb.mo || {}
    if (k === 'rank') return d * cmp(ra.rank, rb.rank)
    if (k === 'fast') {
      const aType = String(pa.fast_ma_type || '').toLowerCase()
      const bType = String(pb.fast_ma_type || '').toLowerCase()
      if (aType !== bType) return d * cmp(aType, bType)
      return d * cmp(num(pa.fast_len, 0), num(pb.fast_len, 0))
    }
    if (k === 'slow') {
      const aType = String(pa.slow_ma_type || '').toLowerCase()
      const bType = String(pb.slow_ma_type || '').toLowerCase()
      if (aType !== bType) return d * cmp(aType, bType)
      return d * cmp(num(pa.slow_len, 0), num(pb.slow_len, 0))
    }
    if (k === 'pnl' || k === 'ret_pct') return d * cmp(num(ma.pnl, 0), num(mb.pnl, 0))
    if (k === 'sharpe') return d * cmp(num(ma.sharpe, 0), num(mb.sharpe, 0))
    if (k === 'max_dd') return d * cmp(num(ma.max_dd, 0), num(mb.max_dd, 0))
    if (k === 'trades') return d * cmp(num(ma.trades, 0), num(mb.trades, 0))
    if (k === 'exposure') return d * cmp(num(ma.exposure, 0), num(mb.exposure, 0))
    if (k === 'maximize_value') return d * cmp(num(moa.maximize_value, Number.NEGATIVE_INFINITY), num(mob.maximize_value, Number.NEGATIVE_INFINITY))
    if (k === 'minimize_value') return d * cmp(num(moa.minimize_value, Number.POSITIVE_INFINITY), num(mob.minimize_value, Number.POSITIVE_INFINITY))
    if (k === 'violation') return d * cmp(num(moa.violation, 0), num(mob.violation, 0))
    if (k === 'score') return d * cmp(scoreOf(ma, objective), scoreOf(mb, objective))
    return d * cmp(ra.rank, rb.rank)
  })
}

function topColumnsFor(req: Record<string, unknown>, res: Record<string, unknown>, sort: { key: string; dir: SortDir }): TopColumn[] {
  const objectives = asRecord(req.objectives)
  const columns: TopColumn[] = isMo(req, res)
    ? [
        { key: 'rank', label: '#', sort: '', alignLeft: true },
        { key: 'fast', label: 'Fast', sort: '', alignLeft: true },
        { key: 'slow', label: 'Slow', sort: '', alignLeft: true },
        { key: 'pnl', label: 'PnL', sort: '', alignLeft: false },
        { key: 'ret_pct', label: 'Ret %', sort: '', alignLeft: false },
        { key: 'sharpe', label: 'Sharpe', sort: '', alignLeft: false },
        { key: 'max_dd', label: 'MaxDD', sort: '', alignLeft: false },
        { key: 'trades', label: 'Trades', sort: '', alignLeft: false },
        { key: 'exposure', label: 'Expo', sort: '', alignLeft: false },
        {
          key: 'maximize_value',
          label: `Max(${String(objectives.maximize ?? req.objective ?? 'maximize')})`,
          sort: '',
          alignLeft: false
        },
        {
          key: 'minimize_value',
          label: `Min(${String(objectives.minimize ?? '?')})`,
          sort: '',
          alignLeft: false
        },
        { key: 'violation', label: 'Viol', sort: '', alignLeft: false }
      ]
    : [
        { key: 'rank', label: '#', sort: '', alignLeft: true },
        { key: 'fast', label: 'Fast', sort: '', alignLeft: true },
        { key: 'slow', label: 'Slow', sort: '', alignLeft: true },
        { key: 'pnl', label: 'PnL', sort: '', alignLeft: false },
        { key: 'ret_pct', label: 'Ret %', sort: '', alignLeft: false },
        { key: 'sharpe', label: 'Sharpe', sort: '', alignLeft: false },
        { key: 'max_dd', label: 'MaxDD', sort: '', alignLeft: false },
        { key: 'trades', label: 'Trades', sort: '', alignLeft: false },
        { key: 'exposure', label: 'Expo', sort: '', alignLeft: false },
        { key: 'score', label: 'Score', sort: '', alignLeft: false }
      ]
  for (const col of columns) if (text(col.key) === sort.key) col.sort = sort.dir
  return columns
}

function cacheSet(cache: Map<string, unknown>, key: string, value: unknown, max = 96): void {
  if (!key) return
  if (cache.has(key)) cache.delete(key)
  cache.set(key, value)
  while (cache.size > Math.max(8, Math.floor(Number(max || 96)))) {
    const oldest = cache.keys().next()
    if (oldest.done) break
    cache.delete(oldest.value)
  }
}

function buildTopRowModelsForCurrentFilters(store: AnalysisStoreState, req: Record<string, unknown>, res: Record<string, unknown>): TopRowModel[] {
  const objective = objectiveOf(req)
  const sort = readSortState(store)
  const collected = collectTopRows(res, { allowBestFallback: requestedTopK(req) !== 0 })
  const sorted = sortRows(collected, sort.key, sort.dir, objective)
  const filters = readFilters(store)
  return applyTopFilters(sorted, filters)
}

function resolveTopRowModelAtIndex(store: AnalysisStoreState, index: number): TopRowModel | null {
  if (topRowsModelCache.length === readTopRows(store).length) {
    return topRowsModelCache[index] ?? null
  }
  const run = readWorkflowRun()
  const req = asRecord(run?.req)
  const res = asRecord(run?.res)
  if (!Object.keys(req).length || !Object.keys(res).length) return null
  const rebuilt = buildTopRowModelsForCurrentFilters(store, req, res)
  topRowsModelCache = rebuilt
  return rebuilt[index] ?? null
}

function nonEmptyRecord(value: unknown): Record<string, unknown> | null {
  const row = asRecord(value)
  return Object.keys(row).length ? row : null
}

export function resolveExecutionTopRowSnapshot(preferredSourceRaw: unknown): AnalysisExecutionTopRowSnapshot {
  const preferredSource = text(preferredSourceRaw).toLowerCase() === 'best' ? 'best' : 'selected'
  const run = readWorkflowRun()
  const runReq = nonEmptyRecord(run?.req)
  const runRes = nonEmptyRecord(run?.res)
  if (!runReq || !runRes) {
    throw new Error('No optimization run loaded. Use Manual parameter source or run an optimization.')
  }

  if (preferredSource === 'best') {
    const bestParams = nonEmptyRecord(runRes.best_params)
    if (bestParams) {
      return {
        params: bestParams,
        metrics: asRecord(runRes.best_metrics),
        source: 'best'
      }
    }

    const store = readAnalysisStore()
    if (store) {
      const filtered = buildTopRowModelsForCurrentFilters(store, runReq, runRes)
      const top = filtered[0] ?? null
      if (top) {
        return {
          params: asRecord(top.params),
          metrics: asRecord(top.metrics),
          source: 'best_top_fallback'
        }
      }
    }

    const collected = collectTopRows(runRes)
    const fallback = collected[0] ?? null
    if (fallback) {
      return {
        params: asRecord(fallback.params),
        metrics: asRecord(fallback.metrics),
        source: 'best_top_fallback'
      }
    }
    throw new Error('No Top rows available. Run an optimization first.')
  }

  const store = readAnalysisStore()
  if (!store) {
    throw new Error('No Top row selected. Click a row in the Top tab or switch Parameter source to Best row.')
  }
  const rows = readTopRows(store)
  const selectedIndex = rows.findIndex((row) => !!row?.selected)
  if (selectedIndex < 0) {
    throw new Error('No Top row selected. Click a row in the Top tab or switch Parameter source to Best row.')
  }
  const selected = resolveTopRowModelAtIndex(store, selectedIndex)
  if (!selected) {
    throw new Error('Selected Top row is unavailable (filters changed). Select a row again or use Best row.')
  }
  return {
    params: asRecord(selected.params),
    metrics: asRecord(selected.metrics),
    source: 'selected'
  }
}

function buildDrilldownInvocation(
  paramsRaw: Record<string, unknown>,
  runReqRaw: Record<string, unknown>,
  runResRaw?: Record<string, unknown> | null,
): DrillInvocation | null {
  const runReq = asRecord(runReqRaw)
  const runRes = asRecord(runResRaw)
  const params = asRecord(paramsRaw)
  const dataId = text(runReq.data_id ?? runReq.dataId)
  const fastMaType = text(params.fast_ma_type).toLowerCase()
  const slowMaType = text(params.slow_ma_type).toLowerCase()
  const fastLen = Math.max(0, Math.floor(num(params.fast_len, 0)))
  const slowLen = Math.max(0, Math.floor(num(params.slow_len, 0)))
  if (!dataId || !fastMaType || !slowMaType || fastLen <= 0 || slowLen <= 0) return null

  const kind = normalizeStrategyKindRaw(runReq.strategy_kind ?? runReq.strategyKind)
  const fastParams = asNumberMap(params.fast_ma_params ?? params.fastMaParams) ?? resolveRunMaParams(runReq, 'fast', fastMaType)
  const slowParams = asNumberMap(params.slow_ma_params ?? params.slowMaParams) ?? resolveRunMaParams(runReq, 'slow', slowMaType)
  const followupCacheKey = text(runRes.followup_cache_key ?? runRes.followupCacheKey) || null

  const base: Record<string, unknown> = {
    data_id: dataId,
    backend_hint: runReq.backend || { backend: 'CpuOnly' },
    params: {
      fast_len: fastLen,
      slow_len: slowLen,
      fast_ma_type: text(params.fast_ma_type),
      slow_ma_type: text(params.slow_ma_type)
    },
    ma_source: text(runReq.ma_source) || 'close',
    fast_ma_params: fastParams,
    slow_ma_params: slowParams,
    strategy: runReq.strategy || runReq.base_strategy || {},
    followup_cache_key: followupCacheKey,
    bins: desiredDrillBins()
  }

  if (isDoubleLikeKind(kind)) {
    return {
      runReq,
      kind,
      command: 'compute_double_ma_drilldown',
      request: {
        ...base,
        backend_hint: runReq.backend || { backend: 'CpuOnly' },
        halftrend_filter: runReq.halftrend_filter || runReq.halftrendFilter || null
      }
    }
  }

  if (kind === 'CompositeMo') {
    return {
      runReq,
      kind,
      command: 'compute_composite_cpu_drilldown',
      request: {
        ...base,
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        signal_mode: params.signal_mode || params.signalMode || runReq.signal_mode || runReq.signalMode || 'hard_gate',
        strategy: params.strategy || base.strategy
      }
    }
  }

  return {
    runReq,
    kind,
    command: 'compute_composite_cpu_drilldown',
    request: {
      ...base,
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
      asr: runReq.asr || null,
      vemr: runReq.vemr || null,
      signal_mode: runReq.signal_mode || runReq.signalMode || 'hard_gate'
    }
  }
}

export function buildExecutionStrategyFromRunContext(paramsRaw: Record<string, unknown>, runReqRaw: Record<string, unknown>): AnalysisExecutionStrategySnapshot | null {
  const invocation = buildDrilldownInvocation(paramsRaw, runReqRaw, null)
  if (!invocation) return null
  const strategyKind = normalizeStrategyKindRaw(invocation.kind)
  const kind = invocation.command === 'compute_double_ma_drilldown' ? 'double_ma' : 'composite_cpu'
  return {
    strategyKind,
    strategy: {
      kind,
      req: invocation.request
    }
  }
}

function buildLocalStabilityInvocation(paramsRaw: Record<string, unknown>, invocation: DrillInvocation): StabilityInvocation | null {
  const runReq = invocation.runReq
  const objective = text(runReq.objective) || 'Sharpe'
  const base: Record<string, unknown> = {
    ...invocation.request,
    objective,
    radius: 1,
    step: 1
  }
  if (invocation.command === 'compute_double_ma_drilldown') {
    return {
      command: 'compute_double_ma_local_stability',
      request: base
    }
  }

  const kind = normalizeStrategyKindRaw(runReq.strategy_kind ?? runReq.strategyKind)
  const params = asRecord(paramsRaw)
  if (kind === 'CompositeMo') {
    return {
      command: 'compute_composite_cpu_local_stability',
      request: {
        ...base,
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        signal_mode: params.signal_mode || params.signalMode || runReq.signal_mode || runReq.signalMode || 'hard_gate',
        strategy: params.strategy || runReq.strategy || runReq.base_strategy || {}
      }
    }
  }
  return {
    command: 'compute_composite_cpu_local_stability',
    request: {
      ...base,
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
      asr: runReq.asr || null,
      vemr: runReq.vemr || null,
      signal_mode: runReq.signal_mode || runReq.signalMode || 'hard_gate'
    }
  }
}

async function runVueDrillPipeline(index: number, row: TopRowModel): Promise<boolean> {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  const runReq = asRecord(run?.req)
  const runRes = asRecord(run?.res)
  const params = asRecord(row.params)
  const metrics = asRecord(row.metrics)
  const invocation = buildDrilldownInvocation(params, runReq, runRes)
  if (!invocation) return false

  drillSeq = (drillSeq + 1) >>> 0
  const seq = drillSeq
  if (store) {
    store.drillStatus = 'Computing drilldown…'
    store.stabilityStatus = '-'
    markAnalysisVue(store)
  }
  setStatus(store, 'Computing drilldown…')
  try {
    const drillKey = requestCacheKey(invocation.command, invocation.request)
    const cachedDrill = drillCurvesCache.get(drillKey)
    const curves = cachedDrill ?? (await invokeCommand(invocation.command, { req: invocation.request }))
    if (!cachedDrill) cacheSet(drillCurvesCache, drillKey, curves, 96)
    if (seq !== drillSeq) return true

    let bars: DrillPriceBar[] | null = null
    const dataId = text(runReq.data_id ?? runReq.dataId)
    if (dataId) {
      try {
        bars = await fetchDrillPriceBars(dataId)
      } catch {
        bars = null
      }
    }
    const rendered = renderDrillDomFallback(curves, params, metrics, row, bars)
    if (rendered) {
      void renderDrillTvChartDom(curves, bars, seq).catch(() => {})
    }

    if (store) {
      store.drillStatus = cachedDrill ? 'Done (cached).' : 'Done.'
      const curvesRow = asRecord(curves)
      const eq = asRecord(curvesRow.equity)
      const metricPart = formatDrillMetricsSummary(metrics)
      const moPart = formatDrillMoSummary(row.mo)
      const parts = [`${text(params.fast_ma_type) || '?'}(${num(params.fast_len, 0) || '?'}) vs ${text(params.slow_ma_type) || '?'}(${num(params.slow_len, 0) || '?'})`]
      if (metricPart) parts.push(metricPart)
      if (moPart) parts.push(moPart)
      parts.push(`bins=${num(eq.bins, 0)} start_t=${num(eq.start_t, 0)} end_t=${num(eq.end_t, 0)}`)
      store.drillMeta = parts.join(' | ')
      store.topSelectionGuardrail = `Selected row ${index + 1} of ${readTopRows(store).length}`
      markAnalysisVue(store)
    }
    setStatus(store, cachedDrill ? 'Drilldown done (cached).' : 'Drilldown done.')

    const stability = buildLocalStabilityInvocation(params, invocation)
    if (!stability) return true
    const stabilityKey = requestCacheKey(stability.command, stability.request)
    const cachedStability = stabilityCache.get(stabilityKey)
    if (store) {
      store.stabilityStatus = 'Computing stability…'
      markAnalysisVue(store)
    }
    const stabilityRes = cachedStability ?? (await invokeCommand(stability.command, { req: stability.request }))
    if (!cachedStability) cacheSet(stabilityCache, stabilityKey, stabilityRes, 96)
    if (seq !== drillSeq) return true
    renderStabilityDom(stabilityRes, store)
    if (store) {
      const sr = asRecord(stabilityRes)
      const summary = asRecord(sr.summary)
      store.stabilityMeta = `objective=${text(sr.objective) || '?'} radius=${num(sr.radius, 0)} step=${num(sr.step, 0)} points=${Array.isArray(sr.points) ? sr.points.length : 0} | score: base=${fmt(summary.base, 6)} median=${fmt(summary.median, 6)} min=${fmt(summary.min, 6)} max=${fmt(summary.max, 6)}`
      store.stabilityStatus = cachedStability ? 'Done (cached).' : 'Done.'
      markAnalysisVue(store)
    }
    return true
  } catch (error) {
    if (seq !== drillSeq) return true
    const message = `Drilldown failed: ${String(error)}`
    if (store) {
      store.drillStatus = message
      markAnalysisVue(store)
    }
    setStatus(store, message)
    return false
  }
}

export function refreshTopTableFromWorkflowRun(): boolean {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  if (!store || !run) return false
  const req = asRecord(run.req)
  const res = asRecord(run.res)
  if (!Object.keys(res).length) return false

  refreshOverviewFromWorkflowRunStore(store, req, res)

  const objective = objectiveOf(req)
  const sort = readSortState(store)
  const topK = requestedTopK(req)
  const collected = collectTopRows(res, { allowBestFallback: topK !== 0 })
  const sorted = sortRows(collected, sort.key, sort.dir, objective)
  const filters = readFilters(store)
  const filtered = buildTopRowModelsForCurrentFilters(store, req, res)
  topRowsModelCache = filtered
  const selectedOld = readTopRows(store).findIndex((row) => !!row?.selected)
  const selectedIndex = filtered.length ? (selectedOld >= 0 ? Math.min(selectedOld, filtered.length - 1) : 0) : -1
  const mo = isMo(req, res)

  const rows: TopRow[] = filtered.length
    ? filtered.map((row, index) => {
        const p = row.params || {}
        const m = row.metrics || {}
        const d = row.mo || {}
        const cells: TopRowCell[] = [
          { text: String(row.rank), className: 'left mono', title: '' },
          {
            text: `${String(p.fast_ma_type || '?')}(${String(p.fast_len || '?')})`,
            className: 'left mono',
            title: ''
          },
          {
            text: `${String(p.slow_ma_type || '?')}(${String(p.slow_len || '?')})`,
            className: 'left mono',
            title: ''
          },
          { text: fmtDisplay(m.pnl, 4), className: 'mono', title: fmt(m.pnl, 6) },
          { text: fmtPctDisplay(m.pnl, 2), className: 'mono', title: fmtPct(m.pnl, 2) },
          { text: fmtDisplay(m.sharpe, 4), className: 'mono', title: fmt(m.sharpe, 6) },
          { text: fmtDisplay(m.max_dd, 4), className: 'mono', title: fmt(m.max_dd, 6) },
          { text: String(num(m.trades, 0)), className: 'mono', title: '' },
          { text: fmtDisplay(m.exposure, 4), className: 'mono', title: fmt(m.exposure, 4) }
        ]
        if (!mo)
          cells.push({
            text: fmtDisplay(scoreOf(m, objective), 4),
            className: 'mono',
            title: fmt(scoreOf(m, objective), 6)
          })
        else {
          cells.push({
            text: fmtDisplay(d.maximize_value, 4),
            className: 'mono',
            title: fmt(d.maximize_value, 6)
          })
          cells.push({
            text: fmtDisplay(d.minimize_value, 4),
            className: 'mono',
            title: fmt(d.minimize_value, 6)
          })
          cells.push({
            text: fmtDisplay(d.violation, 4),
            className: 'mono',
            title: fmt(d.violation, 6)
          })
        }
        return { selected: index === selectedIndex, cells }
      })
    : [
        {
          selected: false,
          cells: [
            {
              text: sorted.length ? 'No rows match the current filters.' : 'No rows.',
              className: 'left muted',
              title: ''
            }
          ]
        }
      ]

  store.topColumns = topColumnsFor(req, res, sort)
  store.topTableRows = rows
  store.topRows = filtered.length
  if (!filtered.length && topK === 0 && !Array.isArray(res.pareto)) {
    const combos = combosOf(res)
    store.topFilterMeta = combos > 0
      ? `Top K disabled for this run (top_k=0). Evaluated ${combos} combinations; only the best candidate was returned.`
      : 'Top K disabled for this run (top_k=0). Only the best candidate was returned.'
    store.topSelectionGuardrail = 'Set Advanced > Top K above 0, then rerun optimization to inspect a ranked shortlist.'
  } else {
    store.topFilterMeta = `showing ${filtered.length}/${sorted.length} rows`
    store.topSelectionGuardrail = filtered.length
      ? (hasValidationResult()
        ? `Validation context loaded for ${text(req.strategy_kind ?? req.strategyKind) || 'DoubleMa'}. Final selection should still include holdout/meta confirmation.`
        : 'Selection-bias guardrail: these are optimization-only winners. Run Validation before treating them as robust.')
      : '—'
  }
  store.canClearTopFilters = Object.keys(filters).length > 0
  markAnalysisVue(store)
  return true
}

export function refreshAnalysisFromWorkflowRun(): boolean {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  if (!store || !run) return false
  const req = asRecord(run.req)
  const res = asRecord(run.res)
  if (!Object.keys(res).length) return false
  refreshOverviewFromWorkflowRunStore(store, req, res)
  refreshTopTableFromWorkflowRun()
  refreshParetoFromWorkflowRun()
  refreshHeatmapFromWorkflowRun()
  markAnalysisVue(store)
  return true
}

function setStatus(store: AnalysisStoreState | null, message: string): void {
  if (!store) return
  store.statusLine = text(message) || 'Ready.'
  markAnalysisVue(store)
}

function kindFromReq(req: Record<string, unknown>): string {
  return text(req.strategy_kind ?? req.strategyKind) || 'DoubleMa'
}

function combosOf(res: Record<string, unknown>): number {
  return num(res.num_combos ?? res.total_evaluations, 0)
}

function runtimeMsOf(res: Record<string, unknown>): number {
  return num(res.runtime_ms, 0)
}

function formatBestTitle(params: Record<string, unknown>): string {
  const fastType = text(params.fast_ma_type).toUpperCase() || '?'
  const fastLen = num(params.fast_len, 0)
  const slowType = text(params.slow_ma_type).toUpperCase() || '?'
  const slowLen = num(params.slow_len, 0)
  return `${fastType} ${fastLen > 0 ? fastLen : '?'} vs ${slowType} ${slowLen > 0 ? slowLen : '?'}`
}

function buildBestStats(metrics: Record<string, unknown>): Array<{ label: string; value: string; tone: string }> {
  return [
    { label: 'PnL', value: fmt(metrics.pnl, 6), tone: 'positive' },
    { label: 'Return', value: fmtPct(metrics.pnl, 2), tone: 'neutral' },
    { label: 'Sharpe', value: fmt(metrics.sharpe, 6), tone: 'positive' },
    { label: 'Downside Dev', value: fmt(metrics.downside_deviation, 6), tone: 'neutral' },
    { label: 'Max DD', value: fmt(metrics.max_dd, 6), tone: 'danger' },
    { label: 'Ulcer', value: fmt(metrics.ulcer_index, 6), tone: 'danger' },
    { label: 'Martin', value: fmt(metrics.martin_ratio, 6), tone: 'positive' },
    { label: 'Trades', value: String(num(metrics.trades, 0)), tone: 'neutral' },
    { label: 'Exposure', value: fmt(metrics.exposure, 4), tone: 'neutral' }
  ].filter((entry) => entry.value && entry.value !== '-')
}

function refreshOverviewFromWorkflowRunStore(store: AnalysisStoreState, req: Record<string, unknown>, res: Record<string, unknown>): void {
  const best = pickBest(res, req)
  const params = best.params || {}
  const metrics = best.metrics || {}
  const strategyKind = text(req.strategy_kind ?? req.strategyKind) || 'DoubleMa'
  const objective = objectiveOf(req)
  const comboCount = combosOf(res)
  const runtimeMs = runtimeMsOf(res)
  const metaParts = [
    `strategy=${strategyKind}`,
    objective ? `objective=${objective}` : '',
    comboCount > 0 ? `combos=${comboCount}` : '',
    runtimeMs > 0 ? `runtime=${runtimeMs}ms` : ''
  ].filter((part) => part.length > 0)

  store.bestKicker = isMo(req, res) ? 'Pareto leader' : 'Best candidate'
  store.bestTitle = formatBestTitle(params)
  store.bestMeta = metaParts.join(' · ') || 'Optimization result loaded.'
  store.bestStats = markRawEntity(buildBestStats(metrics))
  store.bestSummary = [
    `strategy=${strategyKind}`,
    `title=${formatBestTitle(params)}`,
    objective ? `objective=${objective}` : '',
    `pnl=${fmt(metrics.pnl, 6)}`,
    `return=${fmtPct(metrics.pnl, 2)}`,
    `sharpe=${fmt(metrics.sharpe, 6)}`,
    `downside_dev=${fmt(metrics.downside_deviation, 6)}`,
    `max_dd=${fmt(metrics.max_dd, 6)}`,
    `ulcer_index=${fmt(metrics.ulcer_index, 6)}`,
    `martin_ratio=${fmt(metrics.martin_ratio, 6)}`,
    `trades=${num(metrics.trades, 0)}`,
    `exposure=${fmt(metrics.exposure, 4)}`,
    comboCount > 0 ? `combos=${comboCount}` : '',
    runtimeMs > 0 ? `runtime_ms=${runtimeMs}` : ''
  ].filter((part) => part.length > 0).join('\n')
}

function pickBest(res: Record<string, unknown>, req: Record<string, unknown>): { params: Record<string, unknown>; metrics: Record<string, unknown> } {
  if (isMo(req, res) && Array.isArray(res.pareto) && res.pareto.length > 0) {
    const rows = collectTopRows(res)
    if (rows.length > 0) {
      const top = rows[0]
      if (top) return { params: top.params || {}, metrics: top.metrics || {} }
    }
  }
  const bestParams = asRecord(res.best_params)
  const bestMetrics = asRecord(res.best_metrics)
  if (Object.keys(bestParams).length && Object.keys(bestMetrics).length) {
    return { params: bestParams, metrics: bestMetrics }
  }
  const top = Array.isArray(res.top) ? res.top : []
  const first = Array.isArray(top[0]) ? (top[0] as unknown[]) : []
  return { params: asRecord(first[0]), metrics: asRecord(first[1]) }
}

function paretoAxisOptionsFor(req: Record<string, unknown>, res: Record<string, unknown>): Array<{ value: string; label: string; disabled: boolean }> {
  const base = [
    { value: 'pnl', label: 'PnL', disabled: false },
    { value: 'sharpe', label: 'Sharpe', disabled: false },
    { value: 'max_dd', label: 'Max DD', disabled: false },
    { value: 'trades', label: 'Trades', disabled: false },
    { value: 'exposure', label: 'Exposure', disabled: false },
    { value: 'score', label: 'Score', disabled: false },
  ]
  if (isMo(req, res)) {
    return [
      ...base,
      { value: 'maximize_value', label: 'Max objective', disabled: false },
      { value: 'minimize_value', label: 'Min objective', disabled: false },
      { value: 'violation', label: 'Violation', disabled: false },
    ]
  }
  return base
}

function paretoAxisValue(row: TopRowModel, req: Record<string, unknown>, keyRaw: unknown): number {
  const key = text(keyRaw || 'pnl') || 'pnl'
  if (key === 'maximize_value') return num(row.mo?.maximize_value, Number.NaN)
  if (key === 'minimize_value') return num(row.mo?.minimize_value, Number.NaN)
  if (key === 'violation') return num(row.mo?.violation, Number.NaN)
  if (key === 'score') return scoreOf(row.metrics || {}, objectiveOf(req))
  return num((row.metrics || {})[key], Number.NaN)
}

function renderParetoCanvas(store: AnalysisStoreState, req: Record<string, unknown>, res: Record<string, unknown>): boolean {
  const wrap = byIds<HTMLElement>(['paretoWrapVue'])
  const metaEl = byIds<HTMLElement>(['paretoMetaVue'])
  const canvas = byIds<HTMLCanvasElement>(['paretoCanvasVue'])
  if (!wrap || !metaEl || !canvas) return false

  const rows = collectTopRows(res, { allowBestFallback: true }).slice(0, 512)
  const xKey = text(store.paretoX || 'max_dd') || 'max_dd'
  const yKey = text(store.paretoY || 'pnl') || 'pnl'
  const points = rows
    .map((row, index) => ({
      index,
      row,
      x: paretoAxisValue(row, req, xKey),
      y: paretoAxisValue(row, req, yKey),
    }))
    .filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y))

  if (!points.length) {
    wrap.style.display = 'none'
    store.paretoWrapVisible = false
    store.paretoMeta = 'Pareto scatter unavailable for this run.'
    metaEl.textContent = text(store.paretoMeta)
    return true
  }

  const ctx = canvas.getContext('2d')
  if (!ctx) return false
  const width = canvas.width
  const height = canvas.height
  const padding = { left: 56, right: 18, top: 18, bottom: 34 }
  const plotWidth = Math.max(1, width - padding.left - padding.right)
  const plotHeight = Math.max(1, height - padding.top - padding.bottom)
  const xs = points.map((point) => point.x)
  const ys = points.map((point) => point.y)
  let xMin = Math.min(...xs)
  let xMax = Math.max(...xs)
  let yMin = Math.min(...ys)
  let yMax = Math.max(...ys)
  if (xMin === xMax) { xMin -= 1; xMax += 1 }
  if (yMin === yMax) { yMin -= 1; yMax += 1 }

  const xPad = (xMax - xMin) * 0.08
  const yPad = (yMax - yMin) * 0.08
  xMin -= xPad
  xMax += xPad
  yMin -= yPad
  yMax += yPad

  const toX = (value: number): number => padding.left + (((value - xMin) / Math.max(1e-12, xMax - xMin)) * plotWidth)
  const toY = (value: number): number => padding.top + (plotHeight - (((value - yMin) / Math.max(1e-12, yMax - yMin)) * plotHeight))

  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = 'rgba(248, 250, 252, 0.02)'
  ctx.fillRect(padding.left, padding.top, plotWidth, plotHeight)
  ctx.strokeStyle = 'rgba(118, 140, 166, 0.32)'
  ctx.lineWidth = 1
  ctx.strokeRect(padding.left, padding.top, plotWidth, plotHeight)

  ctx.font = '11px ui-monospace, monospace'
  ctx.fillStyle = 'rgba(118, 140, 166, 0.92)'
  ctx.fillText(xKey, width - padding.right - 48, height - 8)
  ctx.save()
  ctx.translate(13, padding.top + (plotHeight / 2))
  ctx.rotate(-Math.PI / 2)
  ctx.fillText(yKey, 0, 0)
  ctx.restore()
  ctx.fillText(fmtDisplay(xMin, 3), padding.left, height - 8)
  ctx.fillText(fmtDisplay(xMax, 3), width - padding.right - 42, height - 8)
  ctx.fillText(fmtDisplay(yMax, 3), 8, padding.top + 4)
  ctx.fillText(fmtDisplay(yMin, 3), 8, padding.top + plotHeight)

  const frontier = [...points].sort((left, right) => left.x - right.x)
  ctx.strokeStyle = 'rgba(82, 164, 255, 0.55)'
  ctx.lineWidth = 1.25
  ctx.beginPath()
  frontier.forEach((point, index) => {
    const x = toX(point.x)
    const y = toY(point.y)
    if (index === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  })
  ctx.stroke()

  const clickable: Array<{ x: number; y: number; row: TopRowModel }> = []
  for (const point of points) {
    const x = toX(point.x)
    const y = toY(point.y)
    clickable.push({ x, y, row: point.row })
    ctx.fillStyle = 'rgba(56, 189, 248, 0.88)'
    ctx.beginPath()
    ctx.arc(x, y, 3.5, 0, Math.PI * 2)
    ctx.fill()
  }

  canvas.onclick = (event: MouseEvent) => {
    const rect = canvas.getBoundingClientRect()
    if (!rect.width || !rect.height) return
    const px = ((event.clientX - rect.left) / rect.width) * width
    const py = ((event.clientY - rect.top) / rect.height) * height
    let best: { row: TopRowModel; dist: number } | null = null
    for (const point of clickable) {
      const dx = point.x - px
      const dy = point.y - py
      const dist = Math.sqrt((dx * dx) + (dy * dy))
      if (!best || dist < best.dist) best = { row: point.row, dist }
    }
    if (!best || best.dist > 18) return
    const pinia = getActivePinia()
    if (pinia) {
      const workflowStore = useWorkflowStore(pinia)
      workflowStore.selectResultsTab('drill', { persist: false })
    }
    void runVueDrillPipeline(0, best.row)
  }

  wrap.style.display = 'block'
  store.paretoWrapVisible = true
  store.paretoMeta = `${points.length} points • x=${xKey} (${fmtDisplay(xMin, 3)} to ${fmtDisplay(xMax, 3)}) • y=${yKey} (${fmtDisplay(yMin, 3)} to ${fmtDisplay(yMax, 3)})`
  metaEl.textContent = text(store.paretoMeta)
  return true
}

export function refreshParetoFromWorkflowRun(): boolean {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  if (!store || !run) return false
  const req = asRecord(run.req)
  const res = asRecord(run.res)
  if (!Object.keys(res).length) return false
  const options = paretoAxisOptionsFor(req, res)
  store.paretoXOptions = options
  store.paretoYOptions = options
  const validKeys = new Set(options.map((option) => option.value))
  if (!validKeys.has(text(store.paretoX))) store.paretoX = isMo(req, res) ? 'maximize_value' : 'max_dd'
  if (!validKeys.has(text(store.paretoY))) store.paretoY = isMo(req, res) ? 'minimize_value' : 'pnl'
  const ok = renderParetoCanvas(store, req, res)
  markAnalysisVue(store)
  return ok
}

function readHeatmapModel(res: Record<string, unknown>): HeatmapModel | null {
  const heatmap = asRecord(res.heatmap)
  if (!Object.keys(heatmap).length) return null
  const binsFast = Math.max(0, Math.floor(num(heatmap.bins_fast, 0)))
  const binsSlow = Math.max(0, Math.floor(num(heatmap.bins_slow, 0)))
  const fastMin = Math.max(0, Math.floor(num(heatmap.fast_min, 0)))
  const fastMax = Math.max(0, Math.floor(num(heatmap.fast_max, 0)))
  const slowMin = Math.max(0, Math.floor(num(heatmap.slow_min, 0)))
  const slowMax = Math.max(0, Math.floor(num(heatmap.slow_max, 0)))
  const rawValues = Array.isArray(heatmap.values) ? heatmap.values : []
  if (binsFast <= 0 || binsSlow <= 0 || !rawValues.length) return null
  return {
    binsFast,
    binsSlow,
    fastMin,
    fastMax,
    slowMin,
    slowMax,
    values: rawValues.map((value) => {
      if (value == null) return null
      const next = Number(value)
      return Number.isFinite(next) ? next : null
    }),
  }
}

function finiteHeatmapValues(model: HeatmapModel): number[] {
  const out: number[] = []
  for (const value of model.values) {
    if (typeof value === 'number' && Number.isFinite(value)) out.push(value)
  }
  return out
}

function quantile(sorted: number[], p: number): number {
  if (!sorted.length) return 0
  const clamped = Math.max(0, Math.min(1, p))
  const pos = (sorted.length - 1) * clamped
  const base = Math.floor(pos)
  const frac = pos - base
  const left = sorted[base] ?? sorted[0] ?? 0
  const right = sorted[Math.min(sorted.length - 1, base + 1)] ?? left
  return left + ((right - left) * frac)
}

function resolveHeatmapRange(values: number[]): { min: number; max: number; label: string } {
  if (!values.length) return { min: -1, max: 1, label: 'zero-centered' }
  const magnitudes = values.map((value) => Math.abs(value)).sort((a, b) => a - b)
  const maxAbs = Math.max(1e-12, quantile(magnitudes, 0.98), magnitudes[magnitudes.length - 1] ?? 1)
  return { min: -maxAbs, max: maxAbs, label: 'zero-centered' }
}

function heatmapPalette(): { negative: string; zero: string; positive: string } {
  return {
    negative: '#b91c1c',
    zero: '#2563eb',
    positive: '#15803d',
  }
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const normalized = hex.replace('#', '').trim()
  const chunk = normalized.length === 3
    ? normalized.split('').map((value) => `${value}${value}`).join('')
    : normalized.padStart(6, '0').slice(0, 6)
  const numValue = Number.parseInt(chunk, 16)
  return {
    r: (numValue >> 16) & 0xff,
    g: (numValue >> 8) & 0xff,
    b: numValue & 0xff,
  }
}

function rgbToCss(rgb: { r: number; g: number; b: number }, alpha = 1): string {
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${Math.max(0, Math.min(1, alpha))})`
}

function sampleHeatmapColor(value: number, maxAbs: number, palette: { negative: string; zero: string; positive: string }): string {
  const anchor = Math.max(1e-12, Math.abs(maxAbs))
  const zero = hexToRgb(palette.zero)
  if (value <= 0) {
    const neg = hexToRgb(palette.negative)
    const ratio = Math.max(0, Math.min(1, Math.abs(value) / anchor))
    return rgbToCss({
      r: Math.round(zero.r + ((neg.r - zero.r) * ratio)),
      g: Math.round(zero.g + ((neg.g - zero.g) * ratio)),
      b: Math.round(zero.b + ((neg.b - zero.b) * ratio)),
    })
  }
  const pos = hexToRgb(palette.positive)
  const ratio = Math.max(0, Math.min(1, value / anchor))
  return rgbToCss({
    r: Math.round(zero.r + ((pos.r - zero.r) * ratio)),
    g: Math.round(zero.g + ((pos.g - zero.g) * ratio)),
    b: Math.round(zero.b + ((pos.b - zero.b) * ratio)),
  })
}

function heatmapIndex(model: HeatmapModel, logicalRow: number, col: number): number {
  return (logicalRow * model.binsFast) + col
}

function fastForHeatmapCol(model: HeatmapModel, col: number): number {
  if (model.binsFast <= 1) return model.fastMin
  const ratio = col / Math.max(1, model.binsFast - 1)
  return Math.round(model.fastMin + ((model.fastMax - model.fastMin) * ratio))
}

function slowForHeatmapLogicalRow(model: HeatmapModel, logicalRow: number): number {
  if (model.binsSlow <= 1) return model.slowMin
  const ratio = logicalRow / Math.max(1, model.binsSlow - 1)
  return Math.round(model.slowMin + ((model.slowMax - model.slowMin) * ratio))
}

function heatmapColForFast(model: HeatmapModel, fast: number): number {
  if (model.binsFast <= 1 || model.fastMax <= model.fastMin) return 0
  const ratio = (fast - model.fastMin) / Math.max(1, model.fastMax - model.fastMin)
  return Math.max(0, Math.min(model.binsFast - 1, Math.round(ratio * (model.binsFast - 1))))
}

function heatmapLogicalRowForSlow(model: HeatmapModel, slow: number): number {
  if (model.binsSlow <= 1 || model.slowMax <= model.slowMin) return 0
  const ratio = (slow - model.slowMin) / Math.max(1, model.slowMax - model.slowMin)
  return Math.max(0, Math.min(model.binsSlow - 1, Math.round(ratio * (model.binsSlow - 1))))
}

function renderHeatmapLegend(min: number, max: number, palette: { negative: string; zero: string; positive: string }): void {
  const legend = byIds<HTMLCanvasElement>(['heatmapLegendVue'])
  if (!legend) return
  const ctx = legend.getContext('2d')
  if (!ctx) return
  const width = legend.width
  const height = legend.height
  const gradient = ctx.createLinearGradient(0, 0, width, 0)
  gradient.addColorStop(0, palette.negative)
  gradient.addColorStop(0.5, palette.zero)
  gradient.addColorStop(1, palette.positive)
  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, width, height)
  const minLabel = byIds<HTMLElement>(['heatmapLegendMinVue'])
  const midLabel = byIds<HTMLElement>(['heatmapLegendMidVue'])
  const maxLabel = byIds<HTMLElement>(['heatmapLegendMaxVue'])
  if (minLabel) minLabel.textContent = fmtDisplay(min, 4)
  if (midLabel) midLabel.textContent = fmtDisplay((min + max) / 2, 4)
  if (maxLabel) maxLabel.textContent = fmtDisplay(max, 4)
}

function heatmapMarkerRows(res: Record<string, unknown>, model: HeatmapModel): Array<{ col: number; drawRow: number; label: string }> {
  const rows = collectTopRows(res, { allowBestFallback: true }).slice(0, 8)
  const out: Array<{ col: number; drawRow: number; label: string }> = []
  for (let i = 0; i < rows.length; i += 1) {
    const row = rows[i]
    if (!row) continue
    const fast = Math.max(0, Math.floor(num(row.params.fast_len, 0)))
    const slow = Math.max(0, Math.floor(num(row.params.slow_len, 0)))
    if (fast <= 0 || slow <= 0) continue
    const col = heatmapColForFast(model, fast)
    const logicalRow = heatmapLogicalRowForSlow(model, slow)
    out.push({
      col,
      drawRow: model.binsSlow - 1 - logicalRow,
      label: String(i + 1),
    })
  }
  return out
}

function hideHeatmapTooltip(): void {
  const tooltip = byIds<HTMLElement>(['heatmapTooltipVue'])
  if (tooltip) tooltip.style.display = 'none'
}

function buildHeatmapCellParams(req: Record<string, unknown>, model: HeatmapModel, col: number, logicalRow: number): Record<string, unknown> {
  const fast = fastForHeatmapCol(model, col)
  const slowBase = slowForHeatmapLogicalRow(model, logicalRow)
  const slow = Math.max(slowBase, fast + 1)
  return {
    fast_len: fast,
    fast_ma_type: text(Array.isArray(req.fast_ma_types) ? req.fast_ma_types[0] : req.fast_ma_type) || '?',
    slow_len: slow,
    slow_ma_type: text(Array.isArray(req.slow_ma_types) ? req.slow_ma_types[0] : req.slow_ma_type) || '?',
    fast_ma_params: req.fast_ma_params ?? req.fastMaParams ?? null,
    slow_ma_params: req.slow_ma_params ?? req.slowMaParams ?? null,
  }
}

function renderHeatmapCanvas(store: AnalysisStoreState, req: Record<string, unknown>, res: Record<string, unknown>): boolean {
  const model = readHeatmapModel(res)
  const wrap = byIds<HTMLElement>(['heatmapWrapVue'])
  const hint = byIds<HTMLElement>(['heatmapEmptyHintVue'])
  const meta = byIds<HTMLElement>(['heatmapMetaVue'])
  const canvas = byIds<HTMLCanvasElement>(['heatmapCanvasVue'])
  const overlay = byIds<HTMLCanvasElement>(['heatmapOverlayVue'])
  const tooltip = byIds<HTMLElement>(['heatmapTooltipVue'])
  if (!wrap || !hint || !meta || !canvas || !overlay || !tooltip) return false

  const topK = requestedTopK(req)
  const heatmapBins = requestedHeatmapBins(req)
  if (!model) {
    wrap.style.display = 'none'
    hint.style.display = 'block'
    store.heatmapWrapVisible = false
    store.canHeatmapExportPng = false
    store.heatmapStatusLine = heatmapBins <= 0
      ? 'Heatmap disabled for this run. Increase Advanced > Heatmap bins above 0, then rerun optimization.'
      : 'No heatmap payload was returned for this run.'
    store.heatmapMeta = heatmapBins <= 0
      ? 'Heatmap disabled for this run (heatmap_bins=0).'
      : 'Heatmap data unavailable for this run.'
    if (meta) meta.textContent = text(store.heatmapMeta)
    hideHeatmapTooltip()
    return true
  }

  const finite = finiteHeatmapValues(model)
  if (!finite.length) {
    wrap.style.display = 'none'
    hint.style.display = 'block'
    store.heatmapWrapVisible = false
    store.canHeatmapExportPng = false
    store.heatmapMeta = 'Heatmap payload was returned, but it contained no populated cells.'
    store.heatmapStatusLine = 'Heatmap unavailable for this run.'
    meta.textContent = text(store.heatmapMeta)
    hideHeatmapTooltip()
    return true
  }

  const range = resolveHeatmapRange(finite)
  const palette = heatmapPalette()
  const width = canvas.width
  const height = canvas.height
  const ctx = canvas.getContext('2d')
  const overlayCtx = overlay.getContext('2d')
  if (!ctx || !overlayCtx) return false

  const cellW = width / Math.max(1, model.binsFast)
  const cellH = height / Math.max(1, model.binsSlow)

  ctx.clearRect(0, 0, width, height)
  for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
    const drawRow = model.binsSlow - 1 - logicalRow
    for (let col = 0; col < model.binsFast; col += 1) {
      const value = model.values[heatmapIndex(model, logicalRow, col)]
      const x = col * cellW
      const y = drawRow * cellH
      if (typeof value !== 'number' || !Number.isFinite(value)) {
        ctx.fillStyle = 'rgba(82, 96, 118, 0.12)'
        ctx.fillRect(x, y, cellW, cellH)
        continue
      }
      ctx.fillStyle = sampleHeatmapColor(value, Math.max(Math.abs(range.min), Math.abs(range.max)), palette)
      ctx.fillRect(x, y, cellW, cellH)
    }
  }

  const markers = heatmapMarkerRows(res, model)
  const paintOverlay = (): void => {
    overlayCtx.clearRect(0, 0, width, height)
    for (const marker of markers) {
      const cx = ((marker.col + 0.5) * cellW)
      const cy = ((marker.drawRow + 0.5) * cellH)
      overlayCtx.fillStyle = 'rgba(248, 250, 252, 0.95)'
      overlayCtx.beginPath()
      overlayCtx.arc(cx, cy, Math.max(3, Math.min(cellW, cellH) * 0.18), 0, Math.PI * 2)
      overlayCtx.fill()
      overlayCtx.strokeStyle = 'rgba(15, 23, 42, 0.95)'
      overlayCtx.lineWidth = 1
      overlayCtx.stroke()
      overlayCtx.fillStyle = 'rgba(15, 23, 42, 0.95)'
      overlayCtx.font = '11px ui-monospace, monospace'
      overlayCtx.fillText(marker.label, cx + 5, cy - 5)
    }

    if (!heatmapHoverCell) return
    const hoverX = heatmapHoverCell.col * cellW
    const hoverY = heatmapHoverCell.row * cellH
    overlayCtx.strokeStyle = 'rgba(248, 250, 252, 0.95)'
    overlayCtx.lineWidth = 1.5
    overlayCtx.strokeRect(hoverX + 0.5, hoverY + 0.5, Math.max(0, cellW - 1), Math.max(0, cellH - 1))
    overlayCtx.beginPath()
    overlayCtx.moveTo(hoverX + (cellW / 2), 0)
    overlayCtx.lineTo(hoverX + (cellW / 2), height)
    overlayCtx.moveTo(0, hoverY + (cellH / 2))
    overlayCtx.lineTo(width, hoverY + (cellH / 2))
    overlayCtx.strokeStyle = 'rgba(248, 250, 252, 0.28)'
    overlayCtx.stroke()
  }

  paintOverlay()

  overlay.onmouseleave = () => {
    heatmapHoverCell = null
    hideHeatmapTooltip()
    paintOverlay()
  }

  overlay.onmousemove = (event: MouseEvent) => {
    const rect = overlay.getBoundingClientRect()
    if (!rect.width || !rect.height) return
    const col = Math.max(0, Math.min(model.binsFast - 1, Math.floor(((event.clientX - rect.left) / rect.width) * model.binsFast)))
    const drawRow = Math.max(0, Math.min(model.binsSlow - 1, Math.floor(((event.clientY - rect.top) / rect.height) * model.binsSlow)))
    const logicalRow = model.binsSlow - 1 - drawRow
    const value = model.values[heatmapIndex(model, logicalRow, col)]
    heatmapHoverCell = { col, row: drawRow }
    const fast = fastForHeatmapCol(model, col)
    const slow = slowForHeatmapLogicalRow(model, logicalRow)
    tooltip.style.display = 'block'
    tooltip.style.left = `${Math.max(8, event.clientX - rect.left + 12)}px`
    tooltip.style.top = `${Math.max(8, event.clientY - rect.top + 12)}px`
    tooltip.textContent = `${text(Array.isArray(req.fast_ma_types) ? req.fast_ma_types[0] : req.fast_ma_type).toUpperCase() || 'FAST'} ${fast} · ${text(Array.isArray(req.slow_ma_types) ? req.slow_ma_types[0] : req.slow_ma_type).toUpperCase() || 'SLOW'} ${slow} · ${objectiveOf(req)} ${fmtDisplay(value, 4)}`
    paintOverlay()
  }

  overlay.onclick = (event: MouseEvent) => {
    const rect = overlay.getBoundingClientRect()
    if (!rect.width || !rect.height) return
    const col = Math.max(0, Math.min(model.binsFast - 1, Math.floor(((event.clientX - rect.left) / rect.width) * model.binsFast)))
    const drawRow = Math.max(0, Math.min(model.binsSlow - 1, Math.floor(((event.clientY - rect.top) / rect.height) * model.binsSlow)))
    const logicalRow = model.binsSlow - 1 - drawRow
    const value = model.values[heatmapIndex(model, logicalRow, col)]
    if (typeof value !== 'number' || !Number.isFinite(value)) return
    const params = buildHeatmapCellParams(req, model, col, logicalRow)
    const objective = objectiveOf(req)
    const metrics: Record<string, unknown> = { [objectiveScoreKey(objective)]: value }
    const pinia = getActivePinia()
    if (pinia) {
      const workflowStore = useWorkflowStore(pinia)
      workflowStore.selectResultsTab('drill', { persist: false })
    }
    void runVueDrillPipeline(0, { rank: 1, params, metrics })
  }

  wrap.style.display = 'block'
  hint.style.display = 'none'
  renderHeatmapLegend(range.min, range.max, palette)
  store.heatmapWrapVisible = true
  store.canHeatmapExportPng = true
  store.heatmapMeta = `${objectiveOf(req)} heatmap • bins=${model.binsFast}x${model.binsSlow} • fast=${model.fastMin}-${model.fastMax} • slow=${model.slowMin}-${model.slowMax} • scale=${range.label}`
  store.heatmapStatusLine = topK === 0
    ? 'Heatmap ready. This run disabled ranked Top rows (top_k=0), so markers use the best candidate only.'
    : 'Heatmap ready. Hover for values and click a populated cell to drill down.'
  meta.textContent = text(store.heatmapMeta)
  return true
}

export function refreshHeatmapFromWorkflowRun(): boolean {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  if (!store || !run) return false
  const req = asRecord(run.req)
  const res = asRecord(run.res)
  if (!Object.keys(res).length) return false

  store.heatmapColor = 'diverging_zero'
  store.heatmapRange = 'zero_centered'
  store.heatmapShowTop = 'true'
  store.heatmapShowCrosshair = 'true'
  store.heatmapColorOptions = [
    { value: 'diverging_zero', label: 'Diverging zero-centered', disabled: false },
  ]
  store.heatmapRangeOptions = [
    { value: 'zero_centered', label: 'Zero-centered', disabled: false },
  ]
  store.heatmapShowTopOptions = [
    { value: 'true', label: 'Show markers', disabled: false },
  ]
  store.heatmapShowCrosshairOptions = [
    { value: 'true', label: 'Show crosshair', disabled: false },
  ]

  const requestedBins = requestedHeatmapBins(req)
  const model = readHeatmapModel(res)
  if (requestedBins <= 0) {
    store.heatmapWrapVisible = false
    store.canHeatmapExportPng = false
    store.heatmapMeta = 'Heatmap disabled for this run (heatmap_bins=0).'
    store.heatmapStatusLine = 'Increase Advanced > Heatmap bins above 0, then rerun optimization to generate a heatmap.'
  } else if (model) {
    store.heatmapWrapVisible = true
    store.canHeatmapExportPng = true
    store.heatmapMeta = `${objectiveOf(req)} heatmap • bins=${model.binsFast}x${model.binsSlow} • fast=${model.fastMin}-${model.fastMax} • slow=${model.slowMin}-${model.slowMax}`
    store.heatmapStatusLine = 'Heatmap available.'
  } else {
    store.heatmapWrapVisible = false
    store.canHeatmapExportPng = false
    store.heatmapMeta = 'Heatmap data unavailable for this run.'
    store.heatmapStatusLine = 'No heatmap payload was returned for this run.'
  }

  const ok = renderHeatmapCanvas(store, req, res)
  markAnalysisVue(store)
  return ok
}

function stableStringify(value: unknown): string {
  if (value == null) return 'null'
  if (typeof value !== 'object') {
    try {
      return JSON.stringify(value)
    } catch {
      return String(value)
    }
  }
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(',')}]`
  }
  const obj = value as Record<string, unknown>
  const keys = Object.keys(obj).sort()
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(obj[key])}`).join(',')}}`
}

function truncate(value: string, maxLen: number): string {
  const src = String(value || '')
  const limit = Math.max(8, Math.floor(Number(maxLen || 120)))
  if (src.length <= limit) return src
  return `${src.slice(0, limit - 1)}…`
}

function diffParams(left: Record<string, unknown>, right: Record<string, unknown>, limit = 12): Array<{ key: string; from: string; to: string }> {
  const keys = Array.from(new Set([...Object.keys(left), ...Object.keys(right)])).sort()
  const out: Array<{ key: string; from: string; to: string }> = []
  for (const key of keys) {
    const from = stableStringify(left[key])
    const to = stableStringify(right[key])
    if (from === to) continue
    out.push({ key, from, to })
    if (out.length >= limit) break
  }
  return out
}

async function loadBaselineFromDisk(path: string): Promise<BaselineRun | null> {
  const normalizedPath = text(path)
  if (!normalizedPath) return null
  const cached = baselineArtifactCache.get(normalizedPath)
  if (cached) return cached
  const raw = (await invokeCommand('load_run_artifact', {
    path: normalizedPath
  })) as RunArtifactPayload
  const row = asRecord(raw)
  const run = asRecord(row.run)
  const req = asRecord(run.req)
  const res = asRecord(run.res)
  if (!Object.keys(req).length || !Object.keys(res).length) return null
  const baseline: BaselineRun = {
    source: 'disk',
    ts: text(row.ts) || text(run.ts),
    req,
    res
  }
  baselineArtifactCache.set(normalizedPath, baseline)
  return baseline
}

async function resolveBaselineSelection(selectedValue: string, current: WorkflowRunState, history: HistoryStoreState | null): Promise<BaselineRun | null> {
  const selected = text(selectedValue)
  if (!selected) return null
  if (selected.startsWith('disk:')) {
    return await loadBaselineFromDisk(selected.slice(5))
  }

  const baselineTs = selected.startsWith('hist:') ? text(selected.slice(5)) : selected
  const currentTs = text(current.ts)
  const currentReq = asRecord(current.req)
  const currentRes = asRecord(current.res)
  if (baselineTs && currentTs === baselineTs && Object.keys(currentReq).length && Object.keys(currentRes).length) {
    return {
      source: 'hist',
      ts: baselineTs,
      req: currentReq,
      res: currentRes
    }
  }

  const savedRuns = Array.isArray(history?.savedRunsRaw) ? (history?.savedRunsRaw as Array<Record<string, unknown>>) : []
  const hit = savedRuns.find((entry) => text(entry.ts) === baselineTs)
  const path = text(hit?.path)
  if (!path) return null
  const loaded = await loadBaselineFromDisk(path)
  if (!loaded) return null
  return {
    source: 'hist',
    ts: baselineTs || loaded.ts,
    req: loaded.req,
    res: loaded.res
  }
}

function buildCompareSummary(currentRun: WorkflowRunState, baseline: BaselineRun): string {
  const currentTs = text(currentRun.ts)
  const cReq = asRecord(currentRun.req)
  const cRes = asRecord(currentRun.res)
  const bReq = baseline.req
  const bRes = baseline.res
  const cBest = pickBest(cRes, cReq)
  const bBest = pickBest(bRes, bReq)
  const cm = cBest.metrics || {}
  const bm = bBest.metrics || {}
  const cp = cBest.params || {}
  const bp = bBest.params || {}
  const cObj = objectiveOf(cReq)
  const bObj = objectiveOf(bReq)
  const cScore = scoreOf(cm, cObj)
  const bScore = scoreOf(bm, bObj)
  const cRt = runtimeMsOf(cRes)
  const bRt = runtimeMsOf(bRes)
  const ratio = cRt > 0 && bRt > 0 ? bRt / cRt : 0
  const delta = (left: unknown, right: unknown): number => num(left, 0) - num(right, 0)

  const lines: string[] = []
  lines.push(`baseline (${baseline.source}): ${baseline.ts || '-'} | strategy=${kindFromReq(bReq)} | combos=${combosOf(bRes)} | runtime_ms=${bRt || 0}`)
  lines.push(`current:          ${currentTs || '-'} | strategy=${kindFromReq(cReq)} | combos=${combosOf(cRes)} | runtime_ms=${cRt || 0}${ratio ? ` | speedup=${ratio.toFixed(2)}x` : ''}`)
  if (kindFromReq(bReq) !== kindFromReq(cReq)) {
    lines.push(`note: strategies differ (baseline=${kindFromReq(bReq)}, current=${kindFromReq(cReq)})`)
  }
  lines.push('')
  lines.push('Optimization best:')
  lines.push(`  baseline: pnl=${fmt(bm.pnl, 6)} ret=${fmtPct(bm.pnl, 2)} sharpe=${fmt(bm.sharpe, 6)} dd=${fmt(bm.max_dd, 6)} trades=${num(bm.trades, 0)} expo=${fmt(bm.exposure, 4)} score(${bObj})=${fmt(bScore, 6)}`)
  lines.push(`  current:  pnl=${fmt(cm.pnl, 6)} ret=${fmtPct(cm.pnl, 2)} sharpe=${fmt(cm.sharpe, 6)} dd=${fmt(cm.max_dd, 6)} trades=${num(cm.trades, 0)} expo=${fmt(cm.exposure, 4)} score(${cObj})=${fmt(cScore, 6)}`)
  lines.push(`  deltas:   pnl=${fmt(delta(cm.pnl, bm.pnl), 6)} sharpe=${fmt(delta(cm.sharpe, bm.sharpe), 6)} dd=${fmt(delta(cm.max_dd, bm.max_dd), 6)} trades=${delta(cm.trades, bm.trades)} expo=${fmt(delta(cm.exposure, bm.exposure), 4)}`)
  if (cObj === bObj) {
    lines.push(`  scoreΔ:   ${fmt(delta(cScore, bScore), 6)} (${cObj})`)
  }
  lines.push('')

  const diffs = diffParams(bp, cp, 12)
  if (diffs.length > 0) {
    lines.push('Params diff (current - baseline):')
    for (const row of diffs) {
      lines.push(`  ${row.key}: ${truncate(row.from, 60)} -> ${truncate(row.to, 60)}`)
    }
    const totalKeys = new Set([...Object.keys(bp), ...Object.keys(cp)]).size
    if (totalKeys > diffs.length) lines.push(`  … (showing ${diffs.length}/${totalKeys} keys)`)
  }

  return lines.join('\n').trim() || '-'
}

function csvEscape(v: unknown): string {
  const s = String(v ?? '')
  if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`
  return s
}

function startEq(req: Record<string, unknown>): number {
  const direct = Number(req.starting_equity ?? req.startingEquity)
  if (Number.isFinite(direct) && direct > 0) return direct
  const strategy = asRecord(req.strategy)
  const base = asRecord(req.base_strategy)
  const nested = Number(strategy.starting_equity ?? strategy.startingEquity ?? base.starting_equity ?? base.startingEquity)
  if (Number.isFinite(nested) && nested > 0) return nested
  return 10_000
}

function finalEq(pnl: unknown, equity: unknown): number {
  const p = Number(pnl)
  const e = Number(equity)
  if (!Number.isFinite(p)) return NaN
  const seed = Number.isFinite(e) && e > 0 ? e : 10_000
  return seed * (1 + p)
}

function sanitizeTs(v: unknown): string {
  const raw = text(v)
  return (raw || new Date().toISOString()).replace(/[^\w.-]+/g, '-')
}

function downloadText(filename: string, body: string, mime: string): boolean {
  if (typeof document === 'undefined') return false
  if (typeof URL === 'undefined' || typeof URL.createObjectURL !== 'function') return false
  const blob = new Blob([body], { type: mime })
  const href = URL.createObjectURL(blob)
  try {
    const a = document.createElement('a')
    a.href = href
    a.download = filename
    a.style.display = 'none'
    document.body?.appendChild(a)
    a.click()
    a.remove()
    return true
  } finally {
    URL.revokeObjectURL(href)
  }
}

async function copyText(body: string): Promise<boolean> {
  const src = text(body)
  if (!src) return false
  try {
    const clip = (
      globalThis as {
        navigator?: {
          clipboard?: { writeText?: (value: string) => Promise<void> }
        }
      }
    ).navigator?.clipboard
    if (clip && typeof clip.writeText === 'function') {
      await clip.writeText(src)
      return true
    }
  } catch {}
  return false
}

function byIds<T extends HTMLElement>(ids: readonly string[]): T | null {
  if (typeof document === 'undefined') return null
  for (const id of ids) {
    const node = typeof document.querySelector === 'function' ? document.querySelector(`[id="${id}"]`) : null
    if (node) return node as T
    const getById = (
      document as unknown as {
        getElementById?: (nextId: string) => Element | null
      }
    ).getElementById
    if (typeof getById === 'function') {
      const fallback = getById.call(document, id)
      if (fallback) return fallback as T
    }
  }
  return null
}

function isShown(node: HTMLElement | null): boolean {
  if (!node) return false
  if (node.hidden) return false
  if (typeof getComputedStyle !== 'function') return true
  return getComputedStyle(node).display !== 'none'
}

function drillWrap(): HTMLElement | null {
  return byIds<HTMLElement>(['drillWrapVue', 'drillWrap'])
}

function drillTradeTableBody(): HTMLElement | null {
  return byIds<HTMLElement>(['tradeTableBodyVue', 'tradeTableBody'])
}

function drillRows(): HTMLTableRowElement[] {
  const body = drillTradeTableBody()
  if (!body) return []
  return Array.from(body.querySelectorAll<HTMLTableRowElement>('tr[data-trade-idx]'))
}

function clickRow(row: HTMLTableRowElement): boolean {
  if (!row) return false
  if (typeof row.click === 'function') {
    row.click()
    return true
  }
  if (typeof MouseEvent === 'function') {
    row.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }))
    return true
  }
  return false
}

function navigateDrillTradeDom(isNext: boolean): string | null {
  if (!isShown(drillWrap())) return null
  const rows = drillRows()
  if (!rows.length) return null
  let index = rows.findIndex((row) => row.classList.contains('selected'))
  if (index < 0) index = rows.length - 1
  const targetIndex = isNext ? Math.min(rows.length - 1, index + 1) : Math.max(0, index - 1)
  const target = rows[targetIndex]
  if (!target) return null
  if (!clickRow(target)) return null
  const selectedAfter = rows.some((row) => row.classList.contains('selected'))
  if (!selectedAfter) {
    for (const row of rows) row.classList.remove('selected')
    target.classList.add('selected')
  }
  return text(target.cells?.[0]?.textContent) || String(targetIndex + 1)
}

function clearDrillTradeSelectionDom(): boolean {
  if (!isShown(drillWrap())) return false
  const body = drillTradeTableBody()
  if (!body) return false
  const selectedRows = Array.from(body.querySelectorAll<HTMLTableRowElement>('tr.selected'))
  drillSelectedTradeIdx = -1
  if (!selectedRows.length) return false
  for (const row of selectedRows) row.classList.remove('selected')
  const legend = byIds<HTMLElement>(['drillTvLegendVue', 'drillTvLegend'])
  if (legend && /^selected trade #/i.test(text(legend.textContent))) {
    legend.textContent = 'Hover chart to inspect OHLC and indicators.'
  }
  return true
}

function renderStabilityFilterChecklist(metricsRaw: Record<string, unknown>, filters: TopFilterValues): string {
  const metrics = asRecord(metricsRaw)
  const checks: string[] = []
  let allOk = true
  let any = false
  const mark = (ok: boolean, label: string): void => {
    any = true
    if (!ok) allOk = false
    checks.push(`${ok ? 'PASS' : 'FAIL'} ${label}`)
  }
  const pnl = num(metrics.pnl, NaN)
  const sharpe = num(metrics.sharpe, NaN)
  const dd = num(metrics.max_dd, NaN)
  const trades = num(metrics.trades, NaN)
  const expo = num(metrics.exposure, NaN)
  if (filters.minPnl != null) mark(Number.isFinite(pnl) && pnl >= filters.minPnl, `PnL>=${fmt(filters.minPnl, 6)} (${fmt(pnl, 6)})`)
  if (filters.minSharpe != null) mark(Number.isFinite(sharpe) && sharpe >= filters.minSharpe, `Sharpe>=${fmt(filters.minSharpe, 6)} (${fmt(sharpe, 6)})`)
  if (filters.maxDd != null) mark(Number.isFinite(dd) && dd <= filters.maxDd, `MaxDD<=${fmt(filters.maxDd, 6)} (${fmt(dd, 6)})`)
  if (filters.minTrades != null) mark(Number.isFinite(trades) && trades >= filters.minTrades, `Trades>=${String(filters.minTrades)} (${Number.isFinite(trades) ? String(Math.floor(trades)) : '-'})`)
  if (filters.minExposure != null) mark(Number.isFinite(expo) && expo >= filters.minExposure, `Expo>=${fmt(filters.minExposure, 6)} (${fmt(expo, 6)})`)
  if (!any) return 'No Top filters set (nothing to validate).'
  return `${allOk ? 'PASS' : 'FAIL'} ${checks.join(' | ')}`
}

function renderStabilityDom(resultRaw: unknown, store: AnalysisStoreState | null): boolean {
  const result = asRecord(resultRaw)
  const wrap = byIds<HTMLElement>(['stabilityWrapVue', 'stabilityWrap'])
  const statusEl = byIds<HTMLElement>(['stabilityStatusVue', 'stabilityStatus'])
  const metaEl = byIds<HTMLElement>(['stabilityMetaVue', 'stabilityMeta'])
  const rejectEl = byIds<HTMLElement>(['rejectMetaVue', 'rejectMeta'])
  const tableWrap = byIds<HTMLElement>(['stabilityTableWrapVue', 'stabilityTableWrap'])
  const tbody = byIds<HTMLElement>(['stabilityTableBodyVue', 'stabilityTableBody'])
  if (!wrap || !statusEl || !metaEl || !rejectEl || !tableWrap || !tbody) return false

  wrap.style.display = 'block'
  const points = Array.isArray(result.points) ? result.points.map((p) => asRecord(p)) : []
  if (!points.length) {
    statusEl.textContent = 'No stability data.'
    statusEl.className = 'muted'
    metaEl.textContent = '-'
    rejectEl.textContent = '-'
    tableWrap.style.display = 'none'
    tbody.innerHTML = ''
    return true
  }

  const summary = asRecord(result.summary)
  const base = asRecord(result.base)
  const objective = text(result.objective) || '?'
  const radius = num(result.radius, 0)
  const step = num(result.step, 0)
  const startT = num(result.start_t, 0)
  const baseScore = num(summary.base, 0)
  const minScore = num(summary.min, 0)
  const medScore = num(summary.median, 0)
  const maxScore = num(summary.max, 0)
  const peakDelta = baseScore - medScore

  statusEl.textContent = 'Done.'
  statusEl.className = 'muted ok'
  metaEl.textContent = `objective=${objective} radius=${radius} step=${step} pts=${points.length} start_t=${startT} | score: base=${fmt(baseScore, 6)} median=${fmt(medScore, 6)} min=${fmt(minScore, 6)} max=${fmt(maxScore, 6)} peakΔ=${fmt(peakDelta, 6)}`

  const bf = Math.floor(num(base.fast_len, NaN))
  const bs = Math.floor(num(base.slow_len, NaN))
  const basePoint = points.find((p) => Math.floor(num(p.fast_len, NaN)) === bf && Math.floor(num(p.slow_len, NaN)) === bs) ?? null
  const filters = store ? readFilters(store) : {}
  rejectEl.textContent = renderStabilityFilterChecklist(asRecord(basePoint?.metrics), filters)

  const sorted = [...points].sort((a, b) => num(b.score, 0) - num(a.score, 0))
  tbody.innerHTML = ''
  for (const point of sorted) {
    const metrics = asRecord(point.metrics)
    const tr = document.createElement('tr')
    const cells: Array<{ text: string; cls: string }> = [
      { text: String(Math.floor(num(point.fast_len, 0))), cls: 'left mono' },
      { text: String(Math.floor(num(point.slow_len, 0))), cls: 'left mono' },
      { text: fmt(point.score, 6), cls: 'mono' },
      { text: fmt(metrics.pnl, 6), cls: 'mono' },
      { text: fmt(metrics.sharpe, 6), cls: 'mono' },
      { text: fmt(metrics.max_dd, 6), cls: 'mono' },
      { text: String(Math.floor(num(metrics.trades, 0))), cls: 'mono' }
    ]
    for (const cell of cells) {
      const td = document.createElement('td')
      td.className = cell.cls
      td.textContent = cell.text
      tr.appendChild(td)
    }
    tbody.appendChild(tr)
  }
  tableWrap.style.display = 'block'
  return true
}

function drawMinMaxChartDom(canvas: HTMLCanvasElement | null, minmaxRaw: unknown, opts?: { stroke?: string; baseline?: number; height?: number }): void {
  const mm = Array.isArray(minmaxRaw) ? minmaxRaw : []
  const bins = Math.floor(mm.length / 2)
  if (!canvas || bins <= 0) return
  const height = Math.max(60, Math.floor(num(opts?.height, 160)))
  canvas.width = bins
  canvas.height = height
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.clearRect(0, 0, bins, height)

  let minV = Number.POSITIVE_INFINITY
  let maxV = Number.NEGATIVE_INFINITY
  for (let i = 0; i < mm.length; i += 1) {
    const v = Number(mm[i])
    if (!Number.isFinite(v)) continue
    if (v < minV) minV = v
    if (v > maxV) maxV = v
  }
  if (!Number.isFinite(minV) || !Number.isFinite(maxV)) return
  if (minV === maxV) {
    minV -= 1
    maxV += 1
  }

  const pad = 6
  const scale = (height - 1 - 2 * pad) / (maxV - minV)
  const yOf = (value: number): number => pad + (maxV - value) * scale

  if (Number.isFinite(opts?.baseline)) {
    const y = yOf(num(opts?.baseline, 0))
    ctx.strokeStyle = 'rgba(148, 163, 184, 0.45)'
    ctx.beginPath()
    ctx.moveTo(0, y + 0.5)
    ctx.lineTo(bins, y + 0.5)
    ctx.stroke()
  }

  ctx.strokeStyle = String(opts?.stroke || 'rgba(52, 211, 153, 0.95)')
  ctx.lineWidth = 1
  for (let x = 0; x < bins; x += 1) {
    const a = Number(mm[2 * x])
    const b = Number(mm[2 * x + 1])
    if (!Number.isFinite(a) || !Number.isFinite(b)) continue
    ctx.beginPath()
    ctx.moveTo(x + 0.5, yOf(a))
    ctx.lineTo(x + 0.5, yOf(b))
    ctx.stroke()
  }
}

function normalizeChartTimeSec(raw: unknown): number {
  const t = num(raw, NaN)
  if (!Number.isFinite(t)) return NaN
  const abs = Math.abs(t)
  if (abs >= 1e17) return Math.floor(t / 1e9)
  if (abs >= 1e14) return Math.floor(t / 1e6)
  if (abs >= 1e11) return Math.floor(t / 1e3)
  return Math.floor(t)
}

async function loadDrillTvModule(): Promise<any | null> {
  if (!drillTvModulePromise) {
    drillTvModulePromise = import('lightweight-charts').then((m) => m as any).catch(() => null)
  }
  return drillTvModulePromise
}

function disposeDrillTvChartDom(): void {
  cancelDrillTvRenderFrame()
  drillTvRuntime.dispose((chart) => {
    if (!chart || typeof chart.remove !== 'function') return
    chart.remove()
  })
  drillTvCandleSeries = null
  drillTvFastSeries = null
  drillTvSlowSeries = null
  drillTvHalftrendSeries = null
  drillTvMaczSeries = null
}

function cancelDrillTvRenderFrame(): void {
  if (drillTvRenderFrame && typeof window !== 'undefined' && typeof window.cancelAnimationFrame === 'function') {
    window.cancelAnimationFrame(drillTvRenderFrame)
  }
  drillTvRenderFrame = 0
  drillTvPendingPayload = null
}

function setSeriesData(series: any, data: unknown[]): boolean {
  if (!series || typeof series.setData !== 'function') return true
  try {
    series.setData(data)
    return true
  } catch {
    return false
  }
}

function clearSeries(series: any): void {
  setSeriesData(series, [])
}

function flushDrillTvRenderFrame(): void {
  drillTvRenderFrame = 0
  const payload = drillTvPendingPayload
  drillTvPendingPayload = null
  if (!payload) return
  if (payload.seq !== drillSeq) return
  const { ctx } = payload
  if (!setSeriesData(ctx.candle, payload.candleData)) {
    clearDrillTvChartDom('Unable to render drill chart.')
    return
  }
  setSeriesData(ctx.fast, payload.fastData)
  setSeriesData(ctx.slow, payload.slowData)
  setSeriesData(ctx.halftrend, payload.halftrendData)
  setSeriesData(ctx.macz, payload.maczData)

  try {
    const ts = ctx.chart.timeScale && ctx.chart.timeScale()
    if (ts && typeof ts.fitContent === 'function') ts.fitContent()
  } catch {}

  const meta = byIds<HTMLElement>(['drillTvChartMetaVue', 'drillTvChartMeta'])
  if (meta) {
    meta.textContent = `candles=${payload.candleData.length} fast=${payload.fastData.length} slow=${payload.slowData.length} trades=${payload.trades}`
  }
}

function scheduleDrillTvRenderFrame(payload: DrillTvRenderPayload): void {
  drillTvPendingPayload = payload
  if (drillTvRenderFrame) return
  if (typeof window === 'undefined' || typeof window.requestAnimationFrame !== 'function') {
    flushDrillTvRenderFrame()
    return
  }
  drillTvRenderFrame = window.requestAnimationFrame(() => {
    flushDrillTvRenderFrame()
  })
}

function clearDrillTvChartDom(msg = 'Hover for OHLC/MA values; click a marker to focus a trade.'): void {
  cancelDrillTvRenderFrame()
  const meta = byIds<HTMLElement>(['drillTvChartMetaVue', 'drillTvChartMeta'])
  if (meta) meta.textContent = msg
  clearSeries(drillTvCandleSeries)
  clearSeries(drillTvFastSeries)
  clearSeries(drillTvSlowSeries)
  clearSeries(drillTvHalftrendSeries)
  clearSeries(drillTvMaczSeries)
}

async function ensureDrillTvChartDom(): Promise<DrillTvChartContext | null> {
  const host = byIds<HTMLElement>(['drillTvChartVue', 'drillTvChart'])
  if (!host) return null
  const existingChart = drillTvRuntime.chartRef.value
  const existingHost = drillTvRuntime.hostRef.value
  if (existingChart && existingHost && existingHost !== host) {
    disposeDrillTvChartDom()
  }
  if (drillTvRuntime.chartRef.value && drillTvCandleSeries) {
    return {
      chart: drillTvRuntime.chartRef.value,
      candle: drillTvCandleSeries,
      fast: drillTvFastSeries,
      slow: drillTvSlowSeries,
      halftrend: drillTvHalftrendSeries,
      macz: drillTvMaczSeries
    }
  }
  const chartModule = await loadDrillTvModule()
  if (!chartModule || typeof chartModule.createChart !== 'function') return null
  const width = Math.max(240, Math.floor(host.clientWidth || 820))
  const height = Math.max(220, Math.floor(host.clientHeight || 360))
  let chart: any
  try {
    chart = chartModule.createChart(host, {
      width,
      height,
      layout: {
        textColor: 'rgba(203, 213, 225, 0.92)',
        background: { type: 'solid', color: 'rgba(2, 6, 23, 0)' }
      },
      grid: {
        vertLines: { color: 'rgba(51, 65, 85, 0.35)' },
        horzLines: { color: 'rgba(51, 65, 85, 0.35)' }
      },
      rightPriceScale: {
        borderColor: 'rgba(100, 116, 139, 0.45)'
      },
      timeScale: {
        borderColor: 'rgba(100, 116, 139, 0.45)',
        fixLeftEdge: false,
        fixRightEdge: false
      },
      crosshair: {
        vertLine: { color: 'rgba(148, 163, 184, 0.45)' },
        horzLine: { color: 'rgba(148, 163, 184, 0.45)' }
      }
    })
  } catch {
    return null
  }
  const chartAny = chart as any
  const candleOpts = {
    upColor: 'rgba(22, 163, 74, 0.9)',
    downColor: 'rgba(225, 29, 72, 0.9)',
    wickUpColor: 'rgba(22, 163, 74, 0.9)',
    wickDownColor: 'rgba(225, 29, 72, 0.9)',
    borderVisible: true,
    borderUpColor: 'rgba(22, 163, 74, 1)',
    borderDownColor: 'rgba(225, 29, 72, 1)'
  }
  let candle = null
  if (typeof chartAny.addSeries === 'function' && chartModule.CandlestickSeries) {
    try {
      candle = chartAny.addSeries(chartModule.CandlestickSeries, candleOpts)
    } catch {
      candle = null
    }
  }
  if (!candle && typeof chartAny.addCandlestickSeries === 'function') {
    try {
      candle = chartAny.addCandlestickSeries(candleOpts)
    } catch {
      candle = null
    }
  }
  if (!candle) {
    if (typeof chartAny.remove === 'function') chartAny.remove()
    return null
  }

  const fastOpts = {
    color: 'rgba(14, 165, 233, 0.95)',
    lineWidth: 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false
  }
  const slowOpts = {
    color: 'rgba(245, 158, 11, 0.95)',
    lineWidth: 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false
  }
  const halftrendOpts = {
    color: 'rgba(125, 211, 252, 0.95)',
    lineWidth: 1,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
    lineStyle: 2
  }
  const maczOpts = {
    color: 'rgba(148, 163, 184, 0.6)',
    priceLineVisible: false,
    lastValueVisible: false,
    base: 0
  }

  let fast = null
  let slow = null
  let halftrend = null
  let macz = null
  if (typeof chartAny.addSeries === 'function' && chartModule.LineSeries) {
    try {
      fast = chartAny.addSeries(chartModule.LineSeries, fastOpts)
      slow = chartAny.addSeries(chartModule.LineSeries, slowOpts)
      halftrend = chartAny.addSeries(chartModule.LineSeries, halftrendOpts)
    } catch {
      fast = null
      slow = null
      halftrend = null
    }
  }
  if ((!fast || !slow) && typeof chartAny.addLineSeries === 'function') {
    try {
      fast = fast || chartAny.addLineSeries(fastOpts)
      slow = slow || chartAny.addLineSeries(slowOpts)
      halftrend = halftrend || chartAny.addLineSeries(halftrendOpts)
    } catch {
      // optional series in older APIs
    }
  }
  if (typeof chartAny.addSeries === 'function' && chartModule.HistogramSeries) {
    try {
      macz = chartAny.addSeries(chartModule.HistogramSeries, maczOpts)
    } catch {
      macz = null
    }
  }

  drillTvCandleSeries = markRawEntity(candle)
  drillTvFastSeries = markRawEntity(fast)
  drillTvSlowSeries = markRawEntity(slow)
  drillTvHalftrendSeries = markRawEntity(halftrend)
  drillTvMaczSeries = markRawEntity(macz)
  drillTvRuntime.setHost(host)
  drillTvRuntime.setChart(chartAny)
  drillTvRuntime.observeHostResize((rect) => {
    const chartRef = drillTvRuntime.chartRef.value
    if (!chartRef || typeof chartRef.applyOptions !== 'function') return
    if (!rect.width || !rect.height) return
    try {
      chartRef.applyOptions({
        width: Math.max(240, Math.floor(rect.width)),
        height: Math.max(220, Math.floor(rect.height))
      })
    } catch {}
  })

  return {
    chart: drillTvRuntime.chartRef.value,
    candle: drillTvCandleSeries,
    fast: drillTvFastSeries,
    slow: drillTvSlowSeries,
    halftrend: drillTvHalftrendSeries,
    macz: drillTvMaczSeries
  }
}

function normalizeIndicatorBarsForChart(raw: unknown): Array<{ time: number; fast: number | null; slow: number | null }> {
  const rows = Array.isArray(raw) ? raw : []
  const out: Array<{ time: number; fast: number | null; slow: number | null }> = []
  for (const rowRaw of rows) {
    const row = asRecord(rowRaw)
    const time = normalizeChartTimeSec(row.time ?? row.timestamp ?? row.ts ?? row.t)
    if (!Number.isFinite(time)) continue
    const fastRaw = num(row.fast, NaN)
    const slowRaw = num(row.slow, NaN)
    const fast = Number.isFinite(fastRaw) ? fastRaw : null
    const slow = Number.isFinite(slowRaw) ? slowRaw : null
    if (fast == null && slow == null) continue
    out.push({ time, fast, slow })
  }
  out.sort((a, b) => a.time - b.time)
  return out
}

function normalizeCompositeIndicatorBarsForChart(raw: unknown): Array<{ time: number; halftrend: number | null; macz: number | null }> {
  const rows = Array.isArray(raw) ? raw : []
  const out: Array<{
    time: number
    halftrend: number | null
    macz: number | null
  }> = []
  for (const rowRaw of rows) {
    const row = asRecord(rowRaw)
    const time = normalizeChartTimeSec(row.time ?? row.timestamp ?? row.ts ?? row.t)
    if (!Number.isFinite(time)) continue
    const halftrendRaw = num(row.halftrend, NaN)
    const maczRaw = num(row.macz_hist ?? row.macz, NaN)
    const halftrend = Number.isFinite(halftrendRaw) ? halftrendRaw : null
    const macz = Number.isFinite(maczRaw) ? maczRaw : null
    if (halftrend == null && macz == null) continue
    out.push({ time, halftrend, macz })
  }
  out.sort((a, b) => a.time - b.time)
  return out
}

async function renderDrillTvChartDom(curvesRaw: unknown, barsRaw: DrillPriceBar[] | null, seq: number): Promise<void> {
  if (seq !== drillSeq) return
  const bars = Array.isArray(barsRaw) ? barsRaw.filter((b) => Number.isFinite(b.time) && Number.isFinite(b.open) && Number.isFinite(b.high) && Number.isFinite(b.low) && Number.isFinite(b.close)) : []
  if (!bars.length) {
    clearDrillTvChartDom('No valid OHLC bars available for chart rendering.')
    return
  }
  const ctx = await ensureDrillTvChartDom()
  if (!ctx || seq !== drillSeq) return
  const candleData = bars.map((b) => ({
    time: b.time,
    open: b.open,
    high: b.high,
    low: b.low,
    close: b.close
  }))
  if (!candleData.length) {
    clearDrillTvChartDom('No candle points generated from loaded bars.')
    return
  }

  const curves = asRecord(curvesRaw)
  const indicatorBars = normalizeIndicatorBarsForChart(curves.indicator_bars)
  const fastData: Array<{ time: number; value: number }> = []
  const slowData: Array<{ time: number; value: number }> = []
  for (const row of indicatorBars) {
    if (Number.isFinite(row.fast ?? NaN)) fastData.push({ time: row.time, value: Number(row.fast) })
    if (Number.isFinite(row.slow ?? NaN)) slowData.push({ time: row.time, value: Number(row.slow) })
  }

  const compositeBars = normalizeCompositeIndicatorBarsForChart(curves.composite_indicator_bars)
  const halftrendData: Array<{ time: number; value: number }> = []
  const maczData: Array<{ time: number; value: number; color?: string }> = []
  for (const row of compositeBars) {
    if (Number.isFinite(row.halftrend ?? NaN)) {
      halftrendData.push({ time: row.time, value: Number(row.halftrend) })
    }
    if (Number.isFinite(row.macz ?? NaN)) {
      const value = Number(row.macz)
      maczData.push({
        time: row.time,
        value,
        color: value >= 0 ? 'rgba(34, 197, 94, 0.75)' : 'rgba(244, 63, 94, 0.75)'
      })
    }
  }
  scheduleDrillTvRenderFrame({
    seq,
    ctx,
    candleData,
    fastData,
    slowData,
    halftrendData,
    maczData,
    trades: Array.isArray(curves.trades) ? curves.trades.length : 0
  })
}

function normalizeDrillPriceBars(raw: unknown): DrillPriceBar[] {
  const rows = Array.isArray(raw) ? raw : []
  const out: DrillPriceBar[] = []
  for (const rowRaw of rows) {
    const row = asRecord(rowRaw)
    const time = normalizeChartTimeSec(row.time ?? row.timestamp ?? row.ts ?? row.t)
    if (!Number.isFinite(time)) continue
    const open = num(row.open ?? row.o, NaN)
    const highRaw = num(row.high ?? row.h, NaN)
    const lowRaw = num(row.low ?? row.l, NaN)
    const close = num(row.close ?? row.c, NaN)
    if (!Number.isFinite(open) || !Number.isFinite(highRaw) || !Number.isFinite(lowRaw) || !Number.isFinite(close)) continue
    const high = Math.max(highRaw, open, close)
    const low = Math.min(lowRaw, open, close)
    const fromIndex = Math.max(0, Math.floor(num(row.from_index ?? row.fromIndex ?? row.from, 0)))
    const toIndex = Math.max(fromIndex, Math.floor(num(row.to_index ?? row.toIndex ?? row.to, fromIndex)))
    out.push({
      time,
      open,
      high,
      low,
      close,
      volume: num(row.volume ?? row.v, 0),
      fromIndex,
      toIndex
    })
  }
  if (out.length <= 1) return out
  out.sort((a, b) => a.time - b.time)
  const deduped: DrillPriceBar[] = []
  for (const row of out) {
    const last = deduped[deduped.length - 1]
    if (last && row.time === last.time) deduped[deduped.length - 1] = row
    else deduped.push(row)
  }
  return deduped
}

async function fetchDrillPriceBars(dataId: string): Promise<DrillPriceBar[]> {
  const id = text(dataId)
  if (!id) return []
  const cached = drillPriceBarsCache.get(id)
  if (cached) return cached
  let raw: unknown
  try {
    raw = await invokeCommand('get_price_bars', {
      dataId: id,
      maxPoints: 8000
    })
  } catch (firstError) {
    try {
      raw = await invokeCommand('get_price_bars', {
        data_id: id,
        max_points: 8000
      })
    } catch {
      throw firstError
    }
  }
  const bars = normalizeDrillPriceBars(raw)
  drillPriceBarsCache.set(id, bars)
  return bars
}

function mapTradeIndexToChartTime(indexRaw: unknown, barsRaw: DrillPriceBar[] | null | undefined): number | null {
  const bars = Array.isArray(barsRaw) ? barsRaw : []
  if (!bars.length) return null
  const x = Math.max(0, Math.floor(num(indexRaw, 0)))
  let monotonic = true
  let prevFrom = Number.NEGATIVE_INFINITY
  for (const bar of bars) {
    if (bar.fromIndex < prevFrom || bar.toIndex < bar.fromIndex) {
      monotonic = false
      break
    }
    prevFrom = bar.fromIndex
  }
  if (!monotonic) {
    let bestTime = bars[0]?.time ?? null
    let bestDist = Number.POSITIVE_INFINITY
    for (const bar of bars) {
      if (x >= bar.fromIndex && x <= bar.toIndex) return bar.time
      const d = x < bar.fromIndex ? bar.fromIndex - x : x - bar.toIndex
      if (d < bestDist) {
        bestDist = d
        bestTime = bar.time
      }
    }
    return bestTime
  }

  let lo = 0
  let hi = bars.length - 1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    const bar = bars[mid]
    if (!bar) break
    if (x < bar.fromIndex) hi = mid - 1
    else if (x > bar.toIndex) lo = mid + 1
    else return bar.time
  }
  if (hi < 0) return bars[0]?.time ?? null
  if (lo >= bars.length) return bars[bars.length - 1]?.time ?? null
  const left = bars[hi]
  const right = bars[lo]
  if (!left || !right) return null
  return Math.abs(x - left.toIndex) <= Math.abs(right.fromIndex - x) ? left.time : right.time
}

function formatUtcTimeFromSec(secRaw: unknown): string {
  const sec = num(secRaw, NaN)
  if (!Number.isFinite(sec)) return '-'
  const d = new Date(sec * 1000)
  const pad = (n: number): string => String(Math.max(0, Math.floor(n))).padStart(2, '0')
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())} UTC`
}

function formatUtcTimeShortFromSec(secRaw: unknown): string {
  const sec = num(secRaw, NaN)
  if (!Number.isFinite(sec)) return '-'
  const d = new Date(sec * 1000)
  const pad = (n: number): string => String(Math.max(0, Math.floor(n))).padStart(2, '0')
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`
}

function computeTradeTableStart(totalRaw: unknown, selectedRaw: unknown, maxRowsRaw: unknown): number {
  const total = Math.max(0, Math.floor(num(totalRaw, 0)))
  const maxRows = Math.max(1, Math.floor(num(maxRowsRaw, 200)))
  if (!Number.isFinite(total) || total <= maxRows) return 0
  const maxStart = Math.max(0, total - maxRows)
  const selected = Math.floor(num(selectedRaw, NaN))
  if (Number.isFinite(selected) && selected >= 0 && selected < total) {
    const half = Math.floor(maxRows / 2)
    return Math.max(0, Math.min(maxStart, selected - half))
  }
  return maxStart
}

function renderTradesDom(curvesRaw: unknown, barsRaw: DrillPriceBar[] | null): boolean {
  const wrap = byIds<HTMLElement>(['tradeStatsWrapVue', 'tradeStatsWrap'])
  const statsEl = byIds<HTMLElement>(['tradeStatsVue', 'tradeStats'])
  const body = byIds<HTMLElement>(['tradeTableBodyVue', 'tradeTableBody'])
  if (!wrap || !statsEl || !body) return false
  const curves = asRecord(curvesRaw)
  const trades = Array.isArray(curves.trades) ? curves.trades.map((t) => asRecord(t)) : []
  const stats = asRecord(curves.trade_stats)

  if (!trades.length && !Object.keys(stats).length) {
    wrap.style.display = 'none'
    statsEl.textContent = '-'
    body.innerHTML = ''
    return true
  }
  wrap.style.display = 'block'
  const maxRows = 200
  const total = trades.length
  let start = Math.max(0, Math.floor(drillTradesRangeStart))
  if (total <= maxRows) {
    start = 0
  } else {
    const maxStart = Math.max(0, total - maxRows)
    start = Math.max(0, Math.min(maxStart, start))
    if (drillSelectedTradeIdx >= 0 && drillSelectedTradeIdx < total) {
      const endExcl = start + maxRows
      if (drillSelectedTradeIdx < start || drillSelectedTradeIdx >= endExcl) {
        start = computeTradeTableStart(total, drillSelectedTradeIdx, maxRows)
      }
    } else if (start === 0) {
      start = maxStart
    }
  }
  drillTradesRangeStart = start

  if (Object.keys(stats).length) {
    const closed = Math.floor(num(stats.closed_trades, 0))
    const open = Math.floor(num(stats.open_trades, 0))
    const wins = Math.floor(num(stats.wins, 0))
    const losses = Math.floor(num(stats.losses, 0))
    const winRate = num(stats.win_rate, NaN)
    const avgWin = num(stats.avg_win, NaN)
    const avgLoss = num(stats.avg_loss, NaN)
    const pf = num(stats.profit_factor, NaN)
    const exp = num(stats.expectancy, NaN)
    const pfStr = Number.isFinite(pf) ? (pf === Infinity ? 'inf' : pf.toFixed(3)) : '-'
    const wrStr = Number.isFinite(winRate) ? `${(winRate * 100).toFixed(1)}%` : '-'
    const avgWinStr = Number.isFinite(avgWin) ? `${(avgWin * 100).toFixed(2)}%` : '-'
    const avgLossStr = Number.isFinite(avgLoss) ? `${(avgLoss * 100).toFixed(2)}%` : '-'
    const expStr = Number.isFinite(exp) ? `${(exp * 100).toFixed(2)}%` : '-'
    statsEl.textContent = `closed=${closed} open=${open} wins=${wins} losses=${losses} win_rate=${wrStr} avg_win=${avgWinStr} avg_loss=${avgLossStr} pf=${pfStr} expectancy=${expStr}`
  } else {
    statsEl.textContent = `trades=${trades.length}`
  }

  const shown = total ? trades.slice(start, Math.min(total, start + maxRows)) : []
  if (total) {
    statsEl.textContent = `${statsEl.textContent} | view=${start + 1}..${start + shown.length}/${total}`
  }

  body.innerHTML = ''
  for (let i = 0; i < shown.length; i += 1) {
    const trade = shown[i] || {}
    const tradeIdx = start + i
    const idxLabel = tradeIdx + 1
    const dirVal = num(trade.direction, 0)
    const dir = dirVal > 0 ? 'Long' : dirVal < 0 ? 'Short' : 'Flat'
    const entry = Math.floor(num(trade.entry_t, 0))
    const bars = Math.floor(num(trade.bars, 0))
    const pnl = num(trade.pnl, NaN)
    const open = !!trade.open
    const exitRaw = num(trade.exit_t, NaN)
    const exit = open && (!Number.isFinite(exitRaw) || exitRaw <= 0) ? NaN : exitRaw

    const tr = document.createElement('tr')
    tr.dataset.tradeIdx = String(tradeIdx)
    if (tradeIdx === drillSelectedTradeIdx) tr.classList.add('selected')
    const entryTime = mapTradeIndexToChartTime(entry, barsRaw)
    const exitTime = Number.isFinite(exit) ? mapTradeIndexToChartTime(exit, barsRaw) : null
    const cells = [
      { text: String(idxLabel), cls: 'left mono', title: '' },
      { text: dir, cls: 'left mono', title: '' },
      {
        text: String(entry),
        cls: 'left mono',
        title: Number.isFinite(entryTime ?? NaN) ? formatUtcTimeFromSec(entryTime) : ''
      },
      {
        text: Number.isFinite(entryTime ?? NaN) ? formatUtcTimeShortFromSec(entryTime) : '-',
        cls: 'left mono',
        title: Number.isFinite(entryTime ?? NaN) ? formatUtcTimeFromSec(entryTime) : ''
      },
      {
        text: Number.isFinite(exit) ? String(Math.floor(exit)) : '-',
        cls: 'left mono',
        title: Number.isFinite(exitTime ?? NaN) ? formatUtcTimeFromSec(exitTime) : ''
      },
      {
        text: Number.isFinite(exitTime ?? NaN) ? formatUtcTimeShortFromSec(exitTime) : '-',
        cls: 'left mono',
        title: Number.isFinite(exitTime ?? NaN) ? formatUtcTimeFromSec(exitTime) : ''
      },
      { text: String(bars), cls: 'mono', title: '' },
      {
        text: Number.isFinite(pnl) ? `${(pnl * 100).toFixed(2)}%` : '-',
        cls: 'mono',
        title: ''
      },
      { text: open ? 'open' : 'closed', cls: 'left mono', title: '' }
    ]
    for (const cell of cells) {
      const td = document.createElement('td')
      td.className = cell.cls
      td.textContent = cell.text
      if (cell.title) td.title = cell.title
      tr.appendChild(td)
    }
    tr.addEventListener('click', () => {
      const rows = body.querySelectorAll('tr.selected')
      for (const rowEl of rows) rowEl.classList.remove('selected')
      tr.classList.add('selected')
      drillSelectedTradeIdx = tradeIdx
    })
    body.appendChild(tr)
  }

  return true
}

function renderDrillDomFallback(curvesRaw: unknown, paramsRaw: Record<string, unknown>, metricsRaw: Record<string, unknown>, row: TopRowModel, bars: DrillPriceBar[] | null): boolean {
  const wrap = drillWrap()
  const hint = byIds<HTMLElement>(['drillEmptyHintVue', 'drillEmptyHint'])
  const meta = byIds<HTMLElement>(['drillMetaVue', 'drillMeta'])
  const eqCanvas = byIds<HTMLCanvasElement>(['equityCanvasVue', 'equityCanvas'])
  const ddCanvas = byIds<HTMLCanvasElement>(['drawdownCanvasVue', 'drawdownCanvas'])
  if (!wrap || !eqCanvas || !ddCanvas) return false

  const curves = asRecord(curvesRaw)
  const eq = asRecord(curves.equity)
  const dd = asRecord(curves.drawdown)
  const bins = Math.floor(num(eq.bins, 0))
  if (bins <= 0) {
    wrap.style.display = 'none'
    if (hint) hint.style.display = 'block'
    clearDrillTvChartDom()
    return true
  }

  wrap.style.display = 'block'
  if (hint) hint.style.display = 'none'
  if (meta) {
    const metricPart = formatDrillMetricsSummary(metricsRaw)
    const moPart = formatDrillMoSummary(row.mo)
    const parts = [`${text(paramsRaw.fast_ma_type) || '?'}(${num(paramsRaw.fast_len, 0) || '?'}) vs ${text(paramsRaw.slow_ma_type) || '?'}(${num(paramsRaw.slow_len, 0) || '?'})`]
    if (metricPart) parts.push(metricPart)
    if (moPart) parts.push(moPart)
    parts.push(`bins=${bins} start_t=${num(eq.start_t, 0)} end_t=${num(eq.end_t, 0)} (min/max per bin)`)
    meta.textContent = parts.join(' | ')
  }

  drawMinMaxChartDom(eqCanvas, eq.minmax, {
    stroke: 'rgba(52, 211, 153, 0.95)',
    baseline: 1.0,
    height: 160
  })
  drawMinMaxChartDom(ddCanvas, dd.minmax, {
    stroke: 'rgba(251, 113, 133, 0.95)',
    baseline: 0.0,
    height: 160
  })
  renderTradesDom(curves, bars)
  return true
}

export async function clearTopFilters(): Promise<void> {
  const store = readAnalysisStore()
  if (!store) return
  store.filterMinPnl = ''
  store.filterMinSharpe = ''
  store.filterMaxDd = ''
  store.filterMinTrades = ''
  store.filterMinExposure = ''
  store.canClearTopFilters = false
  if (!refreshTopTableFromWorkflowRun()) markAnalysisVue(store)
  setStatus(store, 'Top filters cleared.')
}

export async function sortTopTable(key: string): Promise<void> {
  const sortKey = text(key)
  const store = readAnalysisStore()
  if (!store || !sortKey) return
  const current = readSortState(store)
  const nextDir: SortDir = current.key === sortKey ? (current.dir === 'asc' ? 'desc' : 'asc') : defaultSortDir(sortKey)
  const columns = readTopColumns(store)
  if (columns.length > 0) {
    for (const col of columns) col.sort = text(col?.key) === sortKey ? nextDir : ''
    store.topColumns = columns
  } else {
    store.topColumns = [{ key: sortKey, label: sortKey, sort: nextDir, alignLeft: false }]
  }
  if (refreshTopTableFromWorkflowRun()) {
    setStatus(store, `Sorted top rows by ${sortKey} (${nextDir}).`)
  }
}

export async function selectTopRow(index: number): Promise<void> {
  const idx = parseIndex(index)
  const store = readAnalysisStore()
  if (!store) return
  const rows = readTopRows(store)
  if (!rows.length) return
  const bounded = Math.max(0, Math.min(idx, rows.length - 1))
  for (let i = 0; i < rows.length; i += 1) {
    const row = rows[i]
    if (!row) continue
    row.selected = i === bounded
  }
  store.topTableRows = rows
  store.topSelectionGuardrail = `Selected row ${bounded + 1} of ${rows.length}`
  setStatus(store, `Selected top row ${bounded + 1}.`)
  const model = resolveTopRowModelAtIndex(store, bounded)
  if (model) await runVueDrillPipeline(bounded, model)
}

export async function copyBest(): Promise<void> {
  const store = readAnalysisStore()
  const summary = text(store?.bestSummary)
  if (summary && summary !== '-' && summary !== '—') {
    const ok = await copyText(summary)
    setStatus(store, ok ? 'Copied best summary.' : 'Copy failed.')
    return
  }
  setStatus(store, 'Nothing to copy yet.')
}

export async function exportTopCsv(): Promise<void> {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  const req = asRecord(run?.req)
  const res = asRecord(run?.res)
  if (!Object.keys(res).length) {
    setStatus(store, 'No result to export yet.')
    return
  }
  if (requestedTopK(req) === 0 && !Array.isArray(res.pareto)) {
    setStatus(store, 'Top CSV unavailable for this run because Top K was set to 0.')
    return
  }
  const objective = objectiveOf(req)
  const sort = store ? readSortState(store) : { key: 'rank', dir: 'asc' as const }
  const rows = applyTopFilters(sortRows(collectTopRows(res, { allowBestFallback: false }), sort.key, sort.dir, objective), store ? readFilters(store) : {})
  const seed = startEq(req)
  const lines = ['rank,fast_ma,fast_len,slow_ma,slow_len,pnl,pnl_pct,final_equity,sharpe,max_dd,trades,exposure,net_exposure,score,params_json']
  for (const row of rows) {
    const p = row.params || {}
    const m = row.metrics || {}
    const pnl = num(m.pnl, 0)
    const score = scoreOf(m, objective)
    const paramsJson = (() => {
      try {
        return JSON.stringify(p)
      } catch {
        return '{}'
      }
    })()
    lines.push([row.rank, csvEscape(p.fast_ma_type || ''), num(p.fast_len, 0), csvEscape(p.slow_ma_type || ''), num(p.slow_len, 0), pnl, pnl * 100, num(finalEq(pnl, seed), 0), num(m.sharpe, 0), num(m.max_dd, 0), num(m.trades, 0), num(m.exposure, 0), num(m.net_exposure, 0), num(score, 0), csvEscape(paramsJson)].join(','))
  }
  if (downloadText(`vectorgrid_top_${sanitizeTs(run?.ts)}.csv`, lines.join('\n'), 'text/csv')) {
    setStatus(store, 'Exported top CSV.')
    return
  }
  setStatus(store, 'Failed to export top CSV.')
}

export async function exportTopJson(): Promise<void> {
  const store = readAnalysisStore()
  const run = readWorkflowRun()
  const res = run?.res
  if (!res || typeof res !== 'object') {
    setStatus(store, 'No result to export yet.')
    return
  }
  if (downloadText(`vectorgrid_run_${sanitizeTs(run?.ts)}.json`, JSON.stringify(res, null, 2), 'application/json')) {
    setStatus(store, 'Exported run JSON.')
    return
  }
  setStatus(store, 'Failed to export JSON.')
}

export async function refreshCompareSummary(): Promise<void> {
  const store = readAnalysisStore()
  const history = readHistoryStore()
  if (!store) return
  const selected = text(history?.compareSelectedValue)
  if (!selected) {
    store.compareSummary = '-'
    markAnalysisVue(store)
    return
  }

  const current = readWorkflowRun()
  const currentReq = asRecord(current?.req)
  const currentRes = asRecord(current?.res)
  if (!current || !Object.keys(currentReq).length || !Object.keys(currentRes).length) {
    store.compareSummary = 'Select a current run to compare.'
    markAnalysisVue(store)
    return
  }

  compareSeq = (compareSeq + 1) >>> 0
  const seq = compareSeq
  store.compareSummary = 'Loading compare…'
  markAnalysisVue(store)

  try {
    const baseline = await resolveBaselineSelection(selected, current, history)
    if (seq !== compareSeq) return
    if (!baseline) {
      store.compareSummary = 'Baseline run not found (refresh saved runs or pick another baseline).'
      markAnalysisVue(store)
      return
    }
    store.compareSummary = buildCompareSummary(current, baseline)
    markAnalysisVue(store)
  } catch (error) {
    if (seq !== compareSeq) return
    store.compareSummary = `Compare failed: ${String(error)}`
    markAnalysisVue(store)
  }
}

export async function setBaseline(): Promise<void> {
  const store = readAnalysisStore()
  const history = readHistoryStore()
  const run = readWorkflowRun()
  const ts = text(run?.ts)
  const artifact = text(run?.artifactPath ?? run?.artifact_path)
  if (ts) {
    const value = artifact ? `disk:${artifact}` : `hist:${ts}`
    if (history) {
      history.compareSelectedValue = value
      markHistoryVue(history)
    }
    if (store) {
      store.compareSummary = `Baseline selected: ${value}`
      setStatus(store, 'Set baseline = current.')
      await refreshCompareSummary()
      return
    }
  }
  setStatus(store, 'Run an optimization or select a run from history first.')
}

export async function clearBaseline(): Promise<void> {
  const store = readAnalysisStore()
  const history = readHistoryStore()
  if (history) {
    history.compareSelectedValue = ''
    markHistoryVue(history)
  }
  if (store) {
    store.compareSummary = '-'
    setStatus(store, 'Cleared baseline.')
    return
  }
}

export async function heatmapExportPng(): Promise<void> {
  if (typeof document !== 'undefined') {
    const base = byIds<HTMLCanvasElement>(['heatmapCanvasVue', 'heatmapCanvas'])
    const overlay = byIds<HTMLCanvasElement>(['heatmapOverlayVue', 'heatmapOverlay'])
    if (base && overlay && base.width > 0 && base.height > 0) {
      const tmp = document.createElement('canvas')
      tmp.width = base.width
      tmp.height = base.height
      const ctx = tmp.getContext('2d')
      if (ctx) {
        ctx.drawImage(base, 0, 0)
        ctx.drawImage(overlay, 0, 0)
        const run = readWorkflowRun()
        const req = asRecord(run?.req)
        const file = `heatmap_${String(req.objective || 'heatmap').toLowerCase()}_${sanitizeTs(run?.ts)}.png`
        const url = tmp.toDataURL('image/png')
        if (typeof document !== 'undefined') {
          const a = document.createElement('a')
          a.href = url
          a.download = file
          a.style.display = 'none'
          document.body?.appendChild(a)
          a.click()
          a.remove()
          setStatus(readAnalysisStore(), 'Exported heatmap PNG.')
          return
        }
      }
    }
  }
  setStatus(readAnalysisStore(), 'Failed to export heatmap PNG.')
}

export async function rerenderDrill(): Promise<void> {
  const store = readAnalysisStore()
  if (store) {
    const rows = readTopRows(store)
    if (rows.length) {
      const selected = rows.findIndex((row) => !!row?.selected)
      await selectTopRow(selected >= 0 ? selected : 0)
    }
  }
}

export async function drillClearTradeSelection(): Promise<void> {
  const pinia = getActivePinia()
  if (pinia && useDataStore(pinia).polygonOpen) {
    return
  }
  if (clearDrillTradeSelectionDom()) {
    setStatus(readAnalysisStore(), 'Cleared trade selection.')
    return
  }
  setStatus(readAnalysisStore(), 'No selected trade.')
}

export async function drillPrevTrade(): Promise<void> {
  const selected = navigateDrillTradeDom(false)
  if (selected) {
    setStatus(readAnalysisStore(), `Selected trade ${selected}.`)
    return
  }
  setStatus(readAnalysisStore(), 'No previous trade.')
}

export async function drillNextTrade(): Promise<void> {
  const selected = navigateDrillTradeDom(true)
  if (selected) {
    setStatus(readAnalysisStore(), `Selected trade ${selected}.`)
    return
  }
  setStatus(readAnalysisStore(), 'No next trade.')
}
