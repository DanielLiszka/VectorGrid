export function parseTagsInput(raw: unknown): string[] {
  const text = String(raw || '')
  const parts = text
    .split(/[,;\n]+/g)
    .flatMap((part) => String(part || '').trim().split(/\s+/g))
    .map((part) => part.trim())
    .filter((part) => part.length > 0)

  const seen = new Set<string>()
  const out: string[] = []
  for (const tag of parts) {
    const key = tag.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    out.push(tag)
  }
  return out
}

export function defaultBatchTag(): string {
  const date = new Date()
  const pad = (value: unknown) => String(Math.max(0, Math.floor(Number(value || 0)))).padStart(2, '0')
  const y = date.getUTCFullYear()
  const m = pad(date.getUTCMonth() + 1)
  const day = pad(date.getUTCDate())
  const h = pad(date.getUTCHours())
  const min = pad(date.getUTCMinutes())
  const s = pad(date.getUTCSeconds())
  return `batch-${y}${m}${day}-${h}${min}${s}`
}

export function dedupTags(tags: unknown): string[] {
  const values = Array.isArray(tags) ? tags : []
  const seen = new Set<string>()
  const out: string[] = []
  for (const entry of values) {
    const tag = String(entry || '').trim()
    if (!tag) continue
    const key = tag.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    out.push(tag)
  }
  return out
}

export function hashFNV1a32(text: unknown): number {
  let hash = 2166136261
  const value = String(text || '')
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i)
    hash = Math.imul(hash, 16777619)
  }
  return hash >>> 0
}

function canonicalizeJsonValue(value: unknown): unknown {
  if (value == null) return null
  const valueType = typeof value
  if (valueType === 'number') return Number.isFinite(value as number) ? value : null
  if (valueType === 'string' || valueType === 'boolean') return value
  if (Array.isArray(value)) return value.map((entry) => canonicalizeJsonValue(entry))
  if (valueType !== 'object') return null
  const out: Record<string, unknown> = {}
  const keys = Object.keys(value as Record<string, unknown>).sort()
  for (const key of keys) {
    const rowValue = (value as Record<string, unknown>)[key]
    if (rowValue === undefined) continue
    out[key] = canonicalizeJsonValue(rowValue)
  }
  return out
}

export function canonicalJsonString(value: unknown): string {
  return JSON.stringify(canonicalizeJsonValue(value))
}

export function hashFNV1a64Hex(text: unknown): string {
  const value = String(text || '')
  if (typeof BigInt !== 'function') {
    const h32 = hashFNV1a32(value)
    return h32.toString(16).padStart(8, '0')
  }
  let hash = 0xcbf29ce484222325n
  const prime = 0x100000001b3n
  for (let i = 0; i < value.length; i += 1) {
    hash ^= BigInt(value.charCodeAt(i))
    hash = (hash * prime) & 0xffffffffffffffffn
  }
  return hash.toString(16).padStart(16, '0')
}

function stripRunReqForFingerprint(runReq: unknown): Record<string, unknown> | null {
  if (!runReq || typeof runReq !== 'object') return null
  const out = canonicalizeJsonValue(runReq)
  if (!out || typeof out !== 'object') return null
  const record = out as Record<string, unknown>
  delete record.data_id
  delete record.dataId
  delete record.reproducibility
  return record
}

function stripValidationReqForFingerprint(validationReq: unknown): Record<string, unknown> | null {
  if (!validationReq || typeof validationReq !== 'object') return null
  const out = canonicalizeJsonValue(validationReq)
  if (!out || typeof out !== 'object') return null
  const record = out as Record<string, unknown>
  delete record.data_id
  delete record.dataId
  delete record.candidates
  delete record.admission_provenance
  delete record.reproducibility
  const cross = (record.cross_ticker && typeof record.cross_ticker === 'object')
    ? record.cross_ticker as Record<string, unknown>
    : null
  if (cross && Array.isArray(cross.datasets)) {
    const datasets = cross.datasets
      .map((entry) => {
        const row = entry && typeof entry === 'object' ? entry as Record<string, unknown> : {}
        return { csv_path: String(row.csv_path || '').trim() }
      })
      .filter((entry) => entry.csv_path.length > 0)
      .sort((left, right) => left.csv_path.localeCompare(right.csv_path))
    cross.datasets = datasets
  }
  return record
}

export function computeConfigFingerprint(runReq: unknown, validationReq: unknown): string {
  const payload = {
    version: 1,
    run_req: stripRunReqForFingerprint(runReq),
    validation_req: stripValidationReqForFingerprint(validationReq),
  }
  const canonical = canonicalJsonString(payload)
  const hex = hashFNV1a64Hex(canonical)
  return `cfg1-fnv1a64-${hex}`
}

export function splitHoldoutPaths(paths: unknown, holdoutPct: unknown, seed: unknown): { train: string[]; holdout: string[] } {
  const values = Array.isArray(paths)
    ? paths.map((entry) => String(entry || '').trim()).filter((entry) => entry.length > 0)
    : []
  const pct = Number.isFinite(Number(holdoutPct))
    ? Math.max(0, Math.min(50, Number(holdoutPct)))
    : 0
  if (!values.length || pct <= 0) return { train: values, holdout: [] }

  const total = values.length
  const holdoutCount = Math.max(0, Math.min(total, Math.floor(total * (pct / 100))))
  if (holdoutCount <= 0) return { train: values, holdout: [] }
  if (holdoutCount >= total) return { train: [], holdout: values }

  const seed32 = hashFNV1a32(String(seed ?? 0))
  const scored = values.map((path) => ({ path, score: (hashFNV1a32(path) ^ seed32) >>> 0 }))
  scored.sort((left, right) => left.score - right.score)

  const holdout = scored.slice(0, holdoutCount).map((entry) => entry.path)
  const train = scored.slice(holdoutCount).map((entry) => entry.path)
  return { train, holdout }
}
