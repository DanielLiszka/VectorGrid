import { describe, expect, it } from 'vitest'
import { formatUiTimeFromUnixMs } from './uiTime'

describe('formatUiTimeFromUnixMs', () => {
  it('returns an em dash for invalid or empty values', () => {
    expect(formatUiTimeFromUnixMs(undefined)).toBe('—')
    expect(formatUiTimeFromUnixMs('')).toBe('—')
    expect(formatUiTimeFromUnixMs(-1)).toBe('—')
    expect(formatUiTimeFromUnixMs('nope')).toBe('—')
  })

  it('formats valid unix millisecond values as 24-hour time', () => {
    const formatted = formatUiTimeFromUnixMs(Date.UTC(2026, 2, 28, 15, 4, 5))
    expect(formatted).toMatch(/^\d{2}:\d{2}:\d{2}$/)
  })
})
