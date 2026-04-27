import { invokeCommand } from '../tauri/invoke'
import { dynamicIndicatorCatalogQueryKey } from './catalogQueryKeys'

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

function normalizeBool(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeParamKind(value: unknown): DynamicIndicatorParamKind {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'int') return 'int'
  if (kind === 'bool') return 'bool'
  if (kind === 'enum_string') return 'enum_string'
  return 'float'
}

function normalizeValueType(value: unknown): DynamicIndicatorValueType {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'f32') return 'f32'
  if (kind === 'i32') return 'i32'
  if (kind === 'bool') return 'bool'
  return 'f64'
}

function normalizeInputKind(value: unknown): DynamicIndicatorInputKind {
  const kind = normalizeText(value).toLowerCase()
  if (kind === 'candles') return 'candles'
  if (kind === 'ohlc') return 'ohlc'
  if (kind === 'ohlcv') return 'ohlcv'
  if (kind === 'high_low') return 'high_low'
  if (kind === 'close_volume') return 'close_volume'
  return 'slice'
}

function normalizeParamValue(value: unknown): DynamicIndicatorParamValue | null {
  if (!value || typeof value !== 'object') return null
  const row = value as RawDynamicIndicatorParamValue
  const kind = normalizeParamKind(row.kind)
  if (kind === 'bool') {
    return { kind, value: normalizeBool(row.value) }
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
  if (!value || typeof value !== 'object') return null
  const row = value as RawDynamicIndicatorParamDescriptor
  const key = normalizeText(row.key)
  if (!key) return null
  const label = normalizeText(row.label) || key
  const kind = normalizeParamKind(row.kind)
  const required = normalizeBool(row.required)
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
  if (!value || typeof value !== 'object') return null
  const row = value as RawDynamicIndicatorOutputDescriptor
  const id = normalizeText(row.id)
  if (!id) return null
  const label = normalizeText(row.label) || id
  return {
    id,
    label,
    valueType: normalizeValueType(row.value_type),
  }
}

export function normalizeDynamicIndicatorCatalog(value: unknown): DynamicIndicatorCatalogEntry[] {
  if (!Array.isArray(value)) return []
  const out: DynamicIndicatorCatalogEntry[] = []
  const seen = new Set<string>()
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as RawDynamicIndicatorCatalogEntry
    const id = normalizeText(row.id).toLowerCase()
    if (!id || seen.has(id)) continue
    seen.add(id)
    const label = normalizeText(row.label) || id
    const category = normalizeText(row.category) || 'uncategorized'
    const outputsRaw = Array.isArray(row.outputs) ? row.outputs : []
    const outputs: DynamicIndicatorOutputDescriptor[] = []
    const outputSeen = new Set<string>()
    for (const output of outputsRaw) {
      const normalized = normalizeOutputDescriptor(output)
      if (!normalized) continue
      if (outputSeen.has(normalized.id)) continue
      outputSeen.add(normalized.id)
      outputs.push(normalized)
    }
    const paramsRaw = Array.isArray(row.params) ? row.params : []
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
      dynamicStrategyEligible: normalizeBool(row.dynamic_strategy_eligible),
      inputKind: normalizeInputKind(row.input_kind),
      requiresCandles: normalizeBool(row.requires_candles),
      supportsCpuSingle: normalizeBool(row.supports_cpu_single),
      supportsCpuBatch: normalizeBool(row.supports_cpu_batch),
      supportsCudaSingle: normalizeBool(row.supports_cuda_single),
      supportsCudaSweep: normalizeBool(row.supports_cuda_sweep),
      supportsCudaVram: normalizeBool(row.supports_cuda_vram),
      supportsGpuKernel: normalizeBool(row.supports_gpu_kernel),
      prefersGpuBySpeedup: normalizeBool(row.prefers_gpu_by_speedup),
      outputCount: Math.max(0, Math.floor(normalizeFinite(row.output_count) ?? outputs.length)),
      outputs,
      params,
      notes: normalizeText(row.notes),
    })
  }
  return out
}

export async function listDynamicIndicatorCatalog(forceRefresh = false): Promise<DynamicIndicatorCatalogEntry[]> {
  if (!forceRefresh && catalogCache) return catalogCache
  const raw = await invokeCommand('list_dynamic_indicators')
  const catalog = normalizeDynamicIndicatorCatalog(raw)
  catalogCache = catalog
  return catalog
}

export function resetDynamicIndicatorCatalogCacheForTests(): void {
  catalogCache = null
}
