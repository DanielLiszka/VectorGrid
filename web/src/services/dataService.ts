import { invokeCommand } from '../tauri/invoke'

function normalizeText(value: unknown): string {
  return String(value || '').trim()
}

function normalizeInteger(value: unknown, fallback = 0): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return fallback
  if (parsed < 0) return 0
  return Math.floor(parsed)
}

function normalizeUnixTs(value: unknown): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return 0
  return Math.floor(parsed)
}

function normalizeMaybeNumber(value: unknown): number | null {
  if (value == null) return null
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return null
  return parsed
}

interface MarketDataDirInfoResponse {
  default_root_dir?: unknown
  configured_root_dir?: unknown
  effective_root_dir?: unknown
  polygon_cache_dir?: unknown
}

interface SampleCsvEntry {
  label?: unknown
  path?: unknown
}

interface PriceDataMetaResponse {
  len?: unknown
  has_open?: unknown
  has_high?: unknown
  has_low?: unknown
  has_close?: unknown
  has_volume?: unknown
  min_ts?: unknown
  max_ts?: unknown
  timeframe?: unknown
  source_size_bytes?: unknown
  source_mtime_utc_ms?: unknown
}

interface LoadPriceDataResponse {
  id?: unknown
  meta?: unknown
  cache_hit?: unknown
  cache_path?: unknown
}

export interface DataMetaSummary {
  len: number
  hasOpen: boolean
  hasHigh: boolean
  hasLow: boolean
  hasClose: boolean
  hasVolume: boolean
  minTs: number
  maxTs: number
  timeframe: string
  sourceSizeBytes: number | null
  sourceMtimeUtcMs: number | null
}

export interface DataLoadResult {
  path: string
  dataId: string
  meta: DataMetaSummary
  cacheHit: boolean
  cachePath: string
}

export interface DataSampleCsvSummary {
  label: string
  path: string
  disabled: boolean
}

export interface MarketDataDirInfoSummary {
  defaultRootDir: string
  configuredRootDir: string
  effectiveRootDir: string
  polygonCacheDir: string
  hint: string
}

export interface MarketDataDirOpenResult {
  path: string
  openedWith: 'open_path' | 'reveal_path'
}

function buildMarketDataDirHint(summary: MarketDataDirInfoSummary): string {
  const polygonCache = summary.polygonCacheDir || '—'
  if (summary.configuredRootDir) {
    return `Using custom root: ${summary.configuredRootDir} · Polygon cache: ${polygonCache}`
  }
  return `Using default root: ${summary.defaultRootDir || '—'} · Polygon cache: ${polygonCache}`
}

function normalizeMarketDataDirInfo(raw: unknown): MarketDataDirInfoSummary {
  const info = (raw && typeof raw === 'object' ? raw : {}) as MarketDataDirInfoResponse
  const defaultRootDir = normalizeText(info.default_root_dir)
  const configuredRootDir = normalizeText(info.configured_root_dir)
  const effectiveRootDir = normalizeText(info.effective_root_dir)
  const polygonCacheDir = normalizeText(info.polygon_cache_dir)
  const summary: MarketDataDirInfoSummary = {
    defaultRootDir,
    configuredRootDir,
    effectiveRootDir,
    polygonCacheDir,
    hint: '',
  }
  summary.hint = buildMarketDataDirHint(summary)
  return summary
}

function normalizeDataMeta(raw: unknown): DataMetaSummary {
  const meta = (raw && typeof raw === 'object' ? raw : {}) as PriceDataMetaResponse
  return {
    len: normalizeInteger(meta.len),
    hasOpen: Boolean(meta.has_open),
    hasHigh: Boolean(meta.has_high),
    hasLow: Boolean(meta.has_low),
    hasClose: Boolean(meta.has_close),
    hasVolume: Boolean(meta.has_volume),
    minTs: normalizeUnixTs(meta.min_ts),
    maxTs: normalizeUnixTs(meta.max_ts),
    timeframe: normalizeText(meta.timeframe),
    sourceSizeBytes: normalizeMaybeNumber(meta.source_size_bytes),
    sourceMtimeUtcMs: normalizeMaybeNumber(meta.source_mtime_utc_ms),
  }
}

function normalizeLoadPriceDataResponse(raw: unknown, path: string): DataLoadResult {
  const row = (raw && typeof raw === 'object' ? raw : {}) as LoadPriceDataResponse
  return {
    path: normalizeText(path),
    dataId: normalizeText(row.id),
    meta: normalizeDataMeta(row.meta),
    cacheHit: Boolean(row.cache_hit),
    cachePath: normalizeText(row.cache_path),
  }
}

function fileBasename(path: string): string {
  const normalized = normalizeText(path).replace(/\\/g, '/')
  if (!normalized) return ''
  const parts = normalized.split('/')
  return normalizeText(parts[parts.length - 1] || '')
}

