import { requestCancelCurrentRun } from './runControlService'
import { invokeCommand } from '../tauri/invoke'
import { buildDoubleMaRequestFromVueStores, type BuildDoubleMaRequestOptions } from './optimizeService'
import { applyWorkflowArtifactsPayload } from '../stores/workflowArtifactsStore'
import { refreshTopTableFromWorkflowRun } from './analysisService'

interface DistributedWorkerStatusResponse {
  running?: unknown
  bind_addr?: unknown
  bindAddr?: unknown
}

interface PolygonCacheDatasetResponse {
  csv_path?: unknown
}

interface PolygonCacheInfoResponse {
  datasets?: unknown
}

interface DistributedAssetOutcomeResponse {
  csv_path?: unknown
  result?: unknown
  error?: unknown
}

interface DistributedAssetsRunResponse {
  cancelled?: unknown
  total?: unknown
  completed?: unknown
  successes?: unknown
  failures?: unknown
  results?: unknown
}

interface DistributedTiledRunResponse {
  cancelled?: unknown
  tiles_total?: unknown
  tiles_completed?: unknown
  errors?: unknown
  result?: unknown
}

export interface DistributedWorkerStatusSummary {
  running: boolean
  bindAddr: string
  text: string
}

export interface DistributedPolygonFillSummary {
  paths: string[]
  count: number
}

export interface DistributedCurrentCsvSummary {
  path: string
}

export interface DistributedRunAssetsOptions {
  workerUrlsText: string
  authToken: string
  assetsCsvText: string
  rewriteFrom: string
  rewriteTo: string
  requestOverrides?: BuildDoubleMaRequestOptions
}

export interface DistributedRunTilesOptions {
  workerUrlsText: string
  authToken: string
  tileCsvPath: string
  rewriteFrom: string
  rewriteTo: string
  tilesPerWorker: string | number
  requestOverrides?: BuildDoubleMaRequestOptions
}

export interface DistributedRunSummary {
  outputPreview: string
  statusLine: string
  cancelled: boolean
}

function normalizeText(value: unknown): string {
  return String(value || '').trim()
}

