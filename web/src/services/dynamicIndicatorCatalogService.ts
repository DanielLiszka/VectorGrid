import { invokeCommand } from '../tauri/invoke'
import { readCachedCatalog, removeCachedCatalog } from './catalogReadCache'
import { dynamicIndicatorCatalogQueryKey } from './catalogQueryKeys'
import { requireBoundaryArray, requireBoundaryObject } from './serviceBoundarySchemas'

export { dynamicIndicatorCatalogQueryKey }

export type DynamicIndicatorParamKind = 'int' | 'float' | 'bool' | 'enum_string'
export type DynamicIndicatorValueType = 'f64' | 'f32' | 'i32' | 'bool'
export type DynamicIndicatorInputKind =
  | 'slice'
  | 'candles'
  | 'ohlc'
  | 'ohlcv'
  | 'high_low'
  | 'close_volume'

export interface DynamicIndicatorParamValue {
  kind: DynamicIndicatorParamKind
  value: number | boolean | string
}

export interface DynamicIndicatorParamDescriptor {
  key: string
  label: string
  kind: DynamicIndicatorParamKind
  required: boolean
  defaultValue: DynamicIndicatorParamValue | null
  min: number | null
  max: number | null
  step: number | null
  enumValues: string[]
  notes: string
}

export interface DynamicIndicatorOutputDescriptor {
  id: string
  label: string
  valueType: DynamicIndicatorValueType
}

export interface DynamicIndicatorCatalogEntry {
  id: string
  label: string
  category: string
  dynamicStrategyEligible: boolean
  inputKind: DynamicIndicatorInputKind
  requiresCandles: boolean
  supportsCpuSingle: boolean
  supportsCpuBatch: boolean
  supportsCudaSingle: boolean
  supportsCudaSweep: boolean
  supportsCudaVram: boolean
  supportsGpuKernel: boolean
  prefersGpuBySpeedup: boolean
  outputCount: number
  outputs: DynamicIndicatorOutputDescriptor[]
  params: DynamicIndicatorParamDescriptor[]
  notes: string
}

interface RawDynamicIndicatorParamValue {
  kind?: unknown
  value?: unknown
}

interface RawDynamicIndicatorParamDescriptor {
  key?: unknown
  label?: unknown
  kind?: unknown
  required?: unknown
  default?: unknown
  min?: unknown
  max?: unknown
  step?: unknown
  enum_values?: unknown
  notes?: unknown
}

interface RawDynamicIndicatorOutputDescriptor {
  id?: unknown
  label?: unknown
  value_type?: unknown
}

interface RawDynamicIndicatorCatalogEntry {
  id?: unknown
  label?: unknown
  category?: unknown
  dynamic_strategy_eligible?: unknown
  input_kind?: unknown
  requires_candles?: unknown
  supports_cpu_single?: unknown
  supports_cpu_batch?: unknown
  supports_cuda_single?: unknown
  supports_cuda_sweep?: unknown
  supports_cuda_vram?: unknown
  supports_gpu_kernel?: unknown
  prefers_gpu_by_speedup?: unknown
  output_count?: unknown
  outputs?: unknown
  params?: unknown
  notes?: unknown
}

let catalogCache: DynamicIndicatorCatalogEntry[] | null = null

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function requireText(value: unknown, field: string): string {
  const text = normalizeText(value)
  if (!text) throw new Error(`Dynamic indicator catalog field '${field}' must be a non-empty string`)
  return text
}

function normalizeBool(value: unknown, field: string): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  if (text === 'true' || text === '1' || text === 'yes' || text === 'on') return true
  if (text === 'false' || text === '0' || text === 'no' || text === 'off') return false
  throw new Error(`Dynamic indicator catalog field '${field}' must be boolean-like`)
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeParamKind(value: unknown, field: string): DynamicIndicatorParamKind {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'int') return 'int'
  if (kind === 'float') return 'float'
  if (kind === 'bool') return 'bool'
  if (kind === 'enum_string') return 'enum_string'
  throw new Error(`Dynamic indicator catalog field '${field}' has unknown param kind '${kind}'`)
}

function normalizeValueType(value: unknown, field: string): DynamicIndicatorValueType {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'f64') return 'f64'
  if (kind === 'f32') return 'f32'
  if (kind === 'i32') return 'i32'
  if (kind === 'bool') return 'bool'
  throw new Error(`Dynamic indicator catalog field '${field}' has unknown value type '${kind}'`)
}

function normalizeInputKind(value: unknown, field: string): DynamicIndicatorInputKind {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'slice') return 'slice'
  if (kind === 'candles') return 'candles'
  if (kind === 'ohlc') return 'ohlc'
  if (kind === 'ohlcv') return 'ohlcv'
  if (kind === 'high_low') return 'high_low'
  if (kind === 'close_volume') return 'close_volume'
  throw new Error(`Dynamic indicator catalog field '${field}' has unknown input kind '${kind}'`)
}

function normalizeParamValue(value: unknown): DynamicIndicatorParamValue | null {
  if (!value || typeof value !== 'object') return null
  const row = value as RawDynamicIndicatorParamValue
  const kind = normalizeParamKind(row.kind, 'default.kind')
  if (kind === 'bool') {
    return { kind, value: normalizeBool(row.value, 'default.value') }
  }
  if (kind === 'enum_string') {
    const normalized = normalizeText(row.value)
    if (!normalized) return null
    return { kind, value: normalized }
  }
  const numeric = normalizeFinite(row.value)
  if (numeric == null) return null
  return { kind, value: numeric }
}

