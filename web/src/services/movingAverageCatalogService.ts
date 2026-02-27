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
    out.push({ id, label, params })
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
