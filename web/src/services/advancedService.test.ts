import { describe, expect, it } from 'vitest'
import { resetMoSearchSpace } from './advancedService'

describe('advancedService', () => {
  it('returns the default Composite MO search space payload + json', async () => {
    const result = await resetMoSearchSpace()

    expect(Array.isArray(result.searchSpace.params)).toBe(true)
    expect(result.searchSpace.params.length).toBeGreaterThan(0)
    expect(result.json).toContain('"params"')
    expect(result.json).toContain('"halftrend_amplitude"')
    expect(result.json).toContain('"ott_ma_type"')
  })
})
