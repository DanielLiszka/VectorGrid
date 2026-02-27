import { describe, expect, it } from 'vitest'
import { buildMetaMlScope } from './metaMlScopeBuilder'

describe('metaMlScopeBuilder', () => {
  it('builds scope with defaults', () => {
    const scope = buildMetaMlScope({})
    expect(scope).toEqual({
      strategy_kind: 'CompositeMo',
      timeframe: null,
    })
  })

  it('normalizes provided values', () => {
    const scope = buildMetaMlScope({
      strategyKind: ' CompositeCpu ',
      timeframe: ' 1m ',
    })
    expect(scope).toEqual({
      strategy_kind: 'CompositeCpu',
      timeframe: '1m',
    })
  })
})
