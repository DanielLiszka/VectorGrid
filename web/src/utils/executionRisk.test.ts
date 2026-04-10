import { describe, expect, it } from 'vitest'
import { describeExecutionRisk } from './executionRisk'

describe('describeExecutionRisk', () => {
  it('returns no warning for next-bar long-only execution', () => {
    expect(
      describeExecutionRisk({
        long_only: true,
        trade_on_next_bar: true,
      }),
    ).toBeNull()
  })

  it('warns when same-bar execution and short exposure are both enabled', () => {
    const notice = describeExecutionRisk({
      long_only: false,
      trade_on_next_bar: false,
    })

    expect(notice?.title).toBe('Optimistic execution mode')
    expect(String(notice?.body || '')).toContain('Same-bar execution')
  })

  it('warns when only same-bar execution is enabled', () => {
    const notice = describeExecutionRisk({
      long_only: true,
      trade_on_next_bar: false,
    })

    expect(notice?.title).toBe('Same-bar execution enabled')
  })

  it('warns when only short exposure is enabled', () => {
    const notice = describeExecutionRisk({
      long_only: false,
      trade_on_next_bar: true,
    })

    expect(notice?.title).toBe('Short exposure enabled')
    expect(String(notice?.body || '')).toContain('Short exposure is enabled')
  })

  it('supports camelCase strategy flags from frontend state', () => {
    const notice = describeExecutionRisk({
      longOnly: false,
      tradeOnNext: false,
    })

    expect(notice?.title).toBe('Optimistic execution mode')
  })

  it('supports stringified booleans from persisted or replayed strategy state', () => {
    const notice = describeExecutionRisk({
      long_only: 'false',
      trade_on_next_bar: 'true',
    })

    expect(notice?.title).toBe('Short exposure enabled')
  })

  it('falls back conservatively when boolean-like flags are invalid strings', () => {
    expect(
      describeExecutionRisk({
        long_only: 'maybe',
        trade_on_next_bar: 'unknown',
      }),
    ).toBeNull()
  })
})
