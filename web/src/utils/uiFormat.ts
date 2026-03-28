const countFormatter = new Intl.NumberFormat()
const oneDecimalFormatter = new Intl.NumberFormat(undefined, {
  minimumFractionDigits: 0,
  maximumFractionDigits: 1,
})

export function formatUiCount(value: unknown, fallback = '—'): string {
  const count = Number(value || 0)
  if (!Number.isFinite(count) || count <= 0) return fallback
  return countFormatter.format(count)
}

export function formatUiBinaryBytes(value: unknown, fallback = ''): string {
  const bytes = Number(value || 0)
  if (!Number.isFinite(bytes) || bytes <= 0) return fallback
  const gib = bytes / 1024 ** 3
  if (gib >= 1) return `${oneDecimalFormatter.format(gib)} GiB`
  const mib = bytes / 1024 ** 2
  if (mib >= 1) return `${countFormatter.format(Math.round(mib))} MiB`
  const kib = bytes / 1024
  if (kib >= 1) return `${countFormatter.format(Math.round(kib))} KiB`
  return `${countFormatter.format(Math.floor(bytes))} B`
}

export function formatUiPercentFromFraction(
  value: unknown,
  digits = 0,
  fallback = '—',
): string {
  const fraction = Number(value)
  if (!Number.isFinite(fraction)) return fallback
  return `${(fraction * 100).toFixed(Math.max(0, Math.floor(digits)))}%`
}

export function formatUiCompactDurationMs(value: unknown, fallback = '—'): string {
  const ms = Number(value)
  if (!Number.isFinite(ms) || ms < 0) return fallback
  if (ms < 1000) return `${Math.round(ms)}ms`
  if (ms < 60_000) return `${oneDecimalFormatter.format(ms / 1000)}s`
  return `${oneDecimalFormatter.format(ms / 60_000)}m`
}

export function formatUtcDateFromUnixMs(value: unknown, fallback = '—'): string {
  const ms = Number(value)
  if (!Number.isFinite(ms)) return fallback
  const date = new Date(ms)
  if (!Number.isFinite(date.getTime())) return fallback
  return date.toISOString().slice(0, 10)
}

export function formatUtcDateTimeFromUnixMs(value: unknown, fallback = '-'): string {
  const ms = Number(value)
  if (!Number.isFinite(ms) || ms <= 0) return fallback
  const date = new Date(ms)
  if (!Number.isFinite(date.getTime())) return fallback
  const pad = (part: number): string => String(Math.max(0, Math.floor(part))).padStart(2, '0')
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())}:${pad(date.getUTCSeconds())}`
}
