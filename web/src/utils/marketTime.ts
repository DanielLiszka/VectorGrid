export const MARKET_TIME_ZONE = 'America/New_York'

const MARKET_TIME_LABEL = 'ET'
const DATE_ONLY_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/

type MarketDateParts = {
  year: number
  month: number
  day: number
  date: string
}

type MarketDateTimeParts = MarketDateParts & {
  hour: number
  minute: number
  second: number
}

export type ParsedMarketDateInput = {
  date: string
  startUtcIso: string
  endExclusiveUtcIso: string
  timeZone: typeof MARKET_TIME_ZONE
}

const marketDateTimeFormatter = new Intl.DateTimeFormat('en-US', {
  timeZone: MARKET_TIME_ZONE,
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hourCycle: 'h23',
})

export function parseMarketDateInput(value: unknown): ParsedMarketDateInput | null {
  return parsePolygonDateInput(value)
}

export function parsePolygonDateInput(value: unknown): ParsedMarketDateInput | null {
  const parts = parseDateOnlyParts(value)
  if (!parts) return null

  const next = addCalendarDays(parts, 1)
  const startUtcMs = marketLocalDateTimeToUtcMs(parts)
  const endExclusiveUtcMs = marketLocalDateTimeToUtcMs(next)
  if (startUtcMs === null || endExclusiveUtcMs === null) return null

  return {
    date: parts.date,
    startUtcIso: new Date(startUtcMs).toISOString(),
    endExclusiveUtcIso: new Date(endExclusiveUtcMs).toISOString(),
    timeZone: MARKET_TIME_ZONE,
  }
}

export function formatMarketDateLabel(value: unknown, fallback = '-'): string {
  const dateOnly = parseDateOnlyParts(value)
  if (dateOnly) return `${dateOnly.date} ${MARKET_TIME_LABEL}`

  const date = toValidDate(value)
  if (!date) return fallback
  return `${formatDateParts(readMarketDateTimeParts(date))} ${MARKET_TIME_LABEL}`
}

export function formatMarketDate(value: unknown, fallback = ''): string {
  const dateOnly = parseDateOnlyParts(value)
  if (dateOnly) return dateOnly.date

  const date = toValidDate(value)
  if (!date) return fallback
  return formatDateParts(readMarketDateTimeParts(date))
}

export function formatMarketTimestamp(value: unknown, fallback = '-'): string {
  const date = toValidDate(value)
  if (!date) return fallback

  const parts = readMarketDateTimeParts(date)
  const offset = formatUtcOffsetLabel(getMarketUtcOffsetMinutes(date, parts))
  return `${formatDateParts(parts)} ${formatTimeParts(parts)} ${MARKET_TIME_LABEL} (${offset})`
}

export function toUtcIsoString(value: unknown): string | null {
  const date = toValidDate(value)
  return date ? date.toISOString() : null
}

export function formatUtcDiagnosticLabel(value: unknown, fallback = '-'): string {
  const utcIso = toUtcIsoString(value)
  return utcIso ? `UTC ${utcIso}` : fallback
}

function parseDateOnlyParts(value: unknown): MarketDateParts | null {
  const text = String(value ?? '').trim()
  const match = DATE_ONLY_PATTERN.exec(text)
  if (!match) return null

  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  if (!isValidDateParts(year, month, day)) return null

  return {
    year,
    month,
    day,
    date: `${String(year).padStart(4, '0')}-${pad2(month)}-${pad2(day)}`,
  }
}

function isValidDateParts(year: number, month: number, day: number): boolean {
  if (!Number.isInteger(year) || year < 1000 || year > 9999) return false
  if (!Number.isInteger(month) || month < 1 || month > 12) return false
  if (!Number.isInteger(day) || day < 1) return false
  return day <= daysInMonth(year, month)
}

function daysInMonth(year: number, month: number): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate()
}

function addCalendarDays(parts: MarketDateParts, days: number): MarketDateParts {
  const date = new Date(Date.UTC(parts.year, parts.month - 1, parts.day + days))
  const year = date.getUTCFullYear()
  const month = date.getUTCMonth() + 1
  const day = date.getUTCDate()
  return {
    year,
    month,
    day,
    date: `${String(year).padStart(4, '0')}-${pad2(month)}-${pad2(day)}`,
  }
}

function marketLocalDateTimeToUtcMs(parts: MarketDateParts): number | null {
  const wantedAsUtc = Date.UTC(parts.year, parts.month - 1, parts.day, 0, 0, 0)
  let candidate = wantedAsUtc

  for (let attempt = 0; attempt < 4; attempt += 1) {
    const actual = readMarketDateTimeParts(new Date(candidate))
    const actualAsUtc = Date.UTC(
      actual.year,
      actual.month - 1,
      actual.day,
      actual.hour,
      actual.minute,
      actual.second,
    )
    const delta = wantedAsUtc - actualAsUtc
    candidate += delta
    if (delta === 0) break
  }

  const resolved = readMarketDateTimeParts(new Date(candidate))
  if (
    resolved.year !== parts.year ||
    resolved.month !== parts.month ||
    resolved.day !== parts.day ||
    resolved.hour !== 0 ||
    resolved.minute !== 0 ||
    resolved.second !== 0
  ) {
    return null
  }

  return candidate
}

function readMarketDateTimeParts(date: Date): MarketDateTimeParts {
  const values: Partial<Record<Intl.DateTimeFormatPartTypes, string>> = {}
  for (const part of marketDateTimeFormatter.formatToParts(date)) {
    if (part.type !== 'literal') values[part.type] = part.value
  }

  const year = Number(values.year)
  const month = Number(values.month)
  const day = Number(values.day)
  const hour = Number(values.hour) % 24
  const minute = Number(values.minute)
  const second = Number(values.second)

  return {
    year,
    month,
    day,
    hour,
    minute,
    second,
    date: `${String(year).padStart(4, '0')}-${pad2(month)}-${pad2(day)}`,
  }
}

function getMarketUtcOffsetMinutes(date: Date, parts: MarketDateTimeParts): number {
  const localAsUtc = Date.UTC(
    parts.year,
    parts.month - 1,
    parts.day,
    parts.hour,
    parts.minute,
    parts.second,
  )
  return Math.round((localAsUtc - date.getTime()) / 60_000)
}

function toValidDate(value: unknown): Date | null {
  if (typeof value === 'string' && DATE_ONLY_PATTERN.test(value.trim())) return null

  const date =
    value instanceof Date
      ? new Date(value.getTime())
      : typeof value === 'number' || typeof value === 'string'
        ? new Date(value)
        : null

  if (!date || !Number.isFinite(date.getTime())) return null
  return date
}

function formatDateParts(parts: Pick<MarketDateTimeParts, 'year' | 'month' | 'day'>): string {
  return `${String(parts.year).padStart(4, '0')}-${pad2(parts.month)}-${pad2(parts.day)}`
}

function formatTimeParts(parts: Pick<MarketDateTimeParts, 'hour' | 'minute' | 'second'>): string {
  return `${pad2(parts.hour)}:${pad2(parts.minute)}:${pad2(parts.second)}`
}

function formatUtcOffsetLabel(offsetMinutes: number): string {
  const sign = offsetMinutes >= 0 ? '+' : '-'
  const absolute = Math.abs(offsetMinutes)
  const hours = Math.floor(absolute / 60)
  const minutes = absolute % 60
  return `UTC${sign}${pad2(hours)}:${pad2(minutes)}`
}

function pad2(value: number): string {
  return String(Math.max(0, Math.floor(value))).padStart(2, '0')
}
