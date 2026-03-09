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
})
