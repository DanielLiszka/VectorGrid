import { invokeCommand } from '../tauri/invoke'

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

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeBool(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function normalizeParamDescriptor(value: unknown): MaParamDescriptor | null {
  if (!value || typeof value !== 'object') return null
  const row = value as RawMaParamDescriptor
  const key = normalizeText(row.key)
  if (!key) return null

  const label = normalizeText(row.label) || key
  const kind = normalizeText(row.kind).toLowerCase() === 'int' ? 'int' : 'float'
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
  if (!Array.isArray(value)) return []
  const out: MovingAverageCatalogEntry[] = []
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as RawMovingAverageEntry
    const id = normalizeText(row.id)
    if (!id) continue
    const label = normalizeText(row.label) || id
    const supportsCpu = normalizeBool(row.supports_cpu)
    const supportsCuda = normalizeBool(row.supports_cuda)
    const supportsCudaMaSweep = normalizeBool(row.supports_cuda_ma_sweep)
    const supportsCudaKernel = normalizeBool(row.supports_cuda_kernel)
    const prefersCudaKernel = normalizeBool(row.prefers_cuda_kernel)
    const requiresCandles = normalizeBool(row.requires_candles)
    const periodBased = normalizeBool(row.period_based)
    const singleOutput = normalizeBool(row.single_output)
    const notes = normalizeText(row.notes)
    const rawParams = Array.isArray(row.params) ? row.params : []
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
  if (!forceRefresh && catalogCache) return catalogCache
  const raw = await invokeCommand('list_moving_averages')
  const catalog = normalizeMovingAverageCatalog(raw)
  catalogCache = catalog
  return catalog
}

export function resetMovingAverageCatalogCacheForTests(): void {
  catalogCache = null
}
