export type ArtifactHistoryPerfValue = string | number | boolean | null

export interface ArtifactHistoryPerfEntry {
  name: string
  durationMs: number
  atUnixMs: number
  meta?: Record<string, ArtifactHistoryPerfValue>
}

export interface ArtifactHistoryPerfBridge {
  readEntries: () => ArtifactHistoryPerfEntry[]
  clearEntries: () => void
}

interface MeasureArtifactHistoryPerfOptions<T> {
  meta?: Record<string, unknown>
  onSuccessMeta?: (result: T) => Record<string, unknown> | undefined
  onErrorMeta?: (error: unknown) => Record<string, unknown> | undefined
}

const MAX_ARTIFACT_HISTORY_PERF_ENTRIES = 128
const artifactHistoryPerfEntries: ArtifactHistoryPerfEntry[] = []

declare global {
  interface Window {
    __VTA_ARTIFACT_HISTORY_PERF__?: ArtifactHistoryPerfBridge
  }
}

export function artifactHistoryPerfNow(): number {
  if (typeof performance !== 'undefined' && typeof performance.now === 'function') {
    return performance.now()
  }
  return Date.now()
}

function resolveArtifactHistoryPerfBridgeTarget():
  | (typeof globalThis & { __VTA_ARTIFACT_HISTORY_PERF__?: ArtifactHistoryPerfBridge })
  | null {
  if (typeof globalThis === 'undefined') return null
  return globalThis as typeof globalThis & { __VTA_ARTIFACT_HISTORY_PERF__?: ArtifactHistoryPerfBridge }
}

function normalizePerfValue(value: unknown): ArtifactHistoryPerfValue | undefined {
  if (value == null) return null
  if (typeof value === 'string') return value.trim()
  if (typeof value === 'number') return Number.isFinite(value) ? value : String(value)
  if (typeof value === 'boolean') return value
  return String(value).trim()
}

function normalizePerfMeta(
  meta?: Record<string, unknown>,
): Record<string, ArtifactHistoryPerfValue> | undefined {
  if (!meta) return undefined
  const out: Record<string, ArtifactHistoryPerfValue> = {}
  for (const [key, value] of Object.entries(meta)) {
    const normalizedKey = String(key || '').trim()
    if (!normalizedKey) continue
    const normalizedValue = normalizePerfValue(value)
    if (normalizedValue === undefined) continue
    out[normalizedKey] = normalizedValue
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export function recordArtifactHistoryPerf(
  name: string,
  durationMs: number,
  meta?: Record<string, unknown>,
): void {
  const normalizedName = String(name || '').trim()
  if (!normalizedName) return
  const normalizedDurationMs = Number(durationMs)
  if (!Number.isFinite(normalizedDurationMs)) return
  const normalizedMeta = normalizePerfMeta(meta)
  artifactHistoryPerfEntries.unshift({
    name: normalizedName,
    durationMs: Math.max(0, normalizedDurationMs),
    atUnixMs: Date.now(),
    ...(normalizedMeta ? { meta: normalizedMeta } : {}),
  })
  if (artifactHistoryPerfEntries.length > MAX_ARTIFACT_HISTORY_PERF_ENTRIES) {
    artifactHistoryPerfEntries.splice(MAX_ARTIFACT_HISTORY_PERF_ENTRIES)
  }
}

export async function measureArtifactHistoryPerf<T>(
  name: string,
  operation: () => Promise<T> | T,
  options: MeasureArtifactHistoryPerfOptions<T> = {},
): Promise<T> {
  const startedAt = artifactHistoryPerfNow()
  try {
    const result = await operation()
    recordArtifactHistoryPerf(name, artifactHistoryPerfNow() - startedAt, {
      ...(options.meta ?? {}),
      ...(options.onSuccessMeta?.(result) ?? {}),
      outcome: 'ok',
    })
    return result
  } catch (error) {
    recordArtifactHistoryPerf(name, artifactHistoryPerfNow() - startedAt, {
      ...(options.meta ?? {}),
      ...(options.onErrorMeta?.(error) ?? {}),
      outcome: 'error',
    })
    throw error
  }
}

export function readArtifactHistoryPerfEntries(): ArtifactHistoryPerfEntry[] {
  return artifactHistoryPerfEntries.map((entry) => ({
    ...entry,
    ...(entry.meta ? { meta: { ...entry.meta } } : {}),
  }))
}

export function clearArtifactHistoryPerfEntries(): void {
  artifactHistoryPerfEntries.splice(0, artifactHistoryPerfEntries.length)
}

export function installArtifactHistoryPerfBridge(): void {
  const target = resolveArtifactHistoryPerfBridgeTarget()
  if (!target) return
  target.__VTA_ARTIFACT_HISTORY_PERF__ = {
    readEntries: readArtifactHistoryPerfEntries,
    clearEntries: clearArtifactHistoryPerfEntries,
  }
}
