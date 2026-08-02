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
  if (
    raw === 'optimize_only' ||
    raw === 'validate_only' ||
    raw === 'evaluate_candidates'
  ) return raw
  return 'full'
}

interface IntegerRange {
  start: number
  end: number
  step: number
}

function parseIntegerRange(
  value: unknown,
  label: string,
  minimum: number = 1,
): IntegerRange {
  const row = Array.isArray(value)
    ? value
    : value && typeof value === 'object'
      ? [
          (value as Record<string, unknown>).start ??
            (value as Record<string, unknown>).low,
          (value as Record<string, unknown>).end ??
            (value as Record<string, unknown>).high,
          (value as Record<string, unknown>).step,
        ]
      : []
  const start = Math.floor(Number(row[0]))
  const end = Math.floor(Number(row[1]))
  const step = Math.floor(Number(row[2]))
  if (
    !Number.isFinite(start) ||
    !Number.isFinite(end) ||
    !Number.isFinite(step) ||
    start < minimum ||
    end < start ||
    end > 65_535 ||
    step < 1
  ) {
    throw new Error(
      `VCET fixed-candidate replay requires a valid ${label} range [start, end, step] with values in [${minimum}, 65535].`,
    )
  }
  return { start, end, step }
}

export interface VcetFixedCandidatePair {
  fast_len: number
  slow_len: number
}

export function buildVcetFixedCandidateManifest(
  optimizationTemplate: Record<string, unknown>,
  maxCandidates: number,
): VcetFixedCandidatePair[] {
  const limit = Math.max(1, Math.floor(Number(maxCandidates)))
  const fast = parseIntegerRange(optimizationTemplate.fast_range, 'fast', 2)
  const slow = parseIntegerRange(optimizationTemplate.slow_range, 'slow')
  const candidates: VcetFixedCandidatePair[] = []

  for (let fastLen = fast.start; fastLen <= fast.end; fastLen += fast.step) {
    for (let slowLen = slow.start; slowLen <= slow.end; slowLen += slow.step) {
      if (fastLen >= slowLen) continue
      candidates.push({ fast_len: fastLen, slow_len: slowLen })
      if (candidates.length > limit) {
        throw new Error(
          `VCET fixed-candidate replay expands to more than ${limit} candidates. Tighten the current Fast/Slow ranges or raise Validation candidates.`,
        )
      }
    }
  }

  if (candidates.length === 0) {
    throw new Error(
      'VCET fixed-candidate replay found no valid Fast < Slow pairs in the current ranges.',
    )
  }
  return candidates
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
