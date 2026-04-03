function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function normalizePathList(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  const out: string[] = []
  for (const item of value) {
    const next = normalizeText(item)
    if (!next) continue
    out.push(next)
  }
  return out
}

function parseTags(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value
      .map((tag) => normalizeText(tag))
      .filter((tag) => tag.length > 0)
  }
  const raw = normalizeText(value)
  if (!raw) return []
  return raw
    .split(/[,;\n]+/g)
    .flatMap((part) => normalizeText(part).split(/\s+/g))
    .map((part) => normalizeText(part))
    .filter((part) => part.length > 0)
}

function dedupTags(tags: string[]): string[] {
  const out: string[] = []
  const seen = new Set<string>()
  for (const tag of tags) {
    const next = normalizeText(tag)
    if (!next) continue
    const key = next.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    out.push(next)
  }
  return out
}

function normalizePreloadPolicy(value: unknown): string {
  const raw = normalizeText(value).toLowerCase()
  if (raw === 'disabled' || raw === 'host_plus_doublema_vram') return raw
  return 'host_only'
}

function normalizeExecutionMode(value: unknown): string {
  const raw = normalizeText(value).toLowerCase()
  if (raw === 'optimize_only' || raw === 'validate_only') return raw
  return 'full'
}

export interface MetaBatchBuildInputs {
  selectedPaths?: unknown
  holdoutPct?: unknown
  holdoutTag?: unknown
  executionMode?: unknown
  preloadPolicy?: unknown
  autoMeta?: unknown
  autoStage5?: unknown
  skipDone?: unknown
  tags?: unknown
}

export function buildMetaBatchRequest(inputs: MetaBatchBuildInputs): Record<string, unknown> {
  const holdoutPctRaw = Math.floor(Number(inputs.holdoutPct))
  const holdoutPct = Number.isFinite(holdoutPctRaw) ? Math.max(0, Math.min(50, holdoutPctRaw)) : 0
  const holdoutTagRaw = normalizeText(inputs.holdoutTag)
  const tags = dedupTags(parseTags(inputs.tags))

  return {
    selected_paths: normalizePathList(inputs.selectedPaths),
    holdout_pct: holdoutPct,
    holdout_tag: holdoutTagRaw || 'holdout',
    execution_mode: normalizeExecutionMode(inputs.executionMode),
    preload_policy: normalizePreloadPolicy(inputs.preloadPolicy),
    auto_meta: normalizeBoolean(inputs.autoMeta),
    auto_stage5: normalizeBoolean(inputs.autoStage5),
    skip_done: normalizeBoolean(inputs.skipDone),
    tags,
  }
}
