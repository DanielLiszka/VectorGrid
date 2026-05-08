import { invokeCommand } from '../tauri/invoke'
import type { GpuSpeedupEntry as GeneratedGpuSpeedupEntry } from '../tauri/generatedBindings'
import { readCachedCatalog, removeCachedCatalog } from './catalogReadCache'
import { gpuSpeedupsQueryKey } from './catalogQueryKeys'
import { requireBoundaryArray, requireBoundaryObject } from './serviceBoundarySchemas'

export { gpuSpeedupsQueryKey }

export interface GpuSpeedupEntry {
  id: string
  name: string
  speedup: number
  cpuMs: number
  cudaMs: number
}

type RawGpuSpeedupEntry = Partial<GeneratedGpuSpeedupEntry>

let gpuSpeedupsCache: GpuSpeedupEntry[] | null = null
let gpuSpeedupMapCache: Record<string, GpuSpeedupEntry> | null = null

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function requireText(value: unknown, field: string): string {
  const text = normalizeText(value)
  if (!text) throw new Error(`GPU speedup catalog field '${field}' must be a non-empty string`)
  return text
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function requireFinite(value: unknown, field: string): number {
  const parsed = normalizeFinite(value)
  if (parsed == null) throw new Error(`GPU speedup catalog field '${field}' must be finite`)
  return parsed
}

export function normalizeGpuSpeedups(value: unknown): GpuSpeedupEntry[] {
  const rows = requireBoundaryArray(value, 'GPU speedup catalog response')
  const out: GpuSpeedupEntry[] = []
  const seen = new Set<string>()

  for (const entry of rows) {
    const row = requireBoundaryObject(entry, 'GPU speedup catalog entry') as RawGpuSpeedupEntry
    const id = requireText(row.indicator_id, 'indicator_id')
    if (seen.has(id)) continue
    const speedup = requireFinite(row.speedup, `${id}.speedup`)
    const cpuMs = requireFinite(row.cpu_ms, `${id}.cpu_ms`)
    const cudaMs = requireFinite(row.cuda_ms, `${id}.cuda_ms`)
    out.push({
      id,
      name: normalizeText(row.indicator_name) || id,
      speedup,
      cpuMs,
      cudaMs,
    })
    seen.add(id)
  }

  return out
}

export async function listGpuSpeedups(forceRefresh = false): Promise<GpuSpeedupEntry[]> {
  const entries = await readCachedCatalog(
    gpuSpeedupsQueryKey,
    async () => {
      if (!forceRefresh && gpuSpeedupsCache) return gpuSpeedupsCache
      const raw = await invokeCommand('list_gpu_speedups')
      return normalizeGpuSpeedups(raw)
    },
    { forceRefresh },
  )
  gpuSpeedupsCache = entries
  gpuSpeedupMapCache = null
  return entries
}

export async function listGpuSpeedupMap(forceRefresh = false): Promise<Record<string, GpuSpeedupEntry>> {
  if (!forceRefresh && gpuSpeedupMapCache) return gpuSpeedupMapCache
  const entries = await listGpuSpeedups(forceRefresh)
  const out: Record<string, GpuSpeedupEntry> = {}
  for (const entry of entries) {
    out[entry.id] = entry
  }
  gpuSpeedupMapCache = out
  return out
}

export function resetGpuSpeedupsCacheForTests(): void {
  gpuSpeedupsCache = null
  gpuSpeedupMapCache = null
  removeCachedCatalog(gpuSpeedupsQueryKey)
}
