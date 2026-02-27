import { fileBasename } from './path'

export const DATASET_META_SCHEMA_VERSION = 2
export const RESAMPLE_RULE_SESSION_RELATIVE_OHLCV = 'session_relative_ohlcv'
export const RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP = 'day_first_timestamp'

interface DatasetMetaState {
  currentDataMeta?: unknown
  currentBaseDataMeta?: unknown
  currentBaseDatasetCtx?: unknown
  currentDatasetCtx?: unknown
}

export function buildDatasetMetaForArtifact(csvPathValue: unknown, stateValue: DatasetMetaState): Record<string, unknown> | null {
  const path = String(csvPathValue || '').trim()
  if (!path) return null

  const meta = stateValue?.currentDataMeta && typeof stateValue.currentDataMeta === 'object'
    ? stateValue.currentDataMeta as Record<string, unknown>
    : null
  const baseMeta = stateValue?.currentBaseDataMeta && typeof stateValue.currentBaseDataMeta === 'object'
    ? stateValue.currentBaseDataMeta as Record<string, unknown>
    : null
  const baseCtx = stateValue?.currentBaseDatasetCtx && typeof stateValue.currentBaseDatasetCtx === 'object'
    ? stateValue.currentBaseDatasetCtx as Record<string, unknown>
    : {}
  const ctx = stateValue?.currentDatasetCtx && typeof stateValue.currentDatasetCtx === 'object'
    ? stateValue.currentDatasetCtx as Record<string, unknown>
    : {}

  const label = String(ctx.label || '').trim() || fileBasename(path) || path
  const timeframeCurrent = String(ctx.timeframe || meta?.timeframe || '').trim() || null
  const baseTimeframe = String(
    ctx.base_timeframe || baseCtx.base_timeframe || baseCtx.timeframe || baseMeta?.timeframe || timeframeCurrent || '',
  ).trim() || null
  const derivedTimeframe = String(ctx.derived_timeframe || timeframeCurrent || '').trim() || null
  const timeframe = derivedTimeframe || timeframeCurrent || null
  const resampleApplied = !!(baseTimeframe && derivedTimeframe && baseTimeframe !== derivedTimeframe)
  const resampleRule = String(ctx.resample_rule || '').trim()
    || (derivedTimeframe ? RESAMPLE_RULE_SESSION_RELATIVE_OHLCV : null)
  const resampleAnchor = String(ctx.resample_anchor || '').trim()
    || (derivedTimeframe ? RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP : null)
  const source = String(ctx.source || '').trim() || null
  const market = String(ctx.market || '').trim() || null
  const symbol = String(ctx.symbol || '').trim() || null
  const adjusted = ctx.adjusted === true ? true : (ctx.adjusted === false ? false : null)

  const len = Number(meta?.len || 0) || null
  const minTs = Number(meta?.min_ts || 0) || null
  const maxTs = Number(meta?.max_ts || 0) || null

  const sizeRaw = meta?.source_size_bytes ?? baseMeta?.source_size_bytes
  const mtimeRaw = meta?.source_mtime_utc_ms ?? baseMeta?.source_mtime_utc_ms
  const fingerprintRaw = meta?.source_content_fingerprint ?? baseMeta?.source_content_fingerprint
  const sourceSizeBytes = Number.isFinite(Number(sizeRaw)) ? Number(sizeRaw) : null
  const sourceMtimeUtcMs = Number.isFinite(Number(mtimeRaw)) ? Number(mtimeRaw) : null
  const sourceContentFingerprint = String(fingerprintRaw || '').trim() || null

  return {
    schema_version: DATASET_META_SCHEMA_VERSION,
    label,
    csv_path: path,
    timeframe,
    base_timeframe: baseTimeframe,
    derived_timeframe: derivedTimeframe,
    resample_applied: resampleApplied,
    resample_rule: resampleRule,
    resample_anchor: resampleAnchor,
    source,
    market,
    symbol,
    adjusted,
    len,
    min_ts: minTs,
    max_ts: maxTs,
    source_size_bytes: sourceSizeBytes,
    source_mtime_utc_ms: sourceMtimeUtcMs,
    source_content_fingerprint: sourceContentFingerprint,
  }
}
