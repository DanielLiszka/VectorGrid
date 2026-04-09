// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { usePersistedMatrixColumnState } from './usePersistedMatrixColumnState'

describe('usePersistedMatrixColumnState', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('keeps newly added columns visible when persisted layouts predate schema expansion', () => {
    localStorage.setItem('matrix_columns', JSON.stringify({
      visibleColumnKeys: ['rank', 'name'],
      knownColumnKeys: ['rank', 'name'],
    }))

    const columns = ref([
      { key: 'rank', label: '#' },
      { key: 'name', label: 'Name' },
      { key: 'pnl', label: 'PnL' },
    ])

    const state = usePersistedMatrixColumnState(columns, 'matrix_columns')

    expect(state.visibleColumnKeys.value).toEqual(['rank', 'name', 'pnl'])
    expect(state.columnOptions.value).toEqual([
      { key: 'rank', label: '#', visible: true },
      { key: 'name', label: 'Name', visible: true },
      { key: 'pnl', label: 'PnL', visible: true },
    ])
    expect(state.columnVisibility.value).toEqual({})
    expect(state.hiddenCount.value).toBe(0)
    expect(state.canRestoreDefaults.value).toBe(false)
  })
})