function normalizeWorkerStatus(raw: unknown): DistributedWorkerStatusSummary {
  const status = (raw && typeof raw === 'object' ? raw : {}) as DistributedWorkerStatusResponse
  const running = !!status.running
  const bindAddr = normalizeText(status.bind_addr || status.bindAddr || '')
  return {
    running,
    bindAddr,
    text: running ? `Worker: running on ${bindAddr}` : 'Worker: stopped',
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asNumber(value: unknown): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return Number.NaN
  return parsed
}

function parseLines(text: string): string[] {
  return String(text || '')
    .split(/\r?\n/g)
    .map((line) => normalizeText(line))
    .filter((line) => !!line && !line.startsWith('#'))
}

function rewritePath(path: string, fromPrefix: string, toPrefix: string): string {
  const source = String(path || '')
  const from = normalizeText(fromPrefix)
  const to = normalizeText(toPrefix)
  if (!from || !to) return source
  const sourceLower = source.toLowerCase()
  const fromLower = from.toLowerCase()
  if (!sourceLower.startsWith(fromLower)) return source
  return `${to}${source.slice(from.length)}`
}

function rewritePaths(paths: string[], fromPrefix: string, toPrefix: string): string[] {
  const out: string[] = []
  for (const raw of paths) {
    const next = normalizeText(raw)
    if (!next) continue
    out.push(rewritePath(next, fromPrefix, toPrefix))
  }
  return out
}

function normalizeTilesPerWorker(value: string | number): number {
  const parsed = Math.floor(Number(value))
  if (!Number.isFinite(parsed) || parsed <= 0) return 4
  return Math.max(1, parsed)
}

function objectiveScore(metrics: Record<string, unknown> | null, objective: string): number {
  if (!metrics) return Number.NaN
  const obj = normalizeText(objective).toLowerCase()
  if (obj === 'pnl' || obj === 'return') return asNumber(metrics.pnl)
  if (obj === 'trades') return asNumber(metrics.trades)
  if (obj === 'exposure') return asNumber(metrics.exposure)
  if (obj === 'net_exposure' || obj === 'netexposure') return asNumber(metrics.net_exposure)
  if (obj === 'max_dd' || obj === 'maxdd') {
    const dd = asNumber(metrics.max_dd)
    return Number.isFinite(dd) ? -dd : Number.NaN
  }
  return asNumber(metrics.sharpe)
}

function fmt(value: unknown, digits = 6): string {
  const x = Number(value)
  return Number.isFinite(x) ? x.toFixed(digits) : '-'
}

function renderAssetsOutput(respRaw: unknown, req: Record<string, unknown>): string {
  const resp = (respRaw && typeof respRaw === 'object' ? respRaw : {}) as DistributedAssetsRunResponse
  const total = Math.max(0, Math.floor(Number(resp.total || 0)))
  const completed = Math.max(0, Math.floor(Number(resp.completed || 0)))
  const successes = Math.max(0, Math.floor(Number(resp.successes || 0)))
  const failures = Math.max(0, Math.floor(Number(resp.failures || 0)))
  const cancelled = !!resp.cancelled
  const objective = normalizeText(req.objective) || 'Sharpe'

  const rows = Array.isArray(resp.results) ? resp.results : []
  const okRows: Array<{
    csv: string
    params: Record<string, unknown> | null
    metrics: Record<string, unknown> | null
    score: number
  }> = []
  const badRows: Array<{ csv: string; error: string }> = []

  for (const row of rows) {
    const item = (row && typeof row === 'object' ? row : {}) as DistributedAssetOutcomeResponse
    const csv = normalizeText(item.csv_path)
    const error = normalizeText(item.error)
    if (error) {
      badRows.push({ csv, error })
      continue
    }
    const result = asRecord(item.result)
    const params = asRecord(result?.best_params)
    const metrics = asRecord(result?.best_metrics)
    if (!params || !metrics) continue
    okRows.push({
      csv,
      params,
      metrics,
      score: objectiveScore(metrics, objective),
    })
  }

  okRows.sort((a, b) => {
    const left = Number.isFinite(a.score) ? a.score : Number.NEGATIVE_INFINITY
    const right = Number.isFinite(b.score) ? b.score : Number.NEGATIVE_INFINITY
    return right - left
  })

  const lines: string[] = []
  lines.push(`assets: ok=${successes} failed=${failures} cancelled=${cancelled}`)
  lines.push(`progress: ${completed}/${total}`)
  lines.push(`objective=${objective}`)
  lines.push('')
  lines.push('top (by objective):')
  const maxRows = Math.min(50, okRows.length)
  for (let i = 0; i < maxRows; i += 1) {
    const row = okRows[i]
    if (!row) continue
    const p = row.params || {}
    const m = row.metrics || {}
    lines.push(
      `${String(i + 1).padStart(3, ' ')} score=${fmt(row.score, 6)} pnl=${fmt(m.pnl, 6)} sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${fmt(m.trades, 0)} | `
      + `${normalizeText(p.fast_ma_type) || '?'}(${fmt(p.fast_len, 0)}) vs ${normalizeText(p.slow_ma_type) || '?'}(${fmt(p.slow_len, 0)}) | ${row.csv}`,
    )
  }

  if (!okRows.length) lines.push('- no successful runs with best_params/best_metrics.')

  if (badRows.length) {
    lines.push('')
    lines.push('errors (first 25):')
    const maxErrors = Math.min(25, badRows.length)
    for (let i = 0; i < maxErrors; i += 1) {
      const row = badRows[i]
      if (!row) continue
      lines.push(`- ${row.csv}: ${row.error}`)
    }
  }

  return lines.join('\n').trim() || 'Distributed assets run finished.'
}

function renderTilesOutput(respRaw: unknown): string {
  const resp = (respRaw && typeof respRaw === 'object' ? respRaw : {}) as DistributedTiledRunResponse
  const cancelled = !!resp.cancelled
  const total = Math.max(0, Math.floor(Number(resp.tiles_total || 0)))
  const completed = Math.max(0, Math.floor(Number(resp.tiles_completed || 0)))
  if (cancelled) return `Cancelled (${completed}/${total} tiles).`

  const result = asRecord(resp.result)
  if (!result) {
    const errors = Array.isArray(resp.errors)
      ? resp.errors.map((entry) => normalizeText(entry)).filter((entry) => !!entry)
      : []
    if (!errors.length) return 'Tiled run failed (no result).'
    return errors.join('\n')
  }

  return `Tiled run complete (${completed}/${total} tiles). Result loaded into Results.`
}

function hydrateDistributedTilesResult(
  req: Record<string, unknown>,
  result: Record<string, unknown>,
  csvPath: string,
): void {
  const ts = new Date().toISOString()
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: '',
        report_xlsx_path: '',
        csv_path: csvPath,
        tags: [],
        req,
        res: result,
      },
      currentValidation: null,
    },
    'vue',
  )
  refreshTopTableFromWorkflowRun()
}

function buildDistributedStatusLineForAssets(respRaw: unknown): string {
  const resp = (respRaw && typeof respRaw === 'object' ? respRaw : {}) as DistributedAssetsRunResponse
  const cancelled = !!resp.cancelled
  const completed = Math.max(0, Math.floor(Number(resp.completed || 0)))
  const total = Math.max(0, Math.floor(Number(resp.total || 0)))
  const successes = Math.max(0, Math.floor(Number(resp.successes || 0)))
  const failures = Math.max(0, Math.floor(Number(resp.failures || 0)))
  if (cancelled) return `Distributed assets: cancelled (${completed}/${total}).`
  if (failures > 0) return `Distributed assets: done (${successes} ok, ${failures} failed).`
  return `Distributed assets: done (${successes} ok).`
}

