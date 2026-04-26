// @vitest-environment jsdom

import { describe, expect, it, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { usePersistedTableDensity, normalizeTableDensity } from './usePersistedTableDensity'

describe('usePersistedTableDensity', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('normalizes unknown density values to the fallback', () => {
    expect(normalizeTableDensity('compact')).toBe('compact')
    expect(normalizeTableDensity('spacious')).toBe('spacious')
    expect(normalizeTableDensity('tiny', 'comfortable')).toBe('comfortable')
  })

  it('loads, persists, and clears table density preferences', async () => {
    localStorage.setItem('table_density', 'compact')

    const state = usePersistedTableDensity('table_density')
    expect(state.tableDensity.value).toBe('compact')

    state.setTableDensity('spacious')
    await nextTick()
    expect(localStorage.getItem('table_density')).toBe('spacious')

    state.restoreDefaultTableDensity()
    await nextTick()
    expect(state.tableDensity.value).toBe('comfortable')
    expect(localStorage.getItem('table_density')).toBeNull()
  })
})
