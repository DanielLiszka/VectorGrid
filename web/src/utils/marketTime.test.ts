import { describe, expect, it } from 'vitest'
import {
  MARKET_TIME_ZONE,
  formatMarketDate,
  formatMarketDateLabel,
  formatMarketTimestamp,
  formatUtcDiagnosticLabel,
  parseMarketDateInput,
  parsePolygonDateInput,
  toUtcIsoString,
} from './marketTime'

describe('marketTime', () => {
  it('exposes the product market timezone', () => {
    expect(MARKET_TIME_ZONE).toBe('America/New_York')
  })

  it('keeps the Polygon date parser aligned with the generic market parser', () => {
    expect(parsePolygonDateInput('2026-01-15')).toEqual(parseMarketDateInput('2026-01-15'))
  })

  it('parses date-only inputs as New York market dates through the spring DST boundary', () => {
    expect(parseMarketDateInput('2026-03-08')).toEqual({
      date: '2026-03-08',
      startUtcIso: '2026-03-08T05:00:00.000Z',
      endExclusiveUtcIso: '2026-03-09T04:00:00.000Z',
      timeZone: MARKET_TIME_ZONE,
    })

    expect(formatMarketDateLabel('2026-03-08')).toBe('2026-03-08 ET')
    expect(formatMarketDate('2026-03-08')).toBe('2026-03-08')
    expect(formatMarketTimestamp('2026-03-08T06:30:00.000Z')).toBe(
      '2026-03-08 01:30:00 ET (UTC-05:00)',
    )
    expect(formatMarketTimestamp('2026-03-08T07:30:00.000Z')).toBe(
      '2026-03-08 03:30:00 ET (UTC-04:00)',
    )
  })

  it('keeps fall DST date labels stable and disambiguates repeated local times', () => {
    expect(parseMarketDateInput('2026-11-01')).toEqual({
      date: '2026-11-01',
      startUtcIso: '2026-11-01T04:00:00.000Z',
      endExclusiveUtcIso: '2026-11-02T05:00:00.000Z',
      timeZone: MARKET_TIME_ZONE,
    })

    expect(
      ['2026-10-31', '2026-11-01', '2026-11-02'].map((value) =>
        formatMarketDateLabel(value),
      ),
    ).toEqual(['2026-10-31 ET', '2026-11-01 ET', '2026-11-02 ET'])
    expect(formatMarketTimestamp('2026-11-01T05:30:00.000Z')).toBe(
      '2026-11-01 01:30:00 ET (UTC-04:00)',
    )
    expect(formatMarketTimestamp('2026-11-01T06:30:00.000Z')).toBe(
      '2026-11-01 01:30:00 ET (UTC-05:00)',
    )
  })

  it('does not construct date-only market values from the local OS timezone', () => {
    const parsed = parseMarketDateInput('2026-01-15')

    expect(parsed?.date).toBe('2026-01-15')
    expect(parsed?.startUtcIso).toBe('2026-01-15T05:00:00.000Z')
    expect(parsed?.startUtcIso).not.toBe(new Date('2026-01-15').toISOString())
    expect(formatMarketDateLabel('2026-01-15')).toBe('2026-01-15 ET')
  })

  it('formats instants into market date labels independently of local Date rendering', () => {
    expect(formatMarketDate('2026-03-08T04:59:59.000Z')).toBe('2026-03-07')
    expect(formatMarketDate('2026-03-08T05:00:00.000Z')).toBe('2026-03-08')
    expect(formatMarketDateLabel('2026-03-08T04:59:59.000Z')).toBe('2026-03-07 ET')
    expect(formatMarketDateLabel('2026-03-08T05:00:00.000Z')).toBe('2026-03-08 ET')
    expect(formatMarketDateLabel(Date.UTC(2026, 10, 2, 4, 59, 59))).toBe('2026-11-01 ET')
    expect(formatMarketDateLabel(Date.UTC(2026, 10, 2, 5, 0, 0))).toBe('2026-11-02 ET')
  })

  it('preserves canonical UTC ISO values for diagnostic labels', () => {
    expect(toUtcIsoString('2026-11-01T05:30:00Z')).toBe('2026-11-01T05:30:00.000Z')
    expect(formatUtcDiagnosticLabel('2026-11-01T05:30:00Z')).toBe(
      'UTC 2026-11-01T05:30:00.000Z',
    )
    expect(formatUtcDiagnosticLabel(Date.UTC(2026, 2, 8, 7, 30, 0))).toBe(
      'UTC 2026-03-08T07:30:00.000Z',
    )
  })

  it('rejects invalid date-only values and falls back for invalid instants', () => {
    expect(parseMarketDateInput('2026-02-29')).toBeNull()
    expect(parseMarketDateInput('2026-2-09')).toBeNull()
    expect(parseMarketDateInput('')).toBeNull()
    expect(formatMarketDate('bad', '-')).toBe('-')
    expect(formatMarketDateLabel('bad')).toBe('-')
    expect(formatMarketTimestamp('2026-03-08')).toBe('-')
    expect(formatUtcDiagnosticLabel('bad')).toBe('-')
    expect(toUtcIsoString('2026-03-08')).toBeNull()
  })
})