export async function generateToken(): Promise<string> {
  const size = 24
  if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
    const arr = new Uint8Array(size)
    crypto.getRandomValues(arr)
    return Array.from(arr).map((value) => value.toString(16).padStart(2, '0')).join('')
  }
  let out = ''
  for (let i = 0; i < size; i += 1) {
    out += Math.floor(Math.random() * 256)
      .toString(16)
      .padStart(2, '0')
  }
  return out
}

export async function startWorker(bindAddr: string, authToken: string): Promise<DistributedWorkerStatusSummary> {
  const bind_addr = normalizeText(bindAddr) || '127.0.0.1:48991'
  const auth_token = normalizeText(authToken)
  if (!auth_token) throw new Error('Worker auth token is required.')
  const status = await invokeCommand('distributed_start_worker_server', { cfg: { bind_addr, auth_token } })
  return normalizeWorkerStatus(status)
}

export async function stopWorker(): Promise<DistributedWorkerStatusSummary> {
  await invokeCommand('distributed_stop_worker_server')
  return refreshWorkerStatus()
}

export async function refreshWorkerStatus(): Promise<DistributedWorkerStatusSummary> {
  const status = await invokeCommand('distributed_worker_server_status')
  return normalizeWorkerStatus(status)
}

export async function fillFromPolygon(): Promise<DistributedPolygonFillSummary> {
  const raw = (await invokeCommand('polygon_get_cache_info')) as PolygonCacheInfoResponse
  const datasets = Array.isArray(raw?.datasets) ? raw.datasets : []
  const paths: string[] = []
  for (const entry of datasets) {
    const typed = (entry && typeof entry === 'object' ? entry : {}) as PolygonCacheDatasetResponse
    const path = normalizeText(typed.csv_path)
    if (!path) continue
    paths.push(path)
  }
  return {
    paths,
    count: paths.length,
  }
}

export async function useCurrentCsv(path: string): Promise<DistributedCurrentCsvSummary> {
  return {
    path: normalizeText(path),
  }
}

export async function runAssets(options: DistributedRunAssetsOptions): Promise<DistributedRunSummary> {
  const auth_token = normalizeText(options.authToken)
  if (!auth_token) throw new Error('Auth token is required.')

  const worker_urls = parseLines(options.workerUrlsText)
  if (!worker_urls.length) throw new Error('Add at least one worker URL.')

  let csv_paths = parseLines(options.assetsCsvText)
  if (!csv_paths.length) throw new Error('Add at least one CSV path.')
  csv_paths = rewritePaths(csv_paths, options.rewriteFrom, options.rewriteTo)

  const req = await buildDoubleMaRequestFromVueStores(options.requestOverrides || {})
  const dist_req = { worker_urls, auth_token, csv_paths, req }
  const resp = (await invokeCommand('distributed_run_double_ma_assets', { dist_req })) as DistributedAssetsRunResponse

  return {
    outputPreview: renderAssetsOutput(resp, req),
    statusLine: buildDistributedStatusLineForAssets(resp),
    cancelled: !!resp?.cancelled,
  }
}

export async function runTiles(options: DistributedRunTilesOptions): Promise<DistributedRunSummary> {
  const auth_token = normalizeText(options.authToken)
  if (!auth_token) throw new Error('Auth token is required.')

  const worker_urls = parseLines(options.workerUrlsText)
  if (!worker_urls.length) throw new Error('Add at least one worker URL.')

  let csv_path = normalizeText(options.tileCsvPath)
  if (!csv_path) throw new Error('Dataset CSV path is required (shared/UNC path).')
  csv_path = rewritePath(csv_path, options.rewriteFrom, options.rewriteTo)

  const req = await buildDoubleMaRequestFromVueStores(options.requestOverrides || {})
  const dist_req = {
    worker_urls,
    auth_token,
    csv_path,
    req,
    tiles_per_worker: normalizeTilesPerWorker(options.tilesPerWorker),
  }
  const resp = (await invokeCommand('distributed_run_double_ma_tiled', { dist_req })) as DistributedTiledRunResponse
  const cancelled = !!resp?.cancelled
  const outputPreview = renderTilesOutput(resp)
  const result = asRecord(resp?.result)
  if (!cancelled && result) {
    hydrateDistributedTilesResult(req, result, csv_path)
  }
  return {
    outputPreview,
    statusLine: cancelled
      ? 'Distributed tiles: cancelled.'
      : (result ? 'Distributed tiles: done.' : 'Distributed tiles: failed.'),
    cancelled,
  }
}

export async function cancel(): Promise<void> {
  await requestCancelCurrentRun()
}
