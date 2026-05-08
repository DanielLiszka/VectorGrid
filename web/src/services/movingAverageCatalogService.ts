import { invokeCommand } from '../tauri/invoke'
import { readCachedCatalog, removeCachedCatalog } from './catalogReadCache'
import { movingAverageCatalogQueryKey } from './catalogQueryKeys'
import { requireBoundaryArray, requireBoundaryObject } from './serviceBoundarySchemas'

export { movingAverageCatalogQueryKey }

export type MaParamKind = 'int' | 'float'

export interface MaParamDescriptor {
  key: string
  label: string
  kind: MaParamKind
  defaultValue: number
  min: number | null
  max: number | null
  step: number | null
  notes: string
}

export interface MovingAverageCatalogEntry {
  id: string
  label: string
  supportsCpu: boolean
  supportsCuda: boolean
  supportsCudaMaSweep: boolean
  supportsCudaKernel: boolean
  prefersCudaKernel: boolean
  requiresCandles: boolean
  periodBased: boolean
  singleOutput: boolean
  notes: string
  params: MaParamDescriptor[]
}

interface RawMaParamDescriptor {
  key?: unknown
  label?: unknown
  kind?: unknown
  default?: unknown
  min?: unknown
  max?: unknown
  step?: unknown
  notes?: unknown
}

interface RawMovingAverageEntry {
  id?: unknown
  label?: unknown
  supports_cpu?: unknown
  supports_cuda?: unknown
  supports_cuda_ma_sweep?: unknown
  supports_cuda_kernel?: unknown
  prefers_cuda_kernel?: unknown
  requires_candles?: unknown
  period_based?: unknown
  single_output?: unknown
  notes?: unknown
  params?: unknown
}

let catalogCache: MovingAverageCatalogEntry[] | null = null

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function requireText(value: unknown, field: string): string {
  const text = normalizeText(value)
  if (!text) throw new Error(`Moving average catalog field '${field}' must be a non-empty string`)
  return text
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeBool(value: unknown, field: string): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  if (text === 'true' || text === '1' || text === 'yes' || text === 'on') return true
  if (text === 'false' || text === '0' || text === 'no' || text === 'off') return false
  throw new Error(`Moving average catalog field '${field}' must be boolean-like`)
}

function normalizeParamKind(value: unknown, field: string): MaParamKind {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'int' || kind === 'float') return kind
  throw new Error(`Moving average catalog field '${field}' has unknown param kind '${kind}'`)
}

function normalizeParamDescriptor(value: unknown): MaParamDescriptor | null {
  const row = requireBoundaryObject(value, 'moving average catalog param') as RawMaParamDescriptor
  const key = requireText(row.key, 'params.key')

  const label = normalizeText(row.label) || key
  const kind = normalizeParamKind(row.kind, `params.${key}.kind`)
  const defaultValue = normalizeFinite(row.default) ?? 0
  const min = normalizeFinite(row.min)
  const max = normalizeFinite(row.max)
  const stepRaw = normalizeFinite(row.step)
  const step = stepRaw != null && stepRaw > 0 ? stepRaw : null
  const notes = normalizeText(row.notes)

  return {
    key,
    label,
    kind,
    defaultValue,
    min,
    max,
    step,
    notes,
  }
}

export function normalizeMovingAverageCatalog(value: unknown): MovingAverageCatalogEntry[] {
  const rows = requireBoundaryArray(value, 'moving average catalog response')
  const out: MovingAverageCatalogEntry[] = []
  for (const entry of rows) {
    const row = requireBoundaryObject(entry, 'moving average catalog entry') as RawMovingAverageEntry
    const id = requireText(row.id, 'id')
    const label = normalizeText(row.label) || id
    const supportsCpu = normalizeBool(row.supports_cpu, `${id}.supports_cpu`)
    const supportsCuda = normalizeBool(row.supports_cuda, `${id}.supports_cuda`)
    const supportsCudaMaSweep = normalizeBool(row.supports_cuda_ma_sweep, `${id}.supports_cuda_ma_sweep`)
    const supportsCudaKernel = normalizeBool(row.supports_cuda_kernel, `${id}.supports_cuda_kernel`)
    const prefersCudaKernel = normalizeBool(row.prefers_cuda_kernel, `${id}.prefers_cuda_kernel`)
    const requiresCandles = normalizeBool(row.requires_candles, `${id}.requires_candles`)
    const periodBased = normalizeBool(row.period_based, `${id}.period_based`)
    const singleOutput = normalizeBool(row.single_output, `${id}.single_output`)
    const notes = normalizeText(row.notes)
    const rawParams = requireBoundaryArray(row.params, `${id}.params`)
    const params: MaParamDescriptor[] = []
    const seen = new Set<string>()
    for (const rawParam of rawParams) {
      const normalized = normalizeParamDescriptor(rawParam)
      if (!normalized) continue
      if (seen.has(normalized.key)) continue
      seen.add(normalized.key)
      params.push(normalized)
    }
    out.push({
      id,
      label,
      supportsCpu,
      supportsCuda,
      supportsCudaMaSweep,
      supportsCudaKernel,
      prefersCudaKernel,
      requiresCandles,
      periodBased,
      singleOutput,
      notes,
      params,
    })
  }
  return out
}

export async function listMovingAverageCatalog(forceRefresh = false): Promise<MovingAverageCatalogEntry[]> {
  const catalog = await readCachedCatalog(
    movingAverageCatalogQueryKey,
    async () => {
      if (!forceRefresh && catalogCache) return catalogCache
      const raw = await invokeCommand('list_moving_averages')
      return normalizeMovingAverageCatalog(raw)
    },
    { forceRefresh },
  )
  catalogCache = catalog
  return catalog
}

export function resetMovingAverageCatalogCacheForTests(): void {
  catalogCache = null
  removeCachedCatalog(movingAverageCatalogQueryKey)
}
