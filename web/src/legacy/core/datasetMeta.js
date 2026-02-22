import { fileBasename } from "./path.js";

export const DATASET_META_SCHEMA_VERSION = 2;
export const RESAMPLE_RULE_SESSION_RELATIVE_OHLCV = "session_relative_ohlcv";
export const RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP = "day_first_timestamp";

export function buildDatasetMetaForArtifact(csvPath, state) {
  const path = String(csvPath || "").trim();
  if (!path) return null;

  const meta = state?.currentDataMeta && typeof state.currentDataMeta === "object" ? state.currentDataMeta : null;
  const baseMeta = state?.currentBaseDataMeta && typeof state.currentBaseDataMeta === "object" ? state.currentBaseDataMeta : null;
  const baseCtx = state?.currentBaseDatasetCtx && typeof state.currentBaseDatasetCtx === "object" ? state.currentBaseDatasetCtx : {};
  const ctx = state?.currentDatasetCtx && typeof state.currentDatasetCtx === "object" ? state.currentDatasetCtx : {};

  const label = String(ctx.label || "").trim() || fileBasename(path) || path;
  const tfCurrent = String(ctx.timeframe || meta?.timeframe || "").trim() || null;
  const base_timeframe = String(
    ctx.base_timeframe || baseCtx.base_timeframe || baseCtx.timeframe || baseMeta?.timeframe || tfCurrent || ""
  ).trim() || null;
  const derived_timeframe = String(ctx.derived_timeframe || tfCurrent || "").trim() || null;
  const timeframe = derived_timeframe || tfCurrent || null;
  const resample_applied = !!(base_timeframe && derived_timeframe && base_timeframe !== derived_timeframe);
  const resample_rule = String(ctx.resample_rule || "").trim() || (derived_timeframe ? RESAMPLE_RULE_SESSION_RELATIVE_OHLCV : null);
  const resample_anchor = String(ctx.resample_anchor || "").trim() || (derived_timeframe ? RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP : null);
  const source = String(ctx.source || "").trim() || null;
  const market = String(ctx.market || "").trim() || null;
  const symbol = String(ctx.symbol || "").trim() || null;
  const adjusted = ctx.adjusted === true ? true : (ctx.adjusted === false ? false : null);

  const len = Number(meta?.len || 0) || null;
  const min_ts = Number(meta?.min_ts || 0) || null;
  const max_ts = Number(meta?.max_ts || 0) || null;

  const sizeRaw = meta?.source_size_bytes ?? baseMeta?.source_size_bytes;
  const mtimeRaw = meta?.source_mtime_utc_ms ?? baseMeta?.source_mtime_utc_ms;
  const fpRaw = meta?.source_content_fingerprint ?? baseMeta?.source_content_fingerprint;
  const source_size_bytes = Number.isFinite(Number(sizeRaw)) ? Number(sizeRaw) : null;
  const source_mtime_utc_ms = Number.isFinite(Number(mtimeRaw)) ? Number(mtimeRaw) : null;
  const source_content_fingerprint = String(fpRaw || "").trim() || null;

  return {
    schema_version: DATASET_META_SCHEMA_VERSION,
    label,
    csv_path: path,
    timeframe,
    base_timeframe,
    derived_timeframe,
    resample_applied,
    resample_rule,
    resample_anchor,
    source,
    market,
    symbol,
    adjusted,
    len,
    min_ts,
    max_ts,
    source_size_bytes,
    source_mtime_utc_ms,
    source_content_fingerprint,
  };
}