function normalizeParamDescriptor(value: unknown): DynamicIndicatorParamDescriptor | null {
  const row = requireBoundaryObject(value, 'dynamic indicator catalog param') as RawDynamicIndicatorParamDescriptor
  const key = requireText(row.key, 'params.key')
  const label = normalizeText(row.label) || key
  const kind = normalizeParamKind(row.kind, `params.${key}.kind`)
  const required = normalizeBool(row.required, `params.${key}.required`)
  const defaultValue = normalizeParamValue(row.default)
  const min = normalizeFinite(row.min)
  const max = normalizeFinite(row.max)
  const rawStep = normalizeFinite(row.step)
  const step = rawStep != null && rawStep > 0 ? rawStep : null
  const enumValues = Array.isArray(row.enum_values)
    ? row.enum_values
        .map((entry) => normalizeText(entry))
        .filter((entry) => entry.length > 0)
    : []
  const notes = normalizeText(row.notes)
  return {
    key,
    label,
    kind,
    required,
    defaultValue,
    min,
    max,
    step,
    enumValues,
    notes,
  }
}

function normalizeOutputDescriptor(value: unknown): DynamicIndicatorOutputDescriptor | null {
  const row = requireBoundaryObject(value, 'dynamic indicator catalog output') as RawDynamicIndicatorOutputDescriptor
  const id = requireText(row.id, 'outputs.id')
  const label = normalizeText(row.label) || id
  return {
    id,
    label,
    valueType: normalizeValueType(row.value_type, `outputs.${id}.value_type`),
  }
}

export function normalizeDynamicIndicatorCatalog(value: unknown): DynamicIndicatorCatalogEntry[] {
  const rows = requireBoundaryArray(value, 'dynamic indicator catalog response')
  const out: DynamicIndicatorCatalogEntry[] = []
  const seen = new Set<string>()
  for (const entry of rows) {
    const row = requireBoundaryObject(entry, 'dynamic indicator catalog entry') as RawDynamicIndicatorCatalogEntry
    const id = requireText(row.id, 'id').toLowerCase()
    if (seen.has(id)) continue
    seen.add(id)
    const label = normalizeText(row.label) || id
    const category = normalizeText(row.category) || 'uncategorized'
    const outputsRaw = requireBoundaryArray(row.outputs, `${id}.outputs`)
    const outputs: DynamicIndicatorOutputDescriptor[] = []
    const outputSeen = new Set<string>()
    for (const output of outputsRaw) {
      const normalized = normalizeOutputDescriptor(output)
      if (!normalized) continue
      if (outputSeen.has(normalized.id)) continue
      outputSeen.add(normalized.id)
      outputs.push(normalized)
    }
    const paramsRaw = requireBoundaryArray(row.params, `${id}.params`)
    const params: DynamicIndicatorParamDescriptor[] = []
    const paramSeen = new Set<string>()
    for (const param of paramsRaw) {
      const normalized = normalizeParamDescriptor(param)
      if (!normalized) continue
      if (paramSeen.has(normalized.key)) continue
      paramSeen.add(normalized.key)
      params.push(normalized)
    }
    out.push({
      id,
      label,
      category,
      dynamicStrategyEligible: normalizeBool(row.dynamic_strategy_eligible, `${id}.dynamic_strategy_eligible`),
      inputKind: normalizeInputKind(row.input_kind, `${id}.input_kind`),
      requiresCandles: normalizeBool(row.requires_candles, `${id}.requires_candles`),
      supportsCpuSingle: normalizeBool(row.supports_cpu_single, `${id}.supports_cpu_single`),
      supportsCpuBatch: normalizeBool(row.supports_cpu_batch, `${id}.supports_cpu_batch`),
      supportsCudaSingle: normalizeBool(row.supports_cuda_single, `${id}.supports_cuda_single`),
      supportsCudaSweep: normalizeBool(row.supports_cuda_sweep, `${id}.supports_cuda_sweep`),
      supportsCudaVram: normalizeBool(row.supports_cuda_vram, `${id}.supports_cuda_vram`),
      supportsGpuKernel: normalizeBool(row.supports_gpu_kernel, `${id}.supports_gpu_kernel`),
      prefersGpuBySpeedup: normalizeBool(row.prefers_gpu_by_speedup, `${id}.prefers_gpu_by_speedup`),
      outputCount: Math.max(0, Math.floor(normalizeFinite(row.output_count) ?? outputs.length)),
      outputs,
      params,
      notes: normalizeText(row.notes),
    })
  }
  return out
}

export async function listDynamicIndicatorCatalog(forceRefresh = false): Promise<DynamicIndicatorCatalogEntry[]> {
  const catalog = await readCachedCatalog(
    dynamicIndicatorCatalogQueryKey,
    async () => {
      if (!forceRefresh && catalogCache) return catalogCache
      const raw = await invokeCommand('list_dynamic_indicators')
      return normalizeDynamicIndicatorCatalog(raw)
    },
    { forceRefresh },
  )
  catalogCache = catalog
  return catalog
}

export function resetDynamicIndicatorCatalogCacheForTests(): void {
  catalogCache = null
  removeCachedCatalog(dynamicIndicatorCatalogQueryKey)
}
