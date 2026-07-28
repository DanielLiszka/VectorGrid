import { describe, expect, it } from 'vitest'
import { resolveRovingRowIndex } from './rovingRowFocus'

describe('resolveRovingRowIndex', () => {
  it('moves with arrows and clamps at table boundaries', () => {
    expect(resolveRovingRowIndex(1, 3, 'ArrowDown')).toBe(2)
    expect(resolveRovingRowIndex(2, 3, 'ArrowDown')).toBe(2)
    expect(resolveRovingRowIndex(1, 3, 'ArrowUp')).toBe(0)
    expect(resolveRovingRowIndex(0, 3, 'ArrowUp')).toBe(0)
  })

  it('supports Home and End without treating activation keys as navigation', () => {
    expect(resolveRovingRowIndex(1, 4, 'Home')).toBe(0)
    expect(resolveRovingRowIndex(1, 4, 'End')).toBe(3)
    expect(resolveRovingRowIndex(1, 4, 'Enter')).toBeNull()
    expect(resolveRovingRowIndex(0, 0, 'End')).toBeNull()
  })
})
