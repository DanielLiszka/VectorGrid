import { invokeCommand } from '../tauri/invoke'
import { gpuSpeedupsQueryKey } from './catalogQueryKeys'

export { gpuSpeedupsQueryKey }

export interface GpuSpeedupEntry {
  id: string
  name: string
  speedup: number
  cpuMs: number
  cudaMs: number
}

interface RawGpuSpeedupEntry {
  indicator_id?: unknown
  indicator_name?: unknown
  speedup?: unknown
  cpu_ms?: unknown
  cuda_ms?: unknown
}

let gpuSpeedupsCache: GpuSpeedupEntry[] | null = null
let gpuSpeedupMapCache: Record<string, GpuSpeedupEntry> | null = null

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

export function normalizeGpuSpeedups(value: unknown): GpuSpeedupEntry[] {
  if (!Array.isArray(value)) return []
  const out: GpuSpeedupEntry[] = []
  const seen = new Set<string>()

  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as RawGpuSpeedupEntry
    const id = normalizeText(row.indicator_id)
    if (!id || seen.has(id)) continue
    const speedup = normalizeFinite(row.speedup)
    const cpuMs = normalizeFinite(row.cpu_ms)
    const cudaMs = normalizeFinite(row.cuda_ms)
    if (speedup == null || cpuMs == null || cudaMs == null) continue
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
  if (!forceRefresh && gpuSpeedupsCache) return gpuSpeedupsCache
  const raw = await invokeCommand('list_gpu_speedups')
  const entries = normalizeGpuSpeedups(raw)
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
}
