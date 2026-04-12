export type ValidationReportPerfValue = string | number | boolean | null

export interface ValidationReportPerfEntry {
  name: string
  durationMs: number
  atUnixMs: number
  meta?: Record<string, ValidationReportPerfValue>
}

export interface ValidationReportPerfBridge {
  readEntries: () => ValidationReportPerfEntry[]
  clearEntries: () => void
}

interface MeasureValidationReportPerfOptions<T> {
  meta?: Record<string, unknown>
  onSuccessMeta?: (result: T) => Record<string, unknown> | undefined
  onErrorMeta?: (error: unknown) => Record<string, unknown> | undefined
}

const MAX_VALIDATION_REPORT_PERF_ENTRIES = 128
const validationReportPerfEntries: ValidationReportPerfEntry[] = []

declare global {
  interface Window {
    __VTA_VALIDATION_REPORT_PERF__?: ValidationReportPerfBridge
  }
}

export function validationReportPerfNow(): number {
  if (typeof performance !== 'undefined' && typeof performance.now === 'function') {
    return performance.now()
  }
  return Date.now()
}

function resolveValidationReportPerfBridgeTarget():
  | (typeof globalThis & { __VTA_VALIDATION_REPORT_PERF__?: ValidationReportPerfBridge })
  | null {
  if (typeof globalThis === 'undefined') return null
  return globalThis as typeof globalThis & { __VTA_VALIDATION_REPORT_PERF__?: ValidationReportPerfBridge }
}

function normalizePerfValue(value: unknown): ValidationReportPerfValue | undefined {
  if (value == null) return null
  if (typeof value === 'string') return value.trim()
  if (typeof value === 'number') return Number.isFinite(value) ? value : String(value)
  if (typeof value === 'boolean') return value
  return String(value).trim()
}

function normalizePerfMeta(
  meta?: Record<string, unknown>,
): Record<string, ValidationReportPerfValue> | undefined {
  if (!meta) return undefined
  const out: Record<string, ValidationReportPerfValue> = {}
  for (const [key, value] of Object.entries(meta)) {
    const normalizedKey = String(key || '').trim()
    if (!normalizedKey) continue
    const normalizedValue = normalizePerfValue(value)
    if (normalizedValue === undefined) continue
    out[normalizedKey] = normalizedValue
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export function recordValidationReportPerf(
  name: string,
  durationMs: number,
  meta?: Record<string, unknown>,
): void {
  const normalizedName = String(name || '').trim()
  if (!normalizedName) return
  const normalizedDurationMs = Number(durationMs)
  if (!Number.isFinite(normalizedDurationMs)) return
  const normalizedMeta = normalizePerfMeta(meta)
  validationReportPerfEntries.unshift({
    name: normalizedName,
    durationMs: Math.max(0, normalizedDurationMs),
    atUnixMs: Date.now(),
    ...(normalizedMeta ? { meta: normalizedMeta } : {}),
  })
  if (validationReportPerfEntries.length > MAX_VALIDATION_REPORT_PERF_ENTRIES) {
    validationReportPerfEntries.splice(MAX_VALIDATION_REPORT_PERF_ENTRIES)
  }
}

export async function measureValidationReportPerf<T>(
  name: string,
  operation: () => Promise<T> | T,
  options: MeasureValidationReportPerfOptions<T> = {},
): Promise<T> {
  const startedAt = validationReportPerfNow()
  try {
    const result = await operation()
    recordValidationReportPerf(name, validationReportPerfNow() - startedAt, {
      ...(options.meta ?? {}),
      ...(options.onSuccessMeta?.(result) ?? {}),
      outcome: 'ok',
    })
    return result
  } catch (error) {
    recordValidationReportPerf(name, validationReportPerfNow() - startedAt, {
      ...(options.meta ?? {}),
      ...(options.onErrorMeta?.(error) ?? {}),
      outcome: 'error',
    })
    throw error
  }
}

export function readValidationReportPerfEntries(): ValidationReportPerfEntry[] {
  return validationReportPerfEntries.map((entry) => ({
    ...entry,
    ...(entry.meta ? { meta: { ...entry.meta } } : {}),
  }))
}

export function clearValidationReportPerfEntries(): void {
  validationReportPerfEntries.splice(0, validationReportPerfEntries.length)
}

export function installValidationReportPerfBridge(): void {
  const target = resolveValidationReportPerfBridgeTarget()
  if (!target) return
  target.__VTA_VALIDATION_REPORT_PERF__ = {
    readEntries: readValidationReportPerfEntries,
    clearEntries: clearValidationReportPerfEntries,
  }
}