function queryById<T extends Element>(id: string): T | null {
  if (typeof document === 'undefined') return null
  const byQuery = typeof document.querySelector === 'function'
    ? document.querySelector(`[id="${id}"]`)
    : null
  if (byQuery) return byQuery as T
  const getById = (document as unknown as { getElementById?: (nextId: string) => Element | null }).getElementById
  if (typeof getById !== 'function') return null
  return (getById.call(document, id) as T | null) ?? null
}

export async function getMarketDataDirInfo(): Promise<MarketDataDirInfoSummary> {
  const info = await invokeCommand('get_market_data_dir_info')
  return normalizeMarketDataDirInfo(info)
}

export async function listSampleCsvs(): Promise<DataSampleCsvSummary[]> {
  const rows = (await invokeCommand('list_sample_csvs')) as unknown
  const samples = Array.isArray(rows) ? rows : []
  const out: DataSampleCsvSummary[] = []
  for (const entry of samples) {
    const typed = (entry && typeof entry === 'object' ? entry : {}) as SampleCsvEntry
    const path = normalizeText(typed.path)
    if (!path) continue
    const label = normalizeText(typed.label) || fileBasename(path) || path
    out.push({
      label,
      path,
      disabled: false,
    })
  }
  return out
}

export async function browseCsv(): Promise<DataLoadResult | null> {
  try {
    const picked = await invokeCommand<string | null>('pick_csv_file')
    const path = normalizeText(picked)
    if (!path) return null
    return await loadCsv(path)
  } catch {
    // Keep browse action non-throwing to match legacy runtime behavior.
    return null
  }
}

export async function openPolygon(): Promise<void> {
  const modal = queryById<HTMLElement>('polygonModal')
  if (!(modal instanceof HTMLElement)) return
  modal.classList.add('open')
  if (document.body) document.body.style.overflow = 'hidden'
  const focusEl = queryById<HTMLElement>('polygonSymbol') ?? queryById<HTMLElement>('polygonApiKey')
  if (focusEl instanceof HTMLElement && typeof focusEl.focus === 'function') {
    focusEl.focus()
  }
}

export async function closePolygon(): Promise<void> {
  const modal = queryById<HTMLElement>('polygonModal')
  if (!(modal instanceof HTMLElement)) return
  modal.classList.remove('open')
  if (document.body) document.body.style.overflow = ''
}

export async function loadCsv(path?: string): Promise<DataLoadResult> {
  const nextPath = normalizeText(path)
  if (!nextPath) throw new Error('Enter a CSV path.')
  const response = await invokeCommand('load_price_data', { path: nextPath })
  return normalizeLoadPriceDataResponse(response, nextPath)
}

export async function clearData(): Promise<void> {
  // Data clear is handled in Vue store state. No backend command is required.
}

export async function applyTimeframe(timeframe: string, baseDataId?: string): Promise<DataLoadResult> {
  const nextTimeframe = normalizeText(timeframe)
  const baseId = normalizeText(baseDataId)
  if (!baseId) throw new Error('Load a dataset first (missing base data id).')
  if (!nextTimeframe) throw new Error('Choose a timeframe first.')
  const response = await invokeCommand('resample_price_data', { baseDataId: baseId, timeframe: nextTimeframe })
  return normalizeLoadPriceDataResponse(response, '')
}

export async function browseMarketDataDir(): Promise<string> {
  const picked = await invokeCommand<string | null>('pick_folder')
  return normalizeText(picked)
}

export async function setMarketDataDir(dir: string, moveExisting: boolean): Promise<MarketDataDirInfoSummary> {
  const normalizedDir = normalizeText(dir)
  if (!normalizedDir) {
    throw new Error('Enter a market data folder path first.')
  }
  await invokeCommand('set_market_data_dir', { dir: normalizedDir, moveExisting: !!moveExisting })
  return getMarketDataDirInfo()
}

export async function resetMarketDataDir(): Promise<MarketDataDirInfoSummary> {
  await invokeCommand('clear_market_data_dir')
  return getMarketDataDirInfo()
}

export async function openMarketDataDir(): Promise<MarketDataDirOpenResult> {
  const info = await getMarketDataDirInfo()
  const path = normalizeText(info.polygonCacheDir || info.effectiveRootDir)
  if (!path) throw new Error('No market data folder path available.')

  try {
    await invokeCommand('open_path', { path })
    return { path, openedWith: 'open_path' }
  } catch {
    await invokeCommand('reveal_path', { path })
    return { path, openedWith: 'reveal_path' }
  }
}

export async function loadSampleCsv(index: number, path?: string): Promise<DataLoadResult | null> {
  const nextPath = normalizeText(path)
  if (nextPath) {
    return await loadCsv(nextPath)
  }
  if (!Number.isFinite(index) || index < 0) return null
  try {
    const samples = await listSampleCsvs()
    const entry = samples[Math.floor(index)] as DataSampleCsvSummary | undefined
    const samplePath = normalizeText(entry?.path)
    if (!samplePath) return null
    return await loadCsv(samplePath)
  } catch {
    // Keep sample-load action non-throwing to match legacy runtime behavior.
    return null
  }
}
