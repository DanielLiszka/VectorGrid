import { describe, expect, it } from 'vitest'
import {
  formatUiBinaryBytes,
  formatUiCompactDurationMs,
  formatUiCount,
  formatUiPercentFromFraction,
  formatUtcDateFromUnixMs,
  formatUtcDateTimeFromUnixMs,
} from './uiFormat'

describe('uiFormat', () => {
  it('formats counts with locale-aware grouping', () => {
    expect(formatUiCount(1234567)).toBe((1234567).toLocaleString())
    expect(formatUiCount(0)).toBe('—')
  })

  it('formats binary byte sizes compactly', () => {
    expect(formatUiBinaryBytes(1536)).toBe('2 KiB')
    expect(formatUiBinaryBytes(0)).toBe('')
  })

  it('formats percentages from fractions', () => {
    expect(formatUiPercentFromFraction(0.4567, 1)).toBe('45.7%')
    expect(formatUiPercentFromFraction('bad')).toBe('—')
  })

  it('formats compact durations from milliseconds', () => {
    expect(formatUiCompactDurationMs(850)).toBe('850ms')
    expect(formatUiCompactDurationMs(3400)).toBe('3.4s')
    expect(formatUiCompactDurationMs(125000)).toBe('2.1m')
  })

  it('formats UTC dates from unix milliseconds', () => {
    expect(formatUtcDateFromUnixMs(Date.UTC(2026, 2, 28, 15, 4, 5))).toBe('2026-03-28')
    expect(formatUtcDateFromUnixMs('bad')).toBe('—')
  })

  it('formats UTC date-times from unix milliseconds', () => {
    expect(formatUtcDateTimeFromUnixMs(Date.UTC(2026, 2, 28, 15, 4, 5))).toBe('2026-03-28 15:04:05')
    expect(formatUtcDateTimeFromUnixMs(0)).toBe('-')
  })
})
